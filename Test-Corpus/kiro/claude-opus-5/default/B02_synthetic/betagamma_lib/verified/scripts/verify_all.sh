#!/usr/bin/env bash
# Full verification driver: builds the C .so, then runs every phase under every
# feature combination.
#
# CRITICAL: `cargo build` MUST run before `cargo test`. The crate is a
# `cdylib`, and `cargo test` does not rebuild cdylib artifacts (integration
# tests dlopen the .so rather than linking it), so without an explicit build
# step the suite would verify a stale library. tests/common/mod.rs also
# hard-fails on a stale .so as a second line of defence.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE_DIR="$PWD"
ROOT="$(cd .. && pwd)"

TIMEOUT=${TIMEOUT:-600}
rc_total=0

# --- 1. Build the C reference shared library ------------------------------
if ! ls "$ROOT"/c_src/build/*.so >/dev/null 2>&1; then
  echo "== building c_src =="
  ( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . ) || exit 1
fi
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C  .so: $C_SO"

# --- 2. Enumerate feature combinations from Cargo.toml --------------------
# Read the [features] table (excluding `default`) and build the powerset.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table => default and --no-default-features are the only
  # (and identical) configurations. Both are still run.
  COMBOS+=("default:")
  COMBOS+=("no-default:")
else
  COMBOS+=("default:")
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then sel="${sel:+$sel,}${FEATURES[i]}"; fi
    done
    COMBOS+=("no-default:$sel")
  done
fi

echo "feature combinations to verify: ${#COMBOS[@]}"
printf '  - %s\n' "${COMBOS[@]}"

# --- 3. For each combination: build the cdylib, then run all phases -------
for combo in "${COMBOS[@]}"; do
  kind="${combo%%:*}"
  feats="${combo#*:}"

  args=()
  [ "$kind" = "no-default" ] && args+=(--no-default-features)
  [ -n "$feats" ] && args+=(--features "$feats")

  echo
  echo "=================================================================="
  echo "== CONFIG: ${kind}${feats:+ features=$feats}"
  echo "=================================================================="

  echo "-- cargo check"
  if ! timeout "$TIMEOUT" cargo check --release "${args[@]}" 2>&1 | tail -5; then
    echo "!! cargo check FAILED for $combo"; rc_total=1; continue
  fi

  echo "-- cargo build --release (MUST precede cargo test: cdylib is dlopen'd)"
  if ! timeout "$TIMEOUT" cargo build --release "${args[@]}" 2>&1 | tail -5; then
    echo "!! cargo build FAILED for $combo"; rc_total=1; continue
  fi

  R_SO="$CRATE_DIR/target/release/libbetagamma_lib.so"

  echo "-- symbol diff (nm -D)"
  missing="$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u))"
  if [ -n "$missing" ]; then
    echo "!! symbols present in C .so but MISSING from Rust .so:"
    echo "$missing" | sed 's/^/     /'
    rc_total=1
  else
    echo "   OK: 0 missing symbols"
  fi

  echo "-- phases A/B/C/D tests"
  if ! timeout "$TIMEOUT" cargo test --release "${args[@]}" -- --test-threads=1 2>&1 \
        | grep -E "^(test result:|error|warning: unused|test [a-z_0-9]+ \.\.\. FAILED)"; then
    :
  fi
  # Re-run capturing the true exit status (the pipe above masks it).
  timeout "$TIMEOUT" cargo test --release "${args[@]}" -- --test-threads=1 >/tmp/vt.log 2>&1
  st=$?
  if [ $st -ne 0 ]; then
    echo "!! TESTS FAILED for $combo (exit $st)"
    tail -40 /tmp/vt.log
    rc_total=1
  else
    grep -E "^test result:" /tmp/vt.log | sed 's/^/   /'
  fi
done

echo
if [ $rc_total -eq 0 ]; then
  echo "ALL CONFIGURATIONS VERIFIED"
else
  echo "VERIFICATION FAILED"
fi
exit $rc_total
