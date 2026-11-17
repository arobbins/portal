import type { Page } from "@playwright/test";
import {
  ACCOUNT_PASSWORD,
  createAccount,
  logIn,
  logOut,
  register,
  uniqueEmail,
} from "../support/flows";
import { grantAdmin } from "../support/console";
import { waitForEmails } from "../support/mail";
import {
  addPasskeyToAccount,
  addVirtualAuthenticator,
  withoutPasskeyAutofill,
} from "../support/passkeys";
import { adminUser } from "../support/users";
import { expect, test } from "../support/test";

test.use({ allowedConsoleErrors: [/status of 40[34]/] });

async function openAdmin(page: Page) {
  await logIn(page, adminUser.email, adminUser.password);
  await expect(page).toHaveURL("/account");
  await page.getByRole("button", { name: adminUser.name }).click();
  await page.getByRole("link", { name: "Admin" }).click();
  await expect(page).toHaveURL("/admin/confirm");
  await page.getByLabel("Current password").fill(adminUser.password);
  await page.getByLabel("Current password").press("Enter");
  await expect(page).toHaveURL("/admin/users");
}

async function openUser(page: Page, email: string) {
  await page.getByLabel("Search by email or name").fill(email);
  await page.getByLabel("Search by email or name").press("Enter");
  const row = page.getByRole("row", { name: new RegExp(email) });
  await expect(row).toHaveCount(1);
  await row.getByRole("link").click();
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
}

async function runAction(page: Page, label: string) {
  await page.getByRole("button", { name: label }).first().click();
  await page.getByRole("dialog", { name: label }).getByRole("button", { name: label }).click();
}

test("subscribers never see the admin area", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Curious User");

  const response = await page.goto("/admin/users");
  expect(response!.status()).toBe(404);
  await page.goto("/account");
  await page.getByRole("button", { name: "Curious User" }).click();
  await expect(page.getByRole("link", { name: "Admin" })).toHaveCount(0);
});

test("an admin disables an account, which signs it out until it's enabled", async ({
  page,
  browser,
}, testInfo) => {
  const userPage = await (await browser.newContext()).newPage();
  const email = await createAccount(userPage, testInfo, "Suspended User");
  await openAdmin(page);

  await openUser(page, email);
  await runAction(page, "Disable account");
  await expect(page.getByText("Account disabled. They've been signed out everywhere.")).toBeVisible();
  await expect(page.locator("main").getByText("Disabled", { exact: true })).toBeVisible();

  await userPage.goto("/account");
  await expect(userPage).toHaveURL("/login");
  await logIn(userPage, email, ACCOUNT_PASSWORD);
  await expect(userPage.getByText("This account has been disabled.")).toBeVisible();

  await runAction(page, "Enable account");
  await expect(page.getByText("Account enabled. They can sign in again.")).toBeVisible();
  await logIn(userPage, email, ACCOUNT_PASSWORD);
  await expect(userPage).toHaveURL("/account");
  await expect(page.getByText("Account disabled").first()).toBeVisible();
  await userPage.context().close();
});

test("an admin signs a user out everywhere and it shows in their activity", async ({
  page,
  browser,
}, testInfo) => {
  const userPage = await (await browser.newContext()).newPage();
  const email = await createAccount(userPage, testInfo, "Roaming User");
  await openAdmin(page);

  await openUser(page, email);
  await runAction(page, "Sign out everywhere");

  await expect(page.getByText("They've been signed out of every device.")).toBeVisible();
  await userPage.goto("/account");
  await expect(userPage).toHaveURL("/login");
  await logIn(userPage, email, ACCOUNT_PASSWORD);
  await userPage.goto("/account/security");
  const activity = userPage.locator("#activity").getByRole("listitem");
  await expect(activity.filter({ hasText: "Signed out of every device" })).toContainText("By an admin");
  await userPage.context().close();
});

test("an admin resends a confirmation email to someone who never finished signing up", async ({
  page,
}, testInfo) => {
  const email = uniqueEmail(testInfo, "stalled");
  await register(page, "Stalled User", email);
  await waitForEmails(email);
  await openAdmin(page);

  await openUser(page, email);
  await expect(page.locator("main").getByText("Awaiting password")).toBeVisible();
  await runAction(page, "Resend confirmation email");

  await expect(page.getByText("A new confirmation email is on its way.")).toBeVisible();
  await waitForEmails(email, 2);
});

test("an admin deletes an account", async ({ page, browser }, testInfo) => {
  const userPage = await (await browser.newContext()).newPage();
  const email = await createAccount(userPage, testInfo, "Removed User");
  await userPage.context().close();
  await openAdmin(page);

  await openUser(page, email);
  await runAction(page, "Delete account");

  await expect(page).toHaveURL("/admin/users");
  await expect(page.getByText("The account has been deleted.")).toBeVisible();
  await page.getByLabel("Search by email or name").fill(email);
  await page.getByLabel("Search by email or name").press("Enter");
  await expect(page.getByText("No users match that search.")).toBeVisible();

  await page.getByRole("link", { name: "Admin log" }).click();
  const entry = page.getByRole("row", { name: new RegExp(email) });
  await expect(entry).toContainText("Deleted the account");
  await expect(entry).toContainText(adminUser.email);
  await expect(entry.getByRole("link")).toHaveCount(0);
});

test("admins can't change their own account from the admin area", async ({ page }) => {
  await openAdmin(page);

  await openUser(page, adminUser.email);

  await expect(page.getByText("This is your account.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Disable account" })).toHaveCount(0);
});

test("a wrong password keeps the admin area locked", async ({ page }) => {
  await logIn(page, adminUser.email, adminUser.password);
  await expect(page).toHaveURL("/account");

  await page.goto("/admin/users");
  await expect(page).toHaveURL("/admin/confirm");
  await page.getByLabel("Current password").fill("not-the-password");
  await page.getByLabel("Current password").press("Enter");

  await expect(page.getByText("Your current password is incorrect.")).toBeVisible();
  await page.goto("/admin/users");
  await expect(page).toHaveURL("/admin/confirm");
});

test("confirming goes back to the admin page that was asked for", async ({ page }) => {
  await logIn(page, adminUser.email, adminUser.password);
  await expect(page).toHaveURL("/account");

  await page.goto("/admin/users?q=verified");
  await expect(page).toHaveURL("/admin/confirm");
  await page.getByLabel("Current password").fill(adminUser.password);
  await page.getByLabel("Current password").press("Enter");

  await expect(page).toHaveURL("/admin/users?q=verified");
  await expect(page.getByRole("row", { name: /verified@example.test/ })).toHaveCount(1);
});

test.describe("with a passkey-capable device", () => {
  test.skip(
    ({ browserName }) => browserName !== "chromium",
    "virtual authenticators are Chromium-only",
  );

  test("an admin confirms the admin area with a passkey", async ({ page }, testInfo) => {
    await withoutPasskeyAutofill(page);
    await addVirtualAuthenticator(page);
    const email = await createAccount(page, testInfo, "Passkey Admin");
    await addPasskeyToAccount(page);
    grantAdmin(email);

    await page.goto("/admin/log");
    await expect(page).toHaveURL("/admin/confirm");
    await expect(page.getByLabel("Current password")).toBeHidden();
    await page.getByRole("button", { name: "Continue" }).click();

    await expect(page).toHaveURL("/admin/log");
    const entry = page.getByRole("row", { name: new RegExp(email) });
    await expect(entry).toContainText("Made them an admin");
    await expect(entry).toContainText("Server console");
  });

  test("a disabled account can't sign in with its passkey", async ({ page, browser }, testInfo) => {
    await withoutPasskeyAutofill(page);
    await addVirtualAuthenticator(page);
    const email = await createAccount(page, testInfo, "Passkey Holder");
    await addPasskeyToAccount(page);
    await logOut(page, "Passkey Holder");
    const adminPage = await (await browser.newContext()).newPage();
    await openAdmin(adminPage);
    await openUser(adminPage, email);
    await runAction(adminPage, "Disable account");
    await expect(adminPage.getByText("Account disabled. They've been signed out everywhere.")).toBeVisible();

    await page.getByRole("button", { name: "Sign in with a passkey" }).click();

    await expect(page.getByText("This account has been disabled.")).toBeVisible();
    await expect(page).toHaveURL("/login");
    await adminPage.context().close();
  });
});
