ALTER TABLE users ADD COLUMN webauthn_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE;

CREATE TABLE user_passkeys (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id BYTEA NOT NULL UNIQUE,
    passkey JSONB NOT NULL,
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    aaguid UUID,
    backup_eligible BOOLEAN NOT NULL,
    backed_up BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX idx_user_passkeys_user ON user_passkeys (user_id);
