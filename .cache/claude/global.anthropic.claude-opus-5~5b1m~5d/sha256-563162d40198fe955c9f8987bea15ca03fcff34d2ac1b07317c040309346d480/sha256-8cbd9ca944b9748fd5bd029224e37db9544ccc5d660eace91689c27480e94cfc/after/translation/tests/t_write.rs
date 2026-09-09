//! Differential write-side tests: CONFIGS.md rows 28..37.
//!
//! Every configuration is driven twice — once through the reference C
//! `libpng.so` and once through the translated Rust `liblibpng.so` — and the
//! produced PNG byte stream, the flush count and the error/warning log must be
//! byte-for-byte identical.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

mod common;

use common::api::*;
use common::*;
use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

// ---------------------------------------------------------------------------
// Entry points whose declaration in tests/common/api.rs does not match the
// real C prototype in c_src/include/pngpriv.h, plus the ones we need with a
// concrete `png_xy`.  Declared locally so the ABI is right.
// ---------------------------------------------------------------------------
mod lo {
    use crate::common::*;
    use core::ffi::c_int;

    crate::decl_api! {
        // pngpriv.h: (png_structrp, png_byte, png_byte, png_byte, png_byte)
        fn png_write_cICP(pp: png_structp, cp: png_byte, tf: png_byte, mc: png_byte,
                          vfrf: png_byte);
        // pngpriv.h: (png_structrp, png_const_charp, png_const_bytep, png_uint_32)
        fn png_write_iCCP(pp: png_structp, name: png_const_charp,
                          profile: png_const_bytep, proflen: png_uint_32);
        // pngpriv.h: (png_structrp, png_const_charp, png_const_charp, int)
        fn png_write_zTXt(pp: png_structp, key: png_const_charp,
                          text: png_const_charp, compression: c_int);
    }
}

// ---------------------------------------------------------------------------
// The reference `c_src/build/libpng.so` has undefined references to floor,
// pow, frexp and modf but carries no DT_NEEDED entry for libm, so it expects
// the hosting process to provide them.  Publish libm.so.6 in the global
// dynamic-symbol scope before the first dlopen of either library.
// ---------------------------------------------------------------------------
static LIBM: std::sync::OnceLock<()> = std::sync::OnceLock::new();

fn need_libm() {
    LIBM.get_or_init(|| {
        const RTLD_LAZY: c_int = 0x0001;
        const RTLD_GLOBAL: c_int = 0x0100;
        let l = unsafe {
            libloading::os::unix::Library::open(Some("libm.so.6"), RTLD_LAZY | RTLD_GLOBAL)
        };
        std::mem::forget(l.expect("dlopen libm.so.6 (RTLD_GLOBAL)"));
    });
}

fn lb() -> &'static Libs {
    need_libm();
    libs()
}

/// `png_xy` from c_src/include/pngstruct.h (8 x png_fixed_point, in this order).
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct png_xy {
    redx: png_fixed_point,
    redy: png_fixed_point,
    greenx: png_fixed_point,
    greeny: png_fixed_point,
    bluex: png_fixed_point,
    bluey: png_fixed_point,
    whitex: png_fixed_point,
    whitey: png_fixed_point,
}

// ---------------------------------------------------------------------------
// central differential driver
// ---------------------------------------------------------------------------

struct WOut {
    ok: bool,
    log: Vec<Msg>,
    data: Vec<u8>,
    flushes: usize,
}

/// Run one write configuration against a single library.
fn write_one<F: Fn(&Library, png_structp, png_infop)>(lib: &Library, f: &F) -> WOut {
    sink_reset();
    let r = capture(|| unsafe {
        let pp = new_writer(lib);
        let ip = png_create_info_struct(lib, pp);
        assert!(!ip.is_null());
        f(lib, pp, ip);
        let mut p = pp;
        let mut i = ip;
        png_destroy_write_struct(lib, &mut p, &mut i);
    });
    let (data, flushes) = sink_take();
    WOut { ok: r.out.is_some(), log: r.log, data, flushes }
}

fn logs(w: &WOut) -> Vec<String> {
    w.log.iter().map(|m| m.to_string()).collect()
}

fn cmp(label: &str, c: &WOut, r: &WOut) {
    let cl = logs(c);
    let rl = logs(r);
    if std::env::var_os("T_WRITE_TRACE").is_some() {
        eprintln!("TRACE {label}: c={}B/{} flush, rs={}B/{} flush, log={cl:?}",
            c.data.len(), c.flushes, r.data.len(), r.flushes);
    }
    assert_eq!(cl, rl, "[{label}] message log differs (C first)");
    assert_eq!(
        c.ok, r.ok,
        "[{label}] C completed={} but Rust completed={}; C log {cl:?}",
        c.ok, r.ok
    );
    if c.data != r.data {
        let n = c.data.len().min(r.data.len());
        let off = (0..n).find(|&i| c.data[i] != r.data[i]).unwrap_or(n);
        let s = off.saturating_sub(8);
        let ce = (off + 8).min(c.data.len());
        let re = (off + 8).min(r.data.len());
        panic!(
            "[{label}] output differs at byte offset {off} \
             (C len {}, Rust len {})\n  C   [{s}..{ce}]: {}\n  Rust[{s}..{re}]: {}\n  \
             C log: {cl:?}\n  Rust log: {rl:?}",
            c.data.len(),
            r.data.len(),
            hex(&c.data[s..ce]),
            hex(&r.data[s..re]),
        );
    }
    assert_eq!(
        c.flushes, r.flushes,
        "[{label}] flush count differs (C {} vs Rust {})",
        c.flushes, r.flushes
    );
    // Guard against a vacuous configuration: something must have happened.
    assert!(
        !c.data.is_empty() || !c.log.is_empty(),
        "[{label}] neither library produced output nor a message - vacuous test"
    );
}

/// The one and only comparison entry point used by every test below.
fn diff<F: Fn(&Library, png_structp, png_infop)>(label: &str, f: F) {
    let l = lb();
    let c = write_one(&l.c, &f);
    let r = write_one(&l.rs, &f);
    cmp(label, &c, &r);
}

// ---------------------------------------------------------------------------
// image helpers
// ---------------------------------------------------------------------------

fn channels(ct: c_int) -> usize {
    match ct {
        PNG_COLOR_TYPE_GRAY | PNG_COLOR_TYPE_PALETTE => 1,
        PNG_COLOR_TYPE_RGB => 3,
        PNG_COLOR_TYPE_GRAY_ALPHA => 2,
        PNG_COLOR_TYPE_RGB_ALPHA => 4,
        _ => 1,
    }
}

/// (color_type, bit_depth) — every valid combination libpng can write.
const PAIRS: [(c_int, c_int); 15] = [
    (PNG_COLOR_TYPE_GRAY, 1),
    (PNG_COLOR_TYPE_GRAY, 2),
    (PNG_COLOR_TYPE_GRAY, 4),
    (PNG_COLOR_TYPE_GRAY, 8),
    (PNG_COLOR_TYPE_GRAY, 16),
    (PNG_COLOR_TYPE_RGB, 8),
    (PNG_COLOR_TYPE_RGB, 16),
    (PNG_COLOR_TYPE_PALETTE, 1),
    (PNG_COLOR_TYPE_PALETTE, 2),
    (PNG_COLOR_TYPE_PALETTE, 4),
    (PNG_COLOR_TYPE_PALETTE, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    (PNG_COLOR_TYPE_RGB_ALPHA, 16),
];

const SIZES: [(u32, u32); 5] = [(1, 1), (7, 3), (8, 8), (9, 17), (33, 5)];

struct Img {
    w: u32,
    h: u32,
    bd: c_int,
    ct: c_int,
    il: c_int,
    rows: Vec<Vec<u8>>,
    rp: Vec<*mut png_byte>,
    pal: Vec<png_color>,
}

impl Img {
    fn new(w: u32, h: u32, bd: c_int, ct: c_int, il: c_int, seed: u64) -> Img {
        let mut rng = Rng::new(seed);
        // Generous stride: the largest a transform can make a user row is
        // 4 channels * 2 bytes per pixel.
        let stride = (w as usize) * 8 + 16;
        let mut rows: Vec<Vec<u8>> = (0..h as usize).map(|_| rng.bytes(stride)).collect();
        let rp: Vec<*mut png_byte> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
        let npal = if ct == PNG_COLOR_TYPE_PALETTE { 1usize << bd } else { 256 };
        let pal: Vec<png_color> = (0..npal)
            .map(|_| png_color { red: rng.u8(), green: rng.u8(), blue: rng.u8() })
            .collect();
        Img { w, h, bd, ct, il, rows, rp, pal }
    }
    fn rows_ptr(&self) -> *mut png_bytep {
        self.rp.as_ptr() as *mut png_bytep
    }
}

fn sbit_for(ct: c_int, bd: c_int) -> png_color_8 {
    let m = if ct == PNG_COLOR_TYPE_PALETTE { 8 } else { bd };
    let v = if m > 1 { (m - 1) as u8 } else { 1u8 };
    png_color_8 { red: v, green: v, blue: v, gray: v, alpha: v }
}

/// IHDR (+ PLTE for palette images) + caller extras + write_info.
unsafe fn head(l: &Library, pp: png_structp, ip: png_infop, img: &Img) {
    png_set_IHDR(
        l,
        pp,
        ip,
        img.w,
        img.h,
        img.bd,
        img.ct,
        img.il,
        PNG_COMPRESSION_TYPE_BASE,
        PNG_FILTER_TYPE_BASE,
    );
    if img.ct == PNG_COLOR_TYPE_PALETTE {
        png_set_PLTE(l, pp, ip, img.pal.as_ptr(), img.pal.len() as c_int);
    }
}

/// 0 = png_write_image, 1 = png_write_row loop, 2 = png_write_rows loop.
unsafe fn write_body(l: &Library, pp: png_structp, img: &Img, variant: u32) {
    match variant {
        0 => png_write_image(l, pp, img.rows_ptr()),
        1 => {
            let np = png_set_interlace_handling(l, pp);
            for _ in 0..np {
                for r in &img.rp {
                    png_write_row(l, pp, *r as png_const_bytep);
                }
            }
        }
        _ => {
            let np = png_set_interlace_handling(l, pp);
            for _ in 0..np {
                png_write_rows(l, pp, img.rows_ptr(), img.h);
            }
        }
    }
}

/// A minimal ICC profile that passes libpng's iCCP validation.
fn icc_profile(color: bool) -> Vec<u8> {
    let mut p = vec![0u8; 132];
    let len = 132u32;
    p[0..4].copy_from_slice(&len.to_be_bytes());
    p[12..16].copy_from_slice(b"mntr");
    p[16..20].copy_from_slice(if color { b"RGB " } else { b"GRAY" });
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    // rendering intent 0 (perceptual)
    p[64..68].copy_from_slice(&0u32.to_be_bytes());
    // PCS illuminant must be D50
    p[68..80].copy_from_slice(&[
        0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
    ]);
    // tag count 0
    p[128..132].copy_from_slice(&0u32.to_be_bytes());
    p
}

fn cstrings(v: &[&str]) -> Vec<CString> {
    v.iter().map(|s| CString::new(*s).unwrap()).collect()
}

fn charpp(v: &[CString]) -> Vec<*mut c_char> {
    v.iter().map(|s| s.as_ptr() as *mut c_char).collect()
}

fn printable(rng: &mut Rng, n: usize) -> CString {
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(32u8 + (rng.u8() % 94));
    }
    CString::new(v).unwrap()
}

// ===========================================================================
// row 28 — low-level chunk writers called directly
// ===========================================================================

#[test]
fn cfg28_chunk_writers() {
    need_libm();
    let mut rng = Rng::new(0x28_0000_0001);

    // ---- png_write_sig / IHDR / IEND, every colour type & depth ----------
    for &(ct, bd) in PAIRS.iter() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in SIZES.iter() {
                diff(
                    &format!("28.sig+IHDR+IEND ct={ct} bd={bd} il={il} {w}x{h}"),
                    move |l, pp, _ip| unsafe {
                        png_write_sig(l, pp);
                        png_write_IHDR(
                            l,
                            pp,
                            w,
                            h,
                            bd,
                            ct,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                            il,
                        );
                        png_write_IEND(l, pp);
                    },
                );
            }
        }
    }
    // invalid IHDR parameters (compression/filter/interlace/depth)
    for (bd, ct, cm, ft, il) in [
        (3, PNG_COLOR_TYPE_GRAY, 0, 0, 0),
        (8, 1, 0, 0, 0),
        (8, PNG_COLOR_TYPE_RGB, 1, 0, 0),
        (8, PNG_COLOR_TYPE_RGB, 0, 1, 0),
        (8, PNG_COLOR_TYPE_RGB, 0, 0, 7),
        (16, PNG_COLOR_TYPE_PALETTE, 0, 0, 0),
        (2, PNG_COLOR_TYPE_RGB_ALPHA, 0, 0, 0),
    ] {
        diff(
            &format!("28.IHDR bad bd={bd} ct={ct} cm={cm} ft={ft} il={il}"),
            move |l, pp, _ip| unsafe {
                png_write_sig(l, pp);
                png_write_IHDR(l, pp, 5, 5, bd, ct, cm, ft, il);
                png_write_IEND(l, pp);
            },
        );
    }

    // ---- png_write_PLTE --------------------------------------------------
    let pal: Vec<png_color> = (0..256)
        .map(|_| png_color { red: rng.u8(), green: rng.u8(), blue: rng.u8() })
        .collect();
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_PALETTE, 1),
        (PNG_COLOR_TYPE_PALETTE, 2),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_GRAY, 8),
    ] {
        for n in [0u32, 1, 2, 4, 16, 256, 257] {
            let pal = &pal;
            diff(
                &format!("28.PLTE ct={ct} bd={bd} n={n}"),
                move |l, pp, _ip| unsafe {
                    png_write_IHDR(l, pp, 4, 4, bd, ct, 0, 0, 0);
                    png_write_PLTE(l, pp, pal.as_ptr(), n);
                    png_write_IEND(l, pp);
                },
            );
        }
    }

    // ---- gAMA / sRGB -----------------------------------------------------
    for g in [0i32, 1, 45455, 100000, 500000, 2147483647, -1] {
        diff(&format!("28.gAMA {g}"), move |l, pp, _ip| unsafe {
            png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            png_write_gAMA_fixed(l, pp, g);
            png_write_IEND(l, pp);
        });
    }
    for i in 0i32..6 {
        diff(&format!("28.sRGB {i}"), move |l, pp, _ip| unsafe {
            png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            png_write_sRGB(l, pp, i);
            png_write_IEND(l, pp);
        });
    }

    // ---- sBIT for all five colour types ---------------------------------
    for &(ct, bd) in PAIRS.iter() {
        for k in 0..4u8 {
            let sb = match k {
                0 => sbit_for(ct, bd),
                1 => png_color_8 { red: 1, green: 1, blue: 1, gray: 1, alpha: 1 },
                2 => png_color_8 { red: 0, green: 3, blue: 3, gray: 0, alpha: 3 },
                _ => png_color_8 { red: 99, green: 99, blue: 99, gray: 99, alpha: 99 },
            };
            diff(
                &format!("28.sBIT ct={ct} bd={bd} k={k}"),
                move |l, pp, _ip| unsafe {
                    png_write_IHDR(l, pp, 3, 3, bd, ct, 0, 0, 0);
                    png_write_sBIT(l, pp, &sb, ct);
                    png_write_IEND(l, pp);
                },
            );
        }
    }

    // ---- cHRM_fixed ------------------------------------------------------
    let xys = [
        png_xy {
            redx: 64000,
            redy: 33000,
            greenx: 30000,
            greeny: 60000,
            bluex: 15000,
            bluey: 6000,
            whitex: 31270,
            whitey: 32900,
        },
        png_xy::default(),
        png_xy {
            redx: -1,
            redy: 1,
            greenx: 100000,
            greeny: -100000,
            bluex: 2147483647,
            bluey: -2147483648,
            whitex: 12345,
            whitey: 54321,
        },
    ];
    for (i, xy) in xys.iter().enumerate() {
        diff(&format!("28.cHRM {i}"), move |l, pp, _ip| unsafe {
            png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            png_write_cHRM_fixed(l, pp, xy as *const png_xy as *const c_void);
            png_write_IEND(l, pp);
        });
    }

    // ---- cICP / cLLI / mDCV ---------------------------------------------
    for (a, b, c, d) in [(1u8, 13u8, 0u8, 1u8), (0, 0, 0, 0), (255, 255, 255, 255), (9, 16, 9, 0)] {
        diff(
            &format!("28.cICP {a},{b},{c},{d}"),
            move |l, pp, _ip| unsafe {
                png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                lo::png_write_cICP(l, pp, a, b, c, d);
                png_write_IEND(l, pp);
            },
        );
    }
    for (a, b) in [(0u32, 0u32), (1000_0000, 500_0000), (u32::MAX, 1), (10, u32::MAX)] {
        diff(&format!("28.cLLI {a},{b}"), move |l, pp, _ip| unsafe {
            png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            png_write_cLLI_fixed(l, pp, a, b);
            png_write_IEND(l, pp);
        });
    }
    for i in 0..4u32 {
        let v: Vec<u16> = (0..8).map(|_| rng.u32() as u16).collect();
        let m1 = rng.u32();
        let m2 = rng.u32();
        diff(&format!("28.mDCV {i}"), move |l, pp, _ip| unsafe {
            png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            png_write_mDCV_fixed(
                l, pp, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], m1, m2,
            );
            png_write_IEND(l, pp);
        });
    }

    // ---- tRNS ------------------------------------------------------------
    let trans: Vec<u8> = (0..256).map(|_| rng.u8()).collect();
    for &(ct, bd) in PAIRS.iter() {
        for n in [0i32, 1, 3, 16, 300] {
            let trans = &trans;
            let pal = &pal;
            let tc = png_color_16 { index: 3, red: 1, green: 2, blue: 3, gray: 1 };
            diff(
                &format!("28.tRNS ct={ct} bd={bd} n={n}"),
                move |l, pp, _ip| unsafe {
                    png_write_IHDR(l, pp, 4, 4, bd, ct, 0, 0, 0);
                    if ct == PNG_COLOR_TYPE_PALETTE {
                        png_write_PLTE(l, pp, pal.as_ptr(), 1u32 << bd);
                    }
                    png_write_tRNS(l, pp, trans.as_ptr(), &tc, n, ct);
                    png_write_IEND(l, pp);
                },
            );
        }
        // gray/rgb out-of-range values
        let tc2 = png_color_16 { index: 0, red: 0xffff, green: 0, blue: 0, gray: 0xffff };
        let pal = &pal;
        diff(
            &format!("28.tRNS-hi ct={ct} bd={bd}"),
            move |l, pp, _ip| unsafe {
                png_write_IHDR(l, pp, 4, 4, bd, ct, 0, 0, 0);
                if ct == PNG_COLOR_TYPE_PALETTE {
                    png_write_PLTE(l, pp, pal.as_ptr(), 1u32 << bd);
                }
                png_write_tRNS(l, pp, ptr::null(), &tc2, 1, ct);
                png_write_IEND(l, pp);
            },
        );
    }

    // ---- bKGD ------------------------------------------------------------
    for &(ct, bd) in PAIRS.iter() {
        for k in 0..3u8 {
            let bg = match k {
                0 => png_color_16 { index: 0, red: 0, green: 0, blue: 0, gray: 0 },
                1 => png_color_16 { index: 1, red: 1, green: 2, blue: 3, gray: 1 },
                _ => png_color_16 {
                    index: 200,
                    red: 0xfffe,
                    green: 0x1234,
                    blue: 0xabcd,
                    gray: 0xfffe,
                },
            };
            let pal = &pal;
            diff(
                &format!("28.bKGD ct={ct} bd={bd} k={k}"),
                move |l, pp, _ip| unsafe {
                    png_write_IHDR(l, pp, 4, 4, bd, ct, 0, 0, 0);
                    if ct == PNG_COLOR_TYPE_PALETTE {
                        png_write_PLTE(l, pp, pal.as_ptr(), 1u32 << bd);
                    }
                    png_write_bKGD(l, pp, &bg, ct);
                    png_write_IEND(l, pp);
                },
            );
        }
    }

    // ---- hIST ------------------------------------------------------------
    let hist: Vec<u16> = (0..256).map(|_| rng.u32() as u16).collect();
    for bd in [1i32, 2, 4, 8] {
        for n in [0i32, 1, 1 << bd, (1 << bd) + 1] {
            let hist = &hist;
            let pal = &pal;
            diff(
                &format!("28.hIST bd={bd} n={n}"),
                move |l, pp, _ip| unsafe {
                    png_write_IHDR(l, pp, 4, 4, bd, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                    png_write_PLTE(l, pp, pal.as_ptr(), 1u32 << bd);
                    png_write_hIST(l, pp, hist.as_ptr(), n);
                    png_write_IEND(l, pp);
                },
            );
        }
    }

    // ---- oFFs / pHYs -----------------------------------------------------
    for (x, y, u) in [
        (0i32, 0i32, 0i32),
        (1, -1, 1),
        (i32::MIN, i32::MAX, 0),
        (12345, -54321, 2),
        (-7, 7, 99),
    ] {
        diff(&format!("28.oFFs {x},{y},{u}"), move |l, pp, _ip| unsafe {
            png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            png_write_oFFs(l, pp, x, y, u);
            png_write_IEND(l, pp);
        });
    }
    for (x, y, u) in [
        (0u32, 0u32, 0i32),
        (1, 1, 1),
        (u32::MAX, 1, 0),
        (2835, 2835, 1),
        (10, 20, 5),
    ] {
        diff(&format!("28.pHYs {x},{y},{u}"), move |l, pp, _ip| unsafe {
            png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            png_write_pHYs(l, pp, x, y, u);
            png_write_IEND(l, pp);
        });
    }

    // ---- tIME ------------------------------------------------------------
    for t in [
        png_time { year: 2026, month: 9, day: 8, hour: 12, minute: 34, second: 56 },
        png_time { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 65535, month: 255, day: 255, hour: 255, minute: 255, second: 255 },
    ] {
        diff(
            &format!("28.tIME {}-{}", t.year, t.month),
            move |l, pp, _ip| unsafe {
                png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                png_write_tIME(l, pp, &t);
                png_write_IEND(l, pp);
            },
        );
    }

    // ---- eXIf ------------------------------------------------------------
    for n in [0usize, 1, 8, 37, 512] {
        let mut ex: Vec<u8> = b"II*\0\x08\0\0\0".to_vec();
        while ex.len() < n {
            ex.push(rng.u8());
        }
        ex.truncate(n.max(0));
        let ex = &ex.clone();
        diff(&format!("28.eXIf n={n}"), move |l, pp, _ip| unsafe {
            png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            png_write_eXIf(l, pp, ex.as_ptr() as png_bytep, ex.len() as png_int_32);
            png_write_IEND(l, pp);
        });
    }

    // ---- tEXt / zTXt / iTXt ---------------------------------------------
    let long_key: String = std::iter::repeat('k').take(79).collect();
    let too_long_key: String = std::iter::repeat('K').take(120).collect();
    let keys = cstrings(&[
        "a",
        "Title",
        &long_key,
        &too_long_key,
        " lead",
        "trail ",
        "in  ner",
        "ba\td",
    ]);
    let texts = cstrings(&["", "x", "hello world\nsecond line", "\u{1}\u{2}\u{7f}"]);
    let big = printable(&mut rng, 10_000);
    for (ki, k) in keys.iter().enumerate() {
        for (ti, t) in texts.iter().chain(std::iter::once(&big)).enumerate() {
            diff(&format!("28.tEXt k{ki} t{ti}"), move |l, pp, _ip| unsafe {
                png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                png_write_tEXt(l, pp, k.as_ptr(), t.as_ptr(), 0);
                png_write_IEND(l, pp);
            });
            for comp in [-1i32, 0, 1] {
                diff(
                    &format!("28.zTXt k{ki} t{ti} c={comp}"),
                    move |l, pp, _ip| unsafe {
                        png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                        lo::png_write_zTXt(l, pp, k.as_ptr(), t.as_ptr(), comp);
                        png_write_IEND(l, pp);
                    },
                );
            }
        }
    }
    // NULL text for tEXt / zTXt
    diff("28.tEXt null-text", |l, pp, _ip| unsafe {
        png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        png_write_tEXt(l, pp, b"Key\0".as_ptr() as png_const_charp, ptr::null(), 0);
        png_write_IEND(l, pp);
    });
    diff("28.zTXt null-text", |l, pp, _ip| unsafe {
        png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        lo::png_write_zTXt(l, pp, b"Key\0".as_ptr() as png_const_charp, ptr::null(), 0);
        png_write_IEND(l, pp);
    });

    let langs = cstrings(&["", "en", "en-GB", "de-DE-1996"]);
    let lkeys = cstrings(&["", "Titel", "Ünicode Schlüssel"]);
    for (ki, k) in keys.iter().take(4).enumerate() {
        for comp in [-1i32, 0, 1, 2, 3] {
            for (li, lang) in langs.iter().enumerate() {
                let lk = &lkeys[li % lkeys.len()];
                let t = if li % 2 == 0 { &texts[2] } else { &big };
                diff(
                    &format!("28.iTXt k{ki} c={comp} l{li}"),
                    move |l, pp, _ip| unsafe {
                        png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                        png_write_iTXt(
                            l,
                            pp,
                            comp,
                            k.as_ptr(),
                            lang.as_ptr(),
                            lk.as_ptr(),
                            t.as_ptr(),
                        );
                        png_write_IEND(l, pp);
                    },
                );
            }
        }
    }
    diff("28.iTXt nulls", |l, pp, _ip| unsafe {
        png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        png_write_iTXt(
            l,
            pp,
            1,
            b"Key\0".as_ptr() as png_const_charp,
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        png_write_IEND(l, pp);
    });

    // ---- sCAL_s ----------------------------------------------------------
    let scal = cstrings(&[
        "1",
        "0.5",
        "-3.25e-7",
        "12345678901234567890123456789",
        "1e+300000000",
    ]);
    for unit in [0i32, 1, 2, 3] {
        for (i, w) in scal.iter().enumerate() {
            let h = &scal[(i + 1) % scal.len()];
            diff(
                &format!("28.sCAL_s u={unit} i={i}"),
                move |l, pp, _ip| unsafe {
                    png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                    png_write_sCAL_s(l, pp, unit, w.as_ptr(), h.as_ptr());
                    png_write_IEND(l, pp);
                },
            );
        }
    }
    // over-long (buffer too small) case
    let long_w = CString::new("1".repeat(40)).unwrap();
    let long_h = CString::new("2".repeat(40)).unwrap();
    diff("28.sCAL_s toolong", |l, pp, _ip| unsafe {
        png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        png_write_sCAL_s(l, pp, 1, long_w.as_ptr(), long_h.as_ptr());
        png_write_IEND(l, pp);
    });

    // ---- pCAL: every equation type, 0..3 parameters ---------------------
    let purpose = CString::new("Calibration").unwrap();
    let units = CString::new("metres").unwrap();
    let pv = cstrings(&["1.5", "-2", "3.25e3"]);
    let pp_arr = charpp(&pv);
    for typ in 0i32..5 {
        for np in 0i32..4 {
            let purpose = &purpose;
            let units = &units;
            let pp_arr = &pp_arr;
            diff(
                &format!("28.pCAL t={typ} n={np}"),
                move |l, pp, _ip| unsafe {
                    png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                    png_write_pCAL(
                        l,
                        pp,
                        purpose.as_ptr() as png_charp,
                        -100,
                        100,
                        typ,
                        np,
                        units.as_ptr(),
                        pp_arr.as_ptr() as *mut png_charp,
                    );
                    png_write_IEND(l, pp);
                },
            );
        }
    }

    // ---- iCCP ------------------------------------------------------------
    let prof_rgb = icc_profile(true);
    let mut prof_big = icc_profile(true);
    prof_big.resize(1024, 0);
    let n = prof_big.len() as u32;
    prof_big[0..4].copy_from_slice(&n.to_be_bytes());
    for (i, prof) in [&prof_rgb, &prof_big].iter().enumerate() {
        for (ki, k) in keys.iter().take(3).enumerate() {
            let prof = *prof;
            diff(
                &format!("28.iCCP p{i} k{ki}"),
                move |l, pp, _ip| unsafe {
                    png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                    lo::png_write_iCCP(
                        l,
                        pp,
                        k.as_ptr(),
                        prof.as_ptr(),
                        prof.len() as png_uint_32,
                    );
                    png_write_IEND(l, pp);
                },
            );
        }
    }
    // short / inconsistent profiles (error paths)
    let short = vec![0u8; 100];
    diff("28.iCCP short", |l, pp, _ip| unsafe {
        png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        lo::png_write_iCCP(
            l,
            pp,
            b"icc\0".as_ptr() as png_const_charp,
            short.as_ptr(),
            short.len() as png_uint_32,
        );
        png_write_IEND(l, pp);
    });
    let mut bad = icc_profile(true);
    bad[0..4].copy_from_slice(&999u32.to_be_bytes());
    diff("28.iCCP badlen", |l, pp, _ip| unsafe {
        png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        lo::png_write_iCCP(
            l,
            pp,
            b"icc\0".as_ptr() as png_const_charp,
            bad.as_ptr(),
            bad.len() as png_uint_32,
        );
        png_write_IEND(l, pp);
    });

    // ---- sPLT ------------------------------------------------------------
    let mut ent: Vec<png_sPLT_entry> = (0..32)
        .map(|_| png_sPLT_entry {
            red: rng.u32() as u16,
            green: rng.u32() as u16,
            blue: rng.u32() as u16,
            alpha: rng.u32() as u16,
            frequency: rng.u32() as u16,
        })
        .collect();
    let spl_name = CString::new("split palette").unwrap();
    for depth in [8u8, 16u8] {
        for ne in [0i32, 1, 5, 32] {
            let e = ent.as_mut_ptr();
            let nm = spl_name.as_ptr() as png_charp;
            diff(
                &format!("28.sPLT d={depth} n={ne}"),
                move |l, pp, _ip| unsafe {
                    let s = png_sPLT_t { name: nm, depth, entries: e, nentries: ne };
                    png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                    png_write_sPLT(l, pp, &s);
                    png_write_IEND(l, pp);
                },
            );
        }
    }

    // ---- png_write_chunk and the start/data/end trio ---------------------
    for i in 0..12u32 {
        let name: Vec<u8> = {
            let mut v = Vec::with_capacity(5);
            for j in 0..4 {
                let up = (rng.u8() & 1) == 0;
                let c = b'a' + (rng.u8() % 26);
                v.push(if up { c - 32 } else { c });
                let _ = j;
            }
            v.push(0);
            v
        };
        let len = (rng.below(300)) as usize;
        let data = rng.bytes(len);
        let nm = name.clone();
        let d = data.clone();
        diff(&format!("28.chunk {i} len={len}"), move |l, pp, _ip| unsafe {
            png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            png_write_chunk(l, pp, nm.as_ptr(), d.as_ptr(), d.len());
            png_write_IEND(l, pp);
        });
        let nm2 = name.clone();
        let d2 = data.clone();
        diff(
            &format!("28.chunk-split {i} len={len}"),
            move |l, pp, _ip| unsafe {
                png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                png_write_chunk_start(l, pp, nm2.as_ptr(), d2.len() as png_uint_32);
                let mut off = 0usize;
                let mut step = 1usize;
                while off < d2.len() {
                    let n = step.min(d2.len() - off);
                    png_write_chunk_data(l, pp, d2.as_ptr().add(off), n);
                    off += n;
                    step = step * 2 + 1;
                }
                png_write_chunk_data(l, pp, ptr::null(), 0);
                png_write_chunk_end(l, pp);
                png_write_IEND(l, pp);
            },
        );
    }
    // zero-length chunk both ways
    diff("28.chunk empty", |l, pp, _ip| unsafe {
        png_write_IHDR(l, pp, 3, 3, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        png_write_chunk(l, pp, b"teSt\0".as_ptr(), ptr::null(), 0);
        png_write_chunk_start(l, pp, b"teSt\0".as_ptr(), 0);
        png_write_chunk_end(l, pp);
        png_write_IEND(l, pp);
    });
}

// ===========================================================================
// row 29 — png_write_png one-shot
// ===========================================================================

const TRANSFORMS: [(&str, c_int); 11] = [
    ("IDENTITY", PNG_TRANSFORM_IDENTITY),
    ("PACKING", PNG_TRANSFORM_PACKING),
    ("PACKSWAP", PNG_TRANSFORM_PACKSWAP),
    ("INVERT_MONO", PNG_TRANSFORM_INVERT_MONO),
    ("SHIFT", PNG_TRANSFORM_SHIFT),
    ("BGR", PNG_TRANSFORM_BGR),
    ("SWAP_ALPHA", PNG_TRANSFORM_SWAP_ALPHA),
    ("SWAP_ENDIAN", PNG_TRANSFORM_SWAP_ENDIAN),
    ("INVERT_ALPHA", PNG_TRANSFORM_INVERT_ALPHA),
    ("STRIP_FILLER_BEFORE", PNG_TRANSFORM_STRIP_FILLER_BEFORE),
    ("STRIP_FILLER_AFTER", PNG_TRANSFORM_STRIP_FILLER_AFTER),
];

fn run_write_png(label: &str, img: &Img, tf: c_int) {
    diff(label, |l, pp, ip| unsafe {
        head(l, pp, ip, img);
        if (tf & PNG_TRANSFORM_SHIFT) != 0 {
            let sb = sbit_for(img.ct, img.bd);
            png_set_sBIT(l, pp, ip, &sb);
        }
        png_set_rows(l, pp, ip, img.rows_ptr());
        png_write_png(l, pp, ip, tf, ptr::null_mut());
    });
}

#[test]
fn cfg29_write_png_oneshot() {
    need_libm();
    // full grid with the identity transform
    for &(ct, bd) in PAIRS.iter() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in SIZES.iter() {
                let seed = 0x2900_0000u64 ^ ((ct as u64) << 24) ^ ((bd as u64) << 16)
                    ^ ((il as u64) << 12)
                    ^ ((w as u64) << 6)
                    ^ h as u64;
                let img = Img::new(w, h, bd, ct, il, seed);
                run_write_png(
                    &format!("29.IDENTITY ct={ct} bd={bd} il={il} {w}x{h}"),
                    &img,
                    PNG_TRANSFORM_IDENTITY,
                );
            }
        }
    }
    // the full transform x (color_type, bit_depth) x interlace x size grid
    for &(tname, tf) in TRANSFORMS.iter().skip(1) {
        for &(ct, bd) in PAIRS.iter() {
            for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                for &(w, h) in SIZES.iter() {
                    let seed = 0x2911_0000u64
                        ^ ((tf as u64) << 32)
                        ^ ((ct as u64) << 24)
                        ^ ((bd as u64) << 16)
                        ^ ((il as u64) << 12)
                        ^ ((w as u64) << 6)
                        ^ h as u64;
                    let img = Img::new(w, h, bd, ct, il, seed);
                    run_write_png(
                        &format!("29.{tname} ct={ct} bd={bd} il={il} {w}x{h}"),
                        &img,
                        tf,
                    );
                }
            }
        }
    }
    // a couple of combined transforms
    for &(a, b) in &[
        (PNG_TRANSFORM_BGR, PNG_TRANSFORM_INVERT_ALPHA),
        (PNG_TRANSFORM_SWAP_ENDIAN, PNG_TRANSFORM_SWAP_ALPHA),
        (PNG_TRANSFORM_PACKING, PNG_TRANSFORM_PACKSWAP),
        (PNG_TRANSFORM_STRIP_FILLER_BEFORE, PNG_TRANSFORM_STRIP_FILLER_AFTER),
    ] {
        for &(ct, bd) in &[
            (PNG_COLOR_TYPE_RGB_ALPHA, 16),
            (PNG_COLOR_TYPE_GRAY, 2),
            (PNG_COLOR_TYPE_RGB, 8),
        ] {
            let img = Img::new(9, 6, bd, ct, 0, 0x2922_0000 ^ (a as u64) ^ ((b as u64) << 20));
            run_write_png(&format!("29.combo {a:#x}|{b:#x} ct={ct} bd={bd}"), &img, a | b);
        }
    }
    // no rows set at all (png_app_error path)
    diff("29.no-rows", |l, pp, ip| unsafe {
        png_set_IHDR(l, pp, ip, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        png_write_png(l, pp, ip, PNG_TRANSFORM_IDENTITY, ptr::null_mut());
    });
}

// ===========================================================================
// row 30 — the low-level pipeline, three row-writing entry points
// ===========================================================================

#[test]
fn cfg30_lowlevel_pipeline() {
    need_libm();
    for &(ct, bd) in PAIRS.iter() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in SIZES.iter() {
                let seed = 0x3000_0000u64 ^ ((ct as u64) << 24) ^ ((bd as u64) << 16)
                    ^ ((il as u64) << 12)
                    ^ ((w as u64) << 6)
                    ^ h as u64;
                let img = Img::new(w, h, bd, ct, il, seed);
                let mut outs: Vec<Vec<u8>> = Vec::new();
                for variant in 0..3u32 {
                    let label = format!(
                        "30.v{variant} ct={ct} bd={bd} il={il} {w}x{h}"
                    );
                    diff(&label, |l, pp, ip| unsafe {
                        head(l, pp, ip, &img);
                        png_write_info(l, pp, ip);
                        write_body(l, pp, &img, variant);
                        png_write_end(l, pp, ip);
                    });
                    // keep the C output of this variant for the cross-check
                    let c = write_one(&lb().c, &|l: &Library,
                                                   pp: png_structp,
                                                   ip: png_infop| unsafe {
                        head(l, pp, ip, &img);
                        png_write_info(l, pp, ip);
                        write_body(l, pp, &img, variant);
                        png_write_end(l, pp, ip);
                    });
                    outs.push(c.data);
                }
                assert_eq!(
                    outs[0], outs[1],
                    "30: png_write_image and png_write_row disagree \
                     (ct={ct} bd={bd} il={il} {w}x{h})"
                );
                assert_eq!(
                    outs[0], outs[2],
                    "30: png_write_image and png_write_rows disagree \
                     (ct={ct} bd={bd} il={il} {w}x{h})"
                );
            }
        }
    }
    // png_write_info_before_PLTE + explicit PLTE ordering
    let img = Img::new(8, 8, 4, PNG_COLOR_TYPE_PALETTE, 0, 0x3033);
    diff("30.info_before_PLTE", |l, pp, ip| unsafe {
        head(l, pp, ip, &img);
        png_write_info_before_PLTE(l, pp, ip);
        png_write_info(l, pp, ip);
        png_write_image(l, pp, img.rows_ptr());
        png_write_end(l, pp, ip);
    });
    // png_write_end with a NULL info_ptr
    diff("30.end-null-info", |l, pp, ip| unsafe {
        head(l, pp, ip, &img);
        png_write_info(l, pp, ip);
        png_write_image(l, pp, img.rows_ptr());
        png_write_end(l, pp, ptr::null_mut());
    });
}

// ===========================================================================
// row 31 — png_set_filter
// ===========================================================================

#[test]
fn cfg31_set_filter() {
    need_libm();
    let masks: [(&str, c_int); 8] = [
        ("NO_FILTERS", PNG_NO_FILTERS),
        ("NONE", PNG_FILTER_NONE),
        ("SUB", PNG_FILTER_SUB),
        ("UP", PNG_FILTER_UP),
        ("AVG", PNG_FILTER_AVG),
        ("PAETH", PNG_FILTER_PAETH),
        ("ALL", PNG_ALL_FILTERS),
        ("SUB|PAETH", PNG_FILTER_SUB | PNG_FILTER_PAETH),
    ];
    let pairs: [(c_int, c_int); 6] = [
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ];
    for &(mname, mask) in masks.iter() {
        for &(ct, bd) in pairs.iter() {
            for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                for pre in [true, false] {
                    let img = Img::new(
                        17,
                        11,
                        bd,
                        ct,
                        il,
                        0x3100_0000 ^ (mask as u64) ^ ((ct as u64) << 8) ^ ((bd as u64) << 16),
                    );
                    let when = if pre { "pre" } else { "post" };
                    diff(
                        &format!("31.{mname} ct={ct} bd={bd} il={il} {when}"),
                        |l, pp, ip| unsafe {
                            head(l, pp, ip, &img);
                            if pre {
                                png_set_filter(l, pp, PNG_FILTER_TYPE_BASE, mask);
                            }
                            png_write_info(l, pp, ip);
                            if !pre {
                                png_set_filter(l, pp, PNG_FILTER_TYPE_BASE, mask);
                            }
                            png_write_image(l, pp, img.rows_ptr());
                            png_write_end(l, pp, ip);
                        },
                    );
                }
            }
        }
    }
    // invalid method / mask
    for (method, mask) in [(1i32, PNG_ALL_FILTERS), (64, PNG_FILTER_NONE), (0, 0x07)] {
        let img = Img::new(9, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0x3199);
        diff(
            &format!("31.bad method={method} mask={mask:#x}"),
            |l, pp, ip| unsafe {
                head(l, pp, ip, &img);
                png_set_filter(l, pp, method, mask);
                png_write_info(l, pp, ip);
                png_write_image(l, pp, img.rows_ptr());
                png_write_end(l, pp, ip);
            },
        );
    }
}

// ===========================================================================
// row 32 — zlib compression parameters
// ===========================================================================

#[test]
fn cfg32_compression_params() {
    need_libm();
    let levels = [-1i32, 0, 1, 6, 9];
    let strategies = [0i32, 1, 2, 3, 4];
    let windows = [8i32, 9, 15];
    let mems = [1i32, 8, 9];
    let img = Img::new(64, 64, 8, PNG_COLOR_TYPE_RGB, 0, 0x3200_1234);

    // the complete 5 x 5 x 3 x 3 = 225 element cross product
    let mut combos: Vec<(i32, i32, i32, i32)> = Vec::new();
    for &lv in levels.iter() {
        for &st in strategies.iter() {
            for &wb in windows.iter() {
                for &ml in mems.iter() {
                    combos.push((lv, st, wb, ml));
                }
            }
        }
    }
    for (lv, st, wb, ml) in combos {
        diff(
            &format!("32.level={lv} strategy={st} window={wb} mem={ml}"),
            |l, pp, ip| unsafe {
                png_set_compression_level(l, pp, lv);
                png_set_compression_strategy(l, pp, st);
                png_set_compression_window_bits(l, pp, wb);
                png_set_compression_mem_level(l, pp, ml);
                head(l, pp, ip, &img);
                png_write_info(l, pp, ip);
                png_write_image(l, pp, img.rows_ptr());
                png_write_end(l, pp, ip);
            },
        );
    }
    // out-of-range values (warning paths)
    for (lv, st, wb, ml) in [
        (10i32, 0i32, 15i32, 8i32),
        (-2, 0, 15, 8),
        (6, 5, 15, 8),
        (6, 0, 16, 8),
        (6, 0, 7, 8),
        (6, 0, 15, 0),
        (6, 0, 15, 10),
    ] {
        diff(
            &format!("32.bad level={lv} strategy={st} window={wb} mem={ml}"),
            |l, pp, ip| unsafe {
                png_set_compression_level(l, pp, lv);
                png_set_compression_strategy(l, pp, st);
                png_set_compression_window_bits(l, pp, wb);
                png_set_compression_mem_level(l, pp, ml);
                png_set_compression_method(l, pp, 8);
                head(l, pp, ip, &img);
                png_write_info(l, pp, ip);
                png_write_image(l, pp, img.rows_ptr());
                png_write_end(l, pp, ip);
            },
        );
    }
    // invalid compression method
    for m in [0i32, 7, 9] {
        diff(&format!("32.method={m}"), |l, pp, ip| unsafe {
            png_set_compression_method(l, pp, m);
            head(l, pp, ip, &img);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, img.rows_ptr());
            png_write_end(l, pp, ip);
        });
    }
}

// ===========================================================================
// row 33 — compression buffer size (multi-IDAT splitting)
// ===========================================================================

#[test]
fn cfg33_compression_buffer_size() {
    need_libm();
    let sizes = [1usize, 2, 3, 1024, 8192, 65536];

    // png_get_compression_buffer_size must agree, default and after each set
    let (dc, dr) = both(|l| unsafe {
        let pp = new_writer(l);
        let g = png_get_compression_buffer_size(l, pp);
        let mut p = pp;
        png_destroy_write_struct(l, &mut p, ptr::null_mut());
        g
    });
    assert_eq!(dc, dr, "33: default png_get_compression_buffer_size differs");
    for &sz in sizes.iter() {
        let r = capture(|| {
            both(|l| unsafe {
                let pp = new_writer(l);
                png_set_compression_buffer_size(l, pp, sz);
                let g = png_get_compression_buffer_size(l, pp);
                let mut p = pp;
                png_destroy_write_struct(l, &mut p, ptr::null_mut());
                g
            })
        });
        let (a, b) = r.out.expect("33: set/get buffer size must not error");
        assert_eq!(a, b, "33: png_get_compression_buffer_size({sz}) differs");
    }

    for &sz in sizes.iter() {
        let (w, h) = if sz < 6 { (16u32, 16u32) } else { (64, 64) };
        let img = Img::new(w, h, 8, PNG_COLOR_TYPE_RGB, 0, 0x3300_0000 ^ sz as u64);
        diff(&format!("33.bufsize={sz} {w}x{h}"), |l, pp, ip| unsafe {
            png_set_compression_buffer_size(l, pp, sz);
            head(l, pp, ip, &img);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, img.rows_ptr());
            png_write_end(l, pp, ip);
        });
        // and set *after* png_write_info (zowner busy path)
        diff(&format!("33.bufsize-late={sz}"), |l, pp, ip| unsafe {
            head(l, pp, ip, &img);
            png_write_info(l, pp, ip);
            png_write_row(l, pp, img.rp[0] as png_const_bytep);
            png_set_compression_buffer_size(l, pp, sz);
            for r in img.rp.iter().skip(1) {
                png_write_row(l, pp, *r as png_const_bytep);
            }
            png_write_end(l, pp, ip);
        });
    }
    // zero size (error path)
    let img = Img::new(8, 8, 8, PNG_COLOR_TYPE_RGB, 0, 0x3301);
    diff("33.bufsize=0", |l, pp, ip| unsafe {
        png_set_compression_buffer_size(l, pp, 0);
        head(l, pp, ip, &img);
        png_write_info(l, pp, ip);
        png_write_image(l, pp, img.rows_ptr());
        png_write_end(l, pp, ip);
    });
}

// ===========================================================================
// row 34 — png_set_flush / png_write_flush
// ===========================================================================

#[test]
fn cfg34_flush() {
    need_libm();
    let h = 16u32;
    let img = Img::new(20, h, 8, PNG_COLOR_TYPE_RGB, 0, 0x3400_5678);
    for rows in [0i32, 1, 2, h as i32, -3] {
        diff(&format!("34.set_flush({rows})"), |l, pp, ip| unsafe {
            png_set_flush(l, pp, rows);
            head(l, pp, ip, &img);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, img.rows_ptr());
            png_write_end(l, pp, ip);
        });
        // explicit mid-image flushes as well
        diff(
            &format!("34.set_flush({rows})+explicit"),
            |l, pp, ip| unsafe {
                png_set_flush(l, pp, rows);
                head(l, pp, ip, &img);
                png_write_info(l, pp, ip);
                for (i, r) in img.rp.iter().enumerate() {
                    png_write_row(l, pp, *r as png_const_bytep);
                    if i % 5 == 3 {
                        png_write_flush(l, pp);
                    }
                }
                png_write_flush(l, pp);
                png_write_end(l, pp, ip);
            },
        );
    }
    // flush before any row and after the last row
    diff("34.flush-edges", |l, pp, ip| unsafe {
        png_set_flush(l, pp, 3);
        head(l, pp, ip, &img);
        png_write_flush(l, pp);
        png_write_info(l, pp, ip);
        png_write_flush(l, pp);
        png_write_image(l, pp, img.rows_ptr());
        png_write_flush(l, pp);
        png_write_end(l, pp, ip);
        png_write_flush(l, pp);
    });
    // interlaced with flushing
    let img2 = Img::new(13, 9, 4, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_ADAM7, 0x3401);
    for rows in [1i32, 2, 9] {
        diff(&format!("34.interlaced flush={rows}"), |l, pp, ip| unsafe {
            png_set_flush(l, pp, rows);
            head(l, pp, ip, &img2);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, img2.rows_ptr());
            png_write_end(l, pp, ip);
        });
    }
}

// ===========================================================================
// row 35 — text chunks through png_set_text
// ===========================================================================

#[test]
fn cfg35_text_chunks() {
    need_libm();
    let mut rng = Rng::new(0x3500_0001);
    let img = Img::new(8, 8, 8, PNG_COLOR_TYPE_RGB, 0, 0x3500_0002);

    let k1 = CString::new("k").unwrap();
    let k79 = CString::new("K".repeat(79)).unwrap();
    let empty = CString::new("").unwrap();
    let one = CString::new("Z").unwrap();
    let big = printable(&mut rng, 10_000);
    let lang = CString::new("en-GB").unwrap();
    let lkey = CString::new("Schlüssel").unwrap();

    let comps = [
        ("tEXt", PNG_TEXT_COMPRESSION_NONE),
        ("zTXt", PNG_TEXT_COMPRESSION_zTXt),
        ("iTXt-none", PNG_ITXT_COMPRESSION_NONE),
        ("iTXt-zTXt", PNG_ITXT_COMPRESSION_zTXt),
    ];
    for &(cname, comp) in comps.iter() {
        for (kn, key) in [("k1", &k1), ("k79", &k79)] {
            for (tn, txt) in [("empty", &empty), ("one", &one), ("10k", &big)] {
                let itxt = comp >= PNG_ITXT_COMPRESSION_NONE;
                diff(
                    &format!("35.{cname} {kn} {tn}"),
                    |l, pp, ip| unsafe {
                        let t = png_text {
                            compression: comp,
                            key: key.as_ptr() as png_charp,
                            text: txt.as_ptr() as png_charp,
                            text_length: 0,
                            itxt_length: 0,
                            lang: if itxt { lang.as_ptr() as png_charp } else { ptr::null_mut() },
                            lang_key: if itxt {
                                lkey.as_ptr() as png_charp
                            } else {
                                ptr::null_mut()
                            },
                        };
                        head(l, pp, ip, &img);
                        png_set_text(l, pp, ip, &t, 1);
                        png_write_info(l, pp, ip);
                        png_write_image(l, pp, img.rows_ptr());
                        png_write_end(l, pp, ip);
                    },
                );
            }
        }
    }

    // several text chunks at once, mixed kinds, before and after IDAT
    diff("35.multi", |l, pp, ip| unsafe {
        let ts = [
            png_text {
                compression: PNG_TEXT_COMPRESSION_NONE,
                key: k1.as_ptr() as png_charp,
                text: one.as_ptr() as png_charp,
                text_length: 0,
                itxt_length: 0,
                lang: ptr::null_mut(),
                lang_key: ptr::null_mut(),
            },
            png_text {
                compression: PNG_TEXT_COMPRESSION_zTXt,
                key: k79.as_ptr() as png_charp,
                text: big.as_ptr() as png_charp,
                text_length: 0,
                itxt_length: 0,
                lang: ptr::null_mut(),
                lang_key: ptr::null_mut(),
            },
            png_text {
                compression: PNG_ITXT_COMPRESSION_zTXt,
                key: k1.as_ptr() as png_charp,
                text: big.as_ptr() as png_charp,
                text_length: 0,
                itxt_length: 0,
                lang: lang.as_ptr() as png_charp,
                lang_key: lkey.as_ptr() as png_charp,
            },
        ];
        head(l, pp, ip, &img);
        png_set_text(l, pp, ip, ts.as_ptr(), 3);
        png_write_info(l, pp, ip);
        png_write_image(l, pp, img.rows_ptr());
        // more text after the image data -> written by png_write_end
        png_set_text(l, pp, ip, ts.as_ptr(), 3);
        png_write_end(l, pp, ip);
    });

    // invalid compression values
    for comp in [-3i32, 3, 100] {
        diff(&format!("35.bad-comp {comp}"), |l, pp, ip| unsafe {
            let t = png_text {
                compression: comp,
                key: k1.as_ptr() as png_charp,
                text: one.as_ptr() as png_charp,
                text_length: 0,
                itxt_length: 0,
                lang: ptr::null_mut(),
                lang_key: ptr::null_mut(),
            };
            head(l, pp, ip, &img);
            png_set_text(l, pp, ip, &t, 1);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, img.rows_ptr());
            png_write_end(l, pp, ip);
        });
    }

    // png_set_text_compression_* knobs
    let tlevels = [-1i32, 0, 1, 9];
    let tstrats = [0i32, 1, 2, 3, 4];
    let twins = [8i32, 9, 15];
    let tmems = [1i32, 8, 9];
    let mut rng2 = Rng::new(0x3500_cafe);
    for i in 0..20u32 {
        let lv = tlevels[rng2.below(4) as usize];
        let st = tstrats[rng2.below(5) as usize];
        let wb = twins[rng2.below(3) as usize];
        let ml = tmems[rng2.below(3) as usize];
        let comp = comps[(i % 4) as usize].1;
        diff(
            &format!("35.textcomp lv={lv} st={st} wb={wb} ml={ml} c={comp}"),
            |l, pp, ip| unsafe {
                png_set_text_compression_level(l, pp, lv);
                png_set_text_compression_strategy(l, pp, st);
                png_set_text_compression_window_bits(l, pp, wb);
                png_set_text_compression_mem_level(l, pp, ml);
                png_set_text_compression_method(l, pp, 8);
                let itxt = comp >= PNG_ITXT_COMPRESSION_NONE;
                let t = png_text {
                    compression: comp,
                    key: k1.as_ptr() as png_charp,
                    text: big.as_ptr() as png_charp,
                    text_length: 0,
                    itxt_length: 0,
                    lang: if itxt { lang.as_ptr() as png_charp } else { ptr::null_mut() },
                    lang_key: if itxt { lkey.as_ptr() as png_charp } else { ptr::null_mut() },
                };
                head(l, pp, ip, &img);
                png_set_text(l, pp, ip, &t, 1);
                png_write_info(l, pp, ip);
                png_write_image(l, pp, img.rows_ptr());
                png_write_end(l, pp, ip);
            },
        );
    }
    // invalid text compression method
    for m in [0i32, 9] {
        diff(&format!("35.textmethod={m}"), |l, pp, ip| unsafe {
            png_set_text_compression_method(l, pp, m);
            let t = png_text {
                compression: PNG_TEXT_COMPRESSION_zTXt,
                key: k1.as_ptr() as png_charp,
                text: big.as_ptr() as png_charp,
                text_length: 0,
                itxt_length: 0,
                lang: ptr::null_mut(),
                lang_key: ptr::null_mut(),
            };
            head(l, pp, ip, &img);
            png_set_text(l, pp, ip, &t, 1);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, img.rows_ptr());
            png_write_end(l, pp, ip);
        });
    }
}

// ===========================================================================
// row 36 — all ancillary chunks through png_set_*
// ===========================================================================

type Setter<'a> = Box<dyn Fn(&Library, png_structp, png_infop) + 'a>;

#[test]
fn cfg36_ancillary_chunks() {
    need_libm();
    let mut rng = Rng::new(0x3600_0001);

    // palette 8-bit base: allows PLTE, hIST, palette tRNS/bKGD/sBIT
    let pimg = Img::new(12, 9, 8, PNG_COLOR_TYPE_PALETTE, 0, 0x3600_0002);
    // RGB 8-bit base
    let rimg = Img::new(12, 9, 8, PNG_COLOR_TYPE_RGB, 0, 0x3600_0003);
    // gray 16-bit base (for the GRAY ICC profile / gray tRNS)
    let gimg = Img::new(12, 9, 16, PNG_COLOR_TYPE_GRAY, 0, 0x3600_0004);

    let prof_rgb = icc_profile(true);
    let prof_gray = icc_profile(false);
    let iccname = CString::new("Some profile").unwrap();
    let purpose = CString::new("Purpose").unwrap();
    let units = CString::new("units").unwrap();
    let pv = cstrings(&["1", "-2.5", "3e2", "0.0001"]);
    let pvp = charpp(&pv);
    let scal_w = CString::new("2.5").unwrap();
    let scal_h = CString::new("0.75e2").unwrap();
    let scal_bad = CString::new("-0.75e2").unwrap();
    let exif: Vec<u8> = {
        let mut v = b"II*\0\x08\0\0\0".to_vec();
        v.extend(rng.bytes(24));
        v
    };
    let hist: Vec<u16> = (0..256).map(|_| rng.u32() as u16).collect();
    let trans: Vec<u8> = (0..256).map(|_| rng.u8()).collect();
    let spl_entries: Vec<png_sPLT_entry> = (0..7)
        .map(|_| png_sPLT_entry {
            red: rng.u32() as u16,
            green: rng.u32() as u16,
            blue: rng.u32() as u16,
            alpha: rng.u32() as u16,
            frequency: rng.u32() as u16,
        })
        .collect();
    let spl_name = CString::new("suggested").unwrap();
    let tm = png_time { year: 1999, month: 12, day: 31, hour: 23, minute: 59, second: 60 };

    // ---- singly ----------------------------------------------------------
    let mut cases: Vec<(&str, &Img, Setter)> = Vec::new();

    cases.push(("gAMA", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_gAMA(l, pp, ip, 0.45455);
    })));
    cases.push(("gAMA_fixed", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_gAMA_fixed(l, pp, ip, 45455);
    })));
    cases.push(("gAMA_fixed-0", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_gAMA_fixed(l, pp, ip, 0);
    })));
    cases.push(("cHRM", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_cHRM(l, pp, ip, 0.3127, 0.329, 0.64, 0.33, 0.30, 0.60, 0.15, 0.06);
    })));
    cases.push(("cHRM_fixed", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_cHRM_fixed(l, pp, ip, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
    })));
    cases.push(("cHRM_XYZ_fixed", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_cHRM_XYZ_fixed(
            l, pp, ip, 41239, 21264, 1933, 35758, 71517, 11919, 18048, 7219, 95053,
        );
    })));
    cases.push(("sRGB", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_sRGB(l, pp, ip, 0);
    })));
    cases.push(("sRGB-3", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_sRGB(l, pp, ip, 3);
    })));
    cases.push(("sRGB_gAMA_and_cHRM", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_sRGB_gAMA_and_cHRM(l, pp, ip, 1);
    })));
    cases.push(("iCCP-rgb", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_iCCP(
            l,
            pp,
            ip,
            iccname.as_ptr(),
            PNG_COMPRESSION_TYPE_BASE,
            prof_rgb.as_ptr(),
            prof_rgb.len() as png_uint_32,
        );
    })));
    cases.push(("iCCP-gray", &gimg, Box::new(|l, pp, ip| unsafe {
        png_set_iCCP(
            l,
            pp,
            ip,
            iccname.as_ptr(),
            PNG_COMPRESSION_TYPE_BASE,
            prof_gray.as_ptr(),
            prof_gray.len() as png_uint_32,
        );
    })));
    cases.push(("sBIT-rgb", &rimg, Box::new(|l, pp, ip| unsafe {
        let sb = png_color_8 { red: 5, green: 6, blue: 7, gray: 5, alpha: 8 };
        png_set_sBIT(l, pp, ip, &sb);
    })));
    cases.push(("sBIT-gray16", &gimg, Box::new(|l, pp, ip| unsafe {
        let sb = png_color_8 { red: 12, green: 12, blue: 12, gray: 12, alpha: 12 };
        png_set_sBIT(l, pp, ip, &sb);
    })));
    cases.push(("bKGD-pal", &pimg, Box::new(|l, pp, ip| unsafe {
        let bg = png_color_16 { index: 17, red: 0, green: 0, blue: 0, gray: 0 };
        png_set_bKGD(l, pp, ip, &bg);
    })));
    cases.push(("bKGD-rgb", &rimg, Box::new(|l, pp, ip| unsafe {
        let bg = png_color_16 { index: 0, red: 0x00ff, green: 0x0080, blue: 0x0001, gray: 0 };
        png_set_bKGD(l, pp, ip, &bg);
    })));
    cases.push(("hIST", &pimg, Box::new(|l, pp, ip| unsafe {
        png_set_hIST(l, pp, ip, hist.as_ptr());
    })));
    cases.push(("tRNS-pal", &pimg, Box::new(|l, pp, ip| unsafe {
        png_set_tRNS(l, pp, ip, trans.as_ptr(), 200, ptr::null());
    })));
    cases.push(("tRNS-gray", &gimg, Box::new(|l, pp, ip| unsafe {
        let tc = png_color_16 { index: 0, red: 0, green: 0, blue: 0, gray: 0x1234 };
        png_set_tRNS(l, pp, ip, ptr::null(), 1, &tc);
    })));
    cases.push(("oFFs", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_oFFs(l, pp, ip, -1234, 5678, 1);
    })));
    cases.push(("pHYs", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_pHYs(l, pp, ip, 2835, 2836, 1);
    })));
    cases.push(("sCAL", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_sCAL(l, pp, ip, 1, 2.5, 0.125);
    })));
    cases.push(("sCAL_fixed", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_sCAL_fixed(l, pp, ip, 2, 250000, 12500);
    })));
    cases.push(("sCAL_s", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_sCAL_s(l, pp, ip, 1, scal_w.as_ptr(), scal_h.as_ptr());
    })));
    cases.push(("sCAL_s-neg", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_sCAL_s(l, pp, ip, 1, scal_w.as_ptr(), scal_bad.as_ptr());
    })));
    cases.push(("sCAL_s-unit0", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_sCAL_s(l, pp, ip, 0, scal_w.as_ptr(), scal_h.as_ptr());
    })));
    cases.push(("tIME", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_tIME(l, pp, ip, &tm);
    })));
    cases.push(("eXIf_1", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_eXIf_1(l, pp, ip, exif.len() as png_uint_32, exif.as_ptr() as png_bytep);
    })));
    cases.push(("sPLT", &rimg, Box::new(|l, pp, ip| unsafe {
        let s = png_sPLT_t {
            name: spl_name.as_ptr() as png_charp,
            depth: 8,
            entries: spl_entries.as_ptr() as *mut png_sPLT_entry,
            nentries: 7,
        };
        png_set_sPLT(l, pp, ip, &s, 1);
    })));
    cases.push(("mDCV_fixed", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_mDCV_fixed(
            l, pp, ip, 34000, 16000, 13250, 34500, 7500, 3000, 31270, 32900, 1000_0000,
            50,
        );
    })));
    cases.push(("cLLI_fixed", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_cLLI_fixed(l, pp, ip, 1000_0000, 400_0000);
    })));
    cases.push(("cICP", &rimg, Box::new(|l, pp, ip| unsafe {
        png_set_cICP(l, pp, ip, 9, 16, 0, 1);
    })));

    for typ in 0i32..4 {
        for np in 0i32..4 {
            let purpose = &purpose;
            let units = &units;
            let pvp = &pvp;
            let name: &'static str = Box::leak(
                format!("pCAL t={typ} n={np}").into_boxed_str(),
            );
            cases.push((
                name,
                &rimg,
                Box::new(move |l, pp, ip| unsafe {
                    png_set_pCAL(
                        l,
                        pp,
                        ip,
                        purpose.as_ptr(),
                        -50,
                        50,
                        typ,
                        np,
                        units.as_ptr(),
                        pvp.as_ptr() as *mut png_charp,
                    );
                }),
            ));
        }
    }

    for (name, img, set) in cases.iter() {
        diff(&format!("36.single.{name}"), |l, pp, ip| unsafe {
            head(l, pp, ip, img);
            set(l, pp, ip);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, img.rows_ptr());
            png_write_end(l, pp, ip);
        });
    }

    // ---- everything together, palette base -------------------------------
    diff("36.all-palette", |l, pp, ip| unsafe {
        head(l, pp, ip, &pimg);
        png_set_gAMA_fixed(l, pp, ip, 45455);
        png_set_cHRM_fixed(l, pp, ip, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
        let sb = png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
        png_set_sBIT(l, pp, ip, &sb);
        let bg = png_color_16 { index: 5, red: 0, green: 0, blue: 0, gray: 0 };
        png_set_bKGD(l, pp, ip, &bg);
        png_set_hIST(l, pp, ip, hist.as_ptr());
        png_set_tRNS(l, pp, ip, trans.as_ptr(), 128, ptr::null());
        png_set_oFFs(l, pp, ip, 3, -4, 0);
        png_set_pHYs(l, pp, ip, 100, 200, 1);
        png_set_sCAL_s(l, pp, ip, 1, scal_w.as_ptr(), scal_h.as_ptr());
        png_set_pCAL(
            l,
            pp,
            ip,
            purpose.as_ptr(),
            -50,
            50,
            2,
            3,
            units.as_ptr(),
            pvp.as_ptr() as *mut png_charp,
        );
        png_set_tIME(l, pp, ip, &tm);
        png_set_eXIf_1(l, pp, ip, exif.len() as png_uint_32, exif.as_ptr() as png_bytep);
        let s = png_sPLT_t {
            name: spl_name.as_ptr() as png_charp,
            depth: 16,
            entries: spl_entries.as_ptr() as *mut png_sPLT_entry,
            nentries: 7,
        };
        png_set_sPLT(l, pp, ip, &s, 1);
        png_set_mDCV_fixed(
            l, pp, ip, 34000, 16000, 13250, 34500, 7500, 3000, 31270, 32900, 1000_0000, 50,
        );
        png_set_cLLI_fixed(l, pp, ip, 1000_0000, 400_0000);
        png_set_cICP(l, pp, ip, 9, 16, 0, 1);
        png_write_info(l, pp, ip);
        png_write_image(l, pp, pimg.rows_ptr());
        png_write_end(l, pp, ip);
    });

    // ---- everything together, RGB base, sRGB + iCCP conflict -------------
    diff("36.all-rgb", |l, pp, ip| unsafe {
        head(l, pp, ip, &rimg);
        png_set_sRGB_gAMA_and_cHRM(l, pp, ip, 0);
        png_set_iCCP(
            l,
            pp,
            ip,
            iccname.as_ptr(),
            PNG_COMPRESSION_TYPE_BASE,
            prof_rgb.as_ptr(),
            prof_rgb.len() as png_uint_32,
        );
        let sb = png_color_8 { red: 8, green: 7, blue: 6, gray: 8, alpha: 8 };
        png_set_sBIT(l, pp, ip, &sb);
        let bg = png_color_16 { index: 0, red: 0x00aa, green: 0x00bb, blue: 0x00cc, gray: 0 };
        png_set_bKGD(l, pp, ip, &bg);
        let tc = png_color_16 { index: 0, red: 0x0001, green: 0x0002, blue: 0x0003, gray: 0 };
        png_set_tRNS(l, pp, ip, ptr::null(), 1, &tc);
        png_set_oFFs(l, pp, ip, i32::MIN + 1, i32::MAX - 1, 1);
        png_set_pHYs(l, pp, ip, 1, 1, 0);
        png_set_sCAL(l, pp, ip, 2, 1.0e-5, 9.75);
        png_set_tIME(l, pp, ip, &tm);
        png_set_eXIf_1(l, pp, ip, exif.len() as png_uint_32, exif.as_ptr() as png_bytep);
        png_set_cLLI_fixed(l, pp, ip, 0, 0);
        png_set_cICP(l, pp, ip, 1, 13, 0, 1);
        png_write_info(l, pp, ip);
        png_write_image(l, pp, rimg.rows_ptr());
        png_write_end(l, pp, ip);
    });
}

// ===========================================================================
// row 37 — unknown chunks
// ===========================================================================

#[test]
fn cfg37_unknown_chunks() {
    need_libm();
    let mut rng = Rng::new(0x3700_0001);
    let img = Img::new(10, 6, 8, PNG_COLOR_TYPE_RGB, 0, 0x3700_0002);
    let pimg = Img::new(10, 6, 4, PNG_COLOR_TYPE_PALETTE, 0, 0x3700_0003);

    // safe-to-copy is the *4th* letter being lower case; the 2nd letter upper
    // case means "public".  Cover both spellings.
    let names: [&[u8; 5]; 4] = [b"nEwc\0", b"nEwC\0", b"unSc\0", b"unSC\0"];
    let payloads: Vec<Vec<u8>> = vec![vec![], rng.bytes(1), rng.bytes(37), rng.bytes(300)];
    let locations = [PNG_HAVE_IHDR, PNG_HAVE_PLTE, PNG_AFTER_IDAT];
    let keeps = [
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        PNG_HANDLE_CHUNK_NEVER,
        PNG_HANDLE_CHUNK_IF_SAFE,
        PNG_HANDLE_CHUNK_ALWAYS,
    ];

    for &loc in locations.iter() {
        for &keep in keeps.iter() {
            for nch in [0usize, 1, 3] {
                for use_list in [false, true] {
                    let mut chunks: Vec<png_unknown_chunk> = Vec::new();
                    for i in 0..nch {
                        let nm = names[i % names.len()];
                        let pl = &payloads[(i + 1) % payloads.len()];
                        chunks.push(png_unknown_chunk {
                            name: *nm,
                            data: pl.as_ptr() as *mut png_byte,
                            size: pl.len(),
                            location: loc as png_byte,
                        });
                    }
                    // the per-chunk keep list
                    let mut list: Vec<u8> = Vec::new();
                    for nm in names.iter() {
                        list.extend_from_slice(&nm[..]);
                    }
                    let chunks = &chunks;
                    let list = &list;
                    diff(
                        &format!(
                            "37.loc={loc} keep={keep} n={nch} list={}",
                            use_list as u8
                        ),
                        |l, pp, ip| unsafe {
                            head(l, pp, ip, &img);
                            if use_list {
                                png_set_keep_unknown_chunks(
                                    l,
                                    pp,
                                    keep,
                                    list.as_ptr(),
                                    names.len() as c_int,
                                );
                            } else {
                                png_set_keep_unknown_chunks(l, pp, keep, ptr::null(), 0);
                            }
                            if !chunks.is_empty() {
                                png_set_unknown_chunks(
                                    l,
                                    pp,
                                    ip,
                                    chunks.as_ptr(),
                                    chunks.len() as c_int,
                                );
                            }
                            png_write_info(l, pp, ip);
                            png_write_image(l, pp, img.rows_ptr());
                            png_write_end(l, pp, ip);
                        },
                    );
                }
            }
        }
    }

    // png_handle_as_unknown must agree for every name and keep setting
    for &keep in keeps.iter() {
        for nm in names.iter() {
            let mut list: Vec<u8> = Vec::new();
            list.extend_from_slice(&names[0][..]);
            list.extend_from_slice(&names[2][..]);
            let list = &list;
            let r = capture(|| {
                both(|l| unsafe {
                    let pp = new_writer(l);
                    png_set_keep_unknown_chunks(l, pp, keep, list.as_ptr(), 2);
                    let v = png_handle_as_unknown(l, pp, nm.as_ptr());
                    let mut p = pp;
                    png_destroy_write_struct(l, &mut p, ptr::null_mut());
                    v
                })
            });
            let (a, b) = r.out.expect("37: png_handle_as_unknown must not error");
            assert_eq!(
                a, b,
                "37: png_handle_as_unknown({}) keep={keep} differs",
                String::from_utf8_lossy(&nm[..4])
            );
        }
    }

    // png_set_unknown_chunk_location
    for &loc in locations.iter() {
        let pl = &payloads[2];
        let ch = [png_unknown_chunk {
            name: *names[0],
            data: pl.as_ptr() as *mut png_byte,
            size: pl.len(),
            location: PNG_HAVE_IHDR as png_byte,
        }];
        let ch = &ch;
        diff(&format!("37.set_location {loc}"), |l, pp, ip| unsafe {
            head(l, pp, ip, &pimg);
            png_set_unknown_chunks(l, pp, ip, ch.as_ptr(), 1);
            png_set_unknown_chunk_location(l, pp, ip, 0, loc);
            png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, ptr::null(), 0);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, pimg.rows_ptr());
            png_write_end(l, pp, ip);
        });
        // out-of-range chunk index
        diff(&format!("37.set_location-oob {loc}"), |l, pp, ip| unsafe {
            head(l, pp, ip, &pimg);
            png_set_unknown_chunks(l, pp, ip, ch.as_ptr(), 1);
            png_set_unknown_chunk_location(l, pp, ip, 5, loc);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, pimg.rows_ptr());
            png_write_end(l, pp, ip);
        });
    }

    // all three locations at once on a palette image
    let p0 = rng.bytes(9);
    let p1 = rng.bytes(4);
    let p2 = rng.bytes(64);
    let three = [
        png_unknown_chunk {
            name: *b"aBcd\0",
            data: p0.as_ptr() as *mut png_byte,
            size: p0.len(),
            location: PNG_HAVE_IHDR as png_byte,
        },
        png_unknown_chunk {
            name: *b"eFgh\0",
            data: p1.as_ptr() as *mut png_byte,
            size: p1.len(),
            location: PNG_HAVE_PLTE as png_byte,
        },
        png_unknown_chunk {
            name: *b"iJkl\0",
            data: p2.as_ptr() as *mut png_byte,
            size: p2.len(),
            location: PNG_AFTER_IDAT as png_byte,
        },
    ];
    for &keep in keeps.iter() {
        let three = &three;
        diff(&format!("37.three-locations keep={keep}"), |l, pp, ip| unsafe {
            head(l, pp, ip, &pimg);
            png_set_keep_unknown_chunks(l, pp, keep, ptr::null(), 0);
            png_set_unknown_chunks(l, pp, ip, three.as_ptr(), 3);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, pimg.rows_ptr());
            png_write_end(l, pp, ip);
        });
    }

    // negative / oversized `num` arguments to png_set_keep_unknown_chunks
    for num in [-1i32, 0, 1, 4] {
        let mut list: Vec<u8> = Vec::new();
        for nm in names.iter() {
            list.extend_from_slice(&nm[..]);
        }
        let list = &list;
        let three = &three;
        diff(&format!("37.keep-num={num}"), |l, pp, ip| unsafe {
            head(l, pp, ip, &pimg);
            png_set_keep_unknown_chunks(
                l,
                pp,
                PNG_HANDLE_CHUNK_ALWAYS,
                if num <= 0 { ptr::null() } else { list.as_ptr() },
                num,
            );
            png_set_unknown_chunks(l, pp, ip, three.as_ptr(), 3);
            png_write_info(l, pp, ip);
            png_write_image(l, pp, pimg.rows_ptr());
            png_write_end(l, pp, ip);
        });
    }
}
