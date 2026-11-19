---
paths:
  - "src/rate_limit.rs"
  - "src/rate_limit/**"
  - "src/auth/actions/**"
  - "src/auth/password.rs"
  - "src/config.rs"
  - "e2e/tests/login-limits.spec.ts"
---

# Rate limits

- **Rate limits** (`src/rate_limit.rs`, sliding windows in the
  `rate_limit_hits` table, so they hold across instances and restarts; read
  with `env.limits()`, every check is `async` and returns
  `Result<bool, sqlx::Error>`): password submissions (login + both set-password forms)
  10/min per IP, email requests (sign-ups + password resets) 5 per 10 min per
  IP, and at most 3 emails per hour to any one address. Failed logins are
  counted per account (only wrong email/password counts, a
  successful login clears it): 5 per 15 min per account+IP, 50 per hour per
  account from any IP — then even the right password gets "Too many
  attempts". Per-IP blocks show a friendly message with status 429; the
  per-email limit silently skips the email (same response, so it reveals
  nothing). `RATE_LIMITS=relaxed` (used by the e2e server) multiplies limits
  by 1000. Client IPs come from Topcoat's `client_ip(cx)` (the connection
  address via `attach_remote_addr`); **behind a proxy/load balancer set
  `TRUSTED_PROXY_HOPS`** to the number of proxies, or every visitor shares
  the proxy's IP.
- API feedback: 20 per hour per token and 100 per day per account
  (`allow_feedback`; see `api-feedback.md`). Data exports: 5 per hour per
  account, counted after re-authentication (`allow_export`; `data-export.md`).
