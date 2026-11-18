---
paths:
  - "src/secret.rs"
  - "src/api_tokens/**"
  - "src/passkeys/**"
  - "src/*/queries.rs"
---

# Public ids (`src/secret.rs`)

Rows a browser or API can name (API tokens, passkeys) have a `public_id`
(`PublicIdKind::ApiToken` → `tok_…`, `Passkey` → `pk_…`, 20 base62
characters, DB format CHECK). Forms, URLs, events and API responses use it;
the integer `id` never leaves the server. Actions check the format with
`kind.is_valid(id)` before querying and always pair it with the owner.
`random_base62` and `hash_secret` are the only random-secret helpers.
