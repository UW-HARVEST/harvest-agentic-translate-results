# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Mechanically derived from `nm -D --defined-only` on both shared objects:

```
nm -D --defined-only c_src/build/libpng.so                | awk '$2!="U"{print $3}' | sort -u  ->  384 symbols
nm -D --defined-only translation/target/release/liblibpng.so | awk '$2!="U"{print $3}' | sort -u  ->  384 symbols
```

**Result: 0 symbols missing from the Rust `.so`, 0 extra.** The whole
C library was translated; no stubs, no `unimplemented!()`.

The only undefined (`U`) symbols in the Rust `.so` are libc / libm / libz
imports (`malloc`, `memcpy`, `pow`, `deflate`, `inflate`, ...), exactly as in
the C `.so`; libpng itself does not implement DEFLATE and the reference
`CMakeLists.txt` links the system zlib, so the translation links it too.

| # | symbol | in C .so | in Rust .so | Rust definition |
|---|--------|----------|-------------|-----------------|
| 1 | `png_XYZ_from_xy` | yes | yes | `src/png_b.rs` |
| 2 | `png_access_version_number` | yes | yes | `src/png_a.rs` |
| 3 | `png_app_error` | yes | yes | `src/pngerror.rs` |
| 4 | `png_app_warning` | yes | yes | `src/pngerror.rs` |
| 5 | `png_ascii_from_fixed` | yes | yes | `src/png_c.rs` |
| 6 | `png_ascii_from_fp` | yes | yes | `src/png_c.rs` |
| 7 | `png_benign_error` | yes | yes | `src/pngerror.rs` |
| 8 | `png_build_gamma_table` | yes | yes | `src/png_d.rs` |
| 9 | `png_build_grayscale_palette` | yes | yes | `src/png_a.rs` |
| 10 | `png_calculate_crc` | yes | yes | `src/png_a.rs` |
| 11 | `png_calloc` | yes | yes | `src/pngmem.rs` |
| 12 | `png_check_IHDR` | yes | yes | `src/png_c.rs` |
| 13 | `png_check_fp_number` | yes | yes | `src/png_c.rs` |
| 14 | `png_check_fp_string` | yes | yes | `src/png_c.rs` |
| 15 | `png_check_keyword` | yes | yes | `src/pngset_b.rs` |
| 16 | `png_chunk_benign_error` | yes | yes | `src/pngerror.rs` |
| 17 | `png_chunk_error` | yes | yes | `src/pngerror.rs` |
| 18 | `png_chunk_report` | yes | yes | `src/pngerror.rs` |
| 19 | `png_chunk_unknown_handling` | yes | yes | `src/png_a.rs` |
| 20 | `png_chunk_warning` | yes | yes | `src/pngerror.rs` |
| 21 | `png_combine_row` | yes | yes | `src/pngrutil_d.rs` |
| 22 | `png_compress_IDAT` | yes | yes | `src/pngwutil_a.rs` |
| 23 | `png_convert_from_struct_tm` | yes | yes | `src/pngwrite_a.rs` |
| 24 | `png_convert_from_time_t` | yes | yes | `src/pngwrite_a.rs` |
| 25 | `png_convert_to_rfc1123` | yes | yes | `src/png_a.rs` |
| 26 | `png_convert_to_rfc1123_buffer` | yes | yes | `src/png_a.rs` |
| 27 | `png_crc_finish` | yes | yes | `src/pngrutil_a.rs` |
| 28 | `png_crc_read` | yes | yes | `src/pngrutil_a.rs` |
| 29 | `png_create_info_struct` | yes | yes | `src/png_a.rs` |
| 30 | `png_create_png_struct` | yes | yes | `src/png_a.rs` |
| 31 | `png_create_read_struct` | yes | yes | `src/pngread_a.rs` |
| 32 | `png_create_read_struct_2` | yes | yes | `src/pngread_a.rs` |
| 33 | `png_create_write_struct` | yes | yes | `src/pngwrite_a.rs` |
| 34 | `png_create_write_struct_2` | yes | yes | `src/pngwrite_a.rs` |
| 35 | `png_data_freer` | yes | yes | `src/png_a.rs` |
| 36 | `png_default_flush` | yes | yes | `src/pngwio.rs` |
| 37 | `png_default_read_data` | yes | yes | `src/pngrio.rs` |
| 38 | `png_default_write_data` | yes | yes | `src/pngwio.rs` |
| 39 | `png_destroy_gamma_table` | yes | yes | `src/png_d.rs` |
| 40 | `png_destroy_info_struct` | yes | yes | `src/png_a.rs` |
| 41 | `png_destroy_png_struct` | yes | yes | `src/pngmem.rs` |
| 42 | `png_destroy_read_struct` | yes | yes | `src/pngread_a.rs` |
| 43 | `png_destroy_write_struct` | yes | yes | `src/pngwrite_a.rs` |
| 44 | `png_do_bgr` | yes | yes | `src/pngtrans.rs` |
| 45 | `png_do_check_palette_indexes` | yes | yes | `src/pngtrans.rs` |
| 46 | `png_do_invert` | yes | yes | `src/pngtrans.rs` |
| 47 | `png_do_packswap` | yes | yes | `src/pngtrans.rs` |
| 48 | `png_do_read_interlace` | yes | yes | `src/pngrutil_d.rs` |
| 49 | `png_do_read_transformations` | yes | yes | `src/pngrtran_e.rs` |
| 50 | `png_do_strip_channel` | yes | yes | `src/pngtrans.rs` |
| 51 | `png_do_swap` | yes | yes | `src/pngtrans.rs` |
| 52 | `png_do_write_interlace` | yes | yes | `src/pngwutil_c.rs` |
| 53 | `png_do_write_transformations` | yes | yes | `src/pngwtran.rs` |
| 54 | `png_error` | yes | yes | `src/pngerror.rs` |
| 55 | `png_fixed` | yes | yes | `src/png_c.rs` |
| 56 | `png_fixed_ITU` | yes | yes | `src/png_c.rs` |
| 57 | `png_fixed_error` | yes | yes | `src/pngerror.rs` |
| 58 | `png_flush` | yes | yes | `src/pngwio.rs` |
| 59 | `png_format_number` | yes | yes | `src/pngerror.rs` |
| 60 | `png_formatted_warning` | yes | yes | `src/pngerror.rs` |
| 61 | `png_free` | yes | yes | `src/pngmem.rs` |
| 62 | `png_free_buffer_list` | yes | yes | `src/pngwutil_a.rs` |
| 63 | `png_free_data` | yes | yes | `src/png_a.rs` |
| 64 | `png_free_default` | yes | yes | `src/pngmem.rs` |
| 65 | `png_free_jmpbuf` | yes | yes | `src/pngerror.rs` |
| 66 | `png_gamma_16bit_correct` | yes | yes | `src/png_d.rs` |
| 67 | `png_gamma_8bit_correct` | yes | yes | `src/png_d.rs` |
| 68 | `png_gamma_correct` | yes | yes | `src/png_d.rs` |
| 69 | `png_gamma_significant` | yes | yes | `src/png_d.rs` |
| 70 | `png_get_IHDR` | yes | yes | `src/pngget.rs` |
| 71 | `png_get_PLTE` | yes | yes | `src/pngget.rs` |
| 72 | `png_get_bKGD` | yes | yes | `src/pngget.rs` |
| 73 | `png_get_bit_depth` | yes | yes | `src/pngget.rs` |
| 74 | `png_get_cHRM` | yes | yes | `src/pngget.rs` |
| 75 | `png_get_cHRM_XYZ` | yes | yes | `src/pngget.rs` |
| 76 | `png_get_cHRM_XYZ_fixed` | yes | yes | `src/pngget.rs` |
| 77 | `png_get_cHRM_fixed` | yes | yes | `src/pngget.rs` |
| 78 | `png_get_cICP` | yes | yes | `src/pngget.rs` |
| 79 | `png_get_cLLI` | yes | yes | `src/pngget.rs` |
| 80 | `png_get_cLLI_fixed` | yes | yes | `src/pngget.rs` |
| 81 | `png_get_channels` | yes | yes | `src/pngget.rs` |
| 82 | `png_get_chunk_cache_max` | yes | yes | `src/pngget.rs` |
| 83 | `png_get_chunk_malloc_max` | yes | yes | `src/pngget.rs` |
| 84 | `png_get_color_type` | yes | yes | `src/pngget.rs` |
| 85 | `png_get_compression_buffer_size` | yes | yes | `src/pngget.rs` |
| 86 | `png_get_compression_type` | yes | yes | `src/pngget.rs` |
| 87 | `png_get_copyright` | yes | yes | `src/png_a.rs` |
| 88 | `png_get_current_pass_number` | yes | yes | `src/pngtrans.rs` |
| 89 | `png_get_current_row_number` | yes | yes | `src/pngtrans.rs` |
| 90 | `png_get_eXIf` | yes | yes | `src/pngget.rs` |
| 91 | `png_get_eXIf_1` | yes | yes | `src/pngget.rs` |
| 92 | `png_get_error_ptr` | yes | yes | `src/pngerror.rs` |
| 93 | `png_get_filter_type` | yes | yes | `src/pngget.rs` |
| 94 | `png_get_gAMA` | yes | yes | `src/pngget.rs` |
| 95 | `png_get_gAMA_fixed` | yes | yes | `src/pngget.rs` |
| 96 | `png_get_hIST` | yes | yes | `src/pngget.rs` |
| 97 | `png_get_header_ver` | yes | yes | `src/png_a.rs` |
| 98 | `png_get_header_version` | yes | yes | `src/png_a.rs` |
| 99 | `png_get_iCCP` | yes | yes | `src/pngget.rs` |
| 100 | `png_get_image_height` | yes | yes | `src/pngget.rs` |
| 101 | `png_get_image_width` | yes | yes | `src/pngget.rs` |
| 102 | `png_get_int_32` | yes | yes | `src/pngrutil_a.rs` |
| 103 | `png_get_interlace_type` | yes | yes | `src/pngget.rs` |
| 104 | `png_get_io_chunk_type` | yes | yes | `src/pngget.rs` |
| 105 | `png_get_io_ptr` | yes | yes | `src/png_a.rs` |
| 106 | `png_get_io_state` | yes | yes | `src/pngget.rs` |
| 107 | `png_get_libpng_ver` | yes | yes | `src/png_a.rs` |
| 108 | `png_get_mDCV` | yes | yes | `src/pngget.rs` |
| 109 | `png_get_mDCV_fixed` | yes | yes | `src/pngget.rs` |
| 110 | `png_get_mem_ptr` | yes | yes | `src/pngmem.rs` |
| 111 | `png_get_oFFs` | yes | yes | `src/pngget.rs` |
| 112 | `png_get_pCAL` | yes | yes | `src/pngget.rs` |
| 113 | `png_get_pHYs` | yes | yes | `src/pngget.rs` |
| 114 | `png_get_pHYs_dpi` | yes | yes | `src/pngget.rs` |
| 115 | `png_get_palette_max` | yes | yes | `src/pngget.rs` |
| 116 | `png_get_pixel_aspect_ratio` | yes | yes | `src/pngget.rs` |
| 117 | `png_get_pixel_aspect_ratio_fixed` | yes | yes | `src/pngget.rs` |
| 118 | `png_get_pixels_per_inch` | yes | yes | `src/pngget.rs` |
| 119 | `png_get_pixels_per_meter` | yes | yes | `src/pngget.rs` |
| 120 | `png_get_progressive_ptr` | yes | yes | `src/pngpread.rs` |
| 121 | `png_get_rgb_to_gray_status` | yes | yes | `src/pngget.rs` |
| 122 | `png_get_rowbytes` | yes | yes | `src/pngget.rs` |
| 123 | `png_get_rows` | yes | yes | `src/pngget.rs` |
| 124 | `png_get_sBIT` | yes | yes | `src/pngget.rs` |
| 125 | `png_get_sCAL` | yes | yes | `src/pngget.rs` |
| 126 | `png_get_sCAL_fixed` | yes | yes | `src/pngget.rs` |
| 127 | `png_get_sCAL_s` | yes | yes | `src/pngget.rs` |
| 128 | `png_get_sPLT` | yes | yes | `src/pngget.rs` |
| 129 | `png_get_sRGB` | yes | yes | `src/pngget.rs` |
| 130 | `png_get_signature` | yes | yes | `src/pngget.rs` |
| 131 | `png_get_tIME` | yes | yes | `src/pngget.rs` |
| 132 | `png_get_tRNS` | yes | yes | `src/pngget.rs` |
| 133 | `png_get_text` | yes | yes | `src/pngget.rs` |
| 134 | `png_get_uint_16` | yes | yes | `src/pngrutil_a.rs` |
| 135 | `png_get_uint_31` | yes | yes | `src/pngrutil_a.rs` |
| 136 | `png_get_uint_32` | yes | yes | `src/pngrutil_a.rs` |
| 137 | `png_get_unknown_chunks` | yes | yes | `src/pngget.rs` |
| 138 | `png_get_user_chunk_ptr` | yes | yes | `src/pngget.rs` |
| 139 | `png_get_user_height_max` | yes | yes | `src/pngget.rs` |
| 140 | `png_get_user_transform_ptr` | yes | yes | `src/pngtrans.rs` |
| 141 | `png_get_user_width_max` | yes | yes | `src/pngget.rs` |
| 142 | `png_get_valid` | yes | yes | `src/pngget.rs` |
| 143 | `png_get_x_offset_inches` | yes | yes | `src/pngget.rs` |
| 144 | `png_get_x_offset_inches_fixed` | yes | yes | `src/pngget.rs` |
| 145 | `png_get_x_offset_microns` | yes | yes | `src/pngget.rs` |
| 146 | `png_get_x_offset_pixels` | yes | yes | `src/pngget.rs` |
| 147 | `png_get_x_pixels_per_inch` | yes | yes | `src/pngget.rs` |
| 148 | `png_get_x_pixels_per_meter` | yes | yes | `src/pngget.rs` |
| 149 | `png_get_y_offset_inches` | yes | yes | `src/pngget.rs` |
| 150 | `png_get_y_offset_inches_fixed` | yes | yes | `src/pngget.rs` |
| 151 | `png_get_y_offset_microns` | yes | yes | `src/pngget.rs` |
| 152 | `png_get_y_offset_pixels` | yes | yes | `src/pngget.rs` |
| 153 | `png_get_y_pixels_per_inch` | yes | yes | `src/pngget.rs` |
| 154 | `png_get_y_pixels_per_meter` | yes | yes | `src/pngget.rs` |
| 155 | `png_handle_as_unknown` | yes | yes | `src/png_a.rs` |
| 156 | `png_handle_chunk` | yes | yes | `src/pngrutil_c.rs` |
| 157 | `png_handle_unknown` | yes | yes | `src/pngrutil_c.rs` |
| 158 | `png_icc_check_header` | yes | yes | `src/png_b.rs` |
| 159 | `png_icc_check_length` | yes | yes | `src/png_b.rs` |
| 160 | `png_icc_check_tag_table` | yes | yes | `src/png_b.rs` |
| 161 | `png_image_begin_read_from_file` | yes | yes | `src/pngread_b.rs` |
| 162 | `png_image_begin_read_from_memory` | yes | yes | `src/pngread_b.rs` |
| 163 | `png_image_begin_read_from_stdio` | yes | yes | `src/pngread_b.rs` |
| 164 | `png_image_error` | yes | yes | `src/png_d.rs` |
| 165 | `png_image_finish_read` | yes | yes | `src/pngread_d.rs` |
| 166 | `png_image_free` | yes | yes | `src/png_d.rs` |
| 167 | `png_image_write_to_file` | yes | yes | `src/pngwrite_b.rs` |
| 168 | `png_image_write_to_memory` | yes | yes | `src/pngwrite_b.rs` |
| 169 | `png_image_write_to_stdio` | yes | yes | `src/pngwrite_b.rs` |
| 170 | `png_info_init_3` | yes | yes | `src/png_a.rs` |
| 171 | `png_init_io` | yes | yes | `src/png_a.rs` |
| 172 | `png_init_read_transformations` | yes | yes | `src/pngrtran_b.rs` |
| 173 | `png_longjmp` | yes | yes | `src/pngerror.rs` |
| 174 | `png_malloc` | yes | yes | `src/pngmem.rs` |
| 175 | `png_malloc_array` | yes | yes | `src/pngmem.rs` |
| 176 | `png_malloc_base` | yes | yes | `src/pngmem.rs` |
| 177 | `png_malloc_default` | yes | yes | `src/pngmem.rs` |
| 178 | `png_malloc_warn` | yes | yes | `src/pngmem.rs` |
| 179 | `png_muldiv` | yes | yes | `src/png_d.rs` |
| 180 | `png_permit_mng_features` | yes | yes | `src/pngset_b.rs` |
| 181 | `png_process_IDAT_data` | yes | yes | `src/pngpread.rs` |
| 182 | `png_process_data` | yes | yes | `src/pngpread.rs` |
| 183 | `png_process_data_pause` | yes | yes | `src/pngpread.rs` |
| 184 | `png_process_data_skip` | yes | yes | `src/pngpread.rs` |
| 185 | `png_process_some_data` | yes | yes | `src/pngpread.rs` |
| 186 | `png_progressive_combine_row` | yes | yes | `src/pngpread.rs` |
| 187 | `png_push_fill_buffer` | yes | yes | `src/pngpread.rs` |
| 188 | `png_push_have_end` | yes | yes | `src/pngpread.rs` |
| 189 | `png_push_have_info` | yes | yes | `src/pngpread.rs` |
| 190 | `png_push_have_row` | yes | yes | `src/pngpread.rs` |
| 191 | `png_push_process_row` | yes | yes | `src/pngpread.rs` |
| 192 | `png_push_read_IDAT` | yes | yes | `src/pngpread.rs` |
| 193 | `png_push_read_chunk` | yes | yes | `src/pngpread.rs` |
| 194 | `png_push_read_sig` | yes | yes | `src/pngpread.rs` |
| 195 | `png_push_restore_buffer` | yes | yes | `src/pngpread.rs` |
| 196 | `png_push_save_buffer` | yes | yes | `src/pngpread.rs` |
| 197 | `png_read_IDAT_data` | yes | yes | `src/pngrutil_e.rs` |
| 198 | `png_read_chunk_header` | yes | yes | `src/pngrutil_a.rs` |
| 199 | `png_read_data` | yes | yes | `src/pngrio.rs` |
| 200 | `png_read_end` | yes | yes | `src/pngread_a.rs` |
| 201 | `png_read_filter_row` | yes | yes | `src/pngrutil_e.rs` |
| 202 | `png_read_finish_IDAT` | yes | yes | `src/pngrutil_e.rs` |
| 203 | `png_read_finish_row` | yes | yes | `src/pngrutil_e.rs` |
| 204 | `png_read_image` | yes | yes | `src/pngread_a.rs` |
| 205 | `png_read_info` | yes | yes | `src/pngread_a.rs` |
| 206 | `png_read_png` | yes | yes | `src/pngread_a.rs` |
| 207 | `png_read_push_finish_row` | yes | yes | `src/pngpread.rs` |
| 208 | `png_read_row` | yes | yes | `src/pngread_a.rs` |
| 209 | `png_read_rows` | yes | yes | `src/pngread_a.rs` |
| 210 | `png_read_sig` | yes | yes | `src/pngrutil_a.rs` |
| 211 | `png_read_start_row` | yes | yes | `src/pngrutil_e.rs` |
| 212 | `png_read_transform_info` | yes | yes | `src/pngrtran_b.rs` |
| 213 | `png_read_update_info` | yes | yes | `src/pngread_a.rs` |
| 214 | `png_realloc_array` | yes | yes | `src/pngmem.rs` |
| 215 | `png_reciprocal` | yes | yes | `src/png_d.rs` |
| 216 | `png_reciprocal2` | yes | yes | `src/png_d.rs` |
| 217 | `png_reset_crc` | yes | yes | `src/png_a.rs` |
| 218 | `png_reset_zstream` | yes | yes | `src/png_a.rs` |
| 219 | `png_resolve_file_gamma` | yes | yes | `src/pngrtran_b.rs` |
| 220 | `png_sRGB_base` | yes | yes | `src/srgb_tables.rs` |
| 221 | `png_sRGB_delta` | yes | yes | `src/srgb_tables.rs` |
| 222 | `png_sRGB_table` | yes | yes | `src/srgb_tables.rs` |
| 223 | `png_safe_error` | yes | yes | `src/pngerror.rs` |
| 224 | `png_safe_execute` | yes | yes | `src/pngerror.rs` |
| 225 | `png_safe_warning` | yes | yes | `src/pngerror.rs` |
| 226 | `png_safecat` | yes | yes | `src/pngerror.rs` |
| 227 | `png_save_int_32` | yes | yes | `src/png_a.rs` |
| 228 | `png_save_uint_16` | yes | yes | `src/pngwutil_a.rs` |
| 229 | `png_save_uint_32` | yes | yes | `src/pngwutil_a.rs` |
| 230 | `png_set_IHDR` | yes | yes | `src/pngset_a.rs` |
| 231 | `png_set_PLTE` | yes | yes | `src/pngset_a.rs` |
| 232 | `png_set_add_alpha` | yes | yes | `src/pngtrans.rs` |
| 233 | `png_set_alpha_mode` | yes | yes | `src/pngrtran_a.rs` |
| 234 | `png_set_alpha_mode_fixed` | yes | yes | `src/pngrtran_a.rs` |
| 235 | `png_set_bKGD` | yes | yes | `src/pngset_a.rs` |
| 236 | `png_set_background` | yes | yes | `src/pngrtran_a.rs` |
| 237 | `png_set_background_fixed` | yes | yes | `src/pngrtran_a.rs` |
| 238 | `png_set_benign_errors` | yes | yes | `src/pngset_b.rs` |
| 239 | `png_set_bgr` | yes | yes | `src/pngtrans.rs` |
| 240 | `png_set_cHRM` | yes | yes | `src/pngset_a.rs` |
| 241 | `png_set_cHRM_XYZ` | yes | yes | `src/pngset_a.rs` |
| 242 | `png_set_cHRM_XYZ_fixed` | yes | yes | `src/pngset_a.rs` |
| 243 | `png_set_cHRM_fixed` | yes | yes | `src/pngset_a.rs` |
| 244 | `png_set_cICP` | yes | yes | `src/pngset_a.rs` |
| 245 | `png_set_cLLI` | yes | yes | `src/pngset_a.rs` |
| 246 | `png_set_cLLI_fixed` | yes | yes | `src/pngset_a.rs` |
| 247 | `png_set_check_for_invalid_index` | yes | yes | `src/pngset_b.rs` |
| 248 | `png_set_chunk_cache_max` | yes | yes | `src/pngset_b.rs` |
| 249 | `png_set_chunk_malloc_max` | yes | yes | `src/pngset_b.rs` |
| 250 | `png_set_compression_buffer_size` | yes | yes | `src/pngset_b.rs` |
| 251 | `png_set_compression_level` | yes | yes | `src/pngwrite_a.rs` |
| 252 | `png_set_compression_mem_level` | yes | yes | `src/pngwrite_a.rs` |
| 253 | `png_set_compression_method` | yes | yes | `src/pngwrite_a.rs` |
| 254 | `png_set_compression_strategy` | yes | yes | `src/pngwrite_a.rs` |
| 255 | `png_set_compression_window_bits` | yes | yes | `src/pngwrite_a.rs` |
| 256 | `png_set_crc_action` | yes | yes | `src/pngrtran_a.rs` |
| 257 | `png_set_eXIf` | yes | yes | `src/pngset_a.rs` |
| 258 | `png_set_eXIf_1` | yes | yes | `src/pngset_a.rs` |
| 259 | `png_set_error_fn` | yes | yes | `src/pngerror.rs` |
| 260 | `png_set_expand` | yes | yes | `src/pngrtran_a.rs` |
| 261 | `png_set_expand_16` | yes | yes | `src/pngrtran_a.rs` |
| 262 | `png_set_expand_gray_1_2_4_to_8` | yes | yes | `src/pngrtran_a.rs` |
| 263 | `png_set_filler` | yes | yes | `src/pngtrans.rs` |
| 264 | `png_set_filter` | yes | yes | `src/pngwrite_a.rs` |
| 265 | `png_set_filter_heuristics` | yes | yes | `src/pngwrite_a.rs` |
| 266 | `png_set_filter_heuristics_fixed` | yes | yes | `src/pngwrite_a.rs` |
| 267 | `png_set_flush` | yes | yes | `src/pngwrite_a.rs` |
| 268 | `png_set_gAMA` | yes | yes | `src/pngset_a.rs` |
| 269 | `png_set_gAMA_fixed` | yes | yes | `src/pngset_a.rs` |
| 270 | `png_set_gamma` | yes | yes | `src/pngrtran_a.rs` |
| 271 | `png_set_gamma_fixed` | yes | yes | `src/pngrtran_a.rs` |
| 272 | `png_set_gray_to_rgb` | yes | yes | `src/pngrtran_a.rs` |
| 273 | `png_set_hIST` | yes | yes | `src/pngset_a.rs` |
| 274 | `png_set_iCCP` | yes | yes | `src/pngset_a.rs` |
| 275 | `png_set_interlace_handling` | yes | yes | `src/pngtrans.rs` |
| 276 | `png_set_invalid` | yes | yes | `src/pngset_b.rs` |
| 277 | `png_set_invert_alpha` | yes | yes | `src/pngtrans.rs` |
| 278 | `png_set_invert_mono` | yes | yes | `src/pngtrans.rs` |
| 279 | `png_set_keep_unknown_chunks` | yes | yes | `src/pngset_b.rs` |
| 280 | `png_set_longjmp_fn` | yes | yes | `src/pngerror.rs` |
| 281 | `png_set_mDCV` | yes | yes | `src/pngset_a.rs` |
| 282 | `png_set_mDCV_fixed` | yes | yes | `src/pngset_a.rs` |
| 283 | `png_set_mem_fn` | yes | yes | `src/pngmem.rs` |
| 284 | `png_set_oFFs` | yes | yes | `src/pngset_a.rs` |
| 285 | `png_set_option` | yes | yes | `src/png_d.rs` |
| 286 | `png_set_pCAL` | yes | yes | `src/pngset_a.rs` |
| 287 | `png_set_pHYs` | yes | yes | `src/pngset_a.rs` |
| 288 | `png_set_packing` | yes | yes | `src/pngtrans.rs` |
| 289 | `png_set_packswap` | yes | yes | `src/pngtrans.rs` |
| 290 | `png_set_palette_to_rgb` | yes | yes | `src/pngrtran_a.rs` |
| 291 | `png_set_progressive_read_fn` | yes | yes | `src/pngpread.rs` |
| 292 | `png_set_quantize` | yes | yes | `src/pngrtran_a.rs` |
| 293 | `png_set_read_fn` | yes | yes | `src/pngrio.rs` |
| 294 | `png_set_read_status_fn` | yes | yes | `src/pngread_a.rs` |
| 295 | `png_set_read_user_chunk_fn` | yes | yes | `src/pngset_b.rs` |
| 296 | `png_set_read_user_transform_fn` | yes | yes | `src/pngrtran_a.rs` |
| 297 | `png_set_rgb_coefficients` | yes | yes | `src/png_b.rs` |
| 298 | `png_set_rgb_to_gray` | yes | yes | `src/pngrtran_a.rs` |
| 299 | `png_set_rgb_to_gray_fixed` | yes | yes | `src/pngrtran_a.rs` |
| 300 | `png_set_rows` | yes | yes | `src/pngset_b.rs` |
| 301 | `png_set_sBIT` | yes | yes | `src/pngset_a.rs` |
| 302 | `png_set_sCAL` | yes | yes | `src/pngset_a.rs` |
| 303 | `png_set_sCAL_fixed` | yes | yes | `src/pngset_a.rs` |
| 304 | `png_set_sCAL_s` | yes | yes | `src/pngset_a.rs` |
| 305 | `png_set_sPLT` | yes | yes | `src/pngset_b.rs` |
| 306 | `png_set_sRGB` | yes | yes | `src/pngset_a.rs` |
| 307 | `png_set_sRGB_gAMA_and_cHRM` | yes | yes | `src/pngset_a.rs` |
| 308 | `png_set_scale_16` | yes | yes | `src/pngrtran_a.rs` |
| 309 | `png_set_shift` | yes | yes | `src/pngtrans.rs` |
| 310 | `png_set_sig_bytes` | yes | yes | `src/png_a.rs` |
| 311 | `png_set_strip_16` | yes | yes | `src/pngrtran_a.rs` |
| 312 | `png_set_strip_alpha` | yes | yes | `src/pngrtran_a.rs` |
| 313 | `png_set_swap` | yes | yes | `src/pngtrans.rs` |
| 314 | `png_set_swap_alpha` | yes | yes | `src/pngtrans.rs` |
| 315 | `png_set_tIME` | yes | yes | `src/pngset_b.rs` |
| 316 | `png_set_tRNS` | yes | yes | `src/pngset_b.rs` |
| 317 | `png_set_tRNS_to_alpha` | yes | yes | `src/pngrtran_a.rs` |
| 318 | `png_set_text` | yes | yes | `src/pngset_a.rs` |
| 319 | `png_set_text_2` | yes | yes | `src/pngset_a.rs` |
| 320 | `png_set_text_compression_level` | yes | yes | `src/pngwrite_a.rs` |
| 321 | `png_set_text_compression_mem_level` | yes | yes | `src/pngwrite_a.rs` |
| 322 | `png_set_text_compression_method` | yes | yes | `src/pngwrite_a.rs` |
| 323 | `png_set_text_compression_strategy` | yes | yes | `src/pngwrite_a.rs` |
| 324 | `png_set_text_compression_window_bits` | yes | yes | `src/pngwrite_a.rs` |
| 325 | `png_set_unknown_chunk_location` | yes | yes | `src/pngset_b.rs` |
| 326 | `png_set_unknown_chunks` | yes | yes | `src/pngset_b.rs` |
| 327 | `png_set_user_limits` | yes | yes | `src/pngset_b.rs` |
| 328 | `png_set_user_transform_info` | yes | yes | `src/pngtrans.rs` |
| 329 | `png_set_write_fn` | yes | yes | `src/pngwio.rs` |
| 330 | `png_set_write_status_fn` | yes | yes | `src/pngwrite_a.rs` |
| 331 | `png_set_write_user_transform_fn` | yes | yes | `src/pngwrite_a.rs` |
| 332 | `png_sig_cmp` | yes | yes | `src/png_a.rs` |
| 333 | `png_start_read_image` | yes | yes | `src/pngread_a.rs` |
| 334 | `png_user_version_check` | yes | yes | `src/png_a.rs` |
| 335 | `png_warning` | yes | yes | `src/pngerror.rs` |
| 336 | `png_warning_parameter` | yes | yes | `src/pngerror.rs` |
| 337 | `png_warning_parameter_signed` | yes | yes | `src/pngerror.rs` |
| 338 | `png_warning_parameter_unsigned` | yes | yes | `src/pngerror.rs` |
| 339 | `png_write_IEND` | yes | yes | `src/pngwutil_b.rs` |
| 340 | `png_write_IHDR` | yes | yes | `src/pngwutil_a.rs` |
| 341 | `png_write_PLTE` | yes | yes | `src/pngwutil_a.rs` |
| 342 | `png_write_bKGD` | yes | yes | `src/pngwutil_b.rs` |
| 343 | `png_write_cHRM_fixed` | yes | yes | `src/pngwutil_b.rs` |
| 344 | `png_write_cICP` | yes | yes | `src/pngwutil_b.rs` |
| 345 | `png_write_cLLI_fixed` | yes | yes | `src/pngwutil_b.rs` |
| 346 | `png_write_chunk` | yes | yes | `src/pngwutil_a.rs` |
| 347 | `png_write_chunk_data` | yes | yes | `src/pngwutil_a.rs` |
| 348 | `png_write_chunk_end` | yes | yes | `src/pngwutil_a.rs` |
| 349 | `png_write_chunk_start` | yes | yes | `src/pngwutil_a.rs` |
| 350 | `png_write_data` | yes | yes | `src/pngwio.rs` |
| 351 | `png_write_eXIf` | yes | yes | `src/pngwutil_b.rs` |
| 352 | `png_write_end` | yes | yes | `src/pngwrite_a.rs` |
| 353 | `png_write_find_filter` | yes | yes | `src/pngwutil_c.rs` |
| 354 | `png_write_finish_row` | yes | yes | `src/pngwutil_c.rs` |
| 355 | `png_write_flush` | yes | yes | `src/pngwrite_a.rs` |
| 356 | `png_write_gAMA_fixed` | yes | yes | `src/pngwutil_b.rs` |
| 357 | `png_write_hIST` | yes | yes | `src/pngwutil_b.rs` |
| 358 | `png_write_iCCP` | yes | yes | `src/pngwutil_b.rs` |
| 359 | `png_write_iTXt` | yes | yes | `src/pngwutil_b.rs` |
| 360 | `png_write_image` | yes | yes | `src/pngwrite_a.rs` |
| 361 | `png_write_info` | yes | yes | `src/pngwrite_a.rs` |
| 362 | `png_write_info_before_PLTE` | yes | yes | `src/pngwrite_a.rs` |
| 363 | `png_write_mDCV_fixed` | yes | yes | `src/pngwutil_b.rs` |
| 364 | `png_write_oFFs` | yes | yes | `src/pngwutil_b.rs` |
| 365 | `png_write_pCAL` | yes | yes | `src/pngwutil_b.rs` |
| 366 | `png_write_pHYs` | yes | yes | `src/pngwutil_b.rs` |
| 367 | `png_write_png` | yes | yes | `src/pngwrite_a.rs` |
| 368 | `png_write_row` | yes | yes | `src/pngwrite_a.rs` |
| 369 | `png_write_rows` | yes | yes | `src/pngwrite_a.rs` |
| 370 | `png_write_sBIT` | yes | yes | `src/pngwutil_b.rs` |
| 371 | `png_write_sCAL_s` | yes | yes | `src/pngwutil_b.rs` |
| 372 | `png_write_sPLT` | yes | yes | `src/pngwutil_b.rs` |
| 373 | `png_write_sRGB` | yes | yes | `src/pngwutil_b.rs` |
| 374 | `png_write_sig` | yes | yes | `src/pngwutil_a.rs` |
| 375 | `png_write_start_row` | yes | yes | `src/pngwutil_c.rs` |
| 376 | `png_write_tEXt` | yes | yes | `src/pngwutil_b.rs` |
| 377 | `png_write_tIME` | yes | yes | `src/pngwutil_b.rs` |
| 378 | `png_write_tRNS` | yes | yes | `src/pngwutil_b.rs` |
| 379 | `png_write_zTXt` | yes | yes | `src/pngwutil_b.rs` |
| 380 | `png_xy_from_XYZ` | yes | yes | `src/png_b.rs` |
| 381 | `png_zalloc` | yes | yes | `src/png_a.rs` |
| 382 | `png_zfree` | yes | yes | `src/png_a.rs` |
| 383 | `png_zlib_inflate` | yes | yes | `src/pngrutil_a.rs` |
| 384 | `png_zstream_error` | yes | yes | `src/png_a.rs` |

---

## Appendix — how many of the exported symbols the tests actually CALL

Symbol parity (above) only proves the Rust `.so` *exports* the same names.  The
differential suite additionally *invokes* them through the FFI boundary.  Count
produced mechanically by grepping `tests/**.rs` for `.<symbol>` call sites:

```
exported by the C .so : 384
invoked by the tests  : 370
not invoked directly  : 14
```

The remaining symbols are private pipeline steps that cannot be called in
isolation without hand-forging internal `png_struct` state (which would be C
undefined behaviour).  Every one of them IS executed, indirectly, by the
composed pipelines the suite drives -- they are the internals of
`png_read_row` / `png_write_row` / `png_process_data`:

* `png_compress_IDAT`
* `png_process_IDAT_data`
* `png_push_have_end`
* `png_push_have_info`
* `png_push_have_row`
* `png_push_process_row`
* `png_read_IDAT_data`
* `png_read_finish_IDAT`
* `png_read_finish_row`
* `png_read_push_finish_row`
* `png_safe_error`
* `png_safe_warning`
* `png_write_start_row`
* `png_zlib_inflate`

