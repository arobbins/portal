ALTER TABLE api_tokens ADD COLUMN public_id TEXT;
UPDATE api_tokens SET public_id = 'tok_' || substr(md5(random()::text || id::text), 1, 20);
ALTER TABLE api_tokens
    ALTER COLUMN public_id SET NOT NULL,
    ADD CONSTRAINT api_tokens_public_id_key UNIQUE (public_id),
    ADD CONSTRAINT api_tokens_public_id_format CHECK (public_id ~ '^tok_[A-Za-z0-9]{20}$');

ALTER TABLE user_passkeys ADD COLUMN public_id TEXT;
UPDATE user_passkeys SET public_id = 'pk_' || substr(md5(random()::text || id::text), 1, 20);
ALTER TABLE user_passkeys
    ALTER COLUMN public_id SET NOT NULL,
    ADD CONSTRAINT user_passkeys_public_id_key UNIQUE (public_id),
    ADD CONSTRAINT user_passkeys_public_id_format CHECK (public_id ~ '^pk_[A-Za-z0-9]{20}$');
