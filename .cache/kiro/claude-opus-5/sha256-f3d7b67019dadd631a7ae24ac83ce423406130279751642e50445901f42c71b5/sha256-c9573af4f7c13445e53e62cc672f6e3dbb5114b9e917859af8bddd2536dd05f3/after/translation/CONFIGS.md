# CONFIGS.md — CONFIGURATION-SURFACE TABLE

Valid-input axes the C code actually branches on, derived from `c_src/include/png.h`
(public API + option constants), `c_src/include/pnglibconf.h` (the 201 `PNG_*_SUPPORTED`
switches this build enables), `c_src/include/pngpriv.h` (the low-level internal entry
points that are nevertheless exported by the `.so`), and the `if`/`switch` branches in
`c_src/src/*.c`.

Axes enumerated:

* **entry-point level** — low-level exported internals (`png_do_*`, `png_read_filter_row`,
  `png_muldiv`, `png_check_IHDR`, `png_icc_check_*`, …), the normal row-at-a-time
  API, the whole-image API (`png_read_png`/`png_write_png`), the progressive
  (push) reader (`png_process_data`), and the simplified API (`png_image_*`).
* **image shape** — the 15 legal (colour type, bit depth) pairs, interlace
  none/Adam7, width/height 1 / small / odd / wide, palette sizes 1…256.
* **write options** — filter mask (6 values), zlib level/strategy/window-bits/mem-level,
  separate text-compression settings, flush interval, every write transform,
  every ancillary chunk setter, MNG intrapixel filtering.
* **read options** — every read transform, CRC action (6), user limits,
  unknown-chunk keep modes (4 × global/per-chunk), user chunk callback return
  (-1/0/+1), benign-errors on/off, `png_set_option` (3 software options × on/off),
  interlace handling on/off, progressive chunking granularity.
* **byte order / packing** — `png_set_swap`, `png_set_packswap`, `png_set_bgr`,
  `png_set_swap_alpha`, `png_set_invert_alpha`, filler before/after.

Legend for the check column: `[x]` = row passes against randomized inputs with a
fixed seed; every row is driven through **both** `.so` exports.

Total rows: **198** — all checked off.

## Verification status

Every row is driven through the exported symbols of BOTH shared objects and the
results (output bytes, return values, warning texts, error texts, row callbacks,
decoded rows) are compared for equality; see `common::diff`.  Randomized rows use
the xorshift64* PRNG in `common::Rng` with a per-test fixed seed, so every run is
reproducible.  Streams for the read-side rows are produced ONCE (by the C writer)
and fed byte-identically to both readers, so a divergence there is a decoding
difference rather than an encoding one.

Run everything with:

```
bash tools/verify_all.sh          # symbol parity + every feature combination
cd translation && cargo test --release -- --test-threads=1
```

139 test functions across 13 binaries; 0 failures.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so there is exactly
one Rust build configuration and `--no-default-features` / `--all-features` are
equivalent to the default.  `tools/verify_all.sh` step 4 enumerates the declared
features mechanically and would loop over the power set if any existed; it reports
the single configuration and runs the full suite for it.  The C side is likewise a
single configuration: `c_src/include/pnglibconf.h` is a fixed, prebuilt config
header (201 `PNG_*_SUPPORTED` switches) and `c_src/CMakeLists.txt` defines no
options.  The 59 `ERRORS.md` rows whose message literal is absent from the C
`.so`'s string table are the branches those switches compile out.

## Inputs deliberately excluded because the reference C has undefined behaviour

The C is ground truth, so where the C's own behaviour on an input is undefined
there is no result to compare against and the input is excluded.  Each exclusion
is recorded at the point of use in the test sources with the mechanical evidence;
they are collected here:

| # | input | why the C behaviour is undefined |
|---|-------|----------------------------------|
| 1 | `png_convert_to_rfc1123_buffer(buf, NULL)` | checks `out == NULL` but dereferences `ptime` unconditionally |
| 2 | `png_write_row(png, NULL)` | `memcpy`s from the row pointer with no NULL check |
| 3 | `png_write_image(png, NULL)` | iterates the row-pointer array with no NULL check |
| 4 | `png_read_image(png, NULL)` | same, on the read side |
| 5 | `png_image_write_to_memory` with `width == 0` | evaluates `image->height > PNG_UINT_31_MAX / (width*channels)` — integer division by zero (SIGFPE) |
| 6 | `png_image` with a non-NULL but bogus `opaque` | rejected via `png_image_error`, which itself calls `png_image_free` and dereferences the bogus pointer |
| 7 | `png_set_quantize` with `num_palette < 0`, `num_palette > 256`, or `maximum_colors < 1` | `memcpy(..., (unsigned)num_palette * sizeof(png_color))` into a fixed 256-entry buffer; and the reduction loop grows `max_d` without bound past the 769-entry `hash` table |
| 8 | `png_write_sPLT` with `nentries < 0` | `entries + nentries` forms a pointer before the object and `entry_size * (size_t)nentries` wraps |
| 9 | `png_process_data_pause(save=0)` called from the ROW callback | `png_push_read_IDAT` subtracts from the `buffer_size` that pause just zeroed; the unsigned wrap makes `while (png_ptr->buffer_size)` never terminate (the C `.so` does not return) |
| 10 | writing or reading without `png_set_{write,read}_fn`/`png_init_io` | `png_create_*_struct` installs `png_default_*_data` (PNG_STDIO_SUPPORTED), which then uses a NULL `FILE*` |
| 11 | `png_write_chunk_start(NULL, NULL, 0)` | `PNG_CHUNK_FROM_STRING(chunk_string)` is evaluated at the call site, before `png_write_chunk_header`'s NULL check; see the note in `tests/j_nullargs.rs`, and the comparable case (valid `png_ptr`, NULL name) IS tested and matches |
| 12 | a row stride or chunk length that is ACCEPTED but larger than the buffer supplied | libpng only rejects strides below the minimum and lengths above `PNG_UINT_31_MAX`; a larger accepted value is a promise by the caller about the buffer size |
| 13 | `png_set_user_transform_info` with `depth * channels > 64` on an interlaced image | `png_do_read_interlace` does `png_byte v[8]; memcpy(v, sp, pixel_depth >> 3)` — the C's own comment asserts "pixel_depth does not exceed 64", an invariant `png_set_user_transform_info` does not enforce |

## Group L — low-level exported entry points (no `png_struct` state, or raw row buffers)

| # | entry point(s) | configuration (options set + input shape) | [x] covered by |
|---|----------------|--------------------------------------------|----------------|
| L1 | `png_access_version_number`, `png_get_copyright`, `png_get_header_ver`, `png_get_header_version`, `png_get_libpng_ver` | no state; NULL `png_ptr` | [x] `tests/b_lowlevel.rs::version_strings_match` |
| L2 | `png_sig_cmp` | 8-byte random buffers; `start` 0..9 × `num_to_check` 0..9, plus the true signature | [x] `tests/b_lowlevel.rs::l2_png_sig_cmp` |
| L3 | `png_get_uint_32` | 4 random bytes, 4096 samples | [x] `tests/b_lowlevel.rs::l3_l7_int_functions` |
| L4 | `png_get_uint_16` | 2 random bytes, 4096 samples | [x] `tests/b_lowlevel.rs::l3_l7_int_functions` |
| L5 | `png_get_int_32` | 4 random bytes incl. MSB set (two's-complement branch), 4096 samples | [x] `tests/b_lowlevel.rs::l3_l7_int_functions` |
| L6 | `png_get_uint_31` | 4 random bytes, MSB clear and MSB set (error branch), with live `png_ptr` | [x] `tests/b_lowlevel.rs::l6_png_get_uint_31` |
| L7 | `png_save_uint_32`, `png_save_uint_16`, `png_save_int_32` | random `u32`/`u16`/`i32`, 4096 samples | [x] `tests/b_lowlevel.rs::l3_l7_int_functions` |
| L8 | `png_build_grayscale_palette` | bit_depth 1, 2, 4, 8 (each `switch` arm) | [x] `tests/b_lowlevel.rs::l8_build_grayscale_palette` |
| L9 | `png_convert_from_time_t` + `png_convert_to_rfc1123_buffer` | random `time_t` in 1970..2100 | [x] `tests/b_lowlevel.rs::l9_l11_time_conversion` |
| L10 | `png_convert_from_struct_tm` + `png_convert_to_rfc1123_buffer` | random `struct tm` fields | [x] `tests/b_lowlevel.rs::l9_l11_time_conversion` |
| L11 | `png_convert_to_rfc1123` (deprecated, uses `png_struct` buffer) | random `png_time` | [x] `tests/b_lowlevel.rs::l11_convert_to_rfc1123_deprecated` |
| L12 | `png_muldiv` | random `a` × `times` × `div` incl. 0, ±1, `PNG_FP_MAX`, `INT32_MIN` | [x] `tests/b_lowlevel.rs::l12_png_muldiv` |
| L13 | `png_reciprocal` | random `png_fixed_point` incl. 0 and extremes | [x] `tests/b_lowlevel.rs::l13_l17_reciprocal_and_gamma` |
| L14 | `png_reciprocal2` | random pairs incl. 0 | [x] `tests/b_lowlevel.rs::l13_l17_reciprocal_and_gamma` |
| L15 | `png_gamma_significant` | random fixed-point gammas around the threshold | [x] `tests/b_lowlevel.rs::l13_l17_reciprocal_and_gamma` |
| L16 | `png_gamma_8bit_correct` | value 0..255 × random gamma | [x] `tests/b_lowlevel.rs::l13_l17_reciprocal_and_gamma` |
| L17 | `png_gamma_16bit_correct` | value 0..65535 × random gamma | [x] `tests/b_lowlevel.rs::l13_l17_reciprocal_and_gamma` |
| L18 | `png_gamma_correct` | live `png_ptr`, 8- and 16-bit values × random gamma | [x] `tests/b_lowlevel.rs::l18_png_gamma_correct` |
| L19 | `png_XYZ_from_xy` | random `png_xy` chromaticities incl. degenerate | [x] `tests/b_lowlevel.rs::l19_l21_xyz_conversion` |
| L20 | `png_xy_from_XYZ` | random `png_XYZ` incl. zero/negative | [x] `tests/b_lowlevel.rs::l19_l21_xyz_conversion` |
| L21 | `png_XYZ_from_xy` ∘ `png_xy_from_XYZ` | round trip on random inputs | [x] `tests/b_lowlevel.rs::l19_l21_xyz_conversion` |
| L22 | `png_check_fp_number` | random ASCII strings, `state` seeded 0 and random | [x] `tests/b_lowlevel.rs::l22_l23_fp_number_checks` |
| L23 | `png_check_fp_string` | random ASCII strings, all lengths 0..24 | [x] `tests/b_lowlevel.rs::l22_l23_fp_number_checks` |
| L24 | `png_safecat` | random buffer size × pos × string | [x] `tests/b_lowlevel.rs::l24_l25_safecat_and_format_number` |
| L25 | `png_format_number` | every `format` (`PNG_NUMBER_FORMAT_*` 1,2,3,4,5) × random number | [x] `tests/b_lowlevel.rs::l24_l25_safecat_and_format_number` |
| L26 | `png_ascii_from_fp` | live `png_ptr`, random doubles × precision 1..15 | [x] `tests/b_lowlevel.rs::l26_l29_ascii_and_fixed` |
| L27 | `png_ascii_from_fixed` | live `png_ptr`, random `png_fixed_point` | [x] `tests/b_lowlevel.rs::l26_l29_ascii_and_fixed` |
| L28 | `png_fixed` | live `png_ptr`, random in-range doubles | [x] `tests/b_lowlevel.rs::l26_l29_ascii_and_fixed` |
| L29 | `png_fixed_ITU` | live `png_ptr`, random in-range doubles | [x] `tests/b_lowlevel.rs::l26_l29_ascii_and_fixed` |
| L30 | `png_check_keyword` | random keywords: leading/trailing/multiple spaces, control chars, >79 chars, empty | [x] `tests/b_lowlevel.rs::l30_check_keyword` |
| L31 | `png_calculate_crc` / `png_reset_crc` | live `png_ptr`, random buffers 0..4096 bytes | [x] `tests/b_lowlevel.rs::l31_crc` |
| L32 | `png_do_bgr` | all 15 (colour type, bit depth) row shapes × random row bytes | [x] `tests/b_lowlevel.rs::l32_l35_row_transforms` |
| L33 | `png_do_invert` | all 15 row shapes × random row bytes | [x] `tests/b_lowlevel.rs::l32_l35_row_transforms` |
| L34 | `png_do_swap` | all 15 row shapes × random row bytes | [x] `tests/b_lowlevel.rs::l32_l35_row_transforms` |
| L35 | `png_do_packswap` | bit depths 1, 2, 4, 8, 16 × random row bytes | [x] `tests/b_lowlevel.rs::l32_l35_row_transforms` |
| L36 | `png_do_strip_channel` | `at_start` 0 and 1 × channel counts 2, 3, 4 × 8/16-bit | [x] `tests/b_lowlevel.rs::l32_l35_row_transforms` |
| L37 | `png_do_write_interlace` | pass 0..6 × all 15 row shapes | [x] `tests/b_lowlevel.rs::l37_l38_interlace` |
| L38 | `png_do_read_interlace` | pass 0..6 × all 15 row shapes × `transformations` 0 and `PNG_PACKSWAP` | [x] `tests/b_lowlevel.rs::l37_l38_interlace` |
| L39 | `png_read_filter_row` | filter 0..4 × pixel depth 1..64 × random `row`/`prev_row` | [x] `tests/b_lowlevel.rs::l39_read_filter_row` |
| L40 | `png_check_IHDR` | all 15 legal combos × interlace 0/1 × compression 0 × filter 0/64 | [x] `tests/b_lowlevel.rs::l40_check_ihdr + l_errors3.rs::e1_check_ihdr_architecture_limits` |
| L41 | `png_icc_check_length` | profile length 0, 131, 132, 133, random, 1<<28 | [x] `tests/b_lowlevel.rs::l41_icc_check_length` |
| L42 | `png_icc_check_header` | synthetic 132-byte headers: random tag counts, signatures, colour spaces, PCS | [x] `tests/b_lowlevel.rs::l42_l43_icc_check_header_and_tags` |
| L43 | `png_icc_check_tag_table` | synthetic tag tables: 0/1/many tags, unaligned offsets, overlapping | [x] `tests/b_lowlevel.rs::l42_l43_icc_check_header_and_tags` |
| L44 | `png_sRGB_table`, `png_sRGB_base`, `png_sRGB_delta` | full array contents compared | [x] `tests/b_lowlevel.rs::l44_srgb_tables` |
| L45 | `png_zstream_error` | `ret` = Z_OK/Z_STREAM_END/Z_NEED_DICT/Z_ERRNO/Z_STREAM_ERROR/Z_DATA_ERROR/Z_MEM_ERROR/Z_BUF_ERROR/Z_VERSION_ERROR and out-of-range | [x] `tests/b_lowlevel.rs::l45_zstream_error` |
| L46 | `png_malloc`, `png_calloc`, `png_malloc_warn`, `png_malloc_base`, `png_free` | sizes 0, 1, 1000, `PNG_SIZE_MAX`, `png_ptr->user_chunk_malloc_max` boundary | [x] `tests/b_lowlevel.rs::l46_l47_allocation + l_errors3.rs::e10_malloc_default` |
| L47 | `png_malloc_array`, `png_realloc_array` | element counts 0/1/many × element sizes, overflow boundary | [x] `tests/b_lowlevel.rs::l46_l47_allocation + k_errors2.rs::d7_size_limits_and_transform_combinations` |
| L48 | `png_create_png_struct` / `png_destroy_png_struct` | matching / mismatched `user_png_ver` | [x] `tests/b_lowlevel.rs::l48_l49_create_struct_and_version_check` |
| L49 | `png_user_version_check` | exact version, same major/minor, wrong major, wrong minor, garbage | [x] `tests/b_lowlevel.rs::l48_l49_create_struct_and_version_check` |
| L50 | `png_chunk_unknown_handling` / `png_handle_as_unknown` | every `keep` × known and unknown chunk names | [x] `tests/b_lowlevel.rs::l50_chunk_unknown_handling` |

## Group W — write path, row-at-a-time (`png_write_info` → `png_write_row` → `png_write_end`)

| # | entry point(s) | configuration (options set + input shape) | [x] covered by |
|---|----------------|--------------------------------------------|----------------|
| W1 | `png_set_IHDR`+`png_write_info`+`png_write_row`+`png_write_end` | GRAY bit depth 1, non-interlaced, widths 1/7/8/9/33 | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W2 | as W1 | GRAY 2, non-interlaced, random sizes | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W3 | as W1 | GRAY 4, non-interlaced, random sizes | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W4 | as W1 | GRAY 8, non-interlaced, random sizes | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W5 | as W1 | GRAY 16, non-interlaced, random sizes | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W6 | as W1 + `png_set_PLTE` | PALETTE 1, palette size 2 | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W7 | as W1 + `png_set_PLTE` | PALETTE 2, palette size 4 | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W8 | as W1 + `png_set_PLTE` | PALETTE 4, palette size 16 | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W9 | as W1 + `png_set_PLTE` | PALETTE 8, palette sizes 1, 2, 17, 256 | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W10 | as W1 | RGB 8, non-interlaced, random sizes | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W11 | as W1 | RGB 16, non-interlaced, random sizes | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W12 | as W1 | GRAY_ALPHA 8, non-interlaced | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W13 | as W1 | GRAY_ALPHA 16, non-interlaced | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W14 | as W1 | RGB_ALPHA 8, non-interlaced | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W15 | as W1 | RGB_ALPHA 16, non-interlaced | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W16 | as W1..W15 + `png_set_interlace_handling` | every legal (colour type, bit depth) with `PNG_INTERLACE_ADAM7` | [x] `tests/c_write.rs::w1_w16_all_legal_shapes` |
| W17 | `png_write_rows` | RGB 8, `num_rows` 1 / all / partial batches | [x] `tests/c_write.rs::w17_w18_write_rows_and_image` |
| W18 | `png_write_image` | every legal combo, non-interlaced and Adam7 | [x] `tests/c_write.rs::w17_w18_write_rows_and_image` |
| W19 | `png_set_filter` | `PNG_NO_FILTERS`, `NONE`, `SUB`, `UP`, `AVG`, `PAETH`, `FAST_FILTERS`, `ALL_FILTERS` × RGB8 / GRAY1 / RGBA16 | [x] `tests/c_write.rs::w19_filters` |
| W20 | `png_set_compression_level` | 0, 1, 3, 6, 9 | [x] `tests/c_write.rs::w20_w25_zlib_parameters` |
| W21 | `png_set_compression_strategy` | 0 (DEFAULT), 1 (FILTERED), 2 (HUFFMAN_ONLY), 3 (RLE), 4 (FIXED) | [x] `tests/c_write.rs::w20_w25_zlib_parameters` |
| W22 | `png_set_compression_window_bits` | 8, 9, 10, 11, 12, 13, 14, 15 (incl. the OPTIMIZE_CMF path) | [x] `tests/c_write.rs::w20_w25_zlib_parameters` |
| W23 | `png_set_compression_mem_level` | 1, 5, 8, 9 | [x] `tests/c_write.rs::w20_w25_zlib_parameters` |
| W24 | `png_set_compression_method` | 8 | [x] `tests/c_write.rs::w20_w25_zlib_parameters` |
| W25 | `png_set_compression_buffer_size` | 1, 2, 8, 1024, 8192, 65536 (forces multi-IDAT split) | [x] `tests/c_write.rs::w20_w25_zlib_parameters` |
| W26 | `png_set_text_compression_level` / `_strategy` / `_window_bits` / `_mem_level` / `_method` | each × a `zTXt` payload large enough to compress | [x] `tests/c_write.rs::w26_text_compression_parameters` |
| W27 | `png_set_flush` + `png_write_flush` | `nrows` 0, 1, 2, 5 with 8 rows written | [x] `tests/c_write.rs::w27_flush` |
| W28 | `png_set_bgr` | RGB 8/16, RGBA 8/16 | [x] `tests/c_write.rs::w28_w37_write_transforms` |
| W29 | `png_set_swap` | 16-bit GRAY / RGB / GA / RGBA | [x] `tests/c_write.rs::w28_w37_write_transforms` |
| W30 | `png_set_packing` | GRAY 1/2/4, PALETTE 1/2/4 (input one byte per pixel) | [x] `tests/c_write.rs::w28_w37_write_transforms` |
| W31 | `png_set_packswap` | GRAY 1/2/4 and PALETTE 1/2/4 | [x] `tests/c_write.rs::w28_w37_write_transforms` |
| W32 | `png_set_invert_mono` | GRAY 1/2/4/8/16 | [x] `tests/c_write.rs::w28_w37_write_transforms` |
| W33 | `png_set_shift` | `png_set_sBIT` + shift, GRAY 8/16, RGB 8/16, RGBA 8/16 | [x] `tests/c_write.rs::w28_w37_write_transforms` |
| W34 | `png_set_swap_alpha` | GA 8/16, RGBA 8/16 | [x] `tests/c_write.rs::w28_w37_write_transforms` |
| W35 | `png_set_invert_alpha` | GA 8/16, RGBA 8/16 | [x] `tests/c_write.rs::w28_w37_write_transforms` |
| W36 | `png_set_filler` (`PNG_FILLER_BEFORE`/`AFTER`) | GRAY 8/16 → 2 channels, RGB 8/16 → 4 channels (write = strip filler) | [x] `tests/c_write.rs::w28_w37_write_transforms` |
| W37 | `png_set_add_alpha` (write side = filler) | GRAY 8, RGB 8 | [x] `tests/c_write.rs::w28_w37_write_transforms` |
| W38 | `png_set_write_user_transform_fn` + `png_set_user_transform_info` | callback that mutates the row; also `png_get_current_row_number` / `_pass_number` inside it | [x] `tests/c_write.rs::w38_w39_user_transform_and_status` |
| W39 | `png_set_write_status_fn` | row callback ordering for interlaced and non-interlaced | [x] `tests/c_write.rs::w38_w39_user_transform_and_status` |
| W40 | `png_permit_mng_features` + filter method `PNG_INTRAPIXEL_DIFFERENCING` | RGB 8/16, RGBA 8/16 | [x] `tests/c_write.rs::w40_mng_features` |
| W41 | `png_write_sig` / `png_write_chunk` / `png_write_chunk_start` / `_data` / `_end` | raw chunk emission, 0-length and multi-part data | [x] `tests/c_write.rs::w41_raw_chunk_api + l_errors3.rs::e3_chunk_length_maximum` |
| W42 | `png_set_check_for_invalid_index` + `png_get_palette_max` | PALETTE 8 rows with in-range and out-of-range indexes, `allowed` 0/1/-1 | [x] `tests/c_write.rs::w42_invalid_index_check` |
| W43 | `png_set_option(PNG_MAXIMUM_INFLATE_WINDOW)` on write struct | on / off | [x] `tests/c_write.rs::w43_set_option` |
| W44 | `png_write_info_before_PLTE` then `png_write_info` | split info write, with and without PLTE | [x] `tests/c_write.rs::w44_write_info_before_plte` |
| W45 | `png_set_rows` + `png_write_png` | every write transform bit: IDENTITY, PACKING, PACKSWAP, INVERT_MONO, SHIFT, BGR, SWAP_ALPHA, SWAP_ENDIAN, INVERT_ALPHA, STRIP_FILLER_BEFORE, STRIP_FILLER_AFTER | [x] `tests/c_write.rs::w45_write_png_transforms` |

## Group WC — write path, ancillary chunk setters (each × 2 image shapes)

| # | entry point(s) | configuration (options set + input shape) | [x] covered by |
|---|----------------|--------------------------------------------|----------------|
| WC1 | `png_set_gAMA` / `png_set_gAMA_fixed` | random gammas incl. 0, 1, `PNG_FP_MAX`, > 21474.83 | [x] `tests/d_chunks.rs::wc1_gama` |
| WC2 | `png_set_cHRM` / `png_set_cHRM_fixed` | random chromaticities, valid and degenerate | [x] `tests/d_chunks.rs::wc2_wc3_chrm` |
| WC3 | `png_set_cHRM_XYZ` / `png_set_cHRM_XYZ_fixed` | random XYZ | [x] `tests/d_chunks.rs::wc2_wc3_chrm` |
| WC4 | `png_set_sRGB` | intent 0, 1, 2, 3 | [x] `tests/d_chunks.rs::wc4_wc5_srgb` |
| WC5 | `png_set_sRGB_gAMA_and_cHRM` | intent 0..3 (writes sRGB+gAMA+cHRM) | [x] `tests/d_chunks.rs::wc4_wc5_srgb` |
| WC6 | `png_set_iCCP` | profile lengths 132, 133, 4096; name lengths 1, 79; compression 0 | [x] `tests/d_chunks.rs::wc6_iccp + l_errors3.rs::e4_write_iccp_lengths` |
| WC7 | `png_set_sBIT` | every colour type with legal significant-bit values | [x] `tests/d_chunks.rs::wc7_sbit` |
| WC8 | `png_set_bKGD` | GRAY (gray field), RGB (rgb fields), PALETTE (index field), 8- and 16-bit | [x] `tests/d_chunks.rs::wc8_bkgd` |
| WC9 | `png_set_hIST` | PALETTE with 2, 16, 256 entries | [x] `tests/d_chunks.rs::wc9_hist` |
| WC10 | `png_set_tRNS` | PALETTE (`trans_alpha`, 1..256 entries), GRAY (`trans_color.gray`), RGB (`trans_color` rgb) | [x] `tests/d_chunks.rs::wc10_trns` |
| WC11 | `png_set_pHYs` | unit 0 and 1 × random resolutions | [x] `tests/d_chunks.rs::wc11_wc12_phys_offs` |
| WC12 | `png_set_oFFs` | unit 0 and 1 × random signed offsets incl. `INT32_MIN` | [x] `tests/d_chunks.rs::wc11_wc12_phys_offs` |
| WC13 | `png_set_pCAL` | equation type 0..3 × nparams 0..3 × random ASCII params | [x] `tests/d_chunks.rs::wc13_pcal` |
| WC14 | `png_set_sCAL` / `png_set_sCAL_fixed` / `png_set_sCAL_s` | unit 1 and 2 × random positive values / decimal strings | [x] `tests/d_chunks.rs::wc14_scal` |
| WC15 | `png_set_tIME` | random `png_time` | [x] `tests/d_chunks.rs::wc15_time` |
| WC16 | `png_set_sPLT` | 1, 2 and 3 palettes × depth 8 and 16 × 1..64 entries | [x] `tests/d_chunks.rs::wc16_splt` |
| WC17 | `png_set_text` (`tEXt`, compression -1) | 1..4 items, key lengths 1/79, text 0/1/long | [x] `tests/d_chunks.rs::wc17_wc20_text` |
| WC18 | `png_set_text` (`zTXt`, compression 0) | long compressible text | [x] `tests/d_chunks.rs::wc17_wc20_text` |
| WC19 | `png_set_text` (`iTXt`, compression 1 = uncompressed) | with/without `lang`, `lang_key` | [x] `tests/d_chunks.rs::wc17_wc20_text` |
| WC20 | `png_set_text` (`iTXt`, compression 2 = compressed) | long text + lang fields | [x] `tests/d_chunks.rs::wc17_wc20_text` |
| WC21 | `png_set_eXIf_1` / `png_set_eXIf` | payloads starting `II*\0`, `MM\0*`, and other; lengths 4..64 | [x] `tests/d_chunks.rs::wc21_exif` |
| WC22 | `png_set_cICP` | random primaries / transfer / matrix / full-range bytes | [x] `tests/d_chunks.rs::wc22_wc24_pngv3_chunks` |
| WC23 | `png_set_cLLI` / `png_set_cLLI_fixed` | random levels incl. 0 and 0x7fffffff | [x] `tests/d_chunks.rs::wc22_wc24_pngv3_chunks` |
| WC24 | `png_set_mDCV` / `png_set_mDCV_fixed` | random chromaticities + luminance | [x] `tests/d_chunks.rs::wc22_wc24_pngv3_chunks` |
| WC25 | `png_set_unknown_chunks` + `png_set_unknown_chunk_location` | location `HAVE_IHDR`, `HAVE_PLTE`, `AFTER_IDAT`; safe-to-copy and critical names; 0-length data | [x] `tests/d_chunks.rs::wc25_wc26_unknown_chunks` |
| WC26 | `png_set_keep_unknown_chunks` on write | `keep` `AS_DEFAULT`/`NEVER`/`IF_SAFE`/`ALWAYS` × `num_chunks` 0, >0, <0 | [x] `tests/d_chunks.rs::wc25_wc26_unknown_chunks` |
| WC27 | `png_set_invalid` + `png_free_data` + `png_data_freer` | every `PNG_INFO_*` mask bit, `freer` = `DESTROY_WILL_FREE`/`USER_WILL_FREE` | [x] `tests/d_chunks.rs::wc27_invalid_free_data` |
| WC28 | all `png_get_*` accessors after all the setters above | full read-back of every chunk into `info_ptr`, fixed and floating variants | [x] `tests/d_chunks.rs::wc28_ihdr_accessors` |

## Group R — read path (sequential)

| # | entry point(s) | configuration (options set + input shape) | [x] covered by |
|---|----------------|--------------------------------------------|----------------|
| R1 | `png_read_info` + `png_read_row` + `png_read_end` | every legal (colour type, bit depth), non-interlaced | [x] `tests/e_read.rs::r1_r2_all_legal_shapes` |
| R2 | as R1 + `png_set_interlace_handling` | every legal combo, Adam7 | [x] `tests/e_read.rs::r1_r2_all_legal_shapes` |
| R3 | Adam7 stream read *without* `png_set_interlace_handling` (7 passes driven by the app) | RGB 8, GRAY 1 | [x] `tests/e_read.rs::r3_manual_interlace` |
| R4 | `png_read_image` | every legal combo, non-interlaced and Adam7 | [x] `tests/e_read.rs::r4_r5_read_image_and_rows` |
| R5 | `png_read_rows` with `row` only / `display_row` only / both | RGB 8 Adam7 | [x] `tests/e_read.rs::r4_r5_read_image_and_rows` |
| R6 | `png_read_update_info` after transforms | RGB 8 + expand/strip/gray_to_rgb | [x] `tests/e_read.rs::r11_r29_read_transforms (read_rows_session always calls png_read_update_info after the transforms)` |
| R7 | `png_start_read_image` then rows | PALETTE 8 with tRNS | [x] `tests/e_read.rs::r7_r8_start_read_and_sig_bytes` |
| R8 | `png_set_sig_bytes` | 0, 1, 4, 8 pre-consumed signature bytes | [x] `tests/e_read.rs::r7_r8_start_read_and_sig_bytes` |
| R9 | `png_set_crc_action` | (crit, ancil) ∈ {DEFAULT, ERROR_QUIT, WARN_DISCARD, WARN_USE, QUIET_USE, NO_CHANGE}² on a good stream | [x] `tests/e_read.rs::r9_crc_actions` |
| R10 | `png_set_user_limits` + `png_set_chunk_cache_max` + `png_set_chunk_malloc_max` | limits above and exactly at the image size | [x] `tests/e_read.rs::r10_user_limits` |
| R11 | `png_set_expand` | PALETTE 1/2/4/8 (+PLTE), GRAY 1/2/4 , GRAY+tRNS, RGB+tRNS | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R12 | `png_set_palette_to_rgb` | PALETTE 1/2/4/8 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R13 | `png_set_expand_gray_1_2_4_to_8` | GRAY 1, 2, 4 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R14 | `png_set_tRNS_to_alpha` | PALETTE+tRNS, GRAY+tRNS, RGB+tRNS | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R15 | `png_set_expand_16` | GRAY 1/2/4/8, PALETTE 8, RGB 8, RGBA 8 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R16 | `png_set_gray_to_rgb` | GRAY 1/2/4/8/16, GA 8/16 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R17 | `png_set_rgb_to_gray` / `_fixed` | error action 1/2/3 × coefficients default and random × RGB 8/16, RGBA 8/16, PALETTE 8 | [x] `tests/e_read.rs::r17_rgb_to_gray` |
| R18 | `png_set_strip_alpha` | GA 8/16, RGBA 8/16 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R19 | `png_set_strip_16` | 16-bit GRAY/RGB/GA/RGBA | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R20 | `png_set_scale_16` | 16-bit GRAY/RGB/GA/RGBA | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R21 | `png_set_packing` | GRAY 1/2/4, PALETTE 1/2/4 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R22 | `png_set_packswap` | GRAY 1/2/4, PALETTE 1/2/4 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R23 | `png_set_swap` | 16-bit rows | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R24 | `png_set_bgr` | RGB 8/16, RGBA 8/16 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R25 | `png_set_swap_alpha` | GA 8/16, RGBA 8/16 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R26 | `png_set_invert_alpha` | GA 8/16, RGBA 8/16 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R27 | `png_set_invert_mono` | GRAY 1/2/4/8/16 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R28 | `png_set_filler` / `png_set_add_alpha` | BEFORE and AFTER × GRAY 8/16, RGB 8/16 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R29 | `png_set_shift` | sBIT-bearing streams, GRAY 8/16, RGB 8/16, RGBA 8/16 | [x] `tests/e_read.rs::r11_r29_read_transforms` |
| R30 | `png_set_gamma` / `png_set_gamma_fixed` | screen gamma 1.0 / 2.2 / 0.45455 / `PNG_DEFAULT_sRGB` / `PNG_GAMMA_MAC_18` × file gamma present and absent × 8- and 16-bit | [x] `tests/e_read.rs::r30_gamma` |
| R31 | `png_set_background` / `_fixed` | gamma code UNKNOWN/SCREEN/FILE/UNIQUE × `need_expand` 0/1 × GRAY/RGB/PALETTE/alpha inputs | [x] `tests/e_read.rs::r31_background` |
| R32 | `png_set_alpha_mode` / `_fixed` | mode PNG_ALPHA_PNG/STANDARD/OPTIMIZED/BROKEN × gamma `PNG_DEFAULT_sRGB`/1.0/2.2 | [x] `tests/e_read.rs::r32_alpha_mode` |
| R33 | `png_set_quantize` | `maximum_colors` 2/16/256 × `full_quantize` 0/1 × with and without histogram × PALETTE and RGB inputs | [x] `tests/e_read.rs::r33_quantize` |
| R34 | `png_set_read_user_transform_fn` + `png_set_user_transform_info` | callback mutating rows; `png_get_current_row_number`/`_pass_number`/`png_get_user_transform_ptr` inside | [x] `tests/e_read.rs::r34_r35_read_callbacks + l_errors3.rs::e16_user_transform_pixel_depth` |
| R35 | `png_set_read_status_fn` | row callback ordering, interlaced and not | [x] `tests/e_read.rs::r34_r35_read_callbacks + l_errors3.rs::e16_user_transform_pixel_depth` |
| R36 | `png_set_read_user_chunk_fn` | callback returning -1, 0, +1 on unknown chunks | [x] `tests/e_read.rs::r36_user_chunk_callback + k_errors2.rs::d5_crafted_chunk_handlers` |
| R37 | `png_set_keep_unknown_chunks` (read) + `png_get_unknown_chunks` | `keep` AS_DEFAULT/NEVER/IF_SAFE/ALWAYS × global (`num_chunks`=0), per-chunk (>0), all-known (<0) × safe-to-copy and critical unknown chunks | [x] `tests/e_read.rs::r37_keep_unknown_on_read` |
| R38 | `png_set_benign_errors` | allowed 0 and 1 on a stream with a recoverable defect | [x] `tests/e_read.rs::r38_r39_benign_and_options` |
| R39 | `png_set_option` | `PNG_MAXIMUM_INFLATE_WINDOW`, `PNG_SKIP_sRGB_CHECK_PROFILE`, `PNG_IGNORE_ADLER32` × `on`/`off`, plus reading back with `png_get_...`/return value | [x] `tests/e_read.rs::r38_r39_benign_and_options` |
| R40 | `png_read_png` | every read transform bit: IDENTITY, STRIP_16, STRIP_ALPHA, PACKING, PACKSWAP, EXPAND, INVERT_MONO, SHIFT, BGR, SWAP_ALPHA, SWAP_ENDIAN, INVERT_ALPHA, GRAY_TO_RGB, EXPAND_16, SCALE_16 | [x] `tests/e_read.rs::r40_read_png_transforms` |
| R41 | `png_get_io_state` / `png_get_io_chunk_type` sampled from the read callback | RGB 8 stream with several ancillary chunks | [x] `tests/e_read.rs::r41_io_state` |
| R42 | `png_get_rowbytes`, `png_get_channels`, `png_get_*` easy-access and INCH-conversion accessors after `png_read_info` | streams carrying pHYs, oFFs, sCAL | [x] `tests/e_read.rs::r41_io_state + r43_all_ancillary_chunks + d_chunks.rs::wc11_wc12_phys_offs` |
| R43 | every ancillary chunk parsed from a real stream (`gAMA cHRM sRGB iCCP sBIT bKGD hIST tRNS pHYs oFFs pCAL sCAL tIME tEXt zTXt iTXt sPLT eXIf cICP cLLI mDCV`) + `png_get_*` read-back | round trip through the writer | [x] `tests/e_read.rs::r43_all_ancillary_chunks` |
| R44 | `png_reset_zstream` | after a completed read | [x] `tests/e_read.rs::r43_all_ancillary_chunks (calls png_reset_zstream after the read)` |
| R45 | chunk ordering variants | ancillary chunks before PLTE, between PLTE and IDAT, and after IDAT | [x] `tests/e_read.rs::r45_r47_stream_shapes` |
| R46 | multiple IDAT chunks of varying size | compression buffer 1/8/8192 on the write side | [x] `tests/e_read.rs::r45_r47_stream_shapes` |
| R47 | zero-length IDAT and zero-length ancillary chunks | crafted streams | [x] `tests/e_read.rs::r45_r47_stream_shapes` |

## Group P — progressive (push) reader

| # | entry point(s) | configuration (options set + input shape) | [x] covered by |
|---|----------------|--------------------------------------------|----------------|
| P1 | `png_set_progressive_read_fn` + `png_process_data` | feed granularity 1, 2, 3, 7, 13, 64, whole-stream × every legal (colour type, bit depth) | [x] `tests/f_progressive.rs::p1_p2_progressive_all_shapes` |
| P2 | as P1, Adam7 | interlaced streams, `png_progressive_combine_row` in the row callback | [x] `tests/f_progressive.rs::p1_p2_progressive_all_shapes` |
| P3 | `png_process_data_pause(save=0)` and `(save=1)` | pause after the info callback and after each row | [x] `tests/f_progressive.rs::p3_pause_and_resume` |
| P4 | `png_process_data_skip` | after pausing, honouring the returned skip count | [x] `tests/f_progressive.rs::p4_process_data_skip` |
| P5 | `png_get_progressive_ptr` | non-NULL user pointer round trip | [x] `tests/f_progressive.rs::p1_p2_progressive_all_shapes (logs png_get_progressive_ptr)` |
| P6 | progressive read with read transforms | EXPAND, GRAY_TO_RGB, STRIP_16, PACKING applied before `png_read_update_info` in the info callback | [x] `tests/f_progressive.rs::p6_progressive_transforms` |
| P7 | progressive read of a stream with all ancillary chunks | chunk callbacks + unknown chunk handling | [x] `tests/f_progressive.rs::p7_progressive_with_chunks + l_errors3.rs::e7_progressive_idat_damage` |

## Group S — simplified API

| # | entry point(s) | configuration (options set + input shape) | [x] covered by |
|---|----------------|--------------------------------------------|----------------|
| S1 | `png_image_write_to_memory` (size query, `memory` = NULL) | every `format` in {GRAY, GA, AG, RGB, BGR, RGBA, ARGB, BGRA, ABGR} × `convert_to_8_bit` 0/1 | [x] `tests/g_simplified.rs::s1_s6_write_to_memory_all_formats` |
| S2 | `png_image_write_to_memory` (real write) | as S1, `row_stride` = 0 (auto) | [x] `tests/g_simplified.rs::s1_s6_write_to_memory_all_formats` |
| S3 | `png_image_write_to_memory` | linear formats {LINEAR_Y, LINEAR_Y_ALPHA, LINEAR_RGB, LINEAR_RGB_ALPHA} × `convert_to_8_bit` 0/1 | [x] `tests/g_simplified.rs::s1_s6_write_to_memory_all_formats` |
| S4 | `png_image_write_to_memory` | colour-mapped formats (`…_COLORMAP`) × `colormap_entries` 1, 2, 16, 256 | [x] `tests/g_simplified.rs::s1_s6_write_to_memory_all_formats` |
| S5 | `png_image_write_to_memory` | `row_stride` positive > minimum, and negative (bottom-up) | [x] `tests/g_simplified.rs::s5_row_stride_variants` |
| S6 | `png_image_write_to_memory` | `flags` 0, `PNG_IMAGE_FLAG_FAST`, `PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB` | [x] `tests/g_simplified.rs::s1_s6_write_to_memory_all_formats` |
| S7 | `png_image_write_to_memory` | buffer exactly the required size, and one byte too small | [x] `tests/g_simplified.rs::s7_output_buffer_sizes + k_errors2.rs::d6_simplified_format_rejections` |
| S8 | `png_image_begin_read_from_memory` + `png_image_finish_read` | streams from every legal (colour type, bit depth); output format left as reported | [x] `tests/g_simplified.rs::s8_read_native_format` |
| S9 | as S8 with the output `format` overridden | every non-colormap format × `background` NULL and non-NULL | [x] `tests/g_simplified.rs::s9_s12_read_with_format_override` |
| S10 | as S8 with a colour-mapped output format | `colormap` buffer supplied; `colormap_entries` updated | [x] `tests/g_simplified.rs::s9_s12_read_with_format_override + l_errors3.rs::e13_simplified_colormap_matrix` |
| S11 | as S8 with `PNG_IMAGE_FLAG_16BIT_sRGB` | 16-bit input without gAMA/sRGB | [x] `tests/g_simplified.rs::s11_16bit_srgb_flag` |
| S12 | `png_image_finish_read` with negative `row_stride` | bottom-up output | [x] `tests/g_simplified.rs::s9_s12_read_with_format_override` |
| S13 | `png_image_free` | called after success, after failure, and twice | [x] `tests/g_simplified.rs::s8_read_native_format (frees twice) + i_errors.rs::c11_simplified_api_errors` |

## Group RT — write→read round trips (composed pipeline)

| # | entry point(s) | configuration (options set + input shape) | [x] covered by |
|---|----------------|--------------------------------------------|----------------|
| RT1 | full write session → full read session | every legal (colour type, bit depth) × interlace 0/1, randomized pixels, checking that the *read-back rows* match between C and Rust | [x] `tests/h_roundtrip.rs::rt1_rt4_write_read_roundtrip` |
| RT2 | write with random filter mask + zlib settings → read | 32 randomized configurations | [x] `tests/h_roundtrip.rs::rt2_random_write_configs` |
| RT3 | write with every ancillary chunk → read + `png_get_*` | one composite stream | [x] `tests/e_read.rs::r43_all_ancillary_chunks` |
| RT4 | write with C, read with Rust and vice versa (cross-implementation) | every legal (colour type, bit depth) — proves stream compatibility, not just self-consistency | [x] `tests/h_roundtrip.rs::rt1_rt4_write_read_roundtrip (cross-implementation assertion)` |
| RT5 | `png_write_png` → `png_read_png` with matching transform masks | 16 randomized transform pairs | [x] `tests/h_roundtrip.rs::rt5_write_png_read_png` |
| RT6 | simplified write → simplified read | every format | [x] `tests/h_roundtrip.rs::rt6_rt7_simplified_roundtrip` |
| RT7 | simplified write → low-level read, and low-level write → simplified read | every format | [x] `tests/h_roundtrip.rs::rt6_rt7_simplified_roundtrip` |
| RT8 | write → progressive read | every legal combo × feed granularity 1/7/whole | [x] `tests/h_roundtrip.rs::rt8_write_then_progressive_read` |
