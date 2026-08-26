#!/usr/bin/env bash
# check.sh <module-path> [<module-path> ...]
#
# Type-checks one or more translated modules *in isolation* from the other
# in-progress modules.  A throw-away crate is built in $TMPDIR whose `src/`
# is a symlink to this project's `src/`, and whose crate root declares only
#   * the already-stable core modules, and
#   * the modules you name on the command line.
#
# Module paths are given relative to src/ without the .rs extension, e.g.
#   ./check.sh legacy/zstd_v05
#   ./check.sh compress/zstd_fast compress/zstd_double_fast
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${TMPDIR:=/tmp}"

if [ "$#" -lt 1 ]; then
    echo "usage: $0 <module-path-under-src-without-.rs> ..." >&2
    exit 2
fi

NAME="check_$(echo "$*" | tr ' /' '__')"
WORK="$TMPDIR/$NAME"
mkdir -p "$WORK"
rm -f "$WORK/Cargo.toml" "$WORK/Cargo.lock"

ROOTNAME="check_root_$$_$RANDOM.rs"

cat > "$WORK/Cargo.toml" <<EOF
[package]
name = "checkmod"
version = "0.0.0"
edition = "2021"

[lib]
name = "checkmod"
path = "src/$ROOTNAME"
crate-type = ["lib"]

[profile.dev]
overflow-checks = false
debug-assertions = false
EOF

rm -f "$WORK/src"
ln -s "$ROOT/src" "$WORK/src"

ROOTFILE="$ROOT/src/$ROOTNAME"

python3 - "$@" > "$ROOTFILE" <<'PY'
import sys, collections

# modules that are considered stable core and are always included
CORE = {
    '': ['libc', 'zstd_h'],
    'common': ['bits', 'bitstream', 'debug', 'entropy_common', 'error_private',
               'fse', 'fse_decompress', 'huf', 'mem', 'pool', 'threading',
               'xxhash', 'zstd_common', 'zstd_internal'],
    'compress': ['clevels', 'fse_compress', 'hist', 'zstd_compress_internal',
                 'zstd_compress_literals', 'zstd_cwksp', 'zstd_ldm_geartab',
                 'zstd_preSplit'],
    'decompress': ['zstd_ddict', 'zstd_decompress_internal'],
}

groups = collections.defaultdict(list)
for d, mods in CORE.items():
    groups[d].extend(mods)

for m in sys.argv[1:]:
    m = m.removesuffix('.rs')
    parts = m.split('/')
    d = '/'.join(parts[:-1])
    name = parts[-1]
    if name not in groups[d]:
        groups[d].append(name)

print('''#![allow(non_snake_case)]
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

for m in groups.pop('', []):
    print(f"pub mod {m};")

for d in sorted(groups):
    mods = groups[d]
    if not mods:
        continue
    assert '/' not in d, d
    print(f"pub mod {d} {{")
    for m in sorted(set(mods)):
        print(f"    pub mod {m};")
    print("}")
PY

cd "$WORK" || exit 1
CARGO_TARGET_DIR="$WORK/target" cargo build --message-format=short 2>&1
STATUS=$?
rm -f "$ROOTFILE"
exit $STATUS
