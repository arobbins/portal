import { existsSync, readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const SPECS_DIR = fileURLToPath(new URL("../../specs", import.meta.url));
const SCENARIOS_DIR = fileURLToPath(new URL("../scenarios", import.meta.url));

function status(spec: string): string | undefined {
  const text = readFileSync(`${SPECS_DIR}/${spec}`, "utf8");
  return text.match(/^---\n[\s\S]*?^status:\s*(\w+)\s*$[\s\S]*?^---$/m)?.[1];
}

export function approvedScenarios(): string[] {
  if (!existsSync(SPECS_DIR)) return [];
  return readdirSync(SPECS_DIR)
    .filter((spec) => spec.endsWith(".md") && status(spec) === "approved")
    .map((spec) => `scenarios/${spec.replace(/\.md$/, ".spec.ts")}`)
    .filter((scenario) => existsSync(`${SCENARIOS_DIR}/../${scenario}`));
}
