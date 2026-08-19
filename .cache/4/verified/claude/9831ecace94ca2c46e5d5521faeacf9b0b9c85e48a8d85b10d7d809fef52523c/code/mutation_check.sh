#!/usr/bin/env bash
# NEGATIVE CONTROL: deliberately break the Rust translation in several small,
# realistic ways and prove the differential suite CATCHES each one. A suite that
# passes against a broken translation proves nothing.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

SRC="src/lib.rs"
BAK="$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; }
trap restore EXIT

rc=0
run_mutant() {
  local name="$1"
  echo "=============================================================="
  echo ">>> mutant: $name"
  echo "=============================================================="
  if ! timeout 600 cargo build --offline >/dev/null 2>&1; then
    echo "  mutant did not compile — inconclusive, treating as FAILURE"
    rc=1
    return
  fi
  if timeout 600 cargo test --offline --test differential >"${TMPDIR:-/tmp}/mut.log" 2>&1; then
    echo "  !!! SUITE PASSED against a BROKEN translation — the tests are blind"
    tail -5 "${TMPDIR:-/tmp}/mut.log"
    rc=1
  else
    echo "  suite correctly FAILED. Cases that caught it:"
    grep -E '^test .* FAILED' "${TMPDIR:-/tmp}/mut.log" | sed 's/^/    /' | head -20
    echo "    (total caught: $(grep -cE '^test .* FAILED' "${TMPDIR:-/tmp}/mut.log"))"
  fi
  cp "$BAK" "$SRC"
}

# 1. wrong printf conversion: "%2x" instead of "%02x" (loses zero padding)
python3 - <<'PY'
import re
p="src/lib.rs"; s=open(p).read()
s=s.replace('c"%02x"', 'c"%2x"')
open(p,"w").write(s)
PY
run_mutant 'printf("%2x") instead of printf("%02x")'

# 2. byte order reversed (big-endian instead of native)
python3 - <<'PY'
p="src/lib.rs"; s=open(p).read()
s=s.replace('x.to_ne_bytes()', 'x.to_be_bytes()')
open(p,"w").write(s)
PY
run_mutant 'to_be_bytes() instead of to_ne_bytes()'

# 3. NaN canonicalisation (value-preserving but not bit-preserving)
python3 - <<'PY'
p="src/lib.rs"; s=open(p).read()
s=s.replace('let bytes: [u8; 4] = x.to_ne_bytes();',
            'let bytes: [u8; 4] = (if x.is_nan() { f32::NAN } else { x }).to_ne_bytes();')
open(p,"w").write(s)
PY
run_mutant 'quiets signalling NaNs to the canonical NaN'

# 4. off-by-one length: prints 3 bytes instead of 4
python3 - <<'PY'
p="src/lib.rs"; s=open(p).read()
s=s.replace('core::mem::size_of::<f32>() as c_int', '3 as c_int')
open(p,"w").write(s)
PY
run_mutant 'len = 3 instead of sizeof(float)'

# 5. uppercase hex
python3 - <<'PY'
p="src/lib.rs"; s=open(p).read()
s=s.replace('c"%02x"', 'c"%02X"')
open(p,"w").write(s)
PY
run_mutant 'printf("%02X") — uppercase hex'

# 6. missing trailing newline
python3 - <<'PY'
p="src/lib.rs"; s=open(p).read()
s=s.replace('printf(c"\\n".as_ptr());', '{}')
open(p,"w").write(s)
PY
run_mutant 'no trailing newline'

# 7. signed byte promotion (the classic char-vs-unsigned-char bug)
python3 - <<'PY'
p="src/lib.rs"; s=open(p).read()
s=s.replace('printf(c"%02x".as_ptr(), byte as c_int);',
            'printf(c"%02x".as_ptr(), (byte as i8) as c_int);')
open(p,"w").write(s)
PY
run_mutant 'signed char promotion of the byte'

# 8. -0.0 folded to +0.0
python3 - <<'PY'
p="src/lib.rs"; s=open(p).read()
s=s.replace('let bytes: [u8; 4] = x.to_ne_bytes();',
            'let bytes: [u8; 4] = (if x == 0.0 { 0.0f32 } else { x }).to_ne_bytes();')
open(p,"w").write(s)
PY
run_mutant '-0.0 folded to +0.0'

restore
trap - EXIT
echo "=============================================================="
timeout 600 cargo build --offline >/dev/null 2>&1
if [ "$rc" -eq 0 ]; then
  echo "MUTATION CHECK: PASS — every mutant was caught by the suite"
else
  echo "MUTATION CHECK: FAIL — at least one mutant slipped through"
fi
exit "$rc"
