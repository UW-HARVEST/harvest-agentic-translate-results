#!/bin/sh
set -eu

manifest_dir=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
c_library="$manifest_dir/../c_src/build/libharvest-work-7NfxTl.so"
rust_library="$manifest_dir/target/release/libupdate_frame_header_lib.so"

cd "$manifest_dir"

feature_names=$(
    awk '
        /^\[features\]$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            name = $0
            sub(/[[:space:]]*=.*/, "", name)
            if (name != "default") print name
        }
    ' Cargo.toml
)

set -- $feature_names
feature_count=$#
if [ "$feature_count" -gt 12 ]; then
    echo "Refusing to enumerate more than 12 independent Cargo features" >&2
    exit 1
fi

run_configuration() {
    label=$1
    shift
    echo "== feature configuration: $label =="
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test "$@"

    c_symbols=$(mktemp)
    rust_symbols=$(mktemp)
    missing_symbols=$(mktemp)
    trap 'rm -f "$c_symbols" "$rust_symbols" "$missing_symbols"' EXIT HUP INT TERM

    nm -D --defined-only -j "$c_library" | LC_ALL=C sort -u > "$c_symbols"
    nm -D --defined-only -j "$rust_library" | LC_ALL=C sort -u > "$rust_symbols"
    comm -23 "$c_symbols" "$rust_symbols" > "$missing_symbols"
    if [ -s "$missing_symbols" ]; then
        echo "C symbols missing from Rust:" >&2
        sed 's/^/  /' "$missing_symbols" >&2
        exit 1
    fi

    if ldd -r "$rust_library" 2>&1 | grep -q 'undefined symbol:'; then
        echo "Rust library has unresolved dynamic symbols:" >&2
        ldd -r "$rust_library" 2>&1 | grep 'undefined symbol:' >&2
        exit 1
    fi

    rm -f "$c_symbols" "$rust_symbols" "$missing_symbols"
    trap - EXIT HUP INT TERM
}

run_configuration default
run_configuration no-default-features --no-default-features

combination_count=$((1 << feature_count))
mask=1
while [ "$mask" -lt "$combination_count" ]; do
    index=0
    selected=
    for feature in $feature_names; do
        if [ $((mask & (1 << index))) -ne 0 ]; then
            if [ -n "$selected" ]; then
                selected="$selected,$feature"
            else
                selected=$feature
            fi
        fi
        index=$((index + 1))
    done
    run_configuration "$selected" --no-default-features --features "$selected"
    mask=$((mask + 1))
done

if grep -qF '| [ ] |' CONFIGS.md; then
    echo "CONFIGS.md contains unchecked rows" >&2
    exit 1
fi
if grep -qF '| [ ] |' ERRORS.md; then
    echo "ERRORS.md contains unchecked rows" >&2
    exit 1
fi

echo "All feature configurations, differential tests, and symbol checks passed."
