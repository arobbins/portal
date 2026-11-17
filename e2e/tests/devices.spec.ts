import type { Browser, Page } from "@playwright/test";
import { ACCOUNT_PASSWORD, createAccount, logIn } from "../support/flows";
import { expect, test } from "../support/test";

const FIREFOX_ON_LINUX =
  "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0";
const SAFARI_ON_IPHONE =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1";

async function signInElsewhere(browser: Browser, email: string, userAgent: string) {
  const context = await browser.newContext({ userAgent });
  const page = await context.newPage();
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/account");
  return page;
}

function devices(page: Page) {
  return page.locator("#devices");
}

async function expectSignedOut(page: Page) {
  await page.goto("/account/security");
  await expect(page).toHaveURL("/login");
}

test("another device can be signed out from the devices list", async ({
  page,
  browser,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Device Owner");
  const laptop = await signInElsewhere(browser, email, FIREFOX_ON_LINUX);

  await page.goto("/account/security");
  await expect(devices(page).getByText("This device")).toBeVisible();
  await expect(devices(page).getByText("Firefox on Linux")).toBeVisible();

  await devices(page).getByRole("button", { name: "Sign out Firefox on Linux" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Sign out device" }).click();

  await expect(page.getByText("That device has been signed out.")).toBeVisible();
  await expect(devices(page).getByText("Firefox on Linux")).toHaveCount(0);
  await expectSignedOut(laptop);
  await page.goto("/account/security");
  await expect(page).toHaveURL("/account/security");
  await laptop.context().close();
});

test("signing out other devices keeps this one signed in", async ({ page, browser }, testInfo) => {
  const email = await createAccount(page, testInfo, "Everywhere Owner");
  const laptop = await signInElsewhere(browser, email, FIREFOX_ON_LINUX);
  const phone = await signInElsewhere(browser, email, SAFARI_ON_IPHONE);

  await page.goto("/account/security");
  await expect(devices(page).getByText("Safari on iOS")).toBeVisible();
  await page.getByRole("button", { name: "Sign out other devices" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Sign out other devices" }).click();

  await expect(page.getByText("Your other devices have been signed out.", { exact: true })).toBeVisible();
  await expect(devices(page).getByRole("listitem")).toHaveCount(1);
  await expect(page.getByRole("button", { name: "Sign out other devices" })).toHaveCount(0);
  await expectSignedOut(laptop);
  await expectSignedOut(phone);
  await page.goto("/account/security");
  await expect(page).toHaveURL("/account/security");
  await laptop.context().close();
  await phone.context().close();
});

test("signing out from the menu removes the device from the list", async ({
  page,
  browser,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Leaving Owner");
  const phone = await signInElsewhere(browser, email, SAFARI_ON_IPHONE);

  await phone.getByRole("button", { name: "Leaving Owner" }).click();
  await phone.getByRole("button", { name: "Sign out" }).click();
  await expect(phone).toHaveURL("/login");

  await page.goto("/account/security");
  await expect(devices(page).getByText("Safari on iOS")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Sign out other devices" })).toHaveCount(0);
  await phone.context().close();
});

test("recent activity lists sign-ins and account changes, newest first", async ({
  page,
  browser,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Activity Owner");
  const phone = await signInElsewhere(browser, email, SAFARI_ON_IPHONE);
  await phone.context().close();

  await page.goto("/account/security");
  const activity = page.locator("#activity").getByRole("listitem");

  await expect(activity.first()).toContainText("Signed in with a password");
  await expect(activity.first()).toContainText("Safari on iOS");
  await expect(activity.filter({ hasText: "Signed in after setting a password" })).toHaveCount(1);
  await expect(activity.filter({ hasText: "Account created" })).toHaveCount(1);
});

test("the devices list works without JavaScript", async ({ browser }, testInfo) => {
  const owner = await (await browser.newContext()).newPage();
  const email = await createAccount(owner, testInfo, "Plain Owner");
  await owner.context().close();
  const page = await (await browser.newContext({ javaScriptEnabled: false })).newPage();
  await logIn(page, email, ACCOUNT_PASSWORD);
  const laptop = await signInElsewhere(browser, email, FIREFOX_ON_LINUX);

  await page.goto("/account/security");
  await devices(page).getByRole("button", { name: "Sign out Firefox on Linux" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Sign out device" }).click();

  await expect(page).toHaveURL(/\/account\/security/);
  await expect(devices(page).getByText("Firefox on Linux")).toHaveCount(0);
  await expectSignedOut(laptop);
  await laptop.context().close();
  await page.context().close();
});
