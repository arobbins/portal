import html
import json
import re
import sys
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RESULTS = ROOT / "target/state-machines"
CATALOG = ROOT / ".claude/state-machines.md"
OUTPUT = RESULTS / "index.html"
MERMAID = "https://cdn.jsdelivr.net/npm/mermaid@11.4.1/dist/mermaid.esm.min.mjs"
FUNCTION = re.compile(r"\bfn (\w+)")
DEFINITIONS = "@definitions"


def catalog_rows():
    rows = []
    for line in CATALOG.read_text().splitlines():
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 3 or cells[0] in ("State", "---") or set(cells[0]) <= {"-"}:
            continue
        rows.append({"name": cells[0], "type": cells[1], "transitions": cells[2]})
    return rows


def code_text(markdown):
    escaped = html.escape(markdown)
    return re.sub(r"`([^`]+)`", r"<code>\1</code>", escaped)


def source_functions():
    for path in sorted((ROOT / "src").rglob("*.rs")):
        if path.name in ("tests.rs", "testing.rs") or {"tests", "state_machines"} & set(path.parts):
            continue
        code = path.read_text().split("#[cfg(test)]\nmod tests", 1)[0]
        starts = list(FUNCTION.finditer(code))
        for index, found in enumerate(starts):
            end = starts[index + 1].start() if index + 1 < len(starts) else len(code)
            preceding = code[: found.start()].rstrip().splitlines()[-2:]
            if any(line.strip() == "#[cfg(test)]" for line in preceding):
                continue
            yield path.relative_to(ROOT).as_posix(), found.group(1), code[found.start():end]


def write_patterns(stored_in):
    patterns = []
    for place in stored_in:
        place = place.removesuffix(DEFINITIONS)
        if place.startswith("session:"):
            key = re.escape(place.removeprefix("session:"))
            patterns.append(re.compile(rf"\.(insert|remove(::<[^>]*>)?)\(\s*{key}\b"))
            continue
        table, _, column = place.partition(".")
        if column == "*":
            patterns.append(re.compile(rf"UPDATE {table}\s+SET\b"))
        elif column:
            patterns.append(re.compile(rf"UPDATE {table}\s+SET(?:(?!\bWHERE\b)[^\"])*?\b{column}\s*=", re.S))
        else:
            patterns.append(re.compile(rf"(INSERT INTO|DELETE FROM) {table.split()[0]}\b"))
    return patterns


def writer_queries(stored_in):
    patterns = write_patterns(stored_in)
    return sorted({name for _, name, body in source_functions() if any(p.search(body) for p in patterns)})


def found_writers(stored_in):
    if any(place.endswith(DEFINITIONS) for place in stored_in):
        patterns = write_patterns(stored_in)
        return {
            (f"{path}:{name}", name)
            for path, name, body in source_functions()
            if any(p.search(body) for p in patterns)
        }
    queries = writer_queries(stored_in)
    if not queries:
        return set()
    calls = re.compile(r"\b(" + "|".join(map(re.escape, queries)) + r")\(")
    found = set()
    for path, name, body in source_functions():
        for query in calls.findall(body.split("\n", 1)[1] if "\n" in body else ""):
            found.add((f"{path}:{name}", query))
    return found


def check_writers(machine):
    declared = {(writer["caller"], writer["query"]): writer["events"] for writer in machine["writers"]}
    found = found_writers(machine["stored_in"])
    rows = []
    for caller, query in sorted(found | set(declared)):
        if (caller, query) not in found:
            status = "gone"
        elif (caller, query) not in declared:
            status = "unmodeled"
        else:
            status = "ok"
        rows.append({"caller": caller, "query": query, "events": declared.get((caller, query), []), "status": status})
    return rows


def load_machines():
    machines = {}
    for path in sorted(RESULTS.rglob("*.json")):
        machine = json.loads(path.read_text())
        machine["tested_at"] = datetime.fromtimestamp(path.stat().st_mtime).strftime("%b %-d, %Y at %H:%M")
        machine["writes"] = check_writers(machine)
        machines[machine["name"]] = machine
    return machines


def counts(machine):
    cells = machine["cells"]
    checked = [cell for cell in cells if not cell["unreachable"]]
    moves = [cell for cell in checked if not cell["refused"]]
    return {
        "states": len(machine["states"]),
        "events": len(machine["events"]),
        "transitions": len(moves),
        "refused": len(checked) - len(moves),
        "unreachable": len(cells) - len(checked),
        "checked": len(checked),
        "failed": sum(1 for cell in cells if not cell["passed"]),
        "writes": sum(1 for writer in machine["writes"] if writer["status"] != "gone"),
        "unmodeled": sum(1 for writer in machine["writes"] if writer["status"] != "ok"),
    }


def complexity(numbers):
    score = numbers["transitions"] + numbers["writes"]
    if score >= 20:
        return "High"
    if score >= 10:
        return "Medium"
    return "Low"


def diagram(machine):
    events = {event["id"]: event["label"] for event in machine["events"]}
    initial = machine["states"][0]["id"]
    edges = {}
    failing = set()
    for cell in machine["cells"]:
        if not cell["passed"]:
            failing.add(cell["from"])
        if cell["unreachable"] or cell["refused"] or cell["expected"] == cell["from"]:
            continue
        edges.setdefault((cell["from"], cell["expected"]), []).append(events[cell["event"]])
    lines = ["stateDiagram-v2", "  direction LR"]
    for state in machine["states"]:
        if state["id"] != initial:
            lines.append(f'  state "{state["label"]}" as {state["id"]}')
    for (source, target), names in edges.items():
        source = "[*]" if source == initial else source
        lines.append(f"  {source} --> {target} : {', '.join(names)}")
    for state in machine["states"]:
        if state["terminal"]:
            lines.append(f"  {state['id']} --> [*]")
    lines.append("  classDef failing fill:#fde2e1,stroke:#b3261e,color:#5c1410")
    for state in sorted(failing - {initial}):
        lines.append(f"  class {state} failing")
    return "\n".join(lines)


def matrix(machine):
    labels = {state["id"]: state["label"] for state in machine["states"]}
    cells = {(cell["from"], cell["event"]): cell for cell in machine["cells"]}
    head = "".join(f"<th scope='col'>{html.escape(event['label'])}</th>" for event in machine["events"])
    rows = []
    for state in machine["states"]:
        if all(cells[(state["id"], event["id"])]["unreachable"] for event in machine["events"]):
            continue
        row = [f"<th scope='row'>{html.escape(state['label'])}</th>"]
        for event in machine["events"]:
            row.append(matrix_cell(cells[(state["id"], event["id"])], labels))
        rows.append(f"<tr>{''.join(row)}</tr>")
    return f"<table class='matrix'><thead><tr><th></th>{head}</tr></thead><tbody>{''.join(rows)}</tbody></table>"


def matrix_cell(cell, labels):
    if cell["unreachable"]:
        return f"<td class='unreachable' title='{html.escape(cell['unreachable'])}'>Can't happen</td>"
    if not cell["passed"]:
        actual = labels.get(cell["actual"], "nothing")
        expected = "no change" if cell["refused"] else labels[cell["expected"]]
        broken = "".join(f"<li>{html.escape(rule)}</li>" for rule in cell["broken"])
        reason = f"<ul class='broken'>{broken}</ul>" if broken else ""
        return f"<td class='failed'>Expected {html.escape(expected)}, got {html.escape(actual)}{reason}</td>"
    if cell["refused"]:
        return "<td class='refused'>No change</td>"
    return f"<td class='moves'>→ {html.escape(labels[cell['expected']])}</td>"


def status_of(machine):
    if machine is None:
        return ("Not modeled yet", "pending")
    numbers = counts(machine)
    if numbers["failed"]:
        return (f"{numbers['failed']} failing", "failing")
    if numbers["unmodeled"]:
        return (f"{numbers['unmodeled']} writers not modeled", "failing")
    return ("All transitions pass", "passing")


def overview_card(row, machine):
    label, tone = status_of(machine)
    link = f"href='#{slug(row['name'])}'" if machine else ""
    tag = "a" if machine else "div"
    detail = ""
    if machine:
        numbers = counts(machine)
        detail = (
            f"<p class='numbers'>{numbers['states']} states · {numbers['transitions']} transitions · "
            f"{numbers['writes']} write sites · {complexity(numbers)} complexity</p>"
        )
    return (
        f"<{tag} class='card {tone}' {link}><div class='card-top'><h3>{html.escape(row['name'])}</h3>"
        f"<span class='pill {tone}'>{label}</span></div>{detail}"
        f"<p class='type'>{code_text(row['type'])}</p></{tag}>"
    )


def machine_section(machine):
    numbers = counts(machine)
    stats = "".join(
        f"<div class='stat'><span class='value'>{value}</span><span class='name'>{name}</span></div>"
        for name, value in [
            ("States", numbers["states"]),
            ("Events", numbers["events"]),
            ("Transitions", numbers["transitions"]),
            ("Checked pairs", numbers["checked"]),
            ("Write sites", numbers["writes"]),
            ("Complexity", complexity(numbers)),
        ]
    )
    events = {event["id"]: event["label"] for event in machine["events"]}
    writes = "".join(writer_row(writer, events) for writer in machine["writes"])
    rules = "".join(rule_item(rule, machine["cells"]) for rule in machine["rules"])
    stored = ", ".join(f"<code>{html.escape(place)}</code>" for place in machine["stored_in"])
    return f"""
<section class="machine" id="{slug(machine['name'])}">
  <h2>{html.escape(machine['name'])}</h2>
  <p class="summary">{html.escape(machine['summary'])}</p>
  <p class="meta">Tested {machine['tested_at']} by <code>{html.escape(machine.get('tested_by') or 'cargo test state_machines::' + slug(machine['name']).replace('-', '_'))}</code></p>
  <div class="stats">{stats}</div>
  <h3>Diagram</h3>
  <div class="diagram"><pre class="mermaid">{html.escape(diagram(machine))}</pre></div>
  <h3>Every state against every event</h3>
  <p class="hint">Each cell ran through the real actions. Hover a “Can't happen” cell for the reason.</p>
  <div class="scroll">{matrix(machine)}</div>
  <h3>Rules checked after every cell</h3>
  <ul class="rules">{rules}</ul>
  <h3>Where it's written</h3>
  <p class="hint">Stored in {stored}. Every function that calls a query changing it must be in the model, or the check fails.</p>
  <div class="scroll"><table class="writers"><thead><tr><th>Caller</th><th>Query</th><th>Event</th><th></th></tr></thead>
  <tbody>{writes}</tbody></table></div>
</section>"""


WRITER_STATUS = {
    "ok": ("In the model", "ok"),
    "unmodeled": ("Not in the model: add an event or a writer entry", "bad"),
    "gone": ("No longer found: remove the writer entry", "bad"),
}


def writer_row(writer, events):
    label, tone = WRITER_STATUS[writer["status"]]
    names = [events[event] for event in writer["events"]]
    event = ", ".join(names) if names else ("Not an event here (seed data or another machine)" if writer["status"] == "ok" else "—")
    return (
        f"<tr class='{tone}'><td><code>{html.escape(writer['caller'])}</code></td>"
        f"<td><code>{html.escape(writer['query'])}</code></td><td>{html.escape(event)}</td><td>{label}</td></tr>"
    )


def rule_item(rule, cells):
    broken = sum(1 for cell in cells if rule in cell["broken"])
    mark = f"<span class='pill failing'>Broken in {broken}</span>" if broken else "<span class='pill passing'>Holds</span>"
    return f"<li>{mark} {html.escape(rule)}</li>"


def slug(name):
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")


STYLE = """
:root { --bg:#faf8f4; --card:#fff; --text:#3b352d; --strong:#1c1915; --muted:#7d6334; --border:#e4dfd6;
  --pass:#1d6b3a; --pass-bg:#e3f2e7; --fail:#b3261e; --fail-bg:#fde2e1; --pending:#7d6334; --pending-bg:#f3f0ea; }
@media (prefers-color-scheme: dark) { :root { --bg:#12100e; --card:#1e1e1c; --text:#cfb57f; --strong:#ebdec3;
  --muted:#947d4d; --border:#2a2a24; --pass:#7fd49a; --pass-bg:#173322; --fail:#ff8a80; --fail-bg:#3b1715;
  --pending:#947d4d; --pending-bg:#2a2a24; } }
* { box-sizing:border-box; }
body { margin:0; background:var(--bg); color:var(--text); font:15px/1.55 system-ui, -apple-system, "Segoe UI", sans-serif; }
main { max-width:1180px; margin:0 auto; padding:40px 16px 80px; }
h1, h2, h3 { color:var(--strong); line-height:1.2; }
h1 { font-size:32px; margin:0 0 6px; } h2 { font-size:24px; margin:0 0 4px; } h3 { font-size:16px; margin:28px 0 8px; }
code { font-size:.9em; background:var(--pending-bg); padding:1px 5px; border-radius:4px; overflow-wrap:anywhere; }
.lead, .summary, .meta, .hint { color:var(--muted); margin:0 0 12px; }
.grid { display:grid; grid-template-columns:repeat(auto-fill, minmax(320px, 1fr)); gap:14px; margin:24px 0 48px; }
.card { display:block; background:var(--card); border:1px solid var(--border); border-radius:12px; padding:16px;
  color:inherit; text-decoration:none; }
a.card:hover { border-color:var(--muted); }
.card.passing { border-left:4px solid var(--pass); } .card.failing { border-left:4px solid var(--fail); }
.card-top { display:flex; justify-content:space-between; gap:12px; align-items:flex-start; }
.card h3 { margin:0; }
.card .type { font-size:13px; margin:8px 0 0; } .card .numbers { margin:8px 0 0; font-size:13px; color:var(--strong); }
.pill { font-size:12px; border-radius:999px; padding:2px 10px; white-space:nowrap; }
.pill.passing { color:var(--pass); background:var(--pass-bg); } .pill.failing { color:var(--fail); background:var(--fail-bg); }
.pill.pending { color:var(--pending); background:var(--pending-bg); }
.machine { background:var(--card); border:1px solid var(--border); border-radius:14px; padding:24px; margin-bottom:24px; }
.stats { display:grid; grid-template-columns:repeat(auto-fill, minmax(130px, 1fr)); gap:10px; }
.stat { border:1px solid var(--border); border-radius:10px; padding:10px 12px; }
.stat .value { display:block; font-size:22px; color:var(--strong); } .stat .name { font-size:13px; color:var(--muted); }
.diagram { background:#fff; border-radius:10px; padding:12px; overflow-x:auto; }
.scroll { overflow-x:auto; }
.matrix { border-collapse:collapse; font-size:13px; min-width:900px; }
.matrix th, .matrix td { border:1px solid var(--border); padding:8px; text-align:left; vertical-align:top; }
.matrix thead th { color:var(--strong); font-weight:600; }
.matrix tbody th { color:var(--strong); font-weight:600; white-space:nowrap; }
.moves { background:var(--pass-bg); color:var(--pass); } .failed { background:var(--fail-bg); color:var(--fail); font-weight:600; }
.refused { color:var(--muted); }
.unreachable { color:var(--muted); background:repeating-linear-gradient(135deg, transparent 0 6px, var(--pending-bg) 6px 12px); cursor:help; }
.rules { list-style:none; margin:0; padding:0; display:grid; gap:8px; }
.rules .pill { margin-right:8px; }
.broken { margin:6px 0 0; padding-left:16px; font-weight:400; }
.writers { border-collapse:collapse; font-size:13px; width:100%; }
.writers th, .writers td { border-bottom:1px solid var(--border); padding:8px; text-align:left; }
.writers tr.bad td { background:var(--fail-bg); color:var(--fail); }
"""


def render():
    machines = load_machines()
    rows = catalog_rows()
    known = {row["name"] for row in rows}
    rows += [{"name": name, "type": "", "transitions": ""} for name in machines if name not in known]
    modeled = sum(1 for row in rows if row["name"] in machines)
    failing = sum(1 for machine in machines.values() if status_of(machine)[1] == "failing")
    cards = "".join(overview_card(row, machines.get(row["name"])) for row in rows)
    sections = "".join(machine_section(machine) for machine in machines.values())
    page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>State machines</title><style>{STYLE}</style></head>
<body><main>
<h1>State machines</h1>
<p class="lead">{len(rows)} pieces of state in the app · {modeled} modeled and tested · {failing} failing.
Generated by <code>./check.sh</code> from <code>.claude/state-machines.md</code> and <code>cargo test state_machines</code>.</p>
<div class="grid">{cards}</div>
{sections}
</main>
<script type="module">
import mermaid from "{MERMAID}";
mermaid.initialize({{ startOnLoad: true, theme: "neutral" }});
</script>
</body></html>"""
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(page)
    print(f"State machine report: {OUTPUT.relative_to(ROOT)} ({modeled}/{len(rows)} modeled, {failing} failing)")
    return failing


if __name__ == "__main__":
    sys.exit(1 if render() else 0)
