CREATE TABLE api_tokens (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    token_hash TEXT NOT NULL UNIQUE,
    last_four TEXT NOT NULL CHECK (char_length(last_four) = 4),
    scopes TEXT[] NOT NULL CHECK (
        cardinality(scopes) > 0
        AND scopes <@ ARRAY['profile:read', 'profile:write']::TEXT[]
    ),
    audience TEXT NOT NULL CHECK (audience IN ('api')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ,
    CHECK (expires_at > created_at)
);

CREATE INDEX idx_api_tokens_user ON api_tokens (user_id);

CREATE TABLE security_events (
    id BIGSERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (
        kind IN ('api_token_created', 'api_token_revoked', 'api_tokens_revoked_by_password_reset')
    ),
    detail TEXT NOT NULL DEFAULT '',
    ip TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_security_events_user ON security_events (user_id, created_at);
