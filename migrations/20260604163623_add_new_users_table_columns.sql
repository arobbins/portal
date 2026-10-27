ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'subscriber' CHECK (role IN ('subscriber', 'admin'));
ALTER TABLE users ADD COLUMN photo TEXT;
ALTER TABLE users ADD COLUMN subscriber_status TEXT NOT NULL DEFAULT 'inactive' CHECK (subscriber_status IN ('active', 'inactive'));
ALTER TABLE users ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE users ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();