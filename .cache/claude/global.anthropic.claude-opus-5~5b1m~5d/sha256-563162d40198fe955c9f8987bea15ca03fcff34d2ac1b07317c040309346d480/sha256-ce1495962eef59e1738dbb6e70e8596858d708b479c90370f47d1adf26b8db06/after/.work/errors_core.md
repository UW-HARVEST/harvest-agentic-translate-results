| NNN | `png_error` (`pngerror.c:39`) | called with any `error_message`; `png_ptr != NULL && png_ptr->error_fn != NULL` | calls `(*error_fn)(png_ptr, error_message)`; if it returns, falls through to `png_default_error` -> `png_longjmp(png_ptr, 1)` -> non-zero `setjmp`; declared `PNG_NORETURN` |
| NNN | `png_error` (`pngerror.c:42`) | `png_ptr == NULL` or `png_ptr->error_fn == NULL` | skips custom handler, calls `png_default_error(png_ptr, error_message)` -> `png_longjmp(png_ptr,1)`; never returns |
| NNN | `png_safecat` (`pngerror.c:76`) | `buffer == NULL` | returns `pos` unchanged, copies nothing (silent no-op) |
| NNN | `png_safecat` (`pngerror.c:76`) | `pos >= bufsize` (destination already full/overflowed) | returns `pos` unchanged, writes nothing, does not even NUL-terminate |
| NNN | `png_safecat` (`pngerror.c:78`) | `string == NULL` | copies nothing, still writes `buffer[pos] = '\0'`, returns `pos` |
| NNN | `png_safecat` (`pngerror.c:79`) | `string` longer than `bufsize-1-pos` | silently truncates at `bufsize-1` and NUL-terminates |
| NNN | `png_format_number` (`pngerror.c:144`) | `format` is not one of `PNG_NUMBER_FORMAT_fixed`/`_02u`/`_u`/`_02x`/`_x` | `default:` sets `number = 0`, loop terminates; returns pointer to a buffer containing only the NUL/partial output |
| NNN | `png_format_number` (`pngerror.c:106`) | `end <= start` on entry (zero-size buffer window) | loop body never runs; returns `end` pointing at the written `'\0'` |
| NNN | `png_warning` (`pngerror.c:180`) | `png_ptr == NULL` or `png_ptr->warning_fn == NULL` | calls `png_default_warning` -> `fprintf(stderr, "libpng warning: %s", warning_message)` + `PNG_STRING_NEWLINE`; returns void (non-fatal) |
| NNN | `png_warning_parameter` (`pngerror.c:196`) | `number <= 0` | condition `number > 0` false: parameter silently discarded, nothing stored in `p[]` |
| NNN | `png_warning_parameter` (`pngerror.c:196`) | `number > PNG_WARNING_PARAMETER_COUNT` (i.e. `> 8`) | parameter silently discarded, nothing stored in `p[]` |
| NNN | `png_formatted_warning` (`pngerror.c:247`) | formatted message would exceed 191 bytes | silently truncated to `(sizeof msg)-1 == 191` chars then `msg[i]='\0'`; `png_warning(png_ptr, msg)` |
| NNN | `png_formatted_warning` (`pngerror.c:252`) | `'@'` is the last character of `message` (`message[1] == '\0'`) | the lone `'@'` is copied literally instead of being treated as a parameter introducer |
| NNN | `png_formatted_warning` (`pngerror.c:266`) | `@<digit>` where digit is not in `"12345678"` (parameter index >= `PNG_WARNING_PARAMETER_COUNT`) | not treated as a parameter; the character after `@` is copied literally |
| NNN | `png_benign_error` (`pngerror.c:310`) | `(png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0`, read struct (`mode & PNG_IS_READ_STRUCT`) and `chunk_name != 0` | downgraded to `png_chunk_warning(png_ptr, error_message)`; returns normally |
| NNN | `png_benign_error` (`pngerror.c:318`) | `PNG_FLAG_BENIGN_ERRORS_WARN` set but not a read struct, or `chunk_name == 0` | downgraded to `png_warning(png_ptr, error_message)`; returns normally |
| NNN | `png_benign_error` (`pngerror.c:326`) | `PNG_FLAG_BENIGN_ERRORS_WARN` clear, read struct and `chunk_name != 0` | `png_chunk_error(png_ptr, error_message)` -> `png_error` -> longjmp / non-zero `setjmp` |
| NNN | `png_benign_error` (`pngerror.c:329`) | `PNG_FLAG_BENIGN_ERRORS_WARN` clear and (not read struct or `chunk_name == 0`) | `png_error(png_ptr, error_message)` -> longjmp / non-zero `setjmp` |
| NNN | `png_app_warning` (`pngerror.c:340`) | `(png_ptr->flags & PNG_FLAG_APP_WARNINGS_WARN) != 0` | `png_warning(png_ptr, error_message)`, returns normally |
| NNN | `png_app_warning` (`pngerror.c:343`) | `PNG_FLAG_APP_WARNINGS_WARN` clear (default) | `png_error(png_ptr, error_message)` -> longjmp / non-zero `setjmp` |
| NNN | `png_app_error` (`pngerror.c:353`) | `(png_ptr->flags & PNG_FLAG_APP_ERRORS_WARN) != 0` | `png_warning(png_ptr, error_message)`, returns normally |
| NNN | `png_app_error` (`pngerror.c:356`) | `PNG_FLAG_APP_ERRORS_WARN` clear (default) | `png_error(png_ptr, error_message)` -> longjmp / non-zero `setjmp` |
| NNN | `png_format_buffer` (`pngerror.c:391`) | any byte of `png_ptr->chunk_name` fails `isnonalpha(c)` i.e. `c < 65 \|\| c > 122 \|\| (c > 90 && c < 97)` | that byte is emitted as `[HH]` (two uppercase hex digits inside square brackets) in the message prefix |
| NNN | `png_format_buffer` (`pngerror.c:415`) | `error_message` longer than `PNG_MAX_ERROR_TEXT-1` (195 bytes) | message truncated to 195 bytes then NUL-terminated |
| NNN | `png_format_buffer` (`pngerror.c:405`) | `error_message == NULL` | buffer contains only the 4-byte (or bracketed) chunk name plus `'\0'`; no `": "` separator |
| NNN | `png_chunk_error` (`pngerror.c:430`) | `png_ptr == NULL` | `png_error(NULL, error_message)` -> `png_default_error` -> `png_longjmp(NULL,1)` -> `PNG_ABORT()` (process abort) |
| NNN | `png_chunk_error` (`pngerror.c:436`) | `png_ptr != NULL` | `png_error(png_ptr, "<chunkname>: <error_message>")` -> longjmp / non-zero `setjmp`; declared `PNG_NORETURN` |
| NNN | `png_chunk_warning` (`pngerror.c:446`) | `png_ptr == NULL` | `png_warning(NULL, warning_message)` -> default warning to stderr, no chunk-name prefix |
| NNN | `png_chunk_benign_error` (`pngerror.c:463`) | `(png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0` | `png_chunk_warning(png_ptr, error_message)`; returns normally |
| NNN | `png_chunk_benign_error` (`pngerror.c:467`) | `PNG_FLAG_BENIGN_ERRORS_WARN` clear | `png_chunk_error(png_ptr, error_message)` -> longjmp / non-zero `setjmp` |
| NNN | `png_chunk_report` (`pngerror.c:492`) | read struct (`mode & PNG_IS_READ_STRUCT`) and `error < PNG_CHUNK_ERROR` (i.e. `error` is 0 or 1) | `png_chunk_warning(png_ptr, message)`; returns normally |
| NNN | `png_chunk_report` (`pngerror.c:496`) | read struct and `error >= PNG_CHUNK_ERROR` (i.e. `error >= 2`) | `png_chunk_benign_error(png_ptr, message)` (warning or longjmp per `PNG_FLAG_BENIGN_ERRORS_WARN`) |
| NNN | `png_chunk_report` (`pngerror.c:506`) | write struct (`(mode & PNG_IS_READ_STRUCT) == 0`) and `error < PNG_CHUNK_WRITE_ERROR` (i.e. `error == 0`) | `png_app_warning(png_ptr, message)` |
| NNN | `png_chunk_report` (`pngerror.c:510`) | write struct and `error >= PNG_CHUNK_WRITE_ERROR` (i.e. `error >= 1`) | `png_app_error(png_ptr, message)` |
| NNN | `png_fixed_error` (`pngerror.c:534`) | called with any `name` | `png_error(png_ptr, "fixed point overflow in <name>")` -> longjmp / non-zero `setjmp`; `PNG_NORETURN` |
| NNN | `png_fixed_error` (`pngerror.c:528`) | `name` longer than `PNG_MAX_ERROR_TEXT-1` (195) | name truncated to 195 bytes in the `"fixed point overflow in "` message |
| NNN | `png_fixed_error` (`pngerror.c:527`) | `name == NULL` | message is exactly `"fixed point overflow in "` then `png_error` |
| NNN | `png_set_longjmp_fn` (`pngerror.c:557`) | `png_ptr == NULL` | returns NULL |
| NNN | `png_set_longjmp_fn` (`pngerror.c:572`) | first call, `jmp_buf_size > sizeof png_ptr->jmp_buf_local`, and `png_malloc_warn` returns NULL (OOM) | `png_warning(png_ptr, "Out of memory")` from `png_malloc_warn`, then returns NULL |
| NNN | `png_set_longjmp_fn` (`pngerror.c:593`) | `jmp_buf_ptr != NULL`, recorded `jmp_buf_size == 0`, but `jmp_buf_ptr != &png_ptr->jmp_buf_local` | `png_error(png_ptr, "Libpng jmp_buf still allocated")` -> longjmp / non-zero `setjmp` |
| NNN | `png_set_longjmp_fn` (`pngerror.c:598`) | second call with `jmp_buf_size` differing from the previously recorded size | `png_warning(png_ptr, "Application jmp_buf size changed")` and returns NULL |
| NNN | `png_free_jmpbuf` (`pngerror.c:615`) | `png_ptr == NULL` | returns without touching anything |
| NNN | `png_free_jmpbuf` (`pngerror.c:634`) | `png_free` of the jmp_buf triggers `png_error` (user free_fn failure) | caught by local `setjmp(free_jmp_buf)`; error ignored, `jmp_buf_size`/`jmp_buf_ptr`/`longjmp_fn` still zeroed |
| NNN | `png_default_error` (`pngerror.c:662`) | `error_message == NULL` | prints `"libpng error: undefined"` to stderr then `png_longjmp(png_ptr, 1)` |
| NNN | `png_default_error` (`pngerror.c:668`) | any error | `fprintf(stderr, "libpng error: %s", error_message)` + newline, then `png_longjmp(png_ptr, 1)`; `PNG_NORETURN` |
| NNN | `png_longjmp` (`pngerror.c:676`) | `png_ptr == NULL`, or `png_ptr->longjmp_fn == NULL`, or `png_ptr->jmp_buf_ptr == NULL` (no `png_set_longjmp_fn`/`png_setjmp` done) | falls through to `PNG_ABORT()` — calls `abort()`, terminating the process |
| NNN | `png_longjmp` (`pngerror.c:678`) | valid jmp_buf and `longjmp_fn` set | `longjmp_fn(*jmp_buf_ptr, val)`; `setjmp` returns `val` (1 from `png_default_error`) |
| NNN | `png_set_error_fn` (`pngerror.c:721`) | `png_ptr == NULL` | returns without storing `error_ptr`/`error_fn`/`warning_fn` |
| NNN | `png_get_error_ptr` (`pngerror.c:741`) | `png_ptr == NULL` | returns NULL |
| NNN | `png_safe_error` (`pngerror.c:773`) | `png_ptr->error_ptr` (the `png_imagep`) is NULL | skips all logging and falls straight to `abort()` |
| NNN | `png_safe_error` (`pngerror.c:782`) | `image->opaque == NULL` or `image->opaque->error_buf == NULL` (no active `png_safe_execute`) | writes `"bad longjmp: <error_message>"` into `image->message` then `abort()` |
| NNN | `png_safe_error` (`pngerror.c:776`) | any simplified-API error | `image->message` = error text, `image->warning_or_error \|= PNG_IMAGE_ERROR`, `longjmp(png_control_jmp_buf(image->opaque), 1)` |
| NNN | `png_safe_warning` (`pngerror.c:806`) | `image->warning_or_error != 0` (a prior warning or error already logged) | new warning silently dropped, `image->message` left unchanged |
| NNN | `png_safe_execute` (`pngerror.c:821`) | `function(arg)` (or anything below it) calls `png_error` -> `png_safe_error` -> `longjmp` | `setjmp` returns non-zero, `image->opaque->error_buf` restored, returns 0 (failure) |
| NNN | `png_safe_execute` (`pngerror.c:829`) | `function(arg)` returns 0 (false) without longjmp | falls through to the failure path and returns 0 |
| NNN | `png_safe_execute` (`pngerror.c:844`) | failure path with `saved_error_buf == NULL` (outermost call) | `png_image_free(image)` is called before returning 0 |
| NNN | `png_destroy_png_struct` (`pngmem.c:26`) | `png_ptr == NULL` | returns, frees nothing |
| NNN | `png_calloc` (`pngmem.c:56`) | `png_malloc` returned NULL (only possible when `png_ptr == NULL`) | returns NULL without `memset` |
| NNN | `png_malloc_base` (`pngmem.c:88`) | `size > PNG_SIZE_MAX` (i.e. `size > (size_t)-1`, only reachable when `png_alloc_size_t` is wider than `size_t`) | returns NULL (no error, no warning) |
| NNN | `png_malloc_base` (`pngmem.c:83`) | `PNG_MAX_MALLOC_64K` builds only (NOT this config): `size > 65536U` | returns NULL |
| NNN | `png_malloc_base` (`pngmem.c:98`) | system `malloc` fails | returns NULL; caller decides whether that is fatal |
| NNN | `png_malloc_array_checked` (`pngmem.c:113`) | `nelements > PNG_SIZE_MAX/element_size` (multiplication would overflow `size_t`) | returns NULL |
| NNN | `png_malloc_array` (`pngmem.c:125`) | `nelements <= 0` | `png_error(png_ptr, "internal error: array alloc")` -> longjmp / non-zero `setjmp` |
| NNN | `png_malloc_array` (`pngmem.c:125`) | `element_size == 0` | `png_error(png_ptr, "internal error: array alloc")` -> longjmp / non-zero `setjmp` |
| NNN | `png_realloc_array` (`pngmem.c:137`) | `add_elements <= 0` | `png_error(png_ptr, "internal error: array realloc")` -> longjmp |
| NNN | `png_realloc_array` (`pngmem.c:137`) | `element_size == 0` | `png_error(png_ptr, "internal error: array realloc")` -> longjmp |
| NNN | `png_realloc_array` (`pngmem.c:137`) | `old_elements < 0` | `png_error(png_ptr, "internal error: array realloc")` -> longjmp |
| NNN | `png_realloc_array` (`pngmem.c:138`) | `old_array == NULL && old_elements > 0` | `png_error(png_ptr, "internal error: array realloc")` -> longjmp |
| NNN | `png_realloc_array` (`pngmem.c:144`) | `add_elements > INT_MAX - old_elements` (element count would overflow `int`) | skips the allocation and returns NULL |
| NNN | `png_realloc_array` (`pngmem.c:164`) | `png_malloc_array_checked` returned NULL (size overflow or OOM) | returns NULL, old array untouched |
| NNN | `png_malloc` (`pngmem.c:178`) | `png_ptr == NULL` | returns NULL |
| NNN | `png_malloc` (`pngmem.c:183`) | `png_malloc_base` returned NULL (OOM or `size > PNG_SIZE_MAX`) | `png_error(png_ptr, "Out of memory")` -> longjmp / non-zero `setjmp` |
| NNN | `png_malloc_default` (`pngmem.c:196`) | `png_ptr == NULL` | returns NULL |
| NNN | `png_malloc_default` (`pngmem.c:202`) | system `malloc` (bypassing user handler) returned NULL | `png_error(png_ptr, "Out of Memory")` (capital `M`) -> longjmp |
| NNN | `png_malloc_warn` (`pngmem.c:217`) | `png_ptr == NULL` | skips the whole block and returns NULL |
| NNN | `png_malloc_warn` (`pngmem.c:224`) | `png_malloc_base` returned NULL | `png_warning(png_ptr, "Out of memory")` then returns NULL (never longjmps) |
| NNN | `png_free` (`pngmem.c:236`) | `png_ptr == NULL` | returns; memory is leaked, no error |
| NNN | `png_free` (`pngmem.c:236`) | `ptr == NULL` | returns without calling `free`/`free_fn` |
| NNN | `png_free_default` (`pngmem.c:251`) | `png_ptr == NULL \|\| ptr == NULL` | returns without calling `free` |
| NNN | `png_set_mem_fn` (`pngmem.c:266`) | `png_ptr == NULL` | returns, stores nothing |
| NNN | `png_get_mem_ptr` (`pngmem.c:281`) | `png_ptr == NULL` | returns NULL |
| NNN | `png_read_data` (`pngrio.c:35`) | `png_ptr->read_data_fn == NULL` (no `png_set_read_fn`/`png_init_io` done) | `png_error(png_ptr, "Call to NULL read function")` -> longjmp / non-zero `setjmp` |
| NNN | `png_default_read_data` (`pngrio.c:53`) | `png_ptr == NULL` | returns without reading; `data` left uninitialized |
| NNN | `png_default_read_data` (`pngrio.c:61`) | `fread(data,1,length,io_ptr) != length` (short read / EOF / IO error / truncated file) | `png_error(png_ptr, "Read Error")` -> longjmp / non-zero `setjmp` |
| NNN | `png_set_read_fn` (`pngrio.c:89`) | `png_ptr == NULL` | returns, nothing set |
| NNN | `png_set_read_fn` (`pngrio.c:95`) | `read_data_fn == NULL` | silently substitutes `png_default_read_data` (STDIO build) |
| NNN | `png_set_read_fn` (`pngrio.c:106`) | `png_ptr->write_data_fn != NULL` (struct already configured for writing) | clears `write_data_fn` and `png_warning(png_ptr, "Can't set both read_data_fn and write_data_fn in the same structure")` |
| NNN | `png_write_data` (`pngwio.c:35`) | `png_ptr->write_data_fn == NULL` | `png_error(png_ptr, "Call to NULL write function")` -> longjmp / non-zero `setjmp` |
| NNN | `png_default_write_data` (`pngwio.c:54`) | `png_ptr == NULL` | returns without writing |
| NNN | `png_default_write_data` (`pngwio.c:59`) | `fwrite(data,1,length,io_ptr) != length` (disk full / IO error) | `png_error(png_ptr, "Write Error")` -> longjmp / non-zero `setjmp` |
| NNN | `png_flush` (`pngwio.c:72`) | `png_ptr->output_flush_fn == NULL` | silent no-op, nothing flushed |
| NNN | `png_default_flush` (`pngwio.c:82`) | `png_ptr == NULL` | returns without `fflush` |
| NNN | `png_set_write_fn` (`pngwio.c:124`) | `png_ptr == NULL` | returns, nothing set |
| NNN | `png_set_write_fn` (`pngwio.c:130`) | `write_data_fn == NULL` | silently substitutes `png_default_write_data` (STDIO build) |
| NNN | `png_set_write_fn` (`pngwio.c:142`) | `output_flush_fn == NULL` | silently substitutes `png_default_flush` |
| NNN | `png_set_write_fn` (`pngwio.c:157`) | `png_ptr->read_data_fn != NULL` (struct already configured for reading) | clears `read_data_fn` and `png_warning(png_ptr, "Can't set both read_data_fn and write_data_fn in the same structure")` |
| NNN | `png_set_bgr` (`pngtrans.c:24`) | `png_ptr == NULL` | returns; `PNG_BGR` not set |
| NNN | `png_set_swap` (`pngtrans.c:38`) | `png_ptr == NULL` | returns; `PNG_SWAP_BYTES` not set |
| NNN | `png_set_swap` (`pngtrans.c:41`) | `png_ptr->bit_depth != 16` | silent no-op — `PNG_SWAP_BYTES` NOT set, no warning |
| NNN | `png_set_packing` (`pngtrans.c:52`) | `png_ptr == NULL` | returns; `PNG_PACK` not set |
| NNN | `png_set_packing` (`pngtrans.c:56`) | `png_ptr->bit_depth >= 8` | silent no-op — `PNG_PACK` NOT set and `usr_bit_depth` unchanged |
| NNN | `png_set_packswap` (`pngtrans.c:73`) | `png_ptr == NULL` | returns; `PNG_PACKSWAP` not set |
| NNN | `png_set_packswap` (`pngtrans.c:76`) | `png_ptr->bit_depth >= 8` | silent no-op — `PNG_PACKSWAP` NOT set |
| NNN | `png_set_shift` (`pngtrans.c:87`) | `png_ptr == NULL` | returns |
| NNN | `png_set_shift` (`pngtrans.c:87`) | `true_bits == NULL` | returns; no transformation set |
| NNN | `png_set_shift` (`pngtrans.c:97`) | color image (`color_type & PNG_COLOR_MASK_COLOR`) and `true_bits->red == 0` or `true_bits->red > png_ptr->bit_depth` | `invalid = 1` -> `png_app_error(png_ptr, "png_set_shift: invalid shift values")` and return |
| NNN | `png_set_shift` (`pngtrans.c:98`) | color image and `true_bits->green == 0` or `> bit_depth` | `png_app_error(png_ptr, "png_set_shift: invalid shift values")` and return |
| NNN | `png_set_shift` (`pngtrans.c:99`) | color image and `true_bits->blue == 0` or `> bit_depth` | `png_app_error(png_ptr, "png_set_shift: invalid shift values")` and return |
| NNN | `png_set_shift` (`pngtrans.c:104`) | grayscale image and `true_bits->gray == 0` or `> bit_depth` | `png_app_error(png_ptr, "png_set_shift: invalid shift values")` and return |
| NNN | `png_set_shift` (`pngtrans.c:108`) | `color_type & PNG_COLOR_MASK_ALPHA` and `true_bits->alpha == 0` or `> bit_depth` | `png_app_error(png_ptr, "png_set_shift: invalid shift values")` and return |
| NNN | `png_set_shift` (`pngtrans.c:114`) | any of the above (`invalid != 0`) | `png_app_error` (longjmp unless `PNG_FLAG_APP_ERRORS_WARN`), then `return` — `PNG_SHIFT` and `png_ptr->shift` left unchanged |
| NNN | `png_set_interlace_handling` (`pngtrans.c:131`) | `png_ptr == 0` | returns 1 (claims a single pass) — no error reported |
| NNN | `png_set_interlace_handling` (`pngtrans.c:131`) | `png_ptr->interlaced == 0` | returns 1 and does NOT set `PNG_INTERLACE` |
| NNN | `png_set_filler` (`pngtrans.c:152`) | `png_ptr == NULL` | returns |
| NNN | `png_set_filler` (`pngtrans.c:171`) | non-`PNG_READ_FILLER_SUPPORTED` builds (NOT this config), read struct | `png_app_error(png_ptr, "png_set_filler not supported on read")` then return |
| NNN | `png_set_filler` (`pngtrans.c:202`) | write struct, `color_type == PNG_COLOR_TYPE_GRAY` and `bit_depth < 8` | `png_app_error(png_ptr, "png_set_filler is invalid for low bit depth gray output")` then return; `PNG_FILLER` not set |
| NNN | `png_set_filler` (`pngtrans.c:209`) | write struct, `color_type` is not `PNG_COLOR_TYPE_RGB` and not `PNG_COLOR_TYPE_GRAY` (i.e. PALETTE=3, GRAY_ALPHA=4, RGB_ALPHA=6) | `png_app_error(png_ptr, "png_set_filler: inappropriate color type")` then return |
| NNN | `png_set_filler` (`pngtrans.c:214`) | non-`PNG_WRITE_FILLER_SUPPORTED` builds (NOT this config), write struct | `png_app_error(png_ptr, "png_set_filler not supported on write")` then return |
| NNN | `png_set_filler` (`pngtrans.c:224`) | `filler_loc != PNG_FILLER_AFTER` | `PNG_FLAG_FILLER_AFTER` cleared (filler placed before) — no validation of `filler_loc` values |
| NNN | `png_set_add_alpha` (`pngtrans.c:237`) | `png_ptr == NULL` | returns |
| NNN | `png_set_add_alpha` (`pngtrans.c:242`) | inner `png_set_filler` failed, so `(transformations & PNG_FILLER) == 0` | `PNG_ADD_ALPHA` silently not set (the `png_app_error` from `png_set_filler` is the only diagnostic) |
| NNN | `png_set_swap_alpha` (`pngtrans.c:255`) | `png_ptr == NULL` | returns |
| NNN | `png_set_invert_alpha` (`pngtrans.c:269`) | `png_ptr == NULL` | returns |
| NNN | `png_set_invert_mono` (`pngtrans.c:281`) | `png_ptr == NULL` | returns |
| NNN | `png_do_invert` (`pngtrans.c:297`) | `row_info->color_type` is neither `PNG_COLOR_TYPE_GRAY` nor `PNG_COLOR_TYPE_GRAY_ALPHA` | all branches fail: row left completely unmodified, no diagnostic |
| NNN | `png_do_invert` (`pngtrans.c:310`) | `color_type == PNG_COLOR_TYPE_GRAY_ALPHA` and `bit_depth` is neither 8 nor 16 | no branch matches: row unmodified |
| NNN | `png_do_swap` (`pngtrans.c:351`) | `row_info->bit_depth != 16` | silent no-op, row unmodified |
| NNN | `png_do_packswap` (`pngtrans.c:487`) | `row_info->bit_depth >= 8` | silent no-op, row unmodified |
| NNN | `png_do_packswap` (`pngtrans.c:502`) | `bit_depth < 8` but not 1, 2 or 4 (e.g. 3, 5, 6, 7) | `else return;` — row unmodified, no table lookup |
| NNN | `png_do_strip_channel` (`pngtrans.c:576`) | `row_info->channels == 2` and `bit_depth` neither 8 nor 16 | `return;` ("bad bit depth") — `channels`, `pixel_depth`, `rowbytes`, `color_type` all left unchanged |
| NNN | `png_do_strip_channel` (`pngtrans.c:627`) | `row_info->channels == 4` and `bit_depth` neither 8 nor 16 | `return;` ("bad bit depth") — row_info left unchanged |
| NNN | `png_do_strip_channel` (`pngtrans.c:637`) | `row_info->channels` is neither 2 nor 4 (1 or 3 — filler already gone) | `return;` — `rowbytes` not recomputed |
| NNN | `png_do_check_palette_indexes` (`pngtrans.c:732`) | `png_ptr->num_palette >= (1 << row_info->bit_depth)` | whole check skipped; `num_palette_max` not updated (no out-of-range index detection possible) |
| NNN | `png_do_check_palette_indexes` (`pngtrans.c:733`) | `png_ptr->num_palette <= 0` (0 is legal in MNG) | whole check skipped |
| NNN | `png_do_check_palette_indexes` (`pngtrans.c:822`) | `row_info->bit_depth` not in {1,2,4,8} | `default: break;` — no index scanning performed |
| NNN | `png_set_user_transform_info` (`pngtrans.c:838`) | `png_ptr == NULL` | returns |
| NNN | `png_set_user_transform_info` (`pngtrans.c:842`) | read struct (`mode & PNG_IS_READ_STRUCT`) and `(flags & PNG_FLAG_ROW_INIT) != 0` (called after `png_start_read_image`/`png_read_update_info`) | `png_app_error(png_ptr, "info change after png_start_read_image or png_read_update_info")` then return |
| NNN | `png_get_user_transform_ptr` (`pngtrans.c:866`) | `png_ptr == NULL` | returns NULL |
| NNN | `png_get_current_row_number` (`pngtrans.c:883`) | `png_ptr == NULL` | returns `PNG_UINT_32_MAX` (0xffffffff) as a deliberate "help the app not to fail silently" sentinel |
| NNN | `png_get_current_pass_number` (`pngtrans.c:891`) | `png_ptr == NULL` | returns 8 (invalid pass number; valid passes are 0..6) |
| NNN | `png_set_sig_bytes` (`png.c:59`) | `png_ptr == NULL` | returns |
| NNN | `png_set_sig_bytes` (`png.c:62`) | `num_bytes < 0` | clamped: `nb = 0` (no error) |
| NNN | `png_set_sig_bytes` (`png.c:65`) | `nb > 8` (i.e. `num_bytes > 8`) | `png_error(png_ptr, "Too many bytes for PNG signature")` -> longjmp / non-zero `setjmp` |
| NNN | `png_sig_cmp` (`png.c:84`) | `num_to_check > 8` | silently clamped to 8 |
| NNN | `png_sig_cmp` (`png.c:87`) | `num_to_check < 1` (i.e. 0) | returns -1 (failure sentinel) |
| NNN | `png_sig_cmp` (`png.c:90`) | `start > 7` | returns -1 (failure sentinel) |
| NNN | `png_sig_cmp` (`png.c:93`) | `start + num_to_check > 8` | `num_to_check` clamped to `8 - start` |
| NNN | `png_sig_cmp` (`png.c:96`) | `sig[start..]` differs from `{137,80,78,71,13,10,26,10}[start..]` | returns the non-zero `memcmp` result (<0 or >0) |
| NNN | `png_zalloc` (`png.c:109`) | `png_ptr == NULL` | returns NULL (zlib treats as allocation failure -> `Z_MEM_ERROR`) |
| NNN | `png_zalloc` (`png.c:118`) | `size != 0 && items >= (~(png_alloc_size_t)0) / size` (`items * size` would overflow) | `png_warning(png_ptr, "Potential overflow in png_zalloc()")` and returns NULL |
| NNN | `png_zalloc` (`png.c:126`) | `png_malloc_warn` fails (OOM) | `png_warning(png_ptr, "Out of memory")` and returns NULL |
| NNN | `png_calculate_crc` (`png.c:158`) | ancillary chunk and `(flags & PNG_FLAG_CRC_ANCILLARY_MASK) == (PNG_FLAG_CRC_ANCILLARY_USE \| PNG_FLAG_CRC_ANCILLARY_NOWARN)` | `need_crc = 0` — CRC not accumulated, so corruption in this chunk will never be detected |
| NNN | `png_calculate_crc` (`png.c:165`) | critical chunk and `(flags & PNG_FLAG_CRC_CRITICAL_IGNORE) != 0` | `need_crc = 0` — CRC deliberately not computed |
| NNN | `png_calculate_crc` (`png.c:182`) | `length` truncates to 0 when cast to `uInt` (length is an exact multiple of `uInt` range) | `safe_length = (uInt)-1` ("evil, but safe") to avoid an infinite loop |
| NNN | `png_user_version_check` (`png.c:213`) | `user_png_ver == NULL` | `png_ptr->flags \|= PNG_FLAG_LIBRARY_MISMATCH` -> warning path below, returns 0 |
| NNN | `png_user_version_check` (`png.c:221`) | any byte of `user_png_ver` up to and including the second `'.'` differs from `PNG_LIBPNG_VER_STRING` ("1.6.59.git") — e.g. app built with "1.5.30" or "1.4.0" | sets `PNG_FLAG_LIBRARY_MISMATCH` |
| NNN | `png_user_version_check` (`png.c:245`) | `PNG_FLAG_LIBRARY_MISMATCH` set | `png_warning(png_ptr, "Application built with libpng-<user_png_ver> but running with 1.6.59.git")` and returns 0 |
| NNN | `png_create_png_struct` (`png.c:314`) | user memory allocator (or `error_fn`) longjmps out during struct creation | `setjmp(create_jmp_buf)` non-zero, body skipped, returns NULL |
| NNN | `png_create_png_struct` (`png.c:329`) | `png_user_version_check(&create_struct, user_png_ver) == 0` (version mismatch or NULL version) | skips allocation and returns NULL (after the version warning) |
| NNN | `png_create_png_struct` (`png.c:334`) | `png_malloc_warn(&create_struct, sizeof(png_struct))` returns NULL (OOM) | `png_warning "Out of memory"` then returns NULL |
| NNN | `png_create_info_struct` (`png.c:373`) | `png_ptr == NULL` | returns NULL |
| NNN | `png_create_info_struct` (`png.c:384`) | `png_malloc_base(png_ptr, sizeof(png_info))` returns NULL (OOM) | returns NULL without erroring (deliberately never longjmps) |
| NNN | `png_destroy_info_struct` (`png.c:405`) | `png_ptr == NULL` | returns; info struct leaked |
| NNN | `png_destroy_info_struct` (`png.c:408`) | `info_ptr_ptr == NULL` | `info_ptr` stays NULL, nothing freed |
| NNN | `png_destroy_info_struct` (`png.c:411`) | `*info_ptr_ptr == NULL` | nothing freed, returns |
| NNN | `png_info_init_3` (`png.c:444`) | `*ptr_ptr == NULL` | returns without touching anything |
| NNN | `png_info_init_3` (`png.c:447`) | `png_info_struct_size < sizeof(png_info)` (app compiled against an older/smaller `png_info`) | `*ptr_ptr = NULL`, the caller's pointer is `free()`d (bypassing user mem fns) and a new struct is `png_malloc_base`'d |
| NNN | `png_info_init_3` (`png.c:454`) | above plus `png_malloc_base(NULL, sizeof(png_info))` returns NULL | `*ptr_ptr` left NULL and the function returns — caller's struct already freed |
| NNN | `png_data_freer` (`png.c:469`) | `png_ptr == NULL` or `info_ptr == NULL` | returns |
| NNN | `png_data_freer` (`png.c:478`) | `freer` is neither `PNG_DESTROY_WILL_FREE_DATA` (1) nor `PNG_USER_WILL_FREE_DATA` (2) | `png_error(png_ptr, "Unknown freer parameter in png_data_freer")` -> longjmp / non-zero `setjmp` |
| NNN | `png_free_data` (`png.c:488`) | `png_ptr == NULL` or `info_ptr == NULL` | returns, frees nothing |
| NNN | `png_get_io_ptr` (`png.c:693`) | `png_ptr == NULL` | returns NULL |
| NNN | `png_init_io` (`png.c:712`) | `png_ptr == NULL` | returns; `io_ptr` not set |
| NNN | `png_convert_to_rfc1123_buffer` (`png.c:748`) | `out == NULL` | returns 0 (failure) |
| NNN | `png_convert_to_rfc1123_buffer` (`png.c:751`) | `ptime->year > 9999` (RFC1123 limitation) | returns 0, buffer untouched |
| NNN | `png_convert_to_rfc1123_buffer` (`png.c:752`) | `ptime->month == 0` | returns 0 |
| NNN | `png_convert_to_rfc1123_buffer` (`png.c:752`) | `ptime->month > 12` | returns 0 |
| NNN | `png_convert_to_rfc1123_buffer` (`png.c:753`) | `ptime->day == 0` | returns 0 |
| NNN | `png_convert_to_rfc1123_buffer` (`png.c:753`) | `ptime->day > 31` | returns 0 |
| NNN | `png_convert_to_rfc1123_buffer` (`png.c:754`) | `ptime->hour > 23` | returns 0 |
| NNN | `png_convert_to_rfc1123_buffer` (`png.c:754`) | `ptime->minute > 59` | returns 0 |
| NNN | `png_convert_to_rfc1123_buffer` (`png.c:755`) | `ptime->second > 60` (60 allowed for leap seconds) | returns 0 |
| NNN | `png_convert_to_rfc1123_buffer` (`png.c:765`) | `pos >= 28` while appending (`APPEND` macro) | character silently dropped; output stays within the 29-byte buffer |
| NNN | `png_convert_to_rfc1123` (`png.c:798`) | `png_ptr == NULL` | returns NULL |
| NNN | `png_convert_to_rfc1123` (`png.c:801`) | `png_convert_to_rfc1123_buffer(...) == 0` (any invalid `ptime` field above) | `png_warning(png_ptr, "Ignoring invalid time value")` and returns NULL |
| NNN | `png_build_grayscale_palette` (`png.c:889`) | `palette == NULL` | returns, writes nothing |
| NNN | `png_build_grayscale_palette` (`png.c:914`) | `bit_depth` not in {1,2,4,8} | `default:` sets `num_palette = 0, color_inc = 0`; loop writes nothing (silent no-op) |
| NNN | `png_handle_as_unknown` (`png.c:936`) | `png_ptr == NULL` | returns `PNG_HANDLE_CHUNK_AS_DEFAULT` (0) |
| NNN | `png_handle_as_unknown` (`png.c:936`) | `chunk_name == NULL` | returns `PNG_HANDLE_CHUNK_AS_DEFAULT` (0) |
| NNN | `png_handle_as_unknown` (`png.c:936`) | `png_ptr->num_chunk_list == 0` (no list set) | returns `PNG_HANDLE_CHUNK_AS_DEFAULT` (0) |
| NNN | `png_handle_as_unknown` (`png.c:960`) | chunk name not found in `png_ptr->chunk_list` | returns `PNG_HANDLE_CHUNK_AS_DEFAULT` (0) |
| NNN | `png_reset_zstream` (`png.c:981`) | `png_ptr == NULL` | returns `Z_STREAM_ERROR` (-2) |
| NNN | `png_reset_zstream` (`png.c:985`) | `inflateReset` fails (zstream not initialized) | returns the zlib error code (`Z_STREAM_ERROR`) |
| NNN | `png_zstream_error` (`png.c:1013`) | `zstream.msg == NULL` and `ret` is `Z_OK` or any unrecognized value | `zstream.msg = "unexpected zlib return code"` |
| NNN | `png_zstream_error` (`png.c:1018`) | `zstream.msg == NULL` and `ret == Z_STREAM_END` at an unexpected point | `zstream.msg = "unexpected end of LZ stream"` |
| NNN | `png_zstream_error` (`png.c:1025`) | `ret == Z_NEED_DICT` (deflate stream demands a preset dictionary — bogus PNG) | `zstream.msg = "missing LZ dictionary"` |
| NNN | `png_zstream_error` (`png.c:1030`) | `ret == Z_ERRNO` | `zstream.msg = "zlib IO error"` |
| NNN | `png_zstream_error` (`png.c:1035`) | `ret == Z_STREAM_ERROR` (internal libpng error / bad zlib params) | `zstream.msg = "bad parameters to zlib"` |
| NNN | `png_zstream_error` (`png.c:1039`) | `ret == Z_DATA_ERROR` (corrupt compressed data) | `zstream.msg = "damaged LZ stream"` |
| NNN | `png_zstream_error` (`png.c:1043`) | `ret == Z_MEM_ERROR` | `zstream.msg = "insufficient memory"` |
| NNN | `png_zstream_error` (`png.c:1050`) | `ret == Z_BUF_ERROR` (input or output exhausted) | `zstream.msg = "truncated"` |
| NNN | `png_zstream_error` (`png.c:1054`) | `ret == Z_VERSION_ERROR` | `zstream.msg = "unsupported zlib version"` |
| NNN | `png_zstream_error` (`png.c:1063`) | `ret == PNG_UNEXPECTED_ZLIB_RETURN` (-7) | `zstream.msg = "unexpected zlib return"` (no trailing " code") |
| NNN | `png_fp_add` (`png.c:1080`) | `addend0 > 0` and `0x7fffffff - addend0 < addend1` (positive overflow) | `*error = 1` and returns `PNG_FP_1/2` (50000) |
| NNN | `png_fp_add` (`png.c:1085`) | `addend0 < 0` and `-0x7fffffff - addend0 > addend1` (negative overflow) | `*error = 1` and returns `PNG_FP_1/2` (50000) |
| NNN | `png_fp_sub` (`png.c:1101`) | `addend1 > 0` and `-0x7fffffff + addend1 > addend0` (negative overflow of `addend0-addend1`) | `*error = 1` and returns `PNG_FP_1/2` (50000) |
| NNN | `png_fp_sub` (`png.c:1106`) | `addend1 < 0` and `0x7fffffff + addend1 < addend0` (positive overflow) | `*error = 1` and returns `PNG_FP_1/2` (50000) |
| NNN | `png_safe_add` (`png.c:1127`) | either nested `png_fp_add` set `error` | returns 1 (failure) and leaves `*addend0_and_result` unmodified |
| NNN | `png_xy_from_XYZ` (`png.c:1146`) | `XYZ->red_X + red_Y + red_Z` overflows `png_int_32` | returns 1 (error) |
| NNN | `png_xy_from_XYZ` (`png.c:1149`) | `png_muldiv(&xy->redx, red_X, PNG_FP_1, dred) == 0` (dred == 0 or overflow) | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1151`) | `png_muldiv(&xy->redy, red_Y, PNG_FP_1, dred) == 0` | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1155`) | `green_X + green_Y + green_Z` overflows | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1158`) | `png_muldiv(&xy->greenx, green_X, PNG_FP_1, dgreen) == 0` | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1160`) | `png_muldiv(&xy->greeny, green_Y, PNG_FP_1, dgreen) == 0` | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1164`) | `blue_X + blue_Y + blue_Z` overflows | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1167`) | `png_muldiv(&xy->bluex, blue_X, PNG_FP_1, dblue) == 0` | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1169`) | `png_muldiv(&xy->bluey, blue_Y, PNG_FP_1, dblue) == 0` | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1177`) | `dblue + dred + dgreen` (white point sum) overflows | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1185`) | `red_X + green_X + blue_X` overflows | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1190`) | `red_Y + green_Y + blue_Y` overflows | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1194`) | `png_muldiv(&xy->whitex, whiteX, PNG_FP_1, dwhite) == 0` (`dwhite == 0`) | returns 1 |
| NNN | `png_xy_from_XYZ` (`png.c:1196`) | `png_muldiv(&xy->whitey, whiteY, PNG_FP_1, dwhite) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1223`) | `xy->redx < 0` or `xy->redx > fpLimit` (`fpLimit = PNG_FP_1 + PNG_FP_1/10 = 110000`) | returns 1 (error) |
| NNN | `png_XYZ_from_xy` (`png.c:1224`) | `xy->redy < 0` or `xy->redy > 110000 - xy->redx` (redz would be too negative) | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1225`) | `xy->greenx < 0` or `> 110000` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1226`) | `xy->greeny < 0` or `> 110000 - xy->greenx` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1227`) | `xy->bluex < 0` or `> 110000` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1228`) | `xy->bluey < 0` or `> 110000 - xy->bluex` (rejects ACES AP0 bluey = -7700) | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1229`) | `xy->whitex < 0` or `> 110000` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1230`) | `xy->whitey < 5` (guards integer overflow; 0 rejected) or `> 110000 - xy->whitex` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1422`) | `png_muldiv(&left, greenx-bluex, redy-bluey, 8) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1424`) | `png_muldiv(&right, greeny-bluey, redx-bluex, 8) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1427`) | `png_fp_sub(left, right, &error)` set `error` (denominator overflow) | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1431`) | `png_muldiv(&left, greenx-bluex, whitey-bluey, 8) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1433`) | `png_muldiv(&right, greeny-bluey, whitex-bluex, 8) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1442`) | `png_muldiv(&red_inverse, whitey, denominator, png_fp_sub(left,right,&error)) == 0` (divisor 0 / overflow — degenerate cHRM primaries) | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1443`) | `error != 0` from the `png_fp_sub` in the red-numerator | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1444`) | `red_inverse <= xy->whitey` (r+g+b scales inconsistent with white scale) | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1448`) | `png_muldiv(&left, redy-bluey, whitex-bluex, 8) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1450`) | `png_muldiv(&right, redx-bluex, whitey-bluey, 8) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1452`) | `png_muldiv(&green_inverse, ...) == 0`, or `error`, or `green_inverse <= xy->whitey` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1463`) | `error` from the `png_fp_sub`/`png_reciprocal` chain, or `blue_scale <= 0` (extreme cHRM values) | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1471`) | `png_muldiv(&XYZ->red_X, redx, PNG_FP_1, red_inverse) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1473`) | `png_muldiv(&XYZ->red_Y, redy, PNG_FP_1, red_inverse) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1475`) | `png_muldiv(&XYZ->red_Z, PNG_FP_1-redx-redy, PNG_FP_1, red_inverse) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1479`) | `png_muldiv(&XYZ->green_X, greenx, PNG_FP_1, green_inverse) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1481`) | `png_muldiv(&XYZ->green_Y, greeny, PNG_FP_1, green_inverse) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1483`) | `png_muldiv(&XYZ->green_Z, PNG_FP_1-greenx-greeny, PNG_FP_1, green_inverse) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1487`) | `png_muldiv(&XYZ->blue_X, bluex, blue_scale, PNG_FP_1) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1489`) | `png_muldiv(&XYZ->blue_Y, bluey, blue_scale, PNG_FP_1) == 0` | returns 1 |
| NNN | `png_XYZ_from_xy` (`png.c:1491`) | `png_muldiv(&XYZ->blue_Z, PNG_FP_1-bluex-bluey, blue_scale, PNG_FP_1) == 0` | returns 1 |
| NNN | `png_icc_profile_error` (`png.c:1571`) | any iCCP validation failure below | builds `"profile '<name>': <'tag'\|hexh>: <reason>"` (max 195 bytes, name truncated to 79) and calls `png_chunk_benign_error`, then returns 0 |
| NNN | `png_icc_tag_char` (`png.c:1505`) | byte outside printable ASCII 32..126 | substituted with `'?'` in the message |
| NNN | `icc_check_length` (`png.c:1588`) | `profile_length < 132` (shorter than the ICC header + tag count) | `png_icc_profile_error(..., "too short")` -> returns 0 |
| NNN | `png_icc_check_length` (`png.c:1597`) | `icc_check_length` failed (`profile_length < 132`) | returns 0 |
| NNN | `png_icc_check_length` (`png.c:1606`) | `profile_length > png_chunk_max(png_ptr)` i.e. `> png_ptr->user_chunk_malloc_max` (compile default `PNG_USER_CHUNK_MALLOC_MAX` = 8000000) | `png_icc_profile_error(..., "profile too long")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1626`) | `png_get_uint_32(profile) != profile_length` (header-declared size disagrees with chunk size) | `png_icc_profile_error(..., "length does not match profile")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1631`) | `profile[8] > 3` (major version > 3) and `(profile_length & 3) != 0` (not 4-byte aligned) | `png_icc_profile_error(..., "invalid length")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1636`) | tag count `png_get_uint_32(profile+128) > 357913930` | `png_icc_profile_error(..., "tag count too large")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1637`) | `profile_length < 132 + 12*tag_count` (truncated tag table) | `png_icc_profile_error(..., "tag count too large")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1645`) | rendering intent `png_get_uint_32(profile+64) >= 0xffff` (exceeds the ICC 16-bit limit) | `png_icc_profile_error(..., "invalid rendering intent")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1652`) | rendering intent `>= PNG_sRGB_INTENT_LAST` (4) but `< 0xffff` | `(void)png_icc_profile_error(..., "intent outside defined range")` — warning only (benign), execution CONTINUES |
| NNN | `png_icc_check_header` (`png.c:1669`) | `png_get_uint_32(profile+36) != 0x61637370` (missing `'acsp'` file signature) | `png_icc_profile_error(..., "invalid signature")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1680`) | `memcmp(profile+68, D50_nCIEXYZ, 12) != 0` (PCS illuminant not `{00 00 f6 d6, 00 01 00 00, 00 00 d3 2d}`) | `(void)png_icc_profile_error(..., "PCS illuminant is not D50")` — warning only, CONTINUES |
| NNN | `png_icc_check_header` (`png.c:1708`) | data colour space (`profile+16`) is `'RGB '` (0x52474220) and `(color_type & PNG_COLOR_MASK_COLOR) == 0` (grayscale PNG) | `png_icc_profile_error(..., "RGB color space not permitted on grayscale PNG")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1714`) | data colour space is `'GRAY'` (0x47524159) and `(color_type & PNG_COLOR_MASK_COLOR) != 0` (colour PNG) | `png_icc_profile_error(..., "Gray color space not permitted on RGB PNG")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1719`) | data colour space is neither `'RGB '` nor `'GRAY'` (e.g. `'CMYK'`, `'Lab '`) | `png_icc_profile_error(..., "invalid ICC profile color space")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1743`) | profile/device class (`profile+12`) is `'abst'` (0x61627374) | `png_icc_profile_error(..., "invalid embedded Abstract ICC profile")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1748`) | profile class is `'link'` (0x6c696e6b, DeviceLink) | `png_icc_profile_error(..., "unexpected DeviceLink ICC profile class")` -> returns 0 |
| NNN | `png_icc_check_header` (`png.c:1758`) | profile class is `'nmcl'` (0x6e6d636c, NamedColor) | `(void)png_icc_profile_error(..., "unexpected NamedColor ICC profile class")` — warning only, CONTINUES |
| NNN | `png_icc_check_header` (`png.c:1767`) | profile class is none of `'scnr'`,`'mntr'`,`'prtr'`,`'spac'`,`'abst'`,`'link'`,`'nmcl'` | `(void)png_icc_profile_error(..., "unrecognized ICC profile class")` — warning only, CONTINUES |
| NNN | `png_icc_check_header` (`png.c:1788`) | PCS (`profile+20`) is neither `'XYZ '` (0x58595a20) nor `'Lab '` (0x4c616220) | `png_icc_profile_error(..., "unexpected ICC PCS encoding")` -> returns 0 |
| NNN | `png_icc_check_tag_table` (`png.c:1824`) | any tag's `tag_start > profile_length` (tag data begins outside the profile) | `png_icc_profile_error(..., "ICC profile tag outside profile")` -> returns 0 |
| NNN | `png_icc_check_tag_table` (`png.c:1824`) | any tag's `tag_length > profile_length - tag_start` (tag data runs past the end) | `png_icc_profile_error(..., "ICC profile tag outside profile")` -> returns 0 |
| NNN | `png_icc_check_tag_table` (`png.c:1828`) | any tag's `(tag_start & 3) != 0` (start not a multiple of 4) | `(void)png_icc_profile_error(..., "ICC profile tag start not a multiple of 4")` — warning only, loop CONTINUES; function still returns 1 |
| NNN | `have_chromaticities` (`png.c:1871`) | `png_has_chunk(png_ptr, sRGB)` and no mDCV | returns 0 — `png_struct::chromaticities` ignored, sRGB defaults used |
| NNN | `have_chromaticities` (`png.c:1881`) | no mDCV, no sRGB, no cHRM chunk seen | returns 0 — sRGB defaults used |
| NNN | `png_set_rgb_coefficients` (`png.c:1897`) | `have_chromaticities(png_ptr) == 0` or `png_XYZ_from_xy(...) != 0` (cHRM/mDCV values rejected) | falls to `else`: hard-codes REC 709 coefficients `red=6968, green=23434` |
| NNN | `png_set_rgb_coefficients` (`png.c:1908`) | `total = r+g+b <= 0` (non-positive Y sum from the colorspace colorants) | inner `if` fails: `rgb_to_gray_red_coeff`/`green_coeff` left at their existing (0) values |
| NNN | `png_set_rgb_coefficients` (`png.c:1909`) | `r < 0`, or `png_muldiv(&r, r, 32768, total) == 0`, or scaled `r < 0` or `r > 32768` | coefficients left unset (silently) |
| NNN | `png_set_rgb_coefficients` (`png.c:1910`) | `g < 0`, or `png_muldiv(&g, g, 32768, total) == 0`, or scaled `g < 0` or `g > 32768` | coefficients left unset |
| NNN | `png_set_rgb_coefficients` (`png.c:1911`) | `b < 0`, or `png_muldiv(&b, b, 32768, total) == 0`, or scaled `b < 0` or `b > 32768` | coefficients left unset |
| NNN | `png_set_rgb_coefficients` (`png.c:1912`) | `r+g+b > 32769` after scaling | coefficients left unset |
| NNN | `png_set_rgb_coefficients` (`png.c:1937`) | after the `+/-1` adjustment `r+g+b != 32768` | `png_error(png_ptr, "internal error handling cHRM coefficients")` -> longjmp / non-zero `setjmp` |
| NNN | `png_check_IHDR` (`png.c:1969`) | `width == 0` | `png_warning(png_ptr, "Image width is zero in IHDR")`, `error = 1` -> eventual `png_error "Invalid IHDR data"` |
| NNN | `png_check_IHDR` (`png.c:1975`) | `width > PNG_UINT_31_MAX` (0x7fffffff) | `png_warning(png_ptr, "Invalid image width in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:1989`) | `((width + 7) & ~(png_alloc_size_t)7) > (((PNG_SIZE_MAX - 48 - 1) / 8) - 1)` (row buffer would not fit `size_t`) | `png_warning(png_ptr, "Image width is too large for this architecture")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2012`) | `width > png_ptr->user_width_max` (initialized from `PNG_USER_WIDTH_MAX` = 1000000, settable via `png_set_user_limits`) | `png_warning(png_ptr, "Image width exceeds user limit in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2014`) | non-`PNG_SET_USER_LIMITS_SUPPORTED` builds (NOT this config): `width > PNG_USER_WIDTH_MAX` | `png_warning(png_ptr, "Image width exceeds user limit in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2021`) | `height == 0` | `png_warning(png_ptr, "Image height is zero in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2027`) | `height > PNG_UINT_31_MAX` (0x7fffffff) | `png_warning(png_ptr, "Invalid image height in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2034`) | `height > png_ptr->user_height_max` (`PNG_USER_HEIGHT_MAX` = 1000000 default) | `png_warning(png_ptr, "Image height exceeds user limit in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2036`) | non-`PNG_SET_USER_LIMITS_SUPPORTED` builds (NOT this config): `height > PNG_USER_HEIGHT_MAX` | `png_warning(png_ptr, "Image height exceeds user limit in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2044`) | `bit_depth` not one of 1, 2, 4, 8, 16 | `png_warning(png_ptr, "Invalid bit depth in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2051`) | `color_type < 0`, `== 1`, `== 5`, or `> 6` | `png_warning(png_ptr, "Invalid color type in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2058`) | `color_type == PNG_COLOR_TYPE_PALETTE` (3) and `bit_depth > 8` | `png_warning(png_ptr, "Invalid color type/bit depth combination in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2059`) | `color_type` is RGB (2), GRAY_ALPHA (4) or RGB_ALPHA (6) and `bit_depth < 8` | `png_warning(png_ptr, "Invalid color type/bit depth combination in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2067`) | `interlace_type >= PNG_INTERLACE_LAST` (>= 2) | `png_warning(png_ptr, "Unknown interlace method in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2073`) | `compression_type != PNG_COMPRESSION_TYPE_BASE` (!= 0) | `png_warning(png_ptr, "Unknown compression method in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2089`) | `(png_ptr->mode & PNG_HAVE_PNG_SIGNATURE) != 0` and `png_ptr->mng_features_permitted != 0` | `png_warning(png_ptr, "MNG features are not allowed in a PNG datastream")` — warning only, `error` NOT set |
| NNN | `png_check_IHDR` (`png.c:2095`) | `filter_type != PNG_FILTER_TYPE_BASE` and NOT (`mng_features_permitted & PNG_FLAG_MNG_FILTER_64` and `filter_type == PNG_INTRAPIXEL_DIFFERENCING` (64) and no PNG signature seen and `color_type` in {RGB, RGB_ALPHA}) | `png_warning(png_ptr, "Unknown filter method in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2105`) | `filter_type != 0` and `(png_ptr->mode & PNG_HAVE_PNG_SIGNATURE) != 0` | `png_warning(png_ptr, "Invalid filter method in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2113`) | non-`PNG_MNG_FEATURES_SUPPORTED` builds (NOT this config): `filter_type != PNG_FILTER_TYPE_BASE` | `png_warning(png_ptr, "Unknown filter method in IHDR")`, `error = 1` |
| NNN | `png_check_IHDR` (`png.c:2120`) | `error == 1` (any of the above IHDR checks failed) | `png_error(png_ptr, "Invalid IHDR data")` -> longjmp / non-zero `setjmp` |
| NNN | `png_check_fp_number` (`png.c:2155`) | current character is not `'+'`(43), `'-'`(45), `'.'`(46), `'0'`-`'9'`(48-57), `'E'`(69) or `'e'`(101) | `default: goto PNG_FP_End` — scan stops at `*whereami = i` |
| NNN | `png_check_fp_number` (`png.c:2165`) | sign character in `PNG_FP_INTEGER` state when `(state & PNG_FP_SAW_ANY) != 0` (e.g. `"1+2"`) | `goto PNG_FP_End`; sign not consumed |
| NNN | `png_check_fp_number` (`png.c:2173`) | second `'.'` seen (`(state & PNG_FP_SAW_DOT) != 0`), e.g. `"1.2.3"` | `goto PNG_FP_End` |
| NNN | `png_check_fp_number` (`png.c:2193`) | `'E'`/`'e'` in integer state with `(state & PNG_FP_SAW_DIGIT) == 0` (e.g. `"E5"`, `"+E5"`) | `goto PNG_FP_End` |
| NNN | `png_check_fp_number` (`png.c:2215`) | `'E'`/`'e'` in fraction state with no digits yet (`".E5"`) | `goto PNG_FP_End` |
| NNN | `png_check_fp_number` (`png.c:2223`) | sign in exponent state when `(state & PNG_FP_SAW_ANY) != 0` (e.g. `"1E5+"`) | `goto PNG_FP_End` |
| NNN | `png_check_fp_number` (`png.c:2241`) | any other state/type combination, e.g. `'.'` in exponent state (`"1E5.2"`), sign in fraction state | `default: goto PNG_FP_End` |
| NNN | `png_check_fp_number` (`png.c:2255`) | no digit was ever accepted (`(state & PNG_FP_SAW_DIGIT) == 0`), e.g. `""`, `"-"`, `"."`, `"abc"` | returns 0 (failure) |
| NNN | `png_check_fp_string` (`png.c:2266`) | `png_check_fp_number` returned 0 (no digits) | returns 0 (failure sentinel) |
| NNN | `png_check_fp_string` (`png.c:2267`) | trailing garbage: `char_index != size` and `string[char_index] != 0` (e.g. `"1.5x"`) | returns 0 (failure sentinel) |
| NNN | `png_pow10` (`png.c:2290`) | `power < DBL_MIN_10_EXP` (exponent underflows `double`) | returns 0 (used as a "no representable value" sentinel) |
| NNN | `png_ascii_from_fp` (`png.c:2325`) | `precision < 1` (i.e. 0) | silently clamped to `DBL_DIG` |
| NNN | `png_ascii_from_fp` (`png.c:2329`) | `precision > DBL_DIG+1` | silently clamped to `DBL_DIG+1` |
| NNN | `png_ascii_from_fp` (`png.c:2333`) | `size < precision+5` (output buffer too small) | skips everything, falls to `png_error(png_ptr, "ASCII conversion buffer too small")` -> longjmp |
| NNN | `png_ascii_from_fp` (`png.c:2608`) | an exponent is required but `size <= cdigits` (not enough room for the exponent digits) | falls out of the block to `png_error(png_ptr, "ASCII conversion buffer too small")` -> longjmp |
| NNN | `png_ascii_from_fp` (`png.c:2618`) | `!(fp >= DBL_MIN)` — `fp` is a denormal, zero, or NaN | writes the two bytes `"0"` and returns (no error) |
| NNN | `png_ascii_from_fp` (`png.c:2624`) | `fp > DBL_MAX` (positive infinity) | writes `"inf"` and returns (no error) |
| NNN | `png_ascii_from_fp` (`png.c:2368`) | `png_pow10(exp_b10+1) > DBL_MAX` while normalizing | `break` out of the scaling loop (accepts a slightly denormalized value rather than overflowing) |
| NNN | `png_ascii_from_fp` (`png.c:2635`) | buffer too small (either check above) | `png_error(png_ptr, "ASCII conversion buffer too small")` -> longjmp / non-zero `setjmp` |
| NNN | `png_ascii_from_fixed` (`png.c:2649`) | `size <= 12` (need 10 digits + `'.'` + `'-'` + `'\0'`) | skips conversion, falls to `png_error(png_ptr, "ASCII conversion buffer too small")` -> longjmp |
| NNN | `png_ascii_from_fixed` (`png.c:2661`) | `num > 0x80000000` after the `abs` (fixed value magnitude overflowed) | skips conversion, falls to `png_error(png_ptr, "ASCII conversion buffer too small")` -> longjmp |
| NNN | `png_ascii_from_fixed` (`png.c:2713`) | either check above | `png_error(png_ptr, "ASCII conversion buffer too small")` -> longjmp / non-zero `setjmp` |
| NNN | `png_fixed` (`png.c:2730`) | `floor(100000*fp + .5) > 2147483647.` | `png_fixed_error(png_ptr, text)` -> `png_error "fixed point overflow in <text>"` -> longjmp |
| NNN | `png_fixed` (`png.c:2730`) | `floor(100000*fp + .5) < -2147483648.` | `png_fixed_error(png_ptr, text)` -> `png_error "fixed point overflow in <text>"` -> longjmp |
| NNN | `png_fixed_ITU` (`png.c:2749`) | `floor(10000*fp + .5) > 2147483647.` | `png_fixed_error(png_ptr, text)` -> `png_error "fixed point overflow in <text>"` -> longjmp |
| NNN | `png_fixed_ITU` (`png.c:2749`) | `floor(10000*fp + .5) < 0` (negative value where an unsigned ITU value is required) | `png_fixed_error(png_ptr, text)` -> `png_error "fixed point overflow in <text>"` -> longjmp |
| NNN | `png_muldiv` (`png.c:2774`) | `divisor == 0` | falls through to `return 0` (failure boolean); `*res` NOT written |
| NNN | `png_muldiv` (`png.c:2790`) | `a*times/divisor` rounds outside `[-2147483648., 2147483647.]` (float build) | `return 0`; `*res` NOT written |
| NNN | `png_muldiv` (`png.c:2832`) | fixed-arithmetic build only (NOT this config): `s32 >= D` — 64-bit product too large for the divisor | `return 0` |
| NNN | `png_muldiv` (`png.c:2870`) | fixed-arithmetic build only (NOT this config): sign of `result` disagrees with `negative` (overflow) | `return 0` |
| NNN | `png_muldiv` (`png.c:2776`) | `a == 0` or `times == 0` | short-circuit: `*res = 0` and returns 1 (success, not an error) |
| NNN | `png_reciprocal` (`png.c:2889`) | `a == 0` (division by zero -> `r` is inf/NaN so the range test fails) | returns 0 as the error/overflow sentinel |
| NNN | `png_reciprocal` (`png.c:2891`) | `floor(1E10/a+.5)` outside `[-2147483648., 2147483647.]` (|a| < 5) | returns 0 (error/overflow) |
| NNN | `png_gamma_significant` (`png.c:2923`) | `PNG_FP_1 - PNG_GAMMA_THRESHOLD_FIXED <= gamma_val <= PNG_FP_1 + PNG_GAMMA_THRESHOLD_FIXED` (95000..105000) | returns 0 — gamma correction suppressed as insignificant |
| NNN | `png_product2` (`png.c:2943`) | fixed-arithmetic build only (NOT this config): `png_muldiv(&res, a, b, 100000) == 0` | returns 0 (overflow sentinel) |
| NNN | `png_reciprocal2` (`png.c:2956`) | `a == 0` or `b == 0` | skips the computation and returns 0 (overflow/error sentinel) |
| NNN | `png_reciprocal2` (`png.c:2962`) | `floor(1E15/a/b+.5)` outside `[-2147483648., 2147483647.]` | returns 0 (overflow sentinel) |
| NNN | `png_log8bit` (`png.c:3057`) | fixed-arithmetic build only (NOT this config): `(x & 0xff) == 0` (log of zero) | returns -1 as the overflow/undefined sentinel |
| NNN | `png_log16bit` (`png.c:3110`) | fixed-arithmetic build only (NOT this config): `(x & 0xffff) == 0` | returns -1 |
| NNN | `png_exp` (`png.c:3237`) | fixed-arithmetic build only (NOT this config): `x <= 0` (overflow) | returns `png_32bit_exp[0]` = 4294967295U (saturated) |
| NNN | `png_exp` (`png.c:3241`) | fixed-arithmetic build only (NOT this config): `x > 0xfffff` (underflow) | returns 0 |
| NNN | `png_gamma_8bit_correct` (`png.c:3275`) | `value == 0` or `value >= 255` | skips the `pow`, returns `(png_byte)(value & 0xff)` unchanged (endpoints preserved) |
| NNN | `png_gamma_8bit_correct` (`png.c:3308`) | fixed-arithmetic build only (NOT this config): `png_muldiv(&res, gamma_val, lg2, PNG_FP_1) == 0` | `value = 0` then returns 0 |
| NNN | `png_gamma_16bit_correct` (`png.c:3323`) | `value == 0` or `value >= 65535` | skips the `pow`, returns `(png_uint_16)value` unchanged |
| NNN | `png_gamma_16bit_correct` (`png.c:3338`) | fixed-arithmetic build only (NOT this config): `png_muldiv` overflow | `value = 0` then returns 0 |
| NNN | `png_gamma_correct` (`png.c:3367`) | non-`PNG_16BIT_SUPPORTED` builds (NOT this config) with `bit_depth != 8` | `return 0` ("should not reach this") |
| NNN | `png_build_16bit_table` (`png.c:3397`) | `png_calloc` for the table array fails | `png_malloc`->`png_error(png_ptr, "Out of memory")` -> longjmp; caller must clean `*ptable` |
| NNN | `png_build_16bit_table` (`png.c:3402`) | `png_malloc` for a 256-entry sub-table fails | `png_error(png_ptr, "Out of memory")` -> longjmp |
| NNN | `png_build_16bit_table` (`png.c:3407`) | `png_gamma_significant(gamma_val) == 0` | builds an identity/scaling table instead of applying gamma |
| NNN | `png_build_16to8_table` (`png.c:3467`) | `png_calloc` fails | `png_error(png_ptr, "Out of memory")` -> longjmp |
| NNN | `png_build_16to8_table` (`png.c:3474`) | `png_malloc` of a sub-table fails | `png_error(png_ptr, "Out of memory")` -> longjmp |
| NNN | `png_build_8bit_table` (`png.c:3530`) | `png_malloc(png_ptr, 256)` fails | `png_error(png_ptr, "Out of memory")` -> longjmp |
| NNN | `png_build_8bit_table` (`png.c:3532`) | `png_gamma_significant(gamma_val) == 0` | builds the identity table `table[i] = i & 0xff` |
| NNN | `png_build_gamma_table` (`png.c:3632`) | `png_ptr->gamma_table != NULL` or `png_ptr->gamma_16_table != NULL` (repeat call, e.g. `png_read_update_info` called twice) | `png_warning(png_ptr, "gamma table being rebuilt")` then `png_destroy_gamma_table` and rebuild |
| NNN | `png_build_gamma_table` (`png.c:3648`) | `png_ptr->screen_gamma <= 0` (screen gamma unknown/unset) | `correction = PNG_FP_1` (identity) and `linear_to_screen = file_gamma` — no gamma correction applied |
| NNN | `png_build_gamma_table` (`png.c:3713`) | `sig_bit == 0` or `sig_bit >= 16` (invalid/absent sBIT) | `shift = 0` — all 16 bits kept |
| NNN | `png_build_gamma_table` (`png.c:3726`) | 16->8 transform requested and `shift < 16U - PNG_MAX_GAMMA_8` (i.e. `< 5`) | `shift` forced up to `16 - PNG_MAX_GAMMA_8` = 5 |
| NNN | `png_build_gamma_table` (`png.c:3730`) | `shift > 8U` | clamped to `8U` "Guarantees at least one table!" |
| NNN | `png_set_option` (`png.c:3771`) | `png_ptr == NULL` | returns `PNG_OPTION_INVALID` (1) |
| NNN | `png_set_option` (`png.c:3771`) | `option < 0` | returns `PNG_OPTION_INVALID` (1) |
| NNN | `png_set_option` (`png.c:3771`) | `option >= PNG_OPTION_NEXT` (>= 16) | returns `PNG_OPTION_INVALID` (1) |
| NNN | `png_set_option` (`png.c:3772`) | `(option & 1) != 0` (odd option number — options are 2-bit fields at even offsets) | returns `PNG_OPTION_INVALID` (1) |
| NNN | `png_image_free_function` (`png.c:3968`) | `image->opaque->png_ptr == NULL` | returns 0 (failure) — nothing freed |
| NNN | `png_image_free_function` (`png.c:3979`) | `cp->png_ptr->io_ptr` is NULL when `cp->owned_file != 0` | `fclose` skipped; errors from `fclose` are deliberately ignored |
| NNN | `png_image_free_function` (`png.c:4002`) | non-`PNG_SIMPLIFIED_WRITE_SUPPORTED` builds (NOT this config) with `c.for_write != 0` | `png_error(c.png_ptr, "simplified write not supported")` -> longjmp |
| NNN | `png_image_free_function` (`png.c:4010`) | non-`PNG_SIMPLIFIED_READ_SUPPORTED` builds (NOT this config) with `c.for_write == 0` | `png_error(c.png_ptr, "simplified read not supported")` -> longjmp |
| NNN | `png_image_free` (`png.c:4025`) | `image == NULL` | no-op |
| NNN | `png_image_free` (`png.c:4025`) | `image->opaque == NULL` (already freed) | no-op |
| NNN | `png_image_free` (`png.c:4026`) | `image->opaque->error_buf != NULL` (inside an error-handling context) | no-op — deferred to `png_safe_execute` |
| NNN | `png_image_error` (`png.c:4034`) | any simplified-API failure | copies `error_message` into `image->message` (truncated to `sizeof image->message`), sets `PNG_IMAGE_ERROR` in `image->warning_or_error`, `png_image_free(image)`, returns 0 |
| NNN | `png_access_version_number` (`png.c:994`) | version-mismatch detection helper (no failure branch) | returns `PNG_LIBPNG_VER` (10659) so the app can compare against its compile-time `PNG_LIBPNG_VER` |
| NNN | `Your_png_h_is_not_version_1_6_59_git` (`png.c:16`) | `png.h` in the include path is not 1.6.59.git (typedef `png_libpng_version_1_6_59_git` undeclared) | compile-time error — build fails before any runtime check |
| NNN | `PNG_CHUNK` order assertion (`png.c:27`) | `PNG_KNOWN_CHUNKS` index values do not increment from 0 | `#error PNG_KNOWN_CHUNKS chunk definitions are not in order` (compile-time) |
| NNN | `PNG_CHUNK` name assertion (`png.c:39`) | any `png_cHNK` macro undefined or failing `PNG_CHUNK_NAME_VALID` | `#error png_cHNK not defined for some known cHNK` (compile-time) |
