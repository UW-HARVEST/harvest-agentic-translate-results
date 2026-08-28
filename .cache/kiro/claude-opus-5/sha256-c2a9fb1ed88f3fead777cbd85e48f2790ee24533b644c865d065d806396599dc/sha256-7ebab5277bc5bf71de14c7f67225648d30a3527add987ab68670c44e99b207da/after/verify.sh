#!/usr/bin/env bash
# Differential verification of translation/ against c_src/ for every build-time
# configuration.
#
#   * Cargo feature combinations are derived from translation/Cargo.toml.
#     If the crate declares no [features], the single valid combination is the
#     default (empty) one.
#   * Each combination is checked, then built as a cdylib, then exercised by the
#     integration tests in translation/tests/ against the C .so.
#   * Both the release and the dev cdylib are tested: the dev profile enables
#     debug assertions and arithmetic overflow checks, so it catches wrapping
#     that was written as plain `+`/`-`/`*`.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOGDIR="${TMPDIR:-/tmp}/hatch-verify"
mkdir -p "$LOGDIR"

fail=0
step() { printf '\n=== %s ===\n' "$*"; }
run() { # run <logname> <cmd...>
  local log="$LOGDIR/$1.log"; shift
  if timeout 600 "$@" >"$log" 2>&1; then
    echo "  PASS  $* (log: $log)"
  else
    echo "  FAIL  $* (log: $log)"
    tail -n 40 "$log"
    fail=1
  fi
}

# ---------------------------------------------------------------------------
step "Enumerating feature combinations from Cargo.toml"
# Feature names in the [features] table, minus the "default" meta-feature.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); gsub(/[[:space:]]/, "");
      if ($0 != "default") print
    }
  ' "$CRATE/Cargo.toml"
)
echo "  declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Every subset of FEATURES, as a comma-separated string ("" = no features).
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
  [ -z "$f" ] && continue
  for existing in "${COMBOS[@]}"; do
    COMBOS+=("${existing:+$existing,}$f")
  done
done
echo "  combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
step "Building the C shared library"
run c-cmake-configure bash -c "cd '$ROOT/c_src' && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON"
run c-cmake-build bash -c "cd '$ROOT/c_src/build' && cmake --build ."
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -n1)"
echo "  C .so: $C_SO"

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  slug="$(echo "${combo:-default}" | tr ',' '_')"
  feat_args=(--no-default-features)
  [ -n "$combo" ] && feat_args+=(--features "$combo")

  step "Feature combination: $label"

  run "check-$slug"        bash -c "cd '$CRATE' && cargo check ${feat_args[*]} --all-targets"
  run "build-rel-$slug"    bash -c "cd '$CRATE' && cargo build --release ${feat_args[*]}"
  run "build-dev-$slug"    bash -c "cd '$CRATE' && cargo build ${feat_args[*]}"

  step "  Symbol parity ($label)"
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u >"$LOGDIR/c.syms"
  for prof in release debug; do
    so="$CRATE/target/$prof/libhatch_lib.so"
    nm -D --defined-only "$so" | awk '{print $3}' | sort -u >"$LOGDIR/rust-$prof.syms"
    missing="$(comm -23 "$LOGDIR/c.syms" "$LOGDIR/rust-$prof.syms" \
      | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$')"
    if [ -z "$missing" ]; then
      echo "  PASS  $prof cdylib exports every C symbol"
    else
      echo "  FAIL  $prof cdylib is missing:"; echo "$missing" | sed 's/^/          /'
      fail=1
    fi
  done

  step "  Differential tests ($label)"
  run "test-rel-$slug" bash -c "cd '$CRATE' && HATCH_RUST_SO='$CRATE/target/release/libhatch_lib.so' cargo test --release ${feat_args[*]}"
  # Dev profile: debug_assertions + overflow checks are on inside the cdylib.
  run "test-dev-$slug" bash -c "cd '$CRATE' && HATCH_RUST_SO='$CRATE/target/debug/libhatch_lib.so' cargo test --release ${feat_args[*]}"
done

step "Summary"
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS MATCH THE C IMPLEMENTATION"
else
  echo "FAILURES PRESENT — see logs in $LOGDIR"
fi
exit "$fail"
