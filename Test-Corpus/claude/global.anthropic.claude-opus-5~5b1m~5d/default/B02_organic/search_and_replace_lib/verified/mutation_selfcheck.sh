#!/usr/bin/env bash
# Harness self-check: prove the differential tests actually DETECT divergence.
#
# Each mutation deliberately breaks the Rust translation in one place, builds it
# as a standalone cdylib, and re-runs the whole differential suite with
# RUST_DRIVER_SO pointing at the mutant.
#
# Expectation per mutation:
#   KILL  -> the mutation changes observable behaviour, so the suite MUST fail.
#            A surviving KILL mutation is a blind spot in the tests.
#   EQUIV -> the mutation is provably semantically equivalent to the C (the
#            differing branch is unreachable, or the differing branch is a
#            0-byte no-op). Such a mutation MUST survive; if it is "killed",
#            the equivalence proof below is wrong.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
WORK="${TMPDIR:-/tmp}/sar-mutants"
rm -rf "$WORK"; mkdir -p "$WORK/src"
sed '/^\[dev-dependencies\]$/,/^$/d' "$HERE/Cargo.toml" > "$WORK/Cargo.toml"

rc=0
n=0
# Optional: MUT_FROM=<n> to resume from the n-th mutation (1-based).
MUT_FROM="${MUT_FROM:-1}"
run_mutation() { # $1 = KILL|EQUIV, $2 = name, $3.. = sed expressions
  local expect="$1" name="$2"; shift 2
  n=$((n + 1))
  [ "$n" -lt "$MUT_FROM" ] && return
  cp "$HERE/src/lib.rs" "$WORK/src/lib.rs"
  for e in "$@"; do sed -i "$e" "$WORK/src/lib.rs"; done
  if cmp -s "$HERE/src/lib.rs" "$WORK/src/lib.rs"; then
    echo "[$expect] '$name': sed matched nothing -- BAD MUTATION SPEC"; rc=1; return
  fi
  if ! ( cd "$WORK" && cargo build --release --offline >/dev/null 2>&1 ); then
    echo "[$expect] '$name': mutant does not compile -- BAD MUTATION SPEC"; rc=1; return
  fi
  local out failed trc
  out=$(cd "$HERE" && RUST_DRIVER_SO="$WORK/target/release/libdriver.so" \
        timeout 120 cargo test --release --offline --no-fail-fast -- --test-threads=1 2>&1)
  trc=$?
  failed=$(printf '%s\n' "$out" | grep -c '^test .* FAILED$')
  # A mutant that makes the library loop forever also diverges from the C: the
  # suite never finishes. Count the timeout as a detection.
  if [ $trc -eq 124 ]; then
    failed=$((failed + 1))
    names_extra="<suite timed out: mutant does not terminate>"
  else
    names_extra=""
  fi
  local names
  names="$(printf '%s\n' "$out" | grep '^test .* FAILED$' | sed 's/^test //;s/ \.\.\. FAILED//' | tr '\n' ' ')$names_extra"
  if [ "$expect" = KILL ]; then
    if [ "$failed" -gt 0 ]; then
      echo "[KILL ] '$name': KILLED by $failed test(s): $names"
    else
      echo "[KILL ] '$name': *** SURVIVED -- BLIND SPOT ***"; rc=1
    fi
  else
    if [ "$failed" -eq 0 ]; then
      echo "[EQUIV] '$name': survived, as proven equivalent"
    else
      echo "[EQUIV] '$name': *** KILLED by $failed test(s) -- equivalence proof is WRONG ***: $names"; rc=1
    fi
  fi
}

# ---------------------------------------------------------------- KILL --------
run_mutation KILL "leading-copy guard: inx_start > 0 => inx_start > 1" \
  's/if inx_start > 0 {/if inx_start > 1 {/'
run_mutation KILL "rescan start: orig+inx_start+search_len => orig+inx_start+1 (overlapping)" \
  's/orig\.add(inx_start + search_len), search/orig.add(inx_start + 1), search/'
run_mutation KILL "strstr: empty needle returns NULL instead of haystack" \
  's/^        return haystack;$/        return ptr::null();/'
run_mutation KILL "strncpy: drop the NUL padding (plain memcpy semantics)" \
  's/^    while i < n {$/    while false \&\& i < n {/'
run_mutation KILL "no-match path: empty string instead of strdup(orig)" \
  's/tmp = unsafe { strdup(orig) };/tmp = unsafe { strdup(c"".as_ptr()) };/'
run_mutation KILL "add a NULL argument check the C does not have" \
  's|^    let mut p: \*const c_char;$|    if orig.is_null() \|\| search.is_null() \|\| value.is_null() { return ptr::null_mut(); }\n    let mut p: *const c_char;|'
run_mutation KILL "off-by-one: total_bytes_allocated starts at 2" \
  's/let mut total_bytes_allocated: usize = 1;/let mut total_bytes_allocated: usize = 2;/'
run_mutation KILL "inx_start not advanced to inx_start2" \
  's/^            inx_start = inx_start2;$/            inx_start = inx_start;/'
run_mutation KILL "post-loop 'from' resync dropped (from = inx_start)" \
  's/^        from = inx_start + search_len;$/        from = inx_start;/'
run_mutation KILL "grow by search_len instead of value_len" \
  's/total_bytes_allocated += value_len;/total_bytes_allocated += search_len;/'
run_mutation KILL "tail copy length: orig_len - from => orig_len - from - 1" \
  's/orig\.add(from), orig_len - from)/orig.add(from), orig_len - from - 1)/'
run_mutation KILL "gap copy source: orig+from => orig+from+1" \
  's/c_strncpy(tmp\.add(tmp_offset), orig\.add(from), gap)/c_strncpy(tmp.add(tmp_offset), orig.add(from + 1), gap)/'
run_mutation KILL "final NUL written one byte early" \
  's/\*tmp\.add(total_bytes_allocated - 1) = 0/*tmp.add(total_bytes_allocated.saturating_sub(2)) = 0/'

# --------------------------------------------------------------- EQUIV --------
# Proof (1): at the check on lib.c:59, `from` is always (previous match index +
# search_len) and `p = strstr(orig + from, search)`, so inx_start2 >= from
# always holds. When inx_start2 == from the body computes gap == 0, so it grows
# the buffer by 0, reallocs to the same size and strncpy's 0 bytes: a no-op.
# Hence `>` and `>=` are indistinguishable.
run_mutation EQUIV "gap guard: inx_start2 > from => inx_start2 >= from (gap==0 is a no-op)" \
  's/if inx_start2 > from {/if inx_start2 >= from {/'

# Proof (2): `from == 0` at lib.c:78 requires inx_start == 0 && search_len == 0,
# i.e. an empty needle -- but an empty needle makes the `while (p != NULL)` loop
# non-terminating (ERRORS.md E9), so line 78 is never reached with from == 0.
# The `from > 0` term is therefore dead.
run_mutation EQUIV "tail guard: drop the dead 'from > 0' term" \
  's/if (from < orig_len) \&\& from > 0 {/if from < orig_len {/'

# Proof (3): `from <= orig_len` always holds (from = match index + search_len),
# and when from == orig_len the tail body grows by 0, reallocs to the same size
# and strncpy's 0 bytes: a no-op. So `<` and `<=` are indistinguishable.
run_mutation EQUIV "tail guard: from < orig_len => from <= orig_len (0-byte tail is a no-op)" \
  's/if (from < orig_len) \&\& from > 0 {/if (from <= orig_len) \&\& from > 0 {/'

echo "-----"
if [ $rc -eq 0 ]; then echo "SELF-CHECK OK: every KILL mutation was detected, every EQUIV mutation survived."
else echo "SELF-CHECK FAILED"; fi
exit $rc
