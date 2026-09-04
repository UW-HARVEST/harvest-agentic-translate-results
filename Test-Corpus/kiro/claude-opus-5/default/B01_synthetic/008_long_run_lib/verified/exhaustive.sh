#!/usr/bin/env bash
# Exhaustive differential verification of `perform_expensive_operations` over
# ALL 2^32 possible `int` inputs.
#
# The function is `f^100` applied element-wise and depends on nothing but the
# element value, so feeding every 32-bit value through both shared objects is a
# complete proof of kernel equivalence -- not a sample.  2^32 values / 262144
# elements = 16384 array-fulls, sharded across processes.
#
#   usage: ./exhaustive.sh [num_shards]     (default 24)
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
CSO="${C_LIB:-$ROOT/c_src/build/liblong.so}"
RSO="${RUST_LIB:-$HERE/target/release/liblong.so}"
NS="${1:-24}"

for so in "$CSO" "$RSO"; do
  [[ -f "$so" ]] || { echo "missing $so" >&2; exit 1; }
done

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cat > "$TMP/ex.c" <<'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#define N (256*1024)
int main(int argc, char **argv) {
    /* argv: <c.so> <rust.so> <shard> <nshards> */
    long shard = atol(argv[3]), nshard = atol(argv[4]);
    void *hc = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!hc) { fprintf(stderr, "dlopen C: %s\n", dlerror()); return 2; }
    void *hr = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (!hr) { fprintf(stderr, "dlopen Rust: %s\n", dlerror()); return 2; }
    void (*pc)(void) = dlsym(hc, "perform_expensive_operations");
    void (*pr)(void) = dlsym(hr, "perform_expensive_operations");
    int *ac = (int *)dlsym(hc, "array");
    int *ar = (int *)dlsym(hr, "array");
    if (!pc || !pr || !ac || !ar) { fprintf(stderr, "dlsym failed\n"); return 3; }
    int *in = malloc((size_t)N * 4);
    long total = 4294967296L / N;   /* 16384 */
    long bad = 0, done = 0;
    for (long c = shard; c < total; c += nshard) {
        unsigned int base = (unsigned int)(c * N);
        for (long i = 0; i < N; i++) in[i] = (int)(base + (unsigned int)i);
        memcpy(ac, in, (size_t)N * 4);
        memcpy(ar, in, (size_t)N * 4);
        pc();
        pr();
        if (memcmp(ac, ar, (size_t)N * 4) != 0)
            for (long i = 0; i < N; i++)
                if (ac[i] != ar[i]) {
                    if (bad < 5) fprintf(stderr, "MISMATCH in=%d C=%d Rust=%d\n", in[i], ac[i], ar[i]);
                    bad++;
                }
        done++;
    }
    printf("shard %ld/%ld chunks=%ld mismatches=%ld\n", shard, nshard, done, bad);
    return bad ? 1 : 0;
}
EOF
cc -O2 "$TMP/ex.c" -o "$TMP/ex" -ldl

echo "### exhaustive f^100 differential over all 2^32 int inputs, $NS shards"
pids=()
for i in $(seq 0 $((NS - 1))); do
  "$TMP/ex" "$CSO" "$RSO" "$i" "$NS" > "$TMP/out_$i" 2> "$TMP/err_$i" &
  pids+=($!)
done
fail=0
for p in "${pids[@]}"; do wait "$p" || fail=1; done

cat "$TMP"/out_* | sort -V
chunks=$(awk -F'chunks=' '{split($2,a," ");s+=a[1]}END{print s}' "$TMP"/out_*)
bad=$(awk -F'mismatches=' '{s+=$2}END{print s}' "$TMP"/out_*)
cat "$TMP"/err_* 2>/dev/null | head -20
echo "### total chunks=$chunks (expected 16384)  total mismatches=$bad"
[[ "$chunks" == "16384" && "$bad" == "0" && $fail -eq 0 ]] \
  && { echo "### EXHAUSTIVE: all 2^32 inputs identical"; exit 0; } \
  || { echo "### EXHAUSTIVE: FAILED"; exit 1; }
