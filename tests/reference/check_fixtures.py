#!/usr/bin/env python3
"""Assert scoped parity and exact, documented reference-reader differences."""
import argparse
from pathlib import Path
import subprocess
from compare import compare, normalize, parse, run_export


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust-export", type=Path, required=True)
    parser.add_argument("--kcachegrind-export", type=Path, required=True)
    args = parser.parse_args()
    rust, reference = args.rust_export.resolve(), args.kcachegrind_export.resolve()
    root = Path(__file__).resolve().parent
    repo = root.parent.parent
    for path in [root / "fixtures" / f"{name}.callgrind" for name in ("inline-source", "event-remapping", "cycle")] + [
        repo / "crates/callgrind-parser/tests/fixtures" / f"{name}.callgrind" for name in ("recursive-cost", "zero-call-count")
    ]:
        a, b = run_export(rust, path), run_export(reference, path)
        print(path.name, compare(a, b))
        if path.stem == "recursive-cost":
            values = {key[-1]: value[1] for key, value in b.rows.items() if key[0] == "I"}
            require(values == {"main": (9,), "recur": (7,)}, "recursive inclusive policy changed")
        if path.stem == "cycle":
            require(a.rows[("T", 0)][1] == (9,), "cycle self total must be 9")
            require(sum(key[0] == "E" for key in a.rows) == 3, "cycle edges must remain")
        if path.stem == "inline-source":
            functions = {key[-1]: value[1] for key, value in a.rows.items() if key[0] == "F"}
            require(functions == {"main": (10,), "other": (7,)}, "inline attribution split function")

    path = root / "fixtures/combined-aliases.callgrind"
    a = run_export(rust, path)
    require(a.rows[("F", 1, "", "/main.c", "main")][1] == (3,), "combined aliases not retained")
    b = subprocess.run([reference, path], text=True, capture_output=True)
    errors = [line for line in b.stderr.splitlines() if line.startswith("error:")]
    require(b.returncode == 1 and not b.stdout and errors == [
        "error:11: Undefined compressed file index 1",
        "error:11: Invalid file specification, setting to unknown",
        "error:12: Undefined compressed function index 1",
        "error:12: Invalid function specification, setting to unknown",
    ], f"unexpected combined-alias diagnostics: {b}")
    print(path.name, "expected exact alias-scope diagnostic")

    path = root / "fixtures/basename-collision.callgrind"
    a, b = run_export(rust, path), run_export(reference, path)
    fa = {key: value for key, value in a.rows.items() if key[0] == "F"}
    fb = {key: value for key, value in b.rows.items() if key[0] == "F"}
    require(fa == {("F", 0, "/one/lib.so", "/one/main.c", "work"): (None, (2,)),
                   ("F", 0, "/two/lib.so", "/two/main.c", "work"): (None, (3,))}, "Rust identity mismatch")
    require(fb == {("F", 0, "/one/lib.so", "/one/main.c", "work"): (None, (5,))}, "reference collision changed")
    remapped = {}
    for key, value in normalize(a, ["Ir"]).items():
        if key[0] in ("F", "L") and key[2] == "/two/lib.so":
            key = key[:2] + ("/one/lib.so", "/one/main.c") + key[4:]
        if key in remapped:
            value = (None, (remapped[key][1][0] + value[1][0],))
        remapped[key] = value
    require(remapped == normalize(b, ["Ir"]), "unexpected differences beyond basename identity collision")
    print(path.name, "expected identity merge; totals and attribution conserved")

    path = root / "fixtures/position-range.callgrind"
    a = subprocess.run([rust, path], text=True, capture_output=True)
    require(a.returncode != 0 and "kind: InvalidNumber, line: 6" in a.stderr and not a.stdout,
            "expected precise unsupported-range error")
    b = run_export(reference, path)
    expected = parse("P\t0\t4972\nT\t0\t4\nF\t0\t\t2f6d61696e2e63\t6d61696e\t4\n"
                     "L\t0\t\t2f6d61696e2e63\t6d61696e\t2f6d61696e2e63\t1\t4\n")
    compare(expected, b)
    print(path.name, "expected scalar/range dialect difference")


if __name__ == "__main__":
    main()
