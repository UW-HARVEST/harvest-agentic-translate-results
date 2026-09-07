# Error-surface table

Rows are derived from every explicit rejection and `assert` in
`c_src/src/lib.c`. Assertions that are internal invariants and cannot be
independently selected through an exported API are identified as such rather
than silently omitted.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 [x] | `cp_ptr` via `cp_inflate` | `s->bits_left & 7` is nonzero when a stored block asks for its byte pointer | assertion abort (internal alignment invariant) |
| 2 [x] | `cp_peak_bits` via `cp_inflate` | loading a full word makes `word_index > word_count` | assertion abort (internal counter invariant) |
| 3 [x] | `cp_consume_bits` via `cp_inflate` | `s->count < num_bits_to_read` | assertion abort |
| 4 [x] | `cp_read_bits` via `cp_inflate` | `num_bits_to_read > 32` | assertion abort |
| 5 [x] | `cp_read_bits` via `cp_inflate` | `num_bits_to_read < 0` | assertion abort (not constructible: callers use nonnegative constants and `uint8_t` tables) |
| 6 [x] | `cp_read_bits` via `cp_inflate` | `s->bits_left <= 0` | assertion abort |
| 7 [x] | `cp_read_bits` via `cp_inflate` | `s->count > 64` | assertion abort (internal accumulator invariant) |
| 8 [x] | `cp_read_bits` via `cp_inflate` | `(s->bits_left + s->count) - num_bits_to_read < 0` | assertion abort |
| 9 [x] | `cp_build` via `cp_inflate` | a Huffman code length is `>= 16` | assertion abort |
| 10 [x] | `cp_decode` via `cp_inflate` | decoded search prefix does not equal the selected tree key prefix | assertion abort |
| 11 [x] | `cp_stored` via `cp_inflate` | stored-block `LEN != (uint16_t)~NLEN` | returns `0`; reason `Failed to find LEN and NLEN as complements within stored (uncompressed) stream.` |
| 12 [x] | `cp_stored` via `cp_inflate` | after LEN/NLEN, `s->bits_left / 8 > LEN` | returns `0`; reason `Stored block extends beyond end of input stream.` |
| 13 [x] | `cp_block` via `cp_inflate` | literal output requires `s->out + 1 > s->out_end` | returns `0`; reason `Attempted to overwrite out buffer while outputting a symbol.` |
| 14 [x] | `cp_block` via `cp_inflate` | length/distance copy has `s->out - backwards_distance < s->begin` | returns `0`; reason `Attempted to write before out buffer (invalid backwards distance).` |
| 15 [x] | `cp_block` via `cp_inflate` | length/distance copy has `s->out + length > s->out_end` | returns `0`; reason `Attempted to overwrite out buffer while outputting a string.` |
| 16 [x] | `cp_inflate` | DEFLATE block type is reserved value `3` | returns `0`; reason `Detected unknown block type within input stream.` |
| 17 [x] | `cp_unfilter` via `load_png_mem` | first scanline filter byte is not `0..=4` | image has `pix == NULL`; reason `invalid filter byte found` |
| 18 [x] | `cp_unfilter` via `load_png_mem` | a later scanline filter byte is not `0..=4` | image has `pix == NULL`; reason `invalid filter byte found` |
| 19 [x] | `load_png_mem` | first eight bytes differ from `89 50 4e 47 0d 0a 1a 0a` | image has `pix == NULL`; reason `incorrect file signature (is this a png file?)` |
| 20 [x] | `load_png_mem` | first post-signature chunk is not a complete `IHDR` with length at least 13 | image has `pix == NULL`; reason `unable to find IHDR chunk` |
| 21 [x] | `load_png_mem` | IHDR bit depth is not `8` | image has `pix == NULL`; reason `only bit-depth of 8 is supported` |
| 22 [x] | `load_png_mem` | IHDR color type is not one of `0,2,3,4,6` | image has `pix == NULL`; reason `unknown color type` |
| 23 [x] | `load_png_mem` | `cp_make32(IHDR.width) + 1` wraps/converts to a value `< 1` | image has `pix == NULL`; reason `invalid IHDR chunk found, image width was less than 1` |
| 24 [x] | `load_png_mem` | IHDR height is `< 1` | image has `pix == NULL`; reason `invalid IHDR chunk found, image height was less than 1` |
| 25 [x] | `load_png_mem` | `(int64_t)(width + 1) * height * sizeof(cp_pixel_t) >= INT_MAX` | image has `pix == NULL`; reason `image too large` |
| 26 [x] | `load_png_mem` | `malloc(pix_bytes)` returns `NULL` | image has `pix == NULL`; reason `unable to allocate raw image space` |
| 27 [x] | `load_png_mem` | IHDR compression method is nonzero | image has `pix == NULL`; reason `only standard compression DEFLATE is supported` |
| 28 [x] | `load_png_mem` | IHDR filter method is nonzero | image has `pix == NULL`; reason `only standard adaptive filtering is supported` |
| 29 [x] | `load_png_mem` | IHDR interlace method is nonzero | image has `pix == NULL`; reason `interlacing is not supported` |
| 30 [x] | `load_png_mem` | concatenated IDAT data is absent, allocation fails, or total length is `< 6` | image has `pix == NULL`; reason `corrupt zlib structure in DEFLATE stream` |
| 31 [x] | `load_png_mem` | zlib CM nibble `(data[0] & 0x0f) != 8` | image has `pix == NULL`; reason `only zlib compression method (RFC 1950) is supported` |
| 32 [x] | `load_png_mem` | zlib CINFO nibble `(data[0] & 0xf0) > 0x70` | image has `pix == NULL`; reason `innapropriate window size detected` |
| 33 [x] | `load_png_mem` | zlib FLG has preset-dictionary bit `data[1] & 0x20` | image has `pix == NULL`; reason `preset dictionary is present and not supported` |
| 34 [x] | `load_png_mem` | `cp_out_size(&img, 4) < 1` after prior size checks | image has `pix == NULL`; reason `invalid image size found` (unreachable arithmetic guard for accepted dimensions) |
| 35 [x] | `load_png_mem` | `cp_out_size(&img, bpp) < 1` after prior size checks | image has `pix == NULL`; reason `invalid image size found` (unreachable arithmetic guard for accepted dimensions/bpp) |
| 36 [x] | `load_png_mem` | raw DEFLATE payload makes `cp_inflate` return `0` | image has `pix == NULL`; reason `DEFLATE algorithm failed` |
| 37 [x] | `load_png_mem` | indexed color type `3` has no prior complete `PLTE` chunk | image has `pix == NULL`; reason `color type of indexed requires a PLTE chunk` |
| 38 [x] | `cp_inflate` | `in == NULL` with a path that dereferences input | process faults; same external rejection required |
| 39 [x] | `cp_inflate` | `out == NULL` with nonempty decoded output | process faults or output-bound rejection according to pointer arithmetic; same result required |
| 40 [x] | `cp_inflate` | zero, negative, or oversized `in_bytes`/`out_bytes` crossing the FFI boundary | same return/signal and error pointer as C |
| 41 [x] | `load_png_mem` | `png_data == NULL`, or zero/negative/oversized `png_length`, before the unconditional 8-byte signature read | same return/signal and error pointer as C |
