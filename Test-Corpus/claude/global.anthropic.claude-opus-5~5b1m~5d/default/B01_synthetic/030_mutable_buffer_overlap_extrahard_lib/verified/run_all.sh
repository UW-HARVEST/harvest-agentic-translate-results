#!/usr/bin/env bash
# Full verification run: build the C ground truth, then run every differential
# test under every Cargo feature combination and both build profiles, and print
# the C-vs-Rust `nm -D` symbol diff.
#
# Usage:  cd translation && ./run_all.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
FAIL=0

hdr() { printf '\n=========== %s ===========\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library (the ground truth).
# ---------------------------------------------------------------------------
hdr "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml.
#    This crate declares no [features], so the set is just the default build;
#    the loop is written generically so it keeps working if features are added.
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
COMBOS+=("--all-features")            # equals default when there are no features
COMBOS+=("--no-default-features")
if [ -n "$FEATURES" ]; then
  # every individual feature, plus the full set, on top of no-default-features
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
  done
  ALL=$(echo "$FEATURES" | paste -sd,)
  COMBOS+=("--no-default-features --features $ALL")
fi

echo
echo "Feature list from Cargo.toml: ${FEATURES:-<none declared>}"
echo "Combinations to verify:"
printf '  cargo test %s\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. cargo check every combination first (fast failure).
# ---------------------------------------------------------------------------
hdr "cargo check, every feature combination"
for combo in "${COMBOS[@]}"; do
  echo "--- cargo check $combo"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check --all-targets $combo 2>&1 | tail -n 3; then
    echo "CHECK FAILED: $combo"; FAIL=1
  fi
done

# ---------------------------------------------------------------------------
# 4. Run the whole differential suite for every combination x profile.
#    The tests are also run against the RELEASE cdylib, which is built with
#    optimisations and `panic = "abort"` -- a genuinely different code path from
#    the debug build.
# ---------------------------------------------------------------------------
for profile_flag in "" "--release"; do
  pname=$([ -z "$profile_flag" ] && echo debug || echo release)
  for combo in "${COMBOS[@]}"; do
    hdr "TEST profile=$pname $combo"
    # Build the cdylib first so the harness can dlopen it even when a single
    # --test target is selected.
    # shellcheck disable=SC2086
    timeout 600 cargo build $profile_flag $combo >/dev/null 2>&1
    # shellcheck disable=SC2086
    if timeout 600 cargo test $profile_flag $combo 2>&1 | tail -n 60; then
      echo "PASS  profile=$pname $combo"
    else
      echo "FAIL  profile=$pname $combo"; FAIL=1
    fi
  done
done

# ---------------------------------------------------------------------------
# 4b. Re-run the suite against an OPTIMISED (-O2) build of the same C sources.
#     The C code's `a*b + c` is signed-overflow UB, so this confirms the
#     ground-truth wrapping behaviour is not an artefact of the unoptimised
#     default CMake build. c_src/ is never modified: the object goes elsewhere.
# ---------------------------------------------------------------------------
hdr "TEST against an -O2 C build (UB-stability cross-check)"
O2DIR="$HERE/target/c_o2"
mkdir -p "$O2DIR"
if ( cd "$O2DIR" && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
       -DCMAKE_BUILD_TYPE=Release >/dev/null 2>&1 && cmake --build . >/dev/null 2>&1 ); then
  timeout 600 cargo build >/dev/null 2>&1
  if C_SO="$O2DIR/libdriver.so" timeout 600 cargo test 2>&1 | tail -n 40; then
    echo "PASS  C built with -O2"
  else
    echo "FAIL  C built with -O2"; FAIL=1
  fi
else
  echo "skip: could not configure the -O2 C build"
fi

# ---------------------------------------------------------------------------
# 5. Symbol parity diff (must be empty).
# ---------------------------------------------------------------------------
hdr "Symbol parity: nm -D --defined-only"
syms() { nm -D --defined-only "$1" | awk '{print $NF}' \
           | grep -Ev '^(_init|_fini|__bss_start|_edata|_end|_IO_stdin_used)$' | sort -u; }

for pname in debug release; do
  RS_SO="$HERE/target/$pname/libdriver.so"
  [ -f "$RS_SO" ] || { echo "skip: $RS_SO not built"; continue; }
  echo "--- C vs Rust ($pname)"
  DIFF=$(comm -23 <(syms "$C_SO") <(syms "$RS_SO"))
  if [ -n "$DIFF" ]; then
    echo "MISSING FROM RUST ($pname):"; echo "$DIFF"; FAIL=1
  else
    echo "OK: 0 symbols missing from the Rust .so"
    echo "C    exports: $(syms "$C_SO" | tr '\n' ' ')"
    echo "Rust exports: $(syms "$RS_SO" | tr '\n' ' ')"
  fi
  echo "--- ldd -r ($pname): unresolved symbols"
  ldd -r "$RS_SO" 2>&1 | grep -E 'undefined symbol|not found' && FAIL=1 || echo "OK: none"
done

hdr "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$FAIL"
