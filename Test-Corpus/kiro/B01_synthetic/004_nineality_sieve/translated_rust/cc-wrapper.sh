#!/bin/bash
# Custom linker wrapper that modifies Rust's version script to export symbols matching C .so
NEWARGS=()
for arg in "$@"; do
    if [[ "$arg" == *"version-script"* ]]; then
        SCRIPT="${arg#*=}"
        if [[ -f "$SCRIPT" ]]; then
            sed -i '/main;/a\    _init;\n    _fini;\n    __bss_start;\n    _edata;\n    _end;' "$SCRIPT"
        fi
    fi
    NEWARGS+=("$arg")
done
exec cc "${NEWARGS[@]}"
