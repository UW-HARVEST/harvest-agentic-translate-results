#!/usr/bin/env bash
# Mutation test for the differential suite.
#
# Passing differential tests only mean something if they would FAIL on a wrong
# translation. This script injects deliberate bugs into a *copy* of
# src/lib.rs, builds each mutant as its own cdylib, points the suite at it via
# RUST_SO=, and asserts the suite rejects it.
#
# `src/lib.rs` is never modified.
#
# A mutant counts as KILLED if `cargo test` exits non-zero for ANY reason
# (assertion failures, or the harness process dying from a SIGSEGV caused by
# the mutant's out-of-bounds accesses).
set -u

cd "$(dirname "$0")" || exit 1
WORK="${TMPDIR:-/tmp}/mutants"
mkdir -p "$WORK"

# Comment-aware, ambiguity-checking mutator: refuses to patch doc comments and
# refuses to patch an ambiguous pattern (both would silently produce a no-op
# mutant that looks "survived").
cat > "$WORK/mutate.py" <<'PY'
import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
lines = open(p).read().split('\n')
hits = [i for i, l in enumerate(lines)
        if old in l and not l.lstrip().startswith('//')]
if not hits:
    sys.exit(f"MUTATOR ERROR: pattern absent from all CODE lines: {old!r}")
if len(hits) > 1:
    sys.exit(f"MUTATOR ERROR: pattern matches {len(hits)} code lines: {old!r}")
lines[hits[0]] = lines[hits[0]].replace(old, new)
open(p, 'w').write('\n'.join(lines))
PY

pass_n=0; fail_n=0
declare -a FAILURES=()

# run_mut <name> <old> <new> <KILLED|EQUIVALENT> [test-target]
run_mut() {
  local name="$1" old="$2" new="$3" expect="$4" target="${5:-phase_b_valid}"
  cp src/lib.rs "$WORK/$name.rs"
  if ! python3 "$WORK/mutate.py" "$WORK/$name.rs" "$old" "$new" 2>"$WORK/$name.mut"; then
    printf '  %-26s %-26s %s\n' "$name" "HARNESS ERROR" "$(cat "$WORK/$name.mut")"
    fail_n=$((fail_n+1)); FAILURES+=("$name: mutator error"); return
  fi
  if ! rustc --crate-type cdylib --edition 2024 -C debug-assertions=off \
        -o "$WORK/$name.so" "$WORK/$name.rs" 2>"$WORK/$name.build"; then
    printf '  %-26s %-26s %s\n' "$name" "BUILD FAILED" "$(tail -1 "$WORK/$name.build")"
    fail_n=$((fail_n+1)); FAILURES+=("$name: build failed"); return
  fi

  RUST_SO="$WORK/$name.so" timeout 600 cargo test --test "$target" \
      >"$WORK/$name.test" 2>&1
  local rc=$?
  local verdict
  if [ $rc -ne 0 ]; then verdict=KILLED; else verdict=SURVIVED; fi

  local detail
  detail=$(grep -m1 "test result" "$WORK/$name.test" || echo "process died (rc=$rc)")

  local ok
  if [ "$expect" = KILLED ]   && [ "$verdict" = KILLED ];   then ok=1
  elif [ "$expect" = EQUIVALENT ] && [ "$verdict" = SURVIVED ]; then ok=1
  else ok=0; fi

  if [ $ok -eq 1 ]; then
    printf '  \033[32m✓\033[0m %-24s %-10s (expected %-10s) %s\n' "$name" "$verdict" "$expect" "$detail"
    pass_n=$((pass_n+1))
  else
    printf '  \033[31m✗\033[0m %-24s %-10s (expected %-10s) %s\n' "$name" "$verdict" "$expect" "$detail"
    fail_n=$((fail_n+1)); FAILURES+=("$name: got $verdict, expected $expect")
  fi
}

echo "Building baseline artifacts..."
cargo build -q 2>/dev/null || cargo build
[ -f c_src/build/libtranslated_rust.so ] || {
  echo "C .so missing; build it first (see SYMBOLS.md)"; exit 1; }

echo
echo "=== Mutants that MUST be killed by the valid-path suite (Phase B) ==="
run_mut cmp_le_to_lt          'a.sort_bits <= b.sort_bits' 'a.sort_bits < b.sort_bits'                       KILLED
run_mut cmp_swap_operands     'a.sort_bits <= b.sort_bits' 'b.sort_bits <= a.sort_bits'                      KILLED
run_mut cmp_le_to_ge          'a.sort_bits <= b.sort_bits' 'a.sort_bits >= b.sort_bits'                      KILLED
run_mut revive_dead_branch    'if a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id' \
                              'if a.texture_id <= b.texture_id'                                              KILLED
run_mut cmp_returns_zero      'if a.sort_bits <= b.sort_bits {' 'if false && a.sort_bits <= b.sort_bits {'    KILLED
run_mut no_padding_copy \
  'unsafe { core::ptr::copy_nonoverlapping(a.offset(i as isize), b.offset(k as isize), 1) };' \
  'unsafe { let s = &*a.offset(i as isize); let d = &mut *b.offset(k as isize); d.texture_id = s.texture_id; d.sort_bits = s.sort_bits; };' \
  KILLED
run_mut swap_final_buffers    'spritebatch_internal_merge_sort_recurse(b, 0, size, a)' \
                              'spritebatch_internal_merge_sort_recurse(a, 0, size, b)'                       KILLED
run_mut swap_recurse_lhs      'spritebatch_internal_merge_sort_recurse(a, lo, split, b);' \
                              'spritebatch_internal_merge_sort_recurse(b, lo, split, a);'                    KILLED
run_mut swap_iteration_dir    'spritebatch_internal_merge_sort_iteration(b, lo, split, hi, a);' \
                              'spritebatch_internal_merge_sort_iteration(a, lo, split, hi, b);'              KILLED
run_mut base_case_off_by_one  'if hi.wrapping_sub(lo) <= 1 {' 'if hi.wrapping_sub(lo) <= 2 {'                 KILLED
run_mut iter_upper_bound      'while k < hi {'   'while k < hi.wrapping_sub(1) {'                             KILLED
run_mut skip_small_memcpy     'if bytes != 0 {'  'if bytes > 16 {'                                            KILLED
run_mut j_step_two            'j = j.wrapping_add(1);' 'j = j.wrapping_add(2);'                               KILLED
run_mut i_step_two            'i = i.wrapping_add(1);' 'i = i.wrapping_add(2);'                               KILLED
run_mut drop_right_run_check  '&& (j >= hi'  '&& (false'                                                      KILLED
run_mut split_bound_check     'let take_i = i < split' 'let take_i = i <= split'                              KILLED

echo
echo "=== Mutants on the ERROR/boundary surface (Phase C) ==="
run_mut memcpy_size_unsigned  'size_of::<spritebatch_sprite_t>().wrapping_mul(size as usize)' \
                              'size_of::<spritebatch_sprite_t>().wrapping_mul(size as u32 as usize)' \
                              KILLED phase_c_errors
run_mut memcpy_saturating     'if bytes != 0 {' 'if bytes != 0 && size > 0 {'                                 KILLED phase_c_errors

echo
echo "=== Semantically EQUIVALENT mutants (MUST survive — proves no over-fitting) ==="
# (lo+hi)/2 == lo+(hi-lo)/2 for every non-negative lo<=hi reachable from
# merge_sort, so this rewrite is not observable.
run_mut split_no_overflow     'lo.wrapping_add(hi) / 2' '(lo + (hi - lo) / 2)'                                EQUIVALENT
# lib.c:9 is unreachable (lib.c:7 already returned 1 whenever sort_bits are
# equal), so changing what it returns cannot be observed. This is the mutant
# that PROVES the documented dead-code quirk really is dead.
run_mut dead_branch_ret_zero  'if a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id {' \
                              'if a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id && false {' \
                              EQUIVALENT

echo
echo "=================================================================="
printf 'mutation score: %d/%d expectations met\n' "$pass_n" "$((pass_n+fail_n))"
if [ ${#FAILURES[@]} -gt 0 ]; then
  echo "UNEXPECTED RESULTS:"
  for f in "${FAILURES[@]}"; do echo "  - $f"; done
  exit 1
fi
echo "ALL MUTATION EXPECTATIONS MET"
