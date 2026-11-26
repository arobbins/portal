import { randomBytes } from "node:crypto";
import type { APIRequestContext, APIResponse, Page, TestInfo } from "@playwright/test";
import { callMcp, createToken, getMe } from "../support/api";
import {
  ACCOUNT_PASSWORD,
  createAccount,
  logIn,
  logOut,
  setPassword,
  uniqueEmail,
} from "../support/flows";
import { startLoopback, type Loopback } from "../support/loopback";
import { resetLink, verificationLink, waitForEmailAbout, waitForEmails } from "../support/mail";
import {
  authorizePath,
  exchangeCode,
  expectOAuthError,
  expectTokens,
  pkce,
  randomState,
  refreshTokens,
  registerClient,
  revokeToken,
  type Tokens,
} from "../support/oauth";
import { expect, test } from "../support/test";

const DEVELOPER = "/account/developer";
const ACTIVITY = "/account/settings/devices";
const RETURN_HOST = "127.0.0.1";
const NO_APPS = "No apps connected.";
const WRONG_PASSWORD = "Your current password is incorrect.";
const PASSWORD_CHANGED = "Your password has been changed. Your other devices have been signed out.";

const SCOPE_LABELS: Record<string, string> = {
  "profile:read": "Read your profile (profile:read)",
  "profile:write": "Read and update your profile (profile:write)",
  "billing:read": "Read your billing (billing:read)",
  "billing:write": "Read your billing and start checkouts (billing:write)",
};

type Client = { name: string; clientId: string; loopback: Loopback };
type Authorization = { state: string; verifier: string };

let loopbacks: Loopback[] = [];

test.afterEach(async () => {
  await Promise.all(loopbacks.map((loopback) => loopback.close()));
  loopbacks = [];
});

function origin(testInfo: TestInfo): string {
  return String(testInfo.project.use.baseURL);
}

function mcpUrl(testInfo: TestInfo): string {
  return `${origin(testInfo)}/mcp`;
}

async function listen(): Promise<Loopback> {
  const loopback = await startLoopback();
  loopbacks.push(loopback);
  return loopback;
}

async function newClient(request: APIRequestContext): Promise<Client> {
  const loopback = await listen();
  const name = `Claude ${randomBytes(3).toString("hex")}`;
  const clientId = await registerClient(request, name, [loopback.redirectUri]);
  return { name, clientId, loopback };
}

function anonymousMcp(
  request: APIRequestContext,
  method: string,
  params: Record<string, unknown> = {},
) {
  return request.post("/mcp", { data: { jsonrpc: "2.0", id: 1, method, params } });
}

function callTool(request: APIRequestContext, token: string, name: string, args = {}) {
  return callMcp(request, token, "tools/call", { name, arguments: args });
}

async function toolNames(request: APIRequestContext, token: string): Promise<string[]> {
  const list = await callMcp(request, token, "tools/list");
  expect(list.status()).toBe(200);
  return (await list.json()).result.tools.map((tool: { name: string }) => tool.name);
}

async function expectToolWorks(response: APIResponse) {
  expect(response.status(), "the tool call is accepted").toBe(200);
  const body = await response.json();
  expect(body.error, "no JSON-RPC error").toBeUndefined();
  expect(body.result.isError, "the tool succeeded").toBe(false);
  return body.result.structuredContent;
}

async function expectInsufficientScope(response: APIResponse) {
  expect(response.status()).toBe(200);
  const result = (await response.json()).result;
  expect(result.isError).toBe(true);
  expect(JSON.parse(result.content[0].text).code).toBe("insufficient_scope");
}

async function expectTokenDead(request: APIRequestContext, token: string) {
  expect((await callTool(request, token, "get_profile")).status()).toBe(401);
}

async function expectRefreshRefused(
  request: APIRequestContext,
  client: Client,
  refreshToken: string,
  testInfo: TestInfo,
) {
  await expectOAuthError(
    await refreshTokens(request, {
      clientId: client.clientId,
      refreshToken,
      resource: mcpUrl(testInfo),
    }),
    "invalid_grant",
  );
}

async function expectDisconnected(
  request: APIRequestContext,
  client: Client,
  tokens: Tokens,
  testInfo: TestInfo,
) {
  await expectTokenDead(request, tokens.access_token);
  await expectRefreshRefused(request, client, tokens.refresh_token, testInfo);
}

function scopeBox(page: Page, scope: string) {
  return page.getByRole("checkbox", { name: SCOPE_LABELS[scope], exact: true });
}

function approvalHeading(page: Page, client: Client) {
  return page.getByRole("heading", { name: `Connect ${client.name}` });
}

function connectedApps(page: Page) {
  return page.locator("#connected-apps");
}

async function openApproval(
  page: Page,
  client: Client,
  scope: string,
  testInfo: TestInfo,
): Promise<Authorization> {
  const { verifier, challenge } = pkce();
  const state = randomState();
  await page.goto(
    authorizePath({
      clientId: client.clientId,
      redirectUri: client.loopback.redirectUri,
      scope,
      state,
      challenge,
      resource: mcpUrl(testInfo),
    }),
  );
  return { state, verifier };
}

async function receiveCode(client: Client, authorization: Authorization): Promise<string> {
  const params = await client.loopback.nextCallback();
  expect(params.get("state"), "the state comes back unchanged").toBe(authorization.state);
  expect(params.get("error")).toBeNull();
  const code = params.get("code");
  expect(code, "the redirect carries a code").toBeTruthy();
  return code!;
}

function exchange(
  request: APIRequestContext,
  client: Client,
  code: string,
  authorization: Authorization,
  testInfo: TestInfo,
) {
  return exchangeCode(request, {
    clientId: client.clientId,
    redirectUri: client.loopback.redirectUri,
    code,
    verifier: authorization.verifier,
    resource: mcpUrl(testInfo),
  });
}

async function exchangeForTokens(
  request: APIRequestContext,
  client: Client,
  authorization: Authorization,
  testInfo: TestInfo,
): Promise<Tokens> {
  const code = await receiveCode(client, authorization);
  const tokens = await expectTokens(await exchange(request, client, code, authorization, testInfo));
  expect(tokens.expires_in).toBe(3600);
  return tokens;
}

async function approveForCode(page: Page, client: Client, scope: string, testInfo: TestInfo) {
  const authorization = await openApproval(page, client, scope, testInfo);
  await expect(approvalHeading(page, client)).toBeVisible();
  await page.getByRole("button", { name: "Allow access" }).click();
  const code = await receiveCode(client, authorization);
  return { code, authorization };
}

async function connect(
  page: Page,
  request: APIRequestContext,
  client: Client,
  scope: string,
  testInfo: TestInfo,
): Promise<Tokens> {
  const authorization = await openApproval(page, client, scope, testInfo);
  await expect(approvalHeading(page, client)).toBeVisible();
  await page.getByRole("button", { name: "Allow access" }).click();
  return exchangeForTokens(request, client, authorization, testInfo);
}

async function openConnectedApps(page: Page) {
  await page.goto(DEVELOPER);
  await expect(page.getByRole("heading", { name: "Connected apps" })).toBeVisible();
  await expect(connectedApps(page)).toBeVisible();
}

async function expectNoApps(page: Page, client: Client) {
  await openConnectedApps(page);
  await expect(connectedApps(page)).toContainText(NO_APPS);
  await expect(connectedApps(page).getByText(client.name)).toHaveCount(0);
}

test("S1 without a token only public tools work and protected ones point to sign-in", async ({
  request,
}, testInfo) => {
  const initialize = await anonymousMcp(request, "initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
    clientInfo: { name: "e2e", version: "1" },
  });
  expect(initialize.status()).toBe(200);
  expect((await initialize.json()).result.protocolVersion).toBe("2025-06-18");

  const list = await anonymousMcp(request, "tools/list");
  expect(list.status()).toBe(200);
  const names = (await list.json()).result.tools.map((tool: { name: string }) => tool.name);
  expect(names).toEqual(["list_plans"]);

  const profile = await anonymousMcp(request, "tools/call", { name: "get_profile", arguments: {} });
  expect(profile.status()).toBe(401);
  const challenge = profile.headers()["www-authenticate"] ?? "";
  const metadataUrl = challenge.match(/^Bearer\b.*\bresource_metadata="([^"]+)"/)?.[1];
  expect(metadataUrl, `WWW-Authenticate names resource metadata: ${challenge}`).toBeTruthy();

  const resource = await request.get(metadataUrl!);
  expect(resource.status()).toBe(200);
  const resourceMetadata = await resource.json();
  expect(resourceMetadata.resource).toBe(mcpUrl(testInfo));
  const issuer: string = resourceMetadata.authorization_servers?.[0];
  expect(issuer, "the resource names its authorization server").toBeTruthy();
  const issuerUrl = new URL(issuer);
  expect(issuerUrl.origin).toBe(origin(testInfo));

  const server = await request.get(
    `${issuerUrl.origin}/.well-known/oauth-authorization-server${issuerUrl.pathname.replace(/\/$/, "")}`,
  );
  expect(server.status()).toBe(200);
  const serverMetadata = await server.json();
  expect(serverMetadata.issuer).toBe(issuer);
  expect(serverMetadata.registration_endpoint).toBe(`${origin(testInfo)}/oauth/register`);
  expect(serverMetadata.authorization_endpoint).toBe(`${origin(testInfo)}/oauth/authorize`);
  expect(serverMetadata.token_endpoint).toBe(`${origin(testInfo)}/oauth/token`);
  expect(serverMetadata.revocation_endpoint).toBe(`${origin(testInfo)}/oauth/revoke`);
  expect(serverMetadata.code_challenge_methods_supported).toEqual(["S256"]);
});

test("S2 approving a registered client with one scope unticked connects it with the rest", async ({
  page,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Assistant Owner");
  const client = await newClient(request);

  const authorization = await openApproval(page, client, "profile:read billing:read", testInfo);

  await expect(approvalHeading(page, client)).toBeVisible();
  await expect(page.getByText("Unverified name")).toBeVisible();
  await expect(page.getByText(`Returns to ${RETURN_HOST}`)).toBeVisible();
  await expect(page.getByText(new RegExp(`Returns to ${RETURN_HOST.replaceAll(".", "\\.")}:`))).toHaveCount(0);
  await expect(scopeBox(page, "profile:read")).toBeChecked();
  await expect(scopeBox(page, "billing:read")).toBeChecked();
  await expect(page.getByRole("checkbox")).toHaveCount(2);
  await expect(page.getByLabel("Current password")).toHaveCount(0);

  await scopeBox(page, "billing:read").uncheck();
  await page.getByRole("button", { name: "Allow access" }).click();

  const tokens = await exchangeForTokens(request, client, authorization, testInfo);
  expect(tokens.scope).toBe("profile:read");

  const profile = await expectToolWorks(await callTool(request, tokens.access_token, "get_profile"));
  expect(profile).toMatchObject({ name: "Assistant Owner", email });
  const names = await toolNames(request, tokens.access_token);
  expect(names).toContain("get_profile");
  expect(names).not.toContain("get_billing");
  await expectInsufficientScope(await callTool(request, tokens.access_token, "get_billing"));
});

test("S3 a write scope asks for the password before connecting", async ({
  page,
  request,
}, testInfo) => {
  await createAccount(page, testInfo, "Billing Delegator");
  const client = await newClient(request);
  const authorization = await openApproval(page, client, "billing:write", testInfo);
  await expect(approvalHeading(page, client)).toBeVisible();
  await expect(scopeBox(page, "billing:write")).toBeChecked();

  const password = page.getByLabel("Current password");
  await expect(password).toBeVisible();
  await password.fill("not-the-password");
  await password.press("Enter");

  await expect(page.getByText(WRONG_PASSWORD)).toBeVisible();
  expect(client.loopback.received, "a wrong password sends nothing to the client").toEqual([]);

  await password.fill(ACCOUNT_PASSWORD);
  await password.press("Enter");

  const tokens = await exchangeForTokens(request, client, authorization, testInfo);
  expect(tokens.scope).toBe("billing:write");
  await expectToolWorks(await callTool(request, tokens.access_token, "get_billing"));
  expect(client.loopback.received).toHaveLength(1);
});

test("S4 denying sends access_denied and connects nothing", async ({ page, request }, testInfo) => {
  await createAccount(page, testInfo, "Cautious Owner");
  const client = await newClient(request);
  const authorization = await openApproval(page, client, "profile:read", testInfo);
  await expect(approvalHeading(page, client)).toBeVisible();

  await page.getByRole("button", { name: "Deny" }).click();

  const params = await client.loopback.nextCallback();
  expect(params.get("error")).toBe("access_denied");
  expect(params.get("state")).toBe(authorization.state);
  expect(params.get("code")).toBeNull();
  await expectNoApps(page, client);
});

test("S5 refreshing rotates tokens and reusing an old refresh token disconnects the app", async ({
  page,
  request,
}, testInfo) => {
  await createAccount(page, testInfo, "Refresh Owner");
  const client = await newClient(request);
  const first = await connect(page, request, client, "profile:read", testInfo);

  const refreshed = await expectTokens(
    await refreshTokens(request, {
      clientId: client.clientId,
      refreshToken: first.refresh_token,
      resource: mcpUrl(testInfo),
    }),
  );
  expect(refreshed.access_token).not.toBe(first.access_token);
  expect(refreshed.refresh_token).not.toBe(first.refresh_token);
  await expectToolWorks(await callTool(request, refreshed.access_token, "get_profile"));

  await expectRefreshRefused(request, client, first.refresh_token, testInfo);

  await expectDisconnected(request, client, refreshed, testInfo);
});

test("S6 disconnecting an app ends its tokens and shows in recent activity", async ({
  page,
  request,
}, testInfo) => {
  await createAccount(page, testInfo, "Disconnect Owner");
  const client = await newClient(request);
  const tokens = await connect(page, request, client, "profile:read", testInfo);
  await expectToolWorks(await callTool(request, tokens.access_token, "get_profile"));

  await openConnectedApps(page);
  const card = connectedApps(page);
  await expect(card).toContainText(client.name);
  await expect(card).toContainText(RETURN_HOST);
  await expect(card).toContainText("Read your profile");
  await card.getByRole("button", { name: "Disconnect app" }).click();
  const dialog = page.getByRole("dialog", { name: "Disconnect app" });
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "Disconnect app" }).click();

  await expect(card).toContainText(NO_APPS);
  await expect(card.getByText(client.name)).toHaveCount(0);
  await expectDisconnected(request, client, tokens, testInfo);

  await page.goto(ACTIVITY);
  const entries = page.locator("#activity").getByRole("listitem");
  const connected = `Connected ${client.name} (${RETURN_HOST})`;
  const disconnected = `Disconnected ${client.name} (${RETURN_HOST})`;
  const startsWith = (phrase: string) =>
    new RegExp(`^\\s*${phrase.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`);
  await expect(entries.filter({ hasText: startsWith(connected) })).toHaveCount(1);
  await expect(entries.filter({ hasText: startsWith(disconnected) })).toHaveCount(1);
  const texts = await entries.allTextContents();
  const connectedAt = texts.findIndex((text) => text.includes(connected) && !text.includes(disconnected));
  const disconnectedAt = texts.findIndex((text) => text.includes(disconnected));
  expect(disconnectedAt, "newest first: Disconnected is listed above Connected").toBeLessThan(
    connectedAt,
  );
});

test("S7 a new visitor registers from the approval screen and comes back to it", async ({
  browser,
  request,
}, testInfo) => {
  const client = await newClient(request);
  const first = await (await browser.newContext()).newPage();
  const authorization = await openApproval(first, client, "profile:read", testInfo);

  await expect(first).toHaveURL(/\/login(\?|$)/);
  await first.getByRole("main").getByRole("link", { name: "Register" }).click();
  const email = uniqueEmail(testInfo, "assistant-newcomer");
  await first.getByLabel("Name").fill("Assistant Newcomer");
  await first.getByLabel("Email").fill(email);
  await first.getByLabel("Email").press("Enter");

  const second = await (await browser.newContext()).newPage();
  await second.goto(verificationLink((await waitForEmails(email))[0]));
  await setPassword(second, ACCOUNT_PASSWORD);

  await expect(approvalHeading(second, client)).toBeVisible();
  await expect(second).toHaveURL(/\/oauth\/authorize/);
  await second.getByRole("button", { name: "Allow access" }).click();
  const tokens = await exchangeForTokens(request, client, authorization, testInfo);
  const profile = await expectToolWorks(await callTool(request, tokens.access_token, "get_profile"));
  expect(profile).toMatchObject({ name: "Assistant Newcomer", email });

  await first.context().close();
  await second.context().close();
});

test("S8 connecting an app emails the account with the scopes and a link", async ({
  page,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Emailed Owner");
  const client = await newClient(request);
  await connect(page, request, client, "profile:read", testInfo);

  const mail = await waitForEmailAbout(email, `${client.name} was connected to your account`);
  expect(mail.text).toContain("Read your profile");
  expect(mail.text).toContain(`${origin(testInfo)}${DEVELOPER}#connected-apps`);
});

test("S9 a password change keeps apps; signing out everywhere or a reset disconnects them", async ({
  page,
  browser,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Reset Owner");
  const client = await newClient(request);
  const kept = await connect(page, request, client, "profile:read", testInfo);

  await page.goto("/account/settings/authentication");
  await page.getByRole("button", { name: "Change password" }).click();
  const change = page.getByRole("dialog", { name: "Change password" });
  await change.getByLabel("Current password").fill(ACCOUNT_PASSWORD);
  await change.getByLabel("New password", { exact: true }).fill("a-changed-password");
  await change.getByLabel("Confirm new password").fill("a-changed-password");
  await change.getByLabel("Confirm new password").press("Enter");
  await expect(page.getByText(PASSWORD_CHANGED)).toHaveCSS("opacity", "1");
  await expectToolWorks(await callTool(request, kept.access_token, "get_profile"));

  const elsewhere = await (await browser.newContext()).newPage();
  await logIn(elsewhere, email, "a-changed-password");
  await expect(elsewhere).toHaveURL("/account");
  await page.goto(ACTIVITY);
  await page.getByRole("button", { name: "Sign out other devices" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Sign out other devices" }).click();
  await expect(
    page.getByText("Your other devices have been signed out.", { exact: true }),
  ).toBeVisible();
  await elsewhere.context().close();

  await expectDisconnected(request, client, kept, testInfo);
  await expectNoApps(page, client);

  const again = await connect(page, request, client, "profile:read", testInfo);
  await expectToolWorks(await callTool(request, again.access_token, "get_profile"));
  await page.goto("/account");
  await logOut(page, "Reset Owner");
  await page.goto("/reset-password");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Email").press("Enter");
  let link = "";
  await expect
    .poll(async () => {
      const resets = (await waitForEmails(email)).filter((mail) =>
        mail.text.includes("/reset-password/"),
      );
      link = resets.length ? resetLink(resets[resets.length - 1]) : "";
      return link;
    })
    .not.toBe("");
  await page.goto(link);
  await setPassword(page, "a-brand-new-password");
  await expect(page).toHaveURL("/account");

  await expectDisconnected(request, client, again, testInfo);
  await expectNoApps(page, client);
});

test("S10 a sign-in token is refused on the JSON API while hand-made MCP tokens still work", async ({
  page,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Audience Owner");
  const client = await newClient(request);
  const tokens = await connect(page, request, client, "profile:read", testInfo);
  await expectToolWorks(await callTool(request, tokens.access_token, "get_profile"));

  expect((await getMe(request, `Bearer ${tokens.access_token}`)).status()).toBe(401);

  const handMade = await createToken(
    page,
    "Assistant",
    "Read your profile (profile:read)",
    "AI assistants (MCP)",
  );
  const profile = await expectToolWorks(await callTool(request, handMade, "get_profile"));
  expect(profile).toMatchObject({ email });
});

test("S11 a signed-out person with an account signs in and lands on the approval screen", async ({
  page,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Returning Owner");
  await logOut(page, "Returning Owner");
  const client = await newClient(request);

  const authorization = await openApproval(page, client, "profile:read", testInfo);

  await expect(page).toHaveURL(/\/login(\?|$)/);
  await expect(page.getByRole("main").getByRole("link", { name: "Register" })).toBeVisible();
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(ACCOUNT_PASSWORD);
  await page.getByLabel("Password").press("Enter");

  await expect(approvalHeading(page, client)).toBeVisible();
  await expect(page).toHaveURL(/\/oauth\/authorize/);
  await page.getByRole("button", { name: "Allow access" }).click();
  const tokens = await exchangeForTokens(request, client, authorization, testInfo);
  const profile = await expectToolWorks(await callTool(request, tokens.access_token, "get_profile"));
  expect(profile).toMatchObject({ name: "Returning Owner", email });
});

test.describe("with an unregistered redirect", () => {
  test.use({ allowedConsoleErrors: [/status of 400/] });

  test("S12 a wrong redirect URI never redirects and bad code exchanges get invalid_grant", async ({
    page,
    request,
  }, testInfo) => {
    await createAccount(page, testInfo, "Strict Owner");
    const client = await newClient(request);
    const stranger = await listen();

    await openApproval(page, client, "profile:read", testInfo);
    await expect(approvalHeading(page, client)).toBeVisible();

    const response = await page.goto(
      authorizePath({
        clientId: client.clientId,
        redirectUri: stranger.redirectUri,
        scope: "profile:read",
        state: randomState(),
        challenge: pkce().challenge,
        resource: mcpUrl(testInfo),
      }),
    );
    expect(response?.status()).toBe(400);
    await expect(page.getByRole("heading", { name: "Can't connect this app" })).toBeVisible();
    await expect(
      page.getByText("This app asked to return to an address it didn't register."),
    ).toBeVisible();
    expect(new URL(page.url()).origin).toBe(origin(testInfo));
    expect(new URL(page.url()).pathname).toBe("/oauth/authorize");
    await expect(page.getByRole("button", { name: "Allow access" })).toHaveCount(0);
    expect(stranger.received).toEqual([]);
    expect(client.loopback.received).toEqual([]);

    const first = await approveForCode(page, client, "profile:read", testInfo);
    await expectOAuthError(
      await exchange(
        request,
        client,
        first.code,
        { ...first.authorization, verifier: pkce().verifier },
        testInfo,
      ),
      "invalid_grant",
    );

    const second = await approveForCode(page, client, "profile:read", testInfo);
    await expectTokens(await exchange(request, client, second.code, second.authorization, testInfo));
    await expectOAuthError(
      await exchange(request, client, second.code, second.authorization, testInfo),
      "invalid_grant",
    );
  });
});

test("S13 connecting twice keeps one connection with the newer scopes; revoking ends it", async ({
  page,
  request,
}, testInfo) => {
  await createAccount(page, testInfo, "Repeat Owner");
  const client = await newClient(request);
  await connect(page, request, client, "profile:read", testInfo);
  const latest = await connect(page, request, client, "profile:read billing:read", testInfo);

  await openConnectedApps(page);
  const card = connectedApps(page);
  await expect(card).toContainText(client.name);
  await expect(card.getByRole("button", { name: "Disconnect app" })).toHaveCount(1);
  await expect(card).toContainText("Read your billing");

  const revoked = await revokeToken(request, {
    clientId: client.clientId,
    token: latest.refresh_token,
    tokenTypeHint: "refresh_token",
  });
  expect(revoked.status()).toBe(200);

  await expectDisconnected(request, client, latest, testInfo);
  await expectNoApps(page, client);
});
