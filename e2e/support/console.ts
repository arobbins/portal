import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { testDatabaseUrl } from "./environment";

const SERVER_BINARY = fileURLToPath(new URL("../../target/e2e/debug/webserver", import.meta.url));

export function grantAdmin(email: string) {
  execFileSync(SERVER_BINARY, ["admin", "grant", email], {
    env: { ...process.env, DATABASE_URL: testDatabaseUrl() },
    stdio: "pipe",
  });
}
