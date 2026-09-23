#!/usr/bin/env python3
"""Dispatch one conventional version-tag release for our shared-version CLIs."""

import json
import os
import subprocess
import tomllib
from pathlib import Path

BINARIES = {"callgrind-annotate-rs", "pprof2callgrind", "callgrind2pprof"}


def release_tag(releases, workspace_version):
    versions = {r["version"] for r in releases if r["package_name"] in BINARIES}
    if not versions:
        return None
    if versions != {workspace_version}:
        raise ValueError("Binary releases must share the workspace version")
    return f"v{workspace_version}"


def ensure_tag(repo, tag, sha):
    ref = f"repos/{repo}/git/ref/tags/{tag}"
    existing = subprocess.run(["gh", "api", ref], text=True, capture_output=True)
    if existing.returncode == 0:
        obj = json.loads(existing.stdout)["object"]
        if obj["type"] != "commit" or obj["sha"] != sha:
            raise ValueError(f"Refusing to move existing tag {tag}")
    elif "HTTP 404" in existing.stderr:
        subprocess.run(["gh", "api", f"repos/{repo}/git/refs", "--method", "POST",
                        "-f", f"ref=refs/tags/{tag}", "-f", f"sha={sha}"], check=True)
    else:
        raise RuntimeError(existing.stderr)


def main():
    version = tomllib.loads(Path("Cargo.toml").read_text())["workspace"]["package"]["version"]
    tag = release_tag(json.loads(os.environ["RELEASES"]), version)
    if tag is None:
        print("No CLI packages released; no binary build needed.")
        return
    sha = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    ensure_tag(os.environ["GITHUB_REPOSITORY"], tag, sha)
    subprocess.run(["gh", "workflow", "run", "release.yml", "--ref", tag,
                    "-f", f"tag={tag}"], check=True)


if __name__ == "__main__":
    main()
