import type { Page, TestInfo } from "@playwright/test";
import { logIn, register, setPassword, uniqueEmail } from "../support/flows";
import { verificationLink, waitForEmails } from "../support/mail";
import { expect, test } from "../support/test";

const PASSWORD = "a-strong-test-password";

test.use({ allowedConsoleErrors: [/status of 400/] });

async function registerAndGetLink(page: Page, testInfo: TestInfo, label: string) {
  const email = uniqueEmail(testInfo, label);
  await register(page, `${label} tester`, email);
  const [message] = await waitForEmails(email);
  return { email, link: verificationLink(message) };
}

test("a new user can register, set a password from the email link, and log in", async ({
  page,
}, testInfo) => {
  const { email, link } = await registerAndGetLink(page, testInfo, "Journey");

  await page.goto(link);
  await expect(page.getByRole("heading", { name: "Set your password" })).toBeVisible();
  await setPassword(page, PASSWORD);

  await expect(page).toHaveURL("/account");
  await expect(page.getByText("Your email is confirmed. Welcome!")).toHaveCSS("opacity", "1");
  await expect(page.getByText("Name: Journey tester")).toBeVisible();

  await page.getByRole("button", { name: "Journey tester" }).click();
  await page.getByRole("button", { name: "Logout" }).click();
  await logIn(page, email, PASSWORD);

  await expect(page).toHaveURL("/account");
});

test("opening the link does not use it up, so email scanners can't break it", async ({
  page,
}, testInfo) => {
  const { link } = await registerAndGetLink(page, testInfo, "Scanner");

  await page.goto(link);
  await page.goto(link);

  await expect(page.getByRole("heading", { name: "Set your password" })).toBeVisible();
});

test("the link stops working once the password is set", async ({ page }, testInfo) => {
  const { link } = await registerAndGetLink(page, testInfo, "Once");
  await page.goto(link);
  await setPassword(page, PASSWORD);
  await expect(page).toHaveURL("/account");

  const response = await page.goto(link);

  expect(response?.status()).toBe(400);
  await expect(page.getByRole("heading", { name: "This link has expired" })).toBeVisible();
});

test("an unknown link shows a friendly error page", async ({ page }) => {
  const response = await page.goto("/user/verify/not-a-real-token");

  expect(response?.status()).toBe(400);
  await expect(page.getByRole("heading", { name: "This link has expired" })).toBeVisible();
  await expect(page.getByRole("link", { name: "register again" })).toHaveAttribute(
    "href",
    "/register",
  );
});

test("a too-short password is rejected and the link keeps working", async ({ page }, testInfo) => {
  const { link } = await registerAndGetLink(page, testInfo, "Short");
  await page.goto(link);

  await setPassword(page, "short");

  await expect(page.getByText("Password must be at least 8 characters.")).toBeVisible();
  await setPassword(page, PASSWORD);
  await expect(page).toHaveURL("/account");
});

test("mismatched passwords are rejected", async ({ page }, testInfo) => {
  const { link } = await registerAndGetLink(page, testInfo, "Mismatch");
  await page.goto(link);

  await setPassword(page, PASSWORD, "a-different-password");

  await expect(page.getByText("Passwords do not match.")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Set your password" })).toBeVisible();
});

test("registering again sends a new link that replaces the old one", async ({ page }, testInfo) => {
  const email = uniqueEmail(testInfo, "again");
  await register(page, "Again tester", email);
  const firstLink = verificationLink((await waitForEmails(email))[0]);

  await register(page, "Again tester", email);
  const emails = await waitForEmails(email, 2);
  const secondLink = verificationLink(emails[emails.length - 1]);
  expect(secondLink).not.toBe(firstLink);

  const oldResponse = await page.goto(firstLink);
  expect(oldResponse?.status()).toBe(400);

  await page.goto(secondLink);
  await expect(page.getByRole("heading", { name: "Set your password" })).toBeVisible();
});
