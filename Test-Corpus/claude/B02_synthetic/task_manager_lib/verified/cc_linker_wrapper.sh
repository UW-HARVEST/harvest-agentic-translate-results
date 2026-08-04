#!/bin/sh
# Linker wrapper used as `linker = "..."` in .cargo/config.
#
# rustc emits a `--version-script=...` that ends with `local: *;` which
# strips ELF crt symbols (`_init`, `_fini`) from the dynamic symbol
# table. The C `libdriver.so` exports these symbols, so to match it we
# patch rustc's version script to add `_init` and `_fini` to its
# global list before invoking cc.

set -e

NEW_ARGS=""
for a in "$@"; do
    case "$a" in
        -Wl,--version-script=*)
            VS_PATH="${a#-Wl,--version-script=}"
            if [ -f "$VS_PATH" ]; then
                # Insert `_init;` and `_fini;` after the `global:` line.
                # Uses a portable POSIX awk replacement approach.
                TMPF="${VS_PATH}.patched"
                awk '
                    /^[[:space:]]*global:[[:space:]]*$/ {
                        print
                        print "    _init;"
                        print "    _fini;"
                        next
                    }
                    { print }
                ' "$VS_PATH" > "$TMPF"
                mv "$TMPF" "$VS_PATH"
            fi
            NEW_ARGS="$NEW_ARGS $a"
            ;;
        *)
            NEW_ARGS="$NEW_ARGS $a"
            ;;
    esac
done

# shellcheck disable=SC2086
exec cc $NEW_ARGS
