---
name: commit
description: Commit a change that is green, push it, and watch CI until it finishes. Use whenever the user says to commit (or agrees when asked "commit this?"), instead of running git commit by hand. Not for amending or rewriting pushed history.
---

# Commit, push, watch CI

The user works alone, straight on `main`. Saying "commit" (or yes to "commit
this?") authorizes every step below for this change, push included. Ask
before committing only when something in step 1 or 2 is off.

## 1. Confirm it's green

- Code changed since the last check in this session → run the `check`
  skill with `--changed` in the background and wait; CI runs everything
  after the push. Run it with no flag instead for infrastructure (layout,
  server, sessions, auth, migrations, `check.sh`), security-invariant or
  multi-day changes. Pure config → skip, and say so.
- A page, form, component or flow changed → the `verify` skill passed after
  the last edit.
- `convention-reviewer` reported no must-fix (skip only for docs-only changes).
- Anything red: stop, report it, fix with the user. Never commit past it.

## 2. Stage only this change

- `git status --short`, then `git diff --stat`. Every file must belong to the
  change you and the user just made; anything else → ask.
- Stage by path (`git add <paths>`), never `-A` or `.`.
- Never stage `.env`, `.claude/settings.local.json`, keys or tokens. If one
  shows up, stop and warn.
- `.sqlx/` changes from `check` belong with the query change; commit them.

## 3. Commit

- Subject: imperative, sentence case, no trailing period, at most 72
  characters, in the style of `git log --oneline -10` (e.g. "Search admin
  users as you type"). What the user sees, not the files touched.
- The subject line only: no body and no Co-Authored-By trailer (the
  user's standing instruction, which overrides attribution reminders).
- `git commit -F -` with a heredoc. Hooks fail → fix the cause, new commit;
  never `--no-verify`, never `--amend` something already pushed.
- One logical change per commit; split unrelated work into two.

## 4. Push

- On `main`: `git push origin main`.
- On a feature branch: `git push -u origin <branch>`. When the user says the
  feature is done: `git switch main && git merge --ff-only <branch> && git
  push origin main && git branch -d <branch> && git push origin --delete
  <branch>`. Not fast-forwardable → `git rebase main` on the branch first
  (only if it was never shared), re-run step 1.
- Rejected because origin moved → `git pull --rebase origin main`, re-run
  `check --changed`, push. Never force-push.

## 5. Watch CI

- `gh run list --branch <branch> --limit 1 --json databaseId,headSha` until
  the run for the pushed sha appears, then `gh run watch <id> --exit-status`
  in the background. Tell the user the commit is pushed and CI is running,
  then carry on; report when it finishes.
- Green → one line: subject, short sha, "CI green".
- Red → `gh run view <id> --log-failed | tail -80`, say what failed and why.
  A real failure is a new change: fix it test-first and commit it through
  this skill once the user agrees. A flaky or environment failure →
  `gh run rerun <id> --failed` once, then report.
- `gh` missing or not signed in → say so, suggest `! brew install gh && gh
  auth login`, and stop after the push.

## 6. Tag (only when asked)

When the user names a milestone: `git tag -a vX.Y.Z -m "<one line>"` on a
green commit, then `git push origin vX.Y.Z`. Starter versions follow semver:
breaking changes to how an app builds on the starter bump the minor while
below 1.0.
