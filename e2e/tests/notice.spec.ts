import { expect, test } from "../support/test";

test("clicking a dismissible notice fades it out and removes it", async ({ page }) => {
  await page.goto("/styleguide");
  const notice = page.getByText("Info: This is a dismissible notice.");
  await expect(notice).toBeVisible();

  await expect(async () => {
    await notice.click({ timeout: 500 });
    await expect(notice).toBeHidden({ timeout: 1_000 });
  }).toPass();
});

test("plain notices can't be dismissed", async ({ page }) => {
  await page.goto("/styleguide");
  const notice = page.getByText("Info: This is a notice.", { exact: true });

  await notice.click();

  await expect(notice).toBeVisible();
});
