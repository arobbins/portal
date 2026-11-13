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

Only when the change is visible (layout, CSS, a new component or page). Use the screenshot
tool; it runs against the e2e server (current code, fresh database), so it needs no dev server
or browser extension and can sign in:

```sh
cd e2e && SHOT_PATHS=/login,/register SHOT_OUT=<scratchpad>/shots npm run screenshot
```

Settings (environment variables):

| Variable | Default | Meaning |
|---|---|---|
| `SHOT_PATHS` | `/` | comma-separated paths |
| `SHOT_THEMES` | `dark,light` | themes to capture |
| `SHOT_WIDTHS` | `390,1280` | viewport widths (phone, desktop) |
| `SHOT_SIGNED_IN` | off | `1` signs up a fresh user first (for `/account/...`); `new` stops at the passkey prompt (for `/passkey-setup`, which is only shown for 5 minutes after a password sign-in that prompts) |
| `SHOT_SELECTOR` | whole page | CSS selector to crop to one element |
| `SHOT_VARIANTS` | `{"current": ""}` | path to a JSON file `{ "name": "css", ... }` to compare design options; each is injected on top of the real CSS |
| `SHOT_SCALE` | `1` | device pixel ratio (`2`–`3` to judge alignment) |
| `SHOT_OUT` | `e2e/.screenshots` | output folder; files are `<path>-<variant>-<theme>-<width>.png` |

Transitions are disabled before capture, so colors are never caught mid-animation.

1. Capture the affected pages at both widths and both themes, then read the images.
2. At 390px check there's no sideways scroll, the header menu sits behind the Menu button, and
   the account sidebar shows as tabs.
3. When the user is choosing between designs, render each option as a variant and show them
   the file paths with a short comparison and a recommendation.
4. Report what you saw; don't claim it looks right without a screenshot.

For interactive checks the tool can't do (hover, an open modal, a toast), write a temporary
spec in `e2e/screenshots/` next to `screenshot.spec.ts`, run it with `npm run screenshot`, and
delete it afterwards. Claude in Chrome on `http://localhost:3000` is the fallback when the user
wants their own signed-in session.

## 3. Finish

When the targeted checks pass, run `./check.sh` once before calling the work done, and run the
`convention-reviewer` agent on non-trivial diffs.
