# ERRORS.md — the ERROR-SURFACE TABLE (Phase A, gates Phase C)

Mechanically derived from `c_src/src/*.c`: every `png_error` / `png_chunk_error`
/ `png_app_error` / `png_benign_error` / `png_chunk_benign_error` /
`png_chunk_report` / `png_warning` / `png_app_warning` / `png_chunk_warning`
call site, every rejecting `return -1` / `return 0` / `return NULL`, every
explicit range / null / overflow check and every min/max constant comparison.
There are **no `assert()`** calls anywhere in the library.

One row per distinct rejection.  The `[x]` column is ticked when a differential
test constructs that exact condition and asserts both libraries produce the
same error/rejection.

Note on non-testable rows: a handful of C paths dereference a pointer *before*
checking it (`png_muldiv`'s `res`, `png_convert_to_rfc1123_buffer`'s `ptime`,
`png_do_bgr`'s `row`, `png_get_io_state`'s `png_ptr`, ...).  Those are C
undefined behaviour, not error paths, and are marked `n/a (C UB)`.


## Coverage

| ERRORS.md section | differential error-path test file |
|---|---|
| `png.c` / `pngerror.c` / `pngmem.c` / `pngrio.c` / `pngwio.c` | `tests/t10_errors_core.rs` |
| `pngget.c` / `pngset.c` / `pngtrans.c` | `tests/t11_errors_setget.rs` |
| `pngread.c` / `pngrtran.c` / `pngpread.c` | `tests/t12_errors_read.rs` |
| `pngrutil.c` / `pngwrite.c` / `pngwutil.c` / `pngwtran.c` | `tests/t13_errors_write.rs` |

Additionally `tests/t14_argorder.rs` pins down which of several
`"fixed point overflow in ..."` errors fires when a floating-point `png_set_*`
wrapper is given more than one out-of-range argument (the C evaluates the
conversions inside one argument list, so the order is unspecified in C but must
still match the reference build).

## Rows marked `n/a`

A row is `n/a` when it CANNOT be reached in this build configuration, so there
is no observable behaviour to compare:

* **272, 278** -- `"Call to NULL read function"` / `"Call to NULL write
  function"`.  `PNG_STDIO_SUPPORTED` is on, so `png_create_read_struct_2`
  (`pngread.c:76`) and `png_create_write_struct_2` (`pngwrite.c:614`)
  immediately install `png_default_read_data` / `png_default_write_data`, and
  `png_set_read_fn`/`png_set_write_fn` re-install them when passed NULL.  With
  `io_ptr == NULL` those defaults `fread`/`fwrite` a NULL `FILE*`, which is C
  undefined behaviour rather than an error return.
* **477** -- `"iTXt chunk not supported"`: `PNG_iTXt_SUPPORTED` is defined, so
  `pngset.c:1061-1066` is not compiled.
* **499, 500** -- `"no unknown chunk support on read/write"`: both
  `PNG_READ_UNKNOWN_CHUNKS_SUPPORTED` and `PNG_WRITE_UNKNOWN_CHUNKS_SUPPORTED`
  are defined.
* **517** -- `"Compression buffer size limited to system maximum"`:
  `ZLIB_IO_MAX == UINT_MAX`, but `pngset.c:1804` already rejects everything
  above `PNG_UINT_31_MAX`, so `size > ZLIB_IO_MAX` is unreachable here.
* **543, 546** -- `"png_set_filler not supported on read/write"`: both
  `PNG_READ_FILLER_SUPPORTED` and `PNG_WRITE_FILLER_SUPPORTED` are defined.
* **980, 981, 982, 986, 993, 1002, 1025** -- `"... is not defined"` /
  `"... not supported"` on the write side: every write transform and chunk
  writer is compiled in.
* **1107, 1113, 1115, 1123** -- need a text string or a row larger than 2 GB.

Separately, a number of *sub-cases* of otherwise-testable rows are C undefined
behaviour rather than error paths, because the C dereferences a pointer before
checking it.  Each one is documented at its call site in the test files with the
exact `c_src` file:line (search the tests for `C UB`).  Examples:
`png_muldiv`'s `res`, `png_do_bgr`'s `row`, `png_convert_to_rfc1123_buffer`'s
`ptime`, `png_get_PLTE`'s `num_palette`, `png_get_sCAL*`'s out-parameters,
`png_write_row`'s `row`, `png_flush`'s / `png_read_start_row`'s `png_ptr`,
`png_icc_check_*`'s `name`/`profile`.

## png.c / pngerror.c / pngmem.c / pngrio.c / pngwio.c

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `png_set_sig_bytes` | `png_ptr == NULL` (`png.c:59`) | silent `return`, no state change | [x] |
| 2 | `png_set_sig_bytes` | `num_bytes < 0` (`png.c:62`) | silently clamped: `nb = 0`, `png_ptr->sig_bytes = 0` | [x] |
| 3 | `png_set_sig_bytes` | `nb > 8` i.e. more than 8 already-consumed signature bytes (`png.c:65`) | `png_error(png_ptr, "Too many bytes for PNG signature")` -> error, no return | [x] |
| 4 | `png_sig_cmp` | `num_to_check > 8` (`png.c:84`) | silently clamped to `num_to_check = 8` | [x] |
| 5 | `png_sig_cmp` | `num_to_check < 1` (`png.c:87`) | `return -1` (reject) | [x] |
| 6 | `png_sig_cmp` | `start > 7` (`png.c:90`) | `return -1` (reject) | [x] |
| 7 | `png_sig_cmp` | `start + num_to_check > 8` (`png.c:93`) | silently clamped: `num_to_check = 8 - start` | [x] |
| 8 | `png_sig_cmp` | `memcmp(&sig[start], &png_signature[start], num_to_check) != 0` (`png.c:96`) | returns non-zero (signature mismatch) | [x] |
| 9 | `png_zalloc` | `png_ptr == NULL` (`png.c:109`) | `return NULL` | [x] |
| 10 | `png_zalloc` | `size != 0 && items >= (~(png_alloc_size_t)0) / size` — `items*size` overflows `png_alloc_size_t` (`png.c:118`) | `png_warning(png_ptr, "Potential overflow in png_zalloc()")` then `return NULL` | [x] |
| 11 | `png_calculate_crc` | `(uInt)length == 0` after truncation of a `length` that is a non-zero multiple of `uInt` range (`png.c:182`) | defensive fixup `safe_length = (uInt)-1`; loop continues, no error | [x] |
| 12 | `png_user_version_check` | `user_png_ver == NULL` (`png.c:229`) | sets `PNG_FLAG_LIBRARY_MISMATCH` | [x] |
| 13 | `png_user_version_check` | `user_png_ver[i] != PNG_LIBPNG_VER_STRING[i]` up to the second `.` (`png.c:221`) | sets `PNG_FLAG_LIBRARY_MISMATCH` | [x] |
| 14 | `png_user_version_check` | `(png_ptr->flags & PNG_FLAG_LIBRARY_MISMATCH) != 0` (`png.c:232`) | `png_warning(png_ptr, "Application built with libpng-<ver> but running with <ver>")` then `return 0` (failure) | [x] |
| 15 | `png_create_png_struct` | `png_user_version_check(...) == 0` (`png.c:329`) | falls through to `return NULL` | [x] |
| 16 | `png_create_png_struct` | `png_malloc_warn(&create_struct, sizeof *png_ptr) == NULL` i.e. OOM for `png_struct` (`png.c:334`) | `png_warning(..., "Out of memory")` from `png_malloc_warn`, then `return NULL` | [x] |
| 17 | `png_create_png_struct` | user memory allocator calls `png_error`/longjmp back into `create_jmp_buf` (`png.c:314`) | `return NULL` | [x] |
| 18 | `png_create_info_struct` | `png_ptr == NULL` (`png.c:373`) | `return NULL` | [x] |
| 19 | `png_create_info_struct` | `png_malloc_base(png_ptr, sizeof *info_ptr) == NULL` i.e. OOM (`png.c:384`) | `return NULL`, no error raised | [x] |
| 20 | `png_destroy_info_struct` | `png_ptr == NULL` (`png.c:405`) | silent `return` | [x] |
| 21 | `png_destroy_info_struct` | `info_ptr_ptr == NULL` or `*info_ptr_ptr == NULL` (`png.c:408`, `png.c:411`) | silent no-op | [x] |
| 22 | `png_info_init_3` | `*ptr_ptr == NULL` (`png.c:444`) | silent `return` | [x] |
| 23 | `png_info_init_3` | `(sizeof (png_info)) > png_info_struct_size` — caller's struct too small (`png.c:447`) | `*ptr_ptr = NULL`, raw `free(info_ptr)`, re-allocate via `png_malloc_base(NULL, ...)` | [x] |
| 24 | `png_info_init_3` | re-allocation after size mismatch fails: `info_ptr == NULL` (`png.c:454`) | `*ptr_ptr` left `NULL`, `return` (no error raised) | [x] |
| 25 | `png_data_freer` | `png_ptr == NULL` or `info_ptr == NULL` (`png.c:469`) | silent `return` | [x] |
| 26 | `png_data_freer` | `freer` is neither `PNG_DESTROY_WILL_FREE_DATA` nor `PNG_USER_WILL_FREE_DATA` (`png.c:478`) | `png_error(png_ptr, "Unknown freer parameter in png_data_freer")` -> error, no return | [x] |
| 27 | `png_free_data` | `png_ptr == NULL` or `info_ptr == NULL` (`png.c:488`) | silent `return` | [x] |
| 28 | `png_get_io_ptr` | `png_ptr == NULL` (`png.c:693`) | `return NULL` | [x] |
| 29 | `png_init_io` | `png_ptr == NULL` (`png.c:712`) | silent `return` | [x] |
| 30 | `png_convert_to_rfc1123_buffer` | `out == NULL` (`png.c:748`) | `return 0` (reject) | [x] |
| 31 | `png_convert_to_rfc1123_buffer` | `ptime->year > 9999` (RFC1123 limit) (`png.c:751`) | `return 0` (reject) | [x] |
| 32 | `png_convert_to_rfc1123_buffer` | `ptime->month == 0` (`png.c:752`) | `return 0` (reject) | [x] |
| 33 | `png_convert_to_rfc1123_buffer` | `ptime->month > 12` (`png.c:752`) | `return 0` (reject) | [x] |
| 34 | `png_convert_to_rfc1123_buffer` | `ptime->day == 0` (`png.c:753`) | `return 0` (reject) | [x] |
| 35 | `png_convert_to_rfc1123_buffer` | `ptime->day > 31` (`png.c:753`) | `return 0` (reject) | [x] |
| 36 | `png_convert_to_rfc1123_buffer` | `ptime->hour > 23` (`png.c:754`) | `return 0` (reject) | [x] |
| 37 | `png_convert_to_rfc1123_buffer` | `ptime->minute > 59` (`png.c:754`) | `return 0` (reject) | [x] |
| 38 | `png_convert_to_rfc1123_buffer` | `ptime->second > 60` (`png.c:755`) | `return 0` (reject) | [x] |
| 39 | `png_convert_to_rfc1123` | `png_ptr == NULL` (`png.c:798`) | `return NULL` | [x] |
| 40 | `png_convert_to_rfc1123` | `png_convert_to_rfc1123_buffer(...) == 0` (invalid `ptime`) (`png.c:801`) | `png_warning(png_ptr, "Ignoring invalid time value")` then `return NULL` | [x] |
| 41 | `png_build_grayscale_palette` | `palette == NULL` (`png.c:889`) | silent `return` | [x] |
| 42 | `png_build_grayscale_palette` | `bit_depth` not one of 1, 2, 4, 8 (`png.c:914`) | `num_palette = 0`, `color_inc = 0`; no palette entries written, silent no-op | [x] |
| 43 | `png_handle_as_unknown` | `png_ptr == NULL` or `chunk_name == NULL` or `png_ptr->num_chunk_list == 0` (`png.c:936`) | `return PNG_HANDLE_CHUNK_AS_DEFAULT` | [x] |
| 44 | `png_handle_as_unknown` | `chunk_name` not present in `png_ptr->chunk_list` (`png.c:960`) | `return PNG_HANDLE_CHUNK_AS_DEFAULT` | [x] |
| 45 | `png_reset_zstream` | `png_ptr == NULL` (`png.c:981`) | `return Z_STREAM_ERROR` | [x] |
| 46 | `png_reset_zstream` | `inflateReset(&png_ptr->zstream)` fails (uninitialized/bad zstream) (`png.c:985`) | returns zlib error code (e.g. `Z_STREAM_ERROR`) | [x] |
| 47 | `png_zstream_error` | `png_ptr->zstream.msg == NULL` and `ret` is `Z_OK` or unrecognized (`png.c:1011`) | `png_ptr->zstream.msg = "unexpected zlib return code"` | [x] |
| 48 | `png_zstream_error` | `ret == Z_STREAM_END` with no zlib msg (`png.c:1016`) | `png_ptr->zstream.msg = "unexpected end of LZ stream"` | [x] |
| 49 | `png_zstream_error` | `ret == Z_NEED_DICT` — deflate stream needs a dictionary (bogus PNG) (`png.c:1021`) | `png_ptr->zstream.msg = "missing LZ dictionary"` | [x] |
| 50 | `png_zstream_error` | `ret == Z_ERRNO` (`png.c:1028`) | `png_ptr->zstream.msg = "zlib IO error"` | [x] |
| 51 | `png_zstream_error` | `ret == Z_STREAM_ERROR` (internal libpng error) (`png.c:1033`) | `png_ptr->zstream.msg = "bad parameters to zlib"` | [x] |
| 52 | `png_zstream_error` | `ret == Z_DATA_ERROR` — corrupt compressed data (`png.c:1038`) | `png_ptr->zstream.msg = "damaged LZ stream"` | [x] |
| 53 | `png_zstream_error` | `ret == Z_MEM_ERROR` (`png.c:1042`) | `png_ptr->zstream.msg = "insufficient memory"` | [x] |
| 54 | `png_zstream_error` | `ret == Z_BUF_ERROR` — end of input/output (`png.c:1046`) | `png_ptr->zstream.msg = "truncated"` | [x] |
| 55 | `png_zstream_error` | `ret == Z_VERSION_ERROR` (`png.c:1053`) | `png_ptr->zstream.msg = "unsupported zlib version"` | [x] |
| 56 | `png_zstream_error` | `ret == PNG_UNEXPECTED_ZLIB_RETURN` (`png.c:1057`) | `png_ptr->zstream.msg = "unexpected zlib return"` | [x] |
| 57 | `png_fp_add` | `addend0 > 0 && 0x7fffffff - addend0 < addend1` — positive fixed-point overflow (`png.c:1080`) | `*error = 1`, `return PNG_FP_1/2` | [x] |
| 58 | `png_fp_add` | `addend0 < 0 && -0x7fffffff - addend0 > addend1` — negative fixed-point overflow (`png.c:1085`) | `*error = 1`, `return PNG_FP_1/2` | [x] |
| 59 | `png_fp_sub` | `addend1 > 0 && -0x7fffffff + addend1 > addend0` — negative overflow (`png.c:1101`) | `*error = 1`, `return PNG_FP_1/2` | [x] |
| 60 | `png_fp_sub` | `addend1 < 0 && 0x7fffffff + addend1 < addend0` — positive overflow (`png.c:1106`) | `*error = 1`, `return PNG_FP_1/2` | [x] |
| 61 | `png_safe_add` | either inner `png_fp_add` set `error` (`png.c:1123`) | `*addend0_and_result` left unmodified, `return 1` (overflow) | [x] |
| 62 | `png_xy_from_XYZ` | `png_safe_add(&d, XYZ->red_Y, XYZ->red_Z)` overflows (`png.c:1146`) | `return 1` (error) | [x] |
| 63 | `png_xy_from_XYZ` | `png_muldiv(&xy->redx, XYZ->red_X, PNG_FP_1, dred) == 0` — `dred == 0` or overflow (`png.c:1149`) | `return 1` (error) | [x] |
| 64 | `png_xy_from_XYZ` | `png_muldiv(&xy->redy, XYZ->red_Y, PNG_FP_1, dred) == 0` (`png.c:1151`) | `return 1` (error) | [x] |
| 65 | `png_xy_from_XYZ` | `png_safe_add(&d, XYZ->green_Y, XYZ->green_Z)` overflows (`png.c:1155`) | `return 1` (error) | [x] |
| 66 | `png_xy_from_XYZ` | `png_muldiv(&xy->greenx, XYZ->green_X, PNG_FP_1, dgreen) == 0` (`png.c:1158`) | `return 1` (error) | [x] |
| 67 | `png_xy_from_XYZ` | `png_muldiv(&xy->greeny, XYZ->green_Y, PNG_FP_1, dgreen) == 0` (`png.c:1160`) | `return 1` (error) | [x] |
| 68 | `png_xy_from_XYZ` | `png_safe_add(&d, XYZ->blue_Y, XYZ->blue_Z)` overflows (`png.c:1164`) | `return 1` (error) | [x] |
| 69 | `png_xy_from_XYZ` | `png_muldiv(&xy->bluex, XYZ->blue_X, PNG_FP_1, dblue) == 0` (`png.c:1167`) | `return 1` (error) | [x] |
| 70 | `png_xy_from_XYZ` | `png_muldiv(&xy->bluey, XYZ->blue_Y, PNG_FP_1, dblue) == 0` (`png.c:1169`) | `return 1` (error) | [x] |
| 71 | `png_xy_from_XYZ` | `png_safe_add(&d, dred, dgreen)` overflows computing `dwhite` (`png.c:1177`) | `return 1` (error) | [x] |
| 72 | `png_xy_from_XYZ` | `png_safe_add(&d, XYZ->green_X, XYZ->blue_X)` overflows computing `whiteX` (`png.c:1185`) | `return 1` (error) | [x] |
| 73 | `png_xy_from_XYZ` | `png_safe_add(&d, XYZ->green_Y, XYZ->blue_Y)` overflows computing `whiteY` (`png.c:1190`) | `return 1` (error) | [x] |
| 74 | `png_xy_from_XYZ` | `png_muldiv(&xy->whitex, whiteX, PNG_FP_1, dwhite) == 0` (`png.c:1194`) | `return 1` (error) | [x] |
| 75 | `png_xy_from_XYZ` | `png_muldiv(&xy->whitey, whiteY, PNG_FP_1, dwhite) == 0` (`png.c:1196`) | `return 1` (error) | [x] |
| 76 | `png_XYZ_from_xy` | `xy->redx < 0` or `xy->redx > fpLimit` where `fpLimit = PNG_FP_1+(PNG_FP_1/10)` = 110000 (`png.c:1223`) | `return 1` (invalid cHRM) | [x] |
| 77 | `png_XYZ_from_xy` | `xy->redy < 0` or `xy->redy > fpLimit - xy->redx` (`png.c:1224`) | `return 1` (invalid cHRM) | [x] |
| 78 | `png_XYZ_from_xy` | `xy->greenx < 0` or `xy->greenx > fpLimit` (`png.c:1225`) | `return 1` (invalid cHRM) | [x] |
| 79 | `png_XYZ_from_xy` | `xy->greeny < 0` or `xy->greeny > fpLimit - xy->greenx` (`png.c:1226`) | `return 1` (invalid cHRM) | [x] |
| 80 | `png_XYZ_from_xy` | `xy->bluex < 0` or `xy->bluex > fpLimit` (`png.c:1227`) | `return 1` (invalid cHRM) | [x] |
| 81 | `png_XYZ_from_xy` | `xy->bluey < 0` or `xy->bluey > fpLimit - xy->bluex` (`png.c:1228`) | `return 1` (invalid cHRM) | [x] |
| 82 | `png_XYZ_from_xy` | `xy->whitex < 0` or `xy->whitex > fpLimit` (`png.c:1229`) | `return 1` (invalid cHRM) | [x] |
| 83 | `png_XYZ_from_xy` | `xy->whitey < 5` (must be >= 5 to avoid overflow) or `xy->whitey > fpLimit - xy->whitex` (`png.c:1230`) | `return 1` (invalid cHRM) | [x] |
| 84 | `png_XYZ_from_xy` | `png_muldiv(&left, xy->greenx-xy->bluex, xy->redy-xy->bluey, 8) == 0` (`png.c:1422`) | `return 1` (error) | [x] |
| 85 | `png_XYZ_from_xy` | `png_muldiv(&right, xy->greeny-xy->bluey, xy->redx-xy->bluex, 8) == 0` (`png.c:1424`) | `return 1` (error) | [x] |
| 86 | `png_XYZ_from_xy` | `png_fp_sub(left, right, &error)` overflows computing `denominator` (`png.c:1427`) | `return 1` (error) | [x] |
| 87 | `png_XYZ_from_xy` | `png_muldiv(&left, xy->greenx-xy->bluex, xy->whitey-xy->bluey, 8) == 0` (`png.c:1431`) | `return 1` (error) | [x] |
| 88 | `png_XYZ_from_xy` | `png_muldiv(&right, xy->greeny-xy->bluey, xy->whitex-xy->bluex, 8) == 0` (`png.c:1433`) | `return 1` (error) | [x] |
| 89 | `png_XYZ_from_xy` | `png_muldiv(&red_inverse, xy->whitey, denominator, png_fp_sub(left,right,&error)) == 0` or `error` or `red_inverse <= xy->whitey` — extreme cHRM values (`png.c:1442`) | `return 1` (error) | [x] |
| 90 | `png_XYZ_from_xy` | `png_muldiv(&left, xy->redy-xy->bluey, xy->whitex-xy->bluex, 8) == 0` (`png.c:1448`) | `return 1` (error) | [x] |
| 91 | `png_XYZ_from_xy` | `png_muldiv(&right, xy->redx-xy->bluex, xy->whitey-xy->bluey, 8) == 0` (`png.c:1450`) | `return 1` (error) | [x] |
| 92 | `png_XYZ_from_xy` | `png_muldiv(&green_inverse, ...) == 0` or `error` or `green_inverse <= xy->whitey` (`png.c:1452`) | `return 1` (error) | [x] |
| 93 | `png_XYZ_from_xy` | `error` set by the `png_fp_sub`/`png_reciprocal` chain, or `blue_scale <= 0` (`png.c:1463`) | `return 1` (error) | [x] |
| 94 | `png_XYZ_from_xy` | `png_muldiv(&XYZ->red_X, xy->redx, PNG_FP_1, red_inverse) == 0` (`png.c:1471`) | `return 1` (error) | [x] |
| 95 | `png_XYZ_from_xy` | `png_muldiv(&XYZ->red_Y, xy->redy, PNG_FP_1, red_inverse) == 0` (`png.c:1473`) | `return 1` (error) | [x] |
| 96 | `png_XYZ_from_xy` | `png_muldiv(&XYZ->red_Z, PNG_FP_1-xy->redx-xy->redy, PNG_FP_1, red_inverse) == 0` (`png.c:1475`) | `return 1` (error) | [x] |
| 97 | `png_XYZ_from_xy` | `png_muldiv(&XYZ->green_X, xy->greenx, PNG_FP_1, green_inverse) == 0` (`png.c:1479`) | `return 1` (error) | [x] |
| 98 | `png_XYZ_from_xy` | `png_muldiv(&XYZ->green_Y, xy->greeny, PNG_FP_1, green_inverse) == 0` (`png.c:1481`) | `return 1` (error) | [x] |
| 99 | `png_XYZ_from_xy` | `png_muldiv(&XYZ->green_Z, PNG_FP_1-xy->greenx-xy->greeny, PNG_FP_1, green_inverse) == 0` (`png.c:1483`) | `return 1` (error) | [x] |
| 100 | `png_XYZ_from_xy` | `png_muldiv(&XYZ->blue_X, xy->bluex, blue_scale, PNG_FP_1) == 0` (`png.c:1487`) | `return 1` (error) | [x] |
| 101 | `png_XYZ_from_xy` | `png_muldiv(&XYZ->blue_Y, xy->bluey, blue_scale, PNG_FP_1) == 0` (`png.c:1489`) | `return 1` (error) | [x] |
| 102 | `png_XYZ_from_xy` | `png_muldiv(&XYZ->blue_Z, PNG_FP_1-xy->bluex-xy->bluey, blue_scale, PNG_FP_1) == 0` (`png.c:1491`) | `return 1` (error) | [x] |
| 103 | `png_icc_profile_error` | called for any ICC profile defect (`png.c:1571`) | `png_chunk_benign_error(png_ptr, "profile '<name>': <tag-or-hex>: <reason>")` then `return 0` | [x] |
| 104 | `icc_check_length` | `profile_length < 132` — iCCP profile shorter than the ICC header (`png.c:1588`) | `png_icc_profile_error(..., "too short")` -> `png_chunk_benign_error`, `return 0` | [x] |
| 105 | `png_icc_check_length` | `icc_check_length(...)` failed, i.e. `profile_length < 132` (`png.c:1597`) | `return 0` | [x] |
| 106 | `png_icc_check_length` | `profile_length > png_chunk_max(png_ptr)` — over the chunk-malloc limit (`png.c:1606`) | `png_icc_profile_error(..., "profile too long")`, `return 0` | [x] |
| 107 | `png_icc_check_header` | `png_get_uint_32(profile) != profile_length` (`png.c:1626`) | `png_icc_profile_error(..., "length does not match profile")`, `return 0` | [x] |
| 108 | `png_icc_check_header` | `profile[8] > 3 && (profile_length & 3) != 0` — major version > 3 with unaligned length (`png.c:1631`) | `png_icc_profile_error(..., "invalid length")`, `return 0` | [x] |
| 109 | `png_icc_check_header` | tag count `png_get_uint_32(profile+128) > 357913930` (max possible `(2^32-4-132)/12`) (`png.c:1636`) | `png_icc_profile_error(..., "tag count too large")`, `return 0` | [x] |
| 110 | `png_icc_check_header` | `profile_length < 132 + 12*tag_count` — truncated tag table (`png.c:1637`) | `png_icc_profile_error(..., "tag count too large")`, `return 0` | [x] |
| 111 | `png_icc_check_header` | rendering intent `png_get_uint_32(profile+64) >= 0xffff` (ICC limit) (`png.c:1645`) | `png_icc_profile_error(..., "invalid rendering intent")`, `return 0` | [x] |
| 112 | `png_icc_check_header` | rendering intent `>= PNG_sRGB_INTENT_LAST` (`png.c:1652`) | `(void)png_icc_profile_error(..., "intent outside defined range")` -> warning only, checking continues | [x] |
| 113 | `png_icc_check_header` | `png_get_uint_32(profile+36) != 0x61637370` (`'acsp'` signature) (`png.c:1669`) | `png_icc_profile_error(..., "invalid signature")`, `return 0` | [x] |
| 114 | `png_icc_check_header` | `memcmp(profile+68, D50_nCIEXYZ, 12) != 0` — PCS illuminant not D50 (`png.c:1680`) | `(void)png_icc_profile_error(..., "PCS illuminant is not D50")` -> warning only, continues | [x] |
| 115 | `png_icc_check_header` | data colour space `'RGB '` (`0x52474220`) but `(color_type & PNG_COLOR_MASK_COLOR) == 0` (`png.c:1708`) | `png_icc_profile_error(..., "RGB color space not permitted on grayscale PNG")`, `return 0` | [x] |
| 116 | `png_icc_check_header` | data colour space `'GRAY'` (`0x47524159`) but `(color_type & PNG_COLOR_MASK_COLOR) != 0` (`png.c:1714`) | `png_icc_profile_error(..., "Gray color space not permitted on RGB PNG")`, `return 0` | [x] |
| 117 | `png_icc_check_header` | data colour space is neither `'RGB '` nor `'GRAY'` (`png.c:1720`) | `png_icc_profile_error(..., "invalid ICC profile color space")`, `return 0` | [x] |
| 118 | `png_icc_check_header` | profile/device class `'abst'` (`0x61627374`) (`png.c:1745`) | `png_icc_profile_error(..., "invalid embedded Abstract ICC profile")`, `return 0` | [x] |
| 119 | `png_icc_check_header` | profile/device class `'link'` (`0x6c696e6b`) (`png.c:1755`) | `png_icc_profile_error(..., "unexpected DeviceLink ICC profile class")`, `return 0` | [x] |
| 120 | `png_icc_check_header` | profile/device class `'nmcl'` (`0x6e6d636c`) (`png.c:1763`) | `(void)png_icc_profile_error(..., "unexpected NamedColor ICC profile class")` -> warning only, continues | [x] |
| 121 | `png_icc_check_header` | profile/device class not in {`'scnr'`,`'mntr'`,`'prtr'`,`'spac'`,`'abst'`,`'link'`,`'nmcl'`} (`png.c:1773`) | `(void)png_icc_profile_error(..., "unrecognized ICC profile class")` -> warning only, continues | [x] |
| 122 | `png_icc_check_header` | PCS `png_get_uint_32(profile+20)` is neither `'XYZ '` (`0x58595a20`) nor `'Lab '` (`0x4c616220`) (`png.c:1789`) | `png_icc_profile_error(..., "unexpected ICC PCS encoding")`, `return 0` | [x] |
| 123 | `png_icc_check_tag_table` | for any tag: `tag_start > profile_length` or `tag_length > profile_length - tag_start` — tag data outside the profile (`png.c:1824`) | `png_icc_profile_error(..., "ICC profile tag outside profile")`, `return 0` | [x] |
| 124 | `png_icc_check_tag_table` | `(tag_start & 3) != 0` — tag start not 4-byte aligned (`png.c:1828`) | `(void)png_icc_profile_error(..., "ICC profile tag start not a multiple of 4")` -> warning only, `return 1` | [x] |
| 125 | `png_set_rgb_coefficients` | `have_chromaticities(png_ptr) == 0` or `png_XYZ_from_xy(&xyz, &png_ptr->chromaticities) != 0` (`png.c:1897`) | silently falls back to REC 709 defaults `red=6968`, `green=23434` | [x] |
| 126 | `png_set_rgb_coefficients` | `total <= 0`, or any `png_muldiv` fails, or any of `r`,`g`,`b` outside `0..32768`, or `r+g+b > 32769` (`png.c:1908`) | coefficients left unset (silently ignores the cHRM-derived values) | [x] |
| 127 | `png_set_rgb_coefficients` | after rounding adjustment `r+g+b != 32768` (`png.c:1937`) | `png_error(png_ptr, "internal error handling cHRM coefficients")` -> error, no return | [x] |
| 128 | `png_check_IHDR` | `width == 0` (`png.c:1969`) | `png_warning(png_ptr, "Image width is zero in IHDR")`, `error = 1` -> final `png_error` | [x] |
| 129 | `png_check_IHDR` | `width > PNG_UINT_31_MAX` (`png.c:1975`) | `png_warning(png_ptr, "Invalid image width in IHDR")`, `error = 1` | [x] |
| 130 | `png_check_IHDR` | `((width + 7) & ~(png_alloc_size_t)7) > ((PNG_SIZE_MAX - 48 - 1)/8) - 1` — row buffer would exceed `size_t` (`png.c:1989`) | `png_warning(png_ptr, "Image width is too large for this architecture")`, `error = 1` | [x] |
| 131 | `png_check_IHDR` | `width > png_ptr->user_width_max` (or `PNG_USER_WIDTH_MAX`) (`png.c:2012`) | `png_warning(png_ptr, "Image width exceeds user limit in IHDR")`, `error = 1` | [x] |
| 132 | `png_check_IHDR` | `height == 0` (`png.c:2021`) | `png_warning(png_ptr, "Image height is zero in IHDR")`, `error = 1` | [x] |
| 133 | `png_check_IHDR` | `height > PNG_UINT_31_MAX` (`png.c:2027`) | `png_warning(png_ptr, "Invalid image height in IHDR")`, `error = 1` | [x] |
| 134 | `png_check_IHDR` | `height > png_ptr->user_height_max` (or `PNG_USER_HEIGHT_MAX`) (`png.c:2034`) | `png_warning(png_ptr, "Image height exceeds user limit in IHDR")`, `error = 1` | [x] |
| 135 | `png_check_IHDR` | `bit_depth` not in {1,2,4,8,16} (`png.c:2044`) | `png_warning(png_ptr, "Invalid bit depth in IHDR")`, `error = 1` | [x] |
| 136 | `png_check_IHDR` | `color_type < 0` or `color_type == 1` or `color_type == 5` or `color_type > 6` (`png.c:2051`) | `png_warning(png_ptr, "Invalid color type in IHDR")`, `error = 1` | [x] |
| 137 | `png_check_IHDR` | palette color type with `bit_depth > 8`, or RGB/GA/RGBA with `bit_depth < 8` (`png.c:2058`) | `png_warning(png_ptr, "Invalid color type/bit depth combination in IHDR")`, `error = 1` | [x] |
| 138 | `png_check_IHDR` | `interlace_type >= PNG_INTERLACE_LAST` (`png.c:2067`) | `png_warning(png_ptr, "Unknown interlace method in IHDR")`, `error = 1` | [x] |
| 139 | `png_check_IHDR` | `compression_type != PNG_COMPRESSION_TYPE_BASE` (`png.c:2073`) | `png_warning(png_ptr, "Unknown compression method in IHDR")`, `error = 1` | [x] |
| 140 | `png_check_IHDR` | `(png_ptr->mode & PNG_HAVE_PNG_SIGNATURE) != 0 && png_ptr->mng_features_permitted != 0` (`png.c:2089`) | `png_warning(png_ptr, "MNG features are not allowed in a PNG datastream")` -> warning only, `error` not set | [x] |
| 141 | `png_check_IHDR` | `filter_type != PNG_FILTER_TYPE_BASE` and not a permitted MNG intrapixel-differencing case (`png.c:2093`) | `png_warning(png_ptr, "Unknown filter method in IHDR")`, `error = 1` | [x] |
| 142 | `png_check_IHDR` | `filter_type != PNG_FILTER_TYPE_BASE` while `(png_ptr->mode & PNG_HAVE_PNG_SIGNATURE) != 0` (`png.c:2105`) | `png_warning(png_ptr, "Invalid filter method in IHDR")`, `error = 1` | [x] |
| 143 | `png_check_IHDR` | (non-MNG build) `filter_type != PNG_FILTER_TYPE_BASE` (`png.c:2113`) | `png_warning(png_ptr, "Unknown filter method in IHDR")`, `error = 1` | [x] |
| 144 | `png_check_IHDR` | `error == 1` after any of the above (`png.c:2120`) | `png_error(png_ptr, "Invalid IHDR data")` -> error, no return | [x] |
| 145 | `png_check_fp_number` | current char is not in `+-.0123456789Ee` (`png.c:2155`) | `goto PNG_FP_End`; number ends at `*whereami`, returns `(state & PNG_FP_SAW_DIGIT) != 0` | [x] |
| 146 | `png_check_fp_number` | sign in integer part after something was already seen: `PNG_FP_INTEGER + PNG_FP_SAW_SIGN` with `(state & PNG_FP_SAW_ANY) != 0` (`png.c:2165`) | `goto PNG_FP_End` (character rejected as part of the number) | [x] |
| 147 | `png_check_fp_number` | second `.`: `PNG_FP_INTEGER + PNG_FP_SAW_DOT` with `(state & PNG_FP_SAW_DOT) != 0` (`png.c:2173`) | `goto PNG_FP_End` | [x] |
| 148 | `png_check_fp_number` | `E`/`e` with no preceding digit: `PNG_FP_INTEGER + PNG_FP_SAW_E` and `(state & PNG_FP_SAW_DIGIT) == 0` (`png.c:2193`) | `goto PNG_FP_End` | [x] |
| 149 | `png_check_fp_number` | `.E` with no digits: `PNG_FP_FRACTION + PNG_FP_SAW_E` and `(state & PNG_FP_SAW_DIGIT) == 0` (`png.c:2215`) | `goto PNG_FP_End` | [x] |
| 150 | `png_check_fp_number` | second sign in exponent: `PNG_FP_EXPONENT + PNG_FP_SAW_SIGN` with `(state & PNG_FP_SAW_ANY) != 0` (`png.c:2223`) | `goto PNG_FP_End` | [x] |
| 151 | `png_check_fp_number` | any other state/character-type combination (e.g. sign or dot inside the fraction/exponent) (`png.c:2241`) | `goto PNG_FP_End` | [x] |
| 152 | `png_check_fp_number` | no digit was ever seen: `(state & PNG_FP_SAW_DIGIT) == 0` (`png.c:2255`) | `return 0` (not a number) | [x] |
| 153 | `png_check_fp_string` | `png_check_fp_number(...) == 0` — string is not a valid fp number (`png.c:2266`) | `return 0` (fail) | [x] |
| 154 | `png_check_fp_string` | trailing garbage: `char_index != size && string[char_index] != 0` (`png.c:2267`) | `return 0` (fail) | [x] |
| 155 | `png_pow10` | `power < DBL_MIN_10_EXP` — exponent underflows `double` (`png.c:2290`) | `return 0` | [x] |
| 156 | `png_ascii_from_fp` | `precision < 1` (`png.c:2325`) | silently clamped: `precision = DBL_DIG` | [x] |
| 157 | `png_ascii_from_fp` | `precision > DBL_DIG+1` (`png.c:2329`) | silently clamped: `precision = DBL_DIG+1` | [x] |
| 158 | `png_ascii_from_fp` | `size < precision+5` — output buffer too small (`png.c:2333`) | falls through to `png_error(png_ptr, "ASCII conversion buffer too small")` | [x] |
| 159 | `png_ascii_from_fp` | `!(fp >= DBL_MIN)` — value underflows / is zero (`png.c:2618`) | writes `"0"` and returns (no error) | [x] |
| 160 | `png_ascii_from_fp` | `fp > DBL_MAX` or NaN (neither `>= DBL_MIN && <= DBL_MAX` nor `< DBL_MIN`) (`png.c:2624`) | writes `"inf"` and returns (no error) | [x] |
| 161 | `png_ascii_from_fp` | `size <= cdigits` when the exponent digits must still be emitted (`png.c:2608`) | falls through to `png_error(png_ptr, "ASCII conversion buffer too small")` | [x] |
| 162 | `png_ascii_from_fp` | reached end of function without returning (buffer too small) (`png.c:2635`) | `png_error(png_ptr, "ASCII conversion buffer too small")` -> error, no return | [x] |
| 163 | `png_ascii_from_fixed` | `size <= 12` — buffer smaller than 13 bytes (`png.c:2649`) | falls through to `png_error(png_ptr, "ASCII conversion buffer too small")` | [x] |
| 164 | `png_ascii_from_fixed` | `num > 0x80000000` after negation — `fp` magnitude overflows `png_uint_32` (`png.c:2661`) | falls through to `png_error(png_ptr, "ASCII conversion buffer too small")` | [x] |
| 165 | `png_ascii_from_fixed` | reached end of function without returning (`png.c:2713`) | `png_error(png_ptr, "ASCII conversion buffer too small")` -> error, no return | [x] |
| 166 | `png_fixed` | `floor(100000*fp+.5) > 2147483647.` or `< -2147483648.` (`png.c:2730`) | `png_fixed_error(png_ptr, text)` -> `png_error(png_ptr, "fixed point overflow in <text>")`, no return | [x] |
| 167 | `png_fixed_ITU` | `floor(10000*fp+.5) > 2147483647.` or `< 0` (`png.c:2749`) | `png_fixed_error(png_ptr, text)` -> `png_error(png_ptr, "fixed point overflow in <text>")`, no return | [x] |
| 168 | `png_muldiv` | `divisor == 0` (`png.c:2774`) | `return 0` (failure), `*res` unmodified | [x] |
| 169 | `png_muldiv` | (floating build) `floor(a*times/divisor+.5) > 2147483647.` or `< -2147483648.` (`png.c:2790`) | falls through to `return 0` (overflow) | [x] |
| 170 | `png_muldiv` | (integer build) `s32 >= D` — the 64-bit product overflows the 32-bit quotient (`png.c:2832`) | falls through to `return 0` (overflow) | [x] |
| 171 | `png_muldiv` | (integer build) sign of `result` inconsistent with `negative` after rounding — overflow (`png.c:2870`) | falls through to `return 0` | [x] |
| 172 | `png_reciprocal` | `a == 0` (divide by zero) or `floor(1E10/a+.5)` outside `png_fixed_point` range, or `png_muldiv(&res,100000,100000,a) == 0` (`png.c:2889`, `png.c:2896`) | `return 0` (error/overflow) | [x] |
| 173 | `png_product2` | `floor(a*1E-5*b+.5)` out of `png_fixed_point` range, or `png_muldiv(&res,a,b,100000) == 0` (`png.c:2938`, `png.c:2943`) | `return 0` (overflow) | [x] |
| 174 | `png_reciprocal2` | `a == 0` or `b == 0` (`png.c:2956`) | `return 0` (overflow/error) | [x] |
| 175 | `png_reciprocal2` | `floor(1E15/a/b+.5) > 2147483647.` or `< -2147483648.` (`png.c:2962`) | `return 0` (overflow) | [x] |
| 176 | `png_reciprocal2` | (integer build) `png_product2(a, b) == 0` (`png.c:2973`) | `return 0` (overflow) | [x] |
| 177 | `png_log8bit` | `(x &= 0xff) == 0` — log of zero (`png.c:3057`) | `return -1` (overflow marker) | [x] |
| 178 | `png_log16bit` | `(x &= 0xffff) == 0` — log of zero (`png.c:3110`) | `return -1` (overflow marker) | [x] |
| 179 | `png_exp` | `x <= 0` — exponent overflow (`png.c:3237`) | `return png_32bit_exp[0]` (saturates at max 32-bit value) | [x] |
| 180 | `png_exp` | `x > 0xfffff` — exponent underflow (`png.c:3241`) | `return 0` | [x] |
| 181 | `png_gamma_8bit_correct` | `value == 0` or `value >= 255` (`png.c:3275`) | no gamma applied; `return (png_byte)(value & 0xff)` | [x] |
| 182 | `png_gamma_8bit_correct` | (integer build) `png_muldiv(&res, gamma_val, lg2, PNG_FP_1) == 0` — overflow (`png.c:3308`) | `value = 0`, `return 0` | [x] |
| 183 | `png_gamma_16bit_correct` | `value == 0` or `value >= 65535` (`png.c:3323`) | no gamma applied; `return (png_uint_16)value` | [x] |
| 184 | `png_gamma_16bit_correct` | (integer build) `png_muldiv(&res, gamma_val, lg2, PNG_FP_1) == 0` — overflow (`png.c:3338`) | `value = 0`, `return 0` | [x] |
| 185 | `png_gamma_correct` | `png_ptr->bit_depth != 8` and `PNG_16BIT_SUPPORTED` is not defined (`png.c:3367`) | `return 0` ("should not reach this") | [x] |
| 186 | `png_build_gamma_table` | `png_ptr->gamma_table != NULL` or `png_ptr->gamma_16_table != NULL` — table built twice (`png.c:3632`) | `png_warning(png_ptr, "gamma table being rebuilt")` then `png_destroy_gamma_table()` and rebuild | [x] |
| 187 | `png_build_gamma_table` | `sig_bit == 0` or `sig_bit >= 16U` — out-of-range sBIT (`png.c:3713`) | `shift = 0` (all 16 bits kept) | [x] |
| 188 | `png_build_gamma_table` | `shift < (16U - PNG_MAX_GAMMA_8)` while 16-to-8 transform requested (`png.c:3726`) | clamped: `shift = 16U - PNG_MAX_GAMMA_8` | [x] |
| 189 | `png_build_gamma_table` | `shift > 8U` (`png.c:3730`) | clamped: `shift = 8U` (guarantees at least one table) | [x] |
| 190 | `png_set_option` | `png_ptr == NULL` (`png.c:3771`) | `return PNG_OPTION_INVALID` | [x] |
| 191 | `png_set_option` | `option < 0` (`png.c:3771`) | `return PNG_OPTION_INVALID` | [x] |
| 192 | `png_set_option` | `option >= PNG_OPTION_NEXT` (`png.c:3771`) | `return PNG_OPTION_INVALID` | [x] |
| 193 | `png_set_option` | `(option & 1) != 0` — odd (non-option) value (`png.c:3772`) | `return PNG_OPTION_INVALID` | [x] |
| 194 | `png_image_free_function` | `image->opaque->png_ptr == NULL` (`png.c:3968`) | `return 0` (failure) | [x] |
| 195 | `png_image_free_function` | `c.for_write != 0` but `PNG_SIMPLIFIED_WRITE_SUPPORTED` undefined (`png.c:4002`) | `png_error(c.png_ptr, "simplified write not supported")` -> error, no return | [x] |
| 196 | `png_image_free_function` | `c.for_write == 0` but `PNG_SIMPLIFIED_READ_SUPPORTED` undefined (`png.c:4010`) | `png_error(c.png_ptr, "simplified read not supported")` -> error, no return | [x] |
| 197 | `png_image_free` | `image == NULL`, or `image->opaque == NULL`, or `image->opaque->error_buf != NULL` (inside error handling) (`png.c:4025`) | silent no-op (`png_safe_execute` will free later) | [x] |
| 198 | `png_image_error` | called on any simplified-API failure (`png.c:4034`) | copies `error_message` into `image->message`, sets `PNG_IMAGE_ERROR` in `image->warning_or_error`, `png_image_free(image)`, `return 0` | [x] |
| 199 | `png_error` | any fatal error; `png_ptr != NULL && png_ptr->error_fn != NULL` (`pngerror.c:42`) | calls `png_ptr->error_fn(png_ptr, error_message)`; if it returns, `png_default_error()` which never returns | [x] |
| 200 | `png_error` | `png_ptr == NULL` or `png_ptr->error_fn == NULL` (`pngerror.c:42`) | `png_default_error(png_ptr, error_message)` -> prints and `png_longjmp(png_ptr, 1)`, never returns | [x] |
| 201 | `png_err` | (build without `PNG_ERROR_TEXT_SUPPORTED`) any fatal error (`pngerror.c:60`) | `error_fn(png_ptr, "")` then `png_default_error(png_ptr, "")`, never returns | [x] |
| 202 | `png_safecat` | `buffer == NULL` or `pos >= bufsize` (`pngerror.c:76`) | nothing written, `return pos` unchanged | [x] |
| 203 | `png_safecat` | appended string would exceed `bufsize-1` (`pngerror.c:79`) | silently truncated, `'\0'`-terminated | [x] |
| 204 | `png_format_number` | `format` not one of `PNG_NUMBER_FORMAT_{fixed,02u,u,02x,x}` (`pngerror.c:144`) | `number = 0` (error), loop terminates, returns partial/empty string | [x] |
| 205 | `png_warning` | `png_ptr == NULL` or `png_ptr->warning_fn == NULL` (`pngerror.c:180`) | `png_default_warning(png_ptr, warning_message)` -> `fprintf(stderr, "libpng warning: %s", ...)` | [x] |
| 206 | `png_warning_parameter` | `number <= 0` or `number > PNG_WARNING_PARAMETER_COUNT` (`pngerror.c:196`) | silently ignored, parameter not stored | [x] |
| 207 | `png_formatted_warning` | formatted message longer than `sizeof msg - 1` (191 bytes) (`pngerror.c:247`) | message silently truncated then passed to `png_warning` | [x] |
| 208 | `png_formatted_warning` | `@<digit>` where the digit index `>= PNG_WARNING_PARAMETER_COUNT` (`pngerror.c:266`) | not treated as a parameter; the character is copied literally | [x] |
| 209 | `png_benign_error` | `(png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0` and read struct with `chunk_name != 0` (`pngerror.c:313`) | `png_chunk_warning(png_ptr, error_message)` -> warning, processing continues | [x] |
| 210 | `png_benign_error` | `(png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0`, not a read struct or `chunk_name == 0` (`pngerror.c:318`) | `png_warning(png_ptr, error_message)` -> warning, processing continues | [x] |
| 211 | `png_benign_error` | `PNG_FLAG_BENIGN_ERRORS_WARN` clear and read struct with `chunk_name != 0` (`pngerror.c:326`) | `png_chunk_error(png_ptr, error_message)` -> fatal error, no return | [x] |
| 212 | `png_benign_error` | `PNG_FLAG_BENIGN_ERRORS_WARN` clear, not a read struct or `chunk_name == 0` (`pngerror.c:329`) | `png_error(png_ptr, error_message)` -> fatal error, no return | [x] |
| 213 | `png_app_warning` | `(png_ptr->flags & PNG_FLAG_APP_WARNINGS_WARN) != 0` (`pngerror.c:340`) | `png_warning(png_ptr, error_message)` -> warning | [x] |
| 214 | `png_app_warning` | `PNG_FLAG_APP_WARNINGS_WARN` clear (default: app misuse is fatal) (`pngerror.c:343`) | `png_error(png_ptr, error_message)` -> fatal error, no return | [x] |
| 215 | `png_app_error` | `(png_ptr->flags & PNG_FLAG_APP_ERRORS_WARN) != 0` (`pngerror.c:353`) | `png_warning(png_ptr, error_message)` -> warning | [x] |
| 216 | `png_app_error` | `PNG_FLAG_APP_ERRORS_WARN` clear (`pngerror.c:356`) | `png_error(png_ptr, error_message)` -> fatal error, no return | [x] |
| 217 | `png_format_buffer` | chunk name byte fails `isnonalpha(c)`, i.e. `c < 65 or c > 122 or (c > 90 and c < 97)` (`pngerror.c:391`) | byte rendered as `[HH]` hex escape in the message prefix | [x] |
| 218 | `png_format_buffer` | `error_message == NULL` (`pngerror.c:405`) | buffer holds only the chunk-name prefix, `'\0'`-terminated | [x] |
| 219 | `png_format_buffer` | `error_message` longer than `PNG_MAX_ERROR_TEXT-1` (195) (`pngerror.c:415`) | message silently truncated to 195 chars | [x] |
| 220 | `png_chunk_error` | `png_ptr == NULL` (`pngerror.c:430`) | `png_error(png_ptr, error_message)` (unprefixed), never returns | [x] |
| 221 | `png_chunk_error` | any chunk-level fatal error with `png_ptr != NULL` (`pngerror.c:435`) | `png_error(png_ptr, "<cHNK>: <error_message>")`, never returns | [x] |
| 222 | `png_chunk_warning` | `png_ptr == NULL` (`pngerror.c:446`) | `png_warning(png_ptr, warning_message)` (unprefixed) | [x] |
| 223 | `png_chunk_warning` | any chunk-level warning with `png_ptr != NULL` (`pngerror.c:451`) | `png_warning(png_ptr, "<cHNK>: <warning_message>")` | [x] |
| 224 | `png_chunk_benign_error` | `(png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0` (`pngerror.c:463`) | `png_chunk_warning(png_ptr, error_message)` -> warning, chunk ignored | [x] |
| 225 | `png_chunk_benign_error` | `PNG_FLAG_BENIGN_ERRORS_WARN` clear (default on read) (`pngerror.c:467`) | `png_chunk_error(png_ptr, error_message)` -> fatal error, no return | [x] |
| 226 | `png_chunk_report` | read struct and `error < PNG_CHUNK_ERROR` (`pngerror.c:492`) | `png_chunk_warning(png_ptr, message)` | [x] |
| 227 | `png_chunk_report` | read struct and `error >= PNG_CHUNK_ERROR` (`pngerror.c:496`) | `png_chunk_benign_error(png_ptr, message)` (warning or fatal per flags) | [x] |
| 228 | `png_chunk_report` | write struct and `error < PNG_CHUNK_WRITE_ERROR` (`pngerror.c:507`) | `png_app_warning(png_ptr, message)` | [x] |
| 229 | `png_chunk_report` | write struct and `error >= PNG_CHUNK_WRITE_ERROR` (`pngerror.c:510`) | `png_app_error(png_ptr, message)` | [x] |
| 230 | `png_fixed_error` | fixed-point conversion overflow reported by `png_fixed`/`png_fixed_ITU` (`pngerror.c:534`) | `png_error(png_ptr, "fixed point overflow in <name>")`, `name` truncated to `PNG_MAX_ERROR_TEXT-1`, never returns | [x] |
| 231 | `png_set_longjmp_fn` | `png_ptr == NULL` (`pngerror.c:557`) | `return NULL` | [x] |
| 232 | `png_set_longjmp_fn` | `jmp_buf_size > sizeof png_ptr->jmp_buf_local` and `png_malloc_warn` returns NULL (OOM) (`pngerror.c:572`) | `png_warning(..., "Out of memory")` from `png_malloc_warn`, then `return NULL` | [x] |
| 233 | `png_set_longjmp_fn` | `png_ptr->jmp_buf_size == 0` but `png_ptr->jmp_buf_ptr != &png_ptr->jmp_buf_local` — stale stack jmp_buf (internal error) (`pngerror.c:586`) | `png_error(png_ptr, "Libpng jmp_buf still allocated")` -> fatal, no return | [x] |
| 234 | `png_set_longjmp_fn` | `size != jmp_buf_size` — app changed its `jmp_buf` size between calls (`pngerror.c:598`) | `png_warning(png_ptr, "Application jmp_buf size changed")` then `return NULL` | [x] |
| 235 | `png_free_jmpbuf` | `png_ptr == NULL` (`pngerror.c:615`) | silent `return` | [x] |
| 236 | `png_free_jmpbuf` | `jb == NULL` or `png_ptr->jmp_buf_size == 0` (stack allocation) (`pngerror.c:622`) | no free performed; fields still zeroed | [x] |
| 237 | `png_default_error` | `error_message == NULL` (`pngerror.c:662`) | prints `"libpng error: undefined"` then `png_longjmp(png_ptr, 1)` | [x] |
| 238 | `png_default_error` | any fatal error reaching the default handler (`pngerror.c:668`) | `fprintf(stderr, "libpng error: %s", error_message)` then `png_longjmp(png_ptr, 1)`, never returns | [x] |
| 239 | `png_longjmp` | `png_ptr == NULL`, or `png_ptr->longjmp_fn == NULL`, or `png_ptr->jmp_buf_ptr == NULL` — no error-return path installed (`pngerror.c:676`) | falls through to `PNG_ABORT()` — process/thread terminated | [x] |
| 240 | `png_set_error_fn` | `png_ptr == NULL` (`pngerror.c:721`) | silent `return` | [x] |
| 241 | `png_get_error_ptr` | `png_ptr == NULL` (`pngerror.c:741`) | `return NULL` | [x] |
| 242 | `png_safe_error` | `image != NULL` and `image->opaque != NULL` and `image->opaque->error_buf != NULL` (`pngerror.c:782`) | logs `error_message` into `image->message`, sets `PNG_IMAGE_ERROR`, `longjmp(png_control_jmp_buf(image->opaque), 1)` | [x] |
| 243 | `png_safe_error` | `image != NULL` but `image->opaque == NULL` or `image->opaque->error_buf == NULL` — missing longjmp buffer (`pngerror.c:786`) | sets `image->message` to `"bad longjmp: <error_message>"` then `abort()` | [x] |
| 244 | `png_safe_error` | `image == NULL` (`error_ptr` not a `png_image`) (`pngerror.c:773`) | falls through to `abort()` | [x] |
| 245 | `png_safe_warning` | `image->warning_or_error != 0` — a prior warning/error already logged (`pngerror.c:806`) | new warning silently discarded | [x] |
| 246 | `png_safe_execute` | `function(arg)` returned false (`pngerror.c:829`) | `error_buf` restored; `png_image_free(image)` if `saved_error_buf == NULL`; `return 0` (failure) | [x] |
| 247 | `png_safe_execute` | `png_error` inside `function` longjmps back to `safe_jmpbuf` (`pngerror.c:821`) | `error_buf` restored; `png_image_free(image)` if `saved_error_buf == NULL`; `return 0` (failure) | [x] |
| 248 | `png_destroy_png_struct` | `png_ptr == NULL` (`pngmem.c:26`) | silent `return` | [x] |
| 249 | `png_calloc` | `png_malloc(png_ptr, size) == NULL` (only possible when `png_ptr == NULL`) (`pngmem.c:56`) | no `memset`, `return NULL` | [x] |
| 250 | `png_malloc_base` | `PNG_MAX_MALLOC_64K` build and `size > 65536U` (`pngmem.c:83`) | `return NULL` | [x] |
| 251 | `png_malloc_base` | `size > PNG_SIZE_MAX` — would truncate in the `(size_t)` cast to `malloc` (`pngmem.c:88`) | `return NULL` | [x] |
| 252 | `png_malloc_base` | user `malloc_fn` or system `malloc` returns NULL (`pngmem.c:92`, `pngmem.c:98`) | `return NULL` (caller must handle) | [x] |
| 253 | `png_malloc_array_checked` | `req > PNG_SIZE_MAX/element_size` — `nelements*element_size` overflows (`pngmem.c:113`) | `return NULL` (request too large) | [x] |
| 254 | `png_malloc_array` | `nelements <= 0` (`pngmem.c:125`) | `png_error(png_ptr, "internal error: array alloc")` -> fatal, no return | [x] |
| 255 | `png_malloc_array` | `element_size == 0` (`pngmem.c:125`) | `png_error(png_ptr, "internal error: array alloc")` -> fatal, no return | [x] |
| 256 | `png_realloc_array` | `add_elements <= 0` (`pngmem.c:137`) | `png_error(png_ptr, "internal error: array realloc")` -> fatal, no return | [x] |
| 257 | `png_realloc_array` | `element_size == 0` (`pngmem.c:137`) | `png_error(png_ptr, "internal error: array realloc")` -> fatal, no return | [x] |
| 258 | `png_realloc_array` | `old_elements < 0` (`pngmem.c:137`) | `png_error(png_ptr, "internal error: array realloc")` -> fatal, no return | [x] |
| 259 | `png_realloc_array` | `old_array == NULL && old_elements > 0` (`pngmem.c:138`) | `png_error(png_ptr, "internal error: array realloc")` -> fatal, no return | [x] |
| 260 | `png_realloc_array` | `add_elements > INT_MAX - old_elements` — element count overflows `int` (`pngmem.c:144`) | `return NULL` (error) | [x] |
| 261 | `png_realloc_array` | `png_malloc_array_checked(...) == NULL` — allocation failed or too large (`pngmem.c:149`) | `return NULL` (error) | [x] |
| 262 | `png_malloc` | `png_ptr == NULL` (`pngmem.c:178`) | `return NULL` | [x] |
| 263 | `png_malloc` | `png_malloc_base(png_ptr, size) == NULL` — OOM or `size > PNG_SIZE_MAX` (`pngmem.c:183`) | `png_error(png_ptr, "Out of memory")` -> fatal, no return | [x] |
| 264 | `png_malloc_default` | `png_ptr == NULL` (`pngmem.c:196`) | `return NULL` | [x] |
| 265 | `png_malloc_default` | `png_malloc_base(NULL, size) == NULL` (`pngmem.c:202`) | `png_error(png_ptr, "Out of Memory")` -> fatal, no return | [x] |
| 266 | `png_malloc_warn` | `png_ptr == NULL` (`pngmem.c:217`) | `return NULL` (no warning issued) | [x] |
| 267 | `png_malloc_warn` | `png_malloc_base(png_ptr, size) == NULL` (`pngmem.c:221`) | `png_warning(png_ptr, "Out of memory")` then `return NULL` | [x] |
| 268 | `png_free` | `png_ptr == NULL` or `ptr == NULL` (`pngmem.c:236`) | silent `return`, nothing freed | [x] |
| 269 | `png_free_default` | `png_ptr == NULL` or `ptr == NULL` (`pngmem.c:251`) | silent `return`, nothing freed | [x] |
| 270 | `png_set_mem_fn` | `png_ptr == NULL` (`pngmem.c:266`) | silent no-op | [x] |
| 271 | `png_get_mem_ptr` | `png_ptr == NULL` (`pngmem.c:281`) | `return NULL` | [x] |
| 272 | `png_read_data` | `png_ptr->read_data_fn == NULL` — no read function installed (`pngrio.c:35`) | `png_error(png_ptr, "Call to NULL read function")` -> fatal, no return | n/a |
| 273 | `png_default_read_data` | `png_ptr == NULL` (`pngrio.c:53`) | silent `return`, buffer left untouched | [x] |
| 274 | `png_default_read_data` | `fread(...) != length` — truncated input or stream error (`pngrio.c:61`) | `png_error(png_ptr, "Read Error")` -> fatal, no return | [x] |
| 275 | `png_set_read_fn` | `png_ptr == NULL` (`pngrio.c:89`) | silent `return` | [x] |
| 276 | `png_set_read_fn` | `read_data_fn == NULL` (`pngrio.c:95`) | falls back to `png_ptr->read_data_fn = png_default_read_data` (or stores NULL if no STDIO -> later "Call to NULL read function") | [x] |
| 277 | `png_set_read_fn` | `png_ptr->write_data_fn != NULL` — read fn set on a write struct (`pngrio.c:106`) | `write_data_fn` cleared to NULL and `png_warning(png_ptr, "Can't set both read_data_fn and write_data_fn in the same structure")` | [x] |
| 278 | `png_write_data` | `png_ptr->write_data_fn == NULL` — no write function installed (`pngwio.c:35`) | `png_error(png_ptr, "Call to NULL write function")` -> fatal, no return | n/a |
| 279 | `png_default_write_data` | `png_ptr == NULL` (`pngwio.c:54`) | silent `return`, nothing written | [x] |
| 280 | `png_default_write_data` | `fwrite(...) != length` — short write / stream error (`pngwio.c:59`) | `png_error(png_ptr, "Write Error")` -> fatal, no return | [x] |
| 281 | `png_flush` | `png_ptr->output_flush_fn == NULL` (`pngwio.c:72`) | silent no-op (nothing flushed) | [x] |
| 282 | `png_default_flush` | `png_ptr == NULL` (`pngwio.c:82`) | silent `return` | [x] |
| 283 | `png_set_write_fn` | `png_ptr == NULL` (`pngwio.c:124`) | silent `return` | [x] |
| 284 | `png_set_write_fn` | `write_data_fn == NULL` (`pngwio.c:130`) | falls back to `png_ptr->write_data_fn = png_default_write_data` (or stores NULL if no STDIO -> later "Call to NULL write function") | [x] |
| 285 | `png_set_write_fn` | `output_flush_fn == NULL` (`pngwio.c:142`) | falls back to `png_ptr->output_flush_fn = png_default_flush` (or stores NULL if no STDIO) | [x] |
| 286 | `png_set_write_fn` | `png_ptr->read_data_fn != NULL` — write fn set on a read struct (`pngwio.c:157`) | `read_data_fn` cleared to NULL and `png_warning(png_ptr, "Can't set both read_data_fn and write_data_fn in the same structure")` | [x] |

## pngget.c / pngset.c / pngtrans.c

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 287 | `png_get_valid` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:22`, `:36`) | `return 0` | [x] |
| 288 | `png_get_valid` | `flag == PNG_INFO_tRNS && png_ptr->num_trans == 0` (tRNS canceled by `png_handle_PLTE`) (`pngget.c:29-30`) | `return 0` | [x] |
| 289 | `png_get_valid` | requested `flag` bit clear in `info_ptr->valid` (`pngget.c:33`) | `return info_ptr->valid & flag` == `0` | [x] |
| 290 | `png_get_rowbytes` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:42-45`) | `return 0` | [x] |
| 291 | `png_get_rows` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:52-55`) | `return 0` (NULL row-pointer array) | [x] |
| 292 | `png_get_image_width` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:64-67`) | `return 0` | [x] |
| 293 | `png_get_image_height` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:73-76`) | `return 0` | [x] |
| 294 | `png_get_bit_depth` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:82-85`) | `return 0` | [x] |
| 295 | `png_get_color_type` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:91-94`) | `return 0` | [x] |
| 296 | `png_get_filter_type` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:100-103`) | `return 0` | [x] |
| 297 | `png_get_interlace_type` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:109-112`) | `return 0` | [x] |
| 298 | `png_get_compression_type` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:118-121`) | `return 0` | [x] |
| 299 | `png_get_x_pixels_per_meter` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:131-132`) | `return 0` | [x] |
| 300 | `png_get_x_pixels_per_meter` | `info_ptr->phys_unit_type != PNG_RESOLUTION_METER` (`pngget.c:134`) | `return 0` | [x] |
| 301 | `png_get_y_pixels_per_meter` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:152-153`) | `return 0` | [x] |
| 302 | `png_get_y_pixels_per_meter` | `info_ptr->phys_unit_type != PNG_RESOLUTION_METER` (`pngget.c:155`) | `return 0` | [x] |
| 303 | `png_get_pixels_per_meter` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:172-173`) | `return 0` | [x] |
| 304 | `png_get_pixels_per_meter` | `info_ptr->phys_unit_type != PNG_RESOLUTION_METER` (`pngget.c:175`) | `return 0` | [x] |
| 305 | `png_get_pixels_per_meter` | `info_ptr->x_pixels_per_unit != info_ptr->y_pixels_per_unit` (non-square pixels) (`pngget.c:176`) | `return 0` | [x] |
| 306 | `png_get_pixel_aspect_ratio` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:195-196`) | `return (float)0.0` | [x] |
| 307 | `png_get_pixel_aspect_ratio` | `info_ptr->x_pixels_per_unit == 0` (divide-by-zero guard) (`pngget.c:198`) | `return (float)0.0` | [x] |
| 308 | `png_get_pixel_aspect_ratio_fixed` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:219-220`) | `return 0` | [x] |
| 309 | `png_get_pixel_aspect_ratio_fixed` | `info_ptr->x_pixels_per_unit <= 0 \|\| info_ptr->y_pixels_per_unit <= 0` (`pngget.c:221`) | `return 0` | [x] |
| 310 | `png_get_pixel_aspect_ratio_fixed` | `info_ptr->x_pixels_per_unit > PNG_UINT_31_MAX \|\| info_ptr->y_pixels_per_unit > PNG_UINT_31_MAX` (cast-overflow guard) (`pngget.c:222-223`) | `return 0` | [x] |
| 311 | `png_get_pixel_aspect_ratio_fixed` | `png_muldiv(&res, y_pixels_per_unit, PNG_FP_1, x_pixels_per_unit) == 0` (fixed-point overflow) (`pngget.c:230-231`) | `return 0` | [x] |
| 312 | `png_get_x_offset_microns` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_oFFs) == 0` (`pngget.c:249-250`) | `return 0` | [x] |
| 313 | `png_get_x_offset_microns` | `info_ptr->offset_unit_type != PNG_OFFSET_MICROMETER` (`pngget.c:252`) | `return 0` | [x] |
| 314 | `png_get_y_offset_microns` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_oFFs) == 0` (`pngget.c:269-270`) | `return 0` | [x] |
| 315 | `png_get_y_offset_microns` | `info_ptr->offset_unit_type != PNG_OFFSET_MICROMETER` (`pngget.c:272`) | `return 0` | [x] |
| 316 | `png_get_x_offset_pixels` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_oFFs) == 0` (`pngget.c:289-290`) | `return 0` | [x] |
| 317 | `png_get_x_offset_pixels` | `info_ptr->offset_unit_type != PNG_OFFSET_PIXEL` (`pngget.c:292`) | `return 0` | [x] |
| 318 | `png_get_y_offset_pixels` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_oFFs) == 0` (`pngget.c:309-310`) | `return 0` | [x] |
| 319 | `png_get_y_offset_pixels` | `info_ptr->offset_unit_type != PNG_OFFSET_PIXEL` (`pngget.c:312`) | `return 0` | [x] |
| 320 | `ppi_from_ppm` (static; used by `png_get_pixels_per_inch`, `png_get_x_pixels_per_inch`, `png_get_y_pixels_per_inch`) | `ppm > PNG_UINT_31_MAX` (`pngget.c:347`) | `return 0` (overflow) | [x] |
| 321 | `ppi_from_ppm` | `png_muldiv(&result, (png_int_32)ppm, 127, 5000) == 0` (overflow) (`pngget.c:347-352`) | `return 0` | [x] |
| 322 | `png_fixed_inches_from_microns` (static; used by `png_get_x_offset_inches_fixed`, `png_get_y_offset_inches_fixed`) | `png_muldiv(&result, microns, 500, 127) == 0` (fixed-point overflow) (`pngget.c:385-389`) | `png_warning(png_ptr, "fixed point overflow ignored")`, `return 0` | [x] |
| 323 | `png_get_pHYs_dpi` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:442-443`) | `return 0` (retval untouched) | [x] |
| 324 | `png_get_pHYs_dpi` | all of `res_x`, `res_y`, `unit_type` are `NULL` (`pngget.c:445`, `:451`, `:457`) | `return 0` (no `PNG_INFO_pHYs` bit ever OR'ed in) | [x] |
| 325 | `png_get_channels` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:483-486`) | `return 0` | [x] |
| 326 | `png_get_signature` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:493-496`) | `return NULL` | [x] |
| 327 | `png_get_bKGD` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:507`) | `return 0` | [x] |
| 328 | `png_get_bKGD` | `(info_ptr->valid & PNG_INFO_bKGD) == 0` (no bKGD chunk) (`pngget.c:508`) | `return 0` | [x] |
| 329 | `png_get_bKGD` | `background == NULL` (`pngget.c:509`) | `return 0` | [x] |
| 330 | `png_get_cHRM` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:533`) | `return 0` | [x] |
| 331 | `png_get_cHRM` | `(info_ptr->valid & PNG_INFO_cHRM) == 0` (`pngget.c:534`) | `return 0` | [x] |
| 332 | `png_get_cHRM_XYZ` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:567`) | `return 0` | [x] |
| 333 | `png_get_cHRM_XYZ` | `(info_ptr->valid & PNG_INFO_cHRM) == 0` (`pngget.c:568`) | `return 0` | [x] |
| 334 | `png_get_cHRM_XYZ` | `png_XYZ_from_xy(&XYZ, &info_ptr->cHRM) != 0` (degenerate/unrepresentable chromaticities) (`pngget.c:569`) | `return 0` | [x] |
| 335 | `png_get_cHRM_XYZ_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:608`) | `return 0` | [x] |
| 336 | `png_get_cHRM_XYZ_fixed` | `(info_ptr->valid & PNG_INFO_cHRM) == 0U` (`pngget.c:609`) | `return 0` | [x] |
| 337 | `png_get_cHRM_XYZ_fixed` | `png_XYZ_from_xy(&XYZ, &info_ptr->cHRM) != 0` (`pngget.c:610`) | `return 0` | [x] |
| 338 | `png_get_cHRM_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:636`) | `return 0` | [x] |
| 339 | `png_get_cHRM_fixed` | `(info_ptr->valid & PNG_INFO_cHRM) == 0` (`pngget.c:637`) | `return 0` | [x] |
| 340 | `png_get_gAMA_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:664`) | `return 0` | [x] |
| 341 | `png_get_gAMA_fixed` | `(info_ptr->valid & PNG_INFO_gAMA) == 0` (`pngget.c:665`) | `return 0` | [x] |
| 342 | `png_get_gAMA` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:683`) | `return 0` | [x] |
| 343 | `png_get_gAMA` | `(info_ptr->valid & PNG_INFO_gAMA) == 0` (`pngget.c:684`) | `return 0` | [x] |
| 344 | `png_get_sRGB` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:704`) | `return 0` | [x] |
| 345 | `png_get_sRGB` | `(info_ptr->valid & PNG_INFO_sRGB) == 0` (`pngget.c:705`) | `return 0` | [x] |
| 346 | `png_get_iCCP` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:724`) | `return 0` | [x] |
| 347 | `png_get_iCCP` | `(info_ptr->valid & PNG_INFO_iCCP) == 0` (`pngget.c:725`) | `return 0` | [x] |
| 348 | `png_get_iCCP` | `name == NULL \|\| profile == NULL \|\| proflen == NULL` (`pngget.c:726`) | `return 0` | [x] |
| 349 | `png_get_sPLT` | `png_ptr == NULL \|\| info_ptr == NULL \|\| spalettes == NULL` (`pngget.c:750`) | `return 0` | [x] |
| 350 | `png_get_sPLT` | no sPLT stored, i.e. `info_ptr->splt_palettes_num == 0` (`pngget.c:753`) | `return 0` | [x] |
| 351 | `png_get_cICP` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:769`) | `return 0` | [x] |
| 352 | `png_get_cICP` | `(info_ptr->valid & PNG_INFO_cICP) == 0` (`pngget.c:770`) | `return 0` | [x] |
| 353 | `png_get_cICP` | `colour_primaries == NULL \|\| transfer_function == NULL \|\| matrix_coefficients == NULL \|\| video_full_range_flag == NULL` (`pngget.c:771-772`) | `return 0` | [x] |
| 354 | `png_get_cLLI_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:794`) | `return 0` | [x] |
| 355 | `png_get_cLLI_fixed` | `(info_ptr->valid & PNG_INFO_cLLI) == 0` (`pngget.c:795`) | `return 0` | [x] |
| 356 | `png_get_cLLI` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:813`) | `return 0` | [x] |
| 357 | `png_get_cLLI` | `(info_ptr->valid & PNG_INFO_cLLI) == 0` (`pngget.c:814`) | `return 0` | [x] |
| 358 | `png_get_mDCV_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:838`) | `return 0` | [x] |
| 359 | `png_get_mDCV_fixed` | `(info_ptr->valid & PNG_INFO_mDCV) == 0` (`pngget.c:839`) | `return 0` | [x] |
| 360 | `png_get_mDCV` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:867`) | `return 0` | [x] |
| 361 | `png_get_mDCV` | `(info_ptr->valid & PNG_INFO_mDCV) == 0` (`pngget.c:868`) | `return 0` | [x] |
| 362 | `png_get_eXIf` | any call (API permanently disabled) (`pngget.c:895-898`) | `png_warning(png_ptr, "png_get_eXIf does not work; use png_get_eXIf_1")`, `return 0` | [x] |
| 363 | `png_get_eXIf_1` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:907`) | `return 0` | [x] |
| 364 | `png_get_eXIf_1` | `(info_ptr->valid & PNG_INFO_eXIf) == 0` (`pngget.c:908`) | `return 0` | [x] |
| 365 | `png_get_eXIf_1` | `exif == NULL` (`pngget.c:908`) | `return 0` | [x] |
| 366 | `png_get_hIST` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:926`) | `return 0` | [x] |
| 367 | `png_get_hIST` | `(info_ptr->valid & PNG_INFO_hIST) == 0` (`pngget.c:927`) | `return 0` | [x] |
| 368 | `png_get_hIST` | `hist == NULL` (`pngget.c:927`) | `return 0` | [x] |
| 369 | `png_get_IHDR` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:945-946`) | `return 0` | [x] |
| 370 | `png_get_IHDR` | stored IHDR fields invalid (app tampered with `info_ptr` directly): re-validated via `png_check_IHDR(...)` (`pngget.c:974-976`) | `png_error` from `png_check_IHDR` (e.g. `"Invalid image width"`/`"Invalid bit depth"`) | [x] |
| 371 | `png_get_oFFs` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:988`) | `return 0` | [x] |
| 372 | `png_get_oFFs` | `(info_ptr->valid & PNG_INFO_oFFs) == 0` (`pngget.c:989`) | `return 0` | [x] |
| 373 | `png_get_oFFs` | `offset_x == NULL \|\| offset_y == NULL \|\| unit_type == NULL` (`pngget.c:990`) | `return 0` | [x] |
| 374 | `png_get_pCAL` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1010`) | `return 0` | [x] |
| 375 | `png_get_pCAL` | `(info_ptr->valid & PNG_INFO_pCAL) == 0` (`pngget.c:1011`) | `return 0` | [x] |
| 376 | `png_get_pCAL` | any of `purpose`, `X0`, `X1`, `type`, `nparams`, `units`, `params` is `NULL` (`pngget.c:1012-1013`) | `return 0` | [x] |
| 377 | `png_get_sCAL_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1039`) | `return 0` | [x] |
| 378 | `png_get_sCAL_fixed` | `(info_ptr->valid & PNG_INFO_sCAL) == 0` (`pngget.c:1040`) | `return 0` | [x] |
| 379 | `png_get_sCAL_fixed` | stored `scal_s_width`/`scal_s_height` not representable as fixed point (`png_fixed(png_ptr, atof(...), "sCAL width"/"sCAL height")`) (`pngget.c:1047-1049`) | `png_fixed_error` → `png_error(png_ptr, "fixed point overflow in sCAL width")` | [x] |
| 380 | `png_get_sCAL` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1064`) | `return 0` | [x] |
| 381 | `png_get_sCAL` | `(info_ptr->valid & PNG_INFO_sCAL) == 0` (`pngget.c:1065`) | `return 0` | [x] |
| 382 | `png_get_sCAL_s` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1082`) | `return 0` | [x] |
| 383 | `png_get_sCAL_s` | `(info_ptr->valid & PNG_INFO_sCAL) == 0` (`pngget.c:1083`) | `return 0` | [x] |
| 384 | `png_get_pHYs` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:1104-1105`) | `return 0` | [x] |
| 385 | `png_get_pHYs` | all of `res_x`, `res_y`, `unit_type` are `NULL` (`pngget.c:1107`, `:1113`, `:1119`) | `return 0` (retval stays 0) | [x] |
| 386 | `png_get_PLTE` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1136`) | `return 0` | [x] |
| 387 | `png_get_PLTE` | `(info_ptr->valid & PNG_INFO_PLTE) == 0` (`pngget.c:1137`) | `return 0` | [x] |
| 388 | `png_get_PLTE` | `palette == NULL` (`pngget.c:1137`) | `return 0` | [x] |
| 389 | `png_get_sBIT` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1155`) | `return 0` | [x] |
| 390 | `png_get_sBIT` | `(info_ptr->valid & PNG_INFO_sBIT) == 0` (`pngget.c:1156`) | `return 0` | [x] |
| 391 | `png_get_sBIT` | `sig_bit == NULL` (`pngget.c:1156`) | `return 0` | [x] |
| 392 | `png_get_text` | `png_ptr == NULL \|\| info_ptr == NULL \|\| info_ptr->num_text <= 0` (`pngget.c:1171`) | `*num_text = 0` if non-NULL (`:1185-1186`), `return 0` | [x] |
| 393 | `png_get_tIME` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1199`) | `return 0` | [x] |
| 394 | `png_get_tIME` | `(info_ptr->valid & PNG_INFO_tIME) == 0` (`pngget.c:1200`) | `return 0` | [x] |
| 395 | `png_get_tIME` | `mod_time == NULL` (`pngget.c:1200`) | `return 0` | [x] |
| 396 | `png_get_tRNS` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_tRNS) == 0` (`pngget.c:1219-1220`) | `return 0` | [x] |
| 397 | `png_get_tRNS` | `info_ptr->color_type == PNG_COLOR_TYPE_PALETTE && trans_alpha == NULL && num_trans == NULL` (`pngget.c:1222-1230`, `:1246`) | `return 0` (`PNG_INFO_tRNS` never OR'ed in) | [x] |
| 398 | `png_get_tRNS` | `info_ptr->color_type != PNG_COLOR_TYPE_PALETTE && trans_color == NULL && num_trans == NULL` (`pngget.c:1234-1244`, `:1246`) | `return 0` | [x] |
| 399 | `png_get_tRNS` | `info_ptr->color_type != PNG_COLOR_TYPE_PALETTE` with non-NULL `trans_alpha` (no per-palette alpha exists) (`pngget.c:1242-1243`) | `*trans_alpha = NULL` | [x] |
| 400 | `png_get_unknown_chunks` | `png_ptr == NULL \|\| info_ptr == NULL \|\| unknowns == NULL` (`pngget.c:1262`) | `return 0` | [x] |
| 401 | `png_get_unknown_chunks` | no stored unknown chunks, `info_ptr->unknown_chunks_num == 0` (`pngget.c:1265`) | `return 0` | [x] |
| 402 | `png_get_rgb_to_gray_status` | `png_ptr == NULL` (`pngget.c:1276`) | `return 0` | [x] |
| 403 | `png_get_user_chunk_ptr` | `png_ptr == NULL` (`pngget.c:1284`) | `return NULL` | [x] |
| 404 | `png_get_compression_buffer_size` | `png_ptr == NULL` (`pngget.c:1291-1292`) | `return 0` | [x] |
| 405 | `png_get_user_width_max` | `png_ptr == NULL` (`pngget.c:1317`) | `return 0` | [x] |
| 406 | `png_get_user_height_max` | `png_ptr == NULL` (`pngget.c:1323`) | `return 0` | [x] |
| 407 | `png_get_chunk_cache_max` | `png_ptr == NULL` (`pngget.c:1330`) | `return 0` | [x] |
| 408 | `png_get_chunk_malloc_max` | `png_ptr == NULL` (`pngget.c:1337`) | `return 0` | [x] |
| 409 | `png_get_palette_max` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1361-1364`) | `return -1` | [x] |
| 410 | `png_set_bKGD` | `png_ptr == NULL \|\| info_ptr == NULL \|\| background == NULL` (`pngset.c:29-30`) | silent `return`; `PNG_INFO_bKGD` not set | [x] |
| 411 | `png_set_cHRM_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:46-47`) | silent `return` | [x] |
| 412 | `png_set_cHRM_XYZ_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:74-75`) | silent `return` | [x] |
| 413 | `png_set_cHRM_XYZ_fixed` | `png_xy_from_XYZ(&xy, &XYZ) != 0` (XYZ values do not convert to valid xy chromaticities) (`pngset.c:87`, `:94`) | `png_app_error(png_ptr, "invalid cHRM XYZ")`; `PNG_INFO_cHRM` not set | [x] |
| 414 | `png_set_cHRM` | any of `white_x..blue_y` outside `±21474.83647` so `floor(100000*fp+.5)` exceeds `png_fixed_point` range (`pngset.c:104-111`) | `png_fixed_error` → `png_error(png_ptr, "fixed point overflow in cHRM White X")` (etc. per argument name) | [x] |
| 415 | `png_set_cHRM_XYZ` | any of `red_X..blue_Z` not representable as fixed point (`pngset.c:120-128`) | `png_error(png_ptr, "fixed point overflow in cHRM Red X")` (etc.) | [x] |
| 416 | `png_set_cICP` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:142-143`) | silent `return` | [x] |
| 417 | `png_set_cICP` | `matrix_coefficients != 0` (only identity matrix allowed in PNG) (`pngset.c:150-154`) | `png_warning(png_ptr, "Invalid cICP matrix coefficients")`; `PNG_INFO_cICP` not set | [x] |
| 418 | `png_set_cLLI_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:170-171`) | silent `return` | [x] |
| 419 | `png_set_cLLI_fixed` | `maxCLL > 0x7FFFFFFFU \|\| maxFALL > 0x7FFFFFFFU` (`pngset.c:174-185`) | `png_chunk_report(png_ptr, "cLLI light level exceeds PNG limit", PNG_CHUNK_WRITE_ERROR)`; chunk not stored | [x] |
| 420 | `png_set_cLLI` | `maxCLL` or `maxFALL` negative or `floor(10000*fp+.5) > 2147483647` (`pngset.c:197-199`) | `png_fixed_error` → `png_error(png_ptr, "fixed point overflow in png_set_cLLI(maxCLL)")` (or `(maxFALL)`) | [x] |
| 421 | `png_ITU_fixed_16` | `v/2 > 65535 \|\| v/2 < 0` after halving the fixed-point chromaticity (`pngset.c:215-219`) | `*error = 1`, `return 0` | [x] |
| 422 | `png_set_mDCV_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:238-239`) | silent `return` | [x] |
| 423 | `png_set_mDCV_fixed` | any of `white_x, white_y, red_x, red_y, green_x, green_y, blue_x, blue_y` rejected by `png_ITU_fixed_16` (`error != 0`) (`pngset.c:243-258`) | `png_chunk_report(png_ptr, "mDCV chromaticities outside representable range", PNG_CHUNK_WRITE_ERROR)`; chunk not stored | [x] |
| 424 | `png_set_mDCV_fixed` | `maxDL > 0x7FFFFFFFU \|\| minDL > 0x7FFFFFFFU` (`pngset.c:261-272`) | `png_chunk_report(png_ptr, "mDCV display light level exceeds PNG limit", PNG_CHUNK_WRITE_ERROR)`; chunk not stored | [x] |
| 425 | `png_set_mDCV` | any chromaticity double not representable as fixed point (`pngset.c:303-310`) | `png_error(png_ptr, "fixed point overflow in png_set_mDCV(white(x))")` (etc.) | [x] |
| 426 | `png_set_mDCV` | `maxDL`/`minDL` negative or `> 214748.3647` (`png_fixed_ITU`) (`pngset.c:311-312`) | `png_error(png_ptr, "fixed point overflow in png_set_mDCV(maxDL)")` (or `(minDL)`) | [x] |
| 427 | `png_set_eXIf` | any call (API permanently disabled) (`pngset.c:322`) | `png_warning(png_ptr, "png_set_eXIf does not work; use png_set_eXIf_1")`; nothing stored | [x] |
| 428 | `png_set_eXIf_1` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:335`) | silent `return` | [x] |
| 429 | `png_set_eXIf_1` | `(png_ptr->mode & PNG_WROTE_eXIf) != 0` (eXIf already written) (`pngset.c:336`) | silent `return` | [x] |
| 430 | `png_set_eXIf_1` | `exif == NULL` (`pngset.c:337`) | silent `return` | [x] |
| 431 | `png_set_eXIf_1` | `png_malloc_warn(png_ptr, num_exif)` returns `NULL` (out of memory / `num_exif` too large) (`pngset.c:340-346`) | `png_warning(png_ptr, "Insufficient memory for eXIf chunk data")`; `PNG_INFO_eXIf` not set | [x] |
| 432 | `png_set_gAMA_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:366-367`) | silent `return` | [x] |
| 433 | `png_set_gAMA` | `file_gamma` not representable as fixed point (`floor(100000*fp+.5)` out of `int32` range) (`pngset.c:377-378`) | `png_error(png_ptr, "fixed point overflow in png_set_gAMA")` | [x] |
| 434 | `png_set_hIST` | `png_ptr == NULL \|\| info_ptr == NULL \|\| hist == NULL` (`pngset.c:393-394`) | silent `return` | [x] |
| 435 | `png_set_hIST` | `info_ptr->num_palette == 0 \|\| info_ptr->num_palette > PNG_MAX_PALETTE_LENGTH` (hIST set before/with invalid PLTE) (`pngset.c:396-403`) | `png_warning(png_ptr, "Invalid palette size, hIST allocation skipped")`; `PNG_INFO_hIST` not set | [x] |
| 436 | `png_set_hIST` | `png_malloc_warn(...)` for `PNG_MAX_PALETTE_LENGTH` entries returns `NULL` (`pngset.c:417-424`) | `png_warning(png_ptr, "Insufficient memory for hIST chunk data")` | [x] |
| 437 | `png_set_IHDR` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:442-443`) | silent `return` | [x] |
| 438 | `png_set_IHDR` | any invalid IHDR field (`width`/`height` == 0 or > limits, `bit_depth` not in {1,2,4,8,16}, invalid `color_type`, bit-depth/color-type combination, `interlace_type` > 1, `compression_type != PNG_COMPRESSION_TYPE_BASE`, `filter_type != PNG_FILTER_TYPE_BASE`) — checked by `png_check_IHDR` (`pngset.c:453-455`) | `png_error`/`png_warning` from `png_check_IHDR` (e.g. `"Invalid image width"`, `"Invalid bit depth"`, `"Invalid color type"`, `"Invalid image size in IHDR"`) | [x] |
| 439 | `png_set_oFFs` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:481-482`) | silent `return` | [x] |
| 440 | `png_set_pCAL` | `png_ptr == NULL \|\| info_ptr == NULL \|\| purpose == NULL \|\| units == NULL \|\| (nparams > 0 && params == NULL)` (`pngset.c:502-504`) | silent `return` | [x] |
| 441 | `png_set_pCAL` | `type < 0 \|\| type > 3` (equation type outside PNG spec) (`pngset.c:513-518`) | `png_chunk_report(png_ptr, "Invalid pCAL equation type", PNG_CHUNK_WRITE_ERROR)`; chunk not stored | [x] |
| 442 | `png_set_pCAL` | `nparams < 0 \|\| nparams > 255` (`pngset.c:520-525`) | `png_chunk_report(png_ptr, "Invalid pCAL parameter count", PNG_CHUNK_WRITE_ERROR)` | [x] |
| 443 | `png_set_pCAL` | some `params[i] == NULL` or `!png_check_fp_string(params[i], strlen(params[i]))` (not a valid PNG floating-point string) (`pngset.c:528-537`) | `png_chunk_report(png_ptr, "Invalid format for pCAL parameter", PNG_CHUNK_WRITE_ERROR)` | [x] |
| 444 | `png_set_pCAL` | `png_malloc_warn` for `pcal_purpose` returns `NULL` (`pngset.c:539-547`) | `png_chunk_report(png_ptr, "Insufficient memory for pCAL purpose", PNG_CHUNK_WRITE_ERROR)` | [x] |
| 445 | `png_set_pCAL` | `png_malloc_warn` for `pcal_units` returns `NULL` (`pngset.c:563-570`) | `png_warning(png_ptr, "Insufficient memory for pCAL units")`; `PNG_INFO_pCAL` not set | [x] |
| 446 | `png_set_pCAL` | `png_malloc_warn` for the `pcal_params` array returns `NULL` (`pngset.c:574-581`) | `png_warning(png_ptr, "Insufficient memory for pCAL params")` | [x] |
| 447 | `png_set_pCAL` | `png_malloc_warn` for an individual `pcal_params[i]` returns `NULL` (`pngset.c:592-598`) | `png_warning(png_ptr, "Insufficient memory for pCAL parameter")` | [x] |
| 448 | `png_set_sCAL_s` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:616-617`) | silent `return` | [x] |
| 449 | `png_set_sCAL_s` | `unit != 1 && unit != 2` (only meter/radian allowed) (`pngset.c:622-623`) | `png_error(png_ptr, "Invalid sCAL unit")` | [x] |
| 450 | `png_set_sCAL_s` | `swidth == NULL \|\| strlen(swidth) == 0 \|\| swidth[0] == '-' \|\| !png_check_fp_string(swidth, lengthw)` (`pngset.c:625-627`) | `png_error(png_ptr, "Invalid sCAL width")` | [x] |
| 451 | `png_set_sCAL_s` | `sheight == NULL \|\| strlen(sheight) == 0 \|\| sheight[0] == '-' \|\| !png_check_fp_string(sheight, lengthh)` (`pngset.c:629-631`) | `png_error(png_ptr, "Invalid sCAL height")` | [x] |
| 452 | `png_set_sCAL_s` | `png_malloc_warn` for `scal_s_width` returns `NULL` (`pngset.c:639-647`) | `png_warning(png_ptr, "Memory allocation failed while processing sCAL")`; `PNG_INFO_sCAL` not set | [x] |
| 453 | `png_set_sCAL_s` | `png_malloc_warn` for `scal_s_height` returns `NULL` (`pngset.c:655-665`) | frees `scal_s_width`, `png_warning(png_ptr, "Memory allocation failed while processing sCAL")` | [x] |
| 454 | `png_set_sCAL` | `width <= 0` (`pngset.c:681-682`) | `png_warning(png_ptr, "Invalid sCAL width ignored")`; nothing stored | [x] |
| 455 | `png_set_sCAL` | `height <= 0` (`pngset.c:684-685`) | `png_warning(png_ptr, "Invalid sCAL height ignored")` | [x] |
| 456 | `png_set_sCAL_fixed` | `width <= 0` (`pngset.c:711-712`) | `png_warning(png_ptr, "Invalid sCAL width ignored")` | [x] |
| 457 | `png_set_sCAL_fixed` | `height <= 0` (`pngset.c:714-715`) | `png_warning(png_ptr, "Invalid sCAL height ignored")` | [x] |
| 458 | `png_set_pHYs` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:739-740`) | silent `return` | [x] |
| 459 | `png_set_PLTE` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:758-759`) | silent `return` | [x] |
| 460 | `png_set_PLTE` | `num_palette < 0 \|\| num_palette > (1 << info_ptr->bit_depth)` when `info_ptr->color_type == PNG_COLOR_TYPE_PALETTE` (`pngset.c:761-767`) | `png_error(png_ptr, "Invalid palette length")` | [x] |
| 461 | `png_set_PLTE` | `num_palette < 0 \|\| num_palette > PNG_MAX_PALETTE_LENGTH` when `color_type != PNG_COLOR_TYPE_PALETTE` (`pngset.c:764`, `:771-773`) | `png_warning(png_ptr, "Invalid palette length")` then `return` | [x] |
| 462 | `png_set_PLTE` | `num_palette > 0 && palette == NULL` (`pngset.c:777`, `:784`) | `png_error(png_ptr, "Invalid palette")` | [x] |
| 463 | `png_set_PLTE` | `num_palette == 0` and `(png_ptr->mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0` (empty PLTE not permitted) (`pngset.c:778-785`) | `png_error(png_ptr, "Invalid palette")` | [x] |
| 464 | `png_set_sBIT` | `png_ptr == NULL \|\| info_ptr == NULL \|\| sig_bit == NULL` (`pngset.c:840-841`) | silent `return` | [x] |
| 465 | `png_set_sRGB` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:854-855`) | silent `return` | [x] |
| 466 | `png_set_sRGB_gAMA_and_cHRM` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:867-868`) | silent `return` | [x] |
| 467 | `png_set_iCCP` | `png_ptr == NULL \|\| info_ptr == NULL \|\| name == NULL \|\| profile == NULL` (`pngset.c:900-901`) | silent `return` | [x] |
| 468 | `png_set_iCCP` | `compression_type != PNG_COMPRESSION_TYPE_BASE` (`pngset.c:903-904`) | `png_app_error(png_ptr, "Invalid iCCP compression method")` | [x] |
| 469 | `png_set_iCCP` | `png_malloc_warn(png_ptr, strlen(name)+1)` returns `NULL` (`pngset.c:907-914`) | `png_benign_error(png_ptr, "Insufficient memory to process iCCP chunk")`; `PNG_INFO_iCCP` not set | [x] |
| 470 | `png_set_iCCP` | `png_malloc_warn(png_ptr, proflen)` returns `NULL` (`pngset.c:917-927`) | frees name, `png_benign_error(png_ptr, "Insufficient memory to process iCCP profile")` | [x] |
| 471 | `png_set_text` | `png_set_text_2()` returns non-zero (allocation failure / too many text chunks) (`pngset.c:947-950`) | `png_error(png_ptr, "Insufficient memory to store text")` | [x] |
| 472 | `png_set_text_2` | `png_ptr == NULL \|\| info_ptr == NULL \|\| num_text <= 0 \|\| text_ptr == NULL` (`pngset.c:963-964`) | `return 0` (nothing stored) | [x] |
| 473 | `png_set_text_2` | growth overflow: `num_text > INT_MAX - max_text` where `max_text = info_ptr->num_text` (`pngset.c:979`) | `new_text` stays `NULL` → `png_chunk_report(png_ptr, "too many text chunks", PNG_CHUNK_WRITE_ERROR)`, `return 1` | [x] |
| 474 | `png_set_text_2` | `png_realloc_array(...)` for the text array returns `NULL` (out of memory / array-size overflow) (`pngset.c:993-1004`) | `png_chunk_report(png_ptr, "too many text chunks", PNG_CHUNK_WRITE_ERROR)`, `return 1` | [x] |
| 475 | `png_set_text_2` | `text_ptr[i].key == NULL` (`pngset.c:1025-1026`) | `continue` — entry silently skipped | [x] |
| 476 | `png_set_text_2` | `text_ptr[i].compression < PNG_TEXT_COMPRESSION_NONE \|\| text_ptr[i].compression >= PNG_TEXT_COMPRESSION_LAST` (`pngset.c:1028-1034`) | `png_chunk_report(png_ptr, "text compression mode is out of range", PNG_CHUNK_WRITE_ERROR)`, `continue` | [x] |
| 477 | `png_set_text_2` | `text_ptr[i].compression > 0` (iTXt) when built without `PNG_iTXt_SUPPORTED` (`pngset.c:1061-1066`) | `png_chunk_report(png_ptr, "iTXt chunk not supported", PNG_CHUNK_WRITE_ERROR)`, `continue` | n/a |
| 478 | `png_set_text_2` | `png_malloc_base(png_ptr, key_len + text_length + lang_len + lang_key_len + 4)` returns `NULL` (`pngset.c:1087-1097`) | `png_chunk_report(png_ptr, "text chunk: out of memory", PNG_CHUNK_WRITE_ERROR)`, `return 1` | [x] |
| 479 | `png_set_tIME` | `png_ptr == NULL \|\| info_ptr == NULL \|\| mod_time == NULL` (`pngset.c:1161`) | silent `return` | [x] |
| 480 | `png_set_tIME` | `(png_ptr->mode & PNG_WROTE_tIME) != 0` (tIME already written) (`pngset.c:1162`) | silent `return` | [x] |
| 481 | `png_set_tIME` | `mod_time->month == 0 \|\| mod_time->month > 12` (`pngset.c:1165`) | `png_warning(png_ptr, "Ignoring invalid time value")`; `PNG_INFO_tIME` not set | [x] |
| 482 | `png_set_tIME` | `mod_time->day == 0 \|\| mod_time->day > 31` (`pngset.c:1166`) | `png_warning(png_ptr, "Ignoring invalid time value")` | [x] |
| 483 | `png_set_tIME` | `mod_time->hour > 23` (`pngset.c:1167`) | `png_warning(png_ptr, "Ignoring invalid time value")` | [x] |
| 484 | `png_set_tIME` | `mod_time->minute > 59` (`pngset.c:1167`) | `png_warning(png_ptr, "Ignoring invalid time value")` | [x] |
| 485 | `png_set_tIME` | `mod_time->second > 60` (`pngset.c:1168`) | `png_warning(png_ptr, "Ignoring invalid time value")` | [x] |
| 486 | `png_set_tRNS` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:1187-1189`) | silent `return` | [x] |
| 487 | `png_set_tRNS` | `trans_alpha != NULL` but `num_trans <= 0 \|\| num_trans > PNG_MAX_PALETTE_LENGTH` (`pngset.c:1198`, `:1205`, `:1233-1237`) | alpha array not copied; `png_ptr->trans_alpha` freed and set to `NULL` | [x] |
| 488 | `png_set_tRNS` | `info_ptr->bit_depth < 16` and `color_type == PNG_COLOR_TYPE_GRAY && trans_color->gray > (1 << bit_depth) - 1` (`pngset.c:1243-1253`) | `png_warning(png_ptr, "tRNS chunk has out-of-range samples for bit_depth")` (value still stored) | [x] |
| 489 | `png_set_tRNS` | `info_ptr->bit_depth < 16` and `color_type == PNG_COLOR_TYPE_RGB` and `trans_color->red/green/blue > (1 << bit_depth) - 1` (`pngset.c:1249-1254`) | `png_warning(png_ptr, "tRNS chunk has out-of-range samples for bit_depth")` | [x] |
| 490 | `png_set_sPLT` | `png_ptr == NULL \|\| info_ptr == NULL \|\| nentries <= 0 \|\| entries == NULL` (`pngset.c:1292-1293`) | silent `return` | [x] |
| 491 | `png_set_sPLT` | `png_realloc_array(...)` for `splt_palettes` returns `NULL` (out of memory / too many palettes) (`pngset.c:1298-1307`) | `png_chunk_report(png_ptr, "too many sPLT chunks", PNG_CHUNK_WRITE_ERROR)` | [x] |
| 492 | `png_set_sPLT` | `entries->name == NULL \|\| entries->entries == NULL` for some input entry (`pngset.c:1324-1330`) | `png_app_error(png_ptr, "png_set_sPLT: invalid sPLT")`, entry skipped via `continue` | [x] |
| 493 | `png_set_sPLT` | `png_malloc_base(png_ptr, strlen(entries->name)+1)` returns `NULL` (`pngset.c:1338-1341`) | `break` out of loop → falls through to `png_chunk_report(png_ptr, "sPLT out of memory", PNG_CHUNK_WRITE_ERROR)` | [x] |
| 494 | `png_set_sPLT` | `png_malloc_array(png_ptr, entries->nentries, sizeof (png_sPLT_entry))` returns `NULL` (`pngset.c:1349-1357`) | frees `np->name`, `break` → `png_chunk_report(png_ptr, "sPLT out of memory", PNG_CHUNK_WRITE_ERROR)` | [x] |
| 495 | `png_set_sPLT` | `nentries > 0` remaining after the loop terminated early (`pngset.c:1378-1379`) | `png_chunk_report(png_ptr, "sPLT out of memory", PNG_CHUNK_WRITE_ERROR)` | [x] |
| 496 | `check_location` | `(location & (PNG_HAVE_IHDR\|PNG_HAVE_PLTE\|PNG_AFTER_IDAT)) == 0` on a write struct (`(png_ptr->mode & PNG_IS_READ_STRUCT) == 0`) (`pngset.c:1393-1401`) | `png_app_warning(png_ptr, "png_set_unknown_chunks now expects a valid location")`, falls back to `png_ptr->mode` bits | [x] |
| 497 | `check_location` | `location == 0` after the fallback (e.g. read struct with no valid location bits) (`pngset.c:1406-1407`) | `png_error(png_ptr, "invalid location in png_set_unknown_chunks")` | [x] |
| 498 | `png_set_unknown_chunks` | `png_ptr == NULL \|\| info_ptr == NULL \|\| num_unknowns <= 0 \|\| unknowns == NULL` (`pngset.c:1428-1430`) | silent `return` | [x] |
| 499 | `png_set_unknown_chunks` | called on a read struct in a build without `PNG_READ_UNKNOWN_CHUNKS_SUPPORTED` (`pngset.c:1440-1445`) | `png_app_error(png_ptr, "no unknown chunk support on read")`, `return` | n/a |
| 500 | `png_set_unknown_chunks` | called on a write struct in a build without `PNG_WRITE_UNKNOWN_CHUNKS_SUPPORTED` (`pngset.c:1449-1454`) | `png_app_error(png_ptr, "no unknown chunk support on write")`, `return` | n/a |
| 501 | `png_set_unknown_chunks` | `png_realloc_array(...)` for `unknown_chunks` returns `NULL` (out of memory / count overflow) (`pngset.c:1462-1471`) | `png_chunk_report(png_ptr, "too many unknown chunks", PNG_CHUNK_WRITE_ERROR)`, `return` | [x] |
| 502 | `png_set_unknown_chunks` | `png_malloc_base(png_ptr, unknowns->size)` returns `NULL` for one chunk's data (`pngset.c:1500-1509`) | `png_chunk_report(png_ptr, "unknown chunk: out of memory", PNG_CHUNK_WRITE_ERROR)`, chunk skipped via `continue` | [x] |
| 503 | `png_set_unknown_chunk_location` | `png_ptr == NULL \|\| info_ptr == NULL \|\| chunk < 0 \|\| chunk >= info_ptr->unknown_chunks_num` (index out of range) (`pngset.c:1535-1536`) | silent no-op | [x] |
| 504 | `png_set_unknown_chunk_location` | `(location & (PNG_HAVE_IHDR\|PNG_HAVE_PLTE\|PNG_AFTER_IDAT)) == 0` (`pngset.c:1538-1547`) | `png_app_error(png_ptr, "invalid unknown chunk location")`, then location forced to `PNG_AFTER_IDAT` or `PNG_HAVE_IHDR` | [x] |
| 505 | `png_permit_mng_features` | `png_ptr == NULL` (`pngset.c:1561-1562`) | `return 0` | [x] |
| 506 | `png_permit_mng_features` | bits set in `mng_features` outside `PNG_ALL_MNG_FEATURES` (`pngset.c:1564`) | unsupported bits masked off; `return png_ptr->mng_features_permitted` (subset of request) | [x] |
| 507 | `png_set_keep_unknown_chunks` | `png_ptr == NULL` (`pngset.c:1606-1607`) | silent `return` | [x] |
| 508 | `png_set_keep_unknown_chunks` | `keep < 0 \|\| keep >= PNG_HANDLE_CHUNK_LAST` (`pngset.c:1609-1613`) | `png_app_error(png_ptr, "png_set_keep_unknown_chunks: invalid keep")`, `return` | [x] |
| 509 | `png_set_keep_unknown_chunks` | `num_chunks_in == 0` (`pngset.c:1616-1622`) | only `png_ptr->unknown_default = keep`, then `return` (no list processed) | [x] |
| 510 | `png_set_keep_unknown_chunks` | `num_chunks_in > 0 && chunk_list == NULL` (`pngset.c:1660-1667`) | `png_app_error(png_ptr, "png_set_keep_unknown_chunks: no chunk list")`, `return` | [x] |
| 511 | `png_set_keep_unknown_chunks` | `num_chunks + old_num_chunks > UINT_MAX/5` (list-size overflow) (`pngset.c:1679-1684`) | `png_app_error(png_ptr, "png_set_keep_unknown_chunks: too many chunks")`, `return` | [x] |
| 512 | `png_set_read_user_chunk_fn` | `png_ptr == NULL` (`pngset.c:1767-1768`) | silent `return` | [x] |
| 513 | `png_set_rows` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:1782-1783`) | silent `return` | [x] |
| 514 | `png_set_compression_buffer_size` | `png_ptr == NULL` (`pngset.c:1801-1802`) | silent `return` | [x] |
| 515 | `png_set_compression_buffer_size` | `size == 0 \|\| size > PNG_UINT_31_MAX` (`pngset.c:1804-1805`) | `png_error(png_ptr, "invalid compression buffer size")` | [x] |
| 516 | `png_set_compression_buffer_size` | write struct with `png_ptr->zowner != 0` (zstream in use) (`pngset.c:1818-1824`) | `png_warning(png_ptr, "Compression buffer size cannot be changed because it is in use")`, `return` | [x] |
| 517 | `png_set_compression_buffer_size` | `size > ZLIB_IO_MAX` (`pngset.c:1830-1835`) | `png_warning(png_ptr, "Compression buffer size limited to system maximum")`, `size = ZLIB_IO_MAX` | n/a |
| 518 | `png_set_compression_buffer_size` | `size < 6` (would hang deflate on SYNC_FLUSH) (`pngset.c:1838-1847`) | `png_warning(png_ptr, "Compression buffer size cannot be reduced below 6")`, `return` | [x] |
| 519 | `png_set_invalid` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:1861-1862`) | silent no-op | [x] |
| 520 | `png_set_user_limits` | `png_ptr == NULL` (`pngset.c:1878-1879`) | silent `return` | [x] |
| 521 | `png_set_chunk_cache_max` | `png_ptr == NULL` (`pngset.c:1891-1892`) | silent no-op | [x] |
| 522 | `png_set_chunk_malloc_max` | `png_ptr == NULL` (`pngset.c:1907`) | silent no-op | [x] |
| 523 | `png_set_chunk_malloc_max` | `user_chunk_malloc_max == 0U` (request for "unlimited") (`pngset.c:1909-1916`) | value replaced by `PNG_SIZE_MAX` (or `65536U` under `PNG_MAX_MALLOC_64K`) | [x] |
| 524 | `png_check_keyword` | `key == NULL` (`pngset.c:1992-1996`) | `*new_key = 0`, `return 0` (caller must `png_error`) | [x] |
| 525 | `png_check_keyword` | character not in `(32 < ch <= 126)` nor `ch >= 161` (control chars, 0x80..0xA0 incl. non-break space) (`pngset.c:2002-2020`) | character dropped/collapsed to a single space, first offender recorded in `bad_character` | [x] |
| 526 | `png_check_keyword` | trailing space, i.e. `key_len > 0 && space != 0` (`pngset.c:2023-2028`) | trailing space removed, `bad_character = 32` | [x] |
| 527 | `png_check_keyword` | resulting `key_len == 0` (keyword empty or all-invalid) (`pngset.c:2033-2034`) | `return 0` (caller must `png_error`) | [x] |
| 528 | `png_check_keyword` | keyword longer than 79 characters, so `*key != 0` after the `key_len < 79` loop (`pngset.c:1998`, `:2038-2039`) | `png_warning(png_ptr, "keyword truncated")`, keyword truncated to 79 chars | [x] |
| 529 | `png_check_keyword` | `bad_character != 0` (`pngset.c:2041-2049`) | `png_formatted_warning(png_ptr, p, "keyword \"@1\": bad character '0x@2'")` | [x] |
| 530 | `png_set_bgr` | `png_ptr == NULL` (`pngtrans.c:24-25`) | silent `return` | [x] |
| 531 | `png_set_swap` | `png_ptr == NULL` (`pngtrans.c:38-39`) | silent `return` | [x] |
| 532 | `png_set_swap` | `png_ptr->bit_depth != 16` (`pngtrans.c:41`) | request silently ignored; `PNG_SWAP_BYTES` not set | [x] |
| 533 | `png_set_packing` | `png_ptr == NULL` (`pngtrans.c:53-54`) | silent `return` | [x] |
| 534 | `png_set_packing` | `png_ptr->bit_depth >= 8` (`pngtrans.c:56`) | request silently ignored; `PNG_PACK` not set | [x] |
| 535 | `png_set_packswap` | `png_ptr == NULL` (`pngtrans.c:73-74`) | silent `return` | [x] |
| 536 | `png_set_packswap` | `png_ptr->bit_depth >= 8` (`pngtrans.c:76`) | request silently ignored; `PNG_PACKSWAP` not set | [x] |
| 537 | `png_set_shift` | `png_ptr == NULL \|\| true_bits == NULL` (`pngtrans.c:87-88`) | silent `return` | [x] |
| 538 | `png_set_shift` | color image (`color_type & PNG_COLOR_MASK_COLOR`) and `true_bits->red == 0 \|\| red > bit_depth \|\| green == 0 \|\| green > bit_depth \|\| blue == 0 \|\| blue > bit_depth` (`pngtrans.c:95-101`, `:112-116`) | `png_app_error(png_ptr, "png_set_shift: invalid shift values")`, `return` | [x] |
| 539 | `png_set_shift` | grayscale image and `true_bits->gray == 0 \|\| true_bits->gray > bit_depth` (`pngtrans.c:102-106`, `:112-116`) | `png_app_error(png_ptr, "png_set_shift: invalid shift values")`, `return` | [x] |
| 540 | `png_set_shift` | `(color_type & PNG_COLOR_MASK_ALPHA) != 0 && (true_bits->alpha == 0 \|\| true_bits->alpha > bit_depth)` (`pngtrans.c:108-110`, `:112-116`) | `png_app_error(png_ptr, "png_set_shift: invalid shift values")`, `return` | [x] |
| 541 | `png_set_interlace_handling` | `png_ptr == 0 \|\| png_ptr->interlaced == 0` (`pngtrans.c:131`, `:137`) | `return 1` (single pass; `PNG_INTERLACE` not set) | [x] |
| 542 | `png_set_filler` | `png_ptr == NULL` (`pngtrans.c:152-153`) | silent `return` | [x] |
| 543 | `png_set_filler` | read struct in a build without `PNG_READ_FILLER_SUPPORTED` (`pngtrans.c:158`, `:171-173`) | `png_app_error(png_ptr, "png_set_filler not supported on read")`, `return` | n/a |
| 544 | `png_set_filler` | write with `color_type == PNG_COLOR_TYPE_GRAY && png_ptr->bit_depth < 8` (`pngtrans.c:189-206`) | `png_app_error(png_ptr, "png_set_filler is invalid for low bit depth gray output")`, `return` | [x] |
| 545 | `png_set_filler` | write with `color_type` other than `PNG_COLOR_TYPE_RGB`/`PNG_COLOR_TYPE_GRAY` (palette, GA, RGBA) (`pngtrans.c:208-211`) | `png_app_error(png_ptr, "png_set_filler: inappropriate color type")`, `return` | [x] |
| 546 | `png_set_filler` | write struct in a build without `PNG_WRITE_FILLER_SUPPORTED` (`pngtrans.c:213-215`) | `png_app_error(png_ptr, "png_set_filler not supported on write")`, `return` | n/a |
| 547 | `png_set_add_alpha` | `png_ptr == NULL` (`pngtrans.c:237-238`) | silent `return` | [x] |
| 548 | `png_set_add_alpha` | `png_set_filler()` failed so `(png_ptr->transformations & PNG_FILLER) == 0` (`pngtrans.c:242-243`) | `PNG_ADD_ALPHA` not set (error already reported by `png_set_filler`) | [x] |
| 549 | `png_set_swap_alpha` | `png_ptr == NULL` (`pngtrans.c:255-256`) | silent `return` | [x] |
| 550 | `png_set_invert_alpha` | `png_ptr == NULL` (`pngtrans.c:269-270`) | silent `return` | [x] |
| 551 | `png_set_invert_mono` | `png_ptr == NULL` (`pngtrans.c:282-283`) | silent `return` | [x] |
| 552 | `png_do_invert` | `row_info->color_type` is neither `PNG_COLOR_TYPE_GRAY`, nor `PNG_COLOR_TYPE_GRAY_ALPHA` with `bit_depth` 8 or 16 (`pngtrans.c:297`, `:310-311`, `:325-326`) | row left unchanged (no branch taken) | [x] |
| 553 | `png_do_swap` | `row_info->bit_depth != 16` (`pngtrans.c:351`) | row left unchanged | [x] |
| 554 | `png_do_packswap` | `row_info->bit_depth >= 8` (`pngtrans.c:487`) | row left unchanged | [x] |
| 555 | `png_do_packswap` | `row_info->bit_depth < 8` but not 1, 2 or 4 (e.g. 3, 5, 6, 7) (`pngtrans.c:493-503`) | `return` (no swap table) | [x] |
| 556 | `png_do_strip_channel` | `row_info->channels == 2` and `row_info->bit_depth` neither 8 nor 16 (`pngtrans.c:541`, `:559`, `:576-577`) | `return` (bad bit depth; row and `rowbytes` untouched) | [x] |
| 557 | `png_do_strip_channel` | `row_info->channels == 4` and `row_info->bit_depth` neither 8 nor 16 (`pngtrans.c:589`, `:607`, `:627-628`) | `return` (bad bit depth) | [x] |
| 558 | `png_do_strip_channel` | `row_info->channels` neither 2 nor 4 (filler channel already gone) (`pngtrans.c:637-638`) | `return` | [x] |
| 559 | `png_do_bgr` | `(row_info->color_type & PNG_COLOR_MASK_COLOR) == 0` (grayscale) (`pngtrans.c:652`) | row left unchanged | [x] |
| 560 | `png_do_bgr` | color row with `bit_depth` neither 8 nor 16 (`pngtrans.c:655`, `:685`) | row left unchanged | [x] |
| 561 | `png_do_bgr` | `color_type` neither `PNG_COLOR_TYPE_RGB` nor `PNG_COLOR_TYPE_RGB_ALPHA` inside the 8-/16-bit branches (`pngtrans.c:657`, `:670`, `:687`, `:703`) | row left unchanged | [x] |
| 562 | `png_do_check_palette_indexes` | `png_ptr->num_palette >= (1 << row_info->bit_depth) \|\| png_ptr->num_palette == 0` (complete palette, or MNG empty palette) (`pngtrans.c:732-733`) | no index checking performed; `num_palette_max` untouched | [x] |
| 563 | `png_do_check_palette_indexes` | `row_info->bit_depth` not in {1,2,4,8} (`default:` case) (`pngtrans.c:822-823`) | `break` — no index checking performed | [x] |
| 564 | `png_set_user_transform_info` | `png_ptr == NULL` (`pngtrans.c:838-839`) | silent `return` | [x] |
| 565 | `png_set_user_transform_info` | read struct with `(png_ptr->flags & PNG_FLAG_ROW_INIT) != 0` (called too late) (`pngtrans.c:842-848`) | `png_app_error(png_ptr, "info change after png_start_read_image or png_read_update_info")`, `return` | [x] |
| 566 | `png_get_user_transform_ptr` | `png_ptr == NULL` (`pngtrans.c:866-867`) | `return NULL` | [x] |
| 567 | `png_get_current_row_number` | `png_ptr == NULL` (`pngtrans.c:880-883`) | `return PNG_UINT_32_MAX` | [x] |
| 568 | `png_get_current_pass_number` | `png_ptr == NULL` (`pngtrans.c:889-891`) | `return 8` (invalid pass) | [x] |

## pngread.c / pngrtran.c / pngpread.c

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 569 | `png_create_read_struct` / `png_create_read_struct_2` | `png_create_png_struct` fails (out of memory, or `user_png_ver` incompatible with the library) — `png_ptr` stays `NULL` (pngread.c:30, 46, 50, 79) | returns `NULL`; no read struct created | [x] |
| 570 | `png_read_info` | `png_ptr == NULL \|\| info_ptr == NULL` (pngread.c:101-102) | silent `return`; nothing read | [x] |
| 571 | `png_read_info` | first `IDAT` seen while `(png_ptr->mode & PNG_HAVE_IHDR) == 0` (pngread.c:117-118) | `png_chunk_error(png_ptr, "Missing IHDR before IDAT")` — fatal | [x] |
| 572 | `png_read_info` | `IDAT` when `png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && (png_ptr->mode & PNG_HAVE_PLTE) == 0` (pngread.c:120-122) | `png_chunk_error(png_ptr, "Missing PLTE before IDAT")` — fatal | [x] |
| 573 | `png_read_info` | `IDAT` seen after a non-IDAT chunk already followed IDAT: `(png_ptr->mode & PNG_AFTER_IDAT) != 0` (pngread.c:124-125) | `png_chunk_benign_error(png_ptr, "Too many IDATs found")` (error or warning per benign-error flag) | [x] |
| 574 | `png_read_update_info` | `png_ptr == NULL` (pngread.c:176) | silent `return`; info not updated | [x] |
| 575 | `png_read_update_info` | called twice / after `png_start_read_image`: `(png_ptr->flags & PNG_FLAG_ROW_INIT) != 0` (pngread.c:178, 191-192) | `png_app_error(png_ptr, "png_read_update_info/png_start_read_image: duplicate call")` | [x] |
| 576 | `png_start_read_image` | `png_ptr == NULL` (pngread.c:207) | silent `return` | [x] |
| 577 | `png_start_read_image` | called twice / after `png_read_update_info`: `(png_ptr->flags & PNG_FLAG_ROW_INIT) != 0` (pngread.c:209, 214-215) | `png_app_error(png_ptr, "png_start_read_image/png_read_update_info: duplicate call")` | [x] |
| 578 | `png_do_read_intrapixel` | MNG intrapixel differencing at `bit_depth == 8` with a color type that is neither `PNG_COLOR_TYPE_RGB` nor `PNG_COLOR_TYPE_RGB_ALPHA` (pngread.c:241-248) | early `return`; row left untransformed | [x] |
| 579 | `png_do_read_intrapixel` | MNG intrapixel differencing at `bit_depth == 16` with a color type that is neither `PNG_COLOR_TYPE_RGB` nor `PNG_COLOR_TYPE_RGB_ALPHA` (pngread.c:261-268) | early `return`; row left untransformed | [x] |
| 580 | `png_read_row` | `png_ptr == NULL` (pngread.c:292-293) | silent `return` | [x] |
| 581 | `png_read_row` | on first row, `(png_ptr->transformations & PNG_INVERT_MONO) != 0` but `PNG_READ_INVERT_SUPPORTED` not compiled in (pngread.c:317-318) | `png_warning(png_ptr, "PNG_READ_INVERT_SUPPORTED is not defined")`; transform skipped | [x] |
| 582 | `png_read_row` | on first row, `(png_ptr->transformations & PNG_FILLER) != 0` but `PNG_READ_FILLER_SUPPORTED` not compiled in (pngread.c:322-323) | `png_warning(png_ptr, "PNG_READ_FILLER_SUPPORTED is not defined")` | [x] |
| 583 | `png_read_row` | on first row, `(png_ptr->transformations & PNG_PACKSWAP) != 0` but `PNG_READ_PACKSWAP_SUPPORTED` not compiled in (pngread.c:328-329) | `png_warning(png_ptr, "PNG_READ_PACKSWAP_SUPPORTED is not defined")` | [x] |
| 584 | `png_read_row` | on first row, `(png_ptr->transformations & PNG_PACK) != 0` but `PNG_READ_PACK_SUPPORTED` not compiled in (pngread.c:333-334) | `png_warning(png_ptr, "PNG_READ_PACK_SUPPORTED is not defined")` | [x] |
| 585 | `png_read_row` | on first row, `(png_ptr->transformations & PNG_SHIFT) != 0` but `PNG_READ_SHIFT_SUPPORTED` not compiled in (pngread.c:338-339) | `png_warning(png_ptr, "PNG_READ_SHIFT_SUPPORTED is not defined")` | [x] |
| 586 | `png_read_row` | on first row, `(png_ptr->transformations & PNG_BGR) != 0` but `PNG_READ_BGR_SUPPORTED` not compiled in (pngread.c:343-344) | `png_warning(png_ptr, "PNG_READ_BGR_SUPPORTED is not defined")` | [x] |
| 587 | `png_read_row` | on first row, `(png_ptr->transformations & PNG_SWAP_BYTES) != 0` but `PNG_READ_SWAP_SUPPORTED` not compiled in (pngread.c:348-349) | `png_warning(png_ptr, "PNG_READ_SWAP_SUPPORTED is not defined")` | [x] |
| 588 | `png_read_row` | row data requested but no IDAT has been reached: `(png_ptr->mode & PNG_HAVE_IDAT) == 0` (pngread.c:443-444) | `png_error(png_ptr, "Invalid attempt to read row data")` — fatal | [x] |
| 589 | `png_read_row` | filter byte in the decompressed row `png_ptr->row_buf[0] >= PNG_FILTER_VALUE_LAST` (i.e. > 4; includes the sentinel 255 written when no data was produced) (pngread.c:450-456) | `png_error(png_ptr, "bad adaptive filter value")` — fatal | [x] |
| 590 | `png_read_row` | after transforms, first row's `row_info.pixel_depth > png_ptr->maximum_pixel_depth` (pngread.c:485-489) | `png_error(png_ptr, "sequential row overflow")` — fatal | [x] |
| 591 | `png_read_row` | later row's `png_ptr->transformed_pixel_depth != row_info.pixel_depth` (pngread.c:492-493) | `png_error(png_ptr, "internal sequential row size calculation error")` — fatal | [x] |
| 592 | `png_read_rows` | `png_ptr == NULL` (pngread.c:562-563) | silent `return` | [x] |
| 593 | `png_read_rows` | both `row == NULL` and `display_row == NULL` (pngread.c:567-590) | no branch taken; no rows read, no error reported | [x] |
| 594 | `png_read_image` | `png_ptr == NULL` (pngread.c:616-617) | silent `return` | [x] |
| 595 | `png_read_image` | interlaced file where the app called `png_start_read_image`/`png_read_update_info` without enabling `PNG_INTERLACE`: `png_ptr->interlaced != 0 && (png_ptr->transformations & PNG_INTERLACE) == 0` (pngread.c:628-638) | `png_warning(png_ptr, "Interlace handling should be turned on when using png_read_image")`; `num_rows` forced to `height` | [x] |
| 596 | `png_read_image` | interlaced file (`png_ptr->interlaced` non-zero) in a build without `PNG_READ_INTERLACING_SUPPORTED` (pngread.c:647-649) | `png_error(png_ptr, "Cannot read interlaced image -- interlace handler disabled")` — fatal | [x] |
| 597 | `png_read_end` | `png_ptr == NULL` (pngread.c:682-683) | silent `return` | [x] |
| 598 | `png_read_end` | palette image where a pixel index exceeded the palette: `png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && png_ptr->num_palette_max >= png_ptr->num_palette` (pngread.c:695-697) | `png_benign_error(png_ptr, "Read palette index exceeding num_palette")` | [x] |
| 599 | `png_read_end` | IDAT handled via unknown-chunk path with `(length > 0 && !(png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED)) \|\| (png_ptr->mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0` (pngread.c:725-729) | `png_benign_error(png_ptr, ".Too many IDATs found")` | [x] |
| 600 | `png_read_end` | trailing IDAT with `(length > 0 && !(png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED)) \|\| (png_ptr->mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0` (pngread.c:737-747) | `png_benign_error(png_ptr, "..Too many IDATs found")`, then chunk CRC-skipped | [x] |
| 601 | `png_destroy_read_struct` | `png_ptr_ptr == NULL` or `*png_ptr_ptr == NULL` (pngread.c:837-841) | silent `return`; nothing freed | [x] |
| 602 | `png_set_read_status_fn` | `png_ptr == NULL` (pngread.c:858-859) | silent `return` | [x] |
| 603 | `png_read_png` | `png_ptr == NULL \|\| info_ptr == NULL` (pngread.c:873-874) | silent `return` | [x] |
| 604 | `png_read_png` | `info_ptr->height > PNG_UINT_32_MAX/(sizeof (png_bytep))` (row-pointer array would overflow) (pngread.c:880-881) | `png_error(png_ptr, "Image is too high to process with png_read_png()")` — fatal | [x] |
| 605 | `png_read_png` | `transforms & PNG_TRANSFORM_SCALE_16` in a build without `PNG_READ_SCALE_16_TO_8_SUPPORTED` (pngread.c:892-900) | `png_app_error(png_ptr, "PNG_TRANSFORM_SCALE_16 not supported")` | [x] |
| 606 | `png_read_png` | `transforms & PNG_TRANSFORM_STRIP_16` without `PNG_READ_STRIP_16_TO_8_SUPPORTED` (pngread.c:906-911) | `png_app_error(png_ptr, "PNG_TRANSFORM_STRIP_16 not supported")` | [x] |
| 607 | `png_read_png` | `transforms & PNG_TRANSFORM_STRIP_ALPHA` without `PNG_READ_STRIP_ALPHA_SUPPORTED` (pngread.c:916-921) | `png_app_error(png_ptr, "PNG_TRANSFORM_STRIP_ALPHA not supported")` | [x] |
| 608 | `png_read_png` | `transforms & PNG_TRANSFORM_PACKING` without `PNG_READ_PACK_SUPPORTED` (pngread.c:926-931) | `png_app_error(png_ptr, "PNG_TRANSFORM_PACKING not supported")` | [x] |
| 609 | `png_read_png` | `transforms & PNG_TRANSFORM_PACKSWAP` without `PNG_READ_PACKSWAP_SUPPORTED` (pngread.c:936-941) | `png_app_error(png_ptr, "PNG_TRANSFORM_PACKSWAP not supported")` | [x] |
| 610 | `png_read_png` | `transforms & PNG_TRANSFORM_EXPAND` without `PNG_READ_EXPAND_SUPPORTED` (pngread.c:948-953) | `png_app_error(png_ptr, "PNG_TRANSFORM_EXPAND not supported")` | [x] |
| 611 | `png_read_png` | `transforms & PNG_TRANSFORM_INVERT_MONO` without `PNG_READ_INVERT_SUPPORTED` (pngread.c:960-965) | `png_app_error(png_ptr, "PNG_TRANSFORM_INVERT_MONO not supported")` | [x] |
| 612 | `png_read_png` | `transforms & PNG_TRANSFORM_SHIFT` without `PNG_READ_SHIFT_SUPPORTED` (pngread.c:971-977) | `png_app_error(png_ptr, "PNG_TRANSFORM_SHIFT not supported")` | [x] |
| 613 | `png_read_png` | `transforms & PNG_TRANSFORM_SHIFT` requested but file has no sBIT: `(info_ptr->valid & PNG_INFO_sBIT) == 0` (pngread.c:971-974) | `png_set_shift` not called; transform silently ignored, no diagnostic | [x] |
| 614 | `png_read_png` | `transforms & PNG_TRANSFORM_BGR` without `PNG_READ_BGR_SUPPORTED` (pngread.c:980-985) | `png_app_error(png_ptr, "PNG_TRANSFORM_BGR not supported")` | [x] |
| 615 | `png_read_png` | `transforms & PNG_TRANSFORM_SWAP_ALPHA` without `PNG_READ_SWAP_ALPHA_SUPPORTED` (pngread.c:988-993) | `png_app_error(png_ptr, "PNG_TRANSFORM_SWAP_ALPHA not supported")` | [x] |
| 616 | `png_read_png` | `transforms & PNG_TRANSFORM_SWAP_ENDIAN` without `PNG_READ_SWAP_SUPPORTED` (pngread.c:996-1001) | `png_app_error(png_ptr, "PNG_TRANSFORM_SWAP_ENDIAN not supported")` | [x] |
| 617 | `png_read_png` | `transforms & PNG_TRANSFORM_INVERT_ALPHA` without `PNG_READ_INVERT_ALPHA_SUPPORTED` (pngread.c:1005-1010) | `png_app_error(png_ptr, "PNG_TRANSFORM_INVERT_ALPHA not supported")` | [x] |
| 618 | `png_read_png` | `transforms & PNG_TRANSFORM_GRAY_TO_RGB` without `PNG_READ_GRAY_TO_RGB_SUPPORTED` (pngread.c:1014-1019) | `png_app_error(png_ptr, "PNG_TRANSFORM_GRAY_TO_RGB not supported")` | [x] |
| 619 | `png_read_png` | `transforms & PNG_TRANSFORM_EXPAND_16` without `PNG_READ_EXPAND_16_SUPPORTED` (pngread.c:1022-1027) | `png_app_error(png_ptr, "PNG_TRANSFORM_EXPAND_16 not supported")` | [x] |
| 620 | `png_image_read_init` | `image->opaque != NULL` on entry (image already in use / not zeroed) (pngread.c:1130, 1172) | `png_image_error(image, "png_image_read: opaque pointer not NULL")` → returns 0 | [x] |
| 621 | `png_image_read_init` | `png_create_read_struct`, `png_create_info_struct` or `png_malloc_warn(control)` returns `NULL` (pngread.c:1141-1169) | cleanup, then `png_image_error(image, "png_image_read: out of memory")` → returns 0 | [x] |
| 622 | `chromaticities_match_sRGB` | any of `whitex/whitey/redx/redy/greenx/greeny/bluex/bluey` differs from the BT.709 sRGB value by more than `sRGB_TOLERANCE` (1000) — `PNG_OUT_OF_RANGE(...)` (pngread.c:1217-1225) | `return 0`; caller marks image `PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB` | [x] |
| 623 | `png_gamma_not_sRGB` | `g < PNG_LIB_GAMMA_MIN \|\| g > PNG_LIB_GAMMA_MAX` (includes uninitialized `g == 0`) (pngread.c:1238-1239) | `return 0`; treated as "same as sRGB" (no gamma work) | [x] |
| 624 | `png_image_read_header` | computed `cmap_entries > 256` (e.g. `1U << bit_depth` for gray, or `num_palette`) (pngread.c:1326-1327) | clamped: `cmap_entries = 256` | [x] |
| 625 | `png_image_begin_read_from_stdio` | `file == NULL` (pngread.c:1341, 1354-1356) | `png_image_error(image, "png_image_begin_read_from_stdio: invalid argument")` → returns 0 | [x] |
| 626 | `png_image_begin_read_from_stdio` | `image->version != PNG_IMAGE_VERSION` (pngread.c:1339, 1359-1361) | `png_image_error(image, "png_image_begin_read_from_stdio: incorrect PNG_IMAGE_VERSION")` → returns 0 | [x] |
| 627 | `png_image_begin_read_from_stdio` | `image == NULL` (pngread.c:1339, 1363) | `return 0`; no error text recordable | [x] |
| 628 | `png_image_begin_read_from_stdio` | `png_image_read_init(image) == 0` (allocation failure) (pngread.c:1343, 1363) | falls through, `return 0` (message set by `png_image_read_init`) | [x] |
| 629 | `png_image_begin_read_from_file` | `file_name == NULL` (pngread.c:1371, 1392-1394) | `png_image_error(image, "png_image_begin_read_from_file: invalid argument")` → returns 0 | [x] |
| 630 | `png_image_begin_read_from_file` | `fopen(file_name, "rb") == NULL` (pngread.c:1373-1389) | `png_image_error(image, strerror(errno))` → returns 0 | [x] |
| 631 | `png_image_begin_read_from_file` | `image->version != PNG_IMAGE_VERSION` (pngread.c:1369, 1397-1399) | `png_image_error(image, "png_image_begin_read_from_file: incorrect PNG_IMAGE_VERSION")` → returns 0 | [x] |
| 632 | `png_image_begin_read_from_file` | `image == NULL` (pngread.c:1369, 1401) | `return 0` | [x] |
| 633 | `png_image_begin_read_from_file` | `png_image_read_init(image) == 0` after successful `fopen` (pngread.c:1377-1385, 1401) | file closed with `fclose`, `return 0` | [x] |
| 634 | `png_image_memory_read` | request beyond the supplied buffer: `memory == NULL \|\| size < need` (pngread.c:1419-1427) | `png_error(png_ptr, "read beyond end of data")` — fatal (longjmp out of `png_safe_execute`) | [x] |
| 635 | `png_image_memory_read` | `io_ptr` image is `NULL` or `image->opaque == NULL` (pngread.c:1410-1431) | `png_error(png_ptr, "invalid memory read")` — fatal | [x] |
| 636 | `png_image_memory_read` | `png_ptr == NULL` (pngread.c:1408) | silent `return`; nothing copied into `out` | [x] |
| 637 | `png_image_begin_read_from_memory` | `memory == NULL \|\| size == 0` (pngread.c:1440, 1457-1459) | `png_image_error(image, "png_image_begin_read_from_memory: invalid argument")` → returns 0 | [x] |
| 638 | `png_image_begin_read_from_memory` | `image->version != PNG_IMAGE_VERSION` (pngread.c:1438, 1462-1464) | `png_image_error(image, "png_image_begin_read_from_memory: incorrect PNG_IMAGE_VERSION")` → returns 0 | [x] |
| 639 | `png_image_begin_read_from_memory` | `image == NULL` (pngread.c:1438, 1466) | `return 0` | [x] |
| 640 | `png_image_begin_read_from_memory` | `png_image_read_init(image) == 0` (pngread.c:1442, 1466) | `return 0` | [x] |
| 641 | `set_file_encoding` | `png_resolve_file_gamma(png_ptr) == 0` (no gAMA/sRGB/default gamma resolvable) (pngread.c:1531-1537) | `png_error(png_ptr, "internal: default gamma not set")` — fatal | [x] |
| 642 | `decode_gamma` | `encoding` not one of `P_FILE/P_sRGB/P_LINEAR/P_LINEAR8` (GNUC `default:` arm) (pngread.c:1584-1588) | `png_error(png_ptr, "unexpected encoding (internal error)")` — fatal | [x] |
| 643 | `png_create_colormap_entry` | color-map index `ip > 255` (pngread.c:1642-1643) | `png_error(image->opaque->png_ptr, "color-map index out of range")` — fatal | [x] |
| 644 | `png_create_colormap_entry` | after conversion `encoding != output_encoding` (pngread.c:1742-1743) | `png_error(image->opaque->png_ptr, "bad encoding (internal error)")` — fatal | [x] |
| 645 | `png_image_read_colormap` | input has alpha/tRNS, output format has no `PNG_FORMAT_FLAG_ALPHA`, output is sRGB, and `display->background == NULL` (pngread.c:1989-1998) | `png_error(png_ptr, "background color must be supplied to remove alpha/transparency")` — fatal | [x] |
| 646 | `png_image_read_colormap` | gray, `bit_depth <= 8`: `(1U << bit_depth) > image->colormap_entries` (pngread.c:2054-2056) | `png_error(png_ptr, "gray[8] color-map: too few entries")` — fatal | [x] |
| 647 | `png_image_read_colormap` | gray, `bit_depth == 16`: `PNG_GRAY_COLORMAP_ENTRIES (256) > image->colormap_entries` (pngread.c:2134-2135) | `png_error(png_ptr, "gray[16] color-map: too few entries")` — fatal | [x] |
| 648 | `png_image_read_colormap` | GRAY_ALPHA with alpha kept: `PNG_GA_COLORMAP_ENTRIES (256) > image->colormap_entries` (pngread.c:2232-2233) | `png_error(png_ptr, "gray+alpha color-map: too few entries")` — fatal | [x] |
| 649 | `png_image_read_colormap` | GRAY_ALPHA, alpha removed on a gray background: `PNG_GRAY_COLORMAP_ENTRIES > image->colormap_entries` (pngread.c:2266-2267) | `png_error(png_ptr, "gray-alpha color-map: too few entries")` — fatal | [x] |
| 650 | `png_image_read_colormap` | GRAY_ALPHA, alpha removed on a colored background: `PNG_GA_COLORMAP_ENTRIES > image->colormap_entries` (pngread.c:2300-2301) | `png_error(png_ptr, "ga-alpha color-map: too few entries")` — fatal | [x] |
| 651 | `png_image_read_colormap` | RGB/RGBA → gray output with alpha in both: `PNG_GA_COLORMAP_ENTRIES > image->colormap_entries` (pngread.c:2405-2406) | `png_error(png_ptr, "rgb[ga] color-map: too few entries")` — fatal | [x] |
| 652 | `png_image_read_colormap` | RGB/RGBA → gray output without alpha: `PNG_GRAY_COLORMAP_ENTRIES > image->colormap_entries` (pngread.c:2421-2422) | `png_error(png_ptr, "rgb[gray] color-map: too few entries")` — fatal | [x] |
| 653 | `png_image_read_colormap` | RGBA/tRNS → color output with alpha: `PNG_RGB_COLORMAP_ENTRIES+1+27 (244) > image->colormap_entries` (pngread.c:2529-2530) | `png_error(png_ptr, "rgb+alpha color-map: too few entries")` — fatal | [x] |
| 654 | `png_image_read_colormap` | RGBA/tRNS → color output, alpha removed: `PNG_RGB_COLORMAP_ENTRIES+1+27 > image->colormap_entries` (pngread.c:2578-2579) | `png_error(png_ptr, "rgb-alpha color-map: too few entries")` — fatal | [x] |
| 655 | `png_image_read_colormap` | opaque RGB → color output: `PNG_RGB_COLORMAP_ENTRIES (216) > image->colormap_entries` (pngread.c:2663-2664) | `png_error(png_ptr, "rgb color-map: too few entries")` — fatal | [x] |
| 656 | `png_image_read_colormap` | palette image with `png_ptr->num_palette > 256` (pngread.c:2690-2692) | clamped: `cmap_entries = 256` | [x] |
| 657 | `png_image_read_colormap` | palette image: `cmap_entries > (unsigned int)image->colormap_entries` (pngread.c:2694-2695) | `png_error(png_ptr, "palette color-map: too few entries")` — fatal | [x] |
| 658 | `png_image_read_colormap` | `png_ptr->color_type` not one of the 5 valid PNG color types (switch `default:`) (pngread.c:2737-2738) | `png_error(png_ptr, "invalid PNG color type")` — fatal | [x] |
| 659 | `png_image_read_colormap` | `data_encoding` left as something other than `P_sRGB`/`P_FILE` (GNUC `default:`) (pngread.c:2759-2762) | `png_error(png_ptr, "bad data option (internal error)")` — fatal | [x] |
| 660 | `png_image_read_colormap` | `cmap_entries > 256 \|\| cmap_entries > image->colormap_entries` after building the map (pngread.c:2765-2766) | `png_error(png_ptr, "color map overflow (BAD internal error)")` — fatal | [x] |
| 661 | `png_image_read_colormap` | `output_processing` not one of the 5 `PNG_CMAP_*` values (switch `default:`) (pngread.c:2799-2800) | `png_error(png_ptr, "bad processing option (internal error)")` — fatal | [x] |
| 662 | `png_image_read_colormap` | `PNG_CMAP_NONE` but `background_index != PNG_CMAP_NONE_BACKGROUND (256)` (pngread.c:2773-2775, 2802-2803) | `goto bad_background` → `png_error(png_ptr, "bad background index (internal error)")` | [x] |
| 663 | `png_image_read_colormap` | `PNG_CMAP_GA` but `background_index != PNG_CMAP_GA_BACKGROUND (231)` (pngread.c:2778-2780, 2802-2803) | `png_error(png_ptr, "bad background index (internal error)")` | [x] |
| 664 | `png_image_read_colormap` | `PNG_CMAP_TRANS` but `background_index >= cmap_entries \|\| background_index != PNG_CMAP_TRANS_BACKGROUND (254)` (pngread.c:2783-2786, 2802-2803) | `png_error(png_ptr, "bad background index (internal error)")` | [x] |
| 665 | `png_image_read_colormap` | `PNG_CMAP_RGB` but `background_index != PNG_CMAP_RGB_BACKGROUND (256)` (pngread.c:2789-2791, 2802-2803) | `png_error(png_ptr, "bad background index (internal error)")` | [x] |
| 666 | `png_image_read_colormap` | `PNG_CMAP_RGB_ALPHA` but `background_index != PNG_CMAP_RGB_ALPHA_BACKGROUND (216)` (pngread.c:2794-2796, 2802-2803) | `png_error(png_ptr, "bad background index (internal error)")` | [x] |
| 667 | `png_image_read_and_map` | `png_ptr->interlaced` is neither `PNG_INTERLACE_NONE` nor `PNG_INTERLACE_ADAM7` (pngread.c:2825-2836) | `png_error(png_ptr, "unknown interlace type")` — fatal | [x] |
| 668 | `png_image_read_colormapped` | `PNG_CMAP_NONE` but result is not `(PALETTE\|GRAY)` with `info_ptr->bit_depth == 8` (pngread.c:3031-3036, 3073-3074) | `goto bad_output` → `png_error(png_ptr, "bad color-map processing (internal error)")` | [x] |
| 669 | `png_image_read_colormapped` | `PNG_CMAP_TRANS`/`PNG_CMAP_GA` but not `GRAY_ALPHA`, depth 8, `screen_gamma == PNG_GAMMA_sRGB`, `colormap_entries == 256` (pngread.c:3038-3050, 3073-3074) | `png_error(png_ptr, "bad color-map processing (internal error)")` | [x] |
| 670 | `png_image_read_colormapped` | `PNG_CMAP_RGB` but not `RGB`, depth 8, sRGB screen gamma, `colormap_entries == 216` (pngread.c:3052-3060, 3073-3074) | `png_error(png_ptr, "bad color-map processing (internal error)")` | [x] |
| 671 | `png_image_read_colormapped` | `PNG_CMAP_RGB_ALPHA` but not `RGB_ALPHA`, depth 8, sRGB screen gamma, `colormap_entries == 244` (pngread.c:3062-3070, 3073-3074) | `png_error(png_ptr, "bad color-map processing (internal error)")` | [x] |
| 672 | `png_image_read_colormapped` | `display->colormap_processing` is not one of the 5 `PNG_CMAP_*` values (switch `default:`) (pngread.c:3072-3074) | `png_error(png_ptr, "bad color-map processing (internal error)")` | [x] |
| 673 | `png_image_read_direct_scaled` | `png_ptr->interlaced` neither `PNG_INTERLACE_NONE` nor `PNG_INTERLACE_ADAM7` (pngread.c:3148-3159) | `png_error(png_ptr, "unknown interlace type")` — fatal | [x] |
| 674 | `png_image_read_composite` | `png_ptr->interlaced` neither `PNG_INTERLACE_NONE` nor `PNG_INTERLACE_ADAM7` (pngread.c:3197-3208) | `png_error(png_ptr, "unknown interlace type")` — fatal | [x] |
| 675 | `png_image_read_composite` | optimized-alpha path where the composed value exceeds the linear range: `component > 255*65535` (data not linear-premultiplied; CVE-2025-66293 hardening) (pngread.c:3290-3291) | clamped to `255*65535` before `PNG_sRGB_FROM_LINEAR` | [x] |
| 676 | `png_image_read_composite` | non-optimized path where `component > 255` after compositing (pngread.c:3309-3310) | clamped to `255` | [x] |
| 677 | `png_image_read_background` | `(png_ptr->transformations & PNG_RGB_TO_GRAY) == 0` on entry (pngread.c:3357-3358) | `png_error(png_ptr, "lost rgb to gray")` — fatal | [x] |
| 678 | `png_image_read_background` | `(png_ptr->transformations & PNG_COMPOSE) != 0` on entry (pngread.c:3360-3361) | `png_error(png_ptr, "unexpected compose")` — fatal | [x] |
| 679 | `png_image_read_background` | `png_get_channels(png_ptr, info_ptr) != 2` (pngread.c:3363-3364) | `png_error(png_ptr, "lost/gained channels")` — fatal | [x] |
| 680 | `png_image_read_background` | 8-bit output that still carries alpha: `(image->format & PNG_FORMAT_FLAG_LINEAR) == 0 && (image->format & PNG_FORMAT_FLAG_ALPHA) != 0` (pngread.c:3367-3369) | `png_error(png_ptr, "unexpected 8-bit transformation")` — fatal | [x] |
| 681 | `png_image_read_background` | `png_ptr->interlaced` neither `PNG_INTERLACE_NONE` nor `PNG_INTERLACE_ADAM7` (pngread.c:3371-3382) | `png_error(png_ptr, "unknown interlace type")` — fatal | [x] |
| 682 | `png_image_read_background` | `info_ptr->bit_depth` neither 8 nor 16 (GNUC `default:`) (pngread.c:3390, 3607-3610) | `png_error(png_ptr, "unexpected bit depth")` — fatal | [x] |
| 683 | `png_image_read_direct` | requested `image->format` needs a transform libpng cannot supply: `change != 0` after all transform handling (pngread.c:3924-3925) | `png_error(png_ptr, "png_read_image: unsupported transformation")` — fatal | [x] |
| 684 | `png_image_read_direct` | `do_local_compose != 0` yet `info_ptr->color_type` has no alpha channel (pngread.c:3959-3960) | `png_error(png_ptr, "png_image_read: alpha channel lost")` — fatal | [x] |
| 685 | `png_image_read_direct` | `do_local_background == 2` while libpng has `PNG_SWAP_ALPHA`/front-filler `PNG_ADD_ALPHA` set (pngread.c:3981-3986) | `png_error(png_ptr, "unexpected alpha swap transformation")` — fatal | [x] |
| 686 | `png_image_read_direct` | the format libpng will actually produce does not match the requested one: `info_format != format` (pngread.c:3993-3994) | `png_error(png_ptr, "png_read_image: invalid transformations")` — fatal | [x] |
| 687 | `png_image_finish_read` | `image->width > 0x7fffffffU/channels` (row stride cannot be represented in a signed 32-bit value) (pngread.c:4105, 4192-4194) | `png_image_error(image, "png_image_finish_read: row_stride too large")` → returns 0 | [x] |
| 688 | `png_image_finish_read` | `image->opaque == NULL \|\| buffer == NULL \|\| check < png_row_stride` (no begin_read, no output buffer, or stride smaller than one row) (pngread.c:4123, 4187-4189) | `png_image_error(image, "png_image_finish_read: invalid argument")` → returns 0 | [x] |
| 689 | `png_image_finish_read` | `image->height > 0xffffffffU/PNG_IMAGE_PIXEL_COMPONENT_SIZE(image->format)/check` (buffer-size calculation overflows 32 bits) (pngread.c:4141-4142, 4182-4184) | `png_image_error(image, "png_image_finish_read: image too large")` → returns 0 | [x] |
| 690 | `png_image_finish_read` | color-mapped output requested but `image->colormap_entries == 0` or `colormap == NULL` (pngread.c:4144-4145, 4177-4179) | `png_image_error(image, "png_image_finish_read[color-map]: no color-map")` → returns 0 | [x] |
| 691 | `png_image_finish_read` | `image->version != PNG_IMAGE_VERSION` (pngread.c:4091, 4197-4199) | `png_image_error(image, "png_image_finish_read: damaged PNG_IMAGE_VERSION")` → returns 0 | [x] |
| 692 | `png_image_finish_read` | `image == NULL` (pngread.c:4091, 4201) | `return 0` | [x] |
| 693 | `png_set_crc_action` | `png_ptr == NULL` (pngrtran.c:45-46) | silent `return` | [x] |
| 694 | `png_set_crc_action` | `crit_action == PNG_CRC_WARN_DISCARD` (discarding critical data is not allowed) (pngrtran.c:65-67) | `png_warning(png_ptr, "Can't discard critical data on CRC error")`, then falls through to the default (error/quit) behavior | [x] |
| 695 | `png_set_crc_action` | `crit_action` is not a recognized `PNG_CRC_*` value (switch `default:`) (pngrtran.c:71-74) | silently reset to default: `png_ptr->flags &= ~PNG_FLAG_CRC_CRITICAL_MASK` | [x] |
| 696 | `png_set_crc_action` | `ancil_action` is not a recognized `PNG_CRC_*` value (switch `default:`) (pngrtran.c:101-104) | silently reset to default: `png_ptr->flags &= ~PNG_FLAG_CRC_ANCILLARY_MASK` | [x] |
| 697 | `png_rtran_ok` | any read-transform setter called after row init: `(png_ptr->flags & PNG_FLAG_ROW_INIT) != 0` (pngrtran.c:119-121) | `png_app_error(png_ptr, "invalid after png_start_read_image or png_read_update_info")`, `return 0` | [x] |
| 698 | `png_rtran_ok` | transform requiring IHDR called too early: `need_IHDR && (png_ptr->mode & PNG_HAVE_IHDR) == 0` (pngrtran.c:123-124) | `png_app_error(png_ptr, "invalid before the PNG header has been read")`, `return 0` | [x] |
| 699 | `png_rtran_ok` | `png_ptr == NULL` (pngrtran.c:117, 135) | `return 0` (no `png_error` possible); caller aborts silently | [x] |
| 700 | `png_set_background_fixed` | `png_rtran_ok(png_ptr, 0) == 0` or `background_color == NULL` (pngrtran.c:148-149) | `return`; background not set | [x] |
| 701 | `png_set_background_fixed` | `background_gamma_code == PNG_BACKGROUND_GAMMA_UNKNOWN` (pngrtran.c:151-155) | `png_warning(png_ptr, "Application must supply a known background gamma")` and `return` | [x] |
| 702 | `png_set_scale_16` | `png_rtran_ok(png_ptr, 0) == 0` (NULL ptr, or after row init) (pngrtran.c:192-193) | `return`; `PNG_SCALE_16_TO_8` not set | [x] |
| 703 | `png_set_strip_16` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:206-207) | `return`; `PNG_16_TO_8` not set | [x] |
| 704 | `png_set_strip_alpha` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:219-220) | `return`; `PNG_STRIP_ALPHA` not set | [x] |
| 705 | `convert_gamma_value` | `output_gamma > PNG_FP_MAX \|\| output_gamma < PNG_FP_MIN` after scaling/rounding (pngrtran.c:324-325) | `png_fixed_error(png_ptr, "gamma value")` — fatal | [x] |
| 706 | `unsupported_gamma` | `gamma < PNG_LIB_GAMMA_MIN \|\| gamma > PNG_LIB_GAMMA_MAX` with `warn != 0` (called from `png_set_gamma_fixed`) (pngrtran.c:344-351) | `png_app_warning(png_ptr, "gamma out of supported range")`, returns 1 → caller returns without setting gamma | [x] |
| 707 | `unsupported_gamma` | `gamma < PNG_LIB_GAMMA_MIN \|\| gamma > PNG_LIB_GAMMA_MAX` with `warn == 0` (called from `png_set_alpha_mode_fixed`) (pngrtran.c:344-350) | `png_app_error(png_ptr, "gamma out of supported range")`, returns 1 → caller returns | [x] |
| 708 | `png_set_alpha_mode_fixed` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:369-370) | `return`; alpha mode not set | [x] |
| 709 | `png_set_alpha_mode_fixed` | translated `output_gamma` outside `[PNG_LIB_GAMMA_MIN, PNG_LIB_GAMMA_MAX]` (pngrtran.c:372-374) | `png_app_error` via `unsupported_gamma`, then `return` | [x] |
| 710 | `png_set_alpha_mode_fixed` | `mode` not one of `PNG_ALPHA_PNG/ASSOCIATED/OPTIMIZED/BROKEN` (switch `default:`) (pngrtran.c:433-434) | `png_error(png_ptr, "invalid alpha mode")` — fatal | [x] |
| 711 | `png_set_alpha_mode_fixed` | pre-multiplying mode requested when `PNG_COMPOSE` is already set (i.e. `png_set_background` already called) (pngrtran.c:451-453) | `png_error(png_ptr, "conflicting calls to set alpha mode and background")` — fatal | [x] |
| 712 | `png_set_quantize` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:495-496) | `return`; quantize not enabled | [x] |
| 713 | `png_set_quantize` | `palette == NULL` (pngrtran.c:498-499) | `return`; quantize not enabled | [x] |
| 714 | `png_set_quantize` | `num_palette > maximum_colors` (pngrtran.c:524, 814) | palette reduced in place; `num_palette = maximum_colors` (no diagnostic) | [x] |
| 715 | `png_set_quantize` | `png_malloc_warn` for a `png_dsort` node returns `NULL` (`t == NULL`) during the no-histogram reduction (pngrtran.c:708-712, 720-721) | loops `break` out early; reduction abandoned for that pass, no error reported | [x] |
| 716 | `png_set_gamma_fixed` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:898-899) | `return`; gamma not set | [x] |
| 717 | `png_set_gamma_fixed` | `file_gamma <= 0` after flag translation (pngrtran.c:916-917) | `png_app_error(png_ptr, "invalid file gamma in png_set_gamma")` | [x] |
| 718 | `png_set_gamma_fixed` | `scrn_gamma <= 0` after flag translation (pngrtran.c:918-919) | `png_app_error(png_ptr, "invalid screen gamma in png_set_gamma")` | [x] |
| 719 | `png_set_gamma_fixed` | `file_gamma` outside `[PNG_LIB_GAMMA_MIN, PNG_LIB_GAMMA_MAX]` (pngrtran.c:921) | `png_app_warning(png_ptr, "gamma out of supported range")` then `return`; neither gamma stored | [x] |
| 720 | `png_set_gamma_fixed` | `scrn_gamma` outside `[PNG_LIB_GAMMA_MIN, PNG_LIB_GAMMA_MAX]` (pngrtran.c:922) | `png_app_warning(png_ptr, "gamma out of supported range")` then `return`; neither gamma stored | [x] |
| 721 | `png_set_expand` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:953-954) | `return`; `PNG_EXPAND\|PNG_EXPAND_tRNS` not set | [x] |
| 722 | `png_set_palette_to_rgb` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:983-984) | `return`; transform not set | [x] |
| 723 | `png_set_expand_gray_1_2_4_to_8` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:995-996) | `return`; `PNG_EXPAND` not set | [x] |
| 724 | `png_set_tRNS_to_alpha` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:1007-1008) | `return`; transform not set | [x] |
| 725 | `png_set_expand_16` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:1023-1024) | `return`; `PNG_EXPAND_16` not set | [x] |
| 726 | `png_set_gray_to_rgb` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:1036-1037) | `return`; `PNG_GRAY_TO_RGB` not set | [x] |
| 727 | `png_set_rgb_to_gray_fixed` | `png_rtran_ok(png_ptr, 1) == 0` (NULL ptr, after row init, or before IHDR) (pngrtran.c:1054-1055) | `return`; rgb-to-gray not set | [x] |
| 728 | `png_set_rgb_to_gray_fixed` | `error_action` not `PNG_ERROR_ACTION_NONE/WARN/ERROR` (switch `default:`) (pngrtran.c:1071-1072) | `png_error(png_ptr, "invalid error action to rgb_to_gray")` — fatal | [x] |
| 729 | `png_set_rgb_to_gray_fixed` | `png_ptr->color_type == PNG_COLOR_TYPE_PALETTE` in a build without `PNG_READ_EXPAND_SUPPORTED` (pngrtran.c:1075-1088) | `png_error(png_ptr, "Cannot do RGB_TO_GRAY without EXPAND_SUPPORTED")` — fatal | [x] |
| 730 | `png_set_rgb_to_gray_fixed` | `red >= 0 && green >= 0` but `red + green > PNG_FP_1` (pngrtran.c:1090, 1107-1109) | `png_app_warning(png_ptr, "ignoring out of range rgb_to_gray coefficients")`; default coefficients kept | [x] |
| 731 | `png_set_rgb_to_gray_fixed` | `red < 0` or `green < 0` (pngrtran.c:1090, 1107) | neither branch taken: coefficients silently left at their defaults, no diagnostic | [x] |
| 732 | `png_resolve_file_gamma` | `file_gamma`, `chunk_gamma`, `default_gamma` and `screen_gamma` are all 0 (or `png_reciprocal` overflows) (pngrtran.c:1365-1384) | returns 0 → "no usable file gamma"; callers must treat gamma handling as disabled | [x] |
| 733 | `png_init_gamma_values` | resolved `file_gamma <= 0` (nothing set) (pngrtran.c:1402, 1414-1415) | `file_gamma = screen_gamma = PNG_FP_1`; gamma correction suppressed (returns 0) | [x] |
| 734 | `png_init_read_transformations` | `PNG_STRIP_ALPHA` set with no `PNG_COMPOSE` (pngrtran.c:1491-1510) | `PNG_BACKGROUND_EXPAND\|PNG_ENCODE_ALPHA\|PNG_EXPAND_tRNS` cleared and `png_ptr->num_trans = 0`; tRNS data silently discarded | [x] |
| 735 | `png_init_read_transformations` | gamma correction combined with background composition and rgb-to-gray: `PNG_COMPOSE` and `PNG_RGB_TO_GRAY` both set with gamma tables built (pngrtran.c:1696-1698) | `png_warning(png_ptr, "libpng does not support gamma+background+rgb_to_gray")`; result is double gamma corrected | [x] |
| 736 | `png_init_read_transformations` | `png_ptr->background_gamma_type` not `SCREEN`/`FILE`/`UNIQUE` for a non-palette image (switch `default:`) (pngrtran.c:1885-1886) | `png_error(png_ptr, "invalid background gamma type")` — fatal | [x] |
| 737 | `png_init_read_transformations` | palette + `PNG_SHIFT` where red sBIT gives `shift = 8 - sig_bit.red` outside `0 < shift < 8` (e.g. `sig_bit.red == 0` or `>= 8`) (pngrtran.c:2025, 2033) | shift silently not applied to red palette entries ("error condition which is silently ignored") | [x] |
| 738 | `png_init_read_transformations` | palette + `PNG_SHIFT` where `shift = 8 - sig_bit.green` is outside `0 < shift < 8` (pngrtran.c:2042-2043) | shift silently not applied to green palette entries | [x] |
| 739 | `png_init_read_transformations` | palette + `PNG_SHIFT` where `shift = 8 - sig_bit.blue` is outside `0 < shift < 8` (pngrtran.c:2052-2053) | shift silently not applied to blue palette entries | [x] |
| 740 | `png_read_transform_info` | `PNG_EXPAND` on a palette image with `png_ptr->palette == NULL` (pngrtran.c:2086-2104) | `png_error(png_ptr, "Palette is NULL in indexed image")` — fatal | [x] |
| 741 | `png_do_unshift` | for any channel `shift[c] <= 0 \|\| shift[c] >= bit_depth` (sBIT value 0, or >= bit depth) (pngrtran.c:2427-2433) | that channel's `shift[c]` forced to 0 (invalid sBIT silently ignored) | [x] |
| 742 | `png_do_unshift` | all channels end up with zero shift: `have_shift == 0` (pngrtran.c:2439-2440) | early `return`; row unchanged | [x] |
| 743 | `png_do_unshift` | `bit_depth` is 1 (or otherwise unexpected) — switch `default:` "should not be here" (pngrtran.c:2443-2448) | `break` with no processing; row left unshifted | [x] |
| 744 | `png_do_encode_alpha` | called with a row that has no alpha channel, or bit depth not 8/16, or the required `gamma_from_1`/`gamma_16_from_1` table is `NULL` (pngrtran.c:4292-4341) | `png_warning(png_ptr, "png_do_encode_alpha: unexpected call")`; row not encoded | [x] |
| 745 | `png_do_expand_palette` | palette index in the row is >= `num_trans`: `(int)(*sp) >= num_trans` while expanding tRNS (pngrtran.c:4471-4476) | alpha defaulted to `0xff` (opaque) instead of reading past `trans_alpha[]` | [x] |
| 746 | `png_do_read_transformations` | `png_ptr->row_buf == NULL` (pngrtran.c:4885-4891) | `png_error(png_ptr, "NULL row buffer")` — fatal | [x] |
| 747 | `png_do_read_transformations` | transforms set but neither `png_start_read_image` nor `png_read_update_info` called: `(flags & PNG_FLAG_DETECT_UNINITIALIZED) != 0 && (flags & PNG_FLAG_ROW_INIT) == 0` (pngrtran.c:4900-4907) | `png_error(png_ptr, "Uninitialized row")` — fatal | [x] |
| 748 | `png_do_read_transformations` | non-gray pixel found during rgb-to-gray with `PNG_RGB_TO_GRAY_WARN` requested (pngrtran.c:4960-4965) | `png_warning(png_ptr, "png_do_rgb_to_gray found nongray pixel")`; `rgb_to_gray_status = 1` | [x] |
| 749 | `png_do_read_transformations` | non-gray pixel found during rgb-to-gray with `PNG_RGB_TO_GRAY_ERR` requested (pngrtran.c:4967-4969) | `png_error(png_ptr, "png_do_rgb_to_gray found nongray pixel")` — fatal | [x] |
| 750 | `png_do_read_transformations` | palette row with index checking enabled: `row_info->color_type == PNG_COLOR_TYPE_PALETTE && png_ptr->num_palette_max >= 0` (pngrtran.c:5117-5119) | `png_do_check_palette_indexes` records the max index; out-of-range indices reported later as `"Read palette index exceeding num_palette"` | [x] |
| 751 | `png_process_data` | `png_ptr == NULL \|\| info_ptr == NULL` (pngpread.c:53-54) | silent `return`; supplied buffer ignored | [x] |
| 752 | `png_process_data_pause` | `png_ptr == NULL` (pngpread.c:67, 88) | `return 0` | [x] |
| 753 | `png_process_data_pause` | `save == 0` and `png_ptr->save_buffer_size >= remaining` (all pending data is in the save buffer) (pngpread.c:83-88) | `return 0` (no bytes handed back to the caller) | [x] |
| 754 | `png_process_data_skip` | any call — the API is unimplemented (pngpread.c:99-101) | `png_app_warning(png_ptr, "png_process_data_skip is not implemented in any current version of libpng")`, `return 0` | [x] |
| 755 | `png_process_some_data` | `png_ptr == NULL` (pngpread.c:110-111) | silent `return` | [x] |
| 756 | `png_process_some_data` | `png_ptr->process_mode` is not SIG/CHUNK/IDAT mode (e.g. `PNG_READ_DONE_MODE`, `PNG_ERROR_MODE`, tEXt/zTXt/iTXt modes) — switch `default:` (pngpread.c:133-137) | `png_ptr->buffer_size = 0`; remaining input silently discarded | [x] |
| 757 | `png_push_read_sig` | signature mismatch within the first 4 bytes: `png_sig_cmp(...) != 0 && num_checked < 4 && png_sig_cmp(signature, num_checked, num_to_check - 4) != 0` (pngpread.c:162-166) | `png_error(png_ptr, "Not a PNG file")` — fatal | [x] |
| 758 | `png_push_read_sig` | signature mismatch only in the later bytes (CR/LF mangling) (pngpread.c:162, 168-169) | `png_error(png_ptr, "PNG file corrupted by ASCII conversion")` — fatal | [x] |
| 759 | `png_push_read_chunk` | fewer than 8 buffered bytes for the chunk length+tag: `png_ptr->buffer_size < 8` (`PNG_PUSH_SAVE_BUFFER_IF_LT(8)`) (pngpread.c:196, 30-32) | data saved via `png_push_save_buffer`, `return`; caller must supply more data | [x] |
| 760 | `png_push_read_chunk` | `IDAT` reached with `(png_ptr->mode & PNG_HAVE_IHDR) == 0` (pngpread.c:212-213) | `png_error(png_ptr, "Missing IHDR before IDAT")` — fatal | [x] |
| 761 | `png_push_read_chunk` | `IDAT` with `png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && (png_ptr->mode & PNG_HAVE_PLTE) == 0` (pngpread.c:215-217) | `png_error(png_ptr, "Missing PLTE before IDAT")` — fatal | [x] |
| 762 | `png_push_read_chunk` | zero-length `IDAT` after IDAT already seen and no intervening chunk: `HAVE_IDAT && !HAVE_CHUNK_AFTER_IDAT && push_length == 0` (pngpread.c:221-224) | early `return` (chunk header left pending); chunk not processed | [x] |
| 763 | `png_push_read_chunk` | `IDAT` encountered when `(png_ptr->mode & PNG_AFTER_IDAT) != 0` (pngpread.c:228-229) | `png_benign_error(png_ptr, "Too many IDATs found")` | [x] |
| 764 | `png_push_read_chunk` | `IHDR` whose `png_ptr->push_length != 13` (pngpread.c:242-243) | `png_error(png_ptr, "Invalid IHDR length")` — fatal | [x] |
| 765 | `png_push_read_chunk` | whole chunk + CRC not yet buffered: `png_ptr->push_length + 4 > png_ptr->buffer_size` (`PNG_PUSH_SAVE_BUFFER_IF_FULL` for IHDR/IEND/unknown/other chunks) (pngpread.c:245, 251, 261, 283; macro at 27-29) | data saved via `png_push_save_buffer`, `return`; chunk retried when more data arrives | [x] |
| 766 | `png_push_read_IDAT` | fewer than 8 buffered bytes for the next chunk header: `buffer_size < 8` (`PNG_PUSH_SAVE_BUFFER_IF_LT(8)`) (pngpread.c:412) | data saved, `return` | [x] |
| 767 | `png_push_read_IDAT` | next chunk is not `IDAT` while the zlib stream has not ended: `chunk_name != png_IDAT && (png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED) == 0` (pngpread.c:420-425) | `png_error(png_ptr, "Not enough compressed data")` — fatal | [x] |
| 768 | `png_push_read_IDAT` | fewer than 4 buffered bytes for the IDAT CRC: `buffer_size < 4` (`PNG_PUSH_SAVE_BUFFER_IF_LT(4)`) (pngpread.c:488) | data saved, `return` | [x] |
| 769 | `png_push_save_buffer` | save-buffer growth would overflow: `png_ptr->save_buffer_size > PNG_SIZE_MAX - (png_ptr->current_buffer_size + 256)` (pngpread.c:358-361) | `png_error(png_ptr, "Potential overflow of save_buffer")` — fatal | [x] |
| 770 | `png_push_save_buffer` | `png_malloc_warn(new_max)` returns `NULL` (pngpread.c:366-373) | old buffer freed, `png_error(png_ptr, "Insufficient memory for save_buffer")` — fatal | [x] |
| 771 | `png_push_save_buffer` | inconsistent state: `old_buffer == NULL` while `png_ptr->save_buffer_size != 0` (pngpread.c:375-378) | `png_error(png_ptr, "save_buffer error")` — fatal | [x] |
| 772 | `png_push_fill_buffer` | `png_ptr == NULL` (pngpread.c:295-296) | silent `return`; caller's buffer left unfilled | [x] |
| 773 | `png_process_IDAT_data` | `!(buffer_length > 0) \|\| buffer == NULL` (pngpread.c:501-502) | `png_error(png_ptr, "No IDAT data (internal error)")` — fatal | [x] |
| 774 | `png_process_IDAT_data` | zlib returns neither `Z_OK` nor `Z_STREAM_END` and all rows are already done: `png_ptr->row_number >= png_ptr->num_rows \|\| png_ptr->pass > 6` (pngpread.c:544-555) | zstream marked ended; `png_warning(png_ptr, "Truncated compressed data in IDAT")`, `return` | [x] |
| 775 | `png_process_IDAT_data` | zlib returns `Z_DATA_ERROR` while rows are still expected (pngpread.c:559-560) | `png_benign_error(png_ptr, "IDAT: ADLER32 checksum mismatch")`, `return` | [x] |
| 776 | `png_process_IDAT_data` | zlib returns any other failure while rows are still expected (pngpread.c:561-562) | `png_error(png_ptr, "Decompression error in IDAT")` — fatal | [x] |
| 777 | `png_process_IDAT_data` | inflate produced output after the last row: `next_out != row_buf` and `row_number >= num_rows \|\| pass > 6` (pngpread.c:570-580) | `png_warning(png_ptr, "Extra compressed data in IDAT")`; zstream force-ended, `return` | [x] |
| 778 | `png_process_IDAT_data` | bytes left after the zlib end code: `png_ptr->zstream.avail_in > 0` on exit (pngpread.c:604-605) | `png_warning(png_ptr, "Extra compression data in IDAT")` | [x] |
| 779 | `png_push_process_row` | filter byte `png_ptr->row_buf[0] >= PNG_FILTER_VALUE_LAST` (pngpread.c:621-627) | `png_error(png_ptr, "bad adaptive filter value")` — fatal | [x] |
| 780 | `png_push_process_row` | first row's `row_info.pixel_depth > png_ptr->maximum_pixel_depth` after transforms (pngpread.c:643-647) | `png_error(png_ptr, "progressive row overflow")` — fatal | [x] |
| 781 | `png_push_process_row` | later row's `png_ptr->transformed_pixel_depth != row_info.pixel_depth` (pngpread.c:650-651) | `png_error(png_ptr, "internal progressive row size calculation error")` — fatal | [x] |
| 782 | `png_read_push_finish_row` | interlace pass counter runs past the last pass: `png_ptr->pass > 7` (pngpread.c:859-860) | clamped with `png_ptr->pass--`, then loop `break` at `pass >= 7` | [x] |
| 783 | `png_progressive_combine_row` | `png_ptr == NULL` (pngpread.c:910-911) | silent `return`; no combining done | [x] |
| 784 | `png_progressive_combine_row` | `new_row == NULL` (callback was invoked for an empty interlace row) (pngpread.c:917-918) | `png_combine_row` not called; `old_row` left unchanged | [x] |
| 785 | `png_set_progressive_read_fn` | `png_ptr == NULL` (pngpread.c:927-928) | silent `return`; callbacks not installed | [x] |
| 786 | `png_get_progressive_ptr` | `png_ptr == NULL` (pngpread.c:940-941) | `return NULL` | [x] |

## pngrutil.c / pngwrite.c / pngwutil.c / pngwtran.c

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 787 | `png_get_uint_31` | 4-byte big-endian value with the top bit set, i.e. `uval > PNG_UINT_31_MAX` (pngrutil.c:45) | `png_error(png_ptr, "PNG unsigned integer out of range")` — longjmp, read aborted | [x] |
| 788 | `png_get_int_32` | two's-complement value `0x80000000` (negation overflows: `(uval & 0x80000000) != 0` after negate, pngrutil.c:87-93) | silently returns `0` (data known invalid) | [x] |
| 789 | `png_read_sig` | first 8 bytes are not the PNG signature and the mismatch is in the first 4 bytes (`num_checked < 4 && png_sig_cmp(...) != 0`, pngrutil.c:137-139) | `png_error(png_ptr, "Not a PNG file")` | [x] |
| 790 | `png_read_sig` | signature mismatch confined to the CR/LF/^Z/LF trailer bytes (pngrutil.c:140-141) | `png_error(png_ptr, "PNG file corrupted by ASCII conversion")` | [x] |
| 791 | `check_chunk_name` | any of the 4 chunk-type bytes is not in `A-Z`/`a-z` (bit-whack test `(t & 0xe0e0e0e0U) == 0U` fails, pngrutil.c:152-177) | returns `0` (invalid name) — caller errors | [x] |
| 792 | `png_read_chunk_header` | chunk length field with high bit set in the first byte: `buf[0] >= 0x80U` (pngrutil.c:210-211) | `png_chunk_error(png_ptr, "bad header (invalid length)")` | [x] |
| 793 | `png_read_chunk_header` | chunk type containing non-alphabetic bytes: `!check_chunk_name(chunk_name)` (pngrutil.c:214-215) | `png_chunk_error(png_ptr, "bad header (invalid type)")` | [x] |
| 794 | `png_crc_read` | `png_ptr == NULL` (pngrutil.c:228-229) | returns immediately, no read performed | [x] |
| 795 | `png_crc_error` | stored chunk CRC differs from the computed CRC: `crc != png_ptr->crc` (pngrutil.c:293-294) | returns non-zero (CRC error) to `png_crc_finish_critical` | [x] |
| 796 | `png_crc_error` | ancillary chunk with `PNG_FLAG_CRC_ANCILLARY_USE\|NOWARN`, or critical chunk with `PNG_FLAG_CRC_CRITICAL_IGNORE` (pngrutil.c:271-282, 297-298) | CRC not computed; returns `0` — corrupt data accepted by app request | [x] |
| 797 | `png_crc_finish_critical` | CRC error on an ancillary chunk (or `handle_as_ancillary`) without `PNG_FLAG_CRC_ANCILLARY_NOWARN` (pngrutil.c:342-348) | `png_chunk_warning(png_ptr, "CRC error")`, returns `1` (chunk discarded) | [x] |
| 798 | `png_crc_finish_critical` | CRC error on a critical chunk with default flags (`PNG_FLAG_CRC_CRITICAL_USE` not set) (pngrutil.c:350-351) | `png_chunk_error(png_ptr, "CRC error")` | [x] |
| 799 | `png_read_buffer` | requested chunk buffer bigger than the configured limit: `new_size > png_chunk_max(png_ptr)` (pngrutil.c:380) | returns `NULL`; callers emit `png_chunk_benign_error(png_ptr, "out of memory")` | [x] |
| 800 | `png_read_buffer` | `png_malloc_base` returns `NULL` (out of memory) (pngrutil.c:392-404) | returns `NULL` | [x] |
| 801 | `png_inflate_claim` | zstream already owned by another chunk: `png_ptr->zowner != 0`, release build (pngrutil.c:416-428) | `png_chunk_warning(png_ptr, "<cHNK> using zstream")`, ownership stolen | [x] |
| 802 | `png_inflate_claim` | `png_ptr->zowner != 0`, non-release build (pngrutil.c:429-431) | `png_chunk_error(png_ptr, "<cHNK> using zstream")` | [x] |
| 803 | `png_inflate_claim` | `inflateInit2`/`inflateReset2` fails (e.g. `Z_MEM_ERROR`) (pngrutil.c:476-509) | `png_zstream_error`, returns the zlib error code (not `Z_OK`) | [x] |
| 804 | `png_zlib_inflate` | first deflate header byte encodes a window size > 32K: `(*next_in >> 4) > 7` (pngrutil.c:527-534) | sets `zstream.msg = "invalid window size (libpng)"`, returns `Z_DATA_ERROR` | [x] |
| 805 | `png_inflate` | called while the stream is owned by a different chunk: `png_ptr->zowner != owner` (pngrutil.c:560, 662-670) | `zstream.msg = "zstream unclaimed"`, returns `Z_STREAM_ERROR` | [x] |
| 806 | `png_inflate` | `inflate()` returns any error (corrupt/truncated LZ data) (pngrutil.c:636-638, 658-659) | `png_zstream_error`, returns the zlib code (caller rejects the chunk) | [x] |
| 807 | `png_decompress_chunk` | chunk prefix alone already exceeds the memory limit: `limit < prefix_size + (terminate != 0)` (pngrutil.c:695, 821-826) | `png_zstream_error(png_ptr, Z_MEM_ERROR)`, returns `Z_MEM_ERROR` | [x] |
| 808 | `png_decompress_chunk` | `png_inflate_claim` fails (`ret != Z_OK`) (pngrutil.c:705-707, 815-818) | returns the zlib error code; `Z_STREAM_END` is mapped to `PNG_UNEXPECTED_ZLIB_RETURN` | [x] |
| 809 | `png_decompress_chunk` | first (sizing) `png_inflate` returns `Z_OK` instead of `Z_STREAM_END` (truncated LZ stream) (pngrutil.c:808-809) | `ret = PNG_UNEXPECTED_ZLIB_RETURN` | [x] |
| 810 | `png_decompress_chunk` | `inflateReset` fails after the sizing pass (pngrutil.c:724, 800-805) | `png_zstream_error`, `ret = PNG_UNEXPECTED_ZLIB_RETURN` | [x] |
| 811 | `png_decompress_chunk` | `png_malloc_base(buffer_size)` for the decompressed text fails (pngrutil.c:737, 792-797) | `ret = Z_MEM_ERROR`, `png_zstream_error(png_ptr, Z_MEM_ERROR)` | [x] |
| 812 | `png_decompress_chunk` | second inflate pass produces a different length: `new_size != *newlength` (pngrutil.c:747, 764-773) | `ret = PNG_UNEXPECTED_ZLIB_RETURN` ("unexpected end of LZ stream") | [x] |
| 813 | `png_decompress_chunk` | second inflate pass returns `Z_OK` (output buffer not consumed) (pngrutil.c:776-777) | `ret = PNG_UNEXPECTED_ZLIB_RETURN` | [x] |
| 814 | `png_decompress_chunk` | compressed bytes left over after `Z_STREAM_END`: `chunklength - prefix_size != lzsize` (pngrutil.c:787-789) | `png_chunk_benign_error(png_ptr, "extra compressed data")` | [x] |
| 815 | `png_inflate_read` | stream not owned by the current chunk: `png_ptr->zowner != png_ptr->chunk_name` (pngrutil.c:840, 890-894) | `zstream.msg = "zstream unclaimed"`, returns `Z_STREAM_ERROR` | [x] |
| 816 | `png_handle_IHDR` | width or height field with the top bit set (via `png_get_uint_31`, pngrutil.c:917-918) | `png_error(png_ptr, "PNG unsigned integer out of range")` | [x] |
| 817 | `png_handle_IHDR` | invalid bit depth / colour type / compression / filter / interlace combination — delegated to `png_set_IHDR` (pngrutil.c:965-969; the `default:` colour-type case at pngrutil.c:939 just guesses 1 channel) | `png_error` raised inside `png_set_IHDR` | [x] |
| 818 | `png_handle_PLTE` | a second PLTE: `(png_ptr->mode & PNG_HAVE_PLTE) != 0` (pngrutil.c:992-993) | `errmsg = "duplicate"` → error path below | [x] |
| 819 | `png_handle_PLTE` | PLTE after IDAT: `(png_ptr->mode & PNG_HAVE_IDAT) != 0` (pngrutil.c:995-996) | `errmsg = "out of place"` | [x] |
| 820 | `png_handle_PLTE` | PLTE in a greyscale image: `(png_ptr->color_type & PNG_COLOR_MASK_COLOR) == 0` (pngrutil.c:998-999) | `errmsg = "ignored in grayscale PNG"` | [x] |
| 821 | `png_handle_PLTE` | `length > 3*PNG_MAX_PALETTE_LENGTH` or `(length % 3) != 0` (pngrutil.c:1001-1002) | `errmsg = "invalid"` | [x] |
| 822 | `png_handle_PLTE` | PLTE in a truecolour image after tRNS or bKGD was seen (pngrutil.c:1015-1017) | `errmsg = "out of place"` (PLTE dropped in favour of tRNS/bKGD) | [x] |
| 823 | `png_handle_PLTE` | any of the above with `color_type == PNG_COLOR_TYPE_PALETTE` (PLTE critical) (pngrutil.c:1061-1064) | `png_crc_finish` then `png_chunk_error(png_ptr, errmsg)` | [x] |
| 824 | `png_handle_PLTE` | any of the above for a non-colour-mapped image (pngrutil.c:1067-1076) | `png_chunk_benign_error(png_ptr, errmsg)`, returns `handled_error` | [x] |
| 825 | `png_handle_PLTE` | palette larger than the bit depth allows but `<= 256` entries: `length > 3U*max_palette_length` (pngrutil.c:1026-1034) | no error; extra entries silently truncated to `1U << bit_depth` | [x] |
| 826 | `png_handle_IEND` | IEND with data: `length != 0` (pngrutil.c:1091-1092) | `png_chunk_benign_error(png_ptr, "invalid")` (still returns `handled_ok`) | [x] |
| 827 | `png_handle_gAMA` | CRC failure on gAMA: `png_crc_finish(png_ptr, 0) != 0` (pngrutil.c:1111-1112) | returns `handled_error`, chunk discarded | [x] |
| 828 | `png_handle_gAMA` | gamma value with top bit set: `ugamma > PNG_UINT_31_MAX` (pngrutil.c:1116-1120) | `png_chunk_benign_error(png_ptr, "invalid")`, `handled_error` | [x] |
| 829 | `png_handle_sBIT` | `length != truelen` (3 for palette, else `png_ptr->channels`) (pngrutil.c:1161-1166) | `png_crc_finish(length)` + `png_chunk_benign_error(png_ptr, "bad length")` | [x] |
| 830 | `png_handle_sBIT` | CRC failure (pngrutil.c:1171-1172) | returns `handled_error` | [x] |
| 831 | `png_handle_sBIT` | any significant-bit byte zero or too large: `buf[i] == 0 \|\| buf[i] > sample_depth` (pngrutil.c:1174-1181) | `png_chunk_benign_error(png_ptr, "invalid")`, `handled_error` | [x] |
| 832 | `png_get_int_32_checked` | cHRM value `0x80000000` (two's-complement negation overflows) (pngrutil.c:1216-1223) | sets `*error = 1` and returns `0` | [x] |
| 833 | `png_handle_cHRM` | CRC failure (pngrutil.c:1237-1238) | returns `handled_error` | [x] |
| 834 | `png_handle_cHRM` | any of the 8 chromaticity values is the un-negatable `0x80000000`: `error != 0` (pngrutil.c:1249-1253) | `png_chunk_benign_error(png_ptr, "invalid")`, `handled_error` | [x] |
| 835 | `png_handle_sRGB` | CRC failure (pngrutil.c:1290-1291) | returns `handled_error` | [x] |
| 836 | `png_handle_sRGB` | rendering intent outside the PNGv3 range: `intent > 3` (pngrutil.c:1298-1302) | `png_chunk_benign_error(png_ptr, "invalid")`, `handled_error` | [x] |
| 837 | `png_handle_iCCP` | after reading up to 81 keyword bytes, too little data left for a zlib stream: `length < LZ77Min` (= 11) (pngrutil.c:1353-1358) | `png_crc_finish` + `png_chunk_benign_error(png_ptr, "too short")` | [x] |
| 838 | `png_handle_iCCP` | keyword empty or 80+ bytes: `!(keyword_length >= 1 && keyword_length <= 79)` (pngrutil.c:1366, 1532-1533) | `errmsg = "bad keyword"` → `png_chunk_benign_error` | [x] |
| 839 | `png_handle_iCCP` | compression-method byte missing or not 0: `keyword[keyword_length+1] != PNG_COMPRESSION_TYPE_BASE` (pngrutil.c:1371-1372, 1528-1529) | `errmsg = "bad compression method"` | [x] |
| 840 | `png_handle_iCCP` | `png_inflate_claim(png_iCCP)` fails (pngrutil.c:1376, 1524-1525) | `errmsg = png_ptr->zstream.msg` → benign error | [x] |
| 841 | `png_handle_iCCP` | compressed stream too short to yield the 132-byte ICC header: `size != 0` after the first `png_inflate_read` (pngrutil.c:1388, 1517-1518) | `errmsg = png_ptr->zstream.msg` ("profile truncated") | [x] |
| 842 | `png_handle_iCCP` | `png_icc_check_length` rejects `profile_length` (pngrutil.c:1394-1395, 1514) | chunk rejected; error message already emitted by the ICC checker | [x] |
| 843 | `png_handle_iCCP` | `png_icc_check_header` rejects the 132-byte header (pngrutil.c:1400-1401, 1511) | chunk rejected; error already emitted | [x] |
| 844 | `png_handle_iCCP` | `png_read_buffer(profile_length)` returns `NULL` (over limit / OOM) (pngrutil.c:1410-1413, 1507-1508) | `errmsg = "out of memory"` | [x] |
| 845 | `png_handle_iCCP` | tag table truncated: `size != 0` after inflating `12 * tag_count` bytes (pngrutil.c:1427, 1503-1504) | `errmsg = png_ptr->zstream.msg` | [x] |
| 846 | `png_handle_iCCP` | `png_icc_check_tag_table` rejects the tag table (pngrutil.c:1429-1430, 1501) | chunk rejected; error already emitted | [x] |
| 847 | `png_handle_iCCP` | uncompressed chunk data left over and benign errors are errors: `length > 0 && !(flags & PNG_FLAG_BENIGN_ERRORS_WARN)` (pngrutil.c:1443-1445) | `errmsg = "extra compressed data"` → `png_chunk_benign_error` | [x] |
| 848 | `png_handle_iCCP` | leftover data but benign errors warn: `length > 0` at pngrutil.c:1450-1456 | `png_chunk_warning(png_ptr, "extra compressed data")`, profile still accepted | [x] |
| 849 | `png_handle_iCCP` | profile body shorter than `profile_length`: `size != 0` after the final `png_inflate_read` (pngrutil.c:1448, 1498-1499) | `errmsg = png_ptr->zstream.msg` | [x] |
| 850 | `png_handle_iCCP` | `png_malloc_base` for `info_ptr->iccp_name` fails (pngrutil.c:1468-1484) | `errmsg = "out of memory"`, `handled_error` | [x] |
| 851 | `png_handle_sPLT` | chunk cache exhausted: `png_ptr->user_chunk_cache_max == 1` (pngrutil.c:1569-1573) | chunk skipped silently, returns `handled_error` | [x] |
| 852 | `png_handle_sPLT` | last cache slot consumed: `--png_ptr->user_chunk_cache_max == 1` (pngrutil.c:1575-1580) | `png_warning(png_ptr, "No space in chunk cache for sPLT")`, `handled_error` | [x] |
| 853 | `png_handle_sPLT` | `png_read_buffer(length+1)` returns `NULL` (pngrutil.c:1584-1590) | `png_chunk_benign_error(png_ptr, "out of memory")` | [x] |
| 854 | `png_handle_sPLT` | CRC failure (pngrutil.c:1599-1600) | returns `handled_error` | [x] |
| 855 | `png_handle_sPLT` | no sample depth after the name: `length < 2U \|\| entry_start > buffer + (length - 2U)` (pngrutil.c:1610-1614) | `png_warning(png_ptr, "malformed sPLT chunk")`, `handled_error` | [x] |
| 856 | `png_handle_sPLT` | entry data not a whole number of entries: `(data_length % entry_size) != 0` (entry_size 6 for depth 8, else 10) (pngrutil.c:1624-1628) | `png_warning(png_ptr, "sPLT chunk has bad length")`, `handled_error` | [x] |
| 857 | `png_handle_sPLT` | entry count overflows the allocation: `dl > PNG_SIZE_MAX / sizeof(png_sPLT_entry)` (pngrutil.c:1630-1637) | `png_warning(png_ptr, "sPLT chunk too long")`, `handled_error` | [x] |
| 858 | `png_handle_sPLT` | `png_malloc_warn` for the entries fails (pngrutil.c:1641-1648) | `png_warning(png_ptr, "sPLT chunk requires too much memory")`, `handled_error` | [x] |
| 859 | `png_handle_tRNS` | greyscale image and `length != 2` (pngrutil.c:1697-1702) | `png_crc_finish(length)` + `png_chunk_benign_error(png_ptr, "invalid")` | [x] |
| 860 | `png_handle_tRNS` | truecolour image and `length != 6` (pngrutil.c:1713-1718) | `png_chunk_benign_error(png_ptr, "invalid")` | [x] |
| 861 | `png_handle_tRNS` | palette image with no preceding PLTE: `(png_ptr->mode & PNG_HAVE_PLTE) == 0` (pngrutil.c:1729-1734) | `png_chunk_benign_error(png_ptr, "out of place")` | [x] |
| 862 | `png_handle_tRNS` | palette image with `length > num_palette`, `length > PNG_MAX_PALETTE_LENGTH`, or `length == 0` (pngrutil.c:1736-1743) | `png_chunk_benign_error(png_ptr, "invalid")` | [x] |
| 863 | `png_handle_tRNS` | colour type already has an alpha channel (GA / RGBA) (pngrutil.c:1749-1754) | `png_chunk_benign_error(png_ptr, "invalid with alpha channel")` | [x] |
| 864 | `png_handle_tRNS` | CRC failure (pngrutil.c:1756-1760) | `png_ptr->num_trans = 0`, returns `handled_error` | [x] |
| 865 | `png_handle_bKGD` | palette image with no preceding PLTE (pngrutil.c:1782-1787) | `png_chunk_benign_error(png_ptr, "out of place")` | [x] |
| 866 | `png_handle_bKGD` | `length != truelen` (1 palette / 6 colour / 2 grey) (pngrutil.c:1798-1803) | `png_chunk_benign_error(png_ptr, "invalid")` | [x] |
| 867 | `png_handle_bKGD` | CRC failure (pngrutil.c:1807-1808) | returns `handled_error` | [x] |
| 868 | `png_handle_bKGD` | palette index out of range: `buf[0] >= info_ptr->num_palette` (pngrutil.c:1819-1825) | `png_chunk_benign_error(png_ptr, "invalid index")` | [x] |
| 869 | `png_handle_bKGD` | greyscale, `bit_depth <= 8`, and `buf[0] != 0 \|\| buf[1] >= (1 << bit_depth)` (pngrutil.c:1840-1846) | `png_chunk_benign_error(png_ptr, "invalid gray level")` | [x] |
| 870 | `png_handle_bKGD` | colour, `bit_depth <= 8`, and any high byte non-zero: `buf[0] != 0 \|\| buf[2] != 0 \|\| buf[4] != 0` (pngrutil.c:1858-1864) | `png_chunk_benign_error(png_ptr, "invalid color")` | [x] |
| 871 | `png_handle_cICP` | CRC failure (pngrutil.c:1891-1892) | returns `handled_error` | [x] |
| 872 | `png_handle_cLLI` | CRC failure (pngrutil.c:1930-1931) | returns `handled_error` | [x] |
| 873 | `png_handle_cLLI` | out-of-range maxCLL/maxFALL — checking delegated to `png_set_cLLI_fixed` (pngrutil.c:1934-1935) | error/warning raised inside `png_set_cLLI_fixed` | [x] |
| 874 | `png_handle_mDCV` | CRC failure (pngrutil.c:1954-1955) | returns `handled_error` | [x] |
| 875 | `png_handle_mDCV` | out-of-range chromaticities/luminances — delegated to `png_set_mDCV_fixed` (pngrutil.c:1977-1983) | error/warning raised inside `png_set_mDCV_fixed` | [x] |
| 876 | `png_handle_eXIf` | `png_read_buffer(length)` returns `NULL` (pngrutil.c:2005-2012) | `png_chunk_benign_error(png_ptr, "out of memory")` | [x] |
| 877 | `png_handle_eXIf` | CRC failure (pngrutil.c:2016-2017) | returns `handled_error` | [x] |
| 878 | `png_handle_eXIf` | first 4 bytes are neither `0x49492A00` (II) nor `0x4D4D002A` (MM) (pngrutil.c:2024-2031) | `png_chunk_benign_error(png_ptr, "invalid")`, `handled_error` | [x] |
| 879 | `png_handle_hIST` | `length != num * 2`, `num != png_ptr->num_palette`, or `num > PNG_MAX_PALETTE_LENGTH` (pngrutil.c:2056-2065) | `png_crc_finish(length)` + `png_chunk_benign_error(png_ptr, "invalid")` | [x] |
| 880 | `png_handle_hIST` | CRC failure (pngrutil.c:2075-2076) | returns `handled_error` | [x] |
| 881 | `png_handle_pHYs` | CRC failure (pngrutil.c:2097-2098) | returns `handled_error` | [x] |
| 882 | `png_handle_oFFs` | CRC failure (pngrutil.c:2123-2124) | returns `handled_error` | [x] |
| 883 | `png_handle_pCAL` | `png_read_buffer(length+1)` returns `NULL` (pngrutil.c:2156-2163) | `png_chunk_benign_error(png_ptr, "out of memory")` | [x] |
| 884 | `png_handle_pCAL` | CRC failure (pngrutil.c:2167-2168) | returns `handled_error` | [x] |
| 885 | `png_handle_pCAL` | fewer than 13 bytes after the purpose string: `endptr - buf <= 12` (pngrutil.c:2181-2185) | `png_chunk_benign_error(png_ptr, "invalid")` | [x] |
| 886 | `png_handle_pCAL` | parameter count wrong for the equation type (`LINEAR!=2`, `BASE_E!=3`, `ARBITRARY!=3`, `HYPERBOLIC!=4`) (pngrutil.c:2198-2205) | `png_chunk_benign_error(png_ptr, "invalid parameter count")` | [x] |
| 887 | `png_handle_pCAL` | `type >= PNG_EQUATION_LAST` (pngrutil.c:2207-2210) | `png_chunk_benign_error(png_ptr, "unrecognized equation type")` (processing continues) | [x] |
| 888 | `png_handle_pCAL` | `png_malloc_warn` for the `nparams` pointer array fails (pngrutil.c:2217-2224) | `png_chunk_benign_error(png_ptr, "out of memory")` | [x] |
| 889 | `png_handle_pCAL` | a parameter string runs past the end of the chunk: `buf > endptr` (pngrutil.c:2233-2242) | `png_free(params)` + `png_chunk_benign_error(png_ptr, "invalid data")` | [x] |
| 890 | `png_handle_sCAL` | `png_read_buffer(length+1)` returns `NULL` (pngrutil.c:2274-2281) | `png_chunk_benign_error(png_ptr, "out of memory")` | [x] |
| 891 | `png_handle_sCAL` | CRC failure (pngrutil.c:2286-2287) | returns `handled_error` | [x] |
| 892 | `png_handle_sCAL` | unit byte neither 1 nor 2: `buffer[0] != 1 && buffer[0] != 2` (pngrutil.c:2290-2294) | `png_chunk_benign_error(png_ptr, "invalid unit")` | [x] |
| 893 | `png_handle_sCAL` | width is not a valid ASCII float or is not NUL-terminated inside the chunk: `png_check_fp_number(...) == 0 \|\| i >= length \|\| buffer[i++] != 0` (pngrutil.c:2302-2304) | `png_chunk_benign_error(png_ptr, "bad width format")`, `handled_error` | [x] |
| 894 | `png_handle_sCAL` | width parses but is zero/negative: `PNG_FP_IS_POSITIVE(state) == 0` (pngrutil.c:2306-2307) | `png_chunk_benign_error(png_ptr, "non-positive width")` | [x] |
| 895 | `png_handle_sCAL` | height is not a valid ASCII float or does not end exactly at the chunk end: `png_check_fp_number(...) == 0 \|\| i != length` (pngrutil.c:2314-2316) | `png_chunk_benign_error(png_ptr, "bad height format")` | [x] |
| 896 | `png_handle_sCAL` | height parses but is zero/negative (pngrutil.c:2318-2319) | `png_chunk_benign_error(png_ptr, "non-positive height")` | [x] |
| 897 | `png_handle_tIME` | CRC failure (pngrutil.c:2354-2355) | returns `handled_error` | [x] |
| 898 | `png_handle_tIME` | out-of-range month/day/hour/minute/second — checking delegated to `png_set_tIME` (pngrutil.c:2364) | warning/error raised inside `png_set_tIME` | [x] |
| 899 | `png_handle_tEXt` | chunk cache exhausted: `user_chunk_cache_max == 1` (pngrutil.c:2388-2392) | chunk skipped silently, `handled_error` | [x] |
| 900 | `png_handle_tEXt` | last cache slot consumed: `--user_chunk_cache_max == 1` (pngrutil.c:2394-2399) | `png_chunk_benign_error(png_ptr, "no space in chunk cache")` | [x] |
| 901 | `png_handle_tEXt` | `png_read_buffer(length+1)` returns `NULL` (pngrutil.c:2403-2410) | `png_chunk_benign_error(png_ptr, "out of memory")` | [x] |
| 902 | `png_handle_tEXt` | CRC failure (pngrutil.c:2414-2415) | returns `handled_error` | [x] |
| 903 | `png_handle_tEXt` | `png_set_text_2` fails (allocation failure storing the text) (pngrutil.c:2434-2438) | `png_chunk_benign_error(png_ptr, "out of memory")` | [x] |
| 904 | `png_handle_zTXt` | chunk cache exhausted: `user_chunk_cache_max == 1` (pngrutil.c:2458-2462) | chunk skipped silently, `handled_error` | [x] |
| 905 | `png_handle_zTXt` | last cache slot consumed (pngrutil.c:2464-2469) | `png_chunk_benign_error(png_ptr, "no space in chunk cache")` | [x] |
| 906 | `png_handle_zTXt` | `png_read_buffer(length)` returns `NULL` (pngrutil.c:2477-2484) | `png_chunk_benign_error(png_ptr, "out of memory")` | [x] |
| 907 | `png_handle_zTXt` | CRC failure (pngrutil.c:2488-2489) | returns `handled_error` | [x] |
| 908 | `png_handle_zTXt` | keyword empty or too long: `keyword_length > 79 \|\| keyword_length < 1` (pngrutil.c:2497-2498) | `errmsg = "bad keyword"` → `png_chunk_benign_error` | [x] |
| 909 | `png_handle_zTXt` | no room for separator + method + LZ data: `keyword_length + 3 > length` (pngrutil.c:2504-2505) | `errmsg = "truncated"` | [x] |
| 910 | `png_handle_zTXt` | `buffer[keyword_length+1] != PNG_COMPRESSION_TYPE_BASE` (pngrutil.c:2507-2508) | `errmsg = "unknown compression type"` | [x] |
| 911 | `png_handle_zTXt` | `png_decompress_chunk` does not return `Z_STREAM_END` (pngrutil.c:2518-2519, 2549-2550) | `errmsg = png_ptr->zstream.msg` → `png_chunk_benign_error` | [x] |
| 912 | `png_handle_zTXt` | `png_ptr->read_buffer == NULL` after a "successful" decompress (pngrutil.c:2523-2524) | `errmsg = "Read failure in png_handle_zTXt"` | [x] |
| 913 | `png_handle_zTXt` | `png_set_text_2` fails (pngrutil.c:2542-2545) | `errmsg = "out of memory"`, `handled_error` | [x] |
| 914 | `png_handle_iTXt` | chunk cache exhausted: `user_chunk_cache_max == 1` (pngrutil.c:2574-2578) | chunk skipped silently, `handled_error` | [x] |
| 915 | `png_handle_iTXt` | last cache slot consumed (pngrutil.c:2580-2585) | `png_chunk_benign_error(png_ptr, "no space in chunk cache")` | [x] |
| 916 | `png_handle_iTXt` | `png_read_buffer(length+1)` returns `NULL` (pngrutil.c:2589-2596) | `png_chunk_benign_error(png_ptr, "out of memory")` | [x] |
| 917 | `png_handle_iTXt` | CRC failure (pngrutil.c:2600-2601) | returns `handled_error` | [x] |
| 918 | `png_handle_iTXt` | keyword empty or too long: `prefix_length > 79 \|\| prefix_length < 1` (pngrutil.c:2610-2611) | `errmsg = "bad keyword"` | [x] |
| 919 | `png_handle_iTXt` | too short for keyword + flag + method + 2 NULs: `prefix_length + 5 > length` (pngrutil.c:2617-2618) | `errmsg = "truncated"` | [x] |
| 920 | `png_handle_iTXt` | compression flag not 0, or flag 1 with method byte != 0 (pngrutil.c:2620-2622, 2698-2699) | `errmsg = "bad compression info"` → `png_chunk_benign_error` | [x] |
| 921 | `png_handle_iTXt` | compressed iTXt whose prefix consumes the whole chunk: `compressed != 0 && prefix_length >= length` (pngrutil.c:2650-2670) | `errmsg = "truncated"` | [x] |
| 922 | `png_handle_iTXt` | `png_decompress_chunk` does not return `Z_STREAM_END` (pngrutil.c:2661-2666) | `errmsg = png_ptr->zstream.msg` | [x] |
| 923 | `png_handle_iTXt` | `png_set_text_2` fails (pngrutil.c:2691-2694) | `errmsg = "out of memory"`, `handled_error` | [x] |
| 924 | `png_cache_unknown_chunk` | unknown chunk over the memory limit (`length > png_chunk_max`) or `png_malloc_warn` fails (pngrutil.c:2722, 2741-2747) | `png_crc_finish` + `png_chunk_benign_error(png_ptr, "unknown chunk exceeds memory limits")`, returns `0` | [x] |
| 925 | `png_handle_unknown` | user read callback returns a negative value (pngrutil.c:2811-2812) | `png_chunk_error(png_ptr, "error in user chunk")` | [x] |
| 926 | `png_handle_unknown` | callback returns 0 and neither per-chunk nor default keep is `>= PNG_HANDLE_CHUNK_IF_SAFE` (pngrutil.c:2827-2839) | `png_chunk_warning(png_ptr, "Saving unknown chunk:")` + `png_app_warning(png_ptr, "forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks")` | [x] |
| 927 | `png_handle_unknown` | app asked to keep unknown chunks in a build without any save/store support: `keep > PNG_HANDLE_CHUNK_NEVER` (pngrutil.c:2892-2893) | `png_app_error(png_ptr, "no unknown chunk support available")` | [x] |
| 928 | `png_handle_unknown` | chunk cache limit reached while storing: `png_ptr->user_chunk_cache_max == 2` (pngrutil.c:2908-2912) | `png_chunk_benign_error(png_ptr, "no space in chunk cache")`, chunk not stored | [x] |
| 929 | `png_handle_unknown` | an unknown/disabled **critical** chunk was neither handled nor saved: `handled < handled_saved && PNG_CHUNK_CRITICAL(chunk_name)` (pngrutil.c:2956-2957) | `png_chunk_error(png_ptr, "unhandled critical chunk")` | [x] |
| 930 | `png_handle_chunk` | any known chunk other than IHDR arriving before IHDR: `(png_ptr->mode & PNG_HAVE_IHDR) == 0` (pngrutil.c:3133-3135) | `png_chunk_error(png_ptr, "missing IHDR")` — NORETURN | [x] |
| 931 | `png_handle_chunk` | chunk after a `pos_before` marker or before a required `pos_after` marker (table `read_chunks[]`, pngrutil.c:3138-3143) | `errmsg = "out of place"` | [x] |
| 932 | `png_handle_chunk` | second occurrence of a single-instance chunk: `multiple == 0 && png_file_has_chunk(...)` (pngrutil.c:3148-3152) | `errmsg = "duplicate"` | [x] |
| 933 | `png_handle_chunk` | `length < read_chunks[chunk_index].min_length` (e.g. cHRM<32, gAMA<4, eXIf<4, iCCP<14, iTXt<6, tEXt<2, pCAL<14) (pngrutil.c:3154-3155) | `errmsg = "too short"` | [x] |
| 934 | `png_handle_chunk` | `Limit` chunk (eXIf/zTXt/sCAL/fdAT) longer than the memory limit: `length > png_chunk_max(png_ptr)` (pngrutil.c:3169-3178) | `errmsg = "length exceeds libpng limit"` | [x] |
| 935 | `png_handle_chunk` | fixed-max chunk longer than its spec maximum: `length > max_length` (e.g. IHDR>13, tRNS>256, hIST>1024, pHYs>9, tIME>7) (pngrutil.c:3180-3185) | `errmsg = "too long"` | [x] |
| 936 | `png_handle_chunk` | any of the above `errmsg` cases on a **critical** chunk (pngrutil.c:3198-3201) | `png_chunk_error(png_ptr, errmsg)` — read aborted | [x] |
| 937 | `png_handle_chunk` | any of the above `errmsg` cases on an ancillary chunk (pngrutil.c:3202-3207) | `png_crc_finish(length)` (data skipped) + `png_chunk_benign_error(png_ptr, errmsg)` | [x] |
| 938 | `png_combine_row` | called before any row was transformed: `pixel_depth == 0` (pngrutil.c:3242-3243) | `png_error(png_ptr, "internal row logic error")` | [x] |
| 939 | `png_combine_row` | app row size disagrees with libpng: `info_rowbytes != PNG_ROWBYTES(pixel_depth, row_width)` (pngrutil.c:3249-3251) | `png_error(png_ptr, "internal row size calculation error")` | [x] |
| 940 | `png_combine_row` | `row_width == 0` (pngrutil.c:3254-3255) | `png_error(png_ptr, "internal row width error")` | [x] |
| 941 | `png_combine_row` | interlace pass has no pixels in this row: `row_width <= PNG_PASS_START_COL(pass)` (pngrutil.c:3294-3295) | returns without copying | [x] |
| 942 | `png_combine_row` | user transform produced a depth >= 8 that is not a whole number of bytes: `pixel_depth & 7` (pngrutil.c:3477-3478) | `png_error(png_ptr, "invalid user transform pixel depth")` | [x] |
| 943 | `png_do_read_interlace` | `row == NULL \|\| row_info == NULL` (pngrutil.c:3715) | no-op, row left unchanged | [x] |
| 944 | `png_read_filter_row` | filter byte 0 (NONE) or out of range: `!(filter > PNG_FILTER_VALUE_NONE && filter < PNG_FILTER_VALUE_LAST)` (pngrutil.c:4161-4167) | no un-filtering performed (invalid filter byte silently ignored) | [x] |
| 945 | `png_read_IDAT_data` | the chunk following an exhausted IDAT is not another IDAT: `png_ptr->chunk_name != png_IDAT` (pngrutil.c:4192-4201) | `png_error(png_ptr, "Not enough image data")` | [x] |
| 946 | `png_read_IDAT_data` | `png_read_buffer(avail_in)` returns `NULL` (pngrutil.c:4219-4222) | `png_chunk_error(png_ptr, "out of memory")` | [x] |
| 947 | `png_read_IDAT_data` | LZ stream ended but IDAT bytes remain: `zstream.avail_in > 0 \|\| png_ptr->idat_size > 0` (pngrutil.c:4275-4276) | `png_chunk_benign_error(png_ptr, "Extra compressed data")` | [x] |
| 948 | `png_read_IDAT_data` | `inflate` returns an error while producing image rows (`output != NULL`) (pngrutil.c:4280-4285) | `png_zstream_error` + `png_chunk_error(png_ptr, png_ptr->zstream.msg)` | [x] |
| 949 | `png_read_IDAT_data` | `inflate` returns an error during the end-of-stream check (`output == NULL`) (pngrutil.c:4287-4291) | `png_chunk_benign_error(png_ptr, png_ptr->zstream.msg)` and return | [x] |
| 950 | `png_read_IDAT_data` | stream ended before the requested row data was produced: `avail_out > 0` with `output != NULL` (pngrutil.c:4295-4301) | `png_error(png_ptr, "Not enough image data")` | [x] |
| 951 | `png_read_IDAT_data` | extra decompressed data past the end of the image: `avail_out > 0` with `output == NULL` (pngrutil.c:4303-4304) | `png_chunk_benign_error(png_ptr, "Too much image data")` | [x] |
| 952 | `png_read_start_row` | (`PNG_MAX_MALLOC_64K` builds) computed buffer `row_bytes > 65536L` (pngrutil.c:4599-4600) | `png_error(png_ptr, "This image requires a row greater than 64KB")` | [x] |
| 953 | `png_read_start_row` | (`PNG_MAX_MALLOC_64K` builds) `png_ptr->rowbytes > 65535` (pngrutil.c:4644-4645) | `png_error(png_ptr, "This image requires a row greater than 64KB")` | [x] |
| 954 | `png_read_start_row` | `png_ptr->rowbytes > (PNG_SIZE_MAX - 1)` (pngrutil.c:4648-4649) | `png_error(png_ptr, "Row has too many bytes to allocate in memory")` | [x] |
| 955 | `png_read_start_row` | `png_inflate_claim(png_ptr, png_IDAT) != Z_OK` (bad deflate header / OOM) (pngrutil.c:4679-4680) | `png_error(png_ptr, png_ptr->zstream.msg)` | [x] |
| 956 | `icc_check_length` | ICC profile shorter than the fixed header+tag-count: `profile_length < 132` (png.c:1588-1589, reached from `png_handle_iCCP`) | `png_icc_profile_error` → `png_chunk_benign_error(... "profile '<name>': too short")`, returns 0 | [x] |
| 957 | `png_icc_check_length` | `profile_length > png_chunk_max(png_ptr)` (png.c:1606-1608) | `png_icc_profile_error(... "profile too long")`, returns 0 | [x] |
| 958 | `png_icc_check_header` | profile's own length word differs from the chunk-derived length: `png_get_uint_32(profile) != profile_length` (png.c:1625-1628) | `png_icc_profile_error(... "length does not match profile")`, returns 0 | [x] |
| 959 | `png_icc_check_header` | major version > 3 and length not a multiple of 4: `temp > 3 && (profile_length & 3)` (png.c:1630-1633) | `png_icc_profile_error(... "invalid length")`, returns 0 | [x] |
| 960 | `png_icc_check_header` | `tag_count > 357913930` or `profile_length < 132 + 12*tag_count` (truncated tag table) (png.c:1635-1639) | `png_icc_profile_error(... "tag count too large")`, returns 0 | [x] |
| 961 | `png_icc_check_header` | rendering intent field `>= 0xffff` (png.c:1644-1647) | `png_icc_profile_error(... "invalid rendering intent")`, returns 0 | [x] |
| 962 | `png_icc_check_header` | rendering intent `>= PNG_sRGB_INTENT_LAST` but `< 0xffff` (png.c:1652-1654) | warning only: `png_icc_profile_error(... "intent outside defined range")`, profile still accepted | [x] |
| 963 | `png_icc_check_header` | ICC file signature at offset 36 not `'acsp'` (`0x61637370`) (png.c:1668-1671) | `png_icc_profile_error(... "invalid signature")`, returns 0 | [x] |
| 964 | `png_icc_check_header` | PCS illuminant at offset 68 is not the D50 nCIEXYZ value (png.c:1680-1682) | warning only: `png_icc_profile_error(... "PCS illuminant is not D50")` | [x] |
| 965 | `png_icc_check_header` | data colour space `'RGB '` on a greyscale PNG: `(color_type & PNG_COLOR_MASK_COLOR) == 0` (png.c:1707-1710) | `png_icc_profile_error(... "RGB color space not permitted on grayscale PNG")`, returns 0 | [x] |
| 966 | `png_icc_check_header` | data colour space `'GRAY'` on a colour PNG (png.c:1713-1716) | `png_icc_profile_error(... "Gray color space not permitted on RGB PNG")`, returns 0 | [x] |
| 967 | `png_icc_check_header` | data colour space neither `'RGB '` nor `'GRAY'` (png.c:1719-1721) | `png_icc_profile_error(... "invalid ICC profile color space")`, returns 0 | [x] |
| 968 | `png_icc_check_header` | profile class `'abst'` (abstract) embedded in a PNG (png.c:1743-1746) | `png_icc_profile_error(... "invalid embedded Abstract ICC profile")`, returns 0 | [x] |
| 969 | `png_icc_check_header` | profile class `'link'` (DeviceLink) (png.c:1748-1756) | `png_icc_profile_error(... "unexpected DeviceLink ICC profile class")`, returns 0 | [x] |
| 970 | `png_icc_check_header` | profile class `'nmcl'` (NamedColor) (png.c:1758-1765) | warning only: `png_icc_profile_error(... "unexpected NamedColor ICC profile class")` | [x] |
| 971 | `png_icc_check_header` | profile class not one of scnr/mntr/prtr/spac/abst/link/nmcl (png.c:1767-1775) | warning only: `png_icc_profile_error(... "unrecognized ICC profile class")` | [x] |
| 972 | `png_icc_check_header` | PCS encoding at offset 20 neither `'XYZ '` nor `'Lab '` (png.c:1781-1791) | `png_icc_profile_error(... "unexpected ICC PCS encoding")`, returns 0 | [x] |
| 973 | `png_icc_check_tag_table` | a tag lies outside the profile: `tag_start > profile_length \|\| tag_length > profile_length - tag_start` (png.c:1824-1826) | `png_icc_profile_error(... "ICC profile tag outside profile")`, returns 0 | [x] |
| 974 | `png_icc_check_tag_table` | tag offset not 4-byte aligned: `(tag_start & 3) != 0` (png.c:1828-1836) | warning only: `png_icc_profile_error(... "ICC profile tag start not a multiple of 4")` | [x] |
| 975 | `write_unknown_chunks` | an app-supplied unknown chunk with `up->size == 0` (pngwrite.c:63-64) | `png_warning(png_ptr, "Writing zero-length unknown chunk")`, chunk still written | [x] |
| 976 | `png_write_info_before_PLTE` | `png_ptr == NULL \|\| info_ptr == NULL` (pngwrite.c:87-88) | returns, nothing written | [x] |
| 977 | `png_write_info_before_PLTE` | MNG features enabled while writing a real PNG stream: `(mode & PNG_HAVE_PNG_SIGNATURE) != 0 && mng_features_permitted != 0` (pngwrite.c:96-102) | `png_warning(png_ptr, "MNG features are not allowed in a PNG datastream")`, features cleared | [x] |
| 978 | `png_write_info` | `png_ptr == NULL \|\| info_ptr == NULL` (pngwrite.c:231-232) | returns, nothing written | [x] |
| 979 | `png_write_info` | `color_type == PNG_COLOR_TYPE_PALETTE` but `(info_ptr->valid & PNG_INFO_PLTE) == 0` (pngwrite.c:236-241) | `png_error(png_ptr, "Valid palette required for paletted images")` | [x] |
| 980 | `png_write_info` | iTXt text supplied in a build without `PNG_WRITE_iTXt_SUPPORTED` (pngwrite.c:330-347) | `png_warning(png_ptr, "Unable to write international text")`, chunk dropped | n/a |
| 981 | `png_write_info` | zTXt text supplied in a build without `PNG_WRITE_zTXt_SUPPORTED` (pngwrite.c:351-361) | `png_warning(png_ptr, "Unable to write compressed text")`, chunk dropped | n/a |
| 982 | `png_write_info` | tEXt text supplied in a build without `PNG_WRITE_tEXt_SUPPORTED` (pngwrite.c:364-376) | `png_warning(png_ptr, "Unable to write uncompressed text")`, chunk dropped | n/a |
| 983 | `png_write_end` | `png_ptr == NULL` (pngwrite.c:396-397) | returns, nothing written | [x] |
| 984 | `png_write_end` | no image data written: `(png_ptr->mode & PNG_HAVE_IDAT) == 0` (pngwrite.c:399-400) | `png_error(png_ptr, "No IDATs written into file")` | [x] |
| 985 | `png_write_end` | palette image where a written index exceeded the palette: `num_palette_max >= png_ptr->num_palette` (pngwrite.c:403-405) | `png_benign_error(png_ptr, "Wrote palette index exceeding num_palette")` | [x] |
| 986 | `png_write_end` | trailer iTXt/zTXt/tEXt text in a build where that chunk type is not compiled in (pngwrite.c:444, 457, 470) | `png_warning(png_ptr, "Unable to write international/compressed/uncompressed text")` | n/a |
| 987 | `png_convert_from_time_t` | `gmtime(&ttime)` returns `NULL` (unrepresentable `time_t`) (pngwrite.c:527-536) | `memset(ptime, 0, ...)` and return — silently produces an all-zero time | [x] |
| 988 | `png_write_rows` | `png_ptr == NULL` (pngwrite.c:635-636) | returns, no rows written | [x] |
| 989 | `png_write_image` | `png_ptr == NULL` (pngwrite.c:655-656) | returns, no rows written | [x] |
| 990 | `png_do_write_intrapixel` | MNG filter 64 requested for a colour type that is not RGB/RGBA at 8 or 16 bits (pngwrite.c:695-702, 717-724) | returns without transforming the row | [x] |
| 991 | `png_write_row` | `png_ptr == NULL` (pngwrite.c:754-755) | returns | [x] |
| 992 | `png_write_row` | first row written without a preceding `png_write_info`: `(png_ptr->mode & PNG_WROTE_INFO_BEFORE_PLTE) == 0` (pngwrite.c:761-763) | `png_error(png_ptr, "png_write_info was never called before png_write_row")` | [x] |
| 993 | `png_write_row` | a read-side-only transform was set in a build where the write side is compiled out (`PNG_INVERT_MONO`, `PNG_FILLER`, `PNG_PACKSWAP`, `PNG_PACK`, `PNG_SHIFT`, `PNG_BGR`, `PNG_SWAP_BYTES`) (pngwrite.c:766-800) | `png_warning(png_ptr, "PNG_WRITE_<X>_SUPPORTED is not defined")` — one warning per transform, transform ignored | n/a |
| 994 | `png_write_row` | interlaced write and the current `row_number` is not in the current pass (per-pass tests for passes 0-6, including `width < 5`, `< 3`, `< 2`) (pngwrite.c:810-871) | `png_write_finish_row(png_ptr)` and return — row silently discarded | [x] |
| 995 | `png_write_row` | interlacing left the row empty: `row_info.width == 0` after `png_do_write_interlace` (pngwrite.c:899-903) | `png_write_finish_row(png_ptr)` and return | [x] |
| 996 | `png_write_row` | transformed depth disagrees with the header: `row_info.pixel_depth != png_ptr->pixel_depth \|\| row_info.pixel_depth != png_ptr->transformed_pixel_depth` (pngwrite.c:916-918) | `png_error(png_ptr, "internal write transform logic error")` | [x] |
| 997 | `png_set_flush` | `png_ptr == NULL` (pngwrite.c:960-961); `nrows < 0` (pngwrite.c:963) | returns / negative interval clamped to `0` (flushing off) | [x] |
| 998 | `png_write_flush` | `png_ptr == NULL` (pngwrite.c:972-973), or all rows already written: `row_number >= num_rows` (pngwrite.c:976-977) | returns without flushing | [x] |
| 999 | `png_destroy_write_struct` | `png_ptr_ptr == NULL` or `*png_ptr_ptr == NULL` (pngwrite.c:1041-1045) | silently does nothing | [x] |
| 1000 | `png_set_filter` | `png_ptr == NULL` (pngwrite.c:1062-1063) | returns | [x] |
| 1001 | `png_set_filter` | filter value 5, 6 or 7 for method 0: `filters & (PNG_ALL_FILTERS \| 0x07)` in {5,6,7} (pngwrite.c:1073-1078) | `png_app_error(png_ptr, "Unknown row filter for method 0")`, falls through to `PNG_FILTER_NONE` | [x] |
| 1002 | `png_set_filter` | any non-`NONE` filter in a build without `PNG_WRITE_FILTER_SUPPORTED` (pngwrite.c:1099-1101) | `png_app_error(png_ptr, "Unknown row filter for method 0")` | n/a |
| 1003 | `png_set_filter` | UP/AVG/PAETH requested after writing started with no `prev_row`: `(filters & (UP\|AVG\|PAETH)) != 0 && png_ptr->prev_row == NULL` (pngwrite.c:1134-1143) | `png_app_warning(png_ptr, "png_set_filter: UP/AVG/PAETH cannot be added after start")`, those filters removed | [x] |
| 1004 | `png_set_filter` | `method != PNG_FILTER_TYPE_BASE` (pngwrite.c:1179-1180) | `png_error(png_ptr, "Unknown custom filter method")` | [x] |
| 1005 | `png_set_compression_level` | `png_ptr == NULL` (pngwrite.c:1220-1221) | returns | [x] |
| 1006 | `png_set_compression_mem_level` | `png_ptr == NULL` (pngwrite.c:1231-1232) | returns | [x] |
| 1007 | `png_set_compression_strategy` | `png_ptr == NULL` (pngwrite.c:1242-1243) | returns | [x] |
| 1008 | `png_set_compression_window_bits` | `png_ptr == NULL` (pngwrite.c:1259-1260) | returns | [x] |
| 1009 | `png_set_compression_window_bits` | `window_bits > 15` (pngwrite.c:1268-1272) | `png_warning(png_ptr, "Only compression windows <= 32k supported by PNG")`, clamped to 15 | [x] |
| 1010 | `png_set_compression_window_bits` | `window_bits < 8` (incl. negative raw-deflate values) (pngwrite.c:1274-1278) | `png_warning(png_ptr, "Only compression windows >= 256 supported by PNG")`, clamped to 8 | [x] |
| 1011 | `png_set_compression_method` | `png_ptr == NULL` (pngwrite.c:1288-1289) | returns | [x] |
| 1012 | `png_set_compression_method` | `method != 8` (pngwrite.c:1294-1295) | `png_warning(png_ptr, "Only compression method 8 is supported by PNG")` (value still stored; deflate will fail) | [x] |
| 1013 | `png_set_text_compression_level` | `png_ptr == NULL` (pngwrite.c:1308-1309) | returns | [x] |
| 1014 | `png_set_text_compression_mem_level` | `png_ptr == NULL` (pngwrite.c:1319-1320) | returns | [x] |
| 1015 | `png_set_text_compression_strategy` | `png_ptr == NULL` (pngwrite.c:1330-1331) | returns | [x] |
| 1016 | `png_set_text_compression_window_bits` | `png_ptr == NULL` (pngwrite.c:1344-1345) | returns | [x] |
| 1017 | `png_set_text_compression_window_bits` | `window_bits > 15` (pngwrite.c:1347-1351) | `png_warning(png_ptr, "Only compression windows <= 32k supported by PNG")`, clamped to 15 | [x] |
| 1018 | `png_set_text_compression_window_bits` | `window_bits < 8` (pngwrite.c:1353-1357) | `png_warning(png_ptr, "Only compression windows >= 256 supported by PNG")`, clamped to 8 | [x] |
| 1019 | `png_set_text_compression_method` | `png_ptr == NULL` (pngwrite.c:1367-1368) | returns | [x] |
| 1020 | `png_set_text_compression_method` | `method != 8` (pngwrite.c:1370-1371) | `png_warning(png_ptr, "Only compression method 8 is supported by PNG")` | [x] |
| 1021 | `png_set_write_status_fn` | `png_ptr == NULL` (pngwrite.c:1383-1384) | returns | [x] |
| 1022 | `png_set_write_user_transform_fn` | `png_ptr == NULL` (pngwrite.c:1396-1397) | returns | [x] |
| 1023 | `png_write_png` | `png_ptr == NULL \|\| info_ptr == NULL` (pngwrite.c:1412-1413) | returns | [x] |
| 1024 | `png_write_png` | no rows attached: `(info_ptr->valid & PNG_INFO_IDAT) == 0` (pngwrite.c:1415-1419) | `png_app_error(png_ptr, "no rows for png_write_image to write")` and return | [x] |
| 1025 | `png_write_png` | a `PNG_TRANSFORM_*` bit set in a build where that write transform is compiled out (INVERT_MONO, SHIFT, PACKING, SWAP_ALPHA, STRIP_FILLER, BGR, SWAP_ENDIAN, PACKSWAP, INVERT_ALPHA) (pngwrite.c:1427-1516) | `png_app_error(png_ptr, "PNG_TRANSFORM_<X> not supported")` — one per transform | n/a |
| 1026 | `png_write_png` | both `PNG_TRANSFORM_STRIP_FILLER_AFTER` and `..._BEFORE` requested (pngwrite.c:1469-1473) | `png_app_error(png_ptr, "PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported")`, AFTER used if ignored | [x] |
| 1027 | `png_image_write_init` | `png_create_write_struct`, `png_create_info_struct` or the `png_control` malloc fails (pngwrite.c:1536-1567) | cleans up and `return png_image_error(image, "png_image_write_: out of memory")` (returns 0) | [x] |
| 1028 | `png_write_image_16bit` | called for a format without an alpha channel: `(image->format & PNG_FORMAT_FLAG_ALPHA) == 0` (pngwrite.c:1612-1629) | `png_error(png_ptr, "png_write_image: internal call error")` | [x] |
| 1029 | `png_image_write_main` | `image->width > 0x7fffffffU/channels` (row stride computation would overflow) (pngwrite.c:2024, 2052-2053) | `png_error(image->opaque->png_ptr, "image row stride too large")` | [x] |
| 1030 | `png_image_write_main` | supplied `row_stride` smaller in magnitude than one row: `check < png_row_stride` (pngwrite.c:2032-2049) | `png_error(image->opaque->png_ptr, "supplied row stride too small")` | [x] |
| 1031 | `png_image_write_main` | total buffer would exceed 32 bits: `image->height > 0xffffffffU/png_row_stride` (pngwrite.c:2044-2045) | `png_error(image->opaque->png_ptr, "memory image too large")` | [x] |
| 1032 | `png_image_write_main` | colour-mapped format but `display->colormap == NULL` or `image->colormap_entries == 0` (pngwrite.c:2057-2073) | `png_error(image->opaque->png_ptr, "no color-map for color-mapped image")` | [x] |
| 1033 | `png_image_write_main` | `image->format` contains flags other than COLOR/LINEAR/ALPHA/COLORMAP after the handled transforms are removed (pngwrite.c:2154-2156) | `png_error(png_ptr, "png_write_image: unsupported transformation")` | [x] |
| 1034 | `png_image_write_main` | `png_safe_execute(png_write_image_16bit/8bit)` returns 0 (error inside row conversion) (pngwrite.c:2198-2207) | returns 0 without calling `png_write_end` | [x] |
| 1035 | `png_image_set_PLTE` | `image->colormap_entries > 256` (pngwrite.c:1856-1857) | silently truncated to 256 entries and `image->colormap_entries` rewritten | [x] |
| 1036 | `image_memory_write` | output byte count would overflow: `size > ((png_alloc_size_t)-1) - ob` (pngwrite.c:2239, 2252-2253) | `png_error(png_ptr, "png_image_write_to_memory: PNG too big")` | [x] |
| 1037 | `image_memory_write` | supplied buffer too small: `display->memory_bytes < ob+size` (pngwrite.c:2244-2248) | data not copied; only `output_bytes` accumulated (caller detects overflow) | [x] |
| 1038 | `png_image_write_to_memory` | `memory_bytes == NULL \|\| buffer == NULL` (pngwrite.c:2286, 2331-2333) | `png_image_error(image, "png_image_write_to_memory: invalid argument")`, returns 0 | [x] |
| 1039 | `png_image_write_to_memory` | `image->version != PNG_IMAGE_VERSION` (pngwrite.c:2284, 2336-2338) | `png_image_error(image, "png_image_write_to_memory: incorrect PNG_IMAGE_VERSION")` | [x] |
| 1040 | `png_image_write_to_memory` | `image == NULL` (pngwrite.c:2340-2341) | returns 0 (no error message possible) | [x] |
| 1041 | `png_image_write_to_memory` | `png_image_write_init` failed (pngwrite.c:2327-2328) | returns 0 | [x] |
| 1042 | `png_image_write_to_memory` | encoded PNG bigger than the supplied buffer: `memory != NULL && display.output_bytes > *memory_bytes` (pngwrite.c:2318-2321) | returns 0 but `*memory_bytes` set to the required size | [x] |
| 1043 | `png_image_write_to_stdio` | `file == NULL \|\| buffer == NULL` (pngwrite.c:2352, 2381-2383) | `png_image_error(image, "png_image_write_to_stdio: invalid argument")`, returns 0 | [x] |
| 1044 | `png_image_write_to_stdio` | `image->version != PNG_IMAGE_VERSION` (pngwrite.c:2350, 2386-2388) | `png_image_error(image, "png_image_write_to_stdio: incorrect PNG_IMAGE_VERSION")` | [x] |
| 1045 | `png_image_write_to_stdio` | `image == NULL` (pngwrite.c:2390-2391) | returns 0 | [x] |
| 1046 | `png_image_write_to_stdio` | `png_image_write_init` failed (pngwrite.c:2377-2378) | returns 0 | [x] |
| 1047 | `png_image_write_to_file` | `file_name == NULL \|\| buffer == NULL` (pngwrite.c:2402, 2448-2450) | `png_image_error(image, "png_image_write_to_file: invalid argument")`, returns 0 | [x] |
| 1048 | `png_image_write_to_file` | `fopen(file_name, "wb")` returns `NULL` (pngwrite.c:2404, 2444-2445) | `png_image_error(image, strerror(errno))`, returns 0 | [x] |
| 1049 | `png_image_write_to_file` | `fflush`/`ferror`/`fclose` failure after a successful write (pngwrite.c:2414-2432) | file removed with `remove(file_name)`, `png_image_error(image, strerror(error))`, returns 0 | [x] |
| 1050 | `png_image_write_to_file` | `png_image_write_to_stdio` returned 0 (write error) (pngwrite.c:2435-2441) | `fclose` + `remove(file_name)`, returns 0 | [x] |
| 1051 | `png_image_write_to_file` | `image->version != PNG_IMAGE_VERSION` (pngwrite.c:2400, 2453-2455) | `png_image_error(image, "png_image_write_to_file: incorrect PNG_IMAGE_VERSION")` | [x] |
| 1052 | `png_image_write_to_file` | `image == NULL` (pngwrite.c:2457-2458) | returns 0 | [x] |
| 1053 | `png_write_chunk_header` | `png_ptr == NULL` (pngwutil.c:100-101) | returns, nothing written | [x] |
| 1054 | `png_write_chunk_data` | `png_ptr == NULL` (pngwutil.c:147-148) | returns | [x] |
| 1055 | `png_write_chunk_data` | `data == NULL \|\| length == 0` (pngwutil.c:150) | nothing written and CRC not updated (chunk length may then be wrong) | [x] |
| 1056 | `png_write_chunk_end` | `png_ptr == NULL` (pngwutil.c:167) | returns | [x] |
| 1057 | `png_write_complete_chunk` | `png_ptr == NULL` (pngwutil.c:195-196) | returns | [x] |
| 1058 | `png_write_complete_chunk` | chunk data longer than the PNG limit: `length > PNG_UINT_31_MAX` (pngwutil.c:199-200) | `png_error(png_ptr, "length exceeds PNG maximum")` | [x] |
| 1059 | `png_image_size` | `png_ptr->rowbytes >= 32768 \|\| height >= 32768` (pngwutil.c:228, 255-256) | returns `0xffffffffU` — forces the maximum deflate window instead of an exact size | [x] |
| 1060 | `png_deflate_claim` | zstream already owned: `png_ptr->zowner != 0`, release build (pngwutil.c:312-328, 337) | `png_warning(png_ptr, "<cHNK>: <owner> using zstream")`, ownership stolen | [x] |
| 1061 | `png_deflate_claim` | zstream owned by IDAT (release build): `png_ptr->zowner == png_IDAT` (pngwutil.c:331-335) | `zstream.msg = "in use by IDAT"`, returns `Z_STREAM_ERROR` | [x] |
| 1062 | `png_deflate_claim` | `png_ptr->zowner != 0`, non-release build (pngwutil.c:338-340) | `png_error(png_ptr, msg)` | [x] |
| 1063 | `png_deflate_claim` | `deflateEnd` fails when re-initializing with changed parameters (pngwutil.c:412-413) | `png_warning(png_ptr, "deflateEnd failed (ignored)")` | [x] |
| 1064 | `png_deflate_claim` | `deflateInit2`/`deflateReset` returns other than `Z_OK` (bad level/method/windowBits/memLevel/strategy or OOM) (pngwutil.c:429-450) | `png_zstream_error`, returns the zlib code — callers `png_error` with `zstream.msg` | [x] |
| 1065 | `png_text_compress` | `png_deflate_claim` fails (pngwutil.c:520-523) | returns the zlib error code to the chunk writer, which calls `png_error` | [x] |
| 1066 | `png_text_compress` | compressed output would exceed the chunk limit mid-stream: `output_len + prefix_len > PNG_UINT_31_MAX` (pngwutil.c:562-566) | `ret = Z_MEM_ERROR`, loop aborted | [x] |
| 1067 | `png_text_compress` | `png_malloc_base` for an extra compression buffer fails (pngwutil.c:574-581) | `ret = Z_MEM_ERROR` | [x] |
| 1068 | `png_text_compress` | final size check `output_len + prefix_len >= PNG_UINT_31_MAX` (pngwutil.c:619-623) | `zstream.msg = "compressed data too long"`, `ret = Z_MEM_ERROR` | [x] |
| 1069 | `png_text_compress` | `deflate` did not reach `Z_STREAM_END`, or input left unconsumed: `!(ret == Z_STREAM_END && input_len == 0)` (pngwutil.c:634, 647-648) | returns the zlib error code (caller `png_error`s with `zstream.msg`) | [x] |
| 1070 | `png_write_compressed_data_out` | buffer list exhausted before all compressed bytes were written: `output_len > 0` (pngwutil.c:679-680) | `png_error(png_ptr, "error writing ancillary chunked compressed data")` | [x] |
| 1071 | `png_write_IHDR` | greyscale with `bit_depth` not in {1,2,4,8,16} (pngwutil.c:701-716) | `png_error(png_ptr, "Invalid bit depth for grayscale image")` | [x] |
| 1072 | `png_write_IHDR` | RGB with `bit_depth` not 8 (or 16 when `PNG_WRITE_16BIT_SUPPORTED`) (pngwutil.c:719-725) | `png_error(png_ptr, "Invalid bit depth for RGB image")` | [x] |
| 1073 | `png_write_IHDR` | palette with `bit_depth` not in {1,2,4,8} (pngwutil.c:730-742) | `png_error(png_ptr, "Invalid bit depth for paletted image")` | [x] |
| 1074 | `png_write_IHDR` | grey+alpha with `bit_depth` not 8/16 (pngwutil.c:745-751) | `png_error(png_ptr, "Invalid bit depth for grayscale+alpha image")` | [x] |
| 1075 | `png_write_IHDR` | RGBA with `bit_depth` not 8/16 (pngwutil.c:756-762) | `png_error(png_ptr, "Invalid bit depth for RGBA image")` | [x] |
| 1076 | `png_write_IHDR` | `color_type` not 0/2/3/4/6 (pngwutil.c:767-768) | `png_error(png_ptr, "Invalid image color type specified")` | [x] |
| 1077 | `png_write_IHDR` | `compression_type != PNG_COMPRESSION_TYPE_BASE` (pngwutil.c:771-775) | `png_warning(png_ptr, "Invalid compression type specified")`, forced to 0 | [x] |
| 1078 | `png_write_IHDR` | `filter_type != PNG_FILTER_TYPE_BASE` and not a permitted MNG intrapixel case (pngwutil.c:786-798) | `png_warning(png_ptr, "Invalid filter type specified")`, forced to 0 | [x] |
| 1079 | `png_write_IHDR` | `interlace_type` neither `PNG_INTERLACE_NONE` nor `PNG_INTERLACE_ADAM7` (pngwutil.c:801-806) | `png_warning(png_ptr, "Invalid interlace type specified")`, forced to ADAM7 | [x] |
| 1080 | `png_write_PLTE` | palette image with `num_pal == 0` (no MNG empty-PLTE permission) or `num_pal > (1 << bit_depth)` (pngwutil.c:871-880) | `png_error(png_ptr, "Invalid number of colors in palette")` | [x] |
| 1081 | `png_write_PLTE` | same condition for a non-palette (truecolour) image, `num_pal > PNG_MAX_PALETTE_LENGTH` or 0 (pngwutil.c:882-886) | `png_warning(png_ptr, "Invalid number of colors in palette")` and return (chunk dropped) | [x] |
| 1082 | `png_write_PLTE` | PLTE requested for a greyscale image: `(color_type & PNG_COLOR_MASK_COLOR) == 0` (pngwutil.c:889-895) | `png_warning(png_ptr, "Ignoring request to write a PLTE chunk in grayscale PNG")`, chunk dropped | [x] |
| 1083 | `png_compress_IDAT` | `png_deflate_claim(png_IDAT, ...)` fails (pngwutil.c:953-954) | `png_error(png_ptr, png_ptr->zstream.msg)` | [x] |
| 1084 | `png_compress_IDAT` | `deflate` returns `Z_OK` with `input_len == 0` while `flush == Z_FINISH` (pngwutil.c:1030-1033) | `png_error(png_ptr, "Z_OK on Z_FINISH with output space")` | [x] |
| 1085 | `png_compress_IDAT` | `deflate` returns anything other than `Z_OK`/`Z_STREAM_END`-on-FINISH (pngwutil.c:1063-1068) | `png_zstream_error` then `png_error(png_ptr, png_ptr->zstream.msg)` | [x] |
| 1086 | `png_write_sRGB` | `srgb_intent >= PNG_sRGB_INTENT_LAST` (pngwutil.c:1106-1108) | `png_warning(png_ptr, "Invalid sRGB rendering intent specified")`, chunk still written | [x] |
| 1087 | `png_write_iCCP` | `profile == NULL` (pngwutil.c:1131-1132) | `png_error(png_ptr, "No profile for iCCP chunk")` | [x] |
| 1088 | `png_write_iCCP` | `profile_len < 132` (pngwutil.c:1134-1135) | `png_error(png_ptr, "ICC profile too short")` | [x] |
| 1089 | `png_write_iCCP` | `png_get_uint_32(profile) != profile_len` (pngwutil.c:1137-1138) | `png_error(png_ptr, "Incorrect data in iCCP")` | [x] |
| 1090 | `png_write_iCCP` | `profile[8] > 3 && (profile_len & 0x03)` (pngwutil.c:1140-1142) | `png_error(png_ptr, "ICC profile length invalid (not a multiple of 4)")` | [x] |
| 1091 | `png_write_iCCP` | `profile_len != embedded_profile_len` (second, redundant check) (pngwutil.c:1144-1149) | `png_error(png_ptr, "Profile length does not match profile")` | [x] |
| 1092 | `png_write_iCCP` | keyword rejected by `png_check_keyword` (empty / all-space / invalid): `name_len == 0` (pngwutil.c:1151-1154) | `png_error(png_ptr, "iCCP: invalid keyword")` | [x] |
| 1093 | `png_write_iCCP` | `png_text_compress(png_iCCP, ...) != Z_OK` (pngwutil.c:1164-1165) | `png_error(png_ptr, png_ptr->zstream.msg)` | [x] |
| 1094 | `png_write_sPLT` | `png_check_keyword(spalette->name, ...)` returns 0 (pngwutil.c:1191-1194) | `png_error(png_ptr, "sPLT: invalid keyword")` | [x] |
| 1095 | `png_write_sBIT` | colour image with `sbit->red/green/blue == 0` or `> maxbits` (`8` for palette, else `usr_bit_depth`) (pngwutil.c:1250-1256) | `png_warning(png_ptr, "Invalid sBIT depth specified")` and return (chunk dropped) | [x] |
| 1096 | `png_write_sBIT` | greyscale with `sbit->gray == 0 \|\| sbit->gray > png_ptr->usr_bit_depth` (pngwutil.c:1266-1270) | `png_warning(png_ptr, "Invalid sBIT depth specified")`, chunk dropped | [x] |
| 1097 | `png_write_sBIT` | alpha channel with `sbit->alpha == 0 \|\| sbit->alpha > png_ptr->usr_bit_depth` (pngwutil.c:1278-1282) | `png_warning(png_ptr, "Invalid sBIT depth specified")`, chunk dropped | [x] |
| 1098 | `png_write_tRNS` | palette image with `num_trans <= 0` or `num_trans > png_ptr->num_palette` (pngwutil.c:1329-1334) | `png_app_warning(png_ptr, "Invalid number of transparent colors specified")`, chunk dropped | [x] |
| 1099 | `png_write_tRNS` | greyscale with `tran->gray >= (1 << png_ptr->bit_depth)` (pngwutil.c:1344-1350) | `png_app_warning(png_ptr, "Ignoring attempt to write tRNS chunk out-of-range for bit_depth")` | [x] |
| 1100 | `png_write_tRNS` | RGB at bit depth 8 with non-zero high bytes: `bit_depth == 8 && (buf[0] \| buf[2] \| buf[4]) != 0` (pngwutil.c:1362-1371) | `png_app_warning(png_ptr, "Ignoring attempt to write 16-bit tRNS chunk when bit_depth is 8")` | [x] |
| 1101 | `png_write_tRNS` | colour type that already has an alpha channel (pngwutil.c:1376-1379) | `png_app_warning(png_ptr, "Can't write tRNS with an alpha channel")`, chunk dropped | [x] |
| 1102 | `png_write_bKGD` | palette image with `back->index >= png_ptr->num_palette` (pngwutil.c:1392-1403) | `png_warning(png_ptr, "Invalid background palette index")`, chunk dropped | [x] |
| 1103 | `png_write_bKGD` | colour at bit depth 8 with non-zero high bytes (pngwutil.c:1414-1425) | `png_warning(png_ptr, "Ignoring attempt to write 16-bit bKGD chunk when bit_depth is 8")` | [x] |
| 1104 | `png_write_bKGD` | greyscale with `back->gray >= (1 << png_ptr->bit_depth)` (pngwutil.c:1432-1438) | `png_warning(png_ptr, "Ignoring attempt to write bKGD chunk out-of-range for bit_depth")` | [x] |
| 1105 | `png_write_hIST` | `num_hist > (int)png_ptr->num_palette` (pngwutil.c:1545-1552) | `png_warning(png_ptr, "Invalid number of histogram entries specified")`, chunk dropped | [x] |
| 1106 | `png_write_tEXt` | `png_check_keyword` rejects the key: `key_len == 0` (pngwutil.c:1577-1580) | `png_error(png_ptr, "tEXt: invalid keyword")` | [x] |
| 1107 | `png_write_tEXt` | `text_len > PNG_UINT_31_MAX - (key_len+1)` (pngwutil.c:1588-1589) | `png_error(png_ptr, "tEXt: text too long")` | n/a |
| 1108 | `png_write_zTXt` | `compression` neither `PNG_TEXT_COMPRESSION_NONE` nor `PNG_TEXT_COMPRESSION_zTXt` (pngwutil.c:1621-1628) | `png_error(png_ptr, "zTXt: invalid compression type")` | [x] |
| 1109 | `png_write_zTXt` | `png_check_keyword` rejects the key (pngwutil.c:1630-1633) | `png_error(png_ptr, "zTXt: invalid keyword")` | [x] |
| 1110 | `png_write_zTXt` | `png_text_compress(png_zTXt, ...) != Z_OK` (pngwutil.c:1643-1644) | `png_error(png_ptr, png_ptr->zstream.msg)` | [x] |
| 1111 | `png_write_iTXt` | `png_check_keyword` rejects the key (pngwutil.c:1673-1676) | `png_error(png_ptr, "iTXt: invalid keyword")` | [x] |
| 1112 | `png_write_iTXt` | `compression` not one of the four `PNG_(I)TXT_COMPRESSION_NONE/zTXt` values (pngwutil.c:1679-1693) | `png_error(png_ptr, "iTXt: invalid compression")` | [x] |
| 1113 | `png_write_iTXt` | language tag / translated keyword so long that the prefix overflows: `lang_len > PNG_UINT_31_MAX-prefix_len` or `lang_key_len > PNG_UINT_31_MAX-prefix_len` (pngwutil.c:1714-1723) | `prefix_len` saturated to `PNG_UINT_31_MAX`, forcing the length errors below | n/a |
| 1114 | `png_write_iTXt` | `png_text_compress(png_iTXt, ...) != Z_OK` (pngwutil.c:1727-1731) | `png_error(png_ptr, png_ptr->zstream.msg)` | [x] |
| 1115 | `png_write_iTXt` | uncompressed iTXt where `comp.input_len > PNG_UINT_31_MAX-prefix_len` (pngwutil.c:1735-1736) | `png_error(png_ptr, "iTXt: uncompressed text too long")` | n/a |
| 1116 | `png_write_oFFs` | `unit_type >= PNG_OFFSET_LAST` (pngwutil.c:1770-1771) | `png_warning(png_ptr, "Unrecognized unit type for oFFs chunk")`, chunk still written | [x] |
| 1117 | `png_write_pCAL` | `type >= PNG_EQUATION_LAST` (pngwutil.c:1796-1797) | `png_error(png_ptr, "Unrecognized equation type for pCAL chunk")` | [x] |
| 1118 | `png_write_pCAL` | `png_check_keyword` rejects the purpose string (pngwutil.c:1799-1802) | `png_error(png_ptr, "pCAL: invalid keyword")` | [x] |
| 1119 | `png_write_sCAL_s` | `total_len = strlen(width)+strlen(height)+2 > 64` (pngwutil.c:1856-1864) | `png_warning(png_ptr, "Can't write sCAL (buffer too small)")`, chunk dropped | [x] |
| 1120 | `png_write_pHYs` | `unit_type >= PNG_RESOLUTION_LAST` (pngwutil.c:1886-1887) | `png_warning(png_ptr, "Unrecognized unit type for pHYs chunk")`, chunk still written | [x] |
| 1121 | `png_write_tIME` | `month > 12 \|\| month < 1 \|\| day > 31 \|\| day < 1 \|\| hour > 23 \|\| second > 60` (pngwutil.c:1908-1914) | `png_warning(png_ptr, "Invalid time specified for tIME chunk")`, chunk dropped | [x] |
| 1122 | `png_write_start_row` | `png_ptr->height == 1` or `png_ptr->width == 1` with UP/AVG/PAETH/SUB selected (pngwutil.c:1955-1962) | those filters silently removed; if nothing remains, `filters = PNG_FILTER_NONE` | [x] |
| 1123 | `png_write_find_filter` | row so wide that the filter cost sum could overflow: `PNG_SIZE_MAX/128 <= row_bytes` (pngwutil.c:2600-2606) | filter search abandoned; `filter_to_do &= 0U-filter_to_do` selects only the lowest set filter | n/a |
| 1124 | `png_do_write_interlace` | `pass >= 6` (last pass) (pngwutil.c:2108) | no-op; row and `row_info` left unchanged | [x] |
| 1125 | `png_do_pack` | row not 8-bit single channel: `!(row_info->bit_depth == 8 && row_info->channels == 1)` (pngwtran.c:28-30) | no packing performed and `row_info` left unchanged (silently ignored) | [x] |
| 1126 | `png_do_pack` | target `bit_depth` not 1, 2 or 4 (`default:` case) (pngwtran.c:150-151) | no packing done, but `row_info->bit_depth/pixel_depth/rowbytes` still rewritten to the requested depth | [x] |
| 1127 | `png_do_shift` | `row_info->color_type == PNG_COLOR_TYPE_PALETTE` (pngwtran.c:176) | no shifting performed (silently ignored for colour-mapped rows) | [x] |
| 1128 | `png_do_write_transformations` | `png_ptr == NULL` (pngwtran.c:504-505) | returns, no transformations applied | [x] |
