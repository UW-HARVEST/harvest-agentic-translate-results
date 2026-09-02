#!/usr/bin/env bash
# Mutation check: deliberately break the Rust translation in behaviour-changing
# ways and confirm the differential suite FAILS each time. A suite that passes a
# mutant is not verifying anything.
#
# Fields are separated by '@@' (patterns contain '|', so '|' cannot be the
# delimiter). src/lib.rs is always restored, even on interrupt.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(dirname "$here")"
src="$crate/src/lib.rs"
backup="$(mktemp)"
cp "$src" "$backup"
trap 'cp "$backup" "$src"; rm -f "$backup"; (cd "$crate" && cargo build --release >/dev/null 2>&1); echo "[restored src/lib.rs]"' EXIT INT TERM

mutants=(
'drop the hex_pos-- on a trailing odd nibble@@hex_pos = hex_pos.wrapping_sub(1);@@// mutant'
'shift c_num0 by 7 instead of 8@@let c_num0: u8 = ((c_num as u32).wrapping_sub(10) >> 8) as u8;@@let c_num0: u8 = ((c_num as u32).wrapping_sub(10) >> 7) as u8;'
'xor c with 47 instead of 48@@let c_num: u8 = c ^ 48u8;@@let c_num: u8 = c ^ 47u8;'
'mask with !16 instead of !32 (case folding)@@((c as u32 & !32u32).wrapping_sub(55))@@((c as u32 & !16u32).wrapping_sub(55))'
'subtract 54 instead of 55 in c_alpha@@.wrapping_sub(55)) as u8;@@.wrapping_sub(54)) as u8;'
'drop the second xor term in c_alpha0@@let c_alpha0: u8 = (((c_alpha as u32).wrapping_sub(10)@@let c_alpha0: u8 = (((c_alpha as u32).wrapping_sub(10) & 0'
'drop the strchr NUL-terminator quirk@@        if b == c {@@        if b == c && c != 0 {'
'drop the hex_pos != hex_len error branch@@    } else if hex_pos != hex_len {@@    } else if false {'
'drop the state == 0 guard on the ignore set@@if !ignore.is_null() && state == 0u8 &&@@if !ignore.is_null() &&'
'drop the *16 shift for the high nibble@@c_acc = c_val.wrapping_mul(16);@@c_acc = c_val;'
'off-by-one on the buffer-full check@@if bin_pos >= bin_maxlen {@@if bin_pos > bin_maxlen {'
'write to bin on the even nibble too@@            c_acc = c_val.wrapping_mul(16);@@            c_acc = c_val.wrapping_mul(16); if bin_pos < bin_maxlen { unsafe { *bin.add(bin_pos) = c_acc }; }'
'swap the two halves of c_val@@let c_val: u8 = (c_num0 & c_num) | (c_alpha0 & c_alpha);@@let c_val: u8 = (c_alpha0 & c_num) | (c_num0 & c_alpha);'
'return bin_pos even on error@@    if ret != 0 {\n        return ret;\n    }@@    if false {\n        return ret;\n    }'
'treat hex_end_p as always non-null@@    if !hex_end_p.is_null() {@@    if true {'
)

caught=0
missed=0
skipped=0
i=0
for m in "${mutants[@]}"; do
  i=$((i+1))
  desc="${m%%@@*}"
  rest="${m#*@@}"
  from="${rest%%@@*}"
  to="${rest#*@@}"

  cp "$backup" "$src"
  python3 - "$src" "$from" "$to" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
frm = frm.replace('\\n', '\n'); to = to.replace('\\n', '\n')
s = open(path).read()
if frm not in s:
    sys.exit(3)
open(path, 'w').write(s.replace(frm, to, 1))
PY
  rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "SKIP  mutant $i ($desc): pattern not found"
    skipped=$((skipped+1)); continue
  fi

  if ! (cd "$crate" && timeout 300 cargo build --release >/dev/null 2>&1); then
    echo "SKIP  mutant $i ($desc): does not compile"
    skipped=$((skipped+1)); continue
  fi

  if (cd "$crate" && HEX2BIN_RUST_SO="$crate/target/release/libhex2bin_lib.so" \
      timeout 300 cargo test --release >/dev/null 2>&1); then
    echo "BAD   mutant $i ($desc): suite PASSED a broken translation"
    missed=$((missed+1))
  else
    echo "GOOD  mutant $i ($desc): suite caught it"
    caught=$((caught+1))
  fi
done

echo
echo "mutants caught: $caught   missed: $missed   skipped: $skipped"
if [ "$missed" -ne 0 ] || [ "$skipped" -ne 0 ]; then
  echo "MUTATION CHECK: NOT CLEAN"
  exit 1
fi
echo "behaviour-changing mutants: all $caught detected"

# ---------------------------------------------------------------------------
# Provably EQUIVALENT mutants: edits that cannot change observable behaviour.
# The suite is EXPECTED to pass these; a failure would mean the reasoning below
# is wrong (or the suite depends on implementation detail rather than the ABI).
# ---------------------------------------------------------------------------
echo
echo "=== provably equivalent mutants (expected to PASS) ==="
equivalents=(
# `if (ret != 0) bin_pos = 0;` is a DEAD STORE: `ret` can never return to 0
# after being set, so the very next `if (ret != 0) return ret;` always fires
# and `bin_pos` is never read again.
'dead store: skip zeroing bin_pos on error@@        bin_pos = 0;@@        bin_pos = bin_pos.wrapping_add(0);'
# `state` is only ever compared against 0, so 0/1 is indistinguishable from
# the C 0x00/0xFF produced by `state = ~state`.
'state toggles 0/1 instead of 0x00/0xFF@@state = !state;@@state = if state == 0 { 1 } else { 0 };'
# c_val is always a nibble (0..=15), so *16 and <<4 agree.
'high nibble via shift instead of multiply@@c_acc = c_val.wrapping_mul(16);@@c_acc = c_val << 4;'
)
eq_ok=0
eq_bad=0
j=0
for m in "${equivalents[@]}"; do
  j=$((j+1))
  desc="${m%%@@*}"; rest="${m#*@@}"; from="${rest%%@@*}"; to="${rest#*@@}"
  cp "$backup" "$src"
  python3 - "$src" "$from" "$to" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
frm = frm.replace('\\n', '\n'); to = to.replace('\\n', '\n')
s = open(path).read()
if frm not in s:
    sys.exit(3)
open(path, 'w').write(s.replace(frm, to, 1))
PY
  if [ $? -ne 0 ] || ! (cd "$crate" && timeout 300 cargo build --release >/dev/null 2>&1); then
    echo "SKIP  equivalent $j ($desc): pattern not found or does not compile"
    eq_bad=$((eq_bad+1)); continue
  fi
  if (cd "$crate" && HEX2BIN_RUST_SO="$crate/target/release/libhex2bin_lib.so" \
      timeout 300 cargo test --release >/dev/null 2>&1); then
    echo "GOOD  equivalent $j ($desc): passed, as predicted"
    eq_ok=$((eq_ok+1))
  else
    echo "BAD   equivalent $j ($desc): suite FAILED an equivalent edit"
    eq_bad=$((eq_bad+1))
  fi
done

echo
echo "equivalent mutants confirmed: $eq_ok   unexpected: $eq_bad"
if [ "$eq_bad" -ne 0 ]; then
  echo "MUTATION CHECK: NOT CLEAN"
  exit 1
fi
echo "MUTATION CHECK: PASS"
