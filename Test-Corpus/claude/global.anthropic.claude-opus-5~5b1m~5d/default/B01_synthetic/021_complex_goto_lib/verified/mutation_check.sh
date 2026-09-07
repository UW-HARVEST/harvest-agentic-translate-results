#!/usr/bin/env bash
# Mutation coverage check.
#
# Confirms the differential test suite is actually sensitive: each mutation
# below perturbs one branch/statement of the Rust translation, and the suite
# MUST fail for every one of them. A surviving mutant means that branch of the C
# code is not really being compared.
set -u
cd "$(dirname "$0")"

BAK=$(mktemp)
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap 'restore; rm -f "$BAK"' EXIT

# name | sed expression
MUTANTS=(
  "special_case_y|s/x == 1 \&\& y == 4/x == 1 \&\& y == 5/"
  "special_case_x|s/x == 1 \&\& y == 4/x == 2 \&\& y == 4/"
  "special_case_dropped|s/let mut skip_label1 = x == 1 \&\& y == 4;/let mut skip_label1 = false;/"
  "outer_guard_and|s/while x > 0 || y > 0/while x > 0 \&\& y > 0/"
  "outer_guard_ge|s/while x > 0 || y > 0/while x >= 0 || y > 0/"
  "label1_guard|s/if x > 0 {/if x >= 0 {/"
  "y_zero_check|s/if y == 0 {/if y == 1 {/"
  "backward_goto_bound|s/if x < 3 {/if x < 4 {/"
  "backward_goto_removed|s/if x < 3 {/if false {/"
  "x_decrement|s/x = x.wrapping_sub(1)/x = x.wrapping_sub(2)/"
  "y_decrement|s/y = y.wrapping_sub(1)/y = y.wrapping_sub(2)/"
  "swap_x_y_labels|s/print_lit(b\"x\\\\n\\\\0\")/print_lit(b\"y\\\\n\\\\0\")/"
  "loop_banner|s/print_lit(b\"loop\\\\n\\\\0\")/print_lit(b\"lop\\\\n\\\\0\")/"
  "skip_reset_removed|s/^\([[:space:]]*\)skip_label1 = false;/\1\/\/ removed/"
)

# Negative control: a semantics-preserving edit must NOT be reported as a
# failure. If this "kills", the harness is reporting spurious divergences (e.g.
# it is loading a stale `.so`) and every kill below is meaningless.
echo "== negative control (semantics-preserving edit must PASS) =="
sed -i 's/^use std::ffi::{c_char, c_int};/\/\/ null mutant\nuse std::ffi::{c_char, c_int};/' src/lib.rs
if timeout 300 cargo test --release --test configs --test errors -- --skip child_helper \
     >/dev/null 2>&1; then
  echo "OK: null mutant survived"
else
  echo "BROKEN HARNESS: the null mutant was killed — kills below are not trustworthy"
  restore
  exit 1
fi
restore

pass=0
survived=()
for m in "${MUTANTS[@]}"; do
  name=${m%%|*}
  expr=${m#*|}
  restore
  sed -i "$expr" src/lib.rs
  if cmp -s "$BAK" src/lib.rs; then
    echo "SKIP  $name (sed expression matched nothing)"
    survived+=("$name (not applied)")
    continue
  fi
  if timeout 300 cargo test --release --test configs --test errors -- --skip child_helper \
       >/dev/null 2>&1; then
    echo "SURVIVED  $name  <-- test suite is blind to this"
    survived+=("$name")
  else
    echo "killed    $name"
    pass=$((pass + 1))
  fi
done

restore
echo
echo "killed $pass/${#MUTANTS[@]} mutants"
if [ ${#survived[@]} -ne 0 ]; then
  printf 'survivors: %s\n' "${survived[*]}"
  exit 1
fi
echo "all mutants killed"
