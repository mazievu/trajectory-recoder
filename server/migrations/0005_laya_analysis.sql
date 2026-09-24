-- 0005_laya_analysis.sql
-- Laya multi-stage QC analysis results.

CREATE TABLE IF NOT EXISTS laya_action_analysis (
    id BIGSERIAL PRIMARY KEY,
    session_id VARCHAR(128) NOT NULL REFERENCES sessions(session_id),
    machine_id VARCHAR(64) NOT NULL,
    action_index INTEGER NOT NULL,
    timestamp_utc TIMESTAMP WITH TIME ZONE,
    action_type VARCHAR(64),

    -- Stage 1: Mismatch detection
    context_process VARCHAR(255),
    target_framework VARCHAR(64),
    target_name TEXT,
    is_mismatch BOOLEAN DEFAULT FALSE,

    -- Stage 2: Reconciled context
    reconciled_process VARCHAR(255),
    reconcile_confidence REAL,

    -- Stage 3: Work classification
    is_work REAL,
    work_label VARCHAR(32),

    -- Stage 4: SOP Phase
    sop_phase VARCHAR(64),
    sop_phase_confidence REAL,
    task_transition VARCHAR(32),

    analyzed_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(session_id, action_index)
);

CREATE INDEX IF NOT EXISTS idx_laya_session ON laya_action_analysis(session_id);
CREATE INDEX IF NOT EXISTS idx_laya_machine ON laya_action_analysis(machine_id);
CREATE INDEX IF NOT EXISTS idx_laya_mismatch ON laya_action_analysis(is_mismatch) WHERE is_mismatch = TRUE;

-- Track Laya processing status on sessions
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS laya_status VARCHAR(32) DEFAULT 'PENDING';
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS laya_analyzed_at TIMESTAMP WITH TIME ZONE;
