#!/bin/sh
# Regenerates src/c_data_image.bin.
#
# `stb_perlin_noise3_wrap_nonpow2` can index the static tables out of bounds
# (see ERRORS.md). To reproduce what the C program then reads, src/stb_perlin.rs
# embeds a copy of the mapped image of the reference C process,
# 0x400000..0x406000, captured while the process is blocked inside scanf -- the
# exact state the noise routines observe.
#
# Run from the repository root (the directory containing c_src/ and
# translation/), after building the C program with
#   cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
#
# Sanity check afterwards: `cargo test` runs
# `stb_perlin::tests::data_image_agrees_with_the_tables`, which asserts the blob
# still agrees with the tables transcribed from stb_perlin.h.
set -eu

C_BIN=./c_src/build/driver
OUT=./translation/src/c_data_image.bin
FIFO=$(mktemp -u)

# The image is only mapped where the reference build puts it: a non-PIE
# executable based at 0x400000 whose last mapped page ends at 0x406000.
readelf -lW "$C_BIN" | grep -q '0x0000000000400000' || {
    echo "$C_BIN is not the expected non-PIE build based at 0x400000" >&2
    exit 1
}

mkfifo -m 600 "$FIFO"
# Keep the writer alive long enough for the process to block in scanf.
{ sleep 4; echo "0 0 0 0 0 0 0 0 0 0 0 0"; } > "$FIFO" &
writer=$!
"$C_BIN" < "$FIFO" > /dev/null &
target=$!
sleep 1

python3 - "$target" "$OUT" <<'PY'
import sys
pid, out = sys.argv[1], sys.argv[2]
with open(f"/proc/{pid}/mem", "rb", 0) as mem:
    mem.seek(0x400000)
    data = mem.read(0x6000)
assert len(data) == 0x6000, len(data)
with open(out, "wb") as f:
    f.write(data)
print(f"wrote {len(data)} bytes to {out}")
PY

kill "$target" "$writer" 2>/dev/null || true
rm -f "$FIFO"

# Note: a handful of bytes in this image (.got, .got.plt and one .dynamic entry)
# hold pointers into libc/ld.so, which ASLR randomises per run. Out-of-bounds
# reads that land on them make the *C program* nondeterministic, so no capture
# can pin them down; see the last section of ERRORS.md.
