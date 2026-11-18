---
paths:
  - "src/two_factor/**"
  - "src/two_factor.rs"
  - "src/auth/pages/login.rs"
  - "src/auth/actions/sign_in.rs"
  - "src/auth/actions/link_password.rs"
  - "e2e/tests/two-factor.spec.ts"
---

# Two-factor authentication

- **Two-factor authentication** (`src/two_factor/`, `Permission::ManageTwoFactor`,
  browser-only): TOTP (SHA-1, 6 digits, 30 s, one step of skew) from an
  authenticator app, set up on Security → Authentication by scanning a QR code
  (the secret is generated per page view and only stored once a code and
  the password or a passkey confirm it) plus 10 single-use recovery codes
  (hashed; shown once; regenerating replaces them). A correct password, or
  a password-reset link, for a user with two-factor stores
  `PendingSecondFactor` in the session and goes to `/login/two-factor`
  instead of signing in (it keeps the `SignInMethod`, so a reset still
  ends as a reset); wrong codes have their own limit, 5 per 15 minutes per
  account (`FailedSecondFactorsByAccount`), which a correct password does
  not clear. Each app code's time step is accepted once (`last_used_step`).
  The TOTP secret is stored as is (the server needs it to check codes; the
  SQL console hides it); encrypting it at rest is a later option. Turning
  two-factor on or off doesn't bump `session_version`. Passkey sign-in
  skips the second step (a passkey already proves the device). There is no
  admin reset: a locked-out user uses a recovery code. Turning it on or
  off is always emailed.
