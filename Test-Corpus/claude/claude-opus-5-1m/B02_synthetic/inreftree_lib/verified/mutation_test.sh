#!/bin/bash
# Suite-adequacy check (mutation testing).
#
# A differential suite that passes proves nothing unless it can FAIL. This script
# injects a known bug into src/lib.rs, runs the whole suite, and reports whether
# the suite caught it. Every mutant must be CAUGHT; an ESCAPED mutant is a
# blind spot in the tests.
#
# src/lib.rs is restored (and rebuilt) at the end.
set -u
cd "$(dirname "$0")"
ORIG=$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")
LOG="${TMPDIR:-/tmp}/mutation.log"
cp src/lib.rs "$ORIG"
trap 'cp "$ORIG" src/lib.rs; rm -f "$ORIG"; cargo build --release --lib >/dev/null 2>&1' EXIT

CAUGHT=0; ESCAPED=0; EQUIV=0
run_mutant() {
  local name="$1"; shift
  cp "$ORIG" src/lib.rs
  if ! python3 - "$@" <<'PY'
import sys
p = 'src/lib.rs'
old, new = sys.argv[1], sys.argv[2]
t = open(p).read()
assert t.count(old) == 1, f"pattern not unique ({t.count(old)}x)"
open(p, 'w').write(t.replace(old, new))
PY
  then echo "  PATCH-FAIL  $name"; return; fi

  # The mutant must compile, otherwise the result is meaningless.
  if ! timeout 600 cargo build --release --lib > "$LOG" 2>&1; then
    echo "  NO-COMPILE  $name (invalid mutant, not a blind spot)"; EQUIV=$((EQUIV+1)); return
  fi
  out=$(C_SO=c_src/build/libtranslated_rust.so timeout 600 cargo test --release 2>&1)
  # NB: check for failures BEFORE checking for compile errors - cargo prints
  # "error: test failed" on a normal test failure.
  if echo "$out" | grep -qE "test result: FAILED|SIGABRT|SIGSEGV|panicked|STALE ARTIFACT"; then
    f=$(echo "$out" | grep -oE "^test [a-z_0-9]+ \.\.\. FAILED" \
        | sed 's/^test //;s/ \.\.\. FAILED//' | sort -u | head -4 | tr '\n' ' ')
    echo "  CAUGHT      $name  <- ${f:-(abort/panic)}"; CAUGHT=$((CAUGHT+1))
  elif echo "$out" | grep -qE "could not compile"; then
    echo "  NO-COMPILE  $name (test targets)"; EQUIV=$((EQUIV+1))
  else
    echo "  ESCAPED     $name   *** BLIND SPOT ***"; ESCAPED=$((ESCAPED+1))
  fi
}

echo "=== arithmetic entry points ==="
run_mutant "add_op off-by-one"        'a.wrapping_add(b)
}' 'a.wrapping_add(b).wrapping_add(1)
}'
run_mutant "subtract_op reversed"     'a.wrapping_sub(b)
}' 'b.wrapping_sub(a)
}'
run_mutant "modulo uses euclidean rem" 'a.wrapping_rem(b)
}' 'a.rem_euclid(b)
}'
run_mutant "divide uses euclidean div" 'a.wrapping_div(b)
}' 'a.div_euclid(b)
}'
run_mutant "divide by zero -> 1"      'if b == 0 {
        return 0;
    }
    a.wrapping_div(b)' 'if b == 0 {
        return 1;
    }
    a.wrapping_div(b)'

echo "=== libc helpers ==="
run_mutant "strncpy drops the zero-padding" 'while i < n {
        *dst.add(i) = 0;
        i += 1;
    }' '// mutant: padding removed'
run_mutant "strchr misses a needle at the last position" \
'let ch = *p as u8;
        if ch == c {
            return p;
        }
        if ch == 0 {
            return std::ptr::null();
        }' \
'let ch = *p as u8;
        if ch == 0 || *p.add(1) as u8 == 0 {
            return std::ptr::null();
        }
        if ch == c {
            return p;
        }'

echo "=== table / layout ==="
run_mutant "find returns the LAST match" 'if (*node).id == id {
                return node;
            }' 'if (*node).id == id && i + 1 == count {
                return node;
            }'
run_mutant "find loop off-by-one" 'while i < count {
            let node = base.offset(i as isize);
            if (*node).id == id {' 'while i <= count {
            let node = base.offset(i as isize);
            if (*node).id == id {'
run_mutant "capacity check off-by-one" 'if node_count >= MAX_NODES as c_int {' 'if node_count > MAX_NODES as c_int {'
run_mutant "MAX_NODES 51" 'const MAX_NODES: usize = 50;' 'const MAX_NODES: usize = 51;'
run_mutant "node_table one row short" \
'pub static mut node_table: [TreeNode; MAX_NODES] = [EMPTY_NODE; MAX_NODES];' \
'pub static mut node_table: [TreeNode; MAX_NODES - 1] = [EMPTY_NODE; MAX_NODES - 1];'
run_mutant "TreeNode id/value swapped" 'pub struct TreeNode {
    pub id: c_int,
    pub value: c_int,' 'pub struct TreeNode {
    pub value: c_int,
    pub id: c_int,'
run_mutant "label[31] terminator removed" '*label_ptr.add(31) = 0;' '// mutant: no terminator'
run_mutant "roll back the row on parent failure" \
'if parent.is_null() || (*parent).id != parent_id {
                return -1;
            }' \
'if parent.is_null() || (*parent).id != parent_id {
                std::ptr::write_bytes(node as *mut u8, 0, 52);
                return -1;
            }'
run_mutant "right slot filled before left" \
'if (*parent).left_child_id == -1 {
                (*parent).left_child_id = id;
            } else if (*parent).right_child_id == -1 {
                (*parent).right_child_id = id;
            }' \
'if (*parent).right_child_id == -1 {
                (*parent).right_child_id = id;
            } else if (*parent).left_child_id == -1 {
                (*parent).left_child_id = id;
            }'
run_mutant "node_count advanced twice" 'node_count = node_count.wrapping_add(1);
        node_count.wrapping_sub(1)' 'node_count = node_count.wrapping_add(2);
        node_count.wrapping_sub(2)'
run_mutant "index returned before increment" 'node_count = node_count.wrapping_add(1);
        node_count.wrapping_sub(1)' 'let r = node_count;
        node_count = node_count.wrapping_add(1);
        r + 1'

echo "=== recursion ==="
run_mutant "sum starts at 0 not node->value" 'let mut sum = (*node).value;' 'let mut sum: c_int = 0;'
run_mutant "sum skips the right subtree" 'if (*node).right_child_id != -1 {
            sum = sum.wrapping_add(calculate_tree_sum((*node).right_child_id));
        }' '// mutant: right subtree dropped'

echo "=== dispatch ==="
run_mutant "parse checks '*' before '+'" \
"if op_str.is_null() || !c_strchr(op_str, b'+').is_null() {
            return OP_ADD;
        }" \
"if !c_strchr(op_str, b'*').is_null() {
            return OP_MULTIPLY;
        }
        if op_str.is_null() || !c_strchr(op_str, b'+').is_null() {
            return OP_ADD;
        }"
run_mutant "parse(NULL) -> OP_MODULO" \
"if op_str.is_null() || !c_strchr(op_str, b'+').is_null() {
            return OP_ADD;
        }" \
"if op_str.is_null() {
            return OP_MODULO;
        }
        if !c_strchr(op_str, b'+').is_null() {
            return OP_ADD;
        }"
run_mutant "OP_MODULO = 6" 'pub const OP_MODULO: c_int = 5;' 'pub const OP_MODULO: c_int = 6;'
run_mutant "get_operation_func default = modulo_op" '_ => Some(add_op),' '_ => Some(modulo_op),'
run_mutant "get_operation_func default = NULL" '_ => Some(add_op),' '_ => None,'

echo "=== inreftree ==="
run_mutant "label scan looks for 'r' not 'l'" \
"if !c_strchr(label_ptr, b'l').is_null() {" "if !c_strchr(label_ptr, b'r').is_null() {"
run_mutant "tree_sum taken from node 2" 'let tree_sum = calculate_tree_sum(1);' 'let tree_sum = calculate_tree_sum(2);'
run_mutant "target fallback ignores value==0" 'if target.is_null() || (*target).value == 0 {
            target_id = 1;
        }' 'if target.is_null() {
            target_id = 1;
        }'
run_mutant "operand order swapped" \
'let result = (func.unwrap())(tree_sum, target_id, 0, 0);' \
'let result = (func.unwrap())(target_id, tree_sum, 0, 0);'
run_mutant "node_count not reset" 'node_count = 0;

        add_tree_node(1, param1, -1,' 'add_tree_node(1, param1, -1,'
run_mutant 'label "left" -> "LEFT"' \
'add_tree_node(2, param2, 1, b"left\0".as_ptr() as *const c_char);' \
'add_tree_node(2, param2, 1, b"LEFT\0".as_ptr() as *const c_char);'

echo "=== the out-of-bounds op_string read (ERRORS.md row 26) ==="
run_mutant "rodata offset off-by-one" 'const OP_STRING_OFFSET: isize = 26;' 'const OP_STRING_OFFSET: isize = 25;'
run_mutant "rodata byte at -2 becomes an operator" \
'static RODATA: [u8; 31] = *b"root\0left\0right\0left-left\0+*-%\0";' \
'static RODATA: [u8; 31] = *b"root\0left\0right\0left-lef*\0+*-%\0";'
run_mutant "negative remainder index made positive" \
'*op_string.offset(tree_sum.wrapping_rem(4) as isize),' \
'*op_string.offset(tree_sum.wrapping_rem(4).unsigned_abs() as isize),'

printf '\n=== mutation score: %d caught, %d escaped, %d invalid/non-compiling ===\n' \
  "$CAUGHT" "$ESCAPED" "$EQUIV"
if [ "$ESCAPED" -ne 0 ]; then
  echo "FAIL: the suite is blind to $ESCAPED mutant(s)"
  exit 1
fi
echo "PASS: every valid mutant was caught"
