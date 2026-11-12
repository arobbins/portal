import { expect, test } from "../support/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/styleguide");
});

test("a toggle switches on and off", async ({ page }) => {
  const toggle = page.getByRole("switch", { name: "Security alerts" });

  await expect(toggle).not.toBeChecked();
  await toggle.click();
  await expect(toggle).toBeChecked();
  await expect(page.getByRole("switch", { name: "Required notices" })).toBeDisabled();
});

test("a confirm dialog can be cancelled", async ({ page }) => {
  await page.getByRole("button", { name: "Delete account" }).click();
  const dialog = page.getByRole("dialog", { name: "Delete your account?" });

  await expect(dialog.getByText("You can't undo this.")).toBeVisible();
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toBeHidden();
});

test("the copy field says when it has copied", async ({ page, browserName, context }) => {
  test.skip(browserName !== "chromium", "Clipboard permissions are only grantable in Chromium");
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);

  await page.getByRole("button", { name: "Copy", exact: true }).click();

  await expect(page.getByRole("button", { name: "Copied" })).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    "wsk_live_4f9a2c7e1b8d3f60",
  );
});

test("a menu opens below its button", async ({ page }) => {
  const trigger = page.getByRole("button", { name: "Open menu" });
  await trigger.click();
  const item = page.getByRole("link", { name: "Settings" });

  await expect(item).toBeVisible();
  const triggerBox = await trigger.boundingBox();
  const itemBox = await item.boundingBox();
  const menuBox = await page.locator("#styleguide-menu").boundingBox();
  expect(itemBox!.y).toBeGreaterThan(triggerBox!.y + triggerBox!.height);
  expect(Math.abs(menuBox!.x - triggerBox!.x)).toBeLessThan(2);
  expect(menuBox!.y - (triggerBox!.y + triggerBox!.height)).toBeLessThan(12);
});

test("the active tab is marked as the current page", async ({ page }) => {
  const tabs = page.getByRole("navigation", { name: "Billing" });

  await expect(tabs.getByRole("link", { name: "Subscriptions" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  await expect(tabs.getByRole("link", { name: "Invoices" })).not.toHaveAttribute(
    "aria-current",
  );
});
