import { createHmac, randomBytes } from "node:crypto";
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";

type Json = Record<string, unknown>;
type Fields = Record<string, string>;

const PORT = Number(process.env.STRIPE_STUB_PORT);
const PUBLIC_URL = process.env.STRIPE_STUB_PUBLIC_URL ?? `http://localhost:${PORT}`;
const APP_URL = process.env.STRIPE_STUB_APP_URL ?? "";
const SECRET_KEY = process.env.STRIPE_SECRET_KEY ?? "";
const WEBHOOK_SECRET = process.env.STRIPE_WEBHOOK_SECRET ?? "";
const WEBHOOK_PATH = "/webhooks/stripe";
const WRONG_SECRET = "whsec_not_the_real_signing_secret";

const PRODUCTS: Json[] = [
  product("prod_business", "Business", { plan_key: "business" }, "price_business_monthly"),
  product("prod_addon", "Legacy add-on", {}, "price_addon_monthly"),
  product("prod_pro", "Pro", { plan_key: "pro" }, "price_pro_monthly"),
];
const PRICES: Json[] = [
  price("price_business_monthly", "prod_business", 4900),
  price("price_addon_monthly", "prod_addon", 500),
  price("price_pro_monthly", "prod_pro", 1200),
];

const customers = new Map<string, Json>();
const sessions = new Map<string, Json>();
const portals = new Map<string, Json>();
const subscriptions = new Map<string, Json>();
const unreachable = new Set<string>();
const requests: Json[] = [];
const events: Json[] = [];
let clock = 0;

function product(id: string, name: string, metadata: Fields, defaultPrice: string): Json {
  return { id, object: "product", active: true, name, metadata, default_price: defaultPrice };
}

function price(id: string, productId: string, amount: number): Json {
  return {
    id,
    object: "price",
    active: true,
    currency: "usd",
    unit_amount: amount,
    type: "recurring",
    recurring: { interval: "month", interval_count: 1 },
    product: productId,
  };
}

function newId(prefix: string): string {
  return `${prefix}_${randomBytes(12).toString("hex")}`;
}

function now(): number {
  return Math.floor(Date.now() / 1000);
}

function eventTime(): number {
  clock = Math.max(now(), clock + 1);
  return clock;
}

function oneMonthAtNoon(): number {
  const date = new Date();
  date.setUTCMonth(date.getUTCMonth() + 1);
  date.setUTCHours(12, 0, 0, 0);
  return Math.floor(date.getTime() / 1000);
}

function list(data: Json[], url: string): Json {
  return { object: "list", data, has_more: false, url };
}

function send(res: ServerResponse, status: number, body: unknown) {
  res.writeHead(status, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

function html(res: ServerResponse, title: string, body: string) {
  res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
  res.end(
    `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="icon" href="data:,">` +
      `<title>${title}</title></head><body><main><h1>${title}</h1>${body}</main></body></html>`,
  );
}

function redirect(res: ServerResponse, location: string) {
  res.writeHead(303, { Location: location });
  res.end();
}

function stripeError(res: ServerResponse, status: number, message: string) {
  send(res, status, { error: { type: "invalid_request_error", message } });
}

async function readBody(req: IncomingMessage): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(chunk as Buffer);
  return Buffer.concat(chunks).toString("utf8");
}

function parseFields(raw: string, contentType: string): Fields {
  if (!raw) return {};
  if (contentType.includes("application/json")) {
    const fields: Fields = {};
    const flatten = (value: unknown, key: string) => {
      if (value && typeof value === "object") {
        for (const [k, v] of Object.entries(value)) flatten(v, key ? `${key}[${k}]` : k);
      } else fields[key] = String(value);
    };
    flatten(JSON.parse(raw), "");
    return fields;
  }
  return Object.fromEntries(new URLSearchParams(raw));
}

function prefixed(fields: Fields, prefix: string): Fields {
  const out: Fields = {};
  for (const [key, value] of Object.entries(fields)) {
    const match = key.match(new RegExp(`^${prefix.replace(/[[\]]/g, "\\$&")}\\[([^\\]]+)\\]$`));
    if (match) out[match[1]] = value;
  }
  return out;
}

function emailOfCustomer(id: unknown): string | undefined {
  return typeof id === "string" ? (customers.get(id)?.email as string | undefined) : undefined;
}

function emailFor(path: string, fields: Fields): string | undefined {
  if (fields.email) return fields.email.toLowerCase();
  if (fields.customer_email) return fields.customer_email.toLowerCase();
  if (fields.customer) return emailOfCustomer(fields.customer);
  const id = path.split("/").find((part) => /^(cus|sub|cs)_/.test(part));
  if (!id) return undefined;
  if (id.startsWith("cus_")) return emailOfCustomer(id);
  if (id.startsWith("sub_")) return emailOfCustomer(subscriptions.get(id)?.customer);
  return sessions.get(id)?.email as string | undefined;
}

function priceWithProduct(priceId: string, expand: boolean): Json | undefined {
  const found = PRICES.find((p) => p.id === priceId);
  if (!found) return undefined;
  return expand ? { ...found, product: PRODUCTS.find((p) => p.id === found.product) } : found;
}

function createCustomer(email: string, fields: Fields = {}): Json {
  const customer = {
    id: newId("cus"),
    object: "customer",
    email,
    name: fields.name ?? null,
    metadata: prefixed(fields, "metadata"),
    created: now(),
  };
  customers.set(customer.id, customer);
  return customer;
}

function latestSubscription(email: string): Json | undefined {
  return [...subscriptions.values()]
    .filter((sub) => emailOfCustomer(sub.customer) === email)
    .sort((a, b) => (b.created as number) - (a.created as number))[0];
}

function invoice(sub: Json, status: string, reason: string): Json {
  return {
    id: newId("in"),
    object: "invoice",
    customer: sub.customer,
    subscription: sub.id,
    parent: { type: "subscription_details", subscription_details: { subscription: sub.id } },
    status,
    billing_reason: reason,
    amount_due: (sub.items as { data: { price: Json }[] }).data[0].price.unit_amount,
    attempt_count: 1,
    currency: "usd",
    created: now(),
  };
}

function signature(payload: string, secret: string, timestamp = now()): string {
  const v1 = createHmac("sha256", secret).update(`${timestamp}.${payload}`).digest("hex");
  return `t=${timestamp},v1=${v1}`;
}

async function deliver(payload: string, header: string | null): Promise<number> {
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (header) headers["Stripe-Signature"] = header;
  try {
    const response = await fetch(`${APP_URL}${WEBHOOK_PATH}`, {
      method: "POST",
      headers,
      body: payload,
    });
    return response.status;
  } catch {
    return 0;
  }
}

async function emit(email: string, type: string, object: Json, previous?: Json): Promise<Json> {
  const event: Json = {
    id: newId("evt"),
    object: "event",
    api_version: "2025-10-29.clover",
    created: eventTime(),
    livemode: false,
    pending_webhooks: 1,
    type,
    data: previous ? { object, previous_attributes: previous } : { object },
  };
  const payload = JSON.stringify(event);
  const header = signature(payload, WEBHOOK_SECRET);
  const status = await deliver(payload, header);
  events.push({ id: event.id, type, email, payload, header, status });
  return { id: event.id, type, status };
}

async function announceCheckout(email: string): Promise<Json[]> {
  const session = [...sessions.values()]
    .filter((s) => s.email === email && s.status === "complete" && !s.announced)
    .pop();
  if (!session) throw new Error(`no paid Checkout session for ${email}`);
  session.announced = true;
  const sub = subscriptions.get(session.subscription as string)!;
  return [
    await emit(email, "checkout.session.completed", publicSession(session)),
    await emit(email, "customer.subscription.created", sub),
    await emit(email, "invoice.paid", invoice(sub, "paid", "subscription_create")),
  ];
}

async function failRenewal(email: string, sub: Json): Promise<Json[]> {
  sub.status = "past_due";
  return [
    await emit(email, "invoice.payment_failed", invoice(sub, "open", "subscription_cycle")),
    await emit(email, "customer.subscription.updated", sub, { status: "active" }),
  ];
}

async function cancelAtPeriodEnd(email: string, sub: Json): Promise<Json[]> {
  const previous = { cancel_at_period_end: false, cancel_at: null, canceled_at: null };
  sub.cancel_at_period_end = true;
  sub.cancel_at = sub.current_period_end;
  sub.canceled_at = now();
  return [await emit(email, "customer.subscription.updated", sub, previous)];
}

async function end(email: string, sub: Json): Promise<Json[]> {
  sub.status = "canceled";
  sub.ended_at = now();
  sub.canceled_at ??= sub.ended_at;
  return [await emit(email, "customer.subscription.deleted", sub)];
}

function publicSession(session: Json): Json {
  const rest = { ...session };
  for (const key of ["email", "announced", "price", "subscription_metadata"]) delete rest[key];
  return rest;
}

function pay(session: Json) {
  let customerId = session.customer as string | null;
  if (!customerId) {
    customerId = createCustomer(session.email as string).id as string;
    session.customer = customerId;
  }
  const start = now();
  const periodEnd = oneMonthAtNoon();
  const item = {
    id: newId("si"),
    object: "subscription_item",
    price: priceWithProduct(session.price as string, false),
    quantity: 1,
    current_period_start: start,
    current_period_end: periodEnd,
  };
  const sub: Json = {
    id: newId("sub"),
    object: "subscription",
    customer: customerId,
    status: "active",
    cancel_at_period_end: false,
    cancel_at: null,
    canceled_at: null,
    ended_at: null,
    created: start,
    start_date: start,
    current_period_start: start,
    current_period_end: periodEnd,
    metadata: session.subscription_metadata,
    items: { object: "list", data: [item] },
  };
  subscriptions.set(sub.id as string, sub);
  session.status = "complete";
  session.payment_status = "paid";
  session.subscription = sub.id;
}

async function stripeApi(
  req: IncomingMessage,
  res: ServerResponse,
  path: string,
  query: URLSearchParams,
  fields: Fields,
) {
  const method = req.method ?? "GET";
  const expand = [...query.getAll("expand[]"), ...Object.values(prefixed(fields, "expand"))];
  const parts = path.split("/").filter(Boolean).slice(1);

  if (method === "GET" && path === "/v1/products") {
    const active = query.get("active");
    const data = PRODUCTS.filter((p) => active === null || String(p.active) === active);
    return send(res, 200, list(data, path));
  }
  if (method === "GET" && parts[0] === "products" && parts[1]) {
    const found = PRODUCTS.find((p) => p.id === parts[1]);
    return found ? send(res, 200, found) : stripeError(res, 404, "No such product");
  }
  if (method === "GET" && path === "/v1/prices") {
    const expandProduct = expand.includes("data.product");
    const data = PRICES.filter(
      (p) =>
        (!query.get("active") || String(p.active) === query.get("active")) &&
        (!query.get("product") || p.product === query.get("product")) &&
        (!query.get("type") || p.type === query.get("type")),
    ).map((p) => priceWithProduct(p.id as string, expandProduct)!);
    return send(res, 200, list(data, path));
  }
  if (method === "GET" && parts[0] === "prices" && parts[1]) {
    const found = priceWithProduct(parts[1], expand.includes("product"));
    return found ? send(res, 200, found) : stripeError(res, 404, "No such price");
  }
  if (parts[0] === "customers") {
    if (method === "POST" && !parts[1]) {
      if (!fields.email) return stripeError(res, 400, "Missing email");
      return send(res, 200, createCustomer(fields.email.toLowerCase(), fields));
    }
    if (method === "GET" && !parts[1]) {
      const email = query.get("email")?.toLowerCase();
      const data = [...customers.values()].filter((c) => !email || c.email === email);
      return send(res, 200, list(data, path));
    }
    const customer = customers.get(parts[1]);
    if (!customer) return stripeError(res, 404, "No such customer");
    if (method === "POST" && fields.email) customer.email = fields.email.toLowerCase();
    return send(res, 200, customer);
  }
  if (path === "/v1/checkout/sessions" && method === "POST") {
    const priceId = fields["line_items[0][price]"];
    if (fields.mode !== "subscription") return stripeError(res, 400, "mode must be subscription");
    if (Object.keys(fields).some((key) => key.startsWith("payment_method_types"))) {
      return stripeError(
        res,
        400,
        "payment_method_types is not allowed: Checkout must use automatic payment methods",
      );
    }
    if (!PRICES.some((p) => p.id === priceId)) return stripeError(res, 400, "No such price");
    if (!fields.success_url || !fields.cancel_url) {
      return stripeError(res, 400, "success_url and cancel_url are required");
    }
    if (fields.customer && !customers.has(fields.customer)) {
      return stripeError(res, 400, "No such customer");
    }
    const email = fields.customer ? emailOfCustomer(fields.customer) : fields.customer_email;
    if (!email) return stripeError(res, 400, "customer or customer_email is required");
    const id = newId("cs_test");
    const session: Json = {
      id,
      object: "checkout.session",
      url: `${PUBLIC_URL}/checkout/${id}`,
      mode: "subscription",
      status: "open",
      payment_status: "unpaid",
      customer: fields.customer ?? null,
      customer_email: fields.customer_email ?? null,
      client_reference_id: fields.client_reference_id ?? null,
      metadata: prefixed(fields, "metadata"),
      subscription: null,
      success_url: fields.success_url,
      cancel_url: fields.cancel_url,
      created: now(),
      email: email.toLowerCase(),
      price: priceId,
      subscription_metadata: prefixed(fields, "subscription_data[metadata]"),
    };
    sessions.set(id, session);
    return send(res, 200, publicSession(session));
  }
  if (parts[0] === "checkout" && parts[1] === "sessions" && parts[2] && method === "GET") {
    const session = sessions.get(parts[2]);
    return session ? send(res, 200, publicSession(session)) : stripeError(res, 404, "No such session");
  }
  if (path === "/v1/billing_portal/sessions" && method === "POST") {
    if (!customers.has(fields.customer)) return stripeError(res, 400, "No such customer");
    const id = newId("bps");
    const portal = {
      id,
      object: "billing_portal.session",
      customer: fields.customer,
      return_url: fields.return_url ?? null,
      url: `${PUBLIC_URL}/portal/${id}`,
      created: now(),
    };
    portals.set(id, portal);
    return send(res, 200, portal);
  }
  if (parts[0] === "subscriptions") {
    if (method === "GET" && !parts[1]) {
      const data = [...subscriptions.values()].filter(
        (s) =>
          (!query.get("customer") || s.customer === query.get("customer")) &&
          (!query.get("status") || query.get("status") === "all" || s.status === query.get("status")),
      );
      return send(res, 200, list(data, path));
    }
    const sub = subscriptions.get(parts[1]);
    if (!sub) return stripeError(res, 404, "No such subscription");
    if (method === "DELETE") {
      sub.status = "canceled";
      sub.ended_at = now();
      sub.canceled_at ??= sub.ended_at;
      return send(res, 200, sub);
    }
    if (method === "POST" && fields.cancel_at_period_end) {
      sub.cancel_at_period_end = fields.cancel_at_period_end === "true";
    }
    return send(res, 200, sub);
  }
  return stripeError(res, 404, `Unrecognized request URL (${method}: ${path})`);
}

async function hostedPage(req: IncomingMessage, res: ServerResponse, path: string) {
  const [, kind, id, action] = path.split("/");
  if (kind === "checkout") {
    const session = sessions.get(id);
    if (!session) return html(res, "Checkout not found", "");
    if (req.method === "POST" && action === "pay") {
      if (session.status === "open") pay(session);
      return redirect(res, (session.success_url as string).replace("{CHECKOUT_SESSION_ID}", id));
    }
    if (req.method === "POST" && action === "cancel") {
      return redirect(res, session.cancel_url as string);
    }
    const chosen = priceWithProduct(session.price as string, true)!;
    const name = (chosen.product as Json).name as string;
    const amount = `$${(chosen.unit_amount as number) / 100} / month`;
    return html(
      res,
      `Subscribe to ${name}`,
      `<p>${amount}</p><p>${session.email}</p>` +
        `<form method="post" action="/checkout/${id}/pay"><button>Pay</button></form>` +
        `<form method="post" action="/checkout/${id}/cancel"><button>Cancel</button></form>`,
    );
  }
  if (kind === "portal") {
    const portal = portals.get(id);
    if (!portal) return html(res, "Portal not found", "");
    const email = emailOfCustomer(portal.customer)!;
    const sub = latestSubscription(email);
    if (req.method === "POST" && action === "cancel") {
      if (sub && sub.status !== "canceled") await cancelAtPeriodEnd(email, sub);
      return redirect(res, (portal.return_url as string) ?? PUBLIC_URL);
    }
    return html(
      res,
      "Billing portal",
      `<p>${email}</p>` +
        `<form method="post" action="/portal/${id}/cancel"><button>Cancel subscription</button></form>` +
        `<a href="${portal.return_url}">Return</a>`,
    );
  }
  return html(res, "Not found", "");
}

async function control(res: ServerResponse, path: string, query: URLSearchParams, body: Json) {
  const email = String(body.email ?? query.get("email") ?? "").toLowerCase();
  if (path === "/_stub/health") return send(res, 200, { ok: true });
  if (path === "/_stub/account") {
    const customer = [...customers.values()].find((c) => c.email === email);
    return send(res, 200, { customer: customer?.id ?? null, subscription: latestSubscription(email) ?? null });
  }
  if (path === "/_stub/requests") {
    return send(res, 200, requests.filter((r) => r.email === email));
  }
  if (path === "/_stub/events") {
    return send(res, 200, events.filter((e) => e.email === email).map(({ id, type, status }) => ({ id, type, status })));
  }
  if (path === "/_stub/unreachable") {
    if (body.unreachable === false) unreachable.delete(email);
    else unreachable.add(email);
    return send(res, 200, { unreachable: unreachable.has(email) });
  }
  if (path === "/_stub/send") {
    try {
      if (body.event === "checkout_completed") return send(res, 200, await announceCheckout(email));
      const sub = latestSubscription(email);
      if (!sub) return send(res, 404, { error: `no subscription for ${email}` });
      if (body.event === "renewal_failed") return send(res, 200, await failRenewal(email, sub));
      if (body.event === "cancel_at_period_end") return send(res, 200, await cancelAtPeriodEnd(email, sub));
      if (body.event === "ended") return send(res, 200, await end(email, sub));
      return send(res, 400, { error: `unknown event ${body.event}` });
    } catch (error) {
      return send(res, 404, { error: String(error) });
    }
  }
  if (path === "/_stub/forge") {
    const sub = latestSubscription(email);
    if (!sub) return send(res, 404, { error: `no subscription for ${email}` });
    const event = {
      id: newId("evt"),
      object: "event",
      api_version: "2025-10-29.clover",
      created: eventTime(),
      livemode: false,
      type: "customer.subscription.deleted",
      data: { object: { ...sub, status: "canceled", ended_at: now() } },
    };
    const payload = JSON.stringify(event);
    const header = body.signature === "none" ? null : signature(payload, WRONG_SECRET);
    return send(res, 200, { id: event.id, status: await deliver(payload, header) });
  }
  if (path === "/_stub/replay") {
    const original = events.find((e) => e.id === body.id);
    if (!original) return send(res, 404, { error: `no event ${body.id}` });
    const status = await deliver(original.payload as string, original.header as string);
    return send(res, 200, { id: original.id, status });
  }
  return send(res, 404, { error: `unknown control ${path}` });
}

const server = createServer(async (req, res) => {
  const url = new URL(req.url ?? "/", PUBLIC_URL);
  const raw = await readBody(req);
  const contentType = req.headers["content-type"] ?? "";
  try {
    if (url.pathname.startsWith("/_stub/")) {
      return await control(res, url.pathname, url.searchParams, raw ? JSON.parse(raw) : {});
    }
    if (url.pathname.startsWith("/v1/")) {
      const fields = parseFields(raw, contentType);
      const email = emailFor(url.pathname, { ...Object.fromEntries(url.searchParams), ...fields });
      requests.push({ method: req.method, path: url.pathname, query: url.search, body: fields, email });
      if (email && unreachable.has(email)) {
        req.socket.destroy();
        return;
      }
      if (req.headers.authorization !== `Bearer ${SECRET_KEY}`) {
        return send(res, 401, { error: { type: "invalid_request_error", message: "Invalid API Key provided" } });
      }
      return await stripeApi(req, res, url.pathname, url.searchParams, fields);
    }
    return await hostedPage(req, res, url.pathname);
  } catch (error) {
    send(res, 500, { error: { type: "api_error", message: String(error) } });
  }
});

server.listen(PORT, "127.0.0.1");
