import type { Page } from "@playwright/test";
import { openAdmin } from "../support/admin";
import { bearer, createToken, createTokenDialog, type Access } from "../support/api";
import {
  ACCOUNT_PASSWORD,
  createAccount,
  logOut,
  setPassword,
  skipPasskeyPrompt,
  uniqueEmail,
} from "../support/flows";
import { verificationLink, waitForEmails } from "../support/mail";
import { stripe, stripeStubUrl, type Delivery } from "../support/stripe";
import { expect, test } from "../support/test";

const PRICING = "/pricing";
const MONTHLY = "/pricing?billing=monthly";
const PLAN = "/account/billing/plan";
const INVOICES = "/account/billing/invoices";
const ACTIVITY = "/account/settings/devices";
const PLAN_NAMES = ["Free", "Pro", "Business"];
const CURRENT = "Current plan";
const ON_FREE = "You're on the Free plan.";
const PAYMENT_RECEIVED = "Payment received. Your plan will update in a moment.";
const CHECKOUT_CANCELLED = "Checkout cancelled. You haven't been charged.";
const PAYMENT_FAILED = "Your last payment failed. Update your card to keep Pro.";
const UNAVAILABLE = "Billing is unavailable right now. Try again in a few minutes.";
const DELETE_WARNING = "Your Pro subscription is cancelled now, with no refund.";
const INVOICES_IN_STRIPE = "Invoices and receipts are in Stripe.";
const BILLING_READ = "Read your billing (billing:read)";
const BILLING_WRITE = "Read your billing and start checkouts (billing:write)";

test.describe.configure({ timeout: 90_000 });

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function onStripe(path: "checkout" | "portal"): RegExp {
  return new RegExp(`^${escapeRegExp(stripeStubUrl())}/${path}/`);
}

function onPath(path: string): RegExp {
  return new RegExp(`^[^?#]*${escapeRegExp(path)}(?:[?#].*)?$`);
}

function dateShown(epochSeconds: number): string {
  return new Date(epochSeconds * 1000).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function isoDate(epochSeconds: number): string {
  const date = new Date(epochSeconds * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function status(value: "Active" | "Past due" | "Cancelling"): RegExp {
  return new RegExp(`Status:\\s*${value}`);
}

function main(page: Page) {
  return page.locator("main");
}

function chooseButtons(page: Page) {
  return main(page).getByRole("button", { name: /^Choose / });
}

function manageBilling(page: Page) {
  return main(page).getByRole("button", { name: "Manage billing" });
}

function planRow(page: Page, name: string) {
  let row = main(page).locator("*").filter({ hasText: new RegExp(`\\b${name}\\b`) });
  for (const other of PLAN_NAMES.filter((plan) => plan !== name)) {
    row = row.filter({ hasNotText: new RegExp(`\\b${other}\\b`) });
  }
  return row;
}

async function openPricing(page: Page) {
  const response = await page.goto(PRICING);
  expect(response?.status(), "the Pricing page exists").toBe(200);
}

async function openPlan(page: Page) {
  await page.goto(PLAN);
  await expect(page.getByRole("heading", { name: "Billing", level: 1 })).toBeVisible();
}

async function expectCurrentOnPricing(page: Page, name: string) {
  await expect(planRow(page, name).filter({ hasText: CURRENT }).first()).toBeVisible();
  for (const other of PLAN_NAMES.filter((plan) => plan !== name)) {
    await expect(planRow(page, other).filter({ hasText: CURRENT })).toHaveCount(0);
  }
}

async function expectOnFree(page: Page, timeout?: number) {
  await expect(main(page).getByText(ON_FREE)).toBeVisible({ timeout });
}

async function expectOnPro(page: Page, value: "Active" | "Past due" | "Cancelling", timeout?: number) {
  await expect(main(page)).toContainText(status(value), { timeout });
  await expect(main(page)).toContainText(/\bPro\b/);
  await expect(manageBilling(page)).toBeVisible();
  await expect(main(page).getByText(ON_FREE)).toHaveCount(0);
}

async function untilPlanPage(page: Page, check: () => Promise<void>) {
  await expect(async () => {
    await openPlan(page);
    await check();
  }).toPass({ timeout: 15_000 });
}

function expectAccepted(deliveries: Delivery[]) {
  for (const delivery of deliveries) {
    expect(delivery.status, `${delivery.type} gets 200`).toBe(200);
  }
}

async function clickChoosePro(page: Page) {
  const response = await page.goto(MONTHLY);
  expect(response?.status(), "the Pricing page exists").toBe(200);
  const monthlyPro = planRow(page, "Pro")
    .filter({ hasText: "$12 / month" })
    .getByRole("button", { name: "Choose Pro" })
    .first();
  await expect(monthlyPro).toBeVisible();
  await monthlyPro.click();
}

async function choosePro(page: Page) {
  await clickChoosePro(page);
  await expect(page).toHaveURL(onStripe("checkout"));
}

async function expectCheckoutForPro(page: Page, email: string) {
  await expect(page).toHaveURL(onStripe("checkout"));
  await expect(page.getByRole("heading", { name: "Subscribe to Pro" })).toBeVisible();
  await expect(page.getByText(email)).toBeVisible();
}

async function leaveStripe(page: Page, button: string) {
  await page.getByRole("button", { name: button, exact: true }).click();
  await page.waitForURL((url) => !url.href.startsWith(stripeStubUrl()));
}

async function payAndConfirm(page: Page, email: string) {
  await leaveStripe(page, "Pay");
  expectAccepted(await stripe.send(email, "checkout_completed"));
  await untilPlanPage(page, () => expectOnPro(page, "Active", 1_000));
}

async function subscribeToPro(page: Page, email: string) {
  await choosePro(page);
  await payAndConfirm(page, email);
  const { subscription } = await stripe.account(email);
  expect(subscription).not.toBeNull();
  return subscription!;
}

async function expectStaysPro(page: Page) {
  await new Promise((resolve) => setTimeout(resolve, 1_000));
  await openPlan(page);
  await expectOnPro(page, "Active");
}

async function newToken(page: Page, name: string, access: Access) {
  await page.goto("/account/developer");
  await page.getByRole("button", { name: "Create API token" }).click();
  await expect(
    createTokenDialog(page).getByLabel("Access").getByRole("option", { name: access, exact: true }),
  ).toBeAttached();
  await page.keyboard.press("Escape");
  const token = await createToken(page, name, access);
  await createTokenDialog(page).getByRole("button", { name: "Done" }).click();
  return token;
}

function activity(page: Page) {
  return page.locator("#activity").getByRole("listitem");
}

async function openUser(page: Page, email: string) {
  await page.getByLabel("Search by email or name").fill(email);
  await page.getByLabel("Search by email or name").press("Enter");
  const row = page.getByRole("row", { name: new RegExp(escapeRegExp(email)) });
  await expect(row).toHaveCount(1);
  await row.getByRole("link").click();
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
}

type JsonLd = Record<string, unknown>;

function jsonLdNodes(value: unknown): JsonLd[] {
  if (Array.isArray(value)) return value.flatMap(jsonLdNodes);
  if (!value || typeof value !== "object") return [];
  const node = value as JsonLd;
  return [node, ...jsonLdNodes(node["@graph"])];
}

async function priceData(page: Page): Promise<JsonLd[]> {
  const scripts = await page.locator('script[type="application/ld+json"]').allTextContents();
  return scripts
    .flatMap((script) => jsonLdNodes(JSON.parse(script)))
    .filter((node) => node["@type"] === "Product");
}

function offersOf(products: JsonLd[], name: string): JsonLd[] {
  const offers = products.find((product) => product.name === name)?.offers;
  return offers === undefined ? [] : ([] as JsonLd[]).concat(offers as JsonLd | JsonLd[]);
}

async function signInHere(page: Page, email: string) {
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(ACCOUNT_PASSWORD);
  await page.getByLabel("Password").press("Enter");
}

test("S1 Pricing lists the plans cheapest first and a new account is on Free", async ({
  page,
}, testInfo) => {
  await page.goto("/");
  const pricingLink = page.getByRole("navigation", { name: "Main" }).getByRole("link", { name: "Pricing" });
  await expect(pricingLink).toBeVisible();
  await pricingLink.click();
  await expect(page).toHaveURL(PRICING);
  await expect(page.getByRole("heading", { name: "Pricing", level: 1 })).toBeVisible();
  const products = await priceData(page);
  expect(products.map((product) => product.name).sort()).toEqual(["Business", "Pro"]);
  expect(offersOf(products, "Pro")).toContainEqual(
    expect.objectContaining({ price: "12.00", priceCurrency: "USD" }),
  );
  expect(offersOf(products, "Business")).toContainEqual(
    expect.objectContaining({ price: "49.00", priceCurrency: "USD" }),
  );

  await expect(planRow(page, "Free").filter({ hasText: "$0" }).first()).toBeVisible();
  await expect(planRow(page, "Pro").filter({ hasText: "$120 / year" }).first()).toBeVisible();
  await expect(
    planRow(page, "Business").filter({ hasText: "$49 / month" }).first(),
  ).toBeVisible();
  await expect(chooseButtons(page)).toHaveText(["Choose Pro", "Choose Business"]);
  await expect(main(page).getByText("Legacy add-on")).toHaveCount(0);
  await expect(main(page).getByText(CURRENT)).toHaveCount(0);
  const text = await main(page).innerText();
  expect(text.indexOf("$0")).toBeLessThan(text.indexOf("$120 / year"));
  expect(text.indexOf("$120 / year")).toBeLessThan(text.indexOf("$49 / month"));

  await createAccount(page, testInfo, "Plan Browser");
  await openPricing(page);
  await expectCurrentOnPricing(page, "Free");

  await openPlan(page);
  await expectOnFree(page);
  await expect(main(page).getByRole("link", { name: "See plans" })).toHaveAttribute(
    "href",
    onPath(PRICING),
  );

  await page.goto(INVOICES);
  await expect(main(page).getByText("No invoices yet.")).toBeVisible();
  await expect(manageBilling(page)).toHaveCount(0);
});

test("S2 paying in Checkout moves the account to Pro once Stripe confirms", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Payer");
  await choosePro(page);

  await leaveStripe(page, "Pay");

  await expect(page).toHaveURL(onPath(PLAN));
  await expect(main(page).getByText(PAYMENT_RECEIVED)).toBeVisible();
  await expectOnFree(page);
  await page.evaluate(() => {
    (window as unknown as { stayedOnPage: boolean }).stayedOnPage = true;
  });

  expectAccepted(await stripe.send(email, "checkout_completed"));

  await expectOnPro(page, "Active", 15_000);
  expect(
    await page.evaluate(() => (window as unknown as { stayedOnPage?: boolean }).stayedOnPage),
    "the page updated without a reload",
  ).toBe(true);
  const { subscription } = await stripe.account(email);
  await expect(main(page)).toContainText(`Renews ${dateShown(subscription!.current_period_end)}`);
  await expect(page.getByText(PAYMENT_RECEIVED)).toHaveCount(0);

  await openPricing(page);
  await expect(chooseButtons(page)).toHaveCount(0);
  await expectCurrentOnPricing(page, "Pro");
  await expect(manageBilling(page)).toBeVisible();
});

test("S3 cancelling in Checkout leaves the account on Free", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Plan Decliner");
  await choosePro(page);

  await leaveStripe(page, "Cancel");

  await expect(page).toHaveURL(onPath(PLAN));
  await expect(main(page).getByText(CHECKOUT_CANCELLED)).toBeVisible();
  await expectOnFree(page);
});

test("S4 a failed renewal warns and keeps Pro until Stripe ends it", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Late Payer");
  await subscribeToPro(page, email);

  expectAccepted(await stripe.send(email, "renewal_failed"));

  await untilPlanPage(page, () =>
    expect(main(page).getByText(PAYMENT_FAILED)).toBeVisible({ timeout: 1_000 }),
  );
  await expectOnPro(page, "Past due");

  expectAccepted(await stripe.send(email, "ended"));

  await untilPlanPage(page, () => expectOnFree(page, 1_000));
  await expect(page.getByText(PAYMENT_FAILED)).toHaveCount(0);

  await page.goto(INVOICES);
  await expect(main(page).getByText(INVOICES_IN_STRIPE)).toBeVisible();
  await expect(manageBilling(page)).toBeVisible();
  await expect(main(page).getByText("No invoices yet.")).toHaveCount(0);
});

test("S5 cancelling in the portal keeps Pro until the period ends", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Canceller");
  const subscription = await subscribeToPro(page, email);

  await manageBilling(page).click();
  await expect(page).toHaveURL(onStripe("portal"));
  await leaveStripe(page, "Cancel subscription");

  const until = `Pro until ${dateShown(subscription.current_period_end)}, then Free.`;
  await untilPlanPage(page, () =>
    expect(main(page)).toContainText(until, { timeout: 1_000 }),
  );
  await expectOnPro(page, "Cancelling");

  expectAccepted(await stripe.send(email, "ended"));

  await untilPlanPage(page, () => expectOnFree(page, 1_000));
  await expect(main(page)).not.toContainText(until);
});

test("S6 each billing change is listed in Recent activity", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Historian");
  const subscription = await subscribeToPro(page, email);
  expectAccepted(await stripe.send(email, "renewal_failed"));
  expectAccepted(await stripe.send(email, "cancel_at_period_end"));
  expectAccepted(await stripe.send(email, "ended"));
  await untilPlanPage(page, () => expectOnFree(page, 1_000));

  await page.goto(ACTIVITY);

  const ends = dateShown(subscription.current_period_end);
  for (const entry of [
    "Subscribed to Pro",
    "Payment failed for Pro",
    `Cancelled Pro (ends ${ends})`,
    "Pro ended, back on Free",
  ]) {
    await expect(activity(page).filter({ hasText: entry }), entry).toHaveCount(1);
  }
});

test("S7 deleting a Pro account warns and cancels the subscription at Stripe", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Leaver");
  const subscription = await subscribeToPro(page, email);

  await page.goto("/account");
  await page.getByRole("button", { name: "Delete account" }).click();
  const dialog = page.getByRole("dialog", { name: "Delete account" });
  await expect(dialog.getByText(DELETE_WARNING)).toBeVisible();
  await dialog.getByLabel("Type DELETE to confirm").fill("DELETE");
  await dialog.getByLabel("Current password").fill(ACCOUNT_PASSWORD);
  await dialog.getByLabel("Current password").press("Enter");
  await expect(page).toHaveURL("/login");

  await expect
    .poll(
      async () =>
        (await stripe.requests(email)).filter(
          (request) =>
            request.method === "DELETE" && request.path === `/v1/subscriptions/${subscription.id}`,
        ).length,
      { timeout: 15_000 },
    )
    .toBeGreaterThan(0);
});

test("S8 choosing a plan while Stripe is unreachable shows an error on Pricing", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Offline");
  await stripe.unreachable(email);

  await clickChoosePro(page);

  await expect(page.getByText(UNAVAILABLE).first()).toBeVisible();
  await expect(page).toHaveURL(onPath(PRICING));
  expect((await stripe.requests(email)).length).toBeGreaterThan(0);
  await openPricing(page);
  await expectCurrentOnPricing(page, "Free");
});

test("S9 the API returns the plan to billing:read tokens only", async ({
  page,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Scripter");
  const subscription = await subscribeToPro(page, email);
  const reader = await newToken(page, "Billing reader", BILLING_READ);
  const other = await createToken(page, "Profile reader");
  await createTokenDialog(page).getByRole("button", { name: "Done" }).click();

  const response = await request.get("/api/v1/billing", { headers: bearer(reader) });

  expect(response.status()).toBe(200);
  expect(await response.json()).toMatchObject({
    plan: "pro",
    plan_name: "Pro",
    status: "active",
    renews_on: isoDate(subscription.current_period_end),
    ends_on: null,
  });

  const refused = await request.get("/api/v1/billing", { headers: bearer(other) });
  expect(refused.status()).toBe(403);
  expect(await refused.json()).toMatchObject({ status: 403, code: "insufficient_scope" });
});

test("S10 admins see the plan, its status and a Stripe link, with no billing actions", async ({
  page,
  browser,
}, testInfo) => {
  const userPage = await (await browser.newContext()).newPage();
  const email = await createAccount(userPage, testInfo, "Plan Watched");
  await subscribeToPro(userPage, email);
  await userPage.context().close();
  const { customer } = await stripe.account(email);

  await openAdmin(page);
  await openUser(page, email);

  await expect(main(page)).toContainText(/Plan:\s*Pro\b/);
  await expect(main(page)).toContainText(status("Active"));
  await expect(main(page).getByRole("link", { name: "Open in Stripe" })).toHaveAttribute(
    "href",
    `https://dashboard.stripe.com/test/customers/${customer}`,
  );
  await expect(
    main(page).getByRole("button", {
      name: /Choose|Manage billing|Cancel subscription|Refund|Change plan/i,
    }),
  ).toHaveCount(0);
});

test("S11 without JavaScript Choose Pro reaches Checkout", async ({ browser }, testInfo) => {
  const page = await (await browser.newContext({ javaScriptEnabled: false })).newPage();
  await createAccount(page, testInfo, "Plan Plain");

  await clickChoosePro(page);

  await expect(page).toHaveURL(onStripe("checkout"));
  await expect(page.getByRole("button", { name: "Pay", exact: true })).toBeVisible();
  await page.context().close();
});

test("S12 forged or replayed webhooks saying the subscription ended leave it on Pro", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Guarded");
  await subscribeToPro(page, email);

  expect((await stripe.forge(email, "wrong")).status).toBe(400);
  expect((await stripe.forge(email, "none")).status).toBe(400);
  await expectStaysPro(page);

  const ended = await stripe.send(email, "ended");
  expectAccepted(ended);
  await untilPlanPage(page, () => expectOnFree(page, 1_000));
  await subscribeToPro(page, email);

  const deleted = ended.find((delivery) => delivery.type === "customer.subscription.deleted")!;
  expect((await stripe.replay(deleted.id)).status).toBe(200);
  await expectStaysPro(page);
});

test("S13 a billing:write token starts a Checkout that makes Pro current once paid", async ({
  page,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Agent");
  const writer = await newToken(page, "Shopping agent", BILLING_WRITE);
  const reader = await newToken(page, "Billing reader", BILLING_READ);

  const plans = await request.get("/api/v1/billing/plans");
  expect(plans.status()).toBe(200);
  const list = await plans.json();
  expect(list).toMatchObject({ next_cursor: null });
  const offered: Record<string, unknown>[] = list.data;
  const pro = offered.find((plan) => plan.plan === "pro" && plan.interval === "month");
  expect(pro).toMatchObject({ plan_name: "Pro", amount: 1200, currency: "usd", interval: "month" });

  const refused = await request.post("/api/v1/billing/checkout", {
    headers: bearer(reader),
    data: { price: pro!.price },
  });
  expect(refused.status()).toBe(403);
  expect(await refused.json()).toMatchObject({ status: 403, code: "insufficient_scope" });

  const started = await request.post("/api/v1/billing/checkout", {
    headers: bearer(writer),
    data: { price: pro!.price },
  });
  expect(started.status()).toBe(201);
  const { checkout_url: checkoutUrl } = await started.json();
  expect(checkoutUrl).toMatch(onStripe("checkout"));

  const billing = await request.get("/api/v1/billing", { headers: bearer(writer) });
  expect(billing.status()).toBe(200);
  expect(await billing.json()).toMatchObject({ plan: "free", status: "free" });

  await page.goto(checkoutUrl);
  await expectCheckoutForPro(page, email);
  await payAndConfirm(page, email);

  await page.goto(ACTIVITY);
  await expect(
    activity(page).filter({ hasText: "Checkout for Pro started by token Shopping agent" }),
  ).toHaveCount(1);
});

test("S14 the API refuses a second Checkout and an unknown price", async ({
  page,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Repeat Agent");
  const writer = await newToken(page, "Repeat agent", BILLING_WRITE);

  const unknown = await request.post("/api/v1/billing/checkout", {
    headers: bearer(writer),
    data: { price: "price_not_offered" },
  });
  expect(unknown.status()).toBe(422);
  expect(await unknown.json()).toMatchObject({ status: 422, code: "unknown_price" });

  await subscribeToPro(page, email);

  const again = await request.post("/api/v1/billing/checkout", {
    headers: bearer(writer),
    data: { price: "price_pro_monthly" },
  });
  expect(again.status()).toBe(409);
  expect(await again.json()).toMatchObject({ status: 409, code: "already_subscribed" });
  const againUnknown = await request.post("/api/v1/billing/checkout", {
    headers: bearer(writer),
    data: { price: "price_not_offered" },
  });
  expect(againUnknown.status()).toBe(409);
  expect(await againUnknown.json()).toMatchObject({ status: 409, code: "already_subscribed" });
  await openPlan(page);
  await expectOnPro(page, "Active");
});

test("S15 anyone can list the plans, but billing needs a token", async ({ request }) => {
  const plans = await request.get("/api/v1/billing/plans");

  expect(plans.status()).toBe(200);
  const offered: Record<string, unknown>[] = (await plans.json()).data;
  expect([...new Set(offered.map((plan) => plan.plan))].sort()).toEqual(["business", "pro"]);
  expect(offered.find((plan) => plan.plan === "business")).toMatchObject({
    plan_name: "Business",
    amount: 4900,
    currency: "usd",
    interval: "month",
  });

  const billing = await request.get("/api/v1/billing");
  expect(billing.status()).toBe(401);
});

test("S16 a visitor who chooses Pro and signs in lands in Checkout for Pro", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plan Returner");
  await logOut(page, "Plan Returner");

  await clickChoosePro(page);

  await expect(page).toHaveURL(onPath("/login"));
  await signInHere(page, email);
  await expectCheckoutForPro(page, email);
});

test("S17 a visitor who chooses Pro and registers lands in Checkout and can pay", async ({
  page,
  browser,
}, testInfo) => {
  const email = uniqueEmail(testInfo, "Plan-Newcomer");

  await clickChoosePro(page);
  await expect(page).toHaveURL(onPath("/login"));
  await main(page).getByRole("link", { name: "Register" }).click();
  await page.getByLabel("Name").fill("Plan Newcomer");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Email").press("Enter");
  await expect(page.getByText("Check your inbox")).toBeVisible();
  const other = await (await browser.newContext()).newPage();
  await other.goto(verificationLink((await waitForEmails(email))[0]));
  await setPassword(other, ACCOUNT_PASSWORD);

  await expectCheckoutForPro(other, email);
  await payAndConfirm(other, email);
  await other.context().close();
});

function billedYearly(page: Page) {
  return main(page).getByRole("switch", { name: "Billed yearly" });
}

function onMonthly(): RegExp {
  return new RegExp(`${escapeRegExp(MONTHLY)}$`);
}

async function chooseProCheckout(page: Page, email: string) {
  await main(page).getByRole("button", { name: "Choose Pro" }).click();
  await expectCheckoutForPro(page, email);
}

async function checkoutPrices(email: string) {
  return (await stripe.requests(email))
    .filter((request) => request.method === "POST" && request.path === "/v1/checkout/sessions")
    .map((request) => request.body["line_items[0][price]"]);
}

test("S18 the Billed yearly switch shows yearly prices and checks out the shown price", async ({
  page,
}, testInfo) => {
  await openPricing(page);
  await expect(billedYearly(page)).toHaveCount(1);
  await expect(planRow(page, "Pro").getByRole("switch", { name: "Billed yearly" }).first()).toBeVisible();
  await expect(planRow(page, "Business").getByRole("switch")).toHaveCount(0);
  await expect(billedYearly(page)).toBeChecked();
  await expect(planRow(page, "Pro").filter({ hasText: "$120 / year" }).first()).toBeVisible();
  await expect(main(page).getByText("$12 / month")).toBeHidden();
  await expect(main(page).getByText("$10 / month")).toBeHidden();
  await expect(
    planRow(page, "Business").filter({ hasText: "$49 / month" }).first(),
  ).toBeVisible();
  await expect(chooseButtons(page)).toHaveText(["Choose Pro", "Choose Business"]);
  await page.evaluate(() => {
    (window as unknown as { stayedOnPage: boolean }).stayedOnPage = true;
  });

  await billedYearly(page).click();

  await expect(billedYearly(page)).not.toBeChecked();
  await expect(page).toHaveURL(onMonthly());
  await expect(planRow(page, "Pro").filter({ hasText: "$12 / month" }).first()).toBeVisible();
  await expect(main(page).getByText("$120 / year")).toBeHidden();
  await expect(
    planRow(page, "Business").filter({ hasText: "$49 / month" }).first(),
  ).toBeVisible();
  expect(
    await page.evaluate(() => (window as unknown as { stayedOnPage?: boolean }).stayedOnPage),
    "the switch flipped without a reload",
  ).toBe(true);

  const email = await createAccount(page, testInfo, "Plan Yearly");
  await openPricing(page);
  await expect(billedYearly(page)).toBeChecked();
  await chooseProCheckout(page, email);
  expect(await checkoutPrices(email)).toEqual(["price_pro_yearly"]);

  await openPricing(page);
  await billedYearly(page).click();
  await expect(billedYearly(page)).not.toBeChecked();
  await expect(planRow(page, "Pro").filter({ hasText: "$12 / month" }).first()).toBeVisible();
  await chooseProCheckout(page, email);
  expect(await checkoutPrices(email)).toEqual(["price_pro_yearly", "price_pro_monthly"]);
});

test("S19 without JavaScript Show monthly prices loads the monthly prices", async ({ browser }) => {
  const page = await (await browser.newContext({ javaScriptEnabled: false })).newPage();
  await openPricing(page);
  const showMonthly = main(page).getByRole("link", { name: "Show monthly prices" });
  await expect(showMonthly.first()).toBeVisible();

  await showMonthly.first().click();

  await expect(page).toHaveURL(onMonthly());
  await expect(planRow(page, "Pro").filter({ hasText: "$12 / month" }).first()).toBeVisible();
  await expect(main(page).getByText("$120 / year")).toBeHidden();
  await expect(main(page).getByRole("link", { name: "Show yearly prices" }).first()).toBeVisible();
  await page.context().close();
});

function chooseFree(page: Page) {
  return main(page).getByRole("link", { name: "Choose Free" });
}

test("S20 a visitor who chooses Free registers and lands on their account on Free", async ({
  page,
}, testInfo) => {
  const email = uniqueEmail(testInfo, "Plan-Freebie");
  await openPricing(page);
  const freeLink = planRow(page, "Free").getByRole("link", { name: "Choose Free" }).first();
  await expect(freeLink).toBeVisible();

  await freeLink.click();

  await expect(page).toHaveURL(onPath("/register"));
  await page.getByLabel("Name").fill("Plan Freebie");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Email").press("Enter");
  await expect(page.getByText("Check your inbox")).toBeVisible();
  await page.goto(verificationLink((await waitForEmails(email))[0]));
  await setPassword(page, ACCOUNT_PASSWORD);
  await skipPasskeyPrompt(page);

  await openPlan(page);
  await expectOnFree(page);
  expect(await checkoutPrices(email)).toEqual([]);
  expect((await stripe.account(email)).subscription).toBeNull();

  await openPricing(page);
  await expectCurrentOnPricing(page, "Free");
  await expect(chooseFree(page)).toHaveCount(0);
});

test("S20 a Pro account sees neither Current plan nor Choose Free on Free", async ({
  page,
}, testInfo) => {
  await openPricing(page);
  await expect(chooseFree(page).first()).toBeVisible();

  const email = await createAccount(page, testInfo, "Plan Pro Free");
  await subscribeToPro(page, email);

  await openPricing(page);
  await expectCurrentOnPricing(page, "Pro");
  await expect(planRow(page, "Free").filter({ hasText: CURRENT })).toHaveCount(0);
  await expect(chooseFree(page)).toHaveCount(0);
});

test("S20 without JavaScript Choose Free reaches Register", async ({ browser }) => {
  const page = await (await browser.newContext({ javaScriptEnabled: false })).newPage();
  await openPricing(page);
  const freeLink = planRow(page, "Free").getByRole("link", { name: "Choose Free" }).first();
  await expect(freeLink).toBeVisible();

  await freeLink.click();

  await expect(page).toHaveURL(onPath("/register"));
  await expect(page.getByLabel("Email")).toBeVisible();
  await page.context().close();
});
