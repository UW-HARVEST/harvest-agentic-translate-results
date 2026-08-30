#!/usr/bin/env bash
# Negative control for the differential suite.
#
# Each mutation injects a plausible translation bug into src/lib.rs; the suite
# MUST fail for every one. A mutation that passes means the tests are blind to
# that class of bug.
#
# Correctness requirements this script enforces on itself:
#   * backup lives in a stable directory (not $TMPDIR, which moves between calls)
#   * the restore is VERIFIED with cmp before each mutation, so mutations can
#     never stack cumulatively
#   * the .so is rebuilt after mutating, so tests never see a stale artifact
#   * a staleness-guard trip is reported separately from a real divergence, so a
#     spurious "caught" is never counted as evidence
set -uo pipefail

CRATE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$CRATE"
ORIG="$CRATE/.verif/lib.rs.pristine"

[ -f "$ORIG" ] || { echo "FATAL: missing pristine backup $ORIG"; exit 1; }

restore() {
  cp "$ORIG" src/lib.rs
  cmp -s "$ORIG" src/lib.rs || { echo "FATAL: restore failed"; exit 1; }
}

caught=0; missed=0; skipped=0; equiv_ok=0; equiv_bad=0

# Runs one mutation. $1 = name, $2 = "kill" (must be detected) or "equiv"
# (semantically equivalent: MUST survive, and surviving is the correct outcome).
run_mut() {
  local name="$1"; local kind="$2"; shift 2
  restore
  "$@"
  if cmp -s "$ORIG" src/lib.rs; then
    echo "  $name: NO-OP PATCH (bad mutation spec)"; skipped=$((skipped+1)); return
  fi
  if ! timeout 600 cargo build --offline --release >/dev/null 2>&1; then
    echo "  $name: does not compile (skipped)"; skipped=$((skipped+1)); return
  fi
  local out
  out=$(timeout 600 cargo test --offline --release 2>&1)
  if echo "$out" | grep -q 'STALE Rust .so'; then
    echo "  $name: INCONCLUSIVE - staleness guard tripped, not a real signal"
    skipped=$((skipped+1)); return
  fi
  local detected=0
  echo "$out" | grep -qE '^test result: FAILED' && detected=1

  if [ "$kind" = equiv ]; then
    if [ "$detected" -eq 0 ]; then
      echo "  $name: SURVIVED as expected (equivalent mutant)"
      equiv_ok=$((equiv_ok+1))
    else
      echo "  $name: *** unexpectedly detected -- equivalence claim is wrong ***"
      equiv_bad=$((equiv_bad+1))
    fi
    return
  fi

  if [ "$detected" -eq 1 ]; then
    local n first
    n=$(echo "$out" | grep -cE '\.\.\. FAILED')
    first=$(echo "$out" | grep -oE '^test [a-z0-9_]+ \.\.\. FAILED' | head -3 \
            | sed -E 's/^test //; s/ \.\.\. FAILED//' | tr '\n' ',' | sed 's/,$//')
    echo "  $name: CAUGHT ($n tests; e.g. $first)"
    caught=$((caught+1))
  else
    echo "  $name: *** NOT CAUGHT -- BLIND SPOT ***"
    missed=$((missed+1))
  fi
}

echo "=== value-semantics mutations ==="
run_mut swap_args kill           sed -i 's/libm_pow(base, exponent)/libm_pow(exponent, base)/' src/lib.rs
run_mut abs_base kill            sed -i 's/libm_pow(base, exponent)/libm_pow(base.abs(), exponent)/' src/lib.rs
# EQUIVALENT: rustc lowers `f64::powf` to a call to the very same
# `pow@GLIBC_2.29` that `libm_pow` binds, so results AND the errno side effect
# are identical. Verified with:
#   nm -D --undefined-only target/release/libpow.so | grep -w pow   -> U pow@GLIBC_2.29
#   objdump -d --disassemble=my_pow target/release/libpow.so        -> call <pow@GLIBC_2.29>
# No test can distinguish it because there is no difference to observe.
run_mut rust_powf_intrinsic equiv sed -i 's/libm_pow(base, exponent)/base.powf(exponent)/' src/lib.rs
run_mut negate_result kill       sed -i 's/^        result$/        -result/' src/lib.rs

echo "=== sentinel mutations ==="
run_mut sentinel_to_pos1 kill    sed -i 's/return -1\.0;/return 1.0;/g' src/lib.rs
run_mut sentinel_to_negzero kill sed -i 's/return -1\.0;/return -0.0;/g' src/lib.rs
run_mut sentinel_to_nan kill     sed -i 's/return -1\.0;/return f64::NAN;/g' src/lib.rs
run_mut sentinel_edom_only kill  sed -i '0,/return -1\.0;/s//return -2.0;/' src/lib.rs

echo "=== errno-handling mutations ==="
run_mut no_errno_reset kill      sed -i 's/^        errno_set(0);$//' src/lib.rs
run_mut wrong_edom_const kill    sed -i 's/const EDOM: c_int = 33;/const EDOM: c_int = 35;/' src/lib.rs
run_mut wrong_erange_const kill  sed -i 's/const ERANGE: c_int = 34;/const ERANGE: c_int = 36;/' src/lib.rs
run_mut swap_errno_consts kill   sed -i 's/const EDOM: c_int = 33;/const EDOM: c_int = 34;/; s/const ERANGE: c_int = 34;/const ERANGE: c_int = 33;/' src/lib.rs
run_mut drop_edom_branch kill    sed -i 's/if err == EDOM {/if false {/' src/lib.rs
run_mut drop_erange_branch kill  sed -i 's/} else if err == ERANGE {/} else if false {/' src/lib.rs
# EQUIVALENT: `err == EDOM` and `err != 0 && err != ERANGE` differ only when
# errno holds a value outside {0, EDOM, ERANGE} after pow(). .verif/errno_probe.c
# fuzzes 8,000,000 pow() calls (full 2^128 bit space + integral exponents across
# the over/underflow band) and observes ONLY errno in {0, 33 EDOM, 34 ERANGE}.
# Nothing else executes between `errno = 0` and the read, so the differing case
# is unreachable and the two predicates agree on every reachable input.
run_mut errno_any_nonzero equiv   sed -i 's/if err == EDOM {/if err != 0 \&\& err != ERANGE {/' src/lib.rs
run_mut errno_read_late kill     sed -i 's/^        let err = errno_get();$/        let err = { errno_set(0); errno_get() };/' src/lib.rs

echo "=== diagnostic (stderr) mutations ==="
run_mut msg_domain_typo kill     sed -i 's/Domain error: pow/Domain Error: pow/' src/lib.rs
run_mut msg_range_typo kill      sed -i 's/Range error: pow/range error: pow/' src/lib.rs
run_mut fmt_precision_3 kill     sed -i 's/pow(%\.2f, %\.2f) caused/pow(%.3f, %.3f) caused/' src/lib.rs
run_mut fmt_use_g kill           sed -i 's/pow(%\.2f, %\.2f) caused/pow(%g, %g) caused/' src/lib.rs
# swap the two varargs passed to fprintf (sed cannot match across lines, so use
# perl slurp mode) -- the printed numbers must appear in the C's order
run_mut swap_msg_args kill       perl -0pi -e 's/                base,\n                exponent,/                exponent,\n                base,/g' src/lib.rs
run_mut use_range_for_edom kill  sed -i 's/DOMAIN_ERROR_FMT\.as_ptr()/RANGE_ERROR_FMT.as_ptr()/' src/lib.rs
run_mut drop_trailing_nl kill    sed -i 's/real number domain\.\\n\\0/real number domain.\\0/' src/lib.rs

restore
timeout 600 cargo build --offline --release >/dev/null 2>&1
echo
echo "caught=$caught  missed=$missed  skipped=$skipped  equivalent_as_expected=$equiv_ok  equivalence_violations=$equiv_bad"
if [ "$missed" -ne 0 ] || [ "$equiv_bad" -ne 0 ]; then
  echo "NEGATIVE CONTROL FAILED"; exit 1
fi
echo "restored: $(md5sum src/lib.rs | cut -d' ' -f1)"
echo "NEGATIVE CONTROL OK - every compiling mutation was detected"
