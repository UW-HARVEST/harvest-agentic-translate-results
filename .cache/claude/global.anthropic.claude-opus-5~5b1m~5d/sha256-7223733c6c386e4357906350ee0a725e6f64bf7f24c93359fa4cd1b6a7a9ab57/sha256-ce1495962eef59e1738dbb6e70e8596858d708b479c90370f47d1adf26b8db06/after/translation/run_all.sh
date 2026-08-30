#!/usr/bin/env bash
# Full verification sweep: builds the C reference, then runs the whole
# differential suite under every feature combination × build profile.
#
# Usage: ./run_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
CARGO_FLAGS="--offline"   # the crates.io index is not reachable in this sandbox

# Scratch space: keep it inside the crate so the script works in sandboxes
# where /tmp is not writable.
TMP="$ROOT/target/verify-tmp"
mkdir -p "$TMP"

fail=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad()  { printf '\033[31mFAIL: %s\033[0m\n' "$*"; fail=1; }
ok()   { printf '\033[32mok:   %s\033[0m\n' "$*"; }

# --------------------------------------------------------------------------
# 1. Build the C reference library
# --------------------------------------------------------------------------
note "Building the C reference library"
mkdir -p ../c_src/build
( cd ../c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
ok "libString_Slice.so (C)"

# --------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# --------------------------------------------------------------------------
# Everything between a [features] header and the next [section] header.
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
                                          if (a[1] != "default") print a[1] }
' Cargo.toml | sort -u)

if [ -z "$FEATURES" ]; then
  note "Cargo.toml declares no [features]; the only configurations are the default and empty sets"
  COMBOS=("default" "none")
else
  COMBOS=("default" "none")
  for f in $FEATURES; do COMBOS+=("$f"); done
  # All features together, plus every pair.
  COMBOS+=("$(echo "$FEATURES" | paste -sd, -)")
  for a in $FEATURES; do for b in $FEATURES; do
    [ "$a" \< "$b" ] && COMBOS+=("$a,$b")
  done; done
fi
printf 'feature combinations: %s\n' "${COMBOS[*]}"

# --------------------------------------------------------------------------
# 3. Run the suite for every combination × profile
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default) featflags=() ;;
    none)    featflags=(--no-default-features) ;;
    *)       featflags=(--no-default-features --features "$combo") ;;
  esac

  for profile in debug release; do
    relflag=()
    [ "$profile" = release ] && relflag=(--release)
    label="features=$combo profile=$profile"

    note "$label"

    # The cdylib must exist for the profile the tests will look in.
    if ! cargo build $CARGO_FLAGS "${featflags[@]}" "${relflag[@]}" >/dev/null 2>&1; then
      bad "cargo build ($label)"; continue
    fi
    if ! cargo clippy $CARGO_FLAGS "${featflags[@]}" "${relflag[@]}" \
           --all-targets -- -D warnings > "$TMP/clippy.log" 2>&1; then
      if grep -q "no such command" "$TMP/clippy.log"; then
        printf 'clippy unavailable — skipping lint gate\n'
      else
        printf '\033[33mclippy warnings (%s):\033[0m\n' "$label"; tail -20 "$TMP/clippy.log"
      fi
    fi
    rm -f "$TMP/clippy.log"

    if timeout 600 cargo test $CARGO_FLAGS "${featflags[@]}" "${relflag[@]}" \
         -- --nocapture > "$TMP/test.$profile.log" 2>&1; then
      grep -E '\[(PASS|FAIL)\]|^--- |test result' "$TMP/test.$profile.log"
      if grep -qE '\[FAIL\]|test result: FAILED' "$TMP/test.$profile.log"; then
        bad "$label"
      else
        ok "$label"
      fi
    else
      tail -40 "$TMP/test.$profile.log"
      bad "cargo test ($label)"
    fi
    rm -f "$TMP/test.$profile.log"
  done
done

# --------------------------------------------------------------------------
# 4. Symbol diff, printed for the record
# --------------------------------------------------------------------------
note "Symbol diff (C vs Rust exported symbols)"
for prof in debug release; do
  rso="$ROOT/target/$prof/libString_Slice.so"
  [ -f "$rso" ] || continue
  if diff <(nm -D --defined-only ../c_src/build/libString_Slice.so | awk '{print $NF}' | sort) \
          <(nm -D --defined-only "$rso" | awk '{print $NF}' | sort); then
    ok "symbol diff empty ($prof)"
  else
    bad "symbol diff non-empty ($prof)"
  fi
done

note "RESULT"
if [ "$fail" -eq 0 ]; then
  printf '\033[32mALL CONFIGURATIONS PASSED\033[0m\n'
else
  printf '\033[31mSOME CONFIGURATIONS FAILED\033[0m\n'
fi
exit "$fail"
