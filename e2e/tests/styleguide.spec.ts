import { expect, test } from "../support/test";

test("the sidebar links to every section of the styleguide", async ({ page }) => {
  await page.goto("/styleguide");

  const sidebar = page.getByRole("navigation", { name: "Styleguide sections" });
  const links = sidebar.getByRole("link");
  const names = await links.allTextContents();
  const targets = await links.evaluateAll((anchors) =>
    anchors.map((anchor) => anchor.getAttribute("href")),
  );
  for (const target of targets) {
    await expect(page.locator(`${target} h2`).first()).toBeAttached();
  }

  for (const name of names) {
    await sidebar.getByRole("link", { name, exact: true }).click();
    await expect(page.getByRole("heading", { level: 2, name, exact: true })).toBeInViewport();
  }
});

test("the sidebar marks the section you're reading", async ({ page }) => {
  await page.goto("/styleguide#code");

  const sidebar = page.getByRole("navigation", { name: "Styleguide sections" });
  await expect(sidebar.getByRole("link", { name: "Code", exact: true })).toHaveAttribute(
    "aria-current",
    "location",
  );
  await expect(sidebar.locator("[aria-current]")).toHaveCount(1);
});

test("on phones the sections are in a menu above the content", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/styleguide");

  await expect(page.getByRole("navigation", { name: "Styleguide sections" })).toBeHidden();
  await page.getByText("On this page").click();
  const menu = page.getByRole("navigation", { name: "Styleguide sections" });
  await menu.getByRole("link", { name: "Buttons", exact: true }).click();

  await expect(page).toHaveURL(/#buttons$/);
});
