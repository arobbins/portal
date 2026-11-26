CREATE TABLE duplicate_subscriptions (
    stripe_subscription_id TEXT PRIMARY KEY CHECK (stripe_subscription_id ~ '^sub_[A-Za-z0-9_]{1,250}$'),
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_duplicate_subscriptions_user ON duplicate_subscriptions (user_id);
