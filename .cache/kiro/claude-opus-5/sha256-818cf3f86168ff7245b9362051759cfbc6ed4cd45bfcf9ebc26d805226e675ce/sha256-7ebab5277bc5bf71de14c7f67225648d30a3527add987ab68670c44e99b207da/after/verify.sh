#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination for translation/ and runs a
# command for each. Usage: ./verify.sh check | ./verify.sh test | ./verify.sh nm
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CARGO_TOML="$ROOT/translation/Cargo.toml"
C_SO="$(ls "$ROOT"/c_src/build/lib*.so 2>/dev/null | head -1)"

# Pull the feature names out of the [features] table, ignoring "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "="); gsub(/[[:space:]]/, "", kv[1]);
      if (kv[1] != "default") print kv[1]
    }
  ' "$CARGO_TOML"
)

# Build the power set of feature names; the empty set is --no-default-features.
COMBOS=("")
for f in "${FEATURES[@]}"; do
  existing=("${COMBOS[@]}")
  for c in "${existing[@]}"; do
    if [[ -z "$c" ]]; then COMBOS+=("$f"); else COMBOS+=("$c,$f"); fi
  done
done

echo "Discovered features: ${FEATURES[*]:-<none>}"
echo "Combinations to verify: ${#COMBOS[@]} (plus the default feature set)"
echo

# Always verify the crate's own default set too, in case it differs from empty.
run_one() {
  local label="$1"; shift
  echo "--- $label ---"
  ( cd "$ROOT/translation" && timeout 600 "$@" )
  local rc=$?
  if [[ $rc -ne 0 ]]; then
    echo "FAILED ($rc): $label"
    return 1
  fi
  return 0
}

# Confirms the Rust cdylib exports every symbol the C .so exports.
compare_symbols() {
  local label="$1" rust_so="$2"
  local c_syms rust_syms missing
  c_syms="$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TtWwDdBbRr]$/ {print $3}' | sort -u)"
  rust_syms="$(nm -D --defined-only "$rust_so" | awk '$2 ~ /^[TtWwDdBbRr]$/ {print $3}' | sort -u)"
  missing="$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))"
  echo "--- symbols: $label ---"
  echo "C exports:    $(echo "$c_syms" | tr '\n' ' ')"
  echo "Rust exports: $(echo "$rust_syms" | tr '\n' ' ')"
  if [[ -n "$missing" ]]; then
    echo "MISSING from Rust .so: $(echo "$missing" | tr '\n' ' ')"
    return 1
  fi
  echo "OK: every C symbol is exported by the Rust .so"
  return 0
}

MODE="${1:-check}"
STATUS=0

for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    label="--no-default-features"
    flags=(--no-default-features)
  else
    label="--no-default-features --features $combo"
    flags=(--no-default-features --features "$combo")
  fi

  case "$MODE" in
    check)
      run_one "cargo check $label" cargo check "${flags[@]}" || STATUS=1
      run_one "cargo check --tests $label" cargo check --tests "${flags[@]}" || STATUS=1
      ;;
    test)
      run_one "cargo build $label" cargo build "${flags[@]}" || STATUS=1
      run_one "cargo test $label" cargo test "${flags[@]}" -- --test-threads=4 || STATUS=1
      compare_symbols "$label" "$ROOT/translation/target/debug/libcontrast_ratio_lib.so" || STATUS=1
      # The release artifact is the real deliverable and uses different float
      # codegen, so re-run the same suite against it via the env override.
      run_one "cargo build --release $label" cargo build --release "${flags[@]}" || STATUS=1
      CONTRAST_RATIO_RUST_SO="$ROOT/translation/target/release/libcontrast_ratio_lib.so" \
        run_one "cargo test (against release .so) $label" \
        cargo test "${flags[@]}" -- --test-threads=4 || STATUS=1
      compare_symbols "release $label" "$ROOT/translation/target/release/libcontrast_ratio_lib.so" || STATUS=1
      ;;
    nm)
      run_one "cargo build --release $label" cargo build --release "${flags[@]}" || STATUS=1
      compare_symbols "$label" "$ROOT/translation/target/release/libcontrast_ratio_lib.so" || STATUS=1
      ;;
    *)
      echo "unknown mode: $MODE" >&2; exit 2 ;;
  esac
  echo
done

# The crate's declared default feature set, which power-set enumeration above
# deliberately skips (it uses --no-default-features throughout).
case "$MODE" in
  check)
    run_one "cargo check (default features)" cargo check || STATUS=1
    run_one "cargo check --tests (default features)" cargo check --tests || STATUS=1
    ;;
  test)
    run_one "cargo build (default features)" cargo build || STATUS=1
    run_one "cargo test (default features)" cargo test -- --test-threads=4 || STATUS=1
    compare_symbols "default features" "$ROOT/translation/target/debug/libcontrast_ratio_lib.so" || STATUS=1
    run_one "cargo build --release (default features)" cargo build --release || STATUS=1
    CONTRAST_RATIO_RUST_SO="$ROOT/translation/target/release/libcontrast_ratio_lib.so" \
      run_one "cargo test (against release .so, default features)" \
      cargo test -- --test-threads=4 || STATUS=1
    compare_symbols "release default features" "$ROOT/translation/target/release/libcontrast_ratio_lib.so" || STATUS=1
    ;;
  nm)
    run_one "cargo build --release (default features)" cargo build --release || STATUS=1
    compare_symbols "default features" "$ROOT/translation/target/release/libcontrast_ratio_lib.so" || STATUS=1
    ;;
esac

echo "======================================"
if [[ $STATUS -eq 0 ]]; then echo "ALL COMBINATIONS PASSED ($MODE)"; else echo "FAILURES DETECTED ($MODE)"; fi
exit $STATUS
