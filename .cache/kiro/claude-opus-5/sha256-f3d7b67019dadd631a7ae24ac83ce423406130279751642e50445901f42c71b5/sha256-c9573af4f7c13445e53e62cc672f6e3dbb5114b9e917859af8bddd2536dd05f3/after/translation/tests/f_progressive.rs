//! Phase B, group P: the progressive ("push") reader — `png_process_data`,
//! `png_process_data_pause`, `png_process_data_skip` and
//! `png_progressive_combine_row`.  Feed granularity is varied because the C
//! state machine buffers differently for every chunk boundary.
mod common;
use common::*;
use std::cell::RefCell;
use std::ffi::{c_int, c_void};
use std::ptr;

const SEED: u64 = 0x9909_7777_3333_1111;

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

#[derive(Default)]
struct PState {
    rowbytes: usize,
    height: u32,
    rows: Vec<Vec<u8>>,
    /// transforms to apply inside the info callback
    transforms: u32,
    /// 0 = never pause, 1 = pause once in the info callback, 2 = pause once in
    /// the row callback
    pause_where: u32,
    pause_save: c_int,
    paused_remaining: usize,
}

thread_local! {
    static PS: RefCell<PState> = RefCell::new(PState::default());
}

// Which library the callbacks belong to is irrelevant: they only touch PS and
// the Ctx log.  The png_ptr passed back in is used for the libpng calls.
thread_local! {
    static CUR_LIB: RefCell<*const Lib> = const { RefCell::new(ptr::null()) };
}

fn cur_lib() -> &'static Lib {
    CUR_LIB.with(|c| unsafe { &*(*c.borrow()) })
}

unsafe extern "C" fn info_cb(png: *mut c_void, info: *mut c_void) {
    let l = cur_lib();
    log(format!(
        "info_cb: {}x{} bd={} ct={} il={} rb={}",
        (l.api.png_get_image_width)(png, info),
        (l.api.png_get_image_height)(png, info),
        (l.api.png_get_bit_depth)(png, info),
        (l.api.png_get_color_type)(png, info),
        (l.api.png_get_interlace_type)(png, info),
        (l.api.png_get_rowbytes)(png, info)
    ));
    let tr = PS.with(|p| p.borrow().transforms);
    if tr & 1 != 0 {
        (l.api.png_set_expand)(png);
    }
    if tr & 2 != 0 {
        (l.api.png_set_gray_to_rgb)(png);
    }
    if tr & 4 != 0 {
        (l.api.png_set_strip_16)(png);
    }
    if tr & 8 != 0 {
        (l.api.png_set_packing)(png);
    }
    if (l.api.png_get_interlace_type)(png, info) == 1 {
        let passes = (l.api.png_set_interlace_handling)(png);
        log(format!("passes={passes}"));
    }
    (l.api.png_read_update_info)(png, info);
    let rb = (l.api.png_get_rowbytes)(png, info);
    let h = (l.api.png_get_image_height)(png, info);
    log(format!("info_cb after update rb={rb}"));
    PS.with(|p| {
        let mut p = p.borrow_mut();
        p.rowbytes = rb;
        p.height = h;
        p.rows = (0..h).map(|_| vec![0u8; rb + 16]).collect();
    });
    let (want, save) = PS.with(|p| {
        let p = p.borrow();
        (p.pause_where == 1, p.pause_save)
    });
    if want {
        PS.with(|p| p.borrow_mut().pause_where = 0);
        let rem = (l.api.png_process_data_pause)(png, save);
        PS.with(|p| p.borrow_mut().paused_remaining = rem);
        log(format!("info paused save={save} remaining={rem}"));
    }
}

unsafe extern "C" fn row_cb(png: *mut c_void, new_row: *mut u8, row_num: u32, pass: c_int) {
    let l = cur_lib();
    let (rb, have) = PS.with(|p| {
        let p = p.borrow();
        (p.rowbytes, (row_num as usize) < p.rows.len())
    });
    if have {
        PS.with(|p| {
            let mut p = p.borrow_mut();
            let dst = p.rows[row_num as usize].as_mut_ptr();
            (l.api.png_progressive_combine_row)(png, dst, new_row);
        });
    }
    let snapshot = PS.with(|p| {
        let p = p.borrow();
        if have {
            p.rows[row_num as usize][..rb].to_vec()
        } else {
            vec![]
        }
    });
    log(format!(
        "row_cb r={row_num} p={pass} new_row_null={} row={:02x?}",
        new_row.is_null(),
        snapshot
    ));
    let (want, save) = PS.with(|p| {
        let p = p.borrow();
        (p.pause_where == 2, p.pause_save)
    });
    if want {
        PS.with(|p| p.borrow_mut().pause_where = 0);
        let rem = (l.api.png_process_data_pause)(png, save);
        PS.with(|p| p.borrow_mut().paused_remaining = rem);
        log(format!("paused save={save} remaining={rem}"));
    }
}

unsafe extern "C" fn end_cb(png: *mut c_void, info: *mut c_void) {
    let l = cur_lib();
    log(format!(
        "end_cb: valid_IDAT={}",
        (l.api.png_get_valid)(png, info, PNG_INFO_IDAT)
    ));
}

/// Drive a progressive read, feeding `gran` bytes at a time (0 = whole stream).
fn push_read(
    l: &Lib,
    stream: &[u8],
    gran: usize,
    transforms: u32,
    pause: Option<(u32, c_int)>,
) -> Report {
    PS.with(|p| {
        *p.borrow_mut() = PState {
            transforms,
            pause_where: pause.map_or(0, |(w, _)| w),
            pause_save: pause.map_or(0, |(_, s)| s),
            ..Default::default()
        }
    });
    CUR_LIB.with(|c| *c.borrow_mut() = l as *const Lib);
    let rep = read_session(l, vec![], &mut |l, png, info| unsafe {
        (l.api.png_set_progressive_read_fn)(
            png,
            0xabcd as *mut c_void,
            info_cb as *mut c_void,
            row_cb as *mut c_void,
            end_cb as *mut c_void,
        );
        log(format!(
            "progressive_ptr={:?}",
            (l.api.png_get_progressive_ptr)(png)
        ));
        log(format!("stream_len={}", stream.len()));
        let mut pos = 0usize;
        while pos < stream.len() {
            let n = if gran == 0 {
                stream.len() - pos
            } else {
                gran.min(stream.len() - pos)
            };
            PS.with(|p| p.borrow_mut().paused_remaining = 0);
            (l.api.png_process_data)(png, info, stream[pos..].as_ptr() as *mut u8, n);
            let rem = PS.with(|p| p.borrow().paused_remaining);
            let consumed = n - rem.min(n);
            // NOTE: png_process_data_skip must NOT be called here.  In the C it
            // raises png_app_error("png_process_data_skip is not implemented in
            // any current version of libpng") unless process_mode is
            // PNG_SKIP_MODE, which aborts the whole read.  It is covered by its
            // own test below.
            pos += consumed;
            if consumed == 0 {
                // no forward progress: stop rather than spin
                log("no progress".to_string());
                break;
            }
        }
        let rows = PS.with(|p| p.borrow().rows.clone());
        let rb = PS.with(|p| p.borrow().rowbytes);
        for (i, row) in rows.iter().enumerate() {
            log(format!("final row{i}={:02x?}", &row[..rb.min(row.len())]));
        }
    });
    CUR_LIB.with(|c| *c.borrow_mut() = ptr::null());
    rep
}

fn gen(cl: &Lib, w: u32, h: u32, ct: c_int, bd: c_int, il: c_int) -> Vec<u8> {
    let pal = if ct == PNG_COLOR_TYPE_PALETTE {
        make_palette(pal_for(bd), SEED ^ 0x11)
    } else {
        vec![]
    };
    let rep = write_full(
        cl,
        w,
        h,
        ct,
        bd,
        il,
        PNG_FILTER_TYPE_BASE,
        &pal,
        rowbytes(w, bd, ct),
        SEED ^ ((ct as u64) << 16) ^ (bd as u64) ^ ((il as u64) << 8),
        &mut no_setup,
    );
    assert!(rep.error.is_none());
    rep.out
}

// ---------------------------------------------------------------------------
// P1/P2 every legal shape × feed granularity
// ---------------------------------------------------------------------------
#[test]
fn p1_p2_progressive_all_shapes() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let (w, h) = (17u32, 7u32);
            let stream = gen(&c, w, h, ct, bd, il);
            for gran in [1usize, 2, 3, 7, 13, 64, 0] {
                let mut run = |l: &Lib| -> Report { push_read(l, &stream, gran, 0, None) };
                diff(
                    &format!("P1-P2 progressive ct={ct} bd={bd} il={il} gran={gran}"),
                    &c,
                    &r,
                    &mut run,
                );
                // sanity: the callbacks really fired and rows were produced
                let rep = run(&c);
                assert!(
                    rep.log.iter().any(|s| s.starts_with("row_cb")),
                    "no row callbacks for ct={ct} bd={bd} il={il} gran={gran}: {:?}",
                    rep.brief()
                );
                assert!(
                    rep.log.iter().any(|s| s.starts_with("end_cb")),
                    "no end callback for ct={ct} bd={bd} il={il} gran={gran}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// P3/P4 pause and skip
// ---------------------------------------------------------------------------
#[test]
fn p3_pause_and_resume() {
    let (c, r) = libs();
    // Pause patterns that the C library completes:
    //   (1, 0) / (1, 1)  pause from the info callback, without / with save
    //   (2, 1)           pause from the row callback WITH save
    //
    // NOT tested: (2, 0) — pausing from the ROW callback with save=0.  In that
    // state the C `png_push_read_IDAT` subtracts the already-processed size from
    // the `buffer_size` that `png_process_data_pause` has just zeroed, the
    // unsigned subtraction wraps, and `png_process_data`'s
    // `while (png_ptr->buffer_size)` loop never terminates.  Verified against
    // the reference C `.so`: it does not return.  There is therefore no
    // observable C result to compare a translation against.
    for &(wh, save) in &[(1u32, 0i32), (1, 1), (2, 1)] {
        for &(ct, bd) in &[(2i32, 8i32), (0, 1), (6, 16), (3, 4)] {
            for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                let stream = gen(&c, 15, 6, ct, bd, il);
                for gran in [1usize, 5, 37, 0] {
                    let mut run = |l: &Lib| -> Report {
                        push_read(l, &stream, gran, 0, Some((wh, save)))
                    };
                    diff(
                        &format!("P3 pause where={wh} save={save} gran={gran} ct={ct} bd={bd} il={il}"),
                        &c,
                        &r,
                        &mut run,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// P4 png_process_data_skip: in this libpng version it is an app error unless
// the state machine is in PNG_SKIP_MODE, which never happens for a valid
// stream.  Both implementations must reject it identically.
// ---------------------------------------------------------------------------
#[test]
fn p4_process_data_skip() {
    let (c, r) = libs();
    let stream = gen(&c, 9, 4, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    // before any data
    let mut run = |l: &Lib| -> Report {
        CUR_LIB.with(|cc| *cc.borrow_mut() = l as *const Lib);
        let rep = read_session(l, vec![], &mut |l, png, _info| unsafe {
            (l.api.png_set_progressive_read_fn)(
                png,
                ptr::null_mut(),
                info_cb as *mut c_void,
                row_cb as *mut c_void,
                end_cb as *mut c_void,
            );
            log(format!("skip={}", (l.api.png_process_data_skip)(png)));
        });
        CUR_LIB.with(|cc| *cc.borrow_mut() = ptr::null());
        rep
    };
    diff("P4 png_process_data_skip before data", &c, &r, &mut run);
    // after the whole stream
    let mut run = |l: &Lib| -> Report {
        PS.with(|p| *p.borrow_mut() = PState::default());
        CUR_LIB.with(|cc| *cc.borrow_mut() = l as *const Lib);
        let rep = read_session(l, vec![], &mut |l, png, info| unsafe {
            (l.api.png_set_progressive_read_fn)(
                png,
                ptr::null_mut(),
                info_cb as *mut c_void,
                row_cb as *mut c_void,
                end_cb as *mut c_void,
            );
            (l.api.png_process_data)(png, info, stream.as_ptr() as *mut u8, stream.len());
            log(format!("skip={}", (l.api.png_process_data_skip)(png)));
        });
        CUR_LIB.with(|cc| *cc.borrow_mut() = ptr::null());
        rep
    };
    diff("P4 png_process_data_skip after data", &c, &r, &mut run);
}

// ---------------------------------------------------------------------------
// P6 progressive read with transforms applied in the info callback
// ---------------------------------------------------------------------------
#[test]
fn p6_progressive_transforms() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        let stream = gen(&c, 13, 5, ct, bd, PNG_INTERLACE_NONE);
        for tr in 0..16u32 {
            for gran in [1usize, 11, 0] {
                let mut run = |l: &Lib| -> Report { push_read(l, &stream, gran, tr, None) };
                diff(
                    &format!("P6 progressive transforms tr={tr:#x} gran={gran} ct={ct} bd={bd}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// P7 progressive read of a stream carrying ancillary + unknown chunks
// ---------------------------------------------------------------------------
#[test]
fn p7_progressive_with_chunks() {
    let (c, r) = libs();
    let payload = [7u8, 6, 5, 4, 3, 2, 1];
    let rep = write_full(
        &c,
        11,
        5,
        PNG_COLOR_TYPE_RGB,
        8,
        PNG_INTERLACE_NONE,
        PNG_FILTER_TYPE_BASE,
        &[],
        rowbytes(11, 8, PNG_COLOR_TYPE_RGB),
        SEED ^ 0x77,
        &mut |l, png, info| unsafe {
            (l.api.png_set_gAMA_fixed)(png, info, 45455);
            (l.api.png_set_pHYs)(png, info, 72, 72, 1);
            let t = PngTime { year: 2023, month: 4, day: 5, hour: 6, minute: 7, second: 8 };
            (l.api.png_set_tIME)(png, info, &t);
            let key = std::ffi::CString::new("Comment").unwrap();
            let txt = std::ffi::CString::new("progressive").unwrap();
            let tt = PngText {
                compression: -1,
                key: key.as_ptr() as *mut i8,
                text: txt.as_ptr() as *mut i8,
                text_length: 11,
                ..Default::default()
            };
            (l.api.png_set_text)(png, info, &tt, 1);
            std::mem::forget(key);
            std::mem::forget(txt);
            let unk = [PngUnknownChunk {
                name: *b"prVt\0",
                data: payload.as_ptr() as *mut u8,
                size: payload.len(),
                location: PNG_HAVE_IHDR as u8,
            }];
            (l.api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, ptr::null(), 0);
            (l.api.png_set_unknown_chunks)(png, info, unk.as_ptr(), 1);
            (l.api.png_set_unknown_chunk_location)(png, info, 0, PNG_HAVE_IHDR);
        },
    );
    let stream = rep.out;
    for keep in [0i32, 1, 2, 3] {
        for gran in [1usize, 4, 0] {
            let mut run = |l: &Lib| -> Report {
                PS.with(|p| *p.borrow_mut() = PState::default());
                CUR_LIB.with(|cc| *cc.borrow_mut() = l as *const Lib);
                let rep = read_session(l, vec![], &mut |l, png, info| unsafe {
                    (l.api.png_set_keep_unknown_chunks)(png, keep, ptr::null(), 0);
                    (l.api.png_set_progressive_read_fn)(
                        png,
                        ptr::null_mut(),
                        info_cb as *mut c_void,
                        row_cb as *mut c_void,
                        end_cb as *mut c_void,
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
                    let mut e: *mut PngUnknownChunk = ptr::null_mut();
                    log(format!(
                        "unknown={}",
                        (l.api.png_get_unknown_chunks)(png, info, &mut e)
                    ));
                });
                CUR_LIB.with(|cc| *cc.borrow_mut() = ptr::null());
                rep
            };
            diff(
                &format!("P7 progressive with chunks keep={keep} gran={gran}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}
