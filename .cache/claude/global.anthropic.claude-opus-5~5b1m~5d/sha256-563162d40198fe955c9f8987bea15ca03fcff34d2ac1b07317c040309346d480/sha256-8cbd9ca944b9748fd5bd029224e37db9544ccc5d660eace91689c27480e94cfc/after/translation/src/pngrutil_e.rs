//! pngrutil.c lines 3954-4684: the row un-filtering functions, the sequential
//! IDAT reader and the row start/finish bookkeeping.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

pub unsafe extern "C-unwind" fn png_read_filter_row_sub(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    let mut i: usize;
    let istop: usize = (*row_info).rowbytes;
    let bpp: c_uint = (((*row_info).pixel_depth as c_int + 7) >> 3) as c_uint;
    let mut rp: png_bytep = row.add(bpp as usize);

    /* PNG_UNUSED(prev_row) */
    let _ = prev_row;

    i = bpp as usize;
    while i < istop {
        *rp = (((*rp as c_int) + (*rp.sub(bpp as usize) as c_int)) & 0xff) as png_byte;
        rp = rp.add(1);
        i += 1;
    }
}

pub unsafe extern "C-unwind" fn png_read_filter_row_up(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    let mut i: usize;
    let istop: usize = (*row_info).rowbytes;
    let mut rp: png_bytep = row;
    let mut pp: png_const_bytep = prev_row;

    i = 0;
    while i < istop {
        let v = *pp;
        pp = pp.add(1);
        *rp = (((*rp as c_int) + (v as c_int)) & 0xff) as png_byte;
        rp = rp.add(1);
        i += 1;
    }
}

pub unsafe extern "C-unwind" fn png_read_filter_row_avg(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    let mut i: usize;
    let mut rp: png_bytep = row;
    let mut pp: png_const_bytep = prev_row;
    let bpp: c_uint = (((*row_info).pixel_depth as c_int + 7) >> 3) as c_uint;
    let istop: usize = (*row_info).rowbytes.wrapping_sub(bpp as usize);

    i = 0;
    while i < bpp as usize {
        let v = *pp;
        pp = pp.add(1);
        *rp = (((*rp as c_int) + ((v as c_int) / 2)) & 0xff) as png_byte;

        rp = rp.add(1);
        i += 1;
    }

    i = 0;
    while i < istop {
        let v = *pp;
        pp = pp.add(1);
        *rp = (((*rp as c_int) + ((v as c_int) + (*rp.sub(bpp as usize) as c_int)) / 2) & 0xff)
            as png_byte;

        rp = rp.add(1);
        i += 1;
    }
}

pub unsafe extern "C-unwind" fn png_read_filter_row_paeth_1byte_pixel(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    let mut row: png_bytep = row;
    let mut prev_row: png_const_bytep = prev_row;
    let rp_end: png_bytep = row.add((*row_info).rowbytes);
    let mut a: c_int;
    let mut c: c_int;

    /* First pixel/byte */
    c = *prev_row as c_int;
    prev_row = prev_row.add(1);
    a = (*row as c_int) + c;
    *row = a as png_byte;
    row = row.add(1);

    /* Remainder */
    while row < rp_end {
        let b: c_int;
        let mut pa: c_int;
        let pb: c_int;
        let mut pc: c_int;
        let p: c_int;

        a &= 0xff; /* From previous iteration or start */
        b = *prev_row as c_int;
        prev_row = prev_row.add(1);

        p = b - c;
        pc = a - c;

        pa = if p < 0 { -p } else { p };
        pb = if pc < 0 { -pc } else { pc };
        pc = if (p + pc) < 0 { -(p + pc) } else { p + pc };

        /* Find the best predictor, the least of pa, pb, pc favoring the earlier
         * ones in the case of a tie.
         */
        if pb < pa {
            pa = pb;
            a = b;
        }
        if pc < pa {
            a = c;
        }

        /* Calculate the current pixel in a, and move the previous row pixel to c
         * for the next time round the loop
         */
        c = b;
        a += *row as c_int;
        *row = a as png_byte;
        row = row.add(1);
    }
}

pub unsafe extern "C-unwind" fn png_read_filter_row_paeth_multibyte_pixel(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    let mut row: png_bytep = row;
    let mut prev_row: png_const_bytep = prev_row;
    let bpp: c_uint = (((*row_info).pixel_depth as c_int + 7) >> 3) as c_uint;
    let mut rp_end: png_bytep = row.add(bpp as usize);

    /* Process the first pixel in the row completely (this is the same as 'up'
     * because there is only one candidate predictor for the first row).
     */
    while row < rp_end {
        let v = *prev_row;
        prev_row = prev_row.add(1);
        let a: c_int = (*row as c_int) + (v as c_int);
        *row = a as png_byte;
        row = row.add(1);
    }

    /* Remainder */
    rp_end = rp_end.add((*row_info).rowbytes.wrapping_sub(bpp as usize));

    while row < rp_end {
        let mut a: c_int;
        let b: c_int;
        let c: c_int;
        let mut pa: c_int;
        let pb: c_int;
        let mut pc: c_int;
        let p: c_int;

        c = *prev_row.sub(bpp as usize) as c_int;
        a = *row.sub(bpp as usize) as c_int;
        b = *prev_row as c_int;
        prev_row = prev_row.add(1);

        p = b - c;
        pc = a - c;

        pa = if p < 0 { -p } else { p };
        pb = if pc < 0 { -pc } else { pc };
        pc = if (p + pc) < 0 { -(p + pc) } else { p + pc };

        if pb < pa {
            pa = pb;
            a = b;
        }
        if pc < pa {
            a = c;
        }

        a += *row as c_int;
        *row = a as png_byte;
        row = row.add(1);
    }
}

/* This function is called once for every PNG image (except for PNG images
 * that only use PNG_FILTER_VALUE_NONE for all rows) to set the
 * implementations required to reverse the filtering of PNG rows.  Reversing
 * the filter is the first transformation performed on the row data.  It is
 * performed in place, therefore an implementation can be selected based on
 * the image pixel format.  If the implementation depends on image width then
 * take care to ensure that it works correctly if the image is interlaced -
 * interlacing causes the actual row width to vary.
 */
pub unsafe fn png_init_filter_functions(pp: png_structrp) {
    let bpp: c_uint = (((*pp).pixel_depth as c_int + 7) >> 3) as c_uint;

    (*pp).read_filter[(PNG_FILTER_VALUE_SUB - 1) as usize] = Some(png_read_filter_row_sub);
    (*pp).read_filter[(PNG_FILTER_VALUE_UP - 1) as usize] = Some(png_read_filter_row_up);
    (*pp).read_filter[(PNG_FILTER_VALUE_AVG - 1) as usize] = Some(png_read_filter_row_avg);
    if bpp == 1 {
        (*pp).read_filter[(PNG_FILTER_VALUE_PAETH - 1) as usize] =
            Some(png_read_filter_row_paeth_1byte_pixel);
    } else {
        (*pp).read_filter[(PNG_FILTER_VALUE_PAETH - 1) as usize] =
            Some(png_read_filter_row_paeth_multibyte_pixel);
    }

    /* PNG_FILTER_OPTIMIZATIONS is not defined in this build. */
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_filter_row(
    pp: png_structrp,
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
    filter: c_int,
) {
    /* OPTIMIZATION: DO NOT MODIFY THIS FUNCTION, instead #define
     * PNG_FILTER_OPTIMIZATIONS to a function that overrides the generic
     * implementations.  See png_init_filter_functions above.
     */
    if filter > PNG_FILTER_VALUE_NONE && filter < PNG_FILTER_VALUE_LAST {
        if (*pp).read_filter[0].is_none() {
            png_init_filter_functions(pp);
        }

        ((*pp).read_filter[(filter - 1) as usize].unwrap())(row_info, row, prev_row);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_IDAT_data(
    png_ptr: png_structrp,
    output: png_bytep,
    avail_out: png_alloc_size_t,
) {
    let mut avail_out: png_alloc_size_t = avail_out;

    /* Loop reading IDATs and decompressing the result into output[avail_out] */
    (*png_ptr).zstream.next_out = output;
    (*png_ptr).zstream.avail_out = 0; /* safety: set below */

    if output.is_null() {
        avail_out = 0;
    }

    loop {
        let ret: c_int;
        let mut tmpbuf: [png_byte; PNG_INFLATE_BUF_SIZE] = [0; PNG_INFLATE_BUF_SIZE];

        if (*png_ptr).zstream.avail_in == 0 {
            let mut avail_in: uInt;
            let buffer: png_bytep;

            while (*png_ptr).idat_size == 0 {
                png_crc_finish(png_ptr, 0);

                (*png_ptr).idat_size = png_read_chunk_header(png_ptr);
                /* This is an error even in the 'check' case because the code just
                 * consumed a non-IDAT header.
                 */
                if (*png_ptr).chunk_name != png_IDAT {
                    png_error(png_ptr, c"Not enough image data".as_ptr());
                }
            }

            avail_in = (*png_ptr).IDAT_read_size;

            if (avail_in as png_alloc_size_t) > (*png_ptr).user_chunk_malloc_max {
                avail_in = /*SAFE*/ (*png_ptr).user_chunk_malloc_max as uInt;
            }

            if avail_in > (*png_ptr).idat_size {
                avail_in = (*png_ptr).idat_size as uInt;
            }

            /* A PNG with a gradually increasing IDAT size will defeat this attempt
             * to minimize memory usage by causing lots of re-allocs, but
             * realistically doing IDAT_read_size re-allocs is not likely to be a
             * big problem.
             *
             * An error here corresponds to the system being out of memory.
             */
            buffer = png_read_buffer(png_ptr, avail_in as png_alloc_size_t);

            if buffer.is_null() {
                png_chunk_error(png_ptr, c"out of memory".as_ptr());
            }

            png_crc_read(png_ptr, buffer, avail_in);
            (*png_ptr).idat_size -= avail_in;

            (*png_ptr).zstream.next_in = buffer;
            (*png_ptr).zstream.avail_in = avail_in;
        }

        /* And set up the output side. */
        if !output.is_null()
        /* standard read */
        {
            let mut out: uInt = crate::zlib::ZLIB_IO_MAX;

            if (out as png_alloc_size_t) > avail_out {
                out = avail_out as uInt;
            }

            avail_out -= out as png_alloc_size_t;
            (*png_ptr).zstream.avail_out = out;
        } else
        /* after last row, checking for end */
        {
            (*png_ptr).zstream.next_out = tmpbuf.as_mut_ptr();
            (*png_ptr).zstream.avail_out = PNG_INFLATE_BUF_SIZE as uInt;
        }

        /* Use NO_FLUSH; this gives zlib the maximum opportunity to optimize the
         * process.  If the LZ stream is truncated the sequential reader will
         * terminally damage the stream, above, by reading the chunk header of the
         * following chunk (it then exits with png_error).
         *
         * TODO: deal more elegantly with truncated IDAT lists.
         */
        ret = png_zlib_inflate(png_ptr, Z_NO_FLUSH);

        /* Take the unconsumed output back. */
        if !output.is_null() {
            avail_out += (*png_ptr).zstream.avail_out as png_alloc_size_t;
        } else
        /* avail_out counts the extra bytes */
        {
            avail_out +=
                PNG_INFLATE_BUF_SIZE - ((*png_ptr).zstream.avail_out as png_alloc_size_t);
        }

        (*png_ptr).zstream.avail_out = 0;

        if ret == Z_STREAM_END {
            /* Do this for safety; we won't read any more into this row. */
            (*png_ptr).zstream.next_out = core::ptr::null_mut();

            (*png_ptr).mode |= PNG_AFTER_IDAT;
            (*png_ptr).flags |= PNG_FLAG_ZSTREAM_ENDED;

            if (*png_ptr).zstream.avail_in > 0 || (*png_ptr).idat_size > 0 {
                png_chunk_benign_error(png_ptr, c"Extra compressed data".as_ptr());
            }
            break;
        }

        if ret != Z_OK {
            png_zstream_error(png_ptr, ret);

            if !output.is_null() {
                png_chunk_error(png_ptr, (*png_ptr).zstream.msg);
            } else
            /* checking */
            {
                png_chunk_benign_error(png_ptr, (*png_ptr).zstream.msg);
                return;
            }
        }

        if !(avail_out > 0) {
            break;
        }
    }

    if avail_out > 0 {
        /* The stream ended before the image; this is the same as too few IDATs so
         * should be handled the same way.
         */
        if !output.is_null() {
            png_error(png_ptr, c"Not enough image data".as_ptr());
        } else
        /* the deflate stream contained extra data */
        {
            png_chunk_benign_error(png_ptr, c"Too much image data".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_finish_IDAT(png_ptr: png_structrp) {
    /* We don't need any more data and the stream should have ended, however the
     * LZ end code may actually not have been processed.  In this case we must
     * read it otherwise stray unread IDAT data or, more likely, an IDAT chunk
     * may still remain to be consumed.
     */
    if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0 {
        /* The NULL causes png_read_IDAT_data to swallow any remaining bytes in
         * the compressed stream, but the stream may be damaged too, so even after
         * this call we may need to terminate the zstream ownership.
         */
        png_read_IDAT_data(png_ptr, core::ptr::null_mut(), 0);
        (*png_ptr).zstream.next_out = core::ptr::null_mut(); /* safety */

        /* Now clear everything out for safety; the following may not have been
         * done.
         */
        if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0 {
            (*png_ptr).mode |= PNG_AFTER_IDAT;
            (*png_ptr).flags |= PNG_FLAG_ZSTREAM_ENDED;
        }
    }

    /* If the zstream has not been released do it now *and* terminate the reading
     * of the final IDAT chunk.
     */
    if (*png_ptr).zowner == png_IDAT {
        /* Always do this; the pointers otherwise point into the read buffer. */
        (*png_ptr).zstream.next_in = core::ptr::null();
        (*png_ptr).zstream.avail_in = 0;

        /* Now we no longer own the zstream. */
        (*png_ptr).zowner = 0;

        /* The slightly weird semantics of the sequential IDAT reading is that we
         * are always in or at the end of an IDAT chunk, so we always need to do a
         * crc_finish here.  If idat_size is non-zero we also need to read the
         * spurious bytes at the end of the chunk now.
         */
        png_crc_finish(png_ptr, (*png_ptr).idat_size);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_finish_row(png_ptr: png_structrp) {
    (*png_ptr).row_number += 1;
    if (*png_ptr).row_number < (*png_ptr).num_rows {
        return;
    }

    if (*png_ptr).interlaced != 0 {
        (*png_ptr).row_number = 0;

        /* TO DO: don't do this if prev_row isn't needed (requires
         * read-ahead of the next row's filter byte.
         */
        memset(
            (*png_ptr).prev_row as *mut u8,
            0,
            (*png_ptr).rowbytes.wrapping_add(1),
        );

        loop {
            (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);

            if (*png_ptr).pass >= 7 {
                break;
            }

            (*png_ptr).iwidth = ((*png_ptr)
                .width
                .wrapping_add(png_pass_inc[(*png_ptr).pass as usize] as png_uint_32)
                .wrapping_sub(1)
                .wrapping_sub(png_pass_start[(*png_ptr).pass as usize] as png_uint_32))
                / (png_pass_inc[(*png_ptr).pass as usize] as png_uint_32);

            if ((*png_ptr).transformations & PNG_INTERLACE) == 0 {
                (*png_ptr).num_rows = ((*png_ptr)
                    .height
                    .wrapping_add(png_pass_yinc[(*png_ptr).pass as usize] as png_uint_32)
                    .wrapping_sub(1)
                    .wrapping_sub(png_pass_ystart[(*png_ptr).pass as usize] as png_uint_32))
                    / (png_pass_yinc[(*png_ptr).pass as usize] as png_uint_32);
            } else
            /* if (png_ptr->transformations & PNG_INTERLACE) */
            {
                break; /* libpng deinterlacing sees every row */
            }

            if !((*png_ptr).num_rows == 0 || (*png_ptr).iwidth == 0) {
                break;
            }
        }

        if (*png_ptr).pass < 7 {
            return;
        }
    }

    /* Here after at the end of the last row of the last pass. */
    png_read_finish_IDAT(png_ptr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_start_row(png_ptr: png_structrp) {
    let mut max_pixel_depth: c_uint;
    let mut row_bytes: usize;

    png_init_read_transformations(png_ptr);

    if (*png_ptr).interlaced != 0 {
        if ((*png_ptr).transformations & PNG_INTERLACE) == 0 {
            (*png_ptr).num_rows = ((*png_ptr)
                .height
                .wrapping_add(png_pass_yinc[0] as png_uint_32)
                .wrapping_sub(1)
                .wrapping_sub(png_pass_ystart[0] as png_uint_32))
                / (png_pass_yinc[0] as png_uint_32);
        } else {
            (*png_ptr).num_rows = (*png_ptr).height;
        }

        (*png_ptr).iwidth = ((*png_ptr)
            .width
            .wrapping_add(png_pass_inc[(*png_ptr).pass as usize] as png_uint_32)
            .wrapping_sub(1)
            .wrapping_sub(png_pass_start[(*png_ptr).pass as usize] as png_uint_32))
            / (png_pass_inc[(*png_ptr).pass as usize] as png_uint_32);
    } else {
        (*png_ptr).num_rows = (*png_ptr).height;
        (*png_ptr).iwidth = (*png_ptr).width;
    }

    max_pixel_depth = (*png_ptr).pixel_depth as c_uint;

    /* WARNING: * png_read_transform_info (pngrtran.c) performs a simpler set of
     * calculations to calculate the final pixel depth, then
     * png_do_read_transforms actually does the transforms.  This means that the
     * code which effectively calculates this value is actually repeated in three
     * separate places.  They must all match.  Innocent changes to the order of
     * transformations can and will break libpng in a way that causes memory
     * overwrites.
     *
     * TODO: fix this.
     */
    if ((*png_ptr).transformations & PNG_PACK) != 0 && (*png_ptr).bit_depth < 8 {
        max_pixel_depth = 8;
    }

    if ((*png_ptr).transformations & PNG_EXPAND) != 0 {
        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            if (*png_ptr).num_trans != 0 {
                max_pixel_depth = 32;
            } else {
                max_pixel_depth = 24;
            }
        } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY {
            if max_pixel_depth < 8 {
                max_pixel_depth = 8;
            }

            if (*png_ptr).num_trans != 0 {
                max_pixel_depth *= 2;
            }
        } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB {
            if (*png_ptr).num_trans != 0 {
                max_pixel_depth *= 4;
                max_pixel_depth /= 3;
            }
        }
    }

    if ((*png_ptr).transformations & PNG_EXPAND_16) != 0 {
        /* In fact it is an error if it isn't supported, but checking is
         * the safe way.
         */
        if ((*png_ptr).transformations & PNG_EXPAND) != 0 {
            if (*png_ptr).bit_depth < 16 {
                max_pixel_depth *= 2;
            }
        } else {
            (*png_ptr).transformations &= !PNG_EXPAND_16;
        }
    }

    if ((*png_ptr).transformations & (PNG_FILLER)) != 0 {
        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY {
            if max_pixel_depth <= 8 {
                max_pixel_depth = 16;
            } else {
                max_pixel_depth = 32;
            }
        } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
            || (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        {
            if max_pixel_depth <= 32 {
                max_pixel_depth = 32;
            } else {
                max_pixel_depth = 64;
            }
        }
    }

    if ((*png_ptr).transformations & PNG_GRAY_TO_RGB) != 0 {
        if ((*png_ptr).num_trans != 0 && ((*png_ptr).transformations & PNG_EXPAND) != 0)
            || ((*png_ptr).transformations & (PNG_FILLER)) != 0
            || (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA
        {
            if max_pixel_depth <= 16 {
                max_pixel_depth = 32;
            } else {
                max_pixel_depth = 64;
            }
        } else {
            if max_pixel_depth <= 8 {
                if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                    max_pixel_depth = 32;
                } else {
                    max_pixel_depth = 24;
                }
            } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                max_pixel_depth = 64;
            } else {
                max_pixel_depth = 48;
            }
        }
    }

    if ((*png_ptr).transformations & PNG_USER_TRANSFORM) != 0 {
        let user_pixel_depth: c_uint =
            ((*png_ptr).user_transform_depth as c_int * (*png_ptr).user_transform_channels as c_int)
                as c_uint;

        if user_pixel_depth > max_pixel_depth {
            max_pixel_depth = user_pixel_depth;
        }
    }

    /* This value is stored in png_struct and double checked in the row read
     * code.
     */
    (*png_ptr).maximum_pixel_depth = max_pixel_depth as png_byte;
    (*png_ptr).transformed_pixel_depth = 0; /* calculated on demand */

    /* Align the width on the next larger 8 pixels.  Mainly used
     * for interlacing
     */
    row_bytes = ((*png_ptr).width.wrapping_add(7) & !(7 as png_uint_32)) as usize;
    /* Calculate the maximum bytes needed, adding a byte and a pixel
     * for safety's sake
     */
    row_bytes = PNG_ROWBYTES(max_pixel_depth, row_bytes as png_uint_32)
        .wrapping_add(1)
        .wrapping_add(((max_pixel_depth + 7) >> 3u32) as usize);

    if row_bytes.wrapping_add(48) > (*png_ptr).old_big_row_buf_size {
        png_free(png_ptr, (*png_ptr).big_row_buf as png_voidp);
        png_free(png_ptr, (*png_ptr).big_prev_row as png_voidp);
        (*png_ptr).big_prev_row = core::ptr::null_mut();
        (*png_ptr).big_row_buf = core::ptr::null_mut();

        if (*png_ptr).interlaced != 0 {
            (*png_ptr).big_row_buf =
                png_calloc(png_ptr, row_bytes.wrapping_add(48)) as png_bytep;
        } else {
            (*png_ptr).big_row_buf =
                png_malloc(png_ptr, row_bytes.wrapping_add(48)) as png_bytep;
        }

        (*png_ptr).big_prev_row = png_malloc(png_ptr, row_bytes.wrapping_add(48)) as png_bytep;

        /* Use 16-byte aligned memory for row_buf with at least 16 bytes
         * of padding before and after row_buf; treat prev_row similarly.
         * NOTE: the alignment is to the start of the pixels, one beyond the start
         * of the buffer, because of the filter byte.  Prior to libpng 1.5.6 this
         * was incorrect; the filter byte was aligned, which had the exact
         * opposite effect of that intended.
         */
        {
            let mut temp: png_bytep = (*png_ptr).big_row_buf.add(32);
            let mut extra: usize = (temp as usize) & 0x0f;
            (*png_ptr).row_buf = temp.sub(extra).sub(1 /*filter byte*/);

            temp = (*png_ptr).big_prev_row.add(32);
            extra = (temp as usize) & 0x0f;
            (*png_ptr).prev_row = temp.sub(extra).sub(1 /*filter byte*/);
        }

        (*png_ptr).old_big_row_buf_size = row_bytes.wrapping_add(48);
    }

    if (*png_ptr).rowbytes > (PNG_SIZE_MAX - 1) {
        png_error(
            png_ptr,
            c"Row has too many bytes to allocate in memory".as_ptr(),
        );
    }

    memset(
        (*png_ptr).prev_row as *mut u8,
        0,
        (*png_ptr).rowbytes.wrapping_add(1),
    );

    /* The sequential reader needs a buffer for IDAT, but the progressive reader
     * does not, so free the read buffer now regardless; the sequential reader
     * reallocates it on demand.
     */
    if !(*png_ptr).read_buffer.is_null() {
        let buffer: png_bytep = (*png_ptr).read_buffer;

        (*png_ptr).read_buffer_size = 0;
        (*png_ptr).read_buffer = core::ptr::null_mut();
        png_free(png_ptr, buffer as png_voidp);
    }

    /* Finally claim the zstream for the inflate of the IDAT data, use the bits
     * value from the stream (note that this will result in a fatal error if the
     * IDAT stream has a bogus deflate header window_bits value, but this should
     * not be happening any longer!)
     */
    if png_inflate_claim(png_ptr, png_IDAT) != Z_OK {
        png_error(png_ptr, (*png_ptr).zstream.msg);
    }

    (*png_ptr).flags |= PNG_FLAG_ROW_INIT;
}
