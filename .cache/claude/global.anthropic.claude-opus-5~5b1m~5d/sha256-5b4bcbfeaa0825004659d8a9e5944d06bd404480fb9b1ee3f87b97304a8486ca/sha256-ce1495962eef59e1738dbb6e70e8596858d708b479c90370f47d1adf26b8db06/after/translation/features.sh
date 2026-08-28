#!/bin/sh
# Phase D: run the whole differential suite under EVERY feature combination.
#
# The feature set is extracted from Cargo.toml rather than hardcoded, so this
# stays correct if features are ever added.
set -e
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"

feats=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                    if (a[1] != "default" && a[1] != "") print a[1] }
' Cargo.toml | sort -u)

echo "features declared in Cargo.toml: [$(echo $feats | tr '\n' ' ')]"

run() {
  desc="$1"; shift
  echo "=============================================================="
  echo "== $desc   (cargo flags: $*)"
  echo "=============================================================="
  cargo check --offline --all-targets "$@"
  ./run_tests.sh "$@" 2>&1 | grep -E "^(test result|error|     Running)"
}

# combination 0: default features
run "default features"

# combination 1: no default features
run "--no-default-features" --no-default-features

# every non-empty subset of the declared (non-default) features
set -- $feats
n=$#
if [ "$n" -gt 0 ]; then
  total=$(( (1 << n) - 1 ))
  m=1
  while [ "$m" -le "$total" ]; do
    combo=""
    i=1
    for f in $feats; do
      bit=$(( (m >> (i - 1)) & 1 ))
      [ "$bit" -eq 1 ] && combo="$combo,$f"
      i=$((i + 1))
    done
    combo=${combo#,}
    run "features=$combo" --no-default-features --features "$combo"
    m=$((m + 1))
  done
else
  echo
  echo "No [features] table in Cargo.toml: 'default' and '--no-default-features'"
  echo "are the only two configurations, and both were just exercised above."
fi
