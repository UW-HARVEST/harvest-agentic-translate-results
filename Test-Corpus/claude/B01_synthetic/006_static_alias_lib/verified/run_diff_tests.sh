#!/usr/bin/env bash
# Complete differential verification for the StaticAlias C-to-Rust translation.
#
# Two things this script exists to guarantee:
#
#  1. `cargo build` runs BEFORE `cargo test`. The crate is `crate-type =
#     ["cdylib"]` only, so integration tests cannot link it and `cargo test`
#     alone will happily leave a STALE target/<profile>/libStaticAlias.so in
#     place -- silently testing an old translation. The tests dlopen that file.
#  2. Tests run with `--test-threads=1`. `driver` is verified by capturing fd 1,
#     which is process-wide; concurrent test threads would pollute the capture.
#
# Covers: the single feature combination (Cargo.toml declares no [features]),
# both cargo profiles (dev, and release which sets panic="abort" and turns
# debug-assertions off), symbol parity for each, and the C library rebuilt at
# -O2/-O3 to confirm the signed-overflow ground truth is optimization-stable.
set -euo pipefail
cd "$(dirname "$0")"
TMPDIR="${TMPDIR:-/tmp}"

echo "############ 1. C shared library (default cmake config) ############"
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
ls -l c_src/build/libStaticAlias.so

echo
echo "############ 2. feature combinations ############"
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);
             if(a[1]!="default") print a[1]}' Cargo.toml)
echo "declared features: [${FEATS:-<none>}]  => 1 combination (the empty set)"
cargo check --offline --no-default-features 2>&1 | tail -1
cargo check --offline --all-features 2>&1 | tail -1
cargo check --offline --all-targets 2>&1 | tail -1

for PROFILE in debug release; do
  echo
  echo "############ 3. profile=$PROFILE ############"
  if [[ $PROFILE == release ]]; then FLAG=--release; else FLAG=; fi
  cargo build --offline $FLAG 2>&1 | tail -1
  ls -l "target/$PROFILE/libStaticAlias.so"
  ./check_symbols.sh "$PROFILE" | tail -3
  cargo test --offline $FLAG -- --test-threads=1 2>&1 | grep -E 'test result|FAILED'
done

echo
echo "############ 4. C at -O2/-O3 (signed-overflow ground truth) ############"
for OPT in O2 O3; do
  D="$TMPDIR/cbuild_$OPT"; mkdir -p "$D"
  (cd "$D" && cmake "$PWD/../../translated_rust/c_src" \
      -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DCMAKE_C_FLAGS="-$OPT" >/dev/null 2>&1 \
   || cmake "$OLDPWD/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DCMAKE_C_FLAGS="-$OPT" >/dev/null)
  (cd "$D" && cmake --build . >/dev/null)
  echo "--- C -$OPT vs Rust ---"
  STATICALIAS_C_SO="$D/libStaticAlias.so" \
    cargo test --offline -- --test-threads=1 2>&1 | grep -E 'test result|FAILED'
done

echo
echo "ALL DIFFERENTIAL VERIFICATION PASSED"
