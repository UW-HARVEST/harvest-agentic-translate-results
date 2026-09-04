//! Phase B differential tests for the one-shot LZ4 block API (lz4.c).

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

type FnCompressDefault = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnCompressFast =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnCompressFastExt =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnDecompressSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnDecompressPartial =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnDestSize = unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
type FnDestSizeExt =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;
type FnBound = unsafe extern "C" fn(c_int) -> c_int;
type FnDecompressFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnSafeUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;
type FnFastUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, *const c_char, c_int) -> c_int;
type FnPartialUsingDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_char,
    c_int,
) -> c_int;
type FnForceExtDictSafe =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_void, usize) -> c_int;
type FnForceExtDictPartial = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_void,
    usize,
) -> c_int;
// deprecated one-shots
type FnCompressDeprecated = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnCompressLimited = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnCompressWithState = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnCompressLimitedWithState =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnUncompress = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnUncompressUnknown = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;

struct Api {
    version_number: FnIntVoid,
    version_string: FnStrVoid,
    compress_bound: FnBound,
    sizeof_state: FnIntVoid,
    compress_default: FnCompressDefault,
    compress_fast: FnCompressFast,
    compress_fast_ext: FnCompressFastExt,
    compress_fast_ext_fastreset: FnCompressFastExt,
    compress_dest_size: FnDestSize,
    compress_dest_size_ext: FnDestSizeExt,
    decompress_safe: FnDecompressSafe,
    decompress_safe_partial: FnDecompressPartial,
    decompress_fast: FnDecompressFast,
    safe_using_dict: FnSafeUsingDict,
    fast_using_dict: FnFastUsingDict,
    partial_using_dict: FnPartialUsingDict,
    safe_force_ext: FnForceExtDictSafe,
    partial_force_ext: FnForceExtDictPartial,
    safe_prefix64k: FnDecompressSafe,
    fast_prefix64k: FnDecompressFast,
    decoder_ring_buffer_size: FnBound,
    // deprecated
    compress: FnCompressDeprecated,
    compress_limited: FnCompressLimited,
    compress_with_state: FnCompressWithState,
    compress_limited_with_state: FnCompressLimitedWithState,
    uncompress: FnUncompress,
    uncompress_unknown: FnUncompressUnknown,
    sizeof_stream_state: FnIntVoid,
}

fn bind(l: &Lib) -> Api {
    Api {
        version_number: l.sym("LZ4_versionNumber"),
        version_string: l.sym("LZ4_versionString"),
        compress_bound: l.sym("LZ4_compressBound"),
        sizeof_state: l.sym("LZ4_sizeofState"),
        compress_default: l.sym("LZ4_compress_default"),
        compress_fast: l.sym("LZ4_compress_fast"),
        compress_fast_ext: l.sym("LZ4_compress_fast_extState"),
        compress_fast_ext_fastreset: l.sym("LZ4_compress_fast_extState_fastReset"),
        compress_dest_size: l.sym("LZ4_compress_destSize"),
        compress_dest_size_ext: l.sym("LZ4_compress_destSize_extState"),
        decompress_safe: l.sym("LZ4_decompress_safe"),
        decompress_safe_partial: l.sym("LZ4_decompress_safe_partial"),
        decompress_fast: l.sym("LZ4_decompress_fast"),
        safe_using_dict: l.sym("LZ4_decompress_safe_usingDict"),
        fast_using_dict: l.sym("LZ4_decompress_fast_usingDict"),
        partial_using_dict: l.sym("LZ4_decompress_safe_partial_usingDict"),
        safe_force_ext: l.sym("LZ4_decompress_safe_forceExtDict"),
        partial_force_ext: l.sym("LZ4_decompress_safe_partial_forceExtDict"),
        safe_prefix64k: l.sym("LZ4_decompress_safe_withPrefix64k"),
        fast_prefix64k: l.sym("LZ4_decompress_fast_withPrefix64k"),
        decoder_ring_buffer_size: l.sym("LZ4_decoderRingBufferSize"),
        compress: l.sym("LZ4_compress"),
        compress_limited: l.sym("LZ4_compress_limitedOutput"),
        compress_with_state: l.sym("LZ4_compress_withState"),
        compress_limited_with_state: l.sym("LZ4_compress_limitedOutput_withState"),
        uncompress: l.sym("LZ4_uncompress"),
        uncompress_unknown: l.sym("LZ4_uncompress_unknownOutputSize"),
        sizeof_stream_state: l.sym("LZ4_sizeofStreamState"),
    }
}

fn pair() -> (Api, Api) {
    let p = libs();
    (bind(&p.c), bind(&p.r))
}

/// Test corpus: (shape, size) combinations covering every documented boundary.
fn corpus(rng: &mut Rng) -> Vec<(Shape, usize, Vec<u8>)> {
    let mut out = Vec::new();
    let sizes: Vec<usize> = BOUNDARY_SIZES
        .iter()
        .copied()
        .chain([100_000usize, 200_000])
        .collect();
    for &shape in ALL_SHAPES {
        for &n in &sizes {
            out.push((shape, n, gen(shape, n, rng)));
        }
    }
    // plus purely random sizes
    for _ in 0..120 {
        let n = rng.range(0, 40_000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        out.push((shape, n, gen(shape, n, rng)));
    }
    out
}

// --- CONFIGS: version / bound / state-size accessors -------------------------
#[test]
fn block_scalar_accessors() {
    let (c, r) = pair();
    unsafe {
        assert_eq!((c.version_number)(), (r.version_number)());
        assert_eq!(cstr((c.version_string)()), cstr((r.version_string)()));
        assert_eq!((c.sizeof_state)(), (r.sizeof_state)());
        assert_eq!((c.sizeof_stream_state)(), (r.sizeof_stream_state)());
        for n in [
            -1i32,
            0,
            1,
            2,
            16,
            255,
            65536,
            0x7E00_0000,
            0x7E00_0001,
            i32::MAX,
            i32::MIN,
        ] {
            assert_eq!(
                (c.compress_bound)(n),
                (r.compress_bound)(n),
                "LZ4_compressBound({})",
                n
            );
        }
        for n in [-1i32, 0, 1, 16, 65535, 65536, 1 << 20, i32::MAX, i32::MIN] {
            assert_eq!(
                (c.decoder_ring_buffer_size)(n),
                (r.decoder_ring_buffer_size)(n),
                "LZ4_decoderRingBufferSize({})",
                n
            );
        }
    }
}

// --- CONFIGS: LZ4_compress_default / LZ4_compress_fast (accelerations) -------
#[test]
fn block_compress_default_and_fast() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x1001);
    let accels: [c_int; 9] = [-1, 0, 1, 2, 3, 5, 17, 65537, i32::MAX];
    for (shape, n, data) in corpus(&mut rng) {
        let bound = unsafe { (c.compress_bound)(n as c_int) } as usize;
        assert_eq!(bound, unsafe { (r.compress_bound)(n as c_int) } as usize);
        let cap = bound.max(1);
        let mut cbuf = vec![0xAAu8; cap];
        let mut rbuf = vec![0xAAu8; cap];
        let src = data.as_ptr() as *const c_char;

        let a = unsafe { (c.compress_default)(src, cbuf.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int) };
        let b = unsafe { (r.compress_default)(src, rbuf.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int) };
        assert_eq!(a, b, "compress_default rc shape={:?} n={}", shape, n);
        assert_bytes_eq(
            &format!("compress_default shape={:?} n={}", shape, n),
            &cbuf[..a.max(0) as usize],
            &rbuf[..b.max(0) as usize],
        );

        for &acc in &accels {
            let mut cb = vec![0x55u8; cap];
            let mut rb = vec![0x55u8; cap];
            let a = unsafe {
                (c.compress_fast)(src, cb.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int, acc)
            };
            let b = unsafe {
                (r.compress_fast)(src, rb.as_mut_ptr() as *mut c_char, n as c_int, cap as c_int, acc)
            };
            assert_eq!(a, b, "compress_fast rc shape={:?} n={} acc={}", shape, n, acc);
            assert_bytes_eq(
                &format!("compress_fast shape={:?} n={} acc={}", shape, n, acc),
                &cb[..a.max(0) as usize],
                &rb[..b.max(0) as usize],
            );
            // round-trip through the *other* library's decompressor
            if a > 0 {
                let mut co = vec![0u8; n + 1];
                let mut ro = vec![0u8; n + 1];
                let x = unsafe {
                    (c.decompress_safe)(rb.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, a, n as c_int)
                };
                let y = unsafe {
                    (r.decompress_safe)(cb.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, a, n as c_int)
                };
                assert_eq!(x, n as c_int, "C cannot decode Rust output");
                assert_eq!(y, n as c_int, "Rust cannot decode C output");
                assert_bytes_eq("cross round-trip", &co[..n], &data);
                assert_bytes_eq("cross round-trip", &ro[..n], &data);
            }
        }
    }
}

// --- CONFIGS: extState variants ---------------------------------------------
#[test]
fn block_compress_ext_state() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x1002);
    let ss = unsafe { (c.sizeof_state)() } as usize;
    assert_eq!(ss, unsafe { (r.sizeof_state)() } as usize);
    // over-allocate and 8-byte align
    let mut cstate = vec![0u64; ss / 8 + 4];
    let mut rstate = vec![0u64; ss / 8 + 4];
    for (shape, n, data) in corpus(&mut rng) {
        let cap = (unsafe { (c.compress_bound)(n as c_int) } as usize).max(1);
        let src = data.as_ptr() as *const c_char;
        for &acc in &[1i32, 2, 8, 0, -3] {
            for fast_reset in [false, true] {
                // zeroed state is a valid "correctly initialised" state for fastReset
                for v in cstate.iter_mut() {
                    *v = 0;
                }
                for v in rstate.iter_mut() {
                    *v = 0;
                }
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let (a, b) = unsafe {
                    if fast_reset {
                        (
                            (c.compress_fast_ext_fastreset)(
                                cstate.as_mut_ptr() as *mut c_void,
                                src,
                                cb.as_mut_ptr() as *mut c_char,
                                n as c_int,
                                cap as c_int,
                                acc,
                            ),
                            (r.compress_fast_ext_fastreset)(
                                rstate.as_mut_ptr() as *mut c_void,
                                src,
                                rb.as_mut_ptr() as *mut c_char,
                                n as c_int,
                                cap as c_int,
                                acc,
                            ),
                        )
                    } else {
                        (
                            (c.compress_fast_ext)(
                                cstate.as_mut_ptr() as *mut c_void,
                                src,
                                cb.as_mut_ptr() as *mut c_char,
                                n as c_int,
                                cap as c_int,
                                acc,
                            ),
                            (r.compress_fast_ext)(
                                rstate.as_mut_ptr() as *mut c_void,
                                src,
                                rb.as_mut_ptr() as *mut c_char,
                                n as c_int,
                                cap as c_int,
                                acc,
                            ),
                        )
                    }
                };
                assert_eq!(
                    a, b,
                    "extState(fastReset={}) rc shape={:?} n={} acc={}",
                    fast_reset, shape, n, acc
                );
                assert_bytes_eq(
                    &format!("extState(fastReset={}) shape={:?} n={} acc={}", fast_reset, shape, n, acc),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
                // the state itself must also evolve identically
                assert_bytes_eq(
                    "extState internal state",
                    unsafe { std::slice::from_raw_parts(cstate.as_ptr() as *const u8, ss) },
                    unsafe { std::slice::from_raw_parts(rstate.as_ptr() as *const u8, ss) },
                );
            }
        }
    }
}

// --- CONFIGS: LZ4_compress_destSize (+ extState) -----------------------------
#[test]
fn block_compress_dest_size() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x1003);
    let ss = unsafe { (c.sizeof_state)() } as usize;
    let mut cstate = vec![0u64; ss / 8 + 4];
    let mut rstate = vec![0u64; ss / 8 + 4];
    for (shape, n, data) in corpus(&mut rng) {
        let full = (unsafe { (c.compress_bound)(n as c_int) } as usize).max(1);
        // target capacities from far too small to more than enough
        let mut targets: Vec<usize> = vec![0, 1, 2, 3, 4, 8, 13, 16, 32, 64, full];
        if full > 8 {
            targets.push(full / 2);
            targets.push(full / 4);
            targets.push(full / 8 + 1);
            targets.push(rng.range(1, full));
        }
        for &t in &targets {
            let mut cs = n as c_int;
            let mut rs = n as c_int;
            let mut cb = vec![0u8; t + 1];
            let mut rb = vec![0u8; t + 1];
            let a = unsafe {
                (c.compress_dest_size)(
                    data.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    &mut cs,
                    t as c_int,
                )
            };
            let b = unsafe {
                (r.compress_dest_size)(
                    data.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    &mut rs,
                    t as c_int,
                )
            };
            assert_eq!(a, b, "destSize rc shape={:?} n={} target={}", shape, n, t);
            assert_eq!(cs, rs, "destSize *srcSizePtr shape={:?} n={} target={}", shape, n, t);
            assert_bytes_eq(
                &format!("destSize shape={:?} n={} target={}", shape, n, t),
                &cb[..a.max(0) as usize],
                &rb[..b.max(0) as usize],
            );
            // the produced block must decode back to the first `cs` source bytes
            if a > 0 {
                let mut o = vec![0u8; cs as usize + 1];
                let got = unsafe {
                    (r.decompress_safe)(
                        cb.as_ptr() as *const c_char,
                        o.as_mut_ptr() as *mut c_char,
                        a,
                        cs,
                    )
                };
                assert_eq!(got, cs, "destSize output not decodable");
                assert_bytes_eq("destSize roundtrip", &o[..cs as usize], &data[..cs as usize]);
            }

            for &acc in &[1i32, 4, 0, -2] {
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
                    (c.compress_dest_size_ext)(
                        cstate.as_mut_ptr() as *mut c_void,
                        data.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        &mut cs,
                        t as c_int,
                        acc,
                    )
                };
                let b = unsafe {
                    (r.compress_dest_size_ext)(
                        rstate.as_mut_ptr() as *mut c_void,
                        data.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        &mut rs,
                        t as c_int,
                        acc,
                    )
                };
                assert_eq!(
                    a, b,
                    "destSize_extState rc shape={:?} n={} t={} acc={}",
                    shape, n, t, acc
                );
                assert_eq!(cs, rs, "destSize_extState srcSize shape={:?} n={} t={} acc={}", shape, n, t, acc);
                assert_bytes_eq(
                    &format!("destSize_extState shape={:?} n={} t={} acc={}", shape, n, t, acc),
                    &cb[..a.max(0) as usize],
                    &rb[..b.max(0) as usize],
                );
            }
        }
    }
}

// --- CONFIGS: LZ4_decompress_safe with every dstCapacity ---------------------
#[test]
fn block_decompress_safe_capacities() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x1004);
    for (shape, n, data) in corpus(&mut rng) {
        let cap = (unsafe { (c.compress_bound)(n as c_int) } as usize).max(1);
        let mut comp = vec![0u8; cap];
        let clen = unsafe {
            (c.compress_default)(
                data.as_ptr() as *const c_char,
                comp.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
            )
        };
        assert!(clen > 0 || n == 0);
        if clen <= 0 {
            continue;
        }
        let mut caps: Vec<c_int> = vec![0, 1, n as c_int, n as c_int + 1, n as c_int + 100];
        if n > 2 {
            caps.push((n / 2) as c_int);
            caps.push((n - 1) as c_int);
            caps.push(rng.range(1, n) as c_int);
        }
        for &dcap in &caps {
            let sz = (dcap.max(0) as usize) + 16;
            let mut co = vec![0x33u8; sz];
            let mut ro = vec![0x33u8; sz];
            let a = unsafe {
                (c.decompress_safe)(
                    comp.as_ptr() as *const c_char,
                    co.as_mut_ptr() as *mut c_char,
                    clen,
                    dcap,
                )
            };
            let b = unsafe {
                (r.decompress_safe)(
                    comp.as_ptr() as *const c_char,
                    ro.as_mut_ptr() as *mut c_char,
                    clen,
                    dcap,
                )
            };
            assert_eq!(
                a, b,
                "decompress_safe rc shape={:?} n={} dstCapacity={}",
                shape, n, dcap
            );
            if a > 0 {
                assert_bytes_eq(
                    &format!("decompress_safe shape={:?} n={} dcap={}", shape, n, dcap),
                    &co[..a as usize],
                    &ro[..b as usize],
                );
            }
        }
        // truncated compressed input
        for cut in [0i32, 1, clen / 2, clen - 1, clen + 1] {
            if cut < 0 {
                continue;
            }
            let mut co = vec![0u8; n + 16];
            let mut ro = vec![0u8; n + 16];
            let a = unsafe {
                (c.decompress_safe)(
                    comp.as_ptr() as *const c_char,
                    co.as_mut_ptr() as *mut c_char,
                    cut,
                    n as c_int,
                )
            };
            let b = unsafe {
                (r.decompress_safe)(
                    comp.as_ptr() as *const c_char,
                    ro.as_mut_ptr() as *mut c_char,
                    cut,
                    n as c_int,
                )
            };
            assert_eq!(
                a, b,
                "decompress_safe truncated rc shape={:?} n={} srcSize={}",
                shape, n, cut
            );
        }
    }
}

// --- CONFIGS: LZ4_decompress_safe_partial ------------------------------------
#[test]
fn block_decompress_partial() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x1005);
    for (shape, n, data) in corpus(&mut rng) {
        let cap = (unsafe { (c.compress_bound)(n as c_int) } as usize).max(1);
        let mut comp = vec![0u8; cap];
        let clen = unsafe {
            (c.compress_default)(
                data.as_ptr() as *const c_char,
                comp.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
            )
        };
        if clen <= 0 {
            continue;
        }
        let mut targets: Vec<c_int> = vec![0, 1, n as c_int, n as c_int + 5];
        if n > 3 {
            targets.push((n / 3) as c_int);
            targets.push((n - 1) as c_int);
            targets.push(rng.range(1, n) as c_int);
        }
        for &t in &targets {
            for &dcap in &[t, t + 1, n as c_int, n as c_int + 8, 0] {
                let sz = (dcap.max(0) as usize) + 32;
                let mut co = vec![0x77u8; sz];
                let mut ro = vec![0x77u8; sz];
                let a = unsafe {
                    (c.decompress_safe_partial)(
                        comp.as_ptr() as *const c_char,
                        co.as_mut_ptr() as *mut c_char,
                        clen,
                        t,
                        dcap,
                    )
                };
                let b = unsafe {
                    (r.decompress_safe_partial)(
                        comp.as_ptr() as *const c_char,
                        ro.as_mut_ptr() as *mut c_char,
                        clen,
                        t,
                        dcap,
                    )
                };
                assert_eq!(
                    a, b,
                    "partial rc shape={:?} n={} target={} dcap={}",
                    shape, n, t, dcap
                );
                assert_bytes_eq(
                    &format!("partial shape={:?} n={} target={} dcap={}", shape, n, t, dcap),
                    &co,
                    &ro,
                );
            }
        }
    }
}

// --- CONFIGS: LZ4_decompress_fast (deprecated, exact size) -------------------
#[test]
fn block_decompress_fast() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x1006);
    for (shape, n, data) in corpus(&mut rng) {
        let cap = (unsafe { (c.compress_bound)(n as c_int) } as usize).max(1);
        let mut comp = vec![0u8; cap];
        let clen = unsafe {
            (c.compress_default)(
                data.as_ptr() as *const c_char,
                comp.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
            )
        };
        if clen <= 0 {
            continue;
        }
        // over-allocate generously: LZ4_decompress_fast may write up to 32 bytes past
        let mut co = vec![0u8; n + 64];
        let mut ro = vec![0u8; n + 64];
        let a = unsafe {
            (c.decompress_fast)(comp.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, n as c_int)
        };
        let b = unsafe {
            (r.decompress_fast)(comp.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, n as c_int)
        };
        assert_eq!(a, b, "decompress_fast rc shape={:?} n={}", shape, n);
        assert_bytes_eq(
            &format!("decompress_fast shape={:?} n={}", shape, n),
            &co[..n],
            &ro[..n],
        );
        assert_bytes_eq("decompress_fast content", &co[..n], &data);
        // deprecated aliases
        let mut co2 = vec![0u8; n + 64];
        let mut ro2 = vec![0u8; n + 64];
        let a = unsafe {
            (c.uncompress)(comp.as_ptr() as *const c_char, co2.as_mut_ptr() as *mut c_char, n as c_int)
        };
        let b = unsafe {
            (r.uncompress)(comp.as_ptr() as *const c_char, ro2.as_mut_ptr() as *mut c_char, n as c_int)
        };
        assert_eq!(a, b, "LZ4_uncompress rc");
        assert_bytes_eq("LZ4_uncompress", &co2[..n], &ro2[..n]);
        let mut co3 = vec![0u8; n + 64];
        let mut ro3 = vec![0u8; n + 64];
        let a = unsafe {
            (c.uncompress_unknown)(
                comp.as_ptr() as *const c_char,
                co3.as_mut_ptr() as *mut c_char,
                clen,
                n as c_int,
            )
        };
        let b = unsafe {
            (r.uncompress_unknown)(
                comp.as_ptr() as *const c_char,
                ro3.as_mut_ptr() as *mut c_char,
                clen,
                n as c_int,
            )
        };
        assert_eq!(a, b, "LZ4_uncompress_unknownOutputSize rc");
        assert_bytes_eq("LZ4_uncompress_unknownOutputSize", &co3[..n], &ro3[..n]);
    }
}

// --- CONFIGS: deprecated one-shot compressors -------------------------------
#[test]
fn block_deprecated_compressors() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x1007);
    let ss = unsafe { (c.sizeof_state)() } as usize;
    let mut cstate = vec![0u64; ss / 8 + 4];
    let mut rstate = vec![0u64; ss / 8 + 4];
    for (shape, n, data) in corpus(&mut rng) {
        let cap = (unsafe { (c.compress_bound)(n as c_int) } as usize).max(1);
        let src = data.as_ptr() as *const c_char;
        let mut cb = vec![0u8; cap];
        let mut rb = vec![0u8; cap];
        let a = unsafe { (c.compress)(src, cb.as_mut_ptr() as *mut c_char, n as c_int) };
        let b = unsafe { (r.compress)(src, rb.as_mut_ptr() as *mut c_char, n as c_int) };
        assert_eq!(a, b, "LZ4_compress rc shape={:?} n={}", shape, n);
        assert_bytes_eq("LZ4_compress", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);

        for &lim in &[0usize, 1, cap / 2 + 1, cap] {
            let mut cb = vec![0u8; lim + 1];
            let mut rb = vec![0u8; lim + 1];
            let a = unsafe {
                (c.compress_limited)(src, cb.as_mut_ptr() as *mut c_char, n as c_int, lim as c_int)
            };
            let b = unsafe {
                (r.compress_limited)(src, rb.as_mut_ptr() as *mut c_char, n as c_int, lim as c_int)
            };
            assert_eq!(a, b, "LZ4_compress_limitedOutput rc n={} lim={}", n, lim);
            assert_bytes_eq(
                "LZ4_compress_limitedOutput",
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
            (c.compress_with_state)(
                cstate.as_mut_ptr() as *mut c_void,
                src,
                cb.as_mut_ptr() as *mut c_char,
                n as c_int,
            )
        };
        let b = unsafe {
            (r.compress_with_state)(
                rstate.as_mut_ptr() as *mut c_void,
                src,
                rb.as_mut_ptr() as *mut c_char,
                n as c_int,
            )
        };
        assert_eq!(a, b, "LZ4_compress_withState rc n={}", n);
        assert_bytes_eq("LZ4_compress_withState", &cb[..a.max(0) as usize], &rb[..b.max(0) as usize]);

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
            (c.compress_limited_with_state)(
                cstate.as_mut_ptr() as *mut c_void,
                src,
                cb.as_mut_ptr() as *mut c_char,
                n as c_int,
                lim as c_int,
            )
        };
        let b = unsafe {
            (r.compress_limited_with_state)(
                rstate.as_mut_ptr() as *mut c_void,
                src,
                rb.as_mut_ptr() as *mut c_char,
                n as c_int,
                lim as c_int,
            )
        };
        assert_eq!(a, b, "LZ4_compress_limitedOutput_withState rc n={}", n);
        assert_bytes_eq(
            "LZ4_compress_limitedOutput_withState",
            &cb[..a.max(0) as usize],
            &rb[..b.max(0) as usize],
        );
    }
}

// --- CONFIGS: stateless dictionary decompression ----------------------------
#[test]
fn block_decompress_using_dict() {
    let (c, r) = pair();
    let p = libs();
    let mut rng = Rng::new(0x1008);
    // We need dictionary-compressed blocks. Use the streaming compressor from the
    // C library to produce them, then decode with both stateless dict decoders.
    type FnCreateStream = unsafe extern "C" fn() -> *mut c_void;
    type FnFreeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
    type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
    type FnCompressContinue =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
    let create: FnCreateStream = p.c.sym("LZ4_createStream");
    let free: FnFreeStream = p.c.sym("LZ4_freeStream");
    let load: FnLoadDict = p.c.sym("LZ4_loadDict");
    let cont: FnCompressContinue = p.c.sym("LZ4_compress_fast_continue");

    let dict_sizes = [0usize, 1, 4, 13, 64, 1000, 65535, 65536, 70000];
    for &ds in &dict_sizes {
        let dict = gen(Shape::Text, ds, &mut rng);
        for &n in &[0usize, 1, 5, 13, 64, 1000, 40_000] {
            for &shape in &[Shape::Text, Shape::Random, Shape::Runs] {
                // make the payload share content with the dictionary so matches occur
                let mut data = gen(shape, n, &mut rng);
                if ds > 0 && n > 0 {
                    let k = n.min(ds);
                    data[..k].copy_from_slice(&dict[..k]);
                }
                let cap = (unsafe { (c.compress_bound)(n as c_int) } as usize).max(1);
                let mut comp = vec![0u8; cap];
                let clen = unsafe {
                    let s = create();
                    load(s, dict.as_ptr() as *const c_char, ds as c_int);
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
                // LZ4_decompress_safe_usingDict
                let mut co = vec![0u8; n + 64];
                let mut ro = vec![0u8; n + 64];
                let a = unsafe {
                    (c.safe_using_dict)(
                        comp.as_ptr() as *const c_char,
                        co.as_mut_ptr() as *mut c_char,
                        clen,
                        n as c_int,
                        dict.as_ptr() as *const c_char,
                        ds as c_int,
                    )
                };
                let b = unsafe {
                    (r.safe_using_dict)(
                        comp.as_ptr() as *const c_char,
                        ro.as_mut_ptr() as *mut c_char,
                        clen,
                        n as c_int,
                        dict.as_ptr() as *const c_char,
                        ds as c_int,
                    )
                };
                assert_eq!(a, b, "safe_usingDict rc ds={} n={} shape={:?}", ds, n, shape);
                assert_bytes_eq("safe_usingDict", &co[..a.max(0) as usize], &ro[..b.max(0) as usize]);
                if a > 0 {
                    assert_bytes_eq("safe_usingDict content", &co[..n], &data);
                }
                // LZ4_decompress_fast_usingDict
                let mut co = vec![0u8; n + 64];
                let mut ro = vec![0u8; n + 64];
                let a = unsafe {
                    (c.fast_using_dict)(
                        comp.as_ptr() as *const c_char,
                        co.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        dict.as_ptr() as *const c_char,
                        ds as c_int,
                    )
                };
                let b = unsafe {
                    (r.fast_using_dict)(
                        comp.as_ptr() as *const c_char,
                        ro.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        dict.as_ptr() as *const c_char,
                        ds as c_int,
                    )
                };
                assert_eq!(a, b, "fast_usingDict rc ds={} n={}", ds, n);
                assert_bytes_eq("fast_usingDict", &co[..n], &ro[..n]);
                // LZ4_decompress_safe_partial_usingDict
                for &t in &[0i32, 1, (n / 2) as c_int, n as c_int] {
                    let mut co = vec![0u8; n + 64];
                    let mut ro = vec![0u8; n + 64];
                    let a = unsafe {
                        (c.partial_using_dict)(
                            comp.as_ptr() as *const c_char,
                            co.as_mut_ptr() as *mut c_char,
                            clen,
                            t,
                            n as c_int,
                            dict.as_ptr() as *const c_char,
                            ds as c_int,
                        )
                    };
                    let b = unsafe {
                        (r.partial_using_dict)(
                            comp.as_ptr() as *const c_char,
                            ro.as_mut_ptr() as *mut c_char,
                            clen,
                            t,
                            n as c_int,
                            dict.as_ptr() as *const c_char,
                            ds as c_int,
                        )
                    };
                    assert_eq!(a, b, "partial_usingDict rc ds={} n={} t={}", ds, n, t);
                    assert_bytes_eq("partial_usingDict", &co, &ro);
                }
                // forceExtDict entry points (internal, test-only exports)
                let mut co = vec![0u8; n + 64];
                let mut ro = vec![0u8; n + 64];
                let a = unsafe {
                    (c.safe_force_ext)(
                        comp.as_ptr() as *const c_char,
                        co.as_mut_ptr() as *mut c_char,
                        clen,
                        n as c_int,
                        dict.as_ptr() as *const c_void,
                        ds,
                    )
                };
                let b = unsafe {
                    (r.safe_force_ext)(
                        comp.as_ptr() as *const c_char,
                        ro.as_mut_ptr() as *mut c_char,
                        clen,
                        n as c_int,
                        dict.as_ptr() as *const c_void,
                        ds,
                    )
                };
                assert_eq!(a, b, "safe_forceExtDict rc ds={} n={}", ds, n);
                assert_bytes_eq("safe_forceExtDict", &co, &ro);
                let mut co = vec![0u8; n + 64];
                let mut ro = vec![0u8; n + 64];
                let a = unsafe {
                    (c.partial_force_ext)(
                        comp.as_ptr() as *const c_char,
                        co.as_mut_ptr() as *mut c_char,
                        clen,
                        (n / 2) as c_int,
                        n as c_int,
                        dict.as_ptr() as *const c_void,
                        ds,
                    )
                };
                let b = unsafe {
                    (r.partial_force_ext)(
                        comp.as_ptr() as *const c_char,
                        ro.as_mut_ptr() as *mut c_char,
                        clen,
                        (n / 2) as c_int,
                        n as c_int,
                        dict.as_ptr() as *const c_void,
                        ds,
                    )
                };
                assert_eq!(a, b, "partial_forceExtDict rc ds={} n={}", ds, n);
                assert_bytes_eq("partial_forceExtDict", &co, &ro);
            }
        }
    }
}

// --- CONFIGS: prefix64k decoders (dict immediately precedes dst) -------------
#[test]
fn block_decompress_prefix64k() {
    let (c, r) = pair();
    let p = libs();
    type FnCreateStream = unsafe extern "C" fn() -> *mut c_void;
    type FnFreeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
    type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
    type FnCompressContinue =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
    let create: FnCreateStream = p.c.sym("LZ4_createStream");
    let free: FnFreeStream = p.c.sym("LZ4_freeStream");
    let load: FnLoadDict = p.c.sym("LZ4_loadDict");
    let cont: FnCompressContinue = p.c.sym("LZ4_compress_fast_continue");

    let mut rng = Rng::new(0x1009);
    const DICT: usize = 65536;
    for &n in &[1usize, 13, 100, 5000, 40_000] {
        let dict = gen(Shape::Text, DICT, &mut rng);
        let mut data = gen(Shape::Text, n, &mut rng);
        let k = n.min(DICT);
        data[..k].copy_from_slice(&dict[..k]);
        let cap = (unsafe { (c.compress_bound)(n as c_int) } as usize).max(1);
        let mut comp = vec![0u8; cap];
        let clen = unsafe {
            let s = create();
            load(s, dict.as_ptr() as *const c_char, DICT as c_int);
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
        assert!(clen > 0);
        // contiguous buffer: [64KB dict][decoded output]
        let mut cbuf = vec![0u8; DICT + n + 64];
        let mut rbuf = vec![0u8; DICT + n + 64];
        cbuf[..DICT].copy_from_slice(&dict);
        rbuf[..DICT].copy_from_slice(&dict);
        let a = unsafe {
            (c.safe_prefix64k)(
                comp.as_ptr() as *const c_char,
                cbuf.as_mut_ptr().add(DICT) as *mut c_char,
                clen,
                n as c_int,
            )
        };
        let b = unsafe {
            (r.safe_prefix64k)(
                comp.as_ptr() as *const c_char,
                rbuf.as_mut_ptr().add(DICT) as *mut c_char,
                clen,
                n as c_int,
            )
        };
        assert_eq!(a, b, "safe_withPrefix64k rc n={}", n);
        assert_bytes_eq("safe_withPrefix64k", &cbuf, &rbuf);
        assert_bytes_eq("safe_withPrefix64k content", &cbuf[DICT..DICT + n], &data);

        let mut cbuf = vec![0u8; DICT + n + 64];
        let mut rbuf = vec![0u8; DICT + n + 64];
        cbuf[..DICT].copy_from_slice(&dict);
        rbuf[..DICT].copy_from_slice(&dict);
        let a = unsafe {
            (c.fast_prefix64k)(
                comp.as_ptr() as *const c_char,
                cbuf.as_mut_ptr().add(DICT) as *mut c_char,
                n as c_int,
            )
        };
        let b = unsafe {
            (r.fast_prefix64k)(
                comp.as_ptr() as *const c_char,
                rbuf.as_mut_ptr().add(DICT) as *mut c_char,
                n as c_int,
            )
        };
        assert_eq!(a, b, "fast_withPrefix64k rc n={}", n);
        assert_bytes_eq("fast_withPrefix64k", &cbuf[..DICT + n], &rbuf[..DICT + n]);
    }
}
