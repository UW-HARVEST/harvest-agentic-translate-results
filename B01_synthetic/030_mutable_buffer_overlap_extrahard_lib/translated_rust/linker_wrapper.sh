#!/bin/bash
# Patch Rust's generated version script to include _init and _fini
for arg in "$@"; do
    if [[ "$arg" == -Wl,--version-script=* ]]; then
        path="${arg#-Wl,--version-script=}"
        if [ -f "$path" ]; then
            sed -i 's/global:/global:\n    _init;\n    _fini;/' "$path"
        fi
    fi
done
exec /usr/bin/cc "$@"
