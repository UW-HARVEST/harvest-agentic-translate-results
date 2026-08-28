#!/usr/bin/env bash
# Full verification driver: builds the C .so, builds the Rust cdylib in every
# build configuration, checks symbol parity, and runs the whole differential
# suite (Phases A–D) against each configuration.
#
# Usage:  ./run_all.sh          (from translation/)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
XLAT="$ROOT/translation"
CARGO_FLAGS="--offline"
FAIL=0

step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mOK\033[0m   %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------- C library ---
step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
ok "C .so = $C_SO"

# ------------------------------------------------------- feature enumeration --
step "Enumerating cargo feature combinations"
FEATURES="$(sed -n '/^\[features\]/,/^\[/p' "$XLAT/Cargo.toml" \
            | grep -E '^[A-Za-z0-9_-]+[[:space:]]*=' | cut -d= -f1 | tr -d ' ')"
if [ -z "$FEATURES" ]; then
  ok "no [features] table -> the default build is the only configuration"
  COMBOS=("default")
else
  # power set of the declared features
  COMBOS=("default" "--no-default-features")
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
  ok "features: $FEATURES"
fi

step "cargo check for every feature combination"
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "default" ]; then flags=""; else flags="$combo"; fi
  # shellcheck disable=SC2086
  if cargo check $CARGO_FLAGS $flags >/dev/null 2>&1; then
    ok "cargo check [$combo]"
  else
    bad "cargo check [$combo]"
  fi
done

# ------------------------------------------------- build + test per opt level --
# The `x86_mul`/`x86_add`/`x86_sub`/`fneg` helpers in src/lib.rs pin down which
# NaN an expression propagates and whether a sign flip is fused into neighbouring
# arithmetic.  Those are codegen-sensitive, so the whole suite is re-run against
# the cdylib built at every optimisation level.
for OPT in 0 1 2 3 s z; do
  step "Rust cdylib at opt-level=$OPT"
  RUSTFLAGS="-C opt-level=$OPT" \
    cargo build $CARGO_FLAGS --release --target-dir "target/optlevel-$OPT" >/dev/null 2>&1 \
    || { bad "build opt-level=$OPT"; continue; }
  R_SO="$XLAT/target/optlevel-$OPT/release/libomni_manifold_lib.so"
  [ -f "$R_SO" ] || { bad "missing $R_SO"; continue; }

  # -- symbol parity (Phase A / Phase D) --
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "$XLAT/target/.c_syms"
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > "$XLAT/target/.r_syms"
  MISSING="$(comm -23 "$XLAT/target/.c_syms" "$XLAT/target/.r_syms")"
  EXTRA="$(comm -13 "$XLAT/target/.c_syms" "$XLAT/target/.r_syms" | grep -v '^_' || true)"
  if [ -z "$MISSING" ]; then
    ok "symbol parity: $(wc -l < "$XLAT/target/.c_syms") C symbols, 0 missing from Rust"
  else
    bad "symbols missing from the Rust .so:"; echo "$MISSING" | sed 's/^/       /'
  fi
  [ -z "$EXTRA" ] || printf '  note extra Rust symbols: %s\n' "$(echo "$EXTRA" | tr '\n' ' ')"

  # -- undefined symbols must all be libc / unwinder --
  UNDEF="$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
           | grep -v -E '^(_ITM_|__cxa_|__gmon_|_Unwind_|__tls_get_addr|__errno_location|statx|gettid)' \
           | sed 's/@.*//' | sort -u)"
  BAD_UNDEF="$(echo "$UNDEF" | grep -E '^c2|^omni_|^ptr_from' || true)"
  if [ -z "$BAD_UNDEF" ]; then
    ok "no undefined library symbols in the Rust .so"
  else
    bad "undefined non-libc symbols: $BAD_UNDEF"
  fi

  # -- full differential suite against this .so --
  for combo in "${COMBOS[@]}"; do
    if [ "$combo" = "default" ]; then flags=""; else flags="$combo"; fi
    # shellcheck disable=SC2086
    if C_SO="$C_SO" RUST_SO="$R_SO" cargo test $CARGO_FLAGS --no-fail-fast $flags \
         > "$XLAT/target/test-opt$OPT.log" 2>&1; then
      ok "differential suite [opt-level=$OPT, $combo]: $(grep -c '^test .* ok$' "$XLAT/target/test-opt$OPT.log") tests"
    else
      bad "differential suite [opt-level=$OPT, $combo] — see target/test-opt$OPT.log"
      grep -E 'DIVERGENCE|diverged|panicked|SIGSEGV|FAILED' "$XLAT/target/test-opt$OPT.log" | head -20 | sed 's/^/       /'
    fi
  done
done

step "Summary"
if [ "$FAIL" -eq 0 ]; then
  printf '  \033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '  \033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAIL"
