#!/usr/bin/env bash
# Full verification driver: builds both libraries and runs every phase across
# every feature combination declared in Cargo.toml.
#
# Exists because `cargo test` alone is NOT sufficient here: the lib target is
# `crate-type = ["cdylib"]` only, so there is no rlib for the integration tests
# to link and cargo therefore SKIPS rebuilding the cdylib. A bare `cargo test`
# after a source edit silently tests the previous `.so`. The harness has a
# staleness guard for that, and this script always builds first.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
TIMEOUT=600
fail=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

# --------------------------------------------------------------------------
step "Build the C shared library"
# --------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
if ( cd "$ROOT/c_src/build" \
     && timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
     && timeout $TIMEOUT cmake --build . ) > /tmp/hsl_c_build.log 2>&1; then
  ok "cmake build"
else
  bad "cmake build (see /tmp/hsl_c_build.log)"; tail -20 /tmp/hsl_c_build.log; exit 1
fi

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' -type f | sort | head -1)"
[ -n "$C_SO" ] && ok "C .so: $C_SO" || { bad "no C .so produced"; exit 1; }

# --------------------------------------------------------------------------
step "Enumerate feature combinations"
# --------------------------------------------------------------------------
# Mechanically extract the [features] table from Cargo.toml.
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1] }
' "$CRATE/Cargo.toml" | grep -v '^default$' || true)

if [ -z "$FEATURES" ]; then
  echo "  no [features] table -> single configuration"
  COMBOS=("__default__" "__nodefault__")
else
  echo "  features: $FEATURES"
  COMBOS=("__default__" "__nodefault__")
  # Power set of the declared features.
  feats=($FEATURES)
  n=${#feats[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${feats[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "  ${#COMBOS[@]} configuration(s) to verify"

# --------------------------------------------------------------------------
step "cargo check / build / test per configuration"
# --------------------------------------------------------------------------
cd "$CRATE"
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__)   args=();                                            label="default features" ;;
    __nodefault__) args=(--no-default-features);                       label="--no-default-features" ;;
    *)             args=(--no-default-features --features "$combo");   label="--features $combo" ;;
  esac

  if ! timeout $TIMEOUT cargo check --release "${args[@]}" > /tmp/hsl_check.log 2>&1; then
    bad "cargo check [$label]"; tail -20 /tmp/hsl_check.log; continue
  fi
  ok "cargo check [$label]"

  # MUST build the cdylib explicitly; `cargo test` will not do it.
  # Both profiles are exercised: debug and release differ in whether rustc's
  # UB checks are compiled in, which is observable on the null-pointer rows.
  for prof in debug release; do
    if [ "$prof" = release ]; then pflag=(--release); else pflag=(); fi

    if ! timeout $TIMEOUT cargo build "${pflag[@]}" "${args[@]}" > /tmp/hsl_build.log 2>&1; then
      bad "cargo build [$label / $prof]"; tail -20 /tmp/hsl_build.log; continue
    fi
    ok "cargo build [$label / $prof]"

    if timeout $TIMEOUT cargo test "${pflag[@]}" "${args[@]}" > /tmp/hsl_test.log 2>&1; then
      ok "cargo test [$label / $prof]"
      grep -h 'test result:' /tmp/hsl_test.log | sed 's/^/       /'
    else
      bad "cargo test [$label / $prof]"; grep -E 'FAILED|panicked|test result:' /tmp/hsl_test.log | head -30
    fi
  done
done

# --------------------------------------------------------------------------
step "Symbol parity (nm -D diff must be empty)"
# --------------------------------------------------------------------------
RUST_SO="$CRATE/target/release/libhsl_to_rgb_lib.so"
STUBS='_ITM_deregisterTMCloneTable|_ITM_registerTMCloneTable|__cxa_finalize|__cxa_thread_atexit_impl|__gmon_start__'
syms() { nm -D --defined-only "$1" | awk '{print $NF}' | sed 's/@.*//' | grep -Ev "^($STUBS)\$" | sort -u; }

diff_out=$(comm -23 <(syms "$C_SO") <(syms "$RUST_SO"))
if [ -z "$diff_out" ]; then
  ok "0 C symbols missing from the Rust .so"
  echo "       C:    $(syms "$C_SO"    | tr '\n' ' ')"
  echo "       Rust: $(syms "$RUST_SO" | tr '\n' ' ')"
else
  bad "symbols exported by C but missing from Rust:"; echo "$diff_out" | sed 's/^/       /'
fi

# --------------------------------------------------------------------------
step "Result"
# --------------------------------------------------------------------------
if [ "$fail" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit $fail
