ALTER TABLE security_events RENAME TO account_events;
ALTER SEQUENCE security_events_id_seq RENAME TO account_events_id_seq;
ALTER INDEX security_events_pkey RENAME TO account_events_pkey;
ALTER INDEX idx_security_events_user RENAME TO idx_account_events_user;
ALTER TABLE account_events RENAME CONSTRAINT security_events_user_id_fkey TO account_events_user_id_fkey;

ALTER TABLE account_events DROP CONSTRAINT security_events_kind_check;
ALTER TABLE account_events ADD CONSTRAINT account_events_kind_check CHECK (
    kind IN (
        'api_token_created',
        'api_token_revoked',
        'api_tokens_revoked_by_password_reset',
        'profile_updated'
    )
);

ALTER TABLE account_events
    ADD COLUMN channel TEXT NOT NULL DEFAULT 'browser' CHECK (channel IN ('browser', 'api_token')),
    ADD COLUMN api_token_id INTEGER,
    ADD CONSTRAINT account_events_channel_token CHECK ((channel = 'api_token') = (api_token_id IS NOT NULL));
