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
  `status: draft`; it becomes `status: approved` only when the user says
  so in chat (then the main session commits just the spec and scenarios). Spec first,
  always: any behaviour change, including a bug fix (it adds the scenario
  that would have caught it), starts as a spec edit the user approves.
- **Scenarios** (`e2e/scenarios/<capability>.spec.ts`, same name as the
  spec, tests titled `S1 …`) are written with the spec, before any feature
  code, by the `scenario-writer` agent (a hook blocks it from `src/`), and
  are the independent judge of the build. The main Playwright
  config, `check` and `check-summary.sh --changed` run only scenarios whose
  spec is approved (`support/specs.ts`, `changed_specs.py`); run drafts
  with `npx playwright test -c scenarios.config.ts`.
- **Roles**: the main session interviews, drafts specs and amendments, and
  orchestrates, all in one window; `scenario-writer` writes scenarios;
  `spec-builder` builds an approved spec as a subagent in its own git
  worktree and can't read or edit scenarios or edit specs
  (`.claude/hooks/agent-boundary.sh`); `convention-reviewer` checks its
  branch against the spec. The main session doesn't build specs; the user
  merges the branch.
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
