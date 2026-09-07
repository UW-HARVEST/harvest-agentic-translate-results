#!/usr/bin/env bash
# Full C-vs-Rust differential verification sweep (Phases A–D).
#
# Builds the C shared library, then for EVERY cargo feature combination and for
# EVERY Rust build profile, rebuilds the Rust cdylib, diffs `nm -D` against the C
# `.so`, and runs the whole differential test suite against that exact artifact.
#
# Usage: ./verify.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
C_SO="$ROOT/c_src/build/libStaticAlias.so"
TIMEOUT=600
FAILURES=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILURES=$((FAILURES + 1)); }

# ---------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout $TIMEOUT cmake --build . >/dev/null ) || { fail "C build"; exit 1; }
[[ -f "$C_SO" ]] || { fail "missing $C_SO"; exit 1; }
echo "ok: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations mechanically from Cargo.toml: the empty set,
# each individual feature, and the all-features set.
step "Enumerating cargo feature combinations from Cargo.toml"
FEATURES=$(awk '
  /^\[features\]/ { inf = 1; next }
  /^\[/           { inf = 0 }
  inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                    if (a[1] != "default") print a[1] }
' "$HERE/Cargo.toml" | sort -u)

COMBOS=()
if [[ -z "$FEATURES" ]]; then
  echo "no [features] declared -> the single default configuration is the whole surface"
  COMBOS+=("DEFAULT")
  COMBOS+=("NODEFAULT")            # --no-default-features (must be equivalent here)
else
  COMBOS+=("DEFAULT" "NODEFAULT")
  for f in $FEATURES; do COMBOS+=("F:$f"); done
  COMBOS+=("F:$(echo "$FEATURES" | paste -sd, -)")
fi
printf 'combinations: %s\n' "${COMBOS[*]}"

# ---------------------------------------------------------------------------
run_combo() {
  local combo="$1" profile="$2"
  local -a fargs=()
  case "$combo" in
    DEFAULT)    ;;
    NODEFAULT)  fargs=(--no-default-features) ;;
    F:*)        fargs=(--no-default-features --features "${combo#F:}") ;;
  esac

  local -a pargs=() outdir="debug"
  if [[ "$profile" == "release" ]]; then pargs=(--release); outdir="release"; fi

  step "combo=$combo profile=$profile"

  ( cd "$HERE" && timeout $TIMEOUT cargo build "${pargs[@]}" "${fargs[@]}" ) >/dev/null 2>&1 \
    || { fail "cargo build $combo/$profile"; return; }

  local rust_so="$HERE/target/$outdir/libStaticAlias.so"
  [[ -f "$rust_so" ]] || { fail "missing $rust_so"; return; }

  # --- symbol parity: the diff must be empty -------------------------------
  local missing
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort) \
    <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort))
  if [[ -n "$missing" ]]; then
    fail "symbols missing from Rust .so ($combo/$profile): $missing"
  else
    echo "symbol parity: OK (0 missing)"
  fi
  if ldd -r "$rust_so" 2>&1 | grep -qi 'undefined symbol'; then
    fail "unresolved symbols in $rust_so"
  else
    echo "unresolved symbols: none"
  fi

  # --- full differential suite against THIS artifact -----------------------
  ( cd "$HERE" \
    && STATICALIAS_C_SO="$C_SO" STATICALIAS_RUST_SO="$rust_so" \
       timeout $TIMEOUT cargo test "${fargs[@]}" -- --test-threads=1 ) 2>&1 \
    | grep -E 'running |test result|FAILED' \
    || fail "cargo test $combo/$profile"

  ( cd "$HERE" \
    && STATICALIAS_C_SO="$C_SO" STATICALIAS_RUST_SO="$rust_so" \
       timeout $TIMEOUT cargo test "${fargs[@]}" -- --test-threads=1 ) >/dev/null 2>&1 \
    || fail "differential tests failed for $combo/$profile"
}

for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    run_combo "$combo" "$profile"
  done
done

# ---------------------------------------------------------------------------
step "SUMMARY"
if [[ $FAILURES -eq 0 ]]; then
  echo "ALL CONFIGURATIONS PASSED"
  exit 0
fi
echo "$FAILURES failure(s)"
exit 1
