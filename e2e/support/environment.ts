import { execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { basename } from "node:path";

const MAIN_PORT = 3100;
const WORKTREE_PORTS = 800;
const ENV_FILE = new URL("../../.env", import.meta.url);

function git(args: string): string {
  return execSync(`git ${args}`, { cwd: new URL(".", import.meta.url), encoding: "utf8" }).trim();
}

export function worktreeSlug(): string | null {
  const gitDir = git("rev-parse --absolute-git-dir");
  const commonDir = git("rev-parse --path-format=absolute --git-common-dir");
  if (gitDir === commonDir) return null;
  return basename(git("rev-parse --show-toplevel"))
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

export function e2ePort(): number {
  if (process.env.E2E_PORT) return Number(process.env.E2E_PORT);
  const slug = worktreeSlug();
  if (!slug) return MAIN_PORT;
  const hash = createHash("sha256").update(slug).digest().readUInt32BE(0);
  return MAIN_PORT + 1 + (hash % WORKTREE_PORTS);
}

export function testDatabaseUrl(): string {
  if (process.env.TEST_DATABASE_URL) return process.env.TEST_DATABASE_URL;

  const line = readFileSync(ENV_FILE, "utf8")
    .split("\n")
    .find((entry) => entry.startsWith("DATABASE_URL="));
  if (!line) throw new Error("Set TEST_DATABASE_URL, or DATABASE_URL in the project's env file");

  const devUrl = line.slice("DATABASE_URL=".length).trim().replace(/^"|"$/g, "");
  const [base, query] = devUrl.split("?", 2);
  const slug = worktreeSlug();
  const name = slug ? `${base}_${slug}_test` : `${base}_test`;
  return query ? `${name}?${query}` : name;
}
