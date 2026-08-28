#!/usr/bin/env bash
# Sensitivity check for the differential suite.
#
# A passing test suite only means something if it can FAIL. This script builds a
# set of deliberately-altered Rust shared objects (each one a single, plausible
# edit of `src/lib.rs`), points the suite at each of them via HSV_RUST_SO, and
# checks the outcome against a prediction:
#
#   * `<test_name>` — the edit changes observable behaviour, so the suite MUST
#     reject the mutant, and specifically that test must be among the failures.
#     A surviving mutant is a blind spot in the tests.
#   * `SURVIVES`    — the edit is provably *equivalent* (documented per mutant),
#     so the suite MUST still pass. These are the control group: they show the
#     suite is not simply failing at random.
#
# Usage: ./mutation_check.sh        (run from the `translation` directory)
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$CRATE_DIR/target/mutants"
mkdir -p "$OUT"
PROBLEMS=0

mutate() { # name  "old@@@new"
  python3 - "$OUT/$1.rs" "$2" <<'PY'
import sys
dst, script = sys.argv[1], sys.argv[2]
s = open('src/lib.rs').read()
old, new = script.split('@@@')
old, new = old.replace('\\n', '\n'), new.replace('\\n', '\n')
if old not in s:
    sys.exit(f"mutation target not found: {old!r}")
open(dst, 'w').write(s.replace(old, new, 1))
PY
}

run_mutant() { # name  expectation
  local name="$1" expect="$2"
  if ! rustc --crate-type cdylib --edition 2021 -O -A warnings \
       -o "$OUT/lib$name.so" "$OUT/$name.rs" 2>"$OUT/$name.build.log"; then
    echo "  !! could not compile mutant $name"; tail -3 "$OUT/$name.build.log"
    PROBLEMS=$((PROBLEMS + 1)); return
  fi
  local log="$OUT/$name.test.log"
  HSV_RUST_SO="$OUT/lib$name.so" timeout 600 cargo test --offline --tests \
      --no-fail-fast >"$log" 2>&1
  local caught
  caught=$(grep -oE '^    (b[0-9]+_[a-z0-9_]+|err[0-9]+_[a-z0-9_]+)' "$log" |
           sed 's/^ *//' | sort -u | tr '\n' ' ')
  if [ "$expect" = SURVIVES ]; then
    if [ -z "$caught" ]; then
      echo "  survived, as predicted (equivalent transformation)"
    else
      echo "  !! predicted equivalent but the suite failed: $caught"
      PROBLEMS=$((PROBLEMS + 1))
    fi
    return
  fi
  if [ -z "$caught" ]; then
    echo "  !! SURVIVED (blind spot in the tests)"
    PROBLEMS=$((PROBLEMS + 1))
  elif ! grep -q "^    $expect\$" "$log"; then
    echo "  !! killed, but not by the predicted test ($expect); killed by: $caught"
    PROBLEMS=$((PROBLEMS + 1))
  else
    echo "  killed by $(echo "$caught" | wc -w) test(s), incl. $expect"
  fi
}

cd "$CRATE_DIR" || exit 1

echo "== 1 plain (sinkable) loads instead of pinned unconditional movss"
mutate m_plainload 'unsafe fn load_f32(p: *const c_float) -> c_float {\n    let out: c_float;@@@unsafe fn load_f32(p: *const c_float) -> c_float {\n    return *p;\n    #[allow(unreachable_code)] let out: c_float;'
run_mutant m_plainload err22_unconditional_h_load_faults

echo "== 2 p: SSE operand order swapped (NaN payload selection)"
mutate m_order_p '    p = mulss(subss(1.0f32, s), v);@@@    p = mulss(v, subss(1.0f32, s));'
run_mutant m_order_p b41_nan_payload_cross_product

echo "== 3 q: inner s*f operand order swapped"
mutate m_order_q '    q = mulss(subss(1.0f32, mulss(s, f)), v);@@@    q = mulss(subss(1.0f32, mulss(f, s)), v);'
run_mutant m_order_q b41_nan_payload_cross_product

echo "== 4 q: outer *v operand order swapped"
mutate m_order_qv '    q = mulss(subss(1.0f32, mulss(s, f)), v);@@@    q = mulss(v, subss(1.0f32, mulss(s, f)));'
run_mutant m_order_qv b41_nan_payload_cross_product

echo "== 5 t: inner (1-f)*s operand order swapped"
# EQUIVALENT: both operands can only be NaN together when f is NaN, which needs
# h to be NaN, which forces i == INT_MIN and therefore the `default:` arm — and
# `default:` never reads `t`. So this reordering is unobservable, exactly as the
# gcc-vs-clang analysis in NOTES.md predicts.
mutate m_order_t '    t = mulss(subss(1.0f32, mulss(subss(1.0f32, f), s)), v);@@@    t = mulss(subss(1.0f32, mulss(s, subss(1.0f32, f))), v);'
run_mutant m_order_t SURVIVES

echo "== 6 t: outer *v operand order swapped"
mutate m_order_tv '    t = mulss(subss(1.0f32, mulss(subss(1.0f32, f), s)), v);@@@    t = mulss(v, subss(1.0f32, mulss(subss(1.0f32, f), s)));'
run_mutant m_order_tv b41_nan_payload_cross_product

echo "== 7 Rust's saturating float->int cast instead of cvttss2si"
mutate m_satcast '    if x.is_nan() || x >= 2_147_483_648.0f32 || x <= -2_147_483_648.0f32 {\n        c_int::MIN\n    } else {\n        x as c_int\n    }@@@    x as c_int'
run_mutant m_satcast b23_hue_int_conversion_boundary

echo "== 8 float->int range check off by one ULP at +2^31"
mutate m_castbound '    if x.is_nan() || x >= 2_147_483_648.0f32 || x <= -2_147_483_648.0f32 {@@@    if x.is_nan() || x > 2_147_483_648.0f32 || x <= -2_147_483_648.0f32 {'
run_mutant m_castbound b23_hue_int_conversion_boundary

echo "== 9 reversed store order (dest[2] first)"
mutate m_storeorder '    store_f32(dest.add(0), r);\n    store_f32(dest.add(1), g);\n    store_f32(dest.add(2), b);@@@    store_f32(dest.add(2), b);\n    store_f32(dest.add(1), g);\n    store_f32(dest.add(0), r);'
run_mutant m_storeorder err23_partial_store_before_fault

echo "== 10 switch arms 3 and 4 swapped"
mutate m_swaparms '        3 => {\n            r = p;\n            g = q;\n            b = v;\n        }@@@        3 => {\n            r = t;\n            g = p;\n            b = v;\n        }'
run_mutant m_swaparms b13_arm3_random

echo "== 11 default arm returns (v, q, p) instead of (v, p, q)"
mutate m_defaultarm '        _ => {\n            r = v;\n            g = p;\n            b = q;\n        }@@@        _ => {\n            r = v;\n            g = q;\n            b = p;\n        }'
run_mutant m_defaultarm b15_arm5_default_random

echo "== 12 -0.0 no longer takes the s == 0 short-circuit"
mutate m_negzero '    if s == 0.0 {@@@    if s == 0.0 && !s.is_sign_negative() {'
run_mutant m_negzero err02_s_is_negative_zero

echo "== 13 short-circuit widened to subnormal s"
mutate m_subnormal '    if s == 0.0 {@@@    if s.abs() < f32::MIN_POSITIVE {'
run_mutant m_subnormal b27_s_subnormal

echo "== 14 SNaN operands not quieted on propagation"
mutate m_noquiet 'fn quiet(x: c_float) -> c_float {\n    c_float::from_bits(x.to_bits() | 0x0040_0000)@@@fn quiet(x: c_float) -> c_float {\n    x'
run_mutant m_noquiet err04_s_is_signalling_nan

echo "== 15 truncation instead of floor"
mutate m_trunc '    i = c_float_to_int(h.floor());@@@    i = c_float_to_int(h.trunc());'
run_mutant m_trunc b17_arm_negative_default_random

echo "== 16 the 60.0 constant perturbed by one ULP"
mutate m_const '    h = divss(h, 60.0f32);@@@    h = divss(h, 60.000004f32);'
run_mutant m_const b34_full_random_bitpatterns

echo "== 17 hue divided in f64 then rounded back to f32"
# EQUIVALENT: f64 has 53 > 2*24+2 bits, so a single f32 rounding of the f64
# quotient is the correctly-rounded f32 quotient (no double-rounding error).
mutate m_f64div '    h = divss(h, 60.0f32);@@@    h = if h.is_nan() { quiet(h) } else { (h as f64 / 60.0f64) as c_float };'
run_mutant m_f64div SURVIVES

echo "== 18 int->float conversion routed through f64"
# EQUIVALENT: i32 -> f64 is exact, so rounding that to f32 equals i32 -> f32.
mutate m_i2f '    f = subss(h, i as c_float);@@@    f = subss(h, i as f64 as c_float);'
run_mutant m_i2f SURVIVES

echo
if [ "$PROBLEMS" -eq 0 ]; then
  echo "ALL PREDICTIONS HELD — every behavioural mutant was killed by its target"
  echo "test, and every provably-equivalent mutant survived."
else
  echo "$PROBLEMS unexpected result(s)"
fi
exit "$PROBLEMS"
