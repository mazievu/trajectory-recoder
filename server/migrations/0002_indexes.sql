-- 0002_indexes.sql
-- High-throughput query performance indexes.

CREATE INDEX IF NOT EXISTS idx_sessions_machine_id ON sessions(machine_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);
CREATE INDEX IF NOT EXISTS idx_chunks_session ON session_chunks(session_id);
CREATE INDEX IF NOT EXISTS idx_heartbeats_machine ON machine_heartbeats(machine_id, received_at DESC);
