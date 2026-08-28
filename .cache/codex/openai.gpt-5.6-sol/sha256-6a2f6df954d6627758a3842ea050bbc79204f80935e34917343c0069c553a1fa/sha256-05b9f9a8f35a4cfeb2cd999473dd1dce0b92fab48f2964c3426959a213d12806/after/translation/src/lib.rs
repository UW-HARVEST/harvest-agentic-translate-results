use std::ffi::c_int;

#[repr(C)]
pub struct Bs {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

#[repr(C)]
pub struct L3GrInfo {
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

static G_SCF_MIXED: [[u8; 40]; 8] = [
    [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24, 24,
        24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0,
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
        6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24, 24,
        24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0, 0, 0, 0,
    ],
    [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24, 24,
        24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0,
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

unsafe fn get_bits(bs: *mut Bs, n: c_int) -> u32 {
    let s = unsafe { (*bs).pos & 7 } as u32;
    let mut shl = n + s as c_int;
    let mut p = unsafe { (*bs).buf.wrapping_offset(((*bs).pos >> 3) as isize) };

    unsafe {
        (*bs).pos = (*bs).pos.wrapping_add(n);
        if (*bs).pos > (*bs).limit {
            return 0;
        }
    }

    let mut next = unsafe { *p } as u32 & (255_u32 >> s);
    p = p.wrapping_add(1);
    let mut cache = 0_u32;
    loop {
        shl -= 8;
        if shl <= 0 {
            break;
        }
        cache |= next << shl;
        next = unsafe { *p } as u32;
        p = p.wrapping_add(1);
    }
    cache | (next >> (-shl))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn read_side_info(
    bs: *mut Bs,
    mut gr: *mut L3GrInfo,
    hdr: *const u8,
) -> c_int {
    let hdr1 = unsafe { *hdr.add(1) };
    let hdr2 = unsafe { *hdr.add(2) };
    let hdr3 = unsafe { *hdr.add(3) };

    let mut sr_idx = (((hdr2 >> 2) & 3) as c_int
        + (((hdr1 >> 3) & 1) + ((hdr1 >> 4) & 1)) as c_int * 3) as usize;
    sr_idx -= usize::from(sr_idx != 0);

    let mut gr_count: c_int = if hdr3 & 0xc0 == 0xc0 { 1 } else { 2 };
    let main_data_begin;
    let mut scfsi = 0_u32;
    if hdr1 & 0x08 != 0 {
        gr_count *= 2;
        main_data_begin = unsafe { get_bits(bs, 9) } as c_int;
        scfsi = unsafe { get_bits(bs, 7 + gr_count) };
    } else {
        main_data_begin = (unsafe { get_bits(bs, 8 + gr_count) } >> gr_count) as c_int;
    }

    let mut part_23_sum: c_int = 0;
    loop {
        if hdr3 & 0xc0 == 0xc0 {
            scfsi <<= 4;
        }

        unsafe {
            (*gr).part_23_length = get_bits(bs, 12) as u16;
            part_23_sum = part_23_sum.wrapping_add((*gr).part_23_length as c_int);
            (*gr).big_values = get_bits(bs, 9) as u16;
            if (*gr).big_values > 288 {
                return -1;
            }
            (*gr).global_gain = get_bits(bs, 8) as u8;
            (*gr).scalefac_compress = get_bits(bs, if hdr1 & 0x08 != 0 { 4 } else { 9 }) as u16;
            (*gr).sfbtab = G_SCF_LONG[sr_idx].as_ptr();
            (*gr).n_long_sfb = 22;
            (*gr).n_short_sfb = 0;

            let tables;
            if get_bits(bs, 1) != 0 {
                (*gr).block_type = get_bits(bs, 2) as u8;
                if (*gr).block_type == 0 {
                    return -1;
                }
                (*gr).mixed_block_flag = get_bits(bs, 1) as u8;
                (*gr).region_count[0] = 7;
                (*gr).region_count[1] = 255;
                if (*gr).block_type == 2 {
                    scfsi &= 0x0f0f;
                    if (*gr).mixed_block_flag == 0 {
                        (*gr).region_count[0] = 8;
                        (*gr).sfbtab = G_SCF_SHORT[sr_idx].as_ptr();
                        (*gr).n_long_sfb = 0;
                        (*gr).n_short_sfb = 39;
                    } else {
                        (*gr).sfbtab = G_SCF_MIXED[sr_idx].as_ptr();
                        (*gr).n_long_sfb = if hdr1 & 0x08 != 0 { 8 } else { 6 };
                        (*gr).n_short_sfb = 30;
                    }
                }
                tables = get_bits(bs, 10) << 5;
                (*gr).subblock_gain[0] = get_bits(bs, 3) as u8;
                (*gr).subblock_gain[1] = get_bits(bs, 3) as u8;
                (*gr).subblock_gain[2] = get_bits(bs, 3) as u8;
            } else {
                (*gr).block_type = 0;
                (*gr).mixed_block_flag = 0;
                tables = get_bits(bs, 15);
                (*gr).region_count[0] = get_bits(bs, 4) as u8;
                (*gr).region_count[1] = get_bits(bs, 3) as u8;
                (*gr).region_count[2] = 255;
            }

            (*gr).table_select[0] = (tables >> 10) as u8;
            (*gr).table_select[1] = ((tables >> 5) & 31) as u8;
            (*gr).table_select[2] = (tables & 31) as u8;
            (*gr).preflag = if hdr1 & 0x08 != 0 {
                get_bits(bs, 1) as u8
            } else {
                u8::from((*gr).scalefac_compress >= 500)
            };
            (*gr).scalefac_scale = get_bits(bs, 1) as u8;
            (*gr).count1_table = get_bits(bs, 1) as u8;
            (*gr).scfsi = ((scfsi >> 12) & 15) as u8;
        }

        scfsi <<= 4;
        gr = gr.wrapping_add(1);
        gr_count -= 1;
        if gr_count == 0 {
            break;
        }
    }

    let pos = unsafe { (*bs).pos };
    if part_23_sum.wrapping_add(pos)
        > unsafe { (*bs).limit }.wrapping_add(main_data_begin.wrapping_mul(8))
    {
        return -1;
    }
    main_data_begin
}
