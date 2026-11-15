ALTER TABLE user_tokens ADD CONSTRAINT user_tokens_payload_check CHECK (
    (purpose = 'email_change') = (payload IS NOT NULL)
);

ALTER TABLE account_events DROP CONSTRAINT account_events_kind_check;
ALTER TABLE account_events ADD CONSTRAINT account_events_kind_check CHECK (
    kind IN (
        'account_created',
        'api_token_created',
        'api_token_revoked',
        'email_change_requested',
        'email_changed',
        'email_verified',
        'passkey_added',
        'passkey_prompt_declined',
        'passkey_prompt_snoozed',
        'passkey_removed',
        'passkey_renamed',
        'password_changed',
        'password_reset',
        'profile_updated'
    )
);
