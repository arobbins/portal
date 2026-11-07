import type { Page } from "@playwright/test";
import { expect, test } from "../support/test";
import { unverifiedUser, verifiedUser } from "../support/users";

async function logIn(page: Page, email: string, password: string) {
  await page.goto("/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Login" }).click();
}

test("signed-out visitors are sent from the account page to login", async ({ page }) => {
  await page.goto("/account");

  await expect(page).toHaveURL("/login");
});

test("logging in with valid details opens the account page", async ({ page }) => {
  await logIn(page, verifiedUser.email, verifiedUser.password);

  await expect(page).toHaveURL("/account");
  await expect(page.getByText(`Name: ${verifiedUser.name}`)).toBeVisible();
});

test("a wrong password shows an error and keeps the email", async ({ page }) => {
  await logIn(page, verifiedUser.email, "not-the-password");

  await expect(page).toHaveURL("/login");
  await expect(page.getByText("Invalid email or password.")).toBeVisible();
  await expect(page.getByLabel("Email")).toHaveValue(verifiedUser.email);
});

test("an unverified account is asked to verify its email", async ({ page }) => {
  await logIn(page, unverifiedUser.email, unverifiedUser.password);

  await expect(page.getByText("Please verify your email before logging in.")).toBeVisible();
});

test("signed-in users are sent from the login page to their account", async ({ page }) => {
  await logIn(page, verifiedUser.email, verifiedUser.password);
  await expect(page).toHaveURL("/account");

  await page.goto("/login");

  await expect(page).toHaveURL("/account");
});

test("logging out returns to login and protects the account page again", async ({ page }) => {
  await logIn(page, verifiedUser.email, verifiedUser.password);
  await expect(page).toHaveURL("/account");

  await page.getByRole("button", { name: verifiedUser.name }).click();
  await page.getByRole("button", { name: "Logout" }).click();

  await expect(page).toHaveURL("/login");
  await page.goto("/account");
  await expect(page).toHaveURL("/login");
});
