#!/bin/bash
# Harness sensitivity check (NOT part of the verification itself).
#
# Injects a known divergence into the Rust source, confirms the differential
# suite CATCHES it, then restores the source. A mutation that survives means the
# corresponding test is blind and the "all green" result would be meaningless.
#
# Usage: ./mutation_check.sh

set -u
cd "$(dirname "$0")" || exit 1

BAK="${TMPDIR:-/tmp}/src_backup_$$"
cp -a src "$BAK" || exit 1
# NOTE: the timestamps must NOT be preserved here. `cp -a` would restore the
# original (older) mtimes, cargo's fingerprint would consider the tree unchanged
# and the next build would silently keep the *mutant* artifacts around.
restore() {
  rm -rf src && cp -r "$BAK" src && find src -type f -exec touch {} +
}
trap 'restore; rm -rf "$BAK"' EXIT

survived=0

# run_mutation <name> <features> <test-filter> <sed/perl command...>
run_mutation() {
  local name="$1" feats="$2" filter="$3"; shift 3
  restore
  "$@" || { echo "  !! mutation command failed for $name"; survived=$((survived+1)); return; }

  if ! cargo build --no-default-features --features "$feats" --lib --bin driver \
        > "${TMPDIR:-/tmp}/mut.log" 2>&1; then
    echo "  !! mutant did not compile: $name"; survived=$((survived+1)); return
  fi

  if cargo test --no-default-features --features "$feats" -- "$filter" \
        >> "${TMPDIR:-/tmp}/mut.log" 2>&1; then
    echo "SURVIVED  $name  (tests '$filter' did NOT catch it)"
    survived=$((survived+1))
  else
    echo "caught    $name"
  fi
}

echo "=== mutation sensitivity check ==="

# M1: DISPATCH_REP's `default:` arm made to compute 7 steps instead of INIT.
run_mutation "dispatch_rep default arm computes instead of returning INIT" \
  "add,7" "use_generated" \
  sed -i 's|_ => acc,.*default: break.*|_ => rep_n(acc, 7),|' src/mdmacros.rs

# M2: RUN_LOOP unrolled one step short (REPEAT off by one).
run_mutation "RUN_LOOP off-by-one (REPEAT-1 steps)" \
  "mul,5" "helper_call" \
  sed -i 's|rep_n(INIT, REPEAT)|rep_n(INIT, REPEAT.saturating_sub(1))|' src/mdmacros.rs

# M3: helper_ptr routed through the G_OP global instead of OP_FN(OP).
run_mutation "helper_ptr reads G_OP instead of a local fp" \
  "add,5" "helper_ptr" \
  sed -i 's|let fp: OpFn = SELECTED_OP;|let fp: OpFn = unsafe { G_OP };|' src/mdcore.rs

# M4: op_mul computes an addition.
run_mutation "op_mul does addition" \
  "add,5" "op_mul" \
  sed -i '/fn op_mul/,/^}/ s|wrapping_mul|wrapping_add|' src/mdcore.rs

# M5: the writable globals turned back into read-only statics (the real bug that
#     was found and fixed -- .data.rel.ro + RELRO makes a consumer's store fault).
run_mutation "G_OP demoted to an immutable static (.data.rel.ro)" \
  "add,5" "g_op" \
  sed -i 's|pub static mut G_OP: OpFn|pub static G_OP: OpFn|' src/mdcore.rs

# M6: printf format string altered by one space.
run_mutation "helper_call printf format changed by one space" \
  "add,5" "helper_call" \
  sed -i 's|helper.call=%d helper.acc=%d|helper.call=%d  helper.acc=%d|' src/mdcore.rs

# M7: atoi saturates at INT_MAX instead of truncating LONG_MAX.
run_mutation "atoi saturates instead of truncating" \
  "add,5" "atoi" \
  sed -i 's|i64::MAX as c_int|c_int::MAX|' src/main.rs

# M8: op_sub no longer exported.
run_mutation "op_sub loses its #[no_mangle] export" \
  "add,5" "symbol" \
  perl -0pi -e 's/#\[unsafe\(no_mangle\)\]\npub extern "C" fn op_sub/pub extern "C" fn op_sub/' src/mdcore.rs

# M9: INIT_FOR(mul) wrong (1 -> 0).
run_mutation "INIT_mul changed from 1 to 0" \
  "mul,5" "" \
  sed -i '/^mod op_sel {$/,/^}/ s|pub const INIT: c_int = 1;|pub const INIT: c_int = 0;|' src/mdmacros.rs

# M10: STEP_mul uses i instead of i+1.
run_mutation "STEP_mul drops the +1" \
  "mul,4" "" \
  sed -i 's|acc.wrapping_mul(i.wrapping_add(1))|acc.wrapping_mul(i)|' src/mdmacros.rs

# M11: G_OP_NAME spelled wrong.
run_mutation "G_OP_NAME text wrong for OP=sub" \
  "sub,5" "g_op_name" \
  sed -i 's|OP_NAME_C: &\[u8\] = b"sub\\0"|OP_NAME_C: \&[u8] = b"sup\\0"|' src/mdmacros.rs

# M12: main's summary uses a different operand order/omission.
run_mutation "main summary omits x3" \
  "add,5" "" \
  perl -0pi -e 's/\.wrapping_add\(x3\)\n//' src/main.rs

echo "-----------------------------------------------"
if [ "$survived" -ne 0 ]; then
  echo "$survived mutation(s) SURVIVED -- the suite has blind spots"
  exit 1
fi
echo "all mutations caught -- the differential suite is sensitive"
