# Error Surface

Mechanically derived from every `cp_error_reason =`, `return 0`, `assert`, and
input/range check in `../c_src/src/lib.c`. `load_png_mem` error results retain
the parsed `w`/`h` only after the allocation point; all errors return `pix ==
NULL`. Assertion rows terminate the process through C `assert`.

| # | function | trigger (exact invalid input/condition) | expected C result | |
|---|----------|-----------------------------------------|-------------------|---|
| E01 | `cp_stored` via `cp_inflate` | stored-block `LEN != (uint16_t)~NLEN` | `cp_inflate == 0`; complement error reason | [x] |
| E02 | `cp_stored` via `cp_inflate` | after the stored header, `bits_left / 8 > LEN` | `cp_inflate == 0`; stored-block-extends error reason | [x] |
| E03 | `cp_block` via `cp_inflate` | literal with `out + 1 > out_end` | `cp_inflate == 0`; symbol-overwrite error reason | [x] |
| E04 | `cp_block` via `cp_inflate` | match with `out - backwards_distance < begin` | `cp_inflate == 0`; invalid-backwards-distance error reason | [x] |
| E05 | `cp_block` via `cp_inflate` | match with `out + length > out_end` | `cp_inflate == 0`; string-overwrite error reason | [x] |
| E06 | `cp_inflate` | DEFLATE `BTYPE == 3` | `0`; unknown-block-type error reason | [x] |
| E07 | `load_png_mem` | first 8 bytes differ from PNG signature | `{w:0,h:0,pix:NULL}`; signature error reason | [x] |
| E08 | `load_png_mem` | next bounded chunk is not `IHDR` with length at least 13 | zero image; IHDR error reason | [x] |
| E09 | `load_png_mem` | `IHDR.bit_depth != 8` | zero image; bit-depth error reason | [x] |
| E10 | `load_png_mem` | `IHDR.color_type` not in `{0,2,3,4,6}` | zero image; color-type error reason | [x] |
| E11 | `load_png_mem` | `(int)width_be + 1 < 1` (notably `width_be == UINT32_MAX`) | zero image; width error reason | [x] |
| E12 | `load_png_mem` | `(int)height_be < 1` | zero image; height error reason | [x] |
| E13 | `load_png_mem` | `(int64_t)(width + 1) * height * 4 >= INT_MAX` | zero image; image-too-large error reason | [x] |
| E14 | `load_png_mem` | `malloc(pix_bytes) == NULL` | parsed dimensions, `pix:NULL`; allocation error reason | [x] |
| E15 | `load_png_mem` | `IHDR.compression != 0` | parsed dimensions, `pix:NULL`; compression error reason | [x] |
| E16 | `load_png_mem` | `IHDR.filter != 0` | parsed dimensions, `pix:NULL`; adaptive-filter error reason | [x] |
| E17 | `load_png_mem` | `IHDR.interlace != 0` | parsed dimensions, `pix:NULL`; interlace error reason | [x] |
| E18 | `load_png_mem` | no collected IDAT allocation or total IDAT length `< 6` | parsed dimensions, `pix:NULL`; corrupt-zlib error reason | [x] |
| E19 | `load_png_mem` | zlib `CM != 8`, i.e. `(data[0] & 0x0f) != 8` | parsed dimensions, `pix:NULL`; compression-method error reason | [x] |
| E20 | `load_png_mem` | zlib `CINFO > 7`, i.e. `(data[0] & 0xf0) > 0x70` | parsed dimensions, `pix:NULL`; window-size error reason | [x] |
| E21 | `load_png_mem` | zlib `FDICT != 0`, i.e. `(data[1] & 0x20) != 0` | parsed dimensions, `pix:NULL`; dictionary error reason | [x] |
| E22 | `load_png_mem` | `(img.w + 1) * img.h * 4 < 1` after prior size guards | parsed dimensions, `pix:NULL`; invalid-size error reason (internal invariant branch) | [x] |
| E23 | `load_png_mem` | `(img.w + 1) * img.h * bpp < 1` after prior size guards | parsed dimensions, `pix:NULL`; invalid-size error reason (internal invariant branch) | [x] |
| E24 | `load_png_mem` | `cp_inflate(data + 2, datalen - 6, out, pix_bytes) == 0` | parsed dimensions, `pix:NULL`; DEFLATE-failed error reason | [x] |
| E25 | `cp_unfilter` via `load_png_mem` | first row filter byte not in `0..=4` | parsed dimensions, `pix:NULL`; invalid-filter error reason | [x] |
| E26 | `cp_unfilter` via `load_png_mem` | later row filter byte not in `0..=4` | parsed dimensions, `pix:NULL`; invalid-filter error reason | [x] |
| E27 | `load_png_mem` | indexed color (`color_type == 3`) with no found `PLTE` | parsed dimensions, `pix:NULL`; indexed-palette error reason | [x] |
| E28 | `cp_ptr` via `cp_inflate` | stored-block pointer requested with `bits_left & 7 != 0` | assertion termination (internal alignment invariant) | [x] |
| E29 | `cp_peak_bits` via `cp_inflate` | a word load makes `word_index > word_count` | assertion termination (internal index invariant) | [x] |
| E30 | `cp_consume_bits` via `cp_inflate` | `count < num_bits_to_read` | assertion termination | [x] |
| E31 | `cp_read_bits` via `cp_inflate` | internal request `num_bits_to_read > 32` | assertion termination (table/constant invariant) | [x] |
| E32 | `cp_read_bits` via `cp_inflate` | internal request `num_bits_to_read < 0` | assertion termination (table/constant invariant) | [x] |
| E33 | `cp_read_bits` via `cp_inflate` | `bits_left <= 0` | assertion termination on truncated stream | [x] |
| E34 | `cp_read_bits` via `cp_inflate` | `count > 64` | assertion termination (bit-buffer invariant) | [x] |
| E35 | `cp_read_bits` via `cp_inflate` | `(bits_left + count) - num_bits_to_read < 0` | assertion termination on truncated stream | [x] |
| E36 | `cp_build` via `cp_inflate` | nonzero code length `>= 16` | assertion termination (fixed/3-bit dynamic-length invariant) | [x] |
| E37 | `cp_decode` via `cp_inflate` | decoded tree key does not prefix-match `search` | assertion termination on malformed Huffman stream | [x] |
| E38 | `load_png_mem` | `png_data == NULL` | process fault while reading signature (C API has no null guard) | [x] |
| E39 | `load_png_mem` | `png_length` is `0`, `1..7`, negative, or larger than the accessible object | C still reads 8 signature bytes; invalid accessible bytes reject as E07, inaccessible memory is undefined/faulting | [x] |
| E40 | `cp_inflate` | `in == NULL`, `out == NULL`, or lengths exceed accessible objects | no null/range guard; input/output use is undefined or faulting | [x] |
| E41 | `cp_inflate` | `in_bytes <= 0` | assertion termination or undefined bit-buffer access before a return code | [x] |
| E42 | `cp_inflate` | `out_bytes == 0` with nonempty literal output | same as E03 | [x] |
| E43 | `cp_inflate` | `out_bytes < 0` | pointer arithmetic leaves the declared output range outside the object; undefined unless an earlier explicit rejection fires | [x] |

There are no public enum parameters. Therefore there is no out-of-range FFI
enum case beyond the integer-coded PNG fields covered by E09, E10, E15-E17,
E19-E21, and E25-E26.

All rows are covered by `tests/differential.rs`. Faulting/aborting and allocator
cases run in isolated child processes; internal invariant-only assertions are
also checked one-for-one against the C assertion surface.
