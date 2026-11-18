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

  const field = page.getByLabel("API token");
  await expect(field).toBeFocused();
  expect(
    await field.evaluate((input: HTMLInputElement) =>
      input.value.slice(input.selectionStart ?? 0, input.selectionEnd ?? 0),
    ),
  ).toBe("wsk_live_4f9a2c7e1b8d3f60");

  await expect(page.getByRole("button", { name: "Copy", exact: true })).toBeVisible();
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

test("a table that scrolls sideways can be reached with the keyboard", async ({ page }) => {
  const region = page.getByRole("region", { name: "Invoices" });

  await region.focus();
  await expect(region).toBeFocused();
  await expect(region.getByRole("table", { name: "Invoices" })).toBeVisible();
  await expect(region.getByRole("rowheader", { name: "Nov 1, 2026" })).toBeVisible();
});

test("a sortable column says how it's sorted", async ({ page }) => {
  const table = page.getByRole("table", { name: "Invoices" });

  await expect(table.getByRole("columnheader", { name: "Date" })).toHaveAttribute(
    "aria-sort",
    "descending",
  );
  await expect(table.getByRole("columnheader", { name: "Amount" }).getByRole("link")).toBeVisible();
  await expect(table.getByRole("columnheader", { name: "Plan" })).not.toHaveAttribute("aria-sort");
});

test("a sticky table header stays in view while its rows scroll", async ({ page }) => {
  const region = page.getByRole("region", { name: "Recent events" });
  const heading = region.getByRole("columnheader", { name: "When" });
  await region.scrollIntoViewIfNeeded();

  await region.evaluate((element) => element.scrollTo({ top: 200 }));

  const regionBox = (await region.boundingBox())!;
  await expect.poll(async () => (await heading.boundingBox())!.y).toBeCloseTo(regionBox.y + 1, 0);
});

test("a pager links to the previous and next pages", async ({ page }) => {
  const pages = page.getByRole("navigation", { name: "Pages" });

  await expect(pages.getByRole("link", { name: "Previous" })).toBeVisible();
  await expect(pages.getByRole("link", { name: "Next" })).toBeVisible();
});

test("an empty table says so inside the table", async ({ page }) => {
  const table = page.getByRole("table", { name: "Refunds" });

  await expect(table.getByText("No refunds yet.")).toBeVisible();
});

test("a stacked table fits a phone screen", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const region = page.getByRole("region", { name: "Invoices" });

  await expect(region.getByRole("rowheader", { name: "Nov 1, 2026" })).toBeVisible();
  await expect(region.getByRole("columnheader", { name: "Date" })).toBeVisible();
  await expect(region.getByRole("cell", { name: "Paid" }).first()).toBeHidden();
  const overflow = await region.evaluate((element) => element.scrollWidth - element.clientWidth);
  expect(overflow).toBe(0);
});

test("a row heading with a link opens it from anywhere in the cell", async ({ page }) => {
  const table = page.getByRole("table", { name: "Invoices" });
  const cell = table.getByRole("rowheader", { name: "Nov 1, 2026" });
  await cell.scrollIntoViewIfNeeded();
  const box = (await cell.boundingBox())!;

  await expect(cell.getByRole("link", { name: "Nov 1, 2026" })).toBeVisible();
  await page.mouse.click(box.x + box.width - 3, box.y + 3);

  await expect(page).toHaveURL(/#invoice-2026-11-01$/);
});

test("a setting toggle saves as soon as it's switched", async ({ page }) => {
  const setting = page.getByRole("switch", { name: "Weekly summary" });

  await setting.click();

  await expect(page.getByText("Saved.", { exact: true })).toBeVisible();
  await expect(setting).toBeChecked();
  await expect(setting).toBeEnabled();
});
