#!/usr/bin/env bash
# Sanity-check the differential suite by deliberately breaking the Rust
# translation and confirming the tests FAIL. A suite that passes on a mutated
# translation is not verifying anything.
#
# src/lib.rs is restored on every exit path, including Ctrl-C.
set -uo pipefail
cd "$(dirname "$0")"

BACKUP=$(mktemp)
cp src/lib.rs "$BACKUP"
restore() { cp "$BACKUP" src/lib.rs; rm -f "$BACKUP"; }
trap restore EXIT INT TERM

pass=0
fail=0

mutate() {
  local name="$1"; shift
  cp "$BACKUP" src/lib.rs
  "$@"
  if ! cmp -s "$BACKUP" src/lib.rs; then
    :
  else
    echo "MUTATION NOT APPLIED: $name (source unchanged)"; fail=$((fail+1)); return
  fi
  cargo build --quiet 2>/dev/null
  if timeout 600 cargo test --quiet -- --test-threads=1 >/dev/null 2>&1; then
    echo "NOT DETECTED: $name  <-- suite is blind to this bug"
    fail=$((fail+1))
  else
    echo "detected:     $name"
    pass=$((pass+1))
  fi
}

m_drop_forward_goto() {
  # Ignore the `x == 1 && y == 4` forward `goto label2`.
  perl -0pi -e 's/let mut skip_label1 = x == 1 && y == 4;/let mut skip_label1 = false;/' src/lib.rs
}
m_persist_forward_goto() {
  # Let the one-shot skip persist for the whole outer iteration.
  perl -0pi -e 's/^\s*skip_label1 = false;$//m' src/lib.rs
}
m_backedge_off_by_one() {
  # `x < 3` -> `x <= 3`
  perl -0pi -e 's/if x < 3 \{/if x <= 3 {/' src/lib.rs
}
m_backedge_retests_guard() {
  # `goto label1` (stay in the body) -> `continue` (re-test the while guard)
  perl -0pi -e "s/continue 'body;/continue 'outer;/" src/lib.rs
}
m_continue_becomes_break() {
  # `continue` at `y == 0` -> leave the loop entirely
  perl -0pi -e "s/if y == 0 \{\n                continue 'outer;/if y == 0 {\n                break 'outer;/" src/lib.rs
}
m_guard_uses_and() {
  # `x > 0 || y > 0` -> `x > 0 && y > 0`
  perl -0pi -e 's/if !\(x > 0 \|\| y > 0\)/if !(x > 0 \&\& y > 0)/' src/lib.rs
}
m_label1_guard() {
  # `x > 0` at label1 -> `x >= 0`
  perl -0pi -e 's/if x > 0 \{\n                    c_print/if x >= 0 {\n                    c_print/' src/lib.rs
}
m_terminate_instead_of_diverge() {
  # Return early on the one input class where the C never returns.
  perl -0pi -e 's/    let mut x = x;\n    let mut y = y;/    let mut x = x;\n    let mut y = y;\n    if x > 0 \&\& y < 0 { return; }/' src/lib.rs
}
m_swap_print_order() {
  # Emit "y" before decrementing differently: swap the loop/x message text.
  perl -0pi -e 's/c_print\(b"loop\\n\\0"\);/c_print(b"pool\\n\\0");/' src/lib.rs
}

mutate "forward goto dropped"                 m_drop_forward_goto
mutate "forward-goto skip persists"           m_persist_forward_goto
mutate "back-edge boundary x<3 -> x<=3"       m_backedge_off_by_one
mutate "back-edge re-tests while guard"       m_backedge_retests_guard
mutate "y==0 continue -> break"               m_continue_becomes_break
mutate "loop guard || -> &&"                  m_guard_uses_and
mutate "label1 guard x>0 -> x>=0"             m_label1_guard
mutate "diverging class returns early"        m_terminate_instead_of_diverge
mutate "loop message text changed"            m_swap_print_order

echo
echo "mutations detected: $pass   undetected: $fail"
[ "$fail" -eq 0 ]
