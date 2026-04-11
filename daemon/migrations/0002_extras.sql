-- Resource limits stored per app
ALTER TABLE apps ADD COLUMN IF NOT EXISTS memory_mb   INT     DEFAULT NULL;
ALTER TABLE apps ADD COLUMN IF NOT EXISTS cpu_shares  INT     DEFAULT NULL;
ALTER TABLE apps ADD COLUMN IF NOT EXISTS cpu_quota   INT     DEFAULT NULL;

-- Webhooks
CREATE TABLE IF NOT EXISTS webhooks (
    id       UUID PRIMARY KEY,
    app_id   UUID NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    url      TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- App tokens
CREATE TABLE IF NOT EXISTS app_tokens (
    id         UUID PRIMARY KEY,
    app_id     UUID NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    token      TEXT NOT NULL UNIQUE,
    label      TEXT NOT NULL DEFAULT 'default',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);