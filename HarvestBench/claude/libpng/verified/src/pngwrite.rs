//! Translation of pngwrite.c - general routines to write a PNG file.
//!
//! Only the branches active under the full-feature pnglibconf.h are translated
//! (WRITE_* on, SIMPLIFIED_WRITE on, USER_MEM on, BENIGN_WRITE_ERRORS off,
//! RELEASE_BUILD off).
use crate::prelude::*;

// ---- Simplified-write only C library FFI not present in the shared cffi ----
extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    fn fclose(stream: *mut FILE) -> c_int;
    fn ferror(stream: *mut FILE) -> c_int;
    fn remove(path: *const c_char) -> c_int;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn __errno_location() -> *mut c_int;
}

// ---- png_image format / flag constants (png.h) ----
const PNG_FORMAT_FLAG_ALPHA: png_uint_32 = 0x01;
const PNG_FORMAT_FLAG_COLOR: png_uint_32 = 0x02;
const PNG_FORMAT_FLAG_LINEAR: png_uint_32 = 0x04;
const PNG_FORMAT_FLAG_COLORMAP: png_uint_32 = 0x08;
const PNG_FORMAT_FLAG_BGR: png_uint_32 = 0x10;
const PNG_FORMAT_FLAG_AFIRST: png_uint_32 = 0x20;

const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: png_uint_32 = 0x01;
const PNG_IMAGE_FLAG_FAST: png_uint_32 = 0x02;

const PNG_GAMMA_LINEAR: png_fixed_point = PNG_FP_1;

const PNG_FILLER_BEFORE: c_int = 0;
const PNG_FILLER_AFTER: c_int = 1;

/// PNG_IMAGE_SAMPLE_CHANNELS(fmt)
#[inline]
fn png_image_sample_channels(fmt: png_uint_32) -> c_uint {
    ((fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1) as c_uint
}

/// PNG_IMAGE_PIXEL_CHANNELS(fmt)
#[inline]
fn png_image_pixel_channels(fmt: png_uint_32) -> c_uint {
    if (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        1
    } else {
        png_image_sample_channels(fmt)
    }
}

// ---- sRGB conversion tables (from png.c, internal data) ----
static PNG_sRGB_BASE: [png_uint_16; 512] = [
    128, 1782, 3383, 4644, 5675, 6564, 7357, 8074, 8732, 9346, 9921, 10463, 10977, 11466, 11935, 12384,
    12816, 13233, 13634, 14024, 14402, 14769, 15125, 15473, 15812, 16142, 16466, 16781, 17090, 17393, 17690, 17981,
    18266, 18546, 18822, 19093, 19359, 19621, 19879, 20133, 20383, 20630, 20873, 21113, 21349, 21583, 21813, 22041,
    22265, 22487, 22707, 22923, 23138, 23350, 23559, 23767, 23972, 24175, 24376, 24575, 24772, 24967, 25160, 25352,
    25542, 25730, 25916, 26101, 26284, 26465, 26645, 26823, 27000, 27176, 27350, 27523, 27695, 27865, 28034, 28201,
    28368, 28533, 28697, 28860, 29021, 29182, 29341, 29500, 29657, 29813, 29969, 30123, 30276, 30429, 30580, 30730,
    30880, 31028, 31176, 31323, 31469, 31614, 31758, 31902, 32045, 32186, 32327, 32468, 32607, 32746, 32884, 33021,
    33158, 33294, 33429, 33564, 33697, 33831, 33963, 34095, 34226, 34357, 34486, 34616, 34744, 34873, 35000, 35127,
    35253, 35379, 35504, 35629, 35753, 35876, 35999, 36122, 36244, 36365, 36486, 36606, 36726, 36845, 36964, 37083,
    37201, 37318, 37435, 37551, 37668, 37783, 37898, 38013, 38127, 38241, 38354, 38467, 38580, 38692, 38803, 38915,
    39026, 39136, 39246, 39356, 39465, 39574, 39682, 39790, 39898, 40005, 40112, 40219, 40325, 40431, 40537, 40642,
    40747, 40851, 40955, 41059, 41163, 41266, 41369, 41471, 41573, 41675, 41777, 41878, 41979, 42079, 42179, 42279,
    42379, 42478, 42577, 42676, 42775, 42873, 42971, 43068, 43165, 43262, 43359, 43456, 43552, 43648, 43743, 43839,
    43934, 44028, 44123, 44217, 44311, 44405, 44499, 44592, 44685, 44778, 44870, 44962, 45054, 45146, 45238, 45329,
    45420, 45511, 45601, 45692, 45782, 45872, 45961, 46051, 46140, 46229, 46318, 46406, 46494, 46583, 46670, 46758,
    46846, 46933, 47020, 47107, 47193, 47280, 47366, 47452, 47538, 47623, 47709, 47794, 47879, 47964, 48048, 48133,
    48217, 48301, 48385, 48468, 48552, 48635, 48718, 48801, 48884, 48966, 49048, 49131, 49213, 49294, 49376, 49458,
    49539, 49620, 49701, 49782, 49862, 49943, 50023, 50103, 50183, 50263, 50342, 50422, 50501, 50580, 50659, 50738,
    50816, 50895, 50973, 51051, 51129, 51207, 51285, 51362, 51439, 51517, 51594, 51671, 51747, 51824, 51900, 51977,
    52053, 52129, 52205, 52280, 52356, 52432, 52507, 52582, 52657, 52732, 52807, 52881, 52956, 53030, 53104, 53178,
    53252, 53326, 53400, 53473, 53546, 53620, 53693, 53766, 53839, 53911, 53984, 54056, 54129, 54201, 54273, 54345,
    54417, 54489, 54560, 54632, 54703, 54774, 54845, 54916, 54987, 55058, 55129, 55199, 55269, 55340, 55410, 55480,
    55550, 55620, 55689, 55759, 55828, 55898, 55967, 56036, 56105, 56174, 56243, 56311, 56380, 56448, 56517, 56585,
    56653, 56721, 56789, 56857, 56924, 56992, 57059, 57127, 57194, 57261, 57328, 57395, 57462, 57529, 57595, 57662,
    57728, 57795, 57861, 57927, 57993, 58059, 58125, 58191, 58256, 58322, 58387, 58453, 58518, 58583, 58648, 58713,
    58778, 58843, 58908, 58972, 59037, 59101, 59165, 59230, 59294, 59358, 59422, 59486, 59549, 59613, 59677, 59740,
    59804, 59867, 59930, 59993, 60056, 60119, 60182, 60245, 60308, 60370, 60433, 60495, 60558, 60620, 60682, 60744,
    60806, 60868, 60930, 60992, 61054, 61115, 61177, 61238, 61300, 61361, 61422, 61483, 61544, 61605, 61666, 61727,
    61788, 61848, 61909, 61969, 62030, 62090, 62150, 62211, 62271, 62331, 62391, 62450, 62510, 62570, 62630, 62689,
    62749, 62808, 62867, 62927, 62986, 63045, 63104, 63163, 63222, 63281, 63340, 63398, 63457, 63515, 63574, 63632,
    63691, 63749, 63807, 63865, 63923, 63981, 64039, 64097, 64155, 64212, 64270, 64328, 64385, 64443, 64500, 64557,
    64614, 64672, 64729, 64786, 64843, 64900, 64956, 65013, 65070, 65126, 65183, 65239, 65296, 65352, 65409, 65465,
];

static PNG_sRGB_DELTA: [png_byte; 512] = [
    207, 201, 158, 129, 113, 100, 90, 82, 77, 72, 68, 64, 61, 59, 56, 54,
    52, 50, 49, 47, 46, 45, 43, 42, 41, 40, 39, 39, 38, 37, 36, 36,
    35, 34, 34, 33, 33, 32, 32, 31, 31, 30, 30, 30, 29, 29, 28, 28,
    28, 27, 27, 27, 27, 26, 26, 26, 25, 25, 25, 25, 24, 24, 24, 24,
    23, 23, 23, 23, 23, 22, 22, 22, 22, 22, 22, 21, 21, 21, 21, 21,
    21, 20, 20, 20, 20, 20, 20, 20, 20, 19, 19, 19, 19, 19, 19, 19,
    19, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 17, 17, 17, 17, 17,
    17, 17, 17, 17, 17, 17, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 16, 16, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14,
    14, 14, 14, 14, 14, 14, 14, 13, 13, 13, 13, 13, 13, 13, 13, 13,
    13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 12, 12,
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    10, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
];

/// PNG_sRGB_FROM_LINEAR(linear)
#[inline]
fn png_srgb_from_linear(linear: u32) -> png_byte {
    let idx = (linear >> 15) as usize;
    (0xff
        & (((PNG_sRGB_BASE[idx] as u32)
            + (((linear & 0x7fff) * (PNG_sRGB_DELTA[idx] as u32)) >> 12))
            >> 8)) as png_byte
}

/* Write out all the unknown chunks for the current given location */
unsafe fn write_unknown_chunks(
    png_ptr: png_structrp,
    info_ptr: png_const_inforp,
    where_: c_uint,
) {
    if (*info_ptr).unknown_chunks_num != 0 {
        let mut up = (*info_ptr).unknown_chunks as png_const_unknown_chunkp;
        let end = (*info_ptr)
            .unknown_chunks
            .offset((*info_ptr).unknown_chunks_num as isize) as png_const_unknown_chunkp;

        while up < end {
            if ((*up).location as c_uint & where_) != 0 {
                /* If per-chunk unknown chunk handling is enabled use it, otherwise
                 * just write the chunks the application has set.
                 */
                let keep = png_handle_as_unknown(png_ptr, (*up).name.as_ptr());

                if keep != PNG_HANDLE_CHUNK_NEVER
                    && (((*up).name[3] & 0x20) != 0
                        || keep == PNG_HANDLE_CHUNK_ALWAYS
                        || (keep == PNG_HANDLE_CHUNK_AS_DEFAULT
                            && (*png_ptr).unknown_default == PNG_HANDLE_CHUNK_ALWAYS))
                {
                    if (*up).size == 0 {
                        png_warning(png_ptr, c"Writing zero-length unknown chunk".as_ptr());
                    }

                    png_write_chunk(png_ptr, (*up).name.as_ptr(), (*up).data, (*up).size);
                }
            }
            up = up.offset(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_info_before_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_const_inforp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    if ((*png_ptr).mode & PNG_WROTE_INFO_BEFORE_PLTE) == 0 {
        /* Write PNG signature */
        png_write_sig(png_ptr);

        if ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) != 0
            && (*png_ptr).mng_features_permitted != 0
        {
            png_warning(
                png_ptr,
                c"MNG features are not allowed in a PNG datastream".as_ptr(),
            );
            (*png_ptr).mng_features_permitted = 0;
        }

        /* Write IHDR information. */
        png_write_IHDR(
            png_ptr,
            (*info_ptr).width,
            (*info_ptr).height,
            (*info_ptr).bit_depth as c_int,
            (*info_ptr).color_type as c_int,
            (*info_ptr).compression_type as c_int,
            (*info_ptr).filter_type as c_int,
            (*info_ptr).interlace_type as c_int,
        );

        write_unknown_chunks(png_ptr, info_ptr, PNG_HAVE_IHDR);

        if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
            png_write_sBIT(png_ptr, &(*info_ptr).sig_bit, (*info_ptr).color_type as c_int);
        }

        if ((*info_ptr).valid & PNG_INFO_cLLI) != 0 {
            png_write_cLLI_fixed(png_ptr, (*info_ptr).maxCLL, (*info_ptr).maxFALL);
        }

        if ((*info_ptr).valid & PNG_INFO_mDCV) != 0 {
            png_write_mDCV_fixed(
                png_ptr,
                (*info_ptr).mastering_red_x,
                (*info_ptr).mastering_red_y,
                (*info_ptr).mastering_green_x,
                (*info_ptr).mastering_green_y,
                (*info_ptr).mastering_blue_x,
                (*info_ptr).mastering_blue_y,
                (*info_ptr).mastering_white_x,
                (*info_ptr).mastering_white_y,
                (*info_ptr).mastering_maxDL,
                (*info_ptr).mastering_minDL,
            );
        }

        if ((*info_ptr).valid & PNG_INFO_cICP) != 0 {
            png_write_cICP(
                png_ptr,
                (*info_ptr).cicp_colour_primaries,
                (*info_ptr).cicp_transfer_function,
                (*info_ptr).cicp_matrix_coefficients,
                (*info_ptr).cicp_video_full_range_flag,
            );
        }

        if ((*info_ptr).valid & PNG_INFO_iCCP) != 0 {
            png_write_iCCP(
                png_ptr,
                (*info_ptr).iccp_name,
                (*info_ptr).iccp_profile,
                (*info_ptr).iccp_proflen,
            );
        }

        if ((*info_ptr).valid & PNG_INFO_sRGB) != 0 {
            png_write_sRGB(png_ptr, (*info_ptr).rendering_intent);
        }

        if ((*info_ptr).valid & PNG_INFO_gAMA) != 0 {
            png_write_gAMA_fixed(png_ptr, (*info_ptr).gamma);
        }

        if ((*info_ptr).valid & PNG_INFO_cHRM) != 0 {
            png_write_cHRM_fixed(png_ptr, &(*info_ptr).cHRM);
        }

        (*png_ptr).mode |= PNG_WROTE_INFO_BEFORE_PLTE;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_info(png_ptr: png_structrp, info_ptr: png_const_inforp) {
    let mut i: c_int;

    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    png_write_info_before_PLTE(png_ptr, info_ptr);

    if ((*info_ptr).valid & PNG_INFO_PLTE) != 0 {
        png_write_PLTE(png_ptr, (*info_ptr).palette, (*info_ptr).num_palette as png_uint_32);
    } else if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        png_error(png_ptr, c"Valid palette required for paletted images".as_ptr());
    }

    if ((*info_ptr).valid & PNG_INFO_tRNS) != 0 {
        /* Invert the alpha channel (in tRNS) */
        if ((*png_ptr).transformations & PNG_INVERT_ALPHA) != 0
            && (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        {
            let mut jend = (*info_ptr).num_trans as c_int;
            if jend > PNG_MAX_PALETTE_LENGTH {
                jend = PNG_MAX_PALETTE_LENGTH;
            }

            let mut j = 0;
            while j < jend {
                *(*info_ptr).trans_alpha.offset(j as isize) =
                    (255i32 - *(*info_ptr).trans_alpha.offset(j as isize) as i32) as png_byte;
                j += 1;
            }
        }
        png_write_tRNS(
            png_ptr,
            (*info_ptr).trans_alpha,
            &(*info_ptr).trans_color,
            (*info_ptr).num_trans as c_int,
            (*info_ptr).color_type as c_int,
        );
    }

    if ((*info_ptr).valid & PNG_INFO_bKGD) != 0 {
        png_write_bKGD(png_ptr, &(*info_ptr).background, (*info_ptr).color_type as c_int);
    }

    if ((*info_ptr).valid & PNG_INFO_eXIf) != 0 {
        png_write_eXIf(png_ptr, (*info_ptr).exif, (*info_ptr).num_exif as c_int);
        (*png_ptr).mode |= PNG_WROTE_eXIf;
    }

    if ((*info_ptr).valid & PNG_INFO_hIST) != 0 {
        png_write_hIST(png_ptr, (*info_ptr).hist, (*info_ptr).num_palette as c_int);
    }

    if ((*info_ptr).valid & PNG_INFO_oFFs) != 0 {
        png_write_oFFs(
            png_ptr,
            (*info_ptr).x_offset,
            (*info_ptr).y_offset,
            (*info_ptr).offset_unit_type as c_int,
        );
    }

    if ((*info_ptr).valid & PNG_INFO_pCAL) != 0 {
        png_write_pCAL(
            png_ptr,
            (*info_ptr).pcal_purpose,
            (*info_ptr).pcal_X0,
            (*info_ptr).pcal_X1,
            (*info_ptr).pcal_type as c_int,
            (*info_ptr).pcal_nparams as c_int,
            (*info_ptr).pcal_units,
            (*info_ptr).pcal_params,
        );
    }

    if ((*info_ptr).valid & PNG_INFO_sCAL) != 0 {
        png_write_sCAL_s(
            png_ptr,
            (*info_ptr).scal_unit as c_int,
            (*info_ptr).scal_s_width,
            (*info_ptr).scal_s_height,
        );
    }

    if ((*info_ptr).valid & PNG_INFO_pHYs) != 0 {
        png_write_pHYs(
            png_ptr,
            (*info_ptr).x_pixels_per_unit,
            (*info_ptr).y_pixels_per_unit,
            (*info_ptr).phys_unit_type as c_int,
        );
    }

    if ((*info_ptr).valid & PNG_INFO_tIME) != 0 {
        png_write_tIME(png_ptr, &(*info_ptr).mod_time);
        (*png_ptr).mode |= PNG_WROTE_tIME;
    }

    if ((*info_ptr).valid & PNG_INFO_sPLT) != 0 {
        i = 0;
        while i < (*info_ptr).splt_palettes_num as c_int {
            png_write_sPLT(png_ptr, (*info_ptr).splt_palettes.offset(i as isize));
            i += 1;
        }
    }

    /* Check to see if we need to write text chunks */
    i = 0;
    while i < (*info_ptr).num_text {
        let text = (*info_ptr).text.offset(i as isize);
        /* An internationalized chunk? */
        if (*text).compression > 0 {
            /* Write international chunk */
            png_write_iTXt(
                png_ptr,
                (*text).compression,
                (*text).key,
                (*text).lang,
                (*text).lang_key,
                (*text).text,
            );
            /* Mark this chunk as written */
            if (*text).compression == PNG_TEXT_COMPRESSION_NONE {
                (*text).compression = PNG_TEXT_COMPRESSION_NONE_WR;
            } else {
                (*text).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
            }
        } else if (*text).compression == PNG_TEXT_COMPRESSION_zTXt {
            /* Write compressed chunk */
            png_write_zTXt(png_ptr, (*text).key, (*text).text, (*text).compression);
            (*text).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
        } else if (*text).compression == PNG_TEXT_COMPRESSION_NONE {
            /* Write uncompressed chunk */
            png_write_tEXt(png_ptr, (*text).key, (*text).text, 0);
            (*text).compression = PNG_TEXT_COMPRESSION_NONE_WR;
        }
        i += 1;
    }

    write_unknown_chunks(png_ptr, info_ptr, PNG_HAVE_PLTE);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    if png_ptr.is_null() {
        return;
    }

    if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0 {
        png_error(png_ptr, c"No IDATs written into file".as_ptr());
    }

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        && (*png_ptr).num_palette_max >= (*png_ptr).num_palette as c_int
    {
        png_benign_error(png_ptr, c"Wrote palette index exceeding num_palette".as_ptr());
    }

    /* See if user wants us to write information chunks */
    if !info_ptr.is_null() {
        let mut i: c_int;

        /* Check to see if user has supplied a time chunk */
        if ((*info_ptr).valid & PNG_INFO_tIME) != 0 && ((*png_ptr).mode & PNG_WROTE_tIME) == 0 {
            png_write_tIME(png_ptr, &(*info_ptr).mod_time);
        }

        /* Loop through comment chunks */
        i = 0;
        while i < (*info_ptr).num_text {
            let text = (*info_ptr).text.offset(i as isize);
            /* An internationalized chunk? */
            if (*text).compression > 0 {
                /* Write international chunk */
                png_write_iTXt(
                    png_ptr,
                    (*text).compression,
                    (*text).key,
                    (*text).lang,
                    (*text).lang_key,
                    (*text).text,
                );
                if (*text).compression == PNG_TEXT_COMPRESSION_NONE {
                    (*text).compression = PNG_TEXT_COMPRESSION_NONE_WR;
                } else {
                    (*text).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
                }
            } else if (*text).compression >= PNG_TEXT_COMPRESSION_zTXt {
                /* Write compressed chunk */
                png_write_zTXt(png_ptr, (*text).key, (*text).text, (*text).compression);
                (*text).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
            } else if (*text).compression == PNG_TEXT_COMPRESSION_NONE {
                /* Write uncompressed chunk */
                png_write_tEXt(png_ptr, (*text).key, (*text).text, 0);
                (*text).compression = PNG_TEXT_COMPRESSION_NONE_WR;
            }
            i += 1;
        }

        if ((*info_ptr).valid & PNG_INFO_eXIf) != 0 && ((*png_ptr).mode & PNG_WROTE_eXIf) == 0 {
            png_write_eXIf(png_ptr, (*info_ptr).exif, (*info_ptr).num_exif as c_int);
        }

        write_unknown_chunks(png_ptr, info_ptr, PNG_AFTER_IDAT);
    }

    (*png_ptr).mode |= PNG_AFTER_IDAT;

    /* Write end of PNG file */
    png_write_IEND(png_ptr);

    /* PNG_WRITE_FLUSH_AFTER_IEND_SUPPORTED is off, so no flush here. */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_from_struct_tm(ptime: png_timep, ttime: *const tm) {
    (*ptime).year = (1900 + (*ttime).tm_year) as png_uint_16;
    (*ptime).month = ((*ttime).tm_mon + 1) as png_byte;
    (*ptime).day = (*ttime).tm_mday as png_byte;
    (*ptime).hour = (*ttime).tm_hour as png_byte;
    (*ptime).minute = (*ttime).tm_min as png_byte;
    (*ptime).second = (*ttime).tm_sec as png_byte;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_from_time_t(ptime: png_timep, ttime: time_t) {
    let tbuf = gmtime(&ttime);
    if tbuf.is_null() {
        memset(ptime as *mut c_void, 0, core::mem::size_of::<png_time>());
        return;
    }

    png_convert_from_struct_tm(ptime, tbuf);
}

/* Initialize png_ptr structure, and allocate any memory needed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_write_struct(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
) -> png_structp {
    /* USER_MEM is supported: forward to the _2 variant. */
    png_create_write_struct_2(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        ptr::null_mut(),
        None,
        None,
    )
}

/* Alternate initialize png_ptr structure, and allocate any memory needed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_write_struct_2(
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
        /* Set the zlib control values to defaults. */
        (*png_ptr).zbuffer_size = PNG_ZBUF_SIZE as crate::cffi::uInt;

        (*png_ptr).zlib_strategy = PNG_Z_DEFAULT_STRATEGY;
        (*png_ptr).zlib_level = PNG_Z_DEFAULT_COMPRESSION;
        (*png_ptr).zlib_mem_level = 8;
        (*png_ptr).zlib_window_bits = 15;
        (*png_ptr).zlib_method = 8;

        (*png_ptr).zlib_text_strategy = PNG_TEXT_Z_DEFAULT_STRATEGY;
        (*png_ptr).zlib_text_level = PNG_TEXT_Z_DEFAULT_COMPRESSION;
        (*png_ptr).zlib_text_mem_level = 8;
        (*png_ptr).zlib_text_window_bits = 15;
        (*png_ptr).zlib_text_method = 8;

        /* BENIGN_WRITE_ERRORS and RELEASE_BUILD are both off in this config. */

        png_set_write_fn(png_ptr, ptr::null_mut(), None, None);
    }

    png_ptr
}

/* Write a few rows of image data. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_rows(
    png_ptr: png_structrp,
    row: png_bytepp,
    num_rows: png_uint_32,
) {
    if png_ptr.is_null() {
        return;
    }

    /* Loop through the rows */
    let mut i: png_uint_32 = 0;
    let mut rp = row;
    while i < num_rows {
        png_write_row(png_ptr, *rp);
        i += 1;
        rp = rp.offset(1);
    }
}

/* Write the image. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_image(png_ptr: png_structrp, image: png_bytepp) {
    if png_ptr.is_null() {
        return;
    }

    /* Initialize interlace handling.  If image is not interlaced,
     * this will set pass to 1
     */
    let num_pass = png_set_interlace_handling(png_ptr);

    /* Loop through passes */
    let mut pass = 0;
    while pass < num_pass {
        /* Loop through image */
        let mut i: png_uint_32 = 0;
        let mut rp = image;
        while i < (*png_ptr).height {
            png_write_row(png_ptr, *rp);
            i += 1;
            rp = rp.offset(1);
        }
        pass += 1;
    }
}

/* Performs intrapixel differencing  */
unsafe fn png_do_write_intrapixel(row_info: png_row_infop, row: png_bytep) {
    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        let bytes_per_pixel: c_int;
        let row_width = (*row_info).width;
        if (*row_info).bit_depth == 8 {
            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
                bytes_per_pixel = 3;
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                bytes_per_pixel = 4;
            } else {
                return;
            }

            let mut rp = row;
            let mut i: png_uint_32 = 0;
            while i < row_width {
                *rp = (*rp).wrapping_sub(*rp.add(1));
                *rp.add(2) = (*rp.add(2)).wrapping_sub(*rp.add(1));
                i += 1;
                rp = rp.offset(bytes_per_pixel as isize);
            }
        } else if (*row_info).bit_depth == 16 {
            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
                bytes_per_pixel = 6;
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                bytes_per_pixel = 8;
            } else {
                return;
            }

            let mut rp = row;
            let mut i: png_uint_32 = 0;
            while i < row_width {
                let s0: png_uint_32 = ((*rp as u32) << 8) | (*rp.add(1) as u32);
                let s1: png_uint_32 = ((*rp.add(2) as u32) << 8) | (*rp.add(3) as u32);
                let s2: png_uint_32 = ((*rp.add(4) as u32) << 8) | (*rp.add(5) as u32);
                let red: png_uint_32 = s0.wrapping_sub(s1) & 0xffff;
                let blue: png_uint_32 = s2.wrapping_sub(s1) & 0xffff;
                *rp = (red >> 8) as png_byte;
                *rp.add(1) = red as png_byte;
                *rp.add(4) = (blue >> 8) as png_byte;
                *rp.add(5) = blue as png_byte;
                i += 1;
                rp = rp.offset(bytes_per_pixel as isize);
            }
        }
    }
}

/* Called by user to write a row of image data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_row(png_ptr: png_structrp, row: png_const_bytep) {
    /* 1.5.6: moved from png_struct to be a local structure: */
    let mut row_info: png_row_info = core::mem::zeroed();

    if png_ptr.is_null() {
        return;
    }

    /* Initialize transformations and other stuff if first time */
    if (*png_ptr).row_number == 0 && (*png_ptr).pass == 0 {
        /* Make sure we wrote the header info */
        if ((*png_ptr).mode & PNG_WROTE_INFO_BEFORE_PLTE) == 0 {
            png_error(
                png_ptr,
                c"png_write_info was never called before png_write_row".as_ptr(),
            );
        }

        /* All the WRITE transforms are supported in this config, so none of the
         * "not defined" warnings apply.
         */

        png_write_start_row(png_ptr);
    }

    /* If interlaced and not interested in row, return */
    if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        match (*png_ptr).pass {
            0 => {
                if ((*png_ptr).row_number & 0x07) != 0 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            1 => {
                if ((*png_ptr).row_number & 0x07) != 0 || (*png_ptr).width < 5 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            2 => {
                if ((*png_ptr).row_number & 0x07) != 4 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            3 => {
                if ((*png_ptr).row_number & 0x03) != 0 || (*png_ptr).width < 3 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            4 => {
                if ((*png_ptr).row_number & 0x03) != 2 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            5 => {
                if ((*png_ptr).row_number & 0x01) != 0 || (*png_ptr).width < 2 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            6 => {
                if ((*png_ptr).row_number & 0x01) == 0 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            _ => {}
        }
    }

    /* Set up row info for transformations */
    row_info.color_type = (*png_ptr).color_type;
    row_info.width = (*png_ptr).usr_width;
    row_info.channels = (*png_ptr).usr_channels;
    row_info.bit_depth = (*png_ptr).usr_bit_depth;
    row_info.pixel_depth = (row_info.bit_depth as u32 * row_info.channels as u32) as png_byte;
    row_info.rowbytes = png_rowbytes(row_info.pixel_depth as u32, row_info.width);

    /* Copy user's row into buffer, leaving room for filter byte. */
    memcpy(
        (*png_ptr).row_buf.add(1) as *mut c_void,
        row as *const c_void,
        row_info.rowbytes,
    );

    /* Handle interlacing */
    if (*png_ptr).interlaced != 0
        && (*png_ptr).pass < 6
        && ((*png_ptr).transformations & PNG_INTERLACE) != 0
    {
        png_do_write_interlace(&mut row_info, (*png_ptr).row_buf.add(1), (*png_ptr).pass as c_int);
        /* This should always get caught above, but still ... */
        if row_info.width == 0 {
            png_write_finish_row(png_ptr);
            return;
        }
    }

    /* Handle other transformations */
    if (*png_ptr).transformations != 0 {
        png_do_write_transformations(png_ptr, &mut row_info);
    }

    /* At this point the row_info pixel depth must match the 'transformed' depth,
     * which is also the output depth.
     */
    if row_info.pixel_depth != (*png_ptr).pixel_depth
        || row_info.pixel_depth != (*png_ptr).transformed_pixel_depth
    {
        png_error(png_ptr, c"internal write transform logic error".as_ptr());
    }

    /* Write filter_method 64 (intrapixel differencing) only under MNG rules */
    if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
        && ((*png_ptr).filter_type as c_int == PNG_INTRAPIXEL_DIFFERENCING)
    {
        png_do_write_intrapixel(&mut row_info, (*png_ptr).row_buf.add(1));
    }

    /* Check for out-of-range palette index */
    if row_info.color_type as c_int == PNG_COLOR_TYPE_PALETTE && (*png_ptr).num_palette_max >= 0 {
        png_do_check_palette_indexes(png_ptr, &mut row_info);
    }

    /* Find a filter if necessary, filter the row and write it out. */
    png_write_find_filter(png_ptr, &mut row_info);

    if (*png_ptr).write_row_fn.is_some() {
        ((*png_ptr).write_row_fn.unwrap())(png_ptr, (*png_ptr).row_number, (*png_ptr).pass as c_int);
    }
}

/* Set the automatic flush interval or 0 to turn flushing off */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_flush(png_ptr: png_structrp, nrows: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).flush_dist = if nrows < 0 { 0 } else { nrows as png_uint_32 };
}

/* Flush the current output buffers now */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_flush(png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }

    /* We have already written out all of the data */
    if (*png_ptr).row_number >= (*png_ptr).num_rows {
        return;
    }

    png_compress_IDAT(png_ptr, ptr::null(), 0, Z_SYNC_FLUSH);
    (*png_ptr).flush_rows = 0;
    png_flush(png_ptr);
}

/* Free any memory used in png_ptr struct without freeing the struct itself. */
unsafe fn png_write_destroy(png_ptr: png_structrp) {
    /* Free any memory zlib uses */
    if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_INITIALIZED) != 0 {
        deflateEnd(&mut (*png_ptr).zstream);
    }

    /* Free our memory.  png_free checks NULL for us. */
    png_free_buffer_list(png_ptr, &mut (*png_ptr).zbuffer_list);
    png_free(png_ptr, (*png_ptr).row_buf as png_voidp);
    (*png_ptr).row_buf = ptr::null_mut();
    png_free(png_ptr, (*png_ptr).prev_row as png_voidp);
    png_free(png_ptr, (*png_ptr).try_row as png_voidp);
    png_free(png_ptr, (*png_ptr).tst_row as png_voidp);
    (*png_ptr).prev_row = ptr::null_mut();
    (*png_ptr).try_row = ptr::null_mut();
    (*png_ptr).tst_row = ptr::null_mut();

    png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
    (*png_ptr).chunk_list = ptr::null_mut();

    /* Free the independent copy of trans_alpha owned by png_struct. */
    png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
    (*png_ptr).trans_alpha = ptr::null_mut();

    /* Free the independent copy of the palette owned by png_struct. */
    png_free(png_ptr, (*png_ptr).palette as png_voidp);
    (*png_ptr).palette = ptr::null_mut();
}

/* Free all memory used by the write. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_write_struct(
    png_ptr_ptr: png_structpp,
    info_ptr_ptr: png_infopp,
) {
    if !png_ptr_ptr.is_null() {
        let png_ptr = *png_ptr_ptr;

        if !png_ptr.is_null() {
            png_destroy_info_struct(png_ptr, info_ptr_ptr);

            *png_ptr_ptr = ptr::null_mut();
            png_write_destroy(png_ptr);
            png_destroy_png_struct(png_ptr);
        }
    }
}

/* Allow the application to select one or more row filters to use. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter(png_ptr: png_structrp, mut method: c_int, mut filters: c_int) {
    if png_ptr.is_null() {
        return;
    }

    if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
        && (method == PNG_INTRAPIXEL_DIFFERENCING)
    {
        method = PNG_FILTER_TYPE_BASE;
    }

    if method == PNG_FILTER_TYPE_BASE {
        match filters & (PNG_ALL_FILTERS | 0x07) {
            5 | 6 | 7 => {
                png_app_error(png_ptr, c"Unknown row filter for method 0".as_ptr());
                /* FALLTHROUGH */
                (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
            }
            x if x == PNG_FILTER_VALUE_NONE => {
                (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
            }
            x if x == PNG_FILTER_VALUE_SUB => {
                (*png_ptr).do_filter = PNG_FILTER_SUB as png_byte;
            }
            x if x == PNG_FILTER_VALUE_UP => {
                (*png_ptr).do_filter = PNG_FILTER_UP as png_byte;
            }
            x if x == PNG_FILTER_VALUE_AVG => {
                (*png_ptr).do_filter = PNG_FILTER_AVG as png_byte;
            }
            x if x == PNG_FILTER_VALUE_PAETH => {
                (*png_ptr).do_filter = PNG_FILTER_PAETH as png_byte;
            }
            _ => {
                (*png_ptr).do_filter = filters as png_byte;
            }
        }

        if !(*png_ptr).row_buf.is_null() {
            let mut num_filters: c_int;
            let buf_size: png_alloc_size_t;

            /* Repeat the checks in png_write_start_row */
            if (*png_ptr).height == 1 {
                filters &= !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }

            if (*png_ptr).width == 1 {
                filters &= !(PNG_FILTER_SUB | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }

            if (filters & (PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH)) != 0
                && (*png_ptr).prev_row.is_null()
            {
                png_app_warning(
                    png_ptr,
                    c"png_set_filter: UP/AVG/PAETH cannot be added after start".as_ptr(),
                );
                filters &= !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }

            num_filters = 0;

            if (filters & PNG_FILTER_SUB) != 0 {
                num_filters += 1;
            }
            if (filters & PNG_FILTER_UP) != 0 {
                num_filters += 1;
            }
            if (filters & PNG_FILTER_AVG) != 0 {
                num_filters += 1;
            }
            if (filters & PNG_FILTER_PAETH) != 0 {
                num_filters += 1;
            }

            /* Allocate needed row buffers if not already allocated. */
            buf_size = (png_rowbytes(
                ((*png_ptr).usr_channels as u32) * ((*png_ptr).usr_bit_depth as u32),
                (*png_ptr).width,
            ) + 1) as png_alloc_size_t;

            if (*png_ptr).try_row.is_null() {
                (*png_ptr).try_row = png_malloc(png_ptr, buf_size) as png_bytep;
            }

            if num_filters > 1 {
                if (*png_ptr).tst_row.is_null() {
                    (*png_ptr).tst_row = png_malloc(png_ptr, buf_size) as png_bytep;
                }
            }
        }
        (*png_ptr).do_filter = filters as png_byte;
    } else {
        png_error(png_ptr, c"Unknown custom filter method".as_ptr());
    }
}

/* DEPRECATED: filter heuristics APIs are now no-ops. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter_heuristics(
    png_ptr: png_structrp,
    heuristic_method: c_int,
    num_weights: c_int,
    filter_weights: png_const_doublep,
    filter_costs: png_const_doublep,
) {
    let _ = (png_ptr, heuristic_method, num_weights, filter_weights, filter_costs);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter_heuristics_fixed(
    png_ptr: png_structrp,
    heuristic_method: c_int,
    num_weights: c_int,
    filter_weights: png_const_fixed_point_p,
    filter_costs: png_const_fixed_point_p,
) {
    let _ = (png_ptr, heuristic_method, num_weights, filter_weights, filter_costs);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_level(png_ptr: png_structrp, level: c_int) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).zlib_level = level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_mem_level(png_ptr: png_structrp, mem_level: c_int) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).zlib_mem_level = mem_level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_strategy(png_ptr: png_structrp, strategy: c_int) {
    if png_ptr.is_null() {
        return;
    }

    /* The flag setting here prevents the libpng dynamic selection of strategy. */
    (*png_ptr).flags |= PNG_FLAG_ZLIB_CUSTOM_STRATEGY;
    (*png_ptr).zlib_strategy = strategy;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_window_bits(
    png_ptr: png_structrp,
    mut window_bits: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    if window_bits > 15 {
        png_warning(png_ptr, c"Only compression windows <= 32k supported by PNG".as_ptr());
        window_bits = 15;
    } else if window_bits < 8 {
        png_warning(png_ptr, c"Only compression windows >= 256 supported by PNG".as_ptr());
        window_bits = 8;
    }

    (*png_ptr).zlib_window_bits = window_bits;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_method(png_ptr: png_structrp, method: c_int) {
    if png_ptr.is_null() {
        return;
    }

    if method != 8 {
        png_warning(png_ptr, c"Only compression method 8 is supported by PNG".as_ptr());
    }

    (*png_ptr).zlib_method = method;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_level(png_ptr: png_structrp, level: c_int) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).zlib_text_level = level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_mem_level(png_ptr: png_structrp, mem_level: c_int) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).zlib_text_mem_level = mem_level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_strategy(png_ptr: png_structrp, strategy: c_int) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).zlib_text_strategy = strategy;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_window_bits(
    png_ptr: png_structrp,
    mut window_bits: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    if window_bits > 15 {
        png_warning(png_ptr, c"Only compression windows <= 32k supported by PNG".as_ptr());
        window_bits = 15;
    } else if window_bits < 8 {
        png_warning(png_ptr, c"Only compression windows >= 256 supported by PNG".as_ptr());
        window_bits = 8;
    }

    (*png_ptr).zlib_text_window_bits = window_bits;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_method(png_ptr: png_structrp, method: c_int) {
    if png_ptr.is_null() {
        return;
    }

    if method != 8 {
        png_warning(png_ptr, c"Only compression method 8 is supported by PNG".as_ptr());
    }

    (*png_ptr).zlib_text_method = method;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_write_status_fn(
    png_ptr: png_structrp,
    write_row_fn: png_write_status_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).write_row_fn = write_row_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_write_user_transform_fn(
    png_ptr: png_structrp,
    write_user_transform_fn: png_user_transform_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).transformations |= PNG_USER_TRANSFORM;
    (*png_ptr).write_user_transform_fn = write_user_transform_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_png(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    transforms: c_int,
    params: png_voidp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    if ((*info_ptr).valid & PNG_INFO_IDAT) == 0 {
        png_app_error(png_ptr, c"no rows for png_write_image to write".as_ptr());
        return;
    }

    /* Write the file header information. */
    png_write_info(png_ptr, info_ptr);

    /* ------ these transformations don't touch the info structure ------- */

    /* Invert monochrome pixels */
    if (transforms & PNG_TRANSFORM_INVERT_MONO) != 0 {
        png_set_invert_mono(png_ptr);
    }

    /* Shift the pixels up to a legal bit depth and fill in as appropriate. */
    if (transforms & PNG_TRANSFORM_SHIFT) != 0 {
        if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
            png_set_shift(png_ptr, &(*info_ptr).sig_bit);
        }
    }

    /* Pack pixels into bytes */
    if (transforms & PNG_TRANSFORM_PACKING) != 0 {
        png_set_packing(png_ptr);
    }

    /* Swap location of alpha bytes from ARGB to RGBA */
    if (transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0 {
        png_set_swap_alpha(png_ptr);
    }

    /* Remove a filler (X) from XRGB/RGBX/AG/GA into to convert it into RGB. */
    if (transforms & (PNG_TRANSFORM_STRIP_FILLER_AFTER | PNG_TRANSFORM_STRIP_FILLER_BEFORE)) != 0 {
        if (transforms & PNG_TRANSFORM_STRIP_FILLER_AFTER) != 0 {
            if (transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0 {
                png_app_error(
                    png_ptr,
                    c"PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported".as_ptr(),
                );
            }

            /* Continue if ignored - this is the pre-1.6.10 behavior */
            png_set_filler(png_ptr, 0, PNG_FILLER_AFTER);
        } else if (transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0 {
            png_set_filler(png_ptr, 0, PNG_FILLER_BEFORE);
        }
    }

    /* Flip BGR pixels to RGB */
    if (transforms & PNG_TRANSFORM_BGR) != 0 {
        png_set_bgr(png_ptr);
    }

    /* Swap bytes of 16-bit files to most significant byte first */
    if (transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0 {
        png_set_swap(png_ptr);
    }

    /* Swap bits of 1-bit, 2-bit, 4-bit packed pixel formats */
    if (transforms & PNG_TRANSFORM_PACKSWAP) != 0 {
        png_set_packswap(png_ptr);
    }

    /* Invert the alpha channel from opacity to transparency */
    if (transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0 {
        png_set_invert_alpha(png_ptr);
    }

    /* ----------------------- end of transformations ------------------- */

    /* Write the bits */
    png_write_image(png_ptr, (*info_ptr).row_pointers);

    /* It is REQUIRED to call this to finish writing the rest of the file */
    png_write_end(png_ptr, info_ptr);

    let _ = params;
}

// ---- Simplified write API ----

/* Arguments to png_image_write_main: */
#[repr(C)]
struct png_image_write_control {
    /* Arguments */
    image: png_imagep,
    buffer: png_const_voidp,
    row_stride: png_int_32,
    colormap: png_const_voidp,
    convert_to_8bit: c_int,

    /* Instance variables */
    first_row: png_const_voidp,
    local_row: png_voidp,
    row_step: isize, /* ptrdiff_t */

    /* Byte count for memory writing */
    memory: png_bytep,
    memory_bytes: png_alloc_size_t,
    output_bytes: png_alloc_size_t,
}

// Shim adapting png_safe_error (returns `!`) to the png_error_ptr type, which
// has no never-type; the underlying function never returns anyway.
unsafe extern "C" fn png_safe_error_shim(png_ptr: png_structp, msg: png_const_charp) {
    png_safe_error(png_ptr, msg)
}

/* Initialize the write structure - general purpose utility. */
unsafe fn png_image_write_init(image: png_imagep) -> c_int {
    let mut png_ptr = png_create_write_struct(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        image as png_voidp,
        Some(png_safe_error_shim),
        Some(png_safe_warning),
    );

    if !png_ptr.is_null() {
        let mut info_ptr = png_create_info_struct(png_ptr);

        if !info_ptr.is_null() {
            let control =
                png_malloc_warn(png_ptr, core::mem::size_of::<png_control>() as png_alloc_size_t)
                    as png_controlp;

            if !control.is_null() {
                memset(control as *mut c_void, 0, core::mem::size_of::<png_control>());

                (*control).png_ptr = png_ptr;
                (*control).info_ptr = info_ptr;
                (*control).set_for_write(true);

                (*image).opaque = control;
                return 1;
            }

            /* Error clean up */
            png_destroy_info_struct(png_ptr, &mut info_ptr);
        }

        png_destroy_write_struct(&mut png_ptr, ptr::null_mut());
    }

    png_image_error(image, c"png_image_write_: out of memory".as_ptr())
}

/* Write png_uint_16 input to a 16-bit PNG. */
unsafe extern "C" fn png_write_image_16bit(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_write_control;
    let image = (*display).image;
    let png_ptr = (*(*image).opaque).png_ptr;

    let mut input_row = (*display).first_row as png_const_uint_16p;
    let mut output_row = (*display).local_row as png_uint_16p;
    let row_end: png_uint_16p;
    let channels: c_uint = if ((*image).format & PNG_FORMAT_FLAG_COLOR) != 0 {
        3
    } else {
        1
    };
    let mut aindex: c_int = 0;
    let mut y: png_uint_32 = (*image).height;

    if ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0 {
        if ((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0 {
            aindex = -1;
            input_row = input_row.offset(1); /* To point to the first component */
            output_row = output_row.offset(1);
        } else {
            aindex = channels as c_int;
        }
    } else {
        png_error(png_ptr, c"png_write_image: internal call error".as_ptr());
    }

    /* Work out the output row end and count over this. */
    row_end = output_row.offset(((*image).width * (channels + 1)) as isize);

    while y > 0 {
        let mut in_ptr = input_row;
        let mut out_ptr = output_row;

        while out_ptr < row_end {
            let alpha: png_uint_16 = *in_ptr.offset(aindex as isize);
            let mut reciprocal: png_uint_32 = 0;
            let mut c: c_int;

            *out_ptr.offset(aindex as isize) = alpha;

            /* Calculate a reciprocal. */
            if alpha > 0 && alpha < 65535 {
                reciprocal = ((0xffffu32 << 15) + ((alpha as u32) >> 1)) / (alpha as u32);
            }

            c = channels as c_int;
            loop {
                let mut component: png_uint_16 = *in_ptr;
                in_ptr = in_ptr.offset(1);

                if component >= alpha {
                    component = 65535;
                } else if component > 0 && alpha < 65535 {
                    let mut calc: png_uint_32 = (component as u32).wrapping_mul(reciprocal);
                    calc = calc.wrapping_add(16384); /* round to nearest */
                    component = (calc >> 15) as png_uint_16;
                }

                *out_ptr = component;
                out_ptr = out_ptr.offset(1);

                c -= 1;
                if !(c > 0) {
                    break;
                }
            }

            /* Skip to next component (skip the intervening alpha channel) */
            in_ptr = in_ptr.offset(1);
            out_ptr = out_ptr.offset(1);
        }

        png_write_row(png_ptr, (*display).local_row as png_const_bytep);
        input_row = input_row.offset((*display).row_step / 2);
        y -= 1;
    }

    1
}

/* Reverse pre-multiplication, producing sRGB 8-bit output. */
unsafe fn png_unpremultiply(
    mut component: png_uint_32,
    alpha: png_uint_32,
    reciprocal: png_uint_32,
) -> png_byte {
    if component >= alpha || alpha < 128 {
        255
    } else if component > 0 {
        if alpha < 65407 {
            component = component.wrapping_mul(reciprocal);
            component = component.wrapping_add(64); /* round to nearest */
            component >>= 7;
        } else {
            component = component.wrapping_mul(255);
        }

        /* Convert the component to sRGB. */
        png_srgb_from_linear(component)
    } else {
        0
    }
}

unsafe extern "C" fn png_write_image_8bit(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_write_control;
    let image = (*display).image;
    let png_ptr = (*(*image).opaque).png_ptr;

    let mut input_row = (*display).first_row as png_const_uint_16p;
    let mut output_row = (*display).local_row as png_bytep;
    let mut y: png_uint_32 = (*image).height;
    let channels: c_uint = if ((*image).format & PNG_FORMAT_FLAG_COLOR) != 0 {
        3
    } else {
        1
    };

    if ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0 {
        let row_end: png_bytep;
        let aindex: c_int;

        if ((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0 {
            aindex = -1;
            input_row = input_row.offset(1); /* To point to the first component */
            output_row = output_row.offset(1);
        } else {
            aindex = channels as c_int;
        }

        /* Use row_end in place of a loop counter: */
        row_end = output_row.offset(((*image).width * (channels + 1)) as isize);

        while y > 0 {
            let mut in_ptr = input_row;
            let mut out_ptr = output_row;

            while out_ptr < row_end {
                let alpha: png_uint_16 = *in_ptr.offset(aindex as isize);
                let alphabyte: png_byte = png_div257(alpha as u32) as png_byte;
                let mut reciprocal: png_uint_32 = 0;
                let mut c: c_int;

                /* Scale and write the alpha channel. */
                *out_ptr.offset(aindex as isize) = alphabyte;

                if alphabyte > 0 && alphabyte < 255 {
                    reciprocal =
                        (((0xffffu32 * 0xff) << 7) + ((alpha as u32) >> 1)) / (alpha as u32);
                }

                c = channels as c_int;
                loop {
                    *out_ptr = png_unpremultiply(*in_ptr as png_uint_32, alpha as png_uint_32, reciprocal);
                    out_ptr = out_ptr.offset(1);
                    in_ptr = in_ptr.offset(1);
                    c -= 1;
                    if !(c > 0) {
                        break;
                    }
                }

                /* Skip to next component (skip the intervening alpha channel) */
                in_ptr = in_ptr.offset(1);
                out_ptr = out_ptr.offset(1);
            }

            png_write_row(png_ptr, (*display).local_row as png_const_bytep);
            input_row = input_row.offset((*display).row_step / 2);
            y -= 1;
        }
    } else {
        /* No alpha channel. */
        let row_end: png_bytep = output_row.offset(((*image).width * channels) as isize);

        while y > 0 {
            let mut in_ptr = input_row;
            let mut out_ptr = output_row;

            while out_ptr < row_end {
                let mut component: png_uint_32 = *in_ptr as png_uint_32;
                in_ptr = in_ptr.offset(1);

                component = component.wrapping_mul(255);
                *out_ptr = png_srgb_from_linear(component);
                out_ptr = out_ptr.offset(1);
            }

            png_write_row(png_ptr, output_row);
            input_row = input_row.offset((*display).row_step / 2);
            y -= 1;
        }
    }

    1
}

unsafe fn png_image_set_PLTE(display: *mut png_image_write_control) {
    let image = (*display).image;
    let cmap = (*display).colormap;
    let entries: c_int = if (*image).colormap_entries > 256 {
        256
    } else {
        (*image).colormap_entries as c_int
    };

    /* NOTE: the caller must check for cmap != NULL and entries != 0 */
    let format = (*image).format;
    let channels: c_uint = png_image_sample_channels(format);

    let afirst: c_int = ((format & PNG_FORMAT_FLAG_AFIRST) != 0
        && (format & PNG_FORMAT_FLAG_ALPHA) != 0) as c_int;

    let bgr: c_int = if (format & PNG_FORMAT_FLAG_BGR) != 0 { 2 } else { 0 };

    let mut num_trans: c_int;
    let mut palette = [png_color { red: 0, green: 0, blue: 0 }; 256];
    let mut tRNS = [0u8; 256];

    memset(tRNS.as_mut_ptr() as *mut c_void, 255, core::mem::size_of::<[png_byte; 256]>());
    memset(
        palette.as_mut_ptr() as *mut c_void,
        0,
        core::mem::size_of::<[png_color; 256]>(),
    );

    let mut i: c_int = 0;
    num_trans = 0;
    while i < entries {
        /* This gets automatically converted to sRGB with reversal of the
         * pre-multiplication if the color-map has an alpha channel.
         */
        if (format & PNG_FORMAT_FLAG_LINEAR) != 0 {
            let entry = (cmap as png_const_uint_16p).offset((i as c_uint * channels) as isize);

            if (channels & 1) != 0 {
                /* no alpha */
                if channels >= 3 {
                    /* RGB */
                    palette[i as usize].blue =
                        png_srgb_from_linear(255 * *entry.offset((2 ^ bgr) as isize) as u32);
                    palette[i as usize].green =
                        png_srgb_from_linear(255 * *entry.offset(1) as u32);
                    palette[i as usize].red =
                        png_srgb_from_linear(255 * *entry.offset(bgr as isize) as u32);
                } else {
                    /* Gray */
                    let v = png_srgb_from_linear(255 * *entry as u32);
                    palette[i as usize].blue = v;
                    palette[i as usize].red = v;
                    palette[i as usize].green = v;
                }
            } else {
                /* alpha */
                let alpha: png_uint_16 = *entry.offset(if afirst != 0 {
                    0
                } else {
                    channels as isize - 1
                });
                let alphabyte: png_byte = png_div257(alpha as u32) as png_byte;
                let mut reciprocal: png_uint_32 = 0;

                if alphabyte > 0 && alphabyte < 255 {
                    reciprocal =
                        (((0xffffu32 * 0xff) << 7) + ((alpha as u32) >> 1)) / (alpha as u32);
                }

                tRNS[i as usize] = alphabyte;
                if alphabyte < 255 {
                    num_trans = i + 1;
                }

                if channels >= 3 {
                    /* RGB */
                    palette[i as usize].blue = png_unpremultiply(
                        *entry.offset((afirst + (2 ^ bgr)) as isize) as png_uint_32,
                        alpha as png_uint_32,
                        reciprocal,
                    );
                    palette[i as usize].green = png_unpremultiply(
                        *entry.offset((afirst + 1) as isize) as png_uint_32,
                        alpha as png_uint_32,
                        reciprocal,
                    );
                    palette[i as usize].red = png_unpremultiply(
                        *entry.offset((afirst + bgr) as isize) as png_uint_32,
                        alpha as png_uint_32,
                        reciprocal,
                    );
                } else {
                    /* gray */
                    let v = png_unpremultiply(
                        *entry.offset(afirst as isize) as png_uint_32,
                        alpha as png_uint_32,
                        reciprocal,
                    );
                    palette[i as usize].blue = v;
                    palette[i as usize].red = v;
                    palette[i as usize].green = v;
                }
            }
        } else {
            /* Color-map has sRGB values */
            let entry = (cmap as png_const_bytep).offset((i as c_uint * channels) as isize);

            match channels {
                4 => {
                    tRNS[i as usize] = *entry.offset(if afirst != 0 { 0 } else { 3 });
                    if tRNS[i as usize] < 255 {
                        num_trans = i + 1;
                    }
                    /* FALLTHROUGH */
                    palette[i as usize].blue = *entry.offset((afirst + (2 ^ bgr)) as isize);
                    palette[i as usize].green = *entry.offset((afirst + 1) as isize);
                    palette[i as usize].red = *entry.offset((afirst + bgr) as isize);
                }
                3 => {
                    palette[i as usize].blue = *entry.offset((afirst + (2 ^ bgr)) as isize);
                    palette[i as usize].green = *entry.offset((afirst + 1) as isize);
                    palette[i as usize].red = *entry.offset((afirst + bgr) as isize);
                }
                2 => {
                    tRNS[i as usize] = *entry.offset((1 ^ afirst) as isize);
                    if tRNS[i as usize] < 255 {
                        num_trans = i + 1;
                    }
                    /* FALLTHROUGH */
                    let v = *entry.offset(afirst as isize);
                    palette[i as usize].blue = v;
                    palette[i as usize].red = v;
                    palette[i as usize].green = v;
                }
                1 => {
                    let v = *entry.offset(afirst as isize);
                    palette[i as usize].blue = v;
                    palette[i as usize].red = v;
                    palette[i as usize].green = v;
                }
                _ => {}
            }
        }
        i += 1;
    }

    png_set_PLTE(
        (*(*image).opaque).png_ptr,
        (*(*image).opaque).info_ptr,
        palette.as_ptr(),
        entries,
    );

    if num_trans > 0 {
        png_set_tRNS(
            (*(*image).opaque).png_ptr,
            (*(*image).opaque).info_ptr,
            tRNS.as_ptr(),
            num_trans,
            ptr::null(),
        );
    }

    (*image).colormap_entries = entries as png_uint_32;
}

unsafe extern "C" fn png_image_write_main(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_write_control;
    let image = (*display).image;
    let png_ptr = (*(*image).opaque).png_ptr;
    let info_ptr = (*(*image).opaque).info_ptr;
    let mut format = (*image).format;

    /* The following four ints are actually booleans */
    let colormap: c_int = (format & PNG_FORMAT_FLAG_COLORMAP) as c_int;
    let linear: c_int = (colormap == 0 && (format & PNG_FORMAT_FLAG_LINEAR) != 0) as c_int; /* input */
    let alpha: c_int = (colormap == 0 && (format & PNG_FORMAT_FLAG_ALPHA) != 0) as c_int;
    let write_16bit: c_int = (linear != 0 && (*display).convert_to_8bit == 0) as c_int;

    /* Make sure we error out on any bad situation */
    png_set_benign_errors(png_ptr, 0 /*error*/);

    /* Default the 'row_stride' parameter if required, also check limits. */
    {
        let channels: c_uint = png_image_pixel_channels((*image).format);

        if (*image).width <= 0x7fffffffu32 / channels {
            /* no overflow */
            let check: png_uint_32;
            let png_row_stride: png_uint_32 = (*image).width.wrapping_mul(channels);

            if (*display).row_stride == 0 {
                (*display).row_stride = png_row_stride as png_int_32; /*SAFE*/
            }

            if (*display).row_stride < 0 {
                check = ((*display).row_stride as png_uint_32).wrapping_neg();
            } else {
                check = (*display).row_stride as png_uint_32;
            }

            if check >= png_row_stride {
                /* Now check for overflow of the image buffer calculation. */
                if (*image).height > 0xffffffffu32 / png_row_stride {
                    png_error((*(*image).opaque).png_ptr, c"memory image too large".as_ptr());
                }
            } else {
                png_error(
                    (*(*image).opaque).png_ptr,
                    c"supplied row stride too small".as_ptr(),
                );
            }
        } else {
            png_error(
                (*(*image).opaque).png_ptr,
                c"image row stride too large".as_ptr(),
            );
        }
    }

    /* Set the required transforms then write the rows in the correct order. */
    if (format & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        if !(*display).colormap.is_null() && (*image).colormap_entries > 0 {
            let entries = (*image).colormap_entries;

            png_set_IHDR(
                png_ptr,
                info_ptr,
                (*image).width,
                (*image).height,
                if entries > 16 {
                    8
                } else if entries > 4 {
                    4
                } else if entries > 2 {
                    2
                } else {
                    1
                },
                PNG_COLOR_TYPE_PALETTE,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );

            png_image_set_PLTE(display);
        } else {
            png_error(
                (*(*image).opaque).png_ptr,
                c"no color-map for color-mapped image".as_ptr(),
            );
        }
    } else {
        png_set_IHDR(
            png_ptr,
            info_ptr,
            (*image).width,
            (*image).height,
            if write_16bit != 0 { 16 } else { 8 },
            (if (format & PNG_FORMAT_FLAG_COLOR) != 0 {
                PNG_COLOR_MASK_COLOR
            } else {
                0
            }) + (if (format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                PNG_COLOR_MASK_ALPHA
            } else {
                0
            }),
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
    }

    /* Counter-intuitively the data transformations must be called *after*
     * png_write_info, not before as in the read code.
     */
    if write_16bit != 0 {
        /* The gamma here is 1.0 (linear) and the cHRM chunk matches sRGB. */
        png_set_gAMA_fixed(png_ptr, info_ptr, PNG_GAMMA_LINEAR);

        if ((*image).flags & PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB) == 0 {
            png_set_cHRM_fixed(
                png_ptr, info_ptr, /* white */ 31270, 32900, /* red */ 64000, 33000,
                /* green */ 30000, 60000, /* blue */ 15000, 6000,
            );
        }
    } else if ((*image).flags & PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB) == 0 {
        png_set_sRGB(png_ptr, info_ptr, PNG_sRGB_INTENT_PERCEPTUAL);
    } else {
        png_set_gAMA_fixed(png_ptr, info_ptr, PNG_GAMMA_sRGB_INVERSE);
    }

    /* Write the file header. */
    png_write_info(png_ptr, info_ptr);

    /* Now set up the data transformations (*after* the header is written). */
    if write_16bit != 0 {
        let le: png_uint_16 = 0x0001;

        if *(&le as *const png_uint_16 as *const png_byte) != 0 {
            png_set_swap(png_ptr);
        }
    }

    if (format & PNG_FORMAT_FLAG_BGR) != 0 {
        if colormap == 0 && (format & PNG_FORMAT_FLAG_COLOR) != 0 {
            png_set_bgr(png_ptr);
        }
        format &= !PNG_FORMAT_FLAG_BGR;
    }

    if (format & PNG_FORMAT_FLAG_AFIRST) != 0 {
        if colormap == 0 && (format & PNG_FORMAT_FLAG_ALPHA) != 0 {
            png_set_swap_alpha(png_ptr);
        }
        format &= !PNG_FORMAT_FLAG_AFIRST;
    }

    /* If there are 16 or fewer color-map entries we wrote a lower bit depth
     * above, but the application data is still byte packed.
     */
    if colormap != 0 && (*image).colormap_entries <= 16 {
        png_set_packing(png_ptr);
    }

    /* That should have handled all (both) the transforms. */
    if (format
        & !((PNG_FORMAT_FLAG_COLOR
            | PNG_FORMAT_FLAG_LINEAR
            | PNG_FORMAT_FLAG_ALPHA
            | PNG_FORMAT_FLAG_COLORMAP) as png_uint_32))
        != 0
    {
        png_error(png_ptr, c"png_write_image: unsupported transformation".as_ptr());
    }

    {
        let mut row = (*display).buffer as png_const_bytep;
        let mut row_step: isize = (*display).row_stride as isize;

        if linear != 0 {
            row_step *= 2;
        }

        if row_step < 0 {
            row = row.offset(((*image).height - 1) as isize * (-row_step));
        }

        (*display).first_row = row as png_const_voidp;
        (*display).row_step = row_step;
    }

    /* Apply 'fast' options if the flag is set. */
    if ((*image).flags & PNG_IMAGE_FLAG_FAST) != 0 {
        png_set_filter(png_ptr, PNG_FILTER_TYPE_BASE, PNG_NO_FILTERS);
        png_set_compression_level(png_ptr, 3);
    }

    /* Check for the cases that currently require a pre-transform on the row. */
    if linear != 0 && (alpha != 0 || (*display).convert_to_8bit != 0) {
        let row = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr)) as png_bytep;
        let result: c_int;

        (*display).local_row = row as png_voidp;
        if write_16bit != 0 {
            result = png_safe_execute(image, Some(png_write_image_16bit), display as png_voidp);
        } else {
            result = png_safe_execute(image, Some(png_write_image_8bit), display as png_voidp);
        }
        (*display).local_row = ptr::null_mut();

        png_free(png_ptr, row as png_voidp);

        /* Skip the 'write_end' on error: */
        if result == 0 {
            return 0;
        }
    } else {
        let mut row = (*display).first_row as png_const_bytep;
        let row_step: isize = (*display).row_step;
        let mut y: png_uint_32 = (*image).height;

        while y > 0 {
            png_write_row(png_ptr, row);
            row = row.offset(row_step);
            y -= 1;
        }
    }

    png_write_end(png_ptr, info_ptr);
    1
}

unsafe extern "C" fn image_memory_write(png_ptr: png_structp, data: png_bytep, size: size_t) {
    let display = (*png_ptr).io_ptr as *mut png_image_write_control;
    let ob: png_alloc_size_t = (*display).output_bytes;

    /* Check for overflow; this should never happen: */
    if size <= (png_alloc_size_t::MAX) - ob {
        /* I don't think libpng ever does this, but just in case: */
        if size > 0 {
            if (*display).memory_bytes >= ob + size {
                /* writing */
                memcpy(
                    (*display).memory.add(ob) as *mut c_void,
                    data as *const c_void,
                    size,
                );
            }

            /* Always update the size: */
            (*display).output_bytes = ob + size;
        }
    } else {
        png_error(png_ptr, c"png_image_write_to_memory: PNG too big".as_ptr());
    }
}

unsafe extern "C" fn image_memory_flush(png_ptr: png_structp) {
    let _ = png_ptr;
}

unsafe extern "C" fn png_image_write_memory(argument: png_voidp) -> c_int {
    let display = argument as *mut png_image_write_control;

    /* The rest of the memory-specific init and write_main in an error protected
     * environment.
     */
    png_set_write_fn(
        (*(*(*display).image).opaque).png_ptr,
        display as png_voidp, /*io_ptr*/
        Some(image_memory_write),
        Some(image_memory_flush),
    );

    png_image_write_main(display as png_voidp)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_write_to_memory(
    image: png_imagep,
    memory: *mut c_void,
    memory_bytes: *mut png_alloc_size_t,
    convert_to_8bit: c_int,
    buffer: *const c_void,
    row_stride: png_int_32,
    colormap: *const c_void,
) -> c_int {
    /* Write the image to the given buffer, or count the bytes if it is NULL */
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !memory_bytes.is_null() && !buffer.is_null() {
            /* This is to give the caller an easier error detection in the NULL
             * case and guard against uninitialized variable problems:
             */
            if memory.is_null() {
                *memory_bytes = 0;
            }

            if png_image_write_init(image) != 0 {
                let mut display: png_image_write_control = core::mem::zeroed();
                let mut result: c_int;

                display.image = image;
                display.buffer = buffer;
                display.row_stride = row_stride;
                display.colormap = colormap;
                display.convert_to_8bit = convert_to_8bit;
                display.memory = memory as png_bytep;
                display.memory_bytes = *memory_bytes;
                display.output_bytes = 0;

                result = png_safe_execute(
                    image,
                    Some(png_image_write_memory),
                    &mut display as *mut png_image_write_control as png_voidp,
                );
                png_image_free(image);

                /* write_memory returns true even if we ran out of buffer. */
                if result != 0 {
                    /* On out-of-buffer this function returns '0' but still
                     * updates memory_bytes:
                     */
                    if !memory.is_null() && display.output_bytes > *memory_bytes {
                        result = 0;
                    }

                    *memory_bytes = display.output_bytes;
                }

                result
            } else {
                0
            }
        } else {
            png_image_error(image, c"png_image_write_to_memory: invalid argument".as_ptr())
        }
    } else if !image.is_null() {
        png_image_error(
            image,
            c"png_image_write_to_memory: incorrect PNG_IMAGE_VERSION".as_ptr(),
        )
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_write_to_stdio(
    image: png_imagep,
    file: *mut FILE,
    convert_to_8bit: c_int,
    buffer: *const c_void,
    row_stride: png_int_32,
    colormap: *const c_void,
) -> c_int {
    /* Write the image to the given FILE object. */
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !file.is_null() && !buffer.is_null() {
            if png_image_write_init(image) != 0 {
                let mut display: png_image_write_control = core::mem::zeroed();
                let result: c_int;

                /* This is slightly evil, but png_init_io doesn't do anything
                 * other than this.
                 */
                (*(*(*image).opaque).png_ptr).io_ptr = file as png_voidp;

                display.image = image;
                display.buffer = buffer;
                display.row_stride = row_stride;
                display.colormap = colormap;
                display.convert_to_8bit = convert_to_8bit;

                result = png_safe_execute(
                    image,
                    Some(png_image_write_main),
                    &mut display as *mut png_image_write_control as png_voidp,
                );
                png_image_free(image);
                result
            } else {
                0
            }
        } else {
            png_image_error(image, c"png_image_write_to_stdio: invalid argument".as_ptr())
        }
    } else if !image.is_null() {
        png_image_error(
            image,
            c"png_image_write_to_stdio: incorrect PNG_IMAGE_VERSION".as_ptr(),
        )
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_write_to_file(
    image: png_imagep,
    file_name: *const c_char,
    convert_to_8bit: c_int,
    buffer: *const c_void,
    row_stride: png_int_32,
    colormap: *const c_void,
) -> c_int {
    /* Write the image to the named file. */
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !file_name.is_null() && !buffer.is_null() {
            let fp = fopen(file_name, c"wb".as_ptr());

            if !fp.is_null() {
                if png_image_write_to_stdio(image, fp, convert_to_8bit, buffer, row_stride, colormap)
                    != 0
                {
                    let error: c_int; /* from fflush/fclose */

                    /* Make sure the file is flushed correctly. */
                    if fflush(fp) == 0 && ferror(fp) == 0 {
                        if fclose(fp) == 0 {
                            return 1;
                        }

                        error = *__errno_location(); /* from fclose */
                    } else {
                        error = *__errno_location(); /* from fflush or ferror */
                        fclose(fp);
                    }

                    remove(file_name);
                    /* The image has already been cleaned up. */
                    png_image_error(image, strerror(error))
                } else {
                    /* Clean up: just the opened file. */
                    fclose(fp);
                    remove(file_name);
                    0
                }
            } else {
                png_image_error(image, strerror(*__errno_location()))
            }
        } else {
            png_image_error(image, c"png_image_write_to_file: invalid argument".as_ptr())
        }
    } else if !image.is_null() {
        png_image_error(
            image,
            c"png_image_write_to_file: incorrect PNG_IMAGE_VERSION".as_ptr(),
        )
    } else {
        0
    }
}
