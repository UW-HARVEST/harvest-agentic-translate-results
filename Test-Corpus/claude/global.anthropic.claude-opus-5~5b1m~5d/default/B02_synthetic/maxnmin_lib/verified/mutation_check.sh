#!/usr/bin/env bash
# Sensitivity check for the differential suite: deliberately break the Rust
# translation in a handful of ways and confirm the tests CATCH each break.
# A test suite that passes no matter what the Rust does proves nothing.
#
# src/lib.rs is restored on every exit path.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
cd "$here" || exit 1
orig="$(mktemp)"
cp src/lib.rs "$orig"
restore() { cp "$orig" src/lib.rs; rm -f "$orig"; cargo build --offline --release >/dev/null 2>&1; }
trap restore EXIT

rc=0
run_mutation() {
    local name="$1" from="$2" to="$3" tests="$4"
    cp "$orig" src/lib.rs
    if ! python3 - "$from" "$to" <<'PY'
import sys
src = open('src/lib.rs').read()
frm, to = sys.argv[1], sys.argv[2]
n = src.count(frm)
if n == 0:
    sys.stderr.write("PATTERN NOT FOUND: %r\n" % frm)
    sys.exit(1)
open('src/lib.rs','w').write(src.replace(frm, to, 1))
PY
    then
        echo "SKIP  $name (pattern not found)"; rc=1; return
    fi
    if ! cargo build --offline --release >/dev/null 2>&1; then
        echo "SKIP  $name (mutant does not compile)"; rc=1; return
    fi
    # shellcheck disable=SC2086
    if cargo test --offline --release $tests >/dev/null 2>&1; then
        echo "NOT CAUGHT  $name  <-- suite is blind to this mutation!"
        rc=1
    else
        echo "caught      $name"
    fi
}

echo "=== mutation sensitivity check ==="
run_mutation "capacity off-by-one (MAX_NODES-1)" \
    "if count >= MAX_NODES as c_int {" "if count >= MAX_NODES as c_int - 1 {" \
    "--test error_paths --test valid_paths"
run_mutation "name truncation off-by-one" \
    "while i < MAX_NAME_LEN - 1 {" "while i < MAX_NAME_LEN - 2 {" \
    "--test error_paths --test valid_paths"
# NOTE: `>` -> `>=` in the INT_MAX/INT_MIN comparisons is *behaviour
# preserving* (d == (double)INT_MAX falls through to `(int)d`, which yields
# INT_MAX anyway), so it is deliberately NOT used as a mutation. A mutation of
# the clamp VALUE is observable:
run_mutation "clamp returns INT_MAX-1" \
    "        return c_int::MAX;" "        return c_int::MAX - 1;" \
    "--test error_paths --test valid_paths"
run_mutation "active treated as == 1 instead of != 0" \
    "if (*n).id == id && (*n).active != 0 {" "if (*n).id == id && (*n).active == 1 {" \
    "--test error_paths --test valid_paths"
run_mutation "NaN operand order in the accumulator" \
    "sum = add_c_order(calculate_subtree_sum((*n).id), sum);" \
    "sum += calculate_subtree_sum((*n).id);" \
    "--test valid_paths"
run_mutation "euclidean remainder instead of C truncating %" \
    "let node_id = (param1.wrapping_rem(6)).wrapping_add(1);" \
    "let node_id = (param1.rem_euclid(6)).wrapping_add(1);" \
    "--test error_paths --test valid_paths"
run_mutation "non-zero struct padding" \
    "let mut staging: MaybeUninit<Node> = MaybeUninit::zeroed();" \
    "let mut staging: MaybeUninit<Node> = MaybeUninit::zeroed(); unsafe { ptr::write_bytes(staging.as_mut_ptr() as *mut u8, 0xAA, size_of::<Node>()); }" \
    "--test valid_paths"
run_mutation "subtree sum of missing node returns -0.0" \
    "        return 0.0;" "        return -0.0;" \
    "--test error_paths"
run_mutation "process_string drops the sign extension" \
    "result = result.wrapping_add(*p as c_int);" \
    "result = result.wrapping_add(*p as u8 as c_int);" \
    "--test error_paths --test valid_paths"
run_mutation "children count includes inactive nodes" \
    "if (*n).parent_id == parent_id && (*n).active != 0 {" \
    "if (*n).parent_id == parent_id {" \
    "--test error_paths --test valid_paths"

echo "=================================="
if (( rc == 0 )); then echo "ALL MUTATIONS CAUGHT"; else echo "SOME MUTATIONS NOT CAUGHT"; fi
exit $rc
