---
paths:
  - "src/*/actions.rs"
  - "src/*/actions/**"
  - "src/user/events.rs"
  - "src/user/event_source.rs"
  - "src/user/event_history.rs"
---

# Account events

- Every account change (including sign-up, email verification and
  answering the passkey prompt) records an `account_events` row with `record_event(env,
  &EventSource::of(caller) /* or ::browser(user_id) */, kind, json!({…}), &mut
  *tx)` in the same transaction. `detail` is a JSON object of public ids and
  names (never secrets). The caller's channel and token id are stored, which
  is what a future "what your apps did" page reads. Add an
  `AccountEventKind` (+ `ALL`) and extend the table's `kind` CHECK (new
  migration). Not events: usage stamps (`last_used_at` on passkeys and
  tokens) and system cleanup (expired links, abandoned sign-ups).
