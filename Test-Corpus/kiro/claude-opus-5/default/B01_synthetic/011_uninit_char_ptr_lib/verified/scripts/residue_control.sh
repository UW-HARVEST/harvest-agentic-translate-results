#!/usr/bin/env bash
# Builds the residue-attribution controls and runs the control tests.
#
# `bad()` reads an uninitialized stack slot. When it is called directly (not
# through `driver`), the slot's last writer is `dlopen`, so the pointer it
# forwards depends on the loaded object's *load footprint* rather than on any
# translated code. These controls demonstrate that by comparing two C builds
# that are behaviourally identical by construction:
#
#   plain.so — c_src/src/driver.c exactly as shipped
#   fat.so   — the same source plus 2000 unused exported functions and an extra
#              DT_NEEDED (-lm): same behaviour, different dynamic footprint
#
# If plain vs fat diverges on the same rows where C vs Rust diverges, those rows
# measure the loader and are not attributable to the translation.
#
# c_src/ is never modified; the source is copied out first.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(cd "$here/.." && pwd)"
csrc="$crate/../c_src"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cp "$csrc/src/driver.c" "$work/plain.c"
cp "$csrc/include/driver.h" "$work/driver.h"

python3 - "$work" <<'PY'
import sys
w = sys.argv[1]
src = open(f"{w}/plain.c").read()
extra = "\n".join(f"int ctl_sym_{i}(void) {{ return {i}; }}" for i in range(2000))
open(f"{w}/fat.c", "w").write(src.replace('#include "driver.h"',
                                          '#include "driver.h"\n' + extra))
PY

cc -O0 -fPIC -shared -I"$work" -o "$work/plain.so" "$work/plain.c"
cc -O0 -fPIC -shared -I"$work" -o "$work/fat.so"   "$work/fat.c" -lm

echo "control objects:"
for f in plain fat; do
  printf "  %-8s size=%-8s dynsym=%-6s needed=%s\n" "$f" \
    "$(stat -c%s "$work/$f.so")" \
    "$(nm -D --defined-only "$work/$f.so" | wc -l)" \
    "$(objdump -p "$work/$f.so" | grep -c NEEDED)"
done

cd "$crate"
DIFF_CTL_A="$work/plain.so" DIFF_CTL_B="$work/fat.so" \
  cargo test --test phase_b_residue_control -- --test-threads=1 --nocapture
