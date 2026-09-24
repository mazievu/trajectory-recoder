"""
Laya Multi-Stage QC Service
----------------------------
Service chạy phía server, tự động xử lý mọi session qua 4 bước Laya:
  Stage 1: Detect context/target mismatch (deterministic)
  Stage 2: Reconcile context using Laya + behavioral window
  Stage 3: Work / non-work classification (Laya)
  Stage 4: SOP phase assignment (Laya)

Usage:
  python laya_qc_service.py --daemon          # Chạy liên tục, poll mỗi 30s
  python laya_qc_service.py --once            # Xử lý tất cả pending rồi thoát
  python laya_qc_service.py --machine MR-QUAN # Xử lý lại 1 máy cụ thể
"""

import os
import sys
import json
import argparse
import sqlite3
import tempfile
import time
import io
import traceback
from datetime import datetime, timezone, timedelta
from concurrent.futures import ThreadPoolExecutor, as_completed

import urllib3
import zstandard
import tarfile
import boto3
import psycopg2
import psycopg2.extras

sys.stdout.reconfigure(encoding="utf-8")
urllib3.disable_warnings()

# Force unbuffered output for real-time logging
import functools
print = functools.partial(print, flush=True)

import laya

# ─────────────────────────────────────────────
# Config
# ─────────────────────────────────────────────
VN_TZ = timezone(timedelta(hours=7))
REPORT_ROOT = r"D:\tools GTF\trajectory recoder\report"

PG_DSN = dict(
    host="127.0.0.1", port=5432,
    user="trajectory", password="TrajectorySecurePgPass2026!",
    dbname="trajectory",
)

S3_CONF = dict(
    endpoint_url="https://127.0.0.1:9000",
    aws_access_key_id="trajectory-minio-admin",
    aws_secret_access_key="TrajectoryMinioSecurePass2026!",
    verify=False,
)

POLL_INTERVAL = 30  # seconds
MAX_WORKERS = 4     # parallel sessions

# ─────────────────────────────────────────────
# Stage 1: Deterministic mismatch detection
# ─────────────────────────────────────────────

# Maps UIA framework_id → expected process name.
# None = framework too generic to determine process.
FRAMEWORK_TO_PROCESS = {
    "Chrome":            "chrome.exe",
    "InternetExplorer":  "msedge.exe",
    "DOM":               "chrome.exe",
}

def stage1_detect_mismatch(action):
    """Compare target.framework_id vs context.process_name.
    Returns (is_mismatch: bool, expected_process: str|None).
    """
    fw = action.get("target_framework")
    ctx_proc = (action.get("context_process") or "").lower()

    if fw and fw in FRAMEWORK_TO_PROCESS:
        expected = FRAMEWORK_TO_PROCESS[fw]
        if expected and expected.lower() != ctx_proc:
            return True, expected
    return False, None


# ─────────────────────────────────────────────
# Stage 2: Reconcile context via Laya
# ─────────────────────────────────────────────

RECONCILE_QUESTION = {
    "real_app": {
        "type": "choice",
        "instructions": (
            "Dựa vào chuỗi hành vi xung quanh (cửa sổ trước/sau, loại thao tác, "
            "tên nút bấm), người dùng thực sự đang thao tác trên ứng dụng nào "
            "tại thời điểm này?"
        ),
        "criteria": {
            "chrome.exe":       "Trình duyệt web Google Chrome (Facebook, TikTok, Google, Shopee, YouTube...)",
            "msedge.exe":       "Trình duyệt Microsoft Edge",
            "CapCut.exe":       "Phần mềm dựng video CapCut (timeline, preview, export)",
            "Illustrator.exe":  "Adobe Illustrator (thiết kế vector, logo, banner)",
            "Photoshop.exe":    "Adobe Photoshop (chỉnh sửa ảnh, retouch)",
            "explorer.exe":     "Windows File Explorer (quản lý thư mục, file)",
            "Lark.exe":         "Ứng dụng Lark (nhắn tin, quản lý công việc nội bộ)",
            "OTHER":            "Ứng dụng khác không thuộc danh sách trên",
        },
    }
}


def stage2_reconcile(router, action, neighbors):
    """Use Laya to determine the real application when mismatch detected.
    `neighbors` is a list of up to 5 actions (2 before, current, 2 after).
    """
    # Build a context string from neighboring actions
    ctx_lines = []
    for i, nb in enumerate(neighbors):
        marker = ">>>" if nb is action else "   "
        ctx_lines.append(
            f"{marker} [{nb.get('action_type','?')}] "
            f"app={nb.get('context_process','?')} "
            f"window=\"{(nb.get('window_title') or '')[:60]}\" "
            f"target=\"{(nb.get('target_name') or '')[:60]}\""
        )

    state = {
        "action": action.get("action_type", ""),
        "target": action.get("target_name") or "",
        "context_process_claimed": action.get("context_process", ""),
        "target_framework": action.get("target_framework", ""),
        "window": action.get("window_title") or "",
        "behavioral_context": "\n".join(ctx_lines),
    }

    res = router.predict(state, RECONCILE_QUESTION)
    choice = res["answers"]["real_app"]["choice"]
    conf = res["answers"]["real_app"]["confidence"]
    return choice, round(conf, 3)


# ─────────────────────────────────────────────
# Stage 3 & 4: Work classification + SOP phase
# ─────────────────────────────────────────────

WORKFLOW_QUESTIONS = {
    "workflow_phase": {
        "type": "choice",
        "instructions": "Hành vi thao tác này thuộc giai đoạn nào trong quy trình làm việc?",
        "criteria": {
            "Phase_1_Ingestion":  "Thu thập nguyên liệu, mở thư mục Explorer, tải asset, đọc brief/kịch bản",
            "Phase_2_Execution":  "Thao tác phần mềm chính: cắt ghép video CapCut, chỉnh timeline, vẽ đồ họa, thiết lập quảng cáo",
            "Phase_3_Export":     "Xuất bản dự án, lưu file render, mở hộp thoại Export, nén dữ liệu",
            "Phase_4_Handoff":   "Bàn giao thành phẩm, tải file lên Drive/Lark, nhắn tin báo cáo hoàn thành",
        },
    },
    "is_work": {
        "type": "noul",
        "instructions": "Hành vi này có phải là thao tác công việc phục vụ dự án không (hay là lướt web/tin nhắn ngoài)?",
    },
    "task_transition": {
        "type": "choice",
        "instructions": "Trạng thái chuyển tiếp tác vụ tại thời điểm này là gì?",
        "criteria": {
            "IN_PROGRESS":     "Đang liên tục thực hiện trong cùng một tác vụ/clip",
            "TASK_COMPLETED":  "Vừa hoàn thành xong 1 sản phẩm video hoặc đóng dự án",
            "TASK_STARTED":    "Bắt đầu mở 1 file/dự án mới",
        },
    },
}


def stage3_4_classify(router, action):
    """Run Laya work classification + SOP phase in one pass."""
    state = {
        "app": action.get("reconciled_process") or action.get("context_process", ""),
        "window": action.get("window_title") or "",
        "action": action.get("action_type", ""),
        "target": action.get("target_name") or "",
    }
    res = router.predict(state, WORKFLOW_QUESTIONS)

    phase = res["answers"]["workflow_phase"]["choice"]
    phase_conf = round(res["answers"]["workflow_phase"]["confidence"], 3)
    is_work = round(res["answers"]["is_work"]["noul"], 3)
    transition = res["answers"]["task_transition"]["choice"]

    work_label = "WORK" if is_work >= 0.7 else ("AMBIGUOUS" if is_work >= 0.4 else "NON_WORK")

    return {
        "sop_phase": phase,
        "sop_phase_confidence": phase_conf,
        "is_work": is_work,
        "work_label": work_label,
        "task_transition": transition,
    }


# ─────────────────────────────────────────────
# Session data extraction (from MinIO)
# ─────────────────────────────────────────────

def get_s3():
    return boto3.client("s3", **S3_CONF)


def extract_actions_from_archive(storage_key):
    """Download session archive from MinIO, extract canonical_actions from session.db."""
    s3 = get_s3()
    obj = s3.get_object(Bucket="trajectory-archives", Key=storage_key)
    raw = obj["Body"].read()

    dctx = zstandard.ZstdDecompressor()
    sr = dctx.stream_reader(io.BytesIO(raw))
    tf = tarfile.open(fileobj=sr, mode="r|*")

    db_bytes = None
    for member in tf:
        if member.name == "session.db":
            db_bytes = tf.extractfile(member).read()
            break
    if not db_bytes:
        return []

    with tempfile.NamedTemporaryFile(delete=False, suffix=".db") as tmp:
        tmp.write(db_bytes)
        tmp_name = tmp.name

    conn = sqlite3.connect(tmp_name)
    cur = conn.cursor()
    cur.execute("""
        SELECT timestamp_utc, action_type, target_json, context_json, parameters_json
        FROM canonical_actions
        ORDER BY timestamp_utc ASC
    """)

    actions = []
    for idx, (ts_str, act_type, target_j, ctx_j, param_j) in enumerate(cur.fetchall()):
        target = json.loads(target_j) if target_j else {}
        ctx = json.loads(ctx_j) if ctx_j else {}

        proc = (
            ctx.get("process_name")
            or ctx.get("application", {}).get("process_name")
            or "unknown"
        )
        title = ctx.get("window_title") or ctx.get("window", {}).get("title") or ""

        actions.append({
            "action_index": idx,
            "timestamp": ts_str,
            "action_type": act_type,
            "context_process": proc,
            "window_title": title,
            "target_name": target.get("name"),
            "target_framework": target.get("framework_id"),
            "control_type": target.get("control_type"),
        })

    conn.close()
    try:
        os.unlink(tmp_name)
    except Exception:
        pass
    return actions


# ─────────────────────────────────────────────
# Main pipeline: process one session
# ─────────────────────────────────────────────

def process_session(session_id, machine_id, storage_key, router):
    """Run all 4 stages on a single session. Returns list of result rows."""
    ts = datetime.now(VN_TZ).strftime("%H:%M:%S")
    print(f"  [{ts}] Processing {machine_id} / {session_id[:20]}...")

    actions = extract_actions_from_archive(storage_key)
    if not actions:
        print(f"  [{ts}]   ⚠ No actions found in archive, skipping.")
        return []

    # Filter meaningful interactions (user-initiated only, not window lifecycle)
    MEANINGFUL_TYPES = {"Click", "DoubleClick", "RightClick", "DragDrop", "TypeText"}
    meaningful = [a for a in actions if a["action_type"] in MEANINGFUL_TYPES]
    print(f"  [{ts}]   {len(actions)} total actions → {len(meaningful)} meaningful")

    results = []
    mismatch_count = 0
    for i, action in enumerate(meaningful):
        # Progress logging
        if i > 0 and i % 100 == 0:
            elapsed = datetime.now(VN_TZ).strftime("%H:%M:%S")
            print(f"  [{elapsed}]   ... {i}/{len(meaningful)} actions processed ({mismatch_count} mismatches so far)")

        # Stage 1: Deterministic mismatch detection
        is_mismatch, expected_proc = stage1_detect_mismatch(action)
        if is_mismatch:
            mismatch_count += 1

        # Stage 2: Reconcile (only if mismatch detected)
        reconciled_process = action["context_process"]
        reconcile_conf = 1.0

        if is_mismatch:
            # Gather neighbors: 2 before, current, 2 after
            start = max(0, i - 2)
            end = min(len(meaningful), i + 3)
            neighbors = meaningful[start:end]

            choice, conf = stage2_reconcile(router, action, neighbors)
            if choice != "OTHER":
                reconciled_process = choice
                reconcile_conf = conf
            else:
                # Fallback to Stage 1's deterministic guess
                reconciled_process = expected_proc or action["context_process"]
                reconcile_conf = 0.5

        action["reconciled_process"] = reconciled_process

        # Stage 3 & 4: Work classification + SOP phase
        classification = stage3_4_classify(router, action)

        results.append({
            "session_id": session_id,
            "machine_id": machine_id,
            "action_index": action["action_index"],
            "timestamp_utc": action["timestamp"],
            "action_type": action["action_type"],
            "context_process": action["context_process"],
            "target_framework": action["target_framework"],
            "target_name": action["target_name"],
            "is_mismatch": is_mismatch,
            "reconciled_process": reconciled_process,
            "reconcile_confidence": reconcile_conf,
            **classification,
        })

    work_count = sum(1 for r in results if r["work_label"] == "WORK")
    ts2 = datetime.now(VN_TZ).strftime("%H:%M:%S")
    print(
        f"  [{ts2}]   ✓ {len(results)} actions analyzed | "
        f"{mismatch_count} mismatches | {work_count} work actions"
    )
    return results


# ─────────────────────────────────────────────
# Database operations
# ─────────────────────────────────────────────

def get_pg():
    return psycopg2.connect(**PG_DSN)


def get_pending_sessions(machine_filter=None):
    """Query sessions ready for Laya analysis."""
    conn = get_pg()
    cur = conn.cursor()

    sql = """
        SELECT s.session_id, s.machine_id, sc.storage_key
        FROM sessions s
        JOIN session_chunks sc ON s.session_id = sc.session_id
        WHERE sc.chunk_index = 0
          AND s.total_size_bytes > 100000
          AND (s.laya_status IS NULL OR s.laya_status = 'PENDING')
    """
    params = []
    if machine_filter:
        sql += " AND s.machine_id = %s"
        params.append(machine_filter)

    sql += " ORDER BY s.start_time_utc DESC"
    cur.execute(sql, params)
    rows = cur.fetchall()
    conn.close()
    return [(sid, mid, skey) for sid, mid, skey in rows]


def save_results(session_id, results):
    """Write analysis results to laya_action_analysis and update session status."""
    if not results:
        return

    conn = get_pg()
    cur = conn.cursor()

    # Mark session as processing
    cur.execute(
        "UPDATE sessions SET laya_status = 'PROCESSING' WHERE session_id = %s",
        (session_id,),
    )

    # Upsert results
    insert_sql = """
        INSERT INTO laya_action_analysis (
            session_id, machine_id, action_index, timestamp_utc, action_type,
            context_process, target_framework, target_name, is_mismatch,
            reconciled_process, reconcile_confidence,
            is_work, work_label,
            sop_phase, sop_phase_confidence, task_transition
        ) VALUES (
            %(session_id)s, %(machine_id)s, %(action_index)s, %(timestamp_utc)s, %(action_type)s,
            %(context_process)s, %(target_framework)s, %(target_name)s, %(is_mismatch)s,
            %(reconciled_process)s, %(reconcile_confidence)s,
            %(is_work)s, %(work_label)s,
            %(sop_phase)s, %(sop_phase_confidence)s, %(task_transition)s
        )
        ON CONFLICT (session_id, action_index) DO UPDATE SET
            context_process = EXCLUDED.context_process,
            target_framework = EXCLUDED.target_framework,
            target_name = EXCLUDED.target_name,
            is_mismatch = EXCLUDED.is_mismatch,
            reconciled_process = EXCLUDED.reconciled_process,
            reconcile_confidence = EXCLUDED.reconcile_confidence,
            is_work = EXCLUDED.is_work,
            work_label = EXCLUDED.work_label,
            sop_phase = EXCLUDED.sop_phase,
            sop_phase_confidence = EXCLUDED.sop_phase_confidence,
            task_transition = EXCLUDED.task_transition,
            analyzed_at = CURRENT_TIMESTAMP
    """
    psycopg2.extras.execute_batch(cur, insert_sql, results, page_size=500)

    # Mark session as completed
    cur.execute(
        "UPDATE sessions SET laya_status = 'COMPLETED', laya_analyzed_at = NOW() WHERE session_id = %s",
        (session_id,),
    )
    conn.commit()
    conn.close()


def mark_session_failed(session_id, error_msg):
    """Mark a session as failed."""
    conn = get_pg()
    cur = conn.cursor()
    cur.execute(
        "UPDATE sessions SET laya_status = 'FAILED' WHERE session_id = %s",
        (session_id,),
    )
    conn.commit()
    conn.close()


# ─────────────────────────────────────────────
# Worker: process one session end-to-end
# ─────────────────────────────────────────────

def worker(session_id, machine_id, storage_key, router):
    """Process a single session and save results. Returns success bool."""
    try:
        results = process_session(session_id, machine_id, storage_key, router)
        save_results(session_id, results)
        return True
    except Exception as e:
        ts = datetime.now(VN_TZ).strftime("%H:%M:%S")
        print(f"  [{ts}]   ✗ FAILED {machine_id}/{session_id[:20]}: {e}")
        traceback.print_exc()
        mark_session_failed(session_id, str(e))
        return False


# ─────────────────────────────────────────────
# Main loop
# ─────────────────────────────────────────────

def run_batch(machine_filter=None):
    """Process all pending sessions. Returns (success_count, fail_count)."""
    sessions = get_pending_sessions(machine_filter)
    if not sessions:
        return 0, 0

    ts = datetime.now(VN_TZ).strftime("%H:%M:%S")
    print(f"\n[{ts}] Found {len(sessions)} pending session(s) to analyze.")

    # Initialize Laya once — use GPU if available
    import torch
    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"[{ts}] Loading Laya Decision Router on {device.upper()}...")
    router = laya.Router(preload=True, device=device)
    print(f"[{ts}] Laya ready.\n")

    success = 0
    fail = 0

    # Process sessions sequentially — Laya Router (PyTorch) is not thread-safe
    for sid, mid, skey in sessions:
        ok = worker(sid, mid, skey, router)
        if ok:
            success += 1
        else:
            fail += 1

    ts = datetime.now(VN_TZ).strftime("%H:%M:%S")
    print(f"\n[{ts}] Batch complete: {success} succeeded, {fail} failed.")
    return success, fail


def daemon_loop():
    """Run continuously, polling every POLL_INTERVAL seconds."""
    print("=" * 60)
    print("  LAYA QC SERVICE — Daemon Mode")
    print(f"  Poll interval: {POLL_INTERVAL}s | Workers: {MAX_WORKERS}")
    print("=" * 60)

    while True:
        try:
            s, f = run_batch()
            if s == 0 and f == 0:
                ts = datetime.now(VN_TZ).strftime("%H:%M:%S")
                print(f"[{ts}] No pending sessions. Sleeping {POLL_INTERVAL}s...")
        except Exception as e:
            ts = datetime.now(VN_TZ).strftime("%H:%M:%S")
            print(f"[{ts}] Error in batch: {e}")
            traceback.print_exc()

        time.sleep(POLL_INTERVAL)


def main():
    parser = argparse.ArgumentParser(description="Laya Multi-Stage QC Service")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--daemon", action="store_true", help="Run continuously, poll every 30s")
    mode.add_argument("--once", action="store_true", help="Process all pending sessions then exit")
    mode.add_argument("--machine", type=str, help="Re-process all sessions for a specific machine")

    args = parser.parse_args()

    if args.daemon:
        daemon_loop()
    elif args.once:
        run_batch()
    elif args.machine:
        # Reset laya_status for this machine so we can reprocess
        conn = get_pg()
        cur = conn.cursor()
        cur.execute(
            "UPDATE sessions SET laya_status = 'PENDING' WHERE machine_id = %s",
            (args.machine,),
        )
        updated = cur.rowcount
        conn.commit()
        conn.close()
        print(f"Reset {updated} session(s) for machine {args.machine}")
        run_batch(machine_filter=args.machine)


if __name__ == "__main__":
    main()
