import { createAccount } from "../support/flows";
import { expect, test } from "../support/test";

test.use({ timezoneId: "Asia/Tokyo" });

test("times start in the browser's time zone and can be changed", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Zone Chooser");
  await page.goto("/account/settings/time-zone");
  const card = page.locator("#time-zone");
  await expect(card).toContainText("Asia/Tokyo");

  await card.getByRole("button", { name: "Change time zone" }).click();
  const dialog = page.getByRole("dialog", { name: "Change time zone" });
  await dialog.getByLabel("Time zone").selectOption("Europe/Paris");
  await dialog.getByRole("button", { name: "Save time zone" }).click();

  await expect(page.getByText("Time zone saved.")).toBeVisible();
  await expect(card).toContainText("Dates and times are shown in Europe/Paris.");
  await page.goto("/account/settings/devices");
  const activity = page.locator("#activity").getByRole("listitem");
  await expect(activity.filter({ hasText: "Time zone changed" })).toContainText(/CES?T/);
});

test("the time zone can come from this device", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Zone Detector");
  await page.goto("/account/settings/time-zone");

  await page.getByRole("button", { name: "Change time zone" }).click();
  const dialog = page.getByRole("dialog", { name: "Change time zone" });
  await dialog.getByRole("button", { name: "Use this device's time zone" }).click();
  await expect(dialog.getByLabel("Time zone")).toHaveValue("Asia/Tokyo");
  await dialog.getByRole("button", { name: "Save time zone" }).click();

  await expect(page.locator("#time-zone")).toContainText("Asia/Tokyo");
});

test("the time zone can be saved without JavaScript", async ({ browser }, testInfo) => {
  const page = await (await browser.newContext({ javaScriptEnabled: false })).newPage();
  await createAccount(page, testInfo, "Zone Without Script");
  await page.goto("/account/settings/time-zone");

  await page.getByRole("button", { name: "Change time zone" }).click();
  const dialog = page.getByRole("dialog", { name: "Change time zone" });
  await dialog.getByLabel("Time zone").selectOption("America/New_York");
  await dialog.getByRole("button", { name: "Save time zone" }).click();

  await expect(page).toHaveURL("/account/settings/time-zone");
  await expect(page.locator("#time-zone")).toContainText("America/New_York");
  await page.context().close();
});

test.describe("in a browser set to New York time", () => {
  test.use({ timezoneId: "America/New_York" });

  test("a new account starts in the browser's time zone, not UTC", async ({ page }, testInfo) => {
    await createAccount(page, testInfo, "Zoned Person");
    await page.goto("/account/settings/time-zone");

    await expect(page.locator("#time-zone")).toContainText("America/New_York");
    await expect(page.locator("#time-zone")).not.toContainText("UTC.");
  });
});
