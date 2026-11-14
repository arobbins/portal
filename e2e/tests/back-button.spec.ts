import { expect, test } from "../support/test";

const FORMS = [
  { path: "/login", button: "Sign in", exact: true },
  { path: "/register", button: "Register account", exact: false },
];

for (const { path, button, exact } of FORMS) {
  test(`${path}: a submit button made busy is reset when the browser restores the page from its back/forward cache`, async ({
    page,
  }) => {
    await page.goto(path);
    const submit = page.getByRole("button", { name: button, exact });
    await page.evaluate(() =>
      document
        .querySelector("form.a-form")
        ?.addEventListener("submit", (event) => event.preventDefault()),
    );

    await expect(async () => {
      await page.locator("form.a-form").first().dispatchEvent("submit");
      await expect(submit).toBeDisabled({ timeout: 200 });
    }).toPass();

    const reloaded = page.waitForEvent("load");
    await page.evaluate(() =>
      window.dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true })),
    );
    await reloaded;

    await expect(submit).toBeEnabled();
    await expect(submit).not.toHaveClass(/a-button--is-busy/);
  });
}
