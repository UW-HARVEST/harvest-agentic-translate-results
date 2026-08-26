/* pngtrans.c lines 1..521 */

/* Turn on BGR-to-RGB mapping */
/* png_set_bgr */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_bgr(png_ptr: png_structrp) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).transformations |= PNG_BGR;
}

/* Turn on 16-bit byte swapping */
/* png_set_swap */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_swap(png_ptr: png_structrp) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    if (*png_ptr).bit_depth == 16 {
        (*png_ptr).transformations |= PNG_SWAP_BYTES;
    }
}

/* Turn on pixel packing */
/* png_set_packing */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_packing(png_ptr: png_structrp) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    if (*png_ptr).bit_depth < 8 {
        (*png_ptr).transformations |= PNG_PACK;
        (*png_ptr).usr_bit_depth = 8;
    }
}

/* Turn on packed pixel swapping */
/* png_set_packswap */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_packswap(png_ptr: png_structrp) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    if (*png_ptr).bit_depth < 8 {
        (*png_ptr).transformations |= PNG_PACKSWAP;
    }
}

/* png_set_shift */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_shift(png_ptr: png_structrp, true_bits: png_const_color_8p) {
    if png_ptr == core::ptr::null_mut() || true_bits == core::ptr::null() {
        return;
    }

    /* Check the shift values before passing them on to png_do_shift. */
    {
        let bit_depth: png_byte = (*png_ptr).bit_depth;
        let mut invalid: c_int = 0;

        if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
            if (*true_bits).red == 0
                || (*true_bits).red as c_int > bit_depth as c_int
                || (*true_bits).green == 0
                || (*true_bits).green as c_int > bit_depth as c_int
                || (*true_bits).blue == 0
                || (*true_bits).blue as c_int > bit_depth as c_int
            {
                invalid = 1;
            }
        } else {
            if (*true_bits).gray == 0 || (*true_bits).gray as c_int > bit_depth as c_int {
                invalid = 1;
            }
        }

        if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0
            && ((*true_bits).alpha == 0 || (*true_bits).alpha as c_int > bit_depth as c_int)
        {
            invalid = 1;
        }

        if invalid != 0 {
            png_app_error(
                png_ptr,
                b"png_set_shift: invalid shift values\0".as_ptr() as png_const_charp,
            );
            return;
        }
    }

    (*png_ptr).transformations |= PNG_SHIFT;
    (*png_ptr).shift = *true_bits;
}

/* png_set_interlace_handling */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_interlace_handling(png_ptr: png_structrp) -> c_int {
    if png_ptr != core::ptr::null_mut() && (*png_ptr).interlaced != 0 {
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
/* png_set_filler */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filler(
    png_ptr: png_structrp,
    filler: png_uint_32,
    filler_loc: c_int,
) {
    if png_ptr == core::ptr::null_mut() {
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
                        png_ptr,
                        b"png_set_filler is invalid for low bit depth gray output\0".as_ptr()
                            as png_const_charp,
                    );
                    return;
                }
            }

            _ => {
                png_app_error(
                    png_ptr,
                    b"png_set_filler: inappropriate color type\0".as_ptr() as png_const_charp,
                );
                return;
            }
        }
    }

    /* Here on success - libpng supports the operation, set the transformation
     * and the flag to say where the filler channel is.
     */
    (*png_ptr).transformations |= PNG_FILLER;

    if filler_loc == PNG_FILLER_AFTER {
        (*png_ptr).flags |= PNG_FLAG_FILLER_AFTER;
    } else {
        (*png_ptr).flags &= !PNG_FLAG_FILLER_AFTER;
    }
}

/* Added to libpng-1.2.7 */
/* png_set_add_alpha */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_add_alpha(
    png_ptr: png_structrp,
    filler: png_uint_32,
    filler_loc: c_int,
) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    png_set_filler(png_ptr, filler, filler_loc);
    /* The above may fail to do anything. */
    if ((*png_ptr).transformations & PNG_FILLER) != 0 {
        (*png_ptr).transformations |= PNG_ADD_ALPHA;
    }
}

/* png_set_swap_alpha */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_swap_alpha(png_ptr: png_structrp) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).transformations |= PNG_SWAP_ALPHA;
}

/* png_set_invert_alpha */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_invert_alpha(png_ptr: png_structrp) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).transformations |= PNG_INVERT_ALPHA;
}

/* png_set_invert_mono */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_invert_mono(png_ptr: png_structrp) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).transformations |= PNG_INVERT_MONO;
}

/* Invert monochrome grayscale data */
/* png_do_invert */
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
            *rp = !(*rp) as png_byte;
            rp = rp.add(1);
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
            *rp = !(*rp) as png_byte;
            rp = rp.add(2);
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
            *rp = !(*rp) as png_byte;
            *rp.add(1) = !(*rp.add(1)) as png_byte;
            rp = rp.add(4);
            i += 4;
        }
    }
}

/* Swaps byte order on 16-bit depth images */
/* png_do_swap */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_swap(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth == 16 {
        let mut rp: png_bytep = row;
        let mut i: png_uint_32;
        let istop: png_uint_32 = (*row_info).width.wrapping_mul((*row_info).channels as png_uint_32);

        i = 0;
        while i < istop {
            let t: png_byte = *rp;
            *rp = *rp.add(1);
            *rp.add(1) = t;

            i = i.wrapping_add(1);
            rp = rp.add(2);
        }
    }
}

/* Swaps pixel packing order within bytes */
/* png_do_packswap */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_packswap(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth < 8 {
        let table: png_const_bytep;
        let mut rp: png_bytep;
        let row_end: png_bytep = row.add((*row_info).rowbytes);

        if (*row_info).bit_depth == 1 {
            table = onebppswaptable.as_ptr();
        } else if (*row_info).bit_depth == 2 {
            table = twobppswaptable.as_ptr();
        } else if (*row_info).bit_depth == 4 {
            table = fourbppswaptable.as_ptr();
        } else {
            return;
        }

        rp = row;
        while rp < row_end {
            *rp = *table.add(*rp as usize);
            rp = rp.add(1);
        }
    }
}
