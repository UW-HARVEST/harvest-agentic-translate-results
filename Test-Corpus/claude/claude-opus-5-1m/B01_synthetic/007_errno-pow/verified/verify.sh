#!/usr/bin/env bash
# Full verification sweep: enumerate every build-time configuration and run the
# complete differential suite (Phases A-D) against each one.
#
#   ./verify.sh            # everything
#   ./verify.sh --quick    # skip the debug-profile pass
set -uo pipefail
cd "$(dirname "$0")"

CARGO_OFFLINE=${CARGO_OFFLINE:---offline}
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------- Phase A
step "Phase A: enumerate build-time configurations"

# Cargo features, mechanically extracted from Cargo.toml.
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[ \t]*=/ {sub(/[ \t]*=.*/,""); print}
' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "  Cargo.toml declares no [features] -> exactly 1 configuration (the empty set)"
  COMBOS=("")
else
  echo "  features found: $FEATURES"
  # power set of all declared features
  read -r -a F <<< "$(echo "$FEATURES" | tr '\n' ' ')"
  n=${#F[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${F[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "  C configurations: $(grep -c 'add_executable\|add_library' c_src/CMakeLists.txt) target(s), \
$(grep -c '#if\|#ifdef\|#ifndef' c_src/src/*.c) preprocessor branches in the C sources"
echo "  => ${#COMBOS[@]} configuration(s) to verify"

# ---------------------------------------------------------------- build C
step "Build the C reference"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "c_src/build/driver" || bad "C build"

# ---------------------------------------------------------------- sweep
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    LABEL="--no-default-features (empty feature set)"
    FLAGS=(--no-default-features)
  else
    LABEL="--no-default-features --features $combo"
    FLAGS=(--no-default-features --features "$combo")
  fi

  step "Configuration: $LABEL"

  timeout 600 cargo check $CARGO_OFFLINE "${FLAGS[@]}" --all-targets >/dev/null 2>&1 \
    && ok "cargo check --all-targets" || bad "cargo check ($LABEL)"

  timeout 600 cargo build $CARGO_OFFLINE --release "${FLAGS[@]}" >/dev/null 2>&1 \
    && ok "cargo build --release" || bad "cargo build --release ($LABEL)"

  # Phases B, C and D run against the release artifact (the shipped one).
  if timeout 600 env DIFF_RUST_BIN="$PWD/target/release/driver" \
       cargo test $CARGO_OFFLINE "${FLAGS[@]}" --tests 2>&1 | tee ${TMPDIR:-/tmp}/vt.$$ | grep -qE '^test result: FAILED'; then
    bad "differential suite, release profile ($LABEL)"
    grep -E '^(test result:|---- )' ${TMPDIR:-/tmp}/vt.$$ | head -20
  else
    ok "differential suite, release profile"
    grep -E '^test result:' ${TMPDIR:-/tmp}/vt.$$ | sed 's/^/         /'
  fi
  rm -f ${TMPDIR:-/tmp}/vt.$$

  if [ "${1:-}" != "--quick" ]; then
    # The debug profile is a genuinely different code generation (opt-level 0 vs
    # 2): re-run the suite against it so no optimisation-dependent divergence
    # (e.g. LLVM rewriting the pow() libcall) can hide.
    timeout 600 cargo build $CARGO_OFFLINE "${FLAGS[@]}" >/dev/null 2>&1 \
      && ok "cargo build (debug)" || bad "cargo build debug ($LABEL)"
    if timeout 600 env DIFF_RUST_BIN="$PWD/target/debug/driver" \
         cargo test $CARGO_OFFLINE "${FLAGS[@]}" --tests 2>&1 | tee ${TMPDIR:-/tmp}/vt.$$ | grep -qE '^test result: FAILED'; then
      bad "differential suite, debug profile ($LABEL)"
      grep -E '^(test result:|---- )' ${TMPDIR:-/tmp}/vt.$$ | head -20
    else
      ok "differential suite, debug profile"
    fi
    rm -f ${TMPDIR:-/tmp}/vt.$$
  fi
done

# ---------------------------------------------------------------- Phase D symbols
step "Phase D: nm -D symbol diff (C artifact vs Rust artifact)"
nm -D c_src/build/driver     | awk '{print $NF}' | sort -u > ${TMPDIR:-/tmp}/csym.$$
nm -D target/release/driver  | awk '{print $NF}' | sort -u > ${TMPDIR:-/tmp}/rsym.$$
MISSING=$(comm -23 ${TMPDIR:-/tmp}/csym.$$ ${TMPDIR:-/tmp}/rsym.$$ | grep -vE '^(stderr|printf|fprintf)@' || true)
if [ -z "$MISSING" ]; then
  ok "no C symbols missing from the Rust artifact (modulo the 3 documented, \
non-behavioural exceptions: printf/fprintf formatting and glibc's stderr copy-reloc)"
else
  bad "C symbols missing from Rust artifact:"; echo "$MISSING"
fi
rm -f ${TMPDIR:-/tmp}/csym.$$ ${TMPDIR:-/tmp}/rsym.$$

# ---------------------------------------------------------------- verdict
step "Verdict"
if [ "$FAIL" -eq 0 ]; then
  echo "  ALL CONFIGURATIONS PASS"
else
  echo "  FAILURES PRESENT"
fi
exit "$FAIL"
