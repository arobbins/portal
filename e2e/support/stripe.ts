import { e2ePort } from "./environment";

export const STRIPE_SECRET_KEY = "sk_test_e2e_stub";
export const STRIPE_WEBHOOK_SECRET = "whsec_e2e_stub_signing_secret";

export type StripeEvent = "checkout_completed" | "renewal_failed" | "cancel_at_period_end" | "ended";

export type Delivery = { id: string; type: string; status: number };

export type StubSubscription = {
  id: string;
  status: string;
  current_period_end: number;
  cancel_at: number | null;
};

export type StubRequest = { method: string; path: string; body: Record<string, string> };

export function stripeStubPort(): number {
  return e2ePort() + 1000;
}

export function stripeStubUrl(): string {
  return `http://localhost:${stripeStubPort()}`;
}

export function stripeApiBase(): string {
  return `http://127.0.0.1:${stripeStubPort()}`;
}

async function control<T>(path: string, body?: Record<string, unknown>): Promise<T> {
  const response = await fetch(`${stripeApiBase()}/_stub/${path}`, {
    method: body ? "POST" : "GET",
    headers: { "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!response.ok) throw new Error(`Stripe stub ${path}: ${await response.text()}`);
  return response.json() as Promise<T>;
}

export const stripe = {
  account(email: string) {
    return control<{ customer: string | null; subscription: StubSubscription | null }>(
      `account?email=${encodeURIComponent(email)}`,
    );
  },
  requests(email: string) {
    return control<StubRequest[]>(`requests?email=${encodeURIComponent(email)}`);
  },
  events(email: string) {
    return control<Delivery[]>(`events?email=${encodeURIComponent(email)}`);
  },
  send(email: string, event: StripeEvent) {
    return control<Delivery[]>("send", { email, event });
  },
  forge(email: string, signature: "wrong" | "none") {
    return control<Delivery>("forge", { email, signature });
  },
  replay(id: string) {
    return control<Delivery>("replay", { id });
  },
  unreachable(email: string, unreachable = true) {
    return control<{ unreachable: boolean }>("unreachable", { email, unreachable });
  },
};
