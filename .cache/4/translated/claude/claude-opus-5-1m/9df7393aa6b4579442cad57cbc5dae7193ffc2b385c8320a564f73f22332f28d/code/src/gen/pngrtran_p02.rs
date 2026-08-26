/* pngrtran.c lines 489..892 */

/* Dither file to 8-bit.  Supply a palette, the current number
 * of elements in the palette, the maximum number of elements
 * allowed, and a histogram if possible.  If the current number
 * of colors is greater than the maximum number, the palette will be
 * modified to fit in the maximum number.  "full_quantize" indicates
 * whether we need a quantizing cube set up for RGB images, or if we
 * simply are reducing the number of colors in a paletted image.
 */
/* png_set_quantize */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_quantize(
    png_ptr: png_structrp,
    palette: png_colorp,
    mut num_palette: c_int,
    maximum_colors: c_int,
    histogram: png_const_uint_16p,
    full_quantize: c_int,
) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    if palette == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).transformations |= PNG_QUANTIZE;

    if full_quantize == 0 {
        let mut i: c_int;

        /* Initialize the array to index colors.
         *
         * Ensure quantize_index can fit 256 elements (PNG_MAX_PALETTE_LENGTH)
         * rather than num_palette elements. This is to prevent buffer overflows
         * caused by malformed PNG files with out-of-range palette indices.
         *
         * Be careful to avoid leaking memory. Applications are allowed to call
         * this function more than once per png_struct.
         */
        png_free(png_ptr, (*png_ptr).quantize_index as png_voidp);
        (*png_ptr).quantize_index = core::ptr::null_mut();
        (*png_ptr).quantize_index =
            png_malloc(png_ptr, PNG_MAX_PALETTE_LENGTH as png_alloc_size_t) as png_bytep;
        i = 0;
        while i < PNG_MAX_PALETTE_LENGTH {
            *(*png_ptr).quantize_index.offset(i as isize) = i as png_byte;
            i += 1;
        }
    }

    if num_palette > maximum_colors {
        if histogram != core::ptr::null() {
            /* This is easy enough, just throw out the least used colors.
             * Perhaps not the best solution, but good enough.
             */

            let quantize_sort: png_bytep;
            let mut i: c_int;
            let mut j: c_int;

            /* Initialize the local array to sort colors. */
            quantize_sort = png_malloc(png_ptr, num_palette as png_alloc_size_t) as png_bytep;
            i = 0;
            while i < num_palette {
                *quantize_sort.offset(i as isize) = i as png_byte;
                i += 1;
            }

            /* Find the least used palette entries by starting a
             * bubble sort, and running it until we have sorted
             * out enough colors.  Note that we don't care about
             * sorting all the colors, just finding which are
             * least used.
             */

            i = num_palette - 1;
            while i >= maximum_colors {
                let mut done: c_int; /* To stop early if the list is pre-sorted */

                done = 1;
                j = 0;
                while j < i {
                    if *histogram.offset(*quantize_sort.offset(j as isize) as isize)
                        < *histogram.offset(*quantize_sort.offset((j + 1) as isize) as isize)
                    {
                        let t: png_byte;

                        t = *quantize_sort.offset(j as isize);
                        *quantize_sort.offset(j as isize) =
                            *quantize_sort.offset((j + 1) as isize);
                        *quantize_sort.offset((j + 1) as isize) = t;
                        done = 0;
                    }
                    j += 1;
                }

                if done != 0 {
                    break;
                }
                i -= 1;
            }

            /* Swap the palette around, and set up a table, if necessary */
            if full_quantize != 0 {
                j = num_palette;

                /* Put all the useful colors within the max, but don't
                 * move the others.
                 */
                i = 0;
                while i < maximum_colors {
                    if (*quantize_sort.offset(i as isize) as c_int) >= maximum_colors {
                        loop {
                            j -= 1;
                            if !((*quantize_sort.offset(j as isize) as c_int) >= maximum_colors) {
                                break;
                            }
                        }

                        *palette.offset(i as isize) = *palette.offset(j as isize);
                    }
                    i += 1;
                }
            } else {
                j = num_palette;

                /* Move all the used colors inside the max limit, and
                 * develop a translation table.
                 */
                i = 0;
                while i < maximum_colors {
                    /* Only move the colors we need to */
                    if (*quantize_sort.offset(i as isize) as c_int) >= maximum_colors {
                        let tmp_color: png_color;

                        loop {
                            j -= 1;
                            if !((*quantize_sort.offset(j as isize) as c_int) >= maximum_colors) {
                                break;
                            }
                        }

                        tmp_color = *palette.offset(j as isize);
                        *palette.offset(j as isize) = *palette.offset(i as isize);
                        *palette.offset(i as isize) = tmp_color;
                        /* Indicate where the color went */
                        *(*png_ptr).quantize_index.offset(j as isize) = i as png_byte;
                        *(*png_ptr).quantize_index.offset(i as isize) = j as png_byte;
                    }
                    i += 1;
                }

                /* Find closest color for those colors we are not using */
                i = 0;
                while i < num_palette {
                    if (*(*png_ptr).quantize_index.offset(i as isize) as c_int) >= maximum_colors {
                        let mut min_d: c_int;
                        let mut k: c_int;
                        let mut min_k: c_int;
                        let d_index: c_int;

                        /* Find the closest color to one we threw out */
                        d_index = *(*png_ptr).quantize_index.offset(i as isize) as c_int;
                        min_d = PNG_COLOR_DIST(
                            *palette.offset(d_index as isize),
                            *palette.offset(0),
                        );
                        k = 1;
                        min_k = 0;
                        while k < maximum_colors {
                            let d: c_int;

                            d = PNG_COLOR_DIST(
                                *palette.offset(d_index as isize),
                                *palette.offset(k as isize),
                            );

                            if d < min_d {
                                min_d = d;
                                min_k = k;
                            }
                            k += 1;
                        }
                        /* Point to closest color */
                        *(*png_ptr).quantize_index.offset(i as isize) = min_k as png_byte;
                    }
                    i += 1;
                }
            }
            png_free(png_ptr, quantize_sort as png_voidp);
        } else {
            /* This is much harder to do simply (and quickly).  Perhaps
             * we need to go through a median cut routine, but those
             * don't always behave themselves with only a few colors
             * as input.  So we will just find the closest two colors,
             * and throw out one of them (chosen somewhat randomly).
             * [We don't understand this at all, so if someone wants to
             *  work on improving it, be our guest - AED, GRP]
             */
            let mut i: c_int;
            let mut max_d: c_int;
            let mut num_new_palette: c_int;
            let mut t: png_dsortp;
            let hash: png_dsortpp;

            t = core::ptr::null_mut();

            /* Initialize palette index arrays */
            (*png_ptr).index_to_palette =
                png_malloc(png_ptr, num_palette as png_alloc_size_t) as png_bytep;
            (*png_ptr).palette_to_index =
                png_malloc(png_ptr, num_palette as png_alloc_size_t) as png_bytep;

            /* Initialize the sort array */
            i = 0;
            while i < num_palette {
                *(*png_ptr).index_to_palette.offset(i as isize) = i as png_byte;
                *(*png_ptr).palette_to_index.offset(i as isize) = i as png_byte;
                i += 1;
            }

            hash = png_calloc(
                png_ptr,
                (769 * core::mem::size_of::<png_dsortp>()) as png_alloc_size_t,
            ) as png_dsortpp;

            num_new_palette = num_palette;

            /* Initial wild guess at how far apart the farthest pixel
             * pair we will be eliminating will be.  Larger
             * numbers mean more areas will be allocated, Smaller
             * numbers run the risk of not saving enough data, and
             * having to do this all over again.
             *
             * I have not done extensive checking on this number.
             */
            max_d = 96;

            while num_new_palette > maximum_colors {
                i = 0;
                while i < num_new_palette - 1 {
                    let mut j: c_int;

                    j = i + 1;
                    while j < num_new_palette {
                        let d: c_int;

                        d = PNG_COLOR_DIST(
                            *palette.offset(i as isize),
                            *palette.offset(j as isize),
                        );

                        if d <= max_d {
                            t = png_malloc_warn(
                                png_ptr,
                                core::mem::size_of::<png_dsort>() as png_alloc_size_t,
                            ) as png_dsortp;

                            if t == core::ptr::null_mut() {
                                break;
                            }

                            (*t).next = *hash.offset(d as isize);
                            (*t).left = *(*png_ptr).palette_to_index.offset(i as isize);
                            (*t).right = *(*png_ptr).palette_to_index.offset(j as isize);
                            *hash.offset(d as isize) = t;
                        }
                        j += 1;
                    }
                    if t == core::ptr::null_mut() {
                        break;
                    }
                    i += 1;
                }

                if t != core::ptr::null_mut() {
                    i = 0;
                    while i <= max_d {
                        if *hash.offset(i as isize) != core::ptr::null_mut() {
                            let mut p: png_dsortp;

                            p = *hash.offset(i as isize);
                            while !p.is_null() {
                                if (*(*png_ptr).index_to_palette.offset((*p).left as isize)
                                    as c_int)
                                    < num_new_palette
                                    && (*(*png_ptr).index_to_palette.offset((*p).right as isize)
                                        as c_int)
                                        < num_new_palette
                                {
                                    let j: c_int;
                                    let next_j: c_int;

                                    if (num_new_palette & 0x01) != 0 {
                                        j = (*p).left as c_int;
                                        next_j = (*p).right as c_int;
                                    } else {
                                        j = (*p).right as c_int;
                                        next_j = (*p).left as c_int;
                                    }

                                    num_new_palette -= 1;
                                    *palette.offset(
                                        *(*png_ptr).index_to_palette.offset(j as isize) as isize,
                                    ) = *palette.offset(num_new_palette as isize);
                                    if full_quantize == 0 {
                                        let mut k: c_int;

                                        k = 0;
                                        while k < num_palette {
                                            if *(*png_ptr).quantize_index.offset(k as isize)
                                                == *(*png_ptr)
                                                    .index_to_palette
                                                    .offset(j as isize)
                                            {
                                                *(*png_ptr).quantize_index.offset(k as isize) =
                                                    *(*png_ptr)
                                                        .index_to_palette
                                                        .offset(next_j as isize);
                                            }

                                            if (*(*png_ptr).quantize_index.offset(k as isize)
                                                as c_int)
                                                == num_new_palette
                                            {
                                                *(*png_ptr).quantize_index.offset(k as isize) =
                                                    *(*png_ptr)
                                                        .index_to_palette
                                                        .offset(j as isize);
                                            }
                                            k += 1;
                                        }
                                    }

                                    *(*png_ptr).index_to_palette.offset(
                                        *(*png_ptr)
                                            .palette_to_index
                                            .offset(num_new_palette as isize)
                                            as isize,
                                    ) = *(*png_ptr).index_to_palette.offset(j as isize);

                                    *(*png_ptr).palette_to_index.offset(
                                        *(*png_ptr).index_to_palette.offset(j as isize) as isize,
                                    ) = *(*png_ptr)
                                        .palette_to_index
                                        .offset(num_new_palette as isize);

                                    *(*png_ptr).index_to_palette.offset(j as isize) =
                                        num_new_palette as png_byte;

                                    *(*png_ptr)
                                        .palette_to_index
                                        .offset(num_new_palette as isize) = j as png_byte;
                                }
                                if num_new_palette <= maximum_colors {
                                    break;
                                }
                                p = (*p).next;
                            }
                            if num_new_palette <= maximum_colors {
                                break;
                            }
                        }
                        i += 1;
                    }
                }

                i = 0;
                while i < 769 {
                    if *hash.offset(i as isize) != core::ptr::null_mut() {
                        let mut p: png_dsortp = *hash.offset(i as isize);
                        while !p.is_null() {
                            t = (*p).next;
                            png_free(png_ptr, p as png_voidp);
                            p = t;
                        }
                    }
                    *hash.offset(i as isize) = core::ptr::null_mut();
                    i += 1;
                }
                max_d += 96;
            }
            png_free(png_ptr, hash as png_voidp);
            png_free(png_ptr, (*png_ptr).palette_to_index as png_voidp);
            png_free(png_ptr, (*png_ptr).index_to_palette as png_voidp);
            (*png_ptr).palette_to_index = core::ptr::null_mut();
            (*png_ptr).index_to_palette = core::ptr::null_mut();
        }
        num_palette = maximum_colors;
    }
    if (*png_ptr).palette == core::ptr::null_mut() {
        /* Allocate an owned copy rather than aliasing the caller's pointer,
         * so that png_read_destroy can free png_ptr->palette unconditionally.
         */
        (*png_ptr).palette = png_calloc(
            png_ptr,
            (PNG_MAX_PALETTE_LENGTH as usize) * core::mem::size_of::<png_color>(),
        ) as png_colorp;
        memcpy(
            (*png_ptr).palette as *mut c_void,
            palette as *const c_void,
            (num_palette as c_uint as usize) * core::mem::size_of::<png_color>(),
        );
    }
    (*png_ptr).num_palette = num_palette as png_uint_16;

    if full_quantize != 0 {
        let mut i: c_int;
        let distance: png_bytep;
        let total_bits: c_int =
            PNG_QUANTIZE_RED_BITS + PNG_QUANTIZE_GREEN_BITS + PNG_QUANTIZE_BLUE_BITS;
        let num_red: c_int = 1 << PNG_QUANTIZE_RED_BITS;
        let num_green: c_int = 1 << PNG_QUANTIZE_GREEN_BITS;
        let num_blue: c_int = 1 << PNG_QUANTIZE_BLUE_BITS;
        let num_entries: usize = 1usize << total_bits;

        (*png_ptr).palette_lookup =
            png_calloc(png_ptr, num_entries as png_alloc_size_t) as png_bytep;

        distance = png_malloc(png_ptr, num_entries as png_alloc_size_t) as png_bytep;

        memset(distance as *mut c_void, 0xff, num_entries);

        i = 0;
        while i < num_palette {
            let mut ir: c_int;
            let mut ig: c_int;
            let mut ib: c_int;
            let r: c_int = ((*palette.offset(i as isize)).red as c_int)
                >> (8 - PNG_QUANTIZE_RED_BITS);
            let g: c_int = ((*palette.offset(i as isize)).green as c_int)
                >> (8 - PNG_QUANTIZE_GREEN_BITS);
            let b: c_int = ((*palette.offset(i as isize)).blue as c_int)
                >> (8 - PNG_QUANTIZE_BLUE_BITS);

            ir = 0;
            while ir < num_red {
                /* int dr = abs(ir - r); */
                let dr: c_int = if ir > r { ir - r } else { r - ir };
                let index_r: c_int = ir << (PNG_QUANTIZE_BLUE_BITS + PNG_QUANTIZE_GREEN_BITS);

                ig = 0;
                while ig < num_green {
                    /* int dg = abs(ig - g); */
                    let dg: c_int = if ig > g { ig - g } else { g - ig };
                    let dt: c_int = dr + dg;
                    let dm: c_int = if dr > dg { dr } else { dg };
                    let index_g: c_int = index_r | (ig << PNG_QUANTIZE_BLUE_BITS);

                    ib = 0;
                    while ib < num_blue {
                        let d_index: c_int = index_g | ib;
                        /* int db = abs(ib - b); */
                        let db: c_int = if ib > b { ib - b } else { b - ib };
                        let dmax: c_int = if dm > db { dm } else { db };
                        let d: c_int = dmax + dt + db;

                        if d < (*distance.offset(d_index as isize) as c_int) {
                            *distance.offset(d_index as isize) = d as png_byte;
                            *(*png_ptr).palette_lookup.offset(d_index as isize) = i as png_byte;
                        }
                        ib += 1;
                    }
                    ig += 1;
                }
                ir += 1;
            }
            i += 1;
        }

        png_free(png_ptr, distance as png_voidp);
    }
}
