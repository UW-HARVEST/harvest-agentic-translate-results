#!/usr/bin/env bash
# Full verification sweep: builds both libraries in every relevant
# configuration and runs the whole differential suite against each pair.
#
#   ./run_all.sh
#
# Phases A-D artifacts: SYMBOLS.md, ERRORS.md, CONFIGS.md
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
OUT="$HERE/target/verify"
mkdir -p "$OUT"

CARGO_FLAGS="--offline"
export RUST_TEST_THREADS=1

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Feature combinations, extracted from Cargo.toml (not hard-coded).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /=/   {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}
  ' "$HERE/Cargo.toml"
)

note "feature combinations"
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default."
  COMBOS=("__default__" "__nodefault__")
else
  COMBOS=("__default__" "__nodefault__")
  for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
  # all pairs
  n=${#FEATURES[@]}
  for ((i = 0; i < n; i++)); do
    for ((j = i + 1; j < n; j++)); do
      COMBOS+=("${FEATURES[i]},${FEATURES[j]}")
    done
  done
  # everything at once
  COMBOS+=("$(IFS=,; echo "${FEATURES[*]}")")
fi
printf '  %s\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. Build the C library twice: as CMakeLists specifies (no explicit -O, i.e.
#    -O0) and with -O2, because the C relies on signed-overflow wrap-around and
#    an optimising compiler is entitled to treat that differently. The Rust must
#    match the C's *actual* behaviour in both builds.
#    Neither build writes inside c_src except the pre-existing build/ dir.
# ---------------------------------------------------------------------------
build_c() {
  local dir="$1" flags="$2"
  cmake -S "$ROOT/c_src" -B "$dir" \
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="$flags" >"$OUT/cmake.log" 2>&1 || { cat "$OUT/cmake.log"; return 1; }
  cmake --build "$dir" --clean-first >>"$OUT/cmake.log" 2>&1 || { cat "$OUT/cmake.log"; return 1; }
  find "$dir" -maxdepth 1 -name 'lib*.so' | head -n1
}

note "building the C shared library"
C_O0="$(build_c "$OUT/c_O0" "-O0")"           || { echo "C -O0 build FAILED"; exit 1; }
C_O2="$(build_c "$OUT/c_O2" "-O2")"           || { echo "C -O2 build FAILED"; exit 1; }
C_DEFAULT="$(build_c "$OUT/c_default" "")"    || { echo "C default build FAILED"; exit 1; }
echo "  -O0     : $C_O0"
echo "  -O2     : $C_O2"
echo "  default : $C_DEFAULT"

# ---------------------------------------------------------------------------
# 3. For every feature combo x C build x Rust profile: build, diff symbols,
#    run the suite.
# ---------------------------------------------------------------------------
run_suite() {
  local label="$1" cflags="$2" clib="$3" rprofile="$4" rlib="$5"
  note "TEST $label"

  # --- symbol diff (Phase D) ---
  local cs rs missing
  cs="$(nm -D --defined-only --format=posix "$clib" | awk '$2 ~ /^[TDBRWVGS]$/ {print $1}' \
        | grep -Ev '^(_init|_fini|__bss_start|_edata|_end|_ITM_|__cxa_|__gmon_)' | sort -u)"
  rs="$(nm -D --defined-only --format=posix "$rlib" | awk '$2 ~ /^[TDBRWVGS]$/ {print $1}' | sort -u)"
  missing="$(comm -23 <(echo "$cs") <(echo "$rs"))"
  if [ -n "$missing" ]; then
    echo "SYMBOL DIFF NOT EMPTY -- Rust is missing:"; echo "$missing"; fail=1
  else
    echo "symbol diff: EMPTY ($(echo "$cs" | wc -l) C API symbols all present in Rust)"
  fi

  # --- differential suite (full log kept; only a digest printed) ---
  local log="$OUT/$(echo "$label" | tr ' =/' '___').log"
  DIFFTEST_C_LIB="$clib" DIFFTEST_RUST_LIB="$rlib" \
    timeout 600 cargo test $CARGO_FLAGS $cflags $rprofile >"$log" 2>&1
  local rc=$?

  local bins passed failed
  bins=$(grep -c 'Running tests/' "$log")
  passed=$(grep -cE '^test .* \.\.\. ok$' "$log")
  failed=$(grep -oE '[0-9]+ failed' "$log" | awk '{s+=$1} END {print s+0}')

  printf 'test binaries=%s  tests passed=%s  tests failed=%s  exit=%s\n' \
         "$bins" "$passed" "$failed" "$rc"

  if [ "$rc" -ne 0 ] || [ "$bins" -ne "$EXPECTED_BINS" ] || [ "$failed" -ne 0 ] \
     || [ "$passed" -ne "$EXPECTED_TESTS" ]; then
    echo "SUITE FAILED for $label (expected $EXPECTED_BINS binaries / $EXPECTED_TESTS passing tests)"
    grep -nE 'FAILED|panicked|^error' "$log" | head -n 20
    fail=1
  fi
}

# How many test binaries and individual tests the suite must run. Derived from
# the files on disk so it cannot silently drift.
EXPECTED_BINS=$(ls "$HERE"/tests/*.rs | wc -l)
EXPECTED_TESTS=$(grep -hcE '^#\[test\]' "$HERE"/tests/*.rs | awk '{s+=$1} END {print s}')
note "suite size"
echo "  $EXPECTED_BINS test binaries, $EXPECTED_TESTS #[test] functions"

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__)    cflags="" ;;
    __nodefault__)  cflags="--no-default-features" ;;
    *)              cflags="--no-default-features --features $combo" ;;
  esac

  for prof in release debug; do
    if [ "$prof" = release ]; then rprofile="--release"; else rprofile=""; fi
    cargo build $CARGO_FLAGS $cflags $rprofile >"$OUT/build.log" 2>&1 \
      || { echo "cargo build failed for $combo/$prof"; cat "$OUT/build.log"; fail=1; continue; }
    rlib="$HERE/target/$prof/liboverunder_lib.so"

    for c in "O0:$C_O0" "O2:$C_O2" "default:$C_DEFAULT"; do
      run_suite "features=$combo rust=$prof c=${c%%:*}" "$cflags" "${c#*:}" "$rprofile" "$rlib"
    done
  done
done

note "RESULT"
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$fail"
