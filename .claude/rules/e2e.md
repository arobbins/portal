---
paths:
  - "e2e/**"
---

# Behavior tests (Playwright)

`e2e/`, Chromium + Firefox + WebKit: `cd e2e && npm test` (headless),
`npm run ui` (dashboard), `npm run report`. Playwright runs its own server on
**:3100**: it builds into `target/e2e` (so a running `topcoat dev` can never
rebuild or re-bundle under a test run), runs `webserver seed` against
`<dev DATABASE_URL>_test` (or `TEST_DATABASE_URL`), and uses the file mailer
(`e2e/.mail`, cleared each run), so it never touches the dev server or dev
data.

- Import `test`/`expect` from `e2e/support/test.ts`: any console error or
  uncaught exception fails the test; allow expected ones with
  `test.use({ allowedConsoleErrors: [/…/] })`.
- Shared helpers: `support/flows.ts` (`register`, `logIn`, `uniqueEmail`,
  `createAccount`, `logOut`, `setPassword`), `support/mail.ts`
  (`waitForEmails`, `expectNoEmails`, `verificationLink`, `resetLink`),
  `support/users.ts` (seeded users — keep in sync with `src/seed.rs`).
  Tests that change data use their own `uniqueEmail` user, never the seeded
  ones.
- Submit forms with `press("Enter")` on the last field you typed in, never a
  click on the submit button: Firefox under Playwright sometimes drops the
  first click after typing (on any page, not just modals), so the form is
  never posted. Shared helpers in `flows.ts` already do this.
- Toasts are held open in tests (`holdToasts`, on by default in
  `support/test.ts`; held toasts ignore clicks), so a slow machine can't miss
  a toast before it auto-hides. Only `toaster.spec.ts` turns it off.
- Failed logins are rate-limited per account even in relaxed mode, so tests
  that log in with a wrong password use their own `uniqueEmail` account, never
  a seeded one.
- Popovers and menus must not move while opening (fade, don't slide): a
  moving target makes Playwright press outside it, which light-dismisses it.
- To find a flake, stress the suite: `npx playwright test --repeat-each=2
  --workers=16`, then read the failing test's `trace.zip` network and action
  logs.
- Find elements by role/label/text
  (`getByRole("button", { name: "Show toast" })`), not CSS classes; assert
  visible behavior. One spec file per feature.
- The full recipe is the `new-e2e-test` skill; run a single spec quickly
  with the `verify` skill.
- Screenshots: `npm run screenshot` (`screenshot.config.ts`, spec in
  `screenshots/`, settings via `SHOT_*` env vars, documented in the `verify`
  skill). It's a separate config, so `npm test` and `./check.sh` never run
  it; output goes to the gitignored `e2e/.screenshots`.
