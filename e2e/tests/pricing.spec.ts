import { expect, test, type Page } from "../support/test";

const modifier = process.platform === "darwin" ? "Meta" : "Control";

function proCard(page: Page) {
  return page
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: "Pro", exact: true }) });
}

function proPrice(page: Page) {
  return proCard(page).getByText(/^\$[\d.]+ \/ month$/);
}

test("any billing value but monthly opens Pricing with Billed yearly on", async ({ page }) => {
  await page.goto("/pricing?billing=weekly");

  await expect(proCard(page).getByRole("switch", { name: "Billed yearly" })).toBeChecked();
  await expect(proPrice(page)).toHaveText("$10 / month");
});

test("turning Billed yearly back on returns the address to /pricing", async ({ page }) => {
  await page.goto("/pricing?billing=monthly");
  const billedYearly = proCard(page).getByRole("switch", { name: "Billed yearly" });
  await expect(billedYearly).not.toBeChecked();
  await expect(proPrice(page)).toHaveText("$12 / month");

  await billedYearly.click();

  await expect(billedYearly).toBeChecked();
  await expect(proPrice(page)).toHaveText("$10 / month");
  await expect(page).toHaveURL(/\/pricing$/);
});

test("the price animates when it changes, but not for reduced motion", async ({ page }) => {
  await page.goto("/pricing");
  const billedYearly = proCard(page).getByRole("switch", { name: "Billed yearly" });
  await expect(proPrice(page)).toHaveCSS("animation-name", "none");

  await billedYearly.click();
  await expect(proPrice(page)).toHaveText("$12 / month");
  await expect(proPrice(page)).not.toHaveCSS("animation-name", "none");

  await page.emulateMedia({ reducedMotion: "reduce" });
  await billedYearly.click();
  await expect(proPrice(page)).toHaveText("$10 / month");
  await expect(proPrice(page)).toHaveCSS("animation-name", "none");
});

test.describe("without JavaScript", () => {
  test.use({ javaScriptEnabled: false });

  test("a modifier-click on Show monthly prices opens it in a new tab", async ({ page, context }) => {
    await page.goto("/pricing");
    await expect(proCard(page).getByRole("switch")).toBeHidden();
    const showMonthly = proCard(page).getByRole("link", { name: "Show monthly prices" });

    await expect(showMonthly).toBeVisible();
    const [tab] = await Promise.all([
      context.waitForEvent("page"),
      showMonthly.click({ modifiers: [modifier] }),
    ]);
    await tab.waitForLoadState("load");

    await expect(tab).toHaveURL(/\/pricing\?billing=monthly$/);
    await expect(proPrice(tab)).toHaveText("$12 / month");
    await expect(proCard(tab).getByRole("link", { name: "Show yearly prices" })).toBeVisible();
    await expect(page).toHaveURL(/\/pricing$/);
    await expect(proPrice(page)).toHaveText("$10 / month");
  });
});
