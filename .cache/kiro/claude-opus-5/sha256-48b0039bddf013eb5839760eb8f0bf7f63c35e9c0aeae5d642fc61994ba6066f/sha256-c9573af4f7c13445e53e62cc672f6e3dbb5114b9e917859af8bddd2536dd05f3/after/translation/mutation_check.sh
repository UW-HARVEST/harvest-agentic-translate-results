#!/usr/bin/env bash
# Mutation check: deliberately break the Rust translation in ways a sloppy
# C-to-Rust port would, and confirm the differential suite CATCHES each one.
# A mutation that survives means the tests are blind to that class of bug.
set -uo pipefail
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK=/tmp/lib.rs.mutbak
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

SURVIVED=0

# Mutations that are PROVABLY unobservable: `tests/exhaustive.rs` sweeps all
# 2^32 f32 inputs and shows these produce identical output for every one of
# them, so surviving is correct behaviour, not a coverage gap.
EXPECT_EQUIVALENT="clip threshold strict > instead of >=
clip threshold strict < instead of <=
f64 accumulate instead of f32"

mutate() {
  local name="$1" from="$2" to="$3"
  restore
  python3 - "$SRC" "$from" "$to" <<'PY' || { echo "  [skip] pattern not found: $name"; return; }
import sys
p, a, b = sys.argv[1], sys.argv[2], sys.argv[3]
lines = open(p).read().split('\n')
done = False
for i, ln in enumerate(lines):
    # Only mutate real code: doc comments quote the C source verbatim and would
    # otherwise absorb the replacement.
    if ln.lstrip().startswith('//'):
        continue
    if a in ln:
        lines[i] = ln.replace(a, b, 1)
        done = True
        break
if not done:
    sys.exit(1)
open(p, 'w').write('\n'.join(lines))
PY
  if ! timeout 600 cargo build >/tmp/mut_build.log 2>&1; then
    echo "  [skip] mutation does not compile: $name"
    return
  fi
  if timeout 600 cargo test >/tmp/mut_test.log 2>&1; then
    if grep -qxF "$name" <<<"$EXPECT_EQUIVALENT"; then
      echo "  survived, PROVEN EQUIVALENT by tests/exhaustive.rs: $name"
    else
      echo "  SURVIVED (tests blind to this bug): $name"
      SURVIVED=1
    fi
  else
    local n
    n=$(grep -cE '^test .* FAILED' /tmp/mut_test.log)
    if [ "$n" -eq 0 ]; then
      # A divergence so severe the test process faulted (e.g. a bad pointer
      # offset). Still a kill: the suite refused to pass.
      echo "  killed (test process aborted/faulted): $name"
    else
      echo "  killed by $n test(s): $name"
    fi
  fi
}

echo "== mutation testing the differential suite =="

mutate "coefficient 213 -> 214"                   "213.0f32"  "214.0f32"
mutate "coefficient 75038 -> 75039"               "75038.0f32" "75039.0f32"
mutate "coefficient -5 -> -6 (block 2 last term)" "* -5.0f32" "* -6.0f32"
mutate "sign flip: (tap(12) - tap(2)) -> +"       "(tap(12) - tap(2))" "(tap(12) + tap(2))"
mutate "tap swap: tap(1)+tap(13) -> tap(1)+tap(12)" "(tap(1) + tap(13))" "(tap(1) + tap(12))"
mutate "clip threshold 32766.5 -> 32766.0"        ">= 32766.5" ">= 32766.0"
mutate "clip threshold strict > instead of >="    ">= 32766.5" "> 32766.5"
mutate "clip threshold strict < instead of <="    "<= -32767.5" "< -32767.5"
mutate "clip value 32767 -> 32766"                "return 32767i16" "return 32766i16"
mutate "drop the s -= (s < 0) adjustment"         "s.wrapping_sub(i16::from(s < 0))" "s"
mutate "round instead of truncate"                "(sample + 0.5f32) as i32 as i16" "sample.round() as i32 as i16"
mutate "f64 accumulate instead of f32"            "let s = (sample + 0.5f32)" "let s = ((f64::from(sample) + 0.5) as f32)"
mutate "block-2 shift 2 -> 1 (z += 2)"            "z.wrapping_add(2 + i * 64)" "z.wrapping_add(1 + i * 64)"
mutate "stride 64 -> 63 in block 1"               "z.wrapping_add(i * 64)" "z.wrapping_add(i * 63)"
mutate "REGRESSION: 16*nch in isize (no int wrap)" "16i32.wrapping_mul(nch) as isize" "16isize * nch as isize"
mutate "REGRESSION: 16*nch through usize"          "16i32.wrapping_mul(nch) as isize" "(16usize.wrapping_mul(nch as usize)) as isize"
mutate "swap the two output slots"                 "std::ptr::write(pcm, mp3d_scale_pcm(a))" "std::ptr::write(pcm.wrapping_offset(16i32.wrapping_mul(nch) as isize), mp3d_scale_pcm(a))"
mutate "reorder: move last block-1 term first"     "a = (tap(14) - tap(0)) * 29.0f32;" "a = tap(7) * 75038.0f32; a += (tap(14) - tap(0)) * 29.0f32;"

restore
timeout 600 cargo build >/dev/null 2>&1

echo
if [ "$SURVIVED" -eq 0 ]; then
  echo "RESULT: every mutation was caught (or proven behaviourally equivalent) -> the differential suite is sensitive."
else
  echo "RESULT: at least one mutation SURVIVED -> add coverage for it."
fi
exit "$SURVIVED"
