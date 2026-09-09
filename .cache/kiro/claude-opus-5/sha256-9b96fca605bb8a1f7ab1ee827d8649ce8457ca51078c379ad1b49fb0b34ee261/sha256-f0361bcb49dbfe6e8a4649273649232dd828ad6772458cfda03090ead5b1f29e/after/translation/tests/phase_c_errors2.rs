//! Phase C, part 2 — `ERRORS.md` rows D1–D15, E1–E36, M5, M6, M11.
//!
//! Every row constructs the exact invalid input the C rejects and asserts that
//! both `.so` files return the SAME error code (compared via
//! `ZSTD_getErrorCode`, not merely "both failed").

mod common;

use common::*;
use std::os::raw::{c_int, c_uint, c_void};

const P_LEVEL: c_int = 100;
const P_WINDOWLOG: c_int = 101;
const P_CONTENTSIZEFLAG: c_int = 200;
const P_CHECKSUMFLAG: c_int = 201;
const P_STABLEIN: c_int = 1006;
const P_STABLEOUT: c_int = 1007;
const DP_WINDOWLOGMAX: c_int = 100;
const DP_STABLEOUT: c_int = 1001;

type F2 = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;

/// Compare a pair of returns as ERROR CODES, not just "both failed".
#[track_caller]
fn eq_err(p: &Pair, what: &str, c: Sz, r: Sz) {
    let (c_ie, r_ie) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_gc, r_gc) = p.sym::<FnGetErrCode>("ZSTD_getErrorCode");
    let (c_gn, r_gn) = p.sym::<FnGetErrName>("ZSTD_getErrorName");
    unsafe {
        eq(&format!("{what}: isError"), c_ie(c), r_ie(r));
        eq(&format!("{what}: errorCode"), c_gc(c), r_gc(r));
        eq(&format!("{what}: errorName"), cstr(c_gn(c)), cstr(r_gn(r)));
        eq(&format!("{what}: raw return"), c, r);
    }
}

fn make_frame(p: &Pair, src: &[u8], level: c_int, checksum: c_int, content_size: c_int) -> Vec<u8> {
    let (c_new, _) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, _) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_set, _) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_c2, _) = p.sym::<F2>("ZSTD_compress2");
    unsafe {
        let cc = c_new();
        c_set(cc, P_LEVEL, level);
        c_set(cc, P_CHECKSUMFLAG, checksum);
        c_set(cc, P_CONTENTSIZEFLAG, content_size);
        let cap = c_cb(src.len()) + 64;
        let mut b = vec![0u8; cap];
        let n = c_c2(
            cc,
            b.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        );
        c_free(cc);
        assert!(c_ie(n) == 0, "make_frame failed");
        b.truncate(n);
        b
    }
}

// =============================== D — compression side ===============================

/// D1/D2/D3/D4/D8 — `ZSTD_compress` / `ZSTD_compressCCtx` with inadequate dst.
#[test]
fn d1_d8_compress_dst_too_small() {
    let p = libs();
    let (c_co, r_co) = p.sym::<FnCompress>("ZSTD_compress");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    type FC = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz, c_int) -> Sz;
    let (c_cc, r_cc) = p.sym::<FC>("ZSTD_compressCCtx");
    let mut rng = Rng::new(SEED ^ 0xD1);
    unsafe {
        let cc = c_new();
        let rc = r_new();
        for level in [1, 3, 19] {
            for shape in [Shape::Incompressible, Shape::TextLike, Shape::Zeros] {
                for len in [1usize, 64, 1024, 70_000] {
                    let src = gen(shape, len, &mut rng);
                    let full = c_cb(len);
                    for &cap in &[0usize, 1, 2, 8, 16, full / 4, full / 2, full - 1] {
                        let mut cb = vec![0u8; cap.max(1)];
                        let mut rb = vec![0u8; cap.max(1)];
                        let a = c_co(cb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, level);
                        let b = r_co(rb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, level);
                        eq_err(
                            p,
                            &format!("D1 ZSTD_compress(shape={shape:?},len={len},cap={cap},lvl={level})"),
                            a, b,
                        );
                        let a = c_cc(cc, cb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, level);
                        let b = r_cc(rc, rb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, level);
                        eq_err(
                            p,
                            &format!("D8 ZSTD_compressCCtx(shape={shape:?},len={len},cap={cap})"),
                            a, b,
                        );
                    }
                }
            }
        }
        // D3 — dst = NULL, dstCapacity = 0
        for len in [0usize, 1, 1024] {
            let src = gen(Shape::TextLike, len, &mut rng);
            let sp = if len == 0 { std::ptr::null() } else { src.as_ptr() as *const c_void };
            eq_err(
                p,
                &format!("D3 ZSTD_compress(dst=NULL,cap=0,len={len})"),
                c_co(std::ptr::null_mut(), 0, sp, len, 3),
                r_co(std::ptr::null_mut(), 0, sp, len, 3),
            );
        }
        // D4 — src = NULL, srcSize = 0 with adequate dst: success, empty frame
        {
            let cap = c_cb(0) + 64;
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = c_co(cb.as_mut_ptr() as *mut c_void, cap, std::ptr::null(), 0, 3);
            let b = r_co(rb.as_mut_ptr() as *mut c_void, cap, std::ptr::null(), 0, 3);
            eq("D4 ZSTD_compress(src=NULL,0) ret", a, b);
            let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
            if c_ie(a) == 0 {
                eq_bytes("D4 empty frame bytes", &cb[..a], &rb[..b]);
            }
        }
        c_free(cc);
        r_free(rc);
    }
}

/// D9/D10/D11 — the block API's size and stage rejections.
#[test]
fn d9_d11_block_api_rejections() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    type FBegin = unsafe extern "C" fn(*mut c_void, c_int) -> Sz;
    type FBlk = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    type FGetBs = unsafe extern "C" fn(*const c_void) -> Sz;
    let (c_begin, r_begin) = p.sym::<FBegin>("ZSTD_compressBegin");
    let (c_blk, r_blk) = p.sym::<FBlk>("ZSTD_compressBlock");
    let (c_cont, r_cont) = p.sym::<FBlk>("ZSTD_compressContinue");
    let (c_end, r_end) = p.sym::<FBlk>("ZSTD_compressEnd");
    let (c_bs, r_bs) = p.sym::<FGetBs>("ZSTD_getBlockSize");
    let mut rng = Rng::new(SEED ^ 0xD9);
    unsafe {
        // D10 — compressBlock with no ZSTD_compressBegin
        {
            let cc = c_new();
            let rc = r_new();
            let src = gen(Shape::TextLike, 1024, &mut rng);
            let mut cb = vec![0u8; 4096];
            let mut rb = vec![0u8; 4096];
            eq_err(
                p,
                "D10 ZSTD_compressBlock without compressBegin",
                c_blk(cc, cb.as_mut_ptr() as *mut c_void, cb.len(), src.as_ptr() as *const c_void, src.len()),
                r_blk(rc, rb.as_mut_ptr() as *mut c_void, rb.len(), src.as_ptr() as *const c_void, src.len()),
            );
            eq_err(
                p,
                "D10 ZSTD_compressContinue without compressBegin",
                c_cont(cc, cb.as_mut_ptr() as *mut c_void, cb.len(), src.as_ptr() as *const c_void, src.len()),
                r_cont(rc, rb.as_mut_ptr() as *mut c_void, rb.len(), src.as_ptr() as *const c_void, src.len()),
            );
            c_free(cc);
            r_free(rc);
        }
        // D9 — srcSize > ZSTD_BLOCKSIZE_MAX, and D11 — dstCapacity = 0
        for level in [1, 9, 19] {
            let cc = c_new();
            let rc = r_new();
            eq(&format!("compressBegin({level})"), c_begin(cc, level), r_begin(rc, level));
            let bs = c_bs(cc);
            eq("ZSTD_getBlockSize", bs, r_bs(rc));
            for &n in &[bs + 1, bs + 2, bs * 2, 1 << 18, 1 << 20] {
                let src = gen(Shape::TextLike, n, &mut rng);
                let mut cb = vec![0u8; n + 4096];
                let mut rb = vec![0u8; n + 4096];
                eq_err(
                    p,
                    &format!("D9 ZSTD_compressBlock(srcSize={n} > blockSize={bs})"),
                    c_blk(cc, cb.as_mut_ptr() as *mut c_void, cb.len(), src.as_ptr() as *const c_void, n),
                    r_blk(rc, rb.as_mut_ptr() as *mut c_void, rb.len(), src.as_ptr() as *const c_void, n),
                );
            }
            let src = gen(Shape::Incompressible, 4096, &mut rng);
            for &cap in &[0usize, 1, 8, 16, 128] {
                let mut cb = vec![0u8; cap.max(1)];
                let mut rb = vec![0u8; cap.max(1)];
                eq_err(
                    p,
                    &format!("D11 ZSTD_compressContinue(cap={cap})"),
                    c_cont(cc, cb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len()),
                    r_cont(rc, rb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len()),
                );
                eq_err(
                    p,
                    &format!("D11 ZSTD_compressEnd(cap={cap})"),
                    c_end(cc, cb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len()),
                    r_end(rc, rb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len()),
                );
            }
            c_free(cc);
            r_free(rc);
        }
    }
}

/// B10/B13/D12 — stage_wrong and pledgedSrcSize mismatches.
#[test]
fn b10_d12_stage_and_pledged_size() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    type FPledge = unsafe extern "C" fn(*mut c_void, u64) -> Sz;
    type FCS2 = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> Sz;
    let (c_pl, r_pl) = p.sym::<FPledge>("ZSTD_CCtx_setPledgedSrcSize");
    let (c_cs2, r_cs2) = p.sym::<FCS2>("ZSTD_compressStream2");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let mut rng = Rng::new(SEED ^ 0xD12);
    unsafe {
        for (pledge, actual) in [
            (100u64, 100usize),
            (100, 99),
            (100, 101),
            (0, 100),
            (1, 0),
            (u64::MAX, 100),
            (1 << 40, 100),
        ] {
            let src = gen(Shape::TextLike, actual, &mut rng);
            let cc = c_new();
            let rc = r_new();
            eq("reset", c_reset(cc, 2), r_reset(rc, 2));
            eq(
                &format!("D12 setPledgedSrcSize({pledge})"),
                c_pl(cc, pledge),
                r_pl(rc, pledge),
            );
            let cap = c_cb(actual.max(1)) + 64;
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let mut cin = InBuffer { src: src.as_ptr() as *const c_void, size: actual, pos: 0 };
            let mut rin = InBuffer { src: src.as_ptr() as *const c_void, size: actual, pos: 0 };
            let mut cob = OutBuffer { dst: cb.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
            let mut rob = OutBuffer { dst: rb.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
            let a = c_cs2(cc, &mut cob, &mut cin, 2);
            let b = r_cs2(rc, &mut rob, &mut rin, 2);
            eq_err(p, &format!("D12 compressStream2(pledge={pledge},actual={actual})"), a, b);
            eq(&format!("D12 in.pos(pledge={pledge})"), cin.pos, rin.pos);
            eq(&format!("D12 out.pos(pledge={pledge})"), cob.pos, rob.pos);
            eq_bytes(&format!("D12 out bytes(pledge={pledge})"), &cb[..cob.pos], &rb[..rob.pos]);

            // B13 — setPledgedSrcSize mid-frame
            eq_err(p, "B13 setPledgedSrcSize mid-frame", c_pl(cc, 7), r_pl(rc, 7));
            // B10 — setParameter mid-frame
            for prm in [P_WINDOWLOG, P_CHECKSUMFLAG, P_CONTENTSIZEFLAG, P_LEVEL] {
                eq_err(
                    p,
                    &format!("B10 setParameter({prm}) mid-frame"),
                    c_set(cc, prm, 1),
                    r_set(rc, prm, 1),
                );
            }
            c_free(cc);
            r_free(rc);
        }
    }
}

/// D13/D14/D15 — stability violations and out-of-range `ZSTD_EndDirective`.
#[test]
fn d13_d15_stability_and_end_directive() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    type FCS2 = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> Sz;
    let (c_cs2, r_cs2) = p.sym::<FCS2>("ZSTD_compressStream2");
    let mut rng = Rng::new(SEED ^ 0xD13);
    unsafe {
        // D15 — out-of-range ZSTD_EndDirective
        for endop in [-1, 3, 4, 99, c_int::MIN, c_int::MAX] {
            let cc = c_new();
            let rc = r_new();
            let src = gen(Shape::TextLike, 1024, &mut rng);
            let cap = c_cb(1024) + 64;
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let mut cin = InBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut rin = InBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut cob = OutBuffer { dst: cb.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
            let mut rob = OutBuffer { dst: rb.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
            eq_err(
                p,
                &format!("D15 compressStream2(endOp={endop})"),
                c_cs2(cc, &mut cob, &mut cin, endop),
                r_cs2(rc, &mut rob, &mut rin, endop),
            );
            eq(&format!("D15 in.pos(endOp={endop})"), cin.pos, rin.pos);
            eq(&format!("D15 out.pos(endOp={endop})"), cob.pos, rob.pos);
            c_free(cc);
            r_free(rc);
        }

        // D13 — stableInBuffer, then the input buffer MOVES between calls
        for stable_in in [0, 1] {
            for stable_out in [0, 1] {
                let cc = c_new();
                let rc = r_new();
                c_reset(cc, 2);
                r_reset(rc, 2);
                eq("set stableIn", c_set(cc, P_STABLEIN, stable_in), r_set(rc, P_STABLEIN, stable_in));
                eq("set stableOut", c_set(cc, P_STABLEOUT, stable_out), r_set(rc, P_STABLEOUT, stable_out));
                let src_a = gen(Shape::TextLike, 200_000, &mut rng);
                let src_b = gen(Shape::Incompressible, 200_000, &mut rng);
                let cap = 1024usize; // deliberately small so the call cannot finish
                let mut cb1 = vec![0u8; cap];
                let mut rb1 = vec![0u8; cap];
                let mut cb2 = vec![0u8; cap];
                let mut rb2 = vec![0u8; cap];
                let mut cin = InBuffer { src: src_a.as_ptr() as *const c_void, size: src_a.len(), pos: 0 };
                let mut rin = InBuffer { src: src_a.as_ptr() as *const c_void, size: src_a.len(), pos: 0 };
                let mut cob = OutBuffer { dst: cb1.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                let mut rob = OutBuffer { dst: rb1.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                let a = c_cs2(cc, &mut cob, &mut cin, 0);
                let b = r_cs2(rc, &mut rob, &mut rin, 0);
                eq_err(p, &format!("D13 first call(si={stable_in},so={stable_out})"), a, b);
                // now MOVE both buffers
                let mut cin2 = InBuffer { src: src_b.as_ptr() as *const c_void, size: src_b.len(), pos: cin.pos };
                let mut rin2 = InBuffer { src: src_b.as_ptr() as *const c_void, size: src_b.len(), pos: rin.pos };
                let mut cob2 = OutBuffer { dst: cb2.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                let mut rob2 = OutBuffer { dst: rb2.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                eq_err(
                    p,
                    &format!("D13/D14 moved buffers(si={stable_in},so={stable_out})"),
                    c_cs2(cc, &mut cob2, &mut cin2, 0),
                    r_cs2(rc, &mut rob2, &mut rin2, 0),
                );
                // and a moved OUTPUT buffer only
                let mut cob3 = OutBuffer { dst: cb1.as_mut_ptr() as *mut c_void, size: cap / 2, pos: 0 };
                let mut rob3 = OutBuffer { dst: rb1.as_mut_ptr() as *mut c_void, size: cap / 2, pos: 0 };
                eq_err(
                    p,
                    &format!("D14 resized out buffer(si={stable_in},so={stable_out})"),
                    c_cs2(cc, &mut cob3, &mut cin2, 2),
                    r_cs2(rc, &mut rob3, &mut rin2, 2),
                );
                c_free(cc);
                r_free(rc);
            }
        }
    }
}

// =============================== E — decompression side ===============================

/// E1–E12, E35 — malformed and truncated frames.
#[test]
fn e1_e12_frame_rejections() {
    let p = libs();
    let (c_de, r_de) = p.sym::<FnDecompress>("ZSTD_decompress");
    let mut rng = Rng::new(SEED ^ 0xE1);
    unsafe {
        // E1/E2/E3/E35 — truncations of a real frame
        for shape in [Shape::TextLike, Shape::Incompressible, Shape::Repetitive, Shape::Zeros] {
            for checksum in [0, 1] {
                for content_size in [0, 1] {
                    for len in [1usize, 1024, 70_000, 200_000] {
                        let src = gen(shape, len, &mut rng);
                        let frame = make_frame(p, &src, 3, checksum, content_size);
                        let mut cuts: Vec<usize> = (0..=frame.len().min(24)).collect();
                        cuts.extend([
                            frame.len() / 4,
                            frame.len() / 2,
                            frame.len() * 3 / 4,
                            frame.len() - 1,
                            frame.len(),
                        ]);
                        cuts.sort_unstable();
                        cuts.dedup();
                        for cut in cuts {
                            for &cap in &[0usize, 1, len / 2, len, len + 64] {
                                let mut cb = vec![0u8; cap.max(1)];
                                let mut rb = vec![0u8; cap.max(1)];
                                let a = c_de(cb.as_mut_ptr() as *mut c_void, cap, frame.as_ptr() as *const c_void, cut);
                                let b = r_de(rb.as_mut_ptr() as *mut c_void, cap, frame.as_ptr() as *const c_void, cut);
                                eq_err(
                                    p,
                                    &format!("E1-E5 decompress(shape={shape:?},len={len},ck={checksum},cs={content_size},cut={cut},cap={cap})"),
                                    a, b,
                                );
                                eq_bytes("E1-E5 dst image", &cb, &rb);
                            }
                        }

                        // E8 — flip one bit in the block body
                        for pos in [
                            frame.len() / 2,
                            frame.len() - 1,
                            frame.len().saturating_sub(5),
                            9.min(frame.len().saturating_sub(1)),
                        ] {
                            if pos >= frame.len() {
                                continue;
                            }
                            for bit in [0u8, 3, 7] {
                                let mut bad = frame.clone();
                                bad[pos] ^= 1 << bit;
                                let cap = len + 64;
                                let mut cb = vec![0u8; cap];
                                let mut rb = vec![0u8; cap];
                                let a = c_de(cb.as_mut_ptr() as *mut c_void, cap, bad.as_ptr() as *const c_void, bad.len());
                                let b = r_de(rb.as_mut_ptr() as *mut c_void, cap, bad.as_ptr() as *const c_void, bad.len());
                                eq_err(
                                    p,
                                    &format!("E8/E9 corrupted byte {pos} bit {bit} (shape={shape:?},len={len},ck={checksum})"),
                                    a, b,
                                );
                                eq_bytes("E8 dst image", &cb, &rb);
                            }
                        }
                    }
                }
            }
        }

        // E6 — random bytes (bad magic), E12 — bad version byte
        for n in [1usize, 2, 3, 4, 5, 8, 16, 64, 1024] {
            for shape in [Shape::Incompressible, Shape::Zeros] {
                let bad = gen(shape, n, &mut rng);
                let cap = 4096usize;
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                eq_err(
                    p,
                    &format!("E6 decompress(garbage shape={shape:?} n={n})"),
                    c_de(cb.as_mut_ptr() as *mut c_void, cap, bad.as_ptr() as *const c_void, n),
                    r_de(rb.as_mut_ptr() as *mut c_void, cap, bad.as_ptr() as *const c_void, n),
                );
            }
        }
        for magic in [
            0xFD2FB528u32, // valid
            0xFD2FB529,
            0xFD2FB527, // legacy v07
            0xFD2FB51E, // legacy v01
            0x00000000,
            0xFFFFFFFF,
            0x184D2A50, // skippable
            0x184D2A5F, // skippable, max variant
            0x184D2A60, // one past skippable
        ] {
            for extra in [0usize, 1, 4, 8, 16, 64] {
                let mut v = magic.to_le_bytes().to_vec();
                v.extend((0..extra).map(|_| rng.byte()));
                let cap = 4096usize;
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                eq_err(
                    p,
                    &format!("E6/E7/E11/E12 decompress(magic={magic:#x},extra={extra})"),
                    c_de(cb.as_mut_ptr() as *mut c_void, cap, v.as_ptr() as *const c_void, v.len()),
                    r_de(rb.as_mut_ptr() as *mut c_void, cap, v.as_ptr() as *const c_void, v.len()),
                );
                eq_bytes("E6 dst image", &cb, &rb);
            }
        }
        // E11 — reserved bit set in the frame descriptor
        {
            let src = gen(Shape::TextLike, 4096, &mut rng);
            let frame = make_frame(p, &src, 3, 0, 1);
            for bit in 0..8u8 {
                let mut bad = frame.clone();
                bad[4] ^= 1 << bit;
                let cap = 8192usize;
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                eq_err(
                    p,
                    &format!("E11 frame descriptor bit {bit} flipped"),
                    c_de(cb.as_mut_ptr() as *mut c_void, cap, bad.as_ptr() as *const c_void, bad.len()),
                    r_de(rb.as_mut_ptr() as *mut c_void, cap, bad.as_ptr() as *const c_void, bad.len()),
                );
            }
        }
    }
}

/// E9 — corrupted trailing checksum; E10 — windowLog beyond the decoder limit.
#[test]
fn e9_e10_checksum_and_window() {
    let p = libs();
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_dset, r_dset) = p.sym::<FnSetParam>("ZSTD_DCtx_setParameter");
    let (c_dd, r_dd) = p.sym::<F2>("ZSTD_decompressDCtx");
    let mut rng = Rng::new(SEED ^ 0xE9);
    unsafe {
        // E9 — flip bits in the last four bytes of a checksummed frame
        for shape in [Shape::TextLike, Shape::Incompressible] {
            for len in [64usize, 4096, 70_000] {
                let src = gen(shape, len, &mut rng);
                let frame = make_frame(p, &src, 3, 1, 1);
                for back in 1..=4usize {
                    let mut bad = frame.clone();
                    let i = frame.len() - back;
                    bad[i] ^= 0x01;
                    let cap = len + 64;
                    let mut cb = vec![0u8; cap];
                    let mut rb = vec![0u8; cap];
                    let cd = c_dnew();
                    let rd = r_dnew();
                    eq_err(
                        p,
                        &format!("E9 checksum byte -{back} flipped (shape={shape:?},len={len})"),
                        c_dd(cd, cb.as_mut_ptr() as *mut c_void, cap, bad.as_ptr() as *const c_void, bad.len()),
                        r_dd(rd, rb.as_mut_ptr() as *mut c_void, cap, bad.as_ptr() as *const c_void, bad.len()),
                    );
                    eq_bytes("E9 dst image", &cb, &rb);
                    c_dfree(cd);
                    r_dfree(rd);
                }
            }
        }
        // E10 — decoder windowLogMax below the frame's windowLog
        for wl in [10, 15, 20, 23] {
            let src = gen(Shape::Repetitive, 400_000, &mut rng);
            let (c_new, _) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
            let (c_free, _) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
            let (c_set, _) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
            let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
            let (c_c2, _) = p.sym::<F2>("ZSTD_compress2");
            let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
            let cc = c_new();
            c_set(cc, P_WINDOWLOG, wl);
            let cap = c_cb(src.len()) + 64;
            let mut fb = vec![0u8; cap];
            let n = c_c2(cc, fb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            c_free(cc);
            if c_ie(n) != 0 {
                continue;
            }
            fb.truncate(n);
            for dwl in [10, 11, 15, 20, 23, 27, 31] {
                let cd = c_dnew();
                let rd = r_dnew();
                let a = c_dset(cd, DP_WINDOWLOGMAX, dwl);
                let b = r_dset(rd, DP_WINDOWLOGMAX, dwl);
                eq_err(p, &format!("E10 DCtx_setParameter(windowLogMax={dwl})"), a, b);
                if c_ie(a) == 0 {
                    let cap = src.len() + 64;
                    let mut cb = vec![0u8; cap];
                    let mut rb = vec![0u8; cap];
                    eq_err(
                        p,
                        &format!("E10 decode(frame wl={wl}, decoder max={dwl})"),
                        c_dd(cd, cb.as_mut_ptr() as *mut c_void, cap, fb.as_ptr() as *const c_void, fb.len()),
                        r_dd(rd, rb.as_mut_ptr() as *mut c_void, cap, fb.as_ptr() as *const c_void, fb.len()),
                    );
                }
                c_dfree(cd);
                r_dfree(rd);
            }
        }
    }
}

/// E13–E23, E31 — the frame-introspection functions on invalid input.
#[test]
fn e13_e23_introspection_rejections() {
    let p = libs();
    type FU64 = unsafe extern "C" fn(*const c_void, Sz) -> u64;
    type FSZ = unsafe extern "C" fn(*const c_void, Sz) -> Sz;
    type FU = unsafe extern "C" fn(*const c_void, Sz) -> c_uint;
    type FGH = unsafe extern "C" fn(*mut FrameHeader, *const c_void, Sz) -> Sz;
    type FGHA = unsafe extern "C" fn(*mut FrameHeader, *const c_void, Sz, c_int) -> Sz;
    let (c_fcs, r_fcs) = p.sym::<FU64>("ZSTD_getFrameContentSize");
    let (c_gds, r_gds) = p.sym::<FU64>("ZSTD_getDecompressedSize");
    let (c_fds, r_fds) = p.sym::<FU64>("ZSTD_findDecompressedSize");
    let (c_db, r_db) = p.sym::<FU64>("ZSTD_decompressBound");
    let (c_ffcs, r_ffcs) = p.sym::<FSZ>("ZSTD_findFrameCompressedSize");
    let (c_fhs, r_fhs) = p.sym::<FSZ>("ZSTD_frameHeaderSize");
    let (c_isf, r_isf) = p.sym::<FU>("ZSTD_isFrame");
    let (c_isk, r_isk) = p.sym::<FU>("ZSTD_isSkippableFrame");
    let (c_gh, r_gh) = p.sym::<FGH>("ZSTD_getFrameHeader");
    let (c_gha, r_gha) = p.sym::<FGHA>("ZSTD_getFrameHeader_advanced");

    let mut rng = Rng::new(SEED ^ 0xE13);
    let mut corpus: Vec<(String, Vec<u8>)> = Vec::new();
    corpus.push(("empty".into(), Vec::new()));
    for n in [1usize, 2, 3, 4, 5, 8, 16, 64] {
        corpus.push((format!("garbage{n}"), gen(Shape::Incompressible, n, &mut rng)));
        corpus.push((format!("zeros{n}"), vec![0u8; n]));
    }
    for ck in [0, 1] {
        for cs in [0, 1] {
            for len in [0usize, 1, 1024, 70_000] {
                let src = gen(Shape::TextLike, len, &mut rng);
                let f = make_frame(p, &src, 3, ck, cs);
                corpus.push((format!("frame(ck={ck},cs={cs},len={len})"), f.clone()));
                // with trailing garbage (E18)
                let mut g = f.clone();
                g.extend(gen(Shape::Incompressible, 7, &mut rng));
                corpus.push((format!("frame+garbage(ck={ck},cs={cs},len={len})"), g));
                // two frames concatenated
                let mut h = f.clone();
                h.extend_from_slice(&f);
                corpus.push((format!("2frames(ck={ck},cs={cs},len={len})"), h));
            }
        }
    }

    unsafe {
        for (name, v) in &corpus {
            let mut cuts: Vec<usize> = (0..=v.len().min(24)).collect();
            for x in [v.len() / 2, v.len().saturating_sub(1), v.len()] {
                if !cuts.contains(&x) {
                    cuts.push(x);
                }
            }
            cuts.sort_unstable();
            cuts.dedup();
            for cut in cuts {
                let vp = if v.is_empty() { std::ptr::null() } else { v.as_ptr() as *const c_void };
                let tag = format!("{name} cut={cut}");
                eq(&format!("E13-E16 getFrameContentSize {tag}"), c_fcs(vp, cut), r_fcs(vp, cut));
                eq(&format!("E17 getDecompressedSize {tag}"), c_gds(vp, cut), r_gds(vp, cut));
                eq(&format!("E18 findDecompressedSize {tag}"), c_fds(vp, cut), r_fds(vp, cut));
                eq(&format!("E20 decompressBound {tag}"), c_db(vp, cut), r_db(vp, cut));
                eq_err(p, &format!("E19 findFrameCompressedSize {tag}"), c_ffcs(vp, cut), r_ffcs(vp, cut));
                eq_err(p, &format!("frameHeaderSize {tag}"), c_fhs(vp, cut), r_fhs(vp, cut));
                eq(&format!("E31 isFrame {tag}"), c_isf(vp, cut), r_isf(vp, cut));
                eq(&format!("isSkippableFrame {tag}"), c_isk(vp, cut), r_isk(vp, cut));
                // E21 — getFrameHeader must agree on both the return and the struct
                let mut ch = FrameHeader::default();
                let mut rh = FrameHeader::default();
                let a = c_gh(&mut ch, vp, cut);
                let b = r_gh(&mut rh, vp, cut);
                eq(&format!("E21 getFrameHeader {tag} ret"), a, b);
                eq(&format!("E21 getFrameHeader {tag} struct"), ch, rh);
                // E22/E23 — including out-of-range ZSTD_format_e values
                for fmt in [0, 1, -1, 2, 3, 99, c_int::MIN, c_int::MAX] {
                    let mut ch = FrameHeader::default();
                    let mut rh = FrameHeader::default();
                    let a = c_gha(&mut ch, vp, cut, fmt);
                    let b = r_gha(&mut rh, vp, cut, fmt);
                    eq(&format!("E22/E23 getFrameHeader_advanced {tag} fmt={fmt} ret"), a, b);
                    eq(&format!("E22/E23 getFrameHeader_advanced {tag} fmt={fmt} struct"), ch, rh);
                }
            }
        }
    }
}

/// E24/E25 — the raw decode block API's rejections.
#[test]
fn e24_e25_decompress_block_rejections() {
    let p = libs();
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    type FBlk = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    type FBegin = unsafe extern "C" fn(*mut c_void) -> Sz;
    let (c_blk, r_blk) = p.sym::<FBlk>("ZSTD_decompressBlock");
    let (c_bg, r_bg) = p.sym::<FBegin>("ZSTD_decompressBegin");
    let mut rng = Rng::new(SEED ^ 0xE24);
    unsafe {
        for init in [false, true] {
            let cd = c_dnew();
            let rd = r_dnew();
            if init {
                eq("E25 decompressBegin", c_bg(cd), r_bg(rd));
            }
            for &n in &[0usize, 1, 8, 131_072, 131_073, 1 << 18] {
                let src = gen(Shape::Incompressible, n, &mut rng);
                let sp = if n == 0 { std::ptr::null() } else { src.as_ptr() as *const c_void };
                for &cap in &[0usize, 1, 4096, 1 << 18] {
                    let mut cb = vec![0u8; cap.max(1)];
                    let mut rb = vec![0u8; cap.max(1)];
                    eq_err(
                        p,
                        &format!("E24/E25 decompressBlock(init={init},srcSize={n},cap={cap})"),
                        c_blk(cd, cb.as_mut_ptr() as *mut c_void, cap, sp, n),
                        r_blk(rd, rb.as_mut_ptr() as *mut c_void, cap, sp, n),
                    );
                    eq_bytes("E24 dst image", &cb, &rb);
                }
            }
            c_dfree(cd);
            r_dfree(rd);
        }
    }
}

/// E26/E27/E28/E29/E36 — `ZSTD_decompressStream` buffer and progress errors.
#[test]
fn e26_e36_decompress_stream_rejections() {
    let p = libs();
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDStream");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDStream");
    let (c_dset, r_dset) = p.sym::<FnSetParam>("ZSTD_DCtx_setParameter");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_DCtx_reset");
    type FDS = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> Sz;
    let (c_ds, r_ds) = p.sym::<FDS>("ZSTD_decompressStream");
    let mut rng = Rng::new(SEED ^ 0xE26);
    unsafe {
        let src = gen(Shape::TextLike, 200_000, &mut rng);
        let frame = make_frame(p, &src, 3, 1, 1);

        // E26 — input.pos > input.size
        for (isz, ipos) in [(10usize, 11usize), (0, 1), (5, 100), (frame.len(), frame.len() + 1)] {
            let cd = c_dnew();
            let rd = r_dnew();
            let mut cb = vec![0u8; 4096];
            let mut rb = vec![0u8; 4096];
            let mut cin = InBuffer { src: frame.as_ptr() as *const c_void, size: isz, pos: ipos };
            let mut rin = InBuffer { src: frame.as_ptr() as *const c_void, size: isz, pos: ipos };
            let mut cob = OutBuffer { dst: cb.as_mut_ptr() as *mut c_void, size: cb.len(), pos: 0 };
            let mut rob = OutBuffer { dst: rb.as_mut_ptr() as *mut c_void, size: rb.len(), pos: 0 };
            eq_err(
                p,
                &format!("E26 decompressStream(in.size={isz},in.pos={ipos})"),
                c_ds(cd, &mut cob, &mut cin),
                r_ds(rd, &mut rob, &mut rin),
            );
            c_dfree(cd);
            r_dfree(rd);
        }
        // E27 — output.pos > output.size
        for (osz, opos) in [(10usize, 11usize), (0, 1), (5, 100)] {
            let cd = c_dnew();
            let rd = r_dnew();
            let mut cb = vec![0u8; 4096];
            let mut rb = vec![0u8; 4096];
            let mut cin = InBuffer { src: frame.as_ptr() as *const c_void, size: frame.len(), pos: 0 };
            let mut rin = InBuffer { src: frame.as_ptr() as *const c_void, size: frame.len(), pos: 0 };
            let mut cob = OutBuffer { dst: cb.as_mut_ptr() as *mut c_void, size: osz, pos: opos };
            let mut rob = OutBuffer { dst: rb.as_mut_ptr() as *mut c_void, size: osz, pos: opos };
            eq_err(
                p,
                &format!("E27 decompressStream(out.size={osz},out.pos={opos})"),
                c_ds(cd, &mut cob, &mut cin),
                r_ds(rd, &mut rob, &mut rin),
            );
            c_dfree(cd);
            r_dfree(rd);
        }
        // E28 — repeated zero-progress calls with empty input
        {
            let cd = c_dnew();
            let rd = r_dnew();
            let mut cb = vec![0u8; 4096];
            let mut rb = vec![0u8; 4096];
            for i in 0..40 {
                let mut cin = InBuffer { src: std::ptr::null(), size: 0, pos: 0 };
                let mut rin = InBuffer { src: std::ptr::null(), size: 0, pos: 0 };
                let mut cob = OutBuffer { dst: cb.as_mut_ptr() as *mut c_void, size: cb.len(), pos: 0 };
                let mut rob = OutBuffer { dst: rb.as_mut_ptr() as *mut c_void, size: rb.len(), pos: 0 };
                eq_err(
                    p,
                    &format!("E28 decompressStream(empty input, call {i})"),
                    c_ds(cd, &mut cob, &mut cin),
                    r_ds(rd, &mut rob, &mut rin),
                );
            }
            c_dfree(cd);
            r_dfree(rd);
        }
        // E29 — repeated zero-progress calls with a full output buffer
        {
            let cd = c_dnew();
            let rd = r_dnew();
            let mut cb = vec![0u8; 16];
            let mut rb = vec![0u8; 16];
            for i in 0..40 {
                let mut cin = InBuffer { src: frame.as_ptr() as *const c_void, size: frame.len(), pos: 0 };
                let mut rin = InBuffer { src: frame.as_ptr() as *const c_void, size: frame.len(), pos: 0 };
                let mut cob = OutBuffer { dst: cb.as_mut_ptr() as *mut c_void, size: 0, pos: 0 };
                let mut rob = OutBuffer { dst: rb.as_mut_ptr() as *mut c_void, size: 0, pos: 0 };
                eq_err(
                    p,
                    &format!("E29 decompressStream(full output, call {i})"),
                    c_ds(cd, &mut cob, &mut cin),
                    r_ds(rd, &mut rob, &mut rin),
                );
            }
            c_dfree(cd);
            r_dfree(rd);
        }
        // E36 — stableOutBuffer then the output buffer changes
        for stable in [0, 1] {
            let cd = c_dnew();
            let rd = r_dnew();
            c_reset(cd, 2);
            r_reset(rd, 2);
            eq(
                &format!("set d-stableOutBuffer={stable}"),
                c_dset(cd, DP_STABLEOUT, stable),
                r_dset(rd, DP_STABLEOUT, stable),
            );
            let mut cb1 = vec![0u8; 1024];
            let mut rb1 = vec![0u8; 1024];
            let mut cb2 = vec![0u8; 1024];
            let mut rb2 = vec![0u8; 1024];
            let mut cin = InBuffer { src: frame.as_ptr() as *const c_void, size: 512, pos: 0 };
            let mut rin = InBuffer { src: frame.as_ptr() as *const c_void, size: 512, pos: 0 };
            let mut cob = OutBuffer { dst: cb1.as_mut_ptr() as *mut c_void, size: cb1.len(), pos: 0 };
            let mut rob = OutBuffer { dst: rb1.as_mut_ptr() as *mut c_void, size: rb1.len(), pos: 0 };
            eq_err(
                p,
                &format!("E36 first call(stable={stable})"),
                c_ds(cd, &mut cob, &mut cin),
                r_ds(rd, &mut rob, &mut rin),
            );
            let mut cin2 = InBuffer { src: frame[512..].as_ptr() as *const c_void, size: frame.len() - 512, pos: 0 };
            let mut rin2 = InBuffer { src: frame[512..].as_ptr() as *const c_void, size: frame.len() - 512, pos: 0 };
            let mut cob2 = OutBuffer { dst: cb2.as_mut_ptr() as *mut c_void, size: cb2.len(), pos: 0 };
            let mut rob2 = OutBuffer { dst: rb2.as_mut_ptr() as *mut c_void, size: rb2.len(), pos: 0 };
            eq_err(
                p,
                &format!("E36 moved out buffer(stable={stable})"),
                c_ds(cd, &mut cob2, &mut cin2),
                r_ds(rd, &mut rob2, &mut rin2),
            );
            c_dfree(cd);
            r_dfree(rd);
        }
    }
}

/// E30 — a DCtx reused after an error without a reset.
#[test]
fn e30_dctx_reuse_after_error() {
    let p = libs();
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_DCtx_reset");
    let (c_dd, r_dd) = p.sym::<F2>("ZSTD_decompressDCtx");
    let mut rng = Rng::new(SEED ^ 0xE30);
    unsafe {
        let src = gen(Shape::TextLike, 4096, &mut rng);
        let good = make_frame(p, &src, 3, 1, 1);
        let mut bad = good.clone();
        bad[good.len() / 2] ^= 0xFF;
        let cd = c_dnew();
        let rd = r_dnew();
        for round in 0..4 {
            for (nm, f) in [("bad", &bad), ("good", &good), ("truncated", &good)] {
                let n = if nm == "truncated" { f.len() / 2 } else { f.len() };
                let cap = src.len() + 64;
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                eq_err(
                    p,
                    &format!("E30 round {round} {nm} (no reset)"),
                    c_dd(cd, cb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, n),
                    r_dd(rd, rb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, n),
                );
                eq_bytes(&format!("E30 round {round} {nm} dst image"), &cb, &rb);
            }
            // and with an explicit reset in between
            for dir in [0, 1, 2] {
                eq_err(p, &format!("E30 DCtx_reset({dir})"), c_reset(cd, dir), r_reset(rd, dir));
            }
        }
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// E32/E33/E34 — skippable-frame read/write rejections.
#[test]
fn e32_e34_skippable_frame_rejections() {
    let p = libs();
    type FWS = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz, c_uint) -> Sz;
    type FRS = unsafe extern "C" fn(*mut c_void, Sz, *mut c_uint, *const c_void, Sz) -> Sz;
    let (c_ws, r_ws) = p.sym::<FWS>("ZSTD_writeSkippableFrame");
    let (c_rs, r_rs) = p.sym::<FRS>("ZSTD_readSkippableFrame");
    let mut rng = Rng::new(SEED ^ 0xE32);
    unsafe {
        // E33/E34 — undersized dst and out-of-range magicVariant
        for len in [0usize, 1, 7, 8, 1024] {
            let payload = gen(Shape::Incompressible, len, &mut rng);
            let pp = if len == 0 { std::ptr::null() } else { payload.as_ptr() as *const c_void };
            for mv in [0u32, 1, 15, 16, 17, 99, u32::MAX] {
                for &cap in &[0usize, 1, 7, 8, len, len + 7, len + 8, len + 64] {
                    let mut cb = vec![0u8; cap.max(1)];
                    let mut rb = vec![0u8; cap.max(1)];
                    eq_err(
                        p,
                        &format!("E33/E34 writeSkippableFrame(len={len},mv={mv},cap={cap})"),
                        c_ws(cb.as_mut_ptr() as *mut c_void, cap, pp, len, mv),
                        r_ws(rb.as_mut_ptr() as *mut c_void, cap, pp, len, mv),
                    );
                    eq_bytes("E33 dst image", &cb, &rb);
                }
            }
        }
        // E32 — readSkippableFrame on non-skippable input
        let src = gen(Shape::TextLike, 1024, &mut rng);
        let real = make_frame(p, &src, 3, 0, 1);
        let mut inputs: Vec<(String, Vec<u8>)> = vec![
            ("empty".into(), Vec::new()),
            ("zeros8".into(), vec![0u8; 8]),
            ("real-frame".into(), real),
            ("garbage".into(), gen(Shape::Incompressible, 64, &mut rng)),
        ];
        for mv in [0u32, 5, 15] {
            let payload = gen(Shape::TextLike, 32, &mut rng);
            let mut b = vec![0u8; 64];
            let n = c_ws(b.as_mut_ptr() as *mut c_void, 64, payload.as_ptr() as *const c_void, 32, mv);
            let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
            if c_ie(n) == 0 {
                b.truncate(n);
                inputs.push((format!("skippable(mv={mv})"), b));
            }
        }
        for (name, v) in &inputs {
            for cut in 0..=v.len().min(20) {
                let vp = if v.is_empty() { std::ptr::null() } else { v.as_ptr() as *const c_void };
                for &cap in &[0usize, 1, 16, 64, 4096] {
                    let mut cb = vec![0u8; cap.max(1)];
                    let mut rb = vec![0u8; cap.max(1)];
                    let mut cmv = 0xAAAA_AAAAu32;
                    let mut rmv = 0xAAAA_AAAAu32;
                    eq_err(
                        p,
                        &format!("E32 readSkippableFrame({name},cut={cut},cap={cap})"),
                        c_rs(cb.as_mut_ptr() as *mut c_void, cap, &mut cmv, vp, cut),
                        r_rs(rb.as_mut_ptr() as *mut c_void, cap, &mut rmv, vp, cut),
                    );
                    eq(&format!("E32 magicVariant({name},cut={cut},cap={cap})"), cmv, rmv);
                    eq_bytes("E32 dst image", &cb, &rb);
                }
            }
        }
    }
}

/// M5 — out-of-range enum ints across EVERY enum-taking public entry point.
#[test]
fn m5_out_of_range_enums() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_dset, r_dset) = p.sym::<FnSetParam>("ZSTD_DCtx_setParameter");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    let (c_dreset, r_dreset) = p.sym::<FnReset>("ZSTD_DCtx_reset");

    // every enum-valued cParameter, and every "bad" int for it
    const ENUM_PARAMS: [(c_int, &str); 10] = [
        (107, "strategy"),
        (10, "format"),
        (1001, "forceAttachDict"),
        (1002, "literalCompressionMode"),
        (1006, "stableInBuffer"),
        (1007, "stableOutBuffer"),
        (1008, "blockDelimiters"),
        (1010, "splitAfterSequences"),
        (1011, "useRowMatchFinder"),
        (1013, "prefetchCDictTables"),
    ];
    const ENUM_DPARAMS: [(c_int, &str); 4] = [
        (1000, "d_format"),
        (1001, "d_stableOutBuffer"),
        (1002, "d_forceIgnoreChecksum"),
        (1003, "d_refMultipleDDicts"),
    ];
    let bad_vals: [c_int; 10] = [-1, -2, 4, 5, 10, 99, 1000, -1000, c_int::MIN, c_int::MAX];

    unsafe {
        let cc = c_new();
        let rc = r_new();
        for (prm, name) in ENUM_PARAMS {
            for v in bad_vals {
                eq_err(
                    p,
                    &format!("M5 CCtx_setParameter({name}={v})"),
                    c_set(cc, prm, v),
                    r_set(rc, prm, v),
                );
            }
        }
        for d in bad_vals.iter().copied().chain([0, 1, 2]) {
            eq_err(p, &format!("M5 CCtx_reset({d})"), c_reset(cc, d), r_reset(rc, d));
        }
        c_free(cc);
        r_free(rc);

        let cd = c_dnew();
        let rd = r_dnew();
        for (prm, name) in ENUM_DPARAMS {
            for v in bad_vals {
                eq_err(
                    p,
                    &format!("M5 DCtx_setParameter({name}={v})"),
                    c_dset(cd, prm, v),
                    r_dset(rd, prm, v),
                );
            }
        }
        for d in bad_vals.iter().copied().chain([0, 1, 2]) {
            eq_err(p, &format!("M5 DCtx_reset({d})"), c_dreset(cd, d), r_dreset(rd, d));
        }
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// M6 — zero-length everything on every size-taking public entry point.
#[test]
fn m6_zero_lengths() {
    let p = libs();
    let (c_co, r_co) = p.sym::<FnCompress>("ZSTD_compress");
    let (c_de, r_de) = p.sym::<FnDecompress>("ZSTD_decompress");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    unsafe {
        // src = NULL, srcSize = 0, dst = NULL, dstCapacity = 0
        eq_err(
            p,
            "M6 ZSTD_compress(NULL,0,NULL,0)",
            c_co(std::ptr::null_mut(), 0, std::ptr::null(), 0, 3),
            r_co(std::ptr::null_mut(), 0, std::ptr::null(), 0, 3),
        );
        eq_err(
            p,
            "M6 ZSTD_decompress(NULL,0,NULL,0)",
            c_de(std::ptr::null_mut(), 0, std::ptr::null(), 0),
            r_de(std::ptr::null_mut(), 0, std::ptr::null(), 0),
        );
        // empty frame round trip: compress 0 bytes then decompress into 0 bytes
        let cap = c_cb(0) + 64;
        let mut cb = vec![0u8; cap];
        let mut rb = vec![0u8; cap];
        let a = c_co(cb.as_mut_ptr() as *mut c_void, cap, std::ptr::null(), 0, 3);
        let b = r_co(rb.as_mut_ptr() as *mut c_void, cap, std::ptr::null(), 0, 3);
        eq("M6 empty compress ret", a, b);
        let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
        if c_ie(a) == 0 {
            eq_bytes("M6 empty frame", &cb[..a], &rb[..b]);
            eq_err(
                p,
                "M6 decompress(empty frame, dst=NULL, cap=0)",
                c_de(std::ptr::null_mut(), 0, cb.as_ptr() as *const c_void, a),
                r_de(std::ptr::null_mut(), 0, rb.as_ptr() as *const c_void, b),
            );
        }
    }
}

/// M11 — the `ZSTD_XXH*` accessors with NULL input and zero length.
#[test]
fn m11_xxh_null_and_zero() {
    let p = libs();
    type F32 = unsafe extern "C" fn(*const c_void, Sz, u32) -> u32;
    type F64 = unsafe extern "C" fn(*const c_void, Sz, u64) -> u64;
    let (c32, r32) = p.sym::<F32>("ZSTD_XXH32");
    let (c64, r64) = p.sym::<F64>("ZSTD_XXH64");
    unsafe {
        for seed in [0u32, 1, u32::MAX] {
            eq(
                &format!("M11 ZSTD_XXH32(NULL,0,{seed})"),
                c32(std::ptr::null(), 0, seed),
                r32(std::ptr::null(), 0, seed),
            );
        }
        for seed in [0u64, 1, u64::MAX] {
            eq(
                &format!("M11 ZSTD_XXH64(NULL,0,{seed})"),
                c64(std::ptr::null(), 0, seed),
                r64(std::ptr::null(), 0, seed),
            );
        }
    }
}

/// D7 — `ZSTD_compress2` with `dstCapacity` below what the frame needs.
#[test]
fn d7_compress2_dst_too_small() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_c2, r_c2) = p.sym::<F2>("ZSTD_compress2");
    let mut rng = Rng::new(SEED ^ 0xD7);
    unsafe {
        for stable_out in [0, 1] {
            for level in [1, 9, 19] {
                for shape in [Shape::Incompressible, Shape::TextLike, Shape::Zeros] {
                    for len in [1usize, 1024, 70_000] {
                        let src = gen(shape, len, &mut rng);
                        let full = c_cb(len);
                        for &cap in &[0usize, 1, 8, 18, full / 4, full / 2, full - 1, full] {
                            let cc = c_new();
                            let rc = r_new();
                            c_reset(cc, 2);
                            r_reset(rc, 2);
                            eq("set level", c_set(cc, P_LEVEL, level), r_set(rc, P_LEVEL, level));
                            eq(
                                "set stableOutBuffer",
                                c_set(cc, P_STABLEOUT, stable_out),
                                r_set(rc, P_STABLEOUT, stable_out),
                            );
                            let mut cb = vec![0u8; cap.max(1)];
                            let mut rb = vec![0u8; cap.max(1)];
                            let a = c_c2(cc, cb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                            let b = r_c2(rc, rb.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                            eq_err(
                                p,
                                &format!("D7 compress2(so={stable_out},lvl={level},shape={shape:?},len={len},cap={cap})"),
                                a, b,
                            );
                            eq_bytes("D7 dst image", &cb, &rb);
                            c_free(cc);
                            r_free(rc);
                        }
                    }
                }
            }
        }
    }
}

/// F12–F15 — dictionary APIs called mid-frame must all report `stage_wrong`.
#[test]
fn f12_f15_dict_apis_mid_frame() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    let (c_dreset, r_dreset) = p.sym::<FnReset>("ZSTD_DCtx_reset");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FCS2 = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> Sz;
    type FDS = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> Sz;
    type FLoad = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;
    type FRef = unsafe extern "C" fn(*mut c_void, *const c_void) -> Sz;
    type FCreateCDict = unsafe extern "C" fn(*const c_void, Sz, c_int) -> *mut c_void;
    type FCreateDDict = unsafe extern "C" fn(*const c_void, Sz) -> *mut c_void;
    let (c_cs2, r_cs2) = p.sym::<FCS2>("ZSTD_compressStream2");
    let (c_ds, r_ds) = p.sym::<FDS>("ZSTD_decompressStream");
    let (c_cload, r_cload) = p.sym::<FLoad>("ZSTD_CCtx_loadDictionary");
    let (c_cpref, r_cpref) = p.sym::<FLoad>("ZSTD_CCtx_refPrefix");
    let (c_refcd, r_refcd) = p.sym::<FRef>("ZSTD_CCtx_refCDict");
    let (c_dload, r_dload) = p.sym::<FLoad>("ZSTD_DCtx_loadDictionary");
    let (c_dpref, r_dpref) = p.sym::<FLoad>("ZSTD_DCtx_refPrefix");
    let (c_refdd, r_refdd) = p.sym::<FRef>("ZSTD_DCtx_refDDict");
    let (c_ccd, r_ccd) = p.sym::<FCreateCDict>("ZSTD_createCDict");
    let (c_fcd, r_fcd) = p.sym::<FnFreeCtx>("ZSTD_freeCDict");
    let (c_cdd, r_cdd) = p.sym::<FCreateDDict>("ZSTD_createDDict");
    let (c_fdd, r_fdd) = p.sym::<FnFreeCtx>("ZSTD_freeDDict");

    let mut rng = Rng::new(SEED ^ 0xF12);
    unsafe {
        let dict = gen(Shape::TextLike, 4096, &mut rng);
        let dp = dict.as_ptr() as *const c_void;
        let ccd = c_ccd(dp, dict.len(), 3);
        let rcd = r_ccd(dp, dict.len(), 3);
        let cdd = c_cdd(dp, dict.len());
        let rdd = r_cdd(dp, dict.len());

        // ---- compression side: start a frame, then try each dict API ----
        let src = gen(Shape::TextLike, 300_000, &mut rng);
        let cc = c_new();
        let rc = r_new();
        c_reset(cc, 2);
        r_reset(rc, 2);
        let mut cb = vec![0u8; 1024];
        let mut rb = vec![0u8; 1024];
        let mut cin = InBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
        let mut rin = InBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
        let mut cob = OutBuffer { dst: cb.as_mut_ptr() as *mut c_void, size: cb.len(), pos: 0 };
        let mut rob = OutBuffer { dst: rb.as_mut_ptr() as *mut c_void, size: rb.len(), pos: 0 };
        let a = c_cs2(cc, &mut cob, &mut cin, 0);
        let b = r_cs2(rc, &mut rob, &mut rin, 0);
        eq_err(p, "F12-F14 start frame", a, b);
        assert!(c_ie(a) == 0, "F12-F14 setup: could not start a frame");
        eq_err(p, "F12 CCtx_loadDictionary mid-frame", c_cload(cc, dp, dict.len()), r_cload(rc, dp, dict.len()));
        eq_err(p, "F14 CCtx_refPrefix mid-frame", c_cpref(cc, dp, dict.len()), r_cpref(rc, dp, dict.len()));
        eq_err(p, "F13 CCtx_refCDict mid-frame", c_refcd(cc, ccd), r_refcd(rc, rcd));
        eq_err(p, "F13 CCtx_refCDict(NULL) mid-frame", c_refcd(cc, std::ptr::null()), r_refcd(rc, std::ptr::null()));
        c_free(cc);
        r_free(rc);

        // ---- decompression side ----
        let cap = c_cb(src.len()) + 64;
        let mut frame = vec![0u8; cap];
        {
            let (c_c2, _) = p.sym::<F2>("ZSTD_compress2");
            let cc = c_new();
            let n = c_c2(cc, frame.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            c_free(cc);
            assert!(c_ie(n) == 0);
            frame.truncate(n);
        }
        let cd = c_dnew();
        let rd = r_dnew();
        c_dreset(cd, 2);
        r_dreset(rd, 2);
        let mut cout = vec![0u8; 1024];
        let mut rout = vec![0u8; 1024];
        let mut cin = InBuffer { src: frame.as_ptr() as *const c_void, size: frame.len(), pos: 0 };
        let mut rin = InBuffer { src: frame.as_ptr() as *const c_void, size: frame.len(), pos: 0 };
        let mut cob = OutBuffer { dst: cout.as_mut_ptr() as *mut c_void, size: cout.len(), pos: 0 };
        let mut rob = OutBuffer { dst: rout.as_mut_ptr() as *mut c_void, size: rout.len(), pos: 0 };
        let a = c_ds(cd, &mut cob, &mut cin);
        let b = r_ds(rd, &mut rob, &mut rin);
        eq_err(p, "F15 start decode", a, b);
        eq_err(p, "F15 DCtx_loadDictionary mid-frame", c_dload(cd, dp, dict.len()), r_dload(rd, dp, dict.len()));
        eq_err(p, "F15 DCtx_refPrefix mid-frame", c_dpref(cd, dp, dict.len()), r_dpref(rd, dp, dict.len()));
        eq_err(p, "F15 DCtx_refDDict mid-frame", c_refdd(cd, cdd), r_refdd(rd, rdd));
        eq_err(p, "F15 DCtx_refDDict(NULL) mid-frame", c_refdd(cd, std::ptr::null()), r_refdd(rd, std::ptr::null()));
        c_dfree(cd);
        r_dfree(rd);

        c_fcd(ccd);
        r_fcd(rcd);
        c_fdd(cdd);
        r_fdd(rdd);
    }
}

/// G9 — `ZSTD_sizeof_*` on FRESH and on USED contexts (the value grows as the
/// context allocates its tables, so a used context is a distinct observation).
#[test]
fn g9_sizeof_fresh_and_used() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_cnew, r_cnew) = p.sym::<FnCreateCtx>("ZSTD_createCStream");
    let (c_cfree, r_cfree) = p.sym::<FnFreeCtx>("ZSTD_freeCStream");
    let (c_snew, r_snew) = p.sym::<FnCreateCtx>("ZSTD_createDStream");
    let (c_sfree, r_sfree) = p.sym::<FnFreeCtx>("ZSTD_freeDStream");
    type FSizeof = unsafe extern "C" fn(*const c_void) -> Sz;
    let (c_szc, r_szc) = p.sym::<FSizeof>("ZSTD_sizeof_CCtx");
    let (c_szd, r_szd) = p.sym::<FSizeof>("ZSTD_sizeof_DCtx");
    let (c_szcs, r_szcs) = p.sym::<FSizeof>("ZSTD_sizeof_CStream");
    let (c_szds, r_szds) = p.sym::<FSizeof>("ZSTD_sizeof_DStream");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_c2, r_c2) = p.sym::<F2>("ZSTD_compress2");
    let (c_dd, r_dd) = p.sym::<F2>("ZSTD_decompressDCtx");

    let mut rng = Rng::new(SEED ^ 0x69);
    unsafe {
        let cc = c_new();
        let rc = r_new();
        let cd = c_dnew();
        let rd = r_dnew();
        let ccs = c_cnew();
        let rcs = r_cnew();
        let cds = c_snew();
        let rds = r_snew();
        eq("G9 sizeof_CCtx fresh", c_szc(cc), r_szc(rc));
        eq("G9 sizeof_DCtx fresh", c_szd(cd), r_szd(rd));
        eq("G9 sizeof_CStream fresh", c_szcs(ccs), r_szcs(rcs));
        eq("G9 sizeof_DStream fresh", c_szds(cds), r_szds(rds));
        for level in [1, 3, 9, 19] {
            for shape in [Shape::TextLike, Shape::Incompressible, Shape::Repetitive] {
                for len in [1usize, 4096, 200_000] {
                    let src = gen(shape, len, &mut rng);
                    eq("G9 set level", c_set(cc, P_LEVEL, level), r_set(rc, P_LEVEL, level));
                    let cap = c_cb(len) + 64;
                    let mut cf = vec![0u8; cap];
                    let mut rf = vec![0u8; cap];
                    let a = c_c2(cc, cf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                    let b = r_c2(rc, rf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                    eq("G9 compress2 ret", a, b);
                    eq(
                        &format!("G9 sizeof_CCtx used(lvl={level},shape={shape:?},len={len})"),
                        c_szc(cc),
                        r_szc(rc),
                    );
                    if c_ie(a) != 0 {
                        continue;
                    }
                    let mut cb = vec![0u8; len + 64];
                    let mut rb = vec![0u8; len + 64];
                    let x = c_dd(cd, cb.as_mut_ptr() as *mut c_void, cb.len(), cf.as_ptr() as *const c_void, a);
                    let y = r_dd(rd, rb.as_mut_ptr() as *mut c_void, rb.len(), rf.as_ptr() as *const c_void, b);
                    eq("G9 decompressDCtx ret", x, y);
                    eq(
                        &format!("G9 sizeof_DCtx used(lvl={level},shape={shape:?},len={len})"),
                        c_szd(cd),
                        r_szd(rd),
                    );
                }
            }
        }
        c_free(cc);
        r_free(rc);
        c_dfree(cd);
        r_dfree(rd);
        c_cfree(ccs);
        r_cfree(rcs);
        c_sfree(cds);
        r_sfree(rds);
    }
}
