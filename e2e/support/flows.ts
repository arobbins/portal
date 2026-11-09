import { expect, type Page, type TestInfo } from "@playwright/test";
import { verificationLink, waitForEmails } from "./mail";

export function uniqueEmail(testInfo: TestInfo, label: string): string {
  const suffix = `${Date.now()}-${testInfo.workerIndex}-${Math.random().toString(36).slice(2, 8)}`;
  return `${label}-${testInfo.project.name}-${suffix}@example.test`.toLowerCase();
}

export async function register(page: Page, name: string, email: string) {
  await page.goto("/register");
  await page.getByLabel("Name").fill(name);
  await page.getByLabel("Email").fill(email);
  await page.getByRole("button", { name: "Register account" }).click();
}

export async function setPassword(page: Page, password: string, confirmation = password) {
  await page.getByLabel("New password", { exact: true }).fill(password);
  await page.getByLabel("Confirm new password").fill(confirmation);
  await page.getByRole("button", { name: "Set password" }).click();
}

export async function logIn(page: Page, email: string, password: string) {
  await page.goto("/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Login" }).click();
}

export const ACCOUNT_PASSWORD = "a-strong-test-password";

export async function createAccount(page: Page, testInfo: TestInfo, name: string) {
  const email = uniqueEmail(testInfo, name.replaceAll(" ", "-"));
  await register(page, name, email);
  await page.goto(verificationLink((await waitForEmails(email))[0]));
  await setPassword(page, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/account");
  return email;
}

export async function logOut(page: Page, name: string) {
  await page.getByRole("button", { name }).click();
  await page.getByRole("button", { name: "Logout" }).click();
  await expect(page).toHaveURL("/login");
}
