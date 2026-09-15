#!/usr/bin/env bash
# Compatibility entry point for the original native SQLite matrix.
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
fixture=""
workload=""
out=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --fixture) fixture="$2"; shift 2 ;;
    --workload) workload="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    -h|--help) echo "Usage: run-matrix.sh --fixture DB --workload SQL_TEMPLATE --out EMPTY_DIRECTORY"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
[[ -f "$fixture" && -f "$workload" && -n "$out" ]] || { echo "--fixture DB, --workload SQL_TEMPLATE and --out are required" >&2; exit 2; }
plan_dir="$(mktemp -d)"
trap 'rm -rf -- "$plan_dir"' EXIT
python3 - "$fixture" "$workload" "$plan_dir" <<'PYTHON'
import json
from pathlib import Path
import shutil
import sys
fixture, template, root = map(lambda p: Path(p).resolve(), sys.argv[1:])
sqlite = shutil.which("sqlite3")
if not sqlite:
    sys.exit("sqlite3 executable is required")
variants = []
for pages in (64, 4096):
    sql = root / f"sqlite-cache-{pages}.sql"
    sql.write_text(template.read_text().replace("@SQLITE_CACHE_PAGES@", str(pages)))
    variants.append(dict(name=f"sqlite-cache-{pages}", argv=[sqlite, str(fixture)], stdin=str(sql)))
(root / "plan.json").write_text(json.dumps(dict(schema=1, name="sqlite", variants=variants)))
PYTHON
python3 "$repo_root/tests/reference/run_workload.py" --plan "$plan_dir/plan.json" --out "$out"
