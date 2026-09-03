#!/usr/bin/env bash
# Mutation testing: inject a deliberate divergence into the Rust translation and
# assert the differential suite FAILS. A suite that passes on a mutant is not
# testing anything. Every mutation is reverted afterwards.
set -u
cd "$(dirname "$0")/.."
export CARGO_NET_OFFLINE=true

BK="${TMPDIR:-/tmp}/negctl"
mkdir -p "$BK"
cp src/mdcore.rs "$BK/mdcore.rs.bak"
cp src/mdconfig.rs "$BK/mdconfig.rs.bak"
restore() { cp "$BK/mdcore.rs.bak" src/mdcore.rs; cp "$BK/mdconfig.rs.bak" src/mdconfig.rs; }
trap restore EXIT

fail=0
run_mutant() { # name features
  local name="$1" feats="$2"
  cargo build --quiet --no-default-features --features "$feats" >/dev/null 2>&1
  if cargo test --quiet --no-default-features --features "$feats" -- --test-threads=1 >/dev/null 2>&1; then
    echo "  !! NOT DETECTED: $name (features=$feats)"
    fail=1
  else
    echo "  detected: $name (features=$feats)"
  fi
  restore
}

echo "negative controls:"

# M1: off-by-one on the DISPATCH_REP switch domain (accept n == 7).
sed -i 's/        0..=6 => {/        0..=7 => {/' src/mdconfig.rs
run_mutant "dispatch_rep accepts n==7" "add,5"

# M2: wrong INIT for mul.
sed -i 's/^pub const INIT: c_int = 1;/pub const INIT: c_int = 0;/' src/mdconfig.rs
run_mutant "INIT_mul == 0 instead of 1" "mul,5"

# M3: G_OP always points at op_add.
sed -i 's/^pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = mdconfig::op_fn();/pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = op_add;/' src/mdcore.rs
run_mutant "G_OP hardwired to op_add" "sub,5"

# M4: printf text drift in helper_call.
sed -i 's/helper.call={} helper.acc={}\\n/helper.call={}  helper.acc={}\\n/' src/mdcore.rs
run_mutant "helper_call print spacing" "add,5"

# M5: off-by-one in RUN_LOOP (<= instead of <).
sed -i 's/    while i < REPEAT {/    while i <= REPEAT {/' src/mdconfig.rs
run_mutant "run_loop iterates REPEAT+1 times" "add,3"

# M6: checked instead of wrapping in op_add (saturating divergence on overflow).
sed -i 's/^    a.wrapping_add(b)$/    a.saturating_add(b)/' src/mdcore.rs
run_mutant "op_add saturates instead of wrapping" "add,5"

# M7: use_generated forgets to print.
sed -i 's/^    out(&format!("gen.acc={}\\n", r));$//' src/mdcore.rs
run_mutant "use_generated does not print" "add,5"

# M8: G_OP_NAME reports the wrong op.
sed -i 's/^pub const OP_NAME_C: &\[u8\] = b"sub\\0";/pub const OP_NAME_C: \&[u8] = b"add\\0";/' src/mdconfig.rs
run_mutant "G_OP_NAME says add in a sub build" "sub,5"

# Restore and confirm the pristine tree passes again.
restore
cargo build --quiet >/dev/null 2>&1
if cargo test --quiet -- --test-threads=1 >/dev/null 2>&1; then
  echo "pristine tree: ok"
else
  echo "  !! pristine tree FAILS after restore"
  fail=1
fi

echo "negative controls done fail=$fail"
exit $fail
