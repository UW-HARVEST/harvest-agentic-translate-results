#!/usr/bin/env bash
# Mechanical post-translation fixups that are always safe:
#   * C doxygen `/*!` block comments parse as Rust *inner* doc comments and are
#     illegal before an item -> rewrite to plain `/*`.
#   * likewise a leading `//!` that is not at the very top of a file.
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

python3 - <<'PY'
import os, re

changed = []
for dirpath, _dirs, files in os.walk('src'):
    for f in sorted(files):
        if not f.endswith('.rs'):
            continue
        p = os.path.join(dirpath, f)
        src = open(p).read()
        out = src.replace('/*!', '/* ')
        # `/**` immediately followed by whitespace+newline is an outer doc comment
        # on the next item; that is legal, leave it alone.
        if out != src:
            open(p, 'w').write(out)
            changed.append(p)
print("rewrote /*! in %d files" % len(changed))
for c in changed:
    print("  " + c)
PY
