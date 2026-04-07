use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Default)]
struct BsT {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[repr(C)]
#[derive(Clone)]
struct L3GrInfoT {
    sfbtab: *const u8,
    part_23_length: u16,
    big_values: u16,
    scalefac_compress: u16,
    global_gain: u8,
    block_type: u8,
    mixed_block_flag: u8,
    n_long_sfb: u8,
    n_short_sfb: u8,
    table_select: [u8; 3],
    region_count: [u8; 3],
    subblock_gain: [u8; 3],
    preflag: u8,
    scalefac_scale: u8,
    count1_table: u8,
    scfsi: u8,
}

impl Default for L3GrInfoT {
    fn default() -> Self {
        Self {
            sfbtab: std::ptr::null(),
            part_23_length: 0, big_values: 0, scalefac_compress: 0,
            global_gain: 0, block_type: 0, mixed_block_flag: 0,
            n_long_sfb: 0, n_short_sfb: 0,
            table_select: [0; 3], region_count: [0; 3], subblock_gain: [0; 3],
            preflag: 0, scalefac_scale: 0, count1_table: 0, scfsi: 0,
        }
    }
}

type ReadSideInfoFn = unsafe extern "C" fn(*mut BsT, *mut L3GrInfoT, *const u8) -> c_int;

fn load_libs() -> (Library, Library) {
    unsafe {
        let c_lib = Library::new("c_src/build/libtranslated_rust.so")
            .expect("Failed to load C .so");
        let rust_lib = Library::new("target/debug/libread_side_info_lib.so")
            .expect("Failed to load Rust .so");
        (c_lib, rust_lib)
    }
}

fn compare_gr(c: &L3GrInfoT, r: &L3GrInfoT, label: &str) {
    // Compare all fields except sfbtab pointer (compare pointed-to data instead)
    assert_eq!(c.part_23_length, r.part_23_length, "{label}: part_23_length");
    assert_eq!(c.big_values, r.big_values, "{label}: big_values");
    assert_eq!(c.scalefac_compress, r.scalefac_compress, "{label}: scalefac_compress");
    assert_eq!(c.global_gain, r.global_gain, "{label}: global_gain");
    assert_eq!(c.block_type, r.block_type, "{label}: block_type");
    assert_eq!(c.mixed_block_flag, r.mixed_block_flag, "{label}: mixed_block_flag");
    assert_eq!(c.n_long_sfb, r.n_long_sfb, "{label}: n_long_sfb");
    assert_eq!(c.n_short_sfb, r.n_short_sfb, "{label}: n_short_sfb");
    assert_eq!(c.table_select, r.table_select, "{label}: table_select");
    assert_eq!(c.region_count, r.region_count, "{label}: region_count");
    assert_eq!(c.subblock_gain, r.subblock_gain, "{label}: subblock_gain");
    assert_eq!(c.preflag, r.preflag, "{label}: preflag");
    assert_eq!(c.scalefac_scale, r.scalefac_scale, "{label}: scalefac_scale");
    assert_eq!(c.count1_table, r.count1_table, "{label}: count1_table");
    assert_eq!(c.scfsi, r.scfsi, "{label}: scfsi");
    // Compare sfbtab contents (up to 40 bytes, until we hit 0 terminator)
    if !c.sfbtab.is_null() && !r.sfbtab.is_null() {
        for i in 0..40 {
            let cb = unsafe { *c.sfbtab.add(i) };
            let rb = unsafe { *r.sfbtab.add(i) };
            assert_eq!(cb, rb, "{label}: sfbtab[{i}]");
            if cb == 0 { break; }
        }
    }
}

fn call_both(
    c_fn: &ReadSideInfoFn, r_fn: &ReadSideInfoFn,
    data: &[u8], hdr: &[u8; 4], limit_bits: i32, gr_count: usize, label: &str,
) {
    let mut c_bs = BsT { buf: data.as_ptr(), pos: 0, limit: limit_bits };
    let mut r_bs = BsT { buf: data.as_ptr(), pos: 0, limit: limit_bits };
    let mut c_gr = vec![L3GrInfoT::default(); gr_count];
    let mut r_gr = vec![L3GrInfoT::default(); gr_count];

    let c_ret = unsafe { c_fn(&mut c_bs, c_gr.as_mut_ptr(), hdr.as_ptr()) };
    let r_ret = unsafe { r_fn(&mut r_bs, r_gr.as_mut_ptr(), hdr.as_ptr()) };

    assert_eq!(c_ret, r_ret, "{label}: return value (c={c_ret}, r={r_ret})");
    assert_eq!(c_bs.pos, r_bs.pos, "{label}: bs.pos after call");

    // Only compare granule data when both succeeded
    if c_ret >= 0 {
        for i in 0..gr_count {
            compare_gr(&c_gr[i], &r_gr[i], &format!("{label} gr[{i}]"));
        }
    }
}

/// Compute sr_idx from header to check validity (must be 0..=7)
fn sr_idx_from_hdr(hdr: &[u8; 4]) -> i32 {
    let mut sr_idx = (((hdr[2] >> 2) & 3) as i32)
        + (((hdr[1] >> 3) & 1) as i32 + ((hdr[1] >> 4) & 1) as i32) * 3;
    sr_idx -= (sr_idx != 0) as i32;
    sr_idx
}

/// Build a bitstream from a sequence of (value, nbits) pairs.
fn build_bits(fields: &[(u32, u8)]) -> Vec<u8> {
    let total_bits: usize = fields.iter().map(|(_, n)| *n as usize).sum();
    let mut buf = vec![0u8; (total_bits + 7) / 8 + 4]; // extra padding
    let mut pos = 0usize;
    for &(val, nbits) in fields {
        for i in (0..nbits).rev() {
            if i < 32 && (val >> i) & 1 != 0 {
                buf[pos / 8] |= 1 << (7 - (pos % 8));
            }
            pos += 1;
        }
    }
    buf
}

// MPEG1 stereo header: hdr[1] & 0x8 = set, hdr[3] & 0xC0 != 0xC0
// sr_idx from hdr[2] bits 3:2 and hdr[1] bits 4:3
fn make_hdr(mpeg1: bool, mono: bool, sr_bits: u8) -> [u8; 4] {
    let mut hdr = [0u8; 4];
    // hdr[1]: bit3 = mpeg1 flag
    if mpeg1 { hdr[1] |= 0x08; }
    // hdr[1] bits 4:3 contribute to sr_idx: ((hdr[1]>>3)&1) + ((hdr[1]>>4)&1)
    // For simplicity, set hdr[1] bits 4 to control sr_idx offset
    // sr_bits in hdr[2] bits 3:2
    hdr[2] = (sr_bits & 3) << 2;
    // hdr[3]: mono if 0xC0
    if mono { hdr[3] = 0xC0; }
    hdr
}

#[test]
fn test_mpeg1_stereo_normal_blocks() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    let hdr = make_hdr(true, false, 1); // MPEG1, stereo, sr_bits=1
    // gr_count = 2*2 = 4
    // Need: 9(main_data_begin) + 11(scfsi) + 4 * granule_bits
    // Normal block per granule: 12+9+8+4+1(win=0)+15+4+3+1+1+1 = 59 bits
    // Total: 9+11+4*59 = 256 bits
    let mut fields: Vec<(u32, u8)> = vec![
        (0, 9),   // main_data_begin = 0
        (0, 11),  // scfsi = 0
    ];
    for _ in 0..4 {
        fields.extend_from_slice(&[
            (100, 12),  // part_23_length
            (50, 9),    // big_values (< 288)
            (200, 8),   // global_gain
            (5, 4),     // scalefac_compress (MPEG1: 4 bits)
            (0, 1),     // window_switching = 0 → normal block
            (0b101_00011_01010, 15), // tables (3 x 5-bit fields packed)
            (3, 4),     // region_count[0]
            (2, 3),     // region_count[1]
            (1, 1),     // preflag
            (1, 1),     // scalefac_scale
            (0, 1),     // count1_table
        ]);
    }
    let data = build_bits(&fields);
    let total_bits = fields.iter().map(|(_, n)| *n as usize).sum::<usize>();
    call_both(&c_fn, &r_fn, &data, &hdr, (total_bits + 800) as i32, 4, "mpeg1_stereo_normal");
}

#[test]
fn test_mpeg1_stereo_window_switching_block_type_2_no_mixed() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    let hdr = make_hdr(true, false, 0);
    let mut fields: Vec<(u32, u8)> = vec![
        (0, 9),
        (0xFFFF & 0x7FF, 11), // scfsi all 1s
    ];
    for _ in 0..4 {
        fields.extend_from_slice(&[
            (50, 12),   // part_23_length
            (20, 9),    // big_values
            (128, 8),   // global_gain
            (3, 4),     // scalefac_compress
            (1, 1),     // window_switching = 1
            (2, 2),     // block_type = 2
            (0, 1),     // mixed_block_flag = 0 → short blocks
            (0b10101_01010, 10), // tables (2 x 5-bit)
            (5, 3),     // subblock_gain[0]
            (3, 3),     // subblock_gain[1]
            (7, 3),     // subblock_gain[2]
            (0, 1),     // preflag
            (1, 1),     // scalefac_scale
            (1, 1),     // count1_table
        ]);
    }
    let data = build_bits(&fields);
    let total_bits = fields.iter().map(|(_, n)| *n as usize).sum::<usize>();
    call_both(&c_fn, &r_fn, &data, &hdr, (total_bits + 800) as i32, 4,
              "mpeg1_stereo_bt2_nomixed");
}

#[test]
fn test_mpeg1_stereo_window_switching_block_type_2_mixed() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    let hdr = make_hdr(true, false, 2);
    let mut fields: Vec<(u32, u8)> = vec![(0, 9), (0, 11)];
    for _ in 0..4 {
        fields.extend_from_slice(&[
            (30, 12), (10, 9), (100, 8), (7, 4),
            (1, 1),   // window_switching = 1
            (2, 2),   // block_type = 2
            (1, 1),   // mixed_block_flag = 1
            (0b11111_00000, 10), (2, 3), (4, 3), (6, 3),
            (1, 1), (0, 1), (0, 1),
        ]);
    }
    let data = build_bits(&fields);
    let total_bits = fields.iter().map(|(_, n)| *n as usize).sum::<usize>();
    call_both(&c_fn, &r_fn, &data, &hdr, (total_bits + 800) as i32, 4,
              "mpeg1_stereo_bt2_mixed");
}

#[test]
fn test_mpeg1_stereo_window_switching_block_type_1() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    let hdr = make_hdr(true, false, 3);
    let mut fields: Vec<(u32, u8)> = vec![(0, 9), (0, 11)];
    for _ in 0..4 {
        fields.extend_from_slice(&[
            (40, 12), (15, 9), (150, 8), (10, 4),
            (1, 1),   // window_switching = 1
            (1, 2),   // block_type = 1 (not 2, not 0)
            (0, 1),   // mixed_block_flag
            (0b00001_11110, 10), (1, 3), (2, 3), (3, 3),
            (0, 1), (1, 1), (1, 1),
        ]);
    }
    let data = build_bits(&fields);
    let total_bits = fields.iter().map(|(_, n)| *n as usize).sum::<usize>();
    call_both(&c_fn, &r_fn, &data, &hdr, (total_bits + 800) as i32, 4,
              "mpeg1_stereo_bt1");
}

#[test]
fn test_mpeg1_mono() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    let hdr = make_hdr(true, true, 0); // MPEG1, mono → gr_count=1*2=2
    let mut fields: Vec<(u32, u8)> = vec![
        (5, 9),   // main_data_begin = 5
        (0b111_010101, 9), // scfsi (7 + gr_count=2 → 9 bits)
    ];
    for _ in 0..2 {
        fields.extend_from_slice(&[
            (80, 12), (40, 9), (180, 8), (12, 4),
            (0, 1), // normal block
            (0b11100_01100_00011, 15), (5, 4), (4, 3),
            (1, 1), (0, 1), (1, 1),
        ]);
    }
    let data = build_bits(&fields);
    let total_bits = fields.iter().map(|(_, n)| *n as usize).sum::<usize>();
    call_both(&c_fn, &r_fn, &data, &hdr, (total_bits + 800) as i32, 2, "mpeg1_mono");
}

#[test]
fn test_mpeg2_stereo() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    let hdr = make_hdr(false, false, 1); // MPEG2, stereo → gr_count=2
    // main_data_begin: get_bits(bs, 8+2=10) >> 2
    let mut fields: Vec<(u32, u8)> = vec![
        (20, 10), // raw bits for main_data_begin
    ];
    for _ in 0..2 {
        fields.extend_from_slice(&[
            (60, 12), (30, 9), (220, 8), (100, 9), // scalefac_compress: 9 bits for MPEG2
            (0, 1), // normal block
            (0b01010_10101_11111, 15), (7, 4), (5, 3),
            // no preflag bit for MPEG2 (uses scalefac_compress >= 500)
            (1, 1), (0, 1),
        ]);
    }
    let data = build_bits(&fields);
    let total_bits = fields.iter().map(|(_, n)| *n as usize).sum::<usize>();
    call_both(&c_fn, &r_fn, &data, &hdr, (total_bits + 800) as i32, 2, "mpeg2_stereo");
}

#[test]
fn test_mpeg2_mono() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    let hdr = make_hdr(false, true, 2); // MPEG2, mono → gr_count=1
    let mut fields: Vec<(u32, u8)> = vec![
        (10, 9), // main_data_begin raw bits (8+1=9, >>1)
    ];
    fields.extend_from_slice(&[
        (45, 12), (25, 9), (190, 8), (499, 9),
        (1, 1), (2, 2), (0, 1), // window, block_type=2, no mixed
        (0b01100_10011, 10), (7, 3), (5, 3), (3, 3),
        (0, 1), (1, 1),
    ]);
    let data = build_bits(&fields);
    let total_bits = fields.iter().map(|(_, n)| *n as usize).sum::<usize>();
    call_both(&c_fn, &r_fn, &data, &hdr, (total_bits + 800) as i32, 1, "mpeg2_mono");
}

#[test]
fn test_mpeg2_mono_preflag_threshold() {
    // Test scalefac_compress >= 500 → preflag = 1 for MPEG2
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    let hdr = make_hdr(false, true, 0);
    let mut fields: Vec<(u32, u8)> = vec![(0, 9)];
    fields.extend_from_slice(&[
        (20, 12), (10, 9), (100, 8), (500, 9), // scalefac_compress = 500 → preflag=1
        (0, 1), (0b10101_01010_11100, 15), (2, 4), (1, 3),
        (1, 1), (0, 1),
    ]);
    let data = build_bits(&fields);
    let total_bits = fields.iter().map(|(_, n)| *n as usize).sum::<usize>();
    call_both(&c_fn, &r_fn, &data, &hdr, (total_bits + 800) as i32, 1,
              "mpeg2_mono_preflag_500");

    // Also test 499 → preflag=0
    let mut fields2: Vec<(u32, u8)> = vec![(0, 9)];
    fields2.extend_from_slice(&[
        (20, 12), (10, 9), (100, 8), (499, 9),
        (0, 1), (0b10101_01010_11100, 15), (2, 4), (1, 3),
        (1, 1), (0, 1),
    ]);
    let data2 = build_bits(&fields2);
    let total_bits2 = fields2.iter().map(|(_, n)| *n as usize).sum::<usize>();
    call_both(&c_fn, &r_fn, &data2, &hdr, (total_bits2 + 800) as i32, 1,
              "mpeg2_mono_preflag_499");
}

#[test]
fn test_error_big_values_too_large() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    let hdr = make_hdr(true, true, 0);
    let mut fields: Vec<(u32, u8)> = vec![(0, 9), (0, 9)];
    fields.extend_from_slice(&[
        (100, 12),
        (289, 9), // big_values = 289 > 288 → error
    ]);
    // Pad with zeros
    fields.push((0, 64));
    let data = build_bits(&fields);
    call_both(&c_fn, &r_fn, &data, &hdr, 800, 2, "error_big_values");
}

#[test]
fn test_error_block_type_zero_with_window_switching() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    let hdr = make_hdr(true, true, 0);
    let mut fields: Vec<(u32, u8)> = vec![(0, 9), (0, 9)];
    fields.extend_from_slice(&[
        (50, 12), (20, 9), (100, 8), (5, 4),
        (1, 1),   // window_switching = 1
        (0, 2),   // block_type = 0 → error!
    ]);
    fields.push((0, 64));
    let data = build_bits(&fields);
    call_both(&c_fn, &r_fn, &data, &hdr, 800, 2, "error_bt0_window");
}

#[test]
fn test_all_sr_idx_values() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    // sr_idx = ((hdr[2]>>2)&3) + (((hdr[1]>>3)&1) + ((hdr[1]>>4)&1)) * 3
    // sr_idx -= (sr_idx != 0)
    // Test various combinations to hit different sr_idx values
    for sr_bits in 0..4u8 {
        for mpeg1 in [true, false] {
            // Also test with hdr[1] bit4 set to get higher sr_idx
            for bit4 in [false, true] {
                let mut hdr = make_hdr(mpeg1, true, sr_bits);
                if bit4 { hdr[1] |= 0x10; }

                // Skip invalid sr_idx (>= 8)
                let sr = sr_idx_from_hdr(&hdr);
                if sr < 0 || sr >= 8 { continue; }

                let mut fields: Vec<(u32, u8)> = if mpeg1 {
                    vec![(0, 9), (0, 9)] // main_data_begin + scfsi
                } else {
                    vec![(0, 9)] // main_data_begin
                };
                // One granule, normal block
                let sc_bits = if mpeg1 { 4 } else { 9 };
                fields.extend_from_slice(&[
                    (10, 12), (5, 9), (100, 8), (1, sc_bits),
                    (0, 1), (0, 15), (1, 4), (1, 3),
                ]);
                if mpeg1 { fields.push((0, 1)); } // preflag
                fields.extend_from_slice(&[(0, 1), (0, 1)]);

                if mpeg1 {
                    // Second granule for mono MPEG1 (gr_count=2)
                    fields.extend_from_slice(&[
                        (10, 12), (5, 9), (100, 8), (1, 4),
                        (0, 1), (0, 15), (1, 4), (1, 3),
                        (0, 1), (0, 1), (0, 1),
                    ]);
                }

                let data = build_bits(&fields);
                let total_bits = fields.iter().map(|(_, n)| *n as usize).sum::<usize>();
                let gr_count = if mpeg1 { 2 } else { 1 };
                call_both(&c_fn, &r_fn, &data, &hdr,
                          (total_bits + 800) as i32, gr_count,
                          &format!("sr_idx_sr{sr_bits}_mpeg{}_bit4{bit4}", mpeg1 as u8));
            }
        }
    }
}

#[test]
fn test_random_bitstreams() {
    // Fuzz-like test: generate pseudo-random bitstreams and compare
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { c_lib.get(b"read_side_info").unwrap() };
    let r_fn: Symbol<ReadSideInfoFn> = unsafe { rust_lib.get(b"read_side_info").unwrap() };

    // Simple LCG for deterministic pseudo-random
    let mut seed: u64 = 0xDEADBEEF;
    let mut next_rand = move || -> u8 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 33) as u8
    };

    for trial in 0..200 {
        let mut data = vec![0u8; 128];
        for b in data.iter_mut() { *b = next_rand(); }

        // Try all 4 header combos
        for (mpeg1, mono) in [(true, false), (true, true), (false, false), (false, true)] {
            let hdr = make_hdr(mpeg1, mono, next_rand() & 3);
            // Skip invalid sr_idx
            let sr = sr_idx_from_hdr(&hdr);
            if sr < 0 || sr >= 8 { continue; }
            let gr_count = match (mpeg1, mono) {
                (true, false) => 4,
                (true, true) => 2,
                (false, false) => 2,
                (false, true) => 1,
            };

            let mut c_bs = BsT { buf: data.as_ptr(), pos: 0, limit: (data.len() * 8) as i32 };
            let mut r_bs = BsT { buf: data.as_ptr(), pos: 0, limit: (data.len() * 8) as i32 };
            let mut c_gr = vec![L3GrInfoT::default(); gr_count];
            let mut r_gr = vec![L3GrInfoT::default(); gr_count];

            let c_ret = unsafe { c_fn(&mut c_bs, c_gr.as_mut_ptr(), hdr.as_ptr()) };
            let r_ret = unsafe { r_fn(&mut r_bs, r_gr.as_mut_ptr(), hdr.as_ptr()) };

            assert_eq!(c_ret, r_ret,
                       "trial {trial} mpeg1={mpeg1} mono={mono}: return mismatch c={c_ret} r={r_ret}");
            assert_eq!(c_bs.pos, r_bs.pos,
                       "trial {trial} mpeg1={mpeg1} mono={mono}: pos mismatch");

            if c_ret >= 0 {
                for i in 0..gr_count {
                    compare_gr(&c_gr[i], &r_gr[i],
                               &format!("trial {trial} mpeg1={mpeg1} mono={mono} gr[{i}]"));
                }
            }
        }
    }
}
