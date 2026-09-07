# ERRORS.md — error-surface table

Mechanically derived from `c_src/src/lib.c`.  Every `cp_error_reason = ...;
goto cp_err;` branch, every bare `return 0;`, every `assert(...)`, and every
explicit range / null / min-max check gets one row.

`load_png_mem` never "returns an error code": on every failure it sets
`cp_error_reason`, `free()`s what it allocated and returns
`cp_image_t { w, h, pix = NULL }`.  Note that `w`/`h` are **already filled in**
by the time the checks after the `img.w = w - 1; img.h = h;` assignment run, so
the returned `w`/`h` are part of the observable result and are compared too.

`cp_inflate` returns `int` — `1` on success, `0` on failure.

## `load_png_mem`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `load_png_mem` | first 8 bytes != `\211PNG\r\n\032\n` (`memcmp` != 0) | `pix=NULL, w=0, h=0`; reason `"incorrect file signature (is this a png file?)"` | [x] |
| 2 | `load_png_mem` | `cp_chunk(&png,"IHDR",13)` returns 0: chunk-type bytes at `p+4` != `"IHDR"`, **or** declared `len < 13`, **or** `p + len + 12 > end` (truncated) | `pix=NULL, w=0, h=0`; `"unable to find IHDR chunk"` | [x] |
| 3 | `load_png_mem` | `ihdr[8]` (bit depth) != 8 — i.e. 0,1,2,4,16 and every other byte value | `pix=NULL, w=0, h=0`; `"only bit-depth of 8 is supported"` | [x] |
| 4 | `load_png_mem` | `ihdr[9]` (color type) not in {0,2,3,4,6} — `switch` default | `pix=NULL, w=0, h=0`; `"unknown color type"` | [x] |
| 5 | `load_png_mem` | `w = cp_make32(ihdr) + 1 < 1` — i.e. raw width `0xFFFFFFFF` (→ `w=0`) or raw width with bit 31 set (→ `w` negative) | `pix=NULL, w=0, h=0`; `"invalid IHDR chunk found, image width was less than 1"` | [x] |
| 6 | `load_png_mem` | `h = cp_make32(ihdr+4) < 1` — raw height 0, or raw height >= 0x80000000 (negative `int`) | `pix=NULL, w=0, h=0`; `"invalid IHDR chunk found, image height was less than 1"` | [x] |
| 7 | `load_png_mem` | `(int64_t)w * h * 4 >= INT_MAX` (i.e. `w*h >= 0x20000000`) | `pix=NULL, w=0, h=0`; `"image too large"` | [x] |
| 8 | `load_png_mem` | `malloc(pix_bytes)` returns NULL | `pix=NULL, w=w-1, h=h`; `"unable to allocate raw image space"` | [x] unreachable in practice — row 7 caps `pix_bytes` at < 2 GiB; covered by a `w`/`h` pair that is just under the row-7 limit so the allocation is attempted |
| 9 | `load_png_mem` | `ihdr[10]` (compression method) != 0 | `pix=NULL, w=w-1, h=h`; `"only standard compression DEFLATE is supported"` | [x] |
| 10 | `load_png_mem` | `ihdr[11]` (filter method) != 0 | `pix=NULL, w=w-1, h=h`; `"only standard adaptive filtering is supported"` | [x] |
| 11 | `load_png_mem` | `ihdr[12]` (interlace) != 0 | `pix=NULL, w=w-1, h=h`; `"interlacing is not supported"` | [x] |
| 12 | `load_png_mem` | `data == NULL` (i.e. `malloc(datalen)` failed because `datalen` wrapped negative) **or** `datalen < 6` — includes *no IDAT chunk at all* (`datalen == 0`) and IDATs totalling 1..5 bytes | `pix=NULL, w=w-1, h=h`; `"corrupt zlib structure in DEFLATE stream"` | [x] |
| 13 | `load_png_mem` | `(data[0] & 0x0f) != 0x08` — zlib CM nibble not 8 | `pix=NULL, w=w-1, h=h`; `"only zlib compression method (RFC 1950) is supported"` | [x] |
| 14 | `load_png_mem` | `(data[0] & 0xf0) > 0x70` — CINFO nibble 8..15 | `pix=NULL, w=w-1, h=h`; `"innapropriate window size detected"` (misspelling is in the C) | [x] |
| 15 | `load_png_mem` | `data[1] & 0x20` set — FDICT | `pix=NULL, w=w-1, h=h`; `"preset dictionary is present and not supported"` | [x] |
| 16 | `load_png_mem` | `cp_out_size(&img, 4) < 1` | `pix=NULL, w=w-1, h=h`; `"invalid image size found"` | [x] unreachable: row 7 guarantees `(w)*h*4` is a positive `int`; asserted unreachable by testing the closest reachable inputs |
| 17 | `load_png_mem` | `cp_out_size(&img, bpp) < 1` | `pix=NULL, w=w-1, h=h`; `"invalid image size found"` | [x] unreachable, same reason (`bpp <= 4`) |
| 18 | `load_png_mem` | `cp_inflate(...)` returned 0 (any of rows 21..26) | `pix=NULL, w=w-1, h=h`; `"DEFLATE algorithm failed"` | [x] |
| 19 | `load_png_mem` | `cp_unfilter(...)` returned 0 (row 27 or 28: a filter byte > 4) | `pix=NULL, w=w-1, h=h`; `"invalid filter byte found"` | [x] |
| 20 | `load_png_mem` | `color_type == 3` but no `PLTE` chunk was found | `pix=NULL, w=w-1, h=h`; `"color type of indexed requires a PLTE chunk"` | [x] |

## `cp_inflate` and its helpers

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 21 | `cp_stored` | `LEN != (uint16_t)~NLEN` | `cp_stored` -> 0, `cp_inflate` -> **0**; `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | [x] |
| 22 | `cp_stored` | `s->bits_left / 8 > (int)LEN` — a stored block that is not the last data in the stream (also fires when `LEN == 0` and >0 bytes remain) | `cp_inflate` -> **0**; `"Stored block extends beyond end of input stream."` | [x] |
| 23 | `cp_block` | literal symbol < 256 decoded while `s->out + 1 > s->out_end` | `cp_inflate` -> **0**; `"Attempted to overwrite out buffer while outputting a symbol."` | [x] |
| 24 | `cp_block` | back-reference with `s->out - backwards_distance < s->begin` | `cp_inflate` -> **0**; `"Attempted to write before out buffer (invalid backwards distance)."` | [x] |
| 25 | `cp_block` | back-reference with `s->out + length > s->out_end` | `cp_inflate` -> **0**; `"Attempted to overwrite out buffer while outputting a string."` | [x] |
| 26 | `cp_inflate` | `btype == 3` (both type bits set) | `cp_inflate` -> **0**; `"Detected unknown block type within input stream."` | [x] |
| 27 | `cp_unfilter` | row 0 filter byte `> 4` (`switch` default) | `cp_unfilter` -> 0 (surfaces as row 19) | [x] |
| 28 | `cp_unfilter` | row `y >= 1` filter byte `> 4` | `cp_unfilter` -> 0 (surfaces as row 19) | [x] |

## Silent-acceptance rows (no error, but a distinguished branch)

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 29 | `cp_get_alpha_for_indexed_image` | `trns == NULL` | returns `255` | [x] |
| 30 | `cp_get_alpha_for_indexed_image` | `(uint32_t)index >= trns_len` (palette index past the `tRNS` chunk) | returns `255` | [x] |
| 31 | `cp_chunk` | `memcmp` matches and `len >= minlen` but `p + (int)(len+12) > end` | returns `NULL` (ends the IDAT-gathering loop) | [x] |
| 32 | `cp_find` | walks to `png->p >= png->end` without a match | returns `NULL` | [x] |
| 33 | `cp_convert` | `bpp` not in {1,2,3,4} (`switch` with no default → `dst` not advanced) | unreachable from `load_png_mem` (bpp derived from the validated colour type) | [x] |
| 34 | `cp_inflate` | `btype` 0/1/2 loop repeats until `bfinal` — a stream whose last block has `BFINAL=0` keeps reading past the input | reads past the input buffer; **not testable deterministically** (see below) | n/a |

## `assert()` rows — reference build has asserts **ENABLED**

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no `-DNDEBUG`, so the
reference `.so` aborts (`SIGABRT` + a message on stderr) instead of returning
when one of these fires.  The Rust translation deliberately treats `assert()` as
a no-op (documented in `src/lib.rs` and `README.md`), i.e. it matches a C build
with `NDEBUG`.

The differential harness therefore runs **every** call in a forked child and
classifies the outcome.  When the reference C child dies with `SIGABRT` the case
is re-run against a second C build (`gcc -O0 -DNDEBUG`, which has the identical
`.data` layout) and the Rust result must match *that*.  Rows:

| # | function | assert | reachable with malformed input? |
|---|----------|--------|--------------------------------|
| A1 | `cp_ptr` | `!(s->bits_left & 7)` | no — `cp_stored` byte-aligns first and `count ≡ bits_left (mod 8)` is invariant |
| A2 | `cp_peak_bits` | `s->word_index <= s->word_count` | no — guarded by the enclosing `if` |
| A3 | `cp_consume_bits` | `s->count >= num_bits_to_read` | **yes** — truncated stream / garbage Huffman key |
| A4 | `cp_read_bits` | `num_bits_to_read <= 32` | no — all call sites pass <= 16 |
| A5 | `cp_read_bits` | `num_bits_to_read >= 0` | no — `count & 7` and the literal counts are >= 0; `cp_len_extra_bits[]` is `uint8_t` |
| A6 | `cp_read_bits` | `s->bits_left > 0` | **yes** — exhausted stream |
| A7 | `cp_read_bits` | `s->count <= 64` | no |
| A8 | `cp_read_bits` | `!cp_would_overflow(s, n)` | **yes** — exhausted stream |
| A9 | `cp_build` | `len < 16` | **yes** — `cp_dynamic` writes `(uint8_t)sym` from a corrupt tree, `sym` up to 4095 |
| A10 | `cp_decode` | `(search >> len) == (key >> len)` | **yes** — corrupt / empty tree, `tree[-1]` |

## Generic-boundary rows (not distinct C branches, tested anyway)

| # | call | input |
|---|------|-------|
| B1 | `load_png_mem(NULL, 0)` | null pointer — the C dereferences it in `memcmp`; both libraries must fault identically (checked in a forked child) |
| B2 | `load_png_mem(buf, 0)` / `(buf, 1..7)` | length shorter than the 8-byte signature (`memcmp` reads past the buffer) |
| B3 | `load_png_mem(buf, -1)` | negative length → `png.end < png.p`, `cp_find` loop never runs |
| B4 | `load_png_mem(valid_png, INT_MAX)` | length far past the buffer |
| B5 | `cp_inflate(NULL, 0, out, n)` | null input, zero length |
| B6 | `cp_inflate(in, n, NULL, 0)` | null output, zero output size |
| B7 | `cp_inflate(in, -1, out, n)` / `(in, n, out, -1)` | negative sizes |
| B8 | `cp_inflate(in, n, out, 0)` | zero-size output buffer (row 23 fires immediately) |
| B9 | colour type = every `uint8` value 0..255 | "out-of-range enum" across the FFI boundary — C `switch` accepts any `int`; only 0/2/3/4/6 are valid |
| B10 | bit depth = every `uint8` value 0..255 | ditto |
| B11 | filter byte = every `uint8` value 0..255 on row 0 and on row 1 | ditto (`cp_unfilter`'s `switch`) |
| B12 | `btype` = 0,1,2,3 | ditto (`cp_inflate`'s `switch`) |
| B13 | compression / filter-method / interlace = every `uint8` value | ditto |

Row 34 note: a stream whose final block has `BFINAL == 0` makes `cp_inflate`
loop and read past the end of `in`.  Both libraries then read unrelated heap
bytes, which differ between two runs of the *same* library, so this case is
excluded from byte-comparison (it is detected by running each library twice).

## Result

`cargo test --test phase_c_errors` — 29 tests, all passing.  Rows 1..32 each have
a dedicated test that *pins the exact `cp_error_reason` string* the C produces
(not merely "both failed"), and additionally compares the returned `w`, `h`,
`pix == NULL` and, when a buffer survives, every pixel byte:

| test | rows covered |
|---|---|
| `row01_bad_signature` .. `row26_unknown_block_type` | 1..26 (one test per row; rows 9/10/11 and 16/17 and 19/27/28 share a test because they share a trigger family) |
| `row29_30_indexed_alpha_defaults` | 29, 30 |
| `row31_32_chunk_walk_termination` | 31, 32 (2268 truncation offsets through the forked runner) |
| `row48_out_of_range_table_reads` | out-of-range `cp_len_*` / `cp_dist_*` reads |
| `boundaries_b1_b13_via_forked_runner` | B1..B13, 1933 records |
| `fuzz_mutated_pngs` | 1200 mutated PNGs |
| `fuzz_random_deflate_streams` | 2000 corrupt DEFLATE streams |

Rows 16 and 17 are *proved unreachable* rather than triggered:
`cp_out_size(&img, bpp) == w * h * bpp` and row 7 already guarantees
`1 <= w*h*bpp <= w*h*4 < INT_MAX`.  `row16_17_invalid_image_size_is_unreachable`
sweeps the extreme accepted shapes for all five colour types and asserts the
message never appears.  Row 8 is exercised by attempting a ~2 GiB `malloc` at the
largest size row 7 accepts.

Corpus-level classification (from the test output):

```
corpus boundaries:    1933 records | 1895 identical |  36 assert-only |  2 heap-dependent |  0 no-oracle
corpus truncations:   1134 records | 1062 identical |  35 assert-only | 37 heap-dependent |  0 no-oracle
corpus oob_tables:      68 records |   32 identical |  36 assert-only |  0 heap-dependent |  0 no-oracle
corpus fuzz_png:      1200 records | 1192 identical |   4 assert-only |  4 heap-dependent |  0 no-oracle
corpus fuzz_inflate:  2000 records | 1156 identical | 780 assert-only |  0 heap-dependent | 64 no-oracle
```

* **identical** — the reference C `.so` and the Rust `.so` produced byte-identical
  reports (exit status / signal, `w`, `h`, `pix == NULL`, a hash of the pixel
  buffer, and `cp_error_reason`).
* **assert-only** — the reference build aborted in an `assert()` (rows A3, A6, A8,
  A9, A10); the Rust result matched the `-O0 -DNDEBUG` C build byte for byte.
* **heap-dependent** — the report changes when the runner's freed-heap pattern
  changes, i.e. the C hashes the part of `img.pix` a short DEFLATE stream never
  filled.  Not comparable across two processes; detected by running each library
  twice with two different heap patterns (`prime_heap` in
  `examples/diffrunner.rs`).
* **no-oracle** — the C's own `assert()` is the only thing that stops the input;
  with `NDEBUG` the C loops forever (a corrupt tree can decode a zero-bit,
  zero-length match, so `while (length--)` copies nothing and consumes no bits).
  The runner caps each child at one second, and these records are reported rather
  than compared.  Note the harness *does* fail the test if the **Rust** loops
  where the C terminates.

### Two real divergences were found and are fixed

1. **`.data` layout of the exported tables.**  `src/lib.rs` modelled the `-O1+`
   table order; the reference build (`-O0`) uses source order.  Every
   out-of-range `cp_len_extra_bits` / `cp_len_base` / `cp_dist_*` read therefore
   returned the wrong byte.  Fixed in `blob_byte()`; guarded by
   `phase_d_symbols::c_data_layout_is_source_order` and
   `phase_c_errors::row48_out_of_range_table_reads`.
2. **The profile under test.**  A *debug* build of the crate turns the C's NULL
   dereference in `cp_inflate(NULL, 8, out, 1)` into a Rust panic/abort
   (`core`'s debug assertions in `ptr::read_unaligned`), where the C takes a
   SIGSEGV.  The release build — the artifact the crate actually declares, with
   `panic = "abort"` — segfaults identically, and is what the harness now loads
   (`rust_lib()` in `tests/common/mod.rs`).
