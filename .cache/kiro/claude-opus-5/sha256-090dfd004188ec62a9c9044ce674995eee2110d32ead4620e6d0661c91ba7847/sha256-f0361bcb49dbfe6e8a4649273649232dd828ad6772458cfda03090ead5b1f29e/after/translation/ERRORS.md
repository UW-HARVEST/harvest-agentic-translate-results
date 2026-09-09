# ERRORS.md — error-surface table

## Verification result

All **331** rows have a passing differential test. Reproduce with:

```sh
cd translation && cargo build --release && cargo test --release --test diff
# cases passed: 267 / 267
# CONFIGS.md rows covered: 164   ERRORS.md rows covered: 331
```

Each row is discharged by constructing that exact invalid input, driving **both**
the C `.so` and the Rust `.so` through it via their exported symbols, and
comparing the complete transcript *and* the process exit status/signal
byte-for-byte — so "both rejected it identically, with the same message" is the
literal assertion, not "both failed somehow".

Two genuine translation defects were found and fixed by this table:

1. `png_image_write_to_memory` with `image->width == 0`
   (`src/pngwrite2.rs:606`). `png_row_stride` is then `0` and the C evaluates
   `image->height > 0xffffffffU / png_row_stride`, taking the hardware divide
   fault (`SIGFPE`, exit 136). Rust's `/` panicked instead and aborted
   (`SIGABRT`, exit 134). Fixed by routing the division through the new
   `c_div_u32` helper, which issues the raw instruction and reproduces the C
   behaviour exactly. Covered by `prim/simple_zero_dims`.
2. The same defect on the read side, `png_image_finish_read`
   (`src/pngread3.rs:1456`, `0xffffffffU/PNG_IMAGE_PIXEL_COMPONENT_SIZE(fmt)/check`
   where `check == 0` for a zero-width image). Same fix.

An audit of the remaining eight variable-divisor sites in the translation
confirmed each is guarded in the C exactly as it is in the Rust, so no further
change was warranted — adding a guard the C does not have would itself be a
divergence.

---

Derived mechanically from `c_src/src/*.c` by grepping every rejection primitive:

```
grep -nE 'png_(error|chunk_error|chunk_benign_error|app_error|app_warning|benign_error|warning|chunk_warning|chunk_report|fixed_error|image_error)[[:space:]]*\(' c_src/src/*.c
grep -nE 'return *\(?(NULL|0|-1|PNG_UINT_32_MAX)' c_src/src/*.c
```

Each numbered row below is a **distinct rejection** that an external caller can
reach through an exported `png_*` symbol, i.e. it is differentially testable
across the FFI boundary. Rows are checked off only when a differential test
constructs that exact condition, calls **both** the C `.so` and the Rust `.so`,
and both produce the **same** rejection (same message text and/or same sentinel
value), not merely "both failed".

Internal-only rejections (reachable solely via a libpng logic bug, or guarded by
`#ifdef` branches that are compiled out in this `pnglibconf.h`) are listed in the
non-testable appendix at the end; they carry no checkbox because no FFI input can
reach them.

How the message text is observed: the differential harness installs an
`error_fn`/`warning_fn` pair through `png_set_error_fn`, records every message
verbatim, and (for errors) terminates the worker process with a fixed status.
Both libraries are driven through identical code, so the recorded transcripts are
compared byte-for-byte.

## Legend for "expected C result"

* `E:"msg"` — fatal: `png_error`/`png_chunk_error` with exactly that string; the
  error callback fires and control never returns.
* `B:"msg"` — `png_benign_error`/`png_chunk_benign_error`. `PNG_BENIGN_ERRORS_WARN`
  is **not** set in `pnglibconf.h`, so on a read struct these are *fatal* unless
  the app called `png_set_benign_errors(png_ptr, 1)`; with `allowed=1` they
  become warnings. Both variants are tested.
* `W:"msg"` — `png_warning`/`png_chunk_warning`/`png_app_warning`: non-fatal,
  callback fires, call continues.
* `A:"msg"` — `png_app_error`: `PNG_APP_ERRORS_WARN` is not set, so fatal.
* `-> v` — returns sentinel value `v` (no callback).
* `no-op` — silently returns without doing anything.

---

## 1. Signature, struct lifecycle, and generic argument guards

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `png_sig_cmp` | `num_to_check < 1` (e.g. `start=0, num_to_check=0`) | `-> -1` | [x] |
| 2 | `png_sig_cmp` | `start > 7` (e.g. `start=8`) | `-> -1` | [x] |
| 3 | `png_sig_cmp` | `start+num_to_check > 8` clamped, bytes differ from `\211PNG\r\n\032\n` | `-> nonzero` (memcmp result) | [x] |
| 4 | `png_set_sig_bytes` | `num_bytes > 8` (e.g. `9`) | `E:"Too many bytes for PNG signature"` | [x] |
| 5 | `png_set_sig_bytes` | `num_bytes < 0` → coerced to `0` | no-op, `sig_bytes==0` | [x] |
| 6 | `png_create_read_struct` | `user_png_ver` = `"0.0.0"` (major/minor mismatch) | `-> NULL` | [x] |
| 7 | `png_create_write_struct` | `user_png_ver` = `"0.0.0"` | `-> NULL` | [x] |
| 8 | `png_create_read_struct` | `user_png_ver = NULL` | `-> NULL` | [x] |
| 9 | `png_create_info_struct` | `png_ptr == NULL` | `-> NULL` | [x] |
| 10 | `png_get_io_ptr` | `png_ptr == NULL` | `-> NULL` | [x] |
| 11 | `png_get_error_ptr` | `png_ptr == NULL` | `-> NULL` | [x] |
| 12 | `png_set_longjmp_fn` | `png_ptr == NULL` | `-> NULL` | [x] |
| 13 | `png_set_longjmp_fn` | `jmp_buf_size != sizeof(jmp_buf)` after a first successful call | `W:"Application jmp_buf size changed"`, `-> NULL` | [x] |
| 14 | `png_data_freer` | `freer` neither `PNG_DESTROY_WILL_FREE_DATA(1)` nor `PNG_USER_WILL_FREE_DATA(2)` (e.g. `3`, `0`, `-1`) | `E:"Unknown freer parameter in png_data_freer"` | [x] |
| 15 | `png_malloc` | `png_ptr == NULL` | `-> NULL` | [x] |
| 16 | `png_malloc_warn` | allocation of `PNG_SIZE_MAX` fails | `W:"Out of memory"`, `-> NULL` | [x] |
| 17 | `png_set_read_fn` | called on a struct that already has `write_data_fn` (write struct) | `W:"Can't set both read_data_fn and write_data_fn in the same structure"` | [x] |
| 18 | `png_set_write_fn` | called on a struct that already has `read_data_fn` (read struct) | `W:"Can't set both read_data_fn and write_data_fn in the same structure"` | [x] |
| 19 | `png_convert_to_rfc1123_buffer` | `out == NULL` | `-> 0` | [x] |
| 20 | `png_convert_to_rfc1123_buffer` | `year>9999 \|\| month==0 \|\| month>12 \|\| day==0 \|\| day>31 \|\| hour>23 \|\| minute>59 \|\| second>60` (each boundary separately) | `-> 0` | [x] |
| 21 | `png_convert_to_rfc1123` | `ptime` invalid (`month=13`) | `W:"Ignoring invalid time value"`, `-> NULL` | [x] |
| 22 | `png_convert_to_rfc1123` | `png_ptr == NULL` | `-> NULL` | [x] |
| 23 | `png_get_palette_max` | `png_ptr == NULL` or `info_ptr == NULL` | `-> -1` | [x] |
| 24 | `png_get_rowbytes` / `png_get_channels` / `png_get_image_width` / `..._height` / `..._bit_depth` / `..._color_type` / `..._filter_type` / `..._interlace_type` / `..._compression_type` | `png_ptr == NULL` or `info_ptr == NULL` | `-> 0` | [x] |
| 25 | `png_get_signature` / `png_get_user_chunk_ptr` / `png_get_progressive_ptr` / `png_get_user_transform_ptr` / `png_get_mem_ptr` | `png_ptr == NULL` | `-> NULL` | [x] |
| 26 | `png_get_IHDR` | any of `width/height/bit_depth/color_type` out-param `NULL`, or `png_ptr==NULL` | `-> 0` | [x] |
| 27 | `png_get_eXIf` | always (deprecated) | `W:"png_get_eXIf does not work; use png_get_eXIf_1"`, `-> 0` | [x] |
| 28 | `png_get_valid` | `PNG_INFO_tRNS` requested but `num_trans == 0` | `-> 0` | [x] |
| 29 | `png_get_current_row_number` | `png_ptr == NULL` | `-> PNG_UINT_32_MAX` | [x] |
| 30 | `png_get_current_pass_number` | `png_ptr == NULL` | `-> 8` (invalid pass sentinel) | [x] |
| 31 | `png_get_x_pixels_per_inch` / `_y_` / `png_get_pixels_per_inch` | `pHYs` valid with `ppm > PNG_UINT_31_MAX` or `png_muldiv` overflow | `-> 0` | [x] |
| 32 | `png_get_x_offset_inches_fixed` etc. | `png_muldiv` overflow in `png_fixed_inches_from_microns` | `W:"fixed point overflow ignored"`, `-> 0` | [x] |
| 33 | `png_get_pixel_aspect_ratio` / `_fixed` | `pHYs` valid, `x_pixels_per_unit == 0` | `-> 0` | [x] |
| 34 | all `png_get_<chunk>` accessors (`gAMA`,`cHRM`,`sRGB`,`iCCP`,`sBIT`,`tRNS`,`bKGD`,`hIST`,`pHYs`,`oFFs`,`pCAL`,`sCAL`,`tIME`,`sPLT`,`text`,`PLTE`,`unknown_chunks`,`cICP`,`cLLI`,`mDCV`,`eXIf_1`) | corresponding `info_ptr->valid` bit clear | `-> 0` | [x] |
| 35 | `png_get_compression_buffer_size` / `png_get_io_state` / `png_get_io_chunk_type` | `png_ptr == NULL` | `-> 0` | [x] |
| 36 | `png_process_data_skip` | called at all | `W:"png_process_data_skip is not implemented in any current version of libpng"`, `-> 0` | [x] |

## 2. Reader: signature and chunk-stream framing

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 37 | `png_read_info` (via `png_read_sig`) | first 8 bytes are not the PNG signature and differ within the first 4 | `E:"Not a PNG file"` | [x] |
| 38 | `png_read_info` (via `png_read_sig`) | signature matches first 4 bytes but the CRLF/^Z bytes are mangled (ASCII conversion) | `E:"PNG file corrupted by ASCII conversion"` | [x] |
| 39 | `png_read_chunk_header` | chunk length high byte `>= 0x80` (length `> 0x7fffffff`) | `E:"bad header (invalid length)"` | [x] |
| 40 | `png_read_chunk_header` | chunk type contains a non-alphabetic byte (e.g. `"IH\x01R"`) | `E:"bad header (invalid type)"` | [x] |
| 41 | `png_crc_finish` on a critical chunk | CRC field does not match the computed CRC of a critical chunk (`IHDR`) | `E:"CRC error"` (via `png_chunk_error`) | [x] |
| 42 | `png_crc_finish` on an ancillary chunk | CRC mismatch on an ancillary chunk (`tEXt`) with default CRC action | `W:"CRC error"` and the chunk is dropped | [x] |
| 43 | `png_handle_chunk` (table pre-check) | any chunk other than `IHDR` appears first (`mode & PNG_HAVE_IHDR)==0`) | `E:"missing IHDR"` | [x] |
| 44 | `png_handle_chunk` (table pre-check) | chunk out of its legal position (`gAMA` after `IDAT`, `PLTE` after `IDAT`) | `B:"out of place"` (ancillary) / `E:"out of place"` (critical) | [x] |
| 45 | `png_handle_chunk` (table pre-check) | non-repeatable chunk appears twice (two `gAMA`, two `pHYs`) | `B:"duplicate"` | [x] |
| 46 | `png_handle_chunk` (table pre-check) | chunk shorter than its `min_length` (`gAMA` length 3, `cHRM` length 31, `tIME` length 6) | `B:"too short"` | [x] |
| 47 | `png_handle_chunk` (table pre-check) | chunk longer than its `max_length` (`gAMA` length 5, `sRGB` length 2, `tIME` length 8) | `B:"too long"` | [x] |
| 48 | `png_handle_chunk` (table pre-check) | `Limit`-checked chunk longer than `png_chunk_max(png_ptr)` (set via `png_set_chunk_malloc_max`) | `B:"length exceeds libpng limit"` | [x] |
| 49 | `png_read_info` | `IDAT` reached with `(mode & PNG_HAVE_IHDR) == 0` | `E:"Missing IHDR before IDAT"` | [x] |
| 50 | `png_read_info` | palette image with `IDAT` reached and no `PLTE` | `E:"Missing PLTE before IDAT"` | [x] |
| 51 | `png_read_info` | a second `IDAT` run after `PNG_AFTER_IDAT` | `B:"Too many IDATs found"` | [x] |
| 52 | `png_handle_unknown` | critical unknown chunk (e.g. `"cRIT"`, uppercase first letter) present and unhandled | `E:"unhandled critical chunk"` | [x] |
| 53 | `png_handle_unknown` | user read-chunk callback returns `< 0` | `E:"error in user chunk"` | [x] |
| 54 | `png_handle_unknown` | `png_set_keep_unknown_chunks` cache limit reached (`png_set_chunk_cache_max(2)`) while storing | `B:"no space in chunk cache"` | [x] |
| 55 | `png_cache_unknown_chunk` | unknown chunk larger than `png_set_chunk_malloc_max` limit | `B:"unknown chunk exceeds memory limits"` | [x] |
| 56 | `png_read_end` | palette image whose rows contain an index `>= num_palette` | `B:"Read palette index exceeding num_palette"` | [x] |
| 57 | `png_read_end` | extra `IDAT` bytes after the zlib stream ended | `B:".Too many IDATs found"` (leading dot is literal) | [x] |
| 58 | `png_read_end` | non-`IDAT` chunk between `IDAT`s, then more `IDAT` | `B:"..Too many IDATs found"` (two literal dots) | [x] |

## 3. Reader: IHDR validation (`png_handle_IHDR` → `png_check_IHDR`)

All of these are FFI-reachable by crafting the 13 IHDR bytes; each individual
condition emits its own `png_warning` and the chunk ends with
`E:"Invalid IHDR data"`. One row per distinct warning.

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 59 | `png_handle_IHDR` | IHDR chunk length `!= 13` | `E:"Invalid IHDR length"` (sequential) / `E:"Invalid IHDR length"` (progressive) | [x] |
| 60 | `png_check_IHDR` | `width == 0` | `W:"Image width is zero in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 61 | `png_check_IHDR` | `width > PNG_UINT_31_MAX` (`0x80000000`) | `W:"Invalid image width in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 62 | `png_check_IHDR` | `width > user_width_max` (default `1000000`), e.g. `1000001` | `W:"Image width exceeds user limit in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 63 | `png_check_IHDR` | `height == 0` | `W:"Image height is zero in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 64 | `png_check_IHDR` | `height > PNG_UINT_31_MAX` | `W:"Invalid image height in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 65 | `png_check_IHDR` | `height > user_height_max` (default `1000000`) | `W:"Image height exceeds user limit in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 66 | `png_check_IHDR` | `bit_depth` not in `{1,2,4,8,16}` (test `0,3,5,7,9,15,17,32,255`) | `W:"Invalid bit depth in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 67 | `png_check_IHDR` | `color_type` in `{1,5,7..255}` | `W:"Invalid color type in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 68 | `png_check_IHDR` | `color_type==3 && bit_depth==16`; or `color_type in {2,4,6} && bit_depth<8` | `W:"Invalid color type/bit depth combination in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 69 | `png_check_IHDR` | `interlace_type >= PNG_INTERLACE_LAST (2)`, e.g. `2`, `255` | `W:"Unknown interlace method in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 70 | `png_check_IHDR` | `compression_type != 0`, e.g. `1`, `255` | `W:"Unknown compression method in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 71 | `png_check_IHDR` | `filter_type != 0` with `PNG_HAVE_PNG_SIGNATURE` set and MNG intrapixel not permitted | `W:"Invalid filter method in IHDR"` / `W:"Unknown filter method in IHDR"` + `E:"Invalid IHDR data"` | [x] |
| 72 | `png_check_IHDR` | MNG-permitted `filter_type==64` after a PNG signature | `W:"MNG features are not allowed in a PNG datastream"` | [x] |

## 4. Reader: per-chunk content validation

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 73 | `png_handle_PLTE` | `length > 3*PNG_MAX_PALETTE_LENGTH (768)` or `length % 3 != 0` | `E:"invalid"` (palette img) / `B:"invalid"` (non-palette) | [x] |
| 74 | `png_handle_PLTE` | second `PLTE` | `E:"duplicate"` / `B:"duplicate"` | [x] |
| 75 | `png_handle_PLTE` | `PLTE` after `IDAT` | `E:"out of place"` / `B:"out of place"` | [x] |
| 76 | `png_handle_PLTE` | `PLTE` in a grayscale image (`color_type & PNG_COLOR_MASK_COLOR)==0`) | `B:"ignored in grayscale PNG"` | [x] |
| 77 | `png_handle_PLTE` | non-palette image where `tRNS` or `bKGD` already seen | `B:"out of place"` | [x] |
| 78 | `png_handle_IEND` | `IEND` length `!= 0` | `B:"invalid"` | [x] |
| 79 | `png_handle_gAMA` | gamma value `> PNG_UINT_31_MAX` | `B:"invalid"` | [x] |
| 80 | `png_handle_sBIT` | `length != truelen` for the color type (e.g. 2 bytes for RGB) | `B:"bad length"` | [x] |
| 81 | `png_handle_sBIT` | any `buf[i] == 0` or `buf[i] > sample_depth` | `B:"invalid"` | [x] |
| 82 | `png_handle_cHRM` | any of the 8 coordinates overflows `png_get_int_32_checked` (`0x80000000`) | `B:"invalid"` | [x] |
| 83 | `png_handle_sRGB` | rendering intent byte `> 3` (test `4`, `255`) | `B:"invalid"` | [x] |
| 84 | `png_handle_iCCP` | `length < LZ77Min` (11) | `B:"too short"` | [x] |
| 85 | `png_handle_iCCP` | compression-method byte `!= 0` | `B:"bad compression method"` | [x] |
| 86 | `png_handle_iCCP` | keyword length `0` or `> 79` | `B:"bad keyword"` | [x] |
| 87 | `png_handle_iCCP` | trailing bytes after the deflate stream ends | `B:"extra compressed data"` | [x] |
| 88 | `png_handle_iCCP` | deflate payload is not a valid zlib stream | `B:<zlib msg>` (`"incorrect header check"` / `"invalid stored block lengths"` ...) | [x] |
| 89 | `png_handle_iCCP` → `png_icc_check_length` | decoded profile shorter than 132 bytes | `B:"too short"` | [x] |
| 90 | `png_handle_iCCP` → `png_icc_check_header` | `png_get_uint_32(profile) != profile_length` | `B:"length does not match profile"` | [x] |
| 91 | `png_handle_iCCP` → `png_icc_check_header` | `profile[8] > 3 && (profile_length & 3) != 0` | `B:"invalid length"` | [x] |
| 92 | `png_handle_iCCP` → `png_icc_check_header` | tag count `> 357913930` or `profile_length < 132 + 12*tag_count` | `B:"tag count too large"` | [x] |
| 93 | `png_handle_iCCP` → `png_icc_check_header` | rendering intent `>= 0xffff` | `B:"invalid rendering intent"` | [x] |
| 94 | `png_handle_iCCP` → `png_icc_check_header` | rendering intent in `[4, 0xfffe]` | `W:"intent outside defined range"` | [x] |
| 95 | `png_handle_iCCP` → `png_icc_check_header` | `profile+36 != 'acsp'` | `B:"invalid signature"` | [x] |
| 96 | `png_handle_iCCP` → `png_icc_check_header` | PCS illuminant bytes `!= D50` | `W:"PCS illuminant is not D50"` | [x] |
| 97 | `png_handle_iCCP` → `png_icc_check_header` | color space `'RGB '` on a grayscale PNG | `B:"RGB color space not permitted on grayscale PNG"` | [x] |
| 98 | `png_handle_iCCP` → `png_icc_check_header` | color space `'GRAY'` on a color PNG | `B:"Gray color space not permitted on RGB PNG"` | [x] |
| 99 | `png_handle_iCCP` → `png_icc_check_header` | color space neither `'RGB '` nor `'GRAY'` | `B:"invalid ICC profile color space"` | [x] |
| 100 | `png_handle_iCCP` → `png_icc_check_header` | profile class `'abst'` | `B:"invalid embedded Abstract ICC profile"` | [x] |
| 101 | `png_handle_iCCP` → `png_icc_check_header` | profile class `'link'` | `B:"unexpected DeviceLink ICC profile class"` | [x] |
| 102 | `png_handle_iCCP` → `png_icc_check_header` | profile class `'nmcl'` | `W:"unexpected NamedColor ICC profile class"` | [x] |
| 103 | `png_handle_iCCP` → `png_icc_check_header` | profile class unrecognized | `W:"unrecognized ICC profile class"` | [x] |
| 104 | `png_handle_iCCP` → `png_icc_check_header` | PCS encoding neither `'XYZ '` nor `'Lab '` | `B:"unexpected ICC PCS encoding"` | [x] |
| 105 | `png_handle_iCCP` → `png_icc_check_tag_table` | `tag_start > profile_length` or `tag_length > profile_length-tag_start` | `B:"ICC profile tag outside profile"` | [x] |
| 106 | `png_handle_iCCP` → `png_icc_check_tag_table` | `(tag_start & 3) != 0` | `W:"ICC profile tag start not a multiple of 4"` | [x] |
| 107 | `png_handle_sPLT` | `length < 2` or the name is not NUL-terminated within the chunk | `W:"malformed sPLT chunk"` | [x] |
| 108 | `png_handle_sPLT` | `data_length % entry_size != 0` | `W:"sPLT chunk has bad length"` | [x] |
| 109 | `png_handle_sPLT` | `png_set_chunk_cache_max(1)` then an `sPLT` chunk | silently dropped, no message | [x] |
| 110 | `png_handle_sPLT` | `png_set_chunk_cache_max(2)` then an `sPLT` chunk | `W:"No space in chunk cache for sPLT"` | [x] |
| 111 | `png_handle_tRNS` | grayscale image with `length != 2` | `B:"invalid"` | [x] |
| 112 | `png_handle_tRNS` | RGB image with `length != 6` | `B:"invalid"` | [x] |
| 113 | `png_handle_tRNS` | palette image, `tRNS` before `PLTE` | `B:"out of place"` | [x] |
| 114 | `png_handle_tRNS` | palette image, `length > num_palette` or `length == 0` | `B:"invalid"` | [x] |
| 115 | `png_handle_tRNS` | color type already has an alpha channel (`4`/`6`) | `B:"invalid with alpha channel"` | [x] |
| 116 | `png_handle_bKGD` | palette image, `bKGD` before `PLTE` | `B:"out of place"` | [x] |
| 117 | `png_handle_bKGD` | `length != truelen` for the color type | `B:"invalid"` | [x] |
| 118 | `png_handle_bKGD` | palette image with `buf[0] >= num_palette` | `B:"invalid index"` | [x] |
| 119 | `png_handle_bKGD` | grayscale `bit_depth<=8` with `buf[0]!=0` or `buf[1] >= (1<<bit_depth)` | `B:"invalid gray level"` | [x] |
| 120 | `png_handle_bKGD` | RGB `bit_depth<=8` with any high byte nonzero | `B:"invalid color"` | [x] |
| 121 | `png_handle_eXIf` | TIFF header neither `0x49492A00` nor `0x4D4D002A` | `B:"invalid"` | [x] |
| 122 | `png_handle_hIST` | `length != 2*num_palette`, or `num != num_palette`, or `num > PNG_MAX_PALETTE_LENGTH` | `B:"invalid"` | [x] |
| 123 | `png_handle_pCAL` | purpose string not NUL-terminated within `length-12` (`endptr - buf <= 12`) | `B:"invalid"` | [x] |
| 124 | `png_handle_pCAL` | `nparams` wrong for the equation type | `B:"invalid parameter count"` | [x] |
| 125 | `png_handle_pCAL` | equation type `>= PNG_EQUATION_LAST (4)` | `B:"unrecognized equation type"` | [x] |
| 126 | `png_handle_pCAL` | a parameter string runs past the chunk end (`buf > endptr`) | `B:"invalid data"` | [x] |
| 127 | `png_handle_sCAL` | unit byte neither `1` nor `2` (test `0`, `3`, `255`) | `B:"invalid unit"` | [x] |
| 128 | `png_handle_sCAL` | width not a valid fp string / not NUL-terminated | `B:"bad width format"` | [x] |
| 129 | `png_handle_sCAL` | width parses but is not positive (`"0"`, `"-1"`) | `B:"non-positive width"` | [x] |
| 130 | `png_handle_sCAL` | height not a valid fp string / trailing bytes | `B:"bad height format"` | [x] |
| 131 | `png_handle_sCAL` | height parses but is not positive | `B:"non-positive height"` | [x] |
| 132 | `png_handle_tEXt` | `png_set_chunk_cache_max(1)` then `tEXt` | silently dropped | [x] |
| 133 | `png_handle_tEXt` | `png_set_chunk_cache_max(2)` then `tEXt` | `B:"no space in chunk cache"` | [x] |
| 134 | `png_handle_zTXt` | keyword length `0` or `> 79` | `B:"bad keyword"` | [x] |
| 135 | `png_handle_zTXt` | `keyword_length + 3 > length` | `B:"truncated"` | [x] |
| 136 | `png_handle_zTXt` | compression byte `!= 0` | `B:"unknown compression type"` | [x] |
| 137 | `png_handle_zTXt` | deflate payload invalid | `B:<zlib msg>` | [x] |
| 138 | `png_handle_iTXt` | keyword length `0` or `> 79` | `B:"bad keyword"` | [x] |
| 139 | `png_handle_iTXt` | `prefix_length + 5 > length` | `B:"truncated"` | [x] |
| 140 | `png_handle_iTXt` | compression flag not `0`/`1`, or compression method `!= 0` | `B:"bad compression info"` | [x] |
| 141 | `png_handle_iTXt` | compressed, `prefix_length >= length` | `B:"truncated"` | [x] |
| 142 | `png_handle_iTXt` | compressed payload invalid | `B:<zlib msg>` | [x] |
| 143 | `png_inflate_claim` | second chunk needing the zstream while it is still owned | `W:"<chunk> using zstream"` | [x] |

## 5. Reader: row / IDAT decode path

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 144 | `png_read_row` | `png_read_row` called before any `IDAT` seen | `E:"Invalid attempt to read row data"` | [x] |
| 145 | `png_read_row` | row filter byte `>= PNG_FILTER_VALUE_LAST (5)` (test `5`, `255`) | `E:"bad adaptive filter value"` | [x] |
| 146 | `png_read_IDAT_data` | stream ends before all rows are produced | `E:"Not enough image data"` | [x] |
| 147 | `png_read_IDAT_data` | non-`IDAT` chunk while more image data is still required | `E:"Not enough image data"` | [x] |
| 148 | `png_read_IDAT_data` | more inflated bytes than the image needs | `B:"Too much image data"` | [x] |
| 149 | `png_read_IDAT_data` | leftover `IDAT` bytes after `Z_STREAM_END` | `B:"Extra compressed data"` | [x] |
| 150 | `png_read_IDAT_data` | corrupt deflate data in `IDAT` | `E:<zlib msg>` via `png_chunk_error` | [x] |
| 151 | `png_read_IDAT_data` | wrong Adler-32 trailer on the `IDAT` zlib stream | `E:"incorrect data check"` | [x] |
| 152 | `png_read_start_row` | `rowbytes > 65535` (`PNG_MAX_MALLOC_64K`) — e.g. `width=70000, 8-bit RGBA` | `E:"This image requires a row greater than 64KB"` | [x] |
| 153 | `png_read_update_info` | called twice | `A:"png_read_update_info/png_start_read_image: duplicate call"` | [x] |
| 154 | `png_start_read_image` | called after `png_read_update_info` | `A:"png_start_read_image/png_read_update_info: duplicate call"` | [x] |
| 155 | `png_read_png` | `info_ptr->height > PNG_UINT_32_MAX/sizeof(png_bytep)` | `E:"Image is too high to process with png_read_png()"` | [x] |
| 156 | `png_read_image` | called on an interlaced image after `png_set_interlace_handling` was not used — advisory | `W:"Interlace handling should be turned on when using png_read_image"` | [x] |
| 157 | `png_get_int_32` (via `oFFs`) | value `0x80000000` (two's-complement negate overflow) | `-> 0` | [x] |
| 158 | `png_get_uint_31` | value `> PNG_UINT_31_MAX` in a length/dimension field | `E:"PNG unsigned integer out of range"` | [x] |

## 6. Progressive reader (`pngpread.c`)

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 159 | `png_process_data` → `png_push_read_sig` | bad signature, differs in first 4 bytes | `E:"Not a PNG file"` | [x] |
| 160 | `png_process_data` → `png_push_read_sig` | signature differs only in the trailing bytes | `E:"PNG file corrupted by ASCII conversion"` | [x] |
| 161 | `png_push_read_chunk` | `IHDR` length `!= 13` | `E:"Invalid IHDR length"` | [x] |
| 162 | `png_push_read_chunk` | `IDAT` with no `IHDR` | `E:"Missing IHDR before IDAT"` | [x] |
| 163 | `png_push_read_chunk` | palette image, `IDAT` with no `PLTE` | `E:"Missing PLTE before IDAT"` | [x] |
| 164 | `png_push_read_chunk` | `IDAT` after `PNG_AFTER_IDAT` | `B:"Too many IDATs found"` | [x] |
| 165 | `png_push_read_IDAT` | non-`IDAT` chunk while the zstream has not ended | `E:"Not enough compressed data"` | [x] |
| 166 | `png_process_IDAT_data` | inflate returns `Z_DATA_ERROR` on the final Adler-32 | `B:"IDAT: ADLER32 checksum mismatch"` | [x] |
| 167 | `png_process_IDAT_data` | inflate returns another error inside the image rows | `E:"Decompression error in IDAT"` | [x] |
| 168 | `png_process_IDAT_data` | fewer compressed bytes than needed | `W:"Truncated compressed data in IDAT"` | [x] |
| 169 | `png_process_IDAT_data` | more inflated bytes than the image needs | `W:"Extra compressed data in IDAT"` | [x] |
| 170 | `png_push_process_row` | row filter byte `>= PNG_FILTER_VALUE_LAST` | `E:"bad adaptive filter value"` | [x] |

## 7. Read-transform setters (`pngrtran.c`, `pngtrans.c`)

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 171 | any `png_set_*` read transform (`png_set_expand`, `png_set_gray_to_rgb`, `png_set_strip_alpha`, `png_set_packing`, `png_set_swap`, ...) via `png_rtran_ok` | called after `png_read_update_info`/`png_start_read_image` | `A:"invalid after png_start_read_image or png_read_update_info"` | [x] |
| 172 | any `png_set_*` read transform via `png_rtran_ok(need_IHDR)` | called before `png_read_info` on a fresh read struct (e.g. `png_set_background`) | `A:"invalid before the PNG header has been read"` | [x] |
| 173 | any `png_set_*` read transform on a **write** struct | `png_ptr->mode & PNG_IS_READ_STRUCT) == 0` | no-op (silent) | [x] |
| 174 | `png_set_alpha_mode_fixed` | `mode` out of `0..=4` (test `5`, `-1`, `1000`) | `E:"invalid alpha mode"` | [x] |
| 175 | `png_set_alpha_mode_fixed` | called after `png_set_background` (conflicting) | `E:"conflicting calls to set alpha mode and background"` | [x] |
| 176 | `png_set_alpha_mode_fixed` / `png_set_gamma_fixed` | gamma outside `[16, 625000000]` supported range | `A:"gamma out of supported range"` | [x] |
| 177 | `png_set_gamma_fixed` | `scrn_gamma <= 0` (test `0`, `-1`) | `A:"invalid screen gamma in png_set_gamma"` | [x] |
| 178 | `png_set_gamma_fixed` | `file_gamma <= 0` | `A:"invalid file gamma in png_set_gamma"` | [x] |
| 179 | `png_set_gamma` (float) | gamma value that overflows the fixed-point conversion (`1e12`) | `E:"fixed point overflow in gamma value"` | [x] |
| 180 | `png_set_background_fixed` | `background_gamma_code == PNG_BACKGROUND_GAMMA_UNKNOWN (0)` | `W:"Application must supply a known background gamma"` | [x] |
| 181 | `png_set_background_fixed` | `background_gamma_code >= PNG_BACKGROUND_GAMMA_LAST (4)` at row time | `E:"invalid background gamma type"` | [x] |
| 182 | `png_set_rgb_to_gray_fixed` | `error_action` not in `1..=3` (test `0`, `4`, `255`) | `E:"invalid error action to rgb_to_gray"` | [x] |
| 183 | `png_set_rgb_to_gray_fixed` | `red < 0 \|\| green < 0 \|\| red+green > 100000` | `W:"ignoring out of range rgb_to_gray coefficients"` | [x] |
| 184 | `png_set_rgb_to_gray` with `error_action=2` | a genuinely non-gray RGB pixel in the image | `E:"png_do_rgb_to_gray found nongray pixel"` | [x] |
| 185 | `png_set_rgb_to_gray` with `error_action=1` | a genuinely non-gray RGB pixel in the image | `W:"png_do_rgb_to_gray found nongray pixel"` | [x] |
| 186 | `png_set_crc_action` | `crit_action == PNG_CRC_QUIET_USE (4)` or `PNG_CRC_NO_CHANGE (5)` | `W:"Can't discard critical data on CRC error"` (coerced) | [x] |
| 187 | `png_set_shift` | all four shift values `<= 0` or `> bit_depth` | `A:"png_set_shift: invalid shift values"` | [x] |
| 188 | `png_set_filler` | on read: filler requested for a low-bit-depth gray output | `A:"png_set_filler is invalid for low bit depth gray output"` | [x] |
| 189 | `png_set_filler` | color type has no room for a filler (gray/palette) | `A:"png_set_filler: inappropriate color type"` | [x] |
| 190 | `png_set_user_transform_info` | called after `png_start_read_image`/`png_read_update_info` | `A:"info change after png_start_read_image or png_read_update_info"` | [x] |
| 191 | `png_set_interlace_handling` | non-interlaced image | `-> 1` (not 7) | [x] |
| 192 | `png_do_read_transformations` | user transform leaves `pixel_depth & 7 != 0` | `E:"invalid user transform pixel depth"` | [x] |
| 193 | `png_set_quantize` | `num_palette` / `maximum_colors` boundary (`0`, `1`, `256`, `257`) | must not diverge | [x] |

## 8. Writer: `png_set_*` argument validation (`pngset.c`)

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 194 | `png_set_PLTE` | palette image with `num_palette < 0` or `> PNG_MAX_PALETTE_LENGTH (256)` | `E:"Invalid palette length"` | [x] |
| 195 | `png_set_PLTE` | non-palette image with `num_palette` out of range | `W:"Invalid palette length"`, no-op | [x] |
| 196 | `png_set_PLTE` | `num_palette > 0 && palette == NULL`, or `num_palette == 0` | `E:"Invalid palette"` | [x] |
| 197 | `png_set_sCAL_s` | `unit` neither `1` nor `2` | `E:"Invalid sCAL unit"` | [x] |
| 198 | `png_set_sCAL_s` | `swidth == NULL`, empty, starts with `'-'`, or not a valid fp string | `E:"Invalid sCAL width"` | [x] |
| 199 | `png_set_sCAL_s` | `sheight` likewise | `E:"Invalid sCAL height"` | [x] |
| 200 | `png_set_sCAL` / `png_set_sCAL_fixed` | `width <= 0` | `W:"Invalid sCAL width ignored"` | [x] |
| 201 | `png_set_sCAL` / `png_set_sCAL_fixed` | `height <= 0` | `W:"Invalid sCAL height ignored"` | [x] |
| 202 | `png_set_iCCP` | `compression_type != PNG_COMPRESSION_TYPE_BASE (0)` | `A:"Invalid iCCP compression method"` | [x] |
| 203 | `png_set_iCCP` | `name == NULL` or `profile == NULL` | no-op | [x] |
| 204 | `png_set_pCAL` | `type < 0 \|\| type > 3` | `A:"Invalid pCAL equation type"` | [x] |
| 205 | `png_set_pCAL` | `nparams < 0 \|\| nparams > 255` | `A:"Invalid pCAL parameter count"` | [x] |
| 206 | `png_set_pCAL` | any `params[i] == NULL` or not a valid fp string | `A:"Invalid format for pCAL parameter"` | [x] |
| 207 | `png_set_hIST` | `num_palette == 0 \|\| num_palette > PNG_MAX_PALETTE_LENGTH` | `W:"Invalid palette size, hIST allocation skipped"` | [x] |
| 208 | `png_set_tIME` | `month==0 \|\| month>12 \|\| day==0 \|\| day>31 \|\| hour>23 \|\| minute>59 \|\| second>60` | `W:"Ignoring invalid time value"` | [x] |
| 209 | `png_set_tRNS` | grayscale/RGB with `bit_depth < 16` and a sample `> (1<<bit_depth)-1` | `W:"tRNS chunk has out-of-range samples for bit_depth"` | [x] |
| 210 | `png_set_cICP` | `matrix_coefficients != 0` | `W:"Invalid cICP matrix coefficients"`, no-op | [x] |
| 211 | `png_set_cLLI_fixed` | `maxCLL > 0x7FFFFFFF` or `maxFALL > 0x7FFFFFFF` | `A:"cLLI light level exceeds PNG limit"` | [x] |
| 212 | `png_set_mDCV_fixed` | any chromaticity outside `[0, 65535]` after ITU 16-bit scaling | `A:"mDCV chromaticities outside representable range"` | [x] |
| 213 | `png_set_cHRM_XYZ_fixed` | XYZ that `png_xy_from_XYZ` rejects (all zero) | `A:"invalid cHRM XYZ"` | [x] |
| 214 | `png_set_eXIf` | called at all (deprecated) | `W:"png_set_eXIf does not work; use png_set_eXIf_1"` | [x] |
| 215 | `png_set_eXIf_1` | `exif == NULL`, or `PNG_WROTE_eXIf` already set | no-op | [x] |
| 216 | `png_set_text_2` (via `png_set_text`) | `num_text <= 0` or `text_ptr == NULL` | `-> 0`, no-op | [x] |
| 217 | `png_set_text` | keyword `NULL` or empty (`png_check_keyword` → 0) | text entry dropped (`text_ptr->key` zeroed) | [x] |
| 218 | `png_set_text` → `png_check_keyword` | keyword longer than 79 chars | `W:"keyword truncated"` | [x] |
| 219 | `png_set_text` → `png_check_keyword` | keyword containing a control character (`0x01`) | formatted `W:"keyword \"...\": bad character '0x01'"` | [x] |
| 220 | `png_set_unknown_chunks` | `num_unknowns <= 0` or `unknowns == NULL` | no-op | [x] |
| 221 | `png_set_unknown_chunks` on a read struct | `location == 0` | `E:"invalid location in png_set_unknown_chunks"` | [x] |
| 222 | `png_set_unknown_chunk_location` | `location` has none of `PNG_HAVE_IHDR\|PNG_HAVE_PLTE\|PNG_AFTER_IDAT` (e.g. `0`, `0x40`) | `A:"invalid unknown chunk location"` | [x] |
| 223 | `png_set_keep_unknown_chunks` | `keep < 0 \|\| keep >= PNG_HANDLE_CHUNK_LAST (5)` (test `-1`, `5`, `255`) | `A:"png_set_keep_unknown_chunks: invalid keep"` | [x] |
| 224 | `png_set_keep_unknown_chunks` | `num_chunks_in > 0 && chunk_list == NULL` | `A:"png_set_keep_unknown_chunks: no chunk list"` | [x] |
| 225 | `png_set_keep_unknown_chunks` | `num_chunks + old_num_chunks > UINT_MAX/5` | `A:"png_set_keep_unknown_chunks: too many chunks"` | [x] |
| 226 | `png_set_compression_buffer_size` | `size == 0` or `size > PNG_UINT_31_MAX` | `E:"invalid compression buffer size"` | [x] |
| 227 | `png_set_compression_buffer_size` (write) | `size < 6` | `W:"Compression buffer size cannot be reduced below 6"`, no-op | [x] |
| 228 | `png_set_compression_buffer_size` (write) | `size > ZLIB_IO_MAX` | `W:"Compression buffer size limited to system maximum"`, clamped | [x] |

## 9. Writer: chunk emission validation (`pngwutil.c`, `pngwrite.c`)

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 229 | `png_write_IHDR` | `color_type==GRAY` with `bit_depth` not in `{1,2,4,8,16}` | `E:"Invalid bit depth for grayscale image"` | [x] |
| 230 | `png_write_IHDR` | `color_type==RGB` with `bit_depth` not `8`/`16` | `E:"Invalid bit depth for RGB image"` | [x] |
| 231 | `png_write_IHDR` | `color_type==PALETTE` with `bit_depth` not in `{1,2,4,8}` | `E:"Invalid bit depth for paletted image"` | [x] |
| 232 | `png_write_IHDR` | `color_type==GRAY_ALPHA` with `bit_depth` not `8`/`16` | `E:"Invalid bit depth for grayscale+alpha image"` | [x] |
| 233 | `png_write_IHDR` | `color_type==RGBA` with `bit_depth` not `8`/`16` | `E:"Invalid bit depth for RGBA image"` | [x] |
| 234 | `png_write_IHDR` | `color_type` in `{1,5,7,...}` | `E:"Invalid image color type specified"` | [x] |
| 235 | `png_write_IHDR` | `compression_type != 0` | `W:"Invalid compression type specified"`, coerced to `0` | [x] |
| 236 | `png_write_IHDR` | `filter_type != 0` (non-MNG) | `W:"Invalid filter type specified"`, coerced | [x] |
| 237 | `png_write_IHDR` | `interlace_type > 1` | `W:"Invalid interlace type specified"`, coerced to `1` | [x] |
| 238 | `png_write_info` | palette image with no `PLTE` set | `E:"Valid palette required for paletted images"` | [x] |
| 239 | `png_write_PLTE` | `num_pal == 0 \|\| num_pal > max` for a palette image | `E:"Invalid number of colors in palette"` | [x] |
| 240 | `png_write_PLTE` | same, for a non-palette image | `W:"Invalid number of colors in palette"`, chunk skipped | [x] |
| 241 | `png_write_PLTE` | `PLTE` on a grayscale image | `W:"Ignoring request to write a PLTE chunk in grayscale PNG"` | [x] |
| 242 | `png_write_row` | called before `png_write_info` | `E:"png_write_info was never called before png_write_row"` | [x] |
| 243 | `png_write_end` | no `IDAT` was ever written | `E:"No IDATs written into file"` | [x] |
| 244 | `png_write_end` | palette row data contained an index `>= num_palette` | `B:"Wrote palette index exceeding num_palette"` | [x] |
| 245 | `png_write_complete_chunk` | chunk length `> PNG_UINT_31_MAX` | `E:"length exceeds PNG maximum"` | [x] |
| 246 | `png_set_filter` | `filters` value not a valid mask for method 0 (e.g. `7`) | `A:"Unknown row filter for method 0"` | [x] |
| 247 | `png_set_filter` | `method != PNG_FILTER_TYPE_BASE (0)` | `E:"Unknown custom filter method"` | [x] |
| 248 | `png_set_filter` | `UP`/`AVG`/`PAETH` requested after row writing has begun | `W:"Unable to initialize row filter" family` (`png_app_warning`) | [x] |
| 249 | `png_set_compression_window_bits` | `window_bits > 15` | `W:"Only compression windows <= 32k supported by PNG"`, clamped | [x] |
| 250 | `png_set_compression_window_bits` | `window_bits < 8` | `W:"Only compression windows >= 256 supported by PNG"`, clamped | [x] |
| 251 | `png_set_compression_method` | `method != 8` | `W:"Only compression method 8 is supported by PNG"`, coerced | [x] |
| 252 | `png_set_text_compression_window_bits` / `_method` | same three conditions on the text zstream | same three messages | [x] |
| 253 | `png_write_iCCP` | profile shorter than 132 bytes | `E:"ICC profile too short"` | [x] |
| 254 | `png_write_iCCP` | `png_get_uint_32(profile) != profile_len` | `E:"Profile length does not match profile"` | [x] |
| 255 | `png_write_iCCP` | profile length not a multiple of 4 | `E:"ICC profile length invalid (not a multiple of 4)"` | [x] |
| 256 | `png_write_iCCP` | keyword rejected by `png_check_keyword` | `E:"iCCP: invalid keyword"` | [x] |
| 257 | `png_write_tEXt` | keyword rejected | `E:"tEXt: invalid keyword"` | [x] |
| 258 | `png_write_zTXt` | `compression != PNG_TEXT_COMPRESSION_zTXt` | `E:"zTXt: invalid compression type"` | [x] |
| 259 | `png_write_zTXt` | keyword rejected | `E:"zTXt: invalid keyword"` | [x] |
| 260 | `png_write_iTXt` | keyword rejected | `E:"iTXt: invalid keyword"` | [x] |
| 261 | `png_write_iTXt` | `compression` not one of the iTXt values | `E:"iTXt: invalid compression"` | [x] |
| 262 | `png_write_sPLT` | name rejected | `E:"sPLT: invalid keyword"` | [x] |
| 263 | `png_write_pCAL` | `type >= PNG_EQUATION_LAST` reaching the writer | `E:"Unrecognized equation type for pCAL chunk"` | [x] |
| 264 | `png_write_pCAL` | purpose keyword rejected | `E:"pCAL: invalid keyword"` | [x] |
| 265 | `png_write_sBIT` | any `sig_bit` component `0` or `> bit_depth` | `W:"Invalid sBIT depth specified"`, chunk skipped | [x] |
| 266 | `png_write_tRNS` | `num_trans` out of range / wrong color type | `W:"Can't write tRNS with an alpha channel"` and 3 sibling `png_app_warning`s | [x] |
| 267 | `png_write_bKGD` | palette index `>= num_palette` | `W:"Invalid background palette index"` | [x] |
| 268 | `png_write_bKGD` | 8-bit gray/RGB `bKGD` value out of range | `W:"Ignoring attempt to write 16-bit bKGD chunk when bit_depth is 8 or less"` family | [x] |
| 269 | `png_write_hIST` | `num_hist > num_palette` | `W:"Invalid number of histogram entries specified"` | [x] |
| 270 | `png_write_oFFs` | `unit_type >= PNG_OFFSET_LAST` | `W:"Unrecognized unit type for oFFs chunk"` | [x] |
| 271 | `png_write_pHYs` | `unit_type >= PNG_RESOLUTION_LAST` | `W:"Unrecognized unit type for pHYs chunk"` | [x] |
| 272 | `png_write_tIME` | invalid time fields reaching the writer | `W:"Invalid time specified for tIME chunk"` | [x] |
| 273 | `png_write_sCAL_s` | formatted string longer than the internal buffer | `W:"Can't write sCAL (buffer too small)"` | [x] |
| 274 | `png_write_unknown_chunks` | zero-length unknown chunk | `W:"Writing zero-length unknown chunk"` | [x] |
| 275 | `png_write_png` | `info_ptr->row_pointers == NULL` | `A:"no rows for png_write_image to write"` | [x] |
| 276 | `png_write_png` | `PNG_TRANSFORM_STRIP_FILLER_BEFORE\|_AFTER` both set | `A:"PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported"` | [x] |
| 277 | `png_write_row` | write transform changes the pixel depth unexpectedly | `E:"internal write transform logic error"` | [x] |

## 10. Simplified API (`png_image_*`)

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 278 | `png_image_begin_read_from_memory` | `memory == NULL` or `size == 0` | `-> 0`, `image.message == "png_image_begin_read_from_memory: invalid argument"` | [x] |
| 279 | `png_image_begin_read_from_memory` | `image->version != PNG_IMAGE_VERSION (1)` | `-> 0`, `"png_image_begin_read_from_memory: incorrect PNG_IMAGE_VERSION"` | [x] |
| 280 | `png_image_begin_read_from_memory` | `image == NULL` | `-> 0` | [x] |
| 281 | `png_image_begin_read_from_memory` | `image->opaque != NULL` on entry | `-> 0`, `"png_image_read: opaque pointer not NULL"` | [x] |
| 282 | `png_image_begin_read_from_memory` | truncated PNG (read past end of buffer) | `-> 0`, `"read beyond end of data"` | [x] |
| 283 | `png_image_begin_read_from_stdio` | `file == NULL` | `-> 0`, `"png_image_begin_read_from_stdio: invalid argument"` | [x] |
| 284 | `png_image_begin_read_from_stdio` | wrong `version` | `-> 0`, `"png_image_begin_read_from_stdio: incorrect PNG_IMAGE_VERSION"` | [x] |
| 285 | `png_image_begin_read_from_file` | `file_name == NULL` | `-> 0`, `"png_image_begin_read_from_file: invalid argument"` | [x] |
| 286 | `png_image_begin_read_from_file` | nonexistent path | `-> 0`, `image.message == strerror(ENOENT)` (`"No such file or directory"`) | [x] |
| 287 | `png_image_begin_read_from_file` | wrong `version` | `-> 0`, `"png_image_begin_read_from_file: incorrect PNG_IMAGE_VERSION"` | [x] |
| 288 | `png_image_finish_read` | `buffer == NULL` or `row_stride` too small | `-> 0`, `"png_image_finish_read: invalid argument"` | [x] |
| 289 | `png_image_finish_read` | colormapped format with `colormap == NULL` or `colormap_entries <= 0` | `-> 0`, `"png_image_finish_read[color-map]: no color-map"` | [x] |
| 290 | `png_image_finish_read` | `image->height` so large the buffer size overflows | `-> 0`, `"png_image_finish_read: image too large"` | [x] |
| 291 | `png_image_finish_read` | `image->width > 0x7fffffff/channels` | `-> 0`, `"png_image_finish_read: row_stride too large"` | [x] |
| 292 | `png_image_finish_read` | `image->version` corrupted between calls | `-> 0`, `"png_image_finish_read: damaged PNG_IMAGE_VERSION"` | [x] |
| 293 | `png_image_finish_read` | `image == NULL` | `-> 0` | [x] |
| 294 | `png_image_finish_read` | alpha removal requested with `background == NULL` on a non-linear colormap | `-> 0`, `"background color must be supplied to remove alpha/transparency"` | [x] |
| 295 | `png_image_finish_read` | `image->format` with undefined bits set (e.g. `0xff`) | must not diverge | [x] |
| 296 | `png_image_write_to_memory` | `memory_bytes == NULL` / `image == NULL` / bad `version` | `-> 0`, matching `image.message` | [x] |
| 297 | `png_image_write_to_memory` | supplied buffer smaller than the encoded PNG | `-> 0`, `"png_image_write_to_memory: PNG too big"`, `*memory_bytes` set to required size | [x] |
| 298 | `png_image_write_to_file` | unwritable path (`/proc/x/y`) | `-> 0`, `image.message == strerror(errno)` | [x] |
| 299 | `png_image_write_to_stdio` | `file == NULL` | `-> 0`, matching `image.message` | [x] |
| 300 | `png_image_write_*` | `row_stride` smaller than the row needs | `-> 0`, `"supplied row stride too small"` | [x] |
| 301 | `png_image_write_*` | colormapped image with `colormap == NULL` | `-> 0`, `"no color-map for color-mapped image"` | [x] |
| 302 | `png_image_write_*` | `image->width == 0` or `image->height == 0` | must not diverge | [x] |
| 303 | `png_image_free` | `image == NULL`, or called twice | no-op, no crash, `image.opaque == NULL` | [x] |

## 11. Out-of-range enum values crossing the FFI boundary

C enums accept any `int`. Every public entry point taking an enum-like parameter
is called with a value that has no valid variant; C and Rust must agree.

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 304 | `png_set_filter` | `method = -1, 1, 2, 99, INT_MAX, INT_MIN` | `E:"Unknown custom filter method"` for all `!= 0` | [x] |
| 305 | `png_set_filter` | `filters = -2, 6, 7, 9, 0x100, INT_MAX` | `A:"Unknown row filter for method 0"` where the mask is invalid | [x] |
| 306 | `png_set_alpha_mode_fixed` | `mode = -1, 5, 6, INT_MAX, INT_MIN` | `E:"invalid alpha mode"` | [x] |
| 307 | `png_set_rgb_to_gray_fixed` | `error_action = -1, 0, 4, 99, INT_MAX` | `E:"invalid error action to rgb_to_gray"` | [x] |
| 308 | `png_set_background_fixed` | `background_gamma_code = -1, 4, 5, INT_MAX` | `E:"invalid background gamma type"` at row time | [x] |
| 309 | `png_set_crc_action` | `crit_action`/`ancil_action` = `-1, 6, 7, INT_MAX` | coerced; C and Rust must produce the same `flags` behaviour | [x] |
| 310 | `png_set_keep_unknown_chunks` | `keep = -1, 5, 6, INT_MAX, INT_MIN` | `A:"png_set_keep_unknown_chunks: invalid keep"` | [x] |
| 311 | `png_set_unknown_chunk_location` | `location = -1, 0, 0x40, 0x7f, INT_MAX` | `A:"invalid unknown chunk location"` where no valid bit is set | [x] |
| 312 | `png_data_freer` | `freer = -1, 0, 3, INT_MAX`; `mask = 0, -1, 0xffff` | `E:"Unknown freer parameter in png_data_freer"` for invalid `freer` | [x] |
| 313 | `png_free_data` | `mask = -1, 0, INT_MAX`; `num = -1, 0, 9999` | no crash, identical resulting `info_ptr` state | [x] |
| 314 | `png_set_IHDR` (write) | `color_type = -1, 1, 5, 7, 255`; `bit_depth = 0, 3, 7, 32`; `interlace_type = -1, 2, 255`; `compression_type = 1`; `filter_type = 1` | matching `png_error`/`png_warning` per rows 229–237 | [x] |
| 315 | `png_set_option` | `option = -1, PNG_OPTION_NEXT, 99`; `onoff = -1, 3, 99` | `-> PNG_OPTION_INVALID (0)` for out-of-range `option` | [x] |
| 316 | `png_set_sCAL_s` | `unit = -1, 0, 3, INT_MAX` | `E:"Invalid sCAL unit"` | [x] |
| 317 | `png_set_pCAL` | `type = -1, 4, 255, INT_MAX` | `A:"Invalid pCAL equation type"` | [x] |
| 318 | `png_set_text` | `text_ptr->compression = -3, 3, 99, INT_MAX` | writer rejects with the matching `zTXt`/`iTXt` message | [x] |
| 319 | `png_set_gAMA_fixed` / `png_set_sRGB` | `intent = -1, 4, 255`; `file_gamma = 0, -1, INT_MAX` | matching rejection/coercion | [x] |
| 320 | `png_set_compression_level` / `_mem_level` / `_strategy` | out-of-range zlib values (`level = -2, 10`; `strategy = -1, 5`) | identical downstream zlib error or coercion | [x] |
| 321 | `png_set_filler` | `flags = -1, 2, 99` (`PNG_FILLER_LAST`) | identical behaviour | [x] |
| 322 | `png_set_packswap`/`png_set_swap`/... on a write struct after `png_write_info` | ordering violation | identical (silent or warning) | [x] |

## 12. Generic pointer / length boundaries

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 323 | every exported `png_*(png_structp, ...)` | `png_ptr == NULL` | no crash; the documented sentinel or silent return | [x] |
| 324 | every exported `png_*(png_structp, png_infop, ...)` | `info_ptr == NULL` | no crash; documented sentinel | [x] |
| 325 | `png_read_row` / `png_read_rows` | both `row` and `display_row` `NULL` | no crash, row consumed | [x] |
| 326 | `png_write_row` | `row == NULL` | identical behaviour in both | [x] |
| 327 | `png_process_data` | `buffer == NULL, size == 0` | no-op, identical | [x] |
| 328 | `png_read_info` on a 0-byte stream | `read_fn` immediately reports EOF (`E:"Read Error"` from the default handler) | identical | [x] |
| 329 | `png_read_info` on a truncated stream | EOF mid-chunk | identical error | [x] |
| 330 | `png_set_chunk_malloc_max` / `png_set_chunk_cache_max` / `png_set_user_limits` | `0` and `PNG_UINT_31_MAX` boundaries | identical limit behaviour | [x] |
| 331 | `png_get_uint_32` / `png_get_uint_16` / `png_get_int_32` / `png_save_uint_32` / `png_save_int_32` / `png_save_uint_16` | all byte patterns incl. `0x00000000`, `0x7fffffff`, `0x80000000`, `0xffffffff` | byte-identical results | [x] |

---

## Appendix — rejections NOT reachable through the FFI boundary

These sites exist in the C but cannot be triggered by any input an external
caller can supply, because they guard internal invariants or live in `#ifdef`
branches that are compiled out by this `pnglibconf.h`. They carry no checkbox.

* `png.c`: `png_zalloc` overflow warning; `png_user_version_check` mismatch
  warning; `png_set_rgb_coefficients` `"internal error handling cHRM
  coefficients"`; `png_ascii_from_fp`/`_fixed` `"ASCII conversion buffer too
  small"`; the `png_pow10`/`png_muldiv`/`png_reciprocal`/`png_product2`/
  `png_reciprocal2`/`png_log8bit`/`png_log16bit`/`png_exp` overflow sentinels;
  `png_image_free_function` `"simplified read/write not supported"`.
* `pngmem.c`: `png_malloc_base` 64K / `PNG_SIZE_MAX` guards;
  `png_malloc_array` / `png_realloc_array` `"internal error: array alloc"` /
  `"internal error: array realloc"`.
* `pngread.c`: all `png_image_read_colormap` `"... color-map: too few
  entries"` variants, `"color map overflow (BAD internal error)"`,
  `"bad data option (internal error)"`, `"bad processing option (internal
  error)"`, `"bad background index (internal error)"`,
  `"bad encoding (internal error)"`, `"unexpected encoding (internal error)"`,
  `"color-map index out of range"`, `"unknown interlace type"`,
  `"lost rgb to gray"`, `"unexpected compose"`, `"lost/gained channels"`,
  `"unexpected 8-bit transformation"`, `"unexpected bit depth"`,
  `"png_read_image: unsupported transformation"`,
  `"png_image_read: alpha channel lost"`,
  `"unexpected alpha swap transformation"`,
  `"png_read_image: invalid transformations"`,
  `"internal sequential row size calculation error"`,
  `"bad color-map processing (internal error)"`, `"internal: default gamma not
  set"`, `"invalid memory read"`.
* `pngpread.c`: `"No IDAT data (internal error)"`, `"save_buffer error"`,
  `"internal progressive row size calculation error"`,
  `"Potential overflow of save_buffer"`, `"Insufficient memory for
  save_buffer"`.
* `pngrutil.c`: `"internal row logic error"`, `"internal row size calculation
  error"`, `"internal row width error"`, `"Row has too many bytes to allocate
  in memory"`, all `"out of memory"` / `"Insufficient memory ..."` allocation
  failures (not inducible without a failing allocator).
* `pngerror.c`: `"Libpng jmp_buf still allocated"`, `png_longjmp`'s
  `PNG_ABORT()`, `"bad longjmp: "`, `png_fixed_error` for internal call sites.
* `pngwrite.c` / `pngread.c`: every `"PNG_TRANSFORM_* not supported"` and
  `"PNG_WRITE_*_SUPPORTED is not defined"` branch — all the corresponding
  feature macros ARE defined in this `pnglibconf.h`, so these `#else` arms are
  not compiled.
* `pngwutil.c`: `"No profile for iCCP chunk"` (internal), `"Z_OK on Z_FINISH
  with output space"`, `"error writing ancillary chunked compressed data"`,
  `"compressed data too long"`, `deflateEnd` failure warning.
* `pngrtran.c`: `"NULL row buffer"`, `"Uninitialized row"`,
  `"Palette is NULL in indexed image"`, `"png_do_encode_alpha: unexpected
  call"`, `"libpng does not support gamma+background+rgb_to_gray"`.
