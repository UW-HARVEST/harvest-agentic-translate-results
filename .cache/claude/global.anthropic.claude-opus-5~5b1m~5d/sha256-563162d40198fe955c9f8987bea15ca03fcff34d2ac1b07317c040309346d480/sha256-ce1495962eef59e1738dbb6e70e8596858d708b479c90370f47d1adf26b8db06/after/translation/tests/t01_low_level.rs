//! Phase B, lowest layer: the pure / near-pure exported helpers.
//!
//! Every call goes through `libloading` into the two `.so` files, never into
//! the Rust crate directly.
mod common;

use common::*;
use std::ffi::{c_char, c_double, c_int, c_long, c_uint, c_void};

// ---------------------------------------------------------------------------
// byte-order helpers: png_get_uint_32 / _31 / _16, png_get_int_32,
//                     png_save_uint_32 / _16, png_save_int_32
// ---------------------------------------------------------------------------

type FnGetU32 = unsafe extern "C" fn(*const u8) -> u32;
type FnGetU16 = unsafe extern "C" fn(*const u8) -> u16;
type FnGetI32 = unsafe extern "C" fn(*const u8) -> i32;
type FnSaveU32 = unsafe extern "C" fn(*mut u8, u32);
type FnSaveU16 = unsafe extern "C" fn(*mut u8, u16);
type FnSaveI32 = unsafe extern "C" fn(*mut u8, i32);

#[test]
fn get_uint_32_16_int32() {
    let (cg32, rg32) = both::<FnGetU32>("png_get_uint_32");
    let (cg16, rg16) = both::<FnGetU16>("png_get_uint_16");
    let (cgi32, rgi32) = both::<FnGetI32>("png_get_int_32");

    let mut rng = Rng::new(0x1001);
    // exhaustive-ish over interesting patterns plus 20000 random ones
    let mut cases: Vec<[u8; 4]> = Vec::new();
    for a in [0u8, 1, 0x7f, 0x80, 0xfe, 0xff] {
        for b in [0u8, 1, 0x80, 0xff] {
            for c in [0u8, 0x55, 0xaa, 0xff] {
                for d in [0u8, 1, 0x80, 0xff] {
                    cases.push([a, b, c, d]);
                }
            }
        }
    }
    for _ in 0..20000 {
        cases.push([rng.next_u8(), rng.next_u8(), rng.next_u8(), rng.next_u8()]);
    }

    for buf in &cases {
        unsafe {
            eq_dbg(
                &format!("png_get_uint_32({buf:02x?})"),
                cg32(buf.as_ptr()),
                rg32(buf.as_ptr()),
            );
            eq_dbg(
                &format!("png_get_uint_16({buf:02x?})"),
                cg16(buf.as_ptr()),
                rg16(buf.as_ptr()),
            );
            eq_dbg(
                &format!("png_get_int_32({buf:02x?})"),
                cgi32(buf.as_ptr()),
                rgi32(buf.as_ptr()),
            );
        }
    }
}

#[test]
fn save_uint_32_16_int32() {
    let (cs32, rs32) = both::<FnSaveU32>("png_save_uint_32");
    let (cs16, rs16) = both::<FnSaveU16>("png_save_uint_16");
    let (csi32, rsi32) = both::<FnSaveI32>("png_save_int_32");

    let mut rng = Rng::new(0x1002);
    for i in 0..20000u32 {
        let v = if i < 32 {
            [0u32, 1, 2, 0x7f, 0x80, 0xff, 0x100, 0x7fff, 0x8000, 0xffff, 0x1_0000,
             0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0x7fff_fffe, 0x8000_0001][(i % 16) as usize]
        } else {
            rng.interesting_u32()
        };
        let mut a = [0xAAu8; 4];
        let mut b = [0xAAu8; 4];
        unsafe {
            cs32(a.as_mut_ptr(), v);
            rs32(b.as_mut_ptr(), v);
        }
        eq_bytes(&format!("png_save_uint_32({v:#x})"), &a, &b);

        let mut a = [0xAAu8; 4];
        let mut b = [0xAAu8; 4];
        unsafe {
            cs16(a.as_mut_ptr(), v as u16);
            rs16(b.as_mut_ptr(), v as u16);
        }
        eq_bytes(&format!("png_save_uint_16({:#x})", v as u16), &a, &b);

        let mut a = [0xAAu8; 4];
        let mut b = [0xAAu8; 4];
        unsafe {
            csi32(a.as_mut_ptr(), v as i32);
            rsi32(b.as_mut_ptr(), v as i32);
        }
        eq_bytes(&format!("png_save_int_32({})", v as i32), &a, &b);
    }
}

// ---------------------------------------------------------------------------
// png_sig_cmp
// ---------------------------------------------------------------------------

type FnSigCmp = unsafe extern "C" fn(*const u8, usize, usize) -> c_int;

#[test]
fn sig_cmp() {
    let (c, r) = both::<FnSigCmp>("png_sig_cmp");
    let mut rng = Rng::new(0x1003);

    let good = pngbuild::PNG_SIG;
    let mut cases: Vec<Vec<u8>> = Vec::new();
    cases.push(good.to_vec());
    cases.push(vec![]);
    for i in 0..8 {
        let mut s = good.to_vec();
        s[i] ^= 0xff;
        cases.push(s);
    }
    for _ in 0..3000 {
        let n = rng.below(10) as usize;
        cases.push(rng.bytes(n));
    }
    // near-misses
    for _ in 0..3000 {
        let mut s = good.to_vec();
        let i = rng.below(8) as usize;
        s[i] = rng.next_u8();
        cases.push(s);
    }

    for sig in &cases {
        for start in [0usize, 1, 2, 4, 7, 8, 9, 100] {
            for num in [0usize, 1, 2, 3, 4, 8, 9, 100] {
                // Only pass pointers that the library will not read past; libpng
                // reads sig[start..start+num] clamped internally to 8 bytes, so
                // give it a padded 32-byte buffer.
                let mut buf = [0u8; 32];
                let n = sig.len().min(32);
                buf[..n].copy_from_slice(&sig[..n]);
                unsafe {
                    let a = c(buf.as_ptr(), start, num);
                    let b = r(buf.as_ptr(), start, num);
                    eq_dbg(
                        &format!("png_sig_cmp({:02x?}, {start}, {num})", &buf[..8]),
                        a,
                        b,
                    );
                }
            }
        }
    }
    // `sig == NULL` is *not* a testable input: the C reaches
    // `memcmp(&sig[start], ...)` and dereferences it (png.c:96).  Only the
    // start/num_to_check rejections above are real rejections.  `num_to_check`
    // of 0 and `start > 7` are covered by the loops and both return -1 without
    // touching `sig`, which the following checks confirm with a NULL pointer.
    unsafe {
        eq_dbg(
            "png_sig_cmp(NULL,0,0)",
            c(std::ptr::null(), 0, 0),
            r(std::ptr::null(), 0, 0),
        );
        eq_dbg(
            "png_sig_cmp(NULL,8,8)",
            c(std::ptr::null(), 8, 8),
            r(std::ptr::null(), 8, 8),
        );
        eq_dbg(
            "png_sig_cmp(NULL,usize::MAX,4)",
            c(std::ptr::null(), usize::MAX, 4),
            r(std::ptr::null(), usize::MAX, 4),
        );
    }
}

// ---------------------------------------------------------------------------
// version / copyright strings
// ---------------------------------------------------------------------------

type FnU32Void = unsafe extern "C" fn() -> u32;
type FnStrP = unsafe extern "C" fn(*const c_void) -> *const c_char;

#[test]
fn version_and_strings() {
    let (c, r) = both::<FnU32Void>("png_access_version_number");
    unsafe { eq_dbg("png_access_version_number", c(), r()) };

    for name in [
        "png_get_copyright",
        "png_get_header_ver",
        "png_get_header_version",
        "png_get_libpng_ver",
    ] {
        let (c, r) = both::<FnStrP>(name);
        unsafe {
            let a = cstr_to_string(c(std::ptr::null()));
            let b = cstr_to_string(r(std::ptr::null()));
            eq_dbg(name, a, b);
        }
    }
}

// ---------------------------------------------------------------------------
// png_get_uint_31 (needs a png_struct only for the error path; valid values
// return normally)
// ---------------------------------------------------------------------------

type FnGetU31 = unsafe extern "C" fn(png_structp, *const u8) -> u32;

#[test]
fn get_uint_31_valid() {
    let (c, r) = both::<FnGetU31>("png_get_uint_31");
    let mut rng = Rng::new(0x1004);
    for _ in 0..20000 {
        // top bit clear => valid, no png_error
        let v = rng.next_u32() & 0x7fff_ffff;
        let buf = v.to_be_bytes();
        unsafe {
            eq_dbg(
                &format!("png_get_uint_31({v:#x})"),
                c(std::ptr::null_mut(), buf.as_ptr()),
                r(std::ptr::null_mut(), buf.as_ptr()),
            );
        }
    }
    for v in [0u32, 1, 0x7fff_fffe, 0x7fff_ffff] {
        let buf = v.to_be_bytes();
        unsafe {
            eq_dbg(
                &format!("png_get_uint_31({v:#x})"),
                c(std::ptr::null_mut(), buf.as_ptr()),
                r(std::ptr::null_mut(), buf.as_ptr()),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// png_muldiv / png_reciprocal / png_reciprocal2
// ---------------------------------------------------------------------------

type FnMuldiv = unsafe extern "C" fn(*mut i32, i32, i32, i32) -> c_int;
type FnRecip = unsafe extern "C" fn(i32) -> i32;
type FnRecip2 = unsafe extern "C" fn(i32, i32) -> i32;

#[test]
fn muldiv() {
    let (c, r) = both::<FnMuldiv>("png_muldiv");
    let mut rng = Rng::new(0x1005);
    let specials: [i32; 14] = [
        0,
        1,
        -1,
        2,
        -2,
        100000,
        -100000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        65535,
        0x10000,
        -65536,
    ];
    let mut do_case = |a: i32, m: i32, d: i32| {
        let mut ra: i32 = 0x5555_5555;
        let mut rb: i32 = 0x5555_5555;
        unsafe {
            let ok_c = c(&mut ra, a, m, d);
            let ok_r = r(&mut rb, a, m, d);
            eq_dbg(&format!("png_muldiv({a},{m},{d}).ret"), ok_c, ok_r);
            // libpng only stores through res when it returns 1
            if ok_c != 0 {
                eq_dbg(&format!("png_muldiv({a},{m},{d}).res"), ra, rb);
            }
        }
    };
    for &a in &specials {
        for &m in &specials {
            for &d in &specials {
                do_case(a, m, d);
            }
        }
    }
    for _ in 0..40000 {
        do_case(
            rng.next_u32() as i32,
            rng.next_u32() as i32,
            rng.next_u32() as i32,
        );
    }
    // small magnitudes exercise the exact path
    for _ in 0..40000 {
        do_case(
            (rng.below(400001) as i32) - 200000,
            (rng.below(400001) as i32) - 200000,
            (rng.below(400001) as i32) - 200000,
        );
    }
}

#[test]
fn reciprocals() {
    let (c1, r1) = both::<FnRecip>("png_reciprocal");
    let (c2, r2) = both::<FnRecip2>("png_reciprocal2");
    let mut rng = Rng::new(0x1006);
    let mut vals: Vec<i32> = vec![0, 1, -1, 2, -2, 100000, -100000, i32::MAX, i32::MIN, 45455, 1000000];
    for _ in 0..8000 {
        vals.push(rng.next_u32() as i32);
    }
    for _ in 0..8000 {
        vals.push(rng.below(1_000_001) as i32);
    }
    for &a in &vals {
        unsafe { eq_dbg(&format!("png_reciprocal({a})"), c1(a), r1(a)) };
    }
    for i in 0..vals.len() {
        let a = vals[i];
        let b = vals[vals.len() - 1 - i];
        unsafe { eq_dbg(&format!("png_reciprocal2({a},{b})"), c2(a, b), r2(a, b)) };
    }
}

// ---------------------------------------------------------------------------
// gamma helpers
// ---------------------------------------------------------------------------

type FnGammaSig = unsafe extern "C" fn(i32) -> c_int;
type FnGamma16 = unsafe extern "C" fn(c_uint, i32) -> u16;
type FnGamma8 = unsafe extern "C" fn(c_uint, i32) -> u8;

#[test]
fn gamma_helpers() {
    let (cs, rs) = both::<FnGammaSig>("png_gamma_significant");
    let (c16, r16) = both::<FnGamma16>("png_gamma_16bit_correct");
    let (c8, r8) = both::<FnGamma8>("png_gamma_8bit_correct");
    let mut rng = Rng::new(0x1007);

    let mut gammas: Vec<i32> = vec![
        0, 1, -1, 100000, 99999, 100001, 45455, 45454, 220000, 50000, 200000, i32::MAX, i32::MIN,
        99000, 101000, 99001, 100999,
    ];
    for _ in 0..4000 {
        gammas.push(rng.next_u32() as i32);
    }
    for _ in 0..4000 {
        gammas.push(rng.below(300_001) as i32);
    }
    for &g in &gammas {
        unsafe { eq_dbg(&format!("png_gamma_significant({g})"), cs(g), rs(g)) };
    }

    // png_gamma_16bit_correct / _8bit_correct: only sane gamma values (>0) and
    // in-range sample values are meaningful; libpng asserts nothing so all
    // values are legal inputs -- compare them all.
    let sane: Vec<i32> = gammas
        .iter()
        .copied()
        .filter(|g| *g > 0 && *g < 1_000_000)
        .take(400)
        .collect();
    for &g in &sane {
        for v in [0u32, 1, 2, 127, 128, 254, 255, 256, 32767, 32768, 65534, 65535] {
            unsafe {
                eq_dbg(
                    &format!("png_gamma_16bit_correct({v},{g})"),
                    c16(v, g),
                    r16(v, g),
                );
            }
            if v <= 255 {
                unsafe {
                    eq_dbg(
                        &format!("png_gamma_8bit_correct({v},{g})"),
                        c8(v, g),
                        r8(v, g),
                    );
                }
            }
        }
    }
    // full 8-bit sweep for a handful of gammas
    for &g in &[45455i32, 100000, 220000, 50000, 23000, 500000] {
        for v in 0u32..256 {
            unsafe {
                eq_dbg(
                    &format!("png_gamma_8bit_correct({v},{g})"),
                    c8(v, g),
                    r8(v, g),
                );
            }
        }
    }
    // dense 16-bit sweep for a couple of gammas
    for &g in &[45455i32, 220000] {
        for v in (0u32..=65535).step_by(37) {
            unsafe {
                eq_dbg(
                    &format!("png_gamma_16bit_correct({v},{g})"),
                    c16(v, g),
                    r16(v, g),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// colour-space maths: png_XYZ_from_xy / png_xy_from_XYZ
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct png_xy {
    pub redx: i32,
    pub redy: i32,
    pub greenx: i32,
    pub greeny: i32,
    pub bluex: i32,
    pub bluey: i32,
    pub whitex: i32,
    pub whitey: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct png_XYZ {
    pub red_X: i32,
    pub red_Y: i32,
    pub red_Z: i32,
    pub green_X: i32,
    pub green_Y: i32,
    pub green_Z: i32,
    pub blue_X: i32,
    pub blue_Y: i32,
    pub blue_Z: i32,
}

type FnXYZfromxy = unsafe extern "C" fn(*mut png_XYZ, *const png_xy) -> c_int;
type FnxyfromXYZ = unsafe extern "C" fn(*mut png_xy, *const png_XYZ) -> c_int;

#[test]
fn colourspace_maths() {
    let (cf, rf) = both::<FnXYZfromxy>("png_XYZ_from_xy");
    let (cb, rb) = both::<FnxyfromXYZ>("png_xy_from_XYZ");
    let mut rng = Rng::new(0x1008);

    // sRGB primaries (the canonical valid input)
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
    for _ in 0..4000 {
        cases.push(png_xy {
            redx: rng.below(100_001) as i32,
            redy: rng.below(100_001) as i32,
            greenx: rng.below(100_001) as i32,
            greeny: rng.below(100_001) as i32,
            bluex: rng.below(100_001) as i32,
            bluey: rng.below(100_001) as i32,
            whitex: rng.below(100_001) as i32,
            whitey: rng.below(100_001) as i32,
        });
    }
    for _ in 0..4000 {
        cases.push(png_xy {
            redx: rng.next_u32() as i32,
            redy: rng.next_u32() as i32,
            greenx: rng.next_u32() as i32,
            greeny: rng.next_u32() as i32,
            bluex: rng.next_u32() as i32,
            bluey: rng.next_u32() as i32,
            whitex: rng.next_u32() as i32,
            whitey: rng.next_u32() as i32,
        });
    }

    for xy in &cases {
        let mut a = png_XYZ::default();
        let mut b = png_XYZ::default();
        unsafe {
            let ra = cf(&mut a, xy);
            let rb2 = rf(&mut b, xy);
            eq_dbg(&format!("png_XYZ_from_xy({xy:?}).ret"), ra, rb2);
            if ra == 0 {
                eq_dbg(&format!("png_XYZ_from_xy({xy:?}).out"), a, b);
            }
        }
    }

    let mut xyzs = vec![png_XYZ::default()];
    {
        let mut a = png_XYZ::default();
        unsafe { cf(&mut a, &srgb) };
        xyzs.push(a);
    }
    for _ in 0..4000 {
        xyzs.push(png_XYZ {
            red_X: rng.below(200_001) as i32,
            red_Y: rng.below(200_001) as i32,
            red_Z: rng.below(200_001) as i32,
            green_X: rng.below(200_001) as i32,
            green_Y: rng.below(200_001) as i32,
            green_Z: rng.below(200_001) as i32,
            blue_X: rng.below(200_001) as i32,
            blue_Y: rng.below(200_001) as i32,
            blue_Z: rng.below(200_001) as i32,
        });
    }
    for _ in 0..4000 {
        xyzs.push(png_XYZ {
            red_X: rng.next_u32() as i32,
            red_Y: rng.next_u32() as i32,
            red_Z: rng.next_u32() as i32,
            green_X: rng.next_u32() as i32,
            green_Y: rng.next_u32() as i32,
            green_Z: rng.next_u32() as i32,
            blue_X: rng.next_u32() as i32,
            blue_Y: rng.next_u32() as i32,
            blue_Z: rng.next_u32() as i32,
        });
    }
    for xyz in &xyzs {
        let mut a = png_xy::default();
        let mut b = png_xy::default();
        unsafe {
            let ra = cb(&mut a, xyz);
            let rb2 = rb(&mut b, xyz);
            eq_dbg(&format!("png_xy_from_XYZ({xyz:?}).ret"), ra, rb2);
            if ra == 0 {
                eq_dbg(&format!("png_xy_from_XYZ({xyz:?}).out"), a, b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// png_safecat / png_format_number
// ---------------------------------------------------------------------------

type FnSafecat = unsafe extern "C" fn(*mut c_char, usize, usize, *const c_char) -> usize;
type FnFormatNumber =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, usize) -> *mut c_char;

#[test]
fn safecat() {
    let (c, r) = both::<FnSafecat>("png_safecat");
    let mut rng = Rng::new(0x1009);
    let strings: [&[u8]; 7] = [
        b"\0",
        b"a\0",
        b"hello\0",
        b"0123456789abcdef\0",
        b"the quick brown fox jumps over the lazy dog\0",
        b" \0",
        b"\xff\xfe\x01\0",
    ];
    for s in strings {
        for bufsize in [0usize, 1, 2, 5, 8, 16, 32, 64] {
            for pos in [0usize, 1, 2, 5, 7, 8, 15, 16, 31, 63, 64, 100] {
                let mut a = [0x41u8 as c_char; 128];
                let mut b = [0x41u8 as c_char; 128];
                unsafe {
                    let ra = c(a.as_mut_ptr(), bufsize, pos, s.as_ptr() as *const c_char);
                    let rb = r(b.as_mut_ptr(), bufsize, pos, s.as_ptr() as *const c_char);
                    eq_dbg(
                        &format!("png_safecat({:?},{bufsize},{pos}).ret", String::from_utf8_lossy(s)),
                        ra,
                        rb,
                    );
                    let av: Vec<u8> = a.iter().map(|x| *x as u8).collect();
                    let bv: Vec<u8> = b.iter().map(|x| *x as u8).collect();
                    eq_bytes(
                        &format!("png_safecat({:?},{bufsize},{pos}).buf", String::from_utf8_lossy(s)),
                        &av,
                        &bv,
                    );
                }
            }
        }
    }
    // random strings
    for _ in 0..3000 {
        let n = rng.below(30) as usize;
        let mut s: Vec<u8> = (0..n).map(|_| rng.range(1, 255) as u8).collect();
        s.push(0);
        let bufsize = rng.below(40) as usize;
        let pos = rng.below(45) as usize;
        let mut a = [0x41u8 as c_char; 128];
        let mut b = [0x41u8 as c_char; 128];
        unsafe {
            let ra = c(a.as_mut_ptr(), bufsize, pos, s.as_ptr() as *const c_char);
            let rb = r(b.as_mut_ptr(), bufsize, pos, s.as_ptr() as *const c_char);
            eq_dbg("png_safecat.ret", ra, rb);
            let av: Vec<u8> = a.iter().map(|x| *x as u8).collect();
            let bv: Vec<u8> = b.iter().map(|x| *x as u8).collect();
            eq_bytes("png_safecat.buf", &av, &bv);
        }
    }
}

#[test]
fn format_number() {
    let (c, r) = both::<FnFormatNumber>("png_format_number");
    let mut rng = Rng::new(0x100a);
    // PNG_NUMBER_FORMAT_*: u=1 d=1 02u=2 02d=2 x=3 02x=4 (see pngpriv.h)
    let formats: [c_int; 6] = [1, 2, 3, 4, 5, 0];
    let mut nums: Vec<usize> = vec![
        0,
        1,
        9,
        10,
        99,
        100,
        255,
        256,
        65535,
        65536,
        999999,
        1000000,
        u32::MAX as usize,
        usize::MAX,
        usize::MAX / 2,
    ];
    for _ in 0..4000 {
        nums.push(rng.next_u64() as usize);
    }
    for _ in 0..4000 {
        nums.push(rng.below(1_000_000) as usize);
    }

    for &fmt in &formats {
        for &n in &nums {
            for bufsize in [1usize, 2, 4, 8, 16, 24, 32] {
                let mut a = vec![0x41u8 as c_char; 64];
                let mut b = vec![0x41u8 as c_char; 64];
                unsafe {
                    let pa = c(a.as_ptr(), a.as_mut_ptr().add(bufsize), fmt, n);
                    let pb = r(b.as_ptr(), b.as_mut_ptr().add(bufsize), fmt, n);
                    // returned pointer offset within the buffer
                    let oa = pa as usize - a.as_ptr() as usize;
                    let ob = pb as usize - b.as_ptr() as usize;
                    eq_dbg(
                        &format!("png_format_number(fmt={fmt},n={n},bufsize={bufsize}).off"),
                        oa,
                        ob,
                    );
                    let av: Vec<u8> = a.iter().map(|x| *x as u8).collect();
                    let bv: Vec<u8> = b.iter().map(|x| *x as u8).collect();
                    eq_bytes(
                        &format!("png_format_number(fmt={fmt},n={n},bufsize={bufsize}).buf"),
                        &av,
                        &bv,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// png_check_fp_number / png_check_fp_string
// ---------------------------------------------------------------------------

type FnCheckFpNumber =
    unsafe extern "C" fn(*const c_char, usize, *mut c_int, *mut usize) -> c_int;
type FnCheckFpString = unsafe extern "C" fn(*const c_char, usize) -> c_int;

#[test]
fn check_fp() {
    let (cn, rn) = both::<FnCheckFpNumber>("png_check_fp_number");
    let (cs, rs) = both::<FnCheckFpString>("png_check_fp_string");
    let mut rng = Rng::new(0x100b);

    let mut strings: Vec<Vec<u8>> = vec![
        b"1".to_vec(),
        b"0".to_vec(),
        b"-1".to_vec(),
        b"+1".to_vec(),
        b"1.5".to_vec(),
        b"-1.5e10".to_vec(),
        b"1E-10".to_vec(),
        b".5".to_vec(),
        b"5.".to_vec(),
        b"".to_vec(),
        b".".to_vec(),
        b"e".to_vec(),
        b"1e".to_vec(),
        b"1e+".to_vec(),
        b"1e+5".to_vec(),
        b"--1".to_vec(),
        b"1.2.3".to_vec(),
        b"0000000".to_vec(),
        b"1000000000000000000000".to_vec(),
        b"abc".to_vec(),
        b"1abc".to_vec(),
        b" 1".to_vec(),
        b"1 ".to_vec(),
        b"1\0".to_vec(),
        b"-".to_vec(),
        b"+".to_vec(),
        b"1e1e1".to_vec(),
        b"1.e5".to_vec(),
        b"-.0".to_vec(),
        b"+.".to_vec(),
    ];
    let alphabet = b"0123456789.eE+- \0aX";
    for _ in 0..6000 {
        let n = rng.below(12) as usize;
        strings.push((0..n).map(|_| *rng.pick(alphabet)).collect());
    }

    for s in &strings {
        for &size in &[0usize, 1, 2, 3, s.len(), s.len() + 1] {
            if size > s.len() {
                continue;
            }
            for &state0 in &[0i32, 1, 2, 4, 8, 16, 32, 0x7f, -1] {
                let mut st_a = state0;
                let mut st_b = state0;
                let mut wa: usize = 0;
                let mut wb: usize = 0;
                unsafe {
                    let ra = cn(s.as_ptr() as *const c_char, size, &mut st_a, &mut wa);
                    let rb = rn(s.as_ptr() as *const c_char, size, &mut st_b, &mut wb);
                    let d = format!(
                        "png_check_fp_number({:?},{size},state={state0})",
                        String::from_utf8_lossy(s)
                    );
                    eq_dbg(&format!("{d}.ret"), ra, rb);
                    eq_dbg(&format!("{d}.state"), st_a, st_b);
                    eq_dbg(&format!("{d}.whereami"), wa, wb);
                }
            }
            unsafe {
                eq_dbg(
                    &format!(
                        "png_check_fp_string({:?},{size})",
                        String::from_utf8_lossy(s)
                    ),
                    cs(s.as_ptr() as *const c_char, size),
                    rs(s.as_ptr() as *const c_char, size),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// png_ascii_from_fp / png_ascii_from_fixed
// ---------------------------------------------------------------------------

type FnAsciiFromFp =
    unsafe extern "C" fn(png_structp, *mut c_char, usize, c_double, c_uint);
type FnAsciiFromFixed = unsafe extern "C" fn(png_structp, *mut c_char, usize, i32);

#[test]
fn ascii_from_fixed() {
    let (c, r) = both::<FnAsciiFromFixed>("png_ascii_from_fixed");
    let mut rng = Rng::new(0x100c);
    let mut vals: Vec<i32> = vec![
        0, 1, -1, 10, -10, 100000, -100000, 99999, 100001, i32::MAX, i32::MIN, i32::MAX - 1,
        i32::MIN + 1, 45455, 500, -500, 12345678, -12345678,
    ];
    for _ in 0..8000 {
        vals.push(rng.next_u32() as i32);
    }
    for &v in &vals {
        // libpng documents the buffer must be at least PNG_sCAL_MAX_DIGITS+..;
        // use the same generous size for both.
        let mut a = [0x41u8 as c_char; 64];
        let mut b = [0x41u8 as c_char; 64];
        unsafe {
            c(std::ptr::null_mut(), a.as_mut_ptr(), 64, v);
            r(std::ptr::null_mut(), b.as_mut_ptr(), 64, v);
        }
        let av: Vec<u8> = a.iter().map(|x| *x as u8).collect();
        let bv: Vec<u8> = b.iter().map(|x| *x as u8).collect();
        eq_bytes(&format!("png_ascii_from_fixed({v})"), &av, &bv);
    }
}

#[test]
fn ascii_from_fp() {
    let (c, r) = both::<FnAsciiFromFp>("png_ascii_from_fp");
    let mut rng = Rng::new(0x100d);
    let mut vals: Vec<f64> = vec![
        0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e-10,
        1e10,
        1e100,
        1e-100,
        1e300,
        1e-300,
        0.45455,
        2.2,
        1.0 / 3.0,
        123456789.0,
        f64::MIN_POSITIVE,
        f64::MAX,
        -f64::MAX,
        1.0 - f64::EPSILON,
        99999.99999,
    ];
    for _ in 0..6000 {
        // uniform over a wide exponent range
        let m = (rng.next_u32() as f64) / (u32::MAX as f64);
        let e = (rng.below(61) as i32) - 30;
        let sign = if rng.bool() { 1.0 } else { -1.0 };
        vals.push(sign * m * 10f64.powi(e));
    }
    for &v in &vals {
        for precision in [1u32, 2, 3, 5, 6, 8, 10, 15, 17, 20] {
            // buffer sized as libpng requires: precision + 10 at least
            let sz = 64usize;
            let mut a = [0x41u8 as c_char; 64];
            let mut b = [0x41u8 as c_char; 64];
            unsafe {
                c(std::ptr::null_mut(), a.as_mut_ptr(), sz, v, precision);
                r(std::ptr::null_mut(), b.as_mut_ptr(), sz, v, precision);
            }
            let av: Vec<u8> = a.iter().map(|x| *x as u8).collect();
            let bv: Vec<u8> = b.iter().map(|x| *x as u8).collect();
            eq_bytes(
                &format!("png_ascii_from_fp({v:e},precision={precision})"),
                &av,
                &bv,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// png_fixed / png_fixed_ITU  (png_fixed errors on out-of-range input; only the
// in-range domain is exercised here, the error path lives in the error tests)
// ---------------------------------------------------------------------------

type FnFixed = unsafe extern "C" fn(png_structp, c_double, *const c_char) -> i32;
type FnFixedItu = unsafe extern "C" fn(png_structp, c_double, *const c_char) -> u32;

#[test]
fn fixed_conversions() {
    let (c, r) = both::<FnFixed>("png_fixed");
    let (ci, ri) = both::<FnFixedItu>("png_fixed_ITU");
    let mut rng = Rng::new(0x100e);
    let txt = b"t\0";
    // png_fixed multiplies by 100000 and errors if outside [-21474.836, 21474.836]
    let mut vals: Vec<f64> = vec![0.0, 1.0, -1.0, 0.5, -0.5, 2.2, 0.45455, 21474.0, -21474.0,
                                  1e-7, -1e-7, 21474.83, -21474.83];
    for _ in 0..8000 {
        let m = (rng.next_u32() as f64) / (u32::MAX as f64);
        vals.push((m * 2.0 - 1.0) * 21000.0);
    }
    for &v in &vals {
        unsafe {
            eq_dbg(
                &format!("png_fixed({v})"),
                c(std::ptr::null_mut(), v, txt.as_ptr() as *const c_char),
                r(std::ptr::null_mut(), v, txt.as_ptr() as *const c_char),
            );
        }
    }
    // png_fixed_ITU: errors outside [0, 2.14748]
    let mut vals2: Vec<f64> = vec![0.0, 1.0, 0.5, 2.0, 2.14, 2.147, 1e-7];
    for _ in 0..8000 {
        let m = (rng.next_u32() as f64) / (u32::MAX as f64);
        vals2.push(m * 2.1);
    }
    for &v in &vals2 {
        unsafe {
            eq_dbg(
                &format!("png_fixed_ITU({v})"),
                ci(std::ptr::null_mut(), v, txt.as_ptr() as *const c_char),
                ri(std::ptr::null_mut(), v, txt.as_ptr() as *const c_char),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// png_build_grayscale_palette
// ---------------------------------------------------------------------------

type FnBuildGray = unsafe extern "C" fn(c_int, *mut png_color);

#[test]
fn build_grayscale_palette() {
    let (c, r) = both::<FnBuildGray>("png_build_grayscale_palette");
    for bit_depth in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16, 32, 1000] {
        let mut a = [png_color::default(); 256];
        let mut b = [png_color::default(); 256];
        unsafe {
            c(bit_depth, a.as_mut_ptr());
            r(bit_depth, b.as_mut_ptr());
        }
        eq_dbg(
            &format!("png_build_grayscale_palette({bit_depth})"),
            a.to_vec(),
            b.to_vec(),
        );
    }
    // NULL palette must be a no-op in both
    unsafe {
        c(8, std::ptr::null_mut());
        r(8, std::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// time conversion: png_convert_from_time_t / _struct_tm /
//                  png_convert_to_rfc1123_buffer
// ---------------------------------------------------------------------------

type FnFromTimeT = unsafe extern "C" fn(*mut png_time, c_long);
type FnRfc1123Buf = unsafe extern "C" fn(*mut c_char, *const png_time) -> c_int;
type FnRfc1123 = unsafe extern "C" fn(png_structp, *const png_time) -> *mut c_char;

#[test]
fn time_conversions() {
    let (c, r) = both::<FnFromTimeT>("png_convert_from_time_t");
    let (cb, rb) = both::<FnRfc1123Buf>("png_convert_to_rfc1123_buffer");
    let mut rng = Rng::new(0x100f);

    let mut times: Vec<i64> = vec![0, 1, -1, 86399, 86400, 1_000_000_000, 2_000_000_000,
                                   i32::MAX as i64, 951782400, 1234567890];
    for _ in 0..5000 {
        times.push(rng.below(2_000_000_000) as i64);
    }
    for &t in &times {
        let mut a = png_time::default();
        let mut b = png_time::default();
        unsafe {
            c(&mut a, t as c_long);
            r(&mut b, t as c_long);
        }
        eq_dbg(&format!("png_convert_from_time_t({t})"), a, b);

        let mut ba = [0x41u8 as c_char; 40];
        let mut bb = [0x41u8 as c_char; 40];
        unsafe {
            let ra = cb(ba.as_mut_ptr(), &a);
            let rb2 = rb(bb.as_mut_ptr(), &b);
            eq_dbg(&format!("png_convert_to_rfc1123_buffer({a:?}).ret"), ra, rb2);
        }
        let av: Vec<u8> = ba.iter().map(|x| *x as u8).collect();
        let bv: Vec<u8> = bb.iter().map(|x| *x as u8).collect();
        eq_bytes(&format!("png_convert_to_rfc1123_buffer({a:?}).buf"), &av, &bv);
    }

    // completely arbitrary png_time values, including out-of-range fields
    for _ in 0..8000 {
        let t = png_time {
            year: rng.next_u16(),
            month: rng.next_u8(),
            day: rng.next_u8(),
            hour: rng.next_u8(),
            minute: rng.next_u8(),
            second: rng.next_u8(),
        };
        let mut ba = [0x41u8 as c_char; 40];
        let mut bb = [0x41u8 as c_char; 40];
        unsafe {
            let ra = cb(ba.as_mut_ptr(), &t);
            let rb2 = rb(bb.as_mut_ptr(), &t);
            eq_dbg(&format!("png_convert_to_rfc1123_buffer({t:?}).ret"), ra, rb2);
        }
        let av: Vec<u8> = ba.iter().map(|x| *x as u8).collect();
        let bv: Vec<u8> = bb.iter().map(|x| *x as u8).collect();
        eq_bytes(&format!("png_convert_to_rfc1123_buffer({t:?}).buf"), &av, &bv);
    }

    // `out == NULL` is an explicit rejection (png.c:748) and returns 0.
    // `ptime == NULL` is NOT checked by the C — it dereferences ptime->year --
    // so it is not a testable input.
    unsafe {
        let t = png_time::default();
        eq_dbg(
            "png_convert_to_rfc1123_buffer(NULL, &t)",
            cb(std::ptr::null_mut(), &t),
            rb(std::ptr::null_mut(), &t),
        );
    }

    // png_convert_to_rfc1123 with a NULL png_ptr returns NULL in both
    let (c1, r1) = both::<FnRfc1123>("png_convert_to_rfc1123");
    unsafe {
        let t = png_time::default();
        let a = c1(std::ptr::null_mut(), &t);
        let b = r1(std::ptr::null_mut(), &t);
        eq_dbg("png_convert_to_rfc1123(NULL,&t)", a.is_null(), b.is_null());
    }
}
