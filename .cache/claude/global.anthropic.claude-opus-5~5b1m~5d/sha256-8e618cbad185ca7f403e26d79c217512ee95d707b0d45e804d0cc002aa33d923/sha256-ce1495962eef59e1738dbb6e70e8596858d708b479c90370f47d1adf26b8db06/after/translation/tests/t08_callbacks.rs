//! Phase B — CONFIGS.md section F: memory management and every application
//! callback hook (user allocator, status callbacks, user transforms, user chunk
//! handler, error pointer, IO state).
mod common;
use common::*;
use std::cell::RefCell;

// ---------------------------------------------------------------------------
// A user allocator that records every request, so the two libraries' malloc
// call sequences can be compared exactly.
// ---------------------------------------------------------------------------

thread_local! {
    static ALLOC_LOG: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    static FREE_COUNT: RefCell<usize> = const { RefCell::new(0) };
    static FAIL_AFTER: RefCell<usize> = const { RefCell::new(usize::MAX) };
    static ROW_LOG: RefCell<Vec<(u32, c_int)>> = const { RefCell::new(Vec::new()) };
    static XFORM_LOG: RefCell<Vec<(png_row_info, Vec<u8>)>> = const {
        RefCell::new(Vec::new())
    };
    static CHUNK_LOG: RefCell<Vec<(Vec<u8>, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
}

unsafe extern "C-unwind" fn my_malloc(_png: png_structp, size: usize) -> png_voidp {
    let n = ALLOC_LOG.with(|l| {
        l.borrow_mut().push(size);
        l.borrow().len()
    });
    if n > FAIL_AFTER.with(|f| *f.borrow()) {
        return std::ptr::null_mut();
    }
    // Deliberately zero the block: libpng must not depend on malloc garbage
    // for anything we compare.
    let layout = std::alloc::Layout::from_size_align(size.max(1) + 16, 16).unwrap();
    let p = std::alloc::alloc_zeroed(layout);
    // store the size just before the returned pointer so free can recover it
    (p as *mut usize).write(size.max(1) + 16);
    p.add(16) as png_voidp
}

unsafe extern "C-unwind" fn my_free(_png: png_structp, ptr: png_voidp) {
    if ptr.is_null() {
        return;
    }
    FREE_COUNT.with(|c| *c.borrow_mut() += 1);
    let base = (ptr as *mut u8).sub(16);
    let total = (base as *mut usize).read();
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    std::alloc::dealloc(base, layout);
}

unsafe extern "C-unwind" fn my_row_status(_png: png_structp, row: png_uint_32, pass: c_int) {
    ROW_LOG.with(|l| l.borrow_mut().push((row, pass)));
}

unsafe extern "C-unwind" fn my_user_transform(
    _png: png_structp,
    row_info: png_row_infop,
    data: png_bytep,
) {
    unsafe {
        let ri = *row_info;
        let n = ri.rowbytes;
        let bytes = std::slice::from_raw_parts(data, n).to_vec();
        XFORM_LOG.with(|l| l.borrow_mut().push((ri, bytes)));
        // Mutate deterministically so the effect is visible in the output.
        for i in 0..n {
            *data.add(i) = (*data.add(i)).wrapping_add(0x11).rotate_left(1);
        }
    }
}

unsafe extern "C-unwind" fn my_user_chunk(_png: png_structp, ch: png_unknown_chunkp) -> c_int {
    unsafe {
        let c = &*ch;
        let data = if c.data.is_null() || c.size == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(c.data, c.size).to_vec()
        };
        CHUNK_LOG.with(|l| l.borrow_mut().push((c.name.to_vec(), data)));
        // 1 = handled, 0 = not handled, negative = error
        if c.name[0] == b'q' {
            -1
        } else if c.name[0] == b'r' {
            0
        } else {
            1
        }
    }
}

fn reset_logs() {
    ALLOC_LOG.with(|l| l.borrow_mut().clear());
    FREE_COUNT.with(|c| *c.borrow_mut() = 0);
    ROW_LOG.with(|l| l.borrow_mut().clear());
    XFORM_LOG.with(|l| l.borrow_mut().clear());
    CHUNK_LOG.with(|l| l.borrow_mut().clear());
}

// ---------------------------------------------------------------------------
// A minimal valid PNG, produced by the C write path.
// ---------------------------------------------------------------------------

unsafe fn make_png(
    rng: &mut Rng,
    ct: c_int,
    bd: c_int,
    w: u32,
    h: u32,
    il: c_int,
    unknowns: &[(&[u8; 4], Vec<u8>, c_int)],
) -> Vec<u8> {
    let api = c_api();
    set_current_api(api);
    diag_reset();
    let mut sess = WriteSess::new(api);
    let (png, info) = (sess.png, sess.info);
    let pd = channels_of(ct) * bd as u32;
    let rows: Vec<Vec<u8>> = (0..h).map(|_| rng.bytes(rowbytes(pd, w))).collect();
    let npal = if ct == PNG_COLOR_TYPE_PALETTE {
        1usize << bd
    } else {
        0
    };
    let palette: Vec<png_color> = (0..npal)
        .map(|_| png_color {
            red: rng.u8(),
            green: rng.u8(),
            blue: rng.u8(),
        })
        .collect();
    let mut datas: Vec<Vec<u8>> = Vec::new();
    let mut chunks: Vec<png_unknown_chunk> = Vec::new();
    guard(|| {
        (api.png_set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
        if !palette.is_empty() {
            (api.png_set_PLTE)(png, info, palette.as_ptr(), palette.len() as c_int);
        }
        (api.png_set_gAMA_fixed)(png, info, 45455);
        for (name, data, loc) in unknowns {
            datas.push(data.clone());
            let mut nm = [0u8; 5];
            nm[..4].copy_from_slice(&name[..]);
            chunks.push(png_unknown_chunk {
                name: nm,
                data: datas.last().unwrap().as_ptr() as *mut png_byte,
                size: datas.last().unwrap().len(),
                location: *loc as u8,
            });
        }
        if !chunks.is_empty() {
            (api.png_set_keep_unknown_chunks)(
                png,
                PNG_HANDLE_CHUNK_ALWAYS,
                std::ptr::null(),
                0,
            );
            (api.png_set_unknown_chunks)(png, info, chunks.as_ptr(), chunks.len() as c_int);
            for i in 0..chunks.len() {
                (api.png_set_unknown_chunk_location)(
                    png,
                    info,
                    i as c_int,
                    chunks[i].location as c_int,
                );
            }
        }
        (api.png_write_info)(png, info);
        let mut rp: Vec<png_bytep> = rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
        (api.png_write_image)(png, rp.as_mut_ptr());
        (api.png_write_end)(png, info);
    });
    let _ = diag_take();
    std::mem::take(&mut sess.sink.buf)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn user_allocator_read_and_write() {
    let mut rng = Rng::new(0x1234_9876_5432_1001);
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let bytes = unsafe { make_png(&mut rng, ct, bd, 11, 5, il, &[]) };
            // --- read with a user allocator (png_create_read_struct_2) ---
            let mut outs = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    reset_logs();
                    let v = ver();
                    let png = (api.png_create_read_struct_2)(
                        v.as_ptr(),
                        std::ptr::null_mut(),
                        Some(cb_error),
                        Some(cb_warning),
                        std::ptr::null_mut(),
                        Some(my_malloc),
                        Some(my_free),
                    );
                    assert!(!png.is_null());
                    let info = (api.png_create_info_struct)(png);
                    let mut src = Box::new(ReadSource {
                        data: bytes.clone(),
                        pos: 0,
                    });
                    (api.png_set_read_fn)(
                        png,
                        &mut *src as *mut ReadSource as png_voidp,
                        Some(cb_read),
                    );
                    let mut rows: Vec<Vec<u8>> = Vec::new();
                    let ok = guard(|| {
                        (api.png_read_info)(png, info);
                        let rbz = (api.png_get_rowbytes)(png, info);
                        let h = (api.png_get_image_height)(png, info);
                        let np = if il == PNG_INTERLACE_ADAM7 {
                            (api.png_set_interlace_handling)(png)
                        } else {
                            1
                        };
                        let mut buf: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                        for _ in 0..np {
                            for y in 0..h as usize {
                                (api.png_read_row)(
                                    png,
                                    buf[y].as_mut_ptr(),
                                    std::ptr::null_mut(),
                                );
                            }
                        }
                        (api.png_read_end)(png, std::ptr::null_mut());
                        rows = buf;
                    })
                    .is_some();
                    // png_get_mem_ptr must round-trip
                    let mp = (api.png_get_mem_ptr)(png);
                    let mut p = png;
                    let mut i = info;
                    (api.png_destroy_read_struct)(&mut p, &mut i, std::ptr::null_mut());
                    outs.push((
                        ok,
                        diag_take(),
                        rows,
                        mp.is_null(),
                        FREE_COUNT.with(|c| *c.borrow()) > 0,
                    ));
                }
            }
            assert_eq!(outs[0].0, outs[1].0, "alloc read parity ct={} bd={}", ct, bd);
            assert_eq!(outs[0].1, outs[1].1, "alloc read diag");
            assert_eq!(outs[0].2, outs[1].2, "alloc read rows");
            assert_eq!(outs[0].3, outs[1].3, "alloc read mem_ptr");
            assert_eq!(outs[0].4, outs[1].4, "alloc read frees happened");

            // --- write with a user allocator (png_create_write_struct_2) ---
            let mut wouts = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    reset_logs();
                    let v = ver();
                    let png = (api.png_create_write_struct_2)(
                        v.as_ptr(),
                        std::ptr::null_mut(),
                        Some(cb_error),
                        Some(cb_warning),
                        std::ptr::null_mut(),
                        Some(my_malloc),
                        Some(my_free),
                    );
                    assert!(!png.is_null());
                    let info = (api.png_create_info_struct)(png);
                    let mut sink = Box::new(WriteSink {
                        buf: Vec::new(),
                        flushes: 0,
                    });
                    (api.png_set_write_fn)(
                        png,
                        &mut *sink as *mut WriteSink as png_voidp,
                        Some(cb_write),
                        Some(cb_flush),
                    );
                    let pd = channels_of(ct) * bd as u32;
                    let mut r2 = Rng::new(0xfeed_0000_0000_0001);
                    let rows: Vec<Vec<u8>> =
                        (0..5).map(|_| r2.bytes(rowbytes(pd, 11) + 16)).collect();
                    let npal = if ct == PNG_COLOR_TYPE_PALETTE {
                        1usize << bd
                    } else {
                        0
                    };
                    let palette: Vec<png_color> = (0..npal)
                        .map(|i| png_color {
                            red: i as u8,
                            green: (255 - i) as u8,
                            blue: (i * 3) as u8,
                        })
                        .collect();
                    let ok = guard(|| {
                        (api.png_set_IHDR)(png, info, 11, 5, bd, ct, il, 0, 0);
                        if !palette.is_empty() {
                            (api.png_set_PLTE)(
                                png,
                                info,
                                palette.as_ptr(),
                                palette.len() as c_int,
                            );
                        }
                        (api.png_write_info)(png, info);
                        let mut rp: Vec<png_bytep> =
                            rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
                        (api.png_write_image)(png, rp.as_mut_ptr());
                        (api.png_write_end)(png, info);
                    })
                    .is_some();
                    let mut p = png;
                    let mut i = info;
                    (api.png_destroy_write_struct)(&mut p, &mut i);
                    wouts.push((ok, diag_take(), std::mem::take(&mut sink.buf)));
                }
            }
            assert_eq!(wouts[0].0, wouts[1].0, "alloc write parity");
            assert_eq!(wouts[0].1, wouts[1].1, "alloc write diag");
            assert_bytes_eq(
                &format!("alloc write bytes ct={} bd={} il={}", ct, bd, il),
                &wouts[0].2,
                &wouts[1].2,
            );
        }
    }
}

#[test]
fn set_mem_fn_and_error_fn() {
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let s = ReadSess::new(api, &[]);
            let tag = 0x1234_5678usize;
            // NOTE: installing a *different* allocator on a struct whose
            // existing blocks came from the default one would make
            // png_destroy_read_struct free them with the wrong deallocator
            // (inherent to the API, not an error path), so pass NULL fns here:
            // png_free/png_malloc fall back to the system allocator when the
            // hooks are NULL (pngmem.c).
            (api.png_set_mem_fn)(s.png, tag as png_voidp, None, None);
            assert_eq!((api.png_get_mem_ptr)(s.png) as usize, tag, "{}", api.name);
            (api.png_set_error_fn)(
                s.png,
                (tag + 1) as png_voidp,
                Some(cb_error),
                Some(cb_warning),
            );
            assert_eq!(
                (api.png_get_error_ptr)(s.png) as usize,
                tag + 1,
                "{}",
                api.name
            );
            // NULL guards
            (api.png_set_mem_fn)(std::ptr::null_mut(), std::ptr::null_mut(), None, None);
            (api.png_set_error_fn)(std::ptr::null_mut(), std::ptr::null_mut(), None, None);
            assert!((api.png_get_mem_ptr)(std::ptr::null_mut()).is_null());
            assert!((api.png_get_error_ptr)(std::ptr::null()).is_null());
            let _ = diag_take();
        }
    }
}

#[test]
fn allocation_failure_injection() {
    // Make the Nth allocation fail and check both libraries report the same
    // out-of-memory error at the same point.
    let mut rng = Rng::new(0x2222_1111_3333_4444);
    let bytes = unsafe { make_png(&mut rng, PNG_COLOR_TYPE_RGB_ALPHA, 8, 9, 4, 0, &[]) };
    for fail_after in 0usize..14 {
        let mut outs = Vec::new();
        for api in both() {
            unsafe {
                set_current_api(api);
                diag_reset();
                reset_logs();
                FAIL_AFTER.with(|f| *f.borrow_mut() = fail_after);
                let v = ver();
                let png = (api.png_create_read_struct_2)(
                    v.as_ptr(),
                    std::ptr::null_mut(),
                    Some(cb_error),
                    Some(cb_warning),
                    std::ptr::null_mut(),
                    Some(my_malloc),
                    Some(my_free),
                );
                let mut nrows = 0usize;
                let mut ok = false;
                if !png.is_null() {
                    let info = (api.png_create_info_struct)(png);
                    if !info.is_null() {
                        let mut src = Box::new(ReadSource {
                            data: bytes.clone(),
                            pos: 0,
                        });
                        (api.png_set_read_fn)(
                            png,
                            &mut *src as *mut ReadSource as png_voidp,
                            Some(cb_read),
                        );
                        ok = guard(|| {
                            (api.png_read_info)(png, info);
                            let rbz = (api.png_get_rowbytes)(png, info);
                            let h = (api.png_get_image_height)(png, info);
                            let mut buf: Vec<Vec<u8>> =
                                (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                            let mut ptrs: Vec<png_bytep> =
                                buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                            (api.png_read_image)(png, ptrs.as_mut_ptr());
                            nrows = buf.len();
                        })
                        .is_some();
                    }
                    let mut p = png;
                    (api.png_destroy_read_struct)(
                        &mut p,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                }
                FAIL_AFTER.with(|f| *f.borrow_mut() = usize::MAX);
                // Compare the NUMBER of allocations, not their sizes:
                // sizeof(png_struct)/sizeof(png_info) legitimately differ
                // between the C and the Rust build (they are opaque types).
                outs.push((ok, diag_take(), nrows, ALLOC_LOG.with(|l| l.borrow().len())));
            }
        }
        assert_eq!(
            outs[0].0, outs[1].0,
            "OOM at {} : success parity (C diag {:?} RS diag {:?})",
            fail_after, outs[0].1, outs[1].1
        );
        assert_eq!(outs[0].1, outs[1].1, "OOM at {} : diagnostics", fail_after);
        assert_eq!(outs[0].2, outs[1].2, "OOM at {} : rows", fail_after);
        assert_eq!(
            outs[0].3, outs[1].3,
            "OOM at {} : number of allocation requests",
            fail_after
        );
    }
}

#[test]
fn read_and_write_status_callbacks() {
    let mut rng = Rng::new(0x3333_4444_5555_6666);
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let bytes = unsafe { make_png(&mut rng, ct, bd, 13, 6, il, &[]) };
            let mut outs = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    reset_logs();
                    let s = ReadSess::new(api, &bytes);
                    let ok = guard(|| {
                        (api.png_set_read_status_fn)(s.png, Some(my_row_status));
                        (api.png_read_info)(s.png, s.info);
                        let rbz = (api.png_get_rowbytes)(s.png, s.info);
                        let h = (api.png_get_image_height)(s.png, s.info);
                        let mut buf: Vec<Vec<u8>> =
                            (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                        let mut ptrs: Vec<png_bytep> =
                            buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                        (api.png_read_image)(s.png, ptrs.as_mut_ptr());
                        (api.png_read_end)(s.png, s.end);
                    })
                    .is_some();
                    outs.push((ok, diag_take(), ROW_LOG.with(|l| l.borrow().clone())));
                }
            }
            assert_eq!(outs[0].0, outs[1].0, "read status parity");
            assert_eq!(outs[0].1, outs[1].1, "read status diag");
            assert_eq!(
                outs[0].2, outs[1].2,
                "read row status sequence ct={} bd={} il={}",
                ct, bd, il
            );

            // write side
            let mut wouts = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    reset_logs();
                    let mut s = WriteSess::new(api);
                    let pd = channels_of(ct) * bd as u32;
                    let mut r2 = Rng::new(0x5150_0000_0000_0001);
                    let rows: Vec<Vec<u8>> =
                        (0..6).map(|_| r2.bytes(rowbytes(pd, 13) + 16)).collect();
                    let npal = if ct == PNG_COLOR_TYPE_PALETTE {
                        1usize << bd
                    } else {
                        0
                    };
                    let palette: Vec<png_color> = (0..npal)
                        .map(|i| png_color {
                            red: i as u8,
                            green: 0,
                            blue: 0,
                        })
                        .collect();
                    let ok = guard(|| {
                        (api.png_set_write_status_fn)(s.png, Some(my_row_status));
                        (api.png_set_IHDR)(s.png, s.info, 13, 6, bd, ct, il, 0, 0);
                        if !palette.is_empty() {
                            (api.png_set_PLTE)(
                                s.png,
                                s.info,
                                palette.as_ptr(),
                                palette.len() as c_int,
                            );
                        }
                        (api.png_write_info)(s.png, s.info);
                        let mut rp: Vec<png_bytep> =
                            rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
                        (api.png_write_image)(s.png, rp.as_mut_ptr());
                        (api.png_write_end)(s.png, s.info);
                    })
                    .is_some();
                    wouts.push((
                        ok,
                        diag_take(),
                        ROW_LOG.with(|l| l.borrow().clone()),
                        std::mem::take(&mut s.sink.buf),
                    ));
                }
            }
            assert_eq!(wouts[0].0, wouts[1].0, "write status parity");
            assert_eq!(wouts[0].1, wouts[1].1, "write status diag");
            assert_eq!(
                wouts[0].2, wouts[1].2,
                "write row status sequence ct={} bd={} il={}",
                ct, bd, il
            );
            assert_bytes_eq("write status bytes", &wouts[0].3, &wouts[1].3);
            // NULL callback guards
            for api in both() {
                unsafe {
                    (api.png_set_read_status_fn)(std::ptr::null_mut(), None);
                    (api.png_set_write_status_fn)(std::ptr::null_mut(), None);
                }
            }
        }
    }
}

#[test]
fn user_transform_callbacks() {
    let mut rng = Rng::new(0x4444_5555_6666_7777);
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let bytes = unsafe { make_png(&mut rng, ct, bd, 15, 4, il, &[]) };
            // read user transform
            let mut outs = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    reset_logs();
                    let s = ReadSess::new(api, &bytes);
                    let tag = 0xabcd_ef00usize;
                    let mut rows: Vec<Vec<u8>> = Vec::new();
                    let mut uptr_ok = false;
                    let ok = guard(|| {
                        (api.png_read_info)(s.png, s.info);
                        (api.png_set_read_user_transform_fn)(s.png, Some(my_user_transform));
                        // (0, 0) == "the transform does not increase the depth
                        // or channel count".  Passing a LARGER depth/channel
                        // count makes libpng copy more bytes out of its
                        // internal row buffer than exist, which reads
                        // uninitialised memory in both libraries.
                        (api.png_set_user_transform_info)(s.png, tag as png_voidp, 0, 0);
                        uptr_ok = (api.png_get_user_transform_ptr)(s.png) as usize == tag;
                        (api.png_read_update_info)(s.png, s.info);
                        let rbz = (api.png_get_rowbytes)(s.png, s.info);
                        let h = (api.png_get_image_height)(s.png, s.info);
                        let np = if il == PNG_INTERLACE_ADAM7 {
                            (api.png_set_interlace_handling)(s.png)
                        } else {
                            1
                        };
                        let mut buf: Vec<Vec<u8>> =
                            (0..h).map(|_| vec![0u8; rbz + 32]).collect();
                        for _ in 0..np {
                            for y in 0..h as usize {
                                (api.png_read_row)(
                                    s.png,
                                    buf[y].as_mut_ptr(),
                                    std::ptr::null_mut(),
                                );
                            }
                        }
                        (api.png_read_end)(s.png, s.end);
                        rows = buf;
                    })
                    .is_some();
                    outs.push((
                        ok,
                        diag_take(),
                        rows,
                        uptr_ok,
                        XFORM_LOG.with(|l| l.borrow().clone()),
                    ));
                }
            }
            assert_eq!(outs[0].0, outs[1].0, "read user xform parity");
            assert_eq!(outs[0].1, outs[1].1, "read user xform diag");
            assert_eq!(outs[0].3, outs[1].3, "read user transform ptr");
            assert_eq!(
                outs[0].4, outs[1].4,
                "read user transform invocations ct={} bd={} il={}",
                ct, bd, il
            );
            assert_eq!(outs[0].2, outs[1].2, "read user xform rows");

            // write user transform
            let mut wouts = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    reset_logs();
                    let mut s = WriteSess::new(api);
                    let pd = channels_of(ct) * bd as u32;
                    let mut r2 = Rng::new(0x6161_0000_0000_0001);
                    let rows: Vec<Vec<u8>> =
                        (0..4).map(|_| r2.bytes(rowbytes(pd, 15) + 16)).collect();
                    let npal = if ct == PNG_COLOR_TYPE_PALETTE {
                        1usize << bd
                    } else {
                        0
                    };
                    let palette: Vec<png_color> = (0..npal)
                        .map(|i| png_color {
                            red: i as u8,
                            green: 1,
                            blue: 2,
                        })
                        .collect();
                    let ok = guard(|| {
                        (api.png_set_IHDR)(s.png, s.info, 15, 4, bd, ct, il, 0, 0);
                        if !palette.is_empty() {
                            (api.png_set_PLTE)(
                                s.png,
                                s.info,
                                palette.as_ptr(),
                                palette.len() as c_int,
                            );
                        }
                        (api.png_write_info)(s.png, s.info);
                        (api.png_set_write_user_transform_fn)(s.png, Some(my_user_transform));
                        let mut rp: Vec<png_bytep> =
                            rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
                        (api.png_write_image)(s.png, rp.as_mut_ptr());
                        (api.png_write_end)(s.png, s.info);
                    })
                    .is_some();
                    wouts.push((
                        ok,
                        diag_take(),
                        std::mem::take(&mut s.sink.buf),
                        XFORM_LOG.with(|l| l.borrow().clone()),
                    ));
                }
            }
            assert_eq!(wouts[0].0, wouts[1].0, "write user xform parity");
            assert_eq!(wouts[0].1, wouts[1].1, "write user xform diag");
            assert_eq!(
                wouts[0].3, wouts[1].3,
                "write user transform invocations ct={} bd={} il={}",
                ct, bd, il
            );
            assert_bytes_eq("write user xform bytes", &wouts[0].2, &wouts[1].2);
        }
    }
    // NULL guards.  NOTE: png_set_read_user_transform_fn (pngrtran.c:1133) has
    // NO NULL check -- it writes png_ptr->transformations unconditionally, so a
    // NULL png_ptr there is C undefined behaviour and is not tested.
    for api in both() {
        unsafe {
            (api.png_set_write_user_transform_fn)(std::ptr::null_mut(), None);
            (api.png_set_user_transform_info)(std::ptr::null_mut(), std::ptr::null_mut(), 0, 0);
            assert!((api.png_get_user_transform_ptr)(std::ptr::null()).is_null());
        }
    }
}

#[test]
fn user_chunk_callback() {
    let mut rng = Rng::new(0x5555_6666_7777_8888);
    // 'q' -> callback returns -1 (error), 'r' -> 0 (not handled), else 1
    let unknowns: Vec<(&[u8; 4], Vec<u8>, c_int)> = vec![
        (b"prVt", vec![1, 2, 3, 4], 1),
        (b"qbAd", vec![9, 9], 1),
        (b"rSkp", vec![], 1),
        (b"sAfe", vec![0xff; 40], 1),
    ];
    for pick in 0..unknowns.len() {
        let set = vec![unknowns[pick].clone()];
        let bytes = unsafe {
            make_png(
                &mut rng,
                PNG_COLOR_TYPE_RGB,
                8,
                5,
                2,
                PNG_INTERLACE_NONE,
                &set,
            )
        };
        for keep in [
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
        ] {
            let mut outs = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    reset_logs();
                    let s = ReadSess::new(api, &bytes);
                    let tag = 0x9999_0000usize;
                    let mut nunknown = 0i32;
                    let mut uptr_ok = false;
                    let ok = guard(|| {
                        (api.png_set_read_user_chunk_fn)(
                            s.png,
                            tag as png_voidp,
                            Some(my_user_chunk),
                        );
                        uptr_ok = (api.png_get_user_chunk_ptr)(s.png) as usize == tag;
                        (api.png_set_keep_unknown_chunks)(s.png, keep, std::ptr::null(), 0);
                        (api.png_read_info)(s.png, s.info);
                        let rbz = (api.png_get_rowbytes)(s.png, s.info);
                        let h = (api.png_get_image_height)(s.png, s.info);
                        let mut buf: Vec<Vec<u8>> =
                            (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                        let mut ptrs: Vec<png_bytep> =
                            buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                        (api.png_read_image)(s.png, ptrs.as_mut_ptr());
                        (api.png_read_end)(s.png, s.end);
                        let mut list: png_unknown_chunkp = std::ptr::null_mut();
                        nunknown =
                            (api.png_get_unknown_chunks)(s.png, s.info, &mut list) as i32;
                    })
                    .is_some();
                    outs.push((
                        ok,
                        diag_take(),
                        nunknown,
                        uptr_ok,
                        CHUNK_LOG.with(|l| l.borrow().clone()),
                    ));
                }
            }
            assert_eq!(
                outs[0].0, outs[1].0,
                "user chunk parity pick={} keep={} (C {:?} RS {:?})",
                pick, keep, outs[0].1, outs[1].1
            );
            assert_eq!(outs[0].1, outs[1].1, "user chunk diag pick={}", pick);
            assert_eq!(outs[0].2, outs[1].2, "unknown chunk count pick={}", pick);
            assert_eq!(outs[0].3, outs[1].3, "user chunk ptr");
            assert_eq!(
                outs[0].4, outs[1].4,
                "user chunk callback invocations pick={} keep={}",
                pick, keep
            );
        }
    }
    for api in both() {
        unsafe {
            (api.png_set_read_user_chunk_fn)(std::ptr::null_mut(), std::ptr::null_mut(), None);
            assert!((api.png_get_user_chunk_ptr)(std::ptr::null()).is_null());
        }
    }
}

#[test]
fn io_state_tracking() {
    let mut rng = Rng::new(0x6666_7777_8888_9999);
    let bytes = unsafe {
        make_png(
            &mut rng,
            PNG_COLOR_TYPE_RGB,
            8,
            7,
            3,
            PNG_INTERLACE_NONE,
            &[],
        )
    };
    // Record (io_state, io_chunk_type) at every read callback invocation.
    thread_local! {
        static IO_LOG: RefCell<Vec<(u32, u32, usize)>> = const { RefCell::new(Vec::new()) };
    }
    unsafe extern "C-unwind" fn logging_read(png: png_structp, data: png_bytep, len: usize) {
        let api = current_api();
        let st = (api.png_get_io_state)(png);
        let ct = (api.png_get_io_chunk_type)(png);
        IO_LOG.with(|l| l.borrow_mut().push((st as u32, ct, len)));
        cb_read(png, data, len);
    }
    let mut outs = Vec::new();
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            IO_LOG.with(|l| l.borrow_mut().clear());
            let s = ReadSess::new(api, &bytes);
            (api.png_set_read_fn)(
                s.png,
                (api.png_get_io_ptr)(s.png),
                Some(logging_read),
            );
            let ok = guard(|| {
                (api.png_read_info)(s.png, s.info);
                let rbz = (api.png_get_rowbytes)(s.png, s.info);
                let h = (api.png_get_image_height)(s.png, s.info);
                let mut buf: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz + 8]).collect();
                let mut ptrs: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                (api.png_read_image)(s.png, ptrs.as_mut_ptr());
                (api.png_read_end)(s.png, s.end);
            })
            .is_some();
            outs.push((ok, diag_take(), IO_LOG.with(|l| l.borrow().clone())));
        }
    }
    assert_eq!(outs[0].0, outs[1].0, "io state parity");
    assert_eq!(outs[0].1, outs[1].1, "io state diag");
    assert_eq!(outs[0].2, outs[1].2, "io state sequence");
}
