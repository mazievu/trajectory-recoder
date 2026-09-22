"""
Database and S3 storage access layer for Trajectory MCP Server.
Fetches metadata from PostgreSQL and extracts session SQLite databases from MinIO S3.
"""

import os
import sqlite3
import subprocess
import psycopg2
import boto3
import urllib3
import logging

urllib3.disable_warnings()
logger = logging.getLogger("trajectory.mcp.db")

PG_CONFIG = {
    "dbname": "trajectory",
    "user": "trajectory",
    "password": "TrajectorySecurePgPass2026!",
    "host": os.environ.get("POSTGRES_HOST", "localhost"),
    "port": int(os.environ.get("POSTGRES_PORT", 5432)),
}

MINIO_CONFIG = {
    "endpoint_url": os.environ.get("MINIO_ENDPOINT", "https://127.0.0.1:9000"),
    "aws_access_key_id": os.environ.get("MINIO_ROOT_USER", "trajectory-minio-admin"),
    "aws_secret_access_key": os.environ.get("MINIO_ROOT_PASSWORD", "TrajectoryMinioSecurePass2026!"),
    "verify": False,
}

CACHE_DIR = os.path.join(os.path.dirname(__file__), "cache")
os.makedirs(CACHE_DIR, exist_ok=True)

def get_pg():
    return psycopg2.connect(**PG_CONFIG)

def get_s3():
    return boto3.client("s3", **MINIO_CONFIG)

def list_machines_db():
    with get_pg() as conn:
        with conn.cursor() as cur:
            cur.execute("""
                SELECT m.machine_id, m.hostname, m.status, m.last_heartbeat_at,
                       COUNT(s.session_id) as total_sessions
                FROM machines m
                LEFT JOIN sessions s ON m.machine_id = s.machine_id
                GROUP BY m.machine_id, m.hostname, m.status, m.last_heartbeat_at
                ORDER BY m.last_heartbeat_at DESC;
            """)
            rows = cur.fetchall()
            return [
                {
                    "machine_id": r[0],
                    "hostname": r[1],
                    "status": r[2],
                    "last_heartbeat": str(r[3]),
                    "total_sessions": r[4],
                }
                for r in rows
            ]

def get_sessions_for_machine(machine_id: str, date_prefix: str = None):
    with get_pg() as conn:
        with conn.cursor() as cur:
            query = """
                SELECT s.session_id, s.start_time_utc, s.end_time_utc, s.total_size_bytes, s.status, c.storage_key
                FROM sessions s
                JOIN session_chunks c ON s.session_id = c.session_id
                WHERE s.machine_id = %s
            """
            params = [machine_id]
            if date_prefix:
                query += " AND s.session_id LIKE %s"
                params.append(f"%{date_prefix}%")
            query += " ORDER BY s.start_time_utc ASC;"
            cur.execute(query, params)
            return [
                {
                    "session_id": r[0],
                    "start_time": str(r[1]),
                    "end_time": str(r[2]),
                    "size_bytes": r[3],
                    "status": r[4],
                    "storage_key": r[5],
                }
                for r in cur.fetchall()
            ]

def ensure_session_db(session_id: str, storage_key: str) -> str:
    """Downloads chunk from MinIO and extracts session.db into local cache if not present."""
    session_cache_dir = os.path.join(CACHE_DIR, session_id)
    db_file = os.path.join(session_cache_dir, "session.db")
    if os.path.exists(db_file):
        return db_file

    os.makedirs(session_cache_dir, exist_ok=True)
    try:
        s3 = get_s3()
        obj = s3.get_object(Bucket="trajectory-archives", Key=storage_key)
        chunk_bytes = obj["Body"].read()

        tar_zst = os.path.join(session_cache_dir, "session.tar.zst")
        with open(tar_zst, "wb") as f:
            f.write(chunk_bytes)

        # Decompress using Windows tar with built-in libzstd
        subprocess.run(["tar", "-xf", tar_zst, "-C", session_cache_dir], capture_output=True)

        if os.path.exists(tar_zst):
            try:
                os.remove(tar_zst)
            except Exception:
                pass

        if os.path.exists(db_file):
            return db_file
    except Exception as e:
        logger.error(f"Error extracting session {session_id}: {e}")

    return None

def get_raw_actions_from_sqlite(db_file: str, limit: int = 1500) -> list:
    if not db_file or not os.path.exists(db_file):
        return []
    try:
        conn = sqlite3.connect(db_file)
        cur = conn.cursor()
        cur.execute("""
            SELECT global_event_id, session_event_id, timestamp_utc, timestamp_monotonic_ns,
                   action_type, confidence, target_json, context_json, parameters_json, duration_ms
            FROM canonical_actions
            ORDER BY timestamp_monotonic_ns ASC
            LIMIT ?;
        """, (limit,))
        rows = cur.fetchall()
        conn.close()
        return [
            {
                "global_event_id": r[0],
                "session_event_id": r[1],
                "timestamp_utc": r[2],
                "timestamp_monotonic_ns": r[3],
                "action_type": r[4],
                "confidence": r[5],
                "target_json": r[6],
                "context_json": r[7],
                "parameters_json": r[8],
                "duration_ms": r[9],
            }
            for r in rows
        ]
    except Exception as e:
        logger.error(f"SQLite read error on {db_file}: {e}")
        return []
