-- Machine auto-restart control: allows server to enable/disable
-- client-side auto-restart watchdog on a per-machine basis.

ALTER TABLE machines
    ADD COLUMN IF NOT EXISTS auto_restart BOOLEAN NOT NULL DEFAULT TRUE;
