use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone)]
struct bs_t {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[repr(C)]
#[derive(Clone)]
struct L3_gr_info_t {
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

fn zero_gr() -> L3_gr_info_t {
    L3_gr_info_t {
        sfbtab: std::ptr::null(), part_23_length: 0, big_values: 0,
        scalefac_compress: 0, global_gain: 0, block_type: 0,
        mixed_block_flag: 0, n_long_sfb: 0, n_short_sfb: 0,
        table_select: [0; 3], region_count: [0; 3], subblock_gain: [0; 3],
        preflag: 0, scalefac_scale: 0, count1_table: 0, scfsi: 0,
    }
}

type ReadSideInfoFn = unsafe extern "C" fn(*mut bs_t, *mut L3_gr_info_t, *const u8) -> c_int;

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest).join("c_src/build/libread_side_info_lib.so")
}

fn gr_fields_match(a: &L3_gr_info_t, b: &L3_gr_info_t) -> bool {
    a.part_23_length == b.part_23_length
        && a.big_values == b.big_values
        && a.scalefac_compress == b.scalefac_compress
        && a.global_gain == b.global_gain
        && a.block_type == b.block_type
        && a.mixed_block_flag == b.mixed_block_flag
        && a.n_long_sfb == b.n_long_sfb
        && a.n_short_sfb == b.n_short_sfb
        && a.table_select == b.table_select
        && a.region_count == b.region_count
        && a.subblock_gain == b.subblock_gain
        && a.preflag == b.preflag
        && a.scalefac_scale == b.scalefac_scale
        && a.count1_table == b.count1_table
        && a.scfsi == b.scfsi
}

fn fmt_gr(g: &L3_gr_info_t) -> String {
    format!(
        "part23={} big_val={} gain={} sfcomp={} block={} mixed={} nlong={} nshort={} \
         tsel={:?} rcnt={:?} subg={:?} pre={} sfs={} c1t={} scfsi={}",
        g.part_23_length, g.big_values, g.global_gain, g.scalefac_compress,
        g.block_type, g.mixed_block_flag, g.n_long_sfb, g.n_short_sfb,
        g.table_select, g.region_count, g.subblock_gain,
        g.preflag, g.scalefac_scale, g.count1_table, g.scfsi
    )
}

unsafe fn read_sfbtab(ptr: *const u8, max: usize) -> Vec<u8> {
    if ptr.is_null() { return vec![]; }
    let mut v = Vec::new();
    for i in 0..max {
        let b = *ptr.add(i);
        v.push(b);
        if b == 0 { break; }
    }
    v
}

fn make_rust_gr() -> read_side_info_lib::L3_gr_info_t {
    // Safety: all-zeros is valid for this repr(C) struct with a null pointer
    unsafe { std::mem::zeroed() }
}

fn run_test(name: &str, data: &[u8], hdr: &[u8; 4], gr_count: usize) {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { lib.get(b"read_side_info").unwrap() };

    let mut c_bs = bs_t { buf: data.as_ptr(), pos: 0, limit: (data.len() * 8) as c_int };
    let mut c_gr: Vec<L3_gr_info_t> = (0..gr_count).map(|_| zero_gr()).collect();
    let c_ret = unsafe { c_fn(&mut c_bs, c_gr.as_mut_ptr(), hdr.as_ptr()) };

    let mut r_bs = read_side_info_lib::bs_t {
        buf: data.as_ptr(), pos: 0, limit: (data.len() * 8) as c_int,
    };
    let mut r_gr: Vec<read_side_info_lib::L3_gr_info_t> =
        (0..gr_count).map(|_| make_rust_gr()).collect();
    let r_ret = unsafe {
        read_side_info_lib::read_side_info(&mut r_bs, r_gr.as_mut_ptr(), hdr.as_ptr())
    };

    assert_eq!(c_ret, r_ret, "[{name}] return value mismatch: C={c_ret} Rust={r_ret}");
    assert_eq!(c_bs.pos, r_bs.pos, "[{name}] bs.pos mismatch: C={} Rust={}", c_bs.pos, r_bs.pos);

    if c_ret >= 0 {
        for i in 0..gr_count {
            let cg: &L3_gr_info_t = unsafe { &*(&c_gr[i] as *const _ as *const L3_gr_info_t) };
            let rg: &L3_gr_info_t = unsafe { &*(&r_gr[i] as *const _ as *const L3_gr_info_t) };
            assert!(gr_fields_match(cg, rg),
                "[{name}] gr[{i}] mismatch:\n  C:    {}\n  Rust: {}", fmt_gr(cg), fmt_gr(rg));
            let c_sfb = unsafe { read_sfbtab(cg.sfbtab, 50) };
            let r_sfb = unsafe { read_sfbtab(rg.sfbtab, 50) };
            assert_eq!(c_sfb, r_sfb, "[{name}] gr[{i}] sfbtab mismatch");
        }
    }
}

#[test]
fn test_mpeg1_stereo_normal() {
    let hdr: [u8; 4] = [0xFF, 0xFB, 0x90, 0x00];
    let data: Vec<u8> = (0..64).map(|i| ((i * 37 + 13) & 0xFF) as u8).collect();
    run_test("mpeg1_stereo_normal", &data, &hdr, 4);
}

#[test]
fn test_mpeg1_mono_normal() {
    let hdr: [u8; 4] = [0xFF, 0xFB, 0x90, 0xC0];
    let data: Vec<u8> = (0..64).map(|i| ((i * 53 + 7) & 0xFF) as u8).collect();
    run_test("mpeg1_mono_normal", &data, &hdr, 2);
}

#[test]
fn test_mpeg2_stereo() {
    let hdr: [u8; 4] = [0xFF, 0xF3, 0x90, 0x00];
    let data: Vec<u8> = (0..64).map(|i| ((i * 41 + 19) & 0xFF) as u8).collect();
    run_test("mpeg2_stereo", &data, &hdr, 2);
}

#[test]
fn test_mpeg2_mono() {
    let hdr: [u8; 4] = [0xFF, 0xF3, 0x90, 0xC0];
    let data: Vec<u8> = (0..64).map(|i| ((i * 61 + 3) & 0xFF) as u8).collect();
    run_test("mpeg2_mono", &data, &hdr, 1);
}

#[test]
fn test_many_random_inputs() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C") };
    let c_fn: Symbol<ReadSideInfoFn> = unsafe { lib.get(b"read_side_info").unwrap() };

    for seed in 0u64..500 {
        let mut data = vec![0u8; 64];
        let mut s = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        for b in data.iter_mut() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            *b = (s >> 33) as u8;
        }

        for &hdr in &[
            [0xFFu8, 0xFB, 0x90, 0x00], // MPEG1 stereo
            [0xFF, 0xFB, 0x90, 0xC0],   // MPEG1 mono
            [0xFF, 0xF3, 0x90, 0x00],   // MPEG2 stereo
            [0xFF, 0xF3, 0x90, 0xC0],   // MPEG2 mono
            [0xFF, 0xFB, 0x00, 0x00],   // different sr_idx
            [0xFF, 0xFB, 0xB4, 0x00],   // another sr_idx
        ] {
            let is_mpeg1 = (hdr[1] & 0x08) != 0;
            let is_mono = (hdr[3] & 0xC0) == 0xC0;
            let gr_count = if is_mpeg1 {
                if is_mono { 2 } else { 4 }
            } else {
                if is_mono { 1 } else { 2 }
            };

            let mut c_bs = bs_t { buf: data.as_ptr(), pos: 0, limit: (data.len() * 8) as c_int };
            let mut c_gr: Vec<L3_gr_info_t> = (0..gr_count).map(|_| zero_gr()).collect();
            let c_ret = unsafe { c_fn(&mut c_bs, c_gr.as_mut_ptr(), hdr.as_ptr()) };

            let mut r_bs = read_side_info_lib::bs_t {
                buf: data.as_ptr(), pos: 0, limit: (data.len() * 8) as c_int,
            };
            let mut r_gr: Vec<read_side_info_lib::L3_gr_info_t> =
                (0..gr_count).map(|_| make_rust_gr()).collect();
            let r_ret = unsafe {
                read_side_info_lib::read_side_info(&mut r_bs, r_gr.as_mut_ptr(), hdr.as_ptr())
            };

            assert_eq!(c_ret, r_ret,
                "seed={seed} hdr={hdr:02X?} return mismatch: C={c_ret} Rust={r_ret}");

            if c_ret >= 0 {
                assert_eq!(c_bs.pos, r_bs.pos,
                    "seed={seed} hdr={hdr:02X?} bs.pos mismatch");
                for i in 0..gr_count {
                    let cg: &L3_gr_info_t = unsafe { &*(&c_gr[i] as *const _ as *const L3_gr_info_t) };
                    let rg: &L3_gr_info_t = unsafe { &*(&r_gr[i] as *const _ as *const L3_gr_info_t) };
                    if !gr_fields_match(cg, rg) {
                        panic!("seed={seed} hdr={hdr:02X?} gr[{i}] mismatch:\n  C:    {}\n  Rust: {}",
                            fmt_gr(cg), fmt_gr(rg));
                    }
                    let c_sfb = unsafe { read_sfbtab(cg.sfbtab, 50) };
                    let r_sfb = unsafe { read_sfbtab(rg.sfbtab, 50) };
                    assert_eq!(c_sfb, r_sfb,
                        "seed={seed} hdr={hdr:02X?} gr[{i}] sfbtab mismatch:\n  C: {c_sfb:?}\n  Rust: {r_sfb:?}");
                }
            }
        }
    }
}
