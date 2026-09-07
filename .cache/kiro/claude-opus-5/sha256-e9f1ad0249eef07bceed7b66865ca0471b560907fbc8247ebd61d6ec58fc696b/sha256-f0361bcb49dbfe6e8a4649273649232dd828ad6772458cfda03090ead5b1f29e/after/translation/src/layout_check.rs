//! Compile-time assertions that the FFI structs match the C layout
//! (verified against gcc: bs_t = 16/8, L3_gr_info_t = 32/8 with the offsets
//! listed below).

use super::{L3_gr_info_t, bs_t};

const _: () = {
    assert!(size_of::<bs_t>() == 16);
    assert!(align_of::<bs_t>() == 8);
    assert!(size_of::<L3_gr_info_t>() == 32);
    assert!(align_of::<L3_gr_info_t>() == 8);
    assert!(std::mem::offset_of!(bs_t, buf) == 0);
    assert!(std::mem::offset_of!(bs_t, pos) == 8);
    assert!(std::mem::offset_of!(bs_t, limit) == 12);
    assert!(std::mem::offset_of!(L3_gr_info_t, sfbtab) == 0);
    assert!(std::mem::offset_of!(L3_gr_info_t, part_23_length) == 8);
    assert!(std::mem::offset_of!(L3_gr_info_t, big_values) == 10);
    assert!(std::mem::offset_of!(L3_gr_info_t, scalefac_compress) == 12);
    assert!(std::mem::offset_of!(L3_gr_info_t, global_gain) == 14);
    assert!(std::mem::offset_of!(L3_gr_info_t, block_type) == 15);
    assert!(std::mem::offset_of!(L3_gr_info_t, mixed_block_flag) == 16);
    assert!(std::mem::offset_of!(L3_gr_info_t, n_long_sfb) == 17);
    assert!(std::mem::offset_of!(L3_gr_info_t, n_short_sfb) == 18);
    assert!(std::mem::offset_of!(L3_gr_info_t, table_select) == 19);
    assert!(std::mem::offset_of!(L3_gr_info_t, region_count) == 22);
    assert!(std::mem::offset_of!(L3_gr_info_t, subblock_gain) == 25);
    assert!(std::mem::offset_of!(L3_gr_info_t, preflag) == 28);
    assert!(std::mem::offset_of!(L3_gr_info_t, scalefac_scale) == 29);
    assert!(std::mem::offset_of!(L3_gr_info_t, count1_table) == 30);
    assert!(std::mem::offset_of!(L3_gr_info_t, scfsi) == 31);
};
