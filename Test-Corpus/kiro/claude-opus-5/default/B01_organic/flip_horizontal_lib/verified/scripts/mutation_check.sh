#!/usr/bin/env bash
# Anti-vacuity check: deliberately break the Rust translation in several
# semantically NON-equivalent ways and assert the differential suite CATCHES
# each one. A suite that passes against a broken translation proves nothing.
#
# Every mutation is applied to a copy-on-write of src/lib.rs, which is restored
# (and checksum-verified) afterwards.
set -uo pipefail
cd "$(dirname "$0")/.."

GOLDEN=scripts/lib.rs.golden
SRC=src/lib.rs
cp "$SRC" "$GOLDEN"
GOLD_SUM=$(md5sum < "$GOLDEN")

restore() { cp "$GOLDEN" "$SRC"; }
trap restore EXIT

FAILED=0
n=0

# $1 = description, $2 = test binary to run, $3.. = sed expressions
mutate() {
  local desc="$1"; shift
  local bin="$1"; shift
  n=$((n+1))
  restore
  for expr in "$@"; do sed -i "$expr" "$SRC"; done
  if ! cmp -s "$SRC" "$GOLDEN"; then :; else
    printf '\033[31mM%d SKIPPED (sed matched nothing): %s\033[0m\n' "$n" "$desc"
    FAILED=1; return
  fi

  # cargo test does NOT emit the cdylib -> build it explicitly first.
  if ! timeout 300 cargo build --lib >/tmp/mut_build.log 2>&1; then
    printf '\033[31mM%d did not compile: %s\033[0m\n' "$n" "$desc"
    tail -12 /tmp/mut_build.log
    FAILED=1; return
  fi

  if timeout 400 cargo test --test "$bin" >/tmp/mut_test.log 2>&1; then
    printf '\033[31mM%d NOT DETECTED (suite still passed): %s\033[0m\n' "$n" "$desc"
    FAILED=1
  else
    local res
    res=$(grep -E '^test result' /tmp/mut_test.log | tail -1)
    printf '\033[32mM%d detected: %s\033[0m\n    %s\n' "$n" "$desc" "${res:-<process died>}"
  fi
}

# --- M1: drop the alpha channel from the pixel swap -------------------------
mutate "swap drops the alpha channel" phase_b_configs \
  's|core::ptr::swap(a, b);|{ let t = *a; (*a).r = (*b).r; (*a).g = (*b).g; (*a).b = (*b).b; *b = t; }|'

# --- M2: skip the last column of every row ----------------------------------
mutate "inner loop stops one column early" phase_b_configs \
  's|while j < w {|while j < w - 1 {|'

# --- M3: off-by-one in the mirrored row index (h-i-1 -> h-i) ----------------
mutate "mirrored row index off by one" phase_b_configs \
  's|w.wrapping_mul(h.wrapping_sub(i).wrapping_sub(1))|w.wrapping_mul(h.wrapping_sub(i))|'

# --- M4: swap rows in the wrong direction (a and b both walk forward from a) -
mutate "second row pointer duplicates the first" phase_b_configs \
  's|let off_b = |let off_b = 0 * |'

# --- M5: defensive NULL-img guard the C does not have -----------------------
mutate "adds a NULL-img guard C does not have" phase_c_errors \
  's|    let pix: \*mut cp_pixel_t = (\*img).pix;|    if img.is_null() { return; }\n    let pix: *mut cp_pixel_t = (*img).pix;|'

# --- M6: defensive NULL-pix guard the C does not have -----------------------
mutate "adds a NULL-pix guard C does not have" phase_c_errors \
  's|    let w: c_int = (\*img).w;|    if pix.is_null() { return; }\n    let w: c_int = (*img).w;|'

# --- M7: negative height is no longer a silent no-op ------------------------
mutate "negative height perturbs the buffer instead of no-op" phase_c_errors \
  's|    let mut i: c_int = 0;|    if h < 0 { (*pix).r ^= 1; return; }\n    let mut i: c_int = 0;|'

# --- M8: negative width is no longer a silent no-op -------------------------
mutate "negative width perturbs the buffer instead of no-op" phase_c_errors \
  's|    let mut i: c_int = 0;|    if w < 0 { (*pix).g ^= 0x80; return; }\n    let mut i: c_int = 0;|'

restore
trap - EXIT
NEW_SUM=$(md5sum < "$SRC")
if [[ "$GOLD_SUM" != "$NEW_SUM" ]]; then
  printf '\033[31msrc/lib.rs was NOT restored correctly\033[0m\n'; FAILED=1
else
  printf '\nsrc/lib.rs restored (md5 matches golden)\n'
fi
rm -f "$GOLDEN"

# Leave a correct cdylib behind.
timeout 300 cargo build --lib >/dev/null 2>&1

echo
if [[ $FAILED == 0 ]]; then
  printf '\033[32mANTI-VACUITY CHECK PASSED: all %d mutations detected\033[0m\n' "$n"
else
  printf '\033[31mANTI-VACUITY CHECK FAILED\033[0m\n'
fi
exit $FAILED
