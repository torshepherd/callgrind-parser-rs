#!/usr/bin/env python3
"""Profile a declarative workload plan; retain commands and partial failure output."""
import argparse
import csv
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from check_matrix import CONFIGURATIONS, load_plan


def run(plan_path, out, valgrind, reference):
    plan = load_plan(plan_path)
    out = out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    # Never mix old, partial or different workload results into a new run.
    if any(out.iterdir()):
        raise ValueError(f"output directory must be empty: {out}")
    (out / "PLAN.json").write_text(json.dumps(plan, indent=2) + "\n")
    env = dict(os.environ, LC_ALL="C", TZ="UTC")
    timeout = plan["timeout_seconds"]
    with (out / "SUMMARY.tsv").open("w", newline="") as manifest:
        writer = csv.writer(manifest, delimiter="\t")
        writer.writerow(["case", "variant", "events", "summary", "profile_bytes"])
        manifest.flush()
        for variant in plan["variants"]:
            stdin = os.devnull
            if "stdin" in variant:
                inputs = out / "inputs"
                inputs.mkdir(exist_ok=True)
                stdin = str(inputs / (variant["name"] + ".stdin"))
                shutil.copyfile(variant["stdin"], stdin)
            for case in plan["configurations"]:
                stem = f"{variant['name']}__{case}"
                work = out / "work" / stem
                work.mkdir(parents=True)
                profile = out / f"{stem}.callgrind"
                command = [valgrind, "--tool=callgrind", "--error-exitcode=99",
                           f"--callgrind-out-file={profile}",
                           f"--log-file={out / (stem + '.valgrind.log')}",
                           *CONFIGURATIONS[case], *variant["argv"]]
                (out / f"{stem}.command.json").write_text(json.dumps(
                    dict(argv=command, stdin=stdin, cwd=str(work), LC_ALL="C", TZ="UTC"), indent=2) + "\n")
                with open(stdin, "rb") as source, (out / f"{stem}.stdout.txt").open("wb") as stdout, \
                        (out / f"{stem}.stderr.txt").open("wb") as stderr:
                    subprocess.run(command, stdin=source, stdout=stdout, stderr=stderr,
                                   cwd=work, env=env, check=True, timeout=timeout)
                headers = {}
                with profile.open() as stream:
                    for line in stream:
                        for key in ("version", "events", "summary"):
                            if line.startswith(key + ":"):
                                headers.setdefault(key, line.split(":", 1)[1].strip())
                if not all(headers.get(key) for key in ("version", "events", "summary")):
                    raise ValueError(f"{stem}: missing profile headers")
                with (out / f"{stem}.annotated.txt").open("wb") as stdout, \
                        (out / f"{stem}.annotated.stderr.txt").open("wb") as stderr:
                    subprocess.run([reference, "--auto=no", "--inclusive=yes", "--threshold=99", str(profile)],
                                   stdout=stdout, stderr=stderr, cwd=out, env=env, check=True, timeout=timeout)
                writer.writerow([case, variant["name"], headers["events"], headers["summary"], profile.stat().st_size])
                manifest.flush()
                print(f"completed {stem}: {headers['events']}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--valgrind", default="valgrind")
    parser.add_argument("--reference", default="callgrind_annotate")
    args = parser.parse_args()
    tools = [shutil.which(tool) for tool in (args.valgrind, args.reference)]
    if not all(tools):
        parser.error("valgrind and callgrind_annotate executables are required")
    run(args.plan, args.out, *tools)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        sys.exit(str(error))
