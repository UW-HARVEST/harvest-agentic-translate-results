#!/usr/bin/env bash
# Full verification driver: builds both libraries, diffs their exported symbols,
# and runs the differential suite under EVERY feature combination.
#
# Must be run from translation/.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

echo "=============================================================="
echo "1. Build the C shared library"
echo "=============================================================="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations declared in Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[ \t]*=/ { sub(/[ \t]*=.*/, ""); if ($0 != "default") print }
  ' "$CRATE/Cargo.toml"
)

COMBOS=("--no-default-features" "")   # degenerate + default
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi
echo
echo "Declared features: ${FEATURES[*]:-<none>}"
echo "Feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  echo
  echo "=============================================================="
  echo "2. Feature combination: $label"
  echo "=============================================================="

  # shellcheck disable=SC2086
  ( cd "$CRATE" && timeout 600 cargo build --release $combo ) \
    || { echo "RUST BUILD FAILED ($label)"; FAIL=1; continue; }

  RUST_SO="$CRATE/target/release/libdiv_euclid_lib.so"

  echo "--- symbol parity ($label) ---"
  diff <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u) \
       <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u) \
       > /tmp/symdiff.txt
  missing="$(grep '^<' /tmp/symdiff.txt || true)"
  if [[ -n "$missing" ]]; then
    echo "MISSING FROM RUST .so:"; echo "$missing"; FAIL=1
  else
    echo "OK: 0 C symbols missing from the Rust .so"
  fi
  extra="$(grep '^>' /tmp/symdiff.txt || true)"
  [[ -n "$extra" ]] && { echo "extra (Rust-only, informational):"; echo "$extra"; }

  echo "--- non-libc undefined symbols in the Rust .so ---"
  undef="$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
           | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^gettid|^statx' || true)"
  if [[ -n "$undef" ]]; then
    echo "UNRESOLVED NON-LIBC SYMBOLS:"; echo "$undef"; FAIL=1
  else
    echo "OK: 0 non-libc undefined symbols"
  fi

  echo "--- differential tests ($label) ---"
  # shellcheck disable=SC2086
  ( cd "$CRATE" && timeout 600 cargo test --release $combo -- --test-threads=8 ) \
    || { echo "TESTS FAILED ($label)"; FAIL=1; }
done

echo
echo "=============================================================="
if ((FAIL == 0)); then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
echo "=============================================================="
exit "$FAIL"
