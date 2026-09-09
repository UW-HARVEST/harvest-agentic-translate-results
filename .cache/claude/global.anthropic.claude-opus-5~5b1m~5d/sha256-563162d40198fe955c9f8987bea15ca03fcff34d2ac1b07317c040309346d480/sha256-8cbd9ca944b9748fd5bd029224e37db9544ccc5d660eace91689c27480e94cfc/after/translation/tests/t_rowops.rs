//! Differential tests for the low-level exported row operations
//! (CONFIGS.md rows 19..25).
//!
//! Every test drives BOTH the reference C `libpng.so` and the translated Rust
//! `liblibpng.so` through `dlsym` with byte-identical inputs and then compares
//!   * the whole (over-sized) row buffer,
//!   * the mutated `png_row_info`,
//!   * the error/warning message log and whether the call unwound.
//!
//! The row buffers are always allocated `rowbytes + 64` bytes long and filled
//! with a random-but-identical pattern so that any write past `rowbytes` shows
//! up in the comparison.

mod common;

use common::api;
use common::*;
use libloading::Library;
use std::cell::RefCell;
use std::ffi::{c_int, c_void};

// --------------------------------------------------------------- shape tables

const CTS: [u8; 5] = [0, 2, 3, 4, 6];
const BDS: [u8; 5] = [1, 2, 4, 8, 16];
const WS: [u32; 7] = [1, 2, 3, 7, 8, 9, 33];

/// `PNG_PACKSWAP` from `c_src/include/pngpriv.h:674`.
const PNG_PACKSWAP: png_uint_32 = 0x10000;

/// `png_pass_inc` / `png_pass_start` from `c_src/src/pngrutil.c`.
const PASS_INC: [u32; 7] = [8, 8, 4, 4, 2, 2, 1];

/// libpng's channel count for a colour type.
fn chans(ct: u8) -> u8 {
    match ct {
        0 | 3 => 1,             // GRAY / PALETTE
        4 => 2,                 // GRAY_ALPHA
        2 => 3,                 // RGB
        6 => 4,                 // RGB_ALPHA
        _ => 1,
    }
}

/// `rowbytes` as spelled out in the task description for row 19/20.
fn rb_bits(w: u32, pd: u8) -> usize {
    ((w as usize) * (pd as usize) + 7) >> 3
}

/// libpng's own `PNG_ROWBYTES(pixel_bits, width)` macro (pngpriv.h:754).
fn png_rowbytes(pd: u8, w: u32) -> usize {
    if pd >= 8 {
        (w as usize) * ((pd as usize) >> 3)
    } else {
        (((w as usize) * (pd as usize)) + 7) >> 3
    }
}

// ------------------------------------------------------------- comparison core

#[allow(clippy::too_many_arguments)]
fn diff(
    ctx: &str,
    cr: &Run<()>,
    cb: &[u8],
    cri: &png_row_info,
    rr: &Run<()>,
    rb: &[u8],
    rri: &png_row_info,
) {
    let clog: Vec<String> = cr.log.iter().map(|m| m.to_string()).collect();
    let rlog: Vec<String> = rr.log.iter().map(|m| m.to_string()).collect();
    assert_eq!(clog, rlog, "{ctx}\n  message log differs (C first)");
    assert_eq!(
        cr.out.is_some(),
        rr.out.is_some(),
        "{ctx}\n  one library unwound and the other did not (C completed={})",
        cr.out.is_some()
    );
    if cb != rb {
        panic!(
            "{ctx}\n  ROW BYTES DIFFER\n  C : {}\n  Rs: {}\n  first diff at byte {}",
            hex(cb),
            hex(rb),
            cb.iter().zip(rb.iter()).position(|(a, b)| a != b).unwrap_or(cb.len())
        );
    }
    assert_eq!(cri, rri, "{ctx}\n  row_info differs (C first)");
}

type RowOp = unsafe fn(&Library, *mut png_row_info, png_bytep);

/// Row 19 driver: `color_type x bit_depth x width x 20 random row contents`.
fn matrix_simple(label: &str, seed: u64, op: RowOp) {
    let lc = &libs().c;
    let lr = &libs().rs;
    let mut rng = Rng::new(seed);
    let mut n = 0u32;
    for &ct in CTS.iter() {
        for &bd in BDS.iter() {
            for &w in WS.iter() {
                let ch = chans(ct);
                let pd = bd * ch; // max 16*4 = 64, fits in u8
                let rbytes = rb_bits(w, pd);
                let ri0 = png_row_info {
                    width: w,
                    rowbytes: rbytes,
                    color_type: ct,
                    bit_depth: bd,
                    channels: ch,
                    pixel_depth: pd,
                };
                for it in 0..20u32 {
                    let base = rng.bytes(rbytes + 64);

                    let mut bc = base.clone();
                    let mut ric = ri0;
                    let rc = capture(|| unsafe { op(lc, &mut ric, bc.as_mut_ptr()) });

                    let mut br = base.clone();
                    let mut rir = ri0;
                    let rr = capture(|| unsafe { op(lr, &mut rir, br.as_mut_ptr()) });

                    let ctx = format!(
                        "{label}: ct={ct} bd={bd} ch={ch} pd={pd} w={w} rowbytes={rbytes} it={it}\n  in: {}",
                        hex(&base)
                    );
                    diff(&ctx, &rc, &bc, &ric, &rr, &br, &rir);
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 5 * 5 * 7 * 20, "{label}: unexpected number of cases");
}

// =============================================================== row 19

#[test]
fn r19_do_bgr() {
    matrix_simple("png_do_bgr", 0x1901_0001, api::png_do_bgr);
}

#[test]
fn r19_do_invert() {
    matrix_simple("png_do_invert", 0x1902_0002, api::png_do_invert);
}

#[test]
fn r19_do_swap() {
    matrix_simple("png_do_swap", 0x1903_0003, api::png_do_swap);
}

#[test]
fn r19_do_packswap() {
    matrix_simple("png_do_packswap", 0x1904_0004, api::png_do_packswap);
}

// =============================================================== row 20

#[test]
fn r20_do_strip_channel() {
    let lc = &libs().c;
    let lr = &libs().rs;
    let mut rng = Rng::new(0x2000_0020);
    let mut n = 0u32;
    // channels 1 (GRAY, must be a no-op), 2 (GRAY_ALPHA), 3 (RGB, no-op:
    // "the filler channel has gone already"), 4 (RGB_ALPHA).
    for &ct in [0u8, 4, 2, 6].iter() {
        for &bd in [8u8, 16].iter() {
            for &at_start in [0i32, 1].iter() {
                for &w in WS.iter() {
                    let ch = chans(ct);
                    let pd = bd * ch;
                    let rbytes = rb_bits(w, pd);
                    let ri0 = png_row_info {
                        width: w,
                        rowbytes: rbytes,
                        color_type: ct,
                        bit_depth: bd,
                        channels: ch,
                        pixel_depth: pd,
                    };
                    for it in 0..20u32 {
                        let base = rng.bytes(rbytes + 64);

                        let mut bc = base.clone();
                        let mut ric = ri0;
                        let rc = capture(|| unsafe {
                            api::png_do_strip_channel(lc, &mut ric, bc.as_mut_ptr(), at_start)
                        });

                        let mut br = base.clone();
                        let mut rir = ri0;
                        let rr = capture(|| unsafe {
                            api::png_do_strip_channel(lr, &mut rir, br.as_mut_ptr(), at_start)
                        });

                        let ctx = format!(
                            "png_do_strip_channel: ct={ct} bd={bd} ch={ch} pd={pd} w={w} \
                             rowbytes={rbytes} at_start={at_start} it={it}\n  in: {}",
                            hex(&base)
                        );
                        diff(&ctx, &rc, &bc, &ric, &rr, &br, &rir);
                        n += 1;
                    }
                }
            }
        }
    }
    assert_eq!(n, 4 * 2 * 2 * 7 * 20, "unexpected number of cases");
}

// =============================================================== row 21
//
// `png_do_check_palette_indexes` reads the row out of `png_ptr->row_buf` (NOT
// out of a row argument) and its only effect is on `png_ptr->num_palette_max`.
// Both are private, so the state is established / observed exclusively through
// public API:
//   * `png_set_IHDR` + `png_set_PLTE` + `png_write_info` set `num_palette`
//     and (via the first `png_write_row`) allocate and fill `row_buf`;
//   * `png_set_check_for_invalid_index(pp, 1)` resets `num_palette_max` to 0
//     before every probe so each direct call is measured independently;
//   * `png_get_palette_max(pp, ip)` reads `num_palette_max` back.
// The function is then *directly* called with a range of synthesised
// `png_row_info`s (width/bit_depth/pixel_depth/rowbytes varied, always with
// `rowbytes <= the real row_buf size` so the C stays in bounds).

#[derive(Debug, PartialEq)]
struct PalRec {
    /// `num_palette_max` right after `png_write_row` did its own internal check.
    after_write_row: c_int,
    /// `num_palette_max` after each direct `png_do_check_palette_indexes`.
    probes: Vec<c_int>,
}

fn pal_probes(img_rb: usize) -> Vec<png_row_info> {
    let mut v = Vec::new();
    for &pbd in [1u8, 2, 4, 8, 16].iter() {
        for &pw in [0u32, 1, 3, 8, 37].iter() {
            let rbytes = png_rowbytes(pbd, pw);
            if rbytes > img_rb {
                continue;
            }
            v.push(png_row_info {
                width: pw,
                rowbytes: rbytes,
                color_type: PNG_COLOR_TYPE_PALETTE as u8,
                bit_depth: pbd,
                channels: 1,
                pixel_depth: pbd,
            });
        }
    }
    v
}

#[test]
fn r21_do_check_palette_indexes() {
    let mut rng = Rng::new(0x2100_0021);
    for &bd in [1u8, 2, 4, 8].iter() {
        for &np in [1i32, 2, 16, 256].iter() {
            if np > (1i32 << bd) {
                continue; // png_set_PLTE would reject it
            }
            let w: u32 = 37;
            let nrows: u32 = 12;
            let img_rb = png_rowbytes(bd, w);
            let pal: Vec<png_color> = (0..np)
                .map(|_| png_color { red: rng.u8(), green: rng.u8(), blue: rng.u8() })
                .collect();
            let rows: Vec<Vec<u8>> = (0..nrows).map(|_| rng.bytes(img_rb)).collect();
            let probes = pal_probes(img_rb);

            let mut a: Vec<Run<Vec<PalRec>>> = Vec::new();
            let mut b: Vec<Run<(c_int, Vec<Msg>)>> = Vec::new();

            for lib in [&libs().c, &libs().rs] {
                let pp = unsafe { api::new_writer(lib) };
                let ip = unsafe { api::png_create_info_struct(lib, pp) };
                assert!(!ip.is_null());
                sink_reset();

                let ra = capture(|| unsafe {
                    api::png_set_IHDR(
                        lib,
                        pp,
                        ip,
                        w,
                        nrows,
                        bd as c_int,
                        PNG_COLOR_TYPE_PALETTE,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    api::png_set_PLTE(lib, pp, ip, pal.as_ptr(), np);
                    api::png_write_info(lib, pp, ip);

                    let mut recs = Vec::new();
                    for r in rows.iter() {
                        api::png_write_row(lib, pp, r.as_ptr());
                        let m = api::png_get_palette_max(lib, pp, ip);
                        let mut ps = Vec::new();
                        for probe in probes.iter() {
                            api::png_set_check_for_invalid_index(lib, pp, 1);
                            let mut ri = *probe;
                            api::png_do_check_palette_indexes(lib, pp, &mut ri);
                            ps.push(api::png_get_palette_max(lib, pp, ip));
                            // row_info is const for this function; make sure.
                            assert_eq!(&ri, probe, "png_do_check_palette_indexes mutated row_info");
                        }
                        api::png_set_check_for_invalid_index(lib, pp, 1);
                        recs.push(PalRec { after_write_row: m, probes: ps });
                    }
                    recs
                });

                // Second phase: leave num_palette_max at whatever the real row
                // shape produces and let png_write_end report it.
                let rbb = capture(|| unsafe {
                    let mut ri = png_row_info {
                        width: w,
                        rowbytes: img_rb,
                        color_type: PNG_COLOR_TYPE_PALETTE as u8,
                        bit_depth: bd,
                        channels: 1,
                        pixel_depth: bd,
                    };
                    api::png_do_check_palette_indexes(lib, pp, &mut ri);
                    let m = api::png_get_palette_max(lib, pp, ip);
                    api::png_write_end(lib, pp, ip);
                    (m, Vec::new())
                });

                let _ = sink_take();
                let _ = capture(|| unsafe {
                    let mut p = pp;
                    let mut i = ip;
                    api::png_destroy_write_struct(lib, &mut p, &mut i);
                });
                a.push(ra);
                b.push(rbb);
            }

            a[0].assert_eq(&a[1], &format!("png_do_check_palette_indexes probes bd={bd} np={np}"));
            b[0].assert_eq(&b[1], &format!("png_do_check_palette_indexes write_end bd={bd} np={np}"));
        }
    }
}

// =============================================================== row 22

#[test]
fn r22_read_filter_row() {
    let lc = &libs().c;
    let lr = &libs().rs;
    // A plain reader is enough: png_read_filter_row only uses pp->read_filter[]
    // (lazily initialised from pp->pixel_depth, which is 0 on a fresh struct).
    let ppc = unsafe { api::new_reader(lc) };
    let ppr = unsafe { api::new_reader(lr) };
    let mut rng = Rng::new(0x2200_0022);
    let mut n = 0u32;

    for &pd in [1u8, 2, 4, 8, 16, 24, 32, 48, 64].iter() {
        for &filter in [-1i32, 0, 1, 2, 3, 4, 5].iter() {
            let bpp = ((pd as usize) + 7) >> 3;
            for rbytes in 1usize..=40 {
                // The C `avg` and `paeth` implementations compute
                // `istop = row_info->rowbytes - bpp` in `size_t`, which
                // underflows (and then runs off the end of memory) whenever
                // rowbytes < bpp.  libpng never produces such a row_info, so
                // those inputs are simply out of contract and are skipped.
                if (filter == PNG_FILTER_VALUE_AVG || filter == PNG_FILTER_VALUE_PAETH)
                    && rbytes < bpp
                {
                    continue;
                }
                let w = ((rbytes * 8) / (pd as usize)) as u32;
                let ri0 = png_row_info {
                    width: w,
                    rowbytes: rbytes,
                    color_type: 0,
                    bit_depth: if pd >= 8 { 8 } else { pd },
                    channels: 1,
                    pixel_depth: pd,
                };
                for it in 0..30u32 {
                    let row0 = rng.bytes(rbytes + 64);
                    let prev0 = rng.bytes(rbytes + 64);

                    let mut rowc = row0.clone();
                    let prevc = prev0.clone();
                    let mut ric = ri0;
                    let rc = capture(|| unsafe {
                        api::png_read_filter_row(
                            lc,
                            ppc,
                            &mut ric,
                            rowc.as_mut_ptr(),
                            prevc.as_ptr(),
                            filter,
                        )
                    });

                    let mut rowr = row0.clone();
                    let prevr = prev0.clone();
                    let mut rir = ri0;
                    let rr = capture(|| unsafe {
                        api::png_read_filter_row(
                            lr,
                            ppr,
                            &mut rir,
                            rowr.as_mut_ptr(),
                            prevr.as_ptr(),
                            filter,
                        )
                    });

                    let ctx = format!(
                        "png_read_filter_row: pd={pd} filter={filter} rowbytes={rbytes} w={w} \
                         it={it}\n  row : {}\n  prev: {}",
                        hex(&row0),
                        hex(&prev0)
                    );
                    diff(&ctx, &rc, &rowc, &ric, &rr, &rowr, &rir);
                    if prevc != prevr {
                        panic!(
                            "{ctx}\n  PREV_ROW DIFFERS\n  C : {}\n  Rs: {}",
                            hex(&prevc),
                            hex(&prevr)
                        );
                    }
                    assert_eq!(prevc, prev0, "{ctx}\n  C modified the const prev_row");
                    n += 1;
                }
            }
        }
    }

    assert_eq!(n, (9 * 7 * 40 - 2 * 18) * 30, "unexpected number of cases");

    for (lib, pp) in [(lc, ppc), (lr, ppr)] {
        let _ = capture(|| unsafe {
            let mut p = pp;
            api::png_destroy_read_struct(
                lib,
                &mut p,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        });
    }
}

// =============================================================== row 23

#[test]
fn r23_do_read_interlace() {
    let lc = &libs().c;
    let lr = &libs().rs;
    let mut rng = Rng::new(0x2300_0023);
    let mut n = 0u32;
    for pass in 0..7i32 {
        let inc = PASS_INC[pass as usize];
        for &ct in CTS.iter() {
            for &bd in BDS.iter() {
                let ch = chans(ct);
                let pd = bd * ch;
                for &tr in [0u32, PNG_PACKSWAP].iter() {
                    for w in 1..=33u32 {
                        let fw = w * inc;
                        // The function EXPANDS the row in place: size the buffer
                        // for the *output* row.
                        let cap = png_rowbytes(pd, fw) + 64;
                        let ri0 = png_row_info {
                            width: w,
                            rowbytes: png_rowbytes(pd, w),
                            color_type: ct,
                            bit_depth: bd,
                            channels: ch,
                            pixel_depth: pd,
                        };
                        for it in 0..2u32 {
                            let base = rng.bytes(cap);

                            let mut bc = base.clone();
                            let mut ric = ri0;
                            let rc = capture(|| unsafe {
                                api::png_do_read_interlace(lc, &mut ric, bc.as_mut_ptr(), pass, tr)
                            });

                            let mut br = base.clone();
                            let mut rir = ri0;
                            let rr = capture(|| unsafe {
                                api::png_do_read_interlace(lr, &mut rir, br.as_mut_ptr(), pass, tr)
                            });

                            let ctx = format!(
                                "png_do_read_interlace: pass={pass} ct={ct} bd={bd} ch={ch} \
                                 pd={pd} w={w} final_w={fw} transformations=0x{tr:x} it={it}\n  in: {}",
                                hex(&base)
                            );
                            diff(&ctx, &rc, &bc, &ric, &rr, &br, &rir);
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(n, 7 * 5 * 5 * 2 * 33 * 2, "unexpected number of cases");
}

// =============================================================== row 24

#[test]
fn r24_do_write_interlace() {
    let lc = &libs().c;
    let lr = &libs().rs;
    let mut rng = Rng::new(0x2400_0024);
    let mut n = 0u32;
    for pass in 0..7i32 {
        // pass 6 is a documented no-op
        for &ct in CTS.iter() {
            for &bd in BDS.iter() {
                let ch = chans(ct);
                let pd = bd * ch;
                for w in 1..=33u32 {
                    let rbytes = png_rowbytes(pd, w);
                    let ri0 = png_row_info {
                        width: w,
                        rowbytes: rbytes,
                        color_type: ct,
                        bit_depth: bd,
                        channels: ch,
                        pixel_depth: pd,
                    };
                    for it in 0..4u32 {
                        let base = rng.bytes(rbytes + 64);

                        let mut bc = base.clone();
                        let mut ric = ri0;
                        let rc = capture(|| unsafe {
                            api::png_do_write_interlace(lc, &mut ric, bc.as_mut_ptr(), pass)
                        });

                        let mut br = base.clone();
                        let mut rir = ri0;
                        let rr = capture(|| unsafe {
                            api::png_do_write_interlace(lr, &mut rir, br.as_mut_ptr(), pass)
                        });

                        let ctx = format!(
                            "png_do_write_interlace: pass={pass} ct={ct} bd={bd} ch={ch} pd={pd} \
                             w={w} rowbytes={rbytes} it={it}\n  in: {}",
                            hex(&base)
                        );
                        diff(&ctx, &rc, &bc, &ric, &rr, &br, &rir);
                        n += 1;
                    }
                }
            }
        }
    }
    assert_eq!(n, 7 * 5 * 5 * 33 * 4, "unexpected number of cases");
}

// =============================================================== row 25
//
// `png_combine_row` needs pp->row_buf / pp->pass / pp->width / pp->interlaced /
// pp->transformations / pp->row_info, none of which can be established through
// public setters, so it is exercised through its thin public wrapper
// `png_progressive_combine_row(pp, old_row, new_row)`
// (c_src/src/pngpread.c:907, which is literally
//  `if (new_row != NULL) png_combine_row(png_ptr, old_row, 1)`).
//
// The driver:
//   * builds a 7-pass Adam7 PNG with the **C** writer (so that both readers get
//     byte-identical input and the test cannot be perturbed by unrelated
//     deflate differences),
//   * decodes it progressively with each library; the info callback installs
//     `png_set_interlace_handling` + `png_read_update_info`,
//   * from inside every row callback it calls `png_progressive_combine_row`
//     into a pre-filled destination image and snapshots the *whole*
//     destination buffer,
//   * compares the snapshot sequence, the final image and the full sequence of
//     `(row_number, pass)` callback arguments.

struct ProgState {
    lib: Option<&'static Library>,
    dest: Vec<u8>,
    stride: usize,
    height: usize,
    calls: Vec<(png_uint_32, c_int)>,
    snaps: Vec<Vec<u8>>,
}

thread_local! {
    static PS: RefCell<ProgState> = const {
        RefCell::new(ProgState {
            lib: None,
            dest: Vec::new(),
            stride: 0,
            height: 0,
            calls: Vec::new(),
            snaps: Vec::new(),
        })
    };
}

unsafe extern "C-unwind" fn prog_info(pp: png_structp, ip: png_infop) {
    let lib = PS.with(|s| s.borrow().lib.unwrap());
    api::png_set_interlace_handling(lib, pp);
    // Allocates pp->row_buf / prev_row, which the progressive IDAT reader
    // requires before the first IDAT byte is pushed.
    api::png_read_update_info(lib, pp, ip);
}

unsafe extern "C-unwind" fn prog_row(
    pp: png_structp,
    new_row: png_bytep,
    row_num: png_uint_32,
    pass: c_int,
) {
    let (lib, ptr, stride, height) = PS.with(|s| {
        let s = s.borrow();
        (s.lib.unwrap(), s.dest.as_ptr() as *mut u8, s.stride, s.height)
    });
    PS.with(|s| s.borrow_mut().calls.push((row_num, pass)));
    if !new_row.is_null() && (row_num as usize) < height {
        let off = (row_num as usize) * stride;
        api::png_progressive_combine_row(lib, pp, ptr.add(off), new_row);
    }
    PS.with(|s| {
        let mut s = s.borrow_mut();
        let snap = s.dest.clone();
        s.snaps.push(snap);
    });
}

unsafe extern "C-unwind" fn prog_end(_pp: png_structp, _ip: png_infop) {}

/// Encode an Adam7-interlaced image with `lib`; returns the PNG bytes.
fn encode_adam7(
    lib: &'static Library,
    w: u32,
    h: u32,
    ct: c_int,
    bd: c_int,
    pal: Option<&[png_color]>,
    rows: &[Vec<u8>],
) -> Vec<u8> {
    let r = capture(|| unsafe {
        sink_reset();
        let pp = api::new_writer(lib);
        let ip = api::png_create_info_struct(lib, pp);
        assert!(!ip.is_null());
        api::png_set_IHDR(
            lib,
            pp,
            ip,
            w,
            h,
            bd,
            ct,
            PNG_INTERLACE_ADAM7,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        if let Some(p) = pal {
            api::png_set_PLTE(lib, pp, ip, p.as_ptr(), p.len() as c_int);
        }
        api::png_write_info(lib, pp, ip);
        let mut ptrs: Vec<png_bytep> =
            rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
        api::png_write_image(lib, pp, ptrs.as_mut_ptr());
        api::png_write_end(lib, pp, ip);
        let mut p = pp;
        let mut i = ip;
        api::png_destroy_write_struct(lib, &mut p, &mut i);
        sink_take().0
    });
    assert!(r.out.is_some(), "encode failed: {:?}", r.log.iter().map(|m| m.to_string()).collect::<Vec<_>>());
    r.out.unwrap()
}

type ProgOut = (Vec<u8>, Vec<(png_uint_32, c_int)>, Vec<Vec<u8>>);

fn decode_progressive(
    lib: &'static Library,
    png: &[u8],
    stride: usize,
    height: usize,
    fill: &[u8],
) -> Run<ProgOut> {
    PS.with(|s| {
        let mut s = s.borrow_mut();
        s.lib = Some(lib);
        s.dest = fill.to_vec();
        s.stride = stride;
        s.height = height;
        s.calls.clear();
        s.snaps.clear();
    });
    let mut data = png.to_vec();
    let r = capture(|| unsafe {
        let pp = api::new_reader(lib);
        let ip = api::png_create_info_struct(lib, pp);
        assert!(!ip.is_null());
        api::png_set_progressive_read_fn(
            lib,
            pp,
            1usize as png_voidp,
            Some(prog_info),
            Some(prog_row),
            Some(prog_end),
        );
        let mut off = 0usize;
        while off < data.len() {
            let n = core::cmp::min(13, data.len() - off);
            api::png_process_data(lib, pp, ip, data.as_mut_ptr().add(off), n);
            off += n;
        }
        let mut p = pp;
        api::png_destroy_read_struct(lib, &mut p, &mut (ip as *mut c_void), std::ptr::null_mut());
    });
    let out = PS.with(|s| {
        let s = s.borrow();
        (s.dest.clone(), s.calls.clone(), s.snaps.clone())
    });
    Run { out: r.out.map(|_| out), log: r.log }
}

#[test]
fn r25_combine_row_via_progressive_adam7() {
    let mut rng = Rng::new(0x2500_0025);
    let cfgs: [(u32, u32, c_int, c_int); 7] = [
        (17, 9, PNG_COLOR_TYPE_GRAY, 1),
        (33, 5, PNG_COLOR_TYPE_RGB, 8),
        (7, 7, PNG_COLOR_TYPE_RGB_ALPHA, 16),
        (13, 11, PNG_COLOR_TYPE_PALETTE, 4),
        (1, 1, PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (3, 2, PNG_COLOR_TYPE_RGB, 8),
        (9, 4, PNG_COLOR_TYPE_GRAY, 2),
    ];
    for (w, h, ct, bd) in cfgs {
        let ch = chans(ct as u8);
        let pd = (bd as u8) * ch;
        let stride = png_rowbytes(pd, w);
        let rows: Vec<Vec<u8>> = (0..h).map(|_| rng.bytes(stride)).collect();
        let pal: Option<Vec<png_color>> = if ct == PNG_COLOR_TYPE_PALETTE {
            Some(
                (0..(1usize << bd))
                    .map(|_| png_color { red: rng.u8(), green: rng.u8(), blue: rng.u8() })
                    .collect(),
            )
        } else {
            None
        };
        let fill = rng.bytes(stride * (h as usize));

        let png = encode_adam7(
            &libs().c,
            w,
            h,
            ct,
            bd,
            pal.as_deref(),
            &rows,
        );

        let rc = decode_progressive(&libs().c, &png, stride, h as usize, &fill);
        let rr = decode_progressive(&libs().rs, &png, stride, h as usize, &fill);

        let label = format!(
            "png_progressive_combine_row (png_combine_row): w={w} h={h} ct={ct} bd={bd} \
             stride={stride} png_len={}",
            png.len()
        );

        // Report the difference precisely before the generic assert_eq blows up
        // with a huge Debug dump.
        if let (Some(c), Some(r)) = (rc.out.as_ref(), rr.out.as_ref()) {
            assert_eq!(c.1, r.1, "{label}: (row_number, pass) callback sequence differs (C first)");
            if c.0 != r.0 {
                panic!(
                    "{label}: FINAL COMBINED IMAGE DIFFERS\n  C : {}\n  Rs: {}",
                    hex(&c.0),
                    hex(&r.0)
                );
            }
            for (i, (cs, rs)) in c.2.iter().zip(r.2.iter()).enumerate() {
                if cs != rs {
                    panic!(
                        "{label}: destination differs after callback #{i} {:?}\n  C : {}\n  Rs: {}",
                        c.1.get(i),
                        hex(cs),
                        hex(rs)
                    );
                }
            }
            assert_eq!(c.2.len(), r.2.len(), "{label}: number of row callbacks differs");
        }
        rc.assert_eq(&rr, &label);

        // Sanity: the C decode must actually have combined the whole image back
        // to the original rows, otherwise the test would be vacuous.  Only the
        // *meaningful* bits of the last byte of each row are defined (blocky
        // display combining replicates pixels into the padding bits).
        let got = &rc.out.as_ref().unwrap().0;
        let bits = (w as usize) * (pd as usize);
        let full = bits / 8;
        let trail = bits % 8;
        let mask = if trail == 0 { 0u8 } else { 0xffu8 << (8 - trail) };
        for y in 0..(h as usize) {
            let g = &got[y * stride..(y + 1) * stride];
            let e = &rows[y];
            assert_eq!(
                &g[..full],
                &e[..full],
                "{label}: harness bug - C round trip did not reproduce row {y}"
            );
            if trail != 0 {
                assert_eq!(
                    g[full] & mask,
                    e[full] & mask,
                    "{label}: harness bug - C round trip did not reproduce row {y} tail"
                );
            }
        }
    }
}
