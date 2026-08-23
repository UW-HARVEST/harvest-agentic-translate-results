//! Sequential-read differential tests, CONFIGS.md rows R1..R16.
//!
//! Every test builds its input PNG with `support::pngbuild` (independent of
//! both libraries), then drives the C `.so` and the Rust `.so` through the
//! identical call sequence on those identical bytes and compares the complete
//! trace byte for byte.
mod support;

use std::ffi::c_int;
use support::core::*;
use support::pngbuild::{self, Builder};
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

/// Bytes of the row buffer logged past `rowbytes`, so that an overrun in
/// either library becomes visible in the trace.
const SLACK: usize = 16;

/// Seed offsets: every configuration is exercised with several inputs.
const SEEDS: &[u64] = &[0, 0x9e37_79b9];

/// The reference C `libpng.so` (`c_src/CMakeLists.txt`) is linked without
/// `-lm`, so its `floor`/`pow` references stay unresolved and the *first*
/// floating-point entry point that is called (e.g. `png_set_rgb_to_gray`,
/// which calls `png_fixed` -> `floor`) aborts the process with
/// "symbol lookup error: undefined symbol: floor".  Loading libm into the
/// global symbol scope makes the lazy binding resolvable.  This changes symbol
/// resolution only, and both libraries end up using the same libm.
fn ensure_libm() {
    use std::sync::OnceLock;
    static LIBM: OnceLock<libloading::os::unix::Library> = OnceLock::new();
    LIBM.get_or_init(|| unsafe {
        // RTLD_NOW | RTLD_GLOBAL on Linux
        libloading::os::unix::Library::open(Some("libm.so.6"), 0x2 | 0x100)
            .expect("dlopen libm.so.6")
    });
}

fn palette_for(bd: u8, seed: u64) -> Vec<u8> {
    let n = 1usize << bd; // every index of a `bd`-bit image is in range
    let mut r = Rng::new(seed);
    (0..3 * n).map(|_| r.byte()).collect()
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
        3 => {
            let n = 1usize << bd;
            r.bytes(n)
        }
        // Illegal for GRAY_ALPHA/RGB_ALPHA: libpng reports a benign error.
        _ => vec![0x12, 0x34],
    }
}

/// A legal sBIT chunk payload.  For palette images the sample depth is 8, but
/// `png_set_shift` (which `png_read_png` calls for `PNG_TRANSFORM_SHIFT`)
/// validates against the *index* depth, so `big` selects between a payload
/// that `png_set_shift` accepts and one it rejects.
fn sbit_for(ct: u8, bd: u8, big: bool) -> Vec<u8> {
    let v = if bd > 1 { bd - 1 } else { 1 };
    match ct {
        0 => vec![v],
        2 => vec![v, v, v],
        3 => {
            if big {
                vec![8, 7, 6]
            } else {
                vec![bd, bd.saturating_sub(1).max(1), bd.saturating_sub(2).max(1)]
            }
        }
        4 => vec![v, v],
        _ => vec![v, v, v, v],
    }
}

/// Header chunks in canonical order: sBIT, gAMA, PLTE, tRNS.
fn base(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64, trns: bool, sbit: bool) -> Builder {
    let mut b = Builder::new(w, h, bd, ct).interlace(il);
    if sbit {
        b = b.add(b"sBIT", sbit_for(ct, bd, false));
        b = b.add(b"gAMA", 45455u32.to_be_bytes().to_vec());
    }
    if ct == 3 {
        b = b.add(b"PLTE", palette_for(bd, seed ^ 0x5eed_1234));
    }
    if trns {
        b = b.add(b"tRNS", trns_for(ct, bd, seed ^ 0x7a17_9999));
    }
    b
}

/// Valid PNG, all row filters 0.
fn mk(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    base(w, h, ct, bd, il, seed, false, false).build_valid(seed)
}

/// Valid PNG carrying a tRNS chunk.
fn mk_trns(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    base(w, h, ct, bd, il, seed, true, false).build_valid(seed)
}

/// Valid PNG carrying sBIT + gAMA + tRNS.
fn mk_rich(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    base(w, h, ct, bd, il, seed, true, true).build_valid(seed)
}

/// Valid PNG whose (legal) sBIT chunk carries values `png_set_shift` refuses.
fn mk_bigsbit(w: u32, h: u32, ct: u8, bd: u8, seed: u64) -> Vec<u8> {
    let mut b = Builder::new(w, h, bd, ct);
    b = b.add(b"sBIT", sbit_for(ct, bd, true));
    if ct == 3 {
        b = b.add(b"PLTE", palette_for(bd, seed ^ 0x5eed_1234));
    }
    b.build_valid(seed)
}

/// Raw pre-compression stream whose row filter bytes cycle through 0..4.
fn raw_filters(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    let mut out = Vec::new();
    let mut fi = 0u8;
    if il == 0 {
        let rb = pngbuild::rowbytes(ct, bd, w);
        for _ in 0..h {
            out.push(fi % 5);
            fi = fi.wrapping_add(1);
            for _ in 0..rb {
                out.push(r.byte());
            }
        }
    } else {
        for p in 0..7 {
            let pw = pngbuild::pass_width(w, p);
            let ph = pngbuild::pass_height(h, p);
            if pw == 0 || ph == 0 {
                continue;
            }
            let rb = pngbuild::rowbytes(ct, bd, pw);
            for _ in 0..ph {
                out.push(fi % 5);
                fi = fi.wrapping_add(1);
                for _ in 0..rb {
                    out.push(r.byte());
                }
            }
        }
    }
    out
}

/// Valid PNG exercising all five row filter types.
fn mk_filters(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    let b = base(w, h, ct, bd, il, seed, false, false);
    let raw = raw_filters(w, h, ct, bd, il, seed ^ 0xf117_e000);
    b.build(&raw, 0)
}

/// Non-interlaced RGB/RGBA raw stream (filter 0) whose pixels are all grey.
fn raw_grey_rgb(w: u32, h: u32, ct: u8, bd: u8, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    let ch = if ct == 6 { 4 } else { 3 };
    let mut out = Vec::new();
    for _ in 0..h {
        out.push(0u8);
        for _ in 0..w {
            if bd == 8 {
                let v = r.byte();
                for k in 0..ch {
                    if k < 3 {
                        out.push(v);
                    } else {
                        out.push(r.byte());
                    }
                }
            } else {
                let hi = r.byte();
                let lo = r.byte();
                for k in 0..ch {
                    if k < 3 {
                        out.push(hi);
                        out.push(lo);
                    } else {
                        out.push(r.byte());
                        out.push(r.byte());
                    }
                }
            }
        }
    }
    out
}

fn mk_grey_rgb(w: u32, h: u32, ct: u8, bd: u8, seed: u64) -> Vec<u8> {
    let b = base(w, h, ct, bd, 0, seed, false, false);
    let raw = raw_grey_rgb(w, h, ct, bd, seed);
    b.build(&raw, 0)
}

// ---------------------------------------------------------------------------
// drivers
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Copy)]
struct Opts {
    /// log `png_get_rgb_to_gray_status` after every row
    status: bool,
    /// log `png_get_current_pass_number` / `png_get_current_row_number`
    rownum: bool,
}

fn stride_for(w: u32) -> usize {
    // worst case after transforms: 4 channels * 2 bytes per pixel
    w as usize * 8 + SLACK + 24
}

/// read_info -> transforms -> interlace handling -> read_update_info -> rows.
fn drive(
    lib: &Lib,
    png: &[u8],
    h: u32,
    stride: usize,
    bp: *mut u8,
    o: Opts,
    set: &mut dyn FnMut(&Core, Png, Info),
) -> Trace {
    unsafe { std::ptr::write_bytes(bp, 0, h as usize * stride) };
    with_read(lib, png, &mut |c, p, i| unsafe {
        (c.read_info)(p, i);
        set(c, p, i);
        let passes = (c.set_interlace_handling)(p);
        log(format!("passes={passes}"));
        (c.read_update_info)(p, i);
        log_all_info(c, p, i);
        let rb = (c.get_rowbytes)(p, i);
        log(format!(
            "rowbytes_after_update={rb} channels={} depth={} color={}",
            (c.get_channels)(p, i),
            (c.get_bit_depth)(p, i),
            (c.get_color_type)(p, i)
        ));
        if rb + SLACK > stride {
            log(format!("BUF_TOO_SMALL rb={rb} stride={stride}"));
            return;
        }
        for pass in 0..passes {
            for r in 0..h {
                let rp = bp.add(r as usize * stride);
                (c.read_row)(p, rp, std::ptr::null_mut());
                log(format!(
                    "p{pass}r{r}={} slack={}",
                    hex(std::slice::from_raw_parts(rp, rb)),
                    hex(std::slice::from_raw_parts(rp.add(rb), SLACK))
                ));
                if o.status {
                    log(format!("rgb_to_gray_status={}", (c.get_rgb_to_gray_status)(p)));
                }
                if o.rownum {
                    log(format!(
                        "pass_no={} row_no={}",
                        (c.get_current_pass_number)(p),
                        (c.get_current_row_number)(p)
                    ));
                }
            }
        }
        (c.read_end)(p, std::ptr::null_mut());
    })
}

fn case_opts<F: FnMut(&Core, Png, Info)>(
    label: &str,
    png: &[u8],
    w: u32,
    h: u32,
    o: Opts,
    mut set: F,
) {
    let stride = stride_for(w);
    let mut buf = vec![0u8; h as usize * stride];
    let bp = buf.as_mut_ptr();
    diff(label, |lib| drive(lib, png, h, stride, bp, o, &mut set));
}

fn case<F: FnMut(&Core, Png, Info)>(label: &str, png: &[u8], w: u32, h: u32, set: F) {
    case_opts(label, png, w, h, Opts::default(), set)
}

fn noset(_c: &Core, _p: Png, _i: Info) {}

fn ptr_array(base: *mut u8, h: u32, stride: usize) -> Vec<*mut u8> {
    (0..h as usize)
        .map(|y| unsafe { base.add(y * stride) })
        .collect()
}

/// `png_read_rows` driver.  `mode`: 0 = row only, 1 = display only, 2 = both.
/// `one`: one row per call instead of a whole pass per call.
#[allow(clippy::too_many_arguments)]
fn drive_rows(
    lib: &Lib,
    png: &[u8],
    h: u32,
    stride: usize,
    b1: *mut u8,
    b2: *mut u8,
    a1: *mut *mut u8,
    a2: *mut *mut u8,
    mode: u8,
    one: bool,
) -> Trace {
    unsafe {
        std::ptr::write_bytes(b1, 0, h as usize * stride);
        std::ptr::write_bytes(b2, 0, h as usize * stride);
    }
    with_read(lib, png, &mut |c, p, i| unsafe {
        (c.read_info)(p, i);
        let passes = (c.set_interlace_handling)(p);
        (c.read_update_info)(p, i);
        log_all_info(c, p, i);
        let rb = (c.get_rowbytes)(p, i);
        log(format!("passes={passes} rowbytes_after_update={rb}"));
        if rb + SLACK > stride {
            log(format!("BUF_TOO_SMALL rb={rb} stride={stride}"));
            return;
        }
        let rr = if mode == 1 { std::ptr::null_mut() } else { a1 };
        let dd = if mode == 0 { std::ptr::null_mut() } else { a2 };
        for pass in 0..passes {
            if one {
                for y in 0..h as usize {
                    let r2 = if rr.is_null() {
                        std::ptr::null_mut()
                    } else {
                        rr.add(y)
                    };
                    let d2 = if dd.is_null() {
                        std::ptr::null_mut()
                    } else {
                        dd.add(y)
                    };
                    (c.read_rows)(p, r2, d2, 1);
                }
            } else {
                (c.read_rows)(p, rr, dd, h);
            }
            for y in 0..h as usize {
                let r1 = b1.add(y * stride);
                let r2 = b2.add(y * stride);
                log(format!(
                    "p{pass}r{y} row={} rs={} dsp={} ds={}",
                    hex(std::slice::from_raw_parts(r1, rb)),
                    hex(std::slice::from_raw_parts(r1.add(rb), SLACK)),
                    hex(std::slice::from_raw_parts(r2, rb)),
                    hex(std::slice::from_raw_parts(r2.add(rb), SLACK))
                ));
            }
        }
        (c.read_end)(p, std::ptr::null_mut());
    })
}

fn case_rows(label: &str, png: &[u8], w: u32, h: u32, mode: u8, one: bool) {
    let stride = stride_for(w);
    let mut buf1 = vec![0u8; h as usize * stride];
    let mut buf2 = vec![0u8; h as usize * stride];
    let p1 = buf1.as_mut_ptr();
    let p2 = buf2.as_mut_ptr();
    let mut arr1 = ptr_array(p1, h, stride);
    let mut arr2 = ptr_array(p2, h, stride);
    let a1 = arr1.as_mut_ptr();
    let a2 = arr2.as_mut_ptr();
    diff(label, |lib| {
        drive_rows(lib, png, h, stride, p1, p2, a1, a2, mode, one)
    });
}

/// `png_read_image` driver.  `pre`: 0 = nothing, 1 = interlace handling +
/// update_info, 2 = update_info only (the documented "should be turned on"
/// warning path).
#[allow(clippy::too_many_arguments)]
fn drive_image(
    lib: &Lib,
    png: &[u8],
    h: u32,
    stride: usize,
    bp: *mut u8,
    ap: *mut *mut u8,
    pre: u8,
) -> Trace {
    unsafe { std::ptr::write_bytes(bp, 0, h as usize * stride) };
    with_read(lib, png, &mut |c, p, i| unsafe {
        (c.read_info)(p, i);
        if pre == 1 {
            let n = (c.set_interlace_handling)(p);
            log(format!("passes={n}"));
        }
        if pre != 0 {
            (c.read_update_info)(p, i);
        }
        log_all_info(c, p, i);
        let rb = (c.get_rowbytes)(p, i);
        log(format!("rowbytes={rb} channels={}", (c.get_channels)(p, i)));
        if rb + SLACK > stride {
            log(format!("BUF_TOO_SMALL rb={rb} stride={stride}"));
            return;
        }
        (c.read_image)(p, ap);
        for y in 0..h as usize {
            let rp = bp.add(y * stride);
            log(format!(
                "row{y}={} slack={}",
                hex(std::slice::from_raw_parts(rp, rb)),
                hex(std::slice::from_raw_parts(rp.add(rb), SLACK))
            ));
        }
        (c.read_end)(p, std::ptr::null_mut());
    })
}

fn case_image(label: &str, png: &[u8], w: u32, h: u32, pre: u8) {
    let stride = stride_for(w);
    let mut buf = vec![0u8; h as usize * stride];
    let bp = buf.as_mut_ptr();
    let mut arr = ptr_array(bp, h, stride);
    let ap = arr.as_mut_ptr();
    diff(label, |lib| drive_image(lib, png, h, stride, bp, ap, pre));
}

/// `png_read_png` driver: the rows are libpng's own buffers, fetched with
/// `png_get_rows`.
///
/// `png_read_png` allocates the row buffers with `png_malloc` (uninitialised)
/// and `png_combine_row` deliberately *preserves* the bits of the last byte of
/// a row that lie beyond the image width.  Those padding bits are therefore
/// uninitialised heap in both libraries, so they are masked out of the trace
/// here (the mask itself is logged).  Padding-bit preservation is checked
/// deterministically by R1/R5/R6/R7, which read into caller buffers that this
/// file zero-fills first.
fn case_read_png(label: &str, png: &[u8], tr: c_int) {
    diff(label, |lib| {
        with_read(lib, png, &mut |c, p, i| unsafe {
            (c.read_png)(p, i, tr, std::ptr::null_mut());
            log_all_info(c, p, i);
            let rb = (c.get_rowbytes)(p, i);
            let h = (c.get_image_height)(p, i);
            let w = (c.get_image_width)(p, i) as usize;
            let pd = (c.get_channels)(p, i) as usize * (c.get_bit_depth)(p, i) as usize;
            log(format!(
                "rowbytes={rb} height={h} channels={} depth={} color={}",
                (c.get_channels)(p, i),
                (c.get_bit_depth)(p, i),
                (c.get_color_type)(p, i)
            ));
            let m = (pd * w) & 7;
            let keep: u8 = if m == 0 {
                0xff
            } else if tr & PNG_TRANSFORM_PACKSWAP != 0 {
                !(0xffu8 << m)
            } else {
                !(0xffu8 >> m)
            };
            log(format!("last_byte_mask={keep:02x}"));
            let rows = (c.get_rows)(p, i);
            log(format!("rows_null={}", rows.is_null() as u8));
            if !rows.is_null() {
                for y in 0..h as usize {
                    let rp = *rows.add(y);
                    if rp.is_null() {
                        log(format!("row{y}=<null>"));
                    } else if rb == 0 {
                        log(format!("row{y}="));
                    } else {
                        let s = std::slice::from_raw_parts(rp, rb);
                        log(format!(
                            "row{y}={}{:02x}",
                            hex(&s[..rb - 1]),
                            s[rb - 1] & keep
                        ));
                    }
                }
            }
        })
    });
}

// ---------------------------------------------------------------------------
// R1 — png_read_info + png_read_row, all combos, no transforms
// ---------------------------------------------------------------------------

#[test]
fn r1_read_row_all_combos() {
    ensure_libm();
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for &w in &[1u32, 3, 7, 8, 17] {
                for &h in &[1u32, 5] {
                    for &sx in SEEDS {
                        let seed = 0x1000
                            + (ct as u64) * 97
                            + (bd as u64) * 13
                            + w as u64 * 3
                            + h as u64
                            + sx;
                        let png = mk(w, h, ct, bd, il, seed);
                        case(
                            &format!("R1 ct={ct} bd={bd} il={il} w={w} h={h} s={sx} f0"),
                            &png,
                            w,
                            h,
                            noset,
                        );
                        if h > 1 {
                            // all five row filter types
                            let png = mk_filters(w, h, ct, bd, il, seed ^ 0xabc);
                            case(
                                &format!("R1 ct={ct} bd={bd} il={il} w={w} h={h} s={sx} filters"),
                                &png,
                                w,
                                h,
                                noset,
                            );
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R2 — png_read_rows
// ---------------------------------------------------------------------------

#[test]
fn r2_read_rows() {
    ensure_libm();
    let (w, h) = (7u32, 5u32);
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for mode in 0u8..3 {
                for one in [false, true] {
                    for &sx in SEEDS {
                        let seed = 0x2000 + (ct as u64) * 31 + (bd as u64) * 7 + il as u64 + sx;
                        let png = mk(w, h, ct, bd, il, seed);
                        case_rows(
                            &format!("R2 ct={ct} bd={bd} il={il} mode={mode} one={one} s={sx}"),
                            &png,
                            w,
                            h,
                            mode,
                            one,
                        );
                    }
                }
            }
        }
    }
    // a second input shape with all five filter types
    for &(ct, bd) in COMBOS {
        let png = mk_filters(3, 4, ct, bd, 1, 0x2bcd + ct as u64);
        case_rows(&format!("R2f ct={ct} bd={bd}"), &png, 3, 4, 2, true);
    }
}

// ---------------------------------------------------------------------------
// R3 — png_read_image
// ---------------------------------------------------------------------------

#[test]
fn r3_read_image() {
    ensure_libm();
    let (w, h) = (7u32, 5u32);
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for pre in [0u8, 1] {
                for seed in [0x3000u64, 0x3777] {
                    let s = seed + (ct as u64) * 41 + (bd as u64) * 5 + il as u64;
                    let png = mk(w, h, ct, bd, il, s);
                    case_image(
                        &format!("R3 ct={ct} bd={bd} il={il} pre={pre} seed={s}"),
                        &png,
                        w,
                        h,
                        pre,
                    );
                }
            }
        }
        // read_update_info without interlace handling: libpng warns and fixes up
        let png = mk(w, h, ct, bd, 1, 0x3aaa + ct as u64);
        case_image(&format!("R3warn ct={ct} bd={bd}"), &png, w, h, 2);
    }
}

// ---------------------------------------------------------------------------
// R4 — png_read_png with every transform bit it honours
// ---------------------------------------------------------------------------

/// The read transforms `png_read_png` acts on (see `png_read_png` in
/// `c_src/src/pngread.c`).
const RP_BITS: &[(c_int, &str)] = &[
    (PNG_TRANSFORM_IDENTITY, "IDENTITY"),
    (PNG_TRANSFORM_STRIP_16, "STRIP_16"),
    (PNG_TRANSFORM_STRIP_ALPHA, "STRIP_ALPHA"),
    (PNG_TRANSFORM_PACKING, "PACKING"),
    (PNG_TRANSFORM_PACKSWAP, "PACKSWAP"),
    (PNG_TRANSFORM_EXPAND, "EXPAND"),
    (PNG_TRANSFORM_INVERT_MONO, "INVERT_MONO"),
    (PNG_TRANSFORM_SHIFT, "SHIFT"),
    (PNG_TRANSFORM_BGR, "BGR"),
    (PNG_TRANSFORM_SWAP_ALPHA, "SWAP_ALPHA"),
    (PNG_TRANSFORM_SWAP_ENDIAN, "SWAP_ENDIAN"),
    (PNG_TRANSFORM_INVERT_ALPHA, "INVERT_ALPHA"),
    (PNG_TRANSFORM_GRAY_TO_RGB, "GRAY_TO_RGB"),
    (PNG_TRANSFORM_EXPAND_16, "EXPAND_16"),
    (PNG_TRANSFORM_SCALE_16, "SCALE_16"),
];

#[test]
fn r4_read_png_transforms() {
    ensure_libm();
    let h = 3u32;
    // w=8 is byte-aligned at every bit depth (nothing is masked); w=5 leaves a
    // sub-byte remainder for the 1/2/4-bit formats.
    for &(bit, name) in RP_BITS {
        for &(ct, bd) in COMBOS {
            for &w in &[8u32, 5] {
                // rich input: sBIT (for SHIFT), gAMA, tRNS (for EXPAND)
                let png = mk_rich(w, h, ct, bd, 0, 0x4000 + (ct as u64) * 11 + bd as u64 + w as u64);
                case_read_png(
                    &format!("R4 tr={name} ct={ct} bd={bd} w={w} il=0"),
                    &png,
                    bit,
                );
            }
        }
    }
    // PNG_TRANSFORM_SHIFT with an sBIT payload png_set_shift rejects (the
    // palette sample depth is 8 but it validates against the index depth)
    for &(ct, bd) in &[(3u8, 1u8), (3, 2), (3, 4), (3, 8), (0, 8), (2, 16)] {
        let png = mk_bigsbit(8, h, ct, bd, 0x4700 + (ct as u64) * 13 + bd as u64);
        case_read_png(
            &format!("R4 tr=SHIFT-bigsbit ct={ct} bd={bd}"),
            &png,
            PNG_TRANSFORM_SHIFT,
        );
    }
    // seeded random combinations
    let mut r = Rng::new(0x4444_1111);
    for k in 0..8 {
        let nb = 2 + r.below(3) as usize;
        let mut m: c_int = 0;
        for _ in 0..nb {
            m |= RP_BITS[r.below(RP_BITS.len() as u32) as usize].0;
        }
        for &(ct, bd) in COMBOS {
            for &w in &[8u32, 5] {
                let png =
                    mk_rich(w, h, ct, bd, 1, 0x4500 + (ct as u64) * 7 + bd as u64 + k + w as u64);
                case_read_png(
                    &format!("R4 rand{k}=0x{m:04x} ct={ct} bd={bd} w={w} il=1"),
                    &png,
                    m,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R5 — png_set_interlace_handling, all 7 passes, every combo
// ---------------------------------------------------------------------------

#[test]
fn r5_interlace_handling() {
    ensure_libm();
    let o = Opts {
        rownum: true,
        ..Default::default()
    };
    for &(ct, bd) in COMBOS {
        for &(w, h) in &[(1u32, 1u32), (2, 3), (5, 8), (17, 7)] {
            for &sx in SEEDS {
                let seed = 0x5000 + (ct as u64) * 23 + (bd as u64) * 3 + w as u64 + sx;
                let png = mk(w, h, ct, bd, 1, seed);
                case_opts(
                    &format!("R5 ct={ct} bd={bd} w={w} h={h} s={sx} f0"),
                    &png,
                    w,
                    h,
                    o,
                    noset,
                );
                if h > 1 {
                    let png = mk_filters(w, h, ct, bd, 1, seed ^ 0x99);
                    case_opts(
                        &format!("R5 ct={ct} bd={bd} w={w} h={h} s={sx} filters"),
                        &png,
                        w,
                        h,
                        o,
                        noset,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R6 — png_set_packing
// ---------------------------------------------------------------------------

const SUBBYTE: &[(u8, u8)] = &[(0, 1), (0, 2), (0, 4), (3, 1), (3, 2), (3, 4)];

#[test]
fn r6_packing() {
    ensure_libm();
    for &(ct, bd) in SUBBYTE {
        for &w in &[1u32, 3, 7, 8, 17] {
            for il in [0u8, 1] {
                for seed in [0x6000u64, 0x6abc] {
                    let s = seed + (ct as u64) * 17 + bd as u64 + w as u64;
                    let png = mk(w, 4, ct, bd, il, s);
                    case(
                        &format!("R6 ct={ct} bd={bd} w={w} il={il} seed={s}"),
                        &png,
                        w,
                        4,
                        |c, p, _i| unsafe { (c.set_packing)(p) },
                    );
                }
            }
        }
    }
    // png_set_packing on inputs where it must do nothing
    for &(ct, bd) in COMBOS {
        if bd < 8 {
            continue;
        }
        let png = mk(5, 2, ct, bd, 0, 0x6def + ct as u64);
        case(
            &format!("R6noop ct={ct} bd={bd}"),
            &png,
            5,
            2,
            |c, p, _i| unsafe { (c.set_packing)(p) },
        );
    }
}

// ---------------------------------------------------------------------------
// R7 — png_set_packswap (alone and combined with packing)
// ---------------------------------------------------------------------------

#[test]
fn r7_packswap() {
    ensure_libm();
    for &(ct, bd) in SUBBYTE {
        for &w in &[1u32, 3, 7, 17] {
            for il in [0u8, 1] {
                for variant in 0u8..3 {
                  for &sx in SEEDS {
                    let s = 0x7000 + (ct as u64) * 19 + bd as u64 + w as u64 * 5 + il as u64 + sx;
                    let png = mk(w, 4, ct, bd, il, s);
                    case(
                        &format!("R7 ct={ct} bd={bd} w={w} il={il} v={variant} s={sx}"),
                        &png,
                        w,
                        4,
                        |c, p, _i| unsafe {
                            match variant {
                                0 => (c.set_packswap)(p),
                                1 => {
                                    (c.set_packing)(p);
                                    (c.set_packswap)(p);
                                }
                                _ => {
                                    (c.set_packswap)(p);
                                    (c.set_packing)(p);
                                }
                            }
                        },
                    );
                  }
                }
            }
        }
    }
    for &(ct, bd) in COMBOS {
        if bd < 8 {
            continue;
        }
        let png = mk(5, 2, ct, bd, 0, 0x7fff + ct as u64);
        case(
            &format!("R7noop ct={ct} bd={bd}"),
            &png,
            5,
            2,
            |c, p, _i| unsafe { (c.set_packswap)(p) },
        );
    }
}

// ---------------------------------------------------------------------------
// R8 — png_set_expand
// ---------------------------------------------------------------------------

#[test]
fn r8_expand() {
    ensure_libm();
    for &(ct, bd) in COMBOS {
        for trns in [false, true] {
            for &w in &[3u32, 8] {
                for il in [0u8, 1] {
                    for &sx in SEEDS {
                        let s = 0x8000
                            + (ct as u64) * 29
                            + (bd as u64) * 3
                            + w as u64
                            + il as u64
                            + sx;
                        let png = if trns {
                            mk_trns(w, 3, ct, bd, il, s)
                        } else {
                            mk(w, 3, ct, bd, il, s)
                        };
                        case(
                            &format!("R8 ct={ct} bd={bd} w={w} il={il} trns={trns} s={sx}"),
                            &png,
                            w,
                            3,
                            |c, p, _i| unsafe { (c.set_expand)(p) },
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R9 — png_set_expand_16
// ---------------------------------------------------------------------------

#[test]
fn r9_expand_16() {
    ensure_libm();
    for &(ct, bd) in COMBOS {
        for trns in [false, true] {
            for variant in 0u8..2 {
                for il in [0u8, 1] {
                    for &sx in SEEDS {
                        let s = 0x9000 + (ct as u64) * 37 + (bd as u64) * 5 + il as u64 + sx;
                        let png = if trns {
                            mk_trns(7, 3, ct, bd, il, s)
                        } else {
                            mk(7, 3, ct, bd, il, s)
                        };
                        case(
                            &format!("R9 ct={ct} bd={bd} il={il} trns={trns} v={variant} s={sx}"),
                            &png,
                            7,
                            3,
                            |c, p, _i| unsafe {
                                if variant == 1 {
                                    (c.set_expand)(p);
                                }
                                (c.set_expand_16)(p);
                            },
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R10 — the three fine-grained expand entry points
// ---------------------------------------------------------------------------

#[test]
fn r10_expand_parts() {
    ensure_libm();
    for which in 0u8..3 {
        for &(ct, bd) in COMBOS {
            for trns in [false, true] {
              for &sx in SEEDS {
                let s = 0xa000 + (ct as u64) * 43 + (bd as u64) * 7 + which as u64 + sx;
                let png = if trns {
                    mk_trns(7, 3, ct, bd, 0, s)
                } else {
                    mk(7, 3, ct, bd, 0, s)
                };
                case(
                    &format!("R10 which={which} ct={ct} bd={bd} trns={trns} s={sx}"),
                    &png,
                    7,
                    3,
                    |c, p, _i| unsafe {
                        match which {
                            0 => (c.set_expand_gray_1_2_4_to_8)(p),
                            1 => (c.set_palette_to_rgb)(p),
                            _ => (c.set_tRNS_to_alpha)(p),
                        }
                    },
                );
              }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R11 — png_set_gray_to_rgb
// ---------------------------------------------------------------------------

#[test]
fn r11_gray_to_rgb() {
    ensure_libm();
    let combos: &[(u8, u8)] = &[(0, 1), (0, 2), (0, 4), (0, 8), (0, 16), (4, 8), (4, 16)];
    for &(ct, bd) in combos {
        for trns in [false, true] {
            for il in [0u8, 1] {
                for &w in &[1u32, 7, 17] {
                    for &sx in SEEDS {
                        let s = 0xb000 + (ct as u64) * 53 + (bd as u64) * 3 + w as u64 + sx;
                        let png = if trns {
                            mk_trns(w, 3, ct, bd, il, s)
                        } else {
                            mk(w, 3, ct, bd, il, s)
                        };
                        case(
                            &format!("R11 ct={ct} bd={bd} w={w} il={il} trns={trns} s={sx}"),
                            &png,
                            w,
                            3,
                            |c, p, _i| unsafe { (c.set_gray_to_rgb)(p) },
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R12 — png_set_rgb_to_gray / _fixed
// ---------------------------------------------------------------------------

#[test]
fn r12_rgb_to_gray() {
    ensure_libm();
    let coeffs: &[(f64, f64, i32, i32, &str)] = &[
        (-1.0, -1.0, -1, -1, "default"),
        (0.2125, 0.7154, 21250, 71540, "bt709"),
        (0.5, 0.5, 50000, 50000, "half"),
        (0.0, 0.0, 0, 0, "zero"),
        (1.0, 0.0, 100000, 0, "allred"),
    ];
    let combos: &[(u8, u8)] = &[(2, 8), (2, 16), (6, 8), (6, 16)];
    let o = Opts {
        status: true,
        ..Default::default()
    };
    for &(ct, bd) in combos {
        for grey in [false, true] {
          for &sx in SEEDS {
            let s = 0xc000 + (ct as u64) * 61 + bd as u64 + sx;
            let png = if grey {
                mk_grey_rgb(4, 2, ct, bd, s)
            } else {
                mk(4, 2, ct, bd, 0, s)
            };
            for &action in &[
                PNG_ERROR_ACTION_NONE,
                PNG_ERROR_ACTION_WARN,
                PNG_ERROR_ACTION_ERROR,
            ] {
                for &(rf, gf, ri, gi, cname) in coeffs {
                    case_opts(
                        &format!(
                            "R12 fp ct={ct} bd={bd} grey={grey} s={sx} act={action} coef={cname}"
                        ),
                        &png,
                        4,
                        2,
                        o,
                        |c, p, _i| unsafe { (c.set_rgb_to_gray)(p, action, rf, gf) },
                    );
                    case_opts(
                        &format!(
                            "R12 fx ct={ct} bd={bd} grey={grey} s={sx} act={action} coef={cname}"
                        ),
                        &png,
                        4,
                        2,
                        o,
                        |c, p, _i| unsafe { (c.set_rgb_to_gray_fixed)(p, action, ri, gi) },
                    );
                }
            }
          }
        }
    }
}

// ---------------------------------------------------------------------------
// R13 — png_set_strip_16 / png_set_scale_16
// ---------------------------------------------------------------------------

#[test]
fn r13_strip_scale_16() {
    ensure_libm();
    let combos: &[(u8, u8)] = &[(0, 16), (2, 16), (4, 16), (6, 16)];
    for &(ct, bd) in combos {
        for variant in 0u8..4 {
            for il in [0u8, 1] {
                for &w in &[1u32, 7, 17] {
                  for &sx in SEEDS {
                    let s = 0xd000 + (ct as u64) * 67 + w as u64 + il as u64 + sx;
                    let png = mk(w, 3, ct, bd, il, s);
                    case(
                        &format!("R13 ct={ct} bd={bd} w={w} il={il} v={variant} s={sx}"),
                        &png,
                        w,
                        3,
                        |c, p, _i| unsafe {
                            match variant {
                                0 => (c.set_strip_16)(p),
                                1 => (c.set_scale_16)(p),
                                2 => {
                                    (c.set_strip_16)(p);
                                    (c.set_scale_16)(p);
                                }
                                _ => {
                                    (c.set_scale_16)(p);
                                    (c.set_strip_16)(p);
                                }
                            }
                        },
                    );
                  }
                }
            }
        }
    }
    // 8-bit inputs: both must be no-ops
    for &(ct, bd) in COMBOS {
        if bd == 16 {
            continue;
        }
        let png = mk(5, 2, ct, bd, 0, 0xdfff + ct as u64);
        case(
            &format!("R13noop ct={ct} bd={bd}"),
            &png,
            5,
            2,
            |c, p, _i| unsafe {
                (c.set_strip_16)(p);
                (c.set_scale_16)(p);
            },
        );
    }
}

// ---------------------------------------------------------------------------
// R14 — png_set_strip_alpha
// ---------------------------------------------------------------------------

#[test]
fn r14_strip_alpha() {
    ensure_libm();
    let combos: &[(u8, u8)] = &[(4, 8), (4, 16), (6, 8), (6, 16)];
    for &(ct, bd) in combos {
        for il in [0u8, 1] {
            for &w in &[1u32, 7, 17] {
                for seed in [0xe000u64, 0xe555] {
                    let s = seed + (ct as u64) * 71 + w as u64;
                    let png = mk(w, 3, ct, bd, il, s);
                    case(
                        &format!("R14 ct={ct} bd={bd} w={w} il={il} seed={s}"),
                        &png,
                        w,
                        3,
                        |c, p, _i| unsafe { (c.set_strip_alpha)(p) },
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R15 — png_set_filler / png_set_add_alpha
// ---------------------------------------------------------------------------

#[test]
fn r15_filler_add_alpha() {
    ensure_libm();
    let fillers: &[u32] = &[0, 0x7f, 0xff, 0xffff, 0x1234];
    let noalpha: &[(u8, u8)] = &[(0, 8), (0, 16), (2, 8), (2, 16)];
    let alpha: &[(u8, u8)] = &[(4, 8), (4, 16), (6, 8), (6, 16)];
    for &(ct, bd) in noalpha {
        for add in [false, true] {
            for &loc in &[PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
                for &f in fillers {
                  for &sx in SEEDS {
                    let s = 0xf000 + (ct as u64) * 73 + bd as u64 + f as u64 + sx;
                    let png = mk(5, 2, ct, bd, 0, s);
                    case(
                        &format!("R15 ct={ct} bd={bd} add={add} loc={loc} f={f:#x} s={sx}"),
                        &png,
                        5,
                        2,
                        |c, p, _i| unsafe {
                            if add {
                                (c.set_add_alpha)(p, f, loc)
                            } else {
                                (c.set_filler)(p, f, loc)
                            }
                        },
                    );
                  }
                }
            }
        }
    }
    // add_alpha on input that already has an alpha channel
    for &(ct, bd) in alpha {
        for &loc in &[PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
            for &f in fillers {
                for &sx in SEEDS {
                    let s = 0xf800 + (ct as u64) * 79 + bd as u64 + f as u64 + sx;
                    let png = mk(5, 2, ct, bd, 0, s);
                    case(
                        &format!("R15a ct={ct} bd={bd} loc={loc} f={f:#x} s={sx}"),
                        &png,
                        5,
                        2,
                        |c, p, _i| unsafe { (c.set_add_alpha)(p, f, loc) },
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R16 — swap / bgr / invert_mono / invert_alpha / swap_alpha
// ---------------------------------------------------------------------------

const T_SWAP: u32 = 1;
const T_BGR: u32 = 2;
const T_IMONO: u32 = 4;
const T_IALPHA: u32 = 8;
const T_SALPHA: u32 = 16;

fn apply_t(c: &Core, p: Png, m: u32) {
    unsafe {
        if m & T_SWAP != 0 {
            (c.set_swap)(p);
        }
        if m & T_BGR != 0 {
            (c.set_bgr)(p);
        }
        if m & T_IMONO != 0 {
            (c.set_invert_mono)(p);
        }
        if m & T_IALPHA != 0 {
            (c.set_invert_alpha)(p);
        }
        if m & T_SALPHA != 0 {
            (c.set_swap_alpha)(p);
        }
    }
}

#[test]
fn r16_byte_order_transforms() {
    ensure_libm();
    let all16: &[(u8, u8)] = &[(0, 16), (2, 16), (4, 16), (6, 16)];
    let colour: &[(u8, u8)] = &[(2, 8), (2, 16), (6, 8), (6, 16)];
    let grayish: &[(u8, u8)] = &[(0, 1), (0, 2), (0, 4), (0, 8), (0, 16), (4, 8), (4, 16)];
    let alpha: &[(u8, u8)] = &[(4, 8), (4, 16), (6, 8), (6, 16)];
    let cases: &[(u32, &[(u8, u8)], &str)] = &[
        // each alone, on every applicable colour type / depth
        (T_SWAP, all16, "swap"),
        (T_SWAP, &[(0, 8), (2, 8), (3, 4), (6, 8)], "swap_noop"),
        (T_BGR, colour, "bgr"),
        (T_BGR, &[(0, 8), (0, 16), (3, 4), (4, 8)], "bgr_noop"),
        (T_IMONO, grayish, "invert_mono"),
        (T_IMONO, &[(2, 8), (3, 2), (6, 16)], "invert_mono_noop"),
        (T_IALPHA, alpha, "invert_alpha"),
        (T_IALPHA, &[(0, 8), (2, 16), (3, 1)], "invert_alpha_noop"),
        (T_SALPHA, alpha, "swap_alpha"),
        (T_SALPHA, &[(0, 8), (2, 16), (3, 1)], "swap_alpha_noop"),
        // pairwise / higher combinations that apply
        (T_SWAP | T_BGR, &[(2, 16), (6, 16)], "swap+bgr"),
        (T_IALPHA | T_SALPHA, alpha, "invert_alpha+swap_alpha"),
        (T_SWAP | T_IALPHA, &[(4, 16), (6, 16)], "swap+invert_alpha"),
        (T_SWAP | T_SALPHA, &[(4, 16), (6, 16)], "swap+swap_alpha"),
        (T_BGR | T_SALPHA, &[(6, 8), (6, 16)], "bgr+swap_alpha"),
        (T_BGR | T_IALPHA, &[(6, 8), (6, 16)], "bgr+invert_alpha"),
        (T_SWAP | T_IMONO, &[(0, 16), (4, 16)], "swap+invert_mono"),
        (T_IMONO | T_IALPHA, &[(4, 8), (4, 16)], "invert_mono+invert_alpha"),
        (
            T_SWAP | T_BGR | T_IALPHA | T_SALPHA,
            &[(6, 16)],
            "swap+bgr+invert_alpha+swap_alpha",
        ),
    ];
    for &(m, list, name) in cases {
        for &(ct, bd) in list {
            for il in [0u8, 1] {
                for &w in &[1u32, 7] {
                  for &sx in SEEDS {
                    let s = 0x1_6000
                        + (ct as u64) * 83
                        + (bd as u64) * 3
                        + m as u64
                        + w as u64
                        + sx;
                    let png = mk(w, 3, ct, bd, il, s);
                    case(
                        &format!("R16 {name} ct={ct} bd={bd} w={w} il={il} s={sx}"),
                        &png,
                        w,
                        3,
                        |c, p, _i| apply_t(c, p, m),
                    );
                  }
                }
            }
        }
    }
}
