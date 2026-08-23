//! Translation of pngtrans.c - transforms shared by read and write code.
use crate::prelude::*;

/* png.h filler location constants (not present in the shared const module). */
const PNG_FILLER_BEFORE: c_int = 0;
const PNG_FILLER_AFTER: c_int = 1;

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
            png_app_error(png_ptr, c"png_set_shift: invalid shift values".as_ptr());
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

/* Add a filler byte on read, or remove a filler or alpha byte on write. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filler(
    png_ptr: png_structrp,
    filler: png_uint_32,
    filler_loc: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    /* In libpng 1.6 it is possible to determine whether this is a read or write
     * operation and therefore to do more checking here for a valid call.
     */
    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
        /* On read png_set_filler is always valid, regardless of the base PNG
         * format, because other transformations can give a format where the
         * filler code can execute.
         */
        (*png_ptr).filler = filler as png_uint_16;
    } else
    /* write */
    {
        /* On write the usr_channels parameter must be set correctly at the
         * start to record the number of channels in the app-supplied data.
         */
        match (*png_ptr).color_type as c_int {
            x if x == PNG_COLOR_TYPE_RGB => {
                (*png_ptr).usr_channels = 4;
            }

            x if x == PNG_COLOR_TYPE_GRAY => {
                if (*png_ptr).bit_depth >= 8 {
                    (*png_ptr).usr_channels = 2;
                } else {
                    /* There simply isn't any code in libpng to strip out bits
                     * from bytes when the components are less than a byte in
                     * size!
                     */
                    png_app_error(
                        png_ptr,
                        c"png_set_filler is invalid for low bit depth gray output".as_ptr(),
                    );
                    return;
                }
            }

            _ => {
                png_app_error(png_ptr, c"png_set_filler: inappropriate color type".as_ptr());
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_add_alpha(
    png_ptr: png_structrp,
    filler: png_uint_32,
    filler_loc: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    png_set_filler(png_ptr, filler, filler_loc);
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
        let mut i: size_t;
        let istop: size_t = (*row_info).rowbytes;

        i = 0;
        while i < istop {
            *rp = !(*rp);
            rp = rp.add(1);
            i += 1;
        }
    } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA
        && (*row_info).bit_depth == 8
    {
        let mut rp: png_bytep = row;
        let mut i: size_t;
        let istop: size_t = (*row_info).rowbytes;

        i = 0;
        while i < istop {
            *rp = !(*rp);
            rp = rp.add(2);
            i += 2;
        }
    } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA
        && (*row_info).bit_depth == 16
    {
        let mut rp: png_bytep = row;
        let mut i: size_t;
        let istop: size_t = (*row_info).rowbytes;

        i = 0;
        while i < istop {
            *rp = !(*rp);
            *(rp.add(1)) = !(*(rp.add(1)));
            rp = rp.add(4);
            i += 4;
        }
    }
}

/* Swaps byte order on 16-bit depth images */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_swap(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth == 16 {
        let mut rp: png_bytep = row;
        let mut i: png_uint_32;
        let istop: png_uint_32 =
            (*row_info).width.wrapping_mul((*row_info).channels as png_uint_32);

        i = 0;
        while i < istop {
            let t: png_byte = *rp;
            *rp = *(rp.add(1));
            *(rp.add(1)) = t;

            i += 1;
            rp = rp.add(2);
        }
    }
}

static ONEBPPSWAPTABLE: [png_byte; 256] = [
    0x00, 0x80, 0x40, 0xC0, 0x20, 0xA0, 0x60, 0xE0, 0x10, 0x90, 0x50, 0xD0, 0x30, 0xB0, 0x70, 0xF0,
    0x08, 0x88, 0x48, 0xC8, 0x28, 0xA8, 0x68, 0xE8, 0x18, 0x98, 0x58, 0xD8, 0x38, 0xB8, 0x78, 0xF8,
    0x04, 0x84, 0x44, 0xC4, 0x24, 0xA4, 0x64, 0xE4, 0x14, 0x94, 0x54, 0xD4, 0x34, 0xB4, 0x74, 0xF4,
    0x0C, 0x8C, 0x4C, 0xCC, 0x2C, 0xAC, 0x6C, 0xEC, 0x1C, 0x9C, 0x5C, 0xDC, 0x3C, 0xBC, 0x7C, 0xFC,
    0x02, 0x82, 0x42, 0xC2, 0x22, 0xA2, 0x62, 0xE2, 0x12, 0x92, 0x52, 0xD2, 0x32, 0xB2, 0x72, 0xF2,
    0x0A, 0x8A, 0x4A, 0xCA, 0x2A, 0xAA, 0x6A, 0xEA, 0x1A, 0x9A, 0x5A, 0xDA, 0x3A, 0xBA, 0x7A, 0xFA,
    0x06, 0x86, 0x46, 0xC6, 0x26, 0xA6, 0x66, 0xE6, 0x16, 0x96, 0x56, 0xD6, 0x36, 0xB6, 0x76, 0xF6,
    0x0E, 0x8E, 0x4E, 0xCE, 0x2E, 0xAE, 0x6E, 0xEE, 0x1E, 0x9E, 0x5E, 0xDE, 0x3E, 0xBE, 0x7E, 0xFE,
    0x01, 0x81, 0x41, 0xC1, 0x21, 0xA1, 0x61, 0xE1, 0x11, 0x91, 0x51, 0xD1, 0x31, 0xB1, 0x71, 0xF1,
    0x09, 0x89, 0x49, 0xC9, 0x29, 0xA9, 0x69, 0xE9, 0x19, 0x99, 0x59, 0xD9, 0x39, 0xB9, 0x79, 0xF9,
    0x05, 0x85, 0x45, 0xC5, 0x25, 0xA5, 0x65, 0xE5, 0x15, 0x95, 0x55, 0xD5, 0x35, 0xB5, 0x75, 0xF5,
    0x0D, 0x8D, 0x4D, 0xCD, 0x2D, 0xAD, 0x6D, 0xED, 0x1D, 0x9D, 0x5D, 0xDD, 0x3D, 0xBD, 0x7D, 0xFD,
    0x03, 0x83, 0x43, 0xC3, 0x23, 0xA3, 0x63, 0xE3, 0x13, 0x93, 0x53, 0xD3, 0x33, 0xB3, 0x73, 0xF3,
    0x0B, 0x8B, 0x4B, 0xCB, 0x2B, 0xAB, 0x6B, 0xEB, 0x1B, 0x9B, 0x5B, 0xDB, 0x3B, 0xBB, 0x7B, 0xFB,
    0x07, 0x87, 0x47, 0xC7, 0x27, 0xA7, 0x67, 0xE7, 0x17, 0x97, 0x57, 0xD7, 0x37, 0xB7, 0x77, 0xF7,
    0x0F, 0x8F, 0x4F, 0xCF, 0x2F, 0xAF, 0x6F, 0xEF, 0x1F, 0x9F, 0x5F, 0xDF, 0x3F, 0xBF, 0x7F, 0xFF,
];

static TWOBPPSWAPTABLE: [png_byte; 256] = [
    0x00, 0x40, 0x80, 0xC0, 0x10, 0x50, 0x90, 0xD0, 0x20, 0x60, 0xA0, 0xE0, 0x30, 0x70, 0xB0, 0xF0,
    0x04, 0x44, 0x84, 0xC4, 0x14, 0x54, 0x94, 0xD4, 0x24, 0x64, 0xA4, 0xE4, 0x34, 0x74, 0xB4, 0xF4,
    0x08, 0x48, 0x88, 0xC8, 0x18, 0x58, 0x98, 0xD8, 0x28, 0x68, 0xA8, 0xE8, 0x38, 0x78, 0xB8, 0xF8,
    0x0C, 0x4C, 0x8C, 0xCC, 0x1C, 0x5C, 0x9C, 0xDC, 0x2C, 0x6C, 0xAC, 0xEC, 0x3C, 0x7C, 0xBC, 0xFC,
    0x01, 0x41, 0x81, 0xC1, 0x11, 0x51, 0x91, 0xD1, 0x21, 0x61, 0xA1, 0xE1, 0x31, 0x71, 0xB1, 0xF1,
    0x05, 0x45, 0x85, 0xC5, 0x15, 0x55, 0x95, 0xD5, 0x25, 0x65, 0xA5, 0xE5, 0x35, 0x75, 0xB5, 0xF5,
    0x09, 0x49, 0x89, 0xC9, 0x19, 0x59, 0x99, 0xD9, 0x29, 0x69, 0xA9, 0xE9, 0x39, 0x79, 0xB9, 0xF9,
    0x0D, 0x4D, 0x8D, 0xCD, 0x1D, 0x5D, 0x9D, 0xDD, 0x2D, 0x6D, 0xAD, 0xED, 0x3D, 0x7D, 0xBD, 0xFD,
    0x02, 0x42, 0x82, 0xC2, 0x12, 0x52, 0x92, 0xD2, 0x22, 0x62, 0xA2, 0xE2, 0x32, 0x72, 0xB2, 0xF2,
    0x06, 0x46, 0x86, 0xC6, 0x16, 0x56, 0x96, 0xD6, 0x26, 0x66, 0xA6, 0xE6, 0x36, 0x76, 0xB6, 0xF6,
    0x0A, 0x4A, 0x8A, 0xCA, 0x1A, 0x5A, 0x9A, 0xDA, 0x2A, 0x6A, 0xAA, 0xEA, 0x3A, 0x7A, 0xBA, 0xFA,
    0x0E, 0x4E, 0x8E, 0xCE, 0x1E, 0x5E, 0x9E, 0xDE, 0x2E, 0x6E, 0xAE, 0xEE, 0x3E, 0x7E, 0xBE, 0xFE,
    0x03, 0x43, 0x83, 0xC3, 0x13, 0x53, 0x93, 0xD3, 0x23, 0x63, 0xA3, 0xE3, 0x33, 0x73, 0xB3, 0xF3,
    0x07, 0x47, 0x87, 0xC7, 0x17, 0x57, 0x97, 0xD7, 0x27, 0x67, 0xA7, 0xE7, 0x37, 0x77, 0xB7, 0xF7,
    0x0B, 0x4B, 0x8B, 0xCB, 0x1B, 0x5B, 0x9B, 0xDB, 0x2B, 0x6B, 0xAB, 0xEB, 0x3B, 0x7B, 0xBB, 0xFB,
    0x0F, 0x4F, 0x8F, 0xCF, 0x1F, 0x5F, 0x9F, 0xDF, 0x2F, 0x6F, 0xAF, 0xEF, 0x3F, 0x7F, 0xBF, 0xFF,
];

static FOURBPPSWAPTABLE: [png_byte; 256] = [
    0x00, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xA0, 0xB0, 0xC0, 0xD0, 0xE0, 0xF0,
    0x01, 0x11, 0x21, 0x31, 0x41, 0x51, 0x61, 0x71, 0x81, 0x91, 0xA1, 0xB1, 0xC1, 0xD1, 0xE1, 0xF1,
    0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x82, 0x92, 0xA2, 0xB2, 0xC2, 0xD2, 0xE2, 0xF2,
    0x03, 0x13, 0x23, 0x33, 0x43, 0x53, 0x63, 0x73, 0x83, 0x93, 0xA3, 0xB3, 0xC3, 0xD3, 0xE3, 0xF3,
    0x04, 0x14, 0x24, 0x34, 0x44, 0x54, 0x64, 0x74, 0x84, 0x94, 0xA4, 0xB4, 0xC4, 0xD4, 0xE4, 0xF4,
    0x05, 0x15, 0x25, 0x35, 0x45, 0x55, 0x65, 0x75, 0x85, 0x95, 0xA5, 0xB5, 0xC5, 0xD5, 0xE5, 0xF5,
    0x06, 0x16, 0x26, 0x36, 0x46, 0x56, 0x66, 0x76, 0x86, 0x96, 0xA6, 0xB6, 0xC6, 0xD6, 0xE6, 0xF6,
    0x07, 0x17, 0x27, 0x37, 0x47, 0x57, 0x67, 0x77, 0x87, 0x97, 0xA7, 0xB7, 0xC7, 0xD7, 0xE7, 0xF7,
    0x08, 0x18, 0x28, 0x38, 0x48, 0x58, 0x68, 0x78, 0x88, 0x98, 0xA8, 0xB8, 0xC8, 0xD8, 0xE8, 0xF8,
    0x09, 0x19, 0x29, 0x39, 0x49, 0x59, 0x69, 0x79, 0x89, 0x99, 0xA9, 0xB9, 0xC9, 0xD9, 0xE9, 0xF9,
    0x0A, 0x1A, 0x2A, 0x3A, 0x4A, 0x5A, 0x6A, 0x7A, 0x8A, 0x9A, 0xAA, 0xBA, 0xCA, 0xDA, 0xEA, 0xFA,
    0x0B, 0x1B, 0x2B, 0x3B, 0x4B, 0x5B, 0x6B, 0x7B, 0x8B, 0x9B, 0xAB, 0xBB, 0xCB, 0xDB, 0xEB, 0xFB,
    0x0C, 0x1C, 0x2C, 0x3C, 0x4C, 0x5C, 0x6C, 0x7C, 0x8C, 0x9C, 0xAC, 0xBC, 0xCC, 0xDC, 0xEC, 0xFC,
    0x0D, 0x1D, 0x2D, 0x3D, 0x4D, 0x5D, 0x6D, 0x7D, 0x8D, 0x9D, 0xAD, 0xBD, 0xCD, 0xDD, 0xED, 0xFD,
    0x0E, 0x1E, 0x2E, 0x3E, 0x4E, 0x5E, 0x6E, 0x7E, 0x8E, 0x9E, 0xAE, 0xBE, 0xCE, 0xDE, 0xEE, 0xFE,
    0x0F, 0x1F, 0x2F, 0x3F, 0x4F, 0x5F, 0x6F, 0x7F, 0x8F, 0x9F, 0xAF, 0xBF, 0xCF, 0xDF, 0xEF, 0xFF,
];

/* Swaps pixel packing order within bytes */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_packswap(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth < 8 {
        let table: png_const_bytep;
        let mut rp: png_bytep;
        let row_end: png_bytep = row.add((*row_info).rowbytes);

        if (*row_info).bit_depth == 1 {
            table = ONEBPPSWAPTABLE.as_ptr();
        } else if (*row_info).bit_depth == 2 {
            table = TWOBPPSWAPTABLE.as_ptr();
        } else if (*row_info).bit_depth == 4 {
            table = FOURBPPSWAPTABLE.as_ptr();
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

/* Remove a channel - the channel must be the channel at the start or end
 * (not in the middle) of each pixel.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_strip_channel(
    row_info: png_row_infop,
    row: png_bytep,
    at_start: c_int,
) {
    let mut sp: png_bytep = row; /* source pointer */
    let mut dp: png_bytep = row; /* destination pointer */
    let ep: png_bytep = row.add((*row_info).rowbytes); /* One beyond end of row */

    /* At the start sp will point to the first byte to copy and dp to where
     * it is copied to.  ep always points just beyond the end of the row.
     *
     * at_start:        0 -- convert AG, XG, ARGB, XRGB, AAGG, XXGG, etc.
     *            nonzero -- convert GA, GX, RGBA, RGBX, GGAA, RRGGBBXX, etc.
     */

    /* GA, GX, XG cases */
    if (*row_info).channels == 2 {
        if (*row_info).bit_depth == 8 {
            if at_start != 0 {
                /* Skip initial filler */
                sp = sp.add(1);
            } else {
                /* Skip initial channel and, for sp, the filler */
                sp = sp.add(2);
                dp = dp.add(1);
            }

            /* For a 1 pixel wide image there is nothing to do */
            while sp < ep {
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(2);
            }

            (*row_info).pixel_depth = 8;
        } else if (*row_info).bit_depth == 16 {
            if at_start != 0 {
                /* Skip initial filler */
                sp = sp.add(2);
            } else {
                /* Skip initial channel and, for sp, the filler */
                sp = sp.add(4);
                dp = dp.add(2);
            }

            while sp < ep {
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(3);
            }

            (*row_info).pixel_depth = 16;
        } else {
            return; /* bad bit depth */
        }

        (*row_info).channels = 1;

        /* Finally fix the color type if it records an alpha channel */
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
            (*row_info).color_type = PNG_COLOR_TYPE_GRAY as png_byte;
        }
    }
    /* RGBA, RGBX, XRGB cases */
    else if (*row_info).channels == 4 {
        if (*row_info).bit_depth == 8 {
            if at_start != 0 {
                /* Skip initial filler */
                sp = sp.add(1);
            } else {
                /* Skip initial channels and, for sp, the filler */
                sp = sp.add(4);
                dp = dp.add(3);
            }

            /* Note that the loop adds 3 to dp and 4 to sp each time. */
            while sp < ep {
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(2);
            }

            (*row_info).pixel_depth = 24;
        } else if (*row_info).bit_depth == 16 {
            if at_start != 0 {
                /* Skip initial filler */
                sp = sp.add(2);
            } else {
                /* Skip initial channels and, for sp, the filler */
                sp = sp.add(8);
                dp = dp.add(6);
            }

            while sp < ep {
                /* Copy 6 bytes, skip 2 */
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(3);
            }

            (*row_info).pixel_depth = 48;
        } else {
            return; /* bad bit depth */
        }

        (*row_info).channels = 3;

        /* Finally fix the color type if it records an alpha channel */
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
            (*row_info).color_type = PNG_COLOR_TYPE_RGB as png_byte;
        }
    } else {
        return; /* The filler channel has gone already */
    }

    /* Fix the rowbytes value. */
    (*row_info).rowbytes = dp.offset_from(row) as size_t;
}

/* Swaps red and blue bytes within a pixel */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_bgr(row_info: png_row_infop, row: png_bytep) {
    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        let row_width: png_uint_32 = (*row_info).width;
        if (*row_info).bit_depth == 8 {
            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
                let mut rp: png_bytep;
                let mut i: png_uint_32;

                i = 0;
                rp = row;
                while i < row_width {
                    let save: png_byte = *rp;
                    *rp = *(rp.add(2));
                    *(rp.add(2)) = save;

                    i += 1;
                    rp = rp.add(3);
                }
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                let mut rp: png_bytep;
                let mut i: png_uint_32;

                i = 0;
                rp = row;
                while i < row_width {
                    let save: png_byte = *rp;
                    *rp = *(rp.add(2));
                    *(rp.add(2)) = save;

                    i += 1;
                    rp = rp.add(4);
                }
            }
        } else if (*row_info).bit_depth == 16 {
            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
                let mut rp: png_bytep;
                let mut i: png_uint_32;

                i = 0;
                rp = row;
                while i < row_width {
                    let mut save: png_byte = *rp;
                    *rp = *(rp.add(4));
                    *(rp.add(4)) = save;
                    save = *(rp.add(1));
                    *(rp.add(1)) = *(rp.add(5));
                    *(rp.add(5)) = save;

                    i += 1;
                    rp = rp.add(6);
                }
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                let mut rp: png_bytep;
                let mut i: png_uint_32;

                i = 0;
                rp = row;
                while i < row_width {
                    let mut save: png_byte = *rp;
                    *rp = *(rp.add(4));
                    *(rp.add(4)) = save;
                    save = *(rp.add(1));
                    *(rp.add(1)) = *(rp.add(5));
                    *(rp.add(5)) = save;

                    i += 1;
                    rp = rp.add(8);
                }
            }
        }
    }
}

/* Added at libpng-1.5.10 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_check_palette_indexes(
    png_ptr: png_structrp,
    row_info: png_row_infop,
) {
    if ((*png_ptr).num_palette as c_int) < (1 << (*row_info).bit_depth)
        && (*png_ptr).num_palette > 0
    /* num_palette can be 0 in MNG files */
    {
        /* 'padding' is in *bits* within the last byte, it is an 'int'. */
        let mut padding: c_int =
            png_padbits((*row_info).pixel_depth as png_uint_32, (*row_info).width) as c_int;
        let mut rp: png_bytep = (*png_ptr).row_buf.add((*row_info).rowbytes);

        match (*row_info).bit_depth {
            1 => {
                /* in this case, all bytes must be 0 so we don't need
                 * to unpack the pixels except for the rightmost one.
                 */
                while rp > (*png_ptr).row_buf {
                    if ((*rp as c_int) >> padding) != 0 {
                        (*png_ptr).num_palette_max = 1;
                    }
                    padding = 0;
                    rp = rp.offset(-1);
                }
            }

            2 => {
                while rp > (*png_ptr).row_buf {
                    let mut i: c_int = ((*rp as c_int) >> padding) & 0x03;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    i = (((*rp as c_int) >> padding) >> 2) & 0x03;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    i = (((*rp as c_int) >> padding) >> 4) & 0x03;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    i = (((*rp as c_int) >> padding) >> 6) & 0x03;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    padding = 0;
                    rp = rp.offset(-1);
                }
            }

            4 => {
                while rp > (*png_ptr).row_buf {
                    let mut i: c_int = ((*rp as c_int) >> padding) & 0x0f;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    i = (((*rp as c_int) >> padding) >> 4) & 0x0f;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    padding = 0;
                    rp = rp.offset(-1);
                }
            }

            8 => {
                while rp > (*png_ptr).row_buf {
                    if (*rp as c_int) > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = *rp as c_int;
                    }
                    rp = rp.offset(-1);
                }
            }

            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_user_transform_info(
    png_ptr: png_structrp,
    user_transform_ptr: png_voidp,
    user_transform_depth: c_int,
    user_transform_channels: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 && ((*png_ptr).flags & PNG_FLAG_ROW_INIT) != 0 {
        png_app_error(
            png_ptr,
            c"info change after png_start_read_image or png_read_update_info".as_ptr(),
        );
        return;
    }

    (*png_ptr).user_transform_ptr = user_transform_ptr;
    (*png_ptr).user_transform_depth = user_transform_depth as png_byte;
    (*png_ptr).user_transform_channels = user_transform_channels as png_byte;
}

/* This function returns a pointer to the user_transform_ptr associated with
 * the user transform functions.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_transform_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return ptr::null_mut();
    }

    (*png_ptr).user_transform_ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_current_row_number(png_ptr: png_const_structrp) -> png_uint_32 {
    /* See the comments in png.h - this is the sub-image row when reading an
     * interlaced image.
     */
    if !png_ptr.is_null() {
        return (*png_ptr).row_number;
    }

    PNG_UINT_32_MAX /* help the app not to fail silently */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_current_pass_number(png_ptr: png_const_structrp) -> png_byte {
    if !png_ptr.is_null() {
        return (*png_ptr).pass;
    }
    8 /* invalid */
}

/// PNG_PADBITS(pixel_bits, width)
#[inline]
fn png_padbits(pixel_bits: png_uint_32, width: png_uint_32) -> png_uint_32 {
    let trailbits = pixel_bits.wrapping_mul(width % 8) % 8;
    (8u32.wrapping_sub(trailbits)) % 8
}
