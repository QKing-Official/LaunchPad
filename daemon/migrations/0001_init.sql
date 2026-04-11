CREATE TABLE IF NOT EXISTS apps (
    id          UUID        PRIMARY KEY,
    name        TEXT        NOT NULL UNIQUE,
    image       TEXT        NOT NULL,
    status      TEXT        NOT NULL DEFAULT 'pending',
    container_id TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS port_mappings (
    id            UUID    PRIMARY KEY,
    app_id        UUID    NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    internal_port INT     NOT NULL,
    external_port INT     NOT NULL,
    UNIQUE(app_id, internal_port)
);