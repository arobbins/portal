---
name: new-e2e-test
description: Write a Playwright behavior test for this app following its e2e conventions (fixtures, helpers, unique users, email links, modal quirks). Use whenever adding or extending a spec in e2e/tests/.
---

# New e2e test

Conventions are in `.claude/rules/e2e.md`; reference spec: `e2e/tests/profile.spec.ts`.

## 1. File and imports

- One spec per feature: `e2e/tests/<feature>.spec.ts`. Extend the existing spec if the
  feature already has one.
- Always:
  ```ts
  import { expect, test } from "../support/test";
  ```
  never from `@playwright/test` (the fixture fails tests on console errors and uncaught
  exceptions). Types like `Page` may still come from `@playwright/test`.

## 2. Users and data

- A test that changes data creates its own user:
  `const email = await createAccount(page, testInfo, "Feature Tester");` (registers, follows
  the emailed link, sets `ACCOUNT_PASSWORD`, lands on `/account`).
- For sign-in flows: `register`, `setPassword`, `logIn`, `logOut` from `support/flows.ts`,
  and `uniqueEmail(testInfo, "label")` for fresh addresses.
- Seeded users (`support/users.ts`: `verifiedUser`, `awaitingPasswordUser`) are read-only
  fixtures. If you change `src/seed.rs`, update `users.ts` too.
- Emails: `await waitForEmails(email)` then `verificationLink(mail)` / `resetLink(mail)`;
  assert nothing was sent with `expectNoEmails(email)`.

## 3. Writing steps

- Locate by role, label or text: `page.getByRole("button", { name: "Save profile" })`,
  `page.getByLabel("Name")`, `page.getByRole("dialog", { name: "Edit profile details" })`.
  Never CSS classes or ids.
- Inside a modal `<dialog>`, submit with `field.press("Enter")`, not a click on the submit
  button (Firefox drops the first click after typing in a modal).
- Assert what the user sees: text visible, URL (`toHaveURL`), button enabled/disabled,
  `toHaveValue`. Toasts animate: assert `toHaveCSS("opacity", "1")` for shown and `"0"` for
  hidden.
- Prove a JS path didn't reload by setting a `window` marker before and checking it after.
- Simulate the server being unreachable with
  `page.route("**/_topcoat/runtime/procedures/**", (route) => route.abort())`, and allow the
  resulting console errors with
  `test.use({ allowedConsoleErrors: [/Failed to load resource|NetworkError|network connection/i] })`
  inside a `test.describe`.
- Chromium-only features (passkeys' virtual authenticator) use
  `test.skip(browserName !== "chromium", "...")`.
- No fixed sleeps; rely on auto-waiting `expect` assertions.

## 4. What to cover for a feature

- The success path, and that a flash doesn't come back on refresh.
- Each problem message, with the field marked invalid and its value kept.
- The no-JS path for forms (plain POST) if it differs from the JS path.
- Mobile layout if the feature adds UI: extend `mobile.spec.ts` at 390px.

## 5. Run it

Use the `verify` skill: `cd e2e && npx playwright test <feature> --project=chromium`, then all
three browsers via `./check.sh` at the end.
