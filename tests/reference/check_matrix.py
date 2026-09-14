#!/usr/bin/env python3
"""Validate the exact six-by-two matrix and invoke production-parser checks."""
import argparse
from collections import Counter
import csv
from pathlib import Path
import subprocess
import sys
from compare import compare, integer, run_export

CASES = {"ir-only", "branches-and-jumps", "balanced-cache", "constrained-cache", "roomy-cache", "full-simulation"}
EXPECTED = {(case, str(pages)) for case in CASES for pages in (64, 4096)}


def validate(directory):
    stems = {f"sqlite-cache-{pages}__{case}" for case, pages in EXPECTED}
    for suffix in (".callgrind", ".annotated.txt"):
        actual = {p.name.removesuffix(suffix) for p in directory.glob(f"*{suffix}")}
        if actual != stems:
            raise ValueError(f"incorrect {suffix} case set: missing={stems - actual}, extra={actual - stems}")
    with (directory / "SUMMARY.tsv").open(newline="") as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        if reader.fieldnames != ["case", "sqlite_cache_pages", "events", "summary", "profile_bytes"]:
            raise ValueError("incorrect manifest columns")
        rows = list(reader)
    pairs = [(r["case"], r["sqlite_cache_pages"]) for r in rows]
    if len(pairs) != 12 or set(pairs) != EXPECTED:
        raise ValueError("manifest must have exactly the 12 unique case pairs")
    for row in rows:
        stem = f"sqlite-cache-{row['sqlite_cache_pages']}__{row['case']}"
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
    parser.add_argument("--rust-export", type=Path)
    parser.add_argument("--kcachegrind-export", type=Path)
    args = parser.parse_args()
    if bool(args.rust_export) != bool(args.kcachegrind_export):
        parser.error("both exporters must be supplied together")
    paths = validate(args.directory)
    counts = Counter()
    for path in paths:
        subprocess.run([str(args.inspect.resolve()), str(path)], check=True)
        if args.rust_export:
            result = compare(run_export(args.rust_export.resolve(), path), run_export(args.kcachegrind_export.resolve(), path))
            counts.update(result)
            print(path.name, result, flush=True)
    print(f"validated 12 profiles, 12 annotations, 12 manifest rows; compared {dict(counts)}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
