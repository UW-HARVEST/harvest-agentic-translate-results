#!/usr/bin/env bash
# Differential-verification driver.
#
#   1. builds the C shared library (default CMake configuration)
#   2. enumerates every valid feature combination from Cargo.toml
#   3. `cargo check` + `cargo test` each combination, in dev and release
#   4. compares the dynamically exported symbols of the two .so files
#
# Run from the `translation/` directory:  ./verify.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
TIMEOUT=600
rc=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
fail() { printf '  \033[31mFAIL\033[0m %s\n' "$*"; rc=1; }
ok()   { printf '  \033[32mok\033[0m   %s\n' "$*"; }

# --------------------------------------------------------------------------
step "Building the C shared library"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
  timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  timeout $TIMEOUT cmake --build . >/dev/null
) || { fail "C build"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
[[ -f "$C_SO" ]] || { fail "missing $C_SO"; exit 1; }
ok "$C_SO"

# --------------------------------------------------------------------------
# Enumerate feature combinations: the powerset of the declared features.
# `cargo metadata` is authoritative (it also reports an empty set when the
# crate declares no [features] at all).
step "Enumerating feature combinations"
FEATURES=$(
  awk '
    /^\[features\]/       { inf=1; next }
    /^\[/                 { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$HERE/Cargo.toml"
)
FEATURE_ARR=()
while IFS= read -r f; do [[ -n "$f" ]] && FEATURE_ARR+=("$f"); done <<< "$FEATURES"

COMBOS=()
n=${#FEATURE_ARR[@]}
if (( n == 0 )); then
  echo "  crate declares no cargo features -> single configuration"
  COMBOS=("")
else
  for (( mask=0; mask<(1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then
        combo="${combo:+$combo,}${FEATURE_ARR[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
for c in "${COMBOS[@]}"; do echo "  combo: '${c:-<none>}'"; done

# --------------------------------------------------------------------------
run_cargo() {           # run_cargo <label> <cargo args...>
  local label="$1"; shift
  local log
  log="$(mktemp)"
  if timeout $TIMEOUT cargo "$@" >"$log" 2>&1; then
    ok "$label"
  else
    fail "$label"
    tail -n 40 "$log" | sed 's/^/      /'
  fi
  rm -f "$log"
}

cd "$HERE"

step "cargo check, every feature combination"
for combo in "${COMBOS[@]}"; do
  run_cargo "check --no-default-features --features '${combo:-<none>}'" \
    check --no-default-features --features "$combo" --all-targets
done
run_cargo "check (default features)" check --all-targets

step "cargo test, every feature combination x profile"
# Build the cdylib for both profiles up front so the symbol diff below has
# something to look at for each of them.
run_cargo "build cdylib (dev)" build
run_cargo "build cdylib (release)" build --release
for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    args=(test --no-default-features --features "$combo")
    [[ $profile == release ]] && args+=(--release)
    run_cargo "test --features '${combo:-<none>}' ($profile)" "${args[@]}"
  done
done
for profile in dev release; do
  args=(test)
  [[ $profile == release ]] && args+=(--release)
  run_cargo "test (default features, $profile)" "${args[@]}"
done

# --------------------------------------------------------------------------
step "Exported dynamic symbols: C vs Rust"
syms() { nm -D --defined-only "$1" | awk '$2 ~ /^[TtDdBbRrWi]$/ {print $3}' | sort -u; }
for profile in debug release; do
  RUST_SO="$HERE/target/$profile/libdriver.so"
  [[ -f "$RUST_SO" ]] || continue
  missing=$(comm -23 <(syms "$C_SO") <(syms "$RUST_SO") |
            grep -vE '^(_init|_fini|__bss_start|_edata|_end|__gmon_start__|_ITM_(de)?registerTMCloneTable|__cxa_finalize)$')
  if [[ -z "$missing" ]]; then
    ok "target/$profile: exports every C symbol"
  else
    fail "target/$profile is missing: $(echo "$missing" | tr '\n' ' ')"
  fi
done

printf '\n'
if (( rc == 0 )); then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mFAILURES PRESENT\033[0m\n'
fi
exit $rc
