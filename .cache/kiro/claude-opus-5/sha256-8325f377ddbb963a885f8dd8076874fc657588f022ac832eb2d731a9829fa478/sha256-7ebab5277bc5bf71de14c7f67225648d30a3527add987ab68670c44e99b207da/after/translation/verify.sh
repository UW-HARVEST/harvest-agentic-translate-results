#!/usr/bin/env bash
# Full verification sweep: every feature combination is compile-checked, the
# exported symbol sets of the two shared libraries are compared, and the
# differential test suite is run in both cargo profiles.
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
crate="$root/translation"
logdir="${TMPDIR:-/tmp}/gjk-verify"
mkdir -p "$logdir"
rc=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; rc=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
step "feature combinations"
features="$(python3 - "$crate/Cargo.toml" <<'PY'
import re, sys, itertools
text = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        key = line.split('=', 1)[0].strip().strip('"')
        if key != 'default':
            names.append(key)
for n in range(len(names) + 1):
    for combo in itertools.combinations(names, n):
        print(','.join(combo))
PY
)"
if [ -z "$features" ]; then features=""; fi
printf 'declared non-default features: %s\n' "$(echo "$features" | tr '\n' '|')"
echo "(an empty entry means --no-default-features with nothing enabled)"

# ---------------------------------------------------------------------------
# 2. cargo check for every combination, plus the default configuration.
# ---------------------------------------------------------------------------
step "cargo check per combination"
while IFS= read -r combo; do
    label="${combo:-<none>}"
    log="$logdir/check-${combo//,/_}.log"
    if [ -z "$combo" ]; then
        args=(--no-default-features)
    else
        args=(--no-default-features --features "$combo")
    fi
    if timeout 600 cargo check --manifest-path "$crate/Cargo.toml" --all-targets \
        "${args[@]}" >"$log" 2>&1; then
        echo "ok    cargo check --no-default-features --features $label"
    else
        fail "cargo check (features: $label) - see $log"
        tail -n 20 "$log"
    fi
done <<< "$features"

log="$logdir/check-default.log"
if timeout 600 cargo check --manifest-path "$crate/Cargo.toml" --all-targets \
    >"$log" 2>&1; then
    echo "ok    cargo check (default features)"
else
    fail "cargo check (default features) - see $log"
    tail -n 20 "$log"
fi

# ---------------------------------------------------------------------------
# 3. Build the C reference library.
# ---------------------------------------------------------------------------
step "build C reference library"
if timeout 600 bash -c "
    cd '$root/c_src' && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
" >"$logdir/cmake.log" 2>&1; then
    echo "ok    C shared library built"
else
    fail "C build - see $logdir/cmake.log"
    tail -n 20 "$logdir/cmake.log"
fi
c_so="$(ls "$root"/c_src/build/*.so | head -n1)"
echo "C .so:    $c_so"

# ---------------------------------------------------------------------------
# 4/5. Test suite + symbol parity, per profile.
# ---------------------------------------------------------------------------
for profile in debug release; do
    step "cargo test ($profile)"
    extra=()
    [ "$profile" = release ] && extra=(--release)
    log="$logdir/test-$profile.log"
    if timeout 600 cargo test --manifest-path "$crate/Cargo.toml" "${extra[@]}" \
        >"$log" 2>&1; then
        echo "ok    cargo test ($profile)"
        grep -E '^test result:' "$log" | sed 's/^/      /'
    else
        fail "cargo test ($profile) - see $log"
        grep -E '^(test .* FAILED|failures:|thread )' "$log" | head -n 40
        tail -n 30 "$log"
    fi

    step "exported symbol parity ($profile)"
    rs_so="$crate/target/$profile/libgjk_lib.so"
    if [ ! -f "$rs_so" ]; then
        fail "missing $rs_so"
        continue
    fi
    nm -D --defined-only "$c_so"  | awk '{print $3}' | sort -u > "$logdir/c.syms"
    nm -D --defined-only "$rs_so" | awk '{print $3}' | sort -u > "$logdir/rs.syms"
    missing="$(comm -23 "$logdir/c.syms" "$logdir/rs.syms")"
    if [ -n "$missing" ]; then
        fail "Rust .so ($profile) is missing symbols exported by the C .so:"
        echo "$missing" | sed 's/^/      /'
    else
        echo "ok    all $(wc -l < "$logdir/c.syms") C symbols are exported by the Rust .so"
    fi
done

step "summary"
if [ "$rc" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "THERE WERE FAILURES"; fi
exit "$rc"
