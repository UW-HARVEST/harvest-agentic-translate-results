//! Chunk set/get round-trip differential tests (CONFIGS.md rows C1..C24).
//!
//! Every row drives a *complete* round trip inside one library: the image is
//! written (exercising `pngset.c` + `pngwutil.c`), the bytes that were just
//! produced are fed straight back into a read with the SAME library
//! (exercising `pngrutil.c` + `pngget.c`), and the concatenation of both
//! traces is compared byte-for-byte between the C reference and the Rust
//! translation.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::too_many_arguments)]

mod support;

use std::ffi::{c_char, c_int, c_void, CString};
use support::core::*;
use support::*;

// ---------------------------------------------------------------------------
// constants (taken from c_src/include/png.h; note that a few of the values in
// tests/support/core.rs do not match png.h, so the correct ones are defined
// locally here and used for everything this file logs itself)
// ---------------------------------------------------------------------------

const I_gAMA: u32 = 0x0001;
const I_sBIT: u32 = 0x0002;
const I_cHRM: u32 = 0x0004;
const I_PLTE: u32 = 0x0008;
const I_tRNS: u32 = 0x0010;
const I_bKGD: u32 = 0x0020;
const I_hIST: u32 = 0x0040;
const I_pHYs: u32 = 0x0080;
const I_oFFs: u32 = 0x0100;
const I_tIME: u32 = 0x0200;
const I_pCAL: u32 = 0x0400;
const I_sRGB: u32 = 0x0800;
const I_iCCP: u32 = 0x1000;
const I_sPLT: u32 = 0x2000;
const I_sCAL: u32 = 0x4000;
const I_IDAT: u32 = 0x8000;
const I_eXIf: u32 = 0x10000;
const I_cICP: u32 = 0x20000;
const I_cLLI: u32 = 0x40000;
const I_mDCV: u32 = 0x80000;

const F_ROWS: u32 = 0x0040;

const LOC_HAVE_IHDR: c_int = 0x01;
const LOC_HAVE_PLTE: c_int = 0x02;
const LOC_AFTER_IDAT: c_int = 0x08;

const VFLAGS: &[(&str, u32)] = &[
    ("gAMA", I_gAMA),
    ("sBIT", I_sBIT),
    ("cHRM", I_cHRM),
    ("PLTE", I_PLTE),
    ("tRNS", I_tRNS),
    ("bKGD", I_bKGD),
    ("hIST", I_hIST),
    ("pHYs", I_pHYs),
    ("oFFs", I_oFFs),
    ("tIME", I_tIME),
    ("pCAL", I_pCAL),
    ("sRGB", I_sRGB),
    ("iCCP", I_iCCP),
    ("sPLT", I_sPLT),
    ("sCAL", I_sCAL),
    ("IDAT", I_IDAT),
    ("eXIf", I_eXIf),
    ("cICP", I_cICP),
    ("cLLI", I_cLLI),
    ("mDCV", I_mDCV),
];

// ---------------------------------------------------------------------------
// The reference `target/cbuild/libpng.so` is not linked against libm (see
// `ldd`), yet `png_fixed_ITU` / `png_set_sCAL` call `floor`.  Nothing else in
// the process brings libm into the *global* symbol scope, so the first call
// would die with `undefined symbol: floor`.  Make libm globally visible before
// the first libpng call.  This only affects symbol resolution, never the
// behaviour of either library.  (`support::pair()` does the same thing; this
// local copy keeps the file self-contained and is idempotent.)
// ---------------------------------------------------------------------------

fn ensure_libm() {
    static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| unsafe {
        extern "C" {
            fn dlopen(file: *const c_char, flag: c_int) -> *mut c_void;
        }
        const RTLD_NOW: c_int = 2;
        const RTLD_GLOBAL: c_int = 0x100;
        for name in ["libm.so.6", "libm.so"] {
            let n = CString::new(name).unwrap();
            if !dlopen(n.as_ptr(), RTLD_NOW | RTLD_GLOBAL).is_null() {
                break;
            }
        }
    });
}

// ---------------------------------------------------------------------------
// entry points that are not in `Core`
// ---------------------------------------------------------------------------

type Getter32 = unsafe extern "C" fn(Png, Info) -> u32;
type Getteri32 = unsafe extern "C" fn(Png, Info) -> i32;
type Getterf32 = unsafe extern "C" fn(Png, Info) -> f32;

struct Ext {
    set_cHRM: unsafe extern "C" fn(Png, Info, f64, f64, f64, f64, f64, f64, f64, f64),
    get_cHRM: unsafe extern "C" fn(
        Png,
        Info,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
    ) -> u32,
    set_cHRM_XYZ: unsafe extern "C" fn(Png, Info, f64, f64, f64, f64, f64, f64, f64, f64, f64),
    get_cHRM_XYZ: unsafe extern "C" fn(
        Png,
        Info,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
    ) -> u32,
    set_cLLI: unsafe extern "C" fn(Png, Info, f64, f64),
    get_cLLI: unsafe extern "C" fn(Png, Info, *mut f64, *mut f64) -> u32,
    set_mDCV: unsafe extern "C" fn(Png, Info, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64),
    get_mDCV: unsafe extern "C" fn(
        Png,
        Info,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
        *mut f64,
    ) -> u32,
    get_sCAL: unsafe extern "C" fn(Png, Info, *mut c_int, *mut f64, *mut f64) -> u32,
    get_sCAL_fixed: unsafe extern "C" fn(Png, Info, *mut c_int, *mut i32, *mut i32) -> u32,
    set_eXIf: unsafe extern "C" fn(Png, Info, *mut u8),
    get_eXIf: unsafe extern "C" fn(Png, Info, *mut *mut u8) -> u32,
    get_pHYs_dpi: unsafe extern "C" fn(Png, Info, *mut u32, *mut u32, *mut c_int) -> u32,
    ppm: Getter32,
    x_ppm: Getter32,
    y_ppm: Getter32,
    ppi: Getter32,
    x_ppi: Getter32,
    y_ppi: Getter32,
    aspect: Getterf32,
    aspect_fixed: Getteri32,
    x_off_px: Getteri32,
    y_off_px: Getteri32,
    x_off_um: Getteri32,
    y_off_um: Getteri32,
    x_off_in: Getterf32,
    y_off_in: Getterf32,
    x_off_in_fx: Getteri32,
    y_off_in_fx: Getteri32,
    /// `png_set_mDCV_fixed` with the exact png.h signature (the chromaticities
    /// are `png_fixed_point`, i.e. signed).
    set_mDCV_fx: unsafe extern "C" fn(Png, Info, i32, i32, i32, i32, i32, i32, i32, i32, u32, u32),
    get_mDCV_fx: unsafe extern "C" fn(
        Png,
        Info,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut u32,
        *mut u32,
    ) -> u32,
}

impl Ext {
    fn new(lib: &Lib) -> Ext {
        Ext {
            set_cHRM: lib.f("png_set_cHRM"),
            get_cHRM: lib.f("png_get_cHRM"),
            set_cHRM_XYZ: lib.f("png_set_cHRM_XYZ"),
            get_cHRM_XYZ: lib.f("png_get_cHRM_XYZ"),
            set_cLLI: lib.f("png_set_cLLI"),
            get_cLLI: lib.f("png_get_cLLI"),
            set_mDCV: lib.f("png_set_mDCV"),
            get_mDCV: lib.f("png_get_mDCV"),
            get_sCAL: lib.f("png_get_sCAL"),
            get_sCAL_fixed: lib.f("png_get_sCAL_fixed"),
            set_eXIf: lib.f("png_set_eXIf"),
            get_eXIf: lib.f("png_get_eXIf"),
            get_pHYs_dpi: lib.f("png_get_pHYs_dpi"),
            ppm: lib.f("png_get_pixels_per_meter"),
            x_ppm: lib.f("png_get_x_pixels_per_meter"),
            y_ppm: lib.f("png_get_y_pixels_per_meter"),
            ppi: lib.f("png_get_pixels_per_inch"),
            x_ppi: lib.f("png_get_x_pixels_per_inch"),
            y_ppi: lib.f("png_get_y_pixels_per_inch"),
            aspect: lib.f("png_get_pixel_aspect_ratio"),
            aspect_fixed: lib.f("png_get_pixel_aspect_ratio_fixed"),
            x_off_px: lib.f("png_get_x_offset_pixels"),
            y_off_px: lib.f("png_get_y_offset_pixels"),
            x_off_um: lib.f("png_get_x_offset_microns"),
            y_off_um: lib.f("png_get_y_offset_microns"),
            x_off_in: lib.f("png_get_x_offset_inches"),
            y_off_in: lib.f("png_get_y_offset_inches"),
            x_off_in_fx: lib.f("png_get_x_offset_inches_fixed"),
            y_off_in_fx: lib.f("png_get_y_offset_inches_fixed"),
            set_mDCV_fx: lib.f("png_set_mDCV_fixed"),
            get_mDCV_fx: lib.f("png_get_mDCV_fixed"),
        }
    }
}

// ---------------------------------------------------------------------------
// logging helpers (never log a pointer, only null-ness / sizes / contents)
// ---------------------------------------------------------------------------

unsafe fn log_valid(c: &Core, png: Png, info: Info) {
    let mut s = String::new();
    for (n, f) in VFLAGS {
        s.push_str(&format!("{n}:{} ", (c.get_valid)(png, info, *f)));
    }
    log(format!("V[{}]", s.trim_end()));
}

unsafe fn log_chrm_fp(e: &Ext, png: Png, info: Info) {
    let mut v = [-1.0f64; 8];
    let r = (e.get_cHRM)(
        png, info, &mut v[0], &mut v[1], &mut v[2], &mut v[3], &mut v[4], &mut v[5], &mut v[6],
        &mut v[7],
    );
    let mut s = String::new();
    for x in v.iter() {
        s.push_str(&format!("{x:.10} "));
    }
    log(format!("cHRM_fp rc={r} [{}]", s.trim_end()));
    let mut x = [-1.0f64; 9];
    let r = (e.get_cHRM_XYZ)(
        png, info, &mut x[0], &mut x[1], &mut x[2], &mut x[3], &mut x[4], &mut x[5], &mut x[6],
        &mut x[7], &mut x[8],
    );
    let mut s = String::new();
    for q in x.iter() {
        s.push_str(&format!("{q:.10} "));
    }
    log(format!("cHRM_XYZ_fp rc={r} [{}]", s.trim_end()));
}

unsafe fn log_phys_extra(e: &Ext, png: Png, info: Info) {
    let (mut x, mut y, mut u) = (0u32, 0u32, -1i32);
    let rc = (e.get_pHYs_dpi)(png, info, &mut x, &mut y, &mut u);
    log(format!("pHYs_dpi rc={rc} x={x} y={y} unit={u}"));
    log(format!(
        "ppm={} xppm={} yppm={}",
        (e.ppm)(png, info),
        (e.x_ppm)(png, info),
        (e.y_ppm)(png, info)
    ));
    log(format!(
        "ppi={} xppi={} yppi={}",
        (e.ppi)(png, info),
        (e.x_ppi)(png, info),
        (e.y_ppi)(png, info)
    ));
    log(format!(
        "aspect={:.10} aspect_fixed={}",
        (e.aspect)(png, info) as f64,
        (e.aspect_fixed)(png, info)
    ));
}

unsafe fn log_offs_extra(e: &Ext, png: Png, info: Info) {
    log(format!(
        "off_px x={} y={}",
        (e.x_off_px)(png, info),
        (e.y_off_px)(png, info)
    ));
    log(format!(
        "off_um x={} y={}",
        (e.x_off_um)(png, info),
        (e.y_off_um)(png, info)
    ));
    log(format!(
        "off_in x={:.10} y={:.10}",
        (e.x_off_in)(png, info) as f64,
        (e.y_off_in)(png, info) as f64
    ));
    log(format!(
        "off_in_fx x={} y={}",
        (e.x_off_in_fx)(png, info),
        (e.y_off_in_fx)(png, info)
    ));
}

unsafe fn log_scal_extra(e: &Ext, png: Png, info: Info) {
    let mut u: c_int = -1;
    let (mut w, mut h) = (-1.0f64, -1.0f64);
    let r = (e.get_sCAL)(png, info, &mut u, &mut w, &mut h);
    log(format!("sCAL_fp rc={r} unit={u} w={w:.10} h={h:.10}"));
    let mut u2: c_int = -1;
    let (mut wf, mut hf) = (-1i32, -1i32);
    let r = (e.get_sCAL_fixed)(png, info, &mut u2, &mut wf, &mut hf);
    log(format!("sCAL_fx rc={r} unit={u2} w={wf} h={hf}"));
}

unsafe fn log_cLLI_fp(e: &Ext, png: Png, info: Info) {
    let (mut a, mut b) = (-1.0f64, -1.0f64);
    let r = (e.get_cLLI)(png, info, &mut a, &mut b);
    log(format!("cLLI_fp rc={r} maxCLL={a:.10} maxFALL={b:.10}"));
}

unsafe fn log_mDCV_fp(e: &Ext, png: Png, info: Info) {
    let mut v = [-1.0f64; 10];
    let r = (e.get_mDCV)(
        png, info, &mut v[0], &mut v[1], &mut v[2], &mut v[3], &mut v[4], &mut v[5], &mut v[6],
        &mut v[7], &mut v[8], &mut v[9],
    );
    let mut s = String::new();
    for x in v.iter() {
        s.push_str(&format!("{x:.10} "));
    }
    log(format!("mDCV_fp rc={r} [{}]", s.trim_end()));
    let mut xy = [-1i32; 8];
    let (mut maxdl, mut mindl) = (0u32, 0u32);
    let r = (e.get_mDCV_fx)(
        png, info, &mut xy[0], &mut xy[1], &mut xy[2], &mut xy[3], &mut xy[4], &mut xy[5],
        &mut xy[6], &mut xy[7], &mut maxdl, &mut mindl,
    );
    log(format!("mDCV_fx rc={r} {xy:?} maxDL={maxdl} minDL={mindl}"));
}

unsafe fn log_exif_old(e: &Ext, png: Png, info: Info) {
    let mut p: *mut u8 = std::ptr::null_mut();
    let r = (e.get_eXIf)(png, info, &mut p);
    log(format!("eXIf_old rc={r} null={}", p.is_null()));
}

unsafe fn log_rfc1123(c: &Core, png: Png, info: Info) {
    let mut tp: *mut u8 = std::ptr::null_mut();
    let r = (c.get_tIME)(png, info, &mut tp);
    if r != 0 && !tp.is_null() {
        let mut buf = [0 as c_char; 40];
        let rc = (c.convert_to_rfc1123_buffer)(buf.as_mut_ptr(), tp);
        if rc != 0 {
            log(format!("rfc1123 rc={rc} s={}", cstr(buf.as_ptr())));
        } else {
            log(format!("rfc1123 rc={rc}"));
        }
    } else {
        log("rfc1123 no-tIME".to_string());
    }
}

// ---------------------------------------------------------------------------
// image data helpers
// ---------------------------------------------------------------------------

fn stride_of(color: c_int, depth: c_int, w: u32) -> usize {
    support::pngbuild::rowbytes(color as u8, depth as u8, w)
}

fn put_index(row: &mut [u8], x: usize, depth: c_int, val: u8) {
    let d = depth as usize;
    if d == 8 {
        row[x] = val;
    } else {
        let per = 8 / d;
        let byte = x / per;
        let shift = 8 - d * (x % per + 1);
        let mask = ((1u16 << d) - 1) as u8;
        row[byte] = (row[byte] & !(mask << shift)) | ((val & mask) << shift);
    }
}

/// `h` deterministic rows.  For palette images every index is < `npal` so
/// libpng's palette-index check never fires.
fn img(seed: u64, w: u32, h: u32, depth: c_int, color: c_int, npal: u32) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let st = stride_of(color, depth, w);
    let mut v = vec![0u8; st * h as usize];
    if color == PNG_COLOR_TYPE_PALETTE {
        let lim = (1u32 << depth).min(if npal == 0 { 1 } else { npal });
        for y in 0..h as usize {
            for x in 0..w as usize {
                let idx = rng.below(lim) as u8;
                put_index(&mut v[y * st..(y + 1) * st], x, depth, idx);
            }
        }
    } else {
        for b in v.iter_mut() {
            *b = rng.byte();
        }
    }
    v
}

// ---------------------------------------------------------------------------
// the round-trip driver
// ---------------------------------------------------------------------------

/// write (with `set` applied before `png_write_info`) then read back the very
/// bytes just produced, with the same library.  `probe` is called after each
/// `log_all_info` so a test can add getters that `log_all_info` does not cover.
fn roundtrip(
    lib: &Lib,
    w: u32,
    h: u32,
    depth: c_int,
    color: c_int,
    rows: &[u8],
    set: &mut dyn FnMut(&Core, Png, Info),
    probe: &mut dyn FnMut(&Core, Png, Info),
    rdpre: &mut dyn FnMut(&Core, Png, Info),
) -> Trace {
    let st = stride_of(color, depth, w);
    let t1 = with_write(lib, &mut |c, png, info| unsafe {
        (c.set_IHDR)(
            png,
            info,
            w,
            h,
            depth,
            color,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        set(c, png, info);
        log("--- W setters ---");
        log_valid(c, png, info);
        log_all_info(c, png, info);
        probe(c, png, info);
        (c.write_info)(png, info);
        for y in 0..h as usize {
            (c.write_row)(png, rows.as_ptr().add(y * st));
        }
        (c.write_end)(png, info);
        log("--- W after write_end ---");
        log_valid(c, png, info);
        log_all_info(c, png, info);
        probe(c, png, info);
    });
    let produced = t1.out.clone();
    let mut lines = t1.lines.clone();
    let mut buf: Vec<u8> = vec![0u8; st + 32];
    let t2 = with_read(lib, &produced, &mut |c, png, info| unsafe {
        rdpre(c, png, info);
        (c.read_info)(png, info);
        log("--- R after read_info ---");
        log_valid(c, png, info);
        log_all_info(c, png, info);
        probe(c, png, info);
        let n = (c.get_rowbytes)(png, info).min(buf.len());
        for y in 0..h {
            (c.read_row)(png, buf.as_mut_ptr(), std::ptr::null_mut());
            log(format!("row{y}={}", hex(&buf[..n])));
        }
        (c.read_end)(png, info);
        log("--- R after read_end ---");
        log_valid(c, png, info);
        log_all_info(c, png, info);
        probe(c, png, info);
    });
    lines.extend(t2.lines);
    Trace {
        lines,
        out: produced,
        rc: t1.rc | (t2.rc << 8),
    }
}

fn rt(
    lib: &Lib,
    w: u32,
    h: u32,
    depth: c_int,
    color: c_int,
    rows: &[u8],
    set: &mut dyn FnMut(&Core, Png, Info),
) -> Trace {
    let mut a = |_: &Core, _: Png, _: Info| {};
    let mut b = |_: &Core, _: Png, _: Info| {};
    roundtrip(lib, w, h, depth, color, rows, set, &mut a, &mut b)
}

fn rt_probe(
    lib: &Lib,
    w: u32,
    h: u32,
    depth: c_int,
    color: c_int,
    rows: &[u8],
    set: &mut dyn FnMut(&Core, Png, Info),
    probe: &mut dyn FnMut(&Core, Png, Info),
) -> Trace {
    let mut b = |_: &Core, _: Png, _: Info| {};
    roundtrip(lib, w, h, depth, color, rows, set, probe, &mut b)
}

const W: u32 = 8;
const H: u32 = 4;

// ===========================================================================
// C1 — png_set_PLTE / png_get_PLTE
// ===========================================================================

#[test]
fn c1_plte() {
    ensure_libm();
    // (bit depth, palette entries) — only sizes that are legal for the depth
    let cfgs: &[(c_int, c_int)] = &[
        (1, 1),
        (1, 2),
        (2, 1),
        (2, 2),
        (2, 4),
        (4, 1),
        (4, 2),
        (4, 16),
        (8, 1),
        (8, 2),
        (8, 16),
        (8, 255),
        (8, 256),
    ];
    // several random palettes and several image widths per configuration (the
    // widths cover the sub-byte packing remainders)
    for &(depth, npal) in cfgs {
        for (si, &w) in [1u32, 7, 8, 17].iter().enumerate() {
            let mut rng = Rng::new(0xC100_0000 + depth as u64 * 4096 + npal as u64 * 8 + si as u64);
            let pal = rng.bytes(npal as usize * 3);
            let rows = img(
                0xC101_0000 + npal as u64 * 16 + si as u64,
                w,
                H,
                depth,
                PNG_COLOR_TYPE_PALETTE,
                npal as u32,
            );
            diff(&format!("C1 palette depth={depth} n={npal} w={w}"), |lib| {
                rt(
                    lib,
                    w,
                    H,
                    depth,
                    PNG_COLOR_TYPE_PALETTE,
                    &rows,
                    &mut |c, png, info| unsafe {
                        (c.set_PLTE)(png, info, pal.as_ptr(), npal);
                        log(format!(
                            "set_PLTE n={npal} valid={}",
                            (c.get_valid)(png, info, I_PLTE)
                        ));
                    },
                )
            });
        }
    }
    // a palette attached to a truecolour image: legal, written as a suggested
    // palette
    for &npal in &[1i32, 2, 16, 255, 256] {
        for &(depth, color) in &[
            (8i32, PNG_COLOR_TYPE_RGB),
            (16, PNG_COLOR_TYPE_RGB),
            (8, PNG_COLOR_TYPE_RGB_ALPHA),
        ] {
            let mut rng = Rng::new(0xC190_0000 + npal as u64 * 32 + depth as u64 + color as u64);
            let pal = rng.bytes(npal as usize * 3);
            let rows = img(
                0xC191_0000 + npal as u64 * 32 + depth as u64,
                W,
                H,
                depth,
                color,
                0,
            );
            diff(
                &format!("C1 suggested-palette n={npal} depth={depth} color={color}"),
                |lib| {
                    rt(lib, W, H, depth, color, &rows, &mut |c, png, info| unsafe {
                        (c.set_PLTE)(png, info, pal.as_ptr(), npal);
                        log(format!(
                            "set_PLTE n={npal} valid={}",
                            (c.get_valid)(png, info, I_PLTE)
                        ));
                    })
                },
            );
        }
    }
}

// ===========================================================================
// C2 — png_set_tRNS / png_get_tRNS
// ===========================================================================

#[test]
fn c2_trns() {
    ensure_libm();
    // palette alpha arrays, 1..256 entries, three random alpha arrays each
    for &nt in &[1i32, 2, 3, 16, 64, 129, 255, 256] {
        for s in 0..3u64 {
            let mut rng = Rng::new(0xC200_0000 + nt as u64 * 8 + s);
            let pal = rng.bytes(256 * 3);
            let alpha = rng.bytes(nt as usize);
            let rows = img(
                0xC201_0000 + nt as u64 * 8 + s,
                W,
                H,
                8,
                PNG_COLOR_TYPE_PALETTE,
                256,
            );
            diff(&format!("C2 palette-tRNS n={nt}[{s}]"), |lib| {
                rt(
                    lib,
                    W,
                    H,
                    8,
                    PNG_COLOR_TYPE_PALETTE,
                    &rows,
                    &mut |c, png, info| unsafe {
                        (c.set_PLTE)(png, info, pal.as_ptr(), 256);
                        (c.set_tRNS)(png, info, alpha.as_ptr(), nt, std::ptr::null());
                        log(format!(
                            "set_tRNS n={nt} valid={}",
                            (c.get_valid)(png, info, I_tRNS)
                        ));
                    },
                )
            });
        }
    }
    // palette alpha with fewer palette entries than the depth allows
    for &(depth, npal) in &[(1i32, 2i32), (2, 3), (4, 9), (8, 100)] {
        let step = if npal > 12 { 23 } else { 1 };
        for nt in (1..=npal).step_by(step) {
            let mut rng = Rng::new(0xC210_0000 + depth as u64 * 512 + nt as u64);
            let pal = rng.bytes(npal as usize * 3);
            let alpha = rng.bytes(nt as usize);
            let rows = img(
                0xC211_0000 + depth as u64 * 512 + nt as u64,
                W,
                H,
                depth,
                PNG_COLOR_TYPE_PALETTE,
                npal as u32,
            );
            diff(
                &format!("C2 palette-tRNS depth={depth} npal={npal} nt={nt}"),
                |lib| {
                    rt(
                        lib,
                        W,
                        H,
                        depth,
                        PNG_COLOR_TYPE_PALETTE,
                        &rows,
                        &mut |c, png, info| unsafe {
                            (c.set_PLTE)(png, info, pal.as_ptr(), npal);
                            (c.set_tRNS)(png, info, alpha.as_ptr(), nt, std::ptr::null());
                            log(format!(
                                "set_tRNS n={nt} valid={}",
                                (c.get_valid)(png, info, I_tRNS)
                            ));
                        },
                    )
                },
            );
        }
    }
    // grey key, 8 and 16 bit
    for &depth in &[8i32, 16] {
        let maxv: u32 = if depth == 8 { 255 } else { 65535 };
        let mut rng = Rng::new(0xC220_0000 + depth as u64);
        let mut keys: Vec<u16> = vec![0, maxv as u16];
        for _ in 0..4 {
            keys.push(rng.below(maxv + 1) as u16);
        }
        for (i, &g) in keys.iter().enumerate() {
            let key = PngColor16 {
                gray: g,
                ..Default::default()
            };
            let rows = img(
                0xC221_0000 + i as u64 + depth as u64 * 97,
                W,
                H,
                depth,
                PNG_COLOR_TYPE_GRAY,
                0,
            );
            diff(&format!("C2 gray-tRNS depth={depth} gray={g}"), |lib| {
                rt(
                    lib,
                    W,
                    H,
                    depth,
                    PNG_COLOR_TYPE_GRAY,
                    &rows,
                    &mut |c, png, info| unsafe {
                        (c.set_tRNS)(
                            png,
                            info,
                            std::ptr::null(),
                            0,
                            &key as *const PngColor16 as *const u8,
                        );
                        log(format!(
                            "set_tRNS gray={g} valid={}",
                            (c.get_valid)(png, info, I_tRNS)
                        ));
                    },
                )
            });
        }
    }
    // RGB key, 8 and 16 bit
    for &depth in &[8i32, 16] {
        let maxv: u32 = if depth == 8 { 255 } else { 65535 };
        let mut rng = Rng::new(0xC240_0000 + depth as u64);
        let mut keys: Vec<(u16, u16, u16)> = vec![(0, 0, 0), (maxv as u16, 0, maxv as u16)];
        for _ in 0..4 {
            keys.push((
                rng.below(maxv + 1) as u16,
                rng.below(maxv + 1) as u16,
                rng.below(maxv + 1) as u16,
            ));
        }
        for (i, &(r, g, b)) in keys.iter().enumerate() {
            let key = PngColor16 {
                red: r,
                green: g,
                blue: b,
                ..Default::default()
            };
            let rows = img(
                0xC241_0000 + i as u64 + depth as u64 * 31,
                W,
                H,
                depth,
                PNG_COLOR_TYPE_RGB,
                0,
            );
            diff(
                &format!("C2 rgb-tRNS depth={depth} key={r},{g},{b}"),
                |lib| {
                    rt(
                        lib,
                        W,
                        H,
                        depth,
                        PNG_COLOR_TYPE_RGB,
                        &rows,
                        &mut |c, png, info| unsafe {
                            (c.set_tRNS)(
                                png,
                                info,
                                std::ptr::null(),
                                0,
                                &key as *const PngColor16 as *const u8,
                            );
                            log(format!(
                                "set_tRNS rgb valid={}",
                                (c.get_valid)(png, info, I_tRNS)
                            ));
                        },
                    )
                },
            );
        }
    }
}

// ===========================================================================
// C3 — png_set_gAMA / png_set_gAMA_fixed
// ===========================================================================

#[test]
fn c3_gama() {
    ensure_libm();
    let mut rng = Rng::new(0xC300_0001);
    let mut fixed: Vec<i32> = vec![0, 1, 45455, 100000, 220000, 2147483647, -1, i32::MIN];
    for _ in 0..8 {
        fixed.push((rng.below(2_000_000) + 1) as i32);
    }
    let rows = img(0xC303, W, H, 8, PNG_COLOR_TYPE_GRAY, 0);
    for &g in &fixed {
        diff(&format!("C3 gAMA_fixed={g}"), |lib| {
            rt(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_GRAY,
                &rows,
                &mut |c, png, info| unsafe {
                    (c.set_gAMA_fixed)(png, info, g);
                    log(format!(
                        "set_gAMA_fixed({g}) valid={}",
                        (c.get_valid)(png, info, I_gAMA)
                    ));
                },
            )
        });
    }
    let mut rng = Rng::new(0xC300_0002);
    let mut dbl: Vec<f64> = vec![0.0, 1.0, 0.45455, 2.2, 2.2e3, -1.5, 21474.83647];
    for _ in 0..6 {
        dbl.push(rng.f64() * 20.0 + 0.000_01);
    }
    for (i, &g) in dbl.iter().enumerate() {
        diff(&format!("C3 gAMA_fp[{i}]={g:.10}"), |lib| {
            let set_gAMA = c_set_gAMA(lib);
            rt(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_GRAY,
                &rows,
                &mut |c, png, info| unsafe {
                    set_gAMA(png, info, g);
                    log(format!(
                        "set_gAMA({g:.10}) valid={}",
                        (c.get_valid)(png, info, I_gAMA)
                    ));
                },
            )
        });
    }
}

fn c_set_gAMA(lib: &Lib) -> unsafe extern "C" fn(Png, Info, f64) {
    lib.f("png_set_gAMA")
}

fn c_set_sCAL(lib: &Lib) -> unsafe extern "C" fn(Png, Info, c_int, f64, f64) {
    lib.f("png_set_sCAL")
}

// ===========================================================================
// C4 — png_set_sRGB / png_set_sRGB_gAMA_and_cHRM
// ===========================================================================

#[test]
fn c4_srgb() {
    ensure_libm();
    for &(color, depth) in &[
        (PNG_COLOR_TYPE_GRAY, 8i32),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        let npal: c_int = if color == PNG_COLOR_TYPE_PALETTE {
            16
        } else {
            0
        };
        let pal = {
            let mut r = Rng::new(0xC400_0000 + color as u64);
            r.bytes(npal as usize * 3)
        };
        let rows = img(0xC401_0000 + color as u64, W, H, depth, color, npal as u32);
        for intent in 0..4 {
            diff(&format!("C4 sRGB intent={intent} color={color}"), |lib| {
                let e = Ext::new(lib);
                rt_probe(
                    lib,
                    W,
                    H,
                    depth,
                    color,
                    &rows,
                    &mut |c, png, info| unsafe {
                        if npal > 0 {
                            (c.set_PLTE)(png, info, pal.as_ptr(), npal);
                        }
                        (c.set_sRGB)(png, info, intent);
                        log(format!(
                            "set_sRGB({intent}) valid={}",
                            (c.get_valid)(png, info, I_sRGB)
                        ));
                    },
                    &mut |_c, png, info| unsafe { log_chrm_fp(&e, png, info) },
                )
            });
            diff(
                &format!("C4 sRGB_gAMA_and_cHRM intent={intent} color={color}"),
                |lib| {
                    let e = Ext::new(lib);
                    rt_probe(
                        lib,
                        W,
                        H,
                        depth,
                        color,
                        &rows,
                        &mut |c, png, info| unsafe {
                            if npal > 0 {
                                (c.set_PLTE)(png, info, pal.as_ptr(), npal);
                            }
                            (c.set_sRGB_gAMA_and_cHRM)(png, info, intent);
                            log(format!(
                                "set_sRGB_gAMA_and_cHRM({intent}) sRGB={} gAMA={} cHRM={}",
                                (c.get_valid)(png, info, I_sRGB),
                                (c.get_valid)(png, info, I_gAMA),
                                (c.get_valid)(png, info, I_cHRM)
                            ));
                        },
                        &mut |_c, png, info| unsafe { log_chrm_fp(&e, png, info) },
                    )
                },
            );
        }
    }
}

// ===========================================================================
// C5 — png_set_cHRM / _fixed / _XYZ / _XYZ_fixed
// ===========================================================================

/// sRGB primaries in libpng fixed point (white, red, green, blue).
const SRGB_XY: [i32; 8] = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000];
/// sRGB D65 XYZ endpoints scaled by 100000 (red XYZ, green XYZ, blue XYZ).
const SRGB_XYZ: [i32; 9] = [41239, 21264, 1933, 35758, 71517, 11919, 18048, 7220, 95053];

#[test]
fn c5_chrm() {
    ensure_libm();
    let rows = img(0xC501, W, H, 8, PNG_COLOR_TYPE_RGB, 0);

    // fixed-point setter with the sRGB primaries and random plausible values
    let mut rng = Rng::new(0xC500_0001);
    let mut sets: Vec<[i32; 8]> = vec![SRGB_XY];
    for _ in 0..6 {
        let mut v = [0i32; 8];
        for x in v.iter_mut() {
            *x = (rng.below(70_000) + 1000) as i32;
        }
        sets.push(v);
    }
    for (i, v) in sets.iter().enumerate() {
        let v = *v;
        diff(&format!("C5 cHRM_fixed[{i}]"), |lib| {
            let e = Ext::new(lib);
            rt_probe(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (c.set_cHRM_fixed)(png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
                    log(format!(
                        "set_cHRM_fixed{v:?} valid={}",
                        (c.get_valid)(png, info, I_cHRM)
                    ));
                },
                &mut |_c, png, info| unsafe { log_chrm_fp(&e, png, info) },
            )
        });
    }

    // floating-point setter
    for (i, v) in sets.iter().enumerate() {
        let v = *v;
        diff(&format!("C5 cHRM_fp[{i}]"), |lib| {
            let e = Ext::new(lib);
            let d: Vec<f64> = v.iter().map(|x| *x as f64 / 100000.0).collect();
            rt_probe(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (e.set_cHRM)(png, info, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]);
                    log(format!(
                        "set_cHRM valid={}",
                        (c.get_valid)(png, info, I_cHRM)
                    ));
                },
                &mut |_c, png, info| unsafe { log_chrm_fp(&e, png, info) },
            )
        });
    }

    // XYZ round trip, fixed and floating point
    let mut rng = Rng::new(0xC500_0002);
    let mut xyzs: Vec<[i32; 9]> = vec![SRGB_XYZ];
    for _ in 0..4 {
        let mut v = SRGB_XYZ;
        for x in v.iter_mut() {
            *x += (rng.below(4000) as i32) - 2000;
        }
        xyzs.push(v);
    }
    for (i, v) in xyzs.iter().enumerate() {
        let v = *v;
        diff(&format!("C5 cHRM_XYZ_fixed[{i}]"), |lib| {
            let e = Ext::new(lib);
            rt_probe(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (c.set_cHRM_XYZ_fixed)(
                        png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
                    );
                    log(format!(
                        "set_cHRM_XYZ_fixed{v:?} valid={}",
                        (c.get_valid)(png, info, I_cHRM)
                    ));
                },
                &mut |_c, png, info| unsafe { log_chrm_fp(&e, png, info) },
            )
        });
        diff(&format!("C5 cHRM_XYZ_fp[{i}]"), |lib| {
            let e = Ext::new(lib);
            let d: Vec<f64> = v.iter().map(|x| *x as f64 / 100000.0).collect();
            rt_probe(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (e.set_cHRM_XYZ)(
                        png, info, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7], d[8],
                    );
                    log(format!(
                        "set_cHRM_XYZ valid={}",
                        (c.get_valid)(png, info, I_cHRM)
                    ));
                },
                &mut |_c, png, info| unsafe { log_chrm_fp(&e, png, info) },
            )
        });
    }
}

// ===========================================================================
// C6 — png_set_iCCP / png_get_iCCP
// ===========================================================================

/// D50 as an ICC XYZNumber (png.c: `D50_nCIEXYZ`).
const D50: [u8; 12] = [
    0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
];
const ICC_TAGS: &[&[u8; 4]] = &[b"desc", b"wtpt", b"rXYZ", b"gXYZ", b"bXYZ", b"rTRC"];

fn be32(dst: &mut [u8], v: u32) {
    dst[0] = (v >> 24) as u8;
    dst[1] = (v >> 16) as u8;
    dst[2] = (v >> 8) as u8;
    dst[3] = v as u8;
}

/// A synthetic but *valid* ICC profile: 132 byte header, `tags` tag-table
/// entries, correct size field, `acsp` signature, D50 PCS illuminant, valid
/// colour space / PCS / class / rendering intent and 4-byte aligned tag data.
fn icc(seed: u64, total: usize, tags: usize, gray: bool, class: &[u8; 4], major: u8) -> Vec<u8> {
    let table_end = 132 + 12 * tags;
    assert!(total >= table_end && total % 4 == 0);
    let mut rng = Rng::new(seed);
    let mut p = vec![0u8; total];
    for i in table_end..total {
        p[i] = rng.byte();
    }
    be32(&mut p[0..4], total as u32);
    p[4..8].copy_from_slice(b"none");
    p[8] = major;
    p[9] = 0x40;
    p[12..16].copy_from_slice(class);
    p[16..20].copy_from_slice(if gray { b"GRAY" } else { b"RGB " });
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    p[40..44].copy_from_slice(b"APPL");
    be32(&mut p[64..68], 0); /* perceptual */
    p[68..80].copy_from_slice(&D50);
    p[80..84].copy_from_slice(b"none");
    be32(&mut p[128..132], tags as u32);
    if tags > 0 {
        let data_start = (table_end + 3) & !3usize;
        let avail = total.saturating_sub(data_start);
        let chunk = (avail / tags) & !3usize;
        for i in 0..tags {
            let off = if chunk == 0 {
                data_start.min(total)
            } else {
                data_start + i * chunk
            };
            let b = 132 + 12 * i;
            p[b..b + 4].copy_from_slice(ICC_TAGS[i % ICC_TAGS.len()]);
            be32(&mut p[b + 4..b + 8], off as u32);
            be32(&mut p[b + 8..b + 12], chunk as u32);
            if chunk >= 8 && off + 8 <= total {
                p[off..off + 4].copy_from_slice(b"XYZ ");
                be32(&mut p[off + 4..off + 8], 0);
            }
        }
    }
    p
}

#[test]
fn c6_iccp() {
    ensure_libm();
    // (total length, tag count, keyword length, major version)
    let cfgs: &[(usize, usize, usize, u8)] = &[
        (132, 0, 1, 2),
        (136, 0, 3, 2),
        (144, 1, 8, 2),
        (256, 2, 20, 2),
        (300, 3, 40, 2),
        (512, 5, 79, 2),
        (1024, 4, 12, 4),
        (2048, 6, 5, 4),
    ];
    for (i, &(total, tags, klen, major)) in cfgs.iter().enumerate() {
        for &(depth, color) in &[
            (8i32, PNG_COLOR_TYPE_GRAY),
            (8i32, PNG_COLOR_TYPE_RGB),
            (8i32, PNG_COLOR_TYPE_PALETTE),
        ] {
            // only the first three profile shapes get all three colour types,
            // the rest are exercised on RGB only (keeps the test fast)
            if i >= 3 && color != PNG_COLOR_TYPE_RGB {
                continue;
            }
            let gray = color == PNG_COLOR_TYPE_GRAY;
            let prof = icc(0xC600_0000 + i as u64, total, tags, gray, b"mntr", major);
            let kw: String = (0..klen)
                .map(|j| (b'a' + ((j as u8 + i as u8) % 26)) as char)
                .collect();
            let kwc = CString::new(kw.clone()).unwrap();
            let npal: c_int = if color == PNG_COLOR_TYPE_PALETTE {
                16
            } else {
                0
            };
            let mut prng = Rng::new(0xC601_0000 + i as u64);
            let pal = prng.bytes(npal as usize * 3);
            let rows = img(
                0xC602_0000 + i as u64 + color as u64,
                W,
                H,
                depth,
                color,
                npal as u32,
            );
            diff(
                &format!("C6 iCCP len={total} tags={tags} klen={klen} major={major} color={color}"),
                |lib| {
                    rt(lib, W, H, depth, color, &rows, &mut |c, png, info| unsafe {
                        if npal > 0 {
                            (c.set_PLTE)(png, info, pal.as_ptr(), npal);
                        }
                        (c.set_iCCP)(
                            png,
                            info,
                            kwc.as_ptr(),
                            PNG_COMPRESSION_TYPE_BASE,
                            prof.as_ptr(),
                            prof.len() as u32,
                        );
                        log(format!(
                            "set_iCCP klen={klen} plen={} valid={}",
                            prof.len(),
                            (c.get_valid)(png, info, I_iCCP)
                        ));
                    })
                },
            );
        }
    }
    // profile classes other than mntr
    for (i, class) in [b"scnr", b"prtr", b"spac"].iter().enumerate() {
        let prof = icc(0xC680_0000 + i as u64, 256, 2, false, class, 2);
        let kwc = CString::new("classtest").unwrap();
        let rows = img(0xC681_0000 + i as u64, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
        diff(
            &format!("C6 iCCP class={}", String::from_utf8_lossy(*class)),
            |lib| {
                rt(
                    lib,
                    W,
                    H,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    &rows,
                    &mut |c, png, info| unsafe {
                        (c.set_iCCP)(
                            png,
                            info,
                            kwc.as_ptr(),
                            PNG_COMPRESSION_TYPE_BASE,
                            prof.as_ptr(),
                            prof.len() as u32,
                        );
                        log(format!(
                            "set_iCCP valid={}",
                            (c.get_valid)(png, info, I_iCCP)
                        ));
                    },
                )
            },
        );
    }
}

// ===========================================================================
// C7 — png_set_sBIT
// ===========================================================================

const COMBOS: &[(c_int, c_int)] = &[
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

#[test]
fn c7_sbit() {
    ensure_libm();
    for &(color, depth) in COMBOS {
        let maxbits: u8 = if color == PNG_COLOR_TYPE_PALETTE {
            8
        } else {
            depth as u8
        };
        let mut rng = Rng::new(0xC700_0000 + color as u64 * 64 + depth as u64);
        // the maxima, then three random legal combinations
        let mut vals: Vec<PngColor8> = vec![PngColor8 {
            red: maxbits,
            green: maxbits,
            blue: maxbits,
            gray: depth as u8,
            alpha: depth as u8,
        }];
        for _ in 0..3 {
            vals.push(PngColor8 {
                red: 1 + rng.below(maxbits as u32) as u8,
                green: 1 + rng.below(maxbits as u32) as u8,
                blue: 1 + rng.below(maxbits as u32) as u8,
                gray: 1 + rng.below(depth as u32) as u8,
                alpha: 1 + rng.below(depth as u32) as u8,
            });
        }
        let npal: c_int = if color == PNG_COLOR_TYPE_PALETTE {
            1 << depth
        } else {
            0
        };
        let pal = {
            let mut r = Rng::new(0xC701_0000 + depth as u64);
            r.bytes(npal as usize * 3)
        };
        let rows = img(
            0xC702_0000 + color as u64 * 64 + depth as u64,
            W,
            H,
            depth,
            color,
            npal as u32,
        );
        for (i, v) in vals.iter().enumerate() {
            let v = *v;
            diff(
                &format!("C7 sBIT color={color} depth={depth} [{i}] {v:?}"),
                |lib| {
                    rt(lib, W, H, depth, color, &rows, &mut |c, png, info| unsafe {
                        if npal > 0 {
                            (c.set_PLTE)(png, info, pal.as_ptr(), npal);
                        }
                        (c.set_sBIT)(png, info, &v as *const PngColor8 as *const u8);
                        log(format!(
                            "set_sBIT {v:?} valid={}",
                            (c.get_valid)(png, info, I_sBIT)
                        ));
                    })
                },
            );
        }
    }
}

// ===========================================================================
// C8 — png_set_bKGD
// ===========================================================================

#[test]
fn c8_bkgd() {
    ensure_libm();
    // palette index
    for &idx in &[0u8, 1, 7, 15] {
        let pal = {
            let mut r = Rng::new(0xC800_0000 + idx as u64);
            r.bytes(16 * 3)
        };
        let bk = PngColor16 {
            index: idx,
            ..Default::default()
        };
        let rows = img(
            0xC801_0000 + idx as u64,
            W,
            H,
            4,
            PNG_COLOR_TYPE_PALETTE,
            16,
        );
        diff(&format!("C8 bKGD palette idx={idx}"), |lib| {
            rt(
                lib,
                W,
                H,
                4,
                PNG_COLOR_TYPE_PALETTE,
                &rows,
                &mut |c, png, info| unsafe {
                    (c.set_PLTE)(png, info, pal.as_ptr(), 16);
                    (c.set_bKGD)(png, info, &bk as *const PngColor16 as *const u8);
                    log(format!(
                        "set_bKGD idx={idx} valid={}",
                        (c.get_valid)(png, info, I_bKGD)
                    ));
                },
            )
        });
    }
    // grey (colour types 0 and 4), 1..16 bit
    for &(color, depth) in &[
        (PNG_COLOR_TYPE_GRAY, 1i32),
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    ] {
        let maxv: u32 = (1u32 << depth) - 1;
        let mut rng = Rng::new(0xC820_0000 + color as u64 * 32 + depth as u64);
        for k in 0..3 {
            let g = if k == 0 { maxv } else { rng.below(maxv + 1) };
            let bk = PngColor16 {
                gray: g as u16,
                ..Default::default()
            };
            let rows = img(0xC821_0000 + k + depth as u64, W, H, depth, color, 0);
            diff(
                &format!("C8 bKGD gray color={color} depth={depth} g={g}"),
                |lib| {
                    rt(lib, W, H, depth, color, &rows, &mut |c, png, info| unsafe {
                        (c.set_bKGD)(png, info, &bk as *const PngColor16 as *const u8);
                        log(format!(
                            "set_bKGD gray={g} valid={}",
                            (c.get_valid)(png, info, I_bKGD)
                        ));
                    })
                },
            );
        }
    }
    // RGB (colour types 2 and 6), 8 and 16 bit
    for &(color, depth) in &[
        (PNG_COLOR_TYPE_RGB, 8i32),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        let maxv: u32 = if depth == 8 { 255 } else { 65535 };
        let mut rng = Rng::new(0xC840_0000 + color as u64 * 32 + depth as u64);
        for k in 0..3 {
            let bk = PngColor16 {
                red: rng.below(maxv + 1) as u16,
                green: rng.below(maxv + 1) as u16,
                blue: rng.below(maxv + 1) as u16,
                ..Default::default()
            };
            let rows = img(0xC841_0000 + k + depth as u64, W, H, depth, color, 0);
            diff(
                &format!("C8 bKGD rgb color={color} depth={depth} [{k}]"),
                |lib| {
                    rt(lib, W, H, depth, color, &rows, &mut |c, png, info| unsafe {
                        (c.set_bKGD)(png, info, &bk as *const PngColor16 as *const u8);
                        log(format!(
                            "set_bKGD rgb valid={}",
                            (c.get_valid)(png, info, I_bKGD)
                        ));
                    })
                },
            );
        }
    }
}

// ===========================================================================
// C9 — png_set_hIST
// ===========================================================================

#[test]
fn c9_hist() {
    ensure_libm();
    for &(depth, npal) in &[(1i32, 2i32), (2, 4), (4, 16), (8, 2), (8, 16), (8, 256)] {
        for s in 0..3u64 {
            let mut rng = Rng::new(0xC900_0000 + depth as u64 * 1024 + npal as u64 * 8 + s);
            let pal = rng.bytes(npal as usize * 3);
            let hist: Vec<u16> = (0..npal as usize).map(|_| rng.next_u32() as u16).collect();
            let rows = img(
                0xC901_0000 + npal as u64 * 8 + s,
                W,
                H,
                depth,
                PNG_COLOR_TYPE_PALETTE,
                npal as u32,
            );
            diff(&format!("C9 hIST depth={depth} n={npal}[{s}]"), |lib| {
                rt(
                    lib,
                    W,
                    H,
                    depth,
                    PNG_COLOR_TYPE_PALETTE,
                    &rows,
                    &mut |c, png, info| unsafe {
                        (c.set_PLTE)(png, info, pal.as_ptr(), npal);
                        (c.set_hIST)(png, info, hist.as_ptr());
                        log(format!(
                            "set_hIST n={npal} valid={}",
                            (c.get_valid)(png, info, I_hIST)
                        ));
                    },
                )
            });
        }
    }
    // hIST without a preceding PLTE: png_set_hIST refuses it with a warning
    let rows = img(0xC9F0, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
    let hist: Vec<u16> = (0..256).map(|i| (i * 257) as u16).collect();
    diff("C9 hIST without PLTE", |lib| {
        rt(
            lib,
            W,
            H,
            8,
            PNG_COLOR_TYPE_RGB,
            &rows,
            &mut |c, png, info| unsafe {
                (c.set_hIST)(png, info, hist.as_ptr());
                log(format!(
                    "set_hIST valid={}",
                    (c.get_valid)(png, info, I_hIST)
                ));
            },
        )
    });
}

// ===========================================================================
// C10 — png_set_pHYs and every derived getter
// ===========================================================================

#[test]
fn c10_phys() {
    ensure_libm();
    let rows = img(0xCA01, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
    let mut rng = Rng::new(0xCA00_0001);
    let mut cfgs: Vec<(u32, u32, c_int)> = vec![
        (0, 0, 0),
        (0, 100, 1),
        (100, 0, 1),
        (1, 1, 0),
        (2835, 2835, 1),
        (0x7fff_ffff, 0x7fff_ffff, 1),
        (0xffff_ffff, 0xffff_ffff, 1),
        (72, 300, 2),
    ];
    for _ in 0..6 {
        cfgs.push((
            rng.below(2_000_000) + 1,
            rng.below(2_000_000) + 1,
            (rng.below(2)) as c_int,
        ));
    }
    for (i, &(x, y, unit)) in cfgs.iter().enumerate() {
        diff(&format!("C10 pHYs[{i}] x={x} y={y} unit={unit}"), |lib| {
            let e = Ext::new(lib);
            rt_probe(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (c.set_pHYs)(png, info, x, y, unit);
                    log(format!(
                        "set_pHYs valid={}",
                        (c.get_valid)(png, info, I_pHYs)
                    ));
                },
                &mut |_c, png, info| unsafe { log_phys_extra(&e, png, info) },
            )
        });
    }
}

// ===========================================================================
// C11 — png_set_oFFs and every derived getter
// ===========================================================================

#[test]
fn c11_offs() {
    ensure_libm();
    let rows = img(0xCB01, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
    let mut rng = Rng::new(0xCB00_0001);
    let mut cfgs: Vec<(i32, i32, c_int)> = vec![
        (0, 0, 0),
        (1, -1, 0),
        (-1, 1, 1),
        (i32::MAX, i32::MIN, 0),
        (i32::MIN, i32::MAX, 1),
        (-2_000_000, 2_000_000, 1),
        (12345, -54321, 2),
    ];
    for _ in 0..6 {
        let a = rng.next_u32() as i32 / 4;
        let b = -(rng.next_u32() as i32 / 4);
        cfgs.push((a, b, (rng.below(2)) as c_int));
    }
    for (i, &(x, y, unit)) in cfgs.iter().enumerate() {
        diff(&format!("C11 oFFs[{i}] x={x} y={y} unit={unit}"), |lib| {
            let e = Ext::new(lib);
            rt_probe(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (c.set_oFFs)(png, info, x, y, unit);
                    log(format!(
                        "set_oFFs valid={}",
                        (c.get_valid)(png, info, I_oFFs)
                    ));
                },
                &mut |_c, png, info| unsafe { log_offs_extra(&e, png, info) },
            )
        });
    }
}

// ===========================================================================
// C12 — png_set_tIME + png_convert_to_rfc1123_buffer
// ===========================================================================

#[test]
fn c12_time() {
    ensure_libm();
    let rows = img(0xCC01, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
    let mut rng = Rng::new(0xCC00_0001);
    let mut times: Vec<PngTime> = vec![
        PngTime {
            year: 0,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        },
        PngTime {
            year: 9999,
            month: 12,
            day: 31,
            hour: 23,
            minute: 59,
            second: 60,
        },
        PngTime {
            year: 65535,
            month: 6,
            day: 15,
            hour: 12,
            minute: 30,
            second: 30,
        },
        PngTime {
            year: 1995,
            month: 5,
            day: 31,
            hour: 0,
            minute: 0,
            second: 60,
        },
        // rejected by png_set_tIME (month 0) -> warning, chunk not stored
        PngTime {
            year: 2000,
            month: 0,
            day: 1,
            hour: 1,
            minute: 1,
            second: 1,
        },
        // rejected by png_set_tIME (minute 60)
        PngTime {
            year: 2000,
            month: 1,
            day: 1,
            hour: 1,
            minute: 60,
            second: 1,
        },
        // rejected by png_set_tIME (day 32)
        PngTime {
            year: 2000,
            month: 1,
            day: 32,
            hour: 1,
            minute: 1,
            second: 1,
        },
    ];
    for _ in 0..7 {
        times.push(PngTime {
            year: rng.below(10001) as u16,
            month: 1 + rng.below(12) as u8,
            day: 1 + rng.below(31) as u8,
            hour: rng.below(24) as u8,
            minute: rng.below(60) as u8,
            second: rng.below(61) as u8,
        });
    }
    for (i, t) in times.iter().enumerate() {
        let t = *t;
        diff(&format!("C12 tIME[{i}] {t:?}"), |lib| {
            rt_probe(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (c.set_tIME)(png, info, &t as *const PngTime as *const u8);
                    log(format!(
                        "set_tIME valid={}",
                        (c.get_valid)(png, info, I_tIME)
                    ));
                },
                &mut |c, png, info| unsafe { log_rfc1123(c, png, info) },
            )
        });
    }
}

// ===========================================================================
// C13 — png_set_pCAL
// ===========================================================================

const FP_STRINGS: &[&str] = &[
    "1",
    "-2.5",
    "3e5",
    "0.0001",
    "1E-3",
    "+7",
    "0",
    ".5",
    "12.",
    "-0.75e+2",
    "123456789",
    "-1e-10",
];

#[test]
fn c13_pcal() {
    ensure_libm();
    let rows = img(0xCD01, W, H, 8, PNG_COLOR_TYPE_GRAY, 0);
    // purpose and units with 8-bit (>= 0xA1) characters, which png_check_keyword
    // accepts
    let purposes: Vec<CString> = vec![
        CString::new("calibration").unwrap(),
        CString::new(vec![b'p', 0xE9, b'r', b'i', b'o', 0xC5, b'd']).unwrap(),
        CString::new("two words here").unwrap(),
    ];
    let unitss: Vec<CString> = vec![
        CString::new("metres").unwrap(),
        CString::new(vec![0xB5, b'm', b'/', b's', 0xB2]).unwrap(),
        CString::new("").unwrap(),
    ];
    let mut rng = Rng::new(0xCD00_0001);
    // the full grid (the read side rejects the mismatching parameter counts,
    // which is exactly the branch we want covered) plus (3,4) so that the
    // HYPERBOLIC equation also round-trips completely
    let mut grid: Vec<(i32, i32)> = Vec::new();
    for etype in 0..4i32 {
        for nparams in 0..4i32 {
            grid.push((etype, nparams));
        }
    }
    grid.push((3, 4));
    for (etype, nparams) in grid {
        {
            let params: Vec<CString> = (0..nparams as usize)
                .map(|k| {
                    CString::new(
                        FP_STRINGS
                            [(rng.below(FP_STRINGS.len() as u32) as usize + k) % FP_STRINGS.len()],
                    )
                    .unwrap()
                })
                .collect();
            let pptrs: Vec<*mut c_char> =
                params.iter().map(|s| s.as_ptr() as *mut c_char).collect();
            let pi = (etype as usize + nparams as usize) % purposes.len();
            // the empty units string is only used when there is at least one
            // parameter (otherwise png_write_pCAL emits no units terminator)
            let ui = if nparams == 0 {
                etype as usize % 2
            } else {
                (etype as usize * 2 + nparams as usize) % unitss.len()
            };
            let x0 = -(1000 * (etype + 1));
            let x1 = 65535 * (nparams + 1);
            let plist: Vec<String> = params
                .iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            diff(
                &format!("C13 pCAL type={etype} nparams={nparams} params={plist:?}"),
                |lib| {
                    rt(
                        lib,
                        W,
                        H,
                        8,
                        PNG_COLOR_TYPE_GRAY,
                        &rows,
                        &mut |c, png, info| unsafe {
                            (c.set_pCAL)(
                                png,
                                info,
                                purposes[pi].as_ptr(),
                                x0,
                                x1,
                                etype,
                                nparams,
                                unitss[ui].as_ptr(),
                                if nparams == 0 {
                                    std::ptr::null_mut()
                                } else {
                                    pptrs.as_ptr() as *mut *mut c_char
                                },
                            );
                            log(format!(
                                "set_pCAL valid={}",
                                (c.get_valid)(png, info, I_pCAL)
                            ));
                        },
                    )
                },
            );
        }
    }
}

// ===========================================================================
// C14 — png_set_sCAL / _fixed / _s
// ===========================================================================

#[test]
fn c14_scal() {
    ensure_libm();
    let rows = img(0xCE01, W, H, 8, PNG_COLOR_TYPE_GRAY, 0);
    let mut rng = Rng::new(0xCE00_0001);
    for unit in 1..3i32 {
        // double form
        for k in 0..4 {
            let wd = rng.f64() * 1000.0 + 0.001;
            let hd = rng.f64() * 1000.0 + 0.001;
            diff(
                &format!("C14 sCAL fp unit={unit}[{k}] {wd:.9}x{hd:.9}"),
                |lib| {
                    let e = Ext::new(lib);
                    let f = c_set_sCAL(lib);
                    rt_probe(
                        lib,
                        W,
                        H,
                        8,
                        PNG_COLOR_TYPE_GRAY,
                        &rows,
                        &mut |c, png, info| unsafe {
                            f(png, info, unit, wd, hd);
                            log(format!(
                                "set_sCAL valid={}",
                                (c.get_valid)(png, info, I_sCAL)
                            ));
                        },
                        &mut |_c, png, info| unsafe { log_scal_extra(&e, png, info) },
                    )
                },
            );
        }
        // fixed form
        for k in 0..4 {
            let wf = (rng.below(200_000_000) + 1) as i32;
            let hf = (rng.below(200_000_000) + 1) as i32;
            diff(
                &format!("C14 sCAL fixed unit={unit}[{k}] {wf}x{hf}"),
                |lib| {
                    let e = Ext::new(lib);
                    rt_probe(
                        lib,
                        W,
                        H,
                        8,
                        PNG_COLOR_TYPE_GRAY,
                        &rows,
                        &mut |c, png, info| unsafe {
                            (c.set_sCAL_fixed)(png, info, unit, wf, hf);
                            log(format!(
                                "set_sCAL_fixed valid={}",
                                (c.get_valid)(png, info, I_sCAL)
                            ));
                        },
                        &mut |_c, png, info| unsafe { log_scal_extra(&e, png, info) },
                    )
                },
            );
        }
        // string form
        // NOTE: png_get_sCAL_fixed runs the stored string through png_fixed, so
        // the values have to stay inside the fixed-point range (<= 21474.83647)
        // or the getter would png_error.
        for (k, (sw, sh)) in [
            ("1", "1"),
            ("1.5e3", "0.0001"),
            ("42", "0.5"),
            ("12345", "1e-5"),
            ("20000", ".00001"),
        ]
        .iter()
        .enumerate()
        {
            let swc = CString::new(*sw).unwrap();
            let shc = CString::new(*sh).unwrap();
            diff(&format!("C14 sCAL_s unit={unit}[{k}] {sw}x{sh}"), |lib| {
                let e = Ext::new(lib);
                rt_probe(
                    lib,
                    W,
                    H,
                    8,
                    PNG_COLOR_TYPE_GRAY,
                    &rows,
                    &mut |c, png, info| unsafe {
                        (c.set_sCAL_s)(png, info, unit, swc.as_ptr(), shc.as_ptr());
                        log(format!(
                            "set_sCAL_s valid={}",
                            (c.get_valid)(png, info, I_sCAL)
                        ));
                    },
                    &mut |_c, png, info| unsafe { log_scal_extra(&e, png, info) },
                )
            });
        }
    }
}

// ===========================================================================
// C15 — png_set_sPLT / png_get_sPLT
// ===========================================================================

#[test]
fn c15_splt() {
    ensure_libm();
    let rows = img(0xCF01, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
    for npals in 1..4usize {
        for &sdepth in &[8u8, 16u8] {
            for s in 0..3usize {
                let mut rng =
                    Rng::new(0xCF00_0000 + npals as u64 * 64 + sdepth as u64 * 8 + s as u64);
                let names: Vec<CString> = (0..npals)
                    .map(|i| CString::new(format!("suggested pal {i} v{s}")).unwrap())
                    .collect();
                let ents: Vec<Vec<PngSpltEntry>> = (0..npals)
                    .map(|i| {
                        let n = 1 + (i * 5 + npals + s * 3) % 16;
                        (0..n)
                            .map(|_| {
                                if sdepth == 8 {
                                    PngSpltEntry {
                                        red: rng.byte() as u16,
                                        green: rng.byte() as u16,
                                        blue: rng.byte() as u16,
                                        alpha: rng.byte() as u16,
                                        frequency: rng.next_u32() as u16,
                                    }
                                } else {
                                    PngSpltEntry {
                                        red: rng.next_u32() as u16,
                                        green: rng.next_u32() as u16,
                                        blue: rng.next_u32() as u16,
                                        alpha: rng.next_u32() as u16,
                                        frequency: rng.next_u32() as u16,
                                    }
                                }
                            })
                            .collect()
                    })
                    .collect();
                let splt: Vec<PngSpltT> = (0..npals)
                    .map(|i| PngSpltT {
                        name: names[i].as_ptr() as *mut c_char,
                        depth: sdepth,
                        entries: ents[i].as_ptr() as *mut PngSpltEntry,
                        nentries: ents[i].len() as i32,
                    })
                    .collect();
                diff(
                    &format!("C15 sPLT npals={npals} depth={sdepth}[{s}]"),
                    |lib| {
                        rt(
                            lib,
                            W,
                            H,
                            8,
                            PNG_COLOR_TYPE_RGB,
                            &rows,
                            &mut |c, png, info| unsafe {
                                (c.set_sPLT)(
                                    png,
                                    info,
                                    splt.as_ptr() as *const c_void,
                                    npals as c_int,
                                );
                                log(format!(
                                    "set_sPLT npals={npals} valid={}",
                                    (c.get_valid)(png, info, I_sPLT)
                                ));
                            },
                        )
                    },
                );
            }
        }
    }
}

// ===========================================================================
// C16 — png_set_eXIf_1 / png_get_eXIf_1 (+ the deprecated png_set_eXIf)
// ===========================================================================

#[test]
fn c16_exif() {
    ensure_libm();
    let rows = img(0xD001, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
    for (bi, bom) in [[0x49u8, 0x49, 0x2A, 0x00], [0x4D, 0x4D, 0x00, 0x2A]]
        .iter()
        .enumerate()
    {
        for &len in &[4usize, 5, 8, 33, 128, 1024] {
            let mut rng = Rng::new(0xD000_0000 + bi as u64 * 4096 + len as u64);
            let mut blob = bom.to_vec();
            while blob.len() < len {
                blob.push(rng.byte());
            }
            blob.truncate(len.max(4));
            diff(&format!("C16 eXIf bom={bi} len={}", blob.len()), |lib| {
                let e = Ext::new(lib);
                rt_probe(
                    lib,
                    W,
                    H,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    &rows,
                    &mut |c, png, info| unsafe {
                        (c.set_eXIf_1)(png, info, blob.len() as u32, blob.as_ptr());
                        log(format!(
                            "set_eXIf_1 len={} valid={}",
                            blob.len(),
                            (c.get_valid)(png, info, I_eXIf)
                        ));
                    },
                    &mut |_c, png, info| unsafe { log_exif_old(&e, png, info) },
                )
            });
        }
    }
    // the deprecated entry point: documented (pngset.c) to do nothing except
    // warn
    let mut blob: Vec<u8> = vec![0x49, 0x49, 0x2A, 0x00, 1, 2, 3, 4];
    diff("C16 eXIf deprecated setter", |lib| {
        let e = Ext::new(lib);
        rt_probe(
            lib,
            W,
            H,
            8,
            PNG_COLOR_TYPE_RGB,
            &rows,
            &mut |c, png, info| unsafe {
                (e.set_eXIf)(png, info, blob.as_mut_ptr());
                log(format!(
                    "set_eXIf valid={}",
                    (c.get_valid)(png, info, I_eXIf)
                ));
            },
            &mut |_c, png, info| unsafe { log_exif_old(&e, png, info) },
        )
    });
}

// ===========================================================================
// C17 — png_set_cICP
// ===========================================================================

#[test]
fn c17_cicp() {
    ensure_libm();
    let rows = img(0xD101, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
    let mut rng = Rng::new(0xD100_0001);
    let mut cfgs: Vec<(u8, u8, u8, u8)> = vec![
        (1, 13, 0, 1),
        (9, 16, 0, 0),
        (0, 0, 0, 0),
        (255, 255, 0, 255),
        // matrix coefficients != 0 are rejected by png_set_cICP with a warning
        (9, 16, 1, 1),
        (2, 2, 255, 0),
    ];
    for _ in 0..6 {
        cfgs.push((rng.byte(), rng.byte(), 0, rng.byte() & 1));
    }
    for (i, &(p, t, m, f)) in cfgs.iter().enumerate() {
        diff(&format!("C17 cICP[{i}] p={p} t={t} m={m} f={f}"), |lib| {
            rt(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (c.set_cICP)(png, info, p, t, m, f);
                    log(format!(
                        "set_cICP valid={}",
                        (c.get_valid)(png, info, I_cICP)
                    ));
                },
            )
        });
    }
}

// ===========================================================================
// C18 — png_set_cLLI / png_set_cLLI_fixed
// ===========================================================================

#[test]
fn c18_clli() {
    ensure_libm();
    let rows = img(0xD201, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
    let mut rng = Rng::new(0xD200_0001);
    let mut fixed: Vec<(u32, u32)> = vec![
        (0, 0),
        (1, 0),
        (0, 1),
        (10_000, 5_000),
        (0x7fff_ffff, 0x7fff_ffff),
        (2_000_0000, 100),
    ];
    for _ in 0..6 {
        fixed.push((rng.below(50_000_000), rng.below(50_000_000)));
    }
    for (i, &(a, b)) in fixed.iter().enumerate() {
        diff(&format!("C18 cLLI_fixed[{i}] {a},{b}"), |lib| {
            let e = Ext::new(lib);
            rt_probe(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (c.set_cLLI_fixed)(png, info, a, b);
                    log(format!(
                        "set_cLLI_fixed valid={}",
                        (c.get_valid)(png, info, I_cLLI)
                    ));
                },
                &mut |_c, png, info| unsafe { log_cLLI_fp(&e, png, info) },
            )
        });
    }
    let mut rng = Rng::new(0xD200_0002);
    let mut dbl: Vec<(f64, f64)> = vec![(0.0, 0.0), (1000.0, 400.0), (0.0001, 0.0)];
    for _ in 0..5 {
        dbl.push((rng.f64() * 4000.0, rng.f64() * 400.0));
    }
    for (i, &(a, b)) in dbl.iter().enumerate() {
        diff(&format!("C18 cLLI_fp[{i}] {a:.6},{b:.6}"), |lib| {
            let e = Ext::new(lib);
            rt_probe(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (e.set_cLLI)(png, info, a, b);
                    log(format!(
                        "set_cLLI valid={}",
                        (c.get_valid)(png, info, I_cLLI)
                    ));
                },
                &mut |_c, png, info| unsafe { log_cLLI_fp(&e, png, info) },
            )
        });
    }
}

// ===========================================================================
// C19 — png_set_mDCV / png_set_mDCV_fixed
// ===========================================================================

#[test]
fn c19_mdcv() {
    ensure_libm();
    let rows = img(0xD301, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
    let mut rng = Rng::new(0xD300_0001);
    let mut cfgs: Vec<([i32; 8], u32, u32)> = vec![
        (SRGB_XY, 10_000_000, 500),
        ([0, 0, 0, 0, 0, 0, 0, 0], 0, 0),
        (
            [131071, 131070, 131069, 2, 1, 0, 65535, 65534],
            0x7fff_ffff,
            0x7fff_ffff,
        ),
        ([1, 3, 5, 7, 9, 11, 13, 15], 1, 1),
    ];
    for _ in 0..6 {
        let mut v = [0i32; 8];
        for x in v.iter_mut() {
            *x = rng.below(131_072) as i32;
        }
        cfgs.push((v, rng.below(100_000_000), rng.below(100_000)));
    }
    for (i, &(v, maxdl, mindl)) in cfgs.iter().enumerate() {
        diff(
            &format!("C19 mDCV_fixed[{i}] {v:?} {maxdl} {mindl}"),
            |lib| {
                let e = Ext::new(lib);
                rt_probe(
                    lib,
                    W,
                    H,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    &rows,
                    &mut |c, png, info| unsafe {
                        (e.set_mDCV_fx)(
                            png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], maxdl, mindl,
                        );
                        log(format!(
                            "set_mDCV_fixed valid={}",
                            (c.get_valid)(png, info, I_mDCV)
                        ));
                    },
                    &mut |_c, png, info| unsafe { log_mDCV_fp(&e, png, info) },
                )
            },
        );
    }
    let mut rng = Rng::new(0xD300_0002);
    let mut dbl: Vec<([f64; 8], f64, f64)> = vec![
        (
            [0.3127, 0.3290, 0.640, 0.330, 0.300, 0.600, 0.150, 0.060],
            1000.0,
            0.05,
        ),
        ([0.0; 8], 0.0, 0.0),
    ];
    for _ in 0..5 {
        let mut v = [0f64; 8];
        for x in v.iter_mut() {
            *x = rng.f64() * 1.3;
        }
        dbl.push((v, rng.f64() * 4000.0, rng.f64() * 10.0));
    }
    for (i, &(v, maxdl, mindl)) in dbl.iter().enumerate() {
        diff(&format!("C19 mDCV_fp[{i}]"), |lib| {
            let e = Ext::new(lib);
            rt_probe(
                lib,
                W,
                H,
                8,
                PNG_COLOR_TYPE_RGB,
                &rows,
                &mut |c, png, info| unsafe {
                    (e.set_mDCV)(
                        png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], maxdl, mindl,
                    );
                    log(format!(
                        "set_mDCV valid={}",
                        (c.get_valid)(png, info, I_mDCV)
                    ));
                },
                &mut |_c, png, info| unsafe { log_mDCV_fp(&e, png, info) },
            )
        });
    }
}

// ===========================================================================
// C20 — png_set_text / png_get_text
// ===========================================================================

struct TextBag {
    _keys: Vec<CString>,
    _texts: Vec<CString>,
    _langs: Vec<CString>,
    _lkeys: Vec<CString>,
    items: Vec<PngText>,
}

fn build_texts(seed: u64, n: usize) -> TextBag {
    let mut rng = Rng::new(seed);
    let comps = [
        PNG_TEXT_COMPRESSION_NONE,
        PNG_TEXT_COMPRESSION_zTXt,
        PNG_ITXT_COMPRESSION_NONE,
        PNG_ITXT_COMPRESSION_zTXt,
    ];
    let mut keys = Vec::new();
    let mut texts = Vec::new();
    let mut langs = Vec::new();
    let mut lkeys = Vec::new();
    for i in 0..n {
        keys.push(CString::new(format!("Key{i}")).unwrap());
        // mix: empty text, 8-bit characters, plain ASCII
        let t: Vec<u8> = match i % 3 {
            0 => Vec::new(),
            1 => vec![b'h', b'i', 0xE9, 0xFF, b'!', b'\n', b'x'],
            _ => (0..8 + (i % 5)).map(|_| b'a' + (rng.byte() % 26)).collect(),
        };
        texts.push(CString::new(t).unwrap());
        langs.push(CString::new(if i % 2 == 0 { "en-GB" } else { "de" }).unwrap());
        lkeys.push(CString::new(format!("Schl{i}")).unwrap());
    }
    let mut items = Vec::new();
    for i in 0..n {
        let comp = comps[i % comps.len()];
        let itxt = comp > 0;
        items.push(PngText {
            compression: comp,
            key: keys[i].as_ptr() as *mut c_char,
            // every third entry has a NULL text pointer
            text: if i % 3 == 0 && i > 0 {
                std::ptr::null_mut()
            } else {
                texts[i].as_ptr() as *mut c_char
            },
            text_length: 0,
            itxt_length: 0,
            lang: if itxt && i % 4 != 3 {
                langs[i].as_ptr() as *mut c_char
            } else {
                std::ptr::null_mut()
            },
            lang_key: if itxt && i % 5 != 4 {
                lkeys[i].as_ptr() as *mut c_char
            } else {
                std::ptr::null_mut()
            },
        });
    }
    TextBag {
        _keys: keys,
        _texts: texts,
        _langs: langs,
        _lkeys: lkeys,
        items,
    }
}

#[test]
fn c20_text() {
    ensure_libm();
    let rows = img(0xD401, W, H, 8, PNG_COLOR_TYPE_RGB, 0);
    for n in 1..9usize {
        for s in 0..3u64 {
            let bag = build_texts(0xD400_0000 + n as u64 * 16 + s, n);
            diff(&format!("C20 text n={n}[{s}]"), |lib| {
                rt(
                    lib,
                    W,
                    H,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    &rows,
                    &mut |c, png, info| unsafe {
                        (c.set_text)(png, info, bag.items.as_ptr() as *const c_void, n as c_int);
                        let mut tp: *mut c_void = std::ptr::null_mut();
                        let mut nt: c_int = -1;
                        let r = (c.get_text)(png, info, &mut tp, &mut nt);
                        log(format!("set_text n={n} get_text={r} num={nt}"));
                    },
                )
            });
        }
    }
    // two png_set_text calls in a row (the array grows in units of 8)
    let b1 = build_texts(0xD40F_0001, 5);
    let b2 = build_texts(0xD40F_0002, 6);
    diff("C20 text two calls", |lib| {
        rt(
            lib,
            W,
            H,
            8,
            PNG_COLOR_TYPE_RGB,
            &rows,
            &mut |c, png, info| unsafe {
                (c.set_text)(png, info, b1.items.as_ptr() as *const c_void, 5);
                (c.set_text)(png, info, b2.items.as_ptr() as *const c_void, 6);
                let mut tp: *mut c_void = std::ptr::null_mut();
                let mut nt: c_int = -1;
                let r = (c.get_text)(png, info, &mut tp, &mut nt);
                log(format!("get_text={r} num={nt}"));
            },
        )
    });
}

// ===========================================================================
// C21 — unknown chunks
// ===========================================================================

const UNK_NAMES: &[&[u8; 4]] = &[b"prVt", b"teSt", b"abCd", b"xyZw"];

#[test]
fn c21_unknown() {
    ensure_libm();
    let locs = [LOC_HAVE_IHDR, LOC_HAVE_PLTE, LOC_AFTER_IDAT];
    for n in 1..5usize {
        for (li, &loc) in locs.iter().enumerate() {
            for s in 0..2u64 {
                let mut rng = Rng::new(0xD500_0000 + n as u64 * 64 + li as u64 * 8 + s);
                // chunk 0 always has zero-length data
                let datas: Vec<Vec<u8>> = (0..n)
                    .map(|i| {
                        if i == 0 {
                            Vec::new()
                        } else {
                            rng.bytes(1 + (i * 7 + s as usize * 3) % 23)
                        }
                    })
                    .collect();
                let unk: Vec<PngUnknownChunk> = (0..n)
                    .map(|i| {
                        let nm = UNK_NAMES[i % UNK_NAMES.len()];
                        let mut name = [0u8; 5];
                        name[..4].copy_from_slice(nm);
                        PngUnknownChunk {
                            name,
                            data: if datas[i].is_empty() {
                                std::ptr::null_mut()
                            } else {
                                datas[i].as_ptr() as *mut u8
                            },
                            size: datas[i].len(),
                            // give every chunk the same starting location, then move
                            // some of them with png_set_unknown_chunk_location
                            location: locs[i % locs.len()] as u8,
                        }
                    })
                    .collect();
                let urows = img(
                    0xD501_0000 + n as u64 * 8 + s,
                    W,
                    H,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    0,
                );
                diff(&format!("C21 unknown n={n} loc={loc}[{s}]"), |lib| {
                    let mut noprobe = |_: &Core, _: Png, _: Info| {};
                    roundtrip(
                        lib,
                        W,
                        H,
                        8,
                        PNG_COLOR_TYPE_RGB,
                        &urows,
                        &mut |c, png, info| unsafe {
                            (c.set_unknown_chunks)(
                                png,
                                info,
                                unk.as_ptr() as *const c_void,
                                n as c_int,
                            );
                            let mut up: *mut c_void = std::ptr::null_mut();
                            let got = (c.get_unknown_chunks)(png, info, &mut up);
                            log(format!("set_unknown_chunks n={n} got={got}"));
                            // relocate the last chunk
                            (c.set_unknown_chunk_location)(png, info, (n - 1) as c_int, loc);
                            log(format!("relocated {} -> {loc}", n - 1));
                        },
                        &mut noprobe,
                        &mut |c, png, _info| unsafe {
                            (c.set_keep_unknown_chunks)(
                                png,
                                PNG_HANDLE_CHUNK_ALWAYS,
                                std::ptr::null(),
                                0,
                            );
                            for nm in UNK_NAMES {
                                log(format!(
                                    "handle_as_unknown({})={}",
                                    String::from_utf8_lossy(*nm),
                                    (c.handle_as_unknown)(png, nm.as_ptr())
                                ));
                            }
                        },
                    )
                });
            }
        }
    }
}

// ===========================================================================
// C22 — png_set_rows / png_get_rows + png_write_png / png_read_png
// ===========================================================================

#[test]
fn c22_rows_png() {
    ensure_libm();
    for &(color, depth) in COMBOS {
        for &w in &[1u32, 8, 9] {
            let npal: c_int = if color == PNG_COLOR_TYPE_PALETTE {
                1 << depth
            } else {
                0
            };
            let pal = {
                let mut r = Rng::new(0xD600_0000 + depth as u64 * 64 + w as u64);
                r.bytes(npal as usize * 3)
            };
            let data = img(
                0xD601_0000 + depth as u64 * 64 + w as u64,
                w,
                H,
                depth,
                color,
                npal as u32,
            );
            let st = stride_of(color, depth, w);
            diff(
                &format!("C22 write_png/read_png depth={depth} color={color} w={w}"),
                |lib| {
                    let mut wdata = data.clone();
                    let mut rowp: Vec<*mut u8> = (0..H as usize)
                        .map(|y| unsafe { wdata.as_mut_ptr().add(y * st) })
                        .collect();
                    let t1 = with_write(lib, &mut |c, png, info| unsafe {
                        (c.set_IHDR)(
                            png,
                            info,
                            w,
                            H,
                            depth,
                            color,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        if npal > 0 {
                            (c.set_PLTE)(png, info, pal.as_ptr(), npal);
                        }
                        (c.set_rows)(png, info, rowp.as_mut_ptr());
                        log(format!(
                            "W get_rows_null={} IDAT={}",
                            (c.get_rows)(png, info).is_null(),
                            (c.get_valid)(png, info, I_IDAT)
                        ));
                        (c.write_png)(png, info, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut());
                        log("W after write_png");
                        log_valid(c, png, info);
                        (c.free_data)(png, info, F_ROWS, -1);
                        log(format!(
                            "W after free_data rows_null={} IDAT={}",
                            (c.get_rows)(png, info).is_null(),
                            (c.get_valid)(png, info, I_IDAT)
                        ));
                    });
                    let produced = t1.out.clone();
                    let mut lines = t1.lines.clone();
                    let t2 = with_read(lib, &produced, &mut |c, png, info| unsafe {
                        // png_read_png allocates the row buffers itself with png_malloc.
                        // png_combine_row preserves the *padding* bits of the last byte
                        // of a sub-byte row from whatever the destination happened to
                        // contain, so route the allocations through the harness'
                        // zero-filling allocator to keep those bits well defined.
                        (c.set_mem_fn)(png, std::ptr::null_mut(), cb_malloc as Cb, cb_free as Cb);
                        (c.read_png)(png, info, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut());
                        log("R after read_png");
                        log_valid(c, png, info);
                        log_all_info(c, png, info);
                        let rp = (c.get_rows)(png, info);
                        log(format!("R rows_null={}", rp.is_null()));
                        if !rp.is_null() {
                            let n = (c.get_rowbytes)(png, info);
                            for y in 0..H as usize {
                                let r = *rp.add(y);
                                if r.is_null() {
                                    log(format!("R row{y}=<null>"));
                                } else {
                                    log(format!(
                                        "R row{y}={}",
                                        hex(std::slice::from_raw_parts(r, n))
                                    ));
                                }
                            }
                        }
                        (c.free_data)(png, info, F_ROWS, -1);
                        log(format!(
                            "R after free_data rows_null={} IDAT={}",
                            (c.get_rows)(png, info).is_null(),
                            (c.get_valid)(png, info, I_IDAT)
                        ));
                    });
                    lines.extend(t2.lines);
                    Trace {
                        lines,
                        out: produced,
                        rc: t1.rc | (t2.rc << 8),
                    }
                },
            );
        }
    }
}

// ===========================================================================
// "set every chunk" helper, shared by C23 and C24
// ===========================================================================

struct AllBag {
    pal: Vec<u8>,
    npal: c_int,
    trns: Vec<u8>,
    trns_color: PngColor16,
    hist: Vec<u16>,
    sbit: PngColor8,
    bkgd: PngColor16,
    time: PngTime,
    iccp_name: CString,
    iccp: Vec<u8>,
    pcal_purpose: CString,
    pcal_units: CString,
    _pcal_params: Vec<CString>,
    pcal_ptrs: Vec<*mut c_char>,
    scal_w: CString,
    scal_h: CString,
    _splt_names: Vec<CString>,
    _splt_entries: Vec<Vec<PngSpltEntry>>,
    splt: Vec<PngSpltT>,
    exif: Vec<u8>,
    texts: TextBag,
    _unk_data: Vec<Vec<u8>>,
    unk: Vec<PngUnknownChunk>,
    rows: Vec<u8>,
}

fn build_all(seed: u64, depth: c_int, color: c_int) -> AllBag {
    let mut rng = Rng::new(seed);
    let npal: c_int = if color == PNG_COLOR_TYPE_PALETTE {
        1 << depth
    } else {
        16
    };
    let pal = rng.bytes(npal as usize * 3);
    let trns = rng.bytes(npal as usize);
    let maxs: u32 = if depth >= 16 { 65535 } else { (1 << depth) - 1 };
    let trns_color = PngColor16 {
        red: rng.below(maxs + 1) as u16,
        green: rng.below(maxs + 1) as u16,
        blue: rng.below(maxs + 1) as u16,
        gray: rng.below(maxs + 1) as u16,
        index: 0,
    };
    let hist: Vec<u16> = (0..npal as usize).map(|_| rng.next_u32() as u16).collect();
    let maxbits: u8 = if color == PNG_COLOR_TYPE_PALETTE {
        8
    } else {
        depth as u8
    };
    let sbit = PngColor8 {
        red: maxbits,
        green: maxbits,
        blue: maxbits,
        gray: depth as u8,
        alpha: depth as u8,
    };
    let bkgd = if color == PNG_COLOR_TYPE_PALETTE {
        PngColor16 {
            index: (npal - 1) as u8,
            ..Default::default()
        }
    } else {
        PngColor16 {
            red: rng.below(maxs + 1) as u16,
            green: rng.below(maxs + 1) as u16,
            blue: rng.below(maxs + 1) as u16,
            gray: rng.below(maxs + 1) as u16,
            index: 0,
        }
    };
    let time = PngTime {
        year: 2024,
        month: 7,
        day: 4,
        hour: 13,
        minute: 45,
        second: 59,
    };
    let gray_icc = (color & 2) == 0;
    let iccp = icc(seed ^ 0x5EED, 300, 3, gray_icc, b"mntr", 2);
    let params: Vec<CString> = vec![
        CString::new("1.5").unwrap(),
        CString::new("-2e3").unwrap(),
        CString::new("0").unwrap(),
    ];
    let pcal_ptrs: Vec<*mut c_char> = params.iter().map(|s| s.as_ptr() as *mut c_char).collect();
    let splt_names: Vec<CString> = (0..2)
        .map(|i| CString::new(format!("all chunks pal {i}")).unwrap())
        .collect();
    let splt_entries: Vec<Vec<PngSpltEntry>> = (0..2)
        .map(|i| {
            (0..(3 + i * 4))
                .map(|_| PngSpltEntry {
                    red: rng.next_u32() as u16,
                    green: rng.next_u32() as u16,
                    blue: rng.next_u32() as u16,
                    alpha: rng.next_u32() as u16,
                    frequency: rng.next_u32() as u16,
                })
                .collect()
        })
        .collect();
    let splt: Vec<PngSpltT> = (0..2)
        .map(|i| PngSpltT {
            name: splt_names[i].as_ptr() as *mut c_char,
            depth: if i == 0 { 8 } else { 16 },
            entries: splt_entries[i].as_ptr() as *mut PngSpltEntry,
            nentries: splt_entries[i].len() as i32,
        })
        .collect();
    let mut exif: Vec<u8> = vec![0x4D, 0x4D, 0x00, 0x2A];
    exif.extend(rng.bytes(60));
    let texts = build_texts(seed ^ 0x7E77, 5);
    let unk_data: Vec<Vec<u8>> = vec![Vec::new(), rng.bytes(9), rng.bytes(4)];
    let locs = [LOC_HAVE_IHDR, LOC_HAVE_PLTE, LOC_AFTER_IDAT];
    let unk: Vec<PngUnknownChunk> = (0..3)
        .map(|i| {
            let mut name = [0u8; 5];
            name[..4].copy_from_slice(UNK_NAMES[i]);
            PngUnknownChunk {
                name,
                data: if unk_data[i].is_empty() {
                    std::ptr::null_mut()
                } else {
                    unk_data[i].as_ptr() as *mut u8
                },
                size: unk_data[i].len(),
                location: locs[i] as u8,
            }
        })
        .collect();
    let rows = img(seed ^ 0x524F_5753, W, H, depth, color, npal as u32);
    AllBag {
        pal,
        npal,
        trns,
        trns_color,
        hist,
        sbit,
        bkgd,
        time,
        iccp_name: CString::new("embedded profile").unwrap(),
        iccp,
        pcal_purpose: CString::new("everything").unwrap(),
        pcal_units: CString::new("units").unwrap(),
        _pcal_params: params,
        pcal_ptrs,
        scal_w: CString::new("2.5e1").unwrap(),
        scal_h: CString::new("0.125").unwrap(),
        _splt_names: splt_names,
        _splt_entries: splt_entries,
        splt,
        exif,
        texts,
        _unk_data: unk_data,
        unk,
        rows,
    }
}

/// Apply every ancillary chunk setter libpng supports.
unsafe fn set_everything(
    c: &Core,
    e: &Ext,
    png: Png,
    info: Info,
    b: &AllBag,
    color: c_int,
    use_iccp: bool,
    use_srgb: bool,
) {
    (c.set_PLTE)(png, info, b.pal.as_ptr(), b.npal);
    if color == PNG_COLOR_TYPE_PALETTE {
        (c.set_tRNS)(png, info, b.trns.as_ptr(), b.npal, std::ptr::null());
    } else if color == PNG_COLOR_TYPE_GRAY || color == PNG_COLOR_TYPE_RGB {
        (c.set_tRNS)(
            png,
            info,
            std::ptr::null(),
            0,
            &b.trns_color as *const PngColor16 as *const u8,
        );
    }
    (c.set_hIST)(png, info, b.hist.as_ptr());
    (c.set_sBIT)(png, info, &b.sbit as *const PngColor8 as *const u8);
    (c.set_bKGD)(png, info, &b.bkgd as *const PngColor16 as *const u8);
    (c.set_gAMA_fixed)(png, info, 45455);
    (c.set_cHRM_fixed)(
        png, info, SRGB_XY[0], SRGB_XY[1], SRGB_XY[2], SRGB_XY[3], SRGB_XY[4], SRGB_XY[5],
        SRGB_XY[6], SRGB_XY[7],
    );
    if use_srgb {
        (c.set_sRGB)(png, info, PNG_sRGB_INTENT_RELATIVE);
    }
    if use_iccp {
        (c.set_iCCP)(
            png,
            info,
            b.iccp_name.as_ptr(),
            PNG_COMPRESSION_TYPE_BASE,
            b.iccp.as_ptr(),
            b.iccp.len() as u32,
        );
    }
    (c.set_cICP)(png, info, 9, 16, 0, 1);
    (c.set_cLLI_fixed)(png, info, 10_000_000, 4_000_000);
    (e.set_mDCV_fx)(
        png, info, SRGB_XY[0], SRGB_XY[1], SRGB_XY[2], SRGB_XY[3], SRGB_XY[4], SRGB_XY[5],
        SRGB_XY[6], SRGB_XY[7], 10_000_000, 500,
    );
    (c.set_pHYs)(png, info, 2835, 2836, PNG_RESOLUTION_METER);
    (c.set_oFFs)(png, info, -17, 4242, PNG_OFFSET_MICROMETER);
    (c.set_tIME)(png, info, &b.time as *const PngTime as *const u8);
    (c.set_pCAL)(
        png,
        info,
        b.pcal_purpose.as_ptr(),
        -1000,
        250000,
        PNG_EQUATION_ARBITRARY,
        3,
        b.pcal_units.as_ptr(),
        b.pcal_ptrs.as_ptr() as *mut *mut c_char,
    );
    (c.set_sCAL_s)(
        png,
        info,
        PNG_SCALE_METER,
        b.scal_w.as_ptr(),
        b.scal_h.as_ptr(),
    );
    (c.set_sPLT)(png, info, b.splt.as_ptr() as *const c_void, 2);
    (c.set_eXIf_1)(png, info, b.exif.len() as u32, b.exif.as_ptr());
    (c.set_text)(
        png,
        info,
        b.texts.items.as_ptr() as *const c_void,
        b.texts.items.len() as c_int,
    );
    (c.set_unknown_chunks)(png, info, b.unk.as_ptr() as *const c_void, 3);
    let _ = e;
}

fn full_chunks(
    lib: &Lib,
    depth: c_int,
    color: c_int,
    invalidate: Option<u32>,
    use_iccp: bool,
    use_srgb: bool,
    bag: &AllBag,
) -> Trace {
    let e = Ext::new(lib);
    let st = stride_of(color, depth, W);
    let t1 = with_write(lib, &mut |c, png, info| unsafe {
        (c.set_IHDR)(
            png,
            info,
            W,
            H,
            depth,
            color,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        set_everything(c, &e, png, info, bag, color, use_iccp, use_srgb);
        log("--- W all set ---");
        log_valid(c, png, info);
        if let Some(f) = invalidate {
            (c.set_invalid)(png, info, f as c_int);
            log(format!("--- W invalidated {f:#x} ---"));
            log_valid(c, png, info);
        }
        log_all_info(c, png, info);
        log_chrm_fp(&e, png, info);
        log_phys_extra(&e, png, info);
        log_offs_extra(&e, png, info);
        log_scal_extra(&e, png, info);
        log_cLLI_fp(&e, png, info);
        log_mDCV_fp(&e, png, info);
        log_rfc1123(c, png, info);
        (c.write_info)(png, info);
        for y in 0..H as usize {
            (c.write_row)(png, bag.rows.as_ptr().add(y * st));
        }
        (c.write_end)(png, info);
        log("--- W after write_end ---");
        log_valid(c, png, info);
        log_all_info(c, png, info);
    });
    let produced = t1.out.clone();
    let mut lines = t1.lines.clone();
    let mut buf: Vec<u8> = vec![0u8; st + 32];
    let t2 = with_read(lib, &produced, &mut |c, png, info| unsafe {
        (c.set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
        (c.read_info)(png, info);
        log("--- R after read_info ---");
        log_valid(c, png, info);
        log_all_info(c, png, info);
        log_chrm_fp(&e, png, info);
        log_phys_extra(&e, png, info);
        log_offs_extra(&e, png, info);
        log_scal_extra(&e, png, info);
        log_cLLI_fp(&e, png, info);
        log_mDCV_fp(&e, png, info);
        log_rfc1123(c, png, info);
        let n = (c.get_rowbytes)(png, info).min(buf.len());
        for y in 0..H {
            (c.read_row)(png, buf.as_mut_ptr(), std::ptr::null_mut());
            log(format!("row{y}={}", hex(&buf[..n])));
        }
        // read_end into a *separate* end-info struct.  png_get_IHDR (used by
        // log_all_info) re-validates the header, so the end-info struct is
        // given a valid one first; it has no influence on chunk handling.
        let end = (c.create_info)(png);
        log(format!("end_info={}", !end.is_null()));
        (c.set_IHDR)(
            png,
            end,
            W,
            H,
            depth,
            color,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        (c.read_end)(png, end);
        log("--- R end info ---");
        log_valid(c, png, end);
        log_all_info(c, png, end);
        log("--- R main info after read_end ---");
        log_valid(c, png, info);
        log_all_info(c, png, info);
        let mut endp = end;
        (c.destroy_info)(png, &mut endp);
        log(format!("end_destroyed={}", endp.is_null()));
    });
    lines.extend(t2.lines);
    Trace {
        lines,
        out: produced,
        rc: t1.rc | (t2.rc << 8),
    }
}

// ===========================================================================
// C23 — png_set_invalid + png_get_valid
// ===========================================================================

#[test]
fn c23_set_invalid() {
    ensure_libm();
    let bag = build_all(0xD700_0001, 8, PNG_COLOR_TYPE_RGB);
    // baseline: nothing invalidated
    diff("C23 baseline", |lib| {
        full_chunks(lib, 8, PNG_COLOR_TYPE_RGB, None, true, true, &bag)
    });
    for &(name, flag) in VFLAGS {
        diff(&format!("C23 invalidate {name}"), |lib| {
            full_chunks(lib, 8, PNG_COLOR_TYPE_RGB, Some(flag), true, true, &bag)
        });
    }
    // several flags at once
    for &mask in &[
        I_gAMA | I_cHRM | I_sRGB | I_iCCP,
        I_PLTE | I_tRNS | I_bKGD | I_hIST,
        I_pHYs | I_oFFs | I_tIME | I_pCAL | I_sCAL,
        I_cICP | I_cLLI | I_mDCV | I_eXIf | I_sPLT,
        0xFFFFF,
    ] {
        diff(&format!("C23 invalidate mask={mask:#x}"), |lib| {
            full_chunks(lib, 8, PNG_COLOR_TYPE_RGB, Some(mask), true, true, &bag)
        });
    }
}

// ===========================================================================
// C24 — one image carrying every supported ancillary chunk
// ===========================================================================

#[test]
fn c24_everything() {
    ensure_libm();
    // palette image, 8 bit
    let pbag = build_all(0xD800_0001, 8, PNG_COLOR_TYPE_PALETTE);
    diff("C24 palette8 iCCP+sRGB", |lib| {
        full_chunks(lib, 8, PNG_COLOR_TYPE_PALETTE, None, true, true, &pbag)
    });
    diff("C24 palette8 sRGB only", |lib| {
        full_chunks(lib, 8, PNG_COLOR_TYPE_PALETTE, None, false, true, &pbag)
    });
    diff("C24 palette8 iCCP only", |lib| {
        full_chunks(lib, 8, PNG_COLOR_TYPE_PALETTE, None, true, false, &pbag)
    });
    // RGBA, 16 bit (tRNS is not legal with an alpha channel, so it is omitted)
    let rbag = build_all(0xD800_0002, 16, PNG_COLOR_TYPE_RGB_ALPHA);
    diff("C24 rgba16 iCCP+sRGB", |lib| {
        full_chunks(lib, 16, PNG_COLOR_TYPE_RGB_ALPHA, None, true, true, &rbag)
    });
    diff("C24 rgba16 sRGB only", |lib| {
        full_chunks(lib, 16, PNG_COLOR_TYPE_RGB_ALPHA, None, false, true, &rbag)
    });
    diff("C24 rgba16 iCCP only", |lib| {
        full_chunks(lib, 16, PNG_COLOR_TYPE_RGB_ALPHA, None, true, false, &rbag)
    });
    // greyscale 16 bit for the GRAY ICC colour space path
    let gbag = build_all(0xD800_0003, 16, PNG_COLOR_TYPE_GRAY);
    diff("C24 gray16 iCCP+sRGB", |lib| {
        full_chunks(lib, 16, PNG_COLOR_TYPE_GRAY, None, true, true, &gbag)
    });
}
