#!/usr/bin/env bash
# verify.sh — the Phase D completion gate, fully automated.
#
# 1. builds the reference C .so
# 2. enumerates every Cargo feature combination from Cargo.toml
# 3. for each combination: cargo check, build the cdylib, diff `nm -D` against
#    the C .so, and run the whole differential suite
#
# Exits non-zero on the first failure. Run from translation/.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0
TIMEOUT=600

step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ OK ] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# --------------------------------------------------------------------------
step "1. build the reference C shared object"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build .
) > /tmp/verify-c-build.log 2>&1 \
  && ok "C .so built" || { bad "C build (see /tmp/verify-c-build.log)"; tail -20 /tmp/verify-c-build.log; exit 1; }

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -n "$C_SO" ] && ok "C .so = $C_SO" || { bad "no C .so found"; exit 1; }

# --------------------------------------------------------------------------
step "2. enumerate feature combinations from Cargo.toml"
# Read the [features] table, if any, and build the power set of its keys.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); if ($0 != "default") print
    }
  ' Cargo.toml
)
NF_=${#FEATURES[@]}
echo "  declared non-default features: ${NF_} (${FEATURES[*]:-none})"

COMBOS=()
COMBOS+=("--all-features")          # superset
COMBOS+=("")                        # default
COMBOS+=("--no-default-features")   # nothing
if [ "$NF_" -gt 0 ]; then
  for ((mask = 1; mask < (1 << NF_); mask++)); do
    sel=()
    for ((i = 0; i < NF_; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi
# De-duplicate.
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')
echo "  combinations to verify: ${#COMBOS[@]}"

# --------------------------------------------------------------------------
step "3. per-combination: check, build, symbol diff, differential tests"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  printf '\n--- combo: %s ---\n' "$label"

  # shellcheck disable=SC2086
  if timeout $TIMEOUT cargo check --release $combo > /tmp/verify-check.log 2>&1; then
    ok "cargo check"
  else
    bad "cargo check ($label)"; tail -20 /tmp/verify-check.log; continue
  fi

  # shellcheck disable=SC2086
  if timeout $TIMEOUT cargo build --release $combo > /tmp/verify-build.log 2>&1; then
    ok "cargo build"
  else
    bad "cargo build ($label)"; tail -20 /tmp/verify-build.log; continue
  fi

  RUST_SO="target/release/libtritanopia_lib.so"
  [ -f "$RUST_SO" ] || { bad "missing $RUST_SO ($label)"; continue; }

  # Symbol parity: every symbol the C .so exports must be exported by the Rust
  # .so under the exact same name. The diff must be empty.
  nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u > /tmp/verify-c-syms.txt
  nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u > /tmp/verify-rust-syms.txt
  MISSING="$(comm -23 /tmp/verify-c-syms.txt /tmp/verify-rust-syms.txt)"
  if [ -z "$MISSING" ]; then
    ok "symbol parity: 0 missing ($(wc -l < /tmp/verify-c-syms.txt) C symbol(s))"
  else
    bad "symbols missing from the Rust .so ($label):"; echo "$MISSING" | sed 's/^/         /'
  fi

  # Undefined symbols must all be libc / libgcc imports.
  UNDEF="$(nm -D -u "$RUST_SO" | awk '{print $NF}' \
           | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^statx$|^gettid$' || true)"
  if [ -z "$UNDEF" ]; then
    ok "no non-libc undefined symbols"
  else
    bad "unexpected undefined symbols ($label):"; echo "$UNDEF" | sed 's/^/         /'
  fi

  # shellcheck disable=SC2086
  if timeout $TIMEOUT cargo test --release $combo > /tmp/verify-test.log 2>&1; then
    ok "differential suite: $(grep -c '^test .* ok$' /tmp/verify-test.log) test(s) passed"
  else
    bad "differential suite ($label)"; grep -E '^test .*FAILED|divergen' /tmp/verify-test.log | head -20
  fi
done

# --------------------------------------------------------------------------
step "4. debug profile (different codegen, same expected bytes)"
if timeout $TIMEOUT cargo build > /tmp/verify-dbg-build.log 2>&1 \
   && timeout $TIMEOUT cargo test > /tmp/verify-dbg-test.log 2>&1; then
  ok "debug profile: $(grep -c '^test .* ok$' /tmp/verify-dbg-test.log) test(s) passed"
else
  bad "debug profile"
  grep -E '^test .*FAILED|divergen|error' /tmp/verify-dbg-test.log /tmp/verify-dbg-build.log | head -20
fi

# --------------------------------------------------------------------------
step "5. no stubbed / faked symbols in the translation"
if grep -rnE 'unimplemented!|todo!|unreachable!\(\)' src/ > /tmp/verify-stubs.txt 2>/dev/null; then
  bad "stub macros present in src/:"; sed 's/^/         /' /tmp/verify-stubs.txt
else
  ok "no unimplemented!/todo! stubs"
fi

# --------------------------------------------------------------------------
step "6. c_src/ untouched"
if git -C "$ROOT" rev-parse --git-dir > /dev/null 2>&1; then
  DIRTY="$(git -C "$ROOT" status --porcelain -- c_src | grep -v 'c_src/build' || true)"
  [ -z "$DIRTY" ] && ok "c_src/ has no tracked modifications" \
                  || { bad "c_src/ modified:"; echo "$DIRTY" | sed 's/^/         /'; }
else
  ok "not a git repo; c_src/ was only ever read (build/ artifacts excepted)"
fi

printf '\n============================================\n'
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "FAILURES PRESENT"
fi
printf '============================================\n'
exit "$FAIL"
