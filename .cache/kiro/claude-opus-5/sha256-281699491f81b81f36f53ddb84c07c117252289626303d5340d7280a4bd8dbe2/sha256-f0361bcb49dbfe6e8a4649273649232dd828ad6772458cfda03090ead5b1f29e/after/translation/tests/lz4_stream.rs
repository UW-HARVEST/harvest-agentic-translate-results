//! Phase B — CONFIGS.md group 3: `lz4.c` streaming compression and
//! streaming decompression, driven through the low-level entry points.
mod common;
use common::*;

const SEED: u64 = 0x33_5EED_0003;

type FnCreateStream = unsafe extern "C" fn() -> *mut u8;
type FnFreeStream = unsafe extern "C" fn(*mut u8) -> i32;
type FnInitStream = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;
type FnResetStream = unsafe extern "C" fn(*mut u8);
type FnLoadDict = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
type FnAttach = unsafe extern "C" fn(*mut u8, *const u8);
type FnSaveDict = unsafe extern "C" fn(*mut u8, *mut u8, i32) -> i32;
type FnContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnForceExt = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;
type FnCreateStreamDecode = unsafe extern "C" fn() -> *mut u8;
type FnSetStreamDecode = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
type FnDecContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
type FnDecFastContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;

struct Streams {
    c: *mut u8,
    r: *mut u8,
    free: (FnFreeStream, FnFreeStream),
}
impl Drop for Streams {
    fn drop(&mut self) {
        unsafe {
            (self.free.0)(self.c);
            (self.free.1)(self.r);
        }
    }
}
fn new_streams() -> Streams {
    let (cc, rc) = syms::<FnCreateStream>("LZ4_createStream");
    let free = syms::<FnFreeStream>("LZ4_freeStream");
    let s = Streams { c: unsafe { cc() }, r: unsafe { rc() }, free };
    assert!(!s.c.is_null() && !s.r.is_null());
    s
}

/// Row 42-44: contiguous prefix chaining and non-contiguous (extDict) chaining,
/// across acceleration classes.
#[test]
fn g3_compress_fast_continue_chaining() {
    let (cc, rc) = syms::<FnContinue>("LZ4_compress_fast_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED);

    for &acc in &[1i32, 0, 5, 65537] {
        for &shape in ALL_SHAPES.iter() {
            for &contiguous in &[true, false] {
                let s = new_streams();
                // One big contiguous buffer -> prefix mode.
                // Separate per-chunk buffers -> usingExtDict mode.
                let total = mkdata(shape, 150_000, &mut rng);
                let mut off = 0usize;
                let mut chunks: Vec<Vec<u8>> = Vec::new();
                while off < total.len() {
                    let n = rng.range(1, 9000).min(total.len() - off);
                    chunks.push(total[off..off + n].to_vec());
                    off += n;
                }
                let mut k = 0;
                let mut off = 0usize;
                for chunk in &chunks {
                    let n = chunk.len();
                    let bound = unsafe { cb(n as i32) } as usize;
                    let mut cd = vec![0u8; bound];
                    let mut rd = vec![0u8; bound];
                    let (cp, rp): (*const u8, *const u8) = if contiguous {
                        unsafe { (total.as_ptr().add(off), total.as_ptr().add(off)) }
                    } else {
                        (chunk.as_ptr(), chunk.as_ptr())
                    };
                    let cn = unsafe { cc(s.c, cp, cd.as_mut_ptr(), n as i32, bound as i32, acc) };
                    let rn = unsafe { rc(s.r, rp, rd.as_mut_ptr(), n as i32, bound as i32, acc) };
                    same(
                        &format!("continue contig={contiguous} acc={acc} {shape:?} k={k} n={n}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                    off += n;
                    k += 1;
                }
            }
        }
    }
}

/// Row 45 + 63: ring buffer with wraparound on the compression side, verified
/// by round-tripping through the streaming decoder on both sides.
#[test]
fn g3_ring_buffer() {
    let (cc, rc) = syms::<FnContinue>("LZ4_compress_fast_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let (crb, rrb) = syms::<FnCompressBound>("LZ4_decoderRingBufferSize");
    let mut rng = Rng::new(SEED ^ 0x1);

    for &msg_max in &[64usize, 1000, 9000, 70000] {
        let rb_c = unsafe { crb(msg_max as i32) };
        assert_eq!(rb_c, unsafe { rrb(msg_max as i32) });
        for &shape in &[Shape::Textish, Shape::Periodic, Shape::Random] {
            let s = new_streams();
            let mut cring = vec![0u8; rb_c as usize];
            let mut rring = vec![0u8; rb_c as usize];
            let mut pos = 0usize;
            for k in 0..40 {
                let n = rng.range(1, msg_max);
                if pos + n > cring.len() {
                    pos = 0;
                }
                let msg = mkdata(shape, n, &mut rng);
                cring[pos..pos + n].copy_from_slice(&msg);
                rring[pos..pos + n].copy_from_slice(&msg);
                let bound = unsafe { cb(n as i32) } as usize;
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe {
                    cc(s.c, cring.as_ptr().add(pos), cd.as_mut_ptr(), n as i32, bound as i32, 1)
                };
                let rn = unsafe {
                    rc(s.r, rring.as_ptr().add(pos), rd.as_mut_ptr(), n as i32, bound as i32, 1)
                };
                same(
                    &format!("ring msg_max={msg_max} {shape:?} k={k} n={n} pos={pos}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
                pos += n;
            }
        }
    }
}

/// Row 46-47: `LZ4_initStream` on a user buffer, plus `LZ4_resetStream` and
/// `LZ4_resetStream_fast`.
#[test]
fn g3_init_and_reset_stream() {
    let (ci, ri) = syms::<FnInitStream>("LZ4_initStream");
    let (crs, rrs) = syms::<FnResetStream>("LZ4_resetStream");
    let (crf, rrf) = syms::<FnResetStream>("LZ4_resetStream_fast");
    let (cc, rc) = syms::<FnContinue>("LZ4_compress_fast_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    type FnSizeofStreamState = unsafe extern "C" fn() -> i32;
    let (csz, _) = syms::<FnSizeofStreamState>("LZ4_sizeofStreamState");
    let sz = unsafe { csz() } as usize;

    let mut rng = Rng::new(SEED ^ 0x2);
    // 8-byte aligned scratch of exactly the right size.
    let mut cbuf = vec![0u64; sz / 8 + 2];
    let mut rbuf = vec![0u64; sz / 8 + 2];
    let cp = unsafe { ci(cbuf.as_mut_ptr() as *mut u8, sz) };
    let rp = unsafe { ri(rbuf.as_mut_ptr() as *mut u8, sz) };
    assert_eq!(cp.is_null(), rp.is_null(), "initStream nullness");
    assert!(!cp.is_null());
    assert_eq!(cp as usize - cbuf.as_ptr() as usize, rp as usize - rbuf.as_ptr() as usize);

    for reset in 0..3 {
        // 0 = no reset, 1 = resetStream, 2 = resetStream_fast
        if reset == 1 {
            unsafe { crs(cp) };
            unsafe { rrs(rp) };
        } else if reset == 2 {
            unsafe { crf(cp) };
            unsafe { rrf(rp) };
        }
        let total = mkdata(Shape::Textish, 60000, &mut rng);
        let mut off = 0usize;
        let mut k = 0;
        while off < total.len() {
            let n = rng.range(1, 7000).min(total.len() - off);
            let bound = unsafe { cb(n as i32) } as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let cn = unsafe {
                cc(cp, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32, bound as i32, 1)
            };
            let rn = unsafe {
                rc(rp, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32, bound as i32, 1)
            };
            same(&format!("initStream reset={reset} k={k} n={n}"), cn as i64, &cd, rn as i64, &rd);
            off += n;
            k += 1;
        }
        // The whole user-visible state must stay bit-identical.
        assert_eq!(&cbuf[..sz / 8], &rbuf[..sz / 8], "initStream state diverged (reset={reset})");
    }
}

/// Rows 48-49: `LZ4_loadDict` and `LZ4_loadDictSlow` across the dictSize matrix.
#[test]
fn g3_load_dict() {
    let (cc, rc) = syms::<FnContinue>("LZ4_compress_fast_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x3);

    for name in ["LZ4_loadDict", "LZ4_loadDictSlow"] {
        let (cl, rl) = syms::<FnLoadDict>(name);
        for &ds in &[0usize, 1, 2, 3, 4, 5, 100, 1000, 65535, 65536, 70000, 200000] {
            for &shape in &[Shape::Textish, Shape::Periodic, Shape::SmallAlphabet, Shape::Random] {
                let dict = mkdata(shape, ds, &mut rng);
                let s = new_streams();
                let a = unsafe { cl(s.c, dict.as_ptr(), ds as i32) };
                let b = unsafe { rl(s.r, dict.as_ptr(), ds as i32) };
                assert_eq!(a, b, "{name}(ds={ds}) return C={a} R={b}");
                let mut arena = BlockArena::new();
                for k in 0..4 {
                    let n = rng.range(1, 6000);
                    // half the time reuse dictionary content so matches occur
                    let blk = arena.keep(if ds > 32 && k % 2 == 0 {
                        let take = n.min(ds);
                        let st = rng.below(ds - take + 1);
                        let mut v = dict[st..st + take].to_vec();
                        v.extend(mkdata(shape, n - take, &mut rng));
                        v
                    } else {
                        mkdata(shape, n, &mut rng)
                    });
                    let bound = unsafe { cb(blk.len() as i32) } as usize;
                    let mut cd = vec![0u8; bound];
                    let mut rd = vec![0u8; bound];
                    let cn = unsafe {
                        cc(s.c, blk.as_ptr(), cd.as_mut_ptr(), blk.len() as i32, bound as i32, 1)
                    };
                    let rn = unsafe {
                        rc(s.r, blk.as_ptr(), rd.as_mut_ptr(), blk.len() as i32, bound as i32, 1)
                    };
                    same(
                        &format!("{name} ds={ds} {shape:?} k={k} n={}", blk.len()),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
            }
        }
    }
}

/// Rows 50-52: `LZ4_attach_dictionary` (dictCtx path, the >4 KB cold-start
/// threshold, and detaching with NULL).
#[test]
fn g3_attach_dictionary() {
    let (cl, rl) = syms::<FnLoadDict>("LZ4_loadDict");
    let (ca, ra) = syms::<FnAttach>("LZ4_attach_dictionary");
    let (cc, rc) = syms::<FnContinue>("LZ4_compress_fast_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x4);

    for &ds in &[0usize, 1, 4, 100, 5000, 65536, 70000] {
        for &first_chunk in &[1usize, 100, 4095, 4096, 4097, 9000] {
            for &shape in &[Shape::Textish, Shape::Periodic, Shape::Random] {
                let dict = mkdata(shape, ds, &mut rng);
                // dictionary stream
                let dstream = new_streams();
                unsafe { cl(dstream.c, dict.as_ptr(), ds as i32) };
                unsafe { rl(dstream.r, dict.as_ptr(), ds as i32) };
                // working stream, dictionary attached
                let w = new_streams();
                unsafe { ca(w.c, dstream.c) };
                unsafe { ra(w.r, dstream.r) };

                let mut lens = vec![first_chunk];
                for _ in 0..3 {
                    lens.push(rng.range(1, 6000));
                }
                let mut arena = BlockArena::new();
                for (k, &n) in lens.iter().enumerate() {
                    let blk = arena.keep(if ds > 32 {
                        let take = n.min(ds);
                        let st = rng.below(ds - take + 1);
                        let mut v = dict[st..st + take].to_vec();
                        v.extend(mkdata(shape, n - take, &mut rng));
                        v
                    } else {
                        mkdata(shape, n, &mut rng)
                    });
                    let bound = unsafe { cb(blk.len() as i32) } as usize;
                    let mut cd = vec![0u8; bound];
                    let mut rd = vec![0u8; bound];
                    let cn = unsafe {
                        cc(w.c, blk.as_ptr(), cd.as_mut_ptr(), blk.len() as i32, bound as i32, 1)
                    };
                    let rn = unsafe {
                        rc(w.r, blk.as_ptr(), rd.as_mut_ptr(), blk.len() as i32, bound as i32, 1)
                    };
                    same(
                        &format!("attach ds={ds} first={first_chunk} {shape:?} k={k}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
            }
        }
    }

    // Row 52: detach with NULL, then compress
    for &shape in &[Shape::Textish, Shape::Random] {
        let dict = mkdata(shape, 5000, &mut rng);
        let dstream = new_streams();
        unsafe { cl(dstream.c, dict.as_ptr(), 5000) };
        unsafe { rl(dstream.r, dict.as_ptr(), 5000) };
        let w = new_streams();
        unsafe { ca(w.c, dstream.c) };
        unsafe { ra(w.r, dstream.r) };
        unsafe { ca(w.c, std::ptr::null()) };
        unsafe { ra(w.r, std::ptr::null()) };
        let blk = mkdata(shape, 3000, &mut rng);
        let bound = unsafe { cb(3000) } as usize;
        let mut cd = vec![0u8; bound];
        let mut rd = vec![0u8; bound];
        let cn = unsafe { cc(w.c, blk.as_ptr(), cd.as_mut_ptr(), 3000, bound as i32, 1) };
        let rn = unsafe { rc(w.r, blk.as_ptr(), rd.as_mut_ptr(), 3000, bound as i32, 1) };
        same(&format!("attach-detach {shape:?}"), cn as i64, &cd, rn as i64, &rd);
    }
}

/// Row 53: `LZ4_saveDict` across the dictSize matrix, then continue compressing.
#[test]
fn g3_save_dict() {
    let (cs, rs) = syms::<FnSaveDict>("LZ4_saveDict");
    let (cc, rc) = syms::<FnContinue>("LZ4_compress_fast_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x5);

    for &ds in &[0usize, 1, 3, 4, 100, 65535, 65536, 70000] {
        for &shape in &[Shape::Textish, Shape::Periodic, Shape::Random] {
            let s = new_streams();
            let total = mkdata(shape, 100_000, &mut rng);
            // first pass over a contiguous buffer
            let mut off = 0usize;
            while off < 50_000 {
                let n = 7000usize.min(50_000 - off);
                let bound = unsafe { cb(n as i32) } as usize;
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe {
                    cc(s.c, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32, bound as i32, 1)
                };
                let rn = unsafe {
                    rc(s.r, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32, bound as i32, 1)
                };
                same(&format!("saveDict pre ds={ds} {shape:?}"), cn as i64, &cd, rn as i64, &rd);
                off += n;
            }
            let mut csafe = vec![0u8; ds.max(1)];
            let mut rsafe = vec![0u8; ds.max(1)];
            let a = unsafe { cs(s.c, csafe.as_mut_ptr(), ds as i32) };
            let b = unsafe { rs(s.r, rsafe.as_mut_ptr(), ds as i32) };
            assert_eq!(a, b, "saveDict(ds={ds}) C={a} R={b} {shape:?}");
            assert_eq!(csafe, rsafe, "saveDict buffer contents ds={ds} {shape:?}");
            // continue compressing from the saved dictionary
            let mut arena = BlockArena::new();
            for k in 0..3 {
                let n = rng.range(1, 6000);
                let blk = arena.keep(mkdata(shape, n, &mut rng));
                let bound = unsafe { cb(n as i32) } as usize;
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe { cc(s.c, blk.as_ptr(), cd.as_mut_ptr(), n as i32, bound as i32, 1) };
                let rn = unsafe { rc(s.r, blk.as_ptr(), rd.as_mut_ptr(), n as i32, bound as i32, 1) };
                same(&format!("saveDict post ds={ds} {shape:?} k={k}"), cn as i64, &cd, rn as i64, &rd);
            }
        }
    }
    // saveDict with a NULL safeBuffer and dictSize 0
    let s = new_streams();
    let a = unsafe { cs(s.c, std::ptr::null_mut(), 0) };
    let b = unsafe { rs(s.r, std::ptr::null_mut(), 0) };
    assert_eq!(a, b, "saveDict(NULL, 0)");
}

/// Row 54: `LZ4_compress_forceExtDict`.
#[test]
fn g3_compress_force_ext_dict() {
    let (cl, rl) = syms::<FnLoadDict>("LZ4_loadDict");
    let (cf, rf) = syms::<FnForceExt>("LZ4_compress_forceExtDict");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x6);
    for &ds in &[0usize, 4, 1000, 65536] {
        for &shape in &[Shape::Textish, Shape::Periodic, Shape::Random] {
            let dict = mkdata(shape, ds, &mut rng);
            let s = new_streams();
            unsafe { cl(s.c, dict.as_ptr(), ds as i32) };
            unsafe { rl(s.r, dict.as_ptr(), ds as i32) };
            let mut arena = BlockArena::new();
            for k in 0..4 {
                let n = rng.range(1, 5000);
                let blk = arena.keep(mkdata(shape, n, &mut rng));
                // notLimited: dest must be at least compressBound(srcSize)
                let bound = unsafe { cb(n as i32) } as usize;
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe { cf(s.c, blk.as_ptr(), cd.as_mut_ptr(), n as i32) };
                let rn = unsafe { rf(s.r, blk.as_ptr(), rd.as_mut_ptr(), n as i32) };
                same(
                    &format!("forceExtDict ds={ds} {shape:?} k={k} n={n}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
            }
        }
    }
}

/// Rows 55-62: streaming decompression dispatch — all four cases of
/// `LZ4_decompress_safe_continue` / `_fast_continue`, plus `LZ4_setStreamDecode`.
#[test]
fn g3_decompress_continue() {
    let (ccs, rcs) = syms::<FnCreateStreamDecode>("LZ4_createStreamDecode");
    let (cfs, rfs) = syms::<FnFreeStream>("LZ4_freeStreamDecode");
    let (csd, rsd) = syms::<FnSetStreamDecode>("LZ4_setStreamDecode");
    let (cdc, rdc) = syms::<FnDecContinue>("LZ4_decompress_safe_continue");
    let (cfc, rfc) = syms::<FnDecFastContinue>("LZ4_decompress_fast_continue");
    let (cl, _) = syms::<FnLoadDict>("LZ4_loadDict");
    let (cc, _) = syms::<FnContinue>("LZ4_compress_fast_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let (crb, _) = syms::<FnCompressBound>("LZ4_decoderRingBufferSize");

    let mut rng = Rng::new(SEED ^ 0x7);

    // --- (A) contiguous output buffer: exercises prefix64k / smallPrefix / doubleDict
    for &shape in &[Shape::Textish, Shape::Periodic, Shape::SmallAlphabet] {
        for &chunk in &[13usize, 500, 9000, 40000] {
            // produce a chained stream with the C compressor over one contiguous src
            let total = mkdata(shape, 220_000, &mut rng);
            let s = unsafe { syms::<FnCreateStream>("LZ4_createStream").0() };
            let mut blocks: Vec<(usize, Vec<u8>)> = Vec::new();
            let mut off = 0usize;
            while off < total.len() {
                let n = chunk.min(total.len() - off);
                let bound = unsafe { cb(n as i32) } as usize;
                let mut d = vec![0u8; bound];
                let k = unsafe {
                    cc(s, total.as_ptr().add(off), d.as_mut_ptr(), n as i32, bound as i32, 1)
                };
                assert!(k > 0);
                d.truncate(k as usize);
                blocks.push((n, d));
                off += n;
            }
            unsafe { syms::<FnFreeStream>("LZ4_freeStream").0(s) };

            for fast in [false, true] {
                let cs = unsafe { ccs() };
                let rs = unsafe { rcs() };
                assert_eq!(unsafe { csd(cs, std::ptr::null(), 0) }, unsafe {
                    rsd(rs, std::ptr::null(), 0)
                }, "setStreamDecode(NULL,0)");
                let mut cout = vec![0u8; total.len() + 1024];
                let mut rout = vec![0u8; total.len() + 1024];
                let mut o = 0usize;
                for (bi, (n, blk)) in blocks.iter().enumerate() {
                    let (cn, rn) = if fast {
                        (
                            unsafe { cfc(cs, blk.as_ptr(), cout.as_mut_ptr().add(o), *n as i32) },
                            unsafe { rfc(rs, blk.as_ptr(), rout.as_mut_ptr().add(o), *n as i32) },
                        )
                    } else {
                        (
                            unsafe {
                                cdc(cs, blk.as_ptr(), cout.as_mut_ptr().add(o), blk.len() as i32, *n as i32)
                            },
                            unsafe {
                                rdc(rs, blk.as_ptr(), rout.as_mut_ptr().add(o), blk.len() as i32, *n as i32)
                            },
                        )
                    };
                    assert_eq!(
                        cn, rn,
                        "dec_continue fast={fast} {shape:?} chunk={chunk} blk={bi}: C={cn} R={rn}"
                    );
                    assert_eq!(
                        &cout[o..o + *n], &rout[o..o + *n],
                        "dec_continue fast={fast} {shape:?} chunk={chunk} blk={bi} output"
                    );
                    assert_eq!(&cout[o..o + *n], &total[o..o + *n], "roundtrip mismatch");
                    o += *n;
                }
                unsafe { cfs(cs) };
                unsafe { rfs(rs) };
            }
        }
    }

    // --- (B) ring-buffer output: exercises forceExtDict wraparound
    for &shape in &[Shape::Textish, Shape::Periodic] {
        for &msg_max in &[1000usize, 9000] {
            let rbs = unsafe { crb(msg_max as i32) } as usize;
            // compress from a ring buffer with the C library
            let s = unsafe { syms::<FnCreateStream>("LZ4_createStream").0() };
            let mut cring = vec![0u8; rbs];
            let mut msgs: Vec<(usize, Vec<u8>, Vec<u8>)> = Vec::new();
            let mut pos = 0usize;
            for _ in 0..60 {
                let n = rng.range(1, msg_max);
                if pos + n > rbs {
                    pos = 0;
                }
                let msg = mkdata(shape, n, &mut rng);
                cring[pos..pos + n].copy_from_slice(&msg);
                let bound = unsafe { cb(n as i32) } as usize;
                let mut d = vec![0u8; bound];
                let k = unsafe {
                    cc(s, cring.as_ptr().add(pos), d.as_mut_ptr(), n as i32, bound as i32, 1)
                };
                assert!(k > 0);
                d.truncate(k as usize);
                msgs.push((pos, msg, d));
                pos += n;
            }
            unsafe { syms::<FnFreeStream>("LZ4_freeStream").0(s) };

            for fast in [false, true] {
                let cs = unsafe { ccs() };
                let rs = unsafe { rcs() };
                unsafe { csd(cs, std::ptr::null(), 0) };
                unsafe { rsd(rs, std::ptr::null(), 0) };
                let mut cout = vec![0u8; rbs];
                let mut rout = vec![0u8; rbs];
                for (mi, (p, msg, blk)) in msgs.iter().enumerate() {
                    let n = msg.len();
                    let (cn, rn) = if fast {
                        (
                            unsafe { cfc(cs, blk.as_ptr(), cout.as_mut_ptr().add(*p), n as i32) },
                            unsafe { rfc(rs, blk.as_ptr(), rout.as_mut_ptr().add(*p), n as i32) },
                        )
                    } else {
                        (
                            unsafe {
                                cdc(cs, blk.as_ptr(), cout.as_mut_ptr().add(*p), blk.len() as i32, n as i32)
                            },
                            unsafe {
                                rdc(rs, blk.as_ptr(), rout.as_mut_ptr().add(*p), blk.len() as i32, n as i32)
                            },
                        )
                    };
                    assert_eq!(cn, rn, "ring dec fast={fast} {shape:?} msg={mi}: C={cn} R={rn}");
                    assert_eq!(&cout[*p..*p + n], &rout[*p..*p + n], "ring dec output msg={mi}");
                    assert_eq!(&cout[*p..*p + n], &msg[..], "ring dec roundtrip msg={mi}");
                }
                unsafe { cfs(cs) };
                unsafe { rfs(rs) };
            }
        }
    }

    // --- (C) setStreamDecode with a real dictionary
    for &ds in &[0usize, 1, 4, 1000, 65536] {
        for &shape in &[Shape::Textish, Shape::Periodic] {
            let dict = mkdata(shape, ds, &mut rng);
            let blkdata = mkdata(shape, 3000, &mut rng);
            let s = unsafe { syms::<FnCreateStream>("LZ4_createStream").0() };
            unsafe { cl(s, dict.as_ptr(), ds as i32) };
            let bound = unsafe { cb(3000) } as usize;
            let mut comp = vec![0u8; bound];
            let k = unsafe { cc(s, blkdata.as_ptr(), comp.as_mut_ptr(), 3000, bound as i32, 1) };
            comp.truncate(k as usize);
            unsafe { syms::<FnFreeStream>("LZ4_freeStream").0(s) };

            let cs = unsafe { ccs() };
            let rs = unsafe { rcs() };
            let a = unsafe { csd(cs, dict.as_ptr(), ds as i32) };
            let b = unsafe { rsd(rs, dict.as_ptr(), ds as i32) };
            assert_eq!(a, b, "setStreamDecode(ds={ds})");
            let mut cout = vec![0u8; 3000 + 16];
            let mut rout = vec![0u8; 3000 + 16];
            let cn = unsafe { cdc(cs, comp.as_ptr(), cout.as_mut_ptr(), comp.len() as i32, 3016) };
            let rn = unsafe { rdc(rs, comp.as_ptr(), rout.as_mut_ptr(), comp.len() as i32, 3016) };
            same_full(&format!("setStreamDecode ds={ds} {shape:?}"), cn as i64, &cout, rn as i64, &rout);
            unsafe { cfs(cs) };
            unsafe { rfs(rs) };
        }
    }
}
