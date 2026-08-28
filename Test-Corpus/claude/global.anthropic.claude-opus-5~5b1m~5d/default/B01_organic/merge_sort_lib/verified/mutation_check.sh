#!/usr/bin/env bash
# Anti-vacuity check: deliberately inject known-wrong behaviour into the Rust
# translation and confirm the differential suite CATCHES each mutant. A test
# suite that passes no matter what the Rust does would prove nothing.
#
# Every mutant is tried under BOTH profiles, because an optimising build can
# erase some kinds of wrongness (notably infinite recursion).
#
# src/lib.rs is restored and its SHA-256 re-verified before exit.
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$CRATE_DIR" || exit 1
ROOT="$(dirname "$CRATE_DIR")"
SRC="src/lib.rs"
# Keep the backup inside the crate dir: $TMPDIR is not guaranteed to persist
# between invocations in some sandboxes, and losing the backup mid-run would
# leave a mutated source behind.
BAK="$CRATE_DIR/.lib.rs.mutation-backup"
cp "$SRC" "$BAK" || { echo "cannot back up $SRC"; exit 1; }
ORIG_SUM="$(sha256sum < "$SRC" | cut -d' ' -f1)"
echo "backup: $BAK  sha256: $ORIG_SUM"

restore() {
  cp "$BAK" "$SRC"
  local now; now="$(sha256sum < "$SRC" | cut -d' ' -f1)"
  if [ "$now" != "$ORIG_SUM" ]; then
    echo "!! RESTORE FAILED — $SRC does not match the original checksum!"; exit 2
  fi
}
trap 'restore; echo "(restored $SRC)"' EXIT

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -f "$C_SO" ] || { echo "build the C .so first"; exit 1; }

SURVIVORS=0
mutate() {
  local name="$1" profile="$2"; shift 2
  restore
  "$@" || { echo "  $name/$profile: sed failed"; return; }
  if ! grep -q . "$SRC"; then echo "  $name/$profile: empty source"; return; fi

  # Only the .so under test varies; the test harness itself always builds in the
  # dev profile (profile.release sets panic="abort", and a release harness that
  # SIGSEGVs loses its buffered stdout).
  local so flag=""
  if [ "$profile" = release ]; then flag="--release"; so="target/release/libmerge_sort_lib.so";
  else so="target/debug/libmerge_sort_lib.so"; fi

  if ! timeout 600 cargo build $flag >/dev/null 2>&1; then
    echo "  $name/$profile: did not compile (mutant skipped)"; return
  fi
  local out rc
  out=$(C_LIB_PATH="$C_SO" RUST_LIB_PATH="$PWD/$so" TRIALS=4 FUZZ_ITERS=800 \
        timeout 300 cargo test --tests 2>&1)
  rc=$?
  # Exit status is the authoritative signal: a mutant that makes the harness
  # crash (SIGSEGV / stack overflow) may lose its buffered stdout entirely, so
  # grepping the text alone can miss a mutant that was in fact detected.
  if [ "$rc" -ne 0 ] || echo "$out" | grep -qE 'FAILED|panicked|stack overflow'; then
    echo "  $name/$profile: CAUGHT (exit=$rc)"
  else
    echo "  $name/$profile: *** SURVIVED ***"
    SURVIVORS=$((SURVIVORS+1))
  fi
}

declare -a NAMES=(
  cmp_le_to_lt
  cmp_make_dead_branch_live
  drop_padding_word
  drop_first_word
  split_round_up
  swap_recurse_buffers
  guard_le1_to_le0
  guard_drop
  memcpy_skip_when_negative
  memcpy_unsigned_size
  iteration_swap_branches
)
apply() { # $1 = mutant name
  case "$1" in
    cmp_le_to_lt)            sed -i 's/if (\*a).sort_bits <= (\*b).sort_bits {/if (*a).sort_bits < (*b).sort_bits {/' "$SRC" ;;
    cmp_make_dead_branch_live) sed -i 's/if (\*a).sort_bits == (\*b).sort_bits \&\& (\*a).texture_id <= (\*b).texture_id {/if (*a).texture_id <= (*b).texture_id {/' "$SRC" ;;
    drop_padding_word)       sed -i 's/    (dst as \*mut MaybeUninit<u64>).add(1).write(w1);/    let _ = w1;/' "$SRC" ;;
    drop_first_word)         sed -i 's/    (dst as \*mut MaybeUninit<u64>).write(w0);/    let _ = w0;/' "$SRC" ;;
    split_round_up)          sed -i 's|let split: c_int = lo.wrapping_add(hi) / 2;|let split: c_int = lo.wrapping_add(hi).wrapping_add(1) / 2;|' "$SRC" ;;
    swap_recurse_buffers)    sed -i 's/spritebatch_internal_merge_sort_recurse(a, split, hi, b);/spritebatch_internal_merge_sort_recurse(b, split, hi, a);/' "$SRC" ;;
    guard_le1_to_le0)        sed -i 's/if hi.wrapping_sub(lo) <= 1 {/if hi.wrapping_sub(lo) <= 0 {/' "$SRC" ;;
    guard_drop)              sed -i 's/if hi.wrapping_sub(lo) <= 1 {/if false {/' "$SRC" ;;
    memcpy_skip_when_negative) sed -i 's/    memcpy(b as \*mut c_void, a as \*const c_void, bytes);/    if size > 0 { memcpy(b as *mut c_void, a as *const c_void, bytes); }/' "$SRC" ;;
    memcpy_unsigned_size)    sed -i 's/.wrapping_mul(size as isize as usize);/.wrapping_mul(size as u32 as usize);/' "$SRC" ;;
    iteration_swap_branches) sed -i 's/            sprite_assign(b.offset(k as isize), a.offset(i as isize));\n            i = i.wrapping_add(1);/X/' "$SRC"
                             python3 - "$SRC" <<'PY'
import sys,re
p=sys.argv[1]; s=open(p).read()
# swap which run the merge takes when the predicate holds
s=s.replace("if i < split\n            && (j >= hi","if !(i < split)\n            && (j >= hi",1)
open(p,"w").write(s)
PY
                             ;;
  esac
}

echo
for n in "${NAMES[@]}"; do
  echo "mutant: $n"
  for prof in release debug; do mutate "$n" "$prof" apply "$n"; done
done

restore
trap - EXIT          # restore already ran; don't re-run it after the backup goes
rm -f "$BAK"
echo
if [ "$SURVIVORS" -eq 0 ]; then
  echo "RESULT: every mutant was caught in at least one profile."
else
  echo "RESULT: $SURVIVORS mutant/profile pair(s) survived (see above)."
fi
