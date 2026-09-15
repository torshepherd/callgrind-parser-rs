#!/usr/bin/env python3
"""Validate a workload matrix against its plan and invoke production-parser checks."""
import argparse
from collections import Counter
import csv
import json
import re
from pathlib import Path
import subprocess
import sys
from compare import compare, integer, run_export

CONFIGURATIONS = json.loads((Path(__file__).parent / "configurations.json").read_text())
CASES = set(CONFIGURATIONS)
EXPECTED = {(case, str(pages)) for case in CASES for pages in (64, 4096)}


def load_plan(path):
    plan = json.loads(Path(path).read_text())
    identifier = re.compile(r"[a-z0-9][a-z0-9-]*")
    if not isinstance(plan, dict) or plan.get("schema") != 1:
        raise ValueError("unsupported workload plan schema")
    if not isinstance(plan.get("name"), str) or not identifier.fullmatch(plan["name"]):
        raise ValueError("invalid workload name")
    configs = plan.setdefault("configurations", list(CONFIGURATIONS))
    if not isinstance(configs, list) or not configs or any(not isinstance(c, str) or c not in CASES for c in configs) or len(set(configs)) != len(configs):
        raise ValueError("configurations must be unique known names")
    variants = plan.get("variants")
    if not isinstance(variants, list) or not variants:
        raise ValueError("plan must contain variants")
    names = set()
    for variant in variants:
        if not isinstance(variant, dict):
            raise ValueError("variant must be an object")
        name = variant.get("name")
        if not isinstance(name, str) or not identifier.fullmatch(name) or name in names:
            raise ValueError("variant names must be safe and unique")
        names.add(name)
        argv = variant.get("argv")
        if not isinstance(argv, list) or not argv or any(not isinstance(a, str) or "\0" in a for a in argv) or not Path(argv[0]).is_absolute():
            raise ValueError("argv must be strings with an absolute executable path")
        stdin = variant.get("stdin")
        if stdin is not None and (not isinstance(stdin, str) or not Path(stdin).is_absolute()):
            raise ValueError("stdin must be an absolute file path")
    timeout = plan.setdefault("timeout_seconds", 300)
    if type(timeout) is not int or timeout <= 0:
        raise ValueError("timeout_seconds must be a positive integer")
    return plan


def validate(directory, plan=None):
    directory = Path(directory)
    recorded = directory / "PLAN.json"
    if plan is None and recorded.exists():
        plan = load_plan(recorded)
    elif plan is not None and (not recorded.exists() or load_plan(recorded) != plan):
        raise ValueError("recorded plan differs from expected workload plan")
    expected = ({(case, v["name"]) for case in plan["configurations"] for v in plan["variants"]}
                if plan else {(case, f"sqlite-cache-{pages}") for case, pages in EXPECTED})
    stems = {f"{variant}__{case}" for case, variant in expected}
    for suffix in (".callgrind", ".annotated.txt"):
        actual = {p.name.removesuffix(suffix) for p in directory.glob(f"*{suffix}")}
        if actual != stems:
            raise ValueError(f"incorrect {suffix} case set: missing={stems - actual}, extra={actual - stems}")
    with (directory / "SUMMARY.tsv").open(newline="") as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        legacy = reader.fieldnames == ["case", "sqlite_cache_pages", "events", "summary", "profile_bytes"]
        if reader.fieldnames != ["case", "variant", "events", "summary", "profile_bytes"] and not legacy:
            raise ValueError("incorrect manifest columns")
        rows = list(reader)
    if any(None in row or any(v is None for v in row.values()) for row in rows):
        raise ValueError("malformed manifest row")
    pairs = [(r["case"], f"sqlite-cache-{r['sqlite_cache_pages']}" if legacy else r["variant"]) for r in rows]
    if len(pairs) != len(expected) or set(pairs) != expected:
        raise ValueError(f"manifest must have exactly the {len(expected)} unique case pairs")
    for row, (case, variant) in zip(rows, pairs, strict=True):
        stem = f"{variant}__{case}"
        profile = directory / (stem + ".callgrind")
        if integer(row["profile_bytes"]) != profile.stat().st_size or not profile.stat().st_size:
            raise ValueError(f"{stem}: manifest byte count mismatch")
        if not row["events"] or not row["summary"]:
            raise ValueError(f"{stem}: empty event/summary manifest field")
        if "PROGRAM TOTALS" not in (directory / (stem + ".annotated.txt")).read_text():
            raise ValueError(f"{stem}: annotation missing PROGRAM TOTALS")
    return [directory / (stem + ".callgrind") for stem in sorted(stems)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--inspect", type=Path, required=True)
    parser.add_argument("--plan", type=Path)
    parser.add_argument("--rust-export", type=Path)
    parser.add_argument("--kcachegrind-export", type=Path)
    args = parser.parse_args()
    if bool(args.rust_export) != bool(args.kcachegrind_export):
        parser.error("both exporters must be supplied together")
    paths = validate(args.directory, load_plan(args.plan) if args.plan else None)
    counts = Counter()
    for path in paths:
        subprocess.run([str(args.inspect.resolve()), str(path)], check=True)
        if args.rust_export:
            result = compare(run_export(args.rust_export.resolve(), path), run_export(args.kcachegrind_export.resolve(), path))
            counts.update(result)
            print(path.name, result, flush=True)
    print(f"validated {len(paths)} profiles, {len(paths)} annotations, {len(paths)} manifest rows; compared {dict(counts)}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
