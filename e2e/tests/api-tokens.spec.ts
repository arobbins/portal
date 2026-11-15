import {
  ACCOUNT_PASSWORD,
  createAccount,
  logOut,
  setPassword,
} from "../support/flows";
import {
  TOKEN_SHAPE,
  createToken,
  createTokenDialog as createDialog,
  getMe as callMe,
  startCreatingToken,
} from "../support/api";
import { resetLink, waitForEmails } from "../support/mail";
import { addPasskeyToAccount, addVirtualAuthenticator } from "../support/passkeys";
import { expect, test } from "../support/test";

test("a token can be created once, used with the API, and is announced by email", async ({
  page,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Token Maker");

  const token = await createToken(page);
  const response = await callMe(request, `Bearer ${token}`);

  expect(response.status()).toBe(200);
  expect(response.headers()["cache-control"]).toBe("no-store");
  expect(await response.json()).toEqual({ name: "Token Maker", email });
  const emails = await waitForEmails(email, 2);
  expect(emails[emails.length - 1].subject).toBe("A new API token was created");

  await createDialog(page).getByRole("button", { name: "Done" }).click();
  const tokens = page.locator("#api-tokens");
  await expect(tokens.getByText("Deploy script")).toBeVisible();
  await expect(tokens.getByText("profile:read", { exact: true })).toBeVisible();
  await expect(tokens.getByText(/last used/)).toBeVisible();
  await expect(tokens.getByText(token)).toHaveCount(0);
});

test("requests without a valid token are refused", async ({ request }) => {
  const missing = await callMe(request);
  expect(missing.status()).toBe(401);
  expect(missing.headers()["www-authenticate"]).toBe('Bearer realm="api"');

  for (const authorization of [
    "Bearer wsk_not-a-real-token",
    `Bearer wsk_${"a".repeat(49)}`,
    "Basic dXNlcjpwYXNz",
  ]) {
    const response = await callMe(request, authorization);
    expect(response.status()).toBe(401);
    expect(response.headers()["www-authenticate"]).toContain('error="invalid_token"');
    expect(response.headers()["content-type"]).toBe("application/problem+json");
    expect((await response.json()).code).toBe("invalid_token");
  }
});

test("a wrong password creates no token", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Token Guesser");

  await startCreatingToken(page, "Sneaky", "not-my-password");

  await expect(createDialog(page).getByText("Your current password is incorrect.")).toBeVisible();
  await expect(createDialog(page).getByLabel("Current password")).toHaveAttribute(
    "aria-invalid",
    "true",
  );
  await page.goto("/account/developer");
  await expect(page.getByText("No API tokens yet")).toBeVisible();
});

test("a revoked token stops working right away", async ({ page, request }, testInfo) => {
  await createAccount(page, testInfo, "Token Revoker");
  const token = await createToken(page);
  await createDialog(page).getByRole("button", { name: "Done" }).click();

  await page.getByRole("button", { name: "Revoke Deploy script" }).click();
  await page
    .getByRole("dialog", { name: "Revoke API token" })
    .getByRole("button", { name: "Revoke token" })
    .click();

  await expect(page.getByText("API token revoked. Apps using it can no longer sign in.")).toHaveCSS(
    "opacity",
    "1",
  );
  await expect(page.getByText("No API tokens yet")).toBeVisible();
  expect((await callMe(request, `Bearer ${token}`)).status()).toBe(401);
});

test("resetting the password revokes every token", async ({ page, request }, testInfo) => {
  const email = await createAccount(page, testInfo, "Token Resetter");
  const token = await createToken(page);
  await createDialog(page).getByRole("button", { name: "Done" }).click();
  await logOut(page, "Token Resetter");

  await page.goto("/reset-password");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Email").press("Enter");
  const emails = await waitForEmails(email, 3);
  await page.goto(resetLink(emails[emails.length - 1]));
  await setPassword(page, "a-brand-new-password");
  await expect(page).toHaveURL("/account");

  expect((await callMe(request, `Bearer ${token}`)).status()).toBe(401);
});

test("without JavaScript the new token is shown on the page", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Plain Form");
  await page.goto("/account/developer");

  const response = await page.request.post("/account/api-tokens/create", {
    form: {
      name: "Plain script",
      access: "profile:write",
      lifetime: "30",
      current_password: ACCOUNT_PASSWORD,
    },
    headers: { Origin: new URL(page.url()).origin },
  });

  expect(response.status()).toBe(200);
  expect(response.headers()["cache-control"]).toBe("no-store");
  const html = await response.text();
  expect(html).toContain("Your new API token");
  expect(html).toMatch(/value="wsk_[A-Za-z0-9]{49}"/);
});

test("the create form only accepts a scope and expiry from its lists", async ({
  page,
}, testInfo) => {
  await createAccount(page, testInfo, "Scope Forger");
  await page.goto("/account/developer");

  const response = await page.request.post("/account/api-tokens/create", {
    form: {
      name: "Forged",
      access: "admin:*",
      lifetime: "36500",
      current_password: ACCOUNT_PASSWORD,
    },
    headers: { Origin: new URL(page.url()).origin },
  });

  expect(response.status()).toBe(400);
  expect(await response.text()).not.toMatch(/value="wsk_/);
});

test.describe("when the server can't be reached", () => {
  test.use({
    allowedConsoleErrors: [/Failed to load resource|NetworkError|network connection/i],
  });

  test("the create button recovers and explains what happened", async ({ page }, testInfo) => {
    await createAccount(page, testInfo, "Offline Token");
    await page.route("**/_topcoat/runtime/procedures/**", (route) => route.abort());

    await startCreatingToken(page, "Offline");

    await expect(
      createDialog(page).getByText("We couldn't create the token. Please try again."),
    ).toBeVisible();
    await expect(createDialog(page).getByRole("button", { name: "Create API token" })).toBeEnabled();
  });
});

test("a token can be revoked without JavaScript", async ({
  page,
  browser,
  browserName,
  baseURL,
  request,
}, testInfo) => {
  test.skip(browserName !== "chromium", "Dialog commands without JavaScript are checked in Chromium");
  await createAccount(page, testInfo, "Plain Revoker");
  const token = await createToken(page);

  const plain = await browser.newContext({
    javaScriptEnabled: false,
    baseURL,
    storageState: await page.context().storageState(),
  });
  const plainPage = await plain.newPage();
  await plainPage.goto("/account/developer");
  await plainPage.getByRole("button", { name: "Revoke Deploy script" }).click();
  await plainPage
    .getByRole("dialog", { name: "Revoke API token" })
    .getByRole("button", { name: "Revoke token" })
    .click();

  await expect(plainPage.getByText("No API tokens yet")).toBeVisible();
  expect((await callMe(request, `Bearer ${token}`)).status()).toBe(401);
  await plain.close();
});

test.describe("with a passkey", () => {
  test.skip(
    ({ browserName }) => browserName !== "chromium",
    "virtual authenticators are Chromium-only",
  );

  test("a passkey can confirm creating a token", async ({ page }, testInfo) => {
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Passkey Token");
    await addPasskeyToAccount(page);

    await page.goto("/account/developer");
    await page.getByRole("button", { name: "Create API token" }).click();
    await createDialog(page).getByLabel("Name").fill("Passkey script");
    await createDialog(page).getByLabel("Name").press("Enter");

    await expect(createDialog(page).getByLabel("API token")).toHaveValue(TOKEN_SHAPE);
  });
});
