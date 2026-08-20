#!/usr/bin/env bash
# Build both shared objects, then run the differential test suite across every
# valid feature combination.
#
# `cargo test` compiles the lib for the test profile but does NOT relink the
# cdylib artifact, so `cargo build` must run first for every feature
# combination. tests/common/mod.rs also enforces this with a staleness guard.
set -euo pipefail

cd "$(dirname "$0")"

# ---- feature combinations ---------------------------------------------------
# Enumerated from [features] in Cargo.toml. There is no [features] section, so
# the only valid combination is the empty set.
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
if not names:
    print('')            # the empty feature set is the whole surface
else:
    from itertools import combinations
    seen = set()
    for r in range(len(names) + 1):
        for c in combinations(names, r):
            s = ','.join(c)
            if s not in seen:
                seen.add(s)
                print(s)
PY
)

echo "### feature combinations to verify: ${#FEATURES[@]}"
for f in "${FEATURES[@]}"; do echo "  - '${f:-<none>}'"; done

# ---- build the C reference implementation -----------------------------------
echo
echo "### building C shared object"
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
ls -l c_src/build/libdriver.so

# ---- per-combination check / build / test -----------------------------------
#
# Both cargo profiles are exercised. This is NOT redundant: `[profile.release]`
# is a distinct build configuration, and an unsound ABI assumption in the
# translation (declaring a `char` parameter as `c_char`, which lets LLVM elide
# the low-byte truncation gcc always performs) produced identical output in
# `debug` and *divergent* output in `release`. A debug-only run misses it.
for f in "${FEATURES[@]}"; do
  for profile in debug release; do
    label="${f:-<none>}"
    echo
    echo "=============================================================="
    echo "### feature combination: ${label}   profile: ${profile}"
    echo "=============================================================="

    args=(--no-default-features)
    if [[ -n "$f" ]]; then args+=(--features "$f"); fi
    if [[ "$profile" == "release" ]]; then args+=(--release); fi

    echo "--- cargo check ---"
    cargo check --offline "${args[@]}" --all-targets

    echo "--- cargo build (relink cdylib; cargo test does not do this) ---"
    cargo build --offline "${args[@]}"

    echo "--- cargo test ---"
    cargo test --offline "${args[@]}" -- --test-threads=1
  done
done

echo
echo "### ALL FEATURE COMBINATIONS x PROFILES PASSED"
