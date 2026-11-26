---
status: approved
capability: Choose a plan and pay
---

# Choose a plan and pay

## Goal
Anyone can see the plans on a public Pricing page; a signed-in account moves from the
Free plan to a paid plan by paying through Stripe, and later manages or cancels it in
Stripe's own billing portal. Visitors sign in or register first and then go straight on
to pay. Prices and plan names live in Stripe, so they can change without a deploy; the
app shows them, records which plan each account is on, keeps that record in step with
Stripe, and lets AI agents discover plans and start a Checkout for their user.

## Behaviour
- R1 Every account is on exactly one plan. Accounts without a subscription are on
  **Free**; Free is the app's own plan and isn't in Stripe.
- R2 Paid plans are the active Stripe products marked with a plan key (product metadata
  `plan_key`, e.g. `pro`). The key is the plan's stable name; its display name and price
  come from Stripe and may change. Each active recurring price of the product is offered.
  Products without a key are ignored.
- R3 The public **Pricing** page (`/pricing`, heading "Pricing", signed in or out) lists Free ("$0") and
  every paid plan with its name, price and interval ("$12 / month"), cheapest first. For a
  signed-in account, the current plan's row carries the badge "Current plan". The page
  also carries the plans as machine-readable price data for agents and search engines:
  schema.org `Product` JSON-LD per paid plan, with an `Offer` per price (`price` "12.00",
  `priceCurrency` "USD").
- R4 Each paid price has a button **Choose <plan name>** (e.g. "Choose Pro"), shown to
  signed-out visitors and to accounts on Free. For a signed-in account it goes to Stripe
  Checkout for that price. The Stripe customer is created on first use with the account's
  email and is reused afterwards. Checkout uses Stripe's automatic payment methods (no
  fixed list), so Link, Stripe's one-click wallet that AI agents such as Meta's Muse and
  Grok Bot pay with, and other wallets appear when they're enabled in the Stripe Dashboard.
- R5 For a signed-out visitor, Choose is a link to Sign in, which links to Register.
  The chosen price is remembered (it travels with the registration link, so it works even
  if the link is opened in another browser): signing in goes straight on to Checkout for it, and
  registering does too once they've followed the emailed link and set a password
  (before the passkey prompt). The choice lasts as long as the registration link is
  valid, and nothing is bought until they pay in Checkout.
- R6 Returning from Checkout after paying shows the notice "Payment received. Your plan
  will update in a moment." until Stripe confirms; returning after cancelling shows
  "Checkout cancelled. You haven't been charged." Both returns land on the Plan and
  payment page with the message as an inline notice, and neither changes the plan. With
  JavaScript, the page checks every few seconds after paying and updates in place, without
  reloading, as soon as Stripe confirms; without it, a reload does.
- R7 The plan changes only when Stripe sends a signed event. Unsigned, wrongly signed,
  repeated or out-of-order events never change state or move it backwards. A valid event
  gets 200; a bad or missing signature gets 400; an event id already processed gets 200
  and is ignored; events for an unknown customer get 200 and are ignored (server-tested
  for out-of-order and unknown customers).
- R8 The Plan and payment page shows the current plan. On Free it says "You're on the
  Free plan." with a **See plans** link to `/pricing`. With a subscription it shows the
  plan, "Status: Active" (or Cancelling, Past due), the next renewal date in the
  account's time zone ("Renews Dec 25, 2026"), and **Manage billing**, which opens
  Stripe's customer portal (change plan, update card, invoices, cancel). While a
  subscription exists no second Checkout can start: Pricing shows no Choose buttons for
  that account, only "Current plan" on its plan and Manage billing.
- R9 States: Free → Active (payment confirmed) → Cancelling (cancelled, paid until the
  period ends: "Pro until <date>, then Free.") → Free when the period ends. Active →
  Past due when a renewal fails; the plan keeps its paid access and the page warns "Your
  last payment failed. Update your card to keep <plan name>." Past due → Active when paid,
  → Free when Stripe ends the subscription. Changing plan in the portal keeps the state
  and updates the plan.
- R10 Every billing change is an account event shown in the account's **Recent
  activity**: "Subscribed to Pro", "Changed plan to Business", "Cancelled Pro (ends Dec
  25, 2026)", "Payment failed for Pro", "Pro ended, back on Free", and "Checkout for Pro
  started by token <name>" when a token starts one.
- R11 Deleting an account with a subscription, by its owner or by an admin, cancels it
  at Stripe immediately with no refund. The delete warning adds "Your <plan name> subscription is cancelled now, with
  no refund." Deletion doesn't wait for Stripe's confirming event; if Stripe can't be
  reached, deletion still completes and the cancellation is retried until it succeeds
  (server-tested).
- R12 If Stripe can't be reached when choosing a plan or opening Manage billing, the
  user sees the error "Billing is unavailable right now. Try again in a few minutes." on
  the page they came from and nothing changes.
- R13 If billing isn't configured (no Stripe keys), Pricing shows only Free, the Plan and
  payment page says "Billing isn't set up yet." with no buttons, and the rest of the app
  works (server-tested: the e2e server always has billing configured).
- R14 The API and MCP can read billing and start a Checkout, so an AI agent can put its
  user on a plan and hand them the link to pay:
  - `GET /api/v1/billing/plans` (no token needed) / MCP `list_plans`: `{ "data":
    [...], "next_cursor": null }`, each paid price as `plan`, `plan_name`, `price` (the
    Stripe price id), `amount` (in cents), `currency` (lowercase, e.g. `usd`),
    `interval`.
  - `GET /api/v1/billing` / MCP `get_billing`: `plan` (key, `free` on Free),
    `plan_name`, `status` (`free`, `active`, `cancelling`, `past_due`), `renews_on` and
    `ends_on` (`YYYY-MM-DD` in the account's time zone, or null).
  - `POST /api/v1/billing/checkout` with `price` / MCP `start_checkout`: returns 201
    with `checkout_url`, a Stripe Checkout link for the token's account, which the user
    (or their agent's wallet) completes; returning from it behaves as in R6. Refused with
    409 `already_subscribed` while a subscription exists (checked first), 422
    `unknown_price` for a price that isn't offered, and 503 `billing_unavailable` when
    Stripe can't be reached or billing isn't configured. An `Idempotency-Key` header
    returns the same link for a repeated request (server-tested).
  - Reading billing needs the new scope `billing:read` ("Read your billing
    (billing:read)"); starting a Checkout needs `billing:write` ("Read your billing and
    start checkouts (billing:write)"), which implies read. Other tokens get 403 with code
    `insufficient_scope`. Only the plans list works without a token. Nothing on the API
    or MCP opens the portal, changes or cancels a plan. A missing token gets 401, as
    elsewhere in the API.
- R15 Admins see "Plan: Pro" and "Status: Active" (etc.) on the admin user page, with an
  "Open in Stripe" link to `https://dashboard.stripe.com/customers/<id>` (`/test/` before
  `customers` with a test key); they can't change billing in the app.
- R16 Choose, Manage billing and the sign-in hand-off work without JavaScript (plain
  links and form posts that redirect).
- R17 If a second subscription arrives for an account that already has a live one (two
  Checkouts completed at once), the second is cancelled at Stripe and its payment fully
  refunded, once, and Recent activity (also shown to admins) records "Second Pro
  subscription cancelled and refunded". A refund Stripe reports as already made counts as
  done; when there's nothing paid to refund yet, it's cancelled without a refund and a
  warning is logged for the server (server-tested, except the log line).
- R18 Changing the account's email updates the Stripe customer's email (server-tested).

## Placement
- **Pricing** (`/pricing`): a public page linked as "Pricing" from the site header
  (signed in and out) and from the Plan and payment page's See plans link.
- Billing section, **Plan and payment** page (`/account/billing/plan`, the existing name):
  a "Your plan" card (R8, R9 notices) replacing the Subscriptions and Payment methods
  placeholders.
- **Invoices** page (`/account/billing/invoices`): its card says "Invoices and receipts
  are in Stripe." and offers Manage billing to any account that has ever paid, including
  one back on Free; an account that never paid sees "No invoices yet."
- The account menu's billing link goes to the Plan and payment page.
- The webhook is a public `POST /webhooks/stripe`.
- Channels: UI to choose and manage; API and MCP to read and start a Checkout (R14).

## Security and abuse
- Card details never touch the app; Checkout and the portal are Stripe-hosted.
- Only a signed-in account (or its token) can start a Checkout, always for itself; no
  purchase happens without an account.
- Webhooks are verified with the signing secret; the event id is recorded so a replay
  is ignored. The Stripe secret key and webhook secret come only from config and never
  appear in logs, pages, exports or errors.
- The portal session is created for the account's own Stripe customer only, at click
  time, and never stored.
- The remembered price is only a price id, never trusted: it's checked against the
  offered prices when Checkout starts. It never changes which account pays.
- Starting Checkout (page, API or MCP) or the portal is limited to 10 times per hour per
  account; the public plans list to 60 requests per minute per IP (server-tested; the
  e2e server relaxes rate limits).
- Account billing pages are personal data and are never cached (`Cache-Control:
  no-store`); the signed-out Pricing page may be cached briefly.
- No re-authentication: Stripe handles payment authentication (3-D Secure).
- Admins can't read card or invoice details through the app.
- A token can only create a Checkout link; someone still has to pay it, and the plan
  changes only through Stripe's signed events (R7).

## Not in this version
Buying without an account; teams or seats paying; trials; coupons; usage-based prices;
app-sent billing emails (Stripe sends receipts and failed-payment emails); prorated
refunds and refunds other than R17; feature gating by plan; billing in the data export; plan switching
inside the app; tax settings (configured in Stripe); Stripe's agent payment tools
(Shared Payment Tokens, the Agentic Commerce Suite catalog, the Universal Commerce
Protocol), which don't yet support Stripe subscriptions or are in preview. Agent wallets
pay with single-use cards, so such a subscription becomes Past due at its first renewal
until the user adds a card in the portal (R9); no special handling.

## Open questions
- Admin disables an account: its subscription is left as is, unless changed.
- Link in subscription-mode Checkout is confirmed in Stripe's sandbox before launch; if
  it isn't offered there, cards and wallets still work.
- Stripe test mode in development. The e2e suite never uses the network: the app reads
  Stripe's base URL from config, and e2e points it at a local stub in `e2e/support/`
  (written with the scenarios) that serves products Pro ($12 / month, key `pro`) and
  Business ($49 / month, key `business`), hosted Checkout and portal pages, signed
  webhooks (with the e2e webhook secret) sent on demand, a recorded request log, and a
  switch to make it unreachable.

## Scenarios
- S1 Given a signed-out visitor, when they open `/pricing` (linked as "Pricing" in the
  header), then Free ("$0"), Pro and Business are listed with their prices, cheapest
  first, with "Choose Pro" and "Choose Business"; given a new account, the same page marks
  Free "Current plan", its Plan and payment page says "You're on the Free plan." with See
  plans, and its Invoices page says "No invoices yet." (covers R1, R2, R3, R8)
- S2 Given a Free account, when it clicks "Choose Pro" on Pricing and pays in Checkout,
  then it lands on Plan and payment with "Payment received. Your plan will update in a
  moment.", and once Stripe confirms, the page shows Pro, "Status: Active", a renewal
  date and "Manage billing" without a reload, and Pricing shows no Choose buttons.
  (covers R4, R6, R7, R8, R9)
- S3 Given a Free account, when it cancels in Checkout, then it sees "Checkout
  cancelled. You haven't been charged." and is still on Free. (covers R6)
- S4 Given a Pro account, when its renewal fails, then the page warns "Your last payment
  failed. Update your card to keep Pro." and Pro stays current; when Stripe ends the
  subscription, it's back on Free and the Invoices page still offers Manage billing.
  (covers R9, Placement)
- S5 Given a Pro account, when it cancels in the portal, then the page shows "Pro until
  <date>, then Free." and "Status: Cancelling"; when the period ends, it's on Free.
  (covers R8, R9)
- S6 Given an account that subscribed to Pro, had a renewal fail, cancelled and ended,
  when it opens Recent activity, then "Subscribed to Pro", "Payment failed for Pro",
  "Cancelled Pro (ends <date>)" and "Pro ended, back on Free" are listed. (covers R10)
- S7 Given a Pro account, when it opens Delete account, then the warning says "Your Pro
  subscription is cancelled now, with no refund."; after deleting, Stripe has received the
  cancellation. (covers R11)
- S8 Given Stripe is unreachable, when a Free account clicks "Choose Pro", then it stays
  on Pricing with "Billing is unavailable right now. Try again in a few minutes." and is
  still on Free. (covers R12)
- S9 Given a Pro account and a token with `billing:read`, when it calls
  `GET /api/v1/billing`, then it gets the plan and status fields; a token without the
  scope is refused. (covers R14)
- S10 Given an admin, when they open a Pro user's admin page, then they see "Plan: Pro",
  "Status: Active" and an "Open in Stripe" link, and no billing actions. (covers R15)
- S11 Given JavaScript is off, when a Free account clicks "Choose Pro", then it reaches
  Checkout. (covers R16)
- S12 Given a Pro account, when a forged or replayed webhook says the subscription ended,
  then it stays on Pro. (covers R7)
- S13 Given a Free account and a token with `billing:write`, when it lists plans and
  starts a Checkout for Pro's price, then it gets a `checkout_url` that opens Checkout for
  Pro, paying it makes Pro current, and Recent activity lists "Checkout for Pro started
  by token <name>"; a `billing:read` token gets 403 `insufficient_scope`. (covers R10,
  R14)
- S14 Given a Pro account and a `billing:write` token, when it starts a Checkout, then
  it gets 409 `already_subscribed`; an unknown price gets 422 `unknown_price`. (covers R14)
- S15 Given no token, when anyone calls `GET /api/v1/billing/plans`, then it gets Pro and
  Business; `GET /api/v1/billing` without a token is refused. (covers R14)
- S16 Given a signed-out visitor with an account, when they click "Choose Pro" on Pricing
  and sign in, then they land in Checkout for Pro. (covers R5)
- S17 Given a signed-out visitor without an account, when they click "Choose Pro",
  follow Register, open the emailed link and set a password, then they land in Checkout
  for Pro, and paying makes Pro current. (covers R5)

## Changes
- 2026-11-26: the duplicate-subscription refund (R17) and email sync (R18) moved from
  open questions into rules, as decided; admin deletion named in R11.
- 2026-11-25: approved.
- 2026-11-25: public Pricing page; signed-out visitors sign in or register first and go
  straight on to Checkout; Plan and payment shows the current plan with See plans; public
  plans endpoint; billing changes named as Recent activity entries.
- 2026-11-25: Link via automatic payment methods; API and MCP can list plans and start
  a Checkout (`billing:write`).
- 2026-11-25: drafted; wording, webhook responses, auto-update and Invoices access added
  after the scenario writer's report.
