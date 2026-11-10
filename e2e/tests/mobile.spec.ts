import { createAccount } from "../support/flows";
import { expect, test } from "../support/test";

test.use({ viewport: { width: 390, height: 844 } });

async function expectNoSidewaysScroll(page: import("@playwright/test").Page) {
  const width = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(width).toBeLessThanOrEqual(390);
}

test("public pages fit a phone screen without sideways scrolling", async ({ page }) => {
  for (const path of ["/", "/about", "/login", "/register", "/reset-password", "/styleguide"]) {
    await page.goto(path);
    await expectNoSidewaysScroll(page);
  }
});

test("the site menu opens from a button on phones", async ({ page }) => {
  await page.goto("/");
  const navigation = page.getByRole("navigation", { name: "Main" });
  await expect(navigation.getByRole("link", { name: "Login" })).toBeHidden();

  await page.getByRole("button", { name: "Menu" }).click();
  await navigation.getByRole("link", { name: "Login" }).click();

  await expect(page).toHaveURL("/login");
});

test("account pages fit a phone screen", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Phone Tester");

  for (const path of ["/account", "/account/billing"]) {
    await page.goto(path);
    await expectNoSidewaysScroll(page);
  }
  await page.getByRole("link", { name: "Security", exact: true }).click();
  await expect(page).toHaveURL("/account/security");
});
