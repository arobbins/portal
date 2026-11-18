---
paths:
  - "src/auth/reauth.rs"
  - "src/auth/reauth/**"
  - "src/auth/pages/reauth_field.rs"
  - "src/passkeys/actions/verify.rs"
  - "assets/js/passkeys.js"
---

# Confirming it's you with a passkey

- **Confirming it's you** (`passkeys/actions/verify.rs`,
  `auth/reauth.rs`, `auth/pages/reauth_field.rs`, `assets/js/passkeys.js`):
  a user-verified request for the user's own passkeys stores a 5-minute
  `PasskeyReauth` grant bound to `session_version`. Routes
  `take_passkey_proof` (single use) and `return_passkey_proof` when the
  action fails; removing a passkey `forget_passkey_proof`s. Dialogs that
  need re-auth render `reauth_field` (passkey first when the user has one)
  and actions call `check_reauth`. Used by Change email, Delete account,
  Create API token, Remove passkey and the admin area; Change password and Add passkey keep the password.
