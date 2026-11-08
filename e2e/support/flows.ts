import type { Page, TestInfo } from "@playwright/test";

export function uniqueEmail(testInfo: TestInfo, label: string): string {
  return `${label}-${testInfo.project.name}-${Date.now()}@example.test`.toLowerCase();
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
