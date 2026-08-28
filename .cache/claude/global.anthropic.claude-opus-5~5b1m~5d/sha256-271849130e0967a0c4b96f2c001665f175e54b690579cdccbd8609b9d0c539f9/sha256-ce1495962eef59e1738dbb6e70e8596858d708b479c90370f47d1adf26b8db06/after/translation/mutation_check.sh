#!/usr/bin/env bash
# Validity check for the differential suite: a test suite that cannot FAIL
# proves nothing. This builds deliberately-broken copies of the C source and
# requires the suite to reject each one.
#
# c_src/ is NEVER modified: every mutant is compiled from a copy in $TMPDIR.
set -u -o pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
SRC="$ROOT/c_src/src/lib.c"
WORK="${TMPDIR:-/tmp}/mutants.$$"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT

# Match the CMake build: no CMAKE_BUILD_TYPE is set, so no -O flag (i.e. -O0).
CFLAGS=(-shared -fPIC -O0 -I "$ROOT/c_src/include" -I "$ROOT/c_src/src")

pass=0
fail=0
equiv=0

# Which cargo profile to test the mutants against. The release profile matters:
# it is where LLVM's heap-to-stack promotion once hid the malloc-failure branch.
PROFILE_FLAGS="${PROFILE_FLAGS:-}"

run_suite() { # $1 = .so path
  C_SO_PATH="$1" timeout 600 cargo test --offline $PROFILE_FLAGS --quiet 2>&1
}

echo "mutation check profile: ${PROFILE_FLAGS:-dev}"
cargo build --offline $PROFILE_FLAGS -q || { echo "FATAL: rust build failed"; exit 1; }

echo "=============================================================="
echo "CONTROL: unmodified source, compiled with our own gcc flags"
echo "=============================================================="
cp "$SRC" "$WORK/control.c"
if ! gcc "${CFLAGS[@]}" -o "$WORK/control.so" "$WORK/control.c" -lm 2>"$WORK/control.cc.log"; then
  echo "FATAL: control build failed"; cat "$WORK/control.cc.log"; exit 1
fi
if run_suite "$WORK/control.so" >"$WORK/control.log" 2>&1; then
  echo "  [OK] suite PASSES against an unmutated rebuild (flags reproduce CMake)"
  pass=$((pass + 1))
else
  echo "  [FATAL] suite FAILS against the unmutated control -- flags differ from CMake"
  tail -30 "$WORK/control.log"
  exit 1
fi

# ---------------------------------------------------------------------------
# Each mutant: a name and a sed program describing one behavioural change.
# ---------------------------------------------------------------------------
build_mutant() { # $1 name, $2.. sed exprs -> echoes .so path
  local name="$1"; shift
  local f="$WORK/$name.c" so="$WORK/$name.so"
  cp "$SRC" "$f"
  for expr in "$@"; do
    sed -i "$expr" "$f"
  done
  # NOTE: c_src/src/lib.c has CRLF line endings, so `$` anchors never match.
  if cmp -s "$SRC" "$f"; then
    echo "  [FATAL] mutant '$name' did not change the source (bad sed)" >&2; exit 1
  fi
  if ! gcc "${CFLAGS[@]}" -o "$so" "$f" -lm 2>"$WORK/$name.cc.log"; then
    echo "  [FATAL] mutant '$name' failed to compile" >&2
    cat "$WORK/$name.cc.log" >&2; exit 1
  fi
  echo "$so"
}

mutate() { # a behaviour-changing mutant: the suite MUST reject it
  local name="$1"; shift
  local so; so="$(build_mutant "$name" "$@")"
  if run_suite "$so" >"$WORK/$name.log" 2>&1; then
    echo "  [MISSED] $name -- suite PASSED a broken C library!"
    fail=$((fail + 1))
  else
    local n
    n=$(grep -c 'FAILED\|panicked at' "$WORK/$name.log" 2>/dev/null || true)
    echo "  [CAUGHT] $name (failure markers: ${n:-?})"
    pass=$((pass + 1))
  fi
}

mutate_equivalent() { # a mutant PROVEN to be semantically identical: must PASS
  local name="$1"; shift
  local so; so="$(build_mutant "$name" "$@")"
  if run_suite "$so" >"$WORK/$name.log" 2>&1; then
    echo "  [EQUIVALENT] $name -- suite passes, as expected (see comment)"
    equiv=$((equiv + 1))
  else
    echo "  [UNEXPECTED] $name was expected to be semantically equivalent but"
    echo "               the suite rejected it; re-check the equivalence claim."
    tail -20 "$WORK/$name.log"
    fail=$((fail + 1))
  fi
}

echo
echo "=============================================================="
echo "MUTANTS (the suite must reject every one)"
echo "=============================================================="

mutate flag_constant_0200_to_0201 \
  's/#define OCTAL_FLAG   0200/#define OCTAL_FLAG   0201/'

mutate mask_0777_to_0776 \
  's/#define OCTAL_MASK_1 0777/#define OCTAL_MASK_1 0776/'

mutate base_010_to_011 \
  's/#define OCTAL_BASE   010/#define OCTAL_BASE   011/'

mutate mask2_0100_to_0101 \
  's/#define OCTAL_MASK_2 0100/#define OCTAL_MASK_2 0101/'

# Break the deliberate case 0 -> 1 fallthrough.
mutate switch_break_after_case0 \
  's/^            result \*= OCTAL_BASE;/            result *= OCTAL_BASE; break;/'

# Break the case 3 -> 4 fallthrough.
mutate switch_break_after_case3 \
  's/^            result \*= 3;/            result *= 3; break;/'

# NOTE: c_src/src/lib.c uses CRLF line endings, so `$` anchors never match --
# every pattern below is deliberately unanchored at the end of the line.
mutate switch_default_returns_one \
  's/^            result = 0;/            result = 1;/'

# Clamp thresholds moved by one: genuinely observable for d in
# [2147483646, 2147483647) and (-2147483648, -2147483647].
mutate clamp_threshold_high_off_by_one \
  's/if (d >= (double)INT_MAX)/if (d >= (double)INT_MAX - 1)/'

mutate clamp_threshold_low_off_by_one \
  's/if (d <= (double)INT_MIN)/if (d <= (double)INT_MIN + 1)/'

# --- Proven-equivalent mutants: relaxing `>=` to `>` (and `<=` to `<`) on the
# clamp guards changes NOTHING, because at exactly (double)INT_MAX the fallthrough
# `(int)d` already yields INT_MAX (the value is exactly representable in int), and
# likewise at (double)INT_MIN. Verified by brute force: 0 differences over
# 5,173,210 doubles, including exhaustive one-ULP sweeps around both thresholds.
# They are listed here so the suite's mutation score stays honest -- these are
# equivalent mutants, NOT blind spots.
mutate_equivalent clamp_ge_to_gt_EQUIVALENT \
  's/if (d >= (double)INT_MAX)/if (d > (double)INT_MAX)/'

mutate_equivalent clamp_le_to_lt_EQUIVALENT \
  's/if (d <= (double)INT_MIN)/if (d < (double)INT_MIN)/'

mutate nan_returns_one \
  '0,/return 0;/s/return 0;/return 1;/'

mutate inf_sign_swapped \
  's/return d > 0 ? INT_MAX : INT_MIN;/return d > 0 ? INT_MIN : INT_MAX;/'

# Walk forwards instead of backwards.
mutate reverse_walks_forward \
  's/^        ptr--;/        ptr++;/'

mutate flag_guard_gt_to_ge \
  's/if (param3 > OCTAL_FLAG)/if (param3 >= OCTAL_FLAG)/'

mutate alloc_size_off_by_one \
  's/allocate_and_compute(param4 % 10 + 1, 1.5)/allocate_and_compute(param4 % 10 + 2, 1.5)/'

mutate alloc_value_off_by_one \
  's/points\[i\].value = i \* OCTAL_BASE;/points[i].value = (i + 1) * OCTAL_BASE;/'

# Turns BOTH malloc-failure sentinels (allocate_and_compute L106 and
# fallcalc L146) from -1 into 0; only the forced-malloc-failure test can see it.
mutate malloc_failure_sentinel_minus1_to_0 \
  's/^        return -1;/        return 0;/'

mutate float_coefficient_3_7 \
  's/(double)param1 \* 3\.7/(double)param1 * 3.7000000001/'

mutate float_coefficient_2_3 \
  's/(double)param2 \* 2\.3/(double)param2 * 2.2999999999/'

mutate float_sign_flip \
  's/- (double)param3 \* 0\.5/+ (double)param3 * 0.5/'

mutate switch_operand_mod_5_to_4 \
  's/switch_fallthrough_calculator(param2, param3 % 5)/switch_fallthrough_calculator(param2, param3 % 4)/'

mutate data_array_init_off_by_one \
  's/data_array\[i\] = (i + 1) \* OCTAL_BASE + param1;/data_array[i] = (i + 2) * OCTAL_BASE + param1;/'

mutate array_size_5_to_4 \
  's/int array_size = 5;/int array_size = 4;/'

mutate foreach_skips_first \
  's/idx = 0, \\/idx = 1, \\/'

mutate multiplier_1_5_to_1_6 \
  's/param4 % 10 + 1, 1\.5/param4 % 10 + 1, 1.6/'

echo
echo "=============================================================="
echo "RESULT: $pass caught (incl. control), $equiv proven-equivalent, $fail MISSED"
echo "=============================================================="
[ "$fail" -eq 0 ]
