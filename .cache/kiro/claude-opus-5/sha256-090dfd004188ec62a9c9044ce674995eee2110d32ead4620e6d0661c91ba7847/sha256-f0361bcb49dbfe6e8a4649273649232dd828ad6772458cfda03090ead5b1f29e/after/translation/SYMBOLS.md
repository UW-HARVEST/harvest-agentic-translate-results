# SYMBOLS.md — exported-symbol parity (C `libpng.so` vs Rust `liblibpng.so`)

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libpng.so                    | awk '$2~/^[TDBR]$/{print $3}' | sort -u
nm -D --defined-only translation/target/release/liblibpng.so  | awk '$2~/^[TDBR]$/{print $3}' | sort -u
```

## Result

| | count |
|---|---|
| C exported symbols | **384** |
| Rust exported symbols | **384** |
| Missing from Rust (`comm -23`) | **0** |
| Extra in Rust (`comm -13`) | **0** |
| Undefined `png_*` symbols in the Rust `.so` | **0** |
| Exercised **directly** by the differential tests | **335** |
| Exercised **only indirectly**, through the pipeline | **49** |

The symbol diff is empty in both directions, so no source file was skipped
by the translation and no wrapper had to be added: every symbol the C `.so`
exports already had a `#[no_mangle] extern "C"` counterpart in the Rust
`.so` with the identical name, including the macro-generated ones.

This build has no symbol-visibility script, so libpng's *internal*
(`PNG_INTERNAL_FUNCTION` / `PNG_INTERNAL_DATA`) helpers are exported too and
are therefore part of the ABI under test — not just the 249 documented
`PNG_EXPORT` entry points. They are driven directly wherever their contract
permits it (see the `ll/*` cases), which is what "exercise the LOW-LEVEL
entry points directly" requires.

### Undefined imports in the Rust `.so`

`nm -D --undefined-only` lists only libc (`malloc`, `memcpy`, `_setjmp`,
`longjmp`, `pow`, `floor`, `gmtime`, stdio, …), zlib (`deflate*`, `inflate*`,
`crc32`) and the Rust unwinder/TLS runtime. That is the same set the C build
imports, plus the Rust runtime. No libpng symbol is left undefined.

> Note on the reference build: `c_src/CMakeLists.txt` links only
> `ZLIB::ZLIB`, so the C `.so` itself leaves `floor` and `pow` undefined
> (upstream libpng's own build adds `-lm`). `c_src/` is read-only for this
> task, so the test driver links libm instead — a `dlopen`ed object resolves
> undefined symbols from the global scope, which includes the executable and
> its dependencies. Without that the C `.so` fails to load with
> "undefined symbol: floor" and every floating-point path (gamma, cHRM,
> sCAL, the simplified API) would be untestable.

### Symbols reached only through the pipeline

These 49 are deep-pipeline internals that require a `png_struct` already in
a specific mid-operation state (a partially decoded row, a claimed zstream, a
colorspace under construction). Calling them out of context is undefined in
the C too, so they are covered by driving the operations that call them
rather than by direct invocation:

```
png_build_gamma_table png_combine_row png_compress_IDAT png_crc_finish 
png_crc_read png_destroy_gamma_table png_do_check_palette_indexes 
png_do_read_transformations png_do_write_transformations 
png_free_buffer_list png_free_jmpbuf png_handle_chunk png_handle_unknown 
png_icc_check_header png_icc_check_length png_icc_check_tag_table 
png_image_error png_init_read_transformations png_longjmp 
png_process_IDAT_data png_process_some_data png_push_fill_buffer 
png_push_have_end png_push_have_info png_push_have_row png_push_process_row 
png_push_read_IDAT png_push_read_chunk png_push_read_sig 
png_push_restore_buffer png_push_save_buffer png_read_IDAT_data 
png_read_chunk_header png_read_finish_IDAT png_read_finish_row 
png_read_push_finish_row png_read_sig png_read_start_row 
png_read_transform_info png_resolve_file_gamma png_safe_error 
png_safe_execute png_safe_warning png_set_rgb_coefficients 
png_write_find_filter png_write_finish_row png_write_start_row 
png_zlib_inflate png_zstream_error 
```

## Full symbol table

`in C` / `in Rust` are literal set-membership results from the two `nm -D`
runs; `called directly` records whether a differential test invokes the
symbol itself rather than only reaching it through a higher-level operation.

| # | symbol | in C | in Rust | called directly |
|---|--------|------|---------|-----------------|
| 1 | `png_XYZ_from_xy` | yes | yes | yes |
| 2 | `png_access_version_number` | yes | yes | yes |
| 3 | `png_app_error` | yes | yes | yes |
| 4 | `png_app_warning` | yes | yes | yes |
| 5 | `png_ascii_from_fixed` | yes | yes | yes |
| 6 | `png_ascii_from_fp` | yes | yes | yes |
| 7 | `png_benign_error` | yes | yes | yes |
| 8 | `png_build_gamma_table` | yes | yes | via pipeline |
| 9 | `png_build_grayscale_palette` | yes | yes | yes |
| 10 | `png_calculate_crc` | yes | yes | yes |
| 11 | `png_calloc` | yes | yes | yes |
| 12 | `png_check_IHDR` | yes | yes | yes |
| 13 | `png_check_fp_number` | yes | yes | yes |
| 14 | `png_check_fp_string` | yes | yes | yes |
| 15 | `png_check_keyword` | yes | yes | yes |
| 16 | `png_chunk_benign_error` | yes | yes | yes |
| 17 | `png_chunk_error` | yes | yes | yes |
| 18 | `png_chunk_report` | yes | yes | yes |
| 19 | `png_chunk_unknown_handling` | yes | yes | yes |
| 20 | `png_chunk_warning` | yes | yes | yes |
| 21 | `png_combine_row` | yes | yes | via pipeline |
| 22 | `png_compress_IDAT` | yes | yes | via pipeline |
| 23 | `png_convert_from_struct_tm` | yes | yes | yes |
| 24 | `png_convert_from_time_t` | yes | yes | yes |
| 25 | `png_convert_to_rfc1123` | yes | yes | yes |
| 26 | `png_convert_to_rfc1123_buffer` | yes | yes | yes |
| 27 | `png_crc_finish` | yes | yes | via pipeline |
| 28 | `png_crc_read` | yes | yes | via pipeline |
| 29 | `png_create_info_struct` | yes | yes | yes |
| 30 | `png_create_png_struct` | yes | yes | yes |
| 31 | `png_create_read_struct` | yes | yes | yes |
| 32 | `png_create_read_struct_2` | yes | yes | yes |
| 33 | `png_create_write_struct` | yes | yes | yes |
| 34 | `png_create_write_struct_2` | yes | yes | yes |
| 35 | `png_data_freer` | yes | yes | yes |
| 36 | `png_default_flush` | yes | yes | yes |
| 37 | `png_default_read_data` | yes | yes | yes |
| 38 | `png_default_write_data` | yes | yes | yes |
| 39 | `png_destroy_gamma_table` | yes | yes | via pipeline |
| 40 | `png_destroy_info_struct` | yes | yes | yes |
| 41 | `png_destroy_png_struct` | yes | yes | yes |
| 42 | `png_destroy_read_struct` | yes | yes | yes |
| 43 | `png_destroy_write_struct` | yes | yes | yes |
| 44 | `png_do_bgr` | yes | yes | yes |
| 45 | `png_do_check_palette_indexes` | yes | yes | via pipeline |
| 46 | `png_do_invert` | yes | yes | yes |
| 47 | `png_do_packswap` | yes | yes | yes |
| 48 | `png_do_read_interlace` | yes | yes | yes |
| 49 | `png_do_read_transformations` | yes | yes | via pipeline |
| 50 | `png_do_strip_channel` | yes | yes | yes |
| 51 | `png_do_swap` | yes | yes | yes |
| 52 | `png_do_write_interlace` | yes | yes | yes |
| 53 | `png_do_write_transformations` | yes | yes | via pipeline |
| 54 | `png_error` | yes | yes | yes |
| 55 | `png_fixed` | yes | yes | yes |
| 56 | `png_fixed_ITU` | yes | yes | yes |
| 57 | `png_fixed_error` | yes | yes | yes |
| 58 | `png_flush` | yes | yes | yes |
| 59 | `png_format_number` | yes | yes | yes |
| 60 | `png_formatted_warning` | yes | yes | yes |
| 61 | `png_free` | yes | yes | yes |
| 62 | `png_free_buffer_list` | yes | yes | via pipeline |
| 63 | `png_free_data` | yes | yes | yes |
| 64 | `png_free_default` | yes | yes | yes |
| 65 | `png_free_jmpbuf` | yes | yes | via pipeline |
| 66 | `png_gamma_16bit_correct` | yes | yes | yes |
| 67 | `png_gamma_8bit_correct` | yes | yes | yes |
| 68 | `png_gamma_correct` | yes | yes | yes |
| 69 | `png_gamma_significant` | yes | yes | yes |
| 70 | `png_get_IHDR` | yes | yes | yes |
| 71 | `png_get_PLTE` | yes | yes | yes |
| 72 | `png_get_bKGD` | yes | yes | yes |
| 73 | `png_get_bit_depth` | yes | yes | yes |
| 74 | `png_get_cHRM` | yes | yes | yes |
| 75 | `png_get_cHRM_XYZ` | yes | yes | yes |
| 76 | `png_get_cHRM_XYZ_fixed` | yes | yes | yes |
| 77 | `png_get_cHRM_fixed` | yes | yes | yes |
| 78 | `png_get_cICP` | yes | yes | yes |
| 79 | `png_get_cLLI` | yes | yes | yes |
| 80 | `png_get_cLLI_fixed` | yes | yes | yes |
| 81 | `png_get_channels` | yes | yes | yes |
| 82 | `png_get_chunk_cache_max` | yes | yes | yes |
| 83 | `png_get_chunk_malloc_max` | yes | yes | yes |
| 84 | `png_get_color_type` | yes | yes | yes |
| 85 | `png_get_compression_buffer_size` | yes | yes | yes |
| 86 | `png_get_compression_type` | yes | yes | yes |
| 87 | `png_get_copyright` | yes | yes | yes |
| 88 | `png_get_current_pass_number` | yes | yes | yes |
| 89 | `png_get_current_row_number` | yes | yes | yes |
| 90 | `png_get_eXIf` | yes | yes | yes |
| 91 | `png_get_eXIf_1` | yes | yes | yes |
| 92 | `png_get_error_ptr` | yes | yes | yes |
| 93 | `png_get_filter_type` | yes | yes | yes |
| 94 | `png_get_gAMA` | yes | yes | yes |
| 95 | `png_get_gAMA_fixed` | yes | yes | yes |
| 96 | `png_get_hIST` | yes | yes | yes |
| 97 | `png_get_header_ver` | yes | yes | yes |
| 98 | `png_get_header_version` | yes | yes | yes |
| 99 | `png_get_iCCP` | yes | yes | yes |
| 100 | `png_get_image_height` | yes | yes | yes |
| 101 | `png_get_image_width` | yes | yes | yes |
| 102 | `png_get_int_32` | yes | yes | yes |
| 103 | `png_get_interlace_type` | yes | yes | yes |
| 104 | `png_get_io_chunk_type` | yes | yes | yes |
| 105 | `png_get_io_ptr` | yes | yes | yes |
| 106 | `png_get_io_state` | yes | yes | yes |
| 107 | `png_get_libpng_ver` | yes | yes | yes |
| 108 | `png_get_mDCV` | yes | yes | yes |
| 109 | `png_get_mDCV_fixed` | yes | yes | yes |
| 110 | `png_get_mem_ptr` | yes | yes | yes |
| 111 | `png_get_oFFs` | yes | yes | yes |
| 112 | `png_get_pCAL` | yes | yes | yes |
| 113 | `png_get_pHYs` | yes | yes | yes |
| 114 | `png_get_pHYs_dpi` | yes | yes | yes |
| 115 | `png_get_palette_max` | yes | yes | yes |
| 116 | `png_get_pixel_aspect_ratio` | yes | yes | yes |
| 117 | `png_get_pixel_aspect_ratio_fixed` | yes | yes | yes |
| 118 | `png_get_pixels_per_inch` | yes | yes | yes |
| 119 | `png_get_pixels_per_meter` | yes | yes | yes |
| 120 | `png_get_progressive_ptr` | yes | yes | yes |
| 121 | `png_get_rgb_to_gray_status` | yes | yes | yes |
| 122 | `png_get_rowbytes` | yes | yes | yes |
| 123 | `png_get_rows` | yes | yes | yes |
| 124 | `png_get_sBIT` | yes | yes | yes |
| 125 | `png_get_sCAL` | yes | yes | yes |
| 126 | `png_get_sCAL_fixed` | yes | yes | yes |
| 127 | `png_get_sCAL_s` | yes | yes | yes |
| 128 | `png_get_sPLT` | yes | yes | yes |
| 129 | `png_get_sRGB` | yes | yes | yes |
| 130 | `png_get_signature` | yes | yes | yes |
| 131 | `png_get_tIME` | yes | yes | yes |
| 132 | `png_get_tRNS` | yes | yes | yes |
| 133 | `png_get_text` | yes | yes | yes |
| 134 | `png_get_uint_16` | yes | yes | yes |
| 135 | `png_get_uint_31` | yes | yes | yes |
| 136 | `png_get_uint_32` | yes | yes | yes |
| 137 | `png_get_unknown_chunks` | yes | yes | yes |
| 138 | `png_get_user_chunk_ptr` | yes | yes | yes |
| 139 | `png_get_user_height_max` | yes | yes | yes |
| 140 | `png_get_user_transform_ptr` | yes | yes | yes |
| 141 | `png_get_user_width_max` | yes | yes | yes |
| 142 | `png_get_valid` | yes | yes | yes |
| 143 | `png_get_x_offset_inches` | yes | yes | yes |
| 144 | `png_get_x_offset_inches_fixed` | yes | yes | yes |
| 145 | `png_get_x_offset_microns` | yes | yes | yes |
| 146 | `png_get_x_offset_pixels` | yes | yes | yes |
| 147 | `png_get_x_pixels_per_inch` | yes | yes | yes |
| 148 | `png_get_x_pixels_per_meter` | yes | yes | yes |
| 149 | `png_get_y_offset_inches` | yes | yes | yes |
| 150 | `png_get_y_offset_inches_fixed` | yes | yes | yes |
| 151 | `png_get_y_offset_microns` | yes | yes | yes |
| 152 | `png_get_y_offset_pixels` | yes | yes | yes |
| 153 | `png_get_y_pixels_per_inch` | yes | yes | yes |
| 154 | `png_get_y_pixels_per_meter` | yes | yes | yes |
| 155 | `png_handle_as_unknown` | yes | yes | yes |
| 156 | `png_handle_chunk` | yes | yes | via pipeline |
| 157 | `png_handle_unknown` | yes | yes | via pipeline |
| 158 | `png_icc_check_header` | yes | yes | via pipeline |
| 159 | `png_icc_check_length` | yes | yes | via pipeline |
| 160 | `png_icc_check_tag_table` | yes | yes | via pipeline |
| 161 | `png_image_begin_read_from_file` | yes | yes | yes |
| 162 | `png_image_begin_read_from_memory` | yes | yes | yes |
| 163 | `png_image_begin_read_from_stdio` | yes | yes | yes |
| 164 | `png_image_error` | yes | yes | via pipeline |
| 165 | `png_image_finish_read` | yes | yes | yes |
| 166 | `png_image_free` | yes | yes | yes |
| 167 | `png_image_write_to_file` | yes | yes | yes |
| 168 | `png_image_write_to_memory` | yes | yes | yes |
| 169 | `png_image_write_to_stdio` | yes | yes | yes |
| 170 | `png_info_init_3` | yes | yes | yes |
| 171 | `png_init_io` | yes | yes | yes |
| 172 | `png_init_read_transformations` | yes | yes | via pipeline |
| 173 | `png_longjmp` | yes | yes | via pipeline |
| 174 | `png_malloc` | yes | yes | yes |
| 175 | `png_malloc_array` | yes | yes | yes |
| 176 | `png_malloc_base` | yes | yes | yes |
| 177 | `png_malloc_default` | yes | yes | yes |
| 178 | `png_malloc_warn` | yes | yes | yes |
| 179 | `png_muldiv` | yes | yes | yes |
| 180 | `png_permit_mng_features` | yes | yes | yes |
| 181 | `png_process_IDAT_data` | yes | yes | via pipeline |
| 182 | `png_process_data` | yes | yes | yes |
| 183 | `png_process_data_pause` | yes | yes | yes |
| 184 | `png_process_data_skip` | yes | yes | yes |
| 185 | `png_process_some_data` | yes | yes | via pipeline |
| 186 | `png_progressive_combine_row` | yes | yes | yes |
| 187 | `png_push_fill_buffer` | yes | yes | via pipeline |
| 188 | `png_push_have_end` | yes | yes | via pipeline |
| 189 | `png_push_have_info` | yes | yes | via pipeline |
| 190 | `png_push_have_row` | yes | yes | via pipeline |
| 191 | `png_push_process_row` | yes | yes | via pipeline |
| 192 | `png_push_read_IDAT` | yes | yes | via pipeline |
| 193 | `png_push_read_chunk` | yes | yes | via pipeline |
| 194 | `png_push_read_sig` | yes | yes | via pipeline |
| 195 | `png_push_restore_buffer` | yes | yes | via pipeline |
| 196 | `png_push_save_buffer` | yes | yes | via pipeline |
| 197 | `png_read_IDAT_data` | yes | yes | via pipeline |
| 198 | `png_read_chunk_header` | yes | yes | via pipeline |
| 199 | `png_read_data` | yes | yes | yes |
| 200 | `png_read_end` | yes | yes | yes |
| 201 | `png_read_filter_row` | yes | yes | yes |
| 202 | `png_read_finish_IDAT` | yes | yes | via pipeline |
| 203 | `png_read_finish_row` | yes | yes | via pipeline |
| 204 | `png_read_image` | yes | yes | yes |
| 205 | `png_read_info` | yes | yes | yes |
| 206 | `png_read_png` | yes | yes | yes |
| 207 | `png_read_push_finish_row` | yes | yes | via pipeline |
| 208 | `png_read_row` | yes | yes | yes |
| 209 | `png_read_rows` | yes | yes | yes |
| 210 | `png_read_sig` | yes | yes | via pipeline |
| 211 | `png_read_start_row` | yes | yes | via pipeline |
| 212 | `png_read_transform_info` | yes | yes | via pipeline |
| 213 | `png_read_update_info` | yes | yes | yes |
| 214 | `png_realloc_array` | yes | yes | yes |
| 215 | `png_reciprocal` | yes | yes | yes |
| 216 | `png_reciprocal2` | yes | yes | yes |
| 217 | `png_reset_crc` | yes | yes | yes |
| 218 | `png_reset_zstream` | yes | yes | yes |
| 219 | `png_resolve_file_gamma` | yes | yes | via pipeline |
| 220 | `png_sRGB_base` | yes | yes | yes |
| 221 | `png_sRGB_delta` | yes | yes | yes |
| 222 | `png_sRGB_table` | yes | yes | yes |
| 223 | `png_safe_error` | yes | yes | via pipeline |
| 224 | `png_safe_execute` | yes | yes | via pipeline |
| 225 | `png_safe_warning` | yes | yes | via pipeline |
| 226 | `png_safecat` | yes | yes | yes |
| 227 | `png_save_int_32` | yes | yes | yes |
| 228 | `png_save_uint_16` | yes | yes | yes |
| 229 | `png_save_uint_32` | yes | yes | yes |
| 230 | `png_set_IHDR` | yes | yes | yes |
| 231 | `png_set_PLTE` | yes | yes | yes |
| 232 | `png_set_add_alpha` | yes | yes | yes |
| 233 | `png_set_alpha_mode` | yes | yes | yes |
| 234 | `png_set_alpha_mode_fixed` | yes | yes | yes |
| 235 | `png_set_bKGD` | yes | yes | yes |
| 236 | `png_set_background` | yes | yes | yes |
| 237 | `png_set_background_fixed` | yes | yes | yes |
| 238 | `png_set_benign_errors` | yes | yes | yes |
| 239 | `png_set_bgr` | yes | yes | yes |
| 240 | `png_set_cHRM` | yes | yes | yes |
| 241 | `png_set_cHRM_XYZ` | yes | yes | yes |
| 242 | `png_set_cHRM_XYZ_fixed` | yes | yes | yes |
| 243 | `png_set_cHRM_fixed` | yes | yes | yes |
| 244 | `png_set_cICP` | yes | yes | yes |
| 245 | `png_set_cLLI` | yes | yes | yes |
| 246 | `png_set_cLLI_fixed` | yes | yes | yes |
| 247 | `png_set_check_for_invalid_index` | yes | yes | yes |
| 248 | `png_set_chunk_cache_max` | yes | yes | yes |
| 249 | `png_set_chunk_malloc_max` | yes | yes | yes |
| 250 | `png_set_compression_buffer_size` | yes | yes | yes |
| 251 | `png_set_compression_level` | yes | yes | yes |
| 252 | `png_set_compression_mem_level` | yes | yes | yes |
| 253 | `png_set_compression_method` | yes | yes | yes |
| 254 | `png_set_compression_strategy` | yes | yes | yes |
| 255 | `png_set_compression_window_bits` | yes | yes | yes |
| 256 | `png_set_crc_action` | yes | yes | yes |
| 257 | `png_set_eXIf` | yes | yes | yes |
| 258 | `png_set_eXIf_1` | yes | yes | yes |
| 259 | `png_set_error_fn` | yes | yes | yes |
| 260 | `png_set_expand` | yes | yes | yes |
| 261 | `png_set_expand_16` | yes | yes | yes |
| 262 | `png_set_expand_gray_1_2_4_to_8` | yes | yes | yes |
| 263 | `png_set_filler` | yes | yes | yes |
| 264 | `png_set_filter` | yes | yes | yes |
| 265 | `png_set_filter_heuristics` | yes | yes | yes |
| 266 | `png_set_filter_heuristics_fixed` | yes | yes | yes |
| 267 | `png_set_flush` | yes | yes | yes |
| 268 | `png_set_gAMA` | yes | yes | yes |
| 269 | `png_set_gAMA_fixed` | yes | yes | yes |
| 270 | `png_set_gamma` | yes | yes | yes |
| 271 | `png_set_gamma_fixed` | yes | yes | yes |
| 272 | `png_set_gray_to_rgb` | yes | yes | yes |
| 273 | `png_set_hIST` | yes | yes | yes |
| 274 | `png_set_iCCP` | yes | yes | yes |
| 275 | `png_set_interlace_handling` | yes | yes | yes |
| 276 | `png_set_invalid` | yes | yes | yes |
| 277 | `png_set_invert_alpha` | yes | yes | yes |
| 278 | `png_set_invert_mono` | yes | yes | yes |
| 279 | `png_set_keep_unknown_chunks` | yes | yes | yes |
| 280 | `png_set_longjmp_fn` | yes | yes | yes |
| 281 | `png_set_mDCV` | yes | yes | yes |
| 282 | `png_set_mDCV_fixed` | yes | yes | yes |
| 283 | `png_set_mem_fn` | yes | yes | yes |
| 284 | `png_set_oFFs` | yes | yes | yes |
| 285 | `png_set_option` | yes | yes | yes |
| 286 | `png_set_pCAL` | yes | yes | yes |
| 287 | `png_set_pHYs` | yes | yes | yes |
| 288 | `png_set_packing` | yes | yes | yes |
| 289 | `png_set_packswap` | yes | yes | yes |
| 290 | `png_set_palette_to_rgb` | yes | yes | yes |
| 291 | `png_set_progressive_read_fn` | yes | yes | yes |
| 292 | `png_set_quantize` | yes | yes | yes |
| 293 | `png_set_read_fn` | yes | yes | yes |
| 294 | `png_set_read_status_fn` | yes | yes | yes |
| 295 | `png_set_read_user_chunk_fn` | yes | yes | yes |
| 296 | `png_set_read_user_transform_fn` | yes | yes | yes |
| 297 | `png_set_rgb_coefficients` | yes | yes | via pipeline |
| 298 | `png_set_rgb_to_gray` | yes | yes | yes |
| 299 | `png_set_rgb_to_gray_fixed` | yes | yes | yes |
| 300 | `png_set_rows` | yes | yes | yes |
| 301 | `png_set_sBIT` | yes | yes | yes |
| 302 | `png_set_sCAL` | yes | yes | yes |
| 303 | `png_set_sCAL_fixed` | yes | yes | yes |
| 304 | `png_set_sCAL_s` | yes | yes | yes |
| 305 | `png_set_sPLT` | yes | yes | yes |
| 306 | `png_set_sRGB` | yes | yes | yes |
| 307 | `png_set_sRGB_gAMA_and_cHRM` | yes | yes | yes |
| 308 | `png_set_scale_16` | yes | yes | yes |
| 309 | `png_set_shift` | yes | yes | yes |
| 310 | `png_set_sig_bytes` | yes | yes | yes |
| 311 | `png_set_strip_16` | yes | yes | yes |
| 312 | `png_set_strip_alpha` | yes | yes | yes |
| 313 | `png_set_swap` | yes | yes | yes |
| 314 | `png_set_swap_alpha` | yes | yes | yes |
| 315 | `png_set_tIME` | yes | yes | yes |
| 316 | `png_set_tRNS` | yes | yes | yes |
| 317 | `png_set_tRNS_to_alpha` | yes | yes | yes |
| 318 | `png_set_text` | yes | yes | yes |
| 319 | `png_set_text_2` | yes | yes | yes |
| 320 | `png_set_text_compression_level` | yes | yes | yes |
| 321 | `png_set_text_compression_mem_level` | yes | yes | yes |
| 322 | `png_set_text_compression_method` | yes | yes | yes |
| 323 | `png_set_text_compression_strategy` | yes | yes | yes |
| 324 | `png_set_text_compression_window_bits` | yes | yes | yes |
| 325 | `png_set_unknown_chunk_location` | yes | yes | yes |
| 326 | `png_set_unknown_chunks` | yes | yes | yes |
| 327 | `png_set_user_limits` | yes | yes | yes |
| 328 | `png_set_user_transform_info` | yes | yes | yes |
| 329 | `png_set_write_fn` | yes | yes | yes |
| 330 | `png_set_write_status_fn` | yes | yes | yes |
| 331 | `png_set_write_user_transform_fn` | yes | yes | yes |
| 332 | `png_sig_cmp` | yes | yes | yes |
| 333 | `png_start_read_image` | yes | yes | yes |
| 334 | `png_user_version_check` | yes | yes | yes |
| 335 | `png_warning` | yes | yes | yes |
| 336 | `png_warning_parameter` | yes | yes | yes |
| 337 | `png_warning_parameter_signed` | yes | yes | yes |
| 338 | `png_warning_parameter_unsigned` | yes | yes | yes |
| 339 | `png_write_IEND` | yes | yes | yes |
| 340 | `png_write_IHDR` | yes | yes | yes |
| 341 | `png_write_PLTE` | yes | yes | yes |
| 342 | `png_write_bKGD` | yes | yes | yes |
| 343 | `png_write_cHRM_fixed` | yes | yes | yes |
| 344 | `png_write_cICP` | yes | yes | yes |
| 345 | `png_write_cLLI_fixed` | yes | yes | yes |
| 346 | `png_write_chunk` | yes | yes | yes |
| 347 | `png_write_chunk_data` | yes | yes | yes |
| 348 | `png_write_chunk_end` | yes | yes | yes |
| 349 | `png_write_chunk_start` | yes | yes | yes |
| 350 | `png_write_data` | yes | yes | yes |
| 351 | `png_write_eXIf` | yes | yes | yes |
| 352 | `png_write_end` | yes | yes | yes |
| 353 | `png_write_find_filter` | yes | yes | via pipeline |
| 354 | `png_write_finish_row` | yes | yes | via pipeline |
| 355 | `png_write_flush` | yes | yes | yes |
| 356 | `png_write_gAMA_fixed` | yes | yes | yes |
| 357 | `png_write_hIST` | yes | yes | yes |
| 358 | `png_write_iCCP` | yes | yes | yes |
| 359 | `png_write_iTXt` | yes | yes | yes |
| 360 | `png_write_image` | yes | yes | yes |
| 361 | `png_write_info` | yes | yes | yes |
| 362 | `png_write_info_before_PLTE` | yes | yes | yes |
| 363 | `png_write_mDCV_fixed` | yes | yes | yes |
| 364 | `png_write_oFFs` | yes | yes | yes |
| 365 | `png_write_pCAL` | yes | yes | yes |
| 366 | `png_write_pHYs` | yes | yes | yes |
| 367 | `png_write_png` | yes | yes | yes |
| 368 | `png_write_row` | yes | yes | yes |
| 369 | `png_write_rows` | yes | yes | yes |
| 370 | `png_write_sBIT` | yes | yes | yes |
| 371 | `png_write_sCAL_s` | yes | yes | yes |
| 372 | `png_write_sPLT` | yes | yes | yes |
| 373 | `png_write_sRGB` | yes | yes | yes |
| 374 | `png_write_sig` | yes | yes | yes |
| 375 | `png_write_start_row` | yes | yes | via pipeline |
| 376 | `png_write_tEXt` | yes | yes | yes |
| 377 | `png_write_tIME` | yes | yes | yes |
| 378 | `png_write_tRNS` | yes | yes | yes |
| 379 | `png_write_zTXt` | yes | yes | yes |
| 380 | `png_xy_from_XYZ` | yes | yes | yes |
| 381 | `png_zalloc` | yes | yes | yes |
| 382 | `png_zfree` | yes | yes | yes |
| 383 | `png_zlib_inflate` | yes | yes | via pipeline |
| 384 | `png_zstream_error` | yes | yes | via pipeline |
