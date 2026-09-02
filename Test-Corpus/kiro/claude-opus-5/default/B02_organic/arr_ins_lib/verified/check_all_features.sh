#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY cargo feature
# combination declared in Cargo.toml (plus --no-default-features and the
# default), rebuilding the cdylib each time because the tests dlopen it.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

# ---- enumerate optional features from Cargo.toml -------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [[ ${#FEATURES[@]} -eq 0 ]]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default."
  COMBOS+=("DEFAULT")
  COMBOS+=("NODEFAULT")
else
  n=${#FEATURES[@]}
  COMBOS+=("DEFAULT" "NODEFAULT")
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo+="${FEATURES[$i]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
fi

fail=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT)    flags=() ; label="(default features)" ;;
    NODEFAULT)  flags=(--no-default-features) ; label="--no-default-features" ;;
    *)          flags=(--no-default-features --features "$combo") ; label="--features $combo" ;;
  esac

  echo "================================================================"
  echo "== configuration: $label"
  echo "================================================================"

  if ! timeout 600 cargo build --release "${flags[@]}" >/tmp/dp_build.log 2>&1; then
    echo "BUILD FAILED"; tail -20 /tmp/dp_build.log; fail=1; continue
  fi
  if ! ./check_symbols.sh > /tmp/dp_sym.log 2>&1; then
    echo "SYMBOL PARITY FAILED"; tail -20 /tmp/dp_sym.log; fail=1; continue
  fi
  tail -1 /tmp/dp_sym.log
  # --test-threads=1: both .so's have process-global state (stbds_hash_seed,
  # strkey's static buffer) and dlopen returns the same handle to every test.
  for suite in phase_b phase_c phase_d; do
    if timeout 600 cargo test --release "${flags[@]}" --test "$suite" -- --test-threads=1 \
         > "/tmp/dp_${suite}.log" 2>&1; then
      echo "  $suite: $(grep -E '^test result' "/tmp/dp_${suite}.log" | tail -1)"
    else
      echo "  $suite: FAILED"; tail -30 "/tmp/dp_${suite}.log"; fail=1
    fi
  done
done

echo
if [[ $fail -eq 0 ]]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit $fail
