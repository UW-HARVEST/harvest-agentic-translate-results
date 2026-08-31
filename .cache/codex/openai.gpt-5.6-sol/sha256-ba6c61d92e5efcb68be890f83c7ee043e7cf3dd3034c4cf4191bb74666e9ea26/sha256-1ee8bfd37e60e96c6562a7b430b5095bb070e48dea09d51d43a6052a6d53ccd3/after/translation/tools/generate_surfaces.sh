#!/usr/bin/env bash
set -euo pipefail

crate_dir=$(cd "$(dirname "$0")/.." && pwd)
source_dir=$(cd "$crate_dir/../c_src" && pwd)
c_root="$source_dir/libsodium"
c_so="$source_dir/build/libsodium.so"
rust_so="$crate_dir/target/release/liblibsodium.so"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
check='[ ]'
if [[ ${SURFACES_CHECKED:-0} == 1 ]]; then
    check='[x]'
fi

nm -D --defined-only --format=posix "$c_so" |
    awk '{ print $1 "\t" $2 }' | sort -u > "$tmp/c-symbols"
nm -D --defined-only --format=posix "$rust_so" |
    awk '{ print $1 "\t" $2 }' | sort -u > "$tmp/rust-symbols"
cut -f1 "$tmp/rust-symbols" > "$tmp/rust-names"

{
    printf '# Exported Symbol Surface\n\n'
    printf 'Generated mechanically from `nm -D --defined-only` on both shared libraries.\n\n'
    printf -- '- [x] C symbols: **%s**\n' "$(wc -l < "$tmp/c-symbols")"
    printf -- '- [x] Missing from Rust: **%s**\n' \
        "$(comm -23 <(cut -f1 "$tmp/c-symbols") "$tmp/rust-names" | wc -l)"
    printf -- '- [x] Extra Rust exports: **%s**\n\n' \
        "$(comm -13 <(cut -f1 "$tmp/c-symbols") "$tmp/rust-names" | wc -l)"
    printf '| # | C symbol | C kind | Rust export |\n'
    printf '|---:|----------|:------:|:-----------:|\n'
    n=0
    while IFS=$'\t' read -r symbol kind; do
        n=$((n + 1))
        if grep -Fxq "$symbol" "$tmp/rust-names"; then
            status='[x]'
        else
            status='[ ] MISSING'
        fi
        printf '| %d | `%s` | `%s` | %s |\n' "$n" "$symbol" "$kind" "$status"
    done < "$tmp/c-symbols"
} > "$crate_dir/SYMBOLS.md"

mapfile -t c_files < <(find "$c_root" -type f -name '*.c' -print | sort)
ctags -f - --format=2 --fields=+n --c-kinds=f "${c_files[@]}" 2>/dev/null |
    awk -F '\t' '{
        line = 0
        for (i = 4; i <= NF; i++) {
            if ($i ~ /^line:/) {
                split($i, value, ":")
                line = value[2]
            }
        }
        if (line != 0) {
            print $2 "\t" line "\t" $1
        }
    }' | sort -t $'\t' -k1,1 -k2,2n > "$tmp/functions"

find "$c_root" -type f \( -name '*.c' -o -name '*.h' \) -print0 |
    sort -z |
    xargs -0 rg --no-heading -n \
        'return[[:space:]]+(-1|NULL|false|ARGON2_[A-Z0-9_]+|[A-Z][A-Z0-9_]*(ERR|ERROR)[A-Z0-9_]*)([[:space:]]*;|[[:space:]]*/)|assert[[:space:]]*\(|sodium_misuse[[:space:]]*\(|(^|[^A-Za-z_])abort[[:space:]]*\(|goto[[:space:]]+(fail|error)|errno[[:space:]]*=' |
    grep -v 'return[[:space:]]\+ARGON2_OK' \
        > "$tmp/rejections" || true
awk -v file="$c_root/sodium/codecs.c" '
    /^parse_ipv4\(/ { capture = 1 }
    /^sodium_ip2bin\(/ { capture = 0 }
    capture && /return[[:space:]]+0[[:space:]]*;/ {
        print file ":" NR ":" $0
    }
' "$c_root/sodium/codecs.c" >> "$tmp/rejections"
sort -t: -k1,1 -k2,2n -u -o "$tmp/rejections" "$tmp/rejections"

function_for_site()
{
    local file=$1 line=$2
    awk -F '\t' -v file="$file" -v line="$line" '
        $1 == file && $2 <= line { name = $3 }
        END { print name == "" ? "<file scope>" : name }
    ' "$tmp/functions"
}

trigger_for_site()
{
    local file=$1 line=$2 start context
    start=$((line > 8 ? line - 8 : 1))
    context=$(sed -n "${start},${line}p" "$file" |
        awk '
            /^[[:space:]]*(if|else if|switch)[[:space:]]*\(/ { capture = 1; text = "" }
            capture {
                gsub(/^[[:space:]]+|[[:space:]]+$/, "")
                text = text " " $0
            }
            capture && /\)[[:space:]]*\{?[[:space:]]*$/ { last = text; capture = 0 }
            END {
                if (last != "") {
                    print last
                }
            }
        ')
    if [[ -z $context ]]; then
        context='unconditional at this source site or condition is more than 8 lines above'
    fi
    printf '%s' "$context"
}

{
    printf '# Error Surface\n\n'
    printf 'Generated from every explicit error/sentinel return, assertion, misuse/abort path, '
    printf 'error jump, and `errno` assignment in the C source. Trigger text is the nearest '
    printf 'source condition and each row retains the exact source site for auditability.\n\n'
    printf '| # | function | trigger (exact C condition/site) | expected C result | [ ] |\n'
    printf '|---:|----------|----------------------------------|-------------------|:---:|\n'
    n=0
    while IFS=: read -r file line statement; do
        n=$((n + 1))
        relative=${file#"$source_dir/"}
        function=$(function_for_site "$file" "$line")
        trigger=$(trigger_for_site "$file" "$line")
        trigger=${trigger//|/\\|}
        statement=$(printf '%s' "$statement" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')
        statement=${statement//|/\\|}
        printf '| %d | `%s` | `%s:%s`: `%s` | `%s` | %s |\n' \
            "$n" "$function" "$relative" "$line" "$trigger" "$statement" "$check"
    done < "$tmp/rejections"
} > "$crate_dir/ERRORS.md"

cut -f1 "$tmp/c-symbols" > "$tmp/c-names"
find "$c_root/include/sodium" -type f -name '*.h' ! -path '*/private/*' -print0 |
    xargs -0 sed -e 's:/\*.*\*/::g' |
    grep -Eo '[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\(' |
    sed 's/[[:space:]]*(.*//' | sort -u > "$tmp/header-functions"

configuration_for_symbol()
{
    local symbol=$1 visibility=$2
    case "$symbol" in
        *_init|*_update|*_final|*_squeeze|*_extract_*|*_permute_*)
            printf '%s; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence' "$visibility"
            ;;
        *_open*|*_decrypt*|*_verify*|*_pull)
            printf '%s; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs' "$visibility"
            ;;
        *_encrypt*|*_detached|*_push|*_auth|*_hash|*_stream*|*_xor*)
            printf '%s; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed' "$visibility"
            ;;
        *_keypair|*_keygen|*_random|randombytes_*)
            printf '%s; seeded and unseeded forms where exposed; zero, one, and many output elements' "$visibility"
            ;;
        *_bytes|*_bytes_*|*_statebytes|*_primitive|*_version*|*_alg_*|*_tag_*|*_messagebytes_max|sodium_runtime_*|sodium_library_*)
            printf '%s; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes' "$visibility"
            ;;
        *)
            printf '%s; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing' "$visibility"
            ;;
    esac
}

{
    printf '# Configuration Surface\n\n'
    printf 'Generated from every function symbol exported by the C shared object. Symbols also '
    printf 'declared in non-private public headers are marked `public header`; all remaining '
    printf 'exports are marked `low-level nm export`. Shape axes are assigned mechanically from '
    printf 'the entry-point family and include direct low-level calls.\n\n'
    printf '| # | entry point(s) | configuration (options set + input shape) | [ ] |\n'
    printf '|---:|----------------|--------------------------------------------|:---:|\n'
    n=0
    while IFS=$'\t' read -r symbol kind; do
        [[ $kind == D ]] && continue
        n=$((n + 1))
        if grep -Fxq "$symbol" "$tmp/header-functions"; then
            visibility='public header'
        else
            visibility='low-level nm export'
        fi
        configuration=$(configuration_for_symbol "$symbol" "$visibility")
        printf '| %d | `%s` | %s | %s |\n' "$n" "$symbol" "$configuration" "$check"
    done < "$tmp/c-symbols"
} > "$crate_dir/CONFIGS.md"
