//! Phase B differential tests for the LZ4 block *streaming* API (lz4.c):
//! `LZ4_stream_t` / `LZ4_streamDecode_t`, dictionaries, ring buffers and the
//! deprecated streaming wrappers.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnInitStream = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnVoidStream = unsafe extern "C" fn(*mut c_void);
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnLoadDictInternal = unsafe extern "C" fn(*mut c_void, *const c_char, c_int, c_int) -> c_int;
type FnAttach = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnContinue4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnContinue5 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FnSetStreamDecode = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnDecodeContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnDecodeFastContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnForceExtDict = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnSlide = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
type FnResetStreamState = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
type FnCreateLegacy = unsafe extern "C" fn(*mut c_char) -> *mut c_void;
type FnBound = unsafe extern "C" fn(c_int) -> c_int;

struct Api {
    create_stream: FnCreate,
    free_stream: FnFree,
    init_stream: FnInitStream,
    reset_stream: FnVoidStream,
    reset_stream_fast: FnVoidStream,
    load_dict: FnLoadDict,
    load_dict_slow: FnLoadDict,
    load_dict_internal: FnLoadDictInternal,
    attach: FnAttach,
    compress_continue: FnContinue,
    compress_continue_dep: FnContinue4,
    compress_continue_limited: FnContinue5,
    force_ext_dict: FnForceExtDict,
    save_dict: FnSaveDict,
    create_decode: FnCreate,
    free_decode: FnFree,
    set_stream_decode: FnSetStreamDecode,
    decode_continue: FnDecodeContinue,
    decode_fast_continue: FnDecodeFastContinue,
    ring_size: FnBound,
    bound: FnBound,
    sizeof_stream_state: FnIntVoid,
    reset_stream_state: FnResetStreamState,
    create_legacy: FnCreateLegacy,
    slide: FnSlide,
}

fn bind(l: &Lib) -> Api {
    Api {
        create_stream: l.sym("LZ4_createStream"),
        free_stream: l.sym("LZ4_freeStream"),
        init_stream: l.sym("LZ4_initStream"),
        reset_stream: l.sym("LZ4_resetStream"),
        reset_stream_fast: l.sym("LZ4_resetStream_fast"),
        load_dict: l.sym("LZ4_loadDict"),
        load_dict_slow: l.sym("LZ4_loadDictSlow"),
        load_dict_internal: l.sym("LZ4_loadDict_internal"),
        attach: l.sym("LZ4_attach_dictionary"),
        compress_continue: l.sym("LZ4_compress_fast_continue"),
        compress_continue_dep: l.sym("LZ4_compress_continue"),
        compress_continue_limited: l.sym("LZ4_compress_limitedOutput_continue"),
        force_ext_dict: l.sym("LZ4_compress_forceExtDict"),
        save_dict: l.sym("LZ4_saveDict"),
        create_decode: l.sym("LZ4_createStreamDecode"),
        free_decode: l.sym("LZ4_freeStreamDecode"),
        set_stream_decode: l.sym("LZ4_setStreamDecode"),
        decode_continue: l.sym("LZ4_decompress_safe_continue"),
        decode_fast_continue: l.sym("LZ4_decompress_fast_continue"),
        ring_size: l.sym("LZ4_decoderRingBufferSize"),
        bound: l.sym("LZ4_compressBound"),
        sizeof_stream_state: l.sym("LZ4_sizeofStreamState"),
        reset_stream_state: l.sym("LZ4_resetStreamState"),
        create_legacy: l.sym("LZ4_create"),
        slide: l.sym("LZ4_slideInputBuffer"),
    }
}

fn pair() -> (Api, Api) {
    let p = libs();
    (bind(&p.c), bind(&p.r))
}

/// Split `len` into a random sequence of block sizes.
fn split(rng: &mut Rng, len: usize, maxblk: usize) -> Vec<usize> {
    let mut v = Vec::new();
    let mut left = len;
    while left > 0 {
        let n = rng.range(1, left.min(maxblk) + 1);
        v.push(n);
        left -= n;
    }
    if v.is_empty() {
        v.push(0);
    }
    v
}

// --- CONFIGS: LZ4_initStream over sizes/alignments (incl. ERRORS rows) -------
#[test]
fn stream_init_and_reset() {
    let (c, r) = pair();
    let ss = unsafe { (c.sizeof_stream_state)() } as usize;
    assert_eq!(ss, unsafe { (r.sizeof_stream_state)() } as usize);
    let mut buf = vec![0u8; ss + 64];
    // NULL buffer -> NULL
    assert_eq!(
        unsafe { (c.init_stream)(std::ptr::null_mut(), ss) }.is_null(),
        unsafe { (r.init_stream)(std::ptr::null_mut(), ss) }.is_null(),
        "LZ4_initStream(NULL)"
    );
    // undersized -> NULL ; exact & oversized -> the buffer itself
    for size in [0usize, 1, ss - 1, ss, ss + 1, ss + 64] {
        let cp = unsafe { (c.init_stream)(buf.as_mut_ptr() as *mut c_void, size) };
        let rp = unsafe { (r.init_stream)(buf.as_mut_ptr() as *mut c_void, size) };
        assert_eq!(cp.is_null(), rp.is_null(), "LZ4_initStream(size={})", size);
        if !cp.is_null() {
            assert_eq!(cp, buf.as_mut_ptr() as *mut c_void);
            assert_eq!(rp, buf.as_mut_ptr() as *mut c_void);
        }
    }
    // resetStream / resetStream_fast / resetStreamState must leave identical state
    let mut cbuf = vec![0xEEu8; ss];
    let mut rbuf = vec![0xEEu8; ss];
    unsafe {
        (c.reset_stream)(cbuf.as_mut_ptr() as *mut c_void);
        (r.reset_stream)(rbuf.as_mut_ptr() as *mut c_void);
    }
    assert_bytes_eq("LZ4_resetStream", &cbuf, &rbuf);
    unsafe {
        (c.reset_stream_fast)(cbuf.as_mut_ptr() as *mut c_void);
        (r.reset_stream_fast)(rbuf.as_mut_ptr() as *mut c_void);
    }
    assert_bytes_eq("LZ4_resetStream_fast", &cbuf, &rbuf);
    let mut cbuf = vec![0x11u8; ss];
    let mut rbuf = vec![0x11u8; ss];
    let a = unsafe {
        (c.reset_stream_state)(cbuf.as_mut_ptr() as *mut c_void, std::ptr::null_mut())
    };
    let b = unsafe {
        (r.reset_stream_state)(rbuf.as_mut_ptr() as *mut c_void, std::ptr::null_mut())
    };
    assert_eq!(a, b, "LZ4_resetStreamState rc");
    assert_bytes_eq("LZ4_resetStreamState", &cbuf, &rbuf);
    // createStream / freeStream(NULL)
    unsafe {
        let cs = (c.create_stream)();
        let rs = (r.create_stream)();
        assert!(!cs.is_null() && !rs.is_null());
        assert_bytes_eq(
            "LZ4_createStream initial state",
            std::slice::from_raw_parts(cs as *const u8, ss),
            std::slice::from_raw_parts(rs as *const u8, ss),
        );
        assert_eq!((c.free_stream)(cs), (r.free_stream)(rs));
        assert_eq!(
            (c.free_stream)(std::ptr::null_mut()),
            (r.free_stream)(std::ptr::null_mut()),
            "LZ4_freeStream(NULL)"
        );
        // legacy LZ4_create / LZ4_slideInputBuffer
        let cs = (c.create_legacy)(std::ptr::null_mut());
        let rs = (r.create_legacy)(std::ptr::null_mut());
        assert!(!cs.is_null() && !rs.is_null());
        // fresh stream: dictionary == NULL for both
        assert_eq!(
            (c.slide)(cs).is_null(),
            (r.slide)(rs).is_null(),
            "LZ4_slideInputBuffer on fresh stream"
        );
        (c.free_stream)(cs);
        (r.free_stream)(rs);
        // decode-side create/free
        let cd = (c.create_decode)();
        let rd = (r.create_decode)();
        assert!(!cd.is_null() && !rd.is_null());
        assert_bytes_eq(
            "LZ4_createStreamDecode initial state",
            std::slice::from_raw_parts(cd as *const u8, LZ4_STREAMDECODESIZE),
            std::slice::from_raw_parts(rd as *const u8, LZ4_STREAMDECODESIZE),
        );
        assert_eq!((c.free_decode)(cd), (r.free_decode)(rd));
        assert_eq!(
            (c.free_decode)(std::ptr::null_mut()),
            (r.free_decode)(std::ptr::null_mut()),
            "LZ4_freeStreamDecode(NULL)"
        );
    }
}

// --- CONFIGS: LZ4_loadDict / LZ4_loadDictSlow / LZ4_loadDict_internal --------
#[test]
fn stream_load_dict_variants() {
    let (c, r) = pair();
    let ss = unsafe { (c.sizeof_stream_state)() } as usize;
    let mut rng = Rng::new(0x2001);
    let dict_sizes = [0usize, 1, 3, 4, 8, 11, 12, 13, 64, 1000, 65535, 65536, 65537, 100_000];
    for &ds in &dict_sizes {
        for &shape in &[Shape::Text, Shape::Random, Shape::Runs, Shape::Periodic(9)] {
            let dict = gen(shape, ds, &mut rng);
            let dp = if ds == 0 {
                std::ptr::null()
            } else {
                dict.as_ptr() as *const c_char
            };
            for mode in 0..3 {
                unsafe {
                    let cs = (c.create_stream)();
                    let rs = (r.create_stream)();
                    let (a, b) = match mode {
                        0 => (
                            (c.load_dict)(cs, dp, ds as c_int),
                            (r.load_dict)(rs, dp, ds as c_int),
                        ),
                        1 => (
                            (c.load_dict_slow)(cs, dp, ds as c_int),
                            (r.load_dict_slow)(rs, dp, ds as c_int),
                        ),
                        _ => (
                            (c.load_dict_internal)(cs, dp, ds as c_int, 1),
                            (r.load_dict_internal)(rs, dp, ds as c_int, 1),
                        ),
                    };
                    assert_eq!(a, b, "loadDict(mode={}) rc ds={} shape={:?}", mode, ds, shape);
                    // The hash table content is part of the observable state that
                    // determines all future compressed output; compare the whole
                    // struct except the two raw pointers (dictionary/dictCtx),
                    // which legitimately differ (they point into our own buffers,
                    // so they are actually identical here as well).
                    let cst = std::slice::from_raw_parts(cs as *const u8, ss);
                    let rst = std::slice::from_raw_parts(rs as *const u8, ss);
                    assert_bytes_eq(
                        &format!("loadDict(mode={}) state ds={} shape={:?}", mode, ds, shape),
                        cst,
                        rst,
                    );
                    (c.free_stream)(cs);
                    (r.free_stream)(rs);
                }
            }
        }
    }
}

// --- CONFIGS: multi-block streaming compression, contiguous input ------------
#[test]
fn stream_compress_contiguous() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x2002);
    for iter in 0..80 {
        let total = rng.range(0, 150_000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let data = gen(shape, total, &mut rng);
        let maxblk = [1usize, 13, 100, 4096, 65536, 100_000][rng.below(6)];
        let blocks = split(&mut rng, total, maxblk);
        let acc = [1i32, 2, 0, 7][rng.below(4)];
        unsafe {
            let cs = (c.create_stream)();
            let rs = (r.create_stream)();
            let mut off = 0usize;
            let mut cout: Vec<Vec<u8>> = Vec::new();
            for (bi, &n) in blocks.iter().enumerate() {
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let src = data.as_ptr().add(off) as *const c_char;
                let a = (c.compress_continue)(
                    cs,
                    src,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    acc,
                );
                let b = (r.compress_continue)(
                    rs,
                    src,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    acc,
                );
                assert_eq!(
                    a, b,
                    "compress_fast_continue rc iter={} block={} n={} shape={:?} acc={}",
                    iter, bi, n, shape, acc
                );
                assert_bytes_eq(
                    &format!("compress_fast_continue iter={} block={} n={}", iter, bi, n),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                cout.push(cb[..a.max(0) as usize].to_vec());
                off += n;
            }
            // decode with both stream decoders into a contiguous buffer
            for which in 0..2 {
                let dec = if which == 0 {
                    (c.create_decode)()
                } else {
                    (r.create_decode)()
                };
                let mut out = vec![0u8; total + 64];
                let mut doff = 0usize;
                for (bi, blk) in cout.iter().enumerate() {
                    if blk.is_empty() {
                        continue;
                    }
                    let want = blocks[bi];
                    let got = if which == 0 {
                        (c.decode_continue)(
                            dec,
                            blk.as_ptr() as *const c_char,
                            out.as_mut_ptr().add(doff) as *mut c_char,
                            blk.len() as c_int,
                            (total + 64 - doff) as c_int,
                        )
                    } else {
                        (r.decode_continue)(
                            dec,
                            blk.as_ptr() as *const c_char,
                            out.as_mut_ptr().add(doff) as *mut c_char,
                            blk.len() as c_int,
                            (total + 64 - doff) as c_int,
                        )
                    };
                    assert_eq!(got, want as c_int, "decode_continue lib={} block={}", which, bi);
                    doff += got as usize;
                }
                assert_bytes_eq("streaming round-trip", &out[..total], &data);
                if which == 0 {
                    (c.free_decode)(dec);
                } else {
                    (r.free_decode)(dec);
                }
            }
            (c.free_stream)(cs);
            (r.free_stream)(rs);
        }
    }
}

// --- CONFIGS: streaming with saveDict between blocks ------------------------
#[test]
fn stream_compress_with_save_dict() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x2003);
    for iter in 0..60 {
        let total = rng.range(1, 120_000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let data = gen(shape, total, &mut rng);
        let maxblk = [13usize, 500, 4096, 30_000][rng.below(4)];
        let blocks = split(&mut rng, total, maxblk);
        let save_max = [0i32, 1, 100, 1000, 65536][rng.below(5)];
        unsafe {
            let cs = (c.create_stream)();
            let rs = (r.create_stream)();
            let mut cdict = vec![0u8; 70_000];
            let mut rdict = vec![0u8; 70_000];
            let mut off = 0usize;
            for (bi, &n) in blocks.iter().enumerate() {
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                // src must live in a buffer that stays valid; use the shared `data`
                let src = data.as_ptr().add(off) as *const c_char;
                let a = (c.compress_continue)(cs, src, cb.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int, 1);
                let b = (r.compress_continue)(rs, src, rb.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int, 1);
                assert_eq!(a, b, "saveDict flow rc iter={} block={}", iter, bi);
                assert_bytes_eq(
                    &format!("saveDict flow iter={} block={}", iter, bi),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                let sa = (c.save_dict)(cs, cdict.as_mut_ptr() as *mut c_char, save_max);
                let sb = (r.save_dict)(rs, rdict.as_mut_ptr() as *mut c_char, save_max);
                assert_eq!(sa, sb, "LZ4_saveDict rc iter={} block={} max={}", iter, bi, save_max);
                assert_bytes_eq(
                    &format!("LZ4_saveDict content iter={} block={}", iter, bi),
                    &cdict[..sa.max(0) as usize],
                    &rdict[..sb.max(0) as usize],
                );
                off += n;
            }
            (c.free_stream)(cs);
            (r.free_stream)(rs);
        }
    }
}

// --- CONFIGS: attached dictionary (LZ4_attach_dictionary) --------------------
#[test]
fn stream_attach_dictionary() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x2004);
    for &ds in &[0usize, 1, 13, 1000, 65536, 80_000] {
        let dict = gen(Shape::Text, ds, &mut rng);
        for &n in &[0usize, 1, 13, 1000, 50_000] {
            let mut data = gen(Shape::Text, n, &mut rng);
            let k = n.min(ds);
            if k > 0 {
                data[..k].copy_from_slice(&dict[..k]);
            }
            unsafe {
                let cdictS = (c.create_stream)();
                let rdictS = (r.create_stream)();
                let dp = if ds == 0 {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_char
                };
                (c.load_dict)(cdictS, dp, ds as c_int);
                (r.load_dict)(rdictS, dp, ds as c_int);
                let cs = (c.create_stream)();
                let rs = (r.create_stream)();
                (c.reset_stream_fast)(cs);
                (r.reset_stream_fast)(rs);
                (c.attach)(cs, cdictS as *const c_void);
                (r.attach)(rs, rdictS as *const c_void);
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = (c.compress_continue)(
                    cs,
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (r.compress_continue)(
                    rs,
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                assert_eq!(a, b, "attach_dictionary rc ds={} n={}", ds, n);
                assert_bytes_eq(
                    &format!("attach_dictionary ds={} n={}", ds, n),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                // attaching a NULL dictionary stream is explicitly allowed
                (c.reset_stream_fast)(cs);
                (r.reset_stream_fast)(rs);
                (c.attach)(cs, std::ptr::null());
                (r.attach)(rs, std::ptr::null());
                let mut cb2 = vec![0u8; cap];
                let mut rb2 = vec![0u8; cap];
                let a = (c.compress_continue)(
                    cs,
                    data.as_ptr() as *const c_char,
                    cb2.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (r.compress_continue)(
                    rs,
                    data.as_ptr() as *const c_char,
                    rb2.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                assert_eq!(a, b, "attach NULL dict rc ds={} n={}", ds, n);
                assert_bytes_eq("attach NULL dict", &cb2[..a.max(0) as usize], &rb2[..b.max(0) as usize]);
                (c.free_stream)(cs);
                (c.free_stream)(cdictS);
                (r.free_stream)(rs);
                (r.free_stream)(rdictS);
            }
        }
    }
}

// --- CONFIGS: LZ4_compress_forceExtDict -------------------------------------
#[test]
fn stream_force_ext_dict() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x2005);
    for &ds in &[0usize, 13, 1000, 65536] {
        let dict = gen(Shape::Text, ds, &mut rng);
        for &n in &[1usize, 13, 1000, 40_000] {
            let mut data = gen(Shape::Text, n, &mut rng);
            let k = n.min(ds);
            if k > 0 {
                data[..k].copy_from_slice(&dict[..k]);
            }
            unsafe {
                let cs = (c.create_stream)();
                let rs = (r.create_stream)();
                let dp = if ds == 0 {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_char
                };
                (c.load_dict)(cs, dp, ds as c_int);
                (r.load_dict)(rs, dp, ds as c_int);
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = (c.force_ext_dict)(
                    cs,
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                );
                let b = (r.force_ext_dict)(
                    rs,
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                );
                assert_eq!(a, b, "forceExtDict rc ds={} n={}", ds, n);
                assert_bytes_eq(
                    &format!("forceExtDict ds={} n={}", ds, n),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                (c.free_stream)(cs);
                (r.free_stream)(rs);
            }
        }
    }
}

// --- CONFIGS: deprecated streaming compressors ------------------------------
#[test]
fn stream_deprecated_continue() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x2006);
    for iter in 0..40 {
        let total = rng.range(1, 60_000);
        let data = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], total, &mut rng);
        let blocks = split(&mut rng, total, 8192);
        unsafe {
            let cs = (c.create_stream)();
            let rs = (r.create_stream)();
            let mut off = 0usize;
            for (bi, &n) in blocks.iter().enumerate() {
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let src = data.as_ptr().add(off) as *const c_char;
                let a = (c.compress_continue_dep)(cs, src, cb.as_mut_ptr() as *mut c_char, n as c_int);
                let b = (r.compress_continue_dep)(rs, src, rb.as_mut_ptr() as *mut c_char, n as c_int);
                assert_eq!(a, b, "LZ4_compress_continue rc iter={} blk={}", iter, bi);
                assert_bytes_eq("LZ4_compress_continue", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);
                off += n;
            }
            (c.free_stream)(cs);
            (r.free_stream)(rs);

            let cs = (c.create_stream)();
            let rs = (r.create_stream)();
            let mut off = 0usize;
            for (bi, &n) in blocks.iter().enumerate() {
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                for &lim in &[0usize, 1, cap / 2 + 1, cap] {
                    let mut cb = vec![0u8; lim + 1];
                    let mut rb = vec![0u8; lim + 1];
                    let src = data.as_ptr().add(off) as *const c_char;
                    let a = (c.compress_continue_limited)(
                        cs,
                        src,
                        cb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        lim as c_int,
                    );
                    let b = (r.compress_continue_limited)(
                        rs,
                        src,
                        rb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        lim as c_int,
                    );
                    assert_eq!(
                        a, b,
                        "LZ4_compress_limitedOutput_continue rc iter={} blk={} lim={}",
                        iter, bi, lim
                    );
                    assert_bytes_eq(
                        "LZ4_compress_limitedOutput_continue",
                        &cb[..a.max(0) as usize],
                        &rb[..b.max(0) as usize],
                    );
                }
                off += n;
            }
            (c.free_stream)(cs);
            (r.free_stream)(rs);
        }
    }
}

// --- CONFIGS: LZ4_setStreamDecode + safe/fast continue with external dict ----
#[test]
fn stream_decode_with_set_stream_decode() {
    let (c, r) = pair();
    let p = libs();
    let mut rng = Rng::new(0x2007);
    // produce dictionary-compressed blocks with the C compressor
    let create: FnCreate = p.c.sym("LZ4_createStream");
    let free: FnFree = p.c.sym("LZ4_freeStream");
    let load: FnLoadDict = p.c.sym("LZ4_loadDict");
    let cont: FnContinue = p.c.sym("LZ4_compress_fast_continue");

    for &ds in &[0usize, 1, 13, 1000, 65536, 90_000] {
        let dict = gen(Shape::Text, ds, &mut rng);
        for &n in &[1usize, 13, 500, 30_000] {
            let mut data = gen(Shape::Text, n, &mut rng);
            let k = n.min(ds);
            if k > 0 {
                data[..k].copy_from_slice(&dict[..k]);
            }
            let cap = (unsafe { (c.bound)(n as c_int) } as usize).max(1);
            let mut comp = vec![0u8; cap];
            let clen = unsafe {
                let s = create();
                let dp = if ds == 0 {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_char
                };
                load(s, dp, ds as c_int);
                let l = cont(
                    s,
                    data.as_ptr() as *const c_char,
                    comp.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                free(s);
                l
            };
            if clen <= 0 {
                continue;
            }
            unsafe {
                let cd = (c.create_decode)();
                let rd = (r.create_decode)();
                let dp = if ds == 0 {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_char
                };
                let a = (c.set_stream_decode)(cd, dp, ds as c_int);
                let b = (r.set_stream_decode)(rd, dp, ds as c_int);
                assert_eq!(a, b, "LZ4_setStreamDecode rc ds={}", ds);
                assert_bytes_eq(
                    "LZ4_setStreamDecode state",
                    std::slice::from_raw_parts(cd as *const u8, LZ4_STREAMDECODESIZE),
                    std::slice::from_raw_parts(rd as *const u8, LZ4_STREAMDECODESIZE),
                );
                let mut co = vec![0u8; n + 64];
                let mut ro = vec![0u8; n + 64];
                let x = (c.decode_continue)(
                    cd,
                    comp.as_ptr() as *const c_char,
                    co.as_mut_ptr() as *mut c_char,
                    clen,
                    (n + 64) as c_int,
                );
                let y = (r.decode_continue)(
                    rd,
                    comp.as_ptr() as *const c_char,
                    ro.as_mut_ptr() as *mut c_char,
                    clen,
                    (n + 64) as c_int,
                );
                assert_eq!(x, y, "decompress_safe_continue rc ds={} n={}", ds, n);
                assert_bytes_eq("decompress_safe_continue", &co, &ro);
                assert_eq!(x, n as c_int);
                (c.free_decode)(cd);
                (r.free_decode)(rd);

                // fast variant
                let cd = (c.create_decode)();
                let rd = (r.create_decode)();
                (c.set_stream_decode)(cd, dp, ds as c_int);
                (r.set_stream_decode)(rd, dp, ds as c_int);
                let mut co = vec![0u8; n + 64];
                let mut ro = vec![0u8; n + 64];
                let x = (c.decode_fast_continue)(
                    cd,
                    comp.as_ptr() as *const c_char,
                    co.as_mut_ptr() as *mut c_char,
                    n as c_int,
                );
                let y = (r.decode_fast_continue)(
                    rd,
                    comp.as_ptr() as *const c_char,
                    ro.as_mut_ptr() as *mut c_char,
                    n as c_int,
                );
                assert_eq!(x, y, "decompress_fast_continue rc ds={} n={}", ds, n);
                assert_bytes_eq("decompress_fast_continue", &co[..n], &ro[..n]);
                (c.free_decode)(cd);
                (r.free_decode)(rd);
            }
        }
    }
}

// --- CONFIGS: ring-buffer compression + ring-buffer decompression ------------
#[test]
fn stream_ring_buffer() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x2008);
    for &maxblk in &[16usize, 100, 1000, 8192] {
        let ring = unsafe { (c.ring_size)(maxblk as c_int) } as usize;
        assert_eq!(ring, unsafe { (r.ring_size)(maxblk as c_int) } as usize);
        // compress from a ring buffer of exactly `ring` bytes
        let total = 60_000usize;
        let data = gen(Shape::Text, total, &mut rng);
        unsafe {
            let cs = (c.create_stream)();
            let rs = (r.create_stream)();
            let mut cring = vec![0u8; ring];
            let mut rring = vec![0u8; ring];
            let mut cpos = 0usize;
            let mut rpos = 0usize;
            let mut off = 0usize;
            let mut blocks: Vec<(usize, Vec<u8>)> = Vec::new();
            while off < total {
                let n = rng.range(1, maxblk + 1).min(total - off);
                if cpos + n > ring {
                    cpos = 0;
                    rpos = 0;
                }
                cring[cpos..cpos + n].copy_from_slice(&data[off..off + n]);
                rring[rpos..rpos + n].copy_from_slice(&data[off..off + n]);
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = (c.compress_continue)(
                    cs,
                    cring.as_ptr().add(cpos) as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = (r.compress_continue)(
                    rs,
                    rring.as_ptr().add(rpos) as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                assert_eq!(a, b, "ring compress rc maxblk={} n={}", maxblk, n);
                assert_bytes_eq(
                    &format!("ring compress maxblk={} n={}", maxblk, n),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                blocks.push((n, cb[..a.max(0) as usize].to_vec()));
                cpos += n;
                rpos += n;
                off += n;
            }
            (c.free_stream)(cs);
            (r.free_stream)(rs);

            // now decode into a ring buffer with both decoders
            for which in 0..2 {
                let dec = if which == 0 {
                    (c.create_decode)()
                } else {
                    (r.create_decode)()
                };
                let mut dring = vec![0u8; ring];
                let mut pos = 0usize;
                let mut got_all = Vec::with_capacity(total);
                for (n, blk) in &blocks {
                    if pos + maxblk > ring {
                        pos = 0;
                    }
                    let got = if which == 0 {
                        (c.decode_continue)(
                            dec,
                            blk.as_ptr() as *const c_char,
                            dring.as_mut_ptr().add(pos) as *mut c_char,
                            blk.len() as c_int,
                            (ring - pos) as c_int,
                        )
                    } else {
                        (r.decode_continue)(
                            dec,
                            blk.as_ptr() as *const c_char,
                            dring.as_mut_ptr().add(pos) as *mut c_char,
                            blk.len() as c_int,
                            (ring - pos) as c_int,
                        )
                    };
                    assert_eq!(got, *n as c_int, "ring decode lib={} maxblk={}", which, maxblk);
                    got_all.extend_from_slice(&dring[pos..pos + got as usize]);
                    pos += got as usize;
                }
                assert_bytes_eq("ring round-trip", &got_all, &data);
                if which == 0 {
                    (c.free_decode)(dec);
                } else {
                    (r.free_decode)(dec);
                }
            }
        }
    }
}

// --- CONFIGS: dictionary loaded then continued (dictionary + stream) ---------
#[test]
fn stream_load_dict_then_continue() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x2009);
    for &ds in &[0usize, 13, 1000, 65536, 80_000] {
        let dict = gen(Shape::Text, ds, &mut rng);
        let total = 40_000usize;
        let mut data = gen(Shape::Text, total, &mut rng);
        let k = total.min(ds);
        if k > 0 {
            data[..k].copy_from_slice(&dict[..k]);
        }
        for &slow in &[false, true] {
            unsafe {
                let cs = (c.create_stream)();
                let rs = (r.create_stream)();
                let dp = if ds == 0 {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_char
                };
                if slow {
                    (c.load_dict_slow)(cs, dp, ds as c_int);
                    (r.load_dict_slow)(rs, dp, ds as c_int);
                } else {
                    (c.load_dict)(cs, dp, ds as c_int);
                    (r.load_dict)(rs, dp, ds as c_int);
                }
                let blocks = split(&mut rng, total, 5000);
                let mut off = 0usize;
                for (bi, &n) in blocks.iter().enumerate() {
                    let cap = ((c.bound)(n as c_int) as usize).max(1);
                    let mut cb = vec![0u8; cap];
                    let mut rb = vec![0u8; cap];
                    let src = data.as_ptr().add(off) as *const c_char;
                    let a = (c.compress_continue)(cs, src, cb.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int, 1);
                    let b = (r.compress_continue)(rs, src, rb.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int, 1);
                    assert_eq!(a, b, "dict+continue rc ds={} slow={} blk={}", ds, slow, bi);
                    assert_bytes_eq(
                        &format!("dict+continue ds={} slow={} blk={}", ds, slow, bi),
                        &cb[..a.max(0) as usize],
                        &rb[..b.max(0) as usize],
                    );
                    off += n;
                }
                (c.free_stream)(cs);
                (r.free_stream)(rs);
            }
        }
    }
}
