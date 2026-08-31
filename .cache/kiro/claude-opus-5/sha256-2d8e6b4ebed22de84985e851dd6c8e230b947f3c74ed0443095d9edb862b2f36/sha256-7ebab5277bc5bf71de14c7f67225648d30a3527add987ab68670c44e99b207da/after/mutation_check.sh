#!/usr/bin/env bash
# Mutation check: each entry perturbs the Rust translation and asserts that the
# differential suite NOTICES. A mutation that survives means the tests have a
# blind spot there.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/translation"

survived=0

run_one() {
    local name="$1" file="$2" from="$3" to="$4" test_target="$5"
    cp "src/$file" /tmp/mut_backup.rs
    if ! python3 - "$from" "$to" "src/$file" <<'PY'
import sys
frm, to, path = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if frm not in s:
    sys.exit(3)
open(path, 'w').write(s.replace(frm, to, 1))
PY
    then
        echo "  SKIP $name (pattern not found in $file)"
        cp /tmp/mut_backup.rs "src/$file"
        return
    fi
    if ! timeout 600 cargo build --release >/tmp/mut_build.log 2>&1; then
        echo "  SKIP $name (does not compile)"
        cp /tmp/mut_backup.rs "src/$file"
        timeout 600 cargo build --release >/dev/null 2>&1
        return
    fi
    if timeout 900 cargo test --release --test "$test_target" -- --test-threads=1 >/tmp/mut_test.log 2>&1; then
        echo "  SURVIVED  $name  (tests/$test_target.rs did not notice)"
        survived=$((survived + 1))
    else
        echo "  caught    $name  (by tests/$test_target.rs)"
    fi
    cp /tmp/mut_backup.rs "src/$file"
    timeout 600 cargo build --release >/dev/null 2>&1
}

echo "mutation check (each line perturbs the Rust and expects a test failure)"

run_one "utf8_encode 3-byte lead byte"      utf.rs      "0xE0 +"                   "0xE1 +"                  level1_low
run_one "utf8_check_first 4-byte range"     utf.rs      "0xF4"                     "0xF3"                    level1_low
run_one "strbuffer min size"                strbuffer.rs "STRBUFFER_MIN_SIZE: usize = 16" "STRBUFFER_MIN_SIZE: usize = 32" level1_low
run_one "hashtable initial order"           hashtable.rs "INITIAL_HASHTABLE_ORDER: usize = 3" "INITIAL_HASHTABLE_ORDER: usize = 4" level2_hashtable
run_one "dtoa pfive[-1] value"              dtoa.rs     "debug_assert_eq!(idx, -1);
        0"                                                                        "debug_assert_eq!(idx, -1);
        1"                        level3_dtoa
run_one "dtoa_divmax initial value"         dtoa.rs     "dtoa_divmax: c_int = 2"   "dtoa_divmax: c_int = 3"  level3_dtoa
run_one "gethex BIG0"                       dtoa_hex.rs "0x7fef_ffff"              "0x7fef_fffe"             gethex
run_one "gethex emax"                       dtoa_hex.rs "0x7fe - BIAS - P + 1"     "0x7fe - BIAS - P + 2"    gethex
run_one "strtod bc.rounding init"           dtoa_strtod.rs "bc.rounding = 1;"      "bc.rounding = 0;"        strtod_unused
# NOTE: perturbing STRTOD_DIGLIM is an *equivalent* mutant -- it only chooses
# between two paths that are both correctly rounded, so it is deliberately not
# used here. Neutering bigcomp() instead proves the bigcomp path is reached.
run_one "strtod bigcomp neutered"           dtoa_strtod.rs "unsafe fn bigcomp(rv: &mut f64, s0: *const u8, bc: &mut BCinfo) {" "unsafe fn bigcomp(rv: &mut f64, s0: *const u8, bc: &mut BCinfo) { return;" strtod_unused
run_one "strtod sulp returns ulp only"       dtoa_strtod.rs "fn sulp(x: f64, bc: &BCinfo) -> f64 {" "fn sulp(x: f64, bc: &BCinfo) -> f64 { return ulp(x);" strtod_unused
run_one "dump MAX_INTEGER_STR_LENGTH"       dump.rs     "MAX_INTEGER_STR_LENGTH: usize = 25" "MAX_INTEGER_STR_LENGTH: usize = 12" level5_dump
run_one "dump MAX_REAL_STR_LENGTH"          dump.rs     "MAX_REAL_STR_LENGTH: usize = 25"    "MAX_REAL_STR_LENGTH: usize = 15"    level5_dump
run_one "parser max depth"                  types.rs    "JSON_PARSER_MAX_DEPTH: usize = 2048" "JSON_PARSER_MAX_DEPTH: usize = 64" level6_load
run_one "value json_equal always 1"         value.rs    "fn json_equal(json1: *const JsonT, json2: *const JsonT) -> c_int {" "fn json_equal(json1: *const JsonT, json2: *const JsonT) -> c_int { return 1;" level4_value
run_one "object update_missing == update"    value.rs    "fn json_object_update_missing(" "fn json_object_update_missing_MUT(" level4_value

echo
if [ "$survived" -ne 0 ]; then
    echo "$survived mutation(s) SURVIVED -- test coverage gap"
    exit 1
fi
echo "all mutations caught"
