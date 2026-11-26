import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPECS = ROOT / "e2e" / "tests"
SCENARIOS = ROOT / "e2e" / "scenarios"
FEATURE_SPECS = ROOT / "specs"

AREAS = {
    "src/auth/": ["auth", "register", "verify-email", "reset-password", "change-password",
                  "change-email", "delete-account", "login-limits", "passkey-setup", "back-button", "autocomplete", "devices", "admin"],
    "src/passkeys/": ["passkeys", "passkey-setup", "change-email", "delete-account"],
    "src/devices/": ["devices", "auth", "change-password", "change-email", "notifications"],
    "src/activity/": ["devices", "admin"],
    "src/admin/": ["admin", "api-feedback"],
    "src/profile/": ["profile", "change-email", "delete-account", "api-profile"],
    "src/api_tokens/": ["api-tokens", "api-mcp", "api-profile"],
    "src/api/": ["api-docs", "api-mcp", "api-profile", "api-tokens", "api-feedback"],
    "src/mcp": ["api-mcp", "api-feedback"],
    "src/feedback": ["api-feedback"],
    "src/data_export": ["profile", "delete-account"],
    "src/account/": ["account-tabs", "security", "change-password", "mobile", "settings"],
    "src/user/": ["profile", "change-email", "delete-account", "register"],
    "src/pages/": ["home", "styleguide", "components", "not-found", "webtransport"],
    "src/components/": ["components", "styleguide", "form-inputs", "toaster", "notice", "state-machines"],
    "src/layout": ["mobile", "theme", "home"],
    "src/theme": ["theme", "state-machines"],
    "src/flash": ["flash", "toaster"],
    "src/rate_limit": ["login-limits"],
    "src/preferences/": ["settings", "notifications"],
    "src/two_factor/": ["two-factor", "auth", "reset-password"],
    "src/oauth/": ["connected-apps"],
    "src/time_zone": ["settings", "devices", "admin"],
    "assets/css/": ["styleguide", "components", "mobile", "theme", "state-machines"],
    "assets/js/passkeys": ["passkeys", "passkey-setup", "change-email", "delete-account", "connected-apps"],
}
EVERYTHING = ("src/web", "src/server", "src/app", "src/env", "src/action", "src/main",
              "migrations/", "Cargo", "build.rs", "e2e/support/", "e2e/playwright", "assets/js/")
IGNORED = (".claude/", "scripts/", "README", ".sqlx/", "e2e/screenshots/", "src/sql_console", "src/seed")


def unmapped_specs():
    mapped = {spec for specs in AREAS.values() for spec in specs}
    return sorted(path.name for path in SPECS.glob("*.spec.ts")
                  if path.name.removesuffix(".spec.ts") not in mapped)


def spec_status(name):
    path = FEATURE_SPECS / f"{name}.md"
    if not path.exists():
        return None
    front = re.match(r"---\n(.*?)\n---\n", path.read_text(), re.S)
    status = re.search(r"^status:\s*(\w+)\s*$", front.group(1), re.M) if front else None
    return status.group(1) if status else None


def approved_scenarios():
    return sorted(path.name.removesuffix(".spec.ts") for path in SCENARIOS.glob("*.spec.ts")
                  if spec_status(path.name.removesuffix(".spec.ts")) == "approved")


def scenario_for(path):
    if path.startswith("specs/") and path.endswith(".md"):
        return Path(path).name.removesuffix(".md")
    if path.startswith("e2e/scenarios/") and path.endswith(".spec.ts"):
        return Path(path).name.removesuffix(".spec.ts")
    return None


def changed_files():
    tracked = subprocess.run(["git", "diff", "--name-only", "HEAD"], cwd=ROOT,
                             capture_output=True, text=True, check=True).stdout.split()
    untracked = subprocess.run(["git", "ls-files", "--others", "--exclude-standard"], cwd=ROOT,
                               capture_output=True, text=True, check=True).stdout.split()
    return tracked + untracked


def specs_for(path):
    if scenario_for(path):
        return set()
    if path.startswith("e2e/tests/") and path.endswith(".spec.ts"):
        return {Path(path).name.removesuffix(".spec.ts")}
    if path.startswith(IGNORED) or path.endswith(".rs") and "/tests" in path or path.endswith(("tests.rs", "testing.rs")):
        return set()
    if path.startswith(EVERYTHING):
        return None
    matches = [specs for prefix, specs in AREAS.items() if path.startswith(prefix)]
    if not matches and path.startswith(("src/", "assets/")):
        return None
    return {spec for specs in matches for spec in specs}


def main():
    wanted = set()
    scenarios = set()
    for path in changed_files():
        scenarios.add(scenario_for(path))
        specs = specs_for(path)
        if specs is None:
            print("ALL")
            return
        wanted |= specs
    existing = sorted(spec for spec in wanted if (SPECS / f"{spec}.spec.ts").exists())
    approved = sorted(scenarios & set(approved_scenarios()))
    print(" ".join([f"tests/{spec}.spec.ts" for spec in existing]
                   + [f"scenarios/{name}.spec.ts" for name in approved]))


if __name__ == "__main__":
    sys.exit(main())
