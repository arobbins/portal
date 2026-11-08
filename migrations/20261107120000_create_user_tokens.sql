CREATE TABLE user_tokens (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    purpose TEXT NOT NULL CHECK (purpose IN ('email_verification', 'password_reset', 'email_change')),
    token_hash TEXT NOT NULL UNIQUE,
    payload TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_user_tokens_user_purpose ON user_tokens (user_id, purpose);

INSERT INTO user_tokens (user_id, purpose, token_hash, expires_at, created_at)
SELECT user_id, 'email_verification', token_hash, expires_at, created_at
FROM email_verification_tokens
ON CONFLICT (token_hash) DO NOTHING;

DROP TABLE email_verification_tokens;
