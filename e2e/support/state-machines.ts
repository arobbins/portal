import { mkdirSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const RESULTS = fileURLToPath(new URL("../../target/state-machines/browser/", import.meta.url));

export type Expect<S extends string> =
  | { moves: S }
  | { refused: true }
  | { unreachable: string };

export type Machine<S extends string, E extends string> = {
  name: string;
  summary: string;
  testedBy: string;
  states: Record<S, string>;
  terminal?: S[];
  events: Record<E, string>;
  rules: string[];
  expect: (from: S, event: E) => Expect<S>;
};

export type Cell<S extends string, E extends string> = {
  from: S;
  event: E;
  actual: S | null;
  accepted: boolean;
  broken: string[];
};

export function passed<S extends string, E extends string>(
  machine: Machine<S, E>,
  cell: Cell<S, E>,
): boolean {
  const expected = machine.expect(cell.from, cell.event);
  const reached =
    "moves" in expected
      ? cell.accepted && cell.actual === expected.moves
      : "refused" in expected
        ? cell.actual === cell.from
        : cell.actual === null;
  return reached && cell.broken.length === 0;
}

export function writeResults<S extends string, E extends string>(
  machine: Machine<S, E>,
  cells: Cell<S, E>[],
): string[] {
  const terminal = new Set(machine.terminal ?? []);
  const report = {
    name: machine.name,
    summary: machine.summary,
    tested_by: machine.testedBy,
    states: Object.entries(machine.states).map(([id, label]) => ({
      id,
      label,
      terminal: terminal.has(id as S),
    })),
    events: Object.entries(machine.events).map(([id, label]) => ({ id, label, terminal: false })),
    stored_in: [],
    rules: machine.rules,
    writers: [],
    cells: cells.map((cell) => {
      const expected = machine.expect(cell.from, cell.event);
      return {
        from: cell.from,
        event: cell.event,
        expected: "moves" in expected ? expected.moves : "refused" in expected ? cell.from : null,
        refused: "refused" in expected,
        unreachable: "unreachable" in expected ? expected.unreachable : null,
        actual: cell.actual,
        accepted: cell.accepted,
        broken: cell.broken,
        passed: passed(machine, cell),
      };
    }),
  };
  mkdirSync(RESULTS, { recursive: true });
  const slug = machine.name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  writeFileSync(`${RESULTS}${slug}.json`, JSON.stringify(report, null, 2));
  return report.cells
    .filter((cell) => !cell.passed)
    .map(
      (cell) =>
        `${machine.states[cell.from as S]} + ${machine.events[cell.event as E]}: expected ${
          cell.expected ?? "nothing"
        }, got ${cell.actual} (accepted: ${cell.accepted}), broken: ${cell.broken.join("; ")}`,
    );
}

export function eachCell<S extends string, E extends string>(machine: Machine<S, E>) {
  const cells: { from: S; event: E; expected: Expect<S> }[] = [];
  for (const from of Object.keys(machine.states) as S[]) {
    for (const event of Object.keys(machine.events) as E[]) {
      cells.push({ from, event, expected: machine.expect(from, event) });
    }
  }
  return cells;
}
