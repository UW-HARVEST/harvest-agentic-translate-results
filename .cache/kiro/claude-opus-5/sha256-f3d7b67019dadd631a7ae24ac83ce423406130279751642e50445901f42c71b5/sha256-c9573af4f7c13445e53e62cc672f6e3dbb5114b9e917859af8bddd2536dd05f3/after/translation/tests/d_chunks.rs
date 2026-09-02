//! Phase B, group WC: every ancillary-chunk setter on the write side, plus the
//! matching `png_get_*` read-back, driven through both `.so`s.
mod common;
use common::*;
use std::ffi::{c_int, c_void, CString};
use std::ptr;

const SEED: u64 = 0xC0DE_0BAD_1234_5678;

const SHAPES: &[(c_int, c_int)] = &[(0, 8), (2, 8), (3, 8), (4, 16), (6, 16)];

fn base_write(
    l: &Lib,
    ct: c_int,
    bd: c_int,
    setup: &mut dyn FnMut(&Lib, *mut c_void, *mut c_void),
) -> Report {
    let (w, h) = (8u32, 4u32);
    let pal = if ct == PNG_COLOR_TYPE_PALETTE {
        make_palette(256, SEED ^ 1)
    } else {
        vec![]
    };
    write_full(
        l,
        w,
        h,
        ct,
        bd,
        PNG_INTERLACE_NONE,
        PNG_FILTER_TYPE_BASE,
        &pal,
        rowbytes(w, bd, ct),
        SEED ^ ((ct as u64) << 8) ^ bd as u64,
        setup,
    )
}

/// Apply `setup` for every shape in SHAPES and diff.
#[allow(dead_code)]
fn for_shapes(
    label: &str,
    c: &Lib,
    r: &Lib,
    setup: &(dyn Fn(&Lib, *mut c_void, *mut c_void, c_int, c_int) + Sync),
) {
    for &(ct, bd) in SHAPES {
        let mut run = |l: &Lib| -> Report {
            base_write(l, ct, bd, &mut |l, png, info| setup(l, png, info, ct, bd))
        };
        diff(&format!("{label} ct={ct} bd={bd}"), c, r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// WC1 gAMA
// ---------------------------------------------------------------------------
#[test]
fn wc1_gama() {
    let (c, r) = libs();
    let fixed: &[i32] = &[
        0,
        1,
        100,
        45455,
        50000,
        100000,
        220000,
        1_000_000,
        2_147_483_647,
        -1,
        -100000,
        21474,
        2147483,
    ];
    for &g in fixed {
        for &(ct, bd) in &[(2i32, 8i32)] {
            let mut run = |l: &Lib| -> Report {
                base_write(l, ct, bd, &mut |l, png, info| unsafe {
                    (l.api.png_set_gAMA_fixed)(png, info, g);
                    let mut out: i32 = 0;
                    log(format!(
                        "get_gAMA_fixed={} val={out}",
                        (l.api.png_get_gAMA_fixed)(png, info, &mut out)
                    ));
                    let mut d: f64 = 0.0;
                    log(format!(
                        "get_gAMA={} val={d}",
                        (l.api.png_get_gAMA)(png, info, &mut d)
                    ));
                })
            };
            diff(&format!("WC1 gAMA_fixed({g})"), &c, &r, &mut run);
        }
    }
    let mut rng = Rng::new(SEED ^ 0x1000);
    for i in 0..64 {
        let d = (rng.u32() % 400_000) as f64 / 100_000.0;
        let mut run = |l: &Lib| -> Report {
            base_write(l, 2, 8, &mut |l, png, info| unsafe {
                (l.api.png_set_gAMA)(png, info, d);
                let mut out: i32 = 0;
                log(format!(
                    "get={} {out}",
                    (l.api.png_get_gAMA_fixed)(png, info, &mut out)
                ));
            })
        };
        diff(&format!("WC1 gAMA({d}) #{i}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// WC2/WC3 cHRM
// ---------------------------------------------------------------------------
#[test]
fn wc2_wc3_chrm() {
    let (c, r) = libs();
    let srgb = [31270i32, 32900, 64000, 33000, 30000, 60000, 15000, 6000];
    let mut cases: Vec<[i32; 8]> = vec![srgb, [0; 8]];
    let mut rng = Rng::new(SEED ^ 0x2000);
    for _ in 0..48 {
        let mut a = [0i32; 8];
        for v in a.iter_mut() {
            *v = (rng.u32() % 200_001) as i32 - 100_000;
        }
        cases.push(a);
    }
    for (i, a) in cases.iter().enumerate() {
        let mut run = |l: &Lib| -> Report {
            base_write(l, 2, 8, &mut |l, png, info| unsafe {
                (l.api.png_set_cHRM_fixed)(png, info, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]);
                let mut o = [0i32; 8];
                log(format!(
                    "get_cHRM_fixed={} {o:?}",
                    (l.api.png_get_cHRM_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4],
                        &mut o[5], &mut o[6], &mut o[7]
                    )
                ));
                let mut d = [0f64; 8];
                log(format!(
                    "get_cHRM={} {d:?}",
                    (l.api.png_get_cHRM)(
                        png, info, &mut d[0], &mut d[1], &mut d[2], &mut d[3], &mut d[4],
                        &mut d[5], &mut d[6], &mut d[7]
                    )
                ));
                let mut x = [0i32; 9];
                log(format!(
                    "get_cHRM_XYZ_fixed={} {x:?}",
                    (l.api.png_get_cHRM_XYZ_fixed)(
                        png, info, &mut x[0], &mut x[1], &mut x[2], &mut x[3], &mut x[4],
                        &mut x[5], &mut x[6], &mut x[7], &mut x[8]
                    )
                ));
            })
        };
        diff(&format!("WC2 cHRM_fixed #{i} {a:?}"), &c, &r, &mut run);
    }
    // cHRM_XYZ
    let mut rng = Rng::new(SEED ^ 0x3000);
    for i in 0..48 {
        let mut a = [0i32; 9];
        for v in a.iter_mut() {
            *v = (rng.u32() % 200_001) as i32 - 50_000;
        }
        let mut run = |l: &Lib| -> Report {
            base_write(l, 2, 8, &mut |l, png, info| unsafe {
                (l.api.png_set_cHRM_XYZ_fixed)(
                    png, info, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
                );
                let mut o = [0i32; 8];
                log(format!(
                    "get_cHRM_fixed={} {o:?}",
                    (l.api.png_get_cHRM_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4],
                        &mut o[5], &mut o[6], &mut o[7]
                    )
                ));
            })
        };
        diff(&format!("WC3 cHRM_XYZ_fixed #{i}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// WC4/WC5 sRGB
// ---------------------------------------------------------------------------
#[test]
fn wc4_wc5_srgb() {
    let (c, r) = libs();
    for intent in [-1i32, 0, 1, 2, 3, 4, 5, 100] {
        for &(ct, bd) in SHAPES {
            let mut run = |l: &Lib| -> Report {
                base_write(l, ct, bd, &mut |l, png, info| unsafe {
                    (l.api.png_set_sRGB)(png, info, intent);
                    let mut o: c_int = -9;
                    log(format!(
                        "get_sRGB={} {o}",
                        (l.api.png_get_sRGB)(png, info, &mut o)
                    ));
                })
            };
            diff(&format!("WC4 sRGB intent={intent} ct={ct} bd={bd}"), &c, &r, &mut run);
            let mut run = |l: &Lib| -> Report {
                base_write(l, ct, bd, &mut |l, png, info| unsafe {
                    (l.api.png_set_sRGB_gAMA_and_cHRM)(png, info, intent);
                    let mut g: i32 = 0;
                    log(format!(
                        "gAMA={} {g}",
                        (l.api.png_get_gAMA_fixed)(png, info, &mut g)
                    ));
                })
            };
            diff(
                &format!("WC5 sRGB_gAMA_and_cHRM intent={intent} ct={ct} bd={bd}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// WC6 iCCP
// ---------------------------------------------------------------------------
fn make_icc(len: usize) -> Vec<u8> {
    let mut p = vec![0u8; len.max(132)];
    let n = p.len();
    p[0..4].copy_from_slice(&(n as u32).to_be_bytes());
    p[4..8].copy_from_slice(b"ADBE");
    p[8..12].copy_from_slice(&0x0200_0000u32.to_be_bytes());
    p[12..16].copy_from_slice(b"mntr");
    p[16..20].copy_from_slice(b"RGB ");
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    p[64..68].copy_from_slice(&0u32.to_be_bytes());
    p[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
    p[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    p[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
    p[128..132].copy_from_slice(&0u32.to_be_bytes());
    p
}

#[test]
fn wc6_iccp() {
    let (c, r) = libs();
    for len in [132usize, 133, 144, 1000, 4096] {
        for namelen in [1usize, 10, 79] {
            let prof = make_icc(len);
            let name = CString::new("x".repeat(namelen)).unwrap();
            for &(ct, bd) in &[(2i32, 8i32), (0, 8)] {
                let mut run = |l: &Lib| -> Report {
                    base_write(l, ct, bd, &mut |l, png, info| unsafe {
                        (l.api.png_set_iCCP)(
                            png,
                            info,
                            name.as_ptr(),
                            0,
                            prof.as_ptr(),
                            prof.len() as u32,
                        );
                        let mut nm: *mut i8 = ptr::null_mut();
                        let mut comp: c_int = -1;
                        let mut pp: *mut u8 = ptr::null_mut();
                        let mut plen: u32 = 0;
                        let got = (l.api.png_get_iCCP)(
                            png, info, &mut nm, &mut comp, &mut pp, &mut plen,
                        );
                        log(format!("get_iCCP={got} comp={comp} plen={plen}"));
                        if !nm.is_null() {
                            log(format!(
                                "name={:?}",
                                std::ffi::CStr::from_ptr(nm).to_string_lossy()
                            ));
                        }
                    })
                };
                diff(
                    &format!("WC6 iCCP len={len} namelen={namelen} ct={ct} bd={bd}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WC7 sBIT
// ---------------------------------------------------------------------------
#[test]
fn wc7_sbit() {
    let (c, r) = libs();
    for &(ct, bd) in &[
        (0i32, 1i32),
        (0, 2),
        (0, 4),
        (0, 8),
        (0, 16),
        (2, 8),
        (2, 16),
        (3, 8),
        (4, 8),
        (4, 16),
        (6, 8),
        (6, 16),
    ] {
        let maxb = if ct == PNG_COLOR_TYPE_PALETTE { 8 } else { bd as u8 };
        for sb in [1u8, 2, 4, maxb] {
            let sig = PngColor8 {
                red: sb,
                green: sb,
                blue: sb,
                gray: sb,
                alpha: sb,
            };
            let mut run = |l: &Lib| -> Report {
                base_write(l, ct, bd, &mut |l, png, info| unsafe {
                    (l.api.png_set_sBIT)(png, info, &sig);
                    let mut p: *mut PngColor8 = ptr::null_mut();
                    let got = (l.api.png_get_sBIT)(png, info, &mut p);
                    log(format!(
                        "get_sBIT={got} {:?}",
                        if p.is_null() { None } else { Some(*p) }
                    ));
                })
            };
            diff(&format!("WC7 sBIT ct={ct} bd={bd} sb={sb}"), &c, &r, &mut run);
        }
    }
}

// ---------------------------------------------------------------------------
// WC8 bKGD + WC9 hIST + WC10 tRNS
// ---------------------------------------------------------------------------
#[test]
fn wc8_bkgd() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0x8000);
    for &(ct, bd) in &[
        (0i32, 1i32),
        (0, 8),
        (0, 16),
        (2, 8),
        (2, 16),
        (3, 8),
        (4, 8),
        (6, 16),
    ] {
        for i in 0..6 {
            let bg = PngColor16 {
                index: rng.u8(),
                red: (rng.u32() & 0xffff) as u16,
                green: (rng.u32() & 0xffff) as u16,
                blue: (rng.u32() & 0xffff) as u16,
                gray: (rng.u32() & 0xffff) as u16,
            };
            let mut run = |l: &Lib| -> Report {
                base_write(l, ct, bd, &mut |l, png, info| unsafe {
                    (l.api.png_set_bKGD)(png, info, &bg);
                    let mut p: *mut PngColor16 = ptr::null_mut();
                    let got = (l.api.png_get_bKGD)(png, info, &mut p);
                    log(format!(
                        "get_bKGD={got} {:?}",
                        if p.is_null() { None } else { Some(*p) }
                    ));
                })
            };
            diff(&format!("WC8 bKGD ct={ct} bd={bd} #{i}"), &c, &r, &mut run);
        }
    }
}

#[test]
fn wc9_hist() {
    let (c, r) = libs();
    for n in [2usize, 16, 255, 256] {
        let mut rng = Rng::new(SEED ^ 0x9000 ^ n as u64);
        let hist: Vec<u16> = (0..n).map(|_| (rng.u32() & 0xffff) as u16).collect();
        let pal = make_palette(n, SEED ^ 0x9001);
        let mut run = |l: &Lib| -> Report {
            let (w, h) = (8u32, 4u32);
            write_full(
                l,
                w,
                h,
                PNG_COLOR_TYPE_PALETTE,
                8,
                PNG_INTERLACE_NONE,
                PNG_FILTER_TYPE_BASE,
                &pal,
                w as usize,
                SEED ^ 0x9002,
                &mut |l, png, info| unsafe {
                    (l.api.png_set_hIST)(png, info, hist.as_ptr());
                    let mut p: *mut u16 = ptr::null_mut();
                    let got = (l.api.png_get_hIST)(png, info, &mut p);
                    log(format!("get_hIST={got} null={}", p.is_null()));
                    if !p.is_null() {
                        log(format!(
                            "hist={:?}",
                            std::slice::from_raw_parts(p, n)
                        ));
                    }
                },
            )
        };
        diff(&format!("WC9 hIST n={n}"), &c, &r, &mut run);
    }
}

#[test]
fn wc10_trns() {
    let (c, r) = libs();
    // palette tRNS
    for n in [1usize, 2, 16, 255, 256] {
        let mut rng = Rng::new(SEED ^ 0xa000 ^ n as u64);
        let alpha: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
        let pal = make_palette(256, SEED ^ 0xa001);
        let mut run = |l: &Lib| -> Report {
            let (w, h) = (8u32, 4u32);
            write_full(
                l,
                w,
                h,
                PNG_COLOR_TYPE_PALETTE,
                8,
                PNG_INTERLACE_NONE,
                PNG_FILTER_TYPE_BASE,
                &pal,
                w as usize,
                SEED ^ 0xa002,
                &mut |l, png, info| unsafe {
                    (l.api.png_set_tRNS)(png, info, alpha.as_ptr(), n as c_int, ptr::null());
                    let mut ta: *mut u8 = ptr::null_mut();
                    let mut nt: c_int = 0;
                    let mut tc: *mut PngColor16 = ptr::null_mut();
                    let got = (l.api.png_get_tRNS)(png, info, &mut ta, &mut nt, &mut tc);
                    log(format!("get_tRNS={got} num={nt}"));
                    if !ta.is_null() && nt > 0 {
                        log(format!(
                            "alpha={:?}",
                            std::slice::from_raw_parts(ta, nt as usize)
                        ));
                    }
                },
            )
        };
        diff(&format!("WC10 tRNS palette n={n}"), &c, &r, &mut run);
    }
    // gray / rgb tRNS
    let mut rng = Rng::new(SEED ^ 0xa100);
    for &(ct, bd) in &[(0i32, 1i32), (0, 8), (0, 16), (2, 8), (2, 16)] {
        for i in 0..4 {
            let tc = PngColor16 {
                index: 0,
                red: (rng.u32() & 0xffff) as u16,
                green: (rng.u32() & 0xffff) as u16,
                blue: (rng.u32() & 0xffff) as u16,
                gray: (rng.u32() & 0xffff) as u16,
            };
            let mut run = |l: &Lib| -> Report {
                base_write(l, ct, bd, &mut |l, png, info| unsafe {
                    (l.api.png_set_tRNS)(png, info, ptr::null(), 0, &tc);
                    let mut ta: *mut u8 = ptr::null_mut();
                    let mut nt: c_int = 0;
                    let mut out: *mut PngColor16 = ptr::null_mut();
                    let got = (l.api.png_get_tRNS)(png, info, &mut ta, &mut nt, &mut out);
                    log(format!(
                        "get_tRNS={got} num={nt} color={:?}",
                        if out.is_null() { None } else { Some(*out) }
                    ));
                })
            };
            diff(&format!("WC10 tRNS ct={ct} bd={bd} #{i}"), &c, &r, &mut run);
        }
    }
}

// ---------------------------------------------------------------------------
// WC11 pHYs / WC12 oFFs
// ---------------------------------------------------------------------------
#[test]
fn wc11_wc12_phys_offs() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0xb000);
    for unit in [-1i32, 0, 1, 2, 99] {
        for i in 0..6 {
            let (x, y) = (rng.u32(), rng.u32());
            let mut run = |l: &Lib| -> Report {
                base_write(l, 2, 8, &mut |l, png, info| unsafe {
                    (l.api.png_set_pHYs)(png, info, x, y, unit);
                    let mut ox = 0u32;
                    let mut oy = 0u32;
                    let mut ou: c_int = -9;
                    log(format!(
                        "get_pHYs={} {ox} {oy} {ou}",
                        (l.api.png_get_pHYs)(png, info, &mut ox, &mut oy, &mut ou)
                    ));
                    log(format!(
                        "ppm={} xppm={} yppm={} ratio={} ratio_fixed={}",
                        (l.api.png_get_pixels_per_meter)(png, info),
                        (l.api.png_get_x_pixels_per_meter)(png, info),
                        (l.api.png_get_y_pixels_per_meter)(png, info),
                        (l.api.png_get_pixel_aspect_ratio)(png, info),
                        (l.api.png_get_pixel_aspect_ratio_fixed)(png, info)
                    ));
                    log(format!(
                        "ppi={} xppi={} yppi={}",
                        (l.api.png_get_pixels_per_inch)(png, info),
                        (l.api.png_get_x_pixels_per_inch)(png, info),
                        (l.api.png_get_y_pixels_per_inch)(png, info)
                    ));
                    let mut dx = 0u32;
                    let mut dy = 0u32;
                    let mut du: c_int = 0;
                    log(format!(
                        "pHYs_dpi={} {dx} {dy} {du}",
                        (l.api.png_get_pHYs_dpi)(png, info, &mut dx, &mut dy, &mut du)
                    ));
                })
            };
            diff(&format!("WC11 pHYs unit={unit} #{i}"), &c, &r, &mut run);
        }
    }
    for unit in [-1i32, 0, 1, 2, 99] {
        for &(x, y) in &[
            (0i32, 0i32),
            (1, -1),
            (i32::MIN, i32::MAX),
            (123456, -987654),
            (-2147483647, 2147483647),
        ] {
            let mut run = |l: &Lib| -> Report {
                base_write(l, 2, 8, &mut |l, png, info| unsafe {
                    (l.api.png_set_oFFs)(png, info, x, y, unit);
                    let mut ox = 0i32;
                    let mut oy = 0i32;
                    let mut ou: c_int = -9;
                    log(format!(
                        "get_oFFs={} {ox} {oy} {ou}",
                        (l.api.png_get_oFFs)(png, info, &mut ox, &mut oy, &mut ou)
                    ));
                    log(format!(
                        "xpix={} ypix={} xmic={} ymic={} xin={} yin={} xinf={} yinf={}",
                        (l.api.png_get_x_offset_pixels)(png, info),
                        (l.api.png_get_y_offset_pixels)(png, info),
                        (l.api.png_get_x_offset_microns)(png, info),
                        (l.api.png_get_y_offset_microns)(png, info),
                        (l.api.png_get_x_offset_inches)(png, info),
                        (l.api.png_get_y_offset_inches)(png, info),
                        (l.api.png_get_x_offset_inches_fixed)(png, info),
                        (l.api.png_get_y_offset_inches_fixed)(png, info)
                    ));
                })
            };
            diff(&format!("WC12 oFFs unit={unit} x={x} y={y}"), &c, &r, &mut run);
        }
    }
}

// ---------------------------------------------------------------------------
// WC13 pCAL
// ---------------------------------------------------------------------------
#[test]
fn wc13_pcal() {
    let (c, r) = libs();
    for eqtype in [-1i32, 0, 1, 2, 3, 4, 99] {
        for nparams in [0i32, 1, 2, 3] {
            for &(x0, x1) in &[(0i32, 255i32), (-100, 100), (i32::MIN, i32::MAX)] {
                let purpose = CString::new("purpose").unwrap();
                let units = CString::new("units").unwrap();
                let params: Vec<CString> = (0..nparams)
                    .map(|i| CString::new(format!("{}.{}", i + 1, i)).unwrap())
                    .collect();
                let mut pptrs: Vec<*mut i8> =
                    params.iter().map(|p| p.as_ptr() as *mut i8).collect();
                let mut run = |l: &Lib| -> Report {
                    base_write(l, 2, 8, &mut |l, png, info| unsafe {
                        (l.api.png_set_pCAL)(
                            png,
                            info,
                            purpose.as_ptr(),
                            x0,
                            x1,
                            eqtype,
                            nparams,
                            units.as_ptr(),
                            if pptrs.is_empty() {
                                ptr::null_mut()
                            } else {
                                pptrs.as_mut_ptr()
                            },
                        );
                        let mut pp: *mut i8 = ptr::null_mut();
                        let mut ox0 = 0i32;
                        let mut ox1 = 0i32;
                        let mut ot: c_int = -9;
                        let mut on: c_int = -9;
                        let mut ou: *mut i8 = ptr::null_mut();
                        let mut opar: *mut *mut i8 = ptr::null_mut();
                        let got = (l.api.png_get_pCAL)(
                            png, info, &mut pp, &mut ox0, &mut ox1, &mut ot, &mut on, &mut ou,
                            &mut opar,
                        );
                        log(format!("get_pCAL={got} x0={ox0} x1={ox1} t={ot} n={on}"));
                        if !pp.is_null() {
                            log(format!(
                                "purpose={:?}",
                                std::ffi::CStr::from_ptr(pp).to_string_lossy()
                            ));
                        }
                        if !ou.is_null() {
                            log(format!(
                                "units={:?}",
                                std::ffi::CStr::from_ptr(ou).to_string_lossy()
                            ));
                        }
                        if !opar.is_null() && on > 0 {
                            for i in 0..on as usize {
                                let q = *opar.add(i);
                                if !q.is_null() {
                                    log(format!(
                                        "param{i}={:?}",
                                        std::ffi::CStr::from_ptr(q).to_string_lossy()
                                    ));
                                }
                            }
                        }
                    })
                };
                diff(
                    &format!("WC13 pCAL eq={eqtype} n={nparams} x0={x0} x1={x1}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WC14 sCAL
// ---------------------------------------------------------------------------
#[test]
fn wc14_scal() {
    let (c, r) = libs();
    for unit in [-1i32, 0, 1, 2, 3, 99] {
        // floating point
        for &(w, h) in &[(1.0f64, 1.0f64), (0.5, 2.25), (1e-5, 1e5), (0.0, 1.0), (-1.0, 1.0)] {
            let mut run = |l: &Lib| -> Report {
                base_write(l, 2, 8, &mut |l, png, info| unsafe {
                    (l.api.png_set_sCAL)(png, info, unit, w, h);
                    let mut ou: c_int = -9;
                    let mut ow = 0f64;
                    let mut oh = 0f64;
                    log(format!(
                        "get_sCAL={} {ou} {ow} {oh}",
                        (l.api.png_get_sCAL)(png, info, &mut ou, &mut ow, &mut oh)
                    ));
                    let mut fu: c_int = -9;
                    let mut fw = 0i32;
                    let mut fh = 0i32;
                    log(format!(
                        "get_sCAL_fixed={} {fu} {fw} {fh}",
                        (l.api.png_get_sCAL_fixed)(png, info, &mut fu, &mut fw, &mut fh)
                    ));
                    let mut su: c_int = -9;
                    let mut sw: *mut i8 = ptr::null_mut();
                    let mut sh: *mut i8 = ptr::null_mut();
                    let got = (l.api.png_get_sCAL_s)(png, info, &mut su, &mut sw, &mut sh);
                    log(format!("get_sCAL_s={got} unit={su}"));
                    if !sw.is_null() {
                        log(format!(
                            "sw={:?} sh={:?}",
                            std::ffi::CStr::from_ptr(sw).to_string_lossy(),
                            std::ffi::CStr::from_ptr(sh).to_string_lossy()
                        ));
                    }
                })
            };
            diff(&format!("WC14 sCAL unit={unit} {w}x{h}"), &c, &r, &mut run);
        }
        // fixed point
        for &(w, h) in &[(100000i32, 100000i32), (1, 1), (0, 1), (-1, 1), (i32::MAX, 1)] {
            let mut run = |l: &Lib| -> Report {
                base_write(l, 2, 8, &mut |l, png, info| unsafe {
                    (l.api.png_set_sCAL_fixed)(png, info, unit, w, h);
                    let mut su: c_int = -9;
                    let mut sw: *mut i8 = ptr::null_mut();
                    let mut sh: *mut i8 = ptr::null_mut();
                    let got = (l.api.png_get_sCAL_s)(png, info, &mut su, &mut sw, &mut sh);
                    log(format!("get_sCAL_s={got} unit={su}"));
                })
            };
            diff(
                &format!("WC14 sCAL_fixed unit={unit} {w}x{h}"),
                &c,
                &r,
                &mut run,
            );
        }
        // string form
        for &(ws, hs) in &[
            ("1", "1"),
            ("0.5", "2.25"),
            ("1e-5", "1e5"),
            ("0", "1"),
            ("-1", "1"),
            ("", "1"),
            ("abc", "1"),
            ("1.", ".5"),
        ] {
            let cw = CString::new(ws).unwrap();
            let ch = CString::new(hs).unwrap();
            let mut run = |l: &Lib| -> Report {
                base_write(l, 2, 8, &mut |l, png, info| unsafe {
                    (l.api.png_set_sCAL_s)(png, info, unit, cw.as_ptr(), ch.as_ptr());
                    let mut su: c_int = -9;
                    let mut sw: *mut i8 = ptr::null_mut();
                    let mut sh: *mut i8 = ptr::null_mut();
                    let got = (l.api.png_get_sCAL_s)(png, info, &mut su, &mut sw, &mut sh);
                    log(format!("get_sCAL_s={got} unit={su}"));
                    if !sw.is_null() {
                        log(format!(
                            "sw={:?} sh={:?}",
                            std::ffi::CStr::from_ptr(sw).to_string_lossy(),
                            std::ffi::CStr::from_ptr(sh).to_string_lossy()
                        ));
                    }
                })
            };
            diff(
                &format!("WC14 sCAL_s unit={unit} {ws:?}x{hs:?}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// WC15 tIME
// ---------------------------------------------------------------------------
#[test]
fn wc15_time() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0xf000);
    let mut cases = vec![
        PngTime { year: 1995, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        PngTime { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0 },
        PngTime { year: 65535, month: 13, day: 32, hour: 24, minute: 60, second: 61 },
    ];
    for _ in 0..16 {
        cases.push(PngTime {
            year: (rng.u32() % 3000) as u16,
            month: (rng.u32() % 14) as u8,
            day: (rng.u32() % 33) as u8,
            hour: (rng.u32() % 25) as u8,
            minute: (rng.u32() % 61) as u8,
            second: (rng.u32() % 62) as u8,
        });
    }
    for (i, t) in cases.iter().enumerate() {
        let mut run = |l: &Lib| -> Report {
            base_write(l, 2, 8, &mut |l, png, info| unsafe {
                (l.api.png_set_tIME)(png, info, t);
                let mut p: *mut PngTime = ptr::null_mut();
                let got = (l.api.png_get_tIME)(png, info, &mut p);
                log(format!(
                    "get_tIME={got} {:?}",
                    if p.is_null() { None } else { Some(*p) }
                ));
            })
        };
        diff(&format!("WC15 tIME #{i} {t:?}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// WC16 sPLT
// ---------------------------------------------------------------------------
#[test]
fn wc16_splt() {
    let (c, r) = libs();
    for npal in [1usize, 2, 3] {
        for depth in [8u8, 16] {
            for nent in [1i32, 2, 64] {
                let names: Vec<CString> = (0..npal)
                    .map(|i| CString::new(format!("splt{i}")).unwrap())
                    .collect();
                let mut rng = Rng::new(SEED ^ 0x10000 ^ nent as u64);
                let entries: Vec<Vec<PngSpltEntry>> = (0..npal)
                    .map(|_| {
                        (0..nent)
                            .map(|_| PngSpltEntry {
                                red: (rng.u32() & 0xffff) as u16,
                                green: (rng.u32() & 0xffff) as u16,
                                blue: (rng.u32() & 0xffff) as u16,
                                alpha: (rng.u32() & 0xffff) as u16,
                                frequency: (rng.u32() & 0xffff) as u16,
                            })
                            .collect()
                    })
                    .collect();
                let splts: Vec<PngSpltT> = (0..npal)
                    .map(|i| PngSpltT {
                        name: names[i].as_ptr() as *mut i8,
                        depth,
                        entries: entries[i].as_ptr() as *mut PngSpltEntry,
                        nentries: nent,
                    })
                    .collect();
                let mut run = |l: &Lib| -> Report {
                    base_write(l, 2, 8, &mut |l, png, info| unsafe {
                        (l.api.png_set_sPLT)(
                            png,
                            info,
                            splts.as_ptr() as *const c_void,
                            npal as c_int,
                        );
                        let mut p: *mut c_void = ptr::null_mut();
                        let got = (l.api.png_get_sPLT)(png, info, &mut p);
                        log(format!("get_sPLT={got}"));
                        if !p.is_null() && got > 0 {
                            let s = std::slice::from_raw_parts(p as *const PngSpltT, got as usize);
                            for e in s {
                                log(format!("splt depth={} n={}", e.depth, e.nentries));
                            }
                        }
                    })
                };
                diff(
                    &format!("WC16 sPLT npal={npal} depth={depth} nent={nent}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WC17..WC20 text chunks
// ---------------------------------------------------------------------------
#[test]
fn wc17_wc20_text() {
    let (c, r) = libs();
    let long: String = "Compressible text, repeated many times over. ".repeat(30);
    let cases: Vec<(&str, c_int, String, Option<&str>, Option<&str>)> = vec![
        ("tEXt short", -1, "hello".into(), None, None),
        ("tEXt empty", -1, "".into(), None, None),
        ("tEXt long", -1, long.clone(), None, None),
        ("zTXt short", 0, "hello".into(), None, None),
        ("zTXt long", 0, long.clone(), None, None),
        ("iTXt plain", 1, "hello".into(), Some("en"), Some("greeting")),
        ("iTXt no lang", 1, "hello".into(), None, None),
        ("iTXt comp", 2, long.clone(), Some("en-GB"), Some("k")),
        ("iTXt comp nolang", 2, long.clone(), None, None),
    ];
    for (i, (name, comp, text, lang, langkey)) in cases.iter().enumerate() {
        for keylen in [1usize, 5, 79] {
            let key = CString::new("k".repeat(keylen)).unwrap();
            let txt = CString::new(text.as_str()).unwrap();
            let lg = lang.map(|s| CString::new(s).unwrap());
            let lk = langkey.map(|s| CString::new(s).unwrap());
            let mut run = |l: &Lib| -> Report {
                base_write(l, 2, 8, &mut |l, png, info| unsafe {
                    let t = PngText {
                        compression: *comp,
                        key: key.as_ptr() as *mut i8,
                        text: txt.as_ptr() as *mut i8,
                        text_length: text.len(),
                        itxt_length: text.len(),
                        lang: lg.as_ref().map_or(ptr::null_mut(), |s| s.as_ptr() as *mut i8),
                        lang_key: lk.as_ref().map_or(ptr::null_mut(), |s| s.as_ptr() as *mut i8),
                    };
                    (l.api.png_set_text)(png, info, &t, 1);
                    let mut tp: *mut PngText = ptr::null_mut();
                    let mut n: c_int = 0;
                    let got = (l.api.png_get_text)(png, info, &mut tp, &mut n);
                    log(format!("get_text={got} n={n}"));
                    if !tp.is_null() && n > 0 {
                        for j in 0..n as usize {
                            let e = &*tp.add(j);
                            log(format!(
                                "text[{j}] comp={} key={:?} len={} itxt={}",
                                e.compression,
                                if e.key.is_null() {
                                    "<null>".to_string()
                                } else {
                                    std::ffi::CStr::from_ptr(e.key).to_string_lossy().into_owned()
                                },
                                e.text_length,
                                e.itxt_length
                            ));
                        }
                    }
                })
            };
            diff(&format!("WC17-20 {name} #{i} keylen={keylen}"), &c, &r, &mut run);
        }
    }
    // multiple text items at once
    for count in [2i32, 3, 4] {
        let keys: Vec<CString> = (0..count)
            .map(|i| CString::new(format!("Key{i}")).unwrap())
            .collect();
        let txts: Vec<CString> = (0..count)
            .map(|i| CString::new(format!("Value number {i} ...")).unwrap())
            .collect();
        let mut run = |l: &Lib| -> Report {
            base_write(l, 2, 8, &mut |l, png, info| unsafe {
                let items: Vec<PngText> = (0..count as usize)
                    .map(|i| PngText {
                        compression: [-1, 0, 1, 2][i % 4],
                        key: keys[i].as_ptr() as *mut i8,
                        text: txts[i].as_ptr() as *mut i8,
                        text_length: txts[i].as_bytes().len(),
                        itxt_length: txts[i].as_bytes().len(),
                        ..Default::default()
                    })
                    .collect();
                (l.api.png_set_text)(png, info, items.as_ptr(), count);
                let mut tp: *mut PngText = ptr::null_mut();
                let mut n: c_int = 0;
                log(format!(
                    "get_text={} n={n}",
                    (l.api.png_get_text)(png, info, &mut tp, &mut n)
                ));
            })
        };
        diff(&format!("WC17-20 multi text count={count}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// WC21 eXIf
// ---------------------------------------------------------------------------
#[test]
fn wc21_exif() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0x21000);
    let mut cases: Vec<Vec<u8>> = vec![
        b"II*\0".to_vec(),
        b"MM\0*".to_vec(),
        b"XX\0\0".to_vec(),
        vec![],
        vec![0u8],
    ];
    for n in [4usize, 8, 20, 64] {
        let mut v = b"II*\0".to_vec();
        v.extend(rng.bytes(n));
        cases.push(v);
        let mut v = b"MM\0*".to_vec();
        v.extend(rng.bytes(n));
        cases.push(v);
    }
    for (i, e) in cases.iter().enumerate() {
        let mut run = |l: &Lib| -> Report {
            base_write(l, 2, 8, &mut |l, png, info| unsafe {
                (l.api.png_set_eXIf_1)(
                    png,
                    info,
                    e.len() as u32,
                    if e.is_empty() {
                        ptr::null_mut()
                    } else {
                        e.as_ptr() as *mut u8
                    },
                );
                let mut n: u32 = 0;
                let mut p: *mut u8 = ptr::null_mut();
                log(format!(
                    "get_eXIf_1={} n={n} null={}",
                    (l.api.png_get_eXIf_1)(png, info, &mut n, &mut p),
                    p.is_null()
                ));
            })
        };
        diff(&format!("WC21 eXIf #{i} len={}", e.len()), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// WC22 cICP / WC23 cLLI / WC24 mDCV
// ---------------------------------------------------------------------------
#[test]
fn wc22_wc24_pngv3_chunks() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0x22000);
    for i in 0..24 {
        let (p, t, m, f) = (rng.u8(), rng.u8(), rng.u8(), rng.u8() % 4);
        let mut run = |l: &Lib| -> Report {
            base_write(l, 2, 8, &mut |l, png, info| unsafe {
                (l.api.png_set_cICP)(png, info, p, t, m, f);
                let mut o = [0u8; 4];
                log(format!(
                    "get_cICP={} {o:?}",
                    (l.api.png_get_cICP)(
                        png,
                        info,
                        &mut o[0],
                        &mut o[1],
                        &mut o[2],
                        &mut o[3]
                    )
                ));
            })
        };
        diff(&format!("WC22 cICP #{i} {p},{t},{m},{f}"), &c, &r, &mut run);
    }
    for &(a, b) in &[
        (0u32, 0u32),
        (1, 1),
        (10_000, 4_000_000),
        (0x7fff_ffff, 0x7fff_ffff),
        (0x8000_0000, 1),
        (0xffff_ffff, 0xffff_ffff),
    ] {
        let mut run = |l: &Lib| -> Report {
            base_write(l, 2, 8, &mut |l, png, info| unsafe {
                (l.api.png_set_cLLI_fixed)(png, info, a, b);
                let mut x = 0u32;
                let mut y = 0u32;
                log(format!(
                    "get_cLLI_fixed={} {x} {y}",
                    (l.api.png_get_cLLI_fixed)(png, info, &mut x, &mut y)
                ));
                let mut dx = 0f64;
                let mut dy = 0f64;
                log(format!(
                    "get_cLLI={} {dx} {dy}",
                    (l.api.png_get_cLLI)(png, info, &mut dx, &mut dy)
                ));
            })
        };
        diff(&format!("WC23 cLLI_fixed {a},{b}"), &c, &r, &mut run);
    }
    let mut rng = Rng::new(SEED ^ 0x24000);
    for i in 0..24 {
        let mut xy = [0i32; 8];
        for v in xy.iter_mut() {
            *v = (rng.u32() % 140_000) as i32;
        }
        let lmax = rng.u32() % 0x8000_0000;
        let lmin = rng.u32() % 0x8000_0000;
        let mut run = |l: &Lib| -> Report {
            base_write(l, 2, 8, &mut |l, png, info| unsafe {
                (l.api.png_set_mDCV_fixed)(
                    png, info, xy[0], xy[1], xy[2], xy[3], xy[4], xy[5], xy[6], xy[7], lmax, lmin,
                );
                let mut o = [0i32; 8];
                let mut a = 0u32;
                let mut b = 0u32;
                log(format!(
                    "get_mDCV_fixed={} {o:?} {a} {b}",
                    (l.api.png_get_mDCV_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4],
                        &mut o[5], &mut o[6], &mut o[7], &mut a, &mut b
                    )
                ));
                let mut d = [0f64; 10];
                log(format!(
                    "get_mDCV={} {d:?}",
                    (l.api.png_get_mDCV)(
                        png, info, &mut d[0], &mut d[1], &mut d[2], &mut d[3], &mut d[4],
                        &mut d[5], &mut d[6], &mut d[7], &mut d[8], &mut d[9]
                    )
                ));
            })
        };
        diff(&format!("WC24 mDCV_fixed #{i}"), &c, &r, &mut run);
    }
}

// ---------------------------------------------------------------------------
// WC25/WC26 unknown chunks on write
// ---------------------------------------------------------------------------
#[test]
fn wc25_wc26_unknown_chunks() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0x25000);
    let payloads: Vec<Vec<u8>> = vec![vec![], rng.bytes(1), rng.bytes(37), rng.bytes(300)];
    let names: &[&[u8; 5]] = &[b"prVt\0", b"PrVt\0", b"pRVt\0", b"PRVt\0", b"vpAg\0"];
    for loc in [PNG_HAVE_IHDR, PNG_HAVE_PLTE, PNG_AFTER_IDAT, 0, 0xff] {
        for keep in [
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
        ] {
            for num in [0i32, 5, -1] {
                for (pi, payload) in payloads.iter().enumerate() {
                    let mut run = |l: &Lib| -> Report {
                        base_write(l, 2, 8, &mut |l, png, info| unsafe {
                            let mut list: Vec<u8> = Vec::new();
                            for n in names {
                                list.extend_from_slice(&n[..4]);
                                list.push(0);
                            }
                            (l.api.png_set_keep_unknown_chunks)(png, keep, list.as_ptr(), num);
                            let unk: Vec<PngUnknownChunk> = names
                                .iter()
                                .map(|n| PngUnknownChunk {
                                    name: **n,
                                    data: if payload.is_empty() {
                                        ptr::null_mut()
                                    } else {
                                        payload.as_ptr() as *mut u8
                                    },
                                    size: payload.len(),
                                    location: loc as u8,
                                })
                                .collect();
                            (l.api.png_set_unknown_chunks)(
                                png,
                                info,
                                unk.as_ptr(),
                                unk.len() as c_int,
                            );
                            for i in 0..unk.len() as c_int {
                                (l.api.png_set_unknown_chunk_location)(png, info, i, loc);
                            }
                            let mut e: *mut PngUnknownChunk = ptr::null_mut();
                            log(format!(
                                "get_unknown_chunks={}",
                                (l.api.png_get_unknown_chunks)(png, info, &mut e)
                            ));
                        })
                    };
                    diff(
                        &format!("WC25-26 unknown loc={loc} keep={keep} num={num} payload#{pi}"),
                        &c,
                        &r,
                        &mut run,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WC27 png_set_invalid / png_free_data / png_data_freer
// ---------------------------------------------------------------------------
#[test]
fn wc27_invalid_free_data() {
    let (c, r) = libs();
    let masks: &[u32] = &[
        PNG_INFO_gAMA,
        PNG_INFO_sBIT,
        PNG_INFO_cHRM,
        PNG_INFO_PLTE,
        PNG_INFO_tRNS,
        PNG_INFO_bKGD,
        PNG_INFO_hIST,
        PNG_INFO_pHYs,
        PNG_INFO_oFFs,
        PNG_INFO_tIME,
        PNG_INFO_pCAL,
        PNG_INFO_sRGB,
        PNG_INFO_iCCP,
        PNG_INFO_sPLT,
        PNG_INFO_sCAL,
        PNG_INFO_IDAT,
        PNG_INFO_eXIf,
        PNG_INFO_cICP,
        PNG_INFO_cLLI,
        PNG_INFO_mDCV,
        0xffff_ffff,
        0,
    ];
    for &m in masks {
        let mut run = |l: &Lib| -> Report {
            base_write(l, 2, 8, &mut |l, png, info| unsafe {
                (l.api.png_set_gAMA_fixed)(png, info, 45455);
                (l.api.png_set_pHYs)(png, info, 100, 100, 1);
                let t = PngTime { year: 2000, month: 1, day: 1, hour: 1, minute: 1, second: 1 };
                (l.api.png_set_tIME)(png, info, &t);
                for flag in masks {
                    log(format!(
                        "valid({flag:#x})={}",
                        (l.api.png_get_valid)(png, info, *flag)
                    ));
                }
                (l.api.png_set_invalid)(png, info, m as c_int);
                for flag in masks {
                    log(format!(
                        "after valid({flag:#x})={}",
                        (l.api.png_get_valid)(png, info, *flag)
                    ));
                }
            })
        };
        diff(&format!("WC27 png_set_invalid mask={m:#x}"), &c, &r, &mut run);
    }
    for freer in [0i32, 1, 2, 3, 99] {
        for &m in &[PNG_FREE_ALL, PNG_FREE_TEXT, 0u32] {
            let mut run = |l: &Lib| -> Report {
                base_write(l, 2, 8, &mut |l, png, info| unsafe {
                    let key = CString::new("K").unwrap();
                    let txt = CString::new("V").unwrap();
                    let t = PngText {
                        compression: -1,
                        key: key.as_ptr() as *mut i8,
                        text: txt.as_ptr() as *mut i8,
                        text_length: 1,
                        ..Default::default()
                    };
                    (l.api.png_set_text)(png, info, &t, 1);
                    (l.api.png_data_freer)(png, info, freer, m);
                    (l.api.png_free_data)(png, info, m, -1);
                    log("freed".to_string());
                })
            };
            diff(
                &format!("WC27 data_freer freer={freer} mask={m:#x}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// WC28 IHDR accessors read-back
// ---------------------------------------------------------------------------
#[test]
fn wc28_ihdr_accessors() {
    let (c, r) = libs();
    for &(ct, bd) in &[
        (0i32, 1i32),
        (0, 16),
        (2, 8),
        (2, 16),
        (3, 4),
        (3, 8),
        (4, 8),
        (6, 16),
    ] {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let mut run = |l: &Lib| -> Report {
                let (w, h) = (13u32, 7u32);
                let pal = if ct == PNG_COLOR_TYPE_PALETTE {
                    make_palette(16, SEED ^ 0x28000)
                } else {
                    vec![]
                };
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, w, h, bd, ct, il, PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    if !pal.is_empty() {
                        (l.api.png_set_PLTE)(png, info, pal.as_ptr(), pal.len() as c_int);
                    }
                    let mut ow = 0u32;
                    let mut oh = 0u32;
                    let mut obd: c_int = 0;
                    let mut oct: c_int = 0;
                    let mut oil: c_int = 0;
                    let mut ocm: c_int = 0;
                    let mut ofm: c_int = 0;
                    log(format!(
                        "get_IHDR={} {ow}x{oh} bd={obd} ct={oct} il={oil} cm={ocm} fm={ofm}",
                        (l.api.png_get_IHDR)(
                            png, info, &mut ow, &mut oh, &mut obd, &mut oct, &mut oil, &mut ocm,
                            &mut ofm
                        )
                    ));
                    log(format!(
                        "w={} h={} bd={} ct={} ft={} it={} compt={} ch={} rowbytes={}",
                        (l.api.png_get_image_width)(png, info),
                        (l.api.png_get_image_height)(png, info),
                        (l.api.png_get_bit_depth)(png, info),
                        (l.api.png_get_color_type)(png, info),
                        (l.api.png_get_filter_type)(png, info),
                        (l.api.png_get_interlace_type)(png, info),
                        (l.api.png_get_compression_type)(png, info),
                        (l.api.png_get_channels)(png, info),
                        (l.api.png_get_rowbytes)(png, info)
                    ));
                    let mut p: *mut PngColor = ptr::null_mut();
                    let mut n: c_int = 0;
                    log(format!(
                        "get_PLTE={} n={n}",
                        (l.api.png_get_PLTE)(png, info, &mut p, &mut n)
                    ));
                    log(format!(
                        "user_width_max={} user_height_max={} chunk_cache_max={} chunk_malloc_max={}",
                        (l.api.png_get_user_width_max)(png),
                        (l.api.png_get_user_height_max)(png),
                        (l.api.png_get_chunk_cache_max)(png),
                        (l.api.png_get_chunk_malloc_max)(png)
                    ));
                })
            };
            diff(&format!("WC28 IHDR accessors ct={ct} bd={bd} il={il}"), &c, &r, &mut run);
        }
    }
}
