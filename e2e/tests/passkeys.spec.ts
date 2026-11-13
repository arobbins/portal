import type { Page } from "@playwright/test";
import { ACCOUNT_PASSWORD, createAccount, logOut } from "../support/flows";
import { waitForEmails } from "../support/mail";
import { addVirtualAuthenticator, withoutPasskeyAutofill } from "../support/passkeys";
import { expect, test } from "../support/test";

test.use({ allowedConsoleErrors: [/status of 40[01]/] });

function addDialog(page: Page) {
  return page.getByRole("dialog", { name: "Add a passkey" });
}

async function addPasskey(page: Page, password = ACCOUNT_PASSWORD) {
  await page.goto("/account/security");
  await page.getByRole("button", { name: "Add passkey" }).click();
  await addDialog(page).getByLabel("Current password").fill(password);
  await addDialog(page).getByLabel("Current password").press("Enter");
}

test.describe("with a passkey-capable device", () => {
  test.skip(
    ({ browserName }) => browserName !== "chromium",
    "virtual authenticators are Chromium-only",
  );

  test("a user can add a passkey and is told by email", async ({ page }, testInfo) => {
    await addVirtualAuthenticator(page);
    const email = await createAccount(page, testInfo, "Passkey Adder");

    await addPasskey(page);

    await expect(page.getByText("Passkey added. You can now use it to sign in.")).toHaveCSS(
      "opacity",
      "1",
    );
    const passkeys = page.locator("#passkeys");
    await expect(passkeys.getByText("Passkey", { exact: true })).toBeVisible();
    await expect(passkeys.getByText("This device only")).toBeVisible();
    await expect(passkeys.getByText(/Add a second passkey/)).toBeVisible();
    const emails = await waitForEmails(email, 2);
    expect(
      emails.some((message) => message.subject === "A passkey was added to your account"),
    ).toBe(true);
  });

  test("adding a passkey needs the current password", async ({ page }, testInfo) => {
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Wrong Password");

    await addPasskey(page, "not-my-password");

    await expect(addDialog(page).getByRole("alert")).toHaveText(
      "Your current password is incorrect.",
    );
    await expect(addDialog(page).getByLabel("Current password")).toHaveAttribute(
      "aria-invalid",
      "true",
    );
    await page.goto("/account/security");
    await expect(page.locator("#passkeys").getByRole("listitem")).toHaveCount(0);
  });

  test("choosing the passkey from the email field's autofill signs in", async ({
    page,
  }, testInfo) => {
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Autofill Signer");
    await addPasskey(page);
    await expect(page.locator("#passkeys").getByRole("listitem")).toHaveCount(1);

    await page.getByRole("button", { name: "Autofill Signer" }).click();
    await page.getByRole("button", { name: "Logout" }).click();

    await expect(page).toHaveURL("/account");
    await expect(page.getByText("Name: Autofill Signer")).toBeVisible();
  });

  test("the passkey button signs in instead of a password", async ({ page }, testInfo) => {
    await withoutPasskeyAutofill(page);
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Passkey Signer");
    await addPasskey(page);
    await expect(page.locator("#passkeys").getByRole("listitem")).toHaveCount(1);
    await logOut(page, "Passkey Signer");

    await page.getByRole("button", { name: "Sign in with a passkey" }).click();

    await expect(page).toHaveURL("/account");
    await page.goto("/account/security");
    await expect(page.locator("#passkeys").getByText(/last used/)).toBeVisible();
  });

  test("a passkey can be renamed and removed", async ({ page }, testInfo) => {
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Passkey Manager");
    await addPasskey(page);
    const passkeys = page.locator("#passkeys");
    await expect(passkeys.getByRole("listitem")).toHaveCount(1);

    await passkeys.getByRole("button", { name: "Rename Passkey" }).click();
    const rename = page.getByRole("dialog", { name: "Rename passkey" });
    await rename.getByLabel("Name").fill("Work laptop");
    await rename.getByLabel("Name").press("Enter");
    await expect(page.getByText("Passkey renamed.")).toHaveCSS("opacity", "1");
    await expect(passkeys.getByText("Work laptop")).toBeVisible();

    await passkeys.getByRole("button", { name: "Remove Work laptop" }).click();
    const remove = page.getByRole("dialog", { name: "Remove passkey" });
    await expect(remove.getByText("This is your only passkey.")).toBeVisible();
    await remove.getByRole("button", { name: "Remove passkey" }).click();
    await expect(page.getByText("Passkey removed.")).toHaveCSS("opacity", "1");
    await expect(passkeys.getByRole("listitem")).toHaveCount(0);
  });

  test("a removed passkey can no longer sign in", async ({ page }, testInfo) => {
    await withoutPasskeyAutofill(page);
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Removed Passkey");
    await addPasskey(page);
    const passkeys = page.locator("#passkeys");
    await passkeys.getByRole("button", { name: "Remove Passkey" }).click();
    await page
      .getByRole("dialog", { name: "Remove passkey" })
      .getByRole("button", { name: "Remove passkey" })
      .click();
    await expect(page.getByText("Passkey removed.")).toHaveCSS("opacity", "1");
    await logOut(page, "Removed Passkey");

    await page.getByRole("button", { name: "Sign in with a passkey" }).click();

    await expect(page.getByRole("alert")).toHaveText(
      "That passkey isn't registered with this site anymore. Sign in another way.",
    );
    await expect(page).toHaveURL("/login");
  });
});

test("the login page offers passkey sign-in and autofill", async ({ page }) => {
  await page.goto("/login");

  await expect(page.getByRole("button", { name: "Sign in with a passkey" })).toBeVisible();
  await expect(page.getByLabel("Email")).toHaveAttribute("autocomplete", "username webauthn");
});

test("passkey requests can't skip the start step or be replayed", async ({ page }) => {
  await page.goto("/about");
  const response = await page.request.post("/passkeys/login/finish", {
    data: {
      id: "AAAA",
      rawId: "AAAA",
      type: "public-key",
      response: { clientDataJSON: "AAAA", authenticatorData: "AAAA", signature: "AAAA" },
      extensions: {},
    },
    headers: { Origin: new URL(page.url()).origin },
  });

  expect(response.status()).toBe(400);
  expect((await response.json()).error).toBe("That passkey request expired. Please try again.");
});

test("signed-out visitors can't add a passkey", async ({ page }) => {
  await page.goto("/login");
  const response = await page.request.post("/passkeys/register/start", {
    data: { current_password: "anything" },
    headers: { Origin: new URL(page.url()).origin },
  });

  expect(response.status()).toBe(401);
});
