import type { Browser } from "@playwright/test";
import { ACCOUNT_PASSWORD, createAccount, logIn, logOut } from "../support/flows";
import { emailsAbout, waitForEmailAbout } from "../support/mail";
import { expect, test } from "../support/test";

const NEW_SIGN_IN = "New sign-in to your account";

async function signInElsewhere(browser: Browser, email: string) {
  const page = await (await browser.newContext()).newPage();
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/account");
  return page;
}

test("signing in from a new browser sends an alert, but the same browser doesn't", async ({
  page,
  browser,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Alert Receiver");
  expect(await emailsAbout(email, NEW_SIGN_IN)).toEqual([]);

  const other = await signInElsewhere(browser, email);
  const alert = await waitForEmailAbout(email, NEW_SIGN_IN);
  expect(alert.text).toContain("If this was you, there's nothing to do.");
  expect(alert.text).toContain("/account/settings/notifications#notifications");

  await logOut(other, "Alert Receiver");
  await logIn(other, email, ACCOUNT_PASSWORD);
  await expect(other).toHaveURL("/account");
  await new Promise((resolve) => setTimeout(resolve, 1_000));
  expect(await emailsAbout(email, NEW_SIGN_IN)).toHaveLength(1);
  await other.context().close();
});

test("turning sign-in alerts off saves right away and stops them", async ({
  page,
  browser,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Quiet Person");
  await page.goto("/account/settings/notifications");
  const alerts = page.getByRole("switch", { name: "New sign-in alerts" });
  await expect(alerts).toBeChecked();

  await alerts.click();
  await expect(page.getByText("Notification settings saved.")).toBeVisible();
  await expect(alerts).not.toBeChecked();
  await page.reload();
  await expect(page.getByRole("switch", { name: "New sign-in alerts" })).not.toBeChecked();

  const other = await signInElsewhere(browser, email);
  await new Promise((resolve) => setTimeout(resolve, 1_000));
  expect(await emailsAbout(email, NEW_SIGN_IN)).toEqual([]);
  await other.context().close();
});

test("sign-in alerts can be turned off without JavaScript", async ({ browser }, testInfo) => {
  const page = await (await browser.newContext({ javaScriptEnabled: false })).newPage();
  await createAccount(page, testInfo, "Plain Person");
  await page.goto("/account/settings/notifications");

  await page.getByRole("switch", { name: "New sign-in alerts" }).uncheck();
  await page.getByRole("button", { name: "Save notification settings" }).click();

  await expect(page).toHaveURL(/\/account\/settings\/notifications/);
  await expect(page.getByRole("switch", { name: "New sign-in alerts" })).not.toBeChecked();
  await page.context().close();
});

test("changing the password is always emailed", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Password Changer");
  await page.goto("/account/security/authentication");
  await page.getByRole("button", { name: "Change password" }).click();
  const dialog = page.getByRole("dialog", { name: "Change password" });
  await dialog.getByLabel("Current password").fill(ACCOUNT_PASSWORD);
  await dialog.getByLabel("New password", { exact: true }).fill("another-strong-password");
  await dialog.getByLabel("Confirm new password").fill("another-strong-password");
  await dialog.getByLabel("Confirm new password").press("Enter");

  const notice = await waitForEmailAbout(email, "Your password was changed");
  expect(notice.text).toContain("/reset-password");
});
