CREATE FUNCTION random_base62(length INTEGER) RETURNS TEXT
LANGUAGE sql VOLATILE AS $$
    SELECT string_agg(
        substr('0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz', 1 + floor(random() * 62)::INTEGER, 1),
        ''
    )
    FROM generate_series(1, length)
$$;

ALTER TABLE users ADD COLUMN public_id TEXT;
UPDATE users SET public_id = 'usr_' || random_base62(20);
ALTER TABLE users
    ALTER COLUMN public_id SET NOT NULL,
    ALTER COLUMN public_id SET DEFAULT 'usr_' || random_base62(20),
    ADD CONSTRAINT users_public_id_key UNIQUE (public_id),
    ADD CONSTRAINT users_public_id_format CHECK (public_id ~ '^usr_[A-Za-z0-9]{20}$');

ALTER TABLE users ADD COLUMN disabled_at TIMESTAMPTZ;

ALTER TABLE account_events DROP CONSTRAINT account_events_channel_check;
ALTER TABLE account_events ADD CONSTRAINT account_events_channel_check CHECK (
    channel IN ('browser', 'api_token', 'admin', 'console')
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
        'verification_resent'
    )
);
