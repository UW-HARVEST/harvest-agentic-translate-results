# Error Surface

Rows E01-E25 are explicit error/rejection branches. Rows A01-A09 are C
assertions guarding internal DEFLATE invariants. Rows G01-G06 are generic FFI
boundary cases required by the verification protocol.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| E01 | `cp_stored` via `cp_inflate` | stored-block `LEN != (uint16_t)~NLEN` | `cp_inflate` returns `0`; reason is `Failed to find LEN and NLEN as complements within stored (uncompressed) stream.` | [x] |
| E02 | `cp_stored` via `cp_inflate` | after reading LEN/NLEN, `bits_left / 8 > LEN` | `cp_inflate` returns `0`; reason is `Stored block extends beyond end of input stream.` | [x] |
| E03 | `cp_block` via `cp_inflate` | literal symbol with `out + 1 > out_end` | returns `0`; reason is `Attempted to overwrite out buffer while outputting a symbol.` | [x] |
| E04 | `cp_block` via `cp_inflate` | length/distance pair with `out - backwards_distance < begin` | returns `0`; reason is `Attempted to write before out buffer (invalid backwards distance).` | [x] |
| E05 | `cp_block` via `cp_inflate` | length/distance pair with `out + length > out_end` | returns `0`; reason is `Attempted to overwrite out buffer while outputting a string.` | [x] |
| E06 | `cp_inflate` | DEFLATE block type is `3` | returns `0`; reason is `Detected unknown block type within input stream.` | [x] |
| E07 | `cp_unfilter` via `load_png_mem` | first scanline filter byte is outside `0..=4` | image has null `pix`; reason is `invalid filter byte found` | [x] |
| E08 | `cp_unfilter` via `load_png_mem` | later scanline filter byte is outside `0..=4` | image has null `pix`; reason is `invalid filter byte found` | [x] |
| E09 | `load_png_mem` | first 8 bytes differ from PNG signature | zero image; reason is `incorrect file signature (is this a png file?)` | [x] |
| E10 | `load_png_mem`/`cp_chunk` | next chunk is not `IHDR`, IHDR length is below 13, or chunk extends beyond `png.end` | zero image; reason is `unable to find IHDR chunk` | [x] |
| E11 | `load_png_mem` | IHDR bit depth is not exactly `8` | zero image; reason is `only bit-depth of 8 is supported` | [x] |
| E12 | `load_png_mem` | IHDR color type is not one of `0,2,3,4,6` | zero image; reason is `unknown color type` | [x] |
| E13 | `load_png_mem` | big-endian IHDR width plus one, converted to `int`, is less than 1 | zero image; reason is `invalid IHDR chunk found, image width was less than 1` | [x] |
| E14 | `load_png_mem` | big-endian IHDR height converted to `int` is less than 1 | zero image; reason is `invalid IHDR chunk found, image height was less than 1` | [x] |
| E15 | `load_png_mem` | `(int64_t)(width + 1) * height * 4 >= INT_MAX` | zero image; reason is `image too large` | [x] |
| E16 | `load_png_mem` | `malloc(pix_bytes)` returns null | image retains parsed `w/h`, has null `pix`; reason is `unable to allocate raw image space` | [x] |
| E17 | `load_png_mem` | IHDR compression method is nonzero | image retains parsed `w/h`, has null `pix`; reason is `only standard compression DEFLATE is supported` | [x] |
| E18 | `load_png_mem` | IHDR filter method is nonzero | image retains parsed `w/h`, has null `pix`; reason is `only standard adaptive filtering is supported` | [x] |
| E19 | `load_png_mem` | IHDR interlace method is nonzero | image retains parsed `w/h`, has null `pix`; reason is `interlacing is not supported` | [x] |
| E20 | `load_png_mem` | concatenated IDAT allocation is null or total IDAT length is below 6 | image retains parsed `w/h`, has null `pix`; reason is `corrupt zlib structure in DEFLATE stream` | [x] |
| E21 | `load_png_mem` | zlib CM nibble `(data[0] & 0x0f) != 8` | image retains parsed `w/h`, has null `pix`; reason is `only zlib compression method (RFC 1950) is supported` | [x] |
| E22 | `load_png_mem` | zlib CINFO nibble `(data[0] & 0xf0) > 0x70` | image retains parsed `w/h`, has null `pix`; reason is `innapropriate window size detected` | [x] |
| E23 | `load_png_mem` | zlib FDICT bit `(data[1] & 0x20) != 0` | image retains parsed `w/h`, has null `pix`; reason is `preset dictionary is present and not supported` | [x] |
| E24 | `load_png_mem` | `cp_out_size(&img, 4) < 1` or `cp_out_size(&img, bpp) < 1` | image has null `pix`; reason is `invalid image size found` (unreachable after E13-E15 for defined positive dimensions) | [x] |
| E25 | `load_png_mem` | `cp_inflate(...) == 0` | image has null `pix`; reason is overwritten with `DEFLATE algorithm failed` | [x] |
| E26 | `load_png_mem` | indexed color type `3` has no discoverable `PLTE` chunk | image has null `pix`; reason is `color type of indexed requires a PLTE chunk` | [x] |
| A01 | `cp_ptr` | `bits_left & 7 != 0` when stored-block payload pointer is requested | process aborts at C assertion | [x] |
| A02 | `cp_peak_bits` | loading a full word increments `word_index` above `word_count` | process aborts at C assertion (internal invariant) | [x] |
| A03 | `cp_consume_bits` | `count < num_bits_to_read` | process aborts at C assertion | [x] |
| A04 | `cp_read_bits` | requested bit count is greater than 32 | process aborts at C assertion (internal call-site invariant) | [x] |
| A05 | `cp_read_bits` | requested bit count is negative | process aborts at C assertion (internal call-site invariant) | [x] |
| A06 | `cp_read_bits` | `bits_left <= 0` before a read | process aborts at C assertion | [x] |
| A07 | `cp_read_bits` | buffered `count > 64` | process aborts at C assertion (internal invariant) | [x] |
| A08 | `cp_read_bits` | `(bits_left + count) - requested_bits < 0` | process aborts at C assertion | [x] |
| A09 | `cp_build` | a supplied Huffman code length is at least 16 | process aborts at C assertion | [x] |
| A10 | `cp_decode` | decoded prefix does not equal the selected Huffman tree key prefix | process aborts at C assertion | [x] |
| G01 | `load_png_mem` | `png_data == NULL` | process receives the C library's fault signal | [x] |
| G02 | `load_png_mem` | `png_length == 0` with an addressable 8-byte buffer | signature rejection E09 | [x] |
| G03 | `load_png_mem` | `png_length < 0` with an addressable 8-byte buffer | signature rejection E09 unless the bytes contain the signature | [x] |
| G04 | `cp_inflate` | `in == NULL` | process receives the C library's fault/assert signal | [x] |
| G05 | `cp_inflate` | `in_bytes == 0` with an addressable buffer | process aborts at `bits_left > 0` assertion | [x] |
| G06 | public API | out-of-range enum value | not applicable: neither exported function accepts a C enum | [x] |
| G07 | `load_png_mem` | `png_length == INT_MAX` with an addressable non-signature buffer | signature rejection E09 | [x] |
| G08 | `cp_inflate` | `in_bytes == INT_MAX` with an addressable buffer | process aborts after C `int` arithmetic makes `bits_left` nonpositive | [x] |
