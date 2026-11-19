CREATE TABLE newsletter_opt_outs (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    newsletter TEXT NOT NULL CHECK (newsletter IN ('general_newsletter', 'product_updates')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, newsletter)
);

INSERT INTO newsletter_opt_outs (user_id, newsletter)
SELECT users.id, lists.newsletter
FROM users
CROSS JOIN (VALUES ('general_newsletter'), ('product_updates')) AS lists (newsletter);

ALTER TABLE account_events DROP CONSTRAINT account_events_kind_check;
ALTER TABLE account_events ADD CONSTRAINT account_events_kind_check CHECK (
    kind IN (
        'account_created',
        'account_disabled',
        'account_enabled',
        'admin_granted',
        'admin_revoked',
        'api_feedback_sent',
        'data_exported',
        'api_token_created',
        'api_token_revoked',
        'device_signed_out',
        'email_change_requested',
        'email_changed',
        'email_verified',
        'newsletter_subscribed',
        'newsletter_unsubscribed',
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
