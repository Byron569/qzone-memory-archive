CREATE TABLE IF NOT EXISTS pairing_streams (
    pairing_id UUID PRIMARY KEY REFERENCES pairings(id) ON DELETE CASCADE,
    next_position BIGINT NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE sync_items
    ADD COLUMN IF NOT EXISTS operation TEXT NOT NULL DEFAULT 'upsert',
    ADD COLUMN IF NOT EXISTS key_version INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS aad BYTEA,
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS stream_position BIGINT NOT NULL DEFAULT 0;

ALTER TABLE sync_items
    DROP CONSTRAINT IF EXISTS sync_items_operation_check;

ALTER TABLE sync_items
    ADD CONSTRAINT sync_items_operation_check
    CHECK (operation IN ('upsert', 'tombstone'));

CREATE UNIQUE INDEX IF NOT EXISTS sync_items_pairing_stream_position_idx
    ON sync_items (pairing_id, stream_position)
    WHERE stream_position > 0;

INSERT INTO pairing_streams (pairing_id, next_position)
SELECT pairings.id, COALESCE(MAX(sync_items.stream_position), 0) + 1
FROM pairings
LEFT JOIN sync_items ON sync_items.pairing_id = pairings.id
WHERE pairings.status = 'accepted'
GROUP BY pairings.id
ON CONFLICT (pairing_id) DO NOTHING;
