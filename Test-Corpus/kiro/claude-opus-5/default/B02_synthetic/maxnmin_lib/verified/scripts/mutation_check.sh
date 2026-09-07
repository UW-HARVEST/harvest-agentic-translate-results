#!/usr/bin/env bash
# Negative control for the differential suite: inject a known bug into the Rust
# translation, confirm the suite FAILS, then restore. A suite that passes on a
# mutated library is not verifying anything.
#
# A mutant counts as DETECTED if the test run fails in any way -- an assertion
# failure, or an abort (a Rust panic under `panic = "abort"` kills the process,
# which is itself a divergence from the C).
#
#   ./scripts/mutation_check.sh

set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

ORIG=$(mktemp); cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; }
trap 'restore; rm -f "$ORIG"' EXIT

MISSED=0

mutate() {
  local desc="$1" from="$2" to="$3"; shift 3
  restore
  grep -qF "$from" src/lib.rs || { printf '%-56s SKIP (pattern absent)\n' "$desc"; MISSED=1; return; }
  FROM="$from" TO="$to" python3 -c '
import os
p = open("src/lib.rs").read()
assert p.count(os.environ["FROM"]) >= 1
open("src/lib.rs","w").write(p.replace(os.environ["FROM"], os.environ["TO"], 1))
'
  if ! timeout 300 cargo build --release >/dev/null 2>&1; then
    printf '%-56s DETECTED (does not compile)\n' "$desc"; return
  fi
  if timeout 400 cargo test --release "$@" >/tmp/mut.log 2>&1; then
    printf '%-56s *** MISSED ***\n' "$desc"; MISSED=1
  else
    reason=$(grep -m1 -oE 'panicked at [^ ]+|assertion .left == right. failed|SIGABRT' /tmp/mut.log | head -1)
    printf '%-56s DETECTED (%s)\n' "$desc" "${reason:-test failure}"
  fi
}

echo "=== mutation / negative-control run ==="

mutate "modulus 6 -> 7 for param1"                  "let node_id = param1.wrapping_rem(6)"           "let node_id = param1.wrapping_rem(7)"           --test maxnmin_diff
mutate "modulus 6 -> 5 for param2"                  "let second_node_id = param2.wrapping_rem(6)"    "let second_node_id = param2.wrapping_rem(5)"    --test maxnmin_diff
mutate "modulus 3 -> 4 for param4"                  "let parent_id = param4.wrapping_rem(3)"         "let parent_id = param4.wrapping_rem(4)"         --test maxnmin_diff
mutate "children weight 10 -> 11"                   "children.wrapping_mul(10)"                      "children.wrapping_mul(11)"                      --test maxnmin_diff
mutate "find_node_by_id ignores active"             "if n.id == id && n.active != 0 {"               "if n.id == id {"                                --test errors_diff --test lowlevel_diff
mutate "get_children_count ignores active"          "if n.parent_id == parent_id && n.active != 0 {" "if n.parent_id == parent_id {"                  --test errors_diff --test lowlevel_diff
mutate "process_string treats char as unsigned"     "result = result.wrapping_add(*p as c_int);"     "result = result.wrapping_add(*p as u8 as c_int);" --test lowlevel_diff --test errors_diff
mutate "add_node capacity check off by one"         "if count as usize >= MAX_NODES {"               "if count as usize > MAX_NODES {"                --test errors_diff
mutate "add_node returns count, not count-1"        "node_count().wrapping_sub(1)"                   "node_count()"                                   --test lowlevel_diff --test errors_diff
mutate "add_node forgets to set active"             "        active: 1,"                             "        active: 0,"                             --test lowlevel_diff --test errors_diff
mutate "subtree sum skips children"                 "sum = addsd(calculate_subtree_sum(child_id), sum);" "sum = addsd(0.0, sum);"                     --test lowlevel_diff --test errors_diff
mutate "ADDSD NaN rule reverted to sum += child"    "sum = addsd(calculate_subtree_sum(child_id), sum);" "sum += calculate_subtree_sum(child_id);"    --test nan_propagation
mutate "subtree sum returns -0.0 when not found"    "        return 0.0;"                            "        return -0.0;"                          --test errors_diff --test lowlevel_diff
mutate "d2i clamps low side to 0 instead of INT_MIN" "        return c_int::MIN;"                    "        return 0;"                              --test errors_diff --test lowlevel_diff
mutate "d2i NaN returns 1 instead of 0"             "    if d != d {"                                "    if d != d { return 1; } if false {"          --test errors_diff --test lowlevel_diff
mutate "name truncated at 48 instead of 49"         "while i < MAX_NAME_LEN - 1 {"                   "while i < MAX_NAME_LEN - 2 {"                   --test lowlevel_diff --test errors_diff
mutate "maxnmin drops the name-sum term"            "result = result.wrapping_add(unsafe { process_string(name_ptr) });" "result = result.wrapping_add(0);" --test maxnmin_diff
mutate "maxnmin divides by param3 instead of param3+1" "(param3.wrapping_add(1)) as c_double"        "(param3) as c_double"                           --test maxnmin_diff
mutate "maxnmin uses param1-param2 in the quotient" "(param1.wrapping_add(param2)) as c_double"      "(param1.wrapping_sub(param2)) as c_double"      --test maxnmin_diff

echo
if [ "$MISSED" -eq 0 ]; then
  echo "ALL MUTANTS DETECTED -- the differential suite is sensitive."
else
  echo "SOME MUTANTS SURVIVED -- investigate whether they are behaviourally"
  echo "equivalent (unobservable through the public API) or a real test gap."
fi
exit $MISSED
