ALTER TABLE users ADD COLUMN admin_granted_at TIMESTAMPTZ;
UPDATE users SET admin_granted_at = now() WHERE role = 'admin';
ALTER TABLE users ADD CONSTRAINT users_admin_granted_at_matches_role CHECK (
    (role = 'admin') = (admin_granted_at IS NOT NULL)
);
