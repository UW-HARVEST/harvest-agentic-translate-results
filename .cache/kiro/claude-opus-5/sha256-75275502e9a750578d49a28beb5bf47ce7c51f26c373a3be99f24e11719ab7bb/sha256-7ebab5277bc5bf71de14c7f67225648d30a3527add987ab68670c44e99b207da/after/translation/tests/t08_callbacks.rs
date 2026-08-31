//! Tier 8: callbacks (status, user transform, user chunk), the floating point
//! setter variants, the info-struct lifecycle helpers and the file based
//! simplified API.

mod common;
use common::*;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};

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

fn formats() -> Vec<(u8, u8)> {
    let mut v = Vec::new();
    for &ct in &[
        PNG_COLOR_TYPE_GRAY,
        PNG_COLOR_TYPE_PALETTE,
        PNG_COLOR_TYPE_RGB,
        PNG_COLOR_TYPE_GRAY_ALPHA,
        PNG_COLOR_TYPE_RGB_ALPHA,
    ] {
        for &bd in &[1u8, 2, 4, 8, 16] {
            let ok = match ct {
                PNG_COLOR_TYPE_GRAY => true,
                PNG_COLOR_TYPE_PALETTE => bd <= 8,
                _ => bd == 8 || bd == 16,
            };
            if ok {
                v.push((ct, bd));
            }
        }
    }
    v
}

/* ------------------------------------------------------------- call log */

thread_local! {
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

fn log(s: String) {
    LOG.with(|l| l.borrow_mut().push(s));
}

fn log_take() -> Vec<String> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

unsafe extern "C-unwind" fn status_cb(_p: png_structp, row: u32, pass: c_int) {
    log(format!("status row={row} pass={pass}"));
}

unsafe extern "C-unwind" fn user_transform_cb(
    _p: png_structp,
    row_info: *mut png_row_info,
    row: png_bytep,
) {
    let ri = unsafe { *row_info };
    let n = ri.rowbytes.min(4096);
    let data = if row.is_null() {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(row, n) }.to_vec()
    };
    log(format!("transform {ri:?} {}", hex(&data)));
}

unsafe extern "C-unwind" fn user_chunk_cb(_p: png_structp, chunk: *mut png_unknown_chunk) -> c_int {
    let c = unsafe { *chunk };
    let data = if c.data.is_null() || c.size == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(c.data, c.size) }.to_vec()
    };
    log(format!(
        "user_chunk {} loc={} size={} {}",
        String::from_utf8_lossy(&c.name[..4]),
        c.location,
        c.size,
        hex(&data)
    ));
    // claim only chunks whose name starts with 'm'
    if c.name[0] == b'm' {
        1
    } else {
        0
    }
}

/* ------------------------------------------------------- write callbacks */

#[test]
fn write_status_and_user_transform() {
    let l = libs();
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let pd = channels(ct) * bd as u32;
            let (w, h) = (12u32, 5u32);
            let rb = rowbytes(pd, w);
            let mut s: u32 = (bd as u32) * 17 + ct as u32 + 1;
            let rows: Vec<Vec<u8>> = (0..h)
                .map(|_| {
                    (0..rb)
                        .map(|_| {
                            s = s.wrapping_mul(1103515245).wrapping_add(12345);
                            (s >> 16) as u8
                        })
                        .collect()
                })
                .collect();
            let run = |lib: &Lib| {
                let _ = log_take();
                let out = write_with(lib, |c, notes| {
                    let png = c.png;
                    let info = c.info;
                    let f: libloading::Symbol<
                        unsafe extern "C-unwind" fn(
                            png_structp,
                            Option<unsafe extern "C-unwind" fn(png_structp, u32, c_int)>,
                        ),
                    > = c.sym("png_set_write_status_fn");
                    unsafe { f(png, Some(status_cb)) };
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(
                            png_structp,
                            Option<
                                unsafe extern "C-unwind" fn(
                                    png_structp,
                                    *mut png_row_info,
                                    png_bytep,
                                ),
                            >,
                        ),
                    > = c.sym("png_set_write_user_transform_fn");
                    unsafe { g(png, Some(user_transform_cb)) };
                    let ui: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_voidp, c_int, c_int),
                    > = c.sym("png_set_user_transform_info");
                    unsafe { ui(png, 0xbeef as png_voidp, bd as c_int, channels(ct) as c_int) };
                    let up: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp) -> png_voidp,
                    > = c.sym("png_get_user_transform_ptr");
                    notes.push(format!("user_transform_ptr={:#x}", unsafe { up(png) } as usize));

                    type Fihdr = unsafe extern "C-unwind" fn(
                        png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int,
                    );
                    let f: libloading::Symbol<Fihdr> = c.sym("png_set_IHDR");
                    unsafe {
                        f(png, info, w, h, bd as c_int, ct as c_int, il,
                          PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE)
                    };
                    if ct == PNG_COLOR_TYPE_PALETTE {
                        let npal = 1usize << bd;
                        let pal: Vec<png_color> = (0..npal)
                            .map(|i| png_color { red: i as u8, green: 0, blue: 0 })
                            .collect();
                        let g: libloading::Symbol<
                            unsafe extern "C-unwind" fn(
                                png_structp, png_infop, *const png_color, c_int,
                            ),
                        > = c.sym("png_set_PLTE");
                        unsafe { g(png, info, pal.as_ptr(), npal as c_int) };
                    }
                    c.call2("png_write_info");
                    let mut ptrs: Vec<*mut u8> =
                        rows.iter().map(|x| x.as_ptr() as *mut u8).collect();
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32),
                    > = c.sym("png_write_image");
                    unsafe { g(png, ptrs.as_mut_ptr(), h) };
                    c.call2("png_write_end");
                    let cr: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> u32> =
                        c.sym("png_get_current_row_number");
                    let cp: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> u8> =
                        c.sym("png_get_current_pass_number");
                    notes.push(format!("row={} pass={}", unsafe { cr(png) }, unsafe { cp(png) }));
                });
                (out, log_take())
            };
            let (a, la) = run(&l.c);
            let (b, lb) = run(&l.r);
            let ctx = format!("ct={ct} bd={bd} il={il}");
            assert_eq!(a.diag, b.diag, "wcb/{ctx}: diag differs");
            assert_eq!(a.errored, b.errored, "wcb/{ctx}: error differs");
            assert_snapshots_eq(&format!("wcb/{ctx} notes"), &a.notes, &b.notes);
            assert_snapshots_eq(&format!("wcb/{ctx} callbacks"), &la, &lb);
            assert_eq!(a.bytes, b.bytes, "wcb/{ctx}: bytes differ");
            assert!(!la.is_empty(), "wcb/{ctx}: no callbacks fired");
        }
    }
}

/* -------------------------------------------------------- read callbacks */

fn encode_with_unknown(ct: u8, bd: u8, il: c_int) -> Vec<u8> {
    let pd = channels(ct) * bd as u32;
    let (w, h) = (12u32, 5u32);
    let rb = rowbytes(pd, w);
    let mut s: u32 = 31 + bd as u32;
    let rows: Vec<Vec<u8>> = (0..h)
        .map(|_| {
            (0..rb)
                .map(|_| {
                    s = s.wrapping_mul(1103515245).wrapping_add(12345);
                    (s >> 16) as u8
                })
                .collect()
        })
        .collect();
    let out = write_with(&libs().c, |c, _| {
        let png = c.png;
        let info = c.info;
        type Fihdr = unsafe extern "C-unwind" fn(
            png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int,
        );
        let f: libloading::Symbol<Fihdr> = c.sym("png_set_IHDR");
        unsafe {
            f(png, info, w, h, bd as c_int, ct as c_int, il,
              PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE)
        };
        if ct == PNG_COLOR_TYPE_PALETTE {
            let npal = 1usize << bd;
            let pal: Vec<png_color> = (0..npal)
                .map(|i| png_color { red: i as u8, green: 1, blue: 2 })
                .collect();
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color, c_int),
            > = c.sym("png_set_PLTE");
            unsafe { g(png, info, pal.as_ptr(), npal as c_int) };
        }
        // two private chunks: one the user callback claims, one it does not
        const PNG_HAVE_IHDR: u8 = 0x01;
        const PNG_AFTER_IDAT: u8 = 0x08;
        let d0: Vec<u8> = (0u8..12).collect();
        let d1: Vec<u8> = vec![0x77; 6];
        let chunks = [
            png_unknown_chunk {
                name: *b"mINe\0",
                data: d0.as_ptr() as *mut u8,
                size: d0.len(),
                location: PNG_HAVE_IHDR,
            },
            png_unknown_chunk {
                name: *b"yOur\0",
                data: d1.as_ptr() as *mut u8,
                size: d1.len(),
                location: PNG_AFTER_IDAT,
            },
        ];
        let g: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_unknown_chunk, c_int),
        > = c.sym("png_set_unknown_chunks");
        unsafe { g(png, info, chunks.as_ptr(), 2) };
        c.call2("png_write_info");
        let mut ptrs: Vec<*mut u8> = rows.iter().map(|x| x.as_ptr() as *mut u8).collect();
        let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32)> =
            c.sym("png_write_image");
        unsafe { g(png, ptrs.as_mut_ptr(), h) };
        c.call2("png_write_end");
    });
    assert!(!out.errored, "encode_with_unknown failed: {:?}", out.diag);
    out.bytes
}

#[test]
fn read_status_user_transform_and_user_chunk() {
    let l = libs();
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let data = encode_with_unknown(ct, bd, il);
            let run = |lib: &Lib| {
                let _ = log_take();
                let out = read_with(lib, &data, |c, out| {
                    let png = c.png;
                    let f: libloading::Symbol<
                        unsafe extern "C-unwind" fn(
                            png_structp,
                            Option<unsafe extern "C-unwind" fn(png_structp, u32, c_int)>,
                        ),
                    > = c.sym("png_set_read_status_fn");
                    unsafe { f(png, Some(status_cb)) };
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(
                            png_structp,
                            png_voidp,
                            Option<unsafe extern "C-unwind" fn(png_structp, *mut png_unknown_chunk) -> c_int>,
                        ),
                    > = c.sym("png_set_read_user_chunk_fn");
                    unsafe { g(png, 0xcafe as png_voidp, Some(user_chunk_cb)) };
                    let ucp: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp) -> png_voidp,
                    > = c.sym("png_get_user_chunk_ptr");
                    out.notes.push(format!("user_chunk_ptr={:#x}", unsafe { ucp(png) } as usize));

                    c.call2("png_read_info");
                    out.notes.extend(snapshot_info(c));

                    let t: libloading::Symbol<
                        unsafe extern "C-unwind" fn(
                            png_structp,
                            Option<
                                unsafe extern "C-unwind" fn(
                                    png_structp,
                                    *mut png_row_info,
                                    png_bytep,
                                ),
                            >,
                        ),
                    > = c.sym("png_set_read_user_transform_fn");
                    unsafe { t(png, Some(user_transform_cb)) };

                    c.call2("png_read_update_info");
                    out.notes.extend(snapshot_info(c));

                    let rb: usize = {
                        let f: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp, png_infop) -> usize,
                        > = c.sym("png_get_rowbytes");
                        unsafe { f(png, c.info) }
                    };
                    let mut bufs: Vec<Vec<u8>> = (0..5).map(|_| vec![0x5au8; rb + 64]).collect();
                    let rr: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, *mut u8, *mut u8),
                    > = c.sym("png_read_row");
                    for b in bufs.iter_mut() {
                        unsafe { rr(png, b.as_mut_ptr(), std::ptr::null_mut()) };
                    }
                    out.rows = bufs;
                    c.call2("png_read_end");
                    out.notes.extend(snapshot_info(c));
                });
                (out, log_take())
            };
            let (a, la) = run(&l.c);
            let (b, lb) = run(&l.r);
            let ctx = format!("ct={ct} bd={bd} il={il}");
            assert_eq!(a.diag, b.diag, "rcb/{ctx}: diag differs");
            assert_eq!(a.errored, b.errored, "rcb/{ctx}: error differs");
            assert_snapshots_eq(&format!("rcb/{ctx} notes"), &a.notes, &b.notes);
            assert_snapshots_eq(&format!("rcb/{ctx} callbacks"), &la, &lb);
            assert_eq!(a.rows, b.rows, "rcb/{ctx}: rows differ");
            assert!(
                la.iter().any(|s| s.starts_with("user_chunk")),
                "rcb/{ctx}: user chunk callback never fired"
            );
            assert!(
                la.iter().any(|s| s.starts_with("transform")),
                "rcb/{ctx}: user transform callback never fired"
            );
        }
    }
}

/* --------------------------------------------- floating point setters */

#[test]
fn floating_point_setters() {
    let l = libs();
    for (ct, bd) in formats() {
        let run = |lib: &Lib| {
            write_with(lib, |c, notes| {
                let png = c.png;
                let info = c.info;
                let mut keep: Vec<CString> = Vec::new();
                type Fihdr = unsafe extern "C-unwind" fn(
                    png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int,
                );
                let f: libloading::Symbol<Fihdr> = c.sym("png_set_IHDR");
                unsafe {
                    f(png, info, 8, 3, bd as c_int, ct as c_int, PNG_INTERLACE_NONE,
                      PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE)
                };
                if ct == PNG_COLOR_TYPE_PALETTE {
                    let npal = 1usize << bd;
                    let pal: Vec<png_color> =
                        (0..npal).map(|i| png_color { red: i as u8, green: 3, blue: 9 }).collect();
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color, c_int),
                    > = c.sym("png_set_PLTE");
                    unsafe { g(png, info, pal.as_ptr(), npal as c_int) };
                }
                // the double-precision entry points
                {
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_infop, f64),
                    > = c.sym("png_set_gAMA");
                    unsafe { g(png, info, 1.0 / 2.2) };
                }
                {
                    type F = unsafe extern "C-unwind" fn(
                        png_structp, png_infop, f64, f64, f64, f64, f64, f64, f64, f64,
                    );
                    let g: libloading::Symbol<F> = c.sym("png_set_cHRM");
                    unsafe {
                        g(png, info, 0.3127, 0.3290, 0.64, 0.33, 0.30, 0.60, 0.15, 0.06)
                    };
                }
                {
                    type F = unsafe extern "C-unwind" fn(
                        png_structp, png_infop, f64, f64, f64, f64, f64, f64, f64, f64, f64,
                    );
                    let g: libloading::Symbol<F> = c.sym("png_set_cHRM_XYZ");
                    unsafe {
                        g(png, info, 0.4124, 0.2126, 0.0193, 0.3576, 0.7152, 0.1192, 0.1805,
                          0.0722, 0.9505)
                    };
                }
                {
                    type F = unsafe extern "C-unwind" fn(
                        png_structp, png_infop, i32, i32, i32, i32, i32, i32, i32, i32, i32,
                    );
                    let g: libloading::Symbol<F> = c.sym("png_set_cHRM_XYZ_fixed");
                    unsafe {
                        g(png, info, 41240, 21260, 1930, 35760, 71520, 11920, 18050, 7220, 95050)
                    };
                }
                {
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_infop, f64, f64),
                    > = c.sym("png_set_cLLI");
                    unsafe { g(png, info, 1000.0, 400.0) };
                }
                {
                    type F = unsafe extern "C-unwind" fn(
                        png_structp, png_infop, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64,
                    );
                    let g: libloading::Symbol<F> = c.sym("png_set_mDCV");
                    unsafe {
                        g(png, info, 0.3127, 0.3290, 0.64, 0.33, 0.30, 0.60, 0.15, 0.06,
                          1000.0, 0.005)
                    };
                }
                {
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_infop, c_int, f64, f64),
                    > = c.sym("png_set_sCAL");
                    unsafe { g(png, info, 1, 2.5, 3.75) };
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_infop, c_int, i32, i32),
                    > = c.sym("png_set_sCAL_fixed");
                    unsafe { g(png, info, 2, 250000, 375000) };
                }
                {
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_infop, u32, *const u8),
                    > = c.sym("png_set_eXIf");
                    // png_set_eXIf takes the deprecated (info, exif) form
                    let exif: Vec<u8> = b"II\x2a\x00\x08\x00\x00\x00".to_vec();
                    unsafe { g(png, info, exif.len() as u32, exif.as_ptr()) };
                }
                // png_set_text_2 with the same content as png_set_text
                {
                    let key = CString::new("Author").unwrap();
                    let txt = CString::new("someone").unwrap();
                    let t = png_text {
                        compression: PNG_TEXT_COMPRESSION_NONE,
                        key: key.as_ptr() as *mut c_char,
                        text: txt.as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: std::ptr::null_mut(),
                        lang_key: std::ptr::null_mut(),
                    };
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_text, c_int) -> c_int,
                    > = c.sym("png_set_text_2");
                    notes.push(format!("set_text_2={}", unsafe { g(png, info, &t, 1) }));
                    keep.push(key);
                    keep.push(txt);
                }
                notes.extend(snapshot_info(c));
                // png_set_invalid clears the flags again
                {
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_infop, c_int),
                    > = c.sym("png_set_invalid");
                    unsafe { g(png, info, (PNG_INFO_gAMA | PNG_INFO_cHRM) as c_int) };
                    notes.extend(snapshot_info(c));
                }
                // sRGB variants
                {
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_infop, c_int),
                    > = c.sym("png_set_sRGB");
                    unsafe { g(png, info, 0) };
                    notes.extend(snapshot_info(c));
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, png_infop, c_int),
                    > = c.sym("png_set_sRGB_gAMA_and_cHRM");
                    unsafe { g(png, info, 3) };
                    notes.extend(snapshot_info(c));
                }
                c.call2("png_write_info");
                let rb = rowbytes(channels(ct) * bd as u32, 8);
                let rows: Vec<Vec<u8>> = (0..3).map(|r| vec![(r * 11) as u8; rb]).collect();
                let mut ptrs: Vec<*mut u8> = rows.iter().map(|x| x.as_ptr() as *mut u8).collect();
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32),
                > = c.sym("png_write_image");
                unsafe { g(png, ptrs.as_mut_ptr(), 3) };
                c.call2("png_write_end");
                drop(keep);
            })
        };
        let a = run(&l.c);
        let b = run(&l.r);
        let ctx = format!("ct={ct} bd={bd}");
        assert_eq!(a.diag, b.diag, "fp-setters/{ctx}: diag differs");
        assert_eq!(a.errored, b.errored, "fp-setters/{ctx}: error differs");
        assert_snapshots_eq(&format!("fp-setters/{ctx}"), &a.notes, &b.notes);
        assert_eq!(a.bytes, b.bytes, "fp-setters/{ctx}: bytes differ");
    }
}

#[test]
fn deprecated_and_misc_setters() {
    let l = libs();
    let run = |lib: &Lib| {
        write_with(lib, |c, notes| {
            let png = c.png;
            let info = c.info;
            // png_set_filter_heuristics is a no-op that warns
            {
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(
                        png_structp,
                        c_int,
                        c_int,
                        *mut f64,
                        *mut f64,
                    ),
                > = c.sym("png_set_filter_heuristics");
                let mut w = [1.0f64, 2.0];
                let mut cst = [1.0f64, 2.0];
                unsafe { g(png, 1, 2, w.as_mut_ptr(), cst.as_mut_ptr()) };
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, c_int, c_int, *mut i32, *mut i32),
                > = c.sym("png_set_filter_heuristics_fixed");
                let mut w = [100000i32, 200000];
                let mut cst = [100000i32, 200000];
                unsafe { g(png, 1, 2, w.as_mut_ptr(), cst.as_mut_ptr()) };
            }
            // compression method / text compression settings
            for name in [
                "png_set_compression_method",
                "png_set_text_compression_method",
                "png_set_text_compression_level",
                "png_set_text_compression_mem_level",
                "png_set_text_compression_strategy",
                "png_set_text_compression_window_bits",
            ] {
                let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                    c.sym(name);
                for v in [-2i32, 0, 1, 8, 9, 15, 16, 100] {
                    unsafe { g(png, v) };
                }
            }
            // compression buffer size
            {
                let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, usize)> =
                    c.sym("png_set_compression_buffer_size");
                for v in [0usize, 1, 1024, 8192, 1 << 20] {
                    unsafe { g(png, v) };
                    let q: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp) -> usize,
                    > = c.sym("png_get_compression_buffer_size");
                    notes.push(format!("cbs({v})={}", unsafe { q(png) }));
                }
            }
            // check-for-invalid-index and sig bytes
            {
                let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                    c.sym("png_set_check_for_invalid_index");
                for v in [-1i32, 0, 1] {
                    unsafe { g(png, v) };
                }
                let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                    c.sym("png_set_sig_bytes");
                for v in [-1i32, 0, 3, 8, 9] {
                    unsafe { g(png, v) };
                }
            }
            // error pointer plumbing
            {
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(
                        png_structp,
                        png_voidp,
                        Option<unsafe extern "C-unwind" fn(png_structp, *const c_char)>,
                        Option<unsafe extern "C-unwind" fn(png_structp, *const c_char)>,
                    ),
                > = c.sym("png_set_error_fn");
                unsafe { g(png, 0x99 as png_voidp, Some(error_cb), Some(warning_cb)) };
                let q: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> png_voidp> =
                    c.sym("png_get_error_ptr");
                notes.push(format!("error_ptr={:#x}", unsafe { q(png) } as usize));
                let q: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> png_voidp> =
                    c.sym("png_get_io_ptr");
                notes.push(format!("io_ptr_nonnull={}", !unsafe { q(png) }.is_null()));
            }
            // longjmp plumbing: only the returned pointer's nullness is compared
            {
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_voidp, usize) -> *mut c_void,
                > = c.sym("png_set_longjmp_fn");
                let p = unsafe { g(png, std::ptr::null_mut(), 200) };
                notes.push(format!("longjmp_fn_null={}", p.is_null()));
                let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp)> =
                    c.sym("png_free_jmpbuf");
                unsafe { f(png) };
            }
            // reset the deflate stream
            {
                let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> c_int> =
                    c.sym("png_reset_zstream");
                notes.push(format!("reset_zstream={}", unsafe { g(png) }));
            }
            // info struct lifecycle
            {
                let init: libloading::Symbol<
                    unsafe extern "C-unwind" fn(*mut png_infop, usize),
                > = c.sym("png_info_init_3");
                let mut i2 = info;
                unsafe { init(&mut i2, 0) };
                notes.extend(snapshot_info(c));
                let freer: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, c_int, u32),
                > = c.sym("png_data_freer");
                unsafe { freer(png, info, 0, PNG_FREE_ALL as u32) };
                let fd: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop, u32, c_int),
                > = c.sym("png_free_data");
                unsafe { fd(png, info, PNG_FREE_ALL as u32, -1) };
                unsafe { freer(png, info, 1, PNG_FREE_ALL as u32) };
                unsafe { fd(png, info, PNG_FREE_ALL as u32, -1) };
                notes.extend(snapshot_info(c));
            }
        })
    };
    let a = run(&l.c);
    let b = run(&l.r);
    assert_eq!(a.diag, b.diag, "misc setters: diag differs");
    assert_eq!(a.errored, b.errored, "misc setters: error differs");
    assert_snapshots_eq("misc setters", &a.notes, &b.notes);
    assert_eq!(a.bytes, b.bytes, "misc setters: bytes differ");
}

#[test]
fn destroy_info_struct_and_png_struct() {
    let l = libs();
    let run = |lib: &Lib| -> (Vec<String>, Diag, bool) {
        diag_reset();
        let mut notes = Vec::new();
        let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_read_struct");
        let ver = cs(PNG_LIBPNG_VER_STRING);
        let png = unsafe {
            create(ver.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
        };
        let create_info: libloading::Symbol<FnCreateInfo> = lib.sym("png_create_info_struct");
        let mut i1 = unsafe { create_info(png) };
        notes.push(format!("info1_nonnull={}", !i1.is_null()));
        let destroy_info: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, *mut png_infop),
        > = lib.sym("png_destroy_info_struct");
        let r = guard(|| unsafe {
            destroy_info(png, &mut i1);
        });
        notes.push(format!("info1_after={}", i1.is_null()));
        // destroying twice and with NULL must be harmless
        let _ = guard(|| unsafe { destroy_info(png, &mut i1) });
        let _ = guard(|| unsafe { destroy_info(png, std::ptr::null_mut()) });
        let _ = guard(|| unsafe { destroy_info(std::ptr::null_mut(), std::ptr::null_mut()) });
        let destroy: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp),
        > = lib.sym("png_destroy_png_struct");
        let _ = guard(|| unsafe { destroy(png) });
        (notes, diag_take(), r.is_err())
    };
    let a = run(&l.c);
    let b = run(&l.r);
    assert_eq!(a, b, "info/struct destruction differs");
}

/* ----------------------------------------------------- file based APIs */

#[test]
fn file_based_simplified_api() {
    let l = libs();
    let dir = std::env::temp_dir().join(format!("pngdiff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    for &fmt in &[PNG_FORMAT_RGB, PNG_FORMAT_RGBA, PNG_FORMAT_GRAY, PNG_FORMAT_LINEAR_RGB] {
        let (w, h) = (9u32, 4u32);
        let ch = image_pixel_channels(fmt);
        let comp = image_component_size(fmt) as usize;
        let mut s: u32 = fmt * 13 + 1;
        let pixels: Vec<u8> = (0..(w * h * ch) as usize * comp)
            .map(|_| {
                s = s.wrapping_mul(1103515245).wrapping_add(12345);
                (s >> 16) as u8
            })
            .collect();

        // write to a file with each library, then read both files back with both
        let mut written: Vec<(String, Vec<u8>)> = Vec::new();
        for (tag, lib) in [("c", &l.c), ("r", &l.r)] {
            diag_reset();
            let path = dir.join(format!("out-{tag}-{fmt}.png"));
            let cpath = CString::new(path.to_str().unwrap()).unwrap();
            let mut image = png_image::default();
            image.width = w;
            image.height = h;
            image.format = fmt;
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(
                    *mut png_image,
                    *const c_char,
                    c_int,
                    *const c_void,
                    i32,
                    *const c_void,
                ) -> c_int,
            > = lib.sym("png_image_write_to_file");
            let r = unsafe {
                f(
                    &mut image,
                    cpath.as_ptr(),
                    0,
                    pixels.as_ptr() as *const c_void,
                    0,
                    std::ptr::null(),
                )
            };
            assert_ne!(r, 0, "{tag}: png_image_write_to_file failed");
            let free: libloading::Symbol<unsafe extern "C-unwind" fn(*mut png_image)> =
                lib.sym("png_image_free");
            unsafe { free(&mut image) };
            written.push((tag.to_string(), std::fs::read(&path).unwrap()));
        }
        assert_eq!(
            written[0].1, written[1].1,
            "png_image_write_to_file produced different files for fmt {fmt:#x}"
        );

        // read the file back through png_image_begin_read_from_file
        let path = dir.join(format!("out-c-{fmt}.png"));
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let read = |lib: &Lib| -> (String, Vec<u8>, c_int, Diag) {
            diag_reset();
            let mut image = png_image::default();
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(*mut png_image, *const c_char) -> c_int,
            > = lib.sym("png_image_begin_read_from_file");
            let r1 = unsafe { f(&mut image, cpath.as_ptr()) };
            let mut buf = Vec::new();
            let mut r2 = -1;
            if r1 != 0 {
                buf = vec![0x5au8; image_size(&image) + 64];
                let g: libloading::Symbol<
                    unsafe extern "C-unwind" fn(
                        *mut png_image,
                        *const png_color,
                        *mut c_void,
                        i32,
                        *mut c_void,
                    ) -> c_int,
                > = lib.sym("png_image_finish_read");
                r2 = unsafe {
                    g(
                        &mut image,
                        std::ptr::null(),
                        buf.as_mut_ptr() as *mut c_void,
                        0,
                        std::ptr::null_mut(),
                    )
                };
            }
            let snap = format!("{:?}", (image.width, image.height, image.format, image.flags, image.warning_or_error));
            let free: libloading::Symbol<unsafe extern "C-unwind" fn(*mut png_image)> =
                lib.sym("png_image_free");
            unsafe { free(&mut image) };
            (snap, buf, r1 * 10 + r2, diag_take())
        };
        let a = read(&l.c);
        let b = read(&l.r);
        assert_eq!(a.2, b.2, "begin_read_from_file returns differ (fmt {fmt:#x})");
        assert_eq!(a.0, b.0, "begin_read_from_file image differs (fmt {fmt:#x})");
        assert_eq!(a.3, b.3, "begin_read_from_file diag differs (fmt {fmt:#x})");
        assert_eq!(a.1, b.1, "begin_read_from_file pixels differ (fmt {fmt:#x})");

        // a non-existent path must fail identically
        let bad = CString::new(dir.join("does-not-exist.png").to_str().unwrap()).unwrap();
        let fail = |lib: &Lib| -> (String, c_int) {
            let mut image = png_image::default();
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(*mut png_image, *const c_char) -> c_int,
            > = lib.sym("png_image_begin_read_from_file");
            let r = unsafe { f(&mut image, bad.as_ptr()) };
            let raw: Vec<u8> = image.message.iter().map(|&x| x as u8).collect();
            let end = raw.iter().position(|&x| x == 0).unwrap_or(raw.len());
            let msg = String::from_utf8_lossy(&raw[..end]).into_owned();
            let free: libloading::Symbol<unsafe extern "C-unwind" fn(*mut png_image)> =
                lib.sym("png_image_free");
            unsafe { free(&mut image) };
            (msg, r)
        };
        assert_eq!(fail(&l.c), fail(&l.r), "missing-file failure differs");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/* --------------------------------------------------------- png_init_io */

#[test]
fn init_io_round_trip() {
    let l = libs();
    let dir = std::env::temp_dir().join(format!("pngdiff-io-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut outputs = Vec::new();
    for (tag, lib) in [("c", &l.c), ("r", &l.r)] {
        let path = dir.join(format!("io-{tag}.png"));
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let mode = CString::new("wb").unwrap();
        // use the C runtime's fopen: both libraries link the same libc
        let fopen: libloading::Symbol<
            unsafe extern "C-unwind" fn(*const c_char, *const c_char) -> *mut c_void,
        > = unsafe { std::mem::transmute(libc_fopen()) };
        let fp = unsafe { fopen(cpath.as_ptr(), mode.as_ptr()) };
        assert!(!fp.is_null(), "fopen failed");
        diag_reset();
        let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_write_struct");
        let ver = cs(PNG_LIBPNG_VER_STRING);
        let png = unsafe {
            create(ver.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
        };
        let create_info: libloading::Symbol<FnCreateInfo> = lib.sym("png_create_info_struct");
        let info = unsafe { create_info(png) };
        let init: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut c_void)> =
            lib.sym("png_init_io");
        unsafe { init(png, fp) };
        let ctx = Ctx { lib, png, info };
        let _ = guard(|| {
            type Fihdr = unsafe extern "C-unwind" fn(
                png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int,
            );
            let f: libloading::Symbol<Fihdr> = ctx.sym("png_set_IHDR");
            unsafe {
                f(png, info, 10, 3, 8, PNG_COLOR_TYPE_RGB as c_int, PNG_INTERLACE_NONE,
                  PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE)
            };
            ctx.call2("png_write_info");
            let rows: Vec<Vec<u8>> = (0..3u32).map(|r| vec![(r * 5) as u8; 30]).collect();
            let mut ptrs: Vec<*mut u8> = rows.iter().map(|x| x.as_ptr() as *mut u8).collect();
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32)> =
                ctx.sym("png_write_image");
            unsafe { g(png, ptrs.as_mut_ptr(), 3) };
            ctx.call2("png_write_end");
        });
        let destroy: libloading::Symbol<FnDestroyWrite> = lib.sym("png_destroy_write_struct");
        let mut p = png;
        let mut i = info;
        let _ = guard(|| unsafe { destroy(&mut p, &mut i) });
        let fclose: libloading::Symbol<unsafe extern "C-unwind" fn(*mut c_void) -> c_int> =
            unsafe { std::mem::transmute(libc_fclose()) };
        unsafe { fclose(fp) };
        outputs.push((std::fs::read(&path).unwrap(), diag_take()));
    }
    assert_eq!(outputs[0].1, outputs[1].1, "png_init_io diag differs");
    assert_eq!(outputs[0].0, outputs[1].0, "png_init_io output differs");
    assert!(!outputs[0].0.is_empty(), "png_init_io produced nothing");
    let _ = std::fs::remove_dir_all(&dir);
}

unsafe extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

fn libc_fopen() -> *const c_void {
    fopen as *const c_void
}

fn libc_fclose() -> *const c_void {
    fclose as *const c_void
}
