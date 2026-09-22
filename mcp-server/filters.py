"""
Filter and semantic aggregator for Trajectory Recorder data.
Transforms low-level OS telemetry into ultra-compact, high-signal semantic actions for AI.
"""

import json
from datetime import datetime

# Process names of background system services that do not represent direct user UI interactions
BACKGROUND_PROCESSES = {
    "svchost.exe",
    "splwow64.exe",
    "taskhostw.exe",
    "dwm.exe",
    "dllhost.exe",
    "wpscloudsvr.exe",
    "System",
    "sihost.exe",
    "fontdrvhost.exe",
    "conhost.exe",
    "wpsupdate.exe",
    "SDXHelper.exe",
    "ctfmon.exe",
    "RuntimeBroker.exe",
    "SearchHost.exe",
    "SearchIndexer.exe",
}

# Window title prefixes of hidden, message-only, or internal OS helper windows
INVISIBLE_WINDOW_PREFIXES = (
    "OLEChannelWnd",
    "OleMainThreadWndName",
    "Default IME",
    "CicMarshalWnd",
    "MSCTFIME UI",
    "Qt5ClipboardView",
    "DesktopWindowXamlSource",
    "MIT Message Only Window",
    "PopupHost",
    "Input Occlusion Window",
    "Non Client Input Sink Window",
    "Chrome Legacy Window",
    "GDI+ Window",
    "VDevMonitor",
    "XCP",
    "ShellPreviewExtensionHost",
    "TimerNativeWindow",
    "WUIconWindow",
    "KAccountWnd_",
    "updMainWindow",
    "Kingsoft_Update_",
)

def is_background_process(proc_name: str) -> bool:
    if not proc_name:
        return True
    return proc_name.strip() in BACKGROUND_PROCESSES

def is_invisible_window(title: str) -> bool:
    if not title:
        return True
    t = title.strip()
    if not t or len(t) <= 1:
        return True
    for prefix in INVISIBLE_WINDOW_PREFIXES:
        if t.startswith(prefix):
            return True
    return False

def clean_action(action: dict) -> dict:
    """Extract clean app name, window title, and clean parameters."""
    ctx_raw = action.get("context_json") or "{}"
    param_raw = action.get("parameters_json") or "{}"
    try:
        ctx = json.loads(ctx_raw) if isinstance(ctx_raw, str) else ctx_raw
    except Exception:
        ctx = {}
    try:
        params = json.loads(param_raw) if isinstance(param_raw, str) else param_raw
    except Exception:
        params = {}

    pname = ctx.get("process_name") or ""
    wtitle = ctx.get("window_title") or ""
    atype = action.get("action_type") or "Unknown"
    ts = action.get("timestamp_utc") or ""

    return {
        "timestamp": ts,
        "time_str": ts.split("T")[1][:8] if "T" in ts else ts,
        "app": pname,
        "title": wtitle,
        "action_type": atype,
        "params": params,
        "duration_ms": action.get("duration_ms") or 0,
    }

def filter_and_aggregate_actions(raw_actions: list) -> list:
    """
    Applies the two core filtering & aggregation rules:
    1. Skip background processes and invisible message-only windows.
    2. Aggregate MouseMove from initial (x, y) to final (x, y), absorbing into subsequent Click.
    3. Aggregate bursts of typing into text strings.
    """
    cleaned = []
    for act in raw_actions:
        item = clean_action(act)
        # Rule 1: Filter background processes & invisible windows
        if is_background_process(item["app"]):
            continue
        if is_invisible_window(item["title"]):
            continue
        cleaned.append(item)

    # Rule 2: Aggregate MouseMove and Typing
    results = []
    pending_mouse_moves = []
    pending_typing = []

    def flush_mouse():
        nonlocal pending_mouse_moves
        if not pending_mouse_moves:
            return
        first = pending_mouse_moves[0]
        last = pending_mouse_moves[-1]
        p_first = first["params"].get("point", {})
        p_last = last["params"].get("point", {})
        x1, y1 = p_first.get("x", 0), p_first.get("y", 0)
        x2, y2 = p_last.get("x", 0), p_last.get("y", 0)
        dx = x2 - x1
        dy = y2 - y1
        dist = (dx * dx + dy * dy) ** 0.5
        count = len(pending_mouse_moves)

        # Emit the aggregated trajectory vector of the uninterrupted movement.
        # Preserves navigation path from start to end for AI imitation learning.
        results.append({
            "timestamp": first["timestamp"],
            "time_str": first["time_str"],
            "app": last["app"],
            "title": last["title"],
            "action_type": "MouseMove",
            "summary": f"Di chuột: ({x1:.0f}, {y1:.0f}) -> ({x2:.0f}, {y2:.0f}) [cự ly: {dist:.0f}px, {count} mẫu]",
            "from_x": x1,
            "from_y": y1,
            "to_x": x2,
            "to_y": y2,
            "distance_px": round(dist, 1),
            "samples_count": count,
        })
        pending_mouse_moves.clear()

    def flush_typing():
        nonlocal pending_typing
        if not pending_typing:
            return
        first = pending_typing[0]
        combined_text = "".join(
            t["params"].get("text") or t["params"].get("chars") or "" for t in pending_typing
        )
        if combined_text:
            results.append({
                "timestamp": first["timestamp"],
                "time_str": first["time_str"],
                "app": first["app"],
                "title": first["title"],
                "action_type": "TypeText",
                "summary": f'Gõ văn bản: "{combined_text}"',
            })
        pending_typing.clear()

    for item in cleaned:
        atype = item["action_type"]

        if atype == "MouseMove":
            flush_typing()
            pending_mouse_moves.append(item)
            continue

        if atype == "TypeText":
            flush_mouse()
            pending_typing.append(item)
            continue

        # For any other action (Click, WindowSwitch, Scroll, etc.)
        flush_mouse()
        flush_typing()

        summary = ""
        if atype in ("Click", "DoubleClick", "RightClick"):
            btn = item["params"].get("button", "Left")
            p = item["params"].get("point", {})
            summary = f"{atype} ({btn}) tại ({p.get('x', 0):.0f}, {p.get('y', 0):.0f})"
        elif atype == "WindowSwitch":
            summary = f"Chuyển sang cửa sổ: {item['title']}"
        elif atype == "Scroll":
            dy = item["params"].get("delta_y", 0)
            summary = f"Cuộn trang {'xuống' if dy < 0 else 'lên'} (delta: {dy:.0f})"
        elif atype == "DragDrop":
            summary = "Kéo thả phần tử"
        elif atype == "Copy":
            summary = "Sao chép (Copy to Clipboard)"
        elif atype == "Paste":
            summary = "Dán (Paste from Clipboard)"
        else:
            summary = f"{atype}: {item['title']}"

        item["summary"] = summary
        results.append(item)

    flush_mouse()
    flush_typing()
    return results

def format_compact_dsl(actions: list) -> str:
    """Renders actions into compact Markdown DSL: Time | App | Title | Action."""
    if not actions:
        return "Không có thao tác nào ghi nhận."
    lines = []
    lines.append("| Giờ | Ứng dụng | Cửa sổ làm việc | Thao tác |")
    lines.append("| :---: | :--- | :--- | :--- |")
    for a in actions:
        time_str = a.get("time_str") or ""
        app = a.get("app") or ""
        title = a.get("title") or ""
        if len(title) > 50:
            title = title[:47] + "..."
        summary = a.get("summary") or a.get("action_type") or ""
        lines.append(f"| {time_str} | **{app}** | {title} | {summary} |")
    return "\n".join(lines)

def calculate_productivity(actions: list) -> dict:
    """Calculates active vs idle time and per-app usage percentage."""
    if not actions:
        return {"active_minutes": 0, "idle_minutes": 0, "apps": {}}

    app_counts = {}
    for a in actions:
        p = a.get("app") or "Unknown"
        app_counts[p] = app_counts.get(p, 0) + 1

    total_actions = len(actions)
    app_percentages = {
        app: round((cnt / total_actions) * 100, 1)
        for app, cnt in sorted(app_counts.items(), key=lambda x: x[1], reverse=True)
    }

    return {
        "total_actions": total_actions,
        "app_distribution": app_percentages,
    }
