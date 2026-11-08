import { register, uniqueEmail } from "../support/flows";
import { expectNoEmails, waitForEmails } from "../support/mail";
import { expect, test } from "../support/test";
import { verifiedUser } from "../support/users";

const CHECK_EMAIL = "Check your email to confirm your registration.";

test("registering asks only for a name and email, then says to check email", async ({
  page,
}, testInfo) => {
  const email = uniqueEmail(testInfo, "new");

  await register(page, "New Tester", email);

  await expect(page).toHaveURL("/register?sent=true");
  await expect(page.getByText(CHECK_EMAIL)).toHaveCSS("opacity", "1");
  await expect(page.getByLabel("Password")).toHaveCount(0);
  const [message] = await waitForEmails(email);
  expect(message.subject).toBe("Confirm your email");
});

test("registering an email that already has an account looks the same, but emails the owner", async ({
  page,
}) => {
  await register(page, "Someone Else", verifiedUser.email);

  await expect(page).toHaveURL("/register?sent=true");
  await expect(page.getByText(CHECK_EMAIL)).toHaveCSS("opacity", "1");
  const emails = await waitForEmails(verifiedUser.email);
  expect(emails.some((email) => email.subject === "You already have an account")).toBe(true);
});

test("a bot that fills the hidden field gets the same response, but nothing happens", async ({
  page,
}, testInfo) => {
  const email = uniqueEmail(testInfo, "bot");
  await page.goto("/register");

  const response = await page.request.post("/register", {
    form: { name: "Spam Bot", email, website: "https://spam.example" },
    headers: { Origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });

  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe("/register?sent=true");
  await expectNoEmails(email);
});

test("an over-long name shows an error and keeps the entered details", async ({
  page,
}, testInfo) => {
  const email = uniqueEmail(testInfo, "long");

  await register(page, "x".repeat(101), email);

  await expect(page.getByText("Name must be 100 characters or fewer.")).toBeVisible();
  await expect(page.getByLabel("Email")).toHaveValue(email);
  await expectNoEmails(email, 500);
});
