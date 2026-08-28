#!/usr/bin/env bash
# Negative control for the differential suite: deliberately break the Rust
# translation in one specific way, rebuild the cdylib, and require the suite to
# FAIL. A suite that cannot fail proves nothing, so this script is what makes
# the "all green" result of run_all.sh meaningful.
#
# Each mutation is  NAME::sedprog[;;sedprog...]  and is applied to src/lib.rs.
# The original file is always restored, including on error/interrupt.
set -uo pipefail
cd "$(dirname "$0")"
LOG="$PWD/target/verify-logs"; mkdir -p "$LOG"
SRC=src/lib.rs
BAK="$LOG/lib.rs.orig"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; cargo build --release >/dev/null 2>&1; }
trap restore EXIT INT TERM

C_SO="$(find ../c_src/build -maxdepth 1 -name '*.so' | head -1)"
[ -n "$C_SO" ] || { echo "build the C library first"; exit 2; }

# `0,/re/s` restricts the substitution to the FIRST match, which is how the two
# structurally identical `*src.offset(i)` loads are told apart.
LOAD='let v: f32 = unsafe { \*src.offset(i as isize) };'

MUTS=(
  "M01 relax the sum>0 guard to sum>=0::s|if sum > 0.0f32|if sum >= 0.0f32|"

  "M02 accumulate the sum of squares in f64::\
s|let mut sum: f32 = 0.0f32;|let mut sum: f64 = 0.0f64;|;;\
s|sum += v \* v;|sum += (v as f64) * (v as f64);|;;\
s|if sum > 0.0f32 {|if sum > 0.0f64 {|;;\
s|sum = 1.0f32 / sum.sqrt();|let sum: f32 = (1.0f64 / sum.sqrt()) as f32;|"

  "M03 drop the dest!=src aliasing guard::s|} else if dest as \*const f32 != src {|} else if true {|"

  "M04 write the output in reverse order::s|\*dest.offset(i as isize) = v \* sum|*dest.offset((size - 1 - i) as isize) = v * sum|"

  "M05 compute the rsqrt in f64 (double rounding)::s|sum = 1.0f32 / sum.sqrt();|sum = (1.0f64 / (sum as f64).sqrt()) as f32;|"

  "M06 wrong memset element size::s|wrapping_mul(size_of::<f32>())|wrapping_mul(1)|"

  "M07 accumulate src[i] instead of src[i]^2::s|sum += v \* v;|sum += v;|"

  "M08 accumulate in reverse order (FP reassociation)::0,/$LOAD/s||let v: f32 = unsafe { *src.offset((size - 1 - i) as isize) };|"

  "M09 invert the aliasing guard::s|dest as \*const f32 != src|dest as \*const f32 == src|"

  "M10 zero-fill with 0x01 instead of 0x00::s|write_bytes(dest as \*mut u8, 0u8, nbytes)|write_bytes(dest as *mut u8, 1u8, nbytes)|"

  "M11 off-by-one in the accumulation loop::0,/while i < size {/s||while i < size - 1 {|"

  "M12 off-by-one in the store loop::s|i = 0;|i = 1;|"

  "M13 multiply instead of divide for the scale::s|sum = 1.0f32 / sum.sqrt();|sum = 1.0f32 * sum.sqrt();|"

  "M14 forget to sign-extend the memset length::s|(size as usize).wrapping_mul|(size as u32 as usize).wrapping_mul|"
)

# M14 is expected to be UNDETECTABLE: both the C and the mutant compute a memset
# length of many gigabytes and both die with SIGSEGV, so no observable
# difference exists across the FFI boundary. It is listed to document that
# limitation rather than to be caught.
EXPECT_MISSED="M14"

pass=0; missed=0; broken=0
for entry in "${MUTS[@]}"; do
  name="${entry%%::*}"; progs="${entry#*::}"
  id="${name%% *}"
  cp "$BAK" "$SRC"
  IFS=';;' read -r -a _ignore <<<"x" # noop to keep shellcheck quiet
  # apply each sed program in turn
  ok=1
  while [ -n "$progs" ]; do
    case "$progs" in
      *";;"*) prog="${progs%%;;*}"; progs="${progs#*;;}" ;;
      *)      prog="$progs";        progs="" ;;
    esac
    [ -z "$prog" ] && continue
    sed -i "$prog" "$SRC" || ok=0
  done
  if [ "$ok" -eq 0 ] || diff -q "$BAK" "$SRC" >/dev/null; then
    echo "BROKEN $name : mutation did not change the source (stale sed pattern)"
    broken=$((broken+1)); continue
  fi
  if ! cargo build --release >"$LOG/mut-build.log" 2>&1; then
    echo "BROKEN $name : mutant does not compile"
    grep -E '^error' "$LOG/mut-build.log" | head -3
    broken=$((broken+1)); continue
  fi
  out="$LOG/mut-$id.log"
  NORM_C_SO="$C_SO" NORM_RUST_SO="$PWD/target/release/libnormalize_lib.so" \
    timeout 600 cargo test --release >"$out" 2>&1
  rc=$?
  nfail=$(grep -oE '[0-9]+ failed' "$out" | awk '{s+=$1} END {print s+0}')
  if [ "$rc" -ne 0 ]; then
    if [ "$id" = "$EXPECT_MISSED" ]; then
      echo "OK     $name -> caught ($nfail failing tests)  [was only expected to be documented]"
    else
      echo "OK     $name -> suite FAILED as required ($nfail failing tests)"
    fi
    pass=$((pass+1))
  else
    if [ "$id" = "$EXPECT_MISSED" ]; then
      echo "KNOWN  $name -> not observable across the FFI boundary (both sides SIGSEGV); documented in ERRORS.md"
      pass=$((pass+1))
    else
      echo "MISSED $name -> suite still PASSED: the tests are blind to this bug"
      missed=$((missed+1))
    fi
  fi
done

restore
trap - EXIT INT TERM
echo
echo "mutations handled: $pass, missed: $missed, broken: $broken"
if [ "$missed" -eq 0 ] && [ "$broken" -eq 0 ]; then
  echo "NEGATIVE CONTROL OK"
  exit 0
fi
echo "NEGATIVE CONTROL INCOMPLETE"
exit 1
