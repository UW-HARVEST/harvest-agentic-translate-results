#!/usr/bin/env bash
# Negative control for the differential test suite: deliberately break the Rust
# translation in small ways and verify that the tests DETECT each break.
# The pristine source is backed up inside the crate (never /tmp, which is
# shared) and restored unconditionally on exit.
set -u
cd "$(dirname "$0")"

PRISTINE=".lib.rs.pristine"
cp src/lib.rs "$PRISTINE"
restore() { cp "$PRISTINE" src/lib.rs; cargo build --release --offline >/dev/null 2>&1; rm -f "$PRISTINE"; }
trap restore EXIT

run_case() {
  local name="$1"; shift
  cp "$PRISTINE" src/lib.rs
  python3 - "$@" <<'PY'
import sys
old, new = sys.argv[1], sys.argv[2]
p = 'src/lib.rs'
s = open(p).read()
assert old in s, f"mutation anchor not found: {old!r}"
open(p, 'w').write(s.replace(old, new, 1))
PY
  if ! cargo build --release --offline >/dev/null 2>&1; then
    echo "MUT $name: BUILD FAILED (mutation invalid)"; return 1
  fi
  if timeout 600 cargo test --offline >/dev/null 2>&1; then
    echo "MUT $name: *** NOT DETECTED *** (tests still pass — suite is too weak)"
    return 1
  else
    echo "MUT $name: detected (tests fail, as required)"
    return 0
  fi
}

fail=0
run_case "wrong-needle-in-driver" \
  "foo(in_, b'x' as c_char)" "foo(in_, b'X' as c_char)" || fail=1
run_case "reject-negative-needle" \
  "    let needle = c as c_int;" "    let needle = c as c_int; if needle < 0 { return 0; }" || fail=1
run_case "null-check-added" \
  "    let mut s: *const c_char = in_;" "    if in_.is_null() { return 0; }
    let mut s: *const c_char = in_;" || fail=1
run_case "off-by-one-count" \
  "        res = res.wrapping_add(1);" "        res = res.wrapping_add(if res == 0 { 2 } else { 1 });" || fail=1
run_case "changed-format-string" \
  'b"A: %d\n\0"' 'b"A:%d\n\0"' || fail=1
run_case "no-advance-after-match" \
  "        s = s.add(1);" "        if res > 1000000 { break; }" || fail=1

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL MUTATIONS DETECTED — differential suite has real discriminating power"
else
  echo "SOME MUTATIONS SURVIVED — see above"
fi
exit "$fail"
