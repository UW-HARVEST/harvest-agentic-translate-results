#!/usr/bin/env bash
# Full verification matrix: C build(s) x Rust profiles x feature combinations.
#
#   ./run_all.sh
#
# Every combination runs the complete differential suite (Phase B + C + D).
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
CARGO_OFFLINE="${CARGO_OFFLINE:---offline}"
FAILED=0
TMP="${TMPDIR:-/tmp}"

echo "=== building the C shared library (CMake, as specified) ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO_DEFAULT="$(ls -t "$ROOT"/c_src/build/*.so | head -1)"
echo "C .so: $C_SO_DEFAULT"

echo "=== building an extra -O2 C shared library (UB-sensitivity cross-check) ==="
mkdir -p target/cbuild-O2
C_SO_O2="$PWD/target/cbuild-O2/libc_src_O2.so"
gcc -O2 -fPIC -shared -I"$ROOT/c_src/include" "$ROOT/c_src/src/lib.c" -o "$C_SO_O2" \
  && echo "C .so (-O2): $C_SO_O2" || { echo "-O2 C build FAILED"; exit 1; }

echo "=== enumerating feature combinations from Cargo.toml ==="
# All features declared in Cargo.toml (excluding "default"):
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> combinations: {default} and {--no-default-features}"
  COMBOS=("" "--no-default-features")
else
  COMBOS=("" "--no-default-features")
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
    COMBOS+=("--features $f")
  done
  # all features at once
  COMBOS+=("--all-features")
fi

run_matrix() {
  local combo="$1"
  for profile in debug release; do
    local build_flag=""
    [ "$profile" = "release" ] && build_flag="--release"
    echo
    echo "--- cargo build $build_flag $combo ---"
    # shellcheck disable=SC2086
    cargo build $CARGO_OFFLINE $build_flag $combo >/dev/null 2>&1 \
      || { echo "BUILD FAILED: $profile $combo"; FAILED=1; continue; }
    local rust_so="target/$profile/libflac_validate_lib.so"
    if [ ! -f "$rust_so" ]; then
      echo "MISSING cdylib: $rust_so"; FAILED=1; continue
    fi
    for c_so in "$C_SO_DEFAULT" "$C_SO_O2"; do
      echo "--- cargo test $combo   [rust=$profile]  [c=$(basename "$c_so")] ---"
      # shellcheck disable=SC2086
      C_SO="$c_so" RUST_SO="$PWD/$rust_so" \
        cargo test $CARGO_OFFLINE $combo 2>&1 | grep -E "test result|FAILED|panicked" \
        || true
      # shellcheck disable=SC2086
      C_SO="$c_so" RUST_SO="$PWD/$rust_so" \
        cargo test $CARGO_OFFLINE $combo >/dev/null 2>&1 \
        || { echo "TESTS FAILED: rust=$profile c=$(basename "$c_so") combo='$combo'"; FAILED=1; }
    done
  done
}

for combo in "${COMBOS[@]}"; do
  echo
  echo "############ feature combination: '${combo:-<default>}' ############"
  run_matrix "$combo"
done

if [ "${EXHAUSTIVE:-0}" = "1" ]; then
  echo
  echo "############ exhaustive sweeps (all 2^32 size_memory inputs, ~42M validate cases) ############"
  for c_so in "$C_SO_DEFAULT" "$C_SO_O2"; do
    for profile in debug release; do
      echo "--- exhaustive [rust=$profile] [c=$(basename "$c_so")] ---"
      C_SO="$c_so" RUST_SO="$PWD/target/$profile/libflac_validate_lib.so" \
        cargo test $CARGO_OFFLINE --release --test phase_e_exhaustive -- --ignored \
        --test-threads=4 2>&1 | grep -E "test result|panicked" || true
      C_SO="$c_so" RUST_SO="$PWD/target/$profile/libflac_validate_lib.so" \
        cargo test $CARGO_OFFLINE --release --test phase_e_exhaustive -- --ignored \
        --test-threads=4 >/dev/null 2>&1 \
        || { echo "EXHAUSTIVE FAILED: rust=$profile c=$(basename "$c_so")"; FAILED=1; }
    done
  done
fi

echo
echo "=== symbol parity (nm -D) ==="
diff <(nm -D --defined-only --format=posix "$C_SO_DEFAULT" | awk '{print $1}' | sort) \
     <(nm -D --defined-only --format=posix target/release/libflac_validate_lib.so \
        | awk '$2=="T"||$2=="D"||$2=="B"{print $1}' | grep -v '^_' | sort) \
  && echo "symbol diff: EMPTY (C subset fully exported by Rust)" \
  || { echo "SYMBOL DIFF NON-EMPTY"; FAILED=1; }

echo
if [ "$FAILED" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$FAILED"
