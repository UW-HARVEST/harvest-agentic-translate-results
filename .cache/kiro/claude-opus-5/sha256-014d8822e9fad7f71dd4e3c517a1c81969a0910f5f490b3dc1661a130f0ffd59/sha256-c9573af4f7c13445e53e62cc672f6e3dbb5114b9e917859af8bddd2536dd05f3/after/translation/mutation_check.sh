#!/usr/bin/env bash
# Negative control: prove the differential harness really detects divergence.
#
# Each mutation perturbs the Rust translation in exactly one place and the suite
# MUST then fail. A mutation that still passes means either (a) the tests do not
# exercise that code, or (b) the code is genuinely unreachable through the
# public ABI. Case (b) entries are listed in EXPECTED_ESCAPES with a reason.
#
# Detection uses cargo's EXIT STATUS, not a grep of stdout: a diverging build can
# abort the test process (SIGABRT / stack overflow) before libtest prints its
# "test result:" summary line.
set -u
cd "$(dirname "$0")"

BAK=$(mktemp)
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap restore EXIT

# desc::sed-expression
MUTATIONS=(
  "mode3 mask 0o177 -> 0o377::s/result.wrapping_add(flags & 0o177)/result.wrapping_add(flags \& 0o377)/"
  "default sentinel 0o200 -> 0o201::s/result = STATUS_ERROR | 0o200;/result = STATUS_ERROR | 0o201;/"
  "mode1 sentinel 0o20 -> 0o21::s/return STATUS_ERROR | 0o20;/return STATUS_ERROR | 0o21;/"
  "mode2 sentinel 0o40 -> 0o41::s/return STATUS_ERROR | 0o40;/return STATUS_ERROR | 0o41;/"
  "mode4 sentinel 0o100 -> 0o101::s/return STATUS_ERROR | 0o100;/return STATUS_ERROR | 0o101;/"
  "sprintf literal _Depth_ shortened::s/b\"_Depth_\"/b\"_Depth\"/"
  "sprintf literal Node_ shortened::s/b\"Node_\"/b\"Nod_\"/"
  "compute_size_metric *2 -> *3::s/metric.wrapping_mul(2).wrapping_add(0o10)/metric.wrapping_mul(3).wrapping_add(0o10)/"
  "compute_size_metric +0o10 -> +0o11::s/metric.wrapping_mul(2).wrapping_add(0o10)/metric.wrapping_mul(2).wrapping_add(0o11)/"
  "%d drops the minus sign::s/^    if negative {\$/    if false {/"
  "%d digits base 10 -> base 9::s/(magnitude % 10) as u8/(magnitude % 9) as u8/"
  "%d zero prints empty::s/digits\\[0\\] = b'0';/digits[0] = b' ';/"
  "case 0o3 -> 0o5 (mode 3 unreachable)::s/^        0o3 => {\$/        0o5 => {/"
  "case 0o1 -> 0o6 (mode 1 unreachable)::s/^        0o1 => {\$/        0o6 => {/"
  "NODE_COUNT starts at 7 (as if initialize_test_data ran)::s/static mut NODE_COUNT: c_int = 0;/static mut NODE_COUNT: c_int = 7;/"
  "c_strlen off by one::s/    while \\*str.add(n) != 0 {/    while n == 0 || *str.add(n) != 0 {/"
  "default arm returns STATUS_OK::s/result = STATUS_ERROR | 0o200;/result = STATUS_OK;/"
  "mode2 array_size 0o20 -> 0o17::s/array_size = 0o20;/array_size = 0o17;/"
  "mode4 constant 2.718281828 -> 2.7::s/\\* 2.718281828/* 2.7/"
  "mode1 parent weight 1.5 -> 2.5::s/value \\* 1.5/value * 2.5/"
  "safe_double_to_int upper clamp off by one::s/value = 2147483647.0;/value = 2147483646.0;/"
  "%d emits an EXTRA digit (length +1)::s/    while n > 0 {/    while n > 0 { out[*len] = b'9'; *len += 1;/"
  "%d zero emits NO digit (length -1)::s/        n = 1;/        n = 0;/"
  "sprintf literal _Depth_ LENGTHENED::s/b\"_Depth_\"/b\"_Depth__\"/"
)

# Mutations that legitimately cannot be observed through `jumpnode`, because
# `initialize_test_data()` is never called, so `node_count == 0`, so modes 0001 /
# 0002 / 0004 always take their null-node early return. Everything after that
# early return in those arms is dead code in the C original too.
EXPECTED_ESCAPES=(
  # --- dead code: modes 0001/0002/0004 always early-return (node_count == 0) ---
  "mode2 array_size 0o20 -> 0o17"
  "mode4 constant 2.718281828 -> 2.7"
  "mode1 parent weight 1.5 -> 2.5"
  "safe_double_to_int upper clamp off by one"
  # --- unobservable: mode 0003 feeds the buffer to compute_size_metric, which
  # --- uses only strlen(). The DIGIT CONTENT of %d can never affect the return
  # --- value; only the LENGTH can. The three "length +/- 1" mutations below are
  # --- caught, which proves the length is genuinely under test.
  "%d digits base 10 -> base 9"
  "%d zero prints empty"
  # --- semantic no-op: `n == 0 ||` is redundant because the formatted buffer is
  # --- never empty (shortest output is "Node_0_Depth_0", 14 bytes).
  "c_strlen off by one"
)

is_expected_escape() {
  local needle="$1"
  for e in "${EXPECTED_ESCAPES[@]}"; do
    [ "$e" = "$needle" ] && return 0
  done
  return 1
}

caught=0
expected=0
unexpected=0
skipped=0

for entry in "${MUTATIONS[@]}"; do
  desc="${entry%%::*}"
  expr="${entry#*::}"
  restore
  sed -i "$expr" src/lib.rs
  if diff -q src/lib.rs "$BAK" >/dev/null; then
    echo "SKIP  (sed matched nothing)      : $desc"
    skipped=$((skipped+1))
    continue
  fi

  timeout 600 cargo test --release >/dev/null 2>&1
  status=$?

  if [ "$status" -ne 0 ]; then
    echo "CAUGHT (exit $status)              : $desc"
    caught=$((caught+1))
  elif is_expected_escape "$desc"; then
    echo "ESCAPED - EXPECTED (dead code)   : $desc"
    expected=$((expected+1))
  else
    echo "ESCAPED - UNEXPECTED <<<<<<<<<<< : $desc"
    unexpected=$((unexpected+1))
  fi
done

restore
echo
echo "caught: $caught  expected-escapes: $expected  unexpected-escapes: $unexpected  skipped: $skipped"
if [ "$unexpected" -ne 0 ] || [ "$skipped" -ne 0 ]; then
  exit 1
fi
echo "NEGATIVE CONTROL OK"
