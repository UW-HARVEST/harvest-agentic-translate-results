//! Phase B, group L: the LOWEST-LEVEL exported entry points, driven directly
//! through the `.so` exports of both implementations with randomized inputs
//! (fixed seed).  These are the `pngpriv.h` internals plus the stateless public
//! helpers; bugs here are invisible to the convenience wrappers.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

const SEED: u64 = 0x5eed_1234_abcd_0001;

/// The 15 legal (colour type, bit depth) pairs.
pub const LEGAL: &[(c_int, c_int)] = &[
    (0, 1),
    (0, 2),
    (0, 4),
    (0, 8),
    (0, 16),
    (3, 1),
    (3, 2),
    (3, 4),
    (3, 8),
    (2, 8),
    (2, 16),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

fn row_info(width: u32, bit_depth: c_int, color_type: c_int) -> PngRowInfo {
    let ch = channels(color_type) as u8;
    let pd = ch * bit_depth as u8;
    PngRowInfo {
        width,
        rowbytes: rowbytes(width, bit_depth, color_type),
        color_type: color_type as u8,
        bit_depth: bit_depth as u8,
        channels: ch,
        pixel_depth: pd,
    }
}

// ---------------------------------------------------------------------------
// L2 png_sig_cmp
// ---------------------------------------------------------------------------
#[test]
fn l2_png_sig_cmp() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        let mut rng = Rng::new(SEED);
        let sig_ok: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        for start in 0..10usize {
            for num in 0..10usize {
                unsafe {
                    log(format!(
                        "ok {start} {num} -> {}",
                        (l.api.png_sig_cmp)(sig_ok.as_ptr(), start, num)
                    ));
                }
            }
        }
        for _ in 0..512 {
            let b = rng.bytes(8);
            let start = (rng.below(10)) as usize;
            let num = (rng.below(10)) as usize;
            unsafe {
                log(format!(
                    "rnd {:02x?} {start} {num} -> {}",
                    b,
                    (l.api.png_sig_cmp)(b.as_ptr(), start, num)
                ));
            }
        }
        // near-miss signatures: one byte off in each position
        for i in 0..8 {
            let mut b = sig_ok;
            b[i] ^= 1;
            unsafe {
                log(format!("near{i} -> {}", (l.api.png_sig_cmp)(b.as_ptr(), 0, 8)));
            }
        }
    };
    diff_bare("L2 png_sig_cmp", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// L3..L7 integer load/store helpers
// ---------------------------------------------------------------------------
#[test]
fn l3_l7_int_functions() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        let mut rng = Rng::new(SEED ^ 3);
        for _ in 0..4096 {
            let b = rng.bytes(4);
            unsafe {
                log(format!(
                    "u32={} u16={} i32={}",
                    (l.api.png_get_uint_32)(b.as_ptr()),
                    (l.api.png_get_uint_16)(b.as_ptr()),
                    (l.api.png_get_int_32)(b.as_ptr())
                ));
            }
        }
        // Boundary buffers, including the 0x80000000 sign boundary for int_32.
        for v in [
            0u32, 1, 0x7f, 0x80, 0xff, 0x100, 0x7fff_ffff, 0x8000_0000, 0x8000_0001, 0xffff_fffe,
            0xffff_ffff,
        ] {
            let b = v.to_be_bytes();
            unsafe {
                log(format!(
                    "bnd {v:#x}: u32={} u16={} i32={}",
                    (l.api.png_get_uint_32)(b.as_ptr()),
                    (l.api.png_get_uint_16)(b.as_ptr()),
                    (l.api.png_get_int_32)(b.as_ptr())
                ));
            }
        }
        let mut rng = Rng::new(SEED ^ 7);
        for _ in 0..4096 {
            let v = rng.u32();
            let mut b = [0u8; 4];
            unsafe {
                (l.api.png_save_uint_32)(b.as_mut_ptr(), v);
                log(format!("save32 {v:#x} -> {b:02x?}"));
                (l.api.png_save_int_32)(b.as_mut_ptr(), v as i32);
                log(format!("saveI32 {v:#x} -> {b:02x?}"));
                (l.api.png_save_uint_16)(b.as_mut_ptr(), v as c_uint);
                log(format!("save16 {v:#x} -> {b:02x?}"));
            }
        }
    };
    diff_bare("L3-L7 int helpers", &c, &r, &mut run);
}

// L6 png_get_uint_31 needs a live png_struct (it calls png_error on overflow).
#[test]
fn l6_png_get_uint_31() {
    let (c, r) = libs();
    for (i, v) in [
        0u32,
        1,
        0x7fff_fffe,
        0x7fff_ffff,
        0x8000_0000,
        0x8000_0001,
        0xffff_ffff,
    ]
    .into_iter()
    .enumerate()
    {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                let b = v.to_be_bytes();
                let got = (l.api.png_get_uint_31)(png, b.as_ptr());
                log(format!("uint31({v:#x})={got}"));
            })
        };
        diff(&format!("L6 png_get_uint_31 #{i} {v:#x}"), &c, &r, &mut run);
    }
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            let mut rng = Rng::new(SEED ^ 61);
            for _ in 0..256 {
                let b = rng.bytes(4);
                // Keep the MSB clear so no longjmp escapes the loop.
                let mut b = b;
                b[0] &= 0x7f;
                log(format!(
                    "uint31 {:02x?} = {}",
                    b,
                    (l.api.png_get_uint_31)(png, b.as_ptr())
                ));
            }
        })
    };
    diff("L6 png_get_uint_31 random", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// L8 png_build_grayscale_palette
// ---------------------------------------------------------------------------
#[test]
fn l8_build_grayscale_palette() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        for bd in [-1, 0, 1, 2, 3, 4, 5, 8, 16, 32] {
            let mut pal = [PngColor::default(); 256];
            unsafe {
                (l.api.png_build_grayscale_palette)(bd, pal.as_mut_ptr());
            }
            log(format!("bd={bd} pal={:?}", &pal[..]));
        }
        // NULL palette must be tolerated exactly as C tolerates it
        unsafe {
            (l.api.png_build_grayscale_palette)(8, ptr::null_mut());
        }
        log("null palette ok");
    };
    diff_bare("L8 png_build_grayscale_palette", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// L9..L11 time conversion
// ---------------------------------------------------------------------------
#[test]
fn l9_l11_time_conversion() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        let mut rng = Rng::new(SEED ^ 9);
        for _ in 0..512 {
            let t = (rng.next_u64() % 4_200_000_000) as i64;
            let mut pt = PngTime::default();
            let mut buf = [0i8; 29];
            unsafe {
                (l.api.png_convert_from_time_t)(&mut pt, t);
                let ok = (l.api.png_convert_to_rfc1123_buffer)(buf.as_mut_ptr() as *mut c_char, &pt);
                let s: Vec<u8> = buf.iter().map(|&x| x as u8).collect();
                log(format!("t={t} pt={pt:?} ok={ok} s={:?}", String::from_utf8_lossy(&s)));
            }
        }
        // Direct png_time values, including out-of-range fields
        for pt in [
            PngTime { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0 },
            PngTime { year: 1995, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
            PngTime { year: 2000, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
            PngTime { year: 65535, month: 13, day: 32, hour: 24, minute: 60, second: 61 },
            PngTime { year: 9999, month: 6, day: 15, hour: 12, minute: 30, second: 30 },
        ] {
            let mut buf = [0i8; 29];
            unsafe {
                let ok = (l.api.png_convert_to_rfc1123_buffer)(buf.as_mut_ptr() as *mut c_char, &pt);
                let s: Vec<u8> = buf.iter().map(|&x| x as u8).collect();
                log(format!("pt={pt:?} ok={ok} s={:?}", String::from_utf8_lossy(&s)));
            }
        }
        // NULL `out` is explicitly checked by the C (`if (out == NULL) return 0`).
        // NULL `ptime` is NOT checked by the C — it dereferences unconditionally —
        // so it is undefined behaviour rather than a rejection, and is not a
        // comparable input.
        unsafe {
            let pt = PngTime { year: 1995, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
            log(format!(
                "null out -> {}",
                (l.api.png_convert_to_rfc1123_buffer)(ptr::null_mut(), &pt)
            ));
        }
    };
    diff_bare("L9-L11 time conversion", &c, &r, &mut run);
}

#[test]
fn l11_convert_to_rfc1123_deprecated() {
    let (c, r) = libs();
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            let mut rng = Rng::new(SEED ^ 11);
            for _ in 0..64 {
                let pt = PngTime {
                    year: (rng.u32() % 3000) as u16,
                    month: (rng.u32() % 14) as u8,
                    day: (rng.u32() % 33) as u8,
                    hour: (rng.u32() % 25) as u8,
                    minute: (rng.u32() % 61) as u8,
                    second: (rng.u32() % 62) as u8,
                };
                let p = (l.api.png_convert_to_rfc1123)(png, &pt);
                let s = if p.is_null() {
                    "<null>".to_string()
                } else {
                    std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
                };
                log(format!("{pt:?} -> {s}"));
            }
        })
    };
    diff("L11 png_convert_to_rfc1123", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// L12..L18 fixed-point / gamma arithmetic
// ---------------------------------------------------------------------------
#[test]
fn l12_png_muldiv() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        let interesting: [i32; 13] = [
            0,
            1,
            -1,
            2,
            -2,
            100000,
            -100000,
            65535,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            0x7fff,
            -0x7fff,
        ];
        for &a in &interesting {
            for &m in &interesting {
                for &d in &interesting {
                    let mut res: i32 = 0x5a5a_5a5a;
                    unsafe {
                        let ok = (l.pv.png_muldiv)(&mut res, a, m, d);
                        log(format!("muldiv({a},{m},{d})={ok}/{res}"));
                    }
                }
            }
        }
        let mut rng = Rng::new(SEED ^ 12);
        for _ in 0..4096 {
            let a = rng.u32() as i32;
            let m = rng.u32() as i32;
            let d = rng.u32() as i32;
            let mut res: i32 = 0;
            unsafe {
                let ok = (l.pv.png_muldiv)(&mut res, a, m, d);
                log(format!("rnd muldiv({a},{m},{d})={ok}/{res}"));
            }
        }
    };
    diff_bare("L12 png_muldiv", &c, &r, &mut run);
}

#[test]
fn l13_l17_reciprocal_and_gamma() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        let vals: [i32; 15] = [
            0, 1, -1, 2, 10, 100, 1000, 10000, 45455, 50000, 100000, 220000, i32::MAX, i32::MIN,
            -100000,
        ];
        for &a in &vals {
            unsafe {
                log(format!("recip({a})={}", (l.pv.png_reciprocal)(a)));
                for &b in &vals {
                    log(format!("recip2({a},{b})={}", (l.pv.png_reciprocal2)(a, b)));
                }
                log(format!("sig({a})={}", (l.pv.png_gamma_significant)(a)));
            }
        }
        let mut rng = Rng::new(SEED ^ 13);
        for _ in 0..2048 {
            let a = rng.u32() as i32;
            let b = rng.u32() as i32;
            unsafe {
                log(format!(
                    "rnd recip({a})={} recip2({a},{b})={} sig={}",
                    (l.pv.png_reciprocal)(a),
                    (l.pv.png_reciprocal2)(a, b),
                    (l.pv.png_gamma_significant)(a)
                ));
            }
        }
        // 8- and 16-bit gamma correction over the full value range
        for &g in &[
            10i32, 45455, 50000, 100000, 200000, 220000, 1_000_000, 1, i32::MAX,
        ] {
            for v in 0..256u32 {
                unsafe {
                    log(format!(
                        "g8({v},{g})={}",
                        (l.pv.png_gamma_8bit_correct)(v as c_uint, g)
                    ));
                }
            }
            for v in (0..65536u32).step_by(257) {
                unsafe {
                    log(format!(
                        "g16({v},{g})={}",
                        (l.pv.png_gamma_16bit_correct)(v as c_uint, g)
                    ));
                }
            }
        }
    };
    diff_bare("L13-L17 reciprocal + gamma", &c, &r, &mut run);
}

#[test]
fn l18_png_gamma_correct() {
    let (c, r) = libs();
    for bit_depth in [8i32, 16] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_IHDR)(
                    png,
                    info,
                    1,
                    1,
                    bit_depth,
                    PNG_COLOR_TYPE_GRAY,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                let hi: u32 = if bit_depth == 8 { 256 } else { 65536 };
                let step: u32 = if bit_depth == 8 { 1 } else { 313 };
                for &g in &[45455i32, 100000, 220000, 500000] {
                    let mut v = 0u32;
                    while v < hi {
                        log(format!(
                            "gc({v},{g})={}",
                            (l.pv.png_gamma_correct)(png, v as c_uint, g)
                        ));
                        v += step;
                    }
                }
            })
        };
        diff(&format!("L18 png_gamma_correct bd={bit_depth}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// L19..L21 colour-space conversion
// ---------------------------------------------------------------------------
#[test]
fn l19_l21_xyz_conversion() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        let mut rng = Rng::new(SEED ^ 19);
        // sRGB reference chromaticities plus randomized ones (incl. degenerate).
        let mut cases = vec![PngXy {
            redx: 64000,
            redy: 33000,
            greenx: 30000,
            greeny: 60000,
            bluex: 15000,
            bluey: 6000,
            whitex: 31270,
            whitey: 32900,
        }];
        for _ in 0..1024 {
            cases.push(PngXy {
                redx: (rng.u32() % 200_001) as i32 - 100_000,
                redy: (rng.u32() % 200_001) as i32 - 100_000,
                greenx: (rng.u32() % 200_001) as i32 - 100_000,
                greeny: (rng.u32() % 200_001) as i32 - 100_000,
                bluex: (rng.u32() % 200_001) as i32 - 100_000,
                bluey: (rng.u32() % 200_001) as i32 - 100_000,
                whitex: (rng.u32() % 200_001) as i32 - 100_000,
                whitey: (rng.u32() % 200_001) as i32 - 100_000,
            });
        }
        cases.push(PngXy::default());
        for xy in &cases {
            let mut xyz = PngXYZ::default();
            unsafe {
                let ok = (l.pv.png_XYZ_from_xy)(&mut xyz, xy);
                log(format!("XYZ_from_xy({xy:?})={ok} {xyz:?}"));
                if ok == 0 {
                    let mut back = PngXy::default();
                    let ok2 = (l.pv.png_xy_from_XYZ)(&mut back, &xyz);
                    log(format!("  round trip = {ok2} {back:?}"));
                }
            }
        }
        let mut rng = Rng::new(SEED ^ 20);
        for _ in 0..1024 {
            let xyz = PngXYZ {
                red_X: rng.u32() as i32 / 4,
                red_Y: rng.u32() as i32 / 4,
                red_Z: rng.u32() as i32 / 4,
                green_X: rng.u32() as i32 / 4,
                green_Y: rng.u32() as i32 / 4,
                green_Z: rng.u32() as i32 / 4,
                blue_X: rng.u32() as i32 / 4,
                blue_Y: rng.u32() as i32 / 4,
                blue_Z: rng.u32() as i32 / 4,
            };
            let mut xy = PngXy::default();
            unsafe {
                let ok = (l.pv.png_xy_from_XYZ)(&mut xy, &xyz);
                log(format!("xy_from_XYZ({xyz:?})={ok} {xy:?}"));
            }
        }
    };
    diff_bare("L19-L21 XYZ<->xy", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// L22..L25 ASCII / number helpers
// ---------------------------------------------------------------------------
#[test]
fn l22_l23_fp_number_checks() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        let fixed: &[&str] = &[
            "", "0", "1", "-1", "+1", ".", "0.", ".0", "1e10", "1E-10", "1.5e+3", "--1", "1.2.3",
            "e5", "1e", "1e+", " 1", "1 ", "0x10", "inf", "nan", "00000.00000", "1234567890",
            "-.5e-5", "+.", "1e999999",
        ];
        for s in fixed {
            let bytes = s.as_bytes();
            let mut state: c_int = 0;
            let mut where_: usize = 0;
            unsafe {
                let cs = std::ffi::CString::new(*s).unwrap();
                let n = (l.pv.png_check_fp_number)(
                    cs.as_ptr(),
                    bytes.len(),
                    &mut state,
                    &mut where_,
                );
                log(format!("fp_number({s:?})={n} state={state} where={where_}"));
                log(format!(
                    "fp_string({s:?})={}",
                    (l.pv.png_check_fp_string)(cs.as_ptr(), bytes.len())
                ));
            }
        }
        let mut rng = Rng::new(SEED ^ 22);
        for _ in 0..2048 {
            let n = (rng.below(25)) as usize;
            let alphabet = b"0123456789+-.eE ";
            let s: Vec<u8> = (0..n)
                .map(|_| alphabet[(rng.below(alphabet.len() as u32)) as usize])
                .collect();
            let mut buf = s.clone();
            buf.push(0);
            let mut state: c_int = (rng.u32() % 8) as c_int;
            let mut where_: usize = 0;
            unsafe {
                let res = (l.pv.png_check_fp_number)(
                    buf.as_ptr() as *const c_char,
                    n,
                    &mut state,
                    &mut where_,
                );
                log(format!(
                    "rnd fp_number({:?})={res} state={state} where={where_}",
                    String::from_utf8_lossy(&s)
                ));
                log(format!(
                    "rnd fp_string={}",
                    (l.pv.png_check_fp_string)(buf.as_ptr() as *const c_char, n)
                ));
            }
        }
    };
    diff_bare("L22-L23 fp number/string checks", &c, &r, &mut run);
}

#[test]
fn l24_l25_safecat_and_format_number() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        let mut rng = Rng::new(SEED ^ 24);
        for _ in 0..1024 {
            let bufsize = 1 + (rng.below(40)) as usize;
            let pos = (rng.below(bufsize as u32 + 4)) as usize;
            let slen = (rng.below(20)) as usize;
            let s: Vec<u8> = (0..slen).map(|_| b'a' + (rng.u8() % 26)).collect();
            let mut cs = s.clone();
            cs.push(0);
            let mut buf = vec![0u8; bufsize + 8];
            unsafe {
                let newpos = (l.pv.png_safecat)(
                    buf.as_mut_ptr() as *mut c_char,
                    bufsize,
                    pos,
                    cs.as_ptr() as *const c_char,
                );
                log(format!(
                    "safecat(bufsize={bufsize},pos={pos},s={:?})={newpos} buf={:02x?}",
                    String::from_utf8_lossy(&s),
                    buf
                ));
            }
        }
        // png_format_number writes backwards into buffer[start..end]
        let mut rng = Rng::new(SEED ^ 25);
        for fmt in [
            PNG_NUMBER_FORMAT_u,
            PNG_NUMBER_FORMAT_02u,
            PNG_NUMBER_FORMAT_x,
            PNG_NUMBER_FORMAT_02x,
            PNG_NUMBER_FORMAT_fixed,
        ] {
            for _ in 0..256 {
                let mut buf = [0u8; 64];
                let num = rng.next_u64() as usize;
                unsafe {
                    let start = buf.as_ptr() as *const c_char;
                    let end = buf.as_mut_ptr().add(64) as *mut c_char;
                    let p = (l.pv.png_format_number)(start, end, fmt, num);
                    let off = if p.is_null() {
                        usize::MAX
                    } else {
                        p as usize - buf.as_ptr() as usize
                    };
                    log(format!(
                        "format_number(fmt={fmt},num={num})=off{off} buf={:02x?}",
                        buf
                    ));
                }
            }
            // boundary numbers
            for num in [0usize, 1, 9, 10, 99, 100, usize::MAX, 100000, 123456789] {
                let mut buf = [0u8; 64];
                unsafe {
                    let start = buf.as_ptr() as *const c_char;
                    let end = buf.as_mut_ptr().add(64) as *mut c_char;
                    let p = (l.pv.png_format_number)(start, end, fmt, num);
                    let off = if p.is_null() {
                        usize::MAX
                    } else {
                        p as usize - buf.as_ptr() as usize
                    };
                    log(format!("bnd format_number({fmt},{num})=off{off} {:02x?}", buf));
                }
            }
        }
    };
    diff_bare("L24-L25 safecat + format_number", &c, &r, &mut run);
}

#[test]
fn l26_l29_ascii_and_fixed() {
    let (c, r) = libs();
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            let mut rng = Rng::new(SEED ^ 26);
            for _ in 0..512 {
                // Doubles in a range libpng can represent.
                let m = (rng.u32() % 2_000_001) as f64 / 1000.0 - 1000.0;
                for prec in [1u32, 2, 5, 7, 10, 15] {
                    let mut buf = [0u8; 64];
                    (l.pv.png_ascii_from_fp)(
                        png,
                        buf.as_mut_ptr() as *mut c_char,
                        64,
                        m,
                        prec as c_uint,
                    );
                    log(format!("ascii_fp({m},{prec})={:?}", cstr(&buf)));
                }
                let fx = rng.u32() as i32;
                let mut buf = [0u8; 64];
                (l.pv.png_ascii_from_fixed)(png, buf.as_mut_ptr() as *mut c_char, 64, fx);
                log(format!("ascii_fixed({fx})={:?}", cstr(&buf)));
            }
            // png_fixed / png_fixed_ITU on in-range doubles
            let name = b"test\0";
            for _ in 0..512 {
                let d = (rng.u32() % 40_000) as f64 / 10_000.0 - 2.0;
                log(format!(
                    "fixed({d})={} itu={}",
                    (l.pv.png_fixed)(png, d, name.as_ptr() as *const c_char),
                    (l.pv.png_fixed_ITU)(png, d.abs(), name.as_ptr() as *const c_char)
                ));
            }
        })
    };
    diff("L26-L29 ascii/fixed conversion", &c, &r, &mut run);
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&x| x == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

// ---------------------------------------------------------------------------
// L30 png_check_keyword
// ---------------------------------------------------------------------------
#[test]
fn l30_check_keyword() {
    let (c, r) = libs();
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            let fixed: &[&str] = &[
                "",
                " ",
                "  ",
                "a",
                " a",
                "a ",
                " a ",
                "a  b",
                "a b",
                "Title",
                "Author",
                "\x01bad",
                "ok\x7fbad",
                "ok\u{80}hi",
                "0123456789012345678901234567890123456789012345678901234567890123456789012345678",
                "01234567890123456789012345678901234567890123456789012345678901234567890123456789",
                "0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "   leading",
                "trailing   ",
                "mid    dle",
            ];
            for k in fixed {
                let cs = std::ffi::CString::new(*k).unwrap();
                let mut newk = [0u8; 100];
                let n = (l.pv.png_check_keyword)(png, cs.as_ptr(), newk.as_mut_ptr());
                log(format!("keyword({k:?})={n} new={:?}", cstr(&newk)));
            }
            let mut rng = Rng::new(SEED ^ 30);
            for _ in 0..512 {
                let n = (rng.below(90)) as usize;
                let s: Vec<u8> = (0..n)
                    .map(|_| {
                        let v = rng.u8();
                        if v == 0 {
                            b'x'
                        } else {
                            v
                        }
                    })
                    .collect();
                let mut cs = s.clone();
                cs.push(0);
                let mut newk = [0u8; 100];
                let got = (l.pv.png_check_keyword)(
                    png,
                    cs.as_ptr() as *const c_char,
                    newk.as_mut_ptr(),
                );
                log(format!("rnd keyword({:02x?})={got} new={:?}", s, cstr(&newk)));
            }
        })
    };
    diff("L30 png_check_keyword", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// L31 CRC
// ---------------------------------------------------------------------------
#[test]
fn l31_crc() {
    let (c, r) = libs();
    let mut run = |l: &Lib| -> Report {
        read_session(l, vec![], &mut |l, png, _info| unsafe {
            let mut rng = Rng::new(SEED ^ 31);
            for _ in 0..256 {
                (l.pv.png_reset_crc)(png);
                let n = (rng.below(4097)) as usize;
                let b = rng.bytes(n);
                (l.pv.png_calculate_crc)(png, b.as_ptr(), n);
                // The running CRC is not directly readable; feed it into a
                // second chunk and observe via png_crc_finish's decision later.
                (l.pv.png_calculate_crc)(png, b.as_ptr(), 0);
                log(format!("crc n={n}"));
            }
        })
    };
    diff("L31 crc bookkeeping", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// L32..L36 row transforms
// ---------------------------------------------------------------------------
#[test]
fn l32_l35_row_transforms() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        let mut rng = Rng::new(SEED ^ 32);
        for &(ct, bd) in LEGAL {
            for width in [1u32, 2, 3, 7, 8, 9, 15, 16, 17, 33] {
                let ri = row_info(width, bd, ct);
                let n = ri.rowbytes.max(1);
                let base = rng.bytes(n + 8);
                for (name, f) in [
                    ("bgr", l.pv.png_do_bgr),
                    ("invert", l.pv.png_do_invert),
                    ("swap", l.pv.png_do_swap),
                    ("packswap", l.pv.png_do_packswap),
                ] {
                    let mut ri2 = ri;
                    let mut row = base.clone();
                    unsafe {
                        f(&mut ri2, row.as_mut_ptr());
                    }
                    log(format!(
                        "{name} ct={ct} bd={bd} w={width}: ri={ri2:?} row={:02x?}",
                        row
                    ));
                }
            }
        }
        // png_do_strip_channel: at_start 0/1, 2..4 channels, 8/16-bit
        for ct in [4i32, 2, 6] {
            for bd in [8i32, 16] {
                for at_start in [0i32, 1] {
                    for width in [1u32, 2, 5, 8] {
                        let ri = row_info(width, bd, ct);
                        let mut ri2 = ri;
                        let mut row = rng.bytes(ri.rowbytes + 8);
                        unsafe {
                            (l.pv.png_do_strip_channel)(&mut ri2, row.as_mut_ptr(), at_start);
                        }
                        log(format!(
                            "strip ct={ct} bd={bd} at={at_start} w={width}: ri={ri2:?} row={:02x?}",
                            row
                        ));
                    }
                }
            }
        }
    };
    diff_bare("L32-L36 row transforms", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// L37/L38 interlace row expansion
// ---------------------------------------------------------------------------
#[test]
fn l37_l38_interlace() {
    let (c, r) = libs();
    let mut run = |l: &Lib| {
        let mut rng = Rng::new(SEED ^ 37);
        for &(ct, bd) in LEGAL {
            for pass in 0..7i32 {
                for width in [1u32, 3, 8, 9, 16, 17, 33] {
                    // write side: row holds `width` pixels, output is the
                    // sub-sampled pass row.
                    let ri = row_info(width, bd, ct);
                    let mut ri2 = ri;
                    let mut row = rng.bytes(ri.rowbytes + 16);
                    unsafe {
                        (l.pv.png_do_write_interlace)(&mut ri2, row.as_mut_ptr(), pass);
                    }
                    log(format!(
                        "wint ct={ct} bd={bd} p={pass} w={width}: ri={ri2:?} row={:02x?}",
                        row
                    ));

                    // read side: png_do_read_interlace expands the pass row in
                    // place to `row_info->width * png_pass_inc[pass]` pixels, so
                    // the buffer must be sized for that (up to 8x wider).
                    let sub_cols = png_pass_cols(width, pass);
                    if sub_cols == 0 {
                        continue;
                    }
                    let cap = rowbytes(sub_cols.saturating_mul(8), bd, ct) + 32;
                    let mut ri3 = row_info(sub_cols, bd, ct);
                    let mut buf = rng.bytes(cap);
                    unsafe {
                        (l.pv.png_do_read_interlace)(&mut ri3, buf.as_mut_ptr(), pass, 0);
                    }
                    log(format!(
                        "rint ct={ct} bd={bd} p={pass} w={width}: ri={ri3:?} row={:02x?}",
                        buf
                    ));
                    let mut ri4 = row_info(sub_cols, bd, ct);
                    let mut buf2 = rng.bytes(cap);
                    unsafe {
                        (l.pv.png_do_read_interlace)(
                            &mut ri4,
                            buf2.as_mut_ptr(),
                            pass,
                            PNG_PACKSWAP,
                        );
                    }
                    log(format!(
                        "rintps ct={ct} bd={bd} p={pass} w={width}: ri={ri4:?} row={:02x?}",
                        buf2
                    ));
                }
            }
        }
    };
    diff_bare("L37-L38 interlace row ops", &c, &r, &mut run);
}

fn png_pass_col_shift(pass: i32) -> u32 {
    if pass > 1 {
        ((7 - pass) >> 1) as u32
    } else {
        3
    }
}
fn png_pass_start_col(pass: i32) -> u32 {
    (((1 & pass) << (3 - (((pass) + 1) >> 1))) & 7) as u32
}
fn png_pass_cols(width: u32, pass: i32) -> u32 {
    let sh = png_pass_col_shift(pass);
    (width + ((1u32 << sh) - 1 - png_pass_start_col(pass))) >> sh
}

// ---------------------------------------------------------------------------
// L39 png_read_filter_row
// ---------------------------------------------------------------------------
#[test]
fn l39_read_filter_row() {
    let (c, r) = libs();
    let mut run = |l: &Lib| -> Report {
        read_session(l, vec![], &mut |l, png, _info| unsafe {
            let mut rng = Rng::new(SEED ^ 39);
            for &(ct, bd) in LEGAL {
                for width in [1u32, 2, 3, 7, 8, 17, 33] {
                    let ri = row_info(width, bd, ct);
                    for filter in 0..5i32 {
                        let mut ri2 = ri;
                        // png_read_filter_row expects row[0..rowbytes] where the
                        // caller has already stripped the filter byte.
                        let mut row = rng.bytes(ri.rowbytes + 1);
                        let prev = rng.bytes(ri.rowbytes + 1);
                        (l.pv.png_read_filter_row)(
                            png,
                            &mut ri2,
                            row.as_mut_ptr(),
                            prev.as_ptr(),
                            filter,
                        );
                        log(format!(
                            "filter{filter} ct={ct} bd={bd} w={width}: {:02x?}",
                            row
                        ));
                    }
                }
            }
        })
    };
    diff("L39 png_read_filter_row", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// L40 png_check_IHDR
// ---------------------------------------------------------------------------
#[test]
fn l40_check_ihdr() {
    let (c, r) = libs();
    // png_check_IHDR calls png_error for many combinations, so run one
    // configuration per session to keep every longjmp isolated.
    let mut cases: Vec<(u32, u32, c_int, c_int, c_int, c_int, c_int)> = Vec::new();
    for &(ct, bd) in LEGAL {
        for il in [0i32, 1] {
            for fm in [0i32, 64] {
                cases.push((16, 16, bd, ct, il, 0, fm));
            }
        }
    }
    for &(ct, bd) in LEGAL {
        cases.push((1, 1, bd, ct, 0, 0, 0));
        cases.push((0, 16, bd, ct, 0, 0, 0));
        cases.push((16, 0, bd, ct, 0, 0, 0));
    }
    // illegal combinations
    for bd in [0i32, 3, 5, 6, 7, 9, 15, 17, 32] {
        cases.push((8, 8, bd, 0, 0, 0, 0));
    }
    for ct in [1i32, 5, 7, 8, -1] {
        cases.push((8, 8, 8, ct, 0, 0, 0));
    }
    cases.push((8, 8, 16, 3, 0, 0, 0)); // palette 16-bit: illegal
    cases.push((8, 8, 1, 2, 0, 0, 0)); // RGB 1-bit: illegal
    cases.push((8, 8, 8, 0, 2, 0, 0)); // bad interlace
    cases.push((8, 8, 8, 0, 0, 1, 0)); // bad compression
    cases.push((8, 8, 8, 0, 0, 0, 1)); // bad filter
    cases.push((0x8000_0000, 8, 8, 0, 0, 0, 0));
    cases.push((0x7fff_ffff, 8, 8, 0, 0, 0, 0));

    for (i, &(w, h, bd, ct, il, cm, fm)) in cases.iter().enumerate() {
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, _info| unsafe {
                (l.pv.png_check_IHDR)(png, w, h, bd, ct, il, cm, fm);
                log("check_IHDR returned".to_string());
            })
        };
        diff(
            &format!("L40 png_check_IHDR #{i} w={w} h={h} bd={bd} ct={ct} il={il} cm={cm} fm={fm}"),
            &c,
            &r,
            &mut run,
        );
    }
}

// ---------------------------------------------------------------------------
// L41..L43 ICC profile validation
// ---------------------------------------------------------------------------
fn icc_profile(rng: &mut Rng, tags: u32, valid: bool) -> Vec<u8> {
    let tag_bytes = 12 * tags as usize;
    let len = 132 + tag_bytes;
    let mut p = vec![0u8; len];
    p[0..4].copy_from_slice(&(len as u32).to_be_bytes());
    if valid {
        p[4..8].copy_from_slice(b"ADBE"); // preferred CMM
        p[8..12].copy_from_slice(&0x0400_0000u32.to_be_bytes()); // version 4.0
        p[12..16].copy_from_slice(b"mntr");
        p[16..20].copy_from_slice(b"RGB ");
        p[20..24].copy_from_slice(b"XYZ ");
        p[36..40].copy_from_slice(b"acsp");
        p[64..68].copy_from_slice(&0u32.to_be_bytes()); // rendering intent
        // illuminant D50
        p[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
        p[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        p[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
    } else {
        for i in 4..128 {
            p[i] = rng.u8();
        }
    }
    p[128..132].copy_from_slice(&tags.to_be_bytes());
    let mut off = 132 + tag_bytes;
    for t in 0..tags as usize {
        let b = 132 + 12 * t;
        p[b..b + 4].copy_from_slice(b"desc");
        p[b + 4..b + 8].copy_from_slice(&(off as u32).to_be_bytes());
        p[b + 8..b + 12].copy_from_slice(&0u32.to_be_bytes());
        off += 0;
    }
    p
}

#[test]
fn l41_icc_check_length() {
    let (c, r) = libs();
    let lens: &[u32] = &[
        0, 1, 4, 131, 132, 133, 143, 144, 145, 1000, 0x00ff_ffff, 0x0fff_ffff, 0x1000_0000,
        0x7fff_ffff, 0xffff_ffff,
    ];
    for (i, &n) in lens.iter().enumerate() {
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, _info| unsafe {
                let name = b"icc\0";
                let v = (l.pv.png_icc_check_length)(png, name.as_ptr() as *const c_char, n);
                log(format!("icc_check_length({n})={v}"));
            })
        };
        diff(&format!("L41 png_icc_check_length #{i} len={n}"), &c, &r, &mut run);
    }
}

#[test]
fn l42_l43_icc_check_header_and_tags() {
    let (c, r) = libs();
    let mut cases: Vec<(Vec<u8>, c_int)> = Vec::new();
    let mut rng = Rng::new(SEED ^ 42);
    for tags in [0u32, 1, 2, 5] {
        for valid in [true, false] {
            for ct in [0i32, 2, 3, 4, 6] {
                cases.push((icc_profile(&mut rng, tags, valid), ct));
            }
        }
    }
    for (i, (prof, ct)) in cases.iter().enumerate() {
        let plen = prof.len() as u32;
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, _info| unsafe {
                let name = b"icc\0";
                let h = (l.pv.png_icc_check_header)(
                    png,
                    name.as_ptr() as *const c_char,
                    plen,
                    prof.as_ptr(),
                    *ct,
                );
                log(format!("icc_check_header={h}"));
                let t = (l.pv.png_icc_check_tag_table)(
                    png,
                    name.as_ptr() as *const c_char,
                    plen,
                    prof.as_ptr(),
                );
                log(format!("icc_check_tag_table={t}"));
            })
        };
        diff(&format!("L42-L43 icc header/tags #{i} ct={ct} len={plen}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// L44 sRGB tables
// ---------------------------------------------------------------------------
#[test]
fn l44_srgb_tables() {
    let (c, r) = libs();
    unsafe {
        let ct = std::slice::from_raw_parts(c.pv.sRGB_table, 256);
        let rt = std::slice::from_raw_parts(r.pv.sRGB_table, 256);
        assert_eq!(ct, rt, "png_sRGB_table differs");
        let cb = std::slice::from_raw_parts(c.pv.sRGB_base, 512);
        let rb = std::slice::from_raw_parts(r.pv.sRGB_base, 512);
        assert_eq!(cb, rb, "png_sRGB_base differs");
        let cd = std::slice::from_raw_parts(c.pv.sRGB_delta, 512);
        let rd = std::slice::from_raw_parts(r.pv.sRGB_delta, 512);
        assert_eq!(cd, rd, "png_sRGB_delta differs");
    }
}

// ---------------------------------------------------------------------------
// L45 png_zstream_error
// ---------------------------------------------------------------------------
#[test]
fn l45_zstream_error() {
    let (c, r) = libs();
    for ret in [2i32, 1, 0, -1, -2, -3, -4, -5, -6, -7, 99, -99] {
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, _info| unsafe {
                (l.pv.png_zstream_error)(png, ret);
                log(format!("zstream_error({ret}) returned"));
            })
        };
        diff(&format!("L45 png_zstream_error ret={ret}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// L46/L47 allocation helpers
// ---------------------------------------------------------------------------
#[test]
fn l46_l47_allocation() {
    let (c, r) = libs();
    let sizes: &[usize] = &[0, 1, 7, 8, 1000, 65536, 0x7fff_ffff];
    for (i, &n) in sizes.iter().enumerate() {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                let p = (l.api.png_malloc_warn)(png, n);
                log(format!("malloc_warn({n})={}", !p.is_null()));
                if !p.is_null() {
                    (l.api.png_free)(png, p);
                }
                let q = (l.pv.png_malloc_base)(png, n);
                log(format!("malloc_base({n})={}", !q.is_null()));
                if !q.is_null() {
                    (l.api.png_free)(png, q);
                }
                let z = (l.api.png_calloc)(png, n.min(65536));
                log(format!("calloc({})={}", n.min(65536), !z.is_null()));
                if !z.is_null() {
                    let s = std::slice::from_raw_parts(z as *const u8, n.min(65536));
                    log(format!("calloc zeroed={}", s.iter().all(|&b| b == 0)));
                    (l.api.png_free)(png, z);
                }
            })
        };
        diff(&format!("L46 allocation #{i} size={n}"), &c, &r, &mut run);
    }
    // malloc_array / realloc_array
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            for &(ne, es) in &[
                (0i32, 1usize),
                (1, 1),
                (1, 8),
                (16, 16),
                (1000, 8),
                (-1, 8),
                (0x1000_0000, 16),
            ] {
                let p = (l.pv.png_malloc_array)(png, ne, es);
                log(format!("malloc_array({ne},{es})={}", !p.is_null()));
                if !p.is_null() {
                    let q = (l.pv.png_realloc_array)(png, p, ne, 4, es);
                    log(format!("realloc_array(+4)={}", !q.is_null()));
                    if !q.is_null() {
                        (l.api.png_free)(png, q);
                    }
                    (l.api.png_free)(png, p);
                }
            }
        })
    };
    diff("L47 malloc_array/realloc_array", &c, &r, &mut run);
    // png_free(NULL) must be a no-op in both
    let mut run2 = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            (l.api.png_free)(png, ptr::null_mut());
            (l.api.png_free)(ptr::null_mut(), ptr::null_mut());
            log("free(NULL) ok");
        })
    };
    diff("L46 png_free(NULL)", &c, &r, &mut run2);
}

// ---------------------------------------------------------------------------
// L48/L49 struct creation + version check
// ---------------------------------------------------------------------------
#[test]
fn l48_l49_create_struct_and_version_check() {
    let (c, r) = libs();
    let vers: &[&str] = &[
        "1.6.59", "1.6.0", "1.6.99", "1.5.0", "1.7.0", "0.0.0", "", "garbage", "1.6", "1",
    ];
    let mut run = |l: &Lib| {
        for v in vers {
            let cs = std::ffi::CString::new(*v).unwrap();
            unsafe {
                let p = (l.pv.png_create_png_struct)(
                    cs.as_ptr(),
                    ptr::null_mut(),
                    cb_error as *mut c_void,
                    cb_warn as *mut c_void,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                log(format!("create_png_struct({v})={}", !p.is_null()));
                if !p.is_null() {
                    (l.pv.png_destroy_png_struct)(p);
                }
                let w = (l.api.png_create_write_struct)(
                    cs.as_ptr(),
                    ptr::null_mut(),
                    cb_error as *mut c_void,
                    cb_warn as *mut c_void,
                );
                log(format!("create_write_struct({v})={}", !w.is_null()));
                if !w.is_null() {
                    let mut pp = w;
                    (l.api.png_destroy_write_struct)(&mut pp, ptr::null_mut());
                }
                let rd = (l.api.png_create_read_struct)(
                    cs.as_ptr(),
                    ptr::null_mut(),
                    cb_error as *mut c_void,
                    cb_warn as *mut c_void,
                );
                log(format!("create_read_struct({v})={}", !rd.is_null()));
                if !rd.is_null() {
                    let mut pp = rd;
                    (l.api.png_destroy_read_struct)(
                        &mut pp,
                        ptr::null_mut(),
                        ptr::null_mut(),
                    );
                }
            }
        }
        // NULL version string
        unsafe {
            let w = (l.api.png_create_write_struct)(
                ptr::null(),
                ptr::null_mut(),
                cb_error as *mut c_void,
                cb_warn as *mut c_void,
            );
            log(format!("create_write_struct(NULL)={}", !w.is_null()));
            if !w.is_null() {
                let mut pp = w;
                (l.api.png_destroy_write_struct)(&mut pp, ptr::null_mut());
            }
        }
    };
    diff_bare("L48-L49 struct creation + version check", &c, &r, &mut run);

    for v in vers {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                let cs = std::ffi::CString::new(*v).unwrap();
                log(format!(
                    "user_version_check({v})={}",
                    (l.pv.png_user_version_check)(png, cs.as_ptr())
                ));
            })
        };
        diff(&format!("L49 png_user_version_check {v}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// L50 unknown-chunk handling lookups
// ---------------------------------------------------------------------------
#[test]
fn l50_chunk_unknown_handling() {
    let (c, r) = libs();
    let names: &[&[u8; 5]] = &[
        b"IHDR\0", b"PLTE\0", b"IDAT\0", b"IEND\0", b"gAMA\0", b"tEXt\0", b"vpAg\0", b"prVt\0",
        b"XXXX\0", b"aaaa\0",
    ];
    for keep in [-1i32, 0, 1, 2, 3, 4, 5] {
        for num in [-1i32, 0, 1, 3] {
            let mut run = |l: &Lib| -> Report {
                read_session(l, vec![], &mut |l, png, _info| unsafe {
                    let mut list: Vec<u8> = Vec::new();
                    for n in names.iter().take(3) {
                        list.extend_from_slice(&n[..4]);
                        list.push(0);
                    }
                    (l.api.png_set_keep_unknown_chunks)(png, keep, list.as_ptr(), num);
                    for n in names {
                        let name32 = u32::from_be_bytes([n[0], n[1], n[2], n[3]]);
                        log(format!(
                            "{:?}: handle_as_unknown={} chunk_unknown_handling={}",
                            String::from_utf8_lossy(&n[..4]),
                            (l.api.png_handle_as_unknown)(png, n.as_ptr()),
                            (l.pv.png_chunk_unknown_handling)(png, name32)
                        ));
                    }
                })
            };
            diff(
                &format!("L50 keep_unknown keep={keep} num={num}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}
