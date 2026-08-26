// Translated from c_src/src/lib.c
//
// Faithful Rust translation of the original C library. The semantics of
// `get_bits` and `read_side_info` (including all error checks and ordering)
// are preserved exactly.

#[derive(Debug)]
pub struct BsT<'a> {
    pub buf: &'a [u8],
    pub pos: i32,
    pub limit: i32,
}

#[derive(Debug, Default, Clone)]
pub struct L3GrInfoT {
    pub sfbtab: &'static [u8],
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

fn get_bits(bs: &mut BsT<'_>, n: i32) -> u32 {
    let s: u32 = (bs.pos as u32) & 7;
    let mut shl: i32 = n + s as i32;
    let p_idx: usize = (bs.pos >> 3) as usize;
    bs.pos += n;
    if bs.pos > bs.limit {
        return 0;
    }
    let mut p = p_idx;
    let mut next: u32 = (bs.buf[p] as u32) & (255u32 >> s);
    p += 1;
    let mut cache: u32 = 0;
    loop {
        shl -= 8;
        if shl <= 0 {
            break;
        }
        cache |= next << shl;
        next = bs.buf[p] as u32;
        p += 1;
    }
    // shl is <= 0; mirror C's `next >> -shl`
    cache | (next >> ((-shl) as u32))
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
        24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0,
        // Original C declares [40] but initializes 37 values; remaining are
        // implicitly zero. Pad to 40 entries.
        0, 0, 0,
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

pub fn read_side_info(bs: &mut BsT<'_>, gr: &mut [L3GrInfoT], hdr: &[u8]) -> i32 {
    let mut tables: u32;
    let mut scfsi: u32 = 0;
    let main_data_begin: i32;
    let mut part_23_sum: i32 = 0;

    let mut sr_idx: i32 = ((((hdr[2] as i32) >> 2) & 3)
        + (((hdr[1] as i32 >> 3) & 1) + ((hdr[1] as i32 >> 4) & 1)) * 3) as i32;
    sr_idx -= if sr_idx != 0 { 1 } else { 0 };

    let mut gr_count: i32 = if (hdr[3] & 0xC0) == 0xC0 { 1 } else { 2 };

    if (hdr[1] & 0x8) != 0 {
        gr_count *= 2;
        main_data_begin = get_bits(bs, 9) as i32;
        scfsi = get_bits(bs, 7 + gr_count);
    } else {
        main_data_begin = (get_bits(bs, 8 + gr_count) >> gr_count) as i32;
    }

    let mut gr_idx: usize = 0;
    loop {
        if (hdr[3] & 0xC0) == 0xC0 {
            scfsi <<= 4;
        }
        {
            let g = &mut gr[gr_idx];
            g.part_23_length = get_bits(bs, 12) as u16;
            part_23_sum += g.part_23_length as i32;
            g.big_values = get_bits(bs, 9) as u16;
            if g.big_values > 288 {
                return -1;
            }
            g.global_gain = get_bits(bs, 8) as u8;
            g.scalefac_compress = get_bits(bs, if (hdr[1] & 0x8) != 0 { 4 } else { 9 }) as u16;
            g.sfbtab = &G_SCF_LONG[sr_idx as usize];
            g.n_long_sfb = 22;
            g.n_short_sfb = 0;
        }

        if get_bits(bs, 1) != 0 {
            {
                let g = &mut gr[gr_idx];
                g.block_type = get_bits(bs, 2) as u8;
                if g.block_type == 0 {
                    return -1;
                }
                g.mixed_block_flag = get_bits(bs, 1) as u8;
                g.region_count[0] = 7;
                g.region_count[1] = 255;
                if g.block_type == 2 {
                    scfsi &= 0x0F0F;
                    if g.mixed_block_flag == 0 {
                        g.region_count[0] = 8;
                        g.sfbtab = &G_SCF_SHORT[sr_idx as usize];
                        g.n_long_sfb = 0;
                        g.n_short_sfb = 39;
                    } else {
                        g.sfbtab = &G_SCF_MIXED[sr_idx as usize];
                        g.n_long_sfb = if (hdr[1] & 0x8) != 0 { 8 } else { 6 };
                        g.n_short_sfb = 30;
                    }
                }
            }
            tables = get_bits(bs, 10);
            tables <<= 5;
            {
                let g = &mut gr[gr_idx];
                g.subblock_gain[0] = get_bits(bs, 3) as u8;
                g.subblock_gain[1] = get_bits(bs, 3) as u8;
                g.subblock_gain[2] = get_bits(bs, 3) as u8;
            }
        } else {
            {
                let g = &mut gr[gr_idx];
                g.block_type = 0;
                g.mixed_block_flag = 0;
            }
            tables = get_bits(bs, 15);
            {
                let g = &mut gr[gr_idx];
                g.region_count[0] = get_bits(bs, 4) as u8;
                g.region_count[1] = get_bits(bs, 3) as u8;
                g.region_count[2] = 255;
            }
        }

        {
            let g = &mut gr[gr_idx];
            g.table_select[0] = (tables >> 10) as u8;
            g.table_select[1] = ((tables >> 5) & 31) as u8;
            g.table_select[2] = (tables & 31) as u8;
            g.preflag = if (hdr[1] & 0x8) != 0 {
                get_bits(bs, 1) as u8
            } else if g.scalefac_compress >= 500 {
                1
            } else {
                0
            };
            g.scalefac_scale = get_bits(bs, 1) as u8;
            g.count1_table = get_bits(bs, 1) as u8;
            g.scfsi = ((scfsi >> 12) & 15) as u8;
        }
        scfsi <<= 4;
        gr_idx += 1;

        gr_count -= 1;
        if gr_count == 0 {
            break;
        }
    }

    if part_23_sum + bs.pos > bs.limit + main_data_begin * 8 {
        return -1;
    }
    main_data_begin
}
