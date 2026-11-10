import { expect, test } from "../support/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/styleguide");
});

test("a select starts on its placeholder and picks an option by label", async ({ page }) => {
  const plan = page.getByLabel("Plan");
  await expect(plan).toHaveValue("");

  await plan.selectOption({ label: "Team" });

  await expect(plan).toHaveValue("team");
});

test("radios form one group where only one can be chosen", async ({ page }) => {
  const group = page.getByRole("group", { name: "Billing period" });
  await expect(group.getByRole("radio", { name: "Monthly" })).toBeChecked();

  await group.getByText("Yearly (save 20%)").click();

  await expect(group.getByRole("radio", { name: "Yearly (save 20%)" })).toBeChecked();
  await expect(group.getByRole("radio", { name: "Monthly" })).not.toBeChecked();
});

test("checkboxes toggle independently, including by clicking the label", async ({ page }) => {
  const group = page.getByRole("group", { name: "Email me about" });
  await expect(group.getByRole("checkbox", { name: "Security alerts" })).toBeChecked();

  await group.getByText("Product updates").click();
  await group.getByText("Security alerts").click();

  await expect(group.getByRole("checkbox", { name: "Product updates" })).toBeChecked();
  await expect(group.getByRole("checkbox", { name: "Security alerts" })).not.toBeChecked();
  await expect(
    page.getByRole("checkbox", { name: "I agree to the terms of service" }),
  ).not.toBeChecked();
});

test("a textarea accepts multiple lines", async ({ page }) => {
  const message = page.getByLabel("Message");

  await message.fill("First line\nSecond line");

  await expect(message).toHaveValue("First line\nSecond line");
});

test("a form error is announced and points back at the fields it's about", async ({ page }) => {
  await page.goto("/login");
  await page.getByLabel("Email").fill("someone@example.test");
  await page.getByLabel("Password").fill("not-the-password");
  await page.getByRole("button", { name: "Login" }).click();

  await expect(page.getByRole("alert")).toHaveText("Invalid email or password.");
  for (const field of [page.getByLabel("Email"), page.getByLabel("Password")]) {
    await expect(field).toHaveAttribute("aria-invalid", "true");
    await expect(field).toHaveAccessibleDescription("Invalid email or password.");
  }
});

test("a valid form has no error and no invalid fields", async ({ page }) => {
  await page.goto("/login");

  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.getByLabel("Email")).not.toHaveAttribute("aria-invalid", "true");
});

test("side-by-side fields stack on a phone", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const forms = page.locator(".styleguide__section", {
    has: page.getByRole("heading", { name: "Forms" }),
  });
  const password = await forms.getByLabel("New password").boundingBox();
  const confirmation = await forms.getByLabel("Confirm password").boundingBox();

  expect(confirmation!.y).toBeGreaterThan(password!.y);
  expect(confirmation!.x).toBe(password!.x);
});
