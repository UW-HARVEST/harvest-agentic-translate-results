#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination from Cargo.toml, then
# cargo check + cargo test each one, and diff exported symbols C vs Rust.
set -uo pipefail
cd "$(dirname "$0")"

fail=0
TMP="${TMPDIR:-/tmp}"
TMP="${TMP%/}"

# ---------------------------------------------------------------------------
# 1. Enumerate features declared in Cargo.toml -> powerset of combinations.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[ ]*=/{sub(/[ ]*=.*/,"");print}' Cargo.toml
)
echo "=== features declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]-none}) ==="

combos=("")
for f in "${FEATURES[@]-}"; do
  [ -z "$f" ] && continue
  new=()
  for c in "${combos[@]}"; do
    new+=("$c")
    if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  combos=("${new[@]}")
done
echo "=== ${#combos[@]} feature combination(s) to verify ==="

# ---------------------------------------------------------------------------
# 2. Build the C reference .so.
# ---------------------------------------------------------------------------
echo
echo "=== building C reference shared library ==="
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
C_SO=c_src/build/libtranslated_rust.so
echo "built $C_SO"

# ---------------------------------------------------------------------------
# 3. check + test every combination, in debug and release.
# ---------------------------------------------------------------------------
for c in "${combos[@]}"; do
  label="${c:-<no features>}"
  for profile in debug release; do
    relflag=""; [ "$profile" = release ] && relflag="--release"
    echo
    echo "############ combo: $label | profile: $profile ############"

    if ! timeout 600 cargo check --offline --no-default-features --features "$c" $relflag 2>&1 | tail -3; then
      echo "CHECK FAILED [$label/$profile]"; fail=1
    fi

    # Nothing links the cdylib, so `cargo test` will not build it: do it here.
    if ! timeout 600 cargo build --offline --no-default-features --features "$c" $relflag 2>&1 | tail -2; then
      echo "BUILD FAILED [$label/$profile]"; fail=1
    fi

    out=$(timeout 600 cargo test --offline --no-default-features --features "$c" $relflag 2>&1)
    echo "$out" | grep -E "^test result:|FAILED|panicked" || true
    if echo "$out" | grep -qE "FAILED|error\[|error:"; then
      echo "TESTS FAILED [$label/$profile]"; fail=1
    fi

    # ---- symbol parity for this combo/profile ----
    R_SO="target/$profile/libmatrixsum_lib.so"
    if [ ! -f "$R_SO" ]; then
      echo "MISSING RUST SO: $R_SO"; fail=1; continue
    fi
    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "$TMP/c_syms.$$"
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > "$TMP/r_syms.$$"
    missing=$(comm -23 "$TMP/c_syms.$$" "$TMP/r_syms.$$")
    extra=$(comm -13 "$TMP/c_syms.$$" "$TMP/r_syms.$$")
    echo "symbols: C=$(wc -l < "$TMP/c_syms.$$") RUST=$(wc -l < "$TMP/r_syms.$$")"
    if [ -n "$missing" ]; then echo "MISSING FROM RUST: $missing"; fail=1; else echo "missing: none"; fi
    if [ -n "$extra" ]; then echo "extra in rust: $extra"; fi
    rm -f "$TMP/c_syms.$$" "$TMP/r_syms.$$"
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "=============== ALL COMBINATIONS PASSED ==============="
else
  echo "=============== FAILURES PRESENT ==============="
fi
exit "$fail"
