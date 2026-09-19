CREATE TABLE IF NOT EXISTS devices (
    id UUID PRIMARY KEY,
    device_id TEXT NOT NULL UNIQUE,
    public_key TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    label TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS pairings (
    id UUID PRIMARY KEY,
    initiator_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    claimant_device_id UUID REFERENCES devices(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK (status IN ('pending', 'accepted', 'revoked', 'expired')),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    accepted_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS pairings_initiator_idx ON pairings (initiator_device_id, status);
CREATE INDEX IF NOT EXISTS pairings_claimant_idx ON pairings (claimant_device_id, status);

CREATE TABLE IF NOT EXISTS sync_items (
    id UUID PRIMARY KEY,
    pairing_id UUID NOT NULL REFERENCES pairings(id) ON DELETE CASCADE,
    event_key TEXT NOT NULL,
    source_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    payload BYTEA NOT NULL,
    payload_nonce BYTEA NOT NULL,
    content_digest TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (pairing_id, event_key)
);

CREATE INDEX IF NOT EXISTS sync_items_pairing_revision_idx
    ON sync_items (pairing_id, revision, id);

CREATE TABLE IF NOT EXISTS sync_cursors (
    pairing_id UUID NOT NULL REFERENCES pairings(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    last_revision BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (pairing_id, device_id)
);

CREATE TABLE IF NOT EXISTS tombstones (
    id UUID PRIMARY KEY,
    pairing_id UUID NOT NULL REFERENCES pairings(id) ON DELETE CASCADE,
    event_key TEXT NOT NULL,
    source_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (pairing_id, event_key)
);

CREATE TABLE IF NOT EXISTS audit_events (
    id BIGSERIAL PRIMARY KEY,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS audit_events_created_at_idx ON audit_events (created_at DESC);
