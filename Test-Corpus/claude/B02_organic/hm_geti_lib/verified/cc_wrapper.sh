#!/bin/bash
# Linker wrapper that patches rustc's auto-generated version-script
# to also export `_init` and `_fini`, restoring symbol-table parity
# with the C cmake-built shared library (whose dynsym includes the
# crti.o/crtn.o-supplied `_init` and `_fini` as GLOBAL).
set -e

NEW_ARGS=()
for arg in "$@"; do
  if [[ "$arg" == *"version-script="* ]]; then
    # Extract the script path. The arg looks like:
    #   -Wl,--version-script=/some/path/list
    script_path=$(echo "$arg" | sed -E 's/.*version-script=([^,]+).*/\1/')
    if [ -f "$script_path" ]; then
      patched="${script_path}.patched"
      # Insert `_init` and `_fini` into the `global:` section so the
      # final dynsym includes them as GLOBAL symbols, matching the C lib.
      sed 's/^  global:/  global:\n    _init;\n    _fini;/' \
        "$script_path" > "$patched"
      arg="${arg/$script_path/$patched}"
    fi
  fi
  NEW_ARGS+=("$arg")
done

exec /usr/bin/cc "${NEW_ARGS[@]}"
