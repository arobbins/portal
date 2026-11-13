---
paths:
  - "src/user/passkeys.rs"
  - "src/user/pages/account_passkeys.rs"
  - "src/user/pages/login.rs"
  - "src/user/pages/passkey_setup.rs"
  - "assets/js/passkeys.js"
  - "e2e/tests/passkeys.spec.ts"
---

# Passkeys

`user/passkeys.rs` (server), `assets/js/passkeys.js` (browser), Security →
Passkeys UI in `account_passkeys.rs`, built on `webauthn-rs`:

- Passkeys are an *extra* way to sign in; every account keeps its password.
  The relying party is built once from `Config` (`passkey_domain` =
  `APP_BASE_URL`'s host, which must be a domain, so dev and e2e use
  `localhost`, never `127.0.0.1`).
- JSON routes: `POST /passkeys/register/{start,finish}` (signed in; start
  needs the current password and counts wrong ones as failed logins) and
  `POST /passkeys/login/{start,finish}`. Ceremony state lives in the
  session, is single-use and expires after 10 minutes. Register/start may
  omit the password only with a passkey prompt grant (`auth.rs`:
  `grant_passkey_prompt` / `passkey_prompt_granted` /
  `revoke_passkey_prompt`): issued for one user, for 5 minutes, only when a
  password sign-in shows the prompt; cleared by `sign_in`, by answering the
  prompt and by adding a passkey.
- Registration overrides the library's options to `residentKey: required`
  (passkeys must be discoverable). Sign-in is discoverable (no email typed):
  the login page starts conditional mediation (autofill on the
  `username webauthn` email field) on load, and the button aborts it and
  starts a modal request. Starting a sign-in has its own loose rate limit
  (it runs on every login page view); finishing uses the password limiter.
- `users.webauthn_id` (random UUID) is the user handle; never the numeric
  id. `user_passkeys` stores the serialized passkey plus name (from a
  curated AAGUID → provider list, else "Passkey"), backup flags and
  `last_used_at`.
- The browser keeps devices in sync with signal APIs
  (`signalUnknownCredential`, `signalAllAcceptedCredentials`,
  `signalCurrentUserDetails`) from the data in `[data-passkey-signals]` on
  account pages; all feature-detected.
- **Passkey prompt** (`passkey_setup.rs`): after a password login or the
  first password setup, `after_password_sign_in` sends users with no passkey to `/passkey-setup` unless
  `users.passkey_prompt_after` is in the future ("Not now" = +30 days,
  "Don't ask again" = `'infinity'`; the due check lives in SQL so
  `infinity` is never decoded). The page only shows while the grant is
  valid, then redirects to `/account`. Adding a passkey there
  needs no password and lands on Security → Passkeys. Passkey sign-ins and
  password resets never prompt; seeded e2e users have declined it.
- Automatic (conditional) passkey creation after a password sign-in is
  deliberately not implemented: it creates passkeys without user
  verification, which `webauthn-rs` (correctly) rejects.
- Tests: `passkeys.spec.ts` and `passkey-setup.spec.ts` use Chromium's
  virtual authenticator (CDP `WebAuthn.*`, `support/passkeys.ts`); those
  tests skip on Firefox/WebKit. A virtual authenticator approves autofill instantly, so tests of the button path stub
  `isConditionalMediationAvailable` to `false`.
