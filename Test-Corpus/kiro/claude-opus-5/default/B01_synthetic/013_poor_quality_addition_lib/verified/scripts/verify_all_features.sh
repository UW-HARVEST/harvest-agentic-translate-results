#!/usr/bin/env bash
# Verify the Rust translation against the C shared library for every
# build-time feature combination declared in Cargo.toml.
set -uo pipefail

cd "$(dirname "$0")/../.." || exit 1
ROOT="$(pwd)"
CRATE="$ROOT/translation"

# ---------------------------------------------------------------------------
# 1. Build the C ground-truth shared library.
# ---------------------------------------------------------------------------
echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
echo "   ok: c_src/build/libdriver.so"

# ---------------------------------------------------------------------------
# 2. Enumerate every valid feature combination from [features].
#    (Cargo.toml declares no features, so the only configuration is the
#    default/empty one; the power-set logic below covers additions later.)
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f=1; next }
    /^\[/           { in_f=0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

COMBOS=("")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  total=$((1 << n))
  COMBOS=()
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "== feature combinations (${#COMBOS[@]}) =="
for c in "${COMBOS[@]}"; do echo "   - ${c:-<none>}"; done

# ---------------------------------------------------------------------------
# 3. cargo check / build / test for each combination, in both profiles.
# ---------------------------------------------------------------------------
FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  if [ -n "$combo" ]; then
    FEATFLAGS=(--no-default-features --features "$combo")
  else
    FEATFLAGS=(--no-default-features)
  fi

  echo
  echo "== combo: $label =="

  for step in check build; do
    if ! ( cd "$CRATE" && timeout 600 cargo "$step" "${FEATFLAGS[@]}" >/tmp/cargo_$step.log 2>&1 ); then
      echo "   cargo $step FAILED"; tail -30 /tmp/cargo_$step.log; FAIL=1; continue
    fi
    echo "   cargo $step ok"
  done

  # debug-profile cdylib vs C
  if ( cd "$CRATE" && timeout 600 cargo test "${FEATFLAGS[@]}" >/tmp/cargo_test.log 2>&1 ); then
    echo "   cargo test (debug cdylib) ok"
  else
    echo "   cargo test (debug cdylib) FAILED"; tail -40 /tmp/cargo_test.log; FAIL=1
  fi

  # release-profile cdylib (opt-level 3, panic=abort) vs C
  if ! ( cd "$CRATE" && timeout 600 cargo build --release "${FEATFLAGS[@]}" >/tmp/cargo_rel.log 2>&1 ); then
    echo "   cargo build --release FAILED"; tail -30 /tmp/cargo_rel.log; FAIL=1
  elif ( cd "$CRATE" \
         && RUST_DRIVER_SO="$CRATE/target/release/libdriver.so" \
            timeout 600 cargo test "${FEATFLAGS[@]}" >/tmp/cargo_test_rel.log 2>&1 ); then
    echo "   cargo test (release cdylib) ok"
  else
    echo "   cargo test (release cdylib) FAILED"; tail -40 /tmp/cargo_test_rel.log; FAIL=1
  fi

  # symbol parity, checked directly as well as in-test
  for profile in debug release; do
    so="$CRATE/target/$profile/libdriver.so"
    [ -f "$so" ] || continue
    missing=$(comm -23 \
      <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$so" | awk '{print $3}' | sort -u))
    if [ -n "$missing" ]; then
      echo "   symbols MISSING from $profile cdylib:"; echo "$missing" | sed 's/^/     /'; FAIL=1
    else
      echo "   symbol parity ($profile) ok"
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL COMBINATIONS PASS"
else
  echo "FAILURES PRESENT"
fi
exit "$FAIL"
