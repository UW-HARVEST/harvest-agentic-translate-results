//! Phase B cases for the exported PNG_INTERNAL_* entry points.
//!
//! This build ships libpng's private helpers in the ABI (there is no
//! visibility script), so the very lowest-level leaf functions are exercised
//! DIRECTLY here rather than only through the public wrappers. Driving them in
//! isolation pins any C-vs-Rust divergence to a single function instead of
//! seeing it smeared through the whole read/write pipeline.
//!
//! HARD RULES (see the task contract):
//!   * libpng is reachable only through `(api.png_xxx)(...)` / `api.data_tables()`.
//!   * all output goes through `p!`, `emit`, `emit_bytes` — never a pointer value.
//!   * every buffer is sized from the C contract *with slack* and zero-filled, so
//!     neither library ever reads/writes out of bounds (that would be a false diff).

use crate::api::Api;
use crate::p;
use crate::support::*;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

pub fn run(api: &Api, case: &str, seed: u64) -> bool {
    let mut rng = Rng::new(seed);
    match case {
        "ll/math_fixed" => math_fixed(api, &mut rng),
        "ll/gamma_correct" => gamma_correct(api, &mut rng),
        "ll/srgb_tables" => srgb_tables(api, &mut rng),
        "ll/colorspace_xy" => colorspace_xy(api, &mut rng),
        "ll/fp_strings" => fp_strings(api, &mut rng),
        "ll/ascii_from" => ascii_from(api, &mut rng),
        "ll/safecat_format" => safecat_format(api, &mut rng),
        "ll/check_keyword" => check_keyword(api, &mut rng),
        "ll/row_bgr_invert_swap" => row_bgr_invert_swap(api, &mut rng),
        "ll/row_strip_channel" => row_strip_channel(api, &mut rng),
        "ll/row_read_interlace" => row_read_interlace(api, &mut rng),
        "ll/row_write_interlace" => row_write_interlace(api, &mut rng),
        "ll/read_filter_row" => read_filter_row(api, &mut rng),
        "ll/check_ihdr_direct" => check_ihdr_direct(api, &mut rng),
        "ll/chunk_unknown_handling" => chunk_unknown_handling(api, &mut rng),
        "ll/user_version_check" => user_version_check(api, &mut rng),
        "ll/zalloc" => zalloc(api, &mut rng),
        "ll/malloc_internals" => malloc_internals(api, &mut rng),
        "ll/set_text_2" => set_text_2(api, &mut rng),
        "ll/warning_params" => warning_params(api, &mut rng),
        "ll/app_error" => app_error(api, &mut rng),
        "ll/filter_heuristics" => filter_heuristics(api, &mut rng),
        "ll/create_png_struct" => create_png_struct(api, &mut rng),
        "ll/io_direct" => io_direct(api, &mut rng),
        "ll/crc_direct" => crc_direct(api, &mut rng),
        "ll/write_chunks_direct" => write_chunks_direct(api, &mut rng),
        "ll/error_dispatch" => error_dispatch(api, &mut rng),
        _ => return false,
    }
    true
}

/* ------------------------------------------------------------------ */
/* helpers                                                             */
/* ------------------------------------------------------------------ */

/// A read struct wired to the transcript callbacks. Many of the internal
/// entry points only touch a couple of fields (bit_depth, chunk_name, flags)
/// so a plain freshly-created struct is a valid receiver.
unsafe fn read_png(api: &Api) -> png_structp {
    let png = (api.png_create_read_struct)(
        cptr(PNG_LIBPNG_VER_STRING),
        vnull(),
        Some(err_fn),
        Some(warn_fn),
    );
    if png.is_null() {
        emit("create_read_struct: NULL");
        finish(EXIT_PNG_ERROR);
    }
    png
}

unsafe fn write_png(api: &Api) -> png_structp {
    let png = (api.png_create_write_struct)(
        cptr(PNG_LIBPNG_VER_STRING),
        vnull(),
        Some(err_fn),
        Some(warn_fn),
    );
    if png.is_null() {
        emit("create_write_struct: NULL");
        finish(EXIT_PNG_ERROR);
    }
    png
}

unsafe fn destroy_read(api: &Api, png: png_structp) {
    let mut p = png;
    (api.png_destroy_read_struct)(&mut p, null(), null());
}

unsafe fn destroy_write(api: &Api, png: png_structp) {
    let mut p = png;
    (api.png_destroy_write_struct)(&mut p, null());
}

/// Build a row_info by hand for a (color, depth) combo at a given width, laid
/// out exactly like libpng's own row_info.
fn row_info_for(w: u32, color: u8, depth: u8) -> png_row_info {
    let ch = channels_of(color) as u8;
    png_row_info {
        width: w,
        rowbytes: rowbytes_of(w, depth, color),
        color_type: color,
        bit_depth: depth,
        channels: ch,
        pixel_depth: ch.wrapping_mul(depth),
    }
}

/* ------------------------------------------------------------------ */
/* ll/math_fixed                                                       */
/* ------------------------------------------------------------------ */

/// png_muldiv / png_reciprocal / png_reciprocal2 / png_fixed / png_fixed_ITU.
fn math_fixed(api: &Api, rng: &mut Rng) {
    unsafe {
        let bounds: [i32; 15] = [
            0, 1, -1, i32::MAX, i32::MIN, i32::MIN + 1, 100000, 50000, 21474,
            21475, -21474, -21475, 214748, 214749, -214748,
        ];

        // png_muldiv over the cross product of boundary values, then random.
        for &a in &bounds {
            for &t in &bounds {
                for &d in &bounds {
                    let mut res: i32 = 0x5555_5555;
                    let r = (api.png_muldiv)(&mut res, a, t, d);
                    // res is only defined when r != 0; only print it then.
                    if r != 0 {
                        p!("muldiv {} {} {} -> r={} res={}", a, t, d, r, res);
                    } else {
                        p!("muldiv {} {} {} -> r=0", a, t, d);
                    }
                }
            }
        }
        for _ in 0..2200 {
            let a = rng.u32() as i32;
            let t = rng.u32() as i32;
            let d = rng.u32() as i32;
            let mut res: i32 = 0x5555_5555;
            let r = (api.png_muldiv)(&mut res, a, t, d);
            if r != 0 {
                p!("muldiv# {} {} {} -> r={} res={}", a, t, d, r, res);
            } else {
                p!("muldiv# {} {} {} -> r=0", a, t, d);
            }
        }

        // png_reciprocal / png_reciprocal2.
        for &a in &bounds {
            p!("recip {} -> {}", a, (api.png_reciprocal)(a));
            for &b in &bounds {
                p!("recip2 {} {} -> {}", a, b, (api.png_reciprocal2)(a, b));
            }
        }
        for _ in 0..2200 {
            let a = rng.u32() as i32;
            let b = rng.u32() as i32;
            p!("recip# {} -> {}", a, (api.png_reciprocal)(a));
            p!("recip2# {} {} -> {}", a, b, (api.png_reciprocal2)(a, b));
        }

        // png_fixed / png_fixed_ITU only accept in-range doubles (they
        // png_error on overflow, covered elsewhere). png_fixed maps by *1e5,
        // ITU by *1e4 and requires the result >= 0.
        let png = read_png(api);
        let text = cs("math_fixed");
        // Values chosen so 1e5*fp and 1e4*fp stay well within i32 range and,
        // for ITU, non-negative.
        let fixed_vals: [f64; 12] = [
            0.0, 1.0, 0.5, 0.4545, 2.2, 0.00001, 123.456, 21473.9, 0.99999,
            1000.0, 2.0, 0.1,
        ];
        for &fp in &fixed_vals {
            p!("fixed {:.6} -> {}", fp, (api.png_fixed)(png, fp, text.as_ptr()));
            p!("fixed_ITU {:.6} -> {}", fp, (api.png_fixed_ITU)(png, fp, text.as_ptr()));
        }
        for _ in 0..2200 {
            // 0.0 ..= ~200000/1e5, kept non-negative and in range for both.
            let fp = (rng.below(2_000_000) as f64) / 10000.0; // 0 .. 200
            p!("fixed# {:.6} -> {}", fp, (api.png_fixed)(png, fp, text.as_ptr()));
            p!("fixed_ITU# {:.6} -> {}", fp, (api.png_fixed_ITU)(png, fp, text.as_ptr()));
        }
        destroy_read(api, png);
    }
}

/* ------------------------------------------------------------------ */
/* ll/gamma_correct                                                    */
/* ------------------------------------------------------------------ */

fn gamma_correct(api: &Api, rng: &mut Rng) {
    unsafe {
        // A spread of fixed-point gamma values (1.0 == 100000).
        let mut gammas: Vec<i32> = vec![
            0, 1, -1, 45455, 50000, 90000, 99000, 99999, 100000, 100001, 100035,
            110000, 200000, 220000, 250000, 454550, i32::MAX, i32::MIN + 1,
        ];
        for _ in 0..64 {
            gammas.push(rng.below(400001) as i32);
        }

        for &g in &gammas {
            p!("significant {} -> {}", g, (api.png_gamma_significant)(g));
        }

        // 8-bit: ALL 256 input values against a set of gammas.
        let gset8: [i32; 8] = [1, 45455, 50000, 100000, 100001, 220000, 454550, 99999];
        for &g in &gset8 {
            for v in 0u32..=255 {
                p!("g8 v={} g={} -> {}", v, g, (api.png_gamma_8bit_correct)(v, g));
            }
        }

        // 16-bit: a spread of values against a set of gammas.
        let mut vals16: Vec<u32> = vec![
            0, 1, 2, 127, 128, 255, 256, 32767, 32768, 65534, 65535,
        ];
        for _ in 0..200 {
            vals16.push(rng.below(65536));
        }
        for &g in &gset8 {
            for &v in &vals16 {
                p!("g16 v={} g={} -> {}", v, g, (api.png_gamma_16bit_correct)(v, g));
            }
        }

        // png_gamma_correct on a read struct AFTER png_read_info, so its
        // bit_depth field is populated from a real IHDR (that is what selects
        // the 8-bit vs 16-bit path inside the function).
        for &(color, depth) in &[(2u8, 8u8), (2u8, 16u8), (0u8, 8u8), (0u8, 16u8)] {
            let data = build_png(rng, 8, 8, depth, color, &[], &[]);
            let mut r = Reader::new(api, data);
            (api.png_read_info)(r.png, r.info);
            for &g in &gset8 {
                for v in [0u32, 1, 100, 200, 255, 4096, 32768, 65535] {
                    p!("gcorrect c={} d={} v={} g={} -> {}",
                       color, depth, v, g, (api.png_gamma_correct)(r.png, v, g));
                }
            }
            r.destroy();
        }
    }
}

/* ------------------------------------------------------------------ */
/* ll/srgb_tables                                                      */
/* ------------------------------------------------------------------ */

fn srgb_tables(api: &Api, _rng: &mut Rng) {
    unsafe {
        let t = match api.data_tables() {
            Ok(t) => t,
            Err(e) => {
                p!("data_tables err={}", e);
                finish(EXIT_PNG_ERROR);
            }
        };
        // png_sRGB_table: 256 u16. Emit the raw little-endian byte image so a
        // single-element mismatch shows in the digest.
        let table = std::slice::from_raw_parts(t.png_sRGB_table, 256);
        let mut b = Vec::with_capacity(512);
        for x in table {
            b.extend_from_slice(&x.to_le_bytes());
        }
        emit_bytes("sRGB_table", &b);

        // png_sRGB_base: 512 u16.
        let base = std::slice::from_raw_parts(t.png_sRGB_base, 512);
        let mut b = Vec::with_capacity(1024);
        for x in base {
            b.extend_from_slice(&x.to_le_bytes());
        }
        emit_bytes("sRGB_base", &b);

        // png_sRGB_delta: 512 u8.
        let delta = std::slice::from_raw_parts(t.png_sRGB_delta, 512);
        emit_bytes("sRGB_delta", delta);
    }
}

/* ------------------------------------------------------------------ */
/* ll/colorspace_xy                                                    */
/* ------------------------------------------------------------------ */

fn emit_xyz(tag: &str, r: c_int, x: &png_XYZ) {
    p!("{} r={} {} {} {} {} {} {} {} {} {}", tag, r,
       x.red_X, x.red_Y, x.red_Z,
       x.green_X, x.green_Y, x.green_Z,
       x.blue_X, x.blue_Y, x.blue_Z);
}

fn emit_xy(tag: &str, r: c_int, x: &png_xy) {
    p!("{} r={} {} {} {} {} {} {} {} {}", tag, r,
       x.redx, x.redy, x.greenx, x.greeny,
       x.bluex, x.bluey, x.whitex, x.whitey);
}

fn colorspace_xy(api: &Api, rng: &mut Rng) {
    unsafe {
        // sRGB primaries in fixed point (1e5), the canonical valid input.
        let srgb = png_xy {
            redx: 64000, redy: 33000,
            greenx: 30000, greeny: 60000,
            bluex: 15000, bluey: 6000,
            whitex: 31270, whitey: 32900,
        };
        {
            let mut xyz = png_XYZ::default();
            let r = (api.png_XYZ_from_xy)(&mut xyz, &srgb);
            emit_xyz("XYZ<-sRGB", r, &xyz);
            // Round-trip.
            let mut xy = png_xy::default();
            let r2 = (api.png_xy_from_XYZ)(&mut xy, &xyz);
            emit_xy("xy<-XYZ(roundtrip)", r2, &xy);
        }
        // Degenerate all-zero input for both directions.
        {
            let z = png_xy::default();
            let mut xyz = png_XYZ::default();
            emit_xyz("XYZ<-zero", (api.png_XYZ_from_xy)(&mut xyz, &z), &xyz);
            let z2 = png_XYZ::default();
            let mut xy = png_xy::default();
            emit_xy("xy<-zero", (api.png_xy_from_XYZ)(&mut xy, &z2), &xy);
        }

        // >=500 randomized xy and XYZ values. Keep them in the range libpng's
        // arithmetic tolerates (0..110000-ish) but include some out-of-range to
        // exercise the failure return; the function only reads the struct and
        // writes its own out-struct, so this can never touch caller memory.
        for _ in 0..520 {
            let xy_in = png_xy {
                redx: rng.below(130000) as i32,
                redy: rng.below(130000) as i32,
                greenx: rng.below(130000) as i32,
                greeny: rng.below(130000) as i32,
                bluex: rng.below(130000) as i32,
                bluey: rng.below(130000) as i32,
                whitex: rng.below(130000) as i32,
                whitey: (1 + rng.below(130000)) as i32,
            };
            let mut xyz = png_XYZ::default();
            emit_xyz("XYZ#", (api.png_XYZ_from_xy)(&mut xyz, &xy_in), &xyz);

            let xyz_in = png_XYZ {
                red_X: rng.below(110000) as i32,
                red_Y: rng.below(110000) as i32,
                red_Z: rng.below(110000) as i32,
                green_X: rng.below(110000) as i32,
                green_Y: rng.below(110000) as i32,
                green_Z: rng.below(110000) as i32,
                blue_X: rng.below(110000) as i32,
                blue_Y: rng.below(110000) as i32,
                blue_Z: rng.below(110000) as i32,
            };
            let mut xy = png_xy::default();
            emit_xy("xy#", (api.png_xy_from_XYZ)(&mut xy, &xyz_in), &xy);
        }
    }
}

/* ------------------------------------------------------------------ */
/* ll/fp_strings                                                       */
/* ------------------------------------------------------------------ */

fn fp_strings(api: &Api, rng: &mut Rng) {
    unsafe {
        let cases: [&str; 18] = [
            "1", "1.0", "-1.5e10", "+.5", ".", "e5", "1e", "1e+", "", "0", "00",
            "1.2.3", " 1", "1 ", "1e999999", "inf", "nan", "3.14159e-7",
        ];
        for s in &cases {
            let bytes = s.as_bytes();
            let cbuf: Vec<c_char> = bytes.iter().map(|b| *b as c_char).collect();
            // png_check_fp_number over the whole declared size, from state 0.
            let mut state: c_int = 0;
            let mut whereami: usize = 0;
            let r = (api.png_check_fp_number)(
                cbuf.as_ptr(),
                bytes.len(),
                &mut state,
                &mut whereami,
            );
            p!("fpnum {:?} -> r={} state={} where={}", s, r, state, whereami);
            let rs = (api.png_check_fp_string)(cbuf.as_ptr(), bytes.len());
            p!("fpstr {:?} -> {}", s, rs);
        }

        // Randomized short ASCII strings drawn from the numeric alphabet so the
        // parser actually walks its state machine.
        let alpha = b"0123456789.eE+- ";
        for _ in 0..300 {
            let n = rng.range(0, 10) as usize;
            let mut bytes = Vec::with_capacity(n);
            for _ in 0..n {
                bytes.push(alpha[(rng.below(alpha.len() as u32)) as usize]);
            }
            let cbuf: Vec<c_char> = bytes.iter().map(|b| *b as c_char).collect();
            let mut state: c_int = 0;
            let mut whereami: usize = 0;
            let r = (api.png_check_fp_number)(cbuf.as_ptr(), bytes.len(), &mut state, &mut whereami);
            let rs = (api.png_check_fp_string)(cbuf.as_ptr(), bytes.len());
            p!("fp# {:?} num r={} state={} where={} str={}",
               String::from_utf8_lossy(&bytes), r, state, whereami, rs);
        }
    }
}

/* ------------------------------------------------------------------ */
/* ll/ascii_from                                                       */
/* ------------------------------------------------------------------ */

fn ascii_from(api: &Api, rng: &mut Rng) {
    unsafe {
        let png = read_png(api);
        // Both functions png_error on "buffer too small"; always pass a big
        // buffer (>=64) so that never happens here.
        let vals: [f64; 14] = [
            0.0, 1.0, -1.0, 0.5, -0.5, 3.14159265358979, 1e-7, 1e10, 123456.789,
            0.0001, -0.0001, 9999999999.0, 2.2, 0.4545454545,
        ];
        for prec in 1u32..=15 {
            for &fp in &vals {
                let mut buf = [0i8; 64];
                (api.png_ascii_from_fp)(png, buf.as_mut_ptr() as *mut c_char, buf.len(), fp, prec);
                let s = cbuf_string(&buf);
                p!("afp p={} {:.10} -> {:?}", prec, fp, s);
                emit_bytes(&format!("afp p={} {:.10}", prec, fp), &cbuf_bytes(&buf));
            }
        }
        // png_ascii_from_fixed: fixed-point (1e5) integer inputs.
        let fixed_vals: [i32; 12] = [
            0, 1, -1, 100000, -100000, 45455, 250000, 99999, -99999, 2147483647,
            -2147483647, 12345,
        ];
        for &fx in &fixed_vals {
            let mut buf = [0i8; 64];
            (api.png_ascii_from_fixed)(png, buf.as_mut_ptr() as *mut c_char, buf.len(), fx);
            let s = cbuf_string(&buf);
            p!("afx {} -> {:?}", fx, s);
            emit_bytes(&format!("afx {}", fx), &cbuf_bytes(&buf));
        }
        for _ in 0..200 {
            let fp = ((rng.u32() as f64) / (u32::MAX as f64) - 0.5) * 2.0e6;
            let prec = rng.range(1, 15);
            let mut buf = [0i8; 64];
            (api.png_ascii_from_fp)(png, buf.as_mut_ptr() as *mut c_char, buf.len(), fp, prec);
            p!("afp# p={} {:.6} -> {:?}", prec, fp, cbuf_string(&buf));
            let fx = rng.u32() as i32;
            let mut buf2 = [0i8; 64];
            (api.png_ascii_from_fixed)(png, buf2.as_mut_ptr() as *mut c_char, buf2.len(), fx);
            p!("afx# {} -> {:?}", fx, cbuf_string(&buf2));
        }
        destroy_read(api, png);
    }
}

fn cbuf_string(buf: &[i8]) -> String {
    let b: Vec<u8> = buf.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
    String::from_utf8_lossy(&b).into_owned()
}
fn cbuf_bytes(buf: &[i8]) -> Vec<u8> {
    // Emit the entire buffer image (including the bytes past the terminator)
    // so an off-by-one write difference is caught.
    buf.iter().map(|c| *c as u8).collect()
}

/* ------------------------------------------------------------------ */
/* ll/safecat_format                                                   */
/* ------------------------------------------------------------------ */

fn safecat_format(api: &Api, rng: &mut Rng) {
    unsafe {
        // png_safecat with varying bufsize/pos/strings, including truncation.
        let strings = ["", "a", "hello", "0123456789", "abcdefghijklmnopqrst"];
        for bufsize in [0usize, 1, 2, 5, 8, 16, 32] {
            for pos in [0usize, 1, 3, 7, 15, 31] {
                for s in &strings {
                    // Always allocate 64 bytes of backing store so no matter
                    // what bufsize/pos we pass, the C code (which is bounded by
                    // bufsize) can never run past the real allocation.
                    let mut buf = [0u8; 64];
                    let cs_s = cs(s);
                    let r = (api.png_safecat)(
                        buf.as_mut_ptr() as *mut c_char,
                        bufsize.min(64),
                        pos.min(63),
                        cs_s.as_ptr(),
                    );
                    p!("safecat bs={} pos={} {:?} -> {}", bufsize, pos, s, r);
                    emit_bytes(&format!("safecat bs={} pos={} {:?}", bufsize, pos, s), &buf);
                }
            }
        }

        // png_format_number with every defined format over boundary + random.
        // PNG_NUMBER_FORMAT_*: 1=u/d, 2=02u/02d, 3=x, 4=02x, 5=fixed.
        let numbers: [u64; 14] = [
            0, 1, 9, 10, 15, 16, 99, 100, 255, 256, 65535, 100000, 4294967295,
            18446744073709551615,
        ];
        for fmt in [1i32, 2, 3, 4, 5, 0, 6, -1] {
            for &n in &numbers {
                // The C requires a buffer and writes backwards from `end`. Use a
                // generous 32-byte buffer; PNG_NUMBER_BUFFER_SIZE is 24.
                let mut buf = [0u8; 32];
                let end = buf.as_mut_ptr().add(buf.len()) as *mut c_char;
                let start = buf.as_ptr() as *const c_char;
                let ret = (api.png_format_number)(start, end, fmt, n);
                // ret points somewhere inside buf; convert to a string without
                // ever printing the pointer itself.
                let s = if ret.is_null() { "<null>".to_string() } else { cstr(ret) };
                p!("fmtnum f={} n={} -> {:?}", fmt, n, s);
            }
        }
        for _ in 0..300 {
            let fmt = [1i32, 2, 3, 4, 5][(rng.below(5)) as usize];
            let n = rng.next_u64();
            let mut buf = [0u8; 32];
            let end = buf.as_mut_ptr().add(buf.len()) as *mut c_char;
            let start = buf.as_ptr() as *const c_char;
            let ret = (api.png_format_number)(start, end, fmt, n);
            let s = if ret.is_null() { "<null>".to_string() } else { cstr(ret) };
            p!("fmtnum# f={} n={} -> {:?}", fmt, n, s);
        }
    }
}

/* ------------------------------------------------------------------ */
/* ll/check_keyword                                                    */
/* ------------------------------------------------------------------ */

fn check_keyword(api: &Api, rng: &mut Rng) {
    unsafe {
        let png = write_png(api);
        // new_key must be 80 bytes (79 + terminator); allocate slack.
        let mut keywords: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            vec![b'x'; 79],
            vec![b'y'; 80],
            b"  leading".to_vec(),
            b"trailing  ".to_vec(),
            b"double  space".to_vec(),
            b"tab\tinside".to_vec(),
            vec![0x01, b'a', 0x1f, b'b', 0x7f, b'c'],
            vec![0xa1, 0xff, b'h', b'i'],
            b"normal keyword".to_vec(),
            vec![b' '; 10],
        ];
        for _ in 0..40 {
            let n = rng.range(0, 90) as usize;
            let mut k = Vec::with_capacity(n);
            for _ in 0..n {
                k.push(rng.u8());
            }
            keywords.push(k);
        }
        for k in &keywords {
            let mut ck: Vec<c_char> = k.iter().map(|b| *b as c_char).collect();
            ck.push(0);
            let mut new_key = [0u8; 96]; // 80 needed; extra slack.
            let len = (api.png_check_keyword)(png, ck.as_ptr(), new_key.as_mut_ptr());
            p!("keyword {:?} -> len={}", String::from_utf8_lossy(k), len);
            emit_bytes("new_key", &new_key[..80]);
        }
        destroy_write(api, png);
    }
}

/* ------------------------------------------------------------------ */
/* ll/row_bgr_invert_swap                                              */
/* ------------------------------------------------------------------ */

fn row_bgr_invert_swap(api: &Api, rng: &mut Rng) {
    unsafe {
        let widths = [1u32, 2, 5, 8, 17];
        for &(color, depth) in A1.iter() {
            for &w in &widths {
                let rb = rowbytes_of(w, depth, color);
                // Four independent functions; give each its own fresh row so the
                // effect of one does not leak into the next comparison.
                for which in 0..4 {
                    let mut ri = row_info_for(w, color, depth);
                    let mut row = vec![0u8; rb + 8]; // slack
                    for b in row.iter_mut().take(rb) {
                        *b = rng.u8();
                    }
                    let name = match which {
                        0 => {
                            (api.png_do_bgr)(&mut ri, row.as_mut_ptr());
                            "bgr"
                        }
                        1 => {
                            (api.png_do_invert)(&mut ri, row.as_mut_ptr());
                            "invert"
                        }
                        2 => {
                            (api.png_do_swap)(&mut ri, row.as_mut_ptr());
                            "swap"
                        }
                        _ => {
                            (api.png_do_packswap)(&mut ri, row.as_mut_ptr());
                            "packswap"
                        }
                    };
                    emit_bytes(&format!("{} c={} d={} w={}", name, color, depth, w), &row[..rb]);
                }
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* ll/row_strip_channel                                                */
/* ------------------------------------------------------------------ */

fn row_strip_channel(api: &Api, rng: &mut Rng) {
    unsafe {
        // Only color types with >=2 channels: GA(4) and RGBA(6), at 8 and 16.
        let combos: [(u8, u8); 4] = [(4, 8), (4, 16), (6, 8), (6, 16)];
        let widths = [1u32, 2, 5, 8, 17];
        for &(color, depth) in &combos {
            for &w in &widths {
                for at_start in [0i32, 1] {
                    let rb = rowbytes_of(w, depth, color);
                    let mut ri = row_info_for(w, color, depth);
                    let mut row = vec![0u8; rb + 8];
                    for b in row.iter_mut().take(rb) {
                        *b = rng.u8();
                    }
                    (api.png_do_strip_channel)(&mut ri, row.as_mut_ptr(), at_start);
                    p!("strip c={} d={} w={} at={} -> rb={} ch={} pd={} ct={}",
                       color, depth, w, at_start, ri.rowbytes, ri.channels,
                       ri.pixel_depth, ri.color_type);
                    // rowbytes shrinks; emit only the (now smaller) valid part.
                    let nb = ri.rowbytes.min(row.len());
                    emit_bytes(&format!("strip c={} d={} w={} at={}", color, depth, w, at_start),
                               &row[..nb]);
                }
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* ll/row_read_interlace                                               */
/* ------------------------------------------------------------------ */

fn row_read_interlace(api: &Api, rng: &mut Rng) {
    unsafe {
        // png_pass_inc[pass] and png_pass_start[pass] from pngrutil.c.
        const PASS_START: [u32; 7] = [0, 4, 0, 2, 0, 1, 0];
        const PASS_INC: [u32; 7] = [8, 8, 4, 4, 2, 2, 1];

        // Full-image widths whose Adam7 sub-images span the interesting cases.
        let full_widths = [8u32, 9, 16, 17, 33];
        // A couple of representative transformation masks (PACKSWAP is the one
        // that actually changes byte layout for sub-8-bit depths).
        let transforms = [0u32, 0x0008 /*PNG_PACKSWAP*/, 0xffff_ffff];

        for &(color, depth) in A1.iter() {
            for &full_w in &full_widths {
                for pass in 0u32..7 {
                    // Sub-image width for this pass, exactly like libpng's caller.
                    let sub_w = if full_w > PASS_START[pass as usize] {
                        (full_w + PASS_INC[pass as usize] - 1 - PASS_START[pass as usize])
                            / PASS_INC[pass as usize]
                    } else {
                        0
                    };
                    if sub_w == 0 {
                        continue; // no pixels in this pass for this width
                    }
                    // final_width = sub_w * inc; the function expands into a row
                    // that must hold PNG_ROWBYTES(pixel_depth, final_width).
                    // Size the buffer for full_w + slack (>= any final_width).
                    let cap = rowbytes_of(full_w + 8, depth, color) + 16;
                    for &tr in &transforms {
                        let mut ri = row_info_for(sub_w, color, depth);
                        let mut row = vec![0u8; cap];
                        // Fill only the sub-image input region (the rest must be
                        // zero and identical in both libraries).
                        let in_rb = rowbytes_of(sub_w, depth, color);
                        for b in row.iter_mut().take(in_rb) {
                            *b = rng.u8();
                        }
                        (api.png_do_read_interlace)(&mut ri, row.as_mut_ptr(), pass as c_int, tr);
                        p!("rdintl c={} d={} fw={} pass={} sub={} tr={:#x} -> w={} rb={}",
                           color, depth, full_w, pass, sub_w, tr, ri.width, ri.rowbytes);
                        let out = ri.rowbytes.min(row.len());
                        emit_bytes(&format!("rdintl c={} d={} fw={} pass={} tr={:#x}",
                                            color, depth, full_w, pass, tr), &row[..out]);
                    }
                }
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* ll/row_write_interlace                                              */
/* ------------------------------------------------------------------ */

fn row_write_interlace(api: &Api, rng: &mut Rng) {
    unsafe {
        // Input is the FULL-width row; the function shrinks it in place.
        let full_widths = [8u32, 9, 16, 17, 33];
        for &(color, depth) in A1.iter() {
            for &full_w in &full_widths {
                for pass in 0u32..7 {
                    let rb = rowbytes_of(full_w, depth, color);
                    let mut ri = row_info_for(full_w, color, depth);
                    let mut row = vec![0u8; rb + 8];
                    for b in row.iter_mut().take(rb) {
                        *b = rng.u8();
                    }
                    (api.png_do_write_interlace)(&mut ri, row.as_mut_ptr(), pass as c_int);
                    p!("wrintl c={} d={} fw={} pass={} -> w={} rb={}",
                       color, depth, full_w, pass, ri.width, ri.rowbytes);
                    let out = ri.rowbytes.min(row.len());
                    emit_bytes(&format!("wrintl c={} d={} fw={} pass={}", color, depth, full_w, pass),
                               &row[..out]);
                }
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* ll/read_filter_row                                                  */
/* ------------------------------------------------------------------ */

fn read_filter_row(api: &Api, rng: &mut Rng) {
    unsafe {
        // A plain read struct is enough: png_read_filter_row lazily initialises
        // its read_filter[] table on first use.
        let png = read_png(api);
        let widths = [1u32, 2, 5, 8, 17];
        for &(color, depth) in A1.iter() {
            for &w in &widths {
                let rb = rowbytes_of(w, depth, color);
                // filters 0..=6 plus 255. 1..4 reconstruct; the rest are no-ops.
                for filter in [0i32, 1, 2, 3, 4, 5, 6, 255] {
                    let mut ri = row_info_for(w, color, depth);
                    let mut row = vec![0u8; rb + 8];
                    let mut prev = vec![0u8; rb + 8];
                    for b in row.iter_mut().take(rb) {
                        *b = rng.u8();
                    }
                    for b in prev.iter_mut().take(rb) {
                        *b = rng.u8();
                    }
                    (api.png_read_filter_row)(
                        png,
                        &mut ri,
                        row.as_mut_ptr(),
                        prev.as_ptr(),
                        filter,
                    );
                    emit_bytes(&format!("filt c={} d={} w={} f={}", color, depth, w, filter),
                               &row[..rb]);
                }
            }
        }
        destroy_read(api, png);
    }
}

/* ------------------------------------------------------------------ */
/* ll/check_ihdr_direct                                                */
/* ------------------------------------------------------------------ */

fn check_ihdr_direct(api: &Api, _rng: &mut Rng) {
    unsafe {
        // Registered with variants=14. Each index picks a DIFFERENT malformed
        // field combination; index 13 is a fully valid control (returns
        // normally). All the malformed ones warn per problem then png_error
        // "Invalid IHDR data", so they run one-per-process.
        //
        // (w, h, depth, color, interlace, compression, filter)
        let v = variant() as usize % 14;
        let params: [(u32, u32, c_int, c_int, c_int, c_int, c_int); 14] = [
            (0, 8, 8, 2, 0, 0, 0),               // 0: width == 0
            (0x8000_0000, 8, 8, 2, 0, 0, 0),      // 1: width > 2^31
            (2_000_000, 8, 8, 2, 0, 0, 0),        // 2: width > user limit (1000000)
            (8, 0, 8, 2, 0, 0, 0),                // 3: height == 0
            (8, 0x8000_0000, 8, 2, 0, 0, 0),      // 4: height > 2^31
            (8, 2_000_000, 8, 2, 0, 0, 0),        // 5: height > user limit
            (8, 8, 3, 2, 0, 0, 0),                // 6: bad bit depth (3)
            (8, 8, 8, 1, 0, 0, 0),                // 7: bad color type (1)
            (8, 8, 8, 5, 0, 0, 0),                // 8: bad color type (5)
            (8, 8, 16, 3, 0, 0, 0),               // 9: palette @16 (bad combo)
            (8, 8, 8, 2, 9, 0, 0),                // 10: bad interlace
            (8, 8, 8, 2, 0, 9, 0),                // 11: bad compression
            (8, 8, 8, 2, 0, 0, 9),                // 12: bad filter
            (8, 8, 8, 2, 0, 0, 0),                // 13: fully VALID control
        ];
        let (w, h, d, c, i, comp, f) = params[v];
        let png = read_png(api);
        p!("check_ihdr v={} w={} h={} d={} c={} i={} comp={} f={}",
           v, w, h, d, c, i, comp, f);
        (api.png_check_IHDR)(png, w, h, d, c, i, comp, f);
        // Only reached for the valid control (v==13); the others png_error.
        p!("check_ihdr v={} returned OK", v);
        destroy_read(api, png);
    }
}

/* ------------------------------------------------------------------ */
/* ll/chunk_unknown_handling                                           */
/* ------------------------------------------------------------------ */

fn chunk_unknown_handling(api: &Api, rng: &mut Rng) {
    unsafe {
        // Build the chunk name set (as big-endian u32, i.e. the 4 ASCII bytes).
        fn name(b: &[u8; 4]) -> u32 {
            u32::from_be_bytes(*b)
        }
        let names: [u32; 8] = [
            name(b"bKGD"), name(b"tEXt"), name(b"gAMA"), name(b"pHYs"),
            name(b"vpAg"), name(b"prVt"), name(b"XxXx"), name(b"IEND"),
        ];

        // Various png_set_keep_unknown_chunks settings, then query.
        for keep in [
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
        ] {
            let png = read_png(api);
            // Feed a NULL chunk list so keep applies to the default handling.
            (api.png_set_keep_unknown_chunks)(png, keep, null(), 0);
            // Also register a specific list.
            let mut list: Vec<u8> = Vec::new();
            list.extend_from_slice(b"vpAg");
            list.extend_from_slice(b"prVt");
            (api.png_set_keep_unknown_chunks)(png, keep, list.as_ptr(), 2);
            for &nm in &names {
                p!("cuh keep={} name={:08x} -> {}",
                   keep, nm, (api.png_chunk_unknown_handling)(png, nm));
            }
            destroy_read(api, png);
        }
        // A few random names against the ALWAYS setting.
        let png = read_png(api);
        (api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, null(), 0);
        for _ in 0..32 {
            let nm = rng.u32();
            p!("cuh# name={:08x} -> {}", nm, (api.png_chunk_unknown_handling)(png, nm));
        }
        destroy_read(api, png);
    }
}

/* ------------------------------------------------------------------ */
/* ll/user_version_check                                               */
/* ------------------------------------------------------------------ */

fn user_version_check(api: &Api, _rng: &mut Rng) {
    unsafe {
        // A fresh struct via create_png_struct so png_user_version_check runs
        // against a real png_ptr. Matching and mismatching version strings.
        let versions: [&[u8]; 8] = [
            b"1.6.59.git\0",
            b"1.6.59\0",
            b"1.6.0\0",
            b"1.5.0\0",
            b"1.7.0\0",
            b"0.0.0\0",
            b"\0",
            b"garbage\0",
        ];
        for v in &versions {
            let png = read_png(api);
            let r = (api.png_user_version_check)(png, v.as_ptr() as *const c_char);
            p!("user_version_check {:?} -> {}",
               String::from_utf8_lossy(&v[..v.len().saturating_sub(1)]), r);
            destroy_read(api, png);
        }
    }
}

/* ------------------------------------------------------------------ */
/* ll/zalloc                                                           */
/* ------------------------------------------------------------------ */

fn zalloc(api: &Api, _rng: &mut Rng) {
    unsafe {
        let png = read_png(api);
        // png_zalloc takes the png_ptr as its opaque; items * size bytes.
        for (items, size) in [
            (0u32, 0u32), (1, 0), (0, 1), (1, 1), (16, 16), (256, 4),
            (1, 0xffff_ffff), (0xffff_ffff, 1), (65536, 65536),
        ] {
            let q = (api.png_zalloc)(png as *mut c_void, items, size);
            p!("zalloc items={} size={} null={}", items, size, q.is_null());
            if !q.is_null() {
                (api.png_zfree)(png as *mut c_void, q);
            }
        }
        destroy_read(api, png);
    }
}

/* ------------------------------------------------------------------ */
/* ll/malloc_internals                                                 */
/* ------------------------------------------------------------------ */

fn malloc_internals(api: &Api, _rng: &mut Rng) {
    unsafe {
        let png = read_png(api);
        // png_malloc_base / _default with valid and impossible sizes.
        for size in [0u64, 1, 16, 4096, u64::MAX, u64::MAX / 2] {
            let a = (api.png_malloc_base)(png, size);
            p!("malloc_base {} null={}", size, a.is_null());
            if !a.is_null() {
                (api.png_free_default)(png, a);
            }
            let b = (api.png_malloc_default)(png, size);
            p!("malloc_default {} null={}", size, b.is_null());
            if !b.is_null() {
                (api.png_free_default)(png, b);
            }
        }
        // png_free_default(NULL) is a no-op.
        (api.png_free_default)(png, vnull());

        // png_malloc_array / _realloc_array. Do NOT pass nelements<=0 or
        // element_size==0 (those png_error). Use valid element sizes and both a
        // reasonable and an impossible count.
        for (nel, esz) in [(1i32, 1usize), (8, 16), (256, 32), (1, usize::MAX / 2)] {
            let a = (api.png_malloc_array)(png, nel, esz);
            p!("malloc_array nel={} esz={} null={}", nel, esz, a.is_null());
            if !a.is_null() {
                // Grow it with realloc_array (old_elements, add_elements).
                let grown = (api.png_realloc_array)(png, a, nel, 4, esz);
                p!("realloc_array nel={} add=4 esz={} null={}", nel, esz, grown.is_null());
                if !grown.is_null() {
                    (api.png_free)(png, grown);
                } else {
                    (api.png_free)(png, a);
                }
            }
        }
        destroy_read(api, png);
    }
}

/* ------------------------------------------------------------------ */
/* ll/set_text_2                                                       */
/* ------------------------------------------------------------------ */

fn set_text_2(api: &Api, _rng: &mut Rng) {
    unsafe {
        let png = write_png(api);
        let info = (api.png_create_info_struct)(png);

        let key = cs("Comment");
        let txt = cs("hello world");
        let good = png_text {
            compression: -1, // PNG_TEXT_COMPRESSION_NONE
            key: key.as_ptr() as *mut c_char,
            text: txt.as_ptr() as *mut c_char,
            text_length: 0,
            itxt_length: 0,
            lang: null(),
            lang_key: null(),
        };

        // Valid entries.
        let r = (api.png_set_text_2)(png, info, &good, 1);
        p!("set_text_2 valid num=1 -> {}", r);

        // num_text 0 is a no-op that returns 0.
        let r0 = (api.png_set_text_2)(png, info, &good, 0);
        p!("set_text_2 num=0 -> {}", r0);

        // Negative num_text: still returns without touching text_ptr.
        let rn = (api.png_set_text_2)(png, info, &good, -1);
        p!("set_text_2 num=-1 -> {}", rn);

        // NULL text_ptr with num=0 is safe.
        let rnull = (api.png_set_text_2)(png, info, null(), 0);
        p!("set_text_2 null num=0 -> {}", rnull);

        // Read back with png_get_text.
        let mut tp: *mut png_text = null();
        let mut n: c_int = 0;
        let got = (api.png_get_text)(png, info, &mut tp, &mut n);
        p!("get_text -> ret={} n={}", got, n);
        if !tp.is_null() && n > 0 {
            for i in 0..n as isize {
                let t = &*tp.offset(i);
                p!("  text[{}] comp={} key={:?} text={:?}",
                   i, t.compression, cstr(t.key), cstr(t.text));
            }
        }

        let mut pp = png;
        let mut ip = info;
        (api.png_destroy_write_struct)(&mut pp, &mut ip);
    }
}

/* ------------------------------------------------------------------ */
/* ll/warning_params                                                   */
/* ------------------------------------------------------------------ */

fn warning_params(api: &Api, _rng: &mut Rng) {
    unsafe {
        let png = write_png(api);
        // png_warning_parameters is char[8][32]; our flat [u8;256]. Element N
        // (1-based) lives at offset (N-1)*32.
        let mut p: png_warning_parameters = [0u8; 256];
        let base = p.as_mut_ptr() as *mut c_char;

        // Slot 1: a plain string.
        let s1 = cs("hello");
        (api.png_warning_parameter)(base, 1, s1.as_ptr());
        // Slot 2: a string longer than 32 bytes to exercise truncation.
        let s2 = cs("0123456789abcdefghijklmnopqrstuvwxyz-TOOLONG");
        (api.png_warning_parameter)(base, 2, s2.as_ptr());
        // Slot 3..5: unsigned in various formats. (1=u,2=02u,3=x,4=02x,5=fixed)
        (api.png_warning_parameter_unsigned)(base, 3, 1, 42);
        (api.png_warning_parameter_unsigned)(base, 4, 4, 255);
        (api.png_warning_parameter_unsigned)(base, 5, 5, 123456);
        // Slot 6..8: signed, including negatives and the fixed format.
        (api.png_warning_parameter_signed)(base, 6, 1, -17);
        (api.png_warning_parameter_signed)(base, 7, 5, -100000);
        (api.png_warning_parameter_signed)(base, 8, 2, 7);

        // png_formatted_warning consumes @1..@8; the emitted WARN text is the
        // comparison target (goes through warn_fn -> transcript).
        let msg = cs("params: @1 @2 @3 @4 @5 @6 @7 @8 end @9 @0 @@");
        (api.png_formatted_warning)(png, base, msg.as_ptr());

        // A second round with different formats/values.
        let mut p2: png_warning_parameters = [0u8; 256];
        let base2 = p2.as_mut_ptr() as *mut c_char;
        (api.png_warning_parameter_unsigned)(base2, 1, 2, 5);
        (api.png_warning_parameter_unsigned)(base2, 2, 3, 0xdead_beef);
        (api.png_warning_parameter_signed)(base2, 3, 1, i32::MIN + 1);
        let long = cs("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"); // 48 a's
        (api.png_warning_parameter)(base2, 4, long.as_ptr());
        let msg2 = cs("second @1/@2/@3 [@4]");
        (api.png_formatted_warning)(png, base2, msg2.as_ptr());

        destroy_write(api, png);
    }
}

/* ------------------------------------------------------------------ */
/* ll/app_error                                                        */
/* ------------------------------------------------------------------ */

fn app_error(api: &Api, _rng: &mut Rng) {
    unsafe {
        // Registered variants=6.
        //   0 = png_app_warning (non-fatal; loop several)
        //   1 = png_chunk_report level 0  (read struct -> chunk_warning)
        //   2 = png_chunk_report level 1  (read struct -> chunk_warning)
        //   3 = png_chunk_report level 2  (read struct -> chunk_benign_error ->
        //       png_error, FATAL because chunk_name==0)
        //   4 = png_app_error  (FATAL in this build)
        //   5 = png_fixed_error (FATAL)
        let v = variant() % 6;
        match v {
            0 => {
                let png = read_png(api);
                for i in 0..5 {
                    let m = cs(&format!("app_warning {}", i));
                    (api.png_app_warning)(png, m.as_ptr());
                }
                p!("app_warning done");
                destroy_read(api, png);
            }
            1 | 2 | 3 => {
                let level = (v - 1) as c_int;
                let png = read_png(api);
                let m = cs("chunk_report");
                p!("chunk_report level={}", level);
                (api.png_chunk_report)(png, m.as_ptr(), level);
                // Reached for levels 0 and 1 (warnings); level 2 is fatal.
                p!("chunk_report level={} returned", level);
                destroy_read(api, png);
            }
            4 => {
                let png = read_png(api);
                let m = cs("app_error fatal");
                (api.png_app_error)(png, m.as_ptr());
                p!("app_error returned (unexpected)");
                destroy_read(api, png);
            }
            _ => {
                let png = read_png(api);
                let m = cs("some_function");
                (api.png_fixed_error)(png, m.as_ptr());
                p!("fixed_error returned (unexpected)");
                destroy_read(api, png);
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* ll/filter_heuristics                                                */
/* ------------------------------------------------------------------ */

fn filter_heuristics(api: &Api, rng: &mut Rng) {
    unsafe {
        // png_set_filter_heuristics and _fixed on a write struct, then write a
        // small image and compare the bytes.
        let mut w = Writer::new(api);
        // heuristic_method / num_weights / weights / costs.
        let weights = [1.0f64, 1.5, 2.0];
        let costs = [1.0f64, 1.0, 1.0, 1.0, 1.0];
        (api.png_set_filter_heuristics)(w.png, 1, 3, weights.as_ptr(), costs.as_ptr());
        let wfix = [65536i32, 98304, 131072];
        let cfix = [65536i32, 65536, 65536, 65536, 65536];
        (api.png_set_filter_heuristics_fixed)(w.png, 1, 3, wfix.as_ptr(), cfix.as_ptr());
        // Also the degenerate 0-weight and unknown-method cases (non-fatal).
        (api.png_set_filter_heuristics)(w.png, 0, 0, null(), null());

        let (width, height) = (16u32, 16u32);
        (api.png_set_IHDR)(w.png, w.info, width, height, 8, PNG_COLOR_TYPE_RGB,
                           PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE,
                           PNG_FILTER_TYPE_BASE);
        (api.png_write_info)(w.png, w.info);
        let rb = rowbytes_of(width, 8, 2);
        for _ in 0..height {
            let row = rng.image_bytes(rb);
            (api.png_write_row)(w.png, row.as_ptr());
        }
        (api.png_write_end)(w.png, w.info);
        emit_bytes("fh_image", wbuf());
        w.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* ll/create_png_struct                                                */
/* ------------------------------------------------------------------ */

fn create_png_struct(api: &Api, _rng: &mut Rng) {
    unsafe {
        // Matching / mismatching version and with/without custom malloc/free.
        for v in [
            &b"1.6.59.git\0"[..],
            &b"1.6.0\0"[..],
            &b"1.5.0\0"[..],
            &b"0.0.0\0"[..],
            &b"\0"[..],
        ] {
            // Without custom allocators.
            let p1 = (api.png_create_png_struct)(
                v.as_ptr() as *const c_char, vnull(),
                Some(err_fn), Some(warn_fn), vnull(), None, None);
            p!("create_png_struct ver={:?} default null={}",
               String::from_utf8_lossy(&v[..v.len() - 1]), p1.is_null());
            if !p1.is_null() {
                (api.png_destroy_png_struct)(p1);
            }
            // With custom malloc/free (the transcript callbacks).
            let p2 = (api.png_create_png_struct)(
                v.as_ptr() as *const c_char, vnull(),
                Some(err_fn), Some(warn_fn), vnull(),
                Some(custom_malloc), Some(custom_free));
            p!("create_png_struct ver={:?} custom null={}",
               String::from_utf8_lossy(&v[..v.len() - 1]), p2.is_null());
            if !p2.is_null() {
                (api.png_destroy_png_struct)(p2);
            }
        }
    }
}

unsafe extern "C" fn custom_malloc(_p: png_structp, size: usize) -> *mut c_void {
    // A plain malloc via the global allocator, prefixed with the size so the
    // matching free can reconstruct the layout deterministically.
    let total = size + std::mem::size_of::<usize>();
    let layout = std::alloc::Layout::from_size_align(total, std::mem::align_of::<usize>()).unwrap();
    let raw = std::alloc::alloc(layout);
    if raw.is_null() {
        return std::ptr::null_mut();
    }
    *(raw as *mut usize) = size;
    raw.add(std::mem::size_of::<usize>()) as *mut c_void
}

unsafe extern "C" fn custom_free(_p: png_structp, ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let raw = (ptr as *mut u8).sub(std::mem::size_of::<usize>());
    let size = *(raw as *mut usize);
    let total = size + std::mem::size_of::<usize>();
    let layout = std::alloc::Layout::from_size_align(total, std::mem::align_of::<usize>()).unwrap();
    std::alloc::dealloc(raw, layout);
}

/* ------------------------------------------------------------------ */
/* ll/io_direct                                                        */
/* ------------------------------------------------------------------ */

fn io_direct(api: &Api, rng: &mut Rng) {
    unsafe {
        // Write side: our write_fn is installed by Writer::new, so png_write_data
        // funnels into wbuf() and png_flush emits "FLUSH".
        {
            let w = Writer::new(api);
            let bufs = [
                rng.bytes(1),
                rng.bytes(7),
                rng.bytes(64),
                rng.bytes(0),
                rng.bytes(200),
            ];
            for b in &bufs {
                (api.png_write_data)(w.png, b.as_ptr(), b.len());
            }
            (api.png_flush)(w.png);
            emit_bytes("io_write", wbuf());
            let mut pp = w.png;
            let mut ip = w.info;
            (api.png_destroy_write_struct)(&mut pp, &mut ip);
        }

        // Read side: install our read_fn over a known buffer, then pull it back
        // in several lengths. Only request within the buffer (read_fn is fatal
        // on EOF).
        {
            let src = rng.bytes(256);
            set_rbuf(src.clone());
            let png = read_png(api);
            (api.png_set_read_fn)(png, vnull(), Some(read_fn));
            let mut got = Vec::new();
            for len in [1usize, 3, 8, 64, 100, 80] {
                let mut tmp = vec![0u8; len];
                (api.png_read_data)(png, tmp.as_mut_ptr(), len);
                got.extend_from_slice(&tmp);
            }
            emit_bytes("io_read", &got);
            destroy_read(api, png);
        }

        // png_default_write_data / _read_data / _flush over a real FILE*.
        // We declare the tiny bit of libc we need ourselves (no libc crate).
        default_io_via_file(api, rng);
    }
}

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn fread(ptr: *mut c_void, size: usize, n: usize, f: *mut c_void) -> usize;
    fn rewind(f: *mut c_void);
}

fn default_io_via_file(api: &Api, rng: &mut Rng) {
    unsafe {
        // A per-process temp file; the path is never printed so it stays out of
        // the transcript. Both workers write the same bytes and read them back.
        let mut path = std::env::temp_dir();
        path.push(format!("ll_io_direct_{}.bin", std::process::id()));
        let cpath = {
            let s = path.to_string_lossy();
            let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
            v.push(0);
            v
        };
        let mode_w = cs("wb+");
        let f = fopen(cpath.as_ptr(), mode_w.as_ptr());
        if f.is_null() {
            emit("default_io: fopen failed (skipped)");
            return;
        }
        let png = write_png(api);
        // png_init_io installs png_default_write_data / _flush and sets io_ptr.
        (api.png_init_io)(png, f);
        let payload = rng.bytes(128);
        (api.png_write_data)(png, payload.as_ptr(), payload.len());
        (api.png_default_write_data)(png, payload.as_ptr() as *mut u8, payload.len());
        (api.png_default_flush)(png);
        fflush(f);
        // Read the file back through png_default_read_data on a read struct that
        // shares the same FILE*.
        rewind(f);
        let rpng = read_png(api);
        (api.png_init_io)(rpng, f);
        let mut back = vec![0u8; payload.len() * 2];
        (api.png_default_read_data)(rpng, back.as_mut_ptr(), back.len());
        emit_bytes("default_io_file", &back);
        destroy_write(api, png);
        destroy_read(api, rpng);
        fclose(f);
        let _ = std::fs::remove_file(&path);
    }
}

/* ------------------------------------------------------------------ */
/* ll/crc_direct                                                       */
/* ------------------------------------------------------------------ */

fn crc_direct(api: &Api, rng: &mut Rng) {
    unsafe {
        // Order (from pngwutil.c png_write_chunk_header): reset_crc, then
        // calculate_crc over the name and data as they are written, and
        // chunk_end emits the accumulated CRC. We drive the pieces manually so
        // the CRC actually lands in wbuf() and is compared.
        let w = Writer::new(api);

        for _ in 0..6 {
            let name = *b"tEXt";
            let dlen = rng.range(0, 40) as usize;
            let data = rng.bytes(dlen);

            // Mirror png_write_chunk_start: it internally does reset_crc +
            // calculate_crc(name). Then chunk_data does calculate_crc(data), and
            // chunk_end writes the CRC.
            (api.png_write_chunk_start)(w.png, name.as_ptr(), data.len() as u32);
            (api.png_write_chunk_data)(w.png, data.as_ptr(), data.len());
            (api.png_write_chunk_end)(w.png);
        }

        // Also exercise reset_crc + calculate_crc directly, then start/end so
        // the manually accumulated CRC is emitted.
        (api.png_reset_crc)(w.png);
        let blob = rng.bytes(50);
        (api.png_calculate_crc)(w.png, blob.as_ptr(), blob.len());
        // Now a full chunk whose data matches, so both paths are visible.
        let nm = *b"zTXt";
        (api.png_write_chunk_start)(w.png, nm.as_ptr(), blob.len() as u32);
        (api.png_write_chunk_data)(w.png, blob.as_ptr(), blob.len());
        (api.png_write_chunk_end)(w.png);

        emit_bytes("crc_stream", wbuf());
        let mut pp = w.png;
        let mut ip = w.info;
        (api.png_destroy_write_struct)(&mut pp, &mut ip);
    }
}

/* ------------------------------------------------------------------ */
/* ll/write_chunks_direct                                              */
/* ------------------------------------------------------------------ */

fn write_chunks_direct(api: &Api, rng: &mut Rng) {
    unsafe {
        let w = Writer::new(api);
        // A palette image so PLTE sets num_palette (needed by hIST/bKGD/tRNS),
        // bit depth 8 so sBIT's maxbits check passes.
        let width = 8u32;
        let height = 4u32;
        let depth = 8i32;
        let color = PNG_COLOR_TYPE_PALETTE;

        // Emit IHDR first (sets channels/bit_depth/usr_bit_depth/mode).
        (api.png_write_IHDR)(w.png, width, height, depth, color,
                             PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                             PNG_INTERLACE_NONE);

        // PLTE: 256 entries (max for 8-bit palette). Sets num_palette.
        let pal: Vec<png_color> = (0..256)
            .map(|_| png_color { red: rng.u8(), green: rng.u8(), blue: rng.u8() })
            .collect();
        (api.png_write_PLTE)(w.png, pal.as_ptr(), 256);

        // gAMA / cHRM / sRGB / sBIT.
        (api.png_write_gAMA_fixed)(w.png, 45455);
        let chrm = png_xy {
            redx: 64000, redy: 33000, greenx: 30000, greeny: 60000,
            bluex: 15000, bluey: 6000, whitex: 31270, whitey: 32900,
        };
        (api.png_write_cHRM_fixed)(w.png, &chrm);
        (api.png_write_sRGB)(w.png, 0);
        let sbit = png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
        (api.png_write_sBIT)(w.png, &sbit, color);

        // tRNS for a palette image: num_trans <= num_palette bytes of alpha.
        let trns: [u8; 4] = [0, 128, 200, 255];
        (api.png_write_tRNS)(w.png, trns.as_ptr(), null(), 4, color);

        // bKGD (palette index within num_palette).
        let bkgd = png_color_16 { index: 3, red: 0, green: 0, blue: 0, gray: 0 };
        (api.png_write_bKGD)(w.png, &bkgd, color);

        // hIST: one entry per palette entry (num_hist <= num_palette).
        let hist: Vec<u16> = (0..256).map(|_| rng.u32() as u16).collect();
        (api.png_write_hIST)(w.png, hist.as_ptr(), 256);

        // pHYs / oFFs / tIME.
        (api.png_write_pHYs)(w.png, 2835, 2835, 1);
        (api.png_write_oFFs)(w.png, 10, -20, 0);
        let time = png_time { year: 2020, month: 6, day: 15, hour: 12, minute: 30, second: 45 };
        (api.png_write_tIME)(w.png, &time);

        // tEXt / zTXt / iTXt.
        let key = cs("Comment");
        let text = cs("A short text value.");
        (api.png_write_tEXt)(w.png, key.as_ptr(), text.as_ptr(), 0);
        // PNG_TEXT_COMPRESSION_zTXt == 0 (png.h:587). Passing 1 is not a valid
        // zTXt compression value and makes the emitter png_error immediately,
        // which would abort the case before the remaining chunks are written.
        (api.png_write_zTXt)(w.png, key.as_ptr(), text.as_ptr(), 0);
        let lang = cs("en");
        let lang_key = cs("Comment");
        (api.png_write_iTXt)(w.png, 1 /* PNG_ITXT_COMPRESSION_NONE */, key.as_ptr(), lang.as_ptr(), lang_key.as_ptr(), text.as_ptr());

        // sPLT: depth 8, a couple of entries.
        let entries = [
            png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 },
            png_sPLT_entry { red: 10, green: 20, blue: 30, alpha: 40, frequency: 50 },
        ];
        let splt_name = cs("splt-name");
        let mut splt = png_sPLT_t {
            name: splt_name.as_ptr() as *mut c_char,
            depth: 8,
            entries: entries.as_ptr() as *mut png_sPLT_entry,
            nentries: entries.len() as i32,
        };
        (api.png_write_sPLT)(w.png, &splt);
        let _ = &mut splt;

        // pCAL: purpose, X0, X1, type, nparams, units, params.
        let purpose = cs("pcal-purpose");
        let units = cs("meters");
        let p0 = cs("1.0");
        let p1 = cs("2.0");
        let mut params: [*mut c_char; 2] =
            [p0.as_ptr() as *mut c_char, p1.as_ptr() as *mut c_char];
        (api.png_write_pCAL)(w.png, purpose.as_ptr() as *mut c_char, 0, 100, 0, 2,
                             units.as_ptr(), params.as_mut_ptr());

        // sCAL_s: unit + width/height ascii strings.
        let scw = cs("1.5");
        let sch = cs("2.5");
        (api.png_write_sCAL_s)(w.png, 1, scw.as_ptr(), sch.as_ptr());

        // iCCP: profile with its length in the first 4 bytes (big-endian) and
        // byte[8] <= 3 so the multiple-of-4 check is skipped. Must be >= 132.
        let mut profile = vec![0u8; 132];
        let plen = profile.len() as u32;
        profile[0..4].copy_from_slice(&plen.to_be_bytes());
        profile[8] = 2; // version major, keeps temp <= 3
        let iccp_name = cs("an icc profile");
        // png_write_iCCP(png_ptr, name, profile, proflen): the length is BOTH a
        // parameter and embedded in the first 4 bytes, and the two must agree
        // (pngwutil.c: "Incorrect data in iCCP" / "Profile length does not match
        // profile").
        (api.png_write_iCCP)(w.png, iccp_name.as_ptr(), profile.as_ptr(), plen);

        // eXIf: a small exif blob.
        let mut exif = b"MM\0\x2a".to_vec();
        exif.extend_from_slice(&[0u8; 8]);
        (api.png_write_eXIf)(w.png, exif.as_mut_ptr(), exif.len() as c_int);

        // cICP / cLLI_fixed / mDCV_fixed.
        (api.png_write_cICP)(w.png, 1, 13, 0, 1);
        (api.png_write_cLLI_fixed)(w.png, 10000000, 5000000);
        // png_write_mDCV_fixed takes eight png_uint_16 chromaticity components
        // (already ITU-scaled), not a png_xy.
        (api.png_write_mDCV_fixed)(
            w.png, 34000, 16000, 13250, 34500, 7500, 3000, 15635, 16450,
            10000000, 1,
        );

        // IEND closes the stream.
        (api.png_write_IEND)(w.png);

        emit_bytes("chunks_direct", wbuf());

        let mut pp = w.png;
        let mut ip = w.info;
        (api.png_destroy_write_struct)(&mut pp, &mut ip);
    }
}

/* ------------------------------------------------------------------ */
/* ll/error_dispatch                                                   */
/* ------------------------------------------------------------------ */

fn error_dispatch(api: &Api, _rng: &mut Rng) {
    unsafe {
        // Registered variants=8. Fatal dispatchers each get their own process.
        //   0 = png_warning (read)          non-fatal
        //   1 = png_chunk_warning (read)    non-fatal
        //   2 = png_benign_error, benign=1  (read) -> warning (non-fatal)
        //   3 = png_benign_error, benign=0  (read) -> chunk_error/png_error FATAL
        //   4 = png_chunk_benign_error, benign=1 (read) -> chunk_warning
        //   5 = png_chunk_benign_error, benign=0 (read) -> chunk_error FATAL
        //   6 = png_error (write)           FATAL
        //   7 = png_chunk_warning (write)   non-fatal (write allows it)
        let v = variant() % 8;
        let m = cs("dispatch message");
        match v {
            0 => {
                let png = read_png(api);
                (api.png_warning)(png, m.as_ptr());
                p!("warning returned");
                destroy_read(api, png);
            }
            1 => {
                let png = read_png(api);
                (api.png_chunk_warning)(png, m.as_ptr());
                p!("chunk_warning returned");
                destroy_read(api, png);
            }
            2 => {
                let png = read_png(api);
                (api.png_set_benign_errors)(png, 1); // warn
                (api.png_benign_error)(png, m.as_ptr());
                p!("benign_error(warn) returned");
                destroy_read(api, png);
            }
            3 => {
                let png = read_png(api);
                (api.png_set_benign_errors)(png, 0); // error
                (api.png_benign_error)(png, m.as_ptr());
                p!("benign_error(error) returned (unexpected)");
                destroy_read(api, png);
            }
            4 => {
                let png = read_png(api);
                (api.png_set_benign_errors)(png, 1);
                (api.png_chunk_benign_error)(png, m.as_ptr());
                p!("chunk_benign_error(warn) returned");
                destroy_read(api, png);
            }
            5 => {
                let png = read_png(api);
                (api.png_set_benign_errors)(png, 0);
                (api.png_chunk_benign_error)(png, m.as_ptr());
                p!("chunk_benign_error(error) returned (unexpected)");
                destroy_read(api, png);
            }
            6 => {
                let png = write_png(api);
                (api.png_error)(png, m.as_ptr());
                p!("error returned (unexpected)");
                destroy_write(api, png);
            }
            _ => {
                let png = write_png(api);
                (api.png_chunk_warning)(png, m.as_ptr());
                p!("chunk_warning(write) returned");
                destroy_write(api, png);
            }
        }
    }
}
