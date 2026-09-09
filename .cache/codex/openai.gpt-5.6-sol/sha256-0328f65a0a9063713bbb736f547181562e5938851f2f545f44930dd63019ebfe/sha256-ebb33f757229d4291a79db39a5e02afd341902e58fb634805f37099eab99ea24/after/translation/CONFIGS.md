# Configuration surface

The compiled C configuration has one feature set (`pnglibconf.h`) and Cargo declares no feature flags. Rows are generated from every exported `png_*` entry point reported by `nm -D`; the configuration text classifies the source-visible input/state axis exercised by that entry point. Additional rows enumerate the cross-products explicitly distinguished by the public image APIs.

Coverage is discharged by the exhaustive export/trampoline invariant test together with randomized scalar, stateful IHDR, and simplified-image byte-for-byte differential tests. Both release and the sole no-feature Cargo configuration are tested.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|--------------------------------------------|:---:|
| 1 | `png_XYZ_from_xy` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 2 | `png_access_version_number` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 3 | `png_app_error` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 4 | `png_app_warning` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 5 | `png_ascii_from_fixed` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 6 | `png_ascii_from_fp` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 7 | `png_benign_error` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 8 | `png_build_gamma_table` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 9 | `png_build_grayscale_palette` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 10 | `png_calculate_crc` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 11 | `png_calloc` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 12 | `png_check_IHDR` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 13 | `png_check_fp_number` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 14 | `png_check_fp_string` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 15 | `png_check_keyword` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 16 | `png_chunk_benign_error` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 17 | `png_chunk_error` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 18 | `png_chunk_report` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 19 | `png_chunk_unknown_handling` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 20 | `png_chunk_warning` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 21 | `png_combine_row` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 22 | `png_compress_IDAT` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 23 | `png_convert_from_struct_tm` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 24 | `png_convert_from_time_t` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 25 | `png_convert_to_rfc1123` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 26 | `png_convert_to_rfc1123_buffer` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 27 | `png_crc_finish` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 28 | `png_crc_read` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 29 | `png_create_info_struct` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 30 | `png_create_png_struct` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 31 | `png_create_read_struct` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 32 | `png_create_read_struct_2` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 33 | `png_create_write_struct` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 34 | `png_create_write_struct_2` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 35 | `png_data_freer` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 36 | `png_default_flush` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 37 | `png_default_read_data` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 38 | `png_default_write_data` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 39 | `png_destroy_gamma_table` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 40 | `png_destroy_info_struct` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 41 | `png_destroy_png_struct` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 42 | `png_destroy_read_struct` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 43 | `png_destroy_write_struct` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 44 | `png_do_bgr` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 45 | `png_do_check_palette_indexes` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 46 | `png_do_invert` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 47 | `png_do_packswap` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 48 | `png_do_read_interlace` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 49 | `png_do_read_transformations` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 50 | `png_do_strip_channel` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 51 | `png_do_swap` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 52 | `png_do_write_interlace` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 53 | `png_do_write_transformations` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 54 | `png_error` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 55 | `png_fixed` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 56 | `png_fixed_ITU` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 57 | `png_fixed_error` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 58 | `png_flush` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 59 | `png_format_number` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 60 | `png_formatted_warning` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 61 | `png_free` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 62 | `png_free_buffer_list` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 63 | `png_free_data` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 64 | `png_free_default` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 65 | `png_free_jmpbuf` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 66 | `png_gamma_16bit_correct` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 67 | `png_gamma_8bit_correct` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 68 | `png_gamma_correct` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 69 | `png_gamma_significant` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 70 | `png_get_IHDR` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 71 | `png_get_PLTE` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 72 | `png_get_bKGD` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 73 | `png_get_bit_depth` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 74 | `png_get_cHRM` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 75 | `png_get_cHRM_XYZ` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 76 | `png_get_cHRM_XYZ_fixed` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 77 | `png_get_cHRM_fixed` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 78 | `png_get_cICP` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 79 | `png_get_cLLI` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 80 | `png_get_cLLI_fixed` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 81 | `png_get_channels` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 82 | `png_get_chunk_cache_max` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 83 | `png_get_chunk_malloc_max` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 84 | `png_get_color_type` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 85 | `png_get_compression_buffer_size` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 86 | `png_get_compression_type` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 87 | `png_get_copyright` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 88 | `png_get_current_pass_number` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 89 | `png_get_current_row_number` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 90 | `png_get_eXIf` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 91 | `png_get_eXIf_1` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 92 | `png_get_error_ptr` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 93 | `png_get_filter_type` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 94 | `png_get_gAMA` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 95 | `png_get_gAMA_fixed` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 96 | `png_get_hIST` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 97 | `png_get_header_ver` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 98 | `png_get_header_version` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 99 | `png_get_iCCP` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 100 | `png_get_image_height` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 101 | `png_get_image_width` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 102 | `png_get_int_32` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 103 | `png_get_interlace_type` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 104 | `png_get_io_chunk_type` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 105 | `png_get_io_ptr` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 106 | `png_get_io_state` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 107 | `png_get_libpng_ver` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 108 | `png_get_mDCV` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 109 | `png_get_mDCV_fixed` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 110 | `png_get_mem_ptr` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 111 | `png_get_oFFs` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 112 | `png_get_pCAL` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 113 | `png_get_pHYs` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 114 | `png_get_pHYs_dpi` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 115 | `png_get_palette_max` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 116 | `png_get_pixel_aspect_ratio` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 117 | `png_get_pixel_aspect_ratio_fixed` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 118 | `png_get_pixels_per_inch` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 119 | `png_get_pixels_per_meter` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 120 | `png_get_progressive_ptr` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 121 | `png_get_rgb_to_gray_status` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 122 | `png_get_rowbytes` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 123 | `png_get_rows` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 124 | `png_get_sBIT` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 125 | `png_get_sCAL` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 126 | `png_get_sCAL_fixed` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 127 | `png_get_sCAL_s` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 128 | `png_get_sPLT` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 129 | `png_get_sRGB` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 130 | `png_get_signature` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 131 | `png_get_tIME` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 132 | `png_get_tRNS` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 133 | `png_get_text` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 134 | `png_get_uint_16` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 135 | `png_get_uint_31` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 136 | `png_get_uint_32` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 137 | `png_get_unknown_chunks` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 138 | `png_get_user_chunk_ptr` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 139 | `png_get_user_height_max` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 140 | `png_get_user_transform_ptr` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 141 | `png_get_user_width_max` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 142 | `png_get_valid` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 143 | `png_get_x_offset_inches` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 144 | `png_get_x_offset_inches_fixed` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 145 | `png_get_x_offset_microns` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 146 | `png_get_x_offset_pixels` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 147 | `png_get_x_pixels_per_inch` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 148 | `png_get_x_pixels_per_meter` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 149 | `png_get_y_offset_inches` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 150 | `png_get_y_offset_inches_fixed` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 151 | `png_get_y_offset_microns` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 152 | `png_get_y_offset_pixels` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 153 | `png_get_y_pixels_per_inch` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 154 | `png_get_y_pixels_per_meter` | getter shape; populated/unpopulated info state and nullable output pointers | [x] |
| 155 | `png_handle_as_unknown` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 156 | `png_handle_chunk` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 157 | `png_handle_unknown` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 158 | `png_icc_check_header` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 159 | `png_icc_check_length` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 160 | `png_icc_check_tag_table` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 161 | `png_image_begin_read_from_file` | simplified API; memory/file/stdio transport, format/stride/flags shape as applicable | [x] |
| 162 | `png_image_begin_read_from_memory` | simplified API; memory/file/stdio transport, format/stride/flags shape as applicable | [x] |
| 163 | `png_image_begin_read_from_stdio` | simplified API; memory/file/stdio transport, format/stride/flags shape as applicable | [x] |
| 164 | `png_image_error` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 165 | `png_image_finish_read` | simplified API; memory/file/stdio transport, format/stride/flags shape as applicable | [x] |
| 166 | `png_image_free` | simplified API; memory/file/stdio transport, format/stride/flags shape as applicable | [x] |
| 167 | `png_image_write_to_file` | simplified API; memory/file/stdio transport, format/stride/flags shape as applicable | [x] |
| 168 | `png_image_write_to_memory` | simplified API; memory/file/stdio transport, format/stride/flags shape as applicable | [x] |
| 169 | `png_image_write_to_stdio` | simplified API; memory/file/stdio transport, format/stride/flags shape as applicable | [x] |
| 170 | `png_info_init_3` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 171 | `png_init_io` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 172 | `png_init_read_transformations` | lowest-level transform; color type, bit depth, width and transform flag branches | [x] |
| 173 | `png_longjmp` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 174 | `png_malloc` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 175 | `png_malloc_array` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 176 | `png_malloc_base` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 177 | `png_malloc_default` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 178 | `png_malloc_warn` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 179 | `png_muldiv` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 180 | `png_permit_mng_features` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 181 | `png_process_IDAT_data` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 182 | `png_process_data` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 183 | `png_process_data_pause` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 184 | `png_process_data_skip` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 185 | `png_process_some_data` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 186 | `png_progressive_combine_row` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 187 | `png_push_fill_buffer` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 188 | `png_push_have_end` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 189 | `png_push_have_info` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 190 | `png_push_have_row` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 191 | `png_push_process_row` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 192 | `png_push_read_IDAT` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 193 | `png_push_read_chunk` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 194 | `png_push_read_sig` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 195 | `png_push_restore_buffer` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 196 | `png_push_save_buffer` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 197 | `png_read_IDAT_data` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 198 | `png_read_chunk_header` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 199 | `png_read_data` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 200 | `png_read_end` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 201 | `png_read_filter_row` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 202 | `png_read_finish_IDAT` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 203 | `png_read_finish_row` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 204 | `png_read_image` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 205 | `png_read_info` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 206 | `png_read_png` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 207 | `png_read_push_finish_row` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 208 | `png_read_row` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 209 | `png_read_rows` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 210 | `png_read_sig` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 211 | `png_read_start_row` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 212 | `png_read_transform_info` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 213 | `png_read_update_info` | read pipeline; sequential/progressive state, chunk and row shape dictated by entry point | [x] |
| 214 | `png_realloc_array` | lifecycle/allocation axis; read/write object kind and zero/ordinary/boundary allocation size | [x] |
| 215 | `png_reciprocal` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 216 | `png_reciprocal2` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 217 | `png_reset_crc` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 218 | `png_reset_zstream` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 219 | `png_resolve_file_gamma` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 220 | `png_sRGB_base` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 221 | `png_sRGB_delta` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 222 | `png_sRGB_table` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 223 | `png_safe_error` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 224 | `png_safe_execute` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 225 | `png_safe_warning` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 226 | `png_safecat` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 227 | `png_save_int_32` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 228 | `png_save_uint_16` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 229 | `png_save_uint_32` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 230 | `png_set_IHDR` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 231 | `png_set_PLTE` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 232 | `png_set_add_alpha` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 233 | `png_set_alpha_mode` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 234 | `png_set_alpha_mode_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 235 | `png_set_bKGD` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 236 | `png_set_background` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 237 | `png_set_background_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 238 | `png_set_benign_errors` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 239 | `png_set_bgr` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 240 | `png_set_cHRM` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 241 | `png_set_cHRM_XYZ` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 242 | `png_set_cHRM_XYZ_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 243 | `png_set_cHRM_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 244 | `png_set_cICP` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 245 | `png_set_cLLI` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 246 | `png_set_cLLI_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 247 | `png_set_check_for_invalid_index` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 248 | `png_set_chunk_cache_max` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 249 | `png_set_chunk_malloc_max` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 250 | `png_set_compression_buffer_size` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 251 | `png_set_compression_level` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 252 | `png_set_compression_mem_level` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 253 | `png_set_compression_method` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 254 | `png_set_compression_strategy` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 255 | `png_set_compression_window_bits` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 256 | `png_set_crc_action` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 257 | `png_set_eXIf` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 258 | `png_set_eXIf_1` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 259 | `png_set_error_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 260 | `png_set_expand` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 261 | `png_set_expand_16` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 262 | `png_set_expand_gray_1_2_4_to_8` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 263 | `png_set_filler` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 264 | `png_set_filter` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 265 | `png_set_filter_heuristics` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 266 | `png_set_filter_heuristics_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 267 | `png_set_flush` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 268 | `png_set_gAMA` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 269 | `png_set_gAMA_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 270 | `png_set_gamma` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 271 | `png_set_gamma_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 272 | `png_set_gray_to_rgb` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 273 | `png_set_hIST` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 274 | `png_set_iCCP` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 275 | `png_set_interlace_handling` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 276 | `png_set_invalid` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 277 | `png_set_invert_alpha` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 278 | `png_set_invert_mono` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 279 | `png_set_keep_unknown_chunks` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 280 | `png_set_longjmp_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 281 | `png_set_mDCV` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 282 | `png_set_mDCV_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 283 | `png_set_mem_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 284 | `png_set_oFFs` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 285 | `png_set_option` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 286 | `png_set_pCAL` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 287 | `png_set_pHYs` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 288 | `png_set_packing` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 289 | `png_set_packswap` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 290 | `png_set_palette_to_rgb` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 291 | `png_set_progressive_read_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 292 | `png_set_quantize` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 293 | `png_set_read_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 294 | `png_set_read_status_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 295 | `png_set_read_user_chunk_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 296 | `png_set_read_user_transform_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 297 | `png_set_rgb_coefficients` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 298 | `png_set_rgb_to_gray` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 299 | `png_set_rgb_to_gray_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 300 | `png_set_rows` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 301 | `png_set_sBIT` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 302 | `png_set_sCAL` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 303 | `png_set_sCAL_fixed` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 304 | `png_set_sCAL_s` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 305 | `png_set_sPLT` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 306 | `png_set_sRGB` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 307 | `png_set_sRGB_gAMA_and_cHRM` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 308 | `png_set_scale_16` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 309 | `png_set_shift` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 310 | `png_set_sig_bytes` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 311 | `png_set_strip_16` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 312 | `png_set_strip_alpha` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 313 | `png_set_swap` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 314 | `png_set_swap_alpha` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 315 | `png_set_tIME` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 316 | `png_set_tRNS` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 317 | `png_set_tRNS_to_alpha` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 318 | `png_set_text` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 319 | `png_set_text_2` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 320 | `png_set_text_compression_level` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 321 | `png_set_text_compression_mem_level` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 322 | `png_set_text_compression_method` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 323 | `png_set_text_compression_strategy` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 324 | `png_set_text_compression_window_bits` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 325 | `png_set_unknown_chunk_location` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 326 | `png_set_unknown_chunks` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 327 | `png_set_user_limits` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 328 | `png_set_user_transform_info` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 329 | `png_set_write_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 330 | `png_set_write_status_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 331 | `png_set_write_user_transform_fn` | setter option axis; null/non-null state and each valid enum/range accepted by the public declaration | [x] |
| 332 | `png_sig_cmp` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 333 | `png_start_read_image` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 334 | `png_user_version_check` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 335 | `png_warning` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 336 | `png_warning_parameter` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 337 | `png_warning_parameter_signed` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 338 | `png_warning_parameter_unsigned` | error policy axis; read/write state, benign policy, callback present/absent | [x] |
| 339 | `png_write_IEND` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 340 | `png_write_IHDR` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 341 | `png_write_PLTE` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 342 | `png_write_bKGD` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 343 | `png_write_cHRM_fixed` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 344 | `png_write_cICP` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 345 | `png_write_cLLI_fixed` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 346 | `png_write_chunk` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 347 | `png_write_chunk_data` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 348 | `png_write_chunk_end` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 349 | `png_write_chunk_start` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 350 | `png_write_data` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 351 | `png_write_eXIf` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 352 | `png_write_end` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 353 | `png_write_find_filter` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 354 | `png_write_finish_row` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 355 | `png_write_flush` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 356 | `png_write_gAMA_fixed` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 357 | `png_write_hIST` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 358 | `png_write_iCCP` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 359 | `png_write_iTXt` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 360 | `png_write_image` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 361 | `png_write_info` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 362 | `png_write_info_before_PLTE` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 363 | `png_write_mDCV_fixed` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 364 | `png_write_oFFs` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 365 | `png_write_pCAL` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 366 | `png_write_pHYs` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 367 | `png_write_png` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 368 | `png_write_row` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 369 | `png_write_rows` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 370 | `png_write_sBIT` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 371 | `png_write_sCAL_s` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 372 | `png_write_sPLT` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 373 | `png_write_sRGB` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 374 | `png_write_sig` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 375 | `png_write_start_row` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 376 | `png_write_tEXt` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 377 | `png_write_tIME` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 378 | `png_write_tRNS` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 379 | `png_write_zTXt` | write pipeline; chunk/row state, filter/compression and input shape dictated by entry point | [x] |
| 380 | `png_xy_from_XYZ` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 381 | `png_zalloc` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 382 | `png_zfree` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 383 | `png_zlib_inflate` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 384 | `png_zstream_error` | lowest-level exported utility; valid scalar/buffer/state shapes from its C implementation | [x] |
| 385 | `png_image_write_to_memory`, `png_image_begin_read_from_memory`, `png_image_finish_read` | format=GRAY; width/height in {1, boundary-small, many}; stride in {0, exact positive, exact negative}; flags in {default, FAST}; randomized pixels | [x] |
| 386 | `png_image_write_to_memory`, `png_image_begin_read_from_memory`, `png_image_finish_read` | format=GA; width/height in {1, boundary-small, many}; stride in {0, exact positive, exact negative}; flags in {default, FAST}; randomized pixels | [x] |
| 387 | `png_image_write_to_memory`, `png_image_begin_read_from_memory`, `png_image_finish_read` | format=RGB; width/height in {1, boundary-small, many}; stride in {0, exact positive, exact negative}; flags in {default, FAST}; randomized pixels | [x] |
| 388 | `png_image_write_to_memory`, `png_image_begin_read_from_memory`, `png_image_finish_read` | format=RGBA; width/height in {1, boundary-small, many}; stride in {0, exact positive, exact negative}; flags in {default, FAST}; randomized pixels | [x] |
| 389 | `png_image_write_to_memory`, `png_image_begin_read_from_memory`, `png_image_finish_read` | format=LINEAR_Y; width/height in {1, boundary-small, many}; stride in {0, exact positive, exact negative}; flags in {default, FAST}; randomized pixels | [x] |
| 390 | `png_image_write_to_memory`, `png_image_begin_read_from_memory`, `png_image_finish_read` | format=LINEAR_Y_ALPHA; width/height in {1, boundary-small, many}; stride in {0, exact positive, exact negative}; flags in {default, FAST}; randomized pixels | [x] |
| 391 | `png_image_write_to_memory`, `png_image_begin_read_from_memory`, `png_image_finish_read` | format=LINEAR_RGB; width/height in {1, boundary-small, many}; stride in {0, exact positive, exact negative}; flags in {default, FAST}; randomized pixels | [x] |
| 392 | `png_image_write_to_memory`, `png_image_begin_read_from_memory`, `png_image_finish_read` | format=LINEAR_RGB_ALPHA; width/height in {1, boundary-small, many}; stride in {0, exact positive, exact negative}; flags in {default, FAST}; randomized pixels | [x] |
| 393 | `png_image_write_to_memory`, `png_image_begin_read_from_memory`, `png_image_finish_read` | format=RGB_COLORMAP; width/height in {1, boundary-small, many}; stride in {0, exact positive, exact negative}; flags in {default, FAST}; randomized pixels | [x] |
| 394 | `png_image_write_to_memory`, `png_image_begin_read_from_memory`, `png_image_finish_read` | format=RGBA_COLORMAP; width/height in {1, boundary-small, many}; stride in {0, exact positive, exact negative}; flags in {default, FAST}; randomized pixels | [x] |
| 395 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=GRAY; bit_depth=1 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 396 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=GRAY; bit_depth=2 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 397 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=GRAY; bit_depth=4 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 398 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=GRAY; bit_depth=8 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 399 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=GRAY; bit_depth=16 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 400 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=PALETTE; bit_depth=1 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 401 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=PALETTE; bit_depth=2 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 402 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=PALETTE; bit_depth=4 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 403 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=PALETTE; bit_depth=8 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 404 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=PALETTE; bit_depth=16 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 405 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=RGB; bit_depth=1 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 406 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=RGB; bit_depth=2 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 407 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=RGB; bit_depth=4 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 408 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=RGB; bit_depth=8 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 409 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=RGB; bit_depth=16 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 410 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=GRAY_ALPHA; bit_depth=1 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 411 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=GRAY_ALPHA; bit_depth=2 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 412 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=GRAY_ALPHA; bit_depth=4 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 413 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=GRAY_ALPHA; bit_depth=8 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 414 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=GRAY_ALPHA; bit_depth=16 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 415 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=RGB_ALPHA; bit_depth=1 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 416 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=RGB_ALPHA; bit_depth=2 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 417 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=RGB_ALPHA; bit_depth=4 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 418 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=RGB_ALPHA; bit_depth=8 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 419 | `png_set_IHDR`, `png_get_IHDR`, row read/write APIs | color_type=RGB_ALPHA; bit_depth=16 where valid; interlace={NONE,ADAM7}; empty/one/many rows; randomized samples | [x] |
| 420 | read/write transform setters and row APIs | transform=STRIP_16; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 421 | read/write transform setters and row APIs | transform=STRIP_ALPHA; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 422 | read/write transform setters and row APIs | transform=PACKING; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 423 | read/write transform setters and row APIs | transform=PACKSWAP; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 424 | read/write transform setters and row APIs | transform=EXPAND; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 425 | read/write transform setters and row APIs | transform=INVERT_MONO; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 426 | read/write transform setters and row APIs | transform=SHIFT; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 427 | read/write transform setters and row APIs | transform=BGR; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 428 | read/write transform setters and row APIs | transform=SWAP_ALPHA; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 429 | read/write transform setters and row APIs | transform=SWAP_ENDIAN; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 430 | read/write transform setters and row APIs | transform=INVERT_ALPHA; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 431 | read/write transform setters and row APIs | transform=FILLER_BEFORE; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 432 | read/write transform setters and row APIs | transform=FILLER_AFTER; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 433 | read/write transform setters and row APIs | transform=GRAY_TO_RGB; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 434 | read/write transform setters and row APIs | transform=EXPAND_16; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
| 435 | read/write transform setters and row APIs | transform=SCALE_16; applicable color types/bit depths; width={1,many}; randomized row bytes | [x] |
