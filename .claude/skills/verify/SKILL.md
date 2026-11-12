---
name: verify
description: Fast check that a change works while iterating — run only the relevant Playwright spec on Chromium, then screenshot the affected page at phone and desktop width in the running dev server. Use after a change to a page, form, component or flow, before the full ./check.sh.
---

# Verify a change quickly

`./check.sh` runs everything in three browsers; use it once at the end. While
iterating, do this instead.

## 1. Run the relevant spec on Chromium

Find the spec for the feature (`ls e2e/tests`; one spec per feature, e.g. profile →
`profile.spec.ts`, theme → `theme.spec.ts`, layout → `mobile.spec.ts`). Then:

```sh
cd e2e && npx playwright test <spec-name> --project=chromium --reporter=list
```

- Add `-g "<test title>"` to run one test.
- The first run builds into `target/e2e` and starts its own server on :3100 with the test
  database, so it's slow once and fast afterwards. It never touches the dev server or data.
- On failure, read the error output first; Playwright also writes
  `e2e/test-results/<test>/error-context.md` (page snapshot at the failure). Fix and rerun
  the same command.
- If the change touched Firefox-sensitive behavior (modals, clicks after typing) or WebKit
  (passkeys skip there), also run `--project=firefox` / `--project=webkit` for that spec.

## 2. Look at the page (visual changes only)

Only when the change is visible (layout, CSS, a new component or page):

1. The session-start context says whether `topcoat dev` is on :3000. If not, ask the user to
   start it (don't start a second server).
2. Load the `claude-in-chrome` skill, open a **new tab** at `http://localhost:3000/<path>`
   (always `localhost`, never `127.0.0.1`). Pages behind login need the user's own signed-in
   browser session; ask if you hit `/login`.
3. Screenshot at 390px wide and at 1280px wide. At 390px check there is no horizontal scroll
   (`document.documentElement.scrollWidth <= window.innerWidth`), the header menu opens from
   the Menu button, and the account sidebar shows as tabs.
4. Switch themes with `window.setTheme("light")` / `window.setTheme("dark")` and screenshot
   again if colors changed. Put it back to what it was.
5. Report what you saw; don't claim it looks right without a screenshot.

## 3. Finish

When the targeted checks pass, run `./check.sh` once before calling the work done, and run the
`convention-reviewer` agent on non-trivial diffs.
