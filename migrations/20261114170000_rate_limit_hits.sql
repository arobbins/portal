CREATE TABLE rate_limit_hits (
    bucket TEXT NOT NULL,
    key TEXT NOT NULL CHECK (char_length(key) <= 300),
    at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_rate_limit_hits_lookup ON rate_limit_hits (bucket, key, at);
CREATE INDEX idx_rate_limit_hits_at ON rate_limit_hits (at);
