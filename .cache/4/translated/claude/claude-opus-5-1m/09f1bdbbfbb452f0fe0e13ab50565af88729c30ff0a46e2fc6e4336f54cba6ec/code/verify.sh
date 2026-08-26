#!/usr/bin/env bash
# Compares the exported symbols of the Rust build against the C build,
# and (optionally) per-file symbol coverage.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
mkdir -p verify

# ---- overall .so symbol diff (requires a successful cargo build) ----
if [ -f target/release/libzstd.so ]; then
    nm -D --defined-only target/release/libzstd.so | awk '{print $3}' | sort -u > verify/rust_symbols.txt
    sort -u c_symbols.txt > verify/c_symbols.txt
    echo "C   exports: $(wc -l < verify/c_symbols.txt)"
    echo "Rust exports: $(wc -l < verify/rust_symbols.txt)"
    comm -23 verify/c_symbols.txt verify/rust_symbols.txt > verify/missing_in_rust.txt
    comm -13 verify/c_symbols.txt verify/rust_symbols.txt > verify/extra_in_rust.txt
    echo "MISSING in Rust: $(wc -l < verify/missing_in_rust.txt)"
    sed 's/^/  - /' verify/missing_in_rust.txt
    echo "EXTRA in Rust:   $(wc -l < verify/extra_in_rust.txt)"
    sed 's/^/  + /' verify/extra_in_rust.txt
else
    echo "target/release/libzstd.so not built yet; skipping .so diff"
fi

# ---- per-C-file coverage against whatever Rust sources exist ----
echo
echo "=== per-file coverage (Rust sources) ==="
python3 - <<'PY'
import os, re, subprocess

root = os.getcwd()
# map C file -> exported symbols
groups = {}
cur = None
for line in open('SYMBOLS_BY_FILE.txt'):
    s = line.strip()
    if s.startswith('=== '):
        cur = s[4:]
        groups[cur] = []
    elif s:
        groups[cur].append(s)

# gather all no_mangle names present in src/
present = set()
for dirpath, _, files in os.walk('src'):
    for f in files:
        if not f.endswith('.rs'):
            continue
        txt = open(os.path.join(dirpath, f)).read()
        # after every no_mangle attribute, find the next `fn NAME` or `static NAME`
        for m in re.finditer(r'#\[\s*unsafe\s*\(\s*no_mangle\s*\)\s*\]|#\[\s*no_mangle\s*\]', txt):
            tail = txt[m.end(): m.end() + 600]
            n = re.search(r'\bfn\s+([A-Za-z0-9_]+)|\bstatic\s+(?:mut\s+)?([A-Za-z0-9_]+)', tail)
            if n:
                present.add(n.group(1) or n.group(2))

total_missing = 0
for cfile in sorted(groups):
    want = set(groups[cfile])
    if not want:
        continue
    miss = sorted(want - present)
    total_missing += len(miss)
    status = 'OK  ' if not miss else 'MISS'
    print(f"{status} {cfile}: {len(want)-len(miss)}/{len(want)}")
    if miss:
        for m in miss:
            print(f"        - {m}")
print()
print("TOTAL MISSING SYMBOLS:", total_missing)
PY
