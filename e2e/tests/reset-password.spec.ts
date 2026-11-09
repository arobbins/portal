import type { Page, TestInfo } from "@playwright/test";
import {
  ACCOUNT_PASSWORD,
  createAccount,
  logIn,
  logOut,
  register,
  setPassword,
  uniqueEmail,
} from "../support/flows";
import { expectNoEmails, resetLink, verificationLink, waitForEmails } from "../support/mail";
import { expect, test } from "../support/test";

const NEW_PASSWORD = "a-brand-new-password";
const LINK_SENT = "If an account exists for that email, we've sent a link to reset your password.";

test.use({ allowedConsoleErrors: [/status of 400/] });

async function requestReset(page: Page, email: string) {
  await page.goto("/login");
  await page.getByRole("link", { name: "Forgot password?" }).click();
  await page.getByLabel("Email").fill(email);
  await page.getByRole("button", { name: "Send reset link" }).click();
  await expect(page).toHaveURL("/reset-password");
  await expect(page.getByText(LINK_SENT)).toHaveCSS("opacity", "1");
}

async function signedOutAccount(page: Page, testInfo: TestInfo, name: string) {
  const email = await createAccount(page, testInfo, name);
  await logOut(page, name);
  return email;
}

async function emailedResetLink(email: string) {
  const emails = await waitForEmails(email, 2);
  return resetLink(emails[emails.length - 1]);
}

test("a forgotten password can be reset from an emailed link", async ({ page }, testInfo) => {
  const email = await signedOutAccount(page, testInfo, "Reset Tester");

  await requestReset(page, email);
  await page.goto(await emailedResetLink(email));
  await expect(page.getByRole("heading", { name: "Choose a new password" })).toBeVisible();
  await setPassword(page, NEW_PASSWORD);

  await expect(page).toHaveURL("/account");
  await expect(page.getByText("Your password has been reset.")).toHaveCSS("opacity", "1");

  await logOut(page, "Reset Tester");
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page.getByText("Invalid email or password.")).toBeVisible();
  await logIn(page, email, NEW_PASSWORD);
  await expect(page).toHaveURL("/account");
});

test("an unknown email gets the same answer and no email", async ({ page }) => {
  const email = "nobody-at-all@example.test";

  await requestReset(page, email);

  await expectNoEmails(email);
});

test("an account that never set a password gets its sign-up link again", async ({
  page,
}, testInfo) => {
  const email = uniqueEmail(testInfo, "half-done");
  await register(page, "Half Done", email);
  await waitForEmails(email);

  await requestReset(page, email);

  const emails = await waitForEmails(email, 2);
  expect(emails[1].subject).toBe("Confirm your email");
  await page.goto(verificationLink(emails[1]));
  await expect(page.getByRole("heading", { name: "Set your password" })).toBeVisible();
});

test("opening the link doesn't use it up, but resetting does", async ({ page }, testInfo) => {
  const email = await signedOutAccount(page, testInfo, "Once Tester");
  await requestReset(page, email);
  const link = await emailedResetLink(email);

  await page.goto(link);
  await page.goto(link);
  await setPassword(page, NEW_PASSWORD);
  await expect(page).toHaveURL("/account");

  const response = await page.goto(link);
  expect(response?.status()).toBe(400);
  await expect(page.getByRole("heading", { name: "This link has expired" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Request a new link" })).toHaveAttribute(
    "href",
    "/reset-password",
  );
});

test("a rejected password keeps the link working", async ({ page }, testInfo) => {
  const email = await signedOutAccount(page, testInfo, "Mismatch Tester");
  await requestReset(page, email);
  await page.goto(await emailedResetLink(email));

  await setPassword(page, NEW_PASSWORD, "something-else-entirely");
  await expect(page.getByText("Passwords do not match.")).toBeVisible();

  await setPassword(page, NEW_PASSWORD);
  await expect(page).toHaveURL("/account");
});

test("a newer reset link replaces the older one", async ({ page }, testInfo) => {
  const email = await signedOutAccount(page, testInfo, "Twice Tester");
  await requestReset(page, email);
  const firstLink = await emailedResetLink(email);
  await requestReset(page, email);
  const emails = await waitForEmails(email, 3);
  const secondLink = resetLink(emails[emails.length - 1]);

  expect((await page.goto(firstLink))?.status()).toBe(400);
  await page.goto(secondLink);
  await expect(page.getByRole("heading", { name: "Choose a new password" })).toBeVisible();
});

test("resetting signs out every other device", async ({ page, browser }, testInfo) => {
  const email = await createAccount(page, testInfo, "Device Tester");
  const otherDevice = await browser.newPage();

  await requestReset(otherDevice, email);
  await otherDevice.goto(await emailedResetLink(email));
  await setPassword(otherDevice, NEW_PASSWORD);
  await expect(otherDevice).toHaveURL("/account");

  await page.goto("/account");
  await expect(page).toHaveURL("/login");
  await otherDevice.close();
});

test("signed-in users are sent from the reset page to their account", async ({
  page,
}, testInfo) => {
  await createAccount(page, testInfo, "Signed In Tester");

  await page.goto("/reset-password");

  await expect(page).toHaveURL("/account");
});
