#!/usr/bin/env bash
# Harness validation: deliberately inject bugs into the Rust translation and
# confirm the differential suite CATCHES each one. A suite that passes a mutated
# implementation is not actually testing anything.
#
# This exists because an earlier version of the harness silently loaded a STALE
# .so (integration tests do not link the cdylib, so `cargo test` did not rebuild
# it) and every mutation escaped. `crate-type` now includes "rlib" so the lib is
# rebuilt, and tests/common/mod.rs additionally fails on a stale artifact.
#
# Restores src/lib.rs on any exit path, including signals.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT/translation" || exit 1

SRC=src/lib.rs
BAK="$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap 'restore; exit 130' INT TERM HUP
trap restore EXIT

CAUGHT=0
ESCAPED=0
EQUIV=0

# Per-mutation timeout: some mutations (e.g. breaking the loop-exit condition)
# make the function loop forever, so each run must be bounded.
PER_RUN_TIMEOUT=180

# run_mutation <name> <profile-flag> <expected: catch|equiv> <sed-expr>
run_mutation() {
  local name="$1" relflag="$2" expect="$3" expr="$4"
  restore
  if ! sed -i "$expr" "$SRC"; then
    echo "  [skip]    $name (sed failed)"; return
  fi
  if cmp -s "$SRC" "$BAK"; then
    echo "  [skip]    $name (mutation did not change the source)"; return
  fi

  local out rc verdict
  out="$(timeout "$PER_RUN_TIMEOUT" cargo test $relflag 2>&1)"
  rc=$?

  if [ "$rc" -eq 124 ]; then
    # Hang: the C library always terminates, so a hang IS a detected divergence.
    verdict=caught
    echo "  [CAUGHT]  $name  (hang / infinite loop, killed after ${PER_RUN_TIMEOUT}s)"
  elif echo "$out" | grep -qE 'test result: FAILED|panicked|DIVERGENCE|error\[|error: could not compile|SIGABRT|abort'; then
    verdict=caught
    echo "  [CAUGHT]  $name"
  else
    verdict=escaped
  fi

  if [ "$verdict" = caught ]; then
    CAUGHT=$((CAUGHT + 1))
    if [ "$expect" = equiv ]; then
      echo "            NOTE: expected this to be an equivalent mutant, but it was caught."
    fi
  else
    if [ "$expect" = equiv ]; then
      echo "  [equiv]   $name  (provably semantics-preserving - correctly not flagged)"
      EQUIV=$((EQUIV + 1))
    else
      echo "  [ESCAPED] $name  <-- the suite failed to detect this bug!"
      ESCAPED=$((ESCAPED + 1))
    fi
  fi
}

echo "=== Mutation testing the differential suite (release) ==="

run_mutation "M2  'e & 3' -> 'e % 4' (wrong for negative e)" --release catch \
  's|G_EXPFRAC\[(e & 3) as usize\]|G_EXPFRAC[(e % 4) as usize]|'

run_mutation "M4  regroup 'y * (frac*scale)' -> '(y*frac) * scale'" --release catch \
  's|y \*= frac \* (scale as f32);|y = (y * frac) * (scale as f32);|'

run_mutation "M6  clamp '120 > exp_q2' -> '120 < exp_q2'" --release catch \
  's|if (30 \* 4) > exp_q2|if (30 * 4) < exp_q2|'

run_mutation "M7  perturb G_EXPFRAC[1] by 1 ULP" --release catch \
  's|7.83145814e-10f32|7.8314587e-10f32|'

run_mutation "M8  clamp constant 120 -> 124" --release catch \
  's|30 \* 4|31 * 4|g'

run_mutation "M9  loop-exit '<= 0' -> '< 0' (infinite loop at exp_q2==0)" --release catch \
  's|if exp_q2 <= 0 {|if exp_q2 < 0 {|'

run_mutation "M10 '(1i32 << 30)' -> '(1i32 << 29)'" --release catch \
  's|(1i32 << 30)|(1i32 << 29)|'

run_mutation "M11 drop the wrapping_sub -> plain sub" --release catch \
  's|exp_q2 = exp_q2.wrapping_sub(e);|exp_q2 = exp_q2 - e + 1;|'

run_mutation "M12 G_EXPFRAC[2] takes G_EXPFRAC[3]'s value (residue mix-up)" --release catch \
  's|6.58544508e-10f32|5.53767716e-10f32|'

run_mutation "M13 'e >> 2' -> 'e >> 3' (wrong quarter-step divisor)" --release catch \
  's|((e >> 2) \& 31)|((e >> 3) \& 31)|'

# M14 is profile-sensitive for the same reason as M1 (see below): widening the
# mask lets the shift count reach 32..63, which x86 `sar` masks back to 5 bits
# in release but which trips Rust's debug-only shift-overflow check.


# --- Provably equivalent mutants -------------------------------------------
# These change the source but not the observable behaviour, so the suite is
# CORRECT to let them pass. Documented so a reader does not mistake them for
# blind spots:
#
#  M3: for any i32, `e & 3` == `e.rem_euclid(4)` (power-of-two modulus, and
#      rem_euclid is non-negative), so the index is bit-identical.
#  M5: only the low 5 bits of the shift count are used, and arithmetic vs
#      logical right-shift agree on bits 2..6 of `e` (they differ only in the
#      top two bit positions), so `(e>>2)&31 == ((e as u32)>>2)&31`.
run_mutation "M3  'e & 3' -> 'e.rem_euclid(4)'" --release equiv \
  's|G_EXPFRAC\[(e & 3) as usize\]|G_EXPFRAC[e.rem_euclid(4) as usize]|'

run_mutation "M5  'e >> 2' -> logical shift on u32" --release equiv \
  's|((e >> 2) \& 31)|(((e as u32) >> 2) \& 31)|'

# --- Profile-sensitive mutation --------------------------------------------
# M1 removes the `& 31` mask. In RELEASE, Rust lowers `i32 >> negative` to
# `sar %cl` (overflow checks off), which the CPU masks anyway -> equivalent.
# In DEBUG, overflow checks are ON and it panics with "attempt to shift right
# with overflow", so the mask is load-bearing and the debug build catches it.
echo
echo "=== Profile-sensitive mutation: '& 31' shift mask ==="
run_mutation "M1  remove '& 31' shift mask [release]" --release equiv \
  's|>> ((e >> 2) & 31)|>> (e >> 2)|'
run_mutation "M1  remove '& 31' shift mask [debug]" "" catch \
  's|>> ((e >> 2) & 31)|>> (e >> 2)|'
run_mutation "M14 mask '& 31' -> '& 63' [release]" --release equiv \
  's|((e >> 2) \& 31)|((e >> 2) \& 63)|'
run_mutation "M14 mask '& 31' -> '& 63' [debug]" "" catch \
  's|((e >> 2) \& 31)|((e >> 2) \& 63)|'

restore
echo
echo "=== Mutation summary: CAUGHT=$CAUGHT EQUIVALENT=$EQUIV ESCAPED=$ESCAPED ==="
if [ "$ESCAPED" -ne 0 ]; then
  echo "WARNING: $ESCAPED real bug(s) escaped detection - the suite has a blind spot."
  exit 1
fi
echo "Every non-equivalent injected bug was detected by the differential suite."
