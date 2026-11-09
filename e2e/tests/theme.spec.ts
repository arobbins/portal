import type { Page } from "@playwright/test";
import { expect, test } from "../support/test";

const LIGHT_BG = "rgb(250, 248, 244)";
const DARK_BG = "rgb(18, 16, 14)";

function headerSwitcher(page: Page) {
  return page.getByRole("banner").getByRole("group", { name: "Color theme" });
}

test("the site is dark by default", async ({ page }) => {
  await page.goto("/");

  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.locator("body")).toHaveCSS("background-color", DARK_BG);
  await expect(headerSwitcher(page).getByRole("button", { name: "Dark" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});

test("choosing Light switches the theme instantly and remembers it after a reload", async ({
  page,
}) => {
  await page.goto("/");
  const light = headerSwitcher(page).getByRole("button", { name: "Light" });

  await light.click();

  await expect(page).toHaveURL("/");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(page.locator("body")).toHaveCSS("background-color", LIGHT_BG);
  await expect(light).toHaveAttribute("aria-pressed", "true");

  await page.reload();

  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(headerSwitcher(page).getByRole("button", { name: "Light" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});

test("choosing System follows the operating system's color scheme", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");

  await headerSwitcher(page).getByRole("button", { name: "System" }).click();
  await expect(page.locator("body")).toHaveCSS("background-color", LIGHT_BG);

  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("body")).toHaveCSS("background-color", DARK_BG);
});

test("going back after switching keeps the new theme", async ({ page }) => {
  await page.goto("/");
  const about = page.getByRole("link", { name: "About", exact: true }).first();
  await about.hover();

  await headerSwitcher(page).getByRole("button", { name: "Light" }).click();
  await about.click();
  await expect(page).toHaveURL("/about");
  await page.goBack();

  await expect(page).toHaveURL("/");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(headerSwitcher(page).getByRole("button", { name: "Light" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});

test("a page restored from the back/forward cache picks up the current theme", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => {
    document.cookie = "theme=light; path=/";
    window.dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true }));
  });

  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(page.locator("body")).toHaveCSS("background-color", LIGHT_BG);
});
