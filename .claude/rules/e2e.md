---
paths:
  - "e2e/**"
---

# Behavior tests (Playwright)

`e2e/`, Chromium + Firefox + WebKit: `cd e2e && npm test` (headless),
`npm run ui` (dashboard), `npm run report`. Playwright runs its own server on
**:3100**: it builds into `target/e2e` (so a running `topcoat dev` can never
rebuild or re-bundle under a test run), runs `webserver seed` against
`<dev DATABASE_URL>_test`, and uses the file mailer (`e2e/.mail`, cleared
each run), so it never touches the dev server or dev data. A git worktree
gets its own port and `…_<worktree>_test` database
(`support/environment.ts`), so parallel agents never collide; override with
`E2E_PORT` / `TEST_DATABASE_URL`.

**Always run Playwright through `rtk proxy`** (`rtk proxy npx playwright test
… --reporter=line`): rtk's filter reports `PASS (0) FAIL (0)` when the test
server fails to start, which looks like success. Use the `check` skill for the
full suite and the `verify` skill for one spec.

- Import `test`/`expect` from `e2e/support/test.ts`: any console error or
  uncaught exception fails the test; allow expected ones with
  `test.use({ allowedConsoleErrors: [/…/] })`. Pass exactly one regex
  (`/status of 40[34]/`): a two-element array is read by Playwright as a
  `[value, options]` fixture tuple and breaks every test in the file.
- Shared helpers: `support/flows.ts` (`register`, `logIn`, `uniqueEmail`,
  `createAccount`, `createAccountAtPasskeyPrompt`, `skipPasskeyPrompt`,
  `logOut`, `setPassword`), `support/passkeys.ts` (virtual authenticator), `support/api.ts` (`createToken`,
  `bearer`, `getMe`), `support/mail.ts`
  (`waitForEmails`, `expectNoEmails`, `verificationLink`, `resetLink`),
  `support/users.ts` (seeded users — keep in sync with `src/seed.rs`),
  `support/console.ts` (`grantAdmin`, runs `webserver admin grant`),
  `support/state-machines.ts` (browser state machines for the report; see
  `.claude/state-machines.md`).
  Tests that change data use their own `uniqueEmail` user, never the seeded
  ones. New accounts land on the passkey prompt; `createAccount` answers
  "Not now", so later password logins go straight to `/account`.
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
- To find a flake, stress the suite: `rtk proxy npx playwright test --repeat-each=2
  --workers=16`, then read the failing test's `trace.zip` network and action
  logs.
- Find elements by role/label/text
  (`getByRole("button", { name: "Show toast" })`), not CSS classes; assert
  visible behavior. One spec file per feature. Scoping to a settings card
  by its `id` (`page.locator("#passkeys")`, `#api-tokens`) is fine: card ids
  are stable anchors the account nav links to.
- The full recipe is the `new-e2e-test` skill; run a single spec quickly
  with the `verify` skill.
- Screenshots: `npm run screenshot` (`screenshot.config.ts`, spec in
  `screenshots/`, settings via `SHOT_*` env vars, documented in the `verify`
  skill). It's a separate config, so `npm test` and `./check.sh` never run
  it; output goes to the gitignored `e2e/.screenshots`.
