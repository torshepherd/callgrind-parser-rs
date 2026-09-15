#!/usr/bin/env python3
"""Compare plain-text semantic tables on identical raw inputs, not whitespace."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import sys


def table(text):
    events = next((line.split(":", 1)[1].split() for line in text.splitlines() if line.startswith("Events shown:")), None)
    if not events:
        raise ValueError("annotation missing Events shown")
    width = len(events)
    functions, tree = [], {}
    displayed = set()
    pending_callers = []
    current_function = None
    totals = None
    in_functions = False

    def edge(direction, caller, callee, count, values):
        key = (direction, caller, callee)
        old_count, old_values = tree.get(key, (0, (0,) * width))
        tree[key] = (old_count + count, tuple(a + b for a, b in zip(old_values, values, strict=True)))

    for line in text.splitlines():
        if " file:function" in line:
            in_functions = True
            continue
        plain = re.sub(r"\s*\(\s*[0-9.]+%\)", "", line)
        fields = plain.split(None, width)
        if len(fields) != width + 1 or not all(re.fullmatch(r"[0-9,]+|\.", f) for f in fields[:width]):
            continue
        values = tuple(0 if f == "." else int(f.replace(",", "")) for f in fields[:width])
        label = fields[-1]
        if label.startswith("PROGRAM TOTALS"):
            if totals is not None:
                raise ValueError("duplicate PROGRAM TOTALS")
            totals = values
        elif in_functions:
            raw_label = re.sub(r"\s+", " ", label.removeprefix("*").strip()).removesuffix(" []")
            # Perl's identity is file:function, regardless of object. Project
            # explicitly into that view; never change the production model.
            label = re.sub(r" \[[^\]]*\]$", "", raw_label)
            if label.startswith(("<", ">")):
                match = re.fullmatch(r"([<>]) (.*) \(([0-9,]+)x\)", label)
                if not match:
                    raise ValueError(f"malformed tree label: {label}")
                direction, name, count = match.groups()
                count = int(count.replace(",", ""))
                if direction == "<":
                    pending_callers.append((name, count, values))
                elif current_function is None:
                    raise ValueError("callee row without parent function")
                else:
                    edge(direction, current_function, name, count, values)
            else:
                current_function = label
                for caller, count, costs in pending_callers:
                    edge("<", caller, current_function, count, costs)
                pending_callers.clear()
                if any(values):
                    if raw_label in displayed:
                        raise ValueError("duplicate nonzero function display identity")
                    displayed.add(raw_label)
                    functions.append((label, values))
    if totals is None:
        raise ValueError("annotation missing PROGRAM TOTALS")
    if pending_callers:
        raise ValueError("caller rows without parent function")
    if len({name for name, _ in functions}) != len(functions):
        sorting = next((line.split(":", 1)[1].split() for line in text.splitlines()
                        if line.startswith("Event sort order:")), events)
        if any(event not in events for event in sorting):
            raise ValueError("object-collision projection requires visible sort events")
        indices = [events.index(event) for event in sorting]
        key = lambda row: (tuple(-row[1][i] for i in indices), row[0])
        # Projection changes ranks, but must not hide misordered original rows.
        if functions != sorted(functions, key=key):
            raise ValueError("function rows not in declared sort order")
        merged = {}
        for name, values in functions:
            merged[name] = tuple(a + b for a, b in zip(merged.get(name, (0,) * width), values, strict=True))
        functions = sorted(merged.items(), key=key)
    # Keep both endpoint identities and direction; only edge tie order and
    # object distinctions are projected out, not caller/callee attachment.
    return events, totals, functions, sorted(tree.items())


def check(rust, perl, path, options, artifacts=None):
    results = []
    for label, tool in (("rust", rust), ("reference", perl)):
        extra = ["--grouping=source"] if label == "rust" else []
        # The Perl script trims cwd from only some file tags. Run away from
        # the source tree so that presentation quirk cannot split identities.
        command = [str(tool), "--auto=no", *extra, *options, str(path.resolve())]
        result = subprocess.run(command, cwd=path.resolve().parent, capture_output=True, text=True, timeout=120)
        if artifacts:
            artifacts.mkdir(parents=True, exist_ok=True)
            (artifacts / f"{label}.stdout.txt").write_text(result.stdout)
            (artifacts / f"{label}.stderr.txt").write_text(result.stderr)
            (artifacts / f"{label}.command.json").write_text(json.dumps(
                dict(argv=command, cwd=str(path.resolve().parent), returncode=result.returncode), indent=2) + "\n")
        if result.stderr:
            print(result.stderr, file=sys.stderr, end="")
        results.append(result)
    for result in results:
        result.check_returncode()
    outputs = []
    for label, result in zip(("rust", "reference"), results, strict=True):
        try:
            outputs.append(table(result.stdout))
        except ValueError as error:
            raise ValueError(f"{path.name} {options}: {label}: {error}") from error
    if outputs[0] != outputs[1]:
        for label, a, b in zip(("events", "totals", "function order/costs", "tree edges"), *outputs, strict=True):
            if a != b:
                if isinstance(a, list) and isinstance(b, list):
                    differences = [(i, x, y) for i, (x, y) in enumerate(zip(a, b)) if x != y][:5]
                    raise ValueError(f"{path.name} {options}: {label}: lengths {len(a)}/{len(b)}; {differences}")
                raise ValueError(f"{path.name} {options}: {label}: {a!r} != {b!r}")
    return len(outputs[0][2])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--matrix", type=Path)
    parser.add_argument("--plan", type=Path)
    parser.add_argument("--artifacts", type=Path)
    parser.add_argument("--parser-fixtures", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    parser_fixtures = args.parser_fixtures or root.parent.parent / "crates/callgrind-parser/tests/fixtures"
    profiles = [root / "fixtures/annotate-basic.callgrind", parser_fixtures / "recursive-cost.callgrind",
                root / "fixtures/object-collision.callgrind"]
    if args.plan and not args.matrix:
        parser.error("--plan requires --matrix")
    if args.matrix:
        from check_matrix import load_plan, validate
        profiles += validate(args.matrix, load_plan(args.plan) if args.plan else None)
    runs = rows = 0
    for path in profiles:
        for options in [[], ["--threshold=100", "--show-percs=no"], ["--inclusive=yes"],
                        ["--inclusive=yes", "--threshold=100", "--show-percs=no"],
                        ["--threshold=80", "--show=Ir", "--sort=Ir"],
                        ["--threshold=100", "--tree=both", "--show-percs=no"]]:
            rows += check(args.rust.resolve(), args.reference.resolve(), path, options,
                          args.artifacts / path.stem / str(runs) if args.artifacts else None)
            runs += 1
        if path.stem == "annotate-basic":
            for options in [["--show=Dr,Ir", "--sort=Dr,Ir", "--threshold=100"], ["--sort=Ir:80,Dr:90"], ["--threshold=0"]]:
                rows += check(args.rust.resolve(), args.reference.resolve(), path, options,
                          args.artifacts / path.stem / str(runs) if args.artifacts else None)
                runs += 1
        print(path.name, "annotation semantic checks passed", flush=True)
    print(f"passed {runs} same-file annotation comparisons ({rows} nonzero function rows)")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        sys.exit(str(error))
