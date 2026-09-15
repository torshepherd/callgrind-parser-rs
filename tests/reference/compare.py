#!/usr/bin/env python3
"""Strict test-boundary TSV comparison; never parses Callgrind grammar."""
import argparse
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
import re
import subprocess
import sys


def integer(text):
    if not re.fullmatch(r"[0-9]+", text):
        raise ValueError(f"invalid unsigned integer: {text!r}")
    return int(text)


def unhex(text):
    if not re.fullmatch(r"(?:[0-9a-fA-F]{2})*", text):
        raise ValueError(f"invalid hex: {text!r}")
    return bytes.fromhex(text).decode("utf-8", errors="strict")


@dataclass
class Export:
    events: dict
    rows: dict


def parse(text):
    events, rows = {}, {}
    widths = {"P": 3, "T": 3, "F": 6, "I": 6, "E": 10, "L": 8}
    for number, line in enumerate(text.splitlines(), 1):
        fields = line.split("\t")
        tag = fields[0]
        if tag not in widths or len(fields) != widths[tag]:
            raise ValueError(f"line {number}: malformed {tag!r} record")
        part = integer(fields[1])
        if tag == "P":
            names = tuple(unhex(s) for s in fields[2].split(","))
            if part in events or not all(names) or len(names) != len(set(names)):
                raise ValueError(f"line {number}: duplicate part/event or empty event")
            events[part] = names
            continue
        if part not in events:
            raise ValueError(f"line {number}: record before part declaration")
        values = tuple(integer(s) for s in fields[-1].split(","))
        if len(values) != len(events[part]):
            raise ValueError(f"line {number}: vector width mismatch")
        key = (tag, part)
        count = None
        if tag in ("F", "I"):
            key += tuple(unhex(s) for s in fields[2:5])
        elif tag == "E":
            key += tuple(unhex(s) for s in fields[2:8])
            count = integer(fields[8])
        elif tag == "L":
            key += tuple(unhex(s) for s in fields[2:6]) + (integer(fields[6]),)
        if key in rows:
            raise ValueError(f"line {number}: duplicate row {key!r}")
        rows[key] = (count, values)
    if not events or set(events) != set(range(len(events))):
        raise ValueError("part indexes must be nonempty, contiguous, and zero-based")
    if any(("T", part) not in rows for part in events):
        raise ValueError("missing part total")
    return Export(events, rows)


def normalize(export, layout):
    rows = {}
    for key, (count, costs) in export.rows.items():
        if key[0] == "I":
            continue
        if key[0] != "T" and not count and not any(costs):
            continue
        by_name = dict(zip(export.events[key[1]], costs, strict=True))
        rows[key] = (count, tuple(by_name.get(name, 0) for name in layout))
    return rows


def compare(left, right):
    if left.events.keys() != right.events.keys():
        raise ValueError("part sets differ")
    left_union = set().union(*map(set, left.events.values()))
    right_union = set().union(*map(set, right.events.values()))
    if left_union != right_union:
        raise ValueError(f"event sets differ: {left_union!r} != {right_union!r}")
    # A reference may expose its entire dataset layout in each part. Missing
    # local events can be zero-filled only when the extra columns are zero.
    for part in left.events:
        for a, b in ((left, right), (right, left)):
            missing = set(b.events[part]) - set(a.events[part])
            for key, (_, values) in b.rows.items():
                if key[1] == part and key[0] != "I":
                    if any(v for n, v in zip(b.events[part], values, strict=True) if n in missing):
                        raise ValueError(f"part {part}: unmeasured event has nonzero costs")
    layout = sorted(left_union)
    a, b = normalize(left, layout), normalize(right, layout)
    differences = [key for key in sorted(a.keys() | b.keys()) if a.get(key) != b.get(key)]
    if differences:
        detail = "\n".join(f"{key!r}: {a.get(key)!r} != {b.get(key)!r}" for key in differences[:8])
        raise ValueError(f"{len(differences)} mismatched rows\n{detail}")
    return dict(Counter(key[0] for key in a))


def run_export(executable, profile):
    result = subprocess.run([str(executable), str(profile)], capture_output=True, text=True, check=True)
    if result.stderr:
        print(result.stderr, file=sys.stderr, end="")
    return parse(result.stdout)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("left", type=Path)
    parser.add_argument("right", type=Path)
    args = parser.parse_args()
    print(compare(parse(args.left.read_text()), parse(args.right.read_text())))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
