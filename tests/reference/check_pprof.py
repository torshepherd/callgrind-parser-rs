#!/usr/bin/env python3
"""Check upstream fixtures through Rust and independent KCachegrind readers."""
import argparse
from pathlib import Path
import subprocess

from compare import compare, parse

# Input-derived expectations, independent of both Callgrind readers.
# Totals; graph edge count; context-tree edge count.
CASES = {
    "basic": ((100,), 1, 1), "flat": ((100,), 0, 0),
    "ambiguous-a": ((8,), 6, 6), "ambiguous-b": ((8,), 6, 6),
    "recursive": ((9,), 2, 2), "mutual": ((7,), 3, 5),
    "objects": ((90,), 1, 1), "inline": ((9,), 2, 2),
    "units": ((4000,), 0, 0), "large": ((9007199254740993,), 0, 0),
    "multi": ((2, 100), 1, 1),
}

# Root-first sampled paths and unscaled vectors from probe.go, not exporter output.
PATHS = {
    "basic": [(('main',), (10,)), (('main', 'foo'), (90,))],
    "flat": [(('main',), (10,)), (('foo',), (90,))],
    "ambiguous-a": [(('main', 'A', 'M', 'X'), (4,)), (('main', 'B', 'M', 'Y'), (4,))],
    "ambiguous-b": [(('main', 'B', 'M', 'X'), (4,)), (('main', 'A', 'M', 'Y'), (4,))],
    "recursive": [(('main',), (2,)), (('main', 'recur', 'recur'), (7,))],
    "mutual": [(('main', 'A', 'B', 'A', 'B', 'A'), (7,))],
    "objects": [(('main', 'foo'), (90,))],
    "inline": [(('main', 'outer', 'inner'), (9,))],
    "units": [(('a',), (1500,)), (('b',), (2500,))],
    "large": [(('large',), (9007199254740993,))],
    "multi": [(('main', 'foo'), (2, 100))],
}


def add(rows, key, costs):
    rows[key] = tuple(a + b for a, b in zip(rows.get(key, (0,) * len(costs)), costs, strict=True))


def base(identity):
    return identity[2].split(" [pprof:", 1)[0]


def check(export, name, mode):
    totals, graph_edges, tree_edges = CASES[name]
    if export.events != {0: tuple(f"E{i}" for i in range(len(totals)))}:
        raise ValueError(f"{name}: event columns changed")
    if export.rows[("T", 0)][1] != totals:
        raise ValueError(f"{name}: exact self totals changed")
    edges = [(key, value) for key, value in export.rows.items() if key[0] == "E"]
    if len(edges) != (graph_edges if mode == "graph" else tree_edges):
        raise ValueError(f"{name}/{mode}: edge count changed: {len(edges)}")
    if any(count != 0 for _, (count, _) in edges):
        raise ValueError("invented invocation count")
    if name == "objects":
        key, (_, costs) = edges[0]
        if key[2] != "/audit/app" or key[5] != "/audit/lib.so" or costs != (90,):
            raise ValueError("cross-object callee attribution changed")
    functions = {key[2:5]: value[1] for key, value in export.rows.items() if key[0] == "F"}
    expected, actual = {}, {}
    if mode == "tree":
        parents = {}
        for key, _ in edges:
            if key[2:5] not in functions or key[5:8] not in functions:
                raise ValueError("tree edge points at a nonexistent context")
            if key[5:8] in parents:
                raise ValueError("tree context has multiple parents")
            parents[key[5:8]] = key[2:5]
        paths = {}
        for identity in functions:
            chain, seen, current = [], set(), identity
            while current is not None:
                if current in seen:
                    raise ValueError("context tree contains a cycle")
                seen.add(current)
                chain.append(base(current))
                current = parents.get(current)
            paths[identity] = tuple(reversed(chain))
        for identity, costs in functions.items():
            if any(costs):
                add(actual, paths[identity], costs)
        for path, costs in PATHS[name]:
            add(expected, path, costs)
        for key, (_, costs) in edges:
            prefix = paths[key[5:8]]
            cumulative = tuple(sum(v[i] for path, v in PATHS[name] if path[:len(prefix)] == prefix) for i in range(len(totals)))
            if costs != cumulative:
                raise ValueError("tree edge lost exact subtree cost")
    else:
        for identity, costs in functions.items():
            if any(costs):
                add(actual, base(identity), costs)
        for path, costs in PATHS[name]:
            add(expected, path[-1], costs)
        expected_edges, actual_edges = {}, {}
        for path, costs in PATHS[name]:
            for edge in set(zip(path, path[1:])):
                add(expected_edges, edge, costs)
        for key, (_, costs) in edges:
            add(actual_edges, (base(key[2:5]), base(key[5:8])), costs)
        if actual_edges != expected_edges:
            raise ValueError("graph sampled-edge weights changed")
    if actual != expected:
        raise ValueError("exclusive costs or observed stack paths changed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profiles", type=Path)
    parser.add_argument("rust", type=Path)
    parser.add_argument("kcachegrind", type=Path)
    args = parser.parse_args()
    if args.rust.resolve() == args.kcachegrind.resolve():
        raise ValueError("independent readers must be different executables")
    for name in CASES:
        for mode in ("graph", "tree"):
            profile = args.profiles / f"{name}.rust-{mode}.callgrind"
            exports = []
            for label, executable in (("rust", args.rust), ("kcachegrind", args.kcachegrind)):
                result = subprocess.run([str(executable.resolve()), str(profile)], capture_output=True, text=True, timeout=30)
                profile.with_suffix(f".{label}.tsv").write_text(result.stdout)
                profile.with_suffix(f".{label}.stderr").write_text(result.stderr)
                result.check_returncode()
                exports.append(parse(result.stdout))
            check(exports[0], name, mode)
            print(name, mode, compare(*exports))
    a, b = (args.profiles / f"ambiguous-{n}.rust-graph.callgrind" for n in ("a", "b"))
    if a.read_bytes() != b.read_bytes():
        raise ValueError("ordinary graphs should coincide")
    a, b = (args.profiles / f"ambiguous-{n}.rust-tree.callgrind" for n in ("a", "b"))
    if a.read_bytes() == b.read_bytes():
        raise ValueError("context trees must distinguish stack populations")
    print("22 independent KCachegrind comparisons passed")


if __name__ == "__main__":
    main()
