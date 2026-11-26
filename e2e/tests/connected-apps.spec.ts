import type { Page } from "@playwright/test";
import { createAccount } from "../support/flows";
import { type Loopback, startLoopback } from "../support/loopback";
import {
  authorizePath,
  exchangeCode,
  expectTokens,
  pkce,
  randomState,
  registerClient,
} from "../support/oauth";
import {
  addPasskeyToAccount,
  addVirtualAuthenticator,
  withoutPasskeyAutofill,
} from "../support/passkeys";
import { expect, test } from "../support/test";

let loopback: Loopback;

test.beforeEach(async () => {
  loopback = await startLoopback();
});

test.afterEach(async () => {
  await loopback.close();
});

async function openApproval(page: Page, baseURL: string, scope: string) {
  const clientId = await registerClient(page.request, "Approval Tester", [loopback.redirectUri]);
  const { verifier, challenge } = pkce();
  const resource = `${baseURL}/mcp`;
  await page.goto(
    authorizePath({
      clientId,
      redirectUri: loopback.redirectUri,
      scope,
      state: randomState(),
      challenge,
      resource,
    }),
  );
  await expect(page.getByRole("heading", { name: "Connect Approval Tester" })).toBeVisible();
  return async () => {
    const params = await loopback.nextCallback();
    const code = params.get("code");
    expect(code, `the client got a code, not ${params.get("error")}`).toBeTruthy();
    const response = await exchangeCode(page.request, {
      clientId,
      redirectUri: loopback.redirectUri,
      code: code ?? "",
      verifier,
      resource,
    });
    return (await expectTokens(response)).scope;
  };
}

function writeScope(page: Page) {
  return page.getByRole("checkbox", { name: /\(profile:write\)/ });
}

test("the password is asked for only while a ticked scope can write", async ({
  page,
  baseURL,
}, testInfo) => {
  await createAccount(page, testInfo, "Live Toggle");
  const connected = await openApproval(page, baseURL ?? "", "profile:read profile:write");
  const password = page.getByLabel("Current password");

  await expect(password).toBeVisible();
  await writeScope(page).uncheck();
  await expect(password).toBeHidden();
  await writeScope(page).check();
  await expect(password).toBeVisible();
  await writeScope(page).uncheck();
  await page.getByRole("button", { name: "Allow access" }).click();

  expect(await connected()).toBe("profile:read");
});

test.describe("without JavaScript", () => {
  test.use({ javaScriptEnabled: false });

  test("the password field is shown and a read-only grant connects without it", async ({
    page,
    baseURL,
  }, testInfo) => {
    await createAccount(page, testInfo, "No Script Approver");
    const connected = await openApproval(page, baseURL ?? "", "profile:read profile:write");

    await expect(page.getByLabel("Current password")).toBeVisible();
    await page
      .locator('form[method="post"]:has(input[name="scope"])')
      .evaluate((form: HTMLFormElement) => {
        const write = form.querySelector<HTMLInputElement>('input[value="profile:write"]');
        if (write) write.checked = false;
        form.submit();
      });

    expect(await connected()).toBe("profile:read");
  });
});

test.describe("with a passkey", () => {
  test.skip(
    ({ browserName }) => browserName !== "chromium",
    "virtual authenticators are Chromium-only",
  );

  test("a passkey confirms a write grant on the approval screen", async ({
    page,
    baseURL,
  }, testInfo) => {
    await withoutPasskeyAutofill(page);
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Passkey Approver");
    await addPasskeyToAccount(page);
    const connected = await openApproval(page, baseURL ?? "", "profile:write");

    await expect(page.getByText("We'll ask for your passkey when you continue.")).toBeVisible();
    await expect(page.getByLabel("Current password")).toBeHidden();
    await page.getByRole("button", { name: "Allow access" }).click();

    expect(await connected()).toBe("profile:write");
  });

  test("a read-only grant doesn't ask for the passkey", async ({ page, baseURL }, testInfo) => {
    await withoutPasskeyAutofill(page);
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Passkey Reader");
    await addPasskeyToAccount(page);
    const connected = await openApproval(page, baseURL ?? "", "profile:read profile:write");
    const prompts: string[] = [];
    page.on("request", (request) => {
      if (request.url().includes("/passkeys/verify/")) prompts.push(request.url());
    });

    await expect(page.getByText("We'll ask for your passkey when you continue.")).toBeVisible();
    await writeScope(page).uncheck();
    await expect(page.getByText("We'll ask for your passkey when you continue.")).toBeHidden();
    await page.getByRole("button", { name: "Allow access" }).click();

    expect(await connected()).toBe("profile:read");
    expect(prompts).toEqual([]);
  });
});
