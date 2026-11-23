import { expect, type Page } from "@playwright/test";
import { ACCOUNT_PASSWORD } from "./flows";

export async function addVirtualAuthenticator(page: Page) {
  const client = await page.context().newCDPSession(page);
  await client.send("WebAuthn.enable");
  const { authenticatorId } = await client.send("WebAuthn.addVirtualAuthenticator", {
    options: {
      protocol: "ctap2",
      transport: "internal",
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  return authenticatorId;
}

export async function withoutPasskeyAutofill(page: Page) {
  await page.addInitScript(() => {
    PublicKeyCredential.isConditionalMediationAvailable = async () => false;
  });
}

export async function addPasskeyToAccount(page: Page) {
  await page.goto("/account/settings/authentication");
  await page.getByRole("button", { name: "Add passkey" }).click();
  const add = page.getByRole("dialog", { name: "Add passkey" });
  await add.getByLabel("Current password").fill(ACCOUNT_PASSWORD);
  await add.getByLabel("Current password").press("Enter");
  await expect(page.getByText("Passkey added. You can now use it to sign in.")).toHaveCSS(
    "opacity",
    "1",
  );
}
