-- Dashboard presence state. `online_since_at` represents the current
-- continuous heartbeat interval, not the machine's lifetime registration.

ALTER TABLE machines
    ADD COLUMN IF NOT EXISTS online_since_at TIMESTAMP WITH TIME ZONE;

UPDATE machines
SET online_since_at = COALESCE(online_since_at, last_heartbeat_at, registered_at, CURRENT_TIMESTAMP)
WHERE online_since_at IS NULL;

ALTER TABLE machines
    ALTER COLUMN online_since_at SET DEFAULT CURRENT_TIMESTAMP,
    ALTER COLUMN online_since_at SET NOT NULL;
