---
status: approved
capability: Reset my password
---

# Reset my password

## Goal
Someone who forgot their password gets a link by email and chooses a new one, without
signing in. Choosing it signs them in and ends access everywhere else, so a reset also
locks out anyone who had the old password.

## Behaviour
- R1. `/reset-password` asks for an email and always answers the same way, whether or
  not an account exists: the form is replaced by "If an account exists for <email>, we
  sent it a link to choose a new password. It expires in 1 hour."
- R2. An active account gets a "Reset your password" email with a link to
  `/reset-password/{token}`. An account that never set a password gets its sign-up
  link again instead. A disabled account or an unknown email gets nothing.
- R3. A newer reset link replaces any older one. A link works once and expires after
  1 hour.
- R4. Opening the link only shows the "Choose a new password" form; it doesn't use the
  link up. A rejected password (too short, not matching) keeps the link working.
- R5. An expired, used or unknown link shows "This reset link is invalid or has
  already been used. Links work once and expire after 1 hour." with a "Request a new
  link" link, and status 400.
- R6. Submitting a valid new password, in one step: sets it, signs out every device,
  deletes every API token, disconnects every connected app, cancels pending
  email-change links, records one "password reset" activity entry, and emails "your
  password was changed".
- R7. Without two-factor, the browser is then signed in and lands on `/account` with
  "Your password has been reset."
- R8. With two-factor, R6 happens first and the browser goes to `/login/two-factor`;
  a correct code signs it in and lands on `/account` with "Your password has been
  reset." A wrong code doesn't undo R6.
- R9. Signed-in visitors to `/reset-password` are sent to their account.

## Placement
Public pages `/reset-password` and `/reset-password/{token}`, linked from Sign in.
Browser only; no API or MCP.

## Security and abuse
- Asking for links: 5 per 10 minutes per IP and at most 3 emails an hour to one
  address (the extra ones are skipped silently). Choosing a password: 10 a minute per
  IP (429). Wrong two-factor codes: 5 per 15 minutes per account.
- Only the link's hash is stored. GETs never use a link up. The answer never reveals
  whether an account exists. Nothing beyond the CLAUDE.md invariants otherwise.

## Not in this version
Resetting through the API or MCP; an admin reset of two-factor.

## Open questions
None.

## Scenarios
Covered today by `e2e/tests/reset-password.spec.ts` and the reset test in
`e2e/tests/two-factor.spec.ts` (R1–R9).

## Changes
- 2026-11-27: captured from the code as it is today.
- 2026-11-27: capture approved.
