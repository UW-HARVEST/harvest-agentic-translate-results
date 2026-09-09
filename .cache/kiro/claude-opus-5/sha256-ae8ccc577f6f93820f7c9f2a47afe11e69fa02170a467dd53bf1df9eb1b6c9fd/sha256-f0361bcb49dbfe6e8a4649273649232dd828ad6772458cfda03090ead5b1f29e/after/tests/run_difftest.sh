#!/bin/bash
# Differential test: build the original C library and the Rust translation,
# compile the same driver against both, and compare their stdout byte for byte.
set -u
ROOT=$HARVEST_WORKDIR
CBUILD=/tmp/jbuild
OUT=/tmp/jdiff

# 1. reference C library (never modified; built out of tree)
if [ ! -f "$CBUILD/libjansson.so" ]; then
    mkdir -p "$CBUILD"
    (cd "$CBUILD" && cmake "$ROOT/c_src" -DCMAKE_BUILD_TYPE=Release >/dev/null && make >/dev/null)
fi

# 2. Rust translation
cd "$ROOT/translation"
cargo build --release 2>&1 | grep -E "^error" -A 10 && exit 1

# 3. symbol surface
mkdir -p "$ROOT/tests/out"
nm -D --defined-only "$CBUILD/libjansson.so" |
    awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"||$2=="W"{print $2" "$3}' | sort -k2 \
    > "$ROOT/tests/out/c_symbols.txt"
nm -D --defined-only "$ROOT/translation/target/release/libjansson.so" |
    awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"||$2=="W"{print $2" "$3}' | sort -k2 \
    > "$ROOT/tests/out/rust_symbols.txt"
nc=$(wc -l < "$ROOT/tests/out/c_symbols.txt")
nr=$(wc -l < "$ROOT/tests/out/rust_symbols.txt")
echo "exported symbols: C=$nc Rust=$nr"
if diff -q "$ROOT/tests/out/c_symbols.txt" "$ROOT/tests/out/rust_symbols.txt" >/dev/null; then
    echo "SYMBOLS: identical (names and types)"
else
    echo "SYMBOLS: DIFFER"
    diff "$ROOT/tests/out/c_symbols.txt" "$ROOT/tests/out/rust_symbols.txt"
fi

# 4. behaviour
mkdir -p "$OUT"
cd "$ROOT"
gcc -O1 -g -I c_src/include -o "$OUT/dt_c" tests/difftest.c \
    -L "$CBUILD" -ljansson -Wl,-rpath,"$CBUILD" -lm || exit 1
gcc -O1 -g -I c_src/include -o "$OUT/dt_r" tests/difftest.c \
    -L "$ROOT/translation/target/release" -ljansson \
    -Wl,-rpath,"$ROOT/translation/target/release" -lm || exit 1
cd "$OUT"
./dt_c > c.out 2> c.err; echo "C exit=$?"
./dt_r > r.out 2> r.err; echo "Rust exit=$?"
echo "output lines: C=$(wc -l < c.out) Rust=$(wc -l < r.out)"
if cmp -s c.out r.out; then
    echo "BEHAVIOUR: outputs are byte-identical"
else
    echo "BEHAVIOUR: DIFFERENCES ($(diff c.out r.out | grep -c '^[<>]') differing lines)"
    diff c.out r.out | head -40
fi

# 5. same again under glibc heap checking
MALLOC_CHECK_=3 MALLOC_PERTURB_=170 ./dt_c > c2.out 2>/dev/null
MALLOC_CHECK_=3 MALLOC_PERTURB_=170 ./dt_r > r2.out 2>/dev/null
cmp -s c2.out r2.out && echo "HEAP-CHECKED: outputs are byte-identical" \
                     || echo "HEAP-CHECKED: DIFFERENCES"

[ -s r.err ] && { echo "--- rust stderr ---"; head -20 r.err; }
[ -s c.err ] && { echo "--- c stderr ---"; head -20 c.err; }
exit 0
