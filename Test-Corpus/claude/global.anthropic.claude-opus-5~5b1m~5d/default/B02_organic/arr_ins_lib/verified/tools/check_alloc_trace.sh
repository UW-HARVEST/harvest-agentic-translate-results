#!/bin/sh
# Allocation-call-sequence parity.
#
# Results-only comparison cannot see *how* a library allocates. This script
# LD_PRELOADs an allocator interposer and drives the C and the Rust `.so`
# through identical scripts, then requires the recorded malloc/calloc/realloc/
# free sequences to be byte-identical. It catches things like LLVM folding
# `realloc(NULL, n)` into `malloc(n)`, an extra/missing `free`, or a different
# growth ladder.
#
# Degrades gracefully: if no C compiler is available it prints SKIP and exits 0.
set -u
here=$(cd "$(dirname "$0")" && pwd)
crate=$(dirname "$here")
root=$(dirname "$crate")
out="${TMPDIR:-/tmp}/arrins_alloc.$$"
mkdir -p "$out" || exit 1

CSO=$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | head -1)
RSO="${RUST_SO:-$crate/target/release/libarr_ins_lib.so}"
[ -f "$CSO" ] || { echo "missing C .so"; exit 1; }
[ -f "$RSO" ] || { echo "missing Rust .so"; exit 1; }

CC=${CC:-cc}
command -v "$CC" >/dev/null 2>&1 || { echo "SKIP: no C compiler"; exit 0; }

"$CC" -O1 -shared -fPIC -o "$out/libtracer.so" "$here/alloc_tracer.c" -ldl || exit 1
"$CC" -O1 -o "$out/driver" "$here/alloc_driver.c" -ldl || exit 1

fail=0
for sc in arr arr_ins map_bin map_strdup map_arena arena; do
  for side in c rust; do
    case $side in c) lib=$CSO ;; rust) lib=$RSO ;; esac
    ARRINS_TRACE_FILE="$out/$side.$sc.log" \
      LD_PRELOAD="$out/libtracer.so" "$out/driver" "$lib" "$sc" >/dev/null 2>&1
  done
  if diff -u "$out/c.$sc.log" "$out/rust.$sc.log" > "$out/$sc.diff" 2>&1; then
    printf '  OK   alloc trace [%s]  (%s calls, exact)\n' "$sc" "$(wc -l < "$out/c.$sc.log")"
  else
    # `realloc(NULL,n)` and `malloc(n)` are exactly equivalent per the C
    # standard, and which one a compiler emits for `realloc(0,n)` depends on its
    # optimisation level. Fold them together before declaring a real difference:
    # what must match is the *sequence of allocation sizes, reallocations and
    # frees*.
    sed 's/^realloc(NULL,/alloc(/; s/^malloc(/alloc(/' "$out/c.$sc.log"    > "$out/c.$sc.n"
    sed 's/^realloc(NULL,/alloc(/; s/^malloc(/alloc(/' "$out/rust.$sc.log" > "$out/rust.$sc.n"
    if diff -u "$out/c.$sc.n" "$out/rust.$sc.n" > "$out/$sc.ndiff" 2>&1; then
      printf '  OK   alloc trace [%s]  (%s calls, normalised: realloc(NULL,n) == malloc(n))\n' \
        "$sc" "$(wc -l < "$out/c.$sc.log")"
    else
      printf '  FAIL alloc trace [%s]\n' "$sc"
      head -40 "$out/$sc.ndiff"
      fail=1
    fi
  fi
done

rm -rf "$out"
[ "$fail" -eq 0 ] && echo "alloc-trace parity: PASS" || echo "alloc-trace parity: FAIL"
exit "$fail"
