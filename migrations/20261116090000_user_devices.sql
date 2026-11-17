CREATE TABLE user_devices (
    id SERIAL PRIMARY KEY,
    public_id TEXT NOT NULL UNIQUE CONSTRAINT user_devices_public_id_format CHECK (public_id ~ '^dev_[A-Za-z0-9]{20}$'),
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_version INTEGER NOT NULL,
    user_agent TEXT NOT NULL CHECK (char_length(user_agent) <= 512),
    ip TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_user_devices_user ON user_devices (user_id, last_seen_at DESC);
CREATE INDEX idx_user_devices_last_seen ON user_devices (last_seen_at);

CREATE INDEX idx_account_events_user_recent ON account_events (user_id, created_at DESC);

ALTER TABLE account_events DROP CONSTRAINT account_events_kind_check;
ALTER TABLE account_events ADD CONSTRAINT account_events_kind_check CHECK (
    kind IN (
        'account_created',
        'api_token_created',
        'api_token_revoked',
        'device_signed_out',
        'email_change_requested',
        'email_changed',
        'email_verified',
        'other_devices_signed_out',
        'passkey_added',
        'passkey_prompt_declined',
        'passkey_prompt_snoozed',
        'passkey_removed',
        'passkey_renamed',
        'password_changed',
        'password_reset',
        'profile_updated',
        'signed_in'
    )
);
