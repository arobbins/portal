import { readdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { expect } from "@playwright/test";

export const MAIL_DIR = fileURLToPath(new URL("../.mail", import.meta.url));

export type Email = {
  to: string;
  subject: string;
  text: string;
  html: string;
};

async function readEmails(): Promise<Email[]> {
  let files: string[];
  try {
    files = await readdir(MAIL_DIR);
  } catch {
    return [];
  }

  const sorted = files.filter((file) => file.endsWith(".json")).sort();
  return Promise.all(
    sorted.map(async (file) => JSON.parse(await readFile(`${MAIL_DIR}/${file}`, "utf8"))),
  );
}

export async function waitForEmails(address: string, count = 1): Promise<Email[]> {
  let emails: Email[] = [];
  await expect
    .poll(
      async () => {
        emails = (await readEmails()).filter(
          (email) => email.to.toLowerCase() === address.toLowerCase(),
        );
        return emails.length;
      },
      {
        message: `waiting for ${count} email(s) to ${address}`,
        timeout: 10_000,
      },
    )
    .toBeGreaterThanOrEqual(count);
  return emails;
}

export async function expectNoEmails(address: string, waitMs = 1_000) {
  await new Promise((resolve) => setTimeout(resolve, waitMs));
  const emails = (await readEmails()).filter(
    (email) => email.to.toLowerCase() === address.toLowerCase(),
  );
  expect(emails, `expected no emails to ${address}`).toEqual([]);
}

export function verificationLink(email: Email): string {
  const match = email.text.match(/https?:\/\/\S+\/verify-email\/[A-Za-z0-9]+/);
  if (!match) throw new Error(`No verification link in email: ${email.text}`);
  return match[0];
}

export function resetLink(email: Email): string {
  const match = email.text.match(/https?:\/\/\S+\/reset-password\/[A-Za-z0-9]+/);
  if (!match) throw new Error(`No reset link in email: ${email.text}`);
  return match[0];
}

export function confirmEmailLink(email: Email): string {
  const match = email.text.match(/https?:\/\/\S+\/confirm-email\/[A-Za-z0-9]+/);
  if (!match) throw new Error(`No confirm-email link in email: ${email.text}`);
  return match[0];
}

export async function emailsAbout(address: string, subject: string): Promise<Email[]> {
  return (await readEmails()).filter(
    (email) => email.to.toLowerCase() === address.toLowerCase() && email.subject === subject,
  );
}

export async function waitForEmailAbout(address: string, subject: string): Promise<Email> {
  await expect
    .poll(async () => (await emailsAbout(address, subject)).length, {
      message: `waiting for "${subject}" to ${address}`,
      timeout: 10_000,
    })
    .toBeGreaterThan(0);
  return (await emailsAbout(address, subject))[0];
}
