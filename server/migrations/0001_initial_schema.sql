-- 0001_initial_schema.sql
-- Ingestion Server relational schema for machines, sessions, chunks, and heartbeats.

CREATE TABLE IF NOT EXISTS machines (
    machine_id VARCHAR(64) PRIMARY KEY,
    hostname VARCHAR(255) NOT NULL,
    os_version VARCHAR(255) NOT NULL,
    registration_token VARCHAR(255) NOT NULL,
    registered_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_heartbeat_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(32) DEFAULT 'ACTIVE'
);

CREATE TABLE IF NOT EXISTS sessions (
    session_id VARCHAR(128) PRIMARY KEY,
    machine_id VARCHAR(64) NOT NULL REFERENCES machines(machine_id),
    user_id VARCHAR(64) NOT NULL,
    start_time_utc TIMESTAMP WITH TIME ZONE NOT NULL,
    end_time_utc TIMESTAMP WITH TIME ZONE,
    status VARCHAR(32) NOT NULL DEFAULT 'INITIATED', -- INITIATED, UPLOADING, ACCEPTED, REJECTED, FAILED
    expected_chunks INTEGER NOT NULL,
    received_chunks INTEGER DEFAULT 0,
    total_size_bytes BIGINT NOT NULL,
    archive_sha256 VARCHAR(64) NOT NULL,
    verified_sha256 BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE IF NOT EXISTS session_chunks (
    session_id VARCHAR(128) NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    byte_size INTEGER NOT NULL,
    sha256 VARCHAR(64) NOT NULL,
    storage_key VARCHAR(512) NOT NULL,
    uploaded_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (session_id, chunk_index)
);

CREATE TABLE IF NOT EXISTS machine_heartbeats (
    heartbeat_id BIGSERIAL PRIMARY KEY,
    machine_id VARCHAR(64) NOT NULL REFERENCES machines(machine_id) ON DELETE CASCADE,
    disk_usage_pct REAL NOT NULL,
    active_session_id VARCHAR(128),
    received_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
