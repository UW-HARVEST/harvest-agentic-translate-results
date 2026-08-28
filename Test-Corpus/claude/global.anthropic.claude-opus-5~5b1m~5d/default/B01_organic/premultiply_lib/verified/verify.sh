#!/usr/bin/env bash
# Full verification driver: builds the C reference .so, then runs the entire
# differential suite for every feature combination and every profile.
#
#   ./verify.sh            # everything
#   ./verify.sh --quick    # release profile only
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
CSRC="$ROOT/../c_src"
CARGO_FLAGS="--offline"
QUICK=0
[ "${1:-}" = "--quick" ] && QUICK=1

fail=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad()  { printf '\033[31mFAIL: %s\033[0m\n' "$*"; fail=1; }
good() { printf '\033[32mok: %s\033[0m\n' "$*"; }

# --------------------------------------------------------------- C reference
note "Building the C reference shared library"
mkdir -p "$CSRC/build"
( cd "$CSRC/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(ls "$CSRC"/build/lib*.so 2>/dev/null | head -1)"
[ -f "$C_SO" ] || { bad "no C .so produced"; exit 1; }
good "C .so = $C_SO"

# ------------------------------------------------- enumerate feature combos
# Mechanically read the [features] table; if absent the only combo is default.
mapfile -t FEATURES < <(
  awk '
    /^\[/                { in_f = ($0 ~ /^\[features\]/) ; next }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); print
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  note "Cargo.toml declares no [features] -> single configuration"
  COMBOS+=("<default>")
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
else
  note "Features found: ${FEATURES[*]} -> testing the full power set"
  COMBOS+=("<default>")
  COMBOS+=("--no-default-features")
  n=${#FEATURES[@]}
  total=$(( 1 << n ))
  for (( mask = 0; mask < total; mask++ )); do
    sel=()
    for (( i = 0; i < n; i++ )); do
      (( mask & (1 << i) )) && sel+=("${FEATURES[$i]}")
    done
    if [ "${#sel[@]}" -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  COMBOS+=("--all-features")
fi

# Deduplicate.
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

PROFILES=(release)
[ "$QUICK" -eq 0 ] && PROFILES=(release debug)

# ------------------------------------------------------------------- run it
for profile in "${PROFILES[@]}"; do
  pflag=""; [ "$profile" = release ] && pflag="--release"
  for combo in "${COMBOS[@]}"; do
    cflag="$combo"; [ "$combo" = "<default>" ] && cflag=""
    label="profile=$profile features=$combo"

    note "$label"
    # The cdylib must exist before the tests dlopen it.
    if ! timeout 600 cargo build $pflag $CARGO_FLAGS $cflag >/dev/null 2>&1; then
      bad "build ($label)"; continue
    fi
    if timeout 600 cargo test $pflag $CARGO_FLAGS $cflag 2>&1 | tee "$ROOT/.verify.log" \
         | grep -E '^test result:'; then
      if grep -qE '^test result: FAILED|error:' "$ROOT/.verify.log"; then
        bad "tests ($label)"
        grep -E '^test .* FAILED|panicked at|signal:' "$ROOT/.verify.log" | head -20
      else
        good "$label"
      fi
    else
      bad "tests produced no result line ($label)"
      tail -30 "$ROOT/.verify.log"
    fi
  done
done

# --------------------------------------------------------- symbol diff gate
note "Symbol diff gate (nm -D)"
RUST_SO="$ROOT/target/release/libpremultiply_lib.so"
if [ ! -f "$RUST_SO" ]; then
  timeout 600 cargo build --release $CARGO_FLAGS >/dev/null 2>&1
fi
c_syms=$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u)
r_syms=$(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u)
missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
echo "C exports:"; echo "$c_syms" | sed 's/^/  /'
if [ -n "$missing" ]; then
  bad "symbols missing from the Rust .so:"; echo "$missing" | sed 's/^/  /'
else
  good "symbol diff is EMPTY (0 missing)"
fi

rm -f "$ROOT/.verify.log"
note "RESULT"
if [ "$fail" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$fail"
