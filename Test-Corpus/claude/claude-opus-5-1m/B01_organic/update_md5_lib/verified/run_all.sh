#!/usr/bin/env bash
# Full verification matrix: Phase A artifacts → Phase B/C differential tests →
# Phase D symbol parity, for EVERY feature combination and every Rust build
# profile.
#
# IMPORTANT: `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` target
# (the integration tests dlopen it rather than linking it), so every test run
# below is preceded by an explicit `cargo build`.  `tests/harness/mod.rs` also
# asserts the .so is not older than src/*.rs, so a forgotten rebuild fails
# loudly instead of silently validating a stale library.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$PWD"
FAIL=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 0. Enumerate every valid feature combination from Cargo.toml
# ---------------------------------------------------------------------------
step "Feature combinations"
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "  Cargo.toml declares no [features] -> exactly one valid combination: <none>"
  COMBOS=("")
else
  echo "  features: ${FEATURES[*]}"
  COMBOS=("")
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    c=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then c="${c:+$c,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$c")
  done
fi
echo "  ${#COMBOS[@]} combination(s) to verify"

# ---------------------------------------------------------------------------
# 1. cargo check for every combination
# ---------------------------------------------------------------------------
step "cargo check --no-default-features --features <combo>"
for c in "${COMBOS[@]}"; do
  label="${c:-<none>}"
  if timeout 600 cargo check --no-default-features --features "$c" \
       > "${TMPDIR:-/tmp}/check.log" 2>&1; then
    ok "cargo check [$label]"
  else
    bad "cargo check [$label]"; tail -30 "${TMPDIR:-/tmp}/check.log"
  fi
  if timeout 600 cargo check --no-default-features --features "$c" --tests \
       > "${TMPDIR:-/tmp}/check-tests.log" 2>&1; then
    ok "cargo check --tests [$label]"
  else
    bad "cargo check --tests [$label]"; tail -30 "${TMPDIR:-/tmp}/check-tests.log"
  fi
done

# ---------------------------------------------------------------------------
# 2. Build the C shared library (default configuration)
# ---------------------------------------------------------------------------
step "Build C shared library"
mkdir -p c_src/build
if ( cd c_src/build \
     && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
     && cmake --build . ) > "${TMPDIR:-/tmp}/cmake.log" 2>&1; then
  C_SO="$ROOT/$(cd c_src/build && ls *.so | head -1)"
  C_SO="$ROOT/c_src/build/$(cd c_src/build && ls *.so | head -1)"
  ok "C .so -> $C_SO"
else
  bad "cmake build"; tail -30 "${TMPDIR:-/tmp}/cmake.log"; exit 1
fi
export C_SO

# ---------------------------------------------------------------------------
# 3/4. For every combination × profile: build the cdylib, diff symbols, test
# ---------------------------------------------------------------------------
for c in "${COMBOS[@]}"; do
  label="${c:-<none>}"
  for profile in dev release; do
    if [ "$profile" = release ]; then
      pflag=(--release); outdir=target/release
    else
      pflag=(); outdir=target/debug
    fi
    step "features=[$label] profile=$profile"

    if timeout 600 cargo build --no-default-features --features "$c" "${pflag[@]}" \
         > "${TMPDIR:-/tmp}/build.log" 2>&1; then
      ok "cargo build"
    else
      bad "cargo build"; tail -30 "${TMPDIR:-/tmp}/build.log"; continue
    fi

    RUST_SO="$ROOT/$outdir/libupdate_md5_lib.so"
    if [ ! -f "$RUST_SO" ]; then bad "missing $RUST_SO"; continue; fi
    export RUST_SO

    # ---- Phase D: symbol parity (nm -D diff must be EMPTY) ----
    missing=$(comm -23 \
      <(nm -D --defined-only --format=posix "$C_SO"   | awk '{print $1}' | sort -u) \
      <(nm -D --defined-only --format=posix "$RUST_SO" | awk '{print $1}' | sort -u))
    if [ -z "$missing" ]; then
      ok "symbol parity (nm -D diff empty)"
    else
      bad "symbols missing from Rust .so:"; echo "$missing" | sed 's/^/      /'
    fi

    # ---- Phase B + C + D differential tests ----
    if timeout 600 cargo test --no-default-features --features "$c" "${pflag[@]}" --tests \
         > "${TMPDIR:-/tmp}/test.log" 2>&1; then
      ok "$(grep -c '^test .* ok$' "${TMPDIR:-/tmp}/test.log") differential tests passed"
      grep -E '^test result:' "${TMPDIR:-/tmp}/test.log" | sed 's/^/      /'
    else
      bad "differential tests"
      grep -E '^test result:|FAILED|panicked at|mismatch' "${TMPDIR:-/tmp}/test.log" | head -40 | sed 's/^/      /'
    fi
  done
done

step "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
  printf '  \033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '  \033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAIL"
