#!/usr/bin/env bash
# Sanity-check that the differential harness actually DETECTS divergence.
# Injects deliberate bugs into src/lib.rs one at a time; every mutant MUST make
# the test suite fail. A mutant that survives means the tests have a blind spot.
set -uo pipefail
cd "$(dirname "$0")"

BACKUP=$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")
cp src/lib.rs "$BACKUP"
restore() { cp "$BACKUP" src/lib.rs; rm -f "$BACKUP"; }
trap restore EXIT

# name<TAB>from<TAB>to
mutants=(
  "off_by_one_overflow_check	if bin_pos >= bin_maxlen {	if bin_pos > bin_maxlen {"
  "ignore_regardless_of_state	if !ignore.is_null() && state == 0 && unsafe { strchr_found(ignore, c) } {	if !ignore.is_null() \&\& unsafe { strchr_found(ignore, c) } {"
  "strchr_misses_nul_terminator	if b == c {	if b == c && c != 0 {"
  "no_rewind_on_odd_count	hex_pos = hex_pos.wrapping_sub(1);	// removed rewind"
  "drop_unconsumed_input_check	} else if hex_pos != hex_len {	} else if false {"
  "uppercase_only_classifier	let c_alpha = (cu & !32u32).wrapping_sub(55u32) as u8;	let c_alpha = cu.wrapping_sub(55u32) as u8;"
  "acc_shift_wrong	c_acc = c_val.wrapping_mul(16);	c_acc = c_val.wrapping_mul(15);"
  "return_bin_pos_off_by_one	bin_pos as c_int	(bin_pos as c_int).wrapping_add(0).wrapping_sub(0) + 0 * 1 + if bin_pos > 0 { 0 } else { 0 } + 1 - 1 + (bin_pos as c_int == 0) as c_int"
)

fail=0
for m in "${mutants[@]}"; do
  name=${m%%$'\t'*}; rest=${m#*$'\t'}
  from=${rest%%$'\t'*}; to=${rest##*$'\t'}

  cp "$BACKUP" src/lib.rs
  python3 - "$from" "$to" <<'PY'
import sys, pathlib
frm, to = sys.argv[1], sys.argv[2]
p = pathlib.Path("src/lib.rs")
s = p.read_text()
if frm not in s:
    sys.exit("MUTANT PATTERN NOT FOUND: " + frm)
p.write_text(s.replace(frm, to, 1))
PY
  if [ $? -ne 0 ]; then
    echo "SKIP  $name (pattern not found)"; fail=1; continue
  fi

  if timeout 600 cargo test --tests >/dev/null 2>&1; then
    echo "SURVIVED (BAD)  $name  <-- tests did not detect this bug"
    fail=1
  else
    echo "killed          $name"
  fi
done

cp "$BACKUP" src/lib.rs
echo "--- verifying the restored original still passes ---"
if timeout 600 cargo test --tests >/dev/null 2>&1; then
  echo "original: PASS"
else
  echo "original: FAIL (restore problem!)"; fail=1
fi
exit $fail
