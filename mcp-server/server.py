"""
Trajectory Recorder MCP Server for AI Agents.
Provides high-signal, token-optimized workplace trajectory and behavioral data to LLMs.
Filters background noise and aggregates continuous mouse/keyboard actions at the server level.
"""

from mcp.server.fastmcp import FastMCP
import sys
import os
import json
from collections import defaultdict

# Add current directory to path
sys.path.insert(0, os.path.dirname(__file__))

from db import list_machines_db, get_sessions_for_machine, ensure_session_db, get_raw_actions_from_sqlite
from filters import filter_and_aggregate_actions, format_compact_dsl, calculate_productivity

mcp = FastMCP(
    "Trajectory-Recorder",
    instructions="Provides token-optimized employee desktop activity and workflow telemetry from the enterprise Trajectory Ingestion Server."
)

@mcp.tool()
def list_active_machines() -> str:
    """
    List all machines registered in the enterprise Trajectory Recorder cluster,
    including online status, last heartbeat timestamp, and total sessions recorded.
    """
    machines = list_machines_db()
    if not machines:
        return "Khong tim thay may tinh nao trong he thong."
    
    lines = ["| Machine ID | Hostname | Trang Thai | Last Heartbeat (UTC) | Tong Session |"]
    lines.append("| :--- | :--- | :---: | :--- | :---: |")
    for m in machines:
        status_icon = "[ONLINE]" if m["status"] == "ONLINE" else "[OFFLINE]"
        lines.append(f"| **{m['machine_id']}** | {m['hostname']} | {status_icon} | {m['last_heartbeat']} | {m['total_sessions']} |")
    return "\n".join(lines)

@mcp.tool()
def get_daily_summary(machine_id: str, date: str = "2026-09-11") -> str:
    """
    Generate an executive, token-optimized daily work summary for a specific machine/employee.
    Summarizes work episodes, top applications, documents edited, and idle time (~800 - 1,500 tokens).
    Format of date: 'YYYY-MM-DD' or 'YYYYMMDD'. Default: '2026-09-11'.
    """
    clean_date = date.replace("-", "")
    sessions = get_sessions_for_machine(machine_id, clean_date)
    if not sessions:
        return f"Khong tim thay phien lam viec nao cho may '{machine_id}' vao ngay '{date}'."

    total_sessions = len(sessions)
    app_counts = defaultdict(int)
    window_counts = defaultdict(int)
    hourly_episodes = []

    for sess in sessions:
        sid = sess["session_id"]
        skey = sess["storage_key"]
        db_file = ensure_session_db(sid, skey)
        if not db_file:
            continue
        raw_actions = get_raw_actions_from_sqlite(db_file, limit=1500)
        filtered = filter_and_aggregate_actions(raw_actions)

        # Extract hour from session id (e.g. TESTER-PC_20260911_080000_...)
        hour_part = "??:00"
        parts = sid.split("_")
        if len(parts) >= 3 and len(parts[2]) >= 2:
            try:
                hour_utc = int(parts[2][:2])
                hour_vn = (hour_utc + 7) % 24
                hour_part = f"{hour_vn:02d}:00 - {(hour_vn+1)%24:02d}:00"
            except:
                pass

        sess_apps = defaultdict(int)
        sess_windows = defaultdict(int)
        for act in filtered:
            app_counts[act["app"]] += 1
            window_counts[act["title"]] += 1
            sess_apps[act["app"]] += 1
            if act["title"]:
                sess_windows[act["title"]] += 1

        top_sess_app = max(sess_apps.items(), key=lambda x: x[1])[0] if sess_apps else "Idle"
        top_sess_titles = sorted(sess_windows.items(), key=lambda x: x[1], reverse=True)[:2]
        title_summary = ", ".join(f'"{t[0][:40]}"' for t in top_sess_titles) if top_sess_titles else "Nền / Ít thao tác"
        hourly_episodes.append(f"- **{hour_part}**: [{top_sess_app}] {title_summary} ({len(filtered)} thao tác)")

    # Build response markdown
    res = [f"# Báo Cáo Hoạt Động Hàng Ngày: {machine_id} ({date})"]
    res.append(f"- **Tổng số phiên ghi nhận**: {total_sessions} sessions")
    res.append("")
    res.append("## 🕒 Dòng thời gian làm việc (Work Episodes):")
    res.extend(hourly_episodes)
    res.append("")
    res.append("## 💻 Top Ứng Dụng Hoạt Động Nhiều Nhất:")
    for app, count in sorted(app_counts.items(), key=lambda x: x[1], reverse=True)[:8]:
        res.append(f"- **{app}**: {count} thao tác người dùng")
    res.append("")
    res.append("## 📄 Các Cửa Sổ & Tài Liệu Trọng Tâm:")
    for win, count in sorted(window_counts.items(), key=lambda x: x[1], reverse=True)[:10]:
        if win:
            res.append(f"- [{count:2d}x] {win}")

    return "\n".join(res)

@mcp.tool()
def inspect_timeline(machine_id: str, session_id: str = None, limit: int = 50) -> str:
    """
    Inspect granular, step-by-step user actions in a specific session with server-side noise reduction.
    Collapses mouse moves into vectors and ignores invisible message-only windows.
    If session_id is not specified, uses the latest session of the machine.
    """
    sessions = get_sessions_for_machine(machine_id)
    if not sessions:
        return f"Khong tim thay session nao cho may '{machine_id}'."

    target_session = None
    if session_id:
        for s in sessions:
            if s["session_id"] == session_id:
                target_session = s
                break
    else:
        target_session = sessions[-1]

    if not target_session:
        return f"Khong tim thay session_id '{session_id}'."

    sid = target_session["session_id"]
    skey = target_session["storage_key"]
    db_file = ensure_session_db(sid, skey)
    if not db_file:
        return f"Khong the tai database cho session {sid} tu MinIO."

    raw_actions = get_raw_actions_from_sqlite(db_file, limit=limit * 5)
    filtered = filter_and_aggregate_actions(raw_actions)[:limit]

    output = [f"### Chi Tiết Dòng Thời Gian: {sid}"]
    output.append(format_compact_dsl(filtered))
    return "\n".join(output)

@mcp.tool()
def search_actions(keyword: str, machine_id: str = None, date: str = "2026-09-11", limit: int = 20) -> str:
    """
    Search for specific keywords across window titles, typed text, or application names.
    Useful for locating when an employee worked on a specific task, project, or file (e.g. 'Word', 'Lark', 'burugburu').
    """
    clean_date = date.replace("-", "")
    machines = [machine_id] if machine_id else [m["machine_id"] for m in list_machines_db()]
    results = []
    kw_lower = keyword.lower()

    for mid in machines:
        sessions = get_sessions_for_machine(mid, clean_date)
        for sess in sessions:
            sid = sess["session_id"]
            db_file = ensure_session_db(sid, sess["storage_key"])
            if not db_file:
                continue
            raw_actions = get_raw_actions_from_sqlite(db_file, limit=1000)
            filtered = filter_and_aggregate_actions(raw_actions)
            for act in filtered:
                title = act.get("title") or ""
                summary = act.get("summary") or ""
                app = act.get("app") or ""
                if kw_lower in title.lower() or kw_lower in summary.lower() or kw_lower in app.lower():
                    results.append(f"| {act.get('time_str')} | **{mid}** | {app} | {title[:40]} | {summary} |")
                    if len(results) >= limit:
                        break
            if len(results) >= limit:
                break
        if len(results) >= limit:
            break

    if not results:
        return f"Khong tim thay ket qua nao chua tu khoa '{keyword}' vao ngay {date}."

    header = ["| Giờ | Máy tính | App | Cửa sổ | Thao tác |", "| :---: | :--- | :--- | :--- | :--- |"]
    return "\n".join(header + results)

@mcp.tool()
def get_productivity_metrics(machine_id: str, date: str = "2026-09-11") -> str:
    """
    Compute application distribution, context switching, and active work focus for an employee.
    Useful for analyzing workload, software adoption, and deep work vs multitasking.
    """
    clean_date = date.replace("-", "")
    sessions = get_sessions_for_machine(machine_id, clean_date)
    if not sessions:
        return f"Khong co du lieu danh gia cho may '{machine_id}' ngay {date}."

    all_filtered_actions = []
    window_switches = 0

    for sess in sessions:
        db_file = ensure_session_db(sess["session_id"], sess["storage_key"])
        if not db_file:
            continue
        raw_actions = get_raw_actions_from_sqlite(db_file, limit=1000)
        filtered = filter_and_aggregate_actions(raw_actions)
        for a in filtered:
            if a.get("action_type") == "WindowSwitch":
                window_switches += 1
            all_filtered_actions.append(a)

    metrics = calculate_productivity(all_filtered_actions)
    total_actions = metrics["total_actions"]
    apps = metrics["app_distribution"]

    lines = [f"# Phân Tích Hiệu Suất & Ứng Dụng: {machine_id} ({date})"]
    lines.append(f"- **Tổng số thao tác người dùng thật**: {total_actions:,}")
    lines.append(f"- **Tần suất chuyển đổi cửa sổ (Context Switching)**: {window_switches} lần")
    lines.append("")
    lines.append("## 📊 Tỷ Lệ Sử Dụng Ứng Dụng:")
    for app, pct in apps.items():
        bar_len = int(pct // 5)
        bar = "█" * bar_len + "░" * (20 - bar_len)
        lines.append(f"- `{bar}` **{pct:4.1f}%** : {app}")

    return "\n".join(lines)

if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="Trajectory Recorder MCP Server for AI Agents")
    parser.add_argument(
        "--transport",
        choices=["sse", "stdio"],
        default="sse",
        help="Transport mode: 'sse' (network mode for remote machines) or 'stdio' (local process). Default: sse"
    )
    parser.add_argument("--host", default="0.0.0.0", help="Host IP to bind (default: 0.0.0.0)")
    parser.add_argument("--port", type=int, default=8000, help="Port to listen (default: 8000)")
    args = parser.parse_args()

    if args.transport == "sse":
        print(f"[*] Starting Trajectory MCP Server in SSE Network Mode on http://{args.host}:{args.port}/sse")
        print(f"[*] Other machines on LAN can connect via URL: http://192.168.1.24:{args.port}/sse")
        mcp.settings.host = args.host
        mcp.settings.port = args.port
        mcp.run(transport="sse")
    else:
        mcp.run(transport="stdio")

