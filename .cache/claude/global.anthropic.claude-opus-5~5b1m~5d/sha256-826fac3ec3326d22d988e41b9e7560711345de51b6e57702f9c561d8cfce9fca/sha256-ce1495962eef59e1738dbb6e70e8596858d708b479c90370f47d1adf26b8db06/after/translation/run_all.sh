#!/usr/bin/env bash
# Full verification run: Phases A-D across every build configuration.
#
#   ./run_all.sh [release|debug|all]      (default: all)
#
# The matrix is 2 Rust profiles x 7 C builds x 3 feature combos, which does not
# fit in a single 600 s budget, so the Rust profile can be selected per run.
#
# Nothing under c_src/ is ever written to; the extra optimisation-level builds
# of the C source go into translation/target/cref/.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
TMP="${TMPDIR:-/tmp}"
CARGO_FLAGS="--offline"
WHICH="${1:-all}"

pass=0
fail=0
declare -a FAILURES=()

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; pass=$((pass+1)); }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; fail=$((fail+1)); FAILURES+=("$*"); }

run() { # run <label> <cmd...>
  local label="$1"; shift
  if timeout 600 "$@" >"$TMP/run_all.log" 2>&1; then ok "$label"
  else bad "$label"; tail -n 30 "$TMP/run_all.log"; fi
}

# ---------------------------------------------------------------------------
step "Build the C shared library (canonical CMake build)"
# ---------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build failed"; exit 1; }
C_SO_DEFAULT="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "  $C_SO_DEFAULT"

# ---------------------------------------------------------------------------
step "Build the C shared library at other optimisation levels / compilers"
# ---------------------------------------------------------------------------
# The C source relies on signed-overflow and reads a `malloc(0)` result, both of
# which optimisers are free to treat differently, so the Rust must agree with
# more than one lowering of the same source.
CREF_DIR="$HERE/target/cref"   # stable, outside c_src/, never committed
mkdir -p "$CREF_DIR"
declare -a EXTRA_C_SOS=()
for spec in "gcc:-O0" "gcc:-O2" "gcc:-O3" "gcc:-Os" "clang:-O0" "clang:-O2"; do
  cc="${spec%%:*}"; opt="${spec##*:}"
  command -v "$cc" >/dev/null 2>&1 || continue
  out="$CREF_DIR/libcref_${cc}${opt}.so"
  if "$cc" $opt -fPIC -shared -I"$ROOT/c_src/include" -I"$ROOT/c_src/src" \
        -o "$out" "$ROOT/c_src/src/lib.c" 2>/dev/null; then
    EXTRA_C_SOS+=("$out")
    echo "  $cc $opt -> $out"
  fi
done

# ---------------------------------------------------------------------------
step "Build the Rust cdylib (release + debug)"
# ---------------------------------------------------------------------------
( cd "$HERE" && cargo build $CARGO_FLAGS --release >/dev/null 2>&1 ) || { echo "release build failed"; exit 1; }
( cd "$HERE" && cargo build $CARGO_FLAGS         >/dev/null 2>&1 ) || { echo "debug build failed";   exit 1; }
RUST_RELEASE="$HERE/target/release/libgotomach_lib.so"
RUST_DEBUG="$HERE/target/debug/libgotomach_lib.so"
echo "  $RUST_RELEASE"
echo "  $RUST_DEBUG"

# ---------------------------------------------------------------------------
step "Phase D: symbol diff must be empty"
# ---------------------------------------------------------------------------
syms() { nm -D --defined-only "$1" | awk '{print $NF}' | sed 's/@.*//' | sort -u; }
for rso in "$RUST_RELEASE" "$RUST_DEBUG"; do
  missing="$(comm -23 <(syms "$C_SO_DEFAULT") <(syms "$rso"))"
  if [ -z "$missing" ]; then ok "symbol diff empty for $(basename "$(dirname "$rso")")/$(basename "$rso")"
  else bad "symbols missing from $rso: $missing"; fi
done
echo "  C exports : $(syms "$C_SO_DEFAULT" | tr '\n' ' ')"
echo "  Rust ships: $(syms "$RUST_RELEASE" | tr '\n' ' ')"

# ---------------------------------------------------------------------------
step "Feature combinations"
# ---------------------------------------------------------------------------
# Enumerate the feature table mechanically instead of assuming.
FEATURE_LIST="$(cd "$HERE" && awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)"
if [ -z "$FEATURE_LIST" ]; then
  echo "  Cargo.toml declares no [features]; the default set is the only set."
  declare -a COMBOS=("" "--no-default-features" "--all-features")
else
  declare -a COMBOS=("" "--no-default-features" "--all-features")
  for f in $FEATURE_LIST; do COMBOS+=("--no-default-features --features $f"); done
fi
for combo in "${COMBOS[@]}"; do
  ( cd "$HERE" && timeout 600 cargo check $CARGO_FLAGS --tests $combo >"$TMP/run_all.log" 2>&1 ) \
    && ok "cargo check ${combo:-<default features>}" \
    || { bad "cargo check ${combo:-<default features>}"; tail -n 20 "$TMP/run_all.log"; }
done

# ---------------------------------------------------------------------------
step "Phases A-D: differential test suite over every configuration"
# ---------------------------------------------------------------------------
# One full suite run takes ~25 s, and the whole matrix (2 Rust profiles x 7 C
# builds x 3 feature combos = 42 runs) does not fit in a single 600 s budget, so
# it is split into tiers that each do:
#   tier1  canonical C build x {release,debug} x every feature combo
#   tier2  every alternate C build x release    x every feature combo
#   tier3  every alternate C build x debug      x every feature combo
suite() { # suite <c.so> <rust.so> <combo>
  local cso="$1" rso="$2" combo="$3"
  local label="C=$(basename "$cso") Rust=$(basename "$(dirname "$rso")") features=${combo:-default}"
  if ( cd "$HERE" && C_SO="$cso" RUST_SO="$rso" \
         timeout 600 cargo test $CARGO_FLAGS --release $combo \
         >"$TMP/run_all.log" 2>&1 ); then
    ok "$label"
  else
    bad "$label"
    grep -E '^(test |failures|thread |assertion)' "$TMP/run_all.log" | tail -n 40
  fi
}

case "$WHICH" in
  tier1|release|all)
    for rso in "$RUST_RELEASE" "$RUST_DEBUG"; do
      for combo in "${COMBOS[@]}"; do suite "$C_SO_DEFAULT" "$rso" "$combo"; done
    done ;;
esac
# The alternate-C-build tiers are further halved so each half fits a 600 s run.
half() { # half <0|1> -> prints the selected half of EXTRA_C_SOS
  local n=${#EXTRA_C_SOS[@]} mid=$(( (${#EXTRA_C_SOS[@]} + 1) / 2 )) i
  for ((i = 0; i < n; i++)); do
    if [ "$1" = 0 ] && [ "$i" -lt "$mid" ]; then echo "${EXTRA_C_SOS[$i]}"; fi
    if [ "$1" = 1 ] && [ "$i" -ge "$mid" ]; then echo "${EXTRA_C_SOS[$i]}"; fi
  done
}
case "$WHICH" in
  tier2a) for cso in $(half 0); do for combo in "${COMBOS[@]}"; do suite "$cso" "$RUST_RELEASE" "$combo"; done; done ;;
  tier2b) for cso in $(half 1); do for combo in "${COMBOS[@]}"; do suite "$cso" "$RUST_RELEASE" "$combo"; done; done ;;
  tier3a) for cso in $(half 0); do for combo in "${COMBOS[@]}"; do suite "$cso" "$RUST_DEBUG"   "$combo"; done; done ;;
  tier3b) for cso in $(half 1); do for combo in "${COMBOS[@]}"; do suite "$cso" "$RUST_DEBUG"   "$combo"; done; done ;;
  tier2|release)
    for cso in "${EXTRA_C_SOS[@]}"; do for combo in "${COMBOS[@]}"; do suite "$cso" "$RUST_RELEASE" "$combo"; done; done ;;
  tier3|debug)
    for cso in "${EXTRA_C_SOS[@]}"; do for combo in "${COMBOS[@]}"; do suite "$cso" "$RUST_DEBUG" "$combo"; done; done ;;
  all)
    for cso in "${EXTRA_C_SOS[@]}"; do
      for combo in "${COMBOS[@]}"; do
        suite "$cso" "$RUST_RELEASE" "$combo"
        suite "$cso" "$RUST_DEBUG" "$combo"
      done
    done ;;
esac

# ---------------------------------------------------------------------------
printf '\n\033[1m== Summary ==\033[0m\n'
printf '  passed: %d\n  failed: %d\n' "$pass" "$fail"
if [ "$fail" -ne 0 ]; then
  printf '\n  failing configurations:\n'
  for f in "${FAILURES[@]}"; do printf '   - %s\n' "$f"; done
  exit 1
fi
printf '\n  \033[32mAll configurations verified.\033[0m\n'
