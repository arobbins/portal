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
gets its own port, `…_<worktree>_test` database and 25% of the cores (main
75%; `support/environment.ts`), so parallel agents never collide or overload
the machine; override with `E2E_PORT` / `TEST_DATABASE_URL` / `E2E_WORKERS`.

Run Playwright plainly (`npx playwright test … --reporter=line`): rtk
leaves it unfiltered (its filter once hid a server that failed to start). Use the `check` skill
for the full suite and the `verify` skill for one spec.

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
  `support/totp.ts` (`totp`, `turnOnTwoFactor`), `support/state-machines.ts` (browser state machines for the report; see
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
- Find a flake with `--repeat-each=2 --workers=16`, then read the failing
  test's `trace.zip`.
- Find elements by role/label/text
  (`getByRole("button", { name: "Show toast" })`), not CSS classes; assert
  visible behavior. One spec file per feature. Scoping to a settings card
  by its `id` (`page.locator("#passkeys")`, `#api-tokens`) is fine: card ids
  are stable anchors the account nav links to.
- The full recipe is the `new-e2e-test` skill.
- Screenshots: `npm run screenshot` (`screenshot.config.ts`, `SHOT_*` env
  vars in the `verify` skill); a separate config `npm test` never runs,
  output in the gitignored `e2e/.screenshots`.
- `accessibility.spec.ts` runs axe (Chromium) on every page and its dialogs
  and menus, light and dark, at 390 and 1280px: `serious`/`critical` fail,
  `minor`/`moderate` are listed in its output and `accessibility.json`
  attachment. Add new pages to it (it may find dialogs by id).
- Layout checks measure `document.body`, never `innerWidth`/`clientWidth`:
  CI's scrollbars plus `scrollbar-gutter: stable` make those 15px wider.
