ALTER TABLE pairings
    ADD COLUMN IF NOT EXISTS initiator_public_key TEXT,
    ADD COLUMN IF NOT EXISTS claimant_public_key TEXT;

UPDATE pairings
SET initiator_public_key = devices.public_key
FROM devices
WHERE pairings.initiator_device_id = devices.id
  AND pairings.initiator_public_key IS NULL;
