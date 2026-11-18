---
paths:
  - "src/feedback.rs"
  - "src/feedback/**"
  - "src/admin/pages/feedback_page.rs"
  - "e2e/tests/api-feedback.spec.ts"
---

# API feedback

- Agents tell us what got in their way: `POST /api/v1/feedback` and the MCP
  tool `send_feedback` (`feedback/api.rs`) run `send_api_feedback`.
  `Permission::SendApiFeedback` is `TokenAccess::AnyToken`, so every token
  sees the tool whatever its scopes (the struggling agent never has a
  special scope). The MCP `initialize` instructions and `/llms.txt` tell
  agents to use it.
- Input is a typed `FeedbackCategory`, an optional `endpoint` (operation id
  or MCP tool name, checked against the registry and stored as the
  operation id), an optional `problem_code` (must be a code the catalog
  documents), an optional `request_id` (UUID from `x-request-id`) and a
  message of at most `MAX_FEEDBACK_LENGTH`. Unknown names are refused, so
  the table only holds things that exist.
- The message is untrusted text: `redact_secrets` removes `wsk_…` tokens
  and the word after `Bearer` before storing; nothing reads it back to API
  callers; never feed it to a model as instructions.
- Rate limited per token (20/hour) and per account (100/day) in
  `rate_limit.rs`; each send records `api_feedback_sent` on the account.
- `api_feedback` rows belong to the sender (`ON DELETE CASCADE`), keep the
  token's public id as text and are deleted after 90 days by the cleanup
  job. Admins read the last 200 at `/admin/feedback`.
- We dogfood it: while building API features, call the dev server's MCP
  tools as an outside agent would and send feedback for anything unclear.
