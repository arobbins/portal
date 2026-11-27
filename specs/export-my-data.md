---
status: approved
capability: Export my data
---

# Export my data

## Goal

A signed-in user can download a copy of everything the app keeps about them, so they
stay in control of their own data and can take it elsewhere. It is offered again right
before they delete their account, so nobody deletes without the chance to keep a copy.

## Behaviour

- R1. The user starts an export from the "Export data" card and confirms with their
  current password, or with a passkey if they have one. Without a correct confirmation
  nothing is downloaded.
- R2. A confirmed export downloads straight away as one ZIP file named
  `<site>-data-<yyyy-mm-dd>.zip` (the date in UTC).
- R3. The ZIP holds one CSV per kind of data, plus `README.txt` explaining each file
  and its columns:
  - `profile.csv`: public id, name, email, time zone, account created, email confirmed.
  - `passkeys.csv`: name, added, last used, backed up.
  - `api_tokens.csv`: name, used by, scopes, last four characters, created, expires,
    last used.
  - `devices.csv`: signed-in browsers: browser, IP address, first and last seen.
  - `activity.csv`: the account's activity history: what happened, when, how
    (browser, API, admin), IP address, details.
  - `notifications.csv`: each optional email and whether it is on.
  - `two_factor.csv`: whether two-factor is on and since when.
  - `api_feedback.csv`: feedback the user sent through the API.
  - `connected_apps.csv` (`name`, `returns_to`, `scopes`, `connected_at`,
    `last_used_at`): each app connected by approving it; `returns_to` is the bare host
    (`claude.ai`, `127.0.0.1`), `scopes` the raw scope names separated by spaces.
  - `billing.csv` (`plan`, `status`, `renews_on`, `ends_on`, `stripe_customer_id`):
    one row; plan `free`, `pro`, …; status `free`, `active`, `cancelling`, `past_due`;
    empty cells when there's no value. No card, payment or invoice details: those stay
    in Stripe. A paid account's values are server-tested.
- R4. A file is always present, with its header row, even when the user has no rows
  of that kind.
- R5. CSVs are UTF-8 with a header row. Every date and time is UTC in ISO 8601
  (`2026-11-18T14:03:00Z`); the README says so.
- R6. The export never contains a secret: no password hash, API token or token hash,
  two-factor secret, recovery codes, passkey key material, emailed-link tokens or
  remembered-browser hashes.
- R7. Every feature that stores data about a user adds it to the export, or is named
  in the README as deliberately left out, with the reason. A build check fails when a
  per-user table is neither (verified by a server test, not a browser scenario).
- R8. Each export sends the security email "Your account data was downloaded" to the
  account's address, every time, with no way to turn it off.
- R9. Each export is recorded in the account's activity as "Account data exported".
- R10. An account can export at most 5 times an hour. The 6th attempt shows "Too many
  attempts" and downloads nothing, sends no email and records nothing (verified by a
  server test; the e2e server relaxes rate limits).
- R11. The Delete account dialog starts with "Want a copy first?" and an "Export your
  data" link that closes it and opens the Export data dialog.
- R12. Exporting works without JavaScript (a plain form post that returns the file).
- R13. No cell is run as a formula when a CSV is opened in a spreadsheet. Any value
  that starts with `=`, `+`, `-`, `@`, a tab or a carriage return gets an apostrophe
  (`'`) in front, in every cell of every file, so spreadsheets show it as text. A value
  that is entirely a plain number (`-5`, `+3.2`) is left as it is. The README says
  that a leading apostrophe was added and can be removed to get the original value.
  (Plain numbers are verified by a server test; the starter has no numeric columns.)

## Placement

- Profile page: an "Export data" row in the "Your data" card, directly above "Delete
  account". Its button
  "Export data" opens the "Export data" dialog with a "Current password" field and
  the usual passkey option.
- Browser only. No `/api/v1` endpoint and no MCP tool.
- Labels the scenarios rely on: the "Export data" button and dialog, "Current
  password", "Export your data" in the delete dialog, the email subject (R8) and the
  activity entry (R9). The builder chooses all other wording within the rule files.

## Security and abuse

- A stolen or unattended session can't take the data without the password or
  passkey (R1), and the owner is emailed every time (R8).
- Wrong passwords count toward the same lockout as every other password check.
- Rate limit: 5 per hour per account (R10).
- No secrets, ever (R6). The file itself is never stored on the server or logged.
- Text the user, an API token or a browser's user-agent header controls can reach
  the file, so no cell may run as a formula in a spreadsheet (R13).
- The admin log is not exported: it is the operator's record and names admins. That
  an admin acted on the account already appears in the user's own activity.
- Disabled accounts can't sign in, so they can't export.

## Not in this version

- Export through the API or MCP, or an `account:export` scope.
- Background preparation for large exports (a later spec change if exports get slow).
- Admins exporting a user's data, and exports for disabled accounts.
- JSON or other formats besides CSV.
- Scheduled or repeated exports.
- Rejecting names that start with `=`, `+`, `-` or `@` when they're saved. The
  export protects itself (R13); stricter input would be a change to those features'
  own specs.

## Open questions

- The `<site>` part of the file name: the site name in lower case with dashes
  (`webserver-data-2026-11-18.zip`) unless changed.
- Column names: lower snake case (`created_at`) unless changed.

## Scenarios

- S1. Given a signed-in user with an API token, when they export with their password,
  then a ZIP named `<site>-data-<today>.zip` downloads with all the files in R3, each
  with a header row; `profile.csv` holds their name and email and `api_tokens.csv`
  their token's name. (covers R1, R2, R3, R4, R5)
- S2. Given that export, then no file contains a password hash or an API token.
  (covers R6)
- S3. Given that export, then the owner gets "Your account data was downloaded" and
  their activity shows "Account data exported". (covers R8, R9)
- S4. Given a signed-in user, when they confirm with a wrong password, then they see
  "Your current password is incorrect.", nothing downloads and no email is sent.
  (covers R1)
- S5. Given a user with a passkey, when they export confirming with the passkey, then
  the ZIP downloads. (covers R1)
- S6. Given the Delete account dialog is open, when the user follows "Export your
  data", then the Delete account dialog closes and the Export data dialog opens.
  (covers R11)
- S7. Given JavaScript is off, when the user exports with their password, then the
  ZIP downloads. (covers R12)
- S9. Given a user with a connected app, when they export, then `connected_apps.csv`
  holds the app's name, return host and scopes, and `billing.csv` holds `free` as the
  plan and status. (covers R3, R4)
- S8. Given a user whose name is `=1+1` and who has an API token named `@SUM(1,2)`,
  when they export, then `profile.csv` holds `'=1+1`, `api_tokens.csv` holds
  `'@SUM(1,2)`, and `README.txt` explains the leading apostrophe. (covers R13)
- R7, R10 and the plain-number part of R13 are covered by server tests, not browser
  scenarios.

## Changes

- 2026-11-27: approved connected apps and billing in the export.
- 2026-11-27: connected apps and billing added to the export (R3).
- 2026-11-18: drafted.
- 2026-11-18: approved and built.
- 2026-11-18: R13 added: CSV cells never run as spreadsheet formulas (found by
  the post-build review).
- 2026-11-19: Export data moved into a "Your data" card as a row (user's layout choice).
- 2026-11-19: Profile and Security moved under Settings; the activity list is now at /account/settings/devices.
