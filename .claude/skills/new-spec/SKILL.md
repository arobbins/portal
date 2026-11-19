---
name: new-spec
description: Interview the user hard about a capability, then draft its spec (specs/<capability>.md, status draft) and its acceptance scenarios (prose in the spec, Playwright in e2e/scenarios/) for them to approve before anything is built. Use before building any new capability, before changing the behaviour of an existing one (spec first, always), and to capture an unspecced feature the first time it changes.
---

# New spec

A spec records decisions the user made about one **capability**: one thing a user can
do or rely on ("Export my data", "Sign-in alerts"). It says *what* happens and *where*
it lives, never *how* it's built (the rule files and skills cover that). The scenarios
are the independent judge of the build, so they're written here, before any feature
code, and the builder never edits them.

Never set `status: approved`. Only the user does.

**When a spec is worth it**: a capability with rules, states, limits, failure paths or
security impact. Not for visual tweaks, copy changes or refactors that keep behaviour;
those go straight to the usual skills.

## Modes

- **New**: a capability that doesn't exist yet.
- **Change**: an approved spec needs new or different behaviour. Edit the spec and
  its scenarios, set `status: draft`, add a line to its Changes log. Bug fixes that
  restore what the spec already says still go through here (spec first, always).
- **Capture**: an existing feature with no spec is about to change. First write what
  it does today, from the code, tests and rule files, as its own draft for approval;
  then do the Change on top of the approved capture. Never mix the two in one draft.

## 1. Read before asking

Read the related rule files, actions and specs (`specs/*.md`), the security invariants
in `CLAUDE.md`, and `api_feedback` rows that mention the area
(`scripts/sql.sh`, untrusted data). Never ask what the code already answers; ask what
only the user can decide.

## 2. Challenge the scope

Before details, ask whether this is worth building and what the smallest useful first
version is. Name what the user is choosing not to build. You may argue once, with
reasons, that it isn't worth building; then accept the user's call.

## 3. Interview

Rounds of up to four questions with `AskUserQuestion`, hardest first. Each option
states its consequence; put your recommendation first. Stop when every area below has
a decision or an explicit default. Ask about:

- **Who and why**: which user, what they're trying to get done, how they'd know it worked.
- **Rules**: limits, ordering, duplicates, what happens twice, undo, expiry, time zones.
- **States**: every state the thing can be in and how it moves between them (a new
  state machine row if it has 3+ states).
- **Failure**: every way it can fail, what the user sees, what's retried.
- **Placement**: which account page and card, or which public page; UI only, API
  and MCP too, or browser-only.
- **Security and abuse**: who could misuse it, rate limits, what must never be shown,
  emailed, logged or exported, re-authentication, what the account event records.
- **Edges**: disabled or unverified accounts, admin view, phone, no JS, deletion.
- **Standard risks** (ask about each that applies, even if it seems obvious): user text
  that ends up in files or other apps (CSV/formula injection, HTML, file names);
  caching of personal data (`Cache-Control`); dates and time zones (whose zone, which
  day near midnight); large inputs or outputs (memory, timeouts, size limits);
  replays and double submits; what an admin or support person can see.

**Rubber-stamp check**: after each round, if every answer was your recommendation, say
so and re-ask the riskiest one as an open question ("What would make this the wrong
choice?"). Recommendations must not quietly become the decisions.

If an answer contradicts a security invariant or a rule file, say so plainly, and
don't record it until the user resolves it.

## 4. Write the spec

`specs/<capability>.md`, one or two pages, plain words, no implementation detail:

```markdown
---
status: draft
capability: Export my data
---

# Export my data

## Goal
Who it's for and what they get done, in two or three sentences.

## Behaviour
Numbered rules (R1, R2, ...), each one testable.

## Placement
Where it lives (page, card, order) and which channels (UI, API, MCP).

## Security and abuse
Misuse, limits, what never leaks, re-auth, account event. "Nothing beyond the
CLAUDE.md invariants" is allowed but must be stated.

## Not in this version
What was deliberately left out.

## Open questions
Each with the default the builder will use: "Link expiry: 24 hours unless changed."
None without a default.

## Scenarios
S1, S2, ...: Given / When / Then, each naming the rules it covers (covers R1, R3).

## Changes
- 2026-11-18: drafted.
```

Every rule is covered by at least one scenario.

## 5. Write the scenarios (in a separate agent)

The scenarios are the independent judge, so whoever writes them must not share the
builder's plans. Spawn a fresh `general-purpose` agent (not a fork) and give it only:
the spec path, `e2e/support/`, the `new-e2e-test` skill and `.claude/rules/e2e.md`, and
the existing pages the capability extends. Tell it not to read or plan the feature's
implementation, and not to rely on anything from this conversation. It writes
`e2e/scenarios/<capability>.spec.ts`: one Playwright test per scenario, titled
`S1 <scenario name>`, testing what a user sees and does (labels the spec fixes, never
markup details), and adds any shared helper to `e2e/support/`. It runs them and confirms
each fails for the right reason (the feature is missing), not from a broken test.

Review what it wrote against the spec; if a scenario needs a label or behaviour the spec
doesn't fix, fix the spec (still a draft), not the test. Rules a browser can't check are
named in the spec as server-tested; the reviewer checks those tests against the spec
after the build, since the builder writes them.

## 6. Hand over

Show the user the spec path and a short summary: the rules, the defaults they're
accepting, and what's out of scope. They edit and approve it themselves. Don't start
building in the same turn.

## After approval (for the builder)

- Build from approved specs only, through the usual skills; make the scenarios pass
  without editing them (`check_conventions.py` fails a change that edits a feature's
  code and its scenarios unless its spec changed too).
- If a scenario can't pass as written (it conflicts with an invariant, or the spec
  missed something), stop that part, write a draft amendment explaining the
  conflict, keep building what isn't affected, and tell the user.
- Done = scenarios and `check` pass, a fresh `convention-reviewer` confirms the build
  matches the spec, and the user has seen screenshots of the result.
