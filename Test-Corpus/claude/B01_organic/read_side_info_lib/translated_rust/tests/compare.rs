use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

static C_LIB: OnceLock<Library> = OnceLock::new();
static R_LIB: OnceLock<Library> = OnceLock::new();

fn c_lib() -> &'static Library {
    C_LIB.get_or_init(|| unsafe { Library::new(c_lib_path()).expect("load c lib") })
}

fn r_lib() -> &'static Library {
    R_LIB.get_or_init(|| unsafe { Library::new(rust_lib_path()).expect("load rust lib") })
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct BsT {
    buf: *const u8,
    pos: i32,
    limit: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
        L3GrInfoT {
            sfbtab: std::ptr::null(),
            part_23_length: 0,
            big_values: 0,
            scalefac_compress: 0,
            global_gain: 0,
            block_type: 0,
            mixed_block_flag: 0,
            n_long_sfb: 0,
            n_short_sfb: 0,
            table_select: [0; 3],
            region_count: [0; 3],
            subblock_gain: [0; 3],
            preflag: 0,
            scalefac_scale: 0,
            count1_table: 0,
            scfsi: 0,
        }
    }
}

type ReadSideInfoFn =
    unsafe extern "C" fn(bs: *mut BsT, gr: *mut L3GrInfoT, hdr: *const u8) -> i32;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Built as a cdylib in target/<profile>/
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Try release first, then debug
    let release = p.join("release").join("libread_side_info_lib.so");
    if release.exists() {
        return release;
    }
    p.join("debug").join("libread_side_info_lib.so")
}

unsafe fn call_c(buf: &[u8], limit: i32, hdr: &[u8; 4], gr: &mut [L3GrInfoT; 4]) -> i32 {
    let lib = c_lib();
    let func: Symbol<ReadSideInfoFn> = lib.get(b"read_side_info").expect("c sym");
    let mut bs = BsT {
        buf: buf.as_ptr(),
        pos: 0,
        limit,
    };
    func(&mut bs, gr.as_mut_ptr(), hdr.as_ptr())
}

unsafe fn call_rust(
    buf: &[u8],
    limit: i32,
    hdr: &[u8; 4],
    gr: &mut [L3GrInfoT; 4],
) -> i32 {
    let lib = r_lib();
    let func: Symbol<ReadSideInfoFn> = lib.get(b"read_side_info").expect("rust sym");
    let mut bs = BsT {
        buf: buf.as_ptr(),
        pos: 0,
        limit,
    };
    func(&mut bs, gr.as_mut_ptr(), hdr.as_ptr())
}

fn sfbtab_size(g: &L3GrInfoT) -> usize {
    if g.n_long_sfb == 22 && g.n_short_sfb == 0 {
        23
    } else if g.n_long_sfb == 0 && g.n_short_sfb == 39 {
        40
    } else {
        40
    }
}

fn assert_gr_equal(c: &L3GrInfoT, r: &L3GrInfoT, label: &str) {
    assert_eq!(c.part_23_length, r.part_23_length, "{}: part_23_length", label);
    assert_eq!(c.big_values, r.big_values, "{}: big_values", label);
    assert_eq!(
        c.scalefac_compress, r.scalefac_compress,
        "{}: scalefac_compress",
        label
    );
    assert_eq!(c.global_gain, r.global_gain, "{}: global_gain", label);
    assert_eq!(c.block_type, r.block_type, "{}: block_type", label);
    assert_eq!(
        c.mixed_block_flag, r.mixed_block_flag,
        "{}: mixed_block_flag",
        label
    );
    assert_eq!(c.n_long_sfb, r.n_long_sfb, "{}: n_long_sfb", label);
    assert_eq!(c.n_short_sfb, r.n_short_sfb, "{}: n_short_sfb", label);
    assert_eq!(c.table_select, r.table_select, "{}: table_select", label);
    assert_eq!(c.region_count, r.region_count, "{}: region_count", label);
    assert_eq!(c.subblock_gain, r.subblock_gain, "{}: subblock_gain", label);
    assert_eq!(c.preflag, r.preflag, "{}: preflag", label);
    assert_eq!(
        c.scalefac_scale, r.scalefac_scale,
        "{}: scalefac_scale",
        label
    );
    assert_eq!(c.count1_table, r.count1_table, "{}: count1_table", label);
    assert_eq!(c.scfsi, r.scfsi, "{}: scfsi", label);

    let csize = sfbtab_size(c);
    let rsize = sfbtab_size(r);
    assert_eq!(csize, rsize, "{}: sfbtab size deduced", label);
    if !c.sfbtab.is_null() && !r.sfbtab.is_null() {
        let c_slice = unsafe { std::slice::from_raw_parts(c.sfbtab, csize) };
        let r_slice = unsafe { std::slice::from_raw_parts(r.sfbtab, rsize) };
        assert_eq!(c_slice, r_slice, "{}: sfbtab contents", label);
    } else {
        assert_eq!(c.sfbtab.is_null(), r.sfbtab.is_null(), "{}: sfbtab null", label);
    }
}

fn run_case(buf: &[u8], hdr: [u8; 4], label: &str) {
    let limit = (buf.len() as i32) * 8;
    let mut c_gr = [L3GrInfoT::default(); 4];
    let mut r_gr = [L3GrInfoT::default(); 4];

    let c_ret = unsafe { call_c(buf, limit, &hdr, &mut c_gr) };
    let r_ret = unsafe { call_rust(buf, limit, &hdr, &mut r_gr) };

    assert_eq!(c_ret, r_ret, "{}: return value (hdr={:02x?})", label, hdr);

    if c_ret >= 0 {
        // gr_count: 1 if (hdr[3] & 0xC0) == 0xC0 else 2
        // Then doubled if (hdr[1] & 0x8) != 0
        let mut gr_count = if (hdr[3] & 0xC0) == 0xC0 { 1 } else { 2 };
        if (hdr[1] & 0x8) != 0 {
            gr_count *= 2;
        }
        for i in 0..gr_count {
            assert_gr_equal(&c_gr[i], &r_gr[i], &format!("{} gr[{}]", label, i));
        }
    }
}

// Generate a buffer that is "long enough" — 256 bytes of varying patterns.
fn make_buf(seed: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(256);
    let mut s = seed;
    for _ in 0..256 {
        s = s.wrapping_mul(1103515245).wrapping_add(12345);
        v.push((s >> 16) as u8);
    }
    v
}

#[test]
fn test_basic_mpeg1_stereo() {
    // hdr[1] & 0x8: MPEG1 (id bit = 1)
    // hdr[1] >> 3 & 1 = 1 => high bit
    // hdr[1] >> 4 & 1 = 0 (ignore for now)
    // hdr[3] & 0xC0: stereo (mode != 0xC0)
    let hdr = [0xFF, 0xFB, 0x90, 0x00]; // MPEG1 layer3, 128kbps, stereo
    for seed in 1..50 {
        let buf = make_buf(seed);
        run_case(&buf, hdr, &format!("mpeg1-stereo-seed{}", seed));
    }
}

#[test]
fn test_basic_mpeg1_mono() {
    // mono mode: hdr[3] & 0xC0 == 0xC0 (channel mode 11 = mono)
    let hdr = [0xFF, 0xFB, 0x90, 0xC0];
    for seed in 1..50 {
        let buf = make_buf(seed);
        run_case(&buf, hdr, &format!("mpeg1-mono-seed{}", seed));
    }
}

#[test]
fn test_basic_mpeg2_stereo() {
    // MPEG2: id bit = 0, hdr[1] & 0x8 == 0
    // hdr[1] = 11110011 -> ID=0
    let hdr = [0xFF, 0xF3, 0x90, 0x00];
    for seed in 1..50 {
        let buf = make_buf(seed);
        run_case(&buf, hdr, &format!("mpeg2-stereo-seed{}", seed));
    }
}

#[test]
fn test_basic_mpeg2_mono() {
    let hdr = [0xFF, 0xF3, 0x90, 0xC0];
    for seed in 1..50 {
        let buf = make_buf(seed);
        run_case(&buf, hdr, &format!("mpeg2-mono-seed{}", seed));
    }
}

#[test]
fn test_zeros_input() {
    // All zeros should hit certain paths
    let buf = vec![0u8; 256];
    let hdrs = [
        [0xFFu8, 0xFB, 0x90, 0x00],
        [0xFFu8, 0xFB, 0x90, 0xC0],
        [0xFFu8, 0xF3, 0x90, 0x00],
        [0xFFu8, 0xF3, 0x90, 0xC0],
    ];
    for (i, hdr) in hdrs.iter().enumerate() {
        run_case(&buf, *hdr, &format!("zeros-{}", i));
    }
}

#[test]
fn test_ones_input() {
    let buf = vec![0xFFu8; 256];
    let hdrs = [
        [0xFFu8, 0xFB, 0x90, 0x00],
        [0xFFu8, 0xFB, 0x90, 0xC0],
        [0xFFu8, 0xF3, 0x90, 0x00],
        [0xFFu8, 0xF3, 0x90, 0xC0],
    ];
    for (i, hdr) in hdrs.iter().enumerate() {
        run_case(&buf, *hdr, &format!("ones-{}", i));
    }
}

#[test]
fn test_various_sample_rates() {
    // sr_idx = ((hdr[2]>>2)&3) + ((hdr[1]>>3)&1 + (hdr[1]>>4)&1)*3
    // Then sr_idx -= (sr_idx != 0).
    // Final sr_idx must be in [0, 7] (array has 8 entries).
    // Frequency index 3 is "reserved" in real MP3; combined with sum==2 it
    // would overflow the table — skip that case.
    let buf = make_buf(0xCAFEBABE);
    let hdr1_options: &[u8] = &[
        0xFB, // sum = 2 (MPEG1)
        0xF3, // sum = 1 (MPEG2)
        0xE3, // sum = 0
        0xEB, // sum = 1
    ];
    for &hdr1 in hdr1_options {
        let sum = (((hdr1 >> 3) & 1) + ((hdr1 >> 4) & 1)) as u32;
        for hdr2_freq in 0u8..4 {
            // Skip combinations that would make sr_idx >= 8
            let raw = (hdr2_freq as u32) + sum * 3;
            let sr_idx = if raw != 0 { raw - 1 } else { 0 };
            if sr_idx >= 8 {
                continue;
            }
            for hdr3_mode in [0x00u8, 0x40, 0x80, 0xC0].iter() {
                let hdr2 = (hdr2_freq << 2) | (0x90 & !0x0C);
                let hdr = [0xFF, hdr1, hdr2, *hdr3_mode];
                run_case(
                    &buf,
                    hdr,
                    &format!("hdr1={:02x},sr={},mode={:02x}", hdr1, hdr2_freq, hdr3_mode),
                );
            }
        }
    }
}

#[test]
fn test_short_limit() {
    // Test where the buffer/limit may be exceeded → get_bits returns 0
    let buf = make_buf(42);
    let hdr = [0xFF, 0xFB, 0x90, 0x00];
    let limit = 16; // small limit, so get_bits hits the cap quickly
    let mut c_gr = [L3GrInfoT::default(); 4];
    let mut r_gr = [L3GrInfoT::default(); 4];
    let c_ret = unsafe { call_c(&buf, limit, &hdr, &mut c_gr) };
    let r_ret = unsafe { call_rust(&buf, limit, &hdr, &mut r_gr) };
    assert_eq!(c_ret, r_ret, "short limit return");
    // Compare resulting state regardless
    for i in 0..4 {
        assert_eq!(c_gr[i].part_23_length, r_gr[i].part_23_length);
        assert_eq!(c_gr[i].big_values, r_gr[i].big_values);
        assert_eq!(c_gr[i].scalefac_compress, r_gr[i].scalefac_compress);
        assert_eq!(c_gr[i].global_gain, r_gr[i].global_gain);
        assert_eq!(c_gr[i].block_type, r_gr[i].block_type);
        assert_eq!(c_gr[i].mixed_block_flag, r_gr[i].mixed_block_flag);
        assert_eq!(c_gr[i].n_long_sfb, r_gr[i].n_long_sfb);
        assert_eq!(c_gr[i].n_short_sfb, r_gr[i].n_short_sfb);
        assert_eq!(c_gr[i].table_select, r_gr[i].table_select);
        assert_eq!(c_gr[i].region_count, r_gr[i].region_count);
        assert_eq!(c_gr[i].subblock_gain, r_gr[i].subblock_gain);
        assert_eq!(c_gr[i].preflag, r_gr[i].preflag);
        assert_eq!(c_gr[i].scalefac_scale, r_gr[i].scalefac_scale);
        assert_eq!(c_gr[i].count1_table, r_gr[i].count1_table);
        assert_eq!(c_gr[i].scfsi, r_gr[i].scfsi);
    }
}

#[test]
fn test_many_random_seeds() {
    let hdrs = [
        [0xFFu8, 0xFB, 0x90, 0x00],
        [0xFFu8, 0xFB, 0x90, 0xC0],
        [0xFFu8, 0xF3, 0x90, 0x00],
        [0xFFu8, 0xF3, 0x90, 0xC0],
        [0xFFu8, 0xFB, 0x84, 0x40],
        [0xFFu8, 0xFB, 0x88, 0x80],
        [0xFFu8, 0xF3, 0x80, 0x40],
        [0xFFu8, 0xF3, 0x84, 0x80],
    ];
    for seed in 0..200 {
        let buf = make_buf(seed * 7919 + 1);
        for hdr in hdrs.iter() {
            run_case(&buf, *hdr, &format!("rand-seed{}", seed));
        }
    }
}

#[test]
fn test_symbol_parity() {
    // Verify the C lib's exported symbols exist in the Rust lib too.
    // Use nm output via std::process::Command.
    use std::process::Command;
    let c_out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(c_lib_path())
        .output()
        .expect("nm c");
    let r_out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(rust_lib_path())
        .output()
        .expect("nm rust");

    let c_str = String::from_utf8_lossy(&c_out.stdout).to_string();
    let r_str = String::from_utf8_lossy(&r_out.stdout).to_string();

    let extract_user_syms = |s: &str| -> Vec<String> {
        s.lines()
            .filter_map(|l| {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() < 3 {
                    return None;
                }
                let name = parts[2];
                // Filter out runtime-provided symbols common to all shared libs.
                let skip = [
                    "_init",
                    "_fini",
                    "__bss_start",
                    "_edata",
                    "_end",
                    "__cxa_finalize",
                ];
                if skip.contains(&name) {
                    return None;
                }
                if name.starts_with("__") {
                    return None;
                }
                Some(name.to_string())
            })
            .collect()
    };

    let c_syms = extract_user_syms(&c_str);
    let r_syms = extract_user_syms(&r_str);

    for s in &c_syms {
        assert!(
            r_syms.iter().any(|x| x == s),
            "Rust .so missing symbol exported by C .so: {}",
            s
        );
    }
}
