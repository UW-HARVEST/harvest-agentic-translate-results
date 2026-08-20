#!/usr/bin/env bash
# Sanity-check the differential suite itself: deliberately break the Rust
# translation in small ways and confirm the tests CATCH each one. A suite that
# cannot fail proves nothing. src/lib.rs is restored afterwards.
set -uo pipefail
cd "$(dirname "$0")"
TMP="${TMPDIR:-/tmp}"; TMP="${TMP%/}"

cp src/lib.rs "$TMP/lib.rs.pristine"
restore() { cp "$TMP/lib.rs.pristine" src/lib.rs; }
trap restore EXIT

# name | sed expression
mutations=(
  "matrixsum multiplier 0x10 -> 0x11|s/let hex_multiplier: c_int = 0x10;/let hex_multiplier: c_int = 0x11;/"
  "matrixsum flag weight 0xFF -> 0xFE|s/let hex_base: c_int = 0xFF;/let hex_base: c_int = 0xFE;/"
  "matrixsum mask 0xFFF -> 0xFFFF|s/matrix_sum \& 0xFFF)/matrix_sum \& 0xFFFF)/"
  "add_element grow test >= -> >|s/if (\*arr).size >= (\*arr).capacity {/if (*arr).size > (*arr).capacity {/"
  "expand_array doubling 2 -> 3|s/capacity.wrapping_mul(2)/capacity.wrapping_mul(3)/"
  "init_array checked instead of wrapping mul|s/initial_capacity.wrapping_mul(SIZEOF_INT)/initial_capacity.checked_mul(SIZEOF_INT).unwrap_or(usize::MAX)/"
  "process_flags drops FLAG_DELETE|s/.wrapping_add(delete_enabled)//"
  "checksum loop 3x4 -> 3x3|s/for j in 0..4usize {/for j in 0..3usize {/"
  "matrix default 0xD4 -> 0xD5|s/0xA1, 0xB2, 0xC3, 0xD4/0xA1, 0xB2, 0xC3, 0xD5/"
  "matrix default first element 0x01 -> 0x02|s/\[0x01, 0x02, 0x03, 0x04\]/[0x02, 0x02, 0x03, 0x04]/"
  "PERL:null guards return 1 instead of 0|s/if arr\\.is_null\\(\\) \\{\\n            return 0;\\n        \\}/if arr.is_null() { return 1; }/g"
  "PERL:free_array drops its NULL guard|s/if !arr\\.is_null\\(\\) \\{/if true {/"
)

caught=0; missed=0
for m in "${mutations[@]}"; do
  name="${m%%|*}"; expr="${m#*|}"
  restore
  if [[ "$name" == PERL:* ]]; then
    name="${name#PERL:}"
    if ! perl -0pi -e "$expr" src/lib.rs 2>/dev/null; then
      echo "SKIP (perl failed): $name"; continue
    fi
  elif ! sed -i "$expr" src/lib.rs 2>/dev/null; then
    echo "SKIP (sed failed): $name"; continue
  fi
  if cmp -s src/lib.rs "$TMP/lib.rs.pristine"; then
    echo "SKIP (mutation did not apply): $name"; continue
  fi
  # Rebuild the cdylib, then run the differential suites.
  if ! timeout 300 cargo build --offline --no-default-features >/dev/null 2>&1; then
    echo "CAUGHT (build error): $name"; caught=$((caught+1)); continue
  fi
  out=$(timeout 600 cargo test --offline --no-default-features 2>&1)
  if echo "$out" | grep -qE "FAILED|error: test failed|signal: |SIGABRT|SIGSEGV|panicked"; then
    n=$(echo "$out" | grep -cE "^test .* FAILED")
    extra=""
    echo "$out" | grep -qE "signal: |SIGABRT|SIGSEGV" && extra=" +abort/signal"
    echo "CAUGHT ($n failing tests$extra): $name"; caught=$((caught+1))
  else
    echo "*** MISSED (suite still green!): $name"; missed=$((missed+1))
  fi
done

restore
timeout 300 cargo build --offline --no-default-features >/dev/null 2>&1
echo
echo "mutations caught: $caught   MISSED: $missed"
if cmp -s src/lib.rs "$TMP/lib.rs.pristine"; then echo "src/lib.rs restored OK"; else echo "!!! src/lib.rs NOT restored"; exit 1; fi
[ "$missed" -eq 0 ]
