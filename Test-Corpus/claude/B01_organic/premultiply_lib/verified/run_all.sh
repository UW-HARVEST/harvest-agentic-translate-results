#!/usr/bin/env bash
# Full differential verification: every feature combination x every profile.
#
# Usage:  ./run_all.sh
#
# 1. Enumerates the feature combinations from Cargo.toml (the crate declares
#    none, so the only combination is the empty one; the loop is written to
#    handle any number of features should they be added).
# 2. `cargo check` every combination.
# 3. Builds the C reference .so (CMake, exactly as the task specifies).
# 4. Builds the Rust cdylib in dev AND release and runs Phases B, C and D of the
#    differential suite against each.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
LOG="${TMPDIR:-/tmp}/premult_verify"
mkdir -p "$LOG"
FAIL=0

step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations (power set of [features] in Cargo.toml)
# ---------------------------------------------------------------------------
step "Enumerating feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
  ' Cargo.toml
)
echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

COMBOS=()
N=${#FEATURES[@]}
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((b = 0; b < N; b++)); do
    if ((mask & (1 << b))); then combo="${combo:+$combo,}${FEATURES[b]}"; fi
  done
  COMBOS+=("$combo")
done
echo "combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. cargo check for every combination
# ---------------------------------------------------------------------------
step "cargo check for every feature combination"
for combo in "${COMBOS[@]}"; do
  name="${combo:-<none>}"
  if timeout 300 cargo check --no-default-features ${combo:+--features "$combo"} \
        > "$LOG/check.log" 2>&1; then
    ok "cargo check --no-default-features --features '$name'"
  else
    bad "cargo check --no-default-features --features '$name'"; tail -20 "$LOG/check.log"
  fi
done
if timeout 300 cargo check --all-features > "$LOG/check_all.log" 2>&1; then
  ok "cargo check --all-features"
else
  bad "cargo check --all-features"; tail -20 "$LOG/check_all.log"
fi

# ---------------------------------------------------------------------------
# 3. Build the C reference shared library
# ---------------------------------------------------------------------------
step "Building the C reference shared library"
mkdir -p c_src/build
if (cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && cmake --build .) > "$LOG/cmake.log" 2>&1; then
  C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
  ok "C .so: $C_SO"
else
  bad "cmake build"; tail -20 "$LOG/cmake.log"; exit 1
fi
export PREMULT_C_SO="$C_SO"

# ---------------------------------------------------------------------------
# 4. Build the Rust cdylib and run the differential suite, per profile
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  name="${combo:-<none>}"
  for profile in dev release; do
    step "Differential suite: features='$name' profile=$profile"

    if [ "$profile" = release ]; then
      build_flags=(--release); outdir=target/release
    else
      build_flags=();          outdir=target/debug
    fi

    if ! timeout 300 cargo build --lib "${build_flags[@]}" \
          --no-default-features ${combo:+--features "$combo"} \
          > "$LOG/build.log" 2>&1; then
      bad "cargo build --lib ($profile, '$name')"; tail -20 "$LOG/build.log"; continue
    fi
    RUST_SO="$ROOT/$outdir/libpremultiply_lib.so"
    if [ ! -f "$RUST_SO" ]; then
      bad "missing $RUST_SO"; continue
    fi
    export PREMULT_RUST_SO="$RUST_SO"
    echo "  Rust .so: $RUST_SO"

    # nm -D symbol diff, independent of the test harness
    cdef=$(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort)
    rdef=$(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort)
    missing=$(comm -23 <(echo "$cdef") <(echo "$rdef"))
    if [ -z "$missing" ]; then
      ok "nm -D symbol diff empty ($(echo "$cdef" | wc -l) C symbol(s))"
    else
      bad "nm -D: Rust .so missing: $(echo "$missing" | tr '\n' ' ')"
    fi

    for t in phase_d_symbols phase_c_errors phase_b_configs; do
      if timeout 600 cargo test --no-default-features ${combo:+--features "$combo"} \
            --test "$t" -- --test-threads=4 > "$LOG/$t.log" 2>&1; then
        ok "$t ($(grep -oE '[0-9]+ passed' "$LOG/$t.log" | head -1))"
      else
        bad "$t"; tail -40 "$LOG/$t.log"
      fi
    done
  done
done

step "Result"
if [ "$FAIL" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAIL"
