//! Rust translation of `c_src/src/lib.c` (MP3 layer-III side-info reader).
//!
//! The translation is deliberately literal: pointer arithmetic, integer
//! wrapping and out-of-range table indexing all mirror the original C exactly,
//! including behaviour the C standard would call undefined. No bugs are fixed.

#![allow(non_snake_case)]

use std::ffi::c_int;

mod layout_check;

/// Mirrors the C `bs_t` bit-stream cursor.
#[repr(C)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

/// Mirrors the C `L3_gr_info_t` granule descriptor.
#[repr(C)]
pub struct L3_gr_info_t {
    pub sfbtab: *const u8,
    pub part_23_length: u16,
    pub big_values: u16,
    pub scalefac_compress: u16,
    pub global_gain: u8,
    pub block_type: u8,
    pub mixed_block_flag: u8,
    pub n_long_sfb: u8,
    pub n_short_sfb: u8,
    pub table_select: [u8; 3],
    pub region_count: [u8; 3],
    pub subblock_gain: [u8; 3],
    pub preflag: u8,
    pub scalefac_scale: u8,
    pub count1_table: u8,
    pub scfsi: u8,
}

/// Literal translation of the `static uint32_t get_bits(bs_t *, int)` helper.
///
/// # Safety
/// `bs` must be a valid pointer; `bs->buf` is read through raw pointers with
/// the same (possibly out-of-bounds) offsets the C code would use.
unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    let mut next: u32;
    let mut cache: u32 = 0;
    let s: u32 = unsafe { (*bs).pos & 7 } as u32;
    let mut shl: c_int = n.wrapping_add(s as c_int);
    let mut p: *const u8 = unsafe { (*bs).buf.offset(((*bs).pos >> 3) as isize) };

    unsafe {
        (*bs).pos = (*bs).pos.wrapping_add(n);
        if (*bs).pos > (*bs).limit {
            return 0;
        }
    }

    next = unsafe { (*p as u32) & (255u32 >> s) };
    p = unsafe { p.add(1) };

    loop {
        shl = shl.wrapping_sub(8);
        if shl <= 0 {
            break;
        }
        cache |= next.wrapping_shl(shl as u32);
        next = unsafe { *p as u32 };
        p = unsafe { p.add(1) };
    }

    cache | next.wrapping_shr(shl.wrapping_neg() as u32)
}

static G_SCF_LONG: [[u8; 23]; 8] = [
    [
        6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0,
    ],
    [
        12, 12, 12, 12, 12, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 76, 90, 2, 2, 2, 2, 2, 0,
    ],
    [
        6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0,
    ],
    [
        6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 18, 22, 26, 32, 38, 46, 54, 62, 70, 76, 36, 0,
    ],
    [
        6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 6, 6, 8, 8, 10, 12, 16, 20, 24, 28, 34, 42, 50, 54, 76, 158, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 10, 12, 16, 18, 22, 28, 34, 40, 46, 54, 54, 192, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 6, 6, 8, 10, 12, 16, 20, 24, 30, 38, 46, 56, 68, 84, 102, 26, 0,
    ],
];

static G_SCF_SHORT: [[u8; 40]; 8] = [
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18,
        18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0,
    ],
    [
        8, 8, 8, 8, 8, 8, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24, 24, 24, 28, 28, 28, 36,
        36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14, 18, 18, 18,
        26, 26, 26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18,
        18, 24, 24, 24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18,
        18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14,
        18, 18, 18, 22, 22, 22, 30, 30, 30, 56, 56, 56, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 10, 10, 10, 12, 12, 12, 14, 14, 14,
        16, 16, 16, 20, 20, 20, 26, 26, 26, 66, 66, 66, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20,
        26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0,
    ],
];

// The C initialisers for rows 0..=4 and 5..=7 of `g_scf_mixed` are shorter than
// the declared row length of 40, so the trailing elements are zero-filled.
static G_SCF_MIXED: [[u8; 40]; 8] = [
    [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24,
        24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0,
    ],
    [
        12, 12, 12, 4, 4, 4, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24, 24, 24, 28, 28, 28,
        36, 36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0,
    ],
    [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14, 18, 18, 18, 26, 26,
        26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0, 0, 0, 0,
    ],
    [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24,
        24, 24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0, 0, 0, 0,
    ],
    [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24,
        24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18,
        18, 18, 22, 22, 22, 30, 30, 30, 56, 56, 56, 0, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 6, 6, 6, 10, 10, 10, 12, 12, 12, 14, 14, 14, 16,
        16, 16, 20, 20, 20, 26, 26, 26, 66, 66, 66, 0, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 26,
        26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0, 0,
    ],
];

/// `int read_side_info(bs_t *bs, L3_gr_info_t *gr, const uint8_t *hdr)`
///
/// # Safety
/// Same contract as the C function: all three pointers must be valid and `gr`
/// must point to enough granules for the header's granule count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn read_side_info(
    bs: *mut bs_t,
    gr: *mut L3_gr_info_t,
    hdr: *const u8,
) -> c_int {
    let mut gr = gr;

    let mut tables: u32;
    let mut scfsi: u32 = 0;
    let main_data_begin: c_int;
    let mut part_23_sum: c_int = 0;

    let hdr1 = unsafe { *hdr.add(1) } as c_int;
    let hdr2 = unsafe { *hdr.add(2) } as c_int;
    let hdr3 = unsafe { *hdr.add(3) } as c_int;

    let mut sr_idx: c_int = ((hdr2 >> 2) & 3) + (((hdr1 >> 3) & 1) + ((hdr1 >> 4) & 1)) * 3;
    sr_idx -= (sr_idx != 0) as c_int;

    let mut gr_count: c_int = if (hdr3 & 0xC0) == 0xC0 { 1 } else { 2 };

    if (hdr1 & 0x8) != 0 {
        gr_count = gr_count.wrapping_mul(2);
        main_data_begin = unsafe { get_bits(bs, 9) } as c_int;
        scfsi = unsafe { get_bits(bs, 7 + gr_count) };
    } else {
        main_data_begin = (unsafe { get_bits(bs, 8 + gr_count) } >> gr_count) as c_int;
    }

    loop {
        if (hdr3 & 0xC0) == 0xC0 {
            scfsi = scfsi.wrapping_shl(4);
        }

        let part_23_length = unsafe { get_bits(bs, 12) } as u16;
        unsafe { (*gr).part_23_length = part_23_length };
        part_23_sum = part_23_sum.wrapping_add(part_23_length as c_int);

        let big_values = unsafe { get_bits(bs, 9) } as u16;
        unsafe { (*gr).big_values = big_values };
        if big_values > 288 {
            return -1;
        }

        unsafe { (*gr).global_gain = get_bits(bs, 8) as u8 };
        let scalefac_compress =
            unsafe { get_bits(bs, if (hdr1 & 0x8) != 0 { 4 } else { 9 }) } as u16;
        unsafe { (*gr).scalefac_compress = scalefac_compress };

        // Reproduces the C address computation `g_scf_*[sr_idx]` verbatim,
        // including the out-of-range `sr_idx == 8` case.
        unsafe {
            (*gr).sfbtab = (G_SCF_LONG.as_ptr() as *const u8).offset(sr_idx as isize * 23);
            (*gr).n_long_sfb = 22;
            (*gr).n_short_sfb = 0;
        }

        if unsafe { get_bits(bs, 1) } != 0 {
            let block_type = unsafe { get_bits(bs, 2) } as u8;
            unsafe { (*gr).block_type = block_type };
            if block_type == 0 {
                return -1;
            }
            let mixed_block_flag = unsafe { get_bits(bs, 1) } as u8;
            unsafe {
                (*gr).mixed_block_flag = mixed_block_flag;
                (*gr).region_count[0] = 7;
                (*gr).region_count[1] = 255;
            }
            if block_type == 2 {
                scfsi &= 0x0F0F;
                if mixed_block_flag == 0 {
                    unsafe {
                        (*gr).region_count[0] = 8;
                        (*gr).sfbtab =
                            (G_SCF_SHORT.as_ptr() as *const u8).offset(sr_idx as isize * 40);
                        (*gr).n_long_sfb = 0;
                        (*gr).n_short_sfb = 39;
                    }
                } else {
                    unsafe {
                        (*gr).sfbtab =
                            (G_SCF_MIXED.as_ptr() as *const u8).offset(sr_idx as isize * 40);
                        (*gr).n_long_sfb = if (hdr1 & 0x8) != 0 { 8 } else { 6 };
                        (*gr).n_short_sfb = 30;
                    }
                }
            }
            tables = unsafe { get_bits(bs, 10) };
            tables = tables.wrapping_shl(5);
            unsafe {
                (*gr).subblock_gain[0] = get_bits(bs, 3) as u8;
                (*gr).subblock_gain[1] = get_bits(bs, 3) as u8;
                (*gr).subblock_gain[2] = get_bits(bs, 3) as u8;
            }
        } else {
            unsafe {
                (*gr).block_type = 0;
                (*gr).mixed_block_flag = 0;
            }
            tables = unsafe { get_bits(bs, 15) };
            unsafe {
                (*gr).region_count[0] = get_bits(bs, 4) as u8;
                (*gr).region_count[1] = get_bits(bs, 3) as u8;
                (*gr).region_count[2] = 255;
            }
        }

        unsafe {
            (*gr).table_select[0] = (tables >> 10) as u8;
            (*gr).table_select[1] = ((tables >> 5) & 31) as u8;
            (*gr).table_select[2] = (tables & 31) as u8;
            (*gr).preflag = if (hdr1 & 0x8) != 0 {
                get_bits(bs, 1) as u8
            } else {
                ((*gr).scalefac_compress >= 500) as u8
            };
            (*gr).scalefac_scale = get_bits(bs, 1) as u8;
            (*gr).count1_table = get_bits(bs, 1) as u8;
            (*gr).scfsi = ((scfsi >> 12) & 15) as u8;
        }
        scfsi = scfsi.wrapping_shl(4);
        gr = unsafe { gr.add(1) };

        gr_count = gr_count.wrapping_sub(1);
        if gr_count == 0 {
            break;
        }
    }

    let pos = unsafe { (*bs).pos };
    let limit = unsafe { (*bs).limit };
    if part_23_sum.wrapping_add(pos) > limit.wrapping_add(main_data_begin.wrapping_mul(8)) {
        return -1;
    }
    main_data_begin
}
