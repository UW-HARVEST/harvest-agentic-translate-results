#!/usr/bin/env bash
# Anti-vacuity check: build deliberately-wrong copies of src/lib.rs, point the
# differential suite at each one via HDR_RUST_SO, and require that every mutant
# is KILLED by at least one phase. A suite that passes against a broken library
# proves nothing, so this is what makes the green run meaningful.
#
# Usage: ./mutation_check.sh            (from translation/)
set -uo pipefail
cd "$(dirname "$0")"

MUT=$(mktemp -d)
trap 'rm -rf "$MUT"' EXIT

# name -> sed expression applied to src/lib.rs
NAMES=(
  m1_bit0_not_ignored
  m2_bitrate_boundary
  m3_drop_mpeg25_class
  m4_nibble_agreement_inverted
  m5_samplerate_mask
  m6_layer_reserved
  m7_no_shortcircuit
)
SEDS=(
  's/(a1 \^ b1) \& 0xFE/(a1 ^ b1) \& 0xFF/'
  's/if (h2 >> 4) == 15 {/if (h2 >> 4) == 14 {/'
  's/(h1 \& 0xF0) == 0xf0 || (h1 \& 0xFE) == 0xe2/(h1 \& 0xF0) == 0xf0/'
  's/if ((a2 \& 0xF0) == 0) != ((b2 \& 0xF0) == 0) {/if ((a2 \& 0xF0) == 0) == ((b2 \& 0xF0) == 0) {/'
  's/(a2 \^ b2) \& 0x0C/(a2 ^ b2) \& 0x04/'
  's/if ((h1 >> 1) \& 3) == 0 {/if ((h1 >> 1) \& 3) == 1 {/'
  's|if !unsafe { hdr_valid(h2) } {|let _pre = unsafe { core::ptr::read_volatile(h1.add(1)) }; if !unsafe { hdr_valid(h2) } {|'
)

FAIL=0
printf '%-32s %s\n' MUTANT 'KILLED BY'
for i in "${!NAMES[@]}"; do
  n=${NAMES[$i]}
  sed "${SEDS[$i]}" src/lib.rs > "$MUT/$n.rs"
  if diff -q src/lib.rs "$MUT/$n.rs" >/dev/null; then
    echo "!! $n: sed matched nothing - mutation script is stale vs src/lib.rs"; FAIL=1; continue
  fi
  if ! rustc --edition 2024 --crate-type cdylib -O -o "$MUT/lib$n.so" "$MUT/$n.rs" 2>"$MUT/$n.err"; then
    echo "!! $n: failed to compile"; head -3 "$MUT/$n.err"; FAIL=1; continue
  fi
  killers=()
  for t in phase_b_exhaustive phase_b_configs phase_c_errors; do
    out=$(HDR_RUST_SO="$MUT/lib$n.so" timeout 600 cargo test --test "$t" 2>&1)
    if echo "$out" | grep -qE 'test result: FAILED|SIGSEGV|SIGBUS|error: test failed'; then
      killers+=("$t")
    fi
  done
  if [[ ${#killers[@]} -eq 0 ]]; then
    printf '%-32s %s\n' "$n" 'SURVIVED  <-- test suite has a blind spot'; FAIL=1
  else
    printf '%-32s %s\n' "$n" "${killers[*]}"
  fi
done

echo
if [[ $FAIL -eq 0 ]]; then echo "all mutants killed - suite is not vacuous"; else echo "MUTATION CHECK FAILED"; fi
exit $FAIL
