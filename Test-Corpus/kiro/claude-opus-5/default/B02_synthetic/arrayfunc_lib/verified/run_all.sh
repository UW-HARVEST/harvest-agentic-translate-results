#!/usr/bin/env bash
# Full verification matrix: builds both libraries, diffs the exported symbol
# sets, and runs the differential suite under every feature combination and
# against both the release and debug Rust `.so`.
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
FAIL=0
step() { printf '\n=== %s ===\n' "$1"; }
ok()   { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; FAIL=1; }

# --------------------------------------------------------------------------
step "Build the C shared library"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) && ok "C library built" || bad "C library build"

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
[ -f "$C_SO" ] && ok "C .so: $C_SO" || bad "no C .so found"

# --------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,""); gsub(/ /,""); if ($0!="default" && $0!="") print}' \
    "$CRATE_DIR/Cargo.toml"
)
echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

COMBOS=()
COMBOS+=("--all-features")
COMBOS+=("")                       # default features
COMBOS+=("--no-default-features")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ] && [ "$n" -le 12 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $combo")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

# --------------------------------------------------------------------------
step "cargo check + build + test for every feature combination"
cd "$CRATE_DIR"
for combo in "${COMBOS[@]}"; do
  label="cargo ${combo:-<default features>}"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check --quiet $combo >/dev/null 2>&1; then
    bad "check   $label"; continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --quiet --release $combo >/dev/null 2>&1; then
    bad "build   $label"; continue
  fi

  # symbol parity for this combination
  R_SO="$CRATE_DIR/target/release/libarrayfunc_lib.so"
  nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort >/tmp/c_syms.txt
  nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort >/tmp/r_syms.txt
  missing="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
  if [ -n "$missing" ]; then
    bad "symbols $label -- missing from Rust .so: $(echo "$missing" | tr '\n' ' ')"
  else
    ok "symbols $label ($(wc -l </tmp/c_syms.txt) symbols, 0 missing)"
  fi

  for profile in release debug; do
    # shellcheck disable=SC2086
    timeout 600 cargo build --quiet $( [ "$profile" = release ] && echo --release ) $combo >/dev/null 2>&1
    so="$CRATE_DIR/target/$profile/libarrayfunc_lib.so"
    # A `debug-assertions` build inserts Rust's `ub_checks`, which turns a null
    # dereference into a non-unwinding panic (SIGABRT) instead of letting the
    # hardware fault (SIGSEGV) happen. That instrumentation has no C
    # counterpart and is not disableable on stable, so exactly one test -- the
    # null-dereference signal-parity test -- is out of scope for this pass.
    # Every value-comparing test still runs.
    SKIP=()
    if [ "$profile" = debug ]; then
      SKIP=(--skip err_null_pointer_dereferences)
      echo "NOTE  debug pass: skipping err_null_pointer_dereferences (ub_checks turns SIGSEGV into SIGABRT)"
    fi
    # shellcheck disable=SC2086
    if HARVEST_RUST_SO="$so" HARVEST_C_SO="$C_SO" \
        timeout 600 cargo test --quiet --release $combo -- "${SKIP[@]+"${SKIP[@]}"}" \
        >/tmp/test_out.txt 2>&1; then
      # Sum across both test binaries (lib unittests + tests/differential.rs)
      # and require a plausible minimum so a filtered-to-nothing run cannot
      # masquerade as a pass.
      passed=$(grep -oE '[0-9]+ passed' /tmp/test_out.txt | awk '{s+=$1} END{print s+0}')
      failed=$(grep -oE '[0-9]+ failed' /tmp/test_out.txt | awk '{s+=$1} END{print s+0}')
      min=64; [ "$profile" = debug ] && min=63
      if [ "$passed" -ge "$min" ] && [ "$failed" -eq 0 ]; then
        ok "tests   $label [rust .so = $profile]  $passed passed, $failed failed"
      else
        bad "tests   $label [rust .so = $profile]  only $passed passed (expected >= $min), $failed failed"
      fi
    else
      bad "tests   $label [rust .so = $profile]"
      sed -n '/^failures:$/,/^test result/p' /tmp/test_out.txt | head -30
    fi
  done
done

# --------------------------------------------------------------------------
step "Binary-executable comparison"
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" 2>/dev/null; then
  bad "c_src declares add_executable but no driver comparison is wired up"
elif [ -f "$CRATE_DIR/src/main.rs" ] || grep -q '^\[\[bin\]\]' "$CRATE_DIR/Cargo.toml"; then
  bad "the crate declares a binary but c_src does not"
else
  ok "neither side builds an executable (library-only project) -- nothing to diff"
fi

# --------------------------------------------------------------------------
step "Summary"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "SOME CHECKS FAILED"
fi
exit "$FAIL"
