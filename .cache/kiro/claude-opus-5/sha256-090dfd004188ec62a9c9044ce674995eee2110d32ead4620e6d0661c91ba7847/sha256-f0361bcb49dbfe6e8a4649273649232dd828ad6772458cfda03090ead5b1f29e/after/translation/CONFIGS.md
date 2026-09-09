# CONFIGS.md — configuration-surface table (valid inputs)

## Verification result

**164 of 165** rows pass across randomized inputs. The remaining row (149) is
marked `[—]`: it names `png_set_strip_error_numbers`, which
`PNG_ERROR_NUMBERS_SUPPORTED` leaves out of this build, so *neither* `.so`
exports it and there is no entry point to drive. Reproduce with:

```sh
cd translation && cargo build --release && cargo test --release --test diff
# cases passed: 267 / 267
# CONFIGS.md rows covered: 164   ERRORS.md rows covered: 331
```

Every row is exercised with many randomized inputs (fixed seed, derived per case
so the streams are independent), not a single hand-picked value, and the
comparison is byte-for-byte over the complete transcript: emitted PNG bytes,
decoded row bytes, every `png_get_*` return and out-parameter, the full
warning/error text, the callback invocation order, and the process exit status.

The project builds **no** binary driver — `c_src/CMakeLists.txt` produces only
`libpng.so` (it explicitly excludes `pngtest.c` and `example.c`) and the crate
declares no `[[bin]]` target — so the "compare the two binaries' stdout"
requirement does not apply. The worker/driver split in `tests/diff.rs` does the
equivalent thing at the library level: each case runs in its own process per
library and the two stdouts are compared.

Three test defects were found and fixed while discharging these rows, each of
which had been masking real coverage rather than causing a false failure:

* `wr/simple_stride` / `rd/simple_stride` passed an already-offset base pointer
  for a negative `row_stride`. libpng offsets to the last row itself
  (`pngwrite.c`: `row += (image->height-1) * (-row_step)`), so this read past the
  end of the buffer and made *both* libraries produce nondeterministic output.
* `wr/tr_shift`, `wr/tr_filler_after`, `wr/tr_filler_before` applied the
  transform before `png_write_info`. `png_set_shift`/`png_set_filler` validate
  against `png_ptr->bit_depth`/`color_type`, which on a write struct are only
  populated by `png_write_IHDR`; the calls were therefore rejected outright and
  the transforms never ran.
* `rd/user_chunk_fn` used the chunk name `uNkN`. `check_chunk_name`
  (`pngrutil.c:153`) clears bit 5 of bytes 0, 1 and 3 but *not* byte 2, so a
  lowercase reserved byte is rejected as `bad header (invalid type)` before the
  user callback is reached.

---

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the axes the
C source actually branches on:

```
grep -oE 'png_set_[a-z_0-9]+' c_src/include/png.h | sort -u          # 80 runtime option setters
grep -oE 'PNG_TRANSFORM_[A-Z_0-9]+' c_src/include/png.h | sort -u    # 18 one-shot transform bits
grep -nE '^\s*(if|switch|case)' c_src/src/pngrtran.c c_src/src/pngrutil.c c_src/src/pngwutil.c
grep -n 'PNG_FORMAT_FLAG' c_src/include/png.h                        # 5 simplified-API format bits
```

## Axes

**A1 — colour type × bit depth** (the shapes IHDR legally admits, and every
`switch (color_type)` in `pngrtran.c` / `pngwutil.c` / `pngrutil.c` branches on):
`GRAY(0)`×{1,2,4,8,16}, `RGB(2)`×{8,16}, `PALETTE(3)`×{1,2,4,8},
`GRAY_ALPHA(4)`×{8,16}, `RGBA(6)`×{8,16} → **15 legal combinations**.

**A2 — interlace**: `PNG_INTERLACE_NONE(0)`, `PNG_INTERLACE_ADAM7(1)`.
Adam7 has 7 passes with distinct row/column strides; `png_combine_row` and
`png_do_read_interlace` special-case each pass and the sub-byte pixel depths.

**A3 — image dimensions**: `1×1`, `1×N`, `N×1`, sizes that make the last byte of a
row partially filled at depths 1/2/4 (`width % (8/depth) != 0`), widths where an
Adam7 pass is empty (`width < 5`, `height < 5`), and larger random sizes.

**A4 — entry-point level**. Three distinct read drivers and three write drivers,
each composing the pipeline differently:
* `R-LOW` — `png_read_info` → `png_read_update_info` → `png_read_row` per row →
  `png_read_end` (the lowest-level sequential API).
* `R-ROWS` — `png_read_info` → `png_start_read_image` → `png_read_rows` in
  chunks → `png_read_end`.
* `R-IMAGE` — `png_read_info` → `png_set_rows`/`png_read_image` → `png_read_end`.
* `R-PNG` — `png_read_png` one-shot with a `png_transforms` mask.
* `R-PROG` — `png_set_progressive_read_fn` + `png_process_data` fed in varying
  chunk sizes (1, 2, 7, 13, 8192, whole buffer).
* `R-SIMPLE` — `png_image_begin_read_from_memory` + `png_image_finish_read`.
* `W-LOW` — `png_write_sig`/`png_write_info` → `png_write_row` per row →
  `png_write_end`.
* `W-ROWS` — `png_write_info` → `png_write_rows` → `png_write_end`.
* `W-IMAGE` — `png_write_info` → `png_write_image` → `png_write_end`.
* `W-PNG` — `png_write_png` one-shot with a `png_transforms` mask.
* `W-SIMPLE` — `png_image_write_to_memory`.
* `W-CHUNK` — raw `png_write_chunk_start`/`_data`/`_end` and `png_write_chunk`.

**A5 — read transforms** (`pngrtran.c`; each sets a bit in
`png_ptr->transformations` and adds a stage to `png_do_read_transformations`):
`png_set_palette_to_rgb`, `png_set_expand`, `png_set_expand_gray_1_2_4_to_8`,
`png_set_expand_16`, `png_set_gray_to_rgb`, `png_set_rgb_to_gray[_fixed]`
(3 error actions × coefficients), `png_set_strip_16`, `png_set_scale_16`,
`png_set_strip_alpha`, `png_set_swap_alpha`, `png_set_invert_alpha`,
`png_set_filler`/`png_set_add_alpha` (BEFORE/AFTER × filler value),
`png_set_packing`, `png_set_packswap`, `png_set_shift`, `png_set_swap`,
`png_set_bgr`, `png_set_invert_mono`, `png_set_gamma[_fixed]`,
`png_set_alpha_mode[_fixed]` (5 modes), `png_set_background[_fixed]`
(3 gamma codes × need_expand), `png_set_quantize` (with/without hIST,
with/without a full colour cube), `png_set_read_user_transform_fn`,
`png_set_interlace_handling`.

**A6 — write transforms** (`pngwtran.c` / `pngtrans.c`): `png_set_packing`,
`png_set_packswap`, `png_set_shift`, `png_set_swap`, `png_set_bgr`,
`png_set_invert_mono`, `png_set_invert_alpha`, `png_set_swap_alpha`,
`png_set_filler` (strip filler BEFORE/AFTER), `png_set_write_user_transform_fn`.

**A7 — compression parameters** (`pngwutil.c` `png_deflate_claim`): filter mask
(`NONE`, `SUB`, `UP`, `AVG`, `PAETH`, `ALL`, and pairwise combinations),
`png_set_compression_level` 0..9, `_strategy` 0..4, `_mem_level` 1..9,
`_window_bits` 8..15, `png_set_compression_buffer_size`, and the separate text
zstream (`png_set_text_compression_*`).

**A8 — ancillary chunks present** (each has its own `png_handle_*` /
`png_write_*` pair): `gAMA`, `cHRM`, `sRGB`, `iCCP`, `sBIT`, `tRNS`, `bKGD`,
`hIST`, `pHYs`, `oFFs`, `pCAL`, `sCAL`, `tIME`, `sPLT`, `tEXt`, `zTXt`, `iTXt`,
`eXIf`, `cICP`, `cLLI`, `mDCV`, unknown chunks (each `PNG_HANDLE_CHUNK_*` mode).

**A9 — simplified-API format** (`PNG_FORMAT_FLAG_*` cross-product): `ALPHA(0x01)`
× `COLOR(0x02)` × `LINEAR(0x04)` × `COLORMAP(0x08)` × `BGR(0x10)` ×
`AFIRST(0x20)` — all 6 bits are compiled in, and `png_image_read_direct` /
`png_image_read_colormap` branch on every one.

**A10 — global state toggles**: `png_set_benign_errors(0/1)`,
`png_set_crc_action` (6×6 actions), `png_set_check_for_invalid_index(0/1)`,
`png_set_user_limits`, `png_set_chunk_cache_max`, `png_set_chunk_malloc_max`,
`png_set_option` (`PNG_MAXIMUM_INFLATE_WINDOW`, `PNG_SKIP_sRGB_CHECK_PROFILE`,
`PNG_IGNORE_ADLER32`), `png_set_keep_unknown_chunks`,
`png_set_strip_error_numbers`, `png_set_mem_fn`, `png_set_flush`,
`png_set_read_status_fn` / `png_set_write_status_fn`, `png_set_sig_bytes`.

**A11 — byte-order / packing helpers** (lowest-level exported primitives):
`png_get_uint_32`, `png_get_uint_16`, `png_get_int_32`, `png_get_uint_31`,
`png_save_uint_32`, `png_save_uint_16`, `png_save_int_32` over the full 32-bit
value space (randomized + boundary values).

---

## Rows

Every row is exercised by calling **both** `.so`s through their exports in that
exact configuration, over **many randomized inputs** (fixed seed `0x5eed_1b90`,
16–64 images per row depending on cost), and comparing the complete output
byte-for-byte (encoded PNG bytes, decoded row bytes, all `png_get_*` values, and
the full warning/error transcript).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `png_get_uint_32` / `_uint_16` / `_int_32` / `_uint_31` | all 4-byte and 2-byte patterns: boundaries `00000000`, `0000ffff`, `7fffffff`, `80000000`, `ffffffff` + 4096 random words | [x] |
| 2 | `png_save_uint_32` / `_uint_16` / `_int_32` | same value set, round-tripped through `png_get_*` | [x] |
| 3 | `png_sig_cmp` | valid signature at every `start` 0..7 × every `num_to_check` 1..8 | [x] |
| 4 | `png_access_version_number` / `png_get_libpng_ver` / `png_get_header_ver` / `png_get_header_version` / `png_get_copyright` / `png_get_libpng_transforms` | no options | [x] |
| 5 | `png_write_chunk_start`/`_data`/`_end` + `png_write_chunk` (`W-CHUNK`) | raw chunk emission, random type + payload lengths `0,1,2,255,256,8191,8192,8193` | [x] |
| 6 | `png_write_sig` + `W-LOW` | GRAY×1, no ancillary chunks, `filter=NONE`, `level=6`, non-interlaced, sizes from A3 | [x] |
| 7 | `W-LOW` | GRAY×2 — sub-byte packing, widths not a multiple of 4 | [x] |
| 8 | `W-LOW` | GRAY×4 — widths not a multiple of 2 | [x] |
| 9 | `W-LOW` | GRAY×8 | [x] |
| 10 | `W-LOW` | GRAY×16 | [x] |
| 11 | `W-LOW` | RGB×8 | [x] |
| 12 | `W-LOW` | RGB×16 | [x] |
| 13 | `W-LOW` | PALETTE×1 + `PLTE` of 2 entries | [x] |
| 14 | `W-LOW` | PALETTE×2 + `PLTE` of 4 | [x] |
| 15 | `W-LOW` | PALETTE×4 + `PLTE` of 16 | [x] |
| 16 | `W-LOW` | PALETTE×8 + `PLTE` of 256 | [x] |
| 17 | `W-LOW` | GRAY_ALPHA×8 | [x] |
| 18 | `W-LOW` | GRAY_ALPHA×16 | [x] |
| 19 | `W-LOW` | RGBA×8 | [x] |
| 20 | `W-LOW` | RGBA×16 | [x] |
| 21 | `W-LOW` | each of the 15 A1 combinations, **interlaced (ADAM7)** | [x] |
| 22 | `W-ROWS` | all 15 A1 combinations, non-interlaced, rows submitted in chunks of 1/3/all | [x] |
| 23 | `W-IMAGE` | all 15 A1 combinations, non-interlaced | [x] |
| 24 | `W-IMAGE` | all 15 A1 combinations, interlaced (drives `png_write_interlace` internally) | [x] |
| 25 | `W-LOW` | filter mask `PNG_FILTER_NONE` only, RGBA×8, 64×64 random | [x] |
| 26 | `W-LOW` | filter mask `PNG_FILTER_SUB` only | [x] |
| 27 | `W-LOW` | filter mask `PNG_FILTER_UP` only | [x] |
| 28 | `W-LOW` | filter mask `PNG_FILTER_AVG` only | [x] |
| 29 | `W-LOW` | filter mask `PNG_FILTER_PAETH` only | [x] |
| 30 | `W-LOW` | filter mask `PNG_ALL_FILTERS` (adaptive selection — exercises `png_setup_*_row` cost comparison) | [x] |
| 31 | `W-LOW` | filter mask pairs `NONE\|SUB`, `SUB\|UP`, `UP\|AVG`, `AVG\|PAETH`, `NONE\|PAETH` | [x] |
| 32 | `W-LOW` | `png_set_compression_level` 0,1,3,6,9 × `PNG_ALL_FILTERS` | [x] |
| 33 | `W-LOW` | `png_set_compression_strategy` 0,1,2,3,4 | [x] |
| 34 | `W-LOW` | `png_set_compression_mem_level` 1,4,8,9 | [x] |
| 35 | `W-LOW` | `png_set_compression_window_bits` 8,9,11,15 | [x] |
| 36 | `W-LOW` | `png_set_compression_buffer_size` 6, 64, 1024, 8192, 65536 (forces multiple `IDAT` chunks) | [x] |
| 37 | `W-LOW` | `png_set_flush(n)` for n = 1, 2, 5 with `png_write_flush` | [x] |
| 38 | `W-LOW` | `png_set_write_status_fn` installed — callback row/pass sequence recorded and compared | [x] |
| 39 | `W-LOW` + A6 | `png_set_bgr` on RGB×8 / RGB×16 / RGBA×8 / RGBA×16 | [x] |
| 40 | `W-LOW` + A6 | `png_set_swap` on all 16-bit types | [x] |
| 41 | `W-LOW` + A6 | `png_set_packing` with supplied 8-bit-per-pixel input for GRAY×1/2/4 and PALETTE×1/2/4 | [x] |
| 42 | `W-LOW` + A6 | `png_set_packswap` for GRAY×1/2/4 and PALETTE×1/2/4 | [x] |
| 43 | `W-LOW` + A6 | `png_set_shift` with every legal `sig_bit` combination for GRAY×8/16, RGB×8/16, RGBA×8 | [x] |
| 44 | `W-LOW` + A6 | `png_set_invert_mono` on GRAY×1/2/4/8/16 | [x] |
| 45 | `W-LOW` + A6 | `png_set_invert_alpha` on GRAY_ALPHA and RGBA, 8 and 16 bit | [x] |
| 46 | `W-LOW` + A6 | `png_set_swap_alpha` on GRAY_ALPHA and RGBA, 8 and 16 bit | [x] |
| 47 | `W-LOW` + A6 | `png_set_filler(PNG_FILLER_AFTER)` stripping on RGB from RGBA input, 8 and 16 bit | [x] |
| 48 | `W-LOW` + A6 | `png_set_filler(PNG_FILLER_BEFORE)` stripping | [x] |
| 49 | `W-LOW` + A6 | `png_set_write_user_transform_fn` that mutates the row | [x] |
| 50 | `W-LOW` + A6 | two transforms composed: `bgr`+`swap`, `packing`+`packswap`, `invert_alpha`+`swap_alpha`, `shift`+`swap` | [x] |
| 51 | `W-PNG` | `png_transforms = PNG_TRANSFORM_IDENTITY` on all 15 A1 combos | [x] |
| 52 | `W-PNG` | `PNG_TRANSFORM_PACKING`, `_PACKSWAP`, `_INVERT_MONO`, `_SHIFT`, `_BGR`, `_SWAP_ALPHA`, `_SWAP_ENDIAN`, `_INVERT_ALPHA`, `_STRIP_FILLER_BEFORE`, `_STRIP_FILLER_AFTER` individually | [x] |
| 53 | `W-PNG` | pairwise transform masks (`BGR\|SWAP_ENDIAN`, `PACKING\|PACKSWAP`, `INVERT_ALPHA\|SWAP_ALPHA`) | [x] |
| 54 | `W-LOW` + A8 | `gAMA` written via `png_set_gAMA` and `png_set_gAMA_fixed`, values 1, 45455, 100000, 500000, `PNG_UINT_31_MAX` | [x] |
| 55 | `W-LOW` + A8 | `cHRM` via `png_set_cHRM` / `_fixed` / `_XYZ` / `_XYZ_fixed`, random valid chromaticities | [x] |
| 56 | `W-LOW` + A8 | `sRGB` intents 0,1,2,3 and `png_set_sRGB_gAMA_and_cHRM` | [x] |
| 57 | `W-LOW` + A8 | `iCCP` with a synthetic but structurally valid 132+ byte profile, lengths 132, 136, 1024, 8192 | [x] |
| 58 | `W-LOW` + A8 | `sBIT` for every legal `sig_bit` on every A1 combo | [x] |
| 59 | `W-LOW` + A8 | `tRNS`: palette (1..256 entries), GRAY×{1,2,4,8,16}, RGB×{8,16} | [x] |
| 60 | `W-LOW` + A8 | `bKGD`: palette index, GRAY levels, RGB triples, 8 and 16 bit | [x] |
| 61 | `W-LOW` + A8 | `hIST` with `num_palette` = 2, 16, 256 | [x] |
| 62 | `W-LOW` + A8 | `pHYs` units 0 and 1, random resolutions; also `png_set_pHYs` + `png_get_x_pixels_per_inch` round-trip | [x] |
| 63 | `W-LOW` + A8 | `oFFs` units 0 and 1, offsets incl. negative and `INT32_MIN+1` | [x] |
| 64 | `W-LOW` + A8 | `pCAL` equation types 0,1,2,3 with the matching `nparams` (0/1/2/3) | [x] |
| 65 | `W-LOW` + A8 | `sCAL` units 1 and 2 via `png_set_sCAL`, `_fixed`, and `_s` | [x] |
| 66 | `W-LOW` + A8 | `tIME` at every legal boundary (`1/1 00:00:00`, `12/31 23:59:60`, year 0 and 9999) | [x] |
| 67 | `W-LOW` + A8 | `sPLT` with 1, 2, 16 entries at depths 8 and 16 | [x] |
| 68 | `W-LOW` + A8 | `tEXt` — keyword lengths 1 and 79, text lengths 0, 1, 8191, 8192, 8193 | [x] |
| 69 | `W-LOW` + A8 | `zTXt` — same, plus `png_set_text_compression_level` 0,1,6,9 | [x] |
| 70 | `W-LOW` + A8 | `iTXt` — compressed and uncompressed, with and without language/translated-keyword | [x] |
| 71 | `W-LOW` + A8 | `eXIf` via `png_set_eXIf_1`, both TIFF byte orders, lengths 4, 8, 1024 | [x] |
| 72 | `W-LOW` + A8 | `cICP` with `matrix_coefficients == 0`, random primaries/transfer/full-range | [x] |
| 73 | `W-LOW` + A8 | `cLLI` via `png_set_cLLI` and `_fixed`, values 0, 1, `0x7FFFFFFF` | [x] |
| 74 | `W-LOW` + A8 | `mDCV` via `png_set_mDCV` and `_fixed`, in-range chromaticities | [x] |
| 75 | `W-LOW` + A8 | unknown chunks written via `png_set_unknown_chunks` at each of the 3 locations, plus `png_set_keep_unknown_chunks` for each `PNG_HANDLE_CHUNK_*` value 0..4 | [x] |
| 76 | `W-LOW` + A8 | **all** ancillary chunks set at once (maximal info struct) | [x] |
| 77 | `W-SIMPLE` | `png_image_write_to_memory` for each of the 16 non-colormap `PNG_FORMAT_*` combinations of `ALPHA\|COLOR\|LINEAR\|BGR\|AFIRST` | [x] |
| 78 | `W-SIMPLE` | colormapped formats (`FLAG_COLORMAP` set) × `ALPHA` × `COLOR` × `BGR` × `AFIRST` | [x] |
| 79 | `W-SIMPLE` | `convert_to_8bit = 0` and `1` on a linear (16-bit) source | [x] |
| 80 | `W-SIMPLE` | `row_stride` positive, negative (bottom-up), and larger than the minimum | [x] |
| 81 | `W-SIMPLE` | `png_image_write_to_memory` two-pass sizing: first call with `memory=NULL` to obtain the size, second to fill | [x] |
| 82 | `W-SIMPLE` | `png_image_write_to_stdio` / `_to_file` to a temp path, file bytes compared | [x] |
| 83 | `R-LOW` | every A1 combination, non-interlaced, no transforms — decoded rows + all `png_get_*` compared | [x] |
| 84 | `R-LOW` | every A1 combination, **interlaced**, `png_set_interlace_handling` — all 7 passes | [x] |
| 85 | `R-LOW` | interlaced, **without** `png_set_interlace_handling` (caller loops passes itself) | [x] |
| 86 | `R-ROWS` | `png_read_rows` with `row_pointers` only, `display_row` only, and both non-NULL, interlaced and not | [x] |
| 87 | `R-IMAGE` | `png_read_image` on all A1 combos, interlaced and not | [x] |
| 88 | `R-PNG` | `png_read_png` with `PNG_TRANSFORM_IDENTITY` on all A1 combos | [x] |
| 89 | `R-PNG` | `png_read_png` with each of `_STRIP_16`, `_SCALE_16`, `_STRIP_ALPHA`, `_PACKING`, `_PACKSWAP`, `_EXPAND`, `_EXPAND_16`, `_INVERT_MONO`, `_SHIFT`, `_BGR`, `_SWAP_ALPHA`, `_SWAP_ENDIAN`, `_INVERT_ALPHA`, `_GRAY_TO_RGB` individually | [x] |
| 90 | `R-PNG` | pairwise/triple transform masks (`EXPAND\|GRAY_TO_RGB`, `EXPAND_16\|SWAP_ENDIAN`, `STRIP_16\|STRIP_ALPHA`, `PACKING\|PACKSWAP\|INVERT_MONO`) | [x] |
| 91 | `R-PROG` | `png_process_data` fed in chunk sizes 1, 2, 3, 7, 13, 64, 1024, whole — all A1 combos, non-interlaced | [x] |
| 92 | `R-PROG` | same, interlaced (drives `png_progressive_combine_row`) | [x] |
| 93 | `R-PROG` | `png_process_data_pause` with `save = 0` and `1` | [x] |
| 94 | `R-PROG` | progressive with only `info_fn`, only `row_fn`, only `end_fn`, and all three NULL | [x] |
| 95 | `R-SIMPLE` | `png_image_begin_read_from_memory` + `finish_read` for all 16 non-colormap output formats | [x] |
| 96 | `R-SIMPLE` | colormapped output formats (`FLAG_COLORMAP`) × `ALPHA`/`COLOR`/`BGR`/`AFIRST` | [x] |
| 97 | `R-SIMPLE` | `background` supplied vs `NULL` where alpha is being removed | [x] |
| 98 | `R-SIMPLE` | negative `row_stride` (bottom-up), and stride larger than minimum | [x] |
| 99 | `R-SIMPLE` | `png_image_begin_read_from_stdio` and `_from_file` on a temp file | [x] |
| 100 | `R-LOW` + A5 | `png_set_palette_to_rgb` on PALETTE×{1,2,4,8}, with and without `tRNS` | [x] |
| 101 | `R-LOW` + A5 | `png_set_expand` on PALETTE, on GRAY×{1,2,4} , and on images with `tRNS` | [x] |
| 102 | `R-LOW` + A5 | `png_set_expand_gray_1_2_4_to_8` on GRAY×1/2/4 | [x] |
| 103 | `R-LOW` + A5 | `png_set_expand_16` on every A1 combo | [x] |
| 104 | `R-LOW` + A5 | `png_set_gray_to_rgb` on GRAY×{1,2,4,8,16} and GRAY_ALPHA×{8,16} | [x] |
| 105 | `R-LOW` + A5 | `png_set_rgb_to_gray_fixed` error_action 1,2,3 × coefficients (default, `(21000,71000)`, `(0,0)`, `(100000,0)`) on grey-valued RGB input | [x] |
| 106 | `R-LOW` + A5 | `png_set_strip_16` on all 16-bit A1 combos | [x] |
| 107 | `R-LOW` + A5 | `png_set_scale_16` on all 16-bit A1 combos | [x] |
| 108 | `R-LOW` + A5 | `png_set_strip_alpha` on GRAY_ALPHA×{8,16}, RGBA×{8,16} | [x] |
| 109 | `R-LOW` + A5 | `png_set_swap_alpha` on GRAY_ALPHA/RGBA, 8 and 16 bit | [x] |
| 110 | `R-LOW` + A5 | `png_set_invert_alpha` on GRAY_ALPHA/RGBA, 8 and 16 bit | [x] |
| 111 | `R-LOW` + A5 | `png_set_filler(v, BEFORE)` and `(v, AFTER)` on GRAY×8/16 and RGB×8/16, filler `0`, `0x7f`, `0xffff` | [x] |
| 112 | `R-LOW` + A5 | `png_set_add_alpha(v, BEFORE/AFTER)` on GRAY×8/16 and RGB×8/16 | [x] |
| 113 | `R-LOW` + A5 | `png_set_packing` on GRAY×{1,2,4} and PALETTE×{1,2,4} | [x] |
| 114 | `R-LOW` + A5 | `png_set_packswap` on GRAY×{1,2,4} and PALETTE×{1,2,4} | [x] |
| 115 | `R-LOW` + A5 | `png_set_shift` with `sBIT` present, every legal shift on GRAY×{8,16}, RGB×{8,16}, RGBA×8 | [x] |
| 116 | `R-LOW` + A5 | `png_set_swap` on all 16-bit A1 combos | [x] |
| 117 | `R-LOW` + A5 | `png_set_bgr` on RGB×{8,16}, RGBA×{8,16} | [x] |
| 118 | `R-LOW` + A5 | `png_set_invert_mono` on GRAY×{1,2,4,8,16} | [x] |
| 119 | `R-LOW` + A5 | `png_set_gamma_fixed(screen, file)` over `{100000, 45455, 220000, 16, 625000000}²` on GRAY×8, RGB×8, RGB×16, PALETTE×8 | [x] |
| 120 | `R-LOW` + A5 | `png_set_gamma` (float) same grid — exercises `png_fixed` conversion | [x] |
| 121 | `R-LOW` + A5 | image carries `gAMA`; `png_set_gamma` uses `PNG_DEFAULT_sRGB` / `PNG_GAMMA_MAC_18` / explicit | [x] |
| 122 | `R-LOW` + A5 | image carries `sRGB`; gamma handling path via `png_ptr->colorspace` | [x] |
| 123 | `R-LOW` + A5 | `png_set_alpha_mode_fixed` mode `PNG_ALPHA_PNG(0)` × gamma grid | [x] |
| 124 | `R-LOW` + A5 | mode `PNG_ALPHA_STANDARD(1)` × gamma grid, on GRAY_ALPHA and RGBA | [x] |
| 125 | `R-LOW` + A5 | mode `PNG_ALPHA_ASSOCIATED(2)` | [x] |
| 126 | `R-LOW` + A5 | mode `PNG_ALPHA_OPTIMIZED(3)` | [x] |
| 127 | `R-LOW` + A5 | mode `PNG_ALPHA_BROKEN(4)` | [x] |
| 128 | `R-LOW` + A5 | `png_set_background_fixed` with `PNG_BACKGROUND_GAMMA_SCREEN(1)` × `need_expand` 0/1 | [x] |
| 129 | `R-LOW` + A5 | `PNG_BACKGROUND_GAMMA_FILE(2)` × `need_expand` 0/1 | [x] |
| 130 | `R-LOW` + A5 | `PNG_BACKGROUND_GAMMA_UNIQUE(3)` × `need_expand` 0/1 × background gamma grid | [x] |
| 131 | `R-LOW` + A5 | `png_set_background` on a palette image with `need_expand=1` and a palette-index background | [x] |
| 132 | `R-LOW` + A5 | `png_set_background` combined with `tRNS` on GRAY, RGB and PALETTE | [x] |
| 133 | `R-LOW` + A5 | `png_set_quantize` on RGB×8, `maximum_colors` 2, 16, 64, 255, 256, with `hIST` and without, `full_dither` 0/1 | [x] |
| 134 | `R-LOW` + A5 | `png_set_quantize` on PALETTE×8 with `num_palette` 2/16/256 | [x] |
| 135 | `R-LOW` + A5 | `png_set_read_user_transform_fn` (+ `png_set_user_transform_info`) that rewrites the row | [x] |
| 136 | `R-LOW` + A5 | `png_set_read_status_fn` installed — callback row/pass sequence compared | [x] |
| 137 | `R-LOW` + A5 | composed pipelines: `expand`+`gray_to_rgb`+`add_alpha`; `expand_16`+`swap`; `strip_16`+`strip_alpha`+`gray_to_rgb`; `palette_to_rgb`+`bgr`+`invert_alpha`; `gamma`+`background`+`expand`; `alpha_mode`+`expand_16`+`swap`; `packing`+`shift`+`invert_mono` | [x] |
| 138 | `R-LOW` + A10 | `png_set_benign_errors(0)` and `(1)` on a stream with a recoverable defect | [x] |
| 139 | `R-LOW` + A10 | `png_set_crc_action` over the 6×6 `crit_action`×`ancil_action` grid on a stream with a bad ancillary CRC and a bad critical CRC | [x] |
| 140 | `R-LOW` + A10 | `png_set_check_for_invalid_index(0)` / `(1)` on a palette image with an out-of-range index | [x] |
| 141 | `R-LOW` + A10 | `png_set_user_limits(w,h)` at, below and above the image size; `png_get_user_width_max`/`_height_max` compared | [x] |
| 142 | `R-LOW` + A10 | `png_set_chunk_cache_max` 0,1,2,3,1000 with 4 `tEXt` chunks present | [x] |
| 143 | `R-LOW` + A10 | `png_set_chunk_malloc_max` 0, 1, 100, 8000000 with a large `tEXt` | [x] |
| 144 | `R-LOW` + A10 | `png_set_option(PNG_MAXIMUM_INFLATE_WINDOW, ON/OFF)` | [x] |
| 145 | `R-LOW` + A10 | `png_set_option(PNG_SKIP_sRGB_CHECK_PROFILE, ON/OFF)` with an `iCCP` chunk | [x] |
| 146 | `R-LOW` + A10 | `png_set_option(PNG_IGNORE_ADLER32, ON/OFF)` on a stream with a bad Adler-32 | [x] |
| 147 | `R-LOW` + A10 | `png_set_keep_unknown_chunks` for `HANDLE_CHUNK_AS_DEFAULT`/`NEVER`/`IF_SAFE`/`ALWAYS` × known-but-unhandled and truly unknown chunk types | [x] |
| 148 | `R-LOW` + A10 | `png_set_read_user_chunk_fn` returning `0` (not handled), `1` (handled) | [x] |
| 149 | — | `png_set_strip_error_numbers`: **N/A in this build.** `PNG_ERROR_NUMBERS_SUPPORTED` is not defined in `c_src/include/pnglibconf.h`, so neither `.so` exports the symbol (`comm` against `nm -D` confirms it is absent from *both*). There is no entry point to drive, and symbol parity is unaffected. | [—] |
| 150 | `R-LOW` + A10 | `png_set_mem_fn` with custom malloc/free (allocation call sequence compared) | [x] |
| 151 | `R-LOW` + A10 | `png_set_sig_bytes(n)` for n = 0..8 with the caller having pre-consumed n bytes | [x] |
| 152 | `R-LOW` + A8 | read a stream containing every ancillary chunk; all `png_get_*` values compared field by field | [x] |
| 153 | `R-LOW` + A8 | `png_get_unknown_chunks` after reading with `HANDLE_CHUNK_ALWAYS`; count, names, locations and data compared | [x] |
| 154 | `R-LOW` + A8 | ancillary chunks placed **after** `IDAT` (`tEXt`, `tIME`, `eXIf`) | [x] |
| 155 | `R-LOW` + A8 | `IDAT` split across 1, 2, 5, 100 chunks; and a zero-length `IDAT` chunk in the middle | [x] |
| 156 | round trip | write with `W-LOW` then read back with `R-LOW`, all 15 A1 combos × interlace, comparing both the encoded bytes and the decoded rows | [x] |
| 157 | round trip | write with `W-PNG` then read with `R-PNG` using the inverse transform mask | [x] |
| 158 | round trip | `W-SIMPLE` then `R-SIMPLE` over the full `PNG_FORMAT_*` cross-product | [x] |
| 159 | `png_read_update_info` / `png_get_rowbytes` | after each A5 transform, the reported `rowbytes`/`channels`/`bit_depth`/`color_type` compared | [x] |
| 160 | `png_info` lifecycle | `png_create_info_struct`, `png_info_init_3`, `png_free_data` with every `PNG_FREE_*` mask bit, `png_data_freer` with both valid freers, `png_destroy_read_struct`/`_write_struct` in all pointer-NULL combinations | [x] |
| 161 | `png_convert_to_rfc1123_buffer` / `png_convert_from_time_t` / `png_get_tIME` | random valid `png_time` values and `time_t` values | [x] |
| 162 | `png_permit_mng_features` | `PNG_FLAG_MNG_EMPTY_PLTE`, `_MNG_FILTER_64`, both, neither | [x] |
| 163 | `png_get_io_state` / `png_get_io_chunk_type` | sampled at every read and write callback invocation; full sequence compared | [x] |
| 164 | `png_build_grayscale_palette` | bit depths 1, 2, 4, 8 | [x] |
| 165 | `png_get_palette_max` | after reading palette images with indices at and below `num_palette-1` | [x] |

---

Rows 1–5 and 160–165 exercise the exported low-level primitives directly. Rows
6–82 drive the writer, 83–155 the reader, 156–158 the composed round trip; every
row is run over randomized inputs rather than one hand-picked value.
