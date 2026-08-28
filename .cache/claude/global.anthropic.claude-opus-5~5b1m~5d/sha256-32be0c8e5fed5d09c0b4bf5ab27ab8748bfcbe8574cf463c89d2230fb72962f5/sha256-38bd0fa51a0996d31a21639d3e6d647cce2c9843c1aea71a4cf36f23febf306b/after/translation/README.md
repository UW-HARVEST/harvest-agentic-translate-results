# Rust translation of `c_src` (cute_png-derived PNG/DEFLATE decoder)

`src/lib.rs` is a literal, ABI-compatible translation of `c_src/src/lib.c`.
It is built as a `cdylib` and exports exactly the same public symbols as the
shared library produced by `c_src/CMakeLists.txt`.

## Exported ABI

`nm -D --defined-only` on both libraries yields the same 9 public symbols with
identical sizes and types:

| symbol | type | size |
|---|---|---|
| `load_png_mem` | FUNC | - |
| `cp_inflate` | FUNC | - |
| `cp_error_reason` | OBJECT | 8 |
| `cp_fixed_table` | OBJECT | 320 |
| `cp_permutation_order` | OBJECT | 19 |
| `cp_len_extra_bits` | OBJECT | 31 |
| `cp_len_base` | OBJECT | 124 |
| `cp_dist_extra_bits` | OBJECT | 32 |
| `cp_dist_base` | OBJECT | 128 |

There are no namespace/renaming macros in the C headers, so the linker names
equal the source-level names. The six tables and `cp_error_reason` are mutable
globals in C and are `static mut` here; the decoder reads them through raw
pointers on every call, so a caller that mutates them changes behaviour exactly
as it does in C.

`cp_image_t` (16 bytes: `int w; int h; cp_pixel_t *pix;`) is `#[repr(C)]` and
returned by value from `load_png_mem`, matching the SysV AMD64 register return
(RAX/RDX). `cp_state_t` is `#[repr(C)]` and its layout was checked against the
C compiler (size 2464, `lookup` @72, `lit` @1096, `dst` @2248, `len` @2376) --
this matters because `cp_decode` reads `tree[-1]` when a Huffman tree is empty,
i.e. it reads the *preceding struct field*. See the unit tests in `src/lib.rs`.

Memory is allocated with libc `malloc`/`calloc` and released with libc `free`
(declared via `unsafe extern "C"`), so pointers handed back to callers
(`img.pix`) can be `free()`d by C code, and `cp_inflate`'s alignment-sensitive
logic (`first_bytes = ((in + 3) & ~3) - in`) sees the same pointer alignments.

## Fidelity notes

Bugs and quirks of the C are reproduced deliberately, including:

* `w = cp_make32(ihdr) + 1` (the +1 in the C source) and `img.w = w - 1`.
* `cp_stored`'s `s->bits_left / 8 <= LEN` check, which rejects any stored
  DEFLATE block that is not the last thing in the stream, and its `memcpy` that
  ignores `out_end`.
* `cp_unfilter`'s first row: filter 2 is a no-op, filter 4 uses
  `cp_paeth(raw[x-bpp], 0, 0)`, filter 1/3/4 start at `x = bpp`.
* `cp_decode`'s `tree[lo - 1]` read when `hi == 0`.
* Signed vs unsigned pointer arithmetic: `cp_chunk` uses `int offset = len + 12`
  (sign-extended, so a chunk length >= 0x80000000 walks *backwards*), while
  `cp_find` uses `png->p += len + 12` (uint32, zero-extended).
* All integer truncation/wrap-around (`datalen += len` through `uint32`,
  `pix_bytes` truncated to `int`, `cp_make32` shifting into the sign bit, ...).
* The exact order of every validation check and the exact error strings,
  including the misspelled `"innapropriate window size detected"`.

`assert()` is treated as a no-op (i.e. `NDEBUG`, which is what
`-DCMAKE_BUILD_TYPE=Release` gives). Against such a C build the two libraries
are byte-for-byte identical on every input tested. If the C library is built
*with* asserts enabled it aborts on some malformed streams where this port (and
the Release C build) returns an error instead; no valid PNG is affected.

### The `.data` layout of the exported tables

`cp_block` indexes `cp_len_extra_bits`/`cp_len_base` with `symbol - 257` and the
`cp_dist_*` tables with a distance symbol. When a corrupt Huffman tree makes
`cp_decode` return an out-of-range symbol (up to 4095 -- e.g. an all-zero
distance code-length alphabet makes it read `tree[-1]`), the C reads *past* the
end of a table into whichever table the linker placed next. In the reference
library those six tables are the entire `.data` section -- 672 bytes, each table
32-byte aligned, in reverse source order, gap bytes zero:

```
   0  cp_dist_base (128)      160  cp_len_base (124) + 4 gap
 128  cp_dist_extra_bits (32) 288  cp_len_extra_bits (31) + 1 gap
 320  cp_permutation_order (19) + 13 gap      352  cp_fixed_table (320)
```

Rust/LLVM order and align statics differently and that order cannot be
controlled portably, so the four indexed reads resolve their index through a
model of the layout above (`blob_byte`) instead of walking off the end of a Rust
static. The model reads the *live* statics, so a caller that mutates a table
still affects out-of-range reads just as in C; offsets past the blob (where the
C reads unrelated `.bss`) yield 0. `cargo test` checks the model against the
reference layout, including `cp_dist_extra_bits[32] == 3` (the LSB of
`cp_len_base[0]`), `cp_dist_base[32] == 0` and `cp_len_extra_bits[31] == 0`.

One case remains outside anyone's control: if the *consumer* declares the tables
`extern` and links against the library, the linker emits copy relocations, the
tables move into the consumer's `.bss`, and the C's out-of-range reads then
depend on the consumer's layout (which itself derives from the alignments the
producing library happened to use). Consumers that only call the two functions,
or use `dlopen`/`dlsym`, are unaffected.

### Deliberate deviations (all of them C undefined behaviour)

Four local arrays are padded / zero-initialised so that malformed input cannot
cause Rust UB. Every one of these is a place where the C reads or writes
outside an object, with compiler-layout-dependent results, and three of the four
are guarded by the C's own `assert`s:

1. `cp_build`: `int codes[16], first[16], counts[16]` -> 256 entries, all
   zero-initialised. Reachable only when a code length exceeds 15, which the C
   catches with `assert(len < 16)`; with `NDEBUG` it indexes past the arrays and
   uses *uninitialised* `first[len]`/`codes[len]`.
2. `cp_dynamic`: `uint8_t lens[288 + 32]` -> `[288 + 32 + 256]`. The 16/17/18
   run-length symbols can push the write index up to 137 entries past the end of
   the C array (a stack overrun).
3. `cp_dynamic`: `lens[n - 1]` at `n == 0` reads an indeterminate value in C
   (the stack byte below an uninitialised array); 0 is used here, which is what
   gcc actually leaves there at `-O0` through `-O3` (at `-Os` the slot happens to
   hold a leftover code length).
4. `cp_dynamic`: `uint8_t lenlens[19]` -> 256 entries, in case a caller mutates
   `cp_permutation_order` to hold out-of-range indices.

Out-of-bounds *reads* that the C performs on real objects (past the end of a
`PLTE` chunk, past the input buffer, `tree[-1]`, table lookups driven by a
corrupt Huffman tree) are reproduced with raw pointers rather than being
"fixed".

## Verification

A differential harness (compiled twice, once against each `.so`, and comparing
stdout byte-for-byte) was run over:

* 895 crafted PNG files - all 5 colour types, all 5 filter types, palette +
  `tRNS` variants, split/empty IDATs, ancillary chunks, stored/fixed/dynamic
  DEFLATE blocks, 10 zlib header variants, plus ~50 malformed files covering
  every reachable error string;
* 24 hand-built raw DEFLATE streams x 4 input alignments x 7 output sizes
  (exercising `cp_ptr`, the final-word path, empty Huffman trees, back-reference
  distances with extra bits, and the `memset` fast path);
* edge calls: NULL/negative/truncated lengths, repeated calls, and runtime
  mutation of each exported table (which changes the decoded output identically
  in both libraries);
* ~8,900 fuzzed inputs (byte flips, truncations, splices of the valid corpus,
  and random streams).

The harness was run in all three consumer shapes -- linked without referencing
the tables, linked *with* `extern` table declarations (copy relocations), and
`dlopen`/`dlsym` -- and, for the C side, at `-O0/-O1/-O2/-O3/-Os` and with and
without `NDEBUG`.

Result: identical output on every case except those where the C's behaviour is
undefined (the four items above) or nondeterministic in the C itself (hashing
malloc memory that a short image never fills, or reads far outside the input
buffer -- these differ between two runs of the *same* C library). Against a C
build with only those four UB sites made well-defined, every fuzz case and the
entire crafted corpus match exactly.

`gcov` on the C library under this corpus reports **98.3% of lines and 100% of
branches executed** (96.2% taken at least once). The only unexecuted lines are
`cp_would_overflow` (dead code once `assert` is compiled out), the
`malloc`-failure path `"unable to allocate raw image space"`, and the two
`"invalid image size found"` checks, which are unreachable because the earlier
`"image too large"` check guarantees `cp_out_size() >= 4`.
