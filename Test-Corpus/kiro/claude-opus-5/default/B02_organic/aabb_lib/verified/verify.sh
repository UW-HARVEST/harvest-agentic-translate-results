#!/usr/bin/env bash
# Differential-test driver: builds the C reference .so and the Rust cdylib,
# then runs the whole comparison suite for every valid feature combination in
# both the dev and release profiles.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
C_SRC="$ROOT/c_src"
RS="$ROOT/translation"
LOGS="/tmp/xlate-verify"
mkdir -p "$LOGS"
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  PASS  %s\n' "$*"; }
bad()  { printf '  FAIL  %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations declared in Cargo.toml
# ---------------------------------------------------------------------------
step "Feature enumeration"
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
  }
' "$RS/Cargo.toml")

if [ -z "$FEATURES" ]; then
  echo "  Cargo.toml declares no [features] table."
  # Also confirm CMakeLists.txt exposes no build-time options.
  if grep -Eq '^[[:space:]]*(option|add_definitions|target_compile_definitions)' "$C_SRC/CMakeLists.txt"; then
    bad "CMakeLists.txt appears to declare build options; investigate"
  else
    echo "  CMakeLists.txt declares no options/compile definitions either."
  fi
  echo "  => exactly one configuration to verify."
  COMBOS=("")           # the single, default configuration
else
  echo "  Declared features: $FEATURES"
  # Power set of the declared features.
  mapfile -t FEATS <<<"$FEATURES"
  n=${#FEATS[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (( mask & (1 << b) )); then
        combo="${combo:+$combo,}${FEATS[b]}"
      fi
    done
    COMBOS+=("$combo")
  done
  printf '  %d combination(s)\n' "${#COMBOS[@]}"
fi

# ---------------------------------------------------------------------------
# 2. cargo check for every combination
# ---------------------------------------------------------------------------
step "cargo check, all feature combinations"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default/none>}"
  slug="$(echo "${combo:-none}" | tr ',' '_')"
  if timeout 600 cargo check --manifest-path "$RS/Cargo.toml" \
       --no-default-features ${combo:+--features "$combo"} \
       > "$LOGS/check-$slug.log" 2>&1; then
    ok "cargo check [$label]"
  else
    bad "cargo check [$label] -- see $LOGS/check-$slug.log"
    tail -25 "$LOGS/check-$slug.log"
  fi
done

# ---------------------------------------------------------------------------
# 3. Build the C reference shared library
# ---------------------------------------------------------------------------
step "Build C reference .so"
if (mkdir -p "$C_SRC/build" \
    && cd "$C_SRC/build" \
    && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && timeout 600 cmake --build .) > "$LOGS/c-build.log" 2>&1; then
  C_SO=$(find "$C_SRC/build" -maxdepth 1 -name '*.so' | head -1)
  ok "C .so -> $C_SO"
else
  bad "C build -- see $LOGS/c-build.log"
  tail -25 "$LOGS/c-build.log"
  exit 1
fi

# ---------------------------------------------------------------------------
# 4/5. Build the Rust cdylib and run the differential suite, per combination
#      and per profile. `cargo test` alone does not build a cdylib-only lib
#      target, so build explicitly first.
# ---------------------------------------------------------------------------
for profile_flag in "" "--release"; do
  prof_name="${profile_flag:-dev}"
  for combo in "${COMBOS[@]}"; do
    label="${combo:-<default/none>}"
    slug="$(echo "${combo:-none}" | tr ',' '_')-${prof_name//-/}"

    step "Build cdylib + run tests [features: $label] [profile: ${prof_name#--}]"
    if timeout 600 cargo build --manifest-path "$RS/Cargo.toml" $profile_flag \
         --no-default-features ${combo:+--features "$combo"} \
         > "$LOGS/build-$slug.log" 2>&1; then
      ok "cargo build [$label/${prof_name#--}]"
    else
      bad "cargo build [$label/${prof_name#--}] -- see $LOGS/build-$slug.log"
      tail -25 "$LOGS/build-$slug.log"
      continue
    fi

    # Symbol parity, independent of the test harness.
    RS_SO="$RS/target/$([ -n "$profile_flag" ] && echo release || echo debug)/libaabb_lib.so"
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="t"{print $3}' | sort -u) \
      <(nm -D --defined-only "$RS_SO" | awk '$2=="T"||$2=="t"{print $3}' | sort -u))
    if [ -z "$missing" ]; then
      ok "nm -D symbol parity [$label/${prof_name#--}]"
    else
      bad "symbols missing from Rust .so [$label/${prof_name#--}]: $(echo "$missing" | tr '\n' ' ')"
    fi

    if timeout 600 cargo test --manifest-path "$RS/Cargo.toml" $profile_flag \
         --no-default-features ${combo:+--features "$combo"} \
         > "$LOGS/test-$slug.log" 2>&1; then
      ok "cargo test [$label/${prof_name#--}]  ($(grep -c '^test .* ok$' "$LOGS/test-$slug.log") test cases)"
    else
      bad "cargo test [$label/${prof_name#--}] -- see $LOGS/test-$slug.log"
      grep -E "^(test .* FAILED|---- |thread )" "$LOGS/test-$slug.log" | head -30
    fi
  done
done

# ---------------------------------------------------------------------------
# 6. Cross-check: the C reference must be bit-stable across optimisation
#    levels. If it is, matching it is a real property rather than an artifact
#    of the default (unoptimised) CMake build.
# ---------------------------------------------------------------------------
step "Rust (release) vs C at -O1/-O2/-O3"
ALT="$LOGS/alt-c"
mkdir -p "$ALT"
for opt in O1 O2 O3; do
  if gcc -shared -fPIC "-$opt" -o "$ALT/libc_$opt.so" "$C_SRC/src/lib.c" \
       -I"$C_SRC/include" -lm > "$LOGS/altc-$opt.log" 2>&1; then
    if C_SO_PATH="$ALT/libc_$opt.so" timeout 600 cargo test \
         --manifest-path "$RS/Cargo.toml" --release \
         > "$LOGS/test-alt-$opt.log" 2>&1; then
      ok "differential suite vs C -$opt"
    else
      bad "differential suite vs C -$opt -- see $LOGS/test-alt-$opt.log"
      grep -E "^(test .* FAILED|---- |thread )" "$LOGS/test-alt-$opt.log" | head -20
    fi
  else
    bad "could not build C at -$opt"
  fi
done

step "Result"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAILED"