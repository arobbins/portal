import type { Page } from "@playwright/test";
import { ACCOUNT_PASSWORD, createAccountAtPasskeyPrompt, logIn, logOut } from "../support/flows";
import { addVirtualAuthenticator, withoutPasskeyAutofill } from "../support/passkeys";
import { expect, test } from "../support/test";

function prompt(page: Page) {
  return page.getByRole("heading", { name: "Sign in faster next time" });
}

test("setting a first password offers a passkey, and Not now skips it", async ({
  page,
}, testInfo) => {
  const email = await createAccountAtPasskeyPrompt(page, testInfo, "Prompt Tester");

  await expect(page).toHaveURL("/passkey-setup");
  await expect(prompt(page)).toBeVisible();
  await expect(page.getByText("Your email is confirmed. Welcome!")).toHaveCSS("opacity", "1");

  await page.getByRole("button", { name: "Not now" }).click();
  await expect(page).toHaveURL("/account");

  await logOut(page, "Prompt Tester");
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/account");
});

test("a password login offers a passkey until the user answers", async ({ page }, testInfo) => {
  const email = await createAccountAtPasskeyPrompt(page, testInfo, "Undecided Tester");
  await expect(page).toHaveURL("/passkey-setup");
  await page.goto("/account");

  await logOut(page, "Undecided Tester");
  await logIn(page, email, ACCOUNT_PASSWORD);

  await expect(page).toHaveURL("/passkey-setup");
  await expect(prompt(page)).toBeVisible();
});

test("Don't ask again stops the prompt", async ({ page }, testInfo) => {
  const email = await createAccountAtPasskeyPrompt(page, testInfo, "Decliner");

  await page.getByRole("button", { name: "Don't ask again" }).click();
  await expect(page).toHaveURL("/account");

  await logOut(page, "Decliner");
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/account");
});

test("the prompt page sends users to their account once it isn't due", async ({
  page,
}, testInfo) => {
  await createAccountAtPasskeyPrompt(page, testInfo, "Revisitor");
  await page.getByRole("button", { name: "Not now" }).click();
  await expect(page).toHaveURL("/account");

  await page.goto("/passkey-setup");

  await expect(page).toHaveURL("/account");
});

test("adding a passkey without a password only works from the prompt", async ({
  page,
}, testInfo) => {
  await createAccountAtPasskeyPrompt(page, testInfo, "Grant Tester");
  await page.getByRole("button", { name: "Not now" }).click();
  await expect(page).toHaveURL("/account");

  const response = await page.request.post("/passkeys/register/start", { data: {} });

  expect(response.status()).toBe(400);
  expect((await response.json()).error).toBe(
    "This page expired. Add a passkey from your Security settings.",
  );
});

test("signed-out visitors are sent to log in", async ({ page }) => {
  await page.goto("/passkey-setup");

  await expect(page).toHaveURL("/login");
});

test.describe("with a passkey-capable device", () => {
  test.skip(
    ({ browserName }) => browserName !== "chromium",
    "virtual authenticators are Chromium-only",
  );

  test("adding a passkey from the prompt needs no password and stops the prompt", async ({
    page,
  }, testInfo) => {
    await withoutPasskeyAutofill(page);
    await addVirtualAuthenticator(page);
    const email = await createAccountAtPasskeyPrompt(page, testInfo, "Prompt Adder");

    await page.getByRole("button", { name: "Add a passkey" }).click();

    await expect(page).toHaveURL("/account/security#passkeys");
    await expect(page.getByText("Passkey added. You can now use it to sign in.")).toHaveCSS(
      "opacity",
      "1",
    );
    await expect(page.locator("#passkeys").getByRole("listitem")).toHaveCount(1);

    await logOut(page, "Prompt Adder");
    await logIn(page, email, ACCOUNT_PASSWORD);
    await expect(page).toHaveURL("/account");
  });
});
