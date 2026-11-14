import { expect, type APIRequestContext, type Page } from "@playwright/test";
import { ACCOUNT_PASSWORD } from "./flows";

export const TOKEN_SHAPE = /^wsk_[A-Za-z0-9]{49}$/;

export type Access =
  | "Read your profile (profile:read)"
  | "Read and update your profile (profile:write)";

export function createTokenDialog(page: Page) {
  return page.getByRole("dialog", { name: "Create API token" });
}

export async function startCreatingToken(
  page: Page,
  name: string,
  password = ACCOUNT_PASSWORD,
  access: Access = "Read your profile (profile:read)",
) {
  await page.goto("/account/developer");
  await page.getByRole("button", { name: "Create API token" }).click();
  const dialog = createTokenDialog(page);
  await dialog.getByLabel("Name").fill(name);
  await dialog.getByLabel("Access").selectOption({ label: access });
  await dialog.getByLabel("Current password").fill(password);
  await dialog.getByLabel("Current password").press("Enter");
}

export async function createToken(
  page: Page,
  name = "Deploy script",
  access: Access = "Read your profile (profile:read)",
) {
  await startCreatingToken(page, name, ACCOUNT_PASSWORD, access);
  const field = createTokenDialog(page).getByLabel("API token");
  await expect(field).toHaveValue(TOKEN_SHAPE);
  return field.inputValue();
}

export function bearer(token: string) {
  return { Authorization: `Bearer ${token}` };
}

export function getMe(request: APIRequestContext, authorization?: string) {
  return request.get("/api/v1/me", {
    headers: authorization ? { Authorization: authorization } : {},
  });
}
