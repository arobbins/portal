#!/usr/bin/env python3
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"

MAX_FUNCTION_LINES = 40
MAX_VIEW_LINES = 80
MAX_FILE_LINES = 300

VIEW_ATTRIBUTES = ("#[component", "#[page", "#[layout")
FEATURES = ("auth", "passkeys", "profile", "api_tokens", "account")
INFRASTRUCTURE = ("action", "env", "jobs", "mail", "rate_limit", "web", "secret", "flash", "permission")
FN = re.compile(r"^\s*(?:pub(?:\([\w:]+\))? )?(?:async )?fn (\w+)")
COMPOSITION = {"src/web/middleware.rs"}
TEST_MODULE = re.compile(r"^#\[cfg\(test\)\]\n(?:pub )?mod tests", re.M)


def relative(path):
    return path.relative_to(ROOT).as_posix()


def is_test_file(path):
    return path.name in ("tests.rs", "testing.rs")


def production_lines(path):
    text = path.read_text()
    if is_test_file(path):
        return []
    match = TEST_MODULE.search(text)
    lines = (text[: match.start()] if match else text).split("\n")
    return without_test_items(lines)


def without_test_items(lines):
    kept, index = [], 0
    while index < len(lines):
        if lines[index].strip() != "#[cfg(test)]":
            kept.append(lines[index])
            index += 1
            continue
        depth, began = 0, False
        index += 1
        while index < len(lines):
            depth += lines[index].count("{") - lines[index].count("}")
            began = began or "{" in lines[index]
            index += 1
            if (began and depth <= 0) or (not began and lines[index - 1].rstrip().endswith((";", ","))):
                break
    return kept


def functions(lines):
    index = 0
    while index < len(lines):
        match = FN.match(lines[index])
        if not match or lines[index].rstrip().endswith(";"):
            index += 1
            continue
        start, depth, began = index, 0, False
        while index < len(lines):
            depth += lines[index].count("{") - lines[index].count("}")
            began = began or "{" in lines[index]
            if began and depth <= 0:
                break
            index += 1
        attributes = " ".join(lines[max(0, start - 3) : start])
        is_view = any(attribute in attributes for attribute in VIEW_ATTRIBUTES)
        yield match.group(1), start + 1, index - start + 1, is_view
        index += 1


def module_of(path):
    parts = path.relative_to(SRC).with_suffix("").parts
    return parts[0]


def size_problems(path, lines):
    name = relative(path)
    if len(lines) > MAX_FILE_LINES:
        yield f"{name}: {len(lines)} lines before its tests (max {MAX_FILE_LINES}); split it by responsibility"
    for function, line, length, is_view in functions(lines):
        limit = MAX_VIEW_LINES if is_view else MAX_FUNCTION_LINES
        if length > limit:
            kind = "component" if is_view else "function"
            yield f"{name}:{line}: {kind} `{function}` is {length} lines (max {limit}); extract helpers or child components"


def boundary_problems(path, lines):
    name = relative(path)
    code = "\n".join(lines)
    in_actions = "/actions/" in name or name.endswith("/actions.rs")
    if in_actions and re.search(r"\bcx: &Cx\b|\bsession\(cx\)", code):
        yield f"{name}: actions take `env: &Env` (and a `Caller`), never `cx` or the session"
    if "now_utc()" in code and name != "src/env.rs":
        yield f"{name}: read the time from `env.now()` / `app.clock.now()`, not `now_utc()`"
    if re.search(r"(?<!try_)context::<", code) and not name.startswith("src/web"):
        yield f"{name}: read shared values with `web::app(cx)` / `env(cx)` / `config(cx)`, not `context::<…>`"
    if "rewrite(" in code and name != "src/web/context.rs":
        yield f"{name}: send a form back with `web::redisplay(path, failure)`, not `rewrite(…)`"
    if "#[allow(dead_code)]" in code:
        yield f"{name}: delete dead code (or gate it with `#[cfg(test)]`) instead of `#[allow(dead_code)]`"
    if re.search(r"^pub mod queries;", code, re.M) and name != "src/user.rs":
        yield f"{name}: keep `mod queries` private; other features call a function the feature exposes"
    if module_of(path) in INFRASTRUCTURE and name not in COMPOSITION:
        for feature in FEATURES:
            if re.search(rf"\bcrate::{feature}\b", code):
                yield f"{name}: infrastructure must not import the `{feature}` feature; invert it (a trait, a `Job`, or a registry in `server.rs`)"


def problems():
    found = []
    for path in sorted(SRC.rglob("*.rs")):
        lines = production_lines(path)
        found.extend(size_problems(path, lines))
        found.extend(boundary_problems(path, lines))
    return found


def report():
    rows, files = [], []
    for path in sorted(SRC.rglob("*.rs")):
        lines = production_lines(path)
        files.append((len(lines), relative(path)))
        for function, line, length, is_view in functions(lines):
            rows.append((length, f"{relative(path)}:{line}", function, is_view))
    plain = [row for row in rows if not row[3]]
    views = [row for row in rows if row[3]]
    print(f"functions: {len(plain)} plain, {len(views)} components/pages")
    for label, group, limit in (("plain", plain, MAX_FUNCTION_LINES), ("view", views, MAX_VIEW_LINES)):
        lengths = sorted(row[0] for row in group) or [0]
        over = sum(1 for length in lengths if length > limit * 0.75)
        print(f"  {label}: median {lengths[len(lengths) // 2]}, max {lengths[-1]}, within 25% of the limit: {over}")
    print("largest functions:")
    for length, place, function, is_view in sorted(rows, reverse=True)[:12]:
        print(f"  {length:4} {place} {function}{' (view)' if is_view else ''}")
    print("largest files (lines before tests):")
    for length, name in sorted(files, reverse=True)[:8]:
        print(f"  {length:4} {name}")
    found = problems()
    print(f"violations: {len(found)}")
    for problem in found:
        print(f"  {problem}")


def changed_files():
    import subprocess

    tracked = subprocess.run(["git", "diff", "--name-only", "HEAD", "--", "src"], cwd=ROOT, capture_output=True, text=True).stdout
    untracked = subprocess.run(["git", "ls-files", "--others", "--exclude-standard", "--", "src"], cwd=ROOT, capture_output=True, text=True).stdout
    names = {name for name in (tracked + untracked).split() if name.endswith(".rs")}
    return [ROOT / name for name in sorted(names) if (ROOT / name).exists()]


def changed_report():
    near = []
    for path in changed_files():
        lines = production_lines(path)
        if len(lines) > MAX_FILE_LINES * 0.75:
            near.append(f"{relative(path)}: {len(lines)}/{MAX_FILE_LINES} lines")
        for function, line, length, is_view in functions(lines):
            limit = MAX_VIEW_LINES if is_view else MAX_FUNCTION_LINES
            if length > limit * 0.75:
                near.append(f"{relative(path)}:{line}: `{function}` {length}/{limit} lines")
    violations = [problem for problem in problems() if any(problem.startswith(relative(path)) for path in changed_files())]
    if not near and not violations:
        print("Changed files: sizes and boundaries OK")
    for problem in violations:
        print(f"violation: {problem}")
    for item in near:
        print(f"near limit: {item}")


if __name__ == "__main__":
    if "--report" in sys.argv:
        report()
    elif "--changed" in sys.argv:
        changed_report()
    else:
        found = problems()
        print("\n".join(found) or "Clean code OK")
        sys.exit(1 if found else 0)
