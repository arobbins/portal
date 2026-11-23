---
status: approved
capability: Choose my newsletters
---

# Choose my newsletters

## Goal
A signed-in user decides whether they're on the app's two mailing lists, the General
newsletter and Product updates, and can change their mind at any time. New accounts
start on both lists and are told so when they sign up. Nothing sends newsletters yet;
this records each person's choice, ready for whatever sends them later.

## Behaviour
- R1. There are two newsletters: "General newsletter" and "Product updates". Each is
  on or off for each account, independently.
- R2. An account created after this ships starts with both newsletters on.
- R3. An account that existed before this ships starts with both newsletters off.
- R4. The Register page tells people, before they create the account: "We'll send you
  our newsletter and product updates. You can turn them off any time in Settings."
- R5. On Settings → Notifications, a "Newsletters" card has a switch for each
  newsletter ("General newsletter", "Product updates") showing its current state.
  Flipping a switch saves straight away, with no password and no Save button, and
  shows "Newsletter preferences saved."; the choice survives a reload and a new
  sign-in.
- R6. If saving fails, the switch flips back and "We couldn't save your newsletter
  preferences. Please try again." is shown; the stored choice doesn't change.
- R7. Without JavaScript the card still works: the switches sit in a form with a "Save
  newsletter preferences" button that saves both choices.
- R8. Each change is recorded in the account's activity: "Subscribed to General
  newsletter", "Unsubscribed from General newsletter", "Subscribed to Product
  updates", "Unsubscribed from Product updates". Saving a choice that didn't change
  records nothing.
- R9. API: an endpoint reads both choices (scope `profile:read`) and one changes
  either or both (scope `profile:write`), at `GET` and `PATCH /api/v1/newsletters`,
  and the same two actions are MCP tools `get_newsletters` and `update_newsletters`.
  Both choices are booleans named `general_newsletter` and `product_updates`: the read
  returns both; the change takes either or both and returns both. A change through the
  API is recorded in the activity as made with an API token, like other API changes. A
  token without the scope gets 403 `insufficient_scope`. Both appear in the API docs,
  OpenAPI document and `/llms.txt` like every other endpoint.
- R10. The admin user page shows the user's two newsletter choices, read-only, each
  as "Subscribed" or "Not subscribed".
- R11. The data export's `notifications.csv` has a row for each newsletter with
  whether it's on, alongside the existing optional emails.

## Placement
- Register page: the R4 sentence, near the Register button.
- Settings → Notifications: a "Newsletters" card, below the existing card.
- Admin user page: a "Newsletters" section with the two choices (R10).
- API and MCP as in R9. Fixed labels the scenarios rely on: the R4 sentence, the
  "Newsletters" card, the two switch names, the R8 activity entries, "Subscribed" /
  "Not subscribed", "Newsletter preferences saved.", the R6 error, the "Save newsletter
  preferences" button, the R9 paths, tool names and field names. The builder chooses
  all other wording.

## Security and abuse
- Default-on is the user's decision, made knowing that EU and UK law (GDPR, PECR)
  doesn't count a default-on or pre-ticked choice as consent to marketing email. The
  R4 notice and the activity record (R8) are the evidence of what each person was
  told and chose. Before newsletters are sent to EU or UK recipients, revisit R2.
- Newsletter choices aren't security settings, so tokens with `profile:write` may
  change them (R9), and every change is recorded (R8).
- No new rate limit: the API's existing per-token limit applies; flipping a switch is
  idempotent.
- Deleting the account deletes its choices. A disabled account keeps its choices but
  can't change them (it can't sign in, and its tokens are rejected).

## Not in this version
- Sending newsletters, syncing to an email provider, and unsubscribe links in emails.
  Any newsletter email built later must carry a one-click unsubscribe link.
- Telling existing users about the newsletters, or subscribing them.
- More lists, frequency choices, or per-topic preferences beyond the two.
- Admins changing a user's choices.

## Open questions
- "Ships" (R2, R3) means the moment the migration adding newsletters runs.

## Scenarios
- S1. Given someone on the Register page, then they see the R4 sentence; when they
  register and confirm their email, then both newsletter switches are on.
  (covers R2, R4, R5)
- S2. Given a signed-in user with both on, when they turn off General newsletter, then
  a confirmation shows, after a reload General newsletter is off and Product updates
  is still on, and their activity shows "Unsubscribed from General newsletter".
  (covers R1, R5, R8)
- S3. Given that user, when they turn General newsletter back on, then their activity
  shows "Subscribed to General newsletter". (covers R8)
- S4. Given saving fails (the server can't be reached), when the user flips a switch,
  then it flips back and the R6 error shows; after a reload the choice is unchanged.
  (covers R6)
- S5. Given JavaScript is off, when the user unticks Product updates and presses
  Save, then after a reload Product updates is off. (covers R7)
- S6. Given a token with `profile:read`, `GET /api/v1/newsletters` returns
  `general_newsletter` and `product_updates` as booleans, and `/llms.txt` lists both
  newsletter endpoints; given a token with `profile:write`, a `PATCH` turning Product updates off
  changes the switch on the page and records the change with an API token in the
  activity; a token without `profile:write` gets 403 `insufficient_scope`.
  (covers R9)
- S7. Given an MCP token, `tools/list` includes `get_newsletters` and
  `update_newsletters` for a token with both profile scopes, and calling
  `update_newsletters` changes the choice. (covers R9)
- S8. Given a user who turned Product updates off, when an admin opens their admin
  user page, then it shows General newsletter "Subscribed" and Product updates "Not
  subscribed". (covers R10)
- S9. Given a user who turned General newsletter off, when they export their data,
  then `notifications.csv` has a General newsletter row that's off and a Product
  updates row that's on. (covers R11)
- R3 (existing accounts start off) is covered by a server test: a browser can't
  create an account from before the change.

## Changes
- 2026-11-18: drafted.
- 2026-11-18: fixed the confirmation, error and Save button wording and the API field
  names (gaps found by the scenario writer).
- 2026-11-18: approved (interview answers plus accepted defaults and wording).
- 2026-11-19: Profile and Security moved under Settings; the activity list is now at /account/settings/devices.
