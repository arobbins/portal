import type { Download, Page } from "@playwright/test";
import { createToken } from "../support/api";
import { connectApp } from "../support/connected-apps";
import { parseCsv } from "../support/csv";
import { ACCOUNT_PASSWORD, createAccount } from "../support/flows";
import { emailsAbout, waitForEmailAbout } from "../support/mail";
import {
  addPasskeyToAccount,
  addVirtualAuthenticator,
  withoutPasskeyAutofill,
} from "../support/passkeys";
import { expect, test } from "../support/test";
import { readZip } from "../support/zip";

const EXPORT_FILES = [
  "README.txt",
  "profile.csv",
  "passkeys.csv",
  "api_tokens.csv",
  "devices.csv",
  "activity.csv",
  "notifications.csv",
  "two_factor.csv",
  "api_feedback.csv",
  "connected_apps.csv",
  "billing.csv",
];
const DOWNLOADED_EMAIL = "Your account data was downloaded";
const EXPORTED_ACTIVITY = "Account data exported";

function exportDialog(page: Page) {
  return page.getByRole("dialog", { name: "Export data" });
}

async function openExport(page: Page) {
  await page.goto("/account");
  await page.getByRole("button", { name: "Export data" }).click();
  await expect(exportDialog(page)).toBeVisible();
}

async function exportWithPassword(page: Page, password = ACCOUNT_PASSWORD): Promise<Download> {
  const download = page.waitForEvent("download");
  await exportDialog(page).getByLabel("Current password").fill(password);
  await exportDialog(page).getByLabel("Current password").press("Enter");
  return download;
}

async function filesIn(download: Download): Promise<Map<string, string>> {
  return readZip(await download.path());
}

function todayInUtc(): string {
  return new Date().toISOString().slice(0, 10);
}

test.describe("with an API token", () => {
  let files: Map<string, string>;
  let email: string;
  let token: string;
  let page: Page;

  test.beforeEach(async ({ page: testPage }, testInfo) => {
    page = testPage;
    email = await createAccount(page, testInfo, "Data Keeper");
    token = await createToken(page, "Backup script");
    await openExport(page);
    const download = await exportWithPassword(page);
    expect(download.suggestedFilename()).toBe(`webserver-data-${todayInUtc()}.zip`);
    files = await filesIn(download);
  });

  test("S1 a confirmed export downloads every kind of data", async () => {
    expect([...files.keys()].sort()).toEqual([...EXPORT_FILES].sort());
    for (const [name, contents] of files) {
      if (name.endsWith(".csv")) {
        expect(contents.split("\n")[0].trim(), `${name} has a header row`).not.toBe("");
      }
    }
    expect(files.get("profile.csv")).toContain("Data Keeper");
    expect(files.get("profile.csv")).toContain(email);
    expect(files.get("api_tokens.csv")).toContain("Backup script");
    expect(files.get("activity.csv")).toMatch(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z/);
  });

  test("S2 the export contains no secrets", async () => {
    for (const [name, contents] of files) {
      expect(contents, `${name} has no password hash`).not.toContain("$argon2");
      expect(contents, `${name} has no API token`).not.toContain(token);
    }
  });

  test("S3 the owner is told and the export is in their activity", async () => {
    await waitForEmailAbout(email, DOWNLOADED_EMAIL);
    await page.goto("/account/settings/devices");
    const activity = page.locator("#activity").getByRole("listitem");
    await expect(activity.filter({ hasText: EXPORTED_ACTIVITY })).toHaveCount(1);
  });
});

test("S4 a wrong password exports nothing", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Wrong Guess");
  let downloaded = false;
  page.on("download", () => {
    downloaded = true;
  });

  await openExport(page);
  await exportDialog(page).getByLabel("Current password").fill("not-the-password");
  await exportDialog(page).getByLabel("Current password").press("Enter");

  await expect(exportDialog(page).getByRole("alert")).toHaveText(
    "Your current password is incorrect.",
  );
  await new Promise((resolve) => setTimeout(resolve, 1_000));
  expect(downloaded).toBe(false);
  expect(await emailsAbout(email, DOWNLOADED_EMAIL)).toEqual([]);
});

test.describe("with a passkey", () => {
  test.skip(
    ({ browserName }) => browserName !== "chromium",
    "virtual authenticators are Chromium-only",
  );

  test("S5 a passkey confirms the export", async ({ page }, testInfo) => {
    await withoutPasskeyAutofill(page);
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Passkey Keeper");
    await addPasskeyToAccount(page);

    await openExport(page);
    await expect(exportDialog(page).getByLabel("Current password")).toBeHidden();
    const download = page.waitForEvent("download");
    await exportDialog(page).getByRole("button", { name: "Export data" }).click();

    const files = await filesIn(await download);
    expect([...files.keys()].sort()).toEqual([...EXPORT_FILES].sort());
  });
});

test("S6 the delete dialog offers an export first", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Almost Leaving");
  await page.goto("/account");
  await page.getByRole("button", { name: "Delete account" }).click();
  const deleteDialog = page.getByRole("dialog", { name: "Delete account" });
  await expect(deleteDialog).toBeVisible();
  await expect(deleteDialog.getByText("Want a copy first?")).toBeVisible();

  await deleteDialog.getByText("Export your data").click();

  await expect(deleteDialog).toBeHidden();
  await expect(exportDialog(page)).toBeVisible();
});

test("S7 exporting works without JavaScript", async ({ browser }, testInfo) => {
  const page = await (await browser.newContext({ javaScriptEnabled: false })).newPage();
  await createAccount(page, testInfo, "Plain Keeper");

  await openExport(page);
  const files = await filesIn(await exportWithPassword(page));

  expect([...files.keys()].sort()).toEqual([...EXPORT_FILES].sort());
  await page.context().close();
});

test("S9 connected apps and billing are in the export", async ({ page, request }, testInfo) => {
  await createAccount(page, testInfo, "App Keeper");
  const app = await connectApp(page, request, { scope: "profile:read" });

  await openExport(page);
  const files = await filesIn(await exportWithPassword(page));

  const connectedFile = files.get("connected_apps.csv");
  expect(connectedFile, "connected_apps.csv is in the export").toBeDefined();
  const connected = parseCsv(connectedFile!);
  expect(connected.header).toEqual(
    expect.arrayContaining(["name", "returns_to", "scopes", "connected_at", "last_used_at"]),
  );
  expect(connected.rows, "one connected app").toHaveLength(1);
  expect.soft(connected.rows[0].name).toBe(app.name);
  expect.soft(connected.rows[0].returns_to).toBe("127.0.0.1");
  expect.soft(connected.rows[0].scopes).toBe("profile:read");

  const billingFile = files.get("billing.csv");
  expect(billingFile, "billing.csv is in the export").toBeDefined();
  const billing = parseCsv(billingFile!);
  expect(billing.header).toEqual(
    expect.arrayContaining(["plan", "status", "renews_on", "ends_on", "stripe_customer_id"]),
  );
  expect(billing.rows, "billing.csv has one row").toHaveLength(1);
  expect.soft(billing.rows[0].plan).toBe("free");
  expect.soft(billing.rows[0].status).toBe("free");
  expect.soft(billing.rows[0].stripe_customer_id).toBe("");
});

test("S8 no exported cell runs as a spreadsheet formula", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Formula Keeper");
  await page.getByRole("button", { name: "Edit profile" }).click();
  await page.getByLabel("Name").fill("=1+1");
  await page.getByLabel("Name").press("Enter");
  await expect(page.getByText("Profile saved.")).toHaveCSS("opacity", "1");
  await createToken(page, "@SUM(1,2)");

  await openExport(page);
  const files = await filesIn(await exportWithPassword(page));

  expect.soft(files.get("profile.csv")).toContain("'=1+1");
  expect.soft(files.get("api_tokens.csv")).toContain("'@SUM(1,2)");
  expect.soft(files.get("README.txt")).toMatch(/apostrophe/i);
});
