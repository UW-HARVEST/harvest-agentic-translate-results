#!/usr/bin/env bash
# Run the full differential suite under every cargo feature combination and
# every profile.
#
# IMPORTANT: `cargo test` compiles the lib target as a test binary but does NOT
# re-link the `cdylib` artifact that the tests dlopen. Every `cargo test` here is
# therefore preceded by an explicit `cargo build` of the same profile+features,
# so the `.so` under test is always current. (tests/common/mod.rs additionally
# refuses to run against a `.so` older than src/, as a belt-and-braces guard.)
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
C_SO="$ROOT/../c_src/build/libdriver.so"

# ---------------------------------------------------------------------------
# 1. Build the C reference library.
# ---------------------------------------------------------------------------
if [[ ! -f "$C_SO" ]]; then
  echo "== building C reference library =="
  mkdir -p "$ROOT/../c_src/build"
  ( cd "$ROOT/../c_src/build" \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
echo "C reference: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations declared in Cargo.toml.
#    This crate declares no [features] and has no optional dependencies, so the
#    complete set is the default one; the --no-default-features and
#    --all-features spellings are run anyway to prove they are equivalent.
# ---------------------------------------------------------------------------
declare -a FEATURE_ARGS=(
  ""
  "--no-default-features"
  "--all-features"
)

echo "== declared features in Cargo.toml =="
if grep -q '^\[features\]' Cargo.toml; then
  sed -n '/^\[features\]/,/^\[/p' Cargo.toml
else
  echo "(none - single configuration)"
fi

# ---------------------------------------------------------------------------
# 3. Build + test each (profile, feature-combo) pair.
# ---------------------------------------------------------------------------
FAILED=0
for PROFILE in release debug; do
  if [[ "$PROFILE" == "release" ]]; then PROFILE_FLAG="--release"; else PROFILE_FLAG=""; fi
  for FEAT in "${FEATURE_ARGS[@]}"; do
    LABEL="profile=$PROFILE features=${FEAT:-<default>}"
    echo
    echo "=============================================================="
    echo "== $LABEL"
    echo "=============================================================="

    # Re-link the cdylib for THIS configuration first.
    if ! timeout 600 cargo build --offline $PROFILE_FLAG $FEAT 2>&1 | tail -3; then
      echo "BUILD FAILED: $LABEL"; FAILED=1; continue
    fi

    # --no-fail-fast so a failure in one test binary does not hide the others.
    if timeout 600 cargo test --offline --no-fail-fast $PROFILE_FLAG $FEAT 2>&1 | tail -60; then
      echo "PASS: $LABEL"
    else
      echo "FAIL: $LABEL"; FAILED=1
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. Symbol diff must be empty (Phase D gate), reported independently of the
#    in-test assertion.
# ---------------------------------------------------------------------------
echo
echo "== symbol diff (C vs Rust exported dynamic symbols) =="
RUST_SO="$ROOT/target/release/libdriver.so"
diff <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sed 's/@.*//' | sort -u) \
     <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u) \
  && echo "symbol diff EMPTY (ok)" || { echo "symbol diff NON-EMPTY"; FAILED=1; }

echo
if [[ "$FAILED" -eq 0 ]]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAILED"
