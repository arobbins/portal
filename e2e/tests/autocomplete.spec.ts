import { register, uniqueEmail } from "../support/flows";
import { verificationLink, waitForEmails } from "../support/mail";
import { expect, test } from "../support/test";

test("the login form tells password managers which fields are which", async ({ page }) => {
  await page.goto("/login");

  await expect(page.getByLabel("Email")).toHaveAttribute("autocomplete", "email");
  await expect(page.getByLabel("Password")).toHaveAttribute("autocomplete", "current-password");
});

test("the register form tells browsers to fill in name and email", async ({ page }) => {
  await page.goto("/register");

  await expect(page.getByLabel("Name")).toHaveAttribute("autocomplete", "name");
  await expect(page.getByLabel("Email")).toHaveAttribute("autocomplete", "email");
});

test("the set-password form asks password managers for a new password", async ({
  page,
}, testInfo) => {
  const email = uniqueEmail(testInfo, "autofill");
  await register(page, "Autofill tester", email);
  await page.goto(verificationLink((await waitForEmails(email))[0]));

  await expect(page.getByLabel("New password", { exact: true })).toHaveAttribute(
    "autocomplete",
    "new-password",
  );
  await expect(page.getByLabel("Confirm new password")).toHaveAttribute(
    "autocomplete",
    "new-password",
  );
});
