//! Low-level entry point differential tests (CONFIGS.md rows L1..L18).
//!
//! Every function is resolved from both `.so`s with `dlsym` and driven with
//! identical, deterministic inputs; the two traces must match byte for byte.
mod support;

use std::ffi::{c_char, c_int, c_long, c_uint, c_void};
use support::core::*;
use support::*;

// ---------------------------------------------------------------------------
// Local #[repr(C)] types taken verbatim from the C headers
// ---------------------------------------------------------------------------

/// `png_xy` (pngstruct.h): 8 `png_fixed_point` fields.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct PngXy {
    redx: i32,
    redy: i32,
    greenx: i32,
    greeny: i32,
    bluex: i32,
    bluey: i32,
    whitex: i32,
    whitey: i32,
}

/// `png_XYZ` (pngstruct.h): 9 `png_fixed_point` fields.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct PngXYZ {
    red_x: i32,
    red_y: i32,
    red_z: i32,
    green_x: i32,
    green_y: i32,
    green_z: i32,
    blue_x: i32,
    blue_y: i32,
    blue_z: i32,
}

/// glibc `struct tm`.
#[repr(C)]
#[derive(Clone, Copy)]
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

impl Default for Tm {
    fn default() -> Tm {
        Tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 1,
            tm_mon: 0,
            tm_year: 70,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
            tm_gmtoff: 0,
            tm_zone: std::ptr::null(),
        }
    }
}

// pngpriv.h constants
const PNG_WARNING_PARAMETER_SIZE: usize = 32;
const PNG_WARNING_PARAMETER_COUNT: usize = 8;
const PNG_NUMBER_BUFFER_SIZE: usize = 24;

/// `png_warning_parameters` = `char [8][32]`.
#[repr(C)]
#[derive(Clone, Copy)]
struct WarnParams([[u8; PNG_WARNING_PARAMETER_SIZE]; PNG_WARNING_PARAMETER_COUNT]);

impl WarnParams {
    fn zeroed() -> WarnParams {
        WarnParams([[0u8; PNG_WARNING_PARAMETER_SIZE]; PNG_WARNING_PARAMETER_COUNT])
    }
    fn as_ptr(&mut self) -> *mut c_char {
        self.0.as_mut_ptr() as *mut c_char
    }
    fn hex(&self) -> String {
        let mut s = String::new();
        for row in self.0.iter() {
            s.push_str(&hex(row));
            s.push('|');
        }
        s
    }
}

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

/// The reference `libpng.so` in `target/cbuild` is linked without `-lm`, so its
/// undefined `floor`/`pow` references have to be satisfied from the global
/// symbol scope.  Loading `libm` with `RTLD_GLOBAL` does that for both
/// libraries identically; it does not change any libpng behaviour.
fn ensure_libm() {
    use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_NOW};
    use std::sync::OnceLock;
    static LIBM: OnceLock<UnixLib> = OnceLock::new();
    LIBM.get_or_init(|| unsafe {
        UnixLib::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL).expect("dlopen libm.so.6")
    });
}

/// `support::diff` with the libm preload in place.
fn diff2(label: &str, run: impl FnMut(&Lib) -> Trace) {
    ensure_libm();
    support::diff(label, run);
}

/// Run a driver body with a longjmp landing pad and harvest the trace.
/// The session must already have been reset.
fn trace_of(f: impl FnMut()) -> Trace {
    let rc = protected(f);
    Trace {
        lines: take_log(),
        out: take_out(),
        rc,
    }
}

/// Escape a byte slice so it can be logged as a readable literal.
fn esc(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        if c >= 0x20 && c < 0x7f && c != b'\\' {
            s.push(c as char);
        } else {
            s.push_str(&format!("\\x{c:02x}"));
        }
    }
    s
}

/// NUL-terminated copy of `b`.
fn cz(b: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// L1 — png_get_uint_32 / png_get_uint_16 / png_get_int_32
// ---------------------------------------------------------------------------

#[test]
fn l1_get_uint_and_int() {
    let mut bufs: Vec<[u8; 4]> = vec![
        [0x00, 0x00, 0x00, 0x00],
        [0x7f, 0xff, 0xff, 0xff],
        [0x80, 0x00, 0x00, 0x00],
        [0xff, 0xff, 0xff, 0xff],
        [0x00, 0x00, 0x00, 0x01],
        [0x80, 0x00, 0x00, 0x01],
        [0xff, 0xff, 0xff, 0xfe],
        [0x00, 0x00, 0x80, 0x00],
        [0x00, 0x80, 0x00, 0x00],
        [0x01, 0x02, 0x03, 0x04],
    ];
    let mut rng = Rng::new(0x0000_0000_0001_0001);
    for _ in 0..512 {
        let b = rng.bytes(4);
        bufs.push([b[0], b[1], b[2], b[3]]);
    }

    diff2("L1", |lib| {
        session_reset(Vec::new());
        let g32: unsafe extern "C" fn(*const u8) -> u32 = lib.f("png_get_uint_32");
        let g16: unsafe extern "C" fn(*const u8) -> u16 = lib.f("png_get_uint_16");
        let gi32: unsafe extern "C" fn(*const u8) -> i32 = lib.f("png_get_int_32");
        trace_of(|| {
            for (i, b) in bufs.iter().enumerate() {
                unsafe {
                    log(format!(
                        "L1[{i}] buf={} u32={} u16={} i32={}",
                        hex(b),
                        g32(b.as_ptr()),
                        g16(b.as_ptr()),
                        gi32(b.as_ptr())
                    ));
                }
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L2 — png_get_uint_31 (valid values only)
// ---------------------------------------------------------------------------

#[test]
fn l2_get_uint_31_valid() {
    let mut bufs: Vec<[u8; 4]> = vec![
        [0x00, 0x00, 0x00, 0x00],
        [0x00, 0x00, 0x00, 0x01],
        [0x7f, 0xff, 0xff, 0xff],
        [0x7f, 0xff, 0xff, 0xfe],
        [0x40, 0x00, 0x00, 0x00],
        [0x00, 0x00, 0x80, 0x00],
    ];
    let mut rng = Rng::new(0x0000_0000_0002_0002);
    for _ in 0..256 {
        let v = rng.next_u32() & 0x7fff_ffff;
        bufs.push(v.to_be_bytes());
    }

    diff2("L2", |lib| {
        let g31: unsafe extern "C" fn(Png, *const u8) -> u32 = lib.f("png_get_uint_31");
        with_read(lib, &[], &mut |_c, png, _info| unsafe {
            for (i, b) in bufs.iter().enumerate() {
                log(format!(
                    "L2[{i}] buf={} u31={}",
                    hex(b),
                    g31(png, b.as_ptr())
                ));
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L3 — png_save_uint_32 / png_save_int_32 / png_save_uint_16
// ---------------------------------------------------------------------------

#[test]
fn l3_save_uint_and_int() {
    let mut vals: Vec<u32> = vec![
        0,
        1,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        0x0000_0001,
        0x0000_ffff,
        0x0001_0000,
        0xfffe_ffff,
        0x1234_5678,
    ];
    let mut rng = Rng::new(0x0000_0000_0003_0003);
    for _ in 0..512 {
        vals.push(rng.next_u32());
    }

    diff2("L3", |lib| {
        session_reset(Vec::new());
        let s32: unsafe extern "C" fn(*mut u8, u32) = lib.f("png_save_uint_32");
        let si32: unsafe extern "C" fn(*mut u8, i32) = lib.f("png_save_int_32");
        let s16: unsafe extern "C" fn(*mut u8, c_uint) = lib.f("png_save_uint_16");
        let mut buf = [0u8; 8];
        trace_of(|| unsafe {
            for (i, &v) in vals.iter().enumerate() {
                buf = [0xAA; 8];
                s32(buf.as_mut_ptr(), v);
                log(format!("L3[{i}] save_uint_32({v})={}", hex(&buf)));

                buf = [0xAA; 8];
                si32(buf.as_mut_ptr(), v as i32);
                log(format!("L3[{i}] save_int_32({})={}", v as i32, hex(&buf)));

                buf = [0xAA; 8];
                s16(buf.as_mut_ptr(), v as c_uint);
                log(format!("L3[{i}] save_uint_16({v})={}", hex(&buf)));

                buf = [0xAA; 8];
                s16(buf.as_mut_ptr(), (v & 0xffff) as c_uint);
                log(format!(
                    "L3[{i}] save_uint_16({})={}",
                    v & 0xffff,
                    hex(&buf)
                ));
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L4 — png_muldiv
// ---------------------------------------------------------------------------

#[test]
fn l4_muldiv() {
    let mut triples: Vec<(i32, i32, i32)> = vec![
        // divisor 0
        (1, 1, 0),
        (0, 0, 0),
        (-1, 5, 0),
        (i32::MIN, i32::MIN, 0),
        // exact divisions
        (100000, 100000, 100000),
        (10, 10, 5),
        (-10, 10, 5),
        (10, -10, 5),
        (10, 10, -5),
        (-10, -10, -5),
        // rounding half cases
        (1, 1, 2),
        (3, 1, 2),
        (-1, 1, 2),
        (-3, 1, 2),
        (1, 3, 2),
        (5, 1, 10),
        (5, 1, -10),
        (-5, 1, 10),
        (7, 1, 2),
        (7, 1, -2),
        // overflow of 32 bits in the product
        (65536, 65536, 1),
        (65536, 65536, 2),
        (i32::MAX, 2, 1),
        (i32::MAX, i32::MAX, 1),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, 1, 1),
        (i32::MIN, -1, 1),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX, 1),
        (i32::MIN, 2, 2),
        (i32::MIN, 2, -2),
        // small / identity
        (0, 5, 7),
        (5, 0, 7),
        (1, 1, 1),
        (-1, -1, -1),
        (100000, 1, 100000),
        (45455, 220000, 100000),
    ];
    let mut rng = Rng::new(0x0000_0000_0004_0004);
    for _ in 0..512 {
        triples.push((
            rng.next_u32() as i32,
            rng.next_u32() as i32,
            rng.next_u32() as i32,
        ));
    }
    // A batch of small random values so that the non-overflow path dominates.
    for _ in 0..128 {
        triples.push((
            (rng.below(200_001) as i32) - 100_000,
            (rng.below(2001) as i32) - 1000,
            (rng.below(2001) as i32) - 1000,
        ));
    }

    diff2("L4", |lib| {
        session_reset(Vec::new());
        let muldiv: unsafe extern "C" fn(*mut i32, i32, i32, i32) -> c_int = lib.f("png_muldiv");
        let mut res: i32 = 0;
        trace_of(|| unsafe {
            for (i, &(a, t, d)) in triples.iter().enumerate() {
                res = -12345;
                let rc = muldiv(&mut res, a, t, d);
                log(format!("L4[{i}] muldiv({a},{t},{d}) rc={rc} res={res}"));
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L5 — png_reciprocal / png_reciprocal2
// ---------------------------------------------------------------------------

#[test]
fn l5_reciprocal() {
    let mut vals: Vec<i32> = vec![
        0,
        1,
        -1,
        i32::MIN,
        i32::MAX,
        100000,
        -100000,
        45455,
        220000,
        2,
        -2,
        99999,
        100001,
        50000,
        10,
        1000000,
    ];
    let mut rng = Rng::new(0x0000_0000_0005_0005);
    for _ in 0..256 {
        vals.push(rng.next_u32() as i32);
    }
    for _ in 0..64 {
        vals.push((rng.below(400_001) as i32) - 200_000);
    }

    diff2("L5", |lib| {
        session_reset(Vec::new());
        let recip: unsafe extern "C" fn(i32) -> i32 = lib.f("png_reciprocal");
        let recip2: unsafe extern "C" fn(i32, i32) -> i32 = lib.f("png_reciprocal2");
        trace_of(|| unsafe {
            for (i, &a) in vals.iter().enumerate() {
                log(format!("L5[{i}] reciprocal({a})={}", recip(a)));
            }
            // reciprocal2 over a deterministic cross product.
            for i in 0..vals.len() {
                let a = vals[i];
                let b = vals[(i * 7 + 3) % vals.len()];
                log(format!("L5r2[{i}] reciprocal2({a},{b})={}", recip2(a, b)));
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L6 — gamma helpers
// ---------------------------------------------------------------------------

#[test]
fn l6_gamma_helpers() {
    // Gamma values: boundary values around the significance threshold (5000)
    // plus randoms.
    let mut gammas: Vec<i32> = vec![
        0,
        1,
        -1,
        100000,
        95000,
        94999,
        105000,
        105001,
        45455,
        220000,
        i32::MIN,
        i32::MAX,
        50000,
        200000,
        33333,
        333333,
    ];
    let mut rng = Rng::new(0x0000_0000_0006_0006);
    for _ in 0..16 {
        gammas.push(rng.below(500_001) as i32);
    }

    let mut samples8: Vec<c_uint> = vec![0, 1, 2, 127, 128, 254, 255];
    let mut samples16: Vec<c_uint> = vec![0, 1, 2, 32767, 32768, 65534, 65535];
    for _ in 0..12 {
        samples8.push(rng.below(256));
        samples16.push(rng.below(65536));
    }

    diff2("L6 pure", |lib| {
        session_reset(Vec::new());
        let signif: unsafe extern "C" fn(i32) -> c_int = lib.f("png_gamma_significant");
        let c8: unsafe extern "C" fn(c_uint, i32) -> u8 = lib.f("png_gamma_8bit_correct");
        let c16: unsafe extern "C" fn(c_uint, i32) -> u16 = lib.f("png_gamma_16bit_correct");
        trace_of(|| unsafe {
            for (gi, &g) in gammas.iter().enumerate() {
                log(format!("L6 significant[{gi}]({g})={}", signif(g)));
            }
            for (gi, &g) in gammas.iter().enumerate() {
                for &v in samples8.iter() {
                    log(format!("L6 g8[{gi}] v={v} g={g} -> {}", c8(v, g)));
                }
                for &v in samples16.iter() {
                    log(format!("L6 g16[{gi}] v={v} g={g} -> {}", c16(v, g)));
                }
            }
        })
    });

    // png_gamma_correct dispatches on png_ptr->bit_depth, so drive it with a
    // fresh read struct (bit_depth 0) and with structs whose IHDR has been
    // read (bit_depth 8 and 16).
    use support::pngbuild::Builder;
    let png8 = Builder::new(4, 3, 8, 2).build_valid(0x606);
    let png16 = Builder::new(4, 3, 16, 2).build_valid(0x607);
    for (tag, stream, read_hdr) in [
        ("fresh", Vec::new(), false),
        ("depth8", png8, true),
        ("depth16", png16, true),
    ] {
        diff2(&format!("L6 correct {tag}"), |lib| {
            let correct: unsafe extern "C" fn(Png, c_uint, i32) -> u16 =
                lib.f("png_gamma_correct");
            with_read(lib, &stream, &mut |c, png, info| unsafe {
                if read_hdr {
                    (c.read_info)(png, info);
                    log(format!("bit_depth={}", (c.get_bit_depth)(png, info)));
                }
                for (gi, &g) in gammas.iter().enumerate() {
                    for &v in samples8.iter() {
                        log(format!("L6 gc[{gi}] v={v} g={g} -> {}", correct(png, v, g)));
                    }
                }
            })
        });
    }
}

// ---------------------------------------------------------------------------
// L7 — png_XYZ_from_xy / png_xy_from_XYZ
// ---------------------------------------------------------------------------

fn log_xy(tag: &str, rc: c_int, xy: &PngXy) {
    log(format!(
        "{tag} rc={rc} rx={} ry={} gx={} gy={} bx={} by={} wx={} wy={}",
        xy.redx, xy.redy, xy.greenx, xy.greeny, xy.bluex, xy.bluey, xy.whitex, xy.whitey
    ));
}

fn log_xyz(tag: &str, rc: c_int, v: &PngXYZ) {
    log(format!(
        "{tag} rc={rc} rX={} rY={} rZ={} gX={} gY={} gZ={} bX={} bY={} bZ={}",
        v.red_x, v.red_y, v.red_z, v.green_x, v.green_y, v.green_z, v.blue_x, v.blue_y, v.blue_z
    ));
}

#[test]
fn l7_xyz_xy_conversions() {
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
    let mut xys: Vec<PngXy> = vec![
        srgb,
        PngXy::default(), // all zero — degenerate
        PngXy {
            whitey: 0,
            ..srgb
        },
        PngXy { redy: 0, ..srgb },
        PngXy {
            greeny: 0,
            bluey: 0,
            ..srgb
        },
        PngXy {
            redx: 100000,
            redy: 0,
            greenx: 0,
            greeny: 100000,
            bluex: 0,
            bluey: 0,
            whitex: 33333,
            whitey: 33333,
        },
        PngXy {
            redx: -64000,
            ..srgb
        },
        PngXy {
            redx: i32::MAX,
            redy: i32::MIN,
            greenx: i32::MAX,
            greeny: i32::MIN,
            bluex: 1,
            bluey: -1,
            whitex: 0,
            whitey: 1,
        },
    ];
    let mut rng = Rng::new(0x0000_0000_0007_0007);
    // Plausible chromaticities.
    for _ in 0..32 {
        xys.push(PngXy {
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
    // Fully random fixed point values.
    for _ in 0..32 {
        xys.push(PngXy {
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

    let mut xyzs: Vec<PngXYZ> = vec![
        PngXYZ::default(),
        PngXYZ {
            red_x: 41239,
            red_y: 21264,
            red_z: 1933,
            green_x: 35758,
            green_y: 71517,
            green_z: 11919,
            blue_x: 18048,
            blue_y: 7219,
            blue_z: 95053,
        },
        PngXYZ {
            red_x: 100000,
            ..PngXYZ::default()
        },
        PngXYZ {
            red_x: i32::MAX,
            red_y: i32::MIN,
            red_z: 1,
            green_x: -1,
            green_y: 0,
            green_z: i32::MAX,
            blue_x: i32::MIN,
            blue_y: 2,
            blue_z: -2,
        },
    ];
    for _ in 0..32 {
        xyzs.push(PngXYZ {
            red_x: rng.below(120_001) as i32,
            red_y: rng.below(120_001) as i32,
            red_z: rng.below(120_001) as i32,
            green_x: rng.below(120_001) as i32,
            green_y: rng.below(120_001) as i32,
            green_z: rng.below(120_001) as i32,
            blue_x: rng.below(120_001) as i32,
            blue_y: rng.below(120_001) as i32,
            blue_z: rng.below(120_001) as i32,
        });
    }
    for _ in 0..16 {
        xyzs.push(PngXYZ {
            red_x: rng.next_u32() as i32,
            red_y: rng.next_u32() as i32,
            red_z: rng.next_u32() as i32,
            green_x: rng.next_u32() as i32,
            green_y: rng.next_u32() as i32,
            green_z: rng.next_u32() as i32,
            blue_x: rng.next_u32() as i32,
            blue_y: rng.next_u32() as i32,
            blue_z: rng.next_u32() as i32,
        });
    }

    diff2("L7", |lib| {
        session_reset(Vec::new());
        let xyz_from_xy: unsafe extern "C" fn(*mut PngXYZ, *const PngXy) -> c_int =
            lib.f("png_XYZ_from_xy");
        let xy_from_xyz: unsafe extern "C" fn(*mut PngXy, *const PngXYZ) -> c_int =
            lib.f("png_xy_from_XYZ");
        let mut out_xyz = PngXYZ::default();
        let mut out_xy = PngXy::default();
        trace_of(|| unsafe {
            for (i, xy) in xys.iter().enumerate() {
                out_xyz = PngXYZ {
                    red_x: -1,
                    red_y: -1,
                    red_z: -1,
                    green_x: -1,
                    green_y: -1,
                    green_z: -1,
                    blue_x: -1,
                    blue_y: -1,
                    blue_z: -1,
                };
                let rc = xyz_from_xy(&mut out_xyz, xy);
                log_xy(&format!("L7 in_xy[{i}]"), 0, xy);
                log_xyz(&format!("L7 XYZ_from_xy[{i}]"), rc, &out_xyz);
                // round trip
                out_xy = PngXy {
                    redx: -1,
                    redy: -1,
                    greenx: -1,
                    greeny: -1,
                    bluex: -1,
                    bluey: -1,
                    whitex: -1,
                    whitey: -1,
                };
                let rc2 = xy_from_xyz(&mut out_xy, &out_xyz);
                log_xy(&format!("L7 round[{i}]"), rc2, &out_xy);
            }
            for (i, xyz) in xyzs.iter().enumerate() {
                out_xy = PngXy {
                    redx: -1,
                    redy: -1,
                    greenx: -1,
                    greeny: -1,
                    bluex: -1,
                    bluey: -1,
                    whitex: -1,
                    whitey: -1,
                };
                let rc = xy_from_xyz(&mut out_xy, xyz);
                log_xyz(&format!("L7 in_XYZ[{i}]"), 0, xyz);
                log_xy(&format!("L7 xy_from_XYZ[{i}]"), rc, &out_xy);
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L8 — png_check_fp_number / png_check_fp_string
// ---------------------------------------------------------------------------

fn l8_inputs() -> Vec<Vec<u8>> {
    let literal: &[&[u8]] = &[
        b"0",
        b"1",
        b"-1",
        b"+1",
        b".5",
        b"+.5e-3",
        b"1.",
        b"1.5",
        b"1e10",
        b"1E-10",
        b"1e+10",
        b"",
        b".",
        b"-",
        b"+",
        b"e",
        b"E",
        b"e1",
        b".e1",
        b"1.2.3",
        b"0.0",
        b"-0",
        b"00",
        b"000.000",
        b" 1",
        b"1 ",
        b"1x",
        b"x1",
        b"1e",
        b"1e+",
        b"1e-",
        b"--1",
        b"++1",
        b"1-2",
        b"1.5e2.5",
        b"12345678901234567890",
        b"0.00000000001",
        b"1e999",
        b"9999999999e-99",
        b"1e1e1",
        b"-.5",
        b"-.5e+3",
        b".0e0",
        b"+0.0e-0",
        b"1.0E5",
        b"12345.6789",
        b".00001",
        b"0e0",
        b"junk",
        b"3.14159265358979",
    ];
    let mut v: Vec<Vec<u8>> = literal.iter().map(|s| s.to_vec()).collect();
    // embedded NULs
    v.push(b"1\x002".to_vec());
    v.push(b"\x00".to_vec());
    v.push(b"1.5\x00abc".to_vec());
    v.push(b"\x001.5".to_vec());
    // random ASCII soup, seeded
    const ALPHA: &[u8] = b"0123456789.eE+- aXz\x00\x7f";
    let mut rng = Rng::new(0x0000_0000_0008_0008);
    for _ in 0..96 {
        let n = rng.below(11) as usize;
        let s: Vec<u8> = (0..n).map(|_| ALPHA[rng.below(ALPHA.len() as u32) as usize]).collect();
        v.push(s);
    }
    for _ in 0..64 {
        let n = rng.below(9) as usize;
        let s: Vec<u8> = (0..n).map(|_| 0x20 + rng.byte() % 0x5f).collect();
        v.push(s);
    }
    v
}

#[test]
fn l8_check_fp_number_and_string() {
    let inputs = l8_inputs();
    let terminated: Vec<Vec<u8>> = inputs.iter().map(|s| cz(s)).collect();

    diff2("L8", |lib| {
        session_reset(Vec::new());
        let num: unsafe extern "C" fn(*const c_char, usize, *mut c_int, *mut usize) -> c_int =
            lib.f("png_check_fp_number");
        let strf: unsafe extern "C" fn(*const c_char, usize) -> c_int =
            lib.f("png_check_fp_string");
        let mut state: c_int = 0;
        let mut where_: usize = 0;
        trace_of(|| unsafe {
            for (i, raw) in inputs.iter().enumerate() {
                let s = &terminated[i];
                let p = s.as_ptr() as *const c_char;
                // (a) size = number of bytes, terminator not visible
                for &size in [raw.len(), raw.len() + 1].iter() {
                    state = 0;
                    where_ = 0;
                    let mut round = 0;
                    loop {
                        let prev = where_;
                        let rc = num(p, size, &mut state, &mut where_);
                        log(format!(
                            "L8 num[{i}] \"{}\" size={size} round={round} rc={rc} state={state} where={where_}",
                            esc(raw)
                        ));
                        round += 1;
                        if where_ == prev || where_ >= size || round >= 4 {
                            break;
                        }
                    }
                    log(format!(
                        "L8 str[{i}] \"{}\" size={size} rc={}",
                        esc(raw),
                        strf(p, size)
                    ));
                }
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L9 — png_ascii_from_fp / png_ascii_from_fixed
// ---------------------------------------------------------------------------

#[test]
fn l9_ascii_from_fp_and_fixed() {
    let mut fps: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        10.0,
        100000.0,
        1.0 / 3.0,
        2.2,
        0.45455,
        1e-300,
        -1e-300,
        1e300,
        -1e300,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        123456789.0,
        3.14159265358979,
        1e-15,
        9.999999999999999e15,
    ];
    let mut rng = Rng::new(0x0000_0000_0009_0009);
    for _ in 0..24 {
        // Spread over many magnitudes.
        let m = rng.f64();
        let e = (rng.below(41) as i32) - 20;
        fps.push(m * 10f64.powi(e) * if rng.below(2) == 0 { 1.0 } else { -1.0 });
    }

    let mut fixed: Vec<i32> = vec![
        0,
        1,
        -1,
        100000,
        -100000,
        i32::MAX,
        i32::MIN,
        45455,
        99999,
        100001,
        12345,
        -12345,
        2147400000,
        50000,
        99990,
    ];
    for _ in 0..24 {
        fixed.push(rng.next_u32() as i32);
    }

    let precisions: Vec<c_uint> = (0..=15).collect();
    let sizes: Vec<usize> = vec![24, 40, 64];
    let fixed_sizes: Vec<usize> = vec![13, 16, 32, 64];

    diff2("L9", |lib| {
        let from_fp: unsafe extern "C" fn(Png, *mut c_char, usize, f64, c_uint) =
            lib.f("png_ascii_from_fp");
        let from_fixed: unsafe extern "C" fn(Png, *mut c_char, usize, i32) =
            lib.f("png_ascii_from_fixed");
        let mut buf = [0u8; 64];
        with_write(lib, &mut |_c, png, _info| unsafe {
            for (i, &fp) in fps.iter().enumerate() {
                for &prec in precisions.iter() {
                    for &size in sizes.iter() {
                        buf = [0xAA; 64];
                        from_fp(png, buf.as_mut_ptr() as *mut c_char, size, fp, prec);
                        log(format!(
                            "L9 fp[{i}] bits={:016x} prec={prec} size={size} s=\"{}\" buf={}",
                            fp.to_bits(),
                            cstr(buf.as_ptr() as *const c_char),
                            hex(&buf)
                        ));
                    }
                }
            }
            for (i, &fx) in fixed.iter().enumerate() {
                for &size in fixed_sizes.iter() {
                    buf = [0xAA; 64];
                    from_fixed(png, buf.as_mut_ptr() as *mut c_char, size, fx);
                    log(format!(
                        "L9 fixed[{i}] v={fx} size={size} s=\"{}\" buf={}",
                        cstr(buf.as_ptr() as *const c_char),
                        hex(&buf)
                    ));
                }
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L10 — png_fixed / png_fixed_ITU (in-range values only)
// ---------------------------------------------------------------------------

#[test]
fn l10_png_fixed() {
    // png_fixed errors when floor(100000*fp+.5) leaves [-2^31, 2^31-1], i.e.
    // outside roughly +/-21474.836; stay well inside.
    let mut fps: Vec<f64> = vec![
        0.0, -0.0, 1.0, -1.0, 0.5, -0.5, 0.000005, -0.000005, 0.000004, -0.000004, 21474.0,
        -21474.0, 2.2, 0.45455, 1.0 / 3.0, 100.12345, -100.12345, 1e-10, -1e-10, 12345.6789,
    ];
    // png_fixed_ITU: floor(10000*fp+.5) must be in [0, 2^31-1] → [0, 214748.3]
    let mut itu: Vec<f64> = vec![
        0.0, 0.00005, 0.00004, 1.0, 0.5, 1000.0, 214748.0, 2.2, 1.0 / 3.0, 12345.6789, 1e-10,
        99999.99999,
    ];
    let mut rng = Rng::new(0x0000_0000_000a_000a);
    for _ in 0..64 {
        let s = if rng.below(2) == 0 { 1.0 } else { -1.0 };
        fps.push(s * rng.f64() * 21474.0);
    }
    for _ in 0..64 {
        itu.push(rng.f64() * 214748.0);
    }

    diff2("L10", |lib| {
        let fixed: unsafe extern "C" fn(Png, f64, *const c_char) -> i32 = lib.f("png_fixed");
        let fixed_itu: unsafe extern "C" fn(Png, f64, *const c_char) -> u32 =
            lib.f("png_fixed_ITU");
        let text = b"L10\0";
        with_read(lib, &[], &mut |_c, png, _info| unsafe {
            let t = text.as_ptr() as *const c_char;
            for (i, &fp) in fps.iter().enumerate() {
                log(format!(
                    "L10 fixed[{i}] bits={:016x} -> {}",
                    fp.to_bits(),
                    fixed(png, fp, t)
                ));
            }
            for (i, &fp) in itu.iter().enumerate() {
                log(format!(
                    "L10 ITU[{i}] bits={:016x} -> {}",
                    fp.to_bits(),
                    fixed_itu(png, fp, t)
                ));
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L11 — png_check_keyword
// ---------------------------------------------------------------------------

fn l11_keywords() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    let lit: &[&[u8]] = &[
        b"Title",
        b"",
        b" ",
        b"    ",
        b" Title",
        b"Title ",
        b"  Title  ",
        b"Two  Spaces",
        b"Many     Spaces",
        b"a b c d",
        b"Tab\there",
        b"New\nline",
        b"Bell\x07x",
        b"Del\x7fx",
        b"High\x80x",
        b"High\x9fx",
        b"High\xa0x",
        b"High\xa1x",
        b"High\xffx",
        b"\x01",
        b"\x1f",
        b"a\x01b",
        b"a\x01\x02b",
        b"\x01\x02\x03",
        b"~",
        b"!",
        b"\"quoted\"",
        b"end\x00cut",
    ];
    for s in lit {
        v.push(s.to_vec());
    }
    // control characters, one at a time
    for c in 1u8..0x20 {
        v.push(vec![b'k', c, b'k']);
    }
    for c in [0x7fu8, 0x80, 0x81, 0x9e, 0x9f, 0xa0, 0xa1, 0xfe, 0xff] {
        v.push(vec![b'k', c, b'k']);
    }
    // length boundaries
    for n in [1usize, 78, 79, 80, 100, 200] {
        v.push(vec![b'K'; n]);
    }
    v.push({
        let mut s = vec![b'A'; 79];
        s.push(b' ');
        s.extend_from_slice(b"tail");
        s
    });
    // random keywords
    let mut rng = Rng::new(0x0000_0000_000b_000b);
    for _ in 0..48 {
        let n = rng.below(90) as usize;
        v.push((0..n).map(|_| rng.byte()).filter(|b| *b != 0).collect());
    }
    v
}

#[test]
fn l11_check_keyword() {
    let keys = l11_keywords();
    let terminated: Vec<Vec<u8>> = keys.iter().map(|k| cz(k)).collect();

    diff2("L11", |lib| {
        let check: unsafe extern "C" fn(Png, *const c_char, *mut u8) -> u32 =
            lib.f("png_check_keyword");
        let mut new_key = [0u8; 80];
        with_write(lib, &mut |_c, png, _info| unsafe {
            // NULL key
            new_key = [0xAA; 80];
            let n = check(png, std::ptr::null(), new_key.as_mut_ptr());
            log(format!("L11 nullkey len={n} buf={}", hex(&new_key)));
            for (i, k) in terminated.iter().enumerate() {
                new_key = [0xAA; 80];
                let n = check(png, k.as_ptr() as *const c_char, new_key.as_mut_ptr());
                log(format!(
                    "L11[{i}] key=\"{}\" len={n} buf={}",
                    esc(&keys[i]),
                    hex(&new_key)
                ));
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L12 — png_safecat / png_format_number / png_warning_parameter* /
//       png_formatted_warning
// ---------------------------------------------------------------------------

#[test]
fn l12_string_and_warning_helpers() {
    // ---- png_safecat -----------------------------------------------------
    let cat_strings: Vec<Vec<u8>> = vec![
        cz(b""),
        cz(b"a"),
        cz(b"abc"),
        cz(b"0123456789"),
        cz(b"0123456789012345678901234567890123456789"),
        cz(b"tail"),
    ];
    // ---- png_format_number ----------------------------------------------
    let numbers: Vec<u64> = vec![
        0,
        1,
        9,
        10,
        15,
        16,
        99,
        100,
        255,
        256,
        12345,
        99999,
        100000,
        100001,
        123456789,
        4294967295,
        4294967296,
        u64::MAX,
        1000000,
        50000,
    ];
    let formats: Vec<c_int> = vec![0, 1, 2, 3, 4, 5, 6, -1];
    let signed_values: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        -9,
        100000,
        -100000,
        i32::MAX,
        i32::MIN,
        12345,
        -12345,
        -99999,
    ];
    let messages: Vec<Vec<u8>> = vec![
        cz(b"plain message"),
        cz(b"one @1 two @2 three @3"),
        cz(b"@1@2@3@4@5@6@7@8@9"),
        cz(b"trailing @"),
        cz(b"@0 is not a parameter"),
        cz(b"@9 is out of range"),
        cz(b"@@1"),
        cz(b"@a"),
        cz(b""),
        cz(b"keyword \"@1\": bad character '0x@2'"),
    ];

    diff2("L12", |lib| {
        let safecat: unsafe extern "C" fn(*mut c_char, usize, usize, *const c_char) -> usize =
            lib.f("png_safecat");
        let fmtnum: unsafe extern "C" fn(*const c_char, *mut c_char, c_int, u64) -> *mut c_char =
            lib.f("png_format_number");
        let wparam: unsafe extern "C" fn(*mut c_char, c_int, *const c_char) =
            lib.f("png_warning_parameter");
        let wpu: unsafe extern "C" fn(*mut c_char, c_int, c_int, u64) =
            lib.f("png_warning_parameter_unsigned");
        let wps: unsafe extern "C" fn(*mut c_char, c_int, c_int, i32) =
            lib.f("png_warning_parameter_signed");
        let fwarn: unsafe extern "C" fn(Png, *mut c_char, *const c_char) =
            lib.f("png_formatted_warning");

        let mut cbuf = [0u8; 32];
        let mut nbuf = [0u8; PNG_NUMBER_BUFFER_SIZE];
        let mut wp = WarnParams::zeroed();

        with_write(lib, &mut |_c, png, _info| unsafe {
            // png_safecat
            for (si, s) in cat_strings.iter().enumerate() {
                for &bufsize in [0usize, 1, 2, 8, 16, 32].iter() {
                    for &pos in [0usize, 1, 7, 15, 31, 32].iter() {
                        cbuf = [0xAA; 32];
                        let r = safecat(
                            cbuf.as_mut_ptr() as *mut c_char,
                            bufsize,
                            pos,
                            s.as_ptr() as *const c_char,
                        );
                        log(format!(
                            "L12 safecat[{si}] bufsize={bufsize} pos={pos} ret={r} buf={}",
                            hex(&cbuf)
                        ));
                    }
                }
            }
            // NULL buffer / NULL string
            log(format!(
                "L12 safecat nullbuf ret={}",
                safecat(std::ptr::null_mut(), 16, 0, cat_strings[2].as_ptr() as *const c_char)
            ));
            cbuf = [0xAA; 32];
            let r = safecat(cbuf.as_mut_ptr() as *mut c_char, 16, 3, std::ptr::null());
            log(format!("L12 safecat nullstr ret={r} buf={}", hex(&cbuf)));

            // png_format_number
            for &fmt in formats.iter() {
                for (ni, &n) in numbers.iter().enumerate() {
                    nbuf = [0xAA; PNG_NUMBER_BUFFER_SIZE];
                    let start = nbuf.as_ptr() as *const c_char;
                    let end = nbuf.as_mut_ptr().add(PNG_NUMBER_BUFFER_SIZE) as *mut c_char;
                    let ret = fmtnum(start, end, fmt, n);
                    let off = (ret as usize) - (nbuf.as_ptr() as usize);
                    log(format!(
                        "L12 fmtnum fmt={fmt} n[{ni}]={n} off={off} s=\"{}\" buf={}",
                        cstr(ret as *const c_char),
                        hex(&nbuf)
                    ));
                }
            }

            // png_warning_parameter (incl. out-of-range numbers and truncation)
            for (si, s) in cat_strings.iter().enumerate() {
                for num in 0..=9 {
                    wp = WarnParams::zeroed();
                    wparam(wp.as_ptr(), num, s.as_ptr() as *const c_char);
                    log(format!("L12 wparam[{si}] n={num} p={}", wp.hex()));
                }
            }
            wp = WarnParams::zeroed();
            wparam(wp.as_ptr(), 1, std::ptr::null());
            log(format!("L12 wparam null p={}", wp.hex()));

            // unsigned / signed parameter formatting
            for &fmt in formats.iter() {
                wp = WarnParams::zeroed();
                for (ni, &n) in numbers.iter().enumerate() {
                    let slot = (ni % PNG_WARNING_PARAMETER_COUNT) as c_int + 1;
                    wpu(wp.as_ptr(), slot, fmt, n);
                }
                log(format!("L12 wpu fmt={fmt} p={}", wp.hex()));
                wp = WarnParams::zeroed();
                for (ni, &v) in signed_values.iter().enumerate() {
                    let slot = (ni % PNG_WARNING_PARAMETER_COUNT) as c_int + 1;
                    wps(wp.as_ptr(), slot, fmt, v);
                }
                log(format!("L12 wps fmt={fmt} p={}", wp.hex()));
            }

            // png_formatted_warning: the message arrives through cb_warning.
            for (mi, m) in messages.iter().enumerate() {
                wp = WarnParams::zeroed();
                for slot in 1..=PNG_WARNING_PARAMETER_COUNT {
                    let s = format!("p{slot}\0");
                    wparam(wp.as_ptr(), slot as c_int, s.as_ptr() as *const c_char);
                }
                log(format!("L12 fwarn[{mi}] msg=\"{}\"", esc(m)));
                fwarn(png, wp.as_ptr(), m.as_ptr() as *const c_char);
                // NULL parameter block
                fwarn(png, std::ptr::null_mut(), m.as_ptr() as *const c_char);
            }
            // A parameter slot that is not NUL terminated: exercises the
            // `parm < pend` bound in png_formatted_warning.
            wp = WarnParams::zeroed();
            wp.0[0] = [b'X'; PNG_WARNING_PARAMETER_SIZE];
            log("L12 fwarn unterminated".to_string());
            fwarn(png, wp.as_ptr(), messages[1].as_ptr() as *const c_char);
            // Long substitution that overflows the internal 192 byte buffer.
            wp = WarnParams::zeroed();
            for slot in 1..=PNG_WARNING_PARAMETER_COUNT {
                wparam(
                    wp.as_ptr(),
                    slot as c_int,
                    cat_strings[4].as_ptr() as *const c_char,
                );
            }
            let long_msg = cz(b"@1/@2/@3/@4/@5/@6/@7/@8 and some trailing text to overflow the buffer @1 @2 @3 @4 @5 @6 @7 @8");
            log("L12 fwarn overflow".to_string());
            fwarn(png, wp.as_ptr(), long_msg.as_ptr() as *const c_char);
        })
    });
}

// ---------------------------------------------------------------------------
// L13 — png_build_grayscale_palette
// ---------------------------------------------------------------------------

#[test]
fn l13_build_grayscale_palette() {
    let depths: Vec<c_int> = vec![-1, 0, 1, 2, 3, 4, 5, 7, 8, 9, 16, 32];

    diff2("L13", |lib| {
        session_reset(Vec::new());
        let build: unsafe extern "C" fn(c_int, *mut u8) = lib.f("png_build_grayscale_palette");
        let mut pal = [0u8; 256 * 3];
        trace_of(|| unsafe {
            for &d in depths.iter() {
                pal = [0xAA; 256 * 3];
                build(d, pal.as_mut_ptr());
                log(format!("L13 depth={d} pal={}", hex(&pal)));
            }
            // NULL palette must be a no-op.
            build(8, std::ptr::null_mut());
            log("L13 nullpal ok".to_string());
        })
    });
}

// ---------------------------------------------------------------------------
// L14 — time conversions
// ---------------------------------------------------------------------------

#[test]
fn l14_time_conversions() {
    let mut times: Vec<PngTime> = vec![
        PngTime { year: 1970, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2024, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
        PngTime { year: 9999, month: 12, day: 31, hour: 23, minute: 59, second: 59 },
        // out of range fields
        PngTime { year: 10000, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2000, month: 0, day: 1, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2000, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2000, month: 1, day: 0, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2000, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2000, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
        PngTime { year: 2000, month: 1, day: 1, hour: 0, minute: 60, second: 0 },
        PngTime { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
        PngTime { year: 0, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        PngTime { year: 1, month: 1, day: 1, hour: 1, minute: 1, second: 1 },
        PngTime { year: 65535, month: 255, day: 255, hour: 255, minute: 255, second: 255 },
    ];
    let mut rng = Rng::new(0x0000_0000_000e_000e);
    for _ in 0..48 {
        times.push(PngTime {
            year: rng.below(11000) as u16,
            month: rng.below(14) as u8,
            day: rng.below(34) as u8,
            hour: rng.below(26) as u8,
            minute: rng.below(62) as u8,
            second: rng.below(63) as u8,
        });
    }

    let time_ts: Vec<i64> = vec![
        0,
        1,
        -1,
        1000000000,
        2147483647,
        2147483648,
        -2208988800,
        253402300799,
        i64::MAX,
        i64::MIN,
        951782400,
        1234567890,
    ];

    let mut tms: Vec<Tm> = vec![
        Tm::default(),
        Tm { tm_year: 124, tm_mon: 11, tm_mday: 31, tm_hour: 23, tm_min: 59, tm_sec: 60, ..Tm::default() },
        Tm { tm_year: -1900, tm_mon: 0, tm_mday: 1, ..Tm::default() },
        Tm { tm_year: 8100, tm_mon: 11, tm_mday: 31, ..Tm::default() },
        Tm { tm_year: 300, tm_mon: 300, tm_mday: 300, tm_hour: 300, tm_min: 300, tm_sec: 300, ..Tm::default() },
        Tm { tm_year: -1, tm_mon: -1, tm_mday: -1, tm_hour: -1, tm_min: -1, tm_sec: -1, ..Tm::default() },
    ];
    for _ in 0..24 {
        tms.push(Tm {
            tm_sec: rng.below(80) as c_int,
            tm_min: rng.below(80) as c_int,
            tm_hour: rng.below(40) as c_int,
            tm_mday: rng.below(40) as c_int,
            tm_mon: rng.below(20) as c_int,
            tm_year: rng.below(300) as c_int,
            ..Tm::default()
        });
    }

    diff2("L14", |lib| {
        session_reset(Vec::new());
        let rfc: unsafe extern "C" fn(*mut c_char, *const u8) -> c_int =
            lib.f("png_convert_to_rfc1123_buffer");
        let from_t: unsafe extern "C" fn(*mut u8, i64) = lib.f("png_convert_from_time_t");
        let from_tm: unsafe extern "C" fn(*mut u8, *const Tm) =
            lib.f("png_convert_from_struct_tm");
        let mut out = [0u8; 40];
        let mut pt = PngTime::default();
        trace_of(|| unsafe {
            for (i, t) in times.iter().enumerate() {
                out = [0xAA; 40];
                let rc = rfc(
                    out.as_mut_ptr() as *mut c_char,
                    t as *const PngTime as *const u8,
                );
                log(format!(
                    "L14 rfc[{i}] {t:?} rc={rc} s=\"{}\" buf={}",
                    cstr(out.as_ptr() as *const c_char),
                    hex(&out)
                ));
            }
            // NULL out buffer
            log(format!(
                "L14 rfc nullout rc={}",
                rfc(
                    std::ptr::null_mut(),
                    &times[0] as *const PngTime as *const u8
                )
            ));

            for (i, &tt) in time_ts.iter().enumerate() {
                pt = PngTime {
                    year: 0xFFFF,
                    month: 0xFF,
                    day: 0xFF,
                    hour: 0xFF,
                    minute: 0xFF,
                    second: 0xFF,
                };
                from_t(&mut pt as *mut PngTime as *mut u8, tt);
                log(format!("L14 from_time_t[{i}] t={tt} -> {pt:?}"));
            }

            for (i, tm) in tms.iter().enumerate() {
                pt = PngTime {
                    year: 0xFFFF,
                    month: 0xFF,
                    day: 0xFF,
                    hour: 0xFF,
                    minute: 0xFF,
                    second: 0xFF,
                };
                from_tm(&mut pt as *mut PngTime as *mut u8, tm);
                log(format!(
                    "L14 from_tm[{i}] y={} mon={} mday={} h={} m={} s={} -> {pt:?}",
                    tm.tm_year, tm.tm_mon, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec
                ));
            }
        })
    });
}

// ---------------------------------------------------------------------------
// L15 — png_sig_cmp
// ---------------------------------------------------------------------------

#[test]
fn l15_sig_cmp() {
    const SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    diff2("L15", |lib| {
        session_reset(Vec::new());
        let cmp: unsafe extern "C" fn(*const u8, usize, usize) -> c_int = lib.f("png_sig_cmp");
        // 24-byte buffer with deterministic filler so any over-read is defined.
        let mut buf = [0x5Au8; 24];
        trace_of(|| unsafe {
            buf = [0x5A; 24];
            buf[..8].copy_from_slice(&SIG);
            for start in 0..10usize {
                for num in 0..10usize {
                    log(format!(
                        "L15 good start={start} num={num} rc={}",
                        cmp(buf.as_ptr(), start, num)
                    ));
                }
            }
            // Every single-byte corruption.
            for pos in 0..8usize {
                for &delta in [0x01u8, 0x80, 0xff].iter() {
                    buf = [0x5A; 24];
                    buf[..8].copy_from_slice(&SIG);
                    buf[pos] ^= delta;
                    log(format!(
                        "L15 corrupt pos={pos} delta={delta:02x} sig={} rc08={} rc0{pos}1={} rc04={}",
                        hex(&buf[..8]),
                        cmp(buf.as_ptr(), 0, 8),
                        cmp(buf.as_ptr(), pos, 1),
                        cmp(buf.as_ptr(), 0, 4)
                    ));
                }
                // zero out the byte too
                buf = [0x5A; 24];
                buf[..8].copy_from_slice(&SIG);
                buf[pos] = 0;
                log(format!(
                    "L15 zero pos={pos} rc08={} rc_start={}",
                    cmp(buf.as_ptr(), 0, 8),
                    cmp(buf.as_ptr(), pos, 8 - pos)
                ));
            }
            // All-zero and all-0xff buffers.
            buf = [0x00; 24];
            log(format!("L15 zeros rc={}", cmp(buf.as_ptr(), 0, 8)));
            buf = [0xff; 24];
            log(format!("L15 ones rc={}", cmp(buf.as_ptr(), 0, 8)));
        })
    });
}

// ---------------------------------------------------------------------------
// L16 — png_reset_crc / png_calculate_crc / png_crc_finish + crc actions
// ---------------------------------------------------------------------------

#[test]
fn l16_crc() {
    // ---- part 1: reset / calculate / finish over a read struct -----------
    let mut rng = Rng::new(0x0000_0000_0010_0010);
    let mut bufs: Vec<Vec<u8>> = Vec::new();
    for i in 0..24usize {
        bufs.push(rng.bytes(1 + (i * 11) % 97));
    }
    // The input stream must serve, per buffer:
    //   4 bytes: the true CRC          (finish(0) -> 0)
    //   4 bytes: a corrupted CRC       (finish(0) -> 1 + warning)
    //   n bytes of data + 4 bytes CRC  (finish(n) -> 0)
    let mut input: Vec<u8> = Vec::new();
    for b in &bufs {
        let good = pngbuild::crc32(b);
        input.extend_from_slice(&good.to_be_bytes());
        input.extend_from_slice(&(good ^ 1).to_be_bytes());
        input.extend_from_slice(b);
        input.extend_from_slice(&good.to_be_bytes());
    }

    diff2("L16 crc-core", |lib| {
        let reset: unsafe extern "C" fn(Png) = lib.f("png_reset_crc");
        let calc: unsafe extern "C" fn(Png, *const u8, usize) = lib.f("png_calculate_crc");
        let finish: unsafe extern "C" fn(Png, u32) -> c_int = lib.f("png_crc_finish");
        with_read(lib, &input, &mut |c, png, _info| unsafe {
            // WARN_USE for critical chunks so that a CRC mismatch warns instead
            // of longjmp'ing out of the loop.
            (c.set_crc_action)(png, PNG_CRC_WARN_USE, PNG_CRC_WARN_USE);
            for (i, b) in bufs.iter().enumerate() {
                let n = b.len();
                // good CRC, fed in two calls
                reset(png);
                let half = n / 2;
                calc(png, b.as_ptr(), half);
                calc(png, b.as_ptr().add(half), n - half);
                log(format!("L16[{i}] n={n} good rc={}", finish(png, 0)));
                // bad CRC
                reset(png);
                calc(png, b.as_ptr(), n);
                log(format!("L16[{i}] n={n} bad rc={}", finish(png, 0)));
                // let crc_finish read the data itself via its skip argument
                reset(png);
                log(format!("L16[{i}] n={n} skip rc={}", finish(png, n as u32)));
            }
        })
    });

    // ---- part 2: every png_set_crc_action pair on real streams ----------
    use support::pngbuild::{join, split, Builder};
    let base = Builder::new(6, 4, 8, 2)
        .add(b"gAMA", 100000u32.to_be_bytes().to_vec())
        .build_valid(0x1601);
    let mut bad_ancillary = split(&base);
    for ch in bad_ancillary.iter_mut() {
        if &ch.name == b"gAMA" {
            *ch = ch.clone().bad_crc();
        }
    }
    let bad_ancillary = join(&bad_ancillary);
    let mut bad_critical = split(&base);
    for ch in bad_critical.iter_mut() {
        if &ch.name == b"IDAT" {
            *ch = ch.clone().bad_crc();
        }
    }
    let bad_critical = join(&bad_critical);

    let actions = [
        PNG_CRC_DEFAULT,
        PNG_CRC_ERROR_QUIT,
        PNG_CRC_WARN_DISCARD,
        PNG_CRC_WARN_USE,
        PNG_CRC_QUIET_USE,
        PNG_CRC_NO_CHANGE,
    ];
    for (sname, stream) in [
        ("valid", &base),
        ("bad-ancillary", &bad_ancillary),
        ("bad-critical", &bad_critical),
    ] {
        for &crit in actions.iter() {
            for &ancil in actions.iter() {
                diff2(&format!("L16 action {sname} {crit}/{ancil}"), |lib| {
                    let mut row = vec![0u8; 64];
                    with_read(lib, stream, &mut |c, png, info| unsafe {
                        (c.set_crc_action)(png, crit, ancil);
                        (c.read_info)(png, info);
                        let mut g: i32 = -1;
                        log(format!(
                            "gAMA rc={} v={g}",
                            (c.get_gAMA_fixed)(png, info, &mut g)
                        ));
                        let rb = (c.get_rowbytes)(png, info);
                        for r in 0..4 {
                            (c.read_row)(png, row.as_mut_ptr(), std::ptr::null_mut());
                            log(format!("row{r}={}", hex(&row[..rb])));
                        }
                        (c.read_end)(png, std::ptr::null_mut());
                        log("end".to_string());
                    })
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// L17 — exported sRGB tables (data symbols)
// ---------------------------------------------------------------------------

#[test]
fn l17_srgb_tables() {
    diff2("L17", |lib| {
        session_reset(Vec::new());
        let table = lib.data("png_sRGB_table") as *const u16;
        let base = lib.data("png_sRGB_base") as *const u16;
        let delta = lib.data("png_sRGB_delta");
        trace_of(|| unsafe {
            let t = std::slice::from_raw_parts(table, 256);
            let b = std::slice::from_raw_parts(base, 512);
            let d = std::slice::from_raw_parts(delta, 512);
            for (i, v) in t.iter().enumerate() {
                log(format!("L17 table[{i}]={v}"));
            }
            for (i, v) in b.iter().enumerate() {
                log(format!("L17 base[{i}]={v}"));
            }
            log(format!("L17 delta={}", hex(d)));
        })
    });
}

// ---------------------------------------------------------------------------
// L18 — memory allocation APIs
// ---------------------------------------------------------------------------

#[test]
fn l18_memory() {
    const HUGE: u64 = 1u64 << 62;

    diff2("L18", |lib| {
        session_reset(Vec::new());
        let create2: unsafe extern "C" fn(
            *const c_char,
            *mut c_void,
            Cb,
            Cb,
            *mut c_void,
            Cb,
            Cb,
        ) -> Png = lib.f("png_create_read_struct_2");
        let set_longjmp: unsafe extern "C" fn(Png, *const c_void, usize) -> *mut c_void =
            lib.f("png_set_longjmp_fn");
        let destroy: unsafe extern "C" fn(*mut Png, *mut Info, *mut Info) =
            lib.f("png_destroy_read_struct");
        let p_malloc: unsafe extern "C" fn(Png, u64) -> *mut c_void = lib.f("png_malloc");
        let p_calloc: unsafe extern "C" fn(Png, u64) -> *mut c_void = lib.f("png_calloc");
        let p_malloc_warn: unsafe extern "C" fn(Png, u64) -> *mut c_void =
            lib.f("png_malloc_warn");
        let p_malloc_base: unsafe extern "C" fn(Png, u64) -> *mut c_void =
            lib.f("png_malloc_base");
        let p_malloc_array: unsafe extern "C" fn(Png, c_int, usize) -> *mut c_void =
            lib.f("png_malloc_array");
        let p_realloc_array: unsafe extern "C" fn(
            Png,
            *const c_void,
            c_int,
            c_int,
            usize,
        ) -> *mut c_void = lib.f("png_realloc_array");
        let p_free: unsafe extern "C" fn(Png, *mut c_void) = lib.f("png_free");
        let p_zalloc: unsafe extern "C" fn(*mut c_void, c_uint, c_uint) -> *mut c_void =
            lib.f("png_zalloc");
        let p_zfree: unsafe extern "C" fn(*mut c_void, *mut c_void) = lib.f("png_zfree");

        let ok_sizes: [u64; 5] = [0, 1, 7, 1024, 65536];
        let fail_sizes: [u64; 3] = [HUGE, u64::MAX, u64::MAX / 2];
        let seed_array = [0xC3u8; 64];

        let rc = protected(|| unsafe {
            let png = create2(
                VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                cb_error as Cb,
                cb_warning as Cb,
                std::ptr::null_mut(),
                cb_malloc as Cb,
                cb_free as Cb,
            );
            log(format!("create2={}", if png.is_null() { 0 } else { 1 }));
            if png.is_null() {
                return;
            }
            set_longjmp(png, shim().longjmp_ptr, shim().jmp_buf_size);
            // Only now start tracing allocations: the sizes of the internal
            // png_struct / jmp_buf are implementation details.
            with_session(|s| {
                s.trace_alloc = true;
                s.malloc_count = 0;
            });

            // png_malloc / png_calloc / png_malloc_warn / png_malloc_base
            for &sz in ok_sizes.iter() {
                let p = p_malloc(png, sz);
                log(format!("L18 malloc({sz})={}", if p.is_null() { 0 } else { 1 }));
                if !p.is_null() && sz > 0 {
                    let s = std::slice::from_raw_parts(p as *const u8, sz.min(16) as usize);
                    log(format!("L18 malloc({sz}) head={}", hex(s)));
                }
                p_free(png, p);

                let p = p_calloc(png, sz);
                log(format!("L18 calloc({sz})={}", if p.is_null() { 0 } else { 1 }));
                if !p.is_null() && sz > 0 {
                    let s = std::slice::from_raw_parts(p as *const u8, sz.min(16) as usize);
                    log(format!("L18 calloc({sz}) head={}", hex(s)));
                }
                p_free(png, p);

                let p = p_malloc_warn(png, sz);
                log(format!(
                    "L18 malloc_warn({sz})={}",
                    if p.is_null() { 0 } else { 1 }
                ));
                p_free(png, p);

                let p = p_malloc_base(png, sz);
                log(format!(
                    "L18 malloc_base({sz})={}",
                    if p.is_null() { 0 } else { 1 }
                ));
                p_free(png, p);
            }

            // Failing sizes: png_malloc would png_error (Phase C), so only the
            // non-erroring entry points are used here.
            for &sz in fail_sizes.iter() {
                let p = p_malloc_warn(png, sz);
                log(format!(
                    "L18 malloc_warn(huge={sz})={}",
                    if p.is_null() { 0 } else { 1 }
                ));
                let p2 = p_malloc_base(png, sz);
                log(format!(
                    "L18 malloc_base(huge={sz})={}",
                    if p2.is_null() { 0 } else { 1 }
                ));
                p_free(png, p);
                p_free(png, p2);
            }

            // png_malloc_array
            for &(n, es) in [
                (1i32, 1usize),
                (4, 8),
                (3, 32),
                (1024, 7),
                (1, 65536),
                (1 << 20, 1usize << 40),
                (1 << 30, 1usize << 40),
            ]
            .iter()
            {
                let p = p_malloc_array(png, n, es);
                log(format!(
                    "L18 malloc_array({n},{es})={}",
                    if p.is_null() { 0 } else { 1 }
                ));
                if !p.is_null() {
                    let want = (n as usize) * es;
                    let s =
                        std::slice::from_raw_parts(p as *const u8, std::cmp::min(want, 32));
                    log(format!("L18 malloc_array({n},{es}) head={}", hex(s)));
                }
                p_free(png, p);
            }

            // png_realloc_array
            for &(old_n, add_n, es) in [
                (0i32, 4i32, 16usize),
                (4, 4, 16),
                (1, 1, 1),
                (2, 3, 8),
                (4, i32::MAX, 16),
                (4, i32::MAX - 3, 16),
                (1, 1, 1usize << 40),
            ]
            .iter()
            {
                let old: *const c_void = if old_n > 0 {
                    seed_array.as_ptr() as *const c_void
                } else {
                    std::ptr::null()
                };
                let p = p_realloc_array(png, old, old_n, add_n, es);
                log(format!(
                    "L18 realloc_array({old_n},{add_n},{es})={}",
                    if p.is_null() { 0 } else { 1 }
                ));
                if !p.is_null() {
                    let want = ((old_n + add_n) as usize) * es;
                    let s =
                        std::slice::from_raw_parts(p as *const u8, std::cmp::min(want, 160));
                    log(format!(
                        "L18 realloc_array({old_n},{add_n},{es}) data={}",
                        hex(s)
                    ));
                }
                p_free(png, p);
            }

            // png_zalloc / png_zfree
            for &(items, size) in [
                (0u32, 0u32),
                (1, 1),
                (16, 64),
                (1, 65536),
                (65535, 8),
                (0, 32),
                (32, 0),
            ]
            .iter()
            {
                let p = p_zalloc(png, items, size);
                log(format!(
                    "L18 zalloc({items},{size})={}",
                    if p.is_null() { 0 } else { 1 }
                ));
                p_zfree(png, p);
            }
            log(format!(
                "L18 zalloc(null)={}",
                if p_zalloc(std::ptr::null_mut(), 4, 4).is_null() {
                    0
                } else {
                    1
                }
            ));
            p_zfree(png, std::ptr::null_mut());
            p_free(png, std::ptr::null_mut());
            log("L18 frees of NULL ok".to_string());

            with_session(|s| s.trace_alloc = false);
            let mut pp = png;
            let mut ip: Info = std::ptr::null_mut();
            destroy(&mut pp, &mut ip, std::ptr::null_mut());
            log("L18 destroyed".to_string());
        });
        with_session(|s| s.trace_alloc = false);
        Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        }
    });
}
