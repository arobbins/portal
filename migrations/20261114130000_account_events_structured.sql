ALTER TABLE account_events DROP CONSTRAINT account_events_kind_check;

UPDATE account_events
SET kind = 'password_reset'
WHERE kind = 'api_tokens_revoked_by_password_reset';

ALTER TABLE account_events ADD CONSTRAINT account_events_kind_check CHECK (
    kind IN (
        'api_token_created',
        'api_token_revoked',
        'email_verified',
        'passkey_added',
        'passkey_removed',
        'passkey_renamed',
        'password_changed',
        'password_reset',
        'profile_updated'
    )
);

ALTER TABLE account_events ALTER COLUMN detail DROP DEFAULT;
ALTER TABLE account_events ALTER COLUMN detail TYPE JSONB USING (
    CASE
        WHEN kind = 'password_reset' THEN '{}'::JSONB
        WHEN detail = '' THEN '{}'::JSONB
        ELSE jsonb_build_object('note', detail)
    END
);
ALTER TABLE account_events ALTER COLUMN detail SET DEFAULT '{}'::JSONB;
ALTER TABLE account_events ADD CONSTRAINT account_events_detail_object CHECK (jsonb_typeof(detail) = 'object');
