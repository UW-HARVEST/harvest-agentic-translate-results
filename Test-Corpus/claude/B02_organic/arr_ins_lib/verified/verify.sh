#!/usr/bin/env bash
# Full differential verification driver.
#
# Phase A : enumerate build configurations, cargo check each one
# Phase B : valid-path differential tests   (CONFIGS.md)
# Phase C : error-path differential tests   (ERRORS.md)
# Phase D : symbol parity + repeat B/C for every configuration
#
# Usage: ./verify.sh [--quick]
set -u -o pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
LOGDIR="${TMPDIR:-/tmp}/stbds-verify"
mkdir -p "$LOGDIR"
FAILED=0

say()  { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mOK\033[0m   %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------- Phase A
say "Phase A - build-configuration surface"

# Every valid cargo feature combination. There is no [features] section, so the
# powerset of the declared features is the single empty combination.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml
)
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "   Cargo.toml declares no [features] -> exactly 1 combination: <none>"
  COMBOS=("")
else
  # powerset
  COMBOS=("")
  for f in "${FEATURES[@]}"; do
    new=()
    for c in "${COMBOS[@]}"; do
      new+=("$c")
      if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    COMBOS=("${new[@]}")
  done
  echo "   features: ${FEATURES[*]}"
  echo "   combinations: ${#COMBOS[@]}"
fi

grep -qE '^\s*(option|CMAKE_BUILD_TYPE|add_definitions|target_compile_definitions)' c_src/CMakeLists.txt \
  && echo "   NOTE: CMakeLists has configurable options - review!" \
  || echo "   c_src/CMakeLists.txt declares no options -> 1 C configuration"

for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  if timeout 600 cargo check --no-default-features --features "$combo" \
        > "$LOGDIR/check-${combo:-none}.log" 2>&1; then
    ok "cargo check --no-default-features --features '$label'"
  else
    bad "cargo check --no-default-features --features '$label'"
    tail -30 "$LOGDIR/check-${combo:-none}.log"
  fi
done

# ---------------------------------------------------------------- build both
say "Building the C shared library"
mkdir -p c_src/build
if (cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build .) > "$LOGDIR/cmake.log" 2>&1; then
  ok "c_src/build/libtranslated_rust.so"
else
  bad "C build"; tail -30 "$LOGDIR/cmake.log"; exit 1
fi
C_SO="$ROOT/c_src/build/libtranslated_rust.so"

say "Building the Rust shared library (release and debug)"
for profile in release debug; do
  args=(build --no-default-features)
  [ "$profile" = release ] && args+=(--release)
  if timeout 600 cargo "${args[@]}" > "$LOGDIR/build-$profile.log" 2>&1; then
    ok "target/$profile/libarr_ins_lib.so"
  else
    bad "cargo build ($profile)"; tail -30 "$LOGDIR/build-$profile.log"; exit 1
  fi
done

# ---------------------------------------------------------------- Phase D.1
say "Phase D - exported symbol parity (nm -D)"
nm -D --defined-only "$C_SO"                     | awk '{print $3}' | sort > "$LOGDIR/c.syms"
nm -D --defined-only target/release/libarr_ins_lib.so | awk '{print $3}' | sort > "$LOGDIR/rust.syms"
missing=$(comm -23 "$LOGDIR/c.syms" "$LOGDIR/rust.syms")
extra=$(comm -13   "$LOGDIR/c.syms" "$LOGDIR/rust.syms")
printf '   C exports  : %s\n   Rust exports: %s\n' \
  "$(wc -l < "$LOGDIR/c.syms")" "$(wc -l < "$LOGDIR/rust.syms")"
if [ -z "$missing" ] && [ -z "$extra" ]; then
  ok "symbol diff is EMPTY"
else
  [ -n "$missing" ] && bad "missing from Rust .so: $(echo "$missing" | tr '\n' ' ')"
  [ -n "$extra" ]   && bad "extra in Rust .so: $(echo "$extra" | tr '\n' ' ')"
fi
und=$(nm -D --undefined-only target/release/libarr_ins_lib.so \
      | awk '{print $NF}' \
      | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^_Unwind_|^gettid$|^statx$' || true)
if [ -z "$und" ]; then
  ok "no non-libc undefined symbols in the Rust .so"
else
  bad "unexpected undefined symbols: $(echo "$und" | tr '\n' ' ')"
fi

# ---------------------------------------------------------------- Phase B+C
say "Phase B + C - differential tests, every configuration x every Rust profile"
for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    label="features='${combo:-<none>}' rust_so=$profile"
    export C_SO="$C_SO"
    export RUST_SO="$ROOT/target/$profile/libarr_ins_lib.so"
    log="$LOGDIR/test-${combo:-none}-$profile.log"
    if timeout 600 cargo test --no-default-features --features "$combo" \
          -- --test-threads=4 > "$log" 2>&1; then
      n=$(grep -c '^test .* ok$' "$log")
      ok "cargo test [$label] - $n tests passed"
    else
      bad "cargo test [$label]"
      grep -E 'FAILED|panicked|DIVERGENCE|signal|test result' "$log" | head -40
    fi
  done
done

# ---------------------------------------------------------------- completion
say "Completion gate"
for f in SYMBOLS.md ERRORS.md CONFIGS.md; do
  [ -f "$f" ] && ok "$f present ($(wc -l < "$f") lines)" || bad "$f missing"
done
# count DATA rows only (`| <n> | ... | [ ] |`), not the markdown header row
unchecked_e=$(grep -cE '^\| [0-9]+ \|.*\| \[ \] \|$' ERRORS.md  || true)
unchecked_c=$(grep -cE '^\| [0-9]+ \|.*\| \[ \] \|$' CONFIGS.md || true)
rows_e=$(grep -cE '^\| [0-9]+ \|' ERRORS.md  || true)
rows_c=$(grep -cE '^\| [0-9]+ \|' CONFIGS.md || true)
echo "   ERRORS.md  data rows: $rows_e"
echo "   CONFIGS.md data rows: $rows_c"
[ "$unchecked_e" = 0 ] && ok "ERRORS.md: 0 unchecked rows"  || bad "ERRORS.md: $unchecked_e unchecked rows"
[ "$unchecked_c" = 0 ] && ok "CONFIGS.md: 0 unchecked rows" || bad "CONFIGS.md: $unchecked_c unchecked rows"

if [ "$FAILED" = 0 ]; then
  printf '\n\033[1;32mALL CHECKS PASSED\033[0m  (logs in %s)\n' "$LOGDIR"
else
  printf '\n\033[1;31mVERIFICATION FAILED\033[0m  (logs in %s)\n' "$LOGDIR"
fi
exit "$FAILED"
