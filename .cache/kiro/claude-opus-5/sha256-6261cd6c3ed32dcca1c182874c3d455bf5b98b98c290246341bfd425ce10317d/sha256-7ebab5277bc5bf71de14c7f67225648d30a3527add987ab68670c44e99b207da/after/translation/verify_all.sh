#!/usr/bin/env bash
# Verify the translation against the C library for every feature combination
# declared in Cargo.toml, in both dev and release profiles.
set -uo pipefail
cd "$(dirname "$0")"

# --- enumerate features -------------------------------------------------------
# Read the [features] table from Cargo.toml (keys only, ignoring "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "Declared features: ${#FEATURES[@]} ${FEATURES[*]-}"

# Every subset of FEATURES, as comma-separated strings ("" == no features).
COMBOS=("")
for f in "${FEATURES[@]-}"; do
  [ -z "$f" ] && continue
  existing=("${COMBOS[@]}")
  for c in "${existing[@]}"; do
    if [ -z "$c" ]; then COMBOS+=("$f"); else COMBOS+=("$c,$f"); fi
  done
done
echo "Feature combinations to verify: ${#COMBOS[@]}"

# --- build the C reference library -------------------------------------------
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

rc=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  featargs=(--no-default-features)
  [ -n "$combo" ] && featargs+=(--features "$combo")

  echo "=============================================================="
  echo "features: $label"

  if ! timeout 600 cargo check "${featargs[@]}" >/dev/null 2>&1; then
    echo "  cargo check          FAILED"; rc=1
    timeout 600 cargo check "${featargs[@]}" 2>&1 | tail -20
    continue
  fi
  echo "  cargo check          ok"

  for profile in dev release; do
    buildargs=("${featargs[@]}")
    testargs=("${featargs[@]}")
    outdir=debug
    if [ "$profile" = release ]; then
      buildargs+=(--release); testargs+=(--release); outdir=release
    fi

    if ! timeout 600 cargo build --lib "${buildargs[@]}" >/dev/null 2>&1; then
      echo "  build/$profile        FAILED"; rc=1; continue
    fi

    # Exported-symbol parity: every dynamic symbol the C .so defines must also
    # be defined by the Rust .so under the exact same name.
    missing=$(comm -23 \
      <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "target/$outdir/libdriver.so" | awk '{print $NF}' | sort -u))
    if [ -n "$missing" ]; then
      echo "  symbols/$profile      MISSING: $(echo "$missing" | tr '\n' ' ')"; rc=1
    else
      echo "  symbols/$profile      ok"
    fi

    if timeout 600 cargo test "${testargs[@]}" >/tmp/t.$$.log 2>&1; then
      echo "  tests/$profile        ok ($(grep -c "\.\.\. ok" /tmp/t.$$.log) test cases)"
    else
      echo "  tests/$profile        FAILED"; rc=1; tail -30 /tmp/t.$$.log
    fi
    rm -f /tmp/t.$$.log
  done
done

echo "=============================================================="
[ $rc -eq 0 ] && echo "ALL FEATURE COMBINATIONS VERIFIED" || echo "FAILURES PRESENT"
exit $rc
