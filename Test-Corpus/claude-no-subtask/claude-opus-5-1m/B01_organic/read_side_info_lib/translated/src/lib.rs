#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::ffi::c_int;

#[repr(C)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

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

unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    let bs = unsafe { &mut *bs };
    let s: c_int = bs.pos & 7;
    let mut shl: c_int = n + s;
    let mut p = unsafe { bs.buf.offset((bs.pos >> 3) as isize) };
    bs.pos += n;
    if bs.pos > bs.limit {
        return 0;
    }
    let mut next: u32 = unsafe { (*p as u32) & ((255u32) >> s) };
    p = unsafe { p.add(1) };
    let mut cache: u32 = 0;
    loop {
        shl -= 8;
        if shl <= 0 {
            break;
        }
        cache |= next << shl;
        next = unsafe { *p as u32 };
        p = unsafe { p.add(1) };
    }
    cache | (next >> ((-shl) as u32))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn read_side_info(
    bs: *mut bs_t,
    gr: *mut L3_gr_info_t,
    hdr: *const u8,
) -> c_int {
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
            4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18,
            18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0,
        ],
        [
            8, 8, 8, 8, 8, 8, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24, 24, 24, 28, 28, 28,
            36, 36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14, 18, 18,
            18, 26, 26, 26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18,
            18, 18, 24, 24, 24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18,
            18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14,
            14, 18, 18, 18, 22, 22, 22, 30, 30, 30, 56, 56, 56, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 10, 10, 10, 12, 12, 12, 14, 14,
            14, 16, 16, 16, 20, 20, 20, 26, 26, 26, 66, 66, 66, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20,
            20, 26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0,
        ],
    ];
    static G_SCF_MIXED: [[u8; 40]; 8] = [
        [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24,
            24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0,
        ],
        [
            12, 12, 12, 4, 4, 4, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24, 24, 24, 28, 28,
            28, 36, 36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0,
        ],
        [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14, 18, 18, 18, 26,
            26, 26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0, 0, 0, 0,
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
            4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14,
            18, 18, 18, 22, 22, 22, 30, 30, 30, 56, 56, 56, 0, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 6, 6, 6, 10, 10, 10, 12, 12, 12, 14, 14, 14,
            16, 16, 16, 20, 20, 20, 26, 26, 26, 66, 66, 66, 0, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20,
            26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0, 0,
        ],
    ];

    let mut tables: u32;
    let mut scfsi: u32 = 0;
    let main_data_begin: c_int;
    let mut part_23_sum: c_int = 0;

    let hdr2 = unsafe { *hdr.offset(2) };
    let hdr1 = unsafe { *hdr.offset(1) };
    let hdr3 = unsafe { *hdr.offset(3) };

    let mut sr_idx: c_int = (((hdr2 >> 2) & 3) as c_int)
        + (((hdr1 >> 3) & 1) as c_int + ((hdr1 >> 4) & 1) as c_int) * 3;
    sr_idx -= if sr_idx != 0 { 1 } else { 0 };

    let mut gr_count: c_int = if (hdr3 & 0xC0) == 0xC0 { 1 } else { 2 };

    if (hdr1 & 0x8) != 0 {
        gr_count *= 2;
        main_data_begin = unsafe { get_bits(bs, 9) } as c_int;
        scfsi = unsafe { get_bits(bs, 7 + gr_count) };
    } else {
        main_data_begin = (unsafe { get_bits(bs, 8 + gr_count) } >> (gr_count as u32)) as c_int;
    }

    let mut gr_ptr = gr;

    loop {
        if (hdr3 & 0xC0) == 0xC0 {
            scfsi <<= 4;
        }
        let gr_ref = unsafe { &mut *gr_ptr };
        gr_ref.part_23_length = unsafe { get_bits(bs, 12) } as u16;
        part_23_sum += gr_ref.part_23_length as c_int;
        gr_ref.big_values = unsafe { get_bits(bs, 9) } as u16;
        if gr_ref.big_values > 288 {
            return -1;
        }
        gr_ref.global_gain = unsafe { get_bits(bs, 8) } as u8;
        gr_ref.scalefac_compress =
            unsafe { get_bits(bs, if (hdr1 & 0x8) != 0 { 4 } else { 9 }) } as u16;
        gr_ref.sfbtab = G_SCF_LONG[sr_idx as usize].as_ptr();
        gr_ref.n_long_sfb = 22;
        gr_ref.n_short_sfb = 0;
        if unsafe { get_bits(bs, 1) } != 0 {
            gr_ref.block_type = unsafe { get_bits(bs, 2) } as u8;
            if gr_ref.block_type == 0 {
                return -1;
            }
            gr_ref.mixed_block_flag = unsafe { get_bits(bs, 1) } as u8;
            gr_ref.region_count[0] = 7;
            gr_ref.region_count[1] = 255;
            if gr_ref.block_type == 2 {
                scfsi &= 0x0F0F;
                if gr_ref.mixed_block_flag == 0 {
                    gr_ref.region_count[0] = 8;
                    gr_ref.sfbtab = G_SCF_SHORT[sr_idx as usize].as_ptr();
                    gr_ref.n_long_sfb = 0;
                    gr_ref.n_short_sfb = 39;
                } else {
                    gr_ref.sfbtab = G_SCF_MIXED[sr_idx as usize].as_ptr();
                    gr_ref.n_long_sfb = if (hdr1 & 0x8) != 0 { 8 } else { 6 };
                    gr_ref.n_short_sfb = 30;
                }
            }
            tables = unsafe { get_bits(bs, 10) };
            tables <<= 5;
            gr_ref.subblock_gain[0] = unsafe { get_bits(bs, 3) } as u8;
            gr_ref.subblock_gain[1] = unsafe { get_bits(bs, 3) } as u8;
            gr_ref.subblock_gain[2] = unsafe { get_bits(bs, 3) } as u8;
        } else {
            gr_ref.block_type = 0;
            gr_ref.mixed_block_flag = 0;
            tables = unsafe { get_bits(bs, 15) };
            gr_ref.region_count[0] = unsafe { get_bits(bs, 4) } as u8;
            gr_ref.region_count[1] = unsafe { get_bits(bs, 3) } as u8;
            gr_ref.region_count[2] = 255;
        }
        gr_ref.table_select[0] = (tables >> 10) as u8;
        gr_ref.table_select[1] = ((tables >> 5) & 31) as u8;
        gr_ref.table_select[2] = (tables & 31) as u8;
        gr_ref.preflag = if (hdr1 & 0x8) != 0 {
            (unsafe { get_bits(bs, 1) }) as u8
        } else {
            (gr_ref.scalefac_compress >= 500) as u8
        };
        gr_ref.scalefac_scale = unsafe { get_bits(bs, 1) } as u8;
        gr_ref.count1_table = unsafe { get_bits(bs, 1) } as u8;
        gr_ref.scfsi = ((scfsi >> 12) & 15) as u8;
        scfsi <<= 4;
        gr_ptr = unsafe { gr_ptr.add(1) };
        gr_count -= 1;
        if gr_count == 0 {
            break;
        }
    }

    let bs_ref = unsafe { &*bs };
    if part_23_sum + bs_ref.pos > bs_ref.limit + main_data_begin * 8 {
        return -1;
    }
    main_data_begin
}
