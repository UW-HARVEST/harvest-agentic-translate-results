#!/bin/bash
# Linker wrapper: adds _init and _fini to Rust's auto-generated version script
# so the cdylib exports the same symbols as the C shared library.
ARGS=()
for arg in "$@"; do
    if [[ "$arg" == -Wl,--version-script=* ]]; then
        SCRIPT="${arg#-Wl,--version-script=}"
        sed -i '/driver;/a\  _init;\n  _fini;' "$SCRIPT"
    fi
    ARGS+=("$arg")
done
exec cc "${ARGS[@]}"
