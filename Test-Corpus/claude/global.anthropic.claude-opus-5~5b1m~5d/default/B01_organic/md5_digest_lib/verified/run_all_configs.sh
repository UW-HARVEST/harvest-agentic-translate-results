#!/usr/bin/env bash
# Phase D driver: run the whole differential suite under every feature
# combination and every build profile, plus a symbol-parity diff.
#
# Usage:  ./run_all_configs.sh
set -uo pipefail

cd "$(dirname "$0")"
CARGO="cargo"
OFFLINE="--offline"        # the sandbox has no crates.io egress; deps are vendored in ~/.cargo
FAIL=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 0. Locate / build the C ground truth.
# ---------------------------------------------------------------------------
step "Build C shared library (ground truth)"
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO=$(ls ../c_src/build/lib*.so | head -1)
ok "C .so = $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations mechanically from Cargo.toml.
# ---------------------------------------------------------------------------
step "Enumerate cargo feature combinations"
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {gsub(/[[:space:]]*=.*/,""); print}
' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "  Cargo.toml declares no [features] table -> exactly one real combination."
  echo "  Running default / --no-default-features / --all-features anyway to PROVE it."
  COMBOS=("default" "none" "all")
else
  echo "  features found: $FEATURES"
  COMBOS=("default" "none")
  for f in $FEATURES; do COMBOS+=("$f"); done
  COMBOS+=("all")
fi

flags_for() {
  case "$1" in
    default) echo "" ;;
    none)    echo "--no-default-features" ;;
    all)     echo "--all-features" ;;
    *)       echo "--no-default-features --features $1" ;;
  esac
}

# ---------------------------------------------------------------------------
# 2. cargo check + full test suite for each combination x each profile.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  FL=$(flags_for "$combo")
  for profile in dev release; do
    PF=""; [ "$profile" = release ] && PF="--release"
    label="features=$combo profile=$profile"

    step "check   $label"
    if timeout 600 $CARGO check $OFFLINE $FL $PF --tests >/dev/null 2>&1; then
      ok "cargo check ($label)"
    else
      bad "cargo check ($label)"; continue
    fi

    step "build   $label"
    timeout 600 $CARGO build $OFFLINE $FL $PF >/dev/null 2>&1 \
      && ok "cargo build ($label)" || { bad "cargo build ($label)"; continue; }

    step "test    $label"
    out=$(timeout 600 $CARGO test $OFFLINE $FL $PF 2>&1)
    if echo "$out" | grep -qE "^test result: FAILED|error: test failed|panicked"; then
      bad "cargo test ($label)"
      echo "$out" | grep -E "^test .* FAILED|^test result|panicked|C and Rust disagree" | head -30 | sed 's/^/      /'
    else
      echo "$out" | grep -E "^test result" | sed 's/^/      /'
      ok "cargo test ($label)"
    fi

    # ---- symbol parity diff for this profile ----
    step "symbols $label"
    RS="target/$([ "$profile" = release ] && echo release || echo debug)/libmd5_digest_lib.so"
    if [ ! -f "$RS" ]; then bad "missing $RS"; continue; fi
    d=$(diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
             <(nm -D --defined-only "$RS"   | awk '{print $3}' | sort))
    if [ -z "$d" ]; then ok "nm -D diff empty ($label)"; else bad "nm -D diff ($label):"; echo "$d" | sed 's/^/      /'; fi
  done
done

# ---------------------------------------------------------------------------
# 3. Robustness: the same Rust must also match a C built at -O1/-O2/-O3.
#    (The CMake default build is unoptimized; the reload-before-every-store
#    behaviour is mandated by the aliasing rules at every level, so the Rust
#    must agree with all of them.)
# ---------------------------------------------------------------------------
step "Differential vs C compiled at -O0/-O1/-O2/-O3/-Os"
mkdir -p target/copt
for O in 0 1 2 3 s; do
  so="target/copt/libc_O$O.so"
  gcc -O$O -shared -fPIC -I../c_src/include -o "$so" ../c_src/src/lib.c 2>/dev/null \
    || { bad "gcc -O$O"; continue; }
  out=$(C_SO_PATH="$PWD/$so" timeout 600 $CARGO test $OFFLINE 2>&1)
  if echo "$out" | grep -qE "^test result: FAILED|error: test failed"; then
    bad "differential vs C -O$O"
    echo "$out" | grep -E "^test .* FAILED|C and Rust disagree" | head -10 | sed 's/^/      /'
  else
    ok "differential vs C -O$O"
  fi
done

# ---------------------------------------------------------------------------
# 4. Mutation check: the suite must REJECT deliberately-wrong implementations.
# ---------------------------------------------------------------------------
step "Mutation check (suite must FAIL on each mutant)"
if [ -d target/mutants ]; then
  for m in target/mutants/lib*.so; do
    out=$(RUST_SO_PATH="$PWD/$m" timeout 600 $CARGO test $OFFLINE --tests 2>&1)
    if echo "$out" | grep -qE "^test result: FAILED|error: test failed"; then
      ok "mutant $(basename "$m") correctly rejected"
    else
      bad "mutant $(basename "$m") NOT detected - suite is too weak"
    fi
  done
else
  echo "  (no target/mutants; skipping)"
fi

step "SUMMARY"
[ $FAIL -eq 0 ] && echo "  ALL CONFIGURATIONS PASS" || echo "  FAILURES PRESENT"
exit $FAIL
