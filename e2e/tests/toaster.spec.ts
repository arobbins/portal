import { expect, test } from "../support/test";

const TOAST_TEXT = "Success! This is a toaster notice.";

test.beforeEach(async ({ page }) => {
  await page.goto("/styleguide");
});

test("clicking Show toast shows the notice centered at the top of the page", async ({ page }) => {
  const toast = page.getByText(TOAST_TEXT);
  await expect(toast).toHaveCSS("opacity", "0");

  await page.getByRole("button", { name: "Show toast" }).click();

  await expect(toast).toHaveCSS("opacity", "1");

  const box = await toast.boundingBox();
  const pageWidth = await page.evaluate(() => document.documentElement.clientWidth);
  expect(box).not.toBeNull();
  expect(Math.abs(box!.x + box!.width / 2 - pageWidth / 2)).toBeLessThanOrEqual(1);
  expect(box!.y).toBeLessThan(40);
});

test("the toast hides itself after a few seconds", async ({ page }) => {
  const toast = page.getByText(TOAST_TEXT);

  await page.getByRole("button", { name: "Show toast" }).click();
  await expect(toast).toHaveCSS("opacity", "1");

  await expect(toast).toHaveCSS("opacity", "0", { timeout: 6_000 });
});

test("clicking the toast closes it early", async ({ page }) => {
  const toast = page.getByText(TOAST_TEXT);

  await page.getByRole("button", { name: "Show toast" }).click();
  await expect(toast).toHaveCSS("opacity", "1");

  await toast.click();

  await expect(toast).toHaveCSS("opacity", "0", { timeout: 1_000 });
});
