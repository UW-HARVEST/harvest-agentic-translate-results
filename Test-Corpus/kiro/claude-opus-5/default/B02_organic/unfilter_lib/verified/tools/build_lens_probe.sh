#!/usr/bin/env bash
# Builds an instrumented COPY of c_src/src/lib.c (c_src itself is never
# modified) whose only changes are:
#
#   1. `uint8_t lens[288 + 32]`  ->  padded by 512 bytes, so cp_dynamic's
#      documented 137-byte overrun stays inside the object instead of
#      corrupting the stack frame;
#   2. a new exported `int cp_lens_overrun;` set to 1 whenever the fill loop
#      writes at or past index 320.
#
# The decode path up to the overrun is byte-for-byte the same code as the real
# library, so `cp_lens_overrun` is an *exact* predicate for "this input drives
# the real C into undefined behaviour". The differential fuzzer uses it to
# decide whether a divergence is a translation bug (predicate false => must
# match exactly) or the C corrupting itself (predicate true => not comparable).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(cd "$here/.." && pwd)"
src="$crate/../c_src/src/lib.c"
outdir="$crate/target/probe"
mkdir -p "$outdir"

python3 - "$src" "$outdir/lens_probe.c" <<'PY'
import sys
src, dst = sys.argv[1], sys.argv[2]
s = open(src).read()

orig_decl = "  uint8_t lens[288 + 32];\n"
assert s.count(orig_decl) == 1, "cp_dynamic's lens[] declaration moved"
s = s.replace(orig_decl, "  uint8_t lens[288 + 32 + 512];\n")

# Flag every write at or past the real array bound.
before = s
s = s.replace("""    case 16:
      for (int i = 3 + cp_read_bits(s, 2); i; --i, ++n)
        lens[n] = lens[n - 1];
      break;
    case 17:
      for (int i = 3 + cp_read_bits(s, 3); i; --i, ++n)
        lens[n] = 0;
      break;
    case 18:
      for (int i = 11 + cp_read_bits(s, 7); i; --i, ++n)
        lens[n] = 0;
      break;
    default:
      lens[n++] = (uint8_t)sym;
      break;""",
"""    case 16:
      for (int i = 3 + cp_read_bits(s, 2); i; --i, ++n) {
        if (n >= 288 + 32) cp_lens_overrun = 1;
        lens[n] = lens[n - 1];
      }
      break;
    case 17:
      for (int i = 3 + cp_read_bits(s, 3); i; --i, ++n) {
        if (n >= 288 + 32) cp_lens_overrun = 1;
        lens[n] = 0;
      }
      break;
    case 18:
      for (int i = 11 + cp_read_bits(s, 7); i; --i, ++n) {
        if (n >= 288 + 32) cp_lens_overrun = 1;
        lens[n] = 0;
      }
      break;
    default:
      if (n >= 288 + 32) cp_lens_overrun = 1;
      lens[n++] = (uint8_t)sym;
      break;""")
assert s != before, "cp_dynamic's fill loop moved"

s = s.replace('#include "lib.h"', '#include "lib.h"\nint cp_lens_overrun;', 1)
assert "int cp_lens_overrun;" in s
open(dst, "w").write(s)
PY

gcc -O0 -DNDEBUG -shared -fPIC \
    -I"$crate/../c_src/include" -I"$crate/../c_src/src" \
    -o "$outdir/liblens_probe.so" "$outdir/lens_probe.c" -lm

nm -D --defined-only "$outdir/liblens_probe.so" | grep -q cp_lens_overrun
echo "built $outdir/liblens_probe.so"
