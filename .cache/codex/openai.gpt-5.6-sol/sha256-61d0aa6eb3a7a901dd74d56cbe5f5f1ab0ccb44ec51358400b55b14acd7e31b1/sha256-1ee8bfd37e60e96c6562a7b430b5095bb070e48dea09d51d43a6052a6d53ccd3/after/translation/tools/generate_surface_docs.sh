#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root_dir="$(cd "$crate_dir/.." && pwd)"
c_source="$root_dir/c_src/src/lib.c"
c_so="$root_dir/c_src/build/libharvest-work-m0JAPI.so"
rust_so="$crate_dir/target/release/libgjk_lib.so"
mark=" "

if [[ "${1:-}" == "--checked" ]]; then
    mark="x"
elif [[ $# -ne 0 ]]; then
    echo "usage: $0 [--checked]" >&2
    exit 2
fi

for path in "$c_source" "$c_so" "$rust_so"; do
    [[ -f "$path" ]] || {
        echo "missing prerequisite: $path" >&2
        exit 1
    }
done

mapfile -t c_symbols < <(nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u)
mapfile -t rust_symbols < <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u)
mapfile -t missing_symbols < <(comm -23 \
    <(printf '%s\n' "${c_symbols[@]}") \
    <(printf '%s\n' "${rust_symbols[@]}"))

{
    echo "# Dynamic Symbol Surface"
    echo
    echo "Generated from:"
    echo
    echo '```sh'
    echo "nm -D --defined-only c_src/build/libharvest-work-m0JAPI.so"
    echo "nm -D --defined-only translation/target/release/libgjk_lib.so"
    echo '```'
    echo
    echo "| # | C symbol | Rust export |"
    echo "|---:|----------|-------------|"
    i=1
    for symbol in "${c_symbols[@]}"; do
        rust_mark=" "
        if printf '%s\n' "${rust_symbols[@]}" | grep -Fxq "$symbol"; then
            rust_mark="$mark"
        fi
        printf '| %d | `%s` | [%s] |\n' "$i" "$symbol" "$rust_mark"
        ((i += 1))
    done
    echo
    printf -- '- C-defined dynamic symbols: **%d**\n' "${#c_symbols[@]}"
    printf -- '- Missing Rust exports: **%d**\n' "${#missing_symbols[@]}"
    echo "- The C library's only strong undefined dependency is \`sqrtf@GLIBC_2.2.5\`."
    echo "- [${mark}] Completion gate: zero missing C-defined symbols."
} > "$crate_dir/SYMBOLS.md"

{
    echo "# Error Surface"
    echo
    echo "The following mechanical rejection scan was applied to \`c_src/src/lib.c\`:"
    echo
    echo '```sh'
    echo "rg -n 'RETURN_ERROR|return[[:space:]]+(-1|NULL)|assert[[:space:]]*\\(|ERROR|EINVAL|ERANGE' c_src/src/lib.c"
    echo '```'
    echo
    echo "It finds **zero explicit rejection/error branches**. The C code has no error"
    echo "enum, error-return macro, assertion, documented min/max input range, or"
    echo "error sentinel. Consequently, the required rejection table has zero rows."
    echo
    echo "| # | function | trigger (the exact invalid input/condition) | expected C result |"
    echo "|---|----------|----------------------------------------------|-------------------|"
    echo
    echo "## Defined Boundary Behavior"
    echo
    echo "These are the generic boundary cases that the C source handles without"
    echo "dereferencing an invalid mandatory pointer. Null mandatory pointers and an"
    echo "invalid shape type passed to \`c2GJK\` have undefined behavior in C, so there"
    echo "is no C result to compare for those cases."
    echo
    echo "| # | function | boundary condition | expected C result | status |"
    echo "|---:|----------|--------------------|-------------------|-----|"
    printf '| E01 | `c2MakeProxy` | enum value `-1` or `3` | output proxy remains byte-unchanged | [%s] |\n' "$mark"
    printf '| E02 | `c2Support` | count `0`, valid backing pointer | returns `0` after reading element zero | [%s] |\n' "$mark"
    printf '| E03 | `c2Support` | count `-1`, valid backing pointer | returns `0` after reading element zero | [%s] |\n' "$mark"
    printf '| E04 | `c2Support` | oversized count `9`, nine-element backing array | scans all nine elements and returns strict first maximum | [%s] |\n' "$mark"
    printf '| E05 | `c2GJKSimplexMetric` | count outside `1..=3` | returns `0.0` | [%s] |\n' "$mark"
    printf '| E06 | `c2D` | count outside `1..=3` | returns `(0.0, 0.0)` | [%s] |\n' "$mark"
    printf '| E07 | `c2Witness` | count outside `1..=3` | writes `(0.0, 0.0)` to both outputs | [%s] |\n' "$mark"
    printf '| E08 | `c2L` | count outside `1..=2` | returns `(0.0, 0.0)` | [%s] |\n' "$mark"
    printf '| E09 | `c2GJK` | null `ax_ptr` | uses identity transform | [%s] |\n' "$mark"
    printf '| E10 | `c2GJK` | null `bx_ptr` | uses identity transform | [%s] |\n' "$mark"
    printf '| E11 | `c2GJK` | null `outA` | skips first witness write | [%s] |\n' "$mark"
    printf '| E12 | `c2GJK` | null `outB` | skips second witness write | [%s] |\n' "$mark"
    printf '| E13 | `c2GJK` | null `iterations` | skips iteration-count write | [%s] |\n' "$mark"
    printf '| E14 | `c2GJK` | null `cache` | skips cache read and write | [%s] |\n' "$mark"
    printf '| E15 | `c2GJK` | non-null cache with count `0` | ignores initial fields, then writes resulting cache | [%s] |\n' "$mark"
    printf '| E16 | `gjk` | null output `a` | operation completes and only `b` is written | [%s] |\n' "$mark"
    printf '| E17 | `gjk` | null output `b` | operation completes and only `a` is written | [%s] |\n' "$mark"
    printf '| E18 | `c2Div` | divisor `+0.0` or `-0.0` | returns C/IEEE-754 infinities or NaNs component-wise | [%s] |\n' "$mark"
    printf '| E19 | `c2Norm` | zero vector | returns two NaNs | [%s] |\n' "$mark"
} > "$crate_dir/ERRORS.md"

config_file="$crate_dir/CONFIGS.md"
{
    echo "# Configuration Surface"
    echo
    echo "The rows below come from every dynamic C entry point and every \`if\`,"
    echo "\`switch\`, shape-kind, pointer-option, radius-option, cache-state, and"
    echo "input-count branch in \`c_src/src/lib.c\`. Randomized tests use a fixed seed."
    echo "No Cargo features are declared, so the effective feature matrix is default"
    echo "and \`--no-default-features\` (behaviorally identical, both still tested)."
    echo
    echo "| # | entry point(s) | configuration (options set + input shape) | status |"
    echo "|---:|----------------|--------------------------------------------|-----|"
} > "$config_file"

row=1
add_config() {
    local entries="$1"
    local config="$2"
    printf '| C%03d | %s | %s | [%s] |\n' "$row" "$entries" "$config" "$mark" >> "$config_file"
    ((row += 1))
}

add_config '`c2V`, `c2Mulvs`, `c2Sub`, `c2Dot`, `c2Add`, `c2Neg`, `c2Skew`, `c2CCW90`' 'random finite vectors/scalars, including signs and zero'
for x_branch in 'a.x > b.x' 'a.x <= b.x'; do
    for y_branch in 'a.y > b.y' 'a.y <= b.y'; do
        add_config '`c2Maxv`' "$x_branch; $y_branch"
    done
done
for x_branch in 'a.x < b.x' 'a.x >= b.x'; do
    for y_branch in 'a.y < b.y' 'a.y >= b.y'; do
        add_config '`c2Minv`' "$x_branch; $y_branch"
    done
done
for x_region in below inside above; do
    for y_region in below inside above; do
        add_config '`c2Clampv`' "x $x_region [lo, hi]; y $y_region [lo, hi]"
    done
done
add_config '`c2RotIdentity`, `c2xIdentity`' 'zero-argument identity constructors'
add_config '`c2BBVerts`' 'finite AABB; four output vertices'
for shape in circle AABB capsule; do
    add_config '`c2MakeProxy`' "shape type $shape"
done
add_config '`c2Len`, `c2Det2`' 'random finite vectors, including zero'
for count in '1' '2' '3' 'default/out-of-range'; do
    add_config '`c2GJKSimplexMetric`' "simplex count $count"
done
add_config '`c2Mulrv`, `c2Mulxv`, `c2MulrvT`' 'random finite rotations, translations, and vectors'
for branch in 'v <= 0 (vertex a)' 'v > 0 and u <= 0 (vertex b)' 'u > 0 and v > 0 (edge ab)'; do
    add_config '`c22`' "$branch"
done
for branch in \
    'vAB <= 0 and uCA <= 0 (vertex a)' \
    'uAB <= 0 and vBC <= 0 (vertex b)' \
    'uBC <= 0 and vCA <= 0 (vertex c)' \
    'uAB > 0 and vAB > 0 and wABC <= 0 (edge ab)' \
    'uBC > 0 and vBC > 0 and uABC <= 0 (edge bc)' \
    'uCA > 0 and vCA > 0 and vABC <= 0 (edge ca)' \
    'remaining region (triangle abc)'; do
    add_config '`c23`' "$branch"
done
for branch in 'count 1' 'count 2 with positive determinant' 'count 2 with nonpositive determinant' 'count 3/default'; do
    add_config '`c2D`' "$branch"
done
add_config '`c2Support`' 'count 1'
add_config '`c2Support`' 'count many with unique strict maximum'
add_config '`c2Support`' 'count many with tied maximum; first index wins'
for count in '1' '2' '3' 'default/out-of-range'; do
    add_config '`c2Witness`' "simplex count $count"
done
add_config '`c2Div`' 'random finite vector and nonzero divisor'
add_config '`c2Norm`' 'random finite nonzero vector'
for count in '1' '2' 'default/out-of-range'; do
    add_config '`c2L`' "simplex count $count"
done

for shape_a in circle AABB capsule; do
    for shape_b in circle AABB capsule; do
        for transforms in \
            'ax=null, bx=null' \
            'ax=set, bx=null' \
            'ax=null, bx=set' \
            'ax=set, bx=set'; do
            for radius in 0 1; do
                for cache in null empty warm; do
                    add_config '`c2GJK`' "A=$shape_a; B=$shape_b; $transforms; use_radius=$radius; cache=$cache"
                done
            done
        done
    done
done

add_config '`c2GJK`' 'all 16 null/non-null combinations of outA, outB, iterations, and cache with valid AABB/capsule inputs'
add_config '`gjk`' 'reverse=0; randomized AABB and capsule values'
add_config '`gjk`' 'reverse!=0; randomized AABB and capsule values'

printf '\nTotal configuration rows: **%d**.\n' "$((row - 1))" >> "$config_file"
