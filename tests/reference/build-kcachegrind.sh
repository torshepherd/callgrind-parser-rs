#!/usr/bin/env bash
set -euo pipefail
[[ $# -ge 2 && $# -le 3 ]] || { echo "usage: $0 KCACHEGRIND_CHECKOUT OUTPUT [EXTRACTED_QT_PREFIX]" >&2; exit 2; }
source_dir=$(cd "$1" && pwd)
output=$2
pin=764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14
[[ $(git -C "$source_dir" rev-parse HEAD) == "$pin" ]] || { echo 'incorrect KCachegrind revision' >&2; exit 1; }
[[ -z $(git -C "$source_dir" status --porcelain --untracked-files=all -- libcore) ]] || { echo 'libcore must be unmodified' >&2; exit 1; }
adapter=$(cd "$(dirname "$0")" && pwd)/kcachegrind-export.cpp
cppflags=()
ldflags=()
if [[ $# == 3 ]]; then
  qt=$(cd "$3" && pwd)
  include="$qt/usr/include/x86_64-linux-gnu/qt6"
  lib="$qt/usr/lib/x86_64-linux-gnu"
  cppflags+=("-I$include" "-I$include/QtCore")
  ldflags+=("-L$lib" "-Wl,-rpath,$lib" "-Wl,-rpath-link,$lib" -lQt6Core)
else
  read -r -a cppflags <<< "$(pkg-config --cflags Qt6Core)"
  read -r -a ldflags <<< "$(pkg-config --libs Qt6Core)"
fi
sources=()
for unit in context costitem eventtype subcost addr tracedata loader cachegrindloader fixcost pool coverage stackbrowser utils logger config globalconfig; do
  sources+=("$source_dir/libcore/$unit.cpp")
done
mkdir -p "$(dirname "$output")"
"${CXX:-c++}" -std=c++17 -O2 -fPIC -I"$source_dir/libcore" "${cppflags[@]}" \
  "$adapter" "${sources[@]}" "${ldflags[@]}" -o "$output"
