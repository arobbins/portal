-- Add email and password columns to users table
ALTER TABLE users ADD COLUMN email TEXT UNIQUE;
ALTER TABLE users ADD COLUMN password TEXT;
