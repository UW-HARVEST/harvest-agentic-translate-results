//! Phase C -- the ancillary chunk APIs.
//!
//! Covers CONFIGS.md rows C-21 (`iccp`), C-60 (`text_compression`),
//! C-65 (`set_invalid`), C-107 ... C-127 (`round_trip*`), C-128 (`unknown`),
//! C-129 (`rows_and_freer`), C-130 (`text_many`), C-147 (`user_chunk_fn`)
//! and C-153 (`write_order`).
#![allow(non_snake_case)]

mod common;

use common::*;
use core::ffi::{c_char, c_int, c_void};
use core::ptr::null_mut;

/* ------------------------------------------------------------------ */
/* small utilities                                                     */
/* ------------------------------------------------------------------ */

/// A C string as raw bytes (never as text, so that non-UTF-8 keys compare).
unsafe fn bs(p: *const c_char) -> String {
    if p.is_null() {
        "<null>".to_string()
    } else {
        format!("{:02x?}", std::ffi::CStr::from_ptr(p).to_bytes())
    }
}

/// A NULL pointer whose pointee type is inferred at the call site.
fn nz<T>() -> *mut T {
    core::ptr::null_mut()
}

fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// A valid zlib stream built entirely out of "stored" deflate blocks, so the
/// test can hand libpng arbitrary *decompressed* chunk payloads without
/// needing a compressor.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x78u8, 0x01];
    if data.is_empty() {
        v.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    } else {
        let mut i = 0;
        while i < data.len() {
            let n = (data.len() - i).min(65535);
            let last = i + n == data.len();
            v.push(if last { 1 } else { 0 });
            v.extend_from_slice(&(n as u16).to_le_bytes());
            v.extend_from_slice(&(!(n as u16)).to_le_bytes());
            v.extend_from_slice(&data[i..i + n]);
            i += n;
        }
    }
    v.extend_from_slice(&adler32(data).to_be_bytes());
    v
}

/// The raw bytes (length+type+data+crc) of the first chunk called `name`.
fn raw_chunk_of(png: &[u8], name: &str) -> Option<Vec<u8>> {
    split_chunks(png)
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, r)| png[r].to_vec())
}

fn rgb_img(seed: u64) -> Img {
    let mut rng = Rng::new(seed);
    Img::random(&mut rng, 7, 3, PNG_COLOR_TYPE_RGB, 8)
}

/// A paletted 8-bit image with exactly `npal` entries and in-range indices.
fn pal_img(seed: u64, npal: usize) -> Img {
    let mut rng = Rng::new(seed);
    let mut img = Img::random(&mut rng, 7, 3, PNG_COLOR_TYPE_PALETTE, 8);
    img.palette.truncate(npal.max(1));
    if npal < 256 {
        let m = npal.max(1) as u8;
        for r in img.rows.iter_mut() {
            for b in r.iter_mut() {
                *b %= m;
            }
        }
    }
    img
}

fn shape_img(seed: u64, ct: c_int, bd: c_int) -> Img {
    let mut rng = Rng::new(seed);
    Img::random(&mut rng, 6, 3, ct, bd)
}

const INFO_BITS: [(&str, u32); 20] = [
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

/* ------------------------------------------------------------------ */
/* the big "record everything a getter returns" dump                   */
/* ------------------------------------------------------------------ */

/// `iccp_limit` bounds how many profile bytes may safely be read; the length
/// `png_get_iCCP` reports is taken from the profile itself, which the tests
/// deliberately corrupt in places.
unsafe fn dump_chunks(
    api: &Api,
    png: *mut PngStruct,
    info: *mut PngInfo,
    tag: &str,
    iccp_limit: usize,
) {
    log(format!(
        "{}: valid=0x{:x} rowbytes={} channels={} palette_max={}",
        tag,
        (api.png_get_valid)(png, info, 0xffff_ffff),
        (api.png_get_rowbytes)(png, info),
        (api.png_get_channels)(png, info),
        (api.png_get_palette_max)(png, info),
    ));
    {
        let mut s = String::new();
        for (n, b) in INFO_BITS {
            s += &format!("{}={} ", n, (api.png_get_valid)(png, info, b));
        }
        log(format!("{}: bits {}", tag, s));
    }

    /* gAMA */
    {
        let mut d = -1.0f64;
        let mut f = -1i32;
        let r1 = (api.png_get_gAMA)(png, info, &mut d);
        let r2 = (api.png_get_gAMA_fixed)(png, info, &mut f);
        log(format!("{}: gAMA r={} {:?} rf={} {}", tag, r1, d, r2, f));
    }

    /* cHRM */
    {
        let mut c = [-1.0f64; 8];
        let p = c.as_mut_ptr();
        let r1 = (api.png_get_cHRM)(
            png,
            info,
            p,
            p.add(1),
            p.add(2),
            p.add(3),
            p.add(4),
            p.add(5),
            p.add(6),
            p.add(7),
        );
        let mut f = [-1i32; 8];
        let q = f.as_mut_ptr();
        let r2 = (api.png_get_cHRM_fixed)(
            png,
            info,
            q,
            q.add(1),
            q.add(2),
            q.add(3),
            q.add(4),
            q.add(5),
            q.add(6),
            q.add(7),
        );
        log(format!(
            "{}: cHRM r={} {:?} rf={} {:?}",
            tag, r1, c, r2, f
        ));
        let mut x = [-1.0f64; 9];
        let p = x.as_mut_ptr();
        let r3 = (api.png_get_cHRM_XYZ)(
            png,
            info,
            p,
            p.add(1),
            p.add(2),
            p.add(3),
            p.add(4),
            p.add(5),
            p.add(6),
            p.add(7),
            p.add(8),
        );
        let mut z = [-1i32; 9];
        let q = z.as_mut_ptr();
        let r4 = (api.png_get_cHRM_XYZ_fixed)(
            png,
            info,
            q,
            q.add(1),
            q.add(2),
            q.add(3),
            q.add(4),
            q.add(5),
            q.add(6),
            q.add(7),
            q.add(8),
        );
        log(format!(
            "{}: cHRM_XYZ r={} {:?} rf={} {:?}",
            tag, r3, x, r4, z
        ));
    }

    /* sRGB */
    {
        let mut i = -999;
        let r = (api.png_get_sRGB)(png, info, &mut i);
        log(format!("{}: sRGB r={} intent={}", tag, r, i));
    }

    /* iCCP */
    {
        let mut name: *mut c_char = null_mut();
        let mut ct = -999;
        let mut prof: *mut u8 = null_mut();
        let mut plen = 0u32;
        let r = (api.png_get_iCCP)(png, info, &mut name, &mut ct, &mut prof, &mut plen);
        if r != 0 && !prof.is_null() {
            let n = (plen as usize).min(iccp_limit);
            log(format!(
                "{}: iCCP r={} name={} ct={} len={} data={:02x?}",
                tag,
                r,
                bs(name),
                ct,
                plen,
                core::slice::from_raw_parts(prof, n)
            ));
        } else {
            log(format!(
                "{}: iCCP r={} name={} ct={} len={} null={}",
                tag,
                r,
                bs(name),
                ct,
                plen,
                prof.is_null()
            ));
        }
    }

    /* sBIT */
    {
        let mut sb: *mut png_color_8 = null_mut();
        let r = (api.png_get_sBIT)(png, info, &mut sb);
        if !sb.is_null() {
            log(format!("{}: sBIT r={} {:?}", tag, r, *sb));
        } else {
            log(format!("{}: sBIT r={} <null>", tag, r));
        }
    }

    /* PLTE (needed for the hIST length) */
    let mut npal = 0i32;
    {
        let mut pal: *mut png_color = null_mut();
        let r = (api.png_get_PLTE)(png, info, &mut pal, &mut npal);
        if r != 0 && !pal.is_null() && npal > 0 && npal <= 256 {
            log(format!(
                "{}: PLTE r={} n={} {:?}",
                tag,
                r,
                npal,
                core::slice::from_raw_parts(pal, npal as usize)
            ));
        } else {
            log(format!("{}: PLTE r={} n={}", tag, r, npal));
        }
    }

    /* bKGD */
    {
        let mut bg: *mut png_color_16 = null_mut();
        let r = (api.png_get_bKGD)(png, info, &mut bg);
        if !bg.is_null() {
            log(format!("{}: bKGD r={} {:?}", tag, r, *bg));
        } else {
            log(format!("{}: bKGD r={} <null>", tag, r));
        }
    }

    /* hIST */
    {
        let mut h: *mut u16 = null_mut();
        let r = (api.png_get_hIST)(png, info, &mut h);
        if r != 0 && !h.is_null() && npal > 0 && npal <= 256 {
            log(format!(
                "{}: hIST r={} {:?}",
                tag,
                r,
                core::slice::from_raw_parts(h, npal as usize)
            ));
        } else {
            log(format!("{}: hIST r={} null={}", tag, r, h.is_null()));
        }
    }

    /* tRNS */
    {
        let mut ta: *mut u8 = null_mut();
        let mut nt = -999;
        let mut tc: *mut png_color_16 = null_mut();
        let r = (api.png_get_tRNS)(png, info, &mut ta, &mut nt, &mut tc);
        let alpha = if !ta.is_null() && nt > 0 && nt <= 256 {
            format!("{:02x?}", core::slice::from_raw_parts(ta, nt as usize))
        } else {
            format!("null={}", ta.is_null())
        };
        let col = if tc.is_null() {
            "<null>".to_string()
        } else {
            format!("{:?}", *tc)
        };
        log(format!(
            "{}: tRNS r={} num={} alpha={} color={}",
            tag, r, nt, alpha, col
        ));
    }

    /* pHYs and all its derived getters */
    {
        let mut x = 0xffff_ffffu32;
        let mut y = 0xffff_ffffu32;
        let mut u = -999;
        let r = (api.png_get_pHYs)(png, info, &mut x, &mut y, &mut u);
        let mut xd = 0xffff_ffffu32;
        let mut yd = 0xffff_ffffu32;
        let mut ud = -999;
        let rd = (api.png_get_pHYs_dpi)(png, info, &mut xd, &mut yd, &mut ud);
        log(format!(
            "{}: pHYs r={} {} {} unit={} | dpi r={} {} {} unit={}",
            tag, r, x, y, u, rd, xd, yd, ud
        ));
        log(format!(
            "{}: pHYs ppm x={} y={} both={} ppi x={} y={} both={} ar={:?} arf={}",
            tag,
            (api.png_get_x_pixels_per_meter)(png, info),
            (api.png_get_y_pixels_per_meter)(png, info),
            (api.png_get_pixels_per_meter)(png, info),
            (api.png_get_x_pixels_per_inch)(png, info),
            (api.png_get_y_pixels_per_inch)(png, info),
            (api.png_get_pixels_per_inch)(png, info),
            (api.png_get_pixel_aspect_ratio)(png, info),
            (api.png_get_pixel_aspect_ratio_fixed)(png, info),
        ));
    }

    /* oFFs and all its derived getters */
    {
        let mut x = -999i32;
        let mut y = -999i32;
        let mut u = -999;
        let r = (api.png_get_oFFs)(png, info, &mut x, &mut y, &mut u);
        log(format!("{}: oFFs r={} {} {} unit={}", tag, r, x, y, u));
        log(format!(
            "{}: oFFs px x={} y={} um x={} y={} in x={:?} y={:?} inf x={} y={}",
            tag,
            (api.png_get_x_offset_pixels)(png, info),
            (api.png_get_y_offset_pixels)(png, info),
            (api.png_get_x_offset_microns)(png, info),
            (api.png_get_y_offset_microns)(png, info),
            (api.png_get_x_offset_inches)(png, info),
            (api.png_get_y_offset_inches)(png, info),
            (api.png_get_x_offset_inches_fixed)(png, info),
            (api.png_get_y_offset_inches_fixed)(png, info),
        ));
    }

    /* tIME */
    {
        let mut t: *mut png_time = null_mut();
        let r = (api.png_get_tIME)(png, info, &mut t);
        if !t.is_null() {
            log(format!("{}: tIME r={} {:?}", tag, r, *t));
        } else {
            log(format!("{}: tIME r={} <null>", tag, r));
        }
    }

    /* pCAL */
    {
        let mut purpose: *mut c_char = null_mut();
        let mut x0 = -999i32;
        let mut x1 = -999i32;
        let mut ty = -999;
        let mut np = -999;
        let mut units: *mut c_char = null_mut();
        let mut params: *mut *mut c_char = null_mut();
        let r = (api.png_get_pCAL)(
            png,
            info,
            &mut purpose,
            &mut x0,
            &mut x1,
            &mut ty,
            &mut np,
            &mut units,
            &mut params,
        );
        let mut ps = String::new();
        if r != 0 && !params.is_null() && np >= 0 && np <= 255 {
            for k in 0..np as usize {
                ps += &format!("{} ", bs(*params.add(k)));
            }
        }
        log(format!(
            "{}: pCAL r={} purpose={} X0={} X1={} type={} n={} units={} params=[{}]",
            tag,
            r,
            bs(purpose),
            x0,
            x1,
            ty,
            np,
            bs(units),
            ps
        ));
    }

    /* sCAL */
    {
        let mut u = -999;
        let mut w = -1.0f64;
        let mut h = -1.0f64;
        let r1 = (api.png_get_sCAL)(png, info, &mut u, &mut w, &mut h);
        let mut u2 = -999;
        let mut sw: *mut c_char = null_mut();
        let mut sh: *mut c_char = null_mut();
        let r2 = (api.png_get_sCAL_s)(png, info, &mut u2, &mut sw, &mut sh);
        log(format!(
            "{}: sCAL r={} unit={} {:?} {:?} | s r={} unit={} {} {}",
            tag,
            r1,
            u,
            w,
            h,
            r2,
            u2,
            bs(sw),
            bs(sh)
        ));
        // png_get_sCAL_fixed png_error()s when the value does not fit; only
        // exercise it when the value is representable.
        if r1 != 0 && w.abs() < 21000.0 && h.abs() < 21000.0 {
            let mut u3 = -999;
            let mut wf = -1i32;
            let mut hf = -1i32;
            let r3 = (api.png_get_sCAL_fixed)(png, info, &mut u3, &mut wf, &mut hf);
            log(format!(
                "{}: sCAL_fixed r={} unit={} {} {}",
                tag, r3, u3, wf, hf
            ));
        } else {
            log(format!("{}: sCAL_fixed skipped", tag));
        }
    }

    /* sPLT */
    {
        let mut sp: *mut png_sPLT_t = null_mut();
        let n = (api.png_get_sPLT)(png, info, &mut sp);
        log(format!("{}: sPLT n={} null={}", tag, n, sp.is_null()));
        if !sp.is_null() && n > 0 {
            for k in 0..n as usize {
                let e = *sp.add(k);
                let ents = if e.entries.is_null() || e.nentries <= 0 {
                    "[]".to_string()
                } else {
                    format!(
                        "{:?}",
                        core::slice::from_raw_parts(e.entries, e.nentries as usize)
                    )
                };
                log(format!(
                    "{}: sPLT[{}] name={} depth={} n={} {}",
                    tag,
                    k,
                    bs(e.name),
                    e.depth,
                    e.nentries,
                    ents
                ));
            }
        }
    }

    /* text */
    {
        let mut tp: *mut png_text = null_mut();
        let mut n = -999;
        let r = (api.png_get_text)(png, info, &mut tp, &mut n);
        log(format!("{}: text r={} n={} null={}", tag, r, n, tp.is_null()));
        if !tp.is_null() && r > 0 {
            for k in 0..r as usize {
                let t = *tp.add(k);
                // key, text, lang and lang_key all live in one allocation that
                // png_free_data(PNG_FREE_TEXT, num) releases while only
                // clearing `key`, so the other three dangle once key is NULL.
                if t.key.is_null() {
                    log(format!(
                        "{}: text[{}] comp={} key=<null> tlen={} ilen={} (freed)",
                        tag, k, t.compression, t.text_length, t.itxt_length
                    ));
                } else {
                    log(format!(
                        "{}: text[{}] comp={} key={} text={} tlen={} ilen={} lang={} lkey={}",
                        tag,
                        k,
                        t.compression,
                        bs(t.key),
                        bs(t.text),
                        t.text_length,
                        t.itxt_length,
                        bs(t.lang),
                        bs(t.lang_key)
                    ));
                }
            }
        }
    }

    /* eXIf */
    {
        let mut n = 0xffff_ffffu32;
        let mut e: *mut u8 = null_mut();
        let r = (api.png_get_eXIf_1)(png, info, &mut n, &mut e);
        if r != 0 && !e.is_null() && n <= 65536 {
            log(format!(
                "{}: eXIf r={} n={} {:02x?}",
                tag,
                r,
                n,
                core::slice::from_raw_parts(e, n as usize)
            ));
        } else {
            log(format!("{}: eXIf r={} n={} null={}", tag, r, n, e.is_null()));
        }
    }

    /* cICP */
    {
        let mut b = [0xffu8; 4];
        let p = b.as_mut_ptr();
        let r = (api.png_get_cICP)(png, info, p, p.add(1), p.add(2), p.add(3));
        log(format!("{}: cICP r={} {:?}", tag, r, b));
    }

    /* cLLI */
    {
        let mut a = -1.0f64;
        let mut c = -1.0f64;
        let r1 = (api.png_get_cLLI)(png, info, &mut a, &mut c);
        let mut af = 0xffff_ffffu32;
        let mut cf = 0xffff_ffffu32;
        let r2 = (api.png_get_cLLI_fixed)(png, info, &mut af, &mut cf);
        log(format!(
            "{}: cLLI r={} {:?} {:?} rf={} {} {}",
            tag, r1, a, c, r2, af, cf
        ));
    }

    /* mDCV */
    {
        let mut d = [-1.0f64; 10];
        let p = d.as_mut_ptr();
        let r1 = (api.png_get_mDCV)(
            png,
            info,
            p,
            p.add(1),
            p.add(2),
            p.add(3),
            p.add(4),
            p.add(5),
            p.add(6),
            p.add(7),
            p.add(8),
            p.add(9),
        );
        let mut f = [-1i32; 8];
        let q = f.as_mut_ptr();
        let mut dl = [0xffff_ffffu32; 2];
        let s = dl.as_mut_ptr();
        let r2 = (api.png_get_mDCV_fixed)(
            png,
            info,
            q,
            q.add(1),
            q.add(2),
            q.add(3),
            q.add(4),
            q.add(5),
            q.add(6),
            q.add(7),
            s,
            s.add(1),
        );
        log(format!(
            "{}: mDCV r={} {:?} rf={} {:?} {:?}",
            tag, r1, d, r2, f, dl
        ));
    }

    /* unknown chunks */
    {
        let mut up: *mut png_unknown_chunk = null_mut();
        let n = (api.png_get_unknown_chunks)(png, info, &mut up);
        log(format!("{}: unknown n={} null={}", tag, n, up.is_null()));
        if !up.is_null() && n > 0 {
            for k in 0..n as usize {
                let u = *up.add(k);
                let data = if u.data.is_null() || u.size == 0 {
                    format!("<empty size={}>", u.size)
                } else {
                    format!(
                        "{:02x?}",
                        core::slice::from_raw_parts(u.data, u.size.min(1024))
                    )
                };
                log(format!(
                    "{}: unknown[{}] name={:02x?} size={} loc={} data={}",
                    tag, k, u.name, u.size, u.location, data
                ));
            }
        }
    }
}

/// Call every chunk getter with NULL out-parameters.  Safe only when either
/// `png`/`info` is NULL or no chunk at all is valid (see the call sites).
unsafe fn null_getters(api: &Api, p: *mut PngStruct, i: *mut PngInfo, tag: &str) {
    log(format!(
        "{}: gAMA {} {} cHRM {} {} XYZ {} {}",
        tag,
        (api.png_get_gAMA)(p, i, nz()),
        (api.png_get_gAMA_fixed)(p, i, nz()),
        (api.png_get_cHRM)(p, i, nz(), nz(), nz(), nz(), nz(), nz(), nz(), nz()),
        (api.png_get_cHRM_fixed)(p, i, nz(), nz(), nz(), nz(), nz(), nz(), nz(), nz()),
        (api.png_get_cHRM_XYZ)(p, i, nz(), nz(), nz(), nz(), nz(), nz(), nz(), nz(), nz()),
        (api.png_get_cHRM_XYZ_fixed)(p, i, nz(), nz(), nz(), nz(), nz(), nz(), nz(), nz(), nz()),
    ));
    log(format!(
        "{}: sRGB {} iCCP {} sBIT {} bKGD {} PLTE {} hIST {} tRNS {}",
        tag,
        (api.png_get_sRGB)(p, i, nz()),
        (api.png_get_iCCP)(p, i, nz(), nz(), nz(), nz()),
        (api.png_get_sBIT)(p, i, nz()),
        (api.png_get_bKGD)(p, i, nz()),
        (api.png_get_PLTE)(p, i, nz(), nz()),
        (api.png_get_hIST)(p, i, nz()),
        (api.png_get_tRNS)(p, i, nz(), nz(), nz()),
    ));
    log(format!(
        "{}: pHYs {} {} oFFs {} tIME {} pCAL {}",
        tag,
        (api.png_get_pHYs)(p, i, nz(), nz(), nz()),
        (api.png_get_pHYs_dpi)(p, i, nz(), nz(), nz()),
        (api.png_get_oFFs)(p, i, nz(), nz(), nz()),
        (api.png_get_tIME)(p, i, nz()),
        (api.png_get_pCAL)(p, i, nz(), nz(), nz(), nz(), nz(), nz(), nz()),
    ));
    log(format!(
        "{}: sPLT {} text {} eXIf {} unknown {} valid {}",
        tag,
        (api.png_get_sPLT)(p, i, nz()),
        (api.png_get_text)(p, i, nz(), nz()),
        (api.png_get_eXIf_1)(p, i, nz(), nz()),
        (api.png_get_unknown_chunks)(p, i, nz()),
        (api.png_get_valid)(p, i, 0xffff_ffff),
    ));
    log(format!(
        "{}: cICP {} cLLI {} {} mDCV {} {}",
        tag,
        (api.png_get_cICP)(p, i, nz(), nz(), nz(), nz()),
        (api.png_get_cLLI)(p, i, nz(), nz()),
        (api.png_get_cLLI_fixed)(p, i, nz(), nz()),
        (api.png_get_mDCV)(
            p,
            i,
            nz(),
            nz(),
            nz(),
            nz(),
            nz(),
            nz(),
            nz(),
            nz(),
            nz(),
            nz()
        ),
        (api.png_get_mDCV_fixed)(
            p,
            i,
            nz(),
            nz(),
            nz(),
            nz(),
            nz(),
            nz(),
            nz(),
            nz(),
            nz(),
            nz()
        ),
    ));
    log(format!(
        "{}: sCAL {} {} rows_null={}",
        tag,
        (api.png_get_sCAL)(p, i, nz(), nz(), nz()),
        (api.png_get_sCAL_s)(p, i, nz(), nz(), nz()),
        (api.png_get_rows)(p, i).is_null(),
    ));
    log(format!(
        "{}: ppm {} {} {} ppi {} {} {} ar {:?} {}",
        tag,
        (api.png_get_x_pixels_per_meter)(p, i),
        (api.png_get_y_pixels_per_meter)(p, i),
        (api.png_get_pixels_per_meter)(p, i),
        (api.png_get_x_pixels_per_inch)(p, i),
        (api.png_get_y_pixels_per_inch)(p, i),
        (api.png_get_pixels_per_inch)(p, i),
        (api.png_get_pixel_aspect_ratio)(p, i),
        (api.png_get_pixel_aspect_ratio_fixed)(p, i),
    ));
    log(format!(
        "{}: off {} {} {} {} {:?} {:?} {} {}",
        tag,
        (api.png_get_x_offset_pixels)(p, i),
        (api.png_get_y_offset_pixels)(p, i),
        (api.png_get_x_offset_microns)(p, i),
        (api.png_get_y_offset_microns)(p, i),
        (api.png_get_x_offset_inches)(p, i),
        (api.png_get_y_offset_inches)(p, i),
        (api.png_get_x_offset_inches_fixed)(p, i),
        (api.png_get_y_offset_inches_fixed)(p, i),
    ));
}

/// Call every chunk setter with NULL data: all of them must ignore the call.
unsafe fn null_setters(api: &Api, p: *mut PngStruct, i: *mut PngInfo, tag: &str) {
    (api.png_set_bKGD)(p, i, core::ptr::null());
    (api.png_set_sBIT)(p, i, core::ptr::null());
    (api.png_set_hIST)(p, i, core::ptr::null());
    (api.png_set_tIME)(p, i, core::ptr::null());
    (api.png_set_eXIf_1)(p, i, 4, null_mut());
    (api.png_set_iCCP)(p, i, core::ptr::null(), 0, core::ptr::null(), 4);
    (api.png_set_text)(p, i, core::ptr::null(), 1);
    (api.png_set_sPLT)(p, i, core::ptr::null(), 1);
    (api.png_set_unknown_chunks)(p, i, core::ptr::null(), 1);
    (api.png_set_tRNS)(p, i, core::ptr::null(), 0, core::ptr::null());
    (api.png_set_invalid)(p, i, 0);
    log(format!(
        "{}: after null setters valid={}",
        tag,
        (api.png_get_valid)(p, i, 0xffff_ffff)
    ));
}

/* ------------------------------------------------------------------ */
/* drivers                                                             */
/* ------------------------------------------------------------------ */

/// Read `data` with both libraries, dumping the whole info state after
/// `png_read_info` and again after `png_read_end`.
unsafe fn read_and_dump(api: &Api, data: &[u8], o: &mut Outcome) {
    tls().input = data.to_vec();
    tls().in_pos = 0;
    let (png, info) = new_read(api);
    (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
    let g = guarded(api, png, &mut || {
        (api.png_read_info)(png, info);
        dump_chunks(api, png, info, "read_info", !0);
        let h = (api.png_get_image_height)(png, info) as usize;
        let rb = (api.png_get_rowbytes)(png, info);
        let mut row = vec![0u8; rb];
        for _ in 0..h {
            (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
        }
        (api.png_read_end)(png, info);
        dump_chunks(api, png, info, "read_end", !0);
    });
    o.push(format!("read guard={:?}", g));
    destroy_read(api, png, info);
}

/// The whole round trip for one chunk configuration:
///   * write it with both libraries and compare the file bytes,
///   * read the file back with both and compare every getter,
///   * duplicate the raw chunk in the datastream and read again.
fn rt(name: &str, img: &Img, chunkid: &str, set: &dyn Fn(&Api, *mut PngStruct, *mut PngInfo)) {
    let mut file: Vec<u8> = Vec::new();
    assert_same(&format!("{} / write", name), |api| unsafe {
        let mut o = Outcome::default();
        let wr = write_image(api, img, &WriteOpts::default(), &mut |a, p, i| {
            set(a, p, i);
            dump_chunks(a, p, i, "write", !0);
        });
        o.push(format!("write guard={:?}", wr.guard));
        o.output = wr.bytes.clone();
        if api.which == "C" {
            file = wr.bytes.clone();
        }
        o
    });
    if file.len() < 20 {
        return;
    }
    let names: Vec<String> = split_chunks(&file).into_iter().map(|(n, _)| n).collect();
    assert_same(&format!("{} / read", name), |api| unsafe {
        let mut o = Outcome::default();
        o.push(format!("chunks={:?}", names));
        read_and_dump(api, &file, &mut o);
        o
    });
    if chunkid.is_empty() {
        return;
    }
    let Some(raw) = raw_chunk_of(&file, chunkid) else {
        return;
    };
    let dup = insert_after_last(&file, chunkid, &raw);
    assert_same(&format!("{} / twice", name), |api| unsafe {
        let mut o = Outcome::default();
        read_and_dump(api, &dup, &mut o);
        o
    });
}

/* ------------------------------------------------------------------ */
/* C-107 gAMA, C-108 cHRM, C-109 sRGB, C-111 sBIT, C-112 bKGD          */
/* ------------------------------------------------------------------ */

/// C-107 … C-109, C-111, C-112.
#[test]
fn round_trip() {
    /* the chunk-absent baseline, for every colour type */
    for (ct, bd) in VALID_SHAPES {
        let img = shape_img(0xab5e17 ^ ((ct as u64) << 8) ^ bd as u64, ct, bd);
        rt(&format!("absent ct={} bd={}", ct, bd), &img, "", &|_, _, _| {});
    }

    /* ---------------- C-107 gAMA ---------------- */
    let img = rgb_img(0x9a3a);
    for &v in &[
        0i32, 1, 45455, 100000, 500000, PNG_FP_MAX, -1, -100000, PNG_FP_MIN,
    ] {
        rt(
            &format!("gAMA_fixed {}", v),
            &img,
            "gAMA",
            &|a, p, i| unsafe {
                (a.png_set_gAMA_fixed)(p, i, v);
            },
        );
    }
    let mut rng = Rng::new(0x9a3b);
    for it in 0..12 {
        let d = match it {
            0 => 0.0f64,
            1 => 0.45455,
            2 => 1.0,
            3 => 2.2,
            4 => 21474.0,
            5 => 21475.0, // rejected by png_fixed
            6 => -1.0,
            _ => rng.range(-2_000_000, 2_000_000) as f64 / 1000.0,
        };
        rt(
            &format!("gAMA {:?}", d),
            &img,
            "gAMA",
            &|a, p, i| unsafe {
                (a.png_set_gAMA)(p, i, d);
            },
        );
    }

    /* ---------------- C-108 cHRM ---------------- */
    const SRGB_XY: [i32; 8] = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000];
    let mut rng = Rng::new(0xc4c8);
    for it in 0..14 {
        let v: [i32; 8] = match it {
            0 => SRGB_XY,
            1 => [0; 8],
            2 => [50000; 8],
            3 => [-1, -2, -3, -4, -5, -6, -7, -8],
            4 => [PNG_FP_MAX; 8],
            5 => [PNG_FP_MIN; 8],
            6 => [100000, 0, 100000, 0, 0, 100000, 0, 0],
            _ => {
                let mut a = [0i32; 8];
                for x in a.iter_mut() {
                    *x = rng.range(-200000, 200000) as i32;
                }
                a
            }
        };
        rt(
            &format!("cHRM_fixed {:?}", v),
            &img,
            "cHRM",
            &|a, p, i| unsafe {
                (a.png_set_cHRM_fixed)(p, i, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
            },
        );
        let d: Vec<f64> = v.iter().map(|&x| x as f64 / 100000.0).collect();
        rt(
            &format!("cHRM {:?}", d),
            &img,
            "cHRM",
            &|a, p, i| unsafe {
                (a.png_set_cHRM)(p, i, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]);
            },
        );
    }
    let mut rng = Rng::new(0xc4c9);
    for it in 0..10 {
        // the sRGB primaries expressed as XYZ, plus degenerate cases
        let v: [i32; 9] = match it {
            0 => [
                41239, 21264, 1933, 35758, 71517, 11919, 18048, 7218, 95053,
            ],
            1 => [0; 9],
            2 => [100000; 9],
            3 => [-41239, -21264, -1933, 35758, 71517, 11919, 18048, 7218, 95053],
            4 => [PNG_FP_MAX; 9],
            _ => {
                let mut a = [0i32; 9];
                for x in a.iter_mut() {
                    *x = rng.range(-150000, 150000) as i32;
                }
                a
            }
        };
        rt(
            &format!("cHRM_XYZ_fixed {:?}", v),
            &img,
            "cHRM",
            &|a, p, i| unsafe {
                (a.png_set_cHRM_XYZ_fixed)(
                    p, i, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
                );
            },
        );
        let d: Vec<f64> = v.iter().map(|&x| x as f64 / 100000.0).collect();
        rt(
            &format!("cHRM_XYZ {:?}", d),
            &img,
            "cHRM",
            &|a, p, i| unsafe {
                (a.png_set_cHRM_XYZ)(
                    p, i, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7], d[8],
                );
            },
        );
    }

    /* ---------------- C-109 sRGB ---------------- */
    for intent in -1..=5 {
        rt(
            &format!("sRGB intent={}", intent),
            &img,
            "sRGB",
            &|a, p, i| unsafe {
                (a.png_set_sRGB)(p, i, intent);
            },
        );
        rt(
            &format!("sRGB_gAMA_and_cHRM intent={}", intent),
            &img,
            "sRGB",
            &|a, p, i| unsafe {
                (a.png_set_sRGB_gAMA_and_cHRM)(p, i, intent);
            },
        );
    }

    /* ---------------- C-111 sBIT ---------------- */
    for (ct, bd) in VALID_SHAPES {
        let img = shape_img(0x5b17 ^ ((ct as u64) << 8) ^ bd as u64, ct, bd);
        let maxbits = if ct == PNG_COLOR_TYPE_PALETTE { 8 } else { bd };
        let mut rng = Rng::new(0x5b18 ^ ((ct as u64) << 8) ^ bd as u64);
        for it in 0..8 {
            let sb = match it {
                0 => png_color_8 {
                    red: 1,
                    green: 1,
                    blue: 1,
                    gray: 1,
                    alpha: 1,
                },
                1 => png_color_8 {
                    red: maxbits as u8,
                    green: maxbits as u8,
                    blue: maxbits as u8,
                    gray: bd as u8,
                    alpha: bd as u8,
                },
                2 => png_color_8 {
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: 0,
                    alpha: 0,
                },
                3 => png_color_8 {
                    red: 255,
                    green: 255,
                    blue: 255,
                    gray: 255,
                    alpha: 255,
                },
                _ => png_color_8 {
                    red: 1 + rng.u8() % maxbits as u8,
                    green: 1 + rng.u8() % maxbits as u8,
                    blue: 1 + rng.u8() % maxbits as u8,
                    gray: 1 + rng.u8() % bd as u8,
                    alpha: 1 + rng.u8() % bd as u8,
                },
            };
            rt(
                &format!("sBIT ct={} bd={} {:?}", ct, bd, sb),
                &img,
                "sBIT",
                &|a, p, i| unsafe {
                    let v = sb;
                    (a.png_set_sBIT)(p, i, &v);
                },
            );
        }
    }

    /* ---------------- C-112 bKGD ---------------- */
    for (ct, bd) in VALID_SHAPES {
        let npal = if ct == PNG_COLOR_TYPE_PALETTE { 5 } else { 0 };
        let img = if ct == PNG_COLOR_TYPE_PALETTE && bd == 8 {
            pal_img(0xb46d ^ bd as u64, npal)
        } else {
            shape_img(0xb46d ^ ((ct as u64) << 8) ^ bd as u64, ct, bd)
        };
        let mut rng = Rng::new(0xb46e ^ ((ct as u64) << 8) ^ bd as u64);
        for it in 0..8 {
            let bg = match it {
                0 => png_color_16 {
                    index: 0,
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: 0,
                },
                1 => png_color_16 {
                    index: 255,
                    red: 0xffff,
                    green: 0xffff,
                    blue: 0xffff,
                    gray: 0xffff,
                },
                2 => png_color_16 {
                    index: 3,
                    red: 1,
                    green: 2,
                    blue: 3,
                    gray: 4,
                },
                _ => png_color_16 {
                    index: rng.u8(),
                    red: rng.u32() as u16,
                    green: rng.u32() as u16,
                    blue: rng.u32() as u16,
                    gray: rng.u32() as u16,
                },
            };
            rt(
                &format!("bKGD ct={} bd={} {:?}", ct, bd, bg),
                &img,
                "bKGD",
                &|a, p, i| unsafe {
                    let v = bg;
                    (a.png_set_bKGD)(p, i, &v);
                },
            );
        }
    }

    /* The getters' NULL-argument branches.
     *
     * Only some getters tolerate a NULL out-parameter when the chunk is
     * actually present -- png_get_PLTE, _sBIT, _bKGD, _tIME, _oFFs, _eXIf_1,
     * _sCAL* and _pHYs_dpi all dereference unconditionally -- so NULL
     * out-parameters are only used on an info struct where nothing is valid,
     * and the fully populated struct is probed through a NULL png/info.
     */
    assert_same("null out-parameters, nothing valid", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                4,
                2,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            null_getters(api, png, info, "empty");
            null_setters(api, png, info, "empty");
            dump_chunks(api, png, info, "after null setters", !0);
        });
        o.push(format!("guard={:?}", g));
        destroy_write(api, png, info);
        o
    });

    let pimg = pal_img(0x9a11, 8);
    assert_same("null png / null info", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                pimg.w,
                pimg.h,
                pimg.bit_depth,
                pimg.color_type,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_set_PLTE)(
                png,
                info,
                pimg.palette.as_ptr(),
                pimg.palette.len() as c_int,
            );
            set_all_chunks(api, png, info);
            null_getters(api, null_mut(), info, "null png");
            null_getters(api, png, null_mut(), "null info");
            null_getters(api, null_mut(), null_mut(), "both null");
            null_setters(api, null_mut(), info, "null png");
            null_setters(api, png, null_mut(), "null info");
            null_setters(api, png, info, "populated");
            dump_chunks(api, png, info, "after null setters", !0);
        });
        o.push(format!("guard={:?}", g));
        destroy_write(api, png, info);
        o
    });
}

/* ------------------------------------------------------------------ */
/* C-113 hIST, C-114 tRNS, C-115 pHYs, C-116 oFFs, C-117 tIME          */
/* ------------------------------------------------------------------ */

/// C-113 … C-117.
#[test]
fn round_trip_2() {
    /* ---------------- C-113 hIST ---------------- */
    for &npal in &[1usize, 2, 3, 17, 128, 255, 256] {
        let img = pal_img(0x4157 ^ npal as u64, npal);
        let mut rng = Rng::new(0x4158 ^ npal as u64);
        for it in 0..3 {
            let hist: Vec<u16> = (0..256)
                .map(|k| match it {
                    0 => 0,
                    1 => (k as u16).wrapping_mul(257),
                    _ => rng.u32() as u16,
                })
                .collect();
            rt(
                &format!("hIST npal={} it={}", npal, it),
                &img,
                "hIST",
                &|a, p, i| unsafe {
                    (a.png_set_hIST)(p, i, hist.as_ptr());
                },
            );
        }
    }
    // hIST on a non-paletted image: rejected ("Invalid palette size")
    {
        let img = rgb_img(0x4159);
        let hist = vec![7u16; 256];
        rt("hIST on rgb", &img, "hIST", &|a, p, i| unsafe {
            (a.png_set_hIST)(p, i, hist.as_ptr());
        });
    }

    /* ---------------- C-114 tRNS ---------------- */
    for &npal in &[1usize, 4, 256] {
        let img = pal_img(0x74e5 ^ npal as u64, npal);
        for &nt in &[0i32, 1, 2, npal as i32, npal as i32 + 1, 256, 257] {
            let mut rng = Rng::new(0x74e6 ^ (nt as u64) << 8 ^ npal as u64);
            let alpha: Vec<u8> = (0..300).map(|_| rng.u8()).collect();
            rt(
                &format!("tRNS palette npal={} nt={}", npal, nt),
                &img,
                "tRNS",
                &|a, p, i| unsafe {
                    (a.png_set_tRNS)(p, i, alpha.as_ptr(), nt, core::ptr::null());
                },
            );
        }
    }
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    ] {
        let img = shape_img(0x74e7 ^ ((ct as u64) << 8) ^ bd as u64, ct, bd);
        let mut rng = Rng::new(0x74e8 ^ ((ct as u64) << 8) ^ bd as u64);
        for it in 0..6 {
            let tc = match it {
                0 => png_color_16 {
                    index: 0,
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: 0,
                },
                1 => png_color_16 {
                    index: 0,
                    red: 1,
                    green: 1,
                    blue: 1,
                    gray: 1,
                },
                2 => png_color_16 {
                    index: 0,
                    red: 0xffff,
                    green: 0xffff,
                    blue: 0xffff,
                    gray: 0xffff,
                },
                3 => png_color_16 {
                    index: 0,
                    red: 0x00ff,
                    green: 0x00ff,
                    blue: 0x00ff,
                    gray: 0x00ff,
                },
                _ => png_color_16 {
                    index: rng.u8(),
                    red: rng.u32() as u16,
                    green: rng.u32() as u16,
                    blue: rng.u32() as u16,
                    gray: rng.u32() as u16,
                },
            };
            for &nt in &[0i32, 1] {
                rt(
                    &format!("tRNS ct={} bd={} nt={} {:?}", ct, bd, nt, tc),
                    &img,
                    "tRNS",
                    &|a, p, i| unsafe {
                        let v = tc;
                        (a.png_set_tRNS)(p, i, core::ptr::null(), nt, &v);
                    },
                );
            }
        }
    }

    /* ---------------- C-115 pHYs ---------------- */
    let img = rgb_img(0x7845);
    let mut rng = Rng::new(0x7846);
    for unit in 0..3 {
        for it in 0..8 {
            let (x, y) = match it {
                0 => (0u32, 0u32),
                1 => (1, 1),
                2 => (0, 1000),
                3 => (1000, 0),
                4 => (2835, 2835),
                5 => (PNG_UINT_31_MAX, PNG_UINT_31_MAX),
                6 => (0xffff_ffff, 0xffff_ffff),
                _ => (rng.u32(), rng.u32()),
            };
            rt(
                &format!("pHYs unit={} {} {}", unit, x, y),
                &img,
                "pHYs",
                &|a, p, i| unsafe {
                    (a.png_set_pHYs)(p, i, x, y, unit);
                },
            );
        }
    }

    /* ---------------- C-116 oFFs ---------------- */
    let mut rng = Rng::new(0x0ff5);
    for unit in 0..3 {
        for it in 0..8 {
            let (x, y) = match it {
                0 => (0i32, 0i32),
                1 => (1, -1),
                2 => (-1, 1),
                3 => (i32::MAX, i32::MIN),
                4 => (i32::MIN, i32::MAX),
                5 => (1_000_000, -1_000_000),
                _ => (rng.u32() as i32, rng.u32() as i32),
            };
            rt(
                &format!("oFFs unit={} {} {}", unit, x, y),
                &img,
                "oFFs",
                &|a, p, i| unsafe {
                    (a.png_set_oFFs)(p, i, x, y, unit);
                },
            );
        }
    }

    /* ---------------- C-117 tIME ---------------- */
    let mut rng = Rng::new(0x71_3e);
    for it in 0..16 {
        let t = match it {
            0 => png_time {
                year: 2000,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            },
            1 => png_time {
                year: 1999,
                month: 12,
                day: 31,
                hour: 23,
                minute: 59,
                second: 60,
            },
            2 => png_time {
                year: 0,
                month: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
            },
            3 => png_time {
                year: 65535,
                month: 13,
                day: 32,
                hour: 24,
                minute: 60,
                second: 61,
            },
            4 => png_time {
                year: 2024,
                month: 2,
                day: 29,
                hour: 12,
                minute: 30,
                second: 45,
            },
            5 => png_time {
                year: 2024,
                month: 6,
                day: 15,
                hour: 23,
                minute: 59,
                second: 61,
            },
            _ => png_time {
                year: rng.u32() as u16,
                month: rng.u8(),
                day: rng.u8(),
                hour: rng.u8(),
                minute: rng.u8(),
                second: rng.u8(),
            },
        };
        rt(
            &format!("tIME {:?}", t),
            &img,
            "tIME",
            &|a, p, i| unsafe {
                let v = t;
                (a.png_set_tIME)(p, i, &v);
            },
        );
    }
}

/* ------------------------------------------------------------------ */
/* C-118 pCAL, C-119 sCAL, C-120 sPLT, C-110 iCCP                      */
/* ------------------------------------------------------------------ */

/// C-110, C-118 … C-120.
#[test]
fn round_trip_3() {
    let img = rgb_img(0x3c41);

    /* ---------------- C-118 pCAL ---------------- */
    let mut rng = Rng::new(0x3c42);
    for ty in -1..=4 {
        for nparams in 0..=8usize {
            let purpose = cs(match nparams % 4 {
                0 => "cal",
                1 => "a much longer calibration purpose keyword",
                2 => "x",
                _ => "Purpose With Spaces",
            });
            let units = cs(match nparams % 3 {
                0 => "",
                1 => "metres",
                _ => "arbitrary units",
            });
            let mut param_strings: Vec<std::ffi::CString> = Vec::new();
            for k in 0..nparams {
                let v = match k % 5 {
                    0 => "0".to_string(),
                    1 => "-1.5".to_string(),
                    2 => "+2".to_string(),
                    3 => "1e3".to_string(),
                    _ => format!("{}.{}", rng.range(-1000, 1000), rng.below(1000)),
                };
                param_strings.push(cs(&v));
            }
            let x0 = rng.u32() as i32;
            let x1 = rng.u32() as i32;
            rt(
                &format!("pCAL type={} nparams={}", ty, nparams),
                &img,
                "pCAL",
                &|a, p, i| unsafe {
                    let mut ptrs: Vec<*mut c_char> = param_strings
                        .iter()
                        .map(|c| c.as_ptr() as *mut c_char)
                        .collect();
                    let pp = if ptrs.is_empty() {
                        null_mut()
                    } else {
                        ptrs.as_mut_ptr()
                    };
                    (a.png_set_pCAL)(
                        p,
                        i,
                        purpose.as_ptr(),
                        x0,
                        x1,
                        ty,
                        nparams as c_int,
                        units.as_ptr(),
                        pp,
                    );
                },
            );
        }
    }
    // an invalid floating point parameter string
    {
        let purpose = cs("cal");
        let units = cs("u");
        let bad = cs("not-a-number");
        rt("pCAL bad param", &img, "pCAL", &|a, p, i| unsafe {
            let mut ptrs: Vec<*mut c_char> = vec![bad.as_ptr() as *mut c_char];
            (a.png_set_pCAL)(
                p,
                i,
                purpose.as_ptr(),
                0,
                1,
                PNG_EQUATION_LINEAR,
                1,
                units.as_ptr(),
                ptrs.as_mut_ptr(),
            );
        });
    }

    /* ---------------- C-119 sCAL ---------------- */
    let mut rng = Rng::new(0x5ca1);
    for unit in 0..4 {
        for it in 0..8 {
            let (w, h) = match it {
                0 => (1.0f64, 1.0f64),
                1 => (0.0, 1.0),
                2 => (1.0, 0.0),
                3 => (-1.0, 1.0),
                4 => (0.000001, 1000.0),
                5 => (12345.678, 0.5),
                6 => (20000.0, 20000.0),
                _ => (
                    rng.range(1, 100000) as f64 / 100.0,
                    rng.range(1, 100000) as f64 / 100.0,
                ),
            };
            rt(
                &format!("sCAL unit={} {:?} {:?}", unit, w, h),
                &img,
                "sCAL",
                &|a, p, i| unsafe {
                    (a.png_set_sCAL)(p, i, unit, w, h);
                },
            );
            let (wf, hf) = (
                (w * 100000.0).clamp(-2e9, 2e9) as i32,
                (h * 100000.0).clamp(-2e9, 2e9) as i32,
            );
            rt(
                &format!("sCAL_fixed unit={} {} {}", unit, wf, hf),
                &img,
                "sCAL",
                &|a, p, i| unsafe {
                    (a.png_set_sCAL_fixed)(p, i, unit, wf, hf);
                },
            );
        }
        for s in [
            ("1", "1"),
            ("0.5", "2.5"),
            ("1e-3", "5e-2"),
            ("+1.5", "1"),
            ("-1", "1"),
            ("", "1"),
            ("abc", "1"),
            ("1", "1..2"),
            ("00000.000001", "9999.9999"),
        ] {
            let w = cs(s.0);
            let h = cs(s.1);
            rt(
                &format!("sCAL_s unit={} {:?} {:?}", unit, s.0, s.1),
                &img,
                "sCAL",
                &|a, p, i| unsafe {
                    (a.png_set_sCAL_s)(p, i, unit, w.as_ptr(), h.as_ptr());
                },
            );
        }
    }

    /* ---------------- C-120 sPLT ---------------- */
    let mut rng = Rng::new(0x5717);
    for &depth in &[8u8, 16] {
        for &n in &[0i32, 1, 2, 256] {
            for &count in &[1i32, 3] {
                let names: Vec<std::ffi::CString> = (0..count)
                    .map(|k| cs(&format!("palette {}", k)))
                    .collect();
                let entries: Vec<Vec<png_sPLT_entry>> = (0..count)
                    .map(|_| {
                        (0..n.max(0))
                            .map(|_| png_sPLT_entry {
                                red: rng.u32() as u16,
                                green: rng.u32() as u16,
                                blue: rng.u32() as u16,
                                alpha: rng.u32() as u16,
                                frequency: rng.u32() as u16,
                            })
                            .collect()
                    })
                    .collect();
                rt(
                    &format!("sPLT depth={} n={} count={}", depth, n, count),
                    &img,
                    "sPLT",
                    &|a, p, i| unsafe {
                        let mut sp: Vec<png_sPLT_t> = Vec::new();
                        for k in 0..count as usize {
                            sp.push(png_sPLT_t {
                                name: names[k].as_ptr() as *mut c_char,
                                depth,
                                entries: entries[k].as_ptr() as *mut png_sPLT_entry,
                                nentries: n,
                            });
                        }
                        (a.png_set_sPLT)(p, i, sp.as_ptr(), count);
                    },
                );
            }
        }
    }
    // NULL name / NULL entries: an app error
    {
        let ents = vec![png_sPLT_entry::default(); 4];
        rt("sPLT null name", &img, "sPLT", &|a, p, i| unsafe {
            let sp = png_sPLT_t {
                name: null_mut(),
                depth: 8,
                entries: ents.as_ptr() as *mut png_sPLT_entry,
                nentries: 4,
            };
            (a.png_set_sPLT)(p, i, &sp, 1);
        });
        let nm = cs("x");
        rt("sPLT null entries", &img, "sPLT", &|a, p, i| unsafe {
            let sp = png_sPLT_t {
                name: nm.as_ptr() as *mut c_char,
                depth: 8,
                entries: null_mut(),
                nentries: 4,
            };
            (a.png_set_sPLT)(p, i, &sp, 1);
        });
        rt("sPLT nentries=0 count=0", &img, "sPLT", &|a, p, i| unsafe {
            (a.png_set_sPLT)(p, i, core::ptr::null(), 0);
        });
    }

    /* ---------------- C-110 iCCP ---------------- */
    for &total in &[144usize, 148, 260, 1024, 2048] {
        for &keylen in &[1usize, 20, 79, 80] {
            let name = cs(&"k".repeat(keylen));
            let prof = Icc::new(total).build();
            let plen = prof.len() as u32;
            rt(
                &format!("iCCP total={} keylen={}", total, keylen),
                &img,
                "iCCP",
                &|a, p, i| unsafe {
                    (a.png_set_iCCP)(p, i, name.as_ptr(), 0, prof.as_ptr(), plen);
                },
            );
        }
    }
    // a grey profile on a grey image
    {
        let gimg = shape_img(0x1cc9, PNG_COLOR_TYPE_GRAY, 8);
        let name = cs("gray");
        let mut icc = Icc::new(260);
        icc.space = *b"GRAY";
        let prof = icc.build();
        let plen = prof.len() as u32;
        rt("iCCP gray", &gimg, "iCCP", &|a, p, i| unsafe {
            (a.png_set_iCCP)(p, i, name.as_ptr(), 0, prof.as_ptr(), plen);
        });
    }
    // a non-zero compression type
    {
        let name = cs("k");
        let prof = Icc::new(200).build();
        let plen = prof.len() as u32;
        rt("iCCP comp=1", &img, "iCCP", &|a, p, i| unsafe {
            (a.png_set_iCCP)(p, i, name.as_ptr(), 1, prof.as_ptr(), plen);
        });
    }
}

/* ------------------------------------------------------------------ */
/* C-121 tEXt, C-122 zTXt, C-123 iTXt, C-124 eXIf, C-125 cICP,         */
/* C-126 cLLI, C-127 mDCV                                              */
/* ------------------------------------------------------------------ */

/// C-121 … C-127.
#[test]
fn round_trip_4() {
    let img = rgb_img(0x7e47);

    /* ---------------- C-121 tEXt ---------------- */
    let mut rng = Rng::new(0x7e48);
    for &keylen in &[1usize, 2, 20, 79, 80, 100] {
        for &textlen in &[0usize, 1, 7, 100, 4096] {
            let key = cs(&"K".repeat(keylen));
            let text: String = (0..textlen)
                .map(|k| (b'a' + (k % 26) as u8) as char)
                .collect();
            let text = cs(&text);
            rt(
                &format!("tEXt key={} text={}", keylen, textlen),
                &img,
                "tEXt",
                &|a, p, i| unsafe {
                    let t = png_text {
                        compression: PNG_TEXT_COMPRESSION_NONE,
                        key: key.as_ptr() as *mut c_char,
                        text: text.as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: null_mut(),
                        lang_key: null_mut(),
                    };
                    (a.png_set_text)(p, i, &t, 1);
                },
            );
        }
    }
    // several tEXt entries at once, plus awkward keywords
    for keys in [
        vec!["a", "b", "c"],
        vec![" leading", "trailing ", "double  space"],
        vec!["tab\there", "high\u{00e9}", "\u{00a1}bang"],
    ] {
        let ks: Vec<std::ffi::CString> = keys.iter().map(|k| cs(k)).collect();
        let vs: Vec<std::ffi::CString> = (0..keys.len())
            .map(|k| cs(&format!("value {}", k)))
            .collect();
        rt(
            &format!("tEXt many {:?}", keys),
            &img,
            "tEXt",
            &|a, p, i| unsafe {
                let ts: Vec<png_text> = (0..ks.len())
                    .map(|k| png_text {
                        compression: PNG_TEXT_COMPRESSION_NONE,
                        key: ks[k].as_ptr() as *mut c_char,
                        text: vs[k].as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: null_mut(),
                        lang_key: null_mut(),
                    })
                    .collect();
                (a.png_set_text)(p, i, ts.as_ptr(), ts.len() as c_int);
            },
        );
    }

    /* ---------------- C-122 zTXt ---------------- */
    for &textlen in &[0usize, 1, 10, 1000, 65536] {
        for compressible in [true, false] {
            let text: String = if compressible {
                "abcabcabc".chars().cycle().take(textlen).collect()
            } else {
                (0..textlen)
                    .map(|_| (0x21 + (rng.u8() % 0x5e)) as char)
                    .collect()
            };
            let key = cs("zkey");
            let text = cs(&text);
            rt(
                &format!("zTXt len={} compressible={}", textlen, compressible),
                &img,
                "zTXt",
                &|a, p, i| unsafe {
                    let t = png_text {
                        compression: PNG_TEXT_COMPRESSION_zTXt,
                        key: key.as_ptr() as *mut c_char,
                        text: text.as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: null_mut(),
                        lang_key: null_mut(),
                    };
                    (a.png_set_text)(p, i, &t, 1);
                },
            );
        }
    }

    /* ---------------- C-123 iTXt ---------------- */
    for &comp in &[PNG_ITXT_COMPRESSION_NONE, PNG_ITXT_COMPRESSION_zTXt] {
        for (lang, lkey) in [
            ("", ""),
            ("en", "Key"),
            ("en-GB", "\u{00dc}bersetzung"),
            ("", "only-translated"),
            ("zh-Hans", ""),
        ] {
            for &textlen in &[0usize, 5, 500] {
                let key = cs("ikey");
                let l = cs(lang);
                let lk = cs(lkey);
                let text: String = "\u{4f60}\u{597d}world "
                    .chars()
                    .cycle()
                    .take(textlen)
                    .collect();
                let text = cs(&text);
                rt(
                    &format!("iTXt comp={} lang={:?} lkey={:?} len={}", comp, lang, lkey, textlen),
                    &img,
                    "iTXt",
                    &|a, p, i| unsafe {
                        let t = png_text {
                            compression: comp,
                            key: key.as_ptr() as *mut c_char,
                            text: text.as_ptr() as *mut c_char,
                            text_length: 0,
                            itxt_length: 0,
                            lang: l.as_ptr() as *mut c_char,
                            lang_key: lk.as_ptr() as *mut c_char,
                        };
                        (a.png_set_text)(p, i, &t, 1);
                    },
                );
            }
        }
    }
    // NULL lang / lang_key with an iTXt compression value
    {
        let key = cs("ikey");
        let text = cs("hello");
        rt("iTXt null lang", &img, "iTXt", &|a, p, i| unsafe {
            let t = png_text {
                compression: PNG_ITXT_COMPRESSION_NONE,
                key: key.as_ptr() as *mut c_char,
                text: text.as_ptr() as *mut c_char,
                text_length: 0,
                itxt_length: 0,
                lang: null_mut(),
                lang_key: null_mut(),
            };
            (a.png_set_text)(p, i, &t, 1);
        });
    }

    /* ---------------- C-124 eXIf ---------------- */
    let mut rng = Rng::new(0xe41f);
    for &n in &[0usize, 1, 4, 8, 100, 4096] {
        for it in 0..3 {
            let mut d: Vec<u8> = match it {
                0 => {
                    let mut v = b"II\x2a\x00".to_vec();
                    v.extend(rng.bytes(n.saturating_sub(4)));
                    v
                }
                1 => {
                    let mut v = b"MM\x00\x2a".to_vec();
                    v.extend(rng.bytes(n.saturating_sub(4)));
                    v
                }
                _ => rng.bytes(n),
            };
            d.truncate(n);
            while d.len() < n {
                d.push(0);
            }
            rt(
                &format!("eXIf n={} it={}", n, it),
                &img,
                "eXIf",
                &|a, p, i| unsafe {
                    (a.png_set_eXIf_1)(p, i, d.len() as u32, d.as_ptr() as *mut u8);
                },
            );
        }
    }
    // the deprecated entry points, which only warn
    {
        let d = vec![b'I', b'I', 0x2a, 0x00, 1, 2, 3, 4];
        assert_same("eXIf deprecated setter/getter", |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            let g = guarded(api, png, &mut || {
                (api.png_set_eXIf)(png, info, d.as_ptr() as *mut u8);
                let mut e: *mut u8 = null_mut();
                let r = (api.png_get_eXIf)(png, info, &mut e);
                log(format!("get_eXIf r={} null={}", r, e.is_null()));
                (api.png_set_eXIf_1)(png, info, d.len() as u32, d.as_ptr() as *mut u8);
                let r2 = (api.png_get_eXIf)(png, info, &mut e);
                log(format!("get_eXIf again r={} null={}", r2, e.is_null()));
                dump_chunks(api, png, info, "after eXIf", !0);
            });
            o.push(format!("guard={:?}", g));
            destroy_write(api, png, info);
            o
        });
    }

    /* ---------------- C-125 cICP ---------------- */
    let mut rng = Rng::new(0xc1c9);
    for &matrix in &[0u8, 1, 2, 255] {
        for it in 0..6 {
            let (cp, tf, vf) = match it {
                0 => (1u8, 13u8, 0u8),
                1 => (9, 16, 1),
                2 => (0, 0, 2),
                3 => (255, 255, 255),
                4 => (2, 2, 1),
                _ => (rng.u8(), rng.u8(), rng.u8()),
            };
            rt(
                &format!("cICP {} {} {} {}", cp, tf, matrix, vf),
                &img,
                "cICP",
                &|a, p, i| unsafe {
                    (a.png_set_cICP)(p, i, cp, tf, matrix, vf);
                },
            );
        }
    }

    /* ---------------- C-126 cLLI ---------------- */
    for &(cll, fall) in &[
        (0u32, 0u32),
        (1, 1),
        (10000 * 10000, 10000 * 10000),
        (0x7fff_ffff, 0x7fff_ffff),
        (0x8000_0000, 0),
        (0, 0x8000_0000),
        (0xffff_ffff, 0xffff_ffff),
        (123456, 654321),
    ] {
        rt(
            &format!("cLLI_fixed {} {}", cll, fall),
            &img,
            "cLLI",
            &|a, p, i| unsafe {
                (a.png_set_cLLI_fixed)(p, i, cll, fall);
            },
        );
    }
    for &(cll, fall) in &[
        (0.0f64, 0.0f64),
        (0.0001, 0.0001),
        (1000.0, 100.0),
        (214748.0, 1.0),
        (214749.0, 1.0),
        (-1.0, 1.0),
    ] {
        rt(
            &format!("cLLI {:?} {:?}", cll, fall),
            &img,
            "cLLI",
            &|a, p, i| unsafe {
                (a.png_set_cLLI)(p, i, cll, fall);
            },
        );
    }

    /* ---------------- C-127 mDCV ---------------- */
    let mut rng = Rng::new(0x3dc7);
    for it in 0..14 {
        let (v, maxdl, mindl): ([i32; 8], u32, u32) = match it {
            0 => ([31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000], 0, 0),
            1 => ([0; 8], 1, 1),
            2 => ([131070; 8], 10000 * 10000, 1),
            3 => ([131072; 8], 1, 1),
            4 => ([-2; 8], 1, 1),
            5 => ([1; 8], 0x8000_0000, 1),
            6 => ([1; 8], 1, 0x8000_0000),
            7 => ([65535; 8], 0x7fff_ffff, 0x7fff_ffff),
            _ => {
                let mut a = [0i32; 8];
                for x in a.iter_mut() {
                    *x = rng.range(0, 140000) as i32;
                }
                (a, rng.u32() % 1_000_000, rng.u32() % 1_000_000)
            }
        };
        rt(
            &format!("mDCV_fixed {:?} {} {}", v, maxdl, mindl),
            &img,
            "mDCV",
            &|a, p, i| unsafe {
                (a.png_set_mDCV_fixed)(
                    p, i, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], maxdl, mindl,
                );
            },
        );
        let d: Vec<f64> = v.iter().map(|&x| x as f64 / 100000.0).collect();
        let (fmax, fmin) = (maxdl as f64 / 10000.0, mindl as f64 / 10000.0);
        rt(
            &format!("mDCV {:?} {:?} {:?}", d, fmax, fmin),
            &img,
            "mDCV",
            &|a, p, i| unsafe {
                (a.png_set_mDCV)(
                    p, i, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7], fmax, fmin,
                );
            },
        );
    }
}

/* ------------------------------------------------------------------ */
/* synthetic ICC profiles                                              */
/* ------------------------------------------------------------------ */

#[derive(Clone)]
struct Icc {
    total: usize,
    /// The 4-byte length field; `None` means "use the real length".
    len_field: Option<u32>,
    version8: u8,
    class: [u8; 4],
    space: [u8; 4],
    pcs: [u8; 4],
    sig: [u8; 4],
    intent: u32,
    d50: bool,
    /// Number of tags actually written into the table.
    ntags: u32,
    /// The tag-count field; `None` means "same as `ntags`".
    ntags_field: Option<u32>,
    tag_start: u32,
    tag_len: u32,
}

impl Icc {
    fn new(total: usize) -> Icc {
        Icc {
            total,
            len_field: None,
            version8: 2,
            class: *b"mntr",
            space: *b"RGB ",
            pcs: *b"XYZ ",
            sig: *b"acsp",
            intent: 0,
            d50: true,
            ntags: 1,
            ntags_field: None,
            tag_start: 144,
            tag_len: 8,
        }
    }

    fn build(&self) -> Vec<u8> {
        let ntags = self.ntags;
        let need = 132 + 12 * ntags as usize;
        let mut v = vec![0u8; self.total.max(need)];
        let n = v.len() as u32;
        let lf = self.len_field.unwrap_or(n);
        v[0..4].copy_from_slice(&lf.to_be_bytes());
        v[8] = self.version8;
        v[12..16].copy_from_slice(&self.class);
        v[16..20].copy_from_slice(&self.space);
        v[20..24].copy_from_slice(&self.pcs);
        v[36..40].copy_from_slice(&self.sig);
        v[64..68].copy_from_slice(&self.intent.to_be_bytes());
        if self.d50 {
            v[68..80].copy_from_slice(&[
                0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
            ]);
        }
        let nf = self.ntags_field.unwrap_or(ntags);
        v[128..132].copy_from_slice(&nf.to_be_bytes());
        let ids: [&[u8; 4]; 4] = [b"rXYZ", b"gXYZ", b"bXYZ", b"wtpt"];
        for k in 0..ntags as usize {
            let o = 132 + 12 * k;
            if o + 12 > v.len() {
                break;
            }
            v[o..o + 4].copy_from_slice(ids[k % 4]);
            v[o + 4..o + 8].copy_from_slice(&(self.tag_start + 16 * k as u32).to_be_bytes());
            v[o + 8..o + 12].copy_from_slice(&self.tag_len.to_be_bytes());
        }
        v
    }
}

/// Build a raw iCCP chunk carrying `profile` verbatim (as stored deflate).
fn iccp_chunk(keyword: &[u8], comp: u8, profile: &[u8]) -> Vec<u8> {
    let mut d = keyword.to_vec();
    d.push(0);
    d.push(comp);
    d.extend_from_slice(&zlib_stored(profile));
    chunk(b"iCCP", &d)
}

/* ------------------------------------------------------------------ */
/* C-21 iCCP validation                                                */
/* ------------------------------------------------------------------ */

/// C-21: `png_icc_check_header` / `_length` / `_tag_table`, both directly and
/// through an iCCP chunk on the read side, plus `PNG_SKIP_sRGB_CHECK_PROFILE`.
#[test]
fn iccp() {
    /* the profile variants under test */
    let mut variants: Vec<(String, Vec<u8>, u32)> = Vec::new();
    {
        let good = Icc::new(260);
        variants.push(("sRGB-like".into(), good.build(), 260));

        let mut v = Icc::new(260);
        v.len_field = Some(999);
        variants.push(("wrong length field".into(), v.build(), 260));

        let mut v = Icc::new(260);
        v.len_field = Some(100);
        variants.push(("length field too short".into(), v.build(), 260));

        let v = Icc::new(128);
        variants.push(("too short".into(), v.build(), 128));

        let mut v = Icc::new(260);
        v.sig = *b"XXXX";
        variants.push(("bad signature".into(), v.build(), 260));

        let mut v = Icc::new(260);
        v.ntags = 1;
        v.ntags_field = Some(0);
        variants.push(("0 tags".into(), v.build(), 260));

        let mut v = Icc::new(260);
        v.ntags_field = Some(0xffff_ffff);
        variants.push(("huge tag count".into(), v.build(), 260));

        let mut v = Icc::new(260);
        v.ntags_field = Some(400_000_000);
        variants.push(("tag count over max".into(), v.build(), 260));

        let mut v = Icc::new(260);
        v.ntags = 4;
        v.tag_start = 900;
        variants.push(("tag outside profile".into(), v.build(), 260));

        let mut v = Icc::new(260);
        v.ntags = 4;
        v.tag_len = 0xffff_ffff;
        variants.push(("tag length outside profile".into(), v.build(), 260));

        let mut v = Icc::new(260);
        v.ntags = 4;
        v.tag_start = 145;
        variants.push(("tag misaligned".into(), v.build(), 260));

        let mut v = Icc::new(260);
        v.intent = 4;
        variants.push(("intent out of range".into(), v.build(), 260));

        let mut v = Icc::new(260);
        v.intent = 0xffff;
        variants.push(("intent invalid".into(), v.build(), 260));

        let mut v = Icc::new(260);
        v.d50 = false;
        variants.push(("not D50".into(), v.build(), 260));

        for (tag, space) in [("GRAY", *b"GRAY"), ("CMYK", *b"CMYK")] {
            let mut v = Icc::new(260);
            v.space = space;
            variants.push((format!("space {}", tag), v.build(), 260));
        }
        for (tag, class) in [
            ("scnr", *b"scnr"),
            ("prtr", *b"prtr"),
            ("spac", *b"spac"),
            ("abst", *b"abst"),
            ("link", *b"link"),
            ("nmcl", *b"nmcl"),
            ("zzzz", *b"zzzz"),
        ] {
            let mut v = Icc::new(260);
            v.class = class;
            variants.push((format!("class {}", tag), v.build(), 260));
        }
        for (tag, pcs) in [("Lab ", *b"Lab "), ("bad ", *b"nope")] {
            let mut v = Icc::new(260);
            v.pcs = pcs;
            variants.push((format!("pcs {}", tag), v.build(), 260));
        }
        let mut v = Icc::new(262);
        v.version8 = 4;
        variants.push(("v4 unaligned length".into(), v.build(), 262));
    }

    /* the three check functions called directly */
    for (tag, prof, _) in &variants {
        for &ct in &[PNG_COLOR_TYPE_GRAY, PNG_COLOR_TYPE_RGB] {
            assert_same(&format!("icc_check {} ct={}", tag, ct), |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_read(api);
                let name = cs("check");
                let g = guarded(api, png, &mut || {
                    let plen = u32::from_be_bytes([prof[0], prof[1], prof[2], prof[3]]);
                    log(format!(
                        "check_length(real)={} check_length(field)={}",
                        (api.png_icc_check_length)(png, name.as_ptr(), prof.len() as u32),
                        (api.png_icc_check_length)(png, name.as_ptr(), plen)
                    ));
                    log(format!(
                        "check_header={}",
                        (api.png_icc_check_header)(
                            png,
                            name.as_ptr(),
                            prof.len() as u32,
                            prof.as_ptr(),
                            ct
                        )
                    ));
                    log(format!(
                        "check_tag_table={}",
                        (api.png_icc_check_tag_table)(
                            png,
                            name.as_ptr(),
                            prof.len() as u32,
                            prof.as_ptr()
                        )
                    ));
                });
                o.push(format!("guard={:?}", g));
                destroy_read(api, png, info);
                o
            });
        }
    }

    /* the same profiles fed through an iCCP chunk on the read side */
    let mut base: Vec<u8> = Vec::new();
    let gimg = shape_img(0x1cc0, PNG_COLOR_TYPE_GRAY, 8);
    let cimg = rgb_img(0x1cc1);
    for (which, img) in [("gray", &gimg), ("rgb", &cimg)] {
        assert_same(&format!("iccp base {}", which), |api| unsafe {
            let mut o = Outcome::default();
            let wr = write_plain(api, img, &WriteOpts::default());
            o.output = wr.bytes.clone();
            if api.which == "C" {
                base = wr.bytes.clone();
            }
            o
        });
        let base = base.clone();
        for (tag, prof, _) in &variants {
            for &kw in &[&b"k"[..], &b""[..], &b"a very long keyword that goes past the seventy nine character limit for a png keyword"[..]] {
                for &comp in &[0u8, 1] {
                    let raw = iccp_chunk(kw, comp, prof);
                    let data = insert_before(&base, "IDAT", &raw);
                    assert_same(
                        &format!(
                            "iCCP read {} {} kw={} comp={}",
                            which,
                            tag,
                            kw.len(),
                            comp
                        ),
                        |api| unsafe {
                            let mut o = Outcome::default();
                            read_and_dump(api, &data, &mut o);
                            o
                        },
                    );
                }
            }
        }
        // a truncated / empty iCCP payload
        for raw in [
            chunk(b"iCCP", b""),
            chunk(b"iCCP", b"k\0\0"),
            chunk(b"iCCP", b"k\0\0\x78\x01"),
            chunk(b"iCCP", b"\0\0abcdefghij"),
        ] {
            let data = insert_before(&base, "IDAT", &raw);
            assert_same(&format!("iCCP malformed {} {}", which, raw.len()), |api| unsafe {
                let mut o = Outcome::default();
                read_and_dump(api, &data, &mut o);
                o
            });
        }
    }

    /* png_set_option(PNG_SKIP_sRGB_CHECK_PROFILE, ...) */
    let good = Icc::new(260).build();
    let raw = iccp_chunk(b"k", 0, &good);
    let data = insert_before(&base, "IDAT", &raw);
    for &onoff in &[PNG_OPTION_ON, PNG_OPTION_OFF, 7] {
        assert_same(&format!("skip sRGB check {}", onoff), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = data.to_vec();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
            let prev1 = (api.png_set_option)(png, PNG_SKIP_sRGB_CHECK_PROFILE, onoff);
            let prev2 = (api.png_set_option)(png, PNG_SKIP_sRGB_CHECK_PROFILE, onoff);
            o.push(format!("prev={} {}", prev1, prev2));
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                dump_chunks(api, png, info, "skip-check", !0);
                (api.png_read_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }

    /* png_set_iCCP with a bad profile: the write side rejects it */
    let cimg2 = rgb_img(0x1cc2);
    for (tag, prof, plen) in &variants {
        let name = cs("wk");
        let prof = prof.clone();
        let plen = *plen;
        assert_same(&format!("set_iCCP write {}", tag), |api| unsafe {
            let mut o = Outcome::default();
            let wr = write_image(api, &cimg2, &WriteOpts::default(), &mut |a, p, i| {
                (a.png_set_iCCP)(p, i, name.as_ptr(), 0, prof.as_ptr(), plen);
                // the length field may be a lie and libpng only allocated
                // `plen` bytes, so bound the profile dump.
                dump_chunks(a, p, i, "set_iCCP", plen as usize);
            });
            o.push(format!("guard={:?}", wr.guard));
            o.output = wr.bytes.clone();
            o
        });
    }
}

/* ------------------------------------------------------------------ */
/* C-60 text compression knobs                                         */
/* ------------------------------------------------------------------ */

/// C-60: the text-compression knobs observed through zTXt, iTXt and iCCP.
#[test]
fn text_compression() {
    let img = rgb_img(0x7c60);
    let key = cs("zkey");
    let body: String = "the quick brown fox jumps over the lazy dog "
        .chars()
        .cycle()
        .take(3000)
        .collect();
    let text = cs(&body);
    let lang = cs("en");
    let lkey = cs("Key");
    let iname = cs("icc");
    let prof = Icc::new(1024).build();

    #[derive(Clone, Copy, Debug)]
    struct Knobs {
        level: Option<c_int>,
        strategy: Option<c_int>,
        mem_level: Option<c_int>,
        window_bits: Option<c_int>,
        method: Option<c_int>,
    }
    const NONE: Knobs = Knobs {
        level: None,
        strategy: None,
        mem_level: None,
        window_bits: None,
        method: None,
    };

    let mut cases: Vec<Knobs> = Vec::new();
    for l in [-2, -1, 0, 1, 5, 9, 10] {
        cases.push(Knobs {
            level: Some(l),
            ..NONE
        });
    }
    for s in [0, 1, 2, 3, 4, 5] {
        cases.push(Knobs {
            strategy: Some(s),
            ..NONE
        });
    }
    for m in [0, 1, 2, 5, 8, 9, 10] {
        cases.push(Knobs {
            mem_level: Some(m),
            ..NONE
        });
    }
    for w in [7, 8, 9, 11, 14, 15, 16] {
        cases.push(Knobs {
            window_bits: Some(w),
            ..NONE
        });
    }
    for me in [0, 7, 8, 9] {
        cases.push(Knobs {
            method: Some(me),
            ..NONE
        });
    }
    cases.push(Knobs {
        level: Some(9),
        strategy: Some(2),
        mem_level: Some(9),
        window_bits: Some(15),
        method: Some(8),
    });
    cases.push(Knobs {
        level: Some(1),
        strategy: Some(0),
        mem_level: Some(1),
        window_bits: Some(8),
        method: Some(8),
    });

    for k in cases {
        for kind in ["zTXt", "iTXt", "iCCP"] {
            assert_same(&format!("text_compression {:?} via {}", k, kind), |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_write(api);
                (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
                if let Some(v) = k.level {
                    (api.png_set_text_compression_level)(png, v);
                }
                if let Some(v) = k.strategy {
                    (api.png_set_text_compression_strategy)(png, v);
                }
                if let Some(v) = k.mem_level {
                    (api.png_set_text_compression_mem_level)(png, v);
                }
                if let Some(v) = k.window_bits {
                    (api.png_set_text_compression_window_bits)(png, v);
                }
                if let Some(v) = k.method {
                    (api.png_set_text_compression_method)(png, v);
                }
                let g = guarded(api, png, &mut || {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        img.w,
                        img.h,
                        img.bit_depth,
                        img.color_type,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    match kind {
                        "zTXt" => {
                            let t = png_text {
                                compression: PNG_TEXT_COMPRESSION_zTXt,
                                key: key.as_ptr() as *mut c_char,
                                text: text.as_ptr() as *mut c_char,
                                text_length: 0,
                                itxt_length: 0,
                                lang: null_mut(),
                                lang_key: null_mut(),
                            };
                            (api.png_set_text)(png, info, &t, 1);
                        }
                        "iTXt" => {
                            let t = png_text {
                                compression: PNG_ITXT_COMPRESSION_zTXt,
                                key: key.as_ptr() as *mut c_char,
                                text: text.as_ptr() as *mut c_char,
                                text_length: 0,
                                itxt_length: 0,
                                lang: lang.as_ptr() as *mut c_char,
                                lang_key: lkey.as_ptr() as *mut c_char,
                            };
                            (api.png_set_text)(png, info, &t, 1);
                        }
                        _ => {
                            (api.png_set_iCCP)(
                                png,
                                info,
                                iname.as_ptr(),
                                0,
                                prof.as_ptr(),
                                prof.len() as u32,
                            );
                        }
                    }
                    (api.png_write_info)(png, info);
                    for r in &img.rows {
                        (api.png_write_row)(png, r.as_ptr() as *mut u8);
                    }
                    (api.png_write_end)(png, info);
                });
                o.push(format!("guard={:?}", g));
                o.output = std::mem::take(&mut tls().output);
                destroy_write(api, png, info);
                o
            });
        }
    }
}

/* ------------------------------------------------------------------ */
/* C-128 unknown chunks                                                */
/* ------------------------------------------------------------------ */

const UNAMES: [&[u8; 5]; 6] = [
    b"aBCD\0", // ancillary, public, unsafe to copy
    b"aBCd\0", // ancillary, public, safe to copy
    b"abCD\0", // ancillary, private, unsafe to copy
    b"aBcD\0", // reserved bit set
    b"ABCD\0", // critical, unsafe to copy
    b"ABCd\0", // critical, safe to copy
];

const KEEPS: [c_int; 5] = [
    PNG_HANDLE_CHUNK_AS_DEFAULT,
    PNG_HANDLE_CHUNK_NEVER,
    PNG_HANDLE_CHUNK_IF_SAFE,
    PNG_HANDLE_CHUNK_ALWAYS,
    PNG_HANDLE_CHUNK_LAST,
];

/// C-128: the whole `png_set_unknown_chunks` / `png_set_keep_unknown_chunks`
/// cross product.
#[test]
fn unknown() {
    let img = rgb_img(0x0d15);
    let locations = [
        ("IHDR", PNG_HAVE_IHDR),
        ("PLTE", PNG_HAVE_PLTE),
        ("AFTER_IDAT", PNG_AFTER_IDAT),
        ("IHDR|PLTE", PNG_HAVE_IHDR | PNG_HAVE_PLTE),
        ("zero", 0),
    ];

    /* -------- write side -------- */
    for &keep in &KEEPS {
        for named in [false, true] {
            for &(ltag, loc) in &locations {
                for &size in &[0usize, 1, 5, 1024] {
                    let mut rng = Rng::new(0x0d15 ^ size as u64 ^ ((keep as u64) << 20));
                    let datas: Vec<Vec<u8>> = (0..UNAMES.len()).map(|_| rng.bytes(size)).collect();
                    let list: Vec<u8> = UNAMES.iter().flat_map(|n| n.iter().copied()).collect();
                    assert_same(
                        &format!(
                            "unknown write keep={} named={} loc={} size={}",
                            keep, named, ltag, size
                        ),
                        |api| unsafe {
                            let mut o = Outcome::default();
                            let wr =
                                write_image(api, &img, &WriteOpts::default(), &mut |a, p, i| {
                                    if named {
                                        (a.png_set_keep_unknown_chunks)(
                                            p,
                                            keep,
                                            list.as_ptr(),
                                            UNAMES.len() as c_int,
                                        );
                                    } else {
                                        (a.png_set_keep_unknown_chunks)(p, keep, null_mut(), 0);
                                    }
                                    for n in UNAMES {
                                        log(format!(
                                            "handle_as_unknown {:?}={}",
                                            n,
                                            (a.png_handle_as_unknown)(p, n.as_ptr())
                                        ));
                                    }
                                    let mut us: Vec<png_unknown_chunk> = Vec::new();
                                    for (k, n) in UNAMES.iter().enumerate() {
                                        us.push(png_unknown_chunk {
                                            name: **n,
                                            data: datas[k].as_ptr() as *mut u8,
                                            size: datas[k].len(),
                                            location: loc as u8,
                                        });
                                    }
                                    (a.png_set_unknown_chunks)(
                                        p,
                                        i,
                                        us.as_ptr(),
                                        us.len() as c_int,
                                    );
                                    dump_chunks(a, p, i, "unknown-write", !0);
                                });
                            o.push(format!("guard={:?}", wr.guard));
                            o.output = wr.bytes.clone();
                            o
                        },
                    );
                }
            }
        }
    }

    /* -------- png_set_unknown_chunk_location -------- */
    for &loc in &[0, PNG_HAVE_IHDR, PNG_HAVE_PLTE, PNG_AFTER_IDAT, 0x04, 0xff] {
        for idx in [-1i32, 0, 1, 5, 99] {
            let data = vec![1u8, 2, 3];
            assert_same(
                &format!("unknown_chunk_location idx={} loc={}", idx, loc),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let wr = write_image(api, &img, &WriteOpts::default(), &mut |a, p, i| {
                        let u = png_unknown_chunk {
                            name: *b"aBCd\0",
                            data: data.as_ptr() as *mut u8,
                            size: data.len(),
                            location: PNG_HAVE_IHDR as u8,
                        };
                        (a.png_set_unknown_chunks)(p, i, &u, 1);
                        (a.png_set_unknown_chunk_location)(p, i, idx, loc);
                        dump_chunks(a, p, i, "loc", !0);
                    });
                    o.push(format!("guard={:?}", wr.guard));
                    o.output = wr.bytes.clone();
                    o
                },
            );
        }
    }

    /* -------- keep-list edge cases -------- */
    for &(tag, keep, n) in &[
        ("null list n=1", PNG_HANDLE_CHUNK_ALWAYS, 1i32),
        ("null list n=-1", PNG_HANDLE_CHUNK_ALWAYS, -1),
        ("null list n=0", PNG_HANDLE_CHUNK_NEVER, 0),
        ("bad keep -1", -1, 1),
        ("bad keep LAST", PNG_HANDLE_CHUNK_LAST, 1),
    ] {
        assert_same(&format!("keep_unknown {}", tag), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_read(api);
            let g = guarded(api, png, &mut || {
                (api.png_set_keep_unknown_chunks)(png, keep, null_mut(), n);
                for nm in UNAMES {
                    log(format!(
                        "handle_as_unknown {:?}={} chunk_unknown_handling={}",
                        nm,
                        (api.png_handle_as_unknown)(png, nm.as_ptr()),
                        (api.png_chunk_unknown_handling)(
                            png,
                            u32::from_be_bytes([nm[0], nm[1], nm[2], nm[3]])
                        )
                    ));
                }
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }
    // repeated set calls, overriding and resetting entries
    assert_same("keep_unknown repeated", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_read(api);
        let one: Vec<u8> = UNAMES[0].to_vec();
        let two: Vec<u8> = UNAMES[1].to_vec();
        let all: Vec<u8> = UNAMES.iter().flat_map(|n| n.iter().copied()).collect();
        let g = guarded(api, png, &mut || {
            for (keep, list, n) in [
                (PNG_HANDLE_CHUNK_ALWAYS, &all, UNAMES.len() as c_int),
                (PNG_HANDLE_CHUNK_NEVER, &one, 1),
                (PNG_HANDLE_CHUNK_IF_SAFE, &two, 1),
                (PNG_HANDLE_CHUNK_AS_DEFAULT, &all, UNAMES.len() as c_int),
                (PNG_HANDLE_CHUNK_ALWAYS, &one, 1),
            ] {
                (api.png_set_keep_unknown_chunks)(png, keep, list.as_ptr(), n);
                let mut s = String::new();
                for nm in UNAMES {
                    s += &format!("{} ", (api.png_handle_as_unknown)(png, nm.as_ptr()));
                }
                log(format!("after keep={} n={}: {}", keep, n, s));
            }
        });
        o.push(format!("guard={:?}", g));
        destroy_read(api, png, info);
        o
    });

    /* -------- read side -------- */
    let mut base = Vec::new();
    assert_same("unknown base", |api| unsafe {
        let mut o = Outcome::default();
        let wr = write_plain(api, &img, &WriteOpts::default());
        o.output = wr.bytes.clone();
        if api.which == "C" {
            base = wr.bytes.clone();
        }
        o
    });
    let anc: Vec<&[u8; 5]> = UNAMES[0..4].to_vec();
    for &keep in &KEEPS[0..4] {
        for named in [false, true] {
            for &size in &[0usize, 3, 1024] {
                let mut rng = Rng::new(0xbeef ^ size as u64);
                let mut data = base.clone();
                for n in &anc {
                    let payload = rng.bytes(size);
                    let raw = chunk(&[n[0], n[1], n[2], n[3]], &payload);
                    data = insert_before(&data, "IDAT", &raw);
                }
                for n in &anc {
                    let payload = rng.bytes(size);
                    let raw = chunk(&[n[0], n[1], n[2], n[3]], &payload);
                    data = insert_before(&data, "IEND", &raw);
                }
                let list: Vec<u8> = anc.iter().flat_map(|n| n.iter().copied()).collect();
                assert_same(
                    &format!(
                        "unknown read keep={} named={} size={}",
                        keep, named, size
                    ),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        tls().input = data.to_vec();
                        tls().in_pos = 0;
                        let (png, info) = new_read(api);
                        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                        if named {
                            (api.png_set_keep_unknown_chunks)(
                                png,
                                keep,
                                list.as_ptr(),
                                anc.len() as c_int,
                            );
                        } else {
                            (api.png_set_keep_unknown_chunks)(png, keep, null_mut(), 0);
                        }
                        let g = guarded(api, png, &mut || {
                            (api.png_read_info)(png, info);
                            dump_chunks(api, png, info, "unknown-read-info", !0);
                            let h = (api.png_get_image_height)(png, info) as usize;
                            let rb = (api.png_get_rowbytes)(png, info);
                            let mut row = vec![0u8; rb];
                            for _ in 0..h {
                                (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                            }
                            (api.png_read_end)(png, info);
                            dump_chunks(api, png, info, "unknown-read-end", !0);
                        });
                        o.push(format!("guard={:?}", g));
                        destroy_read(api, png, info);
                        o
                    },
                );
            }
        }
    }
    // critical unknown chunks on the read side
    for &keep in &KEEPS[0..4] {
        for n in &UNAMES[4..6] {
            let raw = chunk(&[n[0], n[1], n[2], n[3]], &[9u8, 8, 7]);
            let data = insert_before(&base, "IDAT", &raw);
            let list: Vec<u8> = n.to_vec();
            for named in [false, true] {
                assert_same(
                    &format!("unknown critical read {:?} keep={} named={}", n, keep, named),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        tls().input = data.to_vec();
                        tls().in_pos = 0;
                        let (png, info) = new_read(api);
                        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                        if named {
                            (api.png_set_keep_unknown_chunks)(png, keep, list.as_ptr(), 1);
                        } else {
                            (api.png_set_keep_unknown_chunks)(png, keep, null_mut(), 0);
                        }
                        let g = guarded(api, png, &mut || {
                            (api.png_read_info)(png, info);
                            dump_chunks(api, png, info, "critical", !0);
                            (api.png_read_end)(png, info);
                        });
                        o.push(format!("guard={:?}", g));
                        destroy_read(api, png, info);
                        o
                    },
                );
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* "set every ancillary chunk" -- shared by set_invalid, rows_and_freer */
/* and write_order.  Every setter copies its argument, so plain locals  */
/* are enough here.                                                    */
/* ------------------------------------------------------------------ */

unsafe fn set_all_chunks(a: &Api, p: *mut PngStruct, i: *mut PngInfo) {
    (a.png_set_gAMA_fixed)(p, i, 45455);
    (a.png_set_cHRM_fixed)(p, i, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
    (a.png_set_sRGB)(p, i, PNG_sRGB_INTENT_PERCEPTUAL);

    let iname = cs("icc");
    let prof = Icc::new(260).build();
    (a.png_set_iCCP)(p, i, iname.as_ptr(), 0, prof.as_ptr(), prof.len() as u32);

    let sb = png_color_8 {
        red: 8,
        green: 8,
        blue: 8,
        gray: 8,
        alpha: 8,
    };
    (a.png_set_sBIT)(p, i, &sb);

    let bg = png_color_16 {
        index: 3,
        red: 1,
        green: 2,
        blue: 3,
        gray: 4,
    };
    (a.png_set_bKGD)(p, i, &bg);

    let hist: Vec<u16> = (0..256u16).collect();
    (a.png_set_hIST)(p, i, hist.as_ptr());

    let alpha: Vec<u8> = vec![0, 64, 128, 255];
    (a.png_set_tRNS)(p, i, alpha.as_ptr(), 4, core::ptr::null());

    (a.png_set_pHYs)(p, i, 100, 200, PNG_RESOLUTION_METER);
    (a.png_set_oFFs)(p, i, 5, -5, PNG_OFFSET_MICROMETER);

    let t = png_time {
        year: 2020,
        month: 3,
        day: 4,
        hour: 5,
        minute: 6,
        second: 7,
    };
    (a.png_set_tIME)(p, i, &t);

    let purpose = cs("purpose");
    let units = cs("units");
    let pa = cs("1.5");
    let pb = cs("-2");
    let mut params: Vec<*mut c_char> = vec![pa.as_ptr() as *mut c_char, pb.as_ptr() as *mut c_char];
    (a.png_set_pCAL)(
        p,
        i,
        purpose.as_ptr(),
        -10,
        10,
        PNG_EQUATION_ARBITRARY,
        2,
        units.as_ptr(),
        params.as_mut_ptr(),
    );

    let sw = cs("1.5");
    let sh = cs("2.5");
    (a.png_set_sCAL_s)(p, i, PNG_SCALE_METER, sw.as_ptr(), sh.as_ptr());

    let n1 = cs("splt one");
    let n2 = cs("splt two");
    let ents: Vec<png_sPLT_entry> = (0..4u16)
        .map(|k| png_sPLT_entry {
            red: k,
            green: k + 1,
            blue: k + 2,
            alpha: k + 3,
            frequency: k + 4,
        })
        .collect();
    let sp = [
        png_sPLT_t {
            name: n1.as_ptr() as *mut c_char,
            depth: 8,
            entries: ents.as_ptr() as *mut png_sPLT_entry,
            nentries: 4,
        },
        png_sPLT_t {
            name: n2.as_ptr() as *mut c_char,
            depth: 16,
            entries: ents.as_ptr() as *mut png_sPLT_entry,
            nentries: 4,
        },
    ];
    (a.png_set_sPLT)(p, i, sp.as_ptr(), 2);

    let k1 = cs("plain");
    let v1 = cs("text value");
    let k2 = cs("comp");
    let v2 = cs("compressed value repeated repeated repeated repeated");
    let k3 = cs("intl");
    let v3 = cs("international text");
    let lang = cs("en");
    let lkey = cs("Intl");
    let ts = [
        png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: k1.as_ptr() as *mut c_char,
            text: v1.as_ptr() as *mut c_char,
            text_length: 0,
            itxt_length: 0,
            lang: null_mut(),
            lang_key: null_mut(),
        },
        png_text {
            compression: PNG_TEXT_COMPRESSION_zTXt,
            key: k2.as_ptr() as *mut c_char,
            text: v2.as_ptr() as *mut c_char,
            text_length: 0,
            itxt_length: 0,
            lang: null_mut(),
            lang_key: null_mut(),
        },
        png_text {
            compression: PNG_ITXT_COMPRESSION_zTXt,
            key: k3.as_ptr() as *mut c_char,
            text: v3.as_ptr() as *mut c_char,
            text_length: 0,
            itxt_length: 0,
            lang: lang.as_ptr() as *mut c_char,
            lang_key: lkey.as_ptr() as *mut c_char,
        },
    ];
    (a.png_set_text)(p, i, ts.as_ptr(), 3);

    let exif: Vec<u8> = b"II\x2a\x00\x08\x00\x00\x00".to_vec();
    (a.png_set_eXIf_1)(p, i, exif.len() as u32, exif.as_ptr() as *mut u8);

    (a.png_set_cICP)(p, i, 9, 16, 0, 1);
    (a.png_set_cLLI_fixed)(p, i, 10_000_000, 1_000_000);
    (a.png_set_mDCV_fixed)(
        p,
        i,
        31270,
        32900,
        64000,
        33000,
        30000,
        60000,
        15000,
        6000,
        10_000_000,
        50,
    );

    let ud: Vec<u8> = vec![1, 2, 3, 4];
    let us = [
        png_unknown_chunk {
            name: *b"aBCd\0",
            data: ud.as_ptr() as *mut u8,
            size: 4,
            location: PNG_HAVE_IHDR as u8,
        },
        png_unknown_chunk {
            name: *b"xYZw\0",
            data: ud.as_ptr() as *mut u8,
            size: 4,
            location: PNG_HAVE_PLTE as u8,
        },
    ];
    (a.png_set_unknown_chunks)(p, i, us.as_ptr(), 2);
}

/* ------------------------------------------------------------------ */
/* C-65 png_set_invalid / png_get_valid                                */
/* ------------------------------------------------------------------ */

/// C-65: every `PNG_INFO_*` bit invalidated singly and in combination before
/// `png_write_info`.
#[test]
fn set_invalid() {
    let img = pal_img(0x1_11a, 8);

    // every bit on its own, then combinations
    let mut masks: Vec<(String, u32)> = vec![("none".to_string(), 0)];
    for (n, b) in INFO_BITS {
        masks.push((n.to_string(), b));
    }
    for bit in 20..32 {
        masks.push((format!("spare bit {}", bit), 1u32 << bit));
    }
    masks.push(("all".to_string(), 0xffff_ffff));
    masks.push(("low16".to_string(), 0xffff));
    masks.push((
        "colour".to_string(),
        PNG_INFO_gAMA | PNG_INFO_cHRM | PNG_INFO_sRGB | PNG_INFO_iCCP,
    ));
    masks.push((
        "pal".to_string(),
        PNG_INFO_PLTE | PNG_INFO_tRNS | PNG_INFO_hIST | PNG_INFO_bKGD,
    ));
    masks.push((
        "v3".to_string(),
        PNG_INFO_cICP | PNG_INFO_cLLI | PNG_INFO_mDCV | PNG_INFO_eXIf,
    ));
    let mut rng = Rng::new(0x1_11b);
    for _ in 0..8 {
        let m = rng.u32() & 0xf_ffff;
        masks.push((format!("random 0x{:x}", m), m));
    }

    for (tag, mask) in masks {
        assert_same(&format!("set_invalid {}", tag), |api| unsafe {
            let mut o = Outcome::default();
            let wr = write_image(api, &img, &WriteOpts::default(), &mut |a, p, i| {
                set_all_chunks(a, p, i);
                dump_chunks(a, p, i, "before invalid", !0);
                (a.png_set_invalid)(p, i, mask as c_int);
                log(format!(
                    "after set_invalid(0x{:x}): valid=0x{:x}",
                    mask,
                    (a.png_get_valid)(p, i, 0xffff_ffff)
                ));
                dump_chunks(a, p, i, "after invalid", !0);
                // invalidating twice must be idempotent
                (a.png_set_invalid)(p, i, mask as c_int);
                log(format!(
                    "again: valid=0x{:x}",
                    (a.png_get_valid)(p, i, 0xffff_ffff)
                ));
            });
            o.push(format!("guard={:?}", wr.guard));
            o.push(format!(
                "chunks={:?}",
                split_chunks(&wr.bytes)
                    .into_iter()
                    .map(|(n, _)| n)
                    .collect::<Vec<_>>()
            ));
            o.output = wr.bytes.clone();
            o
        });
    }

    // png_get_valid with each single bit against a fully populated info struct
    assert_same("get_valid single bits", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                img.w,
                img.h,
                img.bit_depth,
                img.color_type,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_set_PLTE)(
                png,
                info,
                img.palette.as_ptr(),
                img.palette.len() as c_int,
            );
            set_all_chunks(api, png, info);
            for bit in 0..32 {
                log(format!(
                    "valid bit {} = 0x{:x}",
                    bit,
                    (api.png_get_valid)(png, info, 1u32 << bit)
                ));
            }
            // and invalidate them one at a time, cumulatively
            for bit in 0..32 {
                (api.png_set_invalid)(png, info, 1i32 << bit);
                log(format!(
                    "cumulative after bit {}: 0x{:x}",
                    bit,
                    (api.png_get_valid)(png, info, 0xffff_ffff)
                ));
            }
            dump_chunks(api, png, info, "all invalid", !0);
        });
        o.push(format!("guard={:?}", g));
        destroy_write(api, png, info);
        o
    });
}

/* ------------------------------------------------------------------ */
/* C-129 png_set_rows / png_get_rows / png_free_data / png_data_freer   */
/* ------------------------------------------------------------------ */

/// C-129.
#[test]
fn rows_and_freer() {
    let img = rgb_img(0x0_1005);

    /* rows owned by the application (PNG_USER_WILL_FREE_DATA) */
    for &freer in &[PNG_USER_WILL_FREE_DATA, 0, 7] {
        assert_same(&format!("app rows freer={}", freer), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            let mut rows: Vec<*mut u8> = img.rows.iter().map(|r| r.as_ptr() as *mut u8).collect();
            let g = guarded(api, png, &mut || {
                (api.png_set_IHDR)(
                    png,
                    info,
                    img.w,
                    img.h,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                log(format!(
                    "get_rows before set: null={}",
                    (api.png_get_rows)(png, info).is_null()
                ));
                (api.png_set_rows)(png, info, rows.as_mut_ptr());
                log(format!(
                    "valid after set_rows=0x{:x}",
                    (api.png_get_valid)(png, info, 0xffff_ffff)
                ));
                // setting the very same pointer again must not free anything
                (api.png_set_rows)(png, info, rows.as_mut_ptr());
                let got = (api.png_get_rows)(png, info);
                log(format!("get_rows same={}", got == rows.as_mut_ptr()));
                let rb = (api.png_get_rowbytes)(png, info);
                for y in 0..img.h as usize {
                    log(format!(
                        "row {} = {:02x?}",
                        y,
                        core::slice::from_raw_parts(*got.add(y), rb)
                    ));
                }
                (api.png_data_freer)(png, info, freer, PNG_FREE_ROWS);
                // free_me does not contain PNG_FREE_ROWS, so this is a no-op
                (api.png_free_data)(png, info, PNG_FREE_ROWS, -1);
                log(format!(
                    "after free_data: null={} valid=0x{:x}",
                    (api.png_get_rows)(png, info).is_null(),
                    (api.png_get_valid)(png, info, 0xffff_ffff)
                ));
                // drop libpng's reference before the struct is destroyed
                (api.png_set_rows)(png, info, null_mut());
            });
            o.push(format!("guard={:?}", g));
            destroy_write(api, png, info);
            o
        });
    }

    /* rows owned by libpng (PNG_DESTROY_WILL_FREE_DATA) */
    for explicit_free in [false, true] {
        assert_same(
            &format!("libpng rows explicit_free={}", explicit_free),
            |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_write(api);
                let g = guarded(api, png, &mut || {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        img.w,
                        img.h,
                        8,
                        PNG_COLOR_TYPE_RGB,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    let rb = (api.png_get_rowbytes)(png, info);
                    let rows = (api.png_malloc)(
                        png,
                        img.h as usize * core::mem::size_of::<*mut u8>(),
                    ) as *mut *mut u8;
                    for y in 0..img.h as usize {
                        let r = (api.png_malloc)(png, rb) as *mut u8;
                        core::ptr::copy_nonoverlapping(img.rows[y].as_ptr(), r, rb);
                        *rows.add(y) = r;
                    }
                    (api.png_set_rows)(png, info, rows);
                    (api.png_data_freer)(png, info, PNG_DESTROY_WILL_FREE_DATA, PNG_FREE_ROWS);
                    let got = (api.png_get_rows)(png, info);
                    log(format!("get_rows same={}", got == rows));
                    for y in 0..img.h as usize {
                        log(format!(
                            "row {} = {:02x?}",
                            y,
                            core::slice::from_raw_parts(*got.add(y), rb)
                        ));
                    }
                    if explicit_free {
                        (api.png_free_data)(png, info, PNG_FREE_ROWS, -1);
                        log(format!(
                            "after free_data: null={} valid=0x{:x}",
                            (api.png_get_rows)(png, info).is_null(),
                            (api.png_get_valid)(png, info, 0xffff_ffff)
                        ));
                        // a second free must be harmless
                        (api.png_free_data)(png, info, PNG_FREE_ROWS, -1);
                    }
                });
                o.push(format!("guard={:?}", g));
                destroy_write(api, png, info);
                o
            },
        );
    }

    /* png_data_freer with an unknown freer parameter: png_error */
    for &freer in &[-1i32, 3, 100] {
        assert_same(&format!("data_freer bad {}", freer), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            let g = guarded(api, png, &mut || {
                (api.png_data_freer)(png, info, freer, PNG_FREE_ALL);
            });
            o.push(format!("guard={:?}", g));
            destroy_write(api, png, info);
            o
        });
    }

    /* every PNG_FREE_* mask against a fully populated info struct */
    let mut freemasks: Vec<(String, u32)> = vec![
        ("TEXT".into(), PNG_FREE_TEXT),
        ("ROWS".into(), PNG_FREE_ROWS),
        ("PLTE".into(), PNG_FREE_PLTE),
        ("TRNS".into(), PNG_FREE_TRNS),
        ("HIST".into(), PNG_FREE_HIST),
        ("ICCP".into(), PNG_FREE_ICCP),
        ("SPLT".into(), PNG_FREE_SPLT),
        ("PCAL".into(), PNG_FREE_PCAL),
        ("SCAL".into(), PNG_FREE_SCAL),
        ("EXIF".into(), PNG_FREE_EXIF),
        ("UNKN".into(), PNG_FREE_UNKN),
        ("MUL".into(), PNG_FREE_MUL),
        ("ALL".into(), PNG_FREE_ALL),
        ("none".into(), 0),
    ];
    for bit in 0..16 {
        freemasks.push((format!("bit {}", bit), 1u32 << bit));
    }
    for (tag, mask) in freemasks {
        for num in [-1i32, 0] {
            assert_same(
                &format!("free_data {} num={}", tag, num),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let pimg = pal_img(0x0_1006, 8);
                    let (png, info) = new_write(api);
                    let g = guarded(api, png, &mut || {
                        (api.png_set_IHDR)(
                            png,
                            info,
                            pimg.w,
                            pimg.h,
                            pimg.bit_depth,
                            pimg.color_type,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        (api.png_set_PLTE)(
                            png,
                            info,
                            pimg.palette.as_ptr(),
                            pimg.palette.len() as c_int,
                        );
                        set_all_chunks(api, png, info);
                        dump_chunks(api, png, info, "before free", !0);
                        (api.png_free_data)(png, info, mask, num);
                        dump_chunks(api, png, info, "after free", !0);
                        // and again, to be sure the second call is harmless
                        (api.png_free_data)(png, info, mask, num);
                        dump_chunks(api, png, info, "after free twice", !0);
                        // then release everything
                        (api.png_free_data)(png, info, PNG_FREE_ALL, -1);
                        dump_chunks(api, png, info, "after free all", !0);
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_write(api, png, info);
                    o
                },
            );
        }
    }
}

/* ------------------------------------------------------------------ */
/* C-130 png_set_text / png_set_text_2                                 */
/* ------------------------------------------------------------------ */

/// C-130: `num_text` 0 / 1 / many (past the realloc threshold) and every
/// compression value in range and out of it.
#[test]
fn text_many() {
    let img = rgb_img(0x7_e444);

    for &n in &[0i32, 1, 2, 8, 9, 17, 40] {
        for use_2 in [false, true] {
            let keys: Vec<std::ffi::CString> =
                (0..n.max(0)).map(|k| cs(&format!("key{}", k))).collect();
            let vals: Vec<std::ffi::CString> = (0..n.max(0))
                .map(|k| cs(&format!("value number {}", k)))
                .collect();
            assert_same(
                &format!("text n={} set_text_2={}", n, use_2),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let wr = write_image(api, &img, &WriteOpts::default(), &mut |a, p, i| {
                        let ts: Vec<png_text> = (0..keys.len())
                            .map(|k| png_text {
                                compression: PNG_TEXT_COMPRESSION_NONE,
                                key: keys[k].as_ptr() as *mut c_char,
                                text: vals[k].as_ptr() as *mut c_char,
                                text_length: 0,
                                itxt_length: 0,
                                lang: null_mut(),
                                lang_key: null_mut(),
                            })
                            .collect();
                        let ptr = if ts.is_empty() {
                            core::ptr::null()
                        } else {
                            ts.as_ptr()
                        };
                        if use_2 {
                            let r = (a.png_set_text_2)(p, i, ptr, n);
                            log(format!("set_text_2 -> {}", r));
                        } else {
                            (a.png_set_text)(p, i, ptr, n);
                        }
                        dump_chunks(a, p, i, "text", !0);
                        // a second call exercises the array growth path
                        if use_2 {
                            let r = (a.png_set_text_2)(p, i, ptr, n);
                            log(format!("set_text_2 again -> {}", r));
                        } else {
                            (a.png_set_text)(p, i, ptr, n);
                        }
                        dump_chunks(a, p, i, "text twice", !0);
                    });
                    o.push(format!("guard={:?}", wr.guard));
                    o.output = wr.bytes.clone();
                    o
                },
            );
        }
    }

    /* every compression value, in range and out */
    for comp in -5..=5 {
        for lang_set in [false, true] {
            let key = cs("ckey");
            let val = cs("some text to compress or not");
            let lang = cs("en");
            let lkey = cs("Ckey");
            assert_same(
                &format!("text comp={} lang={}", comp, lang_set),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let wr = write_image(api, &img, &WriteOpts::default(), &mut |a, p, i| {
                        let t = png_text {
                            compression: comp,
                            key: key.as_ptr() as *mut c_char,
                            text: val.as_ptr() as *mut c_char,
                            text_length: 0,
                            itxt_length: 0,
                            lang: if lang_set {
                                lang.as_ptr() as *mut c_char
                            } else {
                                null_mut()
                            },
                            lang_key: if lang_set {
                                lkey.as_ptr() as *mut c_char
                            } else {
                                null_mut()
                            },
                        };
                        let r = (a.png_set_text_2)(p, i, &t, 1);
                        log(format!("set_text_2 -> {}", r));
                        dump_chunks(a, p, i, "comp", !0);
                    });
                    o.push(format!("guard={:?}", wr.guard));
                    o.output = wr.bytes.clone();
                    o
                },
            );
        }
    }

    /* NULL keys, NULL text and empty text */
    let key = cs("k");
    let empty = cs("");
    let val = cs("v");
    for (tag, mk) in [
        ("null key", 0),
        ("null text", 1),
        ("empty text", 2),
        ("null key + valid", 3),
    ] {
        assert_same(&format!("text {}", tag), |api| unsafe {
            let mut o = Outcome::default();
            let wr = write_image(api, &img, &WriteOpts::default(), &mut |a, p, i| {
                let mut ts: Vec<png_text> = Vec::new();
                let mut push = |c: c_int, k: *mut c_char, t: *mut c_char| {
                    ts.push(png_text {
                        compression: c,
                        key: k,
                        text: t,
                        text_length: 0,
                        itxt_length: 0,
                        lang: null_mut(),
                        lang_key: null_mut(),
                    });
                };
                match mk {
                    0 => push(PNG_TEXT_COMPRESSION_NONE, null_mut(), val.as_ptr() as *mut _),
                    1 => push(
                        PNG_TEXT_COMPRESSION_NONE,
                        key.as_ptr() as *mut _,
                        null_mut(),
                    ),
                    2 => push(
                        PNG_TEXT_COMPRESSION_NONE,
                        key.as_ptr() as *mut _,
                        empty.as_ptr() as *mut _,
                    ),
                    _ => {
                        push(PNG_TEXT_COMPRESSION_NONE, null_mut(), val.as_ptr() as *mut _);
                        push(
                            PNG_ITXT_COMPRESSION_zTXt,
                            key.as_ptr() as *mut _,
                            null_mut(),
                        );
                        push(
                            PNG_TEXT_COMPRESSION_zTXt,
                            key.as_ptr() as *mut _,
                            val.as_ptr() as *mut _,
                        );
                    }
                }
                let r = (a.png_set_text_2)(p, i, ts.as_ptr(), ts.len() as c_int);
                log(format!("set_text_2 -> {}", r));
                dump_chunks(a, p, i, tag, !0);
            });
            o.push(format!("guard={:?}", wr.guard));
            o.output = wr.bytes.clone();
            o
        });
    }
}

/* ------------------------------------------------------------------ */
/* C-147 png_set_read_user_chunk_fn                                    */
/* ------------------------------------------------------------------ */

thread_local! {
    static UCRET: std::cell::Cell<c_int> = const { std::cell::Cell::new(0) };
}

unsafe extern "C" fn user_chunk_cb(png: *mut PngStruct, ch: *mut png_unknown_chunk) -> c_int {
    let u = *ch;
    let data = if u.data.is_null() || u.size == 0 {
        format!("<empty size={}>", u.size)
    } else {
        format!(
            "{:02x?}",
            core::slice::from_raw_parts(u.data, u.size.min(64))
        )
    };
    let ret = UCRET.with(|c| c.get());
    let p = (cur_api().png_get_user_chunk_ptr)(png);
    log(format!(
        "user_chunk name={:02x?} size={} loc={} data={} ptr={} -> {}",
        u.name,
        u.size,
        u.location,
        data,
        p as usize,
        ret
    ));
    ret
}

/// C-147: the user chunk callback returning -1, 0 and 1, on ancillary and
/// critical unknown chunks.
#[test]
fn user_chunk_fn() {
    let img = rgb_img(0x0_c8fa);
    let mut base = Vec::new();
    assert_same("user_chunk base", |api| unsafe {
        let mut o = Outcome::default();
        let wr = write_plain(api, &img, &WriteOpts::default());
        o.output = wr.bytes.clone();
        if api.which == "C" {
            base = wr.bytes.clone();
        }
        o
    });

    for n in UNAMES {
        for &size in &[0usize, 4, 300] {
            let mut rng = Rng::new(0x0_c8fb ^ size as u64);
            let payload = rng.bytes(size);
            let raw = chunk(&[n[0], n[1], n[2], n[3]], &payload);
            for pos in ["IDAT", "IEND"] {
                let data = insert_before(&base, pos, &raw);
                for ret in [-1i32, 0, 1, 2] {
                    for &keep in &[
                        PNG_HANDLE_CHUNK_AS_DEFAULT,
                        PNG_HANDLE_CHUNK_NEVER,
                        PNG_HANDLE_CHUNK_IF_SAFE,
                        PNG_HANDLE_CHUNK_ALWAYS,
                    ] {
                        assert_same(
                            &format!(
                                "user_chunk {:?} size={} pos={} ret={} keep={}",
                                n, size, pos, ret, keep
                            ),
                            |api| unsafe {
                                let mut o = Outcome::default();
                                UCRET.with(|c| c.set(ret));
                                tls().input = data.to_vec();
                                tls().in_pos = 0;
                                let (png, info) = new_read(api);
                                (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                                log(format!(
                                    "user_chunk_ptr before={}",
                                    (api.png_get_user_chunk_ptr)(png) as usize
                                ));
                                (api.png_set_read_user_chunk_fn)(
                                    png,
                                    0x1234 as *mut c_void,
                                    Some(user_chunk_cb),
                                );
                                log(format!(
                                    "user_chunk_ptr after={}",
                                    (api.png_get_user_chunk_ptr)(png) as usize
                                ));
                                (api.png_set_keep_unknown_chunks)(png, keep, null_mut(), 0);
                                let g = guarded(api, png, &mut || {
                                    (api.png_read_info)(png, info);
                                    dump_chunks(api, png, info, "uc-info", !0);
                                    let h = (api.png_get_image_height)(png, info) as usize;
                                    let rb = (api.png_get_rowbytes)(png, info);
                                    let mut row = vec![0u8; rb];
                                    for _ in 0..h {
                                        (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                                    }
                                    (api.png_read_end)(png, info);
                                    dump_chunks(api, png, info, "uc-end", !0);
                                });
                                o.push(format!("guard={:?}", g));
                                destroy_read(api, png, info);
                                o
                            },
                        );
                    }
                }
            }
        }
    }

    // clearing the callback again
    assert_same("user_chunk cleared", |api| unsafe {
        let mut o = Outcome::default();
        let raw = chunk(b"aBCd", &[1u8, 2, 3]);
        let data = insert_before(&base, "IDAT", &raw);
        tls().input = data;
        tls().in_pos = 0;
        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
        (api.png_set_read_user_chunk_fn)(png, 0x99 as *mut c_void, Some(user_chunk_cb));
        (api.png_set_read_user_chunk_fn)(png, null_mut(), None);
        log(format!(
            "user_chunk_ptr={}",
            (api.png_get_user_chunk_ptr)(png) as usize
        ));
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
            dump_chunks(api, png, info, "cleared", !0);
            (api.png_read_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        destroy_read(api, png, info);
        o
    });
}

/* ------------------------------------------------------------------ */
/* C-153 png_write_info_before_PLTE / png_write_info / png_write_end    */
/* ------------------------------------------------------------------ */

/// C-153.
#[test]
fn write_order() {
    for (which, img) in [
        ("palette", pal_img(0x0_0d43, 8)),
        ("rgb", rgb_img(0x0_0d44)),
    ] {
        for mode in 0..7 {
            assert_same(&format!("write_order {} mode={}", which, mode), |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_write(api);
                (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
                let g = guarded(api, png, &mut || {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        img.w,
                        img.h,
                        img.bit_depth,
                        img.color_type,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    if img.color_type == PNG_COLOR_TYPE_PALETTE {
                        (api.png_set_PLTE)(
                            png,
                            info,
                            img.palette.as_ptr(),
                            img.palette.len() as c_int,
                        );
                    }
                    match mode {
                        // the plain path
                        0 => {
                            set_all_chunks(api, png, info);
                            (api.png_write_info)(png, info);
                        }
                        // explicit before_PLTE, then the rest
                        1 => {
                            set_all_chunks(api, png, info);
                            (api.png_write_info_before_PLTE)(png, info);
                            log("before_PLTE done".to_string());
                            (api.png_write_info)(png, info);
                        }
                        // chunks installed only after before_PLTE has run: the
                        // pre-PLTE ones are then silently dropped
                        2 => {
                            (api.png_write_info_before_PLTE)(png, info);
                            set_all_chunks(api, png, info);
                            (api.png_write_info)(png, info);
                        }
                        // repeated calls
                        3 => {
                            set_all_chunks(api, png, info);
                            (api.png_write_info_before_PLTE)(png, info);
                            (api.png_write_info_before_PLTE)(png, info);
                            (api.png_write_info)(png, info);
                        }
                        // png_write_end(png, NULL)
                        4 => {
                            set_all_chunks(api, png, info);
                            (api.png_write_info)(png, info);
                        }
                        // NULL info
                        5 => {
                            set_all_chunks(api, png, info);
                            (api.png_write_info_before_PLTE)(png, null_mut());
                            (api.png_write_info)(png, null_mut());
                            (api.png_write_info)(png, info);
                        }
                        // before_PLTE only, no chunks at all
                        _ => {
                            (api.png_write_info_before_PLTE)(png, info);
                            (api.png_write_info)(png, info);
                        }
                    }
                    for r in &img.rows {
                        (api.png_write_row)(png, r.as_ptr() as *mut u8);
                    }
                    if mode == 4 {
                        (api.png_write_end)(png, null_mut());
                    } else {
                        (api.png_write_end)(png, info);
                    }
                    dump_chunks(api, png, info, "after write_end", !0);
                });
                o.push(format!("guard={:?}", g));
                o.output = std::mem::take(&mut tls().output);
                o.push(format!(
                    "chunks={:?}",
                    split_chunks(&o.output)
                        .into_iter()
                        .map(|(n, _)| n)
                        .collect::<Vec<_>>()
                ));
                destroy_write(api, png, info);
                o
            });
        }
    }

    // png_write_end without any IDAT, and png_write_end(NULL, ...)
    assert_same("write_end without IDAT", |api| unsafe {
        let mut o = Outcome::default();
        let img = rgb_img(0x0_0d45);
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                img.w,
                img.h,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
            (api.png_write_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        o.output = std::mem::take(&mut tls().output);
        destroy_write(api, png, info);
        o
    });
}
