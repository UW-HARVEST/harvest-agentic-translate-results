//! Error-path scenarios: one per row of `ERRORS.md`.
//!
//! Each of these constructs an exact invalid input / invalid call sequence and
//! records whatever the library does with it — the message text of the
//! error/warning, the return values and the process exit status.  A `png_error`
//! ends the child with exit code 70 after recording `ERROR <message>`, so the
//! parent sees both the sentinel *and* the exact message.

use super::mkpng::{self};
use super::pngdefs::*;
use super::scen::{cs, new_read, new_write, synth, ver, Args};
use super::{api, cb_error, cb_warn, cb_write, g, rec};
use std::ffi::{c_char, c_int, c_void};
#[allow(unused_imports)]
use std::ffi::c_double;

/* ------------------------------------------------------------------ */
/* helpers                                                             */
/* ------------------------------------------------------------------ */

/// Read a (possibly malformed) datastream end to end, recording everything.
unsafe fn read_stream(data: &[u8], benign: Option<i32>, tune: &dyn Fn(PngPtr)) {
    let api = api();
    let r = rec();
    r.digest("src", data);
    let (png, info, end) = new_read(data);
    if let Some(b) = benign {
        (api.png_set_benign_errors)(png, b);
    }
    tune(png);
    (api.png_read_info)(png, info);
    r.line("read_info done");
    let mut w = 0u32;
    let mut h = 0u32;
    let (mut bd, mut ct, mut il, mut cm, mut fm) = (0i32, 0i32, 0i32, 0i32, 0i32);
    let got = (api.png_get_IHDR)(png, info, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm);
    r.kv("ihdr", format!("{got} {w} {h} {bd} {ct} {il} {cm} {fm}"));
    let passes = if il == 1 { (api.png_set_interlace_handling)(png) } else { 1 };
    (api.png_read_update_info)(png, info);
    let rb = (api.png_get_rowbytes)(png, info);
    r.kv("rowbytes", rb);
    let hh = (api.png_get_image_height)(png, info);
    if hh > 0 && hh < 100_000 && rb > 0 && rb < 1 << 22 {
        let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0u8; rb]).collect();
        let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
        let _ = passes;
        (api.png_read_image)(png, rp.as_mut_ptr());
        let flat: Vec<u8> = rows.iter().flat_map(|v| v.clone()).collect();
        r.digest("rows", &flat);
    } else {
        r.line("rows skipped");
    }
    (api.png_read_end)(png, end);
    r.line("read_end done");
    let mut txt: *mut png_text = std::ptr::null_mut();
    let mut tn = 0i32;
    r.kv("text", format!("{} {tn}", (api.png_get_text)(png, end, &mut txt, &mut tn)));
    let mut p = png;
    let mut ip = info;
    let mut ep = end;
    (api.png_destroy_read_struct)(&mut p, &mut ip, &mut ep);
    r.line("destroyed");
}

fn nop(_p: PngPtr) {}

/// A minimal well-formed 4x3 RGB image, used as the base for corruption.
fn base(ct: u8, bd: u8) -> Vec<u8> {
    synth(ct, bd, 0, 4, 3, "none", 0, 99).png
}

/// Build a stream: IHDR with the given raw field values, then a valid-looking
/// IDAT for a 4x3 RGB image (contents irrelevant when IHDR is rejected).
fn ihdr_case(w: u32, h: u32, bd: u8, ct: u8, comp: u8, filt: u8, il: u8) -> Vec<u8> {
    let mut v = mkpng::SIG.to_vec();
    v.extend_from_slice(&mkpng::ihdr(w, h, bd, ct, comp, filt, il));
    if ct == 3 {
        v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 3 * 4]));
    }
    let raw = mkpng::filtered_none(&vec![vec![0u8; mkpng::rowbytes(4, 8, 2)]; 3]);
    v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
    v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
    v
}

/// Base stream with one extra chunk inserted before IDAT.  If the injected
/// chunk is itself a PLTE, the automatic palette is omitted so the injected one
/// is the only PLTE in the stream.
fn with_chunk(ct: u8, bd: u8, name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = mkpng::SIG.to_vec();
    v.extend_from_slice(&mkpng::ihdr(4, 3, bd, ct, 0, 0, 0));
    if ct == 3 && name != b"PLTE" {
        v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 3 * 4]));
    }
    v.extend_from_slice(&mkpng::chunk(name, data));
    let raw = mkpng::filtered_none(&vec![vec![0u8; mkpng::rowbytes(4, bd, ct)]; 3]);
    v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
    v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
    v
}

/// Base stream with one extra chunk inserted after IDAT.
fn with_chunk_tail(ct: u8, bd: u8, name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = mkpng::SIG.to_vec();
    v.extend_from_slice(&mkpng::ihdr(4, 3, bd, ct, 0, 0, 0));
    if ct == 3 {
        v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 3 * 4]));
    }
    let raw = mkpng::filtered_none(&vec![vec![0u8; mkpng::rowbytes(4, bd, ct)]; 3]);
    v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
    v.extend_from_slice(&mkpng::chunk(name, data));
    v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
    v
}

/* ------------------------------------------------------------------ */
/* dispatcher                                                          */
/* ------------------------------------------------------------------ */

pub fn run_err(a: &Args) {
    let id = a.s("id", "");
    if stream_cases(&id) {
        return;
    }
    if api_cases(&id) {
        return;
    }
    if simple_cases(&id) {
        return;
    }
    panic!("unknown error scenario id={id}");
}

/* ------------------------------------------------------------------ */
/* malformed datastreams                                               */
/* ------------------------------------------------------------------ */

fn stream_cases(id: &str) -> bool {
    let r = rec();
    unsafe {
        let stream: Vec<u8> = match id {
            /* --- signature --- */
            "sig_empty" => Vec::new(),
            "sig_short" => mkpng::SIG[..4].to_vec(),
            "sig_bad" => {
                let mut v = mkpng::SIG.to_vec();
                v[1] = b'Q';
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v
            }
            "sig_only" => mkpng::SIG.to_vec(),
            "sig_jpeg" => vec![0xff, 0xd8, 0xff, 0xe0, 0, 0x10, b'J', b'F'],

            /* --- IHDR --- */
            "no_ihdr" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "ihdr_first_is_gama" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::chunk(b"gAMA", &45455u32.to_be_bytes()));
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "ihdr_badlen_12" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::chunk(b"IHDR", &[0u8; 12]));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "ihdr_badlen_0" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::chunk(b"IHDR", &[]));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "ihdr_w0" => ihdr_case(0, 3, 8, 2, 0, 0, 0),
            "ihdr_h0" => ihdr_case(4, 0, 8, 2, 0, 0, 0),
            "ihdr_w_msb" => ihdr_case(0x8000_0000, 3, 8, 2, 0, 0, 0),
            "ihdr_h_msb" => ihdr_case(4, 0x8000_0000, 8, 2, 0, 0, 0),
            "ihdr_w_max31" => ihdr_case(0x7fff_ffff, 3, 8, 2, 0, 0, 0),
            "ihdr_bd_0" => ihdr_case(4, 3, 0, 2, 0, 0, 0),
            "ihdr_bd_3" => ihdr_case(4, 3, 3, 2, 0, 0, 0),
            "ihdr_bd_32" => ihdr_case(4, 3, 32, 2, 0, 0, 0),
            "ihdr_bd_255" => ihdr_case(4, 3, 255, 2, 0, 0, 0),
            "ihdr_ct_1" => ihdr_case(4, 3, 8, 1, 0, 0, 0),
            "ihdr_ct_5" => ihdr_case(4, 3, 8, 5, 0, 0, 0),
            "ihdr_ct_7" => ihdr_case(4, 3, 8, 7, 0, 0, 0),
            "ihdr_ct_255" => ihdr_case(4, 3, 8, 255, 0, 0, 0),
            "ihdr_pal_bd16" => ihdr_case(4, 3, 16, 3, 0, 0, 0),
            "ihdr_rgb_bd1" => ihdr_case(4, 3, 1, 2, 0, 0, 0),
            "ihdr_ga_bd4" => ihdr_case(4, 3, 4, 4, 0, 0, 0),
            "ihdr_comp_1" => ihdr_case(4, 3, 8, 2, 1, 0, 0),
            "ihdr_comp_255" => ihdr_case(4, 3, 8, 2, 255, 0, 0),
            "ihdr_filt_1" => ihdr_case(4, 3, 8, 2, 0, 1, 0),
            "ihdr_filt_64" => ihdr_case(4, 3, 8, 2, 0, 64, 0),
            "ihdr_il_2" => ihdr_case(4, 3, 8, 2, 0, 0, 2),
            "ihdr_il_255" => ihdr_case(4, 3, 8, 2, 0, 0, 255),
            "ihdr_dup" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }

            /* --- chunk framing --- */
            "crc_ihdr" => {
                let mut v = mkpng::SIG.to_vec();
                let mut d = Vec::new();
                d.extend_from_slice(&4u32.to_be_bytes());
                d.extend_from_slice(&3u32.to_be_bytes());
                d.extend_from_slice(&[8, 2, 0, 0, 0]);
                v.extend_from_slice(&mkpng::chunk_bad_crc(b"IHDR", &d));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "crc_gama" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk_bad_crc(b"gAMA", &45455u32.to_be_bytes()));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "crc_idat" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 3]);
                v.extend_from_slice(&mkpng::chunk_bad_crc(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "chunk_len_msb" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&0x8000_0000u32.to_be_bytes());
                v.extend_from_slice(b"gAMA");
                v.extend_from_slice(&[0u8; 8]);
                v
            }
            "chunk_name_digits" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"1234", &[1, 2, 3]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "chunk_name_space" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"a b\0", &[1, 2, 3]));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "unknown_critical" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"ABCD", &[1, 2, 3]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "truncated_after_ihdr" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v
            }
            "truncated_mid_chunk" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let c = mkpng::chunk(b"gAMA", &45455u32.to_be_bytes());
                v.extend_from_slice(&c[..c.len() - 3]);
                v
            }
            "no_iend" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v
            }
            "iend_nonzero_len" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[1, 2, 3]));
                v
            }

            /* --- IDAT --- */
            "no_idat" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "idat_empty" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &[]));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "idat_short" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 1]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "idat_long" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 9]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "idat_garbage" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &[0xde, 0xad, 0xbe, 0xef, 0x11, 0x22]));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "idat_bad_zlib_header" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 3]);
                let mut z = mkpng::zlib_stored(&raw);
                z[0] = 0x99;
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &z));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "idat_bad_adler" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 3]);
                let mut z = mkpng::zlib_stored(&raw);
                let n = z.len();
                z[n - 1] ^= 0xff;
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &z));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "idat_bad_filter" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let mut raw = Vec::new();
                for _ in 0..3 {
                    raw.push(5u8);
                    raw.extend_from_slice(&[0u8; 12]);
                }
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "idat_filter_64" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let mut raw = Vec::new();
                for _ in 0..3 {
                    raw.push(64u8);
                    raw.extend_from_slice(&[0u8; 12]);
                }
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "idat_split_noncontig" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 3]);
                let z = mkpng::zlib_stored(&raw);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &z[..4]));
                v.extend_from_slice(&mkpng::chunk(b"gAMA", &45455u32.to_be_bytes()));
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &z[4..]));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "idat_before_plte" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 12]));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }

            /* --- PLTE --- */
            "plte_missing" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "plte_len_not_mult3" => with_chunk(3, 8, b"PLTE", &[1, 2, 3, 4]),
            "plte_len_0" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "plte_too_many" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &vec![0u8; 3 * 300]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "plte_too_many_for_depth" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 2, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &vec![0u8; 3 * 200]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 1]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "plte_on_gray" => with_chunk(0, 8, b"PLTE", &[0u8; 12]),
            "plte_dup" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 12]));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 12]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "plte_after_idat" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 12]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 12]));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "pal_index_oob" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 3 * 2]));
                let raw = mkpng::filtered_none(&vec![vec![200u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }

            /* --- ancillary chunk content --- */
            "gama_len0" => with_chunk(2, 8, b"gAMA", &[]),
            "gama_len3" => with_chunk(2, 8, b"gAMA", &[0, 0, 1]),
            "gama_zero" => with_chunk(2, 8, b"gAMA", &0u32.to_be_bytes()),
            "gama_huge" => with_chunk(2, 8, b"gAMA", &0xffff_ffffu32.to_be_bytes()),
            "gama_dup" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 2, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"gAMA", &45455u32.to_be_bytes()));
                v.extend_from_slice(&mkpng::chunk(b"gAMA", &50000u32.to_be_bytes()));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 12]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "gama_after_idat" => with_chunk_tail(2, 8, b"gAMA", &45455u32.to_be_bytes()),
            "chrm_len0" => with_chunk(2, 8, b"cHRM", &[]),
            "chrm_len31" => with_chunk(2, 8, b"cHRM", &[0u8; 31]),
            "chrm_zero" => with_chunk(2, 8, b"cHRM", &[0u8; 32]),
            "srgb_len0" => with_chunk(2, 8, b"sRGB", &[]),
            "srgb_len2" => with_chunk(2, 8, b"sRGB", &[0, 0]),
            "srgb_intent4" => with_chunk(2, 8, b"sRGB", &[4]),
            "srgb_intent255" => with_chunk(2, 8, b"sRGB", &[255]),
            "sbit_len0" => with_chunk(2, 8, b"sBIT", &[]),
            "sbit_len1_rgb" => with_chunk(2, 8, b"sBIT", &[8]),
            "sbit_zero" => with_chunk(2, 8, b"sBIT", &[0, 0, 0]),
            "sbit_too_deep" => with_chunk(2, 8, b"sBIT", &[9, 9, 9]),
            "trns_gray_len1" => with_chunk(0, 8, b"tRNS", &[1]),
            "trns_rgb_len4" => with_chunk(2, 8, b"tRNS", &[0, 1, 0, 2]),
            "trns_on_rgba" => with_chunk(6, 8, b"tRNS", &[0, 1, 0, 2, 0, 3]),
            "trns_on_ga" => with_chunk(4, 8, b"tRNS", &[0, 1]),
            "trns_pal_too_many" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 3 * 4]));
                v.extend_from_slice(&mkpng::chunk(b"tRNS", &[0u8; 10]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "trns_before_plte" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"tRNS", &[0u8; 2]));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 3 * 4]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "bkgd_len0" => with_chunk(2, 8, b"bKGD", &[]),
            "bkgd_gray_len1" => with_chunk(0, 8, b"bKGD", &[1]),
            "bkgd_rgb_len4" => with_chunk(2, 8, b"bKGD", &[0, 1, 0, 2]),
            "bkgd_pal_oob" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 3 * 4]));
                v.extend_from_slice(&mkpng::chunk(b"bKGD", &[99]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "bkgd_gray_oob" => with_chunk(0, 2, b"bKGD", &99u16.to_be_bytes()),
            "hist_no_plte" => with_chunk(2, 8, b"hIST", &[0, 1, 0, 2]),
            "hist_wrong_len" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[0u8; 3 * 4]));
                v.extend_from_slice(&mkpng::chunk(b"hIST", &[0u8; 6]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                v
            }
            "phys_len0" => with_chunk(2, 8, b"pHYs", &[]),
            "phys_len8" => with_chunk(2, 8, b"pHYs", &[0u8; 8]),
            "phys_bad_unit" => {
                let mut d = Vec::new();
                d.extend_from_slice(&100u32.to_be_bytes());
                d.extend_from_slice(&100u32.to_be_bytes());
                d.push(7);
                with_chunk(2, 8, b"pHYs", &d)
            }
            "offs_len0" => with_chunk(2, 8, b"oFFs", &[]),
            "offs_bad_unit" => {
                let mut d = Vec::new();
                d.extend_from_slice(&1i32.to_be_bytes());
                d.extend_from_slice(&2i32.to_be_bytes());
                d.push(9);
                with_chunk(2, 8, b"oFFs", &d)
            }
            "scal_len0" => with_chunk(2, 8, b"sCAL", &[]),
            "scal_len3" => with_chunk(2, 8, b"sCAL", &[1, b'1', 0]),
            "scal_bad_unit" => with_chunk(2, 8, b"sCAL", b"\x005.0\0006.0"),
            "scal_neg_width" => with_chunk(2, 8, b"sCAL", b"\x01-5.0\0006.0"),
            "scal_bad_format" => with_chunk(2, 8, b"sCAL", b"\x01abc\000def"),
            "scal_no_null" => with_chunk(2, 8, b"sCAL", b"\x011.02.0"),
            "scal_zero_height" => with_chunk(2, 8, b"sCAL", b"\x011.0\0000.0"),
            "pcal_len0" => with_chunk(2, 8, b"pCAL", &[]),
            "pcal_bad_eq" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"p\0");
                d.extend_from_slice(&0i32.to_be_bytes());
                d.extend_from_slice(&255i32.to_be_bytes());
                d.push(9);
                d.push(0);
                d.extend_from_slice(b"u\0");
                with_chunk(2, 8, b"pCAL", &d)
            }
            "pcal_bad_nparams" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"p\0");
                d.extend_from_slice(&0i32.to_be_bytes());
                d.extend_from_slice(&255i32.to_be_bytes());
                d.push(0);
                d.push(3);
                d.extend_from_slice(b"u\0");
                d.extend_from_slice(b"1.0\0");
                with_chunk(2, 8, b"pCAL", &d)
            }
            "pcal_x0_eq_x1" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"p\0");
                d.extend_from_slice(&5i32.to_be_bytes());
                d.extend_from_slice(&5i32.to_be_bytes());
                d.push(0);
                d.push(0);
                d.extend_from_slice(b"u\0");
                with_chunk(2, 8, b"pCAL", &d)
            }
            "splt_len0" => with_chunk(2, 8, b"sPLT", &[]),
            "splt_bad_depth" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"n\0");
                d.push(7);
                d.extend_from_slice(&[0u8; 6]);
                with_chunk(2, 8, b"sPLT", &d)
            }
            "splt_bad_entrylen" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"n\0");
                d.push(8);
                d.extend_from_slice(&[0u8; 5]);
                with_chunk(2, 8, b"sPLT", &d)
            }
            "splt_no_null" => with_chunk(2, 8, b"sPLT", b"name-without-null"),
            "text_no_null" => with_chunk(2, 8, b"tEXt", b"KeyWithoutNull"),
            "text_empty_key" => with_chunk(2, 8, b"tEXt", b"\0value"),
            "text_long_key" => {
                let mut d = vec![b'k'; 100];
                d.push(0);
                d.extend_from_slice(b"v");
                with_chunk(2, 8, b"tEXt", &d)
            }
            "text_bad_key_chars" => with_chunk(2, 8, b"tEXt", b"a\x01b\0v"),
            "text_len0" => with_chunk(2, 8, b"tEXt", &[]),
            "ztxt_bad_method" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"Key\0");
                d.push(7);
                d.extend_from_slice(&mkpng::zlib_stored(b"data"));
                with_chunk(2, 8, b"zTXt", &d)
            }
            "ztxt_bad_stream" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"Key\0");
                d.push(0);
                d.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
                with_chunk(2, 8, b"zTXt", &d)
            }
            "ztxt_truncated" => with_chunk(2, 8, b"zTXt", b"Key\0"),
            "itxt_bad_compflag" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"Key\0");
                d.push(7);
                d.push(0);
                d.extend_from_slice(b"en\0");
                d.extend_from_slice(b"K\0");
                d.extend_from_slice(b"text");
                with_chunk(2, 8, b"iTXt", &d)
            }
            "itxt_bad_method" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"Key\0");
                d.push(1);
                d.push(9);
                d.extend_from_slice(b"en\0");
                d.extend_from_slice(b"K\0");
                d.extend_from_slice(&mkpng::zlib_stored(b"text"));
                with_chunk(2, 8, b"iTXt", &d)
            }
            "itxt_truncated" => with_chunk(2, 8, b"iTXt", b"Key\0\0\0en\0"),
            "iccp_bad_method" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"ICC\0");
                d.push(9);
                d.extend_from_slice(&mkpng::zlib_stored(&super::scen::make_icc(false)));
                with_chunk(2, 8, b"iCCP", &d)
            }
            "iccp_bad_stream" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"ICC\0");
                d.push(0);
                d.extend_from_slice(&[1, 2, 3, 4, 5]);
                with_chunk(2, 8, b"iCCP", &d)
            }
            "iccp_short_profile" => {
                let mut d = Vec::new();
                d.extend_from_slice(b"ICC\0");
                d.push(0);
                d.extend_from_slice(&mkpng::zlib_stored(&[0u8; 20]));
                with_chunk(2, 8, b"iCCP", &d)
            }
            "iccp_no_acsp" => {
                let mut prof = super::scen::make_icc(false);
                prof[36] = b'x';
                let mut d = Vec::new();
                d.extend_from_slice(b"ICC\0");
                d.push(0);
                d.extend_from_slice(&mkpng::zlib_stored(&prof));
                with_chunk(2, 8, b"iCCP", &d)
            }
            "iccp_bad_length_field" => {
                let mut prof = super::scen::make_icc(false);
                prof[0..4].copy_from_slice(&999u32.to_be_bytes());
                let mut d = Vec::new();
                d.extend_from_slice(b"ICC\0");
                d.push(0);
                d.extend_from_slice(&mkpng::zlib_stored(&prof));
                with_chunk(2, 8, b"iCCP", &d)
            }
            "iccp_bad_colorspace" => {
                let mut prof = super::scen::make_icc(false);
                prof[16..20].copy_from_slice(b"CMYK");
                let mut d = Vec::new();
                d.extend_from_slice(b"ICC\0");
                d.push(0);
                d.extend_from_slice(&mkpng::zlib_stored(&prof));
                with_chunk(2, 8, b"iCCP", &d)
            }
            "iccp_bad_devclass" => {
                let mut prof = super::scen::make_icc(false);
                prof[12..16].copy_from_slice(b"zzzz");
                let mut d = Vec::new();
                d.extend_from_slice(b"ICC\0");
                d.push(0);
                d.extend_from_slice(&mkpng::zlib_stored(&prof));
                with_chunk(2, 8, b"iCCP", &d)
            }
            "iccp_no_null" => with_chunk(2, 8, b"iCCP", b"NoNullHere"),
            "iccp_after_plte" => {
                let mut prof = super::scen::make_icc(false);
                prof[0] = prof[0];
                let mut d = Vec::new();
                d.extend_from_slice(b"ICC\0");
                d.push(0);
                d.extend_from_slice(&mkpng::zlib_stored(&prof));
                with_chunk_tail(2, 8, b"iCCP", &d)
            }
            "time_len0" => with_chunk(2, 8, b"tIME", &[]),
            "time_len6" => with_chunk(2, 8, b"tIME", &[0u8; 6]),
            "time_bad_month" => {
                let mut d = Vec::new();
                d.extend_from_slice(&2024u16.to_be_bytes());
                d.extend_from_slice(&[13, 40, 25, 61, 62]);
                with_chunk(2, 8, b"tIME", &d)
            }
            "exif_len0" => with_chunk(2, 8, b"eXIf", &[]),
            "exif_bad_order" => with_chunk(2, 8, b"eXIf", b"XX\x2a\x00\x08\x00\x00\x00"),
            "cicp_len0" => with_chunk(2, 8, b"cICP", &[]),
            "cicp_len3" => with_chunk(2, 8, b"cICP", &[1, 2, 3]),
            "cicp_bad_range" => with_chunk(2, 8, b"cICP", &[9, 16, 0, 7]),
            "cicp_bad_matrix" => with_chunk(2, 8, b"cICP", &[9, 16, 5, 1]),
            "clli_len0" => with_chunk(2, 8, b"cLLI", &[]),
            "clli_len4" => with_chunk(2, 8, b"cLLI", &[0u8; 4]),
            "clli_msb_set" => {
                let mut d = Vec::new();
                d.extend_from_slice(&0x8000_0000u32.to_be_bytes());
                d.extend_from_slice(&1u32.to_be_bytes());
                with_chunk(2, 8, b"cLLI", &d)
            }
            "mdcv_len0" => with_chunk(2, 8, b"mDCV", &[]),
            "mdcv_len23" => with_chunk(2, 8, b"mDCV", &[0u8; 23]),
            "mdcv_msb_set" => {
                let mut d = Vec::new();
                for _ in 0..8 {
                    d.extend_from_slice(&1000u16.to_be_bytes());
                }
                d.extend_from_slice(&0x8000_0000u32.to_be_bytes());
                d.extend_from_slice(&1u32.to_be_bytes());
                with_chunk(2, 8, b"mDCV", &d)
            }
            _ => return false,
        };

        // tuning per id
        match id {
            "crc_quiet_use" => {}
            _ => {}
        }
        read_stream(&stream, None, &nop);
    }
    let _ = r;
    true
}

/* ------------------------------------------------------------------ */
/* API misuse / out-of-range arguments                                 */
/* ------------------------------------------------------------------ */

fn api_cases(id: &str) -> bool {
    let api = api();
    let r = rec();
    unsafe {
        match id {
            /* --- struct creation --- */
            "create_read_ver_100" => {
                let v = cs("1.0.0");
                let p = (api.png_create_read_struct)(v.as_ptr(), std::ptr::null_mut(), None, None);
                r.kv("read_100", p as usize);
            }
            "create_read_ver_170" => {
                let v = cs("1.7.0");
                let p = (api.png_create_read_struct)(v.as_ptr(), std::ptr::null_mut(), None, None);
                r.kv("read_170", p as usize);
            }
            "create_read_ver_junk" => {
                let v = cs("not-a-version");
                let p = (api.png_create_read_struct)(v.as_ptr(), std::ptr::null_mut(), None, None);
                r.kv("read_junk", p as usize);
            }
            "create_read_ver_null" => {
                let p = (api.png_create_read_struct)(std::ptr::null(), std::ptr::null_mut(), None, None);
                r.kv("read_null", p as usize);
                let info = (api.png_create_info_struct)(p);
                r.kv("info_nonnull", !info.is_null());
                let mut pp = p;
                let mut ii = info;
                (api.png_destroy_read_struct)(&mut pp, &mut ii, std::ptr::null_mut());
            }
            "create_read_ver_empty" => {
                let v = cs("");
                let p = (api.png_create_read_struct)(v.as_ptr(), std::ptr::null_mut(), None, None);
                r.kv("read_empty", p as usize);
            }
            "create_write_ver_100" => {
                let v = cs("1.0.0");
                let p = (api.png_create_write_struct)(v.as_ptr(), std::ptr::null_mut(), None, None);
                r.kv("write_100", p as usize);
            }
            "create_read2_ver_100" => {
                let v = cs("1.0.0");
                let p = (api.png_create_read_struct_2)(
                    v.as_ptr(), std::ptr::null_mut(), None, None, std::ptr::null_mut(), None, None,
                );
                r.kv("read2_100", p as usize);
            }
            "create_write2_ver_100" => {
                let v = cs("1.0.0");
                let p = (api.png_create_write_struct_2)(
                    v.as_ptr(), std::ptr::null_mut(), None, None, std::ptr::null_mut(), None, None,
                );
                r.kv("write2_100", p as usize);
            }
            "create_info_null_png" => {
                let info = (api.png_create_info_struct)(std::ptr::null_mut());
                r.kv("info_from_null", info as usize);
            }
            "destroy_null_everything" => {
                let mut p: PngPtr = std::ptr::null_mut();
                (api.png_destroy_read_struct)(&mut p, std::ptr::null_mut(), std::ptr::null_mut());
                (api.png_destroy_write_struct)(&mut p, std::ptr::null_mut());
                (api.png_destroy_read_struct)(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
                r.line("destroy null ok");
            }
            "get_ihdr_on_empty_info" => {
                let (png, _info, end) = new_read(&base(2, 8));
                let mut w = 0u32;
                let mut h = 0u32;
                let (mut bd, mut ct, mut il, mut cm, mut fm) = (0i32, 0i32, 0i32, 0i32, 0i32);
                let ret = (api.png_get_IHDR)(
                    png, end, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm,
                );
                r.kv("ihdr", format!("{ret} {w} {h} {bd} {ct} {il} {cm} {fm}"));
            }
            "get_ihdr_on_empty_info_benign" => {
                let (png, _info, end) = new_read(&base(2, 8));
                (api.png_set_benign_errors)(png, 1);
                let mut w = 0u32;
                let mut h = 0u32;
                let (mut bd, mut ct, mut il, mut cm, mut fm) = (0i32, 0i32, 0i32, 0i32, 0i32);
                let ret = (api.png_get_IHDR)(
                    png, end, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm,
                );
                r.kv("ihdr", format!("{ret} {w} {h} {bd} {ct} {il} {cm} {fm}"));
            }
            "info_init_3_small" => {
                let (png, info) = new_write();
                let mut ip = info;
                (api.png_info_init_3)(&mut ip, 8);
                r.kv("info_init_small_nonnull", !ip.is_null());
                let mut p = png;
                (api.png_destroy_write_struct)(&mut p, &mut ip);
            }
            "argorder_chrm" | "argorder_chrm_xyz" | "argorder_mdcv" | "argorder_clli"
            | "argorder_rgb2gray" | "argorder_gamma" => {
                // Every one of these C wrappers converts several `double`
                // arguments with png_fixed()/convert_gamma_value() inside a
                // single call expression.  C leaves the evaluation order
                // unspecified, so the *message* of the first failing conversion
                // is decided by the compiler; the reference build must be
                // matched exactly.  Feed +inf everywhere and record which
                // conversion reports first.
                let inf = f64::INFINITY;
                match id {
                    "argorder_chrm" => {
                        let (png, info) = new_write();
                        (api.png_set_cHRM)(png, info, inf, inf, inf, inf, inf, inf, inf, inf);
                    }
                    "argorder_chrm_xyz" => {
                        let (png, info) = new_write();
                        (api.png_set_cHRM_XYZ)(png, info, inf, inf, inf, inf, inf, inf, inf, inf, inf);
                    }
                    "argorder_mdcv" => {
                        let (png, info) = new_write();
                        (api.png_set_mDCV)(png, info, inf, inf, inf, inf, inf, inf, inf, inf, inf, inf);
                    }
                    "argorder_clli" => {
                        let (png, info) = new_write();
                        (api.png_set_cLLI)(png, info, inf, inf);
                    }
                    "argorder_rgb2gray" => {
                        let (png, info, _e) = new_read(&base(2, 8));
                        (api.png_read_info)(png, info);
                        (api.png_set_rgb_to_gray)(png, PNG_ERROR_ACTION_NONE, inf, inf);
                    }
                    _ => {
                        let (png, info, _e) = new_read(&base(2, 8));
                        (api.png_read_info)(png, info);
                        (api.png_set_gamma)(png, inf, inf);
                    }
                }
                r.line("no conversion failed");
            }
            | "fp_gama_1e300" | "fp_gama_neg1e300" | "fp_gama_tiny" | "fp_gama_edge"
            | "fp_clli_nan" | "fp_clli_negnan" | "fp_clli_inf" | "fp_clli_neginf"
            | "fp_clli_1e300" | "fp_clli_neg1e300" | "fp_clli_tiny" | "fp_clli_edge" => {
                let v: f64 = if id.ends_with("_nan") {
                    f64::NAN
                } else if id.ends_with("_negnan") {
                    -f64::NAN
                } else if id.ends_with("_inf") {
                    f64::INFINITY
                } else if id.ends_with("_neginf") {
                    f64::NEG_INFINITY
                } else if id.ends_with("_1e300") {
                    1e300
                } else if id.ends_with("_neg1e300") {
                    -1e300
                } else if id.ends_with("_tiny") {
                    1e-300
                } else {
                    21474.83648
                };
                let (png, info) = new_write();
                (api.png_set_benign_errors)(png, 1);
                if id.starts_with("fp_gama") {
                    (api.png_set_gAMA)(png, info, v);
                    let mut got = 0i32;
                    r.kv("gAMA", format!("{} {got}", (api.png_get_gAMA_fixed)(png, info, &mut got)));
                } else {
                    (api.png_set_cLLI)(png, info, v, v);
                    let (mut a, mut b) = (0u32, 0u32);
                    r.kv("cLLI", format!("{} {a} {b}", (api.png_get_cLLI_fixed)(png, info, &mut a, &mut b)));
                }
                r.line("fp value handled");
            }
            "fp_nan_alpha_mode" => {
                let (png, _info, _e) = new_read(&base(6, 8));
                (api.png_set_benign_errors)(png, 1);
                (api.png_set_alpha_mode)(png, PNG_ALPHA_PNG, f64::NAN);
                r.line("alpha_mode NaN accepted");
            }
            "fp_nan_gamma" => {
                let (png, _info, _e) = new_read(&base(2, 8));
                (api.png_set_benign_errors)(png, 1);
                (api.png_set_gamma)(png, f64::NAN, f64::NAN);
                r.line("gamma NaN accepted");
            }
            "fp_nan_rgb_to_gray" => {
                let (png, _info, _e) = new_read(&base(2, 8));
                (api.png_set_benign_errors)(png, 1);
                (api.png_set_rgb_to_gray)(png, PNG_ERROR_ACTION_NONE, f64::NAN, f64::NAN);
                r.line("rgb_to_gray NaN accepted");
            }
            "fp_nan_background" => {
                let (png, info, _e) = new_read(&base(6, 8));
                (api.png_set_benign_errors)(png, 1);
                (api.png_read_info)(png, info);
                let c = png_color_16::default();
                (api.png_set_background)(png, &c, PNG_BACKGROUND_GAMMA_SCREEN, 0, f64::NAN);
                r.line("background NaN accepted");
            }
            "fp_nan_chrm" => {
                let (png, info) = new_write();
                (api.png_set_benign_errors)(png, 1);
                let n = f64::NAN;
                (api.png_set_cHRM)(png, info, n, n, n, n, n, n, n, n);
                let mut o = [0i32; 8];
                r.kv(
                    "chrm",
                    (api.png_get_cHRM_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                        &mut o[6], &mut o[7],
                    ),
                );
                r.kv("vals", format!("{o:?}"));
            }
            "fp_nan_chrm_xyz" => {
                let (png, info) = new_write();
                (api.png_set_benign_errors)(png, 1);
                let n = f64::NAN;
                (api.png_set_cHRM_XYZ)(png, info, n, n, n, n, n, n, n, n, n);
                let mut o = [0i32; 8];
                r.kv(
                    "chrm",
                    (api.png_get_cHRM_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                        &mut o[6], &mut o[7],
                    ),
                );
                r.kv("vals", format!("{o:?}"));
            }
            "fp_nan_scal" => {
                let (png, info) = new_write();
                (api.png_set_benign_errors)(png, 1);
                (api.png_set_sCAL)(png, info, PNG_SCALE_METER, f64::NAN, f64::NAN);
                let mut u = 0i32;
                let mut w: *mut c_char = std::ptr::null_mut();
                let mut h: *mut c_char = std::ptr::null_mut();
                r.kv("scal", format!("{} {u}", (api.png_get_sCAL_s)(png, info, &mut u, &mut w, &mut h)));
                r.cstr("w", w);
                r.cstr("h", h);
            }
            "fp_inf_scal" => {
                let (png, info) = new_write();
                (api.png_set_benign_errors)(png, 1);
                (api.png_set_sCAL)(png, info, PNG_SCALE_METER, f64::INFINITY, 1.0);
                let mut u = 0i32;
                let mut w: *mut c_char = std::ptr::null_mut();
                let mut h: *mut c_char = std::ptr::null_mut();
                r.kv("scal", format!("{} {u}", (api.png_get_sCAL_s)(png, info, &mut u, &mut w, &mut h)));
                r.cstr("w", w);
                r.cstr("h", h);
            }
            "fp_nan_mdcv" => {
                let (png, info) = new_write();
                (api.png_set_benign_errors)(png, 1);
                let n = f64::NAN;
                (api.png_set_mDCV)(png, info, n, n, n, n, n, n, n, n, n, n);
                let mut o = [0i32; 8];
                let (mut a, mut b) = (0u32, 0u32);
                r.kv(
                    "mdcv",
                    (api.png_get_mDCV_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                        &mut o[6], &mut o[7], &mut a, &mut b,
                    ),
                );
                r.kv("vals", format!("{o:?} {a} {b}"));
            }
            "fp_getters_after_fixed" => {
                let (png, info) = new_write();
                (api.png_set_benign_errors)(png, 1);
                for v in [1i32, 45455, 100000, PNG_FP_MAX, -1, -2, 0] {
                    (api.png_set_gAMA_fixed)(png, info, v);
                    let mut d = 0f64;
                    let ret = (api.png_get_gAMA)(png, info, &mut d);
                    r.kv(&format!("gama/{v}"), format!("{ret} {d:.10e}"));
                    (api.png_set_invalid)(png, info, PNG_INFO_gAMA as c_int);
                }
                (api.png_set_pHYs)(png, info, 0, 0, PNG_RESOLUTION_UNKNOWN);
                r.kv("aspect_zero", (api.png_get_pixel_aspect_ratio_fixed)(png, info));
                (api.png_set_pHYs)(png, info, 1, 0xffff_ffff, PNG_RESOLUTION_UNKNOWN);
                r.kv("aspect_big", (api.png_get_pixel_aspect_ratio_fixed)(png, info));
                (api.png_set_oFFs)(png, info, i32::MIN, i32::MAX, PNG_OFFSET_MICROMETER);
                r.kv("xinch", (api.png_get_x_offset_inches_fixed)(png, info));
                r.kv("yinch", (api.png_get_y_offset_inches_fixed)(png, info));
            }

            /* --- getters with NULL arguments --- */
            "getters_null" => {
                let n: PngPtr = std::ptr::null_mut();
                r.kv("valid", (api.png_get_valid)(n, n, PNG_INFO_gAMA));
                r.kv("rowbytes", (api.png_get_rowbytes)(n, n));
                r.kv("channels", (api.png_get_channels)(n, n));
                r.kv("width", (api.png_get_image_width)(n, n));
                r.kv("height", (api.png_get_image_height)(n, n));
                r.kv("bitdepth", (api.png_get_bit_depth)(n, n));
                r.kv("colortype", (api.png_get_color_type)(n, n));
                r.kv("filtertype", (api.png_get_filter_type)(n, n));
                r.kv("interlace", (api.png_get_interlace_type)(n, n));
                r.kv("comptype", (api.png_get_compression_type)(n, n));
                r.kv("ppm", (api.png_get_pixels_per_meter)(n, n));
                r.kv("xppm", (api.png_get_x_pixels_per_meter)(n, n));
                r.kv("yppm", (api.png_get_y_pixels_per_meter)(n, n));
                r.kv("par", (api.png_get_pixel_aspect_ratio_fixed)(n, n));
                r.kv("xoff", (api.png_get_x_offset_pixels)(n, n));
                r.kv("yoff", (api.png_get_y_offset_pixels)(n, n));
                r.kv("xoffm", (api.png_get_x_offset_microns)(n, n));
                r.kv("yoffm", (api.png_get_y_offset_microns)(n, n));
                r.kv("ppi", (api.png_get_pixels_per_inch)(n, n));
                r.kv("xoffi", (api.png_get_x_offset_inches_fixed)(n, n));
                r.kv("yoffi", (api.png_get_y_offset_inches_fixed)(n, n));
                r.kv("errptr", (api.png_get_error_ptr)(n) as usize);
                r.kv("ioptr", (api.png_get_io_ptr)(n) as usize);
                r.kv("iostate", (api.png_get_io_state)(n));
                r.kv("iochunk", (api.png_get_io_chunk_type)(n));
                r.kv("memptr", (api.png_get_mem_ptr)(n) as usize);
                r.kv("progptr", (api.png_get_progressive_ptr)(n) as usize);
                r.kv("uchunkptr", (api.png_get_user_chunk_ptr)(n) as usize);
                r.kv("utptr", (api.png_get_user_transform_ptr)(n) as usize);
                r.kv("widthmax", (api.png_get_user_width_max)(n));
                r.kv("heightmax", (api.png_get_user_height_max)(n));
                r.kv("cachemax", (api.png_get_chunk_cache_max)(n));
                r.kv("mallocmax", (api.png_get_chunk_malloc_max)(n));
                r.kv("cbufsize", (api.png_get_compression_buffer_size)(n));
                r.kv("palettemax", (api.png_get_palette_max)(n, n));
                r.kv("rgb2gray", (api.png_get_rgb_to_gray_status)(n));
                r.kv("currow", (api.png_get_current_row_number)(n));
                r.kv("curpass", (api.png_get_current_pass_number)(n));
                r.kv("signature", (api.png_get_signature)(n, n) as usize);
                r.kv("rows", (api.png_get_rows)(n, n) as usize);
                let mut w = 0u32;
                let mut h = 0u32;
                let (mut bd, mut ct, mut il, mut cm, mut fm) = (0, 0, 0, 0, 0);
                r.kv(
                    "ihdr",
                    (api.png_get_IHDR)(n, n, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm),
                );
                let mut gd = 0f64;
                r.kv("gama", (api.png_get_gAMA)(n, n, &mut gd));
                let mut gf = 0i32;
                r.kv("gamafix", (api.png_get_gAMA_fixed)(n, n, &mut gf));
                let mut intent = 0i32;
                r.kv("srgb", (api.png_get_sRGB)(n, n, &mut intent));
                let mut pal: *mut png_color = std::ptr::null_mut();
                let mut np = 0i32;
                r.kv("plte", (api.png_get_PLTE)(n, n, &mut pal, &mut np));
                let mut ta: *mut u8 = std::ptr::null_mut();
                let mut tn = 0i32;
                let mut tc: *mut png_color_16 = std::ptr::null_mut();
                r.kv("trns", (api.png_get_tRNS)(n, n, &mut ta, &mut tn, &mut tc));
                let mut bk: *mut png_color_16 = std::ptr::null_mut();
                r.kv("bkgd", (api.png_get_bKGD)(n, n, &mut bk));
                let mut sb: *mut png_color_8 = std::ptr::null_mut();
                r.kv("sbit", (api.png_get_sBIT)(n, n, &mut sb));
                let mut hs: *mut u16 = std::ptr::null_mut();
                r.kv("hist", (api.png_get_hIST)(n, n, &mut hs));
                let (mut a1, mut a2, mut a3) = (0u32, 0u32, 0i32);
                r.kv("phys", (api.png_get_pHYs)(n, n, &mut a1, &mut a2, &mut a3));
                let (mut b1, mut b2, mut b3) = (0i32, 0i32, 0i32);
                r.kv("offs", (api.png_get_oFFs)(n, n, &mut b1, &mut b2, &mut b3));
                let mut tp: *mut png_time = std::ptr::null_mut();
                r.kv("time", (api.png_get_tIME)(n, n, &mut tp));
                let mut txt: *mut png_text = std::ptr::null_mut();
                let mut ntxt = 0i32;
                r.kv("text", (api.png_get_text)(n, n, &mut txt, &mut ntxt));
                let mut sp: *mut png_sPLT_t = std::ptr::null_mut();
                r.kv("splt", (api.png_get_sPLT)(n, n, &mut sp));
                let mut uk: *mut png_unknown_chunk = std::ptr::null_mut();
                r.kv("unknown", (api.png_get_unknown_chunks)(n, n, &mut uk));
                let mut en = 0u32;
                let mut ep: *mut u8 = std::ptr::null_mut();
                r.kv("exif", (api.png_get_eXIf_1)(n, n, &mut en, &mut ep));
                let (mut c1, mut c2, mut c3, mut c4) = (0u8, 0u8, 0u8, 0u8);
                r.kv("cicp", (api.png_get_cICP)(n, n, &mut c1, &mut c2, &mut c3, &mut c4));
                let (mut l1, mut l2) = (0u32, 0u32);
                r.kv("clli", (api.png_get_cLLI_fixed)(n, n, &mut l1, &mut l2));
                r.kv("copyright", (api.png_get_copyright)(n) as usize != 0);
                r.kv("libpngver", (api.png_get_libpng_ver)(n) as usize != 0);
            }
            "setters_null_png" => {
                let n: PngPtr = std::ptr::null_mut();
                (api.png_set_IHDR)(n, n, 4, 4, 8, 2, 0, 0, 0);
                (api.png_set_gAMA_fixed)(n, n, 45455);
                (api.png_set_sRGB)(n, n, 0);
                (api.png_set_pHYs)(n, n, 1, 1, 0);
                (api.png_set_oFFs)(n, n, 1, 1, 0);
                (api.png_set_invalid)(n, n, 0xffff);
                (api.png_set_expand)(n);
                (api.png_set_bgr)(n);
                (api.png_set_swap)(n);
                (api.png_set_packing)(n);
                (api.png_set_packswap)(n);
                (api.png_set_invert_mono)(n);
                (api.png_set_invert_alpha)(n);
                (api.png_set_swap_alpha)(n);
                (api.png_set_strip_alpha)(n);
                (api.png_set_strip_16)(n);
                (api.png_set_scale_16)(n);
                (api.png_set_gray_to_rgb)(n);
                (api.png_set_palette_to_rgb)(n);
                (api.png_set_tRNS_to_alpha)(n);
                (api.png_set_expand_16)(n);
                (api.png_set_expand_gray_1_2_4_to_8)(n);
                (api.png_set_benign_errors)(n, 1);
                (api.png_set_check_for_invalid_index)(n, 1);
                (api.png_set_crc_action)(n, 0, 0);
                (api.png_set_user_limits)(n, 1, 1);
                (api.png_set_chunk_cache_max)(n, 1);
                (api.png_set_chunk_malloc_max)(n, 1);
                (api.png_set_compression_level)(n, 5);
                (api.png_set_filter)(n, 0, PNG_ALL_FILTERS);
                (api.png_set_flush)(n, 1);
                r.kv("interlace_null", (api.png_set_interlace_handling)(n));
                r.kv("option_null", (api.png_set_option)(n, 2, 3));
                r.kv("mng_null", (api.png_permit_mng_features)(n, 5));
                r.kv("handleas_null", (api.png_handle_as_unknown)(n, b"gAMA\0".as_ptr()));
                r.kv("resetz_null", (api.png_reset_zstream)(n));
                r.line("setters on NULL survived");
            }

            /* --- signature bookkeeping --- */
            "set_sig_bytes_9" => {
                let (png, _info, _e) = new_read(&[]);
                (api.png_set_sig_bytes)(png, 9);
                r.line("unreachable if error");
            }
            "set_sig_bytes_neg" => {
                let (png, _info, _e) = new_read(&[]);
                (api.png_set_sig_bytes)(png, -1);
                r.line("neg accepted");
            }
            "set_sig_bytes_8_then_read" => {
                let s = base(2, 8);
                let (png, info, end) = new_read(&s[8..]);
                (api.png_set_sig_bytes)(png, 8);
                (api.png_read_info)(png, info);
                r.kv("width", (api.png_get_image_width)(png, info));
                let rb = (api.png_get_rowbytes)(png, info);
                let hh = (api.png_get_image_height)(png, info);
                let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0u8; rb]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_read_image)(png, rp.as_mut_ptr());
                (api.png_read_end)(png, end);
                r.digest("rows", &rows.concat());
            }

            /* --- png_data_freer --- */
            "data_freer_bad" => {
                let (png, info) = new_write();
                (api.png_data_freer)(png, info, 99, PNG_FREE_ALL);
                r.line("unreachable if error");
            }
            "data_freer_user" => {
                let (png, info) = new_write();
                (api.png_data_freer)(png, info, 2, PNG_FREE_ALL);
                (api.png_data_freer)(png, info, 1, PNG_FREE_ALL);
                r.line("freer ok");
            }
            "free_data_null_info" => {
                let (png, _info) = new_write();
                (api.png_free_data)(png, std::ptr::null_mut(), PNG_FREE_ALL, -1);
                r.line("free_data null ok");
            }

            /* --- pngset validators --- */
            "set_scal_unit_0" => {
                let (png, info) = new_write();
                let w = cs("1.0");
                let h = cs("1.0");
                (api.png_set_sCAL_s)(png, info, 0, w.as_ptr(), h.as_ptr());
                r.line("unreachable if error");
            }
            "set_scal_unit_3" => {
                let (png, info) = new_write();
                let w = cs("1.0");
                let h = cs("1.0");
                (api.png_set_sCAL_s)(png, info, 3, w.as_ptr(), h.as_ptr());
                r.line("unit 3 accepted");
            }
            "set_scal_width_neg" => {
                let (png, info) = new_write();
                let w = cs("-1.0");
                let h = cs("1.0");
                (api.png_set_sCAL_s)(png, info, 1, w.as_ptr(), h.as_ptr());
                r.line("unreachable if error");
            }
            "set_scal_width_junk" => {
                let (png, info) = new_write();
                let w = cs("abc");
                let h = cs("1.0");
                (api.png_set_sCAL_s)(png, info, 1, w.as_ptr(), h.as_ptr());
                r.line("unreachable if error");
            }
            "set_scal_height_junk" => {
                let (png, info) = new_write();
                let w = cs("1.0");
                let h = cs("xyz");
                (api.png_set_sCAL_s)(png, info, 1, w.as_ptr(), h.as_ptr());
                r.line("unreachable if error");
            }
            "set_scal_height_zero" => {
                let (png, info) = new_write();
                let w = cs("1.0");
                let h = cs("0");
                (api.png_set_sCAL_s)(png, info, 1, w.as_ptr(), h.as_ptr());
                r.line("unreachable if error");
            }
            "set_scal_fixed_neg" => {
                let (png, info) = new_write();
                (api.png_set_sCAL_fixed)(png, info, 1, -100, 100);
                r.line("unreachable if error");
            }
            "set_plte_257" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                let pal = vec![png_color::default(); 300];
                (api.png_set_PLTE)(png, info, pal.as_ptr(), 257);
                r.line("unreachable if error");
            }
            "set_plte_0" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                let pal = vec![png_color::default(); 4];
                (api.png_set_PLTE)(png, info, pal.as_ptr(), 0);
                r.line("num_palette 0 accepted");
            }
            "set_plte_neg" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                let pal = vec![png_color::default(); 4];
                (api.png_set_PLTE)(png, info, pal.as_ptr(), -1);
                r.line("unreachable if error");
            }
            "set_plte_null" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                (api.png_set_PLTE)(png, info, std::ptr::null(), 4);
                r.line("unreachable if error");
            }
            "set_plte_depth_overflow" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 2, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                let pal = vec![png_color::default(); 200];
                (api.png_set_PLTE)(png, info, pal.as_ptr(), 200);
                r.line("unreachable if error");
            }
            "set_iccp_bad_method" => {
                let (png, info) = new_write();
                let prof = super::scen::make_icc(false);
                let name = cs("ICC");
                (api.png_set_iCCP)(png, info, name.as_ptr(), 7, prof.as_ptr(), prof.len() as u32);
                r.line("unreachable if error");
            }
            "set_iccp_len0" => {
                let (png, info) = new_write();
                let name = cs("ICC");
                (api.png_set_iCCP)(png, info, name.as_ptr(), 0, b"x".as_ptr(), 0);
                r.line("proflen 0 handled");
            }
            "set_iccp_short" => {
                let (png, info) = new_write();
                let name = cs("ICC");
                let prof = [0u8; 20];
                (api.png_set_iCCP)(png, info, name.as_ptr(), 0, prof.as_ptr(), 20);
                r.line("short profile handled");
            }
            "set_iccp_bad_name" => {
                let (png, info) = new_write();
                let name = cs("");
                let prof = super::scen::make_icc(false);
                (api.png_set_iCCP)(png, info, name.as_ptr(), 0, prof.as_ptr(), prof.len() as u32);
                r.line("empty name handled");
            }
            "set_splt_null" => {
                let (png, info) = new_write();
                (api.png_set_sPLT)(png, info, std::ptr::null(), 1);
                r.line("null sPLT handled");
            }
            "set_splt_zero" => {
                let (png, info) = new_write();
                let e = png_sPLT_t {
                    name: std::ptr::null_mut(),
                    depth: 8,
                    entries: std::ptr::null_mut(),
                    nentries: 0,
                };
                (api.png_set_sPLT)(png, info, &e, 1);
                r.line("unreachable if error");
            }
            "set_text_null_key" => {
                let (png, info) = new_write();
                let v = cs("value");
                let t = png_text {
                    compression: -1,
                    key: std::ptr::null_mut(),
                    text: v.as_ptr() as *mut c_char,
                    ..Default::default()
                };
                (api.png_set_text)(png, info, &t, 1);
                let mut gt: *mut png_text = std::ptr::null_mut();
                let mut gn = 0i32;
                r.kv("text", format!("{} {gn}", (api.png_get_text)(png, info, &mut gt, &mut gn)));
            }
            "set_text_bad_comp" => {
                let (png, info) = new_write();
                let k = cs("Key");
                let v = cs("value");
                let t = png_text {
                    compression: 9,
                    key: k.as_ptr() as *mut c_char,
                    text: v.as_ptr() as *mut c_char,
                    ..Default::default()
                };
                (api.png_set_text)(png, info, &t, 1);
                let mut gt: *mut png_text = std::ptr::null_mut();
                let mut gn = 0i32;
                r.kv("text", format!("{} {gn}", (api.png_get_text)(png, info, &mut gt, &mut gn)));
            }
            "set_text_long_key" => {
                let (png, info) = new_write();
                let k = cs(&"k".repeat(200));
                let v = cs("value");
                let t = png_text {
                    compression: -1,
                    key: k.as_ptr() as *mut c_char,
                    text: v.as_ptr() as *mut c_char,
                    ..Default::default()
                };
                (api.png_set_text)(png, info, &t, 1);
                let mut gt: *mut png_text = std::ptr::null_mut();
                let mut gn = 0i32;
                r.kv("text", format!("{} {gn}", (api.png_get_text)(png, info, &mut gt, &mut gn)));
                if gn > 0 {
                    r.cstr("key", (*gt).key);
                }
            }
            "set_text_itxt_no_lang" => {
                let (png, info) = new_write();
                let k = cs("Key");
                let v = cs("value");
                let t = png_text {
                    compression: 1,
                    key: k.as_ptr() as *mut c_char,
                    text: v.as_ptr() as *mut c_char,
                    ..Default::default()
                };
                (api.png_set_text)(png, info, &t, 1);
                let mut gt: *mut png_text = std::ptr::null_mut();
                let mut gn = 0i32;
                r.kv("text", format!("{} {gn}", (api.png_get_text)(png, info, &mut gt, &mut gn)));
                if gn > 0 {
                    r.cstr("lang", (*gt).lang);
                    r.cstr("langkey", (*gt).lang_key);
                }
            }
            "set_unknown_bad_location" => {
                let (png, info) = new_write();
                let data = b"x".to_vec();
                let u = png_unknown_chunk {
                    name: *b"prVt\0",
                    data: data.as_ptr() as *mut u8,
                    size: 1,
                    location: 0,
                };
                (api.png_set_unknown_chunks)(png, info, &u, 1);
                (api.png_set_unknown_chunk_location)(png, info, 0, 99);
                r.line("unreachable if error");
            }
            "set_unknown_loc_index_oob" => {
                let (png, info) = new_write();
                (api.png_set_unknown_chunk_location)(png, info, 5, PNG_HAVE_IHDR);
                r.line("index oob handled");
            }
            "keep_unknown_bad_keep" => {
                let (png, _info, _e) = new_read(&[]);
                (api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_LAST, std::ptr::null(), 0);
                r.line("unreachable if error");
            }
            "keep_unknown_neg_keep" => {
                let (png, _info, _e) = new_read(&[]);
                (api.png_set_keep_unknown_chunks)(png, -1, std::ptr::null(), 0);
                r.line("unreachable if error");
            }
            "keep_unknown_null_list" => {
                let (png, _info, _e) = new_read(&[]);
                (api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 3);
                r.line("unreachable if error");
            }
            "keep_unknown_negative_num" => {
                let (png, _info, _e) = new_read(&[]);
                (api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), -1);
                r.line("negative num handled");
                for probe in ["gAMA", "IHDR", "prVt"] {
                    let mut nb = [0u8; 5];
                    for (k, c) in probe.bytes().take(4).enumerate() {
                        nb[k] = c;
                    }
                    r.kv(probe, (api.png_handle_as_unknown)(png, nb.as_ptr()));
                }
            }
            "keep_unknown_ihdr_listed" => {
                let (png, _info, _e) = new_read(&[]);
                let mut list = Vec::new();
                list.extend_from_slice(b"IHDR\0");
                (api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, list.as_ptr(), 1);
                r.kv("ihdr", (api.png_handle_as_unknown)(png, b"IHDR\0".as_ptr()));
            }
            "cbuf_size_zero" => {
                let (png, _info) = new_write();
                (api.png_set_compression_buffer_size)(png, 0);
                r.line("unreachable if error");
            }
            "cbuf_size_huge" => {
                let (png, _info) = new_write();
                (api.png_set_compression_buffer_size)(png, usize::MAX);
                r.kv("size", (api.png_get_compression_buffer_size)(png));
            }
            "chrm_xyz_invalid" => {
                let (png, info) = new_write();
                (api.png_set_cHRM_XYZ_fixed)(png, info, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                r.line("unreachable if error");
            }
            "chrm_negative" => {
                let (png, info) = new_write();
                (api.png_set_cHRM_fixed)(png, info, -1, -1, -1, -1, -1, -1, -1, -1);
                let mut o = [0i32; 8];
                r.kv(
                    "chrm",
                    (api.png_get_cHRM_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                        &mut o[6], &mut o[7],
                    ),
                );
            }
            "gama_fixed_zero" => {
                let (png, info) = new_write();
                (api.png_set_gAMA_fixed)(png, info, 0);
                let mut v = 0i32;
                r.kv("gama", format!("{} {v}", (api.png_get_gAMA_fixed)(png, info, &mut v)));
            }
            "gama_fixed_neg" => {
                let (png, info) = new_write();
                (api.png_set_gAMA_fixed)(png, info, -5);
                let mut v = 0i32;
                r.kv("gama", format!("{} {v}", (api.png_get_gAMA_fixed)(png, info, &mut v)));
            }
            "gama_double_huge" => {
                let (png, info) = new_write();
                (api.png_set_gAMA)(png, info, 1e30);
                let mut v = 0i32;
                r.kv("gama", format!("{} {v}", (api.png_get_gAMA_fixed)(png, info, &mut v)));
            }
            "gama_double_nan" => {
                let (png, info) = new_write();
                (api.png_set_gAMA)(png, info, f64::NAN);
                let mut v = 0i32;
                r.kv("gama", format!("{} {v}", (api.png_get_gAMA_fixed)(png, info, &mut v)));
            }
            "srgb_intent_oob" => {
                let (png, info) = new_write();
                (api.png_set_sRGB)(png, info, PNG_sRGB_INTENT_LAST);
                let mut v = 0i32;
                r.kv("srgb", format!("{} {v}", (api.png_get_sRGB)(png, info, &mut v)));
                (api.png_set_sRGB_gAMA_and_cHRM)(png, info, 99);
                r.line("srgb oob handled");
            }
            "srgb_intent_neg" => {
                let (png, info) = new_write();
                (api.png_set_sRGB)(png, info, -1);
                let mut v = 0i32;
                r.kv("srgb", format!("{} {v}", (api.png_get_sRGB)(png, info, &mut v)));
            }
            "phys_unit_oob" => {
                let (png, info) = new_write();
                (api.png_set_pHYs)(png, info, 1, 1, PNG_RESOLUTION_LAST + 5);
                let (mut a, mut b, mut c) = (0u32, 0u32, 0i32);
                r.kv("phys", format!("{} {a} {b} {c}", (api.png_get_pHYs)(png, info, &mut a, &mut b, &mut c)));
            }
            "offs_unit_oob" => {
                let (png, info) = new_write();
                (api.png_set_oFFs)(png, info, 1, 1, PNG_OFFSET_LAST + 5);
                let (mut a, mut b, mut c) = (0i32, 0i32, 0i32);
                r.kv("offs", format!("{} {a} {b} {c}", (api.png_get_oFFs)(png, info, &mut a, &mut b, &mut c)));
            }
            "pcal_type_oob" => {
                let (png, info) = new_write();
                let p = cs("purpose");
                let u = cs("units");
                (api.png_set_pCAL)(png, info, p.as_ptr(), 0, 100, PNG_EQUATION_LAST, 0, u.as_ptr(), std::ptr::null_mut());
                r.line("pcal type oob handled");
            }
            "cicp_bad_full_range" => {
                let (png, info) = new_write();
                (api.png_set_cICP)(png, info, 9, 16, 0, 7);
                let (mut a, mut b, mut c, mut d) = (0u8, 0u8, 0u8, 0u8);
                r.kv("cicp", format!("{} {a} {b} {c} {d}", (api.png_get_cICP)(png, info, &mut a, &mut b, &mut c, &mut d)));
            }
            "cicp_nonzero_matrix" => {
                let (png, info) = new_write();
                (api.png_set_cICP)(png, info, 9, 16, 5, 1);
                let (mut a, mut b, mut c, mut d) = (0u8, 0u8, 0u8, 0u8);
                r.kv("cicp", format!("{} {a} {b} {c} {d}", (api.png_get_cICP)(png, info, &mut a, &mut b, &mut c, &mut d)));
            }
            "clli_msb" => {
                let (png, info) = new_write();
                (api.png_set_cLLI_fixed)(png, info, 0x8000_0000, 1);
                let (mut a, mut b) = (0u32, 0u32);
                r.kv("clli", format!("{} {a} {b}", (api.png_get_cLLI_fixed)(png, info, &mut a, &mut b)));
            }
            "mdcv_out_of_range" => {
                let (png, info) = new_write();
                (api.png_set_mDCV_fixed)(png, info, 200000, 200000, 200000, 200000, 200000, 200000, 200000, 200000, 1, 1);
                let mut o = [0i32; 8];
                let (mut a, mut b) = (0u32, 0u32);
                r.kv(
                    "mdcv",
                    (api.png_get_mDCV_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                        &mut o[6], &mut o[7], &mut a, &mut b,
                    ),
                );
            }
            "sbit_zero_write" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                let sb = png_color_8 { red: 0, green: 0, blue: 0, gray: 0, alpha: 0 };
                (api.png_set_sBIT)(png, info, &sb);
                let mut g2: *mut png_color_8 = std::ptr::null_mut();
                r.kv("sbit", (api.png_get_sBIT)(png, info, &mut g2));
            }
            "sbit_too_deep_write" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                let sb = png_color_8 { red: 9, green: 9, blue: 9, gray: 9, alpha: 9 };
                (api.png_set_sBIT)(png, info, &sb);
                let mut g2: *mut png_color_8 = std::ptr::null_mut();
                r.kv("sbit", (api.png_get_sBIT)(png, info, &mut g2));
            }
            "trns_set_too_many" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                let pal = vec![png_color::default(); 4];
                (api.png_set_PLTE)(png, info, pal.as_ptr(), 4);
                let al = vec![0u8; 300];
                (api.png_set_tRNS)(png, info, al.as_ptr(), 300, std::ptr::null());
                let mut ta: *mut u8 = std::ptr::null_mut();
                let mut tn = 0i32;
                let mut tc: *mut png_color_16 = std::ptr::null_mut();
                r.kv("trns", format!("{} {tn}", (api.png_get_tRNS)(png, info, &mut ta, &mut tn, &mut tc)));
            }
            "trns_set_null_both" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_set_tRNS)(png, info, std::ptr::null(), 0, std::ptr::null());
                let mut ta: *mut u8 = std::ptr::null_mut();
                let mut tn = 0i32;
                let mut tc: *mut png_color_16 = std::ptr::null_mut();
                r.kv("trns", format!("{} {tn}", (api.png_get_tRNS)(png, info, &mut ta, &mut tn, &mut tc)));
            }

            /* --- pngtrans / pngrtran validators --- */
            "shift_invalid" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                let sb = png_color_8 { red: 0, green: 0, blue: 0, gray: 0, alpha: 0 };
                (api.png_set_shift)(png, &sb);
                r.line("unreachable if error");
            }
            "shift_too_deep" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                let sb = png_color_8 { red: 99, green: 99, blue: 99, gray: 99, alpha: 99 };
                (api.png_set_shift)(png, &sb);
                r.line("unreachable if error");
            }
            "filler_on_palette" => {
                let (png, info, _e) = new_read(&base(3, 8));
                (api.png_read_info)(png, info);
                (api.png_set_filler)(png, 0, PNG_FILLER_AFTER);
                r.line("unreachable if error");
            }
            "filler_on_rgba" => {
                let (png, info, _e) = new_read(&base(6, 8));
                (api.png_read_info)(png, info);
                (api.png_set_filler)(png, 0, PNG_FILLER_AFTER);
                r.line("unreachable if error");
            }
            "addalpha_on_palette" => {
                let (png, info, _e) = new_read(&base(3, 8));
                (api.png_read_info)(png, info);
                (api.png_set_add_alpha)(png, 0, PNG_FILLER_AFTER);
                r.line("unreachable if error");
            }
            "alpha_mode_4" => {
                let (png, info, _e) = new_read(&base(6, 8));
                (api.png_read_info)(png, info);
                (api.png_set_alpha_mode_fixed)(png, 4, PNG_FP_1);
                r.line("unreachable if error");
            }
            "alpha_mode_neg" => {
                let (png, info, _e) = new_read(&base(6, 8));
                (api.png_read_info)(png, info);
                (api.png_set_alpha_mode_fixed)(png, -1, PNG_FP_1);
                r.line("unreachable if error");
            }
            "alpha_mode_gamma_zero" => {
                let (png, info, _e) = new_read(&base(6, 8));
                (api.png_read_info)(png, info);
                (api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_PNG, 0);
                r.line("gamma 0 handled");
            }
            "alpha_mode_gamma_neg" => {
                let (png, info, _e) = new_read(&base(6, 8));
                (api.png_read_info)(png, info);
                (api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_PNG, -5);
                r.line("unreachable if error");
            }
            "gamma_screen_zero" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                (api.png_set_gamma_fixed)(png, 0, 45455);
                r.line("unreachable if error");
            }
            "gamma_file_zero" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                (api.png_set_gamma_fixed)(png, 100000, 0);
                r.line("file gamma 0 handled");
            }
            "gamma_negative" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                (api.png_set_gamma_fixed)(png, -100000, 45455);
                r.line("unreachable if error");
            }
            "gamma_double_bad" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                (api.png_set_gamma)(png, 1e30, 1e30);
                r.line("huge double gamma handled");
            }
            "rgb2gray_action_0" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                (api.png_set_rgb_to_gray_fixed)(png, 0, -1, -1);
                r.line("action 0 handled");
            }
            "rgb2gray_action_4" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                (api.png_set_rgb_to_gray_fixed)(png, 4, -1, -1);
                r.line("unreachable if error");
            }
            "rgb2gray_bad_coeffs" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                (api.png_set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_NONE, 90000, 90000);
                r.line("coeff sum > 1 handled");
            }
            "rgb2gray_neg_coeffs" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                (api.png_set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_NONE, -50000, -50000);
                r.line("negative coeffs handled");
            }
            "background_gamma_type_4" => {
                let (png, info, _e) = new_read(&base(6, 8));
                (api.png_read_info)(png, info);
                let c = png_color_16::default();
                (api.png_set_background_fixed)(png, &c, 4, 0, PNG_FP_1);
                r.line("unreachable if error");
            }
            "background_gamma_type_neg" => {
                let (png, info, _e) = new_read(&base(6, 8));
                (api.png_read_info)(png, info);
                let c = png_color_16::default();
                (api.png_set_background_fixed)(png, &c, -1, 0, PNG_FP_1);
                r.line("unreachable if error");
            }
            "background_null_color" => {
                let (png, info, _e) = new_read(&base(6, 8));
                (api.png_read_info)(png, info);
                (api.png_set_background_fixed)(png, std::ptr::null(), PNG_BACKGROUND_GAMMA_SCREEN, 0, PNG_FP_1);
                r.line("null background handled");
            }
            "background_before_header" => {
                let (png, _info, _e) = new_read(&base(6, 8));
                let c = png_color_16::default();
                (api.png_set_background_fixed)(png, &c, PNG_BACKGROUND_GAMMA_UNKNOWN, 0, PNG_FP_1);
                r.line("before header handled");
            }
            "quantize_one_color" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                let mut pal = vec![png_color::default(); 4];
                for (i, p) in pal.iter_mut().enumerate() {
                    p.red = i as u8;
                    p.green = (i * 3) as u8;
                    p.blue = (i * 7) as u8;
                }
                let hist: Vec<u16> = vec![1, 2, 3, 4];
                (api.png_set_quantize)(png, pal.as_mut_ptr(), 4, 1, hist.as_ptr(), 1);
                r.kv("pal", format!("{:?}", pal.iter().map(|c| (c.red, c.green, c.blue)).collect::<Vec<_>>()));
                r.line("max 1 handled");
            }
            "quantize_null_palette" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                (api.png_set_quantize)(png, std::ptr::null_mut(), 4, 2, std::ptr::null(), 1);
                r.line("null palette handled");
            }
            "quantize_negative_palette" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                let mut pal = vec![png_color::default(); 4];
                (api.png_set_quantize)(png, pal.as_mut_ptr(), -1, 2, std::ptr::null(), 1);
                r.line("negative num_palette handled");
            }
            "quantize_zero_colors" => {
                let (png, info, _e) = new_read(&base(2, 8));
                (api.png_read_info)(png, info);
                let mut pal = vec![png_color::default(); 4];
                (api.png_set_quantize)(png, pal.as_mut_ptr(), 4, 0, std::ptr::null(), 0);
                r.line("max 0 handled");
            }
            "quantize_more_than_palette" => {
                let (png, info, _e) = new_read(&base(3, 8));
                (api.png_read_info)(png, info);
                let mut pal = vec![png_color::default(); 4];
                (api.png_set_quantize)(png, pal.as_mut_ptr(), 4, 300, std::ptr::null(), 1);
                r.line("max > palette handled");
            }
            "crc_action_invalid" => {
                let (png, _info, _e) = new_read(&base(2, 8));
                (api.png_set_crc_action)(png, 9, 9);
                r.line("crc action 9 handled");
            }
            "crc_action_warn_discard_critical" => {
                let (png, _info, _e) = new_read(&base(2, 8));
                (api.png_set_crc_action)(png, PNG_CRC_WARN_DISCARD, PNG_CRC_DEFAULT);
                r.line("warn/discard on critical handled");
            }
            "uint31_oob" => {
                let (png, _info, _e) = new_read(&[]);
                let b = [0x80u8, 0, 0, 0];
                let v = (api.png_get_uint_31)(png, b.as_ptr());
                r.kv("uint31", v);
            }
            "uint31_ffffffff" => {
                let (png, _info, _e) = new_read(&[]);
                let b = [0xffu8, 0xff, 0xff, 0xff];
                let v = (api.png_get_uint_31)(png, b.as_ptr());
                r.kv("uint31", v);
            }

            /* --- write-side validators --- */
            "filter_bad_method" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_set_filter)(png, 1, PNG_ALL_FILTERS);
                r.line("unreachable if error");
            }
            "filter_bad_filters" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_set_filter)(png, 0, 7);
                r.line("unreachable if error");
            }
            "filter_value_5" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_set_filter)(png, 0, 5);
                r.line("filter 5 handled");
            }
            "filter_negative" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_set_filter)(png, 0, -1);
                r.line("negative filters handled");
            }
            "write_no_palette" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                (api.png_write_info)(png, info);
                r.line("unreachable if error");
            }
            "write_no_idat" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_write_info)(png, info);
                (api.png_write_end)(png, info);
                r.line("unreachable if error");
            }
            "write_image_null_rows" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_write_info)(png, info);
                (api.png_write_image)(png, std::ptr::null_mut());
                r.line("unreachable if error");
            }
            "write_png_no_rows" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_write_png)(png, info, 0, std::ptr::null_mut());
                r.line("unreachable if error");
            }
            "write_row_too_many" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 2, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_write_info)(png, info);
                let row = vec![0u8; 12];
                (api.png_write_row)(png, row.as_ptr());
                (api.png_write_row)(png, row.as_ptr());
                (api.png_write_row)(png, row.as_ptr());
                r.line("extra row accepted");
                let out = std::mem::take(&mut g().wbuf);
                r.digest("out", &out);
            }
            "write_null_fn" => {
                let v = ver();
                let png = (api.png_create_write_struct)(v, std::ptr::null_mut(), Some(cb_error), Some(cb_warn));
                let info = (api.png_create_info_struct)(png);
                (api.png_set_write_fn)(png, std::ptr::null_mut(), None, None);
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_write_info)(png, info);
                r.line("unreachable if error");
            }
            "read_null_fn" => {
                let v = ver();
                let png = (api.png_create_read_struct)(v, std::ptr::null_mut(), Some(cb_error), Some(cb_warn));
                let info = (api.png_create_info_struct)(png);
                (api.png_set_read_fn)(png, std::ptr::null_mut(), None);
                (api.png_read_info)(png, info);
                r.line("unreachable if error");
            }
            "read_row_before_info" => {
                let (png, _info, _e) = new_read(&base(2, 8));
                let mut row = vec![0u8; 64];
                (api.png_read_row)(png, row.as_mut_ptr(), std::ptr::null_mut());
                r.line("unreachable if error");
            }
            "read_beyond_end" => {
                let s = base(2, 8);
                let (png, info, _e) = new_read(&s);
                (api.png_read_info)(png, info);
                let rb = (api.png_get_rowbytes)(png, info);
                let mut row = vec![0u8; rb];
                for _ in 0..10 {
                    (api.png_read_row)(png, row.as_mut_ptr(), std::ptr::null_mut());
                }
                r.line("unreachable if error");
            }
            "write_transform_unsupported" => {
                let (png, info) = new_write();
                (api.png_set_IHDR)(png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                let mut rows: Vec<Vec<u8>> = (0..4).map(|_| vec![0u8; 12]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_set_rows)(png, info, rp.as_mut_ptr());
                (api.png_write_png)(png, info, PNG_TRANSFORM_STRIP_16 | PNG_TRANSFORM_EXPAND, std::ptr::null_mut());
                r.line("unreachable if error");
            }
            "read_transform_unsupported" => {
                let s = base(2, 8);
                let (png, info, _e) = new_read(&s);
                (api.png_read_png)(png, info, PNG_TRANSFORM_STRIP_FILLER_BEFORE, std::ptr::null_mut());
                r.line("unreachable if error");
            }
            "reset_zstream_fresh" => {
                let (png, _info, _e) = new_read(&[]);
                r.kv("reset", (api.png_reset_zstream)(png));
            }
            "write_chunk_bad_name" => {
                let (png, _info) = new_write();
                (api.png_write_sig)(png);
                (api.png_write_chunk)(png, b"12 4".as_ptr(), b"x".as_ptr(), 1);
                let out = std::mem::take(&mut g().wbuf);
                r.bytes("out", &out);
            }
            "write_chunk_len_mismatch" => {
                let (png, _info) = new_write();
                (api.png_write_sig)(png);
                (api.png_write_chunk_start)(png, b"prVt".as_ptr(), 4);
                (api.png_write_chunk_data)(png, b"abcdefgh".as_ptr(), 8);
                (api.png_write_chunk_end)(png);
                let out = std::mem::take(&mut g().wbuf);
                r.bytes("out", &out);
            }
            "write_chunk_null_data" => {
                let (png, _info) = new_write();
                (api.png_write_sig)(png);
                (api.png_write_chunk)(png, b"prVt".as_ptr(), std::ptr::null(), 0);
                (api.png_write_chunk_data)(png, std::ptr::null(), 0);
                let out = std::mem::take(&mut g().wbuf);
                r.bytes("out", &out);
            }
            "user_limits_too_small" => {
                let s = base(2, 8);
                let (png, info, _e) = new_read(&s);
                (api.png_set_user_limits)(png, 2, 2);
                (api.png_read_info)(png, info);
                r.line("unreachable if error");
            }
            "user_limits_zero" => {
                let s = base(2, 8);
                let (png, info, _e) = new_read(&s);
                (api.png_set_user_limits)(png, 0, 0);
                (api.png_read_info)(png, info);
                r.line("zero limits handled");
                r.kv("w", (api.png_get_image_width)(png, info));
            }
            "chunk_cache_max_1" => {
                let s = synth(2, 8, 0, 4, 3, "texttail", 0, 5).png;
                let (png, info, end) = new_read(&s);
                (api.png_set_chunk_cache_max)(png, 1);
                (api.png_read_info)(png, info);
                let rb = (api.png_get_rowbytes)(png, info);
                let hh = (api.png_get_image_height)(png, info);
                let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0u8; rb]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_read_image)(png, rp.as_mut_ptr());
                (api.png_read_end)(png, end);
                let mut txt: *mut png_text = std::ptr::null_mut();
                let mut tn = 0i32;
                r.kv("text", format!("{} {tn}", (api.png_get_text)(png, end, &mut txt, &mut tn)));
            }
            "chunk_malloc_max_8" => {
                let s = synth(2, 8, 0, 4, 3, "texttail", 0, 5).png;
                let (png, info, end) = new_read(&s);
                (api.png_set_chunk_malloc_max)(png, 8);
                (api.png_read_info)(png, info);
                let rb = (api.png_get_rowbytes)(png, info);
                let hh = (api.png_get_image_height)(png, info);
                let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0u8; rb]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_read_image)(png, rp.as_mut_ptr());
                (api.png_read_end)(png, end);
                let mut txt: *mut png_text = std::ptr::null_mut();
                let mut tn = 0i32;
                r.kv("text", format!("{} {tn}", (api.png_get_text)(png, end, &mut txt, &mut tn)));
            }
            "short_read_zero_fill" => {
                let s = base(2, 8);
                g().short_read_mode = 1;
                let (png, info, end) = new_read(&s[..s.len() / 2]);
                g().short_read_mode = 1;
                (api.png_read_info)(png, info);
                let rb = (api.png_get_rowbytes)(png, info);
                let hh = (api.png_get_image_height)(png, info);
                let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0u8; rb]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_read_image)(png, rp.as_mut_ptr());
                (api.png_read_end)(png, end);
                r.digest("rows", &rows.concat());
            }
            "double_read_info" => {
                let s = base(2, 8);
                let (png, info, _e) = new_read(&s);
                (api.png_read_info)(png, info);
                (api.png_read_info)(png, info);
                r.line("double read_info survived");
            }
            "read_end_without_image" => {
                let s = base(2, 8);
                let (png, info, end) = new_read(&s);
                (api.png_read_info)(png, info);
                (api.png_read_end)(png, end);
                r.line("read_end without image survived");
            }
            "update_info_twice" => {
                let s = base(2, 8);
                let (png, info, _e) = new_read(&s);
                (api.png_read_info)(png, info);
                (api.png_read_update_info)(png, info);
                (api.png_read_update_info)(png, info);
                r.kv("rowbytes", (api.png_get_rowbytes)(png, info));
            }
            "start_read_image_twice" => {
                let s = base(2, 8);
                let (png, info, _e) = new_read(&s);
                (api.png_read_info)(png, info);
                (api.png_start_read_image)(png);
                (api.png_start_read_image)(png);
                r.line("start twice survived");
            }
            "write_flush_no_flush_fn" => {
                let v = ver();
                let png = (api.png_create_write_struct)(v, std::ptr::null_mut(), Some(cb_error), Some(cb_warn));
                let info = (api.png_create_info_struct)(png);
                (api.png_set_write_fn)(png, std::ptr::null_mut(), Some(cb_write), None);
                (api.png_set_IHDR)(png, info, 4, 2, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
                (api.png_write_info)(png, info);
                (api.png_write_flush)(png);
                r.line("flush with no flush fn survived");
            }
            "longjmp_fn_bad_size" => {
                let (png, _info, _e) = new_read(&[]);
                let p1 = (api.png_set_longjmp_fn)(png, 0x1 as *mut c_void, 8);
                r.kv("size8", p1 as usize != 0);
                let p2 = (api.png_set_longjmp_fn)(png, 0x1 as *mut c_void, 0);
                r.kv("size0", p2 as usize != 0);
                let p3 = (api.png_set_longjmp_fn)(png, 0x1 as *mut c_void, 200);
                r.kv("size200", p3 as usize != 0);
                let p4 = (api.png_set_longjmp_fn)(png, 0x1 as *mut c_void, usize::MAX);
                r.kv("sizemax", p4 as usize != 0);
                let p5 = (api.png_set_longjmp_fn)(std::ptr::null_mut(), 0x1 as *mut c_void, 200);
                r.kv("nullpng", p5 as usize);
            }
            _ => return false,
        }
    }
    true
}

/* ------------------------------------------------------------------ */
/* simplified-API errors                                               */
/* ------------------------------------------------------------------ */

fn simple_cases(id: &str) -> bool {
    let api = api();
    let r = rec();
    unsafe {
        match id {
            "sr_null_memory" => {
                let mut img = png_image::default();
                let ok = (api.png_image_begin_read_from_memory)(&mut img, std::ptr::null(), 100);
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_zero_size" => {
                let s = base(2, 8);
                let mut img = png_image::default();
                let ok = (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, 0);
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_bad_version" => {
                let s = base(2, 8);
                let mut img = png_image::default();
                img.version = 2;
                let ok = (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, s.len());
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_version_zero" => {
                let s = base(2, 8);
                let mut img = png_image::default();
                img.version = 0;
                let ok = (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, s.len());
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_not_png" => {
                let s = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
                let mut img = png_image::default();
                let ok = (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, s.len());
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_truncated" => {
                let s = base(2, 8);
                let mut img = png_image::default();
                let n = s.len() / 2;
                let ok = (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, n);
                r.kv("begin", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                if ok != 0 {
                    img.format = PNG_FORMAT_RGBA;
                    let sz = image_size(&img).max(1);
                    let mut buf = vec![0u8; sz];
                    let ok2 = (api.png_image_finish_read)(
                        &mut img,
                        std::ptr::null(),
                        buf.as_mut_ptr() as *mut c_void,
                        image_row_stride(&img) as i32,
                        std::ptr::null_mut(),
                    );
                    r.kv("finish", format!("{ok2} {:?}", img.msg()));
                    r.digest("buf", &buf);
                }
                (api.png_image_free)(&mut img);
            }
            "sr_finish_without_begin" => {
                let mut img = png_image::default();
                img.width = 4;
                img.height = 4;
                img.format = PNG_FORMAT_RGBA;
                let mut buf = vec![0u8; 64];
                let ok = (api.png_image_finish_read)(
                    &mut img,
                    std::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    16,
                    std::ptr::null_mut(),
                );
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_bad_format_bits" => {
                let s = base(2, 8);
                let mut img = png_image::default();
                let ok = (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, s.len());
                r.kv("begin", ok);
                img.format = 0xffff_ff00;
                let mut buf = vec![0u8; 1 << 16];
                let ok2 = (api.png_image_finish_read)(
                    &mut img,
                    std::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    16,
                    std::ptr::null_mut(),
                );
                r.kv("finish", format!("{ok2} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_colormap_without_buffer" => {
                let s = base(2, 8);
                let mut img = png_image::default();
                let ok = (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, s.len());
                r.kv("begin", ok);
                img.format = PNG_FORMAT_RGB_COLORMAP;
                let sz = image_size(&img).max(1);
                let mut buf = vec![0u8; sz];
                let ok2 = (api.png_image_finish_read)(
                    &mut img,
                    std::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    image_row_stride(&img) as i32,
                    std::ptr::null_mut(),
                );
                r.kv("finish", format!("{ok2} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_null_buffer" => {
                let s = base(2, 8);
                let mut img = png_image::default();
                let ok = (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, s.len());
                r.kv("begin", ok);
                img.format = PNG_FORMAT_RGBA;
                let ok2 = (api.png_image_finish_read)(
                    &mut img,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    image_row_stride(&img) as i32,
                    std::ptr::null_mut(),
                );
                r.kv("finish", format!("{ok2} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_stride_too_small" => {
                let s = base(2, 8);
                let mut img = png_image::default();
                let ok = (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, s.len());
                r.kv("begin", ok);
                img.format = PNG_FORMAT_RGBA;
                let sz = image_size(&img).max(1);
                let mut buf = vec![0u8; sz];
                let ok2 = (api.png_image_finish_read)(
                    &mut img,
                    std::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    1,
                    std::ptr::null_mut(),
                );
                r.kv("finish", format!("{ok2} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_stride_zero" => {
                let s = base(2, 8);
                let mut img = png_image::default();
                let ok = (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, s.len());
                r.kv("begin", ok);
                img.format = PNG_FORMAT_RGBA;
                let sz = image_size(&img).max(1);
                let mut buf = vec![0u8; sz];
                let ok2 = (api.png_image_finish_read)(
                    &mut img,
                    std::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    0,
                    std::ptr::null_mut(),
                );
                r.kv("finish", format!("{ok2} {} {:?}", img.warning_or_error, img.msg()));
                r.digest("buf", &buf);
                (api.png_image_free)(&mut img);
            }
            "sr_begin_file_missing" => {
                let mut img = png_image::default();
                let name = cs("/nonexistent/path/definitely-not-here.png");
                let ok = (api.png_image_begin_read_from_file)(&mut img, name.as_ptr());
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_begin_file_null" => {
                let mut img = png_image::default();
                let ok = (api.png_image_begin_read_from_file)(&mut img, std::ptr::null());
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_begin_stdio_null" => {
                let mut img = png_image::default();
                let ok = (api.png_image_begin_read_from_stdio)(&mut img, std::ptr::null_mut());
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sr_free_twice" => {
                let s = base(2, 8);
                let mut img = png_image::default();
                (api.png_image_begin_read_from_memory)(&mut img, s.as_ptr() as *const c_void, s.len());
                (api.png_image_free)(&mut img);
                (api.png_image_free)(&mut img);
                (api.png_image_free)(std::ptr::null_mut());
                r.line("free twice ok");
            }

            /* --- simplified write --- */
            "sw_zero_width" => {
                let mut img = png_image::default();
                img.width = 0;
                img.height = 4;
                img.format = PNG_FORMAT_RGBA;
                let buf = vec![0u8; 64];
                let mut n = 0usize;
                let ok = (api.png_image_write_to_memory)(
                    &mut img, std::ptr::null_mut(), &mut n, 0, buf.as_ptr() as *const c_void, 0,
                    std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {n} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_zero_height" => {
                let mut img = png_image::default();
                img.width = 4;
                img.height = 0;
                img.format = PNG_FORMAT_RGBA;
                let buf = vec![0u8; 64];
                let mut n = 0usize;
                let ok = (api.png_image_write_to_memory)(
                    &mut img, std::ptr::null_mut(), &mut n, 0, buf.as_ptr() as *const c_void, 0,
                    std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {n} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_bad_version" => {
                let mut img = png_image::default();
                img.version = 7;
                img.width = 4;
                img.height = 4;
                img.format = PNG_FORMAT_RGBA;
                let buf = vec![0u8; 64];
                let mut n = 0usize;
                let ok = (api.png_image_write_to_memory)(
                    &mut img, std::ptr::null_mut(), &mut n, 0, buf.as_ptr() as *const c_void, 0,
                    std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {n} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_bad_format" => {
                let mut img = png_image::default();
                img.width = 4;
                img.height = 4;
                img.format = 0xffff_ff00;
                let buf = vec![0u8; 4096];
                let mut n = 0usize;
                let ok = (api.png_image_write_to_memory)(
                    &mut img, std::ptr::null_mut(), &mut n, 0, buf.as_ptr() as *const c_void, 0,
                    std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {n} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_colormap_257" => {
                let mut img = png_image::default();
                img.width = 4;
                img.height = 4;
                img.format = PNG_FORMAT_RGB_COLORMAP;
                img.colormap_entries = 257;
                let buf = vec![0u8; 64];
                let cmap = vec![0u8; 4 * 300];
                let mut n = 0usize;
                let ok = (api.png_image_write_to_memory)(
                    &mut img, std::ptr::null_mut(), &mut n, 0, buf.as_ptr() as *const c_void, 0,
                    cmap.as_ptr() as *const c_void,
                );
                r.kv("ok", format!("{ok} {n} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_colormap_null" => {
                let mut img = png_image::default();
                img.width = 4;
                img.height = 4;
                img.format = PNG_FORMAT_RGB_COLORMAP;
                img.colormap_entries = 16;
                let buf = vec![0u8; 64];
                let mut n = 0usize;
                let ok = (api.png_image_write_to_memory)(
                    &mut img, std::ptr::null_mut(), &mut n, 0, buf.as_ptr() as *const c_void, 0,
                    std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {n} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_null_buffer" => {
                let mut img = png_image::default();
                img.width = 4;
                img.height = 4;
                img.format = PNG_FORMAT_RGBA;
                let mut n = 0usize;
                let ok = (api.png_image_write_to_memory)(
                    &mut img, std::ptr::null_mut(), &mut n, 0, std::ptr::null(), 0, std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {n} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_null_memory_bytes" => {
                let mut img = png_image::default();
                img.width = 4;
                img.height = 4;
                img.format = PNG_FORMAT_RGBA;
                let buf = vec![0u8; 64];
                let ok = (api.png_image_write_to_memory)(
                    &mut img, std::ptr::null_mut(), std::ptr::null_mut(), 0,
                    buf.as_ptr() as *const c_void, 0, std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_stride_too_small" => {
                let mut img = png_image::default();
                img.width = 8;
                img.height = 4;
                img.format = PNG_FORMAT_RGBA;
                let buf = vec![0u8; 8 * 4 * 4];
                let mut n = 0usize;
                let ok = (api.png_image_write_to_memory)(
                    &mut img, std::ptr::null_mut(), &mut n, 0, buf.as_ptr() as *const c_void, 4,
                    std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {n} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_buffer_too_small" => {
                let mut img = png_image::default();
                img.width = 8;
                img.height = 8;
                img.format = PNG_FORMAT_RGBA;
                let buf = vec![0x5au8; 8 * 8 * 4];
                let mut n = 4usize;
                let mut out = vec![0u8; 8];
                let ok = (api.png_image_write_to_memory)(
                    &mut img,
                    out.as_mut_ptr() as *mut c_void,
                    &mut n,
                    0,
                    buf.as_ptr() as *const c_void,
                    0,
                    std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {n} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_huge_dims" => {
                let mut img = png_image::default();
                img.width = 0x4000_0000;
                img.height = 0x4000_0000;
                img.format = PNG_FORMAT_RGBA;
                let buf = vec![0u8; 64];
                let mut n = 0usize;
                let ok = (api.png_image_write_to_memory)(
                    &mut img, std::ptr::null_mut(), &mut n, 0, buf.as_ptr() as *const c_void, 0,
                    std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {n} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_file_bad_path" => {
                let mut img = png_image::default();
                img.width = 4;
                img.height = 4;
                img.format = PNG_FORMAT_RGBA;
                let buf = vec![0u8; 64];
                let path = cs("/definitely/not/a/writable/dir/x.png");
                let ok = (api.png_image_write_to_file)(
                    &mut img, path.as_ptr(), 0, buf.as_ptr() as *const c_void, 0, std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            "sw_stdio_null" => {
                let mut img = png_image::default();
                img.width = 4;
                img.height = 4;
                img.format = PNG_FORMAT_RGBA;
                let buf = vec![0u8; 64];
                let ok = (api.png_image_write_to_stdio)(
                    &mut img, std::ptr::null_mut(), 0, buf.as_ptr() as *const c_void, 0,
                    std::ptr::null(),
                );
                r.kv("ok", format!("{ok} {} {:?}", img.warning_or_error, img.msg()));
                (api.png_image_free)(&mut img);
            }
            _ => return false,
        }
    }
    true
}
