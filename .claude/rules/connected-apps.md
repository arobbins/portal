---
paths:
  - "src/oauth.rs"
  - "src/oauth/**"
  - "src/auth/handoff.rs"
  - "src/mcp.rs"
---

# Connected apps (OAuth sign-in for `/mcp`)

- Spec: `specs/connect-an-ai-assistant.md` (approved; change it first).
- The app is its own OAuth 2.1 authorization server for `/mcp` only:
  metadata at `/.well-known/oauth-protected-resource` (also `/…/mcp`) and
  `/.well-known/oauth-authorization-server` (`oauth/routes.rs`), Dynamic
  Client Registration (`POST /oauth/register`, public clients only, `cli_…`
  ids; unknown metadata is ignored as RFC 7591 requires) or an `https`
  Client ID Metadata Document (`metadata_document.rs`: public hosts only
  (no NAT64, 6to4 or IPv4-compatible v6), no proxy,
  the checked address pinned for the fetch, 5 s, 5 KB, no redirects,
  cached an hour in `oauth_clients.fetched_at`). Unapproved clients are
  deleted by cleanup after 24 hours.
- `/oauth/authorize` validates the client and exact redirect URI first
  (else the 400 "Can't connect this app" page, never a redirect), then
  `response_type=code`, PKCE `S256` and `resource` = `app.url("/mcp")`
  (else an error redirect), stores an `oauth_requests` row (`oar_…`) and
  redirects to `?request=oar_…`. Signed out → `/login?authorize=oar_…`;
  the shared sign-in hand-off (`auth/handoff.rs`, also Checkout's `price`)
  carries it through sign-in, registration (the verification link's
  `payload`) and back, before the passkey prompt. A request lives 10
  minutes after it was last acted on (`keep_request_alive`).
- Approving (`connect_app`, `Permission::ManageConnectedApps`, browser
  only) grants only requested, ticked, non-admin scopes
  (`offered_by_sign_in`), asks `check_reauth` when a granted scope
  `can_write` (the screen shows the field only while a ticked scope ends
  in `:write`, in a fieldset disabled otherwise), upserts one `oauth_connections` row per user and client
  (max 50), stores the code's hash on the request, records `app_connected`
  and emails the account. Deny (`deny_app`) deletes the request and
  returns `access_denied`; Deny is its own form so the passkey re-auth
  script never intercepts it.
- Secrets (`secrets.rs`): `wsc_` codes (10 min, single use: consumed with
  `DELETE … RETURNING` before any check; the exchange must repeat the
  stored `redirect_uri`), `wsa_` access tokens (1 hour),
  `wsr_` refresh tokens (30 idle days), all checksummed like `wsk_` and
  stored only as SHA-256 hashes in `oauth_tokens`. A refresh replaces both;
  a replaced refresh token used again ends the connection
  (`end_connection`, `app_disconnected` with channel `app`). `/oauth/revoke`
  (either token, own client only) ends it too. The token endpoint allows 60
  requests per minute per client; errors are `{error, error_description}`.
- `/mcp` without a token answers `initialize`, `ping`, `tools/list` (only
  tools with `run_public`) and public tools; anything else is 401 with
  `WWW-Authenticate: Bearer resource_metadata="…"`. A `wsa_` token becomes
  `Caller::connected_app` (`Channel::App`): scopes like a token, never
  browser-only permissions, never on `/api/v1`, rejected for disabled
  accounts. Its events store channel `app` and the app's label in
  `detail.app`, shown as "By Claude (claude.ai)".
- A password reset, Sign out other devices and an admin's sign out
  everywhere delete every connection; a password change keeps them;
  deleting the account cascades. `oauth_connections` and `oauth_requests`
  are in the data export's `LEFT_OUT` until `export-my-data` adds a file.
- Tests: `oauth/*/tests.rs` (helpers in `oauth/testing.rs`),
  `server/oauth_tests.rs` (headers, metadata, anonymous MCP); e2e helpers
  `e2e/support/oauth.ts` and `loopback.ts`.
