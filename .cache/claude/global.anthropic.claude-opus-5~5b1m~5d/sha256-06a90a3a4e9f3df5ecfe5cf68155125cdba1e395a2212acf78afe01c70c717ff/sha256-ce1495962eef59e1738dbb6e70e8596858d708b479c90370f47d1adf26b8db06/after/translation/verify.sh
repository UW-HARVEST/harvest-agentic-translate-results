#!/usr/bin/env bash
# Full differential-verification matrix for the C -> Rust translation.
#
#   ./verify.sh            run everything
#   SPEC_RAY_N=200000 ./verify.sh   more randomized inputs per CONFIGS.md row
#
# Matrix:
#   * every cargo feature combination (the powerset of [features]; this crate
#     declares none, so that is `default` + `--no-default-features`)
#   * Rust cdylib built in the dev AND the release profile
#   * compared against the C reference built by the documented cmake command
#     (no CMAKE_BUILD_TYPE => -O0) AND against a -O2 build of the same C source
#
# Every combination runs the whole suite: symbol parity, Phase B (CONFIGS.md)
# and Phase C (ERRORS.md).
set -uo pipefail
cd "$(dirname "$0")" || exit 1
ROOT=$(cd .. && pwd)
# Prefer --offline (this sandbox has no crates.io access); fall back to a normal
# cargo if the dependency cache is not populated.
if cargo metadata --offline --format-version 1 >/dev/null 2>&1; then
  CARGO="cargo --offline"
else
  CARGO="cargo"
fi
FAILED=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL\033[0m %s\n' "$*"; FAILED=$((FAILED + 1)); }
ok()   { printf '\033[32m ok \033[0m %s\n' "$*"; }

########################## 1. the two C references ###########################
step "building the C reference (documented cmake command, no CMAKE_BUILD_TYPE => -O0)"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { fail "C -O0 build"; exit 1; }
C_O0=$(ls "$ROOT"/c_src/build/*.so | head -1)
ok "C -O0: $C_O0"

step "building a second C reference at -O2 (for the NaN-payload policy test)"
cmake -S "$ROOT/c_src" -B target/cref-O2 -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build target/cref-O2 >/dev/null || fail "C -O2 build"
C_O2=$(ls target/cref-O2/*.so 2>/dev/null | head -1)
ok "C -O2: ${C_O2:-<none>}"

########################## 2. feature combinations ###########################
# The powerset of the [features] table.  Kept mechanical so a future feature is
# picked up automatically instead of being silently skipped.
mapfile -t FEATS < <(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/ {inside=0}
  inside && /^[A-Za-z0-9_-]+[ \t]*=/ {sub(/[ \t]*=.*/, ""); print}
' Cargo.toml)
step "cargo features declared: ${#FEATS[@]} (${FEATS[*]:-none})"

COMBO_NAMES=()
COMBO_FLAGS=()
COMBO_NAMES+=("default");            COMBO_FLAGS+=("")
COMBO_NAMES+=("no-default-features"); COMBO_FLAGS+=("--no-default-features")
if [ "${#FEATS[@]}" -gt 0 ]; then
  n=${#FEATS[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (( mask & (1 << i) )) && sel+=("${FEATS[i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBO_NAMES+=("$joined")
    COMBO_FLAGS+=("--no-default-features --features $joined")
  done
fi
echo "feature combinations to verify: ${COMBO_NAMES[*]}"

############################### 3. the matrix ################################
for ci in "${!COMBO_NAMES[@]}"; do
  COMBO=${COMBO_NAMES[$ci]}
  FLAGS=${COMBO_FLAGS[$ci]}

  for PROFILE in dev release; do
    if [ "$PROFILE" = release ]; then
      PROFILE_FLAG=--release
      SO=target/release/libspec_ray_lib.so
    else
      PROFILE_FLAG=
      SO=target/debug/libspec_ray_lib.so
    fi

    step "[features=$COMBO] [profile=$PROFILE] building the cdylib"
    # shellcheck disable=SC2086
    $CARGO build $FLAGS $PROFILE_FLAG >/dev/null 2>&1 || { fail "build $COMBO/$PROFILE"; continue; }
    ok "$SO"

    # ---- symbol parity (Phase A / Phase D) ----
    nm -D --defined-only --format=posix "$C_O0" | awk '$2=="T" && $1 !~ /^_/ {print $1}' | sort > target/.c_syms
    nm -D --defined-only --format=posix "$SO"   | awk '$2=="T" && $1 !~ /^_/ {print $1}' | sort > target/.r_syms
    MISSING=$(comm -23 target/.c_syms target/.r_syms)
    EXTRA=$(comm -13 target/.c_syms target/.r_syms)
    if [ -n "$MISSING" ]; then
      fail "symbols exported by the C .so but MISSING from the Rust .so: $(echo $MISSING | tr '\n' ' ')"
    else
      ok "symbol parity: $(wc -l < target/.c_syms) / $(wc -l < target/.r_syms) exports, 0 missing"
    fi
    [ -n "$EXTRA" ] && echo "     (extra Rust exports, harmless: $(echo $EXTRA | tr '\n' ' '))"
    UNDEF=$(nm -D --undefined-only --format=posix "$SO" | awk '{print $1}' \
            | grep -v -E '^(_|abort|bcmp|calloc|close|free|fstat|getcwd|getenv|gettid|lseek|malloc|memcpy|memmove|memset|mmap|munmap|open|posix_memalign|pthread_|read|readlink|realloc|realpath|stat|statx|strlen|syscall|write|writev|dl_iterate_phdr)' )
    if [ -n "$UNDEF" ]; then
      fail "non-libc undefined symbols in the Rust .so: $UNDEF"
    else
      ok "no non-libc undefined symbols"
    fi

    # ---- differential suites against both C builds ----
    for CREF in "$C_O0" "$C_O2"; do
      [ -z "$CREF" ] && continue
      LABEL=$([ "$CREF" = "$C_O0" ] && echo "C(-O0, reference)" || echo "C(-O2)")
      for SUITE in smoke_probe phase_b_valid phase_c_errors nan_payload_policy; do
        # shellcheck disable=SC2086
        OUT=$(SPEC_RAY_C_SO="$CREF" SPEC_RAY_RUST_SO="$PWD/$SO" \
              timeout 900 $CARGO test $FLAGS $PROFILE_FLAG --test $SUITE -- --test-threads=1 2>&1)
        if echo "$OUT" | grep -q "^test result: ok"; then
          SUM=$(echo "$OUT" | grep -oP '\d+ passed' | head -1)
          ok "[features=$COMBO][$PROFILE] $SUITE vs $LABEL: $SUM"
        else
          fail "[features=$COMBO][$PROFILE] $SUITE vs $LABEL"
          echo "$OUT" | grep -E "panicked|FAILED|hard mismatch|^test .* FAILED" | head -20
        fi
      done
    done
  done
done

step "SUMMARY"
if [ "$FAILED" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m across every feature combination, both Rust\n'
  printf 'profiles and both C reference builds.\n'
  exit 0
else
  printf '\033[31m%d CHECK(S) FAILED\033[0m\n' "$FAILED"
  exit 1
fi
