import { createAccount } from "../support/flows";
import { expect, test } from "../support/test";

test.use({ allowedConsoleErrors: [/status of 404/] });

test("an unknown URL shows the 404 page with a 404 status", async ({ page }) => {
  const response = await page.goto("/does-not-exist");

  expect(response?.status()).toBe(404);
  await expect(page.getByRole("heading", { name: "Page not found", level: 1 })).toBeVisible();
  await expect(page.getByText("Error 404")).toBeVisible();
  await expect(page.getByText("/does-not-exist")).toBeVisible();
});

test("an unknown URL has its own page title", async ({ page }) => {
  await page.goto("/does-not-exist");

  await expect(page).toHaveTitle("Page not found · Webserver");
});

test("a signed-out visitor is offered the home page and sign-in", async ({ page }) => {
  await page.goto("/does-not-exist");
  const main = page.getByRole("main");

  await expect(main.getByRole("link", { name: "Sign in" })).toHaveAttribute("href", "/login");
  await main.getByRole("link", { name: "Go to the home page" }).click();
  await expect(page).toHaveURL("/");
});

test("a signed-in user is offered their account", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Lost Person");
  await page.goto("/does-not-exist");

  await page.getByRole("main").getByRole("link", { name: "Go to your account" }).click();
  await expect(page).toHaveURL("/account");
});

test("a long unknown address wraps instead of widening the page", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`/${"a-very-long-segment".repeat(12)}`);

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBe(0);
});
