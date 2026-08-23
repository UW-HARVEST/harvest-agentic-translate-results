#!/bin/sh
# Stamp a content hash of src/*.rs next to the built cdylib. Called by
# run_tests.sh and by the mutation-sweep tooling. See tests/common/mod.rs
# `assert_stale_free` for why this exists.
set -e
cd "$(dirname "$0")"
python3 - "$(dirname "$(ls target/debug/libmujs.so)")/.src_hash" <<'PY'
import os, sys
out = sys.argv[1]
files = sorted(f for f in os.listdir('src') if f.endswith('.rs'))
h = 0xcbf29ce484222325
M = (1 << 64) - 1
def feed(bs):
    global h
    for b in bs:
        h ^= b
        h = (h * 0x100000001b3) & M
for n in files:
    data = open(os.path.join('src', n), 'rb').read()
    feed(n.encode()); feed(b'\0')
    feed(str(len(data)).encode()); feed(b'\0')
    feed(data)
open(out, 'w').write('%016x\n' % h)
PY
