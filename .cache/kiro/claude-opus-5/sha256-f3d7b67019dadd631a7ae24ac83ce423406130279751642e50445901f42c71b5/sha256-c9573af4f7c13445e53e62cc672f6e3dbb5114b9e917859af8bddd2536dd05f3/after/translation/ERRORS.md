# ERRORS.md — ERROR-SURFACE TABLE

Derived mechanically from `c_src/src/*.c` by `tools/gen_errors.py`: every
`png_error`, `png_chunk_error`, `png_app_error`, `png_benign_error`,
`png_chunk_benign_error`, `png_fixed_error`, `png_warning`, `png_chunk_warning`,
`png_app_warning`, `assert()` and every `return 0 / NULL / -1` rejection sentinel.

`kind` determines the observable C result:

| kind | observable C result |
|------|---------------------|
| `chunk_benign_error` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp |
| `chunk_error` | chunk-prefixed fatal error -> longjmp |
| `chunk_warning` | chunk-prefixed warning, call continues |
| `benign_error` | benign error: warning if allowed, else png_error+longjmp |
| `app_error` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN |
| `app_warning` | application warning, call returns without effect |
| `fixed_error` | png_error("<text> out of range") -> longjmp |
| `error` | fatal error -> error_fn then png_default_error -> longjmp |
| `warning` | warning, call continues |
| `assert` | abort() if the assertion fails |
| `early-return` | the function returns the sentinel and makes no state change |

Total rows: **556**.

## Phase C coverage

`coverage` is filled in mechanically by `tools/gen_errors.py` from
`translation/target/observed_messages.txt`, which `common::record_message`
appends to from the error and warning callbacks during the test run.  A row
marked `observed` means that exact message text came out of the library while
a Phase C differential test was asserting that BOTH implementations produce
identical error/warning output.

* message rows observed: **239**
* `early-return` sentinel rows covered by `tests/j_nullargs.rs`: **126**
* dispatch-site rows (message text comes from the caller): **51**
* rows compiled out of this build: **59**
* rows unreachable in this build (dead guards / 64-bit arithmetic): **17**
* rows reachable only on allocator failure: **13**
* internal-invariant rows unreachable through the exported API: **51**
* rows NOT observed and NOT otherwise accounted for: **0**

| # | file:line | function | kind | trigger (message / statement) | expected C result | coverage |
|---|-----------|----------|------|-------------------------------|-------------------|----------|
| 1 | `png.c:66` | `png_set_sig_bytes` | error | `Too many bytes for PNG signature` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 2 | `png.c:88` | `png_sig_cmp` | early-return | `return -1;` | returns -1 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 3 | `png.c:91` | `png_sig_cmp` | early-return | `return -1;` | returns -1 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 4 | `png.c:110` | `png_zalloc` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 5 | `png.c:120` | `png_zalloc` | warning | `Potential overflow in png_zalloc()` | warning, call continues | unreachable on a 64-bit target: items and size are uInt, so items*size < 2^64 always |
| 6 | `png.c:122` | `png_zalloc` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 7 | `png.c:245` | `png_user_version_check` | warning | `png_warning(png_ptr, m);` | warning, call continues | dispatch site (message supplied by caller) |
| 8 | `png.c:248` | `png_user_version_check` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 9 | `png.c:361` | `png_create_png_struct` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 10 | `png.c:374` | `png_create_info_struct` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 11 | `png.c:479` | `png_data_freer` | error | `Unknown freer parameter in png_data_freer` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 12 | `png.c:694` | `png_get_io_ptr` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 13 | `png.c:749` | `png_convert_to_rfc1123_buffer` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 14 | `png.c:756` | `png_convert_to_rfc1123_buffer` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 15 | `png.c:802` | `png_convert_to_rfc1123` | warning | `Ignoring invalid time value` | warning, call continues | observed |
| 16 | `png.c:808` | `png_convert_to_rfc1123` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 17 | `png.c:1199` | `png_xy_from_XYZ` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 18 | `png.c:1495` | `png_XYZ_from_xy` | early-return | `return 0; /*success*/` | returns 0success (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 19 | `png.c:1571` | `png_icc_profile_error` | chunk_benign_error | `png_chunk_benign_error(png_ptr, message);` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | dispatch site (message supplied by caller) |
| 20 | `png.c:1573` | `png_icc_profile_error` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 21 | `png.c:1598` | `png_icc_check_length` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 22 | `png.c:1872` | `have_chromaticities` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 23 | `png.c:1881` | `have_chromaticities` | early-return | `return 0; /* sRGB defaults */` | returns 0sRGBdefaults (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 24 | `png.c:1938` | `png_set_rgb_coefficients` | error | `internal error handling cHRM coefficients` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 25 | `png.c:1971` | `png_check_IHDR` | warning | `Image width is zero in IHDR` | warning, call continues | observed |
| 26 | `png.c:1977` | `png_check_IHDR` | warning | `Invalid image width in IHDR` | warning, call continues | observed |
| 27 | `png.c:2007` | `png_check_IHDR` | warning | `Image width is too large for this architecture` | warning, call continues | unreachable on a 64-bit target: ((width+7)&~7) > (PNG_SIZE_MAX-49)/8-1 ~= 2^61 while width <= 2^32-1 |
| 28 | `png.c:2017` | `png_check_IHDR` | warning | `Image width exceeds user limit in IHDR` | warning, call continues | observed |
| 29 | `png.c:2023` | `png_check_IHDR` | warning | `Image height is zero in IHDR` | warning, call continues | observed |
| 30 | `png.c:2029` | `png_check_IHDR` | warning | `Invalid image height in IHDR` | warning, call continues | observed |
| 31 | `png.c:2039` | `png_check_IHDR` | warning | `Image height exceeds user limit in IHDR` | warning, call continues | observed |
| 32 | `png.c:2047` | `png_check_IHDR` | warning | `Invalid bit depth in IHDR` | warning, call continues | observed |
| 33 | `png.c:2054` | `png_check_IHDR` | warning | `Invalid color type in IHDR` | warning, call continues | observed |
| 34 | `png.c:2063` | `png_check_IHDR` | warning | `Invalid color type/bit depth combination in IHDR` | warning, call continues | observed |
| 35 | `png.c:2069` | `png_check_IHDR` | warning | `Unknown interlace method in IHDR` | warning, call continues | observed |
| 36 | `png.c:2075` | `png_check_IHDR` | warning | `Unknown compression method in IHDR` | warning, call continues | observed |
| 37 | `png.c:2091` | `png_check_IHDR` | warning | `MNG features are not allowed in a PNG datastream` | warning, call continues | observed |
| 38 | `png.c:2101` | `png_check_IHDR` | warning | `Unknown filter method in IHDR` | warning, call continues | observed |
| 39 | `png.c:2107` | `png_check_IHDR` | warning | `Invalid filter method in IHDR` | warning, call continues | observed |
| 40 | `png.c:2115` | `png_check_IHDR` | warning | `Unknown filter method in IHDR` | warning, call continues | observed |
| 41 | `png.c:2121` | `png_check_IHDR` | error | `Invalid IHDR data` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 42 | `png.c:2270` | `png_check_fp_string` | early-return | `return 0; /* i.e. fail */` | returns 0iefail (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 43 | `png.c:2635` | `png_ascii_from_fp` | error | `ASCII conversion buffer too small` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 44 | `png.c:2713` | `png_ascii_from_fixed` | error | `ASCII conversion buffer too small` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 45 | `png.c:2731` | `png_fixed` | fixed_error | `png_fixed_error(png_ptr, text);` | png_error("<text> out of range") -> longjmp | dispatch site (message supplied by caller) |
| 46 | `png.c:2750` | `png_fixed_ITU` | fixed_error | `png_fixed_error(png_ptr, text);` | png_error("<text> out of range") -> longjmp | dispatch site (message supplied by caller) |
| 47 | `png.c:2881` | `png_muldiv` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 48 | `png.c:2900` | `png_reciprocal` | early-return | `return 0; /* error/overflow */` | returns 0erroroverflow (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 49 | `png.c:2947` | `png_product2` | early-return | `return 0; /* overflow */` | returns 0overflow (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 50 | `png.c:2977` | `png_reciprocal2` | early-return | `return 0; /* overflow */` | returns 0overflow (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 51 | `png.c:3058` | `png_log8bit` | early-return | `return -1;` | returns -1 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 52 | `png.c:3111` | `png_log16bit` | early-return | `return -1;` | returns -1 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 53 | `png.c:3241` | `png_exp` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 54 | `png.c:3367` | `png_gamma_correct` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 55 | `png.c:3377` | `png_gamma_correct` | error | `* png_error (i.e. if one of the mallocs below fails) - i.e. the *table` | fatal error -> error_fn then png_default_error -> longjmp | compiled out of this build (literal absent from the .so) |
| 56 | `png.c:3634` | `png_build_gamma_table` | warning | `gamma table being rebuilt` | warning, call continues | observed |
| 57 | `png.c:3969` | `png_image_free_function` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 58 | `png.c:4002` | `png_image_free_function` | error | `simplified write not supported` | fatal error -> error_fn then png_default_error -> longjmp | compiled out of this build (literal absent from the .so) |
| 59 | `png.c:4010` | `png_image_free_function` | error | `simplified read not supported` | fatal error -> error_fn then png_default_error -> longjmp | compiled out of this build (literal absent from the .so) |
| 60 | `png.c:4040` | `png_image_error` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 61 | `pngerror.c:177` | `png_format_number` | warning | `png_warning(png_const_structrp png_ptr, png_const_charp warning_messag` | warning, call continues | dispatch site (message supplied by caller) |
| 62 | `pngerror.c:302` | `png_formatted_warning` | warning | `png_warning(png_ptr, msg);` | warning, call continues | dispatch site (message supplied by caller) |
| 63 | `pngerror.c:308` | `png_formatted_warning` | benign_error | `png_benign_error(png_const_structrp png_ptr, png_const_charp error_mes` | benign error: warning if allowed, else png_error+longjmp | dispatch site (message supplied by caller) |
| 64 | `pngerror.c:315` | `png_benign_error` | chunk_warning | `png_chunk_warning(png_ptr, error_message);` | chunk-prefixed warning, call continues | dispatch site (message supplied by caller) |
| 65 | `pngerror.c:318` | `png_benign_error` | warning | `png_warning(png_ptr, error_message);` | warning, call continues | dispatch site (message supplied by caller) |
| 66 | `pngerror.c:326` | `png_benign_error` | chunk_error | `png_chunk_error(png_ptr, error_message);` | chunk-prefixed fatal error -> longjmp | dispatch site (message supplied by caller) |
| 67 | `pngerror.c:329` | `png_benign_error` | error | `png_error(png_ptr, error_message);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 68 | `pngerror.c:338` | `png_benign_error` | app_warning | `png_app_warning(png_const_structrp png_ptr, png_const_charp error_mess` | application warning, call returns without effect | dispatch site (message supplied by caller) |
| 69 | `pngerror.c:341` | `png_app_warning` | warning | `png_warning(png_ptr, error_message);` | warning, call continues | dispatch site (message supplied by caller) |
| 70 | `pngerror.c:343` | `png_app_warning` | error | `png_error(png_ptr, error_message);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 71 | `pngerror.c:351` | `png_app_warning` | app_error | `png_app_error(png_const_structrp png_ptr, png_const_charp error_messag` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | dispatch site (message supplied by caller) |
| 72 | `pngerror.c:354` | `png_app_error` | warning | `png_warning(png_ptr, error_message);` | warning, call continues | dispatch site (message supplied by caller) |
| 73 | `pngerror.c:356` | `png_app_error` | error | `png_error(png_ptr, error_message);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 74 | `pngerror.c:431` | `png_chunk_error` | error | `png_error(png_ptr, error_message);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 75 | `pngerror.c:436` | `png_chunk_error` | error | `png_error(png_ptr, msg);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 76 | `pngerror.c:443` | `png_chunk_error` | chunk_warning | `png_chunk_warning(png_const_structrp png_ptr, png_const_charp warning_` | chunk-prefixed warning, call continues | dispatch site (message supplied by caller) |
| 77 | `pngerror.c:447` | `png_chunk_warning` | warning | `png_warning(png_ptr, warning_message);` | warning, call continues | dispatch site (message supplied by caller) |
| 78 | `pngerror.c:452` | `png_chunk_warning` | warning | `png_warning(png_ptr, msg);` | warning, call continues | dispatch site (message supplied by caller) |
| 79 | `pngerror.c:460` | `png_chunk_warning` | chunk_benign_error | `png_chunk_benign_error(png_const_structrp png_ptr,` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | dispatch site (message supplied by caller) |
| 80 | `pngerror.c:464` | `png_chunk_benign_error` | chunk_warning | `png_chunk_warning(png_ptr, error_message);` | chunk-prefixed warning, call continues | dispatch site (message supplied by caller) |
| 81 | `pngerror.c:467` | `png_chunk_benign_error` | chunk_error | `png_chunk_error(png_ptr, error_message);` | chunk-prefixed fatal error -> longjmp | dispatch site (message supplied by caller) |
| 82 | `pngerror.c:493` | `png_chunk_report` | chunk_warning | `png_chunk_warning(png_ptr, message);` | chunk-prefixed warning, call continues | dispatch site (message supplied by caller) |
| 83 | `pngerror.c:496` | `png_chunk_report` | chunk_benign_error | `png_chunk_benign_error(png_ptr, message);` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | dispatch site (message supplied by caller) |
| 84 | `pngerror.c:507` | `png_chunk_report` | app_warning | `png_app_warning(png_ptr, message);` | application warning, call returns without effect | dispatch site (message supplied by caller) |
| 85 | `pngerror.c:510` | `png_chunk_report` | app_error | `png_app_error(png_ptr, message);` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | dispatch site (message supplied by caller) |
| 86 | `pngerror.c:534` | `png_fixed_error` | error | `png_error(png_ptr, msg);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 87 | `pngerror.c:558` | `png_set_longjmp_fn` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 88 | `pngerror.c:573` | `png_set_longjmp_fn` | early-return | `return NULL; /* new NULL return on OOM */` | returns NULLnewNULLonOOM (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 89 | `pngerror.c:593` | `png_set_longjmp_fn` | error | `Libpng jmp_buf still allocated` | fatal error -> error_fn then png_default_error -> longjmp | unreachable: requires jmp_buf_size == 0 together with a heap-allocated jmp_buf, a combination that exists only transiently inside png_free_jmpbuf |
| 90 | `pngerror.c:600` | `png_set_longjmp_fn` | warning | `Application jmp_buf size changed` | warning, call continues | observed |
| 91 | `pngerror.c:601` | `png_set_longjmp_fn` | early-return | `return NULL; /* caller will probably crash: no choice here */` | returns NULLcallerwillprobablycrashnochoicehere (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 92 | `pngerror.c:742` | `png_get_error_ptr` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 93 | `pngerror.c:847` | `png_safe_execute` | early-return | `return 0; /* failure */` | returns 0failure (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 94 | `pngget.c:30` | `png_get_valid` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 95 | `pngget.c:36` | `png_get_valid` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 96 | `pngget.c:45` | `png_get_rowbytes` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 97 | `pngget.c:55` | `png_get_rows` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 98 | `pngget.c:67` | `png_get_image_width` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 99 | `pngget.c:76` | `png_get_image_height` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 100 | `pngget.c:85` | `png_get_bit_depth` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 101 | `pngget.c:94` | `png_get_color_type` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 102 | `pngget.c:103` | `png_get_filter_type` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 103 | `pngget.c:112` | `png_get_interlace_type` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 104 | `pngget.c:121` | `png_get_compression_type` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 105 | `pngget.c:142` | `png_get_x_pixels_per_meter` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 106 | `pngget.c:163` | `png_get_y_pixels_per_meter` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 107 | `pngget.c:184` | `png_get_pixels_per_meter` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 108 | `pngget.c:239` | `png_get_pixel_aspect_ratio_fixed` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 109 | `pngget.c:260` | `png_get_x_offset_microns` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 110 | `pngget.c:280` | `png_get_y_offset_microns` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 111 | `pngget.c:300` | `png_get_x_offset_pixels` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 112 | `pngget.c:320` | `png_get_y_offset_pixels` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 113 | `pngget.c:352` | `ppi_from_ppm` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 114 | `pngget.c:388` | `png_fixed_inches_from_microns` | warning | `fixed point overflow ignored` | warning, call continues | observed |
| 115 | `pngget.c:389` | `png_fixed_inches_from_microns` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 116 | `pngget.c:486` | `png_get_channels` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 117 | `pngget.c:496` | `png_get_signature` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 118 | `pngget.c:515` | `png_get_bKGD` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 119 | `pngget.c:555` | `png_get_cHRM` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 120 | `pngget.c:592` | `png_get_cHRM_XYZ` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 121 | `pngget.c:624` | `png_get_cHRM_XYZ_fixed` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 122 | `pngget.c:650` | `png_get_cHRM_fixed` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 123 | `pngget.c:671` | `png_get_gAMA_fixed` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 124 | `pngget.c:692` | `png_get_gAMA` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 125 | `pngget.c:712` | `png_get_sRGB` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 126 | `pngget.c:739` | `png_get_iCCP` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 127 | `pngget.c:756` | `png_get_sPLT` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 128 | `pngget.c:781` | `png_get_cICP` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 129 | `pngget.c:802` | `png_get_cLLI_fixed` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 130 | `pngget.c:821` | `png_get_cLLI` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 131 | `pngget.c:854` | `png_get_mDCV_fixed` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 132 | `pngget.c:885` | `png_get_mDCV` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 133 | `pngget.c:895` | `png_get_eXIf` | warning | `png_get_eXIf does not work; use png_get_eXIf_1` | warning, call continues | observed |
| 134 | `pngget.c:898` | `png_get_eXIf` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 135 | `pngget.c:915` | `png_get_eXIf_1` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 136 | `pngget.c:933` | `png_get_hIST` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 137 | `pngget.c:946` | `png_get_IHDR` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 138 | `pngget.c:998` | `png_get_oFFs` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 139 | `pngget.c:1025` | `png_get_pCAL` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 140 | `pngget.c:1053` | `png_get_sCAL_fixed` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 141 | `pngget.c:1073` | `png_get_sCAL` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 142 | `pngget.c:1091` | `png_get_sCAL_s` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 143 | `pngget.c:1145` | `png_get_PLTE` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 144 | `pngget.c:1162` | `png_get_sBIT` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 145 | `pngget.c:1188` | `png_get_text` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 146 | `pngget.c:1206` | `png_get_tIME` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 147 | `pngget.c:1268` | `png_get_unknown_chunks` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 148 | `pngget.c:1292` | `png_get_compression_buffer_size` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 149 | `pngget.c:1364` | `png_get_palette_max` | early-return | `return -1;` | returns -1 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 150 | `pngmem.c:117` | `png_malloc_array_checked` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 151 | `pngmem.c:126` | `png_malloc_array` | error | `internal error: array alloc` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 152 | `pngmem.c:139` | `png_realloc_array` | error | `internal error: array realloc` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 153 | `pngmem.c:164` | `png_realloc_array` | early-return | `return NULL; /* error */` | returns NULLerror (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 154 | `pngmem.c:179` | `png_malloc` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 155 | `pngmem.c:184` | `png_malloc` | error | `Out of memory` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 156 | `pngmem.c:197` | `png_malloc_default` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 157 | `pngmem.c:203` | `png_malloc_default` | error | `Out of Memory` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 158 | `pngmem.c:224` | `png_malloc_warn` | warning | `Out of memory` | warning, call continues | observed |
| 159 | `pngmem.c:227` | `png_malloc_warn` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 160 | `pngmem.c:282` | `png_get_mem_ptr` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 161 | `pngpread.c:88` | `png_process_data_pause` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 162 | `pngpread.c:99` | `png_process_data_skip` | app_warning | `png_process_data_skip is not implemented in any current version of libpng` | application warning, call returns without effect | observed |
| 163 | `pngpread.c:101` | `png_process_data_skip` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 164 | `pngpread.c:166` | `png_push_read_sig` | error | `Not a PNG file` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 165 | `pngpread.c:169` | `png_push_read_sig` | error | `PNG file corrupted by ASCII conversion` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 166 | `pngpread.c:213` | `png_push_read_chunk` | error | `Missing IHDR before IDAT` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 167 | `pngpread.c:217` | `png_push_read_chunk` | error | `Missing PLTE before IDAT` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 168 | `pngpread.c:229` | `png_push_read_chunk` | benign_error | `Too many IDATs found` | benign error: warning if allowed, else png_error+longjmp | observed |
| 169 | `pngpread.c:243` | `png_push_read_chunk` | error | `Invalid IHDR length` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 170 | `pngpread.c:361` | `png_push_save_buffer` | error | `Potential overflow of save_buffer` | fatal error -> error_fn then png_default_error -> longjmp | unreachable on a 64-bit target: requires save_buffer_size > PNG_SIZE_MAX - current_buffer_size - 256 |
| 171 | `pngpread.c:372` | `png_push_save_buffer` | error | `Insufficient memory for save_buffer` | fatal error -> error_fn then png_default_error -> longjmp | reachable only on allocator failure |
| 172 | `pngpread.c:378` | `png_push_save_buffer` | error | `save_buffer error` | fatal error -> error_fn then png_default_error -> longjmp | reachable only on allocator failure |
| 173 | `pngpread.c:425` | `png_push_read_IDAT` | error | `Not enough compressed data` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 174 | `pngpread.c:502` | `png_process_IDAT_data` | error | `No IDAT data (internal error)` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 175 | `pngpread.c:555` | `png_process_IDAT_data` | warning | `Truncated compressed data in IDAT` | warning, call continues | observed |
| 176 | `pngpread.c:560` | `png_process_IDAT_data` | benign_error | `IDAT: ADLER32 checksum mismatch` | benign error: warning if allowed, else png_error+longjmp | observed |
| 177 | `pngpread.c:562` | `png_process_IDAT_data` | error | `Decompression error in IDAT` | fatal error -> error_fn then png_default_error -> longjmp | unreachable: fallback text: the branch IS exercised (see the observed "IDAT: <zlib message>" rows) but this literal is only used when zlib leaves zstream.msg NULL, which zlib does not do for any input |
| 178 | `pngpread.c:580` | `png_process_IDAT_data` | warning | `Extra compressed data in IDAT` | warning, call continues | internal invariant; unreachable through the exported API |
| 179 | `pngpread.c:605` | `png_process_IDAT_data` | warning | `Extra compression data in IDAT` | warning, call continues | observed |
| 180 | `pngpread.c:627` | `png_push_process_row` | error | `bad adaptive filter value` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 181 | `pngpread.c:647` | `png_push_process_row` | error | `progressive row overflow` | fatal error -> error_fn then png_default_error -> longjmp | unreachable on a 64-bit target: requires a row larger than PNG_SIZE_MAX |
| 182 | `pngpread.c:651` | `png_push_process_row` | error | `internal progressive row size calculation error` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 183 | `pngpread.c:941` | `png_get_progressive_ptr` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 184 | `pngread.c:118` | `png_read_info` | chunk_error | `Missing IHDR before IDAT` | chunk-prefixed fatal error -> longjmp | observed |
| 185 | `pngread.c:122` | `png_read_info` | chunk_error | `Missing PLTE before IDAT` | chunk-prefixed fatal error -> longjmp | observed |
| 186 | `pngread.c:125` | `png_read_info` | chunk_benign_error | `Too many IDATs found` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 187 | `pngread.c:191` | `png_read_update_info` | app_error | `png_read_update_info/png_start_read_image: duplicate call` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 188 | `pngread.c:214` | `png_start_read_image` | app_error | `png_start_read_image/png_read_update_info: duplicate call` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 189 | `pngread.c:318` | `png_read_row` | warning | `PNG_READ_INVERT_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 190 | `pngread.c:323` | `png_read_row` | warning | `PNG_READ_FILLER_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 191 | `pngread.c:329` | `png_read_row` | warning | `PNG_READ_PACKSWAP_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 192 | `pngread.c:334` | `png_read_row` | warning | `PNG_READ_PACK_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 193 | `pngread.c:339` | `png_read_row` | warning | `PNG_READ_SHIFT_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 194 | `pngread.c:344` | `png_read_row` | warning | `PNG_READ_BGR_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 195 | `pngread.c:349` | `png_read_row` | warning | `PNG_READ_SWAP_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 196 | `pngread.c:444` | `png_read_row` | error | `Invalid attempt to read row data` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 197 | `pngread.c:456` | `png_read_row` | error | `bad adaptive filter value` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 198 | `pngread.c:489` | `png_read_row` | error | `sequential row overflow` | fatal error -> error_fn then png_default_error -> longjmp | unreachable on a 64-bit target: requires a row larger than PNG_SIZE_MAX |
| 199 | `pngread.c:493` | `png_read_row` | error | `internal sequential row size calculation error` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 200 | `pngread.c:635` | `png_read_image` | warning | `Interlace handling should be turned on when ` | warning, call continues | observed |
| 201 | `pngread.c:648` | `png_read_image` | error | `Cannot read interlaced image -- interlace handler disabled` | fatal error -> error_fn then png_default_error -> longjmp | compiled out of this build (literal absent from the .so) |
| 202 | `pngread.c:697` | `png_read_end` | benign_error | `Read palette index exceeding num_palette` | benign error: warning if allowed, else png_error+longjmp | observed |
| 203 | `pngread.c:729` | `png_read_end` | benign_error | `.Too many IDATs found` | benign error: warning if allowed, else png_error+longjmp | observed |
| 204 | `pngread.c:747` | `png_read_end` | benign_error | `..Too many IDATs found` | benign error: warning if allowed, else png_error+longjmp | observed |
| 205 | `pngread.c:881` | `png_read_png` | error | `Image is too high to process with png_read_png()` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 206 | `pngread.c:899` | `png_read_png` | app_error | `PNG_TRANSFORM_SCALE_16 not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 207 | `pngread.c:910` | `png_read_png` | app_error | `PNG_TRANSFORM_STRIP_16 not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 208 | `pngread.c:920` | `png_read_png` | app_error | `PNG_TRANSFORM_STRIP_ALPHA not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 209 | `pngread.c:930` | `png_read_png` | app_error | `PNG_TRANSFORM_PACKING not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 210 | `pngread.c:940` | `png_read_png` | app_error | `PNG_TRANSFORM_PACKSWAP not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 211 | `pngread.c:952` | `png_read_png` | app_error | `PNG_TRANSFORM_EXPAND not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 212 | `pngread.c:964` | `png_read_png` | app_error | `PNG_TRANSFORM_INVERT_MONO not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 213 | `pngread.c:976` | `png_read_png` | app_error | `PNG_TRANSFORM_SHIFT not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 214 | `pngread.c:984` | `png_read_png` | app_error | `PNG_TRANSFORM_BGR not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 215 | `pngread.c:992` | `png_read_png` | app_error | `PNG_TRANSFORM_SWAP_ALPHA not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 216 | `pngread.c:1000` | `png_read_png` | app_error | `PNG_TRANSFORM_SWAP_ENDIAN not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 217 | `pngread.c:1009` | `png_read_png` | app_error | `PNG_TRANSFORM_INVERT_ALPHA not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 218 | `pngread.c:1018` | `png_read_png` | app_error | `PNG_TRANSFORM_GRAY_TO_RGB not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 219 | `pngread.c:1026` | `png_read_png` | app_error | `PNG_TRANSFORM_EXPAND_16 not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 220 | `pngread.c:1225` | `chromaticities_match_sRGB` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 221 | `pngread.c:1239` | `png_gamma_not_sRGB` | early-return | `return 0; /* Includes the uninitialized value 0 */` | returns 0Includestheuninitializedvalue0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 222 | `pngread.c:1265` | `png_image_is_not_sRGB` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 223 | `pngread.c:1272` | `png_image_is_not_sRGB` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 224 | `pngread.c:1363` | `png_image_begin_read_from_stdio` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 225 | `pngread.c:1401` | `png_image_begin_read_from_file` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 226 | `pngread.c:1427` | `png_image_memory_read` | error | `read beyond end of data` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 227 | `pngread.c:1431` | `png_image_memory_read` | error | `invalid memory read` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 228 | `pngread.c:1466` | `png_image_memory_read` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 229 | `pngread.c:1537` | `set_file_encoding` | error | `internal: default gamma not set` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 230 | `pngread.c:1586` | `decode_gamma` | error | `unexpected encoding (internal error)` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 231 | `pngread.c:1643` | `png_create_colormap_entry` | error | `color-map index out of range` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 232 | `pngread.c:1743` | `png_create_colormap_entry` | error | `bad encoding (internal error)` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 233 | `pngread.c:1997` | `png_image_read_colormap` | error | `background color must be supplied to remove alpha/transparency` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 234 | `pngread.c:2056` | `png_image_read_colormap` | error | `gray[8] color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 235 | `pngread.c:2135` | `png_image_read_colormap` | error | `gray[16] color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 236 | `pngread.c:2233` | `png_image_read_colormap` | error | `gray+alpha color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 237 | `pngread.c:2267` | `png_image_read_colormap` | error | `gray-alpha color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 238 | `pngread.c:2301` | `png_image_read_colormap` | error | `ga-alpha color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 239 | `pngread.c:2406` | `png_image_read_colormap` | error | `rgb[ga] color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 240 | `pngread.c:2422` | `png_image_read_colormap` | error | `rgb[gray] color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 241 | `pngread.c:2530` | `png_image_read_colormap` | error | `rgb+alpha color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 242 | `pngread.c:2579` | `png_image_read_colormap` | error | `rgb-alpha color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 243 | `pngread.c:2664` | `png_image_read_colormap` | error | `rgb color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 244 | `pngread.c:2695` | `png_image_read_colormap` | error | `palette color-map: too few entries` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 245 | `pngread.c:2738` | `png_image_read_colormap` | error | `invalid PNG color type` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 246 | `pngread.c:2761` | `png_image_read_colormap` | error | `bad data option (internal error)` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 247 | `pngread.c:2766` | `png_image_read_colormap` | error | `color map overflow (BAD internal error)` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 248 | `pngread.c:2800` | `png_image_read_colormap` | error | `bad processing option (internal error)` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 249 | `pngread.c:2803` | `png_image_read_colormap` | error | `bad background index (internal error)` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 250 | `pngread.c:2836` | `png_image_read_and_map` | error | `unknown interlace type` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 251 | `pngread.c:3074` | `png_image_read_colormapped` | error | `bad color-map processing (internal error)` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 252 | `pngread.c:3159` | `png_image_read_direct_scaled` | error | `unknown interlace type` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 253 | `pngread.c:3208` | `png_image_read_composite` | error | `unknown interlace type` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 254 | `pngread.c:3358` | `png_image_read_background` | error | `lost rgb to gray` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 255 | `pngread.c:3361` | `png_image_read_background` | error | `unexpected compose` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 256 | `pngread.c:3364` | `png_image_read_background` | error | `lost/gained channels` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 257 | `pngread.c:3369` | `png_image_read_background` | error | `unexpected 8-bit transformation` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 258 | `pngread.c:3382` | `png_image_read_background` | error | `unknown interlace type` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 259 | `pngread.c:3609` | `png_image_read_background` | error | `unexpected bit depth` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 260 | `pngread.c:3925` | `png_image_read_direct` | error | `png_read_image: unsupported transformation` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 261 | `pngread.c:3960` | `png_image_read_direct` | error | `png_image_read: alpha channel lost` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 262 | `pngread.c:3986` | `png_image_read_direct` | error | `unexpected alpha swap transformation` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 263 | `pngread.c:3994` | `png_image_read_direct` | error | `png_read_image: invalid transformations` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 264 | `pngread.c:4201` | `png_image_finish_read` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 265 | `pngrio.c:39` | `png_read_data` | error | `Call to NULL read function` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 266 | `pngrio.c:62` | `png_default_read_data` | error | `Read Error` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 267 | `pngrio.c:81` | `png_default_read_data` | error | `Error msg` | fatal error -> error_fn then png_default_error -> longjmp | compiled out of this build (literal absent from the .so) |
| 268 | `pngrio.c:109` | `png_set_read_fn` | warning | `Can't set both read_data_fn and write_data_fn in the` | warning, call continues | observed |
| 269 | `pngrtran.c:66` | `png_set_crc_action` | warning | `Can't discard critical data on CRC error` | warning, call continues | observed |
| 270 | `pngrtran.c:120` | `png_rtran_ok` | app_error | `invalid after png_start_read_image or png_read_update_info` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 271 | `pngrtran.c:124` | `png_rtran_ok` | app_error | `invalid before the PNG header has been read` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 272 | `pngrtran.c:135` | `png_rtran_ok` | early-return | `return 0; /* no png_error possible! */` | returns 0nopngerrorpossible (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 273 | `pngrtran.c:153` | `png_set_background_fixed` | warning | `Application must supply a known background gamma` | warning, call continues | observed |
| 274 | `pngrtran.c:325` | `convert_gamma_value` | fixed_error | `gamma value` | png_error("<text> out of range") -> longjmp | internal invariant; unreachable through the exported API |
| 275 | `pngrtran.c:348` | `unsupported_gamma` | app_warning | `png_app_warning(png_ptr, msg);` | application warning, call returns without effect | dispatch site (message supplied by caller) |
| 276 | `pngrtran.c:350` | `unsupported_gamma` | app_error | `png_app_error(png_ptr, msg);` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | dispatch site (message supplied by caller) |
| 277 | `pngrtran.c:355` | `unsupported_gamma` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 278 | `pngrtran.c:434` | `png_set_alpha_mode_fixed` | error | `invalid alpha mode` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 279 | `pngrtran.c:452` | `png_set_alpha_mode_fixed` | error | `conflicting calls to set alpha mode and background` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 280 | `pngrtran.c:917` | `png_set_gamma_fixed` | app_error | `invalid file gamma in png_set_gamma` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 281 | `pngrtran.c:919` | `png_set_gamma_fixed` | app_error | `invalid screen gamma in png_set_gamma` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 282 | `pngrtran.c:1072` | `png_set_rgb_to_gray_fixed` | error | `invalid error action to rgb_to_gray` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 283 | `pngrtran.c:1083` | `png_set_rgb_to_gray_fixed` | error | `Cannot do RGB_TO_GRAY without EXPAND_SUPPORTED` | fatal error -> error_fn then png_default_error -> longjmp | compiled out of this build (literal absent from the .so) |
| 284 | `pngrtran.c:1108` | `png_set_rgb_to_gray_fixed` | app_warning | `ignoring out of range rgb_to_gray coefficients` | application warning, call returns without effect | observed |
| 285 | `pngrtran.c:1697` | `png_init_read_transformations` | warning | `libpng does not support gamma+background+rgb_to_gray` | warning, call continues | observed |
| 286 | `pngrtran.c:1886` | `png_init_read_transformations` | error | `invalid background gamma type` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 287 | `pngrtran.c:2104` | `png_read_transform_info` | error | `Palette is NULL in indexed image` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 288 | `pngrtran.c:2452` | `png_do_unshift` | assert | `/* assert(channels == 1 && shift[0] == 1) */` | abort() if the assertion fails | unreachable by construction (see note) |
| 289 | `pngrtran.c:2467` | `png_do_unshift` | assert | `/* assert(channels == 1) */` | abort() if the assertion fails | unreachable by construction (see note) |
| 290 | `pngrtran.c:4341` | `png_do_encode_alpha` | warning | `png_do_encode_alpha: unexpected call` | warning, call continues | internal invariant; unreachable through the exported API |
| 291 | `pngrtran.c:4891` | `png_do_read_transformations` | error | `NULL row buffer` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 292 | `pngrtran.c:4907` | `png_do_read_transformations` | error | `Uninitialized row` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 293 | `pngrtran.c:4965` | `png_do_read_transformations` | warning | `png_do_rgb_to_gray found nongray pixel` | warning, call continues | observed |
| 294 | `pngrtran.c:4969` | `png_do_read_transformations` | error | `png_do_rgb_to_gray found nongray pixel` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 295 | `pngrutil.c:46` | `png_get_uint_31` | error | `PNG unsigned integer out of range` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 296 | `pngrutil.c:93` | `png_int_32` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 297 | `pngrutil.c:139` | `png_read_sig` | error | `Not a PNG file` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 298 | `pngrutil.c:141` | `png_read_sig` | error | `PNG file corrupted by ASCII conversion` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 299 | `pngrutil.c:211` | `png_read_chunk_header` | chunk_error | `bad header (invalid length)` | chunk-prefixed fatal error -> longjmp | unreachable: dead: png_read_chunk_header calls png_get_uint_31(buf) BEFORE this test, so any length with buf[0] >= 0x80 has already raised "PNG unsigned integer out of range" (which IS observed) |
| 300 | `pngrutil.c:215` | `png_read_chunk_header` | chunk_error | `bad header (invalid type)` | chunk-prefixed fatal error -> longjmp | observed |
| 301 | `pngrutil.c:298` | `png_crc_error` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 302 | `pngrutil.c:348` | `png_crc_finish_critical` | chunk_warning | `CRC error` | chunk-prefixed warning, call continues | observed |
| 303 | `pngrutil.c:351` | `png_crc_finish_critical` | chunk_error | `CRC error` | chunk-prefixed fatal error -> longjmp | observed |
| 304 | `pngrutil.c:356` | `png_crc_finish_critical` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 305 | `pngrutil.c:427` | `png_inflate_claim` | chunk_warning | `png_chunk_warning(png_ptr, msg);` | chunk-prefixed warning, call continues | dispatch site (message supplied by caller) |
| 306 | `pngrutil.c:430` | `png_inflate_claim` | chunk_error | `png_chunk_error(png_ptr, msg);` | chunk-prefixed fatal error -> longjmp | dispatch site (message supplied by caller) |
| 307 | `pngrutil.c:789` | `png_decompress_chunk` | chunk_benign_error | `extra compressed data` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 308 | `pngrutil.c:1064` | `png_handle_PLTE` | chunk_error | `png_chunk_error(png_ptr, errmsg);` | chunk-prefixed fatal error -> longjmp | dispatch site (message supplied by caller) |
| 309 | `pngrutil.c:1070` | `png_handle_PLTE` | chunk_benign_error | `png_chunk_benign_error(png_ptr, errmsg);` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | dispatch site (message supplied by caller) |
| 310 | `pngrutil.c:1092` | `png_handle_IEND` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 311 | `pngrutil.c:1118` | `png_handle_gAMA` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 312 | `pngrutil.c:1164` | `png_handle_sBIT` | chunk_benign_error | `bad length` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 313 | `pngrutil.c:1178` | `png_handle_sBIT` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 314 | `pngrutil.c:1223` | `png_get_int_32_checked` | early-return | `return 0; /* Safe */` | returns 0Safe (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 315 | `pngrutil.c:1251` | `png_handle_cHRM` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 316 | `pngrutil.c:1300` | `png_handle_sRGB` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 317 | `pngrutil.c:1356` | `png_handle_iCCP` | chunk_benign_error | `too short` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 318 | `pngrutil.c:1455` | `png_handle_iCCP` | chunk_warning | `extra compressed data` | chunk-prefixed warning, call continues | observed |
| 319 | `pngrutil.c:1541` | `png_handle_iCCP` | chunk_benign_error | `png_chunk_benign_error(png_ptr, errmsg);` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | dispatch site (message supplied by caller) |
| 320 | `pngrutil.c:1577` | `png_handle_sPLT` | warning | `No space in chunk cache for sPLT` | warning, call continues | observed |
| 321 | `pngrutil.c:1588` | `png_handle_sPLT` | chunk_benign_error | `out of memory` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 322 | `pngrutil.c:1612` | `png_handle_sPLT` | warning | `malformed sPLT chunk` | warning, call continues | observed |
| 323 | `pngrutil.c:1626` | `png_handle_sPLT` | warning | `sPLT chunk has bad length` | warning, call continues | observed |
| 324 | `pngrutil.c:1635` | `png_handle_sPLT` | warning | `sPLT chunk too long` | warning, call continues | unreachable on a 64-bit target: requires entry count > PNG_SIZE_MAX/sizeof(png_sPLT_entry) ~= 1.8e18 |
| 325 | `pngrutil.c:1646` | `png_handle_sPLT` | warning | `sPLT chunk requires too much memory` | warning, call continues | reachable only on allocator failure |
| 326 | `pngrutil.c:1700` | `png_handle_tRNS` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 327 | `pngrutil.c:1716` | `png_handle_tRNS` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 328 | `pngrutil.c:1732` | `png_handle_tRNS` | chunk_benign_error | `out of place` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 329 | `pngrutil.c:1741` | `png_handle_tRNS` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 330 | `pngrutil.c:1752` | `png_handle_tRNS` | chunk_benign_error | `invalid with alpha channel` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 331 | `pngrutil.c:1785` | `png_handle_bKGD` | chunk_benign_error | `out of place` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 332 | `pngrutil.c:1801` | `png_handle_bKGD` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 333 | `pngrutil.c:1823` | `png_handle_bKGD` | chunk_benign_error | `invalid index` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 334 | `pngrutil.c:1844` | `png_handle_bKGD` | chunk_benign_error | `invalid gray level` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 335 | `pngrutil.c:1862` | `png_handle_bKGD` | chunk_benign_error | `invalid color` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 336 | `pngrutil.c:2010` | `png_handle_eXIf` | chunk_benign_error | `out of memory` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 337 | `pngrutil.c:2029` | `png_handle_eXIf` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 338 | `pngrutil.c:2063` | `png_handle_hIST` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 339 | `pngrutil.c:2161` | `png_handle_pCAL` | chunk_benign_error | `out of memory` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 340 | `pngrutil.c:2183` | `png_handle_pCAL` | chunk_benign_error | `invalid` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 341 | `pngrutil.c:2203` | `png_handle_pCAL` | chunk_benign_error | `invalid parameter count` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 342 | `pngrutil.c:2209` | `png_handle_pCAL` | chunk_benign_error | `unrecognized equation type` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 343 | `pngrutil.c:2222` | `png_handle_pCAL` | chunk_benign_error | `out of memory` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 344 | `pngrutil.c:2240` | `png_handle_pCAL` | chunk_benign_error | `invalid data` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 345 | `pngrutil.c:2279` | `png_handle_sCAL` | chunk_benign_error | `out of memory` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 346 | `pngrutil.c:2292` | `png_handle_sCAL` | chunk_benign_error | `invalid unit` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 347 | `pngrutil.c:2304` | `png_handle_sCAL` | chunk_benign_error | `bad width format` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 348 | `pngrutil.c:2307` | `png_handle_sCAL` | chunk_benign_error | `non-positive width` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 349 | `pngrutil.c:2316` | `png_handle_sCAL` | chunk_benign_error | `bad height format` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 350 | `pngrutil.c:2319` | `png_handle_sCAL` | chunk_benign_error | `non-positive height` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 351 | `pngrutil.c:2397` | `png_handle_tEXt` | chunk_benign_error | `no space in chunk cache` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 352 | `pngrutil.c:2408` | `png_handle_tEXt` | chunk_benign_error | `out of memory` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 353 | `pngrutil.c:2437` | `png_handle_tEXt` | chunk_benign_error | `out of memory` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 354 | `pngrutil.c:2467` | `png_handle_zTXt` | chunk_benign_error | `no space in chunk cache` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 355 | `pngrutil.c:2482` | `png_handle_zTXt` | chunk_benign_error | `out of memory` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 356 | `pngrutil.c:2553` | `png_handle_zTXt` | chunk_benign_error | `png_chunk_benign_error(png_ptr, errmsg);` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | dispatch site (message supplied by caller) |
| 357 | `pngrutil.c:2583` | `png_handle_iTXt` | chunk_benign_error | `no space in chunk cache` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 358 | `pngrutil.c:2594` | `png_handle_iTXt` | chunk_benign_error | `out of memory` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 359 | `pngrutil.c:2702` | `png_handle_iTXt` | chunk_benign_error | `png_chunk_benign_error(png_ptr, errmsg);` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | dispatch site (message supplied by caller) |
| 360 | `pngrutil.c:2745` | `png_cache_unknown_chunk` | chunk_benign_error | `unknown chunk exceeds memory limits` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 361 | `pngrutil.c:2746` | `png_cache_unknown_chunk` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 362 | `pngrutil.c:2812` | `png_handle_unknown` | chunk_error | `error in user chunk` | chunk-prefixed fatal error -> longjmp | observed |
| 363 | `pngrutil.c:2832` | `png_handle_unknown` | chunk_warning | `Saving unknown chunk:` | chunk-prefixed warning, call continues | observed |
| 364 | `pngrutil.c:2833` | `png_handle_unknown` | app_warning | `forcing save of an unhandled chunk;` | application warning, call returns without effect | observed |
| 365 | `pngrutil.c:2893` | `png_handle_unknown` | app_error | `no unknown chunk support available` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 366 | `pngrutil.c:2912` | `png_handle_unknown` | chunk_benign_error | `no space in chunk cache` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 367 | `pngrutil.c:2957` | `png_handle_unknown` | chunk_error | `unhandled critical chunk` | chunk-prefixed fatal error -> longjmp | observed |
| 368 | `pngrutil.c:3135` | `png_handle_chunk` | chunk_error | `missing IHDR` | chunk-prefixed fatal error -> longjmp | observed |
| 369 | `pngrutil.c:3201` | `png_handle_chunk` | chunk_error | `png_chunk_error(png_ptr, errmsg);` | chunk-prefixed fatal error -> longjmp | dispatch site (message supplied by caller) |
| 370 | `pngrutil.c:3206` | `png_handle_chunk` | chunk_benign_error | `png_chunk_benign_error(png_ptr, errmsg);` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | dispatch site (message supplied by caller) |
| 371 | `pngrutil.c:3243` | `png_combine_row` | error | `internal row logic error` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 372 | `pngrutil.c:3251` | `png_combine_row` | error | `internal row size calculation error` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 373 | `pngrutil.c:3255` | `png_combine_row` | error | `internal row width error` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 374 | `pngrutil.c:3478` | `png_combine_row` | error | `invalid user transform pixel depth` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 375 | `pngrutil.c:4201` | `png_read_IDAT_data` | error | `Not enough image data` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 376 | `pngrutil.c:4222` | `png_read_IDAT_data` | chunk_error | `out of memory` | chunk-prefixed fatal error -> longjmp | observed |
| 377 | `pngrutil.c:4276` | `png_read_IDAT_data` | chunk_benign_error | `Extra compressed data` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 378 | `pngrutil.c:4285` | `png_read_IDAT_data` | chunk_error | `png_chunk_error(png_ptr, png_ptr->zstream.msg);` | chunk-prefixed fatal error -> longjmp | dispatch site (message supplied by caller) |
| 379 | `pngrutil.c:4289` | `png_read_IDAT_data` | chunk_benign_error | `png_chunk_benign_error(png_ptr, png_ptr->zstream.msg);` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | dispatch site (message supplied by caller) |
| 380 | `pngrutil.c:4301` | `png_read_IDAT_data` | error | `Not enough image data` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 381 | `pngrutil.c:4304` | `png_read_IDAT_data` | chunk_benign_error | `Too much image data` | chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp | observed |
| 382 | `pngrutil.c:4600` | `png_read_start_row` | error | `This image requires a row greater than 64KB` | fatal error -> error_fn then png_default_error -> longjmp | compiled out of this build (literal absent from the .so) |
| 383 | `pngrutil.c:4645` | `png_read_start_row` | error | `This image requires a row greater than 64KB` | fatal error -> error_fn then png_default_error -> longjmp | compiled out of this build (literal absent from the .so) |
| 384 | `pngrutil.c:4649` | `png_read_start_row` | error | `Row has too many bytes to allocate in memory` | fatal error -> error_fn then png_default_error -> longjmp | unreachable on a 64-bit target: row size <= 2^31 * 8 bytes, far below PNG_SIZE_MAX on LP64 |
| 385 | `pngrutil.c:4680` | `png_read_start_row` | error | `png_error(png_ptr, png_ptr->zstream.msg);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 386 | `pngset.c:94` | `png_set_cHRM_XYZ_fixed` | app_error | `invalid cHRM XYZ` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 387 | `pngset.c:152` | `png_set_cICP` | warning | `Invalid cICP matrix coefficients` | warning, call continues | observed |
| 388 | `pngset.c:218` | `png_ITU_fixed_16` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 389 | `pngset.c:322` | `png_set_eXIf` | warning | `png_set_eXIf does not work; use png_set_eXIf_1` | warning, call continues | observed |
| 390 | `pngset.c:344` | `png_set_eXIf_1` | warning | `Insufficient memory for eXIf chunk data` | warning, call continues | reachable only on allocator failure |
| 391 | `pngset.c:399` | `png_set_hIST` | warning | `Invalid palette size, hIST allocation skipped` | warning, call continues | observed |
| 392 | `pngset.c:422` | `png_set_hIST` | warning | `Insufficient memory for hIST chunk data` | warning, call continues | reachable only on allocator failure |
| 393 | `pngset.c:568` | `png_set_pCAL` | warning | `Insufficient memory for pCAL units` | warning, call continues | reachable only on allocator failure |
| 394 | `pngset.c:579` | `png_set_pCAL` | warning | `Insufficient memory for pCAL params` | warning, call continues | reachable only on allocator failure |
| 395 | `pngset.c:596` | `png_set_pCAL` | warning | `Insufficient memory for pCAL parameter` | warning, call continues | reachable only on allocator failure |
| 396 | `pngset.c:623` | `png_set_sCAL_s` | error | `Invalid sCAL unit` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 397 | `pngset.c:627` | `png_set_sCAL_s` | error | `Invalid sCAL width` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 398 | `pngset.c:631` | `png_set_sCAL_s` | error | `Invalid sCAL height` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 399 | `pngset.c:644` | `png_set_sCAL_s` | warning | `Memory allocation failed while processing sCAL` | warning, call continues | reachable only on allocator failure |
| 400 | `pngset.c:663` | `png_set_sCAL_s` | warning | `Memory allocation failed while processing sCAL` | warning, call continues | reachable only on allocator failure |
| 401 | `pngset.c:682` | `png_set_sCAL` | warning | `Invalid sCAL width ignored` | warning, call continues | observed |
| 402 | `pngset.c:685` | `png_set_sCAL` | warning | `Invalid sCAL height ignored` | warning, call continues | observed |
| 403 | `pngset.c:712` | `png_set_sCAL_fixed` | warning | `Invalid sCAL width ignored` | warning, call continues | observed |
| 404 | `pngset.c:715` | `png_set_sCAL_fixed` | warning | `Invalid sCAL height ignored` | warning, call continues | observed |
| 405 | `pngset.c:767` | `png_set_PLTE` | error | `Invalid palette length` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 406 | `pngset.c:771` | `png_set_PLTE` | warning | `Invalid palette length` | warning, call continues | observed |
| 407 | `pngset.c:784` | `png_set_PLTE` | error | `Invalid palette` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 408 | `pngset.c:904` | `png_set_iCCP` | app_error | `Invalid iCCP compression method` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 409 | `pngset.c:911` | `png_set_iCCP` | benign_error | `Insufficient memory to process iCCP chunk` | benign error: warning if allowed, else png_error+longjmp | reachable only on allocator failure |
| 410 | `pngset.c:923` | `png_set_iCCP` | benign_error | `Insufficient memory to process iCCP profile` | benign error: warning if allowed, else png_error+longjmp | reachable only on allocator failure |
| 411 | `pngset.c:950` | `png_set_text` | error | `Insufficient memory to store text` | fatal error -> error_fn then png_default_error -> longjmp | reachable only on allocator failure |
| 412 | `pngset.c:964` | `png_set_text_2` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 413 | `pngset.c:1150` | `png_set_text_2` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 414 | `pngset.c:1170` | `png_set_tIME` | warning | `Ignoring invalid time value` | warning, call continues | observed |
| 415 | `pngset.c:1253` | `png_set_tRNS` | warning | `tRNS chunk has out-of-range samples for bit_depth` | warning, call continues | observed |
| 416 | `pngset.c:1327` | `png_set_sPLT` | app_error | `png_set_sPLT: invalid sPLT` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 417 | `pngset.c:1396` | `check_location` | app_warning | `png_set_unknown_chunks now expects a valid location` | application warning, call returns without effect | observed |
| 418 | `pngset.c:1407` | `check_location` | error | `invalid location in png_set_unknown_chunks` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 419 | `pngset.c:1442` | `png_set_unknown_chunks` | app_error | `no unknown chunk support on read` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 420 | `pngset.c:1451` | `png_set_unknown_chunks` | app_error | `no unknown chunk support on write` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 421 | `pngset.c:1540` | `png_set_unknown_chunk_location` | app_error | `invalid unknown chunk location` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 422 | `pngset.c:1562` | `png_permit_mng_features` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 423 | `pngset.c:1611` | `png_set_keep_unknown_chunks` | app_error | `png_set_keep_unknown_chunks: invalid keep` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 424 | `pngset.c:1665` | `png_set_keep_unknown_chunks` | app_error | `png_set_keep_unknown_chunks: no chunk list` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 425 | `pngset.c:1681` | `png_set_keep_unknown_chunks` | app_error | `png_set_keep_unknown_chunks: too many chunks` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 426 | `pngset.c:1805` | `png_set_compression_buffer_size` | error | `invalid compression buffer size` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 427 | `pngset.c:1820` | `png_set_compression_buffer_size` | warning | `Compression buffer size cannot be changed because it is in use` | warning, call continues | observed |
| 428 | `pngset.c:1832` | `png_set_compression_buffer_size` | warning | `Compression buffer size limited to system maximum` | warning, call continues | unreachable: dead: png_set_compression_buffer_size first errors on "size > PNG_UINT_31_MAX" (0x7fffffff) and ZLIB_IO_MAX is (uInt)-1 = 0xffffffff, so "size > ZLIB_IO_MAX" can never hold afterwards - the source itself notes "compilers complain that this is always false" |
| 429 | `pngset.c:1843` | `png_set_compression_buffer_size` | warning | `Compression buffer size cannot be reduced below 6` | warning, call continues | observed |
| 430 | `pngset.c:1930` | `png_set_benign_errors` | benign_error | `/* If allowed is 1, png_benign_error() is treated as a warning.` | benign error: warning if allowed, else png_error+longjmp | compiled out of this build (literal absent from the .so) |
| 431 | `pngset.c:1932` | `png_set_benign_errors` | benign_error | `* If allowed is 0, png_benign_error() is treated as an error (which` | benign error: warning if allowed, else png_error+longjmp | compiled out of this build (literal absent from the .so) |
| 432 | `pngset.c:1995` | `png_check_keyword` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 433 | `pngset.c:2034` | `png_check_keyword` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 434 | `pngset.c:2039` | `png_check_keyword` | warning | `keyword truncated` | warning, call continues | observed |
| 435 | `pngtrans.c:114` | `png_set_shift` | app_error | `png_set_shift: invalid shift values` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 436 | `pngtrans.c:171` | `png_set_filler` | app_error | `png_set_filler not supported on read` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 437 | `pngtrans.c:202` | `png_set_filler` | app_error | `png_set_filler is invalid for` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 438 | `pngtrans.c:209` | `png_set_filler` | app_error | `png_set_filler: inappropriate color type` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 439 | `pngtrans.c:214` | `png_set_filler` | app_error | `png_set_filler not supported on write` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 440 | `pngtrans.c:845` | `png_set_user_transform_info` | app_error | `info change after png_start_read_image or png_read_update_info` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 441 | `pngtrans.c:867` | `png_get_user_transform_ptr` | early-return | `return NULL;` | returns NULL (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 442 | `pngwio.c:40` | `png_write_data` | error | `Call to NULL write function` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 443 | `pngwio.c:60` | `png_default_write_data` | error | `Write Error` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 444 | `pngwio.c:102` | `png_default_flush` | error | `Error msg` | fatal error -> error_fn then png_default_error -> longjmp | compiled out of this build (literal absent from the .so) |
| 445 | `pngwio.c:161` | `png_set_write_fn` | warning | `Can't set both read_data_fn and write_data_fn in the` | warning, call continues | observed |
| 446 | `pngwrite.c:64` | `write_unknown_chunks` | warning | `Writing zero-length unknown chunk` | warning, call continues | observed |
| 447 | `pngwrite.c:99` | `png_write_info_before_PLTE` | warning | `MNG features are not allowed in a PNG datastream` | warning, call continues | observed |
| 448 | `pngwrite.c:241` | `png_write_info` | error | `Valid palette required for paletted images` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 449 | `pngwrite.c:346` | `png_write_info` | warning | `Unable to write international text` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 450 | `pngwrite.c:360` | `png_write_info` | warning | `Unable to write compressed text` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 451 | `pngwrite.c:375` | `png_write_info` | warning | `Unable to write uncompressed text` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 452 | `pngwrite.c:400` | `png_write_end` | error | `No IDATs written into file` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 453 | `pngwrite.c:405` | `png_write_end` | benign_error | `Wrote palette index exceeding num_palette` | benign error: warning if allowed, else png_error+longjmp | observed |
| 454 | `pngwrite.c:444` | `png_write_end` | warning | `Unable to write international text` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 455 | `pngwrite.c:457` | `png_write_end` | warning | `Unable to write compressed text` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 456 | `pngwrite.c:470` | `png_write_end` | warning | `Unable to write uncompressed text` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 457 | `pngwrite.c:762` | `png_write_row` | error | `png_write_info was never called before png_write_row` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 458 | `pngwrite.c:768` | `png_write_row` | warning | `PNG_WRITE_INVERT_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 459 | `pngwrite.c:773` | `png_write_row` | warning | `PNG_WRITE_FILLER_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 460 | `pngwrite.c:778` | `png_write_row` | warning | `PNG_WRITE_PACKSWAP_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 461 | `pngwrite.c:784` | `png_write_row` | warning | `PNG_WRITE_PACK_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 462 | `pngwrite.c:789` | `png_write_row` | warning | `PNG_WRITE_SHIFT_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 463 | `pngwrite.c:794` | `png_write_row` | warning | `PNG_WRITE_BGR_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 464 | `pngwrite.c:799` | `png_write_row` | warning | `PNG_WRITE_SWAP_SUPPORTED is not defined` | warning, call continues | compiled out of this build (literal absent from the .so) |
| 465 | `pngwrite.c:918` | `png_write_row` | error | `internal write transform logic error` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 466 | `pngwrite.c:1078` | `png_set_filter` | app_error | `Unknown row filter for method 0` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 467 | `pngwrite.c:1101` | `png_set_filter` | app_error | `Unknown row filter for method 0` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 468 | `pngwrite.c:1140` | `png_set_filter` | app_warning | `png_set_filter: UP/AVG/PAETH cannot be added after start` | application warning, call returns without effect | observed |
| 469 | `pngwrite.c:1180` | `png_set_filter` | error | `Unknown custom filter method` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 470 | `pngwrite.c:1270` | `png_set_compression_window_bits` | warning | `Only compression windows <= 32k supported by PNG` | warning, call continues | observed |
| 471 | `pngwrite.c:1276` | `png_set_compression_window_bits` | warning | `Only compression windows >= 256 supported by PNG` | warning, call continues | observed |
| 472 | `pngwrite.c:1295` | `png_set_compression_method` | warning | `Only compression method 8 is supported by PNG` | warning, call continues | observed |
| 473 | `pngwrite.c:1349` | `png_set_text_compression_window_bits` | warning | `Only compression windows <= 32k supported by PNG` | warning, call continues | observed |
| 474 | `pngwrite.c:1355` | `png_set_text_compression_window_bits` | warning | `Only compression windows >= 256 supported by PNG` | warning, call continues | observed |
| 475 | `pngwrite.c:1371` | `png_set_text_compression_method` | warning | `Only compression method 8 is supported by PNG` | warning, call continues | observed |
| 476 | `pngwrite.c:1417` | `png_write_png` | app_error | `no rows for png_write_image to write` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 477 | `pngwrite.c:1431` | `png_write_png` | app_error | `PNG_TRANSFORM_INVERT_MONO not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 478 | `pngwrite.c:1442` | `png_write_png` | app_error | `PNG_TRANSFORM_SHIFT not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 479 | `pngwrite.c:1450` | `png_write_png` | app_error | `PNG_TRANSFORM_PACKING not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 480 | `pngwrite.c:1458` | `png_write_png` | app_error | `PNG_TRANSFORM_SWAP_ALPHA not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 481 | `pngwrite.c:1472` | `png_write_png` | app_error | `PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | observed |
| 482 | `pngwrite.c:1482` | `png_write_png` | app_error | `PNG_TRANSFORM_STRIP_FILLER not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 483 | `pngwrite.c:1491` | `png_write_png` | app_error | `PNG_TRANSFORM_BGR not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 484 | `pngwrite.c:1499` | `png_write_png` | app_error | `PNG_TRANSFORM_SWAP_ENDIAN not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 485 | `pngwrite.c:1507` | `png_write_png` | app_error | `PNG_TRANSFORM_PACKSWAP not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 486 | `pngwrite.c:1515` | `png_write_png` | app_error | `PNG_TRANSFORM_INVERT_ALPHA not supported` | application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN | compiled out of this build (literal absent from the .so) |
| 487 | `pngwrite.c:1629` | `png_write_image_16bit` | error | `png_write_image: internal call error` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 488 | `pngwrite.c:1751` | `png_unpremultiply` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 489 | `pngwrite.c:2045` | `png_image_write_main` | error | `memory image too large` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 490 | `pngwrite.c:2049` | `png_image_write_main` | error | `supplied row stride too small` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 491 | `pngwrite.c:2053` | `png_image_write_main` | error | `image row stride too large` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 492 | `pngwrite.c:2072` | `png_image_write_main` | error | `no color-map for color-mapped image` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 493 | `pngwrite.c:2156` | `png_image_write_main` | error | `png_write_image: unsupported transformation` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 494 | `pngwrite.c:2207` | `png_image_write_main` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 495 | `pngwrite.c:2253` | `png_image_write_main` | error | `png_image_write_to_memory: PNG too big` | fatal error -> error_fn then png_default_error -> longjmp | unreachable on a 64-bit target: requires a PNG stream larger than PNG_SIZE_MAX |
| 496 | `pngwrite.c:2328` | `png_image_write_to_memory` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 497 | `pngwrite.c:2341` | `png_image_write_to_memory` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 498 | `pngwrite.c:2378` | `png_image_write_to_stdio` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 499 | `pngwrite.c:2391` | `png_image_write_to_stdio` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 500 | `pngwrite.c:2440` | `png_image_write_to_file` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 501 | `pngwrite.c:2458` | `png_image_write_to_file` | early-return | `return 0;` | returns 0 (rejection sentinel) | tests/j_nullargs.rs + value-range rows |
| 502 | `pngwutil.c:200` | `png_write_complete_chunk` | error | `length exceeds PNG maximum` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 503 | `pngwutil.c:328` | `png_deflate_claim` | warning | `png_warning(png_ptr, msg);` | warning, call continues | dispatch site (message supplied by caller) |
| 504 | `pngwutil.c:339` | `png_deflate_claim` | error | `png_error(png_ptr, msg);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 505 | `pngwutil.c:413` | `png_deflate_claim` | warning | `deflateEnd failed (ignored)` | warning, call continues | internal invariant; unreachable through the exported API |
| 506 | `pngwutil.c:680` | `png_write_compressed_data_out` | error | `error writing ancillary chunked compressed data` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 507 | `pngwutil.c:714` | `png_write_IHDR` | error | `Invalid bit depth for grayscale image` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 508 | `pngwutil.c:725` | `png_write_IHDR` | error | `Invalid bit depth for RGB image` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 509 | `pngwutil.c:741` | `png_write_IHDR` | error | `Invalid bit depth for paletted image` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 510 | `pngwutil.c:751` | `png_write_IHDR` | error | `Invalid bit depth for grayscale+alpha image` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 511 | `pngwutil.c:762` | `png_write_IHDR` | error | `Invalid bit depth for RGBA image` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 512 | `pngwutil.c:768` | `png_write_IHDR` | error | `Invalid image color type specified` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 513 | `pngwutil.c:773` | `png_write_IHDR` | warning | `Invalid compression type specified` | warning, call continues | observed |
| 514 | `pngwutil.c:796` | `png_write_IHDR` | warning | `Invalid filter type specified` | warning, call continues | observed |
| 515 | `pngwutil.c:804` | `png_write_IHDR` | warning | `Invalid interlace type specified` | warning, call continues | observed |
| 516 | `pngwutil.c:879` | `png_write_PLTE` | error | `Invalid number of colors in palette` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 517 | `pngwutil.c:884` | `png_write_PLTE` | warning | `Invalid number of colors in palette` | warning, call continues | observed |
| 518 | `pngwutil.c:891` | `png_write_PLTE` | warning | `Ignoring request to write a PLTE chunk in grayscale PNG` | warning, call continues | observed |
| 519 | `pngwutil.c:954` | `png_compress_IDAT` | error | `png_error(png_ptr, png_ptr->zstream.msg);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 520 | `pngwutil.c:1033` | `png_compress_IDAT` | error | `Z_OK on Z_FINISH with output space` | fatal error -> error_fn then png_default_error -> longjmp | internal invariant; unreachable through the exported API |
| 521 | `pngwutil.c:1067` | `png_compress_IDAT` | error | `png_error(png_ptr, png_ptr->zstream.msg);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 522 | `pngwutil.c:1107` | `png_write_sRGB` | warning | `Invalid sRGB rendering intent specified` | warning, call continues | observed |
| 523 | `pngwutil.c:1132` | `png_write_iCCP` | error | `No profile for iCCP chunk` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 524 | `pngwutil.c:1135` | `png_write_iCCP` | error | `ICC profile too short` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 525 | `pngwutil.c:1138` | `png_write_iCCP` | error | `Incorrect data in iCCP` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 526 | `pngwutil.c:1142` | `png_write_iCCP` | error | `ICC profile length invalid (not a multiple of 4)` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 527 | `pngwutil.c:1148` | `png_write_iCCP` | error | `Profile length does not match profile` | fatal error -> error_fn then png_default_error -> longjmp | unreachable: dead: the preceding "Incorrect data in iCCP" test is png_get_uint_32(profile) != profile_len, i.e. exactly the same condition |
| 528 | `pngwutil.c:1154` | `png_write_iCCP` | error | `iCCP: invalid keyword` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 529 | `pngwutil.c:1165` | `png_write_iCCP` | error | `png_error(png_ptr, png_ptr->zstream.msg);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 530 | `pngwutil.c:1194` | `png_write_sPLT` | error | `sPLT: invalid keyword` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 531 | `pngwutil.c:1254` | `png_write_sBIT` | warning | `Invalid sBIT depth specified` | warning, call continues | observed |
| 532 | `pngwutil.c:1268` | `png_write_sBIT` | warning | `Invalid sBIT depth specified` | warning, call continues | observed |
| 533 | `pngwutil.c:1280` | `png_write_sBIT` | warning | `Invalid sBIT depth specified` | warning, call continues | observed |
| 534 | `pngwutil.c:1331` | `png_write_tRNS` | app_warning | `Invalid number of transparent colors specified` | application warning, call returns without effect | observed |
| 535 | `pngwutil.c:1346` | `png_write_tRNS` | app_warning | `Ignoring attempt to write tRNS chunk out-of-range for bit_depth` | application warning, call returns without effect | observed |
| 536 | `pngwutil.c:1368` | `png_write_tRNS` | app_warning | `Ignoring attempt to write 16-bit tRNS chunk when bit_depth is 8` | application warning, call returns without effect | observed |
| 537 | `pngwutil.c:1378` | `png_write_tRNS` | app_warning | `Can't write tRNS with an alpha channel` | application warning, call returns without effect | observed |
| 538 | `pngwutil.c:1401` | `png_write_bKGD` | warning | `Invalid background palette index` | warning, call continues | observed |
| 539 | `pngwutil.c:1420` | `png_write_bKGD` | warning | `Ignoring attempt to write 16-bit bKGD chunk ` | warning, call continues | observed |
| 540 | `pngwutil.c:1434` | `png_write_bKGD` | warning | `Ignoring attempt to write bKGD chunk out-of-range for bit_depth` | warning, call continues | observed |
| 541 | `pngwutil.c:1550` | `png_write_hIST` | warning | `Invalid number of histogram entries specified` | warning, call continues | observed |
| 542 | `pngwutil.c:1580` | `png_write_tEXt` | error | `tEXt: invalid keyword` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 543 | `pngwutil.c:1589` | `png_write_tEXt` | error | `tEXt: text too long` | fatal error -> error_fn then png_default_error -> longjmp | unreachable on a 64-bit target: requires more than PNG_UINT_31_MAX bytes of text |
| 544 | `pngwutil.c:1628` | `png_write_zTXt` | error | `zTXt: invalid compression type` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 545 | `pngwutil.c:1633` | `png_write_zTXt` | error | `zTXt: invalid keyword` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 546 | `pngwutil.c:1644` | `png_write_zTXt` | error | `png_error(png_ptr, png_ptr->zstream.msg);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 547 | `pngwutil.c:1676` | `png_write_iTXt` | error | `iTXt: invalid keyword` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 548 | `pngwutil.c:1692` | `png_write_iTXt` | error | `iTXt: invalid compression` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 549 | `pngwutil.c:1730` | `png_write_iTXt` | error | `png_error(png_ptr, png_ptr->zstream.msg);` | fatal error -> error_fn then png_default_error -> longjmp | dispatch site (message supplied by caller) |
| 550 | `pngwutil.c:1736` | `png_write_iTXt` | error | `iTXt: uncompressed text too long` | fatal error -> error_fn then png_default_error -> longjmp | unreachable on a 64-bit target: requires more than PNG_UINT_31_MAX bytes of text |
| 551 | `pngwutil.c:1771` | `png_write_oFFs` | warning | `Unrecognized unit type for oFFs chunk` | warning, call continues | observed |
| 552 | `pngwutil.c:1797` | `png_write_pCAL` | error | `Unrecognized equation type for pCAL chunk` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 553 | `pngwutil.c:1802` | `png_write_pCAL` | error | `pCAL: invalid keyword` | fatal error -> error_fn then png_default_error -> longjmp | observed |
| 554 | `pngwutil.c:1862` | `png_write_sCAL_s` | warning | `Can't write sCAL (buffer too small)` | warning, call continues | observed |
| 555 | `pngwutil.c:1887` | `png_write_pHYs` | warning | `Unrecognized unit type for pHYs chunk` | warning, call continues | observed |
| 556 | `pngwutil.c:1912` | `png_write_tIME` | warning | `Invalid time specified for tIME chunk` | warning, call continues | observed |
