"""
Automated verification test suite for Trajectory MCP Server.
"""

import sys
import os
import time

sys.stdout.reconfigure(encoding="utf-8")
sys.path.insert(0, os.path.dirname(__file__))

from server import (
    list_active_machines,
    get_daily_summary,
    inspect_timeline,
    search_actions,
    get_productivity_metrics,
)

def run_tests():
    print("=================================================================")
    print("       TRAJECTORY MCP SERVER - AUTOMATED VERIFICATION SUITE       ")
    print("=================================================================\n")

    # Test 1: list_active_machines
    t0 = time.time()
    print("[1/5] Testing list_active_machines()...")
    machines_res = list_active_machines()
    assert "Machine ID" in machines_res, "Should contain header"
    assert "ONLINE" in machines_res, "Should detect online machines"
    print(f"      Passed in {time.time() - t0:.2f}s!")
    print(machines_res)
    print("-" * 65)

    # Test 2: get_daily_summary
    t0 = time.time()
    print("[2/5] Testing get_daily_summary('TESTER-PC', '2026-09-11')...")
    summary = get_daily_summary("TESTER-PC", "2026-09-11")
    tokens_est = len(summary) // 4
    assert "Báo Cáo Hoạt Động Hàng Ngày" in summary
    assert "Dòng thời gian làm việc" in summary
    assert tokens_est <= 1500, f"Token count should be <= 1500, got {tokens_est}"
    print(f"      Passed in {time.time() - t0:.2f}s!")
    print(f"      Character length: {len(summary)} | Estimated tokens: ~{tokens_est}")
    print("-" * 65)

    # Test 3: inspect_timeline
    t0 = time.time()
    print("[3/5] Testing inspect_timeline('TESTER-PC', limit=5)...")
    timeline = inspect_timeline("TESTER-PC", limit=5)
    assert "Chi Tiết Dòng Thời Gian" in timeline
    print(f"      Passed in {time.time() - t0:.2f}s!")
    print(timeline)
    print("-" * 65)

    # Test 4: search_actions
    t0 = time.time()
    print("[4/5] Testing search_actions('Word', 'TESTER-PC', '2026-09-11', limit=3)...")
    search_res = search_actions("Word", "TESTER-PC", "2026-09-11", limit=3)
    assert "WINWORD" in search_res or "Word" in search_res
    print(f"      Passed in {time.time() - t0:.2f}s!")
    print(search_res)
    print("-" * 65)

    # Test 5: get_productivity_metrics
    t0 = time.time()
    print("[5/5] Testing get_productivity_metrics('TESTER-PC', '2026-09-11')...")
    metrics_res = get_productivity_metrics("TESTER-PC", "2026-09-11")
    assert "Phân Tích Hiệu Suất" in metrics_res
    assert "Context Switching" in metrics_res
    print(f"      Passed in {time.time() - t0:.2f}s!")
    print(metrics_res)
    print("-" * 65)

    print("\n=================================================================")
    print(" [ALL 5 TESTS PASSED] MCP Server is fully operational & ready!   ")
    print("=================================================================")

if __name__ == "__main__":
    run_tests()
