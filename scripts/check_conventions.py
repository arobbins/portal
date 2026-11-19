import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROUTE_PREFIXES = (
    "account", "login", "logout", "register", "reset-password", "verify-email",
    "passkey-setup", "passkeys", "docs", "api", "styleguide", "about", "theme",
)
ROUTE_LITERAL = re.compile(r'"/(?:' + "|".join(map(re.escape, ROUTE_PREFIXES)) + r')(?:[/#?"{])')
LIMIT_LITERAL = re.compile(r'"[^"\n]*\b\d+ ?(?:characters|KB|MB|hours?|minutes?|days?|seconds?)\b[^"\n]*"')
LIMIT_TEST = re.compile(r"fn \w*(?:match(?:es)?_the_limits?|matches_the_lifetime)\w*\(")
PATH_ALLOWED = {"src/auth/links.rs", "src/api/catalog.rs", "src/routes.rs"}
LIMIT_ALLOWED_PREFIXES = ("src/pages/styleguide",)
SQL = re.compile(r'"\s*(SELECT|INSERT|UPDATE|DELETE|WITH)\b')

problems = []
for path in sorted((ROOT / "src").rglob("*.rs")):
    relative = path.relative_to(ROOT).as_posix()
    if path.name in ("tests.rs", "testing.rs"):
        continue
    text = path.read_text()
    code = text.split("#[cfg(test)]\nmod tests", 1)[0]
    has_limit_test = bool(LIMIT_TEST.search(text))
    in_attribute = False
    for number, line in enumerate(code.splitlines(), start=1):
        stripped = line.strip()
        if re.match(r"#\[(page|route|layout)\b", stripped):
            in_attribute = not stripped.endswith(")]")
            continue
        if in_attribute:
            in_attribute = not stripped.endswith(")]")
            continue
        if relative not in PATH_ALLOWED and ROUTE_LITERAL.search(line) and "const PATH:" not in line:
            problems.append(f"{relative}:{number}: hard-coded app path; use href!(handler) or AccountTab/AccountSection")
        if (
            LIMIT_LITERAL.search(line)
            and not SQL.search(line)
            and not relative.startswith(LIMIT_ALLOWED_PREFIXES)
            and not has_limit_test
        ):
            problems.append(f"{relative}:{number}: a limit is written into a message; add a *_matches_the_limit test that checks it against its constant")

COMPONENTS_DOC = ROOT / ".claude/rules/components.md"
STYLEGUIDE = ROOT / "src/pages/styleguide"
APP_CSS = (ROOT / "assets/css/app.css").read_text()
components_doc = COMPONENTS_DOC.read_text()
styleguide_code = "\n".join(path.read_text() for path in STYLEGUIDE.rglob("*.rs"))
for path in sorted((ROOT / "src/components").glob("*.rs")):
    if "#[component]" not in path.read_text():
        continue
    module = path.stem
    css = f"components/{module.replace('_', '-')}.css"
    relative = path.relative_to(ROOT).as_posix()
    if not (ROOT / "assets/css" / css).exists():
        problems.append(f"{relative}: component has no assets/css/{css}")
    elif f'"./{css}"' not in APP_CSS:
        problems.append(f"{relative}: assets/css/{css} isn't imported in assets/css/app.css")
    if f"components::{module}::" not in styleguide_code:
        problems.append(f"{relative}: component has no styleguide demo in src/pages/styleguide/")
    if f"`{module}`" not in components_doc:
        problems.append(f"{relative}: component isn't listed in .claude/rules/components.md")

events_code = (ROOT / "src/user/events.rs").read_text().split("#[cfg(test)]\nmod tests", 1)[0]
event_kinds = set(re.findall(r'=> "([a-z_]+)"', events_code))
kind_checks = [
    path for path in sorted((ROOT / "migrations").glob("*.sql"))
    if "account_events_kind_check CHECK" in path.read_text()
]
if kind_checks:
    latest = kind_checks[-1].read_text()
    allowed = set(re.findall(r"'([a-z_]+)'", latest.split("account_events_kind_check CHECK", 1)[1].split(");", 1)[0]))
    for kind in sorted(event_kinds - allowed):
        problems.append(f"src/user/events.rs: event kind `{kind}` is missing from the account_events kind CHECK ({kind_checks[-1].name}); add a migration")
    for kind in sorted(allowed - event_kinds):
        problems.append(f"{kind_checks[-1].name}: kind `{kind}` has no AccountEventKind variant")

job_kinds = set()
for path in (ROOT / "src").rglob("*.rs"):
    job_kinds.update(re.findall(r'const KIND: &\'static str = "([a-z_]+)";', path.read_text()))
job_checks = [
    path for path in sorted((ROOT / "migrations").glob("*.sql"))
    if "CREATE TABLE jobs" in path.read_text() or "jobs_kind_check CHECK" in path.read_text()
]
if job_checks:
    latest = job_checks[-1].read_text()
    marker = "jobs_kind_check CHECK" if "jobs_kind_check CHECK" in latest else "kind TEXT NOT NULL CHECK"
    allowed = set(re.findall(r"'([a-z_]+)'", latest.split(marker, 1)[1].split("))", 1)[0]))
    for kind in sorted(job_kinds - allowed):
        problems.append(f"job kind `{kind}` is missing from the jobs kind CHECK ({job_checks[-1].name}); add a migration")

documented = set()
for path in (ROOT / "src").rglob("*.rs"):
    text = path.read_text()
    for method, api_path in re.findall(r'const METHOD: HttpMethod = HttpMethod::(\w+);\s*const PATH: &\'static str = "([^"]+)"', text):
        documented.add((method.upper(), api_path))
for path in sorted((ROOT / "src/api/v1").rglob("*.rs")):
    for method, api_path in re.findall(r'#\[route\((GET|POST|PATCH|PUT|DELETE) "(/api/v1/[^"]+)"', path.read_text()):
        if api_path == "/api/v1/openapi.json":
            continue
        if (method, api_path) not in documented:
            problems.append(f"{path.relative_to(ROOT).as_posix()}: {method} {api_path} has no ApiAction in the registry (src/api/registry.rs), so it's undocumented")

RULE_MAX_BYTES = 4096
RULE_BROAD_GLOBS = {"**", "**/*", "src/**", "src/**/*.rs", "migrations/**", "migrations/*.sql", "e2e/tests/**"}
RULE_REPO_WIDE = {"layout.md", "topcoat.md"}
for path in sorted((ROOT / ".claude/rules").glob("*.md")):
    relative = path.relative_to(ROOT).as_posix()
    text = path.read_text()
    size = len(text.encode())
    if size > RULE_MAX_BYTES:
        problems.append(f"{relative}: {size} bytes; rule files stay under {RULE_MAX_BYTES} (split it by topic)")
    front = re.match(r"---\n(.*?)\n---\n", text, re.S)
    globs = re.findall(r'^\s*-\s*"([^"]+)"', front.group(1), re.M) if front and "paths:" in front.group(1) else []
    if not globs:
        problems.append(f"{relative}: no `paths` frontmatter, so it loads in every session; name the files it governs")
    if path.name not in RULE_REPO_WIDE:
        for glob in sorted(set(globs) & RULE_BROAD_GLOBS):
            problems.append(f"{relative}: glob `{glob}` loads it for unrelated files; name the files the topic governs")

sys.path.insert(0, str(ROOT / "scripts"))
import changed_specs
import clean_code

problems.extend(clean_code.problems())
for spec in changed_specs.unmapped_specs():
    problems.append(f"e2e/tests/{spec}: add it to an area in scripts/changed_specs.py, or `check-summary.sh --changed` never runs it")

for path in sorted((ROOT / "specs").glob("*.md")):
    status = changed_specs.spec_status(path.name.removesuffix(".md"))
    if status not in ("draft", "approved"):
        problems.append(f"specs/{path.name}: frontmatter needs `status: draft` or `status: approved`")
for path in sorted((ROOT / "e2e/scenarios").glob("*.spec.ts")):
    name = path.name.removesuffix(".spec.ts")
    if not (ROOT / "specs" / f"{name}.md").exists():
        problems.append(f"e2e/scenarios/{path.name}: no specs/{name}.md; scenarios come from a spec (new-spec skill)")
changed = changed_specs.changed_files()
builds_feature = any(path.startswith(("src/", "assets/", "migrations/")) for path in changed)
for path in changed:
    name = changed_specs.scenario_for(path)
    if builds_feature and path.startswith("e2e/scenarios/") and f"specs/{name}.md" not in changed:
        problems.append(f"{path}: changed alongside feature code while specs/{name}.md didn't; the builder never edits scenarios (change the spec first with the new-spec skill, or revert)")

if problems:
    print("Convention check failed:")
    print("\n".join(f"  {problem}" for problem in problems))
    sys.exit(1)
print("Conventions OK (paths, limits, components, event kinds, job kinds, API registry, clean code, spec areas, feature specs, rule files)")
