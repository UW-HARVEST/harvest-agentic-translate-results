//! Phase B — the callback-driven entry points that the other files do not reach:
//! user row transforms (read and write), the user chunk callback, the
//! progressive reader's pause/skip API, `png_progressive_combine_row`, the
//! row-status callbacks and the memory callbacks.
//!
//! All callbacks are plain `extern "C"` functions shared by both libraries and
//! record what they were handed in a thread-local transcript, so the ARGUMENTS
//! libpng passes to the application are compared as well as the final output.
mod common;

use common::api::{apis, Api};
use common::harness::*;
use common::pngbuild as pb;
use common::*;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_uint, c_void};

/// `png_row_info` from png.h
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct png_row_info {
    pub width: png_uint_32,
    pub rowbytes: usize,
    pub color_type: png_byte,
    pub bit_depth: png_byte,
    pub channels: png_byte,
    pub pixel_depth: png_byte,
}

const DEPTH_TYPE: [(u8, u8); 15] = [
    (1, 0),
    (2, 0),
    (4, 0),
    (8, 0),
    (16, 0),
    (8, 2),
    (16, 2),
    (1, 3),
    (2, 3),
    (4, 3),
    (8, 3),
    (8, 4),
    (16, 4),
    (8, 6),
    (16, 6),
];

// ---------------------------------------------------------------------------
// callbacks
// ---------------------------------------------------------------------------

thread_local! {
    static XFORM_LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static CHUNK_RET: RefCell<c_int> = const { RefCell::new(0) };
    static PROG_ROWBYTES: RefCell<usize> = const { RefCell::new(0) };
    static ALLOCS: RefCell<usize> = const { RefCell::new(0) };
}

fn xlog(s: String) {
    XFORM_LOG.with(|l| l.borrow_mut().push(s));
}
fn xtake() -> Vec<String> {
    XFORM_LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

/// Read-side user transform: record the row_info and invert every byte.
unsafe extern "C" fn read_user_xform(_p: png_structp, ri: *mut c_void, row: png_bytep) {
    let r = &*(ri as *const png_row_info);
    xlog(format!(
        "RX:{}:{}:{}:{}:{}:{}",
        r.width, r.rowbytes, r.color_type, r.bit_depth, r.channels, r.pixel_depth
    ));
    if !row.is_null() {
        for i in 0..r.rowbytes {
            *row.add(i) = !*row.add(i);
        }
    }
}

/// Write-side user transform: record the row_info and rotate every byte.
unsafe extern "C" fn write_user_xform(_p: png_structp, ri: *mut c_void, row: png_bytep) {
    let r = &*(ri as *const png_row_info);
    xlog(format!(
        "WX:{}:{}:{}:{}:{}:{}",
        r.width, r.rowbytes, r.color_type, r.bit_depth, r.channels, r.pixel_depth
    ));
    if !row.is_null() {
        for i in 0..r.rowbytes {
            let v = *row.add(i);
            *row.add(i) = v.rotate_left(3);
        }
    }
}

/// User chunk callback: record the chunk and return the configured value.
unsafe extern "C" fn user_chunk_cb(_p: png_structp, chunk: *mut c_void) -> c_int {
    // png_unknown_chunkp
    let c = &*(chunk as *const png_unknown_chunk);
    let name: Vec<u8> = c.name[..4].to_vec();
    let data: Vec<u8> = if c.data.is_null() {
        Vec::new()
    } else {
        (0..c.size).map(|i| *c.data.add(i)).collect()
    };
    xlog(format!(
        "UC:{}:{}:{:02x?}",
        String::from_utf8_lossy(&name),
        c.size,
        &data[..data.len().min(16)]
    ));
    CHUNK_RET.with(|r| *r.borrow())
}

/// libpng's progressive reader does NOT deliver any rows unless the application
/// starts the image from the info callback (`png_start_read_image` or
/// `png_read_update_info`); without it the IDAT data is rejected with
/// "Truncated compressed data in IDAT" and the row callback never fires.  The
/// function pointer is resolved from whichever library is being driven.
type StartFn = unsafe extern "C" fn(png_structp);
thread_local! {
    static START: RefCell<Option<StartFn>> = const { RefCell::new(None) };
}

unsafe extern "C" fn prog_info_cb(p: png_structp, _i: png_infop) {
    xlog("PINFO".to_string());
    if let Some(f) = START.with(|c| *c.borrow()) {
        f(p);
    }
}
unsafe extern "C" fn prog_row_cb(
    _p: png_structp,
    row: png_bytep,
    row_num: png_uint_32,
    pass: c_int,
) {
    let n = PROG_ROWBYTES.with(|r| *r.borrow());
    if row.is_null() {
        xlog(format!("PROW:{row_num}:{pass}:NULL"));
    } else {
        let s: Vec<u8> = (0..n).map(|i| *row.add(i)).collect();
        xlog(format!("PROW:{row_num}:{pass}:{:02x?}", s));
    }
}
unsafe extern "C" fn prog_end_cb(_p: png_structp, _i: png_infop) {
    xlog("PEND".to_string());
}

/// A deterministic allocator pair so `png_create_*_struct_2` / `png_set_mem_fn`
/// are exercised through a real user allocator.  Layout bookkeeping is stored in
/// an 16-byte header so `free` can recover it.
unsafe extern "C" fn user_malloc(_p: png_structp, size: usize) -> *mut c_void {
    ALLOCS.with(|a| *a.borrow_mut() += 1);
    let total = size + 16;
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    let base = std::alloc::alloc(layout);
    if base.is_null() {
        return std::ptr::null_mut();
    }
    (base as *mut usize).write(total);
    base.add(16) as *mut c_void
}

unsafe extern "C" fn user_free(_p: png_structp, ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let base = (ptr as *mut u8).sub(16);
    let total = (base as *const usize).read();
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    std::alloc::dealloc(base, layout);
}

// ---------------------------------------------------------------------------
// read with a user transform
// ---------------------------------------------------------------------------

unsafe fn read_with_xform(
    a: &Api,
    is_c: bool,
    png: &[u8],
    depth: c_int,
    channels: c_int,
) -> (Vec<u8>, Vec<String>) {
    set_cur_is_c(is_c);
    reset_all();
    let _ = xtake();
    in_set(png);
    let mut p = (a.png_create_read_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    let mut info = (a.png_create_info_struct)(p);
    let mut end = (a.png_create_info_struct)(p);
    (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
    (a.png_read_info)(p, info);
    (a.png_set_read_user_transform_fn)(p, Some(read_user_xform));
    if depth > 0 {
        (a.png_set_user_transform_info)(p, std::ptr::null_mut(), depth, channels);
    }
    (a.png_read_update_info)(p, info);
    let h = (a.png_get_image_height)(p, info) as usize;
    let rb = (a.png_get_rowbytes)(p, info);
    let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rb]).collect();
    let mut ptrs: Vec<*mut png_byte> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
    (a.png_read_image)(p, ptrs.as_mut_ptr());
    (a.png_read_end)(p, end);
    let flat = rows.concat();
    let ptr_back = (a.png_get_user_transform_ptr)(p);
    let mut log = log_take();
    log.extend(xtake());
    log.push(format!("user_transform_ptr_null:{}", ptr_back.is_null()));
    (a.png_destroy_read_struct)(&mut p, &mut info, &mut end);
    (flat, log)
}

#[test]
fn read_user_transform() {
    let b = apis();
    let mut seed = 0x8_0000u64;
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for &il in &[0u8, 1] {
            seed += 1;
            let png = pb::make_png(seed, 17, 5, bd, ct, il);
            // once without png_set_user_transform_info, once with it (which is
            // what tells libpng the transform changes the pixel size)
            for &(d, c) in &[(0i32, 0i32), (bd as i32, pb::channels_of(ct) as i32)] {
                let (co, cl) = unsafe { read_with_xform(&b.c, true, &png, d, c) };
                let (ro, rl) = unsafe { read_with_xform(&b.rs, false, &png, d, c) };
                eq_bytes(&format!("read xform {bd}/{ct}/il{il} d{d}c{c}: rows"), &co, &ro);
                eq_dbg(&format!("read xform {bd}/{ct}/il{il} d{d}c{c}: log"), cl, rl);
                assert!(!co.is_empty());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// write with a user transform
// ---------------------------------------------------------------------------

unsafe fn write_with_xform(
    a: &Api,
    is_c: bool,
    seed: u64,
    bd: c_int,
    ct: c_int,
    il: c_int,
) -> (Vec<u8>, Vec<String>) {
    set_cur_is_c(is_c);
    reset_all();
    let _ = xtake();
    let mut p = (a.png_create_write_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    let mut info = (a.png_create_info_struct)(p);
    (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
    (a.png_set_IHDR)(p, info, 17, 5, bd, ct, il, 0, 0);
    let mut pal: Vec<png_color> = Vec::new();
    if ct == PNG_COLOR_TYPE_PALETTE {
        let mut r = Rng::new(seed);
        let n = (1usize << bd).min(256);
        pal = (0..n)
            .map(|_| png_color {
                red: r.next_u8(),
                green: r.next_u8(),
                blue: r.next_u8(),
            })
            .collect();
        (a.png_set_PLTE)(p, info, pal.as_ptr(), pal.len() as c_int);
    }
    (a.png_write_info)(p, info);
    (a.png_set_write_user_transform_fn)(p, Some(write_user_xform));
    let passes = if il == PNG_INTERLACE_ADAM7 {
        (a.png_set_interlace_handling)(p)
    } else {
        1
    };
    let rb = (a.png_get_rowbytes)(p, info);
    let mut r = Rng::new(seed ^ 0xabc);
    let rows: Vec<Vec<u8>> = (0..5)
        .map(|_| {
            let mut row: Vec<u8> = (0..rb).map(|_| r.next_u8()).collect();
            if ct == PNG_COLOR_TYPE_PALETTE && bd == 8 && !pal.is_empty() && pal.len() < 256 {
                for x in row.iter_mut() {
                    *x %= pal.len() as u8;
                }
            }
            row
        })
        .collect();
    for _ in 0..passes {
        for row in &rows {
            // png_write_row may modify the caller's buffer via the transform, so
            // hand it a fresh copy each time to keep both runs identical
            let mut copy = row.clone();
            (a.png_write_row)(p, copy.as_mut_ptr());
        }
    }
    (a.png_write_end)(p, info);
    let mut log = log_take();
    log.extend(xtake());
    (a.png_destroy_write_struct)(&mut p, &mut info);
    (out_take(), log)
}

#[test]
fn write_user_transform() {
    let b = apis();
    let mut seed = 0x8_4000u64;
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for &il in &[0i32, 1] {
            seed += 1;
            let (co, cl) =
                unsafe { write_with_xform(&b.c, true, seed, bd as c_int, ct as c_int, il) };
            let (ro, rl) =
                unsafe { write_with_xform(&b.rs, false, seed, bd as c_int, ct as c_int, il) };
            eq_bytes(&format!("write xform {bd}/{ct}/il{il}: stream"), &co, &ro);
            eq_dbg(&format!("write xform {bd}/{ct}/il{il}: log"), cl, rl);
            assert!(!co.is_empty());
        }
    }
}

// ---------------------------------------------------------------------------
// user chunk callback
// ---------------------------------------------------------------------------

unsafe fn read_with_user_chunk(
    a: &Api,
    is_c: bool,
    png: &[u8],
    ret: c_int,
) -> Vec<String> {
    set_cur_is_c(is_c);
    reset_all();
    let _ = xtake();
    CHUNK_RET.with(|r| *r.borrow_mut() = ret);
    in_set(png);
    let mut p = (a.png_create_read_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    let mut info = (a.png_create_info_struct)(p);
    let mut end = (a.png_create_info_struct)(p);
    (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
    (a.png_set_read_user_chunk_fn)(p, 0x1234usize as *mut c_void, Some(user_chunk_cb));
    (a.png_read_info)(p, info);
    let h = (a.png_get_image_height)(p, info) as usize;
    let rb = (a.png_get_rowbytes)(p, info);
    let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rb]).collect();
    let mut ptrs: Vec<*mut png_byte> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
    (a.png_read_image)(p, ptrs.as_mut_ptr());
    (a.png_read_end)(p, end);
    let ucp = (a.png_get_user_chunk_ptr)(p);
    let mut u: *mut png_unknown_chunk = std::ptr::null_mut();
    let n = (a.png_get_unknown_chunks)(p, info, &mut u);
    let mut log = log_take();
    log.extend(xtake());
    log.push(format!("user_chunk_ptr:{}", ucp as usize));
    log.push(format!("stored_unknown:{n}"));
    if n > 0 && !u.is_null() {
        for i in 0..n as usize {
            let c = *u.add(i);
            log.push(format!(
                "stored[{i}]:{}:{}:{}",
                String::from_utf8_lossy(&c.name[..4]),
                c.size,
                c.location
            ));
        }
    }
    (a.png_destroy_read_struct)(&mut p, &mut info, &mut end);
    log
}


unsafe fn read_with_user_chunk_keep(
    a: &Api,
    is_c: bool,
    png: &[u8],
    ret: c_int,
    keep: c_int,
) -> Vec<String> {
    set_cur_is_c(is_c);
    reset_all();
    let _ = xtake();
    CHUNK_RET.with(|r| *r.borrow_mut() = ret);
    in_set(png);
    let mut p = (a.png_create_read_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    let mut info = (a.png_create_info_struct)(p);
    let mut end = (a.png_create_info_struct)(p);
    (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
    (a.png_set_keep_unknown_chunks)(p, keep, std::ptr::null(), 0);
    (a.png_set_read_user_chunk_fn)(p, 0x1234usize as *mut c_void, Some(user_chunk_cb));
    (a.png_read_info)(p, info);
    let h = (a.png_get_image_height)(p, info) as usize;
    let rbz = (a.png_get_rowbytes)(p, info);
    let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz]).collect();
    let mut ptrs: Vec<*mut png_byte> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
    (a.png_read_image)(p, ptrs.as_mut_ptr());
    (a.png_read_end)(p, end);
    let mut u: *mut png_unknown_chunk = std::ptr::null_mut();
    let n = (a.png_get_unknown_chunks)(p, info, &mut u);
    let mut log = log_take();
    log.extend(xtake());
    log.push(format!("stored_unknown:{n}"));
    (a.png_destroy_read_struct)(&mut p, &mut info, &mut end);
    log
}

#[test]
fn user_chunk_callback() {
    let b = apis();
    let mut mk = |chunks: Vec<([u8; 4], Vec<u8>)>, post: Vec<([u8; 4], Vec<u8>)>| {
        let mut spec = pb::PngSpec::new(6, 3, 8, 2, 0);
        spec.pre_idat = chunks;
        spec.post_idat = post;
        let mut r = Rng::new(5);
        spec.raw = pb::raw_rows_none(6, 3, 8, 2, &mut |_y, rb| {
            (0..rb).map(|_| r.next_u8()).collect()
        });
        spec.build()
    };

    let sets: Vec<(&str, Vec<([u8; 4], Vec<u8>)>)> = vec![
        ("one", vec![(*b"prVt", vec![1, 2, 3, 4])]),
        ("empty", vec![(*b"prVt", vec![])]),
        (
            "many",
            vec![
                (*b"prVt", vec![1]),
                (*b"orNg", vec![2, 3]),
                (*b"blUe", vec![4, 5, 6]),
            ],
        ),
        ("known-gAMA", vec![(*b"gAMA", 45455u32.to_be_bytes().to_vec())]),
        ("large", vec![(*b"prVt", vec![0x5a; 500])]),
    ];

    // The callback's return value selects: <0 -> png_chunk_error
    // "<name>: error in user chunk" (FATAL), 0 -> "did not handle" which then
    // reaches png_error "forcing save of an unhandled chunk; please call
    // png_set_keep_unknown_chunks" unless a keep policy was set (also FATAL),
    // and >0 -> handled.  Only the handled case is a valid path; the other two
    // are covered by `user_chunk_rejections` below.
    for ret in [1i32, 2, 100] {
        for (name, chunks) in &sets {
            for post in [false, true] {
                let png = if post {
                    mk(Vec::new(), chunks.clone())
                } else {
                    mk(chunks.clone(), Vec::new())
                };
                // a negative return makes libpng report an error; on the read
                // side that is a benign chunk error (a warning), so it stays on
                // the valid path here.
                let cl = unsafe { read_with_user_chunk(&b.c, true, &png, ret) };
                let rl = unsafe { read_with_user_chunk(&b.rs, false, &png, ret) };
                eq_dbg(&format!("user chunk {name} ret={ret} post={post}"), cl, rl);
                // ret == 0 ("not handled") is only valid when a keep policy
                // tells libpng what to do with the chunk; cover that too.
                // pngrutil.c:2827: with ret == 0 a keep value BELOW
                // PNG_HANDLE_CHUNK_IF_SAFE reaches the fatal
                // "forcing save of an unhandled chunk" app-warning, so only
                // IF_SAFE and ALWAYS are valid here.
                for keep in [PNG_HANDLE_CHUNK_IF_SAFE, PNG_HANDLE_CHUNK_ALWAYS] {
                    let cl = unsafe { read_with_user_chunk_keep(&b.c, true, &png, 0, keep) };
                    let rl = unsafe { read_with_user_chunk_keep(&b.rs, false, &png, 0, keep) };
                    eq_dbg(
                        &format!("user chunk {name} ret=0 keep={keep} post={post}"),
                        cl,
                        rl,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// progressive pause / skip
// ---------------------------------------------------------------------------

unsafe fn progressive_pause(
    a: &Api,
    is_c: bool,
    png: &[u8],
    rowbytes: usize,
    chunk: usize,
    save: c_int,
    pause_every: usize,
) -> Vec<String> {
    set_cur_is_c(is_c);
    reset_all();
    let _ = xtake();
    PROG_ROWBYTES.with(|r| *r.borrow_mut() = rowbytes);
    START.with(|c| {
        *c.borrow_mut() = Some(sym::<StartFn>(
            if is_c { &libs().c } else { &libs().rs },
            "png_start_read_image",
        ))
    });
    let mut p = (a.png_create_read_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    let mut info = (a.png_create_info_struct)(p);
    let mut end = (a.png_create_info_struct)(p);
    (a.png_set_progressive_read_fn)(
        p,
        0x4321usize as *mut c_void,
        Some(prog_info_cb),
        Some(prog_row_cb),
        Some(prog_end_cb),
    );
    let mut i = 0usize;
    let mut n_feeds = 0usize;
    while i < png.len() {
        let n = chunk.max(1).min(png.len() - i);
        (a.png_process_data)(p, info, png[i..].as_ptr() as *mut png_byte, n);
        i += n;
        n_feeds += 1;
        if pause_every != 0 && n_feeds % pause_every == 0 {
            let left = (a.png_process_data_pause)(p, save);
            xlog(format!("PAUSE:{left}"));
            // NOTE: `png_process_data_skip` is NOT called here -- it raises
            // png_app_error "png_process_data_skip is not implemented in any
            // current version of libpng", which is fatal in this build.  That
            // rejection is covered by `process_data_skip_is_rejected` below.
        }
    }
    let pp = (a.png_get_progressive_ptr)(p);
    let mut log = log_take();
    log.extend(xtake());
    log.push(format!("progressive_ptr:{}", pp as usize));
    (a.png_destroy_read_struct)(&mut p, &mut info, &mut end);
    log
}

#[test]
fn progressive_pause_and_skip() {
    let b = apis();
    let mut seed = 0x8_8000u64;
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for &il in &[0u8, 1] {
            let png = pb::make_png(seed, 13, 5, bd, ct, il);
            seed += 1;
            let rb = pb::rowbytes(bd, ct, 13);
            for chunk in [1usize, 3, 17, 1000] {
                for save in [0i32, 1] {
                    for pause_every in [0usize, 1, 2, 5] {
                        let cl = unsafe {
                            progressive_pause(&b.c, true, &png, rb, chunk, save, pause_every)
                        };
                        let rl = unsafe {
                            progressive_pause(&b.rs, false, &png, rb, chunk, save, pause_every)
                        };
                        eq_dbg(
                            &format!(
                                "prog pause {bd}/{ct}/il{il} chunk={chunk} save={save} every={pause_every}"
                            ),
                            cl,
                            rl,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// png_progressive_combine_row
// ---------------------------------------------------------------------------

// `png_progressive_combine_row` is only valid FROM the row callback (libpng's
// documented usage: the application keeps its own row buffer and asks libpng to
// merge the sparse interlaced row into it).  Calling it after the read has
// finished touches a freed `row_buf`, so the call is made from inside the
// callback here, with the destination buffer and the resolved function pointer
// held in thread-locals.
type CombineFn = unsafe extern "C" fn(png_structp, png_bytep, *const png_byte);
thread_local! {
    static COMBINE: RefCell<Option<CombineFn>> = const { RefCell::new(None) };
    static DEST: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
}

unsafe extern "C" fn prog_row_combine_cb(
    p: png_structp,
    row: png_bytep,
    row_num: png_uint_32,
    pass: c_int,
) {
    let f = COMBINE.with(|c| *c.borrow());
    if let Some(f) = f {
        DEST.with(|d| {
            let mut d = d.borrow_mut();
            let y = row_num as usize;
            if y < d.len() {
                let dst = d[y].as_mut_ptr();
                // NULL new_row is a documented no-op; exercise it too
                f(p, dst, std::ptr::null());
                f(p, dst, row as *const png_byte);
            }
        });
    }
    xlog(format!("PROWC:{row_num}:{pass}"));
}

#[test]
fn progressive_combine_row() {
    let b = apis();
    let l = libs();
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for &il in &[0u8, 1] {
            for w in [1u32, 3, 8, 9, 17] {
                let h = 6u32;
                let png = pb::make_png(0x8_c000 + w as u64 + bd as u64 * 41, w, h, bd, ct, il);
                let rb = pb::rowbytes(bd, ct, w);
                let run = |a: &Api, is_c: bool, lib: &'static libloading::Library| unsafe {
                    set_cur_is_c(is_c);
                    reset_all();
                    let _ = xtake();
                    PROG_ROWBYTES.with(|r| *r.borrow_mut() = rb);
                    START.with(|c| {
                        *c.borrow_mut() =
                            Some(sym::<StartFn>(lib, "png_start_read_image"))
                    });
                    COMBINE.with(|c| {
                        *c.borrow_mut() =
                            Some(sym::<CombineFn>(lib, "png_progressive_combine_row"))
                    });
                    // start from a fixed, identical pattern in both libraries so
                    // the bits png_combine_row deliberately PRESERVES outside the
                    // image are the same on both sides
                    DEST.with(|d| {
                        *d.borrow_mut() = (0..h as usize)
                            .map(|y| vec![(0x40 + y) as u8; rb])
                            .collect()
                    });
                    let mut p = (a.png_create_read_struct)(
                        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                        std::ptr::null_mut(),
                        Some(error_cb),
                        Some(warn_cb),
                    );
                    let mut info = (a.png_create_info_struct)(p);
                    let mut end = (a.png_create_info_struct)(p);
                    (a.png_set_progressive_read_fn)(
                        p,
                        std::ptr::null_mut(),
                        Some(prog_info_cb),
                        Some(prog_row_combine_cb),
                        Some(prog_end_cb),
                    );
                    (a.png_process_data)(p, info, png.as_ptr() as *mut png_byte, png.len());
                    let mut log = log_take();
                    log.extend(xtake());
                    COMBINE.with(|c| *c.borrow_mut() = None);
                    let dest = DEST.with(|d| d.borrow().concat());
                    (a.png_destroy_read_struct)(&mut p, &mut info, &mut end);
                    (dest, log)
                };
                let (cd, cl) = run(&b.c, true, &l.c);
                let (rd, rl) = run(&b.rs, false, &l.rs);
                eq_bytes(
                    &format!("combine_row dest {bd}/{ct}/il{il} w{w}"),
                    &cd,
                    &rd,
                );
                eq_dbg(&format!("combine_row log {bd}/{ct}/il{il} w{w}"), cl, rl);
                assert!(!cd.is_empty());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// memory callbacks
// ---------------------------------------------------------------------------

#[test]
fn memory_callbacks() {
    let b = apis();
    // png_create_read_struct_2 / png_create_write_struct_2 and png_set_mem_fn
    let run_read = |a: &Api, is_c: bool, png: &[u8]| unsafe {
        set_cur_is_c(is_c);
        reset_all();
        ALLOCS.with(|x| *x.borrow_mut() = 0);
        in_set(png);
        let mut p = (a.png_create_read_struct_2)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(error_cb),
            Some(warn_cb),
            0x9999usize as *mut c_void,
            Some(user_malloc),
            Some(user_free),
        );
        assert!(!p.is_null());
        let mut info = (a.png_create_info_struct)(p);
        let mut end = (a.png_create_info_struct)(p);
        (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
        (a.png_read_info)(p, info);
        let mem_ptr = (a.png_get_mem_ptr)(p);
        let h = (a.png_get_image_height)(p, info) as usize;
        let rbz = (a.png_get_rowbytes)(p, info);
        let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rbz]).collect();
        let mut ptrs: Vec<*mut png_byte> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
        (a.png_read_image)(p, ptrs.as_mut_ptr());
        (a.png_read_end)(p, end);
        let flat = rows.concat();
        let mut log = log_take();
        log.push(format!("mem_ptr:{}", mem_ptr as usize));
        (a.png_destroy_read_struct)(&mut p, &mut info, &mut end);
        // the user allocator must have been used at least once
        let n = ALLOCS.with(|x| *x.borrow());
        assert!(n > 0, "the user malloc callback was never called");
        (flat, log)
    };
    let png = pb::make_png(0x9_0000, 21, 7, 8, 6, 0);
    let (co, cl) = unsafe { run_read(&b.c, true, &png) };
    let (ro, rl) = unsafe { run_read(&b.rs, false, &png) };
    eq_bytes("create_read_struct_2 rows", &co, &ro);
    eq_dbg("create_read_struct_2 log", cl, rl);

    let run_write = |a: &Api, is_c: bool| unsafe {
        set_cur_is_c(is_c);
        reset_all();
        ALLOCS.with(|x| *x.borrow_mut() = 0);
        let mut p = (a.png_create_write_struct_2)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(error_cb),
            Some(warn_cb),
            0x8888usize as *mut c_void,
            Some(user_malloc),
            Some(user_free),
        );
        assert!(!p.is_null());
        let mut info = (a.png_create_info_struct)(p);
        (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
        (a.png_set_IHDR)(p, info, 21, 7, 8, 6, 0, 0, 0);
        (a.png_write_info)(p, info);
        let rbz = (a.png_get_rowbytes)(p, info);
        let mut r = Rng::new(0x9_1000);
        for _ in 0..7 {
            let row: Vec<u8> = (0..rbz).map(|_| r.next_u8()).collect();
            (a.png_write_row)(p, row.as_ptr());
        }
        (a.png_write_end)(p, info);
        let mem_ptr = (a.png_get_mem_ptr)(p);
        let mut log = log_take();
        log.push(format!("mem_ptr:{}", mem_ptr as usize));
        (a.png_destroy_write_struct)(&mut p, &mut info);
        let n = ALLOCS.with(|x| *x.borrow());
        assert!(n > 0, "the user malloc callback was never called on write");
        (out_take(), log)
    };
    let (co, cl) = unsafe { run_write(&b.c, true) };
    let (ro, rl) = unsafe { run_write(&b.rs, false) };
    eq_bytes("create_write_struct_2 stream", &co, &ro);
    eq_dbg("create_write_struct_2 log", cl, rl);

    // png_set_mem_fn on an existing struct, then png_malloc / png_calloc /
    // png_malloc_warn / png_free through it
    let run_alloc = |a: &Api, is_c: bool| unsafe {
        set_cur_is_c(is_c);
        reset_all();
        ALLOCS.with(|x| *x.borrow_mut() = 0);
        // NOTE: the struct MUST be created with `png_create_read_struct_2` so
        // that every internal allocation goes through the same allocator.
        // Calling `png_set_mem_fn` on a struct created with the DEFAULT
        // allocator makes `png_destroy_read_struct` hand pointers obtained from
        // `malloc` to the user `free`, which is a genuine heap error in the
        // *test*, not a divergence.
        let mut p = (a.png_create_read_struct_2)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(error_cb),
            Some(warn_cb),
            0x7777usize as *mut c_void,
            Some(user_malloc),
            Some(user_free),
        );
        // re-installing the same pair is a no-op and exercises png_set_mem_fn
        (a.png_set_mem_fn)(p, 0x7777usize as *mut c_void, Some(user_malloc), Some(user_free));
        let mut log = vec![format!("mem_ptr:{}", (a.png_get_mem_ptr)(p) as usize)];
        for n in [1usize, 7, 64, 4096] {
            let q = (a.png_malloc)(p, n);
            log.push(format!("malloc({n}):{}", !q.is_null()));
            if !q.is_null() {
                (a.png_free)(p, q);
            }
            let q = (a.png_calloc)(p, n);
            log.push(format!("calloc({n}):{}", !q.is_null()));
            if !q.is_null() {
                // calloc must have zeroed it
                let all0 = (0..n).all(|i| *(q as *const u8).add(i) == 0);
                log.push(format!("calloc({n}).zeroed:{all0}"));
                (a.png_free)(p, q);
            }
            let q = (a.png_malloc_warn)(p, n);
            log.push(format!("malloc_warn({n}):{}", !q.is_null()));
            if !q.is_null() {
                (a.png_free)(p, q);
            }
        }
        log.extend(log_take());
        let mut info: png_infop = std::ptr::null_mut();
        let mut end: png_infop = std::ptr::null_mut();
        (a.png_destroy_read_struct)(&mut p, &mut info, &mut end);
        log
    };
    let cl = unsafe { run_alloc(&b.c, true) };
    let rl = unsafe { run_alloc(&b.rs, false) };
    eq_dbg("png_set_mem_fn allocations", cl, rl);
}

// ---------------------------------------------------------------------------
// png_set_rows / png_get_rows / png_free_data / png_data_freer
// ---------------------------------------------------------------------------

#[test]
fn rows_and_free_data() {
    let b = apis();
    for mask in [
        0u32,
        0x0001, // PNG_FREE_HIST
        0x0002, // PNG_FREE_ICCP
        0x0004, // PNG_FREE_SPLT
        0x0008, // PNG_FREE_ROWS
        0x0010, // PNG_FREE_PCAL
        0x0020, // PNG_FREE_SCAL
        0x0040, // PNG_FREE_UNKN
        0x0100, // PNG_FREE_PLTE
        0x0200, // PNG_FREE_TRNS
        0x0400, // PNG_FREE_TEXT
        0x0800, // PNG_FREE_EXIF
        0x7fff_ffff,
        0xffff_ffff,
    ] {
        // `num != -1` indexes info_ptr->text[num] / sPLT[num] / unknown[num]
        // with NO bounds check (png.c:498), so only -1 and index 0 (which does
        // exist here: one text chunk, one sPLT-less info) are valid inputs.
        // An out-of-range `num` is C-level UB, not a rejection.
        for num in [-1i32, 0] {
            // png.h: PNG_DESTROY_WILL_FREE_DATA == PNG_SET_WILL_FREE_DATA == 1
            // and PNG_USER_WILL_FREE_DATA == 2, so ONLY 1 and 2 are valid.  Any
            // other value raises png_error "Unknown freer parameter in
            // png_data_freer", covered by `data_freer_rejects_bad_freer` below.
            for freer in [1i32 /* DESTROY/SET_WILL_FREE_DATA */,
                          2 /* PNG_USER_WILL_FREE_DATA */] {
                let run = |a: &Api, is_c: bool| unsafe {
                    set_cur_is_c(is_c);
                    reset_all();
                    in_set(&pb::make_png(0x9_2000, 5, 3, 8, 3, 0));
                    let mut p = (a.png_create_read_struct)(
                        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                        std::ptr::null_mut(),
                        Some(error_cb),
                        Some(warn_cb),
                    );
                    let mut info = (a.png_create_info_struct)(p);
                    let mut end = (a.png_create_info_struct)(p);
                    (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
                    (a.png_read_info)(p, info);
                    // populate several freeable items
                    let mut key = b"Title\0".to_vec();
                    let mut val = b"v\0".to_vec();
                    let t = png_text {
                        compression: PNG_TEXT_COMPRESSION_NONE,
                        key: key.as_mut_ptr() as *mut c_char,
                        text: val.as_mut_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: std::ptr::null_mut(),
                        lang_key: std::ptr::null_mut(),
                    };
                    (a.png_set_text)(p, info, &t, 1);
                    let hist = vec![1u16; 256];
                    (a.png_set_hIST)(p, info, hist.as_ptr());
                    (a.png_set_sCAL_s)(p, info, 1, c"1.0".as_ptr(), c"2.0".as_ptr());
                    (a.png_data_freer)(p, info, freer, mask);
                    (a.png_free_data)(p, info, mask, num);
                    let mut log = log_take();
                    // and re-query so the effect is observable
                    let mut tp: *mut png_text = std::ptr::null_mut();
                    let mut nt = 0i32;
                    log.push(format!(
                        "text:{}:{nt}",
                        (a.png_get_text)(p, info, &mut tp, &mut nt)
                    ));
                    let mut hp: *mut png_uint_16 = std::ptr::null_mut();
                    log.push(format!("hist:{}", (a.png_get_hIST)(p, info, &mut hp)));
                    log.push(format!(
                        "valid.sCAL:{}",
                        (a.png_get_valid)(p, info, PNG_INFO_sCAL)
                    ));
                    log.push(format!(
                        "valid.PLTE:{}",
                        (a.png_get_valid)(p, info, PNG_INFO_PLTE)
                    ));
                    let rows = (a.png_get_rows)(p, info);
                    log.push(format!("rows_null:{}", rows.is_null()));
                    // restore ownership so destroy does not double-free
                    (a.png_data_freer)(p, info, 1, 0xffff_ffff);
                    (a.png_destroy_read_struct)(&mut p, &mut info, &mut end);
                    log
                };
                if std::env::var_os("PNG_TRACE").is_some() {
                    eprintln!("CASE free_data mask={mask:#x} num={num} freer={freer}");
                }
                let cl = unsafe { run(&b.c, true) };
                let rl = unsafe { run(&b.rs, false) };
                eq_dbg(
                    &format!("free_data mask={mask:#x} num={num} freer={freer}"),
                    cl,
                    rl,
                );
            }
        }
    }

    // png_set_rows + png_get_rows round trip
    let run = |a: &Api, is_c: bool| unsafe {
        set_cur_is_c(is_c);
        reset_all();
        let mut p = (a.png_create_write_struct)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(error_cb),
            Some(warn_cb),
        );
        let mut info = (a.png_create_info_struct)(p);
        (a.png_set_IHDR)(p, info, 4, 3, 8, 2, 0, 0, 0);
        let mut store: Vec<Vec<u8>> = (0..3).map(|y| vec![y as u8; 12]).collect();
        let mut ptrs: Vec<*mut png_byte> = store.iter_mut().map(|r| r.as_mut_ptr()).collect();
        (a.png_set_rows)(p, info, ptrs.as_mut_ptr());
        let got = (a.png_get_rows)(p, info);
        let same = got == ptrs.as_mut_ptr();
        let mut log = vec![format!("rows_roundtrip:{same}")];
        // setting NULL rows again, and then reading back
        (a.png_set_rows)(p, info, std::ptr::null_mut());
        log.push(format!("rows_null_after:{}", (a.png_get_rows)(p, info).is_null()));
        log.extend(log_take());
        (a.png_destroy_write_struct)(&mut p, &mut info);
        log
    };
    let cl = unsafe { run(&b.c, true) };
    let rl = unsafe { run(&b.rs, false) };
    eq_dbg("png_set_rows/png_get_rows", cl, rl);
}

/// Self-check: prove the callbacks were actually invoked and their arguments
/// recorded, so the comparisons above are not vacuous.
#[test]
fn self_check_callbacks_fire() {
    let b = apis();

    // --- progressive callbacks (checked first, on a fresh struct) -----------
    let rb = pb::rowbytes(8, 2, 13);
    let png = pb::make_png(0x8_8001, 13, 5, 8, 2, 0);
    let log = unsafe { progressive_pause(&b.c, true, &png, rb, 3, 0, 0) };
    assert!(log.iter().any(|l| l == "PINFO"), "progressive info callback: {log:?}");
    let nrows = log.iter().filter(|l| l.starts_with("PROW:")).count();
    assert_eq!(
        nrows, 5,
        "progressive row callback must fire once per row, got {nrows}, log={log:?}"
    );
    assert!(log.iter().any(|l| l == "PEND"), "progressive end callback: {log:?}");

    // --- read / write user transforms ---------------------------------------
    let png = pb::make_png(1, 9, 4, 8, 2, 0);
    let (_, log) = unsafe { read_with_xform(&b.c, true, &png, 0, 0) };
    let rx = log.iter().filter(|l| l.starts_with("RX:")).count();
    assert_eq!(rx, 4, "the read user transform must run once per row, got {rx}");
    let (_, log) = unsafe { write_with_xform(&b.c, true, 1, 8, 2, 0) };
    let wx = log.iter().filter(|l| l.starts_with("WX:")).count();
    assert_eq!(wx, 5, "the write user transform must run once per row, got {wx}");

    // --- user chunk callback ------------------------------------------------
    let mut spec = pb::PngSpec::new(6, 3, 8, 2, 0);
    spec.pre_idat = vec![(*b"prVt", vec![9, 9, 9])];
    let mut r = Rng::new(5);
    spec.raw = pb::raw_rows_none(6, 3, 8, 2, &mut |_y, rb| {
        (0..rb).map(|_| r.next_u8()).collect()
    });
    let png = spec.build();
    let log = unsafe { read_with_user_chunk(&b.c, true, &png, 1) };
    assert!(
        log.iter().any(|l| l.starts_with("UC:prVt")),
        "the user chunk callback must be invoked, log={log:?}"
    );
}

// ---------------------------------------------------------------------------
// Phase C rows that belong to this file's entry points.  They are fatal
// `png_error`s, so they run in a SUB-PROCESS (see tests/t23_err_write.rs for
// the mechanism).
// ---------------------------------------------------------------------------

#[test]
fn harness_child() {
    let Some((case, which)) = child_case() else {
        return;
    };
    set_child_mode(true);
    let b = apis();
    let a = if which == "c" { &b.c } else { &b.rs };
    set_cur_is_c(which == "c");
    reset_all();
    unsafe {
        match case.as_str() {
            // png_process_data_skip is unconditionally rejected
            "skip-before-any-data" => {
                let p = (a.png_create_read_struct)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                let r = (a.png_process_data_skip)(p);
                emit(format!("skip:{r}"));
            }
            "skip-after-header" => {
                let png = pb::make_png(1, 8, 4, 8, 2, 0);
                let p = (a.png_create_read_struct)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                let info = (a.png_create_info_struct)(p);
                PROG_ROWBYTES.with(|r| *r.borrow_mut() = pb::rowbytes(8, 2, 8));
                START.with(|c| {
                    *c.borrow_mut() = Some(sym::<StartFn>(
                        if which == "c" { &libs().c } else { &libs().rs },
                        "png_start_read_image",
                    ))
                });
                (a.png_set_progressive_read_fn)(
                    p,
                    std::ptr::null_mut(),
                    Some(prog_info_cb),
                    Some(prog_row_cb),
                    Some(prog_end_cb),
                );
                (a.png_process_data)(p, info, png.as_ptr() as *mut png_byte, 40);
                let left = (a.png_process_data_pause)(p, 1);
                emit(format!("pause:{left}"));
                let r = (a.png_process_data_skip)(p);
                emit(format!("skip:{r}"));
            }
            // png_data_freer with an unknown freer value
            _ if case.starts_with("freer:") => {
                let v: c_int = case[6..].parse().unwrap();
                let p = (a.png_create_read_struct)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                let info = (a.png_create_info_struct)(p);
                (a.png_data_freer)(p, info, v, 0xffff_ffff);
                emit("data_freer returned");
            }
            // user chunk callback return value / keep policy rejections
            _ if case.starts_with("uchunk:") => {
                let f: Vec<&str> = case[7..].split(',').collect();
                let ret: c_int = f[0].parse().unwrap();
                let keep: c_int = f[1].parse().unwrap();
                CHUNK_RET.with(|r| *r.borrow_mut() = ret);
                let mut spec = pb::PngSpec::new(6, 3, 8, 2, 0);
                spec.pre_idat = vec![(*b"prVt", vec![1, 2, 3])];
                let mut r = Rng::new(5);
                spec.raw = pb::raw_rows_none(6, 3, 8, 2, &mut |_y, rb| {
                    (0..rb).map(|_| r.next_u8()).collect()
                });
                let png = spec.build();
                in_set(&png);
                let p = (a.png_create_read_struct)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                let info = (a.png_create_info_struct)(p);
                (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
                if keep >= 0 {
                    (a.png_set_keep_unknown_chunks)(p, keep, std::ptr::null(), 0);
                }
                (a.png_set_read_user_chunk_fn)(p, 0x1234usize as *mut c_void, Some(user_chunk_cb));
                (a.png_read_info)(p, info);
                emit("read_info returned");
            }
            other => {
                emit(format!("UNKNOWN CASE {other}"));
                std::process::exit(3);
            }
        }
    }
    for l in xtake() {
        emit(l);
    }
    child_finish();
}

#[test]
fn process_data_skip_is_rejected() {
    diff_case("skip-before-any-data");
    diff_case("skip-after-header");
    let t = run_child("skip-before-any-data", "c");
    assert!(
        t.lines.iter().any(|l| l
            == "ERROR:png_process_data_skip is not implemented in any current version of libpng"),
        "expected the documented rejection, got {:?}",
        t.lines
    );
}

#[test]
fn data_freer_rejects_bad_freer() {
    // png.h: PNG_DESTROY_WILL_FREE_DATA == PNG_SET_WILL_FREE_DATA == 1 and
    // PNG_USER_WILL_FREE_DATA == 2, so only 1 and 2 are accepted.
    for v in [-1i32, 0, 3, 4, 5, 255, i32::MAX, i32::MIN] {
        diff_case(&format!("freer:{v}"));
    }
    for v in [1i32, 2] {
        let t = run_child(&format!("freer:{v}"), "c");
        assert_eq!(t.exit, Some(0), "freer={v} must be accepted, got {t:?}");
    }
    let t = run_child("freer:0", "c");
    assert!(
        t.lines
            .iter()
            .any(|l| l == "ERROR:Unknown freer parameter in png_data_freer"),
        "expected the documented rejection, got {:?}",
        t.lines
    );
}

#[test]
fn user_chunk_rejections() {
    // ret < 0  -> png_chunk_error "<name>: error in user chunk"
    // ret == 0 with keep < PNG_HANDLE_CHUNK_IF_SAFE (pngrutil.c:2827) ->
    //   png_app_warning "forcing save of an unhandled chunk; please call
    //   png_set_keep_unknown_chunks", fatal in this build
    for ret in [-1i32, -2, i32::MIN] {
        for keep in [-1i32, PNG_HANDLE_CHUNK_ALWAYS, PNG_HANDLE_CHUNK_NEVER] {
            diff_case(&format!("uchunk:{ret},{keep}"));
        }
    }
    for keep in [-1i32, PNG_HANDLE_CHUNK_AS_DEFAULT, PNG_HANDLE_CHUNK_NEVER] {
        diff_case(&format!("uchunk:0,{keep}"));
    }
    let t = run_child("uchunk:-1,-1", "c");
    assert!(
        t.lines.iter().any(|l| l.contains("error in user chunk")),
        "expected the user-chunk error, got {:?}",
        t.lines
    );
    let t = run_child("uchunk:0,-1", "c");
    assert!(
        t.lines.iter().any(|l| l
            == "ERROR:forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks"),
        "expected the forcing-save error, got {:?}",
        t.lines
    );
}
