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

The user approves by answering: the interview plus one final question about what you
filled in yourself (step 6). Set `status: approved` only after that answer.

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

Every rule is covered by at least one scenario. Before the scenarios are written, fix
every label, message, button name, path and field name a scenario will need to find or
assert, so the scenario writer doesn't have to report them as gaps.

## 5. Write the scenarios (the `scenario-writer` agent)

The scenarios are the independent judge, so you don't write them. Spawn the
`scenario-writer` agent with the spec path and which scenarios are new (all of them for a
new spec). A hook stops it from reading `src/`, so it works from the spec, `e2e/support/`
and the existing pages only. It writes `e2e/scenarios/<capability>.spec.ts` and confirms
each new scenario fails for the right reason.

Check its report against the spec. If it found a gap, fix the spec (still a draft) and
have it update the scenario; never edit the scenarios yourself. Fill wording-level gaps
yourself (labels, messages, field names, paths) and add them to the "filled in" list for
step 6. A gap that changes behaviour (a rule, a limit, who can do what) is the user's
decision: ask it as an interview question. Rules a browser can't check are named in the spec as
server-tested; the builder writes those tests and the reviewer checks them.

## 6. Final question = approval

Ask one last `AskUserQuestion`: "Here's what I filled in myself; build with these?",
listing only what the user didn't decide in the interview: defaults, fixed wording and
labels, field names and paths, and any gaps filled after the scenario writer's report.
Options: build with them (recommended), or change some (the user says which).

- **Build**: their interview answers plus this answer are the approval. Set
  `status: approved`, add a Changes line, commit only the spec and its scenario file
  (`Approve <capability> spec`; a standing authorisation for that one commit), and
  continue with "After approval" in the same turn. Every commit message in this
  workflow is one terse, human-readable line, with no body or trailers.
- **Change some**: revise the draft, have the `scenario-writer` update affected
  scenarios, and ask the final question again.

## After approval

All in this window; give the user a one-line milestone as each step finishes.

1. Worktrees start from `origin/main`, not the local branch. Fast-forward `main` to the
   current branch and push it (`git push origin main`); the user authorised this, but
   only as a fast-forward: if `origin/main` has commits the branch lacks, stop and ask. Then spawn the `spec-builder` agent with
   `isolation: "worktree"` and the spec path; it first checks the approval commit is there. It works
   in its own git worktree (own e2e port and test database), can't read or edit the
   scenarios or edit specs (hooks), and commits once on its worktree branch.
2. If it reports a conflict (a rule that can't be met as written), draft the spec
   amendment (back to `status: draft`, a Changes line), hand over and wait again.
3. Spawn a fresh `convention-reviewer` to check the branch (`git diff <base>...<branch>`)
   against the spec rule by rule, including the builder's server tests. Meanwhile, run
   the full `check` (all browsers) in the builder's worktree. Must-fix findings or a red
   check go back to the builder (`SendMessage`); then review and check again.
4. Merge when everything is green: scenarios and full `check` pass and the review has no
   must-fix. The user authorised this. Merge locally into their current branch with
   `git merge --no-ff <branch> -m "Merge <capability>"` (one merge commit, so
   `git revert -m 1 <sha>` backs it out), then remove the worktree and branch. If the branch isn't based on the current
   HEAD, rerun `check` on the merged result and revert if it's red. Never push.
   Should-fix findings become a follow-up the user decides on; they don't block.
5. Take screenshots from the main checkout (`cd e2e && npm run screenshot`) and report:
   the merge commit, how to revert it, the screenshots, should-fix follow-ups, and any
   wording you filled in. The user judges the live result on the dev server.
