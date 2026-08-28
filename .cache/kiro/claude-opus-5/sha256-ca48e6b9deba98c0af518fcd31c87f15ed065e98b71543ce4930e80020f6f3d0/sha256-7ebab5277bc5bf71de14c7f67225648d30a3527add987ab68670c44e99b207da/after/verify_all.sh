#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every build-time
# configuration: all Cargo feature combinations x several C compiler settings.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
CSRC="$ROOT/c_src"
LOGDIR="/tmp/xlate-verify"
mkdir -p "$LOGDIR"
FAILED=0

# ---------------------------------------------------------------------------
# 1. Enumerate every valid feature combination from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, p, "="); gsub(/[[:space:]]/, "", p[1]); print p[1]
    }
  ' "$CRATE/Cargo.toml"
)
# "default" is not a selectable feature under --no-default-features.
SELECTABLE=()
for f in "${FEATURES[@]:-}"; do
  [[ -n "$f" && "$f" != "default" ]] && SELECTABLE+=("$f")
done

echo "== features declared in Cargo.toml: ${FEATURES[*]:-<none>}"
echo "== selectable features:            ${SELECTABLE[*]:-<none>}"

# Power set of SELECTABLE, as comma-joined strings ("" == no features).
COMBOS=("")
n=${#SELECTABLE[@]}
if (( n > 0 )); then
  COMBOS=()
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then
        combo="${combo:+$combo,}${SELECTABLE[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "== ${#COMBOS[@]} feature combination(s) to verify"
echo

# ---------------------------------------------------------------------------
# 2. Build the C reference in several compiler configurations
# ---------------------------------------------------------------------------
declare -a C_SO_PATHS=()

build_c() {
  local name="$1" build_type="$2" extra="${3:-}"
  local dir="$LOGDIR/cbuild-$name"
  mkdir -p "$dir"
  if ! cmake -S "$CSRC" -B "$dir" \
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_BUILD_TYPE="$build_type" \
        ${extra:+-DCMAKE_C_FLAGS="$extra"} > "$LOGDIR/cmake-$name.log" 2>&1; then
    echo "!! cmake configure failed for $name (see $LOGDIR/cmake-$name.log)"; FAILED=1; return
  fi
  if ! cmake --build "$dir" --clean-first >> "$LOGDIR/cmake-$name.log" 2>&1; then
    echo "!! cmake build failed for $name (see $LOGDIR/cmake-$name.log)"; FAILED=1; return
  fi
  local so
  so="$(find "$dir" -maxdepth 1 -name '*.so' | head -1)"
  if [[ -z "$so" ]]; then
    echo "!! no .so produced for $name"; FAILED=1; return
  fi
  echo "   C build [$name] -> $so"
  C_SO_PATHS+=("$name=$so")
}

echo "== building C reference"
# The default in-tree build (no CMAKE_BUILD_TYPE) is what the task specifies.
if ! (cd "$CSRC" && mkdir -p build && cmake -S "$CSRC" -B build -DCMAKE_POSITION_INDEPENDENT_CODE=ON > "$LOGDIR/cmake-default.log" 2>&1 \
      && cmake --build build >> "$LOGDIR/cmake-default.log" 2>&1); then
  echo "!! default C build failed"; FAILED=1
fi
DEFAULT_SO="$(find "$CSRC/build" -maxdepth 1 -name '*.so' | head -1)"
echo "   C build [default] -> $DEFAULT_SO"
C_SO_PATHS+=("default=$DEFAULT_SO")
build_c "O2"    "Release"        ""
build_c "O0g"   "Debug"          ""
build_c "Os"    "MinSizeRel"     ""
echo

# ---------------------------------------------------------------------------
# 3. cargo check for every combination (both profiles)
# ---------------------------------------------------------------------------
echo "== cargo check"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  for prof in "" "--release"; do
    log="$LOGDIR/check-${combo//,/_}${prof:+-rel}.log"
    if (cd "$CRATE" && cargo check --no-default-features \
          ${combo:+--features "$combo"} --all-targets $prof) > "$log" 2>&1; then
      echo "   ok   check [$label] ${prof:-debug}"
    else
      echo "   FAIL check [$label] ${prof:-debug}  ($log)"; tail -25 "$log"; FAILED=1
    fi
  done
done
# The default feature set must also compile.
for prof in "" "--release"; do
  log="$LOGDIR/check-defaultfeatures${prof:+-rel}.log"
  if (cd "$CRATE" && cargo check --all-targets $prof) > "$log" 2>&1; then
    echo "   ok   check [default features] ${prof:-debug}"
  else
    echo "   FAIL check [default features] ${prof:-debug} ($log)"; tail -25 "$log"; FAILED=1
  fi
done
echo

# ---------------------------------------------------------------------------
# 4. Symbol parity: every symbol the C .so exports, the Rust .so must export
# ---------------------------------------------------------------------------
check_symbols() {
  local combo="$1" prof="$2" c_so="$3" rust_so="$4"
  local label="${combo:-<none>}/${prof}"
  local missing=0
  while read -r sym; do
    [[ -z "$sym" ]] && continue
    if ! nm -D --defined-only "$rust_so" | awk '{print $3}' | grep -qx -- "$sym"; then
      echo "   FAIL symbol [$label] Rust .so is missing '$sym'"
      missing=1; FAILED=1
    fi
  done < <(nm -D --defined-only "$c_so" | awk '{print $3}')
  (( missing == 0 )) && echo "   ok   symbols [$label] all C exports present in Rust .so"
}

# ---------------------------------------------------------------------------
# 5. cargo test for every combination x C configuration
# ---------------------------------------------------------------------------
echo "== cargo build + symbol parity + cargo test"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  for prof in debug release; do
    relflag=""; [[ "$prof" == release ]] && relflag="--release"

    log="$LOGDIR/build-${combo//,/_}-$prof.log"
    if ! (cd "$CRATE" && cargo build --no-default-features \
            ${combo:+--features "$combo"} $relflag) > "$log" 2>&1; then
      echo "   FAIL build [$label/$prof] ($log)"; tail -25 "$log"; FAILED=1; continue
    fi
    RUST_SO="$CRATE/target/$prof/libmerge_sort_lib.so"
    check_symbols "$combo" "$prof" "$DEFAULT_SO" "$RUST_SO"

    for entry in "${C_SO_PATHS[@]}"; do
      cname="${entry%%=*}"; cso="${entry#*=}"
      log="$LOGDIR/test-${combo//,/_}-$prof-$cname.log"
      if C_SO_PATH="$cso" timeout 600 bash -c "cd '$CRATE' && cargo test --no-default-features ${combo:+--features '$combo'} $relflag" > "$log" 2>&1; then
        passed="$(grep -c '^test .* ok$' "$log")"
        echo "   ok   test  [$label/$prof] vs C[$cname]  ($passed tests)"
      else
        echo "   FAIL test  [$label/$prof] vs C[$cname]  ($log)"
        grep -E 'panicked|assertion|FAILED|error' "$log" | head -30
        FAILED=1
      fi
    done
  done
done

echo
if (( FAILED == 0 )); then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "FAILURES DETECTED"
fi
exit $FAILED
