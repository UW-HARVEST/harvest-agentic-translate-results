use std::os::raw::c_int;

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
    let s = (*bs).pos & 7;
    let mut shl = n + s;
    let mut p = (*bs).buf.offset(((*bs).pos >> 3) as isize);
    (*bs).pos += n;
    if (*bs).pos > (*bs).limit {
        return 0;
    }
    let mut cache: u32 = 0;
    let mut next: u32 = (*p as u32) & (255u32 >> s as u32);
    p = p.offset(1);
    shl -= 8;
    while shl > 0 {
        cache |= next << shl as u32;
        next = *p as u32;
        p = p.offset(1);
        shl -= 8;
    }
    cache | (next >> ((-shl) as u32))
}

static G_SCF_LONG: [[u8; 23]; 8] = [
    [6,6,6,6,6,6,8,10,12,14,16,20,24,28,32,38,46,52,60,68,58,54,0],
    [12,12,12,12,12,12,16,20,24,28,32,40,48,56,64,76,90,2,2,2,2,2,0],
    [6,6,6,6,6,6,8,10,12,14,16,20,24,28,32,38,46,52,60,68,58,54,0],
    [6,6,6,6,6,6,8,10,12,14,16,18,22,26,32,38,46,54,62,70,76,36,0],
    [6,6,6,6,6,6,8,10,12,14,16,20,24,28,32,38,46,52,60,68,58,54,0],
    [4,4,4,4,4,4,6,6,8,8,10,12,16,20,24,28,34,42,50,54,76,158,0],
    [4,4,4,4,4,4,6,6,6,8,10,12,16,18,22,28,34,40,46,54,54,192,0],
    [4,4,4,4,4,4,6,6,8,10,12,16,20,24,30,38,46,56,68,84,102,26,0],
];

static G_SCF_SHORT: [[u8; 40]; 8] = [
    [4,4,4,4,4,4,4,4,4,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,30,30,30,40,40,40,18,18,18,0],
    [8,8,8,8,8,8,8,8,8,12,12,12,16,16,16,20,20,20,24,24,24,28,28,28,36,36,36,2,2,2,2,2,2,2,2,2,26,26,26,0],
    [4,4,4,4,4,4,4,4,4,6,6,6,6,6,6,8,8,8,10,10,10,14,14,14,18,18,18,26,26,26,32,32,32,42,42,42,18,18,18,0],
    [4,4,4,4,4,4,4,4,4,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,32,32,32,44,44,44,12,12,12,0],
    [4,4,4,4,4,4,4,4,4,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,30,30,30,40,40,40,18,18,18,0],
    [4,4,4,4,4,4,4,4,4,4,4,4,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,22,22,22,30,30,30,56,56,56,0],
    [4,4,4,4,4,4,4,4,4,4,4,4,6,6,6,6,6,6,10,10,10,12,12,12,14,14,14,16,16,16,20,20,20,26,26,26,66,66,66,0],
    [4,4,4,4,4,4,4,4,4,4,4,4,6,6,6,8,8,8,12,12,12,16,16,16,20,20,20,26,26,26,34,34,34,42,42,42,12,12,12,0],
];

static G_SCF_MIXED: [[u8; 40]; 8] = [
    [6,6,6,6,6,6,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,30,30,30,40,40,40,18,18,18,0,0,0,0],
    [12,12,12,4,4,4,8,8,8,12,12,12,16,16,16,20,20,20,24,24,24,28,28,28,36,36,36,2,2,2,2,2,2,2,2,2,26,26,26,0],
    [6,6,6,6,6,6,6,6,6,6,6,6,8,8,8,10,10,10,14,14,14,18,18,18,26,26,26,32,32,32,42,42,42,18,18,18,0,0,0,0],
    [6,6,6,6,6,6,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,32,32,32,44,44,44,12,12,12,0,0,0,0],
    [6,6,6,6,6,6,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,30,30,30,40,40,40,18,18,18,0,0,0,0],
    [4,4,4,4,4,4,6,6,4,4,4,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,22,22,22,30,30,30,56,56,56,0,0],
    [4,4,4,4,4,4,6,6,4,4,4,6,6,6,6,6,6,10,10,10,12,12,12,14,14,14,16,16,16,20,20,20,26,26,26,66,66,66,0,0],
    [4,4,4,4,4,4,6,6,4,4,4,6,6,6,8,8,8,12,12,12,16,16,16,20,20,20,26,26,26,34,34,34,42,42,42,12,12,12,0,0],
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn read_side_info(
    bs: *mut bs_t,
    gr: *mut L3_gr_info_t,
    hdr: *const u8,
) -> c_int {
    let mut scfsi: u32;
    let main_data_begin: c_int;
    let mut part_23_sum: c_int = 0;

    let mut sr_idx = (((*hdr.offset(2)) >> 2) & 3) as c_int
        + (((((*hdr.offset(1)) >> 3) & 1) + (((*hdr.offset(1)) >> 4) & 1)) as c_int) * 3;
    sr_idx -= (sr_idx != 0) as c_int;

    let mut gr_count: c_int = if ((*hdr.offset(3)) & 0xC0) == 0xC0 { 1 } else { 2 };

    if ((*hdr.offset(1)) & 0x8) != 0 {
        gr_count *= 2;
        main_data_begin = get_bits(bs, 9) as c_int;
        scfsi = get_bits(bs, 7 + gr_count);
    } else {
        let tmp = get_bits(bs, 8 + gr_count);
        main_data_begin = (tmp >> gr_count as u32) as c_int;
        scfsi = 0;
    }

    let mut gr_ptr = gr;
    loop {
        if ((*hdr.offset(3)) & 0xC0) == 0xC0 {
            scfsi <<= 4;
        }
        (*gr_ptr).part_23_length = get_bits(bs, 12) as u16;
        part_23_sum += (*gr_ptr).part_23_length as c_int;
        (*gr_ptr).big_values = get_bits(bs, 9) as u16;
        if (*gr_ptr).big_values > 288 {
            return -1;
        }
        (*gr_ptr).global_gain = get_bits(bs, 8) as u8;
        (*gr_ptr).scalefac_compress = get_bits(
            bs,
            if ((*hdr.offset(1)) & 0x8) != 0 { 4 } else { 9 },
        ) as u16;
        (*gr_ptr).sfbtab = G_SCF_LONG[sr_idx as usize].as_ptr();
        (*gr_ptr).n_long_sfb = 22;
        (*gr_ptr).n_short_sfb = 0;

        let mut tables: u32;
        if get_bits(bs, 1) != 0 {
            (*gr_ptr).block_type = get_bits(bs, 2) as u8;
            if (*gr_ptr).block_type == 0 {
                return -1;
            }
            (*gr_ptr).mixed_block_flag = get_bits(bs, 1) as u8;
            (*gr_ptr).region_count[0] = 7;
            (*gr_ptr).region_count[1] = 255;
            if (*gr_ptr).block_type == 2 {
                scfsi &= 0x0F0F;
                if (*gr_ptr).mixed_block_flag == 0 {
                    (*gr_ptr).region_count[0] = 8;
                    (*gr_ptr).sfbtab = G_SCF_SHORT[sr_idx as usize].as_ptr();
                    (*gr_ptr).n_long_sfb = 0;
                    (*gr_ptr).n_short_sfb = 39;
                } else {
                    (*gr_ptr).sfbtab = G_SCF_MIXED[sr_idx as usize].as_ptr();
                    (*gr_ptr).n_long_sfb = if ((*hdr.offset(1)) & 0x8) != 0 { 8 } else { 6 };
                    (*gr_ptr).n_short_sfb = 30;
                }
            }
            tables = get_bits(bs, 10);
            tables <<= 5;
            (*gr_ptr).subblock_gain[0] = get_bits(bs, 3) as u8;
            (*gr_ptr).subblock_gain[1] = get_bits(bs, 3) as u8;
            (*gr_ptr).subblock_gain[2] = get_bits(bs, 3) as u8;
        } else {
            (*gr_ptr).block_type = 0;
            (*gr_ptr).mixed_block_flag = 0;
            tables = get_bits(bs, 15);
            (*gr_ptr).region_count[0] = get_bits(bs, 4) as u8;
            (*gr_ptr).region_count[1] = get_bits(bs, 3) as u8;
            (*gr_ptr).region_count[2] = 255;
        }
        (*gr_ptr).table_select[0] = (tables >> 10) as u8;
        (*gr_ptr).table_select[1] = ((tables >> 5) & 31) as u8;
        (*gr_ptr).table_select[2] = (tables & 31) as u8;
        (*gr_ptr).preflag = if ((*hdr.offset(1)) & 0x8) != 0 {
            get_bits(bs, 1) as u8
        } else {
            ((*gr_ptr).scalefac_compress >= 500) as u8
        };
        (*gr_ptr).scalefac_scale = get_bits(bs, 1) as u8;
        (*gr_ptr).count1_table = get_bits(bs, 1) as u8;
        (*gr_ptr).scfsi = ((scfsi >> 12) & 15) as u8;
        scfsi <<= 4;
        gr_ptr = gr_ptr.offset(1);
        gr_count -= 1;
        if gr_count == 0 {
            break;
        }
    }

    if part_23_sum + (*bs).pos > (*bs).limit + main_data_begin * 8 {
        return -1;
    }
    main_data_begin
}
