#!/usr/bin/env bash
# Anti-vacuity check: deliberately inject bugs into the Rust translation, rebuild
# the .so, and require the differential suite to FAIL for each one. A suite that
# passes a mutated translation is not actually testing anything.
#
# src/lib.rs is restored on every exit path.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT=$(pwd)
SRC="$ROOT/src/lib.rs"
BAK="$ROOT/target/lib.rs.mutation-backup"

mkdir -p "$ROOT/target"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; cargo build --offline --release >/dev/null 2>&1; }
trap restore EXIT

fail=0
# mutate <name> <expected-failing-test-filter> <python-replacement-expression>
mutate() {
  local name="$1" filter="$2" from="$3" to="$4"
  cp "$BAK" "$SRC"
  python3 - "$SRC" "$from" "$to" <<'PY'
import sys
path, a, b = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
assert a in s, f"mutation anchor not found: {a!r}"
open(path, 'w').write(s.replace(a, b, 1))
PY
  if [ $? -ne 0 ]; then echo "SKIP(anchor missing): $name"; fail=1; return; fi
  cargo build --offline --release >/dev/null 2>&1 || { echo "SKIP(build failed): $name"; fail=1; return; }
  if timeout 600 cargo test --offline --release -- --test-threads=2 $filter >"$ROOT/target/mutation.log" 2>&1; then
    echo "NOT DETECTED: $name  (suite passed a broken translation!)"
    fail=1
  else
    echo "detected:     $name  ($(grep -c 'FAILED' "$ROOT/target/mutation.log" | head -1) failure markers, filter '$filter')"
  fi
}

# 1. search restarts one byte after the match instead of past it: turns the
#    non-overlapping scan into an overlapping one.
mutate "overlapping restart" "c1 c17 c18 c27 c29" \
  'c_strstr(orig.add(inx_start + search_len), search)' \
  'c_strstr(orig.add(inx_start + 1), search)'

# 2. empty needle no longer matches at position 0 (would "fix" the C's infinite
#    loop -> must be reported as a divergence).
mutate "empty needle returns NULL" "e10 e11 e12" \
  'if needle_bytes.is_empty() {
        return haystack;
    }' \
  'if needle_bytes.is_empty() {
        return ptr::null();
    }'

# 3. the no-match early return copies the wrong string.
mutate "strdup(search) on no match" "c1 c2 c3" \
  'tmp = unsafe { strdup(orig) };' \
  'tmp = unsafe { strdup(search) };'

# 4. drop the allocation-failure check on the tail realloc: must turn a NULL
#    return into a crash.
mutate "missing tail OOM check" "e4" \
  '        total_bytes_allocated += orig_len - from;
        tmp = unsafe { realloc(tmp, total_bytes_allocated) };
        if tmp.is_null() {
            return ptr::null_mut();
        }' \
  '        total_bytes_allocated += orig_len - from;
        tmp = unsafe { realloc(tmp, total_bytes_allocated) };'

# 5. off-by-one in the tail copy source offset.
mutate "tail copy off by one" "c1 c4 c10 c28 c29" \
  'c_strncpy(tmp.add(tmp_offset), orig.add(from), orig_len - from)' \
  'c_strncpy(tmp.add(tmp_offset), orig.add(from), orig_len - from - 1)'

# 6. gap copy uses the wrong source offset.
mutate "gap copy source off by one" "c12 c16 c29" \
  'c_strncpy(tmp.add(tmp_offset), orig.add(from), gap)' \
  'c_strncpy(tmp.add(tmp_offset), orig.add(from + 1), gap)'

# 7. strncpy copies one byte too few (prefix / gap / tail copies truncate).
mutate "strncpy copies n-1 bytes" "c10 c12 c28 c29" \
  '    let mut i = 0usize;
    while i < n {' \
  '    let mut i = 0usize;
    while i + 1 < n {'

# 8. strstr returns the LAST occurrence instead of the first.
mutate "strstr returns last match" "c12 c16 c26 c29" \
  'for i in 0..=last {' \
  'for i in (0..=last).rev() {'

# 9. prefix guard flipped: skip copying the text before the first match.
mutate "prefix copy skipped" "c10 c11 c28 c29" \
  'unsafe { c_strncpy(tmp, orig, inx_start) };' \
  'unsafe { c_strncpy(tmp, orig, 0) };'

restore
trap - EXIT
echo "=================================================================="
if [ "$fail" -eq 0 ]; then
  echo "ALL MUTATIONS DETECTED (the suite is not vacuous)"
else
  echo "SOME MUTATIONS WERE NOT DETECTED"
fi
exit "$fail"
