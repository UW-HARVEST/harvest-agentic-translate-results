#!/usr/bin/env bash
# Verifies the Rust translation against the C ground truth for EVERY valid
# feature combination declared in translation/Cargo.toml.
#
#   1. enumerate feature combinations from [features]
#   2. cargo check each combination
#   3. build the C shared library
#   4. cargo test each combination (differential tests via libloading)
#   5. diff exported dynamic symbols: C .so vs Rust .so
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOG_DIR="${TMPDIR:-/tmp}/translation-verify"
mkdir -p "$LOG_DIR"

fail=0
note() { printf '\n== %s\n' "$*"; }
ok()   { printf '   PASS  %s\n' "$*"; }
bad()  { printf '   FAIL  %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations.
# ---------------------------------------------------------------------------
# Feature names come from the [features] table; "default" is excluded because it
# is expressed via --no-default-features / explicit feature lists instead.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "", $0); if ($0 != "default") print $0
    }
  ' "$CRATE/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table: the empty set is the only valid configuration.
  COMBOS=("")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && combo+=("${FEATURES[$i]}")
    done
    COMBOS+=("$(IFS=,; echo "${combo[*]}")")
  done
fi

note "feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do printf '   - %s\n' "${c:-<none>}"; done

# ---------------------------------------------------------------------------
# 3. Build the C shared library (ground truth).
# ---------------------------------------------------------------------------
note "building C shared library"
if (cd "$ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && cmake --build .) > "$LOG_DIR/cmake.log" 2>&1; then
  ok "cmake build"
else
  bad "cmake build (see $LOG_DIR/cmake.log)"
  exit 1
fi

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' -type f | sort | head -n1)"
[ -n "$C_SO" ] || { bad "no lib*.so produced by cmake"; exit 1; }
printf '   C .so: %s\n' "$C_SO"

# ---------------------------------------------------------------------------
# 2 + 4 + 5. Per-combination check, test, and symbol diff.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  slug="$(echo "${combo:-none}" | tr ',' '_')"
  args=(--no-default-features)
  [ -n "$combo" ] && args+=(--features "$combo")

  note "feature combination: $label"

  for profile in "" "--release"; do
    pslug="${profile:---debug}"; pslug="${pslug#--}"

    if timeout 600 cargo check --manifest-path "$CRATE/Cargo.toml" \
         "${args[@]}" $profile > "$LOG_DIR/check-$slug-$pslug.log" 2>&1; then
      ok "cargo check ($pslug)"
    else
      bad "cargo check ($pslug) -- see $LOG_DIR/check-$slug-$pslug.log"
      tail -n 20 "$LOG_DIR/check-$slug-$pslug.log"
      continue
    fi

    if timeout 600 cargo test --manifest-path "$CRATE/Cargo.toml" \
         "${args[@]}" $profile > "$LOG_DIR/test-$slug-$pslug.log" 2>&1; then
      ok "cargo test ($pslug)"
    else
      bad "cargo test ($pslug) -- see $LOG_DIR/test-$slug-$pslug.log"
      grep -E 'panicked|assertion|C returned|test result' \
        "$LOG_DIR/test-$slug-$pslug.log" | head -n 20
    fi
  done

  # Symbol parity. The differential test builds the cdylib into a dedicated
  # target dir; compare that freshly built artifact against the C .so.
  RUST_SO="$(find "$CRATE/target/differential-cdylib" -maxdepth 2 \
               -name 'libmax_size_frame_lib.so' -type f 2>/dev/null | sort | head -n1)"
  if [ -z "$RUST_SO" ]; then
    bad "no Rust .so found for symbol comparison"
    continue
  fi

  nm -D --defined-only "$C_SO"    | awk '{print $3}' | grep -v '^_' | sort -u > "$LOG_DIR/c-syms.txt"
  nm -D --defined-only "$RUST_SO" | awk '{print $3}' | grep -v '^_' | sort -u > "$LOG_DIR/rust-syms-$slug.txt"
  missing="$(comm -23 "$LOG_DIR/c-syms.txt" "$LOG_DIR/rust-syms-$slug.txt")"
  if [ -z "$missing" ]; then
    ok "symbol parity ($(wc -l < "$LOG_DIR/c-syms.txt") C symbols all exported by Rust)"
  else
    bad "Rust .so is missing symbols exported by the C .so:"
    printf '%s\n' "$missing" | sed 's/^/         /'
  fi
done

note "result"
if [ "$fail" -eq 0 ]; then
  echo "   ALL FEATURE COMBINATIONS VERIFIED AGAINST C"
else
  echo "   VERIFICATION FAILED"
fi
exit "$fail"
