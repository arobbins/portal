CREATE TABLE oauth_clients (
    id SERIAL PRIMARY KEY,
    client_id TEXT NOT NULL UNIQUE CHECK (
        client_id ~ '^cli_[A-Za-z0-9]{20}$' OR (client_id LIKE 'https://%' AND length(client_id) <= 2048)
    ),
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    redirect_uris TEXT[] NOT NULL CHECK (cardinality(redirect_uris) BETWEEN 1 AND 10),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    fetched_at TIMESTAMPTZ,
    approved_at TIMESTAMPTZ
);

CREATE INDEX idx_oauth_clients_unapproved ON oauth_clients (created_at) WHERE approved_at IS NULL;

CREATE TABLE oauth_requests (
    id SERIAL PRIMARY KEY,
    public_id TEXT NOT NULL UNIQUE CHECK (public_id ~ '^oar_[A-Za-z0-9]{20}$'),
    client_id INTEGER NOT NULL REFERENCES oauth_clients (id) ON DELETE CASCADE,
    redirect_uri TEXT NOT NULL,
    scopes TEXT[] NOT NULL,
    state TEXT CHECK (length(state) <= 1024),
    code_challenge TEXT NOT NULL CHECK (code_challenge ~ '^[A-Za-z0-9_-]{43}$'),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved')),
    user_id INTEGER REFERENCES users (id) ON DELETE CASCADE,
    connection_id INTEGER,
    code_hash TEXT UNIQUE,
    acted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((status = 'approved') = (code_hash IS NOT NULL AND user_id IS NOT NULL AND connection_id IS NOT NULL))
);

CREATE INDEX idx_oauth_requests_client ON oauth_requests (client_id);
CREATE INDEX idx_oauth_requests_user ON oauth_requests (user_id);
CREATE INDEX idx_oauth_requests_acted ON oauth_requests (acted_at);

CREATE TABLE oauth_connections (
    id SERIAL PRIMARY KEY,
    public_id TEXT NOT NULL UNIQUE CHECK (public_id ~ '^app_[A-Za-z0-9]{20}$'),
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    client_id INTEGER NOT NULL REFERENCES oauth_clients (id) ON DELETE CASCADE,
    return_host TEXT NOT NULL,
    scopes TEXT[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ,
    UNIQUE (user_id, client_id)
);

CREATE INDEX idx_oauth_connections_client ON oauth_connections (client_id);

ALTER TABLE oauth_requests
    ADD CONSTRAINT oauth_requests_connection_id_fkey
    FOREIGN KEY (connection_id) REFERENCES oauth_connections (id) ON DELETE CASCADE;

CREATE TABLE oauth_tokens (
    id SERIAL PRIMARY KEY,
    connection_id INTEGER NOT NULL REFERENCES oauth_connections (id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('access', 'refresh')),
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    replaced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (kind = 'refresh' OR replaced_at IS NULL)
);

CREATE INDEX idx_oauth_tokens_connection ON oauth_tokens (connection_id);
CREATE INDEX idx_oauth_tokens_expires ON oauth_tokens (expires_at);

ALTER TABLE account_events DROP CONSTRAINT account_events_channel_check;
ALTER TABLE account_events ADD CONSTRAINT account_events_channel_check CHECK (
    channel IN ('browser', 'api_token', 'app', 'admin', 'console', 'stripe')
);

ALTER TABLE account_events DROP CONSTRAINT account_events_kind_check;
ALTER TABLE account_events ADD CONSTRAINT account_events_kind_check CHECK (
    kind IN (
        'account_created',
        'account_disabled',
        'account_enabled',
        'admin_granted',
        'admin_revoked',
        'api_feedback_sent',
        'app_connected',
        'app_disconnected',
        'data_exported',
        'api_token_created',
        'api_token_revoked',
        'checkout_started',
        'device_signed_out',
        'duplicate_subscription_refunded',
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
        'payment_failed',
        'plan_changed',
        'profile_updated',
        'recovery_code_used',
        'recovery_codes_regenerated',
        'signed_in',
        'signed_out_everywhere',
        'subscribed',
        'subscription_cancelled',
        'subscription_ended',
        'time_zone_changed',
        'two_factor_disabled',
        'two_factor_enabled',
        'verification_resent'
    )
);

ALTER TABLE user_tokens DROP CONSTRAINT user_tokens_payload_check;
ALTER TABLE user_tokens ADD CONSTRAINT user_tokens_payload_check CHECK (
    CASE purpose
        WHEN 'email_change' THEN payload IS NOT NULL
        WHEN 'email_verification' THEN payload IS NULL
            OR payload ~ '^price_[A-Za-z0-9_]{1,249}$'
            OR payload ~ '^oar_[A-Za-z0-9]{20}$'
        ELSE payload IS NULL
    END
);
