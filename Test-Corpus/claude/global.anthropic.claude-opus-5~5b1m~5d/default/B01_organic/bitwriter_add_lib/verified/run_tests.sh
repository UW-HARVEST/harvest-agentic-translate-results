#!/usr/bin/env bash
# Differential verification driver.
#
# `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` target, so the
# cdylib MUST be built explicitly before the tests run — otherwise every
# differential test silently exercises a stale `.so`.  A freshness guard in
# tests/common/mod.rs also fails the run if that happens.
#
# Also enumerates every Cargo feature combination (Phase D) — currently the
# crate declares none, so the default is the only combination, but the loop is
# generic.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
OFFLINE=${OFFLINE:---offline}
rc=0

# ---------------------------------------------------------------- C reference
if ! ls "$ROOT"/c_src/build/*.so >/dev/null 2>&1; then
  echo "== building C reference shared library =="
  (mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi
C_SO=$(ls "$ROOT"/c_src/build/*.so | head -1)
echo "C  .so: $C_SO"

# ------------------------------------------------- feature-combination matrix
# Every declared feature (excluding "default") -> powerset of combinations.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1]!="default" && a[1]!="") print a[1]}' Cargo.toml
)

COMBOS=("default")
if ((${#FEATURES[@]} > 0)); then
  COMBOS=("default" "--no-default-features")
  n=${#FEATURES[@]}
  for ((m = 1; m < (1 << n); m++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      ((m & (1 << i))) && combo="${combo:+$combo,}${FEATURES[i]}"
    done
    COMBOS+=("--no-default-features --features $combo")
  done
fi

echo "== feature combinations: ${#COMBOS[@]} =="
for c in "${COMBOS[@]}"; do echo "   - $c"; done

for combo in "${COMBOS[@]}"; do
  if [[ $combo == "default" ]]; then FLAGS=(); else read -r -a FLAGS <<<"$combo"; fi
  echo
  echo "############################################################"
  echo "# combo: $combo"
  echo "############################################################"

  # 1) build the cdylib under test (MANDATORY before cargo test)
  cargo build --release $OFFLINE "${FLAGS[@]}" || { echo "BUILD FAILED [$combo]"; rc=1; continue; }
  R_SO=target/release/libbitwriter_add_lib.so
  echo "Rust .so: $R_SO"

  # 2) symbol parity gate (fails loudly, independent of the test harness)
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO"  | awk '$2 ~ /^[TDBR]$/ {split($3,a,"@"); print a[1]}' | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$' | sort -u) \
    <(nm -D --defined-only "$R_SO"  | awk '$2 ~ /^[TDBR]$/ {split($3,a,"@"); print a[1]}' | sort -u))
  if [[ -n $missing ]]; then
    echo "SYMBOL PARITY FAILED [$combo] — missing from Rust .so:"; echo "$missing"; rc=1
  else
    echo "symbol parity: OK (0 missing)"
  fi

  # 3) Phase B + C + D differential tests
  cargo test --release $OFFLINE "${FLAGS[@]}" --no-fail-fast || { echo "TESTS FAILED [$combo]"; rc=1; }

  # 4) robustness: the C source contains shift-count-out-of-range UB, so
  #    re-run the differential suite against C references built at every
  #    optimisation level to prove the match is not an artefact of -O0.
  OPT_DIR="${TMPDIR:-/tmp}/c_opt_refs"
  mkdir -p "$OPT_DIR"
  for O in O0 O1 O2 O3 Os; do
    if gcc "-$O" -fPIC -shared -I"$ROOT/c_src/include" \
         -o "$OPT_DIR/lib$O.so" "$ROOT/c_src/src/lib.c" 2>/dev/null; then
      if C_REF_SO="$OPT_DIR/lib$O.so" cargo test --release $OFFLINE "${FLAGS[@]}" \
           --no-fail-fast --test phase_b_configs --test phase_c_errors >/dev/null 2>&1; then
        echo "C reference -$O: OK"
      else
        echo "C reference -$O: DIVERGED [$combo]"; rc=1
      fi
    fi
  done
done

echo
if ((rc == 0)); then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT (rc=$rc)"; fi
exit $rc
