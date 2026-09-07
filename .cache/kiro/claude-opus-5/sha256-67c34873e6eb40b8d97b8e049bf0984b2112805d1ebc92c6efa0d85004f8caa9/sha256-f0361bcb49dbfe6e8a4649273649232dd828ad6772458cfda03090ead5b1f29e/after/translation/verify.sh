#!/usr/bin/env bash
# Phase D — symbol parity + every Cargo feature combination.
#
#   ./verify.sh
#
# Rebuilds the C .so, enumerates the feature combinations declared in
# Cargo.toml, and for each one runs `cargo check`, diffs `nm -D` against the C
# .so, and runs the whole differential test suite in BOTH debug and release
# (LLVM restructures float code differently at -O, so both matter).
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }
bad()  { printf 'FAIL: %s\n' "$*"; FAIL=1; }

step "building the C shared library"
mkdir -p "$CBUILD"
( cd "$CBUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build failed"; exit 1; }
C_SO="$(ls "$CBUILD"/*.so | head -1)"
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml (the [features] table).
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/,""); print }
' "$CRATE/Cargo.toml" | grep -v '^default$' | sort -u)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features] -> the only combination is the default (empty) one."
  COMBOS=("")
else
  # Full power set of the declared features, plus the plain default build.
  mapfile -t FARR <<<"$FEATURES"
  N=${#FARR[@]}
  COMBOS=("")
  for ((mask=1; mask<(1<<N); mask++)); do
    combo=""
    for ((i=0; i<N; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FARR[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "feature combinations to verify: ${#COMBOS[@]}"

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    LABEL="<default>"; FLAGS=()
  else
    LABEL="--no-default-features --features $combo"
    FLAGS=(--no-default-features --features "$combo")
  fi

  step "combination: $LABEL"

  ( cd "$CRATE" && timeout 600 cargo check "${FLAGS[@]}" >/dev/null 2>&1 ) \
    || bad "cargo check failed for $LABEL"

  # ---- symbol parity -----------------------------------------------------
  ( cd "$CRATE" && timeout 600 cargo build --release --lib "${FLAGS[@]}" \
      --target-dir target/phase-d >/dev/null 2>&1 ) \
    || bad "cargo build failed for $LABEL"
  R_SO="$CRATE/target/phase-d/release/libagglom_lib.so"

  nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort >/tmp/pd_c.txt
  nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort >/tmp/pd_r.txt
  MISSING=$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)
  EXTRA=$(comm -13 /tmp/pd_c.txt /tmp/pd_r.txt)
  printf 'exported: C=%s Rust=%s\n' "$(wc -l </tmp/pd_c.txt)" "$(wc -l </tmp/pd_r.txt)"
  [ -z "$MISSING" ] || bad "symbols missing from the Rust .so ($LABEL): $MISSING"
  [ -z "$EXTRA" ]   || bad "extra symbols in the Rust .so ($LABEL): $EXTRA"

  # Undefined (imported) symbols must all be libc / libgcc-unwind.
  NONLIBC=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' | \
    grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_' || true)
  [ -z "$NONLIBC" ] || bad "non-libc undefined symbols in the Rust .so ($LABEL): $NONLIBC"

  # ---- Phases B + C, both profiles --------------------------------------
  for PROFILE in debug release; do
    EXTRA_ARGS=()
    [ "$PROFILE" = release ] && EXTRA_ARGS=(--release)
    if ( cd "$CRATE" && AGGLOM_TEST_FEATURES="$combo" \
         timeout 600 cargo test "${EXTRA_ARGS[@]}" "${FLAGS[@]}" >/tmp/pd_test.log 2>&1 ); then
      NPASS=$(sed -n 's/^test result: ok\. \([0-9]*\) passed.*/\1/p' /tmp/pd_test.log \
              | awk '{s+=$1} END{print s+0}')
      printf '  %-7s tests: PASS (%s test fns green across %s binaries)\n' \
        "$PROFILE" "$NPASS" "$(grep -c 'test result: ok' /tmp/pd_test.log)"
    else
      bad "tests failed for $LABEL / $PROFILE"
      tail -40 /tmp/pd_test.log
    fi
  done
done

step "binary executable check"
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  bad "c_src builds an executable -- stdout comparison is required but not implemented"
else
  echo "c_src/CMakeLists.txt has no add_executable"
fi
if [ -f "$CRATE/src/main.rs" ] || grep -q '^\[\[bin\]\]' "$CRATE/Cargo.toml"; then
  bad "the crate declares a binary target but the C side does not"
else
  echo "the crate has no [[bin]] and no src/main.rs"
fi
echo "-> neither side builds a driver binary; the stdout gate is not applicable"

step "result"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL PHASE D CHECKS PASSED"
else
  echo "PHASE D FAILURES PRESENT"
fi
exit "$FAIL"
