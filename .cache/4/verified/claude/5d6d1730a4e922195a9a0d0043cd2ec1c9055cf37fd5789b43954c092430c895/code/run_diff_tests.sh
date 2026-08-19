#!/usr/bin/env bash
# Runs the whole C-vs-Rust differential verification for EVERY build
# configuration of this crate.
#
#   ./run_diff_tests.sh              # all configurations
#   ./run_diff_tests.sh --quick      # skip the release configuration
#   ./run_diff_tests.sh nodefault    # just one configuration (nodefault|default|release)
#
# Feature combinations are enumerated mechanically from Cargo.toml (the
# power set of [features] minus `default`), so adding a feature automatically
# adds its combinations here.

cd "$(dirname "$0")" || exit 1
mkdir -p logs

fail=0
run() { # run <logname> <cmd...>
    local log="logs/$1"
    shift
    printf '>>> %s\n' "$*" | tee "$log"
    if timeout 590 "$@" >>"$log" 2>&1; then
        printf '    OK\n'
    else
        printf '    FAILED (see %s)\n' "$log"
        tail -n 25 "$log"
        fail=1
    fi
}

# --------------------------------------------------------------------------
# 1. The C artefacts, exactly as c_src/CMakeLists.txt describes them
# --------------------------------------------------------------------------
echo "=== building the C reference ==="
(
    cd c_src && mkdir -p build && cd build &&
        cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
        cmake --build . >/dev/null
) || {
    echo "cmake build failed"
    exit 1
}
mkdir -p target/debug target/release
for d in target/debug target/release; do
    "${CC:-cc}" -shared -fPIC -O2 -o "$d/libcdriver.so" c_src/src/main.c || exit 1
done
echo "    c_src/build/driver + libcdriver.so ready"

echo
echo "=== exported-symbol parity ==="
nm -D --defined-only target/debug/libcdriver.so | awk '{print $NF}' | sort >logs/symbols-c.txt
cargo build --lib >/dev/null 2>&1
nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort >logs/symbols-rust.txt
if diff -u logs/symbols-c.txt logs/symbols-rust.txt; then
    echo "    identical: $(tr '\n' ' ' <logs/symbols-c.txt)"
else
    echo "    SYMBOL DIFF - see logs/symbols-*.txt"
    fail=1
fi

# --------------------------------------------------------------------------
# 2. Enumerate every feature combination
# --------------------------------------------------------------------------
combos=$(python3 - <<'PY'
import itertools, re
src = open('Cargo.toml').read()
m = re.search(r'^\[features\][^\[]*', src, re.M)
feats = []
if m:
    for line in m.group(0).splitlines()[1:]:
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name != 'default':
                feats.append(name)
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        print(','.join(c))
PY
)

echo
echo "=== feature combinations ==="
n=0
while IFS= read -r combo; do
    n=$((n + 1))
    if [ -z "$combo" ]; then
        echo "  [$n] --no-default-features (no features declared in Cargo.toml)"
    else
        echo "  [$n] --no-default-features --features $combo"
    fi
done <<<"$combos"
echo "  [+] default features"
echo "  [+] default features, --release"

# --------------------------------------------------------------------------
# 3. cargo check for every combination, then the differential suites
# --------------------------------------------------------------------------
SUITES=(symbols ffi_driver ffi_main errors cli_diff)

run_config() { # run_config <label> <extra cargo args...>
    local label="$1"
    shift
    echo
    echo "=== configuration: $label ==="
    run "check-$label.log" cargo check --all-targets "$@"
    run "build-$label.log" cargo build --lib --bins "$@"
    for s in "${SUITES[@]}"; do
        run "test-$label-$s.log" cargo test "$@" --test "$s"
    done
}

want="${1:-all}"
selected() { [ "$want" = "all" ] || [ "$want" = "--quick" ] || [ "$want" = "$1" ]; }

i=0
while IFS= read -r combo; do
    i=$((i + 1))
    if [ -z "$combo" ]; then
        selected nodefault && run_config "nodefault" --no-default-features
    else
        selected "features-$combo" &&
            run_config "features-$combo" --no-default-features --features "$combo"
    fi
done <<<"$combos"

selected default && run_config "default"

if [ "$want" != "--quick" ]; then
    selected release && run_config "release" --release
fi

echo
if [ "$fail" -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "FAILURES PRESENT - see logs/"
fi
exit "$fail"
