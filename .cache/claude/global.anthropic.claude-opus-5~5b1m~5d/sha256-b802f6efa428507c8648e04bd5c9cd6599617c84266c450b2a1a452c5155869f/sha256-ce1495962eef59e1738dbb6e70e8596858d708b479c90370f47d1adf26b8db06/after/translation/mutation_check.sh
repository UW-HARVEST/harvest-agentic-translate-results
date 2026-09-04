#!/usr/bin/env bash
# Sanity check that the differential suite is NOT vacuous: inject a series of
# small faults into src/lib.rs and confirm each one is detected.
#
# A mutant is "killed" if `cargo test` exits non-zero (a failed assertion, or the
# test binary being killed by a signal, both count).
#
# Usage:  cd translation && ./mutation_check.sh
set -uo pipefail

BAK=$(mktemp)
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; timeout 600 cargo build >/dev/null 2>&1; rm -f "$BAK"; }
trap restore EXIT

KILLED=0; SURVIVED=0; SKIPPED=0

mutant() {
  local name="$1" expr="$2" note="${3:-}"
  cp "$BAK" src/lib.rs
  sed -i "$expr" src/lib.rs
  if cmp -s src/lib.rs "$BAK"; then
    printf '  SKIP     %-34s (sed did not apply)\n' "$name"; SKIPPED=$((SKIPPED+1)); return
  fi
  if ! timeout 600 cargo build >/dev/null 2>&1; then
    printf '  SKIP     %-34s (does not compile)\n' "$name"; SKIPPED=$((SKIPPED+1)); return
  fi
  timeout 600 cargo test >/dev/null 2>&1
  if [ $? -ne 0 ]; then
    printf '  KILLED   %-34s\n' "$name"; KILLED=$((KILLED+1))
  else
    printf '  SURVIVED %-34s %s\n' "$name" "$note"; SURVIVED=$((SURVIVED+1))
  fi
}

echo "Injecting faults into src/lib.rs:"
# --- fma_array arithmetic
mutant "saturating instead of wrapping mul" 's/\.wrapping_mul(/.saturating_mul(/'
mutant "square mul1 instead of mul1*mul2"   's/\.wrapping_mul(\*mul2.offset(idx))/.wrapping_mul(*mul1.offset(idx))/'
mutant "off-by-one result"                  's/\*out.offset(idx) = v;/*out.offset(idx) = v.wrapping_add(1);/'
mutant "drop the addend"                    's/\.wrapping_add(\*add.offset(idx))/.wrapping_add(0)/'
# --- loop bounds
mutant "off-by-one loop bound (i <= len)"   's/while i <= len {/XX/; s/while i < len {/while i <= len {/'
mutant "loop stops one early"               's/while i < len {/while i < len - 1 {/'
# --- driver / inner
mutant "printf format loses the newline"    "s/b'\\\\n' as c_char, 0\\]/0, 0]/"
mutant "printf format uses %u"              "s/b'd' as c_char/b'u' as c_char/"
mutant "memcpy one element short"           's/n \* std::mem::size_of::<c_int>()/n.saturating_sub(1) * std::mem::size_of::<c_int>()/'
mutant "no clamp on negative len"           's/let n = if len > 0 { len as usize } else { 0 };/let n = len as usize;/'

echo
echo "killed=$KILLED survived=$SURVIVED skipped=$SKIPPED"
if [ "$SURVIVED" -ne 0 ]; then
  echo "WARNING: a surviving mutant means the suite has a blind spot (or the mutant is"
  echo "         semantically equivalent -- inspect it before dismissing it)."
  exit 1
fi
echo "OK: every injected fault was detected."
