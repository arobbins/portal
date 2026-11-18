CREATE TABLE api_feedback (
    id BIGSERIAL PRIMARY KEY,
    public_id TEXT NOT NULL UNIQUE CHECK (public_id ~ '^fbk_[A-Za-z0-9]{20}$'),
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_public_id TEXT,
    category TEXT NOT NULL CHECK (
        category IN ('confusing_docs', 'unexpected_error', 'missing_capability', 'wrong_schema', 'other')
    ),
    endpoint TEXT,
    problem_code TEXT,
    request_id TEXT,
    message TEXT NOT NULL CHECK (char_length(message) BETWEEN 1 AND 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX api_feedback_user_id ON api_feedback (user_id);
CREATE INDEX api_feedback_created_at ON api_feedback (created_at DESC);

ALTER TABLE account_events DROP CONSTRAINT account_events_kind_check;
ALTER TABLE account_events ADD CONSTRAINT account_events_kind_check CHECK (
    kind IN (
        'account_created',
        'account_disabled',
        'account_enabled',
        'admin_granted',
        'admin_revoked',
        'api_feedback_sent',
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
