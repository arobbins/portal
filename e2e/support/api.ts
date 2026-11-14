import { expect, type APIRequestContext, type Page } from "@playwright/test";
import { ACCOUNT_PASSWORD } from "./flows";

export const TOKEN_SHAPE = /^wsk_[A-Za-z0-9]{49}$/;

export type Access =
  | "Read your profile (profile:read)"
  | "Read and update your profile (profile:write)";

export type UsedBy = "Scripts and apps (JSON API)" | "AI assistants (MCP)";

export function createTokenDialog(page: Page) {
  return page.getByRole("dialog", { name: "Create API token" });
}

export async function startCreatingToken(
  page: Page,
  name: string,
  password = ACCOUNT_PASSWORD,
  access: Access = "Read your profile (profile:read)",
  usedBy: UsedBy = "Scripts and apps (JSON API)",
) {
  await page.goto("/account/developer");
  await page.getByRole("button", { name: "Create API token" }).click();
  const dialog = createTokenDialog(page);
  await dialog.getByLabel("Name").fill(name);
  await dialog.getByLabel("Access").selectOption({ label: access });
  await dialog.getByLabel("Used by").selectOption({ label: usedBy });
  await dialog.getByLabel("Current password").fill(password);
  await dialog.getByLabel("Current password").press("Enter");
}

export async function createToken(
  page: Page,
  name = "Deploy script",
  access: Access = "Read your profile (profile:read)",
  usedBy: UsedBy = "Scripts and apps (JSON API)",
) {
  await startCreatingToken(page, name, ACCOUNT_PASSWORD, access, usedBy);
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

export function callMcp(
  request: APIRequestContext,
  token: string,
  method: string,
  params: Record<string, unknown> = {},
) {
  return request.post("/mcp", {
    headers: bearer(token),
    data: { jsonrpc: "2.0", id: 1, method, params },
  });
}
