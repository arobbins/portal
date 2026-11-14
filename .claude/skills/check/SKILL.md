---
name: check
description: Run the full ./check.sh (format, conventions, clippy, sqlx data, Rust tests, Playwright in three browsers) and get back only what failed. Use before calling work done, instead of running ./check.sh directly, so a long passing log never fills the context.
---

# Check

```sh
scripts/check-summary.sh
```

- Passing: one line with the test counts. Report it and stop.
- Failing: the step that failed, then only its errors. For Playwright, each failing test's
  error details from `e2e/test-results/*/error-context.md` and its screenshot path (open the
  screenshot with Read when the error is visual).
- The full log is in `target/check.log`; read a slice of it only when the summary isn't
  enough (`grep -n` for the test name first).
- It takes a few minutes; run it in the background and wait for the notification.
- After fixing, re-run the narrowest thing first (one spec with the `verify` skill, or
  `cargo test <name>`), then this skill again.
