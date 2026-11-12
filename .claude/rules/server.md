---
paths:
  - "src/main.rs"
  - "src/web.rs"
  - "src/config.rs"
  - "src/app.rs"
  - "src/mail.rs"
  - "src/user/service.rs"
  - ".env.example"
  - "e2e/tests/security.spec.ts"
---

# Server, configuration and email

- **Configuration is parsed once** (`src/config.rs`): `Config::from_env()`
  turns env vars into typed values (`Mailer`, `RateLimitMode`, …) at startup
  and lists every problem at once; nothing else reads env vars (except
  `DATABASE_URL`, read in `main.rs` so `webserver seed` needs nothing else).
  Read it with `config(cx)` / `app_url(cx, path)`. Cookies are `Secure`
  automatically when `APP_BASE_URL` is `https://`. Settings are listed in
  `.env.example`.
- **Security headers** (`main.rs`): every response gets `X-Frame-Options:
  DENY`, `frame-ancestors 'none'`, `nosniff` and a strict referrer policy;
  HSTS is added when `APP_BASE_URL` is https. `inject_globals` adds
  `Cache-Control: no-store` to every response for a signed-in user.
- **One address**: GET/HEAD requests whose `Host` isn't `APP_BASE_URL`'s
  host get a 308 to the same path there (`redirect_to_canonical_host` in
  `web.rs`), so `127.0.0.1:3000` lands on `localhost:3000` (passkeys only
  work on the configured domain). Behind a reverse proxy, the proxy must
  forward the original `Host` header, or every page redirects.
- **Requests run to completion** (`run_to_completion` in `web.rs`, the
  outermost layer): each request runs in its own task, so a client
  disconnecting mid-request can't cancel it halfway. Without this, a dropped
  request could leave a pooled DB connection inside an open transaction
  (sqlx isn't cancellation-safe during `BEGIN`), breaking the next request
  that used it.
- **Logging** defaults to `info` (debug logs every URL, including link
  tokens); set `RUST_LOG` to change it.
- **Email**: `MAIL_TRANSPORT` = `postmark` (default; needs
  `POSTMARK_SERVER_TOKEN`, `POSTMARK_BASE_URL`, `POSTMARK_FROM_EMAIL`),
  `log` (prints emails, including links), or `file` (writes JSON to
  `MAIL_DIR`). Unknown values stop the server at startup.
