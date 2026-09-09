# CONFIGS.md — configuration-surface table (valid inputs)

Derived from the branch structure of `c_src/src/*.c` + `c_src/include/pnglibconf.h`
(all listed `PNG_*_SUPPORTED` are ON — see `TRANSLATION_GUIDE.md` §0) and from the
384 symbols the C `.so` exports (`.verify/c.txt`), which include the
`PNG_INTERNAL_FUNCTION`s (`png_do_*`, `png_read_filter_row`, `png_combine_row`,
`png_icc_check_*`, `png_build_gamma_table`, …).  Those low-level entry points are
tested **directly**, not only through the one-shot wrappers.

Feature axes the C code actually branches on:

* **build config**: single configuration — `Cargo.toml` declares **no `[features]`**,
  so there is exactly one feature combination (the default = only combination).
* **color_type** ∈ {0 GRAY, 2 RGB, 3 PALETTE, 4 GRAY_ALPHA, 6 RGBA}
* **bit_depth** ∈ {1,2,4,8,16} (valid subset per color type — `png_check_IHDR`)
* **interlace** ∈ {0 NONE, 1 ADAM7} (7 passes × `png_do_read/write_interlace`,
  `png_combine_row` masks, `PNG_USE_COMPILE_TIME_MASKS`)
* **filter type** ∈ {NONE, SUB, UP, AVG, PAETH} × `bpp` 1…8 (`png_read_filter_row`)
* **read transforms** (`pngrtran.c` `transformations` bit set): EXPAND, EXPAND_16,
  GRAY_TO_RGB, RGB_TO_GRAY(+3 error actions), BACKGROUND(+`background_gamma_code`
  ∈ {SCREEN,FILE,UNIQUE}), ALPHA_MODE ∈ {UNASSOCIATED, ASSOCIATED/PREMULTIPLIED,
  OPTIMIZED, BROKEN, PNG_DEFAULT_sRGB, PNG_GAMMA_LINEAR}, GAMMA, QUANTIZE
  (with/without histogram, ≤ and > `maximum_colors`), STRIP_16 vs SCALE_16, PACK,
  PACKSWAP, SWAP_BYTES, SWAP_ALPHA, INVERT_MONO, INVERT_ALPHA, FILLER(BEFORE/AFTER)
  vs ADD_ALPHA, SHIFT, STRIP_ALPHA, USER_TRANSFORM, `png_set_interlace_handling`
* **write transforms** (`pngwtran.c`/`pngtrans.c`): PACK, SHIFT, SWAP_BYTES, BGR,
  PACKSWAP, INVERT_MONO, INVERT_ALPHA, SWAP_ALPHA, FILLER/strip, USER_TRANSFORM
* **write options**: `png_set_filter` (`PNG_FILTER_*` mask, `PNG_NO_FILTERS`,
  `PNG_ALL_FILTERS`, `PNG_FILTER_TYPE_BASE`, `PNG_INTRAPIXEL_DIFFERENCING`),
  compression `level` −1…9, `mem_level` 1…9, `strategy` 0…4, `window_bits` 8…15 and
  the negative/`PNG_OPTIMIZE_CMF` path, `png_set_compression_buffer_size`,
  `png_set_flush`/`png_write_flush`, text-compression variants of all of the above
* **error policy**: `png_set_benign_errors(0/1)`, `png_set_crc_action`
  (crit ∈ {DEFAULT,USE,STRIP,QUIET_USE,WARN_USE(≙USE)} × ancil ∈ {DEFAULT,ERROR,
  WARN,QUIET,USE}), `png_set_error_fn` with/without handlers
* **limits**: `png_set_user_limits`, `png_set_chunk_cache_max`,
  `png_set_chunk_malloc_max` (defaults 1000000/1000000/1000/8000000)
* **unknown chunks**: `png_set_keep_unknown_chunks` handling ∈ {DEFAULT, NEVER,
  IF_SAFE, ALWAYS} × (global / per-chunk list), `png_set_read_user_chunk_fn`
  (returns <0 / 0 / >0), `png_set_unknown_chunks` + `location` ∈ {HAVE_IHDR,
  HAVE_PLTE, AFTER_IDAT}
* **options**: `png_set_option(option ∈ {PNG_MAXIMUM_INFLATE_WINDOW,
  PNG_SKIP_sRGB_CHECK_PROFILE, PNG_IGNORE_ADLER32, PNG_ARM_NEON, …},
  onoff ∈ {PNG_OPTION_OFF, PNG_OPTION_ON})`, `png_permit_mng_features`
* **I/O mode**: sequential read (`png_read_info/row/rows/image/end`, `png_read_png`)
  vs **progressive** read (`png_set_progressive_read_fn` + `png_process_data` with
  1-byte / 3-byte / whole-buffer feeds, `png_process_data_pause/skip`) vs
  memory/stdio I/O callbacks, `png_init_io`
* **simplified API**: `png_image` `format` = cross-product of `PNG_FORMAT_FLAG_ALPHA`,
  `_COLOR`, `_LINEAR`, `_COLORMAP`, `_BGR`, `_AFIRST` × `row_stride` sign
  (bottom-up) × read-from-{memory,file,stdio} × write-to-{memory,file,stdio}
  × `convert_to_8bit`
* **input shapes**: width/height 1, 2, 7, 8, 9, 33 (all Adam7 pass residues),
  0-length and 1-byte buffers, `PNG_UINT_31_MAX` boundary values, palette sizes
  1/2/16/256, sBIT extremes, empty/one/many text chunks and unknown chunks

The project builds **no binary/driver** (`CMakeLists.txt` builds only the shared
library; `pngtest.c`/`example.c` are excluded), so there is no stdout comparison row.

## Configurations

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `png_access_version_number`, `png_get_libpng_ver`, `png_get_header_ver`, `png_get_header_version`, `png_get_copyright` | no state; string/number identity | [x] |
| 2 | `png_save_uint_32`, `png_save_uint_16`, `png_save_int_32`, `png_get_uint_32`, `png_get_uint_16`, `png_get_int_32` | 4096 random u32/i32 incl. 0, 1, 0x7fffffff, 0x80000000, 0xffffffff — byte order round-trip | [x] |
| 3 | `png_get_uint_31` | random u32 ≤ `PNG_UINT_31_MAX` (valid path only; error path in ERRORS.md) | [x] |
| 4 | `png_muldiv` | random (times, v1, v2) incl. 0 divisor, negatives, values overflowing 32 bits | [x] |
| 5 | `png_reciprocal`, `png_reciprocal2` | random fixed-point a (incl. 0) and (a,b) pairs | [x] |
| 6 | `png_gamma_significant`, `png_gamma_8bit_correct`, `png_gamma_16bit_correct`, `png_gamma_correct` | random gamma ∈ [1, 1000000], value over full 8-/16-bit range | [x] |
| 7 | `png_build_gamma_table` + `png_destroy_gamma_table` | bit_depth ∈ {8,16} × screen/file gamma pairs incl. 1.0, sRGB (45455/220000), extreme (10, 1000000) — compare every generated table byte | [x] |
| 8 | `png_sRGB_table`, `png_sRGB_base`, `png_sRGB_delta` (exported data) | full array contents byte-compare | [x] |
| 9 | `png_XYZ_from_xy`, `png_xy_from_XYZ` | sRGB primaries + random valid xy/XYZ sets, incl. degenerate/whitepoint-at-0 | [x] |
| 10 | `png_check_fp_number`, `png_check_fp_string` | random ASCII strings from the fp alphabet ("1", "-.5e+10", "0x", "1e", "..", "", "+", 30-char fuzz) | [x] |
| 11 | `png_ascii_from_fp` | precision 1…15 × values 0, ±1, ±1e-10, ±1e10, DBL_MIN/MAX, NaN-free randoms; buffer size exactly `PNG_fp_MAX` | [x] |
| 12 | `png_ascii_from_fixed` | random `png_fixed_point` incl. `INT_MIN`, `INT_MAX`, 0, ±1 | [x] |
| 13 | `png_fixed`, `png_fixed_ITU` | random doubles in range (valid path) | [x] |
| 14 | `png_safecat`, `png_format_number`, `png_warning_parameter*`, `png_formatted_warning` | all `PNG_NUMBER_FORMAT_*` (u, d, 02u, 02x, x) × random values; buffer sizes 1, 2, exact, oversized | [x] |
| 15 | `png_convert_to_rfc1123_buffer`, `png_convert_from_struct_tm`, `png_convert_from_time_t`, `png_convert_to_rfc1123` | valid `png_time`s (all 12 months, day 1/31, hour 0/23, sec 0/60), and `time_t` 0 / 1e9 | [x] |
| 16 | `png_sig_cmp` | correct signature, each single-byte corruption, all `start` 0…8 × `num_to_check` 0…8 | [x] |
| 17 | `png_check_IHDR` | full cross product width ∈ {0,1,7,8,1000000,1000001,PNG_UINT_31_MAX+1} × height same × bit_depth 0…17 × color_type 0…7 × interlace 0…2 × compression 0/1 × filter 0/1 (warnings recorded) | [x] |
| 18 | `png_build_grayscale_palette` | bit_depth ∈ {1,2,4,8} (and invalid 3,16) — full 256-entry palette compare | [x] |
| 19 | `png_do_bgr`, `png_do_invert`, `png_do_swap`, `png_do_packswap` | color_type ∈ {0,2,3,4,6} × bit_depth ∈ {1,2,4,8,16} × width ∈ {1,2,7,8,33} × random pixel data | [x] |
| 20 | `png_do_strip_channel` | channels ∈ {2,3,4} × bit_depth ∈ {8,16} × `at_start` 0/1 × random rows | [x] |
| 21 | `png_do_check_palette_indexes` | palette max ∈ {1,2,16,256} × rows containing in-range and out-of-range indexes × bit_depth ∈ {1,2,4,8} | [x] |
| 22 | `png_read_filter_row` | filter ∈ {NONE,SUB,UP,AVG,PAETH} (+ out-of-range 0 and 5) × `pixel_depth` ∈ {1,2,4,8,16,24,32,48,64} × random row+prev_row, many rows | [x] |
| 23 | `png_do_read_interlace` | pass 0…6 × color_type × bit_depth 1…16 × `transformations` with/without `PNG_PACKSWAP` × width 1…33 | [x] |
| 24 | `png_do_write_interlace` | pass 0…5 × row_info shapes as above | [x] |
| 25 | `png_combine_row` | display 0/1 × pass 0…6 × interlace on/off × `pixel_depth` 1…64 × random dsp_row/row | [x] |
| 26 | `png_do_read_transformations` (via full read) | each single transform in isolation, on a matching input image | [x] |
| 27 | `png_do_write_transformations` (via full write) | each single write transform in isolation | [x] |
| 28 | `png_write_*` chunk writers called directly (`png_write_IHDR`, `_PLTE`, `_IEND`, `_gAMA_fixed`, `_sRGB`, `_cHRM_fixed`, `_cICP`, `_cLLI_fixed`, `_mDCV_fixed`, `_sBIT`, `_bKGD`, `_hIST`, `_tRNS`, `_oFFs`, `_pHYs`, `_pCAL`, `_sCAL_s`, `_tIME`, `_eXIf`, `_tEXt`, `_zTXt`, `_iTXt`, `_iCCP`, `_sPLT`, `png_write_chunk*`, `png_write_sig`) | valid values per chunk; compare the exact byte stream captured by the write callback | [x] |
| 29 | `png_write_png` (one-shot) | color_type × bit_depth valid pairs × interlace 0/1 × `transforms` ∈ {IDENTITY, PACKING, PACKSWAP, INVERT_MONO, SHIFT, BGR, SWAP_ALPHA, SWAP_ENDIAN, INVERT_ALPHA, STRIP_FILLER_BEFORE/AFTER} — compare full PNG output bytes | [x] |
| 30 | `png_write_info` + `png_write_row`/`png_write_rows`/`png_write_image` + `png_write_end` (low-level pipeline) | same shapes as #29, driven row-by-row through the low-level API | [x] |
| 31 | write pipeline + `png_set_filter` | mask ∈ {NO_FILTERS, NONE, SUB, UP, AVG, PAETH, ALL_FILTERS, SUB\|PAETH} × color_type/bit_depth × interlace | [x] |
| 32 | write pipeline + `png_set_compression_level/mem_level/strategy/window_bits/method` | level ∈ {−1,0,1,6,9} × strategy ∈ {0,1,2,3,4} × window_bits ∈ {8,9,15} × mem_level ∈ {1,8,9} | [x] |
| 33 | write pipeline + `png_set_compression_buffer_size` | sizes 1, 2, 3, 1024, 8192, 65536 (forces multi-IDAT split) | [x] |
| 34 | write pipeline + `png_set_flush` + `png_write_flush` | flush_rows ∈ {0,1,2,height} | [x] |
| 35 | write pipeline, text chunks | tEXt / zTXt / iTXt (compressed and not) × key lengths 1/79 × empty text, 1-byte text, 10 kB text × `png_set_text_compression_*` variants | [x] |
| 36 | write pipeline, all ancillary chunks set via `png_set_*` then `png_write_info` | gAMA, cHRM, cHRM_XYZ, sRGB, sRGB_gAMA_and_cHRM, iCCP, sBIT, bKGD, hIST, tRNS, oFFs, pHYs, sCAL (double/fixed/string), pCAL (all 5 equation types, 0…N params), tIME, eXIf, sPLT, mDCV, cLLI, cICP, unknown chunks — singly and all together | [x] |
| 37 | write pipeline + `png_set_unknown_chunks` / `png_set_keep_unknown_chunks` | location ∈ {HAVE_IHDR, HAVE_PLTE, AFTER_IDAT} × handling ∈ {DEFAULT,NEVER,IF_SAFE,ALWAYS} × safe/unsafe chunk names × 0/1/many chunks | [x] |
| 38 | sequential read `png_read_info` + `png_read_row(s)` + `png_read_end` | every PNG produced by rows #29–#37 (round-trip), read back with no transforms; also `png_read_image` and `png_read_png` | [x] |
| 39 | sequential read + each read transform | `png_set_palette_to_rgb`, `_tRNS_to_alpha`, `_expand`, `_expand_gray_1_2_4_to_8`, `_expand_16`, `_gray_to_rgb`, `_rgb_to_gray(_fixed)` (error_action 1/2/3, default & custom coefficients), `_background(_fixed)` (all 3 gamma codes × need_expand 0/1), `_alpha_mode(_fixed)` (all 6 modes × gamma 1.0/2.2/sRGB), `_gamma(_fixed)`, `_quantize` (with/without hist, max_colors 2/16/255), `_strip_16`, `_scale_16`, `_packing`, `_packswap`, `_swap`, `_swap_alpha`, `_invert_mono`, `_invert_alpha`, `_filler(BEFORE/AFTER)`, `_add_alpha`, `_shift`, `_strip_alpha`, `_bgr`, `_interlace_handling`, `_read_user_transform_fn`+`_user_transform_info` | [x] |
| 40 | sequential read, transform **combinations** | expand+gray_to_rgb+add_alpha+swap; background+gamma+strip_16; palette→rgb+expand_16+bgr; quantize after expand; rgb_to_gray+background+alpha_mode; strip_alpha+packing+packswap+invert_mono | [x] |
| 41 | `png_read_update_info` / `png_start_read_image` ordering | called 0×, 1×, 2× before rows; `png_get_rowbytes`/`channels`/`bit_depth` after each | [x] |
| 42 | progressive read (`png_set_progressive_read_fn` + `png_process_data`) | feed granularity 1, 2, 3, 7, 13, whole-file bytes × interlace 0/1 × all color types × info/row/end callbacks recording every row | [x] |
| 43 | progressive read + `png_process_data_pause`/`png_process_data_skip` | pause with save 0/1 at each callback; skip over a large unknown chunk | [x] |
| 44 | progressive read + `png_progressive_combine_row` | display row combining for interlaced input, all 7 passes | [x] |
| 45 | read + `png_set_crc_action` | crit ∈ {DEFAULT,USE,STRIP,QUIET_USE} × ancil ∈ {DEFAULT,ERROR,WARN,QUIET,USE} on a stream with a deliberately bad CRC in a critical and in an ancillary chunk | [x] |
| 46 | read + `png_set_benign_errors(0/1)` | truncated/corrupt ancillary chunk with benign errors allowed vs not | [x] |
| 47 | read + `png_set_user_limits`, `png_set_chunk_cache_max`, `png_set_chunk_malloc_max` | limits above / exactly at / below the image's width, height, chunk count and chunk size; also `png_get_user_width_max`/`_height_max`/`_chunk_cache_max`/`_chunk_malloc_max` | [x] |
| 48 | read + `png_set_read_user_chunk_fn` | callback returning −1, 0, +1 on a private chunk, with `png_get_user_chunk_ptr` | [x] |
| 49 | read + `png_set_keep_unknown_chunks` | handling ∈ {DEFAULT,NEVER,IF_SAFE,ALWAYS} × global vs per-chunk list × `png_get_unknown_chunks` result compare; `png_handle_as_unknown`, `png_chunk_unknown_handling` | [x] |
| 50 | read + `png_set_option` | each of `PNG_MAXIMUM_INFLATE_WINDOW`, `PNG_SKIP_sRGB_CHECK_PROFILE`, `PNG_IGNORE_ADLER32` × {OFF, ON} — return value and effect on decode | [x] |
| 51 | read + `png_permit_mng_features` | 0 and `PNG_FLAG_MNG_FILTER_64` / `PNG_ALL_MNG_FEATURES`, with a filter-type-64 stream | [x] |
| 52 | read/write + `png_set_mem_fn` / `png_malloc*` / `png_calloc` / `png_free` / `png_malloc_array` / `png_realloc_array` / `png_free_buffer_list` | custom allocator counting calls; sizes 0, 1, 8, 1<<20; `png_get_mem_ptr` | [x] |
| 53 | `png_create_info_struct`, `png_info_init_3`, `png_destroy_info_struct`, `png_free_data`, `png_data_freer` | every `PNG_FREE_*` mask × `num` −1 and 0…N × `PNG_DESTROY_WILL_FREE_DATA`/`PNG_SET_WILL_FREE_DATA`/`PNG_USER_WILL_FREE_DATA` | [x] |
| 54 | all `png_get_*` accessors | after a full `png_read_info` of a stream containing every chunk — compare every out-param (incl. `png_get_pHYs_dpi`, `_pixels_per_inch`, `_x_offset_inches(_fixed)`, `_pixel_aspect_ratio(_fixed)`, `_palette_max`, `_rgb_to_gray_status`, `_current_row_number`, `_current_pass_number`, `_io_state`, `_io_chunk_type`, `_signature`, `_rows`, `_valid`) | [x] |
| 55 | `png_set_rows` + `png_read_png`/`png_write_png` | `PNG_INFO_IDAT` path, rows allocated by user | [x] |
| 56 | `png_set_check_for_invalid_index` | 0/1 × palette image with out-of-range index | [x] |
| 57 | `png_icc_check_header`, `png_icc_check_length`, `png_icc_check_tag_table`, `png_resolve_file_gamma` | a valid minimal ICC profile + variations of size/tag count/signature (valid-side boundaries) | [x] |
| 58 | `png_set_iCCP` + write + read round-trip | profile lengths 132, 133, 1 kB; name lengths 1, 79 | [x] |
| 59 | simplified read: `png_image_begin_read_from_memory` + `png_image_finish_read` | input = every PNG shape from #29 × output `format` ∈ full cross-product of {GRAY,GA,AG,RGB,BGR,RGBA,ARGB,BGRA,ABGR} × {8-bit, LINEAR} × {colormap, direct} × `row_stride` positive and negative × `background` NULL and set | [x] |
| 60 | simplified read from file/stdio | `png_image_begin_read_from_file`, `..._from_stdio` on a temp file — same formats as #59 | [x] |
| 61 | simplified write: `png_image_write_to_memory` | `format` cross-product as #59 × `convert_to_8bit` 0/1 × `row_stride` ±; also two-pass size query (`memory == NULL`) | [x] |
| 62 | simplified write to file/stdio | `png_image_write_to_file`, `..._to_stdio` — byte-compare the resulting files | [x] |
| 63 | `png_image_free`, `png_image_error`, `png_safe_execute`, `png_safe_error`, `png_safe_warning` | after successful and after failed reads/writes | [x] |
| 64 | `png_set_read_status_fn` / `png_set_write_status_fn` | row/pass callbacks recorded for interlaced and non-interlaced images | [x] |
| 65 | `png_get_io_state` / `png_get_io_chunk_type` during read and write | sampled inside the read/write callbacks at every call | [x] |
| 66 | `png_reset_crc`, `png_calculate_crc`, `png_crc_read`, `png_crc_finish`, `png_reset_zstream`, `png_zstream_error`, `png_zalloc`/`png_zfree` | random buffers/lengths, incl. length 0 and 1 | [x] |
| 67 | `png_read_chunk_header`, `png_handle_chunk`, `png_handle_unknown`, `png_push_read_chunk` (low-level, driven directly) | each of the known chunk types, valid, in a hand-built stream | [x] |
| 68 | `png_set_invalid`, `png_get_valid` | every `PNG_INFO_*` bit | [x] |
| 69 | `png_set_filter_heuristics`, `png_set_filter_heuristics_fixed` (deprecated stubs) | any args — must behave identically (no-op + warning) | [x] |
| 70 | `png_set_sig_bytes` + `png_read_info` | num_bytes 0…8 with the signature pre-consumed | [x] |

## Coverage

All 70 rows are `[x]`: each is driven against BOTH `.so` files through `dlsym` only, with
fixed-seed randomised inputs, and every byte of output plus the full warning/error message
log compared.  Where the rows are covered:

| rows | test file | tests |
|---|---|---|
| 1–18 | `tests/t_pure.rs` | 22 |
| 19–25 | `tests/t_rowops.rs` | 10 |
| 26–27, 38–51, 54–56, 64–65, 70 | `tests/t_read.rs` | 60 |
| 28–37 | `tests/t_write.rs` | 10 |
| 52–53, 57, 66–67 | `tests/t_mem.rs` | 20 |
| 58 | `tests/t_write.rs` (`cfg36`, iCCP written) + `tests/t_read.rs` (`cfg54`, iCCP read back) |
| 59–63 | `tests/t_simple.rs` | 18 |
| 68 | `tests/t_err2.rs` (`png_set_invalid_out_of_range_mask`) + `tests/t_read.rs` (`cfg54`, every `PNG_INFO_*` bit) + `tests/t_mem.rs` (`cfg53_free_data_masks`) |
| 69 | `tests/t_err5.rs` (`t_set_filter_heuristics_are_noops`) |

Rows 26/27 (`png_do_read_transformations` / `png_do_write_transformations`) are exercised
end-to-end: every individual read transform in `cfg39_*` and every `PNG_TRANSFORM_*` write
transform in `cfg29_write_png_oneshot` / `cfg30_lowlevel_pipeline`, plus 40 000 fixed-seed
random *subsets* of the 24 read transforms in `cfg40_random_transform_combos`.

Feature combinations: `Cargo.toml` declares **no `[features]`**, so the default build is
the only configuration.  `./run_all.sh` derives the feature list from `Cargo.toml` and
loops over it, and reports the single configuration.
