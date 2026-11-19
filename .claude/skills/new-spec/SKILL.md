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

Set `status: approved` only when the user says, in chat, that the spec is approved.

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

## 5. Write the scenarios (the `scenario-writer` agent)

The scenarios are the independent judge, so you don't write them. Spawn the
`scenario-writer` agent with the spec path and which scenarios are new (all of them for a
new spec). A hook stops it from reading `src/`, so it works from the spec, `e2e/support/`
and the existing pages only. It writes `e2e/scenarios/<capability>.spec.ts` and confirms
each new scenario fails for the right reason.

Check its report against the spec. If it found a gap (a label, wording or behaviour the
spec doesn't fix), fix the spec (still a draft) and have it update the scenario; never
edit the scenarios yourself. Rules a browser can't check are named in the spec as
server-tested; the builder writes those tests and the reviewer checks them.

## 6. Hand over and wait

Show the spec path and a short summary: the rules, the defaults they're accepting, and
what's out of scope. End with exactly one line, `Waiting for your review: specs/<name>.md`,
and stop. Do nothing else until the user answers.

- **"Approved"** (or plainly equivalent): set `status: approved`, add a Changes line, and
  commit only the spec and its scenario file (`Approve <capability> spec`). The user
  authorised this one commit as part of the workflow; nothing else gets committed. Then
  continue with "After approval" in the same turn.
- **Changes or questions**: revise the draft (still `status: draft`), have the
  `scenario-writer` update affected scenarios, hand over again and wait.

## After approval

All in this window; give the user a one-line milestone as each step finishes.

1. Spawn the `spec-builder` agent with `isolation: "worktree"` and the spec path. It works
   in its own git worktree (own e2e port and test database), can't read or edit the
   scenarios or edit specs (hooks), and commits once on its worktree branch.
2. If it reports a conflict (a rule that can't be met as written), draft the spec
   amendment (back to `status: draft`, a Changes line), hand over and wait again.
3. Spawn a fresh `convention-reviewer` to check the branch (`git diff <base>...<branch>`)
   against the spec rule by rule, including the builder's server tests. Send must-fix
   findings back to the builder (`SendMessage`) and review again.
4. Take screenshots from the builder's worktree (`cd <worktree>/e2e && npm run
   screenshot`) and show them.
5. Done = scenarios and `check` pass, the review has no must-fix, and the user has seen
   the screenshots. Report and stop at `Waiting for you to judge and merge <branch>`; the
   user judges the result, not the diff, and merges.
