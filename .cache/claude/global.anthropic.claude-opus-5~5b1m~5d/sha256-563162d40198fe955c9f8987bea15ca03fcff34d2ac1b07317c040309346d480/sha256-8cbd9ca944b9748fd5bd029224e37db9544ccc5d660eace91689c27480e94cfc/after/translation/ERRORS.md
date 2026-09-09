# ERRORS.md — error-surface table (derived mechanically from `c_src/src/*.c`)

Every row is ONE distinct rejection site in the C source, found with:

```
grep -nE '\b(png_error|png_chunk_error|png_benign_error|png_app_error|png_warning|png_app_warning|png_chunk_warning|png_chunk_report|png_fixed_error|png_longjmp)[ ]*\(' c_src/src/*.c
```

392 sites total.  There are **no `assert`s** anywhere in the library
(`grep -rn assert c_src/src` finds only comments), so every rejection is one of
these calls or a sentinel return (see the sentinel section at the end).

How the "expected C result" is observed in the differential tests: both libraries get an
`error_fn`/`warning_fn` installed with `png_set_error_fn`, which records the exact message
bytes; the error callback then unwinds (panic, standing in for the app's `longjmp`).  A test
asserts C and Rust produce (a) the same sequence of recorded (kind, message) pairs and
(b) the same return value / `png_image.warning_or_error` + `png_image.message`.

Legend for "expected C result": see the table right below; `msg` is the message column.

| macro | expected C behaviour |
|---|---|
| `png_error` | FATAL: error_fn(msg) then longjmp/abort — never returns |
| `png_chunk_error` | FATAL: error_fn("<chunkname>: msg") then longjmp |
| `png_fixed_error` | FATAL: error_fn("fixed point overflow in <s>") |
| `png_longjmp` | FATAL: longjmp_fn(jmp_buf, val) / PNG_ABORT |
| `png_benign_error` | FATAL if flags&BENIGN_*_ERRORS_WARN==0 else warning_fn(msg) |
| `png_app_error` | FATAL unless flags&APP_WARNINGS_WARN -> warning_fn(msg) |
| `png_warning` | warning_fn(msg), returns normally |
| `png_app_warning` | warning_fn(msg), returns normally |
| `png_chunk_warning` | warning_fn("<chunkname>: msg") |
| `png_chunk_report` | PNG_CHUNK_ERROR->png_chunk_error / WARNING->png_chunk_warning / benign |

## Rejection sites

| # | function (file:line) | macro | trigger (guard in the C source) | message / expected C result | test |
|---|---|---|---|---|---|
| 1 | `png_set_sig_bytes` (png.c:66) | `png_error` | `if (nb > 8)` | "Too many bytes for PNG signature" || [x] |
| 2 | `PNG_FUNCTION` (png.c:120) | `png_warning` | `if (size != 0 && items >= (~(png_alloc_size_t)0) / size) {` | "Potential overflow in png_zalloc()" || n/a |
| 3 | `png_user_version_check` (png.c:245) | `png_warning` | `(unconditional at that point)` | "" || [x] |
| 4 | `png_data_freer` (png.c:479) | `png_error` | `else if (freer == PNG_USER_WILL_FREE_DATA) info_ptr->free_me &= ~mask; else` | "Unknown freer parameter in png_data_freer" || [x] |
| 5 | `png_convert_to_rfc1123` (png.c:802) | `png_warning` | `if (png_convert_to_rfc1123_buffer(png_ptr->time_buffer, ptime) == 0)` | "Ignoring invalid time value" || [x] |
| 6 | `png_set_rgb_coefficients` (png.c:1938) | `png_error` | `if (r+g+b != 32768)` | "internal error handling cHRM coefficients" || n/a |
| 7 | `png_check_IHDR` (png.c:1971) | `png_warning` | `if (width == 0) {` | "Image width is zero in IHDR" || [x] |
| 8 | `png_check_IHDR` (png.c:1977) | `png_warning` | `if (width > PNG_UINT_31_MAX) {` | "Invalid image width in IHDR" || [x] |
| 9 | `png_check_IHDR` (png.c:2007) | `png_warning` | `(unconditional at that point)` | "Image width is too large for this architecture" || n/a |
| 10 | `png_check_IHDR` (png.c:2017) | `png_warning` | `if (width > PNG_USER_WIDTH_MAX) #endif {` | "Image width exceeds user limit in IHDR" || [x] |
| 11 | `png_check_IHDR` (png.c:2023) | `png_warning` | `if (height == 0) {` | "Image height is zero in IHDR" || [x] |
| 12 | `png_check_IHDR` (png.c:2029) | `png_warning` | `if (height > PNG_UINT_31_MAX) {` | "Invalid image height in IHDR" || [x] |
| 13 | `png_check_IHDR` (png.c:2039) | `png_warning` | `if (height > PNG_USER_HEIGHT_MAX) #endif {` | "Image height exceeds user limit in IHDR" || [x] |
| 14 | `png_check_IHDR` (png.c:2047) | `png_warning` | `if (bit_depth != 1 && bit_depth != 2 && bit_depth != 4 && bit_depth != 8 && bit_depth != 16) {` | "Invalid bit depth in IHDR" || [x] |
| 15 | `png_check_IHDR` (png.c:2054) | `png_warning` | `if (color_type < 0 \|\| color_type == 1 \|\| color_type == 5 \|\| color_type > 6) {` | "Invalid color type in IHDR" || [x] |
| 16 | `png_check_IHDR` (png.c:2063) | `png_warning` | `color_type == PNG_COLOR_TYPE_GRAY_ALPHA \|\| color_type == PNG_COLOR_TYPE_RGB_ALPHA) && bit_depth < 8)) {` | "Invalid color type/bit depth combination in IHDR" || [x] |
| 17 | `png_check_IHDR` (png.c:2069) | `png_warning` | `if (interlace_type >= PNG_INTERLACE_LAST) {` | "Unknown interlace method in IHDR" || [x] |
| 18 | `png_check_IHDR` (png.c:2075) | `png_warning` | `if (compression_type != PNG_COMPRESSION_TYPE_BASE) {` | "Unknown compression method in IHDR" || [x] |
| 19 | `png_check_IHDR` (png.c:2091) | `png_warning` | `if ((png_ptr->mode & PNG_HAVE_PNG_SIGNATURE) != 0 && png_ptr->mng_features_permitted != 0)` | "MNG features are not allowed in a PNG datastream" || [x] |
| 20 | `png_check_IHDR` (png.c:2101) | `png_warning` | `(color_type == PNG_COLOR_TYPE_RGB \|\| color_type == PNG_COLOR_TYPE_RGB_ALPHA))) {` | "Unknown filter method in IHDR" || [x] |
| 21 | `png_check_IHDR` (png.c:2107) | `png_warning` | `if ((png_ptr->mode & PNG_HAVE_PNG_SIGNATURE) != 0) {` | "Invalid filter method in IHDR" || [x] |
| 22 | `png_check_IHDR` (png.c:2115) | `png_warning` | `if (filter_type != PNG_FILTER_TYPE_BASE) {` | "Unknown filter method in IHDR" || n/a |
| 23 | `png_check_IHDR` (png.c:2121) | `png_error` | `if (error == 1)` | "Invalid IHDR data" || [x] |
| 24 | `png_ascii_from_fp` (png.c:2635) | `png_error` | `(unconditional at that point)` | "ASCII conversion buffer too small" || [x] |
| 25 | `png_ascii_from_fixed` (png.c:2713) | `png_error` | `(unconditional at that point)` | "ASCII conversion buffer too small" || [x] |
| 26 | `png_fixed` (png.c:2731) | `png_fixed_error` | `if (r > 2147483647. \|\| r < -2147483648.)` | "" || [x] |
| 27 | `png_fixed_ITU` (png.c:2750) | `png_fixed_error` | `if (r > 2147483647. \|\| r < 0)` | "" || [x] |
| 28 | `png_gamma_correct` (png.c:3377) | `png_error` | `(unconditional at that point)` | "" || n/a |
| 29 | `png_build_gamma_table` (png.c:3634) | `png_warning` | `if (png_ptr->gamma_table != NULL \|\| png_ptr->gamma_16_table != NULL) {` | "gamma table being rebuilt" || [x] |
| 30 | `png_image_free_function` (png.c:4002) | `png_error` | `if (c.for_write != 0) { # ifdef PNG_SIMPLIFIED_WRITE_SUPPORTED png_destroy_write_struct(&c.png_ptr, &c.info_ptr); # else` | "simplified write not supported" || n/a |
| 31 | `png_image_free_function` (png.c:4010) | `png_error` | `(unconditional at that point)` | "simplified read not supported" || n/a |
| 32 | `png_warning` (pngerror.c:177) | `png_warning` | `(unconditional at that point)` | "" || [x] |
| 33 | `png_formatted_warning` (pngerror.c:302) | `png_warning` | `(unconditional at that point)` | "" || [x] |
| 34 | `png_benign_error` (pngerror.c:308) | `png_benign_error` | `(unconditional at that point)` | "" || [x] |
| 35 | `png_benign_error` (pngerror.c:315) | `png_chunk_warning` | `if ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0 && png_ptr->chunk_name != 0)` | "" || [x] |
| 36 | `png_benign_error` (pngerror.c:318) | `png_warning` | `if ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0 && png_ptr->chunk_name != 0) png_chunk_warning(png_ptr, error_message); else # endif` | "" || [x] |
| 37 | `png_benign_error` (pngerror.c:326) | `png_chunk_error` | `if ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0 && png_ptr->chunk_name != 0)` | "" || [x] |
| 38 | `png_benign_error` (pngerror.c:329) | `png_error` | `if ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0 && png_ptr->chunk_name != 0) png_chunk_error(png_ptr, error_message); else # endif` | "" || [x] |
| 39 | `png_app_warning` (pngerror.c:338) | `png_app_warning` | `(unconditional at that point)` | "" || [x] |
| 40 | `png_app_warning` (pngerror.c:341) | `png_warning` | `if ((png_ptr->flags & PNG_FLAG_APP_WARNINGS_WARN) != 0)` | "" || [x] |
| 41 | `png_app_warning` (pngerror.c:343) | `png_error` | `if ((png_ptr->flags & PNG_FLAG_APP_WARNINGS_WARN) != 0) png_warning(png_ptr, error_message); else` | "" || [x] |
| 42 | `png_app_error` (pngerror.c:351) | `png_app_error` | `(unconditional at that point)` | "" || [x] |
| 43 | `png_app_error` (pngerror.c:354) | `png_warning` | `if ((png_ptr->flags & PNG_FLAG_APP_ERRORS_WARN) != 0)` | "" || [x] |
| 44 | `png_app_error` (pngerror.c:356) | `png_error` | `if ((png_ptr->flags & PNG_FLAG_APP_ERRORS_WARN) != 0) png_warning(png_ptr, error_message); else` | "" || [x] |
| 45 | `PNG_FUNCTION` (pngerror.c:431) | `png_error` | `if (png_ptr == NULL)` | "" || n/a |
| 46 | `PNG_FUNCTION` (pngerror.c:436) | `png_error` | `if (png_ptr == NULL) png_error(png_ptr, error_message); else {` | "" || [x] |
| 47 | `png_chunk_warning` (pngerror.c:443) | `png_chunk_warning` | `(unconditional at that point)` | "" || [x] |
| 48 | `png_chunk_warning` (pngerror.c:447) | `png_warning` | `if (png_ptr == NULL)` | "" || [x] |
| 49 | `png_chunk_warning` (pngerror.c:452) | `png_warning` | `if (png_ptr == NULL) png_warning(png_ptr, warning_message); else {` | "" || [x] |
| 50 | `png_chunk_benign_error` (pngerror.c:464) | `png_chunk_warning` | `if ((png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0)` | "" || [x] |
| 51 | `png_chunk_benign_error` (pngerror.c:467) | `png_chunk_error` | `if ((png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0) png_chunk_warning(png_ptr, error_message); else` | "" || [x] |
| 52 | `png_chunk_report` (pngerror.c:477) | `png_chunk_report` | `(unconditional at that point)` | "" || [x] |
| 53 | `png_chunk_report` (pngerror.c:493) | `png_chunk_warning` | `if (error < PNG_CHUNK_ERROR)` | "" || [x] |
| 54 | `png_chunk_report` (pngerror.c:507) | `png_app_warning` | `if (error < PNG_CHUNK_WRITE_ERROR)` | "" || [x] |
| 55 | `png_chunk_report` (pngerror.c:510) | `png_app_error` | `if (error < PNG_CHUNK_WRITE_ERROR) png_app_warning(png_ptr, message); else` | "" || [x] |
| 56 | `PNG_FUNCTION` (pngerror.c:534) | `png_error` | `while (iin < (PNG_MAX_ERROR_TEXT-1) && name[iin] != 0) { msg[fixed_message_ln + iin] = name[iin]; ++iin; }` | "" || [x] |
| 57 | `png_set_longjmp_fn` (pngerror.c:593) | `png_error` | `if (png_ptr->jmp_buf_ptr != &png_ptr->jmp_buf_local) { /* This is an internal error in libpng: somehow we have been left * with a stack allocated jmp_buf when the application regained * control. It's` | "Libpng jmp_buf still allocated" || n/a |
| 58 | `png_set_longjmp_fn` (pngerror.c:600) | `png_warning` | `if (size != jmp_buf_size) {` | "Application jmp_buf size changed" || [x] |
| 59 | `PNG_FUNCTION` (pngerror.c:668) | `png_longjmp` | `(unconditional at that point)` | "" || [x] |
| 60 | `PNG_FUNCTION` (pngerror.c:684) | `png_longjmp` | `if (png_ptr != NULL && png_ptr->longjmp_fn != NULL && png_ptr->jmp_buf_ptr != NULL) png_ptr->longjmp_fn(*png_ptr->jmp_buf_ptr, val); #else PNG_UNUSED(png_ptr)` | "" || [x] |
| 61 | `png_fixed_inches_from_microns` (pngget.c:388) | `png_warning` | `if (png_muldiv(&result, microns, 500, 127) != 0) return result;` | "fixed point overflow ignored" || [x] |
| 62 | `png_get_eXIf` (pngget.c:895) | `png_warning` | `(unconditional at that point)` | "png_get_eXIf does not work; use png_get_eXIf_1" || [x] |
| 63 | `PNG_FUNCTION` (pngmem.c:126) | `png_error` | `if (nelements <= 0 \|\| element_size == 0)` | "internal error: array alloc" || [x] |
| 64 | `PNG_FUNCTION` (pngmem.c:139) | `png_error` | `if (add_elements <= 0 \|\| element_size == 0 \|\| old_elements < 0 \|\| (old_array == NULL && old_elements > 0))` | "internal error: array realloc" || [x] |
| 65 | `PNG_FUNCTION` (pngmem.c:184) | `png_error` | `if (ret == NULL)` | "Out of memory" || [x] |
| 66 | `PNG_FUNCTION` (pngmem.c:203) | `png_error` | `if (ret == NULL)` | "Out of Memory" || [x] |
| 67 | `PNG_FUNCTION` (pngmem.c:224) | `png_warning` | `if (ret != NULL) return ret;` | "Out of memory" || [x] |
| 68 | `png_process_data_skip` (pngpread.c:99) | `png_app_warning` | `(unconditional at that point)` | "png_process_data_skip is not implemented in any current version of libpng" || [x] |
| 69 | `png_push_read_sig` (pngpread.c:166) | `png_error` | `if (num_checked < 4 && png_sig_cmp(info_ptr->signature, num_checked, num_to_check - 4) != 0)` | "Not a PNG file" || [x] |
| 70 | `png_push_read_sig` (pngpread.c:169) | `png_error` | `if (num_checked < 4 && png_sig_cmp(info_ptr->signature, num_checked, num_to_check - 4) != 0) png_error(png_ptr, "Not a PNG file"); else` | "PNG file corrupted by ASCII conversion" || [x] |
| 71 | `png_push_read_chunk` (pngpread.c:213) | `png_error` | `if ((png_ptr->mode & PNG_HAVE_IHDR) == 0)` | "Missing IHDR before IDAT" || [x] |
| 72 | `png_push_read_chunk` (pngpread.c:217) | `png_error` | `else if (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && (png_ptr->mode & PNG_HAVE_PLTE) == 0)` | "Missing PLTE before IDAT" || [x] |
| 73 | `png_push_read_chunk` (pngpread.c:229) | `png_benign_error` | `if ((png_ptr->mode & PNG_AFTER_IDAT) != 0)` | "Too many IDATs found" || [x] |
| 74 | `png_push_read_chunk` (pngpread.c:243) | `png_error` | `if (png_ptr->push_length != 13)` | "Invalid IHDR length" || [x] |
| 75 | `png_push_save_buffer` (pngpread.c:361) | `png_error` | `if (png_ptr->save_buffer_size > PNG_SIZE_MAX - (png_ptr->current_buffer_size + 256)) {` | "Potential overflow of save_buffer" || n/a |
| 76 | `png_push_save_buffer` (pngpread.c:372) | `png_error` | `if (png_ptr->save_buffer == NULL) { png_free(png_ptr, old_buffer);` | "Insufficient memory for save_buffer" || [x] |
| 77 | `png_push_save_buffer` (pngpread.c:378) | `png_error` | `else if (png_ptr->save_buffer_size)` | "save_buffer error" || n/a |
| 78 | `png_push_read_IDAT` (pngpread.c:425) | `png_error` | `if ((png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED) == 0)` | "Not enough compressed data" || [x] |
| 79 | `png_process_IDAT_data` (pngpread.c:502) | `png_error` | `if (!(buffer_length > 0) \|\| buffer == NULL)` | "No IDAT data (internal error)" || n/a |
| 80 | `png_process_IDAT_data` (pngpread.c:555) | `png_warning` | `if (png_ptr->row_number >= png_ptr->num_rows \|\| png_ptr->pass > 6)` | "Truncated compressed data in IDAT" || [x] |
| 81 | `png_process_IDAT_data` (pngpread.c:560) | `png_benign_error` | `if (ret == Z_DATA_ERROR)` | "IDAT: ADLER32 checksum mismatch" || [x] |
| 82 | `png_process_IDAT_data` (pngpread.c:562) | `png_error` | `if (ret == Z_DATA_ERROR) png_benign_error(png_ptr, "IDAT: ADLER32 checksum mismatch"); else` | "Decompression error in IDAT" || [x] |
| 83 | `png_process_IDAT_data` (pngpread.c:580) | `png_warning` | `if (png_ptr->row_number >= png_ptr->num_rows \|\| png_ptr->pass > 6) { /* Extra data. */` | "Extra compressed data in IDAT" || [x] |
| 84 | `png_process_IDAT_data` (pngpread.c:605) | `png_warning` | `if (png_ptr->zstream.avail_in > 0)` | "Extra compression data in IDAT" || [x] |
| 85 | `png_push_process_row` (pngpread.c:627) | `png_error` | `if (png_ptr->row_buf[0] < PNG_FILTER_VALUE_LAST) png_read_filter_row(png_ptr, &row_info, png_ptr->row_buf + 1, png_ptr->prev_row + 1, png_ptr->row_buf[0]); else` | "bad adaptive filter value" || [x] |
| 86 | `png_push_process_row` (pngpread.c:647) | `png_error` | `if (row_info.pixel_depth > png_ptr->maximum_pixel_depth)` | "progressive row overflow" || [x] |
| 87 | `png_push_process_row` (pngpread.c:651) | `png_error` | `else if (png_ptr->transformed_pixel_depth != row_info.pixel_depth)` | "internal progressive row size calculation error" || [x] |
| 88 | `png_read_info` (pngread.c:118) | `png_chunk_error` | `if ((png_ptr->mode & PNG_HAVE_IHDR) == 0)` | "Missing IHDR before IDAT" || [x] |
| 89 | `png_read_info` (pngread.c:122) | `png_chunk_error` | `else if (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && (png_ptr->mode & PNG_HAVE_PLTE) == 0)` | "Missing PLTE before IDAT" || [x] |
| 90 | `png_read_update_info` (pngread.c:191) | `png_app_error` | `(unconditional at that point)` | "png_read_update_info/png_start_read_image: duplicate call" || [x] |
| 91 | `png_start_read_image` (pngread.c:214) | `png_app_error` | `if ((png_ptr->flags & PNG_FLAG_ROW_INIT) == 0) png_read_start_row(png_ptr); /* New in 1.6.0 this avoids the bug of doing the initializations twice */ else` | "png_start_read_image/png_read_update_info: duplicate call" || [x] |
| 92 | `png_read_row` (pngread.c:318) | `png_warning` | `if ((png_ptr->transformations & PNG_INVERT_MONO) != 0)` | "PNG_READ_INVERT_SUPPORTED is not defined" || n/a |
| 93 | `png_read_row` (pngread.c:323) | `png_warning` | `if ((png_ptr->transformations & PNG_FILLER) != 0)` | "PNG_READ_FILLER_SUPPORTED is not defined" || n/a |
| 94 | `png_read_row` (pngread.c:329) | `png_warning` | `if ((png_ptr->transformations & PNG_PACKSWAP) != 0)` | "PNG_READ_PACKSWAP_SUPPORTED is not defined" || n/a |
| 95 | `png_read_row` (pngread.c:334) | `png_warning` | `if ((png_ptr->transformations & PNG_PACK) != 0)` | "PNG_READ_PACK_SUPPORTED is not defined" || n/a |
| 96 | `png_read_row` (pngread.c:339) | `png_warning` | `if ((png_ptr->transformations & PNG_SHIFT) != 0)` | "PNG_READ_SHIFT_SUPPORTED is not defined" || n/a |
| 97 | `png_read_row` (pngread.c:344) | `png_warning` | `if ((png_ptr->transformations & PNG_BGR) != 0)` | "PNG_READ_BGR_SUPPORTED is not defined" || n/a |
| 98 | `png_read_row` (pngread.c:349) | `png_warning` | `if ((png_ptr->transformations & PNG_SWAP_BYTES) != 0)` | "PNG_READ_SWAP_SUPPORTED is not defined" || n/a |
| 99 | `png_read_row` (pngread.c:444) | `png_error` | `if ((png_ptr->mode & PNG_HAVE_IDAT) == 0)` | "Invalid attempt to read row data" || [x] |
| 100 | `png_read_row` (pngread.c:456) | `png_error` | `if (png_ptr->row_buf[0] < PNG_FILTER_VALUE_LAST) png_read_filter_row(png_ptr, &row_info, png_ptr->row_buf + 1, png_ptr->prev_row + 1, png_ptr->row_buf[0]); else` | "bad adaptive filter value" || [x] |
| 101 | `png_read_row` (pngread.c:489) | `png_error` | `if (row_info.pixel_depth > png_ptr->maximum_pixel_depth)` | "sequential row overflow" || [x] |
| 102 | `png_read_row` (pngread.c:493) | `png_error` | `else if (png_ptr->transformed_pixel_depth != row_info.pixel_depth)` | "internal sequential row size calculation error" || [x] |
| 103 | `png_read_image` (pngread.c:635) | `png_warning` | `if (png_ptr->interlaced != 0 && (png_ptr->transformations & PNG_INTERLACE) == 0) { /* Caller called png_start_read_image or png_read_update_info without * first turning on the PNG_INTERLACE transform.` | "Interlace handling should be turned on when " || [x] |
| 104 | `png_read_image` (pngread.c:648) | `png_error` | `if (png_ptr->interlaced)` | "Cannot read interlaced image -- interlace handler disabled" || n/a |
| 105 | `png_read_end` (pngread.c:697) | `png_benign_error` | `if (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && png_ptr->num_palette_max >= png_ptr->num_palette)` | "Read palette index exceeding num_palette" || [x] |
| 106 | `png_read_end` (pngread.c:729) | `png_benign_error` | `if ((length > 0 && !(png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED)) \|\| (png_ptr->mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0)` | ".Too many IDATs found" || [x] |
| 107 | `png_read_end` (pngread.c:747) | `png_benign_error` | `if ((length > 0 && !(png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED)) \|\| (png_ptr->mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0)` | "..Too many IDATs found" || [x] |
| 108 | `png_read_png` (pngread.c:881) | `png_error` | `if (info_ptr->height > PNG_UINT_32_MAX/(sizeof (png_bytep)))` | "Image is too high to process with png_read_png()" || [x] |
| 109 | `png_read_png` (pngread.c:899) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_SCALE_16) != 0) /* Added at libpng-1.5.4. "strip_16" produces the same result that it * did in earlier versions, while "scale_16" is now more accurate. */ #ifdef PNG_RE` | "PNG_TRANSFORM_SCALE_16 not supported" || n/a |
| 110 | `png_read_png` (pngread.c:910) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_STRIP_16) != 0) #ifdef PNG_READ_STRIP_16_TO_8_SUPPORTED png_set_strip_16(png_ptr); #else` | "PNG_TRANSFORM_STRIP_16 not supported" || n/a |
| 111 | `png_read_png` (pngread.c:920) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_STRIP_ALPHA) != 0) #ifdef PNG_READ_STRIP_ALPHA_SUPPORTED png_set_strip_alpha(png_ptr); #else` | "PNG_TRANSFORM_STRIP_ALPHA not supported" || n/a |
| 112 | `png_read_png` (pngread.c:930) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_PACKING) != 0) #ifdef PNG_READ_PACK_SUPPORTED png_set_packing(png_ptr); #else` | "PNG_TRANSFORM_PACKING not supported" || n/a |
| 113 | `png_read_png` (pngread.c:940) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_PACKSWAP) != 0) #ifdef PNG_READ_PACKSWAP_SUPPORTED png_set_packswap(png_ptr); #else` | "PNG_TRANSFORM_PACKSWAP not supported" || n/a |
| 114 | `png_read_png` (pngread.c:952) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_EXPAND) != 0) #ifdef PNG_READ_EXPAND_SUPPORTED png_set_expand(png_ptr); #else` | "PNG_TRANSFORM_EXPAND not supported" || n/a |
| 115 | `png_read_png` (pngread.c:964) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_INVERT_MONO) != 0) #ifdef PNG_READ_INVERT_SUPPORTED png_set_invert_mono(png_ptr); #else` | "PNG_TRANSFORM_INVERT_MONO not supported" || n/a |
| 116 | `png_read_png` (pngread.c:976) | `png_app_error` | `if ((info_ptr->valid & PNG_INFO_sBIT) != 0) png_set_shift(png_ptr, &info_ptr->sig_bit); #else` | "PNG_TRANSFORM_SHIFT not supported" || n/a |
| 117 | `png_read_png` (pngread.c:984) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_BGR) != 0) #ifdef PNG_READ_BGR_SUPPORTED png_set_bgr(png_ptr); #else` | "PNG_TRANSFORM_BGR not supported" || n/a |
| 118 | `png_read_png` (pngread.c:992) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0) #ifdef PNG_READ_SWAP_ALPHA_SUPPORTED png_set_swap_alpha(png_ptr); #else` | "PNG_TRANSFORM_SWAP_ALPHA not supported" || n/a |
| 119 | `png_read_png` (pngread.c:1000) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0) #ifdef PNG_READ_SWAP_SUPPORTED png_set_swap(png_ptr); #else` | "PNG_TRANSFORM_SWAP_ENDIAN not supported" || n/a |
| 120 | `png_read_png` (pngread.c:1009) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0) #ifdef PNG_READ_INVERT_ALPHA_SUPPORTED png_set_invert_alpha(png_ptr); #else` | "PNG_TRANSFORM_INVERT_ALPHA not supported" || n/a |
| 121 | `png_read_png` (pngread.c:1018) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_GRAY_TO_RGB) != 0) #ifdef PNG_READ_GRAY_TO_RGB_SUPPORTED png_set_gray_to_rgb(png_ptr); #else` | "PNG_TRANSFORM_GRAY_TO_RGB not supported" || n/a |
| 122 | `png_read_png` (pngread.c:1026) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_EXPAND_16) != 0) #ifdef PNG_READ_EXPAND_16_SUPPORTED png_set_expand_16(png_ptr); #else` | "PNG_TRANSFORM_EXPAND_16 not supported" || n/a |
| 123 | `png_image_memory_read` (pngread.c:1427) | `png_error` | `if (memory != NULL && size >= need) { memcpy(out, memory, need); cp->memory = memory + need; cp->size = size - need;` | "read beyond end of data" || [x] |
| 124 | `png_image_memory_read` (pngread.c:1431) | `png_error` | `(unconditional at that point)` | "invalid memory read" || n/a |
| 125 | `set_file_encoding` (pngread.c:1537) | `png_error` | `if (g == 0)` | "internal: default gamma not set" || n/a |
| 126 | `decode_gamma` (pngread.c:1586) | `png_error` | `default:` | "unexpected encoding (internal error)" || n/a |
| 127 | `png_create_colormap_entry` (pngread.c:1643) | `png_error` | `if (ip > 255)` | "color-map index out of range" || n/a |
| 128 | `png_create_colormap_entry` (pngread.c:1743) | `png_error` | `if (encoding != output_encoding)` | "bad encoding (internal error)" || n/a |
| 129 | `png_image_read_colormap` (pngread.c:1997) | `png_error` | `else if (display->background == NULL /* no way to remove it */)` | "background color must be supplied to remove alpha/transparency" || [x] |
| 130 | `png_image_read_colormap` (pngread.c:2056) | `png_error` | `if (cmap_entries > image->colormap_entries)` | "gray[8] color-map: too few entries" || [x] |
| 131 | `png_image_read_colormap` (pngread.c:2135) | `png_error` | `if (PNG_GRAY_COLORMAP_ENTRIES > image->colormap_entries)` | "gray[16] color-map: too few entries" || [x] |
| 132 | `png_image_read_colormap` (pngread.c:2233) | `png_error` | `if (PNG_GA_COLORMAP_ENTRIES > image->colormap_entries)` | "gray+alpha color-map: too few entries" || [x] |
| 133 | `png_image_read_colormap` (pngread.c:2267) | `png_error` | `if (PNG_GRAY_COLORMAP_ENTRIES > image->colormap_entries)` | "gray-alpha color-map: too few entries" || [x] |
| 134 | `png_image_read_colormap` (pngread.c:2301) | `png_error` | `if (PNG_GA_COLORMAP_ENTRIES > image->colormap_entries)` | "ga-alpha color-map: too few entries" || [x] |
| 135 | `png_image_read_colormap` (pngread.c:2406) | `png_error` | `if (PNG_GA_COLORMAP_ENTRIES > image->colormap_entries)` | "rgb[ga] color-map: too few entries" || [x] |
| 136 | `png_image_read_colormap` (pngread.c:2422) | `png_error` | `if (PNG_GRAY_COLORMAP_ENTRIES > image->colormap_entries)` | "rgb[gray] color-map: too few entries" || [x] |
| 137 | `png_image_read_colormap` (pngread.c:2530) | `png_error` | `if (PNG_RGB_COLORMAP_ENTRIES+1+27 > image->colormap_entries)` | "rgb+alpha color-map: too few entries" || [x] |
| 138 | `png_image_read_colormap` (pngread.c:2579) | `png_error` | `if (PNG_RGB_COLORMAP_ENTRIES+1+27 > image->colormap_entries)` | "rgb-alpha color-map: too few entries" || [x] |
| 139 | `png_image_read_colormap` (pngread.c:2664) | `png_error` | `if (PNG_RGB_COLORMAP_ENTRIES > image->colormap_entries)` | "rgb color-map: too few entries" || [x] |
| 140 | `png_image_read_colormap` (pngread.c:2695) | `png_error` | `if (cmap_entries > (unsigned int)image->colormap_entries)` | "palette color-map: too few entries" || [x] |
| 141 | `png_image_read_colormap` (pngread.c:2738) | `png_error` | `default:` | "invalid PNG color type" || n/a |
| 142 | `png_image_read_colormap` (pngread.c:2761) | `png_error` | `default:` | "bad data option (internal error)" || n/a |
| 143 | `png_image_read_colormap` (pngread.c:2766) | `png_error` | `if (cmap_entries > 256 \|\| cmap_entries > image->colormap_entries)` | "color map overflow (BAD internal error)" || n/a |
| 144 | `png_image_read_colormap` (pngread.c:2800) | `png_error` | `default:` | "bad processing option (internal error)" || n/a |
| 145 | `png_image_read_colormap` (pngread.c:2803) | `png_error` | `default: png_error(png_ptr, "bad processing option (internal error)"); bad_background:` | "bad background index (internal error)" || n/a |
| 146 | `png_image_read_and_map` (pngread.c:2836) | `png_error` | `default:` | "unknown interlace type" || n/a |
| 147 | `png_image_read_colormapped` (pngread.c:3074) | `png_error` | `default: bad_output:` | "bad color-map processing (internal error)" || n/a |
| 148 | `png_image_read_direct_scaled` (pngread.c:3159) | `png_error` | `default:` | "unknown interlace type" || n/a |
| 149 | `png_image_read_composite` (pngread.c:3208) | `png_error` | `default:` | "unknown interlace type" || n/a |
| 150 | `png_image_read_background` (pngread.c:3358) | `png_error` | `if ((png_ptr->transformations & PNG_RGB_TO_GRAY) == 0)` | "lost rgb to gray" || n/a |
| 151 | `png_image_read_background` (pngread.c:3361) | `png_error` | `if ((png_ptr->transformations & PNG_COMPOSE) != 0)` | "unexpected compose" || [x] |
| 152 | `png_image_read_background` (pngread.c:3364) | `png_error` | `if (png_get_channels(png_ptr, info_ptr) != 2)` | "lost/gained channels" || n/a |
| 153 | `png_image_read_background` (pngread.c:3369) | `png_error` | `if ((image->format & PNG_FORMAT_FLAG_LINEAR) == 0 && (image->format & PNG_FORMAT_FLAG_ALPHA) != 0)` | "unexpected 8-bit transformation" || n/a |
| 154 | `png_image_read_background` (pngread.c:3382) | `png_error` | `default:` | "unknown interlace type" || n/a |
| 155 | `png_image_read_background` (pngread.c:3609) | `png_error` | `default:` | "unexpected bit depth" || n/a |
| 156 | `png_image_read_direct` (pngread.c:3925) | `png_error` | `if (change != 0)` | "png_read_image: unsupported transformation" || [x] |
| 157 | `png_image_read_direct` (pngread.c:3960) | `png_error` | `else if (do_local_compose != 0) /* internal error */` | "png_image_read: alpha channel lost" || n/a |
| 158 | `png_image_read_direct` (pngread.c:3986) | `png_error` | `if (do_local_background == 2)` | "unexpected alpha swap transformation" || n/a |
| 159 | `png_image_read_direct` (pngread.c:3994) | `png_error` | `if (info_format != format)` | "png_read_image: invalid transformations" || n/a |
| 160 | `png_read_data` (pngrio.c:39) | `png_error` | `if (png_ptr->read_data_fn != NULL) (*(png_ptr->read_data_fn))(png_ptr, data, length); else` | "Call to NULL read function" || [x] |
| 161 | `png_default_read_data` (pngrio.c:62) | `png_error` | `if (check != length)` | "Read Error" || [x] |
| 162 | `png_default_read_data` (pngrio.c:81) | `png_error` | `(unconditional at that point)` | "Error msg" || n/a |
| 163 | `png_set_read_fn` (pngrio.c:109) | `png_warning` | `if (png_ptr->write_data_fn != NULL) { png_ptr->write_data_fn = NULL;` | "Can't set both read_data_fn and write_data_fn in the" || [x] |
| 164 | `png_set_crc_action` (pngrtran.c:66) | `png_warning` | `case PNG_CRC_WARN_DISCARD: /* Not a valid action for critical data */` | "Can't discard critical data on CRC error" || [x] |
| 165 | `png_rtran_ok` (pngrtran.c:120) | `png_app_error` | `if ((png_ptr->flags & PNG_FLAG_ROW_INIT) != 0)` | "invalid after png_start_read_image or png_read_update_info" || [x] |
| 166 | `png_rtran_ok` (pngrtran.c:124) | `png_app_error` | `else if (need_IHDR && (png_ptr->mode & PNG_HAVE_IHDR) == 0)` | "invalid before the PNG header has been read" || [x] |
| 167 | `png_set_background_fixed` (pngrtran.c:153) | `png_warning` | `if (background_gamma_code == PNG_BACKGROUND_GAMMA_UNKNOWN) {` | "Application must supply a known background gamma" || [x] |
| 168 | `convert_gamma_value` (pngrtran.c:325) | `png_fixed_error` | `if (output_gamma > PNG_FP_MAX \|\| output_gamma < PNG_FP_MIN)` | "gamma value" || [x] |
| 169 | `unsupported_gamma` (pngrtran.c:348) | `png_app_warning` | `if (warn)` | "" || [x] |
| 170 | `unsupported_gamma` (pngrtran.c:350) | `png_app_error` | `if (warn) png_app_warning(png_ptr, msg); else` | "" || [x] |
| 171 | `png_set_alpha_mode_fixed` (pngrtran.c:434) | `png_error` | `default:` | "invalid alpha mode" || [x] |
| 172 | `png_set_alpha_mode_fixed` (pngrtran.c:452) | `png_error` | `if ((png_ptr->transformations & PNG_COMPOSE) != 0)` | "conflicting calls to set alpha mode and background" || [x] |
| 173 | `png_set_gamma_fixed` (pngrtran.c:917) | `png_app_error` | `if (file_gamma <= 0)` | "invalid file gamma in png_set_gamma" || [x] |
| 174 | `png_set_gamma_fixed` (pngrtran.c:919) | `png_app_error` | `if (scrn_gamma <= 0)` | "invalid screen gamma in png_set_gamma" || [x] |
| 175 | `png_set_rgb_to_gray_fixed` (pngrtran.c:1072) | `png_error` | `default:` | "invalid error action to rgb_to_gray" || [x] |
| 176 | `png_set_rgb_to_gray_fixed` (pngrtran.c:1083) | `png_error` | `if (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE) #ifdef PNG_READ_EXPAND_SUPPORTED png_ptr->transformations \|= PNG_EXPAND; #else {` | "Cannot do RGB_TO_GRAY without EXPAND_SUPPORTED" || n/a |
| 177 | `png_set_rgb_to_gray_fixed` (pngrtran.c:1108) | `png_app_warning` | `else if (red >= 0 && green >= 0)` | "ignoring out of range rgb_to_gray coefficients" || [x] |
| 178 | `png_init_read_transformations` (pngrtran.c:1697) | `png_warning` | `if ((png_ptr->transformations & PNG_RGB_TO_GRAY) != 0)` | "libpng does not support gamma+background+rgb_to_gray" || [x] |
| 179 | `png_init_read_transformations` (pngrtran.c:1886) | `png_error` | `default:` | "invalid background gamma type" || [x] |
| 180 | `png_read_transform_info` (pngrtran.c:2104) | `png_error` | `if (png_ptr->palette == NULL)` | "Palette is NULL in indexed image" || [x] |
| 181 | `png_do_encode_alpha` (pngrtran.c:4341) | `png_warning` | `(unconditional at that point)` | "png_do_encode_alpha: unexpected call" || n/a |
| 182 | `png_do_read_transformations` (pngrtran.c:4891) | `png_error` | `if (png_ptr->row_buf == NULL) { /* Prior to 1.5.4 this output row/pass where the NULL pointer is, but this * error is incredibly rare and incredibly easy to debug without this * information.` | "NULL row buffer" || [x] |
| 183 | `png_do_read_transformations` (pngrtran.c:4907) | `png_error` | `if ((png_ptr->flags & PNG_FLAG_DETECT_UNINITIALIZED) != 0 && (png_ptr->flags & PNG_FLAG_ROW_INIT) == 0) { /* Application has failed to call either png_read_start_image() or * png_read_update_info() af` | "Uninitialized row" || n/a |
| 184 | `png_do_read_transformations` (pngrtran.c:4965) | `png_warning` | `if ((png_ptr->transformations & PNG_RGB_TO_GRAY) == PNG_RGB_TO_GRAY_WARN)` | "png_do_rgb_to_gray found nongray pixel" || [x] |
| 185 | `png_do_read_transformations` (pngrtran.c:4969) | `png_error` | `if ((png_ptr->transformations & PNG_RGB_TO_GRAY) == PNG_RGB_TO_GRAY_ERR)` | "png_do_rgb_to_gray found nongray pixel" || [x] |
| 186 | `png_get_uint_31` (pngrutil.c:46) | `png_error` | `if (uval > PNG_UINT_31_MAX)` | "PNG unsigned integer out of range" || [x] |
| 187 | `png_read_sig` (pngrutil.c:139) | `png_error` | `if (num_checked < 4 && png_sig_cmp(info_ptr->signature, num_checked, num_to_check - 4) != 0)` | "Not a PNG file" || [x] |
| 188 | `png_read_sig` (pngrutil.c:141) | `png_error` | `if (num_checked < 4 && png_sig_cmp(info_ptr->signature, num_checked, num_to_check - 4) != 0) png_error(png_ptr, "Not a PNG file"); else` | "PNG file corrupted by ASCII conversion" || [x] |
| 189 | `png_read_chunk_header` (pngrutil.c:211) | `png_chunk_error` | `if (buf[0] >= 0x80U)` | "bad header (invalid length)" || n/a |
| 190 | `png_read_chunk_header` (pngrutil.c:215) | `png_chunk_error` | `if (!check_chunk_name(chunk_name))` | "bad header (invalid type)" || [x] |
| 191 | `png_crc_finish_critical` (pngrutil.c:348) | `png_chunk_warning` | `if (handle_as_ancillary \|\| PNG_CHUNK_ANCILLARY(png_ptr->chunk_name) != 0 ? (png_ptr->flags & PNG_FLAG_CRC_ANCILLARY_NOWARN) == 0 : (png_ptr->flags & PNG_FLAG_CRC_CRITICAL_USE) != 0)` | "CRC error" || [x] |
| 192 | `png_crc_finish_critical` (pngrutil.c:351) | `png_chunk_error` | `if (handle_as_ancillary \|\| PNG_CHUNK_ANCILLARY(png_ptr->chunk_name) != 0 ? (png_ptr->flags & PNG_FLAG_CRC_ANCILLARY_NOWARN) == 0 : (png_ptr->flags & PNG_FLAG_CRC_CRITICAL_USE) != 0) png_chunk_warning(` | "CRC error" || [x] |
| 193 | `png_inflate_claim` (pngrutil.c:427) | `png_chunk_warning` | `(unconditional at that point)` | "" || n/a |
| 194 | `png_inflate_claim` (pngrutil.c:430) | `png_chunk_error` | `(unconditional at that point)` | "" || [x] |
| 195 | `png_handle_PLTE` (pngrutil.c:1064) | `png_chunk_error` | `if (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE) { png_crc_finish(png_ptr, length);` | "" || [x] |
| 196 | `png_handle_iCCP` (pngrutil.c:1455) | `png_chunk_warning` | `if (length > 0) { /* This can be handled completely, so * keep going. */` | "extra compressed data" || [x] |
| 197 | `png_handle_sPLT` (pngrutil.c:1577) | `png_warning` | `if (--png_ptr->user_chunk_cache_max == 1) {` | "No space in chunk cache for sPLT" || [x] |
| 198 | `png_handle_sPLT` (pngrutil.c:1612) | `png_warning` | `if (length < 2U \|\| entry_start > buffer + (length - 2U)) {` | "malformed sPLT chunk" || [x] |
| 199 | `png_handle_sPLT` (pngrutil.c:1626) | `png_warning` | `if ((data_length % (unsigned int)entry_size) != 0) {` | "sPLT chunk has bad length" || [x] |
| 200 | `png_handle_sPLT` (pngrutil.c:1635) | `png_warning` | `if (dl > max_dl) {` | "sPLT chunk too long" || n/a |
| 201 | `png_handle_sPLT` (pngrutil.c:1646) | `png_warning` | `if (new_palette.entries == NULL) {` | "sPLT chunk requires too much memory" || [x] |
| 202 | `png_handle_unknown` (pngrutil.c:2812) | `png_chunk_error` | `if (ret < 0) /* handled_error */` | "error in user chunk" || [x] |
| 203 | `png_handle_unknown` (pngrutil.c:2832) | `png_chunk_warning` | `if (png_ptr->unknown_default < PNG_HANDLE_CHUNK_IF_SAFE) {` | "Saving unknown chunk:" || [x] |
| 204 | `png_handle_unknown` (pngrutil.c:2833) | `png_app_warning` | `if (png_ptr->unknown_default < PNG_HANDLE_CHUNK_IF_SAFE) { png_chunk_warning(png_ptr, "Saving unknown chunk:");` | "forcing save of an unhandled chunk;" || [x] |
| 205 | `png_handle_unknown` (pngrutil.c:2893) | `png_app_error` | `if (keep > PNG_HANDLE_CHUNK_NEVER)` | "no unknown chunk support available" || n/a |
| 206 | `png_handle_unknown` (pngrutil.c:2957) | `png_chunk_error` | `if (handled < handled_saved && PNG_CHUNK_CRITICAL(png_ptr->chunk_name))` | "unhandled critical chunk" || [x] |
| 207 | `png_handle_chunk` (pngrutil.c:3135) | `png_chunk_error` | `else if (chunk_index != PNG_INDEX_IHDR && (png_ptr->mode & PNG_HAVE_IHDR) == 0)` | "missing IHDR" || [x] |
| 208 | `png_handle_chunk` (pngrutil.c:3201) | `png_chunk_error` | `if (PNG_CHUNK_CRITICAL(chunk_name)) /* stop immediately */` | "" || [x] |
| 209 | `png_combine_row` (pngrutil.c:3243) | `png_error` | `if (pixel_depth == 0)` | "internal row logic error" || [x] |
| 210 | `png_combine_row` (pngrutil.c:3251) | `png_error` | `if (png_ptr->info_rowbytes != 0 && png_ptr->info_rowbytes != PNG_ROWBYTES(pixel_depth, row_width))` | "internal row size calculation error" || [x] |
| 211 | `png_combine_row` (pngrutil.c:3255) | `png_error` | `if (row_width == 0)` | "internal row width error" || n/a |
| 212 | `png_combine_row` (pngrutil.c:3478) | `png_error` | `if (pixel_depth & 7)` | "invalid user transform pixel depth" || [x] |
| 213 | `png_read_IDAT_data` (pngrutil.c:4201) | `png_error` | `if (png_ptr->chunk_name != png_IDAT)` | "Not enough image data" || [x] |
| 214 | `png_read_IDAT_data` (pngrutil.c:4222) | `png_chunk_error` | `if (buffer == NULL)` | "out of memory" || [x] |
| 215 | `png_read_IDAT_data` (pngrutil.c:4285) | `png_chunk_error` | `if (output != NULL)` | "" || [x] |
| 216 | `png_read_IDAT_data` (pngrutil.c:4301) | `png_error` | `if (output != NULL)` | "Not enough image data" || [x] |
| 217 | `defined` (pngrutil.c:4600) | `png_error` | `if (row_bytes > (png_uint_32)65536L)` | "This image requires a row greater than 64KB" || n/a |
| 218 | `defined` (pngrutil.c:4645) | `png_error` | `if (png_ptr->rowbytes > 65535)` | "This image requires a row greater than 64KB" || n/a |
| 219 | `defined` (pngrutil.c:4649) | `png_error` | `if (png_ptr->rowbytes > (PNG_SIZE_MAX - 1))` | "Row has too many bytes to allocate in memory" || n/a |
| 220 | `defined` (pngrutil.c:4680) | `png_error` | `if (png_inflate_claim(png_ptr, png_IDAT) != Z_OK)` | "" || [x] |
| 221 | `png_set_cHRM_XYZ_fixed` (pngset.c:94) | `png_app_error` | `if (png_xy_from_XYZ(&xy, &XYZ) == 0) { info_ptr->cHRM = xy; info_ptr->valid \|= PNG_INFO_cHRM; }` | "invalid cHRM XYZ" || [x] |
| 222 | `png_set_cICP` (pngset.c:152) | `png_warning` | `if (info_ptr->cicp_matrix_coefficients != 0) {` | "Invalid cICP matrix coefficients" || [x] |
| 223 | `png_set_cLLI_fixed` (pngset.c:182) | `png_chunk_report` | `if (maxCLL > 0x7FFFFFFFU \|\| maxFALL > 0x7FFFFFFFU) { /* The limit is 200kcd/m2; somewhat bright but not inconceivable because * human vision is said to run up to 100Mcd/m2. The sun is about 2Gcd/m2. *` | "cLLI light level exceeds PNG limit" || [x] |
| 224 | `png_set_mDCV_fixed` (pngset.c:254) | `png_chunk_report` | `if (error) {` | "mDCV chromaticities outside representable range" || [x] |
| 225 | `png_set_mDCV_fixed` (pngset.c:269) | `png_chunk_report` | `if (maxDL > 0x7FFFFFFFU \|\| minDL > 0x7FFFFFFFU) { /* The limit is 200kcd/m2; somewhat bright but not inconceivable because * human vision is said to run up to 100Mcd/m2. The sun is about 2Gcd/m2. *` | "mDCV display light level exceeds PNG limit" || [x] |
| 226 | `png_set_eXIf` (pngset.c:322) | `png_warning` | `(unconditional at that point)` | "png_set_eXIf does not work; use png_set_eXIf_1" || [x] |
| 227 | `png_set_eXIf_1` (pngset.c:344) | `png_warning` | `if (new_exif == NULL) {` | "Insufficient memory for eXIf chunk data" || [x] |
| 228 | `png_set_hIST` (pngset.c:399) | `png_warning` | `if (info_ptr->num_palette == 0 \|\| info_ptr->num_palette > PNG_MAX_PALETTE_LENGTH) {` | "Invalid palette size, hIST allocation skipped" || [x] |
| 229 | `png_set_hIST` (pngset.c:422) | `png_warning` | `if (info_ptr->hist == NULL) {` | "Insufficient memory for hIST chunk data" || [x] |
| 230 | `png_set_pCAL` (pngset.c:515) | `png_chunk_report` | `if (type < 0 \|\| type > 3) {` | "Invalid pCAL equation type" || [x] |
| 231 | `png_set_pCAL` (pngset.c:522) | `png_chunk_report` | `if (nparams < 0 \|\| nparams > 255) {` | "Invalid pCAL parameter count" || [x] |
| 232 | `png_set_pCAL` (pngset.c:533) | `png_chunk_report` | `if (params[i] == NULL \|\| !png_check_fp_string(params[i], strlen(params[i]))) {` | "Invalid format for pCAL parameter" || [x] |
| 233 | `png_set_pCAL` (pngset.c:544) | `png_chunk_report` | `if (info_ptr->pcal_purpose == NULL) {` | "Insufficient memory for pCAL purpose" || [x] |
| 234 | `png_set_pCAL` (pngset.c:568) | `png_warning` | `if (info_ptr->pcal_units == NULL) {` | "Insufficient memory for pCAL units" || [x] |
| 235 | `png_set_pCAL` (pngset.c:579) | `png_warning` | `if (info_ptr->pcal_params == NULL) {` | "Insufficient memory for pCAL params" || [x] |
| 236 | `png_set_pCAL` (pngset.c:596) | `png_warning` | `if (info_ptr->pcal_params[i] == NULL) {` | "Insufficient memory for pCAL parameter" || [x] |
| 237 | `png_set_sCAL_s` (pngset.c:623) | `png_error` | `if (unit != 1 && unit != 2)` | "Invalid sCAL unit" || [x] |
| 238 | `png_set_sCAL_s` (pngset.c:627) | `png_error` | `if (swidth == NULL \|\| (lengthw = strlen(swidth)) == 0 \|\| swidth[0] == 45 /* '-' */ \|\| !png_check_fp_string(swidth, lengthw))` | "Invalid sCAL width" || [x] |
| 239 | `png_set_sCAL_s` (pngset.c:631) | `png_error` | `if (sheight == NULL \|\| (lengthh = strlen(sheight)) == 0 \|\| sheight[0] == 45 /* '-' */ \|\| !png_check_fp_string(sheight, lengthh))` | "Invalid sCAL height" || [x] |
| 240 | `png_set_sCAL_s` (pngset.c:644) | `png_warning` | `if (info_ptr->scal_s_width == NULL) {` | "Memory allocation failed while processing sCAL" || [x] |
| 241 | `png_set_sCAL_s` (pngset.c:663) | `png_warning` | `if (info_ptr->scal_s_height == NULL) { png_free(png_ptr, info_ptr->scal_s_width); info_ptr->scal_s_width = NULL;` | "Memory allocation failed while processing sCAL" || [x] |
| 242 | `png_set_sCAL` (pngset.c:682) | `png_warning` | `if (width <= 0)` | "Invalid sCAL width ignored" || [x] |
| 243 | `png_set_sCAL` (pngset.c:685) | `png_warning` | `else if (height <= 0)` | "Invalid sCAL height ignored" || [x] |
| 244 | `png_set_sCAL_fixed` (pngset.c:712) | `png_warning` | `if (width <= 0)` | "Invalid sCAL width ignored" || [x] |
| 245 | `png_set_sCAL_fixed` (pngset.c:715) | `png_warning` | `else if (height <= 0)` | "Invalid sCAL height ignored" || [x] |
| 246 | `png_set_PLTE` (pngset.c:767) | `png_error` | `if (info_ptr->color_type == PNG_COLOR_TYPE_PALETTE)` | "Invalid palette length" || [x] |
| 247 | `png_set_PLTE` (pngset.c:771) | `png_warning` | `if (info_ptr->color_type == PNG_COLOR_TYPE_PALETTE) png_error(png_ptr, "Invalid palette length"); else {` | "Invalid palette length" || [x] |
| 248 | `png_set_PLTE` (pngset.c:784) | `png_error` | `if ((num_palette > 0 && palette == NULL) \|\| (num_palette == 0 # ifdef PNG_MNG_FEATURES_SUPPORTED && (png_ptr->mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0 # endif` | "Invalid palette" || [x] |
| 249 | `png_set_iCCP` (pngset.c:904) | `png_app_error` | `if (compression_type != PNG_COMPRESSION_TYPE_BASE)` | "Invalid iCCP compression method" || [x] |
| 250 | `png_set_iCCP` (pngset.c:911) | `png_benign_error` | `if (new_iccp_name == NULL) {` | "Insufficient memory to process iCCP chunk" || [x] |
| 251 | `png_set_iCCP` (pngset.c:923) | `png_benign_error` | `if (new_iccp_profile == NULL) { png_free(png_ptr, new_iccp_name);` | "Insufficient memory to process iCCP profile" || [x] |
| 252 | `png_set_text` (pngset.c:950) | `png_error` | `if (ret != 0)` | "Insufficient memory to store text" || [x] |
| 253 | `png_set_text_2` (pngset.c:1000) | `png_chunk_report` | `if (new_text == NULL) {` | "too many text chunks" || [x] |
| 254 | `png_set_text_2` (pngset.c:1031) | `png_chunk_report` | `if (text_ptr[i].compression < PNG_TEXT_COMPRESSION_NONE \|\| text_ptr[i].compression >= PNG_TEXT_COMPRESSION_LAST) {` | "text compression mode is out of range" || [x] |
| 255 | `png_set_text_2` (pngset.c:1063) | `png_chunk_report` | `if (text_ptr[i].lang_key != NULL) lang_key_len = strlen(text_ptr[i].lang_key); else lang_key_len = 0;` | "iTXt chunk not supported" || n/a |
| 256 | `png_set_text_2` (pngset.c:1092) | `png_chunk_report` | `if (textp->key == NULL) {` | "text chunk: out of memory" || [x] |
| 257 | `png_set_tIME` (pngset.c:1170) | `png_warning` | `mod_time->hour > 23 \|\| mod_time->minute > 59 \|\| mod_time->second > 60) {` | "Ignoring invalid time value" || [x] |
| 258 | `png_set_tRNS` (pngset.c:1253) | `png_warning` | `trans_color->green > sample_max \|\| trans_color->blue > sample_max)))` | "tRNS chunk has out-of-range samples for bit_depth" || [x] |
| 259 | `png_set_sPLT` (pngset.c:1305) | `png_chunk_report` | `if (np == NULL) { /* Out of memory or too many chunks */` | "too many sPLT chunks" || [x] |
| 260 | `png_set_sPLT` (pngset.c:1327) | `png_app_error` | `if (entries->name == NULL \|\| entries->entries == NULL) { /* png_handle_sPLT doesn't do this, so this is an app error */` | "png_set_sPLT: invalid sPLT" || [x] |
| 261 | `png_set_sPLT` (pngset.c:1347) | `png_chunk_report` | `if (np->name == NULL) break; memcpy(np->name, entries->name, length);` | "" || [x] |
| 262 | `png_set_sPLT` (pngset.c:1379) | `png_chunk_report` | `if (nentries > 0)` | "sPLT out of memory" || [x] |
| 263 | `check_location` (pngset.c:1396) | `png_app_warning` | `if (location == 0 && (png_ptr->mode & PNG_IS_READ_STRUCT) == 0) { /* Write struct, so unknown chunks come from the app */` | "png_set_unknown_chunks now expects a valid location" || [x] |
| 264 | `check_location` (pngset.c:1407) | `png_error` | `if (location == 0)` | "invalid location in png_set_unknown_chunks" || [x] |
| 265 | `png_set_unknown_chunks` (pngset.c:1442) | `png_app_error` | `if ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0) {` | "no unknown chunk support on read" || n/a |
| 266 | `png_set_unknown_chunks` (pngset.c:1451) | `png_app_error` | `if ((png_ptr->mode & PNG_IS_READ_STRUCT) == 0) {` | "no unknown chunk support on write" || n/a |
| 267 | `png_set_unknown_chunks` (pngset.c:1468) | `png_chunk_report` | `if (np == NULL) {` | "too many unknown chunks" || [x] |
| 268 | `png_set_unknown_chunks` (pngset.c:1505) | `png_chunk_report` | `if (np->data == NULL) {` | "unknown chunk: out of memory" || [x] |
| 269 | `png_set_unknown_chunk_location` (pngset.c:1540) | `png_app_error` | `if ((location & (PNG_HAVE_IHDR\|PNG_HAVE_PLTE\|PNG_AFTER_IDAT)) == 0) {` | "invalid unknown chunk location" || [x] |
| 270 | `png_set_keep_unknown_chunks` (pngset.c:1611) | `png_app_error` | `if (keep < 0 \|\| keep >= PNG_HANDLE_CHUNK_LAST) {` | "png_set_keep_unknown_chunks: invalid keep" || [x] |
| 271 | `png_set_keep_unknown_chunks` (pngset.c:1665) | `png_app_error` | `if (chunk_list == NULL) { /* Prior to 1.6.0 this was silently ignored, now it is an app_error * which can be switched off. */` | "png_set_keep_unknown_chunks: no chunk list" || [x] |
| 272 | `png_set_keep_unknown_chunks` (pngset.c:1681) | `png_app_error` | `if (num_chunks + old_num_chunks > UINT_MAX/5) {` | "png_set_keep_unknown_chunks: too many chunks" || [x] |
| 273 | `png_set_compression_buffer_size` (pngset.c:1805) | `png_error` | `if (size == 0 \|\| size > PNG_UINT_31_MAX)` | "invalid compression buffer size" || [x] |
| 274 | `png_set_compression_buffer_size` (pngset.c:1820) | `png_warning` | `if (png_ptr->zowner != 0) {` | "Compression buffer size cannot be changed because it is in use" || [x] |
| 275 | `png_set_compression_buffer_size` (pngset.c:1832) | `png_warning` | `if (size > ZLIB_IO_MAX) {` | "Compression buffer size limited to system maximum" || n/a |
| 276 | `png_set_compression_buffer_size` (pngset.c:1843) | `png_warning` | `if (size < 6) { /* Deflate will potentially go into an infinite loop on a SYNC_FLUSH * if this is permitted. */` | "Compression buffer size cannot be reduced below 6" || [x] |
| 277 | `png_set_benign_errors` (pngset.c:1930) | `png_benign_error` | `(unconditional at that point)` | "" || [x] |
| 278 | `png_set_benign_errors` (pngset.c:1932) | `png_benign_error` | `(unconditional at that point)` | "" || [x] |
| 279 | `png_check_keyword` (pngset.c:2039) | `png_warning` | `if (*key != 0) /* keyword too long */` | "keyword truncated" || [x] |
| 280 | `png_set_shift` (pngtrans.c:114) | `png_app_error` | `if (invalid) {` | "png_set_shift: invalid shift values" || [x] |
| 281 | `png_set_filler` (pngtrans.c:171) | `png_app_error` | `(unconditional at that point)` | "png_set_filler not supported on read" || n/a |
| 282 | `png_set_filler` (pngtrans.c:202) | `png_app_error` | `(unconditional at that point)` | "png_set_filler is invalid for" || [x] |
| 283 | `png_set_filler` (pngtrans.c:209) | `png_app_error` | `default:` | "png_set_filler: inappropriate color type" || [x] |
| 284 | `png_set_filler` (pngtrans.c:214) | `png_app_error` | `default: png_app_error(png_ptr, "png_set_filler: inappropriate color type"); return; }` | "png_set_filler not supported on write" || n/a |
| 285 | `png_set_user_transform_info` (pngtrans.c:845) | `png_app_error` | `if ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0 && (png_ptr->flags & PNG_FLAG_ROW_INIT) != 0) {` | "info change after png_start_read_image or png_read_update_info" || [x] |
| 286 | `png_write_data` (pngwio.c:40) | `png_error` | `if (png_ptr->write_data_fn != NULL ) (*(png_ptr->write_data_fn))(png_ptr, png_constcast(png_bytep,data), length); else` | "Call to NULL write function" || [x] |
| 287 | `png_default_write_data` (pngwio.c:60) | `png_error` | `if (check != length)` | "Write Error" || [x] |
| 288 | `png_default_flush` (pngwio.c:102) | `png_error` | `(unconditional at that point)` | "Error msg" || n/a |
| 289 | `png_set_write_fn` (pngwio.c:161) | `png_warning` | `if (png_ptr->read_data_fn != NULL) { png_ptr->read_data_fn = NULL;` | "Can't set both read_data_fn and write_data_fn in the" || [x] |
| 290 | `write_unknown_chunks` (pngwrite.c:64) | `png_warning` | `if (up->size == 0)` | "Writing zero-length unknown chunk" || [x] |
| 291 | `png_write_info_before_PLTE` (pngwrite.c:99) | `png_warning` | `if ((png_ptr->mode & PNG_HAVE_PNG_SIGNATURE) != 0 && \ png_ptr->mng_features_permitted != 0) {` | "MNG features are not allowed in a PNG datastream" || [x] |
| 292 | `png_write_info` (pngwrite.c:241) | `png_error` | `else if (info_ptr->color_type == PNG_COLOR_TYPE_PALETTE)` | "Valid palette required for paletted images" || [x] |
| 293 | `png_write_info` (pngwrite.c:346) | `png_warning` | `if (info_ptr->text[i].compression == PNG_TEXT_COMPRESSION_NONE) info_ptr->text[i].compression = PNG_TEXT_COMPRESSION_NONE_WR; else info_ptr->text[i].compression = PNG_TEXT_COMPRESSION_zTXt_WR; #else` | "Unable to write international text" || n/a |
| 294 | `png_write_info` (pngwrite.c:360) | `png_warning` | `(unconditional at that point)` | "Unable to write compressed text" || n/a |
| 295 | `png_write_info` (pngwrite.c:375) | `png_warning` | `(unconditional at that point)` | "Unable to write uncompressed text" || n/a |
| 296 | `png_write_end` (pngwrite.c:400) | `png_error` | `if ((png_ptr->mode & PNG_HAVE_IDAT) == 0)` | "No IDATs written into file" || [x] |
| 297 | `png_write_end` (pngwrite.c:405) | `png_benign_error` | `if (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && png_ptr->num_palette_max >= png_ptr->num_palette)` | "Wrote palette index exceeding num_palette" || [x] |
| 298 | `png_write_end` (pngwrite.c:444) | `png_warning` | `if (info_ptr->text[i].compression == PNG_TEXT_COMPRESSION_NONE) info_ptr->text[i].compression = PNG_TEXT_COMPRESSION_NONE_WR; else info_ptr->text[i].compression = PNG_TEXT_COMPRESSION_zTXt_WR; #else` | "Unable to write international text" || n/a |
| 299 | `png_write_end` (pngwrite.c:457) | `png_warning` | `(unconditional at that point)` | "Unable to write compressed text" || n/a |
| 300 | `png_write_end` (pngwrite.c:470) | `png_warning` | `(unconditional at that point)` | "Unable to write uncompressed text" || n/a |
| 301 | `png_write_row` (pngwrite.c:762) | `png_error` | `if ((png_ptr->mode & PNG_WROTE_INFO_BEFORE_PLTE) == 0)` | "png_write_info was never called before png_write_row" || [x] |
| 302 | `png_write_row` (pngwrite.c:768) | `png_warning` | `if ((png_ptr->transformations & PNG_INVERT_MONO) != 0)` | "PNG_WRITE_INVERT_SUPPORTED is not defined" || n/a |
| 303 | `png_write_row` (pngwrite.c:773) | `png_warning` | `if ((png_ptr->transformations & PNG_FILLER) != 0)` | "PNG_WRITE_FILLER_SUPPORTED is not defined" || n/a |
| 304 | `png_write_row` (pngwrite.c:778) | `png_warning` | `if ((png_ptr->transformations & PNG_PACKSWAP) != 0)` | "PNG_WRITE_PACKSWAP_SUPPORTED is not defined" || n/a |
| 305 | `png_write_row` (pngwrite.c:784) | `png_warning` | `if ((png_ptr->transformations & PNG_PACK) != 0)` | "PNG_WRITE_PACK_SUPPORTED is not defined" || n/a |
| 306 | `png_write_row` (pngwrite.c:789) | `png_warning` | `if ((png_ptr->transformations & PNG_SHIFT) != 0)` | "PNG_WRITE_SHIFT_SUPPORTED is not defined" || n/a |
| 307 | `png_write_row` (pngwrite.c:794) | `png_warning` | `if ((png_ptr->transformations & PNG_BGR) != 0)` | "PNG_WRITE_BGR_SUPPORTED is not defined" || n/a |
| 308 | `png_write_row` (pngwrite.c:799) | `png_warning` | `if ((png_ptr->transformations & PNG_SWAP_BYTES) != 0)` | "PNG_WRITE_SWAP_SUPPORTED is not defined" || n/a |
| 309 | `png_write_row` (pngwrite.c:918) | `png_error` | `if (row_info.pixel_depth != png_ptr->pixel_depth \|\| row_info.pixel_depth != png_ptr->transformed_pixel_depth)` | "internal write transform logic error" || [x] |
| 310 | `png_set_filter` (pngwrite.c:1078) | `png_app_error` | `case 6:` | "Unknown row filter for method 0" || [x] |
| 311 | `png_set_filter` (pngwrite.c:1101) | `png_app_error` | `default:` | "Unknown row filter for method 0" || n/a |
| 312 | `png_set_filter` (pngwrite.c:1140) | `png_app_warning` | `if ((filters & (PNG_FILTER_UP\|PNG_FILTER_AVG\|PNG_FILTER_PAETH)) != 0 && png_ptr->prev_row == NULL) { /* This is the error case, however it is benign - the previous row * is not available so the filter` | "png_set_filter: UP/AVG/PAETH cannot be added after start" || [x] |
| 313 | `png_set_filter` (pngwrite.c:1180) | `png_error` | `(unconditional at that point)` | "Unknown custom filter method" || [x] |
| 314 | `png_set_compression_window_bits` (pngwrite.c:1270) | `png_warning` | `if (window_bits > 15) {` | "Only compression windows <= 32k supported by PNG" || [x] |
| 315 | `png_set_compression_window_bits` (pngwrite.c:1276) | `png_warning` | `else if (window_bits < 8) {` | "Only compression windows >= 256 supported by PNG" || [x] |
| 316 | `png_set_compression_method` (pngwrite.c:1295) | `png_warning` | `if (method != 8)` | "Only compression method 8 is supported by PNG" || [x] |
| 317 | `png_set_text_compression_window_bits` (pngwrite.c:1349) | `png_warning` | `if (window_bits > 15) {` | "Only compression windows <= 32k supported by PNG" || [x] |
| 318 | `png_set_text_compression_window_bits` (pngwrite.c:1355) | `png_warning` | `else if (window_bits < 8) {` | "Only compression windows >= 256 supported by PNG" || [x] |
| 319 | `png_set_text_compression_method` (pngwrite.c:1371) | `png_warning` | `if (method != 8)` | "Only compression method 8 is supported by PNG" || [x] |
| 320 | `png_write_png` (pngwrite.c:1417) | `png_app_error` | `if ((info_ptr->valid & PNG_INFO_IDAT) == 0) {` | "no rows for png_write_image to write" || [x] |
| 321 | `png_write_png` (pngwrite.c:1431) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_INVERT_MONO) != 0) #ifdef PNG_WRITE_INVERT_SUPPORTED png_set_invert_mono(png_ptr); #else` | "PNG_TRANSFORM_INVERT_MONO not supported" || n/a |
| 322 | `png_write_png` (pngwrite.c:1442) | `png_app_error` | `if ((info_ptr->valid & PNG_INFO_sBIT) != 0) png_set_shift(png_ptr, &info_ptr->sig_bit); #else` | "PNG_TRANSFORM_SHIFT not supported" || n/a |
| 323 | `png_write_png` (pngwrite.c:1450) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_PACKING) != 0) #ifdef PNG_WRITE_PACK_SUPPORTED png_set_packing(png_ptr); #else` | "PNG_TRANSFORM_PACKING not supported" || n/a |
| 324 | `png_write_png` (pngwrite.c:1458) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0) #ifdef PNG_WRITE_SWAP_ALPHA_SUPPORTED png_set_swap_alpha(png_ptr); #else` | "PNG_TRANSFORM_SWAP_ALPHA not supported" || n/a |
| 325 | `png_write_png` (pngwrite.c:1472) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0)` | "PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported" || [x] |
| 326 | `png_write_png` (pngwrite.c:1482) | `png_app_error` | `else if ((transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0) png_set_filler(png_ptr, 0, PNG_FILLER_BEFORE); #else` | "PNG_TRANSFORM_STRIP_FILLER not supported" || n/a |
| 327 | `png_write_png` (pngwrite.c:1491) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_BGR) != 0) #ifdef PNG_WRITE_BGR_SUPPORTED png_set_bgr(png_ptr); #else` | "PNG_TRANSFORM_BGR not supported" || n/a |
| 328 | `png_write_png` (pngwrite.c:1499) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0) #ifdef PNG_WRITE_SWAP_SUPPORTED png_set_swap(png_ptr); #else` | "PNG_TRANSFORM_SWAP_ENDIAN not supported" || n/a |
| 329 | `png_write_png` (pngwrite.c:1507) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_PACKSWAP) != 0) #ifdef PNG_WRITE_PACKSWAP_SUPPORTED png_set_packswap(png_ptr); #else` | "PNG_TRANSFORM_PACKSWAP not supported" || n/a |
| 330 | `png_write_png` (pngwrite.c:1515) | `png_app_error` | `if ((transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0) #ifdef PNG_WRITE_INVERT_ALPHA_SUPPORTED png_set_invert_alpha(png_ptr); #else` | "PNG_TRANSFORM_INVERT_ALPHA not supported" || n/a |
| 331 | `png_write_image_16bit` (pngwrite.c:1629) | `png_error` | `(unconditional at that point)` | "png_write_image: internal call error" || n/a |
| 332 | `png_image_write_main` (pngwrite.c:2045) | `png_error` | `if (image->height > 0xffffffffU/png_row_stride)` | "memory image too large" || [x] |
| 333 | `png_image_write_main` (pngwrite.c:2049) | `png_error` | `if (image->height > 0xffffffffU/png_row_stride) png_error(image->opaque->png_ptr, "memory image too large"); } else` | "supplied row stride too small" || [x] |
| 334 | `png_image_write_main` (pngwrite.c:2053) | `png_error` | `(unconditional at that point)` | "image row stride too large" || [x] |
| 335 | `png_image_write_main` (pngwrite.c:2072) | `png_error` | `(unconditional at that point)` | "no color-map for color-mapped image" || [x] |
| 336 | `png_image_write_main` (pngwrite.c:2156) | `png_error` | `if ((format & ~(png_uint_32)(PNG_FORMAT_FLAG_COLOR \| PNG_FORMAT_FLAG_LINEAR \| PNG_FORMAT_FLAG_ALPHA \| PNG_FORMAT_FLAG_COLORMAP)) != 0)` | "png_write_image: unsupported transformation" || [x] |
| 337 | `void` (pngwrite.c:2253) | `png_error` | `(unconditional at that point)` | "png_image_write_to_memory: PNG too big" || n/a |
| 338 | `png_write_complete_chunk` (pngwutil.c:200) | `png_error` | `if (length > PNG_UINT_31_MAX)` | "length exceeds PNG maximum" || [x] |
| 339 | `png_deflate_claim` (pngwutil.c:328) | `png_warning` | `(unconditional at that point)` | "" || n/a |
| 340 | `png_deflate_claim` (pngwutil.c:339) | `png_error` | `if (png_ptr->zowner == png_IDAT) /* don't steal from IDAT */ { png_ptr->zstream.msg = PNGZ_MSG_CAST("in use by IDAT"); return Z_STREAM_ERROR; }` | "" || [x] |
| 341 | `png_deflate_claim` (pngwutil.c:413) | `png_warning` | `if (deflateEnd(&png_ptr->zstream) != Z_OK)` | "deflateEnd failed (ignored)" || n/a |
| 342 | `png_write_compressed_data_out` (pngwutil.c:680) | `png_error` | `if (output_len > 0)` | "error writing ancillary chunked compressed data" || [x] |
| 343 | `png_write_IHDR` (pngwutil.c:714) | `png_error` | `default:` | "Invalid bit depth for grayscale image" || [x] |
| 344 | `png_write_IHDR` (pngwutil.c:725) | `png_error` | `if (is_invalid_depth)` | "Invalid bit depth for RGB image" || [x] |
| 345 | `png_write_IHDR` (pngwutil.c:741) | `png_error` | `default:` | "Invalid bit depth for paletted image" || [x] |
| 346 | `png_write_IHDR` (pngwutil.c:751) | `png_error` | `if (is_invalid_depth)` | "Invalid bit depth for grayscale+alpha image" || [x] |
| 347 | `png_write_IHDR` (pngwutil.c:762) | `png_error` | `if (is_invalid_depth)` | "Invalid bit depth for RGBA image" || [x] |
| 348 | `png_write_IHDR` (pngwutil.c:768) | `png_error` | `default:` | "Invalid image color type specified" || [x] |
| 349 | `png_write_IHDR` (pngwutil.c:773) | `png_warning` | `if (compression_type != PNG_COMPRESSION_TYPE_BASE) {` | "Invalid compression type specified" || [x] |
| 350 | `png_write_IHDR` (pngwutil.c:796) | `png_warning` | `(filter_type == PNG_INTRAPIXEL_DIFFERENCING)) && #endif filter_type != PNG_FILTER_TYPE_BASE) {` | "Invalid filter type specified" || [x] |
| 351 | `png_write_IHDR` (pngwutil.c:804) | `png_warning` | `if (interlace_type != PNG_INTERLACE_NONE && interlace_type != PNG_INTERLACE_ADAM7) {` | "Invalid interlace type specified" || [x] |
| 352 | `png_write_PLTE` (pngwutil.c:879) | `png_error` | `if (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE) {` | "Invalid number of colors in palette" || [x] |
| 353 | `png_write_PLTE` (pngwutil.c:884) | `png_warning` | `if (png_ptr->color_type == PNG_COLOR_TYPE_PALETTE) { png_error(png_ptr, "Invalid number of colors in palette"); }` | "Invalid number of colors in palette" || [x] |
| 354 | `png_write_PLTE` (pngwutil.c:891) | `png_warning` | `if ((png_ptr->color_type & PNG_COLOR_MASK_COLOR) == 0) {` | "Ignoring request to write a PLTE chunk in grayscale PNG" || [x] |
| 355 | `png_compress_IDAT` (pngwutil.c:954) | `png_error` | `if (png_deflate_claim(png_ptr, png_IDAT, png_image_size(png_ptr)) != Z_OK)` | "" || [x] |
| 356 | `png_compress_IDAT` (pngwutil.c:1033) | `png_error` | `if (flush == Z_FINISH)` | "Z_OK on Z_FINISH with output space" || n/a |
| 357 | `png_compress_IDAT` (pngwutil.c:1067) | `png_error` | `(unconditional at that point)` | "" || [x] |
| 358 | `png_write_sRGB` (pngwutil.c:1107) | `png_warning` | `if (srgb_intent >= PNG_sRGB_INTENT_LAST)` | "Invalid sRGB rendering intent specified" || [x] |
| 359 | `png_write_iCCP` (pngwutil.c:1132) | `png_error` | `if (profile == NULL)` | "No profile for iCCP chunk" || [x] |
| 360 | `png_write_iCCP` (pngwutil.c:1135) | `png_error` | `if (profile_len < 132)` | "ICC profile too short" || [x] |
| 361 | `png_write_iCCP` (pngwutil.c:1138) | `png_error` | `if (png_get_uint_32(profile) != profile_len)` | "Incorrect data in iCCP" || [x] |
| 362 | `png_write_iCCP` (pngwutil.c:1142) | `png_error` | `if (temp > 3 && (profile_len & 0x03))` | "ICC profile length invalid (not a multiple of 4)" || [x] |
| 363 | `png_write_iCCP` (pngwutil.c:1148) | `png_error` | `if (profile_len != embedded_profile_len)` | "Profile length does not match profile" || n/a |
| 364 | `png_write_iCCP` (pngwutil.c:1154) | `png_error` | `if (name_len == 0)` | "iCCP: invalid keyword" || [x] |
| 365 | `png_write_iCCP` (pngwutil.c:1165) | `png_error` | `if (png_text_compress(png_ptr, png_iCCP, &comp, name_len) != Z_OK)` | "" || [x] |
| 366 | `png_write_sPLT` (pngwutil.c:1194) | `png_error` | `if (name_len == 0)` | "sPLT: invalid keyword" || [x] |
| 367 | `png_write_sBIT` (pngwutil.c:1254) | `png_warning` | `sbit->green == 0 \|\| sbit->green > maxbits \|\| sbit->blue == 0 \|\| sbit->blue > maxbits) {` | "Invalid sBIT depth specified" || [x] |
| 368 | `png_write_sBIT` (pngwutil.c:1268) | `png_warning` | `if (sbit->gray == 0 \|\| sbit->gray > png_ptr->usr_bit_depth) {` | "Invalid sBIT depth specified" || [x] |
| 369 | `png_write_sBIT` (pngwutil.c:1280) | `png_warning` | `if (sbit->alpha == 0 \|\| sbit->alpha > png_ptr->usr_bit_depth) {` | "Invalid sBIT depth specified" || [x] |
| 370 | `png_write_tRNS` (pngwutil.c:1331) | `png_app_warning` | `if (num_trans <= 0 \|\| num_trans > (int)png_ptr->num_palette) {` | "Invalid number of transparent colors specified" || [x] |
| 371 | `png_write_tRNS` (pngwutil.c:1346) | `png_app_warning` | `if (tran->gray >= (1 << png_ptr->bit_depth)) {` | "Ignoring attempt to write tRNS chunk out-of-range for bit_depth" || [x] |
| 372 | `png_write_tRNS` (pngwutil.c:1368) | `png_app_warning` | `if ((buf[0] \| buf[2] \| buf[4]) != 0) #endif {` | "Ignoring attempt to write 16-bit tRNS chunk when bit_depth is 8" || [x] |
| 373 | `png_write_tRNS` (pngwutil.c:1378) | `png_app_warning` | `(unconditional at that point)` | "Can't write tRNS with an alpha channel" || [x] |
| 374 | `png_write_bKGD` (pngwutil.c:1401) | `png_warning` | `(png_ptr->mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0) && #endif back->index >= png_ptr->num_palette) {` | "Invalid background palette index" || [x] |
| 375 | `png_write_bKGD` (pngwutil.c:1420) | `png_warning` | `if ((buf[0] \| buf[2] \| buf[4]) != 0) #endif {` | "Ignoring attempt to write 16-bit bKGD chunk " || [x] |
| 376 | `png_write_bKGD` (pngwutil.c:1434) | `png_warning` | `if (back->gray >= (1 << png_ptr->bit_depth)) {` | "Ignoring attempt to write bKGD chunk out-of-range for bit_depth" || [x] |
| 377 | `png_write_hIST` (pngwutil.c:1550) | `png_warning` | `if (num_hist > (int)png_ptr->num_palette) { png_debug2(3, "num_hist = %d, num_palette = %d", num_hist, png_ptr->num_palette);` | "Invalid number of histogram entries specified" || [x] |
| 378 | `png_write_tEXt` (pngwutil.c:1580) | `png_error` | `if (key_len == 0)` | "tEXt: invalid keyword" || [x] |
| 379 | `png_write_tEXt` (pngwutil.c:1589) | `png_error` | `if (text_len > PNG_UINT_31_MAX - (key_len+1))` | "tEXt: text too long" || n/a |
| 380 | `png_write_zTXt` (pngwutil.c:1628) | `png_error` | `if (compression != PNG_TEXT_COMPRESSION_zTXt)` | "zTXt: invalid compression type" || [x] |
| 381 | `png_write_zTXt` (pngwutil.c:1633) | `png_error` | `if (key_len == 0)` | "zTXt: invalid keyword" || [x] |
| 382 | `png_write_zTXt` (pngwutil.c:1644) | `png_error` | `if (png_text_compress(png_ptr, png_zTXt, &comp, key_len) != Z_OK)` | "" || [x] |
| 383 | `png_write_iTXt` (pngwutil.c:1676) | `png_error` | `if (key_len == 0)` | "iTXt: invalid keyword" || [x] |
| 384 | `png_write_iTXt` (pngwutil.c:1692) | `png_error` | `default:` | "iTXt: invalid compression" || [x] |
| 385 | `png_write_iTXt` (pngwutil.c:1730) | `png_error` | `if (png_text_compress(png_ptr, png_iTXt, &comp, prefix_len) != Z_OK)` | "" || [x] |
| 386 | `png_write_iTXt` (pngwutil.c:1736) | `png_error` | `if (comp.input_len > PNG_UINT_31_MAX-prefix_len)` | "iTXt: uncompressed text too long" || n/a |
| 387 | `png_write_oFFs` (pngwutil.c:1771) | `png_warning` | `if (unit_type >= PNG_OFFSET_LAST)` | "Unrecognized unit type for oFFs chunk" || [x] |
| 388 | `png_write_pCAL` (pngwutil.c:1797) | `png_error` | `if (type >= PNG_EQUATION_LAST)` | "Unrecognized equation type for pCAL chunk" || [x] |
| 389 | `png_write_pCAL` (pngwutil.c:1802) | `png_error` | `if (purpose_len == 0)` | "pCAL: invalid keyword" || [x] |
| 390 | `png_write_sCAL_s` (pngwutil.c:1862) | `png_warning` | `if (total_len > 64) {` | "Can't write sCAL (buffer too small)" || [x] |
| 391 | `png_write_pHYs` (pngwutil.c:1887) | `png_warning` | `if (unit_type >= PNG_RESOLUTION_LAST)` | "Unrecognized unit type for pHYs chunk" || [x] |
| 392 | `png_write_tIME` (pngwutil.c:1912) | `png_warning` | `mod_time->day > 31 \|\| mod_time->day < 1 \|\| mod_time->hour > 23 \|\| mod_time->second > 60) {` | "Invalid time specified for tIME chunk" || [x] |

## Sentinel-return rejections (no message emitted)

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| S1 | `png_create_read_struct_2` / `png_create_write_struct_2` | `user_png_ver` mismatch (major/minor differs, or ver string differs) | `png_warning "Application built with libpng-X.Y.Z but running with A.B.C"` (warn), then returns `NULL` if major/minor mismatch | [x] |
| S2 | `png_create_read_struct_2` | `png_struct_size`/`png_info_size` args ignored in 1.6 (compat) | still returns valid ptr | [x] |
| S3 | `png_get_*` (all of `pngget.c`, 52 sites) | `png_ptr == NULL` or `info_ptr == NULL` or requested `valid` bit clear | returns `0` / `NULL` and sets no output | [x] |
| S4 | `png_get_valid` | bit not set in `info_ptr->valid` | `0` | [x] |
| S5 | `png_get_rowbytes` / `png_get_image_width/height` etc. | NULL ptr | `0` | [x] |
| S6 | `png_get_io_ptr` / `png_get_error_ptr` / `png_get_progressive_ptr` / `png_get_user_chunk_ptr` / `png_get_mem_ptr` | NULL `png_ptr` | `NULL` | [x] |
| S7 | `png_malloc_warn` / `png_malloc_default` | allocation of 0 bytes or > `PNG_SIZE_MAX` | `NULL` (and `png_malloc` -> `png_error "Out of memory"`) | [x] |
| S8 | `png_image_begin_read_from_memory` | `image == NULL` | `0`, nothing written | [x] |
| S9 | `png_image_begin_read_from_memory` | `image->version != PNG_IMAGE_VERSION` | `0`, `image->message = "png_image version mismatch"`, `warning_or_error = 1` (via `png_image_error`) | [x] |
| S10 | `png_image_begin_read_from_memory` | `memory == NULL \|\| size == 0` | `0`, `message = "png_image_begin_read_from_memory: invalid argument"` | [x] |
| S11 | `png_image_finish_read` | `image->opaque == NULL` / `opaque->png_ptr == NULL` | `0`, `message = "png_image_finish_read: no in-progress read"` | [x] |
| S12 | `png_image_finish_read` | `buffer == NULL` | `0`, `message = "png_image_finish_read: invalid argument"` | [x] |
| S13 | `png_image_finish_read` | `row_stride` too small in magnitude for the format | `0`, `message = "png_image_finish_read: invalid argument"` | [x] |
| S14 | `png_image_finish_read` | `colormap == NULL` while format has `PNG_FORMAT_FLAG_COLORMAP` | `0`, `message = "png_image_finish_read: no color-map for color-mapped image"` | [x] |
| S15 | `png_image_write_to_memory` / `..._to_stdio` | `image == NULL` / version mismatch / `buffer == NULL` while non-zero size | `0` + corresponding `message` | [x] |
| S16 | `png_image_write_to_memory` | output does not fit in `*memory_bytes` | `0`, `message = "png_image_write_to_memory: PNG too big"` | [x] |
| S17 | `png_sig_cmp` | `sig` differs from the 8-byte PNG signature in `[start, start+num_to_check)` | non-zero (`memcmp` result); `num_to_check == 0` or `start > 7` -> `-1` | [x] |
| S18 | `png_access_version_number` | — | always `10659` | [x] |
| S19 | `png_set_longjmp_fn` | `jmp_buf_size != sizeof(jmp_buf)` | `png_warning "Application jmp_buf size changed"` + returns `NULL` | [x] |
| S20 | `png_set_longjmp_fn` | `png_ptr == NULL` or `longjmp_fn == NULL` | `NULL` | [x] |
| S21 | `png_process_data_pause`/`png_process_data_skip` | nothing buffered / no skip pending | `0` | [x] |
| S22 | `png_get_uint_31` | value `> PNG_UINT_31_MAX` | `png_error "PNG unsigned integer out of range"` | [x] |
| S23 | `png_check_sig`-style entry `png_read_sig` | first 8 bytes not the signature | `png_error "Not a PNG file"` or `"PNG file corrupted by ASCII conversion"` | [x] |
| S24 | `png_reset_zstream` | `png_ptr == NULL` or zstream not claimed | `Z_STREAM_ERROR` (`-2`) | [x] |
| S25 | out-of-range enum value across FFI | e.g. `png_set_filter(pp, 0, 0x1234)`, `png_set_crc_action(pp, 99, 99)`, `png_set_alpha_mode(pp, 99, 1.0)`, `png_set_unknown_chunks(..., location=99)`, `png_set_compression_strategy(pp, 99)`, `png_set_gAMA` with negative, `png_data_freer(freer=99)`, `png_set_text(compression=99)`, `png_set_sCAL(unit=99)`, `png_set_filter_heuristics`-style ints | the C code's own `default:`/`else` branch (usually `png_error`/`png_app_error` or silently ignored) — must match exactly | [x] |

## Legend for the "test" column

* `[x]` — a differential test constructs that exact invalid input/condition, calls BOTH
  `.so`s and asserts the SAME error/rejection (same message bytes, same order, same
  return value / sentinel). **311 rows.**
* `n/a` — the site is **not reachable** in this build configuration and is therefore not
  reachable in the Rust translation either. **106 rows.** Every one falls into one of
  these mechanically checkable categories (the exact reason is recorded next to the
  covering test in the corresponding `tests/t_err*.rs` file):
  1. **`#else` / `#ifndef` arm of a feature that `pnglibconf.h` DOES define** — e.g. the
     "…not supported" `png_error`s behind `#ifndef PNG_READ_INVERT_SUPPORTED`,
     `#ifndef PNG_WRITE_FILLER_SUPPORTED`, `#ifndef PNG_iTXt_SUPPORTED`,
     `#ifndef PNG_SIMPLIFIED_READ_SUPPORTED`, … The translation correctly omits them
     (`grep` confirms the message strings do not exist in `translation/src/`).
  2. **`#if PNG_RELEASE_BUILD` arms** — `PNG_LIBPNG_BUILD_BASE_TYPE` is `BETA`, so
     `PNG_RELEASE_BUILD == 0` and the *other* arm (which IS tested) is compiled.
  3. **`#ifdef PNG_MAX_MALLOC_64K` / `PNG_SMALL_SIZE_T` arms** — not defined.
  4. **Arithmetically dead on a 64-bit target** — guards of the form
     `items >= SIZE_MAX/size` with 32-bit `uInt` operands, `rowbytes > PNG_SIZE_MAX-1`,
     `size > ZLIB_IO_MAX` after an earlier `> PNG_UINT_31_MAX` rejection,
     `length > SIZE_MAX/10`, "width too large for this architecture", etc.
  5. **Guarded by an earlier, stricter check on the same path** — e.g.
     `"bad header (invalid length)"` can never fire because `png_get_uint_31` rejects
     the same value first; `png_write_iCCP`'s duplicate `profile_len` test.
  6. **Documented internal invariants** ("internal error", "logic error") that no
     sequence of exported calls can violate — enumerated case by case in `t_err3.rs`
     (`png_image_read_colormap` / `_background` / `_direct`) and `t_err5.rs`.
  7. **`static` in C and exported by neither `.so`** (`png_do_encode_alpha`) — it cannot
     be driven differentially at all.
  8. **Process-fatal by design** — `png_chunk_error(NULL, …)` reaches `PNG_ABORT()`;
     calling it would kill the test process, so it is deliberately not invoked.

Additionally, three argument ranges named in the generic-boundary sweep are
**undefined behaviour in the reference C** (no NULL guard / no range validation, so the
C itself segfaults or reads uninitialised memory): `png_set_read_user_transform_fn(NULL,…)`,
`png_set_benign_errors(NULL,…)`, `png_set_quantize` with negative / >256 counts, and
`png_write_row(pp, NULL)`. These are documented in the test files instead of asserted,
because "both libraries crash identically" is not a meaningful assertion.
