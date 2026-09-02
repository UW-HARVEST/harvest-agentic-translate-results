#!/usr/bin/env bash
# Mutation-tests the differential suite: each mutation injects a deliberate
# divergence into src/lib.rs and the suite MUST fail. A mutation that survives
# is a blind spot in the tests, not a success.
#
# Usage: ./mutate.sh
set -uo pipefail
cd "$(dirname "$0")"

ORIG=$(mktemp /tmp/lib.rs.pristine.XXXXXX)
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; }
trap 'restore; rm -f "$ORIG"' EXIT

pass=0
fail=0

# run_mutation <description> <sed-expression>
run_mutation() {
  local desc="$1" expr="$2"
  restore
  sed -i "$expr" src/lib.rs
  if diff -q "$ORIG" src/lib.rs >/dev/null; then
    echo "SKIP   $desc  (sed matched nothing)"
    fail=$((fail + 1))
    return
  fi
  if ! timeout 600 cargo build --release >/dev/null 2>&1; then
    echo "SKIP   $desc  (mutant does not compile)"
    fail=$((fail + 1))
    return
  fi
  if timeout 600 cargo test --release >/tmp/mutation.log 2>&1; then
    echo "SURVIVED  $desc   <-- BLIND SPOT"
    fail=$((fail + 1))
  else
    local why
    why=$(grep -m1 -oE "^test [a-z0-9_]+ \.\.\. FAILED" /tmp/mutation.log | sed 's/^test //;s/ \.\.\..*//')
    echo "caught    $desc   (first failing test: ${why:-<process aborted>})"
    pass=$((pass + 1))
  fi
}

run_mutation "subtract_op returns a+b"            's/^    a\.wrapping_sub(b)$/    a.wrapping_add(b)/'
run_mutation "multiply_op returns a+b"            's/^    a\.wrapping_mul(b)$/    a.wrapping_add(b)/'
run_mutation "add_op saturates instead of wraps"  's/^    a\.wrapping_add(b)$/    a.saturating_add(b)/'
run_mutation "divide_op wraps (loses SIGFPE)"     's|^    unsafe { idiv_i32(a, b).0 }$|    a.wrapping_div(b)|'
run_mutation "modulo_op wraps (loses SIGFPE)"     's|^    unsafe { idiv_i32(a, b).1 }$|    a.wrapping_rem(b)|'
run_mutation "divide_op drops the b==0 guard"     '0,/^pub extern "C" fn divide_op/!{s/^        return 0;$//}'
run_mutation "label[31] NUL not forced"           's/^        (\*node)\.label\[31\] = 0;$//'
run_mutation "strncpy skips the zero padding"     's/^            \*dst\.add(i) = 0;$//'
run_mutation "add_tree_node fills right first"    's/if (\*parent)\.left_child_id == -1 {/if (*parent).right_child_id == -1 {/'
run_mutation "add_tree_node capacity off-by-one"  's/if node_count >= MAX_NODES as c_int {/if node_count > MAX_NODES as c_int {/'
run_mutation "find_node_by_id skips index 0"      's/^        let mut i: c_int = 0;$/        let mut i: c_int = 1;/'
run_mutation "calculate_tree_sum ignores right"   's/if (\*node)\.right_child_id != -1 {/if false {/'
run_mutation "parse_operation drops NULL check"   "s/if op_str.is_null() || !strchr(op_str, '+' as c_int).is_null() {/if !strchr(op_str, '+' as c_int).is_null() {/"
run_mutation "parse_operation swaps * and -"      "s/return OP_MULTIPLY;/return OP_SUBTRACT;/"
run_mutation "get_operation_func default is mul"  's/^        _ => add_op,$/        _ => multiply_op,/'
run_mutation "rodata offset off by one"           's/const OP_STRING_OFFSET: isize = 26;/const OP_STRING_OFFSET: isize = 27;/'
run_mutation "op index uses rem_euclid"           's/tree_sum\.wrapping_rem(4)/tree_sum.rem_euclid(4)/'
run_mutation "inreftree clears node_table"        's/^        node_count = 0;$/        node_count = 0; node_table_ptr().write_bytes(0, MAX_NODES);/'
run_mutation "inreftree target check drops ==0"   's/if target\.is_null() || (\*target)\.value == 0 {/if target.is_null() {/'

restore
timeout 600 cargo build --release >/dev/null 2>&1
echo
echo "mutations caught: $pass    survived/skipped: $fail"
[ "$fail" -eq 0 ]
