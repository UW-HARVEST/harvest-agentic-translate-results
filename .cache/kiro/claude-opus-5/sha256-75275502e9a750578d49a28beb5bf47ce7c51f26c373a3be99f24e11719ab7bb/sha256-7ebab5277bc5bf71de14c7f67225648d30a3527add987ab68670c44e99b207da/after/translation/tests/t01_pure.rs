//! Tier 1: leaf functions that need no `png_struct` (or ignore it).
//! Both implementations are reached only through their exported symbols.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_long, c_void, CStr};

/* ------------------------------------------------------------------ misc */

#[test]
fn access_version_number() {
    let l = libs();
    let f: libloading::Symbol<unsafe extern "C-unwind" fn() -> u32> = l.c.sym("png_access_version_number");
    let g: libloading::Symbol<unsafe extern "C-unwind" fn() -> u32> = l.r.sym("png_access_version_number");
    assert_eq!(unsafe { f() }, unsafe { g() });
}

#[test]
fn version_strings() {
    let l = libs();
    for name in [
        "png_get_copyright",
        "png_get_header_ver",
        "png_get_header_version",
        "png_get_libpng_ver",
    ] {
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(*const c_void) -> *const c_char> =
            l.c.sym(name);
        let g: libloading::Symbol<unsafe extern "C-unwind" fn(*const c_void) -> *const c_char> =
            l.r.sym(name);
        let a = unsafe { CStr::from_ptr(f(std::ptr::null())) }.to_bytes().to_vec();
        let b = unsafe { CStr::from_ptr(g(std::ptr::null())) }.to_bytes().to_vec();
        assert_eq!(
            String::from_utf8_lossy(&a),
            String::from_utf8_lossy(&b),
            "{name} differs"
        );
    }
}

/* --------------------------------------------------------- byte accessors */

#[test]
fn get_uint_32_16_int32() {
    let l = libs();
    let cu32: libloading::Symbol<unsafe extern "C-unwind" fn(*const u8) -> u32> = l.c.sym("png_get_uint_32");
    let ru32: libloading::Symbol<unsafe extern "C-unwind" fn(*const u8) -> u32> = l.r.sym("png_get_uint_32");
    let cu16: libloading::Symbol<unsafe extern "C-unwind" fn(*const u8) -> u16> = l.c.sym("png_get_uint_16");
    let ru16: libloading::Symbol<unsafe extern "C-unwind" fn(*const u8) -> u16> = l.r.sym("png_get_uint_16");
    let ci32: libloading::Symbol<unsafe extern "C-unwind" fn(*const u8) -> i32> = l.c.sym("png_get_int_32");
    let ri32: libloading::Symbol<unsafe extern "C-unwind" fn(*const u8) -> i32> = l.r.sym("png_get_int_32");

    let mut cases: Vec<[u8; 4]> = vec![
        [0, 0, 0, 0],
        [0, 0, 0, 1],
        [0xff, 0xff, 0xff, 0xff],
        [0x80, 0, 0, 0],
        [0x80, 0, 0, 1],
        [0x7f, 0xff, 0xff, 0xff],
        [0x12, 0x34, 0x56, 0x78],
        [0xde, 0xad, 0xbe, 0xef],
        [0x00, 0x00, 0x80, 0x00],
    ];
    let mut s: u32 = 0x1234_5678;
    for _ in 0..512 {
        s = s.wrapping_mul(1103515245).wrapping_add(12345);
        cases.push(s.to_be_bytes());
    }
    for c in &cases {
        assert_eq!(unsafe { cu32(c.as_ptr()) }, unsafe { ru32(c.as_ptr()) }, "uint32 {c:?}");
        assert_eq!(unsafe { cu16(c.as_ptr()) }, unsafe { ru16(c.as_ptr()) }, "uint16 {c:?}");
        assert_eq!(unsafe { ci32(c.as_ptr()) }, unsafe { ri32(c.as_ptr()) }, "int32 {c:?}");
    }
}

#[test]
fn save_uint_32_16_int32() {
    let l = libs();
    let cs32: libloading::Symbol<unsafe extern "C-unwind" fn(*mut u8, u32)> = l.c.sym("png_save_uint_32");
    let rs32: libloading::Symbol<unsafe extern "C-unwind" fn(*mut u8, u32)> = l.r.sym("png_save_uint_32");
    let csi: libloading::Symbol<unsafe extern "C-unwind" fn(*mut u8, i32)> = l.c.sym("png_save_int_32");
    let rsi: libloading::Symbol<unsafe extern "C-unwind" fn(*mut u8, i32)> = l.r.sym("png_save_int_32");
    let cs16: libloading::Symbol<unsafe extern "C-unwind" fn(*mut u8, u32)> = l.c.sym("png_save_uint_16");
    let rs16: libloading::Symbol<unsafe extern "C-unwind" fn(*mut u8, u32)> = l.r.sym("png_save_uint_16");

    let mut vals: Vec<u32> = vec![0, 1, 0xffff, 0x10000, 0x7fffffff, 0x80000000, 0xffffffff];
    let mut s: u32 = 0xabcd_1234;
    for _ in 0..512 {
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        vals.push(s);
    }
    for v in vals {
        let (mut a, mut b) = ([0u8; 8], [0u8; 8]);
        unsafe { cs32(a.as_mut_ptr(), v) };
        unsafe { rs32(b.as_mut_ptr(), v) };
        assert_eq!(a, b, "save_uint_32({v:#x})");
        let (mut a, mut b) = ([0u8; 8], [0u8; 8]);
        unsafe { csi(a.as_mut_ptr(), v as i32) };
        unsafe { rsi(b.as_mut_ptr(), v as i32) };
        assert_eq!(a, b, "save_int_32({v:#x})");
        let (mut a, mut b) = ([0u8; 8], [0u8; 8]);
        unsafe { cs16(a.as_mut_ptr(), v) };
        unsafe { rs16(b.as_mut_ptr(), v) };
        assert_eq!(a, b, "save_uint_16({v:#x})");
    }
}

#[test]
fn sig_cmp() {
    let l = libs();
    let f: libloading::Symbol<unsafe extern "C-unwind" fn(*const u8, usize, usize) -> c_int> =
        l.c.sym("png_sig_cmp");
    let g: libloading::Symbol<unsafe extern "C-unwind" fn(*const u8, usize, usize) -> c_int> =
        l.r.sym("png_sig_cmp");
    let good: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    let mut sigs: Vec<[u8; 8]> = vec![good, [0; 8], [137, 80, 78, 71, 13, 10, 26, 11]];
    for i in 0..8 {
        let mut s = good;
        s[i] ^= 0xff;
        sigs.push(s);
    }
    for s in &sigs {
        for start in 0..9usize {
            for num in 0..9usize {
                assert_eq!(
                    unsafe { f(s.as_ptr(), start, num) },
                    unsafe { g(s.as_ptr(), start, num) },
                    "sig_cmp({s:?},{start},{num})"
                );
            }
        }
    }
}

/* -------------------------------------------------------------- png_safecat */

#[test]
fn safecat() {
    let l = libs();
    let f: libloading::Symbol<unsafe extern "C-unwind" fn(*mut c_char, usize, usize, *const c_char) -> usize> =
        l.c.sym("png_safecat");
    let g: libloading::Symbol<unsafe extern "C-unwind" fn(*mut c_char, usize, usize, *const c_char) -> usize> =
        l.r.sym("png_safecat");
    let strings = ["", "a", "hello", "0123456789", "a longer string than the buffer"];
    for s in strings {
        let cstr = cs(s);
        for bufsize in [1usize, 2, 5, 8, 16, 32] {
            for pos in 0..bufsize.min(8) + 2 {
                let mut a = vec![0x5au8; 64];
                let mut b = vec![0x5au8; 64];
                let ra = unsafe { f(a.as_mut_ptr() as *mut c_char, bufsize, pos, cstr.as_ptr()) };
                let rb = unsafe { g(b.as_mut_ptr() as *mut c_char, bufsize, pos, cstr.as_ptr()) };
                assert_eq!(ra, rb, "safecat ret {s:?} {bufsize} {pos}");
                assert_eq!(a, b, "safecat buf {s:?} {bufsize} {pos}");
            }
        }
    }
}

/* --------------------------------------------------------- format_number */

#[test]
fn format_number() {
    let l = libs();
    type F = unsafe extern "C-unwind" fn(*const c_char, *mut c_char, c_int, usize) -> *mut c_char;
    let f: libloading::Symbol<F> = l.c.sym("png_format_number");
    let g: libloading::Symbol<F> = l.r.sym("png_format_number");
    let numbers: Vec<usize> = vec![
        0,
        1,
        9,
        10,
        99,
        100,
        12345,
        0xdeadbeef,
        u32::MAX as usize,
        1 << 40,
        usize::MAX,
        (-1i64) as usize,
        (-100000i64) as usize,
        50000,
        100000,
        1000000,
    ];
    for &n in &numbers {
        for format in 1..=5 {
            for bufsize in [4usize, 8, 12, 24] {
                let mut a = vec![0x5au8; 32];
                let mut b = vec![0x5au8; 32];
                let sa = a.as_mut_ptr() as *mut c_char;
                let sb = b.as_mut_ptr() as *mut c_char;
                let pa = unsafe { f(sa, sa.add(bufsize), format, n) };
                let pb = unsafe { g(sb, sb.add(bufsize), format, n) };
                let oa = pa as usize - sa as usize;
                let ob = pb as usize - sb as usize;
                assert_eq!(oa, ob, "format_number ret {n} fmt{format} buf{bufsize}");
                assert_eq!(a, b, "format_number buf {n} fmt{format} buf{bufsize}");
            }
        }
    }
}

/* ------------------------------------------------------- fp number parsing */

#[test]
fn check_fp_number_and_string() {
    let l = libs();
    type Fnum = unsafe extern "C-unwind" fn(*const c_char, usize, *mut c_int, *mut usize) -> c_int;
    type Fstr = unsafe extern "C-unwind" fn(*const c_char, usize) -> c_int;
    let cn: libloading::Symbol<Fnum> = l.c.sym("png_check_fp_number");
    let rn: libloading::Symbol<Fnum> = l.r.sym("png_check_fp_number");
    let ck: libloading::Symbol<Fstr> = l.c.sym("png_check_fp_string");
    let rk: libloading::Symbol<Fstr> = l.r.sym("png_check_fp_string");

    let cases = [
        "", "0", "-0", "+0", "1", "1.", ".1", "1.5", "-1.5", "+1.5", "1e5", "1E5", "1e+5", "1e-5",
        "1.5e-5", ".e5", "e5", "1e", "1e+", "--1", "1..2", "1.2.3", "0.0", "00", "0e0", "abc",
        "1 2", " 1", "1 ", "1.0e10", "12345678901234567890", "-.5e-5", "+.5E+5", "1e5x", "..",
        ".", "-", "+", "1.5e", "0.000", "-0.0", "1e1000",
    ];
    for s in cases {
        let cstr = cs(s);
        for size in 0..=s.len() + 1 {
            for init_state in [0, 1, 2, 3, 4, 8, 16, 32, 64, 128, 256] {
                let (mut sa, mut sb) = (init_state as c_int, init_state as c_int);
                let (mut wa, mut wb) = (0usize, 0usize);
                let ra = unsafe { cn(cstr.as_ptr(), size, &mut sa, &mut wa) };
                let rb = unsafe { rn(cstr.as_ptr(), size, &mut sb, &mut wb) };
                assert_eq!(
                    (ra, sa, wa),
                    (rb, sb, wb),
                    "check_fp_number({s:?}, size={size}, state={init_state})"
                );
            }
            assert_eq!(
                unsafe { ck(cstr.as_ptr(), size) },
                unsafe { rk(cstr.as_ptr(), size) },
                "check_fp_string({s:?}, {size})"
            );
        }
    }
}

/* ---------------------------------------------------------- fixed point math */

#[test]
fn muldiv_reciprocal_gamma() {
    let l = libs();
    type Fmd = unsafe extern "C-unwind" fn(*mut i32, i32, i32, i32) -> c_int;
    let cmd: libloading::Symbol<Fmd> = l.c.sym("png_muldiv");
    let rmd: libloading::Symbol<Fmd> = l.r.sym("png_muldiv");
    type F1 = unsafe extern "C-unwind" fn(i32) -> i32;
    let cr: libloading::Symbol<F1> = l.c.sym("png_reciprocal");
    let rr: libloading::Symbol<F1> = l.r.sym("png_reciprocal");
    type F2 = unsafe extern "C-unwind" fn(i32, i32) -> i32;
    let cr2: libloading::Symbol<F2> = l.c.sym("png_reciprocal2");
    let rr2: libloading::Symbol<F2> = l.r.sym("png_reciprocal2");
    type Fs = unsafe extern "C-unwind" fn(i32) -> c_int;
    let cgs: libloading::Symbol<Fs> = l.c.sym("png_gamma_significant");
    let rgs: libloading::Symbol<Fs> = l.r.sym("png_gamma_significant");
    type F8 = unsafe extern "C-unwind" fn(c_uint, i32) -> u8;
    let c8: libloading::Symbol<F8> = l.c.sym("png_gamma_8bit_correct");
    let r8: libloading::Symbol<F8> = l.r.sym("png_gamma_8bit_correct");
    type F16 = unsafe extern "C-unwind" fn(c_uint, i32) -> u16;
    let c16: libloading::Symbol<F16> = l.c.sym("png_gamma_16bit_correct");
    let r16: libloading::Symbol<F16> = l.r.sym("png_gamma_16bit_correct");

    let interesting: Vec<i32> = vec![
        0, 1, -1, 2, -2, 100, -100, 5000, -5000, 10000, -10000, 45455, 100000, -100000, 65536,
        1 << 20, 1 << 30, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 32768, 2147483, 21474,
        -21474, 214748, 500000, 1000000,
    ];

    for &a in &interesting {
        assert_eq!(unsafe { cr(a) }, unsafe { rr(a) }, "reciprocal({a})");
        assert_eq!(unsafe { cgs(a) }, unsafe { rgs(a) }, "gamma_significant({a})");
        for &b in &interesting {
            assert_eq!(unsafe { cr2(a, b) }, unsafe { rr2(a, b) }, "reciprocal2({a},{b})");
        }
    }

    let mut s: u32 = 7;
    let mut rnd = move || {
        s = s.wrapping_mul(1103515245).wrapping_add(12345);
        s as i32
    };
    let mut trials: Vec<(i32, i32, i32)> = Vec::new();
    for &a in &interesting {
        for &b in &interesting {
            trials.push((a, b, 100000));
            trials.push((a, 100000, b));
        }
    }
    for _ in 0..3000 {
        trials.push((rnd(), rnd(), rnd()));
    }
    for (a, b, c) in trials {
        let (mut ra, mut rb) = (0x5555_5555i32, 0x5555_5555i32);
        let ok_a = unsafe { cmd(&mut ra, a, b, c) };
        let ok_b = unsafe { rmd(&mut rb, a, b, c) };
        assert_eq!(ok_a, ok_b, "muldiv ret ({a},{b},{c})");
        assert_eq!(ra, rb, "muldiv out ({a},{b},{c}) ok={ok_a}");
    }

    // gamma correction over the full 8-bit domain and a sampled 16-bit domain
    let gammas: Vec<i32> = vec![
        0, 1, 100, 1000, 5000, 10000, 20000, 22222, 45455, 50000, 100000, 200000, 250000, 1000000,
        -10000, i32::MAX,
    ];
    for &gv in &gammas {
        for v in 0u32..256 {
            assert_eq!(unsafe { c8(v, gv) }, unsafe { r8(v, gv) }, "gamma8({v},{gv})");
        }
        for v in (0u32..=65535).step_by(97) {
            assert_eq!(unsafe { c16(v, gv) }, unsafe { r16(v, gv) }, "gamma16({v},{gv})");
        }
        for v in [0u32, 1, 2, 3, 32767, 32768, 65534, 65535] {
            assert_eq!(unsafe { c16(v, gv) }, unsafe { r16(v, gv) }, "gamma16({v},{gv})");
        }
    }
}

use std::ffi::c_uint;

/* -------------------------------------------------------- colorspace math */

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct png_xy {
    redx: i32,
    redy: i32,
    greenx: i32,
    greeny: i32,
    bluex: i32,
    bluey: i32,
    whitex: i32,
    whitey: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct png_XYZ {
    red_X: i32,
    red_Y: i32,
    red_Z: i32,
    green_X: i32,
    green_Y: i32,
    green_Z: i32,
    blue_X: i32,
    blue_Y: i32,
    blue_Z: i32,
}

#[test]
fn xyz_conversions() {
    let l = libs();
    type FA = unsafe extern "C-unwind" fn(*mut png_XYZ, *const png_xy) -> c_int;
    type FB = unsafe extern "C-unwind" fn(*mut png_xy, *const png_XYZ) -> c_int;
    let ca: libloading::Symbol<FA> = l.c.sym("png_XYZ_from_xy");
    let ra: libloading::Symbol<FA> = l.r.sym("png_XYZ_from_xy");
    let cb: libloading::Symbol<FB> = l.c.sym("png_xy_from_XYZ");
    let rb: libloading::Symbol<FB> = l.r.sym("png_xy_from_XYZ");

    let srgb = png_xy {
        redx: 64000,
        redy: 33000,
        greenx: 30000,
        greeny: 60000,
        bluex: 15000,
        bluey: 6000,
        whitex: 31270,
        whitey: 32900,
    };
    let mut cases = vec![srgb, png_xy::default()];
    // degenerate / extreme inputs
    cases.push(png_xy { redx: 100000, redy: 0, ..srgb });
    cases.push(png_xy { whitey: 0, ..srgb });
    cases.push(png_xy { redx: 1, redy: 1, greenx: 1, greeny: 1, bluex: 1, bluey: 1, whitex: 1, whitey: 1 });
    cases.push(png_xy { redx: i32::MAX, redy: i32::MIN, ..srgb });
    let mut s: u32 = 99;
    let mut rnd = move || {
        s = s.wrapping_mul(1103515245).wrapping_add(12345);
        (s >> 8) as i32
    };
    for _ in 0..300 {
        cases.push(png_xy {
            redx: rnd() % 120000,
            redy: rnd() % 120000,
            greenx: rnd() % 120000,
            greeny: rnd() % 120000,
            bluex: rnd() % 120000,
            bluey: rnd() % 120000,
            whitex: rnd() % 120000,
            whitey: rnd() % 120000,
        });
    }

    for xy in &cases {
        let mut a = png_XYZ { red_X: 0x5555, ..Default::default() };
        let mut b = png_XYZ { red_X: 0x5555, ..Default::default() };
        let oka = unsafe { ca(&mut a, xy) };
        let okb = unsafe { ra(&mut b, xy) };
        assert_eq!(oka, okb, "XYZ_from_xy ret {xy:?}");
        assert_eq!(a, b, "XYZ_from_xy out {xy:?}");

        if oka == 0 {
            let mut p = png_xy { redx: 0x5555, ..Default::default() };
            let mut q = png_xy { redx: 0x5555, ..Default::default() };
            let oa = unsafe { cb(&mut p, &a) };
            let ob = unsafe { rb(&mut q, &b) };
            assert_eq!(oa, ob, "xy_from_XYZ ret {a:?}");
            assert_eq!(p, q, "xy_from_XYZ out {a:?}");
        }
    }

    // direct xy_from_XYZ fuzz
    let mut s2: u32 = 12345;
    let mut rnd2 = move || {
        s2 = s2.wrapping_mul(1664525).wrapping_add(1013904223);
        (s2 >> 8) as i32 % 200000
    };
    for _ in 0..300 {
        let xyz = png_XYZ {
            red_X: rnd2(),
            red_Y: rnd2(),
            red_Z: rnd2(),
            green_X: rnd2(),
            green_Y: rnd2(),
            green_Z: rnd2(),
            blue_X: rnd2(),
            blue_Y: rnd2(),
            blue_Z: rnd2(),
        };
        let mut p = png_xy::default();
        let mut q = png_xy::default();
        let oa = unsafe { cb(&mut p, &xyz) };
        let ob = unsafe { rb(&mut q, &xyz) };
        assert_eq!((oa, p), (ob, q), "xy_from_XYZ {xyz:?}");
    }
}

/* ------------------------------------------------------------ sRGB tables */

#[test]
fn srgb_tables() {
    let l = libs();
    unsafe {
        let a: libloading::Symbol<*const u16> = l.c.sym("png_sRGB_table");
        let b: libloading::Symbol<*const u16> = l.r.sym("png_sRGB_table");
        let sa = std::slice::from_raw_parts(*a, 256);
        let sb = std::slice::from_raw_parts(*b, 256);
        assert_eq!(sa, sb, "png_sRGB_table");

        let a: libloading::Symbol<*const u16> = l.c.sym("png_sRGB_base");
        let b: libloading::Symbol<*const u16> = l.r.sym("png_sRGB_base");
        let sa = std::slice::from_raw_parts(*a, 512);
        let sb = std::slice::from_raw_parts(*b, 512);
        assert_eq!(sa, sb, "png_sRGB_base");

        let a: libloading::Symbol<*const u8> = l.c.sym("png_sRGB_delta");
        let b: libloading::Symbol<*const u8> = l.r.sym("png_sRGB_delta");
        let sa = std::slice::from_raw_parts(*a, 512);
        let sb = std::slice::from_raw_parts(*b, 512);
        assert_eq!(sa, sb, "png_sRGB_delta");
    }
}

/* ----------------------------------------------------- grayscale palette */

#[test]
fn build_grayscale_palette() {
    let l = libs();
    type F = unsafe extern "C-unwind" fn(c_int, *mut png_color);
    let f: libloading::Symbol<F> = l.c.sym("png_build_grayscale_palette");
    let g: libloading::Symbol<F> = l.r.sym("png_build_grayscale_palette");
    for depth in [-1, 0, 1, 2, 3, 4, 5, 8, 16, 32] {
        let mut a = vec![png_color { red: 0x5a, green: 0x5a, blue: 0x5a }; 256];
        let mut b = a.clone();
        unsafe { f(depth, a.as_mut_ptr()) };
        unsafe { g(depth, b.as_mut_ptr()) };
        assert_eq!(a, b, "build_grayscale_palette({depth})");
    }
    // NULL palette must be tolerated identically
    unsafe { f(8, std::ptr::null_mut()) };
    unsafe { g(8, std::ptr::null_mut()) };
}

/* ------------------------------------------------------------ time helpers */

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    tm_gmtoff: c_long,
    tm_zone: *const c_char,
}

#[test]
fn time_conversions() {
    let l = libs();
    type Ftm = unsafe extern "C-unwind" fn(*mut png_time, *const Tm);
    let ctm: libloading::Symbol<Ftm> = l.c.sym("png_convert_from_struct_tm");
    let rtm: libloading::Symbol<Ftm> = l.r.sym("png_convert_from_struct_tm");
    type Ftt = unsafe extern "C-unwind" fn(*mut png_time, i64);
    let ctt: libloading::Symbol<Ftt> = l.c.sym("png_convert_from_time_t");
    let rtt: libloading::Symbol<Ftt> = l.r.sym("png_convert_from_time_t");
    type Frfc = unsafe extern "C-unwind" fn(*mut c_char, *const png_time) -> c_int;
    let crfc: libloading::Symbol<Frfc> = l.c.sym("png_convert_to_rfc1123_buffer");
    let rrfc: libloading::Symbol<Frfc> = l.r.sym("png_convert_to_rfc1123_buffer");

    let tms = [
        Tm { tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 1, tm_mon: 0, tm_year: 70, ..Default::default() },
        Tm { tm_sec: 59, tm_min: 59, tm_hour: 23, tm_mday: 31, tm_mon: 11, tm_year: 125, ..Default::default() },
        Tm { tm_sec: 30, tm_min: 15, tm_hour: 12, tm_mday: 15, tm_mon: 5, tm_year: 100, ..Default::default() },
        Tm { tm_sec: 61, tm_min: 70, tm_hour: 30, tm_mday: 40, tm_mon: 20, tm_year: 300, ..Default::default() },
    ];
    for t in &tms {
        let (mut a, mut b) = (png_time::default(), png_time::default());
        unsafe { ctm(&mut a, t) };
        unsafe { rtm(&mut b, t) };
        assert_eq!(a, b, "convert_from_struct_tm");
    }

    for tt in [0i64, 1, 1_000_000_000, 2_000_000_000, 951_827_696, -1, 4_000_000_000] {
        let (mut a, mut b) = (png_time::default(), png_time::default());
        unsafe { ctt(&mut a, tt) };
        unsafe { rtt(&mut b, tt) };
        assert_eq!(a, b, "convert_from_time_t({tt})");
    }

    let times = [
        png_time { year: 2025, month: 8, day: 31, hour: 4, minute: 5, second: 6 },
        png_time { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 65535, month: 13, day: 32, hour: 24, minute: 60, second: 61 },
        png_time { year: 1970, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 1999, month: 12, day: 31, hour: 23, minute: 59, second: 59 },
        png_time { year: 100, month: 6, day: 15, hour: 12, minute: 30, second: 0 },
    ];
    for t in &times {
        let mut a = vec![0x5au8; 40];
        let mut b = vec![0x5au8; 40];
        let ra = unsafe { crfc(a.as_mut_ptr() as *mut c_char, t) };
        let rb = unsafe { rrfc(b.as_mut_ptr() as *mut c_char, t) };
        assert_eq!(ra, rb, "rfc1123 ret {t:?}");
        assert_eq!(a, b, "rfc1123 buf {t:?}: {:?} vs {:?}", hex(&a), hex(&b));
    }
    // NULL out buffer is explicitly handled by both; a NULL png_time is
    // dereferenced unconditionally by the C code, so it is not exercised.
    let t = times[0];
    assert_eq!(
        unsafe { crfc(std::ptr::null_mut(), &t) },
        unsafe { rrfc(std::ptr::null_mut(), &t) }
    );
}
