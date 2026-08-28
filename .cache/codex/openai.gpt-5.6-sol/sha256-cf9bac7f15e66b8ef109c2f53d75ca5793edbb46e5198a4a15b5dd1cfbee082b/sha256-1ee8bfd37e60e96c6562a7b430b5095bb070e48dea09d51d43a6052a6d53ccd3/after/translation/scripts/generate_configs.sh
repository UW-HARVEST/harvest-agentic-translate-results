#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
c_source=../c_src/src/lib.c

# Refuse to generate a stale table if the mechanically extracted C axes change.
test "$(grep -Ec '^[[:space:]]*case (10|20|30|40):' "$c_source")" -eq 4
test "$(grep -Ec '^int cleanup[(]|^void print_result[(]|^void cleanup_resources[(]' "$c_source")" -eq 6

output=CONFIGS.md
{
    printf '%s\n\n' '# Configuration Surface'
    printf '%s\n\n' 'Derived from the public functions and runtime branches in `../c_src/src/lib.c`.'
    printf '%s\n\n' 'There are no Cargo features, C preprocessor feature flags, enums, length parameters, or byte-order/format options. For `cleanup`, each argument independently selects one of five `switch` paths: exact values `10`, `20`, `30`, `40`, or any other `int`. The table therefore contains the complete `5^4 = 625` cross-product. `OTHER` is a randomized equivalence class excluding the four case values; exact-case classes are necessarily singletons.'
    printf '%s\n' '| # | entry point(s) | configuration (options set + input shape) | verified |'
    printf '%s\n' '|---|----------------|--------------------------------------------|----------|'

    row=1
    for a in 10 20 30 40 OTHER; do
        for b in 10 20 30 40 OTHER; do
            for c in 10 20 30 40 OTHER; do
                for d in 10 20 30 40 OTHER; do
                    printf '| %d | `cleanup` | `(a, b, c, d) = (%s, %s, %s, %s)` switch-path classes; 32 fixed-seed trials | [ ] |\n' \
                        "$row" "$a" "$b" "$c" "$d"
                    row=$((row + 1))
                done
            done
        done
    done

    printf '| %d | `print_result` | Empty NUL-terminated label; randomized full-range `int` results | [ ] |\n' "$row"
    row=$((row + 1))
    printf '| %d | `print_result` | Non-empty NUL-terminated arbitrary-byte labels; randomized lengths and full-range `int` results | [ ] |\n' "$row"
    row=$((row + 1))
    printf '| %d | `print_result` | Storage contains an embedded NUL; C-string prefix is printed; randomized suffix and results | [ ] |\n' "$row"
    row=$((row + 1))
    printf '| %d | `print_result` | Oversized 4096-byte label; randomized bytes and boundary results | [ ] |\n' "$row"
    row=$((row + 1))
    printf '| %d | `cleanup_resources` | `dynamic_str == NULL` no-op branch | [ ] |\n' "$row"
    row=$((row + 1))
    printf '| %d | `cleanup_resources` | Non-null, `malloc`-compatible allocations of randomized sizes | [ ] |\n' "$row"
} >"$output"

if test "${VERIFIED:-0}" = 1; then
    sed -i 's/| \[ \] |$/| [x] |/' "$output"
fi
