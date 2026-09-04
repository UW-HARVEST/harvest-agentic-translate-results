//! Phase B differential tests for the LZ4 HC API (lz4hc.c), including all
//! compression levels, the streaming HC API, dictionaries and the deprecated
//! `LZ4_compressHC*` wrappers.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

type FnHC = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnHCExt =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnHCDestSize =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;
type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnInitHC = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnResetHC = unsafe extern "C" fn(*mut c_void, c_int);
type FnLoadDictHC = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnAttachHC = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnHCContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnHCContinueDestSize =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
type FnSaveDictHC = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FnSetLevel = unsafe extern "C" fn(*mut c_void, c_int);
type FnFavor = unsafe extern "C" fn(*mut c_void, c_int);
type FnBound = unsafe extern "C" fn(c_int) -> c_int;
type FnDecompressSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
// deprecated
type Fn3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type Fn4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type Fn5 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnS4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnS5 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnS6 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnCreateHCLegacy = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type FnSlideHC = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
type FnResetStateHC = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;

struct Api {
    compress_hc: FnHC,
    sizeof_state_hc: FnIntVoid,
    ext_state_hc: FnHCExt,
    ext_state_hc_fastreset: FnHCExt,
    hc_dest_size: FnHCDestSize,
    create_hc: FnCreate,
    free_hc: FnFree,
    init_hc: FnInitHC,
    reset_hc: FnResetHC,
    reset_hc_fast: FnResetHC,
    load_dict_hc: FnLoadDictHC,
    attach_hc: FnAttachHC,
    hc_continue: FnHCContinue,
    hc_continue_dest_size: FnHCContinueDestSize,
    save_dict_hc: FnSaveDictHC,
    set_level: FnSetLevel,
    favor: FnFavor,
    bound: FnBound,
    decompress_safe: FnDecompressSafe,
    // deprecated
    compresshc: Fn3,
    compresshc_limited: Fn4,
    compresshc2: Fn4,
    compresshc2_limited: Fn5,
    compresshc_with_state: FnS4,
    compresshc_limited_with_state: FnS5,
    compresshc2_with_state: FnS5,
    compresshc2_limited_with_state: FnS6,
    compresshc_continue: FnS4,
    compresshc_limited_continue: FnS5,
    compresshc2_continue: FnS5,
    compresshc2_limited_continue: FnS6,
    sizeof_stream_state_hc: FnIntVoid,
    reset_stream_state_hc: FnResetStateHC,
    create_hc_legacy: FnCreateHCLegacy,
    free_hc_legacy: FnFree,
    slide_hc: FnSlideHC,
}

fn bind(l: &Lib) -> Api {
    Api {
        compress_hc: l.sym("LZ4_compress_HC"),
        sizeof_state_hc: l.sym("LZ4_sizeofStateHC"),
        ext_state_hc: l.sym("LZ4_compress_HC_extStateHC"),
        ext_state_hc_fastreset: l.sym("LZ4_compress_HC_extStateHC_fastReset"),
        hc_dest_size: l.sym("LZ4_compress_HC_destSize"),
        create_hc: l.sym("LZ4_createStreamHC"),
        free_hc: l.sym("LZ4_freeStreamHC"),
        init_hc: l.sym("LZ4_initStreamHC"),
        reset_hc: l.sym("LZ4_resetStreamHC"),
        reset_hc_fast: l.sym("LZ4_resetStreamHC_fast"),
        load_dict_hc: l.sym("LZ4_loadDictHC"),
        attach_hc: l.sym("LZ4_attach_HC_dictionary"),
        hc_continue: l.sym("LZ4_compress_HC_continue"),
        hc_continue_dest_size: l.sym("LZ4_compress_HC_continue_destSize"),
        save_dict_hc: l.sym("LZ4_saveDictHC"),
        set_level: l.sym("LZ4_setCompressionLevel"),
        favor: l.sym("LZ4_favorDecompressionSpeed"),
        bound: l.sym("LZ4_compressBound"),
        decompress_safe: l.sym("LZ4_decompress_safe"),
        compresshc: l.sym("LZ4_compressHC"),
        compresshc_limited: l.sym("LZ4_compressHC_limitedOutput"),
        compresshc2: l.sym("LZ4_compressHC2"),
        compresshc2_limited: l.sym("LZ4_compressHC2_limitedOutput"),
        compresshc_with_state: l.sym("LZ4_compressHC_withStateHC"),
        compresshc_limited_with_state: l.sym("LZ4_compressHC_limitedOutput_withStateHC"),
        compresshc2_with_state: l.sym("LZ4_compressHC2_withStateHC"),
        compresshc2_limited_with_state: l.sym("LZ4_compressHC2_limitedOutput_withStateHC"),
        compresshc_continue: l.sym("LZ4_compressHC_continue"),
        compresshc_limited_continue: l.sym("LZ4_compressHC_limitedOutput_continue"),
        compresshc2_continue: l.sym("LZ4_compressHC2_continue"),
        compresshc2_limited_continue: l.sym("LZ4_compressHC2_limitedOutput_continue"),
        sizeof_stream_state_hc: l.sym("LZ4_sizeofStreamStateHC"),
        reset_stream_state_hc: l.sym("LZ4_resetStreamStateHC"),
        create_hc_legacy: l.sym("LZ4_createHC"),
        free_hc_legacy: l.sym("LZ4_freeHC"),
        slide_hc: l.sym("LZ4_slideInputBufferHC"),
    }
}

fn pair() -> (Api, Api) {
    let p = libs();
    (bind(&p.c), bind(&p.r))
}

/// Every level worth distinguishing: below min, min, the LZ4MID range,
/// hash-chain range, optimal-parser range, max, and above max.
const LEVELS: &[c_int] = &[
    i32::MIN,
    -100,
    -1,
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    100,
    i32::MAX,
];

fn corpus(rng: &mut Rng) -> Vec<(Shape, usize, Vec<u8>)> {
    let mut out = Vec::new();
    for &shape in ALL_SHAPES {
        for &n in &[0usize, 1, 4, 12, 13, 16, 63, 64, 65, 255, 1024, 4096, 65535, 65536, 65537] {
            out.push((shape, n, gen(shape, n, rng)));
        }
    }
    for _ in 0..40 {
        let n = rng.range(0, 30_000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        out.push((shape, n, gen(shape, n, rng)));
    }
    out
}

// --- CONFIGS: LZ4_compress_HC over every level -------------------------------
#[test]
fn hc_compress_all_levels() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x3001);
    for (shape, n, data) in corpus(&mut rng) {
        let cap = (unsafe { (c.bound)(n as c_int) } as usize).max(1);
        for &lvl in LEVELS {
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
            assert_eq!(a, b, "LZ4_compress_HC rc shape={:?} n={} lvl={}", shape, n, lvl);
            assert_bytes_eq(
                &format!("LZ4_compress_HC shape={:?} n={} lvl={}", shape, n, lvl),
                &cb[..a.max(0) as usize],
                &rb[..b.max(0) as usize],
            );
            if a > 0 {
                let mut o = vec![0u8; n + 16];
                let got = unsafe {
                    (r.decompress_safe)(
                        cb.as_ptr() as *const c_char,
                        o.as_mut_ptr() as *mut c_char,
                        a,
                        (n + 16) as c_int,
                    )
                };
                assert_eq!(got, n as c_int, "HC output not decodable lvl={}", lvl);
                assert_bytes_eq("HC round-trip", &o[..n], &data);
            }
        }
    }
}

// --- CONFIGS: dstCapacity pressure at every level ---------------------------
#[test]
fn hc_compress_limited_capacity() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x3002);
    for &shape in &[Shape::Text, Shape::Random, Shape::Runs] {
        for &n in &[13usize, 100, 1000, 20_000] {
            let data = gen(shape, n, &mut rng);
            let full = unsafe { (c.bound)(n as c_int) } as usize;
            for &lvl in &[1i32, 3, 9, 10, 12] {
                for &cap in &[0usize, 1, 2, 4, 8, n / 4 + 1, n / 2 + 1, n, full] {
                    let mut cb = vec![0u8; cap + 1];
                    let mut rb = vec![0u8; cap + 1];
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
                    assert_eq!(
                        a, b,
                        "HC limited rc shape={:?} n={} lvl={} cap={}",
                        shape, n, lvl, cap
                    );
                    assert_bytes_eq(
                        &format!("HC limited shape={:?} n={} lvl={} cap={}", shape, n, lvl, cap),
                        &cb[..a.max(0) as usize],
                        &rb[..b.max(0) as usize],
                    );
                }
            }
        }
    }
}

// --- CONFIGS: extStateHC (+ fastReset) --------------------------------------
#[test]
fn hc_ext_state() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x3003);
    let ss = unsafe { (c.sizeof_state_hc)() } as usize;
    assert_eq!(ss, unsafe { (r.sizeof_state_hc)() } as usize);
    assert_eq!(
        unsafe { (c.sizeof_stream_state_hc)() },
        unsafe { (r.sizeof_stream_state_hc)() }
    );
    let mut cstate = vec![0u64; ss / 8 + 4];
    let mut rstate = vec![0u64; ss / 8 + 4];
    for &shape in &[Shape::Text, Shape::Random, Shape::Runs, Shape::Mixed] {
        for &n in &[0usize, 1, 13, 1000, 40_000] {
            let data = gen(shape, n, &mut rng);
            let cap = (unsafe { (c.bound)(n as c_int) } as usize).max(1);
            for &lvl in &[1i32, 2, 3, 9, 10, 12, 0, -1, 13] {
                for fast in [false, true] {
                    for v in cstate.iter_mut() {
                        *v = 0;
                    }
                    for v in rstate.iter_mut() {
                        *v = 0;
                    }
                    let mut cb = vec![0u8; cap];
                    let mut rb = vec![0u8; cap];
                    let f = if fast {
                        (c.ext_state_hc_fastreset, r.ext_state_hc_fastreset)
                    } else {
                        (c.ext_state_hc, r.ext_state_hc)
                    };
                    let a = unsafe {
                        (f.0)(
                            cstate.as_mut_ptr() as *mut c_void,
                            data.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap as c_int,
                            lvl,
                        )
                    };
                    let b = unsafe {
                        (f.1)(
                            rstate.as_mut_ptr() as *mut c_void,
                            data.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap as c_int,
                            lvl,
                        )
                    };
                    assert_eq!(
                        a, b,
                        "extStateHC(fast={}) rc shape={:?} n={} lvl={}",
                        fast, shape, n, lvl
                    );
                    assert_bytes_eq(
                        &format!("extStateHC(fast={}) shape={:?} n={} lvl={}", fast, shape, n, lvl),
                        &cb[..a.max(0) as usize],
                        &rb[..b.max(0) as usize],
                    );
                }
            }
        }
    }
}

// --- CONFIGS: LZ4_compress_HC_destSize --------------------------------------
#[test]
fn hc_dest_size() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x3004);
    let ss = unsafe { (c.sizeof_state_hc)() } as usize;
    let mut cstate = vec![0u64; ss / 8 + 4];
    let mut rstate = vec![0u64; ss / 8 + 4];
    for &shape in &[Shape::Text, Shape::Random, Shape::Runs, Shape::Periodic(11)] {
        for &n in &[0usize, 1, 13, 100, 1000, 20_000] {
            let data = gen(shape, n, &mut rng);
            let full = (unsafe { (c.bound)(n as c_int) } as usize).max(1);
            let mut targets: Vec<usize> = vec![0, 1, 2, 4, 8, 13, 32, full];
            if full > 8 {
                targets.push(full / 2);
                targets.push(full / 8 + 1);
                targets.push(rng.range(1, full));
            }
            for &lvl in &[1i32, 3, 9, 10, 12, 0, 13] {
                for &t in &targets {
                    for v in cstate.iter_mut() {
                        *v = 0;
                    }
                    for v in rstate.iter_mut() {
                        *v = 0;
                    }
                    let mut cs = n as c_int;
                    let mut rs = n as c_int;
                    let mut cb = vec![0u8; t + 1];
                    let mut rb = vec![0u8; t + 1];
                    let a = unsafe {
                        (c.hc_dest_size)(
                            cstate.as_mut_ptr() as *mut c_void,
                            data.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            &mut cs,
                            t as c_int,
                            lvl,
                        )
                    };
                    let b = unsafe {
                        (r.hc_dest_size)(
                            rstate.as_mut_ptr() as *mut c_void,
                            data.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            &mut rs,
                            t as c_int,
                            lvl,
                        )
                    };
                    assert_eq!(
                        a, b,
                        "HC_destSize rc shape={:?} n={} lvl={} t={}",
                        shape, n, lvl, t
                    );
                    assert_eq!(
                        cs, rs,
                        "HC_destSize *srcSizePtr shape={:?} n={} lvl={} t={}",
                        shape, n, lvl, t
                    );
                    assert_bytes_eq(
                        &format!("HC_destSize shape={:?} n={} lvl={} t={}", shape, n, lvl, t),
                        &cb[..a.max(0) as usize],
                        &rb[..b.max(0) as usize],
                    );
                }
            }
        }
    }
}

// --- CONFIGS: HC stream init / reset ----------------------------------------
#[test]
fn hc_stream_init_reset() {
    let (c, r) = pair();
    let ss = unsafe { (c.sizeof_state_hc)() } as usize;
    let mut buf = vec![0u8; ss + 64];
    assert_eq!(
        unsafe { (c.init_hc)(std::ptr::null_mut(), ss) }.is_null(),
        unsafe { (r.init_hc)(std::ptr::null_mut(), ss) }.is_null(),
        "LZ4_initStreamHC(NULL)"
    );
    for size in [0usize, 1, ss - 1, ss, ss + 1, ss + 64] {
        let cp = unsafe { (c.init_hc)(buf.as_mut_ptr() as *mut c_void, size) };
        let rp = unsafe { (r.init_hc)(buf.as_mut_ptr() as *mut c_void, size) };
        assert_eq!(cp.is_null(), rp.is_null(), "LZ4_initStreamHC(size={})", size);
    }
    // resetStreamHC / _fast / resetStreamStateHC leave identical bytes
    for &lvl in &[0i32, 1, 2, 9, 10, 12, 13, -5] {
        let mut cbuf = vec![0xABu8; ss];
        let mut rbuf = vec![0xABu8; ss];
        unsafe {
            (c.reset_hc)(cbuf.as_mut_ptr() as *mut c_void, lvl);
            (r.reset_hc)(rbuf.as_mut_ptr() as *mut c_void, lvl);
        }
        assert_bytes_eq(&format!("LZ4_resetStreamHC lvl={}", lvl), &cbuf, &rbuf);
        unsafe {
            (c.reset_hc_fast)(cbuf.as_mut_ptr() as *mut c_void, lvl);
            (r.reset_hc_fast)(rbuf.as_mut_ptr() as *mut c_void, lvl);
        }
        assert_bytes_eq(&format!("LZ4_resetStreamHC_fast lvl={}", lvl), &cbuf, &rbuf);
        unsafe {
            (c.set_level)(cbuf.as_mut_ptr() as *mut c_void, lvl);
            (r.set_level)(rbuf.as_mut_ptr() as *mut c_void, lvl);
        }
        assert_bytes_eq(&format!("LZ4_setCompressionLevel lvl={}", lvl), &cbuf, &rbuf);
        for &fav in &[0i32, 1, 2, -1] {
            unsafe {
                (c.favor)(cbuf.as_mut_ptr() as *mut c_void, fav);
                (r.favor)(rbuf.as_mut_ptr() as *mut c_void, fav);
            }
            assert_bytes_eq(
                &format!("LZ4_favorDecompressionSpeed fav={}", fav),
                &cbuf,
                &rbuf,
            );
        }
    }
    let mut cbuf = vec![0x22u8; ss];
    let mut rbuf = vec![0x22u8; ss];
    let a = unsafe { (c.reset_stream_state_hc)(cbuf.as_mut_ptr() as *mut c_void, std::ptr::null_mut()) };
    let b = unsafe { (r.reset_stream_state_hc)(rbuf.as_mut_ptr() as *mut c_void, std::ptr::null_mut()) };
    assert_eq!(a, b, "LZ4_resetStreamStateHC rc");
    assert_bytes_eq("LZ4_resetStreamStateHC", &cbuf, &rbuf);
    unsafe {
        let cs = (c.create_hc)();
        let rs = (r.create_hc)();
        assert!(!cs.is_null() && !rs.is_null());
        assert_bytes_eq(
            "LZ4_createStreamHC initial state",
            std::slice::from_raw_parts(cs as *const u8, ss),
            std::slice::from_raw_parts(rs as *const u8, ss),
        );
        assert_eq!((c.free_hc)(cs), (r.free_hc)(rs));
        assert_eq!(
            (c.free_hc)(std::ptr::null_mut()),
            (r.free_hc)(std::ptr::null_mut()),
            "LZ4_freeStreamHC(NULL)"
        );
        // legacy createHC / freeHC / slideInputBufferHC
        let base = vec![0u8; 64];
        let cs = (c.create_hc_legacy)(base.as_ptr() as *const c_char);
        let rs = (r.create_hc_legacy)(base.as_ptr() as *const c_char);
        assert!(!cs.is_null() && !rs.is_null());
        assert_bytes_eq(
            "LZ4_createHC initial state",
            std::slice::from_raw_parts(cs as *const u8, ss),
            std::slice::from_raw_parts(rs as *const u8, ss),
        );
        assert_eq!((c.free_hc_legacy)(cs), (r.free_hc_legacy)(rs));
        assert_eq!(
            (c.free_hc_legacy)(std::ptr::null_mut()),
            (r.free_hc_legacy)(std::ptr::null_mut()),
            "LZ4_freeHC(NULL)"
        );
    }
}

// --- CONFIGS: HC streaming, contiguous & with dictionary --------------------
#[test]
fn hc_stream_continue() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x3005);
    for iter in 0..40 {
        let total = rng.range(1, 120_000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let data = gen(shape, total, &mut rng);
        let maxblk = [13usize, 500, 4096, 40_000][rng.below(4)];
        let lvl = [1i32, 2, 3, 9, 10, 12][rng.below(6)];
        unsafe {
            let cs = (c.create_hc)();
            let rs = (r.create_hc)();
            (c.reset_hc)(cs, lvl);
            (r.reset_hc)(rs, lvl);
            let mut off = 0usize;
            let mut blocks: Vec<(usize, Vec<u8>)> = Vec::new();
            while off < total {
                let n = rng.range(1, maxblk + 1).min(total - off);
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let src = data.as_ptr().add(off) as *const c_char;
                let a = (c.hc_continue)(cs, src, cb.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int);
                let b = (r.hc_continue)(rs, src, rb.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int);
                assert_eq!(a, b, "HC_continue rc iter={} n={} lvl={}", iter, n, lvl);
                assert_bytes_eq(
                    &format!("HC_continue iter={} n={} lvl={}", iter, n, lvl),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                blocks.push((n, cb[..a.max(0) as usize].to_vec()));
                off += n;
            }
            (c.free_hc)(cs);
            (r.free_hc)(rs);
            // round-trip through the stream decoder
            type FnCreateD = unsafe extern "C" fn() -> *mut c_void;
            type FnDec = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
            let p = libs();
            let cd: FnCreateD = p.r.sym("LZ4_createStreamDecode");
            let dec: FnDec = p.r.sym("LZ4_decompress_safe_continue");
            let fd: FnFree = p.r.sym("LZ4_freeStreamDecode");
            let d = cd();
            let mut out = vec![0u8; total + 64];
            let mut doff = 0usize;
            for (n, blk) in &blocks {
                let got = dec(
                    d,
                    blk.as_ptr() as *const c_char,
                    out.as_mut_ptr().add(doff) as *mut c_char,
                    blk.len() as c_int,
                    (total + 64 - doff) as c_int,
                );
                assert_eq!(got, *n as c_int, "HC stream round-trip decode");
                doff += got as usize;
            }
            fd(d);
            assert_bytes_eq("HC stream round-trip", &out[..total], &data);
        }
    }
}

// --- CONFIGS: HC continue with mid-stream level changes ---------------------
#[test]
fn hc_stream_level_switching() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x3006);
    let total = 80_000usize;
    let data = gen(Shape::Text, total, &mut rng);
    for &favor in &[0i32, 1] {
        unsafe {
            let cs = (c.create_hc)();
            let rs = (r.create_hc)();
            (c.reset_hc)(cs, 9);
            (r.reset_hc)(rs, 9);
            (c.favor)(cs, favor);
            (r.favor)(rs, favor);
            let mut off = 0usize;
            let mut i = 0;
            while off < total {
                let n = rng.range(1, 6000).min(total - off);
                let lvl = [1i32, 2, 3, 6, 9, 10, 11, 12][i % 8];
                (c.set_level)(cs, lvl);
                (r.set_level)(rs, lvl);
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let src = data.as_ptr().add(off) as *const c_char;
                let a = (c.hc_continue)(cs, src, cb.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int);
                let b = (r.hc_continue)(rs, src, rb.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int);
                assert_eq!(a, b, "HC level switch rc favor={} i={} lvl={}", favor, i, lvl);
                assert_bytes_eq(
                    &format!("HC level switch favor={} i={} lvl={}", favor, i, lvl),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                off += n;
                i += 1;
            }
            (c.free_hc)(cs);
            (r.free_hc)(rs);
        }
    }
}

// --- CONFIGS: HC continue_destSize -----------------------------------------
#[test]
fn hc_stream_continue_dest_size() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x3007);
    for &lvl in &[1i32, 3, 9, 10, 12] {
        for &shape in &[Shape::Text, Shape::Random, Shape::Runs] {
            let total = 40_000usize;
            let data = gen(shape, total, &mut rng);
            unsafe {
                let cs = (c.create_hc)();
                let rs = (r.create_hc)();
                (c.reset_hc)(cs, lvl);
                (r.reset_hc)(rs, lvl);
                let mut off = 0usize;
                let mut guard = 0;
                while off < total && guard < 500 {
                    guard += 1;
                    let avail = (total - off).min(9000);
                    let t = rng.range(1, 4000);
                    let mut csz = avail as c_int;
                    let mut rsz = avail as c_int;
                    let mut cb = vec![0u8; t + 1];
                    let mut rb = vec![0u8; t + 1];
                    let src = data.as_ptr().add(off) as *const c_char;
                    let a = (c.hc_continue_dest_size)(
                        cs,
                        src,
                        cb.as_mut_ptr() as *mut c_char,
                        &mut csz,
                        t as c_int,
                    );
                    let b = (r.hc_continue_dest_size)(
                        rs,
                        src,
                        rb.as_mut_ptr() as *mut c_char,
                        &mut rsz,
                        t as c_int,
                    );
                    assert_eq!(
                        a, b,
                        "HC_continue_destSize rc lvl={} shape={:?} off={} t={}",
                        lvl, shape, off, t
                    );
                    assert_eq!(
                        csz, rsz,
                        "HC_continue_destSize srcSize lvl={} shape={:?} off={} t={}",
                        lvl, shape, off, t
                    );
                    assert_bytes_eq(
                        &format!("HC_continue_destSize lvl={} shape={:?} off={}", lvl, shape, off),
                        &cb[..a.max(0) as usize],
                        &rb[..b.max(0) as usize],
                    );
                    if csz <= 0 {
                        break;
                    }
                    off += csz as usize;
                }
                (c.free_hc)(cs);
                (r.free_hc)(rs);
            }
        }
    }
}

// --- CONFIGS: HC dictionaries (loadDictHC, saveDictHC, attach) --------------
#[test]
fn hc_dictionaries() {
    let (c, r) = pair();
    let ss = unsafe { (c.sizeof_state_hc)() } as usize;
    let mut rng = Rng::new(0x3008);
    for &ds in &[0usize, 1, 13, 1000, 65535, 65536, 65537, 90_000] {
        let dict = gen(Shape::Text, ds, &mut rng);
        let dp = if ds == 0 {
            std::ptr::null()
        } else {
            dict.as_ptr() as *const c_char
        };
        for &lvl in &[1i32, 3, 9, 10, 12] {
            unsafe {
                let cs = (c.create_hc)();
                let rs = (r.create_hc)();
                (c.reset_hc)(cs, lvl);
                (r.reset_hc)(rs, lvl);
                let a = (c.load_dict_hc)(cs, dp, ds as c_int);
                let b = (r.load_dict_hc)(rs, dp, ds as c_int);
                assert_eq!(a, b, "LZ4_loadDictHC rc ds={} lvl={}", ds, lvl);
                assert_bytes_eq(
                    &format!("LZ4_loadDictHC state ds={} lvl={}", ds, lvl),
                    std::slice::from_raw_parts(cs as *const u8, ss),
                    std::slice::from_raw_parts(rs as *const u8, ss),
                );
                // compress with the loaded dictionary
                let n = 20_000usize;
                let mut data = gen(Shape::Text, n, &mut rng);
                let k = n.min(ds);
                if k > 0 {
                    data[..k].copy_from_slice(&dict[..k]);
                }
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let x = (c.hc_continue)(
                    cs,
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                let y = (r.hc_continue)(
                    rs,
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                assert_eq!(x, y, "HC dict compress rc ds={} lvl={}", ds, lvl);
                assert_bytes_eq(
                    &format!("HC dict compress ds={} lvl={}", ds, lvl),
                    &cb[..x.max(0) as usize],
                    &rb[..y.max(0) as usize],
                );
                // saveDictHC with several capacities
                for &sm in &[0i32, 1, 100, 65536, 70_000] {
                    let mut cdst = vec![0u8; 80_000];
                    let mut rdst = vec![0u8; 80_000];
                    let sa = (c.save_dict_hc)(cs, cdst.as_mut_ptr() as *mut c_char, sm);
                    let sb = (r.save_dict_hc)(rs, rdst.as_mut_ptr() as *mut c_char, sm);
                    assert_eq!(sa, sb, "LZ4_saveDictHC rc ds={} lvl={} max={}", ds, lvl, sm);
                    assert_bytes_eq(
                        &format!("LZ4_saveDictHC ds={} lvl={} max={}", ds, lvl, sm),
                        &cdst[..sa.max(0) as usize],
                        &rdst[..sb.max(0) as usize],
                    );
                }
                (c.free_hc)(cs);
                (r.free_hc)(rs);
            }
        }
        // attach_HC_dictionary
        for &lvl in &[1i32, 9, 12] {
            unsafe {
                let cdict = (c.create_hc)();
                let rdict = (r.create_hc)();
                (c.reset_hc)(cdict, lvl);
                (r.reset_hc)(rdict, lvl);
                (c.load_dict_hc)(cdict, dp, ds as c_int);
                (r.load_dict_hc)(rdict, dp, ds as c_int);
                let cs = (c.create_hc)();
                let rs = (r.create_hc)();
                (c.reset_hc_fast)(cs, lvl);
                (r.reset_hc_fast)(rs, lvl);
                (c.attach_hc)(cs, cdict as *const c_void);
                (r.attach_hc)(rs, rdict as *const c_void);
                let n = 15_000usize;
                let mut data = gen(Shape::Text, n, &mut rng);
                let k = n.min(ds);
                if k > 0 {
                    data[..k].copy_from_slice(&dict[..k]);
                }
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let x = (c.hc_continue)(
                    cs,
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                let y = (r.hc_continue)(
                    rs,
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                assert_eq!(x, y, "attach_HC_dictionary rc ds={} lvl={}", ds, lvl);
                assert_bytes_eq(
                    &format!("attach_HC_dictionary ds={} lvl={}", ds, lvl),
                    &cb[..x.max(0) as usize],
                    &rb[..y.max(0) as usize],
                );
                // attaching NULL is allowed
                (c.reset_hc_fast)(cs, lvl);
                (r.reset_hc_fast)(rs, lvl);
                (c.attach_hc)(cs, std::ptr::null());
                (r.attach_hc)(rs, std::ptr::null());
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let x = (c.hc_continue)(
                    cs,
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                let y = (r.hc_continue)(
                    rs,
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                assert_eq!(x, y, "attach NULL HC dict rc ds={} lvl={}", ds, lvl);
                assert_bytes_eq("attach NULL HC dict", &cb[..x.max(0) as usize], &rb[..y.max(0) as usize]);
                (c.free_hc)(cs);
                (c.free_hc)(cdict);
                (r.free_hc)(rs);
                (r.free_hc)(rdict);
            }
        }
    }
}

// --- CONFIGS: deprecated LZ4_compressHC* wrappers ---------------------------
#[test]
fn hc_deprecated_wrappers() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x3009);
    let ss = unsafe { (c.sizeof_state_hc)() } as usize;
    let mut cstate = vec![0u64; ss / 8 + 4];
    let mut rstate = vec![0u64; ss / 8 + 4];
    for &shape in &[Shape::Text, Shape::Random, Shape::Runs] {
        for &n in &[0usize, 1, 13, 1000, 20_000] {
            let data = gen(shape, n, &mut rng);
            let src = data.as_ptr() as *const c_char;
            let cap = (unsafe { (c.bound)(n as c_int) } as usize).max(1);

            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = unsafe { (c.compresshc)(src, cb.as_mut_ptr() as *mut c_char, n as c_int) };
            let b = unsafe { (r.compresshc)(src, rb.as_mut_ptr() as *mut c_char, n as c_int) };
            assert_eq!(a, b, "LZ4_compressHC rc n={}", n);
            assert_bytes_eq("LZ4_compressHC", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);

            for &lim in &[0usize, 1, cap / 2 + 1, cap] {
                let mut cb = vec![0u8; lim + 1];
                let mut rb = vec![0u8; lim + 1];
                let a = unsafe {
                    (c.compresshc_limited)(src, cb.as_mut_ptr() as *mut c_char, n as c_int, lim as c_int)
                };
                let b = unsafe {
                    (r.compresshc_limited)(src, rb.as_mut_ptr() as *mut c_char, n as c_int, lim as c_int)
                };
                assert_eq!(a, b, "LZ4_compressHC_limitedOutput rc n={} lim={}", n, lim);
                assert_bytes_eq(
                    "LZ4_compressHC_limitedOutput",
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
            }

            for &lvl in &[0i32, 1, 3, 9, 12, 13, -1] {
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = unsafe { (c.compresshc2)(src, cb.as_mut_ptr() as *mut c_char, n as c_int, lvl) };
                let b = unsafe { (r.compresshc2)(src, rb.as_mut_ptr() as *mut c_char, n as c_int, lvl) };
                assert_eq!(a, b, "LZ4_compressHC2 rc n={} lvl={}", n, lvl);
                assert_bytes_eq("LZ4_compressHC2", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);

                let lim = cap / 2 + 1;
                let mut cb = vec![0u8; lim + 1];
                let mut rb = vec![0u8; lim + 1];
                let a = unsafe {
                    (c.compresshc2_limited)(src, cb.as_mut_ptr() as *mut c_char, n as c_int, lim as c_int, lvl)
                };
                let b = unsafe {
                    (r.compresshc2_limited)(src, rb.as_mut_ptr() as *mut c_char, n as c_int, lim as c_int, lvl)
                };
                assert_eq!(a, b, "LZ4_compressHC2_limitedOutput rc n={} lvl={}", n, lvl);
                assert_bytes_eq(
                    "LZ4_compressHC2_limitedOutput",
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );

                // *_withStateHC family
                for v in cstate.iter_mut() {
                    *v = 0;
                }
                for v in rstate.iter_mut() {
                    *v = 0;
                }
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = unsafe {
                    (c.compresshc2_with_state)(
                        cstate.as_mut_ptr() as *mut c_void,
                        src,
                        cb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        lvl,
                    )
                };
                let b = unsafe {
                    (r.compresshc2_with_state)(
                        rstate.as_mut_ptr() as *mut c_void,
                        src,
                        rb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        lvl,
                    )
                };
                assert_eq!(a, b, "LZ4_compressHC2_withStateHC rc n={} lvl={}", n, lvl);
                assert_bytes_eq(
                    "LZ4_compressHC2_withStateHC",
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );

                for v in cstate.iter_mut() {
                    *v = 0;
                }
                for v in rstate.iter_mut() {
                    *v = 0;
                }
                let lim = cap / 2 + 1;
                let mut cb = vec![0u8; lim + 1];
                let mut rb = vec![0u8; lim + 1];
                let a = unsafe {
                    (c.compresshc2_limited_with_state)(
                        cstate.as_mut_ptr() as *mut c_void,
                        src,
                        cb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        lim as c_int,
                        lvl,
                    )
                };
                let b = unsafe {
                    (r.compresshc2_limited_with_state)(
                        rstate.as_mut_ptr() as *mut c_void,
                        src,
                        rb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        lim as c_int,
                        lvl,
                    )
                };
                assert_eq!(
                    a, b,
                    "LZ4_compressHC2_limitedOutput_withStateHC rc n={} lvl={}",
                    n, lvl
                );
                assert_bytes_eq(
                    "LZ4_compressHC2_limitedOutput_withStateHC",
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
            }

            for v in cstate.iter_mut() {
                *v = 0;
            }
            for v in rstate.iter_mut() {
                *v = 0;
            }
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = unsafe {
                (c.compresshc_with_state)(
                    cstate.as_mut_ptr() as *mut c_void,
                    src,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                )
            };
            let b = unsafe {
                (r.compresshc_with_state)(
                    rstate.as_mut_ptr() as *mut c_void,
                    src,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                )
            };
            assert_eq!(a, b, "LZ4_compressHC_withStateHC rc n={}", n);
            assert_bytes_eq(
                "LZ4_compressHC_withStateHC",
                &cb[..a.max(0) as usize],
                &rb[..b.max(0) as usize],
            );

            for v in cstate.iter_mut() {
                *v = 0;
            }
            for v in rstate.iter_mut() {
                *v = 0;
            }
            let lim = cap / 2 + 1;
            let mut cb = vec![0u8; lim + 1];
            let mut rb = vec![0u8; lim + 1];
            let a = unsafe {
                (c.compresshc_limited_with_state)(
                    cstate.as_mut_ptr() as *mut c_void,
                    src,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    lim as c_int,
                )
            };
            let b = unsafe {
                (r.compresshc_limited_with_state)(
                    rstate.as_mut_ptr() as *mut c_void,
                    src,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    lim as c_int,
                )
            };
            assert_eq!(a, b, "LZ4_compressHC_limitedOutput_withStateHC rc n={}", n);
            assert_bytes_eq(
                "LZ4_compressHC_limitedOutput_withStateHC",
                &cb[..a.max(0) as usize],
                &rb[..b.max(0) as usize],
            );
        }
    }
}

// --- CONFIGS: deprecated HC streaming wrappers ------------------------------
#[test]
fn hc_deprecated_streaming() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x300A);
    let total = 40_000usize;
    let data = gen(Shape::Text, total, &mut rng);
    unsafe {
        // LZ4_compressHC_continue / _limitedOutput_continue on a proper HC stream
        let cs = (c.create_hc)();
        let rs = (r.create_hc)();
        (c.reset_hc)(cs, 9);
        (r.reset_hc)(rs, 9);
        let mut off = 0usize;
        while off < total {
            let n = rng.range(1, 5000).min(total - off);
            let cap = ((c.bound)(n as c_int) as usize).max(1);
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let src = data.as_ptr().add(off) as *const c_char;
            let a = (c.compresshc_continue)(cs, src, cb.as_mut_ptr() as *mut c_char, n as c_int);
            let b = (r.compresshc_continue)(rs, src, rb.as_mut_ptr() as *mut c_char, n as c_int);
            assert_eq!(a, b, "LZ4_compressHC_continue rc n={}", n);
            assert_bytes_eq("LZ4_compressHC_continue", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);
            off += n;
        }
        (c.free_hc)(cs);
        (r.free_hc)(rs);

        let cs = (c.create_hc)();
        let rs = (r.create_hc)();
        (c.reset_hc)(cs, 9);
        (r.reset_hc)(rs, 9);
        let mut off = 0usize;
        while off < total {
            let n = rng.range(1, 5000).min(total - off);
            let cap = ((c.bound)(n as c_int) as usize).max(1);
            let lim = cap / 2 + 1;
            let mut cb = vec![0u8; lim + 1];
            let mut rb = vec![0u8; lim + 1];
            let src = data.as_ptr().add(off) as *const c_char;
            let a = (c.compresshc_limited_continue)(
                cs,
                src,
                cb.as_mut_ptr() as *mut c_char,
                n as c_int,
                lim as c_int,
            );
            let b = (r.compresshc_limited_continue)(
                rs,
                src,
                rb.as_mut_ptr() as *mut c_char,
                n as c_int,
                lim as c_int,
            );
            assert_eq!(a, b, "LZ4_compressHC_limitedOutput_continue rc n={}", n);
            assert_bytes_eq(
                "LZ4_compressHC_limitedOutput_continue",
                &cb[..a.max(0) as usize],
                &rb[..b.max(0) as usize],
            );
            off += n;
        }
        (c.free_hc)(cs);
        (r.free_hc)(rs);

        // LZ4_compressHC2_continue / _limitedOutput_continue take a void* "LZ4HC_Data"
        for &lvl in &[1i32, 9, 12] {
            // The legacy API requires the *input buffer* to be handed to
            // LZ4_createHC(); it becomes the stream's base pointer.
            let cs = (c.create_hc_legacy)(data.as_ptr() as *const c_char);
            let rs = (r.create_hc_legacy)(data.as_ptr() as *const c_char);
            let mut off = 0usize;
            while off < total {
                let n = rng.range(1, 5000).min(total - off);
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let src = data.as_ptr().add(off) as *const c_char;
                let a = (c.compresshc2_continue)(cs, src, cb.as_mut_ptr() as *mut c_char, n as c_int, lvl);
                let b = (r.compresshc2_continue)(rs, src, rb.as_mut_ptr() as *mut c_char, n as c_int, lvl);
                assert_eq!(a, b, "LZ4_compressHC2_continue rc n={} lvl={}", n, lvl);
                assert_bytes_eq(
                    "LZ4_compressHC2_continue",
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                off += n;
            }
            // slideInputBufferHC must agree
            assert_eq!(
                (c.slide_hc)(cs).is_null(),
                (r.slide_hc)(rs).is_null(),
                "LZ4_slideInputBufferHC null-ness lvl={}",
                lvl
            );
            (c.free_hc_legacy)(cs);
            (r.free_hc_legacy)(rs);

            let cs = (c.create_hc_legacy)(data.as_ptr() as *const c_char);
            let rs = (r.create_hc_legacy)(data.as_ptr() as *const c_char);
            let mut off = 0usize;
            while off < total {
                let n = rng.range(1, 5000).min(total - off);
                let cap = ((c.bound)(n as c_int) as usize).max(1);
                let lim = cap / 2 + 1;
                let mut cb = vec![0u8; lim + 1];
                let mut rb = vec![0u8; lim + 1];
                let src = data.as_ptr().add(off) as *const c_char;
                let a = (c.compresshc2_limited_continue)(
                    cs,
                    src,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    lim as c_int,
                    lvl,
                );
                let b = (r.compresshc2_limited_continue)(
                    rs,
                    src,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    lim as c_int,
                    lvl,
                );
                assert_eq!(
                    a, b,
                    "LZ4_compressHC2_limitedOutput_continue rc n={} lvl={}",
                    n, lvl
                );
                assert_bytes_eq(
                    "LZ4_compressHC2_limitedOutput_continue",
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                off += n;
            }
            (c.free_hc_legacy)(cs);
            (r.free_hc_legacy)(rs);
        }
    }
}
