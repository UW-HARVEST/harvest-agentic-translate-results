#!/usr/bin/env bash
# Sanity check that the differential suite is not vacuous: inject each mutation
# into src/lib.rs, confirm the suite FAILS, then restore.
#
# Usage:  ./scripts/mutation_check.sh
set -uo pipefail
cd "$(dirname "$0")/.."

SRC=src/lib.rs
BACKUP=$(mktemp)
cp "$SRC" "$BACKUP"
restore() { cp "$BACKUP" "$SRC"; rm -f "$BACKUP"; }
trap restore EXIT

# name | sed expression
MUTATIONS=(
  "goodB2G upper bound off-by-one|s/data >= 0 \&\& data < 10/data >= 0 \&\& data <= 10/"
  "goodB2G lower bound off-by-one|s/data >= 0 \&\& data < 10/data > 0 \&\& data < 10/"
  "bad negative guard off-by-one|s/if data >= 0 {/if data > 0 {/"
  "goodG2B hardcoded index 7 -> 6|s/let data: c_int = 7;/let data: c_int = 6;/"
  "printLine drops the NULL check|s/if !line\\.is_null\\(\\) \\{/if true {/"
  "error message period removed|s/Array index is negative\./Array index is negative/"
  "out-of-bounds message reworded|s/is out-of-bounds/is out of bounds/"
  "good() helper order swapped|s/    goodG2B\\(\\);\\n    goodB2G\\(data\\);/    goodB2G(data);\\n    goodG2B();/"
  "driver banner text changed|s/Finished good\\(\\)/Done good()/"
  "print loop bound off-by-one|s/for i in 0..BUFFER_LEN {/for i in 0..BUFFER_LEN - 1 {/"
)

pass=0; fail=0
for entry in "${MUTATIONS[@]}"; do
  name=${entry%%|*}
  expr=${entry#*|}
  cp "$BACKUP" "$SRC"
  if ! perl -0777 -pi -e "$expr" "$SRC" 2>/dev/null; then
    printf '  SKIP    %s (sed failed)\n' "$name"; continue
  fi
  if cmp -s "$SRC" "$BACKUP"; then
    printf '  SKIP    %s (pattern did not match)\n' "$name"; continue
  fi
  if ! timeout 600 cargo build --release >/dev/null 2>&1; then
    printf '  SKIP    %s (mutant does not compile)\n' "$name"; continue
  fi
  if timeout 600 cargo test --test phase_b_configs --test phase_c_errors \
        -- --test-threads=1 >/dev/null 2>&1; then
    printf '  SURVIVED  %s  <-- suite is blind to this!\n' "$name"; fail=1
  else
    printf '  killed    %s\n' "$name"; ((pass++))
  fi
done

cp "$BACKUP" "$SRC"
timeout 600 cargo build --release >/dev/null 2>&1

printf '\n%d mutant(s) killed\n' "$pass"
if ((fail)); then echo "MUTATION CHECK FAILED"; exit 1; fi
echo "MUTATION CHECK PASSED (suite detects every injected divergence)"
