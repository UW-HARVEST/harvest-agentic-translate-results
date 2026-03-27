#!/bin/bash
# Modify Rust's version script to add _init and _fini before calling cc
for arg in "$@"; do
    if [[ "$arg" == *"--version-script="* ]]; then
        vs_file="${arg#*--version-script=}"
        if [ -f "$vs_file" ] && grep -q "local:" "$vs_file"; then
            sed -i '/local:/i\    _init;\n    _fini;' "$vs_file"
        fi
    fi
done
exec cc "$@"
