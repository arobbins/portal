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
- Inside a modal `<dialog>`, submit with `press("Enter")` on a field, not a
  click on the submit button: Firefox under Playwright sometimes drops the
  first click after typing in a modal (the click never reaches the page).
- Find elements by role/label/text
  (`getByRole("button", { name: "Show toast" })`), not CSS classes; assert
  visible behavior. One spec file per feature.
- The full recipe is the `new-e2e-test` skill; run a single spec quickly
  with the `verify` skill.
