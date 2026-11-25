CREATE TABLE billing_customers (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    stripe_customer_id TEXT NOT NULL UNIQUE CHECK (stripe_customer_id ~ '^cus_[A-Za-z0-9_]{1,250}$'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE subscriptions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stripe_subscription_id TEXT NOT NULL UNIQUE CHECK (stripe_subscription_id ~ '^sub_[A-Za-z0-9_]{1,250}$'),
    plan_key TEXT NOT NULL CHECK (char_length(plan_key) BETWEEN 1 AND 100),
    plan_name TEXT NOT NULL CHECK (char_length(plan_name) BETWEEN 1 AND 200),
    price_id TEXT NOT NULL CHECK (char_length(price_id) BETWEEN 1 AND 255),
    status TEXT NOT NULL CHECK (status IN ('active', 'cancelling', 'past_due', 'ended')),
    current_period_end TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ,
    last_event_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((status = 'cancelling') <= (ends_at IS NOT NULL))
);

CREATE INDEX idx_subscriptions_user ON subscriptions (user_id, created_at);

CREATE TABLE checkout_requests (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    idempotency_key TEXT NOT NULL CHECK (char_length(idempotency_key) BETWEEN 1 AND 255),
    price_id TEXT NOT NULL CHECK (char_length(price_id) BETWEEN 1 AND 255),
    checkout_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, idempotency_key)
);

CREATE INDEX idx_checkout_requests_created ON checkout_requests (created_at);

CREATE TABLE stripe_events (
    event_id TEXT PRIMARY KEY CHECK (char_length(event_id) BETWEEN 1 AND 255),
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_stripe_events_received ON stripe_events (received_at);

ALTER TABLE api_tokens DROP CONSTRAINT api_tokens_scopes_check;
ALTER TABLE api_tokens ADD CONSTRAINT api_tokens_scopes_check CHECK (
    cardinality(scopes) > 0
    AND scopes <@ ARRAY['profile:read', 'profile:write', 'billing:read', 'billing:write']::TEXT[]
);

ALTER TABLE jobs DROP CONSTRAINT jobs_kind_check;
ALTER TABLE jobs ADD CONSTRAINT jobs_kind_check CHECK (
    kind IN (
        'send_email',
        'send_password_reset',
        'send_email_change',
        'cleanup',
        'cancel_stripe_subscription',
        'update_stripe_customer_email'
    )
);

ALTER TABLE account_events DROP CONSTRAINT account_events_channel_check;
ALTER TABLE account_events ADD CONSTRAINT account_events_channel_check CHECK (
    channel IN ('browser', 'api_token', 'admin', 'console', 'stripe')
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
        'data_exported',
        'api_token_created',
        'api_token_revoked',
        'checkout_started',
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
