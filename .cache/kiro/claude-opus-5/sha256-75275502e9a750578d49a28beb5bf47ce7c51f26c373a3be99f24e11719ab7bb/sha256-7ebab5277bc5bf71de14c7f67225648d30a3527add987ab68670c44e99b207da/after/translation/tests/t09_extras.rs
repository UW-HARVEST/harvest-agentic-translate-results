//! Tier 9: the remaining directly callable entry points - the float variants of
//! the read transform setters, `png_read_rows`/`png_write_rows`, the stdio
//! simplified API, `png_check_IHDR`, `png_gamma_correct`, `png_set_mem_fn` and
//! the deprecated accessors.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void, CString};

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

fn encode(w: u32, h: u32, bd: u8, ct: u8, il: c_int, seed: u32) -> Vec<u8> {
    let rb = rowbytes(channels(ct) * bd as u32, w);
    let mut s = seed | 1;
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
                .map(|i| png_color { red: (i * 3) as u8, green: (i * 5) as u8, blue: i as u8 })
                .collect();
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color, c_int),
            > = c.sym("png_set_PLTE");
            unsafe { g(png, info, pal.as_ptr(), npal as c_int) };
            let trans: Vec<u8> = (0..npal).map(|i| (i * 31 % 256) as u8).collect();
            type Ft = unsafe extern "C-unwind" fn(
                png_structp, png_infop, *const u8, c_int, *const png_color_16,
            );
            let g: libloading::Symbol<Ft> = c.sym("png_set_tRNS");
            unsafe { g(png, info, trans.as_ptr(), npal as c_int, std::ptr::null()) };
        }
        let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, i32)> =
            c.sym("png_set_gAMA_fixed");
        unsafe { g(png, info, 45455) };
        c.call2("png_write_info");
        let mut ptrs: Vec<*mut u8> = rows.iter().map(|x| x.as_ptr() as *mut u8).collect();
        let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32)> =
            c.sym("png_write_image");
        unsafe { g(png, ptrs.as_mut_ptr(), h) };
        c.call2("png_write_end");
    });
    assert!(!out.errored, "encode failed: {:?}", out.diag);
    out.bytes
}

/* ------------------------------- float variants of the read transforms */

#[test]
fn float_read_transform_setters() {
    let l = libs();
    for (ct, bd) in formats() {
        let data = encode(12, 4, bd, ct, PNG_INTERLACE_NONE, 77 + bd as u32 + ct as u32);
        #[derive(Clone, Copy, Debug)]
        enum Which {
            Gamma(f64, f64),
            Background(c_int, f64),
            AlphaMode(c_int, f64),
            RgbToGray(c_int, f64, f64),
            AddAlpha(u32, c_int),
        }
        let mut cases = Vec::new();
        for &(s, f) in &[(2.2f64, 0.45455f64), (1.0, 1.0), (0.45455, 2.2), (1.8, 2.2)] {
            cases.push(Which::Gamma(s, f));
        }
        for &k in &[
            PNG_BACKGROUND_GAMMA_SCREEN,
            PNG_BACKGROUND_GAMMA_FILE,
            PNG_BACKGROUND_GAMMA_UNIQUE,
        ] {
            cases.push(Which::Background(k, 1.0));
        }
        for &m in &[PNG_ALPHA_PNG, PNG_ALPHA_STANDARD, PNG_ALPHA_BROKEN, PNG_ALPHA_OPTIMIZED] {
            cases.push(Which::AlphaMode(m, 2.2));
        }
        cases.push(Which::RgbToGray(1, 0.2126, 0.7152));
        cases.push(Which::RgbToGray(2, -1.0, -1.0));
        cases.push(Which::AddAlpha(0xffff, PNG_FILLER_AFTER));
        cases.push(Which::AddAlpha(0, PNG_FILLER_BEFORE));

        for case in cases {
            let run = |lib: &Lib| {
                read_with(lib, &data, |c, out| {
                    let png = c.png;
                    c.call2("png_read_info");
                    match case {
                        Which::Gamma(s, f) => {
                            let g: libloading::Symbol<
                                unsafe extern "C-unwind" fn(png_structp, f64, f64),
                            > = c.sym("png_set_gamma");
                            unsafe { g(png, s, f) };
                        }
                        Which::Background(k, gam) => {
                            c.call1("png_set_expand");
                            type F = unsafe extern "C-unwind" fn(
                                png_structp, *const png_color_16, c_int, c_int, f64,
                            );
                            let g: libloading::Symbol<F> = c.sym("png_set_background");
                            let bg = png_color_16 {
                                index: 1,
                                red: 100,
                                green: 150,
                                blue: 200,
                                gray: 120,
                            };
                            unsafe { g(png, &bg, k, 0, gam) };
                        }
                        Which::AlphaMode(m, gam) => {
                            let g: libloading::Symbol<
                                unsafe extern "C-unwind" fn(png_structp, c_int, f64),
                            > = c.sym("png_set_alpha_mode");
                            unsafe { g(png, m, gam) };
                        }
                        Which::RgbToGray(e, r, gg) => {
                            c.call1("png_set_expand");
                            let g: libloading::Symbol<
                                unsafe extern "C-unwind" fn(png_structp, c_int, f64, f64),
                            > = c.sym("png_set_rgb_to_gray");
                            unsafe { g(png, e, r, gg) };
                        }
                        Which::AddAlpha(v, loc) => {
                            let g: libloading::Symbol<
                                unsafe extern "C-unwind" fn(png_structp, u32, c_int),
                            > = c.sym("png_set_add_alpha");
                            unsafe { g(png, v, loc) };
                        }
                    }
                    c.call2("png_read_update_info");
                    out.notes.extend(snapshot_info(c));
                    let rb: usize = {
                        let f: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp, png_infop) -> usize,
                        > = c.sym("png_get_rowbytes");
                        unsafe { f(png, c.info) }
                    };
                    let mut bufs: Vec<Vec<u8>> = (0..4).map(|_| vec![0x5au8; rb + 64]).collect();
                    let rr: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, *mut u8, *mut u8),
                    > = c.sym("png_read_row");
                    for b in bufs.iter_mut() {
                        unsafe { rr(png, b.as_mut_ptr(), std::ptr::null_mut()) };
                    }
                    out.rows = bufs;
                    c.call2("png_read_end");
                })
            };
            let a = run(&l.c);
            let b = run(&l.r);
            let ctx = format!("ct={ct} bd={bd} {case:?}");
            assert_eq!(a.diag, b.diag, "float-tr/{ctx}: diag differs");
            assert_eq!(a.errored, b.errored, "float-tr/{ctx}: error differs");
            assert_snapshots_eq(&format!("float-tr/{ctx}"), &a.notes, &b.notes);
            assert_eq!(a.rows, b.rows, "float-tr/{ctx}: rows differ");
        }
    }
}

/* ------------------------------------------- png_read_rows / write_rows */

#[test]
fn read_rows_and_display_rows() {
    let l = libs();
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let (w, h) = (17u32, 6u32);
            let data = encode(w, h, bd, ct, il, 300 + bd as u32 + ct as u32);
            // three shapes: rows only, display only, both
            for mode in 0..3 {
                let run = |lib: &Lib| {
                    read_with(lib, &data, |c, out| {
                        let png = c.png;
                        c.call2("png_read_info");
                        let ih: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp) -> c_int,
                        > = c.sym("png_set_interlace_handling");
                        let passes = unsafe { ih(png) };
                        c.call2("png_read_update_info");
                        let rb: usize = {
                            let f: libloading::Symbol<
                                unsafe extern "C-unwind" fn(png_structp, png_infop) -> usize,
                            > = c.sym("png_get_rowbytes");
                            unsafe { f(png, c.info) }
                        };
                        out.notes.push(format!("passes={passes} rb={rb}"));
                        let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0x11u8; rb + 32]).collect();
                        let mut disp: Vec<Vec<u8>> = (0..h).map(|_| vec![0x22u8; rb + 32]).collect();
                        let f: libloading::Symbol<
                            unsafe extern "C-unwind" fn(
                                png_structp,
                                *mut *mut u8,
                                *mut *mut u8,
                                u32,
                            ),
                        > = c.sym("png_read_rows");
                        for _ in 0..passes {
                            let mut rp: Vec<*mut u8> =
                                rows.iter_mut().map(|b| b.as_mut_ptr()).collect();
                            let mut dp: Vec<*mut u8> =
                                disp.iter_mut().map(|b| b.as_mut_ptr()).collect();
                            unsafe {
                                match mode {
                                    0 => f(png, rp.as_mut_ptr(), std::ptr::null_mut(), h),
                                    1 => f(png, std::ptr::null_mut(), dp.as_mut_ptr(), h),
                                    _ => f(png, rp.as_mut_ptr(), dp.as_mut_ptr(), h),
                                }
                            }
                        }
                        out.rows = rows;
                        out.rows.extend(disp);
                        c.call2("png_read_end");
                        out.notes.extend(snapshot_info(c));
                    })
                };
                let a = run(&l.c);
                let b = run(&l.r);
                let ctx = format!("ct={ct} bd={bd} il={il} mode={mode}");
                assert_eq!(a.diag, b.diag, "read_rows/{ctx}: diag differs");
                assert_eq!(a.errored, b.errored, "read_rows/{ctx}: error differs");
                assert_snapshots_eq(&format!("read_rows/{ctx}"), &a.notes, &b.notes);
                assert_eq!(a.rows, b.rows, "read_rows/{ctx}: rows differ");
            }
        }
    }
}

#[test]
fn write_rows_and_info_before_plte() {
    let l = libs();
    for (ct, bd) in formats() {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let (w, h) = (14u32, 5u32);
            let rb = rowbytes(channels(ct) * bd as u32, w);
            let mut s: u32 = 909 + bd as u32;
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
                write_with(lib, |c, notes| {
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
                            .map(|i| png_color { red: i as u8, green: 7, blue: 3 })
                            .collect();
                        let g: libloading::Symbol<
                            unsafe extern "C-unwind" fn(
                                png_structp, png_infop, *const png_color, c_int,
                            ),
                        > = c.sym("png_set_PLTE");
                        unsafe { g(png, info, pal.as_ptr(), npal as c_int) };
                    }
                    // the two-stage header write
                    c.call2("png_write_info_before_PLTE");
                    notes.push("before_PLTE done".to_string());
                    c.call2("png_write_info");
                    let ih: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> c_int> =
                        c.sym("png_set_interlace_handling");
                    let passes = unsafe { ih(png) };
                    notes.push(format!("passes={passes}"));
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32),
                    > = c.sym("png_write_rows");
                    for _ in 0..passes {
                        let mut ptrs: Vec<*mut u8> =
                            rows.iter().map(|x| x.as_ptr() as *mut u8).collect();
                        unsafe { g(png, ptrs.as_mut_ptr(), h) };
                    }
                    c.call1("png_write_flush");
                    c.call2("png_write_end");
                })
            };
            let a = run(&l.c);
            let b = run(&l.r);
            let ctx = format!("ct={ct} bd={bd} il={il}");
            assert_eq!(a.diag, b.diag, "write_rows/{ctx}: diag differs");
            assert_eq!(a.errored, b.errored, "write_rows/{ctx}: error differs");
            assert_snapshots_eq(&format!("write_rows/{ctx}"), &a.notes, &b.notes);
            assert_eq!(a.flushes, b.flushes, "write_rows/{ctx}: flushes differ");
            assert_eq!(a.bytes, b.bytes, "write_rows/{ctx}: bytes differ");
        }
    }
}

/* --------------------------------------------------------- png_check_IHDR */

#[test]
fn check_ihdr() {
    let l = libs();
    let widths = [0u32, 1, 7, 0x7fffffff, 0x80000000, 0xffffffff, 1000001];
    let heights = widths;
    let depths = [0i32, 1, 2, 3, 4, 8, 16, 17, 32];
    let types = [0i32, 1, 2, 3, 4, 5, 6, 7, 8];
    for &w in &widths {
        for &h in &heights {
            for &bd in &depths {
                for &ct in &types {
                    for &(il, cm, fm) in &[(0i32, 0i32, 0i32), (1, 0, 0), (2, 1, 1)] {
                        let run = |lib: &Lib| -> (bool, Diag) {
                            diag_reset();
                            let create: libloading::Symbol<FnCreateRead> =
                                lib.sym("png_create_write_struct");
                            let ver = cs(PNG_LIBPNG_VER_STRING);
                            let png = unsafe {
                                create(
                                    ver.as_ptr(),
                                    std::ptr::null_mut(),
                                    Some(error_cb),
                                    Some(warning_cb),
                                )
                            };
                            type F = unsafe extern "C-unwind" fn(
                                png_structp, u32, u32, c_int, c_int, c_int, c_int, c_int,
                            );
                            let f: libloading::Symbol<F> = lib.sym("png_check_IHDR");
                            let r = guard(|| unsafe { f(png, w, h, bd, ct, il, cm, fm) });
                            let destroy: libloading::Symbol<FnDestroyWrite> =
                                lib.sym("png_destroy_write_struct");
                            let mut p = png;
                            let _ =
                                guard(|| unsafe { destroy(&mut p, std::ptr::null_mut()) });
                            (r.is_err(), diag_take())
                        };
                        let a = run(&l.c);
                        let b = run(&l.r);
                        assert_eq!(
                            a, b,
                            "png_check_IHDR({w},{h},{bd},{ct},{il},{cm},{fm}) differs"
                        );
                    }
                }
            }
        }
    }
}

/* ------------------------------------------------------- png_gamma_correct */

#[test]
fn gamma_correct_with_struct() {
    let l = libs();
    for bd in [8u8, 16] {
        for gv in [0i32, 1, 45455, 100000, 220000, -10000, i32::MAX] {
            let run = |lib: &Lib| -> (Vec<u16>, Diag, bool) {
                diag_reset();
                let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_write_struct");
                let ver = cs(PNG_LIBPNG_VER_STRING);
                let png = unsafe {
                    create(ver.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
                };
                let create_info: libloading::Symbol<FnCreateInfo> = lib.sym("png_create_info_struct");
                let info = unsafe { create_info(png) };
                let ctx = Ctx { lib, png, info };
                let out = guard(|| {
                    type Fihdr = unsafe extern "C-unwind" fn(
                        png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int,
                    );
                    let f: libloading::Symbol<Fihdr> = ctx.sym("png_set_IHDR");
                    unsafe {
                        f(png, info, 4, 4, bd as c_int, PNG_COLOR_TYPE_GRAY as c_int,
                          PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE)
                    };
                    let g: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, u32, i32) -> u16,
                    > = ctx.sym("png_gamma_correct");
                    let mut v = Vec::new();
                    let limit: u32 = if bd == 8 { 256 } else { 65536 };
                    let step = if bd == 8 { 1u32 } else { 251 };
                    let mut i = 0u32;
                    while i < limit {
                        v.push(unsafe { g(png, i, gv) });
                        i += step;
                    }
                    v
                });
                let destroy: libloading::Symbol<FnDestroyWrite> = lib.sym("png_destroy_write_struct");
                let mut p = png;
                let mut i = info;
                let _ = guard(|| unsafe { destroy(&mut p, &mut i) });
                (out.clone().unwrap_or_default(), diag_take(), out.is_err())
            };
            let a = run(&l.c);
            let b = run(&l.r);
            assert_eq!(a, b, "png_gamma_correct(bd={bd}, gamma={gv}) differs");
        }
    }
}

/* ------------------------------------------------------------ png_set_mem_fn */

thread_local! {
    static ALLOCS: std::cell::RefCell<Vec<(usize, usize)>> = std::cell::RefCell::new(Vec::new());
    static COUNT: std::cell::Cell<usize> = std::cell::Cell::new(0);
}

unsafe extern "C-unwind" fn m_alloc(_p: png_structp, size: usize) -> png_voidp {
    let mut v: Vec<u8> = Vec::with_capacity(size.max(1));
    let ptr = v.as_mut_ptr();
    let cap = v.capacity();
    std::mem::forget(v);
    ALLOCS.with(|a| a.borrow_mut().push((ptr as usize, cap)));
    COUNT.with(|c| c.set(c.get() + 1));
    ptr as png_voidp
}

unsafe extern "C-unwind" fn m_free(_p: png_structp, ptr: png_voidp) {
    if ptr.is_null() {
        return;
    }
    ALLOCS.with(|a| {
        let mut a = a.borrow_mut();
        if let Some(i) = a.iter().position(|&(x, _)| x == ptr as usize) {
            let (x, cap) = a.remove(i);
            unsafe { drop(Vec::from_raw_parts(x as *mut u8, 0, cap)) };
        }
    });
}

#[test]
fn set_mem_fn_and_create_read_struct_2() {
    let l = libs();
    let data = encode(10, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE, 5);
    let run = |lib: &Lib| -> (Vec<Vec<u8>>, Diag, bool, usize) {
        diag_reset();
        ALLOCS.with(|a| a.borrow_mut().clear());
        COUNT.with(|c| c.set(0));
        let mut src = Box::new(MemReader { data: data.clone(), pos: 0 });
        type FnCreate2 = unsafe extern "C-unwind" fn(
            *const c_char,
            png_voidp,
            Option<unsafe extern "C-unwind" fn(png_structp, *const c_char)>,
            Option<unsafe extern "C-unwind" fn(png_structp, *const c_char)>,
            png_voidp,
            Option<unsafe extern "C-unwind" fn(png_structp, usize) -> png_voidp>,
            Option<unsafe extern "C-unwind" fn(png_structp, png_voidp)>,
        ) -> png_structp;
        let create: libloading::Symbol<FnCreate2> = lib.sym("png_create_read_struct_2");
        let ver = cs(PNG_LIBPNG_VER_STRING);
        let png = unsafe {
            create(
                ver.as_ptr(),
                std::ptr::null_mut(),
                Some(error_cb),
                Some(warning_cb),
                std::ptr::null_mut(),
                Some(m_alloc),
                Some(m_free),
            )
        };
        assert!(!png.is_null());
        // re-register the same hooks through png_set_mem_fn
        let setmem: libloading::Symbol<
            unsafe extern "C-unwind" fn(
                png_structp,
                png_voidp,
                Option<unsafe extern "C-unwind" fn(png_structp, usize) -> png_voidp>,
                Option<unsafe extern "C-unwind" fn(png_structp, png_voidp)>,
            ),
        > = lib.sym("png_set_mem_fn");
        unsafe { setmem(png, 0x4321 as png_voidp, Some(m_alloc), Some(m_free)) };
        let getmem: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> png_voidp> =
            lib.sym("png_get_mem_ptr");
        assert_eq!(unsafe { getmem(png) } as usize, 0x4321);

        let create_info: libloading::Symbol<FnCreateInfo> = lib.sym("png_create_info_struct");
        let info = unsafe { create_info(png) };
        sink_register(png, (&mut *src) as *mut MemReader as *mut c_void);
        let set_read: libloading::Symbol<FnSetReadFn> = lib.sym("png_set_read_fn");
        unsafe {
            set_read(png, (&mut *src) as *mut MemReader as *mut c_void, Some(mem_read))
        };
        let ctx = Ctx { lib, png, info };
        let out = guard(|| {
            ctx.call2("png_read_info");
            ctx.call2("png_read_update_info");
            let rb: usize = {
                let f: libloading::Symbol<
                    unsafe extern "C-unwind" fn(png_structp, png_infop) -> usize,
                > = ctx.sym("png_get_rowbytes");
                unsafe { f(png, info) }
            };
            let mut bufs: Vec<Vec<u8>> = (0..4).map(|_| vec![0u8; rb]).collect();
            let rr: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut u8, *mut u8)> =
                ctx.sym("png_read_row");
            for b in bufs.iter_mut() {
                unsafe { rr(png, b.as_mut_ptr(), std::ptr::null_mut()) };
            }
            ctx.call2("png_read_end");
            bufs
        });
        let destroy: libloading::Symbol<FnDestroyRead> = lib.sym("png_destroy_read_struct");
        let mut p = png;
        let mut i = info;
        let _ = guard(|| unsafe { destroy(&mut p, &mut i, std::ptr::null_mut()) });
        sink_clear();
        let leaks = ALLOCS.with(|a| a.borrow().len());
        assert_eq!(leaks, 0, "custom allocator leaked {leaks} blocks");
        let n = COUNT.with(|c| c.get());
        assert!(n > 0, "custom allocator unused");
        (out.clone().unwrap_or_default(), diag_take(), out.is_err(), 0)
    };
    let a = run(&l.c);
    let b = run(&l.r);
    assert_eq!(a.1, b.1, "diag differs");
    assert_eq!(a.2, b.2, "error differs");
    assert_eq!(a.0, b.0, "rows differ");
}

/* ------------------------------------------------- deprecated accessors */

#[test]
fn deprecated_accessors() {
    let l = libs();
    // png_convert_to_rfc1123 uses a buffer inside png_struct
    let times = [
        png_time { year: 2024, month: 12, day: 25, hour: 1, minute: 2, second: 3 },
        png_time { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 65535, month: 13, day: 32, hour: 25, minute: 61, second: 61 },
        png_time { year: 9999, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 10000, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
    ];
    for t in &times {
        let run = |lib: &Lib| -> (Option<String>, Diag, bool) {
            diag_reset();
            let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_write_struct");
            let ver = cs(PNG_LIBPNG_VER_STRING);
            let png = unsafe {
                create(ver.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
            };
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, *const png_time) -> *const c_char,
            > = lib.sym("png_convert_to_rfc1123");
            let r = guard(|| cstr_of(unsafe { f(png, t) }));
            let destroy: libloading::Symbol<FnDestroyWrite> = lib.sym("png_destroy_write_struct");
            let mut p = png;
            let _ = guard(|| unsafe { destroy(&mut p, std::ptr::null_mut()) });
            (r.clone().unwrap_or(None), diag_take(), r.is_err())
        };
        assert_eq!(run(&l.c), run(&l.r), "png_convert_to_rfc1123({t:?}) differs");
    }

    // png_get_eXIf (the deprecated two-output form) and png_get_progressive_ptr
    let data = encode(8, 3, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE, 1);
    let run = |lib: &Lib| -> Vec<String> {
        let mut notes = Vec::new();
        let out = read_with(lib, &data, |c, _| {
            c.call2("png_read_info");
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *mut *mut u8) -> u32,
            > = c.sym("png_get_eXIf");
            let mut p: *mut u8 = std::ptr::null_mut();
            let r = unsafe { f(c.png, c.info, &mut p) };
            notes.push(format!("get_eXIf r={r} null={}", p.is_null()));
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> png_voidp> =
                c.sym("png_get_progressive_ptr");
            notes.push(format!("progressive_ptr={:#x}", unsafe { g(c.png) } as usize));
        });
        notes.push(format!("{:?} {}", out.diag, out.errored));
        notes
    };
    assert_eq!(run(&l.c), run(&l.r), "deprecated accessors differ");
}

/* -------------------------------------------------- stdio simplified API */

#[test]
fn stdio_simplified_api() {
    let l = libs();
    let dir = std::env::temp_dir().join(format!("pngdiff-stdio-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (w, h) = (7u32, 3u32);
    let fmt = PNG_FORMAT_RGBA;
    let pixels: Vec<u8> = (0..(w * h * 4) as usize).map(|i| (i * 7 % 251) as u8).collect();

    let mut files: Vec<Vec<u8>> = Vec::new();
    for (tag, lib) in [("c", &l.c), ("r", &l.r)] {
        let path = dir.join(format!("stdio-{tag}.png"));
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let mode = CString::new("wb").unwrap();
        let fp = unsafe { fopen(cpath.as_ptr(), mode.as_ptr()) };
        assert!(!fp.is_null());
        let mut image = png_image::default();
        image.width = w;
        image.height = h;
        image.format = fmt;
        let f: libloading::Symbol<
            unsafe extern "C-unwind" fn(
                *mut png_image,
                *mut c_void,
                c_int,
                *const c_void,
                i32,
                *const c_void,
            ) -> c_int,
        > = lib.sym("png_image_write_to_stdio");
        let r = unsafe {
            f(&mut image, fp, 0, pixels.as_ptr() as *const c_void, 0, std::ptr::null())
        };
        assert_ne!(r, 0, "{tag}: png_image_write_to_stdio failed");
        let free: libloading::Symbol<unsafe extern "C-unwind" fn(*mut png_image)> =
            lib.sym("png_image_free");
        unsafe { free(&mut image) };
        unsafe { fclose(fp) };
        files.push(std::fs::read(&path).unwrap());
    }
    assert_eq!(files[0], files[1], "png_image_write_to_stdio output differs");

    let path = dir.join("stdio-c.png");
    let read = |lib: &Lib| -> (String, Vec<u8>, c_int) {
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let mode = CString::new("rb").unwrap();
        let fp = unsafe { fopen(cpath.as_ptr(), mode.as_ptr()) };
        assert!(!fp.is_null());
        let mut image = png_image::default();
        let f: libloading::Symbol<
            unsafe extern "C-unwind" fn(*mut png_image, *mut c_void) -> c_int,
        > = lib.sym("png_image_begin_read_from_stdio");
        let r1 = unsafe { f(&mut image, fp) };
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
                g(&mut image, std::ptr::null(), buf.as_mut_ptr() as *mut c_void, 0, std::ptr::null_mut())
            };
        }
        let snap = format!("{} {} {:#x} {}", image.width, image.height, image.format, image.warning_or_error);
        let free: libloading::Symbol<unsafe extern "C-unwind" fn(*mut png_image)> =
            lib.sym("png_image_free");
        unsafe { free(&mut image) };
        unsafe { fclose(fp) };
        (snap, buf, r1 * 10 + r2)
    };
    let a = read(&l.c);
    let b = read(&l.r);
    assert_eq!(a.2, b.2, "stdio read return differs");
    assert_eq!(a.0, b.0, "stdio read image differs");
    assert_eq!(a.1, b.1, "stdio read pixels differ");
    let _ = std::fs::remove_dir_all(&dir);
}

unsafe extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}
