#!/usr/bin/env bash
# Sensitivity check for the differential suite: inject a semantic mutation into
# the Rust translation, rebuild the .so, and confirm the tests CATCH it.
# Restores the original source afterwards.
set -u
W="$(cd "$(dirname "$0")" && pwd)"
SRC="$W/src/lib.rs"
ORIG="$W/.mutbak/lib.rs.orig"
cp "$ORIG" "$SRC"

run() { # $1 = label, $2 = python replace expression already applied
  ( cd "$W" && cargo build --release --offline >/dev/null 2>&1 ) || { echo "BUILD-FAIL  $1"; return; }
  out=$( cd "$W" && cargo test --offline --release -- --test-threads=4 2>&1 )
  if echo "$out" | grep -q 'test result: FAILED'; then
    n=$(echo "$out" | grep -c '\.\.\. FAILED')
    echo "CAUGHT      $1  ($n test(s) failed)"
  else
    echo "*** MISSED  $1  <-- suite is blind to this mutation"
  fi
}

mutate() { # $1 label, $2 from, $3 to
  cp "$ORIG" "$SRC"
  python3 - "$SRC" "$2" "$3" <<'PY'
import sys
p, a, b = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
if a not in s:
    print("PATTERN-NOT-FOUND:", a); sys.exit(1)
open(p, 'w').write(s.replace(a, b, 1))
PY
  [ $? -ne 0 ] && { echo "SKIP        $1 (pattern not found)"; return; }
  run "$1"
}

echo "=== mutation sensitivity battery ==="
mutate "M1  drop the dest!=src aliasing guard" \
       "} else if dest as *const f32 != src {" "} else if true {"
mutate "M2  widen the accumulator to f64" \
       "        sum += v * v;" "        sum = (sum as f64 + (v as f64)*(v as f64)) as f32;"
mutate "M3  branch on sum >= 0.0 instead of > 0.0" \
       "if sum > 0.0f32 {" "if sum >= 0.0f32 {"
mutate "M4  branch on !(sum <= 0.0) (NaN takes scale branch)" \
       "if sum > 0.0f32 {" "if !(sum <= 0.0f32) {"
mutate "M5  divide per element instead of multiplying by the reciprocal" \
       "sum = 1.0f32 / sum.sqrt();" "sum = sum.sqrt();"
mutate "M6  use rsqrt via powf(-0.5)" \
       "sum = 1.0f32 / sum.sqrt();" "sum = sum.powf(-0.5f32);"
mutate "M7  memset length forgets the *4 element size" \
       "(size as usize).wrapping_mul(size_of::<f32>())" "(size as usize)"
mutate "M8  memset length saturates instead of wrapping" \
       "(size as usize).wrapping_mul(size_of::<f32>())" "(size as usize).saturating_mul(size_of::<f32>())"
mutate "M9  negative size clamped to 0 before the memset" \
       "let nbytes: usize = (size as usize).wrapping_mul(size_of::<f32>());" \
       "let nbytes: usize = (size.max(0) as usize).wrapping_mul(size_of::<f32>());"
mutate "M10 memset writes 0xFF bytes instead of 0" \
       "write_bytes(dest as *mut u8, 0u8, nbytes)" "write_bytes(dest as *mut u8, 0xFFu8, nbytes)"
mutate "M11 off-by-one: scale loop stops one element early" \
       "        i = 0;
        while i < size {" "        i = 0;
        while i < size - 1 {"
mutate "M12 off-by-one: sum loop reads one element too many" \
       "    let mut i: c_int = 0;
    while i < size {" "    let mut i: c_int = 0;
    while i <= size {"
mutate "M13 write dest[i] = -v*sum (sign flip)" \
       "*dest.offset(i as isize) = v * sum" "*dest.offset(i as isize) = -(v * sum)"
mutate "M14 read dest instead of src in the scale loop (aliasing-sensitive)" \
       "            let v: f32 = unsafe { *src.offset(i as isize) };
            unsafe { *dest.offset(i as isize) = v * sum };" \
       "            let v: f32 = unsafe { *dest.offset(i as isize) };
            unsafe { *dest.offset(i as isize) = v * sum };"
mutate "M15 scale loop iterates in reverse (aliasing-sensitive)" \
       "        i = 0;
        while i < size {
            let v: f32 = unsafe { *src.offset(i as isize) };
            unsafe { *dest.offset(i as isize) = v * sum };
            i += 1;
        }" \
       "        i = size - 1;
        while i >= 0 {
            let v: f32 = unsafe { *src.offset(i as isize) };
            unsafe { *dest.offset(i as isize) = v * sum };
            i -= 1;
        }"
mutate "M16 use f32::abs on the sum before comparing" \
       "if sum > 0.0f32 {" "if sum.abs() > 0.0f32 {"

cp "$ORIG" "$SRC"
( cd "$W" && cargo build --release --offline >/dev/null 2>&1 )
echo "=== restored original; re-running clean suite ==="
( cd "$W" && cargo test --offline --release -- --test-threads=4 2>&1 | grep 'test result' )
