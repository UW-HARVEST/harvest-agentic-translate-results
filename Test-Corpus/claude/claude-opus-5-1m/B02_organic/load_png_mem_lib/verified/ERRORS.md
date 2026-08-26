# ERRORS.md — error-surface table (Phase A / Phase C)

Derived mechanically from `c_src/src/lib.c`:

```sh
grep -n 'cp_error_reason'                     c_src/src/lib.c   # 26 assignments
grep -n 'return 0;\|return NULL;\|goto cp_err' c_src/src/lib.c
grep -n 'assert'                              c_src/src/lib.c   # 10 assertions
```

`load_png_mem` signals failure by returning `cp_image_t { w, h, pix = NULL }`
(note: `w`/`h` keep whatever value they had reached — they are **not** reset)
and by setting the global `cp_error_reason`. `cp_inflate` signals failure by
returning `0`; the string in `cp_error_reason` identifies which branch fired.

Every row is covered by a differential test in `tests/errors.rs` that constructs
the exact condition, calls **both** `.so`s through `libloading` and asserts the
same sentinel **and** the same `cp_error_reason` string (pointer *contents*, not
pointer value). Rows whose C behaviour is `abort()` are driven through
`fork()`/`waitpid()` (`run_forked_pair`), and for those the C side's
`__assert_fail` message is checked too, which proves that the *intended*
assertion fired and not some other one.

**`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and never defines `NDEBUG`,
so all 10 `assert()`s are live in the C shared library** (`nm -D` shows it
importing `__assert_fail`). A failing assertion is therefore part of the
observable behaviour, and the Rust translation reproduces each one with an
explicit check that calls libc `abort()` (`cp_assert_fail`).

Status: **52 / 52 rows pass** (`cargo test --test errors`, 43 test functions).

## A. `load_png_mem` rejections

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `load_png_mem` (lib.c:528) | `memcmp(png.p, "\211PNG\r\n\032\n", 8) != 0` — any of the 8 signature bytes wrong (all 8 positions x 3 bit patterns, plus all-zero / all-`0xFF` / `"GIF89a"` buffers) | `pix=NULL`, `w=0`, `h=0`, `"incorrect file signature (is this a png file?)"` | `row_1_bad_signature` | [x] |
| 2 | `cp_chunk` (lib.c:381) | the chunk name at `png.p+4` is not `"IHDR"` (`iHDR`, `IHDr`, `XHDR`, `\0\0\0\0`, `IDAT`, `IHD\0`) | `pix=NULL`, `"unable to find IHDR chunk"` | `row_2_4_ihdr_chunk` | [x] |
| 3 | `cp_chunk` (lib.c:381) | IHDR declared length `< 13` (`len >= minlen` fails) — all of `0..=12`; `13` is accepted | `pix=NULL`, `"unable to find IHDR chunk"` | `row_2_4_ihdr_chunk` | [x] |
| 4 | `cp_chunk` (lib.c:383) | `png->p + len + 12 > png->end` — `png_length` in `0..=32` (a signature + IHDR needs 33 bytes), and a declared IHDR length that runs past the end (`13+1`, `13+2`, `13+100`, `13+0xFFFF`, `13+0x7FFFFFFF`, wrap-around) | `pix=NULL`, `"unable to find IHDR chunk"` | `row_2_4_ihdr_chunk` | [x] |
| 5 | `load_png_mem` (lib.c:548) | `ihdr[8] != 8` — **all 255 non-8 byte values** | `pix=NULL`, `"only bit-depth of 8 is supported"` | `row_5_bit_depth` | [x] |
| 6 | `load_png_mem` (lib.c:555-580) | `ihdr[9] ∉ {0,2,3,4,6}` — **all 251 out-of-range byte values** (out-of-range "enum" across the FFI boundary) | `pix=NULL`, `"unknown color type"` | `row_6_colour_type` | [x] |
| 7 | `load_png_mem` (lib.c:584) | `w = cp_make32(ihdr)+1 < 1`: declared `0xFFFFFFFF` (→`w==0`), `0x7FFFFFFF` (→`INT_MIN`), `0x80000000`, `0xC0000000`, `0xABCDEF01`, `0xFFFFFFFE`. Declared `0` gives `w==1` and is **valid** (a 0-pixel-wide image, `img.w == 0`) | `pix=NULL`, `"invalid IHDR chunk found, image width was less than 1"` | `row_7_width_less_than_one` | [x] |
| 8 | `load_png_mem` (lib.c:592) | `h = cp_make32(ihdr+4) < 1`: declared `0`, `0x80000000`, `0x92345678`, `0xFFFFFFFE`, `0xFFFFFFFF`. `1` is accepted | `pix=NULL`, `"invalid IHDR chunk found, image height was less than 1"` | `row_8_height_less_than_one` | [x] |
| 9 | `load_png_mem` (lib.c:601) | `!((int64_t)w*h*sizeof(cp_pixel_t) < INT_MAX)`, i.e. `w*h >= 2^29`: `(65536,8192)`, `(2^29,1)`, `(INT_MAX,1)`, `(65535,65535)`, `(1,INT_MAX)`, `(2^29,2^29)`, and the exact boundary `w*4 == 2147483644` (accepted) vs `2^31` (rejected) | `pix=NULL`, `"image too large"` | `row_9_10_image_too_large` | [x] |
| 10 | `load_png_mem` (lib.c:613) | `malloc(pix_bytes) == NULL`. Only reachable at the `INT_MAX` boundary (a ~2 GiB request); the test accepts either `"unable to allocate raw image space"` or the next check's message, and requires the two libraries to agree | `pix=NULL`, `"unable to allocate raw image space"` | `row_9_10_image_too_large` | [x] |
| 11 | `load_png_mem` (lib.c:624) | `ihdr[10] != 0` — **all 255 values**. `img.w`/`img.h` are **already set** and survive the error path (asserted) | `pix=NULL`, `w=4`, `h=4`, `"only standard compression DEFLATE is supported"` | `row_11_13_ihdr_method_bytes` | [x] |
| 12 | `load_png_mem` (lib.c:632) | `ihdr[11] != 0` — all 255 values | `pix=NULL`, `w`/`h` set, `"only standard adaptive filtering is supported"` | `row_11_13_ihdr_method_bytes` | [x] |
| 13 | `load_png_mem` (lib.c:640) | `ihdr[12] != 0` — all 255 values, including the legal Adam7 value `1` | `pix=NULL`, `w`/`h` set, `"interlacing is not supported"` | `row_11_13_ihdr_method_bytes` | [x] |
| 14 | `load_png_mem` (lib.c:674) | `!(data && datalen >= 6)`: no IDAT chunk at all (`datalen == 0`, `malloc(0) != NULL`), for every colour type | `pix=NULL`, `"corrupt zlib structure in DEFLATE stream"` | `row_14_15_short_or_missing_idat` | [x] |
| 15 | `load_png_mem` (lib.c:674) | `!(data && datalen >= 6)`: IDAT payload total `0..=5` bytes, spread over 1..3 chunks. **`datalen == 6` passes this check and then calls `cp_inflate` with `in_bytes == 0`, which aborts** (see row 36) | `pix=NULL`, `"corrupt zlib structure in DEFLATE stream"` | `row_14_15_short_or_missing_idat` | [x] |
| 16 | `load_png_mem` (lib.c:682) | `(data[0] & 0x0f) != 0x08` — **all 240 CMF byte values** whose low nibble is not 8 | `pix=NULL`, `"only zlib compression method (RFC 1950) is supported"` | `row_16_zlib_method` | [x] |
| 17 | `load_png_mem` (lib.c:690) | `(data[0] & 0xf0) > 0x70` — CINFO `8..=15`; CINFO `7` is the accepted boundary | `pix=NULL`, `"innapropriate window size detected"` | `row_17_zlib_window` | [x] |
| 18 | `load_png_mem` (lib.c:698) | `data[1] & 0x20` (FDICT) — **all 256 FLG values**, asserting rejection iff bit 5 is set | `pix=NULL`, `"preset dictionary is present and not supported"` | `row_18_zlib_preset_dictionary` | [x] |
| 19 | `load_png_mem` (lib.c:706) | `cp_out_size(&img,4) < 1`. **Unreachable**: rows 7/8/9 already force `w>=1`, `h>=1`, `(int64_t)w*h*4 < INT_MAX`, so the product is in `[4, INT_MAX)`. The test sweeps the whole reachable `w`/`h` boundary set (`w ∈ 1..4`, `h ∈ 1..3`, all colour types, all filters) and asserts neither library ever takes the branch | `"invalid image size found"` (unreachable) | `row_19_20_out_size_unreachable` | [x] |
| 20 | `load_png_mem` (lib.c:714) | `cp_out_size(&img,bpp) < 1` — unreachable for the same reason (`bpp ∈ {1,2,3,4}`) | `"invalid image size found"` (unreachable) | `row_19_20_out_size_unreachable` | [x] |
| 21 | `load_png_mem` (lib.c:723) | `cp_inflate(...)` returns 0. Three constructions: `BTYPE=3`; a stored block with non-complementary `LEN`/`NLEN`; a stream that decompresses to more than `pix_bytes`. Note the inner reason is **overwritten** | `pix=NULL`, `"DEFLATE algorithm failed"` | `row_21_deflate_failed` | [x] |
| 22 | `cp_unfilter` (lib.c:424) | row-0 filter byte `> 4` — **all 251 values x all 5 colour types** | `pix=NULL`, `"invalid filter byte found"` | `row_22_23_bad_filter_byte` | [x] |
| 23 | `cp_unfilter` (lib.c:458) | the filter byte of row 1 or row 2 (`y >= 1`) is `> 4` — all 251 values x all 5 colour types. Filter `4` is the accepted boundary | `pix=NULL`, `"invalid filter byte found"` | `row_22_23_bad_filter_byte` | [x] |
| 24 | `load_png_mem` (lib.c:740) | `color_type == 3` and `cp_find("PLTE", 0)` returns NULL: no PLTE at all, and a PLTE that only appears *after* the IDATs. A **zero-length** PLTE *is* found (`minlen == 0`) and accepted | `pix=NULL`, `"color type of indexed requires a PLTE chunk"` | `row_24_indexed_without_plte` | [x] |

## B. `cp_inflate` rejections (also reachable through `load_png_mem`, row 21)

Each row is checked at **all four input alignments**.

| # | function | trigger | expected C result | test | [x] |
|---|----------|---------|-------------------|------|-----|
| 25 | `cp_stored` (lib.c:161) | stored block with `LEN != (uint16_t)~NLEN` — `LEN ∈ {0,1,3,8,64}` x `NLEN ∈ {0x0000,0xFFFF,0x1234,0x5555}` | `0`, `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | `row_25_stored_len_nlen_mismatch` | [x] |
| 26 | `cp_stored` (lib.c:170) | `!(s->bits_left / 8 <= LEN)` — the check is written *backwards*, so it fires whenever **more** input remains than the block declares: every multi-stored-block stream (`1+1`, `4+4`, `16+3`, `100+100`) and a single stored block with 1..8 trailing bytes | `0`, `"Stored block extends beyond end of input stream."` | `row_26_stored_extends_beyond_input` | [x] |
| 27 | `cp_block` (lib.c:245) | a literal decoded while `s->out + 1 > s->out_end` — `n` literals with `out_bytes ∈ 0..n` for `n = 1..8`; `out_bytes == n` succeeds | `0`, `"Attempted to overwrite out buffer while outputting a symbol."` | `row_27_31_literal_overflows_output` | [x] |
| 28 | `cp_block` (lib.c:264) | `s->out - backwards_distance < s->begin` — `(0 literals, dist 1)`, `(1, 2)`, `(1, 5)`, `(3, 4)`, `(3, 300)`, `(8, 32768)` | `0`, `"Attempted to write before out buffer (invalid backwards distance)."` | `row_28_backwards_distance_before_begin` | [x] |
| 29 | `cp_block` (lib.c:273) | `s->out + length > s->out_end` — 4 literals then a match of length 3/4/17/258 with every `out_bytes` in `4..4+length`; `4+length` succeeds | `0`, `"Attempted to overwrite out buffer while outputting a string."` | `row_29_string_overflows_output` | [x] |
| 30 | `cp_inflate` (lib.c:345) | `BTYPE == 3` in the first block, and in a *later* block | `0`, `"Detected unknown block type within input stream."` | `row_30_unknown_block_type` | [x] |
| 31 | `cp_block` (lib.c:245) | `out_bytes == 0` with a stream that emits at least one literal | `0`, `"Attempted to overwrite out buffer while outputting a symbol."` | `row_27_31_literal_overflows_output` | [x] |
| 32 | `cp_inflate` (lib.c:299) | `in_bytes` too small to even read the block header (`in_bytes == 0`, all alignments) | `SIGABRT`, `assert(s->bits_left > 0)` | `row_32_input_too_small` | [x] |

## C. Live `assert()`s — `SIGABRT`

| # | function | trigger | expected C result | test | [x] |
|---|----------|---------|-------------------|------|-----|
| 33 | `cp_ptr` (lib.c:80) | `assert(!(s->bits_left & 7))`. **Analytically derived** (see the doc comment on `row_33_cp_ptr_alignment_assert`): a refill from `s->final_word` adds `bits_left` — not 32 — to `count`, after which `bits_left` at `cp_ptr` is `≡ -c0 (mod 8)` where `c0` is `count` at that refill; feasibility forces `last_bytes = 3`, `c0 = 9`, `word_count = 2`, and the refill must land on the end-of-block `cp_decode`. Concrete 11-byte stream: fixed block with 2 eight-bit + 4 nine-bit literals, then a stored header with `LEN = 0xFFFF` | `SIGABRT`, `assert(!(s->bits_left & 7))` | `row_33_cp_ptr_alignment_assert` | [x] |
| 34 | `cp_peak_bits` (lib.c:89) | `assert(s->word_index <= s->word_count)`. **Unreachable**: the enclosing `if (s->word_index < s->word_count)` guarantees it after the increment. The test sweeps well-formed and truncated streams of every flavour x every alignment and asserts this assertion never fires | `SIGABRT` (unreachable) | `row_34_peak_bits_assert_unreachable` | [x] |
| 35 | `cp_consume_bits` (lib.c:100) | `assert(s->count >= num_bits_to_read)` — a stored-block header needing 32 bits for `LEN`/`NLEN` while fewer are buffered: `[0x23]` (`in=1`), `[0x01,0x00]` (`in=2`, align 0) | `SIGABRT`, `assert(s->count >= num_bits_to_read)` | `row_35_consume_bits_assert` | [x] |
| 36 | `cp_read_bits` (lib.c:110) | `assert(s->bits_left > 0)` — `in_bytes == 0` (all alignments); a fixed block with `BFINAL == 0` so another header read is attempted after the input ends | `SIGABRT`, `assert(s->bits_left > 0)` | `row_36_bits_left_assert` | [x] |
| 37 | `cp_read_bits` (lib.c:112) | `assert(!cp_would_overflow(s, n))`, i.e. `(bits_left + count) - n < 0` — `first_bytes == 1` (align 3) with `in_bytes == 2`: the buffer is empty after `BFINAL`/`BTYPE`/align but 8 real bits remain, so the 16-bit `LEN` read overflows | `SIGABRT`, `assert(!cp_would_overflow(s, num_bits_to_read))` | `row_37_would_overflow_assert` | [x] |
| 38 | `cp_read_bits` (lib.c:108) | `assert(num_bits_to_read <= 32)` — only reachable by writing into the **public writable** `cp_len_extra_bits` / `cp_dist_extra_bits` tables (values 33, 64, 255 / 33, 200), which a real consumer can do because they are exported non-`const` globals. The poke happens inside the fork, and the test asserts the parent's tables are still pristine afterwards | `SIGABRT`, `assert(num_bits_to_read <= 32)` | `row_38_num_bits_range_assert` | [x] |
| 39 | `cp_read_bits` (lib.c:109) | `assert(num_bits_to_read >= 0)`. **Unreachable**: every argument is a literal (`1,2,3,4,5,7,16`), a `uint8_t` table entry, or `s->count & 7`, and `count` can never go negative because `cp_consume_bits` asserts `count >= num` first. The test sweeps the stored-block path (the only caller of `cp_read_bits(s, s->count & 7)`) and asserts it never fires | `SIGABRT` (unreachable) | `row_39_negative_num_bits_unreachable` | [x] |
| 40 | `cp_read_bits` (lib.c:111) | `assert(s->count <= 64)`. **Unreachable**: `count` only grows in `cp_peak_bits`, and only while `count < num_bits_to_read`, which is at most 16 for every call that can refill. The `words[]` branch adds 32 (`count < 48`); the `final_word` branch adds `bits_left = 8*last_bytes + count <= 24 + 15`, so `count <= 54`. Swept | `SIGABRT` (unreachable) | `row_40_count_bound_unreachable` | [x] |
| 41 | `cp_build` (lib.c:139) | `assert(len < 16)` — a code length `>= 16`, only reachable through the public writable `cp_fixed_table` (poked with 16, 17, 20, 31, 40, 47). Note that the earlier `counts[lens[n]]++` overflow for those values lands on `first[]`/`codes[]`, which are both fully recomputed afterwards, so the C behaviour is still well defined here | `SIGABRT`, `assert(len < 16)` | `row_41_code_length_assert` | [x] |
| 42 | `cp_decode` (lib.c:202) | `assert((search >> len) == (key >> len))`, `len = 32 - (key & 0xF)` — the peeked bits match no code in the tree. Two cases: the discovered 5-byte stream `[1C 41 66 8B B0]`, and a dynamic block with `HCLEN = 0` and all four transmitted code lengths `0`, so `cp_build` returns 0, `cp_decode` reads `tree[-1] == s->dst[31] == 0`, `len == 32` (gcc masks the shift to 0) and the assertion degenerates to `search == 0`, which can never hold | `SIGABRT`, `assert((search >> len) == (key >> len))` | `row_42_decode_assert` | [x] |

## D. Generic FFI boundary conditions

| # | entry point | condition | expected C result | test | [x] |
|---|-------------|-----------|-------------------|------|-----|
| 43 | `load_png_mem` | `png_length ∈ 0..=32` on a valid PNG. The signature `memcmp` reads 8 bytes **regardless** of `png_length`, so the signature check passes and `cp_chunk` fails instead. `png_length == 33` (signature + IHDR) is enough for the IHDR but leaves no IDAT in range. `34 .. real_len` are compared with the paired fork driver | `"unable to find IHDR chunk"` (0..32) / `"corrupt zlib structure in DEFLATE stream"` (33) | `row_43_zero_png_length` | [x] |
| 44 | `load_png_mem` | negative `png_length` (`-1, -2, -8, -1024, INT_MIN, INT_MIN+1`) → `png.end < png.p`; and the same lengths with a bad signature (the length is then never looked at) | `"unable to find IHDR chunk"` / `"incorrect file signature …"` | `row_44_negative_png_length` | [x] |
| 45 | `load_png_mem` | `png_length` far larger than the buffer (`INT_MAX`, `INT_MAX-1`, `1<<20`, `1<<28`) with a bad signature | `"incorrect file signature (is this a png file?)"`, nothing past byte 8 is read | `row_45_oversized_png_length` | [x] |
| 46 | `load_png_mem` | a valid PNG with 0/1/3/12/64/1000 trailing garbage bytes, all colour types; output cross-checked against the reference decoder | decodes identically | `row_46_trailing_garbage` | [x] |
| 47 | `cp_inflate` | `out_bytes == 0` and negative (`-1, -8, -4096`) → `out_end < out`, all alignments. An empty stored block emits nothing, so `out_bytes == 0` then succeeds | `"Attempted to overwrite out buffer while outputting a symbol."` / success | `row_47_out_bytes_boundaries` | [x] |
| 48 | `cp_inflate` | negative `in_bytes`: `-1, -4, -1024` abort on `assert(s->bits_left > 0)`. For large magnitudes (`INT_MIN`, `INT_MIN+1`, `INT_MIN+8`, `-2^29`, `-100000`) `in_bytes*8` overflows and `s->final_word` is filled from far *before* the buffer, so the process may die with `SIGSEGV`; the test requires only that both libraries die the same way | `SIGABRT` / identical fatal signal | `row_48_negative_in_bytes` | [x] |
| 49 | `cp_inflate` | every input alignment x every input-tail residue (`first_bytes` 0..3 x `last_bytes` 0..3), with coverage asserted | identical output | `row_49_alignment_matrix` (+ `inflate.rs` rows 1-9) | [x] |
| 50 | `load_png_mem`, `cp_inflate` | **out-of-range enum values crossing the FFI boundary**, one step outside every documented range: bit depth `0,1,2,3,4,5,7,9,15,16,17,32,128,255`; colour type `1,5,7,8,9,127,128,255`; compression/filter/interlace `1,2,3,255`; filter byte `5,6,128,255`; DEFLATE `BTYPE = 3` | identical rejection | `row_50_out_of_range_enums` | [x] |
| 51 | `load_png_mem` | `cp_get_alpha_for_indexed_image` boundaries: `trns == NULL`, `trns_len ∈ {0,1,128,255,256,257,1024}` with indices spanning `0..=255`, asserting `alpha == 255` for every `index >= trns_len` | identical pixels | `row_51_trns_boundaries` | [x] |
| 52 | `load_png_mem` | PLTE shorter than the largest index used (`plte[c*3]` reads past the chunk): lengths `0,1,2,3,4,5,6,30,300,765,766,767,768` | identical pixels (both read the identical padded buffer) | `row_52_short_plte` | [x] |

**Null pointers** are deliberately *not* passed: the C code dereferences
`png_data` (`memcmp(png.p, sig, 8)`) and `in`/`out` unconditionally, so
`load_png_mem(NULL, n)` and `cp_inflate(NULL, …)` segfault in C. That is not a
rejection branch of the library; it is recorded here for completeness.

## Behaviour that no translation can reproduce

Two inputs classes put the C library into undefined behaviour whose outcome
depends on the machine code layout rather than on the algorithm. They are
documented rather than pinned by a test:

1. **`cp_dynamic` smashes its own stack.** `uint8_t lens[288 + 32]` is written at
   indices up to `nlit + ndst - 1 + 137 = 456`. The Rust translation reproduces
   gcc's `-O0` frame layout byte for byte (see the block comment above
   `cp_dynamic` in `src/lib.rs`), which removes the divergence for this gcc
   build — but the *initial* contents of `lens` are uninitialised stack in C and
   zero in Rust, and a different code layout would alias different locals.
   Verified with `-fstack-protector-all`: `*** stack smashing detected ***`.
2. **`load_png_mem` lets `cp_inflate` write past `img.pix`.** `out` is set to
   `img.pix + cp_out_size(4) - cp_out_size(bpp)` but `out_bytes` is the *full*
   `pix_bytes`, so for `bpp < 4` the decoder may write up to
   `(img.w+1)*img.h*(4-bpp)` bytes beyond the allocation, clobbering glibc heap
   metadata. Whether glibc *notices* depends on the surrounding heap, which is
   why every malformed-input test uses `run_forked_pair` (both children forked
   back to back from byte-identical parent state). `cp_stored`'s
   `memcpy(s->out, p, LEN)` is likewise completely unbounded.
