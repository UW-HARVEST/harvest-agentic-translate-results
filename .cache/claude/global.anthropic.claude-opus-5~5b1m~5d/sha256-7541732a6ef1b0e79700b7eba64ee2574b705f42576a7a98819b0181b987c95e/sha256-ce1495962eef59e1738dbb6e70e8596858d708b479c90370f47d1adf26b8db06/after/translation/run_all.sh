#!/usr/bin/env bash
# Full verification run: build C + Rust, check symbol parity, then run the
# differential suite under EVERY feature combination declared in Cargo.toml.
#
# Always builds before testing: `cargo test` does not rebuild a cdylib-only lib
# target, so testing without an explicit build can validate a stale artifact.
set -u
cd "$(dirname "$0")" || exit 1
ROOT=..
rc=0
OFFLINE=${OFFLINE:---offline}

echo "########## 1. build the C shared library ##########"
( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }

echo "########## 2. enumerate feature combinations ##########"
# Read the [features] table from Cargo.toml (excluding "default").
feats=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/[ \t]/,"",a[1]); if(a[1]!="default"&&a[1]!="")print a[1]}' Cargo.toml)
if [ -z "$feats" ]; then
    echo "No [features] section in Cargo.toml -> the default (empty) feature set"
    echo "is the ONLY configuration. One combination to verify."
    combos=("DEFAULT")
else
    echo "Declared features: $feats"
    # Full power set of the declared features.
    combos=("DEFAULT")
    list=($feats)
    n=${#list[@]}
    for ((mask=0; mask<(1<<n); mask++)); do
        sel=""
        for ((b=0; b<n; b++)); do
            (( mask & (1<<b) )) && sel="$sel,${list[$b]}"
        done
        combos+=("${sel#,}")
    done
fi

echo "########## 3. cargo check every combination ##########"
for c in "${combos[@]}"; do
    if [ "$c" = "DEFAULT" ]; then args=()
    elif [ -z "$c" ]; then args=(--no-default-features)
    else args=(--no-default-features --features "$c"); fi
    printf '  check [%s] ... ' "$c"
    if timeout 300 cargo check --release $OFFLINE "${args[@]}" >/dev/null 2>&1
    then echo OK; else echo FAILED; rc=1; fi
done

echo "########## 4. build + symbol parity + tests per combination ##########"
for c in "${combos[@]}"; do
    if [ "$c" = "DEFAULT" ]; then args=()
    elif [ -z "$c" ]; then args=(--no-default-features)
    else args=(--no-default-features --features "$c"); fi

    echo "----- combination: [$c] -----"
    timeout 300 cargo build --release $OFFLINE "${args[@]}" 2>&1 | tail -2 \
        || { echo "build FAILED"; rc=1; continue; }

    bash ./check_symbols.sh || { echo "symbol parity FAILED for [$c]"; rc=1; }

    timeout 600 cargo test --release $OFFLINE "${args[@]}" 2>&1 \
        | grep -E '^test result|FAILED$|^error' \
        || { echo "test run produced no result line"; rc=1; }
    timeout 600 cargo test --release $OFFLINE "${args[@]}" >/dev/null 2>&1 \
        || { echo "TESTS FAILED for [$c]"; rc=1; }
done

echo "########## summary ##########"
[ $rc -eq 0 ] && echo "ALL CHECKS PASSED" || echo "SOME CHECKS FAILED"
exit $rc
