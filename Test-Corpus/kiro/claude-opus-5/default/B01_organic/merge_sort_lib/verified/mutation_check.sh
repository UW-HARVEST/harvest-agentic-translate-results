#!/usr/bin/env bash
# Mutation-adequacy check for the differential test suite.
#
# Each mutant introduces ONE deliberate divergence from the C into
# translation/src/lib.rs. A mutant that survives (0 failing tests) means the
# suite has a blind spot on that branch. Every mutant MUST be killed.
set -u
cd "$(dirname "$0")"

ORIG=$(mktemp); cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; }
trap 'restore; rm -f "$ORIG"' EXIT

# Baseline must be green.
restore
cargo build -q 2>/dev/null
if ! timeout 600 cargo test -q >/dev/null 2>&1; then
  echo "BASELINE IS RED — fix before mutation testing"; exit 1
fi
echo "baseline: green"
echo

survivors=0
mutate() { # name  from  to
  name="$1"; from="$2"; to="$3"
  restore
  python3 - "$from" "$to" <<'EOF'
import sys, difflib
p = 'src/lib.rs'
before = open(p).read()
frm, to = sys.argv[1], sys.argv[2]
if frm not in before:
    sys.stderr.write("PATTERN NOT FOUND: %r\n" % frm); sys.exit(2)
after = before.replace(frm, to, 1)
if after == before:
    sys.stderr.write("MUTATION IS A NO-OP: %r -> %r\n" % (frm, to)); sys.exit(3)
# Guard against the classic mistake of mutating a doc comment that happens to
# quote the code: at least one changed line must be real code.
changed = [
    l[2:] for l in difflib.unified_diff(before.splitlines(), after.splitlines(), n=0)
    if l.startswith(('+', '-')) and not l.startswith(('+++', '---'))
]
if not any(
    ls and not ls.startswith('//') and not ls.startswith('*') and not ls.startswith('///')
    for ls in (c.strip() for c in changed)
):
    sys.stderr.write(
        "MUTATION ONLY TOUCHED COMMENTS (%r); make the pattern unique to code\n" % frm)
    sys.exit(4)
open(p, 'w').write(after)
EOF
  if [ $? -ne 0 ]; then echo "!! $name: could not apply mutation"; survivors=$((survivors+1)); return; fi
  if ! cargo build -q 2>/dev/null; then
    echo "-- $name: does not compile (skipped)"; return
  fi
  # A mutant is KILLED if `cargo test` fails for ANY reason: assertion failure,
  # signal (SIGSEGV from an out-of-bounds write / stack overflow), or timeout.
  log=$(mktemp)
  timeout 300 cargo test 2>&1 >"$log"
  rc=$?
  n=$(grep -cE '^test .* FAILED' "$log")
  crash=""
  if grep -qiE 'stack overflow|SIGSEGV|signal|abort' "$log"; then crash=" [crash]"; fi
  if [ "$rc" -eq 124 ]; then crash=" [timeout]"; fi
  rm -f "$log"
  if [ "$rc" -ne 0 ]; then
    echo "KILLED   $name  (rc=$rc, $n assertion failures$crash)"
  else
    echo "SURVIVED $name  <-- BLIND SPOT"
    survivors=$((survivors+1))
  fi
}

# --- comparison function -------------------------------------------------
mutate "cmp: <= becomes <  (revives the dead texture_id tiebreak)" \
  'if a.sort_bits <= b.sort_bits {' 'if a.sort_bits < b.sort_bits {'
mutate "cmp: <= becomes >=" \
  'if a.sort_bits <= b.sort_bits {' 'if a.sort_bits >= b.sort_bits {'
mutate "cmp: signed compare becomes unsigned" \
  'if a.sort_bits <= b.sort_bits {' 'if (a.sort_bits as u32) <= (b.sort_bits as u32) {'
mutate "cmp: swap operands" \
  'if a.sort_bits <= b.sort_bits {' 'if b.sort_bits <= a.sort_bits {'

# --- struct copy width ---------------------------------------------------
mutate "copy: 12 bytes instead of 16 (drops padding)" \
  'dst.cast::<u8>(), SPRITE_SIZE)' 'dst.cast::<u8>(), 12)'
mutate "copy: 8 bytes (texture_id only)" \
  'dst.cast::<u8>(), SPRITE_SIZE)' 'dst.cast::<u8>(), 8)'

# --- merge iteration ----------------------------------------------------
mutate "iter: i < split becomes i <= split" \
  'let take_left = i < split' 'let take_left = i <= split'
mutate "iter: j >= hi becomes j > hi" \
  '&& (j >= hi
                || unsafe {' '&& (j > hi
                || unsafe {'
mutate "iter: j >= hi becomes j >= hi - 1" \
  '&& (j >= hi
                || unsafe {' '&& (j >= hi - 1
                || unsafe {'
mutate "iter: drop the j>=hi short-circuit guard" \
  'let take_left = i < split
            && (j >= hi
                || unsafe {' 'let take_left = i < split
            && (false
                || unsafe {'
mutate "iter: take right branch on tie (invert take_left)" \
  'if take_left {' 'if !take_left {'
mutate "iter: loop bound k < hi becomes k <= hi" \
  'while k < hi {' 'while k <= hi {'
mutate "iter: loop bound k < hi becomes k < hi - 1" \
  'while k < hi {' 'while k < hi - 1 {'
mutate "iter: forget to advance i" \
  'i = i.wrapping_add(1);' 'i = i.wrapping_add(0);'
mutate "iter: forget to advance j" \
  'j = j.wrapping_add(1);' 'j = j.wrapping_add(0);'

# --- recursion ----------------------------------------------------------
mutate "recurse: base case <= 1 becomes <= 0" \
  'if hi.wrapping_sub(lo) <= 1 {' 'if hi.wrapping_sub(lo) <= 0 {'
mutate "recurse: base case <= 1 becomes <= 2" \
  'if hi.wrapping_sub(lo) <= 1 {' 'if hi.wrapping_sub(lo) <= 2 {'
mutate "recurse: split rounds up instead of down" \
  'let split = lo.wrapping_add(hi) / 2;' 'let split = lo.wrapping_add(hi).wrapping_add(1) / 2;'
mutate "recurse: split biased toward lo" \
  'let split = lo.wrapping_add(hi) / 2;' 'let split = lo.wrapping_add(lo.wrapping_add(hi) / 2) / 2;'
mutate "recurse: buffer roles NOT swapped on the recursive calls" \
  'spritebatch_internal_merge_sort_recurse(a, lo, split, b);
        spritebatch_internal_merge_sort_recurse(a, split, hi, b);' \
  'spritebatch_internal_merge_sort_recurse(b, lo, split, a);
        spritebatch_internal_merge_sort_recurse(b, split, hi, a);'
mutate "recurse: final merge reads/writes the wrong buffer" \
  'spritebatch_internal_merge_sort_iteration(b, lo, split, hi, a);' \
  'spritebatch_internal_merge_sort_iteration(a, lo, split, hi, b);'
mutate "recurse: skip the second half" \
  'spritebatch_internal_merge_sort_recurse(a, split, hi, b);' \
  '{}'

# --- merge_sort entry ---------------------------------------------------
mutate "entry: skip the initial memcpy" \
  'if bytes != 0 {' 'if false {'
mutate "entry: memcpy one element short" \
  'let bytes = SPRITE_SIZE.wrapping_mul(size as usize);' \
  'let bytes = SPRITE_SIZE.wrapping_mul((size as usize).saturating_sub(1));'
mutate "entry: recurse over 0..size-1" \
  'spritebatch_internal_merge_sort_recurse(b, 0, size, a)' \
  'spritebatch_internal_merge_sort_recurse(b, 0, size - 1, a)'
mutate "entry: swap a and b at the top level" \
  'spritebatch_internal_merge_sort_recurse(b, 0, size, a)' \
  'spritebatch_internal_merge_sort_recurse(a, 0, size, b)'

echo
restore
cargo build -q 2>/dev/null
if [ "$survivors" -eq 0 ]; then
  echo "RESULT: all mutants killed — no blind spots found."
else
  echo "RESULT: $survivors mutant(s) SURVIVED — the suite has blind spots."
  exit 1
fi
