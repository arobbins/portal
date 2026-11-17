import { logIn } from "../support/flows";
import { expect, test } from "../support/test";
import { verifiedUser } from "../support/users";

test.use({ allowedConsoleErrors: [/status of 404/] });

test.beforeEach(async ({ page }) => {
  await logIn(page, verifiedUser.email, verifiedUser.password);
  await expect(page).toHaveURL("/account");
});

test("unfinished sections say so instead of showing the profile", async ({ page }) => {
  const tabs = { Billing: ["subscriptions", "invoices", "payment-methods"], Settings: ["notifications"] };
  for (const [heading, sections] of Object.entries(tabs)) {
    await page.goto(`/account/${heading.toLowerCase()}`);

    await expect(page.getByRole("heading", { name: heading, level: 1 })).toBeVisible();
    await expect(page.getByRole("link", { name: heading, exact: true })).toHaveAttribute(
      "aria-current",
      "page",
    );
    for (const anchor of sections) {
      await expect(page.locator(`#${anchor}`)).toContainText("Coming soon.");
    }
    await expect(page.getByText(`Name: ${verifiedUser.name}`)).toHaveCount(0);
  }
});

test("the profile tab shows the profile", async ({ page }) => {
  await page.goto("/account/profile");

  await expect(page.getByText(`Name: ${verifiedUser.name}`)).toBeVisible();
});

test("an unknown account section is a 404", async ({ page }) => {
  const response = await page.goto("/account/not-a-tab");

  expect(response?.status()).toBe(404);
  await expect(page.getByRole("heading", { name: "404" })).toBeVisible();
});
