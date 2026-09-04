//! Phase C — error-path differential tests for lz4.c / lz4hc.c.
//! Each test names the `ERRORS.md` row(s) it covers.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

type F4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type F5 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type F3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FBound = unsafe extern "C" fn(c_int) -> c_int;
type FDestSize = unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
type FExt = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FInit = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FCreate = unsafe extern "C" fn() -> *mut c_void;
type FFree = unsafe extern "C" fn(*mut c_void) -> c_int;
type FLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FContinue = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FSetDecode = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FDecContinue = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;
type FHC = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FHCDest =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;
type FInitHC = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FResetStateHC = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;

struct Api {
    bound: FBound,
    compress_default: F4,
    compress_fast: F5,
    compress_fast_ext: FExt,
    dest_size: FDestSize,
    decompress_safe: F4,
    decompress_partial: F5,
    decompress_fast: F3,
    safe_using_dict: FUsingDict,
    init_stream: FInit,
    create_stream: FCreate,
    free_stream: FFree,
    load_dict: FLoadDict,
    load_dict_slow: FLoadDict,
    save_dict: FSaveDict,
    compress_continue: FContinue,
    create_decode: FCreate,
    free_decode: FFree,
    set_decode: FSetDecode,
    dec_continue: FDecContinue,
    sizeof_stream_state: FnIntVoid,
    sizeof_state: FnIntVoid,
    // HC
    compress_hc: FHC,
    hc_dest_size: FHCDest,
    init_hc: FInitHC,
    sizeof_state_hc: FnIntVoid,
    reset_state_hc: FResetStateHC,
    ext_hc: FExt,
    ext_hc_fastreset: FExt,
}

fn bind(l: &Lib) -> Api {
    Api {
        bound: l.sym("LZ4_compressBound"),
        compress_default: l.sym("LZ4_compress_default"),
        compress_fast: l.sym("LZ4_compress_fast"),
        compress_fast_ext: l.sym("LZ4_compress_fast_extState"),
        dest_size: l.sym("LZ4_compress_destSize"),
        decompress_safe: l.sym("LZ4_decompress_safe"),
        decompress_partial: l.sym("LZ4_decompress_safe_partial"),
        decompress_fast: l.sym("LZ4_decompress_fast"),
        safe_using_dict: l.sym("LZ4_decompress_safe_usingDict"),
        init_stream: l.sym("LZ4_initStream"),
        create_stream: l.sym("LZ4_createStream"),
        free_stream: l.sym("LZ4_freeStream"),
        load_dict: l.sym("LZ4_loadDict"),
        load_dict_slow: l.sym("LZ4_loadDictSlow"),
        save_dict: l.sym("LZ4_saveDict"),
        compress_continue: l.sym("LZ4_compress_fast_continue"),
        create_decode: l.sym("LZ4_createStreamDecode"),
        free_decode: l.sym("LZ4_freeStreamDecode"),
        set_decode: l.sym("LZ4_setStreamDecode"),
        dec_continue: l.sym("LZ4_decompress_safe_continue"),
        sizeof_stream_state: l.sym("LZ4_sizeofStreamState"),
        sizeof_state: l.sym("LZ4_sizeofState"),
        compress_hc: l.sym("LZ4_compress_HC"),
        hc_dest_size: l.sym("LZ4_compress_HC_destSize"),
        init_hc: l.sym("LZ4_initStreamHC"),
        sizeof_state_hc: l.sym("LZ4_sizeofStateHC"),
        reset_state_hc: l.sym("LZ4_resetStreamStateHC"),
        ext_hc: l.sym("LZ4_compress_HC_extStateHC"),
        ext_hc_fastreset: l.sym("LZ4_compress_HC_extStateHC_fastReset"),
    }
}

fn pair() -> (Api, Api) {
    let p = libs();
    (bind(&p.c), bind(&p.r))
}

/// ERRORS rows 106 (compressBound out of range), 171-173 (ring buffer size).
#[test]
fn err_bound_out_of_range() {
    let (c, r) = pair();
    for n in [
        i32::MIN,
        -1_000_000,
        -1,
        0,
        1,
        LZ4_MAX_INPUT_SIZE as c_int - 1,
        LZ4_MAX_INPUT_SIZE as c_int,
        LZ4_MAX_INPUT_SIZE as c_int + 1,
        i32::MAX,
    ] {
        assert_eq!(
            unsafe { (c.bound)(n) },
            unsafe { (r.bound)(n) },
            "LZ4_compressBound({})",
            n
        );
    }
}

/// ERRORS rows 107 (srcSize negative / > LZ4_MAX_INPUT_SIZE), 108, 109
/// (srcSize == 0 with and without output room).
#[test]
fn err_compress_src_size_and_empty() {
    let (c, r) = pair();
    let src = vec![0x41u8; 4096];
    let mut cb = vec![0u8; 4096];
    let mut rb = vec![0u8; 4096];
    for &n in &[
        -1i32,
        -100,
        i32::MIN,
        LZ4_MAX_INPUT_SIZE as c_int + 1,
        i32::MAX,
    ] {
        for &cap in &[0i32, 1, 4096] {
            let a = unsafe {
                (c.compress_default)(src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, n, cap)
            };
            let b = unsafe {
                (r.compress_default)(src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, n, cap)
            };
            assert_eq!(a, b, "compress_default(srcSize={}, cap={})", n, cap);
            assert_eq!(a, 0, "expected rejection for srcSize={}", n);
            let a = unsafe {
                (c.compress_fast)(src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, n, cap, 1)
            };
            let b = unsafe {
                (r.compress_fast)(src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, n, cap, 1)
            };
            assert_eq!(a, b, "compress_fast(srcSize={}, cap={})", n, cap);
        }
    }
    // srcSize == 0: dstCapacity 0 -> 0 ; >= 1 -> 1 (a single zero token byte)
    for &cap in &[-1i32, 0, 1, 2, 16] {
        let a = unsafe {
            (c.compress_default)(src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 0, cap)
        };
        let b = unsafe {
            (r.compress_default)(src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 0, cap)
        };
        assert_eq!(a, b, "compress_default(srcSize=0, cap={})", cap);
        if a > 0 {
            assert_bytes_eq("empty block", &cb[..a as usize], &rb[..b as usize]);
        }
    }
}

/// ERRORS rows 110-114 + 133: `limitedOutput` rejections at every stage of the
/// encoder (literal run, match length, last literals).
#[test]
fn err_compress_dst_too_small() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x6001);
    for &shape in ALL_SHAPES {
        for &n in &[13usize, 20, 100, 1000, 5000, 70_000] {
            let data = gen(shape, n, &mut rng);
            // sweep *every* capacity from 0 to a bit beyond the compressed size
            let full = unsafe { (c.bound)(n as c_int) } as usize;
            let mut probe = vec![0u8; full];
            let clen = unsafe {
                (c.compress_default)(
                    data.as_ptr() as *const c_char,
                    probe.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    full as c_int,
                )
            } as usize;
            let hi = (clen + 8).min(full);
            for cap in 0..=hi {
                let mut cb = vec![0xCCu8; cap + 1];
                let mut rb = vec![0xCCu8; cap + 1];
                let a = unsafe {
                    (c.compress_default)(
                        data.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                    )
                };
                let b = unsafe {
                    (r.compress_default)(
                        data.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                    )
                };
                assert_eq!(
                    a, b,
                    "compress_default rejection shape={:?} n={} cap={}",
                    shape, n, cap
                );
                assert_bytes_eq(
                    &format!("compress_default cap={} shape={:?} n={}", cap, shape, n),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                // negative dstCapacity
                if cap == 0 {
                    for &neg in &[-1i32, i32::MIN] {
                        let a = unsafe {
                            (c.compress_default)(
                                data.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                n as c_int,
                                neg,
                            )
                        };
                        let b = unsafe {
                            (r.compress_default)(
                                data.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                n as c_int,
                                neg,
                            )
                        };
                        assert_eq!(a, b, "compress_default negative cap={}", neg);
                    }
                }
            }
        }
    }
}

/// ERRORS rows 123-125: LZ4_initStream / LZ4_initStreamHC reject NULL,
/// undersized and misaligned buffers.
#[test]
fn err_init_stream_rejections() {
    let (c, r) = pair();
    let ss = unsafe { (c.sizeof_stream_state)() } as usize;
    let sshc = unsafe { (c.sizeof_state_hc)() } as usize;
    // over-allocate so we can offset the pointer to create misalignment
    let mut buf = vec![0u8; sshc + 128];
    let base = buf.as_mut_ptr() as usize;
    // find an 8-byte-aligned starting point
    let aligned = (base + 63) & !63usize;
    for off in 0..16usize {
        let p = (aligned + off) as *mut c_void;
        for &size in &[0usize, 1, ss - 1, ss, ss + 1] {
            let a = unsafe { (c.init_stream)(p, size) };
            let b = unsafe { (r.init_stream)(p, size) };
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "LZ4_initStream(off={}, size={}) null-ness",
                off,
                size
            );
            assert_eq!(a, b, "LZ4_initStream(off={}, size={}) pointer", off, size);
        }
        for &size in &[0usize, 1, sshc - 1, sshc, sshc + 1] {
            let a = unsafe { (c.init_hc)(p, size) };
            let b = unsafe { (r.init_hc)(p, size) };
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "LZ4_initStreamHC(off={}, size={}) null-ness",
                off,
                size
            );
            assert_eq!(a, b, "LZ4_initStreamHC(off={}, size={}) pointer", off, size);
        }
    }
    assert!(unsafe { (c.init_stream)(std::ptr::null_mut(), ss) }.is_null());
    assert!(unsafe { (r.init_stream)(std::ptr::null_mut(), ss) }.is_null());
    assert!(unsafe { (c.init_hc)(std::ptr::null_mut(), sshc) }.is_null());
    assert!(unsafe { (r.init_hc)(std::ptr::null_mut(), sshc) }.is_null());
}

/// ERRORS row 220: LZ4_resetStreamStateHC returns 1 (error) when the state
/// buffer is misaligned; 0 otherwise. Inverted return convention.
#[test]
fn err_reset_stream_state_hc() {
    let (c, r) = pair();
    let sshc = unsafe { (c.sizeof_state_hc)() } as usize;
    let mut buf = vec![0u8; sshc + 128];
    let base = buf.as_mut_ptr() as usize;
    let aligned = (base + 63) & !63usize;
    for off in 0..16usize {
        let p = (aligned + off) as *mut c_void;
        let a = unsafe { (c.reset_state_hc)(p, std::ptr::null_mut()) };
        let b = unsafe { (r.reset_state_hc)(p, std::ptr::null_mut()) };
        assert_eq!(a, b, "LZ4_resetStreamStateHC(off={})", off);
    }
}

/// ERRORS rows 196, 197, 201: extStateHC / HC_destSize reject a misaligned state
/// with 0.
#[test]
fn err_hc_ext_state_misaligned() {
    let (c, r) = pair();
    let sshc = unsafe { (c.sizeof_state_hc)() } as usize;
    let mut buf = vec![0u8; sshc + 128];
    let base = buf.as_mut_ptr() as usize;
    let aligned = (base + 63) & !63usize;
    let data = vec![0x5Au8; 4096];
    let mut cb = vec![0u8; 8192];
    let mut rb = vec![0u8; 8192];
    for off in 0..16usize {
        let p = (aligned + off) as *mut c_void;
        for &lvl in &[1i32, 9, 12] {
            let a = unsafe {
                (c.ext_hc)(
                    p,
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    data.len() as c_int,
                    cb.len() as c_int,
                    lvl,
                )
            };
            let b = unsafe {
                (r.ext_hc)(
                    p,
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    data.len() as c_int,
                    rb.len() as c_int,
                    lvl,
                )
            };
            assert_eq!(a, b, "extStateHC(off={}, lvl={})", off, lvl);
            let a = unsafe {
                (c.ext_hc_fastreset)(
                    p,
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    data.len() as c_int,
                    cb.len() as c_int,
                    lvl,
                )
            };
            let b = unsafe {
                (r.ext_hc_fastreset)(
                    p,
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    data.len() as c_int,
                    rb.len() as c_int,
                    lvl,
                )
            };
            assert_eq!(a, b, "extStateHC_fastReset(off={}, lvl={})", off, lvl);
            let mut cs = data.len() as c_int;
            let mut rs = data.len() as c_int;
            let a = unsafe {
                (c.hc_dest_size)(
                    p,
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    &mut cs,
                    100,
                    lvl,
                )
            };
            let b = unsafe {
                (r.hc_dest_size)(
                    p,
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    &mut rs,
                    100,
                    lvl,
                )
            };
            assert_eq!(a, b, "HC_destSize(off={}, lvl={})", off, lvl);
            assert_eq!(cs, rs, "HC_destSize srcSize(off={}, lvl={})", off, lvl);
        }
    }
}

/// ERRORS rows 118 + 116/117: `LZ4_compress_fast_extState` with a misaligned
/// state, and acceleration clamping.
#[test]
fn err_compress_fast_ext_state_misaligned() {
    let (c, r) = pair();
    let ss = unsafe { (c.sizeof_state)() } as usize;
    let mut buf = vec![0u8; ss + 128];
    let base = buf.as_mut_ptr() as usize;
    let aligned = (base + 63) & !63usize;
    let data = vec![0x5Au8; 4096];
    let mut cb = vec![0u8; 8192];
    let mut rb = vec![0u8; 8192];
    // NOTE: lz4.c:1384 only asserts the LZ4_initStream() result, so a MISALIGNED
    // state is UB in C (NULL deref). Only test aligned offsets here, which is
    // where the C has defined behaviour.
    for off in [0usize, 8] {
        let p = (aligned + off) as *mut c_void;
        for &acc in &[i32::MIN, -1, 0, 1, 2, 65536, 65537, 65538, i32::MAX] {
            unsafe { std::ptr::write_bytes(p as *mut u8, 0, ss) };
            let a = unsafe {
                (c.compress_fast_ext)(
                    p,
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    data.len() as c_int,
                    cb.len() as c_int,
                    acc,
                )
            };
            unsafe { std::ptr::write_bytes(p as *mut u8, 0, ss) };
            let b = unsafe {
                (r.compress_fast_ext)(
                    p,
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    data.len() as c_int,
                    rb.len() as c_int,
                    acc,
                )
            };
            assert_eq!(a, b, "compress_fast_extState(off={}, acc={})", off, acc);
            assert_bytes_eq(
                &format!("compress_fast_extState acc={}", acc),
                &cb[..a.max(0) as usize],
                &rb[..b.max(0) as usize],
            );
        }
    }
}

/// ERRORS rows 127, 128, 134, 135: loadDict / saveDict edge behaviour.
#[test]
fn err_load_save_dict_edges() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x6002);
    let big = gen(Shape::Text, 200_000, &mut rng);
    for &ds in &[0i32, 1, 2, 3, 4, 11, 12, 65535, 65536, 65537, 200_000] {
        unsafe {
            for slow in [false, true] {
                let cs = (c.create_stream)();
                let rs = (r.create_stream)();
                let dp = big.as_ptr() as *const c_char;
                let (a, b) = if slow {
                    ((c.load_dict_slow)(cs, dp, ds), (r.load_dict_slow)(rs, dp, ds))
                } else {
                    ((c.load_dict)(cs, dp, ds), (r.load_dict)(rs, dp, ds))
                };
                assert_eq!(a, b, "loadDict(slow={}, ds={})", slow, ds);
                // saveDict with every clamping case
                for &sm in &[-1i32, 0, 1, 11, 12, 100, 65536, 65537, 200_000] {
                    let mut cd = vec![0u8; 210_000];
                    let mut rd = vec![0u8; 210_000];
                    let x = (c.save_dict)(cs, cd.as_mut_ptr() as *mut c_char, sm);
                    let y = (r.save_dict)(rs, rd.as_mut_ptr() as *mut c_char, sm);
                    assert_eq!(x, y, "saveDict(ds={}, max={})", ds, sm);
                    assert_bytes_eq(
                        &format!("saveDict content ds={} max={}", ds, sm),
                        &cd[..x.max(0) as usize],
                        &rd[..y.max(0) as usize],
                    );
                }
                (c.free_stream)(cs);
                (r.free_stream)(rs);
            }
            // dictionary == NULL with dictSize == 0 is legal
            let cs = (c.create_stream)();
            let rs = (r.create_stream)();
            assert_eq!(
                (c.load_dict)(cs, std::ptr::null(), 0),
                (r.load_dict)(rs, std::ptr::null(), 0),
                "loadDict(NULL, 0)"
            );
            (c.free_stream)(cs);
            (r.free_stream)(rs);
        }
    }
}

/// ERRORS rows 143-165: the whole `LZ4_decompress_generic` rejection surface,
/// exercised by feeding corrupted / truncated / hostile blocks and comparing the
/// exact negative sentinel (`-(ip-src)-1`).
#[test]
fn err_decompress_safe_malformed() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x6003);
    let mut cases: Vec<Vec<u8>> = Vec::new();
    // 1) purely random blocks of many lengths
    for len in [0usize, 1, 2, 3, 4, 5, 8, 16, 32, 64, 128, 512, 2000] {
        for _ in 0..40 {
            cases.push(gen(Shape::Random, len, &mut rng));
        }
    }
    // 2) valid blocks with single-byte corruptions
    for &n in &[13usize, 100, 1000, 20_000] {
        for &shape in &[Shape::Text, Shape::Runs, Shape::Random] {
            let data = gen(shape, n, &mut rng);
            let full = unsafe { (c.bound)(n as c_int) } as usize;
            let mut comp = vec![0u8; full];
            let clen = unsafe {
                (c.compress_default)(
                    data.as_ptr() as *const c_char,
                    comp.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    full as c_int,
                )
            } as usize;
            comp.truncate(clen);
            cases.push(comp.clone());
            for _ in 0..60 {
                let mut bad = comp.clone();
                let pos = rng.below(bad.len());
                bad[pos] ^= 1u8 << rng.below(8);
                cases.push(bad);
            }
            // truncations
            for cut in [0usize, 1, clen / 3, clen / 2, clen - 1] {
                cases.push(comp[..cut.min(clen)].to_vec());
            }
        }
    }
    // 3) hand-built hostile tokens: huge literal / match lengths
    for tok in [0xF0u8, 0x0F, 0xFF, 0xF1, 0x1F] {
        for tail in 0..6usize {
            let mut v = vec![tok];
            v.extend_from_slice(&vec![0xFFu8; tail]);
            cases.push(v);
        }
    }
    // offset 0 (invalid) and huge offsets
    for lit in [0u8, 1, 2] {
        for off in [0u16, 1, 0xFFFF, 0x8000] {
            let mut v = vec![(lit << 4) | 0x0F];
            v.extend_from_slice(&vec![0x41u8; lit as usize]);
            v.extend_from_slice(&off.to_le_bytes());
            v.extend_from_slice(&[0xFF, 0xFF, 0x00]);
            cases.push(v);
        }
    }

    for (i, blk) in cases.iter().enumerate() {
        for &dcap in &[0i32, 1, 5, 16, 100, 4096, 65536] {
            let sz = dcap.max(0) as usize + 64;
            let mut co = vec![0x99u8; sz];
            let mut ro = vec![0x99u8; sz];
            let a = unsafe {
                (c.decompress_safe)(
                    blk.as_ptr() as *const c_char,
                    co.as_mut_ptr() as *mut c_char,
                    blk.len() as c_int,
                    dcap,
                )
            };
            let b = unsafe {
                (r.decompress_safe)(
                    blk.as_ptr() as *const c_char,
                    ro.as_mut_ptr() as *mut c_char,
                    blk.len() as c_int,
                    dcap,
                )
            };
            assert_eq!(
                a, b,
                "decompress_safe case={} len={} dcap={} blk={}",
                i,
                blk.len(),
                dcap,
                hexdump(blk)
            );
            if a > 0 {
                assert_bytes_eq(
                    &format!("decompress_safe out case={} dcap={}", i, dcap),
                    &co[..a as usize],
                    &ro[..b as usize],
                );
            }
            // partial variant, incl. negative targetOutputSize / dstCapacity
            for &t in &[-1i32, 0, 1, dcap / 2, dcap, dcap + 1] {
                let a = unsafe {
                    (c.decompress_partial)(
                        blk.as_ptr() as *const c_char,
                        co.as_mut_ptr() as *mut c_char,
                        blk.len() as c_int,
                        t,
                        dcap,
                    )
                };
                let b = unsafe {
                    (r.decompress_partial)(
                        blk.as_ptr() as *const c_char,
                        ro.as_mut_ptr() as *mut c_char,
                        blk.len() as c_int,
                        t,
                        dcap,
                    )
                };
                assert_eq!(
                    a, b,
                    "decompress_safe_partial case={} len={} target={} dcap={}",
                    i,
                    blk.len(),
                    t,
                    dcap
                );
            }
        }
        // ERRORS row 165: a negative `compressedSize` is *not* validated by the
        // C code -- `iend = ip + srcSize` ends up before `ip`, and the parser
        // then reads out of bounds. Verified experimentally: the C library
        // itself segfaults for some block contents, so this is genuine UB and
        // not a comparable rejection. Not tested.
    }
}

/// ERRORS rows 176, 177: `LZ4_decompress_safe_usingDict` with dictSize == 0
/// (dictionary ignored) and with a dictionary present.
#[test]
fn err_decompress_using_dict_edges() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x6004);
    let dict = gen(Shape::Text, 70_000, &mut rng);
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for len in [0usize, 1, 4, 16, 64, 300] {
        for _ in 0..25 {
            cases.push(gen(Shape::Random, len, &mut rng));
        }
    }
    for blk in &cases {
        for &ds in &[0i32, 1, 13, 65536, 70_000] {
            for &dcap in &[0i32, 1, 100, 4096] {
                let mut co = vec![0u8; dcap.max(0) as usize + 64];
                let mut ro = vec![0u8; dcap.max(0) as usize + 64];
                let a = unsafe {
                    (c.safe_using_dict)(
                        blk.as_ptr() as *const c_char,
                        co.as_mut_ptr() as *mut c_char,
                        blk.len() as c_int,
                        dcap,
                        dict.as_ptr() as *const c_char,
                        ds,
                    )
                };
                let b = unsafe {
                    (r.safe_using_dict)(
                        blk.as_ptr() as *const c_char,
                        ro.as_mut_ptr() as *mut c_char,
                        blk.len() as c_int,
                        dcap,
                        dict.as_ptr() as *const c_char,
                        ds,
                    )
                };
                assert_eq!(
                    a, b,
                    "safe_usingDict blk={} ds={} dcap={}",
                    hexdump(blk),
                    ds,
                    dcap
                );
                if a > 0 {
                    assert_bytes_eq("safe_usingDict out", &co[..a as usize], &ro[..b as usize]);
                }
            }
        }
    }
}

/// ERRORS rows 174, 175: `*_continue` decoders treat a <= 0 result as failure and
/// leave the stream un-advanced.
#[test]
fn err_decompress_continue_failures() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x6005);
    let data = gen(Shape::Text, 40_000, &mut rng);
    let full = unsafe { (c.bound)(20_000) } as usize;
    // build two valid blocks with the C streaming compressor
    let p = libs();
    let create: FCreate = p.c.sym("LZ4_createStream");
    let free: FFree = p.c.sym("LZ4_freeStream");
    let cont: FContinue = p.c.sym("LZ4_compress_fast_continue");
    let mut blocks: Vec<Vec<u8>> = Vec::new();
    unsafe {
        let s = create();
        for i in 0..2 {
            let mut b = vec![0u8; full];
            let l = cont(
                s,
                data.as_ptr().add(i * 20_000) as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                20_000,
                full as c_int,
                1,
            );
            b.truncate(l as usize);
            blocks.push(b);
        }
        free(s);
    }
    // feed block 0 correctly, then a corrupted block 1, then block 1 again:
    // both libraries must agree on every return value and on the recovery.
    for _ in 0..40 {
        let mut bad = blocks[1].clone();
        let pos = rng.below(bad.len());
        bad[pos] ^= 0xFF;
        unsafe {
            let cd = (c.create_decode)();
            let rd = (r.create_decode)();
            let mut co = vec![0u8; 80_000];
            let mut ro = vec![0u8; 80_000];
            let x = (c.dec_continue)(
                cd,
                blocks[0].as_ptr() as *const c_char,
                co.as_mut_ptr() as *mut c_char,
                blocks[0].len() as c_int,
                40_000,
            );
            let y = (r.dec_continue)(
                rd,
                blocks[0].as_ptr() as *const c_char,
                ro.as_mut_ptr() as *mut c_char,
                blocks[0].len() as c_int,
                40_000,
            );
            assert_eq!(x, y, "continue block0");
            let x2 = (c.dec_continue)(
                cd,
                bad.as_ptr() as *const c_char,
                co.as_mut_ptr().add(x.max(0) as usize) as *mut c_char,
                bad.len() as c_int,
                40_000,
            );
            let y2 = (r.dec_continue)(
                rd,
                bad.as_ptr() as *const c_char,
                ro.as_mut_ptr().add(y.max(0) as usize) as *mut c_char,
                bad.len() as c_int,
                40_000,
            );
            assert_eq!(x2, y2, "continue corrupted block1 pos={}", pos);
            // retry with the good block: the streams must still agree
            let x3 = (c.dec_continue)(
                cd,
                blocks[1].as_ptr() as *const c_char,
                co.as_mut_ptr().add(x.max(0) as usize) as *mut c_char,
                blocks[1].len() as c_int,
                40_000,
            );
            let y3 = (r.dec_continue)(
                rd,
                blocks[1].as_ptr() as *const c_char,
                ro.as_mut_ptr().add(y.max(0) as usize) as *mut c_char,
                blocks[1].len() as c_int,
                40_000,
            );
            assert_eq!(x3, y3, "continue retry block1 pos={}", pos);
            if x3 > 0 {
                assert_bytes_eq(
                    "continue retry output",
                    &co[..(x.max(0) + x3) as usize],
                    &ro[..(y.max(0) + y3) as usize],
                );
            }
            (c.free_decode)(cd);
            (r.free_decode)(rd);
        }
    }
    // zero-length block through decompress_safe_continue (row 174)
    unsafe {
        let cd = (c.create_decode)();
        let rd = (r.create_decode)();
        let mut co = vec![0u8; 1024];
        let mut ro = vec![0u8; 1024];
        let empty: [u8; 1] = [0];
        for &sl in &[0i32, 1] {
            let x = (c.dec_continue)(
                cd,
                empty.as_ptr() as *const c_char,
                co.as_mut_ptr() as *mut c_char,
                sl,
                1024,
            );
            let y = (r.dec_continue)(
                rd,
                empty.as_ptr() as *const c_char,
                ro.as_mut_ptr() as *mut c_char,
                sl,
                1024,
            );
            assert_eq!(x, y, "dec_continue srcSize={}", sl);
        }
        (c.free_decode)(cd);
        (r.free_decode)(rd);
    }
}

/// ERRORS rows 137-141: `LZ4_decompress_fast` family on malformed input, using
/// only *self-consistent* originalSize values so the C stays in-bounds.
#[test]
fn err_decompress_fast_negative() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x6006);
    for &n in &[13usize, 100, 1000] {
        let data = gen(Shape::Text, n, &mut rng);
        let full = unsafe { (c.bound)(n as c_int) } as usize;
        let mut comp = vec![0u8; full];
        let clen = unsafe {
            (c.compress_default)(
                data.as_ptr() as *const c_char,
                comp.as_mut_ptr() as *mut c_char,
                n as c_int,
                full as c_int,
            )
        } as usize;
        comp.truncate(clen);
        // originalSize smaller than the true size makes the parser stop early
        // and return an error; larger values are UB in C, so they are excluded.
        for &orig in &[0i32, 1, (n / 2) as c_int, (n - 1) as c_int] {
            let mut co = vec![0u8; n + 64];
            let mut ro = vec![0u8; n + 64];
            let a = unsafe {
                (c.decompress_fast)(comp.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, orig)
            };
            let b = unsafe {
                (r.decompress_fast)(comp.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, orig)
            };
            assert_eq!(a, b, "decompress_fast n={} originalSize={}", n, orig);
            assert_bytes_eq(
                &format!("decompress_fast n={} orig={}", n, orig),
                &co[..orig.max(0) as usize],
                &ro[..orig.max(0) as usize],
            );
        }
        // negative originalSize
        for &neg in &[-1i32, -1000, i32::MIN] {
            let mut co = vec![0u8; n + 64];
            let mut ro = vec![0u8; n + 64];
            let a = unsafe {
                (c.decompress_fast)(comp.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, neg)
            };
            let b = unsafe {
                (r.decompress_fast)(comp.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, neg)
            };
            assert_eq!(a, b, "decompress_fast negative originalSize={}", neg);
        }
    }
}

/// ERRORS rows 119-121: LZ4_compress_destSize with degenerate targetDstSize.
#[test]
fn err_dest_size_edges() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x6007);
    for &n in &[0usize, 1, 13, 1000, 70_000] {
        let data = gen(Shape::Text, n, &mut rng);
        for &t in &[-1i32, 0, 1, 2, 3, 4, 5, 8, 12, 13] {
            let mut cs = n as c_int;
            let mut rs = n as c_int;
            let mut cb = vec![0u8; 64];
            let mut rb = vec![0u8; 64];
            let a = unsafe {
                (c.dest_size)(
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    &mut cs,
                    t,
                )
            };
            let b = unsafe {
                (r.dest_size)(
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    &mut rs,
                    t,
                )
            };
            assert_eq!(a, b, "destSize n={} target={}", n, t);
            assert_eq!(cs, rs, "destSize *srcSizePtr n={} target={}", n, t);
            assert_bytes_eq(
                &format!("destSize n={} t={}", n, t),
                &cb[..a.max(0) as usize],
                &rb[..b.max(0) as usize],
            );
        }
        // negative *srcSizePtr
        for &neg in &[-1i32, i32::MIN] {
            let mut cs = neg;
            let mut rs = neg;
            let mut cb = vec![0u8; 64];
            let mut rb = vec![0u8; 64];
            let a = unsafe {
                (c.dest_size)(
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    &mut cs,
                    64,
                )
            };
            let b = unsafe {
                (r.dest_size)(
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    &mut rs,
                    64,
                )
            };
            assert_eq!(a, b, "destSize negative srcSize={}", neg);
            assert_eq!(cs, rs, "destSize negative srcSize={} out", neg);
        }
    }
}

/// ERRORS rows 199, 215: HC compressors return 0 when the output cannot fit.
#[test]
fn err_hc_dst_too_small() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x6008);
    for &shape in &[Shape::Text, Shape::Random, Shape::Runs] {
        for &n in &[13usize, 100, 1000, 20_000] {
            let data = gen(shape, n, &mut rng);
            let full = unsafe { (c.bound)(n as c_int) } as usize;
            for &lvl in &[1i32, 2, 3, 9, 10, 12] {
                let mut probe = vec![0u8; full];
                let clen = unsafe {
                    (c.compress_hc)(
                        data.as_ptr() as *const c_char,
                        probe.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        full as c_int,
                        lvl,
                    )
                } as usize;
                let hi = (clen + 4).min(full);
                let lo = hi.saturating_sub(40);
                for cap in lo..=hi {
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
                        "HC rejection shape={:?} n={} lvl={} cap={}",
                        shape, n, lvl, cap
                    );
                    assert_bytes_eq(
                        &format!("HC cap={} lvl={}", cap, lvl),
                        &cb[..a.max(0) as usize],
                        &rb[..b.max(0) as usize],
                    );
                }
                // negative / zero capacity and negative srcSize
                for &cap in &[i32::MIN, -1, 0] {
                    let mut cb = vec![0u8; 8];
                    let mut rb = vec![0u8; 8];
                    let a = unsafe {
                        (c.compress_hc)(
                            data.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap,
                            lvl,
                        )
                    };
                    let b = unsafe {
                        (r.compress_hc)(
                            data.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap,
                            lvl,
                        )
                    };
                    assert_eq!(a, b, "HC cap={} lvl={}", cap, lvl);
                }
                for &sn in &[i32::MIN, -1, LZ4_MAX_INPUT_SIZE as c_int + 1, i32::MAX] {
                    let mut cb = vec![0u8; 64];
                    let mut rb = vec![0u8; 64];
                    let a = unsafe {
                        (c.compress_hc)(
                            data.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            sn,
                            64,
                            lvl,
                        )
                    };
                    let b = unsafe {
                        (r.compress_hc)(
                            data.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            sn,
                            64,
                            lvl,
                        )
                    };
                    assert_eq!(a, b, "HC srcSize={} lvl={}", sn, lvl);
                    assert_eq!(a, 0, "HC srcSize={} should be rejected", sn);
                }
            }
        }
    }
}

/// ERRORS rows 126, 169, 203, 222: free-on-NULL for every allocator wrapper.
#[test]
fn err_free_on_null() {
    let (c, r) = pair();
    let p = libs();
    for name in [
        "LZ4_freeStream",
        "LZ4_freeStreamDecode",
        "LZ4_freeStreamHC",
        "LZ4_freeHC",
    ] {
        let f: FFree = p.c.sym(name);
        let g: FFree = p.r.sym(name);
        assert_eq!(
            unsafe { f(std::ptr::null_mut()) },
            unsafe { g(std::ptr::null_mut()) },
            "{}(NULL)",
            name
        );
    }
    // setStreamDecode always returns 1 (row 170); dictSize 0 accepts NULL
    unsafe {
        let cd = (c.create_decode)();
        let rd = (r.create_decode)();
        assert_eq!(
            (c.set_decode)(cd, std::ptr::null(), 0),
            (r.set_decode)(rd, std::ptr::null(), 0),
            "setStreamDecode(NULL,0)"
        );
        let d = vec![7u8; 100];
        for &ds in &[0i32, 1, 100] {
            assert_eq!(
                (c.set_decode)(cd, d.as_ptr() as *const c_char, ds),
                (r.set_decode)(rd, d.as_ptr() as *const c_char, ds),
                "setStreamDecode(ds={})",
                ds
            );
            assert_bytes_eq(
                "setStreamDecode state",
                std::slice::from_raw_parts(cd as *const u8, LZ4_STREAMDECODESIZE),
                std::slice::from_raw_parts(rd as *const u8, LZ4_STREAMDECODESIZE),
            );
        }
        (c.free_decode)(cd);
        (r.free_decode)(rd);
    }
}
