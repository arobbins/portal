import { readFileSync } from "node:fs";
import { defineConfig, devices } from "@playwright/test";
import { MAIL_DIR } from "./support/mail";

const port = 3100;
const baseURL = `http://localhost:${port}`;

function testDatabaseUrl(): string {
  if (process.env.TEST_DATABASE_URL) return process.env.TEST_DATABASE_URL;

  const line = readFileSync(new URL("../.env", import.meta.url), "utf8")
    .split("\n")
    .find((entry) => entry.startsWith("DATABASE_URL="));
  if (!line) throw new Error("Set TEST_DATABASE_URL or DATABASE_URL in ../.env");

  const devUrl = line.slice("DATABASE_URL=".length).trim().replace(/^"|"$/g, "");
  return `${devUrl}_test`;
}

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
  webServer: {
    command:
      `rm -rf "${MAIL_DIR}" && sqlx database create && sqlx migrate run && topcoat asset bundle && ./target/e2e/debug/webserver seed && exec ./target/e2e/debug/webserver`,
    cwd: "..",
    url: baseURL,
    reuseExistingServer: false,
    timeout: 300_000,
    env: {
      ...process.env,
      DATABASE_URL: testDatabaseUrl(),
      HOST: "127.0.0.1",
      PORT: String(port),
      APP_BASE_URL: baseURL,
      MAIL_TRANSPORT: "file",
      MAIL_DIR,
      RATE_LIMITS: "relaxed",
      RUST_LOG: "warn",
      CARGO_TARGET_DIR: "target/e2e",
    },
  },
});
