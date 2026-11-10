import { createAccount, logIn, uniqueEmail } from "../support/flows";
import { expect, test } from "../support/test";
import { awaitingPasswordUser, verifiedUser } from "../support/users";

test("signed-out visitors are sent from the account page to login", async ({ page }) => {
  await page.goto("/account");

  await expect(page).toHaveURL("/login");
});

test("logging in with valid details opens the account page", async ({ page }) => {
  await logIn(page, verifiedUser.email, verifiedUser.password);

  await expect(page).toHaveURL("/account");
  await expect(page.getByText(`Name: ${verifiedUser.name}`)).toBeVisible();
});

test("a wrong password shows an error and keeps the email", async ({ page }) => {
  await logIn(page, verifiedUser.email, "not-the-password");

  await expect(page).toHaveURL("/login");
  await expect(page.getByText("Invalid email or password.")).toBeVisible();
  await expect(page.getByLabel("Email")).toHaveValue(verifiedUser.email);
});

test("an account that never set a password can't log in", async ({ page }) => {
  await logIn(page, awaitingPasswordUser.email, "any-password-at-all");

  await expect(page).toHaveURL("/login");
  await expect(page.getByText("Invalid email or password.")).toBeVisible();
});

test("signed-in users are sent from the login page to their account", async ({ page }) => {
  await logIn(page, verifiedUser.email, verifiedUser.password);
  await expect(page).toHaveURL("/account");

  await page.goto("/login");

  await expect(page).toHaveURL("/account");
});

test("logging out returns to login and protects the account page again", async ({ page }) => {
  await logIn(page, verifiedUser.email, verifiedUser.password);
  await expect(page).toHaveURL("/account");

  await page.getByRole("button", { name: verifiedUser.name }).click();
  await page.getByRole("button", { name: "Logout" }).click();

  await expect(page).toHaveURL("/login");
  await page.goto("/account");
  await expect(page).toHaveURL("/login");
});

test("a login takes as long for an unknown email as for a real account", async ({
  page,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Timing Tester");
  const origin = new URL(page.url()).origin;
  const timeLogin = async (address: string) => {
    const started = performance.now();
    await page.request.post("/login", {
      form: { email: address, password: "not-the-password" },
      headers: { Origin: origin },
      maxRedirects: 0,
    });
    return performance.now() - started;
  };
  const median = (values: number[]) => values.sort((a, b) => a - b)[1];

  const known = median([await timeLogin(email), await timeLogin(email), await timeLogin(email)]);
  const unknown = median([
    await timeLogin(uniqueEmail(testInfo, "nobody-1")),
    await timeLogin(uniqueEmail(testInfo, "nobody-2")),
    await timeLogin(uniqueEmail(testInfo, "nobody-3")),
  ]);

  expect(unknown).toBeGreaterThan(known * 0.5);
});
