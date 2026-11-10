import { expect, test } from "../support/test";

test("home page renders the header navigation", async ({ page }) => {
  await page.goto("/");

  const nav = page.getByRole("navigation");
  for (const name of ["Home", "About", "WebTransport", "Styleguide"]) {
    await expect(nav.getByRole("link", { name })).toBeVisible();
  }
});

test("clicking a header link opens that page", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("navigation").getByRole("link", { name: "About" }).click();

  await expect(page).toHaveURL("/about");
  await expect(page.getByRole("heading", { level: 1, name: "About" })).toBeVisible();
});

test("every page has its own title and a mobile viewport", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveTitle("Home · Webserver");
  await expect(page.locator('meta[name="viewport"]')).toHaveAttribute(
    "content",
    /width=device-width/,
  );

  await page.goto("/login");
  await expect(page).toHaveTitle("Login · Webserver");
});

test("the full navigation shows without a menu button on desktop", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("button", { name: "Menu" })).toBeHidden();
  await expect(
    page.getByRole("navigation", { name: "Main" }).getByRole("link", { name: "Login" }),
  ).toBeVisible();
});
