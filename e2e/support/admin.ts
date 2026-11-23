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
  const dialog = page.getByRole("dialog", { name: "Admin area" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("Current password").fill(adminUser.password);
  await dialog.getByLabel("Current password").press("Enter");
  await expect(page).toHaveURL("/admin/users");
}
