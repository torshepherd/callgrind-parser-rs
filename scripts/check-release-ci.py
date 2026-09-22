#!/usr/bin/env python3
"""Require completed native and Rust CI for the exact release commit."""

import argparse
import json
import os
import subprocess


def api(path):
    return json.loads(subprocess.check_output(["gh", "api", path], text=True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sha")
    parser.add_argument("--require-head", action="store_true")
    args = parser.parse_args()
    repo = os.environ.get("GITHUB_REPOSITORY", "torshepherd/callgrind-parser-rs")
    sha = subprocess.check_output(
        ["git", "rev-parse", f"{args.sha}^{{commit}}"], text=True
    ).strip()
    if args.require_head:
        head = api(f"repos/{repo}/git/ref/heads/master")["object"]["sha"]
        if head != sha:
            raise SystemExit("Release commit is no longer the master head; rerun CI there.")
    runs = api(
        f"repos/{repo}/actions/workflows/ci.yml/runs?head_sha={sha}&event=push&per_page=100"
    )["workflow_runs"]
    runs = [run for run in runs if run["head_branch"] == "master"]
    if not runs:
        raise SystemExit(f"No push CI run on master for {sha}")
    run = max(runs, key=lambda item: item["id"])
    if run["status"] != "completed" or run["conclusion"] != "success":
        raise SystemExit(f"Release requires successful CI: {run['html_url']}")
    jobs = api(f"repos/{repo}/actions/runs/{run['id']}/jobs?per_page=100")["jobs"]
    passed = {
        job["name"] for job in jobs
        if job["status"] == "completed" and job["conclusion"] == "success"
    }
    required = {
        "Rust and Python tests",
        "Callgrind integration (sqlite)",
        "Pprof and KCachegrind integration",
    }
    if not required <= passed:
        raise SystemExit(f"Missing successful jobs: {sorted(required - passed)}")
    print(f"Verified release commit {sha}: {run['html_url']}")


if __name__ == "__main__":
    main()
