ALTER TABLE users
    ADD CONSTRAINT users_verified_requires_password
    CHECK (verified_at IS NULL OR password IS NOT NULL);
