import { expect, test } from "../support/test";

test.use({ allowedConsoleErrors: [/status of 404/] });

test("an unknown URL shows the 404 page with a 404 status", async ({ page }) => {
  const response = await page.goto("/does-not-exist");

  expect(response?.status()).toBe(404);
  await expect(page.getByRole("heading", { name: "404" })).toBeVisible();
  await expect(page.getByText("You requested /does-not-exist")).toBeVisible();
});

test("an unknown URL has its own page title", async ({ page }) => {
  await page.goto("/does-not-exist");

  await expect(page).toHaveTitle("Page not found · Webserver");
});
