#!/usr/bin/env bash
# Negative control for the differential suite.
#
# Injects deliberate divergences into the Rust translation one at a time and
# asserts the suite CATCHES each one. A suite that passes a mutated translation
# would be vacuous, so every non-equivalent mutation must fail at least one test.
#
# Usage: ./mutation_check.sh
set -u
cd "$(dirname "$0")"

BK_IMP="$(mktemp)"; BK_LIB="$(mktemp)"
cp src/imp.rs "$BK_IMP"; cp src/lib.rs "$BK_LIB"
restore() { cp "$BK_IMP" src/imp.rs; cp "$BK_LIB" src/lib.rs; }
trap restore EXIT

# file @@ literal-search @@ literal-replace @@ description
MUTATIONS=$(cat <<'LIST'
src/imp.rs@@t >= 2147483648.0@@t > 2147483648.0@@int-cast upper bound off by one
src/imp.rs@@let t = v.trunc();@@let t = v.round();@@int cast rounds instead of truncating
src/imp.rs@@while buf.len() + 1 < size {@@while buf.len() + 2 < size {@@fgets takes one byte too few
src/imp.rs@@if byte[0] == b'\n' {@@if byte[0] == b'\r' {@@fgets stops on CR instead of LF
src/imp.rs@@if kept < 28 {@@if kept < 3 {@@hex-float mantissa truncated early
src/imp.rs@@let mut exp: i32 = -4 * frac_count;@@let mut exp: i32 = -3 * frac_count;@@hex-float fraction scaled wrongly
src/imp.rs@@b"fgets() failed."@@b"fgets() failed!"@@diagnostic string typo
src/imp.rs@@b"This would result in a divide by zero"@@b"This would result in a divide by Zero"@@guard message typo
src/imp.rs@@write!(out, "{}\n", int_number)@@write!(out, "{}\r\n", int_number)@@printIntLine line ending
src/imp.rs@@let data: f32 = 2.0f32;@@let data: f32 = 3.0f32;@@goodG2B constant changed
src/imp.rs@@let _ = out.write_all(line);@@let _ = out.write_all(&line[..line.len().saturating_sub(1)]);@@printLine drops last byte
src/imp.rs@@eq_ci_prefix(rest, b"inf")@@eq_ci_prefix(rest, b"infx")@@"inf" no longer recognised
src/imp.rs@@matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')@@matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c)@@CR not treated as whitespace
src/imp.rs@@negative = s[i] == b'-';@@negative = false;@@atof ignores the sign
src/imp.rs@@let magnitude: f64 = text.parse().unwrap_or(0.0);@@let magnitude: f64 = text.parse::<f32>().unwrap_or(0.0) as f64;@@atof parses at f32 precision
src/imp.rs@@if (data as f64).abs() > 0.000001 {@@if (data as f64) > 0.000001 {@@guard drops fabs()
src/imp.rs@@if v.is_nan() {@@if v.is_nan() && false {@@NaN no longer forced to INT_MIN
src/imp.rs@@print_line(Some(b"fgets() failed."));@@{}@@fgets failure prints nothing
src/lib.rs@@if line.is_null() {@@if !line.is_null() {@@printLine NULL check inverted
src/lib.rs@@imp::print_int_line(int_number as i32);@@imp::print_int_line((int_number as i32).wrapping_neg());@@printIntLine negates
src/lib.rs@@imp::program_main() as c_int@@{ imp::program_main(); 1 }@@main returns 1 instead of 0
LIST
)

pass=0; caught=0; missed=0; skipped=0
echo "=== mutation negative control ==="
while IFS= read -r entry; do
  [ -z "$entry" ] && continue
  file="${entry%%@@*}";  rest="${entry#*@@}"
  find="${rest%%@@*}";   rest="${rest#*@@}"
  repl="${rest%%@@*}";   desc="${rest#*@@}"

  restore
  if ! F="$find" R="$repl" perl -0pi \
        -e 'my $f=$ENV{F}; my $r=$ENV{R}; my $n = s/\Q$f\E/$r/g; exit(3) unless $n;' "$file"; then
    printf '  SKIP    %-46s (pattern not found)\n' "$desc"; skipped=$((skipped+1)); continue
  fi

  out=$(timeout 600 cargo test --offline --no-default-features --test differential \
          -- --test-threads=1 2>&1)
  if echo "$out" | grep -qE "could not compile|^error\[E" ; then
    printf '  SKIP    %-46s (does not compile)\n' "$desc"; skipped=$((skipped+1))
  elif echo "$out" | grep -q "test result: ok"; then
    printf '  MISSED  %-46s <-- suite is blind to this!\n' "$desc"; missed=$((missed+1))
  elif echo "$out" | grep -q "test result: FAILED"; then
    n=$(echo "$out" | grep -oE "[0-9]+ passed; [0-9]+ failed" | head -1)
    printf '  CAUGHT  %-46s (%s)\n' "$desc" "$n"; caught=$((caught+1))
  else
    # No "test result" line at all: the mutated library crashed the runner,
    # which is still a detected divergence.
    printf '  CAUGHT  %-46s (test binary aborted)\n' "$desc"; caught=$((caught+1))
  fi
done <<< "$MUTATIONS"

restore
echo
echo "caught=$caught missed=$missed skipped=$skipped"
out=$(timeout 600 cargo test --offline --no-default-features --test differential \
        -- --test-threads=1 2>&1)
if echo "$out" | grep -q "test result: ok"; then
  echo "restored (correct) tree: PASS"
else
  echo "restored (correct) tree: FAIL <-- restore went wrong"; exit 1
fi
[ "$missed" -eq 0 ] || { echo "FAIL: suite missed $missed mutation(s)"; exit 1; }
echo "OK: every injected divergence was detected"
