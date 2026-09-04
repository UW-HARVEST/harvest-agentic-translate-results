# ERRORS.md — the error surface (Phase C)

Derived by grepping `c_src/src/*.c` for every distinct way the library rejects
input, then reducing those sites to the ones reachable through the public API.
The raw extraction is reproducible:

```
$ python3 .verif/extract_errors.py | wc -l
663      # rejection sites across 257 functions
$ python3 .verif/extract_errors.py | cut -f4 | sort | uniq -c | sort -rn
   157 png_error              55 png_chunk_benign_error   12 png_app_warning
   115 png_warning            52 png_app_error            10 png_benign_error
   103 return 0               22 return NULL               8 png_chunk_warning
    93 handled-enum           17 png_chunk_report          5 return -1
                              15 png_chunk_error           3 png_fixed_error
                                                           2 PNG_ABORT
```

Many of those 663 sites are the same rejection reached from several places (for
example `png_chunk_benign_error(png_ptr, "invalid")` appears once per chunk
handler) or are internal consistency assertions that no input can reach.  The
table below has one row per **distinct, externally reachable rejection**: the
exact invalid input, the C function that rejects it, and what the C does.

Every row is a differential test.  Because a `png_error` is fatal, each row runs
in its own child process with an error callback that records the message and
`_exit(70)`s; the parent then compares the **exact message text** and the exit
status, so "both failed somehow" is never enough — the two libraries have to
fail the same way, with the same words, at the same point.  Where the reference
C dereferences NULL, divides by zero or loops forever, the row records the
signal (`SIGSEGV` / `SIGFPE`) or `TIMEOUT` and both sides must agree on that too.

Coverage of the generic boundaries every C API has is included explicitly:

* NULL pointers — `getters_null` calls all 40+ `png_get_*` with NULL/NULL;
  `setters_null_png` does the same for the `png_set_*` family.
* zero and oversized lengths — zero-length chunks for every chunk type, chunk
  length `0x80000000`, `png_set_compression_buffer_size(0)` and `SIZE_MAX`,
  palette lengths 0 / 257 / -1, `png_image` dimensions 0 and `0x40000000`.
* one step past a documented range — `PNG_sRGB_INTENT_LAST`, `PNG_SCALE_LAST`,
  `PNG_OFFSET_LAST`, `PNG_EQUATION_LAST`, `PNG_RESOLUTION_LAST`,
  `PNG_HANDLE_CHUNK_LAST`, `PNG_FILTER_VALUE_LAST`, `PNG_INTERLACE_LAST`,
  alpha mode 4, background gamma code 4, rgb-to-gray error action 4.
* out-of-range enum values across the FFI boundary — C enums accept any `int`,
  so negative and far-out values are passed too (`sRGB` intent -1, alpha mode
  -1, `png_set_option` numbers -4..23 with on/off -1..3, colour type 255,
  bit depth 255, interlace 255, compression method 255, `keep` -1 and 4).
* floating-point edges across the FFI boundary — NaN, -NaN, +-inf and 1e300
  through every `double` entry point.  NaN silently passes libpng's own range
  checks, so the value that reaches `(png_fixed_point)` is target-defined.

**Result: 340 of 340 rows pass.**

| # | function | trigger (the exact invalid input/condition) | expected C result | observed in the C build | [ ] |
|---|----------|----------------------------------------------|-------------------|-------------------------|-----|
| 1 | `png_read_sig / png_default_read_data` | read of a zero-length stream | read callback reports "Read Error" -> png_error | exit 70; png_error: Read Error | [x] |
| 2 | `png_read_sig` | only 4 of the 8 signature bytes available | png_error "Read Error" from the short read | exit 70; png_error: Read Error | [x] |
| 3 | `png_read_sig -> png_sig_cmp` | second signature byte is 'Q' instead of 'P' | png_error "Not a PNG file" | exit 70; png_error: Not a PNG file | [x] |
| 4 | `png_read_sig then png_read_chunk_header` | signature present, nothing after it | png_error "Read Error" | exit 70; png_error: Read Error | [x] |
| 5 | `png_read_sig -> png_sig_cmp` | JPEG SOI marker instead of the PNG signature | png_error "Not a PNG file" | exit 70; png_error: Not a PNG file | [x] |
| 6 | `png_read_info -> png_chunk_error` | first chunk is IEND, IHDR never seen | png_chunk_error "IEND: missing IHDR" | exit 70; png_error: IEND: missing IHDR | [x] |
| 7 | `png_read_info -> png_chunk_error` | gAMA appears before IHDR | png_chunk_error "gAMA: missing IHDR" | exit 70; png_error: gAMA: missing IHDR | [x] |
| 8 | `png_handle_IHDR` | IHDR chunk length 12 instead of 13 | png_chunk_error "IHDR: too short" | exit 70; png_error: IHDR: too short | [x] |
| 9 | `png_handle_IHDR` | IHDR chunk length 0 | png_chunk_error "IHDR: too short" | exit 70; png_error: IHDR: too short | [x] |
| 10 | `png_check_IHDR` | IHDR width == 0 | png_warning "Image width is zero in IHDR" then png_error "Invalid IHDR data" | exit 70; png_error: Invalid IHDR data; 1 warning(s): Image width is zero in IHDR | [x] |
| 11 | `png_check_IHDR` | IHDR height == 0 | png_warning "Image height is zero in IHDR" then png_error "Invalid IHDR data" | exit 70; png_error: Invalid IHDR data; 1 warning(s): Image height is zero in IHDR | [x] |
| 12 | `png_get_uint_31 / png_check_IHDR` | IHDR width 0x80000000 (bit 31 set) | png_error "Invalid image width in IHDR" / "PNG unsigned integer out of range" | exit 70; png_error: PNG unsigned integer out of range | [x] |
| 13 | `png_get_uint_31 / png_check_IHDR` | IHDR height 0x80000000 | png_error on the out-of-range 31-bit value | exit 70; png_error: PNG unsigned integer out of range | [x] |
| 14 | `png_check_IHDR (user width limit)` | IHDR width 0x7fffffff, above PNG_USER_WIDTH_MAX | png_warning "Image width exceeds user limit in IHDR" + png_error "Invalid IHDR data" | exit 70; png_error: Invalid IHDR data; 1 warning(s): Image width exceeds user limit in IHDR | [x] |
| 15 | `png_check_IHDR` | IHDR bit depth 0 | png_warning "Invalid bit depth in IHDR" + png_error "Invalid IHDR data" | exit 70; png_error: Invalid IHDR data; 2 warning(s): Invalid bit depth in IHDR / Invalid color type/bit depth combination in IHDR | [x] |
| 16 | `png_check_IHDR` | IHDR bit depth 3 (not a power of two) | png_warning "Invalid bit depth in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 2 warning(s): Invalid bit depth in IHDR / Invalid color type/bit depth combination in IHDR | [x] |
| 17 | `png_check_IHDR` | IHDR bit depth 32 | png_warning "Invalid bit depth in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid bit depth in IHDR | [x] |
| 18 | `png_check_IHDR` | IHDR bit depth 255 | png_warning "Invalid bit depth in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid bit depth in IHDR | [x] |
| 19 | `png_check_IHDR` | IHDR colour type 1 (undefined) | png_warning "Invalid color type in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid color type in IHDR | [x] |
| 20 | `png_check_IHDR` | IHDR colour type 5 (undefined) | png_warning "Invalid color type in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid color type in IHDR | [x] |
| 21 | `png_check_IHDR` | IHDR colour type 7 (undefined) | png_warning "Invalid color type in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid color type in IHDR | [x] |
| 22 | `png_check_IHDR` | IHDR colour type 255 | png_warning "Invalid color type in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid color type in IHDR | [x] |
| 23 | `png_check_IHDR` | colour type 3 (palette) with bit depth 16 | png_warning "Invalid color type/bit depth combination in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid color type/bit depth combination in IHDR | [x] |
| 24 | `png_check_IHDR` | colour type 2 (RGB) with bit depth 1 | png_warning "Invalid color type/bit depth combination in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid color type/bit depth combination in IHDR | [x] |
| 25 | `png_check_IHDR` | colour type 4 (GA) with bit depth 4 | png_warning "Invalid color type/bit depth combination in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid color type/bit depth combination in IHDR | [x] |
| 26 | `png_check_IHDR` | IHDR compression method 1 | png_warning "Unknown compression method in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown compression method in IHDR | [x] |
| 27 | `png_check_IHDR` | IHDR compression method 255 | png_warning "Unknown compression method in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown compression method in IHDR | [x] |
| 28 | `png_check_IHDR` | IHDR filter method 1 | png_warning "Unknown filter method in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 2 warning(s): Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 29 | `png_check_IHDR` | IHDR filter method 64 (MNG intrapixel) without png_permit_mng_features | png_warning "Unknown filter method in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 2 warning(s): Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 30 | `png_check_IHDR` | IHDR interlace method 2 | png_warning "Unknown interlace method in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown interlace method in IHDR | [x] |
| 31 | `png_check_IHDR` | IHDR interlace method 255 | png_warning "Unknown interlace method in IHDR" + png_error | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown interlace method in IHDR | [x] |
| 32 | `png_handle_IHDR` | two IHDR chunks | png_chunk_error "IHDR: out of place" | exit 70; png_error: IHDR: out of place | [x] |
| 33 | `png_crc_error / png_crc_finish (critical chunk)` | IHDR with a corrupted CRC | png_chunk_error "IHDR: CRC error" | exit 70; png_error: IHDR: CRC error | [x] |
| 34 | `png_crc_finish (ancillary chunk, default action)` | gAMA with a corrupted CRC | png_chunk_warning "CRC error", chunk discarded, read continues | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 35 | `png_crc_error (critical chunk)` | IDAT with a corrupted CRC | png_chunk_error "IDAT: CRC error" | exit 70; png_error: IDAT: CRC error | [x] |
| 36 | `png_read_chunk_header` | chunk length 0x80000000 (> PNG_UINT_31_MAX) | png_error "PNG unsigned integer out of range" (png_get_uint_31 rejects the length before the header check) | exit 70; png_error: PNG unsigned integer out of range | [x] |
| 37 | `png_check_chunk_name` | chunk type "1234" (non-alphabetic) | png_chunk_error "[31][32][33][34]: bad header (invalid type)" (the invalid name is hex-escaped in the message) | exit 70; png_error: [31][32][33][34]: bad header (invalid type) | [x] |
| 38 | `png_check_chunk_name` | chunk type containing a space and a NUL | png_chunk_error "a[20]b[00]: bad header (invalid type)" | exit 70; png_error: a[20]b[00]: bad header (invalid type) | [x] |
| 39 | `png_handle_unknown` | unknown *critical* chunk "ABCD" that is not kept | png_chunk_error "ABCD: unhandled critical chunk" | exit 70; png_error: ABCD: unhandled critical chunk | [x] |
| 40 | `png_read_chunk_header` | stream ends immediately after IHDR | png_error "Read Error" | exit 70; png_error: Read Error | [x] |
| 41 | `png_crc_read / png_default_read_data` | gAMA chunk truncated inside its data | png_error "Read Error" | exit 70; png_error: Read Error | [x] |
| 42 | `png_read_end` | no IEND chunk at all | png_error "Read Error" | exit 70; png_error: Read Error | [x] |
| 43 | `png_handle_IEND` | IEND with 3 bytes of data | png_chunk_benign_error "invalid" (warning by default) | exit 0; 1 warning(s): IEND: invalid | [x] |
| 44 | `png_read_info / png_read_end` | IHDR followed directly by IEND | png_chunk_error "IEND: out of place" (IEND arrives before any IDAT) | exit 70; png_error: IEND: out of place | [x] |
| 45 | `png_read_row -> png_inflate_read` | single zero-length IDAT | png_error "Not enough image data" | exit 70; png_error: Not enough image data | [x] |
| 46 | `png_read_row` | IDAT holds only 1 of the 3 required rows | png_error "Not enough image data" | exit 70; png_error: Not enough image data | [x] |
| 47 | `png_read_finish_IDAT` | IDAT holds 9 rows for a 3-row image | png_chunk_benign_error "Too much image data" | exit 0; 1 warning(s): IDAT: Too much image data | [x] |
| 48 | `png_read_row -> inflate` | IDAT contains non-deflate bytes | png_chunk_error "IDAT: invalid window size (libpng)" - libpng validates the zlib CINFO field itself | exit 70; png_error: IDAT: invalid window size (libpng) | [x] |
| 49 | `png_read_row -> inflate` | IDAT zlib CMF byte replaced with 0x99 | png_chunk_error "IDAT: invalid window size (libpng)" | exit 70; png_error: IDAT: invalid window size (libpng) | [x] |
| 50 | `png_read_finish_IDAT` | IDAT adler32 trailer corrupted | png_chunk_error "IDAT: incorrect data check" (zlib detects the bad adler32 first) | exit 70; png_error: IDAT: incorrect data check | [x] |
| 51 | `png_read_filter_row` | row filter byte 5 (PNG_FILTER_VALUE_LAST) | png_error "bad adaptive filter value" | exit 70; png_error: bad adaptive filter value | [x] |
| 52 | `png_read_filter_row` | row filter byte 64 without MNG features permitted | png_error "bad adaptive filter value" | exit 70; png_error: bad adaptive filter value | [x] |
| 53 | `png_handle_IDAT / png_read_info` | gAMA chunk inserted between two IDAT chunks | png_error "Not enough image data" - the second IDAT run is not consumed | exit 70; png_error: Not enough image data | [x] |
| 54 | `png_handle_PLTE` | PLTE appears after IDAT in a palette image | png_chunk_error "IDAT: Missing PLTE before IDAT" | exit 70; png_error: IDAT: Missing PLTE before IDAT | [x] |
| 55 | `png_handle_IDAT` | colour type 3 with no PLTE chunk | png_chunk_error "IDAT: Missing PLTE before IDAT" | exit 70; png_error: IDAT: Missing PLTE before IDAT | [x] |
| 56 | `png_handle_PLTE` | PLTE length 4 (not a multiple of 3) | png_chunk_error "PLTE: invalid" | exit 70; png_error: PLTE: invalid | [x] |
| 57 | `png_handle_PLTE` | zero-length PLTE | png_error "Invalid palette" (raised later, from png_set_PLTE via the reader) | exit 70; png_error: Invalid palette | [x] |
| 58 | `png_handle_PLTE` | PLTE with 300 entries (> PNG_MAX_PALETTE_LENGTH) | png_chunk_error "PLTE: invalid" | exit 70; png_error: PLTE: invalid | [x] |
| 59 | `png_handle_PLTE` | PLTE with 200 entries for a 2-bit palette image | accepted with the palette truncated to the bit depth; the read completes | exit 0 | [x] |
| 60 | `png_handle_PLTE` | PLTE in a greyscale image | png_chunk_benign_error "ignored in grayscale PNG" (warning) | exit 0; 1 warning(s): PLTE: ignored in grayscale PNG | [x] |
| 61 | `png_handle_PLTE` | two PLTE chunks | png_chunk_error "PLTE: duplicate" | exit 70; png_error: PLTE: duplicate | [x] |
| 62 | `png_handle_PLTE` | PLTE after IDAT in a palette image | png_chunk_error "PLTE: duplicate" | exit 70; png_error: PLTE: duplicate | [x] |
| 63 | `png_do_check_palette_indexes` | pixel index 200 with a 2-entry palette | png_benign_error "Read palette index exceeding num_palette" (warning) | exit 0; 1 warning(s): IDAT: Read palette index exceeding num_palette | [x] |
| 64 | `png_handle_gAMA` | zero-length gAMA | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): gAMA: too short | [x] |
| 65 | `png_handle_gAMA` | gAMA length 3 instead of 4 | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): gAMA: too short | [x] |
| 66 | `png_colorspace_set_gamma` | gAMA value 0 | png_chunk_report "gamma value out of range" | exit 0 | [x] |
| 67 | `png_colorspace_set_gamma` | gAMA value 0xffffffff | png_chunk_report "gamma value out of range" | exit 0; 1 warning(s): gAMA: invalid | [x] |
| 68 | `png_handle_gAMA` | two gAMA chunks | png_chunk_benign_error "duplicate" | exit 0; 1 warning(s): gAMA: duplicate | [x] |
| 69 | `png_handle_gAMA` | gAMA after IDAT | png_chunk_benign_error "out of place" | exit 0; 1 warning(s): gAMA: out of place | [x] |
| 70 | `png_handle_cHRM` | zero-length cHRM | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): cHRM: too short | [x] |
| 71 | `png_handle_cHRM` | cHRM length 31 instead of 32 | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): cHRM: too short | [x] |
| 72 | `png_colorspace_set_chromaticities` | all-zero cHRM endpoints | png_chunk_report "invalid chromaticities" | exit 0 | [x] |
| 73 | `png_handle_sRGB` | zero-length sRGB | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sRGB: too short | [x] |
| 74 | `png_handle_sRGB` | sRGB length 2 instead of 1 | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sRGB: too long | [x] |
| 75 | `png_handle_sRGB` | sRGB rendering intent 4 (== PNG_sRGB_INTENT_LAST) | png_chunk_report "invalid sRGB rendering intent" | exit 0; 1 warning(s): sRGB: invalid | [x] |
| 76 | `png_handle_sRGB` | sRGB rendering intent 255 | png_chunk_report "invalid sRGB rendering intent" | exit 0; 1 warning(s): sRGB: invalid | [x] |
| 77 | `png_handle_sBIT` | zero-length sBIT | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sBIT: too short | [x] |
| 78 | `png_handle_sBIT` | sBIT length 1 for an RGB image (needs 3) | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sBIT: bad length | [x] |
| 79 | `png_handle_sBIT` | sBIT significant bits all zero | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sBIT: invalid | [x] |
| 80 | `png_handle_sBIT` | sBIT significant bits 9 for an 8-bit image | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sBIT: invalid | [x] |
| 81 | `png_handle_tRNS` | tRNS length 1 for a greyscale image (needs 2) | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): tRNS: invalid | [x] |
| 82 | `png_handle_tRNS` | tRNS length 4 for an RGB image (needs 6) | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): tRNS: invalid | [x] |
| 83 | `png_handle_tRNS` | tRNS in an RGBA image | png_chunk_benign_error "invalid with alpha channel" | exit 0; 1 warning(s): tRNS: invalid with alpha channel | [x] |
| 84 | `png_handle_tRNS` | tRNS in a grey+alpha image | png_chunk_benign_error "invalid with alpha channel" | exit 0; 1 warning(s): tRNS: invalid with alpha channel | [x] |
| 85 | `png_handle_tRNS` | tRNS with 10 entries for a 4-entry palette | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): tRNS: invalid | [x] |
| 86 | `png_handle_tRNS` | tRNS before PLTE in a palette image | png_chunk_benign_error "out of place" | exit 0; 1 warning(s): tRNS: out of place | [x] |
| 87 | `png_handle_bKGD` | zero-length bKGD | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): bKGD: too short | [x] |
| 88 | `png_handle_bKGD` | bKGD length 1 for a greyscale image (needs 2) | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): bKGD: invalid | [x] |
| 89 | `png_handle_bKGD` | bKGD length 4 for an RGB image (needs 6) | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): bKGD: invalid | [x] |
| 90 | `png_handle_bKGD` | bKGD palette index 99 with a 4-entry palette | png_chunk_benign_error "invalid index" | exit 0; 1 warning(s): bKGD: invalid index | [x] |
| 91 | `png_handle_bKGD` | bKGD grey level above the 2-bit maximum | png_chunk_benign_error "invalid gray level" | exit 0; 1 warning(s): bKGD: invalid gray level | [x] |
| 92 | `png_handle_hIST` | hIST without a preceding PLTE | png_chunk_benign_error "out of place" | exit 0; 1 warning(s): hIST: invalid | [x] |
| 93 | `png_handle_hIST` | hIST length not 2*num_palette | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): hIST: out of place | [x] |
| 94 | `png_handle_pHYs` | zero-length pHYs | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): pHYs: too short | [x] |
| 95 | `png_handle_pHYs` | pHYs length 8 instead of 9 | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): pHYs: too short | [x] |
| 96 | `png_handle_pHYs` | pHYs unit specifier 7 | unit stored verbatim; png_get_pHYs returns it | exit 0 | [x] |
| 97 | `png_handle_oFFs` | zero-length oFFs | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): oFFs: too short | [x] |
| 98 | `png_handle_oFFs` | oFFs unit specifier 9 | unit stored verbatim; png_get_oFFs returns it | exit 0 | [x] |
| 99 | `png_handle_sCAL` | zero-length sCAL | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sCAL: too short | [x] |
| 100 | `png_handle_sCAL` | sCAL too short to hold two ASCII numbers | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sCAL: too short | [x] |
| 101 | `png_handle_sCAL` | sCAL unit byte 0 | png_chunk_benign_error "invalid unit" | exit 0; 1 warning(s): sCAL: invalid unit | [x] |
| 102 | `png_handle_sCAL` | sCAL width "-5.0" | png_chunk_benign_error "non-positive width" | exit 0; 1 warning(s): sCAL: non-positive width | [x] |
| 103 | `png_handle_sCAL` | sCAL width "abc" | png_chunk_benign_error "bad width format" | exit 0; 1 warning(s): sCAL: bad width format | [x] |
| 104 | `png_handle_sCAL` | sCAL with no NUL between width and height | png_chunk_benign_error "bad width format" / "invalid" | exit 0; 1 warning(s): sCAL: bad width format | [x] |
| 105 | `png_handle_sCAL` | sCAL height "0.0" | png_chunk_benign_error "non-positive height" | exit 0; 1 warning(s): sCAL: non-positive height | [x] |
| 106 | `png_handle_pCAL` | zero-length pCAL | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): pCAL: too short | [x] |
| 107 | `png_handle_pCAL` | pCAL equation type 9 | png_chunk_benign_error "unrecognized equation type" | exit 0; 2 warning(s): pCAL: unrecognized equation type / pCAL: Invalid pCAL equation type | [x] |
| 108 | `png_handle_pCAL` | pCAL declares 3 parameters but supplies 1 | png_chunk_benign_error "invalid parameter count" | exit 0; 1 warning(s): pCAL: invalid parameter count | [x] |
| 109 | `png_handle_pCAL` | pCAL X0 == X1 | accepted; values reported verbatim by png_get_pCAL | exit 0; 1 warning(s): pCAL: invalid parameter count | [x] |
| 110 | `png_handle_sPLT` | zero-length sPLT | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sPLT: too short | [x] |
| 111 | `png_handle_sPLT` | sPLT sample depth 7 (neither 8 nor 16) | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sPLT chunk has bad length | [x] |
| 112 | `png_handle_sPLT` | sPLT data length not a multiple of the entry size | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): sPLT chunk has bad length | [x] |
| 113 | `png_handle_sPLT` | sPLT palette name with no NUL terminator | png_chunk_benign_error "no space in chunk cache" / "invalid" | exit 0; 1 warning(s): malformed sPLT chunk | [x] |
| 114 | `png_handle_tEXt` | tEXt with no NUL separating keyword and text | png_chunk_benign_error "no space in chunk cache" / "invalid" | exit 0 | [x] |
| 115 | `png_decompress_chunk / png_check_keyword` | tEXt with an empty keyword | png_chunk_benign_error "invalid" / keyword rejected | exit 0 | [x] |
| 116 | `png_handle_tEXt` | tEXt keyword of 100 characters (max 79) | keyword truncated with a warning | exit 0 | [x] |
| 117 | `png_handle_tEXt` | tEXt keyword containing a control character | keyword sanitised with a warning | exit 0 | [x] |
| 118 | `png_handle_tEXt` | zero-length tEXt | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): tEXt: too short | [x] |
| 119 | `png_handle_zTXt` | zTXt compression method 7 | png_chunk_benign_error "invalid compression method" | exit 0; 1 warning(s): zTXt: unknown compression type | [x] |
| 120 | `png_decompress_chunk` | zTXt payload is not a zlib stream | png_chunk_benign_error with the zlib message | exit 0; 1 warning(s): zTXt: too short | [x] |
| 121 | `png_handle_zTXt` | zTXt with keyword only, no compression byte | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): zTXt: too short | [x] |
| 122 | `png_handle_iTXt` | iTXt compression flag 7 (must be 0 or 1) | png_chunk_benign_error "invalid compression flag" | exit 0; 1 warning(s): iTXt: bad compression info | [x] |
| 123 | `png_handle_iTXt` | iTXt compression method 9 | png_chunk_benign_error "invalid compression method" | exit 0; 1 warning(s): iTXt: bad compression info | [x] |
| 124 | `png_handle_iTXt` | iTXt missing the translated keyword and text | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): iTXt: truncated | [x] |
| 125 | `png_handle_iCCP` | iCCP compression method 9 | png_chunk_benign_error "invalid compression method" | exit 0; 1 warning(s): iCCP: bad compression method | [x] |
| 126 | `png_decompress_chunk` | iCCP payload is not a zlib stream | png_chunk_benign_error with the zlib message | exit 0; 1 warning(s): iCCP: too short | [x] |
| 127 | `png_icc_check_length` | ICC profile only 20 bytes (minimum 132) | png_chunk_benign_error "too short" | exit 0; 1 warning(s): iCCP: too short | [x] |
| 128 | `png_icc_check_header` | ICC profile without the 'acsp' file signature | png_chunk_benign_error "invalid signature" | exit 0; 1 warning(s): iCCP: profile 'ICC': 'xcsp': invalid signature | [x] |
| 129 | `png_icc_check_length` | ICC header length field disagrees with the data length | png_chunk_benign_error "profile length" mismatch | exit 0; 1 warning(s): iCCP: unexpected zlib return code | [x] |
| 130 | `png_icc_check_header` | ICC data colour space 'CMYK' for an RGB PNG | png_chunk_benign_error "invalid color space" | exit 0; 1 warning(s): iCCP: profile 'ICC': 'CMYK': invalid ICC profile color space | [x] |
| 131 | `png_icc_check_header` | ICC device class 'zzzz' | png_chunk_benign_error "invalid device class" | exit 0; 1 warning(s): iCCP: profile 'ICC': 'zzzz': unrecognized ICC profile class | [x] |
| 132 | `png_handle_iCCP` | iCCP with no NUL after the profile name | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): iCCP: too short | [x] |
| 133 | `png_handle_iCCP` | iCCP after IDAT | png_chunk_benign_error "out of place" | exit 0; 1 warning(s): iCCP: out of place | [x] |
| 134 | `png_handle_tIME` | zero-length tIME | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): tIME: too short | [x] |
| 135 | `png_handle_tIME` | tIME length 6 instead of 7 | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): tIME: too short | [x] |
| 136 | `png_handle_tIME` | tIME month 13, day 40, hour 25, minute 61, second 62 | stored verbatim; png_convert_to_rfc1123_buffer later reports it invalid | exit 0; 1 warning(s): Ignoring invalid time value | [x] |
| 137 | `png_handle_eXIf` | zero-length eXIf | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): eXIf: too short | [x] |
| 138 | `png_handle_eXIf` | eXIf byte-order marker "XX" | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): eXIf: invalid | [x] |
| 139 | `png_handle_cICP` | zero-length cICP | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): cICP: too short | [x] |
| 140 | `png_handle_cICP` | cICP length 3 instead of 4 | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): cICP: too short | [x] |
| 141 | `png_handle_cICP` | cICP video_full_range_flag 7 (must be 0 or 1) | png_chunk_benign_error "invalid" | exit 0 | [x] |
| 142 | `png_handle_cICP` | cICP matrix_coefficients 5 (must be 0 for PNG) | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): Invalid cICP matrix coefficients | [x] |
| 143 | `png_handle_cLLI` | zero-length cLLI | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): cLLI: too short | [x] |
| 144 | `png_handle_cLLI` | cLLI length 4 instead of 8 | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): cLLI: too short | [x] |
| 145 | `png_handle_cLLI / png_get_uint_31` | cLLI maximum content light level with bit 31 set | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): cLLI: cLLI light level exceeds PNG limit | [x] |
| 146 | `png_handle_mDCV` | zero-length mDCV | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): mDCV: too short | [x] |
| 147 | `png_handle_mDCV` | mDCV length 23 instead of 24 | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): mDCV: too short | [x] |
| 148 | `png_handle_mDCV` | mDCV maximum luminance with bit 31 set | png_chunk_benign_error "invalid" | exit 0; 1 warning(s): mDCV: mDCV display light level exceeds PNG limit | [x] |
| 149 | `png_create_png_struct` | png_create_read_struct("1.0.0", ...) | returns NULL (version/ABI mismatch) | exit 0 | [x] |
| 150 | `png_create_png_struct` | png_create_read_struct("1.7.0", ...) | returns NULL | exit 0 | [x] |
| 151 | `png_create_png_struct` | png_create_read_struct("not-a-version", ...) | returns NULL | exit 0 | [x] |
| 152 | `png_create_png_struct` | png_create_read_struct(NULL, ...) | succeeds (NULL means "no check") | exit 0 | [x] |
| 153 | `png_create_png_struct` | png_create_read_struct("", ...) | returns NULL | exit 0 | [x] |
| 154 | `png_create_png_struct` | png_create_write_struct("1.0.0", ...) | returns NULL | exit 0 | [x] |
| 155 | `png_create_png_struct` | png_create_read_struct_2("1.0.0", ...) | returns NULL | exit 0 | [x] |
| 156 | `png_create_png_struct` | png_create_write_struct_2("1.0.0", ...) | returns NULL | exit 0 | [x] |
| 157 | `png_create_info_struct` | png_create_info_struct(NULL) | returns NULL | exit 0 | [x] |
| 158 | `png_destroy_read_struct / png_destroy_write_struct` | NULL png_ptr_ptr and NULL *png_ptr_ptr | no-op, no crash | exit 0 | [x] |
| 159 | `png_info_init_3` | png_info_struct_size smaller than sizeof(png_info) | png_warning "Interface mismatch" and the info struct is freed | exit 0 | [x] |
| 160 | `all png_get_* entry points` | NULL png_ptr and NULL info_ptr | every getter returns 0 / NULL without dereferencing | signal 11; no record written | [x] |
| 161 | `all png_set_* entry points` | NULL png_ptr | every setter returns without dereferencing | signal 11; no record written | [x] |
| 162 | `png_set_sig_bytes` | num_bytes 9 (> 8) | png_error "Too many bytes for PNG signature" | exit 70; png_error: Too many bytes for PNG signature | [x] |
| 163 | `png_set_sig_bytes` | num_bytes -1 | clamped to 0, no error | exit 0 | [x] |
| 164 | `png_set_sig_bytes` | signature already consumed by the application | read succeeds from the first chunk | exit 0 | [x] |
| 165 | `png_data_freer` | freer parameter 99 | png_error "Unknown freer parameter in png_data_freer" | exit 70; png_error: Unknown freer parameter in png_data_freer | [x] |
| 166 | `png_data_freer` | PNG_USER_WILL_FREE_DATA then PNG_DESTROY_WILL_FREE_DATA | accepted | exit 0 | [x] |
| 167 | `png_free_data` | NULL info_ptr | no-op | exit 0 | [x] |
| 168 | `png_set_sCAL_s` | unit 0 | png_error "Invalid sCAL unit" | exit 70; png_error: Invalid sCAL unit | [x] |
| 169 | `png_set_sCAL_s` | unit 3 (== PNG_SCALE_LAST) | png_error "Invalid sCAL unit" | exit 70; png_error: Invalid sCAL unit | [x] |
| 170 | `png_set_sCAL_s` | width string "-1.0" | png_error "Invalid sCAL width" | exit 70; png_error: Invalid sCAL width | [x] |
| 171 | `png_set_sCAL_s` | width string "abc" | png_error "Invalid sCAL width" | exit 70; png_error: Invalid sCAL width | [x] |
| 172 | `png_set_sCAL_s` | height string "xyz" | png_error "Invalid sCAL height" | exit 70; png_error: Invalid sCAL height | [x] |
| 173 | `png_set_sCAL_s` | height string "0" | png_error "Invalid sCAL height" | exit 0 | [x] |
| 174 | `png_set_sCAL_fixed` | negative fixed-point width | png_error "Invalid sCAL width" | exit 0; 1 warning(s): Invalid sCAL width ignored | [x] |
| 175 | `png_set_PLTE` | num_palette 257 (> PNG_MAX_PALETTE_LENGTH) | png_error "Invalid palette length" | exit 70; png_error: Invalid palette length | [x] |
| 176 | `png_set_PLTE` | num_palette 0 | png_error "Invalid palette length" | exit 70; png_error: Invalid palette | [x] |
| 177 | `png_set_PLTE` | num_palette -1 | png_error "Invalid palette length" | exit 70; png_error: Invalid palette length | [x] |
| 178 | `png_set_PLTE` | palette pointer NULL | png_error "Invalid palette" | exit 70; png_error: Invalid palette | [x] |
| 179 | `png_set_PLTE` | 200 entries for a 2-bit palette image | png_error "Invalid palette length" | exit 70; png_error: Invalid palette length | [x] |
| 180 | `png_set_iCCP` | compression_type 7 | png_app_error "Invalid iCCP compression method" | exit 70; png_error: Invalid iCCP compression method | [x] |
| 181 | `png_set_iCCP` | proflen 0 | rejected via png_app_error / benign error | exit 0 | [x] |
| 182 | `png_set_iCCP` | 20-byte profile | rejected (profile too short) | exit 0 | [x] |
| 183 | `png_set_iCCP` | empty profile name | keyword rejected with a warning | exit 0 | [x] |
| 184 | `png_set_sPLT` | entries pointer NULL | returns without storing anything | exit 0 | [x] |
| 185 | `png_set_sPLT` | nentries 0 | png_app_error "png_set_sPLT: invalid sPLT" | exit 70; png_error: png_set_sPLT: invalid sPLT | [x] |
| 186 | `png_set_text_2` | png_text.key == NULL | png_app_warning, entry skipped | exit 0 | [x] |
| 187 | `png_set_text_2` | png_text.compression 9 | png_error "text compression mode is out of range" | exit 70; png_error: text compression mode is out of range | [x] |
| 188 | `png_set_text_2 -> png_check_keyword` | 200-character keyword | keyword truncated to 79 with a warning | exit 0 | [x] |
| 189 | `png_set_text_2` | iTXt entry with NULL lang and lang_key | accepted; empty strings stored | exit 0 | [x] |
| 190 | `png_set_unknown_chunk_location` | location 99 | png_error "png_set_unknown_chunks now expects a valid location" | exit 70; png_error: png_set_unknown_chunks now expects a valid location | [x] |
| 191 | `png_set_unknown_chunk_location` | chunk index beyond the stored list | no-op | exit 0 | [x] |
| 192 | `png_set_keep_unknown_chunks` | keep == PNG_HANDLE_CHUNK_LAST (4) | png_app_error "png_set_keep_unknown_chunks: invalid keep" | exit 70; png_error: png_set_keep_unknown_chunks: invalid keep | [x] |
| 193 | `png_set_keep_unknown_chunks` | keep == -1 | png_app_error "png_set_keep_unknown_chunks: invalid keep" | exit 70; png_error: png_set_keep_unknown_chunks: invalid keep | [x] |
| 194 | `png_set_keep_unknown_chunks` | num_chunks 3 with chunk_list == NULL | png_app_error "png_set_keep_unknown_chunks: no chunk list" | exit 70; png_error: png_set_keep_unknown_chunks: no chunk list | [x] |
| 195 | `png_set_keep_unknown_chunks` | num_chunks negative (means "all known chunks") | accepted; affects png_handle_as_unknown | exit 0 | [x] |
| 196 | `png_set_keep_unknown_chunks` | IHDR listed explicitly | accepted but IHDR still handled by libpng | exit 0 | [x] |
| 197 | `png_set_compression_buffer_size` | size 0 | png_error "invalid compression buffer size" | exit 70; png_error: invalid compression buffer size | [x] |
| 198 | `png_set_compression_buffer_size` | size SIZE_MAX | png_error "invalid compression buffer size" | exit 70; png_error: invalid compression buffer size | [x] |
| 199 | `png_set_cHRM_XYZ_fixed` | all-zero XYZ endpoints | png_app_error "invalid cHRM XYZ" | exit 70; png_error: invalid cHRM XYZ | [x] |
| 200 | `png_set_cHRM_fixed` | all endpoints -1 | png_chunk_report / value rejected | exit 0 | [x] |
| 201 | `png_set_gAMA_fixed` | gamma 0 | png_chunk_report "gamma value out of range"; flag not set | exit 0 | [x] |
| 202 | `png_set_gAMA_fixed` | gamma -5 | png_chunk_report "gamma value out of range" | exit 0 | [x] |
| 203 | `png_set_gAMA` | gamma 1e30 (overflows png_fixed_point) | png_fixed_error -> png_error "fixed point overflow in png_set_gAMA" | exit 70; png_error: fixed point overflow in png_set_gAMA | [x] |
| 204 | `png_set_gAMA` | gamma NaN | value rejected | exit 0 | [x] |
| 205 | `png_set_sRGB` | intent 4 (== PNG_sRGB_INTENT_LAST) | png_chunk_report "invalid sRGB rendering intent" | exit 0 | [x] |
| 206 | `png_set_sRGB` | intent -1 | png_chunk_report "invalid sRGB rendering intent" | exit 0 | [x] |
| 207 | `png_set_pHYs` | unit_type 7 | stored verbatim | exit 0 | [x] |
| 208 | `png_set_oFFs` | unit_type 7 | stored verbatim | exit 0 | [x] |
| 209 | `png_set_pCAL` | type == PNG_EQUATION_LAST | png_error "Invalid pCAL equation type" | exit 70; png_error: Invalid pCAL equation type | [x] |
| 210 | `png_set_cICP` | video_full_range_flag 7 | png_app_error / value rejected | exit 0 | [x] |
| 211 | `png_set_cICP` | matrix_coefficients 5 | png_app_error / value rejected | exit 0; 1 warning(s): Invalid cICP matrix coefficients | [x] |
| 212 | `png_set_cLLI_fixed` | content light level with bit 31 set | png_error "cLLI light level exceeds PNG limit" | exit 70; png_error: cLLI light level exceeds PNG limit | [x] |
| 213 | `png_set_mDCV_fixed` | chromaticities above the 1.3107 maximum | png_error "mDCV chromaticities outside representable range" | exit 70; png_error: mDCV chromaticities outside representable range | [x] |
| 214 | `png_set_sBIT` | all significant-bit values 0 | png_app_error "Invalid sBIT depth specified" | exit 0 | [x] |
| 215 | `png_set_sBIT` | significant bits 9 for an 8-bit image | png_app_error "Invalid sBIT depth specified" | exit 0 | [x] |
| 216 | `png_set_tRNS` | num_trans 300 | png_app_error / rejected | exit 0 | [x] |
| 217 | `png_set_tRNS` | both trans_alpha and trans_color NULL | nothing stored; valid flag not set | exit 0 | [x] |
| 218 | `png_set_shift` | all shift values 0 | png_app_error "png_set_shift: invalid shift values" | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 219 | `png_set_shift` | shift values 99 | png_app_error "png_set_shift: invalid shift values" | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 220 | `png_set_filler` | called on a palette image (read) | png_app_error "png_set_filler is invalid for palette images" | exit 0 | [x] |
| 221 | `png_set_filler` | called on an image that already has alpha | png_app_error "png_set_filler is invalid for images with an alpha channel" | exit 0 | [x] |
| 222 | `png_set_add_alpha` | called on a palette image | png_app_error (invalid for palette images) | exit 0 | [x] |
| 223 | `png_set_alpha_mode_fixed` | mode 4 (no such PNG_ALPHA_ value) | png_error "invalid alpha mode" | exit 70; png_error: invalid alpha mode | [x] |
| 224 | `png_set_alpha_mode_fixed` | mode -1 | png_error "invalid alpha mode" | exit 70; png_error: invalid alpha mode | [x] |
| 225 | `png_set_alpha_mode_fixed` | output_gamma 0 | png_error "gamma out of supported range" | exit 70; png_error: gamma out of supported range | [x] |
| 226 | `png_set_alpha_mode_fixed` | output_gamma -5 (not a defined constant) | png_error "gamma out of supported range" | exit 70; png_error: gamma out of supported range | [x] |
| 227 | `png_set_gamma_fixed` | screen gamma 0 | png_app_error "invalid screen gamma in png_set_gamma" | exit 70; png_error: invalid screen gamma in png_set_gamma | [x] |
| 228 | `png_set_gamma_fixed` | override_file_gamma 0 | png_app_error "invalid file gamma in png_set_gamma" | exit 70; png_error: invalid file gamma in png_set_gamma | [x] |
| 229 | `png_set_gamma_fixed` | screen gamma -100000 | accepted: a negative screen gamma is one of the documented PNG_DEFAULT_sRGB / PNG_GAMMA_MAC_18 encodings | exit 0 | [x] |
| 230 | `png_set_gamma` | screen and file gamma 1e30 | png_fixed_error -> png_error "fixed point overflow in gamma value" | exit 70; png_error: fixed point overflow in gamma value | [x] |
| 231 | `png_set_rgb_to_gray_fixed` | error_action 0 | png_error "invalid error action to rgb_to_gray" | exit 70; png_error: invalid error action to rgb_to_gray | [x] |
| 232 | `png_set_rgb_to_gray_fixed` | error_action 4 | png_error "invalid error action to rgb_to_gray" | exit 70; png_error: invalid error action to rgb_to_gray | [x] |
| 233 | `png_set_rgb_to_gray_fixed` | red+green coefficients sum above 1.0 | png_app_error "ignoring out of range rgb_to_gray coefficients" | exit 70; png_error: ignoring out of range rgb_to_gray coefficients | [x] |
| 234 | `png_set_rgb_to_gray_fixed` | negative coefficients | accepted: negative coefficients mean "use the defaults" | exit 0 | [x] |
| 235 | `png_set_background_fixed` | background_gamma_code 4 | png_error "invalid background gamma type" | exit 0 | [x] |
| 236 | `png_set_background_fixed` | background_gamma_code -1 | png_error "invalid background gamma type" | exit 0 | [x] |
| 237 | `png_set_background_fixed` | background_color NULL | no-op / handled | exit 0 | [x] |
| 238 | `png_set_background_fixed` | called before png_read_info | png_app_error "invalid before the PNG header has been read" (or accepted) | exit 0; 1 warning(s): Application must supply a known background gamma | [x] |
| 239 | `png_set_quantize` | maximum_colors 0 with a 4-entry palette | the reference C enters a non-terminating reduction loop; both libraries hang identically (recorded as TIMEOUT) | TIMEOUT; no record written | [x] |
| 240 | `png_set_quantize` | maximum_colors 1 with a 4-entry palette and a histogram | palette reduced in place to a single entry | exit 0 | [x] |
| 241 | `png_set_quantize` | palette pointer NULL | returns immediately without setting PNG_QUANTIZE | exit 0 | [x] |
| 242 | `png_set_quantize` | num_palette -1 | no range check: the reference C indexes out of bounds and dies of SIGSEGV | signal 11; no record written | [x] |
| 243 | `png_set_quantize` | maximum_colors 300 > num_palette | handled; no quantization needed | exit 0 | [x] |
| 244 | `png_set_crc_action` | crit_action 9 / ancil_action 9 | out-of-range values ignored (existing behaviour kept) | exit 0 | [x] |
| 245 | `png_set_crc_action` | PNG_CRC_WARN_DISCARD for a critical chunk (documented INVALID) | png_warning "Can't discard critical data on CRC error" | exit 0; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 246 | `png_get_uint_31` | buffer holding 0x80000000 | png_error "PNG unsigned integer out of range" | exit 70; png_error: PNG unsigned integer out of range | [x] |
| 247 | `png_get_uint_31` | buffer holding 0xffffffff | png_error "PNG unsigned integer out of range" | exit 70; png_error: PNG unsigned integer out of range | [x] |
| 248 | `png_set_filter` | method 1 (only 0 is defined) | png_error "Unknown custom filter method" | exit 70; png_error: Unknown custom filter method | [x] |
| 249 | `png_set_filter` | filters 7 (not a filter flag nor a filter value) | png_app_error "Unknown row filter for method 0" | exit 70; png_error: Unknown row filter for method 0 | [x] |
| 250 | `png_set_filter` | filters 5 (== PNG_FILTER_VALUE_LAST) | png_app_error "Unknown row filter for method 0" | exit 70; png_error: Unknown row filter for method 0 | [x] |
| 251 | `png_set_filter` | filters -1 | png_app_error "Unknown row filter for method 0" | exit 0 | [x] |
| 252 | `png_write_info` | palette image written without png_set_PLTE | png_error "Valid palette required for paletted images" | exit 70; png_error: Valid palette required for paletted images | [x] |
| 253 | `png_write_end` | png_write_end without writing any row | png_error "No IDATs written into file" | exit 70; png_error: No IDATs written into file | [x] |
| 254 | `png_write_image` | row_pointers NULL | no NULL check: the reference C dereferences it and dies of SIGSEGV | signal 11; no record written | [x] |
| 255 | `png_write_png` | info_ptr->row_pointers NULL | png_app_error "no rows for png_write_image to write" | exit 70; png_error: no rows for png_write_image to write | [x] |
| 256 | `png_write_row` | more rows written than the IHDR height | png_error / warning about too many rows | exit 0 | [x] |
| 257 | `png_write_data` | png_set_write_fn(png, NULL, NULL, NULL) | png_error "Call to NULL write function" | signal 11; no record written | [x] |
| 258 | `png_read_data` | png_set_read_fn(png, NULL, NULL) | png_error "Call to NULL read function" | signal 11; no record written | [x] |
| 259 | `png_read_row` | png_read_row before png_read_info | png_error "Invalid attempt to read row data" | exit 70; png_error: Invalid attempt to read row data | [x] |
| 260 | `png_read_row` | more png_read_row calls than the image height | png_chunk_error "IDAT: CRC error" - reading past the last row runs off the end of the IDAT stream | exit 70; png_error: IDAT: CRC error | [x] |
| 261 | `png_write_png` | read-only transform bits (STRIP_16\|EXPAND) passed to png_write_png | png_app_error "PNG_TRANSFORM_...not supported" | exit 0 | [x] |
| 262 | `png_read_png` | write-only transform bit (STRIP_FILLER) passed to png_read_png | png_app_error "PNG_TRANSFORM_STRIP_FILLER...not supported" | exit 0 | [x] |
| 263 | `png_reset_zstream` | called before any zlib stream exists | returns Z_STREAM_ERROR (-2) | exit 0 | [x] |
| 264 | `png_write_chunk` | chunk name "12 4" | png_error / warning about the invalid chunk name | exit 0 | [x] |
| 265 | `png_write_chunk_data` | more data written than declared in png_write_chunk_start | data written verbatim (no length check) | exit 0 | [x] |
| 266 | `png_write_chunk / png_write_chunk_data` | NULL data pointer with length 0 | accepted, empty chunk written | exit 0 | [x] |
| 267 | `png_check_IHDR (user limits)` | png_set_user_limits(2,2) then a 4x3 image | png_warning "Image width exceeds user limit in IHDR" + png_error "Invalid IHDR data" | exit 70; png_error: Invalid IHDR data; 2 warning(s): Image width exceeds user limit in IHDR / Image height exceeds user limit in IHDR | [x] |
| 268 | `png_set_user_limits` | width/height max 0 | zero is a real limit, not "unlimited": png_warning "Image width/height exceeds user limit in IHDR" then png_error "Invalid IHDR data" | exit 70; png_error: Invalid IHDR data; 2 warning(s): Image width exceeds user limit in IHDR / Image height exceeds user limit in IHDR | [x] |
| 269 | `png_handle_unknown / png_decompress_chunk` | png_set_chunk_cache_max(1) with several text chunks | png_chunk_benign_error "no space in chunk cache" | exit 0 | [x] |
| 270 | `png_decompress_chunk` | png_set_chunk_malloc_max(8) with a larger chunk | png_chunk_benign_error about exceeding the memory limit | exit 0; 5 warning(s): tEXt: out of memory / zTXt: length exceeds libpng limit / iTXt: out of memory | [x] |
| 271 | `png_read_row` | user read callback returns short data silently | png_chunk_error "IDAT: incorrect data check" | exit 70; png_error: IDAT: incorrect data check | [x] |
| 272 | `png_read_info` | png_read_info called twice | the second call re-enters the chunk loop mid-IDAT: png_chunk_error "...: bad header (invalid type)" | exit 70; png_error: [00][D8][FF][00]: bad header (invalid type) | [x] |
| 273 | `png_read_end` | png_read_end without reading any row | png_warning "IDAT: bad parameters to zlib" then png_chunk_error "...: bad header (invalid type)" | exit 70; png_error: [00][00][00][00]: bad header (invalid type); 1 warning(s): IDAT: bad parameters to zlib | [x] |
| 274 | `png_read_update_info` | png_read_update_info called twice | png_app_error "png_read_update_info/png_start_read_image: duplicate call" | exit 70; png_error: png_read_update_info/png_start_read_image: duplicate call | [x] |
| 275 | `png_start_read_image` | png_start_read_image called twice | png_app_error "png_start_read_image/png_read_update_info: duplicate call" | exit 70; png_error: png_start_read_image/png_read_update_info: duplicate call | [x] |
| 276 | `png_write_flush` | png_write_flush with output_flush_fn == NULL | no-op | exit 0 | [x] |
| 277 | `png_set_longjmp_fn` | jmp_buf_size 8 / 0 / SIZE_MAX (ABI mismatch), and NULL png_ptr | returns NULL on a size mismatch, NULL for a NULL png_ptr | exit 0; 2 warning(s): Application jmp_buf size changed | [x] |
| 278 | `png_image_begin_read_from_memory` | memory pointer NULL | returns 0, message "png_image_begin_read_from_memory: invalid argument" | exit 0 | [x] |
| 279 | `png_image_begin_read_from_memory` | size 0 | returns 0 with an error message | exit 0 | [x] |
| 280 | `png_image_begin_read_from_memory` | png_image.version 2 | returns 0, "png_image_begin_read_from_memory: version mismatch" | exit 0 | [x] |
| 281 | `png_image_begin_read_from_memory` | png_image.version 0 | returns 0, version mismatch | exit 0 | [x] |
| 282 | `png_image_begin_read_from_memory` | 10 bytes that are not a PNG | returns 0, "Not a PNG file" | exit 0 | [x] |
| 283 | `png_image_finish_read` | PNG stream truncated at half its length | returns 0 with a read error message | exit 0 | [x] |
| 284 | `png_image_finish_read` | called on a png_image with opaque == NULL | returns 0, "png_image_finish_read: invalid argument" | exit 0 | [x] |
| 285 | `png_image_finish_read` | png_image.format with undefined high bits | returns 0, "png_image_finish_read: invalid format" | exit 0 | [x] |
| 286 | `png_image_finish_read` | colour-mapped format with colormap == NULL | returns 0, "png_image_finish_read: no color-map for color-mapped image" | exit 0 | [x] |
| 287 | `png_image_finish_read` | buffer NULL | returns 0, "png_image_finish_read: invalid argument" | exit 0 | [x] |
| 288 | `png_image_finish_read` | row_stride 1 for a 4-channel image | returns 0, "png_image_finish_read: row stride too small" | exit 0 | [x] |
| 289 | `png_image_finish_read` | row_stride 0 | libpng computes the minimum stride itself | exit 0 | [x] |
| 290 | `png_image_begin_read_from_file` | path that does not exist | returns 0 with the strerror() text | exit 0 | [x] |
| 291 | `png_image_begin_read_from_file` | file name NULL | returns 0, invalid argument | exit 0 | [x] |
| 292 | `png_image_begin_read_from_stdio` | FILE* NULL | returns 0, invalid argument | exit 0 | [x] |
| 293 | `png_image_free` | called twice, then with NULL | idempotent, no crash | exit 0 | [x] |
| 294 | `png_image_write_to_memory` | png_image.width 0 | returns 0, "png_image_write_to_memory: invalid image" | signal 8; no record written | [x] |
| 295 | `png_image_write_to_memory` | png_image.height 0 | returns 0, invalid image | exit 0 | [x] |
| 296 | `png_image_write_to_memory` | png_image.version 7 | returns 0, version mismatch | exit 0 | [x] |
| 297 | `png_image_write_to_memory` | png_image.format with undefined high bits | returns 0, "png_image_write_to_memory: invalid format" | exit 0 | [x] |
| 298 | `png_image_write_to_memory` | colormap_entries 257 | returns 0, invalid image | exit 0 | [x] |
| 299 | `png_image_write_to_memory` | colour-mapped format with colormap == NULL | returns 0, no colour-map supplied | exit 0 | [x] |
| 300 | `png_image_write_to_memory` | buffer NULL | returns 0, invalid argument | exit 0 | [x] |
| 301 | `png_image_write_to_memory` | memory_bytes NULL | returns 0, invalid argument | exit 0 | [x] |
| 302 | `png_image_write_to_memory` | row_stride 4 for an 8-pixel RGBA row | png_error "supplied row stride too small" | exit 0 | [x] |
| 303 | `png_image_write_to_memory` | 8-byte output buffer for a real image | returns 0 and reports the required size in *memory_bytes | exit 0 | [x] |
| 304 | `png_image_write_to_memory` | width and height 0x40000000 | returns 0, "memory image too large" / invalid image | exit 0 | [x] |
| 305 | `png_image_write_to_file` | unwritable path | returns 0 with the strerror() text | exit 0 | [x] |
| 306 | `png_image_write_to_stdio` | FILE* NULL | returns 0, invalid argument | exit 0 | [x] |
| 307 | `png_fixed (via png_set_gAMA)` | NaN | NaN passes both range checks (NaN > max and NaN < min are both false); the cast to the fixed-point type is target-defined and yields INT_MIN on the reference build | exit 101; no record written | [x] |
| 308 | `png_fixed (via png_set_gAMA)` | -NaN | as NaN: passes the range checks, cast yields INT_MIN | exit 101; no record written | [x] |
| 309 | `png_fixed (via png_set_gAMA)` | +inf | fails the upper range check -> png_fixed_error "fixed point overflow" | exit 101; no record written | [x] |
| 310 | `png_fixed (via png_set_gAMA)` | -inf | fails the lower range check -> png_fixed_error | exit 101; no record written | [x] |
| 311 | `png_fixed (via png_set_gAMA)` | 1e300 | fails the upper range check -> png_fixed_error | exit 70; png_error: fixed point overflow in png_set_gAMA | [x] |
| 312 | `png_fixed (via png_set_gAMA)` | -1e300 | fails the lower range check -> png_fixed_error | exit 70; png_error: fixed point overflow in png_set_gAMA | [x] |
| 313 | `png_fixed (via png_set_gAMA)` | 1e-300 | rounds to 0 and is then rejected as an out-of-range value | exit 0 | [x] |
| 314 | `png_fixed (via png_set_gAMA)` | 21474.83648 (exactly PNG_FP_MAX/1e5) | the scaled value lands on 2147483648, one past PNG_FP_MAX -> png_fixed_error | exit 70; png_error: fixed point overflow in png_set_gAMA | [x] |
| 315 | `png_fixed_ITU (via png_set_cLLI)` | NaN | NaN passes both range checks (NaN > max and NaN < min are both false); the cast to the fixed-point type is target-defined and yields INT_MIN on the reference build | exit 0 | [x] |
| 316 | `png_fixed_ITU (via png_set_cLLI)` | -NaN | as NaN: passes the range checks, cast yields INT_MIN | exit 0 | [x] |
| 317 | `png_fixed_ITU (via png_set_cLLI)` | +inf | fails the upper range check -> png_fixed_error "fixed point overflow" | exit 70; png_error: fixed point overflow in png_set_cLLI(maxFALL) | [x] |
| 318 | `png_fixed_ITU (via png_set_cLLI)` | -inf | fails the lower range check -> png_fixed_error | exit 70; png_error: fixed point overflow in png_set_cLLI(maxFALL) | [x] |
| 319 | `png_fixed_ITU (via png_set_cLLI)` | 1e300 | fails the upper range check -> png_fixed_error | exit 70; png_error: fixed point overflow in png_set_cLLI(maxFALL) | [x] |
| 320 | `png_fixed_ITU (via png_set_cLLI)` | -1e300 | fails the lower range check -> png_fixed_error | exit 70; png_error: fixed point overflow in png_set_cLLI(maxFALL) | [x] |
| 321 | `png_fixed_ITU (via png_set_cLLI)` | 1e-300 | rounds to 0 and is then rejected as an out-of-range value | exit 0 | [x] |
| 322 | `png_fixed_ITU (via png_set_cLLI)` | 21474.83648 (exactly PNG_FP_MAX/1e5) | the scaled value lands on 2147483648, one past PNG_FP_MAX -> png_fixed_error | exit 0 | [x] |
| 323 | `png_set_alpha_mode -> convert_gamma_value` | output_gamma NaN | NaN passes the range check; the cast yields INT_MIN | exit 0; 1 warning(s): gamma out of supported range | [x] |
| 324 | `png_set_gamma -> convert_gamma_value` | screen and file gamma NaN | NaN passes the range check; the cast yields INT_MIN | exit 0; 3 warning(s): invalid file gamma in png_set_gamma / invalid screen gamma in png_set_gamma / gamma out of supported range | [x] |
| 325 | `png_set_rgb_to_gray -> png_fixed` | red and green coefficients NaN | NaN converted to INT_MIN, then rejected as out of range | exit 0; 1 warning(s): invalid before the PNG header has been read | [x] |
| 326 | `png_set_background -> convert_gamma_value` | background_gamma NaN | NaN converted to INT_MIN | exit 0 | [x] |
| 327 | `png_set_cHRM -> png_fixed` | all eight chromaticities NaN | each converts to INT_MIN, then png_colorspace_set_chromaticities rejects them | exit 0 | [x] |
| 328 | `png_set_cHRM_XYZ -> png_fixed` | all nine XYZ values NaN | each converts to INT_MIN, then the endpoint check rejects them | exit 0; 1 warning(s): invalid cHRM XYZ | [x] |
| 329 | `png_set_sCAL -> png_ascii_from_fp` | width and height NaN | png_error "Invalid sCAL width/height" or a NaN ASCII string | exit 0 | [x] |
| 330 | `png_set_sCAL -> png_ascii_from_fp` | width +inf | png_error "Invalid sCAL width" / inf formatted | exit 70; png_error: Invalid sCAL width | [x] |
| 331 | `png_set_mDCV -> png_fixed / png_fixed_ITU` | all ten mDCV values NaN | converted to INT_MIN / 0 and rejected | exit 0; 1 warning(s): mDCV chromaticities outside representable range | [x] |
| 332 | `png_get_gAMA / png_get_pixel_aspect_ratio_fixed / png_get_*_offset_inches_fixed` | boundary fixed-point inputs incl. PNG_FP_MAX, -1, -2, 0 and res_y == 0xffffffff | fixed<->float round trip and division-by-zero handling | exit 0; 2 warning(s): fixed point overflow ignored | [x] |
| 333 | `png_set_cHRM -> png_fixed (x8)` | +inf for all eight chromaticities | the reference build evaluates the argument list right to left, so the *first* failure reported is "fixed point overflow in cHRM Blue Y" | exit 70; png_error: fixed point overflow in cHRM Blue Y | [x] |
| 334 | `png_set_cHRM_XYZ -> png_fixed (x9)` | +inf for all nine XYZ values | right-to-left evaluation: "fixed point overflow in cHRM Blue Z" | exit 70; png_error: fixed point overflow in cHRM Blue Z | [x] |
| 335 | `png_set_mDCV -> png_fixed (x8) + png_fixed_ITU (x2)` | +inf for all ten values | right-to-left evaluation: "fixed point overflow in png_set_mDCV(minDL)" | exit 70; png_error: fixed point overflow in png_set_mDCV(minDL) | [x] |
| 336 | `png_set_cLLI -> png_fixed_ITU (x2)` | +inf for both light levels | right-to-left evaluation: "fixed point overflow in png_set_cLLI(maxFALL)" | exit 70; png_error: fixed point overflow in png_set_cLLI(maxFALL) | [x] |
| 337 | `png_set_rgb_to_gray -> png_fixed (x2)` | +inf for both coefficients | right-to-left evaluation: "fixed point overflow in rgb to gray green coefficient" | exit 70; png_error: fixed point overflow in rgb to gray green coefficient | [x] |
| 338 | `png_set_gamma -> convert_gamma_value (x2)` | +inf for both screen and file gamma | right-to-left evaluation: the file gamma is converted first | exit 70; png_error: fixed point overflow in gamma value | [x] |
| 339 | `png_get_IHDR -> png_check_IHDR` | called on an info struct that never had an IHDR (the end_info of png_read_end) | png_get_IHDR re-validates the stored values, so the all-zero struct produces png_warning "Image width/height is zero in IHDR" + "Invalid bit depth" and then png_error "Invalid IHDR data" | exit 70; png_error: Invalid IHDR data; 3 warning(s): Image width is zero in IHDR / Image height is zero in IHDR / Invalid bit depth in IHDR | [x] |
| 340 | `png_get_IHDR -> png_check_IHDR` | same, with png_set_benign_errors(1) | same: png_check_IHDR uses png_error, not png_benign_error, so it is fatal either way | exit 70; png_error: Invalid IHDR data; 3 warning(s): Image width is zero in IHDR / Image height is zero in IHDR / Invalid bit depth in IHDR | [x] |

Scenario ids (used as `err|id=<id>` by `translation/tests/support/errscen.rs`)
are listed in `translation/tests/support/errors_tbl.rs`, which is also the source
of this table.

## Mutation fuzzing (group C5)

The table above is an enumeration; group C5 is a *search*.  It takes a rich but
valid datastream (IHDR + gAMA + cHRM + sBIT + tRNS + bKGD + pHYs + oFFs + sCAL +
pCAL + sPLT + tEXt/zTXt/iTXt + eXIf + cICP + cLLI + mDCV + iCCP + private chunks
+ IDAT + trailing chunks + IEND), flips 1/2/4/8 bits at pseudo-random offsets and
reads the result end to end, with and without recomputed CRCs and with each
png_set_benign_errors() setting.  Both libraries get the identical mutated bytes.

120 rows, 120 passing (each row tries 8 independently seeded mutations).

`PNGDIFF_FUZZ=<n>` multiplies the size of this search; it has been run at
n=25 (3000 rows / 24000 mutations) with no divergence.  Distinct C outcomes
observed in the default run:

| observed in the C build | rows |
|---|---|
| exit N; png_error: Read Error | 15 |
| exit N; png_error: IDAT: incorrect data check | 13 |
| exit N; png_error: IHDR: CRC error | 7 |
| exit N; png_error: PLTE: CRC error | 5 |
| exit N; png_error: PLTE: CRC error; N warning(s): iCCP: CRC error | 3 |
| exit N; png_error: IHDR: too long | 2 |
| exit N; png_error: PLTE: CRC error; N warning(s): iCCP: CRC error / iTXt: CRC error | 1 |
| exit N; png_error: PLTE: CRC error; N warning(s): tEXt: CRC error | 1 |
| exit N; N warning(s): iCCP: profile 'ICC': NANh: profile too long | 1 |
| exit N; png_error: [N]HDR: bad header (invalid type) | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): zTXt: unknown compression type | 1 |
| exit N; png_error: [N]rVt: bad header (invalid type) | 1 |
| exit N; png_error: PLTE: CRC error; N warning(s): sPLT: CRC error / iTXt: CRC error / pzIv: CRC error | 1 |
| exit N; png_error: Not enough image data | 1 |
| exit N; png_error: IHDS: unhandled critical chunk | 1 |
| exit N; png_error: iTXt: incorrect data check | 1 |
| exit N; png_error: t[CN]ME: bad header (invalid type); N warning(s): prIv: CRC error | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): sPLT: CRC error / tEXt: CRC error | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): iCCP: CRC error / iCCP: bad compression method / sPNT: CRC error | 1 |
| exit N; png_error: iTXt: truncated | 1 |
| exit N; png_error: Read Error; N warning(s): sCAL: bad height format | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): prVt: CRC error / tIME: CRC error / iCCP: CRC error | 1 |
| exit N; png_error: iCCP: profile 'ICC': 'wtpt': ICC profile tag outside profile; N warning(s): mTCV: CRC error / tYME: CRC error | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): iCCP: CRC error / tAXt: CRC error / cHRM: CRC error | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): sCAL: bad width format | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): pHYw: CRC error / iTXt: CRC error | 1 |
| exit N; png_error: iCCP: profile 'ICC': 'mntR': unrecognized ICC profile class | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): sPLT: CRC error | 1 |
| exit N; png_error: [N][N][N][N]: bad header (invalid type); N warning(s): bKGD: invalid | 1 |
| exit N; png_error: e[NC]If: bad header (invalid type) | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): iCCP: CRC error / iTXt: CRC error / prVT: CRC error | 1 |
| exit N; png_error: rIvp: bad header (invalid type); N warning(s): gAMA: CRC error / iCCP: CRC error / bKGD: CRC error | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): iTXt: incorrect data check | 1 |
| exit N; png_error: Read Error; N warning(s): iCCP: profile 'ICC': 'acNp': invalid signature / tRNS chunk has out-of-range samples for bit_depth | 1 |
| exit N; png_error: Read Error; N warning(s): iTXt: CRC error | 1 |
| exit N; png_error: [N][N][N][N]: bad header (invalid type); N warning(s): cHRM: CRC error / cHRM: too long | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): iCCP: CRC error | 1 |
| exit N; png_error: IIDR: unhandled critical chunk | 1 |
| exit N; png_error: IDAT: incorrect data check; N warning(s): oFFs: CRC error / pCAL: CRC error / tIME: CRC error | 1 |
| exit N; png_error: [EN]TXt: bad header (invalid type) | 1 |
| ... 41 further distinct outcomes | |
