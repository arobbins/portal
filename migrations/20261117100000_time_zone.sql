ALTER TABLE users ADD COLUMN time_zone TEXT CHECK (time_zone IS NULL OR length(time_zone) BETWEEN 1 AND 64);

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
        'time_zone_changed',
        'verification_resent'
    )
);
