#!/usr/bin/env bash
# Phase D driver: symbol parity + every Cargo feature combination.
#
# Enumerates the powerset of the features declared in Cargo.toml (excluding
# "default") and, for each combination, runs cargo check, builds the cdylib,
# diffs its exported symbols against the C .so, and runs the full differential
# test suite. Nothing here is hard-coded per configuration.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
FAIL=0

C_SO=../c_src/build/libdriver.so

# ---------------------------------------------------------------- C build ----
if [ ! -f "$C_SO" ]; then
  echo "building the C shared library"
  ( cd ../c_src && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

syms() { nm -D --defined-only "$1" | awk '{print $2, $3}' | sort; }

# --------------------------------------------------- feature enumeration ----
# Parse the [features] table out of Cargo.toml. Absent table => no features.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "features declared in Cargo.toml: $N ${FEATURES[*]:-(none)}"

# Build the list of combinations to test: always the default build, plus (when
# features exist) --no-default-features with every subset of them.
COMBOS=("default")
if [ "$N" -gt 0 ]; then
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("nodefault:$combo")
  done
fi

echo "combinations to verify: ${#COMBOS[@]}"
echo

# ------------------------------------------------------------- per combo ----
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "default" ]; then
    ARGS=()
    LABEL="default features"
  else
    feats="${combo#nodefault:}"
    ARGS=(--no-default-features)
    [ -n "$feats" ] && ARGS+=(--features "$feats")
    LABEL="--no-default-features${feats:+ --features $feats}"
  fi

  echo "=============================================================="
  echo "COMBO: $LABEL"
  echo "=============================================================="

  if ! timeout 600 cargo check "${ARGS[@]}" >/dev/null 2>&1; then
    echo "  cargo check   FAILED"; FAIL=1; continue
  fi
  echo "  cargo check   ok"

  RUST_SO=target/phased/release/libdriver.so
  if ! timeout 600 cargo build --release --lib "${ARGS[@]}" \
        --target-dir target/phased >/dev/null 2>&1; then
    echo "  cargo build   FAILED"; FAIL=1; continue
  fi
  echo "  cargo build   ok"

  # ---- symbol parity: every C symbol must be exported by the Rust .so ----
  MISSING=$(comm -23 <(syms "$C_SO") <(syms "$RUST_SO"))
  if [ -n "$MISSING" ]; then
    echo "  symbol parity FAILED — missing from Rust .so:"; echo "$MISSING" | sed 's/^/      /'
    FAIL=1
  else
    echo "  symbol parity ok ($(syms "$C_SO" | wc -l) C symbol(s), 0 missing)"
  fi

  # ---- undefined non-libc symbols in the Rust .so ----
  UNDEF=$(nm -D --undefined-only "$RUST_SO" | awk '$1 == "U" {print $2}' \
            | grep -v '@GLIBC\|@GCC\|@CXXABI' || true)
  if [ -n "$UNDEF" ]; then
    echo "  undefined non-libc symbols FAILED:"; echo "$UNDEF" | sed 's/^/      /'; FAIL=1
  else
    echo "  undefined     ok (all imports resolve to glibc/libgcc)"
  fi

  # ---- Phases B + C differential tests ----
  if timeout 600 cargo test --no-fail-fast "${ARGS[@]}" >/tmp/difftest.$$ 2>&1; then
    echo "  differential  ok"
    grep -E '^test result' /tmp/difftest.$$ | sed 's/^/      /'
  else
    echo "  differential  FAILED"; FAIL=1
    grep -E '^test .* FAILED|^test result' /tmp/difftest.$$ | sed 's/^/      /'
  fi
  rm -f /tmp/difftest.$$
  echo
done

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then
  echo "PHASE D: PASS — all ${#COMBOS[@]} combination(s) clean"
else
  echo "PHASE D: FAIL"
fi
exit "$FAIL"
