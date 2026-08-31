//! Tier 10: randomised differential fuzzing.  A deterministic PRNG drives
//! random encoder settings, random read-transform combinations and random byte
//! mutations of otherwise valid files.  Everything observable must still match.

mod common;
use common::*;
use std::ffi::{c_char, c_int, CString};

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    fn next(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
    fn flip(&mut self) -> bool {
        self.next() & 1 == 1
    }
    fn byte(&mut self) -> u8 {
        (self.next() >> 24) as u8
    }
}

/// Iteration counts scale with PNG_FUZZ_SCALE so a longer hunt can be run on
/// demand without changing the default test runtime.
fn scale() -> u64 {
    std::env::var("PNG_FUZZ_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .max(1)
}

fn rowbytes(pixel_depth: u32, width: u32) -> usize {
    if pixel_depth >= 8 {
        (width as usize) * ((pixel_depth as usize) >> 3)
    } else {
        ((width as usize) * (pixel_depth as usize) + 7) >> 3
    }
}

fn channels(ct: u8) -> u32 {
    match ct {
        PNG_COLOR_TYPE_GRAY | PNG_COLOR_TYPE_PALETTE => 1,
        PNG_COLOR_TYPE_GRAY_ALPHA => 2,
        PNG_COLOR_TYPE_RGB => 3,
        PNG_COLOR_TYPE_RGB_ALPHA => 4,
        _ => 4,
    }
}

fn random_format(r: &mut Rng) -> (u8, u8) {
    loop {
        let ct = r.pick(&[
            PNG_COLOR_TYPE_GRAY,
            PNG_COLOR_TYPE_PALETTE,
            PNG_COLOR_TYPE_RGB,
            PNG_COLOR_TYPE_GRAY_ALPHA,
            PNG_COLOR_TYPE_RGB_ALPHA,
        ]);
        let bd = r.pick(&[1u8, 2, 4, 8, 16]);
        let ok = match ct {
            PNG_COLOR_TYPE_GRAY => true,
            PNG_COLOR_TYPE_PALETTE => bd <= 8,
            _ => bd == 8 || bd == 16,
        };
        if ok {
            return (ct, bd);
        }
    }
}

/// Random encoder configuration, applied identically to both libraries.
struct EncCfg {
    w: u32,
    h: u32,
    ct: u8,
    bd: u8,
    il: c_int,
    level: c_int,
    strategy: c_int,
    window_bits: c_int,
    mem_level: c_int,
    filters: c_int,
    gama: Option<i32>,
    srgb: Option<c_int>,
    sbit: bool,
    trns: bool,
    bkgd: bool,
    phys: bool,
    offs: bool,
    time: bool,
    text: u32,
    unknown: u32,
    rows: Vec<Vec<u8>>,
}

fn random_enc(r: &mut Rng) -> EncCfg {
    let (ct, bd) = random_format(r);
    let w = 1 + r.below(24) as u32;
    let h = 1 + r.below(8) as u32;
    let rb = rowbytes(channels(ct) * bd as u32, w);
    let rows: Vec<Vec<u8>> = (0..h).map(|_| (0..rb).map(|_| r.byte()).collect()).collect();
    EncCfg {
        w,
        h,
        ct,
        bd,
        il: if r.flip() { PNG_INTERLACE_ADAM7 } else { PNG_INTERLACE_NONE },
        level: r.pick(&[-1i32, 0, 1, 3, 6, 9]),
        strategy: r.pick(&[0i32, 1, 2, 3, 4]),
        window_bits: r.pick(&[8i32, 9, 11, 13, 15]),
        mem_level: r.pick(&[1i32, 4, 8, 9]),
        filters: r.pick(&[
            PNG_NO_FILTERS,
            PNG_ALL_FILTERS,
            PNG_FILTER_NONE,
            PNG_FILTER_SUB,
            PNG_FILTER_UP,
            PNG_FILTER_AVG,
            PNG_FILTER_PAETH,
            PNG_FILTER_SUB | PNG_FILTER_UP,
            PNG_FILTER_AVG | PNG_FILTER_PAETH,
        ]),
        gama: if r.flip() { Some(r.pick(&[1i32, 45455, 100000, 220000, 1000000])) } else { None },
        srgb: if r.below(4) == 0 { Some(r.pick(&[0i32, 1, 2, 3])) } else { None },
        sbit: r.flip(),
        trns: r.flip(),
        bkgd: r.flip(),
        phys: r.flip(),
        offs: r.flip(),
        time: r.flip(),
        text: r.below(4) as u32,
        unknown: r.below(3) as u32,
        rows,
    }
}

fn encode_cfg(lib: &Lib, cfg: &EncCfg) -> WriteOutcome {
    write_with(lib, |c, _| {
        let png = c.png;
        let info = c.info;
        let mut keep: Vec<CString> = Vec::new();

        {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_compression_level");
            unsafe { f(png, cfg.level) };
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_compression_strategy");
            unsafe { f(png, cfg.strategy) };
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_compression_window_bits");
            unsafe { f(png, cfg.window_bits) };
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_compression_mem_level");
            unsafe { f(png, cfg.mem_level) };
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int, c_int)> =
                c.sym("png_set_filter");
            unsafe { f(png, 0, cfg.filters) };
        }
        type Fihdr = unsafe extern "C-unwind" fn(
            png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int,
        );
        let f: libloading::Symbol<Fihdr> = c.sym("png_set_IHDR");
        unsafe {
            f(png, info, cfg.w, cfg.h, cfg.bd as c_int, cfg.ct as c_int, cfg.il,
              PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE)
        };

        let npal = 1usize << cfg.bd.min(8);
        if cfg.ct == PNG_COLOR_TYPE_PALETTE {
            let pal: Vec<png_color> = (0..npal)
                .map(|i| png_color {
                    red: (i * 11 % 256) as u8,
                    green: (i * 17 % 256) as u8,
                    blue: (i * 23 % 256) as u8,
                })
                .collect();
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color, c_int),
            > = c.sym("png_set_PLTE");
            unsafe { g(png, info, pal.as_ptr(), npal as c_int) };
        }
        if let Some(gv) = cfg.gama {
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, i32)> =
                c.sym("png_set_gAMA_fixed");
            unsafe { g(png, info, gv) };
        }
        if let Some(s) = cfg.srgb {
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, c_int)> =
                c.sym("png_set_sRGB");
            unsafe { g(png, info, s) };
        }
        if cfg.sbit {
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color_8),
            > = c.sym("png_set_sBIT");
            let b = cfg.bd.min(8);
            let sb = match cfg.ct {
                PNG_COLOR_TYPE_GRAY => png_color_8 { gray: b, ..Default::default() },
                PNG_COLOR_TYPE_PALETTE => {
                    png_color_8 { red: 8, green: 8, blue: 8, ..Default::default() }
                }
                PNG_COLOR_TYPE_RGB => png_color_8 { red: b, green: b, blue: b, ..Default::default() },
                PNG_COLOR_TYPE_GRAY_ALPHA => {
                    png_color_8 { gray: b, alpha: b, ..Default::default() }
                }
                _ => png_color_8 { red: b, green: b, blue: b, alpha: b, ..Default::default() },
            };
            unsafe { g(png, info, &sb) };
        }
        let maxv = if cfg.bd == 16 { 65535u16 } else { (1u16 << cfg.bd) - 1 };
        if cfg.trns && (cfg.ct & PNG_COLOR_MASK_ALPHA) == 0 {
            type Ft = unsafe extern "C-unwind" fn(
                png_structp, png_infop, *const u8, c_int, *const png_color_16,
            );
            let g: libloading::Symbol<Ft> = c.sym("png_set_tRNS");
            if cfg.ct == PNG_COLOR_TYPE_PALETTE {
                let trans: Vec<u8> = (0..npal).map(|i| (i * 13 % 256) as u8).collect();
                unsafe { g(png, info, trans.as_ptr(), npal as c_int, std::ptr::null()) };
            } else if (cfg.ct & PNG_COLOR_MASK_COLOR) != 0 {
                let tc = png_color_16 { red: 1, green: 2, blue: 3, ..Default::default() };
                unsafe { g(png, info, std::ptr::null(), 0, &tc) };
            } else {
                let tc = png_color_16 { gray: maxv / 3, ..Default::default() };
                unsafe { g(png, info, std::ptr::null(), 0, &tc) };
            }
        }
        if cfg.bkgd {
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color_16),
            > = c.sym("png_set_bKGD");
            let bg = if cfg.ct == PNG_COLOR_TYPE_PALETTE {
                png_color_16 { index: (npal as u16 - 1).min(2) as u8, ..Default::default() }
            } else if (cfg.ct & PNG_COLOR_MASK_COLOR) != 0 {
                png_color_16 { red: maxv / 2, green: maxv / 4, blue: maxv / 8, ..Default::default() }
            } else {
                png_color_16 { gray: maxv / 2, ..Default::default() }
            };
            unsafe { g(png, info, &bg) };
        }
        if cfg.phys {
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, u32, u32, c_int),
            > = c.sym("png_set_pHYs");
            unsafe { g(png, info, 2540, 2540, 1) };
        }
        if cfg.offs {
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, i32, i32, c_int),
            > = c.sym("png_set_oFFs");
            unsafe { g(png, info, -5, 5, 1) };
        }
        if cfg.time {
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_time),
            > = c.sym("png_set_tIME");
            let t = png_time { year: 2001, month: 9, day: 9, hour: 1, minute: 46, second: 40 };
            unsafe { g(png, info, &t) };
        }
        if cfg.text > 0 {
            let mut texts = Vec::new();
            let mut owned = Vec::new();
            for i in 0..cfg.text {
                let key = CString::new(format!("Key{i}")).unwrap();
                let txt = CString::new("value ".repeat(1 + i as usize * 12)).unwrap();
                owned.push((key, txt));
            }
            for (i, (key, txt)) in owned.iter().enumerate() {
                texts.push(png_text {
                    compression: if i % 2 == 0 {
                        PNG_TEXT_COMPRESSION_NONE
                    } else {
                        PNG_TEXT_COMPRESSION_zTXt
                    },
                    key: key.as_ptr() as *mut c_char,
                    text: txt.as_ptr() as *mut c_char,
                    text_length: 0,
                    itxt_length: 0,
                    lang: std::ptr::null_mut(),
                    lang_key: std::ptr::null_mut(),
                });
            }
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_text, c_int),
            > = c.sym("png_set_text");
            unsafe { g(png, info, texts.as_ptr(), texts.len() as c_int) };
            for (k, t) in owned {
                keep.push(k);
                keep.push(t);
            }
        }
        if cfg.unknown > 0 {
            const PNG_HAVE_IHDR: u8 = 0x01;
            let payloads: Vec<Vec<u8>> =
                (0..cfg.unknown).map(|i| vec![(i * 37) as u8; 1 + i as usize * 5]).collect();
            let chunks: Vec<png_unknown_chunk> = payloads
                .iter()
                .enumerate()
                .map(|(i, p)| png_unknown_chunk {
                    name: [b'u', b'n', b'K', b'0' + i as u8, 0],
                    data: p.as_ptr() as *mut u8,
                    size: p.len(),
                    location: PNG_HAVE_IHDR,
                })
                .collect();
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(
                    png_structp,
                    png_infop,
                    *const png_unknown_chunk,
                    c_int,
                ),
            > = c.sym("png_set_unknown_chunks");
            unsafe { g(png, info, chunks.as_ptr(), chunks.len() as c_int) };
        }

        c.call2("png_write_info");
        let mut ptrs: Vec<*mut u8> = cfg.rows.iter().map(|x| x.as_ptr() as *mut u8).collect();
        let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32)> =
            c.sym("png_write_image");
        unsafe { g(png, ptrs.as_mut_ptr(), cfg.h) };
        c.call2("png_write_end");
        drop(keep);
    })
}

/// Random read-side configuration.
struct DecCfg {
    calls: Vec<&'static str>,
    gamma: Option<(i32, i32)>,
    background: Option<(c_int, i32)>,
    alpha_mode: Option<(c_int, i32)>,
    rgb_to_gray: Option<(c_int, i32, i32)>,
    filler: Option<(u32, c_int)>,
    quantize: Option<c_int>,
    shift: bool,
    interlace_handling: bool,
    update_info: bool,
    whole: bool,
    benign: bool,
    keep_unknown: Option<c_int>,
    crc_action: Option<(c_int, c_int)>,
}

fn random_dec(r: &mut Rng) -> DecCfg {
    const SIMPLE: [&str; 15] = [
        "png_set_expand",
        "png_set_palette_to_rgb",
        "png_set_expand_gray_1_2_4_to_8",
        "png_set_tRNS_to_alpha",
        "png_set_expand_16",
        "png_set_gray_to_rgb",
        "png_set_strip_16",
        "png_set_scale_16",
        "png_set_strip_alpha",
        "png_set_swap_alpha",
        "png_set_invert_alpha",
        "png_set_packing",
        "png_set_packswap",
        "png_set_bgr",
        "png_set_swap",
    ];
    let mut calls = Vec::new();
    for name in SIMPLE {
        if r.below(4) == 0 {
            calls.push(name);
        }
    }
    if r.below(6) == 0 {
        calls.push("png_set_invert_mono");
    }
    DecCfg {
        calls,
        gamma: if r.below(3) == 0 {
            Some((r.pick(&[45455i32, 100000, 220000]), r.pick(&[45455i32, 100000, 220000])))
        } else {
            None
        },
        background: if r.below(5) == 0 {
            Some((
                r.pick(&[
                    PNG_BACKGROUND_GAMMA_SCREEN,
                    PNG_BACKGROUND_GAMMA_FILE,
                    PNG_BACKGROUND_GAMMA_UNIQUE,
                ]),
                100000,
            ))
        } else {
            None
        },
        alpha_mode: if r.below(5) == 0 {
            Some((
                r.pick(&[PNG_ALPHA_PNG, PNG_ALPHA_STANDARD, PNG_ALPHA_BROKEN, PNG_ALPHA_OPTIMIZED]),
                r.pick(&[45455i32, 100000, 220000]),
            ))
        } else {
            None
        },
        rgb_to_gray: if r.below(6) == 0 {
            Some((r.pick(&[1i32, 2]), r.pick(&[-1i32, 21260]), r.pick(&[-1i32, 71520])))
        } else {
            None
        },
        filler: if r.below(6) == 0 {
            Some((r.below(65536) as u32, r.pick(&[PNG_FILLER_BEFORE, PNG_FILLER_AFTER])))
        } else {
            None
        },
        quantize: if r.below(8) == 0 { Some(r.pick(&[2i32, 16, 64, 216, 256])) } else { None },
        shift: r.below(6) == 0,
        interlace_handling: r.flip(),
        update_info: r.below(4) != 0,
        whole: r.flip(),
        benign: r.flip(),
        keep_unknown: if r.flip() {
            Some(r.pick(&[
                PNG_HANDLE_CHUNK_AS_DEFAULT,
                PNG_HANDLE_CHUNK_NEVER,
                PNG_HANDLE_CHUNK_IF_SAFE,
                PNG_HANDLE_CHUNK_ALWAYS,
            ]))
        } else {
            None
        },
        crc_action: if r.below(4) == 0 {
            Some((
                r.pick(&[
                    PNG_CRC_DEFAULT,
                    PNG_CRC_ERROR_QUIT,
                    PNG_CRC_WARN_DISCARD,
                    PNG_CRC_WARN_USE,
                    PNG_CRC_QUIET_USE,
                    PNG_CRC_NO_CHANGE,
                ]),
                r.pick(&[PNG_CRC_DEFAULT, PNG_CRC_ERROR_QUIT, PNG_CRC_WARN_USE, PNG_CRC_QUIET_USE]),
            ))
        } else {
            None
        },
    }
}

fn decode_cfg(lib: &Lib, data: &[u8], cfg: &DecCfg) -> ReadOutcome {
    read_with(lib, data, |c, out| {
        let png = c.png;
        if cfg.benign {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_benign_errors");
            unsafe { f(png, 1) };
        }
        if let Some(k) = cfg.keep_unknown {
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, c_int, *const u8, c_int),
            > = c.sym("png_set_keep_unknown_chunks");
            unsafe { f(png, k, std::ptr::null(), 0) };
        }
        if let Some((a, b)) = cfg.crc_action {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int, c_int)> =
                c.sym("png_set_crc_action");
            unsafe { f(png, a, b) };
        }

        c.call2("png_read_info");
        out.notes.extend(snapshot_info(c));

        for name in &cfg.calls {
            c.call1(name);
        }
        if cfg.shift {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *const png_color_8)> =
                c.sym("png_set_shift");
            let sb = png_color_8 { red: 3, green: 3, blue: 3, gray: 3, alpha: 3 };
            unsafe { f(png, &sb) };
        }
        if let Some((s, f)) = cfg.gamma {
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, i32, i32)> =
                c.sym("png_set_gamma_fixed");
            unsafe { g(png, s, f) };
        }
        if let Some((k, gv)) = cfg.background {
            type F = unsafe extern "C-unwind" fn(
                png_structp, *const png_color_16, c_int, c_int, i32,
            );
            let g: libloading::Symbol<F> = c.sym("png_set_background_fixed");
            let bg = png_color_16 { index: 1, red: 100, green: 150, blue: 200, gray: 120 };
            unsafe { g(png, &bg, k, 0, gv) };
        }
        if let Some((m, gv)) = cfg.alpha_mode {
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int, i32)> =
                c.sym("png_set_alpha_mode_fixed");
            unsafe { g(png, m, gv) };
        }
        if let Some((e, rr, gg)) = cfg.rgb_to_gray {
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, c_int, i32, i32),
            > = c.sym("png_set_rgb_to_gray_fixed");
            unsafe { g(png, e, rr, gg) };
        }
        if let Some((v, loc)) = cfg.filler {
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, u32, c_int)> =
                c.sym("png_set_filler");
            unsafe { g(png, v, loc) };
        }
        if let Some(n) = cfg.quantize {
            let pal: Vec<png_color> = (0..216u32)
                .map(|i| png_color {
                    red: ((i / 36) * 51) as u8,
                    green: (((i / 6) % 6) * 51) as u8,
                    blue: ((i % 6) * 51) as u8,
                })
                .collect();
            let hist: Vec<u16> = (0..216u16).map(|i| i * 5).collect();
            type F = unsafe extern "C-unwind" fn(
                png_structp, *mut png_color, c_int, c_int, *const u16, c_int,
            );
            let g: libloading::Symbol<F> = c.sym("png_set_quantize");
            unsafe {
                g(png, pal.as_ptr() as *mut png_color, 216, n, hist.as_ptr(), 1)
            };
        }

        let passes = if cfg.interlace_handling {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> c_int> =
                c.sym("png_set_interlace_handling");
            unsafe { f(png) }
        } else {
            1
        };

        if cfg.update_info {
            c.call2("png_read_update_info");
            out.notes.extend(snapshot_info(c));
        } else {
            c.call1("png_start_read_image");
        }

        let rb: usize = {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> usize> =
                c.sym("png_get_rowbytes");
            unsafe { f(png, c.info) }
        };
        let height: u32 = {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> u32> =
                c.sym("png_get_image_height");
            unsafe { f(png, c.info) }
        };
        let width: u32 = {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> u32> =
                c.sym("png_get_image_width");
            unsafe { f(png, c.info) }
        };
        out.notes.push(format!("rb={rb} h={height} passes={passes}"));
        // Without png_read_update_info the reported rowbytes is the *untransformed*
        // size, so bound the buffer by the widest possible transformed row
        // (four channels of 16 bits per pixel).
        let cap = rb.max(width as usize * 8 + 8) + 64;
        let mut bufs: Vec<Vec<u8>> = (0..height).map(|_| vec![0x5au8; cap]).collect();
        if cfg.whole && passes == 1 {
            let mut ptrs: Vec<*mut u8> = bufs.iter_mut().map(|b| b.as_mut_ptr()).collect();
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8)> =
                c.sym("png_read_image");
            unsafe { f(png, ptrs.as_mut_ptr()) };
        } else {
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, *mut u8, *mut u8),
            > = c.sym("png_read_row");
            for _ in 0..passes {
                for b in bufs.iter_mut() {
                    unsafe { f(png, b.as_mut_ptr(), std::ptr::null_mut()) };
                }
            }
        }
        out.rows = bufs;
        c.call2("png_read_end");
        out.notes.extend(snapshot_info(c));
    })
}

fn compare(label: &str, a: &ReadOutcome, b: &ReadOutcome) {
    assert_eq!(a.diag, b.diag, "{label}: diagnostics differ");
    assert_eq!(a.errored, b.errored, "{label}: error flag differs");
    let n = a.notes.len().min(b.notes.len());
    assert_snapshots_eq(label, &a.notes[..n], &b.notes[..n]);
    assert_eq!(a.notes.len(), b.notes.len(), "{label}: note count differs");
    assert_eq!(a.rows.len(), b.rows.len(), "{label}: row count differs");
    for (i, (x, y)) in a.rows.iter().zip(b.rows.iter()).enumerate() {
        assert_eq!(x, y, "{label}: row {i} differs\n C: {}\n R: {}", hex(x), hex(y));
    }
}

#[test]
fn fuzz_encode_decode() {
    let l = libs();
    for seed in 0..600u64 * scale() {
        let mut r = Rng::new(0x5eed_0000 + seed);
        let enc = random_enc(&mut r);
        let ea = encode_cfg(&l.c, &enc);
        let eb = encode_cfg(&l.r, &enc);
        let ctx = format!(
            "seed={seed} {}x{} ct={} bd={} il={} lvl={} strat={} wb={} ml={} filt={:#x}",
            enc.w, enc.h, enc.ct, enc.bd, enc.il, enc.level, enc.strategy, enc.window_bits,
            enc.mem_level, enc.filters
        );
        assert_eq!(ea.diag, eb.diag, "enc/{ctx}: diag differs");
        assert_eq!(ea.errored, eb.errored, "enc/{ctx}: error differs");
        assert_eq!(ea.bytes, eb.bytes, "enc/{ctx}: bytes differ");
        assert!(!ea.errored, "enc/{ctx}: C encoder failed: {:?}", ea.diag);

        for k in 0..3 {
            let dec = random_dec(&mut r);
            let da = decode_cfg(&l.c, &ea.bytes, &dec);
            let db = decode_cfg(&l.r, &ea.bytes, &dec);
            compare(&format!("dec/{ctx}/k={k} calls={:?}", dec.calls), &da, &db);
        }
    }
}

#[test]
fn fuzz_mutations() {
    let l = libs();
    // build a handful of valid files, then mutate them byte by byte
    let mut bases: Vec<Vec<u8>> = Vec::new();
    for seed in 0..6u64 {
        let mut r = Rng::new(0xba5e_0000 + seed);
        let enc = random_enc(&mut r);
        let out = encode_cfg(&l.c, &enc);
        assert!(!out.errored);
        bases.push(out.bytes);
    }
    let mut r = Rng::new(0xf0f0);
    for (bi, base) in bases.iter().enumerate() {
        for it in 0..250u64 * scale() {
            let mut data = base.clone();
            let nmut = 1 + r.below(3) as usize;
            for _ in 0..nmut {
                let pos = r.below(data.len() as u64) as usize;
                match r.below(3) {
                    0 => data[pos] ^= 1 << r.below(8),
                    1 => data[pos] = r.byte(),
                    _ => data[pos] = if r.flip() { 0 } else { 0xff },
                }
            }
            if r.below(5) == 0 && data.len() > 16 {
                let n = 8 + r.below(data.len() as u64 - 8) as usize;
                data.truncate(n);
            }
            let dec = random_dec(&mut r);
            let da = decode_cfg(&l.c, &data, &dec);
            let db = decode_cfg(&l.r, &data, &dec);
            compare(&format!("mut/base={bi}/it={it}"), &da, &db);
        }
    }
}

#[test]
fn fuzz_random_bytes() {
    // Entirely random input, prefixed with a valid signature often enough to
    // get past the front door.
    let l = libs();
    let mut r = Rng::new(0xdead_beef);
    for it in 0..400u64 * scale() {
        let n = 8 + r.below(200) as usize;
        let mut data: Vec<u8> = (0..n).map(|_| r.byte()).collect();
        if r.below(3) != 0 {
            data[..8].copy_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
        }
        if r.below(3) == 0 && n > 30 {
            // give it a plausible IHDR
            data[8..12].copy_from_slice(&13u32.to_be_bytes());
            data[12..16].copy_from_slice(b"IHDR");
            data[16..20].copy_from_slice(&4u32.to_be_bytes());
            data[20..24].copy_from_slice(&4u32.to_be_bytes());
            data[24] = 8;
            data[25] = 2;
            data[26] = 0;
            data[27] = 0;
            data[28] = 0;
        }
        let dec = random_dec(&mut r);
        let da = decode_cfg(&l.c, &data, &dec);
        let db = decode_cfg(&l.r, &data, &dec);
        compare(&format!("rnd/it={it}"), &da, &db);
    }
}

/* ------------------------------------------- progressive reader fuzzing */

use std::cell::RefCell;

thread_local! {
    static PLOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static PROWBYTES: std::cell::Cell<usize> = std::cell::Cell::new(0);
    static PISC: std::cell::Cell<bool> = std::cell::Cell::new(true);
}

unsafe extern "C-unwind" fn p_info(png: png_structp, info: png_infop) {
    let l = libs();
    let lib = if PISC.with(|c| c.get()) { &l.c } else { &l.r };
    let ctx = Ctx { lib, png, info };
    PLOG.with(|v| {
        let mut v = v.borrow_mut();
        v.push("info".to_string());
        v.extend(snapshot_info(&ctx));
    });
    let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> c_int> =
        ctx.sym("png_set_interlace_handling");
    let passes = unsafe { f(png) };
    ctx.call2("png_read_update_info");
    let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> usize> =
        ctx.sym("png_get_rowbytes");
    let rb = unsafe { g(png, info) };
    PROWBYTES.with(|c| c.set(rb));
    PLOG.with(|v| v.borrow_mut().push(format!("passes={passes} rb={rb}")));
}

unsafe extern "C-unwind" fn p_row(_png: png_structp, new_row: png_bytep, row: u32, pass: c_int) {
    let rb = PROWBYTES.with(|c| c.get());
    let data = if new_row.is_null() {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(new_row, rb) }.to_vec()
    };
    PLOG.with(|v| v.borrow_mut().push(format!("row {row} pass {pass} {}", hex(&data))));
}

unsafe extern "C-unwind" fn p_end(png: png_structp, info: png_infop) {
    let l = libs();
    let lib = if PISC.with(|c| c.get()) { &l.c } else { &l.r };
    let ctx = Ctx { lib, png, info };
    PLOG.with(|v| {
        let mut v = v.borrow_mut();
        v.push("end".to_string());
        v.extend(snapshot_info(&ctx));
    });
}

fn progressive_run(lib: &Lib, is_c: bool, data: &[u8], chunk: usize) -> (Vec<String>, Diag, bool) {
    diag_reset();
    PLOG.with(|v| v.borrow_mut().clear());
    PROWBYTES.with(|c| c.set(0));
    PISC.with(|c| c.set(is_c));

    let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_read_struct");
    let ver = cs(PNG_LIBPNG_VER_STRING);
    let png = unsafe {
        create(ver.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
    };
    let create_info: libloading::Symbol<FnCreateInfo> = lib.sym("png_create_info_struct");
    let info = unsafe { create_info(png) };
    type FnSetProg = unsafe extern "C-unwind" fn(
        png_structp,
        png_voidp,
        Option<unsafe extern "C-unwind" fn(png_structp, png_infop)>,
        Option<unsafe extern "C-unwind" fn(png_structp, png_bytep, u32, c_int)>,
        Option<unsafe extern "C-unwind" fn(png_structp, png_infop)>,
    );
    let setp: libloading::Symbol<FnSetProg> = lib.sym("png_set_progressive_read_fn");
    unsafe { setp(png, std::ptr::null_mut(), Some(p_info), Some(p_row), Some(p_end)) };
    let process: libloading::Symbol<
        unsafe extern "C-unwind" fn(png_structp, png_infop, png_bytep, usize),
    > = lib.sym("png_process_data");
    let res = guard(|| {
        let mut off = 0usize;
        while off < data.len() {
            let n = chunk.min(data.len() - off);
            let mut buf = data[off..off + n].to_vec();
            unsafe { process(png, info, buf.as_mut_ptr(), n) };
            off += n;
        }
    });
    let destroy: libloading::Symbol<FnDestroyRead> = lib.sym("png_destroy_read_struct");
    let mut p = png;
    let mut i = info;
    let _ = guard(|| unsafe { destroy(&mut p, &mut i, std::ptr::null_mut()) });
    let log = PLOG.with(|v| std::mem::take(&mut *v.borrow_mut()));
    (log, diag_take(), res.is_err())
}

#[test]
fn fuzz_progressive() {
    let l = libs();
    let mut bases: Vec<Vec<u8>> = Vec::new();
    for seed in 0..5u64 {
        let mut r = Rng::new(0xc0de_0000 + seed);
        let enc = random_enc(&mut r);
        let out = encode_cfg(&l.c, &enc);
        assert!(!out.errored);
        bases.push(out.bytes);
    }
    let mut r = Rng::new(0x1234_5678);
    for (bi, base) in bases.iter().enumerate() {
        for it in 0..120u64 * scale() {
            let mut data = base.clone();
            // half the iterations run the pristine file, half a mutated one
            if it % 2 == 1 {
                for _ in 0..1 + r.below(3) {
                    let pos = r.below(data.len() as u64) as usize;
                    data[pos] ^= 1 << r.below(8);
                }
                if r.below(4) == 0 && data.len() > 16 {
                    let n = 8 + r.below(data.len() as u64 - 8) as usize;
                    data.truncate(n);
                }
            }
            let chunk = *[1usize, 2, 7, 33, 512, 65536].get(r.below(6) as usize).unwrap();
            let a = progressive_run(&l.c, true, &data, chunk);
            let b = progressive_run(&l.r, false, &data, chunk);
            let label = format!("prog-fuzz/base={bi}/it={it}/chunk={chunk}");
            assert_eq!(a.1, b.1, "{label}: diag differs");
            assert_eq!(a.2, b.2, "{label}: error differs");
            let n = a.0.len().min(b.0.len());
            assert_snapshots_eq(&label, &a.0[..n], &b.0[..n]);
            assert_eq!(a.0.len(), b.0.len(), "{label}: log length differs");
        }
    }
}
