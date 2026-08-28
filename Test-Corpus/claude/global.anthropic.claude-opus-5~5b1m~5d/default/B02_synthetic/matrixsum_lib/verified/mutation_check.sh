#!/usr/bin/env bash
# Sanity-check the differential suite itself: inject a deliberate bug into a
# COPY of the Rust crate (c_src and translation/ are never touched), build that
# copy's cdylib, point the test suite at it via RUST_SO, and require the suite to
# FAIL. A mutation that survives means the suite has a blind spot.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE="$PWD"
WORK="${TMPDIR:-/tmp}/mutcheck.$$"
C_SO="$(ls "$CRATE"/../c_src/build/lib*.so | head -1)"
SURVIVORS=()

# Mutants that are PROVABLY semantics-preserving, so no test can ever kill them.
# They are not blind spots — killing them would mean the suite is wrong.
#   * "process_flags no !! on read": has_read = flags & 0b0001 is already in
#     {0,1}, so `!!has_read == has_read` for every possible int.
#   * "matrixsum valid1 no !!": valid1 is consumed only by `if valid1 != 0`, and
#     `((x != 0) as int) != 0`  <=>  `x != 0`, for every possible int.
EQUIVALENT=("process_flags no !! on read" "matrixsum valid1 no !!")

is_equivalent() {
  local n="$1" e
  for e in "${EQUIVALENT[@]}"; do [ "$e" = "$n" ] && return 0; done
  return 1
}

# run_mutation <name> <sed expr for src/lib.rs> [sed expr for Cargo.toml]
#
# The mutant is built in BOTH profiles and the suite is run against each, because
# some divergences only appear in one profile (rustc's debug assertions add
# alignment/null aborts; LLVM's release inliner elides allocations).
run_mutation() {
  local name="$1" sed_expr="$2" cargo_expr="${3:-}"
  rm -rf "$WORK"; mkdir -p "$WORK"
  cp -r "$CRATE/src" "$CRATE/Cargo.toml" "$CRATE/Cargo.lock" "$WORK/" 2>/dev/null

  if [ -n "$sed_expr" ]; then
    sed -i "$sed_expr" "$WORK/src/lib.rs" || { echo "  [$name] SED FAILED"; SURVIVORS+=("$name (sed failed)"); return; }
  fi
  if [ -n "$cargo_expr" ]; then
    sed -i "$cargo_expr" "$WORK/Cargo.toml" || { echo "  [$name] CARGO SED FAILED"; SURVIVORS+=("$name (sed failed)"); return; }
  fi
  if diff -q "$CRATE/src/lib.rs" "$WORK/src/lib.rs" >/dev/null \
     && diff -q "$CRATE/Cargo.toml" "$WORK/Cargo.toml" >/dev/null; then
    echo "  [$name] MUTATION DID NOT APPLY"; SURVIVORS+=("$name (no-op)"); return
  fi

  local killed_by="" compiled=0
  for profile in debug release; do
    local flag="" so
    if [ "$profile" = release ]; then flag="--release"; fi
    # shellcheck disable=SC2086
    ( cd "$WORK" && timeout 300 cargo build $flag --offline ) >"$WORK/build.$profile.log" 2>&1 || continue
    so="$WORK/target/$profile/libmatrixsum_lib.so"
    [ -f "$so" ] || continue
    compiled=1
    local out
    out=$(C_SO="$C_SO" RUST_SO="$so" timeout 600 cargo test --offline 2>&1)
    if echo "$out" | grep -qE "test result: FAILED|error: test failed"; then
      local failed
      failed=$(echo "$out" | grep -oE "^test [a-z0-9_]+ \.\.\. FAILED" | sed 's/^test //; s/ \.\.\. FAILED//' | tr '\n' ' ')
      killed_by="$killed_by[$profile] $failed"
    fi
  done

  if [ "$compiled" -eq 0 ]; then
    echo "  [$name] mutant did not compile (skipped)"; return
  fi

  if [ -n "$killed_by" ]; then
    echo "  [$name] KILLED  <- $killed_by"
    if is_equivalent "$name"; then
      echo "  [$name] *** UNEXPECTEDLY KILLED — this mutant is semantics-preserving,"
      echo "         so a test that fails on it is asserting something wrong ***"
      SURVIVORS+=("$name (equivalent mutant was killed)")
    fi
  elif is_equivalent "$name"; then
    echo "  [$name] survived — EXPECTED (provably semantics-preserving)"
  else
    echo "  [$name] *** SURVIVED *** (blind spot!)"
    SURVIVORS+=("$name")
  fi
}

echo "=== mutation check: the suite must kill every one of these ==="
run_mutation "hex_base 0xFF->0xFE"        's/let hex_base: c_int = 0xFF;/let hex_base: c_int = 0xFE;/'
run_mutation "hex_multiplier 0x10->0x08"  's/let hex_multiplier: c_int = 0x10;/let hex_multiplier: c_int = 0x08;/'
run_mutation "matrix 0xD4->0xD5"          's/0xA1, 0xB2, 0xC3, 0xD4/0xA1, 0xB2, 0xC3, 0xD5/'
run_mutation "FLAG_DELETE bit moved"      's/const FLAG_DELETE: c_int = 0b0000_1000;/const FLAG_DELETE: c_int = 0b0001_0000;/'
run_mutation "add_element >= becomes >"   's/rd(\&raw const (\*arr).size) >= rd(\&raw const (\*arr).capacity)/rd(\&raw const (*arr).size) > rd(\&raw const (*arr).capacity)/'
run_mutation "init_array saturating mul"  's/initial_capacity.wrapping_mul(SIZEOF_INT)/initial_capacity.saturating_mul(SIZEOF_INT)/'
run_mutation "expand saturating cap mul"  's/rd(\&raw const (\*arr).capacity).wrapping_mul(2)/rd(\&raw const (*arr).capacity).saturating_mul(2)/'
run_mutation "expand saturating byte mul" 's/new_capacity.wrapping_mul(SIZEOF_INT)/new_capacity.saturating_mul(SIZEOF_INT)/'
run_mutation "mask 0xFFF->0xFFFF"         's/matrix_sum & 0xFFF)/matrix_sum \& 0xFFFF)/'
run_mutation "expand *2 becomes +2"       's/rd(\&raw const (\*arr).capacity).wrapping_mul(2)/rd(\&raw const (*arr).capacity).wrapping_add(2)/'
run_mutation "process_flags no !! on read" 's/let read_enabled = (has_read != 0) as c_int;/let read_enabled = has_read;/'
run_mutation "matrix loop 3->2 rows"      's/while i < 3 {/while i < 2 {/'
run_mutation "matrix loop 4->3 cols"      's/while j < 4 {/while j < 3 {/'
run_mutation "add_element size += 2"      's/wr(\&raw mut (\*arr).size, idx.wrapping_add(1));/wr(\&raw mut (*arr).size, idx.wrapping_add(2));/'
run_mutation "expand_array null->1"       's/^        return 0;$/        return 1;/'
run_mutation "matrixsum sum += 1"         's/sum = sum.wrapping_add(rd(rd(\&raw const (\*arr).data).wrapping_add(i)));/sum = sum.wrapping_add(rd(rd(\&raw const (*arr).data).wrapping_add(i))).wrapping_add(1);/'
run_mutation "checksum sum starts at 1"   's/let mut sum: c_int = 0;/let mut sum: c_int = 1;/'
# --- initializer of the exported `matrix` global (killed only by phase_b_pristine) ---
run_mutation "matrix[0][0] 0x01->0x02"    's/\[0x01, 0x02, 0x03, 0x04\],/[0x02, 0x02, 0x03, 0x04],/'
run_mutation "matrix[1][2] 0x30->0x31"    's/\[0x10, 0x20, 0x30, 0x40\],/[0x10, 0x20, 0x31, 0x40],/'
run_mutation "matrix rows swapped"        's/\[0x01, 0x02, 0x03, 0x04\],/[0xA1, 0xB2, 0xC3, 0xD4],/'
# --- `!!` truthiness: only the FLAG_READ bit is value-preserving without it, so
#     the WRITE/EXECUTE/DELETE variants MUST be killed ---
run_mutation "process_flags no !! (write)"   's/let write_enabled = (has_write != 0) as c_int;/let write_enabled = has_write;/'
run_mutation "process_flags no !! (exec)"    's/let execute_enabled = (has_execute != 0) as c_int;/let execute_enabled = has_execute;/'
run_mutation "process_flags no !! (delete)"  's/let delete_enabled = (has_delete != 0) as c_int;/let delete_enabled = has_delete;/'
# --- matrixsum's `!!paramN` truthiness ---
run_mutation "matrixsum valid1 no !!"     's/let valid1 = (check1 != 0) as c_int;/let valid1 = check1;/'
# --- silent resource leaks: no return value changes, only the allocator notices
#     (killed by phase_c_leak.rs) ---
run_mutation "free_array skips data free"   's/free(rd(\&raw const (\*arr).data) as \*mut c_void);/;/'
run_mutation "free_array skips struct free" 's/free(arr as \*mut c_void);/;/'
run_mutation "matrixsum skips free_array"   's/^    free_array(arr);$/    let _ = arr;/'
run_mutation "init_array size=1"          's/wr(\&raw mut (\*arr).size, 0);/wr(\&raw mut (*arr).size, 1);/'
run_mutation "init_array capacity+1"      's/wr(\&raw mut (\*arr).capacity, initial_capacity);/wr(\&raw mut (*arr).capacity, initial_capacity.wrapping_add(1));/'
run_mutation "valid3 uses param4"         's/let check3 = param3;/let check3 = param4;/'

# --- side-effect ORDER inside add_element (killed by phase_c_crash z5) ---
run_mutation "add_element size committed last" \
  's|^    wr(\&raw mut (\*arr).size, idx.wrapping_add(1));$|    wr(data.wrapping_add(idx), value); wr(\&raw mut (*arr).size, idx.wrapping_add(1)); if true { return 1; }|'

# --- raw unchecked accesses replaced by checked ones: aborts where C does not
#     (killed by phase_c_crash z2/z3/z4 in the debug profile) ---
run_mutation "add_element direct deref store" \
  's|    wr(data.wrapping_add(idx), value);|    *data.add(idx) = value;|' \
  's|^debug-assertions = false$|debug-assertions = true|'
run_mutation "struct fields read via direct deref" \
  's|rd(\&raw const (\*arr).size) >= rd(\&raw const (\*arr).capacity)|(*arr).size >= (*arr).capacity|' \
  's|^debug-assertions = false$|debug-assertions = true|'

# --- inlining: helper chain folded into matrixsum, struct allocation elided
#     (killed by phase_d_alloc_traffic t1/t2/t3) ---
run_mutation "no inline(never) on helpers" 's/^#\[inline(never)\]$//'

rm -rf "$WORK"
echo
if [ "${#SURVIVORS[@]}" -eq 0 ]; then
  echo "########## ALL MUTANTS KILLED — the suite has no blind spot on these ##########"
  exit 0
else
  echo "########## SURVIVING MUTANTS (suite blind spots): ${SURVIVORS[*]} ##########"
  exit 1
fi
