import type { Page } from "@playwright/test";
import { ACCOUNT_PASSWORD, createAccount, logIn, logOut } from "../support/flows";
import { waitForEmailAbout } from "../support/mail";
import { totp, turnOnTwoFactor } from "../support/totp";
import { expect, test } from "../support/test";

const THIRTY_SECONDS = 30_000;

async function signInWithPassword(page: Page, email: string) {
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/login/two-factor");
}

test("two-factor authentication asks for a code after the password", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Code Keeper");
  const { secret } = await turnOnTwoFactor(page);
  await waitForEmailAbout(email, "Two-factor authentication is on");
  await logOut(page, "Code Keeper");

  await signInWithPassword(page, email);
  await page.getByLabel("Code").fill("000000");
  await page.getByLabel("Code").press("Enter");
  await expect(page.getByText("That code didn't work.")).toBeVisible();
  await page.getByLabel("Code").fill(totp(secret, Date.now() + THIRTY_SECONDS));
  await page.getByLabel("Code").press("Enter");

  await expect(page).toHaveURL("/account");
});

test("a recovery code works once instead of the app", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Phone Loser");
  const { codes } = await turnOnTwoFactor(page);
  await logOut(page, "Phone Loser");

  await signInWithPassword(page, email);
  await page.getByLabel("Code").fill(codes[0].toUpperCase());
  await page.getByLabel("Code").press("Enter");
  await expect(page).toHaveURL("/account");
  await page.goto("/account/settings/authentication");
  await expect(page.locator("#two-factor")).toContainText("9 of 10 recovery codes left.");
  await logOut(page, "Phone Loser");

  await signInWithPassword(page, email);
  await page.getByLabel("Code").fill(codes[0]);
  await page.getByLabel("Code").press("Enter");
  await expect(page.getByText("That code didn't work.")).toBeVisible();
});

test("turning two-factor off needs the password and is emailed", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Code Dropper");
  await turnOnTwoFactor(page);

  await page.locator("#two-factor").getByRole("button", { name: "Turn off" }).click();
  const dialog = page.getByRole("dialog", { name: "Turn off two-factor authentication" });
  await dialog.getByLabel("Current password").fill(ACCOUNT_PASSWORD);
  await dialog.getByLabel("Current password").press("Enter");

  await expect(page.getByText("Two-factor authentication is off.")).toBeVisible();
  await expect(page.locator("#two-factor").getByRole("button", { name: "Set up" })).toBeVisible();
  await waitForEmailAbout(email, "Two-factor authentication is off");
  await logOut(page, "Code Dropper");
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page).toHaveURL("/account");
});

test("the second step can't be skipped by going to the account", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Step Skipper");
  await turnOnTwoFactor(page);
  await logOut(page, "Step Skipper");

  await signInWithPassword(page, email);
  await page.goto("/account");

  await expect(page).toHaveURL("/login");
});

test("the authentication page says what is set up", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Status Reader");
  await page.goto("/account/settings/authentication");

  await expect(page.locator("#password")).toContainText(/^Set |Set [A-Z][a-z]{2} \d/);
  await expect(page.locator("#passkeys")).toContainText("None yet");
  await expect(page.locator("#two-factor")).toContainText("Off");
});
