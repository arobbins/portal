import { createHash, randomBytes } from "node:crypto";
import { expect, type APIRequestContext, type APIResponse } from "@playwright/test";

export type Pkce = { verifier: string; challenge: string };

export type Tokens = {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in?: number;
  scope?: string;
};

export function pkce(): Pkce {
  const verifier = randomBytes(32).toString("base64url");
  const challenge = createHash("sha256").update(verifier).digest("base64url");
  return { verifier, challenge };
}

export function randomState(): string {
  return randomBytes(16).toString("base64url");
}

export async function registerClient(
  request: APIRequestContext,
  clientName: string,
  redirectUris: string[],
): Promise<string> {
  const response = await request.post("/oauth/register", {
    data: {
      client_name: clientName,
      redirect_uris: redirectUris,
      grant_types: ["authorization_code", "refresh_token"],
      response_types: ["code"],
      token_endpoint_auth_method: "none",
    },
  });
  expect(response.status(), "dynamic client registration succeeds").toBe(201);
  const body = await response.json();
  expect(typeof body.client_id, "registration returns a client_id").toBe("string");
  return body.client_id;
}

export function authorizePath(params: {
  clientId: string;
  redirectUri: string;
  scope: string;
  state: string;
  challenge: string;
  resource: string;
}): string {
  const query = new URLSearchParams({
    response_type: "code",
    client_id: params.clientId,
    redirect_uri: params.redirectUri,
    scope: params.scope,
    state: params.state,
    code_challenge: params.challenge,
    code_challenge_method: "S256",
    resource: params.resource,
  });
  return `/oauth/authorize?${query}`;
}

export function exchangeCode(
  request: APIRequestContext,
  params: { clientId: string; redirectUri: string; code: string; verifier: string; resource: string },
): Promise<APIResponse> {
  return request.post("/oauth/token", {
    form: {
      grant_type: "authorization_code",
      code: params.code,
      redirect_uri: params.redirectUri,
      client_id: params.clientId,
      code_verifier: params.verifier,
      resource: params.resource,
    },
  });
}

export function refreshTokens(
  request: APIRequestContext,
  params: { clientId: string; refreshToken: string; resource: string },
): Promise<APIResponse> {
  return request.post("/oauth/token", {
    form: {
      grant_type: "refresh_token",
      refresh_token: params.refreshToken,
      client_id: params.clientId,
      resource: params.resource,
    },
  });
}

export async function expectTokens(response: APIResponse): Promise<Tokens> {
  expect(response.status(), `token endpoint answered ${await response.text()}`).toBe(200);
  const body = (await response.json()) as Tokens;
  expect(typeof body.access_token).toBe("string");
  expect(typeof body.refresh_token).toBe("string");
  expect(body.token_type.toLowerCase()).toBe("bearer");
  return body;
}

export async function expectOAuthError(response: APIResponse, error: string) {
  expect(response.status(), "the token endpoint refuses the request").toBe(400);
  expect((await response.json()).error).toBe(error);
}

export function revokeToken(
  request: APIRequestContext,
  params: { clientId: string; token: string; tokenTypeHint?: "access_token" | "refresh_token" },
): Promise<APIResponse> {
  return request.post("/oauth/revoke", {
    form: {
      token: params.token,
      client_id: params.clientId,
      ...(params.tokenTypeHint ? { token_type_hint: params.tokenTypeHint } : {}),
    },
  });
}
