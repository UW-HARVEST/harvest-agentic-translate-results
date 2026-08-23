//! Sequential-read differential tests, CONFIGS.md rows R17..R33.
//!
//! Every test builds its input PNG with `support::pngbuild` (independent of
//! both libraries), then drives the C `.so` and the Rust `.so` through the
//! identical call sequence on those identical bytes and compares the complete
//! trace byte for byte.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod support;

use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void};
use support::core::*;
use support::pngbuild::{self, Builder, Chunk};
use support::*;

// ---------------------------------------------------------------------------
// input construction
// ---------------------------------------------------------------------------

/// The 15 legal (colour_type, bit_depth) combinations.
const COMBOS: &[(u8, u8)] = &[
    (0, 1),
    (0, 2),
    (0, 4),
    (0, 8),
    (0, 16),
    (2, 8),
    (2, 16),
    (3, 1),
    (3, 2),
    (3, 4),
    (3, 8),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

/// Bytes of the row buffer logged past the transformed row, so that an overrun
/// in either library becomes visible in the trace.
const SLACK: usize = 24;

/// The reference C `libpng.so` (`c_src/CMakeLists.txt`) is linked without
/// `-lm`, so its `floor`/`pow` references stay unresolved and the *first*
/// floating-point entry point that is called aborts the process with
/// "symbol lookup error: undefined symbol: floor".  Loading libm into the
/// global symbol scope makes the lazy binding resolvable.  This changes symbol
/// resolution only, and both libraries end up using the same libm.
fn ensure_libm() {
    use std::sync::OnceLock;
    static LIBM: OnceLock<libloading::os::unix::Library> = OnceLock::new();
    LIBM.get_or_init(|| unsafe {
        libloading::os::unix::Library::open(Some("libm.so.6"), 0x2 | 0x100)
            .expect("dlopen libm.so.6")
    });
}

fn palette_for(bd: u8, seed: u64) -> Vec<u8> {
    let n = 1usize << bd; // every index of a `bd`-bit image is in range
    let mut r = Rng::new(seed);
    (0..3 * n).map(|_| r.byte()).collect()
}

fn plte_chunk(bd: u8, seed: u64) -> Chunk {
    Chunk::new(b"PLTE", palette_for(bd, seed))
}

fn trns_for(ct: u8, bd: u8, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    match ct {
        0 => {
            let m: u32 = if bd >= 16 { 0xffff } else { (1u32 << bd) - 1 };
            ((r.next_u32() % (m + 1)) as u16).to_be_bytes().to_vec()
        }
        2 => {
            let m: u32 = if bd >= 16 { 0xffff } else { 0xff };
            let mut v = Vec::new();
            for _ in 0..3 {
                v.extend_from_slice(&((r.next_u32() % (m + 1)) as u16).to_be_bytes());
            }
            v
        }
        _ => {
            let n = 1usize << bd;
            r.bytes(n)
        }
    }
}

/// A legal sBIT payload: one byte per channel, three bytes for palette images
/// (`png_handle_sBIT` in `pngrutil.c` uses `truelen = 3` there), every value in
/// `1..=sample_depth`.
fn sbit_payload(ct: u8, bd: u8, seed: u64) -> Vec<u8> {
    let sd = if ct == 3 { 8 } else { bd };
    let n = match ct {
        0 => 1,
        2 => 3,
        3 => 3,
        4 => 2,
        _ => 4,
    };
    let mut r = Rng::new(seed);
    (0..n).map(|_| 1 + r.below(sd as u32) as u8).collect()
}

/// Valid PNG with optional sBIT / gAMA / tRNS.  tRNS is only emitted for the
/// colour types where it is legal (0, 2, 3).
fn mkpng(
    w: u32,
    h: u32,
    ct: u8,
    bd: u8,
    il: u8,
    seed: u64,
    sbit: bool,
    gama: Option<u32>,
    trns: bool,
) -> Vec<u8> {
    let mut b = Builder::new(w, h, bd, ct).interlace(il);
    if sbit {
        b = b.add(b"sBIT", sbit_payload(ct, bd, seed ^ 0x5b17_0001));
    }
    if let Some(g) = gama {
        b = b.add(b"gAMA", g.to_be_bytes().to_vec());
    }
    if ct == 3 {
        b = b.add(b"PLTE", palette_for(bd, seed ^ 0x91e7_0002));
    }
    if trns && (ct == 0 || ct == 2 || ct == 3) {
        b = b.add(b"tRNS", trns_for(ct, bd, seed ^ 0x7a17_0003));
    }
    b.build_valid(seed)
}

/// Plain valid PNG.
fn mk(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    mkpng(w, h, ct, bd, il, seed, false, None, false)
}

/// Explicit chunk list: IHDR, `pre`, IDAT, `post`, IEND.  The caller supplies
/// PLTE (in `pre`) for palette images.
fn chunks_of(
    w: u32,
    h: u32,
    ct: u8,
    bd: u8,
    il: u8,
    seed: u64,
    pre: Vec<Chunk>,
    post: Vec<Chunk>,
) -> Vec<Chunk> {
    let b = Builder::new(w, h, bd, ct).interlace(il);
    let mut v = vec![Chunk::new(b"IHDR", b.ihdr_bytes())];
    v.extend(pre);
    v.push(Chunk::new(
        b"IDAT",
        pngbuild::zlib_stored(&b.raw_rows(seed)),
    ));
    v.extend(post);
    v.push(Chunk::new(b"IEND", Vec::new()));
    v
}

/// Palette image whose palette has exactly `npal` entries and whose indices all
/// stay inside it (so that no "out of range" warning is produced).
fn mk_pal_n(w: u32, h: u32, npal: usize, seed: u64) -> (Vec<u8>, Vec<u8>) {
    let mut r = Rng::new(seed);
    let pal: Vec<u8> = (0..3 * npal).map(|_| r.byte()).collect();
    let mut raw = Vec::new();
    for _ in 0..h {
        raw.push(0u8);
        for _ in 0..w {
            raw.push((r.byte() as usize % npal) as u8);
        }
    }
    let b = Builder::new(w, h, 8, 3).add(b"PLTE", pal.clone());
    (b.build(&raw, 0), pal)
}

fn chunk_str(v: u32) -> String {
    let mut s = String::new();
    for &c in v.to_be_bytes().iter() {
        if c.is_ascii_graphic() {
            s.push(c as char);
        } else {
            s.push_str(&format!("\\x{c:02x}"));
        }
    }
    s
}

// ---------------------------------------------------------------------------
// generic read driver
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Copy)]
struct Opts {
    /// log `png_get_current_pass_number` / `png_get_current_row_number`
    rownum: bool,
    /// 0 = `read_update_info`, 1 = `start_read_image`, 2 = `read_update_info`
    /// twice, 3 = neither, 4 = `start_read_image` twice,
    /// 5 = `read_update_info` then `start_read_image`
    seq: u8,
    /// pass a second info struct to `png_read_end` and dump it
    end_info: bool,
}

fn stride_for(w: u32) -> usize {
    // worst case after transforms: 4 channels * 2 bytes per pixel
    w as usize * 8 + 64
}

/// How many bytes of each row buffer are logged.  Independent of the applied
/// transforms so that the trace covers everything either library could have
/// written (plus slack).
fn loglen(w: u32) -> usize {
    w as usize * 8 + 8 + SLACK
}

#[allow(clippy::too_many_arguments)]
fn drive(
    lib: &Lib,
    png: &[u8],
    w: u32,
    h: u32,
    stride: usize,
    bp: *mut u8,
    o: Opts,
    pre: &mut dyn FnMut(&Core, Png, Info),
    set: &mut dyn FnMut(&Core, Png, Info),
) -> Trace {
    unsafe { std::ptr::write_bytes(bp, 0, h as usize * stride) };
    with_read(lib, png, &mut |c, p, i| unsafe {
        pre(c, p, i);
        (c.read_info)(p, i);
        set(c, p, i);
        let passes = (c.set_interlace_handling)(p);
        log(format!("passes={passes}"));
        match o.seq {
            0 => (c.read_update_info)(p, i),
            1 => (c.start_read_image)(p),
            2 => {
                (c.read_update_info)(p, i);
                (c.read_update_info)(p, i);
            }
            3 => {}
            4 => {
                (c.start_read_image)(p);
                (c.start_read_image)(p);
            }
            _ => {
                (c.read_update_info)(p, i);
                (c.start_read_image)(p);
            }
        }
        log_all_info(c, p, i);
        let rb = (c.get_rowbytes)(p, i);
        log(format!(
            "rowbytes={rb} channels={} depth={} color={}",
            (c.get_channels)(p, i),
            (c.get_bit_depth)(p, i),
            (c.get_color_type)(p, i)
        ));
        let n = loglen(w).min(stride);
        if rb + 8 > n {
            log(format!("BUF_TOO_SMALL rb={rb} n={n}"));
            return;
        }
        for pass in 0..passes {
            for y in 0..h {
                let rp = bp.add(y as usize * stride);
                (c.read_row)(p, rp, std::ptr::null_mut());
                log(format!(
                    "p{pass}r{y}={}",
                    hex(std::slice::from_raw_parts(rp, n))
                ));
                if o.rownum {
                    log(format!(
                        "pass_no={} row_no={}",
                        (c.get_current_pass_number)(p),
                        (c.get_current_row_number)(p)
                    ));
                }
            }
        }
        if o.end_info {
            let e = (c.create_info)(p);
            log(format!("end_info={}", if e.is_null() { 0 } else { 1 }));
            (c.read_end)(p, e);
            if !e.is_null() {
                log_all_info(c, p, e);
                let mut e2 = e;
                (c.destroy_info)(p, &mut e2);
                log("end_info_destroyed".to_string());
            }
        } else {
            (c.read_end)(p, std::ptr::null_mut());
        }
    })
}

fn case_all<P: FnMut(&Core, Png, Info), S: FnMut(&Core, Png, Info)>(
    label: &str,
    png: &[u8],
    w: u32,
    h: u32,
    o: Opts,
    mut pre: P,
    mut set: S,
) {
    let stride = stride_for(w);
    let mut buf = vec![0u8; h as usize * stride];
    let bp = buf.as_mut_ptr();
    diff(label, |lib| {
        drive(lib, png, w, h, stride, bp, o, &mut pre, &mut set)
    });
}

fn noset(_c: &Core, _p: Png, _i: Info) {}

/// Transforms set after `png_read_info`.
fn case<S: FnMut(&Core, Png, Info)>(label: &str, png: &[u8], w: u32, h: u32, set: S) {
    case_all(label, png, w, h, Opts::default(), noset, set)
}

/// Options set before `png_read_info`.
fn case_pre<P: FnMut(&Core, Png, Info)>(label: &str, png: &[u8], w: u32, h: u32, pre: P) {
    case_all(label, png, w, h, Opts::default(), pre, noset)
}

// ---------------------------------------------------------------------------
// R17 — png_set_shift
// ---------------------------------------------------------------------------

fn shift_of(bd: u8, seed: u64) -> PngColor8 {
    let mut r = Rng::new(seed);
    let mut v = || 1 + r.below(bd as u32) as u8;
    PngColor8 {
        red: v(),
        green: v(),
        blue: v(),
        gray: v(),
        alpha: v(),
    }
}

#[test]
fn r17_set_shift() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for sbit in [false, true] {
                for k in 0..3u64 {
                    let seed = 0x17_0000 + (ct as u64) * 101 + (bd as u64) * 13 + il as u64 + k * 7;
                    let png = mkpng(w, h, ct, bd, il, seed, sbit, None, false);
                    let sh = shift_of(bd, seed ^ 0x5117_dead);
                    case(
                        &format!("R17 ct={ct} bd={bd} il={il} sbit={sbit} k={k} sh={sh:?}"),
                        &png,
                        w,
                        h,
                        |c, p, _i| unsafe {
                            (c.set_shift)(p, &sh as *const PngColor8 as *const u8)
                        },
                    );
                }
            }
        }
    }
    // png_set_shift combined with the expanding transforms, and with sBIT in
    // the stream so that png_read_transform_info sees both.
    for &(ct, bd) in COMBOS {
        let seed = 0x17_8000 + (ct as u64) * 31 + bd as u64;
        let png = mkpng(w, h, ct, bd, 0, seed, true, None, true);
        let sh = shift_of(bd, seed ^ 0x1234);
        case(
            &format!("R17x ct={ct} bd={bd} sh={sh:?}"),
            &png,
            w,
            h,
            |c, p, _i| unsafe {
                (c.set_expand)(p);
                (c.set_shift)(p, &sh as *const PngColor8 as *const u8);
                (c.set_packing)(p);
            },
        );
    }
    // A palette image whose sBIT chunk carries a single byte: the length libpng
    // demands there is 3, so this is the "bad length" benign-error path.
    for &bd in &[1u8, 2, 4, 8] {
        let seed = 0x17_c000 + bd as u64;
        let b = Builder::new(w, h, bd, 3)
            .add(b"sBIT", vec![bd])
            .add(b"PLTE", palette_for(bd, seed));
        let png = b.build_valid(seed);
        let sh = shift_of(bd, seed ^ 0x99);
        case(
            &format!("R17 sbit1 bd={bd} sh={sh:?}"),
            &png,
            w,
            h,
            |c, p, _i| unsafe { (c.set_shift)(p, &sh as *const PngColor8 as *const u8) },
        );
    }
}

// ---------------------------------------------------------------------------
// R18 — png_set_gamma / png_set_gamma_fixed
// ---------------------------------------------------------------------------

const PNG_FP_1: i32 = 100_000;
const PNG_DEFAULT_sRGB: i32 = -1;
const PNG_GAMMA_MAC_18: i32 = -2;

/// (screen_gamma, override_file_gamma) pairs for the floating point API.
const GPAIRS_FP: &[(f64, f64, &str)] = &[
    (2.2, 0.0, "2.2_0"),
    (1.0, 1.0, "1_1"),
    (2.2, 0.45455, "2.2_srgb"),
    (1.8, 2.2, "1.8_2.2"),
    (0.0, 0.0, "0_0"),
];

/// Fixed-point pairs, including the reserved flag values and the boundaries of
/// `PNG_LIB_GAMMA_MIN`/`MAX` (1000 / 10000000 in `pngpriv.h`).
const GPAIRS_FX: &[(i32, i32, &str)] = &[
    (220_000, 45455, "fx_srgb"),
    (PNG_FP_1, PNG_FP_1, "fx_1_1"),
    (180_000, 220_000, "fx_1.8_2.2"),
    (1_000, 1_000, "fx_min"),
    (10_000_000, 10_000_000, "fx_max"),
    (999, 999, "fx_below_min"),
    (10_000_001, 10_000_001, "fx_above_max"),
    (PNG_DEFAULT_sRGB, PNG_DEFAULT_sRGB, "fx_flag_srgb"),
    (PNG_GAMMA_MAC_18, PNG_GAMMA_MAC_18, "fx_flag_mac"),
    (0, 0, "fx_0_0"),
    (i32::MAX, i32::MAX, "fx_intmax"),
];

const GCOMBOS: &[(u8, u8)] = &[
    (0, 8),
    (0, 16),
    (2, 8),
    (2, 16),
    (3, 8),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

#[test]
fn r18_set_gamma() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    for &(ct, bd) in GCOMBOS {
        for gama in [None, Some(45455u32), Some(100_000u32)] {
            let seed = 0x18_0000
                + (ct as u64) * 97
                + (bd as u64) * 11
                + gama.unwrap_or(0) as u64 % 1000;
            let png = mkpng(w, h, ct, bd, 0, seed, false, gama, false);
            let gtag = match gama {
                None => "none",
                Some(45455) => "srgb",
                _ => "one",
            };
            for &(s, f, name) in GPAIRS_FP {
                case(
                    &format!("R18 fp ct={ct} bd={bd} gama={gtag} {name}"),
                    &png,
                    w,
                    h,
                    |c, p, _i| unsafe { (c.set_gamma)(p, s, f) },
                );
            }
            for &(s, f, name) in GPAIRS_FX {
                case(
                    &format!("R18 fx ct={ct} bd={bd} gama={gtag} {name}"),
                    &png,
                    w,
                    h,
                    |c, p, _i| unsafe { (c.set_gamma_fixed)(p, s, f) },
                );
            }
        }
    }
    // interlaced + tRNS-bearing inputs, one gamma pair each
    for &(ct, bd) in GCOMBOS {
        let seed = 0x18_8000 + (ct as u64) * 41 + bd as u64;
        let png = mkpng(w, h, ct, bd, 1, seed, false, Some(45455), true);
        case(
            &format!("R18il ct={ct} bd={bd}"),
            &png,
            w,
            h,
            |c, p, _i| unsafe { (c.set_gamma)(p, 2.2, 0.45455) },
        );
    }
    // 16-bit inputs carrying sBIT: `png_build_gamma_table` (png.c) derives
    // `gamma_shift = 16 - sig_bit`, clamped to 8, so the shape of the 16-bit
    // gamma tables depends on the significant-bit count.  Also exercised
    // together with strip_16/scale_16, which forces
    // `shift >= 16 - PNG_MAX_GAMMA_8`.
    //
    // NOTE (translation bug, reported): for every `sb <= 8` the *Rust* library
    // aborts here with "attempt to shift right with overflow" in
    // `src/pngrtran.rs` (`*sp.offset(1) >> gamma_shift` with `gamma_shift == 8`
    // on a `u8`), while the C library promotes `sp[1]` to `int` and correctly
    // yields 0.  The configurations are deliberately kept.
    for &(ct, bd) in &[(0u8, 16u8), (2, 16), (4, 16), (6, 16)] {
        let nch = pngbuild::channels(ct) as usize;
        for &sb in &[1u8, 4, 8, 9, 12, 16] {
            let seed = 0x18_c000 + (ct as u64) * 31 + sb as u64;
            let png = Builder::new(w, h, bd, ct)
                .add(b"sBIT", vec![sb; nch])
                .add(b"gAMA", 45455u32.to_be_bytes().to_vec())
                .build_valid(seed);
            for v in 0u8..3 {
                case(
                    &format!("R18sbit ct={ct} bd={bd} sBIT={sb} v={v}"),
                    &png,
                    w,
                    h,
                    |c, p, _i| unsafe {
                        (c.set_gamma)(p, 1.8, 2.2);
                        match v {
                            1 => (c.set_strip_16)(p),
                            2 => (c.set_scale_16)(p),
                            _ => {}
                        }
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R19 — png_set_alpha_mode / png_set_alpha_mode_fixed
// ---------------------------------------------------------------------------

const ACOMBOS: &[(u8, u8)] = &[
    (0, 8),
    (0, 16),
    (2, 8),
    (3, 8),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

#[test]
fn r19_alpha_mode() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    let modes: &[(c_int, &str)] = &[
        (PNG_ALPHA_PNG, "PNG"),
        (PNG_ALPHA_STANDARD, "STANDARD"),
        (PNG_ALPHA_OPTIMIZED, "OPTIMIZED"),
        (PNG_ALPHA_BROKEN, "BROKEN"),
    ];
    let gammas_fp: &[(f64, &str)] = &[(1.0, "1.0"), (2.2, "2.2"), (0.45455, "0.45455")];
    let gammas_fx: &[(i32, &str)] = &[
        (45455, "fx45455"),
        (220_000, "fx220000"),
        (PNG_DEFAULT_sRGB, "fxsRGB"),
    ];
    for &(ct, bd) in ACOMBOS {
        for gama in [None, Some(45455u32)] {
            let seed = 0x19_0000 + (ct as u64) * 89 + (bd as u64) * 7 + gama.is_some() as u64;
            let png = mkpng(w, h, ct, bd, 0, seed, false, gama, false);
            let gtag = if gama.is_some() { "g" } else { "n" };
            for &(m, mname) in modes {
                for &(g, gname) in gammas_fp {
                    case(
                        &format!("R19 fp ct={ct} bd={bd} gama={gtag} m={mname} og={gname}"),
                        &png,
                        w,
                        h,
                        |c, p, _i| unsafe { (c.set_alpha_mode)(p, m, g) },
                    );
                }
                for &(g, gname) in gammas_fx {
                    case(
                        &format!("R19 fx ct={ct} bd={bd} gama={gtag} m={mname} og={gname}"),
                        &png,
                        w,
                        h,
                        |c, p, _i| unsafe { (c.set_alpha_mode_fixed)(p, m, g) },
                    );
                }
            }
        }
    }
    // tRNS-bearing inputs (the alpha channel appears only after expansion)
    for &(ct, bd) in &[(0u8, 8u8), (2, 8), (3, 8)] {
        for &(m, mname) in modes {
            let seed = 0x19_8000 + (ct as u64) * 13 + bd as u64 + m as u64;
            let png = mkpng(w, h, ct, bd, 0, seed, false, Some(45455), true);
            case(
                &format!("R19trns ct={ct} bd={bd} m={mname}"),
                &png,
                w,
                h,
                |c, p, _i| unsafe {
                    (c.set_expand)(p);
                    (c.set_alpha_mode)(p, m, 2.2);
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// R20 — png_set_background / png_set_background_fixed
// ---------------------------------------------------------------------------

fn bg_for(_ct: u8, bd: u8, seed: u64) -> PngColor16 {
    let mut r = Rng::new(seed);
    let m: u32 = if bd >= 16 { 0xffff } else { 0xff };
    let nidx: u32 = 1u32 << bd.min(8);
    PngColor16 {
        index: (r.byte() as u32 % nidx) as u8,
        red: (r.next_u32() % (m + 1)) as u16,
        green: (r.next_u32() % (m + 1)) as u16,
        blue: (r.next_u32() % (m + 1)) as u16,
        gray: (r.next_u32() % (m + 1)) as u16,
    }
}

#[test]
fn r20_background() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    let codes: &[(c_int, &str)] = &[
        (PNG_BACKGROUND_GAMMA_UNKNOWN, "UNKNOWN"),
        (PNG_BACKGROUND_GAMMA_SCREEN, "SCREEN"),
        (PNG_BACKGROUND_GAMMA_FILE, "FILE"),
        (PNG_BACKGROUND_GAMMA_UNIQUE, "UNIQUE"),
    ];
    let combos: &[(u8, u8)] = &[
        (0, 8),
        (0, 16),
        (2, 8),
        (2, 16),
        (3, 4),
        (3, 8),
        (4, 8),
        (4, 16),
        (6, 8),
        (6, 16),
    ];
    for &(ct, bd) in combos {
        // tRNS for the colour types that can carry it, a real alpha channel
        // otherwise.
        let seed = 0x20_0000 + (ct as u64) * 79 + (bd as u64) * 5;
        let png = mkpng(w, h, ct, bd, 0, seed, false, Some(45455), true);
        let bg = bg_for(ct, bd, seed ^ 0xbeef);
        for &(code, cname) in codes {
            for ne in [0 as c_int, 1] {
                for withg in [false, true] {
                    for fixed in [false, true] {
                        let label = format!(
                            "R20 ct={ct} bd={bd} code={cname} ne={ne} g={withg} fx={fixed}"
                        );
                        case(&label, &png, w, h, |c, p, _i| unsafe {
                            if withg {
                                (c.set_gamma)(p, 2.2, 0.45455);
                            }
                            let bp = &bg as *const PngColor16 as *const u8;
                            if fixed {
                                (c.set_background_fixed)(p, bp, code, ne, 100_000);
                            } else {
                                (c.set_background)(p, bp, code, ne, 1.0);
                            }
                        });
                    }
                }
            }
        }
    }
    // 16-bit background values on 16-bit inputs, and a palette index background
    // on a palette input, combined with the expanding transforms.
    for &(ct, bd) in &[(0u8, 16u8), (2, 16), (3, 4), (3, 8), (6, 16)] {
        let seed = 0x20_8000 + (ct as u64) * 17 + bd as u64;
        let png = mkpng(w, h, ct, bd, 1, seed, false, Some(45455), true);
        let bg = bg_for(ct, bd, seed ^ 0x5151);
        for &(code, cname) in codes {
            case(
                &format!("R20x ct={ct} bd={bd} code={cname}"),
                &png,
                w,
                h,
                |c, p, _i| unsafe {
                    (c.set_expand)(p);
                    (c.set_gamma)(p, 1.8, 0.5);
                    (c.set_background)(p, &bg as *const PngColor16 as *const u8, code, 0, 1.8);
                },
            );
        }
    }
    // several distinct background colours per input: two random ones, all-zero
    // and all-ones (0xffff), so that both ends of the composite arithmetic are
    // covered for grey, RGB, 16-bit and palette-index backgrounds.
    for &(ct, bd) in combos {
        let seed = 0x20_c000 + (ct as u64) * 23 + bd as u64;
        let png = mkpng(w, h, ct, bd, 0, seed, false, Some(45455), true);
        let bgs: [(PngColor16, &str); 4] = [
            (bg_for(ct, bd, seed ^ 0x11), "rnd1"),
            (bg_for(ct, bd, seed ^ 0x22), "rnd2"),
            (PngColor16::default(), "zero"),
            // The largest legal background for this input: with
            // need_expand == 0 libpng indexes its 8-bit gamma table with the
            // background components directly, so anything above 0xff would be
            // an out-of-range value for an 8-bit-sample image.
            (
                PngColor16 {
                    index: ((1u32 << bd.min(8)) - 1) as u8,
                    red: if bd == 16 { 0xffff } else { 0xff },
                    green: if bd == 16 { 0xffff } else { 0xff },
                    blue: if bd == 16 { 0xffff } else { 0xff },
                    gray: if bd == 16 { 0xffff } else { 0xff },
                },
                "max",
            ),
        ];
        for (bg, bname) in bgs.iter() {
            for &(code, cname) in &codes[1..] {
                for ne in [0 as c_int, 1] {
                    case(
                        &format!("R20bg ct={ct} bd={bd} bg={bname} code={cname} ne={ne}"),
                        &png,
                        w,
                        h,
                        |c, p, _i| unsafe {
                            (c.set_gamma)(p, 2.2, 1.0);
                            (c.set_background)(
                                p,
                                bg as *const PngColor16 as *const u8,
                                code,
                                ne,
                                0.9,
                            );
                        },
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R21 — png_set_quantize
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn r21_case(
    label: &str,
    png: &[u8],
    w: u32,
    h: u32,
    own: Option<usize>,
    max_colors: c_int,
    use_hist: bool,
    full: c_int,
    seed: u64,
    twice: bool,
) {
    let stride = stride_for(w);
    let mut buf = vec![0u8; h as usize * stride];
    let bp = buf.as_mut_ptr();
    diff(label, |lib| {
        // Fresh palette / histogram for every run: png_set_quantize rearranges
        // the caller's palette in place.
        let n = own.unwrap_or(256);
        let mut r = Rng::new(seed);
        let mut pal: Vec<u8> = (0..3 * n).map(|_| r.byte()).collect();
        let mut hist: Vec<u16> = (0..n).map(|_| (r.next_u32() & 0xffff) as u16).collect();
        let pp = pal.as_mut_ptr();
        let hp = hist.as_mut_ptr();
        unsafe { std::ptr::write_bytes(bp, 0, h as usize * stride) };
        let t = with_read(lib, png, &mut |c, p, i| unsafe {
            (c.read_info)(p, i);
            let (palp, npal) = match own {
                Some(k) => (pp, k as c_int),
                None => {
                    let mut ip: *mut u8 = std::ptr::null_mut();
                    let mut nn: c_int = 0;
                    let rc = (c.get_PLTE)(p, i, &mut ip, &mut nn);
                    log(format!("PLTE_in rc={rc} n={nn}"));
                    (ip, nn)
                }
            };
            let hptr: *const u16 = if use_hist {
                hp as *const u16
            } else {
                std::ptr::null()
            };
            (c.set_quantize)(p, palp, npal, max_colors, hptr, full);
            if twice {
                // documented as legal: "Applications are allowed to call this
                // function more than once per png_struct."
                (c.set_quantize)(p, palp, npal, max_colors, hptr, full);
            }
            let passes = (c.set_interlace_handling)(p);
            log(format!("passes={passes}"));
            (c.read_update_info)(p, i);
            log_all_info(c, p, i);
            let rb = (c.get_rowbytes)(p, i);
            log(format!("rowbytes={rb}"));
            let nn = loglen(w).min(stride);
            if rb + 8 > nn {
                log(format!("BUF_TOO_SMALL rb={rb}"));
                return;
            }
            for pass in 0..passes {
                for y in 0..h {
                    let rp = bp.add(y as usize * stride);
                    (c.read_row)(p, rp, std::ptr::null_mut());
                    log(format!(
                        "p{pass}r{y}={}",
                        hex(std::slice::from_raw_parts(rp, nn))
                    ));
                }
            }
            (c.read_end)(p, std::ptr::null_mut());
            // the caller's palette, possibly rearranged by png_set_quantize
            if own.is_some() {
                log(format!(
                    "caller_palette={}",
                    hex(std::slice::from_raw_parts(pp, 3 * n))
                ));
            }
        });
        t
    });
}

#[test]
fn r21_quantize() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    let maxc: &[c_int] = &[2, 4, 16, 100, 256];
    // palette inputs: png_set_quantize gets the image's own palette
    for &bd in &[2u8, 4] {
        let seed = 0x21_0000 + bd as u64 * 7;
        let png = mkpng(w, h, 3, bd, 0, seed, false, None, false);
        for &full in &[0 as c_int, 1] {
            for &mc in maxc {
                for uh in [false, true] {
                    r21_case(
                        &format!("R21 pal bd={bd} full={full} mc={mc} hist={uh}"),
                        &png,
                        w,
                        h,
                        None,
                        mc,
                        uh,
                        full,
                        seed ^ 0x1111,
                        false,
                    );
                }
            }
        }
    }
    // an 8-bit palette image with 24 entries (keeps the quantize cube cheap)
    {
        let seed = 0x21_2000;
        let (png, _pal) = mk_pal_n(w, h, 24, seed);
        for &full in &[0 as c_int, 1] {
            for &mc in maxc {
                for uh in [false, true] {
                    r21_case(
                        &format!("R21 pal24 full={full} mc={mc} hist={uh}"),
                        &png,
                        w,
                        h,
                        None,
                        mc,
                        uh,
                        full,
                        seed ^ 0x2222,
                        false,
                    );
                }
            }
        }
    }
    // RGB / RGBA inputs: a random caller palette
    for &(ct, bd) in &[(2u8, 8u8), (6, 8), (2, 16), (6, 16)] {
        let seed = 0x21_4000 + (ct as u64) * 13 + bd as u64;
        let png = mkpng(w, h, ct, bd, 0, seed, false, None, false);
        for &full in &[0 as c_int, 1] {
            for &mc in maxc {
                for uh in [false, true] {
                    r21_case(
                        &format!("R21 rgb ct={ct} bd={bd} full={full} mc={mc} hist={uh}"),
                        &png,
                        w,
                        h,
                        Some(32),
                        mc,
                        uh,
                        full,
                        seed ^ 0x3333,
                        false,
                    );
                }
            }
        }
    }
    // a 256-entry caller palette, both full_quantize settings
    for &full in &[0 as c_int, 1] {
        let seed = 0x21_6000 + full as u64;
        let png = mkpng(w, h, 2, 8, 0, seed, false, None, false);
        r21_case(
            &format!("R21 rgb256 full={full}"),
            &png,
            w,
            h,
            Some(256),
            100,
            true,
            full,
            seed ^ 0x4444,
            false,
        );
    }
    // interlaced palette input
    {
        let seed = 0x21_8000;
        let png = mkpng(w, h, 3, 4, 1, seed, false, None, false);
        for &mc in &[4 as c_int, 16] {
            r21_case(
                &format!("R21 il mc={mc}"),
                &png,
                w,
                h,
                None,
                mc,
                false,
                0,
                seed ^ 0x5555,
                false,
            );
        }
    }
    // png_set_quantize called twice on the same struct
    for &(ct, bd, own) in &[(3u8, 4u8, None), (2, 8, Some(32usize)), (6, 8, Some(32))] {
        let seed = 0x21_a000 + (ct as u64) * 19 + bd as u64;
        let png = mkpng(w, h, ct, bd, 0, seed, false, None, false);
        for &full in &[0 as c_int, 1] {
            for &mc in &[4 as c_int, 16] {
                for uh in [false, true] {
                    r21_case(
                        &format!("R21 twice ct={ct} bd={bd} full={full} mc={mc} hist={uh}"),
                        &png,
                        w,
                        h,
                        own,
                        mc,
                        uh,
                        full,
                        seed ^ 0x6666,
                        true,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R22 — png_set_read_user_transform_fn + png_set_user_transform_info
// ---------------------------------------------------------------------------

static DUMMY_PTR: u8 = 0;

unsafe extern "C" fn cb_user_transform(_png: Png, ri: *mut PngRowInfo, row: *mut u8) {
    if ri.is_null() || row.is_null() {
        log("UT null".to_string());
        return;
    }
    let r = *ri;
    log(format!(
        "UT w={} rb={} ct={} bd={} ch={} pd={}",
        r.width, r.rowbytes, r.color_type, r.bit_depth, r.channels, r.pixel_depth
    ));
    let s = std::slice::from_raw_parts(row, r.rowbytes);
    log(format!("UT in={}", hex(s)));
    let d = std::slice::from_raw_parts_mut(row, r.rowbytes);
    for (k, b) in d.iter_mut().enumerate() {
        *b = b.rotate_left(3) ^ (0x5au8.wrapping_add(k as u8));
    }
    log(format!(
        "UT out={}",
        hex(std::slice::from_raw_parts(row, r.rowbytes))
    ));
}

#[test]
fn r22_user_transform() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for variant in 0u8..4 {
                let seed = 0x22_0000
                    + (ct as u64) * 71
                    + (bd as u64) * 5
                    + il as u64
                    + variant as u64 * 3;
                let png = mkpng(w, h, ct, bd, il, seed, false, None, true);
                case(
                    &format!("R22 ct={ct} bd={bd} il={il} v={variant}"),
                    &png,
                    w,
                    h,
                    |c, p, _i| unsafe {
                        log(format!(
                            "utp_before={}",
                            if (c.get_user_transform_ptr)(p).is_null() {
                                0
                            } else {
                                1
                            }
                        ));
                        (c.set_read_user_transform_fn)(p, cb_user_transform as Cb);
                        match variant {
                            1 => (c.set_expand)(p),
                            2 => (c.set_gray_to_rgb)(p),
                            3 => {
                                (c.set_expand)(p);
                                (c.set_gray_to_rgb)(p);
                            }
                            _ => {}
                        }
                        log(format!(
                            "utp_after={}",
                            if (c.get_user_transform_ptr)(p).is_null() {
                                0
                            } else {
                                1
                            }
                        ));
                    },
                );
            }
        }
    }
    // png_set_user_transform_info: a non-NULL user pointer plus depth/channel
    // overrides.  Only non-increasing (depth, channels) pairs are used: a larger
    // pixel depth would make libpng copy more bytes out of its own row buffer
    // than it holds, which is documented as the application's responsibility.
    for &(ct, bd) in COMBOS {
        let chans: c_int = match ct {
            0 | 3 => 1,
            2 => 3,
            4 => 2,
            _ => 4,
        };
        let infos: &[(c_int, c_int, &str)] = &[
            (0, 0, "none"),
            (bd as c_int, chans, "same"),
            (bd as c_int, 1, "ch1"),
            (if bd == 16 { 8 } else { bd as c_int }, chans, "d8"),
            (if bd == 16 { 8 } else { 1 }, 1, "min"),
        ];
        for &(d, n, iname) in infos {
            let seed = 0x22_8000 + (ct as u64) * 23 + (bd as u64) * 3 + d as u64 + n as u64;
            let png = mkpng(w, h, ct, bd, 0, seed, false, None, false);
            for hasptr in [false, true] {
                case(
                    &format!("R22i ct={ct} bd={bd} info={iname} d={d} n={n} ptr={hasptr}"),
                    &png,
                    w,
                    h,
                    |c, p, i| unsafe {
                        (c.set_read_user_transform_fn)(p, cb_user_transform as Cb);
                        let up = if hasptr {
                            &DUMMY_PTR as *const u8 as *mut c_void
                        } else {
                            std::ptr::null_mut()
                        };
                        (c.set_user_transform_info)(p, up, d, n);
                        log(format!(
                            "utp={} rowbytes_now={}",
                            if (c.get_user_transform_ptr)(p).is_null() {
                                0
                            } else {
                                1
                            },
                            (c.get_rowbytes)(p, i)
                        ));
                    },
                );
            }
        }
    }
    // png_set_user_transform_info after png_read_update_info: app error path
    for &(ct, bd) in &[(0u8, 8u8), (6, 16)] {
        let seed = 0x22_c000 + ct as u64;
        let png = mkpng(w, h, ct, bd, 0, seed, false, None, false);
        case_all(
            &format!("R22late ct={ct} bd={bd}"),
            &png,
            w,
            h,
            Opts::default(),
            noset,
            |c, p, _i| unsafe {
                (c.set_read_user_transform_fn)(p, cb_user_transform as Cb);
                (c.set_user_transform_info)(p, std::ptr::null_mut(), 8, 1);
            },
        );
    }
}

// ---------------------------------------------------------------------------
// R23 — png_set_read_status_fn
// ---------------------------------------------------------------------------

unsafe extern "C" fn cb_row_status(_png: Png, row: u32, pass: c_int) {
    log(format!("STATUS row={row} pass={pass}"));
}

fn r23_case(label: &str, png: &[u8], w: u32, h: u32, mode: u8) {
    let stride = stride_for(w);
    let mut buf = vec![0u8; h as usize * stride];
    let bp = buf.as_mut_ptr();
    let mut arr: Vec<*mut u8> = (0..h as usize)
        .map(|y| unsafe { bp.add(y * stride) })
        .collect();
    let ap = arr.as_mut_ptr();
    diff(label, |lib| {
        unsafe { std::ptr::write_bytes(bp, 0, h as usize * stride) };
        with_read(lib, png, &mut |c, p, i| unsafe {
            (c.set_read_status_fn)(p, cb_row_status as Cb);
            (c.read_info)(p, i);
            let passes = (c.set_interlace_handling)(p);
            log(format!("passes={passes}"));
            (c.read_update_info)(p, i);
            let rb = (c.get_rowbytes)(p, i);
            let n = loglen(w).min(stride);
            log(format!("rowbytes={rb}"));
            if rb + 8 > n {
                log("BUF_TOO_SMALL".to_string());
                return;
            }
            match mode {
                0 => {
                    for _pass in 0..passes {
                        for y in 0..h {
                            (c.read_row)(p, bp.add(y as usize * stride), std::ptr::null_mut());
                        }
                    }
                }
                1 => {
                    for _pass in 0..passes {
                        (c.read_rows)(p, ap, std::ptr::null_mut(), h);
                    }
                }
                2 => {
                    for _pass in 0..passes {
                        for y in 0..h as usize {
                            (c.read_rows)(p, ap.add(y), std::ptr::null_mut(), 1);
                        }
                    }
                }
                _ => (c.read_image)(p, ap),
            }
            for y in 0..h as usize {
                log(format!(
                    "row{y}={}",
                    hex(std::slice::from_raw_parts(bp.add(y * stride), n))
                ));
            }
            (c.read_end)(p, std::ptr::null_mut());
        })
    });
}

#[test]
fn r23_read_status_fn() {
    ensure_libm();
    let (w, h) = (5u32, 5u32);
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for mode in 0u8..4 {
                let seed = 0x23_0000 + (ct as u64) * 61 + (bd as u64) * 3 + il as u64 + mode as u64;
                let png = mk(w, h, ct, bd, il, seed);
                r23_case(
                    &format!("R23 ct={ct} bd={bd} il={il} mode={mode}"),
                    &png,
                    w,
                    h,
                    mode,
                );
            }
        }
    }
    // taller interlaced images exercise every Adam7 pass row count
    for &(w2, h2) in &[(1u32, 1u32), (9, 8), (17, 3)] {
        for il in [0u8, 1] {
            for mode in 0u8..4 {
                let seed = 0x23_8000 + w2 as u64 * 7 + h2 as u64 + il as u64;
                let png = mk(w2, h2, 6, 8, il, seed);
                r23_case(
                    &format!("R23s w={w2} h={h2} il={il} mode={mode}"),
                    &png,
                    w2,
                    h2,
                    mode,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R24 — png_set_crc_action
// ---------------------------------------------------------------------------

#[test]
fn r24_crc_action() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    let names = [
        "DEFAULT",
        "ERROR_QUIT",
        "WARN_DISCARD",
        "WARN_USE",
        "QUIET_USE",
        "NO_CHANGE",
    ];
    // (label, bytes) — five streams, one clean and four with a broken CRC.
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    {
        let seed = 0x24_0001;
        let text = Chunk::new(b"tEXt", b"Key\0value".to_vec());
        // all CRCs correct
        let good = chunks_of(w, h, 2, 8, 0, seed, vec![text.clone()], vec![]);
        inputs.push(("good".to_string(), pngbuild::join(&good)));
        // ancillary chunk with a bad CRC
        let mut bad = good.clone();
        bad[1] = bad[1].clone().bad_crc();
        inputs.push(("bad_tEXt".to_string(), pngbuild::join(&bad)));
        // critical: IDAT
        let mut bad = good.clone();
        let k = bad.iter().position(|c| &c.name == b"IDAT").unwrap();
        bad[k] = bad[k].clone().bad_crc();
        inputs.push(("bad_IDAT".to_string(), pngbuild::join(&bad)));
        // critical: IHDR
        let mut bad = good.clone();
        bad[0] = bad[0].clone().bad_crc();
        inputs.push(("bad_IHDR".to_string(), pngbuild::join(&bad)));
    }
    {
        // critical: PLTE (needs a palette image)
        let seed = 0x24_0002;
        let mut cs = chunks_of(w, h, 3, 4, 0, seed, vec![plte_chunk(4, seed)], vec![]);
        let k = cs.iter().position(|c| &c.name == b"PLTE").unwrap();
        cs[k] = cs[k].clone().bad_crc();
        inputs.push(("bad_PLTE".to_string(), pngbuild::join(&cs)));
        // and the same stream with every CRC correct
        let cs = chunks_of(w, h, 3, 4, 0, seed, vec![plte_chunk(4, seed)], vec![]);
        inputs.push(("good_pal".to_string(), pngbuild::join(&cs)));
    }
    for (iname, png) in &inputs {
        for crit in 0..6usize {
            for anc in 0..6usize {
                // Each combination is its own run so that a fatal error in one
                // does not hide the others.
                case_pre(
                    &format!(
                        "R24 in={iname} crit={} anc={}",
                        names[crit], names[anc]
                    ),
                    png,
                    w,
                    h,
                    |c, p, _i| unsafe {
                        (c.set_crc_action)(p, crit as c_int, anc as c_int);
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R25 — png_set_keep_unknown_chunks / png_set_read_user_chunk_fn
// ---------------------------------------------------------------------------

thread_local! {
    static UCHUNK_RET: Cell<c_int> = const { Cell::new(0) };
}

unsafe extern "C" fn cb_user_chunk(_png: Png, ch: *mut PngUnknownChunk) -> c_int {
    if ch.is_null() {
        log("UCH null".to_string());
        return 0;
    }
    let u = *ch;
    let nm = String::from_utf8_lossy(&u.name[..4]).into_owned();
    let data = if u.data.is_null() {
        "<null>".to_string()
    } else {
        hex(std::slice::from_raw_parts(u.data, u.size))
    };
    let r = UCHUNK_RET.with(|c| c.get());
    log(format!(
        "UCH name={nm} size={} loc={} data={data} ret={r}",
        u.size, u.location
    ));
    r
}

/// 5-byte-per-entry chunk list as `png_set_keep_unknown_chunks` expects.
fn keep_list(names: &[&[u8; 4]]) -> Vec<u8> {
    let mut v = Vec::new();
    for n in names {
        v.extend_from_slice(&n[..]);
        v.push(0);
    }
    v
}

/// Private unknown chunks: `saFe` / `moRe` are safe-to-copy (lower-case final
/// letter), `unSF` / `biGX` are not.
const UNK_NAMES: &[&[u8; 4]] = &[b"saFe", b"unSF", b"moRe", b"biGX"];

/// Every name `png_handle_as_unknown` can be asked about: the four private
/// chunks above, the complete `chunks_to_ignore` list of `png_set_keep_unknown_chunks`
/// (which `num_chunks == -1` installs), and the five chunks that list omits.
const HAU_NAMES: &[&[u8; 4]] = &[
    b"saFe", b"unSF", b"moRe", b"biGX", b"bKGD", b"cHRM", b"cICP", b"cLLI", b"eXIf", b"gAMA",
    b"hIST", b"iCCP", b"iTXt", b"mDCV", b"oFFs", b"pCAL", b"pHYs", b"sBIT", b"sCAL", b"sPLT",
    b"sTER", b"sRGB", b"tEXt", b"tIME", b"zTXt", b"IHDR", b"PLTE", b"tRNS", b"IDAT", b"IEND",
];

/// `png_handle_as_unknown` for every name.  Uses a fixed-size stack buffer so
/// that nothing needing `Drop` is live across the libpng calls.
unsafe fn log_hau(c: &Core, p: Png) {
    for nm in HAU_NAMES {
        let mut z = [0u8; 5];
        z[..4].copy_from_slice(&nm[..]);
        log(format!(
            "hau {}={}",
            std::str::from_utf8(&nm[..]).unwrap_or("?"),
            (c.handle_as_unknown)(p, z.as_ptr())
        ));
    }
    log(format!(
        "hau <null>={}",
        (c.handle_as_unknown)(p, std::ptr::null())
    ));
}

fn r25_input(seed: u64) -> Vec<u8> {
    let pre = vec![
        Chunk::new(b"saFe", vec![0x11, 0x22]),
        plte_chunk(4, seed),
        Chunk::new(b"unSF", vec![0x33, 0x44, 0x55]),
        Chunk::new(b"tEXt", b"K\0v".to_vec()),
    ];
    let post = vec![
        Chunk::new(b"moRe", vec![0x66]),
        Chunk::new(b"biGX", vec![0x77, 0x88, 0x99, 0xaa]),
    ];
    pngbuild::join(&chunks_of(5, 3, 3, 4, 0, seed, pre, post))
}

fn r25_input_rgb(seed: u64) -> Vec<u8> {
    let pre = vec![
        Chunk::new(b"saFe", vec![]),
        Chunk::new(b"unSF", vec![0x01]),
    ];
    let post = vec![Chunk::new(b"moRe", vec![0x02, 0x03])];
    pngbuild::join(&chunks_of(5, 3, 2, 8, 0, seed, pre, post))
}

#[test]
fn r25_unknown_chunks() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    let png_pal = r25_input(0x25_0001);
    let png_rgb = r25_input_rgb(0x25_0002);
    let keeps: &[(c_int, &str)] = &[
        (PNG_HANDLE_CHUNK_AS_DEFAULT, "AS_DEFAULT"),
        (PNG_HANDLE_CHUNK_NEVER, "NEVER"),
        (PNG_HANDLE_CHUNK_IF_SAFE, "IF_SAFE"),
        (PNG_HANDLE_CHUNK_ALWAYS, "ALWAYS"),
        (-1, "MINUS1"),
    ];
    let lists: &[(&[&[u8; 4]], &str)] = &[
        (&[], "empty"),
        (UNK_NAMES, "all4"),
        (&[b"saFe"], "one"),
        (&[b"saFe", b"unSF"], "two"),
        (&[b"moRe", b"biGX", b"tEXt"], "three"),
    ];
    for (png, iname) in [(&png_pal, "pal"), (&png_rgb, "rgb")] {
        for &(keep, kname) in keeps {
            // chunk_list == NULL with num_chunks == 0 (just set the default),
            // == -1 (ignore all known chunks) and > 0 (app error).
            for &nn in &[0 as c_int, -1, 2] {
                case_pre(
                    &format!("R25 in={iname} keep={kname} null_list n={nn}"),
                    png,
                    w,
                    h,
                    |c, p, _i| unsafe {
                        (c.set_keep_unknown_chunks)(p, keep, std::ptr::null(), nn);
                        log_hau(c, p);
                    },
                );
            }
            for &(names, lname) in lists {
                let list = keep_list(names);
                let n = names.len() as c_int;
                case_pre(
                    &format!("R25 in={iname} keep={kname} list={lname} n={n}"),
                    png,
                    w,
                    h,
                    |c, p, _i| unsafe {
                        (c.set_keep_unknown_chunks)(p, keep, list.as_ptr(), n);
                        log_hau(c, p);
                    },
                );
            }
        }
    }
    // png_set_read_user_chunk_fn, callback returning -1 / 0 / 1
    for (png, iname) in [(&png_pal, "pal"), (&png_rgb, "rgb")] {
        for &ret in &[-1 as c_int, 0, 1] {
            for &keep in &[
                PNG_HANDLE_CHUNK_AS_DEFAULT,
                PNG_HANDLE_CHUNK_NEVER,
                PNG_HANDLE_CHUNK_IF_SAFE,
                PNG_HANDLE_CHUNK_ALWAYS,
            ] {
                for hasptr in [false, true] {
                    UCHUNK_RET.with(|c| c.set(ret));
                    case_pre(
                        &format!("R25u in={iname} ret={ret} keep={keep} ptr={hasptr}"),
                        png,
                        w,
                        h,
                        |c, p, _i| unsafe {
                            UCHUNK_RET.with(|x| x.set(ret));
                            log(format!(
                                "ucp_before={}",
                                if (c.get_user_chunk_ptr)(p).is_null() { 0 } else { 1 }
                            ));
                            let up = if hasptr {
                                &DUMMY_PTR as *const u8 as *mut c_void
                            } else {
                                std::ptr::null_mut()
                            };
                            (c.set_read_user_chunk_fn)(p, up, cb_user_chunk as Cb);
                            log(format!(
                                "ucp_after={}",
                                if (c.get_user_chunk_ptr)(p).is_null() { 0 } else { 1 }
                            ));
                            (c.set_keep_unknown_chunks)(p, keep, std::ptr::null(), 0);
                            log_hau(c, p);
                        },
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R26 — png_set_user_limits / chunk_cache_max / chunk_malloc_max
// ---------------------------------------------------------------------------

unsafe fn log_limits(c: &Core, p: Png, tag: &str) {
    log(format!(
        "{tag} wmax={} hmax={} cache={} malloc={}",
        (c.get_user_width_max)(p),
        (c.get_user_height_max)(p),
        (c.get_chunk_cache_max)(p),
        (c.get_chunk_malloc_max)(p)
    ));
}

#[test]
fn r26_user_limits() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    // a stream with a handful of ancillary chunks so that the chunk cache limit
    // has something to count
    let seed = 0x26_0001;
    let pre = vec![
        Chunk::new(b"saFe", vec![1, 2, 3]),
        Chunk::new(b"unSF", vec![4, 5]),
        Chunk::new(b"tEXt", b"A\0b".to_vec()),
        Chunk::new(b"moRe", vec![6]),
    ];
    let png = pngbuild::join(&chunks_of(w, h, 2, 8, 0, seed, pre, vec![]));
    let png_plain = mk(w, h, 6, 8, 0, seed ^ 0x9);

    // dimension limits: above, exactly at, and below the image size
    let dims: &[(u32, u32, &str)] = &[
        (0x7fff_ffff, 0x7fff_ffff, "max"),
        (w + 10, h + 10, "above"),
        (w, h, "exact"),
        (w - 1, h, "narrow"),
        (w, h - 1, "short"),
        (1, 1, "tiny"),
        (0, 0, "zero"),
    ];
    for &(uw, uh, dname) in dims {
        for (pg, pname) in [(&png, "chunks"), (&png_plain, "plain")] {
            case_pre(
                &format!("R26 dim={dname} w={uw} h={uh} in={pname}"),
                pg,
                w,
                h,
                |c, p, _i| unsafe {
                    log_limits(c, p, "before");
                    (c.set_user_limits)(p, uw, uh);
                    log_limits(c, p, "after");
                },
            );
        }
    }
    // chunk cache limit: one run per value
    for &n in &[0u32, 1, 2, 3, 4, 5, 1000] {
        case_pre(
            &format!("R26 cache={n}"),
            &png,
            w,
            h,
            |c, p, _i| unsafe {
                (c.set_chunk_cache_max)(p, n);
                log_limits(c, p, "cache");
            },
        );
    }
    // chunk malloc limit: one run per value
    for &n in &[0usize, 1, 2, 4, 16, 8_000_000] {
        case_pre(
            &format!("R26 malloc={n}"),
            &png,
            w,
            h,
            |c, p, _i| unsafe {
                (c.set_chunk_malloc_max)(p, n);
                log_limits(c, p, "malloc");
            },
        );
    }
    // all three at once, generous values
    case_pre("R26 all_generous", &png, w, h, |c, p, _i| unsafe {
        (c.set_user_limits)(p, 0x7fff_ffff, 0x7fff_ffff);
        (c.set_chunk_cache_max)(p, 1000);
        (c.set_chunk_malloc_max)(p, 8_000_000);
        log_limits(c, p, "all");
    });
}

// ---------------------------------------------------------------------------
// R27 — png_set_sig_bytes
// ---------------------------------------------------------------------------

#[test]
fn r27_sig_bytes() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    for &(ct, bd) in COMBOS {
        for n in 0..9usize {
            let seed = 0x27_0000 + (ct as u64) * 53 + (bd as u64) * 3 + n as u64;
            let full = mk(w, h, ct, bd, 0, seed);
            let cut = full[n.min(8)..].to_vec();
            case_pre(
                &format!("R27 ct={ct} bd={bd} n={n}"),
                &cut,
                w,
                h,
                |c, p, _i| unsafe {
                    (c.set_sig_bytes)(p, n as c_int);
                },
            );
        }
    }
    // png_set_sig_bytes(n) while the whole signature is still in the stream:
    // libpng then treats the first bytes of the signature as already consumed.
    for n in 1..8usize {
        let seed = 0x27_8000 + n as u64;
        let full = mk(w, h, 0, 8, 0, seed);
        case_pre(
            &format!("R27x n={n}"),
            &full,
            w,
            h,
            |c, p, _i| unsafe {
                (c.set_sig_bytes)(p, n as c_int);
            },
        );
    }
}

// ---------------------------------------------------------------------------
// R28 — png_start_read_image / png_read_update_info
// ---------------------------------------------------------------------------

#[test]
fn r28_start_read_image() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for seq in 0u8..6 {
                for tr in 0u8..3 {
                    let seed = 0x28_0000
                        + (ct as u64) * 67
                        + (bd as u64) * 5
                        + il as u64
                        + seq as u64 * 3
                        + tr as u64;
                    let png = mkpng(w, h, ct, bd, il, seed, false, None, true);
                    let o = Opts {
                        seq,
                        ..Default::default()
                    };
                    case_all(
                        &format!("R28 ct={ct} bd={bd} il={il} seq={seq} tr={tr}"),
                        &png,
                        w,
                        h,
                        o,
                        noset,
                        |c, p, _i| unsafe {
                            match tr {
                                1 => {
                                    (c.set_expand)(p);
                                    (c.set_gray_to_rgb)(p);
                                }
                                2 => {
                                    (c.set_strip_16)(p);
                                    (c.set_packing)(p);
                                    (c.set_bgr)(p);
                                }
                                _ => {}
                            }
                        },
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R29 — png_read_end
// ---------------------------------------------------------------------------

fn tIME_bytes() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&2024u16.to_be_bytes());
    v.extend_from_slice(&[6, 15, 12, 30, 45]);
    v
}

#[test]
fn r29_read_end() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    for &(ct, bd) in COMBOS {
        let seed = 0x29_0000 + (ct as u64) * 43 + bd as u64;
        let mut pre: Vec<Chunk> = Vec::new();
        if ct == 3 {
            pre.push(plte_chunk(bd, seed));
        }
        let post = vec![
            Chunk::new(b"tEXt", b"After\0IDAT text".to_vec()),
            Chunk::new(b"tIME", tIME_bytes()),
            Chunk::new(b"moRe", vec![0xde, 0xad]),
            Chunk::new(b"unSF", vec![0xbe, 0xef, 0x00]),
        ];
        let cs = chunks_of(w, h, ct, bd, 0, seed, pre.clone(), post.clone());
        let png = pngbuild::join(&cs);
        // trailing garbage after IEND
        let mut png_tail = png.clone();
        png_tail.extend_from_slice(&[0, 0, 0, 4, b'j', b'u', b'N', b'k', 1, 2, 3, 4, 9, 9, 9, 9]);
        // no post-IDAT chunks at all
        let png_bare = pngbuild::join(&chunks_of(w, h, ct, bd, 0, seed, pre, vec![]));
        for (pg, pname) in [
            (&png, "post"),
            (&png_tail, "post+tail"),
            (&png_bare, "bare"),
        ] {
            for ei in [false, true] {
                let o = Opts {
                    end_info: ei,
                    ..Default::default()
                };
                case_all(
                    &format!("R29 ct={ct} bd={bd} in={pname} end_info={ei}"),
                    pg,
                    w,
                    h,
                    o,
                    noset,
                    noset,
                );
            }
            // the same, with the unknown chunks kept
            for ei in [false, true] {
                let o = Opts {
                    end_info: ei,
                    ..Default::default()
                };
                case_all(
                    &format!("R29k ct={ct} bd={bd} in={pname} end_info={ei}"),
                    pg,
                    w,
                    h,
                    o,
                    |c, p, _i| unsafe {
                        (c.set_keep_unknown_chunks)(
                            p,
                            PNG_HANDLE_CHUNK_ALWAYS,
                            std::ptr::null(),
                            0,
                        );
                    },
                    noset,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R30 — transform stacking
// ---------------------------------------------------------------------------

const T_EXPAND: u32 = 1;
const T_G2RGB: u32 = 2;
const T_ADDA: u32 = 4;
const T_SWAP: u32 = 8;
const T_BGR: u32 = 16;
const T_STRIP16: u32 = 32;
const T_SCALE16: u32 = 64;
const T_SWAPA: u32 = 128;

fn apply30(c: &Core, p: Png, m: u32) {
    unsafe {
        if m & T_EXPAND != 0 {
            (c.set_expand)(p);
        }
        if m & T_G2RGB != 0 {
            (c.set_gray_to_rgb)(p);
        }
        if m & T_ADDA != 0 {
            (c.set_add_alpha)(p, 0xffff, PNG_FILLER_AFTER);
        }
        if m & T_SWAP != 0 {
            (c.set_swap)(p);
        }
        if m & T_BGR != 0 {
            (c.set_bgr)(p);
        }
        if m & T_STRIP16 != 0 {
            (c.set_strip_16)(p);
        }
        if m & T_SCALE16 != 0 {
            (c.set_scale_16)(p);
        }
        if m & T_SWAPA != 0 {
            (c.set_swap_alpha)(p);
        }
    }
}

#[test]
fn r30_transform_stacking() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    let full_a = T_EXPAND | T_G2RGB | T_ADDA | T_SWAP | T_BGR | T_STRIP16;
    let full_b = T_EXPAND | T_G2RGB | T_ADDA | T_SWAP | T_BGR | T_SCALE16 | T_SWAPA;
    let mut masks: Vec<(u32, String)> = vec![
        (full_a, "fullA".to_string()),
        (full_b, "fullB".to_string()),
    ];
    let mut r = Rng::new(0x30_1234);
    for k in 0..8 {
        let m = r.next_u32() & 0xff;
        masks.push((m, format!("rand{k}_{m:02x}")));
    }
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for trns in [false, true] {
                for (m, mname) in &masks {
                    let seed = 0x30_0000
                        + (ct as u64) * 59
                        + (bd as u64) * 7
                        + il as u64
                        + *m as u64
                        + trns as u64;
                    let png = mkpng(w, h, ct, bd, il, seed, false, None, trns);
                    case(
                        &format!("R30 ct={ct} bd={bd} il={il} trns={trns} m={mname}"),
                        &png,
                        w,
                        h,
                        |c, p, _i| apply30(c, p, *m),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R31 — png_create_read_struct_2 + user memory, allocation trace compared
// ---------------------------------------------------------------------------

fn r31_case(label: &str, png: &[u8], w: u32, h: u32, mask: u32) {
    let stride = stride_for(w);
    let mut buf = vec![0u8; h as usize * stride];
    let bp = buf.as_mut_ptr();
    diff(label, |lib| {
        session_reset(png.to_vec());
        let c = Core::new(lib);
        unsafe { std::ptr::write_bytes(bp, 0, h as usize * stride) };
        let rc = protected(|| unsafe {
            let p = (c.create_read_2)(
                VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                cb_error as Cb,
                cb_warning as Cb,
                std::ptr::null_mut(),
                cb_malloc as Cb,
                cb_free as Cb,
            );
            log(format!("create2={}", if p.is_null() { 0 } else { 1 }));
            if p.is_null() {
                return;
            }
            (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
            let i = (c.create_info)(p);
            log(format!("create_info={}", if i.is_null() { 0 } else { 1 }));
            // Only now start tracing: the sizes of png_struct / png_info
            // themselves are implementation details.
            with_session(|s| {
                s.trace_alloc = true;
                s.malloc_count = 0;
            });
            (c.set_read_fn)(p, std::ptr::null_mut(), cb_read as Cb);
            (c.read_info)(p, i);
            apply30(&c, p, mask);
            let passes = (c.set_interlace_handling)(p);
            log(format!("passes={passes}"));
            (c.read_update_info)(p, i);
            let rb = (c.get_rowbytes)(p, i);
            let n = loglen(w).min(stride);
            log(format!("rowbytes={rb}"));
            if rb + 8 <= n {
                for pass in 0..passes {
                    for y in 0..h {
                        let rp = bp.add(y as usize * stride);
                        (c.read_row)(p, rp, std::ptr::null_mut());
                        log(format!(
                            "p{pass}r{y}={}",
                            hex(std::slice::from_raw_parts(rp, n))
                        ));
                    }
                }
            }
            (c.read_end)(p, std::ptr::null_mut());
            let mut pp = p;
            let mut ii = i;
            (c.destroy_read)(&mut pp, &mut ii, std::ptr::null_mut());
            with_session(|s| s.trace_alloc = false);
            let live = with_session(|s| s.live_allocs);
            let cnt = with_session(|s| s.malloc_count);
            log(format!("live_allocs={live} malloc_count={cnt}"));
        });
        Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        }
    });
}

#[test]
fn r31_create_read_struct_2() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            let seed = 0x31_0000 + (ct as u64) * 37 + (bd as u64) * 3 + il as u64;
            let png = mkpng(w, h, ct, bd, il, seed, true, Some(45455), true);
            r31_case(
                &format!("R31 ct={ct} bd={bd} il={il}"),
                &png,
                w,
                h,
                0,
            );
        }
    }
    // with transforms, so that the extra row buffers / gamma tables show up
    for &(ct, bd) in &[(0u8, 1u8), (3, 4), (2, 16), (6, 16), (4, 8)] {
        let seed = 0x31_8000 + (ct as u64) * 11 + bd as u64;
        let png = mkpng(w, h, ct, bd, 0, seed, true, Some(45455), true);
        r31_case(
            &format!("R31t ct={ct} bd={bd}"),
            &png,
            w,
            h,
            T_EXPAND | T_G2RGB | T_ADDA | T_STRIP16,
        );
    }
    // a stream with unknown chunks (extra chunk-data allocations)
    {
        let png = r25_input(0x31_c000);
        r31_case("R31 unknown", &png, w, h, 0);
    }
}

// ---------------------------------------------------------------------------
// R32 — png_get_io_state / png_get_io_chunk_type
// ---------------------------------------------------------------------------

struct IoFns {
    state: unsafe extern "C" fn(Png) -> u32,
    chunk: unsafe extern "C" fn(Png) -> u32,
}

thread_local! {
    static IOFNS: Cell<*const IoFns> = const { Cell::new(std::ptr::null()) };
}

/// Read callback that polls the I/O state before serving the request.
unsafe extern "C" fn cb_read_io(png: *mut c_void, data: *mut u8, len: usize) {
    let f = IOFNS.with(|c| c.get());
    if !f.is_null() {
        let f = &*f;
        let st = (f.state)(png);
        let ct = (f.chunk)(png);
        log(format!(
            "IO len={len} state=0x{st:04x} chunk={}",
            chunk_str(ct)
        ));
    }
    cb_read(png, data, len);
}

fn r32_case(label: &str, png: &[u8], w: u32, h: u32) {
    let stride = stride_for(w);
    let mut buf = vec![0u8; h as usize * stride];
    let bp = buf.as_mut_ptr();
    diff(label, |lib| {
        let fns = IoFns {
            state: lib.f("png_get_io_state"),
            chunk: lib.f("png_get_io_chunk_type"),
        };
        IOFNS.with(|c| c.set(&fns as *const IoFns));
        unsafe { std::ptr::write_bytes(bp, 0, h as usize * stride) };
        let t = with_read(lib, png, &mut |c, p, i| unsafe {
            (c.set_read_fn)(p, std::ptr::null_mut(), cb_read_io as Cb);
            log(format!(
                "pre state=0x{:04x} chunk={}",
                (c.get_io_state)(p),
                chunk_str((c.get_io_chunk_type)(p))
            ));
            (c.read_info)(p, i);
            log(format!(
                "post_info state=0x{:04x} chunk={}",
                (c.get_io_state)(p),
                chunk_str((c.get_io_chunk_type)(p))
            ));
            let passes = (c.set_interlace_handling)(p);
            (c.read_update_info)(p, i);
            let rb = (c.get_rowbytes)(p, i);
            let n = loglen(w).min(stride);
            log(format!("passes={passes} rowbytes={rb}"));
            if rb + 8 <= n {
                for pass in 0..passes {
                    for y in 0..h {
                        let rp = bp.add(y as usize * stride);
                        (c.read_row)(p, rp, std::ptr::null_mut());
                        log(format!(
                            "p{pass}r{y}={} state=0x{:04x} chunk={}",
                            hex(std::slice::from_raw_parts(rp, n)),
                            (c.get_io_state)(p),
                            chunk_str((c.get_io_chunk_type)(p))
                        ));
                    }
                }
            }
            (c.read_end)(p, std::ptr::null_mut());
            log(format!(
                "post_end state=0x{:04x} chunk={}",
                (c.get_io_state)(p),
                chunk_str((c.get_io_chunk_type)(p))
            ));
        });
        IOFNS.with(|c| c.set(std::ptr::null()));
        t
    });
}

#[test]
fn r32_io_state() {
    ensure_libm();
    let (w, h) = (5u32, 3u32);
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            let seed = 0x32_0000 + (ct as u64) * 29 + (bd as u64) * 3 + il as u64;
            let png = mkpng(w, h, ct, bd, il, seed, true, Some(45455), true);
            r32_case(&format!("R32 ct={ct} bd={bd} il={il}"), &png, w, h);
        }
    }
    // a stream with unknown / post-IDAT chunks so that more chunk types appear
    {
        let png = r25_input(0x32_8000);
        r32_case("R32 unknown", &png, w, h);
    }
    {
        let seed = 0x32_9000;
        let cs = chunks_of(
            w,
            h,
            2,
            8,
            0,
            seed,
            vec![Chunk::new(b"tEXt", b"K\0v".to_vec())],
            vec![Chunk::new(b"tIME", tIME_bytes())],
        );
        r32_case("R32 text+time", &pngbuild::join(&cs), w, h);
    }
    // multiple IDAT chunks
    {
        let seed = 0x32_a000;
        let b = Builder::new(w, h, 8, 6);
        let png = b.build(&b.raw_rows(seed), 7);
        r32_case("R32 multi_idat", &png, w, h);
    }
}

// ---------------------------------------------------------------------------
// R33 — png_get_current_row_number / png_get_current_pass_number
// ---------------------------------------------------------------------------

#[test]
fn r33_current_row_pass() {
    ensure_libm();
    let o = Opts {
        rownum: true,
        ..Default::default()
    };
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for &(w, h) in &[(1u32, 1u32), (5u32, 5u32), (9u32, 8u32)] {
                let seed = 0x33_0000
                    + (ct as u64) * 23
                    + (bd as u64) * 3
                    + il as u64
                    + w as u64 * 5
                    + h as u64;
                let png = mk(w, h, ct, bd, il, seed);
                case_all(
                    &format!("R33 ct={ct} bd={bd} il={il} w={w} h={h}"),
                    &png,
                    w,
                    h,
                    o,
                    noset,
                    noset,
                );
            }
        }
    }
    // with transforms in the pipeline
    for &(ct, bd) in &[(0u8, 2u8), (3, 4), (2, 16), (6, 8)] {
        for il in [0u8, 1] {
            let seed = 0x33_8000 + (ct as u64) * 7 + bd as u64 + il as u64;
            let png = mkpng(5, 5, ct, bd, il, seed, false, None, true);
            case_all(
                &format!("R33t ct={ct} bd={bd} il={il}"),
                &png,
                5,
                5,
                o,
                noset,
                |c, p, _i| unsafe {
                    (c.set_expand)(p);
                    (c.set_gray_to_rgb)(p);
                },
            );
        }
    }
}



