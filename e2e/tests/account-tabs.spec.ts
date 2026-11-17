import { logIn } from "../support/flows";
import { expect, test } from "../support/test";
import { verifiedUser } from "../support/users";

test.use({ allowedConsoleErrors: [/status of 404/] });

test.beforeEach(async ({ page }) => {
  await logIn(page, verifiedUser.email, verifiedUser.password);
  await expect(page).toHaveURL("/account");
});

test("unfinished pages say so instead of showing the profile", async ({
  page,
}) => {
  const pages = [
    {
      path: "/account/billing/plan",
      heading: "Billing",
      page: "Plan and payment",
      cards: ["subscriptions", "payment-methods"],
    },
    {
      path: "/account/billing/invoices",
      heading: "Billing",
      page: "Invoices",
      cards: ["invoices"],
    },
    {
      path: "/account/settings",
      heading: "Settings",
      page: "Settings",
      cards: ["notifications"],
    },
  ];
  for (const expected of pages) {
    await page.goto(expected.path);

    await expect(
      page.getByRole("heading", { name: expected.heading, level: 1 }),
    ).toBeVisible();
    await expect(
      page
        .getByRole("navigation", { name: "Account" })
        .getByRole("link", { name: expected.page, exact: true }),
    ).toHaveAttribute("aria-current", "page");
    for (const anchor of expected.cards) {
      await expect(page.locator(`#${anchor}`)).toContainText("Coming soon.");
    }
    await expect(page.getByText(`Name: ${verifiedUser.name}`)).toHaveCount(0);
  }
});

test("a section with several pages opens its first page and shows them as tabs", async ({
  page,
}) => {
  await page.goto("/account/billing");

  await expect(page).toHaveURL("/account/billing/plan");
  const tabs = page.getByRole("navigation", { name: "Billing" });
  await expect(
    tabs.getByRole("link", { name: "Plan and payment" }),
  ).toHaveAttribute("aria-current", "page");
  await tabs.getByRole("link", { name: "Invoices" }).click();
  await expect(page).toHaveURL("/account/billing/invoices");
  await expect(page).toHaveTitle(/^Invoices/);
});

test("a single-page section has no tabs", async ({ page }) => {
  await page.goto("/account/settings");

  await expect(
    page.getByRole("heading", { name: "Settings", level: 1 }),
  ).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Settings" })).toHaveCount(
    0,
  );
});

test("the profile tab shows the profile", async ({ page }) => {
  await page.goto("/account/profile");

  await expect(page.getByText(`Name: ${verifiedUser.name}`)).toBeVisible();
});

test("an unknown account section is a 404", async ({ page }) => {
  for (const path of [
    "/account/not-a-tab",
    "/account/security/not-a-page",
    "/account/profile/extra",
  ]) {
    const response = await page.goto(path);
    expect(response?.status(), path).toBe(404);
  }
  await expect(page.getByRole("heading", { name: "404" })).toBeVisible();
});
