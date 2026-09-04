#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: run-matrix.sh --fixture DB --workload SQL_TEMPLATE --out DIRECTORY

Runs six Callgrind configurations against two SQLite page-cache sizes. The
output directory receives raw profiles, reference callgrind_annotate output,
Valgrind logs, commands, SQLite output, and SUMMARY.tsv.
EOF
}

fixture=""
workload=""
out=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --fixture) fixture="$2"; shift 2 ;;
    --workload) workload="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ -n "$fixture" && -f "$fixture" ]] || { echo "missing --fixture DB" >&2; exit 2; }
[[ -n "$workload" && -f "$workload" ]] || { echo "missing --workload SQL_TEMPLATE" >&2; exit 2; }
[[ -n "$out" ]] || { echo "missing --out DIRECTORY" >&2; exit 2; }

for command in valgrind callgrind_annotate sqlite3 sed grep wc; do
  command -v "$command" >/dev/null || { echo "required command not found: $command" >&2; exit 2; }
done

mkdir -p "$out"
printf 'case\tsqlite_cache_pages\tevents\tsummary\tprofile_bytes\n' > "$out/SUMMARY.tsv"

run_case() {
  local name="$1"
  local sqlite_cache_pages="$2"
  shift 2

  local stem="sqlite-cache-${sqlite_cache_pages}__${name}"
  local sql="$out/sqlite-cache-${sqlite_cache_pages}.sql"
  local profile="$out/${stem}.callgrind"
  local annotation="$out/${stem}.annotated.txt"
  local log="$out/${stem}.valgrind.log"
  local stdout="$out/${stem}.sqlite-output.txt"
  local command_file="$out/${stem}.command.txt"

  if [[ ! -f "$sql" ]]; then
    sed "s/@SQLITE_CACHE_PAGES@/${sqlite_cache_pages}/g" "$workload" > "$sql"
  fi

  {
    printf '%q ' valgrind --tool=callgrind --error-exitcode=99 \
      "--callgrind-out-file=$profile" "--log-file=$log" "$@" sqlite3 "$fixture"
    printf '< %q\n' "$sql"
  } > "$command_file"

  valgrind \
    --tool=callgrind \
    --error-exitcode=99 \
    "--callgrind-out-file=$profile" \
    "--log-file=$log" \
    "$@" \
    sqlite3 "$fixture" < "$sql" > "$stdout"

  grep -q '^version:' "$profile"
  grep -q '^events:' "$profile"
  grep -q '^summary:' "$profile"

  callgrind_annotate \
    --inclusive=yes \
    --threshold=99 \
    "$profile" > "$annotation"

  grep -q 'PROGRAM TOTALS' "$annotation"

  local events summary bytes
  events="$(sed -n 's/^events: //p' "$profile" | head -n 1)"
  summary="$(sed -n 's/^summary: //p' "$profile" | head -n 1)"
  bytes="$(wc -c < "$profile" | tr -d ' ')"

  printf '%s\t%s\t%s\t%s\t%s\n' \
    "$name" "$sqlite_cache_pages" "$events" "$summary" "$bytes" \
    >> "$out/SUMMARY.tsv"

  echo "completed ${stem}: ${events}"
}

for sqlite_cache_pages in 64 4096; do
  run_case ir-only "$sqlite_cache_pages" \
    --cache-sim=no \
    --branch-sim=no

  run_case branches-and-jumps "$sqlite_cache_pages" \
    --cache-sim=no \
    --branch-sim=yes \
    --collect-jumps=yes

  run_case balanced-cache "$sqlite_cache_pages" \
    --cache-sim=yes \
    --branch-sim=no \
    --I1=32768,8,64 \
    --D1=32768,8,64 \
    --LL=8388608,16,64

  run_case constrained-cache "$sqlite_cache_pages" \
    --cache-sim=yes \
    --branch-sim=no \
    --I1=16384,4,64 \
    --D1=16384,4,64 \
    --LL=524288,8,64

  run_case roomy-cache "$sqlite_cache_pages" \
    --cache-sim=yes \
    --branch-sim=no \
    --I1=65536,8,64 \
    --D1=65536,8,64 \
    --LL=16777216,16,64

  run_case full-simulation "$sqlite_cache_pages" \
    --cache-sim=yes \
    --branch-sim=yes \
    --collect-jumps=yes \
    --collect-systime=usec \
    --simulate-wb=yes \
    --simulate-hwpref=yes \
    --cacheuse=yes \
    --I1=32768,8,64 \
    --D1=32768,8,64 \
    --LL=8388608,16,64
done

echo "verified 12 Callgrind profiles and 12 reference annotations"
