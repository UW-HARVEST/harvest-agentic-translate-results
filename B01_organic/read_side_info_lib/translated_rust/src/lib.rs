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
    let mut next: u32 = (*p as u32) & (255u32 >> s);
    p = p.offset(1);
    let mut cache: u32 = 0;
    shl -= 8;
    while shl > 0 {
        cache |= next << shl;
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
    let mut scfsi: u32 = 0;
    let mut part_23_sum: c_int = 0;
    let mut gr = gr;

    let hdr1 = *hdr.offset(1);
    let hdr2 = *hdr.offset(2);
    let hdr3 = *hdr.offset(3);

    let mut sr_idx: c_int = (((hdr2 >> 2) & 3) as c_int)
        + (((hdr1 >> 3) & 1) as c_int + ((hdr1 >> 4) & 1) as c_int) * 3;
    sr_idx -= (sr_idx != 0) as c_int;

    let mut gr_count: c_int = if (hdr3 & 0xC0) == 0xC0 { 1 } else { 2 };

    let main_data_begin: c_int;
    if (hdr1 & 0x8) != 0 {
        gr_count *= 2;
        main_data_begin = get_bits(bs, 9) as c_int;
        scfsi = get_bits(bs, 7 + gr_count);
    } else {
        main_data_begin = (get_bits(bs, 8 + gr_count) >> gr_count) as c_int;
    }

    loop {
        if (hdr3 & 0xC0) == 0xC0 {
            scfsi <<= 4;
        }
        (*gr).part_23_length = get_bits(bs, 12) as u16;
        part_23_sum += (*gr).part_23_length as c_int;
        (*gr).big_values = get_bits(bs, 9) as u16;
        if (*gr).big_values > 288 {
            return -1;
        }
        (*gr).global_gain = get_bits(bs, 8) as u8;
        (*gr).scalefac_compress = get_bits(bs, if (hdr1 & 0x8) != 0 { 4 } else { 9 }) as u16;
        (*gr).sfbtab = G_SCF_LONG[sr_idx as usize].as_ptr();
        (*gr).n_long_sfb = 22;
        (*gr).n_short_sfb = 0;

        let tables: u32;
        if get_bits(bs, 1) != 0 {
            (*gr).block_type = get_bits(bs, 2) as u8;
            if (*gr).block_type == 0 {
                return -1;
            }
            (*gr).mixed_block_flag = get_bits(bs, 1) as u8;
            (*gr).region_count[0] = 7;
            (*gr).region_count[1] = 255;
            if (*gr).block_type == 2 {
                scfsi &= 0x0F0F;
                if (*gr).mixed_block_flag == 0 {
                    (*gr).region_count[0] = 8;
                    (*gr).sfbtab = G_SCF_SHORT[sr_idx as usize].as_ptr();
                    (*gr).n_long_sfb = 0;
                    (*gr).n_short_sfb = 39;
                } else {
                    (*gr).sfbtab = G_SCF_MIXED[sr_idx as usize].as_ptr();
                    (*gr).n_long_sfb = if (hdr1 & 0x8) != 0 { 8 } else { 6 };
                    (*gr).n_short_sfb = 30;
                }
            }
            tables = {
                let t = get_bits(bs, 10);
                t << 5
            };
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
        (*gr).preflag = if (hdr1 & 0x8) != 0 {
            get_bits(bs, 1) as u8
        } else {
            ((*gr).scalefac_compress >= 500) as u8
        };
        (*gr).scalefac_scale = get_bits(bs, 1) as u8;
        (*gr).count1_table = get_bits(bs, 1) as u8;
        (*gr).scfsi = ((scfsi >> 12) & 15) as u8;
        scfsi <<= 4;
        gr = gr.offset(1);

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
