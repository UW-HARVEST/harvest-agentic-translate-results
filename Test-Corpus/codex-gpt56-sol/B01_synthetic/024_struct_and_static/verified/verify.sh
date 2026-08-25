#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

mkdir -p c_src/build target/c-reference
timeout 600 cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build
timeout 600 cc -shared -fPIC -O0 \
    -o target/c-reference/libdriver_c.so c_src/src/main.c

features=()
while IFS= read -r feature; do
    features+=("$feature")
done < <(
    awk '
        /^\[features\]$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
            name = $0
            sub(/^[[:space:]]*/, "", name)
            sub(/[[:space:]]*=.*/, "", name)
            if (name != "default") print name
        }
    ' Cargo.toml
)

combination_count=$((1 << ${#features[@]}))
for ((mask = 0; mask < combination_count; mask++)); do
    selected=()
    for ((index = 0; index < ${#features[@]}; index++)); do
        if ((mask & (1 << index))); then
            selected+=("${features[index]}")
        fi
    done

    args=(--no-default-features)
    if ((${#selected[@]})); then
        combo=$(IFS=,; printf '%s' "${selected[*]}")
        args+=(--features "$combo")
    else
        combo="<empty>"
    fi

    printf 'Verifying feature combination: %s\n' "$combo"
    timeout 600 cargo check "${args[@]}"
    timeout 600 cargo build "${args[@]}" --lib
    timeout 600 cargo test "${args[@]}"
done

comm -23 \
    <(nm -D --defined-only --format=posix target/c-reference/libdriver_c.so |
        awk '{print $1}' | sort -u) \
    <(nm -D --defined-only --format=posix target/debug/libdriver.so |
        awk '{print $1}' | sort -u) \
    >target/missing-symbols.txt

if [[ -s target/missing-symbols.txt ]]; then
    printf 'Rust library is missing C symbols:\n' >&2
    cat target/missing-symbols.txt >&2
    exit 1
fi

printf 'All feature combinations and dynamic symbols verified.\n'
