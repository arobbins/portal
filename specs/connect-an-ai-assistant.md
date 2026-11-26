---
status: approved
capability: Connect an AI assistant by signing in
---

# Connect an AI assistant by signing in

## Goal
A person using an AI assistant (Claude, ChatGPT, Cursor or any MCP client) connects it
to their account by signing in and approving what it may do, instead of creating and
pasting a token. When a tool needs an account, the assistant is told where to sign in,
the person signs in or registers in their browser, approves, and the assistant carries
on. They can see and disconnect connected assistants at any time.

## Behaviour
- R1 The MCP endpoint (`/mcp`) follows the MCP authorization standard (OAuth 2.1): the
  app is both the MCP server and its own authorization server. It publishes Protected
  Resource Metadata (`/.well-known/oauth-protected-resource`, also under `/mcp`) and
  Authorization Server Metadata (`/.well-known/oauth-authorization-server`; the issuer is
  the app's origin), with the
  standard field names (`resource`, `authorization_servers`; `issuer`,
  `authorization_endpoint`, `token_endpoint`, `registration_endpoint`,
  `revocation_endpoint`, `code_challenge_methods_supported: ["S256"]`), and token
  responses use the standard `access_token`, `refresh_token`, `token_type`,
  `expires_in`, `scope`. The MCP protocol version is unchanged.
- R2 Without a token, `initialize`, `ping`, `tools/list` and public tools work;
  `tools/list` shows exactly the public tools (today `list_plans`). Calling any other tool gets HTTP
  401 with `WWW-Authenticate: Bearer resource_metadata="<metadata URL>"`, so the client
  starts sign-in and then continues.
- R3 Any MCP client may register itself, by Dynamic Client Registration (`POST
  /oauth/register`, JSON `client_name` and `redirect_uris`, answered 201 with
  `client_id`; public clients only, `token_endpoint_auth_method: "none"`) or by an
  `https` client id URL pointing at its Client ID Metadata Document (server-tested). Redirect URIs must be `https`, or `http` on a loopback host; they're matched
  exactly. Registered clients that never get approved are deleted after 24 hours.
- R4 Authorization uses the authorization code flow with PKCE (`S256` only) and the
  `resource` parameter, which must be this app's MCP URL. A wrong or unregistered
  redirect URI shows a 400 page headed "Can't connect this app" ("This app asked to
  return to an address it didn't register.") and never redirects. A wrong PKCE verifier,
  reused or expired code (10 minutes), wrong resource, or a refresh token that's unknown,
  already replaced or disconnected gets 400 `invalid_grant`.
- R5 Approval screen (`/oauth/authorize`), heading "Connect <name>": the client's name
  with the label "Unverified name", the host it returns to in bold, without a port
  ("Returns to claude.ai", "Returns to 127.0.0.1"), and each requested scope as a ticked
  checkbox labelled as in the token form ("Read your profile (profile:read)"); the person
  can untick any. Buttons **Allow access** (the form's default, first) and **Deny**.
  Scopes the app doesn't know are ignored. Deny returns `access_denied` to the client and connects nothing.
- R6 When any ticked scope can write, the approval screen shows a "Current password"
  field (and "Use a passkey" for accounts with one) above the buttons, and Allow access
  checks it first, as creating a token does; a wrong one shows "Your current password is
  incorrect.", connects nothing, and counts as a failed sign-in. Read-only grants need no re-authentication.
- R7 Admin-only scopes (none exist yet) are never offered or granted through sign-in,
  even to admins (server-tested with a test scope). Scopes the client didn't ask for are
  never added. A sign-in token missing a scope is treated like a hand-made token today:
  `tools/list` hides tools it can't use and calling one gives an `isError` result with
  `insufficient_scope`.
- R8 A signed-out person opening the approval screen is sent to Sign in (`/login`, which
  links to Register) and returns to the approval screen after signing in. Registering
  resumes it too, once they've followed the emailed link and set a password (before the
  passkey prompt), even in another browser, as long as the request hasn't
  expired (10 minutes after they last acted on it, or the client's own timeout).
- R9 Approval issues an access token valid for 1 hour and a refresh token. Each refresh
  replaces both; a refresh token lapses after 30 days unused. Reusing an already-replaced
  refresh token disconnects the app (both tokens stop working). Tokens are stored hashed,
  never shown on any page, and work only on `/mcp` (never `/api/v1`) with the granted
  scopes (server-tested for expiry).
- R10 The Developer tab has a **Connected apps** card (`#connected-apps`) listing each
  connection: name, return host, granted scopes (labels), connected date and last used;
  with none, "No apps connected." Each row's **Disconnect app** opens a modal "Disconnect
  app" whose **Disconnect app** button ends it at once. `/oauth/revoke` (200, with either the access or the refresh token) ends it too. Connecting the same client again updates that connection's
  scopes rather than adding a second. At most 50 connections per account.
- R11 Recent activity records "Connected Claude (claude.ai)" and "Disconnected Claude
  (claude.ai)"; actions a connected app takes are recorded as today, naming the app
  ("by Claude (claude.ai)") where a token's name would appear (server-tested).
- R12 Connecting an app emails the account "Claude was connected to your account", with
  the granted scopes and a link to `/account/developer#connected-apps`.
- R13 A password reset or signing out everywhere (the person's own "Sign out other
  devices", or an admin's) disconnects every app; a routine
  password change keeps them. A disabled account's apps stop working; deleting the
  account deletes them (server-tested for disabled and deleted).
- R14 Hand-made API and MCP tokens keep working exactly as today.

## Placement
- `/.well-known/oauth-protected-resource`, `/.well-known/oauth-authorization-server`,
  `/oauth/register`, `/oauth/authorize` (approval screen, a public-layout page),
  `/oauth/token`, `/oauth/revoke`.
- Developer tab: a Connected apps card after the API tokens card.
- MCP only; the JSON API keeps hand-made tokens.

## Security and abuse
- The client's name is self-chosen and labelled unverified; the return domain is what
  the person judges by. Redirects go only to exactly registered URIs.
- PKCE S256 required; codes single use, 10 minutes; tokens audience-bound to `/mcp`
  through `resource`; refresh tokens rotate with reuse detection (R9).
- Limits: registration 20 per hour per IP; the token endpoint 60 per minute per client;
  approvals share the failed-sign-in limits; 50 connections per account; codes,
  expiry, Client ID Metadata fetching and caching/framing headers (server-tested; the e2e server relaxes rate limits).
- Connected apps can never change security settings (password, passkeys, two-factor,
  tokens, connections), as API tokens can't today.
- Tokens are never logged, put in a flash, the session or a URL (the code travels in the
  redirect URL once, as OAuth requires, and is single use).
- The approval screen is never cached and can't be framed.

## Not in this version
Signing in for `/api/v1`; admin scopes by sign-in; verified or featured apps; per-tool
approvals; social sign-in; device-code flow for assistants without a browser.

## Open questions
- Clients that only start sign-in on `initialize` (not on a later 401) won't prompt
  until a protected tool is called; most current clients handle the later 401. Default:
  keep R2 as written.
- Client ID Metadata Documents are fetched with a 5-second timeout and 5 KB limit, and
  only from `https` URLs on public hosts (server-tested).

## Scenarios
- S1 Given no token, when a client sends `initialize` and `tools/list` to `/mcp`, then
  both work and only `list_plans` is listed; calling `get_profile` gets 401 whose
  `resource_metadata` URL leads to metadata naming this app's authorization server and
  its `/oauth/register`, `/oauth/authorize` and `/oauth/token`. (covers R1, R2)
- S2 Given a signed-in account and a client registered through `/oauth/register`, when
  it starts authorization with PKCE, then the approval screen shows "Connect <name>",
  "Unverified name", "Returns to <domain>" and ticked read scopes; after unticking one and
  clicking Allow access, the client gets a code, exchanges it for tokens, and
  `get_profile` works while the unticked scope's tool is refused. (covers R3, R4, R5, R7)
- S3 Given a client asking for `billing:write`, when the person clicks Allow access,
  then the password is asked for first; a wrong one is refused and nothing connects; the
  right one connects. (covers R6)
- S4 Given the approval screen, when the person clicks Deny, then the client receives
  `access_denied` and Connected apps is empty. (covers R5)
- S5 Given a connected client, when it refreshes, then it gets new tokens that work;
  when the old refresh token is used again, then the app is disconnected and the new
  tokens stop working. (covers R9)
- S6 Given a connected app, when the person opens Connected apps and disconnects it,
  then its tokens stop working and Recent activity shows "Connected …" then
  "Disconnected …". (covers R10, R11)
- S7 Given a signed-out visitor without an account, when a client sends them to
  authorize and they register, open the emailed link in a fresh browser and set a
  password, then they land on the approval screen and connecting works. (covers R8)
- S8 Given a person connects an app, then they receive "<name> was connected to your
  account" listing the scopes and linking to Connected apps. (covers R12)
- S9 Given a connected app, when the person changes their password, then it still
  works; when they sign out everywhere, or reset their password, then its tokens stop
  working and Connected apps says "No apps connected." (covers R13)
- S10 Given a token from sign-in, when it's used on `/api/v1`, then it's refused; a
  hand-made MCP token still works on `/mcp`. (covers R9, R14)
- S11 Given a signed-out person with an account, when a client sends them to authorize,
  then they're sent to Sign in and, after signing in, land on the approval screen and
  connecting works. (covers R8)
- S12 Given a client, when it authorizes with an unregistered redirect URI, then an
  error page is shown and no redirect happens; when it exchanges a code with the wrong
  PKCE verifier or a used code, then it gets `invalid_grant`. (covers R4)
- S13 Given the same client connects twice, then Connected apps lists one connection
  with the newer scopes; when the client calls `/oauth/revoke` with its refresh token,
  then that connection ends. (covers R10)

## Changes
- 2026-11-26: approved.
- 2026-11-26: drafted; wording, field names, error pages and scenarios filled in after
  the scenario writer's report.
