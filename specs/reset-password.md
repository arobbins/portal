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
- R5. An expired, used or unknown link shows the heading "This link has expired", "This reset link is invalid or has
  already been used. Links work once and expire after 1 hour." with a "Request a new
  link" link, and status 400.
- R6. Submitting a valid new password, in one step: sets it, signs out every device,
  deletes every API token, disconnects every connected app, cancels pending
  email-change links, records one "password reset" activity entry, and emails "your
  password was changed".
- R7. Without two-factor, the browser is then signed in and lands on `/account` with
  "Your password has been reset."
- R8. For an account with two-factor, the "Choose a new password" form also has a
  "Code" field, below the two password fields and above the button, with the hint "Open your authenticator app and enter the 6-digit code
  for this account. No phone? Enter one of your recovery codes." R6 happens only when
  the passwords are acceptable and the code is right (an authenticator code, or a
  recovery code, which is then used up); the browser is then signed in and lands on
  `/account` with "Your password has been reset." The browser never goes through
  `/login/two-factor` for a reset.
- R8a. A wrong code uses the link up (and any other reset link for the account) and
  changes nothing else: the password, devices, API tokens, connected apps and pending
  email-change links stay as they were, no activity entry is recorded and no email is
  sent. The page shows "That code didn't work, so this link has been used up. Request
  a new link to try again." with a "Request a new link" link, status 400; opening the
  link again shows the invalid-link page (R5).
- R8b. Wrong codes on the reset form also count toward the limit on wrong codes at
  sign-in (5 per 15 minutes per account). Past it the form shows "Too many attempts.
  Please wait a few minutes and try again." (429), the code isn't checked, and the link
  isn't used up.
- R9. Signed-in visitors to `/reset-password` are sent to their account.
- R10. Changing the password while signed in cancels any unused reset link.

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
Resetting through the API or MCP; an admin reset of two-factor; telling the owner
about a reset that failed on the code (the reset email went to the same mailbox).

## Open questions
None.

## Scenarios
R1–R7 and R9 stay covered by `e2e/tests/reset-password.spec.ts`. The reset test in
`e2e/tests/two-factor.spec.ts` describes the old R8 and is replaced by S1 (the build deletes it). R8b is
server-tested (the e2e server relaxes rate limits): concurrent and sequential wrong
codes on the reset form count toward the sign-in limit, and past it a right code is
refused and the link is not used up.

- S1. Given an account with two-factor and a reset link, when the visitor enters a new
  password and a current authenticator code, then they land on `/account` with "Your
  password has been reset.", and the old password no longer signs in (covers R6, R8).
- S2. Given an account with two-factor, a working API token and a reset link, when the
  visitor enters a new password and a wrong code, then the page shows the used-up
  message, the old password still gets past the password step at Sign in, the API
  token still works, and opening the same link shows the invalid-link page; a newly
  requested link then succeeds with a right code (covers R8a).
- S3. Given an account with two-factor and a reset link, when the visitor enters a new
  password and one of the recovery codes, then the reset succeeds, and that recovery
  code no longer works at sign-in (covers R8).
- S4. Given an account without two-factor, the reset form has no "Code" field and a
  reset signs straight in (covers R7).
- S5. Given a reset link, when the user signs in and changes their password before
  using it, then the link shows the invalid-link page (covers R10).

## Changes
- 2026-11-27: captured from the code as it is today.
- 2026-11-27: capture approved.
- 2026-11-27: change drafted: the two-factor code is asked on the reset form, and
  a wrong code uses the link up and changes nothing else (R8, R8a, R8b); a password change cancels reset
  links (R10).
- 2026-11-27: change approved.
