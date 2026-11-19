---
paths:
  - "specs/*.md"
  - "e2e/scenarios/*.spec.ts"
  - "e2e/scenarios.config.ts"
  - "e2e/support/specs.ts"
  - "e2e/support/zip.ts"
---

# Feature specs and acceptance scenarios

- **A spec** (`specs/<capability>.md`) is one capability the user decided on:
  goal, numbered rules (R1…), placement, security and abuse, what's left
  out, open questions with defaults, and scenarios (S1…) naming the rules
  they cover. It's written with the `new-spec` skill and starts as
  `status: draft`; only the user sets `status: approved`. Spec first,
  always: any behaviour change, including a bug fix (it adds the scenario
  that would have caught it), starts as a spec edit the user approves.
- **Scenarios** (`e2e/scenarios/<capability>.spec.ts`, same name as the
  spec, tests titled `S1 …`) are written with the spec, before any feature
  code, by a fresh agent that sees only the spec and the e2e support (never
  the builder's plans), and are the independent judge of the build. The main Playwright
  config, `check` and `check-summary.sh --changed` run only scenarios whose
  spec is approved (`support/specs.ts`, `changed_specs.py`); run drafts
  with `npx playwright test -c scenarios.config.ts`.
- **The builder never edits scenarios.** `check_conventions.py` fails when
  a scenario changes alongside `src/`, `assets/` or `migrations/` while its
  spec didn't. Commit an approved spec and its scenarios before building,
  so the guard sees the build on its own.
- **When a scenario can't pass as written** (it conflicts with an
  invariant or a rule file, or the spec missed something): stop that part,
  draft an amendment to the spec explaining the conflict (back to
  `status: draft`, a line in its Changes log), keep building what isn't
  affected, and tell the user.
- **Rules a browser can't check** (rate limits, which the e2e server
  relaxes; build-time completeness) are covered by server tests the spec
  names; everything else needs a scenario. The builder writes those tests,
  so the reviewer checks them against the spec rule by rule.
- Specs are for capabilities with rules, states, limits or security impact,
  not visual tweaks, copy changes or behaviour-preserving refactors.
- **Done** = scenarios and `check` pass, a fresh `convention-reviewer`
  confirms the build matches the spec rule by rule, and the user has seen
  screenshots of the result. The user judges the result, not the diff.
- `e2e/support/zip.ts` (`readZip`) reads a downloaded ZIP's files as text.
