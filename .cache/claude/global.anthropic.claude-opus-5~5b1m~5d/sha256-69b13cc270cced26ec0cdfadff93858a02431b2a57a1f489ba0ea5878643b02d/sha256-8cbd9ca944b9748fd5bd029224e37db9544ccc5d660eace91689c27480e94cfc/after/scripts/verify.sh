#!/usr/bin/env bash
# Full differential verification: builds the C .so and the Rust .so, then runs
# every phase of the test suite under EVERY cargo feature combination.
#
# Usage:  scripts/verify.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CARGO_FLAGS="--offline"
FAIL=0

note() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
note "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls -1 "$ROOT"/c_src/build/*.so | head -n1)"
echo "C  .so: $C_SO"

# --------------------------------------------------------------------------
note "Enumerating cargo feature combinations"
# Features declared in Cargo.toml's [features] table (excluding `default`).
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/          {inf=0}
  inf && /=/     {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}
' "$CRATE/Cargo.toml")

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "no [features] table -> only the default (empty) feature set exists"
  COMBOS+=("DEFAULT")
  COMBOS+=("NODEFAULT")
else
  echo "declared features: $FEATURES"
  COMBOS+=("DEFAULT")
  COMBOS+=("NODEFAULT")
  # Power set of the declared features, with --no-default-features.
  feats=($FEATURES)
  n=${#feats[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="$combo,${feats[$i]}"; fi
    done
    COMBOS+=("FEATS:${combo#,}")
  done
fi

# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT)    args=() ; label="default features" ;;
    NODEFAULT)  args=(--no-default-features) ; label="--no-default-features" ;;
    FEATS:*)    args=(--no-default-features --features "${combo#FEATS:}")
                label="--no-default-features --features ${combo#FEATS:}" ;;
  esac

  note "cargo check ($label)"
  ( cd "$CRATE" && cargo check $CARGO_FLAGS "${args[@]}" ) || { FAIL=1; continue; }

  # Debug cdylib (built implicitly by cargo test) --------------------------
  note "cargo test ($label, debug cdylib)"
  ( cd "$CRATE" && CHARINBUF_C_SO="$C_SO" \
      cargo test $CARGO_FLAGS "${args[@]}" -- --test-threads=1 ) || FAIL=1

  # Release cdylib -- what actually ships (panic = "abort") ----------------
  note "cargo build --release ($label)"
  ( cd "$CRATE" && cargo build $CARGO_FLAGS --release "${args[@]}" ) || { FAIL=1; continue; }
  R_SO="$(ls -1 "$CRATE"/target/release/libcharinbuf_lib.so | head -n1)"
  note "cargo test ($label, RELEASE cdylib $R_SO)"
  ( cd "$CRATE" && CHARINBUF_C_SO="$C_SO" CHARINBUF_RUST_SO="$R_SO" \
      cargo test $CARGO_FLAGS "${args[@]}" -- --test-threads=1 ) || FAIL=1
done

# --------------------------------------------------------------------------
note "Symbol diff (C .so vs Rust .so)"
R_SO="$(ls -1 "$CRATE"/target/release/libcharinbuf_lib.so | head -n1)"
diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort) \
  && echo "symbol sets are IDENTICAL" || { echo "SYMBOL DIFF NON-EMPTY"; FAIL=1; }

note "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL PHASES PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
