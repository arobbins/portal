UPDATE users SET password = NULL WHERE verified_at IS NULL;

ALTER TABLE users DROP CONSTRAINT users_verified_requires_password;

ALTER TABLE users
    ADD CONSTRAINT users_password_iff_verified
    CHECK ((password IS NULL) = (verified_at IS NULL));
