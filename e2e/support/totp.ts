import { createHmac } from "node:crypto";
import { expect, type Page } from "@playwright/test";
import { ACCOUNT_PASSWORD } from "./flows";

const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

function base32(secret: string): Buffer {
  let bits = "";
  for (const char of secret.replace(/=+$/, "")) {
    bits += ALPHABET.indexOf(char).toString(2).padStart(5, "0");
  }
  const bytes = bits.match(/.{8}/g) ?? [];
  return Buffer.from(bytes.map((byte) => parseInt(byte, 2)));
}

export function totp(secret: string, at = Date.now()): string {
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(at / 1000 / 30)));
  const hash = createHmac("sha1", base32(secret)).update(counter).digest();
  const offset = hash[hash.length - 1] & 0xf;
  const value = (hash.readUInt32BE(offset) & 0x7fffffff) % 1_000_000;
  return value.toString().padStart(6, "0");
}

export async function turnOnTwoFactor(page: Page): Promise<{ secret: string; codes: string[] }> {
  await page.goto("/account/settings/authentication");
  const card = page.locator("#two-factor");
  await card.getByRole("button", { name: "Set up" }).click();
  const dialog = page.getByRole("dialog", { name: "Set up two-factor authentication" });
  await expect(dialog.getByRole("img", { name: "QR code for your authenticator app" })).toBeVisible();
  const secret = await dialog.getByLabel("Or enter this setup key").inputValue();
  await dialog.getByLabel("Code from the app").fill(totp(secret));
  await dialog.getByLabel("Current password").fill(ACCOUNT_PASSWORD);
  await dialog.getByLabel("Current password").press("Enter");

  await expect(card).toContainText("On since");
  const codes = (await card.locator("pre").innerText()).split("\n").filter(Boolean);
  expect(codes).toHaveLength(10);
  return { secret, codes };
}
