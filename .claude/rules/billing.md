---
paths:
  - "src/billing.rs"
  - "src/billing/**"
  - "src/stripe.rs"
  - "src/stripe/**"
---

# Billing (Stripe)

- Spec: `specs/choose-a-plan.md` (approved; change it first).
- `src/stripe.rs` is infrastructure: a form-encoded REST client built once
  in `Config` from `STRIPE_SECRET_KEY` + `STRIPE_WEBHOOK_SECRET` (both or
  neither; neither = billing off) and `STRIPE_API_BASE` (no `/v1`). Its
  `Debug` never shows the keys; `get_cached` keeps a GET for 60 s.
  `verify_signature` checks `Stripe-Signature` (HMAC-SHA256 of `t.body`
  keyed by the whole `whsec_…`, 5-minute tolerance).
- Plans are Stripe products with metadata `plan_key` (`catalog.rs`); Free
  is the app's own. Checkout only accepts a price in the active catalog.
- The plan changes only in `actions/sync.rs`, from signed webhooks
  (`POST /webhooks/stripe`): the network lookup of the price happens
  before the transaction; the event id is recorded in the same transaction
  as the change, so a failed apply is retried by Stripe. Events record
  account events with channel `stripe`.
- A subscription event is re-read from Stripe (`GET /v1/subscriptions/{id}`)
  before the transaction and its current state applied; a known
  subscription keeps its stored plan when the price can't be found. One
  live subscription per account (partial unique index, customer row locked
  `FOR UPDATE`); a second is cancelled and refunded by a job, once per Stripe
  subscription id (`duplicate_subscriptions`); `charge_already_refunded` counts as done.
- Pricing shows one `BillingPeriod` (`period.rs`; `?billing=yearly`) at a
  time: `pages/plan_offers.rs` groups prices per plan, picks each plan's
  price for a period and the smallest yearly saving (integer cents);
  `pages/pricing_periods.rs` renders the `segmented` control and hides the
  other period's price with `:hidden` bound to one signal.
- Only a token's or a connected app's Checkout records an event (`checkout_started`).
- Stripe calls never run inside a transaction. Deleting an account
  enqueues `CancelStripeSubscription` in the deletion's transaction; an
  email change enqueues `UpdateStripeCustomerEmail`.
- Checkout and the portal share 10 per hour per account; the public plans
  list 60 per minute per IP. `Idempotency-Key` (API) is stored in
  `checkout_requests` for a day and also sent to Stripe.
- Rust tests run against `testing::FakeStripe` (a local axum server);
  e2e uses `e2e/support/stripe-stub.ts`.
