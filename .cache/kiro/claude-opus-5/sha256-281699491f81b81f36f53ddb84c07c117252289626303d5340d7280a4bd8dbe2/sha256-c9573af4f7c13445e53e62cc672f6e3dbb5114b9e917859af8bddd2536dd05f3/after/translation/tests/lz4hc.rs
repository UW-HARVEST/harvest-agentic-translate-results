//! Phase B/C — high-compression API (`lz4hc.c`).
//! CONFIGS.md rows 40–56, ERRORS.md rows 48–72.
#![allow(non_snake_case)]

mod common;
use common::*;
use libloading::Library;

type FnHC5 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnHCState6 = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnHCDestSize =
    unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, *mut i32, i32, i32) -> i32;
type FnCreate0 = unsafe extern "C" fn() -> *mut CVoid;
type FnFree1 = unsafe extern "C" fn(*mut CVoid) -> i32;
type FnResetLevel = unsafe extern "C" fn(*mut CVoid, i32);
type FnLoadDict = unsafe extern "C" fn(*mut CVoid, *const u8, i32) -> i32;
type FnHCContinue = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32, i32) -> i32;
type FnHCContinueDestSize =
    unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, *mut i32, i32) -> i32;
type FnSaveDict = unsafe extern "C" fn(*mut CVoid, *mut u8, i32) -> i32;
type FnAttach = unsafe extern "C" fn(*mut CVoid, *const CVoid);
type FnInitHC = unsafe extern "C" fn(*mut CVoid, usize) -> *mut CVoid;
type FnDecSafe = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnHC3 = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
type FnHC4 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnHCSt4 = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32) -> i32;
type FnHCSt5 = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32, i32) -> i32;
type FnHCSt6 = unsafe extern "C" fn(*mut CVoid, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnCreateHC = unsafe extern "C" fn(*const u8) -> *mut CVoid;
type FnSlideHC = unsafe extern "C" fn(*mut CVoid) -> *mut u8;
type FnResetStateHC = unsafe extern "C" fn(*mut CVoid, *mut u8) -> i32;

/// Every distinct compression-level branch, plus the out-of-range values the
/// C clamps (`<1` → 9, `>12` → 12).
const LEVELS: [i32; 15] = [
    i32::MIN,
    -100,
    -1,
    0,
    1,
    2,
    3,
    4,
    6,
    9,
    10,
    11,
    12,
    13,
    99,
];

fn sizeof_stateHC(lib: &Library) -> usize {
    unsafe { sym::<FnVoidI32>(lib, "LZ4_sizeofStateHC")() as usize }
}

/* ================================================================== */
/* rows 40,41 / errors 48–51 — LZ4_compress_HC                        */
/* ================================================================== */

#[test]
fn r040_compress_HC_levels() {
    let mut rng = Rng::new(0x5EED_0040);
    for &shape in ALL_SHAPES.iter() {
        for &len in [
            0usize, 1, 2, 3, 4, 5, 12, 15, 16, 63, 64, 65, 1000, 65535, 65536, 65537, 150000,
        ]
        .iter()
        {
            let src = mkdata(shape, len, &mut rng);
            for &lvl in LEVELS.iter() {
                let bound = compress_bound(len as i32).max(1);
                diff(&format!("HC {shape:?} len={len} lvl={lvl}"), |lib| {
                    let f = unsafe { sym::<FnHC5>(lib, "LZ4_compress_HC") };
                    let mut dst = vec![0xA5u8; bound as usize + 8];
                    let n = unsafe {
                        f(
                            src.as_ptr(),
                            dst.as_mut_ptr(),
                            len as i32,
                            bound,
                            lvl,
                        )
                    };
                    dst.truncate(if n > 0 { n as usize } else { 0 });
                    (n, dst)
                });
            }
        }
    }
    // randomized
    for i in 0..250 {
        let shape = ALL_SHAPES[i % ALL_SHAPES.len()];
        let len = rng.range(1, 60000);
        let src = mkdata(shape, len, &mut rng);
        let lvl = LEVELS[rng.below(LEVELS.len())];
        let bound = compress_bound(len as i32);
        diff(&format!("HC rand #{i} len={len} lvl={lvl}"), |lib| {
            let f = unsafe { sym::<FnHC5>(lib, "LZ4_compress_HC") };
            let mut dst = vec![0xA5u8; bound as usize + 8];
            let n = unsafe { f(src.as_ptr(), dst.as_mut_ptr(), len as i32, bound, lvl) };
            dst.truncate(if n > 0 { n as usize } else { 0 });
            (n, dst)
        });
    }
}

#[test]
fn e050_compress_HC_bad_sizes_and_tight_dst() {
    let mut rng = Rng::new(0x5EED_1050);
    let src = mkdata(Shape::Textish, 8000, &mut rng);
    for &bad in [-1i32, -1000, i32::MIN, LZ4_MAX_INPUT_SIZE + 1, i32::MAX].iter() {
        diff(&format!("HC badsrc {bad}"), |lib| {
            let f = unsafe { sym::<FnHC5>(lib, "LZ4_compress_HC") };
            let mut dst = vec![0u8; 16384];
            unsafe { f(src.as_ptr(), dst.as_mut_ptr(), bad, dst.len() as i32, 9) }
        });
    }
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 4000, &mut rng);
        for &lvl in [1i32, 3, 9, 10, 12].iter() {
            let exact = {
                let i = impls();
                let f = unsafe { sym::<FnHC5>(&i.c, "LZ4_compress_HC") };
                let bound = compress_bound(4000);
                let mut dst = vec![0u8; bound as usize];
                unsafe { f(src.as_ptr(), dst.as_mut_ptr(), 4000, bound, lvl) }
            };
            for cap in [0i32, 1, 2, 8, exact / 4, exact / 2, exact - 1, exact, exact + 1] {
                diff(&format!("HC tight {shape:?} lvl={lvl} cap={cap}"), |lib| {
                    let f = unsafe { sym::<FnHC5>(lib, "LZ4_compress_HC") };
                    let mut dst = vec![0xA5u8; (cap.max(0) as usize) + 32];
                    let n =
                        unsafe { f(src.as_ptr(), dst.as_mut_ptr(), 4000, cap, lvl) };
                    dst.truncate(if n > 0 { n as usize } else { 0 });
                    (n, dst)
                });
            }
        }
    }
}

/* ================================================================== */
/* rows 42,43 / errors 52,53,54 — extStateHC variants                  */
/* ================================================================== */

#[test]
fn r042_extStateHC() {
    let mut rng = Rng::new(0x5EED_0042);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 4, 64, 5000, 65536, 120000].iter() {
            let src = mkdata(shape, len, &mut rng);
            for &lvl in LEVELS.iter() {
                for limited in [false, true] {
                    let bound = compress_bound(len as i32).max(1);
                    let cap = if limited { (bound / 2).max(1) } else { bound };
                    diff(
                        &format!("extStateHC {shape:?} len={len} lvl={lvl} lim={limited}"),
                        |lib| {
                            let ss = sizeof_stateHC(lib);
                            let mut st = Aligned::new(ss + STATE_SLOP, 16);
                            let f =
                                unsafe { sym::<FnHCState6>(lib, "LZ4_compress_HC_extStateHC") };
                            let mut dst = vec![0xA5u8; bound as usize + 8];
                            let n = unsafe {
                                f(
                                    st.as_mut_ptr() as *mut CVoid,
                                    src.as_ptr(),
                                    dst.as_mut_ptr(),
                                    len as i32,
                                    cap,
                                    lvl,
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
fn e052_extStateHC_bad_state() {
    let mut rng = Rng::new(0x5EED_1052);
    let src = mkdata(Shape::Textish, 2000, &mut rng);
    diff("extStateHC NULL state", |lib| {
        let f = unsafe { sym::<FnHCState6>(lib, "LZ4_compress_HC_extStateHC") };
        let mut dst = vec![0u8; 8192];
        unsafe {
            f(
                std::ptr::null_mut(),
                src.as_ptr(),
                dst.as_mut_ptr(),
                2000,
                8192,
                9,
            )
        }
    });
    diff("extStateHC misaligned state", |lib| {
        let ss = sizeof_stateHC(lib);
        let mut st = Aligned::new(ss + 64, 64);
        let f = unsafe { sym::<FnHCState6>(lib, "LZ4_compress_HC_extStateHC") };
        let mut dst = vec![0u8; 8192];
        let mut r = Vec::new();
        for off in 1usize..8 {
            r.push(unsafe {
                f(
                    (st.as_mut_ptr() as usize + off) as *mut CVoid,
                    src.as_ptr(),
                    dst.as_mut_ptr(),
                    2000,
                    8192,
                    9,
                )
            });
        }
        r
    });
    // NOTE: `LZ4_compress_HC_extStateHC_fastReset` documents that the state is
    // "presumed correctly initialized" — the C dereferences it before any NULL
    // check, so a NULL state faults in the C too and is not a testable
    // rejection. Its *misalignment* guard is, however, real:
    diff("extStateHC_fastReset misaligned state", |lib| {
        let ss = sizeof_stateHC(lib);
        let mut st = Aligned::new(ss + 64, 64);
        unsafe {
            sym::<FnInitHC>(lib, "LZ4_initStreamHC")(st.as_mut_ptr() as *mut CVoid, ss);
        }
        let f = unsafe { sym::<FnHCState6>(lib, "LZ4_compress_HC_extStateHC_fastReset") };
        let mut dst = vec![0u8; 8192];
        let mut r = Vec::new();
        for off in 1usize..8 {
            r.push(unsafe {
                f(
                    (st.as_mut_ptr() as usize + off) as *mut CVoid,
                    src.as_ptr(),
                    dst.as_mut_ptr(),
                    2000,
                    8192,
                    9,
                )
            });
        }
        r
    });
}

#[test]
fn r043_extStateHC_fastReset() {
    let mut rng = Rng::new(0x5EED_0043);
    for &shape in ALL_SHAPES.iter() {
        let chunks: Vec<Vec<u8>> = (0..5)
            .map(|_| {
                let l = rng.range(1, 40000);
                mkdata(shape, l, &mut rng)
            })
            .collect();
        for &lvl in [-1i32, 1, 3, 9, 10, 12, 99].iter() {
            for limited in [false, true] {
                diff(
                    &format!("fastResetHC {shape:?} lvl={lvl} lim={limited}"),
                    |lib| {
                        let ss = sizeof_stateHC(lib);
                        let mut st = Aligned::new(ss + STATE_SLOP, 16);
                        unsafe {
                            let p = sym::<FnInitHC>(lib, "LZ4_initStreamHC")(
                                st.as_mut_ptr() as *mut CVoid,
                                ss,
                            );
                            assert!(!p.is_null());
                        }
                        let f = unsafe {
                            sym::<FnHCState6>(lib, "LZ4_compress_HC_extStateHC_fastReset")
                        };
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
                                    lvl,
                                )
                            };
                            out.push(n);
                            dst.truncate(if n > 0 { n as usize } else { 0 });
                            out.extend(dst.iter().map(|&b| b as i32));
                        }
                        out
                    },
                );
            }
        }
    }
}

/* ================================================================== */
/* row 44 / errors 55–58 — LZ4_compress_HC_destSize                    */
/* ================================================================== */

#[test]
fn r044_HC_destSize() {
    let mut rng = Rng::new(0x5EED_0044);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 4, 30, 700, 9000, 65536, 100000].iter() {
            let src = mkdata(shape, len, &mut rng);
            let bound = compress_bound(len as i32);
            for &lvl in [-1i32, 1, 3, 9, 10, 12, 99].iter() {
                for &t in [
                    -1i32,
                    0,
                    1,
                    2,
                    8,
                    (len / 4) as i32,
                    (len / 2) as i32,
                    len as i32,
                    bound,
                    bound + 50,
                ]
                .iter()
                {
                    diff(
                        &format!("HC_destSize {shape:?} len={len} lvl={lvl} t={t}"),
                        |lib| {
                            let ss = sizeof_stateHC(lib);
                            let mut st = Aligned::new(ss + STATE_SLOP, 16);
                            let f = unsafe { sym::<FnHCDestSize>(lib, "LZ4_compress_HC_destSize") };
                            let mut sp = len as i32;
                            let mut dst = vec![0xA5u8; (t.max(0) as usize) + 64];
                            let n = unsafe {
                                f(
                                    st.as_mut_ptr() as *mut CVoid,
                                    src.as_ptr(),
                                    dst.as_mut_ptr(),
                                    &mut sp,
                                    t,
                                    lvl,
                                )
                            };
                            dst.truncate(if n > 0 { n as usize } else { 0 });
                            (n, sp, dst)
                        },
                    );
                }
            }
        }
    }
    // negative *srcSizePtr, NULL state
    diff("HC_destSize negative srcSize / NULL state", |lib| {
        let ss = sizeof_stateHC(lib);
        let mut st = Aligned::new(ss + STATE_SLOP, 16);
        let f = unsafe { sym::<FnHCDestSize>(lib, "LZ4_compress_HC_destSize") };
        let src = [1u8; 64];
        let mut dst = vec![0u8; 256];
        let mut a = -1i32;
        let ra = unsafe {
            f(
                st.as_mut_ptr() as *mut CVoid,
                src.as_ptr(),
                dst.as_mut_ptr(),
                &mut a,
                128,
                9,
            )
        };
        let mut b = 64i32;
        let rb = unsafe {
            f(
                st.as_mut_ptr() as *mut CVoid,
                src.as_ptr(),
                dst.as_mut_ptr(),
                &mut b,
                -5,
                9,
            )
        };
        let mut c = 64i32;
        let rc = unsafe {
            f(
                std::ptr::null_mut(),
                src.as_ptr(),
                dst.as_mut_ptr(),
                &mut c,
                128,
                9,
            )
        };
        (ra, a, rb, b, rc, c)
    });
    // randomized
    for i in 0..300 {
        let shape = ALL_SHAPES[i % ALL_SHAPES.len()];
        let len = rng.range(1, 20000);
        let src = mkdata(shape, len, &mut rng);
        let t = rng.range(0, compress_bound(len as i32) as usize + 8) as i32;
        let lvl = LEVELS[rng.below(LEVELS.len())];
        diff(&format!("HC_destSize rand #{i}"), |lib| {
            let ss = sizeof_stateHC(lib);
            let mut st = Aligned::new(ss + STATE_SLOP, 16);
            let f = unsafe { sym::<FnHCDestSize>(lib, "LZ4_compress_HC_destSize") };
            let mut sp = len as i32;
            let mut dst = vec![0xA5u8; (t.max(0) as usize) + 64];
            let n = unsafe {
                f(
                    st.as_mut_ptr() as *mut CVoid,
                    src.as_ptr(),
                    dst.as_mut_ptr(),
                    &mut sp,
                    t,
                    lvl,
                )
            };
            dst.truncate(if n > 0 { n as usize } else { 0 });
            (n, sp, dst)
        });
    }
}

/* ================================================================== */
/* rows 45,46,47,48 — HC streaming                                     */
/* ================================================================== */

#[test]
fn r045_r046_HC_streaming() {
    let mut rng = Rng::new(0x5EED_0045);
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 200000, &mut rng);
        for fast in [false, true] {
            for &lvl in [-1i32, 1, 2, 3, 9, 10, 12, 99].iter() {
                for pattern in 0..3 {
                    let chunks: Vec<usize> = match pattern {
                        0 => vec![1; 100],
                        1 => vec![20000; 10],
                        _ => (0..40).map(|_| rng.range(1, 12000)).collect(),
                    };
                    diff(
                        &format!("HC stream {shape:?} fast={fast} lvl={lvl} pat={pattern}"),
                        |lib| unsafe {
                            let s = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
                            assert!(!s.is_null());
                            if fast {
                                sym::<FnResetLevel>(lib, "LZ4_resetStreamHC_fast")(s, lvl);
                            } else {
                                sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(s, lvl);
                            }
                            let f = sym::<FnHCContinue>(lib, "LZ4_compress_HC_continue");
                            let mut out = Vec::new();
                            let mut off = 0usize;
                            for &c in chunks.iter() {
                                if off >= src.len() {
                                    break;
                                }
                                let n = c.min(src.len() - off);
                                let bound = compress_bound(n as i32).max(1);
                                let mut dst = vec![0u8; bound as usize];
                                let r = f(
                                    s,
                                    src[off..].as_ptr(),
                                    dst.as_mut_ptr(),
                                    n as i32,
                                    bound,
                                );
                                out.push(r);
                                dst.truncate(if r > 0 { r as usize } else { 0 });
                                out.extend(dst.iter().map(|&b| b as i32));
                                off += n;
                            }
                            sym::<FnFree1>(lib, "LZ4_freeStreamHC")(s);
                            out
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn r047_setCompressionLevel_midstream() {
    let mut rng = Rng::new(0x5EED_0047);
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 120000, &mut rng);
        diff(&format!("HC level switch {shape:?}"), |lib| unsafe {
            let s = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
            sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(s, 1);
            let setl = sym::<FnResetLevel>(lib, "LZ4_setCompressionLevel");
            let f = sym::<FnHCContinue>(lib, "LZ4_compress_HC_continue");
            let mut out = Vec::new();
            let levels = [-5i32, 0, 1, 3, 9, 10, 11, 12, 20];
            let mut off = 0usize;
            for &lvl in levels.iter() {
                if off >= src.len() {
                    break;
                }
                setl(s, lvl);
                let n = 12000usize.min(src.len() - off);
                let bound = compress_bound(n as i32);
                let mut dst = vec![0u8; bound as usize];
                let r = f(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound);
                out.push(r);
                dst.truncate(if r > 0 { r as usize } else { 0 });
                out.extend(dst.iter().map(|&b| b as i32));
                off += n;
            }
            sym::<FnFree1>(lib, "LZ4_freeStreamHC")(s);
            out
        });
    }
}

#[test]
fn r048_favorDecompressionSpeed() {
    let mut rng = Rng::new(0x5EED_0048);
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 90000, &mut rng);
        for &fav in [0i32, 1, 2, -1].iter() {
            for &lvl in [1i32, 9, 10, 11, 12].iter() {
                diff(
                    &format!("favorDecSpeed {shape:?} fav={fav} lvl={lvl}"),
                    |lib| unsafe {
                        let s = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
                        sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(s, lvl);
                        sym::<FnResetLevel>(lib, "LZ4_favorDecompressionSpeed")(s, fav);
                        let f = sym::<FnHCContinue>(lib, "LZ4_compress_HC_continue");
                        let bound = compress_bound(src.len() as i32);
                        let mut dst = vec![0u8; bound as usize];
                        let r = f(
                            s,
                            src.as_ptr(),
                            dst.as_mut_ptr(),
                            src.len() as i32,
                            bound,
                        );
                        dst.truncate(if r > 0 { r as usize } else { 0 });
                        sym::<FnFree1>(lib, "LZ4_freeStreamHC")(s);
                        (r, dst)
                    },
                );
            }
        }
    }
}

/* ================================================================== */
/* rows 49,50,51 / errors 67,68,69 — HC dictionaries                   */
/* ================================================================== */

const DICT_SIZES: [usize; 10] = [0, 1, 3, 4, 5, 64, 4096, 65535, 65536, 70000];

#[test]
fn r049_loadDictHC() {
    let mut rng = Rng::new(0x5EED_0049);
    for &ds in DICT_SIZES.iter() {
        for &lvl in [-1i32, 1, 3, 9, 10, 12, 99].iter() {
            for &shape in ALL_SHAPES.iter() {
                let dict = mkdata(shape, ds, &mut rng);
                let src = mkdata(shape, 25000, &mut rng);
                diff(&format!("loadDictHC ds={ds} lvl={lvl} {shape:?}"), |lib| unsafe {
                    let s = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
                    sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(s, lvl);
                    let loaded = sym::<FnLoadDict>(lib, "LZ4_loadDictHC")(
                        s,
                        if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() },
                        ds as i32,
                    );
                    let f = sym::<FnHCContinue>(lib, "LZ4_compress_HC_continue");
                    let mut out = vec![loaded];
                    let mut off = 0usize;
                    for cl in [6000usize, 19000] {
                        let n = cl.min(src.len() - off);
                        let bound = compress_bound(n as i32);
                        let mut dst = vec![0u8; bound as usize];
                        let r = f(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound);
                        out.push(r);
                        dst.truncate(if r > 0 { r as usize } else { 0 });
                        out.extend(dst.iter().map(|&b| b as i32));
                        off += n;
                    }
                    sym::<FnFree1>(lib, "LZ4_freeStreamHC")(s);
                    out
                });
            }
        }
    }
}

#[test]
fn r050_attach_HC_dictionary() {
    let mut rng = Rng::new(0x5EED_0050);
    for &ds in [0usize, 4, 1000, 65536].iter() {
        for &lvl in [1i32, 3, 9, 12].iter() {
            for detach in [false, true] {
                let dict = mkdata(Shape::Textish, ds, &mut rng);
                let src = mkdata(Shape::Textish, 30000, &mut rng);
                diff(
                    &format!("attachHC ds={ds} lvl={lvl} detach={detach}"),
                    |lib| unsafe {
                        let d = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
                        let w = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
                        sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(d, lvl);
                        let loaded = sym::<FnLoadDict>(lib, "LZ4_loadDictHC")(
                            d,
                            if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() },
                            ds as i32,
                        );
                        sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(w, lvl);
                        let at = sym::<FnAttach>(lib, "LZ4_attach_HC_dictionary");
                        if detach {
                            at(w, std::ptr::null());
                        } else {
                            at(w, d as *const CVoid);
                        }
                        let f = sym::<FnHCContinue>(lib, "LZ4_compress_HC_continue");
                        let bound = compress_bound(src.len() as i32);
                        let mut dst = vec![0u8; bound as usize];
                        let r = f(
                            w,
                            src.as_ptr(),
                            dst.as_mut_ptr(),
                            src.len() as i32,
                            bound,
                        );
                        dst.truncate(if r > 0 { r as usize } else { 0 });
                        sym::<FnFree1>(lib, "LZ4_freeStreamHC")(d);
                        sym::<FnFree1>(lib, "LZ4_freeStreamHC")(w);
                        (loaded, r, dst)
                    },
                );
            }
        }
    }
}

#[test]
fn r051_saveDictHC() {
    let mut rng = Rng::new(0x5EED_0051);
    for &md in [-1i32, 0, 1, 4, 1000, 65535, 65536, 70000].iter() {
        for &lvl in [1i32, 9, 12].iter() {
            for &shape in ALL_SHAPES.iter() {
                let src = mkdata(shape, 90000, &mut rng);
                diff(&format!("saveDictHC md={md} lvl={lvl} {shape:?}"), |lib| unsafe {
                    let s = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
                    sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(s, lvl);
                    let f = sym::<FnHCContinue>(lib, "LZ4_compress_HC_continue");
                    let sd = sym::<FnSaveDict>(lib, "LZ4_saveDictHC");
                    let mut safebuf = vec![0u8; 80000];
                    let mut out = Vec::new();
                    let mut off = 0usize;
                    for _ in 0..3 {
                        let n = 25000usize.min(src.len() - off);
                        let bound = compress_bound(n as i32);
                        let mut dst = vec![0u8; bound as usize];
                        let r = f(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound);
                        out.push(r);
                        dst.truncate(if r > 0 { r as usize } else { 0 });
                        out.extend(dst.iter().map(|&b| b as i32));
                        off += n;
                        let k = sd(s, safebuf.as_mut_ptr(), md);
                        out.push(k);
                        if k > 0 {
                            out.extend(safebuf[..k as usize].iter().map(|&b| b as i32));
                        }
                    }
                    sym::<FnFree1>(lib, "LZ4_freeStreamHC")(s);
                    out
                });
            }
        }
    }
}

/* ================================================================== */
/* row 52 / error 71 — LZ4_compress_HC_continue_destSize               */
/* ================================================================== */

#[test]
fn r052_HC_continue_destSize() {
    let mut rng = Rng::new(0x5EED_0052);
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 80000, &mut rng);
        for &lvl in [-1i32, 1, 3, 9, 10, 12, 99].iter() {
            for &t in [0i32, 1, 2, 16, 300, 4000, 30000].iter() {
                diff(
                    &format!("HC_continue_destSize {shape:?} lvl={lvl} t={t}"),
                    |lib| unsafe {
                        let s = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
                        sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(s, lvl);
                        let f = sym::<FnHCContinueDestSize>(
                            lib,
                            "LZ4_compress_HC_continue_destSize",
                        );
                        let mut out = Vec::new();
                        let mut off = 0usize;
                        for _ in 0..6 {
                            if off >= src.len() {
                                break;
                            }
                            let mut sp = (src.len() - off).min(30000) as i32;
                            let mut dst = vec![0u8; (t.max(0) as usize) + 64];
                            let r = f(
                                s,
                                src[off..].as_ptr(),
                                dst.as_mut_ptr(),
                                &mut sp,
                                t,
                            );
                            out.push(r);
                            out.push(sp);
                            dst.truncate(if r > 0 { r as usize } else { 0 });
                            out.extend(dst.iter().map(|&b| b as i32));
                            if sp <= 0 {
                                break;
                            }
                            off += sp as usize;
                        }
                        sym::<FnFree1>(lib, "LZ4_freeStreamHC")(s);
                        out
                    },
                );
            }
        }
    }
}

/* ================================================================== */
/* row 53 / errors 59,60,61 — LZ4_initStreamHC                         */
/* ================================================================== */

#[test]
fn e059_initStreamHC_guards() {
    diff("initStreamHC guards", |lib| {
        let ss = sizeof_stateHC(lib);
        let ini = unsafe { sym::<FnInitHC>(lib, "LZ4_initStreamHC") };
        let mut buf = Aligned::new(ss + 64, 64);
        unsafe {
            let a = ini(std::ptr::null_mut(), ss).is_null();
            let b = ini(buf.as_mut_ptr() as *mut CVoid, ss - 1).is_null();
            let c = ini(buf.as_mut_ptr() as *mut CVoid, 0).is_null();
            let d = ini(buf.as_mut_ptr() as *mut CVoid, ss).is_null();
            let e = ini(buf.as_mut_ptr() as *mut CVoid, ss + 64).is_null();
            let mut mis = Vec::new();
            for off in 1usize..8 {
                mis.push(ini(buf.as_mut_ptr().add(off) as *mut CVoid, ss).is_null());
            }
            (a, b, c, d, e, mis)
        }
    });
}

#[test]
fn r053_initStreamHC_then_compress() {
    let mut rng = Rng::new(0x5EED_0053);
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 40000, &mut rng);
        for &lvl in [1i32, 9, 12].iter() {
            diff(&format!("initStreamHC {shape:?} lvl={lvl}"), |lib| unsafe {
                let ss = sizeof_stateHC(lib);
                let mut buf = Aligned::new(ss + 64, 64);
                let s = sym::<FnInitHC>(lib, "LZ4_initStreamHC")(
                    buf.as_mut_ptr() as *mut CVoid,
                    ss + 64,
                );
                assert!(!s.is_null());
                sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(s, lvl);
                let f = sym::<FnHCContinue>(lib, "LZ4_compress_HC_continue");
                let mut out = Vec::new();
                let mut off = 0usize;
                while off < src.len() {
                    let n = (src.len() - off).min(9000);
                    let bound = compress_bound(n as i32);
                    let mut dst = vec![0u8; bound as usize];
                    let r = f(s, src[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound);
                    out.push(r);
                    dst.truncate(if r > 0 { r as usize } else { 0 });
                    out.extend(dst.iter().map(|&b| b as i32));
                    off += n;
                }
                out
            });
        }
    }
}

/* ================================================================== */
/* row 54 — deprecated HC one-shot family                              */
/* ================================================================== */

#[test]
fn r054_deprecated_HC_oneshot() {
    let mut rng = Rng::new(0x5EED_0054);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 100, 5000, 65536].iter() {
            let src = mkdata(shape, len, &mut rng);
            let bound = compress_bound(len as i32).max(1);
            let sp = src.as_ptr();
            for &lvl in [-1i32, 1, 9, 12, 99].iter() {
                diff(
                    &format!("depr HC {shape:?} len={len} lvl={lvl}"),
                    |lib| unsafe {
                        let ss = sizeof_stateHC(lib);
                        let mut st = Aligned::new(ss + STATE_SLOP, 16);
                        let stp = st.as_mut_ptr() as *mut CVoid;
                        let mut res: Vec<(i32, Vec<u8>)> = Vec::new();
                        let mut go = |n: i32, d: Vec<u8>| res.push((n, d));

                        let mut d = vec![0u8; bound as usize + 8];
                        let n = sym::<FnHC3>(lib, "LZ4_compressHC")(sp, d.as_mut_ptr(), len as i32);
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        go(n, d);

                        let mut d = vec![0u8; bound as usize + 8];
                        let n = sym::<FnHC4>(lib, "LZ4_compressHC_limitedOutput")(
                            sp,
                            d.as_mut_ptr(),
                            len as i32,
                            bound / 2,
                        );
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        go(n, d);

                        let mut d = vec![0u8; bound as usize + 8];
                        let n = sym::<FnHC4>(lib, "LZ4_compressHC2")(
                            sp,
                            d.as_mut_ptr(),
                            len as i32,
                            lvl,
                        );
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        go(n, d);

                        let mut d = vec![0u8; bound as usize + 8];
                        let n = sym::<FnHC5>(lib, "LZ4_compressHC2_limitedOutput")(
                            sp,
                            d.as_mut_ptr(),
                            len as i32,
                            bound,
                            lvl,
                        );
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        go(n, d);

                        let mut d = vec![0u8; bound as usize + 8];
                        let n = sym::<FnHCSt4>(lib, "LZ4_compressHC_withStateHC")(
                            stp,
                            sp,
                            d.as_mut_ptr(),
                            len as i32,
                        );
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        go(n, d);

                        let mut d = vec![0u8; bound as usize + 8];
                        let n = sym::<FnHCSt5>(
                            lib,
                            "LZ4_compressHC_limitedOutput_withStateHC",
                        )(stp, sp, d.as_mut_ptr(), len as i32, bound);
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        go(n, d);

                        let mut d = vec![0u8; bound as usize + 8];
                        let n = sym::<FnHCSt5>(lib, "LZ4_compressHC2_withStateHC")(
                            stp,
                            sp,
                            d.as_mut_ptr(),
                            len as i32,
                            lvl,
                        );
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        go(n, d);

                        let mut d = vec![0u8; bound as usize + 8];
                        let n = sym::<FnHCSt6>(
                            lib,
                            "LZ4_compressHC2_limitedOutput_withStateHC",
                        )(stp, sp, d.as_mut_ptr(), len as i32, bound, lvl);
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        go(n, d);

                        res
                    },
                );
            }
        }
    }
}

#[test]
fn r055_legacy_HC_streaming() {
    let mut rng = Rng::new(0x5EED_0055);
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 60000, &mut rng);
        diff(&format!("depr HC continue {shape:?}"), |lib| unsafe {
            let s = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
            sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(s, 9);
            let f = sym::<FnHCSt4>(lib, "LZ4_compressHC_continue");
            let g = sym::<FnHCSt5>(lib, "LZ4_compressHC_limitedOutput_continue");
            let mut out = Vec::new();
            let mut off = 0usize;
            let mut k = 0;
            while off < src.len() {
                let n = 6000usize.min(src.len() - off);
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
            sym::<FnFree1>(lib, "LZ4_freeStreamHC")(s);
            out
        });
    }
    // LZ4_createHC / LZ4_compressHC2_continue / LZ4_slideInputBufferHC / LZ4_freeHC
    let src = mkdata(Shape::Textish, 50000, &mut rng);
    diff("legacy createHC pipeline", |lib| unsafe {
        let mut inbuf = vec![0u8; 200000];
        inbuf[..src.len()].copy_from_slice(&src);
        let st = sym::<FnCreateHC>(lib, "LZ4_createHC")(inbuf.as_ptr());
        let isnull = st.is_null();
        let mut out: Vec<i32> = Vec::new();
        if !isnull {
            let f = sym::<FnHCSt5>(lib, "LZ4_compressHC2_continue");
            let g = sym::<FnHCSt6>(lib, "LZ4_compressHC2_limitedOutput_continue");
            let mut off = 0usize;
            let mut k = 0;
            while off < src.len() {
                let n = 7000usize.min(src.len() - off);
                let bound = compress_bound(n as i32);
                let mut dst = vec![0u8; bound as usize];
                let r = if k % 2 == 0 {
                    f(st, inbuf[off..].as_ptr(), dst.as_mut_ptr(), n as i32, 9)
                } else {
                    g(st, inbuf[off..].as_ptr(), dst.as_mut_ptr(), n as i32, bound, 9)
                };
                out.push(r);
                dst.truncate(if r > 0 { r as usize } else { 0 });
                out.extend(dst.iter().map(|&b| b as i32));
                off += n;
                k += 1;
            }
            // slide: returned pointer offset relative to inbuf must match
            let p = sym::<FnSlideHC>(lib, "LZ4_slideInputBufferHC")(st);
            let delta = (p as isize) - (inbuf.as_ptr() as isize);
            out.push(delta as i32);
            sym::<FnFree1>(lib, "LZ4_freeHC")(st);
        }
        (isnull, out)
    });
}

#[test]
fn r056_resetStreamStateHC() {
    let mut rng = Rng::new(0x5EED_0056);
    let src = mkdata(Shape::Textish, 20000, &mut rng);
    diff("resetStreamStateHC", |lib| unsafe {
        let ss = sizeof_stateHC(lib);
        let mut st = Aligned::new(ss + 64, 64);
        let f = sym::<FnResetStateHC>(lib, "LZ4_resetStreamStateHC");
        let mut inbuf = vec![0u8; 64];
        let ok = f(st.as_mut_ptr() as *mut CVoid, inbuf.as_mut_ptr());
        // NULL and misaligned: LZ4_initStreamHC returns NULL -> 1
        let nul = f(std::ptr::null_mut(), inbuf.as_mut_ptr());
        let mut mis = Vec::new();
        for off in 1usize..8 {
            mis.push(f(
                (st.as_mut_ptr() as usize + off) as *mut CVoid,
                inbuf.as_mut_ptr(),
            ));
        }
        // usable afterwards
        let bound = compress_bound(src.len() as i32);
        let mut dst = vec![0u8; bound as usize];
        let n = sym::<FnHCSt4>(lib, "LZ4_compressHC_continue")(
            st.as_mut_ptr() as *mut CVoid,
            src.as_ptr(),
            dst.as_mut_ptr(),
            src.len() as i32,
        );
        dst.truncate(if n > 0 { n as usize } else { 0 });
        (ok, nul, mis, n, dst)
    });
}

/* ================================================================== */
/* HC output must decode identically through both libraries            */
/* ================================================================== */

#[test]
fn hc_roundtrip_cross_impl() {
    let mut rng = Rng::new(0x5EED_00FF);
    let i = impls();
    for &shape in ALL_SHAPES.iter() {
        for &len in [1usize, 100, 5000, 70000].iter() {
            for &lvl in [1i32, 3, 9, 10, 12].iter() {
                let src = mkdata(shape, len, &mut rng);
                let bound = compress_bound(len as i32);
                // compress with each library, decode with the other
                let mut outs = Vec::new();
                for lib in [&i.c, &i.r] {
                    let f = unsafe { sym::<FnHC5>(lib, "LZ4_compress_HC") };
                    let mut d = vec![0u8; bound as usize];
                    let n = unsafe {
                        f(src.as_ptr(), d.as_mut_ptr(), len as i32, bound, lvl)
                    };
                    assert!(n > 0);
                    d.truncate(n as usize);
                    outs.push(d);
                }
                assert_eq!(outs[0], outs[1], "HC bytes differ {shape:?} {len} {lvl}");
                for lib in [&i.c, &i.r] {
                    for c in outs.iter() {
                        let g = unsafe { sym::<FnDecSafe>(lib, "LZ4_decompress_safe") };
                        let mut o = vec![0u8; len + 16];
                        let n = unsafe {
                            g(c.as_ptr(), o.as_mut_ptr(), c.len() as i32, (len + 16) as i32)
                        };
                        assert_eq!(n, len as i32);
                        assert_eq!(&o[..len], &src[..]);
                    }
                }
            }
        }
    }
}

/* ================================================================== */
/* row 70 — LZ4_compress_HC_continue with an insufficient dst budget   */
/* ================================================================== */

#[test]
fn e070_HC_continue_tight_dst() {
    let mut rng = Rng::new(0x5EED_1070);
    let chunk = 9000usize;
    for &shape in ALL_SHAPES.iter() {
        let src = mkdata(shape, 60000, &mut rng);
        for &lvl in [1i32, 3, 9, 12].iter() {
            let exact = {
                let i = impls();
                unsafe {
                    let s = sym::<FnCreate0>(&i.c, "LZ4_createStreamHC")();
                    sym::<FnResetLevel>(&i.c, "LZ4_resetStreamHC")(s, lvl);
                    let bound = compress_bound(chunk as i32);
                    let mut d = vec![0u8; bound as usize];
                    let n = sym::<FnHCContinue>(&i.c, "LZ4_compress_HC_continue")(
                        s,
                        src.as_ptr(),
                        d.as_mut_ptr(),
                        chunk as i32,
                        bound,
                    );
                    sym::<FnFree1>(&i.c, "LZ4_freeStreamHC")(s);
                    n
                }
            };
            for cap in [0i32, 1, 2, 8, exact / 4, exact / 2, exact - 1, exact] {
                diff(
                    &format!("HC continue tight {shape:?} lvl={lvl} cap={cap}"),
                    |lib| unsafe {
                        let s = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
                        sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(s, lvl);
                        let f = sym::<FnHCContinue>(lib, "LZ4_compress_HC_continue");
                        let mut out = Vec::new();
                        let mut off = 0usize;
                        for _ in 0..5 {
                            let n = chunk.min(src.len() - off);
                            let mut d = vec![0u8; (cap.max(0) as usize) + 32];
                            let r =
                                f(s, src[off..].as_ptr(), d.as_mut_ptr(), n as i32, cap);
                            out.push(r);
                            d.truncate(if r > 0 { r as usize } else { 0 });
                            out.extend(d.iter().map(|&b| b as i32));
                            off += n;
                        }
                        sym::<FnFree1>(lib, "LZ4_freeStreamHC")(s);
                        out
                    },
                );
            }
            // oversized / negative srcSize
            for bad in [-1i32, LZ4_MAX_INPUT_SIZE + 1, i32::MAX] {
                diff(
                    &format!("HC continue bad srcSize {shape:?} lvl={lvl} {bad}"),
                    |lib| unsafe {
                        let s = sym::<FnCreate0>(lib, "LZ4_createStreamHC")();
                        sym::<FnResetLevel>(lib, "LZ4_resetStreamHC")(s, lvl);
                        let mut d = vec![0u8; 1 << 16];
                        let r = sym::<FnHCContinue>(lib, "LZ4_compress_HC_continue")(
                            s,
                            src.as_ptr(),
                            d.as_mut_ptr(),
                            bad,
                            d.len() as i32,
                        );
                        sym::<FnFree1>(lib, "LZ4_freeStreamHC")(s);
                        r
                    },
                );
            }
        }
    }
}
