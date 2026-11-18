CREATE TABLE user_two_factor (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    secret TEXT NOT NULL CHECK (secret ~ '^[A-Z2-7]{32}$'),
    last_used_step BIGINT NOT NULL DEFAULT 0,
    enabled_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE recovery_codes (
    id BIGSERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    used_at TIMESTAMPTZ,
    CONSTRAINT recovery_codes_user_code UNIQUE (user_id, code_hash)
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
        'recovery_code_used',
        'recovery_codes_regenerated',
        'signed_in',
        'signed_out_everywhere',
        'time_zone_changed',
        'two_factor_disabled',
        'two_factor_enabled',
        'verification_resent'
    )
);
