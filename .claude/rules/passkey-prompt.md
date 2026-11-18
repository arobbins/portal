---
paths:
  - "src/passkeys/pages/passkey_setup.rs"
  - "src/auth/session.rs"
  - "src/auth/actions/sign_in.rs"
  - "e2e/tests/passkey-setup.spec.ts"
---

# Passkey prompt

- **Passkey prompt** (`passkey_setup.rs`): after a password login or the
  first password setup, `after_password_sign_in` sends users with no passkey to `/passkey-setup` unless
  `users.passkey_prompt_after` is in the future ("Not now" = +30 days,
  "Don't ask again" = `'infinity'`; the due check lives in SQL so
  `infinity` is never decoded). The page only shows while the grant is
  valid, then redirects to `/account`; "Not now" / "Don't ask again"
  only count while the prompt is showing (`prompt_showing`), so an old tab
  can't undo an earlier answer. Adding a passkey there
  needs no password and lands on Security → Passkeys. Passkey sign-ins and
  password resets never prompt; seeded e2e users have declined it.
