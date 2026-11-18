CREATE TABLE known_devices (
    id BIGSERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT known_devices_user_token UNIQUE (user_id, token_hash)
);
CREATE INDEX known_devices_last_seen_at ON known_devices (last_seen_at);

CREATE TABLE notification_opt_outs (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    topic TEXT NOT NULL CHECK (topic IN ('sign_in_alerts')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, topic)
);

ALTER TABLE account_events DROP CONSTRAINT account_events_kind_check;
ALTER TABLE account_events ADD CONSTRAINT account_events_kind_check CHECK (
    kind IN (
        'account_created',
        'account_disabled',
        'account_enabled',
        'admin_granted',
        'admin_revoked',
        'api_token_created',
        'api_token_revoked',
        'device_signed_out',
        'email_change_requested',
        'email_changed',
        'email_verified',
        'notifications_updated',
        'other_devices_signed_out',
        'passkey_added',
        'passkey_prompt_declined',
        'passkey_prompt_snoozed',
        'passkey_removed',
        'passkey_renamed',
        'password_changed',
        'password_reset',
        'profile_updated',
        'signed_in',
        'signed_out_everywhere',
        'time_zone_changed',
        'verification_resent'
    )
);
