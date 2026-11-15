---
name: world-class-review
description: Judge whether this codebase meets a "world class" software-quality bar — score ten dimensions (correctness and tests, security, design, reliability, performance, accessibility, supply chain, delivery, documentation, developer experience) from 0 to 4 against fixed level descriptors, citing evidence for every score, and give a yes/no verdict with the gaps that block it, ranked. Read-only. Use when asked "is this world class?", "how good is this codebase?", "is it production ready?", or for an outside-in quality assessment; use clean-code-audit instead to clean the code up.
---

# World-class review

A verdict, not a cleanup. Don't change code; at the end, offer to fix the
top gaps. Every score cites evidence (`file:line`, a command and its output,
or a missing file). If something can't be measured, write **not measured**
and score that criterion 0. Don't guess, and don't install tools without
asking.

## 1. Gather evidence

Run these and keep the output:

```sh
git status --short | wc -l; git log --oneline | wc -l
python3 scripts/clean_code.py --report
cargo audit
grep -rn --include=*.rs -E '\.unwrap\(\)|\.expect\(|panic!|todo!|unimplemented!' src | grep -v tests
grep -rn --include=*.rs 'unsafe' src
ls .github/workflows README.md LICENSE deny.toml 2>&1
```

Then the `check` skill (the full `./check.sh`). Note its wall-clock time.
For coverage, `cargo llvm-cov --summary-only` and `cargo mutants --list`
if they're installed; otherwise they're **not measured**.

## 2. Score each dimension (0–4)

The levels are the same everywhere: **0** absent, **1** ad hoc, **2** solid
(a good professional team), **3** strong (held by automated checks, few
gaps), **4** world class (exemplary: the bar other projects copy, enforced
so it can't regress). A 4 needs both the practice *and* a check that fails
when it slips.

| # | Dimension | What a 4 looks like here |
|---|---|---|
| 1 | Correctness and tests | Every action has Rust tests for success, each `ActionProblem` and authorization; every page and flow has a Playwright spec; line coverage ≥ 85% on actions and queries; mutation testing kills most mutants; no flaky tests |
| 2 | Security | The CLAUDE.md invariants each have a test that would fail if broken; `cargo audit` clean; CSRF, security headers, rate limits and session rotation tested (`security.spec.ts`); no `unwrap` on input; secrets only via `config.rs`; a threat model or `SECURITY.md` |
| 3 | Design and maintainability | `clean_code.py` shows no violations with headroom under the limits; feature boundaries held by scripts; one error contract; read the five largest files and check them against the `clean-code-audit` checklist (step 2), citing findings |
| 4 | Reliability and operations | Health and readiness checks; graceful shutdown; structured logs with request ids; jobs are idempotent with retry and backoff; migrations are forward-only and tested on a fresh DB; config fails fast at startup; no `unwrap`/`expect` outside startup and tests |
| 5 | Performance | No N+1 queries; an index for every `WHERE`/`JOIN` column (check `migrations/`); a pool sized from config; assets fingerprinted, compressed and cached; page weight and response-time budgets that a test checks |
| 6 | Accessibility and UX | Every input has a label; keyboard and focus order work in modals; contrast passes in light and dark; an automated a11y check (`@axe-core/playwright`) in e2e; 390px layout tested |
| 7 | Supply chain | Toolchain and lockfile pinned; minimum publish age; `cargo audit` and `cargo deny` (licenses, duplicates, bans) in CI; no unused dependencies |
| 8 | Delivery | CI runs the whole of `./check.sh` on every push; the main branch is protected; builds are reproducible; a deploy and rollback path is written down; the change history reads well (small commits that say why) |
| 9 | Documentation | A README that takes a new developer from a fresh clone to a running app; architecture and decisions (CLAUDE.md, `.claude/rules/`, `state-machines.md`) match the code; public API docs generated from code (`/docs/api`, OpenAPI) |
| 10 | Developer experience | One command to run and one to check; check time under ~5 minutes; fast, clear errors from the convention scripts; skills and hooks cover repeated work; seed data |

For each dimension, write the score, two or three lines of evidence, and
what is missing for the next level.

## 3. Verdict

- **World class**: every dimension ≥ 3, the average ≥ 3.5, and no critical
  gap (a broken security invariant, failing checks, or no CI).
- **Strong, not yet world class**: every dimension ≥ 2.
- **Not there yet**: anything lower.

Be strict: a 4 is rare and needs proof. Starter-project size isn't an
excuse; judge what exists against the bar.

## 4. Report

1. The verdict in one sentence, with the average.
2. A table: dimension, score, the one-line reason.
3. **Gaps that block the verdict**, ranked by impact per effort, each with
   the concrete fix (file, tool or test) and the dimension it raises.
4. What is already exemplary: things worth keeping as they are.
5. Measurements to compare against next time (scores, `clean_code.py`
   numbers, check time, coverage if measured).

Offer to publish the report as a page, and to fix the top gaps (one at a
time, test first, following CLAUDE.md).
