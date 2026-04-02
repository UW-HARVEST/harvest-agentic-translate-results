#!/bin/bash
# Test all configurations of SPHINCS+ C vs Rust
set -e

BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
C_SRC="$BASE_DIR/c_src"
RUST_DIR="$BASE_DIR"

HASH_BACKENDS=(blake sha2 shake haraka)
SECPARS=(128s 128f 192s 192f 256s 256f)
THASHS=(simple robust)

PASS=0
FAIL=0
SKIP=0
RESULTS=""

for hash in "${HASH_BACKENDS[@]}"; do
  for secpar in "${SECPARS[@]}"; do
    for thash in "${THASHS[@]}"; do
      CONFIG="${hash}_${secpar}_${thash}"
      echo "=== Testing $CONFIG ==="

      # Build C
      BUILD_DIR="$C_SRC/build_${CONFIG}"
      rm -rf "$BUILD_DIR"
      mkdir -p "$BUILD_DIR"
      cd "$BUILD_DIR"
      if ! timeout 120 cmake "$C_SRC" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
          -DHASH_BACKEND="$hash" -DSECPAR="$secpar" -DTHASH="$thash" > /dev/null 2>&1; then
        echo "  CMAKE FAILED - skipping"
        SKIP=$((SKIP + 1))
        RESULTS="$RESULTS\nSKIP $CONFIG (cmake failed)"
        continue
      fi
      if ! timeout 120 cmake --build . > /dev/null 2>&1; then
        echo "  BUILD FAILED - skipping"
        SKIP=$((SKIP + 1))
        RESULTS="$RESULTS\nSKIP $CONFIG (build failed)"
        continue
      fi

      # Run C driver
      export LD_LIBRARY_PATH="$BUILD_DIR/app:$BUILD_DIR/lib/$hash:$LD_LIBRARY_PATH"
      C_OUTPUT=$(timeout 600 "$BUILD_DIR/app/driver" 2>&1) || {
        echo "  C DRIVER FAILED - skipping"
        SKIP=$((SKIP + 1))
        RESULTS="$RESULTS\nSKIP $CONFIG (C driver failed)"
        continue
      }
      C_DIGEST=$(echo "$C_OUTPUT" | grep -oP '[0-9A-F]{64}')
      echo "  C digest: $C_DIGEST"

      # Build Rust
      cd "$RUST_DIR"
      FEATURES="${hash},${thash},${secpar}"
      if ! timeout 300 cargo build --release --no-default-features --features "$FEATURES" --bin driver > /dev/null 2>&1; then
        echo "  RUST BUILD FAILED"
        FAIL=$((FAIL + 1))
        RESULTS="$RESULTS\nFAIL $CONFIG (Rust build failed)"
        continue
      fi

      # Run Rust driver
      R_OUTPUT=$(timeout 600 "$RUST_DIR/target/release/driver" 2>&1) || {
        echo "  RUST DRIVER FAILED"
        FAIL=$((FAIL + 1))
        RESULTS="$RESULTS\nFAIL $CONFIG (Rust driver failed)"
        continue
      }
      R_DIGEST=$(echo "$R_OUTPUT" | grep -oP '[0-9A-F]{64}')
      echo "  Rust digest: $R_DIGEST"

      if [ "$C_DIGEST" = "$R_DIGEST" ]; then
        echo "  PASS"
        PASS=$((PASS + 1))
        RESULTS="$RESULTS\nPASS $CONFIG ($C_DIGEST)"
      else
        echo "  FAIL: digests differ!"
        FAIL=$((FAIL + 1))
        RESULTS="$RESULTS\nFAIL $CONFIG (C=$C_DIGEST Rust=$R_DIGEST)"
      fi
    done
  done
done

echo ""
echo "========================================="
echo "RESULTS: $PASS passed, $FAIL failed, $SKIP skipped"
echo "========================================="
echo -e "$RESULTS"
