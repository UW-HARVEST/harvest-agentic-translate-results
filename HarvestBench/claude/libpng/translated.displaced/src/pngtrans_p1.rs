use crate::*;

/* Turn on BGR-to-RGB mapping */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_bgr(png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).transformations |= PNG_BGR;
}

/* Turn on 16-bit byte swapping */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_swap(png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }

    if (*png_ptr).bit_depth == 16 {
        (*png_ptr).transformations |= PNG_SWAP_BYTES;
    }
}

/* Turn on pixel packing */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_packing(png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }

    if (*png_ptr).bit_depth < 8 {
        (*png_ptr).transformations |= PNG_PACK;

        (*png_ptr).usr_bit_depth = 8;
    }
}

/* Turn on packed pixel swapping */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_packswap(png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }

    if (*png_ptr).bit_depth < 8 {
        (*png_ptr).transformations |= PNG_PACKSWAP;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_shift(png_ptr: png_structrp, true_bits: png_const_color_8p) {
    if png_ptr.is_null() || true_bits.is_null() {
        return;
    }

    /* Check the shift values before passing them on to png_do_shift. */
    {
        let bit_depth: png_byte = (*png_ptr).bit_depth;
        let mut invalid: c_int = 0;

        if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
            if (*true_bits).red == 0
                || (*true_bits).red > bit_depth
                || (*true_bits).green == 0
                || (*true_bits).green > bit_depth
                || (*true_bits).blue == 0
                || (*true_bits).blue > bit_depth
            {
                invalid = 1;
            }
        } else {
            if (*true_bits).gray == 0 || (*true_bits).gray > bit_depth {
                invalid = 1;
            }
        }

        if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0
            && ((*true_bits).alpha == 0 || (*true_bits).alpha > bit_depth)
        {
            invalid = 1;
        }

        if invalid != 0 {
            png_app_error(
                png_ptr as png_const_structrp,
                cstr!("png_set_shift: invalid shift values"),
            );
            return;
        }
    }

    (*png_ptr).transformations |= PNG_SHIFT;
    (*png_ptr).shift = *true_bits;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_interlace_handling(png_ptr: png_structrp) -> c_int {
    if !png_ptr.is_null() && (*png_ptr).interlaced != 0 {
        (*png_ptr).transformations |= PNG_INTERLACE;
        return 7;
    }

    1
}

/* Add a filler byte on read, or remove a filler or alpha byte on write.
 * The filler type has changed in v0.95 to allow future 2-byte fillers
 * for 48-bit input data, as well as to avoid problems with some compilers
 * that don't like bytes as parameters.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filler(png_ptr: png_structrp, filler: png_uint_32, flags: c_int) {
    if png_ptr.is_null() {
        return;
    }

    /* In libpng 1.6 it is possible to determine whether this is a read or write
     * operation and therefore to do more checking here for a valid call.
     */
    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
        /* On read png_set_filler is always valid, regardless of the base PNG
         * format, because other transformations can give a format where the
         * filler code can execute (basically an 8 or 16-bit component RGB or G
         * format.)
         *
         * NOTE: usr_channels is not used by the read code!  (This has led to
         * confusion in the past.)  The filler is only used in the read code.
         */
        (*png_ptr).filler = filler as png_uint_16;
    } else
    /* write */
    {
        /* On write the usr_channels parameter must be set correctly at the
         * start to record the number of channels in the app-supplied data.
         */
        match (*png_ptr).color_type as c_int {
            PNG_COLOR_TYPE_RGB => {
                (*png_ptr).usr_channels = 4;
            }

            PNG_COLOR_TYPE_GRAY => {
                if (*png_ptr).bit_depth >= 8 {
                    (*png_ptr).usr_channels = 2;
                } else {
                    /* There simply isn't any code in libpng to strip out bits
                     * from bytes when the components are less than a byte in
                     * size!
                     */
                    png_app_error(
                        png_ptr as png_const_structrp,
                        cstr!("png_set_filler is invalid for low bit depth gray output"),
                    );
                    return;
                }
            }

            _ => {
                png_app_error(
                    png_ptr as png_const_structrp,
                    cstr!("png_set_filler: inappropriate color type"),
                );
                return;
            }
        }
    }

    /* Here on success - libpng supports the operation, set the transformation
     * and the flag to say where the filler channel is.
     */
    (*png_ptr).transformations |= PNG_FILLER;

    if flags == PNG_FILLER_AFTER {
        (*png_ptr).flags |= PNG_FLAG_FILLER_AFTER;
    } else {
        (*png_ptr).flags &= !PNG_FLAG_FILLER_AFTER;
    }
}

/* Added to libpng-1.2.7 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_add_alpha(
    png_ptr: png_structrp,
    filler: png_uint_32,
    flags: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    png_set_filler(png_ptr, filler, flags);
    /* The above may fail to do anything. */
    if ((*png_ptr).transformations & PNG_FILLER) != 0 {
        (*png_ptr).transformations |= PNG_ADD_ALPHA;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_swap_alpha(png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).transformations |= PNG_SWAP_ALPHA;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_invert_alpha(png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).transformations |= PNG_INVERT_ALPHA;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_invert_mono(png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).transformations |= PNG_INVERT_MONO;
}

/* Invert monochrome grayscale data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_invert(row_info: png_row_infop, row: png_bytep) {
    /* This test removed from libpng version 1.0.13 and 1.2.0:
     *   if (row_info->bit_depth == 1 &&
     */
    if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY {
        let mut rp: png_bytep = row;
        let mut i: usize;
        let istop: usize = (*row_info).rowbytes;

        i = 0;
        while i < istop {
            *rp = !(*rp);
            rp = rp.offset(1);
            i += 1;
        }
    } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA
        && (*row_info).bit_depth == 8
    {
        let mut rp: png_bytep = row;
        let mut i: usize;
        let istop: usize = (*row_info).rowbytes;

        i = 0;
        while i < istop {
            *rp = !(*rp);
            rp = rp.offset(2);
            i += 2;
        }
    } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA
        && (*row_info).bit_depth == 16
    {
        let mut rp: png_bytep = row;
        let mut i: usize;
        let istop: usize = (*row_info).rowbytes;

        i = 0;
        while i < istop {
            *rp = !(*rp);
            *rp.offset(1) = !(*rp.offset(1));
            rp = rp.offset(4);
            i += 4;
        }
    }
}
