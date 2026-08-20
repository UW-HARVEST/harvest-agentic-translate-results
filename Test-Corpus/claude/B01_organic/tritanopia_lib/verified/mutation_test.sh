#!/bin/bash
# Mutation testing: proves the differential suite is NOT vacuous.
#
# Each mutation injects a realistic mistranslation into src/lib.rs, rebuilds,
# and runs the full suite. Mutations marked EXPECT=caught must be detected.
# Mutations marked EXPECT=equivalent are provably semantics-preserving at the
# library's observable (u8) output precision and MUST survive -- if one of
# those is ever "caught" the suite has become non-deterministic.
set -uo pipefail
cd "$(dirname "$0")"

WORK="${TMPDIR:-/tmp}/trit_mut.$$"
mkdir -p "$WORK" || exit 1
ORIG="$WORK/lib.rs.orig"
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; cargo build --release >/dev/null 2>&1; }
trap 'restore; rm -rf "$WORK"' EXIT

RC=0
run_mutation() {
  local desc="$1" expect="$2" sedexpr="$3"
  cp "$ORIG" src/lib.rs
  sed -i "$sedexpr" src/lib.rs
  if diff -q "$ORIG" src/lib.rs >/dev/null; then
    printf '  [BADSPEC] %-58s (sed matched nothing)\n' "$desc"; RC=1; return
  fi
  if ! cargo build --release >"$WORK/build.log" 2>&1; then
    printf '  [SKIP   ] %-58s (does not compile)\n' "$desc"; return
  fi
  cargo test --release >"$WORK/test.log" 2>&1
  local outcome
  if [ ! -s "$WORK/test.log" ]; then
    outcome="ERROR"
  elif grep -qE '^test result: FAILED' "$WORK/test.log"; then
    outcome="caught"
  else
    outcome="survived"
  fi
  if [ "$expect" = caught ] && [ "$outcome" = caught ]; then
    local n; n=$(grep -cE '^test .* FAILED' "$WORK/test.log")
    printf '  [ OK    ] %-58s caught (%s failing tests)\n' "$desc" "$n"
  elif [ "$expect" = equivalent ] && [ "$outcome" = survived ]; then
    printf '  [ OK    ] %-58s survived, as expected (provably equivalent)\n' "$desc"
  else
    printf '  [FAIL   ] %-58s expected %s, got %s\n' "$desc" "$expect" "$outcome"; RC=1
  fi
}

echo "=== Mutations that MUST be caught ==="
run_mutation "M1  saturating cast instead of C wraparound" caught \
  's|(as_i32 as u32 \& 0xff) as ::std::os::raw::c_uchar|value as ::std::os::raw::c_uchar|'
run_mutation "M2  removeGamma threshold 0.04045 -> 0.0392" caught \
  's|c > 0.04045|c > 0.0392|'
run_mutation "M4  removeGamma math in f32 instead of f64" caught \
  's|let c = channel as f64;|let c = channel;|'
run_mutation "M5  R-channel regrouped as R+(aG-bB)" caught \
  's|\*Red = (R + 0.127_398_863_108_80_f32 \* G) - 0.127_398_863_410_72_f32 \* B;|*Red = R + (0.127_398_863_108_80_f32 * G - 0.127_398_863_410_72_f32 * B);|'
run_mutation "M11 drop the +0.5f offset in cbDenorm" caught \
  's|\* 255.0f32 + 0.5f32|* 255.0f32|g'
run_mutation "M12 round() instead of trunc() in the cast" caught \
  's|let truncated = value.trunc();|let truncated = value.round();|'
run_mutation "M14 12.92 -> 12.29 (digit transposition)" caught \
  's|12\.92|12.29|g'
run_mutation "M16 removeGamma exponent 2.4 -> 2.2" caught \
  's|powf(2.4)|powf(2.2)|'
run_mutation "M20 applyGamma threshold x10" caught \
  's|0.003_130_804_953_560_371_517_027_863_777_09|0.031_308_049_535_603_715_170_278_637_770_9|'
run_mutation "M22 cbNorm divides by 256 instead of 255" caught \
  's|/ 255.0f32|/ 256.0f32|g'

echo
echo "=== Mutations that MUST survive (provably equivalent / unreachable) ==="
# The C source spells these two coefficients with 14 significant decimal digits
# but suffixes them with `f`, so BOTH round to the SAME f32 bit pattern
# (difference ~3e-10 vs f32 spacing ~1.5e-8). Swapping them is a no-op.
run_mutation "M9  R coeff pair collapses to one f32 (bit-identical)" equivalent \
  's|0.127_398_863_108_80_f32|0.127_398_863_410_72_f32|'
run_mutation "M10 G/B 0.8739 pair collapses to one f32" equivalent \
  's|0.873_909_299_283_61_f32|0.873_909_297_258_48_f32|'
# Perturbations far below the u8 output precision (verified over all 2^24 inputs).
run_mutation "M3  applyGamma exponent +1e-11 (below u8 precision)" equivalent \
  's|powf(0.4166666666)|powf(0.41666666666)|'
run_mutation "M19 sign flip of the 4.486E-11 cross-term" equivalent \
  's|(-4.486E-11_f32)|(4.486E-11_f32)|'
# ERRORS.md E6: the NaN branch of the cast is unreachable from the public API.
run_mutation "M23 NaN sentinel changed (unreachable path, ERRORS E6)" equivalent \
  's|-2147483648i32|0i32|'

echo
if [ $RC -eq 0 ]; then echo "RESULT: all mutations behaved as expected"; else echo "RESULT: UNEXPECTED mutation outcomes"; fi
exit $RC
