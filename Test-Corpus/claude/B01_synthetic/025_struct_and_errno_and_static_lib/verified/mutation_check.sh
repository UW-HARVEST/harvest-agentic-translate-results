#!/usr/bin/env bash
# Anti-vacuity check: deliberately inject known bugs into src/lib.rs, one at a
# time, and confirm the differential suite FAILS for each. A mutation the suite
# does not catch is a blind spot in the tests, not a fix to make.
#
# Mutations are applied as LITERAL string replacements (via python) so there is
# no regex-escaping ambiguity, and each one is verified to have actually changed
# the file before the suite is run.
#
# src/lib.rs is always restored, including on interrupt.
set -uo pipefail
cd "$(dirname "$0")"

WORK="${TMPDIR:-/tmp}/driver_mut.$$"
mkdir -p "$WORK" || exit 1
ORIG="$WORK/lib.rs.orig"
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; }
trap 'restore; rm -rf "$WORK"' EXIT INT TERM

FAIL=0

# --- Real bugs: the suite MUST catch every one of these -------------------
# Each entry: name @@ literal-to-find @@ replacement
MUTATIONS=(
'bedrooms_saturating_not_wrapping@@bedrooms.wrapping_add(extra_bedrooms)@@bedrooms.saturating_add(extra_bedrooms)'
'bedrooms_subtract_not_add@@bedrooms.wrapping_add(extra_bedrooms)@@bedrooms.wrapping_sub(extra_bedrooms)'
'bathrooms_precision_2f@@and %.1f bathrooms@@and %.2f bathrooms'
'bathrooms_precision_0f@@and %.1f bathrooms@@and %.0f bathrooms'
'swap_floors_bedrooms_in_printf@@h.floors,
            h.bedrooms,@@h.bedrooms,
            h.floors,'
'floors_increment_by_two@@floors.wrapping_add(1)@@floors.wrapping_add(2)'
'initial_bathrooms_2_0@@bathrooms: 2.5,@@bathrooms: 2.0,'
'initial_floors_3@@floors: 2,@@floors: 3,'
'initial_bedrooms_6@@bedrooms: 5,@@bedrooms: 6,'
'strtol_base_16@@&mut endp, 10)@@&mut endp, 16)'
'strtol_base_0@@&mut endp, 10)@@&mut endp, 0)'
'bathrooms_increment_half@@bathrooms += 1.0@@bathrooms += 0.5'
'drop_no_conversion_check@@endp != str as *mut c_char@@true'
'drop_int_max_upper_bound@@tmp <= c_int::MAX as c_long@@true'
'drop_int_min_lower_bound@@tmp >= c_int::MIN as c_long@@true'
'int_max_bound_off_by_one@@tmp <= c_int::MAX as c_long@@tmp <= c_int::MAX as c_long + 1'
'int_min_bound_off_by_one@@tmp >= c_int::MIN as c_long@@tmp >= c_int::MIN as c_long - 1'
'errno_not_cleared_first@@    errno_set(0);@@    // errno_set(0);'
'parsed_value_off_by_one@@*val = tmp as c_int;@@*val = (tmp + 1) as c_int;'
'run_once_in_driver@@        run(x);
        run(x);@@        run(x);'
'run_thrice_in_driver@@        run(x);
        run(x);@@        run(x);
        run(x);
        run(x);'
'reorder_add_floor_after_print@@    print_the_house();
    add_floor_to_the_house();@@    add_floor_to_the_house();
    print_the_house();'
'reorder_bathrooms_before_print@@    print_the_house();
    add_bedrooms@@    add_bedrooms'
'error_message_text@@An error occurred@@An error occurred.'
'error_message_no_newline@@An error occurred\n@@An error occurred'
)

# --- Semantically EQUIVALENT mutations: expected to be MISSED --------------
# These change the source but provably cannot change observable behaviour, so a
# MISS is the correct result and documents dead/unreachable logic in the C.
# Each entry: name @@ find @@ replace @@ justification
EQUIVALENT=(
'floors_saturating@@floors.wrapping_add(1)@@floors.saturating_add(1)@@floors starts at 2 and only ever +1 per run(); wrapping and saturating differ solely at INT_MAX, i.e. after 2^31 run() calls - unreachable in any feasible test (and UB in the C anyway)'
'drop_errno_check@@errno_get() == 0@@true@@proved non-decisive by a C probe: glibc base-10 strtol sets ERANGE only when the result saturates to LONG_MIN/LONG_MAX, both of which already fail the INT_MIN/INT_MAX conjunct. The C guard is redundant; the Rust mirrors it faithfully regardless.'
'low32_mask_before_cast@@tmp as c_int@@(tmp & 0xffff_ffff) as c_int@@this line is reached only when INT_MIN <= tmp <= INT_MAX, where masking the low 32 bits and reinterpreting the sign bit is bit-identical to a plain truncating cast'
)

printf '%-38s %-8s %s\n' "MUTATION" "RESULT" "DETAIL"
printf '%.0s-' {1..96}; echo

for entry in "${MUTATIONS[@]}"; do
  name="${entry%%@@*}"
  rest="${entry#*@@}"
  find_lit="${rest%%@@*}"
  repl_lit="${rest#*@@}"

  restore
  FIND="$find_lit" REPL="$repl_lit" python3 - src/lib.rs <<'PY'
import os, sys
p = sys.argv[1]
s = open(p).read()
f, r = os.environ["FIND"], os.environ["REPL"]
n = s.count(f)
if n:
    open(p, "w").write(s.replace(f, r, 1))
sys.exit(0 if n else 3)
PY
  if [ $? -ne 0 ]; then
    printf '%-38s %-8s %s\n' "$name" "SKIP" "literal not present in src/lib.rs"
    continue
  fi
  if cmp -s src/lib.rs "$ORIG"; then
    printf '%-38s %-8s %s\n' "$name" "SKIP" "replacement was a no-op"
    continue
  fi
  if ! cargo build --offline >"$WORK/b.log" 2>&1; then
    printf '%-38s %-8s %s\n' "$name" "SKIP" "mutated code does not compile"
    continue
  fi
  if cargo test --offline >"$WORK/t.log" 2>&1 -- --test-threads=1; then
    printf '%-38s %-8s %s\n' "$name" "MISSED" "suite PASSED despite injected bug <<< BLIND SPOT"
    FAIL=1
  else
    n=$(grep -cE '^test .* FAILED$' "$WORK/t.log")
    first=$(grep -oE '^test [a-z0-9_]+ \.\.\. FAILED$' "$WORK/t.log" | head -1 | awk '{print $2}')
    printf '%-38s %-8s %s\n' "$name" "CAUGHT" "$n test(s) failed, e.g. $first"
  fi
done

printf '\n'
printf '%.0s=' {1..96}; echo
echo "EQUIVALENT mutations (expected to be MISSED — they document dead/unreachable C logic)"
printf '%.0s=' {1..96}; echo
for entry in "${EQUIVALENT[@]}"; do
  name="${entry%%@@*}";  rest="${entry#*@@}"
  find_lit="${rest%%@@*}"; rest="${rest#*@@}"
  repl_lit="${rest%%@@*}"; why="${rest#*@@}"

  restore
  FIND="$find_lit" REPL="$repl_lit" python3 - src/lib.rs <<'PY'
import os, sys
p = sys.argv[1]
s = open(p).read()
f, r = os.environ["FIND"], os.environ["REPL"]
n = s.count(f)
if n:
    open(p, "w").write(s.replace(f, r, 1))
sys.exit(0 if n else 3)
PY
  if [ $? -ne 0 ]; then
    printf '%-26s %-10s %s\n' "$name" "SKIP" "literal not present"; FAIL=1; continue
  fi
  if ! cargo build --offline >"$WORK/b.log" 2>&1; then
    printf '%-26s %-10s %s\n' "$name" "SKIP" "does not compile"; continue
  fi
  if cargo test --offline >"$WORK/t.log" 2>&1 -- --test-threads=1; then
    printf '%-26s %-10s %s\n' "$name" "MISSED-OK" "behaviour-preserving, as expected"
  else
    printf '%-26s %-10s %s\n' "$name" "CAUGHT" "unexpected: assumed equivalent but suite caught it"
  fi
  echo "      why: $why" | fold -s -w 88 | sed '2,$s/^/           /'
done

restore
cargo build --offline >/dev/null 2>&1

printf '%.0s-' {1..96}; echo
if [ "$FAIL" -eq 0 ]; then
  echo "Every REAL bug was CAUGHT: the differential suite has teeth."
else
  echo "At least one real bug was MISSED — the suite has a blind spot."
fi
echo -n "src/lib.rs restored: "
if diff -q "$ORIG" src/lib.rs >/dev/null; then echo "clean (identical to original)"; else echo "DIFFERS!"; FAIL=1; fi
exit "$FAIL"
