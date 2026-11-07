import { readFileSync } from "node:fs";
import { defineConfig, devices } from "@playwright/test";

const port = 3100;
const baseURL = `http://127.0.0.1:${port}`;

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
      "topcoat asset bundle && ./target/debug/webserver seed && exec ./target/debug/webserver",
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
      MAIL_TRANSPORT: "log",
      RUST_LOG: "warn",
    },
  },
});
