# SYMBOLS.md — dynamic-symbol parity, C `libpng.so` vs Rust `liblibpng.so`

Generated mechanically from `nm -D --defined-only` on both shared libraries.

* C   `.so`: `tmp/cbuild/libpng.so` (cmake build of `c_src/`)
* Rust `.so`: `translated_rust/target/debug/liblibpng.so` (`cargo build`)

**Totals: C exports 384 symbols, Rust exports 384 symbols, 0 missing from Rust, 0 extra in Rust.**

| # | symbol | C type | Rust type | in Rust .so | C source file |
|---|--------|--------|-----------|-------------|---------------|
| 1 | `png_XYZ_from_xy` | T | T | yes | png.c |
| 2 | `png_access_version_number` | T | T | yes | png.c |
| 3 | `png_app_error` | T | T | yes | pngerror.c |
| 4 | `png_app_warning` | T | T | yes | pngerror.c |
| 5 | `png_ascii_from_fixed` | T | T | yes | png.c |
| 6 | `png_ascii_from_fp` | T | T | yes | png.c |
| 7 | `png_benign_error` | T | T | yes | pngerror.c |
| 8 | `png_build_gamma_table` | T | T | yes | png.c |
| 9 | `png_build_grayscale_palette` | T | T | yes | png.c |
| 10 | `png_calculate_crc` | T | T | yes | png.c |
| 11 | `png_calloc` | T | T | yes | (see pngrutil/pngtrans parts) |
| 12 | `png_check_IHDR` | T | T | yes | png.c |
| 13 | `png_check_fp_number` | T | T | yes | png.c |
| 14 | `png_check_fp_string` | T | T | yes | png.c |
| 15 | `png_check_keyword` | T | T | yes | pngset.c |
| 16 | `png_chunk_benign_error` | T | T | yes | pngerror.c |
| 17 | `png_chunk_error` | T | T | yes | (see pngrutil/pngtrans parts) |
| 18 | `png_chunk_report` | T | T | yes | pngerror.c |
| 19 | `png_chunk_unknown_handling` | T | T | yes | png.c |
| 20 | `png_chunk_warning` | T | T | yes | pngerror.c |
| 21 | `png_combine_row` | T | T | yes | pngrutil.c |
| 22 | `png_compress_IDAT` | T | T | yes | pngwutil.c |
| 23 | `png_convert_from_struct_tm` | T | T | yes | pngwrite.c |
| 24 | `png_convert_from_time_t` | T | T | yes | pngwrite.c |
| 25 | `png_convert_to_rfc1123` | T | T | yes | png.c |
| 26 | `png_convert_to_rfc1123_buffer` | T | T | yes | png.c |
| 27 | `png_crc_finish` | T | T | yes | pngrutil.c |
| 28 | `png_crc_read` | T | T | yes | pngrutil.c |
| 29 | `png_create_info_struct` | T | T | yes | (see pngrutil/pngtrans parts) |
| 30 | `png_create_png_struct` | T | T | yes | (see pngrutil/pngtrans parts) |
| 31 | `png_create_read_struct` | T | T | yes | (see pngrutil/pngtrans parts) |
| 32 | `png_create_read_struct_2` | T | T | yes | (see pngrutil/pngtrans parts) |
| 33 | `png_create_write_struct` | T | T | yes | (see pngrutil/pngtrans parts) |
| 34 | `png_create_write_struct_2` | T | T | yes | (see pngrutil/pngtrans parts) |
| 35 | `png_data_freer` | T | T | yes | png.c |
| 36 | `png_default_flush` | T | T | yes | pngwio.c |
| 37 | `png_default_read_data` | T | T | yes | pngrio.c |
| 38 | `png_default_write_data` | T | T | yes | pngwio.c |
| 39 | `png_destroy_gamma_table` | T | T | yes | png.c |
| 40 | `png_destroy_info_struct` | T | T | yes | png.c |
| 41 | `png_destroy_png_struct` | T | T | yes | pngmem.c |
| 42 | `png_destroy_read_struct` | T | T | yes | pngread.c |
| 43 | `png_destroy_write_struct` | T | T | yes | pngwrite.c |
| 44 | `png_do_bgr` | T | T | yes | pngtrans.c |
| 45 | `png_do_check_palette_indexes` | T | T | yes | pngtrans.c |
| 46 | `png_do_invert` | T | T | yes | pngtrans.c |
| 47 | `png_do_packswap` | T | T | yes | pngtrans.c |
| 48 | `png_do_read_interlace` | T | T | yes | pngrutil.c |
| 49 | `png_do_read_transformations` | T | T | yes | pngrtran.c |
| 50 | `png_do_strip_channel` | T | T | yes | pngtrans.c |
| 51 | `png_do_swap` | T | T | yes | pngtrans.c |
| 52 | `png_do_write_interlace` | T | T | yes | pngwutil.c |
| 53 | `png_do_write_transformations` | T | T | yes | pngwtran.c |
| 54 | `png_error` | T | T | yes | (see pngrutil/pngtrans parts) |
| 55 | `png_fixed` | T | T | yes | png.c |
| 56 | `png_fixed_ITU` | T | T | yes | png.c |
| 57 | `png_fixed_error` | T | T | yes | (see pngrutil/pngtrans parts) |
| 58 | `png_flush` | T | T | yes | pngwio.c |
| 59 | `png_format_number` | T | T | yes | pngerror.c |
| 60 | `png_formatted_warning` | T | T | yes | pngerror.c |
| 61 | `png_free` | T | T | yes | pngmem.c |
| 62 | `png_free_buffer_list` | T | T | yes | pngwutil.c |
| 63 | `png_free_data` | T | T | yes | png.c |
| 64 | `png_free_default` | T | T | yes | (see pngrutil/pngtrans parts) |
| 65 | `png_free_jmpbuf` | T | T | yes | pngerror.c |
| 66 | `png_gamma_16bit_correct` | T | T | yes | png.c |
| 67 | `png_gamma_8bit_correct` | T | T | yes | png.c |
| 68 | `png_gamma_correct` | T | T | yes | png.c |
| 69 | `png_gamma_significant` | T | T | yes | png.c |
| 70 | `png_get_IHDR` | T | T | yes | pngget.c |
| 71 | `png_get_PLTE` | T | T | yes | pngget.c |
| 72 | `png_get_bKGD` | T | T | yes | pngget.c |
| 73 | `png_get_bit_depth` | T | T | yes | pngget.c |
| 74 | `png_get_cHRM` | T | T | yes | pngget.c |
| 75 | `png_get_cHRM_XYZ` | T | T | yes | pngget.c |
| 76 | `png_get_cHRM_XYZ_fixed` | T | T | yes | pngget.c |
| 77 | `png_get_cHRM_fixed` | T | T | yes | pngget.c |
| 78 | `png_get_cICP` | T | T | yes | pngget.c |
| 79 | `png_get_cLLI` | T | T | yes | pngget.c |
| 80 | `png_get_cLLI_fixed` | T | T | yes | pngget.c |
| 81 | `png_get_channels` | T | T | yes | pngget.c |
| 82 | `png_get_chunk_cache_max` | T | T | yes | pngget.c |
| 83 | `png_get_chunk_malloc_max` | T | T | yes | pngget.c |
| 84 | `png_get_color_type` | T | T | yes | pngget.c |
| 85 | `png_get_compression_buffer_size` | T | T | yes | pngget.c |
| 86 | `png_get_compression_type` | T | T | yes | pngget.c |
| 87 | `png_get_copyright` | T | T | yes | png.c |
| 88 | `png_get_current_pass_number` | T | T | yes | pngtrans.c |
| 89 | `png_get_current_row_number` | T | T | yes | pngtrans.c |
| 90 | `png_get_eXIf` | T | T | yes | pngget.c |
| 91 | `png_get_eXIf_1` | T | T | yes | pngget.c |
| 92 | `png_get_error_ptr` | T | T | yes | pngerror.c |
| 93 | `png_get_filter_type` | T | T | yes | pngget.c |
| 94 | `png_get_gAMA` | T | T | yes | pngget.c |
| 95 | `png_get_gAMA_fixed` | T | T | yes | pngget.c |
| 96 | `png_get_hIST` | T | T | yes | pngget.c |
| 97 | `png_get_header_ver` | T | T | yes | png.c |
| 98 | `png_get_header_version` | T | T | yes | png.c |
| 99 | `png_get_iCCP` | T | T | yes | pngget.c |
| 100 | `png_get_image_height` | T | T | yes | pngget.c |
| 101 | `png_get_image_width` | T | T | yes | pngget.c |
| 102 | `png_get_int_32` | T | T | yes | (see pngrutil/pngtrans parts) |
| 103 | `png_get_interlace_type` | T | T | yes | pngget.c |
| 104 | `png_get_io_chunk_type` | T | T | yes | pngget.c |
| 105 | `png_get_io_ptr` | T | T | yes | png.c |
| 106 | `png_get_io_state` | T | T | yes | pngget.c |
| 107 | `png_get_libpng_ver` | T | T | yes | png.c |
| 108 | `png_get_mDCV` | T | T | yes | pngget.c |
| 109 | `png_get_mDCV_fixed` | T | T | yes | pngget.c |
| 110 | `png_get_mem_ptr` | T | T | yes | pngmem.c |
| 111 | `png_get_oFFs` | T | T | yes | pngget.c |
| 112 | `png_get_pCAL` | T | T | yes | pngget.c |
| 113 | `png_get_pHYs` | T | T | yes | pngget.c |
| 114 | `png_get_pHYs_dpi` | T | T | yes | pngget.c |
| 115 | `png_get_palette_max` | T | T | yes | pngget.c |
| 116 | `png_get_pixel_aspect_ratio` | T | T | yes | pngget.c |
| 117 | `png_get_pixel_aspect_ratio_fixed` | T | T | yes | pngget.c |
| 118 | `png_get_pixels_per_inch` | T | T | yes | pngget.c |
| 119 | `png_get_pixels_per_meter` | T | T | yes | pngget.c |
| 120 | `png_get_progressive_ptr` | T | T | yes | pngpread.c |
| 121 | `png_get_rgb_to_gray_status` | T | T | yes | pngget.c |
| 122 | `png_get_rowbytes` | T | T | yes | pngget.c |
| 123 | `png_get_rows` | T | T | yes | pngget.c |
| 124 | `png_get_sBIT` | T | T | yes | pngget.c |
| 125 | `png_get_sCAL` | T | T | yes | pngget.c |
| 126 | `png_get_sCAL_fixed` | T | T | yes | pngget.c |
| 127 | `png_get_sCAL_s` | T | T | yes | pngget.c |
| 128 | `png_get_sPLT` | T | T | yes | pngget.c |
| 129 | `png_get_sRGB` | T | T | yes | pngget.c |
| 130 | `png_get_signature` | T | T | yes | pngget.c |
| 131 | `png_get_tIME` | T | T | yes | pngget.c |
| 132 | `png_get_tRNS` | T | T | yes | pngget.c |
| 133 | `png_get_text` | T | T | yes | pngget.c |
| 134 | `png_get_uint_16` | T | T | yes | (see pngrutil/pngtrans parts) |
| 135 | `png_get_uint_31` | T | T | yes | pngrutil.c |
| 136 | `png_get_uint_32` | T | T | yes | (see pngrutil/pngtrans parts) |
| 137 | `png_get_unknown_chunks` | T | T | yes | pngget.c |
| 138 | `png_get_user_chunk_ptr` | T | T | yes | pngget.c |
| 139 | `png_get_user_height_max` | T | T | yes | pngget.c |
| 140 | `png_get_user_transform_ptr` | T | T | yes | pngtrans.c |
| 141 | `png_get_user_width_max` | T | T | yes | pngget.c |
| 142 | `png_get_valid` | T | T | yes | pngget.c |
| 143 | `png_get_x_offset_inches` | T | T | yes | pngget.c |
| 144 | `png_get_x_offset_inches_fixed` | T | T | yes | pngget.c |
| 145 | `png_get_x_offset_microns` | T | T | yes | pngget.c |
| 146 | `png_get_x_offset_pixels` | T | T | yes | pngget.c |
| 147 | `png_get_x_pixels_per_inch` | T | T | yes | pngget.c |
| 148 | `png_get_x_pixels_per_meter` | T | T | yes | pngget.c |
| 149 | `png_get_y_offset_inches` | T | T | yes | pngget.c |
| 150 | `png_get_y_offset_inches_fixed` | T | T | yes | pngget.c |
| 151 | `png_get_y_offset_microns` | T | T | yes | pngget.c |
| 152 | `png_get_y_offset_pixels` | T | T | yes | pngget.c |
| 153 | `png_get_y_pixels_per_inch` | T | T | yes | pngget.c |
| 154 | `png_get_y_pixels_per_meter` | T | T | yes | pngget.c |
| 155 | `png_handle_as_unknown` | T | T | yes | png.c |
| 156 | `png_handle_chunk` | T | T | yes | pngrutil.c |
| 157 | `png_handle_unknown` | T | T | yes | pngrutil.c |
| 158 | `png_icc_check_header` | T | T | yes | png.c |
| 159 | `png_icc_check_length` | T | T | yes | png.c |
| 160 | `png_icc_check_tag_table` | T | T | yes | png.c |
| 161 | `png_image_begin_read_from_file` | T | T | yes | pngread.c |
| 162 | `png_image_begin_read_from_memory` | T | T | yes | (see pngrutil/pngtrans parts) |
| 163 | `png_image_begin_read_from_stdio` | T | T | yes | pngread.c |
| 164 | `png_image_error` | T | T | yes | png.c |
| 165 | `png_image_finish_read` | T | T | yes | pngread.c |
| 166 | `png_image_free` | T | T | yes | png.c |
| 167 | `png_image_write_to_file` | T | T | yes | pngwrite.c |
| 168 | `png_image_write_to_memory` | T | T | yes | pngwrite.c |
| 169 | `png_image_write_to_stdio` | T | T | yes | pngwrite.c |
| 170 | `png_info_init_3` | T | T | yes | (see pngrutil/pngtrans parts) |
| 171 | `png_init_io` | T | T | yes | png.c |
| 172 | `png_init_read_transformations` | T | T | yes | pngrtran.c |
| 173 | `png_longjmp` | T | T | yes | (see pngrutil/pngtrans parts) |
| 174 | `png_malloc` | T | T | yes | (see pngrutil/pngtrans parts) |
| 175 | `png_malloc_array` | T | T | yes | (see pngrutil/pngtrans parts) |
| 176 | `png_malloc_base` | T | T | yes | (see pngrutil/pngtrans parts) |
| 177 | `png_malloc_default` | T | T | yes | (see pngrutil/pngtrans parts) |
| 178 | `png_malloc_warn` | T | T | yes | (see pngrutil/pngtrans parts) |
| 179 | `png_muldiv` | T | T | yes | png.c |
| 180 | `png_permit_mng_features` | T | T | yes | pngset.c |
| 181 | `png_process_IDAT_data` | T | T | yes | pngpread.c |
| 182 | `png_process_data` | T | T | yes | pngpread.c |
| 183 | `png_process_data_pause` | T | T | yes | pngpread.c |
| 184 | `png_process_data_skip` | T | T | yes | pngpread.c |
| 185 | `png_process_some_data` | T | T | yes | pngpread.c |
| 186 | `png_progressive_combine_row` | T | T | yes | pngpread.c |
| 187 | `png_push_fill_buffer` | T | T | yes | pngpread.c |
| 188 | `png_push_have_end` | T | T | yes | pngpread.c |
| 189 | `png_push_have_info` | T | T | yes | pngpread.c |
| 190 | `png_push_have_row` | T | T | yes | pngpread.c |
| 191 | `png_push_process_row` | T | T | yes | pngpread.c |
| 192 | `png_push_read_IDAT` | T | T | yes | pngpread.c |
| 193 | `png_push_read_chunk` | T | T | yes | pngpread.c |
| 194 | `png_push_read_sig` | T | T | yes | pngpread.c |
| 195 | `png_push_restore_buffer` | T | T | yes | pngpread.c |
| 196 | `png_push_save_buffer` | T | T | yes | pngpread.c |
| 197 | `png_read_IDAT_data` | T | T | yes | pngrutil.c |
| 198 | `png_read_chunk_header` | T | T | yes | pngrutil.c |
| 199 | `png_read_data` | T | T | yes | pngrio.c |
| 200 | `png_read_end` | T | T | yes | pngread.c |
| 201 | `png_read_filter_row` | T | T | yes | pngrutil.c |
| 202 | `png_read_finish_IDAT` | T | T | yes | pngrutil.c |
| 203 | `png_read_finish_row` | T | T | yes | pngrutil.c |
| 204 | `png_read_image` | T | T | yes | pngread.c |
| 205 | `png_read_info` | T | T | yes | pngread.c |
| 206 | `png_read_png` | T | T | yes | pngread.c |
| 207 | `png_read_push_finish_row` | T | T | yes | pngpread.c |
| 208 | `png_read_row` | T | T | yes | pngread.c |
| 209 | `png_read_rows` | T | T | yes | pngread.c |
| 210 | `png_read_sig` | T | T | yes | pngrutil.c |
| 211 | `png_read_start_row` | T | T | yes | pngrutil.c |
| 212 | `png_read_transform_info` | T | T | yes | pngrtran.c |
| 213 | `png_read_update_info` | T | T | yes | pngread.c |
| 214 | `png_realloc_array` | T | T | yes | (see pngrutil/pngtrans parts) |
| 215 | `png_reciprocal` | T | T | yes | png.c |
| 216 | `png_reciprocal2` | T | T | yes | png.c |
| 217 | `png_reset_crc` | T | T | yes | png.c |
| 218 | `png_reset_zstream` | T | T | yes | png.c |
| 219 | `png_resolve_file_gamma` | T | T | yes | pngrtran.c |
| 220 | `png_sRGB_base` | R | R | yes | (see pngrutil/pngtrans parts) |
| 221 | `png_sRGB_delta` | R | R | yes | (see pngrutil/pngtrans parts) |
| 222 | `png_sRGB_table` | R | R | yes | (see pngrutil/pngtrans parts) |
| 223 | `png_safe_error` | T | T | yes | (see pngrutil/pngtrans parts) |
| 224 | `png_safe_execute` | T | T | yes | pngerror.c |
| 225 | `png_safe_warning` | T | T | yes | pngerror.c |
| 226 | `png_safecat` | T | T | yes | pngerror.c |
| 227 | `png_save_int_32` | T | T | yes | png.c |
| 228 | `png_save_uint_16` | T | T | yes | pngwutil.c |
| 229 | `png_save_uint_32` | T | T | yes | pngwutil.c |
| 230 | `png_set_IHDR` | T | T | yes | pngset.c |
| 231 | `png_set_PLTE` | T | T | yes | pngset.c |
| 232 | `png_set_add_alpha` | T | T | yes | pngtrans.c |
| 233 | `png_set_alpha_mode` | T | T | yes | pngrtran.c |
| 234 | `png_set_alpha_mode_fixed` | T | T | yes | pngrtran.c |
| 235 | `png_set_bKGD` | T | T | yes | pngset.c |
| 236 | `png_set_background` | T | T | yes | pngrtran.c |
| 237 | `png_set_background_fixed` | T | T | yes | pngrtran.c |
| 238 | `png_set_benign_errors` | T | T | yes | pngset.c |
| 239 | `png_set_bgr` | T | T | yes | pngtrans.c |
| 240 | `png_set_cHRM` | T | T | yes | pngset.c |
| 241 | `png_set_cHRM_XYZ` | T | T | yes | pngset.c |
| 242 | `png_set_cHRM_XYZ_fixed` | T | T | yes | pngset.c |
| 243 | `png_set_cHRM_fixed` | T | T | yes | pngset.c |
| 244 | `png_set_cICP` | T | T | yes | pngset.c |
| 245 | `png_set_cLLI` | T | T | yes | pngset.c |
| 246 | `png_set_cLLI_fixed` | T | T | yes | pngset.c |
| 247 | `png_set_check_for_invalid_index` | T | T | yes | pngset.c |
| 248 | `png_set_chunk_cache_max` | T | T | yes | pngset.c |
| 249 | `png_set_chunk_malloc_max` | T | T | yes | pngset.c |
| 250 | `png_set_compression_buffer_size` | T | T | yes | pngset.c |
| 251 | `png_set_compression_level` | T | T | yes | pngwrite.c |
| 252 | `png_set_compression_mem_level` | T | T | yes | pngwrite.c |
| 253 | `png_set_compression_method` | T | T | yes | pngwrite.c |
| 254 | `png_set_compression_strategy` | T | T | yes | pngwrite.c |
| 255 | `png_set_compression_window_bits` | T | T | yes | pngwrite.c |
| 256 | `png_set_crc_action` | T | T | yes | pngrtran.c |
| 257 | `png_set_eXIf` | T | T | yes | pngset.c |
| 258 | `png_set_eXIf_1` | T | T | yes | pngset.c |
| 259 | `png_set_error_fn` | T | T | yes | pngerror.c |
| 260 | `png_set_expand` | T | T | yes | pngrtran.c |
| 261 | `png_set_expand_16` | T | T | yes | pngrtran.c |
| 262 | `png_set_expand_gray_1_2_4_to_8` | T | T | yes | pngrtran.c |
| 263 | `png_set_filler` | T | T | yes | pngtrans.c |
| 264 | `png_set_filter` | T | T | yes | pngwrite.c |
| 265 | `png_set_filter_heuristics` | T | T | yes | pngwrite.c |
| 266 | `png_set_filter_heuristics_fixed` | T | T | yes | pngwrite.c |
| 267 | `png_set_flush` | T | T | yes | pngwrite.c |
| 268 | `png_set_gAMA` | T | T | yes | pngset.c |
| 269 | `png_set_gAMA_fixed` | T | T | yes | pngset.c |
| 270 | `png_set_gamma` | T | T | yes | pngrtran.c |
| 271 | `png_set_gamma_fixed` | T | T | yes | pngrtran.c |
| 272 | `png_set_gray_to_rgb` | T | T | yes | pngrtran.c |
| 273 | `png_set_hIST` | T | T | yes | pngset.c |
| 274 | `png_set_iCCP` | T | T | yes | pngset.c |
| 275 | `png_set_interlace_handling` | T | T | yes | pngtrans.c |
| 276 | `png_set_invalid` | T | T | yes | pngset.c |
| 277 | `png_set_invert_alpha` | T | T | yes | pngtrans.c |
| 278 | `png_set_invert_mono` | T | T | yes | pngtrans.c |
| 279 | `png_set_keep_unknown_chunks` | T | T | yes | pngset.c |
| 280 | `png_set_longjmp_fn` | T | T | yes | pngerror.c |
| 281 | `png_set_mDCV` | T | T | yes | pngset.c |
| 282 | `png_set_mDCV_fixed` | T | T | yes | pngset.c |
| 283 | `png_set_mem_fn` | T | T | yes | pngmem.c |
| 284 | `png_set_oFFs` | T | T | yes | pngset.c |
| 285 | `png_set_option` | T | T | yes | png.c |
| 286 | `png_set_pCAL` | T | T | yes | pngset.c |
| 287 | `png_set_pHYs` | T | T | yes | pngset.c |
| 288 | `png_set_packing` | T | T | yes | pngtrans.c |
| 289 | `png_set_packswap` | T | T | yes | pngtrans.c |
| 290 | `png_set_palette_to_rgb` | T | T | yes | pngrtran.c |
| 291 | `png_set_progressive_read_fn` | T | T | yes | pngpread.c |
| 292 | `png_set_quantize` | T | T | yes | pngrtran.c |
| 293 | `png_set_read_fn` | T | T | yes | pngrio.c |
| 294 | `png_set_read_status_fn` | T | T | yes | pngread.c |
| 295 | `png_set_read_user_chunk_fn` | T | T | yes | pngset.c |
| 296 | `png_set_read_user_transform_fn` | T | T | yes | pngrtran.c |
| 297 | `png_set_rgb_coefficients` | T | T | yes | png.c |
| 298 | `png_set_rgb_to_gray` | T | T | yes | pngrtran.c |
| 299 | `png_set_rgb_to_gray_fixed` | T | T | yes | pngrtran.c |
| 300 | `png_set_rows` | T | T | yes | pngset.c |
| 301 | `png_set_sBIT` | T | T | yes | pngset.c |
| 302 | `png_set_sCAL` | T | T | yes | pngset.c |
| 303 | `png_set_sCAL_fixed` | T | T | yes | pngset.c |
| 304 | `png_set_sCAL_s` | T | T | yes | pngset.c |
| 305 | `png_set_sPLT` | T | T | yes | pngset.c |
| 306 | `png_set_sRGB` | T | T | yes | pngset.c |
| 307 | `png_set_sRGB_gAMA_and_cHRM` | T | T | yes | pngset.c |
| 308 | `png_set_scale_16` | T | T | yes | pngrtran.c |
| 309 | `png_set_shift` | T | T | yes | pngtrans.c |
| 310 | `png_set_sig_bytes` | T | T | yes | png.c |
| 311 | `png_set_strip_16` | T | T | yes | pngrtran.c |
| 312 | `png_set_strip_alpha` | T | T | yes | pngrtran.c |
| 313 | `png_set_swap` | T | T | yes | pngtrans.c |
| 314 | `png_set_swap_alpha` | T | T | yes | pngtrans.c |
| 315 | `png_set_tIME` | T | T | yes | pngset.c |
| 316 | `png_set_tRNS` | T | T | yes | pngset.c |
| 317 | `png_set_tRNS_to_alpha` | T | T | yes | pngrtran.c |
| 318 | `png_set_text` | T | T | yes | pngset.c |
| 319 | `png_set_text_2` | T | T | yes | pngset.c |
| 320 | `png_set_text_compression_level` | T | T | yes | pngwrite.c |
| 321 | `png_set_text_compression_mem_level` | T | T | yes | pngwrite.c |
| 322 | `png_set_text_compression_method` | T | T | yes | pngwrite.c |
| 323 | `png_set_text_compression_strategy` | T | T | yes | pngwrite.c |
| 324 | `png_set_text_compression_window_bits` | T | T | yes | pngwrite.c |
| 325 | `png_set_unknown_chunk_location` | T | T | yes | pngset.c |
| 326 | `png_set_unknown_chunks` | T | T | yes | pngset.c |
| 327 | `png_set_user_limits` | T | T | yes | pngset.c |
| 328 | `png_set_user_transform_info` | T | T | yes | pngtrans.c |
| 329 | `png_set_write_fn` | T | T | yes | pngwio.c |
| 330 | `png_set_write_status_fn` | T | T | yes | pngwrite.c |
| 331 | `png_set_write_user_transform_fn` | T | T | yes | pngwrite.c |
| 332 | `png_sig_cmp` | T | T | yes | png.c |
| 333 | `png_start_read_image` | T | T | yes | pngread.c |
| 334 | `png_user_version_check` | T | T | yes | png.c |
| 335 | `png_warning` | T | T | yes | pngerror.c |
| 336 | `png_warning_parameter` | T | T | yes | pngerror.c |
| 337 | `png_warning_parameter_signed` | T | T | yes | pngerror.c |
| 338 | `png_warning_parameter_unsigned` | T | T | yes | pngerror.c |
| 339 | `png_write_IEND` | T | T | yes | pngwutil.c |
| 340 | `png_write_IHDR` | T | T | yes | pngwutil.c |
| 341 | `png_write_PLTE` | T | T | yes | pngwutil.c |
| 342 | `png_write_bKGD` | T | T | yes | pngwutil.c |
| 343 | `png_write_cHRM_fixed` | T | T | yes | pngwutil.c |
| 344 | `png_write_cICP` | T | T | yes | pngwutil.c |
| 345 | `png_write_cLLI_fixed` | T | T | yes | pngwutil.c |
| 346 | `png_write_chunk` | T | T | yes | pngwutil.c |
| 347 | `png_write_chunk_data` | T | T | yes | pngwutil.c |
| 348 | `png_write_chunk_end` | T | T | yes | pngwutil.c |
| 349 | `png_write_chunk_start` | T | T | yes | pngwutil.c |
| 350 | `png_write_data` | T | T | yes | pngwio.c |
| 351 | `png_write_eXIf` | T | T | yes | pngwutil.c |
| 352 | `png_write_end` | T | T | yes | pngwrite.c |
| 353 | `png_write_find_filter` | T | T | yes | pngwutil.c |
| 354 | `png_write_finish_row` | T | T | yes | pngwutil.c |
| 355 | `png_write_flush` | T | T | yes | pngwrite.c |
| 356 | `png_write_gAMA_fixed` | T | T | yes | pngwutil.c |
| 357 | `png_write_hIST` | T | T | yes | pngwutil.c |
| 358 | `png_write_iCCP` | T | T | yes | pngwutil.c |
| 359 | `png_write_iTXt` | T | T | yes | pngwutil.c |
| 360 | `png_write_image` | T | T | yes | pngwrite.c |
| 361 | `png_write_info` | T | T | yes | pngwrite.c |
| 362 | `png_write_info_before_PLTE` | T | T | yes | pngwrite.c |
| 363 | `png_write_mDCV_fixed` | T | T | yes | pngwutil.c |
| 364 | `png_write_oFFs` | T | T | yes | pngwutil.c |
| 365 | `png_write_pCAL` | T | T | yes | pngwutil.c |
| 366 | `png_write_pHYs` | T | T | yes | pngwutil.c |
| 367 | `png_write_png` | T | T | yes | pngwrite.c |
| 368 | `png_write_row` | T | T | yes | pngwrite.c |
| 369 | `png_write_rows` | T | T | yes | pngwrite.c |
| 370 | `png_write_sBIT` | T | T | yes | pngwutil.c |
| 371 | `png_write_sCAL_s` | T | T | yes | pngwutil.c |
| 372 | `png_write_sPLT` | T | T | yes | pngwutil.c |
| 373 | `png_write_sRGB` | T | T | yes | pngwutil.c |
| 374 | `png_write_sig` | T | T | yes | pngwutil.c |
| 375 | `png_write_start_row` | T | T | yes | pngwutil.c |
| 376 | `png_write_tEXt` | T | T | yes | pngwutil.c |
| 377 | `png_write_tIME` | T | T | yes | pngwutil.c |
| 378 | `png_write_tRNS` | T | T | yes | pngwutil.c |
| 379 | `png_write_zTXt` | T | T | yes | pngwutil.c |
| 380 | `png_xy_from_XYZ` | T | T | yes | png.c |
| 381 | `png_zalloc` | T | T | yes | (see pngrutil/pngtrans parts) |
| 382 | `png_zfree` | T | T | yes | png.c |
| 383 | `png_zlib_inflate` | T | T | yes | pngrutil.c |
| 384 | `png_zstream_error` | T | T | yes | png.c |

## Build configurations

`Cargo.toml` declares **no `[features]` section**, so the crate has exactly ONE
build configuration: the default (empty) feature set. All three of

```
cargo check                          # default
cargo check --no-default-features    # identical (no features exist)
cargo check --all-features           # identical (no features exist)
```

resolve to the same code and compile without errors, so "every valid feature
combination" is the single default combination. The C side likewise has one
configuration: `c_src/CMakeLists.txt` globs all of `src/*.c` and the feature set
is fixed by the prebuilt `c_src/include/pnglibconf.h` (full feature set; ARM/
NEON/MIPS/SSE/POWERPC/RISCV, `PNG_ERROR_NUMBERS` and
`PNG_BENIGN_WRITE_ERRORS` off).

Additionally the Rust `.so` is verified in both codegen configurations:

* `target/debug/liblibpng.so` — `overflow-checks = on` (dev profile default), so
  any arithmetic that the C wraps but the Rust does not explicitly wrap shows up
  as a panic.
* `target/release/liblibpng.so` — `panic = "abort"`, `overflow-checks = false`
  (the shipped configuration).

The same test suite is run against both by pointing `PNG_RUST_SO` at the
respective library.

## Undefined (imported) symbols

`nm -D --undefined-only` on the Rust `.so` lists only libc/libm/zlib/unwinder
imports (`malloc`, `memcpy`, `pow`, `deflate`, `inflate`, `crc32`,
`_Unwind_*`, …) — no unresolved libpng symbol. `png_sRGB_base`,
`png_sRGB_delta` and `png_sRGB_table` are exported as data (`R`) by both
libraries and their full contents are compared by test `l17_srgb_tables`.
