#!/bin/sh
set -eu

crate_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
c_dir=$(CDPATH= cd -- "$crate_dir/../c_src" && pwd)
c_so=$(find "$c_dir/build" -maxdepth 1 -type f -name '*.so' -print -quit)
rust_so="$crate_dir/target/release/libenvy_lib.so"

if [ -z "$c_so" ] || [ ! -f "$rust_so" ]; then
    echo "Build both shared libraries before generating surfaces." >&2
    exit 1
fi

{
    printf '# Exported Symbol Surface\n\n'
    printf 'Generated from `nm -D --defined-only %s`.\n\n' "$(basename "$c_so")"
    printf '| C symbol | C type | Rust export | Status |\n'
    printf '|----------|--------|-------------|--------|\n'
    nm -D --defined-only "$c_so" |
        awk '{ print $2, $3 }' |
        while read -r symbol_type symbol_name; do
            if nm -D --defined-only "$rust_so" |
                awk '{ print $3 }' |
                grep -Fqx "$symbol_name"; then
                rust_export='present'
                status='[x]'
            else
                rust_export='missing'
                status='[ ]'
            fi
            printf '| `%s` | `%s` | %s | %s |\n' \
                "$symbol_name" "$symbol_type" "$rust_export" "$status"
        done
    printf '\nUndefined C imports are libc/toolchain symbols: '
    printf '`atoi`, `fprintf`, `getenv`, `printf`, `puts`, `snprintf`, '
    printf '`stderr`, `strchr`, `_ITM_*`, `__cxa_finalize`, and `__gmon_start__`.\n'
} > "$crate_dir/SYMBOLS.md"

{
    printf '# Error Surface\n\n'
    printf '| # | function | trigger (the exact invalid input/condition) | expected C result |\n'
    printf '|---|----------|----------------------------------------------|-------------------|\n'
    printf '| 1 | `parse_env_numeric` | `getenv(env_name) == NULL` (requested variable is absent) | [ ] returns `default_val` |\n'
    printf '| 2 | `parse_env_numeric` | value contains `,` | [ ] warns `Invalid character` and returns `default_val` |\n'
    printf '| 3 | `parse_env_numeric` | value contains `;` and no earlier comma branch applies | [ ] warns `Semicolon found` and returns `default_val` |\n'
    printf '| 4 | `envy` | computed result is `< 0` after bit operations and base offset | [ ] restores the backup and returns original `param1` |\n'
    printf '| 5 | `parse_env_numeric` | `env_name == NULL` | [ ] no C guard; process terminates on invalid libc input |\n'
    printf '| 6 | `init_config_from_env` | `flags == NULL` | [ ] no C guard; process terminates on invalid dereference |\n'
    printf '| 7 | `perform_operation` | `flags == NULL` | [ ] no C guard; process terminates on invalid dereference |\n'
    printf '| 8 | `apply_bit_operations` | `flags == NULL` | [ ] no C guard; process terminates on invalid dereference |\n'
    printf '\nThere are no length parameters, public enums, assertions, error enums, '
    printf 'explicit min/max constants, or `return -1`/`RETURN_ERROR` branches. '
    printf 'The 3-bit `log_level` boundary and arbitrary raw flag words are valid '
    printf 'configuration inputs and are covered in `CONFIGS.md` rows 11-18.\n'
} > "$crate_dir/ERRORS.md"

{
    printf '# Configuration Surface\n\n'
    printf '| # | entry point(s) | configuration (options set + input shape) | [ ] |\n'
    printf '|---|----------------|--------------------------------------------|-----|\n'

    row=1
    printf '| %d | `parse_env_numeric` | variable absent; randomized `default_val` | [ ] |\n' "$row"
    row=$((row + 1))
    printf '| %d | `parse_env_numeric` | variable present; valid `atoi` input shapes (empty/non-numeric, signed, whitespace, numeric suffix) | [ ] |\n' "$row"
    row=$((row + 1))

    for verbose in 0 1; do
        for debug in 0 1; do
            for optimize in 0 1; do
                printf '| %d | `init_config_from_env` | verbose=%d, debug=%d, optimize=%d; absent/present-without-`1` representations randomized for false states | [ ] |\n' \
                    "$row" "$verbose" "$debug" "$optimize"
                row=$((row + 1))
            done
        done
    done

    for optimize in 0 1; do
        for debug in 0 1; do
            printf '| %d | `perform_operation` | optimize=%d, debug=%d; randomized signed operands and raw non-field flag bits | [ ] |\n' \
                "$row" "$optimize" "$debug"
            row=$((row + 1))
        done
    done

    for verbose in 0 1; do
        for cache in 0 1; do
            printf '| %d | `apply_bit_operations` | verbose=%d, cache_enabled=%d; randomized signed values and raw non-field flag bits | [ ] |\n' \
                "$row" "$verbose" "$cache"
            row=$((row + 1))
        done
    done

    for verbose in 0 1; do
        for debug in 0 1; do
            for optimize in 0 1; do
                for base in default explicit; do
                    for multiplier in default explicit; do
                        for param3 in zero nonzero; do
                            for param4 in zero nonzero; do
                                for result in nonnegative negative; do
                                    printf '| %d | `envy` | verbose=%d, debug=%d, optimize=%d; base_offset=%s, multiplier=%s, param3=%s, param4=%s, pre-fallback result=%s; randomized values | [ ] |\n' \
                                        "$row" "$verbose" "$debug" "$optimize" \
                                        "$base" "$multiplier" "$param3" "$param4" "$result"
                                    row=$((row + 1))
                                done
                            done
                        done
                    done
                done
            done
        done
    done

    printf '\nRows cover all five dynamic entry points. `ConfigFlags` is exercised '
    printf 'as its four-byte C ABI storage, including all 3-bit `log_level` values, '
    printf 'the one-step-past encoding that spills into `reserved`, and unrelated '
    printf 'high bits. There are no Cargo features; the sole feature combination '
    printf 'is both the default invocation and `--no-default-features`.\n'

    if [ "$row" -ne 275 ]; then
        echo "unexpected configuration row count: $((row - 1))" >&2
        exit 1
    fi
} > "$crate_dir/CONFIGS.md"
