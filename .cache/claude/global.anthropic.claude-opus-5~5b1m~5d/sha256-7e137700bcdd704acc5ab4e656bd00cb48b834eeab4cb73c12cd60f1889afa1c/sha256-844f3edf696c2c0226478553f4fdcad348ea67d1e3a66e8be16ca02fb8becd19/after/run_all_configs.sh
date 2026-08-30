#!/usr/bin/env bash
# Phases B + C, repeated for EVERY feature combination (Phase D).
#
# For each configuration:
#   1. `cargo build`  with that feature set (so target/debug/libdriver.so and
#      target/debug/driver exist for the .so-level and CLI-level tests),
#   2. `cargo test`   with the SAME feature set. The test harness
#      (translation/tests/common/mod.rs) resolves the active features to an
#      (OP, REPEAT) pair using the crate's documented priority rules and loads
#      the matching cbuild/libcdriver_<op>_<repeat>.so as the C reference.
#
# Rows 1-24 of CONFIGS.md are the 24 canonical CMake configurations.
# Rows 25-28 are the Cargo-only cases: no features (defaults), conflicting OP
# features, conflicting REPEAT features, and one axis left at its default.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGD="$ROOT/logs"
mkdir -p "$LOGD"
cd "$ROOT/translation"
export CARGO_NET_OFFLINE=true

# Extra flags forwarded to both `cargo build` and `cargo test`, e.g.
#   CARGO_EXTRA=--release ./run_all_configs.sh
# The dev profile is the default because it enables Rust's arithmetic overflow
# checks, which makes the wrapping-arithmetic parity assertions strictly harder
# to satisfy; the release pass additionally covers the optimised cdylib that a
# real consumer links against.
EXTRA=${CARGO_EXTRA:-}
SUFFIX=""
[[ $EXTRA == *--release* ]] && SUFFIX="_release"

# Make sure the C references exist for all 24 configurations.
[[ -f "$ROOT/cbuild/libcdriver_mul_7.so" ]] || "$ROOT/build_c_so.sh" >/dev/null

configs=()
# CONFIGS.md rows 1-24: the full OP x REPEAT cross-product.
for op in add sub mul; do
  for r in 0 1 2 3 4 5 6 7; do configs+=("$op,$r"); done
done
# CONFIGS.md row 25: no features at all -> must behave as add / 5.
configs+=("")
# CONFIGS.md row 26: conflicting OP features -> mul > sub > add.
configs+=("add,sub" "add,mul" "sub,mul" "add,sub,mul")
# CONFIGS.md row 27: conflicting REPEAT features -> highest wins.
configs+=("0,1,2,3,4,5,6,7" "2,5" "0,7" "sub,3,6" "mul,1,2,4")
# CONFIGS.md row 28: one axis left at its #ifndef default.
configs+=("mul" "sub" "add" "0" "7" "3")
# Everything at once.
configs+=("add,sub,mul,0,1,2,3,4,5,6,7")

# Resolve a Cargo feature list to the (OP, REPEAT) pair the crate documents:
# OP priority mul > sub > add (default add); REPEAT = highest enabled (default 5).
# Mirrors translation/tests/common/mod.rs, so the reported pairing is the one the
# harness used to pick cbuild/libcdriver_<op>_<repeat>.so.
resolve() {
  local f=",$1,"
  local op=add rep=5 n
  [[ $f == *,sub,* ]] && op=sub
  [[ $f == *,mul,* ]] && op=mul
  for n in 0 1 2 3 4 5 6 7; do [[ $f == *,$n,* ]] && rep=$n; done
  echo "OP=$op REPEAT=$rep"
}

echo "running Phase B + C for ${#configs[@]} configuration(s)"
printf '%s\n' "-----------------------------------------------"

fail=0
i=0
for combo in "${configs[@]}"; do
  i=$((i + 1))
  label="${combo:-<none>}"
  safe="${combo//,/_}"
  safe="${safe:-none}"
  log="$LOGD/test_$safe$SUFFIX.log"

  if ! timeout 600 cargo build --offline $EXTRA --no-default-features --features "$combo" \
      >"$log" 2>&1; then
    echo "BUILD FAIL [$i/${#configs[@]}] features='$label' (see $log)"
    fail=$((fail + 1))
    continue
  fi
  if ! timeout 600 cargo test --offline $EXTRA --no-fail-fast --no-default-features \
      --features "$combo" >>"$log" 2>&1; then
    echo "TEST FAIL  [$i/${#configs[@]}] features='$label' (see $log)"
    grep -E '^(test .* FAILED|failures:|thread .* panicked|assertion)' "$log" | head -10
    grep -A6 '^---- ' "$log" | head -40
    fail=$((fail + 1))
    continue
  fi
  passed=$(grep -Eo '^test result: ok\. [0-9]+' "$log" |
    grep -Eo '[0-9]+$' | awk '{s+=$1} END {print s+0}')
  echo "ok         [$i/${#configs[@]}] features='$label' -> $(resolve "$combo"), $passed tests passed"
done

printf '%s\n' "-----------------------------------------------"
if ((fail == 0)); then
  echo "PASS: all ${#configs[@]} configurations agree with the C reference"
else
  echo "FAIL: $fail / ${#configs[@]} configurations diverged"
fi
exit $((fail > 0))
