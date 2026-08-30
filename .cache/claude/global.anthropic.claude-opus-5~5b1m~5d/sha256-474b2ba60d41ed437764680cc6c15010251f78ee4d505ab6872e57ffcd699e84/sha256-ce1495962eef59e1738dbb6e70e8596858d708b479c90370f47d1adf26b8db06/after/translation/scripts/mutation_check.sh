#!/usr/bin/env bash
# Harness self-validation: inject known mistranslations into the Rust source,
# one at a time, and require the differential suite to FAIL for each.
# A green suite is only meaningful if it can go red.
#
# The original src/lib.rs is restored on exit (including on interrupt).
#
# Only OBSERVABLE mutants are listed. Because goodG2B/goodB2G/bad take no input
# and use hard-coded constants, many textual mutations are observationally
# EQUIVALENT and are deliberately excluded (a differential test suite cannot and
# should not detect them). Verified equivalent, hence omitted:
#   * `data > 0`  ->  `data >= 0`            (data == CHAR_MAX == 127, both true)
#   * `data < CHAR_MAX/2` -> `<=`            (127 < 63 and 127 <= 63 both false)
#   * `CHAR_MAX/2` (63) -> `(CHAR_MAX+1)/2` (64)  (127 < 64 still false)
#   * `data < CHAR_MAX/2` -> `data < CHAR_MAX`    (127 < 127 still false)
#   * `(x*2) as c_char` -> `(x*2) as u8 as c_char` (both give -2 for x=127)
#   * printf("%02x", i32) -> Rust format!("{:02x}", i32) (identical rendering)
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
LIB=src/lib.rs
BACKUP="$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")"
cp "$LIB" "$BACKUP"
restore() { cp "$BACKUP" "$LIB"; }
trap 'restore; rm -f "$BACKUP"' EXIT INT TERM

CARGO_FLAGS="--offline"

# Each mutant is "name@@@literal-from@@@literal-to" (plain text, not regex).
MUTANTS=(
"driver-truncates-flag-to-u8@@@if useGood != 0 {@@@if (useGood as u8) != 0 {"
"driver-inverts-flag@@@if useGood != 0 {@@@if useGood == 0 {"
"driver-flag-as-bool-low-bit@@@if useGood != 0 {@@@if (useGood & 1) != 0 {"
"printhex-prints-unsigned-byte@@@let promoted: c_int = charHex as c_int;@@@let promoted: c_int = charHex as u8 as c_int;"
"printhex-drops-zero-pad@@@printf(b\"%02x\\n\\0\".as_ptr() as *const c_char, promoted);@@@printf(b\"%x\\n\\0\".as_ptr() as *const c_char, promoted);"
"printhex-no-abi-truncation@@@    print_hex_char_line_impl(charHex as c_char)@@@    printf(b\"%02x\\n\\0\".as_ptr() as *const c_char, charHex);"
"printhex-masks-to-byte@@@let promoted: c_int = charHex as c_int;@@@let promoted: c_int = (charHex as c_int) & 0xff;"
"printline-drops-null-guard@@@if !line.is_null() {@@@if true {"
"printline-prints-null-as-text@@@if !line.is_null() {@@@if line.is_null() { printf(b\"(null)\\n\\0\".as_ptr() as *const c_char); return; } else if true {"
"printline-adds-no-newline@@@printf(b\"%s\\n\\0\".as_ptr() as *const c_char, line);@@@printf(b\"%s\\0\".as_ptr() as *const c_char, line);"
"printline-uses-lossy-utf8@@@printf(b\"%s\\n\\0\".as_ptr() as *const c_char, line);@@@{ let cs = std::ffi::CStr::from_ptr(line); let owned = std::ffi::CString::new(cs.to_string_lossy().as_bytes().to_vec()).unwrap(); printf(b\"%s\\n\\0\".as_ptr() as *const c_char, owned.as_ptr()); }"
"goodb2g-honours-dead-store@@@    data = b' ' as c_char;
    data = CHAR_MAX as c_char;@@@    data = b' ' as c_char;"
"goodb2g-inverts-range-check@@@if (data as c_int) < (CHAR_MAX / 2) {@@@if (data as c_int) > (CHAR_MAX / 2) {"
"goodb2g-takes-accept-branch@@@if (data as c_int) < (CHAR_MAX / 2) {@@@if (data as c_int) <= CHAR_MAX {"
"goodb2g-wrong-message@@@arithmetic safely.@@@arithmetic safely"
"goodg2b-wrong-constant@@@    data = 2;@@@    data = 3;"
"good-swaps-submode-order@@@    goodG2B();
    goodB2G();@@@    goodB2G();
    goodG2B();"
"good-drops-one-submode@@@    goodG2B();
    goodB2G();@@@    goodG2B();"
"bad-uses-safe-constant@@@    data = CHAR_MAX as c_char;
    if data as c_int > 0 {@@@    data = 2;
    if data as c_int > 0 {"
"bad-guard-never-taken@@@    data = CHAR_MAX as c_char;
    if data as c_int > 0 {@@@    data = CHAR_MAX as c_char;
    if data as c_int > CHAR_MAX {"
"bad-saturates-instead-of-wrapping@@@        let result: c_char = ((data as c_int) * 2) as c_char;
        printHexCharLine(result as c_int);@@@        let result: c_char = data.saturating_mul(2);
        printHexCharLine(result as c_int);"
"driver-calls-both-branches@@@    if useGood != 0 {
        good();
    } else {
        bad();
    }@@@    good();
    bad();"
)

pass=0
notcaught=0
skipped=0
printf '%-38s %s\n' "MUTANT" "RESULT"
printf '%-38s %s\n' "$(printf '%.0s-' {1..38})" "------"

for entry in "${MUTANTS[@]}"; do
  name="${entry%%@@@*}"
  rest="${entry#*@@@}"
  from="${rest%%@@@*}"
  to="${rest#*@@@}"

  cp "$BACKUP" "$LIB"
  MUT_FROM="$from" MUT_TO="$to" perl -0777 -i -pe \
    's/\Q$ENV{MUT_FROM}\E/$ENV{MUT_TO}/' "$LIB"

  if cmp -s "$BACKUP" "$LIB"; then
    printf '%-38s %s\n' "$name" "SKIP (pattern not found)"
    skipped=$((skipped + 1))
    continue
  fi
  if ! cargo build $CARGO_FLAGS >/dev/null 2>&1; then
    printf '%-38s %s\n' "$name" "SKIP (mutant does not compile)"
    skipped=$((skipped + 1))
    continue
  fi
  caught_in=""
  if ! cargo test $CARGO_FLAGS -- --test-threads=1 >/dev/null 2>&1; then
    caught_in="debug"
  fi
  if cargo build $CARGO_FLAGS --release >/dev/null 2>&1 &&
    ! cargo test $CARGO_FLAGS --release -- --test-threads=1 >/dev/null 2>&1; then
    caught_in="${caught_in:+$caught_in+}release"
  fi
  if [ -z "$caught_in" ]; then
    printf '%-38s %s\n' "$name" "*** NOT CAUGHT ***"
    notcaught=$((notcaught + 1))
  else
    printf '%-38s %s\n' "$name" "caught ($caught_in)"
    pass=$((pass + 1))
  fi
done

restore
cargo build $CARGO_FLAGS >/dev/null 2>&1
cargo build $CARGO_FLAGS --release >/dev/null 2>&1

echo
echo "caught: $pass   not caught: $notcaught   skipped: $skipped"
if [ "$notcaught" -ne 0 ] || [ "$skipped" -ne 0 ]; then
  echo "FAIL: every mutant must apply and be caught."
  exit 1
fi
echo "OK: all mutants applied and were caught."
