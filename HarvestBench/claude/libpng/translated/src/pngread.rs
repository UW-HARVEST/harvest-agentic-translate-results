//! Translation of pngread.c - read a PNG file.
use crate::prelude::*;

// ---------------------------------------------------------------------------
// Local constants and FFI not present in the shared prelude modules.
// (These mirror png.h / pngpriv.h values for the enabled configuration.)
// ---------------------------------------------------------------------------

// png_image::format flags (png.h)
const PNG_FORMAT_FLAG_ALPHA: png_uint_32 = 0x01;
const PNG_FORMAT_FLAG_COLOR: png_uint_32 = 0x02;
const PNG_FORMAT_FLAG_LINEAR: png_uint_32 = 0x04;
const PNG_FORMAT_FLAG_COLORMAP: png_uint_32 = 0x08;
const PNG_FORMAT_FLAG_BGR: png_uint_32 = 0x10;
const PNG_FORMAT_FLAG_AFIRST: png_uint_32 = 0x20;
const PNG_FORMAT_FLAG_ASSOCIATED_ALPHA: png_uint_32 = 0x40;

// png_image::flags (png.h)
const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: png_uint_32 = 0x01;
const PNG_IMAGE_FLAG_16BIT_sRGB: png_uint_32 = 0x04;

// gamma / alpha-mode constants (png.h)
const PNG_ALPHA_PNG: c_int = 0;
const PNG_ALPHA_STANDARD: c_int = 1;
const PNG_ALPHA_OPTIMIZED: c_int = 2;
const PNG_DEFAULT_sRGB: png_fixed_point = -1;
const PNG_GAMMA_sRGB: png_fixed_point = 220000;
const PNG_GAMMA_LINEAR: png_fixed_point = PNG_FP_1;
const PNG_BACKGROUND_GAMMA_SCREEN: c_int = 1;
const PNG_ERROR_ACTION_NONE: c_int = 1;
const PNG_RGB_TO_GRAY_DEFAULT: png_fixed_point = -1;
const PNG_FILLER_BEFORE: c_int = 0;
const PNG_FILLER_AFTER: c_int = 1;

// PNG_IMAGE_SAMPLE_CHANNELS(fmt): (fmt & (COLOR|ALPHA)) + 1
#[inline]
fn png_image_sample_channels(fmt: png_uint_32) -> png_uint_32 {
    (fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
}

// PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt): ((fmt & LINEAR) >> 2) + 1
#[inline]
fn png_image_sample_component_size(fmt: png_uint_32) -> png_uint_32 {
    ((fmt & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1
}

// PNG_IMAGE_SAMPLE_SIZE(fmt)
#[inline]
fn png_image_sample_size(fmt: png_uint_32) -> png_uint_32 {
    png_image_sample_channels(fmt) * png_image_sample_component_size(fmt)
}

// PNG_IMAGE_PIXEL_(test,fmt): colormap => 1 else test(fmt)
#[inline]
fn png_image_pixel_channels(fmt: png_uint_32) -> png_uint_32 {
    if (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        1
    } else {
        png_image_sample_channels(fmt)
    }
}

#[inline]
fn png_image_pixel_component_size(fmt: png_uint_32) -> png_uint_32 {
    if (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        1
    } else {
        png_image_sample_component_size(fmt)
    }
}

// Interlace pass geometry macros (png.h)
#[inline]
fn png_pass_start_row(pass: c_int) -> c_int {
    ((1 & !pass) << (3 - (pass >> 1))) & 7
}
#[inline]
fn png_pass_start_col(pass: c_int) -> c_int {
    ((1 & pass) << (3 - ((pass + 1) >> 1))) & 7
}
#[inline]
fn png_pass_row_offset(pass: c_int) -> c_int {
    if pass > 2 {
        8 >> (((pass) - 1) >> 1)
    } else {
        8
    }
}
#[inline]
fn png_pass_col_offset(pass: c_int) -> c_int {
    1 << ((7 - pass) >> 1)
}
#[inline]
fn png_pass_col_shift(pass: c_int) -> c_int {
    if pass > 1 {
        (7 - pass) >> 1
    } else {
        3
    }
}
#[inline]
fn png_pass_cols(width: png_uint_32, pass: c_int) -> png_uint_32 {
    (width.wrapping_add(
        (((1u32 << png_pass_col_shift(pass)) - 1) as i32 - png_pass_start_col(pass)) as u32,
    ))
        >> png_pass_col_shift(pass)
}

// PNG_sRGB_FROM_LINEAR(linear): uses the png_sRGB_base/delta tables
#[inline]
unsafe fn png_srgb_from_linear(linear: png_uint_32) -> png_byte {
    (0xff
        & (((png_sRGB_base[(linear >> 15) as usize] as u32)
            + ((((linear & 0x7fff) * png_sRGB_delta[(linear >> 15) as usize] as u32) >> 12)))
            >> 8)) as png_byte
}

// PNG_DIV51(v8): ((v8) * 5 + 130) >> 8
#[inline]
fn png_div51(v8: png_uint_32) -> png_uint_32 {
    (v8 * 5 + 130) >> 8
}

// PNG_RGB_INDEX(r,g,b)
#[inline]
fn png_rgb_index(r: png_uint_32, g: png_uint_32, b: png_uint_32) -> png_byte {
    (6 * (6 * png_div51(r) + png_div51(g)) + png_div51(b)) as png_byte
}

// P_ encoding values
const P_NOTSET: c_int = 0;
const P_sRGB: c_int = 1;
const P_LINEAR: c_int = 2;
const P_FILE: c_int = 3;
const P_LINEAR8: c_int = 4;

// Color-map processing options
const PNG_CMAP_NONE: c_uint = 0;
const PNG_CMAP_GA: c_uint = 1;
const PNG_CMAP_TRANS: c_uint = 2;
const PNG_CMAP_RGB: c_uint = 3;
const PNG_CMAP_RGB_ALPHA: c_uint = 4;

// Background locations
const PNG_CMAP_NONE_BACKGROUND: c_uint = 256;
const PNG_CMAP_GA_BACKGROUND: c_uint = 231;
const PNG_CMAP_TRANS_BACKGROUND: c_uint = 254;
const PNG_CMAP_RGB_BACKGROUND: c_uint = 256;
const PNG_CMAP_RGB_ALPHA_BACKGROUND: c_uint = 216;

const PNG_GRAY_COLORMAP_ENTRIES: c_uint = 256;
const PNG_GA_COLORMAP_ENTRIES: c_uint = 256;
const PNG_RGB_COLORMAP_ENTRIES: c_uint = 216;

// Chunk index values (pngpriv.h PNG_KNOWN_CHUNKS)
const PNG_INDEX_cHRM: c_int = 6;
const PNG_INDEX_cICP: c_int = 7;
const PNG_INDEX_mDCV: c_int = 16;
const PNG_INDEX_sRGB: c_int = 23;

// png_file_has_chunk(png_ptr, i)
#[inline]
unsafe fn png_file_has_chunk(png_ptr: png_const_structrp, i: c_int) -> bool {
    ((*png_ptr).chunks & (0x80000000u32 >> (31 - i))) != 0
}

// Cross-module read-only tables (defined in the png module).
extern "C" {
    static png_sRGB_table: [png_uint_16; 256];
    static png_sRGB_base: [png_uint_16; 512];
    static png_sRGB_delta: [png_byte; 512];
}

// stdio FFI needed for the simplified file reader.
extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    fn fclose(stream: *mut FILE) -> c_int;
    fn strerror(errnum: c_int) -> *mut c_char;
    #[link_name = "__errno_location"]
    fn errno_location() -> *mut c_int;
}

// ---------------------------------------------------------------------------
// png_create_read_struct / png_create_read_struct_2
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_read_struct(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
) -> png_structp {
    // PNG_USER_MEM_SUPPORTED is defined.
    png_create_read_struct_2(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        ptr::null_mut(),
        None,
        None,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_read_struct_2(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) -> png_structp {
    let png_ptr = png_create_png_struct(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        mem_ptr,
        malloc_fn,
        free_fn,
    );

    if !png_ptr.is_null() {
        (*png_ptr).mode = PNG_IS_READ_STRUCT;

        // PNG_SEQUENTIAL_READ_SUPPORTED
        (*png_ptr).IDAT_read_size = PNG_IDAT_READ_SIZE as uInt;

        // PNG_BENIGN_READ_ERRORS_SUPPORTED
        (*png_ptr).flags |= PNG_FLAG_BENIGN_ERRORS_WARN;

        // PNG_RELEASE_BUILD is (BUILD_BASE_TYPE >= BUILD_RC); BASE_TYPE is BETA
        // (2) < RC (3), so this is false in this build; skip APP_WARNINGS_WARN.

        png_set_read_fn(png_ptr, ptr::null_mut(), None);
    }

    png_ptr
}

// ---------------------------------------------------------------------------
// png_read_info
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    // Read and check the PNG file signature.
    png_read_sig(png_ptr, info_ptr);

    loop {
        let length = png_read_chunk_header(png_ptr);
        let chunk_name = (*png_ptr).chunk_name;

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
        } else {
            // PNG_HANDLE_AS_UNKNOWN_SUPPORTED
            let keep = png_chunk_unknown_handling(png_ptr, chunk_name);
            if keep != 0 {
                png_handle_unknown(png_ptr, info_ptr, length, keep);

                if chunk_name == png_PLTE {
                    (*png_ptr).mode |= PNG_HAVE_PLTE;
                } else if chunk_name == png_IDAT {
                    (*png_ptr).idat_size = 0; // It has been consumed
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

// ---------------------------------------------------------------------------
// png_read_update_info
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_update_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    if !png_ptr.is_null() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);

            // PNG_READ_TRANSFORMS_SUPPORTED
            png_read_transform_info(png_ptr, info_ptr);
        } else {
            png_app_error(
                png_ptr,
                c"png_read_update_info/png_start_read_image: duplicate call".as_ptr(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// png_start_read_image
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_start_read_image(png_ptr: png_structrp) {
    if !png_ptr.is_null() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);
        } else {
            png_app_error(
                png_ptr,
                c"png_start_read_image/png_read_update_info: duplicate call".as_ptr(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// png_do_read_intrapixel (MNG_FEATURES)
// ---------------------------------------------------------------------------

unsafe fn png_do_read_intrapixel(row_info: png_row_infop, row: png_bytep) {
    if ((*row_info).color_type & PNG_COLOR_MASK_COLOR as png_byte) != 0 {
        let bytes_per_pixel: c_int;
        let row_width = (*row_info).width;

        if (*row_info).bit_depth == 8 {
            if (*row_info).color_type == PNG_COLOR_TYPE_RGB as png_byte {
                bytes_per_pixel = 3;
            } else if (*row_info).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte {
                bytes_per_pixel = 4;
            } else {
                return;
            }

            let mut rp = row;
            let mut i = 0u32;
            while i < row_width {
                *rp = ((256i32 + *rp as i32 + *rp.add(1) as i32) & 0xff) as png_byte;
                *rp.add(2) = ((256i32 + *rp.add(2) as i32 + *rp.add(1) as i32) & 0xff) as png_byte;
                i += 1;
                rp = rp.add(bytes_per_pixel as usize);
            }
        } else if (*row_info).bit_depth == 16 {
            if (*row_info).color_type == PNG_COLOR_TYPE_RGB as png_byte {
                bytes_per_pixel = 6;
            } else if (*row_info).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte {
                bytes_per_pixel = 8;
            } else {
                return;
            }

            let mut rp = row;
            let mut i = 0u32;
            while i < row_width {
                let s0: png_uint_32 = ((*rp as png_uint_32) << 8) | *rp.add(1) as png_uint_32;
                let s1: png_uint_32 =
                    ((*rp.add(2) as png_uint_32) << 8) | *rp.add(3) as png_uint_32;
                let s2: png_uint_32 =
                    ((*rp.add(4) as png_uint_32) << 8) | *rp.add(5) as png_uint_32;
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

// ---------------------------------------------------------------------------
// png_read_row
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_row(png_ptr: png_structrp, row: png_bytep, dsp_row: png_bytep) {
    let mut row_info = png_row_info {
        width: 0,
        rowbytes: 0,
        color_type: 0,
        bit_depth: 0,
        channels: 0,
        pixel_depth: 0,
    };

    if png_ptr.is_null() {
        return;
    }

    if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
        png_read_start_row(png_ptr);
    }

    row_info.width = (*png_ptr).iwidth;
    row_info.color_type = (*png_ptr).color_type;
    row_info.bit_depth = (*png_ptr).bit_depth;
    row_info.channels = (*png_ptr).channels;
    row_info.pixel_depth = (*png_ptr).pixel_depth;
    row_info.rowbytes = png_rowbytes(row_info.pixel_depth as u32, row_info.width) as size_t;

    // PNG_WARNINGS_SUPPORTED: the WRITE_*/!READ_* checks are all inactive in
    // this full-feature build (both WRITE and READ variants are enabled).

    // PNG_READ_INTERLACING_SUPPORTED
    if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        match (*png_ptr).pass {
            0 => {
                if (*png_ptr).row_number & 0x07 != 0 {
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            1 => {
                if ((*png_ptr).row_number & 0x07) != 0 || (*png_ptr).width < 5 {
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            2 => {
                if ((*png_ptr).row_number & 0x07) != 4 {
                    if !dsp_row.is_null() && ((*png_ptr).row_number & 4) != 0 {
                        png_combine_row(png_ptr, dsp_row, 1);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            3 => {
                if ((*png_ptr).row_number & 3) != 0 || (*png_ptr).width < 3 {
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            4 => {
                if ((*png_ptr).row_number & 3) != 2 {
                    if !dsp_row.is_null() && ((*png_ptr).row_number & 2) != 0 {
                        png_combine_row(png_ptr, dsp_row, 1);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            5 => {
                if ((*png_ptr).row_number & 1) != 0 || (*png_ptr).width < 2 {
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            _ => {
                // default and case 6
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

    // Fill the row with IDAT data:
    *(*png_ptr).row_buf.add(0) = 255; // to force error if no data was found
    png_read_IDAT_data(
        png_ptr,
        (*png_ptr).row_buf,
        (row_info.rowbytes + 1) as png_alloc_size_t,
    );

    if *(*png_ptr).row_buf.add(0) > PNG_FILTER_VALUE_NONE as png_byte {
        if (*(*png_ptr).row_buf.add(0) as c_int) < PNG_FILTER_VALUE_LAST {
            png_read_filter_row(
                png_ptr,
                &mut row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).prev_row.add(1),
                *(*png_ptr).row_buf.add(0) as c_int,
            );
        } else {
            png_error(png_ptr, c"bad adaptive filter value".as_ptr());
        }
    }

    memcpy(
        (*png_ptr).prev_row as *mut c_void,
        (*png_ptr).row_buf as *const c_void,
        row_info.rowbytes + 1,
    );

    // PNG_MNG_FEATURES_SUPPORTED
    if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
        && (*png_ptr).filter_type == PNG_INTRAPIXEL_DIFFERENCING as png_byte
    {
        png_do_read_intrapixel(&mut row_info, (*png_ptr).row_buf.add(1));
    }

    // PNG_READ_TRANSFORMS_SUPPORTED
    if (*png_ptr).transformations != 0
        // PNG_CHECK_FOR_INVALID_INDEX_SUPPORTED
        || (*png_ptr).num_palette_max >= 0
    {
        png_do_read_transformations(png_ptr, &mut row_info);
    }

    // The transformed pixel depth should match the depth now in row_info.
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

    // PNG_READ_INTERLACING_SUPPORTED
    if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        if (*png_ptr).pass < 6 {
            png_do_read_interlace(
                &mut row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).pass as c_int,
                (*png_ptr).transformations,
            );
        }

        if !dsp_row.is_null() {
            png_combine_row(png_ptr, dsp_row, 1);
        }

        if !row.is_null() {
            png_combine_row(png_ptr, row, 0);
        }
    } else {
        if !row.is_null() {
            png_combine_row(png_ptr, row, -1);
        }

        if !dsp_row.is_null() {
            png_combine_row(png_ptr, dsp_row, -1);
        }
    }
    png_read_finish_row(png_ptr);

    if (*png_ptr).read_row_fn.is_some() {
        ((*png_ptr).read_row_fn.unwrap())(png_ptr, (*png_ptr).row_number, (*png_ptr).pass as c_int);
    }
}

// ---------------------------------------------------------------------------
// png_read_rows
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_rows(
    png_ptr: png_structrp,
    row: png_bytepp,
    display_row: png_bytepp,
    num_rows: png_uint_32,
) {
    if png_ptr.is_null() {
        return;
    }

    let mut rp = row;
    let mut dp = display_row;
    if !rp.is_null() && !dp.is_null() {
        let mut i = 0u32;
        while i < num_rows {
            let rptr = *rp;
            rp = rp.add(1);
            let dptr = *dp;
            dp = dp.add(1);
            png_read_row(png_ptr, rptr, dptr);
            i += 1;
        }
    } else if !rp.is_null() {
        let mut i = 0u32;
        while i < num_rows {
            let rptr = *rp;
            png_read_row(png_ptr, rptr, ptr::null_mut());
            rp = rp.add(1);
            i += 1;
        }
    } else if !dp.is_null() {
        let mut i = 0u32;
        while i < num_rows {
            let dptr = *dp;
            png_read_row(png_ptr, ptr::null_mut(), dptr);
            dp = dp.add(1);
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// png_read_image
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_image(png_ptr: png_structrp, image: png_bytepp) {
    let pass: c_int;
    let image_height: png_uint_32;

    if png_ptr.is_null() {
        return;
    }

    // PNG_READ_INTERLACING_SUPPORTED
    if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
        pass = png_set_interlace_handling(png_ptr);
        png_start_read_image(png_ptr);
    } else {
        if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) == 0 {
            png_warning(
                png_ptr,
                c"Interlace handling should be turned on when using png_read_image".as_ptr(),
            );
            (*png_ptr).num_rows = (*png_ptr).height;
        }

        pass = png_set_interlace_handling(png_ptr);
    }

    image_height = (*png_ptr).height;

    let mut j = 0;
    while j < pass {
        let mut rp = image;
        let mut i = 0u32;
        while i < image_height {
            png_read_row(png_ptr, *rp, ptr::null_mut());
            rp = rp.add(1);
            i += 1;
        }
        j += 1;
    }
}

// ---------------------------------------------------------------------------
// png_read_end
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    if png_ptr.is_null() {
        return;
    }

    // PNG_HANDLE_AS_UNKNOWN_SUPPORTED
    if png_chunk_unknown_handling(png_ptr, png_IDAT) == 0 {
        png_read_finish_IDAT(png_ptr);
    }

    // PNG_READ_CHECK_FOR_INVALID_INDEX_SUPPORTED
    if (*png_ptr).color_type == PNG_COLOR_TYPE_PALETTE as png_byte
        && (*png_ptr).num_palette_max >= (*png_ptr).num_palette as c_int
    {
        png_benign_error(
            png_ptr,
            c"Read palette index exceeding num_palette".as_ptr(),
        );
    }

    loop {
        let length = png_read_chunk_header(png_ptr);
        let chunk_name = (*png_ptr).chunk_name;

        if chunk_name != png_IDAT {
            (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT | PNG_AFTER_IDAT;
        }

        if chunk_name == png_IEND {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if chunk_name == png_IHDR {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if info_ptr.is_null() {
            png_crc_finish(png_ptr, length);
        } else {
            // PNG_HANDLE_AS_UNKNOWN_SUPPORTED
            let keep = png_chunk_unknown_handling(png_ptr, chunk_name);
            if keep != 0 {
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
                if (length > 0 && ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0)
                    || ((*png_ptr).mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0
                {
                    png_benign_error(png_ptr, c"..Too many IDATs found".as_ptr());
                }

                png_crc_finish(png_ptr, length);
            } else {
                png_handle_chunk(png_ptr, info_ptr, length);
            }
        }

        if ((*png_ptr).mode & PNG_HAVE_IEND) != 0 {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// png_read_destroy / png_destroy_read_struct
// ---------------------------------------------------------------------------

unsafe fn png_read_destroy(png_ptr: png_structrp) {
    // PNG_READ_GAMMA_SUPPORTED
    png_destroy_gamma_table(png_ptr);

    png_free(png_ptr, (*png_ptr).big_row_buf as png_voidp);
    (*png_ptr).big_row_buf = ptr::null_mut();
    png_free(png_ptr, (*png_ptr).big_prev_row as png_voidp);
    (*png_ptr).big_prev_row = ptr::null_mut();
    png_free(png_ptr, (*png_ptr).read_buffer as png_voidp);
    (*png_ptr).read_buffer = ptr::null_mut();

    // PNG_READ_QUANTIZE_SUPPORTED
    png_free(png_ptr, (*png_ptr).palette_lookup as png_voidp);
    (*png_ptr).palette_lookup = ptr::null_mut();
    png_free(png_ptr, (*png_ptr).quantize_index as png_voidp);
    (*png_ptr).quantize_index = ptr::null_mut();

    png_free(png_ptr, (*png_ptr).palette as png_voidp);
    (*png_ptr).palette = ptr::null_mut();

    // tRNS || READ_EXPAND || READ_BACKGROUND
    png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
    (*png_ptr).trans_alpha = ptr::null_mut();

    inflateEnd(&mut (*png_ptr).zstream);

    // PNG_PROGRESSIVE_READ_SUPPORTED
    png_free(png_ptr, (*png_ptr).save_buffer as png_voidp);
    (*png_ptr).save_buffer = ptr::null_mut();

    // STORE_UNKNOWN_CHUNKS && READ_UNKNOWN_CHUNKS
    png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
    (*png_ptr).unknown_chunk.data = ptr::null_mut();

    // PNG_SET_UNKNOWN_CHUNKS_SUPPORTED
    png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
    (*png_ptr).chunk_list = ptr::null_mut();

    // ARM_NEON/RISCV riffled_palette: OFF, skip.
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_read_struct(
    png_ptr_ptr: png_structpp,
    info_ptr_ptr: png_infopp,
    end_info_ptr_ptr: png_infopp,
) {
    let mut png_ptr: png_structrp = ptr::null_mut();

    if !png_ptr_ptr.is_null() {
        png_ptr = *png_ptr_ptr;
    }

    if png_ptr.is_null() {
        return;
    }

    png_destroy_info_struct(png_ptr, end_info_ptr_ptr);
    png_destroy_info_struct(png_ptr, info_ptr_ptr);

    *png_ptr_ptr = ptr::null_mut();
    png_read_destroy(png_ptr);
    png_destroy_png_struct(png_ptr);
}

// ---------------------------------------------------------------------------
// png_set_read_status_fn
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_status_fn(
    png_ptr: png_structrp,
    read_row_fn: png_read_status_ptr,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).read_row_fn = read_row_fn;
}

// ---------------------------------------------------------------------------
// png_read_png (INFO_IMAGE)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_png(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    transforms: c_int,
    params: png_voidp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    png_read_info(png_ptr, info_ptr);
    if (*info_ptr).height > PNG_UINT_32_MAX / (core::mem::size_of::<png_bytep>() as png_uint_32) {
        png_error(
            png_ptr,
            c"Image is too high to process with png_read_png()".as_ptr(),
        );
    }

    // PNG_READ_SCALE_16_TO_8_SUPPORTED
    if (transforms & PNG_TRANSFORM_SCALE_16) != 0 {
        png_set_scale_16(png_ptr);
    }

    // PNG_READ_STRIP_16_TO_8_SUPPORTED
    if (transforms & PNG_TRANSFORM_STRIP_16) != 0 {
        png_set_strip_16(png_ptr);
    }

    // PNG_READ_STRIP_ALPHA_SUPPORTED
    if (transforms & PNG_TRANSFORM_STRIP_ALPHA) != 0 {
        png_set_strip_alpha(png_ptr);
    }

    // PNG_READ_PACK_SUPPORTED
    if (transforms & PNG_TRANSFORM_PACKING) != 0 {
        png_set_packing(png_ptr);
    }

    // PNG_READ_PACKSWAP_SUPPORTED
    if (transforms & PNG_TRANSFORM_PACKSWAP) != 0 {
        png_set_packswap(png_ptr);
    }

    // PNG_READ_EXPAND_SUPPORTED
    if (transforms & PNG_TRANSFORM_EXPAND) != 0 {
        png_set_expand(png_ptr);
    }

    // PNG_READ_INVERT_SUPPORTED
    if (transforms & PNG_TRANSFORM_INVERT_MONO) != 0 {
        png_set_invert_mono(png_ptr);
    }

    // PNG_READ_SHIFT_SUPPORTED
    if (transforms & PNG_TRANSFORM_SHIFT) != 0 {
        if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
            png_set_shift(png_ptr, &mut (*info_ptr).sig_bit);
        }
    }

    // PNG_READ_BGR_SUPPORTED
    if (transforms & PNG_TRANSFORM_BGR) != 0 {
        png_set_bgr(png_ptr);
    }

    // PNG_READ_SWAP_ALPHA_SUPPORTED
    if (transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0 {
        png_set_swap_alpha(png_ptr);
    }

    // PNG_READ_SWAP_SUPPORTED
    if (transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0 {
        png_set_swap(png_ptr);
    }

    // PNG_READ_INVERT_ALPHA_SUPPORTED
    if (transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0 {
        png_set_invert_alpha(png_ptr);
    }

    // PNG_READ_GRAY_TO_RGB_SUPPORTED
    if (transforms & PNG_TRANSFORM_GRAY_TO_RGB) != 0 {
        png_set_gray_to_rgb(png_ptr);
    }

    // PNG_READ_EXPAND_16_SUPPORTED
    if (transforms & PNG_TRANSFORM_EXPAND_16) != 0 {
        png_set_expand_16(png_ptr);
    }

    let _ = png_set_interlace_handling(png_ptr);

    png_read_update_info(png_ptr, info_ptr);

    png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0);
    if (*info_ptr).row_pointers.is_null() {
        (*info_ptr).row_pointers = png_malloc(
            png_ptr,
            (*info_ptr).height as png_alloc_size_t
                * (core::mem::size_of::<png_bytep>() as png_alloc_size_t),
        ) as png_bytepp;

        let mut iptr = 0u32;
        while iptr < (*info_ptr).height {
            *(*info_ptr).row_pointers.add(iptr as usize) = ptr::null_mut();
            iptr += 1;
        }

        (*info_ptr).free_me |= PNG_FREE_ROWS;

        let mut iptr = 0u32;
        while iptr < (*info_ptr).height {
            *(*info_ptr).row_pointers.add(iptr as usize) =
                png_malloc(png_ptr, (*info_ptr).rowbytes as png_alloc_size_t) as png_bytep;
            iptr += 1;
        }
    }

    png_read_image(png_ptr, (*info_ptr).row_pointers);
    (*info_ptr).valid |= PNG_INFO_IDAT;

    png_read_end(png_ptr, info_ptr);

    let _ = params;
}

// ===========================================================================
// SIMPLIFIED READ
// ===========================================================================

#[repr(C)]
struct png_image_read_control {
    // Arguments
    image: png_imagep,
    buffer: png_voidp,
    row_stride: png_int_32,
    colormap: png_voidp,
    background: png_const_colorp,

    // Instance variables
    local_row: png_voidp,
    first_row: png_voidp,
    row_step: isize,
    file_encoding: c_int,
    gamma_to_linear: png_fixed_point,
    colormap_processing: c_int,
}

// Shim adapting png_safe_error (returns `!`) to the png_error_ptr type, which
// has no never-type; the underlying function never returns anyway.
unsafe extern "C" fn png_safe_error_shim(png_ptr: png_structp, msg: png_const_charp) {
    png_safe_error(png_ptr, msg)
}

// png_image_read_init - safe initialization
unsafe fn png_image_read_init(image: png_imagep) -> c_int {
    if (*image).opaque.is_null() {
        let png_ptr = png_create_read_struct(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            image as png_voidp,
            Some(png_safe_error_shim),
            Some(png_safe_warning),
        );

        memset(image as *mut c_void, 0, core::mem::size_of::<png_image>());
        (*image).version = PNG_IMAGE_VERSION;

        if !png_ptr.is_null() {
            let mut info_ptr = png_create_info_struct(png_ptr);

            if !info_ptr.is_null() {
                let control = png_malloc_warn(
                    png_ptr,
                    core::mem::size_of::<png_control>() as png_alloc_size_t,
                ) as png_controlp;

                if !control.is_null() {
                    memset(control as *mut c_void, 0, core::mem::size_of::<png_control>());

                    (*control).png_ptr = png_ptr;
                    (*control).info_ptr = info_ptr;
                    (*control).set_for_write(false);

                    (*image).opaque = control;
                    return 1;
                }

                png_destroy_info_struct(png_ptr, &mut info_ptr);
            }

            let mut p = png_ptr;
            png_destroy_read_struct(&mut p, ptr::null_mut(), ptr::null_mut());
        }

        return png_image_error(image, c"png_image_read: out of memory".as_ptr());
    }

    png_image_error(image, c"png_image_read: opaque pointer not NULL".as_ptr())
}

// png_image_format
unsafe fn png_image_format(png_ptr: png_structrp) -> png_uint_32 {
    let mut format: png_uint_32 = 0;

    if ((*png_ptr).color_type & PNG_COLOR_MASK_COLOR as png_byte) != 0 {
        format |= PNG_FORMAT_FLAG_COLOR;
    }

    if ((*png_ptr).color_type & PNG_COLOR_MASK_ALPHA as png_byte) != 0 {
        format |= PNG_FORMAT_FLAG_ALPHA;
    } else if (*png_ptr).num_trans > 0 {
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

const sRGB_TOLERANCE: png_int_32 = 1000;

unsafe fn chromaticities_match_sRGB(xy: *const png_xy) -> c_int {
    let sRGB_xy = png_xy {
        redx: 64000,
        redy: 33000,
        greenx: 30000,
        greeny: 60000,
        bluex: 15000,
        bluey: 6000,
        whitex: 31270,
        whitey: 32900,
    };

    if png_out_of_range((*xy).whitex, sRGB_xy.whitex, sRGB_TOLERANCE)
        || png_out_of_range((*xy).whitey, sRGB_xy.whitey, sRGB_TOLERANCE)
        || png_out_of_range((*xy).redx, sRGB_xy.redx, sRGB_TOLERANCE)
        || png_out_of_range((*xy).redy, sRGB_xy.redy, sRGB_TOLERANCE)
        || png_out_of_range((*xy).greenx, sRGB_xy.greenx, sRGB_TOLERANCE)
        || png_out_of_range((*xy).greeny, sRGB_xy.greeny, sRGB_TOLERANCE)
        || png_out_of_range((*xy).bluex, sRGB_xy.bluex, sRGB_TOLERANCE)
        || png_out_of_range((*xy).bluey, sRGB_xy.bluey, sRGB_TOLERANCE)
    {
        return 0;
    }
    1
}

unsafe fn png_gamma_not_sRGB(g: png_fixed_point) -> c_int {
    if g < PNG_LIB_GAMMA_MIN || g > PNG_LIB_GAMMA_MAX {
        return 0;
    }

    png_gamma_significant((g * 11 + 2) / 5)
}

unsafe fn png_image_is_not_sRGB(png_ptr: png_const_structrp) -> c_int {
    if png_file_has_chunk(png_ptr, PNG_INDEX_cICP) || png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        return (chromaticities_match_sRGB(&(*png_ptr).chromaticities) == 0) as c_int;
    }

    if png_file_has_chunk(png_ptr, PNG_INDEX_sRGB) {
        return 0;
    }

    if png_file_has_chunk(png_ptr, PNG_INDEX_cHRM) {
        return (chromaticities_match_sRGB(&(*png_ptr).chromaticities) == 0) as c_int;
    }

    0
}

unsafe extern "C" fn png_image_read_header(argument: png_voidp) -> c_int {
    let image = argument as png_imagep;
    let png_ptr = (*(*image).opaque).png_ptr;
    let info_ptr = (*(*image).opaque).info_ptr;

    // PNG_BENIGN_ERRORS_SUPPORTED
    png_set_benign_errors(png_ptr, 1);
    png_read_info(png_ptr, info_ptr);

    (*image).width = (*png_ptr).width;
    (*image).height = (*png_ptr).height;

    {
        let format = png_image_format(png_ptr);

        (*image).format = format;

        if (format & PNG_FORMAT_FLAG_COLOR) != 0 && png_image_is_not_sRGB(png_ptr) != 0 {
            (*image).flags |= PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB;
        }
    }

    {
        let mut cmap_entries: png_uint_32;

        match (*png_ptr).color_type as c_int {
            x if x == PNG_COLOR_TYPE_GRAY => {
                cmap_entries = 1u32 << (*png_ptr).bit_depth;
            }
            x if x == PNG_COLOR_TYPE_PALETTE => {
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

// PNG_STDIO_SUPPORTED
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_stdio(
    image: png_imagep,
    file: *mut FILE,
) -> c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !file.is_null() {
            if png_image_read_init(image) != 0 {
                (*(*(*image).opaque).png_ptr).io_ptr = file as png_voidp;
                return png_safe_execute(image, Some(png_image_read_header), image as png_voidp);
            }
        } else {
            return png_image_error(
                image,
                c"png_image_begin_read_from_stdio: invalid argument".as_ptr(),
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            c"png_image_begin_read_from_stdio: incorrect PNG_IMAGE_VERSION".as_ptr(),
        );
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_file(
    image: png_imagep,
    file_name: *const c_char,
) -> c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !file_name.is_null() {
            let fp = fopen(file_name, c"rb".as_ptr());

            if !fp.is_null() {
                if png_image_read_init(image) != 0 {
                    (*(*(*image).opaque).png_ptr).io_ptr = fp as png_voidp;
                    (*(*image).opaque).set_owned_file(true);
                    return png_safe_execute(image, Some(png_image_read_header), image as png_voidp);
                }

                fclose(fp);
            } else {
                return png_image_error(image, strerror(*errno_location()));
            }
        } else {
            return png_image_error(
                image,
                c"png_image_begin_read_from_file: invalid argument".as_ptr(),
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            c"png_image_begin_read_from_file: incorrect PNG_IMAGE_VERSION".as_ptr(),
        );
    }

    0
}

unsafe extern "C" fn png_image_memory_read(png_ptr: png_structp, out: png_bytep, need: size_t) {
    if !png_ptr.is_null() {
        let image = (*png_ptr).io_ptr as png_imagep;
        if !image.is_null() {
            let cp = (*image).opaque;
            if !cp.is_null() {
                let memory = (*cp).memory;
                let size = (*cp).size;

                if !memory.is_null() && size >= need {
                    memcpy(out as *mut c_void, memory as *const c_void, need);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_memory(
    image: png_imagep,
    memory: png_const_voidp,
    size: size_t,
) -> c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !memory.is_null() && size > 0 {
            if png_image_read_init(image) != 0 {
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
    } else if !image.is_null() {
        return png_image_error(
            image,
            c"png_image_begin_read_from_memory: incorrect PNG_IMAGE_VERSION".as_ptr(),
        );
    }

    0
}

// PNG_HANDLE_AS_UNKNOWN_SUPPORTED
unsafe fn png_image_skip_unused_chunks(png_ptr: png_structrp) {
    static CHUNKS_TO_PROCESS: [png_byte; 35] = [
        98, 75, 71, 68, 0, // bKGD
        99, 72, 82, 77, 0, // cHRM
        99, 73, 67, 80, 0, // cICP
        103, 65, 77, 65, 0, // gAMA
        109, 68, 67, 86, 0, // mDCV
        115, 66, 73, 84, 0, // sBIT
        115, 82, 71, 66, 0, // sRGB
    ];

    png_set_keep_unknown_chunks(png_ptr, PNG_HANDLE_CHUNK_NEVER, ptr::null(), -1);

    png_set_keep_unknown_chunks(
        png_ptr,
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        CHUNKS_TO_PROCESS.as_ptr(),
        (core::mem::size_of_val(&CHUNKS_TO_PROCESS) / 5) as c_int,
    );
}

#[inline]
unsafe fn png_skip_chunks(png_ptr: png_structrp) {
    png_image_skip_unused_chunks(png_ptr);
}

unsafe fn set_file_encoding(display: *mut png_image_read_control) {
    let png_ptr = (*(*(*display).image).opaque).png_ptr;
    let g = png_resolve_file_gamma(png_ptr);

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

unsafe fn decode_gamma(
    display: *mut png_image_read_control,
    mut value: png_uint_32,
    mut encoding: c_int,
) -> c_uint {
    if encoding == P_FILE {
        encoding = (*display).file_encoding;
    }

    if encoding == P_NOTSET {
        set_file_encoding(display);
        encoding = (*display).file_encoding;
    }

    match encoding {
        x if x == P_FILE => {
            value = png_gamma_16bit_correct(value * 257, (*display).gamma_to_linear) as png_uint_32;
        }
        x if x == P_sRGB => {
            value = png_sRGB_table[value as usize] as png_uint_32;
        }
        x if x == P_LINEAR => {}
        x if x == P_LINEAR8 => {
            value *= 257;
        }
        // __GNUC__ default
        _ => {
            png_error(
                (*(*(*display).image).opaque).png_ptr,
                c"unexpected encoding (internal error)".as_ptr(),
            );
        }
    }

    value
}

unsafe fn png_colormap_compose(
    display: *mut png_image_read_control,
    foreground: png_uint_32,
    foreground_encoding: c_int,
    alpha: png_uint_32,
    background: png_uint_32,
    encoding: c_int,
) -> png_uint_32 {
    let fg = decode_gamma(display, foreground, foreground_encoding);
    let b = decode_gamma(display, background, encoding);

    let mut f = fg * alpha + b * (255 - alpha);

    if encoding == P_LINEAR {
        f *= 257;
        f += f >> 16;
        f = (f + 32768) >> 16;
    } else {
        // P_sRGB
        f = png_srgb_from_linear(f) as png_uint_32;
    }

    f
}

unsafe fn png_create_colormap_entry(
    display: *mut png_image_read_control,
    ip: png_uint_32,
    mut red: png_uint_32,
    mut green: png_uint_32,
    mut blue: png_uint_32,
    mut alpha: png_uint_32,
    mut encoding: c_int,
) {
    let image = (*display).image;
    let output_encoding = if ((*image).format & PNG_FORMAT_FLAG_LINEAR) != 0 {
        P_LINEAR
    } else {
        P_sRGB
    };
    let convert_to_Y =
        ((*image).format & PNG_FORMAT_FLAG_COLOR) == 0 && (red != green || green != blue);

    if ip > 255 {
        png_error(
            (*(*image).opaque).png_ptr,
            c"color-map index out of range".as_ptr(),
        );
    }

    if encoding == P_FILE {
        if (*display).file_encoding == P_NOTSET {
            set_file_encoding(display);
        }
        encoding = (*display).file_encoding;
    }

    if encoding == P_FILE {
        let g = (*display).gamma_to_linear;

        red = png_gamma_16bit_correct(red * 257, g) as png_uint_32;
        green = png_gamma_16bit_correct(green * 257, g) as png_uint_32;
        blue = png_gamma_16bit_correct(blue * 257, g) as png_uint_32;

        if convert_to_Y || output_encoding == P_LINEAR {
            alpha *= 257;
            encoding = P_LINEAR;
        } else {
            red = png_srgb_from_linear(red * 255) as png_uint_32;
            green = png_srgb_from_linear(green * 255) as png_uint_32;
            blue = png_srgb_from_linear(blue * 255) as png_uint_32;
            encoding = P_sRGB;
        }
    } else if encoding == P_LINEAR8 {
        red *= 257;
        green *= 257;
        blue *= 257;
        alpha *= 257;
        encoding = P_LINEAR;
    } else if encoding == P_sRGB && (convert_to_Y || output_encoding == P_LINEAR) {
        red = png_sRGB_table[red as usize] as png_uint_32;
        green = png_sRGB_table[green as usize] as png_uint_32;
        blue = png_sRGB_table[blue as usize] as png_uint_32;
        alpha *= 257;
        encoding = P_LINEAR;
    }

    if encoding == P_LINEAR {
        if convert_to_Y {
            let mut y: png_uint_32 = 6968u32 * red + 23434u32 * green + 2366u32 * blue;

            if output_encoding == P_LINEAR {
                y = (y + 16384) >> 15;
            } else {
                y = (y + 128) >> 8;
                y *= 255;
                y = png_srgb_from_linear((y + 64) >> 7) as png_uint_32;
                alpha = png_div257(alpha);
                encoding = P_sRGB;
            }

            blue = y;
            red = y;
            green = y;
        } else if output_encoding == P_sRGB {
            red = png_srgb_from_linear(red * 255) as png_uint_32;
            green = png_srgb_from_linear(green * 255) as png_uint_32;
            blue = png_srgb_from_linear(blue * 255) as png_uint_32;
            alpha = png_div257(alpha);
            encoding = P_sRGB;
        }
    }

    if encoding != output_encoding {
        png_error(
            (*(*image).opaque).png_ptr,
            c"bad encoding (internal error)".as_ptr(),
        );
    }

    // Store the value.
    // PNG_FORMAT_AFIRST_SUPPORTED / PNG_FORMAT_BGR_SUPPORTED are ON.
    let afirst = (((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0
        && ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0) as isize;
    let bgr = if ((*image).format & PNG_FORMAT_FLAG_BGR) != 0 {
        2isize
    } else {
        0
    };

    if output_encoding == P_LINEAR {
        let entry0 = (*display).colormap as png_uint_16p;
        let entry = entry0.offset((ip * png_image_sample_channels((*image).format)) as isize);

        match png_image_sample_channels((*image).format) {
            4 => {
                *entry.offset(if afirst != 0 { 0 } else { 3 }) = alpha as png_uint_16;
                // FALLTHROUGH
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
                *entry.offset(afirst + (2 ^ bgr)) = blue as png_uint_16;
                *entry.offset(afirst + 1) = green as png_uint_16;
                *entry.offset(afirst + bgr) = red as png_uint_16;
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
                *entry.offset(afirst + (2 ^ bgr)) = blue as png_uint_16;
                *entry.offset(afirst + 1) = green as png_uint_16;
                *entry.offset(afirst + bgr) = red as png_uint_16;
            }
            2 => {
                *entry.offset(1 ^ afirst) = alpha as png_uint_16;
                // FALLTHROUGH
                if alpha < 65535 {
                    if alpha > 0 {
                        green = (green * alpha + 32767u32) / 65535u32;
                    } else {
                        green = 0;
                    }
                }
                *entry.offset(afirst) = green as png_uint_16;
            }
            1 => {
                if alpha < 65535 {
                    if alpha > 0 {
                        green = (green * alpha + 32767u32) / 65535u32;
                    } else {
                        green = 0;
                    }
                }
                *entry.offset(afirst) = green as png_uint_16;
            }
            _ => {}
        }
    } else {
        // output encoding is P_sRGB
        let entry0 = (*display).colormap as png_bytep;
        let entry = entry0.offset((ip * png_image_sample_channels((*image).format)) as isize);

        match png_image_sample_channels((*image).format) {
            4 => {
                *entry.offset(if afirst != 0 { 0 } else { 3 }) = alpha as png_byte;
                // FALLTHROUGH
                *entry.offset(afirst + (2 ^ bgr)) = blue as png_byte;
                *entry.offset(afirst + 1) = green as png_byte;
                *entry.offset(afirst + bgr) = red as png_byte;
            }
            3 => {
                *entry.offset(afirst + (2 ^ bgr)) = blue as png_byte;
                *entry.offset(afirst + 1) = green as png_byte;
                *entry.offset(afirst + bgr) = red as png_byte;
            }
            2 => {
                *entry.offset(1 ^ afirst) = alpha as png_byte;
                // FALLTHROUGH
                *entry.offset(afirst) = green as png_byte;
            }
            1 => {
                *entry.offset(afirst) = green as png_byte;
            }
            _ => {}
        }
    }
}

unsafe fn make_gray_file_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i = 0u32;
    while i < 256 {
        png_create_colormap_entry(display, i, i, i, i, 255, P_FILE);
        i += 1;
    }
    i as c_int
}

unsafe fn make_gray_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i = 0u32;
    while i < 256 {
        png_create_colormap_entry(display, i, i, i, i, 255, P_sRGB);
        i += 1;
    }
    i as c_int
}

unsafe fn make_ga_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i = 0u32;
    while i < 231 {
        let gray = (i * 256 + 115) / 231;
        png_create_colormap_entry(display, i, gray, gray, gray, 255, P_sRGB);
        i += 1;
    }

    png_create_colormap_entry(display, i, 255, 255, 255, 0, P_sRGB);
    i += 1;

    let mut a = 1u32;
    while a < 5 {
        let mut g = 0u32;
        while g < 6 {
            png_create_colormap_entry(display, i, g * 51, g * 51, g * 51, a * 51, P_sRGB);
            i += 1;
            g += 1;
        }
        a += 1;
    }

    i as c_int
}

unsafe fn make_rgb_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i = 0u32;
    let mut r = 0u32;
    while r < 6 {
        let mut g = 0u32;
        while g < 6 {
            let mut b = 0u32;
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

unsafe extern "C" fn png_image_read_colormap(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_read_control;
    let image = (*display).image;

    let png_ptr = (*(*image).opaque).png_ptr;
    let output_format = (*image).format;
    let output_encoding = if (output_format & PNG_FORMAT_FLAG_LINEAR) != 0 {
        P_LINEAR
    } else {
        P_sRGB
    };

    let mut cmap_entries: c_uint = 0;
    let mut output_processing: c_uint = PNG_CMAP_NONE;
    let mut data_encoding: c_int = P_NOTSET;

    let mut background_index: c_uint = 256;
    let mut back_r: png_uint_32;
    let mut back_g: png_uint_32;
    let mut back_b: png_uint_32;

    let mut expand_tRNS: c_int = 0;

    if (((*png_ptr).color_type & PNG_COLOR_MASK_ALPHA as png_byte) != 0
        || (*png_ptr).num_trans > 0)
        && ((output_format & PNG_FORMAT_FLAG_ALPHA) == 0)
    {
        if output_encoding == P_LINEAR {
            back_b = 0;
            back_g = 0;
            back_r = 0;
        } else if (*display).background.is_null() {
            png_error(
                png_ptr,
                c"background color must be supplied to remove alpha/transparency".as_ptr(),
            );
        } else {
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

    if (*png_ptr).bit_depth == 16 && ((*image).flags & PNG_IMAGE_FLAG_16BIT_sRGB) == 0 {
        (*png_ptr).default_gamma = PNG_GAMMA_LINEAR;
    } else {
        (*png_ptr).default_gamma = PNG_GAMMA_sRGB_INVERSE;
    }

    match (*png_ptr).color_type as c_int {
        x if x == PNG_COLOR_TYPE_GRAY => {
            if (*png_ptr).bit_depth <= 8 {
                let step: c_uint;
                let mut i: c_uint;
                let mut val: c_uint;
                let mut trans: c_uint = 256;
                let mut back_alpha: c_uint = 0;

                cmap_entries = 1u32 << (*png_ptr).bit_depth;
                if cmap_entries > (*image).colormap_entries {
                    png_error(png_ptr, c"gray[8] color-map: too few entries".as_ptr());
                }

                step = 255 / (cmap_entries - 1);
                output_processing = PNG_CMAP_NONE;

                if (*png_ptr).num_trans > 0 {
                    trans = (*png_ptr).trans_color.gray as c_uint;

                    if (output_format & PNG_FORMAT_FLAG_ALPHA) == 0 {
                        back_alpha = if output_encoding == P_LINEAR { 65535 } else { 255 };
                    }
                }

                i = 0;
                val = 0;
                while i < cmap_entries {
                    if i != trans {
                        png_create_colormap_entry(display, i, val, val, val, 255, P_FILE);
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

                data_encoding = P_FILE;

                if (*png_ptr).bit_depth < 8 {
                    png_set_packing(png_ptr);
                }
            } else {
                // bit depth is 16
                data_encoding = P_sRGB;

                if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                    png_error(png_ptr, c"gray[16] color-map: too few entries".as_ptr());
                }

                cmap_entries = make_gray_colormap(display) as c_uint;

                if (*png_ptr).num_trans > 0 {
                    let back_alpha: c_uint;
                    let mut broke = false;

                    if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                        back_alpha = 0;
                    } else {
                        if back_r == back_g && back_g == back_b {
                            let mut c: png_color_16 = core::mem::zeroed();
                            let mut gray = back_g;

                            if output_encoding == P_LINEAR {
                                gray = png_srgb_from_linear(gray * 255) as png_uint_32;

                                png_create_colormap_entry(
                                    display, gray, back_g, back_g, back_g, 65535, P_LINEAR,
                                );
                            }

                            c.index = 0;
                            c.blue = gray as png_uint_16;
                            c.green = gray as png_uint_16;
                            c.red = gray as png_uint_16;
                            c.gray = gray as png_uint_16;

                            png_set_background_fixed(png_ptr, &c, PNG_BACKGROUND_GAMMA_SCREEN, 0, 0);

                            output_processing = PNG_CMAP_NONE;
                            broke = true;
                            back_alpha = 0; // unused after break
                        } else {
                            back_alpha = if output_encoding == P_LINEAR { 65535 } else { 255 };
                        }
                    }

                    if !broke {
                        expand_tRNS = 1;
                        output_processing = PNG_CMAP_TRANS;
                        background_index = 254;

                        png_create_colormap_entry(
                            display, 254, back_r, back_g, back_b, back_alpha, output_encoding,
                        );
                    }
                } else {
                    output_processing = PNG_CMAP_NONE;
                }
            }
        }

        x if x == PNG_COLOR_TYPE_GRAY_ALPHA => {
            data_encoding = P_sRGB;

            if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                    png_error(png_ptr, c"gray+alpha color-map: too few entries".as_ptr());
                }

                cmap_entries = make_ga_colormap(display) as c_uint;

                background_index = PNG_CMAP_GA_BACKGROUND;
                output_processing = PNG_CMAP_GA;
            } else {
                if (output_format & PNG_FORMAT_FLAG_COLOR) == 0
                    || (back_r == back_g && back_g == back_b)
                {
                    let mut c: png_color_16 = core::mem::zeroed();
                    let mut gray = back_g;

                    if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                        png_error(png_ptr, c"gray-alpha color-map: too few entries".as_ptr());
                    }

                    cmap_entries = make_gray_colormap(display) as c_uint;

                    if output_encoding == P_LINEAR {
                        gray = png_srgb_from_linear(gray * 255) as png_uint_32;

                        png_create_colormap_entry(
                            display, gray, back_g, back_g, back_g, 65535, P_LINEAR,
                        );
                    }

                    c.index = 0;
                    c.blue = gray as png_uint_16;
                    c.green = gray as png_uint_16;
                    c.red = gray as png_uint_16;
                    c.gray = gray as png_uint_16;

                    png_set_background_fixed(png_ptr, &c, PNG_BACKGROUND_GAMMA_SCREEN, 0, 0);

                    output_processing = PNG_CMAP_NONE;
                } else {
                    let mut i: png_uint_32;
                    let mut a: png_uint_32;

                    if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                        png_error(png_ptr, c"ga-alpha color-map: too few entries".as_ptr());
                    }

                    i = 0;
                    while i < 231 {
                        let gray = (i * 256 + 115) / 231;
                        png_create_colormap_entry(display, i, gray, gray, gray, 255, P_sRGB);
                        i += 1;
                    }

                    background_index = i;
                    png_create_colormap_entry(
                        display,
                        i,
                        back_r,
                        back_g,
                        back_b,
                        if output_encoding == P_LINEAR { 65535 } else { 255 },
                        output_encoding,
                    );
                    i += 1;

                    if output_encoding == P_sRGB {
                        back_r = png_sRGB_table[back_r as usize] as png_uint_32;
                        back_g = png_sRGB_table[back_g as usize] as png_uint_32;
                        back_b = png_sRGB_table[back_b as usize] as png_uint_32;
                    }

                    a = 1;
                    while a < 5 {
                        let alpha = 51 * a;
                        let back_rx = (255 - alpha) * back_r;
                        let back_gx = (255 - alpha) * back_g;
                        let back_bx = (255 - alpha) * back_b;

                        let mut g = 0u32;
                        while g < 6 {
                            let gray = png_sRGB_table[(g * 51) as usize] as png_uint_32 * alpha;

                            png_create_colormap_entry(
                                display,
                                i,
                                png_srgb_from_linear(gray + back_rx) as png_uint_32,
                                png_srgb_from_linear(gray + back_gx) as png_uint_32,
                                png_srgb_from_linear(gray + back_bx) as png_uint_32,
                                255,
                                P_sRGB,
                            );
                            i += 1;
                            g += 1;
                        }
                        a += 1;
                    }

                    cmap_entries = i;
                    output_processing = PNG_CMAP_GA;
                }
            }
        }

        x if x == PNG_COLOR_TYPE_RGB || x == PNG_COLOR_TYPE_RGB_ALPHA => {
            if (output_format & PNG_FORMAT_FLAG_COLOR) == 0 {
                png_set_rgb_to_gray_fixed(png_ptr, PNG_ERROR_ACTION_NONE, -1, -1);
                data_encoding = P_sRGB;

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
                    output_processing = PNG_CMAP_GA;
                } else {
                    let gamma = png_resolve_file_gamma(png_ptr);

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

                    if (*png_ptr).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte
                        || (*png_ptr).num_trans > 0
                    {
                        let mut c: png_color_16 = core::mem::zeroed();
                        let mut gray = back_g;

                        if data_encoding == P_FILE {
                            if output_encoding == P_sRGB {
                                gray = png_sRGB_table[gray as usize] as png_uint_32;
                            }

                            gray = png_div257(png_gamma_16bit_correct(gray, gamma) as png_uint_32);

                            png_create_colormap_entry(
                                display, gray, back_g, back_g, back_g, 0, output_encoding,
                            );
                        } else if output_encoding == P_LINEAR {
                            gray = png_srgb_from_linear(gray * 255) as png_uint_32;

                            png_create_colormap_entry(
                                display, gray, back_g, back_g, back_g, 0, P_LINEAR,
                            );
                        }

                        c.index = 0;
                        c.blue = gray as png_uint_16;
                        c.green = gray as png_uint_16;
                        c.red = gray as png_uint_16;
                        c.gray = gray as png_uint_16;

                        expand_tRNS = 1;
                        png_set_background_fixed(png_ptr, &c, PNG_BACKGROUND_GAMMA_SCREEN, 0, 0);
                    }

                    output_processing = PNG_CMAP_NONE;
                }
            } else {
                // output is color
                data_encoding = P_sRGB;

                if (*png_ptr).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte
                    || (*png_ptr).num_trans > 0
                {
                    if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                        let mut r: png_uint_32;

                        if PNG_RGB_COLORMAP_ENTRIES + 1 + 27 > (*image).colormap_entries {
                            png_error(png_ptr, c"rgb+alpha color-map: too few entries".as_ptr());
                        }

                        cmap_entries = make_rgb_colormap(display) as c_uint;

                        png_create_colormap_entry(display, cmap_entries, 255, 255, 255, 0, P_sRGB);

                        background_index = cmap_entries;
                        cmap_entries += 1;

                        r = 0;
                        while r < 256 {
                            let mut g = 0u32;
                            while g < 256 {
                                let mut b = 0u32;
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
                        output_processing = PNG_CMAP_RGB_ALPHA;
                    } else {
                        let sample_size = png_image_sample_size(output_format);
                        let r: png_uint_32;
                        let g: png_uint_32;
                        let b: png_uint_32;

                        if PNG_RGB_COLORMAP_ENTRIES + 1 + 27 > (*image).colormap_entries {
                            png_error(png_ptr, c"rgb-alpha color-map: too few entries".as_ptr());
                        }

                        cmap_entries = make_rgb_colormap(display) as c_uint;

                        png_create_colormap_entry(
                            display, cmap_entries, back_r, back_g, back_b, 0, output_encoding,
                        );

                        if output_encoding == P_LINEAR {
                            r = png_srgb_from_linear(back_r * 255) as png_uint_32;
                            g = png_srgb_from_linear(back_g * 255) as png_uint_32;
                            b = png_srgb_from_linear(back_b * 255) as png_uint_32;
                        } else {
                            r = back_r;
                            g = back_g;
                            b = back_b;
                        }

                        if memcmp(
                            ((*display).colormap as png_const_bytep)
                                .offset((sample_size * cmap_entries) as isize)
                                as *const c_void,
                            ((*display).colormap as png_const_bytep).offset(
                                (sample_size * png_rgb_index(r, g, b) as png_uint_32) as isize,
                            ) as *const c_void,
                            sample_size as size_t,
                        ) != 0
                        {
                            background_index = cmap_entries;
                            cmap_entries += 1;

                            let mut r2 = 0u32;
                            while r2 < 256 {
                                let mut g2 = 0u32;
                                while g2 < 256 {
                                    let mut b2 = 0u32;
                                    while b2 < 256 {
                                        png_create_colormap_entry(
                                            display,
                                            cmap_entries,
                                            png_colormap_compose(
                                                display, r2, P_sRGB, 128, back_r, output_encoding,
                                            ),
                                            png_colormap_compose(
                                                display, g2, P_sRGB, 128, back_g, output_encoding,
                                            ),
                                            png_colormap_compose(
                                                display, b2, P_sRGB, 128, back_b, output_encoding,
                                            ),
                                            0,
                                            output_encoding,
                                        );
                                        cmap_entries += 1;
                                        b2 = (b2 << 1) | 0x7f;
                                    }
                                    g2 = (g2 << 1) | 0x7f;
                                }
                                r2 = (r2 << 1) | 0x7f;
                            }

                            expand_tRNS = 1;
                            output_processing = PNG_CMAP_RGB_ALPHA;
                        } else {
                            let mut c: png_color_16 = core::mem::zeroed();

                            c.index = 0;
                            c.red = back_r as png_uint_16;
                            c.green = back_g as png_uint_16;
                            c.gray = back_g as png_uint_16;
                            c.blue = back_b as png_uint_16;

                            png_set_background_fixed(png_ptr, &c, PNG_BACKGROUND_GAMMA_SCREEN, 0, 0);

                            output_processing = PNG_CMAP_RGB;
                        }
                    }
                } else {
                    if PNG_RGB_COLORMAP_ENTRIES > (*image).colormap_entries {
                        png_error(png_ptr, c"rgb color-map: too few entries".as_ptr());
                    }

                    cmap_entries = make_rgb_colormap(display) as c_uint;
                    output_processing = PNG_CMAP_RGB;
                }
            }
        }

        x if x == PNG_COLOR_TYPE_PALETTE => {
            let mut num_trans = (*png_ptr).num_trans as c_uint;
            let trans: png_const_bytep = if num_trans > 0 {
                (*png_ptr).trans_alpha
            } else {
                ptr::null()
            };
            let colormap = (*png_ptr).palette as png_const_colorp;
            let do_background =
                (!trans.is_null() && (output_format & PNG_FORMAT_FLAG_ALPHA) == 0) as c_int;
            let mut i: c_uint;

            if trans.is_null() {
                num_trans = 0;
            }

            output_processing = PNG_CMAP_NONE;
            data_encoding = P_FILE;
            cmap_entries = (*png_ptr).num_palette as c_uint;
            if cmap_entries > 256 {
                cmap_entries = 256;
            }

            if cmap_entries > (*image).colormap_entries {
                png_error(png_ptr, c"palette color-map: too few entries".as_ptr());
            }

            i = 0;
            while i < cmap_entries {
                if do_background != 0 && i < num_trans && *trans.offset(i as isize) < 255 {
                    if *trans.offset(i as isize) == 0 {
                        png_create_colormap_entry(
                            display, i, back_r, back_g, back_b, 0, output_encoding,
                        );
                    } else {
                        let ti = *trans.offset(i as isize) as png_uint_32;
                        png_create_colormap_entry(
                            display,
                            i,
                            png_colormap_compose(
                                display,
                                (*colormap.offset(i as isize)).red as png_uint_32,
                                P_FILE,
                                ti,
                                back_r,
                                output_encoding,
                            ),
                            png_colormap_compose(
                                display,
                                (*colormap.offset(i as isize)).green as png_uint_32,
                                P_FILE,
                                ti,
                                back_g,
                                output_encoding,
                            ),
                            png_colormap_compose(
                                display,
                                (*colormap.offset(i as isize)).blue as png_uint_32,
                                P_FILE,
                                ti,
                                back_b,
                                output_encoding,
                            ),
                            if output_encoding == P_LINEAR { ti * 257 } else { ti },
                            output_encoding,
                        );
                    }
                } else {
                    png_create_colormap_entry(
                        display,
                        i,
                        (*colormap.offset(i as isize)).red as png_uint_32,
                        (*colormap.offset(i as isize)).green as png_uint_32,
                        (*colormap.offset(i as isize)).blue as png_uint_32,
                        if i < num_trans {
                            *trans.offset(i as isize) as png_uint_32
                        } else {
                            255
                        },
                        P_FILE,
                    );
                }
                i += 1;
            }

            if (*png_ptr).bit_depth < 8 {
                png_set_packing(png_ptr);
            }
        }

        _ => {
            png_error(png_ptr, c"invalid PNG color type".as_ptr());
        }
    }

    // ---- common tail (after the switch) ----
    if expand_tRNS != 0
        && (*png_ptr).num_trans > 0
        && ((*png_ptr).color_type & PNG_COLOR_MASK_ALPHA as png_byte) == 0
    {
        png_set_tRNS_to_alpha(png_ptr);
    }

    match data_encoding {
        x if x == P_sRGB => {
            png_set_alpha_mode_fixed(png_ptr, PNG_ALPHA_PNG, PNG_GAMMA_sRGB);
            // FALLTHROUGH
            if (*png_ptr).bit_depth > 8 {
                png_set_scale_16(png_ptr);
            }
        }
        x if x == P_FILE => {
            if (*png_ptr).bit_depth > 8 {
                png_set_scale_16(png_ptr);
            }
        }
        // __GNUC__ default
        _ => {
            png_error(png_ptr, c"bad data option (internal error)".as_ptr());
        }
    }

    if cmap_entries > 256 || cmap_entries > (*image).colormap_entries {
        png_error(png_ptr, c"color map overflow (BAD internal error)".as_ptr());
    }

    (*image).colormap_entries = cmap_entries;

    let mut bad = false;
    match output_processing {
        x if x == PNG_CMAP_NONE => {
            if background_index != PNG_CMAP_NONE_BACKGROUND {
                bad = true;
            }
        }
        x if x == PNG_CMAP_GA => {
            if background_index != PNG_CMAP_GA_BACKGROUND {
                bad = true;
            }
        }
        x if x == PNG_CMAP_TRANS => {
            if background_index >= cmap_entries || background_index != PNG_CMAP_TRANS_BACKGROUND {
                bad = true;
            }
        }
        x if x == PNG_CMAP_RGB => {
            if background_index != PNG_CMAP_RGB_BACKGROUND {
                bad = true;
            }
        }
        x if x == PNG_CMAP_RGB_ALPHA => {
            if background_index != PNG_CMAP_RGB_ALPHA_BACKGROUND {
                bad = true;
            }
        }
        _ => {
            png_error(png_ptr, c"bad processing option (internal error)".as_ptr());
        }
    }

    if bad {
        png_error(png_ptr, c"bad background index (internal error)".as_ptr());
    }

    (*display).colormap_processing = output_processing as c_int;

    1
}

unsafe extern "C" fn png_image_read_and_map(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_read_control;
    let image = (*display).image;
    let png_ptr = (*(*image).opaque).png_ptr;
    let passes: c_int;

    match (*png_ptr).interlaced as c_int {
        x if x == PNG_INTERLACE_NONE => passes = 1,
        x if x == PNG_INTERLACE_ADAM7 => passes = PNG_INTERLACE_ADAM7_PASSES,
        _ => png_error(png_ptr, c"unknown interlace type".as_ptr()),
    }

    {
        let height = (*image).height;
        let width = (*image).width;
        let proc = (*display).colormap_processing;
        let first_row = (*display).first_row as png_bytep;
        let row_step = (*display).row_step;

        let mut pass = 0;
        while pass < passes {
            let startx: c_uint;
            let stepx: c_uint;
            let stepy: c_uint;
            let mut y: png_uint_32;

            if (*png_ptr).interlaced as c_int == PNG_INTERLACE_ADAM7 {
                if png_pass_cols(width, pass) == 0 {
                    pass += 1;
                    continue;
                }

                startx = png_pass_start_col(pass) as c_uint;
                stepx = png_pass_col_offset(pass) as c_uint;
                y = png_pass_start_row(pass) as png_uint_32;
                stepy = png_pass_row_offset(pass) as c_uint;
            } else {
                y = 0;
                startx = 0;
                stepx = 1;
                stepy = 1;
            }

            while y < height {
                let mut inrow = (*display).local_row as png_bytep;
                let mut outrow = first_row.offset((y as isize) * row_step);
                let row_end = outrow.offset(width as isize);

                png_read_row(png_ptr, inrow, ptr::null_mut());

                outrow = outrow.offset(startx as isize);
                match proc as c_uint {
                    x if x == PNG_CMAP_GA => {
                        while outrow < row_end {
                            let gray = *inrow as c_uint;
                            inrow = inrow.add(1);
                            let alpha = *inrow as c_uint;
                            inrow = inrow.add(1);
                            let entry: c_uint;

                            if alpha > 229 {
                                entry = (231 * gray + 128) >> 8;
                            } else if alpha < 26 {
                                entry = 231;
                            } else {
                                entry = 226 + 6 * png_div51(alpha) + png_div51(gray);
                            }

                            *outrow = entry as png_byte;
                            outrow = outrow.offset(stepx as isize);
                        }
                    }
                    x if x == PNG_CMAP_TRANS => {
                        while outrow < row_end {
                            let gray = *inrow;
                            inrow = inrow.add(1);
                            let alpha = *inrow;
                            inrow = inrow.add(1);

                            if alpha == 0 {
                                *outrow = PNG_CMAP_TRANS_BACKGROUND as png_byte;
                            } else if gray != PNG_CMAP_TRANS_BACKGROUND as png_byte {
                                *outrow = gray;
                            } else {
                                *outrow = (PNG_CMAP_TRANS_BACKGROUND + 1) as png_byte;
                            }
                            outrow = outrow.offset(stepx as isize);
                        }
                    }
                    x if x == PNG_CMAP_RGB => {
                        while outrow < row_end {
                            *outrow = png_rgb_index(
                                *inrow.add(0) as png_uint_32,
                                *inrow.add(1) as png_uint_32,
                                *inrow.add(2) as png_uint_32,
                            );
                            inrow = inrow.add(3);
                            outrow = outrow.offset(stepx as isize);
                        }
                    }
                    x if x == PNG_CMAP_RGB_ALPHA => {
                        while outrow < row_end {
                            let alpha = *inrow.add(3) as c_uint;

                            if alpha >= 196 {
                                *outrow = png_rgb_index(
                                    *inrow.add(0) as png_uint_32,
                                    *inrow.add(1) as png_uint_32,
                                    *inrow.add(2) as png_uint_32,
                                );
                            } else if alpha < 64 {
                                *outrow = PNG_CMAP_RGB_ALPHA_BACKGROUND as png_byte;
                            } else {
                                let mut back_i = PNG_CMAP_RGB_ALPHA_BACKGROUND + 1;

                                if *inrow.add(0) & 0x80 != 0 {
                                    back_i += 9;
                                }
                                if *inrow.add(0) & 0x40 != 0 {
                                    back_i += 9;
                                }
                                if *inrow.add(1) & 0x80 != 0 {
                                    back_i += 3;
                                }
                                if *inrow.add(1) & 0x40 != 0 {
                                    back_i += 3;
                                }
                                if *inrow.add(2) & 0x80 != 0 {
                                    back_i += 1;
                                }
                                if *inrow.add(2) & 0x40 != 0 {
                                    back_i += 1;
                                }

                                *outrow = back_i as png_byte;
                            }

                            inrow = inrow.add(4);
                            outrow = outrow.offset(stepx as isize);
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

unsafe extern "C" fn png_image_read_colormapped(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_read_control;
    let image = (*display).image;
    let control = (*image).opaque;
    let png_ptr = (*control).png_ptr;
    let info_ptr = (*control).info_ptr;

    let mut passes: c_int = 0;

    png_skip_chunks(png_ptr);

    if (*display).colormap_processing == PNG_CMAP_NONE as c_int {
        passes = png_set_interlace_handling(png_ptr);
    }

    png_read_update_info(png_ptr, info_ptr);

    let mut bad_output = false;
    match (*display).colormap_processing as c_uint {
        x if x == PNG_CMAP_NONE => {
            if !(((*info_ptr).color_type == PNG_COLOR_TYPE_PALETTE as png_byte
                || (*info_ptr).color_type == PNG_COLOR_TYPE_GRAY as png_byte)
                && (*info_ptr).bit_depth == 8)
            {
                bad_output = true;
            }
        }
        x if x == PNG_CMAP_TRANS || x == PNG_CMAP_GA => {
            if !((*info_ptr).color_type == PNG_COLOR_TYPE_GRAY_ALPHA as png_byte
                && (*info_ptr).bit_depth == 8
                && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                && (*image).colormap_entries == 256)
            {
                bad_output = true;
            }
        }
        x if x == PNG_CMAP_RGB => {
            if !((*info_ptr).color_type == PNG_COLOR_TYPE_RGB as png_byte
                && (*info_ptr).bit_depth == 8
                && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                && (*image).colormap_entries == 216)
            {
                bad_output = true;
            }
        }
        x if x == PNG_CMAP_RGB_ALPHA => {
            if !((*info_ptr).color_type == PNG_COLOR_TYPE_RGB_ALPHA as png_byte
                && (*info_ptr).bit_depth == 8
                && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                && (*image).colormap_entries == 244)
            {
                bad_output = true;
            }
        }
        _ => {
            bad_output = true;
        }
    }

    if bad_output {
        png_error(png_ptr, c"bad color-map processing (internal error)".as_ptr());
    }

    {
        let mut first_row = (*display).buffer;
        let row_step = (*display).row_stride as isize;

        if row_step < 0 {
            let mut p = first_row as *mut c_char;
            p = p.offset(((*image).height - 1) as isize * (-row_step));
            first_row = p as png_voidp;
        }

        (*display).first_row = first_row;
        (*display).row_step = row_step;
    }

    if passes == 0 {
        let result: c_int;
        let row = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr) as png_alloc_size_t);

        (*display).local_row = row;
        result = png_safe_execute(image, Some(png_image_read_and_map), display as png_voidp);
        (*display).local_row = ptr::null_mut();
        png_free(png_ptr, row);

        result
    } else {
        let row_step = (*display).row_step;

        loop {
            passes -= 1;
            if passes < 0 {
                break;
            }
            let mut y = (*image).height;
            let mut row = (*display).first_row as png_bytep;

            while y > 0 {
                png_read_row(png_ptr, row, ptr::null_mut());
                row = row.offset(row_step);
                y -= 1;
            }
        }

        1
    }
}

unsafe extern "C" fn png_image_read_direct_scaled(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_read_control;
    let image = (*display).image;
    let png_ptr = (*(*image).opaque).png_ptr;
    let info_ptr = (*(*image).opaque).info_ptr;
    let local_row = (*display).local_row as png_bytep;
    let first_row = (*display).first_row as png_bytep;
    let row_step = (*display).row_step;
    let row_bytes = png_get_rowbytes(png_ptr, info_ptr);
    let mut passes: c_int;

    match (*png_ptr).interlaced as c_int {
        x if x == PNG_INTERLACE_NONE => passes = 1,
        x if x == PNG_INTERLACE_ADAM7 => passes = PNG_INTERLACE_ADAM7_PASSES,
        _ => png_error(png_ptr, c"unknown interlace type".as_ptr()),
    }

    loop {
        passes -= 1;
        if passes < 0 {
            break;
        }
        let mut y = (*image).height;
        let mut output_row = first_row;

        while y > 0 {
            png_read_row(png_ptr, local_row, ptr::null_mut());

            memcpy(output_row as *mut c_void, local_row as *const c_void, row_bytes);
            output_row = output_row.offset(row_step);
            y -= 1;
        }
    }

    1
}

unsafe extern "C" fn png_image_read_composite(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_read_control;
    let image = (*display).image;
    let png_ptr = (*(*image).opaque).png_ptr;
    let passes: c_int;

    match (*png_ptr).interlaced as c_int {
        x if x == PNG_INTERLACE_NONE => passes = 1,
        x if x == PNG_INTERLACE_ADAM7 => passes = PNG_INTERLACE_ADAM7_PASSES,
        _ => png_error(png_ptr, c"unknown interlace type".as_ptr()),
    }

    {
        let height = (*image).height;
        let width = (*image).width;
        let row_step = (*display).row_step;
        let channels: c_uint = if ((*image).format & PNG_FORMAT_FLAG_COLOR) != 0 {
            3
        } else {
            1
        };
        let optimize_alpha = ((*png_ptr).flags & PNG_FLAG_OPTIMIZE_ALPHA) != 0;

        let mut pass = 0;
        while pass < passes {
            let startx: c_uint;
            let stepx: c_uint;
            let stepy: c_uint;
            let mut y: png_uint_32;

            if (*png_ptr).interlaced as c_int == PNG_INTERLACE_ADAM7 {
                if png_pass_cols(width, pass) == 0 {
                    pass += 1;
                    continue;
                }

                startx = png_pass_start_col(pass) as c_uint * channels;
                stepx = png_pass_col_offset(pass) as c_uint * channels;
                y = png_pass_start_row(pass) as png_uint_32;
                stepy = png_pass_row_offset(pass) as c_uint;
            } else {
                y = 0;
                startx = 0;
                stepx = channels;
                stepy = 1;
            }

            while y < height {
                let mut inrow = (*display).local_row as png_bytep;
                let mut outrow: png_bytep;
                let row_end: png_const_bytep;

                png_read_row(png_ptr, inrow, ptr::null_mut());

                outrow = (*display).first_row as png_bytep;
                outrow = outrow.offset((y as isize) * row_step);
                row_end = outrow.offset((width * channels) as isize);

                outrow = outrow.offset(startx as isize);
                while (outrow as png_const_bytep) < row_end {
                    let alpha = *inrow.add(channels as usize);

                    if alpha > 0 {
                        let mut c = 0u32;
                        while c < channels {
                            let mut component = *inrow.add(c as usize) as png_uint_32;

                            if alpha < 255 {
                                if optimize_alpha {
                                    component *= 257 * 255;
                                    component += (255 - alpha as png_uint_32)
                                        * png_sRGB_table[*outrow.add(c as usize) as usize]
                                            as png_uint_32;

                                    if component > 255 * 65535 {
                                        component = 255 * 65535;
                                    }

                                    component = png_srgb_from_linear(component) as png_uint_32;
                                } else {
                                    let background = *outrow.add(c as usize) as png_uint_32;
                                    component +=
                                        ((255 - alpha as png_uint_32) * background + 127) / 255;
                                    if component > 255 {
                                        component = 255;
                                    }
                                }
                            }

                            *outrow.add(c as usize) = component as png_byte;
                            c += 1;
                        }
                    }

                    inrow = inrow.add((channels + 1) as usize);
                    outrow = outrow.offset(stepx as isize);
                }
                y += stepy;
            }
            pass += 1;
        }
    }

    1
}

unsafe extern "C" fn png_image_read_background(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_read_control;
    let image = (*display).image;
    let png_ptr = (*(*image).opaque).png_ptr;
    let info_ptr = (*(*image).opaque).info_ptr;
    let height = (*image).height;
    let width = (*image).width;
    let passes: c_int;

    if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) == 0 {
        png_error(png_ptr, c"lost rgb to gray".as_ptr());
    }

    if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
        png_error(png_ptr, c"unexpected compose".as_ptr());
    }

    if png_get_channels(png_ptr, info_ptr) != 2 {
        png_error(png_ptr, c"lost/gained channels".as_ptr());
    }

    if ((*image).format & PNG_FORMAT_FLAG_LINEAR) == 0
        && ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0
    {
        png_error(png_ptr, c"unexpected 8-bit transformation".as_ptr());
    }

    match (*png_ptr).interlaced as c_int {
        x if x == PNG_INTERLACE_NONE => passes = 1,
        x if x == PNG_INTERLACE_ADAM7 => passes = PNG_INTERLACE_ADAM7_PASSES,
        _ => png_error(png_ptr, c"unknown interlace type".as_ptr()),
    }

    match (*info_ptr).bit_depth as c_int {
        8 => {
            let first_row = (*display).first_row as png_bytep;
            let row_step = (*display).row_step;

            let mut pass = 0;
            while pass < passes {
                let startx: c_uint;
                let stepx: c_uint;
                let stepy: c_uint;
                let mut y: png_uint_32;

                if (*png_ptr).interlaced as c_int == PNG_INTERLACE_ADAM7 {
                    if png_pass_cols(width, pass) == 0 {
                        pass += 1;
                        continue;
                    }

                    startx = png_pass_start_col(pass) as c_uint;
                    stepx = png_pass_col_offset(pass) as c_uint;
                    y = png_pass_start_row(pass) as png_uint_32;
                    stepy = png_pass_row_offset(pass) as c_uint;
                } else {
                    y = 0;
                    startx = 0;
                    stepx = 1;
                    stepy = 1;
                }

                if (*display).background.is_null() {
                    while y < height {
                        let mut inrow = (*display).local_row as png_bytep;
                        let mut outrow = first_row.offset((y as isize) * row_step);
                        let row_end = outrow.offset(width as isize);

                        png_read_row(png_ptr, inrow, ptr::null_mut());

                        outrow = outrow.offset(startx as isize);
                        while outrow < row_end {
                            let alpha = *inrow.add(1);

                            if alpha > 0 {
                                let mut component = *inrow.add(0) as png_uint_32;

                                if alpha < 255 {
                                    component = png_sRGB_table[component as usize] as png_uint_32
                                        * alpha as png_uint_32;
                                    component += png_sRGB_table[*outrow.add(0) as usize]
                                        as png_uint_32
                                        * (255 - alpha as png_uint_32);
                                    component = png_srgb_from_linear(component) as png_uint_32;
                                }

                                *outrow.add(0) = component as png_byte;
                            }

                            inrow = inrow.add(2);
                            outrow = outrow.offset(stepx as isize);
                        }
                        y += stepy;
                    }
                } else {
                    let background8 = (*(*display).background).green;
                    let background = png_sRGB_table[background8 as usize];

                    while y < height {
                        let mut inrow = (*display).local_row as png_bytep;
                        let mut outrow = first_row.offset((y as isize) * row_step);
                        let row_end = outrow.offset(width as isize);

                        png_read_row(png_ptr, inrow, ptr::null_mut());

                        outrow = outrow.offset(startx as isize);
                        while outrow < row_end {
                            let alpha = *inrow.add(1);

                            if alpha > 0 {
                                let mut component = *inrow.add(0) as png_uint_32;

                                if alpha < 255 {
                                    component = png_sRGB_table[component as usize] as png_uint_32
                                        * alpha as png_uint_32;
                                    component +=
                                        background as png_uint_32 * (255 - alpha as png_uint_32);
                                    component = png_srgb_from_linear(component) as png_uint_32;
                                }

                                *outrow.add(0) = component as png_byte;
                            } else {
                                *outrow.add(0) = background8;
                            }

                            inrow = inrow.add(2);
                            outrow = outrow.offset(stepx as isize);
                        }
                        y += stepy;
                    }
                }
                pass += 1;
            }
        }

        16 => {
            let first_row = (*display).first_row as png_uint_16p;
            let row_step = (*display).row_step / 2;
            let preserve_alpha = (((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0) as c_uint;
            let outchannels = 1u32 + preserve_alpha;
            let mut swap_alpha: c_int = 0;

            // PNG_SIMPLIFIED_READ_AFIRST_SUPPORTED
            if preserve_alpha != 0 && ((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0 {
                swap_alpha = 1;
            }

            let mut pass = 0;
            while pass < passes {
                let startx: c_uint;
                let stepx: c_uint;
                let stepy: c_uint;
                let mut y: png_uint_32;

                if (*png_ptr).interlaced as c_int == PNG_INTERLACE_ADAM7 {
                    if png_pass_cols(width, pass) == 0 {
                        pass += 1;
                        continue;
                    }

                    startx = png_pass_start_col(pass) as c_uint * outchannels;
                    stepx = png_pass_col_offset(pass) as c_uint * outchannels;
                    y = png_pass_start_row(pass) as png_uint_32;
                    stepy = png_pass_row_offset(pass) as c_uint;
                } else {
                    y = 0;
                    startx = 0;
                    stepx = outchannels;
                    stepy = 1;
                }

                while y < height {
                    let mut inrow: png_const_uint_16p;
                    let mut outrow = first_row.offset((y as isize) * row_step);
                    let row_end = outrow.offset((width * outchannels) as isize);

                    png_read_row(png_ptr, (*display).local_row as png_bytep, ptr::null_mut());
                    inrow = (*display).local_row as png_const_uint_16p;

                    outrow = outrow.offset(startx as isize);
                    while outrow < row_end {
                        let mut component = *inrow.add(0) as png_uint_32;
                        let alpha = *inrow.add(1);

                        if alpha > 0 {
                            if alpha < 65535 {
                                component *= alpha as png_uint_32;
                                component += 32767;
                                component /= 65535;
                            }
                        } else {
                            component = 0;
                        }

                        *outrow.offset(swap_alpha as isize) = component as png_uint_16;
                        if preserve_alpha != 0 {
                            *outrow.offset((1 ^ swap_alpha) as isize) = alpha;
                        }

                        inrow = inrow.add(2);
                        outrow = outrow.offset(stepx as isize);
                    }
                    y += stepy;
                }
                pass += 1;
            }
        }

        // __GNUC__ default
        _ => {
            png_error(png_ptr, c"unexpected bit depth".as_ptr());
        }
    }

    1
}

unsafe extern "C" fn png_image_read_direct(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_read_control;
    let image = (*display).image;
    let png_ptr = (*(*image).opaque).png_ptr;
    let info_ptr = (*(*image).opaque).info_ptr;

    let mut format = (*image).format;
    let linear = ((format & PNG_FORMAT_FLAG_LINEAR) != 0) as c_int;
    let mut do_local_compose: c_int = 0;
    let mut do_local_background: c_int = 0;
    let mut do_local_scale: c_int = 0;
    let mut passes: c_int = 0;

    png_set_expand(png_ptr);

    {
        let base_format = png_image_format(png_ptr) & !PNG_FORMAT_FLAG_COLORMAP;
        let mut change = format ^ base_format;
        let output_gamma: png_fixed_point;
        let mut mode: c_int;

        if (change & PNG_FORMAT_FLAG_COLOR) != 0 {
            if (format & PNG_FORMAT_FLAG_COLOR) != 0 {
                png_set_gray_to_rgb(png_ptr);
            } else {
                if (base_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    do_local_background = 1;
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
                mode = PNG_ALPHA_STANDARD;
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

        if do_local_background != 0 {
            let mut gtest: png_fixed_point = 0;

            if png_muldiv(
                &mut gtest,
                output_gamma,
                png_resolve_file_gamma(png_ptr),
                PNG_FP_1,
            ) != 0
                && png_gamma_significant(gtest) == 0
            {
                do_local_background = 0;
            } else if mode == PNG_ALPHA_STANDARD {
                do_local_background = 2;
                mode = PNG_ALPHA_PNG;
            }
        }

        if (change & PNG_FORMAT_FLAG_LINEAR) != 0 {
            if linear != 0 {
                png_set_expand_16(png_ptr);
            } else {
                png_set_scale_16(png_ptr);

                if (*png_ptr).interlaced != 0 {
                    do_local_scale = 1;
                }
            }

            change &= !PNG_FORMAT_FLAG_LINEAR;
        }

        if (change & PNG_FORMAT_FLAG_ALPHA) != 0 {
            if (base_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                if do_local_background != 0 {
                    do_local_background = 2;
                } else if linear != 0 {
                    png_set_strip_alpha(png_ptr);
                } else if !(*display).background.is_null() {
                    let mut c: png_color_16 = core::mem::zeroed();

                    c.index = 0;
                    c.red = (*(*display).background).red as png_uint_16;
                    c.green = (*(*display).background).green as png_uint_16;
                    c.blue = (*(*display).background).blue as png_uint_16;
                    c.gray = (*(*display).background).green as png_uint_16;

                    png_set_background_fixed(png_ptr, &c, PNG_BACKGROUND_GAMMA_SCREEN, 0, 0);
                } else {
                    do_local_compose = 1;
                    mode = PNG_ALPHA_OPTIMIZED;
                }
            } else {
                let filler: png_uint_32;
                let where_: c_int;

                if linear != 0 {
                    filler = 65535;
                } else {
                    filler = 255;
                }

                // PNG_FORMAT_AFIRST_SUPPORTED
                if (format & PNG_FORMAT_FLAG_AFIRST) != 0 {
                    where_ = PNG_FILLER_BEFORE;
                    change &= !PNG_FORMAT_FLAG_AFIRST;
                } else {
                    where_ = PNG_FILLER_AFTER;
                }

                png_set_add_alpha(png_ptr, filler, where_);
            }

            change &= !PNG_FORMAT_FLAG_ALPHA;
        }

        png_set_alpha_mode_fixed(png_ptr, mode, output_gamma);

        // PNG_FORMAT_BGR_SUPPORTED
        if (change & PNG_FORMAT_FLAG_BGR) != 0 {
            if (format & PNG_FORMAT_FLAG_COLOR) != 0 {
                png_set_bgr(png_ptr);
            } else {
                format &= !PNG_FORMAT_FLAG_BGR;
            }

            change &= !PNG_FORMAT_FLAG_BGR;
        }

        // PNG_FORMAT_AFIRST_SUPPORTED
        if (change & PNG_FORMAT_FLAG_AFIRST) != 0 {
            if (format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                if do_local_background != 2 {
                    png_set_swap_alpha(png_ptr);
                }
            } else {
                format &= !PNG_FORMAT_FLAG_AFIRST;
            }

            change &= !PNG_FORMAT_FLAG_AFIRST;
        }

        if linear != 0 {
            let le: png_uint_16 = 0x0001;

            if (*(&le as *const png_uint_16 as png_const_bytep) & (le as png_byte)) != 0 {
                png_set_swap(png_ptr);
            }
        }

        if change != 0 {
            png_error(png_ptr, c"png_read_image: unsupported transformation".as_ptr());
        }
    }

    png_skip_chunks(png_ptr);

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
            if do_local_compose == 0 {
                if do_local_background != 2 || (format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    info_format |= PNG_FORMAT_FLAG_ALPHA;
                }
            }
        } else if do_local_compose != 0 {
            png_error(png_ptr, c"png_image_read: alpha channel lost".as_ptr());
        }

        if (format & PNG_FORMAT_FLAG_ASSOCIATED_ALPHA) != 0 {
            info_format |= PNG_FORMAT_FLAG_ASSOCIATED_ALPHA;
        }

        if (*info_ptr).bit_depth == 16 {
            info_format |= PNG_FORMAT_FLAG_LINEAR;
        }

        // PNG_FORMAT_BGR_SUPPORTED
        if ((*png_ptr).transformations & PNG_BGR) != 0 {
            info_format |= PNG_FORMAT_FLAG_BGR;
        }

        // PNG_FORMAT_AFIRST_SUPPORTED
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

        if info_format != format {
            png_error(png_ptr, c"png_read_image: invalid transformations".as_ptr());
        }
    }

    {
        let mut first_row = (*display).buffer;
        let mut row_step = (*display).row_stride as isize;

        if linear != 0 {
            row_step *= 2;
        }

        if row_step < 0 {
            let mut p = first_row as *mut c_char;
            p = p.offset(((*image).height - 1) as isize * (-row_step));
            first_row = p as png_voidp;
        }

        (*display).first_row = first_row;
        (*display).row_step = row_step;
    }

    if do_local_compose != 0 {
        let result: c_int;
        let row = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr) as png_alloc_size_t);

        (*display).local_row = row;
        result = png_safe_execute(image, Some(png_image_read_composite), display as png_voidp);
        (*display).local_row = ptr::null_mut();
        png_free(png_ptr, row);

        result
    } else if do_local_background == 2 {
        let result: c_int;
        let row = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr) as png_alloc_size_t);

        (*display).local_row = row;
        result = png_safe_execute(image, Some(png_image_read_background), display as png_voidp);
        (*display).local_row = ptr::null_mut();
        png_free(png_ptr, row);

        result
    } else if do_local_scale != 0 {
        let result: c_int;
        let row = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr) as png_alloc_size_t);

        (*display).local_row = row;
        result = png_safe_execute(image, Some(png_image_read_direct_scaled), display as png_voidp);
        (*display).local_row = ptr::null_mut();
        png_free(png_ptr, row);

        result
    } else {
        let row_step = (*display).row_step;

        loop {
            passes -= 1;
            if passes < 0 {
                break;
            }
            let mut y = (*image).height;
            let mut row = (*display).first_row as png_bytep;

            while y > 0 {
                png_read_row(png_ptr, row, ptr::null_mut());
                row = row.offset(row_step);
                y -= 1;
            }
        }

        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_finish_read(
    image: png_imagep,
    background: png_const_colorp,
    buffer: *mut c_void,
    mut row_stride: png_int_32,
    colormap: *mut c_void,
) -> c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        let channels = png_image_pixel_channels((*image).format);

        if (*image).width <= 0x7fffffffu32 / channels {
            let check: png_uint_32;
            let png_row_stride: png_uint_32 = (*image).width * channels;

            if row_stride == 0 {
                row_stride = png_row_stride as png_int_32;
            }

            if row_stride < 0 {
                check = (row_stride as png_uint_32).wrapping_neg();
            } else {
                check = row_stride as png_uint_32;
            }

            if !(*image).opaque.is_null() && !buffer.is_null() && check >= png_row_stride {
                if (*image).height
                    <= 0xffffffffu32 / png_image_pixel_component_size((*image).format) / check
                {
                    if ((*image).format & PNG_FORMAT_FLAG_COLORMAP) == 0
                        || ((*image).colormap_entries > 0 && !colormap.is_null())
                    {
                        let result: c_int;
                        let mut display: png_image_read_control = core::mem::zeroed();

                        display.image = image;
                        display.buffer = buffer;
                        display.row_stride = row_stride;
                        display.colormap = colormap;
                        display.background = background;
                        display.local_row = ptr::null_mut();

                        if ((*image).format & PNG_FORMAT_FLAG_COLORMAP) != 0 {
                            result = (png_safe_execute(
                                image,
                                Some(png_image_read_colormap),
                                &mut display as *mut _ as png_voidp,
                            ) != 0
                                && png_safe_execute(
                                    image,
                                    Some(png_image_read_colormapped),
                                    &mut display as *mut _ as png_voidp,
                                ) != 0) as c_int;
                        } else {
                            result = png_safe_execute(
                                image,
                                Some(png_image_read_direct),
                                &mut display as *mut _ as png_voidp,
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
    } else if !image.is_null() {
        return png_image_error(
            image,
            c"png_image_finish_read: damaged PNG_IMAGE_VERSION".as_ptr(),
        );
    }

    0
}
