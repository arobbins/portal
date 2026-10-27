UPDATE users SET email = lower(trim(email));
ALTER TABLE users ALTER COLUMN email SET NOT NULL;
ALTER TABLE users ALTER COLUMN password SET NOT NULL;
