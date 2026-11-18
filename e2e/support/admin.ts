import { expect, type Page } from "@playwright/test";
import { logIn } from "./flows";
import { adminUser } from "./users";

export async function openAdmin(page: Page) {
  await logIn(page, adminUser.email, adminUser.password);
  await expect(page).toHaveURL("/account");
  await page
    .getByRole("navigation", { name: "Account" })
    .getByRole("link", { name: "Admin", exact: true })
    .click();
  await expect(page).toHaveURL("/admin/confirm");
  await page.getByLabel("Current password").fill(adminUser.password);
  await page.getByLabel("Current password").press("Enter");
  await expect(page).toHaveURL("/admin/users");
}
