#!/usr/bin/env bash
# Phase A / Phase D symbol diff: every symbol the C .so exports must also be
# exported by the Rust .so, with the exact same name. Exits non-zero if the diff
# is not empty.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
c_so="${DIFFTEST_C_SO:-$root/c_src/build/libdriver.so}"
profile="${1:-release}"
rust_so="${DIFFTEST_RUST_SO:-$here/target/$profile/libdriver.so}"

if [[ ! -f $c_so ]]; then
    echo "missing C library: $c_so" >&2
    echo "build it: cd c_src && mkdir -p build && cd build &&" \
         "cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
    exit 2
fi
if [[ ! -f $rust_so ]]; then
    echo "missing Rust library: $rust_so" >&2
    echo "build it: cd translation && cargo build --$profile" >&2
    exit 2
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$c_so"    | awk '{print $NF}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u > "$tmp/r.txt"

echo "C   .so exports $(wc -l < "$tmp/c.txt") symbols: $c_so"
echo "Rust .so exports $(wc -l < "$tmp/r.txt") symbols: $rust_so"

missing=$(comm -23 "$tmp/c.txt" "$tmp/r.txt")
extra=$(comm -13 "$tmp/c.txt" "$tmp/r.txt")

rc=0
if [[ -n $missing ]]; then
    echo "MISSING from the Rust .so (must be 0):" >&2
    echo "$missing" | sed 's/^/  /' >&2
    rc=1
else
    echo "MISSING from the Rust .so: none"
fi
if [[ -n $extra ]]; then
    echo "EXTRA in the Rust .so:" >&2
    echo "$extra" | sed 's/^/  /' >&2
    rc=1
else
    echo "EXTRA in the Rust .so: none"
fi

# Undefined symbols in the Rust object must all be satisfied at load time.
if ldd -r "$rust_so" 2>&1 | grep -E 'undefined symbol|not found'; then
    echo "unresolved symbols in the Rust .so" >&2
    rc=1
else
    echo "Unresolved (non-libc) symbols in the Rust .so: none"
fi

# Codegen parity of the four exported functions, which is what makes the
# uninitialised-read behaviour comparable at all.
#
# Normalisation, matching tests/configs.rs::disasm: stop at the function's `ret`
# (so trailing int3 padding is ignored), drop instruction addresses and the
# "# addr <sym>" comments objdump appends, collapse RIP-relative displacements
# (which legitimately differ), and reduce call/jmp targets to the callee name
# (or `local` for an intra-function branch).
norm() {
    objdump -d --no-show-raw-insn "$1" \
        | awk -v f="$2" '
            $0 ~ "<"f">:" { inside=1; next }
            !inside { next }
            NF==0 { exit }
            {
                sub(/^[^\t]*\t/, "");          # drop "  1139:\t"
                sub(/#.*$/, "");               # drop objdump comment
                gsub(/-?0x[0-9a-f]+\(%rip\)/, "DISP(%rip)");
                # call/jmp: keep the callee name, drop its address; an
                # intra-function target like <driver+0x1d> becomes "local".
                if ($0 ~ /^(call|jmp|j[a-z]+)[[:space:]]/) {
                    mnem = $1;
                    if (match($0, /<[^>]*>/)) {
                        tgt = substr($0, RSTART+1, RLENGTH-2);
                        if (index(tgt, "+") > 0) tgt = "local";
                    } else {
                        tgt = "local";
                    }
                    $0 = mnem " " tgt;
                }
                gsub(/^[[:space:]]+|[[:space:]]+$/, "");
                gsub(/[[:space:]]+/, " ");
                print;
                if ($0 ~ /^ret/) exit;
            }'
}
for f in printIntPtrLine bad good driver; do
    if diff <(norm "$c_so" "$f") <(norm "$rust_so" "$f") > "$tmp/diff.$f"; then
        echo "Codegen parity for $f: identical"
    else
        echo "Codegen DIFFERS for $f:" >&2
        sed 's/^/  /' "$tmp/diff.$f" >&2
        rc=1
    fi
done

exit $rc
