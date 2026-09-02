//! Phase B, group RT: composed write→read pipelines, including CROSS
//! implementation round trips (write with C, read with Rust and vice versa) so
//! stream compatibility is proved, not just self-consistency.
mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

const SEED: u64 = 0x2717_0505_9090_3131;

const LEGAL: &[(c_int, c_int)] = &[
    (0, 1),
    (0, 2),
    (0, 4),
    (0, 8),
    (0, 16),
    (3, 1),
    (3, 2),
    (3, 4),
    (3, 8),
    (2, 8),
    (2, 16),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

fn pal_for(bd: c_int) -> usize {
    match bd {
        1 => 2,
        2 => 4,
        4 => 16,
        _ => 256,
    }
}

fn write_stream(
    l: &Lib,
    w: u32,
    h: u32,
    ct: c_int,
    bd: c_int,
    il: c_int,
    seed: u64,
    setup: &mut dyn FnMut(&Lib, *mut c_void, *mut c_void),
) -> Report {
    let pal = if ct == PNG_COLOR_TYPE_PALETTE {
        make_palette(pal_for(bd), seed ^ 0x5a5a)
    } else {
        vec![]
    };
    write_full(
        l,
        w,
        h,
        ct,
        bd,
        il,
        PNG_FILTER_TYPE_BASE,
        &pal,
        rowbytes(w, bd, ct),
        seed,
        setup,
    )
}

fn read_back(l: &Lib, stream: Vec<u8>) -> Report {
    read_session(l, stream, &mut |l, png, info| unsafe {
        (l.api.png_read_info)(png, info);
        let mut ow = 0u32;
        let mut oh = 0u32;
        let mut obd: c_int = 0;
        let mut oct: c_int = 0;
        let mut oil: c_int = 0;
        let mut ocm: c_int = 0;
        let mut ofm: c_int = 0;
        log(format!(
            "IHDR={} {ow}x{oh} bd={obd} ct={oct} il={oil} cm={ocm} fm={ofm}",
            (l.api.png_get_IHDR)(
                png, info, &mut ow, &mut oh, &mut obd, &mut oct, &mut oil, &mut ocm, &mut ofm
            )
        ));
        let passes = if oil == 1 {
            (l.api.png_set_interlace_handling)(png)
        } else {
            1
        };
        (l.api.png_read_update_info)(png, info);
        let rb = (l.api.png_get_rowbytes)(png, info);
        let mut rows: Vec<Vec<u8>> = (0..oh).map(|_| vec![0u8; rb + 8]).collect();
        for _ in 0..passes {
            for row in rows.iter_mut() {
                (l.api.png_read_row)(png, row.as_mut_ptr(), ptr::null_mut());
            }
        }
        for (i, row) in rows.iter().enumerate() {
            log(format!("row{i}={:02x?}", &row[..rb]));
        }
        (l.api.png_read_end)(png, info);
    })
}

// ---------------------------------------------------------------------------
// RT1/RT4 write then read, same and crossed implementations
// ---------------------------------------------------------------------------
#[test]
fn rt1_rt4_write_read_roundtrip() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let (w, h) = (19u32, 7u32);
            let seed = SEED ^ ((ct as u64) << 24) ^ ((bd as u64) << 16) ^ ((il as u64) << 8);

            // Both writers must emit identical bytes.
            let mut wrun =
                |l: &Lib| -> Report { write_stream(l, w, h, ct, bd, il, seed, &mut no_setup) };
            diff(
                &format!("RT1 write ct={ct} bd={bd} il={il}"),
                &c,
                &r,
                &mut wrun,
            );
            let sc = wrun(&c).out;
            let sr = wrun(&r).out;
            assert_eq!(sc, sr, "streams differ");

            // Same-implementation read back.
            let mut rrun = |l: &Lib| -> Report { read_back(l, sc.clone()) };
            diff(
                &format!("RT1 read-back ct={ct} bd={bd} il={il}"),
                &c,
                &r,
                &mut rrun,
            );

            // Cross: C stream -> Rust reader must match Rust stream -> C reader.
            let cross_a = read_back(&r, sc.clone());
            let cross_b = read_back(&c, sr.clone());
            assert_eq!(
                cross_a, cross_b,
                "RT4 cross round trip mismatch ct={ct} bd={bd} il={il}\n  Rust-reads-C : {}\n  C-reads-Rust : {}",
                cross_a.brief(),
                cross_b.brief()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// RT2 randomized writer configurations, then read back
// ---------------------------------------------------------------------------
#[test]
fn rt2_random_write_configs() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0x2222);
    for i in 0..64u64 {
        let (ct, bd) = LEGAL[(rng.below(LEGAL.len() as u32)) as usize];
        let il = if rng.u32() & 1 == 0 {
            PNG_INTERLACE_NONE
        } else {
            PNG_INTERLACE_ADAM7
        };
        let w = 1 + rng.below(40);
        let h = 1 + rng.below(12);
        let level = rng.below(10) as c_int;
        let strategy = rng.below(5) as c_int;
        let wb = 8 + rng.below(8) as c_int;
        let ml = 1 + rng.below(9) as c_int;
        let bs = 1usize << (1 + rng.below(14));
        let mask = [
            PNG_NO_FILTERS,
            PNG_FILTER_NONE,
            PNG_FILTER_SUB,
            PNG_FILTER_UP,
            PNG_FILTER_AVG,
            PNG_FILTER_PAETH,
            PNG_ALL_FILTERS,
            PNG_FILTER_SUB | PNG_FILTER_PAETH,
        ][(rng.below(8)) as usize];
        let seed = SEED ^ 0x2222 ^ i;
        let mut wrun = |l: &Lib| -> Report {
            write_stream(l, w, h, ct, bd, il, seed, &mut |l, png, _info| unsafe {
                (l.api.png_set_compression_level)(png, level);
                (l.api.png_set_compression_strategy)(png, strategy);
                (l.api.png_set_compression_window_bits)(png, wb);
                (l.api.png_set_compression_mem_level)(png, ml);
                (l.api.png_set_compression_buffer_size)(png, bs);
                (l.api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, mask);
            })
        };
        let label = format!(
            "RT2 #{i} ct={ct} bd={bd} il={il} {w}x{h} lvl={level} st={strategy} wb={wb} ml={ml} bs={bs} f={mask:#x}"
        );
        diff(&label, &c, &r, &mut wrun);
        let s = wrun(&c).out;
        let mut rrun = |l: &Lib| -> Report { read_back(l, s.clone()) };
        diff(&format!("{label} (read-back)"), &c, &r, &mut rrun);
    }
}

// ---------------------------------------------------------------------------
// RT5 png_write_png -> png_read_png with matching transform masks
// ---------------------------------------------------------------------------
#[test]
fn rt5_write_png_read_png() {
    let (c, r) = libs();
    let (w, h) = (14u32, 6u32);
    let masks: &[c_int] = &[
        PNG_TRANSFORM_IDENTITY,
        PNG_TRANSFORM_PACKING,
        PNG_TRANSFORM_PACKSWAP,
        PNG_TRANSFORM_BGR,
        PNG_TRANSFORM_SWAP_ENDIAN,
        PNG_TRANSFORM_INVERT_MONO,
        PNG_TRANSFORM_SWAP_ALPHA,
        PNG_TRANSFORM_INVERT_ALPHA,
        PNG_TRANSFORM_PACKING | PNG_TRANSFORM_PACKSWAP,
        PNG_TRANSFORM_BGR | PNG_TRANSFORM_INVERT_ALPHA,
    ];
    for &(ct, bd) in LEGAL {
        for &wm in masks {
            let bps = ((bd as usize) / 8).max(1);
            let rb = rowbytes(w, bd, ct).max(w as usize * (channels(ct) + 1) * bps);
            let pal = if ct == PNG_COLOR_TYPE_PALETTE {
                make_palette(pal_for(bd), SEED ^ 0x5555)
            } else {
                vec![]
            };
            let seed = SEED ^ 0x5555 ^ ((ct as u64) << 8) ^ (bd as u64) ^ ((wm as u64) << 20);
            let mut wrun = |l: &Lib| -> Report {
                let rows = make_rows(h as usize, rb, seed);
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, w, h, bd, ct, PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    if !pal.is_empty() {
                        (l.api.png_set_PLTE)(png, info, pal.as_ptr(), pal.len() as c_int);
                    }
                    let sig = PngColor8 { red: 4, green: 4, blue: 4, gray: 4, alpha: 4 };
                    (l.api.png_set_sBIT)(png, info, &sig);
                    let mut ptrs: Vec<*mut u8> =
                        rows.iter().map(|v| v.as_ptr() as *mut u8).collect();
                    (l.api.png_set_rows)(png, info, ptrs.as_mut_ptr());
                    (l.api.png_write_png)(png, info, wm, ptr::null_mut());
                })
            };
            diff(
                &format!("RT5 write_png mask={wm:#x} ct={ct} bd={bd}"),
                &c,
                &r,
                &mut wrun,
            );
            let s = wrun(&c).out;
            if s.is_empty() {
                continue;
            }
            for &rm in masks {
                let mut rrun = |l: &Lib| -> Report {
                    let cap = w as usize * 16 + 32;
                    let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; cap]).collect();
                    let mut ptrs: Vec<*mut u8> =
                        rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                    read_session(l, s.clone(), &mut |l, png, info| unsafe {
                        (l.api.png_set_rows)(png, info, ptrs.as_mut_ptr());
                        (l.api.png_read_png)(png, info, rm, ptr::null_mut());
                        let rbn = (l.api.png_get_rowbytes)(png, info);
                        log(format!("rb={rbn}"));
                        let got = (l.api.png_get_rows)(png, info);
                        if !got.is_null() {
                            for i in 0..h as usize {
                                let p = *got.add(i);
                                if !p.is_null() {
                                    log(format!(
                                        "row{i}={:02x?}",
                                        std::slice::from_raw_parts(p, rbn)
                                    ));
                                }
                            }
                        }
                    })
                };
                diff(
                    &format!("RT5 read_png w={wm:#x} r={rm:#x} ct={ct} bd={bd}"),
                    &c,
                    &r,
                    &mut rrun,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RT6/RT7 simplified <-> low-level round trips
// ---------------------------------------------------------------------------
fn s_channels(fmt: u32) -> u32 {
    (fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
}
fn s_comp(fmt: u32) -> u32 {
    ((fmt & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1
}
fn p_channels(fmt: u32) -> u32 {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        s_channels(fmt)
    }
}
fn p_comp(fmt: u32) -> u32 {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        s_comp(fmt)
    }
}
fn img_size(fmt: u32, w: u32, h: u32) -> usize {
    p_comp(fmt) as usize * h as usize * (p_channels(fmt) * w) as usize
}

#[test]
fn rt6_rt7_simplified_roundtrip() {
    let (c, r) = libs();
    let (w, h) = (10u32, 6u32);
    for fmt in 0u32..0x40 {
        for conv in [0i32, 1] {
            let cmap_entries = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 { 32 } else { 0 };
            let mut rng = Rng::new(SEED ^ 0x6666 ^ (fmt as u64) ^ ((conv as u64) << 8));
            let src = rng.bytes(img_size(fmt, w, h).max(1) + 64);
            let cmap =
                rng.bytes((s_channels(fmt) * s_comp(fmt) * cmap_entries.max(1)) as usize + 64);

            // simplified write
            let mut wrun = |l: &Lib| -> Report {
                let src = src.clone();
                let cmap = cmap.clone();
                scratch(&mut || unsafe {
                    let mut im = PngImage {
                        version: PNG_IMAGE_VERSION,
                        width: w,
                        height: h,
                        format: fmt,
                        colormap_entries: cmap_entries,
                        ..Default::default()
                    };
                    let mut out = vec![0u8; 1 << 18];
                    let mut sz = out.len();
                    let ok = (l.api.png_image_write_to_memory)(
                        &mut im,
                        out.as_mut_ptr() as *mut c_void,
                        &mut sz,
                        conv,
                        src.as_ptr() as *mut c_void,
                        0,
                        if cmap_entries > 0 {
                            cmap.as_ptr() as *mut c_void
                        } else {
                            ptr::null_mut()
                        },
                    );
                    log(format!("write ok={ok} sz={sz} woe={}", im.warning_or_error));
                    if ok != 0 {
                        out.truncate(sz);
                        // record the stream for the reader
                        ctx_out(&out);
                    }
                    (l.api.png_image_free)(&mut im);
                })
            };
            diff(
                &format!("RT6 simplified write fmt={fmt:#x} conv={conv}"),
                &c,
                &r,
                &mut wrun,
            );
            let stream = wrun(&c).out;
            if stream.is_empty() {
                continue;
            }
            // RT7: read the simplified-written stream with the low-level API
            let mut rrun = |l: &Lib| -> Report { read_back(l, stream.clone()) };
            diff(
                &format!("RT7 low-level read of simplified stream fmt={fmt:#x} conv={conv}"),
                &c,
                &r,
                &mut rrun,
            );
            // RT6: read it back with the simplified reader in the same format
            let mut srun = |l: &Lib| -> Report {
                scratch(&mut || unsafe {
                    let mut im = PngImage::default();
                    let ok = (l.api.png_image_begin_read_from_memory)(
                        &mut im,
                        stream.as_ptr() as *const c_void,
                        stream.len(),
                    );
                    log(format!("begin ok={ok} fmt={:#x}", im.format));
                    if ok != 0 {
                        im.format = fmt;
                        let sz = img_size(fmt, im.width, im.height).max(1);
                        let mut buf = vec![0u8; sz + 128];
                        let cm = (s_channels(fmt) * s_comp(fmt) * 256) as usize;
                        let mut cmo = vec![0u8; cm + 128];
                        let ok2 = (l.api.png_image_finish_read)(
                            &mut im,
                            ptr::null(),
                            buf.as_mut_ptr() as *mut c_void,
                            0,
                            cmo.as_mut_ptr() as *mut c_void,
                        );
                        log_img(&format!("finish ok={ok2}"), &im);
                        if ok2 != 0 {
                            log(format!("pixels={:02x?}", &buf[..sz]));
                        }
                    }
                    (l.api.png_image_free)(&mut im);
                })
            };
            diff(
                &format!("RT6 simplified read-back fmt={fmt:#x} conv={conv}"),
                &c,
                &r,
                &mut srun,
            );
        }
    }
}

fn scratch(f: &mut dyn FnMut()) -> Report {
    let mut ctxb = Box::new(Ctx::default());
    set_ctx(&mut *ctxb as *mut Ctx);
    f();
    let rep = ctxb.digest();
    set_ctx(ptr::null_mut());
    rep
}

fn ctx_out(b: &[u8]) {
    out_extend(b);
}

// ---------------------------------------------------------------------------
// RT8 write -> progressive read
// ---------------------------------------------------------------------------
#[test]
fn rt8_write_then_progressive_read() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let seed = SEED ^ 0x8888 ^ ((ct as u64) << 8) ^ (bd as u64) ^ ((il as u64) << 16);
            let stream = write_stream(&c, 15, 6, ct, bd, il, seed, &mut no_setup).out;
            for gran in [1usize, 7, 0] {
                let mut run = |l: &Lib| -> Report {
                    read_session(l, vec![], &mut |l, png, info| unsafe {
                        (l.api.png_set_progressive_read_fn)(
                            png,
                            ptr::null_mut(),
                            rt_info_cb as *mut c_void,
                            rt_row_cb as *mut c_void,
                            rt_end_cb as *mut c_void,
                        );
                        let mut pos = 0usize;
                        while pos < stream.len() {
                            let n = if gran == 0 {
                                stream.len() - pos
                            } else {
                                gran.min(stream.len() - pos)
                            };
                            (l.api.png_process_data)(
                                png,
                                info,
                                stream[pos..].as_ptr() as *mut u8,
                                n,
                            );
                            pos += n;
                        }
                    })
                };
                diff(
                    &format!("RT8 write->progressive ct={ct} bd={bd} il={il} gran={gran}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

unsafe extern "C" fn rt_info_cb(_png: *mut c_void, _info: *mut c_void) {
    log("rt info");
}
unsafe extern "C" fn rt_row_cb(_png: *mut c_void, row: *mut u8, n: u32, p: c_int) {
    if row.is_null() {
        log(format!("rt row {n} {p} null"));
    } else {
        log(format!("rt row {n} {p}"));
    }
}
unsafe extern "C" fn rt_end_cb(_png: *mut c_void, _info: *mut c_void) {
    log("rt end");
}
