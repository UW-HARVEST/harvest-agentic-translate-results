# SYMBOLS.md — exported-symbol parity (C `libpng.so` vs Rust `liblibpng.so`)

Generated mechanically with:
```
nm -D --defined-only c_src/build/libpng.so                   | awk '$2!="U"{print $3}' | sort -u
nm -D --defined-only translation/target/release/liblibpng.so  | awk '$2!="U"{print $3}' | sort -u
```

C exports: 384    Rust exports: 384

**Missing from Rust: 0.  Extra in Rust: 0.  `comm -3` diff is EMPTY.**

Undefined (imported) symbols of the Rust `.so` are only libc / libm / libgcc_s
unwinder / zlib (`crc32 deflate deflateEnd deflateInit2_ deflateReset inflate`
`inflateEnd inflateInit2_ inflateReset inflateReset2 zlibVersion`) — the same
external surface the reference C build links against. 0 missing non-libc symbols.

`T` = code (function), `D`/`B`/`R` = data object.

## Verified by `run_verification.sh symbols`

```
=== Phase A / D: exported-symbol parity ===
C exports:    384
Rust exports: 384
missing from Rust: none
extra in Rust: none

=== undefined (imported) symbols of the Rust .so ===
all undefined symbols are libc / libm / libgcc_s / zlib: OK
```

Two symbols are DECLARED in `png.h` but exported by NEITHER library, because
they are `#ifdef`-ed out of this configuration.  They are therefore not part of
the ABI and not a parity gap:

| symbol | why absent |
|--------|------------|
| `png_err` | only declared when `PNG_ERROR_TEXT_SUPPORTED` is OFF; it is ON here |
| `png_set_strip_error_numbers` | `PNG_ERROR_NUMBERS_SUPPORTED` is OFF |

`tests/gen_api.py` records both in its `NOT_EXPORTED` set so the generated
`Api` table matches the real export table, and building an `Api` resolves all
308 entries in BOTH `.so` files — which is itself an independent, run-time
check of symbol parity for every entry point the tests drive.

| # | symbol | nm type (C) | in C .so | in Rust .so |
|---|--------|-------------|----------|-------------|
| 1 | `png_XYZ_from_xy` | T | yes | yes |
| 2 | `png_access_version_number` | T | yes | yes |
| 3 | `png_app_error` | T | yes | yes |
| 4 | `png_app_warning` | T | yes | yes |
| 5 | `png_ascii_from_fixed` | T | yes | yes |
| 6 | `png_ascii_from_fp` | T | yes | yes |
| 7 | `png_benign_error` | T | yes | yes |
| 8 | `png_build_gamma_table` | T | yes | yes |
| 9 | `png_build_grayscale_palette` | T | yes | yes |
| 10 | `png_calculate_crc` | T | yes | yes |
| 11 | `png_calloc` | T | yes | yes |
| 12 | `png_check_IHDR` | T | yes | yes |
| 13 | `png_check_fp_number` | T | yes | yes |
| 14 | `png_check_fp_string` | T | yes | yes |
| 15 | `png_check_keyword` | T | yes | yes |
| 16 | `png_chunk_benign_error` | T | yes | yes |
| 17 | `png_chunk_error` | T | yes | yes |
| 18 | `png_chunk_report` | T | yes | yes |
| 19 | `png_chunk_unknown_handling` | T | yes | yes |
| 20 | `png_chunk_warning` | T | yes | yes |
| 21 | `png_combine_row` | T | yes | yes |
| 22 | `png_compress_IDAT` | T | yes | yes |
| 23 | `png_convert_from_struct_tm` | T | yes | yes |
| 24 | `png_convert_from_time_t` | T | yes | yes |
| 25 | `png_convert_to_rfc1123` | T | yes | yes |
| 26 | `png_convert_to_rfc1123_buffer` | T | yes | yes |
| 27 | `png_crc_finish` | T | yes | yes |
| 28 | `png_crc_read` | T | yes | yes |
| 29 | `png_create_info_struct` | T | yes | yes |
| 30 | `png_create_png_struct` | T | yes | yes |
| 31 | `png_create_read_struct` | T | yes | yes |
| 32 | `png_create_read_struct_2` | T | yes | yes |
| 33 | `png_create_write_struct` | T | yes | yes |
| 34 | `png_create_write_struct_2` | T | yes | yes |
| 35 | `png_data_freer` | T | yes | yes |
| 36 | `png_default_flush` | T | yes | yes |
| 37 | `png_default_read_data` | T | yes | yes |
| 38 | `png_default_write_data` | T | yes | yes |
| 39 | `png_destroy_gamma_table` | T | yes | yes |
| 40 | `png_destroy_info_struct` | T | yes | yes |
| 41 | `png_destroy_png_struct` | T | yes | yes |
| 42 | `png_destroy_read_struct` | T | yes | yes |
| 43 | `png_destroy_write_struct` | T | yes | yes |
| 44 | `png_do_bgr` | T | yes | yes |
| 45 | `png_do_check_palette_indexes` | T | yes | yes |
| 46 | `png_do_invert` | T | yes | yes |
| 47 | `png_do_packswap` | T | yes | yes |
| 48 | `png_do_read_interlace` | T | yes | yes |
| 49 | `png_do_read_transformations` | T | yes | yes |
| 50 | `png_do_strip_channel` | T | yes | yes |
| 51 | `png_do_swap` | T | yes | yes |
| 52 | `png_do_write_interlace` | T | yes | yes |
| 53 | `png_do_write_transformations` | T | yes | yes |
| 54 | `png_error` | T | yes | yes |
| 55 | `png_fixed` | T | yes | yes |
| 56 | `png_fixed_ITU` | T | yes | yes |
| 57 | `png_fixed_error` | T | yes | yes |
| 58 | `png_flush` | T | yes | yes |
| 59 | `png_format_number` | T | yes | yes |
| 60 | `png_formatted_warning` | T | yes | yes |
| 61 | `png_free` | T | yes | yes |
| 62 | `png_free_buffer_list` | T | yes | yes |
| 63 | `png_free_data` | T | yes | yes |
| 64 | `png_free_default` | T | yes | yes |
| 65 | `png_free_jmpbuf` | T | yes | yes |
| 66 | `png_gamma_16bit_correct` | T | yes | yes |
| 67 | `png_gamma_8bit_correct` | T | yes | yes |
| 68 | `png_gamma_correct` | T | yes | yes |
| 69 | `png_gamma_significant` | T | yes | yes |
| 70 | `png_get_IHDR` | T | yes | yes |
| 71 | `png_get_PLTE` | T | yes | yes |
| 72 | `png_get_bKGD` | T | yes | yes |
| 73 | `png_get_bit_depth` | T | yes | yes |
| 74 | `png_get_cHRM` | T | yes | yes |
| 75 | `png_get_cHRM_XYZ` | T | yes | yes |
| 76 | `png_get_cHRM_XYZ_fixed` | T | yes | yes |
| 77 | `png_get_cHRM_fixed` | T | yes | yes |
| 78 | `png_get_cICP` | T | yes | yes |
| 79 | `png_get_cLLI` | T | yes | yes |
| 80 | `png_get_cLLI_fixed` | T | yes | yes |
| 81 | `png_get_channels` | T | yes | yes |
| 82 | `png_get_chunk_cache_max` | T | yes | yes |
| 83 | `png_get_chunk_malloc_max` | T | yes | yes |
| 84 | `png_get_color_type` | T | yes | yes |
| 85 | `png_get_compression_buffer_size` | T | yes | yes |
| 86 | `png_get_compression_type` | T | yes | yes |
| 87 | `png_get_copyright` | T | yes | yes |
| 88 | `png_get_current_pass_number` | T | yes | yes |
| 89 | `png_get_current_row_number` | T | yes | yes |
| 90 | `png_get_eXIf` | T | yes | yes |
| 91 | `png_get_eXIf_1` | T | yes | yes |
| 92 | `png_get_error_ptr` | T | yes | yes |
| 93 | `png_get_filter_type` | T | yes | yes |
| 94 | `png_get_gAMA` | T | yes | yes |
| 95 | `png_get_gAMA_fixed` | T | yes | yes |
| 96 | `png_get_hIST` | T | yes | yes |
| 97 | `png_get_header_ver` | T | yes | yes |
| 98 | `png_get_header_version` | T | yes | yes |
| 99 | `png_get_iCCP` | T | yes | yes |
| 100 | `png_get_image_height` | T | yes | yes |
| 101 | `png_get_image_width` | T | yes | yes |
| 102 | `png_get_int_32` | T | yes | yes |
| 103 | `png_get_interlace_type` | T | yes | yes |
| 104 | `png_get_io_chunk_type` | T | yes | yes |
| 105 | `png_get_io_ptr` | T | yes | yes |
| 106 | `png_get_io_state` | T | yes | yes |
| 107 | `png_get_libpng_ver` | T | yes | yes |
| 108 | `png_get_mDCV` | T | yes | yes |
| 109 | `png_get_mDCV_fixed` | T | yes | yes |
| 110 | `png_get_mem_ptr` | T | yes | yes |
| 111 | `png_get_oFFs` | T | yes | yes |
| 112 | `png_get_pCAL` | T | yes | yes |
| 113 | `png_get_pHYs` | T | yes | yes |
| 114 | `png_get_pHYs_dpi` | T | yes | yes |
| 115 | `png_get_palette_max` | T | yes | yes |
| 116 | `png_get_pixel_aspect_ratio` | T | yes | yes |
| 117 | `png_get_pixel_aspect_ratio_fixed` | T | yes | yes |
| 118 | `png_get_pixels_per_inch` | T | yes | yes |
| 119 | `png_get_pixels_per_meter` | T | yes | yes |
| 120 | `png_get_progressive_ptr` | T | yes | yes |
| 121 | `png_get_rgb_to_gray_status` | T | yes | yes |
| 122 | `png_get_rowbytes` | T | yes | yes |
| 123 | `png_get_rows` | T | yes | yes |
| 124 | `png_get_sBIT` | T | yes | yes |
| 125 | `png_get_sCAL` | T | yes | yes |
| 126 | `png_get_sCAL_fixed` | T | yes | yes |
| 127 | `png_get_sCAL_s` | T | yes | yes |
| 128 | `png_get_sPLT` | T | yes | yes |
| 129 | `png_get_sRGB` | T | yes | yes |
| 130 | `png_get_signature` | T | yes | yes |
| 131 | `png_get_tIME` | T | yes | yes |
| 132 | `png_get_tRNS` | T | yes | yes |
| 133 | `png_get_text` | T | yes | yes |
| 134 | `png_get_uint_16` | T | yes | yes |
| 135 | `png_get_uint_31` | T | yes | yes |
| 136 | `png_get_uint_32` | T | yes | yes |
| 137 | `png_get_unknown_chunks` | T | yes | yes |
| 138 | `png_get_user_chunk_ptr` | T | yes | yes |
| 139 | `png_get_user_height_max` | T | yes | yes |
| 140 | `png_get_user_transform_ptr` | T | yes | yes |
| 141 | `png_get_user_width_max` | T | yes | yes |
| 142 | `png_get_valid` | T | yes | yes |
| 143 | `png_get_x_offset_inches` | T | yes | yes |
| 144 | `png_get_x_offset_inches_fixed` | T | yes | yes |
| 145 | `png_get_x_offset_microns` | T | yes | yes |
| 146 | `png_get_x_offset_pixels` | T | yes | yes |
| 147 | `png_get_x_pixels_per_inch` | T | yes | yes |
| 148 | `png_get_x_pixels_per_meter` | T | yes | yes |
| 149 | `png_get_y_offset_inches` | T | yes | yes |
| 150 | `png_get_y_offset_inches_fixed` | T | yes | yes |
| 151 | `png_get_y_offset_microns` | T | yes | yes |
| 152 | `png_get_y_offset_pixels` | T | yes | yes |
| 153 | `png_get_y_pixels_per_inch` | T | yes | yes |
| 154 | `png_get_y_pixels_per_meter` | T | yes | yes |
| 155 | `png_handle_as_unknown` | T | yes | yes |
| 156 | `png_handle_chunk` | T | yes | yes |
| 157 | `png_handle_unknown` | T | yes | yes |
| 158 | `png_icc_check_header` | T | yes | yes |
| 159 | `png_icc_check_length` | T | yes | yes |
| 160 | `png_icc_check_tag_table` | T | yes | yes |
| 161 | `png_image_begin_read_from_file` | T | yes | yes |
| 162 | `png_image_begin_read_from_memory` | T | yes | yes |
| 163 | `png_image_begin_read_from_stdio` | T | yes | yes |
| 164 | `png_image_error` | T | yes | yes |
| 165 | `png_image_finish_read` | T | yes | yes |
| 166 | `png_image_free` | T | yes | yes |
| 167 | `png_image_write_to_file` | T | yes | yes |
| 168 | `png_image_write_to_memory` | T | yes | yes |
| 169 | `png_image_write_to_stdio` | T | yes | yes |
| 170 | `png_info_init_3` | T | yes | yes |
| 171 | `png_init_io` | T | yes | yes |
| 172 | `png_init_read_transformations` | T | yes | yes |
| 173 | `png_longjmp` | T | yes | yes |
| 174 | `png_malloc` | T | yes | yes |
| 175 | `png_malloc_array` | T | yes | yes |
| 176 | `png_malloc_base` | T | yes | yes |
| 177 | `png_malloc_default` | T | yes | yes |
| 178 | `png_malloc_warn` | T | yes | yes |
| 179 | `png_muldiv` | T | yes | yes |
| 180 | `png_permit_mng_features` | T | yes | yes |
| 181 | `png_process_IDAT_data` | T | yes | yes |
| 182 | `png_process_data` | T | yes | yes |
| 183 | `png_process_data_pause` | T | yes | yes |
| 184 | `png_process_data_skip` | T | yes | yes |
| 185 | `png_process_some_data` | T | yes | yes |
| 186 | `png_progressive_combine_row` | T | yes | yes |
| 187 | `png_push_fill_buffer` | T | yes | yes |
| 188 | `png_push_have_end` | T | yes | yes |
| 189 | `png_push_have_info` | T | yes | yes |
| 190 | `png_push_have_row` | T | yes | yes |
| 191 | `png_push_process_row` | T | yes | yes |
| 192 | `png_push_read_IDAT` | T | yes | yes |
| 193 | `png_push_read_chunk` | T | yes | yes |
| 194 | `png_push_read_sig` | T | yes | yes |
| 195 | `png_push_restore_buffer` | T | yes | yes |
| 196 | `png_push_save_buffer` | T | yes | yes |
| 197 | `png_read_IDAT_data` | T | yes | yes |
| 198 | `png_read_chunk_header` | T | yes | yes |
| 199 | `png_read_data` | T | yes | yes |
| 200 | `png_read_end` | T | yes | yes |
| 201 | `png_read_filter_row` | T | yes | yes |
| 202 | `png_read_finish_IDAT` | T | yes | yes |
| 203 | `png_read_finish_row` | T | yes | yes |
| 204 | `png_read_image` | T | yes | yes |
| 205 | `png_read_info` | T | yes | yes |
| 206 | `png_read_png` | T | yes | yes |
| 207 | `png_read_push_finish_row` | T | yes | yes |
| 208 | `png_read_row` | T | yes | yes |
| 209 | `png_read_rows` | T | yes | yes |
| 210 | `png_read_sig` | T | yes | yes |
| 211 | `png_read_start_row` | T | yes | yes |
| 212 | `png_read_transform_info` | T | yes | yes |
| 213 | `png_read_update_info` | T | yes | yes |
| 214 | `png_realloc_array` | T | yes | yes |
| 215 | `png_reciprocal` | T | yes | yes |
| 216 | `png_reciprocal2` | T | yes | yes |
| 217 | `png_reset_crc` | T | yes | yes |
| 218 | `png_reset_zstream` | T | yes | yes |
| 219 | `png_resolve_file_gamma` | T | yes | yes |
| 220 | `png_sRGB_base` | R | yes | yes |
| 221 | `png_sRGB_delta` | R | yes | yes |
| 222 | `png_sRGB_table` | R | yes | yes |
| 223 | `png_safe_error` | T | yes | yes |
| 224 | `png_safe_execute` | T | yes | yes |
| 225 | `png_safe_warning` | T | yes | yes |
| 226 | `png_safecat` | T | yes | yes |
| 227 | `png_save_int_32` | T | yes | yes |
| 228 | `png_save_uint_16` | T | yes | yes |
| 229 | `png_save_uint_32` | T | yes | yes |
| 230 | `png_set_IHDR` | T | yes | yes |
| 231 | `png_set_PLTE` | T | yes | yes |
| 232 | `png_set_add_alpha` | T | yes | yes |
| 233 | `png_set_alpha_mode` | T | yes | yes |
| 234 | `png_set_alpha_mode_fixed` | T | yes | yes |
| 235 | `png_set_bKGD` | T | yes | yes |
| 236 | `png_set_background` | T | yes | yes |
| 237 | `png_set_background_fixed` | T | yes | yes |
| 238 | `png_set_benign_errors` | T | yes | yes |
| 239 | `png_set_bgr` | T | yes | yes |
| 240 | `png_set_cHRM` | T | yes | yes |
| 241 | `png_set_cHRM_XYZ` | T | yes | yes |
| 242 | `png_set_cHRM_XYZ_fixed` | T | yes | yes |
| 243 | `png_set_cHRM_fixed` | T | yes | yes |
| 244 | `png_set_cICP` | T | yes | yes |
| 245 | `png_set_cLLI` | T | yes | yes |
| 246 | `png_set_cLLI_fixed` | T | yes | yes |
| 247 | `png_set_check_for_invalid_index` | T | yes | yes |
| 248 | `png_set_chunk_cache_max` | T | yes | yes |
| 249 | `png_set_chunk_malloc_max` | T | yes | yes |
| 250 | `png_set_compression_buffer_size` | T | yes | yes |
| 251 | `png_set_compression_level` | T | yes | yes |
| 252 | `png_set_compression_mem_level` | T | yes | yes |
| 253 | `png_set_compression_method` | T | yes | yes |
| 254 | `png_set_compression_strategy` | T | yes | yes |
| 255 | `png_set_compression_window_bits` | T | yes | yes |
| 256 | `png_set_crc_action` | T | yes | yes |
| 257 | `png_set_eXIf` | T | yes | yes |
| 258 | `png_set_eXIf_1` | T | yes | yes |
| 259 | `png_set_error_fn` | T | yes | yes |
| 260 | `png_set_expand` | T | yes | yes |
| 261 | `png_set_expand_16` | T | yes | yes |
| 262 | `png_set_expand_gray_1_2_4_to_8` | T | yes | yes |
| 263 | `png_set_filler` | T | yes | yes |
| 264 | `png_set_filter` | T | yes | yes |
| 265 | `png_set_filter_heuristics` | T | yes | yes |
| 266 | `png_set_filter_heuristics_fixed` | T | yes | yes |
| 267 | `png_set_flush` | T | yes | yes |
| 268 | `png_set_gAMA` | T | yes | yes |
| 269 | `png_set_gAMA_fixed` | T | yes | yes |
| 270 | `png_set_gamma` | T | yes | yes |
| 271 | `png_set_gamma_fixed` | T | yes | yes |
| 272 | `png_set_gray_to_rgb` | T | yes | yes |
| 273 | `png_set_hIST` | T | yes | yes |
| 274 | `png_set_iCCP` | T | yes | yes |
| 275 | `png_set_interlace_handling` | T | yes | yes |
| 276 | `png_set_invalid` | T | yes | yes |
| 277 | `png_set_invert_alpha` | T | yes | yes |
| 278 | `png_set_invert_mono` | T | yes | yes |
| 279 | `png_set_keep_unknown_chunks` | T | yes | yes |
| 280 | `png_set_longjmp_fn` | T | yes | yes |
| 281 | `png_set_mDCV` | T | yes | yes |
| 282 | `png_set_mDCV_fixed` | T | yes | yes |
| 283 | `png_set_mem_fn` | T | yes | yes |
| 284 | `png_set_oFFs` | T | yes | yes |
| 285 | `png_set_option` | T | yes | yes |
| 286 | `png_set_pCAL` | T | yes | yes |
| 287 | `png_set_pHYs` | T | yes | yes |
| 288 | `png_set_packing` | T | yes | yes |
| 289 | `png_set_packswap` | T | yes | yes |
| 290 | `png_set_palette_to_rgb` | T | yes | yes |
| 291 | `png_set_progressive_read_fn` | T | yes | yes |
| 292 | `png_set_quantize` | T | yes | yes |
| 293 | `png_set_read_fn` | T | yes | yes |
| 294 | `png_set_read_status_fn` | T | yes | yes |
| 295 | `png_set_read_user_chunk_fn` | T | yes | yes |
| 296 | `png_set_read_user_transform_fn` | T | yes | yes |
| 297 | `png_set_rgb_coefficients` | T | yes | yes |
| 298 | `png_set_rgb_to_gray` | T | yes | yes |
| 299 | `png_set_rgb_to_gray_fixed` | T | yes | yes |
| 300 | `png_set_rows` | T | yes | yes |
| 301 | `png_set_sBIT` | T | yes | yes |
| 302 | `png_set_sCAL` | T | yes | yes |
| 303 | `png_set_sCAL_fixed` | T | yes | yes |
| 304 | `png_set_sCAL_s` | T | yes | yes |
| 305 | `png_set_sPLT` | T | yes | yes |
| 306 | `png_set_sRGB` | T | yes | yes |
| 307 | `png_set_sRGB_gAMA_and_cHRM` | T | yes | yes |
| 308 | `png_set_scale_16` | T | yes | yes |
| 309 | `png_set_shift` | T | yes | yes |
| 310 | `png_set_sig_bytes` | T | yes | yes |
| 311 | `png_set_strip_16` | T | yes | yes |
| 312 | `png_set_strip_alpha` | T | yes | yes |
| 313 | `png_set_swap` | T | yes | yes |
| 314 | `png_set_swap_alpha` | T | yes | yes |
| 315 | `png_set_tIME` | T | yes | yes |
| 316 | `png_set_tRNS` | T | yes | yes |
| 317 | `png_set_tRNS_to_alpha` | T | yes | yes |
| 318 | `png_set_text` | T | yes | yes |
| 319 | `png_set_text_2` | T | yes | yes |
| 320 | `png_set_text_compression_level` | T | yes | yes |
| 321 | `png_set_text_compression_mem_level` | T | yes | yes |
| 322 | `png_set_text_compression_method` | T | yes | yes |
| 323 | `png_set_text_compression_strategy` | T | yes | yes |
| 324 | `png_set_text_compression_window_bits` | T | yes | yes |
| 325 | `png_set_unknown_chunk_location` | T | yes | yes |
| 326 | `png_set_unknown_chunks` | T | yes | yes |
| 327 | `png_set_user_limits` | T | yes | yes |
| 328 | `png_set_user_transform_info` | T | yes | yes |
| 329 | `png_set_write_fn` | T | yes | yes |
| 330 | `png_set_write_status_fn` | T | yes | yes |
| 331 | `png_set_write_user_transform_fn` | T | yes | yes |
| 332 | `png_sig_cmp` | T | yes | yes |
| 333 | `png_start_read_image` | T | yes | yes |
| 334 | `png_user_version_check` | T | yes | yes |
| 335 | `png_warning` | T | yes | yes |
| 336 | `png_warning_parameter` | T | yes | yes |
| 337 | `png_warning_parameter_signed` | T | yes | yes |
| 338 | `png_warning_parameter_unsigned` | T | yes | yes |
| 339 | `png_write_IEND` | T | yes | yes |
| 340 | `png_write_IHDR` | T | yes | yes |
| 341 | `png_write_PLTE` | T | yes | yes |
| 342 | `png_write_bKGD` | T | yes | yes |
| 343 | `png_write_cHRM_fixed` | T | yes | yes |
| 344 | `png_write_cICP` | T | yes | yes |
| 345 | `png_write_cLLI_fixed` | T | yes | yes |
| 346 | `png_write_chunk` | T | yes | yes |
| 347 | `png_write_chunk_data` | T | yes | yes |
| 348 | `png_write_chunk_end` | T | yes | yes |
| 349 | `png_write_chunk_start` | T | yes | yes |
| 350 | `png_write_data` | T | yes | yes |
| 351 | `png_write_eXIf` | T | yes | yes |
| 352 | `png_write_end` | T | yes | yes |
| 353 | `png_write_find_filter` | T | yes | yes |
| 354 | `png_write_finish_row` | T | yes | yes |
| 355 | `png_write_flush` | T | yes | yes |
| 356 | `png_write_gAMA_fixed` | T | yes | yes |
| 357 | `png_write_hIST` | T | yes | yes |
| 358 | `png_write_iCCP` | T | yes | yes |
| 359 | `png_write_iTXt` | T | yes | yes |
| 360 | `png_write_image` | T | yes | yes |
| 361 | `png_write_info` | T | yes | yes |
| 362 | `png_write_info_before_PLTE` | T | yes | yes |
| 363 | `png_write_mDCV_fixed` | T | yes | yes |
| 364 | `png_write_oFFs` | T | yes | yes |
| 365 | `png_write_pCAL` | T | yes | yes |
| 366 | `png_write_pHYs` | T | yes | yes |
| 367 | `png_write_png` | T | yes | yes |
| 368 | `png_write_row` | T | yes | yes |
| 369 | `png_write_rows` | T | yes | yes |
| 370 | `png_write_sBIT` | T | yes | yes |
| 371 | `png_write_sCAL_s` | T | yes | yes |
| 372 | `png_write_sPLT` | T | yes | yes |
| 373 | `png_write_sRGB` | T | yes | yes |
| 374 | `png_write_sig` | T | yes | yes |
| 375 | `png_write_start_row` | T | yes | yes |
| 376 | `png_write_tEXt` | T | yes | yes |
| 377 | `png_write_tIME` | T | yes | yes |
| 378 | `png_write_tRNS` | T | yes | yes |
| 379 | `png_write_zTXt` | T | yes | yes |
| 380 | `png_xy_from_XYZ` | T | yes | yes |
| 381 | `png_zalloc` | T | yes | yes |
| 382 | `png_zfree` | T | yes | yes |
| 383 | `png_zlib_inflate` | T | yes | yes |
| 384 | `png_zstream_error` | T | yes | yes |
