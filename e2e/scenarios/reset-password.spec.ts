import type { Page } from "@playwright/test";
import { bearer, createToken, getMe } from "../support/api";
import { ACCOUNT_PASSWORD, createAccount, logIn, logOut, setPassword } from "../support/flows";
import { emailsAbout, resetLink } from "../support/mail";
import { totp, turnOnTwoFactor } from "../support/totp";
import { expect, test } from "../support/test";

const NEW_PASSWORD = "a-brand-new-password";
const THIRTY_SECONDS = 30_000;
const RESET_SUBJECT = "Reset your password";
const RESET_DONE = "Your password has been reset.";
const CODE_HINT =
  "Open your authenticator app and enter the 6-digit code for this account. No phone? Enter one of your recovery codes.";
const USED_UP =
  "That code didn't work, so this link has been used up. Request a new link to try again.";
const INVALID_LINK =
  "This reset link is invalid or has already been used. Links work once and expire after 1 hour.";

test.use({ allowedConsoleErrors: [/status of 400/] });

async function requestResetLink(page: Page, email: string): Promise<string> {
  const before = (await emailsAbout(email, RESET_SUBJECT)).length;
  await page.goto("/reset-password");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Email").press("Enter");
  await expect(page.getByText("Check your inbox")).toBeVisible();
  await expect
    .poll(async () => (await emailsAbout(email, RESET_SUBJECT)).length, {
      message: `waiting for a new "${RESET_SUBJECT}" to ${email}`,
      timeout: 10_000,
    })
    .toBeGreaterThan(before);
  const emails = await emailsAbout(email, RESET_SUBJECT);
  return resetLink(emails[emails.length - 1]);
}

function codeField(page: Page) {
  return page.getByLabel("Code", { exact: true });
}

async function resetWithCode(page: Page, link: string, code: string) {
  await page.goto(link);
  await expect(page.getByRole("heading", { name: "Choose a new password" })).toBeVisible();
  await page.getByLabel("New password", { exact: true }).fill(NEW_PASSWORD);
  await page.getByLabel("Confirm new password").fill(NEW_PASSWORD);
  await expect(codeField(page)).toBeVisible();
  await codeField(page).fill(code);
  await codeField(page).press("Enter");
}

function currentCode(secret: string) {
  return totp(secret, Date.now() + THIRTY_SECONDS);
}

async function expectInvalidLink(page: Page, link: string) {
  const response = await page.goto(link);
  expect(response?.status()).toBe(400);
  await expect(page.getByText(INVALID_LINK)).toBeVisible();
  await expect(page.getByRole("link", { name: "Request a new link" })).toHaveAttribute(
    "href",
    "/reset-password",
  );
}

async function changePasswordSignedIn(page: Page, next: string) {
  await page.goto("/account/settings/authentication");
  await page.getByRole("button", { name: "Change password" }).click();
  const dialog = page.getByRole("dialog", { name: "Change password" });
  await dialog.getByLabel("Current password").fill(ACCOUNT_PASSWORD);
  await dialog.getByLabel("New password", { exact: true }).fill(next);
  await dialog.getByLabel("Confirm new password").fill(next);
  await dialog.getByLabel("Confirm new password").press("Enter");
  await expect(page.getByText("Your password has been changed.")).toBeVisible();
}

test("S1 two-factor reset with an authenticator code signs in", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Reset Coder");
  const { secret } = await turnOnTwoFactor(page);
  await logOut(page, "Reset Coder");
  const link = await requestResetLink(page, email);

  await page.goto(link);
  await expect(codeField(page)).toBeVisible();
  await expect(page.getByText(CODE_HINT)).toBeVisible();
  await resetWithCode(page, link, currentCode(secret));

  await expect(page).toHaveURL("/account");
  await expect(page.getByText(RESET_DONE)).toBeVisible();
  await logOut(page, "Reset Coder");
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page.getByText("Invalid email or password.")).toBeVisible();
});

test("S2 a wrong code uses the link up and changes nothing else", async ({
  page,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Wrong Coder");
  const token = await createToken(page);
  const { secret } = await turnOnTwoFactor(page);
  await logOut(page, "Wrong Coder");
  const link = await requestResetLink(page, email);

  await resetWithCode(page, link, "000000");

  await expect(page.getByText(USED_UP)).toBeVisible();
  await expect(page.getByRole("link", { name: "Request a new link" })).toBeVisible();
  await expectInvalidLink(page, link);
  expect((await getMe(request, bearer(token).Authorization)).status()).toBe(200);
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/login/two-factor");

  await page.context().clearCookies();
  const newLink = await requestResetLink(page, email);
  await resetWithCode(page, newLink, currentCode(secret));
  await expect(page).toHaveURL("/account");
  await expect(page.getByText(RESET_DONE)).toBeVisible();
});

test("S3 two-factor reset with a recovery code uses that code up", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Recovery Coder");
  const { codes } = await turnOnTwoFactor(page);
  await logOut(page, "Recovery Coder");
  const link = await requestResetLink(page, email);

  await resetWithCode(page, link, codes[0]);

  await expect(page).toHaveURL("/account");
  await expect(page.getByText(RESET_DONE)).toBeVisible();
  await logOut(page, "Recovery Coder");
  await logIn(page, email, NEW_PASSWORD);
  await expect(page).toHaveURL("/login/two-factor");
  await page.getByLabel("Code").fill(codes[0]);
  await page.getByLabel("Code").press("Enter");
  await expect(page.getByText("That code didn't work.")).toBeVisible();
});

test("S4 without two-factor the reset form has no code and signs straight in", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Plain Resetter");
  await logOut(page, "Plain Resetter");
  const link = await requestResetLink(page, email);

  await page.goto(link);
  await expect(page.getByRole("heading", { name: "Choose a new password" })).toBeVisible();
  await expect(codeField(page)).toHaveCount(0);
  await setPassword(page, NEW_PASSWORD);

  await expect(page).toHaveURL("/account");
  await expect(page.getByText(RESET_DONE)).toBeVisible();
});

test("S5 changing the password while signed in cancels the reset link", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Changer Resetter");
  await logOut(page, "Changer Resetter");
  const link = await requestResetLink(page, email);

  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/account");
  await changePasswordSignedIn(page, NEW_PASSWORD);
  await logOut(page, "Changer Resetter");

  await expectInvalidLink(page, link);
});
