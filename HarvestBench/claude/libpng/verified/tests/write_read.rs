//! Phase B — the low-level sequential write and read entry points.
//!
//! Covers CONFIGS.md rows C-26 … C-64 (the colour-type × bit-depth × interlace
//! matrix and the write-side option axes).
#![allow(non_snake_case)]

mod common;

use common::*;
use core::ffi::c_int;

const SIZES: [(u32, u32); 12] = [
    (1, 1),
    (1, 7),
    (7, 1),
    (2, 3),
    (3, 2),
    (5, 5),
    (8, 8),
    (9, 5),
    (15, 2),
    (16, 16),
    (17, 3),
    (33, 4),
];

/// C-26 … C-55: every legal (colour type, bit depth) × interlace, randomised
/// over size, pixel data, filter mask and zlib level.
#[test]
fn matrix() {
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for (si, &(w, h)) in SIZES.iter().enumerate() {
                let mut rng = Rng::new(
                    0x5eed ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ ((il as u64) << 24) ^ si as u64,
                );
                let mut img = Img::random(&mut rng, w, h, ct, bd);
                img.interlace = il;
                let opts = WriteOpts {
                    filter_mask: Some(rng.pick(&[
                        PNG_NO_FILTERS,
                        PNG_FILTER_NONE,
                        PNG_FILTER_SUB,
                        PNG_FILTER_UP,
                        PNG_FILTER_AVG,
                        PNG_FILTER_PAETH,
                        PNG_FAST_FILTERS,
                        PNG_ALL_FILTERS,
                    ])),
                    level: Some(rng.range(0, 10) as c_int),
                    status_fn: true,
                    ..Default::default()
                };
                let case = format!("write ct={} bd={} il={} {}x{}", ct, bd, il, w, h);
                let mut file: Vec<u8> = Vec::new();
                assert_same(&case, |api| unsafe {
                    let mut o = Outcome::default();
                    let wr = write_image(api, &img, &opts, &mut |_, _, _| {});
                    o.push(format!("guard={:?}", wr.guard));
                    o.output = wr.bytes.clone();
                    if api.which == "C" {
                        file = wr.bytes.clone();
                    }
                    o
                });
                assert!(!file.is_empty(), "{}: nothing written", case);

                // and read it straight back with both libraries
                let ropts = ReadOpts {
                    status_fn: true,
                    ..Default::default()
                };
                assert_same(&format!("read back {}", case), |api| unsafe {
                    let mut o = Outcome::default();
                    let rr = read_plain(api, &file, &ropts);
                    o.push(format!("guard={:?}", rr.guard));
                    for r in &rr.rows {
                        o.output.extend_from_slice(r);
                    }
                    o
                });
            }
        }
    }
}

/// C-56: `png_write_rows` / `png_write_image` / `png_read_rows` /
/// `png_read_image` instead of the one-row entry points.
#[test]
fn bulk_rows() {
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let mut rng = Rng::new(0xb01c ^ ((ct as u64) << 8) ^ bd as u64 ^ ((il as u64) << 16));
            let mut img = Img::random(&mut rng, 11, 6, ct, bd);
            img.interlace = il;
            for (tag, opts) in [
                (
                    "write_image",
                    WriteOpts {
                        bulk: true,
                        ..Default::default()
                    },
                ),
                (
                    "write_rows(1)",
                    WriteOpts {
                        rows_at_a_time: 1,
                        ..Default::default()
                    },
                ),
                (
                    "write_rows(4)",
                    WriteOpts {
                        rows_at_a_time: 4,
                        ..Default::default()
                    },
                ),
            ] {
                let case = format!("{} ct={} bd={} il={}", tag, ct, bd, il);
                let mut file = Vec::new();
                assert_same(&case, |api| unsafe {
                    let mut o = Outcome::default();
                    let wr = write_image(api, &img, &opts, &mut |_, _, _| {});
                    o.push(format!("guard={:?}", wr.guard));
                    o.output = wr.bytes.clone();
                    if api.which == "C" {
                        file = wr.bytes.clone();
                    }
                    o
                });
                for mode in [
                    RowMode::Row,
                    RowMode::RowDisplay,
                    RowMode::Rows(1),
                    RowMode::Rows(3),
                    RowMode::Image,
                ] {
                    let ropts = ReadOpts {
                        rows: mode,
                        ..Default::default()
                    };
                    assert_same(&format!("{} read {:?}", case, mode), |api| unsafe {
                        let mut o = Outcome::default();
                        let rr = read_plain(api, &file, &ropts);
                        o.push(format!("guard={:?}", rr.guard));
                        for r in &rr.rows {
                            o.output.extend_from_slice(r);
                        }
                        o
                    });
                }
            }
        }
    }
}

/// C-57: every filter mask, including changing it between rows.
#[test]
fn filters() {
    let masks = [
        PNG_NO_FILTERS,
        PNG_FILTER_NONE,
        PNG_FILTER_SUB,
        PNG_FILTER_UP,
        PNG_FILTER_AVG,
        PNG_FILTER_PAETH,
        PNG_FAST_FILTERS,
        PNG_ALL_FILTERS,
        PNG_FILTER_SUB | PNG_FILTER_PAETH,
        PNG_FILTER_NONE | PNG_FILTER_AVG,
    ];
    for (ct, bd) in VALID_SHAPES {
        for &m in &masks {
            let mut rng = Rng::new(0xf117 ^ (m as u64) ^ ((ct as u64) << 16) ^ ((bd as u64) << 24));
            let img = Img::random(&mut rng, 13, 7, ct, bd);
            let opts = WriteOpts {
                filter_mask: Some(m),
                ..Default::default()
            };
            assert_same(
                &format!("filter mask 0x{:02x} ct={} bd={}", m, ct, bd),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let wr = write_plain(api, &img, &opts);
                    o.push(format!("guard={:?}", wr.guard));
                    o.output = wr.bytes.clone();
                    o
                },
            );
        }
    }
    // change the mask between rows
    let mut rng = Rng::new(0xf118);
    let img = Img::random(&mut rng, 20, 9, PNG_COLOR_TYPE_RGB, 8);
    assert_same("filter mask changed per row", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
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
            (api.png_write_info)(png, info);
            for (i, r) in img.rows.iter().enumerate() {
                (api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, masks[i % masks.len()]);
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

/// C-58: the zlib knobs.
#[test]
fn zlib_knobs() {
    let mut rng = Rng::new(0x21_1b);
    let img = Img::random(&mut rng, 24, 12, PNG_COLOR_TYPE_RGB, 8);
    for level in [-1, 0, 1, 5, 9] {
        for strategy in [0, 1, 2, 3, 4] {
            let opts = WriteOpts {
                level: Some(level),
                strategy: Some(strategy),
                ..Default::default()
            };
            assert_same(&format!("level={} strategy={}", level, strategy), |api| unsafe {
                let mut o = Outcome::default();
                let wr = write_plain(api, &img, &opts);
                o.push(format!("guard={:?}", wr.guard));
                o.output = wr.bytes.clone();
                o
            });
        }
    }
    for mem_level in [1, 2, 5, 8, 9] {
        for wbits in [8, 9, 11, 14, 15] {
            let opts = WriteOpts {
                mem_level: Some(mem_level),
                window_bits: Some(wbits),
                ..Default::default()
            };
            assert_same(
                &format!("mem_level={} window_bits={}", mem_level, wbits),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let wr = write_plain(api, &img, &opts);
                    o.push(format!("guard={:?}", wr.guard));
                    o.output = wr.bytes.clone();
                    o
                },
            );
        }
    }
    // out-of-range values: libpng clamps them and warns
    for (tag, opts) in [
        (
            "level=-2",
            WriteOpts {
                level: Some(-2),
                ..Default::default()
            },
        ),
        (
            "level=10",
            WriteOpts {
                level: Some(10),
                ..Default::default()
            },
        ),
        (
            "strategy=5",
            WriteOpts {
                strategy: Some(5),
                ..Default::default()
            },
        ),
        (
            "mem_level=0",
            WriteOpts {
                mem_level: Some(0),
                ..Default::default()
            },
        ),
        (
            "mem_level=10",
            WriteOpts {
                mem_level: Some(10),
                ..Default::default()
            },
        ),
        (
            "window_bits=7",
            WriteOpts {
                window_bits: Some(7),
                ..Default::default()
            },
        ),
        (
            "window_bits=16",
            WriteOpts {
                window_bits: Some(16),
                ..Default::default()
            },
        ),
        (
            "window_bits=-15",
            WriteOpts {
                window_bits: Some(-15),
                ..Default::default()
            },
        ),
        (
            "method=7",
            WriteOpts {
                method: Some(7),
                ..Default::default()
            },
        ),
    ] {
        assert_same(tag, |api| unsafe {
            let mut o = Outcome::default();
            let wr = write_plain(api, &img, &opts);
            o.push(format!("guard={:?}", wr.guard));
            o.output = wr.bytes.clone();
            o
        });
    }
}

/// C-59: `png_set_compression_buffer_size` / `png_get_compression_buffer_size`.
#[test]
fn buffer_size() {
    let mut rng = Rng::new(0xb_0f5);
    let img = Img::random(&mut rng, 40, 30, PNG_COLOR_TYPE_RGB_ALPHA, 8);
    for n in [1usize, 2, 3, 8, 100, 1024, 8192, 65536] {
        let opts = WriteOpts {
            buffer_size: Some(n),
            ..Default::default()
        };
        assert_same(&format!("buffer_size={}", n), |api| unsafe {
            let mut o = Outcome::default();
            let (png, _info) = new_write(api);
            (api.png_set_compression_buffer_size)(png, n);
            o.push(format!(
                "get_compression_buffer_size={}",
                (api.png_get_compression_buffer_size)(png)
            ));
            destroy_write(api, png, core::ptr::null_mut());
            let wr = write_plain(api, &img, &opts);
            o.push(format!("guard={:?}", wr.guard));
            o.output = wr.bytes.clone();
            o
        });
    }
}

/// C-60: the raw chunk writer.
#[test]
fn raw_chunks() {
    let names: [&[u8; 4]; 6] = [b"IHDR", b"tEXt", b"prVt", b"PrVt", b"pRvT", b"ReSe"];
    for name in names {
        for len in [0usize, 1, 7, 8191, 8192, 8193] {
            for pieces in [1usize, 2, 5] {
                let mut rng = Rng::new(0xc4c4 ^ len as u64 ^ ((pieces as u64) << 20));
                let data = rng.bytes(len);
                assert_same(
                    &format!(
                        "raw chunk {} len={} pieces={}",
                        String::from_utf8_lossy(name),
                        len,
                        pieces
                    ),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        let (png, _info) = new_write(api);
                        (api.png_set_write_fn)(
                            png,
                            core::ptr::null_mut(),
                            Some(write_cb),
                            Some(flush_cb),
                        );
                        let g = guarded(api, png, &mut || {
                            (api.png_write_sig)(png);
                            if pieces == 1 {
                                (api.png_write_chunk)(
                                    png,
                                    name.as_ptr(),
                                    data.as_ptr(),
                                    data.len(),
                                );
                            } else {
                                (api.png_write_chunk_start)(png, name.as_ptr(), data.len() as u32);
                                let step = data.len().div_ceil(pieces).max(1);
                                let mut i = 0;
                                while i < data.len() {
                                    let k = step.min(data.len() - i);
                                    (api.png_write_chunk_data)(png, data.as_ptr().add(i), k);
                                    i += k;
                                }
                                (api.png_write_chunk_end)(png);
                            }
                        });
                        o.push(format!("guard={:?}", g));
                        o.output = std::mem::take(&mut tls().output);
                        destroy_write(api, png, core::ptr::null_mut());
                        o
                    },
                );
            }
        }
    }
}

/// C-61: `png_set_flush` / `png_write_flush`.
#[test]
fn flush() {
    let mut rng = Rng::new(0xf1_05);
    let img = Img::random(&mut rng, 30, 20, PNG_COLOR_TYPE_RGB, 8);
    for every in [0, 1, 2, 7, 1000] {
        let opts = WriteOpts {
            flush_every: Some(every),
            ..Default::default()
        };
        assert_same(&format!("flush every {}", every), |api| unsafe {
            let mut o = Outcome::default();
            let wr = write_plain(api, &img, &opts);
            o.push(format!("guard={:?}", wr.guard));
            o.output = wr.bytes.clone();
            o
        });
    }
    // explicit png_write_flush between rows
    assert_same("explicit write_flush", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
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
            for (i, r) in img.rows.iter().enumerate() {
                (api.png_write_row)(png, r.as_ptr() as *mut u8);
                if i % 3 == 0 {
                    (api.png_write_flush)(png);
                }
            }
            (api.png_write_end)(png, info);
        });
        o.push(format!("guard={:?} flushes={}", g, tls().flushes));
        o.output = std::mem::take(&mut tls().output);
        destroy_write(api, png, info);
        o
    });
}

/// C-62: the info getters, before and after `png_read_update_info`.
#[test]
fn info_getters() {
    for (ct, bd) in VALID_SHAPES {
        let mut rng = Rng::new(0x9e77 ^ ((ct as u64) << 8) ^ bd as u64);
        let img = Img::random(&mut rng, 12, 4, ct, bd);
        let mut file = Vec::new();
        assert_same(&format!("getters write ct={} bd={}", ct, bd), |api| unsafe {
            let mut o = Outcome::default();
            let wr = write_plain(api, &img, &WriteOpts::default());
            o.output = wr.bytes.clone();
            if api.which == "C" {
                file = wr.bytes.clone();
            }
            o
        });
        assert_same(&format!("getters read ct={} bd={}", ct, bd), |api| unsafe {
            let mut o = Outcome::default();
            let rr = read_image(
                api,
                &file,
                &ReadOpts::default(),
                &mut |api, png, info| {
                    log(format!(
                        "extra: w={} h={} d={} ct={} il={} comp={} filt={} rb={} ch={}",
                        (api.png_get_image_width)(png, info),
                        (api.png_get_image_height)(png, info),
                        (api.png_get_bit_depth)(png, info),
                        (api.png_get_color_type)(png, info),
                        (api.png_get_interlace_type)(png, info),
                        (api.png_get_compression_type)(png, info),
                        (api.png_get_filter_type)(png, info),
                        (api.png_get_rowbytes)(png, info),
                        (api.png_get_channels)(png, info),
                    ));
                },
            );
            o.push(format!("guard={:?}", rr.guard));
            for r in &rr.rows {
                o.output.extend_from_slice(r);
            }
            o
        });
    }
}

/// C-64: `png_set_sig_bytes` / `png_get_signature`.
#[test]
fn sig_bytes() {
    let mut rng = Rng::new(0x5169);
    let img = Img::random(&mut rng, 6, 3, PNG_COLOR_TYPE_GRAY, 8);
    let mut file = Vec::new();
    assert_same("sig write", |api| unsafe {
        let mut o = Outcome::default();
        let wr = write_plain(api, &img, &WriteOpts::default());
        o.output = wr.bytes.clone();
        if api.which == "C" {
            file = wr.bytes.clone();
        }
        o
    });
    for pre in 0..=8usize {
        assert_same(&format!("sig_bytes={}", pre), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = file[pre..].to_vec();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
            let g = guarded(api, png, &mut || {
                (api.png_set_sig_bytes)(png, pre as c_int);
                (api.png_read_info)(png, info);
                let sig = (api.png_get_signature)(png, info);
                if sig.is_null() {
                    log("signature=<null>".to_string());
                } else {
                    log(format!(
                        "signature={:02x?}",
                        core::slice::from_raw_parts(sig, 8)
                    ));
                }
                let rb = (api.png_get_rowbytes)(png, info);
                let h = (api.png_get_image_height)(png, info) as usize;
                let mut row = vec![0u8; rb];
                for _ in 0..h {
                    (api.png_read_row)(png, row.as_mut_ptr(), core::ptr::null_mut());
                    log(format!("row {:02x?}", row));
                }
                (api.png_read_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }
}

/// C-154: `png_read_end` variants and unread rows.
#[test]
fn read_end() {
    let mut rng = Rng::new(0xe4d);
    let img = Img::random(&mut rng, 10, 8, PNG_COLOR_TYPE_RGB, 8);
    let mut file = Vec::new();
    assert_same("read_end write", |api| unsafe {
        let mut o = Outcome::default();
        let wr = write_plain(api, &img, &WriteOpts::default());
        o.output = wr.bytes.clone();
        if api.which == "C" {
            file = wr.bytes.clone();
        }
        o
    });
    for (tag, opts) in [
        (
            "all rows + read_end(info)",
            ReadOpts {
                ..Default::default()
            },
        ),
        (
            "all rows + read_end(NULL)",
            ReadOpts {
                end_null_info: true,
                ..Default::default()
            },
        ),
        (
            "no rows + read_end(info)",
            ReadOpts {
                rows: RowMode::None,
                ..Default::default()
            },
        ),
        (
            "no rows, no read_end",
            ReadOpts {
                rows: RowMode::None,
                read_end: false,
                ..Default::default()
            },
        ),
        (
            "no update_info",
            ReadOpts {
                update_info: false,
                ..Default::default()
            },
        ),
    ] {
        assert_same(tag, |api| unsafe {
            let mut o = Outcome::default();
            let rr = read_plain(api, &file, &opts);
            o.push(format!("guard={:?}", rr.guard));
            for r in &rr.rows {
                o.output.extend_from_slice(r);
            }
            o
        });
    }
}
