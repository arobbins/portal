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
PATH_ALLOWED = {"src/auth/links.rs", "src/api/catalog.rs"}
LIMIT_ALLOWED_PREFIXES = ("src/pages/styleguide",)
SQL = re.compile(r'"\s*(SELECT|INSERT|UPDATE|DELETE|WITH)\b')

problems = []
for path in sorted((ROOT / "src").rglob("*.rs")):
    relative = path.relative_to(ROOT).as_posix()
    text = path.read_text()
    code = text.split("#[cfg(test)]", 1)[0]
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
        if relative not in PATH_ALLOWED and ROUTE_LITERAL.search(line):
            problems.append(f"{relative}:{number}: hard-coded app path; use href!(handler) or AccountTab/AccountSection")
        if (
            LIMIT_LITERAL.search(line)
            and not SQL.search(line)
            and not relative.startswith(LIMIT_ALLOWED_PREFIXES)
            and not has_limit_test
        ):
            problems.append(f"{relative}:{number}: a limit is written into a message; add a *_matches_the_limit test that checks it against its constant")

if problems:
    print("Convention check failed:")
    print("\n".join(f"  {problem}" for problem in problems))
    sys.exit(1)
print("Conventions OK")
