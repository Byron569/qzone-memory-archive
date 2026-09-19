-- Account-level auto sync: route encrypted changes by target QQ account,
-- keep the legacy pairing-based flow intact for older clients.

ALTER TABLE devices ADD COLUMN IF NOT EXISTS account_uin TEXT;
CREATE INDEX IF NOT EXISTS devices_account_uin_idx ON devices (account_uin, created_at);

ALTER TABLE sync_items ADD COLUMN IF NOT EXISTS target_uin TEXT;
ALTER TABLE sync_items ALTER COLUMN pairing_id DROP NOT NULL;
DROP INDEX IF EXISTS sync_items_target_event_key_idx;
CREATE UNIQUE INDEX IF NOT EXISTS sync_items_target_event_key_idx
    ON sync_items (target_uin, event_key);
CREATE INDEX IF NOT EXISTS sync_items_target_stream_idx
    ON sync_items (target_uin, stream_position) WHERE target_uin IS NOT NULL AND stream_position > 0;

ALTER TABLE tombstones ADD COLUMN IF NOT EXISTS target_uin TEXT;
DROP INDEX IF EXISTS tombstones_target_event_key_idx;
CREATE UNIQUE INDEX IF NOT EXISTS tombstones_target_event_key_idx
    ON tombstones (target_uin, event_key);

CREATE TABLE IF NOT EXISTS account_streams (
    account_uin TEXT PRIMARY KEY,
    next_position BIGINT NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS account_sync_cursors (
    account_uin TEXT NOT NULL,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    last_position BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (account_uin, device_id)
);

CREATE TABLE IF NOT EXISTS account_public_keys (
    account_uin TEXT NOT NULL,
    key_version INTEGER NOT NULL,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    public_key TEXT NOT NULL,
    label TEXT,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (account_uin, key_version)
);

CREATE INDEX IF NOT EXISTS account_public_keys_device_idx ON account_public_keys (device_id);
