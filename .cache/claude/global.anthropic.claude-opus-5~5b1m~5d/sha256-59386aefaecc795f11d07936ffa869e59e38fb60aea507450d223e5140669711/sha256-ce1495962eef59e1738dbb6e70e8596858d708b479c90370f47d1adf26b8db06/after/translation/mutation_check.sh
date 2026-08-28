#!/usr/bin/env bash
# Negative control for the Phase B/C differential suites.
#
# A suite that passes on the real translation proves nothing unless it also
# FAILS on a broken one. This script injects deliberate, individually plausible
# translation bugs into COPIES of the Rust crate and checks the suite's verdict.
#
# Two categories:
#   REQUIRED_CAUGHT  -- a real behavioural bug; the suite MUST fail.
#   KNOWN_EQUIVALENT -- a source change that is provably output-identical to the
#                       C for every input; the suite MUST still pass. Asserting
#                       this direction too guards against a suite that "fails on
#                       everything" and therefore proves nothing either.
#
# translation/src/lib.rs is never modified: all work happens in .mutscratch/.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
C_SO="$(ls "$HERE"/../c_src/build/lib*.so | head -1)"
WORK="$HERE/.mutscratch"
rm -rf "$WORK"; mkdir -p "$WORK"

REQUIRED_CAUGHT="
M01_min_max_nan_semantics
M02_drop_short_circuit_max_zero
M03_branch_a_numerator_sign
M04_branch_b_constant
M05_branch_c_numerator_sign
M06_wrap_constant
M07_wrap_condition_le
M08_reload_src_after_store
M09_dest_lane_order
M10_saturation_denominator
M11_min_max_swapped
M12_early_out_writes_s_as_delta
"

KNOWN_EQUIVALENT="
E01_tie_precedence_g_before_r
"

apply_mutation() { # $1 = lib.rs path, $2 = mutation name
  python3 - "$1" "$2" <<'PY'
import sys
path, mut = sys.argv[1], sys.argv[2]
s = before = open(path).read()

MIN_BODY = "    if a < b {\n        a\n    } else {\n        b\n    }\n"
MAX_BODY = "    if a > b {\n        a\n    } else {\n        b\n    }\n"
STORES   = ("    unsafe {\n"
            "        *dest.add(0) = h;\n"
            "        *dest.add(1) = s;\n"
            "        *dest.add(2) = v;\n"
            "    }\n}")

if mut == "M01_min_max_nan_semantics":
    # f32::min/max suppress NaN and normalise signed zeros -- the single most
    # likely mistake when translating the C ternary macros.
    s = s.replace(MIN_BODY, "    a.min(b)\n", 1).replace(MAX_BODY, "    a.max(b)\n", 1)
elif mut == "M02_drop_short_circuit_max_zero":
    s = s.replace("if delta == 0.0 || max == 0.0 {", "if delta == 0.0 {", 1)
elif mut == "M03_branch_a_numerator_sign":
    s = s.replace("h = (g - b) / delta;", "h = (b - g) / delta;", 1)
elif mut == "M04_branch_b_constant":
    s = s.replace("h = 2.0 + (b - r) / delta;", "h = 3.0 + (b - r) / delta;", 1)
elif mut == "M05_branch_c_numerator_sign":
    s = s.replace("h = 4.0 + (r - g) / delta;", "h = 4.0 + (g - r) / delta;", 1)
elif mut == "M06_wrap_constant":
    s = s.replace("h += 360.0;", "h += 359.9;", 1)
elif mut == "M07_wrap_condition_le":
    s = s.replace("if h < 0.0 {", "if h <= 0.0 {", 1)
elif mut == "M08_reload_src_after_store":
    # Output-identical for disjoint buffers; differs only under aliasing.
    s = s.replace(STORES, STORES.replace(
        "*dest.add(2) = v;",
        "*dest.add(2) = c_max(c_max(*src.add(0), *src.add(1)), *src.add(2));"), 1)
elif mut == "M09_dest_lane_order":
    s = s.replace(STORES, ("    unsafe {\n"
                           "        *dest.add(0) = s;\n"
                           "        *dest.add(1) = h;\n"
                           "        *dest.add(2) = v;\n"
                           "    }\n}"), 1)
elif mut == "M10_saturation_denominator":
    s = s.replace("s = delta / max;", "s = delta / v;", 1)   # v==max, but then:
    s = s.replace("let v: c_float;", "let mut v: c_float;", 1)
    s = s.replace("v = max;", "v = max; if v > 1.0 { v = 1.0; }", 1)  # bogus clamp
elif mut == "M11_min_max_swapped":
    s = s.replace("min = c_min(min, g);", "min = c_max(min, g);", 1)
elif mut == "M12_early_out_writes_s_as_delta":
    s = s.replace("            *dest.add(1) = s;", "            *dest.add(1) = delta;", 1)
elif mut == "E01_tie_precedence_g_before_r":
    # Test `g == max` BEFORE `r == max`. When r == g == max the two formulas
    # are algebraically identical: min is then b, delta == max - b, so
    #   branch A: (g - b)/delta     = (max - b)/(max - b)      = +1.0
    #   branch B: 2 + (b - r)/delta = 2 + (b - max)/(max - b)  = 2 - 1 = +1.0
    # Both are EXACT in binary32 (x/x == 1.0, 2.0 + -1.0 == 1.0), so no input
    # can distinguish them. Hence: provably equivalent, must NOT be caught.
    s = s.replace("""    if r == max {
        h = (g - b) / delta;
    } else if g == max {
        h = 2.0 + (b - r) / delta;
    } else {""", """    if g == max {
        h = 2.0 + (b - r) / delta;
    } else if r == max {
        h = (g - b) / delta;
    } else {""", 1)
else:
    sys.exit("unknown mutation " + mut)

if s == before:
    sys.exit("MUTATION %s DID NOT APPLY -- pattern not found" % mut)
open(path, "w").write(s)
PY
}

run_one() { # $1 = mutation name -> echoes "CAUGHT <names>" or "PASSED"
  local m="$1" d="$WORK/$1"
  mkdir -p "$d/.cargo"
  cp -r "$HERE/src" "$HERE/tests" "$HERE/Cargo.toml" "$d/"
  [ -f "$HERE/Cargo.lock" ] && cp "$HERE/Cargo.lock" "$d/"
  printf '[net]\noffline = true\n' > "$d/.cargo/config.toml"

  apply_mutation "$d/src/lib.rs" "$m" || { echo "APPLY_FAIL"; return; }
  ( cd "$d" && cargo build -q 2>"$d/build.err" && cargo build -q --release 2>>"$d/build.err" ) \
      || { echo "BUILD_FAIL"; return; }

  # --no-fail-fast: without it cargo stops after the first failing test binary,
  # so the second suite's attribution would be silently lost.
  ( cd "$d" && HARVEST_C_SO="$C_SO" \
      cargo test -q --no-fail-fast \
        --test phase_b_configs --test phase_c_errors 2>&1 ) > "$d/test.log"

  if grep -q 'test result: FAILED' "$d/test.log"; then
    local names
    names=$(sed -n '/^failures:$/,$p' "$d/test.log" \
            | grep -oE '\b(cfg|err)_row[0-9]+[a-z_0-9]*' | sort -u | tr '\n' ' ')
    echo "CAUGHT ${names:-<unnamed>}"
  else
    echo "PASSED"
  fi
}

fails=0

echo "=== REQUIRED_CAUGHT: injected bug must make the suite FAIL ==="
for m in $REQUIRED_CAUGHT; do
  r=$(run_one "$m"); verdict=${r%% *}; names=${r#* }
  case "$verdict" in
    CAUGHT) printf '  OK   %-36s caught by: %s\n' "$m" "$names" ;;
    PASSED) printf '  FAIL %-36s *** BLIND SPOT: suite did not notice ***\n' "$m"; fails=$((fails+1)) ;;
    *)      printf '  FAIL %-36s %s (see %s)\n' "$m" "$verdict" "$WORK/$m"; fails=$((fails+1)) ;;
  esac
done

echo
echo "=== KNOWN_EQUIVALENT: provably output-identical, suite must still PASS ==="
for m in $KNOWN_EQUIVALENT; do
  r=$(run_one "$m"); verdict=${r%% *}; names=${r#* }
  case "$verdict" in
    PASSED) printf '  OK   %-36s not flagged (correct: equivalent mutant)\n' "$m" ;;
    CAUGHT) printf '  FAIL %-36s flagged by %s -- equivalence claim is WRONG\n' "$m" "$names"; fails=$((fails+1)) ;;
    *)      printf '  FAIL %-36s %s\n' "$m" "$verdict"; fails=$((fails+1)) ;;
  esac
done

echo
if [ "$fails" -eq 0 ]; then
  echo "mutation check PASSED: every real bug detected, every equivalent mutant ignored."
  rm -rf "$WORK"
  exit 0
fi
echo "mutation check FAILED: $fails problem(s). Artifacts kept in $WORK"
exit 1
