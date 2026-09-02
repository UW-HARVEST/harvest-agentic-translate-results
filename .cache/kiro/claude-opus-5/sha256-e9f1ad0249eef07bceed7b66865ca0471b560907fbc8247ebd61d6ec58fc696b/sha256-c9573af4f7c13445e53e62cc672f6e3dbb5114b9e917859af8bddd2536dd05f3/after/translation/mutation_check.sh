#!/usr/bin/env bash
# Negative control for the differential harness: inject a deliberate bug into
# the Rust translation, rebuild the cdylib, and confirm the test suite FAILS.
# Every mutation must be caught, otherwise the corresponding behaviour is not
# actually being verified.  Restores src/lib.rs unconditionally on exit.
set -uo pipefail
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; cargo build --release >/dev/null 2>&1; }
trap restore EXIT

fail=0
run_mutation() {
  local name="$1" from="$2" to="$3"
  cp "$BAK" "$SRC"
  if ! python3 - "$SRC" "$from" "$to" <<'PY'
import sys
p, a, b = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
if a not in s:
    sys.exit(2)
open(p, "w").write(s.replace(a, b, 1))
PY
  then
    echo "SKIP   $name (pattern not found)"
    fail=1
    return
  fi
  if ! timeout 300 cargo build --release >/dev/null 2>&1; then
    echo "SKIP   $name (mutant does not compile)"
    return
  fi
  if timeout 600 cargo test --release >/dev/null 2>&1; then
    echo "MISSED $name  <-- harness did NOT detect this bug"
    fail=1
  else
    echo "caught $name"
  fi
}

run_mutation "big_values bound 288->289"        "big_values > 288"                       "big_values > 289"
run_mutation "big_values bound 288->287"        "big_values > 288"                       "big_values > 287"
run_mutation "preflag threshold 500->501"       "scalefac_compress >= 500"               "scalefac_compress > 500"
run_mutation "reservoir check > -> >="          "> limit.wrapping_add(main_data_begin.wrapping_mul(8))" ">= limit.wrapping_add(main_data_begin.wrapping_mul(8))"
run_mutation "scfsi mask 0x0F0F->0x0FFF"        "scfsi &= 0x0F0F"                        "scfsi &= 0x0FFF"
run_mutation "n_long_sfb mixed 8/6 swapped"     "if (hdr1 & 0x8) != 0 { 8 } else { 6 }"  "if (hdr1 & 0x8) != 0 { 6 } else { 8 }"
run_mutation "n_short_sfb short 39->38"         "n_short_sfb = 39"                       "n_short_sfb = 38"
run_mutation "region_count[1] 255->254"         "(*gr).region_count[1] = 255"            "(*gr).region_count[1] = 254"
run_mutation "region_count[2] 255->254"         "(*gr).region_count[2] = 255"            "(*gr).region_count[2] = 254"
run_mutation "sr_idx decrement dropped"         "sr_idx -= (sr_idx != 0) as c_int;"      ""
run_mutation "scfsi width 7+gr -> 8+gr"         "get_bits(bs, 7 + gr_count)"             "get_bits(bs, 8 + gr_count)"
run_mutation "mdb width 8+gr -> 9+gr"           "get_bits(bs, 8 + gr_count)"             "get_bits(bs, 9 + gr_count)"
run_mutation "table_select[0] shift 10->11"     "(tables >> 10) as u8"                   "(tables >> 11) as u8"
run_mutation "table_select[1] mask 31->30"      "((tables >> 5) & 31) as u8"             "((tables >> 5) & 30) as u8"
run_mutation "tables <<= 5 -> <<= 4"            "tables.wrapping_shl(5)"                 "tables.wrapping_shl(4)"
run_mutation "scalefac_compress width 4->5"     "if (hdr1 & 0x8) != 0 { 4 } else { 9 }"  "if (hdr1 & 0x8) != 0 { 5 } else { 9 }"
run_mutation "get_bits first-byte mask 255->127" "(255u32 >> s)"                         "(127u32 >> s)"
run_mutation "get_bits early return 0 -> 1"     "return 0;"                              "return 1;"
run_mutation "get_bits limit > -> >="           "if (*bs).pos > (*bs).limit"             "if (*bs).pos >= (*bs).limit"
run_mutation "gr_count mono/stereo swapped"     "if (hdr3 & 0xC0) == 0xC0 { 1 } else { 2 }" "if (hdr3 & 0xC0) == 0xC0 { 2 } else { 1 }"
run_mutation "scfsi output shift 12->11"        "((scfsi >> 12) & 15) as u8"             "((scfsi >> 11) & 15) as u8"
run_mutation "g_scf_long[5] literal 158->157"   "50, 54, 76, 158, 0,"                    "50, 54, 76, 157, 0,"
run_mutation "g_scf_mixed[5] literal 56->57"    "30, 30, 30, 56, 56, 56, 0, 0,"          "30, 30, 30, 56, 56, 57, 0, 0,"
run_mutation "g_scf_short[1] literal 26->25"    "36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0," "36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 25, 26, 26, 0,"
run_mutation "g_scf_mixed[0] tail zero-fill"    "24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0," "24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 1, 0, 0,"
run_mutation "region_count[0] short 8->7"       "(*gr).region_count[0] = 8;"             "(*gr).region_count[0] = 7;"
run_mutation "sfbtab long row stride 23->24"    "offset(sr_idx as isize * 23)"           "offset(sr_idx as isize * 24)"
run_mutation "sfbtab short/mixed stride 40->41" "G_SCF_SHORT.as_ptr() as *const u8).offset(sr_idx as isize * 40)" "G_SCF_SHORT.as_ptr() as *const u8).offset(sr_idx as isize * 41)"
run_mutation "block_type reject 0 -> reject 1"  "if block_type == 0 {"                   "if block_type == 1 {"
run_mutation "subblock_gain width 3->2"         "(*gr).subblock_gain[0] = get_bits(bs, 3) as u8" "(*gr).subblock_gain[0] = get_bits(bs, 2) as u8"

echo
if [ "$fail" -ne 0 ]; then
  echo "RESULT: at least one mutation was NOT caught."
  exit 1
fi
echo "RESULT: every mutation was caught by the differential suite."
