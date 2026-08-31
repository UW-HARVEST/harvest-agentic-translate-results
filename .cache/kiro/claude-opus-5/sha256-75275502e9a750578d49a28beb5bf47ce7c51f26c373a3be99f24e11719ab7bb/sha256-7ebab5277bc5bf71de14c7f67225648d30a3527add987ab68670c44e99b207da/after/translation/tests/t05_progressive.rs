//! Tier 5: the progressive (push) reader, `png_write_png`, and the simplified
//! `png_image_*` API.

mod common;
use common::*;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CString};

/* ------------------------------------------------------------- test images */

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
        _ => unreachable!(),
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

/// Encode an image with the C library; optionally add a full chunk set.
fn encode(width: u32, height: u32, bd: u8, ct: u8, interlace: c_int, rich: bool, seed: u32) -> Vec<u8> {
    let pd = channels(ct) * bd as u32;
    let rb = rowbytes(pd, width);
    let mut s = seed | 1;
    let rows: Vec<Vec<u8>> = (0..height)
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
        let mut keep: Vec<CString> = Vec::new();
        type Fihdr = unsafe extern "C-unwind" fn(
            png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int,
        );
        let f: libloading::Symbol<Fihdr> = c.sym("png_set_IHDR");
        unsafe {
            f(
                png, info, width, height, bd as c_int, ct as c_int, interlace,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
            )
        };
        if ct == PNG_COLOR_TYPE_PALETTE {
            let npal = 1usize << bd;
            let pal: Vec<png_color> = (0..npal)
                .map(|i| png_color {
                    red: (i * 7 % 256) as u8,
                    green: (i * 13 % 256) as u8,
                    blue: (i * 29 % 256) as u8,
                })
                .collect();
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color, c_int),
            > = c.sym("png_set_PLTE");
            unsafe { g(png, info, pal.as_ptr(), npal as c_int) };
        }
        if rich {
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, i32)> =
                c.sym("png_set_gAMA_fixed");
            unsafe { g(png, info, 45455) };
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, u32, u32, c_int),
            > = c.sym("png_set_pHYs");
            unsafe { g(png, info, 3000, 2500, 1) };
            let key = CString::new("Title").unwrap();
            let txt = CString::new("progressive test image").unwrap();
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
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_text, c_int),
            > = c.sym("png_set_text");
            unsafe { g(png, info, &t, 1) };
            keep.push(key);
            keep.push(txt);
        }
        c.call2("png_write_info");
        let mut ptrs: Vec<*mut u8> = rows.iter().map(|r| r.as_ptr() as *mut u8).collect();
        let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32)> =
            c.sym("png_write_image");
        unsafe { g(png, ptrs.as_mut_ptr(), height) };
        c.call2("png_write_end");
        drop(keep);
    });
    assert!(!out.errored, "encode failed: {:?}", out.diag);
    out.bytes
}

/* ------------------------------------------------------- progressive read */

thread_local! {
    static PROG: RefCell<ProgState> = RefCell::new(ProgState::default());
}

#[derive(Default)]
struct ProgState {
    log: Vec<String>,
    rows: Vec<(u32, c_int, Vec<u8>)>,
    /// row buffers indexed by row number, used for png_progressive_combine_row
    canvas: Vec<Vec<u8>>,
    rowbytes: usize,
    combine: bool,
    lib_is_c: bool,
}

unsafe extern "C-unwind" fn info_cb(png: png_structp, info: png_infop) {
    let l = libs();
    PROG.with(|p| {
        let mut p = p.borrow_mut();
        let lib = if p.lib_is_c { &l.c } else { &l.r };
        let ctx = Ctx { lib, png, info };
        p.log.push("info_callback".to_string());
        for s in snapshot_info(&ctx) {
            p.log.push(s);
        }
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> usize> =
            ctx.sym("png_get_rowbytes");
        let rb = unsafe { f(png, info) };
        let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> u32> =
            ctx.sym("png_get_image_height");
        let h = unsafe { g(png, info) };
        p.rowbytes = rb;
        p.canvas = (0..h).map(|_| vec![0x5au8; rb + 64]).collect();
        // ask for pass decomposition so the callback sees complete rows
        let s: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> c_int> =
            ctx.sym("png_set_interlace_handling");
        let passes = unsafe { s(png) };
        p.log.push(format!("passes={passes} rowbytes={rb}"));
        ctx.call2("png_read_update_info");
        for s in snapshot_info(&ctx) {
            p.log.push(s);
        }
    });
}

unsafe extern "C-unwind" fn row_cb(png: png_structp, new_row: png_bytep, row_num: u32, pass: c_int) {
    let l = libs();
    PROG.with(|p| {
        let mut p = p.borrow_mut();
        let lib = if p.lib_is_c { &l.c } else { &l.r };
        let rb = p.rowbytes;
        if p.combine {
            if (row_num as usize) < p.canvas.len() {
                let dst = p.canvas[row_num as usize].as_mut_ptr();
                let f: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_bytep, png_bytep),
                > = unsafe { lib.lib.get(b"png_progressive_combine_row").unwrap() };
                unsafe { f(png, dst, new_row) };
            }
            p.log.push(format!("row_callback row={row_num} pass={pass} new_row_null={}", new_row.is_null()));
        } else {
            let data = if new_row.is_null() {
                Vec::new()
            } else {
                unsafe { std::slice::from_raw_parts(new_row, rb) }.to_vec()
            };
            p.rows.push((row_num, pass, data));
        }
    });
}

unsafe extern "C-unwind" fn end_cb(png: png_structp, info: png_infop) {
    let l = libs();
    PROG.with(|p| {
        let mut p = p.borrow_mut();
        let lib = if p.lib_is_c { &l.c } else { &l.r };
        let ctx = Ctx { lib, png, info };
        p.log.push("end_callback".to_string());
        for s in snapshot_info(&ctx) {
            p.log.push(s);
        }
    });
}

struct ProgOutcome {
    log: Vec<String>,
    rows: Vec<(u32, c_int, Vec<u8>)>,
    canvas: Vec<Vec<u8>>,
    diag: Diag,
    errored: bool,
}

fn progressive(lib: &Lib, is_c: bool, data: &[u8], chunk: usize, combine: bool) -> ProgOutcome {
    diag_reset();
    PROG.with(|p| {
        let mut p = p.borrow_mut();
        *p = ProgState::default();
        p.combine = combine;
        p.lib_is_c = is_c;
    });

    let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_read_struct");
    let ver = cs(PNG_LIBPNG_VER_STRING);
    let png = unsafe {
        create(ver.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
    };
    let create_info: libloading::Symbol<FnCreateInfo> = lib.sym("png_create_info_struct");
    let info = unsafe { create_info(png) };
    let end_info = unsafe { create_info(png) };

    type FnSetProg = unsafe extern "C-unwind" fn(
        png_structp,
        png_voidp,
        Option<unsafe extern "C-unwind" fn(png_structp, png_infop)>,
        Option<unsafe extern "C-unwind" fn(png_structp, png_bytep, u32, c_int)>,
        Option<unsafe extern "C-unwind" fn(png_structp, png_infop)>,
    );
    let setp: libloading::Symbol<FnSetProg> = lib.sym("png_set_progressive_read_fn");
    unsafe { setp(png, std::ptr::null_mut(), Some(info_cb), Some(row_cb), Some(end_cb)) };

    let process: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, png_bytep, usize)> =
        lib.sym("png_process_data");

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
    let mut e = end_info;
    let _ = guard(|| unsafe { destroy(&mut p, &mut i, &mut e) });

    let (log, rows, canvas) = PROG.with(|p| {
        let mut p = p.borrow_mut();
        (
            std::mem::take(&mut p.log),
            std::mem::take(&mut p.rows),
            std::mem::take(&mut p.canvas),
        )
    });
    ProgOutcome { log, rows, canvas, diag: diag_take(), errored: res.is_err() }
}

#[test]
fn progressive_matches_c() {
    let l = libs();
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let data = encode(16, 7, bd, ct, il, true, 99 + bd as u32 + ct as u32);
            for &chunk in &[1usize, 3, 64, 8192] {
                for &combine in &[false, true] {
                    let a = progressive(&l.c, true, &data, chunk, combine);
                    let b = progressive(&l.r, false, &data, chunk, combine);
                    let ctx = format!("ct={ct} bd={bd} il={il} chunk={chunk} combine={combine}");
                    assert_eq!(a.errored, b.errored, "prog/{ctx}: error differs {:?} {:?}", a.diag, b.diag);
                    assert_eq!(a.diag, b.diag, "prog/{ctx}: diag differs");
                    assert_snapshots_eq(&format!("prog/{ctx}"), &a.log, &b.log);
                    assert_eq!(a.rows, b.rows, "prog/{ctx}: rows differ");
                    assert_eq!(a.canvas, b.canvas, "prog/{ctx}: canvas differs");
                }
            }
        }
    }
}

#[test]
fn process_data_pause_and_skip() {
    let l = libs();
    let data = encode(16, 5, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE, true, 7);
    // png_process_data_pause / png_process_data_skip only make sense with the
    // "save" variant; compare their return values across a full stream.
    let run = |lib: &Lib, is_c: bool, save: c_int| -> (Vec<usize>, Vec<u32>, Diag, bool) {
        diag_reset();
        PROG.with(|p| {
            let mut p = p.borrow_mut();
            *p = ProgState::default();
            p.lib_is_c = is_c;
            p.combine = true;
        });
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
        unsafe { setp(png, std::ptr::null_mut(), Some(info_cb), Some(row_cb), Some(end_cb)) };
        let process: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, png_bytep, usize),
        > = lib.sym("png_process_data");
        let pause: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int) -> usize> =
            lib.sym("png_process_data_pause");
        let skip: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> u32> =
            lib.sym("png_process_data_skip");
        let mut pauses = Vec::new();
        let mut skips = Vec::new();
        let res = guard(|| {
            let mut off = 0usize;
            while off < data.len() {
                let n = 37.min(data.len() - off);
                let mut buf = data[off..off + n].to_vec();
                unsafe { process(png, info, buf.as_mut_ptr(), n) };
                pauses.push(unsafe { pause(png, save) });
                skips.push(unsafe { skip(png) });
                off += n;
            }
        });
        let destroy: libloading::Symbol<FnDestroyRead> = lib.sym("png_destroy_read_struct");
        let mut p = png;
        let mut i = info;
        let _ = guard(|| unsafe {
            destroy(&mut p, &mut i, std::ptr::null_mut())
        });
        (pauses, skips, diag_take(), res.is_err())
    };
    for save in [0, 1] {
        let a = run(&l.c, true, save);
        let b = run(&l.r, false, save);
        assert_eq!(a.0, b.0, "pause returns differ (save={save})");
        assert_eq!(a.1, b.1, "skip returns differ (save={save})");
        assert_eq!(a.2, b.2, "diag differs (save={save})");
        assert_eq!(a.3, b.3, "error differs (save={save})");
    }
}

/* -------------------------------------------------------- png_write_png */

#[test]
fn write_png_high_level() {
    let l = libs();
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let pd = channels(ct) * bd as u32;
            let width = 16u32;
            let height = 5u32;
            let rb = rowbytes(pd, width);
            let mut s = (bd as u32) * 31 + ct as u32 + il as u32;
            s |= 1;
            let rows: Vec<Vec<u8>> = (0..height)
                .map(|_| {
                    (0..rb)
                        .map(|_| {
                            s = s.wrapping_mul(1103515245).wrapping_add(12345);
                            (s >> 16) as u8
                        })
                        .collect()
                })
                .collect();

            for &tr in &[
                PNG_TRANSFORM_IDENTITY,
                PNG_TRANSFORM_PACKING,
                PNG_TRANSFORM_PACKSWAP,
                PNG_TRANSFORM_BGR,
                PNG_TRANSFORM_SWAP_ALPHA,
                PNG_TRANSFORM_SWAP_ENDIAN,
                PNG_TRANSFORM_INVERT_ALPHA,
                PNG_TRANSFORM_INVERT_MONO,
                PNG_TRANSFORM_SHIFT,
            ] {
                let run = |lib: &Lib| {
                    write_with(lib, |c, _| {
                        let png = c.png;
                        let info = c.info;
                        type Fihdr = unsafe extern "C-unwind" fn(
                            png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int,
                        );
                        let f: libloading::Symbol<Fihdr> = c.sym("png_set_IHDR");
                        unsafe {
                            f(
                                png, info, width, height, bd as c_int, ct as c_int, il,
                                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                            )
                        };
                        if ct == PNG_COLOR_TYPE_PALETTE {
                            let npal = 1usize << bd;
                            let pal: Vec<png_color> = (0..npal)
                                .map(|i| png_color {
                                    red: (i * 7 % 256) as u8,
                                    green: (i * 13 % 256) as u8,
                                    blue: (i * 29 % 256) as u8,
                                })
                                .collect();
                            let g: libloading::Symbol<
                                unsafe extern "C-unwind" fn(
                                    png_structp,
                                    png_infop,
                                    *const png_color,
                                    c_int,
                                ),
                            > = c.sym("png_set_PLTE");
                            unsafe { g(png, info, pal.as_ptr(), npal as c_int) };
                        }
                        if tr & PNG_TRANSFORM_SHIFT != 0 {
                            let g: libloading::Symbol<
                                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color_8),
                            > = c.sym("png_set_sBIT");
                            let b = bd.min(8).max(1);
                            let sb = png_color_8 { red: b, green: b, blue: b, gray: b, alpha: b };
                            unsafe { g(png, info, &sb) };
                        }
                        let mut ptrs: Vec<*mut u8> =
                            rows.iter().map(|r| r.as_ptr() as *mut u8).collect();
                        let g: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp, png_infop, *mut *mut u8),
                        > = c.sym("png_set_rows");
                        unsafe { g(png, info, ptrs.as_mut_ptr()) };
                        let h: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp, png_infop, c_int, *mut c_void),
                        > = c.sym("png_write_png");
                        unsafe { h(png, info, tr, std::ptr::null_mut()) };
                    })
                };
                let a = run(&l.c);
                let b = run(&l.r);
                let ctx = format!("ct={ct} bd={bd} il={il} tr={tr:#x}");
                assert_eq!(a.errored, b.errored, "write_png/{ctx}: error differs {:?} {:?}", a.diag, b.diag);
                assert_eq!(a.diag, b.diag, "write_png/{ctx}: diag differs");
                assert_eq!(a.bytes.len(), b.bytes.len(), "write_png/{ctx}: length differs");
                assert!(a.bytes == b.bytes, "write_png/{ctx}: bytes differ");
            }
        }
    }
}

/* ----------------------------------------------------- simplified API */

fn image_snapshot(img: &png_image) -> String {
    let msg = {
        let raw: Vec<u8> = img.message.iter().map(|&c| c as u8).collect();
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        String::from_utf8_lossy(&raw[..end]).into_owned()
    };
    format!(
        "v={} {}x{} fmt={:#x} flags={:#x} cmap={} woe={} msg={:?}",
        img.version, img.width, img.height, img.format, img.flags, img.colormap_entries,
        img.warning_or_error, msg
    )
}

#[test]
fn simplified_read_from_memory() {
    let l = libs();
    let mut decoded = 0usize;
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let data = encode(13, 6, bd, ct, il, true, 500 + bd as u32 + ct as u32);
            for &fmt in &[
                None,
                Some(PNG_FORMAT_GRAY),
                Some(PNG_FORMAT_GA),
                Some(PNG_FORMAT_AG),
                Some(PNG_FORMAT_RGB),
                Some(PNG_FORMAT_BGR),
                Some(PNG_FORMAT_RGBA),
                Some(PNG_FORMAT_ARGB),
                Some(PNG_FORMAT_BGRA),
                Some(PNG_FORMAT_ABGR),
                Some(PNG_FORMAT_LINEAR_Y),
                Some(PNG_FORMAT_LINEAR_Y_ALPHA),
                Some(PNG_FORMAT_LINEAR_RGB),
                Some(PNG_FORMAT_LINEAR_RGB_ALPHA),
                Some(PNG_FORMAT_RGB | PNG_FORMAT_FLAG_COLORMAP),
                Some(PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_COLORMAP),
                Some(PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_COLORMAP),
                Some(PNG_FORMAT_GRAY | PNG_FORMAT_FLAG_COLORMAP),
                Some(PNG_FORMAT_GA | PNG_FORMAT_FLAG_COLORMAP),
            ] {
              for bg in [None, Some(png_color { red: 40, green: 80, blue: 120 })] {
                let run = |lib: &Lib| -> (String, Vec<u8>, i32, Diag, Vec<u8>) {
                    diag_reset();
                    let mut image = png_image::default();
                    let begin: libloading::Symbol<
                        unsafe extern "C-unwind" fn(*mut png_image, *const c_void, usize) -> c_int,
                    > = lib.sym("png_image_begin_read_from_memory");
                    let r1 = unsafe {
                        begin(&mut image, data.as_ptr() as *const c_void, data.len())
                    };
                    let mut buf: Vec<u8> = Vec::new();
                    let mut cmap: Vec<u8> = Vec::new();
                    let mut r2 = -1;
                    if r1 != 0 {
                        if let Some(f) = fmt {
                            image.format = f;
                        }
                        buf = vec![0x5au8; image_size(&image) + 64];
                        // a colour-mapped format needs a palette buffer
                        let needs_cmap = image.format & PNG_FORMAT_FLAG_COLORMAP != 0;
                        if needs_cmap {
                            cmap = vec![0x5au8; image_colormap_size(&image) + 64];
                        }
                        let finish: libloading::Symbol<
                            unsafe extern "C-unwind" fn(
                                *mut png_image,
                                *const png_color,
                                *mut c_void,
                                i32,
                                *mut c_void,
                            ) -> c_int,
                        > = lib.sym("png_image_finish_read");
                        r2 = unsafe {
                            finish(
                                &mut image,
                                bg.as_ref().map_or(std::ptr::null(), |b| b as *const png_color),
                                buf.as_mut_ptr() as *mut c_void,
                                0,
                                if needs_cmap {
                                    cmap.as_mut_ptr() as *mut c_void
                                } else {
                                    std::ptr::null_mut()
                                },
                            )
                        };
                    }
                    let snap = image_snapshot(&image);
                    let free: libloading::Symbol<unsafe extern "C-unwind" fn(*mut png_image)> =
                        lib.sym("png_image_free");
                    unsafe { free(&mut image) };
                    (snap, buf, r1 * 10 + r2, diag_take(), cmap)
                };
                let a = run(&l.c);
                let b = run(&l.r);
                let ctx = format!("ct={ct} bd={bd} il={il} fmt={fmt:?} bg={bg:?}");
                assert_eq!(a.2, b.2, "simplified/{ctx}: return codes differ");
                if a.2 == 11 {
                    decoded += 1;
                }
                assert_eq!(a.0, b.0, "simplified/{ctx}: png_image differs");
                assert_eq!(a.3, b.3, "simplified/{ctx}: diag differs");
                assert_eq!(a.1, b.1, "simplified/{ctx}: pixels differ");
                assert_eq!(a.4, b.4, "simplified/{ctx}: colormap differs");
              }
            }
        }
    }
    // a decent share of the format requests must actually have produced pixels
    assert!(decoded > 200, "only {decoded} simplified reads succeeded");
}

#[test]
fn simplified_write_to_memory() {
    let l = libs();
    for &fmt in &[
        PNG_FORMAT_GRAY,
        PNG_FORMAT_GA,
        PNG_FORMAT_AG,
        PNG_FORMAT_RGB,
        PNG_FORMAT_BGR,
        PNG_FORMAT_RGBA,
        PNG_FORMAT_ARGB,
        PNG_FORMAT_BGRA,
        PNG_FORMAT_ABGR,
        PNG_FORMAT_LINEAR_Y,
        PNG_FORMAT_LINEAR_Y_ALPHA,
        PNG_FORMAT_LINEAR_RGB,
        PNG_FORMAT_LINEAR_RGB_ALPHA,
    ] {
        for &convert_to_8bit in &[0i32, 1] {
            let width = 11u32;
            let height = 5u32;
            let channels = image_pixel_channels(fmt);
            let comp = image_component_size(fmt) as usize;
            let nbytes = (width * height * channels) as usize * comp;
            let mut s: u32 = fmt * 7 + 1;
            let pixels: Vec<u8> = (0..nbytes)
                .map(|_| {
                    s = s.wrapping_mul(1103515245).wrapping_add(12345);
                    (s >> 16) as u8
                })
                .collect();

            let run = |lib: &Lib| -> (String, Vec<u8>, c_int, Diag) {
                diag_reset();
                let mut image = png_image::default();
                image.width = width;
                image.height = height;
                image.format = fmt;
                image.flags = 0;
                image.colormap_entries = 0;
                let f: libloading::Symbol<
                    unsafe extern "C-unwind" fn(
                        *mut png_image,
                        *mut c_void,
                        *mut usize,
                        c_int,
                        *const c_void,
                        i32,
                        *const c_void,
                    ) -> c_int,
                > = lib.sym("png_image_write_to_memory");
                // first pass: query the size
                let mut size: usize = 0;
                let r = unsafe {
                    f(
                        &mut image,
                        std::ptr::null_mut(),
                        &mut size,
                        convert_to_8bit,
                        pixels.as_ptr() as *const c_void,
                        0,
                        std::ptr::null(),
                    )
                };
                let mut out = vec![0u8; size];
                let mut size2 = size;
                let r2 = if r != 0 && size > 0 {
                    unsafe {
                        f(
                            &mut image,
                            out.as_mut_ptr() as *mut c_void,
                            &mut size2,
                            convert_to_8bit,
                            pixels.as_ptr() as *const c_void,
                            0,
                            std::ptr::null(),
                        )
                    }
                } else {
                    -1
                };
                out.truncate(size2);
                let snap = format!("{} size={size} size2={size2} r={r} r2={r2}", image_snapshot(&image));
                let free: libloading::Symbol<unsafe extern "C-unwind" fn(*mut png_image)> =
                    lib.sym("png_image_free");
                unsafe { free(&mut image) };
                (snap, out, r * 10 + r2, diag_take())
            };
            let a = run(&l.c);
            let b = run(&l.r);
            let ctx = format!("fmt={fmt:#x} to8={convert_to_8bit}");
            assert!(!a.1.is_empty(), "simplified-write/{ctx}: C produced nothing: {}", a.0);
            assert_eq!(a.0, b.0, "simplified-write/{ctx}: image state differs");
            assert_eq!(a.3, b.3, "simplified-write/{ctx}: diag differs");
            assert_eq!(a.2, b.2, "simplified-write/{ctx}: return differs");
            assert_eq!(
                a.1, b.1,
                "simplified-write/{ctx}: bytes differ\n C: {}\n R: {}",
                hex(&a.1),
                hex(&b.1)
            );
        }
    }
}
