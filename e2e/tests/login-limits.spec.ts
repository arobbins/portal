import { logIn, register, setPassword, skipPasskeyPrompt, uniqueEmail } from "../support/flows";
import { verificationLink, waitForEmails } from "../support/mail";
import { expect, test } from "../support/test";

const PASSWORD = "a-strong-test-password";
const TOO_MANY = "Too many attempts. Please wait a few minutes and try again.";

test.use({ allowedConsoleErrors: [/status of 429/] });

test("after 5 wrong passwords, even the right password is refused for a while", async ({
  page,
}, testInfo) => {
  const email = uniqueEmail(testInfo, "locked");
  await register(page, "Locked tester", email);
  await page.goto(verificationLink((await waitForEmails(email))[0]));
  await setPassword(page, PASSWORD);
  await skipPasskeyPrompt(page);
  await page.getByRole("button", { name: "Locked tester" }).click();
  await page.getByRole("button", { name: "Logout" }).click();

  for (let attempt = 1; attempt <= 5; attempt++) {
    await logIn(page, email, `wrong-password-${attempt}`);
    await expect(page.getByText("Invalid email or password.")).toBeVisible();
  }

  await logIn(page, email, PASSWORD);

  await expect(page).toHaveURL("/login");
  await expect(page.getByText(TOO_MANY)).toBeVisible();
});

test("failed logins on one account don't block a different account", async ({
  page,
}, testInfo) => {
  const target = uniqueEmail(testInfo, "target");
  const bystander = uniqueEmail(testInfo, "bystander");

  for (let attempt = 1; attempt <= 6; attempt++) {
    await logIn(page, target, "wrong-password");
  }
  await expect(page.getByText(TOO_MANY)).toBeVisible();

  await logIn(page, bystander, "wrong-password");

  await expect(page.getByText("Invalid email or password.")).toBeVisible();
});
