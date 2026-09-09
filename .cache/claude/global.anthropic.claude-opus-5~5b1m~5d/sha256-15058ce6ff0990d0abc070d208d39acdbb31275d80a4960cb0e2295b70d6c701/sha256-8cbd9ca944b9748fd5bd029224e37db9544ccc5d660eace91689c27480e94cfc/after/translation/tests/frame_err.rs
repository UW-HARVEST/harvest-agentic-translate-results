//! Phase C — error-path differential tests for the `lz4frame.c` surface.
//!
//! Covers ERRORS.md rows 71-115 plus 142, 143, 144, 156.  Every assertion
//! compares the EXACT `LZ4F_*` return value (i.e. the exact error code, not
//! merely "both failed"), and where relevant the exact in/out `*srcSizePtr` /
//! `*dstSizePtr` values the C leaves behind.
mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_int, c_uint};
use std::ptr;

// ---------------------------------------------------------------------------
// small helpers that drive one library at a time
// ---------------------------------------------------------------------------

struct Cctx {
    lib: &'static Lib,
    p: *mut c_void,
}
impl Cctx {
    fn new(lib: &'static Lib) -> Cctx {
        let mut p: *mut c_void = ptr::null_mut();
        let rc = unsafe {
            lib.get::<Fn_F_createCompressionContext>("LZ4F_createCompressionContext")(
                &mut p,
                LZ4F_VERSION,
            )
        };
        assert!(!is_error(rc), "{}: createCompressionContext -> {}", lib.which, show(rc));
        assert!(!p.is_null());
        Cctx { lib, p }
    }
}
impl Drop for Cctx {
    fn drop(&mut self) {
        unsafe {
            self.lib
                .get::<Fn_F_freeCompressionContext>("LZ4F_freeCompressionContext")(self.p);
        }
    }
}

struct Dctx {
    lib: &'static Lib,
    p: *mut c_void,
}
impl Dctx {
    fn new(lib: &'static Lib) -> Dctx {
        let mut p: *mut c_void = ptr::null_mut();
        let rc = unsafe {
            lib.get::<Fn_F_createDecompressionContext>("LZ4F_createDecompressionContext")(
                &mut p,
                LZ4F_VERSION,
            )
        };
        assert!(!is_error(rc), "{}: createDecompressionContext -> {}", lib.which, show(rc));
        assert!(!p.is_null());
        Dctx { lib, p }
    }
    fn reset(&self) {
        unsafe {
            self.lib
                .get::<Fn_F_resetDecompressionContext>("LZ4F_resetDecompressionContext")(self.p);
        }
    }
}
impl Drop for Dctx {
    fn drop(&mut self) {
        unsafe {
            self.lib
                .get::<Fn_F_freeDecompressionContext>("LZ4F_freeDecompressionContext")(self.p);
        }
    }
}

/// One-shot frame compression with the given library.
fn compress_frame(lib: &'static Lib, src: &[u8], prefs: Option<&LZ4F_preferences_t>) -> Vec<u8> {
    let pp = prefs.map(|p| p as *const _).unwrap_or(ptr::null());
    let bound =
        unsafe { lib.get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(src.len(), pp) };
    assert!(!is_error(bound));
    let mut out = vec![0u8; bound];
    let n = unsafe {
        lib.get::<Fn_F_compressFrame>("LZ4F_compressFrame")(
            out.as_mut_ptr() as *mut c_void,
            out.len(),
            src.as_ptr() as *const c_void,
            src.len(),
            pp,
        )
    };
    assert!(!is_error(n), "{}: compressFrame -> {}", lib.which, show(n));
    out.truncate(n);
    out
}

/// Feed `frame` to `LZ4F_decompress` in one call with an ample output buffer
/// and return `(return_code, dst_written, src_consumed, output_bytes)`.
fn decompress_once(lib: &'static Lib, frame: &[u8], dst_cap: usize) -> (usize, usize, usize, Vec<u8>) {
    let dctx = Dctx::new(lib);
    let mut out = vec![0u8; dst_cap.max(1)];
    let mut dst_size = dst_cap;
    let mut src_size = frame.len();
    let rc = unsafe {
        lib.get::<Fn_F_decompress>("LZ4F_decompress")(
            dctx.p,
            out.as_mut_ptr() as *mut c_void,
            &mut dst_size,
            frame.as_ptr() as *const c_void,
            &mut src_size,
            ptr::null(),
        )
    };
    out.truncate(dst_size);
    (rc, dst_size, src_size, out)
}

/// Drive `LZ4F_decompress` in a loop until it returns 0, errors, or stalls.
/// Returns `(final_return_code, total_out, total_consumed, output)`.
fn decompress_loop(
    lib: &'static Lib,
    frame: &[u8],
    chunk: usize,
    out_chunk: usize,
) -> (usize, usize, usize, Vec<u8>) {
    let dctx = Dctx::new(lib);
    let mut consumed = 0usize;
    let mut produced = Vec::new();
    let mut buf = vec![0u8; out_chunk.max(1)];
    let mut rc = 1usize;
    let mut guard = 0;
    while consumed < frame.len() && rc != 0 {
        guard += 1;
        if guard > 100_000 {
            break;
        }
        let take = chunk.min(frame.len() - consumed);
        let mut src_size = take;
        let mut dst_size = buf.len();
        rc = unsafe {
            lib.get::<Fn_F_decompress>("LZ4F_decompress")(
                dctx.p,
                buf.as_mut_ptr() as *mut c_void,
                &mut dst_size,
                frame[consumed..].as_ptr() as *const c_void,
                &mut src_size,
                ptr::null(),
            )
        };
        if is_error(rc) {
            return (rc, produced.len(), consumed, produced);
        }
        produced.extend_from_slice(&buf[..dst_size]);
        consumed += src_size;
        if src_size == 0 && dst_size == 0 {
            break; // stalled
        }
    }
    (rc, produced.len(), consumed, produced)
}

// ===========================================================================
// ERRORS.md rows 71, 72 — LZ4F_getBlockSize range check + out-of-range enums
// ===========================================================================

#[test]
fn err_getBlockSize_all_int_values() {
    let mut vals: Vec<c_int> = (-8..=24).collect();
    vals.extend_from_slice(&[
        99,
        255,
        256,
        1000,
        -100,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x0001_0004, // low bits look like max64KB but the value is out of range
    ]);
    for v in vals {
        let (cv, rv) = both(|l| unsafe { l.get::<Fn_F_getBlockSize>("LZ4F_getBlockSize")(v) });
        assert_eq!(
            cv,
            rv,
            "LZ4F_getBlockSize({}) C={} Rust={}",
            v,
            show(cv),
            show(rv)
        );
    }
    // sanity: the four valid IDs must yield the documented sizes
    for (id, want) in [
        (LZ4F_DEFAULT, 64 * 1024),
        (LZ4F_MAX64KB, 64 * 1024),
        (LZ4F_MAX256KB, 256 * 1024),
        (LZ4F_MAX1MB, 1024 * 1024),
        (LZ4F_MAX4MB, 4 * 1024 * 1024),
    ] {
        let (cv, _) = both(|l| unsafe { l.get::<Fn_F_getBlockSize>("LZ4F_getBlockSize")(id) });
        assert_eq!(cv, want, "C LZ4F_getBlockSize({})", id);
    }
}

// ===========================================================================
// ERRORS.md rows 142, 143 — error classification / naming round-trip
// ===========================================================================

#[test]
fn err_isError_getErrorCode_getErrorName_parity() {
    let mut probes: Vec<usize> = Vec::new();
    // every code 0..=30 in the negated-size_t representation
    for code in 0..=30usize {
        probes.push(0usize.wrapping_sub(code));
    }
    // plus plausible non-error return values and the exact boundary
    probes.extend_from_slice(&[
        0,
        1,
        2,
        100,
        65535,
        usize::MAX / 2,
        usize::MAX - LZ4F_ERROR_MAX_CODE,     // NOT an error (boundary)
        usize::MAX - LZ4F_ERROR_MAX_CODE + 1, // IS an error (boundary)
        usize::MAX - 1,
        usize::MAX,
    ]);
    for p in probes {
        let (ci, ri) = both(|l| unsafe { l.get::<Fn_F_isError>("LZ4F_isError")(p) });
        assert_eq!(ci, ri, "LZ4F_isError({:#x})", p);

        let (cc, rc) = both(|l| unsafe { l.get::<Fn_F_getErrorCode>("LZ4F_getErrorCode")(p) });
        assert_eq!(cc, rc, "LZ4F_getErrorCode({:#x})", p);

        let (cn, rn) = both(|l| unsafe {
            cstr(l.get::<Fn_F_getErrorName>("LZ4F_getErrorName")(p))
        });
        assert_eq!(cn, rn, "LZ4F_getErrorName({:#x})", p);
    }
}

// ===========================================================================
// ERRORS.md rows 76, 77, 92, 93 — context creation
// ===========================================================================

#[test]
fn err_create_context_null_out_pointer() {
    // LZ4F_createCompressionContext(NULL, version) -> parameter_null
    let (cv, rv) = both(|l| unsafe {
        l.get::<Fn_F_createCompressionContext>("LZ4F_createCompressionContext")(
            ptr::null_mut(),
            LZ4F_VERSION,
        )
    });
    assert_eq!(cv, rv, "createCompressionContext(NULL): C={} Rust={}", show(cv), show(rv));
    assert!(is_error(cv));

    let (cv, rv) = both(|l| unsafe {
        l.get::<Fn_F_createDecompressionContext>("LZ4F_createDecompressionContext")(
            ptr::null_mut(),
            LZ4F_VERSION,
        )
    });
    assert_eq!(cv, rv, "createDecompressionContext(NULL): C={} Rust={}", show(cv), show(rv));
    assert!(is_error(cv));
}

#[test]
fn err_create_context_version_values() {
    // the C stores but does not validate @version; every value must behave the same
    for v in [0u32, 1, 99, LZ4F_VERSION, 101, 1000, u32::MAX] {
        let cv = {
            let mut p = ptr::null_mut();
            let rc = unsafe {
                c().get::<Fn_F_createCompressionContext>("LZ4F_createCompressionContext")(
                    &mut p, v,
                )
            };
            if !p.is_null() {
                unsafe {
                    c().get::<Fn_F_freeCompressionContext>("LZ4F_freeCompressionContext")(p)
                };
            }
            (rc, p.is_null())
        };
        let rv = {
            let mut p = ptr::null_mut();
            let rc = unsafe {
                r().get::<Fn_F_createCompressionContext>("LZ4F_createCompressionContext")(
                    &mut p, v,
                )
            };
            if !p.is_null() {
                unsafe {
                    r().get::<Fn_F_freeCompressionContext>("LZ4F_freeCompressionContext")(p)
                };
            }
            (rc, p.is_null())
        };
        assert_eq!(cv, rv, "createCompressionContext version={}", v);

        let cv = {
            let mut p = ptr::null_mut();
            let rc = unsafe {
                c().get::<Fn_F_createDecompressionContext>("LZ4F_createDecompressionContext")(
                    &mut p, v,
                )
            };
            if !p.is_null() {
                unsafe {
                    c().get::<Fn_F_freeDecompressionContext>("LZ4F_freeDecompressionContext")(p)
                };
            }
            (rc, p.is_null())
        };
        let rv = {
            let mut p = ptr::null_mut();
            let rc = unsafe {
                r().get::<Fn_F_createDecompressionContext>("LZ4F_createDecompressionContext")(
                    &mut p, v,
                )
            };
            if !p.is_null() {
                unsafe {
                    r().get::<Fn_F_freeDecompressionContext>("LZ4F_freeDecompressionContext")(p)
                };
            }
            (rc, p.is_null())
        };
        assert_eq!(cv, rv, "createDecompressionContext version={}", v);
    }
}

#[test]
fn err_free_context_null() {
    let (cv, rv) = both(|l| unsafe {
        l.get::<Fn_F_freeCompressionContext>("LZ4F_freeCompressionContext")(ptr::null_mut())
    });
    assert_eq!(cv, rv, "freeCompressionContext(NULL)");
    let (cv, rv) = both(|l| unsafe {
        l.get::<Fn_F_freeDecompressionContext>("LZ4F_freeDecompressionContext")(ptr::null_mut())
    });
    assert_eq!(cv, rv, "freeDecompressionContext(NULL)");
}

/// `LZ4F_freeDecompressionContext` documents that its return value reports the
/// dctx stage — a dctx released mid-frame must report the SAME value in both.
#[test]
fn err_free_decompression_context_reports_stage() {
    let mut rng = Rng::new(0xF00D_0001);
    let src = gen(&mut rng, 40_000, Shape::Text);
    let frame = compress_frame(c(), &src, None);

    for take in [0usize, 1, 5, 7, 10, 19, 20, 25, 40, frame.len() / 2, frame.len() - 1] {
        let vals = both(|l| {
            let dctx = Dctx::new(l);
            let mut out = vec![0u8; 70_000];
            let mut ds = out.len();
            let mut ss = take.min(frame.len());
            unsafe {
                l.get::<Fn_F_decompress>("LZ4F_decompress")(
                    dctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    &mut ds,
                    frame.as_ptr() as *const c_void,
                    &mut ss,
                    ptr::null(),
                );
                // release explicitly to capture the return value
                let stage = l
                    .get::<Fn_F_freeDecompressionContext>("LZ4F_freeDecompressionContext")(dctx.p);
                std::mem::forget(dctx);
                stage
            }
        });
        assert_eq!(vals.0, vals.1, "freeDecompressionContext stage after take={}", take);
    }
}

// ===========================================================================
// ERRORS.md rows 102, 103, 104 — LZ4F_headerSize
// ===========================================================================

#[test]
fn err_headerSize_null_short_and_bad_magic() {
    // NULL src -> srcPtr_wrong
    for len in [0usize, 1, 4, 5, 7, 19, 100] {
        let (cv, rv) =
            both(|l| unsafe { l.get::<Fn_F_headerSize>("LZ4F_headerSize")(ptr::null(), len) });
        assert_eq!(cv, rv, "headerSize(NULL, {}): C={} Rust={}", len, show(cv), show(rv));
    }

    // srcSize below LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH (5)
    let mut buf = [0u8; 32];
    buf[..4].copy_from_slice(&LZ4F_MAGICNUMBER.to_le_bytes());
    buf[4] = 0x40;
    for len in 0..=8usize {
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_F_headerSize>("LZ4F_headerSize")(buf.as_ptr() as *const c_void, len)
        });
        assert_eq!(cv, rv, "headerSize(len={}): C={} Rust={}", len, show(cv), show(rv));
    }

    // every magic number variation: valid, each skippable value, and neighbours
    let magics: Vec<u32> = {
        let mut v = vec![
            0u32,
            1,
            0xFFFF_FFFF,
            LZ4F_MAGICNUMBER,
            LZ4F_MAGICNUMBER - 1,
            LZ4F_MAGICNUMBER + 1,
            LZ4F_MAGIC_SKIPPABLE_START - 1,
            LZ4F_MAGIC_SKIPPABLE_START + 16,
            0x184D_2205,
            0x184D_2A4F,
        ];
        for i in 0..16u32 {
            v.push(LZ4F_MAGIC_SKIPPABLE_START + i);
        }
        v
    };
    for m in magics {
        let mut b = [0u8; 32];
        b[..4].copy_from_slice(&m.to_le_bytes());
        b[4] = 0x40;
        b[5] = 0x70;
        for len in [5usize, 7, 8, 19, 32] {
            let (cv, rv) = both(|l| unsafe {
                l.get::<Fn_F_headerSize>("LZ4F_headerSize")(b.as_ptr() as *const c_void, len)
            });
            assert_eq!(
                cv,
                rv,
                "headerSize(magic={:#x}, len={}): C={} Rust={}",
                m,
                len,
                show(cv),
                show(rv)
            );
        }
    }
}

// ===========================================================================
// ERRORS.md rows 79, 82 — LZ4F_compressBegin family
// ===========================================================================

#[test]
fn err_compressBegin_dst_too_small() {
    for cap in 0..=LZ4F_HEADER_SIZE_MAX + 2 {
        let vals = both(|l| {
            let cctx = Cctx::new(l);
            let mut out = vec![0u8; LZ4F_HEADER_SIZE_MAX + 8];
            unsafe {
                l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                    cctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr::null(),
                )
            }
        });
        assert_eq!(
            vals.0,
            vals.1,
            "compressBegin(cap={}): C={} Rust={}",
            cap,
            show(vals.0),
            show(vals.1)
        );
        // rows: cap < 19 must be dstMaxSize_tooSmall
        if cap < LZ4F_HEADER_SIZE_MAX {
            assert!(is_error(vals.0), "cap={} should be an error", cap);
            assert_eq!(err_code(vals.0), 11, "cap={} expected dstMaxSize_tooSmall", cap);
        }
    }
}

/// ERRORS.md row 82: `dictSize > INT_MAX` in `LZ4F_compressBegin_usingDict`.
/// The C checks the range BEFORE touching `dictBuffer` (lz4frame.c:768), so a
/// small real buffer with a huge declared size is safe and is exactly the
/// input the C rejects.
#[test]
fn err_compressBegin_usingDict_dictSize_too_large() {
    let dict = [7u8; 64];
    // Every value here is STRICTLY greater than INT_MAX, so the C returns
    // parameter_invalid at lz4frame.c:768 before it ever reads @dictBuffer.
    // (i32::MAX itself is NOT rejected, so it must not appear in this list —
    // the C would then try to load a 2 GB dictionary from a 64-byte buffer.)
    for &dict_size in &[
        i32::MAX as usize + 1,
        usize::MAX / 2,
        usize::MAX,
        0x1_0000_0000usize,
    ] {
        for level in [0i32, 3, 9, 12] {
            let mut prefs = LZ4F_preferences_t::default();
            prefs.compressionLevel = level;
            let vals = both(|l| {
                let cctx = Cctx::new(l);
                let mut out = vec![0u8; 64];
                unsafe {
                    l.get::<Fn_F_compressBegin_usingDict>("LZ4F_compressBegin_usingDict")(
                        cctx.p,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        dict.as_ptr() as *const c_void,
                        dict_size,
                        &prefs,
                    )
                }
            });
            assert_eq!(
                vals.0,
                vals.1,
                "compressBegin_usingDict(dictSize={:#x}, level={}): C={} Rust={}",
                dict_size,
                level,
                show(vals.0),
                show(vals.1)
            );
        }
    }
}

/// `LZ4F_compressBegin_usingDict` / `_usingDictOnce` / `_usingCDict` /
/// `_internal` with NULL dict, zero dictSize, and NULL cdict.
#[test]
fn err_compressBegin_dict_variants_null_and_zero() {
    let dict = [3u8; 128];
    let cases: Vec<(*const c_void, usize)> = vec![
        (ptr::null(), 0),
        (ptr::null(), 100),
        (dict.as_ptr() as *const c_void, 0),
        (dict.as_ptr() as *const c_void, 1),
        (dict.as_ptr() as *const c_void, 128),
    ];
    for (dp, ds) in cases {
        for name in [
            "LZ4F_compressBegin_usingDict",
            "LZ4F_compressBegin_usingDictOnce",
        ] {
            let vals = both(|l| {
                let cctx = Cctx::new(l);
                let mut out = vec![0u8; 64];
                let n = unsafe {
                    l.get::<Fn_F_compressBegin_usingDict>(name)(
                        cctx.p,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        dp,
                        ds,
                        ptr::null(),
                    )
                };
                (n, if is_error(n) { Vec::new() } else { out[..n].to_vec() })
            });
            assert_eq!(
                vals.0 .0,
                vals.1 .0,
                "{}(dict={:?}, size={}): C={} Rust={}",
                name,
                dp,
                ds,
                show(vals.0 .0),
                show(vals.1 .0)
            );
            assert_bytes_eq!(format!("{} header bytes", name), vals.0 .1, vals.1 .1);
        }

        // _usingCDict with a NULL cdict
        let vals = both(|l| {
            let cctx = Cctx::new(l);
            let mut out = vec![0u8; 64];
            let n = unsafe {
                l.get::<Fn_F_compressBegin_usingCDict>("LZ4F_compressBegin_usingCDict")(
                    cctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    out.len(),
                    ptr::null(),
                    ptr::null(),
                )
            };
            (n, if is_error(n) { Vec::new() } else { out[..n].to_vec() })
        });
        assert_eq!(vals.0 .0, vals.1 .0, "compressBegin_usingCDict(NULL cdict)");
        assert_bytes_eq!("usingCDict header", vals.0 .1, vals.1 .1);

        // _internal with both dict and cdict NULL
        let vals = both(|l| {
            let cctx = Cctx::new(l);
            let mut out = vec![0u8; 64];
            let n = unsafe {
                l.get::<Fn_F_compressBegin_internal>("LZ4F_compressBegin_internal")(
                    cctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    out.len(),
                    dp,
                    ds,
                    ptr::null(),
                    ptr::null(),
                )
            };
            (n, if is_error(n) { Vec::new() } else { out[..n].to_vec() })
        });
        assert_eq!(
            vals.0 .0,
            vals.1 .0,
            "compressBegin_internal(dict={:?}, size={})",
            dp,
            ds
        );
        assert_bytes_eq!("internal header", vals.0 .1, vals.1 .1);
    }
}

// ===========================================================================
// ERRORS.md rows 83, 84, 85 — compressUpdate / uncompressedUpdate
// ===========================================================================

/// Row 83: `LZ4F_compressUpdate` before `LZ4F_compressBegin` -> uninitialized.
/// Also after `LZ4F_compressEnd` (cStage is reset to 0).
#[test]
fn err_compressUpdate_wrong_stage() {
    let src = [1u8; 100];
    for name in ["LZ4F_compressUpdate", "LZ4F_uncompressedUpdate"] {
        // (a) no compressBegin at all
        let vals = both(|l| {
            let cctx = Cctx::new(l);
            let mut out = vec![0u8; 100_000];
            unsafe {
                l.get::<Fn_F_compressUpdate>(name)(
                    cctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    out.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null(),
                )
            }
        });
        assert_eq!(vals.0, vals.1, "{} before begin: C={} Rust={}", name, show(vals.0), show(vals.1));
        assert_eq!(err_code(vals.0), 20, "{} before begin should be state_uninitialized", name);

        // (b) after compressEnd
        let vals = both(|l| {
            let cctx = Cctx::new(l);
            let mut out = vec![0u8; 200_000];
            unsafe {
                let h = l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                    cctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    out.len(),
                    ptr::null(),
                );
                assert!(!is_error(h));
                let e = l.get::<Fn_F_compressEnd>("LZ4F_compressEnd")(
                    cctx.p,
                    out.as_mut_ptr().add(h) as *mut c_void,
                    out.len() - h,
                    ptr::null(),
                );
                assert!(!is_error(e));
                l.get::<Fn_F_compressUpdate>(name)(
                    cctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    out.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null(),
                )
            }
        });
        assert_eq!(vals.0, vals.1, "{} after end: C={} Rust={}", name, show(vals.0), show(vals.1));
        assert_eq!(err_code(vals.0), 20, "{} after end should be state_uninitialized", name);
    }
}

/// Rows 84 / 85: insufficient `dstCapacity` for `LZ4F_compressUpdate` and
/// `LZ4F_uncompressedUpdate`, swept over every block size and both block
/// modes so the required bound differs per configuration.
#[test]
fn err_compressUpdate_dst_too_small() {
    let mut rng = Rng::new(0xF00D_0002);
    for &bsid in &[LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
        for &bmode in &[LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for &bcs in &[LZ4F_NO_BLOCK_CHECKSUM, LZ4F_BLOCK_CHECKSUM_ENABLED] {
                let mut prefs = LZ4F_preferences_t::default();
                prefs.frameInfo.blockSizeID = bsid;
                prefs.frameInfo.blockMode = bmode;
                prefs.frameInfo.blockChecksumFlag = bcs;
                for &srclen in &[0usize, 1, 100, 5000, 70_000] {
                    let src = gen(&mut rng, srclen, Shape::Text);
                    // the required bound, per the C
                    let bound = unsafe {
                        c().get::<Fn_F_compressBound>("LZ4F_compressBound")(srclen, &prefs)
                    };
                    assert!(!is_error(bound));
                    let caps = [
                        0usize,
                        1,
                        bound / 4,
                        bound / 2,
                        bound.saturating_sub(1),
                        bound,
                        bound + 1,
                    ];
                    for &cap in &caps {
                        for name in ["LZ4F_compressUpdate", "LZ4F_uncompressedUpdate"] {
                            let vals = both(|l| {
                                let cctx = Cctx::new(l);
                                let mut hdr = vec![0u8; 64];
                                let h = unsafe {
                                    l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                                        cctx.p,
                                        hdr.as_mut_ptr() as *mut c_void,
                                        hdr.len(),
                                        &prefs,
                                    )
                                };
                                assert!(!is_error(h));
                                let mut out = vec![0u8; bound + 64];
                                let n = unsafe {
                                    l.get::<Fn_F_compressUpdate>(name)(
                                        cctx.p,
                                        out.as_mut_ptr() as *mut c_void,
                                        cap,
                                        src.as_ptr() as *const c_void,
                                        src.len(),
                                        ptr::null(),
                                    )
                                };
                                (n, if is_error(n) { Vec::new() } else { out[..n].to_vec() })
                            });
                            assert_eq!(
                                vals.0 .0,
                                vals.1 .0,
                                "{} bsid={} bmode={} bcs={} srclen={} cap={} (bound={}): C={} Rust={}",
                                name, bsid, bmode, bcs, srclen, cap, bound,
                                show(vals.0 .0),
                                show(vals.1 .0)
                            );
                            assert_bytes_eq!(
                                format!("{} bytes cap={} srclen={}", name, cap, srclen),
                                vals.0 .1,
                                vals.1 .1
                            );
                        }
                    }
                }
            }
        }
    }
}

/// `LZ4F_compressUpdate` with a NULL src and a zero srcSize.
///
/// Only `srcSize == 0` is probed: a non-zero size with a NULL src makes the C
/// `memcpy`/compress from address 0, which is undefined behaviour rather than a
/// rejection the library defines, so it is not part of the error surface.
#[test]
fn err_compressUpdate_null_src() {
    for size in [0usize] {
        for name in ["LZ4F_compressUpdate", "LZ4F_uncompressedUpdate"] {
            let vals = both(|l| {
                let cctx = Cctx::new(l);
                let mut out = vec![0u8; 300_000];
                unsafe {
                    let h = l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                        cctx.p,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        ptr::null(),
                    );
                    assert!(!is_error(h));
                    l.get::<Fn_F_compressUpdate>(name)(
                        cctx.p,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        ptr::null(),
                        size,
                        ptr::null(),
                    )
                }
            });
            // size == 0 with a NULL src is legal and must be a no-op; a
            // non-zero size with NULL src is UB in C, so only probe size == 0.
            if size == 0 {
                assert_eq!(vals.0, vals.1, "{}(NULL, 0)", name);
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 86-91 — flush / compressEnd
// ===========================================================================

#[test]
fn err_flush_and_end_dst_too_small() {
    let mut rng = Rng::new(0xF00D_0003);
    let src = gen(&mut rng, 3000, Shape::Text);
    for &ccs in &[LZ4F_NO_CONTENT_CHECKSUM, LZ4F_CONTENT_CHECKSUM_ENABLED] {
        for &autoflush in &[0u32, 1] {
            let mut prefs = LZ4F_preferences_t::default();
            prefs.frameInfo.contentChecksumFlag = ccs;
            prefs.autoFlush = autoflush;
            for cap in [0usize, 1, 2, 3, 4, 5, 7, 8, 9, 12, 100, 5000] {
                // flush
                let vals = both(|l| {
                    let cctx = Cctx::new(l);
                    let mut out = vec![0u8; 200_000];
                    unsafe {
                        let h = l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                            cctx.p,
                            out.as_mut_ptr() as *mut c_void,
                            out.len(),
                            &prefs,
                        );
                        assert!(!is_error(h));
                        let u = l.get::<Fn_F_compressUpdate>("LZ4F_compressUpdate")(
                            cctx.p,
                            out.as_mut_ptr().add(h) as *mut c_void,
                            out.len() - h,
                            src.as_ptr() as *const c_void,
                            src.len(),
                            ptr::null(),
                        );
                        assert!(!is_error(u));
                        let mut fbuf = vec![0u8; 200_000];
                        let f = l.get::<Fn_F_flush>("LZ4F_flush")(
                            cctx.p,
                            fbuf.as_mut_ptr() as *mut c_void,
                            cap,
                            ptr::null(),
                        );
                        (f, if is_error(f) { Vec::new() } else { fbuf[..f].to_vec() })
                    }
                });
                assert_eq!(
                    vals.0 .0,
                    vals.1 .0,
                    "flush(cap={}, ccs={}, autoflush={}): C={} Rust={}",
                    cap, ccs, autoflush,
                    show(vals.0 .0),
                    show(vals.1 .0)
                );
                assert_bytes_eq!(format!("flush bytes cap={}", cap), vals.0 .1, vals.1 .1);

                // compressEnd
                let vals = both(|l| {
                    let cctx = Cctx::new(l);
                    let mut out = vec![0u8; 200_000];
                    unsafe {
                        let h = l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                            cctx.p,
                            out.as_mut_ptr() as *mut c_void,
                            out.len(),
                            &prefs,
                        );
                        assert!(!is_error(h));
                        let u = l.get::<Fn_F_compressUpdate>("LZ4F_compressUpdate")(
                            cctx.p,
                            out.as_mut_ptr().add(h) as *mut c_void,
                            out.len() - h,
                            src.as_ptr() as *const c_void,
                            src.len(),
                            ptr::null(),
                        );
                        assert!(!is_error(u));
                        let mut ebuf = vec![0u8; 200_000];
                        let e = l.get::<Fn_F_compressEnd>("LZ4F_compressEnd")(
                            cctx.p,
                            ebuf.as_mut_ptr() as *mut c_void,
                            cap,
                            ptr::null(),
                        );
                        (e, if is_error(e) { Vec::new() } else { ebuf[..e].to_vec() })
                    }
                });
                assert_eq!(
                    vals.0 .0,
                    vals.1 .0,
                    "compressEnd(cap={}, ccs={}, autoflush={}): C={} Rust={}",
                    cap, ccs, autoflush,
                    show(vals.0 .0),
                    show(vals.1 .0)
                );
                assert_bytes_eq!(format!("end bytes cap={}", cap), vals.0 .1, vals.1 .1);
            }
        }
    }
}

/// ERRORS.md row 86: `LZ4F_flush` on a context that has not begun a frame.
#[test]
fn err_flush_wrong_stage() {
    let vals = both(|l| {
        let cctx = Cctx::new(l);
        let mut out = vec![0u8; 1000];
        unsafe {
            l.get::<Fn_F_flush>("LZ4F_flush")(
                cctx.p,
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                ptr::null(),
            )
        }
    });
    assert_eq!(vals.0, vals.1, "flush before begin: C={} Rust={}", show(vals.0), show(vals.1));
}

/// ERRORS.md row 90: declared `contentSize` != actual bytes fed.
#[test]
fn err_compressEnd_content_size_mismatch() {
    let mut rng = Rng::new(0xF00D_0004);
    let src = gen(&mut rng, 5000, Shape::Text);
    for &declared in &[
        0u64,      // 0 means "unknown" -> no check
        1,
        4999,
        5000,      // exact -> no error
        5001,
        u64::MAX,
        100_000,
    ] {
        let mut prefs = LZ4F_preferences_t::default();
        prefs.frameInfo.contentSize = declared;
        let vals = both(|l| {
            let cctx = Cctx::new(l);
            let mut out = vec![0u8; 200_000];
            unsafe {
                let h = l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                    cctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    out.len(),
                    &prefs,
                );
                assert!(!is_error(h));
                let u = l.get::<Fn_F_compressUpdate>("LZ4F_compressUpdate")(
                    cctx.p,
                    out.as_mut_ptr().add(h) as *mut c_void,
                    out.len() - h,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null(),
                );
                assert!(!is_error(u));
                let e = l.get::<Fn_F_compressEnd>("LZ4F_compressEnd")(
                    cctx.p,
                    out.as_mut_ptr().add(h + u) as *mut c_void,
                    out.len() - h - u,
                    ptr::null(),
                );
                (e, h, u)
            }
        });
        assert_eq!(
            vals.0,
            vals.1,
            "compressEnd contentSize={} (actual 5000): C={} Rust={}",
            declared,
            show(vals.0 .0),
            show(vals.1 .0)
        );
        if declared != 0 && declared != 5000 {
            assert_eq!(err_code(vals.0 .0), 14, "declared={} expected frameSize_wrong", declared);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 94-101 — frame header decoding
// ===========================================================================

/// Build a valid 19-byte-max header via `LZ4F_compressBegin`, then mutate the
/// FLG / BD / HC bytes to hit every header rejection branch.
#[test]
fn err_decode_header_all_reserved_and_version_bits() {
    // start from a header with contentSize + dictID present so it is 19 bytes
    let mut prefs = LZ4F_preferences_t::default();
    prefs.frameInfo.contentSize = 1234;
    prefs.frameInfo.dictID = 0xABCD_1234;
    prefs.frameInfo.blockSizeID = LZ4F_MAX256KB;
    prefs.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;
    prefs.frameInfo.blockChecksumFlag = LZ4F_BLOCK_CHECKSUM_ENABLED;

    let base = {
        let cctx = Cctx::new(c());
        let mut out = vec![0u8; 64];
        let n = unsafe {
            c().get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                cctx.p,
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                &prefs,
            )
        };
        assert!(!is_error(n));
        out.truncate(n);
        out
    };
    assert_eq!(base.len(), LZ4F_HEADER_SIZE_MAX, "expected a maximal header");

    // Exhaustively vary the FLG byte (index 4) and the BD byte (index 5).
    for flg in 0u16..=255 {
        for bd in [0u8, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0xF0, 0x71, 0x7F, 0x4F] {
            let mut f = base.clone();
            f[4] = flg as u8;
            f[5] = bd;
            // note: the HC byte is intentionally left stale so most of these
            // also exercise the header-checksum branch; both libraries must
            // agree on WHICH error wins.
            let cv = decompress_once(c(), &f, 1 << 20);
            let rv = decompress_once(r(), &f, 1 << 20);
            assert_eq!(
                (cv.0, cv.1, cv.2),
                (rv.0, rv.1, rv.2),
                "header flg={:#04x} bd={:#04x}: C=({}, dst={}, src={}) Rust=({}, dst={}, src={})",
                flg, bd, show(cv.0), cv.1, cv.2, show(rv.0), rv.1, rv.2
            );
        }
    }
}

/// Same sweep, but the header checksum is RECOMPUTED after each mutation so
/// the checksum branch never masks the FLG/BD branches.  The HC byte is
/// `(XXH32(header+4, len-5, 0) >> 8) & 0xFF`.
#[test]
fn err_decode_header_with_valid_checksum() {
    let mut prefs = LZ4F_preferences_t::default();
    prefs.frameInfo.contentSize = 4242;
    prefs.frameInfo.dictID = 0x1111_2222;
    let base = {
        let cctx = Cctx::new(c());
        let mut out = vec![0u8; 64];
        let n = unsafe {
            c().get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                cctx.p,
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                &prefs,
            )
        };
        assert!(!is_error(n));
        out.truncate(n);
        out
    };

    let xxh32 = unsafe { c().get::<Fn_XXH32>("LZ4_XXH32") };
    for flg in 0u16..=255 {
        for bd in 0u16..=255 {
            // only interesting BD values (bits 0-3 and 7 reserved, 4-6 = id)
            if bd & 0x0F != 0 && bd & 0x0F != 1 && bd != 0xFF {
                continue;
            }
            let mut f = base.clone();
            f[4] = flg as u8;
            f[5] = bd as u8;
            // recompute HC over bytes [4 .. len-1)
            let body = &f[4..f.len() - 1];
            let hc = ((unsafe { xxh32(body.as_ptr() as *const c_void, body.len(), 0) } >> 8)
                & 0xFF) as u8;
            let last = f.len() - 1;
            f[last] = hc;

            let cv = decompress_once(c(), &f, 1 << 20);
            let rv = decompress_once(r(), &f, 1 << 20);
            assert_eq!(
                (cv.0, cv.1, cv.2),
                (rv.0, rv.1, rv.2),
                "valid-HC header flg={:#04x} bd={:#04x}: C=({}, {}, {}) Rust=({}, {}, {})",
                flg, bd, show(cv.0), cv.1, cv.2, show(rv.0), rv.1, rv.2
            );
        }
    }
}

/// ERRORS.md row 101: a deliberately wrong header-checksum byte.
#[test]
fn err_decode_header_checksum_invalid() {
    let mut rng = Rng::new(0xF00D_0005);
    let src = gen(&mut rng, 1000, Shape::Text);
    for &(cs, did) in &[(0u64, 0u32), (1000, 0), (0, 42), (1000, 42)] {
        let mut prefs = LZ4F_preferences_t::default();
        prefs.frameInfo.contentSize = cs;
        prefs.frameInfo.dictID = did;
        let frame = compress_frame(c(), &src, Some(&prefs));
        let hdr_len = unsafe {
            c().get::<Fn_F_headerSize>("LZ4F_headerSize")(frame.as_ptr() as *const c_void, frame.len())
        };
        assert!(!is_error(hdr_len));
        for delta in 1u8..=8 {
            let mut f = frame.clone();
            f[hdr_len - 1] = f[hdr_len - 1].wrapping_add(delta);
            let cv = decompress_once(c(), &f, 1 << 20);
            let rv = decompress_once(r(), &f, 1 << 20);
            assert_eq!(
                (cv.0, cv.1, cv.2),
                (rv.0, rv.1, rv.2),
                "bad HC (cs={}, dictID={}, delta={}): C={} Rust={}",
                cs, did, delta, show(cv.0), show(rv.0)
            );
            assert_eq!(err_code(cv.0), 17, "expected headerChecksum_invalid");
        }
    }
}

/// ERRORS.md row 95: bad magic number on the decompression path.
#[test]
fn err_decompress_bad_magic() {
    let mut rng = Rng::new(0xF00D_0006);
    let src = gen(&mut rng, 500, Shape::Text);
    let frame = compress_frame(c(), &src, None);
    let mut magics: Vec<u32> = vec![
        0,
        1,
        0xFFFF_FFFF,
        LZ4F_MAGICNUMBER ^ 1,
        LZ4F_MAGICNUMBER + 1,
        LZ4F_MAGICNUMBER - 1,
        LZ4F_MAGIC_SKIPPABLE_START - 1,
        LZ4F_MAGIC_SKIPPABLE_START + 16,
    ];
    for i in 0..16u32 {
        magics.push(LZ4F_MAGIC_SKIPPABLE_START + i);
    }
    for m in magics {
        let mut f = frame.clone();
        f[..4].copy_from_slice(&m.to_le_bytes());
        let cv = decompress_once(c(), &f, 1 << 20);
        let rv = decompress_once(r(), &f, 1 << 20);
        assert_eq!(
            (cv.0, cv.1, cv.2),
            (rv.0, rv.1, rv.2),
            "magic={:#x}: C=({}, {}, {}) Rust=({}, {}, {})",
            m, show(cv.0), cv.1, cv.2, show(rv.0), rv.1, rv.2
        );
        // and via the looping driver, which handles skippable frames
        let cv = decompress_loop(c(), &f, 7, 4096);
        let rv = decompress_loop(r(), &f, 7, 4096);
        assert_eq!(
            (cv.0, cv.1, cv.2),
            (rv.0, rv.1, rv.2),
            "loop magic={:#x}",
            m
        );
        assert_bytes_eq!(format!("loop magic={:#x} output", m), cv.3, rv.3);
    }
}

// ===========================================================================
// ERRORS.md rows 105, 106 — LZ4F_getFrameInfo
// ===========================================================================

#[test]
fn err_getFrameInfo_incomplete_and_already_started() {
    let mut rng = Rng::new(0xF00D_0007);
    let src = gen(&mut rng, 30_000, Shape::Text);
    let mut prefs = LZ4F_preferences_t::default();
    prefs.frameInfo.contentSize = src.len() as u64;
    prefs.frameInfo.dictID = 7;
    let frame = compress_frame(c(), &src, Some(&prefs));

    // (a) too few bytes to know the header size / to decode the header
    for take in 0..=25usize {
        let vals = both(|l| {
            let dctx = Dctx::new(l);
            let mut fi = LZ4F_frameInfo_t::default();
            let mut ss = take.min(frame.len());
            let rc = unsafe {
                l.get::<Fn_F_getFrameInfo>("LZ4F_getFrameInfo")(
                    dctx.p,
                    &mut fi,
                    frame.as_ptr() as *const c_void,
                    &mut ss,
                )
            };
            (rc, ss, fi)
        });
        assert_eq!(
            vals.0,
            vals.1,
            "getFrameInfo(take={}): C=({}, srcSize={}, {:?}) Rust=({}, srcSize={}, {:?})",
            take,
            show(vals.0 .0), vals.0 .1, vals.0 .2,
            show(vals.1 .0), vals.1 .1, vals.1 .2
        );
    }

    // (b) called while the dctx is mid-header (dstage_storeFrameHeader):
    // feed 5 bytes first (enough to know hSize but not to decode), then ask.
    let vals = both(|l| {
        let dctx = Dctx::new(l);
        let mut out = vec![0u8; 1 << 16];
        let mut ds = out.len();
        let mut ss = 5usize;
        let r1 = unsafe {
            l.get::<Fn_F_decompress>("LZ4F_decompress")(
                dctx.p,
                out.as_mut_ptr() as *mut c_void,
                &mut ds,
                frame.as_ptr() as *const c_void,
                &mut ss,
                ptr::null(),
            )
        };
        let mut fi = LZ4F_frameInfo_t::default();
        let mut ss2 = frame.len() - 5;
        let r2 = unsafe {
            l.get::<Fn_F_getFrameInfo>("LZ4F_getFrameInfo")(
                dctx.p,
                &mut fi,
                frame[5..].as_ptr() as *const c_void,
                &mut ss2,
            )
        };
        (r1, ss, ds, r2, ss2, fi)
    });
    assert_eq!(
        vals.0, vals.1,
        "getFrameInfo mid-header: C={:?} Rust={:?}",
        (show(vals.0 .0), vals.0 .1, vals.0 .2, show(vals.0 .3), vals.0 .4, vals.0 .5),
        (show(vals.1 .0), vals.1 .1, vals.1 .2, show(vals.1 .3), vals.1 .4, vals.1 .5)
    );

    // (c) the happy path must report identical frame info
    let vals = both(|l| {
        let dctx = Dctx::new(l);
        let mut fi = LZ4F_frameInfo_t::default();
        let mut ss = frame.len();
        let rc = unsafe {
            l.get::<Fn_F_getFrameInfo>("LZ4F_getFrameInfo")(
                dctx.p,
                &mut fi,
                frame.as_ptr() as *const c_void,
                &mut ss,
            )
        };
        (rc, ss, fi)
    });
    assert_eq!(vals.0, vals.1, "getFrameInfo happy path");
    assert_eq!(vals.0 .2.contentSize, src.len() as u64);
    assert_eq!(vals.0 .2.dictID, 7);
}

// ===========================================================================
// ERRORS.md row 109 — block size larger than the frame's declared maximum
// ===========================================================================

#[test]
fn err_decompress_block_size_too_large() {
    let mut rng = Rng::new(0xF00D_0008);
    let src = gen(&mut rng, 2000, Shape::Text);
    for &bsid in &[LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
        let mut prefs = LZ4F_preferences_t::default();
        prefs.frameInfo.blockSizeID = bsid;
        let frame = compress_frame(c(), &src, Some(&prefs));
        let hs = unsafe {
            c().get::<Fn_F_headerSize>("LZ4F_headerSize")(frame.as_ptr() as *const c_void, frame.len())
        };
        assert!(!is_error(hs));
        let max_bs = unsafe { c().get::<Fn_F_getBlockSize>("LZ4F_getBlockSize")(bsid) };
        for &bh in &[
            max_bs as u32,
            max_bs as u32 + 1,
            0x7FFF_FFFF,
            0xFFFF_FFFF,
            0x8000_0000 | (max_bs as u32 + 1), // uncompressed flag + oversize
            0x8000_0000 | max_bs as u32,
            0,
            0x8000_0000,
        ] {
            let mut f = frame.clone();
            f[hs..hs + 4].copy_from_slice(&bh.to_le_bytes());
            let cv = decompress_loop(c(), &f, 4096, 1 << 16);
            let rv = decompress_loop(r(), &f, 4096, 1 << 16);
            assert_eq!(
                (cv.0, cv.1, cv.2),
                (rv.0, rv.1, rv.2),
                "bsid={} blockHeader={:#x}: C=({}, out={}, in={}) Rust=({}, out={}, in={})",
                bsid, bh, show(cv.0), cv.1, cv.2, show(rv.0), rv.1, rv.2
            );
            assert_bytes_eq!(format!("bsid={} bh={:#x}", bsid, bh), cv.3, rv.3);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 110, 111, 115 — checksum corruption
// ===========================================================================

#[test]
fn err_decompress_block_checksum_invalid() {
    let mut rng = Rng::new(0xF00D_0009);
    let src = gen(&mut rng, 5000, Shape::Text);
    let mut prefs = LZ4F_preferences_t::default();
    prefs.frameInfo.blockChecksumFlag = LZ4F_BLOCK_CHECKSUM_ENABLED;
    prefs.frameInfo.blockSizeID = LZ4F_MAX64KB;
    let frame = compress_frame(c(), &src, Some(&prefs));

    let hs = unsafe {
        c().get::<Fn_F_headerSize>("LZ4F_headerSize")(frame.as_ptr() as *const c_void, frame.len())
    };
    let bsize =
        u32::from_le_bytes(frame[hs..hs + 4].try_into().unwrap()) & 0x7FFF_FFFF;
    let crc_off = hs + 4 + bsize as usize;
    assert!(crc_off + 4 <= frame.len());

    for byte in 0..4usize {
        for delta in [1u8, 0x80, 0xFF] {
            let mut f = frame.clone();
            f[crc_off + byte] = f[crc_off + byte].wrapping_add(delta);
            let cv = decompress_loop(c(), &f, 4096, 1 << 16);
            let rv = decompress_loop(r(), &f, 4096, 1 << 16);
            assert_eq!(
                (cv.0, cv.1, cv.2),
                (rv.0, rv.1, rv.2),
                "block crc byte {} +{}: C={} Rust={}",
                byte, delta, show(cv.0), show(rv.0)
            );
            assert_eq!(err_code(cv.0), 7, "expected blockChecksum_invalid");
            assert_bytes_eq!("block crc output", cv.3, rv.3);
        }
    }

    // the same, but with an UNCOMPRESSED (stored) block, which is a different
    // branch in the C (dstage_getBlockChecksum vs dstage_getCBlock)
    let inc = gen(&mut rng, 3000, Shape::Incompressible);
    let frame = compress_frame(c(), &inc, Some(&prefs));
    let hs = unsafe {
        c().get::<Fn_F_headerSize>("LZ4F_headerSize")(frame.as_ptr() as *const c_void, frame.len())
    };
    let raw = u32::from_le_bytes(frame[hs..hs + 4].try_into().unwrap());
    assert_ne!(raw & 0x8000_0000, 0, "expected a stored/uncompressed block");
    let bsize = raw & 0x7FFF_FFFF;
    let crc_off = hs + 4 + bsize as usize;
    for byte in 0..4usize {
        let mut f = frame.clone();
        f[crc_off + byte] ^= 0x55;
        let cv = decompress_loop(c(), &f, 4096, 1 << 16);
        let rv = decompress_loop(r(), &f, 4096, 1 << 16);
        assert_eq!(
            (cv.0, cv.1, cv.2),
            (rv.0, rv.1, rv.2),
            "stored-block crc byte {}: C={} Rust={}",
            byte, show(cv.0), show(rv.0)
        );
        assert_bytes_eq!("stored crc output", cv.3, rv.3);
    }
}

#[test]
fn err_decompress_content_checksum_invalid() {
    let mut rng = Rng::new(0xF00D_000A);
    for len in [0usize, 1, 100, 5000, 70_000] {
        let src = gen(&mut rng, len, Shape::Text);
        let mut prefs = LZ4F_preferences_t::default();
        prefs.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;
        let frame = compress_frame(c(), &src, Some(&prefs));
        let n = frame.len();
        for byte in 0..4usize {
            let mut f = frame.clone();
            f[n - 4 + byte] ^= 0xA5;
            let cv = decompress_loop(c(), &f, 4096, 1 << 16);
            let rv = decompress_loop(r(), &f, 4096, 1 << 16);
            assert_eq!(
                (cv.0, cv.1, cv.2),
                (rv.0, rv.1, rv.2),
                "content crc len={} byte={}: C={} Rust={}",
                len, byte, show(cv.0), show(rv.0)
            );
            assert_eq!(err_code(cv.0), 18, "expected contentChecksum_invalid");
            assert_bytes_eq!("content crc output", cv.3, rv.3);
        }
        // ... and with skipChecksums = 1 the corruption must be IGNORED,
        // identically in both libraries.
        let mut f = frame.clone();
        f[n - 1] ^= 0xFF;
        let dopts = LZ4F_decompressOptions_t {
            stableDst: 0,
            skipChecksums: 1,
            reserved1: 0,
            reserved0: 0,
        };
        let vals = both(|l| {
            let dctx = Dctx::new(l);
            let mut out = vec![0u8; len + 1024];
            let mut ds = out.len();
            let mut ss = f.len();
            let rc = unsafe {
                l.get::<Fn_F_decompress>("LZ4F_decompress")(
                    dctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    &mut ds,
                    f.as_ptr() as *const c_void,
                    &mut ss,
                    &dopts,
                )
            };
            out.truncate(ds);
            (rc, ds, ss, out)
        });
        assert_eq!(
            (vals.0 .0, vals.0 .1, vals.0 .2),
            (vals.1 .0, vals.1 .1, vals.1 .2),
            "skipChecksums len={}: C={} Rust={}",
            len,
            show(vals.0 .0),
            show(vals.1 .0)
        );
        assert_bytes_eq!(format!("skipChecksums out len={}", len), vals.0 .3, vals.1 .3);
    }
}

// ===========================================================================
// ERRORS.md rows 112, 113 — corrupt compressed block -> decompressionFailed
// ERRORS.md row 114 — declared contentSize != decoded size
// ===========================================================================

#[test]
fn err_decompress_corrupt_block_and_frame_size_wrong() {
    let mut rng = Rng::new(0xF00D_000B);
    let src = gen(&mut rng, 6000, Shape::Text);
    let mut prefs = LZ4F_preferences_t::default();
    prefs.frameInfo.contentSize = src.len() as u64;
    let frame = compress_frame(c(), &src, Some(&prefs));
    let hs = unsafe {
        c().get::<Fn_F_headerSize>("LZ4F_headerSize")(frame.as_ptr() as *const c_void, frame.len())
    };
    let bsize = u32::from_le_bytes(frame[hs..hs + 4].try_into().unwrap()) & 0x7FFF_FFFF;

    // (a) corrupt bytes inside the compressed block payload
    for i in 0..(bsize as usize).min(200) {
        let mut f = frame.clone();
        f[hs + 4 + i] ^= 0x5A;
        let cv = decompress_loop(c(), &f, 4096, 1 << 16);
        let rv = decompress_loop(r(), &f, 4096, 1 << 16);
        assert_eq!(
            (cv.0, cv.1, cv.2),
            (rv.0, rv.1, rv.2),
            "corrupt payload byte {}: C=({}, {}, {}) Rust=({}, {}, {})",
            i, show(cv.0), cv.1, cv.2, show(rv.0), rv.1, rv.2
        );
        assert_bytes_eq!(format!("corrupt payload {} output", i), cv.3, rv.3);
    }

    // (b) shrink the declared block size so the endMark arrives early ->
    // frameRemainingSize != 0 -> frameSize_wrong
    for shrink in [1u32, 2, 5, 17, 100] {
        if shrink >= bsize {
            continue;
        }
        let nb = bsize - shrink;
        let mut f = frame[..hs + 4 + nb as usize].to_vec();
        f.extend_from_slice(&0u32.to_le_bytes()); // endMark
        let cv = decompress_loop(c(), &f, 4096, 1 << 16);
        let rv = decompress_loop(r(), &f, 4096, 1 << 16);
        assert_eq!(
            (cv.0, cv.1, cv.2),
            (rv.0, rv.1, rv.2),
            "shrink={}: C={} Rust={}",
            shrink, show(cv.0), show(rv.0)
        );
        assert_bytes_eq!(format!("shrink={} output", shrink), cv.3, rv.3);
    }

    // (c) an empty frame that declares a non-zero contentSize
    for declared in [1u64, 5, 100, u64::MAX] {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.contentSize = declared;
        // build header + endMark manually via compressBegin / compressEnd on
        // an empty input, then patch the contentSize field.  Simpler: compress
        // real data of the declared length is impractical for u64::MAX, so
        // patch the field directly.
        let empty = compress_frame(c(), &[], None);
        let hs0 = unsafe {
            c().get::<Fn_F_headerSize>("LZ4F_headerSize")(
                empty.as_ptr() as *const c_void,
                empty.len(),
            )
        };
        assert!(!is_error(hs0));
        // Only patchable if the header actually carries a contentSize field,
        // which the default prefs do not, so build one that does.
        let mut p2 = LZ4F_preferences_t::default();
        p2.frameInfo.contentSize = 8; // a real 8-byte frame
        let real = compress_frame(c(), &[1u8; 8], Some(&p2));
        let hsr = unsafe {
            c().get::<Fn_F_headerSize>("LZ4F_headerSize")(
                real.as_ptr() as *const c_void,
                real.len(),
            )
        };
        let mut f = real.clone();
        // contentSize sits right after magic(4) + FLG(1) + BD(1)
        f[6..14].copy_from_slice(&declared.to_le_bytes());
        // recompute the header checksum
        let xxh32 = unsafe { c().get::<Fn_XXH32>("LZ4_XXH32") };
        let body_len = hsr - 5;
        let hc =
            ((unsafe { xxh32(f[4..].as_ptr() as *const c_void, body_len, 0) } >> 8) & 0xFF) as u8;
        f[hsr - 1] = hc;
        let cv = decompress_loop(c(), &f, 4096, 1 << 16);
        let rv = decompress_loop(r(), &f, 4096, 1 << 16);
        assert_eq!(
            (cv.0, cv.1, cv.2),
            (rv.0, rv.1, rv.2),
            "declared contentSize={} vs 8 actual: C={} Rust={}",
            declared, show(cv.0), show(rv.0)
        );
        assert_bytes_eq!(format!("declared={} output", declared), cv.3, rv.3);
        let _ = hs0;
    }
}

// ===========================================================================
// ERRORS.md row 144 — resetDecompressionContext after an error
// ===========================================================================

#[test]
fn err_resetDecompressionContext_recovers() {
    let mut rng = Rng::new(0xF00D_000C);
    let src = gen(&mut rng, 4000, Shape::Text);
    let mut prefs = LZ4F_preferences_t::default();
    prefs.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;
    let good = compress_frame(c(), &src, Some(&prefs));
    let mut bad = good.clone();
    let n = bad.len();
    bad[n - 2] ^= 0x33;

    let vals = both(|l| {
        let dctx = Dctx::new(l);
        let mut out = vec![0u8; src.len() + 4096];

        // 1) fail on the corrupt frame
        let mut ds = out.len();
        let mut ss = bad.len();
        let e1 = unsafe {
            l.get::<Fn_F_decompress>("LZ4F_decompress")(
                dctx.p,
                out.as_mut_ptr() as *mut c_void,
                &mut ds,
                bad.as_ptr() as *const c_void,
                &mut ss,
                ptr::null(),
            )
        };
        // 2) reset
        dctx.reset();
        // 3) the same dctx must now decode the good frame correctly
        let mut ds2 = out.len();
        let mut ss2 = good.len();
        let e2 = unsafe {
            l.get::<Fn_F_decompress>("LZ4F_decompress")(
                dctx.p,
                out.as_mut_ptr() as *mut c_void,
                &mut ds2,
                good.as_ptr() as *const c_void,
                &mut ss2,
                ptr::null(),
            )
        };
        (e1, e2, ds2, ss2, out[..ds2].to_vec())
    });
    assert_eq!(
        (vals.0 .0, vals.0 .1, vals.0 .2, vals.0 .3),
        (vals.1 .0, vals.1 .1, vals.1 .2, vals.1 .3),
        "reset recovery: C=({}, {}, {}, {}) Rust=({}, {}, {}, {})",
        show(vals.0 .0), show(vals.0 .1), vals.0 .2, vals.0 .3,
        show(vals.1 .0), show(vals.1 .1), vals.1 .2, vals.1 .3
    );
    assert_bytes_eq!("reset recovery output", vals.0 .4, vals.1 .4);
    assert_bytes_eq!("reset recovery == src", vals.0 .4, src);

    // resetDecompressionContext on a fresh, never-used dctx must also be safe
    let vals = both(|l| {
        let dctx = Dctx::new(l);
        dctx.reset();
        dctx.reset();
        let mut out = vec![0u8; src.len() + 4096];
        let mut ds = out.len();
        let mut ss = good.len();
        let rc = unsafe {
            l.get::<Fn_F_decompress>("LZ4F_decompress")(
                dctx.p,
                out.as_mut_ptr() as *mut c_void,
                &mut ds,
                good.as_ptr() as *const c_void,
                &mut ss,
                ptr::null(),
            )
        };
        (rc, ds, ss)
    });
    assert_eq!(vals.0, vals.1, "double reset on a fresh dctx");
}

// ===========================================================================
// ERRORS.md row 73 — LZ4F_compressFrame dst too small
// ===========================================================================

#[test]
fn err_compressFrame_dst_too_small() {
    let mut rng = Rng::new(0xF00D_000D);
    for &len in &[0usize, 1, 100, 5000, 70_000] {
        let src = gen(&mut rng, len, Shape::Text);
        for &bsid in &[LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX4MB] {
            let mut prefs = LZ4F_preferences_t::default();
            prefs.frameInfo.blockSizeID = bsid;
            let bound = unsafe {
                c().get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(len, &prefs)
            };
            let rbound = unsafe {
                r().get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(len, &prefs)
            };
            assert_eq!(bound, rbound, "compressFrameBound({}, bsid={})", len, bsid);

            for &cap in &[
                0usize,
                1,
                7,
                19,
                bound / 4,
                bound / 2,
                bound.saturating_sub(1),
                bound,
                bound + 1,
            ] {
                let vals = both(|l| {
                    let mut out = vec![0u8; bound + 64];
                    let n = unsafe {
                        l.get::<Fn_F_compressFrame>("LZ4F_compressFrame")(
                            out.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            src.len(),
                            &prefs,
                        )
                    };
                    (n, if is_error(n) { Vec::new() } else { out[..n].to_vec() })
                });
                assert_eq!(
                    vals.0 .0,
                    vals.1 .0,
                    "compressFrame(len={}, bsid={}, cap={}, bound={}): C={} Rust={}",
                    len, bsid, cap, bound,
                    show(vals.0 .0),
                    show(vals.1 .0)
                );
                assert_bytes_eq!(
                    format!("compressFrame len={} cap={}", len, cap),
                    vals.0 .1,
                    vals.1 .1
                );
                if cap < bound {
                    assert!(is_error(vals.0 .0), "cap={} < bound={} should error", cap, bound);
                    assert_eq!(err_code(vals.0 .0), 11);
                }
            }
        }
    }
}

/// `LZ4F_compressFrame_usingCDict` with a NULL cctx (the C allocates one) and
/// a NULL cdict, plus a too-small dst.
#[test]
fn err_compressFrame_usingCDict_variants() {
    let mut rng = Rng::new(0xF00D_000E);
    let src = gen(&mut rng, 3000, Shape::Text);
    let dict = gen(&mut rng, 4096, Shape::Text);
    for &level in &[0i32, 3, 9, 12] {
        let mut prefs = LZ4F_preferences_t::default();
        prefs.compressionLevel = level;
        let bound = unsafe {
            c().get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(src.len(), &prefs)
        };
        for &use_cdict in &[false, true] {
            for &cap in &[0usize, 1, 19, bound / 2, bound.saturating_sub(1), bound] {
                let vals = both(|l| {
                    let cd = if use_cdict {
                        unsafe {
                            l.get::<Fn_F_createCDict>("LZ4F_createCDict")(
                                dict.as_ptr() as *const c_void,
                                dict.len(),
                            )
                        }
                    } else {
                        ptr::null_mut()
                    };
                    // @cctx MUST be non-NULL: LZ4F_compressFrame_usingCDict
                    // forwards it straight to LZ4F_compressBegin_usingCDict,
                    // which dereferences it (only LZ4F_compressFrame allocates
                    // a context on the caller's behalf).
                    let cctx = Cctx::new(l);
                    let mut out = vec![0u8; bound + 64];
                    let n = unsafe {
                        l.get::<Fn_F_compressFrame_usingCDict>("LZ4F_compressFrame_usingCDict")(
                            cctx.p,
                            out.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            src.len(),
                            cd,
                            &prefs,
                        )
                    };
                    if !cd.is_null() {
                        unsafe { l.get::<Fn_F_freeCDict>("LZ4F_freeCDict")(cd) };
                    }
                    (n, if is_error(n) { Vec::new() } else { out[..n].to_vec() })
                });
                assert_eq!(
                    vals.0 .0,
                    vals.1 .0,
                    "compressFrame_usingCDict(level={}, cdict={}, cap={}): C={} Rust={}",
                    level, use_cdict, cap,
                    show(vals.0 .0),
                    show(vals.1 .0)
                );
                assert_bytes_eq!(
                    format!("usingCDict level={} cdict={} cap={}", level, use_cdict, cap),
                    vals.0 .1,
                    vals.1 .1
                );
            }
        }
    }
}

/// `LZ4F_createCDict` with a NULL / zero-size dictionary and an oversized one
/// (>64 KB is truncated to the last 64 KB — ERRORS.md row 152).
#[test]
fn err_createCDict_edge_sizes() {
    let mut rng = Rng::new(0xF00D_000F);
    let big = gen(&mut rng, 200_000, Shape::Text);
    let src = gen(&mut rng, 2000, Shape::Text);
    for &ds in &[0usize, 1, 4, 8, 64, 65535, 65536, 65537, 100_000, 200_000] {
        let vals = both(|l| {
            let cd = unsafe {
                l.get::<Fn_F_createCDict>("LZ4F_createCDict")(
                    big.as_ptr() as *const c_void,
                    ds,
                )
            };
            let null = cd.is_null();
            let cctx = Cctx::new(l);
            let mut out = vec![0u8; 300_000];
            let n = if null {
                usize::MAX // marker; not used
            } else {
                unsafe {
                    l.get::<Fn_F_compressFrame_usingCDict>("LZ4F_compressFrame_usingCDict")(
                        cctx.p,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        src.as_ptr() as *const c_void,
                        src.len(),
                        cd,
                        ptr::null(),
                    )
                }
            };
            if !cd.is_null() {
                unsafe { l.get::<Fn_F_freeCDict>("LZ4F_freeCDict")(cd) };
            }
            (null, n, if null || is_error(n) { Vec::new() } else { out[..n].to_vec() })
        });
        assert_eq!(
            (vals.0 .0, vals.0 .1),
            (vals.1 .0, vals.1 .1),
            "createCDict(dictSize={}): C=(null={}, {}) Rust=(null={}, {})",
            ds, vals.0 .0, show(vals.0 .1), vals.1 .0, show(vals.1 .1)
        );
        assert_bytes_eq!(format!("cdict dictSize={} output", ds), vals.0 .2, vals.1 .2);
    }

    // NULL dictionary pointer.  Only dictSize == 0 is a defined input:
    // LZ4F_createCDict_advanced unconditionally `memcpy`s @dictSize bytes from
    // @dictBuffer, so (NULL, non-zero) is undefined behaviour in the C rather
    // than a rejection it defines.
    for &ds in &[0usize] {
        let vals = both(|l| {
            let cd = unsafe {
                l.get::<Fn_F_createCDict>("LZ4F_createCDict")(ptr::null(), ds)
            };
            let null = cd.is_null();
            if !cd.is_null() {
                unsafe { l.get::<Fn_F_freeCDict>("LZ4F_freeCDict")(cd) };
            }
            null
        });
        assert_eq!(vals.0, vals.1, "createCDict(NULL, {}) nullness", ds);
    }

    // freeCDict(NULL) must be a no-op in both
    both(|l| unsafe { l.get::<Fn_F_freeCDict>("LZ4F_freeCDict")(ptr::null_mut()) });
}

// ===========================================================================
// ERRORS.md row 156 + generic boundaries: out-of-range enum values in prefs
// ===========================================================================

/// C enums accept any `int`.  Feed values with no valid variant through every
/// enum field of `LZ4F_preferences_t` and assert both libraries agree — either
/// both reject with the same code or both silently produce the same bytes.
#[test]
fn err_out_of_range_enum_values_in_preferences() {
    let mut rng = Rng::new(0xF00D_0010);
    let src = gen(&mut rng, 1500, Shape::Text);
    let odd: &[c_int] = &[-1, 2, 3, 8, 9, 99, 255, 256, i32::MAX, i32::MIN];

    for &v in odd {
        // one field at a time
        let variants: Vec<(&str, LZ4F_preferences_t)> = vec![
            ("blockSizeID", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockSizeID = v;
                p
            }),
            ("blockMode", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockMode = v;
                p
            }),
            ("contentChecksumFlag", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.contentChecksumFlag = v;
                p
            }),
            ("frameType", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.frameType = v;
                p
            }),
            ("blockChecksumFlag", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockChecksumFlag = v;
                p
            }),
            ("compressionLevel", {
                let mut p = LZ4F_preferences_t::default();
                p.compressionLevel = v;
                p
            }),
            ("autoFlush", {
                let mut p = LZ4F_preferences_t::default();
                p.autoFlush = v as c_uint;
                p
            }),
            ("favorDecSpeed", {
                let mut p = LZ4F_preferences_t::default();
                p.favorDecSpeed = v as c_uint;
                p
            }),
        ];
        for (field, prefs) in variants {
            // compressFrameBound may itself reject
            let (cb, rb) = both(|l| unsafe {
                l.get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(src.len(), &prefs)
            });
            assert_eq!(cb, rb, "compressFrameBound {}={}", field, v);

            let (cbb, rbb) = both(|l| unsafe {
                l.get::<Fn_F_compressBound>("LZ4F_compressBound")(src.len(), &prefs)
            });
            assert_eq!(cbb, rbb, "compressBound {}={}", field, v);

            // An out-of-range blockSizeID can make the bound enormous (it is
            // used as a multiplier).  Comparing the bounds is the meaningful
            // check there; do not try to allocate gigabytes.
            const MAX_ALLOC: usize = 1 << 23;
            if !is_error(cb) && cb > MAX_ALLOC {
                continue;
            }
            let cap = if is_error(cb) { 1 << 22 } else { cb + 64 };
            let vals = both(|l| {
                let mut out = vec![0u8; cap];
                let n = unsafe {
                    l.get::<Fn_F_compressFrame>("LZ4F_compressFrame")(
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        src.as_ptr() as *const c_void,
                        src.len(),
                        &prefs,
                    )
                };
                (n, if is_error(n) { Vec::new() } else { out[..n].to_vec() })
            });
            assert_eq!(
                vals.0 .0,
                vals.1 .0,
                "compressFrame {}={}: C={} Rust={}",
                field, v,
                show(vals.0 .0),
                show(vals.1 .0)
            );
            assert_bytes_eq!(format!("compressFrame {}={}", field, v), vals.0 .1, vals.1 .1);

            // and drive the streaming path with the same prefs
            let vals = both(|l| {
                let cctx = Cctx::new(l);
                let mut out = vec![0u8; 1 << 22];
                unsafe {
                    let h = l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                        cctx.p,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        &prefs,
                    );
                    if is_error(h) {
                        return (h, 0usize, 0usize, Vec::new());
                    }
                    let u = l.get::<Fn_F_compressUpdate>("LZ4F_compressUpdate")(
                        cctx.p,
                        out.as_mut_ptr().add(h) as *mut c_void,
                        out.len() - h,
                        src.as_ptr() as *const c_void,
                        src.len(),
                        ptr::null(),
                    );
                    if is_error(u) {
                        return (h, u, 0usize, Vec::new());
                    }
                    let e = l.get::<Fn_F_compressEnd>("LZ4F_compressEnd")(
                        cctx.p,
                        out.as_mut_ptr().add(h + u) as *mut c_void,
                        out.len() - h - u,
                        ptr::null(),
                    );
                    if is_error(e) {
                        return (h, u, e, Vec::new());
                    }
                    (h, u, e, out[..h + u + e].to_vec())
                }
            });
            assert_eq!(
                (vals.0 .0, vals.0 .1, vals.0 .2),
                (vals.1 .0, vals.1 .1, vals.1 .2),
                "streaming {}={}: C=({}, {}, {}) Rust=({}, {}, {})",
                field, v,
                show(vals.0 .0), show(vals.0 .1), show(vals.0 .2),
                show(vals.1 .0), show(vals.1 .1), show(vals.1 .2)
            );
            assert_bytes_eq!(format!("streaming {}={}", field, v), vals.0 .3, vals.1 .3);
        }
    }
}

/// Non-zero `reserved` fields in `LZ4F_preferences_t` /
/// `LZ4F_compressOptions_t` / `LZ4F_decompressOptions_t`.  The C ignores some
/// and may reject others — whatever it does, the Rust must match.
#[test]
fn err_nonzero_reserved_fields() {
    let mut rng = Rng::new(0xF00D_0011);
    let src = gen(&mut rng, 1200, Shape::Text);
    for &fill in &[1u32, 0xFFFF_FFFF, 0x5A5A_5A5A] {
        let mut prefs = LZ4F_preferences_t::default();
        prefs.reserved = [fill, fill, fill];
        let vals = both(|l| {
            let mut out = vec![0u8; 1 << 20];
            let n = unsafe {
                l.get::<Fn_F_compressFrame>("LZ4F_compressFrame")(
                    out.as_mut_ptr() as *mut c_void,
                    out.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                    &prefs,
                )
            };
            (n, if is_error(n) { Vec::new() } else { out[..n].to_vec() })
        });
        assert_eq!(vals.0 .0, vals.1 .0, "prefs.reserved={:#x}", fill);
        assert_bytes_eq!(format!("prefs.reserved={:#x}", fill), vals.0 .1, vals.1 .1);

        let copts = LZ4F_compressOptions_t {
            stableSrc: 0,
            reserved: [fill, fill, fill],
        };
        let vals = both(|l| {
            let cctx = Cctx::new(l);
            let mut out = vec![0u8; 1 << 20];
            unsafe {
                let h = l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                    cctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    out.len(),
                    ptr::null(),
                );
                let u = l.get::<Fn_F_compressUpdate>("LZ4F_compressUpdate")(
                    cctx.p,
                    out.as_mut_ptr().add(h) as *mut c_void,
                    out.len() - h,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    &copts,
                );
                let e = l.get::<Fn_F_compressEnd>("LZ4F_compressEnd")(
                    cctx.p,
                    out.as_mut_ptr().add(h + u) as *mut c_void,
                    out.len() - h - u,
                    &copts,
                );
                (h, u, e, out[..h + u + e].to_vec())
            }
        });
        assert_eq!(
            (vals.0 .0, vals.0 .1, vals.0 .2),
            (vals.1 .0, vals.1 .1, vals.1 .2),
            "copts.reserved={:#x}",
            fill
        );
        assert_bytes_eq!(format!("copts.reserved={:#x}", fill), vals.0 .3, vals.1 .3);

        let frame = compress_frame(c(), &src, None);
        let dopts = LZ4F_decompressOptions_t {
            stableDst: 0,
            skipChecksums: 0,
            reserved1: fill,
            reserved0: fill,
        };
        let vals = both(|l| {
            let dctx = Dctx::new(l);
            let mut out = vec![0u8; src.len() + 4096];
            let mut ds = out.len();
            let mut ss = frame.len();
            let rc = unsafe {
                l.get::<Fn_F_decompress>("LZ4F_decompress")(
                    dctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    &mut ds,
                    frame.as_ptr() as *const c_void,
                    &mut ss,
                    &dopts,
                )
            };
            (rc, ds, ss, out[..ds].to_vec())
        });
        assert_eq!(
            (vals.0 .0, vals.0 .1, vals.0 .2),
            (vals.1 .0, vals.1 .1, vals.1 .2),
            "dopts.reserved={:#x}",
            fill
        );
        assert_bytes_eq!(format!("dopts.reserved={:#x}", fill), vals.0 .3, vals.1 .3);
    }
}

// ===========================================================================
// Generic boundaries: truncation and corruption fuzzing of whole frames
// ===========================================================================

/// Truncate a frame at EVERY possible length and assert both libraries return
/// the same code, consume the same amount, and emit the same bytes.
#[test]
fn err_frame_truncation_at_every_length() {
    let mut rng = Rng::new(0xF00D_0012);
    let configs: Vec<LZ4F_preferences_t> = {
        let mut v = Vec::new();
        for &(bsid, bmode, ccs, bcs, cs, did) in &[
            (LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 0, 0u64, 0u32),
            (LZ4F_MAX64KB, LZ4F_BLOCK_INDEPENDENT, 1, 1, 0, 0),
            (LZ4F_MAX256KB, LZ4F_BLOCK_LINKED, 1, 0, 3000, 99),
            (LZ4F_MAX4MB, LZ4F_BLOCK_INDEPENDENT, 0, 1, 0, 0),
        ] {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockSizeID = bsid;
            p.frameInfo.blockMode = bmode;
            p.frameInfo.contentChecksumFlag = ccs;
            p.frameInfo.blockChecksumFlag = bcs;
            p.frameInfo.contentSize = cs;
            p.frameInfo.dictID = did;
            v.push(p);
        }
        v
    };
    let src = gen(&mut rng, 3000, Shape::Text);
    for (i, prefs) in configs.iter().enumerate() {
        let mut p = *prefs;
        if p.frameInfo.contentSize != 0 {
            p.frameInfo.contentSize = src.len() as u64;
        }
        let frame = compress_frame(c(), &src, Some(&p));
        for take in 0..=frame.len() {
            let f = &frame[..take];
            let cv = decompress_loop(c(), f, 4096, 1 << 16);
            let rv = decompress_loop(r(), f, 4096, 1 << 16);
            assert_eq!(
                (cv.0, cv.1, cv.2),
                (rv.0, rv.1, rv.2),
                "cfg={} truncate at {}/{}: C=({}, out={}, in={}) Rust=({}, out={}, in={})",
                i, take, frame.len(),
                show(cv.0), cv.1, cv.2, show(rv.0), rv.1, rv.2
            );
            assert_bytes_eq!(format!("cfg={} truncate {} output", i, take), cv.3, rv.3);
        }
    }
}

/// Random bit/byte corruption anywhere in a frame, plus purely random input.
/// The exact error code AND the amount consumed/produced must match.
#[test]
fn err_frame_random_corruption_fuzz() {
    let mut rng = Rng::new(0xF00D_0013);
    let mut prefs_list = Vec::new();
    for &(bsid, bmode, ccs, bcs) in &[
        (LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 0),
        (LZ4F_MAX64KB, LZ4F_BLOCK_INDEPENDENT, 1, 1),
        (LZ4F_MAX256KB, LZ4F_BLOCK_LINKED, 1, 1),
    ] {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = bsid;
        p.frameInfo.blockMode = bmode;
        p.frameInfo.contentChecksumFlag = ccs;
        p.frameInfo.blockChecksumFlag = bcs;
        prefs_list.push(p);
    }

    for iter in 0..3000 {
        let prefs = prefs_list[rng.below(prefs_list.len())];
        let n = rng.range(1, 8000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, n, shape);
        let mut frame = compress_frame(c(), &src, Some(&prefs));

        // apply 1..=4 random corruptions
        let nmut = rng.range(1, 4);
        for _ in 0..nmut {
            let i = rng.below(frame.len());
            match rng.below(3) {
                0 => frame[i] ^= 1 << rng.below(8),
                1 => frame[i] = rng.byte(),
                _ => frame[i] = !frame[i],
            }
        }
        let chunk = [1usize, 7, 64, 4096, usize::MAX][rng.below(5)].min(frame.len().max(1));
        let out_chunk = [1usize, 13, 1024, 1 << 16][rng.below(4)];

        let cv = decompress_loop(c(), &frame, chunk, out_chunk);
        let rv = decompress_loop(r(), &frame, chunk, out_chunk);
        assert_eq!(
            (cv.0, cv.1, cv.2),
            (rv.0, rv.1, rv.2),
            "iter={} srclen={} chunk={} out_chunk={}: C=({}, out={}, in={}) Rust=({}, out={}, in={})",
            iter, n, chunk, out_chunk,
            show(cv.0), cv.1, cv.2, show(rv.0), rv.1, rv.2
        );
        assert_bytes_eq!(format!("iter={} output", iter), cv.3, rv.3);
    }

    // pure garbage "frames"
    for iter in 0..2000 {
        let n = rng.range(0, 200);
        let mut junk = gen(&mut rng, n, Shape::Incompressible);
        // half the time, give it a valid magic so it gets past the first gate
        if rng.bool() && n >= 4 {
            junk[..4].copy_from_slice(&LZ4F_MAGICNUMBER.to_le_bytes());
        }
        let cv = decompress_loop(c(), &junk, 64, 4096);
        let rv = decompress_loop(r(), &junk, 64, 4096);
        assert_eq!(
            (cv.0, cv.1, cv.2),
            (rv.0, rv.1, rv.2),
            "junk iter={} len={}: C=({}, {}, {}) Rust=({}, {}, {})",
            iter, n, show(cv.0), cv.1, cv.2, show(rv.0), rv.1, rv.2
        );
        assert_bytes_eq!(format!("junk iter={}", iter), cv.3, rv.3);
    }
}

/// `LZ4F_decompress` with a zero-size output buffer, a zero-size input, and
/// NULL option pointers — all legal calls whose behaviour must match.
#[test]
fn err_decompress_zero_sizes() {
    let mut rng = Rng::new(0xF00D_0014);
    let src = gen(&mut rng, 2000, Shape::Text);
    let frame = compress_frame(c(), &src, None);
    for &(ds, ss) in &[
        (0usize, 0usize),
        (0, 1),
        (0, frame.len()),
        (1, 0),
        (frame.len(), 0),
        (1, 1),
    ] {
        let vals = both(|l| {
            let dctx = Dctx::new(l);
            let mut out = vec![0u8; 4096];
            let mut dsz = ds;
            let mut ssz = ss;
            let rc = unsafe {
                l.get::<Fn_F_decompress>("LZ4F_decompress")(
                    dctx.p,
                    out.as_mut_ptr() as *mut c_void,
                    &mut dsz,
                    frame.as_ptr() as *const c_void,
                    &mut ssz,
                    ptr::null(),
                )
            };
            (rc, dsz, ssz)
        });
        assert_eq!(
            vals.0, vals.1,
            "decompress(ds={}, ss={}): C=({}, {}, {}) Rust=({}, {}, {})",
            ds, ss,
            show(vals.0 .0), vals.0 .1, vals.0 .2,
            show(vals.1 .0), vals.1 .1, vals.1 .2
        );
    }
}

/// `LZ4F_decompress_usingDict` error paths: NULL dict, zero dictSize, and the
/// WRONG dictionary (which must fail identically).
#[test]
fn err_decompress_usingDict_wrong_dict() {
    let mut rng = Rng::new(0xF00D_0015);
    let dict = gen(&mut rng, 8192, Shape::Text);
    let wrong = gen(&mut rng, 8192, Shape::Incompressible);
    let src = gen(&mut rng, 5000, Shape::Text);

    // build a frame that USES the dictionary
    let frame = {
        let cctx = Cctx::new(c());
        let mut out = vec![0u8; 1 << 20];
        unsafe {
            let h = c().get::<Fn_F_compressBegin_usingDict>("LZ4F_compressBegin_usingDict")(
                cctx.p,
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                dict.as_ptr() as *const c_void,
                dict.len(),
                ptr::null(),
            );
            assert!(!is_error(h));
            let u = c().get::<Fn_F_compressUpdate>("LZ4F_compressUpdate")(
                cctx.p,
                out.as_mut_ptr().add(h) as *mut c_void,
                out.len() - h,
                src.as_ptr() as *const c_void,
                src.len(),
                ptr::null(),
            );
            assert!(!is_error(u));
            let e = c().get::<Fn_F_compressEnd>("LZ4F_compressEnd")(
                cctx.p,
                out.as_mut_ptr().add(h + u) as *mut c_void,
                out.len() - h - u,
                ptr::null(),
            );
            assert!(!is_error(e));
            out.truncate(h + u + e);
        }
        out
    };

    let dict_cases: Vec<(&str, *const c_void, usize)> = vec![
        ("correct", dict.as_ptr() as *const c_void, dict.len()),
        ("wrong", wrong.as_ptr() as *const c_void, wrong.len()),
        ("truncated", dict.as_ptr() as *const c_void, dict.len() / 2),
        ("null-0", ptr::null(), 0),
        ("ptr-0", dict.as_ptr() as *const c_void, 0),
    ];
    for (label, dp, dsz) in dict_cases {
        let vals = both(|l| {
            let dctx = Dctx::new(l);
            let mut out = vec![0u8; src.len() + 8192];
            let mut consumed = 0usize;
            let mut produced = 0usize;
            let mut rc = 1usize;
            while consumed < frame.len() && rc != 0 {
                let mut ds = out.len() - produced;
                let mut ss = frame.len() - consumed;
                rc = unsafe {
                    l.get::<Fn_F_decompress_usingDict>("LZ4F_decompress_usingDict")(
                        dctx.p,
                        out.as_mut_ptr().add(produced) as *mut c_void,
                        &mut ds,
                        frame[consumed..].as_ptr() as *const c_void,
                        &mut ss,
                        dp,
                        dsz,
                        ptr::null(),
                    )
                };
                if is_error(rc) {
                    break;
                }
                produced += ds;
                consumed += ss;
                if ds == 0 && ss == 0 {
                    break;
                }
            }
            (rc, produced, consumed, out[..produced].to_vec())
        });
        assert_eq!(
            (vals.0 .0, vals.0 .1, vals.0 .2),
            (vals.1 .0, vals.1 .1, vals.1 .2),
            "decompress_usingDict({}): C=({}, out={}, in={}) Rust=({}, out={}, in={})",
            label,
            show(vals.0 .0), vals.0 .1, vals.0 .2,
            show(vals.1 .0), vals.1 .1, vals.1 .2
        );
        assert_bytes_eq!(format!("usingDict {} output", label), vals.0 .3, vals.1 .3);
        if label == "correct" {
            assert_bytes_eq!("usingDict correct == src", vals.0 .3, src);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 74, 75, 91 — *_advanced with a custom allocator that FAILS.
// This is the only way to reach the allocation_failed branches from outside.
// ===========================================================================

static mut FAIL_AFTER: c_int = 0;

unsafe extern "C" fn failing_alloc(_o: *mut c_void, size: usize) -> *mut c_void {
    if FAIL_AFTER <= 0 {
        return ptr::null_mut();
    }
    FAIL_AFTER -= 1;
    libc_malloc(size)
}
unsafe extern "C" fn failing_calloc(_o: *mut c_void, size: usize) -> *mut c_void {
    if FAIL_AFTER <= 0 {
        return ptr::null_mut();
    }
    FAIL_AFTER -= 1;
    let p = libc_malloc(size);
    if !p.is_null() {
        ptr::write_bytes(p as *mut u8, 0, size);
    }
    p
}
unsafe extern "C" fn tracking_free(_o: *mut c_void, p: *mut c_void) {
    if !p.is_null() {
        libc_free(p);
    }
}

// Use the process's libc allocator directly so the C and Rust libraries share
// the same heap semantics.
extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

#[test]
fn err_advanced_allocation_failure() {
    let cm = CustomMem {
        customAlloc: Some(failing_alloc),
        customCalloc: Some(failing_calloc),
        customFree: Some(tracking_free),
        opaqueState: ptr::null_mut(),
    };

    // Budget 0 -> the very first allocation fails.
    for budget in 0..4i32 {
        for name in [
            "LZ4F_createCompressionContext_advanced",
            "LZ4F_createDecompressionContext_advanced",
        ] {
            let cnull = unsafe {
                FAIL_AFTER = budget;
                let p = c().get::<Fn_F_createCompressionContext_advanced>(name)(cm, LZ4F_VERSION);
                let n = p.is_null();
                if !p.is_null() {
                    if name.contains("Decompression") {
                        c().get::<Fn_F_freeDecompressionContext>("LZ4F_freeDecompressionContext")(p);
                    } else {
                        c().get::<Fn_F_freeCompressionContext>("LZ4F_freeCompressionContext")(p);
                    }
                }
                n
            };
            let rnull = unsafe {
                FAIL_AFTER = budget;
                let p = r().get::<Fn_F_createCompressionContext_advanced>(name)(cm, LZ4F_VERSION);
                let n = p.is_null();
                if !p.is_null() {
                    if name.contains("Decompression") {
                        r().get::<Fn_F_freeDecompressionContext>("LZ4F_freeDecompressionContext")(p);
                    } else {
                        r().get::<Fn_F_freeCompressionContext>("LZ4F_freeCompressionContext")(p);
                    }
                }
                n
            };
            assert_eq!(cnull, rnull, "{} with alloc budget {}", name, budget);
        }

        // createCDict_advanced
        let dict = [9u8; 4096];
        let cnull = unsafe {
            FAIL_AFTER = budget;
            let p = c().get::<Fn_F_createCDict_advanced>("LZ4F_createCDict_advanced")(
                cm,
                dict.as_ptr() as *const c_void,
                dict.len(),
            );
            let n = p.is_null();
            if !p.is_null() {
                c().get::<Fn_F_freeCDict>("LZ4F_freeCDict")(p);
            }
            n
        };
        let rnull = unsafe {
            FAIL_AFTER = budget;
            let p = r().get::<Fn_F_createCDict_advanced>("LZ4F_createCDict_advanced")(
                cm,
                dict.as_ptr() as *const c_void,
                dict.len(),
            );
            let n = p.is_null();
            if !p.is_null() {
                r().get::<Fn_F_freeCDict>("LZ4F_freeCDict")(p);
            }
            n
        };
        assert_eq!(cnull, rnull, "createCDict_advanced with alloc budget {}", budget);
    }

    // A generous budget must succeed in both, and the resulting cctx must
    // produce identical output.
    let mut rng = Rng::new(0xF00D_0016);
    let src = gen(&mut rng, 3000, Shape::Text);
    let vals = both(|l| unsafe {
        FAIL_AFTER = 1000;
        let cctx = l.get::<Fn_F_createCompressionContext_advanced>(
            "LZ4F_createCompressionContext_advanced",
        )(cm, LZ4F_VERSION);
        assert!(!cctx.is_null());
        let mut out = vec![0u8; 1 << 20];
        let h = l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
            cctx,
            out.as_mut_ptr() as *mut c_void,
            out.len(),
            ptr::null(),
        );
        let u = l.get::<Fn_F_compressUpdate>("LZ4F_compressUpdate")(
            cctx,
            out.as_mut_ptr().add(h) as *mut c_void,
            out.len() - h,
            src.as_ptr() as *const c_void,
            src.len(),
            ptr::null(),
        );
        let e = l.get::<Fn_F_compressEnd>("LZ4F_compressEnd")(
            cctx,
            out.as_mut_ptr().add(h + u) as *mut c_void,
            out.len() - h - u,
            ptr::null(),
        );
        l.get::<Fn_F_freeCompressionContext>("LZ4F_freeCompressionContext")(cctx);
        (h, u, e, out[..h + u + e].to_vec())
    });
    assert_eq!(
        (vals.0 .0, vals.0 .1, vals.0 .2),
        (vals.1 .0, vals.1 .1, vals.1 .2),
        "advanced cctx compression"
    );
    assert_bytes_eq!("advanced cctx bytes", vals.0 .3, vals.1 .3);
}
