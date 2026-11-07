import { expect, test } from "../support/test";

test("registering a new account asks the user to confirm their email", async ({
  page,
}, testInfo) => {
  const email = `new-${testInfo.project.name}-${Date.now()}@example.test`;

  await page.goto("/register");
  await page.getByLabel("Name").fill("New Tester");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("New password", { exact: true }).fill("a-strong-test-password");
  await page.getByLabel("Confirm new password").fill("a-strong-test-password");
  await page.getByRole("button", { name: "Register account" }).click();

  await expect(
    page.getByText("Account created! Check your email to confirm your account."),
  ).toBeVisible();
});

test("mismatched passwords show an error and keep the entered details", async ({ page }) => {
  await page.goto("/register");
  await page.getByLabel("Name").fill("New Tester");
  await page.getByLabel("Email").fill("mismatch@example.test");
  await page.getByLabel("New password", { exact: true }).fill("first-password");
  await page.getByLabel("Confirm new password").fill("second-password");
  await page.getByRole("button", { name: "Register account" }).click();

  await expect(page.getByText("Passwords do not match.")).toBeVisible();
  await expect(page.getByLabel("Name")).toHaveValue("New Tester");
  await expect(page.getByLabel("Email")).toHaveValue("mismatch@example.test");
});
