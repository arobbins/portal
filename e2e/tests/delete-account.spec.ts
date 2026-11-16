import type { Page } from "@playwright/test";
import { ACCOUNT_PASSWORD, createAccount, logIn } from "../support/flows";
import { waitForEmails } from "../support/mail";
import {
  addPasskeyToAccount,
  addVirtualAuthenticator,
  withoutPasskeyAutofill,
} from "../support/passkeys";
import { expect, test } from "../support/test";

const DELETED = "Your account has been deleted. You've been signed out everywhere.";

function dialog(page: Page) {
  return page.getByRole("dialog", { name: "Delete account" });
}

async function openDialog(page: Page) {
  await page.goto("/account/profile");
  await page.getByRole("button", { name: "Delete account" }).click();
  await expect(dialog(page)).toBeVisible();
}

test("a user deletes their account and is signed out everywhere", async ({
  browser,
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Leaver");
  const other = await browser.newPage();
  await logIn(other, email, ACCOUNT_PASSWORD);
  await expect(other).toHaveURL("/account");

  await openDialog(page);
  await dialog(page).getByLabel("Type DELETE to confirm").fill("DELETE");
  await dialog(page).getByLabel("Current password").fill(ACCOUNT_PASSWORD);
  await dialog(page).getByLabel("Current password").press("Enter");

  await expect(page).toHaveURL("/login");
  await expect(page.getByText(DELETED)).toHaveCSS("opacity", "1");
  const emails = await waitForEmails(email, 2);
  expect(emails.map((sent) => sent.subject)).toContain("Your account was deleted");

  await other.goto("/account/profile");
  await expect(other).toHaveURL("/login");
  await other.close();
  await logIn(page, email, ACCOUNT_PASSWORD);
  await expect(page.getByText("Invalid email or password.")).toBeVisible();
});

test("the confirmation word has to be typed exactly", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Hesitant");

  await openDialog(page);
  await dialog(page).getByLabel("Type DELETE to confirm").fill("delete");
  await dialog(page).getByLabel("Current password").fill(ACCOUNT_PASSWORD);
  await dialog(page).getByLabel("Current password").press("Enter");

  await expect(dialog(page).getByRole("alert")).toHaveText(
    "Type DELETE in capital letters to confirm.",
  );
  await expect(dialog(page).getByLabel("Type DELETE to confirm")).toHaveAttribute(
    "aria-invalid",
    "true",
  );
  await page.goto("/account/profile");
  await expect(page.locator("#email")).toContainText(email);
});

test("a wrong password keeps the account", async ({ page }, testInfo) => {
  const email = await createAccount(page, testInfo, "Wrong Leaver");

  await openDialog(page);
  await dialog(page).getByLabel("Type DELETE to confirm").fill("DELETE");
  await dialog(page).getByLabel("Current password").fill("not-my-password");
  await dialog(page).getByLabel("Current password").press("Enter");

  await expect(dialog(page).getByRole("alert")).toHaveText("Your current password is incorrect.");
  await expect(dialog(page).getByLabel("Current password")).toHaveAttribute("aria-invalid", "true");
  await page.goto("/account/profile");
  await expect(page.locator("#email")).toContainText(email);
});

test.describe("without JavaScript", () => {
  test.use({ javaScriptEnabled: false });

  async function submitDeletion(page: Page, confirmation: string) {
    await page
      .locator('form[action="/account/user/delete"]')
      .evaluate((form: HTMLFormElement, values) => {
        form.elements.namedItem("confirmation").value = values.confirmation;
        form.elements.namedItem("current_password").value = values.password;
        form.submit();
      }, { confirmation, password: ACCOUNT_PASSWORD });
  }

  test("the form still deletes the account and shows why on failure", async ({
    page,
  }, testInfo) => {
    await createAccount(page, testInfo, "No Script");
    await page.goto("/account/profile");

    await submitDeletion(page, "delete");
    await expect(page.locator(".a-toaster--is-open")).toHaveText(
      "Type DELETE in capital letters to confirm.",
    );
    for (const other of ["#edit-profile", "#change-email"]) {
      await expect(page.locator(`${other} .a-form-error`)).toHaveClass(/a-form-error--is-hidden/);
    }

    await submitDeletion(page, "DELETE");
    await expect(page).toHaveURL("/login");
    await expect(page.locator(".a-toaster--is-open")).toHaveText(DELETED);
  });
});

test.describe("with a passkey", () => {
  test.skip(
    ({ browserName }) => browserName !== "chromium",
    "virtual authenticators are Chromium-only",
  );

  test("a passkey confirms deleting the account", async ({ page }, testInfo) => {
    await withoutPasskeyAutofill(page);
    await addVirtualAuthenticator(page);
    await createAccount(page, testInfo, "Passkey Leaver");
    await addPasskeyToAccount(page);

    await openDialog(page);
    await expect(dialog(page).getByLabel("Current password")).toBeHidden();
    await dialog(page).getByLabel("Type DELETE to confirm").fill("DELETE");
    await dialog(page).getByLabel("Type DELETE to confirm").press("Enter");

    await expect(page).toHaveURL("/login");
    await expect(page.getByText(DELETED)).toHaveCSS("opacity", "1");
  });
});
