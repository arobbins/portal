import type { Page, TestInfo } from "@playwright/test";
import { ACCOUNT_PASSWORD, createAccount, logIn, logOut, uniqueEmail } from "../support/flows";
import { confirmEmailLink, expectNoEmails, waitForEmails } from "../support/mail";
import { addPasskeyToAccount, addVirtualAuthenticator } from "../support/passkeys";
import { expect, test } from "../support/test";

const SENT = "Check your new inbox for a link to confirm the change. Your email stays the same until you do.";
const CHANGED = "Your email has been changed.";

function dialog(page: Page) {
  return page.getByRole("dialog", { name: "Change email" });
}

async function requestChange(page: Page, newEmail: string, password = ACCOUNT_PASSWORD) {
  await page.goto("/account/profile");
  await page.getByRole("button", { name: "Change email" }).click();
  await expect(dialog(page)).toBeVisible();
  await dialog(page).getByLabel("New email").fill(newEmail);
  await dialog(page).getByLabel("Current password").fill(password);
  await dialog(page).getByLabel("Current password").press("Enter");
}

function newAddress(testInfo: TestInfo) {
  return uniqueEmail(testInfo, "moved");
}

test("a user changes their email after confirming the new address", async ({
  page,
}, testInfo) => {
  const oldEmail = await createAccount(page, testInfo, "Mover");
  const newEmail = newAddress(testInfo);

  await page.goto("/account/profile");
  await page.getByRole("button", { name: "Change email" }).click();
  await expect(dialog(page).getByLabel("Current email")).toBeDisabled();
  await expect(dialog(page).getByLabel("Current email")).toHaveValue(oldEmail);
  await dialog(page).getByRole("button", { name: "Close" }).click();

  await requestChange(page, newEmail);

  await expect(page.getByText(SENT)).toHaveCSS("opacity", "1");
  await expect(dialog(page)).toBeHidden();
  await expect(page.locator("#email")).toContainText(oldEmail);

  const [email] = await waitForEmails(newEmail);
  const link = confirmEmailLink(email);
  await page.goto(link);
  await page.goto(link);
  await expect(page.getByRole("heading", { name: "Confirm your new email" })).toBeVisible();
  await expect(page.getByText(newEmail)).toBeVisible();
  await page.getByRole("button", { name: "Confirm email" }).press("Enter");

  await expect(page).toHaveURL("/account/profile");
  await expect(page.getByText(CHANGED)).toHaveCSS("opacity", "1");
  await expect(page.locator("#email")).toContainText(newEmail);
  const notices = await waitForEmails(oldEmail, 2);
  expect(notices.map((sent) => sent.subject)).toContain("Your email was changed");

  const spent = await page.request.get(link);
  expect(spent.status()).toBe(400);
  expect(await spent.text()).toContain("This link has expired");

  await logOut(page, "Mover");
  await logIn(page, oldEmail, ACCOUNT_PASSWORD);
  await expect(page.getByText("Invalid email or password.")).toBeVisible();
  await logIn(page, newEmail, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/account");
});

test("a wrong current password is shown on that field", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Wrong Password");
  const newEmail = newAddress(testInfo);

  await requestChange(page, newEmail, "not-my-password");

  await expect(dialog(page).getByRole("alert")).toHaveText("Your current password is incorrect.");
  await expect(dialog(page).getByLabel("Current password")).toHaveAttribute("aria-invalid", "true");
  await expect(dialog(page).getByLabel("New email")).toHaveAttribute("aria-invalid", "false");
  await expectNoEmails(newEmail);
});

test("the same address is rejected on the email field", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Same");

  await requestChange(page, email);

  await expect(dialog(page).getByRole("alert")).toHaveText("That's already your email address.");
  await expect(dialog(page).getByLabel("New email")).toHaveAttribute("aria-invalid", "true");
});

test("an address that has an account looks the same but only gets a notice", async ({
  browser,
  page,
}, testInfo) => {
  await createAccount(page, testInfo, "Taker");
  const other = await browser.newPage();
  const taken = await createAccount(other, testInfo, "Owner");
  await other.close();
  const before = (await waitForEmails(taken)).length;

  await requestChange(page, taken);

  await expect(page.getByText(SENT)).toHaveCSS("opacity", "1");
  const emails = await waitForEmails(taken, before + 1);
  const latest = emails[emails.length - 1];
  expect(latest.subject).toBe("This email already has an account");
  expect(latest.text).not.toContain("/confirm-email/");
});

test("confirming a new email signs out every other device", async ({ browser, page }, testInfo) => {
  const oldEmail = await createAccount(page, testInfo, "Devices");
  const newEmail = newAddress(testInfo);
  const other = await browser.newPage();
  await logIn(other, oldEmail, ACCOUNT_PASSWORD);
  await expect(other).toHaveURL("/account");

  await requestChange(page, newEmail);
  await expect(page.getByText(SENT)).toHaveCSS("opacity", "1");
  await page.goto(confirmEmailLink((await waitForEmails(newEmail))[0]));
  await page.getByRole("button", { name: "Confirm email" }).press("Enter");

  await expect(page).toHaveURL("/account/profile");
  await expect(page.locator("#email")).toContainText(newEmail);
  await other.goto("/account/profile");
  await expect(other).toHaveURL("/login");
  await other.close();
});

test("without a passkey, only the password confirms the change", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "No Passkey");
  await page.goto("/account/profile");
  await page.getByRole("button", { name: "Change email" }).click();

  await expect(dialog(page).getByLabel("Current password")).toBeVisible();
  await expect(dialog(page).getByRole("button", { name: "Use a passkey instead" })).toHaveCount(0);
});

test.describe("with a passkey", () => {
  test.skip(
    ({ browserName }) => browserName !== "chromium",
    "virtual authenticators are Chromium-only",
  );

  test("a passkey confirms the change instead of the password", async ({ page }, testInfo) => {
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Passkey Mover");
    await addPasskeyToAccount(page);
    const newEmail = newAddress(testInfo);

    await page.goto("/account/profile");
    await page.getByRole("button", { name: "Change email" }).click();
    await dialog(page).getByLabel("New email").fill(newEmail);
    await dialog(page).getByRole("button", { name: "Use a passkey instead" }).click();

    await expect(page.getByText(SENT)).toHaveCSS("opacity", "1");
    await expect(dialog(page)).toBeHidden();
    await waitForEmails(newEmail);

    await page.getByRole("button", { name: "Change email" }).click();
    await expect(dialog(page).getByLabel("Current password")).toBeEnabled();
    await dialog(page)
      .getByLabel("Current password")
      .evaluate((input) => input.removeAttribute("required"));
    await dialog(page).getByLabel("New email").fill(newAddress(testInfo));
    await dialog(page).getByLabel("New email").press("Enter");
    await expect(dialog(page).getByRole("alert")).toHaveText(
      "Enter your current password or confirm with a passkey.",
    );
  });
});
