#!/usr/bin/env bash
# Mutation check: proves the differential suite is NOT vacuous.
#
# For each mutant, a deliberately-wrong copy of src/lib.rs is built into its own
# cdylib and the suite is pointed at it with RUST_LIB_PATH. A mutant that the
# suite fails to catch means the suite has a blind spot.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$PWD"
WORK="$ROOT/target/mutants"
rm -rf "$WORK"; mkdir -p "$WORK"

# The tests themselves are already built; reuse the existing test binaries.
cargo build --offline >/dev/null 2>&1
cargo test --offline --no-run >/dev/null 2>&1 || { echo "FATAL: cannot build tests"; exit 1; }

# name|sed-expression   (applied to a copy of src/lib.rs)
MUTANTS=(
  'float_saturating_cast|s|^        c_int::MIN$|        truncated as c_int|'
  'status_bit_offset|s|bitfield!(status, set_status, 11, 5)|bitfield!(status, set_status, 12, 5)|'
  'counter_mask_width|s|bitfield!(counter, set_counter, 3, 5)|bitfield!(counter, set_counter, 3, 4)|'
  'mode_initial_value|s|flags.set_mode(3);|flags.set_mode(2);|'
  'printf_text|s|Debug: param1 = %d|Debug: param_1 = %d|'
  'uint_mask|s|(state_ref.data.uint_val() \& 0xFF)|(state_ref.data.uint_val() \& 0x7F)|'
  'bytes_sum_unsigned|s|result = (bytes\[0\] as c_int).wrapping_add(bytes\[1\] as c_int);|result = (bytes[0] as u8 as c_int).wrapping_add(bytes[1] as u8 as c_int);|'
  'process_buffer_offbyone|s|let consumed = (found as usize - ptr_cur as usize) + 1;|let consumed = found as usize - ptr_cur as usize;|'
  'capacity_zero_extend|s|malloc(capacity as isize as usize)|malloc(capacity as u32 as usize)|'
  'confuse_default_case|s|^        _ => {}$|        _ => { result = 1; }|'
)

CAUGHT=0; MISSED=0
declare -a REPORT=()

for spec in "${MUTANTS[@]}"; do
  name="${spec%%|*}"
  expr="${spec#*|}"
  d="$WORK/$name"
  mkdir -p "$d/src"
  cp "$ROOT/Cargo.toml" "$d/Cargo.toml"
  sed -i 's/^name = "translation"/name = "mutant"/' "$d/Cargo.toml"
  sed -i '/^\[dev-dependencies\]/,+1d' "$d/Cargo.toml"
  sed "$expr" "$ROOT/src/lib.rs" > "$d/src/lib.rs"

  if cmp -s "$ROOT/src/lib.rs" "$d/src/lib.rs"; then
    REPORT+=("ERROR   $name: sed expression did not change anything")
    MISSED=$((MISSED+1)); continue
  fi

  if ! ( cd "$d" && cargo build --offline --release >"$d/build.log" 2>&1 ); then
    REPORT+=("ERROR   $name: mutant did not compile (see $d/build.log)")
    MISSED=$((MISSED+1)); continue
  fi
  so="$d/target/release/libconfusion_lib.so"
  [ -f "$so" ] || so=$(ls "$d"/target/release/*.so | head -1)

  # Run the suite against the mutant. It MUST fail.
  if RUST_LIB_PATH="$so" timeout 600 cargo test --offline >"$d/test.log" 2>&1; then
    REPORT+=("MISSED  $name  <-- suite did not detect this bug!")
    MISSED=$((MISSED+1))
  else
    n=$(grep -cE '^test .* FAILED' "$d/test.log")
    first=$(grep -oE '^test [a-z0-9_]+ \.\.\. FAILED' "$d/test.log" | head -3 | sed 's/^test //;s/ \.\.\. FAILED//' | paste -sd, -)
    REPORT+=("CAUGHT  $name  ($n failing tests: $first)")
    CAUGHT=$((CAUGHT+1))
  fi
done

echo
echo "================= mutation check ================="
for r in "${REPORT[@]}"; do echo "  $r"; done
echo "  caught: $CAUGHT   missed/error: $MISSED"
[ "$MISSED" -eq 0 ] || { echo "  SUITE HAS BLIND SPOTS"; exit 1; }
echo "  every injected bug was detected"
