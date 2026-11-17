CREATE TABLE admin_log (
    id BIGSERIAL PRIMARY KEY,
    admin_public_id TEXT CHECK (admin_public_id ~ '^usr_[A-Za-z0-9]{20}$'),
    admin_email TEXT,
    action TEXT NOT NULL CHECK (
        action IN (
            'account_deleted',
            'account_disabled',
            'account_enabled',
            'admin_granted',
            'admin_revoked',
            'signed_out_everywhere',
            'verification_resent'
        )
    ),
    target_public_id TEXT NOT NULL CHECK (target_public_id ~ '^usr_[A-Za-z0-9]{20}$'),
    target_email TEXT NOT NULL,
    ip TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((admin_public_id IS NULL) = (admin_email IS NULL))
);

CREATE INDEX admin_log_created_at_idx ON admin_log (created_at DESC, id DESC);
