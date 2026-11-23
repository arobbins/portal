import type { Browser, Download, Page } from "@playwright/test";
import { openAdmin } from "../support/admin";
import { bearer, callMcp, createToken, createTokenDialog } from "../support/api";
import { ACCOUNT_PASSWORD, createAccount, logIn, logOut } from "../support/flows";
import { expect, test } from "../support/test";
import { readZip } from "../support/zip";

const NOTICE =
  "We'll send you our newsletter and product updates. You can turn them off any time in Settings.";
const GENERAL = "General newsletter";
const PRODUCT = "Product updates";
const SAVED = "Newsletter preferences saved.";
const SAVE_FAILED = "We couldn't save your newsletter preferences. Please try again.";
const NOTIFICATIONS = "/account/settings/notifications";
const PROCEDURES = "**/_topcoat/runtime/procedures/**";
const WRITE = "Read and update your profile (profile:write)" as const;

function newsletterSwitch(page: Page, name: string) {
  return page.getByRole("switch", { name, exact: true });
}

function activity(page: Page) {
  return page.locator("#activity").getByRole("listitem");
}

async function openNotifications(page: Page) {
  await page.goto(NOTIFICATIONS);
  await expect(page.getByText("Newsletters", { exact: true })).toBeVisible();
}

async function flip(page: Page, name: string) {
  await newsletterSwitch(page, name).click();
  await expect(page.getByText(SAVED).last()).toHaveCSS("opacity", "1");
}

async function openActivity(page: Page) {
  await page.goto("/account/settings/devices");
  return activity(page);
}

async function otherPage(browser: Browser) {
  return (await browser.newContext()).newPage();
}

async function exportData(page: Page): Promise<Map<string, string>> {
  await page.goto("/account");
  await page.getByRole("button", { name: "Export data" }).click();
  const dialog = page.getByRole("dialog", { name: "Export data" });
  const download: Promise<Download> = page.waitForEvent("download");
  await dialog.getByLabel("Current password").fill(ACCOUNT_PASSWORD);
  await dialog.getByLabel("Current password").press("Enter");
  return readZip(await (await download).path());
}

async function openUser(page: Page, email: string) {
  await page.getByLabel("Search by email or name").fill(email);
  await page.getByLabel("Search by email or name").press("Enter");
  const row = page.getByRole("row", { name: new RegExp(email) });
  await expect(row).toHaveCount(1);
  await row.getByRole("link").click();
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
}

test("S1 registering tells people and starts both newsletters on", async ({ page }, testInfo) => {
  await page.goto("/register");
  await expect(page.getByText(NOTICE)).toBeVisible();

  await createAccount(page, testInfo, "Newsletter Joiner");
  await openNotifications(page);

  await expect(newsletterSwitch(page, GENERAL)).toBeChecked();
  await expect(newsletterSwitch(page, PRODUCT)).toBeChecked();
});

test("S2 turning General newsletter off saves and is recorded", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Newsletter Leaver");
  await openNotifications(page);

  await flip(page, GENERAL);
  await expect(newsletterSwitch(page, GENERAL)).not.toBeChecked();

  await page.reload();
  await expect(newsletterSwitch(page, GENERAL)).not.toBeChecked();
  await expect(newsletterSwitch(page, PRODUCT)).toBeChecked();

  await logOut(page, "Newsletter Leaver");
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/account");
  await openNotifications(page);
  await expect(newsletterSwitch(page, GENERAL)).not.toBeChecked();
  await expect(newsletterSwitch(page, PRODUCT)).toBeChecked();

  const entries = await openActivity(page);
  await expect(entries.filter({ hasText: "Unsubscribed from General newsletter" })).toHaveCount(1);
});

test("S3 turning General newsletter back on is recorded", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Newsletter Returner");
  await openNotifications(page);
  await flip(page, GENERAL);
  await expect(newsletterSwitch(page, GENERAL)).not.toBeChecked();

  await page.reload();
  await flip(page, GENERAL);
  await expect(newsletterSwitch(page, GENERAL)).toBeChecked();

  const entries = await openActivity(page);
  await expect(entries.filter({ hasText: "Subscribed to General newsletter" })).toHaveCount(1);
  await expect(entries.filter({ hasText: "Unsubscribed from General newsletter" })).toHaveCount(1);
});

test.describe("when the server can't be reached", () => {
  test.use({
    allowedConsoleErrors: [/Failed to load resource|NetworkError|network connection/i],
  });

  test("S4 a failed save flips the switch back and changes nothing", async ({ page }, testInfo) => {
    await createAccount(page, testInfo, "Newsletter Offline");
    await openNotifications(page);
    await expect(newsletterSwitch(page, PRODUCT)).toBeChecked();
    let aborted = 0;
    await page.route(PROCEDURES, (route) => {
      aborted += 1;
      return route.abort();
    });

    await newsletterSwitch(page, PRODUCT).click();

    await expect.poll(() => aborted).toBeGreaterThan(0);
    await expect(page.getByText(SAVE_FAILED).last()).toBeVisible();
    await expect(newsletterSwitch(page, PRODUCT)).toBeChecked();
    await expect(page.getByText(SAVED)).toHaveCount(0);

    await page.unroute(PROCEDURES);
    await page.reload();
    await expect(newsletterSwitch(page, PRODUCT)).toBeChecked();
  });
});

test("S5 without JavaScript the Save button saves the choices", async ({ browser }, testInfo) => {
  const page = await (await browser.newContext({ javaScriptEnabled: false })).newPage();
  await createAccount(page, testInfo, "Newsletter Plain");
  await openNotifications(page);
  const product = newsletterSwitch(page, PRODUCT);

  await product.uncheck();
  await page.getByRole("button", { name: "Save newsletter preferences" }).click();

  await expect(page).toHaveURL(/\/account\/settings\/notifications/);
  await page.reload();
  await expect(newsletterSwitch(page, PRODUCT)).not.toBeChecked();
  await expect(newsletterSwitch(page, GENERAL)).toBeChecked();

  const entries = await openActivity(page);
  await expect(entries.filter({ hasText: "Unsubscribed from Product updates" })).toHaveCount(1);
  await expect(entries.filter({ hasText: GENERAL })).toHaveCount(0);
  await page.context().close();
});

test("S6 the API reads and changes the choices within its scopes", async ({
  page,
  request,
}, testInfo) => {
  await createAccount(page, testInfo, "Newsletter Scripter");
  const reader = await createToken(page, "Newsletter reader");
  await createTokenDialog(page).getByRole("button", { name: "Done" }).click();
  const writer = await createToken(page, "Newsletter writer", WRITE);
  await createTokenDialog(page).getByRole("button", { name: "Done" }).click();

  const read = await request.get("/api/v1/newsletters", { headers: bearer(reader) });
  expect(read.status()).toBe(200);
  expect(await read.json()).toMatchObject({ general_newsletter: true, product_updates: true });
  const llms = await (await request.get("/llms.txt")).text();
  expect(llms).toContain("GET /api/v1/newsletters");
  expect(llms).toContain("PATCH /api/v1/newsletters");
  const change = { product_updates: false };

  const refused = await request.patch("/api/v1/newsletters", {
    headers: bearer(reader),
    data: change,
  });
  expect(refused.status()).toBe(403);
  expect(await refused.json()).toMatchObject({ status: 403, code: "insufficient_scope" });
  await openNotifications(page);
  await expect(newsletterSwitch(page, PRODUCT)).toBeChecked();

  const changed = await request.patch("/api/v1/newsletters", {
    headers: bearer(writer),
    data: change,
  });
  expect(changed.status()).toBe(200);
  expect(await changed.json()).toMatchObject({ general_newsletter: true, product_updates: false });

  await openNotifications(page);
  await expect(newsletterSwitch(page, PRODUCT)).not.toBeChecked();
  await expect(newsletterSwitch(page, GENERAL)).toBeChecked();
  const entries = await openActivity(page);
  const entry = entries.filter({ hasText: "Unsubscribed from Product updates" });
  await expect(entry).toHaveCount(1);
  await expect(entry).toContainText("API token");
});

test("S7 MCP tools read and change the choices", async ({ page, request }, testInfo) => {
  await createAccount(page, testInfo, "Newsletter Assistant");
  const token = await createToken(page, "Claude", WRITE, "AI assistants (MCP)");

  const list = await callMcp(request, token, "tools/list");
  const names = (await list.json()).result.tools.map((tool: { name: string }) => tool.name);
  expect(names).toEqual(expect.arrayContaining(["get_newsletters", "update_newsletters"]));

  const update = await callMcp(request, token, "tools/call", {
    name: "update_newsletters",
    arguments: { product_updates: false },
  });
  const result = (await update.json()).result;
  expect(result.isError).toBe(false);
  expect(result.structuredContent).toMatchObject({
    general_newsletter: true,
    product_updates: false,
  });

  await openNotifications(page);
  await expect(newsletterSwitch(page, PRODUCT)).not.toBeChecked();
  await expect(newsletterSwitch(page, GENERAL)).toBeChecked();
});

test("S8 the admin user page shows the choices", async ({ page, browser }, testInfo) => {
  const userPage = await otherPage(browser);
  const email = await createAccount(userPage, testInfo, "Newsletter Watched");
  await openNotifications(userPage);
  await flip(userPage, PRODUCT);
  await userPage.context().close();

  await openAdmin(page);
  await openUser(page, email);

  const main = page.locator("main");
  await expect(main.getByText("Newsletters", { exact: true })).toBeVisible();
  await expect(main).toContainText(/General newsletter\s*:?\s*Subscribed/);
  await expect(main).toContainText(/Product updates\s*:?\s*Not subscribed/);
});

test("S9 the data export includes the newsletter choices", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Newsletter Exporter");
  await openNotifications(page);
  await flip(page, GENERAL);

  const files = await exportData(page);
  const rows = (files.get("notifications.csv") ?? "").split("\n").map((row) => row.trim());

  expect(rows).toContain("New sign-in alerts,yes");
  expect(rows).toContain("General newsletter,no");
  expect(rows).toContain("Product updates,yes");
});
