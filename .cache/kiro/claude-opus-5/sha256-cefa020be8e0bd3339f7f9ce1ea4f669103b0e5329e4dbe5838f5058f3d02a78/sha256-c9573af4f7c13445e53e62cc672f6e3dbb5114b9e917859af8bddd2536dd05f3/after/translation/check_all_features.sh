#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY feature combination.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so the
# sweep stays correct if features are added later. For each combination the
# release `.so` is rebuilt first, because that is the artifact the tests dlopen.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# --- enumerate declared features (excluding "default") ----------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the combination list --------------------------------------------
# Always include the default build and the empty (no-default-features) build.
COMBOS=("--all-features" "" "--no-default-features")

n=${#FEATURES[@]}
if (( n > 0 )); then
  # Full power set of the declared features, with default features off.
  for (( mask = 1; mask < (1 << n); mask++ )); do
    sel=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        sel="${sel:+$sel,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("--no-default-features --features $sel")
    COMBOS+=("--features $sel")
  done
fi

# --- run ---------------------------------------------------------------------
fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  echo
  echo "=============================================================="
  echo "combination: $label"
  echo "=============================================================="

  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release $combo > /tmp/fc-build.log 2>&1; then
    echo "BUILD FAILED for $label"; tail -30 /tmp/fc-build.log; fail=1; continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check $combo > /tmp/fc-check.log 2>&1; then
    echo "CHECK FAILED for $label"; tail -30 /tmp/fc-check.log; fail=1; continue
  fi

  # Symbol parity is per-combination: a feature-gated export would break it.
  c_syms=$(nm -D --defined-only ../c_src/build/libdriver.so | awk '$2 ~ /^[TDBWVR]$/ {print $3}' | sort)
  r_syms=$(nm -D --defined-only target/release/libdriver.so | awk '$2 ~ /^[TDBWVR]$/ {print $3}' | sort)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [[ -n "$missing" ]]; then
    echo "SYMBOL PARITY FAILED for $label; missing from Rust .so:"; echo "$missing"; fail=1; continue
  fi
  echo "symbol parity: OK (C exports: $(echo "$c_syms" | tr '\n' ' '))"

  # shellcheck disable=SC2086
  if ! timeout 600 cargo test $combo > /tmp/fc-test.log 2>&1; then
    echo "TESTS FAILED for $label"; grep -E "^test .* FAILED|test result|panicked" /tmp/fc-test.log | head -40; fail=1; continue
  fi
  grep -E "test result" /tmp/fc-test.log | sed 's/^/  /'
  echo "TESTS PASSED for $label"
done

echo
if (( fail )); then
  echo "RESULT: at least one feature combination FAILED"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} feature combinations PASSED"
