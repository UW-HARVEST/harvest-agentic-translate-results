//! Phase B/C: the BUFFER-LESS streaming API — the lowest-level public streaming
//! entry points, where the caller owns all buffering and drives the decoder
//! state machine explicitly via `ZSTD_nextSrcSizeToDecompress` /
//! `ZSTD_nextInputType`.
//!
//! These are exactly the "composed pipeline" paths that per-wrapper tests miss:
//! the caller must interleave header parsing, block decoding and checksum
//! verification by hand, so any disagreement in the state machine shows up here
//! and nowhere else.

mod common;
use common::*;

type CCtx = *mut std::ffi::c_void;
type DCtx = *mut std::ffi::c_void;

type Fn_createCCtx = unsafe extern "C" fn() -> CCtx;
type Fn_freeCCtx = unsafe extern "C" fn(CCtx) -> usize;
type Fn_createDCtx = unsafe extern "C" fn() -> DCtx;
type Fn_freeDCtx = unsafe extern "C" fn(DCtx) -> usize;
type Fn_bound = unsafe extern "C" fn(usize) -> usize;
type Fn_errCode = unsafe extern "C" fn(usize) -> i32;
type Fn_isError = unsafe extern "C" fn(usize) -> u32;

type Fn_compressBegin = unsafe extern "C" fn(CCtx, i32) -> usize;
type Fn_compressBeginDict =
    unsafe extern "C" fn(CCtx, *const u8, usize, i32) -> usize;
type Fn_chunk = unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize;
type Fn_decompressBegin = unsafe extern "C" fn(DCtx) -> usize;
type Fn_decompressBeginDict = unsafe extern "C" fn(DCtx, *const u8, usize) -> usize;
type Fn_nextSize = unsafe extern "C" fn(DCtx) -> usize;
type Fn_nextType = unsafe extern "C" fn(DCtx) -> i32;
type Fn_setParam = unsafe extern "C" fn(CCtx, i32, i32) -> usize;
type Fn_reset = unsafe extern "C" fn(CCtx, i32) -> usize;

/// Drive the buffer-less COMPRESSOR: begin, N x continue, end.
/// Returns (frame bytes, per-call trace of return codes).
unsafe fn bufferless_compress(
    begin: &Fn_compressBegin,
    cont: &Fn_chunk,
    end: &Fn_chunk,
    ctx: CCtx,
    src: &[u8],
    lvl: i32,
    chunk: usize,
    bound: &Fn_bound,
) -> (Vec<u8>, Vec<usize>) {
    let mut trace = Vec::new();
    // IMPORTANT: `ZSTD_compressBound(srcSize)` is NOT enough capacity here.
    // `ZSTD_compressContinue` emits one BLOCK per call and checks the remaining
    // dstCapacity against `ZSTD_compressBound(chunk)` on every call, so a
    // chunked buffer-less stream needs room for the per-block bound times the
    // number of blocks (with 1-byte chunks each block still carries a 3-byte
    // header). Undersizing this makes the C return dstSize_tooSmall mid-frame
    // and yields a TRUNCATED frame — which both libraries would agree on, so the
    // test would silently pass while testing nothing. Hence the generous bound
    // plus the hard assertion on the trace below.
    let nchunks = if chunk == 0 {
        1
    } else {
        src.len() / chunk + 2
    };
    let per_block = unsafe { bound(chunk.max(1)) } + 16;
    let cap = unsafe { bound(src.len()) } + nchunks * per_block + 1024;
    let mut out = vec![0u8; cap];
    let mut written = 0usize;

    trace.push(unsafe { begin(ctx, lvl) });

    let mut pos = 0usize;
    // all but the final chunk go through compressContinue
    while src.len() - pos > chunk {
        let n = unsafe {
            cont(
                ctx,
                out.as_mut_ptr().add(written),
                cap - written,
                src.as_ptr().add(pos),
                chunk,
            )
        };
        trace.push(n);
        if n > usize::MAX - 200 {
            return (out[..written].to_vec(), trace);
        }
        written += n;
        pos += chunk;
    }
    // the remainder (possibly empty) is flushed with compressEnd
    let n = unsafe {
        end(
            ctx,
            out.as_mut_ptr().add(written),
            cap - written,
            src.as_ptr().add(pos),
            src.len() - pos,
        )
    };
    trace.push(n);
    if n <= usize::MAX - 200 {
        written += n;
    }
    (out[..written].to_vec(), trace)
}

/// Drive the buffer-less DECODER exactly as the zstd manual prescribes:
/// repeatedly ask `ZSTD_nextSrcSizeToDecompress` how many bytes are needed, hand
/// over exactly that many, and record the `ZSTD_nextInputType` state each round.
unsafe fn bufferless_decompress(
    dbegin: &Fn_decompressBegin,
    nextsz: &Fn_nextSize,
    nexttype: &Fn_nextType,
    dcont: &Fn_chunk,
    ctx: DCtx,
    frame: &[u8],
    out_cap: usize,
    is_err: &Fn_isError,
) -> (Vec<u8>, Vec<(usize, i32, usize)>) {
    // trace entries: (nextSrcSize, nextInputType, decompressContinue rc)
    let mut trace = Vec::new();
    let mut out = vec![0u8; out_cap.max(1)];
    let mut produced = 0usize;
    let mut pos = 0usize;

    let rc0 = unsafe { dbegin(ctx) };
    trace.push((0, -1, rc0));
    if unsafe { is_err(rc0) } != 0 {
        return (out[..produced].to_vec(), trace);
    }

    let mut guard = 0usize;
    loop {
        let need = unsafe { nextsz(ctx) };
        let ty = unsafe { nexttype(ctx) };
        if need == 0 {
            trace.push((need, ty, 0));
            break; // frame complete
        }
        if unsafe { is_err(need) } != 0 {
            trace.push((need, ty, need));
            break;
        }
        if pos + need > frame.len() {
            // caller has run out of input; the C requires exactly `need` bytes,
            // so stop here and record the state both libs must agree on.
            trace.push((need, ty, usize::MAX));
            break;
        }
        let rc = unsafe {
            dcont(
                ctx,
                out.as_mut_ptr().add(produced),
                out.len() - produced,
                frame.as_ptr().add(pos),
                need,
            )
        };
        trace.push((need, ty, rc));
        if unsafe { is_err(rc) } != 0 {
            break;
        }
        produced += rc;
        pos += need;
        guard += 1;
        if guard > 1_000_000 {
            panic!("bufferless_decompress did not terminate");
        }
    }
    (out[..produced].to_vec(), trace)
}

/// Buffer-less compress -> buffer-less decompress, byte-identical in both libs,
/// across levels, chunk sizes, shapes and sizes.
#[test]
fn bufferless_roundtrip_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_beg, r_beg) = i.pair::<Fn_compressBegin>("ZSTD_compressBegin");
    let (c_cont, r_cont) = i.pair::<Fn_chunk>("ZSTD_compressContinue");
    let (c_end, r_end) = i.pair::<Fn_chunk>("ZSTD_compressEnd");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_dbeg, r_dbeg) = i.pair::<Fn_decompressBegin>("ZSTD_decompressBegin");
    let (c_nsz, r_nsz) = i.pair::<Fn_nextSize>("ZSTD_nextSrcSizeToDecompress");
    let (c_nty, r_nty) = i.pair::<Fn_nextType>("ZSTD_nextInputType");
    let (c_dcont, r_dcont) = i.pair::<Fn_chunk>("ZSTD_decompressContinue");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { cd_new() };
    let rd = unsafe { rd_new() };

    let mut rng = Rng::new(0xBF11_0000);

    for &shape in &ALL_SHAPES {
        for &len in &[0usize, 1, 2, 64, 1000, 20_000, 131_072, 131_073, 200_000] {
            for &lvl in &[1i32, 3, 9, 19] {
                for &chunk in &[1usize, 100, 1 << 14, 131_072] {
                    if chunk == 1 && len > 2000 {
                        continue;
                    }
                    let src = gen_shape(shape, len, &mut rng);
                    let tag = format!(
                        "bufferless shape={shape:?} len={len} lvl={lvl} chunk={chunk}"
                    );

                    let (cf, ct) = unsafe {
                        bufferless_compress(
                            &c_beg, &c_cont, &c_end, cc, &src, lvl, chunk, &c_bound,
                        )
                    };
                    let (rf, rt) = unsafe {
                        bufferless_compress(
                            &r_beg, &r_cont, &r_end, rc, &src, lvl, chunk, &c_bound,
                        )
                    };
                    assert_eq_dbg(&format!("{tag} / call trace"), ct.clone(), rt.clone());
                    // Guard against a vacuous pass: if the compressor errored
                    // (e.g. the harness undersized dst) the frame would be
                    // truncated and both libraries would "agree" on garbage.
                    for (k, rcv) in ct.iter().enumerate() {
                        assert!(
                            unsafe { c_isE(*rcv) } == 0,
                            "{tag}: C compressor errored at call {k} with {rcv:#x} \
                             (code {}) — the harness, not the library, is at fault",
                            unsafe { c_cd(*rcv) }
                        );
                    }
                    for (k, (a, b)) in ct.iter().zip(rt.iter()).enumerate() {
                        unsafe {
                            assert_eq_dbg(
                                &format!("{tag} / trace[{k}] errcode"),
                                c_cd(*a),
                                r_cd(*b),
                            )
                        };
                    }
                    assert_bytes_eq(&format!("{tag} / frame"), &cf, &rf);

                    // now decode it buffer-lessly with both libraries
                    let (cp, cdt) = unsafe {
                        bufferless_decompress(
                            &c_dbeg, &c_nsz, &c_nty, &c_dcont, cd, &cf, len + 1024, &c_isE,
                        )
                    };
                    let (rp, rdt) = unsafe {
                        bufferless_decompress(
                            &r_dbeg, &r_nsz, &r_nty, &r_dcont, rd, &rf, len + 1024, &c_isE,
                        )
                    };
                    assert_eq_dbg(&format!("{tag} / decode trace len"), cdt.len(), rdt.len());
                    for (k, (a, b)) in cdt.iter().zip(rdt.iter()).enumerate() {
                        assert!(
                            a == b,
                            "{tag}: decode state divergence at step {k}: \
                             C(need,type,rc)={a:?} Rust={b:?}"
                        );
                    }
                    assert_bytes_eq(&format!("{tag} / payload"), &cp, &rp);
                    assert_bytes_eq(&format!("{tag} / payload == src"), &src, &cp);
                }
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

/// `ZSTD_compressBegin_usingDict` / `ZSTD_decompressBegin_usingDict` — the
/// buffer-less path with a dictionary, including empty and raw dictionaries.
#[test]
fn bufferless_with_dict_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_begd, r_begd) = i.pair::<Fn_compressBeginDict>("ZSTD_compressBegin_usingDict");
    let (c_cont, r_cont) = i.pair::<Fn_chunk>("ZSTD_compressContinue");
    let (c_end, r_end) = i.pair::<Fn_chunk>("ZSTD_compressEnd");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");

    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_dbegd, r_dbegd) =
        i.pair::<Fn_decompressBeginDict>("ZSTD_decompressBegin_usingDict");
    let (c_nsz, r_nsz) = i.pair::<Fn_nextSize>("ZSTD_nextSrcSizeToDecompress");
    let (c_nty, r_nty) = i.pair::<Fn_nextType>("ZSTD_nextInputType");
    let (c_dcont, r_dcont) = i.pair::<Fn_chunk>("ZSTD_decompressContinue");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { cd_new() };
    let rd = unsafe { rd_new() };
    let mut rng = Rng::new(0xBFD1_C700);

    // dictionaries: empty, 1 byte, raw random, and text sharing substrings with
    // the payload (so it actually gets used for matches)
    let dicts: Vec<(String, Vec<u8>)> = vec![
        ("empty".into(), vec![]),
        ("one".into(), vec![0x42]),
        ("rand1k".into(), gen_shape(Shape::Random, 1024, &mut rng)),
        ("text8k".into(), gen_shape(Shape::SkewedText, 8192, &mut rng)),
        ("tab64k".into(), gen_shape(Shape::Tabular, 65536, &mut rng)),
    ];

    for (dname, dict) in &dicts {
        for &lvl in &[1i32, 3, 19] {
            for &len in &[0usize, 1, 500, 30_000] {
                let src = gen_shape(Shape::Tabular, len, &mut rng);
                let tag = format!("bufferless dict={dname} lvl={lvl} len={len}");

                let cap = unsafe { c_bound(len) } + 1024;
                let mut cf = vec![0u8; cap];
                let mut rf = vec![0u8; cap];

                let (a0, b0) = unsafe {
                    (
                        c_begd(cc, dict.as_ptr(), dict.len(), lvl),
                        r_begd(rc, dict.as_ptr(), dict.len(), lvl),
                    )
                };
                assert_eq_dbg(&format!("{tag} / compressBegin_usingDict"), a0, b0);

                let a1 = unsafe { c_end(cc, cf.as_mut_ptr(), cap, src.as_ptr(), len) };
                let b1 = unsafe { r_end(rc, rf.as_mut_ptr(), cap, src.as_ptr(), len) };
                assert_eq_dbg(&format!("{tag} / compressEnd"), a1, b1);
                if a1 > usize::MAX - 200 {
                    continue;
                }
                assert_bytes_eq(&format!("{tag} / frame"), &cf[..a1], &rf[..b1]);

                // decode with the same dict
                let (x0, y0) = unsafe {
                    (
                        c_dbegd(cd, dict.as_ptr(), dict.len()),
                        r_dbegd(rd, dict.as_ptr(), dict.len()),
                    )
                };
                assert_eq_dbg(&format!("{tag} / decompressBegin_usingDict"), x0, y0);

                let mut co = vec![0u8; len + 1024];
                let mut ro = vec![0u8; len + 1024];
                let mut cprod = 0usize;
                let mut rprod = 0usize;
                let mut cpos = 0usize;
                let mut rpos = 0usize;
                loop {
                    let cneed = unsafe { c_nsz(cd) };
                    let rneed = unsafe { r_nsz(rd) };
                    let cty = unsafe { c_nty(cd) };
                    let rty = unsafe { r_nty(rd) };
                    assert_eq_dbg(&format!("{tag} / nextSrcSize"), cneed, rneed);
                    assert_eq_dbg(&format!("{tag} / nextInputType"), cty, rty);
                    if cneed == 0 || unsafe { c_isE(cneed) } != 0 {
                        break;
                    }
                    if cpos + cneed > a1 {
                        break;
                    }
                    let x = unsafe {
                        c_dcont(
                            cd,
                            co.as_mut_ptr().add(cprod),
                            co.len() - cprod,
                            cf.as_ptr().add(cpos),
                            cneed,
                        )
                    };
                    let y = unsafe {
                        r_dcont(
                            rd,
                            ro.as_mut_ptr().add(rprod),
                            ro.len() - rprod,
                            rf.as_ptr().add(rpos),
                            rneed,
                        )
                    };
                    assert_eq_dbg(&format!("{tag} / decompressContinue"), x, y);
                    if unsafe { c_isE(x) } != 0 {
                        break;
                    }
                    cprod += x;
                    rprod += y;
                    cpos += cneed;
                    rpos += rneed;
                }
                assert_bytes_eq(&format!("{tag} / payload"), &co[..cprod], &ro[..rprod]);
                assert_bytes_eq(&format!("{tag} / payload == src"), &src, &co[..cprod]);
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

/// Phase C: buffer-less misuse — wrong call order, undersized destinations, and
/// feeding the decoder a different number of bytes than it asked for.
#[test]
fn bufferless_error_paths_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_beg, r_beg) = i.pair::<Fn_compressBegin>("ZSTD_compressBegin");
    let (c_cont, r_cont) = i.pair::<Fn_chunk>("ZSTD_compressContinue");
    let (c_end, r_end) = i.pair::<Fn_chunk>("ZSTD_compressEnd");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");

    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_dbeg, r_dbeg) = i.pair::<Fn_decompressBegin>("ZSTD_decompressBegin");
    let (c_nsz, r_nsz) = i.pair::<Fn_nextSize>("ZSTD_nextSrcSizeToDecompress");
    let (c_nty, r_nty) = i.pair::<Fn_nextType>("ZSTD_nextInputType");
    let (c_dcont, r_dcont) = i.pair::<Fn_chunk>("ZSTD_decompressContinue");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { cd_new() };
    let rd = unsafe { rd_new() };
    let mut rng = Rng::new(0xBFE7_7000);

    let src = gen_shape(Shape::SkewedText, 4000, &mut rng);

    // ---- compressContinue / compressEnd called BEFORE compressBegin
    {
        // fresh contexts so no prior begin leaks in
        let cc2 = unsafe { c_new() };
        let rc2 = unsafe { r_new() };
        let mut o1 = vec![0u8; 1 << 16];
        let mut o2 = vec![0u8; 1 << 16];
        let a = unsafe { c_cont(cc2, o1.as_mut_ptr(), o1.len(), src.as_ptr(), src.len()) };
        let b = unsafe { r_cont(rc2, o2.as_mut_ptr(), o2.len(), src.as_ptr(), src.len()) };
        assert_eq_dbg("compressContinue before begin", a, b);
        unsafe { assert_eq_dbg("compressContinue before begin code", c_cd(a), r_cd(b)) };
        unsafe {
            c_free(cc2);
            r_free(rc2);
        }
    }

    // ---- undersized destination for compressContinue / compressEnd
    for cap in [0usize, 1, 8, 64, 512] {
        unsafe {
            assert_eq_dbg("begin", c_beg(cc, 3), r_beg(rc, 3));
        }
        let mut o1 = vec![0u8; cap.max(1)];
        let mut o2 = vec![0u8; cap.max(1)];
        let a = unsafe { c_cont(cc, o1.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        let b = unsafe { r_cont(rc, o2.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        assert_eq_dbg(&format!("compressContinue dst={cap}"), a, b);
        unsafe { assert_eq_dbg(&format!("compressContinue dst={cap} code"), c_cd(a), r_cd(b)) };

        unsafe {
            c_beg(cc, 3);
            r_beg(rc, 3);
        }
        let a = unsafe { c_end(cc, o1.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        let b = unsafe { r_end(rc, o2.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        assert_eq_dbg(&format!("compressEnd dst={cap}"), a, b);
        unsafe { assert_eq_dbg(&format!("compressEnd dst={cap} code"), c_cd(a), r_cd(b)) };
    }

    // ---- compressBegin with out-of-range levels
    for lvl in [i32::MIN, -131_073, -131_072, 0, 22, 23, 100] {
        let (a, b) = unsafe { (c_beg(cc, lvl), r_beg(rc, lvl)) };
        assert_eq_dbg(&format!("compressBegin({lvl})"), a, b);
        unsafe { assert_eq_dbg(&format!("compressBegin({lvl}) code"), c_cd(a), r_cd(b)) };
    }

    // ---- decompressContinue: fed the WRONG number of bytes (not what
    // nextSrcSizeToDecompress asked for), and with undersized dst
    let cap = unsafe { c_bound(src.len()) } + 64;
    let mut frame = vec![0u8; cap];
    unsafe {
        c_beg(cc, 3);
    }
    let fl = unsafe { c_end(cc, frame.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    let frame = &frame[..fl];

    for wrong_delta in [-1i64, 1, 2, 100] {
        unsafe {
            assert_eq_dbg("decompressBegin", c_dbeg(cd), r_dbeg(rd));
        }
        let need = unsafe { c_nsz(cd) };
        let need2 = unsafe { r_nsz(rd) };
        assert_eq_dbg("nextSrcSizeToDecompress", need, need2);
        unsafe {
            assert_eq_dbg("nextInputType", c_nty(cd), r_nty(rd));
        }
        let give = (need as i64 + wrong_delta).max(0) as usize;
        if give > frame.len() {
            continue;
        }
        let mut o1 = vec![0u8; 1 << 18];
        let mut o2 = vec![0u8; 1 << 18];
        let a = unsafe { c_dcont(cd, o1.as_mut_ptr(), o1.len(), frame.as_ptr(), give) };
        let b = unsafe { r_dcont(rd, o2.as_mut_ptr(), o2.len(), frame.as_ptr(), give) };
        let tag = format!("decompressContinue wrong size need={need} give={give}");
        assert_eq_dbg(&tag, a, b);
        unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
    }

    // ---- decompressContinue with undersized dst for the block
    for dcap in [0usize, 1, 16] {
        unsafe {
            c_dbeg(cd);
            r_dbeg(rd);
        }
        let mut pos = 0usize;
        let mut o1 = vec![0u8; dcap.max(1)];
        let mut o2 = vec![0u8; dcap.max(1)];
        loop {
            let need = unsafe { c_nsz(cd) };
            let n2 = unsafe { r_nsz(rd) };
            assert_eq_dbg("nextSrcSize (small dst)", need, n2);
            if need == 0 || unsafe { c_isE(need) } != 0 || pos + need > frame.len() {
                break;
            }
            let a = unsafe { c_dcont(cd, o1.as_mut_ptr(), dcap, frame.as_ptr().add(pos), need) };
            let b = unsafe { r_dcont(rd, o2.as_mut_ptr(), dcap, frame.as_ptr().add(pos), need) };
            let tag = format!("decompressContinue dcap={dcap} pos={pos}");
            assert_eq_dbg(&tag, a, b);
            unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
            if unsafe { c_isE(a) } != 0 {
                break;
            }
            pos += need;
        }
    }

    // ---- decompressContinue before decompressBegin, on a fresh dctx
    {
        let cd2 = unsafe { cd_new() };
        let rd2 = unsafe { rd_new() };
        let mut o1 = vec![0u8; 4096];
        let mut o2 = vec![0u8; 4096];
        // ask the state machine first — a fresh dctx has a defined initial state
        unsafe {
            assert_eq_dbg("fresh nextSrcSize", c_nsz(cd2), r_nsz(rd2));
            assert_eq_dbg("fresh nextInputType", c_nty(cd2), r_nty(rd2));
        }
        let a = unsafe { c_dcont(cd2, o1.as_mut_ptr(), o1.len(), frame.as_ptr(), frame.len()) };
        let b = unsafe { r_dcont(rd2, o2.as_mut_ptr(), o2.len(), frame.as_ptr(), frame.len()) };
        assert_eq_dbg("decompressContinue on fresh dctx", a, b);
        unsafe { assert_eq_dbg("decompressContinue fresh code", c_cd(a), r_cd(b)) };
        unsafe {
            cd_free(cd2);
            rd_free(rd2);
        }
    }

    // ---- buffer-less decode of corrupted frames: the state machine must fail
    // at the same step with the same code
    for _ in 0..200 {
        let mut f = frame.to_vec();
        let p = rng.below(f.len());
        f[p] ^= 1u8 << rng.below(8);
        unsafe {
            c_dbeg(cd);
            r_dbeg(rd);
        }
        let mut pos = 0usize;
        let mut o1 = vec![0u8; 1 << 18];
        let mut o2 = vec![0u8; 1 << 18];
        let mut step = 0usize;
        loop {
            let need = unsafe { c_nsz(cd) };
            let n2 = unsafe { r_nsz(rd) };
            assert_eq_dbg(&format!("corrupt@{p} step={step} nextSrcSize"), need, n2);
            unsafe {
                assert_eq_dbg(
                    &format!("corrupt@{p} step={step} nextInputType"),
                    c_nty(cd),
                    r_nty(rd),
                )
            };
            if need == 0 || unsafe { c_isE(need) } != 0 || pos + need > f.len() {
                break;
            }
            let a = unsafe { c_dcont(cd, o1.as_mut_ptr(), o1.len(), f.as_ptr().add(pos), need) };
            let b = unsafe { r_dcont(rd, o2.as_mut_ptr(), o2.len(), f.as_ptr().add(pos), need) };
            assert_eq_dbg(&format!("corrupt@{p} step={step} rc"), a, b);
            unsafe {
                assert_eq_dbg(&format!("corrupt@{p} step={step} code"), c_cd(a), r_cd(b))
            };
            if unsafe { c_isE(a) } != 0 {
                break;
            }
            pos += need;
            step += 1;
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}
