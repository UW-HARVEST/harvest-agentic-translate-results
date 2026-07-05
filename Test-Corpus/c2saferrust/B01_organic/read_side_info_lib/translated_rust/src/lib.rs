





pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct bs_t {
    pub buf: *const uint8_t,
    pub pos: ::core::ffi::c_int,
    pub limit: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct L3_gr_info_t {
    pub sfbtab: *const uint8_t,
    pub part_23_length: uint16_t,
    pub big_values: uint16_t,
    pub scalefac_compress: uint16_t,
    pub global_gain: uint8_t,
    pub block_type: uint8_t,
    pub mixed_block_flag: uint8_t,
    pub n_long_sfb: uint8_t,
    pub n_short_sfb: uint8_t,
    pub table_select: [uint8_t; 3],
    pub region_count: [uint8_t; 3],
    pub subblock_gain: [uint8_t; 3],
    pub preflag: uint8_t,
    pub scalefac_scale: uint8_t,
    pub count1_table: uint8_t,
    pub scfsi: uint8_t,
}
fn get_bits(bs: *mut bs_t, n: ::core::ffi::c_int) -> u32 {
    unsafe {
        let s = ((*bs).pos & 7) as u32;
        let mut shl = (n as u32).wrapping_add(s) as ::core::ffi::c_int;
        let mut p = (*bs).buf.wrapping_add(((*bs).pos >> 3) as usize);

        (*bs).pos += n;
        if (*bs).pos > (*bs).limit {
            return 0;
        }

        let mut cache: u32 = 0;
        let mut next: u32 = ((*p as ::core::ffi::c_int & (255 >> s) as ::core::ffi::c_int) as u32);
        p = p.wrapping_add(1);

        loop {
            shl -= 8;
            if shl <= 0 {
                break;
            }
            cache |= next << shl;
            next = *p as u32;
            p = p.wrapping_add(1);
        }

        cache | (next >> (-shl))
    }
}

#[no_mangle]
pub unsafe extern "C" fn read_side_info(
    mut bs: *mut bs_t,
    mut gr: *mut L3_gr_info_t,
    mut hdr: *const uint8_t,
) -> ::core::ffi::c_int {
    static g_scf_long: [[u8; 23]; 8] = [
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

    static g_scf_short: [[u8; 40]; 8] = [
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14,
        18, 18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0,
    ],
    [
        8, 8, 8, 8, 8, 8, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24, 24, 24, 28, 28,
        28, 36, 36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14, 18,
        18, 18, 26, 26, 26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14,
        18, 18, 18, 24, 24, 24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14,
        18, 18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14,
        14, 14, 18, 18, 18, 22, 22, 22, 30, 30, 30, 56, 56, 56, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 10, 10, 10, 12, 12, 12, 14,
        14, 14, 16, 16, 16, 20, 20, 20, 26, 26, 26, 66, 66, 66, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20,
        20, 20, 26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0,
    ],
];

    static g_scf_mixed: [[u8; 40]; 8] = [
    [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14,
        14, 18, 18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0,
        0, 0,
    ],
    [
        12, 12, 12, 4, 4, 4, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24,
        24, 24, 28, 28, 28, 36, 36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26,
        0,
    ],
    [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14,
        18, 18, 18, 26, 26, 26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0, 0, 0, 0,
    ],
    [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14,
        14, 18, 18, 18, 24, 24, 24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0, 0,
        0, 0,
    ],
    [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14,
        14, 18, 18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0,
        0, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12,
        12, 14, 14, 14, 18, 18, 18, 22, 22, 22, 30, 30, 30, 56, 56, 56, 0, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 6, 6, 6, 10, 10, 10, 12, 12,
        12, 14, 14, 14, 16, 16, 16, 20, 20, 20, 26, 26, 26, 66, 66, 66, 0, 0,
    ],
    [
        4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 8, 8, 8, 12, 12, 12, 16, 16,
        16, 20, 20, 20, 26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0, 0,
    ],
];

    let mut tables: u32 = 0;
let mut scfsi: u32 = 0;
let mut main_data_begin: i32 = 0;
let mut part_23_sum: i32 = 0;

let hdr1 = *hdr.add(1) as i32;
let hdr2 = *hdr.add(2) as i32;
let hdr3 = *hdr.add(3) as i32;

let mut sr_idx: i32 = ((hdr2 >> 2) & 3) + ((((hdr1 >> 3) & 1) + ((hdr1 >> 4) & 1)) * 3);
sr_idx -= (sr_idx != 0) as i32;

let mut gr_count: i32 = if (hdr3 & 0xc0) == 0xc0 { 1 } else { 2 };

if (hdr1 & 0x8) != 0 {
    gr_count *= 2;
    main_data_begin = get_bits(bs, 9) as i32;
    scfsi = get_bits(bs, 7 + gr_count);
} else {
    main_data_begin = (get_bits(bs, 8 + gr_count) >> gr_count) as i32;
}

while gr_count != 0 {
    if (hdr3 & 0xc0) == 0xc0 {
        scfsi <<= 4;
    }

    (*gr).part_23_length = get_bits(bs, 12) as uint16_t;
    part_23_sum += (*gr).part_23_length as i32;

    (*gr).big_values = get_bits(bs, 9) as uint16_t;
    if (*gr).big_values as i32 > 288 {
        return -1;
    }

    (*gr).global_gain = get_bits(bs, 8) as uint8_t;
    (*gr).scalefac_compress = get_bits(bs, if (hdr1 & 0x8) != 0 { 4 } else { 9 }) as uint16_t;

    (*gr).sfbtab = &raw const *(&raw const g_scf_long as *const [uint8_t; 23]).add(sr_idx as usize)
        as *const uint8_t;
    (*gr).n_long_sfb = 22 as uint8_t;
    (*gr).n_short_sfb = 0 as uint8_t;

    if get_bits(bs, 1) != 0 {
        (*gr).block_type = get_bits(bs, 2) as uint8_t;
        if (*gr).block_type == 0 {
            return -1;
        }

        (*gr).mixed_block_flag = get_bits(bs, 1) as uint8_t;
        (*gr).region_count[0] = 7 as uint8_t;
        (*gr).region_count[1] = 255 as uint8_t;

        if (*gr).block_type as i32 == 2 {
            scfsi &= 0xf0f;
            if (*gr).mixed_block_flag == 0 {
                (*gr).region_count[0] = 8 as uint8_t;
                (*gr).sfbtab =
                    &raw const *(&raw const g_scf_short as *const [uint8_t; 40]).add(sr_idx as usize)
                        as *const uint8_t;
                (*gr).n_long_sfb = 0 as uint8_t;
                (*gr).n_short_sfb = 39 as uint8_t;
            } else {
                (*gr).sfbtab =
                    &raw const *(&raw const g_scf_mixed as *const [uint8_t; 40]).add(sr_idx as usize)
                        as *const uint8_t;
                (*gr).n_long_sfb = if (hdr1 & 0x8) != 0 { 8 } else { 6 } as uint8_t;
                (*gr).n_short_sfb = 30 as uint8_t;
            }
        }

        tables = get_bits(bs, 10);
        tables <<= 5;
        (*gr).subblock_gain[0] = get_bits(bs, 3) as uint8_t;
        (*gr).subblock_gain[1] = get_bits(bs, 3) as uint8_t;
        (*gr).subblock_gain[2] = get_bits(bs, 3) as uint8_t;
    } else {
        (*gr).block_type = 0 as uint8_t;
        (*gr).mixed_block_flag = 0 as uint8_t;
        tables = get_bits(bs, 15);
        (*gr).region_count[0] = get_bits(bs, 4) as uint8_t;
        (*gr).region_count[1] = get_bits(bs, 3) as uint8_t;
        (*gr).region_count[2] = 255 as uint8_t;
    }

    (*gr).table_select[0] = (tables >> 10) as uint8_t;
    (*gr).table_select[1] = ((tables >> 5) & 31) as uint8_t;
    (*gr).table_select[2] = (tables & 31) as uint8_t;

    (*gr).preflag = if (hdr1 & 0x8) != 0 {
        get_bits(bs, 1) as uint8_t
    } else {
        ((*gr).scalefac_compress as i32 >= 500) as u8
    };

    (*gr).scalefac_scale = get_bits(bs, 1) as uint8_t;
    (*gr).count1_table = get_bits(bs, 1) as uint8_t;
    (*gr).scfsi = ((scfsi >> 12) & 15) as uint8_t;
    scfsi <<= 4;

    gr = gr.add(1);
    gr_count -= 1;
}

if part_23_sum + (*bs).pos > (*bs).limit + main_data_begin * 8 {
    return -1;
}

return main_data_begin;


    0
}

