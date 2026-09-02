#!/usr/bin/env bash
# Negative control / mutation testing.
#
# Injects a known bug into src/lib.rs, rebuilds the .so, and asserts the
# differential suite FAILS. A mutant that SURVIVES means the suite has a blind
# spot on that axis, so a green suite proves nothing there.
#
# Field delimiter is @@@ (cannot appear in Rust) because `::` collides with Rust
# path syntax. Replacements are exact literal substitutions done in Python, not
# sed, so raw-pointer syntax like `addr_of_mut!((*t).frame_header)` survives
# intact instead of being reinterpreted as regex metacharacters.
#
# Both profiles are exercised: release (how the .so ships) and debug (integer
# overflow checks and rustc's debug dereference checks are on). A mutant counts
# as killed if EITHER profile catches it; the per-mutant output records which.
#
# Usage: ./mutation_check.sh
set -uo pipefail
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; }
trap restore EXIT

mutate() { # <from> <to>  exact literal; exit 2 if <from> is not in the source
  FROM="$1" TO="$2" SRCF="$SRC" BAKF="$BAK" python3 - <<'PY'
import os, sys
text = open(os.environ["BAKF"]).read()
frm, to = os.environ["FROM"], os.environ["TO"]
if frm not in text:
    sys.exit(2)
open(os.environ["SRCF"], "w").write(text.replace(frm, to, 1))
PY
}

# echoes: "pass" | "fail" | "builderr"
run_suite() {
  local prof_flag="$1"
  if ! timeout 300 cargo build $prof_flag >/dev/null 2>&1; then echo builderr; return; fi
  if timeout 600 cargo test $prof_flag --tests >/dev/null 2>&1; then echo pass; else echo fail; fi
}

# name @@@ from @@@ to
MUTANTS=(
"blocksize 192 nibble wrong@@@192 => 0x01u32 << 12@@@192 => 0x02u32 << 12"
"blocksize default threshold off-by-one@@@if cur_blocksize <= 256@@@if cur_blocksize <= 257"
"blocksize 32768 nibble wrong@@@32768 => 0x0Fu32 << 12@@@32768 => 0x0Eu32 << 12"
"blocksize default arms swapped@@@0x06u32 << 12@@@0x07u32 << 12"
"samplerate 882000 'fixed' to 88200@@@882000 => frame_header@@@88200 => frame_header"
"samplerate 44100 nibble wrong@@@44100 => frame_header |= 0x09u32@@@44100 => frame_header |= 0x0Au32"
"samplerate 96000 nibble wrong@@@96000 => frame_header |= 0x0Bu32@@@96000 => frame_header |= 0x0Cu32"
"samplerate kHz range check off-by-one@@@if samplerate / 1000 < 256@@@if samplerate / 1000 <= 256"
"samplerate 65536 range check off-by-one@@@else if samplerate < 65536@@@else if samplerate <= 65536"
"samplerate daHz range check off-by-one@@@if samplerate / 10 < 65536@@@if samplerate / 10 <= 65536"
"samplerate d3 nibble wrong@@@frame_header |= 0x0Du32 << 8@@@frame_header |= 0x0Eu32 << 8"
"samplerate modulo 1000 -> 100@@@samplerate % 1000 == 0@@@samplerate % 100 == 0"
"samplerate modulo 10 -> 100@@@samplerate % 10 == 0@@@samplerate % 100 == 0"
"channel_mode modulus 4 -> 8@@@channel_mode % 4@@@channel_mode % 8"
"channel_mode modulus dropped@@@let mode: tflac_u8 = channel_mode % 4;@@@let mode: tflac_u8 = channel_mode;"
"channels underflow 'fixed' with saturating_sub@@@channels.wrapping_sub(1).wrapping_shl(4)@@@channels.saturating_sub(1).wrapping_shl(4)"
"channels shift 4 -> 5@@@channels.wrapping_sub(1).wrapping_shl(4)@@@channels.wrapping_sub(1).wrapping_shl(5)"
"left_side nibble wrong@@@LEFT_SIDE => frame_header |= 0x08u32@@@LEFT_SIDE => frame_header |= 0x09u32"
"side_right nibble wrong@@@SIDE_RIGHT => frame_header |= 0x09u32@@@SIDE_RIGHT => frame_header |= 0x0Au32"
"mid_side nibble wrong@@@MID_SIDE => frame_header |= 0x0Au32@@@MID_SIDE => frame_header |= 0x0Bu32"
"bitdepth 8 nibble wrong@@@8 => frame_header |= 1u32 << 1@@@8 => frame_header |= 2u32 << 1"
"bitdepth 16 nibble wrong@@@16 => frame_header |= 4u32 << 1@@@16 => frame_header |= 3u32 << 1"
"bitdepth 32 nibble wrong@@@32 => frame_header |= 7u32 << 1@@@32 => frame_header |= 6u32 << 1"
"bitdepth shift 1 -> 2@@@20 => frame_header |= 5u32 << 1@@@20 => frame_header |= 5u32 << 2"
"base constant wrong@@@0xFFF8u32 << 16@@@0xFFF9u32 << 16"
"base shift wrong@@@0xFFF8u32 << 16@@@0xFFF8u32 << 15"
"ORs into incoming frame_header@@@addr_of_mut!((*t).frame_header).write(frame_header);@@@addr_of_mut!((*t).frame_header).write(frame_header | addr_of!((*t).frame_header).read());"
"clobbers an input field@@@addr_of_mut!((*t).frame_header).write(frame_header);@@@addr_of_mut!((*t).frame_header).write(frame_header); addr_of_mut!((*t).channels).write(0);"
"writes the wrong field@@@addr_of_mut!((*t).frame_header).write(frame_header);@@@addr_of_mut!((*t).cur_blocksize).write(frame_header);"
"reads samplerate where blocksize is meant@@@let cur_blocksize: tflac_u32 = addr_of!((*t).cur_blocksize).read();@@@let cur_blocksize: tflac_u32 = addr_of!((*t).samplerate).read();"
"reference instead of raw ptr (turns the NULL fault into SIGABRT)@@@let cur_blocksize: tflac_u32@@@let _reborrow = &mut *t; let cur_blocksize: tflac_u32"
)

killed=0; survived=0; stale=0
echo "=== mutation testing: ${#MUTANTS[@]} mutants x 2 profiles ==="
for m in "${MUTANTS[@]}"; do
  desc="${m%%@@@*}"; rest="${m#*@@@}"; from="${rest%%@@@*}"; to="${rest#*@@@}"
  cp "$BAK" "$SRC"
  if ! mutate "$from" "$to"; then
    echo "STALE (pattern absent)  $desc"; stale=$((stale+1)); continue
  fi

  rel=$(run_suite "--release")
  dbg=$(run_suite "")
  if [ "$rel" = fail ] || [ "$dbg" = fail ]; then
    echo "killed   [release=$rel debug=$dbg]  $desc"; killed=$((killed+1))
  else
    echo "SURVIVED [release=$rel debug=$dbg]  (BLIND SPOT!)  $desc"; survived=$((survived+1))
  fi
done

cp "$BAK" "$SRC"
timeout 300 cargo build --release >/dev/null 2>&1
timeout 300 cargo build >/dev/null 2>&1
echo "=== killed=$killed survived=$survived stale=$stale ==="

# ---------------------------------------------------------------------------
# Known EQUIVALENT mutant: it changes the source but provably cannot change
# behaviour, so it is EXPECTED to survive. Stating it keeps the "0 survivors"
# result above honest instead of implying the suite is infinitely sensitive.
#
#   `if cur_blocksize <= 256` -> `< 256`
#     The two differ only at cur_blocksize == 256, and 256 is one of the 13
#     enumerated `match` arms, so the default arm is never reached with 256 —
#     the comparison is dead exactly at its own boundary.
# ---------------------------------------------------------------------------
echo "=== equivalent-mutant control (expected to SURVIVE) ==="
cp "$BAK" "$SRC"
mutate "if cur_blocksize <= 256" "if cur_blocksize < 256"
eq=$(run_suite "--release")
if [ "$eq" = pass ]; then
  echo "survived as expected  blocksize <= 256 -> < 256 (256 is an enumerated case)"
  equiv_ok=1
else
  echo "UNEXPECTEDLY $eq     blocksize <= 256 -> < 256"
  equiv_ok=0
fi

cp "$BAK" "$SRC"
timeout 300 cargo build --release >/dev/null 2>&1
timeout 300 cargo build >/dev/null 2>&1

[ "$survived" -eq 0 ] && [ "$stale" -eq 0 ] && [ "$equiv_ok" -eq 1 ]
