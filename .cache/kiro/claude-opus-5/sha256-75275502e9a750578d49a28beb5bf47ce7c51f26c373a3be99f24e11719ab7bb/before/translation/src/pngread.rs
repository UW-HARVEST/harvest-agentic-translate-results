//! Translation of pngread.c
//!
//! This file contains routines that an application calls directly to
//! read a PNG file or stream.

use crate::*;

/* ------------------------------------------------------------------------- *
 *  Private helpers that are macros in the C source and are not provided by
 *  the prelude.  They are defined privately here per the translation
 *  contract.
 * ------------------------------------------------------------------------- */

/* png_voidcast(type, value) is just a cast in C. */

/* PNG_PASS_* macros from png.h (interlace geometry). */
#[inline]
fn PNG_PASS_START_ROW(pass: c_int) -> c_int {
    (((1 & !pass) << (3 - (pass >> 1))) & 7) as c_int
}
#[inline]
fn PNG_PASS_START_COL(pass: c_int) -> c_int {
    (((1 & pass) << (3 - ((pass + 1) >> 1))) & 7) as c_int
}
#[inline]
fn PNG_PASS_ROW_OFFSET(pass: c_int) -> c_int {
    if pass > 2 {
        8 >> (((pass - 1) >> 1))
    } else {
        8
    }
}
#[inline]
fn PNG_PASS_COL_OFFSET(pass: c_int) -> c_int {
    1 << ((7 - pass) >> 1)
}
#[inline]
fn PNG_PASS_COLS(width: png_uint_32, pass: c_int) -> png_uint_32 {
    /* #define PNG_PASS_COLS(width, pass)
     *    (((width)+(((1<<PNG_PASS_COL_SHIFT(pass))
     *    -1)-PNG_PASS_START_COL(pass)))>>PNG_PASS_COL_SHIFT(pass))
     * PNG_PASS_COL_SHIFT(pass) ((7-(pass))>>1)  */
    let shift = ((7 - pass) >> 1) as u32;
    (width.wrapping_add(((1u32 << shift) - 1).wrapping_sub(PNG_PASS_START_COL(pass) as png_uint_32)))
        >> shift
}

/* PNG_RGB_TO_GRAY_DEFAULT from png.h */
const PNG_RGB_TO_GRAY_DEFAULT: c_int = -1;

/* PNG_sRGB_FROM_LINEAR from pngpriv.h; uses the exported png_sRGB_base and
 * png_sRGB_delta tables from png.c.
 */
#[inline]
unsafe fn PNG_sRGB_FROM_LINEAR(linear: png_uint_32) -> png_byte {
    unsafe {
        (0xff
            & ((*png_sRGB_base.as_ptr().add((linear >> 15) as usize) as png_uint_32
                + ((((linear) & 0x7fff)
                    * *png_sRGB_delta.as_ptr().add((linear >> 15) as usize) as png_uint_32)
                    >> 12))
                >> 8)) as png_byte
    }
}

/* Encoding of PNG data (used by the color-map code) */
const P_NOTSET: c_int = 0; /* File encoding not yet known */
const P_sRGB: c_int = 1; /* 8-bit encoded to sRGB gamma */
const P_LINEAR: c_int = 2; /* 16-bit linear: not encoded, NOT pre-multiplied! */
const P_FILE: c_int = 3; /* 8-bit encoded to file gamma, not sRGB or linear */
const P_LINEAR8: c_int = 4; /* 8-bit linear: only from a file value */

/* Color-map processing */
const PNG_CMAP_NONE: c_int = 0;
const PNG_CMAP_GA: c_int = 1;
const PNG_CMAP_TRANS: c_int = 2;
const PNG_CMAP_RGB: c_int = 3;
const PNG_CMAP_RGB_ALPHA: c_int = 4;

/* The following document where the background is for each processing case. */
const PNG_CMAP_NONE_BACKGROUND: c_uint = 256;
const PNG_CMAP_GA_BACKGROUND: c_uint = 231;
const PNG_CMAP_TRANS_BACKGROUND: c_uint = 254;
const PNG_CMAP_RGB_BACKGROUND: c_uint = 256;
const PNG_CMAP_RGB_ALPHA_BACKGROUND: c_uint = 216;

/* strerror / errno for the simplified read-from-file API. */
unsafe extern "C" {
    fn strerror(errnum: c_int) -> *mut c_char;
    fn __errno_location() -> *mut c_int;
}

/* ------------------------------------------------------------------------- *
 *  png_create_read_struct / png_create_read_struct_2
 * ------------------------------------------------------------------------- */

/* Create a PNG structure for reading, and allocate any memory needed. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_create_read_struct(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
) -> png_structp {
    unsafe {
        /* PNG_USER_MEM_SUPPORTED is defined for this build. */
        png_create_read_struct_2(
            user_png_ver,
            error_ptr,
            error_fn,
            warn_fn,
            core::ptr::null_mut(),
            None,
            None,
        )
    }
}

/* Alternate create PNG structure for reading, and allocate any memory
 * needed.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_create_read_struct_2(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) -> png_structp {
    unsafe {
        let png_ptr: png_structp = png_create_png_struct(
            user_png_ver,
            error_ptr,
            error_fn,
            warn_fn,
            mem_ptr,
            malloc_fn,
            free_fn,
        );

        if png_ptr != core::ptr::null_mut() {
            (*png_ptr).mode = PNG_IS_READ_STRUCT;

            /* Added in libpng-1.6.0; this can be used to detect a read structure if
             * required (it will be zero in a write structure.)
             */
            (*png_ptr).IDAT_read_size = PNG_IDAT_READ_SIZE as uInt;

            (*png_ptr).flags |= PNG_FLAG_BENIGN_ERRORS_WARN;

            /* In stable builds only warn if an application error can be completely
             * handled.  PNG_RELEASE_BUILD is 0 in this configuration.
             */

            /* TODO: delay this, it can be done in png_init_io (if the app doesn't
             * do it itself) avoiding setting the default function if it is not
             * required.
             */
            png_set_read_fn(png_ptr, core::ptr::null_mut(), None);
        }

        png_ptr
    }
}

/* ------------------------------------------------------------------------- *
 *  png_read_info
 * ------------------------------------------------------------------------- */

/* Read the information before the actual image data. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    unsafe {
        let mut keep: c_int;

        if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
            return;
        }

        /* Read and check the PNG file signature. */
        png_read_sig(png_ptr, info_ptr);

        loop {
            let length: png_uint_32 = png_read_chunk_header(png_ptr);
            let chunk_name: png_uint_32 = (*png_ptr).chunk_name;

            /* IDAT logic needs to happen here to simplify getting the two flags
             * right.
             */
            if chunk_name == png_IDAT {
                if ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
                    png_chunk_error(png_ptr, c"Missing IHDR before IDAT".as_ptr());
                } else if (*png_ptr).color_type == PNG_COLOR_TYPE_PALETTE as png_byte
                    && ((*png_ptr).mode & PNG_HAVE_PLTE) == 0
                {
                    png_chunk_error(png_ptr, c"Missing PLTE before IDAT".as_ptr());
                } else if ((*png_ptr).mode & PNG_AFTER_IDAT) != 0 {
                    png_chunk_benign_error(png_ptr, c"Too many IDATs found".as_ptr());
                }

                (*png_ptr).mode |= PNG_HAVE_IDAT;
            } else if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
                (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT;
                (*png_ptr).mode |= PNG_AFTER_IDAT;
            }

            if chunk_name == png_IHDR {
                png_handle_chunk(png_ptr, info_ptr, length);
            } else if chunk_name == png_IEND {
                png_handle_chunk(png_ptr, info_ptr, length);
            } else if {
                keep = png_chunk_unknown_handling(png_ptr, chunk_name);
                keep != 0
            } {
                png_handle_unknown(png_ptr, info_ptr, length, keep);

                if chunk_name == png_PLTE {
                    (*png_ptr).mode |= PNG_HAVE_PLTE;
                } else if chunk_name == png_IDAT {
                    (*png_ptr).idat_size = 0; /* It has been consumed */
                    break;
                }
            } else if chunk_name == png_IDAT {
                (*png_ptr).idat_size = length;
                break;
            } else {
                png_handle_chunk(png_ptr, info_ptr, length);
            }
        }
    }
}

/* ------------------------------------------------------------------------- *
 *  png_read_update_info
 * ------------------------------------------------------------------------- */

/* Optional call to update the users info_ptr structure */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_update_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    unsafe {
        if png_ptr != core::ptr::null_mut() {
            if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
                png_read_start_row(png_ptr);

                png_read_transform_info(png_ptr, info_ptr);
            }
            /* New in 1.6.0 this avoids the bug of doing the initializations twice */
            else {
                png_app_error(
                    png_ptr,
                    c"png_read_update_info/png_start_read_image: duplicate call".as_ptr(),
                );
            }
        }
    }
}

/* ------------------------------------------------------------------------- *
 *  png_start_read_image
 * ------------------------------------------------------------------------- */

/* Initialize palette, background, etc, after transformations
 * are set, but before any reading takes place.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_start_read_image(png_ptr: png_structrp) {
    unsafe {
        if png_ptr != core::ptr::null_mut() {
            if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
                png_read_start_row(png_ptr);
            }
            /* New in 1.6.0 this avoids the bug of doing the initializations twice */
            else {
                png_app_error(
                    png_ptr,
                    c"png_start_read_image/png_read_update_info: duplicate call".as_ptr(),
                );
            }
        }
    }
}

/* ------------------------------------------------------------------------- *
 *  png_do_read_intrapixel (static)
 * ------------------------------------------------------------------------- */

/* Undoes intrapixel differencing. */
unsafe fn png_do_read_intrapixel(row_info: png_row_infop, row: png_bytep) {
    unsafe {
        if ((*row_info).color_type & PNG_COLOR_MASK_COLOR as png_byte) != 0 {
            let bytes_per_pixel: c_int;
            let row_width: png_uint_32 = (*row_info).width;

            if (*row_info).bit_depth == 8 {
                let mut rp: png_bytep;
                let mut i: png_uint_32;

                if (*row_info).color_type == PNG_COLOR_TYPE_RGB as png_byte {
                    bytes_per_pixel = 3;
                } else if (*row_info).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte {
                    bytes_per_pixel = 4;
                } else {
                    return;
                }

                i = 0;
                rp = row;
                while i < row_width {
                    *rp = ((256 + *rp as c_int + *rp.add(1) as c_int) & 0xff) as png_byte;
                    *rp.add(2) =
                        ((256 + *rp.add(2) as c_int + *rp.add(1) as c_int) & 0xff) as png_byte;
                    i += 1;
                    rp = rp.add(bytes_per_pixel as usize);
                }
            } else if (*row_info).bit_depth == 16 {
                let mut rp: png_bytep;
                let mut i: png_uint_32;

                if (*row_info).color_type == PNG_COLOR_TYPE_RGB as png_byte {
                    bytes_per_pixel = 6;
                } else if (*row_info).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte {
                    bytes_per_pixel = 8;
                } else {
                    return;
                }

                i = 0;
                rp = row;
                while i < row_width {
                    let s0: png_uint_32 =
                        ((*rp as c_int) << 8) as png_uint_32 | *rp.add(1) as png_uint_32;
                    let s1: png_uint_32 =
                        ((*rp.add(2) as c_int) << 8) as png_uint_32 | *rp.add(3) as png_uint_32;
                    let s2: png_uint_32 =
                        ((*rp.add(4) as c_int) << 8) as png_uint_32 | *rp.add(5) as png_uint_32;
                    let red: png_uint_32 = (s0 + s1 + 65536) & 0xffff;
                    let blue: png_uint_32 = (s2 + s1 + 65536) & 0xffff;
                    *rp = ((red >> 8) & 0xff) as png_byte;
                    *rp.add(1) = (red & 0xff) as png_byte;
                    *rp.add(4) = ((blue >> 8) & 0xff) as png_byte;
                    *rp.add(5) = (blue & 0xff) as png_byte;
                    i += 1;
                    rp = rp.add(bytes_per_pixel as usize);
                }
            }
        }
    }
}

/* ------------------------------------------------------------------------- *
 *  png_read_row
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_row(
    png_ptr: png_structrp,
    row: png_bytep,
    dsp_row: png_bytep,
) {
    unsafe {
        let mut row_info: png_row_info = png_row_info::default();

        if png_ptr == core::ptr::null_mut() {
            return;
        }

        /* png_read_start_row sets the information (in particular iwidth) for this
         * interlace pass.
         */
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);
        }

        /* 1.5.6: row_info moved out of png_struct to a local here. */
        row_info.width = (*png_ptr).iwidth; /* NOTE: width of current interlaced row */
        row_info.color_type = (*png_ptr).color_type;
        row_info.bit_depth = (*png_ptr).bit_depth;
        row_info.channels = (*png_ptr).channels;
        row_info.pixel_depth = (*png_ptr).pixel_depth;
        row_info.rowbytes = PNG_ROWBYTES(row_info.pixel_depth as usize, row_info.width as usize);

        /* Check for transforms that have been set but were defined out.  In this
         * build all of the corresponding READ options are defined so none of the
         * warnings survive the preprocessor.
         */
        if (*png_ptr).row_number == 0 && (*png_ptr).pass == 0 {
            /* (all warning branches removed for this configuration) */
        }

        /* If interlaced and we do not need a new row, combine row and return. */
        if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
            match (*png_ptr).pass {
                0 => {
                    if (*png_ptr).row_number & 0x07 != 0 {
                        if dsp_row != core::ptr::null_mut() {
                            png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                        }
                        png_read_finish_row(png_ptr);
                        return;
                    }
                }

                1 => {
                    if ((*png_ptr).row_number & 0x07) != 0 || (*png_ptr).width < 5 {
                        if dsp_row != core::ptr::null_mut() {
                            png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                        }
                        png_read_finish_row(png_ptr);
                        return;
                    }
                }

                2 => {
                    if ((*png_ptr).row_number & 0x07) != 4 {
                        if dsp_row != core::ptr::null_mut() && ((*png_ptr).row_number & 4) != 0 {
                            png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                        }
                        png_read_finish_row(png_ptr);
                        return;
                    }
                }

                3 => {
                    if ((*png_ptr).row_number & 3) != 0 || (*png_ptr).width < 3 {
                        if dsp_row != core::ptr::null_mut() {
                            png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                        }
                        png_read_finish_row(png_ptr);
                        return;
                    }
                }

                4 => {
                    if ((*png_ptr).row_number & 3) != 2 {
                        if dsp_row != core::ptr::null_mut() && ((*png_ptr).row_number & 2) != 0 {
                            png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                        }
                        png_read_finish_row(png_ptr);
                        return;
                    }
                }

                5 => {
                    if ((*png_ptr).row_number & 1) != 0 || (*png_ptr).width < 2 {
                        if dsp_row != core::ptr::null_mut() {
                            png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                        }
                        png_read_finish_row(png_ptr);
                        return;
                    }
                }

                6 | _ => {
                    if ((*png_ptr).row_number & 1) == 0 {
                        png_read_finish_row(png_ptr);
                        return;
                    }
                }
            }
        }

        if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0 {
            png_error(png_ptr, c"Invalid attempt to read row data".as_ptr());
        }

        /* Fill the row with IDAT data: */
        *(*png_ptr).row_buf.add(0) = 255; /* to force error if no data was found */
        png_read_IDAT_data(png_ptr, (*png_ptr).row_buf, row_info.rowbytes + 1);

        if *(*png_ptr).row_buf.add(0) > PNG_FILTER_VALUE_NONE as png_byte {
            if *(*png_ptr).row_buf.add(0) < PNG_FILTER_VALUE_LAST as png_byte {
                png_read_filter_row(
                    png_ptr,
                    &raw mut row_info,
                    (*png_ptr).row_buf.add(1),
                    (*png_ptr).prev_row.add(1),
                    *(*png_ptr).row_buf.add(0) as c_int,
                );
            } else {
                png_error(png_ptr, c"bad adaptive filter value".as_ptr());
            }
        }

        /* libpng 1.5.6: copy the interlaced count. */
        memcpy(
            (*png_ptr).prev_row as *mut c_void,
            (*png_ptr).row_buf as *const c_void,
            row_info.rowbytes + 1,
        );

        if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
            && ((*png_ptr).filter_type == PNG_INTRAPIXEL_DIFFERENCING as png_byte)
        {
            /* Intrapixel differencing */
            png_do_read_intrapixel(&raw mut row_info, (*png_ptr).row_buf.add(1));
        }

        if (*png_ptr).transformations != 0 || (*png_ptr).num_palette_max >= 0 {
            png_do_read_transformations(png_ptr, &raw mut row_info);
        }

        /* The transformed pixel depth should match the depth now in row_info. */
        if (*png_ptr).transformed_pixel_depth == 0 {
            (*png_ptr).transformed_pixel_depth = row_info.pixel_depth;
            if row_info.pixel_depth > (*png_ptr).maximum_pixel_depth {
                png_error(png_ptr, c"sequential row overflow".as_ptr());
            }
        } else if (*png_ptr).transformed_pixel_depth != row_info.pixel_depth {
            png_error(
                png_ptr,
                c"internal sequential row size calculation error".as_ptr(),
            );
        }

        /* Expand interlaced rows to full size */
        if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
            if (*png_ptr).pass < 6 {
                png_do_read_interlace(
                    &raw mut row_info,
                    (*png_ptr).row_buf.add(1),
                    (*png_ptr).pass as c_int,
                    (*png_ptr).transformations,
                );
            }

            if dsp_row != core::ptr::null_mut() {
                png_combine_row(png_ptr, dsp_row, 1 /*display*/);
            }

            if row != core::ptr::null_mut() {
                png_combine_row(png_ptr, row, 0 /*row*/);
            }
        } else {
            if row != core::ptr::null_mut() {
                png_combine_row(png_ptr, row, -1 /*ignored*/);
            }

            if dsp_row != core::ptr::null_mut() {
                png_combine_row(png_ptr, dsp_row, -1 /*ignored*/);
            }
        }
        png_read_finish_row(png_ptr);

        if (*png_ptr).read_row_fn.is_some() {
            ((*png_ptr).read_row_fn.unwrap())(png_ptr, (*png_ptr).row_number, (*png_ptr).pass as c_int);
        }
    }
}

/* ------------------------------------------------------------------------- *
 *  png_read_rows
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_rows(
    png_ptr: png_structrp,
    row: png_bytepp,
    display_row: png_bytepp,
    num_rows: png_uint_32,
) {
    unsafe {
        let mut i: png_uint_32;
        let mut rp: png_bytepp;
        let mut dp: png_bytepp;

        if png_ptr == core::ptr::null_mut() {
            return;
        }

        rp = row;
        dp = display_row;
        if rp != core::ptr::null_mut() && dp != core::ptr::null_mut() {
            i = 0;
            while i < num_rows {
                let rptr: png_bytep = *rp;
                rp = rp.add(1);
                let dptr: png_bytep = *dp;
                dp = dp.add(1);

                png_read_row(png_ptr, rptr, dptr);
                i += 1;
            }
        } else if rp != core::ptr::null_mut() {
            i = 0;
            while i < num_rows {
                let rptr: png_bytep = *rp;
                png_read_row(png_ptr, rptr, core::ptr::null_mut());
                rp = rp.add(1);
                i += 1;
            }
        } else if dp != core::ptr::null_mut() {
            i = 0;
            while i < num_rows {
                let dptr: png_bytep = *dp;
                png_read_row(png_ptr, core::ptr::null_mut(), dptr);
                dp = dp.add(1);
                i += 1;
            }
        }
    }
}

/* ------------------------------------------------------------------------- *
 *  png_read_image
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_image(png_ptr: png_structrp, image: png_bytepp) {
    unsafe {
        let mut i: png_uint_32;
        let image_height: png_uint_32;
        let pass: c_int;
        let mut j: c_int;
        let mut rp: png_bytepp;

        if png_ptr == core::ptr::null_mut() {
            return;
        }

        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            pass = png_set_interlace_handling(png_ptr);
            /* And make sure transforms are initialized. */
            png_start_read_image(png_ptr);
        } else {
            if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) == 0 {
                /* Caller called png_start_read_image or png_read_update_info without
                 * first turning on the PNG_INTERLACE transform.
                 */
                png_warning(
                    png_ptr,
                    c"Interlace handling should be turned on when using png_read_image".as_ptr(),
                );
                /* Make sure this is set correctly */
                (*png_ptr).num_rows = (*png_ptr).height;
            }

            /* Obtain the pass number, which also turns on the PNG_INTERLACE flag in
             * the above error case.
             */
            pass = png_set_interlace_handling(png_ptr);
        }

        image_height = (*png_ptr).height;

        j = 0;
        while j < pass {
            rp = image;
            i = 0;
            while i < image_height {
                png_read_row(png_ptr, *rp, core::ptr::null_mut());
                rp = rp.add(1);
                i += 1;
            }
            j += 1;
        }
    }
}

/* ------------------------------------------------------------------------- *
 *  png_read_end
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    unsafe {
        let mut keep: c_int;

        if png_ptr == core::ptr::null_mut() {
            return;
        }

        /* If png_read_end is called in the middle of reading the rows there may
         * still be pending IDAT data and an owned zstream.  Deal with this here.
         */
        if png_chunk_unknown_handling(png_ptr, png_IDAT) == 0 {
            png_read_finish_IDAT(png_ptr);
        }

        /* Report invalid palette index; added at libpng-1.5.10 */
        if (*png_ptr).color_type == PNG_COLOR_TYPE_PALETTE as png_byte
            && (*png_ptr).num_palette_max >= (*png_ptr).num_palette as c_int
        {
            png_benign_error(
                png_ptr,
                c"Read palette index exceeding num_palette".as_ptr(),
            );
        }

        loop {
            let length: png_uint_32 = png_read_chunk_header(png_ptr);
            let chunk_name: png_uint_32 = (*png_ptr).chunk_name;

            if chunk_name != png_IDAT {
                /* These flags must be set consistently for all non-IDAT chunks,
                 * including the unknown chunks.
                 */
                (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT | PNG_AFTER_IDAT;
            }

            if chunk_name == png_IEND {
                png_handle_chunk(png_ptr, info_ptr, length);
            } else if chunk_name == png_IHDR {
                png_handle_chunk(png_ptr, info_ptr, length);
            } else if info_ptr == core::ptr::null_mut() {
                png_crc_finish(png_ptr, length);
            } else if {
                keep = png_chunk_unknown_handling(png_ptr, chunk_name);
                keep != 0
            } {
                if chunk_name == png_IDAT {
                    if (length > 0 && ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0)
                        || ((*png_ptr).mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0
                    {
                        png_benign_error(png_ptr, c".Too many IDATs found".as_ptr());
                    }
                }
                png_handle_unknown(png_ptr, info_ptr, length, keep);
                if chunk_name == png_PLTE {
                    (*png_ptr).mode |= PNG_HAVE_PLTE;
                }
            } else if chunk_name == png_IDAT {
                /* Zero length IDATs are legal after the last IDAT has been read. */
                if (length > 0 && ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0)
                    || ((*png_ptr).mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0
                {
                    png_benign_error(png_ptr, c"..Too many IDATs found".as_ptr());
                }

                png_crc_finish(png_ptr, length);
            } else {
                png_handle_chunk(png_ptr, info_ptr, length);
            }

            if ((*png_ptr).mode & PNG_HAVE_IEND) != 0 {
                break;
            }
        }
    }
}

/* ------------------------------------------------------------------------- *
 *  png_read_destroy (static) / png_destroy_read_struct
 * ------------------------------------------------------------------------- */

/* Free all memory used in the read struct */
unsafe fn png_read_destroy(png_ptr: png_structrp) {
    unsafe {
        png_destroy_gamma_table(png_ptr);

        png_free(png_ptr, (*png_ptr).big_row_buf as png_voidp);
        (*png_ptr).big_row_buf = core::ptr::null_mut();
        png_free(png_ptr, (*png_ptr).big_prev_row as png_voidp);
        (*png_ptr).big_prev_row = core::ptr::null_mut();
        png_free(png_ptr, (*png_ptr).read_buffer as png_voidp);
        (*png_ptr).read_buffer = core::ptr::null_mut();

        png_free(png_ptr, (*png_ptr).palette_lookup as png_voidp);
        (*png_ptr).palette_lookup = core::ptr::null_mut();
        png_free(png_ptr, (*png_ptr).quantize_index as png_voidp);
        (*png_ptr).quantize_index = core::ptr::null_mut();

        /* png_ptr->palette is always independently allocated. */
        png_free(png_ptr, (*png_ptr).palette as png_voidp);
        (*png_ptr).palette = core::ptr::null_mut();

        /* png_ptr->trans_alpha is always independently allocated. */
        png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
        (*png_ptr).trans_alpha = core::ptr::null_mut();

        inflateEnd(&raw mut (*png_ptr).zstream);

        png_free(png_ptr, (*png_ptr).save_buffer as png_voidp);
        (*png_ptr).save_buffer = core::ptr::null_mut();

        png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
        (*png_ptr).unknown_chunk.data = core::ptr::null_mut();

        png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
        (*png_ptr).chunk_list = core::ptr::null_mut();

        /* NOTE: the 'setjmp' buffer may still be allocated and the memory and error
         * callbacks are still set at this point.
         */
    }
}

/* Free all memory used by the read */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_destroy_read_struct(
    png_ptr_ptr: png_structpp,
    info_ptr_ptr: png_infopp,
    end_info_ptr_ptr: png_infopp,
) {
    unsafe {
        let mut png_ptr: png_structrp = core::ptr::null_mut();

        if png_ptr_ptr != core::ptr::null_mut() {
            png_ptr = *png_ptr_ptr;
        }

        if png_ptr == core::ptr::null_mut() {
            return;
        }

        /* libpng 1.6.0: use the API to destroy info structs. */
        png_destroy_info_struct(png_ptr, end_info_ptr_ptr);
        png_destroy_info_struct(png_ptr, info_ptr_ptr);

        *png_ptr_ptr = core::ptr::null_mut();
        png_read_destroy(png_ptr);
        png_destroy_png_struct(png_ptr);
    }
}

/* ------------------------------------------------------------------------- *
 *  png_set_read_status_fn
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_read_status_fn(
    png_ptr: png_structrp,
    read_row_fn: png_read_status_ptr,
) {
    unsafe {
        if png_ptr == core::ptr::null_mut() {
            return;
        }

        (*png_ptr).read_row_fn = read_row_fn;
    }
}

/* ------------------------------------------------------------------------- *
 *  png_read_png
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_png(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    transforms: c_int,
    params: png_voidp,
) {
    unsafe {
        if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
            return;
        }

        /* png_read_info() gives us all of the information from the
         * PNG file before the first IDAT (image data chunk).
         */
        png_read_info(png_ptr, info_ptr);
        if (*info_ptr).height as usize
            > PNG_UINT_32_MAX as usize / core::mem::size_of::<png_bytep>()
        {
            png_error(
                png_ptr,
                c"Image is too high to process with png_read_png()".as_ptr(),
            );
        }

        /* -------------- image transformations start here ------------------- */

        /* Tell libpng to strip 16-bit/color files down to 8 bits per color. */
        if (transforms & PNG_TRANSFORM_SCALE_16) != 0 {
            png_set_scale_16(png_ptr);
        }

        /* If both SCALE and STRIP are required pngrtran will effectively cancel the
         * latter by doing SCALE first.
         */
        if (transforms & PNG_TRANSFORM_STRIP_16) != 0 {
            png_set_strip_16(png_ptr);
        }

        /* Strip alpha bytes from the input data without combining with
         * the background (not recommended).
         */
        if (transforms & PNG_TRANSFORM_STRIP_ALPHA) != 0 {
            png_set_strip_alpha(png_ptr);
        }

        /* Extract multiple pixels with bit depths of 1, 2, or 4 from a single
         * byte into separate bytes.
         */
        if (transforms & PNG_TRANSFORM_PACKING) != 0 {
            png_set_packing(png_ptr);
        }

        /* Change the order of packed pixels to least significant bit first. */
        if (transforms & PNG_TRANSFORM_PACKSWAP) != 0 {
            png_set_packswap(png_ptr);
        }

        /* Expand paletted colors into true RGB triplets, etc. */
        if (transforms & PNG_TRANSFORM_EXPAND) != 0 {
            png_set_expand(png_ptr);
        }

        /* We don't handle background color or gamma transformation or quantizing.
         */

        /* Invert monochrome files to have 0 as white and 1 as black */
        if (transforms & PNG_TRANSFORM_INVERT_MONO) != 0 {
            png_set_invert_mono(png_ptr);
        }

        /* If you want to shift the pixel values ... */
        if (transforms & PNG_TRANSFORM_SHIFT) != 0 {
            if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
                png_set_shift(png_ptr, &raw mut (*info_ptr).sig_bit);
            }
        }

        /* Flip the RGB pixels to BGR (or RGBA to BGRA) */
        if (transforms & PNG_TRANSFORM_BGR) != 0 {
            png_set_bgr(png_ptr);
        }

        /* Swap the RGBA or GA data to ARGB or AG (or BGRA to ABGR) */
        if (transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0 {
            png_set_swap_alpha(png_ptr);
        }

        /* Swap bytes of 16-bit files to least significant byte first */
        if (transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0 {
            png_set_swap(png_ptr);
        }

        /* Added at libpng-1.2.41 */
        /* Invert the alpha channel from opacity to transparency */
        if (transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0 {
            png_set_invert_alpha(png_ptr);
        }

        /* Added at libpng-1.2.41 */
        /* Expand grayscale image to RGB */
        if (transforms & PNG_TRANSFORM_GRAY_TO_RGB) != 0 {
            png_set_gray_to_rgb(png_ptr);
        }

        /* Added at libpng-1.5.4 */
        if (transforms & PNG_TRANSFORM_EXPAND_16) != 0 {
            png_set_expand_16(png_ptr);
        }

        /* We don't handle adding filler bytes */

        /* We use png_read_image and rely on that for interlace handling, but we also
         * call png_read_update_info therefore must turn on interlace handling now:
         */
        let _ = png_set_interlace_handling(png_ptr);

        /* Optional call to gamma correct and add the background to the palette
         * and update info structure.
         */
        png_read_update_info(png_ptr, info_ptr);

        /* -------------- image transformations end here ------------------- */

        png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0);
        if (*info_ptr).row_pointers == core::ptr::null_mut() {
            let mut iptr: png_uint_32;

            (*info_ptr).row_pointers = png_malloc(
                png_ptr,
                (*info_ptr).height as usize * core::mem::size_of::<png_bytep>(),
            ) as png_bytepp;

            iptr = 0;
            while iptr < (*info_ptr).height {
                *(*info_ptr).row_pointers.add(iptr as usize) = core::ptr::null_mut();
                iptr += 1;
            }

            (*info_ptr).free_me |= PNG_FREE_ROWS;

            iptr = 0;
            while iptr < (*info_ptr).height {
                *(*info_ptr).row_pointers.add(iptr as usize) =
                    png_malloc(png_ptr, (*info_ptr).rowbytes) as png_bytep;
                iptr += 1;
            }
        }

        png_read_image(png_ptr, (*info_ptr).row_pointers);
        (*info_ptr).valid |= PNG_INFO_IDAT;

        /* Read rest of file, and get additional chunks in info_ptr - REQUIRED */
        png_read_end(png_ptr, info_ptr);

        PNG_UNUSED(params);
    }
}

/* ========================================================================= *
 *  SIMPLIFIED READ
 *
 *  This code currently relies on the sequential reader.
 * ========================================================================= */

#[repr(C)]
#[derive(Clone, Copy)]
struct png_image_read_control {
    /* Arguments */
    image: png_imagep,
    buffer: png_voidp,
    row_stride: png_int_32,
    colormap: png_voidp,
    background: png_const_colorp,

    /* Instance variables */
    local_row: png_voidp,
    first_row: png_voidp,
    row_step: isize, /* step between rows (ptrdiff_t) */
    file_encoding: c_int, /* E_ values above */
    gamma_to_linear: png_fixed_point, /* For P_FILE, reciprocal of gamma */
    colormap_processing: c_int, /* PNG_CMAP_ values above */
}

/* Do all the *safe* initialization. */
unsafe fn png_image_read_init(image: png_imagep) -> c_int {
    unsafe {
        if (*image).opaque == core::ptr::null_mut() {
            let png_ptr: png_structp = png_create_read_struct(
                PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
                image as png_voidp,
                Some(png_safe_error),
                Some(png_safe_warning),
            );

            /* And set the rest of the structure to NULL to ensure that the various
             * fields are consistent.
             */
            memset(image as png_voidp, 0, core::mem::size_of::<png_image>());
            (*image).version = PNG_IMAGE_VERSION;

            if png_ptr != core::ptr::null_mut() {
                let mut info_ptr: png_infop = png_create_info_struct(png_ptr);

                if info_ptr != core::ptr::null_mut() {
                    let control: png_controlp = png_malloc_warn(
                        png_ptr,
                        core::mem::size_of::<png_control>(),
                    ) as png_controlp;

                    if control != core::ptr::null_mut() {
                        memset(control as png_voidp, 0, core::mem::size_of::<png_control>());

                        (*control).png_ptr = png_ptr;
                        (*control).info_ptr = info_ptr;
                        (*control).set_for_write(0);

                        (*image).opaque = control;
                        return 1;
                    }

                    /* Error clean up */
                    png_destroy_info_struct(png_ptr, &raw mut info_ptr);
                }

                let mut png_ptr_local = png_ptr;
                png_destroy_read_struct(
                    &raw mut png_ptr_local,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                );
            }

            return png_image_error(image, c"png_image_read: out of memory".as_ptr());
        }

        png_image_error(image, c"png_image_read: opaque pointer not NULL".as_ptr())
    }
}

/* Utility to find the base format of a PNG file from a png_struct. */
unsafe fn png_image_format(png_ptr: png_structrp) -> png_uint_32 {
    unsafe {
        let mut format: png_uint_32 = 0;

        if ((*png_ptr).color_type & PNG_COLOR_MASK_COLOR as png_byte) != 0 {
            format |= PNG_FORMAT_FLAG_COLOR;
        }

        if ((*png_ptr).color_type & PNG_COLOR_MASK_ALPHA as png_byte) != 0 {
            format |= PNG_FORMAT_FLAG_ALPHA;
        }
        /* Use png_ptr here, not info_ptr. */
        else if (*png_ptr).num_trans > 0 {
            format |= PNG_FORMAT_FLAG_ALPHA;
        }

        if (*png_ptr).bit_depth == 16 {
            format |= PNG_FORMAT_FLAG_LINEAR;
        }

        if ((*png_ptr).color_type & PNG_COLOR_MASK_PALETTE as png_byte) != 0 {
            format |= PNG_FORMAT_FLAG_COLORMAP;
        }

        format
    }
}

const sRGB_TOLERANCE: png_fixed_point = 1000;

unsafe fn chromaticities_match_sRGB(xy: *const png_xy) -> c_int {
    unsafe {
        static sRGB_xy: png_xy = png_xy {
            /* From ITU-R BT.709-3 */
            redx: 64000,
            redy: 33000,
            greenx: 30000,
            greeny: 60000,
            bluex: 15000,
            bluey: 6000,
            whitex: 31270,
            whitey: 32900,
        };

        if PNG_OUT_OF_RANGE((*xy).whitex, sRGB_xy.whitex, sRGB_TOLERANCE)
            || PNG_OUT_OF_RANGE((*xy).whitey, sRGB_xy.whitey, sRGB_TOLERANCE)
            || PNG_OUT_OF_RANGE((*xy).redx, sRGB_xy.redx, sRGB_TOLERANCE)
            || PNG_OUT_OF_RANGE((*xy).redy, sRGB_xy.redy, sRGB_TOLERANCE)
            || PNG_OUT_OF_RANGE((*xy).greenx, sRGB_xy.greenx, sRGB_TOLERANCE)
            || PNG_OUT_OF_RANGE((*xy).greeny, sRGB_xy.greeny, sRGB_TOLERANCE)
            || PNG_OUT_OF_RANGE((*xy).bluex, sRGB_xy.bluex, sRGB_TOLERANCE)
            || PNG_OUT_OF_RANGE((*xy).bluey, sRGB_xy.bluey, sRGB_TOLERANCE)
        {
            return 0;
        }
        1
    }
}

/* Is the given gamma significantly different from sRGB? */
unsafe fn png_gamma_not_sRGB(g: png_fixed_point) -> c_int {
    unsafe {
        /* 1.6.47: use the same sanity checks as used in pngrtran.c */
        if g < PNG_LIB_GAMMA_MIN || g > PNG_LIB_GAMMA_MAX {
            return 0; /* Includes the uninitialized value 0 */
        }

        png_gamma_significant((g * 11 + 2) / 5 /* i.e. *2.2, rounded */)
    }
}

unsafe fn png_image_is_not_sRGB(png_ptr: png_const_structrp) -> c_int {
    unsafe {
        /* Highest priority: check to be safe. */
        if png_file_has_chunk(png_ptr, PNG_INDEX_cICP) || png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
            return (chromaticities_match_sRGB(&raw const (*png_ptr).chromaticities) == 0) as c_int;
        }

        /* If the image is marked as sRGB then it is... */
        if png_file_has_chunk(png_ptr, PNG_INDEX_sRGB) {
            return 0;
        }

        /* Last stop: cHRM, must check: */
        if png_file_has_chunk(png_ptr, PNG_INDEX_cHRM) {
            return (chromaticities_match_sRGB(&raw const (*png_ptr).chromaticities) == 0) as c_int;
        }

        /* Else default to sRGB */
        0
    }
}

unsafe extern "C-unwind" fn png_image_read_header(argument: png_voidp) -> c_int {
    unsafe {
        let image: png_imagep = argument as png_imagep;
        let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
        let info_ptr: png_inforp = (*(*image).opaque).info_ptr;

        png_set_benign_errors(png_ptr, 1 /*warn*/);
        png_read_info(png_ptr, info_ptr);

        /* Do this the fast way; just read directly out of png_struct. */
        (*image).width = (*png_ptr).width;
        (*image).height = (*png_ptr).height;

        {
            let format: png_uint_32 = png_image_format(png_ptr);

            (*image).format = format;

            if (format & PNG_FORMAT_FLAG_COLOR) != 0 && png_image_is_not_sRGB(png_ptr) != 0 {
                (*image).flags |= PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB;
            }
        }

        /* We need the maximum number of entries regardless of the format. */
        {
            let mut cmap_entries: png_uint_32;

            match (*png_ptr).color_type as c_int {
                PNG_COLOR_TYPE_GRAY => {
                    cmap_entries = 1u32 << (*png_ptr).bit_depth;
                }

                PNG_COLOR_TYPE_PALETTE => {
                    cmap_entries = (*png_ptr).num_palette as png_uint_32;
                }

                _ => {
                    cmap_entries = 256;
                }
            }

            if cmap_entries > 256 {
                cmap_entries = 256;
            }

            (*image).colormap_entries = cmap_entries;
        }

        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_begin_read_from_stdio(
    image: png_imagep,
    file: *mut FILE,
) -> c_int {
    unsafe {
        if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
            if file != core::ptr::null_mut() {
                if png_image_read_init(image) != 0 {
                    /* This is slightly evil, but png_init_io doesn't do anything other
                     * than this.
                     */
                    (*(*(*image).opaque).png_ptr).io_ptr = file as png_voidp;
                    return png_safe_execute(image, Some(png_image_read_header), image as png_voidp);
                }
            } else {
                return png_image_error(
                    image,
                    c"png_image_begin_read_from_stdio: invalid argument".as_ptr(),
                );
            }
        } else if image != core::ptr::null_mut() {
            return png_image_error(
                image,
                c"png_image_begin_read_from_stdio: incorrect PNG_IMAGE_VERSION".as_ptr(),
            );
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_begin_read_from_file(
    image: png_imagep,
    file_name: *const c_char,
) -> c_int {
    unsafe {
        if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
            if file_name != core::ptr::null() {
                let fp: *mut FILE = fopen(file_name, c"rb".as_ptr());

                if fp != core::ptr::null_mut() {
                    if png_image_read_init(image) != 0 {
                        (*(*(*image).opaque).png_ptr).io_ptr = fp as png_voidp;
                        (*(*image).opaque).set_owned_file(1);
                        return png_safe_execute(
                            image,
                            Some(png_image_read_header),
                            image as png_voidp,
                        );
                    }

                    /* Clean up: just the opened file. */
                    let _ = fclose(fp);
                } else {
                    return png_image_error(image, strerror(*__errno_location()));
                }
            } else {
                return png_image_error(
                    image,
                    c"png_image_begin_read_from_file: invalid argument".as_ptr(),
                );
            }
        } else if image != core::ptr::null_mut() {
            return png_image_error(
                image,
                c"png_image_begin_read_from_file: incorrect PNG_IMAGE_VERSION".as_ptr(),
            );
        }

        0
    }
}

unsafe extern "C-unwind" fn png_image_memory_read(
    png_ptr: png_structp,
    out: png_bytep,
    need: usize,
) {
    unsafe {
        if png_ptr != core::ptr::null_mut() {
            let image: png_imagep = (*png_ptr).io_ptr as png_imagep;
            if image != core::ptr::null_mut() {
                let cp: png_controlp = (*image).opaque;
                if cp != core::ptr::null_mut() {
                    let memory: png_const_bytep = (*cp).memory;
                    let size: usize = (*cp).size;

                    if memory != core::ptr::null() && size >= need {
                        memcpy(out as png_voidp, memory as png_const_voidp, need);
                        (*cp).memory = memory.add(need);
                        (*cp).size = size - need;
                        return;
                    }

                    png_error(png_ptr, c"read beyond end of data".as_ptr());
                }
            }

            png_error(png_ptr, c"invalid memory read".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_begin_read_from_memory(
    image: png_imagep,
    memory: png_const_voidp,
    size: usize,
) -> c_int {
    unsafe {
        if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
            if memory != core::ptr::null() && size > 0 {
                if png_image_read_init(image) != 0 {
                    /* Now set the IO functions to read from the memory buffer. */
                    (*(*image).opaque).memory = memory as png_const_bytep;
                    (*(*image).opaque).size = size;
                    (*(*(*image).opaque).png_ptr).io_ptr = image as png_voidp;
                    (*(*(*image).opaque).png_ptr).read_data_fn = Some(png_image_memory_read);

                    return png_safe_execute(image, Some(png_image_read_header), image as png_voidp);
                }
            } else {
                return png_image_error(
                    image,
                    c"png_image_begin_read_from_memory: invalid argument".as_ptr(),
                );
            }
        } else if image != core::ptr::null_mut() {
            return png_image_error(
                image,
                c"png_image_begin_read_from_memory: incorrect PNG_IMAGE_VERSION".as_ptr(),
            );
        }

        0
    }
}

/* Utility function to skip chunks that are not used by the simplified image
 * read functions.
 */
unsafe fn png_image_skip_unused_chunks(png_ptr: png_structrp) {
    unsafe {
        static chunks_to_process: [png_byte; 35] = [
            98, 75, 71, 68, b'\0', /* bKGD */
            99, 72, 82, 77, b'\0', /* cHRM */
            99, 73, 67, 80, b'\0', /* cICP */
            103, 65, 77, 65, b'\0', /* gAMA */
            109, 68, 67, 86, b'\0', /* mDCV */
            115, 66, 73, 84, b'\0', /* sBIT */
            115, 82, 71, 66, b'\0', /* sRGB */
        ];

        /* Ignore unknown chunks and all other chunks except for the
         * IHDR, PLTE, tRNS, IDAT, and IEND chunks.
         */
        png_set_keep_unknown_chunks(png_ptr, PNG_HANDLE_CHUNK_NEVER, core::ptr::null(), -1);

        /* But do not ignore image data handling chunks */
        png_set_keep_unknown_chunks(
            png_ptr,
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            chunks_to_process.as_ptr(),
            (core::mem::size_of_val(&chunks_to_process) / 5) as c_int,
        );
    }
}

/* PNG_SKIP_CHUNKS(p) */
unsafe fn PNG_SKIP_CHUNKS(p: png_structrp) {
    unsafe {
        png_image_skip_unused_chunks(p);
    }
}

/* The following gives the exact rounded answer for all values in the
 * range 0..255 (it actually divides by 51.2).
 */
#[inline]
fn PNG_DIV51(v8: png_uint_32) -> png_uint_32 {
    ((v8) * 5 + 130) >> 8
}

/* Utility functions to make particular color-maps */
unsafe fn set_file_encoding(display: *mut png_image_read_control) {
    unsafe {
        let png_ptr: png_structrp = (*(*(*display).image).opaque).png_ptr;
        let g: png_fixed_point = png_resolve_file_gamma(png_ptr);

        /* PNGv3: zero is an error */
        if g == 0 {
            png_error(png_ptr, c"internal: default gamma not set".as_ptr());
        }

        if png_gamma_significant(g) != 0 {
            if png_gamma_not_sRGB(g) != 0 {
                (*display).file_encoding = P_FILE;
                (*display).gamma_to_linear = png_reciprocal(g);
            } else {
                (*display).file_encoding = P_sRGB;
            }
        } else {
            (*display).file_encoding = P_LINEAR8;
        }
    }
}

unsafe fn decode_gamma(
    display: *mut png_image_read_control,
    mut value: png_uint_32,
    mut encoding: c_int,
) -> c_uint {
    unsafe {
        if encoding == P_FILE {
            /* double check */
            encoding = (*display).file_encoding;
        }

        if encoding == P_NOTSET {
            /* must be the file encoding */
            set_file_encoding(display);
            encoding = (*display).file_encoding;
        }

        match encoding {
            e if e == P_FILE => {
                value = png_gamma_16bit_correct(value * 257, (*display).gamma_to_linear)
                    as png_uint_32;
            }

            e if e == P_sRGB => {
                value = *png_sRGB_table.as_ptr().add(value as usize) as png_uint_32;
            }

            e if e == P_LINEAR => {}

            e if e == P_LINEAR8 => {
                value *= 257;
            }

            _ => {
                png_error(
                    (*(*(*display).image).opaque).png_ptr,
                    c"unexpected encoding (internal error)".as_ptr(),
                );
            }
        }

        value as c_uint
    }
}

unsafe fn png_colormap_compose(
    display: *mut png_image_read_control,
    foreground: png_uint_32,
    foreground_encoding: c_int,
    alpha: png_uint_32,
    background: png_uint_32,
    encoding: c_int,
) -> png_uint_32 {
    unsafe {
        let mut f: png_uint_32 = decode_gamma(display, foreground, foreground_encoding);
        let b: png_uint_32 = decode_gamma(display, background, encoding);

        /* The alpha is always an 8-bit value. */
        f = f * alpha + b * (255 - alpha);

        if encoding == P_LINEAR {
            /* Scale to 65535 */
            f *= 257; /* Now scaled by 65535 */
            f += f >> 16;
            f = (f + 32768) >> 16;
        } else {
            /* P_sRGB */
            f = PNG_sRGB_FROM_LINEAR(f) as png_uint_32;
        }

        f
    }
}

/* NOTE: P_LINEAR values to this routine must be 16-bit, but P_FILE values must
 * be 8-bit.
 */
unsafe fn png_create_colormap_entry(
    display: *mut png_image_read_control,
    ip: png_uint_32,
    mut red: png_uint_32,
    mut green: png_uint_32,
    mut blue: png_uint_32,
    mut alpha: png_uint_32,
    mut encoding: c_int,
) {
    unsafe {
        let image: png_imagep = (*display).image;
        let output_encoding: c_int = if ((*image).format & PNG_FORMAT_FLAG_LINEAR) != 0 {
            P_LINEAR
        } else {
            P_sRGB
        };
        let convert_to_Y: c_int = (((*image).format & PNG_FORMAT_FLAG_COLOR) == 0
            && (red != green || green != blue)) as c_int;

        if ip > 255 {
            png_error(
                (*(*image).opaque).png_ptr,
                c"color-map index out of range".as_ptr(),
            );
        }

        /* Update the cache with whether the file gamma is significantly different
         * from sRGB.
         */
        if encoding == P_FILE {
            if (*display).file_encoding == P_NOTSET {
                set_file_encoding(display);
            }

            encoding = (*display).file_encoding;
        }

        if encoding == P_FILE {
            let g: png_fixed_point = (*display).gamma_to_linear;

            red = png_gamma_16bit_correct(red * 257, g) as png_uint_32;
            green = png_gamma_16bit_correct(green * 257, g) as png_uint_32;
            blue = png_gamma_16bit_correct(blue * 257, g) as png_uint_32;

            if convert_to_Y != 0 || output_encoding == P_LINEAR {
                alpha *= 257;
                encoding = P_LINEAR;
            } else {
                red = PNG_sRGB_FROM_LINEAR(red * 255) as png_uint_32;
                green = PNG_sRGB_FROM_LINEAR(green * 255) as png_uint_32;
                blue = PNG_sRGB_FROM_LINEAR(blue * 255) as png_uint_32;
                encoding = P_sRGB;
            }
        } else if encoding == P_LINEAR8 {
            red *= 257;
            green *= 257;
            blue *= 257;
            alpha *= 257;
            encoding = P_LINEAR;
        } else if encoding == P_sRGB && (convert_to_Y != 0 || output_encoding == P_LINEAR) {
            /* The values are 8-bit sRGB values, converted to 16-bit linear. */
            red = *png_sRGB_table.as_ptr().add(red as usize) as png_uint_32;
            green = *png_sRGB_table.as_ptr().add(green as usize) as png_uint_32;
            blue = *png_sRGB_table.as_ptr().add(blue as usize) as png_uint_32;
            alpha *= 257;
            encoding = P_LINEAR;
        }

        /* This is set if the color isn't gray but the output is. */
        if encoding == P_LINEAR {
            if convert_to_Y != 0 {
                /* NOTE: these values are copied from png_do_rgb_to_gray */
                let mut y: png_uint_32 =
                    6968u32 * red + 23434u32 * green + 2366u32 * blue;

                if output_encoding == P_LINEAR {
                    y = (y + 16384) >> 15;
                } else {
                    /* y is scaled by 32768, we need it scaled by 255: */
                    y = (y + 128) >> 8;
                    y *= 255;
                    y = PNG_sRGB_FROM_LINEAR((y + 64) >> 7) as png_uint_32;
                    alpha = PNG_DIV257(alpha);
                    encoding = P_sRGB;
                }

                blue = y;
                red = y;
                green = y;
            } else if output_encoding == P_sRGB {
                red = PNG_sRGB_FROM_LINEAR(red * 255) as png_uint_32;
                green = PNG_sRGB_FROM_LINEAR(green * 255) as png_uint_32;
                blue = PNG_sRGB_FROM_LINEAR(blue * 255) as png_uint_32;
                alpha = PNG_DIV257(alpha);
                encoding = P_sRGB;
            }
        }

        if encoding != output_encoding {
            png_error(
                (*(*image).opaque).png_ptr,
                c"bad encoding (internal error)".as_ptr(),
            );
        }

        /* Store the value. */
        {
            let afirst: c_int = (((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0
                && ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0) as c_int;
            let bgr: c_int = if ((*image).format & PNG_FORMAT_FLAG_BGR) != 0 {
                2
            } else {
                0
            };

            if output_encoding == P_LINEAR {
                let mut entry: png_uint_16p = (*display).colormap as png_uint_16p;

                entry = entry.add((ip * PNG_IMAGE_SAMPLE_CHANNELS((*image).format)) as usize);

                /* The linear 16-bit values must be pre-multiplied by the alpha channel
                 * value, if less than 65535.
                 */
                match PNG_IMAGE_SAMPLE_CHANNELS((*image).format) {
                    4 => {
                        *entry.add(if afirst != 0 { 0 } else { 3 }) = alpha as png_uint_16;
                        /* FALLTHROUGH */
                        if alpha < 65535 {
                            if alpha > 0 {
                                blue = (blue * alpha + 32767u32) / 65535u32;
                                green = (green * alpha + 32767u32) / 65535u32;
                                red = (red * alpha + 32767u32) / 65535u32;
                            } else {
                                red = 0;
                                green = 0;
                                blue = 0;
                            }
                        }
                        *entry.add((afirst + (2 ^ bgr)) as usize) = blue as png_uint_16;
                        *entry.add((afirst + 1) as usize) = green as png_uint_16;
                        *entry.add((afirst + bgr) as usize) = red as png_uint_16;
                    }

                    3 => {
                        if alpha < 65535 {
                            if alpha > 0 {
                                blue = (blue * alpha + 32767u32) / 65535u32;
                                green = (green * alpha + 32767u32) / 65535u32;
                                red = (red * alpha + 32767u32) / 65535u32;
                            } else {
                                red = 0;
                                green = 0;
                                blue = 0;
                            }
                        }
                        *entry.add((afirst + (2 ^ bgr)) as usize) = blue as png_uint_16;
                        *entry.add((afirst + 1) as usize) = green as png_uint_16;
                        *entry.add((afirst + bgr) as usize) = red as png_uint_16;
                    }

                    2 => {
                        *entry.add((1 ^ afirst) as usize) = alpha as png_uint_16;
                        /* FALLTHROUGH */
                        if alpha < 65535 {
                            if alpha > 0 {
                                green = (green * alpha + 32767u32) / 65535u32;
                            } else {
                                green = 0;
                            }
                        }
                        *entry.add(afirst as usize) = green as png_uint_16;
                    }

                    1 => {
                        if alpha < 65535 {
                            if alpha > 0 {
                                green = (green * alpha + 32767u32) / 65535u32;
                            } else {
                                green = 0;
                            }
                        }
                        *entry.add(afirst as usize) = green as png_uint_16;
                    }

                    _ => {}
                }
            } else {
                /* output encoding is P_sRGB */
                let mut entry: png_bytep = (*display).colormap as png_bytep;

                entry = entry.add((ip * PNG_IMAGE_SAMPLE_CHANNELS((*image).format)) as usize);

                match PNG_IMAGE_SAMPLE_CHANNELS((*image).format) {
                    4 => {
                        *entry.add(if afirst != 0 { 0 } else { 3 }) = alpha as png_byte;
                        /* FALLTHROUGH */
                        *entry.add((afirst + (2 ^ bgr)) as usize) = blue as png_byte;
                        *entry.add((afirst + 1) as usize) = green as png_byte;
                        *entry.add((afirst + bgr) as usize) = red as png_byte;
                    }
                    3 => {
                        *entry.add((afirst + (2 ^ bgr)) as usize) = blue as png_byte;
                        *entry.add((afirst + 1) as usize) = green as png_byte;
                        *entry.add((afirst + bgr) as usize) = red as png_byte;
                    }

                    2 => {
                        *entry.add((1 ^ afirst) as usize) = alpha as png_byte;
                        /* FALLTHROUGH */
                        *entry.add(afirst as usize) = green as png_byte;
                    }
                    1 => {
                        *entry.add(afirst as usize) = green as png_byte;
                    }

                    _ => {}
                }
            }
        }
    }
}

unsafe fn make_gray_file_colormap(display: *mut png_image_read_control) -> c_int {
    unsafe {
        let mut i: c_uint = 0;

        while i < 256 {
            png_create_colormap_entry(display, i, i, i, i, 255, P_FILE);
            i += 1;
        }

        i as c_int
    }
}

unsafe fn make_gray_colormap(display: *mut png_image_read_control) -> c_int {
    unsafe {
        let mut i: c_uint = 0;

        while i < 256 {
            png_create_colormap_entry(display, i, i, i, i, 255, P_sRGB);
            i += 1;
        }

        i as c_int
    }
}

const PNG_GRAY_COLORMAP_ENTRIES: c_uint = 256;

unsafe fn make_ga_colormap(display: *mut png_image_read_control) -> c_int {
    unsafe {
        let mut i: c_uint;
        let mut a: c_uint;

        i = 0;
        while i < 231 {
            let gray: c_uint = (i * 256 + 115) / 231;
            png_create_colormap_entry(display, i, gray, gray, gray, 255, P_sRGB);
            i += 1;
        }

        /* 255 is used here for the component values for consistency with the code
         * that undoes premultiplication in pngwrite.c.
         */
        png_create_colormap_entry(display, i, 255, 255, 255, 0, P_sRGB);
        i += 1;

        a = 1;
        while a < 5 {
            let mut g: c_uint = 0;

            while g < 6 {
                png_create_colormap_entry(display, i, g * 51, g * 51, g * 51, a * 51, P_sRGB);
                i += 1;
                g += 1;
            }
            a += 1;
        }

        i as c_int
    }
}

const PNG_GA_COLORMAP_ENTRIES: c_uint = 256;

unsafe fn make_rgb_colormap(display: *mut png_image_read_control) -> c_int {
    unsafe {
        let mut i: c_uint;
        let mut r: c_uint;

        /* Build a 6x6x6 opaque RGB cube */
        i = 0;
        r = 0;
        while r < 6 {
            let mut g: c_uint = 0;

            while g < 6 {
                let mut b: c_uint = 0;

                while b < 6 {
                    png_create_colormap_entry(display, i, r * 51, g * 51, b * 51, 255, P_sRGB);
                    i += 1;
                    b += 1;
                }
                g += 1;
            }
            r += 1;
        }

        i as c_int
    }
}

const PNG_RGB_COLORMAP_ENTRIES: c_uint = 216;

/* Return a palette index to the above palette given three 8-bit sRGB values. */
#[inline]
fn PNG_RGB_INDEX(r: png_uint_32, g: png_uint_32, b: png_uint_32) -> png_byte {
    (6 * (6 * PNG_DIV51(r) + PNG_DIV51(g)) + PNG_DIV51(b)) as png_byte
}

unsafe extern "C-unwind" fn png_image_read_colormap(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_read_control = argument as *mut png_image_read_control;
        let image: png_imagep = (*display).image;

        let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
        let output_format: png_uint_32 = (*image).format;
        let output_encoding: c_int = if (output_format & PNG_FORMAT_FLAG_LINEAR) != 0 {
            P_LINEAR
        } else {
            P_sRGB
        };

        let mut cmap_entries: c_uint;
        let mut output_processing: c_uint; /* Output processing option */
        let mut data_encoding: c_int = P_NOTSET; /* Encoding libpng must produce */

        /* Background information */
        let mut background_index: c_uint = 256;
        let mut back_r: png_uint_32;
        let mut back_g: png_uint_32;
        let mut back_b: png_uint_32;

        /* Flags to accumulate things that need to be done to the input. */
        let mut expand_tRNS: c_int = 0;

        /* Exclude the NYI feature of compositing onto a color-mapped buffer. */
        if (((*png_ptr).color_type & PNG_COLOR_MASK_ALPHA as png_byte) != 0
            || (*png_ptr).num_trans > 0) /* alpha in input */
            && ((output_format & PNG_FORMAT_FLAG_ALPHA) == 0)
        /* no alpha in output */
        {
            if output_encoding == P_LINEAR {
                /* compose on black */
                back_b = 0;
                back_g = 0;
                back_r = 0;
            } else if (*display).background == core::ptr::null() {
                /* no way to remove it */
                png_error(
                    png_ptr,
                    c"background color must be supplied to remove alpha/transparency".as_ptr(),
                );
            }
            /* Get a copy of the background color. */
            else {
                back_g = (*(*display).background).green as png_uint_32;
                if (output_format & PNG_FORMAT_FLAG_COLOR) != 0 {
                    back_r = (*(*display).background).red as png_uint_32;
                    back_b = (*(*display).background).blue as png_uint_32;
                } else {
                    back_b = back_g;
                    back_r = back_g;
                }
            }
        } else if output_encoding == P_LINEAR {
            back_b = 65535;
            back_r = 65535;
            back_g = 65535;
        } else {
            back_b = 255;
            back_r = 255;
            back_g = 255;
        }

        /* Default the input file gamma if required. */
        if (*png_ptr).bit_depth == 16 && ((*image).flags & PNG_IMAGE_FLAG_16BIT_sRGB) == 0 {
            (*png_ptr).default_gamma = PNG_GAMMA_LINEAR;
        } else {
            (*png_ptr).default_gamma = PNG_GAMMA_sRGB_INVERSE;
        }

        /* Decide what to do based on the PNG color type of the input data. */
        'color_switch: {
        match (*png_ptr).color_type as c_int {
            PNG_COLOR_TYPE_GRAY => {
                if (*png_ptr).bit_depth <= 8 {
                    /* There at most 256 colors in the output. */
                    let step: c_uint;
                    let mut i: c_uint;
                    let mut val: c_uint;
                    let mut trans: c_uint = 256; /*ignore*/
                    let mut back_alpha: c_uint = 0;

                    cmap_entries = 1u32 << (*png_ptr).bit_depth;
                    if cmap_entries > (*image).colormap_entries {
                        png_error(png_ptr, c"gray[8] color-map: too few entries".as_ptr());
                    }

                    step = 255 / (cmap_entries - 1);
                    output_processing = PNG_CMAP_NONE as c_uint;

                    /* If there is a tRNS chunk then this either selects a transparent
                     * value or the background color.
                     */
                    if (*png_ptr).num_trans > 0 {
                        trans = (*png_ptr).trans_color.gray as c_uint;

                        if (output_format & PNG_FORMAT_FLAG_ALPHA) == 0 {
                            back_alpha = if output_encoding == P_LINEAR { 65535 } else { 255 };
                        }
                    }

                    i = 0;
                    val = 0;
                    while i < cmap_entries {
                        /* 'i' is a file value. */
                        if i != trans {
                            png_create_colormap_entry(
                                display, i, val, val, val, 255,
                                P_FILE, /*8-bit with file gamma*/
                            );
                        } else {
                            png_create_colormap_entry(
                                display,
                                i,
                                back_r,
                                back_g,
                                back_b,
                                back_alpha,
                                output_encoding,
                            );
                        }
                        i += 1;
                        val += step;
                    }

                    /* We need libpng to preserve the original encoding. */
                    data_encoding = P_FILE;

                    /* The rows may need to be expanded to 1 byte per pixel. */
                    if (*png_ptr).bit_depth < 8 {
                        png_set_packing(png_ptr);
                    }
                } else {
                    /* bit depth is 16 */
                    data_encoding = P_sRGB;

                    if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                        png_error(png_ptr, c"gray[16] color-map: too few entries".as_ptr());
                    }

                    cmap_entries = make_gray_colormap(display) as c_uint;

                    if (*png_ptr).num_trans > 0 {
                        let back_alpha: c_uint;

                        if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                            back_alpha = 0;
                        } else {
                            if back_r == back_g && back_g == back_b {
                                /* Background is gray; no special processing required. */
                                let mut c: png_color_16 = png_color_16::default();
                                let mut gray: png_uint_32 = back_g;

                                if output_encoding == P_LINEAR {
                                    gray = PNG_sRGB_FROM_LINEAR(gray * 255) as png_uint_32;

                                    png_create_colormap_entry(
                                        display, gray, back_g, back_g, back_g, 65535, P_LINEAR,
                                    );
                                }

                                c.index = 0; /*unused*/
                                c.blue = gray as png_uint_16;
                                c.green = c.blue;
                                c.red = c.green;
                                c.gray = c.red;

                                png_set_background_fixed(
                                    png_ptr,
                                    &raw mut c,
                                    PNG_BACKGROUND_GAMMA_SCREEN,
                                    0, /*need_expand*/
                                    0, /*gamma: not used*/
                                );

                                output_processing = PNG_CMAP_NONE as c_uint;
                                break 'color_switch;
                            }

                            back_alpha = if output_encoding == P_LINEAR { 65535 } else { 255 };
                        }

                        /* output_processing means the libpng row will be 8-bit GA. */
                        expand_tRNS = 1;
                        output_processing = PNG_CMAP_TRANS as c_uint;
                        background_index = 254;

                        /* Overwrite color-map entry 254 to the actual background color. */
                        png_create_colormap_entry(
                            display, 254, back_r, back_g, back_b, back_alpha, output_encoding,
                        );
                    } else {
                        output_processing = PNG_CMAP_NONE as c_uint;
                    }
                }
            }

            PNG_COLOR_TYPE_GRAY_ALPHA => {
                data_encoding = P_sRGB;

                if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                        png_error(png_ptr, c"gray+alpha color-map: too few entries".as_ptr());
                    }

                    cmap_entries = make_ga_colormap(display) as c_uint;

                    background_index = PNG_CMAP_GA_BACKGROUND;
                    output_processing = PNG_CMAP_GA as c_uint;
                } else {
                    /* alpha is removed */
                    if (output_format & PNG_FORMAT_FLAG_COLOR) == 0
                        || (back_r == back_g && back_g == back_b)
                    {
                        /* Background is gray; no special processing required. */
                        let mut c: png_color_16 = png_color_16::default();
                        let mut gray: png_uint_32 = back_g;

                        if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, c"gray-alpha color-map: too few entries".as_ptr());
                        }

                        cmap_entries = make_gray_colormap(display) as c_uint;

                        if output_encoding == P_LINEAR {
                            gray = PNG_sRGB_FROM_LINEAR(gray * 255) as png_uint_32;

                            png_create_colormap_entry(
                                display, gray, back_g, back_g, back_g, 65535, P_LINEAR,
                            );
                        }

                        c.index = 0; /*unused*/
                        c.blue = gray as png_uint_16;
                        c.green = c.blue;
                        c.red = c.green;
                        c.gray = c.red;

                        png_set_background_fixed(
                            png_ptr,
                            &raw mut c,
                            PNG_BACKGROUND_GAMMA_SCREEN,
                            0, /*need_expand*/
                            0, /*gamma: not used*/
                        );

                        output_processing = PNG_CMAP_NONE as c_uint;
                    } else {
                        let mut i: png_uint_32;
                        let mut a: png_uint_32;

                        /* This is the same as png_make_ga_colormap but opaque. */
                        if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, c"ga-alpha color-map: too few entries".as_ptr());
                        }

                        i = 0;
                        while i < 231 {
                            let gray: png_uint_32 = (i * 256 + 115) / 231;
                            png_create_colormap_entry(display, i, gray, gray, gray, 255, P_sRGB);
                            i += 1;
                        }

                        /* NOTE: preserves full precision of the background color. */
                        background_index = i;
                        png_create_colormap_entry(
                            display,
                            i,
                            back_r,
                            back_g,
                            back_b,
                            if output_encoding == P_LINEAR { 65535u32 } else { 255u32 },
                            output_encoding,
                        );
                        i += 1;

                        /* For non-opaque input composite on the sRGB background. */
                        if output_encoding == P_sRGB {
                            /* else already linear */
                            back_r = *png_sRGB_table.as_ptr().add(back_r as usize) as png_uint_32;
                            back_g = *png_sRGB_table.as_ptr().add(back_g as usize) as png_uint_32;
                            back_b = *png_sRGB_table.as_ptr().add(back_b as usize) as png_uint_32;
                        }

                        a = 1;
                        while a < 5 {
                            let mut g: c_uint;

                            let alpha: png_uint_32 = 51 * a;
                            let back_rx: png_uint_32 = (255 - alpha) * back_r;
                            let back_gx: png_uint_32 = (255 - alpha) * back_g;
                            let back_bx: png_uint_32 = (255 - alpha) * back_b;

                            g = 0;
                            while g < 6 {
                                let gray: png_uint_32 =
                                    *png_sRGB_table.as_ptr().add((g * 51) as usize) as png_uint_32
                                        * alpha;

                                png_create_colormap_entry(
                                    display,
                                    i,
                                    PNG_sRGB_FROM_LINEAR(gray + back_rx) as png_uint_32,
                                    PNG_sRGB_FROM_LINEAR(gray + back_gx) as png_uint_32,
                                    PNG_sRGB_FROM_LINEAR(gray + back_bx) as png_uint_32,
                                    255,
                                    P_sRGB,
                                );
                                i += 1;
                                g += 1;
                            }
                            a += 1;
                        }

                        cmap_entries = i;
                        output_processing = PNG_CMAP_GA as c_uint;
                    }
                }
            }

            PNG_COLOR_TYPE_RGB | PNG_COLOR_TYPE_RGB_ALPHA => {
                /* Exclude the case where the output is gray. */
                if (output_format & PNG_FORMAT_FLAG_COLOR) == 0 {
                    png_set_rgb_to_gray_fixed(png_ptr, PNG_ERROR_ACTION_NONE, -1, -1);
                    data_encoding = P_sRGB;

                    /* The more complex case arises when the input has alpha. */
                    if ((*png_ptr).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte
                        || (*png_ptr).num_trans > 0)
                        && (output_format & PNG_FORMAT_FLAG_ALPHA) != 0
                    {
                        expand_tRNS = 1;

                        if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, c"rgb[ga] color-map: too few entries".as_ptr());
                        }

                        cmap_entries = make_ga_colormap(display) as c_uint;
                        background_index = PNG_CMAP_GA_BACKGROUND;
                        output_processing = PNG_CMAP_GA as c_uint;
                    } else {
                        let gamma: png_fixed_point = png_resolve_file_gamma(png_ptr);

                        if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, c"rgb[gray] color-map: too few entries".as_ptr());
                        }

                        if ((*png_ptr).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte
                            || (*png_ptr).num_trans > 0)
                            && png_gamma_not_sRGB(gamma) != 0
                        {
                            cmap_entries = make_gray_file_colormap(display) as c_uint;
                            data_encoding = P_FILE;
                        } else {
                            cmap_entries = make_gray_colormap(display) as c_uint;
                        }

                        /* But if the input has alpha or transparency it must be removed */
                        if (*png_ptr).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte
                            || (*png_ptr).num_trans > 0
                        {
                            let mut c: png_color_16 = png_color_16::default();
                            let mut gray: png_uint_32 = back_g;

                            if data_encoding == P_FILE {
                                /* from the fixup above */
                                if output_encoding == P_sRGB {
                                    gray = *png_sRGB_table.as_ptr().add(gray as usize)
                                        as png_uint_32; /* now P_LINEAR */
                                }

                                gray = PNG_DIV257(
                                    png_gamma_16bit_correct(gray, gamma) as png_uint_32,
                                );
                                /* now P_FILE */

                                png_create_colormap_entry(
                                    display, gray, back_g, back_g, back_g, 0, /*unused*/
                                    output_encoding,
                                );
                            } else if output_encoding == P_LINEAR {
                                gray = PNG_sRGB_FROM_LINEAR(gray * 255) as png_uint_32;

                                png_create_colormap_entry(
                                    display, gray, back_g, back_g, back_g, 0, /*unused*/
                                    P_LINEAR,
                                );
                            }

                            c.index = 0; /*unused*/
                            c.blue = gray as png_uint_16;
                            c.green = c.blue;
                            c.red = c.green;
                            c.gray = c.red;

                            /* NOTE: apparently a bug in libpng. */
                            expand_tRNS = 1;
                            png_set_background_fixed(
                                png_ptr,
                                &raw mut c,
                                PNG_BACKGROUND_GAMMA_SCREEN,
                                0, /*need_expand*/
                                0, /*gamma: not used*/
                            );
                        }

                        output_processing = PNG_CMAP_NONE as c_uint;
                    }
                } else {
                    /* output is color */
                    data_encoding = P_sRGB;

                    /* Is there any transparency or alpha? */
                    if (*png_ptr).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte
                        || (*png_ptr).num_trans > 0
                    {
                        /* Is there alpha in the output too? */
                        if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                            let mut r: png_uint_32;

                            if PNG_RGB_COLORMAP_ENTRIES + 1 + 27 > (*image).colormap_entries {
                                png_error(
                                    png_ptr,
                                    c"rgb+alpha color-map: too few entries".as_ptr(),
                                );
                            }

                            cmap_entries = make_rgb_colormap(display) as c_uint;

                            /* Add a transparent entry. */
                            png_create_colormap_entry(
                                display, cmap_entries, 255, 255, 255, 0, P_sRGB,
                            );

                            /* This is stored as the background index. */
                            background_index = cmap_entries;
                            cmap_entries += 1;

                            /* Add 27 r,g,b entries each with alpha 0.5. */
                            r = 0;
                            while r < 256 {
                                let mut g: png_uint_32 = 0;

                                while g < 256 {
                                    let mut b: png_uint_32 = 0;

                                    while b < 256 {
                                        png_create_colormap_entry(
                                            display, cmap_entries, r, g, b, 128, P_sRGB,
                                        );
                                        cmap_entries += 1;
                                        b = (b << 1) | 0x7f;
                                    }
                                    g = (g << 1) | 0x7f;
                                }
                                r = (r << 1) | 0x7f;
                            }

                            expand_tRNS = 1;
                            output_processing = PNG_CMAP_RGB_ALPHA as c_uint;
                        } else {
                            /* Alpha/transparency must be removed. */
                            let sample_size: c_uint = PNG_IMAGE_SAMPLE_SIZE(output_format);
                            let r: png_uint_32;
                            let g: png_uint_32;
                            let b: png_uint_32;

                            if PNG_RGB_COLORMAP_ENTRIES + 1 + 27 > (*image).colormap_entries {
                                png_error(
                                    png_ptr,
                                    c"rgb-alpha color-map: too few entries".as_ptr(),
                                );
                            }

                            cmap_entries = make_rgb_colormap(display) as c_uint;

                            png_create_colormap_entry(
                                display, cmap_entries, back_r, back_g, back_b, 0, /*unused*/
                                output_encoding,
                            );

                            if output_encoding == P_LINEAR {
                                r = PNG_sRGB_FROM_LINEAR(back_r * 255) as png_uint_32;
                                g = PNG_sRGB_FROM_LINEAR(back_g * 255) as png_uint_32;
                                b = PNG_sRGB_FROM_LINEAR(back_b * 255) as png_uint_32;
                            } else {
                                r = back_r;
                                g = back_g;
                                b = back_b;
                            }

                            /* Compare the newly-created entry with the one PNG_CMAP_RGB
                             * will use.
                             */
                            if memcmp(
                                ((*display).colormap as png_const_bytep)
                                    .add((sample_size * cmap_entries) as usize)
                                    as png_const_voidp,
                                ((*display).colormap as png_const_bytep).add(
                                    (sample_size * PNG_RGB_INDEX(r, g, b) as png_uint_32) as usize,
                                ) as png_const_voidp,
                                sample_size as usize,
                            ) != 0
                            {
                                /* The background color must be added. */
                                background_index = cmap_entries;
                                cmap_entries += 1;

                                /* Add 27 r,g,b entries composed with background at alpha 0.5. */
                                let mut rr: png_uint_32 = 0;
                                while rr < 256 {
                                    let mut gg: png_uint_32 = 0;
                                    while gg < 256 {
                                        let mut bb: png_uint_32 = 0;
                                        while bb < 256 {
                                            png_create_colormap_entry(
                                                display,
                                                cmap_entries,
                                                png_colormap_compose(
                                                    display,
                                                    rr,
                                                    P_sRGB,
                                                    128,
                                                    back_r,
                                                    output_encoding,
                                                ),
                                                png_colormap_compose(
                                                    display,
                                                    gg,
                                                    P_sRGB,
                                                    128,
                                                    back_g,
                                                    output_encoding,
                                                ),
                                                png_colormap_compose(
                                                    display,
                                                    bb,
                                                    P_sRGB,
                                                    128,
                                                    back_b,
                                                    output_encoding,
                                                ),
                                                0, /*unused*/
                                                output_encoding,
                                            );
                                            cmap_entries += 1;
                                            bb = (bb << 1) | 0x7f;
                                        }
                                        gg = (gg << 1) | 0x7f;
                                    }
                                    rr = (rr << 1) | 0x7f;
                                }

                                expand_tRNS = 1;
                                output_processing = PNG_CMAP_RGB_ALPHA as c_uint;
                            } else {
                                /* background color is in the standard color-map */
                                let mut c: png_color_16 = png_color_16::default();

                                c.index = 0; /*unused*/
                                c.red = back_r as png_uint_16;
                                c.green = back_g as png_uint_16;
                                c.gray = c.green;
                                c.blue = back_b as png_uint_16;

                                png_set_background_fixed(
                                    png_ptr,
                                    &raw mut c,
                                    PNG_BACKGROUND_GAMMA_SCREEN,
                                    0, /*need_expand*/
                                    0, /*gamma: not used*/
                                );

                                output_processing = PNG_CMAP_RGB as c_uint;
                            }
                        }
                    } else {
                        /* no alpha or transparency in the input */
                        if PNG_RGB_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, c"rgb color-map: too few entries".as_ptr());
                        }

                        cmap_entries = make_rgb_colormap(display) as c_uint;
                        output_processing = PNG_CMAP_RGB as c_uint;
                    }
                }
            }

            PNG_COLOR_TYPE_PALETTE => {
                /* It's already got a color-map. */
                let mut num_trans: c_uint = (*png_ptr).num_trans as c_uint;
                let trans: png_const_bytep = if num_trans > 0 {
                    (*png_ptr).trans_alpha
                } else {
                    core::ptr::null()
                };
                let colormap: png_const_colorp = (*png_ptr).palette;
                let do_background: c_int = (trans != core::ptr::null()
                    && (output_format & PNG_FORMAT_FLAG_ALPHA) == 0)
                    as c_int;
                let mut i: c_uint;

                /* Just in case: */
                if trans == core::ptr::null() {
                    num_trans = 0;
                }

                output_processing = PNG_CMAP_NONE as c_uint;
                data_encoding = P_FILE; /* Don't change from color-map indices */
                cmap_entries = (*png_ptr).num_palette as c_uint;
                if cmap_entries > 256 {
                    cmap_entries = 256;
                }

                if cmap_entries > (*image).colormap_entries {
                    png_error(png_ptr, c"palette color-map: too few entries".as_ptr());
                }

                i = 0;
                while i < cmap_entries {
                    if do_background != 0
                        && i < num_trans
                        && *trans.add(i as usize) < 255
                    {
                        if *trans.add(i as usize) == 0 {
                            png_create_colormap_entry(
                                display, i, back_r, back_g, back_b, 0, output_encoding,
                            );
                        } else {
                            /* Must compose the PNG file color on the sRGB color. */
                            png_create_colormap_entry(
                                display,
                                i,
                                png_colormap_compose(
                                    display,
                                    (*colormap.add(i as usize)).red as png_uint_32,
                                    P_FILE,
                                    *trans.add(i as usize) as png_uint_32,
                                    back_r,
                                    output_encoding,
                                ),
                                png_colormap_compose(
                                    display,
                                    (*colormap.add(i as usize)).green as png_uint_32,
                                    P_FILE,
                                    *trans.add(i as usize) as png_uint_32,
                                    back_g,
                                    output_encoding,
                                ),
                                png_colormap_compose(
                                    display,
                                    (*colormap.add(i as usize)).blue as png_uint_32,
                                    P_FILE,
                                    *trans.add(i as usize) as png_uint_32,
                                    back_b,
                                    output_encoding,
                                ),
                                if output_encoding == P_LINEAR {
                                    *trans.add(i as usize) as png_uint_32 * 257u32
                                } else {
                                    *trans.add(i as usize) as png_uint_32
                                },
                                output_encoding,
                            );
                        }
                    } else {
                        png_create_colormap_entry(
                            display,
                            i,
                            (*colormap.add(i as usize)).red as png_uint_32,
                            (*colormap.add(i as usize)).green as png_uint_32,
                            (*colormap.add(i as usize)).blue as png_uint_32,
                            if i < num_trans {
                                *trans.add(i as usize) as png_uint_32
                            } else {
                                255u32
                            },
                            P_FILE, /*8-bit*/
                        );
                    }
                    i += 1;
                }

                /* The PNG data may have indices packed in fewer than 8 bits. */
                if (*png_ptr).bit_depth < 8 {
                    png_set_packing(png_ptr);
                }
            }

            _ => {
                png_error(png_ptr, c"invalid PNG color type".as_ptr());
                /*NOT REACHED*/
            }
        }
        } /* end 'color_switch */

        /* Now deal with the output processing */
        if expand_tRNS != 0
            && (*png_ptr).num_trans > 0
            && ((*png_ptr).color_type & PNG_COLOR_MASK_ALPHA as png_byte) == 0
        {
            png_set_tRNS_to_alpha(png_ptr);
        }

        match data_encoding {
            e if e == P_sRGB => {
                /* Change to 8-bit sRGB */
                png_set_alpha_mode_fixed(png_ptr, PNG_ALPHA_PNG, PNG_GAMMA_sRGB);
                /* FALLTHROUGH */
                if (*png_ptr).bit_depth > 8 {
                    png_set_scale_16(png_ptr);
                }
            }

            e if e == P_FILE => {
                if (*png_ptr).bit_depth > 8 {
                    png_set_scale_16(png_ptr);
                }
            }

            _ => {
                png_error(png_ptr, c"bad data option (internal error)".as_ptr());
            }
        }

        if cmap_entries > 256 || cmap_entries > (*image).colormap_entries {
            png_error(png_ptr, c"color map overflow (BAD internal error)".as_ptr());
        }

        (*image).colormap_entries = cmap_entries;

        /* Double check using the recorded background index */
        'bad_background: {
            match output_processing as c_int {
                PNG_CMAP_NONE => {
                    if background_index != PNG_CMAP_NONE_BACKGROUND {
                        break 'bad_background;
                    }
                }

                PNG_CMAP_GA => {
                    if background_index != PNG_CMAP_GA_BACKGROUND {
                        break 'bad_background;
                    }
                }

                PNG_CMAP_TRANS => {
                    if background_index >= cmap_entries
                        || background_index != PNG_CMAP_TRANS_BACKGROUND
                    {
                        break 'bad_background;
                    }
                }

                PNG_CMAP_RGB => {
                    if background_index != PNG_CMAP_RGB_BACKGROUND {
                        break 'bad_background;
                    }
                }

                PNG_CMAP_RGB_ALPHA => {
                    if background_index != PNG_CMAP_RGB_ALPHA_BACKGROUND {
                        break 'bad_background;
                    }
                }

                _ => {
                    png_error(png_ptr, c"bad processing option (internal error)".as_ptr());
                }
            }

            (*display).colormap_processing = output_processing as c_int;

            return 1; /*ok*/
        }

        /* bad_background: */
        png_error(png_ptr, c"bad background index (internal error)".as_ptr());
    }
}

/* The final part of the color-map read called from png_image_finish_read. */
unsafe extern "C-unwind" fn png_image_read_and_map(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_read_control = argument as *mut png_image_read_control;
        let image: png_imagep = (*display).image;
        let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
        let passes: c_int;

        /* Called when the libpng data must be transformed into the color-mapped
         * form.
         */
        match (*png_ptr).interlaced as c_int {
            PNG_INTERLACE_NONE => {
                passes = 1;
            }

            PNG_INTERLACE_ADAM7 => {
                passes = PNG_INTERLACE_ADAM7_PASSES as c_int;
            }

            _ => {
                png_error(png_ptr, c"unknown interlace type".as_ptr());
            }
        }

        {
            let height: png_uint_32 = (*image).height;
            let width: png_uint_32 = (*image).width;
            let proc: c_int = (*display).colormap_processing;
            let first_row: png_bytep = (*display).first_row as png_bytep;
            let row_step: isize = (*display).row_step;
            let mut pass: c_int;

            pass = 0;
            while pass < passes {
                let startx: c_uint;
                let stepx: c_uint;
                let stepy: c_uint;
                let mut y: png_uint_32;

                if (*png_ptr).interlaced == PNG_INTERLACE_ADAM7 as png_byte {
                    /* The row may be empty for a short image: */
                    if PNG_PASS_COLS(width, pass) == 0 {
                        pass += 1;
                        continue;
                    }

                    startx = PNG_PASS_START_COL(pass) as c_uint;
                    stepx = PNG_PASS_COL_OFFSET(pass) as c_uint;
                    y = PNG_PASS_START_ROW(pass) as png_uint_32;
                    stepy = PNG_PASS_ROW_OFFSET(pass) as c_uint;
                } else {
                    y = 0;
                    startx = 0;
                    stepx = 1;
                    stepy = 1;
                }

                while y < height {
                    let mut inrow: png_bytep = (*display).local_row as png_bytep;
                    let mut outrow: png_bytep = first_row.offset(y as isize * row_step);
                    let row_end: png_const_bytep = outrow.add(width as usize);

                    /* Read the libpng data into the temporary buffer. */
                    png_read_row(png_ptr, inrow, core::ptr::null_mut());

                    /* Now process the row according to the processing option. */
                    outrow = outrow.add(startx as usize);
                    match proc {
                        p if p == PNG_CMAP_GA => {
                            while (outrow as *const png_byte) < row_end {
                                /* The data is always in the PNG order */
                                let gray: c_uint = *inrow as c_uint;
                                inrow = inrow.add(1);
                                let alpha: c_uint = *inrow as c_uint;
                                inrow = inrow.add(1);
                                let entry: c_uint;

                                if alpha > 229 {
                                    /* opaque */
                                    entry = (231 * gray + 128) >> 8;
                                } else if alpha < 26 {
                                    /* transparent */
                                    entry = 231;
                                } else {
                                    /* partially opaque */
                                    entry = 226 + 6 * PNG_DIV51(alpha) + PNG_DIV51(gray);
                                }

                                *outrow = entry as png_byte;
                                outrow = outrow.add(stepx as usize);
                            }
                        }

                        p if p == PNG_CMAP_TRANS => {
                            while (outrow as *const png_byte) < row_end {
                                let gray: png_byte = *inrow;
                                inrow = inrow.add(1);
                                let alpha: png_byte = *inrow;
                                inrow = inrow.add(1);

                                if alpha == 0 {
                                    *outrow = PNG_CMAP_TRANS_BACKGROUND as png_byte;
                                } else if gray != PNG_CMAP_TRANS_BACKGROUND as png_byte {
                                    *outrow = gray;
                                } else {
                                    *outrow = (PNG_CMAP_TRANS_BACKGROUND + 1) as png_byte;
                                }
                                outrow = outrow.add(stepx as usize);
                            }
                        }

                        p if p == PNG_CMAP_RGB => {
                            while (outrow as *const png_byte) < row_end {
                                *outrow = PNG_RGB_INDEX(
                                    *inrow.add(0) as png_uint_32,
                                    *inrow.add(1) as png_uint_32,
                                    *inrow.add(2) as png_uint_32,
                                );
                                inrow = inrow.add(3);
                                outrow = outrow.add(stepx as usize);
                            }
                        }

                        p if p == PNG_CMAP_RGB_ALPHA => {
                            while (outrow as *const png_byte) < row_end {
                                let alpha: c_uint = *inrow.add(3) as c_uint;

                                if alpha >= 196 {
                                    *outrow = PNG_RGB_INDEX(
                                        *inrow.add(0) as png_uint_32,
                                        *inrow.add(1) as png_uint_32,
                                        *inrow.add(2) as png_uint_32,
                                    );
                                } else if alpha < 64 {
                                    *outrow = PNG_CMAP_RGB_ALPHA_BACKGROUND as png_byte;
                                } else {
                                    let mut back_i: c_uint = PNG_CMAP_RGB_ALPHA_BACKGROUND + 1;

                                    if *inrow.add(0) & 0x80 != 0 {
                                        back_i += 9;
                                    } /* red */
                                    if *inrow.add(0) & 0x40 != 0 {
                                        back_i += 9;
                                    }
                                    if *inrow.add(1) & 0x80 != 0 {
                                        back_i += 3;
                                    } /* green */
                                    if *inrow.add(1) & 0x40 != 0 {
                                        back_i += 3;
                                    }
                                    if *inrow.add(2) & 0x80 != 0 {
                                        back_i += 1;
                                    } /* blue */
                                    if *inrow.add(2) & 0x40 != 0 {
                                        back_i += 1;
                                    }

                                    *outrow = back_i as png_byte;
                                }

                                inrow = inrow.add(4);
                                outrow = outrow.add(stepx as usize);
                            }
                        }

                        _ => {}
                    }
                    y += stepy;
                }
                pass += 1;
            }
        }

        1
    }
}

unsafe extern "C-unwind" fn png_image_read_colormapped(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_read_control = argument as *mut png_image_read_control;
        let image: png_imagep = (*display).image;
        let control: png_controlp = (*image).opaque;
        let png_ptr: png_structrp = (*control).png_ptr;
        let info_ptr: png_inforp = (*control).info_ptr;

        let mut passes: c_int = 0; /* As a flag */

        PNG_SKIP_CHUNKS(png_ptr);

        /* Update the 'info' structure. */
        if (*display).colormap_processing == PNG_CMAP_NONE {
            passes = png_set_interlace_handling(png_ptr);
        }

        png_read_update_info(png_ptr, info_ptr);

        /* The expected output can be deduced from the colormap_processing option. */
        'bad_output: {
            match (*display).colormap_processing {
                PNG_CMAP_NONE => {
                    /* Output must be one channel and one byte per pixel. */
                    if ((*info_ptr).color_type == PNG_COLOR_TYPE_PALETTE as png_byte
                        || (*info_ptr).color_type == PNG_COLOR_TYPE_GRAY as png_byte)
                        && (*info_ptr).bit_depth == 8
                    {
                        break 'bad_output;
                    }
                    /* goto bad_output */
                    png_error(png_ptr, c"bad color-map processing (internal error)".as_ptr());
                }

                PNG_CMAP_TRANS | PNG_CMAP_GA => {
                    /* Output must be two channels and the 'G' one must be sRGB. */
                    if (*info_ptr).color_type == PNG_COLOR_TYPE_GRAY_ALPHA as png_byte
                        && (*info_ptr).bit_depth == 8
                        && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                        && (*image).colormap_entries == 256
                    {
                        break 'bad_output;
                    }
                    png_error(png_ptr, c"bad color-map processing (internal error)".as_ptr());
                }

                PNG_CMAP_RGB => {
                    /* Output must be 8-bit sRGB encoded RGB */
                    if (*info_ptr).color_type == PNG_COLOR_TYPE_RGB as png_byte
                        && (*info_ptr).bit_depth == 8
                        && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                        && (*image).colormap_entries == 216
                    {
                        break 'bad_output;
                    }
                    png_error(png_ptr, c"bad color-map processing (internal error)".as_ptr());
                }

                PNG_CMAP_RGB_ALPHA => {
                    /* Output must be 8-bit sRGB encoded RGBA */
                    if (*info_ptr).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte
                        && (*info_ptr).bit_depth == 8
                        && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                        && (*image).colormap_entries == 244
                    /* 216 + 1 + 27 */
                    {
                        break 'bad_output;
                    }
                    png_error(png_ptr, c"bad color-map processing (internal error)".as_ptr());
                }

                _ => {
                    /* bad_output: */
                    png_error(png_ptr, c"bad color-map processing (internal error)".as_ptr());
                }
            }
        }

        /* Now read the rows. */
        {
            let mut first_row: png_voidp = (*display).buffer;
            let row_step: isize = (*display).row_stride as isize;

            /* Ensure calculations are correct regardless of the sign of row_step. */
            if row_step < 0 {
                let mut ptr: *mut c_char = first_row as *mut c_char;
                ptr = ptr.offset(((*image).height - 1) as isize * (-row_step));
                first_row = ptr as png_voidp;
            }

            (*display).first_row = first_row;
            (*display).row_step = row_step;
        }

        if passes == 0 {
            let result: c_int;
            let row: png_voidp = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr));

            (*display).local_row = row;
            result = png_safe_execute(image, Some(png_image_read_and_map), display as png_voidp);
            (*display).local_row = core::ptr::null_mut();
            png_free(png_ptr, row);

            result
        } else {
            let row_step: isize = (*display).row_step;

            loop {
                passes -= 1;
                if passes < 0 {
                    break;
                }
                let mut y: png_uint_32 = (*image).height;
                let mut row: png_bytep = (*display).first_row as png_bytep;

                while y > 0 {
                    png_read_row(png_ptr, row, core::ptr::null_mut());
                    row = row.offset(row_step);
                    y -= 1;
                }
            }

            1
        }
    }
}

/* Row reading for interlaced 16-to-8 bit depth conversion with local buffer. */
unsafe extern "C-unwind" fn png_image_read_direct_scaled(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_read_control = argument as *mut png_image_read_control;
        let image: png_imagep = (*display).image;
        let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
        let info_ptr: png_inforp = (*(*image).opaque).info_ptr;
        let local_row: png_bytep = (*display).local_row as png_bytep;
        let first_row: png_bytep = (*display).first_row as png_bytep;
        let row_step: isize = (*display).row_step;
        let row_bytes: usize = png_get_rowbytes(png_ptr, info_ptr);
        let mut passes: c_int;

        /* Handle interlacing. */
        match (*png_ptr).interlaced as c_int {
            PNG_INTERLACE_NONE => {
                passes = 1;
            }

            PNG_INTERLACE_ADAM7 => {
                passes = PNG_INTERLACE_ADAM7_PASSES as c_int;
            }

            _ => {
                png_error(png_ptr, c"unknown interlace type".as_ptr());
            }
        }

        /* Read each pass using local_row as intermediate buffer. */
        loop {
            passes -= 1;
            if passes < 0 {
                break;
            }
            let mut y: png_uint_32 = (*image).height;
            let mut output_row: png_bytep = first_row;

            while y > 0 {
                /* Read into local_row (gets transformed 8-bit data). */
                png_read_row(png_ptr, local_row, core::ptr::null_mut());

                /* Copy from local_row to user buffer. */
                memcpy(
                    output_row as png_voidp,
                    local_row as png_const_voidp,
                    row_bytes,
                );
                output_row = output_row.offset(row_step);
                y -= 1;
            }
        }

        1
    }
}

/* Just the row reading part of png_image_read. */
unsafe extern "C-unwind" fn png_image_read_composite(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_read_control = argument as *mut png_image_read_control;
        let image: png_imagep = (*display).image;
        let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
        let passes: c_int;

        match (*png_ptr).interlaced as c_int {
            PNG_INTERLACE_NONE => {
                passes = 1;
            }

            PNG_INTERLACE_ADAM7 => {
                passes = PNG_INTERLACE_ADAM7_PASSES as c_int;
            }

            _ => {
                png_error(png_ptr, c"unknown interlace type".as_ptr());
            }
        }

        {
            let height: png_uint_32 = (*image).height;
            let width: png_uint_32 = (*image).width;
            let row_step: isize = (*display).row_step;
            let channels: c_uint = if ((*image).format & PNG_FORMAT_FLAG_COLOR) != 0 {
                3
            } else {
                1
            };
            let optimize_alpha: c_int = ((*png_ptr).flags & PNG_FLAG_OPTIMIZE_ALPHA != 0) as c_int;
            let mut pass: c_int;

            pass = 0;
            while pass < passes {
                let startx: c_uint;
                let stepx: c_uint;
                let stepy: c_uint;
                let mut y: png_uint_32;

                if (*png_ptr).interlaced == PNG_INTERLACE_ADAM7 as png_byte {
                    /* The row may be empty for a short image: */
                    if PNG_PASS_COLS(width, pass) == 0 {
                        pass += 1;
                        continue;
                    }

                    startx = PNG_PASS_START_COL(pass) as c_uint * channels;
                    stepx = PNG_PASS_COL_OFFSET(pass) as c_uint * channels;
                    y = PNG_PASS_START_ROW(pass) as png_uint_32;
                    stepy = PNG_PASS_ROW_OFFSET(pass) as c_uint;
                } else {
                    y = 0;
                    startx = 0;
                    stepx = channels;
                    stepy = 1;
                }

                while y < height {
                    let mut inrow: png_bytep = (*display).local_row as png_bytep;
                    let mut outrow: png_bytep;
                    let row_end: png_const_bytep;

                    /* Read the row, which is packed: */
                    png_read_row(png_ptr, inrow, core::ptr::null_mut());

                    outrow = (*display).first_row as png_bytep;
                    outrow = outrow.offset(y as isize * row_step);
                    row_end = outrow.add((width * channels) as usize);

                    /* Now do the composition on each pixel in this row. */
                    outrow = outrow.add(startx as usize);
                    while (outrow as *const png_byte) < row_end {
                        let alpha: png_byte = *inrow.add(channels as usize);

                        if alpha > 0 {
                            /* else no change to the output */
                            let mut c: c_uint = 0;

                            while c < channels {
                                let mut component: png_uint_32 = *inrow.add(c as usize) as png_uint_32;

                                if alpha < 255 {
                                    /* else just use component */
                                    if optimize_alpha != 0 {
                                        component *= 257 * 255; /* =65535 */
                                        component += (255 - alpha as png_uint_32)
                                            * *png_sRGB_table
                                                .as_ptr()
                                                .add(*outrow.add(c as usize) as usize)
                                                as png_uint_32;

                                        if component > 255 * 65535 {
                                            component = 255 * 65535;
                                        }

                                        component = PNG_sRGB_FROM_LINEAR(component) as png_uint_32;
                                    } else {
                                        let background: png_uint_32 =
                                            *outrow.add(c as usize) as png_uint_32;
                                        component += ((255 - alpha as png_uint_32) * background
                                            + 127)
                                            / 255;
                                        if component > 255 {
                                            component = 255;
                                        }
                                    }
                                }

                                *outrow.add(c as usize) = component as png_byte;
                                c += 1;
                            }
                        }

                        inrow = inrow.add((channels + 1) as usize); /* components and alpha channel */
                        outrow = outrow.add(stepx as usize);
                    }
                    y += stepy;
                }
                pass += 1;
            }
        }

        1
    }
}

/* The do_local_background case. */
unsafe extern "C-unwind" fn png_image_read_background(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_read_control = argument as *mut png_image_read_control;
        let image: png_imagep = (*display).image;
        let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
        let info_ptr: png_inforp = (*(*image).opaque).info_ptr;
        let height: png_uint_32 = (*image).height;
        let width: png_uint_32 = (*image).width;
        let mut pass: c_int;
        let passes: c_int;

        /* Double check the convoluted logic below. */
        if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) == 0 {
            png_error(png_ptr, c"lost rgb to gray".as_ptr());
        }

        if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
            png_error(png_ptr, c"unexpected compose".as_ptr());
        }

        if png_get_channels(png_ptr, info_ptr) != 2 {
            png_error(png_ptr, c"lost/gained channels".as_ptr());
        }

        /* Expect the 8-bit case to always remove the alpha channel */
        if ((*image).format & PNG_FORMAT_FLAG_LINEAR) == 0
            && ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0
        {
            png_error(png_ptr, c"unexpected 8-bit transformation".as_ptr());
        }

        match (*png_ptr).interlaced as c_int {
            PNG_INTERLACE_NONE => {
                passes = 1;
            }

            PNG_INTERLACE_ADAM7 => {
                passes = PNG_INTERLACE_ADAM7_PASSES as c_int;
            }

            _ => {
                png_error(png_ptr, c"unknown interlace type".as_ptr());
            }
        }

        match (*info_ptr).bit_depth as c_int {
            8 => {
                /* 8-bit sRGB gray values with an alpha channel. */
                let first_row: png_bytep = (*display).first_row as png_bytep;
                let row_step: isize = (*display).row_step;

                pass = 0;
                while pass < passes {
                    let startx: c_uint;
                    let stepx: c_uint;
                    let stepy: c_uint;
                    let mut y: png_uint_32;

                    if (*png_ptr).interlaced == PNG_INTERLACE_ADAM7 as png_byte {
                        if PNG_PASS_COLS(width, pass) == 0 {
                            pass += 1;
                            continue;
                        }

                        startx = PNG_PASS_START_COL(pass) as c_uint;
                        stepx = PNG_PASS_COL_OFFSET(pass) as c_uint;
                        y = PNG_PASS_START_ROW(pass) as png_uint_32;
                        stepy = PNG_PASS_ROW_OFFSET(pass) as c_uint;
                    } else {
                        y = 0;
                        startx = 0;
                        stepx = 1;
                        stepy = 1;
                    }

                    if (*display).background == core::ptr::null() {
                        while y < height {
                            let mut inrow: png_bytep = (*display).local_row as png_bytep;
                            let mut outrow: png_bytep = first_row.offset(y as isize * row_step);
                            let row_end: png_const_bytep = outrow.add(width as usize);

                            png_read_row(png_ptr, inrow, core::ptr::null_mut());

                            outrow = outrow.add(startx as usize);
                            while (outrow as *const png_byte) < row_end {
                                let alpha: png_byte = *inrow.add(1);

                                if alpha > 0 {
                                    /* else no change to the output */
                                    let mut component: png_uint_32 = *inrow.add(0) as png_uint_32;

                                    if alpha < 255 {
                                        /* else just use component */
                                        component = *png_sRGB_table
                                            .as_ptr()
                                            .add(component as usize)
                                            as png_uint_32
                                            * alpha as png_uint_32;
                                        component += *png_sRGB_table
                                            .as_ptr()
                                            .add(*outrow.add(0) as usize)
                                            as png_uint_32
                                            * (255 - alpha as png_uint_32);
                                        component = PNG_sRGB_FROM_LINEAR(component) as png_uint_32;
                                    }

                                    *outrow.add(0) = component as png_byte;
                                }

                                inrow = inrow.add(2); /* gray and alpha channel */
                                outrow = outrow.add(stepx as usize);
                            }
                            y += stepy;
                        }
                    } else {
                        /* constant background value */
                        let background8: png_byte = (*(*display).background).green;
                        let background: png_uint_16 =
                            *png_sRGB_table.as_ptr().add(background8 as usize);

                        while y < height {
                            let mut inrow: png_bytep = (*display).local_row as png_bytep;
                            let mut outrow: png_bytep = first_row.offset(y as isize * row_step);
                            let row_end: png_const_bytep = outrow.add(width as usize);

                            png_read_row(png_ptr, inrow, core::ptr::null_mut());

                            outrow = outrow.add(startx as usize);
                            while (outrow as *const png_byte) < row_end {
                                let alpha: png_byte = *inrow.add(1);

                                if alpha > 0 {
                                    /* else use background */
                                    let mut component: png_uint_32 = *inrow.add(0) as png_uint_32;

                                    if alpha < 255 {
                                        /* else just use component */
                                        component = *png_sRGB_table
                                            .as_ptr()
                                            .add(component as usize)
                                            as png_uint_32
                                            * alpha as png_uint_32;
                                        component +=
                                            background as png_uint_32 * (255 - alpha as png_uint_32);
                                        component = PNG_sRGB_FROM_LINEAR(component) as png_uint_32;
                                    }

                                    *outrow.add(0) = component as png_byte;
                                } else {
                                    *outrow.add(0) = background8;
                                }

                                inrow = inrow.add(2); /* gray and alpha channel */
                                outrow = outrow.add(stepx as usize);
                            }
                            y += stepy;
                        }
                    }
                    pass += 1;
                }
            }

            16 => {
                /* 16-bit linear with pre-multiplied alpha. */
                let first_row: png_uint_16p = (*display).first_row as png_uint_16p;
                /* The division by two is safe. */
                let row_step: isize = (*display).row_step / 2;
                let preserve_alpha: c_uint =
                    (((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0) as c_uint;
                let outchannels: c_uint = 1u32 + preserve_alpha;
                let mut swap_alpha: c_int = 0;

                if preserve_alpha != 0 && ((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0 {
                    swap_alpha = 1;
                }

                pass = 0;
                while pass < passes {
                    let startx: c_uint;
                    let stepx: c_uint;
                    let stepy: c_uint;
                    let mut y: png_uint_32;

                    /* The 'x' start and step are adjusted to output components here. */
                    if (*png_ptr).interlaced == PNG_INTERLACE_ADAM7 as png_byte {
                        if PNG_PASS_COLS(width, pass) == 0 {
                            pass += 1;
                            continue;
                        }

                        startx = PNG_PASS_START_COL(pass) as c_uint * outchannels;
                        stepx = PNG_PASS_COL_OFFSET(pass) as c_uint * outchannels;
                        y = PNG_PASS_START_ROW(pass) as png_uint_32;
                        stepy = PNG_PASS_ROW_OFFSET(pass) as c_uint;
                    } else {
                        y = 0;
                        startx = 0;
                        stepx = outchannels;
                        stepy = 1;
                    }

                    while y < height {
                        let mut inrow: png_const_uint_16p;
                        let mut outrow: png_uint_16p = first_row.offset(y as isize * row_step);
                        let row_end: png_uint_16p = outrow.add((width * outchannels) as usize);

                        /* Read the row, which is packed: */
                        png_read_row(
                            png_ptr,
                            (*display).local_row as png_bytep,
                            core::ptr::null_mut(),
                        );
                        inrow = (*display).local_row as png_const_uint_16p;

                        /* Now do the pre-multiplication on each pixel in this row. */
                        outrow = outrow.add(startx as usize);
                        while (outrow as usize) < (row_end as usize) {
                            let mut component: png_uint_32 = *inrow.add(0) as png_uint_32;
                            let alpha: png_uint_16 = *inrow.add(1);

                            if alpha > 0 {
                                /* else 0 */
                                if alpha < 65535 {
                                    /* else just use component */
                                    component *= alpha as png_uint_32;
                                    component += 32767;
                                    component /= 65535;
                                }
                            } else {
                                component = 0;
                            }

                            *outrow.add(swap_alpha as usize) = component as png_uint_16;
                            if preserve_alpha != 0 {
                                *outrow.add((1 ^ swap_alpha) as usize) = alpha;
                            }

                            inrow = inrow.add(2); /* components and alpha channel */
                            outrow = outrow.add(stepx as usize);
                        }
                        y += stepy;
                    }
                    pass += 1;
                }
            }

            _ => {
                png_error(png_ptr, c"unexpected bit depth".as_ptr());
            }
        }

        1
    }
}

/* The guts of png_image_finish_read as a png_safe_execute callback. */
unsafe extern "C-unwind" fn png_image_read_direct(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_read_control = argument as *mut png_image_read_control;
        let image: png_imagep = (*display).image;
        let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
        let info_ptr: png_inforp = (*(*image).opaque).info_ptr;

        let mut format: png_uint_32 = (*image).format;
        let linear: c_int = ((format & PNG_FORMAT_FLAG_LINEAR) != 0) as c_int;
        let mut do_local_compose: c_int = 0;
        let mut do_local_background: c_int = 0; /* to avoid double gamma correction bug */
        let mut do_local_scale: c_int = 0; /* for interlaced 16-to-8 bit conversion */
        let mut passes: c_int = 0;

        /* Add transforms to ensure the correct output format is produced. */
        png_set_expand(png_ptr);

        /* Now check the format to see if it was modified. */
        {
            let base_format: png_uint_32 =
                png_image_format(png_ptr) & !PNG_FORMAT_FLAG_COLORMAP /* removed by png_set_expand */;
            let mut change: png_uint_32 = format ^ base_format;
            let output_gamma: png_fixed_point;
            let mut mode: c_int; /* alpha mode */

            /* Do this first so that we have a record if rgb to gray is happening. */
            if (change & PNG_FORMAT_FLAG_COLOR) != 0 {
                /* gray<->color transformation required. */
                if (format & PNG_FORMAT_FLAG_COLOR) != 0 {
                    png_set_gray_to_rgb(png_ptr);
                } else {
                    if (base_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                        do_local_background = 1; /*maybe*/
                    }

                    png_set_rgb_to_gray_fixed(
                        png_ptr,
                        PNG_ERROR_ACTION_NONE,
                        PNG_RGB_TO_GRAY_DEFAULT,
                        PNG_RGB_TO_GRAY_DEFAULT,
                    );
                }

                change &= !PNG_FORMAT_FLAG_COLOR;
            }

            /* Set the gamma appropriately. */
            {
                let input_gamma_default: png_fixed_point;

                if (base_format & PNG_FORMAT_FLAG_LINEAR) != 0
                    && ((*image).flags & PNG_IMAGE_FLAG_16BIT_sRGB) == 0
                {
                    input_gamma_default = PNG_GAMMA_LINEAR;
                } else {
                    input_gamma_default = PNG_DEFAULT_sRGB;
                }

                png_set_alpha_mode_fixed(png_ptr, PNG_ALPHA_PNG, input_gamma_default);
            }

            if linear != 0 {
                if (base_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    mode = PNG_ALPHA_STANDARD; /* associated alpha */
                } else {
                    mode = PNG_ALPHA_PNG;
                }

                output_gamma = PNG_GAMMA_LINEAR;
            } else {
                mode = PNG_ALPHA_PNG;
                output_gamma = PNG_DEFAULT_sRGB;
            }

            if (change & PNG_FORMAT_FLAG_ASSOCIATED_ALPHA) != 0 {
                mode = PNG_ALPHA_OPTIMIZED;
                change &= !PNG_FORMAT_FLAG_ASSOCIATED_ALPHA;
            }

            /* If 'do_local_background' is set check for the presence of gamma
             * correction.
             */
            if do_local_background != 0 {
                let mut gtest: png_fixed_point = 0;

                if png_muldiv(
                    &raw mut gtest,
                    output_gamma,
                    png_resolve_file_gamma(png_ptr),
                    PNG_FP_1,
                ) != 0
                    && png_gamma_significant(gtest) == 0
                {
                    do_local_background = 0;
                } else if mode == PNG_ALPHA_STANDARD {
                    do_local_background = 2; /*required*/
                    mode = PNG_ALPHA_PNG; /* prevent libpng doing it */
                }

                /* else leave as 1 for the checks below */
            }

            /* If the bit-depth changes then handle that here. */
            if (change & PNG_FORMAT_FLAG_LINEAR) != 0 {
                if linear != 0 {
                    /*16-bit output*/
                    png_set_expand_16(png_ptr);
                } else {
                    /* 8-bit output */
                    png_set_scale_16(png_ptr);

                    /* For interlaced images, use local_row buffer to avoid overflow. */
                    if (*png_ptr).interlaced != 0 {
                        do_local_scale = 1;
                    }
                }

                change &= !PNG_FORMAT_FLAG_LINEAR;
            }

            /* Now the background/alpha channel changes. */
            if (change & PNG_FORMAT_FLAG_ALPHA) != 0 {
                if (base_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    if do_local_background != 0 {
                        do_local_background = 2; /*required*/
                    }
                    /* 16-bit output: just remove the channel */
                    else if linear != 0 {
                        /* compose on black (well, pre-multiply) */
                        png_set_strip_alpha(png_ptr);
                    }
                    /* 8-bit output: do an appropriate compose */
                    else if (*display).background != core::ptr::null() {
                        let mut c: png_color_16 = png_color_16::default();

                        c.index = 0; /*unused*/
                        c.red = (*(*display).background).red as png_uint_16;
                        c.green = (*(*display).background).green as png_uint_16;
                        c.blue = (*(*display).background).blue as png_uint_16;
                        c.gray = (*(*display).background).green as png_uint_16;

                        png_set_background_fixed(
                            png_ptr,
                            &raw mut c,
                            PNG_BACKGROUND_GAMMA_SCREEN,
                            0, /*need_expand*/
                            0, /*gamma: not used*/
                        );
                    } else {
                        /* compose on row: implemented below. */
                        do_local_compose = 1;
                        mode = PNG_ALPHA_OPTIMIZED;
                    }
                } else {
                    /* output needs an alpha channel */
                    let filler: png_uint_32; /* opaque filler */
                    let where_: c_int;

                    if linear != 0 {
                        filler = 65535;
                    } else {
                        filler = 255;
                    }

                    if (format & PNG_FORMAT_FLAG_AFIRST) != 0 {
                        where_ = PNG_FILLER_BEFORE;
                        change &= !PNG_FORMAT_FLAG_AFIRST;
                    } else {
                        where_ = PNG_FILLER_AFTER;
                    }

                    png_set_add_alpha(png_ptr, filler, where_);
                }

                /* This stops the (irrelevant) call to swap_alpha below. */
                change &= !PNG_FORMAT_FLAG_ALPHA;
            }

            /* Now set the alpha mode correctly. */
            png_set_alpha_mode_fixed(png_ptr, mode, output_gamma);

            if (change & PNG_FORMAT_FLAG_BGR) != 0 {
                /* Check only the output format; PNG is never BGR. */
                if (format & PNG_FORMAT_FLAG_COLOR) != 0 {
                    png_set_bgr(png_ptr);
                } else {
                    format &= !PNG_FORMAT_FLAG_BGR;
                }

                change &= !PNG_FORMAT_FLAG_BGR;
            }

            if (change & PNG_FORMAT_FLAG_AFIRST) != 0 {
                /* Only relevant if there is an alpha channel. */
                if (format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    /* Disable this if doing a local background. */
                    if do_local_background != 2 {
                        png_set_swap_alpha(png_ptr);
                    }
                } else {
                    format &= !PNG_FORMAT_FLAG_AFIRST;
                }

                change &= !PNG_FORMAT_FLAG_AFIRST;
            }

            /* If the *output* is 16-bit then we need to check for a byte-swap. */
            if linear != 0 {
                let le: png_uint_16 = 0x0001;

                if (*(&raw const le as png_const_bytep) as c_int & le as c_int) != 0 {
                    png_set_swap(png_ptr);
                }
            }

            /* If change is not now 0 some transformation is missing - error out. */
            if change != 0 {
                png_error(png_ptr, c"png_read_image: unsupported transformation".as_ptr());
            }
        }

        PNG_SKIP_CHUNKS(png_ptr);

        /* Update the 'info' structure. */
        if do_local_compose == 0 && do_local_background != 2 {
            passes = png_set_interlace_handling(png_ptr);
        }

        png_read_update_info(png_ptr, info_ptr);

        {
            let mut info_format: png_uint_32 = 0;

            if ((*info_ptr).color_type & PNG_COLOR_MASK_COLOR as png_byte) != 0 {
                info_format |= PNG_FORMAT_FLAG_COLOR;
            }

            if ((*info_ptr).color_type & PNG_COLOR_MASK_ALPHA as png_byte) != 0 {
                /* do_local_compose removes this channel below. */
                if do_local_compose == 0 {
                    /* do_local_background does the same if required. */
                    if do_local_background != 2 || (format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                        info_format |= PNG_FORMAT_FLAG_ALPHA;
                    }
                }
            } else if do_local_compose != 0 {
                /* internal error */
                png_error(png_ptr, c"png_image_read: alpha channel lost".as_ptr());
            }

            if (format & PNG_FORMAT_FLAG_ASSOCIATED_ALPHA) != 0 {
                info_format |= PNG_FORMAT_FLAG_ASSOCIATED_ALPHA;
            }

            if (*info_ptr).bit_depth == 16 {
                info_format |= PNG_FORMAT_FLAG_LINEAR;
            }

            if ((*png_ptr).transformations & PNG_BGR) != 0 {
                info_format |= PNG_FORMAT_FLAG_BGR;
            }

            if do_local_background == 2 {
                if (format & PNG_FORMAT_FLAG_AFIRST) != 0 {
                    info_format |= PNG_FORMAT_FLAG_AFIRST;
                }
            }

            if ((*png_ptr).transformations & PNG_SWAP_ALPHA) != 0
                || (((*png_ptr).transformations & PNG_ADD_ALPHA) != 0
                    && ((*png_ptr).flags & PNG_FLAG_FILLER_AFTER) == 0)
            {
                if do_local_background == 2 {
                    png_error(png_ptr, c"unexpected alpha swap transformation".as_ptr());
                }

                info_format |= PNG_FORMAT_FLAG_AFIRST;
            }

            /* This is actually an internal error. */
            if info_format != format {
                png_error(png_ptr, c"png_read_image: invalid transformations".as_ptr());
            }
        }

        /* Now read the rows. */
        {
            let mut first_row: png_voidp = (*display).buffer;
            let mut row_step: isize = (*display).row_stride as isize;

            if linear != 0 {
                row_step *= 2;
            }

            /* Ensure calculations are correct regardless of the sign of row_step. */
            if row_step < 0 {
                let mut ptr: *mut c_char = first_row as *mut c_char;
                ptr = ptr.offset(((*image).height - 1) as isize * (-row_step));
                first_row = ptr as png_voidp;
            }

            (*display).first_row = first_row;
            (*display).row_step = row_step;
        }

        if do_local_compose != 0 {
            let result: c_int;
            let row: png_voidp = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr));

            (*display).local_row = row;
            result = png_safe_execute(image, Some(png_image_read_composite), display as png_voidp);
            (*display).local_row = core::ptr::null_mut();
            png_free(png_ptr, row);

            result
        } else if do_local_background == 2 {
            let result: c_int;
            let row: png_voidp = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr));

            (*display).local_row = row;
            result = png_safe_execute(image, Some(png_image_read_background), display as png_voidp);
            (*display).local_row = core::ptr::null_mut();
            png_free(png_ptr, row);

            result
        } else if do_local_scale != 0 {
            /* For interlaced 16-to-8 conversion, use an intermediate row buffer. */
            let result: c_int;
            let row: png_voidp = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr));

            (*display).local_row = row;
            result =
                png_safe_execute(image, Some(png_image_read_direct_scaled), display as png_voidp);
            (*display).local_row = core::ptr::null_mut();
            png_free(png_ptr, row);

            result
        } else {
            let row_step: isize = (*display).row_step;

            loop {
                passes -= 1;
                if passes < 0 {
                    break;
                }
                let mut y: png_uint_32 = (*image).height;
                let mut row: png_bytep = (*display).first_row as png_bytep;

                while y > 0 {
                    png_read_row(png_ptr, row, core::ptr::null_mut());
                    row = row.offset(row_step);
                    y -= 1;
                }
            }

            1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_finish_read(
    image: png_imagep,
    background: png_const_colorp,
    buffer: *mut c_void,
    mut row_stride: png_int_32,
    colormap: *mut c_void,
) -> c_int {
    unsafe {
        if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
            /* Check for row_stride overflow. */
            let channels: c_uint = PNG_IMAGE_PIXEL_CHANNELS((*image).format);

            if (*image).width <= 0x7fffffffu32 / channels {
                /* no overflow */
                let check: png_uint_32;
                let png_row_stride: png_uint_32 = (*image).width * channels;

                if row_stride == 0 {
                    row_stride = png_row_stride as png_int_32; /*SAFE*/
                }

                if row_stride < 0 {
                    check = (row_stride as png_uint_32).wrapping_neg();
                } else {
                    check = row_stride as png_uint_32;
                }

                /* This verifies 'check'. */
                if (*image).opaque != core::ptr::null_mut()
                    && buffer != core::ptr::null_mut()
                    && check >= png_row_stride
                {
                    /* Now check for overflow of the image buffer calculation. */
                    if (*image).height
                        <= 0xffffffffu32
                            / PNG_IMAGE_PIXEL_COMPONENT_SIZE((*image).format)
                            / check
                    {
                        if ((*image).format & PNG_FORMAT_FLAG_COLORMAP) == 0
                            || ((*image).colormap_entries > 0 && colormap != core::ptr::null_mut())
                        {
                            let result: c_int;
                            let mut display: png_image_read_control =
                                core::mem::zeroed::<png_image_read_control>();

                            memset(
                                &raw mut display as png_voidp,
                                0,
                                core::mem::size_of::<png_image_read_control>(),
                            );
                            display.image = image;
                            display.buffer = buffer;
                            display.row_stride = row_stride;
                            display.colormap = colormap;
                            display.background = background;
                            display.local_row = core::ptr::null_mut();

                            /* Choose the correct 'end' routine. */
                            if ((*image).format & PNG_FORMAT_FLAG_COLORMAP) != 0 {
                                result = (png_safe_execute(
                                    image,
                                    Some(png_image_read_colormap),
                                    &raw mut display as png_voidp,
                                ) != 0
                                    && png_safe_execute(
                                        image,
                                        Some(png_image_read_colormapped),
                                        &raw mut display as png_voidp,
                                    ) != 0) as c_int;
                            } else {
                                result = png_safe_execute(
                                    image,
                                    Some(png_image_read_direct),
                                    &raw mut display as png_voidp,
                                );
                            }

                            png_image_free(image);
                            return result;
                        } else {
                            return png_image_error(
                                image,
                                c"png_image_finish_read[color-map]: no color-map".as_ptr(),
                            );
                        }
                    } else {
                        return png_image_error(
                            image,
                            c"png_image_finish_read: image too large".as_ptr(),
                        );
                    }
                } else {
                    return png_image_error(
                        image,
                        c"png_image_finish_read: invalid argument".as_ptr(),
                    );
                }
            } else {
                return png_image_error(
                    image,
                    c"png_image_finish_read: row_stride too large".as_ptr(),
                );
            }
        } else if image != core::ptr::null_mut() {
            return png_image_error(
                image,
                c"png_image_finish_read: damaged PNG_IMAGE_VERSION".as_ptr(),
            );
        }

        0
    }
}
