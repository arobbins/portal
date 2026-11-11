import type { Page } from "@playwright/test";
import { ACCOUNT_PASSWORD, createAccount, logIn, logOut } from "../support/flows";
import { expect, test } from "../support/test";

const NEW_PASSWORD = "a-brand-new-password";
const CHANGED = "Your password has been changed. Your other devices have been signed out.";

function dialog(page: Page) {
  return page.getByRole("dialog", { name: "Change password" });
}

async function openDialog(page: Page) {
  await page.goto("/account/security");
  await page.getByRole("button", { name: "Change password" }).click();
  await expect(dialog(page)).toBeVisible();
}

async function changePassword(page: Page, current: string, next: string, confirmation = next) {
  await openDialog(page);
  await dialog(page).getByLabel("Current password").fill(current);
  await dialog(page).getByLabel("New password", { exact: true }).fill(next);
  await dialog(page).getByLabel("Confirm new password").fill(confirmation);
  await dialog(page).getByLabel("Confirm new password").press("Enter");
}

test("the security page lists password, two-factor and single sign-on", async ({
  page,
}, testInfo) => {
  await createAccount(page, testInfo, "Sections");
  await page.goto("/account/security");

  await expect(page.getByRole("button", { name: "Change password" })).toBeEnabled();
  await expect(page.getByRole("button", { name: "Set up" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "Connect" })).toBeDisabled();
  await expect(dialog(page)).toBeHidden();
});

test("a signed-in user can change their password in a dialog", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Changer");

  await changePassword(page, ACCOUNT_PASSWORD, NEW_PASSWORD);

  await expect(page.getByText(CHANGED)).toHaveCSS("opacity", "1");
  await expect(dialog(page)).toBeHidden();
  await expect(page).toHaveURL("/account/security");

  await logOut(page, "Changer");
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page.getByText("Invalid email or password.")).toBeVisible();
  await logIn(page, email, NEW_PASSWORD);
  await expect(page).toHaveURL("/account");
});

test("a wrong current password is shown in the dialog, on that field", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Wrong Current");

  await changePassword(page, "not-my-password", NEW_PASSWORD);

  await expect(dialog(page).getByRole("alert")).toHaveText("Your current password is incorrect.");
  await expect(dialog(page).getByLabel("Current password")).toHaveAttribute("aria-invalid", "true");
  await expect(dialog(page).getByLabel("New password", { exact: true })).toHaveAttribute(
    "aria-invalid",
    "false",
  );

  await dialog(page).getByRole("button", { name: "Close" }).click();
  await logOut(page, "Wrong Current");
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/account");
});

test("the new password must be confirmed", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Mismatch");

  await changePassword(page, ACCOUNT_PASSWORD, NEW_PASSWORD, "something-else-entirely");

  await expect(dialog(page).getByRole("alert")).toHaveText("Passwords do not match.");
  await expect(dialog(page).getByLabel("Confirm new password")).toHaveAttribute(
    "aria-invalid",
    "true",
  );
});

test("reopening the dialog starts fresh", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Reopen");
  await changePassword(page, "not-my-password", NEW_PASSWORD);
  await expect(dialog(page).getByRole("alert")).toBeVisible();

  await dialog(page).getByRole("button", { name: "Close" }).click();
  await page.getByRole("button", { name: "Change password" }).click();

  await expect(dialog(page).getByLabel("Current password")).toHaveValue("");
  await expect(dialog(page).getByRole("alert")).toBeHidden();
});

test("changing the password signs out other devices but not this one", async ({
  page,
  browser,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Two Devices");
  const otherDevice = await browser.newPage();
  await logIn(otherDevice, email, ACCOUNT_PASSWORD);
  await expect(otherDevice).toHaveURL("/account");

  await changePassword(page, ACCOUNT_PASSWORD, NEW_PASSWORD);
  await expect(page.getByText(CHANGED)).toHaveCSS("opacity", "1");

  await otherDevice.goto("/account");
  await expect(otherDevice).toHaveURL("/login");
  await page.goto("/account");
  await expect(page).toHaveURL("/account");
  await otherDevice.close();
});

test("without JavaScript the form still works", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "No Script");
  const post = (current: string) =>
    page.request.post("/account/password", {
      form: { current_password: current, password: NEW_PASSWORD, confirm_password: NEW_PASSWORD },
      headers: { Origin: new URL(page.url()).origin },
      maxRedirects: 0,
    });

  const rejected = await post("not-my-password");
  expect(await rejected.text()).toContain("Your current password is incorrect.");

  const accepted = await post(ACCOUNT_PASSWORD);
  expect(accepted.status()).toBe(303);
  expect(accepted.headers().location).toBe("/account/security");
  await page.goto("/account/security");
  await expect(page.getByText(CHANGED)).toHaveCSS("opacity", "1");

  await logOut(page, "No Script");
  await logIn(page, email, NEW_PASSWORD);
  await expect(page).toHaveURL("/account");
});

test("the account sidebar links to the password section", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Sidebar");

  await page.getByRole("link", { name: "Password", exact: true }).click();

  await expect(page).toHaveURL("/account/security#password");
  await expect(page.getByRole("button", { name: "Change password" })).toBeVisible();
});

test("signed-out visitors can't change a password", async ({ page }) => {
  await page.goto("/login");
  const response = await page.request.post("/account/password", {
    form: { current_password: "x", password: NEW_PASSWORD, confirm_password: NEW_PASSWORD },
    headers: { Origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });

  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe("/login");
});
