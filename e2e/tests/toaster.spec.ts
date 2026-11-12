import { expect, test } from "../support/test";

const TOAST_TEXT = "Success! This is a toaster notice.";

test.use({ holdToasts: false });

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

test("the toast stays above form controls", async ({ page }) => {
  await page.getByRole("button", { name: "Show toast" }).click();
  await expect(page.getByText(TOAST_TEXT)).toHaveCSS("opacity", "1");
  await page.getByLabel("Plan", { exact: true }).evaluate((select) => {
    const box = select.getBoundingClientRect();
    window.scrollBy(0, box.top - 20);
  });

  const toast = await page.getByText(TOAST_TEXT).boundingBox();
  const textOnTop = await page.evaluate(
    ({ x, y }) => document.elementFromPoint(x, y)?.textContent ?? "",
    { x: toast!.x + toast!.width / 2, y: toast!.y + toast!.height / 2 },
  );
  expect(textOnTop).toContain(TOAST_TEXT);
});
