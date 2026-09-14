#!/usr/bin/env python3
"""Compare plain-text semantic tables on identical raw inputs, not whitespace."""
import argparse
from pathlib import Path
import re
import subprocess
import sys


def table(text):
    events = next((line.split(":", 1)[1].split() for line in text.splitlines() if line.startswith("Events shown:")), None)
    if not events:
        raise ValueError("annotation missing Events shown")
    width = len(events)
    functions, tree = [], []
    totals = None
    in_functions = False
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
            label = re.sub(r"\s+", " ", label.removeprefix("*").strip())
            label = label.removesuffix(" []")
            # Perl drops object decorations on some inline-attribution rows.
            # Full object identity is checked separately by the libcore gate.
            label = re.sub(r" \[[^\]]*\]$", "", label)
            if label.startswith(("<", ">")):
                tree.append((label, values))
            elif any(values):
                functions.append((label, values))
    if totals is None:
        raise ValueError("annotation missing PROGRAM TOTALS")
    if len({name for name, _ in functions}) != len(functions):
        raise ValueError("duplicate nonzero function display identity")
    # Tree tie order is unspecified upstream; compare the multiset only.
    return events, totals, functions, sorted(tree)


def check(rust, perl, path, options):
    outputs = []
    for tool in (rust, perl):
        extra = ["--grouping=source"] if tool == rust else []
        # The Perl script trims cwd from only some file tags. Run away from
        # the source tree so that presentation quirk cannot split identities.
        result = subprocess.run([str(tool), "--auto=no", *extra, *options, str(path.resolve())], cwd=path.resolve().parent,
                                capture_output=True, text=True, check=True)
        if result.stderr:
            print(result.stderr, file=sys.stderr, end="")
        outputs.append(table(result.stdout))
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
    parser.add_argument("--parser-fixtures", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    parser_fixtures = args.parser_fixtures or root.parent.parent / "crates/callgrind-parser/tests/fixtures"
    profiles = [root / "fixtures/annotate-basic.callgrind", parser_fixtures / "recursive-cost.callgrind"]
    if args.matrix:
        from check_matrix import validate
        profiles += validate(args.matrix)
    runs = rows = 0
    for path in profiles:
        for options in [[], ["--threshold=100", "--show-percs=no"], ["--inclusive=yes"],
                        ["--inclusive=yes", "--threshold=100", "--show-percs=no"],
                        ["--threshold=80", "--show=Ir", "--sort=Ir"],
                        ["--threshold=100", "--tree=both", "--show-percs=no"]]:
            rows += check(args.rust.resolve(), args.reference.resolve(), path, options)
            runs += 1
        if path.stem == "annotate-basic":
            for options in [["--show=Dr,Ir", "--sort=Dr,Ir", "--threshold=100"], ["--sort=Ir:80,Dr:90"], ["--threshold=0"]]:
                rows += check(args.rust.resolve(), args.reference.resolve(), path, options)
                runs += 1
        print(path.name, "annotation semantic checks passed", flush=True)
    print(f"passed {runs} same-file annotation comparisons ({rows} nonzero function rows)")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
