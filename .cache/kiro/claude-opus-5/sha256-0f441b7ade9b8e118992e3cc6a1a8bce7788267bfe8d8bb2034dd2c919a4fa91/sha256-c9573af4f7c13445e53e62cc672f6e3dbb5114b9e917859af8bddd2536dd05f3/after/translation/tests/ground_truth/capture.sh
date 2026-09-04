#!/usr/bin/env bash
# Capture C ground truth for the full `long_exec` pipeline.
#
# `long_exec` runs 2000 * 262144 * 100 kernel steps, which takes ~470 s per seed
# in the unoptimised C build.  That is too slow to run inside `cargo test`, so
# the C reference output is recorded here, once, and the differential tests in
# tests/configs.rs compare the Rust `.so`'s live output against these bytes.
#
# Everything recorded comes from dlopen'ing the *C* shared object and calling its
# exported `long_exec` / reading its exported `array`.  Nothing Rust is involved,
# so these files are genuine C ground truth.
#
# Usage:  tests/ground_truth/capture.sh [seed ...]
# Output: c_<seed>.out   exact stdout bytes printed by the C long_exec
#         arr_<seed>.bin final 1 MiB image of the C `array` (little-endian int32)

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
CLIB="${C_LIB:-$ROOT/c_src/build/liblong.so}"

if [[ ! -f "$CLIB" ]]; then
  echo "C library not found at $CLIB" >&2
  echo "build it with: cd c_src && mkdir -p build && cd build &&" >&2
  echo "  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
  exit 1
fi

SEEDS=("$@")
if [[ ${#SEEDS[@]} -eq 0 ]]; then
  SEEDS=(0 1 3 7 42 100 255 12345 65535 999983 2147483648 4294967295)
fi
# Seeds for which the final `array` image is recorded as well.
ARRAY_SEEDS=(0 42 4294967295)

BIN="$(mktemp -d)/capture"
cat > "$BIN.c" <<'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
int main(int argc, char **argv) {
    /* argv: <c.so> <seed> <stdout-file> [array-file] */
    void *h = dlopen(argv[1], RTLD_NOW);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 2; }
    void (*long_exec)(unsigned int) = dlsym(h, "long_exec");
    int *array = (int *)dlsym(h, "array");
    if (!long_exec || !array) { fprintf(stderr, "dlsym failed\n"); return 3; }
    unsigned int seed = (unsigned int)strtoul(argv[2], NULL, 10);
    if (!freopen(argv[3], "w", stdout)) { perror("freopen"); return 4; }
    long_exec(seed);
    fflush(stdout);
    if (argc > 4) {
        FILE *g = fopen(argv[4], "wb");
        if (!g) { perror("fopen"); return 5; }
        fwrite(array, sizeof(int), 256 * 1024, g);
        fclose(g);
    }
    return 0;
}
EOF
cc -O2 "$BIN.c" -o "$BIN" -ldl

pids=()
for s in "${SEEDS[@]}"; do
  want_array=""
  for a in "${ARRAY_SEEDS[@]}"; do
    [[ "$a" == "$s" ]] && want_array="$HERE/arr_$s.bin"
  done
  if [[ -n "$want_array" ]]; then
    "$BIN" "$CLIB" "$s" "$HERE/c_$s.out" "$want_array" &
  else
    "$BIN" "$CLIB" "$s" "$HERE/c_$s.out" &
  fi
  pids+=($!)
  echo "started seed $s (pid ${pids[-1]})"
done

fail=0
for p in "${pids[@]}"; do wait "$p" || fail=1; done
[[ $fail -eq 0 ]] || { echo "at least one capture failed" >&2; exit 1; }

echo "--- captured ---"
for s in "${SEEDS[@]}"; do printf '%-12s %s' "$s" "$(cat "$HERE/c_$s.out")"; echo; done
