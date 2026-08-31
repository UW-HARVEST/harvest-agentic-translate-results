//! Phase B — every ancillary-chunk setter/getter pair, driven differentially
//! through both `.so`s.
//!
//! For each chunk and each of a representative set of base images the test
//!
//!   1. sets the chunk on a *write* struct, runs the full write pipeline
//!      (`png_set_IHDR` -> setters -> `png_write_info` -> `png_write_image` ->
//!      `png_write_end`) and asserts the produced PNG bytes, the captured
//!      `Diag` and the post-write state of *every* `png_get_*` getter are
//!      identical between C and Rust, and
//!   2. reads the produced bytes back (`png_read_info` + `png_read_image` +
//!      `png_read_end`) and asserts every getter's return code and every
//!      out-parameter (including strings and byte arrays), the decoded rows,
//!      the `png_get_valid` masks and the `Diag` are identical.
//!
//! Notes on deliberately untested cases (C undefined behaviour, not error
//! paths, see HARNESS.md):
//!   * `png_get_sCAL`, `png_get_sCAL_fixed`, `png_get_sCAL_s`, `png_get_oFFs`,
//!     `png_get_PLTE`, `png_get_pCAL` and `png_get_eXIf_1` dereference some of
//!     their out-pointers *before* (or without) a NULL check, so they are only
//!     ever called with non-NULL out-parameters here.
//!   * `png_free_data(.., PNG_FREE_TEXT|PNG_FREE_SPLT|PNG_FREE_UNKN, num)`
//!     indexes `info_ptr->text[num]` etc. without a range check, so only
//!     `num == -1` and in-range `num` values are used.
//!   * `png_set_text_2` with `compression > 0` and a NULL `lang`/`lang_key`
//!     reaches `memcpy(dst, NULL, 0)`, so iTXt entries always carry non-NULL
//!     language strings.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ptr::{null, null_mut};

// ---------------------------------------------------------------------------
// formatting helpers — everything is stringified so that a mismatch prints
// the offending value instead of a pointer.
// ---------------------------------------------------------------------------

fn fd(x: c_double) -> String {
    format!("{:?}#{:016x}", x, x.to_bits())
}
fn ff(x: f32) -> String {
    format!("{:?}#{:08x}", x, x.to_bits())
}
fn fdv(xs: &[c_double]) -> String {
    xs.iter().map(|x| fd(*x)).collect::<Vec<_>>().join(",")
}

unsafe fn sbytes(p: *const png_byte, n: usize) -> String {
    if p.is_null() {
        "<null>".to_string()
    } else if n == 0 {
        "<empty>".to_string()
    } else {
        hex(std::slice::from_raw_parts(p, n))
    }
}

unsafe fn sstr(p: *const c_char) -> String {
    match rs_str(p) {
        Some(s) => format!("{:?}", s),
        None => "<null>".to_string(),
    }
}

macro_rules! rec {
    ($o:expr, $tag:expr, $k:expr, $v:expr) => {
        $o.push(format!("{}.{} = {}", $tag, $k, $v))
    };
}

/// Report the first differing entry of two value logs.
fn assert_vals_eq(what: &str, c: &[String], r: &[String]) {
    let n = c.len().min(r.len());
    for i in 0..n {
        if c[i] != r[i] {
            panic!(
                "{}: getter mismatch at record {} (of C {} / RS {})\n  C : {}\n  RS: {}",
                what,
                i,
                c.len(),
                r.len(),
                c[i],
                r[i]
            );
        }
    }
    if c.len() != r.len() {
        panic!(
            "{}: getter log length mismatch (C {} / RS {}); first extra: {}",
            what,
            c.len(),
            r.len(),
            if c.len() > r.len() { &c[n] } else { &r[n] }
        );
    }
}

// ---------------------------------------------------------------------------
// A minimal-but-valid ICC profile (passes png_icc_check_length / _header /
// _tag_table).  `ntags` 12-byte tag-table entries plus `extra` bytes of tag
// data.
// ---------------------------------------------------------------------------

fn put32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_be_bytes());
}
fn put4(b: &mut [u8], at: usize, v: &[u8; 4]) {
    b[at..at + 4].copy_from_slice(v);
}

/// Encoded D50 as an ICC XYZNumber, from png.c.
const D50: [u8; 12] = [
    0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
];

fn icc_profile(color_type: c_int, class: &[u8; 4], pcs: &[u8; 4], intent: u32, extra: usize) -> Vec<u8> {
    let ntags = 1usize;
    let hdr = 132 + 12 * ntags; // 144, already 4-aligned
    let len = hdr + extra;
    let mut p = vec![0u8; len];
    put32(&mut p, 0, len as u32); // profile size
    put4(&mut p, 4, b"none"); // preferred CMM
    p[8] = 2; // version 2.0
    put4(&mut p, 12, class);
    put4(
        &mut p,
        16,
        if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
            b"RGB "
        } else {
            b"GRAY"
        },
    );
    put4(&mut p, 20, pcs);
    put4(&mut p, 36, b"acsp");
    put32(&mut p, 64, intent);
    p[68..80].copy_from_slice(&D50);
    put32(&mut p, 128, ntags as u32);
    put4(&mut p, 132, b"desc");
    put32(&mut p, 136, hdr as u32); // tag start (4-aligned)
    put32(&mut p, 140, extra as u32); // tag length
    // Fill the tag data with an incompressible-looking pattern.  png_handle_iCCP
    // rejects any iCCP chunk whose data is shorter than 81 + LZ77Min bytes with
    // "too short", so a profile of all zeros (which deflates to almost nothing)
    // could never survive a round trip.
    let mut x = 0x2545_f491u32;
    for i in hdr..len {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        p[i] = (x >> 24) as u8;
    }
    p
}

fn good_icc(color_type: c_int) -> Vec<u8> {
    icc_profile(color_type, b"mntr", b"XYZ ", 0, 512)
}

// ---------------------------------------------------------------------------
// Base images
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Base {
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
    pub color_type: c_int,
    pub bit_depth: c_int,
    pub palette: Vec<png_color>,
    pub rows: Vec<Vec<u8>>,
}

impl Base {
    fn num_palette(&self) -> usize {
        self.palette.len()
    }
    fn sample_max(&self) -> u32 {
        if self.bit_depth >= 16 {
            0xffff
        } else {
            (1u32 << self.bit_depth) - 1
        }
    }
}

fn bases() -> Vec<Base> {
    let specs: &[(&'static str, c_int, c_int)] = &[
        ("GRAY@8", PNG_COLOR_TYPE_GRAY, 8),
        ("PALETTE@4", PNG_COLOR_TYPE_PALETTE, 4),
        ("RGB@8", PNG_COLOR_TYPE_RGB, 8),
        ("RGB_ALPHA@16", PNG_COLOR_TYPE_RGB_ALPHA, 16),
        ("GRAY_ALPHA@8", PNG_COLOR_TYPE_GRAY_ALPHA, 8),
    ];
    let mut rng = Rng::new(0x0b1e_5eed_0000_0001);
    let (w, h) = (9u32, 5u32);
    specs
        .iter()
        .map(|&(label, ct, bd)| {
            let pd = channels_of(ct) * bd as u32;
            let rb = rowbytes(pd, w);
            let rows: Vec<Vec<u8>> = (0..h).map(|_| rng.bytes(rb)).collect();
            let palette: Vec<png_color> = if ct == PNG_COLOR_TYPE_PALETTE {
                (0..(1usize << bd))
                    .map(|_| png_color {
                        red: rng.u8(),
                        green: rng.u8(),
                        blue: rng.u8(),
                    })
                    .collect()
            } else {
                Vec::new()
            };
            Base {
                label,
                width: w,
                height: h,
                color_type: ct,
                bit_depth: bd,
                palette,
                rows,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The universal probe: every getter mentioned in the task, applied to one
// (png_struct, png_info) pair.
// ---------------------------------------------------------------------------

const VALID_FLAGS: &[(&str, png_uint_32)] = &[
    ("gAMA", PNG_INFO_gAMA),
    ("sBIT", PNG_INFO_sBIT),
    ("cHRM", PNG_INFO_cHRM),
    ("PLTE", PNG_INFO_PLTE),
    ("tRNS", PNG_INFO_tRNS),
    ("bKGD", PNG_INFO_bKGD),
    ("hIST", PNG_INFO_hIST),
    ("pHYs", PNG_INFO_pHYs),
    ("oFFs", PNG_INFO_oFFs),
    ("tIME", PNG_INFO_tIME),
    ("pCAL", PNG_INFO_pCAL),
    ("sRGB", PNG_INFO_sRGB),
    ("iCCP", PNG_INFO_iCCP),
    ("sPLT", PNG_INFO_sPLT),
    ("sCAL", PNG_INFO_sCAL),
    ("IDAT", PNG_INFO_IDAT),
    ("eXIf", PNG_INFO_eXIf),
    ("cICP", PNG_INFO_cICP),
    ("cLLI", PNG_INFO_cLLI),
    ("mDCV", PNG_INFO_mDCV),
];

/// `icc_trusted` says whether the `proflen` reported by `png_get_iCCP` may be
/// used to bound a read of the stored profile.  That is only true after a
/// *read*: `png_handle_iCCP` has verified that the profile's own length field
/// equals the size of the buffer it allocated.  On a write struct the
/// application supplies both independently, so `png_get_iCCP` can report a
/// length far larger than the buffer (`png_get_iCCP` itself reads the first 4
/// bytes of the profile unconditionally -- C UB for a profile shorter than
/// that, which is why no profile below 4 bytes is ever stored here).
unsafe fn probe_all(
    api: &'static Api,
    png: png_structp,
    info: png_infop,
    tag: &str,
    o: &mut Vec<String>,
    icc_trusted: bool,
) {
    // ---- IHDR and the simple accessors ------------------------------------
    {
        let mut w = 0u32;
        let mut h = 0u32;
        let mut bd = 0i32;
        let mut ct = 0i32;
        let mut il = 0i32;
        let mut cm = 0i32;
        let mut ft = 0i32;
        let ret = guard(|| {
            (api.png_get_IHDR)(
                png, info, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut ft,
            )
        });
        rec!(o, tag, "get_IHDR.ret", format!("{:?}", ret));
        rec!(
            o,
            tag,
            "get_IHDR.out",
            format!("{} {} {} {} {} {} {}", w, h, bd, ct, il, cm, ft)
        );
        // all out-parameters NULL: legal, png_get_IHDR checks each one
        let ret2 = guard(|| {
            (api.png_get_IHDR)(
                png,
                info,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        });
        rec!(o, tag, "get_IHDR.allnull", format!("{:?}", ret2));
    }
    rec!(o, tag, "image_width", (api.png_get_image_width)(png, info));
    rec!(o, tag, "image_height", (api.png_get_image_height)(png, info));
    rec!(o, tag, "bit_depth", (api.png_get_bit_depth)(png, info));
    rec!(o, tag, "color_type", (api.png_get_color_type)(png, info));
    rec!(o, tag, "channels", (api.png_get_channels)(png, info));
    rec!(o, tag, "rowbytes", (api.png_get_rowbytes)(png, info));
    rec!(o, tag, "interlace_type", (api.png_get_interlace_type)(png, info));
    rec!(o, tag, "compression_type", (api.png_get_compression_type)(png, info));
    rec!(o, tag, "filter_type", (api.png_get_filter_type)(png, info));
    rec!(
        o,
        tag,
        "palette_max",
        (api.png_get_palette_max)(png as png_const_structp, info as png_const_infop)
    );
    {
        let sig = (api.png_get_signature)(png, info);
        rec!(o, tag, "signature", sbytes(sig, 8));
    }

    // ---- png_get_valid ----------------------------------------------------
    for (n, f) in VALID_FLAGS {
        rec!(o, tag, &format!("valid.{}", n), (api.png_get_valid)(png, info, *f));
    }
    rec!(o, tag, "valid.zero", (api.png_get_valid)(png, info, 0));
    rec!(o, tag, "valid.all", (api.png_get_valid)(png, info, 0xffff_ffff));

    // ---- PLTE ------------------------------------------------------------
    let mut num_palette = 0i32;
    {
        let mut pal: png_colorp = null_mut();
        let mut np = -1i32;
        let ret = guard(|| (api.png_get_PLTE)(png, info, &mut pal, &mut np));
        rec!(o, tag, "get_PLTE.ret", format!("{:?}", ret));
        rec!(o, tag, "get_PLTE.num", np);
        if ret == Some(PNG_INFO_PLTE) && !pal.is_null() && np > 0 {
            num_palette = np;
            let s: Vec<String> = (0..np as usize)
                .map(|i| {
                    let c = *pal.add(i);
                    format!("{:02x}{:02x}{:02x}", c.red, c.green, c.blue)
                })
                .collect();
            rec!(o, tag, "get_PLTE.entries", s.join(","));
        } else {
            rec!(o, tag, "get_PLTE.entries", "<none>");
        }
    }

    // ---- gAMA ------------------------------------------------------------
    {
        let mut g = -1.0f64;
        let ret = guard(|| (api.png_get_gAMA)(png, info, &mut g));
        rec!(o, tag, "get_gAMA.ret", format!("{:?}", ret));
        rec!(o, tag, "get_gAMA.val", fd(g));
        let mut gf = -1i32;
        let retf = guard(|| (api.png_get_gAMA_fixed)(png, info, &mut gf));
        rec!(o, tag, "get_gAMA_fixed.ret", format!("{:?}", retf));
        rec!(o, tag, "get_gAMA_fixed.val", gf);
        rec!(
            o,
            tag,
            "get_gAMA.null",
            format!("{:?}", guard(|| (api.png_get_gAMA)(png, info, null_mut())))
        );
        rec!(
            o,
            tag,
            "get_gAMA_fixed.null",
            format!("{:?}", guard(|| (api.png_get_gAMA_fixed)(png, info, null_mut())))
        );
    }

    // ---- cHRM ------------------------------------------------------------
    {
        let mut v = [-1.0f64; 8];
        let ret = guard(|| {
            (api.png_get_cHRM)(
                png, info, &mut v[0], &mut v[1], &mut v[2], &mut v[3], &mut v[4], &mut v[5],
                &mut v[6], &mut v[7],
            )
        });
        rec!(o, tag, "get_cHRM.ret", format!("{:?}", ret));
        rec!(o, tag, "get_cHRM.val", fdv(&v));

        let mut f = [-1i32; 8];
        let retf = guard(|| {
            (api.png_get_cHRM_fixed)(
                png, info, &mut f[0], &mut f[1], &mut f[2], &mut f[3], &mut f[4], &mut f[5],
                &mut f[6], &mut f[7],
            )
        });
        rec!(o, tag, "get_cHRM_fixed.ret", format!("{:?}", retf));
        rec!(o, tag, "get_cHRM_fixed.val", format!("{:?}", f));

        let mut x = [-1.0f64; 9];
        let retx = guard(|| {
            (api.png_get_cHRM_XYZ)(
                png, info, &mut x[0], &mut x[1], &mut x[2], &mut x[3], &mut x[4], &mut x[5],
                &mut x[6], &mut x[7], &mut x[8],
            )
        });
        rec!(o, tag, "get_cHRM_XYZ.ret", format!("{:?}", retx));
        rec!(o, tag, "get_cHRM_XYZ.val", fdv(&x));

        let mut xf = [-1i32; 9];
        let retxf = guard(|| {
            (api.png_get_cHRM_XYZ_fixed)(
                png, info, &mut xf[0], &mut xf[1], &mut xf[2], &mut xf[3], &mut xf[4],
                &mut xf[5], &mut xf[6], &mut xf[7], &mut xf[8],
            )
        });
        rec!(o, tag, "get_cHRM_XYZ_fixed.ret", format!("{:?}", retxf));
        rec!(o, tag, "get_cHRM_XYZ_fixed.val", format!("{:?}", xf));
    }

    // ---- sRGB ------------------------------------------------------------
    {
        let mut i = -1i32;
        let ret = guard(|| (api.png_get_sRGB)(png, info, &mut i));
        rec!(o, tag, "get_sRGB.ret", format!("{:?}", ret));
        rec!(o, tag, "get_sRGB.intent", i);
        rec!(
            o,
            tag,
            "get_sRGB.null",
            format!("{:?}", guard(|| (api.png_get_sRGB)(png, info, null_mut())))
        );
    }

    // ---- iCCP ------------------------------------------------------------
    {
        let mut name: png_charp = null_mut();
        let mut comp = -1i32;
        let mut prof: png_bytep = null_mut();
        let mut plen = 0u32;
        let ret = guard(|| {
            (api.png_get_iCCP)(png, info, &mut name, &mut comp, &mut prof, &mut plen)
        });
        rec!(o, tag, "get_iCCP.ret", format!("{:?}", ret));
        rec!(o, tag, "get_iCCP.name", sstr(name));
        rec!(o, tag, "get_iCCP.comp", comp);
        rec!(o, tag, "get_iCCP.proflen", plen);
        if ret == Some(PNG_INFO_iCCP) && !prof.is_null() && icc_trusted {
            let n = (plen as usize).min(65536);
            rec!(o, tag, "get_iCCP.profile", sbytes(prof, n));
        } else if ret == Some(PNG_INFO_iCCP) && !prof.is_null() {
            rec!(o, tag, "get_iCCP.profile", "<untrusted-length>");
        } else {
            rec!(o, tag, "get_iCCP.profile", "<none>");
        }
        // NULL out-parameters: name/profile/proflen are all NULL-checked
        rec!(
            o,
            tag,
            "get_iCCP.null",
            format!(
                "{:?}",
                guard(|| (api.png_get_iCCP)(png, info, null_mut(), null_mut(), null_mut(), null_mut()))
            )
        );
    }

    // ---- sBIT ------------------------------------------------------------
    {
        let mut sb: png_color_8p = null_mut();
        let ret = guard(|| (api.png_get_sBIT)(png, info, &mut sb));
        rec!(o, tag, "get_sBIT.ret", format!("{:?}", ret));
        if ret == Some(PNG_INFO_sBIT) && !sb.is_null() {
            rec!(o, tag, "get_sBIT.val", format!("{:?}", *sb));
        } else {
            rec!(o, tag, "get_sBIT.val", "<none>");
        }
        rec!(
            o,
            tag,
            "get_sBIT.null",
            format!("{:?}", guard(|| (api.png_get_sBIT)(png, info, null_mut())))
        );
    }

    // ---- tRNS ------------------------------------------------------------
    {
        let mut ta: png_bytep = null_mut();
        let mut nt = -1i32;
        let mut tc: png_color_16p = null_mut();
        let ret = guard(|| (api.png_get_tRNS)(png, info, &mut ta, &mut nt, &mut tc));
        rec!(o, tag, "get_tRNS.ret", format!("{:?}", ret));
        rec!(o, tag, "get_tRNS.num", nt);
        if !ta.is_null() && nt > 0 {
            rec!(o, tag, "get_tRNS.alpha", sbytes(ta, (nt as usize).min(256)));
        } else {
            rec!(o, tag, "get_tRNS.alpha", "<none>");
        }
        if !tc.is_null() {
            rec!(o, tag, "get_tRNS.color", format!("{:?}", *tc));
        } else {
            rec!(o, tag, "get_tRNS.color", "<none>");
        }
        // partial out-parameters: each is NULL-checked individually
        let mut nt2 = -1i32;
        rec!(
            o,
            tag,
            "get_tRNS.numonly",
            format!(
                "{:?}/{}",
                guard(|| (api.png_get_tRNS)(png, info, null_mut(), &mut nt2, null_mut())),
                nt2
            )
        );
    }

    // ---- bKGD ------------------------------------------------------------
    {
        let mut b: png_color_16p = null_mut();
        let ret = guard(|| (api.png_get_bKGD)(png, info, &mut b));
        rec!(o, tag, "get_bKGD.ret", format!("{:?}", ret));
        if ret == Some(PNG_INFO_bKGD) && !b.is_null() {
            rec!(o, tag, "get_bKGD.val", format!("{:?}", *b));
        } else {
            rec!(o, tag, "get_bKGD.val", "<none>");
        }
        rec!(
            o,
            tag,
            "get_bKGD.null",
            format!("{:?}", guard(|| (api.png_get_bKGD)(png, info, null_mut())))
        );
    }

    // ---- hIST ------------------------------------------------------------
    {
        let mut hp: png_uint_16p = null_mut();
        let ret = guard(|| (api.png_get_hIST)(png, info, &mut hp));
        rec!(o, tag, "get_hIST.ret", format!("{:?}", ret));
        if ret == Some(PNG_INFO_hIST) && !hp.is_null() && num_palette > 0 {
            let s: Vec<String> = (0..num_palette as usize)
                .map(|i| format!("{}", *hp.add(i)))
                .collect();
            rec!(o, tag, "get_hIST.val", s.join(","));
        } else {
            rec!(o, tag, "get_hIST.val", "<none>");
        }
        rec!(
            o,
            tag,
            "get_hIST.null",
            format!("{:?}", guard(|| (api.png_get_hIST)(png, info, null_mut())))
        );
    }

    // ---- pHYs and everything derived from it ------------------------------
    {
        let mut rx = 0u32;
        let mut ry = 0u32;
        let mut ut = -1i32;
        let ret = guard(|| (api.png_get_pHYs)(png, info, &mut rx, &mut ry, &mut ut));
        rec!(o, tag, "get_pHYs.ret", format!("{:?}", ret));
        rec!(o, tag, "get_pHYs.val", format!("{} {} {}", rx, ry, ut));

        let mut dx = 0u32;
        let mut dy = 0u32;
        let mut du = -1i32;
        let retd = guard(|| (api.png_get_pHYs_dpi)(png, info, &mut dx, &mut dy, &mut du));
        rec!(o, tag, "get_pHYs_dpi.ret", format!("{:?}", retd));
        rec!(o, tag, "get_pHYs_dpi.val", format!("{} {} {}", dx, dy, du));

        // partial out-parameters, each individually NULL-checked
        let mut px = 0u32;
        rec!(
            o,
            tag,
            "get_pHYs.xonly",
            format!(
                "{:?}/{}",
                guard(|| (api.png_get_pHYs)(png, info, &mut px, null_mut(), null_mut())),
                px
            )
        );
        let mut qx = 0u32;
        rec!(
            o,
            tag,
            "get_pHYs_dpi.xonly",
            format!(
                "{:?}/{}",
                guard(|| (api.png_get_pHYs_dpi)(png, info, &mut qx, null_mut(), null_mut())),
                qx
            )
        );

        rec!(o, tag, "x_pixels_per_meter", (api.png_get_x_pixels_per_meter)(png, info));
        rec!(o, tag, "y_pixels_per_meter", (api.png_get_y_pixels_per_meter)(png, info));
        rec!(o, tag, "pixels_per_meter", (api.png_get_pixels_per_meter)(png, info));
        rec!(o, tag, "x_pixels_per_inch", (api.png_get_x_pixels_per_inch)(png, info));
        rec!(o, tag, "y_pixels_per_inch", (api.png_get_y_pixels_per_inch)(png, info));
        rec!(o, tag, "pixels_per_inch", (api.png_get_pixels_per_inch)(png, info));
        rec!(
            o,
            tag,
            "pixel_aspect_ratio",
            match guard(|| (api.png_get_pixel_aspect_ratio)(png, info)) {
                Some(v) => ff(v),
                None => "<err>".to_string(),
            }
        );
        rec!(
            o,
            tag,
            "pixel_aspect_ratio_fixed",
            format!("{:?}", guard(|| (api.png_get_pixel_aspect_ratio_fixed)(png, info)))
        );
    }

    // ---- oFFs and everything derived from it ------------------------------
    {
        let mut ox = 0i32;
        let mut oy = 0i32;
        let mut ut = -1i32;
        let ret = guard(|| (api.png_get_oFFs)(png, info, &mut ox, &mut oy, &mut ut));
        rec!(o, tag, "get_oFFs.ret", format!("{:?}", ret));
        rec!(o, tag, "get_oFFs.val", format!("{} {} {}", ox, oy, ut));
        rec!(o, tag, "x_offset_pixels", (api.png_get_x_offset_pixels)(png, info));
        rec!(o, tag, "y_offset_pixels", (api.png_get_y_offset_pixels)(png, info));
        rec!(o, tag, "x_offset_microns", (api.png_get_x_offset_microns)(png, info));
        rec!(o, tag, "y_offset_microns", (api.png_get_y_offset_microns)(png, info));
        rec!(
            o,
            tag,
            "x_offset_inches",
            match guard(|| (api.png_get_x_offset_inches)(png, info)) {
                Some(v) => ff(v),
                None => "<err>".to_string(),
            }
        );
        rec!(
            o,
            tag,
            "y_offset_inches",
            match guard(|| (api.png_get_y_offset_inches)(png, info)) {
                Some(v) => ff(v),
                None => "<err>".to_string(),
            }
        );
        rec!(
            o,
            tag,
            "x_offset_inches_fixed",
            format!("{:?}", guard(|| (api.png_get_x_offset_inches_fixed)(png, info)))
        );
        rec!(
            o,
            tag,
            "y_offset_inches_fixed",
            format!("{:?}", guard(|| (api.png_get_y_offset_inches_fixed)(png, info)))
        );
    }

    // ---- sCAL ------------------------------------------------------------
    {
        let mut u = -1i32;
        let mut w = -1.0f64;
        let mut h = -1.0f64;
        let ret = guard(|| (api.png_get_sCAL)(png, info, &mut u, &mut w, &mut h));
        rec!(o, tag, "get_sCAL.ret", format!("{:?}", ret));
        rec!(o, tag, "get_sCAL.val", format!("{} {} {}", u, fd(w), fd(h)));

        let mut uf = -1i32;
        let mut wf = -1i32;
        let mut hf = -1i32;
        let retf = guard(|| (api.png_get_sCAL_fixed)(png, info, &mut uf, &mut wf, &mut hf));
        rec!(o, tag, "get_sCAL_fixed.ret", format!("{:?}", retf));
        rec!(o, tag, "get_sCAL_fixed.val", format!("{} {} {}", uf, wf, hf));

        let mut us = -1i32;
        let mut ws: png_charp = null_mut();
        let mut hs: png_charp = null_mut();
        let rets = guard(|| (api.png_get_sCAL_s)(png, info, &mut us, &mut ws, &mut hs));
        rec!(o, tag, "get_sCAL_s.ret", format!("{:?}", rets));
        rec!(
            o,
            tag,
            "get_sCAL_s.val",
            format!("{} {} {}", us, sstr(ws), sstr(hs))
        );
    }

    // ---- pCAL ------------------------------------------------------------
    {
        let mut purpose: png_charp = null_mut();
        let mut x0 = 0i32;
        let mut x1 = 0i32;
        let mut ty = -1i32;
        let mut np = -1i32;
        let mut units: png_charp = null_mut();
        let mut params: png_charpp = null_mut();
        let ret = guard(|| {
            (api.png_get_pCAL)(
                png, info, &mut purpose, &mut x0, &mut x1, &mut ty, &mut np, &mut units,
                &mut params,
            )
        });
        rec!(o, tag, "get_pCAL.ret", format!("{:?}", ret));
        rec!(
            o,
            tag,
            "get_pCAL.val",
            format!(
                "{} {} {} {} {} {}",
                sstr(purpose),
                x0,
                x1,
                ty,
                np,
                sstr(units)
            )
        );
        if ret == Some(PNG_INFO_pCAL) && !params.is_null() && np > 0 {
            let s: Vec<String> = (0..np as usize).map(|i| sstr(*params.add(i))).collect();
            rec!(o, tag, "get_pCAL.params", s.join(","));
        } else {
            rec!(o, tag, "get_pCAL.params", "<none>");
        }
    }

    // ---- tIME ------------------------------------------------------------
    {
        let mut t: png_timep = null_mut();
        let ret = guard(|| (api.png_get_tIME)(png, info, &mut t));
        rec!(o, tag, "get_tIME.ret", format!("{:?}", ret));
        if ret == Some(PNG_INFO_tIME) && !t.is_null() {
            rec!(o, tag, "get_tIME.val", format!("{:?}", *t));
        } else {
            rec!(o, tag, "get_tIME.val", "<none>");
        }
        rec!(
            o,
            tag,
            "get_tIME.null",
            format!("{:?}", guard(|| (api.png_get_tIME)(png, info, null_mut())))
        );
    }

    // ---- tEXt / zTXt / iTXt ----------------------------------------------
    {
        let mut tp: png_textp = null_mut();
        let mut n = -1i32;
        let ret = guard(|| (api.png_get_text)(png, info, &mut tp, &mut n));
        rec!(o, tag, "get_text.ret", format!("{:?}", ret));
        rec!(o, tag, "get_text.num", n);
        if !tp.is_null() && n > 0 {
            for i in 0..n as usize {
                let t = *tp.add(i);
                rec!(
                    o,
                    tag,
                    &format!("get_text[{}]", i),
                    format!(
                        "comp={} key={} text={} tlen={} ilen={} lang={} langkey={}",
                        t.compression,
                        sstr(t.key),
                        sstr(t.text),
                        t.text_length,
                        t.itxt_length,
                        sstr(t.lang),
                        sstr(t.lang_key)
                    )
                );
                let blen = t.text_length.max(t.itxt_length);
                rec!(
                    o,
                    tag,
                    &format!("get_text[{}].bytes", i),
                    sbytes(t.text as *const png_byte, blen.min(4096))
                );
            }
        }
        // NULL out-parameters are both individually checked
        rec!(
            o,
            tag,
            "get_text.null",
            format!("{:?}", guard(|| (api.png_get_text)(png, info, null_mut(), null_mut())))
        );
    }

    // ---- sPLT ------------------------------------------------------------
    {
        let mut sp: png_sPLT_tp = null_mut();
        let ret = guard(|| (api.png_get_sPLT)(png, info, &mut sp));
        rec!(o, tag, "get_sPLT.ret", format!("{:?}", ret));
        if let Some(cnt) = ret {
            if cnt > 0 && !sp.is_null() {
                for i in 0..cnt as usize {
                    let s = *sp.add(i);
                    rec!(
                        o,
                        tag,
                        &format!("get_sPLT[{}]", i),
                        format!(
                            "name={} depth={} nentries={}",
                            sstr(s.name),
                            s.depth,
                            s.nentries
                        )
                    );
                    if !s.entries.is_null() && s.nentries > 0 {
                        let e: Vec<String> = (0..s.nentries as usize)
                            .map(|j| {
                                let x = *s.entries.add(j);
                                format!(
                                    "{}/{}/{}/{}/{}",
                                    x.red, x.green, x.blue, x.alpha, x.frequency
                                )
                            })
                            .collect();
                        rec!(o, tag, &format!("get_sPLT[{}].e", i), e.join(","));
                    }
                }
            }
        }
        rec!(
            o,
            tag,
            "get_sPLT.null",
            format!("{:?}", guard(|| (api.png_get_sPLT)(png, info, null_mut())))
        );
    }

    // ---- eXIf ------------------------------------------------------------
    {
        let mut n = 0u32;
        let mut e: png_bytep = null_mut();
        let ret = guard(|| (api.png_get_eXIf_1)(png, info, &mut n, &mut e));
        rec!(o, tag, "get_eXIf_1.ret", format!("{:?}", ret));
        rec!(o, tag, "get_eXIf_1.num", n);
        if ret == Some(PNG_INFO_eXIf) && !e.is_null() {
            rec!(o, tag, "get_eXIf_1.data", sbytes(e, (n as usize).min(4096)));
        } else {
            rec!(o, tag, "get_eXIf_1.data", "<none>");
        }
        // The deprecated API: warns and returns 0.
        let mut e2: png_bytep = null_mut();
        rec!(
            o,
            tag,
            "get_eXIf.ret",
            format!("{:?}", guard(|| (api.png_get_eXIf)(png, info, &mut e2)))
        );
    }

    // ---- cICP ------------------------------------------------------------
    {
        let mut a = 0xffu8;
        let mut b = 0xffu8;
        let mut c = 0xffu8;
        let mut d = 0xffu8;
        let ret = guard(|| (api.png_get_cICP)(png, info, &mut a, &mut b, &mut c, &mut d));
        rec!(o, tag, "get_cICP.ret", format!("{:?}", ret));
        rec!(o, tag, "get_cICP.val", format!("{} {} {} {}", a, b, c, d));
        rec!(
            o,
            tag,
            "get_cICP.null",
            format!(
                "{:?}",
                guard(|| (api.png_get_cICP)(png, info, null_mut(), null_mut(), null_mut(), null_mut()))
            )
        );
    }

    // ---- cLLI ------------------------------------------------------------
    {
        let mut a = -1.0f64;
        let mut b = -1.0f64;
        let ret = guard(|| (api.png_get_cLLI)(png, info, &mut a, &mut b));
        rec!(o, tag, "get_cLLI.ret", format!("{:?}", ret));
        rec!(o, tag, "get_cLLI.val", format!("{} {}", fd(a), fd(b)));
        let mut af = 0xffff_ffffu32;
        let mut bf = 0xffff_ffffu32;
        let retf = guard(|| (api.png_get_cLLI_fixed)(png, info, &mut af, &mut bf));
        rec!(o, tag, "get_cLLI_fixed.ret", format!("{:?}", retf));
        rec!(o, tag, "get_cLLI_fixed.val", format!("{} {}", af, bf));
        rec!(
            o,
            tag,
            "get_cLLI.null",
            format!("{:?}", guard(|| (api.png_get_cLLI)(png, info, null_mut(), null_mut())))
        );
        rec!(
            o,
            tag,
            "get_cLLI_fixed.null",
            format!(
                "{:?}",
                guard(|| (api.png_get_cLLI_fixed)(png, info, null_mut(), null_mut()))
            )
        );
    }

    // ---- mDCV ------------------------------------------------------------
    {
        let mut v = [-1.0f64; 8];
        let mut mx = -1.0f64;
        let mut mn = -1.0f64;
        let ret = guard(|| {
            (api.png_get_mDCV)(
                png, info, &mut v[0], &mut v[1], &mut v[2], &mut v[3], &mut v[4], &mut v[5],
                &mut v[6], &mut v[7], &mut mx, &mut mn,
            )
        });
        rec!(o, tag, "get_mDCV.ret", format!("{:?}", ret));
        rec!(
            o,
            tag,
            "get_mDCV.val",
            format!("{} | {} {}", fdv(&v), fd(mx), fd(mn))
        );
        let mut f = [-1i32; 8];
        let mut fx = 0xffff_ffffu32;
        let mut fn_ = 0xffff_ffffu32;
        let retf = guard(|| {
            (api.png_get_mDCV_fixed)(
                png, info, &mut f[0], &mut f[1], &mut f[2], &mut f[3], &mut f[4], &mut f[5],
                &mut f[6], &mut f[7], &mut fx, &mut fn_,
            )
        });
        rec!(o, tag, "get_mDCV_fixed.ret", format!("{:?}", retf));
        rec!(
            o,
            tag,
            "get_mDCV_fixed.val",
            format!("{:?} | {} {}", f, fx, fn_)
        );
    }

    // ---- unknown chunks ---------------------------------------------------
    {
        let mut up: png_unknown_chunkp = null_mut();
        let ret = guard(|| (api.png_get_unknown_chunks)(png, info, &mut up));
        rec!(o, tag, "get_unknown_chunks.ret", format!("{:?}", ret));
        if let Some(cnt) = ret {
            if cnt > 0 && !up.is_null() {
                for i in 0..cnt as usize {
                    let u = *up.add(i);
                    rec!(
                        o,
                        tag,
                        &format!("unknown[{}]", i),
                        format!(
                            "name={:?} size={} loc={} data={}",
                            u.name,
                            u.size,
                            u.location,
                            sbytes(u.data, u.size.min(4096))
                        )
                    );
                }
            }
        }
        rec!(
            o,
            tag,
            "get_unknown_chunks.null",
            format!("{:?}", guard(|| (api.png_get_unknown_chunks)(png, info, null_mut())))
        );
    }

    // ---- png_handle_as_unknown for a fixed probe set ----------------------
    for name in [
        &b"bKGD\0"[..],
        &b"gAMA\0"[..],
        &b"vpAg\0"[..],
        &b"ABCD\0"[..],
        &b"IDAT\0"[..],
    ] {
        rec!(
            o,
            tag,
            &format!("handle_as_unknown.{}", String::from_utf8_lossy(&name[..4])),
            format!("{:?}", guard(|| (api.png_handle_as_unknown)(png, name.as_ptr())))
        );
    }
}

// ---------------------------------------------------------------------------
// Drivers
// ---------------------------------------------------------------------------

type Hook = dyn Fn(&'static Api, png_structp, png_infop);

fn nop(_: &'static Api, _: png_structp, _: png_infop) {}

pub struct WOut {
    pub bytes: Vec<u8>,
    pub diag: Diag,
    pub ok: bool,
    pub post: Vec<String>,
}

unsafe fn write_run(api: &'static Api, b: &Base, setup: &Hook, endup: &Hook) -> WOut {
    set_current_api(api);
    diag_reset();
    let mut sess = WriteSess::new(api);
    let png = sess.png;
    let info = sess.info;
    let ok = guard(|| {
        (api.png_set_IHDR)(
            png,
            info,
            b.width,
            b.height,
            b.bit_depth,
            b.color_type,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        if !b.palette.is_empty() {
            (api.png_set_PLTE)(png, info, b.palette.as_ptr(), b.palette.len() as c_int);
        }
        setup(api, png, info);
        (api.png_write_info)(png, info);
        let mut rp: Vec<png_bytep> = b.rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
        (api.png_write_image)(png, rp.as_mut_ptr());
        endup(api, png, info);
        (api.png_write_end)(png, info);
    })
    .is_some();
    let mut post = Vec::new();
    guard(|| probe_all(api, png, info, "w", &mut post, false));
    let diag = diag_take();
    let bytes = std::mem::take(&mut sess.sink.buf);
    WOut {
        bytes,
        diag,
        ok,
        post,
    }
}

pub struct ROut {
    pub diag: Diag,
    pub ok: bool,
    pub vals: Vec<String>,
}

unsafe fn read_run(api: &'static Api, data: &[u8], pre: &Hook) -> ROut {
    set_current_api(api);
    diag_reset();
    let sess = ReadSess::new(api, data);
    let png = sess.png;
    let info = sess.info;
    let end = sess.end;
    let mut vals: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<u8>> = Vec::new();
    let ok = guard(|| {
        pre(api, png, info);
        (api.png_read_info)(png, info);
        probe_all(api, png, info, "ri", &mut vals, true);
        let rb = (api.png_get_rowbytes)(png, info);
        let h = (api.png_get_image_height)(png, info) as usize;
        rows = (0..h).map(|_| vec![0u8; rb + 16]).collect();
        let mut rp: Vec<png_bytep> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
        (api.png_read_image)(png, rp.as_mut_ptr());
        (api.png_read_end)(png, end);
        probe_all(api, png, info, "info", &mut vals, true);
        probe_all(api, png, end, "end", &mut vals, true);
    })
    .is_some();
    for (i, r) in rows.iter().enumerate() {
        vals.push(format!("row[{}] = {}", i, hex(r)));
    }
    let diag = diag_take();
    ROut { diag, ok, vals }
}

/// Write with `setup`/`endup`, compare bytes+diag+post-state, then read back
/// with `rpre` and compare every getter.
fn roundtrip(label: &str, b: &Base, setup: &Hook, endup: &Hook, rpre: &Hook) {
    unsafe {
        let cw = write_run(c_api(), b, setup, endup);
        let rw = write_run(rs_api(), b, setup, endup);
        assert_eq!(
            cw.ok, rw.ok,
            "{}: write error parity (C ok={} RS ok={})\n C diag {:?}\n RS diag {:?}",
            label, cw.ok, rw.ok, cw.diag, rw.diag
        );
        assert_eq!(cw.diag, rw.diag, "{}: write diagnostics", label);
        assert_bytes_eq(&format!("{} [write]", label), &cw.bytes, &rw.bytes);
        assert_vals_eq(&format!("{} [write-state]", label), &cw.post, &rw.post);

        if cw.bytes.len() < 8 {
            return;
        }
        let cr = read_run(c_api(), &cw.bytes, rpre);
        let rr = read_run(rs_api(), &rw.bytes, rpre);
        assert_eq!(
            cr.ok, rr.ok,
            "{}: read error parity (C ok={} RS ok={})\n C diag {:?}\n RS diag {:?}",
            label, cr.ok, rr.ok, cr.diag, rr.diag
        );
        assert_eq!(cr.diag, rr.diag, "{}: read diagnostics", label);
        assert_vals_eq(&format!("{} [read]", label), &cr.vals, &rr.vals);
    }
}

/// The common case: chunk set before `png_write_info`, plain read.
fn rt(label: &str, b: &Base, setup: &Hook) {
    roundtrip(label, b, setup, &nop, &nop);
}

/// Read `bytes` through both libraries and compare everything.
fn diff_read(label: &str, bytes: &[u8], rpre: &Hook) {
    unsafe {
        let cr = read_run(c_api(), bytes, rpre);
        let rr = read_run(rs_api(), bytes, rpre);
        assert_eq!(
            cr.ok, rr.ok,
            "{}: read error parity (C ok={} RS ok={})\n C diag {:?}\n RS diag {:?}",
            label, cr.ok, rr.ok, cr.diag, rr.diag
        );
        assert_eq!(cr.diag, rr.diag, "{}: read diagnostics", label);
        assert_vals_eq(label, &cr.vals, &rr.vals);
    }
}

// ---------------------------------------------------------------------------
// Byte-stream surgery: reorder chunks so that read paths which png_write_info's
// fixed chunk order can never produce are still reachable.  Chunk payloads (and
// therefore their CRCs) are never touched.
// ---------------------------------------------------------------------------

fn split_chunks(d: &[u8]) -> (Vec<u8>, Vec<([u8; 4], Vec<u8>)>) {
    let mut out: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    let mut i = 8usize;
    while i + 8 <= d.len() {
        let len = u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]) as usize;
        let total = 12 + len;
        if i + total > d.len() {
            break;
        }
        let name = [d[i + 4], d[i + 5], d[i + 6], d[i + 7]];
        out.push((name, d[i..i + total].to_vec()));
        i += total;
    }
    (d[..8.min(d.len())].to_vec(), out)
}

/// Move the first `what` chunk so that it immediately precedes the first
/// `before` chunk.  Returns `None` if either chunk is absent.
fn move_before(d: &[u8], what: &[u8; 4], before: &[u8; 4]) -> Option<Vec<u8>> {
    let (sig, mut chunks) = split_chunks(d);
    let wi = chunks.iter().position(|(n, _)| n == what)?;
    let item = chunks.remove(wi);
    let bi = chunks.iter().position(|(n, _)| n == before)?;
    chunks.insert(bi, item);
    let mut out = sig;
    for (_, c) in chunks {
        out.extend_from_slice(&c);
    }
    Some(out)
}

/// Read hook that makes libpng store every unknown chunk.
fn keep_all(api: &'static Api, png: png_structp, _info: png_infop) {
    unsafe {
        (api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, null(), 0);
    }
}

// ===========================================================================
// Tests
// ===========================================================================

/// Self-check: the differential comparison above is only meaningful if the
/// chunks really do reach the byte stream and come back out of it.  This asserts
/// that for a representative selection, on *both* libraries.
#[test]
fn sanity_chunks_really_round_trip() {
    let all = bases();
    let b = &all[1]; // PALETTE@4: PLTE/hIST/tRNS all natural for this type
    let bb = b.clone();
    let setup = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
        fill_everything(api, png, info, &bb);
    };
    for api in both() {
        unsafe {
            let w = write_run(api, b, &setup, &nop);
            assert!(w.ok, "{}: sanity write failed: {:?}", api.name, w.diag);
            assert!(
                w.bytes.len() > 400,
                "{}: sanity stream too small ({} bytes)",
                api.name,
                w.bytes.len()
            );
            for tag in [
                &b"IHDR"[..],
                b"PLTE",
                b"gAMA",
                b"cHRM",
                b"sRGB",
                b"iCCP",
                b"sBIT",
                b"tRNS",
                b"bKGD",
                // hIST *is* written, but this tree's read-side chunk table
                // gives it pos_before = PNG_HAVE_PLTE while png_write_info
                // emits it after PLTE, so it can never be read back ("hIST:
                // out of place").  Both libraries agree, so this is not a
                // divergence -- see chunk_hIST_reordered below.
                b"hIST",
                b"pHYs",
                b"oFFs",
                b"sCAL",
                b"pCAL",
                b"tIME",
                b"tEXt",
                b"sPLT",
                b"eXIf",
                b"cICP",
                b"cLLI",
                b"mDCV",
                b"vpAg",
                b"IDAT",
                b"IEND",
            ] {
                assert!(
                    w.bytes.windows(4).any(|x| x == tag),
                    "{}: {} missing from the produced stream",
                    api.name,
                    String::from_utf8_lossy(tag)
                );
            }
            let r = read_run(api, &w.bytes, &keep_all);
            assert!(r.ok, "{}: sanity read failed: {:?}", api.name, r.diag);
            // every getter that must report data after the read
            for want in [
                "info.get_gAMA.ret = Some(1)",
                "info.get_cHRM.ret = Some(4)",
                "info.get_sRGB.ret = Some(2048)",
                "info.get_iCCP.ret = Some(4096)",
                "info.get_sBIT.ret = Some(2)",
                "info.get_PLTE.ret = Some(8)",
                "info.get_tRNS.ret = Some(16)",
                "info.get_bKGD.ret = Some(32)",
                "info.get_pHYs.ret = Some(128)",
                "info.get_oFFs.ret = Some(256)",
                "info.get_sCAL.ret = Some(16384)",
                "info.get_pCAL.ret = Some(1024)",
                "info.get_tIME.ret = Some(512)",
                "info.get_eXIf_1.ret = Some(65536)",
                "info.get_cICP.ret = Some(131072)",
                "info.get_cLLI.ret = Some(262144)",
                "info.get_mDCV.ret = Some(524288)",
                "info.get_text.num = 1",
                "info.get_sPLT.ret = Some(1)",
                "info.get_unknown_chunks.ret = Some(1)",
            ] {
                assert!(
                    r.vals.iter().any(|v| v == want),
                    "{}: expected read record {:?} not found",
                    api.name,
                    want
                );
            }
        }
    }
}

/// `png_get_palette_max` only ever reports something other than 0 / -1 when
/// `png_do_check_palette_indexes` actually runs, which needs
/// `num_palette < (1 << bit_depth)`.  Build exactly that case (a 4-bit paletted
/// image with a short PLTE and row indices that overflow it) and compare the
/// tracked maximum, plus every value of `png_set_check_for_invalid_index`.
#[test]
fn palette_max_and_invalid_index() {
    let mut b = bases()[1].clone(); // PALETTE@4
    let mut saw_nonzero = false;
    for np in [1usize, 2, 5, 15, 16] {
        b.palette.truncate(np);
        for allowed in [-1i32, 0, 1, 2] {
            let setup = move |api: &'static Api, png: png_structp, _i: png_infop| unsafe {
                (api.png_set_check_for_invalid_index)(png, allowed);
            };
            let rpre = move |api: &'static Api, png: png_structp, _i: png_infop| unsafe {
                (api.png_set_check_for_invalid_index)(png, allowed);
            };
            roundtrip(
                &format!("palette_max np={} allowed={}", np, allowed),
                &b,
                &setup,
                &nop,
                &rpre,
            );
            unsafe {
                let w = write_run(c_api(), &b, &setup, &nop);
                if w.post.iter().any(|v| {
                    v.starts_with("w.palette_max = ")
                        && v != "w.palette_max = 0"
                        && v != "w.palette_max = -1"
                }) {
                    saw_nonzero = true;
                }
            }
        }
    }
    assert!(
        saw_nonzero,
        "png_get_palette_max never reported a tracked maximum -- the test is not \
         exercising png_do_check_palette_indexes"
    );
}

#[test]
fn ihdr_and_basic_accessors() {
    for b in bases() {
        rt(&format!("bare {}", b.label), &b, &nop);
    }
}

// ---------------------------------------------------------------------------
// gAMA
// ---------------------------------------------------------------------------

#[test]
fn chunk_gAMA() {
    let mut rng = Rng::new(0x9a11_0001);
    let mut fixed: Vec<i32> = vec![
        0,
        1,
        PNG_FP_1,
        PNG_FP_HALF,
        45455,
        100_000,
        500_000,
        PNG_FP_MAX,
        -1,
        -100_000,
        PNG_FP_MIN,
        i32::MIN,
    ];
    for _ in 0..12 {
        fixed.push(rng.range(-2_000_000, 2_000_000) as i32);
    }
    let dbl: Vec<f64> = vec![
        0.0, 1.0, 0.45455, 2.2, 1.0 / 2.2, 0.00001, 21474.0, -1.0, 1e-7, 99999.9,
    ];
    for b in bases() {
        for &g in &fixed {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_gAMA_fixed)(png, info, g);
            };
            rt(&format!("gAMA_fixed {} {}", b.label, g), &b, &f);
        }
        for &g in &dbl {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_gAMA)(png, info, g);
            };
            rt(&format!("gAMA {} {}", b.label, g), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// cHRM
// ---------------------------------------------------------------------------

#[test]
fn chunk_cHRM() {
    let mut rng = Rng::new(0xc4_0002);
    // sRGB primaries, all-zero, all-max, negative, random
    let mut sets: Vec<[i32; 8]> = vec![
        [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000],
        [0; 8],
        [1; 8],
        [PNG_FP_MAX; 8],
        [-1; 8],
        [PNG_FP_MIN; 8],
        [100_000, 100_000, 100_000, 0, 0, 100_000, 0, 0],
    ];
    for _ in 0..14 {
        let mut a = [0i32; 8];
        for x in a.iter_mut() {
            *x = rng.range(-200_000, 200_000) as i32;
        }
        sets.push(a);
    }
    let mut xyz: Vec<[i32; 9]> = vec![
        // The sRGB XYZ endpoints (from png_XYZ_from_xy of the sRGB xy above)
        [41239, 21264, 1933, 35758, 71517, 11919, 18048, 7219, 95053],
        [0; 9],
        [1; 9],
        [100_000; 9],
        [-1; 9],
    ];
    for _ in 0..14 {
        let mut a = [0i32; 9];
        for x in a.iter_mut() {
            *x = rng.range(-150_000, 150_000) as i32;
        }
        xyz.push(a);
    }

    for b in bases() {
        for s in &sets {
            let v = *s;
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_cHRM_fixed)(
                    png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],
                );
            };
            rt(&format!("cHRM_fixed {} {:?}", b.label, v), &b, &f);

            let d: [f64; 8] = std::array::from_fn(|i| v[i] as f64 / 100_000.0);
            let g = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_cHRM)(
                    png, info, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7],
                );
            };
            rt(&format!("cHRM {} {:?}", b.label, d), &b, &g);
        }
        for s in &xyz {
            let v = *s;
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_cHRM_XYZ_fixed)(
                    png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
                );
            };
            rt(&format!("cHRM_XYZ_fixed {} {:?}", b.label, v), &b, &f);

            let d: [f64; 9] = std::array::from_fn(|i| v[i] as f64 / 100_000.0);
            let g = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_cHRM_XYZ)(
                    png, info, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7], d[8],
                );
            };
            rt(&format!("cHRM_XYZ {} {:?}", b.label, d), &b, &g);
        }
    }
}

// ---------------------------------------------------------------------------
// sRGB
// ---------------------------------------------------------------------------

#[test]
fn chunk_sRGB() {
    for b in bases() {
        for intent in [-1i32, 0, 1, 2, 3, 4, 5, 127, 255, 256, i32::MAX] {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_sRGB)(png, info, intent);
            };
            rt(&format!("sRGB {} {}", b.label, intent), &b, &f);

            let g = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_sRGB_gAMA_and_cHRM)(png, info, intent);
            };
            rt(&format!("sRGB_gAMA_and_cHRM {} {}", b.label, intent), &b, &g);
        }
    }
}

// ---------------------------------------------------------------------------
// iCCP
// ---------------------------------------------------------------------------

#[test]
fn chunk_iCCP() {
    let mut rng = Rng::new(0x1cc9_0003);
    for b in bases() {
        // (a) a real, minimally valid profile with several tag-data sizes
        for extra in [0usize, 4, 64, 260, 512, 1024] {
            for intent in [0u32, 1, 2, 3] {
                let prof = icc_profile(b.color_type, b"mntr", b"XYZ ", intent, extra);
                let name = format!("icc{}", extra);
                let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                    let n = cs(&name);
                    (api.png_set_iCCP)(
                        png,
                        info,
                        n.as_ptr(),
                        PNG_COMPRESSION_TYPE_BASE,
                        prof.as_ptr(),
                        prof.len() as png_uint_32,
                    );
                };
                rt(
                    &format!("iCCP good {} extra={} intent={}", b.label, extra, intent),
                    &b,
                    &f,
                );
            }
        }
        // (b) profile classes and PCS encodings that trigger the various
        //     png_icc_check_header diagnostics on read
        for (class, pcs) in [
            (b"scnr", b"XYZ "),
            (b"prtr", b"Lab "),
            (b"spac", b"XYZ "),
            (b"abst", b"XYZ "),
            (b"link", b"XYZ "),
            (b"nmcl", b"XYZ "),
            (b"zzzz", b"XYZ "),
            (b"mntr", b"CMYK"),
        ] {
            let prof = icc_profile(b.color_type, class, pcs, 0, 64);
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let n = cs("cls");
                (api.png_set_iCCP)(
                    png,
                    info,
                    n.as_ptr(),
                    PNG_COMPRESSION_TYPE_BASE,
                    prof.as_ptr(),
                    prof.len() as png_uint_32,
                );
            };
            rt(
                &format!(
                    "iCCP class {} {}/{}",
                    b.label,
                    String::from_utf8_lossy(class),
                    String::from_utf8_lossy(pcs)
                ),
                &b,
                &f,
            );
        }
        // (c) a profile with a deliberately wrong colour space for this image
        {
            let other = b.color_type ^ PNG_COLOR_MASK_COLOR;
            let prof = icc_profile(other, b"mntr", b"XYZ ", 0, 64);
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let n = cs("wrongspace");
                (api.png_set_iCCP)(
                    png,
                    info,
                    n.as_ptr(),
                    PNG_COMPRESSION_TYPE_BASE,
                    prof.as_ptr(),
                    prof.len() as png_uint_32,
                );
            };
            rt(&format!("iCCP wrong-space {}", b.label), &b, &f);
        }
        // (d) an out-of-range rendering intent (>= 0xffff is a hard error)
        for intent in [4u32, 0xfffe, 0xffff, 0x1_0000] {
            let prof = icc_profile(b.color_type, b"mntr", b"XYZ ", intent, 64);
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let n = cs("intent");
                (api.png_set_iCCP)(
                    png,
                    info,
                    n.as_ptr(),
                    PNG_COMPRESSION_TYPE_BASE,
                    prof.as_ptr(),
                    prof.len() as png_uint_32,
                );
            };
            rt(&format!("iCCP intent {} {}", b.label, intent), &b, &f);
        }
        // (e) short and garbage profiles (rejected by png_write_iCCP).
        //     Nothing shorter than 4 bytes: png_get_iCCP unconditionally reads
        //     the profile's first 4 bytes, so a shorter buffer would be a heap
        //     over-read (C UB, not an error path -- see HARNESS.md).
        let mut bad: Vec<Vec<u8>> = vec![
            vec![0u8; 4],
            vec![0xff; 5],
            vec![0u8; 131],
            vec![0u8; 132],
            vec![0xff; 132],
            {
                // right length field, garbage everywhere else
                let mut v = vec![0xaa; 132];
                put32(&mut v, 0, 132);
                v
            },
            {
                // length field disagrees with the buffer length
                let mut v = vec![0u8; 200];
                put32(&mut v, 0, 132);
                v
            },
            {
                // version > 3 with a length that is not a multiple of 4
                let mut v = vec![0u8; 133];
                put32(&mut v, 0, 133);
                v[8] = 4;
                v
            },
            {
                // absurd tag count
                let mut v = icc_profile(b.color_type, b"mntr", b"XYZ ", 0, 64);
                put32(&mut v, 128, 0xffff_ffff);
                v
            },
            {
                // tag pointing outside the profile
                let mut v = icc_profile(b.color_type, b"mntr", b"XYZ ", 0, 64);
                put32(&mut v, 136, 0xffff_0000);
                v
            },
            {
                // misaligned tag start (warning only)
                let mut v = icc_profile(b.color_type, b"mntr", b"XYZ ", 0, 64);
                put32(&mut v, 136, 145);
                put32(&mut v, 140, 4);
                v
            },
            {
                // not D50 (warning only)
                let mut v = icc_profile(b.color_type, b"mntr", b"XYZ ", 0, 64);
                v[68] = 1;
                v
            },
        ];
        for _ in 0..4 {
            bad.push(rng.bytes(140));
        }
        for (i, prof) in bad.iter().enumerate() {
            let p = prof.clone();
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let n = cs("bad");
                (api.png_set_iCCP)(
                    png,
                    info,
                    n.as_ptr(),
                    PNG_COMPRESSION_TYPE_BASE,
                    p.as_ptr(),
                    p.len() as png_uint_32,
                );
            };
            rt(&format!("iCCP bad#{} {}", i, b.label), &b, &f);
        }
        // (f) invalid compression method, and invalid keywords
        for (cm, key) in [
            (1i32, "k"),
            (-1, "k"),
            (255, "k"),
            (0, ""),
            (0, " leading"),
            (0, "trailing "),
            (0, "a very long keyword that goes on and on and on and on past the seventy nine character limit"),
        ] {
            let prof = good_icc(b.color_type);
            let key = key.to_string();
            let key2 = key.clone();
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let n = cs(&key2);
                (api.png_set_iCCP)(
                    png,
                    info,
                    n.as_ptr(),
                    cm,
                    prof.as_ptr(),
                    prof.len() as png_uint_32,
                );
            };
            rt(&format!("iCCP cm={} key={:?} {}", cm, key, b.label), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// sBIT
// ---------------------------------------------------------------------------

#[test]
fn chunk_sBIT() {
    let mut rng = Rng::new(0x5b17_0004);
    for b in bases() {
        let bd = b.bit_depth as u8;
        let mut cases: Vec<png_color_8> = vec![
            png_color_8 { red: 0, green: 0, blue: 0, gray: 0, alpha: 0 },
            png_color_8 { red: 1, green: 1, blue: 1, gray: 1, alpha: 1 },
            png_color_8 { red: bd, green: bd, blue: bd, gray: bd, alpha: bd },
            png_color_8 {
                red: bd.wrapping_add(1),
                green: bd,
                blue: bd,
                gray: bd.wrapping_add(1),
                alpha: bd,
            },
            png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 },
            png_color_8 { red: 255, green: 255, blue: 255, gray: 255, alpha: 255 },
        ];
        for _ in 0..10 {
            cases.push(png_color_8 {
                red: rng.u8() % 20,
                green: rng.u8() % 20,
                blue: rng.u8() % 20,
                gray: rng.u8() % 20,
                alpha: rng.u8() % 20,
            });
        }
        for s in cases {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_sBIT)(png, info, &s);
            };
            rt(&format!("sBIT {} {:?}", b.label, s), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// PLTE
// ---------------------------------------------------------------------------

#[test]
fn chunk_PLTE() {
    let mut rng = Rng::new(0x9173_0005);
    for b in bases() {
        let max = if b.color_type == PNG_COLOR_TYPE_PALETTE {
            1usize << b.bit_depth
        } else {
            256
        };
        let mut ns: Vec<i32> = vec![0, 1, 2, max as i32, max as i32 + 1, 256, 257, -1];
        for _ in 0..6 {
            ns.push(rng.below(max as u32 + 2) as i32);
        }
        for n in ns {
            let pal: Vec<png_color> = (0..n.max(0) as usize)
                .map(|_| png_color {
                    red: rng.u8(),
                    green: rng.u8(),
                    blue: rng.u8(),
                })
                .collect();
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_PLTE)(
                    png,
                    info,
                    if pal.is_empty() {
                        null()
                    } else {
                        pal.as_ptr()
                    },
                    n,
                );
            };
            rt(&format!("PLTE {} n={}", b.label, n), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// tRNS -- all three colour-type forms
// ---------------------------------------------------------------------------

#[test]
fn chunk_tRNS() {
    let mut rng = Rng::new(0x7205_0006);
    for b in bases() {
        let np = b.num_palette();
        let smax = b.sample_max();
        // palette form: an alpha array
        let mut alphas: Vec<Vec<u8>> = vec![vec![], vec![0], vec![0xff], vec![0x80; np.max(1)]];
        for _ in 0..6 {
            let n = 1 + rng.below((np.max(1) + 2) as u32) as usize;
            alphas.push(rng.bytes(n));
        }
        for a in alphas {
            let n = a.len() as c_int;
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_tRNS)(
                    png,
                    info,
                    if a.is_empty() { null() } else { a.as_ptr() },
                    n,
                    null(),
                );
            };
            rt(&format!("tRNS pal {} n={}", b.label, n), &b, &f);
        }
        // gray / rgb forms: a colour value
        let mut cols: Vec<png_color_16> = vec![
            png_color_16 { index: 0, red: 0, green: 0, blue: 0, gray: 0 },
            png_color_16 {
                index: 0,
                red: smax as u16,
                green: smax as u16,
                blue: smax as u16,
                gray: smax as u16,
            },
            png_color_16 { index: 3, red: 0xffff, green: 0xffff, blue: 0xffff, gray: 0xffff },
            png_color_16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 },
            png_color_16 { index: 0, red: 256, green: 256, blue: 256, gray: 256 },
        ];
        for _ in 0..8 {
            cols.push(png_color_16 {
                index: rng.u8(),
                red: rng.u32() as u16,
                green: rng.u32() as u16,
                blue: rng.u32() as u16,
                gray: rng.u32() as u16,
            });
        }
        for c in cols {
            for nt in [0i32, 1, 2] {
                let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                    (api.png_set_tRNS)(png, info, null(), nt, &c);
                };
                rt(&format!("tRNS col {} {:?} nt={}", b.label, c, nt), &b, &f);
            }
        }
        // both at once
        {
            let a: Vec<u8> = (0..np.max(1)).map(|i| (i * 7) as u8).collect();
            let c = png_color_16 { index: 1, red: 5, green: 6, blue: 7, gray: 8 };
            let n = a.len() as c_int;
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_tRNS)(png, info, a.as_ptr(), n, &c);
            };
            rt(&format!("tRNS both {}", b.label), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// bKGD
// ---------------------------------------------------------------------------

#[test]
fn chunk_bKGD() {
    let mut rng = Rng::new(0xb69d_0007);
    for b in bases() {
        let np = b.num_palette() as u32;
        let smax = b.sample_max();
        let mut cases: Vec<png_color_16> = vec![
            png_color_16 { index: 0, red: 0, green: 0, blue: 0, gray: 0 },
            png_color_16 {
                index: (np.max(1) - 1) as u8,
                red: smax as u16,
                green: smax as u16,
                blue: smax as u16,
                gray: smax as u16,
            },
            png_color_16 { index: 255, red: 0xffff, green: 0xffff, blue: 0xffff, gray: 0xffff },
            png_color_16 { index: np as u8, red: 1, green: 2, blue: 3, gray: 4 },
            png_color_16 { index: 0, red: 300, green: 300, blue: 300, gray: 300 },
        ];
        for _ in 0..10 {
            cases.push(png_color_16 {
                index: rng.u8(),
                red: rng.u32() as u16,
                green: rng.u32() as u16,
                blue: rng.u32() as u16,
                gray: rng.u32() as u16,
            });
        }
        for c in cases {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_bKGD)(png, info, &c);
            };
            rt(&format!("bKGD {} {:?}", b.label, c), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// hIST
// ---------------------------------------------------------------------------

#[test]
fn chunk_hIST() {
    let mut rng = Rng::new(0x4157_0008);
    for b in bases() {
        // png_set_hIST reads info_ptr->num_palette entries, so the buffer must
        // always be at least that long (fewer would read past the end -- C UB).
        let n = 256usize;
        let mut cases: Vec<Vec<u16>> = vec![
            vec![0u16; n],
            vec![1u16; n],
            vec![0xffff; n],
            (0..n).map(|i| i as u16).collect(),
        ];
        for _ in 0..8 {
            cases.push((0..n).map(|_| rng.u32() as u16).collect());
        }
        for (i, h) in cases.into_iter().enumerate() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_hIST)(png, info, h.as_ptr());
            };
            rt(&format!("hIST#{} {}", i, b.label), &b, &f);
        }
    }
}

/// `png_write_info` always emits hIST *after* PLTE, but this tree's read-side
/// chunk table gives hIST `pos_before = PNG_HAVE_PLTE`, so the round trip above
/// only ever reaches the "out of place" rejection.  Reorder the produced stream
/// so that `png_handle_hIST`'s body (its length / num_palette checks) is
/// actually entered, and compare that too.
#[test]
fn chunk_hIST_reordered() {
    for b in bases() {
        for n in [0usize, 1, 16, 128, 256] {
            let h: Vec<u16> = (0..256).map(|i| (i as u16).wrapping_mul(37)).collect();
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_hIST)(png, info, h.as_ptr());
            };
            unsafe {
                let cw = write_run(c_api(), &b, &f, &nop);
                let rw = write_run(rs_api(), &b, &f, &nop);
                assert_bytes_eq(&format!("hIST-reorder {} n={}", b.label, n), &cw.bytes, &rw.bytes);
                if let Some(moved) = move_before(&cw.bytes, b"hIST", b"PLTE") {
                    diff_read(
                        &format!("hIST before PLTE {} n={}", b.label, n),
                        &moved,
                        &nop,
                    );
                }
                // also try it in front of IDAT and after IEND-adjacent chunks
                if let Some(moved) = move_before(&cw.bytes, b"hIST", b"IDAT") {
                    diff_read(
                        &format!("hIST before IDAT {} n={}", b.label, n),
                        &moved,
                        &nop,
                    );
                }
                if let Some(moved) = move_before(&cw.bytes, b"hIST", b"IHDR") {
                    diff_read(
                        &format!("hIST before IHDR {} n={}", b.label, n),
                        &moved,
                        &nop,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// pHYs (+ all the derived accessors, exercised inside probe_all)
// ---------------------------------------------------------------------------

#[test]
fn chunk_pHYs() {
    let mut rng = Rng::new(0x0845_0009);
    let mut cases: Vec<(u32, u32, c_int)> = vec![
        (0, 0, 0),
        (0, 0, 1),
        (1, 1, 0),
        (1, 1, 1),
        (2835, 2835, 1),
        (2835, 1417, 1),
        (PNG_UINT_31_MAX, PNG_UINT_31_MAX, 1),
        (PNG_UINT_31_MAX, 1, 1),
        (1, PNG_UINT_31_MAX, 1),
        (0xffff_ffff, 0xffff_ffff, 1),
        (0xffff_ffff, 1, 0),
        (100, 0, 1),
        (0, 100, 1),
        (72, 72, 2),
        (72, 72, 255),
        (72, 72, -1),
    ];
    for _ in 0..12 {
        cases.push((
            rng.u32() % PNG_UINT_31_MAX,
            rng.u32() % PNG_UINT_31_MAX,
            rng.below(3) as c_int,
        ));
    }
    for b in bases() {
        for (x, y, u) in cases.clone() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_pHYs)(png, info, x, y, u);
            };
            rt(&format!("pHYs {} {} {} {}", b.label, x, y, u), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// oFFs (+ all the derived accessors)
// ---------------------------------------------------------------------------

#[test]
fn chunk_oFFs() {
    let mut rng = Rng::new(0x0ff5_000a);
    let mut cases: Vec<(i32, i32, c_int)> = vec![
        (0, 0, 0),
        (0, 0, 1),
        (1, -1, 0),
        (-1, 1, 1),
        (i32::MAX, i32::MAX, 0),
        (i32::MIN, i32::MIN, 1),
        (i32::MAX, i32::MIN, 1),
        (PNG_UINT_31_MAX as i32, -(PNG_UINT_31_MAX as i32), 1),
        (1000, 2000, 2),
        (1000, 2000, 255),
        (1000, 2000, -1),
        (5_000_000, -5_000_000, 1),
    ];
    for _ in 0..12 {
        cases.push((
            rng.range(i32::MIN as i64, i32::MAX as i64) as i32,
            rng.range(i32::MIN as i64, i32::MAX as i64) as i32,
            rng.below(3) as c_int,
        ));
    }
    for b in bases() {
        for (x, y, u) in cases.clone() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_oFFs)(png, info, x, y, u);
            };
            rt(&format!("oFFs {} {} {} {}", b.label, x, y, u), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// sCAL
// ---------------------------------------------------------------------------

#[test]
fn chunk_sCAL() {
    let mut rng = Rng::new(0x5ca1_000b);
    let mut dbl: Vec<(c_int, f64, f64)> = vec![
        (1, 1.0, 1.0),
        (2, 1.0, 1.0),
        (0, 1.0, 1.0),
        (3, 1.0, 1.0),
        (-1, 1.0, 1.0),
        (1, 0.0, 1.0),
        (1, 1.0, 0.0),
        (1, -1.0, 1.0),
        (1, 1.0, -1.0),
        (1, 1e-10, 1e10),
        (1, 1e30, 1e-30),
        (1, 0.000_001, 1_000_000.0),
        (2, 123.456, 0.000_789),
    ];
    for _ in 0..10 {
        dbl.push((
            1 + rng.below(2) as c_int,
            (rng.u32() % 1_000_000) as f64 / 1000.0 + 0.001,
            (rng.u32() % 1_000_000) as f64 / 1000.0 + 0.001,
        ));
    }
    let mut fixd: Vec<(c_int, i32, i32)> = vec![
        (1, 1, 1),
        (1, 0, 1),
        (1, 1, 0),
        (1, -1, 1),
        (2, PNG_FP_MAX, PNG_FP_MAX),
        (1, PNG_FP_MIN, 1),
        (1, 100_000, 100_000),
        (2, 1, PNG_FP_MAX),
        (0, 1, 1),
        (5, 1, 1),
    ];
    for _ in 0..10 {
        fixd.push((
            1 + rng.below(2) as c_int,
            1 + (rng.u32() % 1_000_000) as i32,
            1 + (rng.u32() % 1_000_000) as i32,
        ));
    }
    let strs: Vec<(c_int, &str, &str)> = vec![
        (1, "1", "1"),
        (2, "1.0", "2.0"),
        (1, "1e10", "1e-10"),
        (1, "1E30", "1"),
        (1, "0", "0"),
        (1, "", "1"),
        (1, "1", ""),
        (1, "-1", "1"),
        (1, "1", "-1"),
        (1, "abc", "1"),
        (1, "1.2.3", "1"),
        (1, "+1", "1"),
        (1, ".5", "0.5"),
        (1, "1e", "1"),
        (0, "1", "1"),
        (3, "1", "1"),
        (1, "99999999999999999999999999999999", "1"),
        (
            1,
            "1.23456789012345678901234567890123456789012345678901234567890123456789",
            "1",
        ),
    ];

    for b in bases() {
        for (u, w, h) in dbl.clone() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_sCAL)(png, info, u, w, h);
            };
            rt(&format!("sCAL {} {} {} {}", b.label, u, w, h), &b, &f);
        }
        for (u, w, h) in fixd.clone() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_sCAL_fixed)(png, info, u, w, h);
            };
            rt(&format!("sCAL_fixed {} {} {} {}", b.label, u, w, h), &b, &f);
        }
        for (u, w, h) in strs.clone() {
            let (ws, hs) = (w.to_string(), h.to_string());
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let a = cs(&ws);
                let c = cs(&hs);
                (api.png_set_sCAL_s)(png, info, u, a.as_ptr(), c.as_ptr());
            };
            rt(&format!("sCAL_s {} {} {:?} {:?}", b.label, u, w, h), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// pCAL
// ---------------------------------------------------------------------------

#[test]
fn chunk_pCAL() {
    let mut rng = Rng::new(0x9ca1_000c);
    let mut cases: Vec<(String, i32, i32, c_int, Vec<String>, String)> = vec![
        ("p".into(), 0, 255, 0, vec!["0".into(), "1".into()], "u".into()),
        ("linear".into(), 0, 255, 0, vec!["1".into(), "2".into()], "m".into()),
        ("euler".into(), 0, 255, 1, vec!["1".into(), "2".into(), "3".into()], "m".into()),
        (
            "arb".into(),
            -100,
            100,
            2,
            vec!["1".into(), "2".into(), "3".into(), "4".into()],
            "m".into(),
        ),
        (
            "hyp".into(),
            i32::MIN,
            i32::MAX,
            3,
            vec!["1".into(), "2".into(), "3".into(), "4".into()],
            "m".into(),
        ),
        // out-of-range equation types
        ("bad".into(), 0, 1, -1, vec!["1".into()], "u".into()),
        ("bad".into(), 0, 1, 4, vec!["1".into()], "u".into()),
        ("bad".into(), 0, 1, 255, vec!["1".into()], "u".into()),
        // no params at all
        ("none".into(), 0, 1, 0, vec![], "u".into()),
        // malformed floating point parameters
        ("np".into(), 0, 1, 0, vec!["x".into()], "u".into()),
        ("np".into(), 0, 1, 0, vec!["".into()], "u".into()),
        ("np".into(), 0, 1, 0, vec!["1".into(), "zz".into()], "u".into()),
        ("np".into(), 0, 1, 0, vec!["1e".into()], "u".into()),
        ("np".into(), 0, 1, 0, vec!["-1.5e+3".into()], "u".into()),
        // invalid keywords
        ("".into(), 0, 1, 0, vec!["1".into()], "u".into()),
        (" lead".into(), 0, 1, 0, vec!["1".into()], "u".into()),
        ("trail ".into(), 0, 1, 0, vec!["1".into()], "u".into()),
        // X0 == X1 (a read-side error)
        ("eq".into(), 7, 7, 0, vec!["1".into(), "2".into()], "u".into()),
    ];
    for _ in 0..8 {
        let n = rng.below(5) as usize;
        let params: Vec<String> = (0..n)
            .map(|_| format!("{}.{}", rng.range(-999, 999), rng.below(1000)))
            .collect();
        cases.push((
            format!("p{}", rng.below(1000)),
            rng.range(i32::MIN as i64, i32::MAX as i64) as i32,
            rng.range(i32::MIN as i64, i32::MAX as i64) as i32,
            rng.below(4) as c_int,
            params,
            format!("u{}", rng.below(100)),
        ));
    }

    for b in bases() {
        for (purpose, x0, x1, ty, params, units) in cases.clone() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let p = cs(&purpose);
                let u = cs(&units);
                let owned: Vec<std::ffi::CString> = params.iter().map(|s| cs(s)).collect();
                let mut ptrs: Vec<png_charp> =
                    owned.iter().map(|c| c.as_ptr() as png_charp).collect();
                (api.png_set_pCAL)(
                    png,
                    info,
                    p.as_ptr(),
                    x0,
                    x1,
                    ty,
                    ptrs.len() as c_int,
                    u.as_ptr(),
                    if ptrs.is_empty() {
                        null_mut()
                    } else {
                        ptrs.as_mut_ptr()
                    },
                );
            };
            rt(
                &format!("pCAL {} {} {} {}", b.label, x0, x1, ty),
                &b,
                &f,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// tIME (both before IDAT via png_write_info and after via png_write_end)
// ---------------------------------------------------------------------------

#[test]
fn chunk_tIME() {
    let mut rng = Rng::new(0x7146_000d);
    let mut cases: Vec<png_time> = vec![
        png_time { year: 0, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 1970, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2026, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
        png_time { year: 0xffff, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
        // each field out of range in turn -> "Ignoring invalid time value"
        png_time { year: 2000, month: 0, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 60, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
        png_time { year: 2000, month: 255, day: 255, hour: 255, minute: 255, second: 255 },
    ];
    for _ in 0..10 {
        cases.push(png_time {
            year: rng.u32() as u16,
            month: 1 + rng.u8() % 12,
            day: 1 + rng.u8() % 31,
            hour: rng.u8() % 24,
            minute: rng.u8() % 60,
            second: rng.u8() % 61,
        });
    }
    for b in bases() {
        for t in cases.clone() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_tIME)(png, info, &t);
            };
            rt(&format!("tIME {} {:?}", b.label, t), &b, &f);
            // set it only after the image data: png_write_end writes it then
            roundtrip(
                &format!("tIME-end {} {:?}", b.label, t),
                &b,
                &nop,
                &f,
                &nop,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// tEXt / zTXt / iTXt
// ---------------------------------------------------------------------------

const TEXT_COMPRESSIONS: &[c_int] = &[
    PNG_TEXT_COMPRESSION_NONE_WR,
    PNG_TEXT_COMPRESSION_zTXt_WR,
    PNG_TEXT_COMPRESSION_NONE,
    PNG_TEXT_COMPRESSION_zTXt,
    PNG_ITXT_COMPRESSION_NONE,
    PNG_ITXT_COMPRESSION_zTXt,
    PNG_TEXT_COMPRESSION_LAST,
    -4,
    4,
    99,
];

#[test]
fn chunk_text() {
    let mut rng = Rng::new(0x7e87_000e);
    for b in bases() {
        for &comp in TEXT_COMPRESSIONS {
            for (key, text) in [
                ("Title", "hello"),
                ("Comment", ""),
                ("k", "x"),
                (
                    "Description",
                    "a longer piece of text, repeated: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                ),
            ] {
                let (k, t) = (key.to_string(), text.to_string());
                let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                    let kk = cs(&k);
                    let tt = cs(&t);
                    let lang = cs("en-GB");
                    let lkey = cs("Titel");
                    let e = png_text {
                        compression: comp,
                        key: kk.as_ptr() as png_charp,
                        text: tt.as_ptr() as png_charp,
                        text_length: t.len(),
                        itxt_length: 0,
                        // iTXt entries need a non-NULL language (see header)
                        lang: if comp > 0 {
                            lang.as_ptr() as png_charp
                        } else {
                            null_mut()
                        },
                        lang_key: if comp > 0 {
                            lkey.as_ptr() as png_charp
                        } else {
                            null_mut()
                        },
                    };
                    (api.png_set_text)(png, info, &e, 1);
                };
                rt(
                    &format!("text {} comp={} key={:?}", b.label, comp, key),
                    &b,
                    &f,
                );
                // The same entry, but written by png_write_end (after IDAT)
                roundtrip(
                    &format!("text-end {} comp={} key={:?}", b.label, comp, key),
                    &b,
                    &nop,
                    &f,
                    &nop,
                );
            }
        }
        // many entries at once, mixed compressions, via png_set_text_2
        for _ in 0..6 {
            let n = 1 + rng.below(5) as usize;
            let items: Vec<(c_int, String, String)> = (0..n)
                .map(|i| {
                    (
                        TEXT_COMPRESSIONS[rng.below(6) as usize],
                        format!("Key{}", i),
                        format!("value {} {}", i, rng.u32()),
                    )
                })
                .collect();
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let mut keep: Vec<std::ffi::CString> = Vec::new();
                let lang = cs("de");
                let lkey = cs("Schl");
                let mut texts: Vec<png_text> = Vec::new();
                for (c, k, t) in &items {
                    keep.push(cs(k));
                    let kp = keep.last().unwrap().as_ptr() as png_charp;
                    keep.push(cs(t));
                    let tp = keep.last().unwrap().as_ptr() as png_charp;
                    texts.push(png_text {
                        compression: *c,
                        key: kp,
                        text: tp,
                        text_length: t.len(),
                        itxt_length: 0,
                        lang: if *c > 0 {
                            lang.as_ptr() as png_charp
                        } else {
                            null_mut()
                        },
                        lang_key: if *c > 0 {
                            lkey.as_ptr() as png_charp
                        } else {
                            null_mut()
                        },
                    });
                }
                let r = (api.png_set_text_2)(png, info, texts.as_ptr(), texts.len() as c_int);
                // Fold the return value into the diagnostics so it is compared.
                if r != 0 {
                    (api.png_warning)(png, cs(&format!("set_text_2={}", r)).as_ptr());
                }
            };
            rt(&format!("text_2 {} n={}", b.label, n), &b, &f);
        }
        // NULL key / NULL text / zero-length text edge cases
        for (has_key, has_text) in [(false, true), (true, false)] {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let kk = cs("Key");
                let tt = cs("");
                let e = png_text {
                    compression: PNG_TEXT_COMPRESSION_NONE,
                    key: if has_key {
                        kk.as_ptr() as png_charp
                    } else {
                        null_mut()
                    },
                    text: if has_text {
                        tt.as_ptr() as png_charp
                    } else {
                        null_mut()
                    },
                    text_length: 0,
                    itxt_length: 0,
                    lang: null_mut(),
                    lang_key: null_mut(),
                };
                let r = (api.png_set_text_2)(png, info, &e, 1);
                if r != 0 {
                    (api.png_warning)(png, cs(&format!("set_text_2={}", r)).as_ptr());
                }
            };
            rt(
                &format!("text nulls {} key={} text={}", b.label, has_key, has_text),
                &b,
                &f,
            );
        }
        // num_text <= 0
        for n in [-1i32, 0] {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let kk = cs("Key");
                let tt = cs("v");
                let e = png_text {
                    compression: PNG_TEXT_COMPRESSION_NONE,
                    key: kk.as_ptr() as png_charp,
                    text: tt.as_ptr() as png_charp,
                    text_length: 1,
                    itxt_length: 0,
                    lang: null_mut(),
                    lang_key: null_mut(),
                };
                let r = (api.png_set_text_2)(png, info, &e, n);
                if r != 0 {
                    (api.png_warning)(png, cs(&format!("set_text_2={}", r)).as_ptr());
                }
            };
            rt(&format!("text n={} {}", n, b.label), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// sPLT
// ---------------------------------------------------------------------------

#[test]
fn chunk_sPLT() {
    let mut rng = Rng::new(0x5919_000f);
    for b in bases() {
        for depth in [8u8, 16] {
            for n in [0usize, 1, 2, 5, 17] {
                let entries: Vec<png_sPLT_entry> = (0..n)
                    .map(|_| png_sPLT_entry {
                        red: rng.u32() as u16,
                        green: rng.u32() as u16,
                        blue: rng.u32() as u16,
                        alpha: rng.u32() as u16,
                        frequency: rng.u32() as u16,
                    })
                    .collect();
                let name = format!("splt{}x{}", depth, n);
                let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                    let nm = cs(&name);
                    let s = png_sPLT_t {
                        name: nm.as_ptr() as png_charp,
                        depth,
                        entries: if entries.is_empty() {
                            null_mut()
                        } else {
                            entries.as_ptr() as png_sPLT_entryp
                        },
                        nentries: entries.len() as png_int_32,
                    };
                    (api.png_set_sPLT)(png, info, &s, 1);
                };
                rt(&format!("sPLT {} d={} n={}", b.label, depth, n), &b, &f);
            }
        }
        // invalid depths and keywords
        for (depth, name) in [(1u8, "d1"), (4, "d4"), (0, "d0"), (255, "d255"), (8, ""), (8, " x")] {
            let name = name.to_string();
            let name2 = name.clone();
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let nm = cs(&name2);
                let e = [png_sPLT_entry {
                    red: 1,
                    green: 2,
                    blue: 3,
                    alpha: 4,
                    frequency: 5,
                }];
                let s = png_sPLT_t {
                    name: nm.as_ptr() as png_charp,
                    depth,
                    entries: e.as_ptr() as png_sPLT_entryp,
                    nentries: 1,
                };
                (api.png_set_sPLT)(png, info, &s, 1);
            };
            rt(&format!("sPLT bad {} d={} n={:?}", b.label, depth, name), &b, &f);
        }
        // several palettes in one call, and nentries <= 0
        {
            let f = |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let n0 = cs("first");
                let n1 = cs("second");
                let e0 = [
                    png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 },
                    png_sPLT_entry { red: 6, green: 7, blue: 8, alpha: 9, frequency: 10 },
                ];
                let e1 = [png_sPLT_entry {
                    red: 0xffff,
                    green: 0,
                    blue: 0xffff,
                    alpha: 0,
                    frequency: 0xffff,
                }];
                let arr = [
                    png_sPLT_t {
                        name: n0.as_ptr() as png_charp,
                        depth: 8,
                        entries: e0.as_ptr() as png_sPLT_entryp,
                        nentries: 2,
                    },
                    png_sPLT_t {
                        name: n1.as_ptr() as png_charp,
                        depth: 16,
                        entries: e1.as_ptr() as png_sPLT_entryp,
                        nentries: 1,
                    },
                ];
                (api.png_set_sPLT)(png, info, arr.as_ptr(), 2);
                // nentries <= 0 and a NULL entries pointer
                (api.png_set_sPLT)(png, info, arr.as_ptr(), 0);
                (api.png_set_sPLT)(png, info, arr.as_ptr(), -1);
                (api.png_set_sPLT)(png, info, null(), 1);
            };
            rt(&format!("sPLT multi {}", b.label), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// eXIf
// ---------------------------------------------------------------------------

#[test]
fn chunk_eXIf() {
    let mut rng = Rng::new(0xe81f_0010);
    let mut cases: Vec<Vec<u8>> = vec![
        b"II*\0".to_vec(),
        b"MM\0*".to_vec(),
        {
            let mut v = b"II*\0".to_vec();
            v.extend_from_slice(&[8, 0, 0, 0, 0, 0]);
            v
        },
        {
            let mut v = b"MM\0*".to_vec();
            v.extend_from_slice(&[0, 0, 0, 8, 0, 0]);
            v
        },
        // invalid headers -> benign error on read
        b"XX\0\0".to_vec(),
        vec![0, 0, 0, 0],
        vec![0xff; 4],
        vec![],
        vec![1],
        vec![1, 2, 3],
    ];
    for _ in 0..8 {
        let mut v = if rng.bool() {
            b"II*\0".to_vec()
        } else {
            b"MM\0*".to_vec()
        };
        let n = 1 + rng.below(60) as usize;
        v.extend(rng.bytes(n));
        cases.push(v);
    }
    for b in bases() {
        for (i, e) in cases.clone().into_iter().enumerate() {
            let n = e.len() as png_uint_32;
            let e2 = e.clone();
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                // png_set_eXIf_1 returns early on a NULL pointer; use a valid
                // (possibly zero-length) buffer instead.
                let p = if e2.is_empty() {
                    b"".as_ptr() as png_bytep
                } else {
                    e2.as_ptr() as png_bytep
                };
                (api.png_set_eXIf_1)(png, info, n, p);
                // the deprecated setter only warns
                (api.png_set_eXIf)(png, info, p);
            };
            rt(&format!("eXIf#{} {} n={}", i, b.label, n), &b, &f);
            // after IDAT
            roundtrip(
                &format!("eXIf-end#{} {} n={}", i, b.label, n),
                &b,
                &nop,
                &f,
                &nop,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// cICP
// ---------------------------------------------------------------------------

#[test]
fn chunk_cICP() {
    let mut rng = Rng::new(0xc1c9_0011);
    let mut cases: Vec<(u8, u8, u8, u8)> = vec![
        (1, 13, 0, 1),
        (9, 16, 0, 0),
        (0, 0, 0, 0),
        (255, 255, 0, 255),
        // matrix_coefficients != 0 -> "Invalid cICP matrix coefficients"
        (1, 13, 1, 1),
        (1, 13, 255, 1),
    ];
    for _ in 0..12 {
        cases.push((rng.u8(), rng.u8(), if rng.bool() { 0 } else { rng.u8() }, rng.u8()));
    }
    for b in bases() {
        for (p, t, m, r) in cases.clone() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_cICP)(png, info, p, t, m, r);
            };
            rt(&format!("cICP {} {} {} {} {}", b.label, p, t, m, r), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// cLLI
// ---------------------------------------------------------------------------

#[test]
fn chunk_cLLI() {
    let mut rng = Rng::new(0xc111_0012);
    let mut fixed: Vec<(u32, u32)> = vec![
        (0, 0),
        (1, 1),
        (10_000, 10_000),
        (10_000_000, 4_000_000),
        (0x7fff_ffff, 0x7fff_ffff),
        (0x8000_0000, 0),
        (0, 0x8000_0000),
        (0xffff_ffff, 0xffff_ffff),
    ];
    for _ in 0..12 {
        fixed.push((rng.u32(), rng.u32() % 0x7fff_ffff));
    }
    let dbl: Vec<(f64, f64)> = vec![
        (0.0, 0.0),
        (1.0, 1.0),
        (1000.0, 400.0),
        (0.0001, 0.0001),
        (214748.3647, 1.0),
        (300000.0, 1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (1e30, 1.0),
    ];
    for b in bases() {
        for (a, c) in fixed.clone() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_cLLI_fixed)(png, info, a, c);
            };
            rt(&format!("cLLI_fixed {} {} {}", b.label, a, c), &b, &f);
        }
        for (a, c) in dbl.clone() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_cLLI)(png, info, a, c);
            };
            rt(&format!("cLLI {} {} {}", b.label, a, c), &b, &f);
        }
    }
}

// ---------------------------------------------------------------------------
// mDCV
// ---------------------------------------------------------------------------

#[test]
fn chunk_mDCV() {
    let mut rng = Rng::new(0x0dc9_0013);
    let mut fixed: Vec<([i32; 8], u32, u32)> = vec![
        ([31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000], 10_000_000, 50),
        ([0; 8], 0, 0),
        ([1; 8], 1, 1),
        ([131070; 8], 0x7fff_ffff, 0x7fff_ffff),
        ([131071; 8], 1, 1),
        ([131072; 8], 1, 1),
        ([-1; 8], 1, 1),
        ([-2; 8], 1, 1),
        ([100_000; 8], 0x8000_0000, 1),
        ([100_000; 8], 1, 0x8000_0000),
        ([PNG_FP_MAX; 8], 1, 1),
    ];
    for _ in 0..12 {
        let mut a = [0i32; 8];
        for x in a.iter_mut() {
            *x = rng.range(-10_000, 140_000) as i32;
        }
        fixed.push((a, rng.u32() % 0x7fff_ffff, rng.u32() % 0x7fff_ffff));
    }
    for b in bases() {
        for (v, mx, mn) in fixed.clone() {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_mDCV_fixed)(
                    png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], mx, mn,
                );
            };
            rt(&format!("mDCV_fixed {} {:?} {} {}", b.label, v, mx, mn), &b, &f);

            let d: [f64; 8] = std::array::from_fn(|i| v[i] as f64 / 100_000.0);
            let (dx, dn) = (mx as f64 / 10_000.0, mn as f64 / 10_000.0);
            let g = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                (api.png_set_mDCV)(
                    png, info, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7], dx, dn,
                );
            };
            rt(&format!("mDCV {} {:?} {} {}", b.label, d, dx, dn), &b, &g);
        }
    }
}

// ---------------------------------------------------------------------------
// Unknown chunks
// ---------------------------------------------------------------------------

const LOCATIONS: &[c_int] = &[
    PNG_HAVE_IHDR as c_int,
    PNG_HAVE_PLTE as c_int,
    PNG_AFTER_IDAT as c_int,
    (PNG_HAVE_IHDR | PNG_HAVE_PLTE) as c_int,
    (PNG_HAVE_IHDR | PNG_AFTER_IDAT) as c_int,
    (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT) as c_int,
];

fn chunk_name(s: &[u8; 4]) -> [png_byte; 5] {
    [s[0], s[1], s[2], s[3], 0]
}

#[test]
fn chunk_unknown() {
    let mut rng = Rng::new(0x0f_0014);
    let names: &[&[u8; 4]] = &[b"vpAg", b"prVt", b"ABCD", b"abcd", b"zzZz", b"sTER"];
    for b in bases() {
        for loc in LOCATIONS.iter().copied() {
            for name in names {
                for size in [0usize, 1, 7, 64] {
                    let data = rng.bytes(size);
                    let nm = chunk_name(name);
                    let d2 = data.clone();
                    let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                        let u = png_unknown_chunk {
                            name: nm,
                            data: if d2.is_empty() {
                                null_mut()
                            } else {
                                d2.as_ptr() as *mut png_byte
                            },
                            size: d2.len(),
                            location: loc as png_byte,
                        };
                        (api.png_set_unknown_chunks)(png, info, &u, 1);
                    };
                    roundtrip(
                        &format!(
                            "unknown {} {} loc={} size={}",
                            b.label,
                            String::from_utf8_lossy(*name),
                            loc,
                            size
                        ),
                        &b,
                        &f,
                        &nop,
                        &keep_all,
                    );
                }
            }
        }
        // location 0 on a write struct is an app warning (fatal in this build)
        {
            let f = |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let d = [1u8, 2, 3];
                let u = png_unknown_chunk {
                    name: chunk_name(b"loC0"),
                    data: d.as_ptr() as *mut png_byte,
                    size: 3,
                    location: 0,
                };
                (api.png_set_unknown_chunks)(png, info, &u, 1);
            };
            roundtrip(&format!("unknown loc0 {}", b.label), &b, &f, &nop, &keep_all);
        }
        // num_unknowns <= 0 and NULL array: silently ignored
        {
            let f = |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let d = [9u8];
                let u = png_unknown_chunk {
                    name: chunk_name(b"nOpe"),
                    data: d.as_ptr() as *mut png_byte,
                    size: 1,
                    location: PNG_HAVE_IHDR as png_byte,
                };
                (api.png_set_unknown_chunks)(png, info, &u, 0);
                (api.png_set_unknown_chunks)(png, info, &u, -1);
                (api.png_set_unknown_chunks)(png, info, null(), 1);
            };
            roundtrip(&format!("unknown none {}", b.label), &b, &f, &nop, &keep_all);
        }
        // several chunks at once + png_set_unknown_chunk_location afterwards
        for relocate in LOCATIONS.iter().copied().chain([0, 8, 16, -1]) {
            let f = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let d0 = [1u8, 2, 3, 4];
                let d1 = [5u8, 6];
                let arr = [
                    png_unknown_chunk {
                        name: chunk_name(b"muL1"),
                        data: d0.as_ptr() as *mut png_byte,
                        size: 4,
                        location: PNG_HAVE_IHDR as png_byte,
                    },
                    png_unknown_chunk {
                        name: chunk_name(b"muL2"),
                        data: d1.as_ptr() as *mut png_byte,
                        size: 2,
                        location: PNG_AFTER_IDAT as png_byte,
                    },
                ];
                (api.png_set_unknown_chunks)(png, info, arr.as_ptr(), 2);
                // in-range, and out-of-range chunk indices (checked by libpng)
                (api.png_set_unknown_chunk_location)(png, info, 0, relocate);
                (api.png_set_unknown_chunk_location)(png, info, 1, relocate);
                (api.png_set_unknown_chunk_location)(png, info, 2, relocate);
                (api.png_set_unknown_chunk_location)(png, info, -1, relocate);
            };
            roundtrip(
                &format!("unknown multi {} reloc={}", b.label, relocate),
                &b,
                &f,
                &nop,
                &keep_all,
            );
        }
    }
}

#[test]
fn keep_unknown_chunks_and_handle_as_unknown() {
    // png_set_keep_unknown_chunks / png_handle_as_unknown, exercised on a
    // write struct (no IO needed) so that every combination can be compared.
    let probe: &[&[u8; 5]] = &[
        b"vpAg\0", b"prVt\0", b"ABCD\0", b"bKGD\0", b"gAMA\0", b"IHDR\0", b"IDAT\0", b"tEXt\0",
        b"sTER\0", b"zzzz\0",
    ];
    let keeps = [
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        PNG_HANDLE_CHUNK_NEVER,
        PNG_HANDLE_CHUNK_IF_SAFE,
        PNG_HANDLE_CHUNK_ALWAYS,
        PNG_HANDLE_CHUNK_LAST,
        -1,
        99,
    ];
    // 5-byte-per-entry chunk lists
    let lists: Vec<(&str, Vec<u8>, c_int)> = vec![
        ("empty/0", vec![], 0),
        ("empty/-1", vec![], -1),
        ("null/1", vec![], 1),
        ("one", b"vpAg\0".to_vec(), 1),
        ("two", b"vpAg\0prVt\0".to_vec(), 2),
        ("three", b"vpAg\0bKGD\0ABCD\0".to_vec(), 3),
        ("dup", b"vpAg\0vpAg\0".to_vec(), 2),
    ];

    for (lname, list, n) in lists {
        for keep in keeps {
            let mut out: Vec<Vec<String>> = Vec::new();
            let mut diags: Vec<Diag> = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    let sess = WriteSess::new(api);
                    let png = sess.png;
                    let mut v: Vec<String> = Vec::new();
                    let ok = guard(|| {
                        let p = if list.is_empty() {
                            null()
                        } else {
                            list.as_ptr()
                        };
                        (api.png_set_keep_unknown_chunks)(png, keep, p, n);
                    })
                    .is_some();
                    v.push(format!("set.ok={}", ok));
                    for name in probe {
                        v.push(format!(
                            "{} -> {:?}",
                            String::from_utf8_lossy(&name[..4]),
                            guard(|| (api.png_handle_as_unknown)(png, name.as_ptr()))
                        ));
                    }
                    // a second call, layering a different keep on top
                    let ok2 = guard(|| {
                        (api.png_set_keep_unknown_chunks)(
                            png,
                            PNG_HANDLE_CHUNK_NEVER,
                            b"vpAg\0ABCD\0".as_ptr(),
                            2,
                        );
                    })
                    .is_some();
                    v.push(format!("set2.ok={}", ok2));
                    for name in probe {
                        v.push(format!(
                            "2:{} -> {:?}",
                            String::from_utf8_lossy(&name[..4]),
                            guard(|| (api.png_handle_as_unknown)(png, name.as_ptr()))
                        ));
                    }
                    // reset everything back to the default
                    let ok3 = guard(|| {
                        (api.png_set_keep_unknown_chunks)(
                            png,
                            PNG_HANDLE_CHUNK_AS_DEFAULT,
                            b"vpAg\0ABCD\0prVt\0bKGD\0".as_ptr(),
                            4,
                        );
                    })
                    .is_some();
                    v.push(format!("set3.ok={}", ok3));
                    for name in probe {
                        v.push(format!(
                            "3:{} -> {:?}",
                            String::from_utf8_lossy(&name[..4]),
                            guard(|| (api.png_handle_as_unknown)(png, name.as_ptr()))
                        ));
                    }
                    diags.push(diag_take());
                    out.push(v);
                }
            }
            let label = format!("keep {} keep={}", lname, keep);
            assert_eq!(diags[0], diags[1], "{}: diagnostics", label);
            assert_vals_eq(&label, &out[0], &out[1]);
        }
    }
}

#[test]
fn keep_unknown_chunks_on_read() {
    // The same, but driving a real read so that the keep setting actually
    // changes which chunks end up in the info struct.
    let b = &bases()[2]; // RGB@8
    for keep in [
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        PNG_HANDLE_CHUNK_NEVER,
        PNG_HANDLE_CHUNK_IF_SAFE,
        PNG_HANDLE_CHUNK_ALWAYS,
    ] {
        for list in [false, true] {
            let setup = |api: &'static Api, png: png_structp, info: png_infop| unsafe {
                let d0 = [1u8, 2, 3, 4];
                let d1 = [7u8, 8];
                let arr = [
                    // a safe-to-copy (lowercase 2nd letter) private chunk
                    png_unknown_chunk {
                        name: chunk_name(b"vpAg"),
                        data: d0.as_ptr() as *mut png_byte,
                        size: 4,
                        location: PNG_HAVE_IHDR as png_byte,
                    },
                    // an unsafe-to-copy private chunk
                    png_unknown_chunk {
                        name: chunk_name(b"prVt"),
                        data: d1.as_ptr() as *mut png_byte,
                        size: 2,
                        location: PNG_AFTER_IDAT as png_byte,
                    },
                ];
                (api.png_set_unknown_chunks)(png, info, arr.as_ptr(), 2);
            };
            let rpre = move |api: &'static Api, png: png_structp, _i: png_infop| unsafe {
                if list {
                    (api.png_set_keep_unknown_chunks)(png, keep, b"vpAg\0".as_ptr(), 1);
                } else {
                    (api.png_set_keep_unknown_chunks)(png, keep, null(), 0);
                }
            };
            roundtrip(
                &format!("read-keep keep={} list={}", keep, list),
                b,
                &setup,
                &nop,
                &rpre,
            );
        }
    }
    // num_chunks_in < 0: libpng substitutes its built-in "ignore every known
    // ancillary chunk" list, so a stream carrying every chunk gets read with
    // all of them routed through png_handle_unknown instead.
    let full = bases();
    for bx in &full {
        let bb = bx.clone();
        let setup = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
            fill_everything(api, png, info, &bb);
        };
        for keep in [
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
        ] {
            let rpre = move |api: &'static Api, png: png_structp, _i: png_infop| unsafe {
                (api.png_set_keep_unknown_chunks)(png, keep, null(), -1);
            };
            roundtrip(
                &format!("read-keep-all-known {} keep={}", bx.label, keep),
                bx,
                &setup,
                &nop,
                &rpre,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// png_free_data / png_data_freer
// ---------------------------------------------------------------------------

const FREE_MASKS: &[(&str, c_int)] = &[
    ("HIST", PNG_FREE_HIST),
    ("ICCP", PNG_FREE_ICCP),
    ("SPLT", PNG_FREE_SPLT),
    ("ROWS", PNG_FREE_ROWS),
    ("PCAL", PNG_FREE_PCAL),
    ("SCAL", PNG_FREE_SCAL),
    ("UNKN", PNG_FREE_UNKN),
    ("PLTE", PNG_FREE_PLTE),
    ("TRNS", PNG_FREE_TRNS),
    ("TEXT", PNG_FREE_TEXT),
    ("EXIF", PNG_FREE_EXIF),
    ("ALL", PNG_FREE_ALL),
    ("MUL", PNG_FREE_MUL),
    ("zero", 0),
];

/// Fill an info struct with one instance of every ancillary chunk.
unsafe fn fill_everything(api: &'static Api, png: png_structp, info: png_infop, b: &Base) {
    (api.png_set_IHDR)(
        png,
        info,
        b.width,
        b.height,
        b.bit_depth,
        b.color_type,
        PNG_INTERLACE_NONE,
        PNG_COMPRESSION_TYPE_BASE,
        PNG_FILTER_TYPE_BASE,
    );
    let pal: Vec<png_color> = if b.palette.is_empty() {
        (0..16)
            .map(|i| png_color {
                red: i as u8,
                green: (i * 3) as u8,
                blue: (i * 5) as u8,
            })
            .collect()
    } else {
        b.palette.clone()
    };
    (api.png_set_PLTE)(png, info, pal.as_ptr(), pal.len() as c_int);
    (api.png_set_gAMA_fixed)(png, info, 45455);
    (api.png_set_cHRM_fixed)(png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
    (api.png_set_sRGB)(png, info, 0);
    let prof = good_icc(b.color_type);
    let iname = cs("prof");
    (api.png_set_iCCP)(
        png,
        info,
        iname.as_ptr(),
        PNG_COMPRESSION_TYPE_BASE,
        prof.as_ptr(),
        prof.len() as png_uint_32,
    );
    let sb = png_color_8 {
        red: 4,
        green: 4,
        blue: 4,
        gray: 4,
        alpha: 4,
    };
    (api.png_set_sBIT)(png, info, &sb);
    let alpha = vec![0x80u8; pal.len()];
    (api.png_set_tRNS)(png, info, alpha.as_ptr(), alpha.len() as c_int, null());
    let bk = png_color_16 {
        index: 1,
        red: 2,
        green: 3,
        blue: 4,
        gray: 5,
    };
    (api.png_set_bKGD)(png, info, &bk);
    let hist = vec![7u16; 256];
    (api.png_set_hIST)(png, info, hist.as_ptr());
    (api.png_set_pHYs)(png, info, 2835, 2835, 1);
    (api.png_set_oFFs)(png, info, -10, 20, 1);
    (api.png_set_sCAL_fixed)(png, info, 1, 100_000, 200_000);
    {
        let purpose = cs("purp");
        let units = cs("un");
        let p0 = cs("1");
        let p1 = cs("2");
        let mut ptrs = [p0.as_ptr() as png_charp, p1.as_ptr() as png_charp];
        (api.png_set_pCAL)(
            png,
            info,
            purpose.as_ptr(),
            0,
            255,
            0,
            2,
            units.as_ptr(),
            ptrs.as_mut_ptr(),
        );
    }
    let t = png_time {
        year: 2026,
        month: 8,
        day: 31,
        hour: 12,
        minute: 34,
        second: 56,
    };
    (api.png_set_tIME)(png, info, &t);
    {
        let k = cs("Title");
        let v = cs("value");
        let e = png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: k.as_ptr() as png_charp,
            text: v.as_ptr() as png_charp,
            text_length: 5,
            itxt_length: 0,
            lang: null_mut(),
            lang_key: null_mut(),
        };
        (api.png_set_text)(png, info, &e, 1);
    }
    {
        let nm = cs("spal");
        let ents = [png_sPLT_entry {
            red: 1,
            green: 2,
            blue: 3,
            alpha: 4,
            frequency: 5,
        }];
        let s = png_sPLT_t {
            name: nm.as_ptr() as png_charp,
            depth: 8,
            entries: ents.as_ptr() as png_sPLT_entryp,
            nentries: 1,
        };
        (api.png_set_sPLT)(png, info, &s, 1);
    }
    {
        let mut exif = b"II*\0".to_vec();
        exif.extend_from_slice(&[8, 0, 0, 0]);
        (api.png_set_eXIf_1)(png, info, exif.len() as png_uint_32, exif.as_ptr() as png_bytep);
    }
    (api.png_set_cICP)(png, info, 1, 13, 0, 1);
    (api.png_set_cLLI_fixed)(png, info, 10_000_000, 50_000);
    (api.png_set_mDCV_fixed)(
        png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 10_000_000, 50,
    );
    {
        let d = [1u8, 2, 3, 4];
        let u = png_unknown_chunk {
            name: chunk_name(b"vpAg"),
            data: d.as_ptr() as *mut png_byte,
            size: 4,
            location: PNG_HAVE_IHDR as png_byte,
        };
        (api.png_set_unknown_chunks)(png, info, &u, 1);
    }
}

#[test]
fn free_data_and_data_freer() {
    for b in bases() {
        for &(mname, mask) in FREE_MASKS {
            for freer in [
                PNG_DATA_FREER,
                PNG_DESTROY_WILL_FREE_DATA,
                PNG_SET_WILL_FREE_DATA,
                PNG_USER_WILL_FREE_DATA,
                4,
                -1,
            ] {
                // `num` is only ever -1 or a known-in-range index; libpng does
                // not range-check it (C UB otherwise -- see the header note).
                //
                // For the three masks that consult `num` at all
                // (PNG_FREE_TEXT / _SPLT / _UNKN, i.e. PNG_FREE_MUL) a
                // per-item free leaves the *rest* of the entry dangling:
                // png_free_data(PNG_FREE_TEXT, 0) releases text[0].key but
                // leaves text[0].text (which points into the same block) and
                // num_text untouched, so any later png_get_text is a
                // use-after-free.  Those masks are therefore only exercised
                // with num == -1.
                let nums: &[i32] = if (mask & PNG_FREE_MUL) != 0 {
                    &[-1]
                } else {
                    &[-1, 0]
                };
                for &num in nums {
                    let mut out: Vec<Vec<String>> = Vec::new();
                    let mut diags: Vec<Diag> = Vec::new();
                    for api in both() {
                        unsafe {
                            set_current_api(api);
                            diag_reset();
                            let sess = WriteSess::new(api);
                            let png = sess.png;
                            let info = sess.info;
                            let mut v: Vec<String> = Vec::new();
                            let ok0 =
                                guard(|| fill_everything(api, png, info, &b)).is_some();
                            v.push(format!("fill.ok={}", ok0));
                            probe_all(api, png, info, "filled", &mut v, false);
                            let ok1 = guard(|| {
                                (api.png_data_freer)(png, info, freer, mask as png_uint_32)
                            })
                            .is_some();
                            v.push(format!("data_freer.ok={}", ok1));
                            let ok2 = guard(|| {
                                (api.png_free_data)(png, info, mask as png_uint_32, num)
                            })
                            .is_some();
                            v.push(format!("free_data.ok={}", ok2));
                            probe_all(api, png, info, "freed", &mut v, false);
                            // idempotence: freeing again must be safe
                            let ok3 = guard(|| {
                                (api.png_free_data)(png, info, mask as png_uint_32, num)
                            })
                            .is_some();
                            v.push(format!("free_data2.ok={}", ok3));
                            probe_all(api, png, info, "freed2", &mut v, false);
                            diags.push(diag_take());
                            out.push(v);
                        }
                    }
                    let label = format!(
                        "free_data {} mask={} freer={} num={}",
                        b.label, mname, freer, num
                    );
                    assert_eq!(diags[0], diags[1], "{}: diagnostics", label);
                    assert_vals_eq(&label, &out[0], &out[1]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// High-volume fuzz of the setter/getter pairs whose implementation is pure
// fixed-/floating-point arithmetic (png_fixed, png_float, png_muldiv,
// png_XYZ_from_xy, png_xy_from_XYZ, png_ascii_from_fp, atof, ppi_from_ppm).
// No IO, so thousands of samples are cheap.
// ---------------------------------------------------------------------------

unsafe fn probe_math(api: &'static Api, png: png_structp, info: png_infop, o: &mut Vec<String>) {
    let mut f8 = [-1i32; 8];
    o.push(format!(
        "cHRM_fixed {:?} {:?}",
        guard(|| (api.png_get_cHRM_fixed)(
            png, info, &mut f8[0], &mut f8[1], &mut f8[2], &mut f8[3], &mut f8[4], &mut f8[5],
            &mut f8[6], &mut f8[7]
        )),
        f8
    ));
    let mut d8 = [-1.0f64; 8];
    o.push(format!(
        "cHRM {:?} {}",
        guard(|| (api.png_get_cHRM)(
            png, info, &mut d8[0], &mut d8[1], &mut d8[2], &mut d8[3], &mut d8[4], &mut d8[5],
            &mut d8[6], &mut d8[7]
        )),
        fdv(&d8)
    ));
    let mut x9 = [-1i32; 9];
    o.push(format!(
        "cHRM_XYZ_fixed {:?} {:?}",
        guard(|| (api.png_get_cHRM_XYZ_fixed)(
            png, info, &mut x9[0], &mut x9[1], &mut x9[2], &mut x9[3], &mut x9[4], &mut x9[5],
            &mut x9[6], &mut x9[7], &mut x9[8]
        )),
        x9
    ));
    let mut y9 = [-1.0f64; 9];
    o.push(format!(
        "cHRM_XYZ {:?} {}",
        guard(|| (api.png_get_cHRM_XYZ)(
            png, info, &mut y9[0], &mut y9[1], &mut y9[2], &mut y9[3], &mut y9[4], &mut y9[5],
            &mut y9[6], &mut y9[7], &mut y9[8]
        )),
        fdv(&y9)
    ));
    let mut g = -1.0f64;
    let mut gf = -1i32;
    o.push(format!(
        "gAMA {:?} {} {:?} {}",
        guard(|| (api.png_get_gAMA)(png, info, &mut g)),
        fd(g),
        guard(|| (api.png_get_gAMA_fixed)(png, info, &mut gf)),
        gf
    ));
    let (mut rx, mut ry, mut ru) = (0u32, 0u32, -1i32);
    let (mut dx, mut dy, mut du) = (0u32, 0u32, -1i32);
    o.push(format!(
        "pHYs {:?} {} {} {} | dpi {:?} {} {} {}",
        guard(|| (api.png_get_pHYs)(png, info, &mut rx, &mut ry, &mut ru)),
        rx,
        ry,
        ru,
        guard(|| (api.png_get_pHYs_dpi)(png, info, &mut dx, &mut dy, &mut du)),
        dx,
        dy,
        du
    ));
    o.push(format!(
        "ppm {} {} {} ppi {} {} {} ar {} arf {:?}",
        (api.png_get_x_pixels_per_meter)(png, info),
        (api.png_get_y_pixels_per_meter)(png, info),
        (api.png_get_pixels_per_meter)(png, info),
        (api.png_get_x_pixels_per_inch)(png, info),
        (api.png_get_y_pixels_per_inch)(png, info),
        (api.png_get_pixels_per_inch)(png, info),
        match guard(|| (api.png_get_pixel_aspect_ratio)(png, info)) {
            Some(v) => ff(v),
            None => "<err>".to_string(),
        },
        guard(|| (api.png_get_pixel_aspect_ratio_fixed)(png, info)),
    ));
    let (mut ox, mut oy, mut ou) = (0i32, 0i32, -1i32);
    o.push(format!(
        "oFFs {:?} {} {} {}",
        guard(|| (api.png_get_oFFs)(png, info, &mut ox, &mut oy, &mut ou)),
        ox,
        oy,
        ou
    ));
    o.push(format!(
        "off px {} {} um {} {} in {} {} inf {:?} {:?}",
        (api.png_get_x_offset_pixels)(png, info),
        (api.png_get_y_offset_pixels)(png, info),
        (api.png_get_x_offset_microns)(png, info),
        (api.png_get_y_offset_microns)(png, info),
        match guard(|| (api.png_get_x_offset_inches)(png, info)) {
            Some(v) => ff(v),
            None => "<err>".to_string(),
        },
        match guard(|| (api.png_get_y_offset_inches)(png, info)) {
            Some(v) => ff(v),
            None => "<err>".to_string(),
        },
        guard(|| (api.png_get_x_offset_inches_fixed)(png, info)),
        guard(|| (api.png_get_y_offset_inches_fixed)(png, info)),
    ));
    let (mut su, mut sw, mut sh) = (-1i32, -1.0f64, -1.0f64);
    let (mut su2, mut sw2, mut sh2) = (-1i32, -1i32, -1i32);
    let (mut su3, mut sws, mut shs) = (-1i32, null_mut::<c_char>(), null_mut::<c_char>());
    o.push(format!(
        "sCAL {:?} {} {} {} | fixed {:?} {} {} {} | s {:?} {} {} {}",
        guard(|| (api.png_get_sCAL)(png, info, &mut su, &mut sw, &mut sh)),
        su,
        fd(sw),
        fd(sh),
        guard(|| (api.png_get_sCAL_fixed)(png, info, &mut su2, &mut sw2, &mut sh2)),
        su2,
        sw2,
        sh2,
        guard(|| (api.png_get_sCAL_s)(png, info, &mut su3, &mut sws, &mut shs)),
        su3,
        sstr(sws),
        sstr(shs)
    ));
    let (mut ca, mut cb) = (-1.0f64, -1.0f64);
    let (mut caf, mut cbf) = (0xffff_ffffu32, 0xffff_ffffu32);
    o.push(format!(
        "cLLI {:?} {} {} | fixed {:?} {} {}",
        guard(|| (api.png_get_cLLI)(png, info, &mut ca, &mut cb)),
        fd(ca),
        fd(cb),
        guard(|| (api.png_get_cLLI_fixed)(png, info, &mut caf, &mut cbf)),
        caf,
        cbf
    ));
    let mut m8 = [-1.0f64; 8];
    let (mut mmx, mut mmn) = (-1.0f64, -1.0f64);
    let mut mf8 = [-1i32; 8];
    let (mut mfx, mut mfn) = (0xffff_ffffu32, 0xffff_ffffu32);
    o.push(format!(
        "mDCV {:?} {} {} {} | fixed {:?} {:?} {} {}",
        guard(|| (api.png_get_mDCV)(
            png, info, &mut m8[0], &mut m8[1], &mut m8[2], &mut m8[3], &mut m8[4], &mut m8[5],
            &mut m8[6], &mut m8[7], &mut mmx, &mut mmn
        )),
        fdv(&m8),
        fd(mmx),
        fd(mmn),
        guard(|| (api.png_get_mDCV_fixed)(
            png, info, &mut mf8[0], &mut mf8[1], &mut mf8[2], &mut mf8[3], &mut mf8[4],
            &mut mf8[5], &mut mf8[6], &mut mf8[7], &mut mfx, &mut mfn
        )),
        mf8,
        mfx,
        mfn
    ));
}

#[test]
fn fuzz_fixed_and_floating_point_conversions() {
    const N: usize = 4000;
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut diags: Vec<Diag> = Vec::new();
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let mut rng = Rng::new(0xfeed_face_0000_0021);
            let mut v: Vec<String> = Vec::with_capacity(N * 12);
            for i in 0..N {
                let sess = WriteSess::new(api);
                let png = sess.png;
                let info = sess.info;
                v.push(format!("--- case {}", i));
                // Sample across several magnitudes so that overflow paths in
                // png_muldiv / png_fixed / png_xy_from_XYZ are all reached.
                let mag = |r: &mut Rng| -> i64 {
                    match r.below(6) {
                        0 => r.range(-3, 3),
                        1 => r.range(-100_000, 100_000),
                        2 => r.range(-2_000_000, 2_000_000),
                        3 => r.range(i32::MIN as i64, i32::MAX as i64),
                        4 => {
                            if r.bool() {
                                PNG_FP_MAX as i64
                            } else {
                                PNG_FP_MIN as i64
                            }
                        }
                        _ => r.range(0, 200_000),
                    }
                };
                let ok = guard(|| {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        7,
                        3,
                        8,
                        PNG_COLOR_TYPE_RGB,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (api.png_set_gAMA_fixed)(png, info, mag(&mut rng) as i32);
                    if rng.bool() {
                        (api.png_set_cHRM_fixed)(
                            png,
                            info,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                        );
                    } else {
                        (api.png_set_cHRM_XYZ_fixed)(
                            png,
                            info,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                        );
                    }
                    (api.png_set_pHYs)(
                        png,
                        info,
                        mag(&mut rng) as u32,
                        mag(&mut rng) as u32,
                        rng.below(3) as c_int,
                    );
                    (api.png_set_oFFs)(
                        png,
                        info,
                        mag(&mut rng) as i32,
                        mag(&mut rng) as i32,
                        rng.below(3) as c_int,
                    );
                    match rng.below(3) {
                        0 => (api.png_set_sCAL_fixed)(
                            png,
                            info,
                            1 + rng.below(2) as c_int,
                            mag(&mut rng) as i32,
                            mag(&mut rng) as i32,
                        ),
                        1 => (api.png_set_sCAL)(
                            png,
                            info,
                            1 + rng.below(2) as c_int,
                            mag(&mut rng) as f64 / 1000.0,
                            mag(&mut rng) as f64 * 1000.0,
                        ),
                        _ => {
                            let w = cs(&format!("{}e{}", rng.below(1000), rng.range(-40, 40)));
                            let h = cs(&format!("{}.{}", rng.below(1000), rng.below(100000)));
                            (api.png_set_sCAL_s)(
                                png,
                                info,
                                1 + rng.below(2) as c_int,
                                w.as_ptr(),
                                h.as_ptr(),
                            )
                        }
                    }
                    (api.png_set_cLLI_fixed)(png, info, mag(&mut rng) as u32, mag(&mut rng) as u32);
                    (api.png_set_mDCV_fixed)(
                        png,
                        info,
                        mag(&mut rng) as i32,
                        mag(&mut rng) as i32,
                        mag(&mut rng) as i32,
                        mag(&mut rng) as i32,
                        mag(&mut rng) as i32,
                        mag(&mut rng) as i32,
                        mag(&mut rng) as i32,
                        mag(&mut rng) as i32,
                        mag(&mut rng) as u32,
                        mag(&mut rng) as u32,
                    );
                })
                .is_some();
                v.push(format!("set.ok={}", ok));
                probe_math(api, png, info, &mut v);
            }
            diags.push(diag_take());
            out.push(v);
        }
    }
    assert_eq!(diags[0].warnings.len(), diags[1].warnings.len(), "fuzz: warning count");
    assert_eq!(diags[0], diags[1], "fuzz: diagnostics");
    assert_vals_eq("fuzz math", &out[0], &out[1]);
}

// ---------------------------------------------------------------------------
// Everything at once: a PNG carrying every ancillary chunk, round-tripped.
// ---------------------------------------------------------------------------

#[test]
fn all_chunks_at_once() {
    for b in bases() {
        let bb = b.clone();
        let setup = move |api: &'static Api, png: png_structp, info: png_infop| unsafe {
            fill_everything(api, png, info, &bb);
        };
        roundtrip(
            &format!("everything {}", b.label),
            &b,
            &setup,
            &nop,
            &keep_all,
        );
    }
}
