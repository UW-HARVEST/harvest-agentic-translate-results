#!/usr/bin/env bash
# Regenerates src/lib.rs from whatever module files currently exist under src/.
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

python3 - <<'PY' > src/lib.rs
import os

SKIP = {'lib.rs', 'layoutcheck.rs', 'layoutmain.rs'}

print('''//! Rust translation of the zstd C library (v1.5.7), built as a cdylib that
//! exposes the same public ABI as the original shared object.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unused_unsafe)]
#![allow(dead_code)]
#![allow(static_mut_refs)]
#![allow(clippy::all)]
''')

top = sorted(f[:-3] for f in os.listdir('src')
             if f.endswith('.rs') and f not in SKIP and not f.startswith('check_root'))
for m in top:
    print(f"pub mod {m};")
print()

for d in ['common', 'compress', 'decompress', 'dictbuilder', 'deprecated', 'legacy']:
    p = os.path.join('src', d)
    if not os.path.isdir(p):
        continue
    mods = sorted(f[:-3] for f in os.listdir(p) if f.endswith('.rs'))
    # files that are `include!`d by another module must not be declared as modules
    import re as _re
    mods = [m for m in mods if not _re.search(r'_p[0-9]+$', m)]
    if not mods:
        continue
    print(f"pub mod {d} {{")
    for m in mods:
        print(f"    pub mod {m};")
    print("}")
    print()
PY

echo "regenerated src/lib.rs:"
grep -c "pub mod" src/lib.rs
