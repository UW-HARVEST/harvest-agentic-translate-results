//! Regression test for a REAL bug in the C (`lz4frame.c`) that the Rust
//! translation must reproduce exactly.
//!
//! `LZ4F_compressUpdateImpl` validates `dstCapacity` **before** it calls
//! `LZ4F_flush()` on a `blockCompressMode` switch, then advances `dstPtr` by the
//! flushed byte count **without deducting it from the remaining budget**:
//!
//! ```c
//! if (dstCapacity < LZ4F_compressBound_internal(srcSize, &prefs, tmpInSize))
//!     RETURN_ERROR(dstMaxSize_tooSmall);                 /* lz4frame.c:1006 */
//! if (blockCompression == LZ4B_UNCOMPRESSED && dstCapacity < srcSize)
//!     RETURN_ERROR(dstMaxSize_tooSmall);                 /* lz4frame.c:1009 */
//! if (cctxPtr->blockCompressMode != blockCompression) {
//!     bytesWritten = LZ4F_flush(cctxPtr, dstBuffer, dstCapacity, ...);
//!     dstPtr += bytesWritten;                            /* lz4frame.c:1014 */
//!     ...
//! }
//! ```
//!
//! So on a compressed -> uncompressed switch with buffered data, the C writes —
//! and reports — MORE bytes than the caller's `dstCapacity`. Observed here:
//! `LZ4F_uncompressedUpdate(srcSize=65536, dstCapacity=65560)` returns **65574**.
//!
//! The C is the ground truth, so the Rust must overrun by exactly the same
//! amount. This test allocates a large slack region past `dstCapacity` so the
//! overrun lands inside our own allocation (rather than corrupting the heap and
//! aborting the process) and then requires C and Rust to agree bit-for-bit,
//! including in the overrun region.

mod common;

use common::*;
use std::os::raw::{c_uint, c_void};
use std::ptr;

type FnCreateC = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeC = unsafe extern "C" fn(*mut c_void) -> usize;
type FnBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
) -> usize;
type FnFlush = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void) -> usize;
type FnBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;

/// Slack allocated past `dstCapacity`, large enough to absorb a whole flushed
/// block plus its header/footer.
const SLACK: usize = 1 << 17;
const FILL: u8 = 0xAA;

#[derive(Clone, Copy, Debug)]
enum Op {
    Upd(usize),
    Unc(usize),
    Flush,
}

/// Result of one library's run: for each step, the returned value and the full
/// (capacity + slack) destination buffer.
struct Run {
    rets: Vec<usize>,
    bufs: Vec<Vec<u8>>,
    caps: Vec<usize>,
}

fn run_plan(which: &str, prefs: &LZ4F_preferences_t, plan: &[Op]) -> Run {
    let l = libs();
    let lib = if which == "C" { &l.c } else { &l.rust };
    let create: FnCreateC = lib.sym("LZ4F_createCompressionContext");
    let free: FnFreeC = lib.sym("LZ4F_freeCompressionContext");
    let begin: FnBegin = lib.sym("LZ4F_compressBegin");
    let upd: FnUpdate = lib.sym("LZ4F_compressUpdate");
    let unc: FnUpdate = lib.sym("LZ4F_uncompressedUpdate");
    let flush: FnFlush = lib.sym("LZ4F_flush");
    let bound: FnBound = lib.sym("LZ4F_compressBound");

    let mut out = Run { rets: Vec::new(), bufs: Vec::new(), caps: Vec::new() };
    unsafe {
        let mut ctx: *mut c_void = ptr::null_mut();
        let rc = create(&mut ctx, LZ4F_VERSION);
        assert!(!lz4f_is_error(rc), "{}: createCompressionContext failed", which);

        let mut hdr = vec![FILL; 64];
        let n = begin(ctx, hdr.as_mut_ptr() as *mut c_void, hdr.len(), prefs);
        assert!(
            !lz4f_is_error(n),
            "{}: compressBegin failed with code {}",
            which,
            lz4f_error_code(n)
        );

        // Same seed on both sides so the inputs are identical.
        let mut rng = Rng::new(0xD1A6_0000_1234_5678);
        for (i, op) in plan.iter().enumerate() {
            let (sz, is_unc, is_flush) = match *op {
                Op::Upd(n) => (n, false, false),
                Op::Unc(n) => (n, true, false),
                Op::Flush => (0, false, true),
            };
            let src = gen_shape(&mut rng, i % N_SHAPES, sz);
            let cap = bound(sz, prefs).max(sz) + 16;
            let mut buf = vec![FILL; cap + SLACK];
            let r = if is_flush {
                flush(ctx, buf.as_mut_ptr() as *mut c_void, cap, ptr::null())
            } else if is_unc {
                unc(
                    ctx,
                    buf.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                    ptr::null(),
                )
            } else {
                upd(
                    ctx,
                    buf.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                    ptr::null(),
                )
            };
            assert!(
                r <= cap + SLACK || lz4f_is_error(r),
                "{}: step {} {:?} returned {} which exceeds cap {} + slack {}",
                which,
                i,
                op,
                r,
                cap,
                SLACK
            );
            out.rets.push(r);
            out.caps.push(cap);
            out.bufs.push(buf);
        }
        free(ctx);
    }
    out
}

/// The exact sequence that triggers the C's over-write: a compressed update
/// that leaves data buffered, then a switch to an uncompressed update.
fn overrun_plan(bs: usize) -> Vec<Op> {
    vec![
        Op::Upd(bs - 10), // buffers bs-10 bytes (autoFlush == 0)
        Op::Unc(20),      // switch -> flush buffered data first
        Op::Upd(30),      // switch back
        Op::Unc(bs),      // switch again, with a FULL block to store  <-- overruns
        Op::Upd(5),
        Op::Flush,
        Op::Unc(5),
    ]
}

#[test]
fn c_and_rust_overrun_dstcapacity_identically() {
    let bs = 65536usize;
    let mut saw_overrun = false;

    for af in [0u32, 1] {
        let mut prefs = LZ4F_preferences_t::default();
        prefs.frameInfo.blockSizeID = LZ4F_max64KB;
        // `LZ4F_uncompressedUpdate` is only supported for blockIndependent
        // (lz4frame.h:707); blockLinked hits an assert at lz4frame.c:1071.
        prefs.frameInfo.blockMode = LZ4F_blockIndependent;
        prefs.compressionLevel = 1;
        prefs.autoFlush = af;

        let plan = overrun_plan(bs);
        let c = run_plan("C", &prefs, &plan);
        let r = run_plan("Rust", &prefs, &plan);

        assert_eq!(c.rets.len(), r.rets.len());
        for i in 0..c.rets.len() {
            let cap = c.caps[i];
            assert_eq!(cap, r.caps[i], "step {}: capacity setup differs", i);
            assert_eq!(
                c.rets[i], r.rets[i],
                "autoFlush={} step {} {:?}: return value differs (C={} Rust={})",
                af, i, plan[i], c.rets[i] as i64, r.rets[i] as i64
            );
            // Compare the usable region AND the slack region.
            assert_bytes_eq(
                &format!(
                    "autoFlush={} step {} {:?} (cap={}, +{} slack)",
                    af, i, plan[i], cap, SLACK
                ),
                &c.bufs[i],
                &r.bufs[i],
            );
            if !lz4f_is_error(c.rets[i]) && c.rets[i] > cap {
                saw_overrun = true;
                println!(
                    "autoFlush={} step {} {:?}: C and Rust BOTH report {} bytes \
                     for dstCapacity={} (over by {})",
                    af, i, plan[i], c.rets[i], cap, c.rets[i] - cap
                );
            }
        }
    }

    // If this ever stops holding, the C's behaviour changed and the comment at
    // the top of this file (and the SLACK workaround in
    // tests/lz4frame_stream_diff.rs) should be revisited.
    assert!(
        saw_overrun,
        "expected to observe the documented dstCapacity over-write on the \
         compressed->uncompressed switch; the plan no longer reaches it"
    );
}
