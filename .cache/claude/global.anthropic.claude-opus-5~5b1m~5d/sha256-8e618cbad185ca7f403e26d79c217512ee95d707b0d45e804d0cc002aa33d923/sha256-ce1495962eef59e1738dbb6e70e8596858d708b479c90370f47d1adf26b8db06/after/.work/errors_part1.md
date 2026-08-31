| | `png_set_sig_bytes` | `png_ptr == NULL` (`png.c:59`) | silent `return`, no state change |
| | `png_set_sig_bytes` | `num_bytes < 0` (`png.c:62`) | silently clamped: `nb = 0`, `png_ptr->sig_bytes = 0` |
| | `png_set_sig_bytes` | `nb > 8` i.e. more than 8 already-consumed signature bytes (`png.c:65`) | `png_error(png_ptr, "Too many bytes for PNG signature")` -> error, no return |
| | `png_sig_cmp` | `num_to_check > 8` (`png.c:84`) | silently clamped to `num_to_check = 8` |
| | `png_sig_cmp` | `num_to_check < 1` (`png.c:87`) | `return -1` (reject) |
| | `png_sig_cmp` | `start > 7` (`png.c:90`) | `return -1` (reject) |
| | `png_sig_cmp` | `start + num_to_check > 8` (`png.c:93`) | silently clamped: `num_to_check = 8 - start` |
| | `png_sig_cmp` | `memcmp(&sig[start], &png_signature[start], num_to_check) != 0` (`png.c:96`) | returns non-zero (signature mismatch) |
| | `png_zalloc` | `png_ptr == NULL` (`png.c:109`) | `return NULL` |
| | `png_zalloc` | `size != 0 && items >= (~(png_alloc_size_t)0) / size` — `items*size` overflows `png_alloc_size_t` (`png.c:118`) | `png_warning(png_ptr, "Potential overflow in png_zalloc()")` then `return NULL` |
| | `png_calculate_crc` | `(uInt)length == 0` after truncation of a `length` that is a non-zero multiple of `uInt` range (`png.c:182`) | defensive fixup `safe_length = (uInt)-1`; loop continues, no error |
| | `png_user_version_check` | `user_png_ver == NULL` (`png.c:229`) | sets `PNG_FLAG_LIBRARY_MISMATCH` |
| | `png_user_version_check` | `user_png_ver[i] != PNG_LIBPNG_VER_STRING[i]` up to the second `.` (`png.c:221`) | sets `PNG_FLAG_LIBRARY_MISMATCH` |
| | `png_user_version_check` | `(png_ptr->flags & PNG_FLAG_LIBRARY_MISMATCH) != 0` (`png.c:232`) | `png_warning(png_ptr, "Application built with libpng-<ver> but running with <ver>")` then `return 0` (failure) |
| | `png_create_png_struct` | `png_user_version_check(...) == 0` (`png.c:329`) | falls through to `return NULL` |
| | `png_create_png_struct` | `png_malloc_warn(&create_struct, sizeof *png_ptr) == NULL` i.e. OOM for `png_struct` (`png.c:334`) | `png_warning(..., "Out of memory")` from `png_malloc_warn`, then `return NULL` |
| | `png_create_png_struct` | user memory allocator calls `png_error`/longjmp back into `create_jmp_buf` (`png.c:314`) | `return NULL` |
| | `png_create_info_struct` | `png_ptr == NULL` (`png.c:373`) | `return NULL` |
| | `png_create_info_struct` | `png_malloc_base(png_ptr, sizeof *info_ptr) == NULL` i.e. OOM (`png.c:384`) | `return NULL`, no error raised |
| | `png_destroy_info_struct` | `png_ptr == NULL` (`png.c:405`) | silent `return` |
| | `png_destroy_info_struct` | `info_ptr_ptr == NULL` or `*info_ptr_ptr == NULL` (`png.c:408`, `png.c:411`) | silent no-op |
| | `png_info_init_3` | `*ptr_ptr == NULL` (`png.c:444`) | silent `return` |
| | `png_info_init_3` | `(sizeof (png_info)) > png_info_struct_size` — caller's struct too small (`png.c:447`) | `*ptr_ptr = NULL`, raw `free(info_ptr)`, re-allocate via `png_malloc_base(NULL, ...)` |
| | `png_info_init_3` | re-allocation after size mismatch fails: `info_ptr == NULL` (`png.c:454`) | `*ptr_ptr` left `NULL`, `return` (no error raised) |
| | `png_data_freer` | `png_ptr == NULL` or `info_ptr == NULL` (`png.c:469`) | silent `return` |
| | `png_data_freer` | `freer` is neither `PNG_DESTROY_WILL_FREE_DATA` nor `PNG_USER_WILL_FREE_DATA` (`png.c:478`) | `png_error(png_ptr, "Unknown freer parameter in png_data_freer")` -> error, no return |
| | `png_free_data` | `png_ptr == NULL` or `info_ptr == NULL` (`png.c:488`) | silent `return` |
| | `png_get_io_ptr` | `png_ptr == NULL` (`png.c:693`) | `return NULL` |
| | `png_init_io` | `png_ptr == NULL` (`png.c:712`) | silent `return` |
| | `png_convert_to_rfc1123_buffer` | `out == NULL` (`png.c:748`) | `return 0` (reject) |
| | `png_convert_to_rfc1123_buffer` | `ptime->year > 9999` (RFC1123 limit) (`png.c:751`) | `return 0` (reject) |
| | `png_convert_to_rfc1123_buffer` | `ptime->month == 0` (`png.c:752`) | `return 0` (reject) |
| | `png_convert_to_rfc1123_buffer` | `ptime->month > 12` (`png.c:752`) | `return 0` (reject) |
| | `png_convert_to_rfc1123_buffer` | `ptime->day == 0` (`png.c:753`) | `return 0` (reject) |
| | `png_convert_to_rfc1123_buffer` | `ptime->day > 31` (`png.c:753`) | `return 0` (reject) |
| | `png_convert_to_rfc1123_buffer` | `ptime->hour > 23` (`png.c:754`) | `return 0` (reject) |
| | `png_convert_to_rfc1123_buffer` | `ptime->minute > 59` (`png.c:754`) | `return 0` (reject) |
| | `png_convert_to_rfc1123_buffer` | `ptime->second > 60` (`png.c:755`) | `return 0` (reject) |
| | `png_convert_to_rfc1123` | `png_ptr == NULL` (`png.c:798`) | `return NULL` |
| | `png_convert_to_rfc1123` | `png_convert_to_rfc1123_buffer(...) == 0` (invalid `ptime`) (`png.c:801`) | `png_warning(png_ptr, "Ignoring invalid time value")` then `return NULL` |
| | `png_build_grayscale_palette` | `palette == NULL` (`png.c:889`) | silent `return` |
| | `png_build_grayscale_palette` | `bit_depth` not one of 1, 2, 4, 8 (`png.c:914`) | `num_palette = 0`, `color_inc = 0`; no palette entries written, silent no-op |
| | `png_handle_as_unknown` | `png_ptr == NULL` or `chunk_name == NULL` or `png_ptr->num_chunk_list == 0` (`png.c:936`) | `return PNG_HANDLE_CHUNK_AS_DEFAULT` |
| | `png_handle_as_unknown` | `chunk_name` not present in `png_ptr->chunk_list` (`png.c:960`) | `return PNG_HANDLE_CHUNK_AS_DEFAULT` |
| | `png_reset_zstream` | `png_ptr == NULL` (`png.c:981`) | `return Z_STREAM_ERROR` |
| | `png_reset_zstream` | `inflateReset(&png_ptr->zstream)` fails (uninitialized/bad zstream) (`png.c:985`) | returns zlib error code (e.g. `Z_STREAM_ERROR`) |
| | `png_zstream_error` | `png_ptr->zstream.msg == NULL` and `ret` is `Z_OK` or unrecognized (`png.c:1011`) | `png_ptr->zstream.msg = "unexpected zlib return code"` |
| | `png_zstream_error` | `ret == Z_STREAM_END` with no zlib msg (`png.c:1016`) | `png_ptr->zstream.msg = "unexpected end of LZ stream"` |
| | `png_zstream_error` | `ret == Z_NEED_DICT` — deflate stream needs a dictionary (bogus PNG) (`png.c:1021`) | `png_ptr->zstream.msg = "missing LZ dictionary"` |
| | `png_zstream_error` | `ret == Z_ERRNO` (`png.c:1028`) | `png_ptr->zstream.msg = "zlib IO error"` |
| | `png_zstream_error` | `ret == Z_STREAM_ERROR` (internal libpng error) (`png.c:1033`) | `png_ptr->zstream.msg = "bad parameters to zlib"` |
| | `png_zstream_error` | `ret == Z_DATA_ERROR` — corrupt compressed data (`png.c:1038`) | `png_ptr->zstream.msg = "damaged LZ stream"` |
| | `png_zstream_error` | `ret == Z_MEM_ERROR` (`png.c:1042`) | `png_ptr->zstream.msg = "insufficient memory"` |
| | `png_zstream_error` | `ret == Z_BUF_ERROR` — end of input/output (`png.c:1046`) | `png_ptr->zstream.msg = "truncated"` |
| | `png_zstream_error` | `ret == Z_VERSION_ERROR` (`png.c:1053`) | `png_ptr->zstream.msg = "unsupported zlib version"` |
| | `png_zstream_error` | `ret == PNG_UNEXPECTED_ZLIB_RETURN` (`png.c:1057`) | `png_ptr->zstream.msg = "unexpected zlib return"` |
| | `png_fp_add` | `addend0 > 0 && 0x7fffffff - addend0 < addend1` — positive fixed-point overflow (`png.c:1080`) | `*error = 1`, `return PNG_FP_1/2` |
| | `png_fp_add` | `addend0 < 0 && -0x7fffffff - addend0 > addend1` — negative fixed-point overflow (`png.c:1085`) | `*error = 1`, `return PNG_FP_1/2` |
| | `png_fp_sub` | `addend1 > 0 && -0x7fffffff + addend1 > addend0` — negative overflow (`png.c:1101`) | `*error = 1`, `return PNG_FP_1/2` |
| | `png_fp_sub` | `addend1 < 0 && 0x7fffffff + addend1 < addend0` — positive overflow (`png.c:1106`) | `*error = 1`, `return PNG_FP_1/2` |
| | `png_safe_add` | either inner `png_fp_add` set `error` (`png.c:1123`) | `*addend0_and_result` left unmodified, `return 1` (overflow) |
| | `png_xy_from_XYZ` | `png_safe_add(&d, XYZ->red_Y, XYZ->red_Z)` overflows (`png.c:1146`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_muldiv(&xy->redx, XYZ->red_X, PNG_FP_1, dred) == 0` — `dred == 0` or overflow (`png.c:1149`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_muldiv(&xy->redy, XYZ->red_Y, PNG_FP_1, dred) == 0` (`png.c:1151`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_safe_add(&d, XYZ->green_Y, XYZ->green_Z)` overflows (`png.c:1155`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_muldiv(&xy->greenx, XYZ->green_X, PNG_FP_1, dgreen) == 0` (`png.c:1158`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_muldiv(&xy->greeny, XYZ->green_Y, PNG_FP_1, dgreen) == 0` (`png.c:1160`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_safe_add(&d, XYZ->blue_Y, XYZ->blue_Z)` overflows (`png.c:1164`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_muldiv(&xy->bluex, XYZ->blue_X, PNG_FP_1, dblue) == 0` (`png.c:1167`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_muldiv(&xy->bluey, XYZ->blue_Y, PNG_FP_1, dblue) == 0` (`png.c:1169`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_safe_add(&d, dred, dgreen)` overflows computing `dwhite` (`png.c:1177`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_safe_add(&d, XYZ->green_X, XYZ->blue_X)` overflows computing `whiteX` (`png.c:1185`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_safe_add(&d, XYZ->green_Y, XYZ->blue_Y)` overflows computing `whiteY` (`png.c:1190`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_muldiv(&xy->whitex, whiteX, PNG_FP_1, dwhite) == 0` (`png.c:1194`) | `return 1` (error) |
| | `png_xy_from_XYZ` | `png_muldiv(&xy->whitey, whiteY, PNG_FP_1, dwhite) == 0` (`png.c:1196`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `xy->redx < 0` or `xy->redx > fpLimit` where `fpLimit = PNG_FP_1+(PNG_FP_1/10)` = 110000 (`png.c:1223`) | `return 1` (invalid cHRM) |
| | `png_XYZ_from_xy` | `xy->redy < 0` or `xy->redy > fpLimit - xy->redx` (`png.c:1224`) | `return 1` (invalid cHRM) |
| | `png_XYZ_from_xy` | `xy->greenx < 0` or `xy->greenx > fpLimit` (`png.c:1225`) | `return 1` (invalid cHRM) |
| | `png_XYZ_from_xy` | `xy->greeny < 0` or `xy->greeny > fpLimit - xy->greenx` (`png.c:1226`) | `return 1` (invalid cHRM) |
| | `png_XYZ_from_xy` | `xy->bluex < 0` or `xy->bluex > fpLimit` (`png.c:1227`) | `return 1` (invalid cHRM) |
| | `png_XYZ_from_xy` | `xy->bluey < 0` or `xy->bluey > fpLimit - xy->bluex` (`png.c:1228`) | `return 1` (invalid cHRM) |
| | `png_XYZ_from_xy` | `xy->whitex < 0` or `xy->whitex > fpLimit` (`png.c:1229`) | `return 1` (invalid cHRM) |
| | `png_XYZ_from_xy` | `xy->whitey < 5` (must be >= 5 to avoid overflow) or `xy->whitey > fpLimit - xy->whitex` (`png.c:1230`) | `return 1` (invalid cHRM) |
| | `png_XYZ_from_xy` | `png_muldiv(&left, xy->greenx-xy->bluex, xy->redy-xy->bluey, 8) == 0` (`png.c:1422`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&right, xy->greeny-xy->bluey, xy->redx-xy->bluex, 8) == 0` (`png.c:1424`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_fp_sub(left, right, &error)` overflows computing `denominator` (`png.c:1427`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&left, xy->greenx-xy->bluex, xy->whitey-xy->bluey, 8) == 0` (`png.c:1431`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&right, xy->greeny-xy->bluey, xy->whitex-xy->bluex, 8) == 0` (`png.c:1433`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&red_inverse, xy->whitey, denominator, png_fp_sub(left,right,&error)) == 0` or `error` or `red_inverse <= xy->whitey` — extreme cHRM values (`png.c:1442`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&left, xy->redy-xy->bluey, xy->whitex-xy->bluex, 8) == 0` (`png.c:1448`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&right, xy->redx-xy->bluex, xy->whitey-xy->bluey, 8) == 0` (`png.c:1450`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&green_inverse, ...) == 0` or `error` or `green_inverse <= xy->whitey` (`png.c:1452`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `error` set by the `png_fp_sub`/`png_reciprocal` chain, or `blue_scale <= 0` (`png.c:1463`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&XYZ->red_X, xy->redx, PNG_FP_1, red_inverse) == 0` (`png.c:1471`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&XYZ->red_Y, xy->redy, PNG_FP_1, red_inverse) == 0` (`png.c:1473`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&XYZ->red_Z, PNG_FP_1-xy->redx-xy->redy, PNG_FP_1, red_inverse) == 0` (`png.c:1475`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&XYZ->green_X, xy->greenx, PNG_FP_1, green_inverse) == 0` (`png.c:1479`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&XYZ->green_Y, xy->greeny, PNG_FP_1, green_inverse) == 0` (`png.c:1481`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&XYZ->green_Z, PNG_FP_1-xy->greenx-xy->greeny, PNG_FP_1, green_inverse) == 0` (`png.c:1483`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&XYZ->blue_X, xy->bluex, blue_scale, PNG_FP_1) == 0` (`png.c:1487`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&XYZ->blue_Y, xy->bluey, blue_scale, PNG_FP_1) == 0` (`png.c:1489`) | `return 1` (error) |
| | `png_XYZ_from_xy` | `png_muldiv(&XYZ->blue_Z, PNG_FP_1-xy->bluex-xy->bluey, blue_scale, PNG_FP_1) == 0` (`png.c:1491`) | `return 1` (error) |
| | `png_icc_profile_error` | called for any ICC profile defect (`png.c:1571`) | `png_chunk_benign_error(png_ptr, "profile '<name>': <tag-or-hex>: <reason>")` then `return 0` |
| | `icc_check_length` | `profile_length < 132` — iCCP profile shorter than the ICC header (`png.c:1588`) | `png_icc_profile_error(..., "too short")` -> `png_chunk_benign_error`, `return 0` |
| | `png_icc_check_length` | `icc_check_length(...)` failed, i.e. `profile_length < 132` (`png.c:1597`) | `return 0` |
| | `png_icc_check_length` | `profile_length > png_chunk_max(png_ptr)` — over the chunk-malloc limit (`png.c:1606`) | `png_icc_profile_error(..., "profile too long")`, `return 0` |
| | `png_icc_check_header` | `png_get_uint_32(profile) != profile_length` (`png.c:1626`) | `png_icc_profile_error(..., "length does not match profile")`, `return 0` |
| | `png_icc_check_header` | `profile[8] > 3 && (profile_length & 3) != 0` — major version > 3 with unaligned length (`png.c:1631`) | `png_icc_profile_error(..., "invalid length")`, `return 0` |
| | `png_icc_check_header` | tag count `png_get_uint_32(profile+128) > 357913930` (max possible `(2^32-4-132)/12`) (`png.c:1636`) | `png_icc_profile_error(..., "tag count too large")`, `return 0` |
| | `png_icc_check_header` | `profile_length < 132 + 12*tag_count` — truncated tag table (`png.c:1637`) | `png_icc_profile_error(..., "tag count too large")`, `return 0` |
| | `png_icc_check_header` | rendering intent `png_get_uint_32(profile+64) >= 0xffff` (ICC limit) (`png.c:1645`) | `png_icc_profile_error(..., "invalid rendering intent")`, `return 0` |
| | `png_icc_check_header` | rendering intent `>= PNG_sRGB_INTENT_LAST` (`png.c:1652`) | `(void)png_icc_profile_error(..., "intent outside defined range")` -> warning only, checking continues |
| | `png_icc_check_header` | `png_get_uint_32(profile+36) != 0x61637370` (`'acsp'` signature) (`png.c:1669`) | `png_icc_profile_error(..., "invalid signature")`, `return 0` |
| | `png_icc_check_header` | `memcmp(profile+68, D50_nCIEXYZ, 12) != 0` — PCS illuminant not D50 (`png.c:1680`) | `(void)png_icc_profile_error(..., "PCS illuminant is not D50")` -> warning only, continues |
| | `png_icc_check_header` | data colour space `'RGB '` (`0x52474220`) but `(color_type & PNG_COLOR_MASK_COLOR) == 0` (`png.c:1708`) | `png_icc_profile_error(..., "RGB color space not permitted on grayscale PNG")`, `return 0` |
| | `png_icc_check_header` | data colour space `'GRAY'` (`0x47524159`) but `(color_type & PNG_COLOR_MASK_COLOR) != 0` (`png.c:1714`) | `png_icc_profile_error(..., "Gray color space not permitted on RGB PNG")`, `return 0` |
| | `png_icc_check_header` | data colour space is neither `'RGB '` nor `'GRAY'` (`png.c:1720`) | `png_icc_profile_error(..., "invalid ICC profile color space")`, `return 0` |
| | `png_icc_check_header` | profile/device class `'abst'` (`0x61627374`) (`png.c:1745`) | `png_icc_profile_error(..., "invalid embedded Abstract ICC profile")`, `return 0` |
| | `png_icc_check_header` | profile/device class `'link'` (`0x6c696e6b`) (`png.c:1755`) | `png_icc_profile_error(..., "unexpected DeviceLink ICC profile class")`, `return 0` |
| | `png_icc_check_header` | profile/device class `'nmcl'` (`0x6e6d636c`) (`png.c:1763`) | `(void)png_icc_profile_error(..., "unexpected NamedColor ICC profile class")` -> warning only, continues |
| | `png_icc_check_header` | profile/device class not in {`'scnr'`,`'mntr'`,`'prtr'`,`'spac'`,`'abst'`,`'link'`,`'nmcl'`} (`png.c:1773`) | `(void)png_icc_profile_error(..., "unrecognized ICC profile class")` -> warning only, continues |
| | `png_icc_check_header` | PCS `png_get_uint_32(profile+20)` is neither `'XYZ '` (`0x58595a20`) nor `'Lab '` (`0x4c616220`) (`png.c:1789`) | `png_icc_profile_error(..., "unexpected ICC PCS encoding")`, `return 0` |
| | `png_icc_check_tag_table` | for any tag: `tag_start > profile_length` or `tag_length > profile_length - tag_start` — tag data outside the profile (`png.c:1824`) | `png_icc_profile_error(..., "ICC profile tag outside profile")`, `return 0` |
| | `png_icc_check_tag_table` | `(tag_start & 3) != 0` — tag start not 4-byte aligned (`png.c:1828`) | `(void)png_icc_profile_error(..., "ICC profile tag start not a multiple of 4")` -> warning only, `return 1` |
| | `png_set_rgb_coefficients` | `have_chromaticities(png_ptr) == 0` or `png_XYZ_from_xy(&xyz, &png_ptr->chromaticities) != 0` (`png.c:1897`) | silently falls back to REC 709 defaults `red=6968`, `green=23434` |
| | `png_set_rgb_coefficients` | `total <= 0`, or any `png_muldiv` fails, or any of `r`,`g`,`b` outside `0..32768`, or `r+g+b > 32769` (`png.c:1908`) | coefficients left unset (silently ignores the cHRM-derived values) |
| | `png_set_rgb_coefficients` | after rounding adjustment `r+g+b != 32768` (`png.c:1937`) | `png_error(png_ptr, "internal error handling cHRM coefficients")` -> error, no return |
| | `png_check_IHDR` | `width == 0` (`png.c:1969`) | `png_warning(png_ptr, "Image width is zero in IHDR")`, `error = 1` -> final `png_error` |
| | `png_check_IHDR` | `width > PNG_UINT_31_MAX` (`png.c:1975`) | `png_warning(png_ptr, "Invalid image width in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `((width + 7) & ~(png_alloc_size_t)7) > ((PNG_SIZE_MAX - 48 - 1)/8) - 1` — row buffer would exceed `size_t` (`png.c:1989`) | `png_warning(png_ptr, "Image width is too large for this architecture")`, `error = 1` |
| | `png_check_IHDR` | `width > png_ptr->user_width_max` (or `PNG_USER_WIDTH_MAX`) (`png.c:2012`) | `png_warning(png_ptr, "Image width exceeds user limit in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `height == 0` (`png.c:2021`) | `png_warning(png_ptr, "Image height is zero in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `height > PNG_UINT_31_MAX` (`png.c:2027`) | `png_warning(png_ptr, "Invalid image height in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `height > png_ptr->user_height_max` (or `PNG_USER_HEIGHT_MAX`) (`png.c:2034`) | `png_warning(png_ptr, "Image height exceeds user limit in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `bit_depth` not in {1,2,4,8,16} (`png.c:2044`) | `png_warning(png_ptr, "Invalid bit depth in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `color_type < 0` or `color_type == 1` or `color_type == 5` or `color_type > 6` (`png.c:2051`) | `png_warning(png_ptr, "Invalid color type in IHDR")`, `error = 1` |
| | `png_check_IHDR` | palette color type with `bit_depth > 8`, or RGB/GA/RGBA with `bit_depth < 8` (`png.c:2058`) | `png_warning(png_ptr, "Invalid color type/bit depth combination in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `interlace_type >= PNG_INTERLACE_LAST` (`png.c:2067`) | `png_warning(png_ptr, "Unknown interlace method in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `compression_type != PNG_COMPRESSION_TYPE_BASE` (`png.c:2073`) | `png_warning(png_ptr, "Unknown compression method in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `(png_ptr->mode & PNG_HAVE_PNG_SIGNATURE) != 0 && png_ptr->mng_features_permitted != 0` (`png.c:2089`) | `png_warning(png_ptr, "MNG features are not allowed in a PNG datastream")` -> warning only, `error` not set |
| | `png_check_IHDR` | `filter_type != PNG_FILTER_TYPE_BASE` and not a permitted MNG intrapixel-differencing case (`png.c:2093`) | `png_warning(png_ptr, "Unknown filter method in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `filter_type != PNG_FILTER_TYPE_BASE` while `(png_ptr->mode & PNG_HAVE_PNG_SIGNATURE) != 0` (`png.c:2105`) | `png_warning(png_ptr, "Invalid filter method in IHDR")`, `error = 1` |
| | `png_check_IHDR` | (non-MNG build) `filter_type != PNG_FILTER_TYPE_BASE` (`png.c:2113`) | `png_warning(png_ptr, "Unknown filter method in IHDR")`, `error = 1` |
| | `png_check_IHDR` | `error == 1` after any of the above (`png.c:2120`) | `png_error(png_ptr, "Invalid IHDR data")` -> error, no return |
| | `png_check_fp_number` | current char is not in `+-.0123456789Ee` (`png.c:2155`) | `goto PNG_FP_End`; number ends at `*whereami`, returns `(state & PNG_FP_SAW_DIGIT) != 0` |
| | `png_check_fp_number` | sign in integer part after something was already seen: `PNG_FP_INTEGER + PNG_FP_SAW_SIGN` with `(state & PNG_FP_SAW_ANY) != 0` (`png.c:2165`) | `goto PNG_FP_End` (character rejected as part of the number) |
| | `png_check_fp_number` | second `.`: `PNG_FP_INTEGER + PNG_FP_SAW_DOT` with `(state & PNG_FP_SAW_DOT) != 0` (`png.c:2173`) | `goto PNG_FP_End` |
| | `png_check_fp_number` | `E`/`e` with no preceding digit: `PNG_FP_INTEGER + PNG_FP_SAW_E` and `(state & PNG_FP_SAW_DIGIT) == 0` (`png.c:2193`) | `goto PNG_FP_End` |
| | `png_check_fp_number` | `.E` with no digits: `PNG_FP_FRACTION + PNG_FP_SAW_E` and `(state & PNG_FP_SAW_DIGIT) == 0` (`png.c:2215`) | `goto PNG_FP_End` |
| | `png_check_fp_number` | second sign in exponent: `PNG_FP_EXPONENT + PNG_FP_SAW_SIGN` with `(state & PNG_FP_SAW_ANY) != 0` (`png.c:2223`) | `goto PNG_FP_End` |
| | `png_check_fp_number` | any other state/character-type combination (e.g. sign or dot inside the fraction/exponent) (`png.c:2241`) | `goto PNG_FP_End` |
| | `png_check_fp_number` | no digit was ever seen: `(state & PNG_FP_SAW_DIGIT) == 0` (`png.c:2255`) | `return 0` (not a number) |
| | `png_check_fp_string` | `png_check_fp_number(...) == 0` — string is not a valid fp number (`png.c:2266`) | `return 0` (fail) |
| | `png_check_fp_string` | trailing garbage: `char_index != size && string[char_index] != 0` (`png.c:2267`) | `return 0` (fail) |
| | `png_pow10` | `power < DBL_MIN_10_EXP` — exponent underflows `double` (`png.c:2290`) | `return 0` |
| | `png_ascii_from_fp` | `precision < 1` (`png.c:2325`) | silently clamped: `precision = DBL_DIG` |
| | `png_ascii_from_fp` | `precision > DBL_DIG+1` (`png.c:2329`) | silently clamped: `precision = DBL_DIG+1` |
| | `png_ascii_from_fp` | `size < precision+5` — output buffer too small (`png.c:2333`) | falls through to `png_error(png_ptr, "ASCII conversion buffer too small")` |
| | `png_ascii_from_fp` | `!(fp >= DBL_MIN)` — value underflows / is zero (`png.c:2618`) | writes `"0"` and returns (no error) |
| | `png_ascii_from_fp` | `fp > DBL_MAX` or NaN (neither `>= DBL_MIN && <= DBL_MAX` nor `< DBL_MIN`) (`png.c:2624`) | writes `"inf"` and returns (no error) |
| | `png_ascii_from_fp` | `size <= cdigits` when the exponent digits must still be emitted (`png.c:2608`) | falls through to `png_error(png_ptr, "ASCII conversion buffer too small")` |
| | `png_ascii_from_fp` | reached end of function without returning (buffer too small) (`png.c:2635`) | `png_error(png_ptr, "ASCII conversion buffer too small")` -> error, no return |
| | `png_ascii_from_fixed` | `size <= 12` — buffer smaller than 13 bytes (`png.c:2649`) | falls through to `png_error(png_ptr, "ASCII conversion buffer too small")` |
| | `png_ascii_from_fixed` | `num > 0x80000000` after negation — `fp` magnitude overflows `png_uint_32` (`png.c:2661`) | falls through to `png_error(png_ptr, "ASCII conversion buffer too small")` |
| | `png_ascii_from_fixed` | reached end of function without returning (`png.c:2713`) | `png_error(png_ptr, "ASCII conversion buffer too small")` -> error, no return |
| | `png_fixed` | `floor(100000*fp+.5) > 2147483647.` or `< -2147483648.` (`png.c:2730`) | `png_fixed_error(png_ptr, text)` -> `png_error(png_ptr, "fixed point overflow in <text>")`, no return |
| | `png_fixed_ITU` | `floor(10000*fp+.5) > 2147483647.` or `< 0` (`png.c:2749`) | `png_fixed_error(png_ptr, text)` -> `png_error(png_ptr, "fixed point overflow in <text>")`, no return |
| | `png_muldiv` | `divisor == 0` (`png.c:2774`) | `return 0` (failure), `*res` unmodified |
| | `png_muldiv` | (floating build) `floor(a*times/divisor+.5) > 2147483647.` or `< -2147483648.` (`png.c:2790`) | falls through to `return 0` (overflow) |
| | `png_muldiv` | (integer build) `s32 >= D` — the 64-bit product overflows the 32-bit quotient (`png.c:2832`) | falls through to `return 0` (overflow) |
| | `png_muldiv` | (integer build) sign of `result` inconsistent with `negative` after rounding — overflow (`png.c:2870`) | falls through to `return 0` |
| | `png_reciprocal` | `a == 0` (divide by zero) or `floor(1E10/a+.5)` outside `png_fixed_point` range, or `png_muldiv(&res,100000,100000,a) == 0` (`png.c:2889`, `png.c:2896`) | `return 0` (error/overflow) |
| | `png_product2` | `floor(a*1E-5*b+.5)` out of `png_fixed_point` range, or `png_muldiv(&res,a,b,100000) == 0` (`png.c:2938`, `png.c:2943`) | `return 0` (overflow) |
| | `png_reciprocal2` | `a == 0` or `b == 0` (`png.c:2956`) | `return 0` (overflow/error) |
| | `png_reciprocal2` | `floor(1E15/a/b+.5) > 2147483647.` or `< -2147483648.` (`png.c:2962`) | `return 0` (overflow) |
| | `png_reciprocal2` | (integer build) `png_product2(a, b) == 0` (`png.c:2973`) | `return 0` (overflow) |
| | `png_log8bit` | `(x &= 0xff) == 0` — log of zero (`png.c:3057`) | `return -1` (overflow marker) |
| | `png_log16bit` | `(x &= 0xffff) == 0` — log of zero (`png.c:3110`) | `return -1` (overflow marker) |
| | `png_exp` | `x <= 0` — exponent overflow (`png.c:3237`) | `return png_32bit_exp[0]` (saturates at max 32-bit value) |
| | `png_exp` | `x > 0xfffff` — exponent underflow (`png.c:3241`) | `return 0` |
| | `png_gamma_8bit_correct` | `value == 0` or `value >= 255` (`png.c:3275`) | no gamma applied; `return (png_byte)(value & 0xff)` |
| | `png_gamma_8bit_correct` | (integer build) `png_muldiv(&res, gamma_val, lg2, PNG_FP_1) == 0` — overflow (`png.c:3308`) | `value = 0`, `return 0` |
| | `png_gamma_16bit_correct` | `value == 0` or `value >= 65535` (`png.c:3323`) | no gamma applied; `return (png_uint_16)value` |
| | `png_gamma_16bit_correct` | (integer build) `png_muldiv(&res, gamma_val, lg2, PNG_FP_1) == 0` — overflow (`png.c:3338`) | `value = 0`, `return 0` |
| | `png_gamma_correct` | `png_ptr->bit_depth != 8` and `PNG_16BIT_SUPPORTED` is not defined (`png.c:3367`) | `return 0` ("should not reach this") |
| | `png_build_gamma_table` | `png_ptr->gamma_table != NULL` or `png_ptr->gamma_16_table != NULL` — table built twice (`png.c:3632`) | `png_warning(png_ptr, "gamma table being rebuilt")` then `png_destroy_gamma_table()` and rebuild |
| | `png_build_gamma_table` | `sig_bit == 0` or `sig_bit >= 16U` — out-of-range sBIT (`png.c:3713`) | `shift = 0` (all 16 bits kept) |
| | `png_build_gamma_table` | `shift < (16U - PNG_MAX_GAMMA_8)` while 16-to-8 transform requested (`png.c:3726`) | clamped: `shift = 16U - PNG_MAX_GAMMA_8` |
| | `png_build_gamma_table` | `shift > 8U` (`png.c:3730`) | clamped: `shift = 8U` (guarantees at least one table) |
| | `png_set_option` | `png_ptr == NULL` (`png.c:3771`) | `return PNG_OPTION_INVALID` |
| | `png_set_option` | `option < 0` (`png.c:3771`) | `return PNG_OPTION_INVALID` |
| | `png_set_option` | `option >= PNG_OPTION_NEXT` (`png.c:3771`) | `return PNG_OPTION_INVALID` |
| | `png_set_option` | `(option & 1) != 0` — odd (non-option) value (`png.c:3772`) | `return PNG_OPTION_INVALID` |
| | `png_image_free_function` | `image->opaque->png_ptr == NULL` (`png.c:3968`) | `return 0` (failure) |
| | `png_image_free_function` | `c.for_write != 0` but `PNG_SIMPLIFIED_WRITE_SUPPORTED` undefined (`png.c:4002`) | `png_error(c.png_ptr, "simplified write not supported")` -> error, no return |
| | `png_image_free_function` | `c.for_write == 0` but `PNG_SIMPLIFIED_READ_SUPPORTED` undefined (`png.c:4010`) | `png_error(c.png_ptr, "simplified read not supported")` -> error, no return |
| | `png_image_free` | `image == NULL`, or `image->opaque == NULL`, or `image->opaque->error_buf != NULL` (inside error handling) (`png.c:4025`) | silent no-op (`png_safe_execute` will free later) |
| | `png_image_error` | called on any simplified-API failure (`png.c:4034`) | copies `error_message` into `image->message`, sets `PNG_IMAGE_ERROR` in `image->warning_or_error`, `png_image_free(image)`, `return 0` |
| | `png_error` | any fatal error; `png_ptr != NULL && png_ptr->error_fn != NULL` (`pngerror.c:42`) | calls `png_ptr->error_fn(png_ptr, error_message)`; if it returns, `png_default_error()` which never returns |
| | `png_error` | `png_ptr == NULL` or `png_ptr->error_fn == NULL` (`pngerror.c:42`) | `png_default_error(png_ptr, error_message)` -> prints and `png_longjmp(png_ptr, 1)`, never returns |
| | `png_err` | (build without `PNG_ERROR_TEXT_SUPPORTED`) any fatal error (`pngerror.c:60`) | `error_fn(png_ptr, "")` then `png_default_error(png_ptr, "")`, never returns |
| | `png_safecat` | `buffer == NULL` or `pos >= bufsize` (`pngerror.c:76`) | nothing written, `return pos` unchanged |
| | `png_safecat` | appended string would exceed `bufsize-1` (`pngerror.c:79`) | silently truncated, `'\0'`-terminated |
| | `png_format_number` | `format` not one of `PNG_NUMBER_FORMAT_{fixed,02u,u,02x,x}` (`pngerror.c:144`) | `number = 0` (error), loop terminates, returns partial/empty string |
| | `png_warning` | `png_ptr == NULL` or `png_ptr->warning_fn == NULL` (`pngerror.c:180`) | `png_default_warning(png_ptr, warning_message)` -> `fprintf(stderr, "libpng warning: %s", ...)` |
| | `png_warning_parameter` | `number <= 0` or `number > PNG_WARNING_PARAMETER_COUNT` (`pngerror.c:196`) | silently ignored, parameter not stored |
| | `png_formatted_warning` | formatted message longer than `sizeof msg - 1` (191 bytes) (`pngerror.c:247`) | message silently truncated then passed to `png_warning` |
| | `png_formatted_warning` | `@<digit>` where the digit index `>= PNG_WARNING_PARAMETER_COUNT` (`pngerror.c:266`) | not treated as a parameter; the character is copied literally |
| | `png_benign_error` | `(png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0` and read struct with `chunk_name != 0` (`pngerror.c:313`) | `png_chunk_warning(png_ptr, error_message)` -> warning, processing continues |
| | `png_benign_error` | `(png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0`, not a read struct or `chunk_name == 0` (`pngerror.c:318`) | `png_warning(png_ptr, error_message)` -> warning, processing continues |
| | `png_benign_error` | `PNG_FLAG_BENIGN_ERRORS_WARN` clear and read struct with `chunk_name != 0` (`pngerror.c:326`) | `png_chunk_error(png_ptr, error_message)` -> fatal error, no return |
| | `png_benign_error` | `PNG_FLAG_BENIGN_ERRORS_WARN` clear, not a read struct or `chunk_name == 0` (`pngerror.c:329`) | `png_error(png_ptr, error_message)` -> fatal error, no return |
| | `png_app_warning` | `(png_ptr->flags & PNG_FLAG_APP_WARNINGS_WARN) != 0` (`pngerror.c:340`) | `png_warning(png_ptr, error_message)` -> warning |
| | `png_app_warning` | `PNG_FLAG_APP_WARNINGS_WARN` clear (default: app misuse is fatal) (`pngerror.c:343`) | `png_error(png_ptr, error_message)` -> fatal error, no return |
| | `png_app_error` | `(png_ptr->flags & PNG_FLAG_APP_ERRORS_WARN) != 0` (`pngerror.c:353`) | `png_warning(png_ptr, error_message)` -> warning |
| | `png_app_error` | `PNG_FLAG_APP_ERRORS_WARN` clear (`pngerror.c:356`) | `png_error(png_ptr, error_message)` -> fatal error, no return |
| | `png_format_buffer` | chunk name byte fails `isnonalpha(c)`, i.e. `c < 65 or c > 122 or (c > 90 and c < 97)` (`pngerror.c:391`) | byte rendered as `[HH]` hex escape in the message prefix |
| | `png_format_buffer` | `error_message == NULL` (`pngerror.c:405`) | buffer holds only the chunk-name prefix, `'\0'`-terminated |
| | `png_format_buffer` | `error_message` longer than `PNG_MAX_ERROR_TEXT-1` (195) (`pngerror.c:415`) | message silently truncated to 195 chars |
| | `png_chunk_error` | `png_ptr == NULL` (`pngerror.c:430`) | `png_error(png_ptr, error_message)` (unprefixed), never returns |
| | `png_chunk_error` | any chunk-level fatal error with `png_ptr != NULL` (`pngerror.c:435`) | `png_error(png_ptr, "<cHNK>: <error_message>")`, never returns |
| | `png_chunk_warning` | `png_ptr == NULL` (`pngerror.c:446`) | `png_warning(png_ptr, warning_message)` (unprefixed) |
| | `png_chunk_warning` | any chunk-level warning with `png_ptr != NULL` (`pngerror.c:451`) | `png_warning(png_ptr, "<cHNK>: <warning_message>")` |
| | `png_chunk_benign_error` | `(png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0` (`pngerror.c:463`) | `png_chunk_warning(png_ptr, error_message)` -> warning, chunk ignored |
| | `png_chunk_benign_error` | `PNG_FLAG_BENIGN_ERRORS_WARN` clear (default on read) (`pngerror.c:467`) | `png_chunk_error(png_ptr, error_message)` -> fatal error, no return |
| | `png_chunk_report` | read struct and `error < PNG_CHUNK_ERROR` (`pngerror.c:492`) | `png_chunk_warning(png_ptr, message)` |
| | `png_chunk_report` | read struct and `error >= PNG_CHUNK_ERROR` (`pngerror.c:496`) | `png_chunk_benign_error(png_ptr, message)` (warning or fatal per flags) |
| | `png_chunk_report` | write struct and `error < PNG_CHUNK_WRITE_ERROR` (`pngerror.c:507`) | `png_app_warning(png_ptr, message)` |
| | `png_chunk_report` | write struct and `error >= PNG_CHUNK_WRITE_ERROR` (`pngerror.c:510`) | `png_app_error(png_ptr, message)` |
| | `png_fixed_error` | fixed-point conversion overflow reported by `png_fixed`/`png_fixed_ITU` (`pngerror.c:534`) | `png_error(png_ptr, "fixed point overflow in <name>")`, `name` truncated to `PNG_MAX_ERROR_TEXT-1`, never returns |
| | `png_set_longjmp_fn` | `png_ptr == NULL` (`pngerror.c:557`) | `return NULL` |
| | `png_set_longjmp_fn` | `jmp_buf_size > sizeof png_ptr->jmp_buf_local` and `png_malloc_warn` returns NULL (OOM) (`pngerror.c:572`) | `png_warning(..., "Out of memory")` from `png_malloc_warn`, then `return NULL` |
| | `png_set_longjmp_fn` | `png_ptr->jmp_buf_size == 0` but `png_ptr->jmp_buf_ptr != &png_ptr->jmp_buf_local` — stale stack jmp_buf (internal error) (`pngerror.c:586`) | `png_error(png_ptr, "Libpng jmp_buf still allocated")` -> fatal, no return |
| | `png_set_longjmp_fn` | `size != jmp_buf_size` — app changed its `jmp_buf` size between calls (`pngerror.c:598`) | `png_warning(png_ptr, "Application jmp_buf size changed")` then `return NULL` |
| | `png_free_jmpbuf` | `png_ptr == NULL` (`pngerror.c:615`) | silent `return` |
| | `png_free_jmpbuf` | `jb == NULL` or `png_ptr->jmp_buf_size == 0` (stack allocation) (`pngerror.c:622`) | no free performed; fields still zeroed |
| | `png_default_error` | `error_message == NULL` (`pngerror.c:662`) | prints `"libpng error: undefined"` then `png_longjmp(png_ptr, 1)` |
| | `png_default_error` | any fatal error reaching the default handler (`pngerror.c:668`) | `fprintf(stderr, "libpng error: %s", error_message)` then `png_longjmp(png_ptr, 1)`, never returns |
| | `png_longjmp` | `png_ptr == NULL`, or `png_ptr->longjmp_fn == NULL`, or `png_ptr->jmp_buf_ptr == NULL` — no error-return path installed (`pngerror.c:676`) | falls through to `PNG_ABORT()` — process/thread terminated |
| | `png_set_error_fn` | `png_ptr == NULL` (`pngerror.c:721`) | silent `return` |
| | `png_get_error_ptr` | `png_ptr == NULL` (`pngerror.c:741`) | `return NULL` |
| | `png_safe_error` | `image != NULL` and `image->opaque != NULL` and `image->opaque->error_buf != NULL` (`pngerror.c:782`) | logs `error_message` into `image->message`, sets `PNG_IMAGE_ERROR`, `longjmp(png_control_jmp_buf(image->opaque), 1)` |
| | `png_safe_error` | `image != NULL` but `image->opaque == NULL` or `image->opaque->error_buf == NULL` — missing longjmp buffer (`pngerror.c:786`) | sets `image->message` to `"bad longjmp: <error_message>"` then `abort()` |
| | `png_safe_error` | `image == NULL` (`error_ptr` not a `png_image`) (`pngerror.c:773`) | falls through to `abort()` |
| | `png_safe_warning` | `image->warning_or_error != 0` — a prior warning/error already logged (`pngerror.c:806`) | new warning silently discarded |
| | `png_safe_execute` | `function(arg)` returned false (`pngerror.c:829`) | `error_buf` restored; `png_image_free(image)` if `saved_error_buf == NULL`; `return 0` (failure) |
| | `png_safe_execute` | `png_error` inside `function` longjmps back to `safe_jmpbuf` (`pngerror.c:821`) | `error_buf` restored; `png_image_free(image)` if `saved_error_buf == NULL`; `return 0` (failure) |
| | `png_destroy_png_struct` | `png_ptr == NULL` (`pngmem.c:26`) | silent `return` |
| | `png_calloc` | `png_malloc(png_ptr, size) == NULL` (only possible when `png_ptr == NULL`) (`pngmem.c:56`) | no `memset`, `return NULL` |
| | `png_malloc_base` | `PNG_MAX_MALLOC_64K` build and `size > 65536U` (`pngmem.c:83`) | `return NULL` |
| | `png_malloc_base` | `size > PNG_SIZE_MAX` — would truncate in the `(size_t)` cast to `malloc` (`pngmem.c:88`) | `return NULL` |
| | `png_malloc_base` | user `malloc_fn` or system `malloc` returns NULL (`pngmem.c:92`, `pngmem.c:98`) | `return NULL` (caller must handle) |
| | `png_malloc_array_checked` | `req > PNG_SIZE_MAX/element_size` — `nelements*element_size` overflows (`pngmem.c:113`) | `return NULL` (request too large) |
| | `png_malloc_array` | `nelements <= 0` (`pngmem.c:125`) | `png_error(png_ptr, "internal error: array alloc")` -> fatal, no return |
| | `png_malloc_array` | `element_size == 0` (`pngmem.c:125`) | `png_error(png_ptr, "internal error: array alloc")` -> fatal, no return |
| | `png_realloc_array` | `add_elements <= 0` (`pngmem.c:137`) | `png_error(png_ptr, "internal error: array realloc")` -> fatal, no return |
| | `png_realloc_array` | `element_size == 0` (`pngmem.c:137`) | `png_error(png_ptr, "internal error: array realloc")` -> fatal, no return |
| | `png_realloc_array` | `old_elements < 0` (`pngmem.c:137`) | `png_error(png_ptr, "internal error: array realloc")` -> fatal, no return |
| | `png_realloc_array` | `old_array == NULL && old_elements > 0` (`pngmem.c:138`) | `png_error(png_ptr, "internal error: array realloc")` -> fatal, no return |
| | `png_realloc_array` | `add_elements > INT_MAX - old_elements` — element count overflows `int` (`pngmem.c:144`) | `return NULL` (error) |
| | `png_realloc_array` | `png_malloc_array_checked(...) == NULL` — allocation failed or too large (`pngmem.c:149`) | `return NULL` (error) |
| | `png_malloc` | `png_ptr == NULL` (`pngmem.c:178`) | `return NULL` |
| | `png_malloc` | `png_malloc_base(png_ptr, size) == NULL` — OOM or `size > PNG_SIZE_MAX` (`pngmem.c:183`) | `png_error(png_ptr, "Out of memory")` -> fatal, no return |
| | `png_malloc_default` | `png_ptr == NULL` (`pngmem.c:196`) | `return NULL` |
| | `png_malloc_default` | `png_malloc_base(NULL, size) == NULL` (`pngmem.c:202`) | `png_error(png_ptr, "Out of Memory")` -> fatal, no return |
| | `png_malloc_warn` | `png_ptr == NULL` (`pngmem.c:217`) | `return NULL` (no warning issued) |
| | `png_malloc_warn` | `png_malloc_base(png_ptr, size) == NULL` (`pngmem.c:221`) | `png_warning(png_ptr, "Out of memory")` then `return NULL` |
| | `png_free` | `png_ptr == NULL` or `ptr == NULL` (`pngmem.c:236`) | silent `return`, nothing freed |
| | `png_free_default` | `png_ptr == NULL` or `ptr == NULL` (`pngmem.c:251`) | silent `return`, nothing freed |
| | `png_set_mem_fn` | `png_ptr == NULL` (`pngmem.c:266`) | silent no-op |
| | `png_get_mem_ptr` | `png_ptr == NULL` (`pngmem.c:281`) | `return NULL` |
| | `png_read_data` | `png_ptr->read_data_fn == NULL` — no read function installed (`pngrio.c:35`) | `png_error(png_ptr, "Call to NULL read function")` -> fatal, no return |
| | `png_default_read_data` | `png_ptr == NULL` (`pngrio.c:53`) | silent `return`, buffer left untouched |
| | `png_default_read_data` | `fread(...) != length` — truncated input or stream error (`pngrio.c:61`) | `png_error(png_ptr, "Read Error")` -> fatal, no return |
| | `png_set_read_fn` | `png_ptr == NULL` (`pngrio.c:89`) | silent `return` |
| | `png_set_read_fn` | `read_data_fn == NULL` (`pngrio.c:95`) | falls back to `png_ptr->read_data_fn = png_default_read_data` (or stores NULL if no STDIO -> later "Call to NULL read function") |
| | `png_set_read_fn` | `png_ptr->write_data_fn != NULL` — read fn set on a write struct (`pngrio.c:106`) | `write_data_fn` cleared to NULL and `png_warning(png_ptr, "Can't set both read_data_fn and write_data_fn in the same structure")` |
| | `png_write_data` | `png_ptr->write_data_fn == NULL` — no write function installed (`pngwio.c:35`) | `png_error(png_ptr, "Call to NULL write function")` -> fatal, no return |
| | `png_default_write_data` | `png_ptr == NULL` (`pngwio.c:54`) | silent `return`, nothing written |
| | `png_default_write_data` | `fwrite(...) != length` — short write / stream error (`pngwio.c:59`) | `png_error(png_ptr, "Write Error")` -> fatal, no return |
| | `png_flush` | `png_ptr->output_flush_fn == NULL` (`pngwio.c:72`) | silent no-op (nothing flushed) |
| | `png_default_flush` | `png_ptr == NULL` (`pngwio.c:82`) | silent `return` |
| | `png_set_write_fn` | `png_ptr == NULL` (`pngwio.c:124`) | silent `return` |
| | `png_set_write_fn` | `write_data_fn == NULL` (`pngwio.c:130`) | falls back to `png_ptr->write_data_fn = png_default_write_data` (or stores NULL if no STDIO -> later "Call to NULL write function") |
| | `png_set_write_fn` | `output_flush_fn == NULL` (`pngwio.c:142`) | falls back to `png_ptr->output_flush_fn = png_default_flush` (or stores NULL if no STDIO) |
| | `png_set_write_fn` | `png_ptr->read_data_fn != NULL` — write fn set on a read struct (`pngwio.c:157`) | `read_data_fn` cleared to NULL and `png_warning(png_ptr, "Can't set both read_data_fn and write_data_fn in the same structure")` |
