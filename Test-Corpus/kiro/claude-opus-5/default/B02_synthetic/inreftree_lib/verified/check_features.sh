#!/usr/bin/env bash
# Phase D: run the whole verification under EVERY cargo feature combination.
#
# The feature list is parsed out of Cargo.toml rather than hard-coded, so a
# feature added later is picked up automatically.
set -uo pipefail
cd "$(dirname "$0")"

# Features declared in [features], excluding "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/      { inblock = 1; next }
    /^\[/                { inblock = 0 }
    inblock && /=/       { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                           if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"
echo

# Every combination to test, as a whitespace-separated argument string.
# The empty string means "default features".
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  # No features exist, so default + no-default-features is the whole space.
  COMBOS+=("--no-default-features")
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    if [ ${#sel[@]} -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      joined=$(
        IFS=,
        echo "${sel[*]}"
      )
      COMBOS+=("--no-default-features --features $joined")
    fi
  done
fi

status=0
for combo in "${COMBOS[@]}"; do
  read -r -a args <<<"$combo"
  echo "=============================================================="
  echo "combination: ${combo:-<default features>}"
  echo "=============================================================="
  if ! timeout 600 cargo build --release "${args[@]}" >/tmp/feat_build.log 2>&1; then
    echo "  BUILD FAILED"
    tail -20 /tmp/feat_build.log
    status=1
    continue
  fi
  ./check_symbols.sh | grep -E "symbol diff|undefined non-libc" | sed 's/^/  /' || status=1
  if timeout 600 cargo test --release "${args[@]}" >/tmp/feat_test.log 2>&1; then
    grep -E "^test result:" /tmp/feat_test.log | sed 's/^/  /'
  else
    echo "  TESTS FAILED"
    grep -E "^test .* FAILED|panicked at" /tmp/feat_test.log | head -10
    status=1
  fi
  echo
done

if [ $status -eq 0 ]; then
  echo "ALL ${#COMBOS[@]} feature combination(s) pass."
else
  echo "at least one feature combination FAILED"
fi
exit $status
