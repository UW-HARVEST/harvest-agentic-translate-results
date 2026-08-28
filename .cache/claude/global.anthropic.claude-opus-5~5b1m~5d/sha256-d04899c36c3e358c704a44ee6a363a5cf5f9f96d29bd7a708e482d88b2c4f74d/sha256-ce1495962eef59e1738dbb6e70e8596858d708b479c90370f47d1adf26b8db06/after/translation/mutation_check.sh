#!/usr/bin/env bash
# Validate that the differential suite actually DETECTS divergence: inject a
# series of small bugs into src/lib.rs, one at a time, and confirm the tests
# fail. Any mutation that survives is a blind spot in the test suite.
#
# Fields are separated by '@@' (not '|', which occurs inside the code).
# Restores src/lib.rs unconditionally on exit.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here" || exit 2

orig="$here/target/lib.rs.orig"
cp src/lib.rs "$orig"
restore() { cp "$orig" src/lib.rs; }
trap 'restore' EXIT

mutations=(
"valid_1 mask 0x80 -> 0xC0@@at(x, 0) & 0x80 == 0@@at(x, 0) & 0xC0 == 0"
"valid_2 lead mask 0xE0 -> 0xF0@@(b0 & 0xE0) == 0xC0@@(b0 & 0xF0) == 0xC0"
"valid_2 lower bound 0xC2 -> 0xC1@@(0xC2u8 as i8)@@(0xC1u8 as i8)"
"valid_2 continuation 0x80 -> 0xC0@@(at(x, 1) & 0xC0) == 0x80@@(at(x, 1) & 0xC0) == 0xC0"
"valid_3 lead mask 0xF0 -> 0xE0@@(b0 & 0xF0) != 0xE0@@(b0 & 0xE0) != 0xE0"
"valid_3 reads x[1] instead of x[2]@@(at(x, 2) & 0xC0) == 0x80@@(at(x, 1) & 0xC0) == 0x80"
"valid_3 E0 guard >= 0xA0 -> > 0xA0@@b1 >= 0xA0)@@b1 > 0xA0)"
"valid_3 ED guard < 0xA0 -> <= 0xA0@@b1 < 0xA0)@@b1 <= 0xA0)"
"valid_3 EF guard <= 0xBF -> < 0xBF@@b1 <= 0xBF)@@b1 < 0xBF)"
"valid_4 drop the b0 <= 0xF4 guard@@|| b0 > 0xF4@@|| false"
"valid_4 upper bound 0xF4 -> 0xF5@@b0 > 0xF4@@b0 > 0xF5"
"valid_4 reads x[2] instead of x[3]@@(at(x, 3) & 0xC0) == 0x80@@(at(x, 2) & 0xC0) == 0x80"
"valid_4 F0 guard >= 0x90 -> > 0x90@@b1 >= 0x90)@@b1 > 0x90)"
"valid_4 F4 guard <= 0x8F -> < 0x8F@@b1 <= 0x8F)@@b1 < 0x8F)"
"drop advances 2 -> 1 for valid_2@@string = string.add(2);@@string = string.add(1);"
"drop advances 4 -> 3 for valid_4@@string = string.add(4);@@string = string.add(3);"
"REPLACEMENT_INC 4096 -> 2048@@const REPLACEMENT_INC: usize = 4096;@@const REPLACEMENT_INC: usize = 2048;"
"repl < 3 -> repl < 4@@if repl < 3 {@@if repl < 4 {"
"repl -= 3 -> repl -= 6@@repl.wrapping_sub(3)@@repl.wrapping_sub(6)"
"initial size strlen+1 -> strlen+2@@strlen(string) + 1@@strlen(string) + 2"
"U+FFFD first byte 0xEF -> 0xEE@@0xEFu8 as c_char@@0xEEu8 as c_char"
"U+FFFD third byte 0xBD -> 0xBC@@0xBDu8 as c_char@@0xBCu8 as c_char"
"filter copies 3 bytes instead of 4@@for _ in 0..4 {@@for _ in 0..3 {"
"memcpy prefix length off by one@@let mut i: usize = valid.offset_from(string) as usize;@@let mut i: usize = (valid.offset_from(string) as usize).saturating_sub(1);"
"skip the strdup fast path@@if *valid == 0 {@@if false && *valid == 0 {"
"bool test != 0 -> & 1 != 0@@let replacement = replacement != 0;@@let replacement = replacement & 1 != 0;"
"assert line 40 -> 41@@assert_fail_string_not_null(40,@@assert_fail_string_not_null(41,"
"assert function name@@c\"w_utf8_drop\"@@c\"w_utf8_dropx\""
"assert message text@@c\"string != NULL\"@@c\"string == NULL\""
"null check removed in filter@@if string.is_null() {\n        assert_fail_string_not_null(60,@@if false {\n        assert_fail_string_not_null(60,"
# these two are *content-equivalent* — only the guard-page row (E32) can see
# them, because they only change WHICH BYTES ARE READ, not the verdict
"valid_3 reads x[2] before checking x[1] (over-read)@@(b1 & 0xC0) == 0x80\n            && (at(x, 2) & 0xC0) == 0x80@@(at(x, 2) & 0xC0) == 0x80\n            && (b1 & 0xC0) == 0x80"
"valid_4 reads x[3] before checking x[1] (over-read)@@(b1 & 0xC0) == 0x80\n            && (at(x, 2) & 0xC0) == 0x80\n            && (at(x, 3) & 0xC0) == 0x80@@(at(x, 3) & 0xC0) == 0x80\n            && (at(x, 2) & 0xC0) == 0x80\n            && (b1 & 0xC0) == 0x80"
)

survivors=0
n=0
for m in "${mutations[@]}"; do
    desc="${m%%@@*}"; rest="${m#*@@}"; from="${rest%%@@*}"; to="${rest##*@@}"
    n=$((n+1))
    restore
    if ! FROM="$from" TO="$to" python3 - <<'PY'
import os, sys
frm = os.environ['FROM'].replace('\\n', '\n')
to  = os.environ['TO'].replace('\\n', '\n')
p = 'src/lib.rs'
s = open(p).read()
if frm not in s:
    sys.exit(3)
open(p, 'w').write(s.replace(frm, to, 1))
PY
    then
        echo "!! [$n] PATTERN NOT FOUND for '$desc' -- fix the script"
        survivors=$((survivors+1)); continue
    fi
    if ! cargo build --offline           >/dev/null 2>&1 || \
       ! cargo build --offline --release >/dev/null 2>&1; then
        echo "?? [$n] $desc: DOES NOT COMPILE -- fix the script"
        survivors=$((survivors+1)); continue
    fi
    out="$(cargo test --offline 2>&1)"; rc=$?
    if [ "$rc" -ne 0 ] || printf '%s' "$out" | grep -qE '^test result: FAILED'; then
        names="$(printf '%s' "$out" | grep -E '^    [a-z0-9_]+$' | tr -d ' ' | sort -u | head -4 | tr '\n' ' ')"
        crash="$(printf '%s' "$out" | grep -oE 'SIGABRT|SIGSEGV|corrupted [a-z ]+' | head -1)"
        echo "-- [$n] DETECTED  $desc   (rc=$rc ${crash:+crash=$crash }e.g. $names)"
    else
        echo "!! [$n] SURVIVED  $desc   <-- BLIND SPOT"
        survivors=$((survivors+1))
    fi
done
restore
cargo build --offline >/dev/null 2>&1
cargo build --offline --release >/dev/null 2>&1
echo
echo "mutations: $n, survivors: $survivors"
if [ "$survivors" -eq 0 ]; then echo "MUTATION CHECK: OK"; else echo "MUTATION CHECK: BLIND SPOTS FOUND"; fi
exit "$survivors"
