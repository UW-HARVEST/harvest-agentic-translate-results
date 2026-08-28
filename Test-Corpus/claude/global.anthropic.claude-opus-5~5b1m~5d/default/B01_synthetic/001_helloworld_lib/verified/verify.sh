#!/usr/bin/env bash
# Full C-vs-Rust differential verification.
#
# Runs the whole Phase B / Phase C suite against a freshly built pair of shared
# objects, for every profile and every feature combination, and finishes with the
# Phase D symbol-parity diff.
#
# `cargo test` alone is NOT sufficient: it does not build a cdylib-only library
# target, so it would test whatever `.so` an earlier build left behind.  Every
# `cargo test` below is therefore preceded by the matching `cargo build`.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE=$PWD
ROOT=$(cd .. && pwd)
CARGO_FLAGS=${CARGO_FLAGS:---offline}

fail=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# 1. Build the C reference library
# ---------------------------------------------------------------------------
step "Building the C reference library"
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) && ok "c_src/build/libhello.so" || { bad "C build"; exit 1; }

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ { inf = 1; next }
  /^\[/           { inf = 0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
' Cargo.toml | grep -v '^default$' | sort -u)

COMBOS=()
if [ -z "$FEATURES" ]; then
  # No [features] table: --no-default-features and --all-features are the same
  # single configuration.  Exercise each spelling anyway so the matrix is honest.
  COMBOS=("" "--no-default-features" "--all-features")
else
  COMBOS=("" "--no-default-features" "--all-features")
  while read -r f; do
    [ -n "$f" ] && COMBOS+=("--no-default-features --features $f")
  done <<<"$FEATURES"
fi

step "Feature combinations to verify"
printf '  %s\n' "declared features: ${FEATURES:-(none)}"
for c in "${COMBOS[@]}"; do printf '  combo: [%s]\n' "${c:-default}"; done

# ---------------------------------------------------------------------------
# 3. Build + test every (profile, feature combination) pair
# ---------------------------------------------------------------------------
for profile in debug release; do
  prof_flag=""
  [ "$profile" = release ] && prof_flag="--release"

  for combo in "${COMBOS[@]}"; do
    step "profile=$profile features=[${combo:-default}]"

    # shellcheck disable=SC2086
    if ! cargo build $CARGO_FLAGS $prof_flag $combo >/dev/null 2>&1; then
      bad "cargo build ($profile, ${combo:-default})"
      continue
    fi
    so="target/$profile/libhello.so"
    [ -f "$so" ] || { bad "missing $so"; continue; }
    ok "built $so"

    # shellcheck disable=SC2086
    out=$(cargo test $CARGO_FLAGS $prof_flag $combo 2>&1)
    if printf '%s' "$out" | grep -q "^test result: FAILED" || [ -n "$(printf '%s' "$out" | grep -E '^error')" ]; then
      bad "cargo test ($profile, ${combo:-default})"
      printf '%s\n' "$out" | grep -E "^test result:|panicked at|differ|^error" | head -20 | sed 's/^/      /'
    else
      printf '%s\n' "$out" | grep -E "^test result:" | sed 's/^/      /'
      ok "cargo test ($profile, ${combo:-default})"
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. Phase D — symbol parity
# ---------------------------------------------------------------------------
step "Phase D: symbol parity (nm -D --defined-only)"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for profile in debug release; do
  so="target/$profile/libhello.so"
  [ -f "$so" ] || continue
  nm -D --defined-only "$ROOT/c_src/build/libhello.so" | awk '{print $NF}' | sort >"$tmp/c"
  nm -D --defined-only "$so" | awk '{print $NF}' | sort >"$tmp/r"
  missing=$(comm -23 "$tmp/c" "$tmp/r")
  extra=$(comm -13 "$tmp/c" "$tmp/r")
  if [ -z "$missing" ]; then
    ok "$profile: 0 symbols missing from the Rust .so ($(wc -l <"$tmp/c") exported by C)"
  else
    bad "$profile: symbols missing from the Rust .so:"
    printf '        %s\n' $missing
  fi
  [ -n "$extra" ] && printf '      note: extra symbols in Rust .so: %s\n' "$(echo $extra)"

  # No undefined symbol may be anything other than libc / libgcc runtime.
  undef=$(nm -D --undefined-only "$so" | awk '{print $NF}' | sed 's/@.*//' | sort -u)
  needed=$(objdump -p "$so" | awk '/NEEDED/ {print $2}' | tr '\n' ' ')
  printf '      NEEDED: %s\n' "$needed"
  bad_dep=$(printf '%s\n' "$needed" | tr ' ' '\n' | grep -vE '^(libc\.so\.6|libgcc_s\.so\.1|ld-linux-x86-64\.so\.2|)$')
  if [ -z "$bad_dep" ]; then
    ok "$profile: only libc/libgcc are needed ($(printf '%s\n' "$undef" | grep -c .) imported symbols)"
  else
    bad "$profile: unexpected shared-library dependency: $bad_dep"
  fi
done

# ---------------------------------------------------------------------------
# 5. Optional: prove the suite is not vacuous
# ---------------------------------------------------------------------------
if [ "${1:-}" = "--with-mutation" ]; then
  step "Meta-check: mutation testing the Rust translation"
  if ./mutation_test.sh; then
    ok "every mutant behaved as expected"
  else
    bad "mutation testing found a blind spot or a false divergence"
  fi
  # mutation_test.sh leaves debug artifacts rebuilt; make sure both profiles are
  # current again for anyone inspecting the tree afterwards.
  cargo build $CARGO_FLAGS >/dev/null 2>&1
  cargo build $CARGO_FLAGS --release >/dev/null 2>&1
fi

step "RESULT"
if [ "$fail" = 0 ]; then
  printf '  \033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '  \033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$fail"
