# Error Surface

Mechanically extracted from C error calls, assertion statements, and explicit error/sentinel returns.
Each predicate and result includes its C source location; rows remain unchecked until a differential test reaches that exact branch.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---:|----------|---------------------------------------------|-------------------|:---:|
| 1 | `png_set_sig_bytes` | `c_src/src/png.c:66: (nb > 8)` | `error callback/longjmp: png_error(png_ptr, "Too many bytes for PNG signature");` | [ ] |
| 2 | `png_sig_cmp` | `c_src/src/png.c:88: (num_to_check < 1)` | `sentinel return: return -1;` | [x] |
| 3 | `png_sig_cmp` | `c_src/src/png.c:91: (start > 7)` | `sentinel return: return -1;` | [x] |
| 4 | `png_zalloc` | `c_src/src/png.c:110: (png_ptr == NULL)` | `sentinel return: return NULL;` | [x] |
| 5 | `png_zalloc` | `c_src/src/png.c:122: (size != 0 && items >= (~(png_alloc_size_t)0) / size)` | `sentinel return: return NULL;` | [ ] |
| 6 | `png_create_png_struct` | `c_src/src/png.c:361: unconditional rejection/state failure` | `sentinel return: return NULL;` | [ ] |
| 7 | `png_create_info_struct` | `c_src/src/png.c:374: (png_ptr == NULL)` | `sentinel return: return NULL;` | [x] |
| 8 | `png_data_freer` | `c_src/src/png.c:479: (freer == PNG_USER_WILL_FREE_DATA)` | `error callback/longjmp: png_error(png_ptr, "Unknown freer parameter in png_data_freer");` | [ ] |
| 9 | `png_get_io_ptr` | `c_src/src/png.c:694: (png_ptr == NULL)` | `sentinel return: return NULL;` | [x] |
| 10 | `png_convert_to_rfc1123` | `c_src/src/png.c:808: (png_convert_to_rfc1123_buffer(png_ptr->time_buffer, ptime) == 0)` | `sentinel return: return NULL;` | [ ] |
| 11 | `png_reset_zstream` | `c_src/src/png.c:982: (png_ptr == NULL)` | `sentinel return: return Z_STREAM_ERROR;` | [x] |
| 12 | `png_icc_profile_error` | `c_src/src/png.c:1571: unconditional rejection/state failure` | `error callback/longjmp: png_chunk_benign_error(png_ptr, message);` | [ ] |
| 13 | `icc_check_length` | `c_src/src/png.c:1589: (profile_length < 132)` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, profile_length, "too short");` | [ ] |
| 14 | `png_icc_check_length` | `c_src/src/png.c:1607: (profile_length > png_chunk_max(png_ptr))` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, profile_length, "profile too long");` | [ ] |
| 15 | `png_icc_check_header` | `c_src/src/png.c:1627: (temp != profile_length)` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, temp, "length does not match profile");` | [ ] |
| 16 | `png_icc_check_header` | `c_src/src/png.c:1632: (temp > 3 && (profile_length & 3))` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, profile_length, "invalid length");` | [ ] |
| 17 | `png_icc_check_header` | `c_src/src/png.c:1638: (temp > 357913930 \|\| profile_length < 132+12*temp)` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, temp, "tag count too large");` | [ ] |
| 18 | `png_icc_check_header` | `c_src/src/png.c:1646: (temp >= 0xffff)` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, temp, "invalid rendering intent");` | [ ] |
| 19 | `png_icc_check_header` | `c_src/src/png.c:1653: (temp >= PNG_sRGB_INTENT_LAST)` | `error callback/longjmp: (void)png_icc_profile_error(png_ptr, name, temp, "intent outside defined range");` | [ ] |
| 20 | `png_icc_check_header` | `c_src/src/png.c:1670: (temp != 0x61637370)` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, temp, "invalid signature");` | [ ] |
| 21 | `png_icc_check_header` | `c_src/src/png.c:1681: (memcmp(profile+68, D50_nCIEXYZ, 12) != 0)` | `error callback/longjmp: (void)png_icc_profile_error(png_ptr, name, 0/*no tag value*/, "PCS illuminant is not D50");` | [ ] |
| 22 | `png_icc_check_header` | `c_src/src/png.c:1709: ((color_type & PNG_COLOR_MASK_COLOR) == 0)` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, temp, "RGB color space not permitted on grayscale PNG");` | [ ] |
| 23 | `png_icc_check_header` | `c_src/src/png.c:1715: ((color_type & PNG_COLOR_MASK_COLOR) != 0)` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, temp, "Gray color space not permitted on RGB PNG");` | [ ] |
| 24 | `png_icc_check_header` | `c_src/src/png.c:1720: ((color_type & PNG_COLOR_MASK_COLOR) != 0)` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, temp, "invalid ICC profile color space");` | [ ] |
| 25 | `png_icc_check_header` | `c_src/src/png.c:1745: unconditional rejection/state failure` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, temp, "invalid embedded Abstract ICC profile");` | [ ] |
| 26 | `png_icc_check_header` | `c_src/src/png.c:1755: unconditional rejection/state failure` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, temp, "unexpected DeviceLink ICC profile class");` | [ ] |
| 27 | `png_icc_check_header` | `c_src/src/png.c:1763: unconditional rejection/state failure` | `error callback/longjmp: (void)png_icc_profile_error(png_ptr, name, temp, "unexpected NamedColor ICC profile class");` | [ ] |
| 28 | `png_icc_check_header` | `c_src/src/png.c:1773: unconditional rejection/state failure` | `error callback/longjmp: (void)png_icc_profile_error(png_ptr, name, temp, "unrecognized ICC profile class");` | [ ] |
| 29 | `png_icc_check_header` | `c_src/src/png.c:1789: unconditional rejection/state failure` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, temp, "unexpected ICC PCS encoding");` | [ ] |
| 30 | `png_icc_check_tag_table` | `c_src/src/png.c:1825: (tag_start > profile_length \|\| tag_length > profile_length - tag_start)` | `error callback/longjmp: return png_icc_profile_error(png_ptr, name, tag_id, "ICC profile tag outside profile");` | [ ] |
| 31 | `png_icc_check_tag_table` | `c_src/src/png.c:1834: ((tag_start & 3) != 0)` | `error callback/longjmp: (void)png_icc_profile_error(png_ptr, name, tag_id, "ICC profile tag start not a multiple of 4");` | [ ] |
| 32 | `png_set_rgb_coefficients` | `c_src/src/png.c:1938: (r+g+b != 32768)` | `error callback/longjmp: png_error(png_ptr, "internal error handling cHRM coefficients");` | [ ] |
| 33 | `png_check_IHDR` | `c_src/src/png.c:2121: (error == 1)` | `error callback/longjmp: png_error(png_ptr, "Invalid IHDR data");` | [ ] |
| 34 | `png_ascii_from_fp` | `c_src/src/png.c:2635: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "ASCII conversion buffer too small");` | [ ] |
| 35 | `png_ascii_from_fixed` | `c_src/src/png.c:2713: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "ASCII conversion buffer too small");` | [ ] |
| 36 | `png_fixed` | `c_src/src/png.c:2731: (r > 2147483647. \|\| r < -2147483648.)` | `error callback/longjmp: png_fixed_error(png_ptr, text);` | [ ] |
| 37 | `png_fixed_ITU` | `c_src/src/png.c:2750: (r > 2147483647. \|\| r < 0)` | `error callback/longjmp: png_fixed_error(png_ptr, text);` | [ ] |
| 38 | `png_set_option` | `c_src/src/png.c:3783: (png_ptr != NULL && option >= 0 && option < PNG_OPTION_NEXT && (option & 1) == 0)` | `sentinel return: return PNG_OPTION_INVALID;` | [x] |
| 39 | `png_image_free_function` | `c_src/src/png.c:4002: (c.for_write != 0)` | `error callback/longjmp: png_error(c.png_ptr, "simplified write not supported");` | [ ] |
| 40 | `png_image_free_function` | `c_src/src/png.c:4010: unconditional rejection/state failure` | `error callback/longjmp: png_error(c.png_ptr, "simplified read not supported");` | [ ] |
| 41 | `png_error` | `c_src/src/pngerror.c:48: (png_ptr != NULL && png_ptr->error_fn != NULL)` | `error callback/longjmp: png_default_error(png_ptr, error_message);` | [ ] |
| 42 | `png_benign_error` | `c_src/src/pngerror.c:326: ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0 && png_ptr->chunk_name != 0)` | `error callback/longjmp: png_chunk_error(png_ptr, error_message);` | [ ] |
| 43 | `png_benign_error` | `c_src/src/pngerror.c:329: ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0 && png_ptr->chunk_name != 0)` | `error callback/longjmp: png_error(png_ptr, error_message);` | [ ] |
| 44 | `png_app_warning` | `c_src/src/pngerror.c:343: ((png_ptr->flags & PNG_FLAG_APP_WARNINGS_WARN) != 0)` | `error callback/longjmp: png_error(png_ptr, error_message);` | [ ] |
| 45 | `png_app_error` | `c_src/src/pngerror.c:356: ((png_ptr->flags & PNG_FLAG_APP_ERRORS_WARN) != 0)` | `error callback/longjmp: png_error(png_ptr, error_message);` | [ ] |
| 46 | `png_chunk_error` | `c_src/src/pngerror.c:431: (png_ptr == NULL)` | `error callback/longjmp: png_error(png_ptr, error_message);` | [ ] |
| 47 | `png_chunk_error` | `c_src/src/pngerror.c:436: (png_ptr == NULL)` | `error callback/longjmp: png_error(png_ptr, msg);` | [ ] |
| 48 | `png_chunk_benign_error` | `c_src/src/pngerror.c:467: ((png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0)` | `error callback/longjmp: png_chunk_error(png_ptr, error_message);` | [ ] |
| 49 | `png_chunk_report` | `c_src/src/pngerror.c:496: (error < PNG_CHUNK_ERROR)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, message);` | [ ] |
| 50 | `png_chunk_report` | `c_src/src/pngerror.c:510: (error < PNG_CHUNK_WRITE_ERROR)` | `error callback/longjmp: png_app_error(png_ptr, message);` | [ ] |
| 51 | `png_fixed_error` | `c_src/src/pngerror.c:534: (name != NULL)` | `error callback/longjmp: png_error(png_ptr, msg);` | [ ] |
| 52 | `png_set_longjmp_fn` | `c_src/src/pngerror.c:558: (png_ptr == NULL)` | `sentinel return: return NULL;` | [ ] |
| 53 | `png_set_longjmp_fn` | `c_src/src/pngerror.c:573: (png_ptr->jmp_buf_ptr == NULL)` | `sentinel return: return NULL; /* new NULL return on OOM */` | [ ] |
| 54 | `png_set_longjmp_fn` | `c_src/src/pngerror.c:593: (png_ptr->jmp_buf_ptr != &png_ptr->jmp_buf_local)` | `error callback/longjmp: png_error(png_ptr, "Libpng jmp_buf still allocated");` | [ ] |
| 55 | `png_set_longjmp_fn` | `c_src/src/pngerror.c:601: (size != jmp_buf_size)` | `sentinel return: return NULL; /* caller will probably crash: no choice here */` | [ ] |
| 56 | `png_get_error_ptr` | `c_src/src/pngerror.c:742: (png_ptr == NULL)` | `sentinel return: return NULL;` | [ ] |
| 57 | `png_get_signature` | `c_src/src/pngget.c:496: (png_ptr != NULL && info_ptr != NULL)` | `sentinel return: return NULL;` | [ ] |
| 58 | `png_get_palette_max` | `c_src/src/pngget.c:1364: (png_ptr != NULL && info_ptr != NULL)` | `sentinel return: return -1;` | [ ] |
| 59 | `png_malloc_base` | `c_src/src/pngmem.c:83: (size > 65536U)` | `sentinel return: if (size > 65536U) return NULL;` | [ ] |
| 60 | `png_malloc_base` | `c_src/src/pngmem.c:88: (size > PNG_SIZE_MAX)` | `sentinel return: if (size > PNG_SIZE_MAX) return NULL;` | [ ] |
| 61 | `png_malloc_array_checked` | `c_src/src/pngmem.c:117: (req <= PNG_SIZE_MAX/element_size)` | `sentinel return: return NULL;` | [ ] |
| 62 | `png_malloc_array` | `c_src/src/pngmem.c:126: (nelements <= 0 \|\| element_size == 0)` | `error callback/longjmp: png_error(png_ptr, "internal error: array alloc");` | [ ] |
| 63 | `png_realloc_array` | `c_src/src/pngmem.c:139: (add_elements <= 0 \|\| element_size == 0 \|\| old_elements < 0 \|\| (old_array == NULL && old_elements > 0))` | `error callback/longjmp: png_error(png_ptr, "internal error: array realloc");` | [ ] |
| 64 | `png_realloc_array` | `c_src/src/pngmem.c:164: (old_elements > 0)` | `sentinel return: return NULL; /* error */` | [ ] |
| 65 | `png_malloc` | `c_src/src/pngmem.c:179: (png_ptr == NULL)` | `sentinel return: return NULL;` | [ ] |
| 66 | `png_malloc` | `c_src/src/pngmem.c:184: (ret == NULL)` | `error callback/longjmp: png_error(png_ptr, "Out of memory"); /* 'm' means png_malloc */` | [ ] |
| 67 | `png_malloc_default` | `c_src/src/pngmem.c:197: (png_ptr == NULL)` | `sentinel return: return NULL;` | [ ] |
| 68 | `png_malloc_default` | `c_src/src/pngmem.c:203: (ret == NULL)` | `error callback/longjmp: png_error(png_ptr, "Out of Memory"); /* 'M' means png_malloc_default */` | [ ] |
| 69 | `png_malloc_warn` | `c_src/src/pngmem.c:227: (ret != NULL)` | `sentinel return: return NULL;` | [ ] |
| 70 | `png_get_mem_ptr` | `c_src/src/pngmem.c:282: (png_ptr == NULL)` | `sentinel return: return NULL;` | [ ] |
| 71 | `png_push_read_sig` | `c_src/src/pngpread.c:166: (num_checked < 4 && png_sig_cmp(info_ptr->signature, num_checked, num_to_check - 4) != 0)` | `error callback/longjmp: png_error(png_ptr, "Not a PNG file");` | [ ] |
| 72 | `png_push_read_sig` | `c_src/src/pngpread.c:169: (num_checked < 4 && png_sig_cmp(info_ptr->signature, num_checked, num_to_check - 4) != 0)` | `error callback/longjmp: png_error(png_ptr, "PNG file corrupted by ASCII conversion");` | [ ] |
| 73 | `png_push_read_chunk` | `c_src/src/pngpread.c:213: ((png_ptr->mode & PNG_HAVE_IHDR) == 0)` | `error callback/longjmp: png_error(png_ptr, "Missing IHDR before IDAT");` | [ ] |
| 74 | `png_push_read_chunk` | `c_src/src/pngpread.c:217: (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && (png_ptr->mode & PNG_HAVE_PLTE) == 0)` | `error callback/longjmp: png_error(png_ptr, "Missing PLTE before IDAT");` | [ ] |
| 75 | `png_push_read_chunk` | `c_src/src/pngpread.c:229: ((png_ptr->mode & PNG_AFTER_IDAT) != 0)` | `error callback/longjmp: png_benign_error(png_ptr, "Too many IDATs found");` | [ ] |
| 76 | `png_push_read_chunk` | `c_src/src/pngpread.c:243: (png_ptr->push_length != 13)` | `error callback/longjmp: png_error(png_ptr, "Invalid IHDR length");` | [ ] |
| 77 | `png_push_save_buffer` | `c_src/src/pngpread.c:361: (png_ptr->save_buffer_size > PNG_SIZE_MAX - (png_ptr->current_buffer_size + 256))` | `error callback/longjmp: png_error(png_ptr, "Potential overflow of save_buffer");` | [ ] |
| 78 | `png_push_save_buffer` | `c_src/src/pngpread.c:372: (png_ptr->save_buffer == NULL)` | `error callback/longjmp: png_error(png_ptr, "Insufficient memory for save_buffer");` | [ ] |
| 79 | `png_push_save_buffer` | `c_src/src/pngpread.c:378: (png_ptr->save_buffer_size)` | `error callback/longjmp: png_error(png_ptr, "save_buffer error");` | [ ] |
| 80 | `png_push_read_IDAT` | `c_src/src/pngpread.c:425: ((png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED) == 0)` | `error callback/longjmp: png_error(png_ptr, "Not enough compressed data");` | [ ] |
| 81 | `png_process_IDAT_data` | `c_src/src/pngpread.c:502: (!(buffer_length > 0) \|\| buffer == NULL)` | `error callback/longjmp: png_error(png_ptr, "No IDAT data (internal error)");` | [ ] |
| 82 | `png_process_IDAT_data` | `c_src/src/pngpread.c:560: (ret == Z_DATA_ERROR)` | `error callback/longjmp: png_benign_error(png_ptr, "IDAT: ADLER32 checksum mismatch");` | [ ] |
| 83 | `png_process_IDAT_data` | `c_src/src/pngpread.c:562: (ret == Z_DATA_ERROR)` | `error callback/longjmp: png_error(png_ptr, "Decompression error in IDAT");` | [ ] |
| 84 | `png_push_process_row` | `c_src/src/pngpread.c:627: (png_ptr->row_buf[0] < PNG_FILTER_VALUE_LAST)` | `error callback/longjmp: png_error(png_ptr, "bad adaptive filter value");` | [ ] |
| 85 | `png_push_process_row` | `c_src/src/pngpread.c:647: (row_info.pixel_depth > png_ptr->maximum_pixel_depth)` | `error callback/longjmp: png_error(png_ptr, "progressive row overflow");` | [ ] |
| 86 | `png_push_process_row` | `c_src/src/pngpread.c:651: (png_ptr->transformed_pixel_depth != row_info.pixel_depth)` | `error callback/longjmp: png_error(png_ptr, "internal progressive row size calculation error");` | [ ] |
| 87 | `png_get_progressive_ptr` | `c_src/src/pngpread.c:941: (png_ptr == NULL)` | `sentinel return: return NULL;` | [ ] |
| 88 | `png_read_info` | `c_src/src/pngread.c:118: ((png_ptr->mode & PNG_HAVE_IHDR) == 0)` | `error callback/longjmp: png_chunk_error(png_ptr, "Missing IHDR before IDAT");` | [ ] |
| 89 | `png_read_info` | `c_src/src/pngread.c:122: (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && (png_ptr->mode & PNG_HAVE_PLTE) == 0)` | `error callback/longjmp: png_chunk_error(png_ptr, "Missing PLTE before IDAT");` | [ ] |
| 90 | `png_read_info` | `c_src/src/pngread.c:125: ((png_ptr->mode & PNG_AFTER_IDAT) != 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "Too many IDATs found");` | [ ] |
| 91 | `png_read_update_info` | `c_src/src/pngread.c:191: unconditional rejection/state failure` | `error callback/longjmp: png_app_error(png_ptr, "png_read_update_info/png_start_read_image: duplicate call");` | [ ] |
| 92 | `png_start_read_image` | `c_src/src/pngread.c:214: ((png_ptr->flags & PNG_FLAG_ROW_INIT) == 0)` | `error callback/longjmp: png_app_error(png_ptr, "png_start_read_image/png_read_update_info: duplicate call");` | [ ] |
| 93 | `png_read_row` | `c_src/src/pngread.c:444: ((png_ptr->mode & PNG_HAVE_IDAT) == 0)` | `error callback/longjmp: png_error(png_ptr, "Invalid attempt to read row data");` | [ ] |
| 94 | `png_read_row` | `c_src/src/pngread.c:456: (png_ptr->row_buf[0] < PNG_FILTER_VALUE_LAST)` | `error callback/longjmp: png_error(png_ptr, "bad adaptive filter value");` | [ ] |
| 95 | `png_read_row` | `c_src/src/pngread.c:489: (row_info.pixel_depth > png_ptr->maximum_pixel_depth)` | `error callback/longjmp: png_error(png_ptr, "sequential row overflow");` | [ ] |
| 96 | `png_read_row` | `c_src/src/pngread.c:493: (png_ptr->transformed_pixel_depth != row_info.pixel_depth)` | `error callback/longjmp: png_error(png_ptr, "internal sequential row size calculation error");` | [ ] |
| 97 | `png_read_image` | `c_src/src/pngread.c:648: (png_ptr->interlaced)` | `error callback/longjmp: png_error(png_ptr, "Cannot read interlaced image -- interlace handler disabled");` | [ ] |
| 98 | `png_read_end` | `c_src/src/pngread.c:697: (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && png_ptr->num_palette_max >= png_ptr->num_palette)` | `error callback/longjmp: png_benign_error(png_ptr, "Read palette index exceeding num_palette");` | [ ] |
| 99 | `png_read_end` | `c_src/src/pngread.c:729: ((length > 0 && !(png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED)) \|\| (png_ptr->mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0)` | `error callback/longjmp: png_benign_error(png_ptr, ".Too many IDATs found");` | [ ] |
| 100 | `png_read_end` | `c_src/src/pngread.c:747: ((length > 0 && !(png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED)) \|\| (png_ptr->mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0)` | `error callback/longjmp: png_benign_error(png_ptr, "..Too many IDATs found");` | [ ] |
| 101 | `png_read_png` | `c_src/src/pngread.c:881: (info_ptr->height > PNG_UINT_32_MAX/(sizeof (png_bytep)))` | `error callback/longjmp: png_error(png_ptr, "Image is too high to process with png_read_png()");` | [ ] |
| 102 | `png_read_png` | `c_src/src/pngread.c:899: ((transforms & PNG_TRANSFORM_SCALE_16) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_SCALE_16 not supported");` | [ ] |
| 103 | `png_read_png` | `c_src/src/pngread.c:910: ((transforms & PNG_TRANSFORM_STRIP_16) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_STRIP_16 not supported");` | [ ] |
| 104 | `png_read_png` | `c_src/src/pngread.c:920: ((transforms & PNG_TRANSFORM_STRIP_ALPHA) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_STRIP_ALPHA not supported");` | [ ] |
| 105 | `png_read_png` | `c_src/src/pngread.c:930: ((transforms & PNG_TRANSFORM_PACKING) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_PACKING not supported");` | [ ] |
| 106 | `png_read_png` | `c_src/src/pngread.c:940: ((transforms & PNG_TRANSFORM_PACKSWAP) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_PACKSWAP not supported");` | [ ] |
| 107 | `png_read_png` | `c_src/src/pngread.c:952: ((transforms & PNG_TRANSFORM_EXPAND) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_EXPAND not supported");` | [ ] |
| 108 | `png_read_png` | `c_src/src/pngread.c:964: ((transforms & PNG_TRANSFORM_INVERT_MONO) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_INVERT_MONO not supported");` | [ ] |
| 109 | `png_read_png` | `c_src/src/pngread.c:976: ((info_ptr->valid & PNG_INFO_sBIT) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_SHIFT not supported");` | [ ] |
| 110 | `png_read_png` | `c_src/src/pngread.c:984: ((transforms & PNG_TRANSFORM_BGR) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_BGR not supported");` | [ ] |
| 111 | `png_read_png` | `c_src/src/pngread.c:992: ((transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_SWAP_ALPHA not supported");` | [ ] |
| 112 | `png_read_png` | `c_src/src/pngread.c:1000: ((transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_SWAP_ENDIAN not supported");` | [ ] |
| 113 | `png_read_png` | `c_src/src/pngread.c:1009: ((transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_INVERT_ALPHA not supported");` | [ ] |
| 114 | `png_read_png` | `c_src/src/pngread.c:1018: ((transforms & PNG_TRANSFORM_GRAY_TO_RGB) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_GRAY_TO_RGB not supported");` | [ ] |
| 115 | `png_read_png` | `c_src/src/pngread.c:1026: ((transforms & PNG_TRANSFORM_EXPAND_16) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_EXPAND_16 not supported");` | [ ] |
| 116 | `png_image_read_init` | `c_src/src/pngread.c:1169: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_read: out of memory");` | [ ] |
| 117 | `png_image_read_init` | `c_src/src/pngread.c:1172: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_read: opaque pointer not NULL");` | [ ] |
| 118 | `png_image_begin_read_from_stdio` | `c_src/src/pngread.c:1355: (png_image_read_init(image) != 0)` | `error callback/longjmp: return png_image_error(image, "png_image_begin_read_from_stdio: invalid argument");` | [ ] |
| 119 | `png_image_begin_read_from_stdio` | `c_src/src/pngread.c:1360: (image != NULL)` | `error callback/longjmp: return png_image_error(image, "png_image_begin_read_from_stdio: incorrect PNG_IMAGE_VERSION");` | [ ] |
| 120 | `png_image_begin_read_from_file` | `c_src/src/pngread.c:1389: (png_image_read_init(image) != 0)` | `error callback/longjmp: return png_image_error(image, strerror(errno));` | [ ] |
| 121 | `png_image_begin_read_from_file` | `c_src/src/pngread.c:1393: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_begin_read_from_file: invalid argument");` | [ ] |
| 122 | `png_image_begin_read_from_file` | `c_src/src/pngread.c:1398: (image != NULL)` | `error callback/longjmp: return png_image_error(image, "png_image_begin_read_from_file: incorrect PNG_IMAGE_VERSION");` | [ ] |
| 123 | `png_image_memory_read` | `c_src/src/pngread.c:1427: (memory != NULL && size >= need)` | `error callback/longjmp: png_error(png_ptr, "read beyond end of data");` | [ ] |
| 124 | `png_image_memory_read` | `c_src/src/pngread.c:1431: (memory != NULL && size >= need)` | `error callback/longjmp: png_error(png_ptr, "invalid memory read");` | [ ] |
| 125 | `png_image_begin_read_from_memory` | `c_src/src/pngread.c:1458: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_begin_read_from_memory: invalid argument");` | [x] |
| 126 | `png_image_begin_read_from_memory` | `c_src/src/pngread.c:1463: (image != NULL)` | `error callback/longjmp: return png_image_error(image, "png_image_begin_read_from_memory: incorrect PNG_IMAGE_VERSION");` | [x] |
| 127 | `set_file_encoding` | `c_src/src/pngread.c:1537: (g == 0)` | `error callback/longjmp: png_error(png_ptr, "internal: default gamma not set");` | [ ] |
| 128 | `decode_gamma` | `c_src/src/pngread.c:1586: unconditional rejection/state failure` | `error callback/longjmp: png_error(display->image->opaque->png_ptr, "unexpected encoding (internal error)");` | [ ] |
| 129 | `png_create_colormap_entry` | `c_src/src/pngread.c:1643: (ip > 255)` | `error callback/longjmp: png_error(image->opaque->png_ptr, "color-map index out of range");` | [ ] |
| 130 | `png_create_colormap_entry` | `c_src/src/pngread.c:1743: (encoding != output_encoding)` | `error callback/longjmp: png_error(image->opaque->png_ptr, "bad encoding (internal error)");` | [ ] |
| 131 | `png_image_read_colormap` | `c_src/src/pngread.c:1997: (display->background == NULL )` | `error callback/longjmp: png_error(png_ptr, "background color must be supplied to remove alpha/transparency");` | [ ] |
| 132 | `png_image_read_colormap` | `c_src/src/pngread.c:2056: (cmap_entries > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "gray[8] color-map: too few entries");` | [ ] |
| 133 | `png_image_read_colormap` | `c_src/src/pngread.c:2135: (PNG_GRAY_COLORMAP_ENTRIES > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "gray[16] color-map: too few entries");` | [ ] |
| 134 | `png_image_read_colormap` | `c_src/src/pngread.c:2233: (PNG_GA_COLORMAP_ENTRIES > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "gray+alpha color-map: too few entries");` | [ ] |
| 135 | `png_image_read_colormap` | `c_src/src/pngread.c:2267: (PNG_GRAY_COLORMAP_ENTRIES > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "gray-alpha color-map: too few entries");` | [ ] |
| 136 | `png_image_read_colormap` | `c_src/src/pngread.c:2301: (PNG_GA_COLORMAP_ENTRIES > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "ga-alpha color-map: too few entries");` | [ ] |
| 137 | `png_image_read_colormap` | `c_src/src/pngread.c:2406: (PNG_GA_COLORMAP_ENTRIES > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "rgb[ga] color-map: too few entries");` | [ ] |
| 138 | `png_image_read_colormap` | `c_src/src/pngread.c:2422: (PNG_GRAY_COLORMAP_ENTRIES > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "rgb[gray] color-map: too few entries");` | [ ] |
| 139 | `png_image_read_colormap` | `c_src/src/pngread.c:2530: (PNG_RGB_COLORMAP_ENTRIES+1+27 > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "rgb+alpha color-map: too few entries");` | [ ] |
| 140 | `png_image_read_colormap` | `c_src/src/pngread.c:2579: (PNG_RGB_COLORMAP_ENTRIES+1+27 > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "rgb-alpha color-map: too few entries");` | [ ] |
| 141 | `png_image_read_colormap` | `c_src/src/pngread.c:2664: (PNG_RGB_COLORMAP_ENTRIES > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "rgb color-map: too few entries");` | [ ] |
| 142 | `png_image_read_colormap` | `c_src/src/pngread.c:2695: (cmap_entries > (unsigned int)image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "palette color-map: too few entries");` | [ ] |
| 143 | `png_image_read_colormap` | `c_src/src/pngread.c:2738: (png_ptr->bit_depth < 8)` | `error callback/longjmp: png_error(png_ptr, "invalid PNG color type");` | [ ] |
| 144 | `png_image_read_colormap` | `c_src/src/pngread.c:2761: (png_ptr->bit_depth > 8)` | `error callback/longjmp: png_error(png_ptr, "bad data option (internal error)");` | [ ] |
| 145 | `png_image_read_colormap` | `c_src/src/pngread.c:2766: (cmap_entries > 256 \|\| cmap_entries > image->colormap_entries)` | `error callback/longjmp: png_error(png_ptr, "color map overflow (BAD internal error)");` | [ ] |
| 146 | `png_image_read_colormap` | `c_src/src/pngread.c:2800: (background_index != PNG_CMAP_RGB_ALPHA_BACKGROUND)` | `error callback/longjmp: png_error(png_ptr, "bad processing option (internal error)");` | [ ] |
| 147 | `png_image_read_colormap` | `c_src/src/pngread.c:2803: (background_index != PNG_CMAP_RGB_ALPHA_BACKGROUND)` | `error callback/longjmp: png_error(png_ptr, "bad background index (internal error)");` | [ ] |
| 148 | `png_image_read_and_map` | `c_src/src/pngread.c:2836: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "unknown interlace type");` | [ ] |
| 149 | `png_image_read_colormapped` | `c_src/src/pngread.c:3074: (info_ptr->color_type == PNG_COLOR_TYPE_RGB_ALPHA && info_ptr->bit_depth == 8 && png_ptr->screen_gamma == PNG_GAMMA_sRGB && image->colormap_entries == 244 )` | `error callback/longjmp: png_error(png_ptr, "bad color-map processing (internal error)");` | [ ] |
| 150 | `png_image_read_direct_scaled` | `c_src/src/pngread.c:3159: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "unknown interlace type");` | [ ] |
| 151 | `png_image_read_composite` | `c_src/src/pngread.c:3208: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "unknown interlace type");` | [ ] |
| 152 | `png_image_read_background` | `c_src/src/pngread.c:3358: ((png_ptr->transformations & PNG_RGB_TO_GRAY) == 0)` | `error callback/longjmp: png_error(png_ptr, "lost rgb to gray");` | [ ] |
| 153 | `png_image_read_background` | `c_src/src/pngread.c:3361: ((png_ptr->transformations & PNG_COMPOSE) != 0)` | `error callback/longjmp: png_error(png_ptr, "unexpected compose");` | [ ] |
| 154 | `png_image_read_background` | `c_src/src/pngread.c:3364: (png_get_channels(png_ptr, info_ptr) != 2)` | `error callback/longjmp: png_error(png_ptr, "lost/gained channels");` | [ ] |
| 155 | `png_image_read_background` | `c_src/src/pngread.c:3369: ((image->format & PNG_FORMAT_FLAG_LINEAR) == 0 && (image->format & PNG_FORMAT_FLAG_ALPHA) != 0)` | `error callback/longjmp: png_error(png_ptr, "unexpected 8-bit transformation");` | [ ] |
| 156 | `png_image_read_background` | `c_src/src/pngread.c:3382: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "unknown interlace type");` | [ ] |
| 157 | `png_image_read_background` | `c_src/src/pngread.c:3609: (preserve_alpha != 0)` | `error callback/longjmp: png_error(png_ptr, "unexpected bit depth");` | [ ] |
| 158 | `png_image_read_direct` | `c_src/src/pngread.c:3925: (change != 0)` | `error callback/longjmp: png_error(png_ptr, "png_read_image: unsupported transformation");` | [ ] |
| 159 | `png_image_read_direct` | `c_src/src/pngread.c:3960: (do_local_compose != 0)` | `error callback/longjmp: png_error(png_ptr, "png_image_read: alpha channel lost");` | [ ] |
| 160 | `png_image_read_direct` | `c_src/src/pngread.c:3986: (do_local_background == 2)` | `error callback/longjmp: png_error(png_ptr, "unexpected alpha swap transformation");` | [ ] |
| 161 | `png_image_read_direct` | `c_src/src/pngread.c:3994: (info_format != format)` | `error callback/longjmp: png_error(png_ptr, "png_read_image: invalid transformations");` | [ ] |
| 162 | `png_image_finish_read` | `c_src/src/pngread.c:4178: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_finish_read[color-map]: no color-map");` | [x] |
| 163 | `png_image_finish_read` | `c_src/src/pngread.c:4183: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_finish_read: image too large");` | [x] |
| 164 | `png_image_finish_read` | `c_src/src/pngread.c:4188: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_finish_read: invalid argument");` | [x] |
| 165 | `png_image_finish_read` | `c_src/src/pngread.c:4193: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_finish_read: row_stride too large");` | [x] |
| 166 | `png_image_finish_read` | `c_src/src/pngread.c:4198: (image != NULL)` | `error callback/longjmp: return png_image_error(image, "png_image_finish_read: damaged PNG_IMAGE_VERSION");` | [x] |
| 167 | `png_read_data` | `c_src/src/pngrio.c:39: (png_ptr->read_data_fn != NULL)` | `error callback/longjmp: png_error(png_ptr, "Call to NULL read function");` | [ ] |
| 168 | `png_default_read_data` | `c_src/src/pngrio.c:62: (check != length)` | `error callback/longjmp: png_error(png_ptr, "Read Error");` | [ ] |
| 169 | `png_rtran_ok` | `c_src/src/pngrtran.c:120: ((png_ptr->flags & PNG_FLAG_ROW_INIT) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "invalid after png_start_read_image or png_read_update_info");` | [ ] |
| 170 | `png_rtran_ok` | `c_src/src/pngrtran.c:124: (need_IHDR && (png_ptr->mode & PNG_HAVE_IHDR) == 0)` | `error callback/longjmp: png_app_error(png_ptr, "invalid before the PNG header has been read");` | [ ] |
| 171 | `convert_gamma_value` | `c_src/src/pngrtran.c:325: (output_gamma > PNG_FP_MAX \|\| output_gamma < PNG_FP_MIN)` | `error callback/longjmp: png_fixed_error(png_ptr, "gamma value");` | [ ] |
| 172 | `unsupported_gamma` | `c_src/src/pngrtran.c:350: (warn)` | `error callback/longjmp: png_app_error(png_ptr, msg);` | [ ] |
| 173 | `png_set_alpha_mode_fixed` | `c_src/src/pngrtran.c:434: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "invalid alpha mode");` | [ ] |
| 174 | `png_set_alpha_mode_fixed` | `c_src/src/pngrtran.c:452: ((png_ptr->transformations & PNG_COMPOSE) != 0)` | `error callback/longjmp: png_error(png_ptr, "conflicting calls to set alpha mode and background");` | [ ] |
| 175 | `png_set_gamma_fixed` | `c_src/src/pngrtran.c:917: (file_gamma <= 0)` | `error callback/longjmp: png_app_error(png_ptr, "invalid file gamma in png_set_gamma");` | [ ] |
| 176 | `png_set_gamma_fixed` | `c_src/src/pngrtran.c:919: (scrn_gamma <= 0)` | `error callback/longjmp: png_app_error(png_ptr, "invalid screen gamma in png_set_gamma");` | [ ] |
| 177 | `png_set_rgb_to_gray_fixed` | `c_src/src/pngrtran.c:1072: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "invalid error action to rgb_to_gray");` | [ ] |
| 178 | `png_set_rgb_to_gray_fixed` | `c_src/src/pngrtran.c:1083: (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE)` | `error callback/longjmp: png_error(png_ptr, "Cannot do RGB_TO_GRAY without EXPAND_SUPPORTED");` | [ ] |
| 179 | `png_init_read_transformations` | `c_src/src/pngrtran.c:1886: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "invalid background gamma type");` | [ ] |
| 180 | `png_read_transform_info` | `c_src/src/pngrtran.c:2104: (png_ptr->palette == NULL)` | `error callback/longjmp: png_error (png_ptr, "Palette is NULL in indexed image");` | [ ] |
| 181 | `png_do_read_transformations` | `c_src/src/pngrtran.c:4891: (png_ptr->row_buf == NULL)` | `error callback/longjmp: png_error(png_ptr, "NULL row buffer");` | [ ] |
| 182 | `png_do_read_transformations` | `c_src/src/pngrtran.c:4907: ((png_ptr->flags & PNG_FLAG_DETECT_UNINITIALIZED) != 0 && (png_ptr->flags & PNG_FLAG_ROW_INIT) == 0)` | `error callback/longjmp: png_error(png_ptr, "Uninitialized row");` | [ ] |
| 183 | `png_do_read_transformations` | `c_src/src/pngrtran.c:4969: ((png_ptr->transformations & PNG_RGB_TO_GRAY) == PNG_RGB_TO_GRAY_ERR)` | `error callback/longjmp: png_error(png_ptr, "png_do_rgb_to_gray found nongray pixel");` | [ ] |
| 184 | `png_get_uint_31` | `c_src/src/pngrutil.c:46: (uval > PNG_UINT_31_MAX)` | `error callback/longjmp: png_error(png_ptr, "PNG unsigned integer out of range");` | [ ] |
| 185 | `png_read_sig` | `c_src/src/pngrutil.c:139: (num_checked < 4 && png_sig_cmp(info_ptr->signature, num_checked, num_to_check - 4) != 0)` | `error callback/longjmp: png_error(png_ptr, "Not a PNG file");` | [ ] |
| 186 | `png_read_sig` | `c_src/src/pngrutil.c:141: (num_checked < 4 && png_sig_cmp(info_ptr->signature, num_checked, num_to_check - 4) != 0)` | `error callback/longjmp: png_error(png_ptr, "PNG file corrupted by ASCII conversion");` | [ ] |
| 187 | `png_read_chunk_header` | `c_src/src/pngrutil.c:211: (buf[0] >= 0x80U)` | `error callback/longjmp: png_chunk_error(png_ptr, "bad header (invalid length)");` | [ ] |
| 188 | `png_read_chunk_header` | `c_src/src/pngrutil.c:215: (!check_chunk_name(chunk_name))` | `error callback/longjmp: png_chunk_error(png_ptr, "bad header (invalid type)");` | [ ] |
| 189 | `png_crc_finish_critical` | `c_src/src/pngrutil.c:342: (png_crc_error(png_ptr, handle_as_ancillary) != 0)` | `error callback/longjmp: if (png_crc_error(png_ptr, handle_as_ancillary) != 0) { /* See above for the explanation of how the flags work. */ if (handle_as_ancillary \|\| PNG_CHUNK_ANCILLARY(png_ptr->chunk_name) != 0 ? (png_ptr->flags & PNG_FLAG_CRC_ANCILLARY_NOWARN) == 0 : (png_ptr->flags & PNG_FLAG_CRC_CRITICAL_USE) != 0) png_chunk_warning(png_ptr, "CRC error");` | [ ] |
| 190 | `png_crc_finish_critical` | `c_src/src/pngrutil.c:351: (handle_as_ancillary \|\| PNG_CHUNK_ANCILLARY(png_ptr->chunk_name) != 0 ? (png_ptr->flags & PNG_FLAG_CRC_ANCILLARY_NOWARN) == 0 : (png_ptr->flags & PNG_FLAG_CRC_CRITICAL_USE) != 0)` | `error callback/longjmp: png_chunk_error(png_ptr, "CRC error");` | [ ] |
| 191 | `png_read_buffer` | `c_src/src/pngrutil.c:380: (new_size > png_chunk_max(png_ptr))` | `sentinel return: if (new_size > png_chunk_max(png_ptr)) return NULL;` | [ ] |
| 192 | `png_inflate_claim` | `c_src/src/pngrutil.c:430: unconditional rejection/state failure` | `error callback/longjmp: png_chunk_error(png_ptr, msg);` | [ ] |
| 193 | `png_inflate_claim` | `c_src/src/pngrutil.c:507: (ret == Z_OK)` | `error callback/longjmp: png_zstream_error(png_ptr, ret);` | [ ] |
| 194 | `png_zlib_inflate` | `c_src/src/pngrutil.c:532: ((*png_ptr->zstream.next_in >> 4) > 7)` | `sentinel return: return Z_DATA_ERROR;` | [ ] |
| 195 | `png_inflate` | `c_src/src/pngrutil.c:658: (avail_in > 0)` | `error callback/longjmp: png_zstream_error(png_ptr, ret);` | [ ] |
| 196 | `png_inflate` | `c_src/src/pngrutil.c:669: unconditional rejection/state failure` | `sentinel return: return Z_STREAM_ERROR;` | [ ] |
| 197 | `png_decompress_chunk` | `c_src/src/pngrutil.c:789: (ret == Z_STREAM_END && chunklength - prefix_size != lzsize)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "extra compressed data");` | [ ] |
| 198 | `png_decompress_chunk` | `c_src/src/pngrutil.c:796: (ret == Z_STREAM_END && chunklength - prefix_size != lzsize)` | `error callback/longjmp: png_zstream_error(png_ptr, Z_MEM_ERROR);` | [ ] |
| 199 | `png_decompress_chunk` | `c_src/src/pngrutil.c:803: unconditional rejection/state failure` | `error callback/longjmp: png_zstream_error(png_ptr, ret);` | [ ] |
| 200 | `png_decompress_chunk` | `c_src/src/pngrutil.c:824: (ret == Z_STREAM_END)` | `error callback/longjmp: png_zstream_error(png_ptr, Z_MEM_ERROR);` | [ ] |
| 201 | `png_decompress_chunk` | `c_src/src/pngrutil.c:825: (ret == Z_STREAM_END)` | `sentinel return: return Z_MEM_ERROR;` | [ ] |
| 202 | `png_inflate_read` | `c_src/src/pngrutil.c:886: unconditional rejection/state failure` | `error callback/longjmp: png_zstream_error(png_ptr, ret);` | [ ] |
| 203 | `png_inflate_read` | `c_src/src/pngrutil.c:893: unconditional rejection/state failure` | `sentinel return: return Z_STREAM_ERROR;` | [ ] |
| 204 | `png_handle_PLTE` | `c_src/src/pngrutil.c:1064: (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE)` | `error callback/longjmp: png_chunk_error(png_ptr, errmsg);` | [ ] |
| 205 | `png_handle_PLTE` | `c_src/src/pngrutil.c:1070: (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, errmsg);` | [ ] |
| 206 | `png_handle_IEND` | `c_src/src/pngrutil.c:1092: (length != 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 207 | `png_handle_gAMA` | `c_src/src/pngrutil.c:1118: (ugamma > PNG_UINT_31_MAX)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 208 | `png_handle_sBIT` | `c_src/src/pngrutil.c:1164: (length != truelen)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "bad length");` | [ ] |
| 209 | `png_handle_sBIT` | `c_src/src/pngrutil.c:1178: (buf[i] == 0 \|\| buf[i] > sample_depth)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 210 | `png_handle_cHRM` | `c_src/src/pngrutil.c:1251: (error)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 211 | `png_handle_sRGB` | `c_src/src/pngrutil.c:1300: (intent > 3)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 212 | `png_handle_iCCP` | `c_src/src/pngrutil.c:1356: (length < LZ77Min)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "too short");` | [ ] |
| 213 | `png_handle_iCCP` | `c_src/src/pngrutil.c:1541: (errmsg != NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, errmsg);` | [ ] |
| 214 | `png_handle_sPLT` | `c_src/src/pngrutil.c:1588: (buffer == NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of memory");` | [ ] |
| 215 | `png_handle_tRNS` | `c_src/src/pngrutil.c:1700: (length != 2)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 216 | `png_handle_tRNS` | `c_src/src/pngrutil.c:1716: (length != 6)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 217 | `png_handle_tRNS` | `c_src/src/pngrutil.c:1732: ((png_ptr->mode & PNG_HAVE_PLTE) == 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of place");` | [ ] |
| 218 | `png_handle_tRNS` | `c_src/src/pngrutil.c:1741: (length > (unsigned int) png_ptr->num_palette \|\| length > (unsigned int) PNG_MAX_PALETTE_LENGTH \|\| length == 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 219 | `png_handle_tRNS` | `c_src/src/pngrutil.c:1752: unconditional rejection/state failure` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid with alpha channel");` | [ ] |
| 220 | `png_handle_bKGD` | `c_src/src/pngrutil.c:1785: ((png_ptr->mode & PNG_HAVE_PLTE) == 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of place");` | [ ] |
| 221 | `png_handle_bKGD` | `c_src/src/pngrutil.c:1801: (length != truelen)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 222 | `png_handle_bKGD` | `c_src/src/pngrutil.c:1823: (buf[0] >= info_ptr->num_palette)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid index");` | [ ] |
| 223 | `png_handle_bKGD` | `c_src/src/pngrutil.c:1844: (buf[0] != 0 \|\| buf[1] >= (unsigned int)(1 << png_ptr->bit_depth))` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid gray level");` | [ ] |
| 224 | `png_handle_bKGD` | `c_src/src/pngrutil.c:1862: (buf[0] != 0 \|\| buf[2] != 0 \|\| buf[4] != 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid color");` | [ ] |
| 225 | `png_handle_eXIf` | `c_src/src/pngrutil.c:2010: (buffer == NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of memory");` | [ ] |
| 226 | `png_handle_eXIf` | `c_src/src/pngrutil.c:2029: (header != 0x49492A00 && header != 0x4D4D002A)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 227 | `png_handle_hIST` | `c_src/src/pngrutil.c:2063: (length != num * 2 \|\| num != (unsigned int)png_ptr->num_palette \|\| num > (unsigned int)PNG_MAX_PALETTE_LENGTH)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 228 | `png_handle_pCAL` | `c_src/src/pngrutil.c:2161: (buffer == NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of memory");` | [ ] |
| 229 | `png_handle_pCAL` | `c_src/src/pngrutil.c:2183: (endptr - buf <= 12)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid");` | [ ] |
| 230 | `png_handle_pCAL` | `c_src/src/pngrutil.c:2203: ((type == PNG_EQUATION_LINEAR && nparams != 2) \|\| (type == PNG_EQUATION_BASE_E && nparams != 3) \|\| (type == PNG_EQUATION_ARBITRARY && nparams != 3) \|\| (type == PNG_EQUATION_HYPERBOLIC && nparams != 4))` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid parameter count");` | [ ] |
| 231 | `png_handle_pCAL` | `c_src/src/pngrutil.c:2209: (type >= PNG_EQUATION_LAST)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "unrecognized equation type");` | [ ] |
| 232 | `png_handle_pCAL` | `c_src/src/pngrutil.c:2222: (params == NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of memory");` | [ ] |
| 233 | `png_handle_pCAL` | `c_src/src/pngrutil.c:2240: (buf > endptr)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid data");` | [ ] |
| 234 | `png_handle_sCAL` | `c_src/src/pngrutil.c:2279: (buffer == NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of memory");` | [ ] |
| 235 | `png_handle_sCAL` | `c_src/src/pngrutil.c:2292: (buffer[0] != 1 && buffer[0] != 2)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "invalid unit");` | [ ] |
| 236 | `png_handle_sCAL` | `c_src/src/pngrutil.c:2304: (png_check_fp_number((png_const_charp)buffer, length, &state, &i) == 0 \|\| i >= length \|\| buffer[i++] != 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "bad width format");` | [ ] |
| 237 | `png_handle_sCAL` | `c_src/src/pngrutil.c:2307: (PNG_FP_IS_POSITIVE(state) == 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "non-positive width");` | [ ] |
| 238 | `png_handle_sCAL` | `c_src/src/pngrutil.c:2316: (png_check_fp_number((png_const_charp)buffer, length, &state, &i) == 0 \|\| i != length)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "bad height format");` | [ ] |
| 239 | `png_handle_sCAL` | `c_src/src/pngrutil.c:2319: (PNG_FP_IS_POSITIVE(state) == 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "non-positive height");` | [ ] |
| 240 | `png_handle_tEXt` | `c_src/src/pngrutil.c:2397: (--png_ptr->user_chunk_cache_max == 1)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "no space in chunk cache");` | [ ] |
| 241 | `png_handle_tEXt` | `c_src/src/pngrutil.c:2408: (buffer == NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of memory");` | [ ] |
| 242 | `png_handle_tEXt` | `c_src/src/pngrutil.c:2437: (png_set_text_2(png_ptr, info_ptr, &text_info, 1) == 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of memory");` | [ ] |
| 243 | `png_handle_zTXt` | `c_src/src/pngrutil.c:2467: (--png_ptr->user_chunk_cache_max == 1)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "no space in chunk cache");` | [ ] |
| 244 | `png_handle_zTXt` | `c_src/src/pngrutil.c:2482: (buffer == NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of memory");` | [ ] |
| 245 | `png_handle_zTXt` | `c_src/src/pngrutil.c:2553: (png_set_text_2(png_ptr, info_ptr, &text, 1) == 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, errmsg);` | [ ] |
| 246 | `png_handle_iTXt` | `c_src/src/pngrutil.c:2583: (--png_ptr->user_chunk_cache_max == 1)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "no space in chunk cache");` | [ ] |
| 247 | `png_handle_iTXt` | `c_src/src/pngrutil.c:2594: (buffer == NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "out of memory");` | [ ] |
| 248 | `png_handle_iTXt` | `c_src/src/pngrutil.c:2702: (errmsg != NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, errmsg);` | [ ] |
| 249 | `png_cache_unknown_chunk` | `c_src/src/pngrutil.c:2745: (png_ptr->unknown_chunk.data == NULL && length > 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "unknown chunk exceeds memory limits");` | [ ] |
| 250 | `png_handle_unknown` | `c_src/src/pngrutil.c:2812: (ret < 0)` | `error callback/longjmp: png_chunk_error(png_ptr, "error in user chunk");` | [ ] |
| 251 | `png_handle_unknown` | `c_src/src/pngrutil.c:2893: (keep > PNG_HANDLE_CHUNK_NEVER)` | `error callback/longjmp: png_app_error(png_ptr, "no unknown chunk support available");` | [ ] |
| 252 | `png_handle_unknown` | `c_src/src/pngrutil.c:2912: (keep == PNG_HANDLE_CHUNK_ALWAYS \|\| (keep == PNG_HANDLE_CHUNK_IF_SAFE && PNG_CHUNK_ANCILLARY(png_ptr->chunk_name)))` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "no space in chunk cache");` | [ ] |
| 253 | `png_handle_unknown` | `c_src/src/pngrutil.c:2957: (handled < handled_saved && PNG_CHUNK_CRITICAL(png_ptr->chunk_name))` | `error callback/longjmp: png_chunk_error(png_ptr, "unhandled critical chunk");` | [ ] |
| 254 | `png_handle_chunk` | `c_src/src/pngrutil.c:3135: (chunk_index != PNG_INDEX_IHDR && (png_ptr->mode & PNG_HAVE_IHDR) == 0)` | `error callback/longjmp: png_chunk_error(png_ptr, "missing IHDR"); /* NORETURN */` | [ ] |
| 255 | `png_handle_chunk` | `c_src/src/pngrutil.c:3201: (PNG_CHUNK_CRITICAL(chunk_name))` | `error callback/longjmp: png_chunk_error(png_ptr, errmsg);` | [ ] |
| 256 | `png_handle_chunk` | `c_src/src/pngrutil.c:3206: (PNG_CHUNK_CRITICAL(chunk_name))` | `error callback/longjmp: png_chunk_benign_error(png_ptr, errmsg);` | [ ] |
| 257 | `png_combine_row` | `c_src/src/pngrutil.c:3243: (pixel_depth == 0)` | `error callback/longjmp: png_error(png_ptr, "internal row logic error");` | [ ] |
| 258 | `png_combine_row` | `c_src/src/pngrutil.c:3251: (png_ptr->info_rowbytes != 0 && png_ptr->info_rowbytes != PNG_ROWBYTES(pixel_depth, row_width))` | `error callback/longjmp: png_error(png_ptr, "internal row size calculation error");` | [ ] |
| 259 | `png_combine_row` | `c_src/src/pngrutil.c:3255: (row_width == 0)` | `error callback/longjmp: png_error(png_ptr, "internal row width error");` | [ ] |
| 260 | `png_combine_row` | `c_src/src/pngrutil.c:3478: (pixel_depth & 7)` | `error callback/longjmp: png_error(png_ptr, "invalid user transform pixel depth");` | [ ] |
| 261 | `png_read_IDAT_data` | `c_src/src/pngrutil.c:4201: (png_ptr->chunk_name != png_IDAT)` | `error callback/longjmp: png_error(png_ptr, "Not enough image data");` | [ ] |
| 262 | `png_read_IDAT_data` | `c_src/src/pngrutil.c:4222: (buffer == NULL)` | `error callback/longjmp: png_chunk_error(png_ptr, "out of memory");` | [ ] |
| 263 | `png_read_IDAT_data` | `c_src/src/pngrutil.c:4276: (png_ptr->zstream.avail_in > 0 \|\| png_ptr->idat_size > 0)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "Extra compressed data");` | [ ] |
| 264 | `png_read_IDAT_data` | `c_src/src/pngrutil.c:4282: (ret != Z_OK)` | `error callback/longjmp: png_zstream_error(png_ptr, ret);` | [ ] |
| 265 | `png_read_IDAT_data` | `c_src/src/pngrutil.c:4285: (output != NULL)` | `error callback/longjmp: png_chunk_error(png_ptr, png_ptr->zstream.msg);` | [ ] |
| 266 | `png_read_IDAT_data` | `c_src/src/pngrutil.c:4289: (output != NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, png_ptr->zstream.msg);` | [ ] |
| 267 | `png_read_IDAT_data` | `c_src/src/pngrutil.c:4301: (output != NULL)` | `error callback/longjmp: png_error(png_ptr, "Not enough image data");` | [ ] |
| 268 | `png_read_IDAT_data` | `c_src/src/pngrutil.c:4304: (output != NULL)` | `error callback/longjmp: png_chunk_benign_error(png_ptr, "Too much image data");` | [ ] |
| 269 | `png_read_start_row` | `c_src/src/pngrutil.c:4600: (row_bytes > (png_uint_32)65536L)` | `error callback/longjmp: png_error(png_ptr, "This image requires a row greater than 64KB");` | [ ] |
| 270 | `png_read_start_row` | `c_src/src/pngrutil.c:4645: (png_ptr->rowbytes > 65535)` | `error callback/longjmp: png_error(png_ptr, "This image requires a row greater than 64KB");` | [ ] |
| 271 | `png_read_start_row` | `c_src/src/pngrutil.c:4649: (png_ptr->rowbytes > (PNG_SIZE_MAX - 1))` | `error callback/longjmp: png_error(png_ptr, "Row has too many bytes to allocate in memory");` | [ ] |
| 272 | `png_read_start_row` | `c_src/src/pngrutil.c:4680: (png_inflate_claim(png_ptr, png_IDAT) != Z_OK)` | `error callback/longjmp: png_error(png_ptr, png_ptr->zstream.msg);` | [ ] |
| 273 | `png_set_cHRM_XYZ_fixed` | `c_src/src/pngset.c:94: (png_xy_from_XYZ(&xy, &XYZ) == 0)` | `error callback/longjmp: png_app_error(png_ptr, "invalid cHRM XYZ");` | [ ] |
| 274 | `png_set_sCAL_s` | `c_src/src/pngset.c:623: (unit != 1 && unit != 2)` | `error callback/longjmp: png_error(png_ptr, "Invalid sCAL unit");` | [ ] |
| 275 | `png_set_sCAL_s` | `c_src/src/pngset.c:627: (swidth == NULL \|\| (lengthw = strlen(swidth)) == 0 \|\| swidth[0] == 45 \|\| !png_check_fp_string(swidth, lengthw))` | `error callback/longjmp: png_error(png_ptr, "Invalid sCAL width");` | [ ] |
| 276 | `png_set_sCAL_s` | `c_src/src/pngset.c:631: (sheight == NULL \|\| (lengthh = strlen(sheight)) == 0 \|\| sheight[0] == 45 \|\| !png_check_fp_string(sheight, lengthh))` | `error callback/longjmp: png_error(png_ptr, "Invalid sCAL height");` | [ ] |
| 277 | `png_set_PLTE` | `c_src/src/pngset.c:767: (info_ptr->color_type == PNG_COLOR_TYPE_PALETTE)` | `error callback/longjmp: png_error(png_ptr, "Invalid palette length");` | [ ] |
| 278 | `png_set_PLTE` | `c_src/src/pngset.c:784: ((num_palette > 0 && palette == NULL) \|\| (num_palette == 0 # ifdef PNG_MNG_FEATURES_SUPPORTED && (png_ptr->mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0 # endif ))` | `error callback/longjmp: png_error(png_ptr, "Invalid palette");` | [ ] |
| 279 | `png_set_iCCP` | `c_src/src/pngset.c:904: (compression_type != PNG_COMPRESSION_TYPE_BASE)` | `error callback/longjmp: png_app_error(png_ptr, "Invalid iCCP compression method");` | [ ] |
| 280 | `png_set_iCCP` | `c_src/src/pngset.c:911: (new_iccp_name == NULL)` | `error callback/longjmp: png_benign_error(png_ptr, "Insufficient memory to process iCCP chunk");` | [ ] |
| 281 | `png_set_iCCP` | `c_src/src/pngset.c:923: (new_iccp_profile == NULL)` | `error callback/longjmp: png_benign_error(png_ptr, "Insufficient memory to process iCCP profile");` | [ ] |
| 282 | `png_set_text` | `c_src/src/pngset.c:950: (ret != 0)` | `error callback/longjmp: png_error(png_ptr, "Insufficient memory to store text");` | [ ] |
| 283 | `png_set_sPLT` | `c_src/src/pngset.c:1327: (entries->name == NULL \|\| entries->entries == NULL)` | `error callback/longjmp: png_app_error(png_ptr, "png_set_sPLT: invalid sPLT");` | [ ] |
| 284 | `check_location` | `c_src/src/pngset.c:1407: (location == 0)` | `error callback/longjmp: png_error(png_ptr, "invalid location in png_set_unknown_chunks");` | [ ] |
| 285 | `png_set_unknown_chunks` | `c_src/src/pngset.c:1442: ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "no unknown chunk support on read");` | [ ] |
| 286 | `png_set_unknown_chunks` | `c_src/src/pngset.c:1451: ((png_ptr->mode & PNG_IS_READ_STRUCT) == 0)` | `error callback/longjmp: png_app_error(png_ptr, "no unknown chunk support on write");` | [ ] |
| 287 | `png_set_unknown_chunk_location` | `c_src/src/pngset.c:1540: ((location & (PNG_HAVE_IHDR\|PNG_HAVE_PLTE\|PNG_AFTER_IDAT)) == 0)` | `error callback/longjmp: png_app_error(png_ptr, "invalid unknown chunk location");` | [ ] |
| 288 | `png_set_keep_unknown_chunks` | `c_src/src/pngset.c:1611: (keep < 0 \|\| keep >= PNG_HANDLE_CHUNK_LAST)` | `error callback/longjmp: png_app_error(png_ptr, "png_set_keep_unknown_chunks: invalid keep");` | [ ] |
| 289 | `png_set_keep_unknown_chunks` | `c_src/src/pngset.c:1665: (chunk_list == NULL)` | `error callback/longjmp: png_app_error(png_ptr, "png_set_keep_unknown_chunks: no chunk list");` | [ ] |
| 290 | `png_set_keep_unknown_chunks` | `c_src/src/pngset.c:1681: (num_chunks + old_num_chunks > UINT_MAX/5)` | `error callback/longjmp: png_app_error(png_ptr, "png_set_keep_unknown_chunks: too many chunks");` | [ ] |
| 291 | `png_set_compression_buffer_size` | `c_src/src/pngset.c:1805: (size == 0 \|\| size > PNG_UINT_31_MAX)` | `error callback/longjmp: png_error(png_ptr, "invalid compression buffer size");` | [ ] |
| 292 | `png_set_shift` | `c_src/src/pngtrans.c:114: (invalid)` | `error callback/longjmp: png_app_error(png_ptr, "png_set_shift: invalid shift values");` | [ ] |
| 293 | `png_set_filler` | `c_src/src/pngtrans.c:171: unconditional rejection/state failure` | `error callback/longjmp: png_app_error(png_ptr, "png_set_filler not supported on read");` | [ ] |
| 294 | `png_set_filler` | `c_src/src/pngtrans.c:202: (png_ptr->bit_depth >= 8)` | `error callback/longjmp: png_app_error(png_ptr, "png_set_filler is invalid for" " low bit depth gray output");` | [ ] |
| 295 | `png_set_filler` | `c_src/src/pngtrans.c:209: unconditional rejection/state failure` | `error callback/longjmp: png_app_error(png_ptr, "png_set_filler: inappropriate color type");` | [ ] |
| 296 | `png_set_filler` | `c_src/src/pngtrans.c:214: unconditional rejection/state failure` | `error callback/longjmp: png_app_error(png_ptr, "png_set_filler not supported on write");` | [ ] |
| 297 | `png_set_user_transform_info` | `c_src/src/pngtrans.c:845: ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0 && (png_ptr->flags & PNG_FLAG_ROW_INIT) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "info change after png_start_read_image or png_read_update_info");` | [ ] |
| 298 | `png_get_user_transform_ptr` | `c_src/src/pngtrans.c:867: (png_ptr == NULL)` | `sentinel return: return NULL;` | [ ] |
| 299 | `png_write_data` | `c_src/src/pngwio.c:40: (png_ptr->write_data_fn != NULL )` | `error callback/longjmp: png_error(png_ptr, "Call to NULL write function");` | [ ] |
| 300 | `png_default_write_data` | `c_src/src/pngwio.c:60: (check != length)` | `error callback/longjmp: png_error(png_ptr, "Write Error");` | [ ] |
| 301 | `png_write_info` | `c_src/src/pngwrite.c:241: (info_ptr->color_type == PNG_COLOR_TYPE_PALETTE)` | `error callback/longjmp: png_error(png_ptr, "Valid palette required for paletted images");` | [ ] |
| 302 | `png_write_end` | `c_src/src/pngwrite.c:400: ((png_ptr->mode & PNG_HAVE_IDAT) == 0)` | `error callback/longjmp: png_error(png_ptr, "No IDATs written into file");` | [ ] |
| 303 | `png_write_end` | `c_src/src/pngwrite.c:405: (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && png_ptr->num_palette_max >= png_ptr->num_palette)` | `error callback/longjmp: png_benign_error(png_ptr, "Wrote palette index exceeding num_palette");` | [ ] |
| 304 | `png_write_row` | `c_src/src/pngwrite.c:762: ((png_ptr->mode & PNG_WROTE_INFO_BEFORE_PLTE) == 0)` | `error callback/longjmp: png_error(png_ptr, "png_write_info was never called before png_write_row");` | [ ] |
| 305 | `png_write_row` | `c_src/src/pngwrite.c:918: (row_info.pixel_depth != png_ptr->pixel_depth \|\| row_info.pixel_depth != png_ptr->transformed_pixel_depth)` | `error callback/longjmp: png_error(png_ptr, "internal write transform logic error");` | [ ] |
| 306 | `png_set_filter` | `c_src/src/pngwrite.c:1078: (method == PNG_FILTER_TYPE_BASE)` | `error callback/longjmp: case 7: png_app_error(png_ptr, "Unknown row filter for method 0");` | [ ] |
| 307 | `png_set_filter` | `c_src/src/pngwrite.c:1101: unconditional rejection/state failure` | `error callback/longjmp: png_app_error(png_ptr, "Unknown row filter for method 0");` | [ ] |
| 308 | `png_set_filter` | `c_src/src/pngwrite.c:1180: (png_ptr->tst_row == NULL)` | `error callback/longjmp: png_error(png_ptr, "Unknown custom filter method");` | [ ] |
| 309 | `png_write_png` | `c_src/src/pngwrite.c:1417: ((info_ptr->valid & PNG_INFO_IDAT) == 0)` | `error callback/longjmp: png_app_error(png_ptr, "no rows for png_write_image to write");` | [ ] |
| 310 | `png_write_png` | `c_src/src/pngwrite.c:1431: ((transforms & PNG_TRANSFORM_INVERT_MONO) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_INVERT_MONO not supported");` | [ ] |
| 311 | `png_write_png` | `c_src/src/pngwrite.c:1442: ((info_ptr->valid & PNG_INFO_sBIT) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_SHIFT not supported");` | [ ] |
| 312 | `png_write_png` | `c_src/src/pngwrite.c:1450: ((transforms & PNG_TRANSFORM_PACKING) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_PACKING not supported");` | [ ] |
| 313 | `png_write_png` | `c_src/src/pngwrite.c:1458: ((transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_SWAP_ALPHA not supported");` | [ ] |
| 314 | `png_write_png` | `c_src/src/pngwrite.c:1472: ((transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported");` | [ ] |
| 315 | `png_write_png` | `c_src/src/pngwrite.c:1482: ((transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_STRIP_FILLER not supported");` | [ ] |
| 316 | `png_write_png` | `c_src/src/pngwrite.c:1491: ((transforms & PNG_TRANSFORM_BGR) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_BGR not supported");` | [ ] |
| 317 | `png_write_png` | `c_src/src/pngwrite.c:1499: ((transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_SWAP_ENDIAN not supported");` | [ ] |
| 318 | `png_write_png` | `c_src/src/pngwrite.c:1507: ((transforms & PNG_TRANSFORM_PACKSWAP) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_PACKSWAP not supported");` | [ ] |
| 319 | `png_write_png` | `c_src/src/pngwrite.c:1515: ((transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0)` | `error callback/longjmp: png_app_error(png_ptr, "PNG_TRANSFORM_INVERT_ALPHA not supported");` | [ ] |
| 320 | `png_image_write_init` | `c_src/src/pngwrite.c:1567: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_write_: out of memory");` | [ ] |
| 321 | `png_write_image_16bit` | `c_src/src/pngwrite.c:1629: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "png_write_image: internal call error");` | [ ] |
| 322 | `png_image_write_main` | `c_src/src/pngwrite.c:2045: (image->height > 0xffffffffU/png_row_stride)` | `error callback/longjmp: png_error(image->opaque->png_ptr, "memory image too large");` | [ ] |
| 323 | `png_image_write_main` | `c_src/src/pngwrite.c:2049: (image->height > 0xffffffffU/png_row_stride)` | `error callback/longjmp: png_error(image->opaque->png_ptr, "supplied row stride too small");` | [ ] |
| 324 | `png_image_write_main` | `c_src/src/pngwrite.c:2053: (image->height > 0xffffffffU/png_row_stride)` | `error callback/longjmp: png_error(image->opaque->png_ptr, "image row stride too large");` | [ ] |
| 325 | `png_image_write_main` | `c_src/src/pngwrite.c:2072: unconditional rejection/state failure` | `error callback/longjmp: png_error(image->opaque->png_ptr, "no color-map for color-mapped image");` | [ ] |
| 326 | `png_image_write_main` | `c_src/src/pngwrite.c:2156: ((format & ~(png_uint_32)(PNG_FORMAT_FLAG_COLOR \| PNG_FORMAT_FLAG_LINEAR \| PNG_FORMAT_FLAG_ALPHA \| PNG_FORMAT_FLAG_COLORMAP)) != 0)` | `error callback/longjmp: png_error(png_ptr, "png_write_image: unsupported transformation");` | [ ] |
| 327 | `image_memory_write` | `c_src/src/pngwrite.c:2253: (display->memory_bytes >= ob+size)` | `error callback/longjmp: png_error(png_ptr, "png_image_write_to_memory: PNG too big");` | [ ] |
| 328 | `png_image_write_to_memory` | `c_src/src/pngwrite.c:2332: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_write_to_memory: invalid argument");` | [x] |
| 329 | `png_image_write_to_memory` | `c_src/src/pngwrite.c:2337: (image != NULL)` | `error callback/longjmp: return png_image_error(image, "png_image_write_to_memory: incorrect PNG_IMAGE_VERSION");` | [x] |
| 330 | `png_image_write_to_stdio` | `c_src/src/pngwrite.c:2382: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_write_to_stdio: invalid argument");` | [ ] |
| 331 | `png_image_write_to_stdio` | `c_src/src/pngwrite.c:2387: (image != NULL)` | `error callback/longjmp: return png_image_error(image, "png_image_write_to_stdio: incorrect PNG_IMAGE_VERSION");` | [ ] |
| 332 | `png_image_write_to_file` | `c_src/src/pngwrite.c:2432: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, strerror(error));` | [ ] |
| 333 | `png_image_write_to_file` | `c_src/src/pngwrite.c:2445: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, strerror(errno));` | [ ] |
| 334 | `png_image_write_to_file` | `c_src/src/pngwrite.c:2449: unconditional rejection/state failure` | `error callback/longjmp: return png_image_error(image, "png_image_write_to_file: invalid argument");` | [ ] |
| 335 | `png_image_write_to_file` | `c_src/src/pngwrite.c:2454: (image != NULL)` | `error callback/longjmp: return png_image_error(image, "png_image_write_to_file: incorrect PNG_IMAGE_VERSION");` | [ ] |
| 336 | `png_write_complete_chunk` | `c_src/src/pngwutil.c:200: (length > PNG_UINT_31_MAX)` | `error callback/longjmp: png_error(png_ptr, "length exceeds PNG maximum");` | [ ] |
| 337 | `png_deflate_claim` | `c_src/src/pngwutil.c:334: (png_ptr->zowner == png_IDAT)` | `sentinel return: return Z_STREAM_ERROR;` | [ ] |
| 338 | `png_deflate_claim` | `c_src/src/pngwutil.c:339: (png_ptr->zowner == png_IDAT)` | `error callback/longjmp: png_error(png_ptr, msg);` | [ ] |
| 339 | `png_deflate_claim` | `c_src/src/pngwutil.c:448: (ret == Z_OK)` | `error callback/longjmp: png_zstream_error(png_ptr, ret);` | [ ] |
| 340 | `png_text_compress` | `c_src/src/pngwutil.c:626: (output_len + prefix_len >= PNG_UINT_31_MAX)` | `error callback/longjmp: png_zstream_error(png_ptr, ret);` | [ ] |
| 341 | `png_write_compressed_data_out` | `c_src/src/pngwutil.c:680: (output_len > 0)` | `error callback/longjmp: png_error(png_ptr, "error writing ancillary chunked compressed data");` | [ ] |
| 342 | `png_write_IHDR` | `c_src/src/pngwutil.c:714: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "Invalid bit depth for grayscale image");` | [ ] |
| 343 | `png_write_IHDR` | `c_src/src/pngwutil.c:725: (is_invalid_depth)` | `error callback/longjmp: png_error(png_ptr, "Invalid bit depth for RGB image");` | [ ] |
| 344 | `png_write_IHDR` | `c_src/src/pngwutil.c:741: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "Invalid bit depth for paletted image");` | [ ] |
| 345 | `png_write_IHDR` | `c_src/src/pngwutil.c:751: (is_invalid_depth)` | `error callback/longjmp: png_error(png_ptr, "Invalid bit depth for grayscale+alpha image");` | [ ] |
| 346 | `png_write_IHDR` | `c_src/src/pngwutil.c:762: (is_invalid_depth)` | `error callback/longjmp: png_error(png_ptr, "Invalid bit depth for RGBA image");` | [ ] |
| 347 | `png_write_IHDR` | `c_src/src/pngwutil.c:768: (is_invalid_depth)` | `error callback/longjmp: png_error(png_ptr, "Invalid image color type specified");` | [ ] |
| 348 | `png_write_PLTE` | `c_src/src/pngwutil.c:879: (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE)` | `error callback/longjmp: png_error(png_ptr, "Invalid number of colors in palette");` | [ ] |
| 349 | `png_compress_IDAT` | `c_src/src/pngwutil.c:954: (png_deflate_claim(png_ptr, png_IDAT, png_image_size(png_ptr)) != Z_OK)` | `error callback/longjmp: png_error(png_ptr, png_ptr->zstream.msg);` | [ ] |
| 350 | `png_compress_IDAT` | `c_src/src/pngwutil.c:1033: (flush == Z_FINISH)` | `error callback/longjmp: png_error(png_ptr, "Z_OK on Z_FINISH with output space");` | [ ] |
| 351 | `png_compress_IDAT` | `c_src/src/pngwutil.c:1066: unconditional rejection/state failure` | `error callback/longjmp: png_zstream_error(png_ptr, ret);` | [ ] |
| 352 | `png_compress_IDAT` | `c_src/src/pngwutil.c:1067: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, png_ptr->zstream.msg);` | [ ] |
| 353 | `png_write_iCCP` | `c_src/src/pngwutil.c:1132: (profile == NULL)` | `error callback/longjmp: png_error(png_ptr, "No profile for iCCP chunk"); /* internal error */` | [ ] |
| 354 | `png_write_iCCP` | `c_src/src/pngwutil.c:1135: (profile_len < 132)` | `error callback/longjmp: png_error(png_ptr, "ICC profile too short");` | [ ] |
| 355 | `png_write_iCCP` | `c_src/src/pngwutil.c:1138: (png_get_uint_32(profile) != profile_len)` | `error callback/longjmp: png_error(png_ptr, "Incorrect data in iCCP");` | [ ] |
| 356 | `png_write_iCCP` | `c_src/src/pngwutil.c:1142: (temp > 3 && (profile_len & 0x03))` | `error callback/longjmp: png_error(png_ptr, "ICC profile length invalid (not a multiple of 4)");` | [ ] |
| 357 | `png_write_iCCP` | `c_src/src/pngwutil.c:1148: (profile_len != embedded_profile_len)` | `error callback/longjmp: png_error(png_ptr, "Profile length does not match profile");` | [ ] |
| 358 | `png_write_iCCP` | `c_src/src/pngwutil.c:1154: (name_len == 0)` | `error callback/longjmp: png_error(png_ptr, "iCCP: invalid keyword");` | [ ] |
| 359 | `png_write_iCCP` | `c_src/src/pngwutil.c:1165: (png_text_compress(png_ptr, png_iCCP, &comp, name_len) != Z_OK)` | `error callback/longjmp: png_error(png_ptr, png_ptr->zstream.msg);` | [ ] |
| 360 | `png_write_sPLT` | `c_src/src/pngwutil.c:1194: (name_len == 0)` | `error callback/longjmp: png_error(png_ptr, "sPLT: invalid keyword");` | [ ] |
| 361 | `png_write_tEXt` | `c_src/src/pngwutil.c:1580: (key_len == 0)` | `error callback/longjmp: png_error(png_ptr, "tEXt: invalid keyword");` | [ ] |
| 362 | `png_write_tEXt` | `c_src/src/pngwutil.c:1589: (text_len > PNG_UINT_31_MAX - (key_len+1))` | `error callback/longjmp: png_error(png_ptr, "tEXt: text too long");` | [ ] |
| 363 | `png_write_zTXt` | `c_src/src/pngwutil.c:1628: (compression != PNG_TEXT_COMPRESSION_zTXt)` | `error callback/longjmp: png_error(png_ptr, "zTXt: invalid compression type");` | [ ] |
| 364 | `png_write_zTXt` | `c_src/src/pngwutil.c:1633: (key_len == 0)` | `error callback/longjmp: png_error(png_ptr, "zTXt: invalid keyword");` | [ ] |
| 365 | `png_write_zTXt` | `c_src/src/pngwutil.c:1644: (png_text_compress(png_ptr, png_zTXt, &comp, key_len) != Z_OK)` | `error callback/longjmp: png_error(png_ptr, png_ptr->zstream.msg);` | [ ] |
| 366 | `png_write_iTXt` | `c_src/src/pngwutil.c:1676: (key_len == 0)` | `error callback/longjmp: png_error(png_ptr, "iTXt: invalid keyword");` | [ ] |
| 367 | `png_write_iTXt` | `c_src/src/pngwutil.c:1692: unconditional rejection/state failure` | `error callback/longjmp: png_error(png_ptr, "iTXt: invalid compression");` | [ ] |
| 368 | `png_write_iTXt` | `c_src/src/pngwutil.c:1730: (png_text_compress(png_ptr, png_iTXt, &comp, prefix_len) != Z_OK)` | `error callback/longjmp: png_error(png_ptr, png_ptr->zstream.msg);` | [ ] |
| 369 | `png_write_iTXt` | `c_src/src/pngwutil.c:1736: (comp.input_len > PNG_UINT_31_MAX-prefix_len)` | `error callback/longjmp: png_error(png_ptr, "iTXt: uncompressed text too long");` | [ ] |
| 370 | `png_write_pCAL` | `c_src/src/pngwutil.c:1797: (type >= PNG_EQUATION_LAST)` | `error callback/longjmp: png_error(png_ptr, "Unrecognized equation type for pCAL chunk");` | [ ] |
| 371 | `png_write_pCAL` | `c_src/src/pngwutil.c:1802: (purpose_len == 0)` | `error callback/longjmp: png_error(png_ptr, "pCAL: invalid keyword");` | [ ] |
