#!/usr/bin/env bash
# Full differential-verification run: builds the C .so and the Rust .so, then
# runs every integration test in every feature combination.
#
#   ./verify.sh            # everything
#   ./verify.sh block hc   # only the named test targets
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
LOG="$CRATE/target/verify.log"

echo "=== 1/4  building the C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >"$LOG" 2>&1 || { tail -30 "$LOG"; exit 1; }
ls -l "$ROOT/c_src/build/liblz4.so"

echo "=== 2/4  building the Rust shared library ==="
( cd "$CRATE" && cargo build --offline --release ) >>"$LOG" 2>&1 \
  || { tail -30 "$LOG"; exit 1; }
ls -l "$CRATE/target/release/liblz4.so"

echo "=== 3/4  symbol parity (nm -D) ==="
nm -D --defined-only "$ROOT/c_src/build/liblz4.so" | awk '{print $3}' | sort -u \
  > "$CRATE/target/c_syms.txt"
nm -D --defined-only "$CRATE/target/release/liblz4.so" | awk '{print $3}' | sort -u \
  > "$CRATE/target/r_syms.txt"
printf 'C   exports: %s\n' "$(wc -l < "$CRATE/target/c_syms.txt")"
printf 'Rust exports: %s\n' "$(wc -l < "$CRATE/target/r_syms.txt")"
MISSING="$(comm -23 "$CRATE/target/c_syms.txt" "$CRATE/target/r_syms.txt")"
if [ -n "$MISSING" ]; then
  echo "MISSING FROM RUST:"; echo "$MISSING"; exit 1
fi
echo "symbol diff is EMPTY (0 missing)"

echo "=== 4/4  differential tests ==="
# The crate declares no [features], so the default set is the only combination.
# The loop is kept so that adding a feature automatically extends the matrix.
COMBOS=("")
FEATURES="$(sed -n '/^\[features\]/,/^\[/p' "$CRATE/Cargo.toml" \
            | grep -oE '^[a-zA-Z0-9_-]+' | grep -v '^default$' || true)"
if [ -n "$FEATURES" ]; then
  COMBOS=("--no-default-features")
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
  COMBOS+=("--all-features")
fi

TARGETS=("$@")
if [ ${#TARGETS[@]} -eq 0 ]; then
  TARGETS=(smoke xxh block hc frame frame_err file extra)
fi

FAIL=0
for combo in "${COMBOS[@]}"; do
  echo "--- feature combination: ${combo:-<default>} ---"
  for t in "${TARGETS[@]}"; do
    printf '%-12s ' "$t"
    # shellcheck disable=SC2086
    out="$( cd "$CRATE" && timeout 900 cargo test --offline --release $combo --test "$t" 2>&1 )"
    line="$(printf '%s\n' "$out" | grep -E '^test result' | tail -1)"
    if [ -z "$line" ]; then
      echo "NO RESULT LINE"; printf '%s\n' "$out" | tail -20; FAIL=1
    else
      echo "$line"
      printf '%s\n' "$line" | grep -q ' 0 failed' || FAIL=1
      printf '%s\n' "$out" | grep -E '^test .* FAILED|panicked' | head -20
    fi
  done
done

# The >2 GiB stream tests in tests/extra.rs are #[ignore]d because they take
# ~20 minutes; run them explicitly unless VERIFY_SKIP_SLOW is set.
if [ -z "${VERIFY_SKIP_SLOW:-}" ]; then
  echo "--- slow (#[ignore]d) >1 GiB / >2 GiB stream tests ---"
  printf '%-12s ' "extra --ignored"
  out="$( cd "$CRATE" && cargo test --offline --release --test extra -- --ignored --test-threads=1 2>&1 )"
  line="$(printf '%s\n' "$out" | grep -E '^test result' | tail -1)"
  echo "${line:-NO RESULT LINE}"
  printf '%s\n' "$line" | grep -q ' 0 failed' || FAIL=1
  printf '%s\n' "$out" | grep -E '^test .* FAILED|panicked' | head -20
else
  echo "--- slow tests SKIPPED (VERIFY_SKIP_SLOW set) ---"
fi

if [ "$FAIL" -eq 0 ]; then
  echo "=== ALL DIFFERENTIAL TESTS PASSED ==="
else
  echo "=== FAILURES PRESENT ==="
fi
exit "$FAIL"
