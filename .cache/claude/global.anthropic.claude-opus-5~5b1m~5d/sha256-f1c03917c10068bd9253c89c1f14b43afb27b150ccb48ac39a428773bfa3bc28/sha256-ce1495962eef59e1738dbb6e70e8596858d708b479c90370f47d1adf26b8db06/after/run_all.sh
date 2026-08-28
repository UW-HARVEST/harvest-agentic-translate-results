#!/bin/bash
# Full verification run: build both libraries, check symbol parity, then run the
# whole differential suite against every feature combination AND every build
# profile of the Rust cdylib.
#
# Nothing in c_src/ is modified.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
C_BUILD="$ROOT/c_src/build"
CRATE="$ROOT/translation"
rc=0
step() { printf '\n\033[1m=== %s\033[0m\n' "$*"; }

# ---------------------------------------------------------------- C library ---
step "Building the C shared library"
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) | tail -2 || { echo "C build FAILED"; exit 1; }
CSO="$C_BUILD/libdriver.so"
ls -l "$CSO"

# ------------------------------------------------- feature-combo enumeration ---
# Enumerate the powerset of the features declared in Cargo.toml. This crate
# declares none, so the matrix is {default, --no-default-features}; the loop is
# written generically so it stays correct if features are ever added.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
         sub(/[[:space:]]*=.*/,""); if ($0 != "default") print }' "$CRATE/Cargo.toml"
)
COMBOS=("" "--no-default-features")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((m=1; m<(1<<n); m++)); do
    set=""
    for ((b=0; b<n; b++)); do
      (( m & (1<<b) )) && set="${set:+$set,}${FEATURES[$b]}"
    done
    COMBOS+=("--no-default-features --features $set")
  done
fi
echo "feature combinations to verify: ${#COMBOS[@]}"
printf '  [%s]\n' "${COMBOS[@]/#/cargo test }"

# ----------------------------------------------------------------- the matrix ---
for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    label="features='${combo:-<default>}' profile=$profile"
    step "$label"

    if [ "$profile" = release ]; then
      ( cd "$CRATE" && cargo build --release $combo -q ) || { rc=1; continue; }
      RSO="$CRATE/target/release/libdriver.so"
    else
      ( cd "$CRATE" && cargo build $combo -q ) || { rc=1; continue; }
      RSO="$CRATE/target/debug/libdriver.so"
    fi

    # Symbol parity: every symbol the C .so exports must exist in the Rust .so.
    if command -v nm >/dev/null; then
      nm -D --defined-only "$CSO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort -u > "${TMPDIR:-/tmp}/c.syms"
      nm -D --defined-only "$RSO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort -u > "${TMPDIR:-/tmp}/r.syms"
      missing=$(comm -23 "${TMPDIR:-/tmp}/c.syms" "${TMPDIR:-/tmp}/r.syms")
      if [ -n "$missing" ]; then
        echo "SYMBOL PARITY FAILED -- missing from the Rust .so:"; echo "$missing"; rc=1
      else
        echo "symbol parity OK ($(wc -l < "${TMPDIR:-/tmp}/c.syms") C symbols, 0 missing)"
      fi
    fi

    ( cd "$CRATE" && C_SO="$CSO" RUST_SO="$RSO" timeout 600 cargo test $combo 2>&1 ) \
      | grep -E "^(test |running |test result|error|warning: unused)" | tail -60
    # shellcheck disable=SC2181
    if [ "${PIPESTATUS[0]}" -ne 0 ]; then echo "TESTS FAILED for $label"; rc=1; fi
  done
done

# ------------------------------------------------------------ negative control ---
step "Negative control (mutation testing)"
"$ROOT/mutants.sh" | tail -20 || rc=1

step "RESULT"
[ "$rc" -eq 0 ] && echo "ALL CHECKS PASSED" || echo "FAILURES PRESENT"
exit "$rc"
