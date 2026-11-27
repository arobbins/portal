import { randomBytes } from "node:crypto";
import { expect, type APIRequestContext, type Page } from "@playwright/test";
import { startLoopback } from "./loopback";
import {
  authorizePath,
  exchangeCode,
  expectTokens,
  pkce,
  randomState,
  registerClient,
  type Tokens,
} from "./oauth";

export type ConnectedApp = { name: string; returnHost: string; scope: string; tokens: Tokens };

export async function connectApp(
  page: Page,
  request: APIRequestContext,
  options: { scope: string; name?: string },
): Promise<ConnectedApp> {
  const name = options.name ?? `App ${randomBytes(3).toString("hex")}`;
  const resource = new URL("/mcp", page.url()).href;
  const loopback = await startLoopback();
  try {
    const clientId = await registerClient(request, name, [loopback.redirectUri]);
    const { verifier, challenge } = pkce();
    const state = randomState();
    await page.goto(
      authorizePath({
        clientId,
        redirectUri: loopback.redirectUri,
        scope: options.scope,
        state,
        challenge,
        resource,
      }),
    );
    await expect(page.getByRole("heading", { name: `Connect ${name}` })).toBeVisible();
    await page.getByRole("button", { name: "Allow access" }).click();
    const params = await loopback.nextCallback();
    expect(params.get("state"), "the state comes back unchanged").toBe(state);
    const code = params.get("code");
    expect(code, "the redirect carries a code").toBeTruthy();
    const tokens = await expectTokens(
      await exchangeCode(request, {
        clientId,
        redirectUri: loopback.redirectUri,
        code: code!,
        verifier,
        resource,
      }),
    );
    return { name, returnHost: new URL(loopback.redirectUri).hostname, scope: options.scope, tokens };
  } finally {
    await loopback.close();
  }
}
