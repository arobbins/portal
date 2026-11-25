ALTER TABLE user_tokens DROP CONSTRAINT user_tokens_payload_check;
ALTER TABLE user_tokens ADD CONSTRAINT user_tokens_payload_check CHECK (
    CASE purpose
        WHEN 'email_change' THEN payload IS NOT NULL
        WHEN 'email_verification' THEN payload IS NULL OR payload ~ '^price_[A-Za-z0-9_]{1,249}$'
        ELSE payload IS NULL
    END
);
