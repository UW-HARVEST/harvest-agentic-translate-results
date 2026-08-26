#!/usr/bin/env bash
# Phase A/D driver: enumerate every valid build configuration, `cargo check` it,
# then run the full differential suite (Phases B + C + deep paths) under each.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
FAIL=0

echo "=== Enumerating features from Cargo.toml ==="
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/[ \t]/,"",a[1]);if(a[1]!="default"&&a[1]!="")print a[1]}' Cargo.toml)
echo "features: ${FEATURES:-<none>}"

COMBOS=("" "--no-default-features" "--all-features")
for f in $FEATURES; do
  COMBOS+=("--no-default-features --features $f")
done
printf 'combination: %s\n' "${COMBOS[@]/#/}" | sed 's/combination: $/combination: (default)/'

echo
echo "=== Building the ground-truth C shared library (c_src, unmodified) ==="
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null ) \
  || { echo "C BUILD FAILED"; exit 1; }
ls -l c_src/build/*.so

echo
echo "=== Building the C probe library (shadow_c; #includes c_src/src/lib.c) ==="
mkdir -p shadow_c/build
( cd shadow_c/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null ) \
  || { echo "SHADOW C BUILD FAILED"; exit 1; }
ls -l shadow_c/build/*.so

for combo in "${COMBOS[@]}"; do
  label=${combo:-"(default)"}
  echo
  echo "############################################################"
  echo "### CONFIG: $label"
  echo "############################################################"

  # shellcheck disable=SC2086
  echo "--- cargo check --all-targets $combo"
  if ! timeout 600 cargo check --all-targets $combo 2>&1 | tail -n 15; then
    echo "CHECK FAILED for $label"; FAIL=1; continue
  fi

  # Keep only the artifact built for THIS configuration on disk, so the tests
  # cannot dlopen a stale .so from a previous combination.
  rm -f target/debug/libjumpnode_lib.so target/release/libjumpnode_lib.so

  # shellcheck disable=SC2086
  echo "--- cargo build $combo"
  if ! timeout 600 cargo build $combo 2>&1 | tail -n 8; then
    echo "BUILD FAILED for $label"; FAIL=1; continue
  fi

  echo "--- exported symbols for this config:"
  nm -D --defined-only --format=posix target/debug/libjumpnode_lib.so \
    | awk '$2=="T" && $1!~/^_ZN/{print "      " $1}' | sort

  # shellcheck disable=SC2086
  echo "--- cargo test $combo"
  if ! timeout 600 cargo test $combo --no-fail-fast 2>&1 | grep -E "^(test |running|test result|error|warning: unused)" | tail -n 60; then
    echo "TESTS FAILED for $label"; FAIL=1; continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test $combo --no-fail-fast >/dev/null 2>&1; then
    echo "### CONFIG $label: TESTS FAILED"; FAIL=1; continue
  fi
  echo "### CONFIG $label: OK"
done

echo
echo "=== Phase D symbol diff (C exports vs Rust exports, default build) ==="
rm -f target/debug/libjumpnode_lib.so
timeout 600 cargo build >/dev/null 2>&1
if diff <(nm -D --defined-only --format=posix c_src/build/libtranslated_rust.so | awk '$2=="T"{print $1}' | sort) \
        <(nm -D --defined-only --format=posix target/debug/libjumpnode_lib.so | awk '$2=="T"&&$1!~/^_ZN/{print $1}' | sort); then
  echo "symbol diff: EMPTY (full parity)"
else
  echo "symbol diff: NON-EMPTY (see above)"; FAIL=1
fi

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAIL"
