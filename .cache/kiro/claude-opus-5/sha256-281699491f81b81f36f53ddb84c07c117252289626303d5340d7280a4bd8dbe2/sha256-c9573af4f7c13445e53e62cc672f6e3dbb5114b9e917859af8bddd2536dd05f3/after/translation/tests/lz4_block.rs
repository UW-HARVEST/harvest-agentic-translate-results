//! Phase B/C — block-level LZ4 API (`lz4.c`).
//! CONFIGS.md rows 1–39, ERRORS.md rows 1–47.
#![allow(non_snake_case)]

mod common;
use common::*;
use libloading::Library;
use std::ffi::CStr;

type FnCompress4 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnCompress5 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnCompressState = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnDestSize = unsafe extern "C" fn(*const u8, *mut u8, *mut i32, i32) -> i32;
type FnDecSafe = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnDecPartial = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnDecFast = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
type FnDecUsingDict = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, *const u8, i32) -> i32;
type FnDecPartialUsingDict =
    unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32, *const u8, i32) -> i32;
type FnCreateStream = unsafe extern "C" fn() -> *mut CVoid;
type FnFreeStream = unsafe extern "C" fn(*mut CVoid) -> i32;
type FnVoidPtr = unsafe extern "C" fn(*mut CVoid);
type FnLoadDict = unsafe extern "C" fn(*mut CVoid, *const u8, i32) -> i32;
type FnAttach = unsafe extern "C" fn(*mut CVoid, *const CVoid);
type FnContinue = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnSaveDict = unsafe extern "C" fn(*mut CVoid, *mut u8, i32) -> i32;
type FnDecContinue = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32, i32) -> i32;
type FnDecFastContinue = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32) -> i32;
type FnSetStreamDecode = unsafe extern "C" fn(*mut CVoid, *const u8, i32) -> i32;
type FnInitStream = unsafe extern "C" fn(*mut CVoid, usize) -> *mut CVoid;
type FnCompress3 = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
type FnCompressWithState4 = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32) -> i32;
type FnCompressWithState5 = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32, i32) -> i32;
type FnResetStreamState = unsafe extern "C" fn(*mut CVoid, *mut u8) -> i32;
type FnCreate = unsafe extern "C" fn(*mut u8) -> *mut CVoid;
type FnSlide = unsafe extern "C" fn(*mut CVoid) -> *mut u8;

/// Size of `LZ4_stream_t` as reported by the library itself.
fn sizeof_state(lib: &Library) -> usize {
    unsafe { sym::<FnVoidI32>(lib, "LZ4_sizeofState")() as usize }
}

/* ================================================================== */
/* row 1 / errors 1,2 — LZ4_compressBound                             */
/* ================================================================== */

#[test]
fn r001_compress_bound() {
    let cases: Vec<i32> = vec![
        i32::MIN,
        -1000000,
        -1,
        0,
        1,
        2,
        3,
        4,
        15,
        16,
        63,
        64,
        255,
        256,
        65534,
        65535,
        65536,
        1 << 20,
        LZ4_MAX_INPUT_SIZE - 1,
        LZ4_MAX_INPUT_SIZE,
        LZ4_MAX_INPUT_SIZE + 1,
        i32::MAX,
    ];
    diff("LZ4_compressBound", |lib| {
        let f = unsafe { sym::<FnI32I32>(lib, "LZ4_compressBound") };
        cases.iter().map(|&c| unsafe { f(c) }).collect::<Vec<i32>>()
    });
}

#[test]
fn r011_sizeof_and_versions() {
    diff("sizeof/version", |lib| {
        unsafe {
            let vn = sym::<FnVoidI32>(lib, "LZ4_versionNumber")();
            let vsf = sym::<unsafe extern "C" fn() -> *const CChar>(lib, "LZ4_versionString");
            let vs = CStr::from_ptr(vsf()).to_string_lossy().into_owned();
            (
                vn,
                vs,
                sym::<FnVoidI32>(lib, "LZ4_sizeofState")(),
                sym::<FnVoidI32>(lib, "LZ4_sizeofStreamState")(),
                sym::<FnVoidI32>(lib, "LZ4_sizeofStateHC")(),
                sym::<FnVoidI32>(lib, "LZ4_sizeofStreamStateHC")(),
            )
        }
    });
}

/* ================================================================== */
/* rows 2,3,4,5 — LZ4_compress_default across size / shape / dst       */
/* ================================================================== */

fn compress_default_case(lib: &Library, src: &[u8], dst_cap: i32) -> (i32, Vec<u8>) {
    let f = unsafe { sym::<FnCompress4>(lib, "LZ4_compress_default") };
    let cap = dst_cap.max(0) as usize;
    let mut dst = vec![0xA5u8; cap + 8];
    let n = unsafe {
        f(
            if src.is_empty() {
                std::ptr::null()
            } else {
                src.as_ptr()
            },
            dst.as_mut_ptr(),
            src.len() as i32,
            dst_cap,
        )
    };
    let produced = if n > 0 { n as usize } else { 0 };
    dst.truncate(produced);
    (n, dst)
}

#[test]
fn r002_compress_default_small() {
    let mut rng = Rng::new(0x5EED_0002);
    for &shape in ALL_SHAPES.iter() {
        for len in 0usize..=64 {
            let src = mkdata(shape, len, &mut rng);
            let bound = compress_bound(len as i32);
            diff(&format!("compress_default small {shape:?} len={len}"), |lib| {
                compress_default_case(lib, &src, bound)
            });
        }
    }
}

#[test]
fn r003_compress_default_byU16() {
    let mut rng = Rng::new(0x5EED_0003);
    // srcSize < LZ4_64Klimit (65535+1) => byU16 hash table
    let lens = [
        65usize, 100, 511, 512, 1000, 4095, 4096, 16384, 65533, 65534, 65535,
    ];
    for &shape in ALL_SHAPES.iter() {
        for &len in lens.iter() {
            let src = mkdata(shape, len, &mut rng);
            let bound = compress_bound(len as i32);
            diff(&format!("compress_default byU16 {shape:?} len={len}"), |lib| {
                compress_default_case(lib, &src, bound)
            });
        }
    }
    // randomized
    for i in 0..200 {
        let shape = ALL_SHAPES[i % ALL_SHAPES.len()];
        let len = rng.range(1, 65536);
        let src = mkdata(shape, len, &mut rng);
        let bound = compress_bound(len as i32);
        diff(&format!("compress_default rand16 #{i} {shape:?} len={len}"), |lib| {
            compress_default_case(lib, &src, bound)
        });
    }
}

#[test]
fn r004_compress_default_byU32() {
    let mut rng = Rng::new(0x5EED_0004);
    let lens = [65536usize, 65537, 70000, 131072, 200000, 300000];
    for &shape in ALL_SHAPES.iter() {
        for &len in lens.iter() {
            let src = mkdata(shape, len, &mut rng);
            let bound = compress_bound(len as i32);
            diff(&format!("compress_default byU32 {shape:?} len={len}"), |lib| {
                compress_default_case(lib, &src, bound)
            });
        }
    }
    for i in 0..40 {
        let shape = ALL_SHAPES[i % ALL_SHAPES.len()];
        let len = rng.range(65536, 400000);
        let src = mkdata(shape, len, &mut rng);
        let bound = compress_bound(len as i32);
        diff(&format!("compress_default rand32 #{i} {shape:?} len={len}"), |lib| {
            compress_default_case(lib, &src, bound)
        });
    }
}

/* errors 3,4,5,6,7 */
#[test]
fn e003_compress_default_bad_sizes() {
    let mut rng = Rng::new(0x5EED_1003);
    let src = mkdata(Shape::Random, 4096, &mut rng);
    // negative and oversized srcSize
    for &bad in [-1i32, -100, i32::MIN, LZ4_MAX_INPUT_SIZE + 1, i32::MAX].iter() {
        diff(&format!("compress_default badsrc {bad}"), |lib| {
            let f = unsafe { sym::<FnCompress4>(lib, "LZ4_compress_default") };
            let mut dst = vec![0u8; 8192];
            unsafe { f(src.as_ptr(), dst.as_mut_ptr(), bad, dst.len() as i32) }
        });
    }
    // tight dstCapacity sweep
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 3000, &mut rng);
        let exact = {
            let i = impls();
            compress_default_case(&i.c, &src, compress_bound(3000)).0
        };
        for cap in [
            0i32,
            1,
            2,
            8,
            exact / 4,
            exact / 2,
            exact - 2,
            exact - 1,
            exact,
            exact + 1,
        ] {
            diff(&format!("compress_default tight {shape:?} cap={cap}"), |lib| {
                compress_default_case(lib, &src, cap)
            });
        }
    }
    // srcSize == 0 with various dstCapacity
    for cap in [-1i32, 0, 1, 2, 16] {
        diff(&format!("compress_default empty cap={cap}"), |lib| {
            let f = unsafe { sym::<FnCompress4>(lib, "LZ4_compress_default") };
            let mut dst = vec![0xEEu8; 32];
            let n = unsafe { f(std::ptr::null(), dst.as_mut_ptr(), 0, cap) };
            (n, dst[0])
        });
    }
}

/* ================================================================== */
/* rows 6,7,8 / errors 8,9,10,11 — acceleration sweep                  */
/* ================================================================== */

const ACCELS: [i32; 12] = [
    i32::MIN,
    -1,
    0,
    1,
    2,
    3,
    17,
    1000,
    65536,
    LZ4_ACCELERATION_MAX,
    LZ4_ACCELERATION_MAX + 1,
    i32::MAX,
];

#[test]
fn r006_compress_fast_accel() {
    let mut rng = Rng::new(0x5EED_0006);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 4, 63, 1000, 40000, 65536, 120000].iter() {
            let src = mkdata(shape, len, &mut rng);
            for &a in ACCELS.iter() {
                let bound = compress_bound(len as i32).max(1);
                diff(&format!("compress_fast {shape:?} len={len} a={a}"), |lib| {
                    let f = unsafe { sym::<FnCompress5>(lib, "LZ4_compress_fast") };
                    let mut dst = vec![0xA5u8; bound as usize + 8];
                    let n = unsafe {
                        f(
                            if src.is_empty() { std::ptr::null() } else { src.as_ptr() },
                            dst.as_mut_ptr(),
                            len as i32,
                            bound,
                            a,
                        )
                    };
                    dst.truncate(if n > 0 { n as usize } else { 0 });
                    (n, dst)
                });
            }
        }
    }
}

#[test]
fn r007_compress_fast_extState() {
    let mut rng = Rng::new(0x5EED_0007);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 5, 64, 1000, 65535, 65536, 150000].iter() {
            let src = mkdata(shape, len, &mut rng);
            for &a in [-1i32, 1, 2, 17, LZ4_ACCELERATION_MAX + 1].iter() {
                for limited in [false, true] {
                    let bound = compress_bound(len as i32).max(1);
                    let cap = if limited { (bound / 2).max(1) } else { bound };
                    diff(
                        &format!("extState {shape:?} len={len} a={a} lim={limited}"),
                        |lib| {
                            let ss = sizeof_state(lib);
                            let mut st = Aligned::new(ss + STATE_SLOP, 16);
                            let f = unsafe { sym::<FnCompressState>(lib, "LZ4_compress_fast_extState") };
                            let mut dst = vec![0xA5u8; bound as usize + 8];
                            let n = unsafe {
                                f(
                                    st.as_mut_ptr() as *mut CVoid,
                                    if src.is_empty() { std::ptr::null() } else { src.as_ptr() },
                                    dst.as_mut_ptr(),
                                    len as i32,
                                    cap,
                                    a,
                                )
                            };
                            dst.truncate(if n > 0 { n as usize } else { 0 });
                            (n, dst)
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn r008_compress_fast_extState_fastReset() {
    let mut rng = Rng::new(0x5EED_0008);
    // reuse the same state across several calls: exercises the currentOffset!=0
    // (dictSmall) branch on the 2nd+ call.
    for &shape in ALL_SHAPES.iter() {
        let chunks: Vec<Vec<u8>> = (0..6)
            .map(|_| {
                let l = rng.range(1, 70000);
                mkdata(shape, l, &mut rng)
            })
            .collect();
        for &a in [-1i32, 1, 4, LZ4_ACCELERATION_MAX + 1].iter() {
            for limited in [false, true] {
                diff(&format!("fastReset {shape:?} a={a} lim={limited}"), |lib| {
                    let ss = sizeof_state(lib);
                    let mut st = Aligned::new(ss + STATE_SLOP, 16);
                    // must be initialized once via LZ4_initStream
                    unsafe {
                        let ini = sym::<FnInitStream>(lib, "LZ4_initStream");
                        let p = ini(st.as_mut_ptr() as *mut CVoid, ss);
                        assert!(!p.is_null());
                    }
                    let f =
                        unsafe { sym::<FnCompressState>(lib, "LZ4_compress_fast_extState_fastReset") };
                    let mut out = Vec::new();
                    for c in chunks.iter() {
                        let bound = compress_bound(c.len() as i32).max(1);
                        let cap = if limited { (bound / 2).max(1) } else { bound };
                        let mut dst = vec![0xA5u8; bound as usize + 8];
                        let n = unsafe {
                            f(
                                st.as_mut_ptr() as *mut CVoid,
                                c.as_ptr(),
                                dst.as_mut_ptr(),
                                c.len() as i32,
                                cap,
                                a,
                            )
                        };
                        out.push(n);
                        dst.truncate(if n > 0 { n as usize } else { 0 });
                        out.extend(dst.iter().map(|&b| b as i32));
                    }
                    out
                });
            }
        }
    }
}

/* ================================================================== */
/* rows 9,10 / errors 12,13 — LZ4_compress_destSize                    */
/* ================================================================== */

#[test]
fn r009_compress_destSize() {
    let mut rng = Rng::new(0x5EED_0009);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 4, 20, 500, 5000, 65535, 65536, 100000].iter() {
            let src = mkdata(shape, len, &mut rng);
            let bound = compress_bound(len as i32);
            let targets: Vec<i32> = vec![
                -1,
                0,
                1,
                2,
                8,
                16,
                (len / 4) as i32,
                (len / 2) as i32,
                len as i32,
                bound - 1,
                bound,
                bound + 100,
            ];
            for &t in targets.iter() {
                diff(&format!("destSize {shape:?} len={len} t={t}"), |lib| {
                    let f = unsafe { sym::<FnDestSize>(lib, "LZ4_compress_destSize") };
                    let mut sp = len as i32;
                    let mut dst = vec![0xA5u8; (t.max(0) as usize) + 32];
                    let n = unsafe {
                        f(
                            if src.is_empty() { std::ptr::null() } else { src.as_ptr() },
                            dst.as_mut_ptr(),
                            &mut sp,
                            t,
                        )
                    };
                    dst.truncate(if n > 0 { n as usize } else { 0 });
                    (n, sp, dst)
                });
            }
        }
    }
    // randomized
    for i in 0..300 {
        let shape = ALL_SHAPES[i % ALL_SHAPES.len()];
        let len = rng.range(1, 30000);
        let src = mkdata(shape, len, &mut rng);
        let t = rng.range(0, compress_bound(len as i32) as usize + 10) as i32;
        diff(&format!("destSize rand #{i} len={len} t={t}"), |lib| {
            let f = unsafe { sym::<FnDestSize>(lib, "LZ4_compress_destSize") };
            let mut sp = len as i32;
            let mut dst = vec![0xA5u8; (t.max(0) as usize) + 32];
            let n = unsafe { f(src.as_ptr(), dst.as_mut_ptr(), &mut sp, t) };
            dst.truncate(if n > 0 { n as usize } else { 0 });
            (n, sp, dst)
        });
    }
}

/* ================================================================== */
/* rows 23,24 / errors 14–21 — LZ4_decompress_safe                     */
/* ================================================================== */

fn c_compress(src: &[u8]) -> Vec<u8> {
    let i = impls();
    let f = unsafe { sym::<FnCompress4>(&i.c, "LZ4_compress_default") };
    let bound = compress_bound(src.len() as i32).max(1);
    let mut dst = vec![0u8; bound as usize];
    let n = unsafe {
        f(
            if src.is_empty() { std::ptr::null() } else { src.as_ptr() },
            dst.as_mut_ptr(),
            src.len() as i32,
            bound,
        )
    };
    assert!(n > 0, "C compress failed for len {}", src.len());
    dst.truncate(n as usize);
    dst
}

#[test]
fn r023_decompress_safe_roundtrip() {
    let mut rng = Rng::new(0x5EED_0023);
    for &shape in ALL_SHAPES.iter() {
        for &len in [
            0usize, 1, 2, 3, 4, 5, 12, 13, 14, 15, 16, 17, 63, 64, 65, 1000, 65535, 65536, 150000,
        ]
        .iter()
        {
            let src = mkdata(shape, len, &mut rng);
            let comp = c_compress(&src);
            for slack in [0i32, 1, 16] {
                diff(&format!("dec_safe {shape:?} len={len} slack={slack}"), |lib| {
                    let f = unsafe { sym::<FnDecSafe>(lib, "LZ4_decompress_safe") };
                    let cap = len as i32 + slack;
                    let mut out = vec![0x5Au8; cap as usize + 8];
                    let n = unsafe {
                        f(comp.as_ptr(), out.as_mut_ptr(), comp.len() as i32, cap)
                    };
                    out.truncate(if n > 0 { n as usize } else { 0 });
                    (n, out)
                });
            }
            // dstCapacity one short
            if len > 0 {
                diff(&format!("dec_safe short {shape:?} len={len}"), |lib| {
                    let f = unsafe { sym::<FnDecSafe>(lib, "LZ4_decompress_safe") };
                    let mut out = vec![0x5Au8; len + 8];
                    unsafe { f(comp.as_ptr(), out.as_mut_ptr(), comp.len() as i32, len as i32 - 1) }
                });
            }
        }
    }
}

#[test]
fn e014_decompress_safe_bad_inputs() {
    let mut rng = Rng::new(0x5EED_1014);
    let src = mkdata(Shape::Textish, 5000, &mut rng);
    let comp = c_compress(&src);

    // null src
    diff("dec_safe null src", |lib| {
        let f = unsafe { sym::<FnDecSafe>(lib, "LZ4_decompress_safe") };
        let mut out = vec![0u8; 8192];
        unsafe { f(std::ptr::null(), out.as_mut_ptr(), 10, 8192) }
    });
    // negative dstCapacity
    for &neg in [-1i32, -1000, i32::MIN].iter() {
        diff(&format!("dec_safe neg cap {neg}"), |lib| {
            let f = unsafe { sym::<FnDecSafe>(lib, "LZ4_decompress_safe") };
            let mut out = vec![0u8; 8192];
            unsafe { f(comp.as_ptr(), out.as_mut_ptr(), comp.len() as i32, neg) }
        });
    }
    // zero / negative srcSize
    for &s in [0i32, -1, -1000].iter() {
        diff(&format!("dec_safe srcSize {s}"), |lib| {
            let f = unsafe { sym::<FnDecSafe>(lib, "LZ4_decompress_safe") };
            let mut out = vec![0u8; 8192];
            unsafe { f(comp.as_ptr(), out.as_mut_ptr(), s, 8192) }
        });
    }
    // truncations
    for cut in 1..=comp.len().min(64) {
        let t = &comp[..comp.len() - cut];
        diff(&format!("dec_safe trunc {cut}"), |lib| {
            let f = unsafe { sym::<FnDecSafe>(lib, "LZ4_decompress_safe") };
            let mut out = vec![0u8; src.len() + 64];
            let n = unsafe { f(t.as_ptr(), out.as_mut_ptr(), t.len() as i32, (src.len() + 64) as i32) };
            out.truncate(if n > 0 { n as usize } else { 0 });
            (n, out)
        });
    }
    // hand-built invalid streams
    let bad: Vec<Vec<u8>> = vec![
        vec![0x10, 0xAA, 0x00, 0x00],             // offset 0
        vec![0x10, 0xAA, 0xFF, 0xFF],             // offset way before start
        vec![0xF0, 0xFF, 0xFF, 0xFF, 0xFF],       // literal length overflow
        vec![0x0F, 0x01, 0x00, 0xFF, 0xFF, 0xFF], // match length overflow
        vec![0x00],                               // token only
        vec![0xFF],                               // 15 lits + 15 ml, nothing follows
        vec![0x1F, 0xAA, 0x01, 0x00, 0xFF, 0xFF, 0xFF, 0xFF],
        vec![0x40, 0xAA, 0xBB, 0xCC, 0xDD],       // 4 lits then EOF (LASTLITERALS rule)
        vec![0x50, 0x01, 0x02, 0x03, 0x04, 0x05], // 5 lits ending stream
    ];
    for (k, b) in bad.iter().enumerate() {
        for cap in [0i32, 1, 4, 16, 1024] {
            diff(&format!("dec_safe bad#{k} cap={cap}"), |lib| {
                let f = unsafe { sym::<FnDecSafe>(lib, "LZ4_decompress_safe") };
                let mut out = vec![0x5Au8; 4096];
                let n = unsafe { f(b.as_ptr(), out.as_mut_ptr(), b.len() as i32, cap) };
                out.truncate(if n > 0 { n as usize } else { 0 });
                (n, out)
            });
        }
    }
    // random fuzz — must agree bit-for-bit including partial output
    for i in 0..3000 {
        let l = rng.range(1, 80);
        let mut b = vec![0u8; l];
        for x in b.iter_mut() {
            *x = rng.byte();
        }
        let cap = rng.range(0, 300) as i32;
        diff(&format!("dec_safe fuzz #{i}"), |lib| {
            let f = unsafe { sym::<FnDecSafe>(lib, "LZ4_decompress_safe") };
            let mut out = vec![0x5Au8; 512];
            let n = unsafe { f(b.as_ptr(), out.as_mut_ptr(), l as i32, cap) };
            out.truncate(if n > 0 { n as usize } else { 0 });
            (n, out)
        });
    }
    // corrupted-valid fuzz: flip bytes in a real stream
    for i in 0..2000 {
        let mut b = comp.clone();
        let nflip = rng.range(1, 4);
        for _ in 0..nflip {
            let p = rng.below(b.len());
            b[p] = rng.byte();
        }
        let cap = src.len() as i32;
        diff(&format!("dec_safe corrupt #{i}"), |lib| {
            let f = unsafe { sym::<FnDecSafe>(lib, "LZ4_decompress_safe") };
            let mut out = vec![0x5Au8; src.len() + 64];
            let n = unsafe { f(b.as_ptr(), out.as_mut_ptr(), b.len() as i32, cap) };
            out.truncate(if n > 0 { n as usize } else { 0 });
            (n, out)
        });
    }
}

/* ================================================================== */
/* row 25 / errors 22–25 — LZ4_decompress_safe_partial                 */
/* ================================================================== */

#[test]
fn r025_decompress_safe_partial() {
    let mut rng = Rng::new(0x5EED_0025);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 5, 100, 4096, 65536].iter() {
            let src = mkdata(shape, len, &mut rng);
            let comp = c_compress(&src);
            let targets: Vec<i32> = vec![-1, 0, 1, (len / 3) as i32, (len / 2) as i32, len as i32, len as i32 + 1];
            let caps: Vec<i32> = vec![0, 1, (len / 2) as i32, len as i32, len as i32 + 16];
            for &t in targets.iter() {
                for &c in caps.iter() {
                    diff(
                        &format!("dec_partial {shape:?} len={len} t={t} c={c}"),
                        |lib| {
                            let f =
                                unsafe { sym::<FnDecPartial>(lib, "LZ4_decompress_safe_partial") };
                            let mut out = vec![0x5Au8; (c.max(0) as usize) + 64];
                            let n = unsafe {
                                f(comp.as_ptr(), out.as_mut_ptr(), comp.len() as i32, t, c)
                            };
                            out.truncate(if n > 0 { n as usize } else { 0 });
                            (n, out)
                        },
                    );
                }
            }
        }
    }
    // null src / negative cap
    diff("dec_partial null", |lib| {
        let f = unsafe { sym::<FnDecPartial>(lib, "LZ4_decompress_safe_partial") };
        let mut out = vec![0u8; 128];
        (
            unsafe { f(std::ptr::null(), out.as_mut_ptr(), 4, 4, 128) },
            unsafe { f(out.as_ptr(), out.as_mut_ptr(), 4, 4, -1) },
        )
    });
    // fuzz
    for i in 0..1500 {
        let l = rng.range(1, 60);
        let mut b = vec![0u8; l];
        for x in b.iter_mut() {
            *x = rng.byte();
        }
        let t = rng.range(0, 200) as i32;
        let c = rng.range(0, 200) as i32;
        diff(&format!("dec_partial fuzz #{i}"), |lib| {
            let f = unsafe { sym::<FnDecPartial>(lib, "LZ4_decompress_safe_partial") };
            let mut out = vec![0x5Au8; 300];
            let n = unsafe { f(b.as_ptr(), out.as_mut_ptr(), l as i32, t, c) };
            out.truncate(if n > 0 { n as usize } else { 0 });
            (n, out)
        });
    }
}

/* ================================================================== */
/* row 26 / errors 26,27 — LZ4_decompress_fast (deprecated)            */
/* ================================================================== */

#[test]
fn r026_decompress_fast() {
    let mut rng = Rng::new(0x5EED_0026);
    for &shape in ALL_SHAPES.iter() {
        for &len in [1usize, 5, 100, 4096, 65536].iter() {
            let src = mkdata(shape, len, &mut rng);
            let comp = c_compress(&src);
            diff(&format!("dec_fast {shape:?} len={len}"), |lib| {
                let f = unsafe { sym::<FnDecFast>(lib, "LZ4_decompress_fast") };
                let mut out = vec![0x5Au8; len + 64];
                let n = unsafe { f(comp.as_ptr(), out.as_mut_ptr(), len as i32) };
                out.truncate(len);
                (n, out)
            });
            // originalSize too small -> must both report the same failure
            diff(&format!("dec_fast short {shape:?} len={len}"), |lib| {
                let f = unsafe { sym::<FnDecFast>(lib, "LZ4_decompress_fast") };
                let mut out = vec![0x5Au8; len + 64];
                unsafe { f(comp.as_ptr(), out.as_mut_ptr(), (len / 2) as i32) }
            });
        }
    }
    // LZ4_uncompress / LZ4_uncompress_unknownOutputSize (row 38)
    let src = mkdata(Shape::Textish, 3000, &mut rng);
    let comp = c_compress(&src);
    diff("uncompress family", |lib| {
        let f1 = unsafe { sym::<FnDecFast>(lib, "LZ4_uncompress") };
        let f2 = unsafe { sym::<FnDecSafe>(lib, "LZ4_uncompress_unknownOutputSize") };
        let mut o1 = vec![0x5Au8; src.len() + 64];
        let a = unsafe { f1(comp.as_ptr(), o1.as_mut_ptr(), src.len() as i32) };
        let mut o2 = vec![0x5Au8; src.len() + 64];
        let b = unsafe {
            f2(
                comp.as_ptr(),
                o2.as_mut_ptr(),
                comp.len() as i32,
                (src.len() + 64) as i32,
            )
        };
        o1.truncate(src.len());
        o2.truncate(if b > 0 { b as usize } else { 0 });
        (a, o1, b, o2)
    });
}

/* ================================================================== */
/* rows 12, 28-33 / errors 28-32, 38 — stream lifecycle & init         */
/* ================================================================== */

#[test]
fn e028_initStream_guards() {
    diff("initStream guards", |lib| {
        let ss = sizeof_state(lib);
        let ini = unsafe { sym::<FnInitStream>(lib, "LZ4_initStream") };
        let mut buf = Aligned::new(ss + 64, 64);
        unsafe {
            let null_ok = ini(std::ptr::null_mut(), ss).is_null();
            let too_small = ini(buf.as_mut_ptr() as *mut CVoid, ss - 1).is_null();
            let zero = ini(buf.as_mut_ptr() as *mut CVoid, 0).is_null();
            let exact = ini(buf.as_mut_ptr() as *mut CVoid, ss).is_null();
            let big = ini(buf.as_mut_ptr() as *mut CVoid, ss + 64).is_null();
            // misaligned by 1..7
            let mut mis = Vec::new();
            for off in 1usize..8 {
                let p = buf.as_mut_ptr().add(off) as *mut CVoid;
                mis.push(ini(p, ss).is_null());
            }
            (null_ok, too_small, zero, exact, big, mis)
        }
    });
}

#[test]
fn e031_free_on_null() {
    diff("free on NULL", |lib| unsafe {
        (
            sym::<FnFreeStream>(lib, "LZ4_freeStream")(std::ptr::null_mut()),
            sym::<FnFreeStream>(lib, "LZ4_freeStreamDecode")(std::ptr::null_mut()),
            sym::<FnFreeStream>(lib, "LZ4_freeStreamHC")(std::ptr::null_mut()),
            sym::<FnFreeStream>(lib, "LZ4_freeHC")(std::ptr::null_mut()),
        )
    });
}

#[test]
fn r012_initStream_then_compress() {
    let mut rng = Rng::new(0x5EED_0012);
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 20000, &mut rng);
        diff(&format!("initStream+continue {shape:?}"), |lib| {
            let ss = sizeof_state(lib);
            let mut buf = Aligned::new(ss + 64, 64);
            unsafe {
                let s = sym::<FnInitStream>(lib, "LZ4_initStream")(
                    buf.as_mut_ptr() as *mut CVoid,
                    ss + 64,
                );
                assert!(!s.is_null());
                let f = sym::<FnContinue>(lib, "LZ4_compress_fast_continue");
                let mut all = Vec::new();
                let mut off = 0usize;
                while off < src.len() {
                    let n = (src.len() - off).min(4000);
                    let bound = compress_bound(n as i32);
                    let mut dst = vec![0u8; bound as usize];
                    let r = f(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound, 1);
                    all.push(r);
                    dst.truncate(if r > 0 { r as usize } else { 0 });
                    all.extend(dst.iter().map(|&b| b as i32));
                    off += n;
                }
                all
            }
        });
    }
}

/* ================================================================== */
/* rows 13,14,20,21 — prefix-mode streaming                            */
/* ================================================================== */

fn stream_prefix(lib: &Library, src: &[u8], chunks: &[usize], fast_reset: bool) -> Vec<i32> {
    unsafe {
        let cs = sym::<FnCreateStream>(lib, "LZ4_createStream");
        let s = cs();
        assert!(!s.is_null());
        if fast_reset {
            sym::<FnVoidPtr>(lib, "LZ4_resetStream_fast")(s);
        } else {
            sym::<FnVoidPtr>(lib, "LZ4_resetStream")(s);
        }
        let f = sym::<FnContinue>(lib, "LZ4_compress_fast_continue");
        let mut out = Vec::new();
        let mut off = 0usize;
        for &c in chunks {
            if off >= src.len() {
                break;
            }
            let n = c.min(src.len() - off);
            let bound = compress_bound(n as i32).max(1);
            let mut dst = vec![0u8; bound as usize];
            let r = f(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound, 1);
            out.push(r);
            dst.truncate(if r > 0 { r as usize } else { 0 });
            out.extend(dst.iter().map(|&b| b as i32));
            off += n;
        }
        sym::<FnFreeStream>(lib, "LZ4_freeStream")(s);
        out
    }
}

#[test]
fn r013_r014_prefix_streaming() {
    let mut rng = Rng::new(0x5EED_0013);
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 250000, &mut rng);
        for fr in [false, true] {
            for pattern in 0..4 {
                let chunks: Vec<usize> = match pattern {
                    0 => vec![1; 200],
                    1 => vec![4096; 40],
                    2 => vec![65536; 4],
                    _ => (0..60).map(|_| rng.range(1, 20000)).collect(),
                };
                diff(
                    &format!("prefix {shape:?} fr={fr} pat={pattern}"),
                    |lib| stream_prefix(lib, &src, &chunks, fr),
                );
            }
        }
    }
}

/* ================================================================== */
/* rows 15,16,17,18,19,22 — dictionary modes                           */
/* ================================================================== */

const DICT_SIZES: [usize; 10] = [0, 1, 3, 4, 5, 64, 4096, 65535, 65536, 70000];

#[test]
fn r015_r016_loadDict_extDict() {
    let mut rng = Rng::new(0x5EED_0015);
    for slow in [false, true] {
        for &ds in DICT_SIZES.iter() {
            for &shape in ALL_SHAPES.iter() {
                let dict = mkdata(shape, ds, &mut rng);
                let src = mkdata(shape, 30000, &mut rng);
                let name = if slow { "LZ4_loadDictSlow" } else { "LZ4_loadDict" };
                diff(&format!("{name} ds={ds} {shape:?}"), |lib| {
                    unsafe {
                        let s = sym::<FnCreateStream>(lib, "LZ4_createStream")();
                        let ld = sym::<FnLoadDict>(lib, name);
                        let loaded = ld(
                            s,
                            if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() },
                            ds as i32,
                        );
                        let f = sym::<FnContinue>(lib, "LZ4_compress_fast_continue");
                        let mut out = vec![loaded];
                        // two chunks so the 2nd goes through the ext-dict path again
                        let mut off = 0usize;
                        for cl in [7000usize, 23000] {
                            let n = cl.min(src.len() - off);
                            let bound = compress_bound(n as i32);
                            let mut dst = vec![0u8; bound as usize];
                            let r =
                                f(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound, 1);
                            out.push(r);
                            dst.truncate(if r > 0 { r as usize } else { 0 });
                            out.extend(dst.iter().map(|&b| b as i32));
                            off += n;
                        }
                        sym::<FnFreeStream>(lib, "LZ4_freeStream")(s);
                        out
                    }
                });
            }
        }
    }
}

#[test]
fn r017_r018_attach_dictionary() {
    let mut rng = Rng::new(0x5EED_0017);
    for &ds in [0usize, 4, 1000, 65536].iter() {
        for &inlen in [100usize, 4096, 4097, 40000].iter() {
            for detach in [false, true] {
                let dict = mkdata(Shape::Textish, ds, &mut rng);
                let src = mkdata(Shape::Textish, inlen, &mut rng);
                diff(
                    &format!("attach ds={ds} in={inlen} detach={detach}"),
                    |lib| unsafe {
                        let dstream = sym::<FnCreateStream>(lib, "LZ4_createStream")();
                        let wstream = sym::<FnCreateStream>(lib, "LZ4_createStream")();
                        let loaded = sym::<FnLoadDict>(lib, "LZ4_loadDict")(
                            dstream,
                            if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() },
                            ds as i32,
                        );
                        sym::<FnVoidPtr>(lib, "LZ4_resetStream_fast")(wstream);
                        let at = sym::<FnAttach>(lib, "LZ4_attach_dictionary");
                        if detach {
                            at(wstream, std::ptr::null());
                        } else {
                            at(wstream, dstream as *const CVoid);
                        }
                        let f = sym::<FnContinue>(lib, "LZ4_compress_fast_continue");
                        let bound = compress_bound(inlen as i32);
                        let mut dst = vec![0u8; bound as usize];
                        let r = f(
                            wstream,
                            src.as_ptr(),
                            dst.as_mut_ptr(),
                            inlen as i32,
                            bound,
                            1,
                        );
                        dst.truncate(if r > 0 { r as usize } else { 0 });
                        sym::<FnFreeStream>(lib, "LZ4_freeStream")(dstream);
                        sym::<FnFreeStream>(lib, "LZ4_freeStream")(wstream);
                        (loaded, r, dst)
                    },
                );
            }
        }
    }
}

#[test]
fn r019_saveDict() {
    let mut rng = Rng::new(0x5EED_0019);
    for &md in [-1i32, 0, 1, 4, 1000, 65535, 65536, 70000].iter() {
        for &shape in ALL_SHAPES.iter() {
            let src = mkdata(shape, 90000, &mut rng);
            diff(&format!("saveDict md={md} {shape:?}"), |lib| unsafe {
                let s = sym::<FnCreateStream>(lib, "LZ4_createStream")();
                sym::<FnVoidPtr>(lib, "LZ4_resetStream")(s);
                let f = sym::<FnContinue>(lib, "LZ4_compress_fast_continue");
                let mut out = Vec::new();
                let mut safebuf = vec![0u8; 80000];
                let mut off = 0usize;
                for _ in 0..3 {
                    let n = 25000usize.min(src.len() - off);
                    let bound = compress_bound(n as i32);
                    let mut dst = vec![0u8; bound as usize];
                    let r = f(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound, 1);
                    out.push(r);
                    dst.truncate(if r > 0 { r as usize } else { 0 });
                    out.extend(dst.iter().map(|&b| b as i32));
                    off += n;
                    let sd = sym::<FnSaveDict>(lib, "LZ4_saveDict")(s, safebuf.as_mut_ptr(), md);
                    out.push(sd);
                    if sd > 0 {
                        out.extend(safebuf[..sd as usize].iter().map(|&b| b as i32));
                    }
                }
                sym::<FnFreeStream>(lib, "LZ4_freeStream")(s);
                out
            });
        }
    }
    // saveDict with NULL buffer and dictSize 0
    diff("saveDict null", |lib| unsafe {
        let s = sym::<FnCreateStream>(lib, "LZ4_createStream")();
        sym::<FnVoidPtr>(lib, "LZ4_resetStream")(s);
        let r = sym::<FnSaveDict>(lib, "LZ4_saveDict")(s, std::ptr::null_mut(), 0);
        sym::<FnFreeStream>(lib, "LZ4_freeStream")(s);
        r
    });
}

#[test]
fn r020_r021_overlapping_and_tiny_dict() {
    let mut rng = Rng::new(0x5EED_0020);
    // ring-buffer style: source moves within a single buffer that also holds the dict
    for &shape in ALL_SHAPES.iter() {
        let buf = mkdata(shape, 200000, &mut rng);
        diff(&format!("ringbuf {shape:?}"), |lib| unsafe {
            let s = sym::<FnCreateStream>(lib, "LZ4_createStream")();
            sym::<FnVoidPtr>(lib, "LZ4_resetStream")(s);
            let f = sym::<FnContinue>(lib, "LZ4_compress_fast_continue");
            let mut out = Vec::new();
            // deliberately overlap: feed windows that step back
            let steps = [0usize, 3000, 5000, 4000, 12000, 11000, 30000];
            for &st in steps.iter() {
                let n = 6000usize.min(buf.len() - st);
                let bound = compress_bound(n as i32);
                let mut dst = vec![0u8; bound as usize];
                let r = f(s, buf[st..].as_ptr(), dst.as_mut_ptr(), n as i32, bound, 1);
                out.push(r);
                dst.truncate(if r > 0 { r as usize } else { 0 });
                out.extend(dst.iter().map(|&b| b as i32));
            }
            sym::<FnFreeStream>(lib, "LZ4_freeStream")(s);
            out
        });
    }
    // tiny dict invalidation branch (dictSize<4 and dictEnd != source)
    for &ds in [1usize, 2, 3].iter() {
        let dict = mkdata(Shape::Random, ds, &mut rng);
        let src = mkdata(Shape::Textish, 5000, &mut rng);
        diff(&format!("tiny dict ds={ds}"), |lib| unsafe {
            let s = sym::<FnCreateStream>(lib, "LZ4_createStream")();
            sym::<FnLoadDict>(lib, "LZ4_loadDict")(s, dict.as_ptr(), ds as i32);
            let f = sym::<FnContinue>(lib, "LZ4_compress_fast_continue");
            let bound = compress_bound(src.len() as i32);
            let mut dst = vec![0u8; bound as usize];
            let r = f(s, src.as_ptr(), dst.as_mut_ptr(), src.len() as i32, bound, 1);
            dst.truncate(if r > 0 { r as usize } else { 0 });
            sym::<FnFreeStream>(lib, "LZ4_freeStream")(s);
            (r, dst)
        });
    }
}

#[test]
fn r022_compress_forceExtDict() {
    let mut rng = Rng::new(0x5EED_0022);
    for &shape in ALL_SHAPES.iter() {
        for &ds in [0usize, 4, 1000, 65536].iter() {
            let dict = mkdata(shape, ds, &mut rng);
            let src = mkdata(shape, 30000, &mut rng);
            diff(&format!("forceExtDict {shape:?} ds={ds}"), |lib| unsafe {
                let s = sym::<FnCreateStream>(lib, "LZ4_createStream")();
                sym::<FnLoadDict>(lib, "LZ4_loadDict")(
                    s,
                    if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() },
                    ds as i32,
                );
                let f = sym::<FnCompressWithState4>(lib, "LZ4_compress_forceExtDict");
                let bound = compress_bound(src.len() as i32);
                let mut dst = vec![0u8; bound as usize];
                let r = f(s, src.as_ptr(), dst.as_mut_ptr(), src.len() as i32);
                dst.truncate(if r > 0 { r as usize } else { 0 });
                sym::<FnFreeStream>(lib, "LZ4_freeStream")(s);
                (r, dst)
            });
        }
    }
}

/* ================================================================== */
/* rows 27,28,29 — streaming decode                                    */
/* ================================================================== */

#[test]
fn r027_r029_decode_continue() {
    let mut rng = Rng::new(0x5EED_0027);
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 120000, &mut rng);
        let chunk = 7000usize;
        // build the compressed chunks with the C library
        let i = impls();
        let blocks: Vec<(usize, Vec<u8>)> = unsafe {
            let s = sym::<FnCreateStream>(&i.c, "LZ4_createStream")();
            sym::<FnVoidPtr>(&i.c, "LZ4_resetStream")(s);
            let f = sym::<FnContinue>(&i.c, "LZ4_compress_fast_continue");
            let mut v = Vec::new();
            let mut off = 0usize;
            while off < src.len() {
                let n = chunk.min(src.len() - off);
                let bound = compress_bound(n as i32);
                let mut dst = vec![0u8; bound as usize];
                let r = f(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound, 1);
                assert!(r > 0);
                dst.truncate(r as usize);
                v.push((n, dst));
                off += n;
            }
            sym::<FnFreeStream>(&i.c, "LZ4_freeStream")(s);
            v
        };
        diff(&format!("dec_safe_continue {shape:?}"), |lib| unsafe {
            let sd = sym::<FnCreateStream>(lib, "LZ4_createStreamDecode")();
            let f = sym::<FnDecContinue>(lib, "LZ4_decompress_safe_continue");
            let mut out = vec![0u8; src.len() + 64];
            let mut off = 0usize;
            let mut codes = Vec::new();
            for (orig, blk) in blocks.iter() {
                let r = f(
                    sd,
                    blk.as_ptr(),
                    out[off..].as_mut_ptr(),
                    blk.len() as i32,
                    (out.len() - off) as i32,
                );
                codes.push(r);
                if r <= 0 {
                    break;
                }
                off += r as usize;
                assert_eq!(r as usize, *orig);
            }
            sym::<FnFreeStream>(lib, "LZ4_freeStreamDecode")(sd);
            out.truncate(off);
            (codes, out)
        });
        diff(&format!("dec_fast_continue {shape:?}"), |lib| unsafe {
            let sd = sym::<FnCreateStream>(lib, "LZ4_createStreamDecode")();
            let f = sym::<FnDecFastContinue>(lib, "LZ4_decompress_fast_continue");
            let mut out = vec![0u8; src.len() + 64];
            let mut off = 0usize;
            let mut codes = Vec::new();
            for (orig, blk) in blocks.iter() {
                let r = f(sd, blk.as_ptr(), out[off..].as_mut_ptr(), *orig as i32);
                codes.push(r);
                if r < 0 {
                    break;
                }
                off += *orig;
            }
            sym::<FnFreeStream>(lib, "LZ4_freeStreamDecode")(sd);
            out.truncate(off);
            (codes, out)
        });
    }
}

#[test]
fn r028_setStreamDecode() {
    let mut rng = Rng::new(0x5EED_0028);
    for &ds in DICT_SIZES.iter() {
        let dict = mkdata(Shape::Textish, ds, &mut rng);
        let src = mkdata(Shape::Textish, 20000, &mut rng);
        // compress with the same dict using C
        let i = impls();
        let comp: Vec<u8> = unsafe {
            let s = sym::<FnCreateStream>(&i.c, "LZ4_createStream")();
            sym::<FnLoadDict>(&i.c, "LZ4_loadDict")(
                s,
                if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() },
                ds as i32,
            );
            let f = sym::<FnContinue>(&i.c, "LZ4_compress_fast_continue");
            let bound = compress_bound(src.len() as i32);
            let mut dst = vec![0u8; bound as usize];
            let r = f(s, src.as_ptr(), dst.as_mut_ptr(), src.len() as i32, bound, 1);
            assert!(r > 0);
            dst.truncate(r as usize);
            sym::<FnFreeStream>(&i.c, "LZ4_freeStream")(s);
            dst
        };
        diff(&format!("setStreamDecode ds={ds}"), |lib| unsafe {
            let sd = sym::<FnCreateStream>(lib, "LZ4_createStreamDecode")();
            let ok = sym::<FnSetStreamDecode>(lib, "LZ4_setStreamDecode")(
                sd,
                if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() },
                ds as i32,
            );
            let f = sym::<FnDecContinue>(lib, "LZ4_decompress_safe_continue");
            let mut out = vec![0u8; src.len() + 64];
            let r = f(
                sd,
                comp.as_ptr(),
                out.as_mut_ptr(),
                comp.len() as i32,
                out.len() as i32,
            );
            sym::<FnFreeStream>(lib, "LZ4_freeStreamDecode")(sd);
            out.truncate(if r > 0 { r as usize } else { 0 });
            (ok, r, out)
        });
    }
}

/* ================================================================== */
/* rows 30,31,32,33 — usingDict decoders                               */
/* ================================================================== */

#[test]
fn r030_r032_usingDict_decoders() {
    let mut rng = Rng::new(0x5EED_0030);
    for &ds in DICT_SIZES.iter() {
        for &shape in ALL_SHAPES.iter() {
            let dict = mkdata(shape, ds, &mut rng);
            let src = mkdata(shape, 12000, &mut rng);
            let i = impls();
            let comp: Vec<u8> = unsafe {
                let s = sym::<FnCreateStream>(&i.c, "LZ4_createStream")();
                sym::<FnLoadDict>(&i.c, "LZ4_loadDict")(
                    s,
                    if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() },
                    ds as i32,
                );
                let f = sym::<FnContinue>(&i.c, "LZ4_compress_fast_continue");
                let bound = compress_bound(src.len() as i32);
                let mut dst = vec![0u8; bound as usize];
                let r = f(s, src.as_ptr(), dst.as_mut_ptr(), src.len() as i32, bound, 1);
                assert!(r > 0);
                dst.truncate(r as usize);
                sym::<FnFreeStream>(&i.c, "LZ4_freeStream")(s);
                dst
            };
            let dp = if dict.is_empty() {
                std::ptr::null()
            } else {
                dict.as_ptr()
            };
            diff(&format!("safe_usingDict ds={ds} {shape:?}"), |lib| unsafe {
                let f = sym::<FnDecUsingDict>(lib, "LZ4_decompress_safe_usingDict");
                let mut out = vec![0x5Au8; src.len() + 64];
                let r = f(
                    comp.as_ptr(),
                    out.as_mut_ptr(),
                    comp.len() as i32,
                    (src.len() + 64) as i32,
                    dp,
                    ds as i32,
                );
                out.truncate(if r > 0 { r as usize } else { 0 });
                (r, out)
            });
            diff(&format!("fast_usingDict ds={ds} {shape:?}"), |lib| unsafe {
                let f = sym::<unsafe extern "C" fn(*const u8, *mut u8, i32, *const u8, i32) -> i32>(
                    lib,
                    "LZ4_decompress_fast_usingDict",
                );
                let mut out = vec![0x5Au8; src.len() + 64];
                let r = f(
                    comp.as_ptr(),
                    out.as_mut_ptr(),
                    src.len() as i32,
                    dp,
                    ds as i32,
                );
                out.truncate(src.len());
                (r, out)
            });
            for &t in [0i32, 1, (src.len() / 2) as i32, src.len() as i32].iter() {
                diff(
                    &format!("partial_usingDict ds={ds} {shape:?} t={t}"),
                    |lib| unsafe {
                        let f = sym::<FnDecPartialUsingDict>(
                            lib,
                            "LZ4_decompress_safe_partial_usingDict",
                        );
                        let mut out = vec![0x5Au8; src.len() + 64];
                        let r = f(
                            comp.as_ptr(),
                            out.as_mut_ptr(),
                            comp.len() as i32,
                            t,
                            (src.len() + 64) as i32,
                            dp,
                            ds as i32,
                        );
                        out.truncate(if r > 0 { r as usize } else { 0 });
                        (r, out)
                    },
                );
            }
            // corrupt variants
            if !comp.is_empty() {
                for k in 0..8 {
                    let mut b = comp.clone();
                    let p = rng.below(b.len());
                    b[p] = rng.byte();
                    diff(
                        &format!("safe_usingDict corrupt ds={ds} {shape:?} #{k}"),
                        |lib| unsafe {
                            let f = sym::<FnDecUsingDict>(lib, "LZ4_decompress_safe_usingDict");
                            let mut out = vec![0x5Au8; src.len() + 64];
                            let r = f(
                                b.as_ptr(),
                                out.as_mut_ptr(),
                                b.len() as i32,
                                (src.len() + 64) as i32,
                                dp,
                                ds as i32,
                            );
                            out.truncate(if r > 0 { r as usize } else { 0 });
                            (r, out)
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn r033_withPrefix64k() {
    let mut rng = Rng::new(0x5EED_0033);
    // A single buffer: [64 KB prefix][data]. Compress the data with the prefix
    // as history, then decode with the *_withPrefix64k entry points.
    for &shape in ALL_SHAPES.iter() {
        let total = 65536 + 20000;
        let buf = mkdata(shape, total, &mut rng);
        let i = impls();
        let comp: Vec<u8> = unsafe {
            let s = sym::<FnCreateStream>(&i.c, "LZ4_createStream")();
            sym::<FnLoadDict>(&i.c, "LZ4_loadDict")(s, buf.as_ptr(), 65536);
            let f = sym::<FnContinue>(&i.c, "LZ4_compress_fast_continue");
            let n = total - 65536;
            let bound = compress_bound(n as i32);
            let mut dst = vec![0u8; bound as usize];
            let r = f(s, buf[65536..].as_ptr(), dst.as_mut_ptr(), n as i32, bound, 1);
            assert!(r > 0);
            dst.truncate(r as usize);
            sym::<FnFreeStream>(&i.c, "LZ4_freeStream")(s);
            dst
        };
        let n = total - 65536;
        diff(&format!("withPrefix64k {shape:?}"), |lib| unsafe {
            let mut out = vec![0u8; total + 64];
            out[..65536].copy_from_slice(&buf[..65536]);
            let f = sym::<FnDecSafe>(lib, "LZ4_decompress_safe_withPrefix64k");
            let r = f(
                comp.as_ptr(),
                out[65536..].as_mut_ptr(),
                comp.len() as i32,
                (n + 64) as i32,
            );
            let mut out2 = vec![0u8; total + 64];
            out2[..65536].copy_from_slice(&buf[..65536]);
            let g = sym::<FnDecFast>(lib, "LZ4_decompress_fast_withPrefix64k");
            let r2 = g(comp.as_ptr(), out2[65536..].as_mut_ptr(), n as i32);
            (
                r,
                out[65536..65536 + if r > 0 { r as usize } else { 0 }].to_vec(),
                r2,
                out2[65536..65536 + n].to_vec(),
            )
        });
    }
}

/* ================================================================== */
/* row 34 / errors 39,40,41 — LZ4_decoderRingBufferSize                */
/* ================================================================== */

#[test]
fn r034_decoderRingBufferSize() {
    let cases: Vec<i32> = vec![
        i32::MIN,
        -1000,
        -1,
        0,
        1,
        2,
        15,
        16,
        17,
        64,
        65535,
        65536,
        4 << 20,
        LZ4_MAX_INPUT_SIZE,
        LZ4_MAX_INPUT_SIZE + 1,
        i32::MAX,
    ];
    diff("decoderRingBufferSize", |lib| {
        let f = unsafe { sym::<FnI32I32>(lib, "LZ4_decoderRingBufferSize") };
        cases.iter().map(|&c| unsafe { f(c) }).collect::<Vec<i32>>()
    });
}

/* ================================================================== */
/* rows 35,36,37,39 / errors 46,47 — deprecated block API              */
/* ================================================================== */

#[test]
fn r035_r037_deprecated_block_api() {
    let mut rng = Rng::new(0x5EED_0035);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 100, 5000, 65536, 100000].iter() {
            let src = mkdata(shape, len, &mut rng);
            let bound = compress_bound(len as i32).max(1);
            diff(&format!("deprecated oneshot {shape:?} len={len}"), |lib| unsafe {
                let mut r = Vec::new();
                let mut d1 = vec![0u8; bound as usize + 8];
                let a = sym::<FnCompress3>(lib, "LZ4_compress")(
                    if src.is_empty() { std::ptr::null() } else { src.as_ptr() },
                    d1.as_mut_ptr(),
                    len as i32,
                );
                d1.truncate(if a > 0 { a as usize } else { 0 });
                r.push((a, d1));
                for cap in [1i32, bound / 2, bound] {
                    let mut d2 = vec![0u8; bound as usize + 8];
                    let b = sym::<FnCompress4>(lib, "LZ4_compress_limitedOutput")(
                        if src.is_empty() { std::ptr::null() } else { src.as_ptr() },
                        d2.as_mut_ptr(),
                        len as i32,
                        cap,
                    );
                    d2.truncate(if b > 0 { b as usize } else { 0 });
                    r.push((b, d2));
                }
                r
            });
            diff(&format!("deprecated withState {shape:?} len={len}"), |lib| unsafe {
                let ss = sizeof_state(lib);
                let mut st = Aligned::new(ss + STATE_SLOP, 16);
                let mut r = Vec::new();
                let mut d1 = vec![0u8; bound as usize + 8];
                let a = sym::<FnCompressWithState4>(lib, "LZ4_compress_withState")(
                    st.as_mut_ptr() as *mut CVoid,
                    if src.is_empty() { std::ptr::null() } else { src.as_ptr() },
                    d1.as_mut_ptr(),
                    len as i32,
                );
                d1.truncate(if a > 0 { a as usize } else { 0 });
                r.push((a, d1));
                for cap in [1i32, bound / 2, bound] {
                    let mut d2 = vec![0u8; bound as usize + 8];
                    let b = sym::<FnCompressWithState5>(
                        lib,
                        "LZ4_compress_limitedOutput_withState",
                    )(
                        st.as_mut_ptr() as *mut CVoid,
                        if src.is_empty() { std::ptr::null() } else { src.as_ptr() },
                        d2.as_mut_ptr(),
                        len as i32,
                        cap,
                    );
                    d2.truncate(if b > 0 { b as usize } else { 0 });
                    r.push((b, d2));
                }
                r
            });
        }
    }
    // streaming deprecated
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 60000, &mut rng);
        diff(&format!("deprecated continue {shape:?}"), |lib| unsafe {
            let s = sym::<FnCreateStream>(lib, "LZ4_createStream")();
            sym::<FnVoidPtr>(lib, "LZ4_resetStream")(s);
            let f = sym::<FnCompressWithState4>(lib, "LZ4_compress_continue");
            let g = sym::<FnCompressWithState5>(lib, "LZ4_compress_limitedOutput_continue");
            let mut out = Vec::new();
            let mut off = 0usize;
            let mut k = 0;
            while off < src.len() {
                let n = 5000usize.min(src.len() - off);
                let bound = compress_bound(n as i32);
                let mut dst = vec![0u8; bound as usize];
                let r = if k % 2 == 0 {
                    f(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32)
                } else {
                    g(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound)
                };
                out.push(r);
                dst.truncate(if r > 0 { r as usize } else { 0 });
                out.extend(dst.iter().map(|&b| b as i32));
                off += n;
                k += 1;
            }
            sym::<FnFreeStream>(lib, "LZ4_freeStream")(s);
            out
        });
    }
}

#[test]
fn r039_legacy_create_and_resetStreamState() {
    let mut rng = Rng::new(0x5EED_0039);
    let src = mkdata(Shape::Textish, 40000, &mut rng);
    diff("LZ4_create + slideInputBuffer", |lib| unsafe {
        let mut inbuf = vec![0u8; 200000];
        let st = sym::<FnCreate>(lib, "LZ4_create")(inbuf.as_mut_ptr());
        let isnull = st.is_null();
        let mut out = Vec::new();
        if !isnull {
            let f = sym::<FnCompressWithState4>(lib, "LZ4_compress_continue");
            let mut off = 0usize;
            while off < src.len() {
                let n = 8000usize.min(src.len() - off);
                let bound = compress_bound(n as i32);
                let mut dst = vec![0u8; bound as usize];
                let r = f(st, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32);
                out.push(r);
                dst.truncate(if r > 0 { r as usize } else { 0 });
                out.extend(dst.iter().map(|&b| b as i32));
                off += n;
            }
            let p = sym::<FnSlide>(lib, "LZ4_slideInputBufferHC");
            let _ = p; // exercised in the HC test
            sym::<FnFreeStream>(lib, "LZ4_freeStream")(st);
        }
        (isnull, out)
    });
    diff("LZ4_resetStreamState", |lib| unsafe {
        let ss = sizeof_state(lib);
        let mut st = Aligned::new(ss + 64, 64);
        let f = sym::<FnResetStreamState>(lib, "LZ4_resetStreamState");
        let mut inbuf = vec![0u8; 16];
        // NOTE: the C implementation is just `LZ4_resetStream(state); return 0;`
        // — it performs no NULL / alignment validation, so a NULL or misaligned
        // `state` is undefined behaviour (it faults in the C too) and is not a
        // testable rejection. Only the defined path is compared here.
        let ok = f(st.as_mut_ptr() as *mut CVoid, inbuf.as_mut_ptr());
        let ok2 = f(st.as_mut_ptr() as *mut CVoid, std::ptr::null_mut());
        // usable afterwards
        let src = vec![7u8; 500];
        let bound = compress_bound(500);
        let mut dst = vec![0u8; bound as usize];
        let n = sym::<FnCompressWithState4>(lib, "LZ4_compress_continue")(
            st.as_mut_ptr() as *mut CVoid,
            src.as_ptr(),
            dst.as_mut_ptr(),
            500,
        );
        dst.truncate(if n > 0 { n as usize } else { 0 });
        (ok, ok2, n, dst)
    });
    diff("LZ4_slideInputBuffer", |lib| unsafe {
        let ss = sizeof_state(lib);
        let mut st = Aligned::new(ss + 64, 64);
        sym::<FnInitStream>(lib, "LZ4_initStream")(st.as_mut_ptr() as *mut CVoid, ss);
        let p = sym::<FnSlide>(lib, "LZ4_slideInputBuffer")(st.as_mut_ptr() as *mut CVoid);
        p.is_null()
    });
}

/* ================================================================== */
/* row 45 — LZ4_compress_fast_continue with an insufficient dst budget */
/* ================================================================== */

#[test]
fn e045_continue_tight_dst() {
    let mut rng = Rng::new(0x5EED_1045);
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 60000, &mut rng);
        // Establish the exact size the first chunk needs, then squeeze it.
        let chunk = 9000usize;
        let exact = {
            let i = impls();
            unsafe {
                let s = sym::<FnCreateStream>(&i.c, "LZ4_createStream")();
                sym::<FnVoidPtr>(&i.c, "LZ4_resetStream")(s);
                let bound = compress_bound(chunk as i32);
                let mut d = vec![0u8; bound as usize];
                let n = sym::<FnContinue>(&i.c, "LZ4_compress_fast_continue")(
                    s,
                    src.as_ptr(),
                    d.as_mut_ptr(),
                    chunk as i32,
                    bound,
                    1,
                );
                sym::<FnFreeStream>(&i.c, "LZ4_freeStream")(s);
                n
            }
        };
        for cap in [0i32, 1, 2, 8, exact / 4, exact / 2, exact - 1, exact] {
            diff(&format!("continue tight {shape:?} cap={cap}"), |lib| unsafe {
                let s = sym::<FnCreateStream>(lib, "LZ4_createStream")();
                sym::<FnVoidPtr>(lib, "LZ4_resetStream")(s);
                let f = sym::<FnContinue>(lib, "LZ4_compress_fast_continue");
                let mut out = Vec::new();
                let mut off = 0usize;
                // several chunks so the failure happens mid-stream and the
                // subsequent calls see the resulting state
                for _ in 0..5 {
                    let n = chunk.min(src.len() - off);
                    let mut d = vec![0u8; (cap.max(0) as usize) + 32];
                    let r = f(s, src[off..].as_ptr(), d.as_mut_ptr(), n as i32, cap, 1);
                    out.push(r);
                    d.truncate(if r > 0 { r as usize } else { 0 });
                    out.extend(d.iter().map(|&b| b as i32));
                    off += n;
                }
                sym::<FnFreeStream>(lib, "LZ4_freeStream")(s);
                out
            });
        }
        // oversized / negative srcSize
        for bad in [-1i32, LZ4_MAX_INPUT_SIZE + 1, i32::MAX] {
            diff(&format!("continue bad srcSize {shape:?} {bad}"), |lib| unsafe {
                let s = sym::<FnCreateStream>(lib, "LZ4_createStream")();
                sym::<FnVoidPtr>(lib, "LZ4_resetStream")(s);
                let mut d = vec![0u8; 1 << 16];
                let r = sym::<FnContinue>(lib, "LZ4_compress_fast_continue")(
                    s,
                    src.as_ptr(),
                    d.as_mut_ptr(),
                    bad,
                    d.len() as i32,
                    1,
                );
                sym::<FnFreeStream>(lib, "LZ4_freeStream")(s);
                r
            });
        }
    }
}
