//! Differential tests for CONFIGS.md rows 35-65:
//!   * "lz4 streaming (chained-block) compression"  (rows 35-52)
//!   * "lz4 streaming + dictionary decompression"   (rows 53-65)
//!
//! Every call goes through a `.so` export. Opaque state (`LZ4_stream_t`,
//! `LZ4_streamDecode_t`) is *always* created by the library that will use it and
//! freed by that same library -- a C-allocated context is never handed to the
//! Rust `.so` or vice versa.
//!
//! Cross-decompression is built into `decode_*` helpers: the blocks produced by
//! the C compressor are fed to the *Rust* stream decoder and the blocks produced
//! by the Rust compressor are fed to the *C* stream decoder.

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Signature aliases
// ---------------------------------------------------------------------------

type FnVoidP = unsafe extern "C" fn(*mut c_void);
type FnCfc =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnCc4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnCc5 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnAttach = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FnIntToInt = unsafe extern "C" fn(c_int) -> c_int;
type FnSud =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;
type FnSpud = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_char,
    c_int,
) -> c_int;
type FnFud = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, *const c_char, c_int) -> c_int;
type FnSfed =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_void, usize) -> c_int;
type FnSpfed = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_void,
    usize,
) -> c_int;
type FnPtrToPtr = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
type FnCreateLegacy = unsafe extern "C" fn(*mut c_char) -> *mut c_void;
type FnResetSS = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;

// ---------------------------------------------------------------------------
// One "API view" per library, so the two sides are driven symmetrically.
// ---------------------------------------------------------------------------

macro_rules! api {
    ($sname:ident, $ctor:ident, { $($f:ident : $t:ty = $n:literal,)* }) => {
        #[derive(Clone, Copy)]
        #[allow(non_snake_case)]
        struct $sname { $($f: $t,)* }
        fn $ctor() -> ($sname, $sname) {
            let l = libs();
            unsafe {
                $( let $f = { let (a, b) = l.sym::<$t>($n); (*a, *b) }; )*
                ( $sname { $($f: $f.0,)* }, $sname { $($f: $f.1,)* } )
            }
        }
    };
}

api!(Cs, capi, {
    createStream: FnVoidToPtr = "LZ4_createStream",
    freeStream: FnFreePtr = "LZ4_freeStream",
    resetStream: FnVoidP = "LZ4_resetStream",
    resetStream_fast: FnVoidP = "LZ4_resetStream_fast",
    loadDict: FnLoadDict = "LZ4_loadDict",
    loadDictSlow: FnLoadDict = "LZ4_loadDictSlow",
    attach_dictionary: FnAttach = "LZ4_attach_dictionary",
    compress_fast_continue: FnCfc = "LZ4_compress_fast_continue",
    saveDict: FnSaveDict = "LZ4_saveDict",
    compress_forceExtDict: FnCc4 = "LZ4_compress_forceExtDict",
    compressBound: FnCompressBound = "LZ4_compressBound",
    sizeofState: FnVoidToInt = "LZ4_sizeofState",
    sizeofStreamState: FnVoidToInt = "LZ4_sizeofStreamState",
    compress: FnDecompressFast = "LZ4_compress",
    compress_limitedOutput: FnDecompressSafe = "LZ4_compress_limitedOutput",
    compress_withState: FnCc4 = "LZ4_compress_withState",
    compress_limitedOutput_withState: FnCc5 = "LZ4_compress_limitedOutput_withState",
    compress_continue: FnCc4 = "LZ4_compress_continue",
    compress_limitedOutput_continue: FnCc5 = "LZ4_compress_limitedOutput_continue",
    create: FnCreateLegacy = "LZ4_create",
    slideInputBuffer: FnPtrToPtr = "LZ4_slideInputBuffer",
    resetStreamState: FnResetSS = "LZ4_resetStreamState",
});

api!(Ds, dapi, {
    createStreamDecode: FnVoidToPtr = "LZ4_createStreamDecode",
    freeStreamDecode: FnFreePtr = "LZ4_freeStreamDecode",
    setStreamDecode: FnCc5o = "LZ4_setStreamDecode",
    safe_continue: FnCc5 = "LZ4_decompress_safe_continue",
    fast_continue: FnCc4 = "LZ4_decompress_fast_continue",
    decoderRingBufferSize: FnIntToInt = "LZ4_decoderRingBufferSize",
    safe_usingDict: FnSud = "LZ4_decompress_safe_usingDict",
    safe_partial_usingDict: FnSpud = "LZ4_decompress_safe_partial_usingDict",
    fast_usingDict: FnFud = "LZ4_decompress_fast_usingDict",
    safe_forceExtDict: FnSfed = "LZ4_decompress_safe_forceExtDict",
    safe_partial_forceExtDict: FnSpfed = "LZ4_decompress_safe_partial_forceExtDict",
    safe_withPrefix64k: FnDecompressSafe = "LZ4_decompress_safe_withPrefix64k",
    fast_withPrefix64k: FnDecompressFast = "LZ4_decompress_fast_withPrefix64k",
    safe: FnDecompressSafe = "LZ4_decompress_safe",
});

/// `LZ4_setStreamDecode(state, dict, dictSize)`
type FnCc5o = FnLoadDict;

// ---------------------------------------------------------------------------
// Buffers with a sentinel guard region
// ---------------------------------------------------------------------------

const SENT: u8 = 0xCD;
const GUARD: usize = 64;

fn dbuf(cap: usize) -> Vec<u8> {
    vec![SENT; cap + GUARD]
}

/// Compare a pair of `(return value, destination buffer)` results.
#[track_caller]
fn check(ctx: &str, a: c_int, b: c_int, dc: &[u8], dr: &[u8], cap: usize) {
    same_int_and_bytes(ctx, a, b, dc, dr);
    assert!(
        dc[cap..].iter().all(|&x| x == SENT),
        "{ctx}: C wrote past dstCapacity {cap}"
    );
    assert!(
        dr[cap..].iter().all(|&x| x == SENT),
        "{ctx}: Rust wrote past dstCapacity {cap}"
    );
    same_full_buffers(ctx, dc, dr);
}

// ---------------------------------------------------------------------------
// Randomised block-size / shape pickers
// ---------------------------------------------------------------------------

fn one_size(rng: &mut Rng, max: usize) -> usize {
    let v = match rng.below(13) {
        0 => 0,
        1 => rng.range(1, 12),
        2 => rng.range(13, 64),
        3 => rng.range(3900, 4300), // around the 4 KB dictCtx / prepareTable threshold
        4 => 4096,
        5 => rng.range(64000, 66000), // around the 64 KB window
        6 => 65536,
        7 => rng.range(65537, 70000), // > 64 KB
        8 => rng.range(100, 3000),
        9 => rng.range(5000, 40000),
        10 => rng.range(1, 8),
        11 => 3,
        _ => rng.range(200, 20000),
    };
    v.min(max)
}

/// `nblocks` sizes, each `<= max_block`, with the total capped at `cap_total`.
fn sizes(rng: &mut Rng, nblocks: usize, max_block: usize, cap_total: usize) -> Vec<usize> {
    let mut v = Vec::with_capacity(nblocks);
    let mut tot = 0usize;
    for _ in 0..nblocks {
        let n = one_size(rng, max_block).min(cap_total.saturating_sub(tot));
        tot += n;
        v.push(n);
    }
    v
}

fn shape(rng: &mut Rng) -> Shape {
    ALL_SHAPES[rng.below(ALL_SHAPES.len())]
}

/// Concatenate `sizes.len()` freshly generated blocks into one flat buffer.
fn flat_input(rng: &mut Rng, sz: &[usize]) -> Vec<u8> {
    let mut v = Vec::with_capacity(sz.iter().sum::<usize>() + 8);
    for &n in sz {
        let s = shape(rng);
        v.extend_from_slice(&gen(rng, s, n));
    }
    v
}

// ---------------------------------------------------------------------------
// Lockstep compression drivers
// ---------------------------------------------------------------------------

/// Drive `LZ4_compress_fast_continue` over `sz` contiguous slices of `src`
/// (contiguous-prefix continuation) in lockstep. Returns (C blocks, Rust blocks).
#[allow(clippy::too_many_arguments)]
unsafe fn chain_prefix(
    cc: &Cs,
    rc: &Cs,
    src: &[u8],
    sz: &[usize],
    accel: c_int,
    ctx: &str,
) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let sc = (cc.createStream)();
    let sr = (rc.createStream)();
    assert!(!sc.is_null() && !sr.is_null(), "{ctx}: createStream failed");
    let out = chain_prefix_on(cc, rc, sc, sr, src, 0, sz, accel, ctx);
    assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr), "{ctx}: freeStream");
    out
}

/// Same, but on caller-supplied streams starting at `start` inside `src`.
#[allow(clippy::too_many_arguments)]
unsafe fn chain_prefix_on(
    cc: &Cs,
    rc: &Cs,
    sc: *mut c_void,
    sr: *mut c_void,
    src: &[u8],
    start: usize,
    sz: &[usize],
    accel: c_int,
    ctx: &str,
) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut bc = Vec::with_capacity(sz.len());
    let mut br = Vec::with_capacity(sz.len());
    let mut off = start;
    for (i, &n) in sz.iter().enumerate() {
        let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
        assert_eq!(
            (cc.compressBound)(n as c_int),
            (rc.compressBound)(n as c_int),
            "{ctx}: compressBound({n})"
        );
        let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
        let p = src.as_ptr().add(off) as *const c_char;
        let a = (cc.compress_fast_continue)(
            sc,
            p,
            dc.as_mut_ptr() as *mut c_char,
            n as c_int,
            cap as c_int,
            accel,
        );
        let b = (rc.compress_fast_continue)(
            sr,
            p,
            dr.as_mut_ptr() as *mut c_char,
            n as c_int,
            cap as c_int,
            accel,
        );
        let c = format!("{ctx}: LZ4_compress_fast_continue block {i} len {n} accel {accel}");
        check(&c, a, b, &dc, &dr, cap);
        assert!(a > 0, "{c}: unexpected failure (ret {a})");
        dc.truncate(a as usize);
        dr.truncate(b as usize);
        bc.push(dc);
        br.push(dr);
        off += n;
    }
    (bc, br)
}

// ---------------------------------------------------------------------------
// Lockstep decompression drivers (with cross-decompression built in)
// ---------------------------------------------------------------------------

/// Decode a chain into one flat contiguous destination (prefix continuation).
/// C blocks are given to the Rust decoder and vice versa.
unsafe fn decode_flat(
    cd: &Ds,
    rd: &Ds,
    bc: &[Vec<u8>],
    br: &[Vec<u8>],
    sz: &[usize],
    expect: &[u8],
    ctx: &str,
) {
    let total: usize = sz.iter().sum();
    let sc = (cd.createStreamDecode)();
    let sr = (rd.createStreamDecode)();
    let (mut oc, mut or) = (dbuf(total), dbuf(total));
    let mut off = 0usize;
    for (i, &n) in sz.iter().enumerate() {
        let a = (cd.safe_continue)(
            sc,
            br[i].as_ptr() as *const c_char, // Rust-produced block -> C decoder
            oc.as_mut_ptr().add(off) as *mut c_char,
            br[i].len() as c_int,
            n as c_int,
        );
        let b = (rd.safe_continue)(
            sr,
            bc[i].as_ptr() as *const c_char, // C-produced block -> Rust decoder
            or.as_mut_ptr().add(off) as *mut c_char,
            bc[i].len() as c_int,
            n as c_int,
        );
        let c = format!("{ctx}: LZ4_decompress_safe_continue block {i} len {n}");
        assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
        assert_eq!(a, n as c_int, "{c}: wrong decoded size (got {a})");
        off += n;
    }
    assert_eq!((cd.freeStreamDecode)(sc), (rd.freeStreamDecode)(sr));
    same_full_buffers(&format!("{ctx}: flat decode buffers"), &oc, &or);
    assert_eq!(&oc[..total], expect, "{ctx}: C round-trip mismatch");
    assert_eq!(&or[..total], expect, "{ctx}: Rust round-trip mismatch");
    assert!(oc[total..].iter().all(|&x| x == SENT), "{ctx}: guard clobbered");
}

// ===========================================================================
// Row 35 - createStream / freeStream / freeStream(NULL) / resetStream
// ===========================================================================

#[test]
fn row_35_create_free_reset_and_caller_provided_stream_buffer() {
    let (cc, rc) = capi();
    let mut rng = Rng::new(35);
    unsafe {
        assert_eq!(
            (cc.sizeofState)(),
            (rc.sizeofState)(),
            "LZ4_sizeofState differs"
        );
        assert_eq!(
            (cc.sizeofStreamState)(),
            (rc.sizeofStreamState)(),
            "LZ4_sizeofStreamState differs"
        );
        // free(NULL) is explicitly supported and must return 0.
        assert_eq!(
            (cc.freeStream)(std::ptr::null_mut()),
            (rc.freeStream)(std::ptr::null_mut()),
            "LZ4_freeStream(NULL)"
        );
        assert_eq!((cc.freeStream)(std::ptr::null_mut()), 0);

        let state_bytes = (cc.sizeofStreamState)() as usize;
        assert!(state_bytes >= 16416, "unexpected stream size {state_bytes}");

        for it in 0..2400 {
            let nb = rng.range(1, 30);
            let sz = sizes(&mut rng, nb, 70000, 300_000);
            let src = flat_input(&mut rng, &sz);
            let ctx = format!("row35/createStream iter {it}");

            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
            let (cd, rd) = dapi();
            decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);

            // A zeroed, 8-byte-aligned caller buffer is a valid LZ4_stream_t.
            // One buffer per library -- layouts are independent.
            let mut zc = vec![0u64; (state_bytes + 7) / 8];
            let mut zr = vec![0u64; (state_bytes + 7) / 8];
            let (pc, pr) = (
                zc.as_mut_ptr() as *mut c_void,
                zr.as_mut_ptr() as *mut c_void,
            );
            let ctx2 = format!("row35/zeroed-caller-buffer iter {it}");
            let (bc2, br2) = chain_prefix_on(&cc, &rc, pc, pr, &src, 0, &sz, 1, &ctx2);
            decode_flat(&cd, &rd, &bc2, &br2, &sz, &src, &ctx2);
            // The zeroed buffer must produce the exact same chain as createStream.
            assert_eq!(bc, bc2, "{ctx2}: zeroed buffer != createStream (C)");
            assert_eq!(br, br2, "{ctx2}: zeroed buffer != createStream (Rust)");

            // Deprecated LZ4_resetStream on an already-used stream.
            (cc.resetStream)(pc);
            (rc.resetStream)(pr);
            let ctx3 = format!("row35/resetStream iter {it}");
            let (bc3, br3) = chain_prefix_on(&cc, &rc, pc, pr, &src, 0, &sz, 1, &ctx3);
            decode_flat(&cd, &rd, &bc3, &br3, &sz, &src, &ctx3);
            assert_eq!(bc, bc3, "{ctx3}: resetStream != fresh stream (C)");
        }
    }
}

// ===========================================================================
// Row 36 - LZ4_resetStream_fast reuse of a stream that already compressed
// ===========================================================================

#[test]
fn row_36_reset_stream_fast_reuse() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(36);
    unsafe {
        for it in 0..1800 {
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();

            // resetStream_fast on a brand-new (clearedTable) stream is a no-op path.
            (cc.resetStream_fast)(sc);
            (rc.resetStream_fast)(sr);

            for round in 0..6 {
                let nb = rng.range(1, 12);
                let sz = sizes(&mut rng, nb, 70000, 200_000);
                let src = flat_input(&mut rng, &sz);
                let ctx = format!("row36 iter {it} round {round}");
                let (bc, br) = chain_prefix_on(&cc, &rc, sc, sr, &src, 0, &sz, 1, &ctx);
                decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
                // Reuse: prepareTable(byU32) + a 64 KB offset gap.
                (cc.resetStream_fast)(sc);
                (rc.resetStream_fast)(sr);
            }
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
        }
    }
}

// ===========================================================================
// Rows 37/38/39 - LZ4_loadDict / LZ4_loadDictSlow
// ===========================================================================

/// Load `dict` with `f` into both libraries, compare the return value, then
/// compress `nblk` separate-buffer blocks and cross-decode them.
#[allow(clippy::too_many_arguments)]
unsafe fn load_dict_case(
    cc: &Cs,
    rc: &Cs,
    cd: &Ds,
    rd: &Ds,
    slow: bool,
    dict: &[u8],
    rng: &mut Rng,
    ctx: &str,
) -> c_int {
    let sc = (cc.createStream)();
    let sr = (rc.createStream)();
    let f = if slow {
        (cc.loadDictSlow, rc.loadDictSlow)
    } else {
        (cc.loadDict, rc.loadDict)
    };
    let dp = if dict.is_empty() {
        std::ptr::null()
    } else {
        dict.as_ptr() as *const c_char
    };
    let a = (f.0)(sc, dp, dict.len() as c_int);
    let b = (f.1)(sr, dp, dict.len() as c_int);
    assert_eq!(
        a, b,
        "{ctx}: loadDict{} return mismatch (C={a} Rust={b})",
        if slow { "Slow" } else { "" }
    );

    // Blocks whose head repeats the tail of the dictionary, so the encoder
    // really references the dictionary (and straddles the dict/block boundary).
    let effective = a.max(0) as usize;
    let tail = &dict[dict.len() - effective.min(dict.len())..];
    let nblk = rng.range(1, 6);
    let mut sz = Vec::new();
    let mut bufs: Vec<Vec<u8>> = Vec::new();
    for _ in 0..nblk {
        let n = one_size(rng, 40000);
        let mut v = Vec::with_capacity(n + 8);
        if !tail.is_empty() {
            while v.len() < n {
                let take = (n - v.len()).min(tail.len());
                v.extend_from_slice(&tail[tail.len() - take..]);
            }
        }
        v.resize(n, 0);
        let s = shape(rng);
        let extra = gen(rng, s, n / 4);
        for (i, x) in extra.iter().enumerate() {
            if n / 2 + i < n {
                v[n / 2 + i] = *x;
            }
        }
        sz.push(n);
        bufs.push(v);
    }

    // Compress each block from its own separate buffer (extDict mode).
    let mut bc = Vec::new();
    let mut br = Vec::new();
    for (i, buf) in bufs.iter().enumerate() {
        let n = buf.len();
        let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
        let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
        let p = buf.as_ptr() as *const c_char;
        let x = (cc.compress_fast_continue)(
            sc,
            p,
            dc.as_mut_ptr() as *mut c_char,
            n as c_int,
            cap as c_int,
            1,
        );
        let y = (rc.compress_fast_continue)(
            sr,
            p,
            dr.as_mut_ptr() as *mut c_char,
            n as c_int,
            cap as c_int,
            1,
        );
        check(&format!("{ctx}: block {i} len {n}"), x, y, &dc, &dr, cap);
        assert!(x > 0);
        dc.truncate(x as usize);
        dr.truncate(y as usize);
        bc.push(dc);
        br.push(dr);
    }
    assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));

    // Decode: setStreamDecode(effective dict tail), then one dst buffer per
    // block so every step is a buffer switch (forceExtDict).
    let sdc = (cd.createStreamDecode)();
    let sdr = (rd.createStreamDecode)();
    let dtail = &dict[dict.len() - effective.min(dict.len())..];
    let dtp = if dtail.is_empty() {
        std::ptr::null()
    } else {
        dtail.as_ptr() as *const c_char
    };
    assert_eq!(
        (cd.setStreamDecode)(sdc, dtp, dtail.len() as c_int),
        (rd.setStreamDecode)(sdr, dtp, dtail.len() as c_int),
        "{ctx}: setStreamDecode"
    );
    let mut outs_c: Vec<Vec<u8>> = Vec::new();
    let mut outs_r: Vec<Vec<u8>> = Vec::new();
    for (i, &n) in sz.iter().enumerate() {
        outs_c.push(dbuf(n));
        outs_r.push(dbuf(n));
        let (oc, or) = (outs_c.last_mut().unwrap(), outs_r.last_mut().unwrap());
        let a = (cd.safe_continue)(
            sdc,
            br[i].as_ptr() as *const c_char,
            oc.as_mut_ptr() as *mut c_char,
            br[i].len() as c_int,
            n as c_int,
        );
        let b = (rd.safe_continue)(
            sdr,
            bc[i].as_ptr() as *const c_char,
            or.as_mut_ptr() as *mut c_char,
            bc[i].len() as c_int,
            n as c_int,
        );
        let c = format!("{ctx}: decode block {i} len {n}");
        assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
        assert_eq!(a, n as c_int, "{c}: wrong size {a}");
        assert_eq!(&oc[..n], &bufs[i][..], "{c}: C round-trip mismatch");
        assert_eq!(&or[..n], &bufs[i][..], "{c}: Rust round-trip mismatch");
    }
    assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
    a
}

fn dict_sizes_37_39() -> Vec<usize> {
    vec![0, 3, 7, 8, 9, 100, 4096, 32768, 65535, 65536, 65537, 100 * 1024]
}

#[test]
fn row_37_load_dict_zero_and_below_hash_unit() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(37);
    unsafe {
        for &n in &[0usize, 1, 2, 3, 4, 5, 6, 7] {
            for s in ALL_SHAPES {
                let dict = gen(&mut rng, s, n);
                let ctx = format!("row37 loadDict size {n} shape {s:?}");
                let r = load_dict_case(&cc, &rc, &cd, &rd, false, &dict, &mut rng, &ctx);
                assert_eq!(r, 0, "{ctx}: expected 0 (dictSize < HASH_UNIT), got {r}");
            }
        }
    }
}

#[test]
fn row_38_load_dict_64k_boundary_and_oversized() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(38);
    unsafe {
        for &n in &[8usize, 9, 1000, 32768, 65535, 65536, 65537, 100 * 1024, 200 * 1024] {
            for s in [Shape::TextLike, Shape::Compressible, Shape::Incompressible] {
                let dict = gen(&mut rng, s, n);
                let ctx = format!("row38 loadDict size {n} shape {s:?}");
                let r = load_dict_case(&cc, &rc, &cd, &rd, false, &dict, &mut rng, &ctx);
                assert_eq!(
                    r as usize,
                    n.min(65536),
                    "{ctx}: loadDict return (got {r})"
                );
            }
        }
    }
}

#[test]
fn row_39_load_dict_slow() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(39);
    unsafe {
        for &n in &dict_sizes_37_39() {
            for s in [Shape::TextLike, Shape::Periodic, Shape::Incompressible] {
                let dict = gen(&mut rng, s, n);
                let ctx = format!("row39 loadDictSlow size {n} shape {s:?}");
                let r = load_dict_case(&cc, &rc, &cd, &rd, true, &dict, &mut rng, &ctx);
                let want = if n < 8 { 0 } else { n.min(65536) as c_int };
                assert_eq!(r, want, "{ctx}: loadDictSlow return");
            }
        }
        // Fast vs slow must agree on the reported dictionary size.
        for &n in &[8usize, 4096, 65536, 100 * 1024] {
            let dict = gen(&mut rng, Shape::TextLike, n);
            let ctx = format!("row39 fast-vs-slow {n}");
            let a = load_dict_case(&cc, &rc, &cd, &rd, false, &dict, &mut rng, &ctx);
            let b = load_dict_case(&cc, &rc, &cd, &rd, true, &dict, &mut rng, &ctx);
            assert_eq!(a, b, "{ctx}: loadDict/loadDictSlow return differ");
        }
    }
}

// ===========================================================================
// Rows 40/41 - contiguous-prefix continuation
// ===========================================================================

#[test]
fn row_40_prefix_continuation_dict_small() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(40);
    unsafe {
        // Many small contiguous blocks: dictSize stays < 64 KB while
        // currentOffset > dictSize  =>  withPrefix64k + dictSmall.
        for it in 0..4500 {
            let nb = rng.range(2, 30);
            let sz: Vec<usize> = (0..nb).map(|_| rng.range(0, 2500)).collect();
            let total: usize = sz.iter().sum();
            assert!(total < 65536);
            let src = flat_input(&mut rng, &sz);
            let ctx = format!("row40 iter {it}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
            decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
        }
        // Same, but the stream already carries a 64 KB currentOffset gap from
        // LZ4_loadDict, guaranteeing dictSize < currentOffset from block 0.
        for it in 0..1500 {
            let dn = rng.range(8, 60000);
            let dict = gen(&mut rng, Shape::TextLike, dn);
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            assert_eq!(
                (cc.loadDict)(sc, dict.as_ptr() as *const c_char, dict.len() as c_int),
                (rc.loadDict)(sr, dict.as_ptr() as *const c_char, dict.len() as c_int)
            );
            // src laid immediately after a copy of the dict, so block 0 is
            // *not* a prefix continuation but block 1.. are.
            let nb = rng.range(2, 10);
            let sz: Vec<usize> = (0..nb).map(|_| rng.range(1, 3000)).collect();
            let src = flat_input(&mut rng, &sz);
            let ctx = format!("row40/loadDict iter {it}");
            let (bc, br) = chain_prefix_on(&cc, &rc, sc, sr, &src, 0, &sz, 1, &ctx);
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
            // Decode with the dictionary announced up front.
            let sdc = (cd.createStreamDecode)();
            let sdr = (rd.createStreamDecode)();
            (cd.setStreamDecode)(sdc, dict.as_ptr() as *const c_char, dict.len() as c_int);
            (rd.setStreamDecode)(sdr, dict.as_ptr() as *const c_char, dict.len() as c_int);
            let total: usize = sz.iter().sum();
            let (mut oc, mut or) = (dbuf(total), dbuf(total));
            let mut off = 0;
            for (i, &n) in sz.iter().enumerate() {
                let a = (cd.safe_continue)(
                    sdc,
                    br[i].as_ptr() as *const c_char,
                    oc.as_mut_ptr().add(off) as *mut c_char,
                    br[i].len() as c_int,
                    n as c_int,
                );
                let b = (rd.safe_continue)(
                    sdr,
                    bc[i].as_ptr() as *const c_char,
                    or.as_mut_ptr().add(off) as *mut c_char,
                    bc[i].len() as c_int,
                    n as c_int,
                );
                assert_eq!(a, b, "{ctx}: decode block {i}");
                assert_eq!(a, n as c_int, "{ctx}: decode block {i} size");
                off += n;
            }
            assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
            same_full_buffers(&ctx, &oc, &or);
            assert_eq!(&oc[..total], &src[..]);
        }
    }
}

#[test]
fn row_41_prefix_continuation_no_dict_issue_and_first_block() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(41);
    unsafe {
        // Blocks large enough that the accumulated prefix passes 64 KB, so the
        // stream crosses dictSmall -> noDictIssue mid-chain. Block 0 is the
        // dictSize == 0 "very first block" case.
        for it in 0..2400 {
            let nb = rng.range(1, 20);
            let sz = sizes(&mut rng, nb, 70000, 400_000);
            let src = flat_input(&mut rng, &sz);
            let ctx = format!("row41 iter {it}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
            decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
        }
        // Deterministic: one 100 KB block, then contiguous small blocks.
        let mut sz = vec![100 * 1024];
        sz.extend([1usize, 0, 13, 12, 4096, 65536, 3]);
        let src = flat_input(&mut rng, &sz);
        let ctx = "row41 deterministic";
        let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, ctx);
        decode_flat(&cd, &rd, &bc, &br, &sz, &src, ctx);
    }
}

// ===========================================================================
// Row 42 - separate src buffer after LZ4_loadDict => usingExtDict
// ===========================================================================

#[test]
fn row_42_ext_dict_after_load_dict() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(42);
    unsafe {
        for &dn in &[8usize, 100, 4096, 32768, 65535, 65536, 100 * 1024] {
            for s in [Shape::TextLike, Shape::Compressible] {
                let dict = gen(&mut rng, s, dn);
                let ctx = format!("row42 dict {dn} shape {s:?}");
                let r = load_dict_case(&cc, &rc, &cd, &rd, false, &dict, &mut rng, &ctx);
                assert_eq!(r as usize, dn.min(65536));
            }
        }
    }
}

// ===========================================================================
// Row 43 - alternating double buffer (each block < 64 KB, buffers separated)
// ===========================================================================

/// A single allocation holding two regions separated by a 16-byte gap, so that
/// the encoder's overlap test is fully deterministic.
struct Double {
    mem: Vec<u8>,
    cap: usize,
}

impl Double {
    fn new(cap: usize) -> Double {
        Double {
            mem: vec![0u8; 2 * cap + 16],
            cap,
        }
    }
    fn region(&self, which: usize) -> usize {
        if which == 0 {
            0
        } else {
            self.cap + 16
        }
    }
}

#[test]
fn row_43_alternating_double_buffer() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(43);
    unsafe {
        for it in 0..3600 {
            let maxb = rng.range(64, 60000);
            let nb = rng.range(1, 30);
            let sz: Vec<usize> = (0..nb).map(|_| one_size(&mut rng, maxb)).collect();
            let mut enc = Double::new(maxb + 8);
            let mut orig: Vec<Vec<u8>> = Vec::new();

            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            let mut bc = Vec::new();
            let mut br = Vec::new();
            let ctx = format!("row43 iter {it} maxb {maxb}");
            for (i, &n) in sz.iter().enumerate() {
                let base = enc.region(i & 1);
                let s = shape(&mut rng);
                let data = gen(&mut rng, s, n);
                enc.mem[base..base + n].copy_from_slice(&data);
                orig.push(data);
                let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
                let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
                let p = enc.mem.as_ptr().add(base) as *const c_char;
                let a = (cc.compress_fast_continue)(
                    sc,
                    p,
                    dc.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (rc.compress_fast_continue)(
                    sr,
                    p,
                    dr.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                check(&format!("{ctx} block {i} len {n}"), a, b, &dc, &dr, cap);
                assert!(a > 0);
                dc.truncate(a as usize);
                dr.truncate(b as usize);
                bc.push(dc);
                br.push(dr);
            }
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));

            // Decode with a matching alternating double buffer per library.
            let mut oc = Double::new(maxb + 8 + GUARD);
            let mut or = Double::new(maxb + 8 + GUARD);
            oc.mem.iter_mut().for_each(|x| *x = SENT);
            or.mem.iter_mut().for_each(|x| *x = SENT);
            let sdc = (cd.createStreamDecode)();
            let sdr = (rd.createStreamDecode)();
            for (i, &n) in sz.iter().enumerate() {
                let base = oc.region(i & 1);
                let a = (cd.safe_continue)(
                    sdc,
                    br[i].as_ptr() as *const c_char,
                    oc.mem.as_mut_ptr().add(base) as *mut c_char,
                    br[i].len() as c_int,
                    n as c_int,
                );
                let b = (rd.safe_continue)(
                    sdr,
                    bc[i].as_ptr() as *const c_char,
                    or.mem.as_mut_ptr().add(base) as *mut c_char,
                    bc[i].len() as c_int,
                    n as c_int,
                );
                let c = format!("{ctx}: decode block {i} len {n}");
                assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                assert_eq!(a, n as c_int, "{c}");
                assert_eq!(&oc.mem[base..base + n], &orig[i][..], "{c}: C round-trip");
                assert_eq!(&or.mem[base..base + n], &orig[i][..], "{c}: Rust round-trip");
            }
            assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
        }
    }
}

// ===========================================================================
// Row 44 - encoder ring buffer < 64 KB where src overlaps the dictionary
// ===========================================================================

#[test]
fn row_44_ring_buffer_overlapping_dictionary() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(44);
    unsafe {
        for it in 0..3600 {
            // Small ring + large blocks so the overlap recompute really fires.
            let ring_size = rng.range(2000, 60000);
            // A block must never *fully* cover the previous block, otherwise the
            // encoder's dictionary is clobbered without the overlap recompute
            // firing (that is outside the documented ring-buffer contract).
            // n <= ring_size/2 makes full coverage impossible while still
            // producing plenty of partial overlaps.
            let maxb = ring_size / 2;
            let nb = rng.range(3, 30);
            let mut ring = vec![0u8; ring_size];
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            let mut bc = Vec::new();
            let mut br = Vec::new();
            let mut sz = Vec::new();
            let mut logical: Vec<u8> = Vec::new();
            let mut pos = 0usize;
            let mut prev_end = 0usize;
            let ctx = format!("row44 iter {it} ring {ring_size}");
            for i in 0..nb {
                // Every few blocks, aim the block end 2 bytes short of the
                // previous block end so the recomputed dictSize drops below 4.
                let mut n = if i % 5 == 4 && prev_end >= 6 {
                    prev_end - 2
                } else {
                    rng.range(1, maxb)
                };
                n = n.min(maxb).max(1);
                if pos + n > ring_size {
                    pos = 0;
                }
                if pos + n > ring_size {
                    n = ring_size - pos;
                }
                if n == 0 {
                    continue;
                }
                let s = shape(&mut rng);
                let data = gen(&mut rng, s, n);
                ring[pos..pos + n].copy_from_slice(&data);
                logical.extend_from_slice(&data);
                let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
                let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
                let p = ring.as_ptr().add(pos) as *const c_char;
                let a = (cc.compress_fast_continue)(
                    sc,
                    p,
                    dc.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (rc.compress_fast_continue)(
                    sr,
                    p,
                    dr.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                check(
                    &format!("{ctx} block {i} len {n} pos {pos}"),
                    a,
                    b,
                    &dc,
                    &dr,
                    cap,
                );
                assert!(a > 0);
                dc.truncate(a as usize);
                dr.truncate(b as usize);
                bc.push(dc);
                br.push(dr);
                sz.push(n);
                prev_end = pos + n;
                pos += n;
            }
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
            // The encoder's window is a suffix of the logical stream, so a flat
            // decode buffer (full prefix) always satisfies the references.
            decode_flat(&cd, &rd, &bc, &br, &sz, &logical, &ctx);
        }
    }
}

// ===========================================================================
// Row 45 - previous dictSize < 4 with a non-prefix src
// ===========================================================================

#[test]
fn row_45_tiny_previous_dict_discarded() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(45);
    unsafe {
        for it in 0..3600 {
            let tiny = rng.range(1, 3); // 1..3 bytes -> dictSize < 4
            let dn = rng.range(8, 40000);
            let dict = gen(&mut rng, Shape::TextLike, dn);
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            assert_eq!(
                (cc.loadDict)(sc, dict.as_ptr() as *const c_char, dict.len() as c_int),
                (rc.loadDict)(sr, dict.as_ptr() as *const c_char, dict.len() as c_int)
            );
            let ctx = format!("row45 iter {it} tiny {tiny}");
            // block 0: tiny, from its own buffer  -> dictSize becomes 1..3
            // block 1: normal, from another buffer -> tiny dict is discarded
            let mut sz = vec![tiny];
            let mut bufs = vec![gen(&mut rng, Shape::Degenerate, tiny)];
            for _ in 0..rng.range(1, 4) {
                let n = one_size(&mut rng, 30000).max(1);
                sz.push(n);
                let sh = shape(&mut rng);
                bufs.push(gen(&mut rng, sh, n));
            }
            let mut bc = Vec::new();
            let mut br = Vec::new();
            for (i, buf) in bufs.iter().enumerate() {
                let n = buf.len();
                let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
                let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
                let p = buf.as_ptr() as *const c_char;
                let a = (cc.compress_fast_continue)(
                    sc,
                    p,
                    dc.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (rc.compress_fast_continue)(
                    sr,
                    p,
                    dr.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                check(&format!("{ctx} block {i} len {n}"), a, b, &dc, &dr, cap);
                dc.truncate(a as usize);
                dr.truncate(b as usize);
                bc.push(dc);
                br.push(dr);
            }
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));

            // Decode: dict announced, then one buffer per block.
            let sdc = (cd.createStreamDecode)();
            let sdr = (rd.createStreamDecode)();
            (cd.setStreamDecode)(sdc, dict.as_ptr() as *const c_char, dict.len() as c_int);
            (rd.setStreamDecode)(sdr, dict.as_ptr() as *const c_char, dict.len() as c_int);
            let mut oc: Vec<Vec<u8>> = Vec::new();
            let mut or: Vec<Vec<u8>> = Vec::new();
            for (i, &n) in sz.iter().enumerate() {
                oc.push(dbuf(n));
                or.push(dbuf(n));
                let a = (cd.safe_continue)(
                    sdc,
                    br[i].as_ptr() as *const c_char,
                    oc[i].as_mut_ptr() as *mut c_char,
                    br[i].len() as c_int,
                    n as c_int,
                );
                let b = (rd.safe_continue)(
                    sdr,
                    bc[i].as_ptr() as *const c_char,
                    or[i].as_mut_ptr() as *mut c_char,
                    bc[i].len() as c_int,
                    n as c_int,
                );
                let c = format!("{ctx}: decode block {i} len {n}");
                assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                assert_eq!(a, n as c_int, "{c}");
                assert_eq!(&oc[i][..n], &bufs[i][..], "{c}: C round-trip");
                assert_eq!(&or[i][..n], &bufs[i][..], "{c}: Rust round-trip");
            }
            assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
        }
    }
}

// ===========================================================================
// Rows 46/47/48 - LZ4_attach_dictionary
// ===========================================================================

/// `attach` variant: each library builds its **own** dictionary stream with its
/// own `LZ4_createStream`/`LZ4_loadDict`, then attaches it to its own working
/// stream. Blocks come from separate buffers, so the dictCtx paths are used.
#[allow(clippy::too_many_arguments)]
unsafe fn attach_case(
    cc: &Cs,
    rc: &Cs,
    cd: &Ds,
    rd: &Ds,
    dict: &[u8],
    blocks: &[Vec<u8>],
    attach_null: bool,
    decode: bool,
    ctx: &str,
) {
    let dc_stream = (cc.createStream)();
    let dr_stream = (rc.createStream)();
    let dp = if dict.is_empty() {
        std::ptr::null()
    } else {
        dict.as_ptr() as *const c_char
    };
    let eff_c = (cc.loadDict)(dc_stream, dp, dict.len() as c_int);
    let eff_r = (rc.loadDict)(dr_stream, dp, dict.len() as c_int);
    assert_eq!(eff_c, eff_r, "{ctx}: loadDict on dictionary stream");

    let wc = (cc.createStream)();
    let wr = (rc.createStream)();
    if attach_null {
        // First attach the real dictionary (bumps currentOffset 0 -> 64 KB),
        // then unset it with dictionaryStream == NULL.
        (cc.attach_dictionary)(wc, dc_stream as *const c_void);
        (rc.attach_dictionary)(wr, dr_stream as *const c_void);
        (cc.attach_dictionary)(wc, std::ptr::null());
        (rc.attach_dictionary)(wr, std::ptr::null());
    } else {
        (cc.attach_dictionary)(wc, dc_stream as *const c_void);
        (rc.attach_dictionary)(wr, dr_stream as *const c_void);
    }

    let mut bc = Vec::new();
    let mut br = Vec::new();
    for (i, buf) in blocks.iter().enumerate() {
        let n = buf.len();
        let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
        let (mut dcb, mut drb) = (dbuf(cap), dbuf(cap));
        let p = if n == 0 {
            std::ptr::null()
        } else {
            buf.as_ptr() as *const c_char
        };
        let a = (cc.compress_fast_continue)(
            wc,
            p,
            dcb.as_mut_ptr() as *mut c_char,
            n as c_int,
            cap as c_int,
            1,
        );
        let b = (rc.compress_fast_continue)(
            wr,
            p,
            drb.as_mut_ptr() as *mut c_char,
            n as c_int,
            cap as c_int,
            1,
        );
        check(&format!("{ctx}: block {i} len {n}"), a, b, &dcb, &drb, cap);
        assert!(a > 0, "{ctx}: block {i} len {n} ret {a}");
        dcb.truncate(a as usize);
        drb.truncate(b as usize);
        bc.push(dcb);
        br.push(drb);
    }

    assert_eq!((cc.freeStream)(wc), (rc.freeStream)(wr), "{ctx}: freeStream");
    assert_eq!(
        (cc.freeStream)(dc_stream),
        (rc.freeStream)(dr_stream),
        "{ctx}: freeStream(dict)"
    );

    if !decode {
        return;
    }
    // The dictionary the encoder could see is the trailing `eff_c` bytes.
    let eff = eff_c.max(0) as usize;
    let dtail = &dict[dict.len() - eff.min(dict.len())..];
    let dtp = if dtail.is_empty() {
        std::ptr::null()
    } else {
        dtail.as_ptr() as *const c_char
    };
    let sdc = (cd.createStreamDecode)();
    let sdr = (rd.createStreamDecode)();
    assert_eq!(
        (cd.setStreamDecode)(sdc, dtp, dtail.len() as c_int),
        (rd.setStreamDecode)(sdr, dtp, dtail.len() as c_int)
    );
    let mut oc: Vec<Vec<u8>> = Vec::new();
    let mut or: Vec<Vec<u8>> = Vec::new();
    for (i, buf) in blocks.iter().enumerate() {
        let n = buf.len();
        oc.push(dbuf(n));
        or.push(dbuf(n));
        let a = (cd.safe_continue)(
            sdc,
            br[i].as_ptr() as *const c_char,
            oc[i].as_mut_ptr() as *mut c_char,
            br[i].len() as c_int,
            n as c_int,
        );
        let b = (rd.safe_continue)(
            sdr,
            bc[i].as_ptr() as *const c_char,
            or[i].as_mut_ptr() as *mut c_char,
            bc[i].len() as c_int,
            n as c_int,
        );
        let c = format!("{ctx}: decode block {i} len {n}");
        assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
        assert_eq!(a, n as c_int, "{c}: size");
        assert_eq!(&oc[i][..n], &buf[..], "{c}: C round-trip");
        assert_eq!(&or[i][..n], &buf[..], "{c}: Rust round-trip");
    }
    assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
}

/// Blocks whose content is drawn from the dictionary tail so the dictCtx hash
/// table is actually consulted.
fn dict_derived_blocks(rng: &mut Rng, dict: &[u8], sz: &[usize]) -> Vec<Vec<u8>> {
    sz.iter()
        .map(|&n| {
            let mut v = Vec::with_capacity(n + 8);
            if !dict.is_empty() {
                while v.len() < n {
                    let start = rng.below(dict.len());
                    let take = (dict.len() - start).min(n - v.len()).max(1);
                    v.extend_from_slice(&dict[start..start + take.min(dict.len() - start)]);
                }
            }
            v.resize(n, 0);
            let s = shape(rng);
            let noise = gen(rng, s, n / 8);
            for (i, x) in noise.iter().enumerate() {
                let at = (i * 7) % n.max(1);
                if at < n {
                    v[at] = *x;
                }
            }
            v
        })
        .collect()
}

#[test]
fn row_46_attach_dictionary_small_src_using_dict_ctx() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(46);
    unsafe {
        for &dn in &[8usize, 1000, 4096, 32768, 65536, 100 * 1024] {
            for it in 0..300 {
                let s = shape(&mut rng);
                let dict = gen(&mut rng, s, dn);
                // every block <= 4 KB  =>  usingDictCtx
                let nb = rng.range(1, 12);
                let sz: Vec<usize> = (0..nb).map(|_| rng.range(1, 4096)).collect();
                let blocks = dict_derived_blocks(&mut rng, &dict, &sz);
                let ctx = format!("row46 dict {dn} iter {it}");
                attach_case(&cc, &rc, &cd, &rd, &dict, &blocks, false, true, &ctx);
            }
        }
        // exactly at the 4 KB threshold (<= 4 KB is still usingDictCtx)
        let dict = gen(&mut rng, Shape::TextLike, 40000);
        for &n in &[4095usize, 4096] {
            let blocks = dict_derived_blocks(&mut rng, &dict, &[n, n, n]);
            let ctx = format!("row46 threshold {n}");
            attach_case(&cc, &rc, &cd, &rd, &dict, &blocks, false, true, &ctx);
        }
    }
}

#[test]
fn row_47_attach_dictionary_large_src_dict_ctx_memcpy() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(47);
    unsafe {
        for &dn in &[8usize, 1000, 4096, 32768, 65536, 100 * 1024] {
            for it in 0..300 {
                let s = shape(&mut rng);
                let dict = gen(&mut rng, s, dn);
                // every block > 4 KB  =>  dictCtx memcpy'd -> usingExtDict
                let nb = rng.range(1, 8);
                let sz: Vec<usize> = (0..nb).map(|_| rng.range(4097, 70000)).collect();
                let blocks = dict_derived_blocks(&mut rng, &dict, &sz);
                let ctx = format!("row47 dict {dn} iter {it}");
                attach_case(&cc, &rc, &cd, &rd, &dict, &blocks, false, true, &ctx);
            }
        }
        // 4097 = first size on the ">4 KB" side of the branch
        let dict = gen(&mut rng, Shape::TextLike, 65536);
        let blocks = dict_derived_blocks(&mut rng, &dict, &[4097, 4097, 4097]);
        attach_case(&cc, &rc, &cd, &rd, &dict, &blocks, false, true, "row47 threshold 4097");
        // mixed: alternate around the threshold inside one stream
        let sz = vec![100usize, 5000, 4096, 4097, 1, 65536, 3];
        let blocks = dict_derived_blocks(&mut rng, &dict, &sz);
        attach_case(&cc, &rc, &cd, &rd, &dict, &blocks, false, true, "row47 mixed");
    }
}

#[test]
fn row_48_attach_dictionary_null_empty_and_offset_bump() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(48);
    unsafe {
        // (a) dictionaryStream = NULL  =>  dictCtx unset
        for it in 0..900 {
            let dn = rng.range(8, 40000);
            let dict = gen(&mut rng, Shape::TextLike, dn);
            let nb = rng.range(1, 8);
            let sz: Vec<usize> = (0..nb).map(|_| one_size(&mut rng, 40000)).collect();
            let blocks: Vec<Vec<u8>> = sz
                .iter()
                .map(|&n| {
                    let s = shape(&mut rng);
                    gen(&mut rng, s, n)
                })
                .collect();
            let ctx = format!("row48/attach-NULL iter {it}");
            // No dictionary is reachable, so the decoder needs no dict.
            attach_case(&cc, &rc, &cd, &rd, &dict, &blocks, true, false, &ctx);
        }
        // (b) dictCtx with dictSize == 0  =>  not attached (but currentOffset
        //     is still bumped from 0 to 64 KB)
        for &dn in &[0usize, 1, 3, 7] {
            let dict = gen(&mut rng, Shape::Degenerate, dn);
            let sz = vec![1usize, 4096, 4097, 0, 30000];
            let blocks: Vec<Vec<u8>> = sz
                .iter()
                .map(|&n| {
                    let s = shape(&mut rng);
                    gen(&mut rng, s, n)
                })
                .collect();
            let ctx = format!("row48/empty-dictCtx dictSize {dn}");
            attach_case(&cc, &rc, &cd, &rd, &dict, &blocks, false, true, &ctx);
        }
        // (c) working stream with currentOffset != 0 before attach: compress a
        //     block first, then attach.
        for it in 0..900 {
            let dn = rng.range(8, 40000);
            let dict = gen(&mut rng, Shape::TextLike, dn);
            let dsc = (cc.createStream)();
            let dsr = (rc.createStream)();
            let dp = dict.as_ptr() as *const c_char;
            let eff = (cc.loadDict)(dsc, dp, dict.len() as c_int);
            assert_eq!(eff, (rc.loadDict)(dsr, dp, dict.len() as c_int));

            let wc = (cc.createStream)();
            let wr = (rc.createStream)();
            let warm = gen(&mut rng, Shape::TextLike, 5000);
            let cap = ((cc.compressBound)(5000) as usize).max(1);
            let (mut a1, mut b1) = (dbuf(cap), dbuf(cap));
            let x = (cc.compress_fast_continue)(
                wc,
                warm.as_ptr() as *const c_char,
                a1.as_mut_ptr() as *mut c_char,
                5000,
                cap as c_int,
                1,
            );
            let y = (rc.compress_fast_continue)(
                wr,
                warm.as_ptr() as *const c_char,
                b1.as_mut_ptr() as *mut c_char,
                5000,
                cap as c_int,
                1,
            );
            let ctx = format!("row48/warm-then-attach iter {it}");
            check(&format!("{ctx} warmup"), x, y, &a1, &b1, cap);
            // Now attach: currentOffset != 0, so it is left alone.
            (cc.attach_dictionary)(wc, dsc as *const c_void);
            (rc.attach_dictionary)(wr, dsr as *const c_void);
            let sz: Vec<usize> = vec![1000, 4096, 4097, 20000];
            let blocks = dict_derived_blocks(&mut rng, &dict, &sz);
            for (i, buf) in blocks.iter().enumerate() {
                let n = buf.len();
                let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
                let (mut dcb, mut drb) = (dbuf(cap), dbuf(cap));
                let p = buf.as_ptr() as *const c_char;
                let a = (cc.compress_fast_continue)(
                    wc,
                    p,
                    dcb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (rc.compress_fast_continue)(
                    wr,
                    p,
                    drb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                check(&format!("{ctx} block {i} len {n}"), a, b, &dcb, &drb, cap);
            }
            assert_eq!((cc.freeStream)(wc), (rc.freeStream)(wr));
            assert_eq!((cc.freeStream)(dsc), (rc.freeStream)(dsr));
        }
    }
}

// ===========================================================================
// Row 49 - LZ4_renormDictT (currentOffset + srcSize > 0x80000000)
// ===========================================================================

/// Bring `currentOffset` up to `65536 * (1 + k)` cheaply.
///
/// `LZ4_loadDict` with `dictSize < HASH_UNIT` returns early leaving
/// `tableType == clearedTable` and `currentOffset == 64 KB`; every subsequent
/// `LZ4_resetStream_fast` then skips the reset branch of `LZ4_prepareTable`
/// and simply adds another 64 KB.
unsafe fn bump_offset(cc: &Cs, rc: &Cs, sc: *mut c_void, sr: *mut c_void, k: usize, tiny: &[u8]) {
    let p = tiny.as_ptr() as *const c_char;
    assert_eq!(
        (cc.loadDict)(sc, p, tiny.len() as c_int),
        (rc.loadDict)(sr, p, tiny.len() as c_int)
    );
    for _ in 0..k {
        (cc.resetStream_fast)(sc);
        (rc.resetStream_fast)(sr);
    }
}

#[test]
fn row_49_renorm_dict_and_long_block_runs() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(49);
    let tiny = vec![7u8, 8, 9]; // dictSize 3 < HASH_UNIT
    unsafe {
        // (a) renorm on the very first block (currentOffset == 0x80000000).
        for s in ALL_SHAPES {
            let sz = vec![20000usize, 30000, 1, 4096, 0, 12, 65536];
            let src = {
                let mut v = Vec::new();
                for &n in &sz {
                    v.extend_from_slice(&gen(&mut rng, s, n));
                }
                v
            };
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            bump_offset(&cc, &rc, sc, sr, 32767, &tiny); // 65536 * 32768 == 0x80000000
            let ctx = format!("row49/renorm-first-block shape {s:?}");
            let (bc, br) = chain_prefix_on(&cc, &rc, sc, sr, &src, 0, &sz, 1, &ctx);
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
            decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
        }

        // (b) renorm mid-chain with a populated hash table, dictSize < 64 KB.
        for s in ALL_SHAPES {
            let sz = vec![40000usize, 30000, 5000, 1, 20000];
            let src = {
                let mut v = Vec::new();
                for &n in &sz {
                    v.extend_from_slice(&gen(&mut rng, s, n));
                }
                v
            };
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            bump_offset(&cc, &rc, sc, sr, 32766, &tiny); // 0x7FFF0000
            let ctx = format!("row49/renorm-mid-chain shape {s:?}");
            let (bc, br) = chain_prefix_on(&cc, &rc, sc, sr, &src, 0, &sz, 1, &ctx);
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
            decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
        }

        // (c) renorm with dictSize > 64 KB  =>  dictSize clamped to 64 KB and
        //     dictionary re-anchored at dictEnd - 64 KB.
        for s in ALL_SHAPES {
            let sz = vec![70000usize, 61073, 4096, 30000, 1];
            let src = {
                let mut v = Vec::new();
                for &n in &sz {
                    v.extend_from_slice(&gen(&mut rng, s, n));
                }
                v
            };
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            bump_offset(&cc, &rc, sc, sr, 32765, &tiny); // 0x7FFE0000
            let ctx = format!("row49/renorm-clamp shape {s:?}");
            let (bc, br) = chain_prefix_on(&cc, &rc, sc, sr, &src, 0, &sz, 1, &ctx);
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
            decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
        }

        // (d) a long run of blocks with a large cumulative offset (capped so the
        //     test stays fast): 400 blocks, ~3 MB cumulative.
        for s in [Shape::TextLike, Shape::Compressible, Shape::Incompressible] {
            let sz: Vec<usize> = (0..400).map(|_| 8000usize).collect();
            let src = {
                let mut v = Vec::new();
                for &n in &sz {
                    v.extend_from_slice(&gen(&mut rng, s, n));
                }
                v
            };
            let ctx = format!("row49/long-run shape {s:?}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
            decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
        }
    }
}

// ===========================================================================
// Row 50 - dstCapacity < LZ4_compressBound  +  acceleration on chained blocks
// ===========================================================================

#[test]
fn row_50_dst_too_small_and_acceleration() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(50);
    unsafe {
        // (a) acceleration variants over full chains (must round-trip).
        for &accel in &[0i32, 1, 2, 64, 65537, 65538, 1_000_000, -1, -100_000] {
            for it in 0..240 {
                let nb = rng.range(1, 20);
                let sz = sizes(&mut rng, nb, 70000, 250_000);
                let src = flat_input(&mut rng, &sz);
                let ctx = format!("row50/accel {accel} iter {it}");
                let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, accel, &ctx);
                decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
            }
        }

        // (b) a too-small dstCapacity mid-chain: expect 0 from both, then keep
        //     driving both (now-invalid) streams and compare every step.
        for it in 0..2700 {
            let nb = rng.range(3, 20);
            let sz: Vec<usize> = (0..nb).map(|_| rng.range(64, 40000)).collect();
            // incompressible, so a fraction of the bound really cannot fit
            let mut src = Vec::new();
            for &n in &sz {
                src.extend_from_slice(&gen_incompressible(&mut rng, n));
            }
            let fail_at = rng.below(nb);
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            let ctx = format!("row50/dst-too-small iter {it} fail_at {fail_at}");
            let mut off = 0usize;
            for (i, &n) in sz.iter().enumerate() {
                let bound = ((cc.compressBound)(n as c_int) as usize).max(1);
                let cap = if i == fail_at {
                    (n / 4).max(1)
                } else {
                    bound
                };
                let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
                let p = src.as_ptr().add(off) as *const c_char;
                let a = (cc.compress_fast_continue)(
                    sc,
                    p,
                    dc.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (rc.compress_fast_continue)(
                    sr,
                    p,
                    dr.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let c = format!("{ctx}: block {i} len {n} cap {cap}");
                check(&c, a, b, &dc, &dr, cap);
                if i == fail_at {
                    assert_eq!(a, 0, "{c}: expected failure (0), got {a}");
                }
                off += n;
            }
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
        }

        // (c) dstCapacity exactly bound-1 and exactly 1, over a chain.
        for it in 0..1500 {
            let nb = rng.range(2, 12);
            let sz: Vec<usize> = (0..nb).map(|_| one_size(&mut rng, 40000)).collect();
            let src = flat_input(&mut rng, &sz);
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            let ctx = format!("row50/tight-caps iter {it}");
            let mut off = 0usize;
            for (i, &n) in sz.iter().enumerate() {
                for cap in [
                    1usize,
                    ((cc.compressBound)(n as c_int) as usize).max(2) - 1,
                ] {
                    let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
                    let p = src.as_ptr().add(off) as *const c_char;
                    let a = (cc.compress_fast_continue)(
                        sc,
                        p,
                        dc.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                        1,
                    );
                    let b = (rc.compress_fast_continue)(
                        sr,
                        p,
                        dr.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                        1,
                    );
                    check(
                        &format!("{ctx}: block {i} len {n} cap {cap}"),
                        a,
                        b,
                        &dc,
                        &dr,
                        cap,
                    );
                }
                off += n;
            }
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
        }
    }
}

// ===========================================================================
// Row 51 - LZ4_saveDict and LZ4_compress_forceExtDict
// ===========================================================================

#[test]
fn row_51_save_dict_and_compress_force_ext_dict() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(51);
    unsafe {
        // ---- LZ4_saveDict ----
        for it in 0..1500 {
            let nb = rng.range(1, 12);
            let sz = sizes(&mut rng, nb, 70000, 200_000);
            let src = flat_input(&mut rng, &sz);
            let total: usize = sz.iter().sum();
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            let ctx = format!("row51/saveDict iter {it} total {total}");
            chain_prefix_on(&cc, &rc, sc, sr, &src, 0, &sz, 1, &ctx);

            for &maxd in &[65536usize, 65535, total / 2, 100, 1, 0, 100_000] {
                // one safe buffer per library
                let mut safec = vec![0u8; maxd.max(1) + GUARD];
                let mut safer = vec![0u8; maxd.max(1) + GUARD];
                safec.iter_mut().for_each(|x| *x = SENT);
                safer.iter_mut().for_each(|x| *x = SENT);
                let a = (cc.saveDict)(sc, safec.as_mut_ptr() as *mut c_char, maxd as c_int);
                let b = (rc.saveDict)(sr, safer.as_mut_ptr() as *mut c_char, maxd as c_int);
                let c = format!("{ctx}: saveDict maxDictSize {maxd}");
                assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                assert!(a >= 0 && a as usize <= maxd.min(65536), "{c}: bad return {a}");
                let k = a as usize;
                assert_eq!(&safec[..k], &safer[..k], "{c}: saved bytes differ");
                assert_eq!(
                    &safec[..k],
                    &src[total.saturating_sub(k)..total],
                    "{c}: saved bytes are not the dictionary tail"
                );
                assert!(
                    safec[maxd.max(1)..].iter().all(|&x| x == SENT)
                        && safer[maxd.max(1)..].iter().all(|&x| x == SENT),
                    "{c}: wrote past maxDictSize"
                );

                // The stream must now be usable straight away, with the
                // dictionary living in each library's own safe buffer.
                let n = one_size(&mut rng, 30000).max(1);
                let s = shape(&mut rng);
                let blk = gen(&mut rng, s, n);
                let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
                let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
                let x = (cc.compress_fast_continue)(
                    sc,
                    blk.as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let y = (rc.compress_fast_continue)(
                    sr,
                    blk.as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                check(&format!("{c}: continue after saveDict"), x, y, &dc, &dr, cap);
                assert!(x > 0);
                // Decode with the saved dictionary.
                let (mut oc, mut or) = (dbuf(n), dbuf(n));
                let dp = if k == 0 {
                    std::ptr::null()
                } else {
                    safer.as_ptr() as *const c_char
                };
                let dq = if k == 0 {
                    std::ptr::null()
                } else {
                    safec.as_ptr() as *const c_char
                };
                let ra = (cd.safe_usingDict)(
                    dr.as_ptr() as *const c_char,
                    oc.as_mut_ptr() as *mut c_char,
                    y,
                    n as c_int,
                    dp,
                    k as c_int,
                );
                let rb = (rd.safe_usingDict)(
                    dc.as_ptr() as *const c_char,
                    or.as_mut_ptr() as *mut c_char,
                    x,
                    n as c_int,
                    dq,
                    k as c_int,
                );
                assert_eq!(ra, rb, "{c}: usingDict decode mismatch");
                assert_eq!(ra, n as c_int, "{c}: usingDict decode size");
                assert_eq!(&oc[..n], &blk[..], "{c}: C round-trip");
                assert_eq!(&or[..n], &blk[..], "{c}: Rust round-trip");
                // Re-seed the stream for the next maxDictSize with a fresh chain.
                (cc.resetStream)(sc);
                (rc.resetStream)(sr);
                chain_prefix_on(&cc, &rc, sc, sr, &src, 0, &sz, 1, &ctx);
            }
            // safeBuffer == NULL with maxDictSize 0 is the documented no-op.
            let a = (cc.saveDict)(sc, std::ptr::null_mut(), 0);
            let b = (rc.saveDict)(sr, std::ptr::null_mut(), 0);
            assert_eq!(a, b, "{ctx}: saveDict(NULL, 0)");
            assert_eq!(a, 0, "{ctx}: saveDict(NULL, 0) should be 0");
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
        }

        // ---- LZ4_compress_forceExtDict ----
        for &dn in &[0usize, 3, 8, 1000, 32768, 65535, 65536, 100 * 1024] {
            for it in 0..180 {
                let s = shape(&mut rng);
                let dict = gen(&mut rng, s, dn);
                let sc = (cc.createStream)();
                let sr = (rc.createStream)();
                let dp = if dn == 0 {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_char
                };
                let eff = (cc.loadDict)(sc, dp, dn as c_int);
                assert_eq!(eff, (rc.loadDict)(sr, dp, dn as c_int));
                let ctx = format!("row51/forceExtDict dict {dn} iter {it}");

                let sz: Vec<usize> = (0..rng.range(1, 8)).map(|_| one_size(&mut rng, 40000)).collect();
                let blocks = dict_derived_blocks(&mut rng, &dict, &sz);
                let mut bc = Vec::new();
                let mut br = Vec::new();
                for (i, buf) in blocks.iter().enumerate() {
                    let n = buf.len();
                    // forceExtDict is notLimited: dst must be bound-sized.
                    let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
                    let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
                    let p = if n == 0 {
                        std::ptr::null()
                    } else {
                        buf.as_ptr() as *const c_char
                    };
                    let a = (cc.compress_forceExtDict)(
                        sc,
                        p,
                        dc.as_mut_ptr() as *mut c_char,
                        n as c_int,
                    );
                    let b = (rc.compress_forceExtDict)(
                        sr,
                        p,
                        dr.as_mut_ptr() as *mut c_char,
                        n as c_int,
                    );
                    check(&format!("{ctx}: block {i} len {n}"), a, b, &dc, &dr, cap);
                    assert!(a > 0, "{ctx}: block {i} len {n} ret {a}");
                    dc.truncate(a as usize);
                    dr.truncate(b as usize);
                    bc.push(dc);
                    br.push(dr);
                }
                assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));

                // Decode: dict, then each previous block as the ext dictionary.
                let e = eff.max(0) as usize;
                let dtail = &dict[dict.len() - e.min(dict.len())..];
                let dtp = if dtail.is_empty() {
                    std::ptr::null()
                } else {
                    dtail.as_ptr() as *const c_char
                };
                let sdc = (cd.createStreamDecode)();
                let sdr = (rd.createStreamDecode)();
                (cd.setStreamDecode)(sdc, dtp, dtail.len() as c_int);
                (rd.setStreamDecode)(sdr, dtp, dtail.len() as c_int);
                let mut oc: Vec<Vec<u8>> = Vec::new();
                let mut or: Vec<Vec<u8>> = Vec::new();
                for (i, buf) in blocks.iter().enumerate() {
                    let n = buf.len();
                    oc.push(dbuf(n));
                    or.push(dbuf(n));
                    let a = (cd.safe_continue)(
                        sdc,
                        br[i].as_ptr() as *const c_char,
                        oc[i].as_mut_ptr() as *mut c_char,
                        br[i].len() as c_int,
                        n as c_int,
                    );
                    let b = (rd.safe_continue)(
                        sdr,
                        bc[i].as_ptr() as *const c_char,
                        or[i].as_mut_ptr() as *mut c_char,
                        bc[i].len() as c_int,
                        n as c_int,
                    );
                    let c = format!("{ctx}: decode block {i} len {n}");
                    assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                    assert_eq!(a, n as c_int, "{c}: size");
                    assert_eq!(&oc[i][..n], &buf[..], "{c}: C round-trip");
                    assert_eq!(&or[i][..n], &buf[..], "{c}: Rust round-trip");
                }
                assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
            }
        }
    }
}

// ===========================================================================
// Row 52 - legacy / deprecated streaming + one-shot wrappers
// ===========================================================================

#[test]
fn row_52_legacy_and_deprecated_wrappers() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(52);
    unsafe {
        assert_eq!((cc.sizeofStreamState)(), (rc.sizeofStreamState)());
        assert_eq!((cc.sizeofStreamState)(), (cc.sizeofState)());
        let state_bytes = (cc.sizeofStreamState)() as usize;

        // ---- LZ4_compress / LZ4_compress_limitedOutput (one-shot) ----
        for &n in &[0usize, 1, 12, 13, 100, 4096, 65535, 65547, 100_000] {
            for s in ALL_SHAPES {
                let src = gen(&mut rng, s, n);
                let bound = ((cc.compressBound)(n as c_int) as usize).max(1);
                let p = if n == 0 {
                    std::ptr::null()
                } else {
                    src.as_ptr() as *const c_char
                };
                let (mut dc, mut dr) = (dbuf(bound), dbuf(bound));
                let a = (cc.compress)(p, dc.as_mut_ptr() as *mut c_char, n as c_int);
                let b = (rc.compress)(p, dr.as_mut_ptr() as *mut c_char, n as c_int);
                let ctx = format!("row52/LZ4_compress n {n} shape {s:?}");
                check(&ctx, a, b, &dc, &dr, bound);

                for cap in [bound, bound.saturating_sub(1).max(1), (n / 3).max(1), 1, 0] {
                    let (mut ec, mut er) = (dbuf(cap), dbuf(cap));
                    let x = (cc.compress_limitedOutput)(
                        p,
                        ec.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                    );
                    let y = (rc.compress_limitedOutput)(
                        p,
                        er.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                    );
                    check(
                        &format!("row52/LZ4_compress_limitedOutput n {n} cap {cap} shape {s:?}"),
                        x,
                        y,
                        &ec,
                        &er,
                        cap,
                    );
                }

                // ---- LZ4_compress_withState / _limitedOutput_withState ----
                // One caller-allocated state buffer per library.
                let mut stc = vec![0u64; (state_bytes + 7) / 8];
                let mut str_ = vec![0u64; (state_bytes + 7) / 8];
                let (spc, spr) = (
                    stc.as_mut_ptr() as *mut c_void,
                    str_.as_mut_ptr() as *mut c_void,
                );
                let (mut fc, mut fr) = (dbuf(bound), dbuf(bound));
                let x = (cc.compress_withState)(
                    spc,
                    p,
                    fc.as_mut_ptr() as *mut c_char,
                    n as c_int,
                );
                let y = (rc.compress_withState)(
                    spr,
                    p,
                    fr.as_mut_ptr() as *mut c_char,
                    n as c_int,
                );
                check(
                    &format!("row52/LZ4_compress_withState n {n} shape {s:?}"),
                    x,
                    y,
                    &fc,
                    &fr,
                    bound,
                );
                for cap in [bound, (n / 3).max(1), 1, 0] {
                    let (mut gc, mut gr) = (dbuf(cap), dbuf(cap));
                    let x = (cc.compress_limitedOutput_withState)(
                        spc,
                        p,
                        gc.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                    );
                    let y = (rc.compress_limitedOutput_withState)(
                        spr,
                        p,
                        gr.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                    );
                    check(
                        &format!(
                            "row52/LZ4_compress_limitedOutput_withState n {n} cap {cap} shape {s:?}"
                        ),
                        x,
                        y,
                        &gc,
                        &gr,
                        cap,
                    );
                }
                // LZ4_uncompress_unknownOutputSize / LZ4_uncompress round-trip
                if a > 0 {
                    let (mut oc, mut or) = (dbuf(n.max(1)), dbuf(n.max(1)));
                    let ra = (cd.safe)(
                        dr.as_ptr() as *const c_char,
                        oc.as_mut_ptr() as *mut c_char,
                        b,
                        n as c_int,
                    );
                    let rb = (rd.safe)(
                        dc.as_ptr() as *const c_char,
                        or.as_mut_ptr() as *mut c_char,
                        a,
                        n as c_int,
                    );
                    assert_eq!(ra, rb, "{ctx}: decode mismatch");
                    assert_eq!(ra, n as c_int, "{ctx}: decode size");
                    assert_eq!(&oc[..n], &src[..], "{ctx}: C round-trip");
                }
            }
        }

        // ---- LZ4_create / LZ4_slideInputBuffer / LZ4_resetStreamState /
        //      LZ4_compress_continue / LZ4_compress_limitedOutput_continue ----
        for it in 0..1350 {
            let nb = rng.range(1, 20);
            let sz = sizes(&mut rng, nb, 70000, 250_000);
            let src = flat_input(&mut rng, &sz);
            let mut inbuf = vec![0u8; 64];
            let ctx = format!("row52/legacy-stream iter {it}");

            let sc = (cc.create)(inbuf.as_mut_ptr() as *mut c_char);
            let sr = (rc.create)(inbuf.as_mut_ptr() as *mut c_char);
            assert!(!sc.is_null() && !sr.is_null(), "{ctx}: LZ4_create returned NULL");

            // Fresh stream: dictionary == NULL, so slideInputBuffer is NULL.
            let pa = (cc.slideInputBuffer)(sc);
            let pb = (rc.slideInputBuffer)(sr);
            assert_eq!(
                pa as usize, pb as usize,
                "{ctx}: LZ4_slideInputBuffer on a fresh stream (C={pa:?} Rust={pb:?})"
            );
            assert!(pa.is_null(), "{ctx}: expected NULL dictionary pointer");

            let mut bc = Vec::new();
            let mut br = Vec::new();
            let mut off = 0usize;
            for (i, &n) in sz.iter().enumerate() {
                let bound = ((cc.compressBound)(n as c_int) as usize).max(1);
                let p = src.as_ptr().add(off) as *const c_char;
                let (mut dc, mut dr) = (dbuf(bound), dbuf(bound));
                let (a, b) = if i % 2 == 0 {
                    (
                        (cc.compress_continue)(sc, p, dc.as_mut_ptr() as *mut c_char, n as c_int),
                        (rc.compress_continue)(sr, p, dr.as_mut_ptr() as *mut c_char, n as c_int),
                    )
                } else {
                    (
                        (cc.compress_limitedOutput_continue)(
                            sc,
                            p,
                            dc.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            bound as c_int,
                        ),
                        (rc.compress_limitedOutput_continue)(
                            sr,
                            p,
                            dr.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            bound as c_int,
                        ),
                    )
                };
                check(&format!("{ctx}: block {i} len {n}"), a, b, &dc, &dr, bound);
                assert!(a > 0);
                dc.truncate(a as usize);
                dr.truncate(b as usize);
                bc.push(dc);
                br.push(dr);

                // The dictionary now points into the shared `src` buffer, so
                // both libraries must report the same address.
                let pa = (cc.slideInputBuffer)(sc);
                let pb = (rc.slideInputBuffer)(sr);
                assert_eq!(
                    pa as usize, pb as usize,
                    "{ctx}: LZ4_slideInputBuffer after block {i} (C={pa:?} Rust={pb:?})"
                );
                off += n;
            }
            decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);

            // LZ4_resetStreamState on the same (library-owned) state.
            let a = (cc.resetStreamState)(sc, inbuf.as_mut_ptr() as *mut c_char);
            let b = (rc.resetStreamState)(sr, inbuf.as_mut_ptr() as *mut c_char);
            assert_eq!(a, b, "{ctx}: LZ4_resetStreamState");
            assert_eq!(a, 0, "{ctx}: LZ4_resetStreamState should return 0");
            let ctx2 = format!("{ctx} after resetStreamState");
            let (bc2, br2) = chain_prefix_on(&cc, &rc, sc, sr, &src, 0, &sz, 1, &ctx2);
            decode_flat(&cd, &rd, &bc2, &br2, &sz, &src, &ctx2);

            // limitedOutput_continue with a dst that cannot hold the block.
            let sc2 = (cc.createStream)();
            let sr2 = (rc.createStream)();
            let mut off = 0usize;
            for (i, &n) in sz.iter().enumerate() {
                let cap = (n / 4).max(1);
                let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
                let p = src.as_ptr().add(off) as *const c_char;
                let a = (cc.compress_limitedOutput_continue)(
                    sc2,
                    p,
                    dc.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                let b = (rc.compress_limitedOutput_continue)(
                    sr2,
                    p,
                    dr.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                check(
                    &format!("{ctx}: tight limitedOutput_continue block {i} len {n} cap {cap}"),
                    a,
                    b,
                    &dc,
                    &dr,
                    cap,
                );
                off += n;
            }
            assert_eq!((cc.freeStream)(sc2), (rc.freeStream)(sr2));

            // LZ4_create'd streams are freed with LZ4_freeStream.
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr), "{ctx}: freeStream");
        }
    }
}

// ===========================================================================
// Rows 53-58 - streaming decompression state and prefix/extDict branches
// ===========================================================================

/// `n` regions of `cap` bytes inside one allocation, separated by 16-byte gaps
/// so that the encoder's overlap test can never fire accidentally.
struct MultiBuf {
    mem: Vec<u8>,
    cap: usize,
    n: usize,
}

impl MultiBuf {
    fn new(n: usize, cap: usize) -> MultiBuf {
        MultiBuf {
            mem: vec![SENT; n * (cap + 16)],
            cap,
            n,
        }
    }
    fn base(&self, i: usize) -> usize {
        (i % self.n) * (self.cap + 16)
    }
}

/// Encode `sz` blocks through `n_bufs` rotating buffers, then decode through
/// `n_bufs` rotating buffers per library (every step is a buffer switch, i.e.
/// `LZ4_decompress_safe_continue`'s forceExtDict branch).
unsafe fn rotating_round_trip(
    cc: &Cs,
    rc: &Cs,
    cd: &Ds,
    rd: &Ds,
    rng: &mut Rng,
    n_bufs: usize,
    sz: &[usize],
    ctx: &str,
) {
    let cap = sz.iter().copied().max().unwrap_or(1).max(1);
    // Fill *and* compress each block in the same step: with rotating buffers a
    // later block overwrites an earlier one, so the data must be written
    // immediately before it is compressed.
    let mut m = MultiBuf::new(n_bufs, cap);
    let mut orig: Vec<Vec<u8>> = Vec::new();
    let sc = (cc.createStream)();
    let sr = (rc.createStream)();
    let mut bc = Vec::new();
    let mut br = Vec::new();
    for (i, &n) in sz.iter().enumerate() {
        let b = m.base(i);
        {
            let s = shape(rng);
            let d = gen(rng, s, n);
            m.mem[b..b + n].copy_from_slice(&d);
            orig.push(d);
        }
        let capd = ((cc.compressBound)(n as c_int) as usize).max(1);
        let (mut dc, mut dr) = (dbuf(capd), dbuf(capd));
        let p = m.mem.as_ptr().add(b) as *const c_char;
        let a = (cc.compress_fast_continue)(
            sc,
            p,
            dc.as_mut_ptr() as *mut c_char,
            n as c_int,
            capd as c_int,
            1,
        );
        let x = (rc.compress_fast_continue)(
            sr,
            p,
            dr.as_mut_ptr() as *mut c_char,
            n as c_int,
            capd as c_int,
            1,
        );
        check(&format!("{ctx}: encode block {i} len {n}"), a, x, &dc, &dr, capd);
        assert!(a > 0);
        dc.truncate(a as usize);
        dr.truncate(x as usize);
        bc.push(dc);
        br.push(dr);
    }
    assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));

    let mut oc = MultiBuf::new(n_bufs, cap + GUARD);
    let mut or = MultiBuf::new(n_bufs, cap + GUARD);
    let sdc = (cd.createStreamDecode)();
    let sdr = (rd.createStreamDecode)();
    for (i, &n) in sz.iter().enumerate() {
        let b = oc.base(i);
        let a = (cd.safe_continue)(
            sdc,
            br[i].as_ptr() as *const c_char,
            oc.mem.as_mut_ptr().add(b) as *mut c_char,
            br[i].len() as c_int,
            n as c_int,
        );
        let x = (rd.safe_continue)(
            sdr,
            bc[i].as_ptr() as *const c_char,
            or.mem.as_mut_ptr().add(b) as *mut c_char,
            bc[i].len() as c_int,
            n as c_int,
        );
        let c = format!("{ctx}: decode block {i} len {n}");
        assert_eq!(a, x, "{c}: return mismatch (C={a} Rust={x})");
        assert_eq!(a, n as c_int, "{c}: size");
        assert_eq!(&oc.mem[b..b + n], &orig[i][..], "{c}: C round-trip");
        assert_eq!(&or.mem[b..b + n], &orig[i][..], "{c}: Rust round-trip");
    }
    assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
    let _ = or.n;
}

#[test]
fn row_53_create_free_and_set_stream_decode() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(53);
    unsafe {
        // free(NULL) is supported.
        assert_eq!(
            (cd.freeStreamDecode)(std::ptr::null_mut()),
            (rd.freeStreamDecode)(std::ptr::null_mut()),
            "LZ4_freeStreamDecode(NULL)"
        );
        assert_eq!((cd.freeStreamDecode)(std::ptr::null_mut()), 0);

        let sdc = (cd.createStreamDecode)();
        let sdr = (rd.createStreamDecode)();
        assert!(!sdc.is_null() && !sdr.is_null(), "createStreamDecode failed");
        // NULL dictionary with size 0 == reset.
        assert_eq!(
            (cd.setStreamDecode)(sdc, std::ptr::null(), 0),
            (rd.setStreamDecode)(sdr, std::ptr::null(), 0),
            "LZ4_setStreamDecode(NULL, 0)"
        );
        assert_eq!((cd.setStreamDecode)(sdc, std::ptr::null(), 0), 1);
        assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));

        // Small dictionary, exactly-64 KB dictionary, and >64 KB: compress with
        // LZ4_loadDict, decode after LZ4_setStreamDecode with the same bytes.
        for &dn in &[0usize, 1, 3, 8, 100, 4096, 65535, 65536, 65537, 100 * 1024] {
            for s in [Shape::TextLike, Shape::Periodic] {
                let dict = gen(&mut rng, s, dn);
                let ctx = format!("row53 dict {dn} shape {s:?}");
                load_dict_case(&cc, &rc, &cd, &rd, false, &dict, &mut rng, &ctx);
            }
        }
        // A decode state re-used across several independent streams, reset by
        // LZ4_setStreamDecode(NULL, 0) between them.
        let sdc = (cd.createStreamDecode)();
        let sdr = (rd.createStreamDecode)();
        for it in 0..1350 {
            assert_eq!(
                (cd.setStreamDecode)(sdc, std::ptr::null(), 0),
                (rd.setStreamDecode)(sdr, std::ptr::null(), 0)
            );
            let nb = rng.range(1, 10);
            let sz = sizes(&mut rng, nb, 40000, 150_000);
            let src = flat_input(&mut rng, &sz);
            let ctx = format!("row53/reuse iter {it}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
            let total: usize = sz.iter().sum();
            let (mut oc, mut or) = (dbuf(total), dbuf(total));
            let mut off = 0usize;
            for (i, &n) in sz.iter().enumerate() {
                let a = (cd.safe_continue)(
                    sdc,
                    br[i].as_ptr() as *const c_char,
                    oc.as_mut_ptr().add(off) as *mut c_char,
                    br[i].len() as c_int,
                    n as c_int,
                );
                let b = (rd.safe_continue)(
                    sdr,
                    bc[i].as_ptr() as *const c_char,
                    or.as_mut_ptr().add(off) as *mut c_char,
                    bc[i].len() as c_int,
                    n as c_int,
                );
                assert_eq!(a, b, "{ctx}: block {i}");
                assert_eq!(a, n as c_int, "{ctx}: block {i} size");
                off += n;
            }
            same_full_buffers(&ctx, &oc, &or);
            assert_eq!(&oc[..total], &src[..]);
        }
        assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
    }
}

#[test]
fn row_54_first_block_plain_safe_decode() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(54);
    unsafe {
        // A brand new decode state: prefixSize == 0 -> plain LZ4_decompress_safe.
        for &n in &[0usize, 1, 12, 13, 100, 4096, 65535, 65536, 65547, 200_000] {
            for s in ALL_SHAPES {
                let sz = vec![n];
                let src = gen(&mut rng, s, n);
                let ctx = format!("row54 n {n} shape {s:?}");
                let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
                let sdc = (cd.createStreamDecode)();
                let sdr = (rd.createStreamDecode)();
                for cap in [n, n + 1, n + 64] {
                    let (mut oc, mut or) = (dbuf(cap), dbuf(cap));
                    let a = (cd.safe_continue)(
                        sdc,
                        br[0].as_ptr() as *const c_char,
                        oc.as_mut_ptr() as *mut c_char,
                        br[0].len() as c_int,
                        cap as c_int,
                    );
                    let b = (rd.safe_continue)(
                        sdr,
                        bc[0].as_ptr() as *const c_char,
                        or.as_mut_ptr() as *mut c_char,
                        bc[0].len() as c_int,
                        cap as c_int,
                    );
                    let c = format!("{ctx} cap {cap}");
                    assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                    assert_eq!(a, n as c_int, "{c}: size");
                    assert_eq!(&oc[..n], &src[..], "{c}: C round-trip");
                    same_full_buffers(&c, &oc, &or);
                }
                assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
            }
        }
    }
}

#[test]
fn row_55_contiguous_small_prefix() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(55);
    unsafe {
        // Whole chain stays below 64 KB - 1, so every continuation block takes
        // the withSmallPrefix branch (extDictSize == 0).
        for it in 0..5400 {
            let nb = rng.range(2, 30);
            let mut sz: Vec<usize> = Vec::new();
            let mut tot = 0usize;
            for _ in 0..nb {
                let n = rng.range(0, 2500).min(65534 - tot);
                tot += n;
                sz.push(n);
            }
            assert!(tot < 65535);
            let src = flat_input(&mut rng, &sz);
            let ctx = format!("row55 iter {it} total {tot}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
            decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
        }
        // exactly at the boundary: cumulative prefix == 65534 then one more block
        let sz = vec![65534usize, 1, 13, 4096];
        let src = flat_input(&mut rng, &sz);
        let ctx = "row55 boundary 65534";
        let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, ctx);
        decode_flat(&cd, &rd, &bc, &br, &sz, &src, ctx);
    }
}

#[test]
fn row_56_contiguous_prefix_64k() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(56);
    unsafe {
        // Cross the 64 KB - 1 prefix boundary inside the chain.
        for &first in &[65534usize, 65535, 65536, 65537, 100_000] {
            for s in ALL_SHAPES {
                let sz = vec![first, 1, 12, 13, 4096, 0, 65536, 3];
                let mut src = Vec::new();
                for &n in &sz {
                    src.extend_from_slice(&gen(&mut rng, s, n));
                }
                let ctx = format!("row56 first {first} shape {s:?}");
                let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
                decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
            }
        }
        for it in 0..2700 {
            let nb = rng.range(2, 25);
            let sz = sizes(&mut rng, nb, 70000, 400_000);
            let src = flat_input(&mut rng, &sz);
            let ctx = format!("row56 iter {it}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
            decode_flat(&cd, &rd, &bc, &br, &sz, &src, &ctx);
        }
    }
}

#[test]
fn row_57_double_dict_contiguous_dst_with_ext_dict() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(57);
    unsafe {
        // The encoder sees one flat window (so it may reference data that ends
        // up in the decoder's buffer A), while the decoder decodes the first
        // `k` blocks into A and the rest contiguously into B. From block k+1 on,
        // prefixEnd == dest AND extDictSize != 0  =>  doubleDict.
        for it in 0..4200 {
            let ka = rng.range(1, 8);
            let kb = rng.range(2, 12);
            let mut sz: Vec<usize> = (0..ka).map(|_| rng.range(1, 30000)).collect();
            let mut totb = 0usize;
            for _ in 0..kb {
                // keep B's prefix under 64 KB - 1 so doubleDict stays selected
                let n = rng.range(1, 6000).min(65534usize.saturating_sub(totb));
                totb += n;
                sz.push(n.max(1));
            }
            let totb: usize = sz[ka..].iter().sum();
            let tota: usize = sz[..ka].iter().sum();
            let src = flat_input(&mut rng, &sz);
            let ctx = format!("row57 iter {it} A {tota} B {totb}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);

            // one A/B pair per library
            let (mut ac, mut bcuf) = (dbuf(tota), dbuf(totb));
            let (mut ar, mut brf) = (dbuf(tota), dbuf(totb));
            let sdc = (cd.createStreamDecode)();
            let sdr = (rd.createStreamDecode)();
            let mut offa = 0usize;
            let mut offb = 0usize;
            for (i, &n) in sz.iter().enumerate() {
                let (pc, pr) = if i < ka {
                    (
                        ac.as_mut_ptr().add(offa) as *mut c_char,
                        ar.as_mut_ptr().add(offa) as *mut c_char,
                    )
                } else {
                    (
                        bcuf.as_mut_ptr().add(offb) as *mut c_char,
                        brf.as_mut_ptr().add(offb) as *mut c_char,
                    )
                };
                let a = (cd.safe_continue)(
                    sdc,
                    br[i].as_ptr() as *const c_char,
                    pc,
                    br[i].len() as c_int,
                    n as c_int,
                );
                let b = (rd.safe_continue)(
                    sdr,
                    bc[i].as_ptr() as *const c_char,
                    pr,
                    bc[i].len() as c_int,
                    n as c_int,
                );
                let c = format!("{ctx}: decode block {i} len {n}");
                assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                assert_eq!(a, n as c_int, "{c}: size");
                if i < ka {
                    offa += n;
                } else {
                    offb += n;
                }
            }
            assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
            same_full_buffers(&format!("{ctx}: A"), &ac, &ar);
            same_full_buffers(&format!("{ctx}: B"), &bcuf, &brf);
            assert_eq!(&ac[..tota], &src[..tota], "{ctx}: A round-trip");
            assert_eq!(&bcuf[..totb], &src[tota..tota + totb], "{ctx}: B round-trip");
        }
    }
}

#[test]
fn row_58_dst_buffer_switch_force_ext_dict() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(58);
    unsafe {
        for it in 0..4200 {
            let nbufs = rng.range(2, 4);
            let nb = rng.range(1, 30);
            let maxb = rng.range(16, 60000);
            let sz: Vec<usize> = (0..nb).map(|_| one_size(&mut rng, maxb).max(1)).collect();
            let ctx = format!("row58 iter {it} bufs {nbufs} maxb {maxb}");
            rotating_round_trip(&cc, &rc, &cd, &rd, &mut rng, nbufs, &sz, &ctx);
        }
        // blocks > 64 KB through a double buffer
        for it in 0..720 {
            let sz: Vec<usize> = (0..8).map(|_| rng.range(65537, 90000)).collect();
            let ctx = format!("row58/big iter {it}");
            rotating_round_trip(&cc, &rc, &cd, &rd, &mut rng, 2, &sz, &ctx);
        }
    }
}

// ===========================================================================
// Row 59 - decoder ring buffer sized exactly LZ4_decoderRingBufferSize()
// ===========================================================================

#[test]
fn row_59_decoder_ring_buffer_exact_size() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(59);
    unsafe {
        for &maxb in &[17usize, 1024, 4096, 20000, 65536] {
            let ring_c = (cd.decoderRingBufferSize)(maxb as c_int);
            let ring_r = (rd.decoderRingBufferSize)(maxb as c_int);
            assert_eq!(ring_c, ring_r, "row59: decoderRingBufferSize({maxb})");
            let ring = ring_c as usize;
            assert_eq!(ring, 65536 + 14 + maxb);

            // Encoder: one flat window (blocks <= maxb). Decoder: a ring of
            // exactly the advertised size, resuming from 0 whenever fewer than
            // maxb bytes remain.
            let want_total = ring * 3 + 1000;
            let mut sz: Vec<usize> = Vec::new();
            let mut tot = 0usize;
            while tot < want_total {
                let n = rng.range(17, maxb);
                sz.push(n);
                tot += n;
            }
            let src = flat_input(&mut rng, &sz);
            let ctx = format!("row59 maxb {maxb} ring {ring}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);

            let mut oc = vec![SENT; ring];
            let mut or = vec![SENT; ring];
            let sdc = (cd.createStreamDecode)();
            let sdr = (rd.createStreamDecode)();
            let mut off = 0usize;
            let mut logical = 0usize;
            let mut wraps = 0usize;
            for (i, &n) in sz.iter().enumerate() {
                if ring - off < maxb {
                    off = 0;
                    wraps += 1;
                }
                let a = (cd.safe_continue)(
                    sdc,
                    br[i].as_ptr() as *const c_char,
                    oc.as_mut_ptr().add(off) as *mut c_char,
                    br[i].len() as c_int,
                    maxb as c_int,
                );
                let b = (rd.safe_continue)(
                    sdr,
                    bc[i].as_ptr() as *const c_char,
                    or.as_mut_ptr().add(off) as *mut c_char,
                    bc[i].len() as c_int,
                    maxb as c_int,
                );
                let c = format!("{ctx}: block {i} len {n} off {off}");
                assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                assert_eq!(a, n as c_int, "{c}: size");
                assert_eq!(
                    &oc[off..off + n],
                    &src[logical..logical + n],
                    "{c}: C round-trip"
                );
                assert_eq!(
                    &or[off..off + n],
                    &src[logical..logical + n],
                    "{c}: Rust round-trip"
                );
                off += n;
                logical += n;
            }
            assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
            assert!(wraps >= 2, "{ctx}: expected several ring wraps, got {wraps}");
        }
    }
}

// ===========================================================================
// Row 60 - synchronized small ring buffer (< 64 KB), exact block sizes
// ===========================================================================

#[test]
fn row_60_synchronized_small_ring_buffer() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(60);
    unsafe {
        for it in 0..2100 {
            let ring = rng.range(4096, 60000);
            // maxb <= ring/2 keeps a block from fully covering the previous one
            let maxb = (ring / 2).max(17);
            let nblocks = rng.range(10, 120);

            let mut enc = vec![0u8; ring];
            let mut oc = vec![SENT; ring];
            let mut or = vec![SENT; ring];
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            let sdc = (cd.createStreamDecode)();
            let sdr = (rd.createStreamDecode)();
            let ctx = format!("row60 iter {it} ring {ring} maxb {maxb}");
            let mut pos = 0usize;
            let mut wraps = 0usize;
            for i in 0..nblocks {
                if ring - pos < maxb {
                    pos = 0;
                    wraps += 1;
                }
                let n = rng.range(1, maxb);
                let s = shape(&mut rng);
                let data = gen(&mut rng, s, n);
                enc[pos..pos + n].copy_from_slice(&data);
                let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
                let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
                let p = enc.as_ptr().add(pos) as *const c_char;
                let a = (cc.compress_fast_continue)(
                    sc,
                    p,
                    dc.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (rc.compress_fast_continue)(
                    sr,
                    p,
                    dr.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let c = format!("{ctx}: block {i} len {n} pos {pos}");
                check(&c, a, b, &dc, &dr, cap);
                assert!(a > 0);

                // Decoder: exactly the same ring size, same update rule, exact
                // decompressed size for every block.
                let x = (cd.safe_continue)(
                    sdc,
                    dr.as_ptr() as *const c_char, // Rust block -> C decoder
                    oc.as_mut_ptr().add(pos) as *mut c_char,
                    b,
                    n as c_int,
                );
                let y = (rd.safe_continue)(
                    sdr,
                    dc.as_ptr() as *const c_char, // C block -> Rust decoder
                    or.as_mut_ptr().add(pos) as *mut c_char,
                    a,
                    n as c_int,
                );
                assert_eq!(x, y, "{c}: decode return mismatch (C={x} Rust={y})");
                assert_eq!(x, n as c_int, "{c}: decode size");
                assert_eq!(&oc[pos..pos + n], &data[..], "{c}: C round-trip");
                assert_eq!(&or[pos..pos + n], &data[..], "{c}: Rust round-trip");
                pos += n;
            }
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
            assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
            assert!(wraps >= 1, "{ctx}: no ring wrap happened");
        }
    }
}

// ===========================================================================
// Row 61 - mid-stream decode failure, then keep using both decoders
// ===========================================================================

#[test]
fn row_61_mid_stream_failure_then_continue() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(61);
    unsafe {
        for it in 0..4200 {
            let nb = rng.range(3, 20);
            let sz: Vec<usize> = (0..nb).map(|_| rng.range(20, 30000)).collect();
            let src = flat_input(&mut rng, &sz);
            let total: usize = sz.iter().sum();
            let ctx = format!("row61 iter {it}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
            let fail_at = 1 + rng.below(nb - 1);
            // Only *guaranteed* failures belong here: a dstCapacity smaller
            // than the block's decompressed size, and a truncated block. Both
            // must leave the decoder's prefix state untouched, which is proven
            // by decoding the very same block correctly right afterwards.
            // (Random single-byte corruption may legitimately still decode, so
            // it lives in the fuzz test instead.)
            let mode = rng.below(2);

            let (mut oc, mut or) = (dbuf(total), dbuf(total));
            let sdc = (cd.createStreamDecode)();
            let sdr = (rd.createStreamDecode)();
            let mut off = 0usize;
            for (i, &n) in sz.iter().enumerate() {
                if i == fail_at {
                    let (tc, tr, cap): (Vec<u8>, Vec<u8>, usize) = if mode == 0 {
                        (br[i].clone(), bc[i].clone(), (n / 3).max(1))
                    } else {
                        (
                            br[i][..br[i].len() * 2 / 3].to_vec(),
                            bc[i][..bc[i].len() * 2 / 3].to_vec(),
                            n,
                        )
                    };
                    let (mut fc, mut fr) = (dbuf(cap), dbuf(cap));
                    let a = (cd.safe_continue)(
                        sdc,
                        tc.as_ptr() as *const c_char,
                        fc.as_mut_ptr() as *mut c_char,
                        tc.len() as c_int,
                        cap as c_int,
                    );
                    let b = (rd.safe_continue)(
                        sdr,
                        tr.as_ptr() as *const c_char,
                        fr.as_mut_ptr() as *mut c_char,
                        tr.len() as c_int,
                        cap as c_int,
                    );
                    let c =
                        format!("{ctx}: injected failure at block {i} mode {mode} cap {cap}");
                    check(&c, a, b, &fc, &fr, cap);
                    assert!(a < 0, "{c}: expected a negative result, got {a}");
                }
                let a = (cd.safe_continue)(
                    sdc,
                    br[i].as_ptr() as *const c_char,
                    oc.as_mut_ptr().add(off) as *mut c_char,
                    br[i].len() as c_int,
                    n as c_int,
                );
                let b = (rd.safe_continue)(
                    sdr,
                    bc[i].as_ptr() as *const c_char,
                    or.as_mut_ptr().add(off) as *mut c_char,
                    bc[i].len() as c_int,
                    n as c_int,
                );
                let c = format!("{ctx}: block {i} len {n} (after failure at {fail_at})");
                assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                assert_eq!(a, n as c_int, "{c}: prefix state was not preserved");
                off += n;
            }
            assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
            same_full_buffers(&ctx, &oc, &or);
            assert_eq!(&oc[..total], &src[..], "{ctx}: round-trip");
        }
    }
}

// ===========================================================================
// Row 62 - LZ4_decompress_fast_continue, all three branches
// ===========================================================================

#[test]
fn row_62_decompress_fast_continue_branches() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(62);
    unsafe {
        // (a) first block + contiguous prefix continuation
        for it in 0..2700 {
            let nb = rng.range(1, 25);
            let sz = sizes(&mut rng, nb, 70000, 300_000);
            let src = flat_input(&mut rng, &sz);
            let total: usize = sz.iter().sum();
            let ctx = format!("row62/flat iter {it}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);
            let (mut oc, mut or) = (dbuf(total), dbuf(total));
            let sdc = (cd.createStreamDecode)();
            let sdr = (rd.createStreamDecode)();
            let mut off = 0usize;
            for (i, &n) in sz.iter().enumerate() {
                let a = (cd.fast_continue)(
                    sdc,
                    br[i].as_ptr() as *const c_char,
                    oc.as_mut_ptr().add(off) as *mut c_char,
                    n as c_int,
                );
                let b = (rd.fast_continue)(
                    sdr,
                    bc[i].as_ptr() as *const c_char,
                    or.as_mut_ptr().add(off) as *mut c_char,
                    n as c_int,
                );
                let c = format!("{ctx}: block {i} len {n}");
                assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                assert_eq!(
                    a,
                    br[i].len() as c_int,
                    "{c}: should report bytes read from input"
                );
                off += n;
            }
            assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
            same_full_buffers(&ctx, &oc, &or);
            assert_eq!(&oc[..total], &src[..], "{ctx}: C round-trip");
            assert_eq!(&or[..total], &src[..], "{ctx}: Rust round-trip");
        }

        // (b) prefix -> extDict switch (rotating destination buffers)
        for it in 0..2100 {
            let nbufs = rng.range(2, 4);
            let nb = rng.range(2, 20);
            let maxb = rng.range(32, 50000);
            let sz: Vec<usize> = (0..nb).map(|_| one_size(&mut rng, maxb).max(1)).collect();
            let cap = sz.iter().copied().max().unwrap();
            let mut m = MultiBuf::new(nbufs, cap);
            let mut orig: Vec<Vec<u8>> = Vec::new();
            let sc = (cc.createStream)();
            let sr = (rc.createStream)();
            let mut bc = Vec::new();
            let mut br = Vec::new();
            let ctx = format!("row62/rotating iter {it} bufs {nbufs}");
            for (i, &n) in sz.iter().enumerate() {
                let b = m.base(i);
                let s = shape(&mut rng);
                let d = gen(&mut rng, s, n);
                m.mem[b..b + n].copy_from_slice(&d);
                orig.push(d);
                let capd = ((cc.compressBound)(n as c_int) as usize).max(1);
                let (mut dc, mut dr) = (dbuf(capd), dbuf(capd));
                let p = m.mem.as_ptr().add(b) as *const c_char;
                let a = (cc.compress_fast_continue)(
                    sc,
                    p,
                    dc.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    capd as c_int,
                    1,
                );
                let x = (rc.compress_fast_continue)(
                    sr,
                    p,
                    dr.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    capd as c_int,
                    1,
                );
                check(&format!("{ctx}: encode {i}"), a, x, &dc, &dr, capd);
                dc.truncate(a as usize);
                dr.truncate(x as usize);
                bc.push(dc);
                br.push(dr);
            }
            assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));

            let mut oc = MultiBuf::new(nbufs, cap + GUARD);
            let mut or = MultiBuf::new(nbufs, cap + GUARD);
            let sdc = (cd.createStreamDecode)();
            let sdr = (rd.createStreamDecode)();
            for (i, &n) in sz.iter().enumerate() {
                let b = oc.base(i);
                let a = (cd.fast_continue)(
                    sdc,
                    br[i].as_ptr() as *const c_char,
                    oc.mem.as_mut_ptr().add(b) as *mut c_char,
                    n as c_int,
                );
                let x = (rd.fast_continue)(
                    sdr,
                    bc[i].as_ptr() as *const c_char,
                    or.mem.as_mut_ptr().add(b) as *mut c_char,
                    n as c_int,
                );
                let c = format!("{ctx}: decode {i} len {n}");
                assert_eq!(a, x, "{c}: return mismatch (C={a} Rust={x})");
                assert_eq!(a, br[i].len() as c_int, "{c}: bytes read");
                assert_eq!(&oc.mem[b..b + n], &orig[i][..], "{c}: C round-trip");
                assert_eq!(&or.mem[b..b + n], &orig[i][..], "{c}: Rust round-trip");
            }
            assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
        }
    }
}

// ===========================================================================
// Row 63 - LZ4_decoderRingBufferSize
// ===========================================================================

#[test]
fn row_63_decoder_ring_buffer_size() {
    let (cd, rd) = dapi();
    unsafe {
        let mut cases: Vec<c_int> = vec![
            0,
            1,
            15,
            16,
            17,
            64,
            1024,
            65535,
            65536,
            65537,
            4 * 1024 * 1024,
            LZ4_MAX_INPUT_SIZE as c_int - 1,
            LZ4_MAX_INPUT_SIZE as c_int,
            LZ4_MAX_INPUT_SIZE as c_int + 1,
            i32::MAX,
            -1,
            -16,
            -65536,
            i32::MIN,
        ];
        cases.sort();
        cases.dedup();
        for &n in &cases {
            let a = (cd.decoderRingBufferSize)(n);
            let b = (rd.decoderRingBufferSize)(n);
            assert_eq!(a, b, "row63: LZ4_decoderRingBufferSize({n}) C={a} Rust={b}");
            let want = if n < 0 || n as i64 > LZ4_MAX_INPUT_SIZE as i64 {
                0
            } else {
                65536 + 14 + n.max(16)
            };
            assert_eq!(a, want, "row63: LZ4_decoderRingBufferSize({n})");
        }
    }
}

// ===========================================================================
// Rows 64/65 - stateless dictionary decompression entry points
// ===========================================================================

/// Compress one block against `dict` (via `LZ4_loadDict` +
/// `LZ4_compress_fast_continue`, so the block genuinely references the
/// dictionary). Returns `(C block, Rust block, effective dictSize)`.
unsafe fn dict_block_pair(
    cc: &Cs,
    rc: &Cs,
    dict: &[u8],
    blk: &[u8],
    ctx: &str,
) -> (Vec<u8>, Vec<u8>, usize) {
    let sc = (cc.createStream)();
    let sr = (rc.createStream)();
    let dp = if dict.is_empty() {
        std::ptr::null()
    } else {
        dict.as_ptr() as *const c_char
    };
    let eff = (cc.loadDict)(sc, dp, dict.len() as c_int);
    assert_eq!(eff, (rc.loadDict)(sr, dp, dict.len() as c_int), "{ctx}: loadDict");
    let n = blk.len();
    let cap = ((cc.compressBound)(n as c_int) as usize).max(1);
    let (mut dc, mut dr) = (dbuf(cap), dbuf(cap));
    let p = if n == 0 {
        std::ptr::null()
    } else {
        blk.as_ptr() as *const c_char
    };
    let a = (cc.compress_fast_continue)(
        sc,
        p,
        dc.as_mut_ptr() as *mut c_char,
        n as c_int,
        cap as c_int,
        1,
    );
    let b = (rc.compress_fast_continue)(
        sr,
        p,
        dr.as_mut_ptr() as *mut c_char,
        n as c_int,
        cap as c_int,
        1,
    );
    check(&format!("{ctx}: compress block len {n}"), a, b, &dc, &dr, cap);
    assert!(a > 0, "{ctx}: compress failed ({a})");
    dc.truncate(a as usize);
    dr.truncate(b as usize);
    assert_eq!((cc.freeStream)(sc), (rc.freeStream)(sr));
    (dc, dr, eff.max(0) as usize)
}

/// A block that starts by repeating the tail of the dictionary several times,
/// so the encoder emits a match that starts inside the dictionary and continues
/// past `dictEnd` into the block itself.
fn straddling_block(rng: &mut Rng, dict: &[u8], n: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(n + 8);
    if !dict.is_empty() {
        let tail = dict.len().min(rng.range(8, 64));
        while v.len() < n {
            let take = tail.min(n - v.len());
            v.extend_from_slice(&dict[dict.len() - tail..dict.len() - tail + take]);
        }
    }
    v.resize(n, 0);
    // a little noise in the second half so the block isn't purely periodic
    let s = shape(rng);
    let noise = gen(rng, s, n / 6);
    for (i, x) in noise.iter().enumerate() {
        let at = n / 2 + i;
        if at < n {
            v[at] = *x;
        }
    }
    v
}

#[test]
fn row_64_safe_using_dict_and_partial_using_dict() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(64);
    unsafe {
        // ---- dictSize == 0 (delegates to LZ4_decompress_safe[_partial]) ----
        for &n in &[0usize, 1, 13, 100, 4096, 65536, 100_000] {
            let s = shape(&mut rng);
            let blk = gen(&mut rng, s, n);
            let (dc, dr, eff) = dict_block_pair(&cc, &rc, &[], &blk, "row64/dict0");
            assert_eq!(eff, 0);
            let (mut oc, mut or) = (dbuf(n.max(1)), dbuf(n.max(1)));
            let a = (cd.safe_usingDict)(
                dr.as_ptr() as *const c_char,
                oc.as_mut_ptr() as *mut c_char,
                dr.len() as c_int,
                n as c_int,
                std::ptr::null(),
                0,
            );
            let b = (rd.safe_usingDict)(
                dc.as_ptr() as *const c_char,
                or.as_mut_ptr() as *mut c_char,
                dc.len() as c_int,
                n as c_int,
                std::ptr::null(),
                0,
            );
            let ctx = format!("row64/dictSize0 n {n}");
            assert_eq!(a, b, "{ctx}: return mismatch (C={a} Rust={b})");
            assert_eq!(a, n as c_int, "{ctx}: size");
            assert_eq!(&oc[..n], &blk[..], "{ctx}: round-trip");
            same_full_buffers(&ctx, &oc, &or);
            for &t in &[0usize, 1, n / 2, n, n + 10] {
                for &dcap in &[n, n / 2 + 1, t.max(1)] {
                    let (mut pc, mut pr) = (dbuf(dcap.max(1)), dbuf(dcap.max(1)));
                    let x = (cd.safe_partial_usingDict)(
                        dr.as_ptr() as *const c_char,
                        pc.as_mut_ptr() as *mut c_char,
                        dr.len() as c_int,
                        t as c_int,
                        dcap as c_int,
                        std::ptr::null(),
                        0,
                    );
                    let y = (rd.safe_partial_usingDict)(
                        dc.as_ptr() as *const c_char,
                        pr.as_mut_ptr() as *mut c_char,
                        dc.len() as c_int,
                        t as c_int,
                        dcap as c_int,
                        std::ptr::null(),
                        0,
                    );
                    check(
                        &format!("{ctx}: partial target {t} cap {dcap}"),
                        x,
                        y,
                        &pc,
                        &pr,
                        dcap.max(1),
                    );
                }
            }
        }

        // ---- prefix layout (dictStart + dictSize == dst) and extDict ----
        for &dn in &[8usize, 100, 4096, 32768, 65534, 65535, 65536, 65537, 100 * 1024] {
            for it in 0..180 {
                let s = shape(&mut rng);
                let dict = gen(&mut rng, s, dn);
                let n = one_size(&mut rng, 50000).max(1);
                let blk = straddling_block(&mut rng, &dict, n);
                let ctx = format!("row64 dict {dn} n {n} iter {it}");
                let (dc, dr, eff) = dict_block_pair(&cc, &rc, &dict, &blk, &ctx);
                let dtail = &dict[dict.len() - eff..];

                // (a) dictionary contiguous with dst
                let mut pc = vec![SENT; eff + n + GUARD];
                let mut pr = vec![SENT; eff + n + GUARD];
                pc[..eff].copy_from_slice(dtail);
                pr[..eff].copy_from_slice(dtail);
                let a = (cd.safe_usingDict)(
                    dr.as_ptr() as *const c_char,
                    pc.as_mut_ptr().add(eff) as *mut c_char,
                    dr.len() as c_int,
                    n as c_int,
                    pc.as_ptr() as *const c_char,
                    eff as c_int,
                );
                let b = (rd.safe_usingDict)(
                    dc.as_ptr() as *const c_char,
                    pr.as_mut_ptr().add(eff) as *mut c_char,
                    dc.len() as c_int,
                    n as c_int,
                    pr.as_ptr() as *const c_char,
                    eff as c_int,
                );
                let c = format!("{ctx}: usingDict contiguous (eff {eff})");
                assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                assert_eq!(a, n as c_int, "{c}: size");
                assert_eq!(&pc[eff..eff + n], &blk[..], "{c}: C round-trip");
                assert_eq!(&pr[eff..eff + n], &blk[..], "{c}: Rust round-trip");
                same_full_buffers(&c, &pc, &pr);

                // (b) separate ext dictionary
                let (mut ec, mut er) = (dbuf(n), dbuf(n));
                let a = (cd.safe_usingDict)(
                    dr.as_ptr() as *const c_char,
                    ec.as_mut_ptr() as *mut c_char,
                    dr.len() as c_int,
                    n as c_int,
                    dtail.as_ptr() as *const c_char,
                    eff as c_int,
                );
                let b = (rd.safe_usingDict)(
                    dc.as_ptr() as *const c_char,
                    er.as_mut_ptr() as *mut c_char,
                    dc.len() as c_int,
                    n as c_int,
                    dtail.as_ptr() as *const c_char,
                    eff as c_int,
                );
                let c = format!("{ctx}: usingDict extDict (eff {eff})");
                assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                assert_eq!(a, n as c_int, "{c}: size");
                assert_eq!(&ec[..n], &blk[..], "{c}: C round-trip");
                same_full_buffers(&c, &ec, &er);

                // (c) partial variants, both layouts
                for &t in &[0usize, 1, 13, n / 3, n, n + 100] {
                    for &dcap in &[n, (n / 2).max(1), t.max(1)] {
                        // contiguous
                        let mut qc = vec![SENT; eff + dcap.max(1) + GUARD];
                        let mut qr = vec![SENT; eff + dcap.max(1) + GUARD];
                        qc[..eff].copy_from_slice(dtail);
                        qr[..eff].copy_from_slice(dtail);
                        let x = (cd.safe_partial_usingDict)(
                            dr.as_ptr() as *const c_char,
                            qc.as_mut_ptr().add(eff) as *mut c_char,
                            dr.len() as c_int,
                            t as c_int,
                            dcap as c_int,
                            qc.as_ptr() as *const c_char,
                            eff as c_int,
                        );
                        let y = (rd.safe_partial_usingDict)(
                            dc.as_ptr() as *const c_char,
                            qr.as_mut_ptr().add(eff) as *mut c_char,
                            dc.len() as c_int,
                            t as c_int,
                            dcap as c_int,
                            qr.as_ptr() as *const c_char,
                            eff as c_int,
                        );
                        let c = format!("{ctx}: partial contiguous t {t} cap {dcap}");
                        assert_eq!(x, y, "{c}: return mismatch (C={x} Rust={y})");
                        same_full_buffers(&c, &qc, &qr);
                        // extDict
                        let (mut rc2, mut rr2) = (dbuf(dcap.max(1)), dbuf(dcap.max(1)));
                        let x = (cd.safe_partial_usingDict)(
                            dr.as_ptr() as *const c_char,
                            rc2.as_mut_ptr() as *mut c_char,
                            dr.len() as c_int,
                            t as c_int,
                            dcap as c_int,
                            dtail.as_ptr() as *const c_char,
                            eff as c_int,
                        );
                        let y = (rd.safe_partial_usingDict)(
                            dc.as_ptr() as *const c_char,
                            rr2.as_mut_ptr() as *mut c_char,
                            dc.len() as c_int,
                            t as c_int,
                            dcap as c_int,
                            dtail.as_ptr() as *const c_char,
                            eff as c_int,
                        );
                        check(
                            &format!("{ctx}: partial extDict t {t} cap {dcap}"),
                            x,
                            y,
                            &rc2,
                            &rr2,
                            dcap.max(1),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn row_65_fast_using_dict_force_ext_dict_and_prefix64k() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(65);
    unsafe {
        for &dn in &[0usize, 8, 100, 4096, 32768, 65534, 65535, 65536, 65537, 100 * 1024] {
            for it in 0..180 {
                let s = shape(&mut rng);
                let dict = gen(&mut rng, s, dn);
                let n = one_size(&mut rng, 50000).max(1);
                let blk = straddling_block(&mut rng, &dict, n);
                let ctx = format!("row65 dict {dn} n {n} iter {it}");
                let (dc, dr, eff) = dict_block_pair(&cc, &rc, &dict, &blk, &ctx);
                let dtail = &dict[dict.len() - eff..];

                // ---- LZ4_decompress_fast_usingDict ----
                // (a) dictSize == 0 / prefix branch (dictStart+dictSize == dst)
                if eff == 0 {
                    let (mut oc, mut or) = (dbuf(n), dbuf(n));
                    let a = (cd.fast_usingDict)(
                        dr.as_ptr() as *const c_char,
                        oc.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        std::ptr::null(),
                        0,
                    );
                    let b = (rd.fast_usingDict)(
                        dc.as_ptr() as *const c_char,
                        or.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        std::ptr::null(),
                        0,
                    );
                    let c = format!("{ctx}: fast_usingDict dictSize 0");
                    assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                    assert_eq!(a, dr.len() as c_int, "{c}: bytes read");
                    assert_eq!(&oc[..n], &blk[..], "{c}: round-trip");
                    same_full_buffers(&c, &oc, &or);
                } else {
                    let mut pc = vec![SENT; eff + n + GUARD];
                    let mut pr = vec![SENT; eff + n + GUARD];
                    pc[..eff].copy_from_slice(dtail);
                    pr[..eff].copy_from_slice(dtail);
                    let a = (cd.fast_usingDict)(
                        dr.as_ptr() as *const c_char,
                        pc.as_mut_ptr().add(eff) as *mut c_char,
                        n as c_int,
                        pc.as_ptr() as *const c_char,
                        eff as c_int,
                    );
                    let b = (rd.fast_usingDict)(
                        dc.as_ptr() as *const c_char,
                        pr.as_mut_ptr().add(eff) as *mut c_char,
                        n as c_int,
                        pr.as_ptr() as *const c_char,
                        eff as c_int,
                    );
                    let c = format!("{ctx}: fast_usingDict contiguous (eff {eff})");
                    assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                    assert_eq!(a, dr.len() as c_int, "{c}: bytes read");
                    assert_eq!(&pc[eff..eff + n], &blk[..], "{c}: C round-trip");
                    same_full_buffers(&c, &pc, &pr);

                    // (b) separate ext dictionary
                    let (mut ec, mut er) = (dbuf(n), dbuf(n));
                    let a = (cd.fast_usingDict)(
                        dr.as_ptr() as *const c_char,
                        ec.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        dtail.as_ptr() as *const c_char,
                        eff as c_int,
                    );
                    let b = (rd.fast_usingDict)(
                        dc.as_ptr() as *const c_char,
                        er.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        dtail.as_ptr() as *const c_char,
                        eff as c_int,
                    );
                    let c = format!("{ctx}: fast_usingDict extDict (eff {eff})");
                    assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
                    assert_eq!(a, dr.len() as c_int, "{c}: bytes read");
                    assert_eq!(&ec[..n], &blk[..], "{c}: C round-trip");
                    same_full_buffers(&c, &ec, &er);

                    // ---- LZ4_decompress_safe_forceExtDict ----
                    // dictSize < 64 KB keeps checkOffset enabled, >= 64 KB
                    // disables it.
                    for &cap in &[n, n + 16, (n / 2).max(1)] {
                        let (mut fc, mut fr) = (dbuf(cap), dbuf(cap));
                        let a = (cd.safe_forceExtDict)(
                            dr.as_ptr() as *const c_char,
                            fc.as_mut_ptr() as *mut c_char,
                            dr.len() as c_int,
                            cap as c_int,
                            dtail.as_ptr() as *const c_void,
                            eff,
                        );
                        let b = (rd.safe_forceExtDict)(
                            dc.as_ptr() as *const c_char,
                            fr.as_mut_ptr() as *mut c_char,
                            dc.len() as c_int,
                            cap as c_int,
                            dtail.as_ptr() as *const c_void,
                            eff,
                        );
                        let c = format!("{ctx}: safe_forceExtDict cap {cap} eff {eff}");
                        check(&c, a, b, &fc, &fr, cap);
                        if cap >= n {
                            assert_eq!(a, n as c_int, "{c}: size");
                            assert_eq!(&fc[..n], &blk[..], "{c}: round-trip");
                        }
                    }
                    // ---- LZ4_decompress_safe_partial_forceExtDict ----
                    for &t in &[0usize, 1, 13, n / 3, n, n + 100] {
                        for &cap in &[n, (n / 2).max(1), t.max(1)] {
                            let (mut fc, mut fr) = (dbuf(cap), dbuf(cap));
                            let a = (cd.safe_partial_forceExtDict)(
                                dr.as_ptr() as *const c_char,
                                fc.as_mut_ptr() as *mut c_char,
                                dr.len() as c_int,
                                t as c_int,
                                cap as c_int,
                                dtail.as_ptr() as *const c_void,
                                eff,
                            );
                            let b = (rd.safe_partial_forceExtDict)(
                                dc.as_ptr() as *const c_char,
                                fr.as_mut_ptr() as *mut c_char,
                                dc.len() as c_int,
                                t as c_int,
                                cap as c_int,
                                dtail.as_ptr() as *const c_void,
                                eff,
                            );
                            check(
                                &format!("{ctx}: safe_partial_forceExtDict t {t} cap {cap}"),
                                a,
                                b,
                                &fc,
                                &fr,
                                cap,
                            );
                        }
                    }
                }
            }
        }

        // ---- legacy 64 KB-prefix decoders ----
        // A dictionary of exactly 64 KB laid immediately before dst.
        for it in 0..900 {
            let s = shape(&mut rng);
            let dict = gen(&mut rng, s, 65536);
            let n = one_size(&mut rng, 50000).max(1);
            let blk = straddling_block(&mut rng, &dict, n);
            let ctx = format!("row65/prefix64k iter {it} n {n}");
            let (dc, dr, eff) = dict_block_pair(&cc, &rc, &dict, &blk, &ctx);
            assert_eq!(eff, 65536);

            let mut pc = vec![SENT; 65536 + n + GUARD];
            let mut pr = vec![SENT; 65536 + n + GUARD];
            pc[..65536].copy_from_slice(&dict);
            pr[..65536].copy_from_slice(&dict);
            let a = (cd.safe_withPrefix64k)(
                dr.as_ptr() as *const c_char,
                pc.as_mut_ptr().add(65536) as *mut c_char,
                dr.len() as c_int,
                n as c_int,
            );
            let b = (rd.safe_withPrefix64k)(
                dc.as_ptr() as *const c_char,
                pr.as_mut_ptr().add(65536) as *mut c_char,
                dc.len() as c_int,
                n as c_int,
            );
            let c = format!("{ctx}: safe_withPrefix64k");
            assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
            assert_eq!(a, n as c_int, "{c}: size");
            assert_eq!(&pc[65536..65536 + n], &blk[..], "{c}: C round-trip");
            same_full_buffers(&c, &pc, &pr);

            // too-small dstCapacity must fail identically
            for &cap in &[(n / 2).max(1), 1usize] {
                let mut qc = vec![SENT; 65536 + cap + GUARD];
                let mut qr = vec![SENT; 65536 + cap + GUARD];
                qc[..65536].copy_from_slice(&dict);
                qr[..65536].copy_from_slice(&dict);
                let x = (cd.safe_withPrefix64k)(
                    dr.as_ptr() as *const c_char,
                    qc.as_mut_ptr().add(65536) as *mut c_char,
                    dr.len() as c_int,
                    cap as c_int,
                );
                let y = (rd.safe_withPrefix64k)(
                    dc.as_ptr() as *const c_char,
                    qr.as_mut_ptr().add(65536) as *mut c_char,
                    dc.len() as c_int,
                    cap as c_int,
                );
                assert_eq!(
                    x, y,
                    "{c}: safe_withPrefix64k cap {cap} return mismatch (C={x} Rust={y})"
                );
                same_full_buffers(&format!("{c} cap {cap}"), &qc, &qr);
            }

            let mut fc = vec![SENT; 65536 + n + GUARD];
            let mut fr = vec![SENT; 65536 + n + GUARD];
            fc[..65536].copy_from_slice(&dict);
            fr[..65536].copy_from_slice(&dict);
            let a = (cd.fast_withPrefix64k)(
                dr.as_ptr() as *const c_char,
                fc.as_mut_ptr().add(65536) as *mut c_char,
                n as c_int,
            );
            let b = (rd.fast_withPrefix64k)(
                dc.as_ptr() as *const c_char,
                fr.as_mut_ptr().add(65536) as *mut c_char,
                n as c_int,
            );
            let c = format!("{ctx}: fast_withPrefix64k");
            assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
            assert_eq!(a, dr.len() as c_int, "{c}: bytes read");
            assert_eq!(&fc[65536..65536 + n], &blk[..], "{c}: C round-trip");
            same_full_buffers(&c, &fc, &fr);
        }

        // forceExtDict with dictSize == 0, and with a dictionary that happens
        // to be contiguous with dst (still forced through the extDict path).
        for it in 0..30 {
            let n = one_size(&mut rng, 40000).max(1);
            let s = shape(&mut rng);
            let blk = gen(&mut rng, s, n);
            let ctx = format!("row65/forceExtDict-dict0 iter {it} n {n}");
            let (dc, dr, eff) = dict_block_pair(&cc, &rc, &[], &blk, &ctx);
            assert_eq!(eff, 0);
            let (mut oc, mut or) = (dbuf(n), dbuf(n));
            let a = (cd.safe_forceExtDict)(
                dr.as_ptr() as *const c_char,
                oc.as_mut_ptr() as *mut c_char,
                dr.len() as c_int,
                n as c_int,
                std::ptr::null(),
                0,
            );
            let b = (rd.safe_forceExtDict)(
                dc.as_ptr() as *const c_char,
                or.as_mut_ptr() as *mut c_char,
                dc.len() as c_int,
                n as c_int,
                std::ptr::null(),
                0,
            );
            let c = format!("{ctx}: safe_forceExtDict dictSize 0");
            assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
            assert_eq!(a, n as c_int, "{c}: size");
            assert_eq!(&oc[..n], &blk[..], "{c}: round-trip");
            same_full_buffers(&c, &oc, &or);
            for &t in &[0usize, 1, n / 2, n, n + 5] {
                let (mut pc, mut pr) = (dbuf(n), dbuf(n));
                let x = (cd.safe_partial_forceExtDict)(
                    dr.as_ptr() as *const c_char,
                    pc.as_mut_ptr() as *mut c_char,
                    dr.len() as c_int,
                    t as c_int,
                    n as c_int,
                    std::ptr::null(),
                    0,
                );
                let y = (rd.safe_partial_forceExtDict)(
                    dc.as_ptr() as *const c_char,
                    pr.as_mut_ptr() as *mut c_char,
                    dc.len() as c_int,
                    t as c_int,
                    n as c_int,
                    std::ptr::null(),
                    0,
                );
                check(
                    &format!("{ctx}: safe_partial_forceExtDict dictSize 0 t {t}"),
                    x,
                    y,
                    &pc,
                    &pr,
                    n,
                );
            }

            // dictionary contiguous with dst, but decoded through forceExtDict
            let dn = one_size(&mut rng, 90000).max(8);
            let dict = gen(&mut rng, Shape::TextLike, dn);
            let blk2 = straddling_block(&mut rng, &dict, n);
            let ctx2 = format!("row65/forceExtDict-contiguous iter {it} dict {dn}");
            let (dc2, dr2, eff2) = dict_block_pair(&cc, &rc, &dict, &blk2, &ctx2);
            let dtail2 = &dict[dict.len() - eff2..];
            let mut qc = vec![SENT; eff2 + n + GUARD];
            let mut qr = vec![SENT; eff2 + n + GUARD];
            qc[..eff2].copy_from_slice(dtail2);
            qr[..eff2].copy_from_slice(dtail2);
            let a = (cd.safe_forceExtDict)(
                dr2.as_ptr() as *const c_char,
                qc.as_mut_ptr().add(eff2) as *mut c_char,
                dr2.len() as c_int,
                n as c_int,
                qc.as_ptr() as *const c_void,
                eff2,
            );
            let b = (rd.safe_forceExtDict)(
                dc2.as_ptr() as *const c_char,
                qr.as_mut_ptr().add(eff2) as *mut c_char,
                dc2.len() as c_int,
                n as c_int,
                qr.as_ptr() as *const c_void,
                eff2,
            );
            let c = format!("{ctx2}: safe_forceExtDict contiguous (eff {eff2})");
            assert_eq!(a, b, "{c}: return mismatch (C={a} Rust={b})");
            assert_eq!(a, n as c_int, "{c}: size");
            assert_eq!(&qc[eff2..eff2 + n], &blk2[..], "{c}: C round-trip");
            same_full_buffers(&c, &qc, &qr);
        }
    }
}

// ===========================================================================
// FUZZ - corrupted / truncated chains (rows 54-62 decode paths)
// ===========================================================================

#[test]
fn fuzz_rows_54_62_corrupted_and_truncated_chains() {
    let (cc, rc) = capi();
    let (cd, rd) = dapi();
    let mut rng = Rng::new(1061);
    unsafe {
        for it in 0..13500 {
            let nb = rng.range(2, 12);
            let sz: Vec<usize> = (0..nb).map(|_| rng.range(16, 20000)).collect();
            let src = flat_input(&mut rng, &sz);
            let total: usize = sz.iter().sum();
            let ctx = format!("fuzz iter {it}");
            let (bc, br) = chain_prefix(&cc, &rc, &src, &sz, 1, &ctx);

            // Damage one or more blocks: bit flips, byte substitutions and
            // truncation. Both libraries get *identical* damaged input.
            let mut tc: Vec<Vec<u8>> = bc.clone();
            let mut tr: Vec<Vec<u8>> = br.clone();
            let ndmg = rng.range(1, 3);
            let mut how = Vec::new();
            for _ in 0..ndmg {
                let i = rng.below(nb);
                if tc[i].is_empty() {
                    continue; // already truncated away by an earlier round
                }
                match rng.below(3) {
                    0 => {
                        let at = rng.below(tc[i].len());
                        let x = rng.byte() | 1;
                        tc[i][at] ^= x;
                        tr[i][at] ^= x;
                        how.push(format!("flip block {i} @{at}"));
                    }
                    1 => {
                        let at = rng.below(tc[i].len());
                        let x = rng.byte();
                        tc[i][at] = x;
                        tr[i][at] = x;
                        how.push(format!("set block {i} @{at}={x}"));
                    }
                    _ => {
                        let keep = rng.below(tc[i].len().max(1));
                        tc[i].truncate(keep);
                        tr[i].truncate(keep);
                        how.push(format!("truncate block {i} to {keep}"));
                    }
                }
            }
            let ctx = format!("{ctx} [{}]", how.join(", "));

            // Keep driving BOTH decoders over the whole damaged chain and
            // require identical results at identical steps.
            let (mut oc, mut or) = (dbuf(total + 4096), dbuf(total + 4096));
            let sdc = (cd.createStreamDecode)();
            let sdr = (rd.createStreamDecode)();
            let mut off = 0usize;
            for (i, &n) in sz.iter().enumerate() {
                let cap = n.min(total + 4096 - off);
                let a = (cd.safe_continue)(
                    sdc,
                    tr[i].as_ptr() as *const c_char,
                    oc.as_mut_ptr().add(off) as *mut c_char,
                    tr[i].len() as c_int,
                    cap as c_int,
                );
                let b = (rd.safe_continue)(
                    sdr,
                    tc[i].as_ptr() as *const c_char,
                    or.as_mut_ptr().add(off) as *mut c_char,
                    tc[i].len() as c_int,
                    cap as c_int,
                );
                let c = format!("{ctx}: step {i} (len {n}, cap {cap})");
                assert_eq!(
                    a, b,
                    "{c}: C and Rust disagree on a damaged block (C={a} Rust={b})"
                );
                if a > 0 {
                    assert_eq!(
                        &oc[off..off + a as usize],
                        &or[off..off + a as usize],
                        "{c}: decoded bytes differ"
                    );
                    off += a as usize;
                }
            }
            assert_eq!((cd.freeStreamDecode)(sdc), (rd.freeStreamDecode)(sdr));
            same_full_buffers(&format!("{ctx}: full buffers"), &oc, &or);
        }

        // Truncated / corrupted input for the stateless dictionary decoders.
        for it in 0..8400 {
            let dn = one_size(&mut rng, 80000);
            let s = shape(&mut rng);
            let dict = gen(&mut rng, s, dn);
            let n = one_size(&mut rng, 30000).max(1);
            let blk = straddling_block(&mut rng, &dict, n);
            let ctx = format!("fuzz/usingDict iter {it} dict {dn} n {n}");
            let (dc, dr, eff) = dict_block_pair(&cc, &rc, &dict, &blk, &ctx);
            let dtail = &dict[dict.len() - eff..];
            let mut tc = dc.clone();
            let mut tr = dr.clone();
            match rng.below(3) {
                0 => {
                    let at = rng.below(tc.len());
                    let x = rng.byte() | 1;
                    tc[at] ^= x;
                    tr[at] ^= x;
                }
                1 => {
                    let keep = rng.below(tc.len().max(1));
                    tc.truncate(keep);
                    tr.truncate(keep);
                }
                _ => {
                    let at = rng.below(tc.len());
                    tc[at] = 0xFF;
                    tr[at] = 0xFF;
                }
            }
            for &cap in &[n, (n / 2).max(1), 1usize] {
                let (mut oc, mut or) = (dbuf(cap), dbuf(cap));
                let dp = if eff == 0 {
                    std::ptr::null()
                } else {
                    dtail.as_ptr() as *const c_char
                };
                let a = (cd.safe_usingDict)(
                    tr.as_ptr() as *const c_char,
                    oc.as_mut_ptr() as *mut c_char,
                    tr.len() as c_int,
                    cap as c_int,
                    dp,
                    eff as c_int,
                );
                let b = (rd.safe_usingDict)(
                    tc.as_ptr() as *const c_char,
                    or.as_mut_ptr() as *mut c_char,
                    tc.len() as c_int,
                    cap as c_int,
                    dp,
                    eff as c_int,
                );
                check(
                    &format!("{ctx}: safe_usingDict damaged cap {cap}"),
                    a,
                    b,
                    &oc,
                    &or,
                    cap,
                );
                let (mut pc, mut pr) = (dbuf(cap), dbuf(cap));
                let x = (cd.safe_partial_usingDict)(
                    tr.as_ptr() as *const c_char,
                    pc.as_mut_ptr() as *mut c_char,
                    tr.len() as c_int,
                    cap as c_int,
                    cap as c_int,
                    dp,
                    eff as c_int,
                );
                let y = (rd.safe_partial_usingDict)(
                    tc.as_ptr() as *const c_char,
                    pr.as_mut_ptr() as *mut c_char,
                    tc.len() as c_int,
                    cap as c_int,
                    cap as c_int,
                    dp,
                    eff as c_int,
                );
                check(
                    &format!("{ctx}: safe_partial_usingDict damaged cap {cap}"),
                    x,
                    y,
                    &pc,
                    &pr,
                    cap,
                );
                if eff > 0 {
                    let (mut fc, mut fr) = (dbuf(cap), dbuf(cap));
                    let x = (cd.safe_forceExtDict)(
                        tr.as_ptr() as *const c_char,
                        fc.as_mut_ptr() as *mut c_char,
                        tr.len() as c_int,
                        cap as c_int,
                        dtail.as_ptr() as *const c_void,
                        eff,
                    );
                    let y = (rd.safe_forceExtDict)(
                        tc.as_ptr() as *const c_char,
                        fr.as_mut_ptr() as *mut c_char,
                        tc.len() as c_int,
                        cap as c_int,
                        dtail.as_ptr() as *const c_void,
                        eff,
                    );
                    check(
                        &format!("{ctx}: safe_forceExtDict damaged cap {cap}"),
                        x,
                        y,
                        &fc,
                        &fr,
                        cap,
                    );
                }
            }
        }
    }
}

