#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY buildable
# configuration of the crate.
#
#   1. enumerate the `[features]` table from Cargo.toml (mechanically, no
#      hard-coded list) and build the power set of optional features;
#   2. for each combination: cargo check, cargo build --release, cargo test;
#   3. additionally re-run the suite against the *debug* artifact, which is
#      compiled with overflow checks on and `panic = unwind` — a different code
#      path for every arithmetic operation in the library.
#
# Usage:  ./check_all_features.sh
set -uo pipefail

cd "$(dirname "$0")"
CARGO="cargo"
OFFLINE="--offline"          # crates.io is unreachable in this sandbox
FAIL=0

banner() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate features from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

banner "features declared in Cargo.toml"
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "(none — the crate has no [features] table, so the default build is the"
  echo " ONLY buildable configuration; --no-default-features is equivalent to it)"
else
  printf '  %s\n' "${FEATURES[@]}"
fi

# ---------------------------------------------------------------------------
# 2. Build the list of feature-flag argument sets to test
# ---------------------------------------------------------------------------
COMBOS=()
COMBOS+=("")                            # default
COMBOS+=("--no-default-features")
COMBOS+=("--all-features")

n=${#FEATURES[@]}
if [ "$n" -gt 0 ] && [ "$n" -le 12 ]; then
  total=$(( 1 << n ))
  for (( mask = 0; mask < total; mask++ )); do
    sel=""
    for (( b = 0; b < n; b++ )); do
      if (( (mask >> b) & 1 )); then
        sel="${sel:+$sel,}${FEATURES[$b]}"
      fi
    done
    COMBOS+=("--no-default-features --features ${sel}")
    COMBOS+=("--features ${sel}")
  done
fi

# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

banner "configurations to verify (${#COMBOS[@]})"
for cfg in "${COMBOS[@]}"; do echo "  cargo <cmd> ${cfg:-<default>}"; done

# ---------------------------------------------------------------------------
# 3. Make sure the C ground-truth library exists
# ---------------------------------------------------------------------------
banner "building the C ground-truth shared library"
( mkdir -p ../c_src/build \
  && cd ../c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -1 ../c_src/build/lib*.so

# ---------------------------------------------------------------------------
# 4. Verify every configuration
# ---------------------------------------------------------------------------
for cfg in "${COMBOS[@]}"; do
  banner "cargo check ${cfg:-<default>}"
  # shellcheck disable=SC2086
  if ! timeout 600 $CARGO check $OFFLINE --all-targets $cfg 2>&1 | tail -3; then
    echo "CHECK FAILED for '${cfg}'"; FAIL=1; continue
  fi

  banner "cargo build --release ${cfg:-<default>}"
  # shellcheck disable=SC2086
  if ! timeout 600 $CARGO build $OFFLINE --release $cfg 2>&1 | tail -3; then
    echo "BUILD FAILED for '${cfg}'"; FAIL=1; continue
  fi

  banner "symbol parity ${cfg:-<default>}"
  C_SO=$(ls ../c_src/build/lib*.so | head -1)
  diff <(nm -D --defined-only "$C_SO"                        | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/release/libsh_puts_lib.so | awk '{print $NF}' | sort) \
    && echo "  symbol diff EMPTY (ok)" \
    || { echo "  SYMBOL DIFF NON-EMPTY for '${cfg}'"; FAIL=1; }

  banner "cargo test --release ${cfg:-<default>}"
  # shellcheck disable=SC2086
  if ! timeout 600 $CARGO test $OFFLINE --release $cfg -- --test-threads=1 2>&1 \
        | grep -E 'test result|^error|FAILED'; then
    echo "TEST RUN PRODUCED NO RESULT LINE for '${cfg}'"; FAIL=1
  fi
  # shellcheck disable=SC2086
  timeout 600 $CARGO test $OFFLINE --release $cfg -- --test-threads=1 >/dev/null 2>&1 \
    || { echo "TESTS FAILED for '${cfg}'"; FAIL=1; }
done

# ---------------------------------------------------------------------------
# 5. Re-run the suite against the DEBUG artifact (overflow checks on,
#    panic = unwind): different codegen for every arithmetic op.
# ---------------------------------------------------------------------------
banner "debug-profile artifact (overflow checks ON)"
timeout 600 $CARGO build $OFFLINE >/dev/null 2>&1 || { echo "debug build FAILED"; FAIL=1; }
if [ -f target/debug/libsh_puts_lib.so ]; then
  C_SO=$(ls ../c_src/build/lib*.so | head -1)
  diff <(nm -D --defined-only "$C_SO"                      | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/debug/libsh_puts_lib.so | awk '{print $NF}' | sort) \
    && echo "  debug symbol diff EMPTY (ok)" \
    || { echo "  DEBUG SYMBOL DIFF NON-EMPTY"; FAIL=1; }

  RUST_TRANSLATION_SO="$(pwd)/target/debug/libsh_puts_lib.so" \
    timeout 600 $CARGO test $OFFLINE --release -- --test-threads=1 2>&1 \
    | grep -E 'test result|^error|FAILED'
  RUST_TRANSLATION_SO="$(pwd)/target/debug/libsh_puts_lib.so" \
    timeout 600 $CARGO test $OFFLINE --release -- --test-threads=1 >/dev/null 2>&1 \
    || { echo "TESTS FAILED against the debug artifact"; FAIL=1; }
else
  echo "target/debug/libsh_puts_lib.so not produced"; FAIL=1
fi

banner "RESULT"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "FAILURES DETECTED"
fi
exit "$FAIL"
