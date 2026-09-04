//! Phase B — the remaining `CONFIGS.md` rows that the per-module test files do
//! not reach: large inputs, ring-buffer wraparound on every decoder, state
//! reuse across frames, direct `LZ4HC_searchExtDict` calls, and the legacy
//! stream lifecycles.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};

type F4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type F5 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type F3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FBound = unsafe extern "C" fn(c_int) -> c_int;
type FCreate = unsafe extern "C" fn() -> *mut c_void;
type FFree = unsafe extern "C" fn(*mut c_void) -> c_int;
type FLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FDecContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FDecFastContinue = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FSetDecode = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FVoid1 = unsafe extern "C" fn(*mut c_void);
type FSlide = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
type FCreateLegacy = unsafe extern "C" fn(*mut c_char) -> *mut c_void;
type FResetState = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
type FExt = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FXxh32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> u32;
type FXxh64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
// HC
type FHC = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FHCCreate = unsafe extern "C" fn() -> *mut c_void;
type FHCReset = unsafe extern "C" fn(*mut c_void, c_int);
type FHCContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FHCLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FHCSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FHCAttach = unsafe extern "C" fn(*mut c_void, *const c_void);
// frame
type FnCompressFrame =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnBoundP = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCreateCtx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
type FnFlush =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_compressOptions_t) -> usize;
type FnDecompress = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnGetBlockSize = unsafe extern "C" fn(c_int) -> usize;

struct Api {
    bound: FBound,
    compress_default: F4,
    compress_fast: F5,
    compress_fast_ext_fastreset: FExt,
    decompress_safe: F4,
    decompress_fast: F3,
    create_stream: FCreate,
    free_stream: FFree,
    reset_stream: FVoid1,
    reset_stream_fast: FVoid1,
    load_dict: FLoadDict,
    save_dict: FSaveDict,
    compress_continue: FContinue,
    slide: FSlide,
    create_legacy: FCreateLegacy,
    reset_stream_state: FResetState,
    create_decode: FCreate,
    free_decode: FFree,
    set_decode: FSetDecode,
    dec_continue: FDecContinue,
    dec_fast_continue: FDecFastContinue,
    ring_size: FBound,
    xxh32: FXxh32,
    xxh64: FXxh64,
    sizeof_state: FnIntVoid,
    // HC
    compress_hc: FHC,
    create_hc: FHCCreate,
    free_hc: FFree,
    reset_hc: FHCReset,
    reset_hc_fast: FHCReset,
    hc_continue: FHCContinue,
    load_dict_hc: FHCLoadDict,
    save_dict_hc: FHCSaveDict,
    attach_hc: FHCAttach,
    sizeof_state_hc: FnIntVoid,
    // frame
    compress_frame: FnCompressFrame,
    frame_bound: FnBoundP,
    fbound: FnBoundP,
    create_cctx: FnCreateCtx,
    free_cctx: FnFreeCtx,
    begin: FnBegin,
    update: FnUpdate,
    uncompressed_update: FnUpdate,
    flush: FnFlush,
    end: FnFlush,
    create_dctx: FnCreateCtx,
    free_dctx: FnFreeCtx,
    decompress: FnDecompress,
    is_error: FnIsError,
    get_block_size: FnGetBlockSize,
}

fn bind(l: &Lib) -> Api {
    Api {
        bound: l.sym("LZ4_compressBound"),
        compress_default: l.sym("LZ4_compress_default"),
        compress_fast: l.sym("LZ4_compress_fast"),
        compress_fast_ext_fastreset: l.sym("LZ4_compress_fast_extState_fastReset"),
        decompress_safe: l.sym("LZ4_decompress_safe"),
        decompress_fast: l.sym("LZ4_decompress_fast"),
        create_stream: l.sym("LZ4_createStream"),
        free_stream: l.sym("LZ4_freeStream"),
        reset_stream: l.sym("LZ4_resetStream"),
        reset_stream_fast: l.sym("LZ4_resetStream_fast"),
        load_dict: l.sym("LZ4_loadDict"),
        save_dict: l.sym("LZ4_saveDict"),
        compress_continue: l.sym("LZ4_compress_fast_continue"),
        slide: l.sym("LZ4_slideInputBuffer"),
        create_legacy: l.sym("LZ4_create"),
        reset_stream_state: l.sym("LZ4_resetStreamState"),
        create_decode: l.sym("LZ4_createStreamDecode"),
        free_decode: l.sym("LZ4_freeStreamDecode"),
        set_decode: l.sym("LZ4_setStreamDecode"),
        dec_continue: l.sym("LZ4_decompress_safe_continue"),
        dec_fast_continue: l.sym("LZ4_decompress_fast_continue"),
        ring_size: l.sym("LZ4_decoderRingBufferSize"),
        xxh32: l.sym("LZ4_XXH32"),
        xxh64: l.sym("LZ4_XXH64"),
        sizeof_state: l.sym("LZ4_sizeofState"),
        compress_hc: l.sym("LZ4_compress_HC"),
        create_hc: l.sym("LZ4_createStreamHC"),
        free_hc: l.sym("LZ4_freeStreamHC"),
        reset_hc: l.sym("LZ4_resetStreamHC"),
        reset_hc_fast: l.sym("LZ4_resetStreamHC_fast"),
        hc_continue: l.sym("LZ4_compress_HC_continue"),
        load_dict_hc: l.sym("LZ4_loadDictHC"),
        save_dict_hc: l.sym("LZ4_saveDictHC"),
        attach_hc: l.sym("LZ4_attach_HC_dictionary"),
        sizeof_state_hc: l.sym("LZ4_sizeofStateHC"),
        compress_frame: l.sym("LZ4F_compressFrame"),
        frame_bound: l.sym("LZ4F_compressFrameBound"),
        fbound: l.sym("LZ4F_compressBound"),
        create_cctx: l.sym("LZ4F_createCompressionContext"),
        free_cctx: l.sym("LZ4F_freeCompressionContext"),
        begin: l.sym("LZ4F_compressBegin"),
        update: l.sym("LZ4F_compressUpdate"),
        uncompressed_update: l.sym("LZ4F_uncompressedUpdate"),
        flush: l.sym("LZ4F_flush"),
        end: l.sym("LZ4F_compressEnd"),
        create_dctx: l.sym("LZ4F_createDecompressionContext"),
        free_dctx: l.sym("LZ4F_freeDecompressionContext"),
        decompress: l.sym("LZ4F_decompress"),
        is_error: l.sym("LZ4F_isError"),
        get_block_size: l.sym("LZ4F_getBlockSize"),
    }
}

fn pair() -> (Api, Api) {
    let p = libs();
    (bind(&p.c), bind(&p.r))
}

/// CONFIGS rows 9, 15: multi-megabyte xxHash inputs, aligned and unaligned.
#[test]
fn gap_xxhash_large_inputs() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x8001);
    for &shape in &[Shape::Text, Shape::Random] {
        let big = gen(shape, 5 << 20, &mut rng);
        for &off in &[0usize, 1, 2, 3, 7, 8] {
            for &len in &[65536usize, 1 << 20, (5 << 20) - 8] {
                let p = unsafe { big.as_ptr().add(off) } as *const c_void;
                for &s in &[0u32, 0x9E37_79B1] {
                    assert_eq!(
                        unsafe { (c.xxh32)(p, len, s) },
                        unsafe { (r.xxh32)(p, len, s) },
                        "XXH32 big shape={:?} off={} len={} seed={:#x}",
                        shape,
                        off,
                        len,
                        s
                    );
                }
                for &s in &[0u64, 0x9E37_79B1_85EB_CA87] {
                    assert_eq!(
                        unsafe { (c.xxh64)(p, len, s) },
                        unsafe { (r.xxh64)(p, len, s) },
                        "XXH64 big shape={:?} off={} len={} seed={:#x}",
                        shape,
                        off,
                        len,
                        s
                    );
                }
            }
        }
    }
}

/// CONFIGS rows 25, 36, 70: multi-megabyte block compression at every strategy.
#[test]
fn gap_large_block_inputs() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x8002);
    let cases: Vec<(String, Vec<u8>)> = vec![
        ("1MB run".into(), vec![0x61u8; 1 << 20]),
        ("5MB random".into(), gen(Shape::Random, 5 << 20, &mut rng)),
        ("5MB text".into(), gen(Shape::Text, 5 << 20, &mut rng)),
        ("2MB periodic".into(), gen(Shape::Periodic(1021), 2 << 20, &mut rng)),
    ];
    for (name, data) in &cases {
        let n = data.len();
        let cap = unsafe { (c.bound)(n as c_int) } as usize;
        let mut cb = vec![0u8; cap];
        let mut rb = vec![0u8; cap];
        for &acc in &[1i32, 3] {
            let a = unsafe {
                (c.compress_fast)(
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    acc,
                )
            };
            let b = unsafe {
                (r.compress_fast)(
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    acc,
                )
            };
            assert_eq!(a, b, "{} compress_fast acc={}", name, acc);
            assert_bytes_eq(
                &format!("{} compress_fast acc={}", name, acc),
                &cb[..a as usize],
                &rb[..b as usize],
            );
            // round-trip through both decoders
            let mut co = vec![0u8; n + 64];
            let mut ro = vec![0u8; n + 64];
            let x = unsafe {
                (c.decompress_safe)(
                    rb.as_ptr() as *const c_char,
                    co.as_mut_ptr() as *mut c_char,
                    a,
                    n as c_int,
                )
            };
            let y = unsafe {
                (r.decompress_safe)(
                    cb.as_ptr() as *const c_char,
                    ro.as_mut_ptr() as *mut c_char,
                    a,
                    n as c_int,
                )
            };
            assert_eq!(x, n as c_int, "{} C decode of Rust output", name);
            assert_eq!(y, n as c_int, "{} Rust decode of C output", name);
            assert_bytes_eq("large round-trip", &co[..n], data);
            assert_bytes_eq("large round-trip", &ro[..n], data);
            // LZ4_decompress_fast on the same block
            let mut co = vec![0u8; n + 64];
            let mut ro = vec![0u8; n + 64];
            let x = unsafe {
                (c.decompress_fast)(cb.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, n as c_int)
            };
            let y = unsafe {
                (r.decompress_fast)(cb.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, n as c_int)
            };
            assert_eq!(x, y, "{} decompress_fast rc", name);
            assert_bytes_eq("large decompress_fast", &co[..n], &ro[..n]);
        }
        // HC at each strategy boundary
        for &lvl in &[1i32, 2, 3, 9, 10, 12] {
            if n > (2 << 20) && lvl >= 10 {
                continue; // keeps the optimal parser's runtime bounded
            }
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = unsafe {
                (c.compress_hc)(
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    lvl,
                )
            };
            let b = unsafe {
                (r.compress_hc)(
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    lvl,
                )
            };
            assert_eq!(a, b, "{} compress_HC lvl={}", name, lvl);
            assert_bytes_eq(
                &format!("{} compress_HC lvl={}", name, lvl),
                &cb[..a as usize],
                &rb[..b as usize],
            );
        }
    }
}

/// CONFIGS rows 30, 43: `LZ4_compress_fast_extState_fastReset` on a state whose
/// `currentOffset != 0`, and reset on used streams.
#[test]
fn gap_fast_reset_used_state() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x8003);
    let ss = unsafe { (c.sizeof_state)() } as usize;
    let dict = gen(Shape::Text, 65536, &mut rng);
    for &n in &[100usize, 4095, 4096, 4097, 60_000, 70_000] {
        let data = gen(Shape::Text, n, &mut rng);
        let cap = (unsafe { (c.bound)(n as c_int) } as usize).max(1);
        unsafe {
            // 1) build a state with currentOffset != 0 via loadDict, then use the
            //    fastReset entry point directly on it (several times).
            let cs = (c.create_stream)();
            let rs = (r.create_stream)();
            (c.load_dict)(cs, dict.as_ptr() as *const c_char, dict.len() as c_int);
            (r.load_dict)(rs, dict.as_ptr() as *const c_char, dict.len() as c_int);
            for round in 0..3 {
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = (c.compress_fast_ext_fastreset)(
                    cs,
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (r.compress_fast_ext_fastreset)(
                    rs,
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                assert_eq!(a, b, "fastReset used state n={} round={}", n, round);
                assert_bytes_eq(
                    &format!("fastReset used state n={} round={}", n, round),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                assert_bytes_eq(
                    "fastReset state bytes",
                    std::slice::from_raw_parts(cs as *const u8, ss),
                    std::slice::from_raw_parts(rs as *const u8, ss),
                );
            }
            // 2) resetStream / resetStream_fast on the used stream
            (c.reset_stream_fast)(cs);
            (r.reset_stream_fast)(rs);
            assert_bytes_eq(
                "resetStream_fast on used stream",
                std::slice::from_raw_parts(cs as *const u8, ss),
                std::slice::from_raw_parts(rs as *const u8, ss),
            );
            (c.reset_stream)(cs);
            (r.reset_stream)(rs);
            assert_bytes_eq(
                "resetStream on used stream",
                std::slice::from_raw_parts(cs as *const u8, ss),
                std::slice::from_raw_parts(rs as *const u8, ss),
            );
            (c.free_stream)(cs);
            (r.free_stream)(rs);
        }
    }
}

/// CONFIGS rows 53, 61, 62: ring-buffer compression decoded through
/// `LZ4_decompress_safe_continue` *and* `LZ4_decompress_fast_continue`, with a
/// destination that wraps (prefix becomes extDict, then doubleDict).
#[test]
fn gap_ring_buffer_decoders() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x8004);
    for &maxblk in &[16usize, 1000, 4096, 65536] {
        let ring = unsafe { (c.ring_size)(maxblk as c_int) } as usize;
        assert_eq!(ring, unsafe { (r.ring_size)(maxblk as c_int) } as usize);
        let total = 300_000usize.min(ring * 6);
        let data = gen(Shape::Text, total, &mut rng);
        // compress from a ring buffer with both libraries in lock-step
        let mut blocks: Vec<(usize, Vec<u8>)> = Vec::new();
        unsafe {
            let cs = (c.create_stream)();
            let rs = (r.create_stream)();
            let mut cring = vec![0u8; ring];
            let mut rring = vec![0u8; ring];
            let mut pos = 0usize;
            let mut off = 0usize;
            while off < total {
                let n = rng.range(1, maxblk + 1).min(total - off);
                if pos + n > ring {
                    pos = 0;
                }
                cring[pos..pos + n].copy_from_slice(&data[off..off + n]);
                rring[pos..pos + n].copy_from_slice(&data[off..off + n]);
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = (c.compress_continue)(
                    cs,
                    cring.as_ptr().add(pos) as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (r.compress_continue)(
                    rs,
                    rring.as_ptr().add(pos) as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                assert_eq!(a, b, "ring compress maxblk={} n={}", maxblk, n);
                assert_bytes_eq("ring compress", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);
                blocks.push((n, cb[..a.max(0) as usize].to_vec()));
                pos += n;
                off += n;
            }
            (c.free_stream)(cs);
            (r.free_stream)(rs);
        }
        // decode into a ring buffer with safe_continue and with fast_continue
        for fast in [false, true] {
            unsafe {
                let cd = (c.create_decode)();
                let rd = (r.create_decode)();
                let mut cring = vec![0u8; ring + 64];
                let mut rring = vec![0u8; ring + 64];
                let mut cpos = 0usize;
                let mut rpos = 0usize;
                let mut cout = Vec::with_capacity(total);
                let mut rout = Vec::with_capacity(total);
                for (n, blk) in &blocks {
                    if cpos + maxblk > ring {
                        cpos = 0;
                        rpos = 0;
                    }
                    let (x, y) = if fast {
                        (
                            (c.dec_fast_continue)(
                                cd,
                                blk.as_ptr() as *const c_char,
                                cring.as_mut_ptr().add(cpos) as *mut c_char,
                                *n as c_int,
                            ),
                            (r.dec_fast_continue)(
                                rd,
                                blk.as_ptr() as *const c_char,
                                rring.as_mut_ptr().add(rpos) as *mut c_char,
                                *n as c_int,
                            ),
                        )
                    } else {
                        (
                            (c.dec_continue)(
                                cd,
                                blk.as_ptr() as *const c_char,
                                cring.as_mut_ptr().add(cpos) as *mut c_char,
                                blk.len() as c_int,
                                (ring - cpos) as c_int,
                            ),
                            (r.dec_continue)(
                                rd,
                                blk.as_ptr() as *const c_char,
                                rring.as_mut_ptr().add(rpos) as *mut c_char,
                                blk.len() as c_int,
                                (ring - rpos) as c_int,
                            ),
                        )
                    };
                    assert_eq!(x, y, "ring decode(fast={}) maxblk={}", fast, maxblk);
                    let produced = if fast { *n } else { x.max(0) as usize };
                    cout.extend_from_slice(&cring[cpos..cpos + produced]);
                    rout.extend_from_slice(&rring[rpos..rpos + produced]);
                    cpos += produced;
                    rpos += produced;
                }
                assert_bytes_eq("ring decode output", &cout, &rout);
                assert_bytes_eq("ring decode content", &cout, &data);
                (c.free_decode)(cd);
                (r.free_decode)(rd);
            }
        }
        // and: alternate between two separate destination buffers, which forces
        // the decoder through forceExtDict and then doubleDict
        for fast in [false, true] {
            unsafe {
                let cd = (c.create_decode)();
                let rd = (r.create_decode)();
                let mut cbufs = [vec![0u8; ring + 64], vec![0u8; ring + 64]];
                let mut rbufs = [vec![0u8; ring + 64], vec![0u8; ring + 64]];
                let mut which = 0usize;
                let mut pos = 0usize;
                let mut cout = Vec::new();
                let mut rout = Vec::new();
                for (i, (n, blk)) in blocks.iter().enumerate() {
                    if i % 3 == 0 {
                        which ^= 1;
                        pos = 0;
                    }
                    if pos + maxblk > ring {
                        pos = 0;
                    }
                    let (x, y) = if fast {
                        (
                            (c.dec_fast_continue)(
                                cd,
                                blk.as_ptr() as *const c_char,
                                cbufs[which].as_mut_ptr().add(pos) as *mut c_char,
                                *n as c_int,
                            ),
                            (r.dec_fast_continue)(
                                rd,
                                blk.as_ptr() as *const c_char,
                                rbufs[which].as_mut_ptr().add(pos) as *mut c_char,
                                *n as c_int,
                            ),
                        )
                    } else {
                        (
                            (c.dec_continue)(
                                cd,
                                blk.as_ptr() as *const c_char,
                                cbufs[which].as_mut_ptr().add(pos) as *mut c_char,
                                blk.len() as c_int,
                                (ring - pos) as c_int,
                            ),
                            (r.dec_continue)(
                                rd,
                                blk.as_ptr() as *const c_char,
                                rbufs[which].as_mut_ptr().add(pos) as *mut c_char,
                                blk.len() as c_int,
                                (ring - pos) as c_int,
                            ),
                        )
                    };
                    assert_eq!(
                        x, y,
                        "two-buffer decode(fast={}) maxblk={} block={}",
                        fast, maxblk, i
                    );
                    if x <= 0 && !fast {
                        // both agreed on the failure; stop this scenario
                        break;
                    }
                    let produced = if fast { *n } else { x as usize };
                    cout.extend_from_slice(&cbufs[which][pos..pos + produced]);
                    rout.extend_from_slice(&rbufs[which][pos..pos + produced]);
                    pos += produced;
                }
                assert_bytes_eq("two-buffer decode output", &cout, &rout);
                (c.free_decode)(cd);
                (r.free_decode)(rd);
            }
        }
        // relocate the history mid-stream with LZ4_setStreamDecode
        unsafe {
            let cd = (c.create_decode)();
            let rd = (r.create_decode)();
            let mut cbuf = vec![0u8; total + 64];
            let mut rbuf = vec![0u8; total + 64];
            let mut off = 0usize;
            for (i, (n, blk)) in blocks.iter().enumerate() {
                if i % 5 == 4 && off >= 64 {
                    let ds = off.min(65536);
                    let a = (c.set_decode)(cd, cbuf.as_ptr().add(off - ds) as *const c_char, ds as c_int);
                    let b = (r.set_decode)(rd, rbuf.as_ptr().add(off - ds) as *const c_char, ds as c_int);
                    assert_eq!(a, b, "setStreamDecode mid-stream");
                }
                let x = (c.dec_continue)(
                    cd,
                    blk.as_ptr() as *const c_char,
                    cbuf.as_mut_ptr().add(off) as *mut c_char,
                    blk.len() as c_int,
                    (total + 64 - off) as c_int,
                );
                let y = (r.dec_continue)(
                    rd,
                    blk.as_ptr() as *const c_char,
                    rbuf.as_mut_ptr().add(off) as *mut c_char,
                    blk.len() as c_int,
                    (total + 64 - off) as c_int,
                );
                assert_eq!(x, y, "setStreamDecode relocate maxblk={} block={}", maxblk, i);
                assert_eq!(x, *n as c_int);
                off += x as usize;
            }
            assert_bytes_eq("relocate output", &cbuf[..off], &rbuf[..off]);
            assert_bytes_eq("relocate content", &cbuf[..off], &data[..off]);
            (c.free_decode)(cd);
            (r.free_decode)(rd);
        }
    }
}

/// CONFIGS rows 54, 55, 91: `LZ4_saveDict` continuation, `LZ4_slideInputBuffer`
/// after real use, and the legacy fast-stream lifecycle.
#[test]
fn gap_save_dict_and_legacy_stream() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x8005);
    let total = 200_000usize;
    let data = gen(Shape::Text, total, &mut rng);
    // saveDict into a fresh buffer, then keep compressing from that buffer
    for &sm in &[0i32, 4, 1024, 65536, 100_000] {
        unsafe {
            let cs = (c.create_stream)();
            let rs = (r.create_stream)();
            let mut cdict = vec![0u8; 110_000];
            let mut rdict = vec![0u8; 110_000];
            let mut off = 0usize;
            for round in 0..6 {
                let n = 20_000usize.min(total - off);
                if n == 0 {
                    break;
                }
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = (c.compress_continue)(
                    cs,
                    data.as_ptr().add(off) as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (r.compress_continue)(
                    rs,
                    data.as_ptr().add(off) as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                assert_eq!(a, b, "saveDict cont sm={} round={}", sm, round);
                assert_bytes_eq("saveDict cont", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);
                let sa = (c.save_dict)(cs, cdict.as_mut_ptr() as *mut c_char, sm);
                let sb = (r.save_dict)(rs, rdict.as_mut_ptr() as *mut c_char, sm);
                assert_eq!(sa, sb, "saveDict rc sm={} round={}", sm, round);
                assert_bytes_eq(
                    "saveDict bytes",
                    &cdict[..sa.max(0) as usize],
                    &rdict[..sb.max(0) as usize],
                );
                // LZ4_slideInputBuffer now returns the (relocated) dictionary
                let cp = (c.slide)(cs);
                let rp = (r.slide)(rs);
                assert_eq!(cp.is_null(), rp.is_null(), "slideInputBuffer null-ness");
                off += n;
            }
            (c.free_stream)(cs);
            (r.free_stream)(rs);
        }
    }
    // legacy lifecycle: LZ4_create + LZ4_resetStreamState + LZ4_compress_continue
    unsafe {
        type FCont4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
        let p = libs();
        let ccont: FCont4 = p.c.sym("LZ4_compress_continue");
        let rcont: FCont4 = p.r.sym("LZ4_compress_continue");
        let cs = (c.create_legacy)(data.as_ptr() as *mut c_char);
        let rs = (r.create_legacy)(data.as_ptr() as *mut c_char);
        let mut off = 0usize;
        for round in 0..4 {
            let n = 10_000usize;
            let cap = ((c.bound)(n as c_int) as usize).max(1);
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = ccont(cs, data.as_ptr().add(off) as *const c_char, cb.as_mut_ptr() as *mut c_char, n as c_int);
            let b = rcont(rs, data.as_ptr().add(off) as *const c_char, rb.as_mut_ptr() as *mut c_char, n as c_int);
            assert_eq!(a, b, "legacy compress_continue round={}", round);
            assert_bytes_eq("legacy compress_continue", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);
            off += n;
        }
        assert_eq!((c.slide)(cs).is_null(), (r.slide)(rs).is_null());
        (c.free_stream)(cs);
        (r.free_stream)(rs);
        // resetStreamState on a properly aligned caller buffer
        let ss = (c.sizeof_state)() as usize;
        let mut cbuf = vec![0u64; ss / 8 + 2];
        let mut rbuf = vec![0u64; ss / 8 + 2];
        let a = (c.reset_stream_state)(cbuf.as_mut_ptr() as *mut c_void, data.as_ptr() as *mut c_char);
        let b = (r.reset_stream_state)(rbuf.as_mut_ptr() as *mut c_void, data.as_ptr() as *mut c_char);
        assert_eq!(a, b, "legacy resetStreamState");
        assert_bytes_eq(
            "legacy resetStreamState bytes",
            std::slice::from_raw_parts(cbuf.as_ptr() as *const u8, ss),
            std::slice::from_raw_parts(rbuf.as_ptr() as *const u8, ss),
        );
    }
}

/// CONFIGS row 86: `LZ4HC_searchExtDict` called directly through the FFI, plus a
/// HC ring-buffer round trip and `saveDictHC` continuation.
#[test]
fn gap_hc_ext_dict_and_ring() {
    let (c, r) = pair();
    let p = libs();
    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    struct Lz4hcMatch {
        off: c_int,
        len: c_int,
        back: c_int,
    }
    type FSearchExtDict = unsafe extern "C" fn(
        *const u8,
        u32,
        *const u8,
        *const u8,
        *const c_void,
        u32,
        c_int,
        c_int,
    ) -> Lz4hcMatch;
    let cse: FSearchExtDict = p.c.sym("LZ4HC_searchExtDict");
    let rse: FSearchExtDict = p.r.sym("LZ4HC_searchExtDict");

    let mut rng = Rng::new(0x8006);
    let dict = gen(Shape::Text, 65536, &mut rng);
    let mut input = gen(Shape::Text, 65536, &mut rng);
    input[..1024].copy_from_slice(&dict[..1024]);

    // Build a dictionary context with LZ4_loadDictHC in each library, then call
    // LZ4HC_searchExtDict directly with identical arguments.
    for &lvl in &[3i32, 6, 9, 12] {
        unsafe {
            let cdict = (c.create_hc)();
            let rdict = (r.create_hc)();
            (c.reset_hc)(cdict, lvl);
            (r.reset_hc)(rdict, lvl);
            (c.load_dict_hc)(cdict, dict.as_ptr() as *const c_char, dict.len() as c_int);
            (r.load_dict_hc)(rdict, dict.as_ptr() as *const c_char, dict.len() as c_int);
            for &nb in &[1i32, 4, 16, 64] {
                for &ip_off in &[0usize, 1, 7, 64, 500, 1000] {
                    // Keep every pointer well inside `input` so both libraries do
                    // exactly the same (in-bounds) reads.
                    let ip = input.as_ptr().add(ip_off);
                    let ilow = input.as_ptr();
                    let ihigh = input.as_ptr().add(input.len() - 16);
                    let ip_index = (65536 + ip_off) as u32;
                    let gdict_end = 65536u32;
                    let a = cse(
                        ip,
                        ip_index,
                        ilow,
                        ihigh,
                        cdict as *const c_void,
                        gdict_end,
                        3,
                        nb,
                    );
                    let b = rse(
                        ip,
                        ip_index,
                        ilow,
                        ihigh,
                        rdict as *const c_void,
                        gdict_end,
                        3,
                        nb,
                    );
                    assert_eq!(
                        a, b,
                        "LZ4HC_searchExtDict lvl={} nb={} ip_off={}",
                        lvl, nb, ip_off
                    );
                }
            }
            (c.free_hc)(cdict);
            (r.free_hc)(rdict);
        }
    }

    // HC ring-buffer round trip decoded with LZ4_decompress_safe_continue
    let total = 200_000usize;
    let data = gen(Shape::Text, total, &mut rng);
    for &lvl in &[2i32, 9, 12] {
        let maxblk = 8192usize;
        let ring = unsafe { (c.ring_size)(maxblk as c_int) } as usize;
        let mut blocks: Vec<(usize, Vec<u8>)> = Vec::new();
        unsafe {
            let cs = (c.create_hc)();
            let rs = (r.create_hc)();
            (c.reset_hc)(cs, lvl);
            (r.reset_hc)(rs, lvl);
            let mut cring = vec![0u8; ring];
            let mut rring = vec![0u8; ring];
            let mut pos = 0usize;
            let mut off = 0usize;
            while off < total {
                let n = maxblk.min(total - off);
                if pos + n > ring {
                    pos = 0;
                }
                cring[pos..pos + n].copy_from_slice(&data[off..off + n]);
                rring[pos..pos + n].copy_from_slice(&data[off..off + n]);
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = (c.hc_continue)(
                    cs,
                    cring.as_ptr().add(pos) as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                let b = (r.hc_continue)(
                    rs,
                    rring.as_ptr().add(pos) as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                assert_eq!(a, b, "HC ring compress lvl={} n={}", lvl, n);
                assert_bytes_eq("HC ring compress", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);
                blocks.push((n, cb[..a.max(0) as usize].to_vec()));
                pos += n;
                off += n;
            }
            (c.free_hc)(cs);
            (r.free_hc)(rs);
            // decode
            let cd = (c.create_decode)();
            let rd = (r.create_decode)();
            let mut cring = vec![0u8; ring + 64];
            let mut rring = vec![0u8; ring + 64];
            let mut cpos = 0usize;
            let mut cout = Vec::new();
            let mut rout = Vec::new();
            for (n, blk) in &blocks {
                if cpos + maxblk > ring {
                    cpos = 0;
                }
                let x = (c.dec_continue)(
                    cd,
                    blk.as_ptr() as *const c_char,
                    cring.as_mut_ptr().add(cpos) as *mut c_char,
                    blk.len() as c_int,
                    (ring - cpos) as c_int,
                );
                let y = (r.dec_continue)(
                    rd,
                    blk.as_ptr() as *const c_char,
                    rring.as_mut_ptr().add(cpos) as *mut c_char,
                    blk.len() as c_int,
                    (ring - cpos) as c_int,
                );
                assert_eq!(x, y, "HC ring decode lvl={}", lvl);
                assert_eq!(x, *n as c_int);
                cout.extend_from_slice(&cring[cpos..cpos + x as usize]);
                rout.extend_from_slice(&rring[cpos..cpos + y as usize]);
                cpos += x as usize;
            }
            assert_bytes_eq("HC ring round trip", &cout, &rout);
            assert_bytes_eq("HC ring content", &cout, &data);
            (c.free_decode)(cd);
            (r.free_decode)(rd);
        }
    }

    // saveDictHC continuation: keep compressing from the saved buffer
    for &lvl in &[2i32, 9, 12] {
        unsafe {
            let cs = (c.create_hc)();
            let rs = (r.create_hc)();
            (c.reset_hc)(cs, lvl);
            (r.reset_hc)(rs, lvl);
            let mut csave = vec![0u8; 70_000];
            let mut rsave = vec![0u8; 70_000];
            let mut off = 0usize;
            for round in 0..5 {
                let n = 20_000usize.min(total - off);
                if n == 0 {
                    break;
                }
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = (c.hc_continue)(
                    cs,
                    data.as_ptr().add(off) as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                let b = (r.hc_continue)(
                    rs,
                    data.as_ptr().add(off) as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                assert_eq!(a, b, "HC saveDict cont lvl={} round={}", lvl, round);
                assert_bytes_eq("HC saveDict cont", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);
                let sa = (c.save_dict_hc)(cs, csave.as_mut_ptr() as *mut c_char, 65536);
                let sb = (r.save_dict_hc)(rs, rsave.as_mut_ptr() as *mut c_char, 65536);
                assert_eq!(sa, sb, "saveDictHC rc lvl={} round={}", lvl, round);
                assert_bytes_eq(
                    "saveDictHC bytes",
                    &csave[..sa.max(0) as usize],
                    &rsave[..sb.max(0) as usize],
                );
                off += n;
            }
            (c.free_hc)(cs);
            (r.free_hc)(rs);
        }
    }

    // attach_HC_dictionary with an incompatible strategy (row 81)
    for &(dict_lvl, work_lvl) in &[(2i32, 9i32), (9, 2), (12, 2), (2, 12), (9, 9)] {
        for &n in &[1000usize, 4096, 4097, 40_000] {
            unsafe {
                let cdict = (c.create_hc)();
                let rdict = (r.create_hc)();
                (c.reset_hc)(cdict, dict_lvl);
                (r.reset_hc)(rdict, dict_lvl);
                (c.load_dict_hc)(cdict, dict.as_ptr() as *const c_char, dict.len() as c_int);
                (r.load_dict_hc)(rdict, dict.as_ptr() as *const c_char, dict.len() as c_int);
                let cs = (c.create_hc)();
                let rs = (r.create_hc)();
                (c.reset_hc_fast)(cs, work_lvl);
                (r.reset_hc_fast)(rs, work_lvl);
                (c.attach_hc)(cs, cdict as *const c_void);
                (r.attach_hc)(rs, rdict as *const c_void);
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = (c.hc_continue)(
                    cs,
                    input.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                let b = (r.hc_continue)(
                    rs,
                    input.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                assert_eq!(
                    a, b,
                    "attach_HC incompatible dict_lvl={} work_lvl={} n={}",
                    dict_lvl, work_lvl, n
                );
                assert_bytes_eq(
                    &format!("attach_HC dict_lvl={} work_lvl={} n={}", dict_lvl, work_lvl, n),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                (c.free_hc)(cs);
                (c.free_hc)(cdict);
                (r.free_hc)(rs);
                (r.free_hc)(rdict);
            }
        }
    }
}

/// CONFIGS rows 99, 107, 114, 121, 131: large frames, cctx reuse across frames
/// with level changes, alternating update modes, and a big skippable frame.
#[test]
fn gap_frame_reuse_and_large() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x8007);

    // (a) 10MB through max4MB blocks, and 1MB exactly / +1 through max1MB
    for &(bsid, n) in &[(7i32, 10 << 20), (6, 1 << 20), (6, (1 << 20) + 1), (5, 1 << 20)] {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = bsid;
        p.frameInfo.contentChecksumFlag = 1;
        let data = gen(Shape::Text, n, &mut rng);
        let cap = unsafe { (c.frame_bound)(n, &p) };
        let mut cb = vec![0u8; cap];
        let mut rb = vec![0u8; cap];
        let a = unsafe {
            (c.compress_frame)(cb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
        };
        let b = unsafe {
            (r.compress_frame)(rb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
        };
        assert_eq!(a, b, "large frame bsid={} n={}", bsid, n);
        assert_bytes_eq(&format!("large frame bsid={} n={}", bsid, n), &cb[..a], &rb[..b]);
    }

    // (b) cctx reuse across frames with level transitions
    unsafe {
        let mut cc: *mut c_void = std::ptr::null_mut();
        let mut rc_: *mut c_void = std::ptr::null_mut();
        (c.create_cctx)(&mut cc, LZ4F_VERSION);
        (r.create_cctx)(&mut rc_, LZ4F_VERSION);
        for (i, &lvl) in [1i32, 9, 1, 12, 12, 2, 9, 0].iter().enumerate() {
            let mut p = LZ4F_preferences_t::default();
            p.compressionLevel = lvl;
            p.frameInfo.blockSizeID = [0i32, 4, 5, 6, 7][i % 5];
            p.autoFlush = (i % 2) as c_uint;
            let n = 120_000usize;
            let data = gen(Shape::Text, n, &mut rng);
            let mut chb = vec![0u8; 64];
            let mut rhb = vec![0u8; 64];
            let a = (c.begin)(cc, chb.as_mut_ptr() as *mut c_void, 64, &p);
            let b = (r.begin)(rc_, rhb.as_mut_ptr() as *mut c_void, 64, &p);
            assert_eq!(a, b, "reuse begin i={} lvl={}", i, lvl);
            let mut cframe = chb[..a].to_vec();
            let mut rframe = rhb[..b].to_vec();
            let ucap = (c.fbound)(n, &p);
            let mut ub = vec![0u8; ucap];
            let mut vb = vec![0u8; ucap];
            let a = (c.update)(cc, ub.as_mut_ptr() as *mut c_void, ucap, data.as_ptr() as *const c_void, n, std::ptr::null());
            let b = (r.update)(rc_, vb.as_mut_ptr() as *mut c_void, ucap, data.as_ptr() as *const c_void, n, std::ptr::null());
            assert_eq!(a, b, "reuse update i={} lvl={}", i, lvl);
            assert_bytes_eq("reuse update", &ub[..a], &vb[..b]);
            cframe.extend_from_slice(&ub[..a]);
            rframe.extend_from_slice(&vb[..b]);
            let ecap = (c.fbound)(0, &p);
            let mut ub = vec![0u8; ecap];
            let mut vb = vec![0u8; ecap];
            let a = (c.end)(cc, ub.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
            let b = (r.end)(rc_, vb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
            assert_eq!(a, b, "reuse end i={} lvl={}", i, lvl);
            cframe.extend_from_slice(&ub[..a]);
            rframe.extend_from_slice(&vb[..b]);
            assert_bytes_eq(&format!("reuse frame i={} lvl={}", i, lvl), &cframe, &rframe);
        }
        (c.free_cctx)(cc);
        (r.free_cctx)(rc_);
    }

    // (c) alternating compressUpdate / uncompressedUpdate in both orders
    for start_raw in [false, true] {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockMode = LZ4F_BLOCK_INDEPENDENT;
        p.frameInfo.blockChecksumFlag = 1;
        p.frameInfo.contentChecksumFlag = 1;
        let total = 150_000usize;
        let data = gen(Shape::Text, total, &mut rng);
        unsafe {
            let mut cc: *mut c_void = std::ptr::null_mut();
            let mut rc_: *mut c_void = std::ptr::null_mut();
            (c.create_cctx)(&mut cc, LZ4F_VERSION);
            (r.create_cctx)(&mut rc_, LZ4F_VERSION);
            let mut chb = vec![0u8; 64];
            let mut rhb = vec![0u8; 64];
            let a = (c.begin)(cc, chb.as_mut_ptr() as *mut c_void, 64, &p);
            let b = (r.begin)(rc_, rhb.as_mut_ptr() as *mut c_void, 64, &p);
            let mut cframe = chb[..a].to_vec();
            let mut rframe = rhb[..b].to_vec();
            let mut off = 0usize;
            let mut i = 0usize;
            while off < total {
                let n = 17_000usize.min(total - off);
                let raw = if start_raw { i % 2 == 0 } else { i % 2 == 1 };
                let cap = (c.fbound)(n, &p).max(n + 64);
                let mut ub = vec![0u8; cap];
                let mut vb = vec![0u8; cap];
                let f = if raw {
                    (c.uncompressed_update, r.uncompressed_update)
                } else {
                    (c.update, r.update)
                };
                let a = (f.0)(cc, ub.as_mut_ptr() as *mut c_void, cap, data.as_ptr().add(off) as *const c_void, n, std::ptr::null());
                let b = (f.1)(rc_, vb.as_mut_ptr() as *mut c_void, cap, data.as_ptr().add(off) as *const c_void, n, std::ptr::null());
                assert_eq!(a, b, "alternating start_raw={} i={} raw={}", start_raw, i, raw);
                assert!((c.is_error)(a) == 0, "alternating failed: {}", fmt_lz4f(a));
                assert_bytes_eq("alternating update", &ub[..a], &vb[..b]);
                cframe.extend_from_slice(&ub[..a]);
                rframe.extend_from_slice(&vb[..b]);
                off += n;
                i += 1;
            }
            let ecap = (c.fbound)(0, &p);
            let mut ub = vec![0u8; ecap];
            let mut vb = vec![0u8; ecap];
            let a = (c.end)(cc, ub.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
            let b = (r.end)(rc_, vb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
            assert_eq!(a, b, "alternating end start_raw={}", start_raw);
            cframe.extend_from_slice(&ub[..a]);
            rframe.extend_from_slice(&vb[..b]);
            assert_bytes_eq(&format!("alternating frame start_raw={}", start_raw), &cframe, &rframe);
            (c.free_cctx)(cc);
            (r.free_cctx)(rc_);
            // round trip
            let mut cd: *mut c_void = std::ptr::null_mut();
            let mut rd: *mut c_void = std::ptr::null_mut();
            (c.create_dctx)(&mut cd, LZ4F_VERSION);
            (r.create_dctx)(&mut rd, LZ4F_VERSION);
            for (api, dctx, frame) in [(&c, cd, &cframe), (&r, rd, &rframe)] {
                let mut out = vec![0u8; total + 4096];
                let mut ds = out.len();
                let mut ss = frame.len();
                let rc2 = (api.decompress)(
                    dctx,
                    out.as_mut_ptr() as *mut c_void,
                    &mut ds,
                    frame.as_ptr() as *const c_void,
                    &mut ss,
                    std::ptr::null(),
                );
                assert!((api.is_error)(rc2) == 0, "alternating decode: {}", fmt_lz4f(rc2));
                assert_bytes_eq("alternating decode content", &out[..ds], &data);
            }
            (c.free_dctx)(cd);
            (r.free_dctx)(rd);
        }
    }

    // (d) a 1MB skippable frame followed by a normal frame, fed in fragments
    {
        let payload = gen(Shape::Random, 1 << 20, &mut rng);
        let mut stream = Vec::new();
        stream.extend_from_slice(&0x184D2A50u32.to_le_bytes());
        stream.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        stream.extend_from_slice(&payload);
        let n = 50_000usize;
        let data = gen(Shape::Text, n, &mut rng);
        let p = LZ4F_preferences_t::default();
        let cap = unsafe { (c.frame_bound)(n, &p) };
        let mut fb = vec![0u8; cap];
        let flen = unsafe {
            (c.compress_frame)(fb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
        };
        stream.extend_from_slice(&fb[..flen]);
        for &chunk in &[1usize, 3, 7777, usize::MAX] {
            unsafe {
                let mut cd: *mut c_void = std::ptr::null_mut();
                let mut rd: *mut c_void = std::ptr::null_mut();
                (c.create_dctx)(&mut cd, LZ4F_VERSION);
                (r.create_dctx)(&mut rd, LZ4F_VERSION);
                let mut cout = Vec::new();
                let mut rout = Vec::new();
                let mut sp = 0usize;
                let mut cbuf = vec![0u8; 1 << 16];
                let mut rbuf = vec![0u8; 1 << 16];
                let mut guard = 0u64;
                while sp < stream.len() {
                    guard += 1;
                    if guard > 20_000_000 {
                        panic!("stalled");
                    }
                    let mut cs = (stream.len() - sp).min(chunk.max(1));
                    let mut rs = cs;
                    let mut cds = cbuf.len();
                    let mut rds = rbuf.len();
                    let x = (c.decompress)(
                        cd,
                        cbuf.as_mut_ptr() as *mut c_void,
                        &mut cds,
                        stream.as_ptr().add(sp) as *const c_void,
                        &mut cs,
                        std::ptr::null(),
                    );
                    let y = (r.decompress)(
                        rd,
                        rbuf.as_mut_ptr() as *mut c_void,
                        &mut rds,
                        stream.as_ptr().add(sp) as *const c_void,
                        &mut rs,
                        std::ptr::null(),
                    );
                    assert_eq!(x, y, "skippable+frame chunk={} rc", chunk);
                    assert_eq!(cs, rs, "skippable+frame chunk={} consumed", chunk);
                    assert_eq!(cds, rds, "skippable+frame chunk={} produced", chunk);
                    assert_bytes_eq("skippable+frame out", &cbuf[..cds], &rbuf[..rds]);
                    if (c.is_error)(x) != 0 {
                        break;
                    }
                    cout.extend_from_slice(&cbuf[..cds]);
                    rout.extend_from_slice(&rbuf[..rds]);
                    sp += cs;
                    if cs == 0 && cds == 0 {
                        break;
                    }
                }
                assert_bytes_eq("skippable+frame content", &cout, &rout);
                assert_bytes_eq("skippable+frame payload", &cout, &data);
                (c.free_dctx)(cd);
                (r.free_dctx)(rd);
            }
        }
    }
}

/// CONFIGS rows 115, 116, 119, 128: `LZ4F_compressUpdate` fed one byte at a time
/// across a block boundary, and `LZ4F_decompress` with a 1-byte destination.
#[test]
fn gap_frame_byte_at_a_time() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x8008);
    for &bsid in &[LZ4F_MAX64KB, LZ4F_MAX256KB] {
        for &bmode in &[LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for &af in &[0u32, 1] {
                for &stable in &[0u32, 1] {
                    let mut p = LZ4F_preferences_t::default();
                    p.frameInfo.blockSizeID = bsid;
                    p.frameInfo.blockMode = bmode;
                    p.autoFlush = af;
                    let bs = unsafe { (c.get_block_size)(bsid) };
                    let total = bs + 37;
                    let data = gen(Shape::Text, total, &mut rng);
                    let co = LZ4F_compressOptions_t {
                        stableSrc: stable,
                        reserved: [0; 3],
                    };
                    unsafe {
                        let mut cc: *mut c_void = std::ptr::null_mut();
                        let mut rc_: *mut c_void = std::ptr::null_mut();
                        (c.create_cctx)(&mut cc, LZ4F_VERSION);
                        (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                        let mut chb = vec![0u8; 64];
                        let mut rhb = vec![0u8; 64];
                        let a = (c.begin)(cc, chb.as_mut_ptr() as *mut c_void, 64, &p);
                        let b = (r.begin)(rc_, rhb.as_mut_ptr() as *mut c_void, 64, &p);
                        let mut cframe = chb[..a].to_vec();
                        let mut rframe = rhb[..b].to_vec();
                        let cap = (c.fbound)(1, &p);
                        let mut ub = vec![0u8; cap];
                        let mut vb = vec![0u8; cap];
                        for i in 0..total {
                            let a = (c.update)(
                                cc,
                                ub.as_mut_ptr() as *mut c_void,
                                cap,
                                data.as_ptr().add(i) as *const c_void,
                                1,
                                &co,
                            );
                            let b = (r.update)(
                                rc_,
                                vb.as_mut_ptr() as *mut c_void,
                                cap,
                                data.as_ptr().add(i) as *const c_void,
                                1,
                                &co,
                            );
                            assert_eq!(
                                a, b,
                                "1-byte update bsid={} bmode={} af={} stable={} i={}",
                                bsid, bmode, af, stable, i
                            );
                            assert_bytes_eq("1-byte update", &ub[..a], &vb[..b]);
                            cframe.extend_from_slice(&ub[..a]);
                            rframe.extend_from_slice(&vb[..b]);
                        }
                        let ecap = (c.fbound)(0, &p);
                        let mut ub = vec![0u8; ecap];
                        let mut vb = vec![0u8; ecap];
                        let a = (c.end)(cc, ub.as_mut_ptr() as *mut c_void, ecap, &co);
                        let b = (r.end)(rc_, vb.as_mut_ptr() as *mut c_void, ecap, &co);
                        assert_eq!(a, b, "1-byte end");
                        cframe.extend_from_slice(&ub[..a]);
                        rframe.extend_from_slice(&vb[..b]);
                        assert_bytes_eq("1-byte frame", &cframe, &rframe);
                        (c.free_cctx)(cc);
                        (r.free_cctx)(rc_);

                        // decode with a 1-byte destination (drives tmpOut/flushOut)
                        for &sd in &[0u32, 1] {
                            let opts = LZ4F_decompressOptions_t {
                                stableDst: sd,
                                skipChecksums: 0,
                                reserved1: 0,
                                reserved0: 0,
                            };
                            let mut cd: *mut c_void = std::ptr::null_mut();
                            let mut rd: *mut c_void = std::ptr::null_mut();
                            (c.create_dctx)(&mut cd, LZ4F_VERSION);
                            (r.create_dctx)(&mut rd, LZ4F_VERSION);
                            let mut cout = vec![0u8; total + 64];
                            let mut rout = vec![0u8; total + 64];
                            let mut cdp = 0usize;
                            let mut rdp = 0usize;
                            let mut sp = 0usize;
                            loop {
                                let mut cs = cframe.len() - sp;
                                let mut rs = cs;
                                let mut cds = 1usize;
                                let mut rds = 1usize;
                                let x = (c.decompress)(
                                    cd,
                                    cout.as_mut_ptr().add(cdp) as *mut c_void,
                                    &mut cds,
                                    cframe.as_ptr().add(sp) as *const c_void,
                                    &mut cs,
                                    &opts,
                                );
                                let y = (r.decompress)(
                                    rd,
                                    rout.as_mut_ptr().add(rdp) as *mut c_void,
                                    &mut rds,
                                    rframe.as_ptr().add(sp) as *const c_void,
                                    &mut rs,
                                    &opts,
                                );
                                assert_eq!(x, y, "1-byte dst decode rc sd={}", sd);
                                assert_eq!(cs, rs, "1-byte dst decode consumed");
                                assert_eq!(cds, rds, "1-byte dst decode produced");
                                if (c.is_error)(x) != 0 {
                                    break;
                                }
                                cdp += cds;
                                rdp += rds;
                                sp += cs;
                                if x == 0 {
                                    break;
                                }
                                if cs == 0 && cds == 0 {
                                    break;
                                }
                            }
                            assert_eq!(cdp, rdp, "1-byte dst decode total");
                            assert_bytes_eq("1-byte dst decode", &cout[..cdp], &rout[..rdp]);
                            assert_bytes_eq("1-byte dst content", &cout[..cdp], &data[..cdp]);
                            (c.free_dctx)(cd);
                            (r.free_dctx)(rd);
                        }
                    }
                }
            }
        }
    }
}
