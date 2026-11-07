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
