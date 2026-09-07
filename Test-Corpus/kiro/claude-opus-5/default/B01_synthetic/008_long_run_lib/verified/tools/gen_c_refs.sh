#!/usr/bin/env bash
# Regenerate tests/cref/*.txt -- the recorded stdout of the **C** shared object.
#
# One `long_exec` call in the C library performs 2000 * 262144 * 100 = 5.24e10
# kernel applications and takes roughly 8 minutes, so the seed sweep in
# CONFIGS.md rows 20-33 is recorded once (in parallel) rather than re-run live for
# every seed on every `cargo test`.  Each file is produced by dlopen()ing the C
# .so under test and calling its exported `long_exec`, i.e. it is genuine C
# ground truth, not a hand-written constant.
#
# Usage:  tools/gen_c_refs.sh [seed ...]
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
crate="$(dirname "$here")"
root="$(dirname "$crate")"
cso="${LONG_C_SO:-$root/c_src/build/liblong.so}"
out="$crate/tests/cref"
mkdir -p "$out"

[ -f "$cso" ] || {
    echo "C .so not found at $cso -- build it with:" >&2
    echo "  cd $root/c_src && mkdir -p build && cd build && \\" >&2
    echo "    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
    exit 1
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

cat > "$tmp/drv.c" <<'EOF'
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
// argv: <so> <seed> [array-dump-path]
int main(int argc, char **argv) {
    void *h = dlopen(argv[1], RTLD_NOW);
    if (!h) { fprintf(stderr, "%s\n", dlerror()); return 1; }
    void (*fn)(unsigned int) = dlsym(h, "long_exec");
    int *arr = dlsym(h, "array");
    if (!fn || !arr) { fprintf(stderr, "symbol missing\n"); return 1; }
    fn((unsigned int)strtoul(argv[2], NULL, 10));
    fflush(stdout);
    if (argc > 3) {
        FILE *f = fopen(argv[3], "wb");
        if (!f) { perror("fopen"); return 1; }
        fwrite(arr, 4, 256*1024, f);
        fclose(f);
    }
    return 0;
}
EOF
cc -O2 "$tmp/drv.c" -o "$tmp/drv" -ldl

# Seeds whose full post-run `array` object is also dumped (1 MiB each), so the
# differential can compare the whole state and not just the printed XOR.
array_seeds=(0 7 4294967295)

seeds=("$@")
if [ ${#seeds[@]} -eq 0 ]; then
    seeds=(0 1 2 3 5 7 9 11 13 17 23 29 37 42 99 100 254 255 256 777 1000 4096
           8191 8192 8193 12345 31337 32768 65535 65536 66666 424242 999983
           1048576 7777777 16777216 88888888 305419896 123456789 555555555
           987654321 1073741824 1234567890 2000000000 2147483647 2147483648
           2147483649 2718281828 2863311530 1431655765 3000000000 3141592653
           3221225472 4000000000 4026531840 4294967294 4294967295)
fi

want_array() {
    local s="$1"
    for a in "${array_seeds[@]}"; do [ "$a" = "$s" ] && return 0; done
    return 1
}

echo "recording ${#seeds[@]} seeds from $cso (in parallel; ~8 min wall)" >&2
for s in "${seeds[@]}"; do
    if want_array "$s"; then
        ( "$tmp/drv" "$cso" "$s" "$out/arr_$s.bin" > "$out/c_$s.txt" ) &
    else
        ( "$tmp/drv" "$cso" "$s" > "$out/c_$s.txt" ) &
    fi
done
wait
echo "wrote ${#seeds[@]} files to $out" >&2
