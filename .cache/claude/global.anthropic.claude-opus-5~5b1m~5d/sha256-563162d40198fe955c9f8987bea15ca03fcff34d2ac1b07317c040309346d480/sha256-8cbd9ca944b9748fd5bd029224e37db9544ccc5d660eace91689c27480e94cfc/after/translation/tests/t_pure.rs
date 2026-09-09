//! Differential tests for the "pure" (side-effect free / self contained)
//! libpng entry points listed in CONFIGS.md rows 1..18.
//!
//! Every test drives BOTH `c_src/build/libpng.so` (the reference) and
//! `target/release/liblibpng.so` (the translation) through `dlsym` with exactly
//! the same inputs and asserts that the return values, every out parameter and
//! the recorded warning/error message log are byte identical.
#![allow(non_snake_case)]
#![allow(clippy::type_complexity)]

mod common;

use common::api;
use common::*;
use libloading::Library;
use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void, CStr};

// ---------------------------------------------------------------- helpers

/// `png_fixed_ITU` returns `png_uint_32` in the C (see png.c), so declare it
/// locally with the exact return type instead of using `api::png_fixed_ITU`.
mod myapi {
    use super::common::*;
    use std::ffi::{c_double, c_int, c_void};

    crate::decl_api! {
        fn png_fixed_ITU(pp: png_structp, fp: c_double, text: png_const_charp) -> png_uint_32;
        fn png_XYZ_from_xy(xyz: *mut c_void, xy: *const c_void) -> c_int;
        fn png_xy_from_XYZ(xy: *mut c_void, xyz: *const c_void) -> c_int;
    }
}

/// The reference `libpng.so` was linked without `-lm`, so `floor`, `pow`,
/// `modf` and friends are undefined in it and are expected to come from the
/// global symbol scope of the process.  The test executable does not
/// necessarily pull in libm, so load it explicitly with RTLD_GLOBAL before any
/// libpng call.  (The translated Rust library has libm as a DT_NEEDED entry,
/// but it is loaded RTLD_LOCAL so that does not help the C library.)
fn ensure_libm() {
    use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
    static LIBM: std::sync::OnceLock<UnixLibrary> = std::sync::OnceLock::new();
    LIBM.get_or_init(|| unsafe {
        UnixLibrary::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL)
            .expect("dlopen libm.so.6 with RTLD_GLOBAL")
    });
}

/// Compare one captured run of the C library against the same run of the Rust
/// library: the message log, whether the library longjmp'd out, and the output.
fn cmp_run<V: PartialEq + std::fmt::Debug>(label: &str, c: &Run<V>, r: &Run<V>) {
    let cl: Vec<String> = c.log.iter().map(|m| m.to_string()).collect();
    let rl: Vec<String> = r.log.iter().map(|m| m.to_string()).collect();
    assert!(
        cl == rl,
        "{label}: message log differs\n  C   : {cl:?}\n  Rust: {rl:?}"
    );
    assert!(
        c.out.is_some() == r.out.is_some(),
        "{label}: png_error (longjmp) mismatch: C errored={}, Rust errored={}\n  log: {cl:?}",
        c.out.is_none(),
        r.out.is_none()
    );
    assert!(
        c.out == r.out,
        "{label}: output differs\n  C   : {:?}\n  Rust: {:?}",
        c.out,
        r.out
    );
}

/// Run `f` for every input against both libraries and compare.
fn diff<K, V, F>(label: &str, inputs: &[K], f: F)
where
    K: std::fmt::Debug,
    V: PartialEq + std::fmt::Debug,
    F: Fn(&'static Library, &K) -> V,
{
    ensure_libm();
    let l = libs();
    let mut nerr = 0usize;
    let mut nlog = 0usize;
    for (i, k) in inputs.iter().enumerate() {
        let c = capture_strict(|| f(&l.c, k));
        let r = capture_strict(|| f(&l.rs, k));
        if c.out.is_none() {
            nerr += 1;
        }
        if !c.log.is_empty() {
            nlog += 1;
        }
        cmp_run(&format!("{label}[{i}] input={k:?}"), &c, &r);
    }
    eprintln!(
        "STATS {label}: {} cases, {} png_error, {} with messages",
        inputs.len(),
        nerr,
        nlog
    );
}

unsafe fn cstr_vec(p: png_const_charp) -> Vec<u8> {
    if p.is_null() {
        b"<null>".to_vec()
    } else {
        CStr::from_ptr(p).to_bytes().to_vec()
    }
}

fn buf_u8(b: &[c_char]) -> Vec<u8> {
    b.iter().map(|&c| c as u8).collect()
}

const FILL: c_char = 0x5a; // sentinel used to fill out-buffers ('Z')

unsafe fn destroy_reader(lib: &Library, pp: png_structp) {
    let mut p = pp;
    api::png_destroy_read_struct(lib, &mut p, std::ptr::null_mut(), std::ptr::null_mut());
}

/// Address of a *data* symbol.  `libloading::Symbol<T>::into_raw()` yields the
/// raw `dlsym` result, i.e. the address of the object itself (for an array that
/// is the address of its first element).
unsafe fn data_sym(lib: &Library, name: &[u8]) -> *const u8 {
    let s: libloading::Symbol<*const u8> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("data symbol {}: {e}", String::from_utf8_lossy(name)));
    s.into_raw().into_raw() as *const u8
}

// ============================================================ row 1
// png_access_version_number / png_get_libpng_ver / png_get_header_ver /
// png_get_header_version / png_get_copyright

#[test]
fn r01_version_numbers_and_strings() {
    ensure_libm();
    // with a NULL png_structp (the C ignores it) and with a real one
    let cases: Vec<bool> = vec![false, true];
    diff("row1", &cases, |lib, &with_pp| unsafe {
        let pp: png_structp = if with_pp {
            api::new_reader(lib)
        } else {
            std::ptr::null_mut()
        };
        let out = (
            api::png_access_version_number(lib),
            cstr_vec(api::png_get_libpng_ver(lib, pp)),
            cstr_vec(api::png_get_header_ver(lib, pp)),
            cstr_vec(api::png_get_header_version(lib, pp)),
            cstr_vec(api::png_get_copyright(lib, pp)),
        );
        if with_pp {
            destroy_reader(lib, pp);
        }
        out
    });

    // and the absolute values, so that a "both wrong the same way" result is
    // still caught for the version.
    let l = libs();
    unsafe {
        assert_eq!(api::png_access_version_number(&l.c), 10659);
        assert_eq!(api::png_access_version_number(&l.rs), 10659);
        // c_src/include/png.h: PNG_LIBPNG_VER_STRING "1.6.59.git"
        assert_eq!(
            cstr_vec(api::png_get_libpng_ver(&l.rs, std::ptr::null_mut())),
            b"1.6.59.git".to_vec()
        );
        // PNG_HEADER_VERSION_STRING plus a trailing PNG_STRING_NEWLINE
        assert_eq!(
            cstr_vec(api::png_get_header_version(&l.rs, std::ptr::null_mut())),
            b" libpng version 1.6.59.git\n\n".to_vec()
        );
    }
}

// ============================================================ row 2
// png_save_uint_32 / png_save_uint_16 / png_save_int_32 and the readers

#[test]
fn r02_save_and_get_uint_roundtrip() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 2);
    let mut vals: Vec<u32> = vec![0, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0xffff, 0x1_0000];
    for _ in 0..4096 {
        vals.push(rng.u32());
    }

    diff("row2", &vals, |lib, &v| unsafe {
        let mut b32 = [0xAAu8; 4];
        api::png_save_uint_32(lib, b32.as_mut_ptr(), v);
        let g32 = api::png_get_uint_32(lib, b32.as_ptr());
        let gi32 = api::png_get_int_32(lib, b32.as_ptr());
        let g16 = api::png_get_uint_16(lib, b32.as_ptr());

        let mut bi = [0xAAu8; 4];
        api::png_save_int_32(lib, bi.as_mut_ptr(), v as i32);
        let gi32b = api::png_get_int_32(lib, bi.as_ptr());

        let mut b16 = [0xAAu8; 4];
        api::png_save_uint_16(lib, b16.as_mut_ptr(), v as c_uint);
        let g16b = api::png_get_uint_16(lib, b16.as_ptr());

        (b32, g32, gi32, g16, bi, gi32b, b16, g16b)
    });
}

// ============================================================ row 3
// png_get_uint_31 (needs a png_structp)

#[test]
fn r03_get_uint_31() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 3);
    // valid path: value <= 0x7fffffff
    let mut vals: Vec<u32> = vec![0, 1, 0x7fff_fffe, 0x7fff_ffff];
    for _ in 0..2048 {
        vals.push(rng.u32() & 0x7fff_ffff);
    }
    // plus the error path (png_error "PNG unsigned integer out of range")
    vals.extend_from_slice(&[0x8000_0000, 0xffff_ffff, 0x8000_0001]);

    diff("row3", &vals, |lib, &v| unsafe {
        let pp = api::new_reader(lib);
        let b = v.to_be_bytes();
        let r = api::png_get_uint_31(lib, pp, b.as_ptr());
        destroy_reader(lib, pp);
        r
    });
}

// ============================================================ row 4
// png_muldiv

#[test]
fn r04_muldiv() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 4);
    let edge: [i32; 9] = [
        0,
        1,
        -1,
        i32::MIN,
        i32::MAX,
        100000,
        -100000,
        65535,
        0x4000_0000,
    ];
    let mut cases: Vec<(i32, i32, i32)> = Vec::new();
    for &a in &edge {
        for &t in &edge {
            for &d in &edge {
                cases.push((a, t, d));
            }
        }
    }
    for _ in 0..4096 {
        cases.push((rng.u32() as i32, rng.u32() as i32, rng.u32() as i32));
    }
    // smaller, "realistic" fixed point magnitudes too
    for _ in 0..2048 {
        cases.push((
            rng.range_i32(-200000, 200000),
            rng.range_i32(-200000, 200000),
            rng.range_i32(-200000, 200000),
        ));
    }

    diff("row4", &cases, |lib, &(a, t, d)| unsafe {
        let mut res: png_fixed_point = 0x5A5A_5A5A;
        let ret = api::png_muldiv(lib, &mut res, a, t, d);
        (ret, res)
    });
}

// ============================================================ row 5
// png_reciprocal / png_reciprocal2

#[test]
fn r05_reciprocal() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 5);
    let edge: [i32; 8] = [0, 1, -1, i32::MIN, i32::MAX, 100000, -100000, 45455];
    let mut cases: Vec<(i32, i32)> = Vec::new();
    for &a in &edge {
        for &b in &edge {
            cases.push((a, b));
        }
    }
    for _ in 0..4096 {
        cases.push((rng.u32() as i32, rng.u32() as i32));
    }
    for _ in 0..2048 {
        cases.push((rng.range_i32(-300000, 300000), rng.range_i32(-300000, 300000)));
    }

    diff("row5", &cases, |lib, &(a, b)| unsafe {
        (
            api::png_reciprocal(lib, a),
            api::png_reciprocal(lib, b),
            api::png_reciprocal2(lib, a, b),
            api::png_reciprocal2(lib, b, a),
        )
    });
}

// ============================================================ row 6
// png_gamma_significant / png_gamma_8bit_correct / png_gamma_16bit_correct /
// png_gamma_correct
//
// NOTE: png_gamma_correct dispatches on png_struct::bit_depth, which is 0 for a
// freshly created reader, so it takes the 16-bit path.  Gamma values are kept
// >= 0 because a negative gamma makes the C compute a double outside the range
// of the return type, which is undefined behaviour in C.

#[test]
fn r06_gamma_correct() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 6);
    let mut gammas: Vec<i32> = vec![100000, 45455, 220000, 1, 1000000, 0, 95000, 105000, 104999];
    for _ in 0..64 {
        gammas.push(rng.range_i32(1, 1000000));
    }

    let mut cases: Vec<(u32, u32, i32)> = Vec::new();
    for &g in &gammas {
        for &v8 in &[0u32, 1, 127, 128, 254, 255] {
            cases.push((v8, v8, g));
        }
        for &v16 in &[0u32, 1, 255, 256, 32767, 32768, 65534, 65535] {
            cases.push((v16 & 0xff, v16, g));
        }
        for _ in 0..8 {
            cases.push((rng.below(256), rng.below(65536), g));
        }
    }

    diff("row6", &cases, |lib, &(v8, v16, g)| unsafe {
        let pp = api::new_reader(lib);
        let out = (
            api::png_gamma_significant(lib, g),
            api::png_gamma_8bit_correct(lib, v8 as c_uint, g),
            api::png_gamma_16bit_correct(lib, v16 as c_uint, g),
            api::png_gamma_correct(lib, pp, v16 as c_uint, g),
            api::png_gamma_correct(lib, pp, v8 as c_uint, g),
        );
        destroy_reader(lib, pp);
        out
    });
}

// ============================================================ row 7
// png_build_gamma_table / png_destroy_gamma_table
//
// There is no exported getter for the generated tables, so what is verified
// here is the *observable* behaviour of the pair:
//   * the gamma values are installed through the public API
//     (png_set_gamma_fixed / png_set_alpha_mode_fixed), whose warnings and
//     app-errors ("invalid file gamma in png_set_gamma", "gamma out of
//     supported range", ...) must match;
//   * building twice without destroying must warn "gamma table being rebuilt"
//     in both libraries (this proves the table pointers were actually set),
//     and building again *after* png_destroy_gamma_table must NOT warn (this
//     proves the destroy actually cleared them);
//   * neither library may crash or error for the same inputs, for bit_depth 8
//     and 16.

#[test]
fn r07_build_and_destroy_gamma_table() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 7);
    let mut cases: Vec<(i32, i32, c_int, c_int, i32)> = Vec::new();
    // (screen, file, bit_depth, alpha_mode, alpha_gamma); alpha_mode < 0 == skip
    for &(screen, file) in &[
        (100000i32, 100000i32),
        (220000, 45455),
        (45455, 220000),
        (100000, 45455),
        (1000, 10000000),
        (10000000, 1000),
        (999, 100000),   /* out of supported range -> app warning */
        (100000, 999),   /* out of supported range -> app warning */
        (0, 100000),     /* invalid -> app error */
        (100000, 0),     /* invalid -> app error */
        (-1, -1),        /* PNG_DEFAULT_sRGB flags */
        (-2, -2),        /* PNG_GAMMA_MAC_18 flags */
        (-5, 100000),    /* negative -> app error */
    ] {
        for &bd in &[8, 16] {
            cases.push((screen, file, bd, -1, 0));
            cases.push((screen, file, bd, PNG_ALPHA_PNG, 100000));
            cases.push((screen, file, bd, PNG_ALPHA_STANDARD, 45455));
            cases.push((screen, file, bd, PNG_ALPHA_OPTIMIZED, 220000));
        }
    }
    for _ in 0..40 {
        let screen = rng.range_i32(1, 1000000);
        let file = rng.range_i32(1, 1000000);
        let bd = if rng.u32() % 2 == 0 { 8 } else { 16 };
        cases.push((screen, file, bd, -1, 0));
    }

    diff(
        "row7",
        &cases,
        |lib, &(screen, file, bd, amode, agamma)| unsafe {
            let pp = api::new_reader(lib);
            if amode >= 0 {
                api::png_set_alpha_mode_fixed(lib, pp, amode, agamma);
            }
            api::png_set_gamma_fixed(lib, pp, screen, file);
            api::png_build_gamma_table(lib, pp, bd);
            // second build without destroy: must warn identically
            api::png_build_gamma_table(lib, pp, bd);
            api::png_destroy_gamma_table(lib, pp);
            // after the destroy there must be no "rebuilt" warning
            api::png_build_gamma_table(lib, pp, bd);
            api::png_destroy_gamma_table(lib, pp);
            api::png_destroy_gamma_table(lib, pp); // idempotent
            destroy_reader(lib, pp);
            0u8
        },
    );
}

// ============================================================ row 8
// exported data arrays

#[test]
fn r08_sRGB_tables() {
    ensure_libm();
    let l = libs();
    unsafe {
        let c_tab = std::slice::from_raw_parts(data_sym(&l.c, b"png_sRGB_table\0") as *const u16, 256);
        let r_tab = std::slice::from_raw_parts(data_sym(&l.rs, b"png_sRGB_table\0") as *const u16, 256);
        for i in 0..256 {
            assert!(
                c_tab[i] == r_tab[i],
                "png_sRGB_table[{i}]: C={} Rust={}",
                c_tab[i],
                r_tab[i]
            );
        }
        // a couple of absolute values from the libpng source
        assert_eq!(c_tab[0], 0);
        assert_eq!(c_tab[255], 65535);

        let c_base = std::slice::from_raw_parts(data_sym(&l.c, b"png_sRGB_base\0") as *const u16, 512);
        let r_base = std::slice::from_raw_parts(data_sym(&l.rs, b"png_sRGB_base\0") as *const u16, 512);
        for i in 0..512 {
            assert!(
                c_base[i] == r_base[i],
                "png_sRGB_base[{i}]: C={} Rust={}",
                c_base[i],
                r_base[i]
            );
        }

        let c_d = std::slice::from_raw_parts(data_sym(&l.c, b"png_sRGB_delta\0"), 512);
        let r_d = std::slice::from_raw_parts(data_sym(&l.rs, b"png_sRGB_delta\0"), 512);
        for i in 0..512 {
            assert!(
                c_d[i] == r_d[i],
                "png_sRGB_delta[{i}]: C={} Rust={}",
                c_d[i],
                r_d[i]
            );
        }
    }
}

// ============================================================ row 9
// png_XYZ_from_xy / png_xy_from_XYZ
//
// Field order taken from c_src/include/pngstruct.h (NOT alphabetical: png_xy
// starts with redx/redy and ends with whitex/whitey).

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PngXy {
    redx: png_fixed_point,
    redy: png_fixed_point,
    greenx: png_fixed_point,
    greeny: png_fixed_point,
    bluex: png_fixed_point,
    bluey: png_fixed_point,
    whitex: png_fixed_point,
    whitey: png_fixed_point,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PngXYZ {
    red_X: png_fixed_point,
    red_Y: png_fixed_point,
    red_Z: png_fixed_point,
    green_X: png_fixed_point,
    green_Y: png_fixed_point,
    green_Z: png_fixed_point,
    blue_X: png_fixed_point,
    blue_Y: png_fixed_point,
    blue_Z: png_fixed_point,
}

const SENTINEL: png_fixed_point = 0x5A5A_5A5A;

#[test]
fn r09_XYZ_from_xy_and_back() {
    ensure_libm();
    let srgb = PngXy {
        redx: 64000,
        redy: 33000,
        greenx: 30000,
        greeny: 60000,
        bluex: 15000,
        bluey: 6000,
        whitex: 31270,
        whitey: 32900,
    };

    let mut rng = Rng::new(0xC0FFEE + 9);
    let mut xys: Vec<PngXy> = vec![srgb, PngXy::default()];
    // realistic-ish chromaticities (0..110001, i.e. straddling the fpLimit)
    for _ in 0..600 {
        xys.push(PngXy {
            redx: rng.range_i32(0, 110001),
            redy: rng.range_i32(0, 110001),
            greenx: rng.range_i32(0, 110001),
            greeny: rng.range_i32(0, 110001),
            bluex: rng.range_i32(0, 110001),
            bluey: rng.range_i32(0, 110001),
            whitex: rng.range_i32(0, 110001),
            whitey: rng.range_i32(0, 110001),
        });
    }
    // small values, where the fixed point maths is close to overflow
    for _ in 0..400 {
        xys.push(PngXy {
            redx: rng.range_i32(0, 100),
            redy: rng.range_i32(0, 100),
            greenx: rng.range_i32(0, 100),
            greeny: rng.range_i32(0, 100),
            bluex: rng.range_i32(0, 100),
            bluey: rng.range_i32(0, 100),
            whitex: rng.range_i32(0, 100),
            whitey: rng.range_i32(0, 100),
        });
    }
    // full range
    for _ in 0..400 {
        xys.push(PngXy {
            redx: rng.u32() as i32,
            redy: rng.u32() as i32,
            greenx: rng.u32() as i32,
            greeny: rng.u32() as i32,
            bluex: rng.u32() as i32,
            bluey: rng.u32() as i32,
            whitex: rng.u32() as i32,
            whitey: rng.u32() as i32,
        });
    }
    // perturbations of the sRGB primaries
    for _ in 0..600 {
        let mut xy = srgb;
        let f = rng.below(8);
        let d = rng.range_i32(-2000, 2000);
        match f {
            0 => xy.redx += d,
            1 => xy.redy += d,
            2 => xy.greenx += d,
            3 => xy.greeny += d,
            4 => xy.bluex += d,
            5 => xy.bluey += d,
            6 => xy.whitex += d,
            _ => xy.whitey += d,
        }
        xys.push(xy);
    }

    diff("row9.XYZ_from_xy", &xys, |lib, xy| unsafe {
        let mut xyz = PngXYZ {
            red_X: SENTINEL,
            red_Y: SENTINEL,
            red_Z: SENTINEL,
            green_X: SENTINEL,
            green_Y: SENTINEL,
            green_Z: SENTINEL,
            blue_X: SENTINEL,
            blue_Y: SENTINEL,
            blue_Z: SENTINEL,
        };
        let ret = myapi::png_XYZ_from_xy(
            lib,
            &mut xyz as *mut PngXYZ as *mut c_void,
            xy as *const PngXy as *const c_void,
        );
        (ret, xyz)
    });

    // and the inverse direction
    let mut rng = Rng::new(0xC0FFEE + 90);
    let mut xyzs: Vec<PngXYZ> = Vec::new();
    // the XYZ of the sRGB primaries, obtained from the C library
    unsafe {
        let mut xyz = PngXYZ::default();
        let ret = myapi::png_XYZ_from_xy(
            &libs().c,
            &mut xyz as *mut PngXYZ as *mut c_void,
            &srgb as *const PngXy as *const c_void,
        );
        assert_eq!(ret, 0, "sRGB primaries must convert");
        xyzs.push(xyz);
    }
    xyzs.push(PngXYZ::default());
    for _ in 0..600 {
        xyzs.push(PngXYZ {
            red_X: rng.range_i32(-10000, 110000),
            red_Y: rng.range_i32(-10000, 110000),
            red_Z: rng.range_i32(-10000, 110000),
            green_X: rng.range_i32(-10000, 110000),
            green_Y: rng.range_i32(-10000, 110000),
            green_Z: rng.range_i32(-10000, 110000),
            blue_X: rng.range_i32(-10000, 110000),
            blue_Y: rng.range_i32(-10000, 110000),
            blue_Z: rng.range_i32(-10000, 110000),
        });
    }
    for _ in 0..600 {
        xyzs.push(PngXYZ {
            red_X: rng.u32() as i32,
            red_Y: rng.u32() as i32,
            red_Z: rng.u32() as i32,
            green_X: rng.u32() as i32,
            green_Y: rng.u32() as i32,
            green_Z: rng.u32() as i32,
            blue_X: rng.u32() as i32,
            blue_Y: rng.u32() as i32,
            blue_Z: rng.u32() as i32,
        });
    }

    // Guard against the test degenerating into "both libraries reject every
    // input in the range checks": a good number of inputs must really reach
    // (and complete) the fixed point arithmetic.
    let mut ok = 0usize;
    unsafe {
        for xy in &xys {
            let mut xyz = PngXYZ::default();
            if myapi::png_XYZ_from_xy(
                &libs().c,
                &mut xyz as *mut PngXYZ as *mut c_void,
                xy as *const PngXy as *const c_void,
            ) == 0
            {
                ok += 1;
            }
        }
    }
    assert!(
        ok > 300,
        "only {ok}/{} xy inputs converted successfully",
        xys.len()
    );

    diff("row9.xy_from_XYZ", &xyzs, |lib, xyz| unsafe {
        let mut xy = PngXy {
            redx: SENTINEL,
            redy: SENTINEL,
            greenx: SENTINEL,
            greeny: SENTINEL,
            bluex: SENTINEL,
            bluey: SENTINEL,
            whitex: SENTINEL,
            whitey: SENTINEL,
        };
        let ret = myapi::png_xy_from_XYZ(
            lib,
            &mut xy as *mut PngXy as *mut c_void,
            xyz as *const PngXYZ as *const c_void,
        );
        (ret, xy)
    });

    let mut ok = 0usize;
    unsafe {
        for xyz in &xyzs {
            let mut xy = PngXy::default();
            if myapi::png_xy_from_XYZ(
                &libs().c,
                &mut xy as *mut PngXy as *mut c_void,
                xyz as *const PngXYZ as *const c_void,
            ) == 0
            {
                ok += 1;
            }
        }
    }
    assert!(
        ok > 300,
        "only {ok}/{} XYZ inputs converted successfully",
        xyzs.len()
    );
}

// ============================================================ row 10
// png_check_fp_number / png_check_fp_string

#[test]
fn r10_check_fp_number_and_string() {
    ensure_libm();
    let fixed = [
        "1", "-.5e+10", "0x", "1e", "..", "", "+", "1.5E-3", "00", "1.", ".5", "e5", "-", ".",
        "+.", "1e+", "1e-", "1E5", "0.0", "000.000e000", "1.2.3", "--1", "1-1", "1e5e5", ".e5",
        "12345678901234567890", "+0", "-0", " 1", "1 ", "1e999999999999",
    ];
    let alphabet: &[u8] = b"0123456789+-.eE ";
    let mut rng = Rng::new(0xC0FFEE + 10);

    // (string bytes, size to pass)
    let mut cases: Vec<(Vec<u8>, usize)> = Vec::new();
    for s in fixed {
        let b = s.as_bytes().to_vec();
        cases.push((b.clone(), b.len()));
        cases.push((b.clone(), b.len() + 1)); // include the NUL
        if !b.is_empty() {
            cases.push((b.clone(), b.len() - 1));
        }
        cases.push((b, 0));
    }
    for _ in 0..500 {
        let n = 1 + rng.below(12) as usize;
        let b: Vec<u8> = (0..n)
            .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
            .collect();
        cases.push((b.clone(), b.len()));
        cases.push((b.clone(), b.len() + 1));
    }

    diff("row10", &cases, |lib, (b, size)| unsafe {
        // NUL-terminated copy so that reading `size`+1 bytes is always in bounds
        let mut owned = b.clone();
        owned.push(0);
        owned.push(0);
        let p = owned.as_ptr() as png_const_charp;
        let mut state: c_int = 0;
        let mut whereami: usize = 0;
        let ret = api::png_check_fp_number(lib, p, *size, &mut state, &mut whereami);
        let rets = api::png_check_fp_string(lib, p, *size);
        (ret, state, whereami, rets)
    });
}

// ============================================================ row 11
// png_ascii_from_fp

#[test]
fn r11_ascii_from_fp() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 11);
    // (value, precision, buffer size to pass)
    let mut cases: Vec<(f64, c_uint, usize)> = Vec::new();
    let fixed: [f64; 14] = [
        0.0,
        1.0,
        -1.0,
        1e-10,
        1e10,
        f64::MIN_POSITIVE,
        f64::MAX,
        -f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -0.0,
        0.5,
        1.0 / 3.0,
    ];
    for &v in &fixed {
        for p in 0..=16u32 {
            cases.push((v, p as c_uint, PNG_fp_MAX));
        }
    }
    for _ in 0..500 {
        let mag = 10f64.powi(rng.range_i32(-12, 12));
        let mut v = rng.f64_unit() * mag;
        if rng.u32() % 2 == 0 {
            v = -v;
        }
        cases.push((v, 1 + rng.below(15) as c_uint, PNG_fp_MAX));
    }
    // buffer-too-small: must png_error identically
    for &sz in &[0usize, 1, 5, 6, 8, 15, 19, 20, 21] {
        cases.push((1.0 / 7.0, 15, sz));
        cases.push((-1.0 / 7.0, 1, sz));
    }

    diff("row11", &cases, |lib, &(v, prec, size)| unsafe {
        let pp = api::new_reader(lib);
        let mut buf = [FILL; PNG_fp_MAX];
        api::png_ascii_from_fp(lib, pp, buf.as_mut_ptr(), size, v, prec);
        destroy_reader(lib, pp);
        buf_u8(&buf)
    });
}

// ============================================================ row 12
// png_ascii_from_fixed

#[test]
fn r12_ascii_from_fixed() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 12);
    let mut cases: Vec<(i32, usize)> = vec![];
    let fixed: [i32; 14] = [
        0,
        1,
        -1,
        i32::MIN,
        i32::MAX,
        100000,
        -100000,
        99999,
        -99999,
        10,
        -10,
        2147400000,
        -2147400000,
        50000,
    ];
    for &v in &fixed {
        cases.push((v, PNG_fp_MAX));
        cases.push((v, 13));
        cases.push((v, 12)); // too small -> png_error
        cases.push((v, 0));
    }
    for _ in 0..1000 {
        cases.push((rng.u32() as i32, PNG_fp_MAX));
    }
    for _ in 0..500 {
        cases.push((rng.range_i32(-300000, 300000), PNG_fp_MAX));
    }

    diff("row12", &cases, |lib, &(v, size)| unsafe {
        let pp = api::new_reader(lib);
        let mut buf = [FILL; PNG_fp_MAX];
        api::png_ascii_from_fixed(lib, pp, buf.as_mut_ptr(), size, v);
        destroy_reader(lib, pp);
        buf_u8(&buf)
    });
}

// ============================================================ row 13
// png_fixed / png_fixed_ITU
//
// png_fixed accepts |100000*d| <= 2^31-1 (see png.c), png_fixed_ITU accepts
// 0 <= 10000*d <= 2^31-1.  Out-of-range values call png_fixed_error, which is
// exercised too (the message log must match).

#[test]
fn r13_fixed_conversions() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 13);
    let mut cases: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e-5,
        -1e-5,
        21474.0,
        -21474.0,
        21474.83647,
        -21474.83648,
        1.0 / 3.0,
        0.000005,
        -0.000005,
        0.0000049999,
    ];
    for _ in 0..1000 {
        let mut v = rng.f64_unit() * 21473.0;
        if rng.u32() % 2 == 0 {
            v = -v;
        }
        cases.push(v);
    }
    for _ in 0..200 {
        cases.push(rng.f64_unit() * 214748.0); // in range for ITU, out for png_fixed
    }
    // out of range for both
    cases.extend_from_slice(&[1e6, -1e6, 1e30, -1e30, 21475.0, -21475.0, 214749.0]);

    diff("row13.png_fixed", &cases, |lib, &d| unsafe {
        let pp = api::new_reader(lib);
        let txt = cs("x");
        let r = api::png_fixed(lib, pp, d, txt.as_ptr());
        destroy_reader(lib, pp);
        r
    });

    diff("row13.png_fixed_ITU", &cases, |lib, &d| unsafe {
        let pp = api::new_reader(lib);
        let txt = cs("x");
        let r = myapi::png_fixed_ITU(lib, pp, d, txt.as_ptr());
        destroy_reader(lib, pp);
        r
    });
}

// ============================================================ row 14
// png_safecat / png_format_number / png_warning_parameter* /
// png_formatted_warning

#[test]
fn r14a_safecat() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 141);
    // (bufsize, pos, string, null_buf, null_str)
    let mut cases: Vec<(usize, usize, Vec<u8>, bool, bool)> = Vec::new();
    let strs: [&str; 7] = ["", "a", "ab", "abcdefgh", "0123456789ABCDEF0123456789", "x\ty", " "];
    for bufsize in 0..=17usize {
        for pos in 0..=18usize {
            for s in strs {
                cases.push((bufsize, pos, s.as_bytes().to_vec(), false, false));
            }
        }
    }
    for _ in 0..300 {
        let n = rng.below(20) as usize;
        let s: Vec<u8> = (0..n).map(|_| 0x21 + rng.u8() % 0x5d).collect();
        cases.push((
            rng.below(20) as usize,
            rng.below(24) as usize,
            s,
            false,
            false,
        ));
    }
    cases.push((8, 0, b"abc".to_vec(), true, false));
    cases.push((8, 0, b"abc".to_vec(), false, true));
    cases.push((8, 3, b"abc".to_vec(), false, true));
    cases.push((8, 99, b"abc".to_vec(), false, true));

    const N: usize = 24;
    diff(
        "row14a",
        &cases,
        |lib, (bufsize, pos, s, null_buf, null_str)| unsafe {
            let mut buf = [FILL; N];
            let mut owned = s.clone();
            owned.push(0);
            let bp = if *null_buf {
                std::ptr::null_mut()
            } else {
                buf.as_mut_ptr()
            };
            let sp = if *null_str {
                std::ptr::null()
            } else {
                owned.as_ptr() as png_const_charp
            };
            // bufsize is bounded by N so that the callee can never write out of
            // bounds of `buf`.
            let ret = api::png_safecat(lib, bp, (*bufsize).min(N), *pos, sp);
            (ret, buf_u8(&buf))
        },
    );
}

#[test]
fn r14b_format_number() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 142);
    let mut cases: Vec<(c_int, u64)> = Vec::new();
    let nums: [u64; 16] = [
        0,
        1,
        9,
        10,
        99,
        100,
        12345,
        99999,
        100000,
        100001,
        4294967295,
        4294967296,
        u64::MAX,
        u64::MAX / 2,
        0xdeadbeef,
        50000,
    ];
    for fmt in 0..=6i32 {
        for &n in &nums {
            cases.push((fmt, n));
        }
    }
    for _ in 0..600 {
        let fmt = rng.range_i32(1, 5);
        let n = if rng.u32() % 2 == 0 {
            rng.u32() as u64
        } else {
            ((rng.u32() as u64) << 32) | rng.u32() as u64
        };
        cases.push((fmt, n));
    }

    diff("row14b", &cases, |lib, &(fmt, n)| unsafe {
        let mut buf = [FILL; PNG_NUMBER_BUFFER_SIZE];
        let start = buf.as_ptr();
        let end = buf.as_mut_ptr().add(PNG_NUMBER_BUFFER_SIZE);
        let ret = api::png_format_number(lib, start, end, fmt, n as c_ulong);
        // compare the OFFSET, not the absolute pointer
        let off = ret as usize - buf.as_ptr() as usize;
        (off, buf_u8(&buf))
    });
}

#[derive(Clone, Debug)]
enum WP {
    Str(c_int, String),
    Uns(c_int, c_int, u32),
    Sgn(c_int, c_int, i32),
}

#[test]
fn r14c_warning_parameters_and_formatted_warning() {
    ensure_libm();
    let msgs: Vec<String> = [
        "@1",
        "@1@2",
        "x@1y@2z",
        "@0",
        "@9",
        "@",
        "a@",
        "@@1",
        "no params",
        "@1 @1",
        "all: @1 @2 @3 @4 @5 @6 @7 @8 @9 @0",
        "@8@7@6@5@4@3@2@1",
        "@a",
        "@ 1",
        "trailing @1@",
        "",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let long40 = "0123456789012345678901234567890123456789".to_string();
    let mut opsets: Vec<Vec<WP>> = vec![
        vec![],
        vec![WP::Str(1, "one".into())],
        vec![WP::Str(1, "one".into()), WP::Str(2, "two".into())],
        vec![WP::Str(0, "zero".into())],
        vec![WP::Str(9, "nine".into())],
        vec![WP::Str(-1, "neg".into())],
        vec![WP::Str(8, "eight".into())],
        vec![WP::Str(1, long40.clone())],
        vec![WP::Str(1, "abcdefghijklmnopqrstuvwxyz01234".into())], // exactly 31
        vec![WP::Str(1, "abcdefghijklmnopqrstuvwxyz012345".into())], // 32
        vec![WP::Str(1, "".into())],
        vec![
            WP::Str(1, "a".into()),
            WP::Str(2, "b".into()),
            WP::Str(3, "c".into()),
            WP::Str(4, "d".into()),
            WP::Str(5, "e".into()),
            WP::Str(6, "f".into()),
            WP::Str(7, "g".into()),
            WP::Str(8, "h".into()),
        ],
    ];
    for fmt in 0..=5i32 {
        opsets.push(vec![WP::Uns(1, fmt, 0)]);
        opsets.push(vec![WP::Uns(1, fmt, 1)]);
        opsets.push(vec![WP::Uns(1, fmt, 100000)]);
        opsets.push(vec![WP::Uns(1, fmt, u32::MAX)]);
        opsets.push(vec![WP::Sgn(1, fmt, 0)]);
        opsets.push(vec![WP::Sgn(1, fmt, -1)]);
        opsets.push(vec![WP::Sgn(1, fmt, i32::MIN)]);
        opsets.push(vec![WP::Sgn(1, fmt, i32::MAX)]);
        opsets.push(vec![WP::Sgn(1, fmt, -100000)]);
        opsets.push(vec![
            WP::Sgn(1, fmt, -12345),
            WP::Uns(2, fmt, 12345),
            WP::Str(3, "s".into()),
        ]);
    }

    let mut cases: Vec<(Vec<WP>, String, bool)> = Vec::new();
    for ops in &opsets {
        for m in &msgs {
            cases.push((ops.clone(), m.clone(), false));
        }
    }
    // NULL parameter block
    for m in &msgs {
        cases.push((vec![], m.clone(), true));
    }
    // random messages
    let mut rng = Rng::new(0xC0FFEE + 143);
    let alpha: &[u8] = b"@0123456789abZ ";
    for _ in 0..300 {
        let n = 1 + rng.below(20) as usize;
        let m: String = (0..n)
            .map(|_| alpha[rng.below(alpha.len() as u32) as usize] as char)
            .collect();
        let mut ops = Vec::new();
        for _ in 0..rng.below(4) {
            match rng.below(3) {
                0 => ops.push(WP::Str(
                    rng.range_i32(0, 9),
                    format!("p{}", rng.below(1000)),
                )),
                1 => ops.push(WP::Uns(rng.range_i32(0, 9), rng.range_i32(0, 6), rng.u32())),
                _ => ops.push(WP::Sgn(
                    rng.range_i32(0, 9),
                    rng.range_i32(0, 6),
                    rng.u32() as i32,
                )),
            }
        }
        cases.push((ops, m, false));
    }

    const PSIZE: usize = PNG_WARNING_PARAMETER_SIZE * PNG_WARNING_PARAMETER_COUNT;
    diff("row14c", &cases, |lib, (ops, msg, null_p)| unsafe {
        let pp = api::new_reader(lib);
        let mut p = [0 as c_char; PSIZE];
        for op in ops {
            match op {
                WP::Str(n, s) => {
                    let cstr = cs(s);
                    api::png_warning_parameter(lib, p.as_mut_ptr(), *n, cstr.as_ptr());
                }
                WP::Uns(n, fmt, v) => {
                    api::png_warning_parameter_unsigned(lib, p.as_mut_ptr(), *n, *fmt, *v);
                }
                WP::Sgn(n, fmt, v) => {
                    api::png_warning_parameter_signed(lib, p.as_mut_ptr(), *n, *fmt, *v);
                }
            }
        }
        let m = cs(msg);
        let pptr = if *null_p {
            std::ptr::null_mut()
        } else {
            p.as_mut_ptr()
        };
        api::png_formatted_warning(lib, pp, pptr, m.as_ptr());
        destroy_reader(lib, pp);
        buf_u8(&p)
    });
}

// ============================================================ row 15
// png_convert_to_rfc1123_buffer / png_convert_to_rfc1123 /
// png_convert_from_time_t / png_convert_from_struct_tm

#[test]
fn r15a_convert_to_rfc1123() {
    ensure_libm();
    let mut cases: Vec<png_time> = Vec::new();
    for month in 0..=13u8 {
        for &day in &[0u8, 1, 31, 32] {
            for &hour in &[0u8, 23, 24] {
                for &minute in &[0u8, 59, 60] {
                    for &second in &[0u8, 60, 61] {
                        cases.push(png_time {
                            year: 2024,
                            month,
                            day,
                            hour,
                            minute,
                            second,
                        });
                    }
                }
            }
        }
    }
    for &year in &[0u16, 1, 999, 1000, 9999, 10000, 65535] {
        cases.push(png_time {
            year,
            month: 6,
            day: 15,
            hour: 12,
            minute: 30,
            second: 30,
        });
    }
    let mut rng = Rng::new(0xC0FFEE + 151);
    for _ in 0..300 {
        cases.push(png_time {
            year: (rng.u32() % 12000) as u16,
            month: rng.u8() % 15,
            day: rng.u8() % 34,
            hour: rng.u8() % 26,
            minute: rng.u8() % 62,
            second: rng.u8() % 63,
        });
    }
    for _ in 0..100 {
        cases.push(png_time {
            year: rng.u32() as u16,
            month: rng.u8(),
            day: rng.u8(),
            hour: rng.u8(),
            minute: rng.u8(),
            second: rng.u8(),
        });
    }

    diff("row15a", &cases, |lib, t| unsafe {
        let mut out = [FILL; 29];
        let ret = api::png_convert_to_rfc1123_buffer(lib, out.as_mut_ptr(), t);
        // NULL out buffer
        let ret_null = api::png_convert_to_rfc1123_buffer(lib, std::ptr::null_mut(), t);
        let pp = api::new_reader(lib);
        let s = api::png_convert_to_rfc1123(lib, pp, t);
        let text = if s.is_null() {
            None
        } else {
            Some(cstr_vec(s))
        };
        // NULL png_ptr must return NULL without touching anything
        let s2 = api::png_convert_to_rfc1123(lib, std::ptr::null_mut(), t);
        destroy_reader(lib, pp);
        (ret, ret_null, buf_u8(&out), text, s2.is_null())
    });
}

#[test]
fn r15b_convert_from_time_t() {
    ensure_libm();
    let mut cases: Vec<i64> = vec![
        0,
        1,
        -1,
        1000000000,
        2147483647,
        2147483648,
        -2147483648,
        253402300799, // 9999-12-31T23:59:59Z
        253402300800,
        67768036191676799,
        67768036191676800,
        i64::MAX,
        i64::MIN,
    ];
    let mut rng = Rng::new(0xC0FFEE + 152);
    for _ in 0..400 {
        let v = (rng.u32() as i64) - 0x8000_0000;
        cases.push(v);
    }
    for _ in 0..100 {
        cases.push(rng.next_u64() as i64);
    }

    diff("row15b", &cases, |lib, &secs| unsafe {
        let mut t = png_time {
            year: 0xAAAA,
            month: 0xAA,
            day: 0xAA,
            hour: 0xAA,
            minute: 0xAA,
            second: 0xAA,
        };
        api::png_convert_from_time_t(lib, &mut t, secs as c_long);
        t
    });
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
fn r15c_convert_from_struct_tm() {
    ensure_libm();
    let mut rng = Rng::new(0xC0FFEE + 153);
    // (sec, min, hour, mday, mon, year)
    let mut cases: Vec<(c_int, c_int, c_int, c_int, c_int, c_int)> = Vec::new();
    for mon in 0..12 {
        for &mday in &[1, 31] {
            for &hour in &[0, 23] {
                cases.push((0, 0, hour, mday, mon, 124));
                cases.push((60, 59, hour, mday, mon, 0));
            }
        }
    }
    for _ in 0..300 {
        cases.push((
            rng.range_i32(-100, 100),
            rng.range_i32(-100, 100),
            rng.range_i32(-100, 100),
            rng.range_i32(-100, 400),
            rng.range_i32(-20, 20),
            rng.range_i32(-2000, 9000),
        ));
    }

    diff(
        "row15c",
        &cases,
        |lib, &(sec, min, hour, mday, mon, year)| unsafe {
            let tm = Tm {
                tm_sec: sec,
                tm_min: min,
                tm_hour: hour,
                tm_mday: mday,
                tm_mon: mon,
                tm_year: year,
                tm_wday: 0,
                tm_yday: 0,
                tm_isdst: 0,
                tm_gmtoff: 0,
                tm_zone: std::ptr::null(),
            };
            let mut t = png_time {
                year: 0xAAAA,
                month: 0xAA,
                day: 0xAA,
                hour: 0xAA,
                minute: 0xAA,
                second: 0xAA,
            };
            api::png_convert_from_struct_tm(lib, &mut t, &tm as *const Tm as *const c_void);
            t
        },
    );
}

// ============================================================ row 16
// png_sig_cmp

#[test]
fn r16_sig_cmp() {
    ensure_libm();
    const SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    let mut cases: Vec<([u8; 8], usize, usize)> = Vec::new();
    for start in 0..=9usize {
        for num in 0..=9usize {
            cases.push((SIG, start, num));
            // every single byte corruption
            for i in 0..8 {
                for &d in &[0u8, 0xff] {
                    let mut s = SIG;
                    s[i] = d;
                    cases.push((s, start, num));
                }
                let mut s = SIG;
                s[i] = SIG[i].wrapping_add(1);
                cases.push((s, start, num));
                let mut s = SIG;
                s[i] = SIG[i].wrapping_sub(1);
                cases.push((s, start, num));
            }
        }
    }
    let mut rng = Rng::new(0xC0FFEE + 16);
    for _ in 0..500 {
        let mut s = [0u8; 8];
        for b in s.iter_mut() {
            *b = rng.u8();
        }
        cases.push((s, rng.below(10) as usize, rng.below(10) as usize));
    }

    diff("row16", &cases, |lib, &(sig, start, num)| unsafe {
        api::png_sig_cmp(lib, sig.as_ptr(), start, num)
    });
}

// ============================================================ row 17
// png_check_IHDR (calls png_error on any problem)

#[test]
fn r17_check_IHDR() {
    ensure_libm();
    let ws: [u32; 7] = [0, 1, 7, 8, 1000000, 1000001, 0x8000_0000];
    let mut cases: Vec<(u32, u32, c_int, c_int, c_int, c_int, c_int)> = Vec::new();
    // exhaustive over bd x ct x il x cm x ft for a couple of (w,h) pairs
    for &(w, h) in &[(1u32, 1u32), (8, 8)] {
        for bd in 0..=17i32 {
            for ct in 0..=7i32 {
                for il in 0..=2i32 {
                    for cm in 0..=1i32 {
                        for ft in 0..=1i32 {
                            cases.push((w, h, bd, ct, il, cm, ft));
                        }
                    }
                }
            }
        }
    }
    // all (w,h) pairs with a valid rest
    for &w in &ws {
        for &h in &ws {
            cases.push((w, h, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0));
            cases.push((w, h, 16, PNG_COLOR_TYPE_GRAY, 1, 0, 0));
        }
    }
    // random sample of the full cross product
    let mut rng = Rng::new(0xC0FFEE + 17);
    for _ in 0..3000 {
        cases.push((
            ws[rng.below(7) as usize],
            ws[rng.below(7) as usize],
            rng.range_i32(0, 17),
            rng.range_i32(0, 7),
            rng.range_i32(0, 2),
            rng.range_i32(0, 1),
            rng.range_i32(0, 1),
        ));
    }
    // negative / large color types and the MNG filter value 64
    for &ct in &[-1i32, 8, 100] {
        cases.push((8, 8, 8, ct, 0, 0, 0));
    }
    for &ft in &[64i32, 65, -1] {
        cases.push((8, 8, 8, PNG_COLOR_TYPE_RGB, 0, 0, ft));
        cases.push((8, 8, 8, PNG_COLOR_TYPE_GRAY, 0, 0, ft));
    }

    diff(
        "row17",
        &cases,
        |lib, &(w, h, bd, ct, il, cm, ft)| unsafe {
            let pp = api::new_reader(lib);
            api::png_check_IHDR(lib, pp, w, h, bd, ct, il, cm, ft);
            destroy_reader(lib, pp);
            0u8
        },
    );
}

// ============================================================ row 18
// png_build_grayscale_palette

#[test]
fn r18_build_grayscale_palette() {
    ensure_libm();
    let mut cases: Vec<c_int> = vec![0, 1, 2, 3, 4, 8, 16];
    cases.extend_from_slice(&[5, 6, 7, 9, 15, 17, 32, -1, -8, i32::MIN, i32::MAX]);

    diff("row18", &cases, |lib, &bd| unsafe {
        let mut pal = [png_color {
            red: 1,
            green: 2,
            blue: 3,
        }; 256];
        api::png_build_grayscale_palette(lib, bd, pal.as_mut_ptr());
        pal.to_vec()
    });

    // NULL palette must be a no-op in both
    diff("row18.null", &[0i32, 1, 8, 16], |lib, &bd| unsafe {
        api::png_build_grayscale_palette(lib, bd, std::ptr::null_mut());
        0u8
    });
}
