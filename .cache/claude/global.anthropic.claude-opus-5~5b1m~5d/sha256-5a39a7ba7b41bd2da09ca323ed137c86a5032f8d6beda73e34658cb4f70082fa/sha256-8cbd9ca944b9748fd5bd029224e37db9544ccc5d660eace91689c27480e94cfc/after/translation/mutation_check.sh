#!/usr/bin/env bash
# Meta-verification: prove the differential test suite is not vacuous.
#
# Injects known-wrong changes into translation/src/lib.rs one at a time, rebuilds
# the cdylib, and checks the suite FAILS for each. A mutant that survives is
# either a genuine gap in the tests or a provably equivalent mutant (see the two
# documented cases at the bottom).
#
# src/lib.rs is backed up and restored after every mutant, and the restore is
# verified by checksum.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

BAK="$PWD/.lib.rs.orig"
cp src/lib.rs "$BAK" || { echo "FATAL: could not back up src/lib.rs"; exit 1; }
SUM_GOOD=$(md5sum < "$BAK" | cut -d' ' -f1)
trap 'cp "$BAK" src/lib.rs 2>/dev/null; rm -f "$BAK"' EXIT

# Make sure the C reference exists.
( mkdir -p ../c_src/build && cd ../c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "FATAL: C build failed"; exit 1; }

escaped=0

mutate () {
  local name="$1" old="$2" new="$3" expect="${4:-caught}"
  cp "$BAK" src/lib.rs
  if ! OLD="$old" NEW="$new" python3 -c "
import os, sys
p='src/lib.rs'; s=open(p).read()
o,n = os.environ['OLD'], os.environ['NEW']
if o not in s: sys.exit('pattern not found: '+o[:70])
open(p,'w').write(s.replace(o,n,1))
"; then
    echo "MUTANT [$name]: SKIPPED (pattern not found)"
    cp "$BAK" src/lib.rs
    return
  fi

  cargo build --release --lib --offline >/dev/null 2>&1
  local out failed passed
  out=$(timeout 600 cargo test --release --offline 2>&1)
  failed=$(echo "$out" | grep -cE '^test .*FAILED')
  passed=$(echo "$out" | grep -cE '^test .* \.\.\. ok')

  if [[ "$failed" -gt 0 ]]; then
    echo "MUTANT [$name]: CAUGHT ($failed failed / $passed ok)"
  elif [[ "$expect" == "equivalent" ]]; then
    echo "MUTANT [$name]: survived, as expected (provably equivalent mutant)"
  else
    echo "MUTANT [$name]: *** ESCAPED *** ($passed ok) -- TEST GAP"
    escaped=1
  fi

  cp "$BAK" src/lib.rs
  [[ "$(md5sum < src/lib.rs | cut -d' ' -f1)" == "$SUM_GOOD" ]] \
    || { echo "FATAL: restore of src/lib.rs failed"; exit 1; }
}

echo "=== mutants that MUST be caught ==============================="
mutate "m__base[300] 0x8000->0x8001" \
  '0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8001,' \
  '0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8001, 0x8001,'
mutate "index mask 0x1ff -> 0xff" \
  'let j: u32 = (n >> 23) & 0x1ff;' 'let j: u32 = (n >> 23) & 0xff;'
mutate "mantissa mask -> 0x3fffff" \
  '(n & 0x007f_ffff) >> shift' '(n & 0x003f_ffff) >> shift'
mutate "off-by-one result (round-to-nearest 'fix')" \
  'base.wrapping_add(mantissa) as u16' 'base.wrapping_add(mantissa).wrapping_add(1) as u16'
mutate "remove the #[no_mangle] export wrapper" \
  '#[unsafe(no_mangle)]' ''
mutate "exponent shift 23 -> 22" \
  'let j: u32 = (n >> 23) & 0x1ff;' 'let j: u32 = (n >> 22) & 0x1ff;'
mutate "swap the two tables' roles (base<->shift)" \
  'let base = M_BASE[j] as u32;' 'let base = M_SHIFT[j] as u32;'

echo
echo "=== mutants that are PROVABLY EQUIVALENT (survival expected) ==="
# The maximum attainable base + (mantissa >> shift) is exactly 0xffff -- asserted
# for all 512 buckets by errors_truncating_cast -- so saturation never triggers.
mutate "sum in u16 with saturating_add" \
  'base.wrapping_add(mantissa) as u16' '(base as u16).saturating_add(mantissa as u16)' \
  equivalent
# A mantissa is 23 bits, so m >> 23 and m >> 24 are both identically zero:
# shifts of 23 and 24 are indistinguishable through the public API.
mutate "m__shift[103] 23 -> 24" \
  '0x18, 0x18, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13,' \
  '0x18, 0x18, 0x18, 0x18, 0x16, 0x15, 0x14, 0x13,' \
  equivalent

cargo build --release --lib --offline >/dev/null 2>&1
echo
if [[ $escaped -eq 0 ]]; then
  echo "RESULT: no unexpected escapes -- the differential suite is non-vacuous."
else
  echo "RESULT: at least one mutant ESCAPED -- the suite has a gap."
fi
exit $escaped
