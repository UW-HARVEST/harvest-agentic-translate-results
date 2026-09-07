#!/usr/bin/env bash
# Meta-test: prove the differential suite actually DETECTS divergence.
# Each mutation is a small, plausible mistranslation of c_src/src/lib.c.
# Every one MUST make the suite fail; a surviving mutant means a blind spot.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"
orig="$(mktemp)"; cp src/lib.rs "$orig"
restore() { cp "$orig" src/lib.rs; }
trap restore EXIT

survivors=0
equivalents_ok=0

# mutate <expect: kill|equiv> <name> <from> <to>
#   kill  = the mutant changes observable behaviour; the suite MUST fail.
#   equiv = the mutant is provably observationally equivalent to the C (see the
#           proof next to each one); the suite MUST still pass. A "kill" here
#           would mean the suite is asserting something the C does not promise.
mutate() {
  local expect="$1" name="$2" from="$3" to="$4"
  restore
  python3 - "$from" "$to" <<'PY'
import sys
p='src/lib.rs'; s=open(p).read()
n=s.replace(sys.argv[1], sys.argv[2], 1)
assert n!=s, "mutation pattern not found: "+sys.argv[1]
open(p,'w').write(n)
PY
  if [ $? -ne 0 ]; then echo "!! could not apply mutation: $name"; survivors=$((survivors+1)); return; fi
  cargo build --offline --release >/dev/null 2>&1
  if RUST_SO_PATH="$here/target/release/librgb_to_hsv_lib.so" \
     cargo test --offline --release >/dev/null 2>&1; then
    if [ "$expect" = "equiv" ]; then
      echo "survived (expected, equivalent mutant): $name"
      equivalents_ok=$((equivalents_ok+1))
    else
      echo "SURVIVED (BAD): $name"
      survivors=$((survivors+1))
    fi
  else
    if [ "$expect" = "equiv" ]; then
      echo "KILLED (BAD - suite over-asserts): $name"
      survivors=$((survivors+1))
    else
      echo "killed         : $name"
    fi
  fi
}

# EQUIVALENCE PROOF (c_min <=): `<` and `<=` differ only when a == b, where they
# return a vs b. For equal non-zero floats the bit patterns are identical, so the
# only distinguishable case is {a,b} = {+0.0, -0.0}, giving min = +0.0 vs -0.0.
# `min` is used only in `delta = max - min` and (via delta) in `s`/`h`. If min is
# a zero then max >= 0, and max - (+0.0) == max - (-0.0) == max for max > 0. If
# max is also a zero, every component is a zero, so `max == 0` fires and the
# early-out returns before delta is ever observed. Hence unobservable.
mutate equiv "c_min uses <= instead of <"          "fn c_min(a: c_float, b: c_float) -> c_float {
    if a < b {"  "fn c_min(a: c_float, b: c_float) -> c_float {
    if a <= b {"
mutate kill "c_max uses >= instead of >"          "fn c_max(a: c_float, b: c_float) -> c_float {
    if a > b {"  "fn c_max(a: c_float, b: c_float) -> c_float {
    if a >= b {"
mutate kill "c_min replaced by f32::min"          "    if a < b {
        a
    } else {
        b
    }" "    a.min(b)"
mutate kill "c_max replaced by f32::max"          "    if a > b {
        a
    } else {
        b
    }
}

/// Convert" "    a.max(b)
}

/// Convert"
mutate kill "early-out || becomes &&"             "if delta == 0.0 || max == 0.0" "if delta == 0.0 && max == 0.0"
mutate kill "early-out drops the max==0 disjunct" "if delta == 0.0 || max == 0.0" "if delta == 0.0"
mutate kill "early-out drops the delta==0 disjunct" "if delta == 0.0 || max == 0.0" "if max == 0.0"
# EQUIVALENCE PROOF (branch order r/g): the two orders differ only when
# r == max AND g == max. Then b <= max, so b == min and delta == max - b.
# C takes the r-branch: h = (g - b)/delta = (max - b)/delta = delta/delta = 1.
# The mutant takes the g-branch: h = 2 + (b - r)/delta = 2 + (-delta)/delta = 1.
# Both are exactly 1 in IEEE-754 (delta != 0 here, else the early-out fired),
# and both yield NaN identically when delta is infinite. Hence unobservable.
mutate equiv "hue branch order: g tested before r" "if r == max {
        h = (g - b) / delta;
    } else if g == max {
        h = 2.0 + (b - r) / delta;" "if g == max {
        h = 2.0 + (b - r) / delta;
    } else if r == max {
        h = (g - b) / delta;"
mutate kill "g-branch constant 2 -> 4"            "h = 2.0 + (b - r) / delta;" "h = 4.0 + (b - r) / delta;"
mutate kill "else-branch operands swapped"        "h = 4.0 + (r - g) / delta;" "h = 4.0 + (g - r) / delta;"
mutate kill "r-branch operands swapped"           "h = (g - b) / delta;" "h = (b - g) / delta;"
mutate kill "wrap uses <= instead of <"           "if h < 0.0 {
        h += 360.0;" "if h <= 0.0 {
        h += 360.0;"
mutate kill "wrap removed"                        "if h < 0.0 {
        h += 360.0;
    }" ""
mutate kill "saturation inverted"                 "s = delta / max;" "s = max / delta;"
mutate kill "value uses min instead of max"       "v = max;" "v = min;"
mutate kill "hue scaled by 6 instead of 60"       "h *= 60.0;" "h *= 6.0;"
mutate kill "h initialised to 1 not 0"            "let mut h: c_float = 0.0;" "let mut h: c_float = 1.0;"
mutate kill "s initialised to 1 not 0"            "let mut s: c_float = 0.0;" "let mut s: c_float = 1.0;"
mutate kill "dest indices 0 and 2 swapped (early)" "            *dest.add(0) = h;
            *dest.add(1) = s;
            *dest.add(2) = v;
        }
        return;" "            *dest.add(2) = h;
            *dest.add(1) = s;
            *dest.add(0) = v;
        }
        return;"
mutate kill "min seeded from g instead of r"      "let mut min: c_float = r;" "let mut min: c_float = g;"
mutate kill "loads reordered (src[1] read as src[2])" "let g: c_float = unsafe { *src.add(1) };" "let g: c_float = unsafe { *src.add(2) };"
# EQUIVALENCE PROOF (delta in f64): the difference of two f32 values is exactly
# representable in f64 (Sterbenz / exponent-range argument), so the f64
# subtraction is exact and the single `as f32` rounding reproduces the f32
# subtraction bit for bit, including overflow to +/-inf. A SINGLE arithmetic op
# widened is safe; note that widening a *chain* is NOT (see the s= and h= f64
# mutants below, which the suite kills via double rounding).
mutate equiv "operates in f64 internally"         "delta = max - min;" "delta = ((max as f64) - (min as f64)) as c_float;"
mutate kill "delta computed as min - max"         "delta = max - min;" "delta = min - max;"

# --- Sharper near-miss mutants: these MUST be killed. ---------------------
# Double rounding: widening the DIVISION (not just one subtraction) changes the
# result for some inputs, unlike the equivalent single-subtraction widening.
# EQUIVALENCE PROOF (single op widened to f64): Figueroa's theorem states that
# double rounding is innocuous for +, -, *, / and sqrt when the intermediate
# precision p2 >= 2*p1 + 2. Here p1 = 24 (binary32) and p2 = 53 (binary64), and
# 53 >= 50, so widening a SINGLE f32 operation to f64 and rounding back gives
# the f32 result bit for bit. `delta` and `max` are already f32 values, so this
# is one widened division => equivalent. (Contrast the mutants below that widen
# a CHAIN: there the intermediate `g - b` is kept in f64 instead of being
# rounded to f32 as the C does, which IS observable and IS killed.)
mutate equiv "saturation computed in f64 (single widened op)" \
  "s = delta / max;" "s = ((delta as f64) / (max as f64)) as c_float;"
mutate kill "r-branch hue computed in f64 (double rounding)" \
  "h = (g - b) / delta;" "h = (((g as f64) - (b as f64)) / (delta as f64)) as c_float;"
mutate kill "g-branch hue fused into one f64 expression" \
  "h = 2.0 + (b - r) / delta;" "h = (2.0f64 + ((b as f64) - (r as f64)) / (delta as f64)) as c_float;"
mutate kill "hue scale 60 -> 60.000004 (1 ulp)" \
  "h *= 60.0;" "h *= 60.000004;"
mutate kill "hue scale split into two roundings (h*30*2 -> h*7*60/7)" \
  "h *= 60.0;" "h = h * 7.0 * (60.0 / 7.0);"
# EQUIVALENCE PROOF (any branch reordering): the three branches can only
# disagree when two components tie for the max, and there the formulae coincide
# exactly. If r == g == max then b == min, so the r-branch gives
# (g-b)/delta = delta/delta = 1 and the g-branch gives 2 + (b-r)/delta = 2-1 = 1.
# Symmetrically for r == b == max (r-branch -> -1, b-branch -> 4+1 = 5, and
# -1*60 + 360 == 5*60) and g == b == max. When any component is NaN every
# equality is false and each ordering lands in a branch whose result is NaN.
# So branch ORDER is genuinely unobservable; only the per-branch FORMULAE are
# observable, and mutating those is killed above.
mutate equiv "hue branch order: else-branch tested first" \
  "if r == max {
        h = (g - b) / delta;
    } else if g == max {
        h = 2.0 + (b - r) / delta;
    } else {
        h = 4.0 + (r - g) / delta;
    }" "if r != max && g != max {
        h = 4.0 + (r - g) / delta;
    } else if g == max {
        h = 2.0 + (b - r) / delta;
    } else {
        h = (g - b) / delta;
    }"
# EQUIVALENCE PROOF (rem_euclid): because |g-b| <= delta, |b-r| <= delta and
# |r-g| <= delta, the pre-scale hue always lies in [-1, 5], so after `h *= 60`
# we have h in [-60, 300] (or NaN) and h can never overflow. For h in [0, 300]
# `h % 360 == h`, and for h in [-60, 0) rem_euclid performs exactly the same
# `h + 360.0` f32 addition the C does; -0.0 stays -0.0 in both; NaN stays NaN.
# Equivalent. (It would NOT be equivalent if h could reach +/-inf, which the
# bound above rules out.)
mutate equiv "wrap replaced by rem_euclid(360)" \
  "if h < 0.0 {
        h += 360.0;
    }" "h = h.rem_euclid(360.0);"
# NaN-suppressing min/max via clamp-style logic.
mutate kill "max via total_cmp ordering" \
  "    if a > b {
        a
    } else {
        b
    }
}

/// Convert" "    if a.total_cmp(&b) == std::cmp::Ordering::Greater {
        a
    } else {
        b
    }
}

/// Convert"
# Early-out comparisons made sign-of-zero sensitive.
mutate kill "early-out uses to_bits() == 0 instead of == 0.0" \
  "if delta == 0.0 || max == 0.0" "if delta.to_bits() == 0 || max.to_bits() == 0"
# Stores clamped / sanitised (a very common \"helpful\" translation error).
mutate kill "outputs clamped to [0,1] / [0,360)" \
  "        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}" "        *dest.add(0) = h.clamp(0.0, 360.0);
        *dest.add(1) = s.clamp(0.0, 1.0);
        *dest.add(2) = v.clamp(0.0, 1.0);
    }
}"
# NaN normalisation on output.
mutate kill "NaN outputs normalised to 0" \
  "        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}" "        *dest.add(0) = if h.is_nan() { 0.0 } else { h };
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}"
# Aliasing/ordering slip: v recomputed from src AFTER dest[0..2] were written.
# Indistinguishable for disjoint buffers, but wrong whenever dest overlaps src -
# exactly what CONFIGS.md rows 26-29 and ERRORS.md rows 13-14 exist to catch.
mutate kill "v recomputed from src after the stores (aliasing-visible)" \
  "        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}" "        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = c_max(c_max(*src.add(0), *src.add(1)), *src.add(2));
    }
}"
# Same slip on the early-out path.
mutate kill "early-out re-reads src after writing dest[0] (aliasing-visible)" \
  "            *dest.add(0) = h;
            *dest.add(1) = s;
            *dest.add(2) = v;
        }
        return;" "            *dest.add(0) = h;
            *dest.add(1) = s;
            *dest.add(2) = c_max(c_max(*src.add(0), *src.add(1)), *src.add(2));
        }
        return;"

restore
cargo build --offline --release >/dev/null 2>&1
echo
echo "equivalent mutants that correctly survived: $equivalents_ok"
if [ "$survivors" -eq 0 ]; then
  echo "MUTATION CHECK PASSED: every behaviour-changing mutant was killed, and"
  echo "every provably-equivalent mutant correctly survived."
else
  echo "MUTATION CHECK FAILED: $survivors mutant(s) survived -> the suite has a blind spot."
fi
exit "$survivors"
