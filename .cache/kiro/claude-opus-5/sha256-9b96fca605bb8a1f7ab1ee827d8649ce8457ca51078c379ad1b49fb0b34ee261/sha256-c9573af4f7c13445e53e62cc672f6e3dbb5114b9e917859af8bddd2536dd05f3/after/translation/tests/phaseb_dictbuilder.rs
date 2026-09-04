//! Phase B — differential tests for the DICTIONARY BUILDER API (ZDICT_*,
//! COVER_*) and the DEPRECATED buffered streaming API (ZBUFF_*).
//!
//! Every call goes through `dlsym` on both `.so`s (via `fnpair!`); no Rust
//! function is ever invoked directly. Outputs (return values, mutated
//! parameter structs, and produced byte buffers) are compared exactly.
//!
//! All inputs are randomized with a FIXED seed so runs are reproducible.
//! Dictionary training is expensive, so each test bounds corpus sizes to a
//! few hundred KB and keeps the number of parameter combinations modest.

mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};

// ---------------------------------------------------------------- signatures --

// ZDICT_trainFromBuffer(dictBuffer, dictBufferCapacity, samplesBuffer, samplesSizes, nbSamples)
type FnTrain = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
) -> size_t;

// ZDICT_trainFromBuffer_cover(..., ZDICT_cover_params_t)  [by value]
type FnTrainCover = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    ZDICT_cover_params_t,
) -> size_t;

// ZDICT_optimizeTrainFromBuffer_cover(..., ZDICT_cover_params_t*)  [in/out ptr]
type FnOptCover = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    *mut ZDICT_cover_params_t,
) -> size_t;

type FnTrainFast = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    ZDICT_fastCover_params_t,
) -> size_t;

type FnOptFast = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    *mut ZDICT_fastCover_params_t,
) -> size_t;

type FnTrainLegacy = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    ZDICT_legacy_params_t,
) -> size_t;

// ZDICT_finalizeDictionary(dst, maxDictSize, dictContent, dictContentSize,
//                          samplesBuffer, samplesSizes, nbSamples, ZDICT_params_t)
type FnFinalize = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    ZDICT_params_t,
) -> size_t;

// ZDICT_addEntropyTablesFromBuffer(dictBuffer, dictContentSize, dictBufferCapacity,
//                                  samplesBuffer, samplesSizes, nbSamples)
type FnAddEntropy = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
) -> size_t;

type FnGetDictID = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnGetHdrSize = unsafe extern "C" fn(*const c_void, size_t) -> size_t;

// COVER helpers
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct COVER_epoch_info_t {
    num: c_uint,
    size: c_uint,
}
type FnComputeEpochs =
    unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> COVER_epoch_info_t;
type FnCoverSum = unsafe extern "C" fn(*const size_t, c_uint) -> size_t;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[allow(non_snake_case)]
struct COVER_dictSelection_t {
    dictContent: *mut u8,
    dictSize: size_t,
    totalCompressedSize: size_t,
}
type FnDictSelError = unsafe extern "C" fn(size_t) -> COVER_dictSelection_t;
type FnDictSelIsError = unsafe extern "C" fn(COVER_dictSelection_t) -> c_uint;

// ZBUFF signatures
type FnZbuffCreate = unsafe extern "C" fn() -> *mut c_void;
type FnZbuffFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnZbuffCInit = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnZbuffCInitDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
// ZBUFF_compressContinue(cctx, dst, dstCapacityPtr, src, srcSizePtr)
type FnZbuffCContinue = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut size_t,
    *const c_void,
    *mut size_t,
) -> size_t;
// ZBUFF_compressFlush / End (cctx, dst, dstCapacityPtr)
type FnZbuffCFlush = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t) -> size_t;
type FnZbuffDInit = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnZbuffDInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnZbuffDContinue = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut size_t,
    *const c_void,
    *mut size_t,
) -> size_t;
type FnZbuffRecommended = unsafe extern "C" fn() -> size_t;

const FILL: u8 = 0xAA;

// -------------------------------------------------------- corpus generation --

/// Build a training corpus: `n` samples of "real-ish" redundant data. Returns
/// the concatenated buffer plus the per-sample sizes.
///
/// `skewed` produces a highly non-uniform distribution of sample sizes.
fn build_corpus(
    rng: &mut Rng,
    n: usize,
    base_lo: usize,
    base_hi: usize,
    skewed: bool,
    shapes: &[Shape],
) -> (Vec<u8>, Vec<size_t>) {
    let mut buf = Vec::new();
    let mut sizes = Vec::with_capacity(n);
    for i in 0..n {
        let shape = shapes[rng.below(shapes.len())];
        let len = if skewed {
            // most samples tiny, a few large
            if rng.below(8) == 0 {
                rng.range(base_hi as i32, (base_hi * 3) as i32) as usize
            } else {
                rng.range(base_lo as i32, ((base_lo + base_hi) / 2) as i32) as usize
            }
        } else {
            rng.range(base_lo as i32, base_hi as i32) as usize
        };
        let len = len.max(8);
        // Concatenate several overlapping chunks so cross-sample redundancy
        // exists (that is what the dictionary builder keys off of).
        let mut sample = gen(shape, len, rng);
        // Splice in a shared "motif" so different samples share content.
        let motif = MOTIF;
        if sample.len() > motif.len() + 4 {
            let off = rng.below(sample.len() - motif.len());
            sample[off..off + motif.len()].copy_from_slice(motif);
            if i % 2 == 0 && sample.len() > 2 * motif.len() + 8 {
                let off2 = (off + motif.len() + 4).min(sample.len() - motif.len());
                sample[off2..off2 + motif.len()].copy_from_slice(motif);
            }
        }
        sizes.push(sample.len());
        buf.extend_from_slice(&sample);
    }
    (buf, sizes)
}

const MOTIF: &[u8] =
    b"{\"id\":12345,\"name\":\"zstd-dictionary-training-shared-motif\",\"kind\":\"record\"}";

// --------------------------------------------------------------- assertions --

/// Compare a training result: return code exactly, error-name string on error,
/// and the FULL produced dictionary buffer byte-for-byte on success.
#[track_caller]
fn assert_train_result(
    ctx: &str,
    is_err: &(FnIsError, FnIsError),
    err_name: &(FnErrName, FnErrName),
    rc: size_t,
    rr: size_t,
    dbuf_c: &[u8],
    dbuf_r: &[u8],
) {
    unsafe {
        let ec = (is_err.0)(rc);
        let er = (is_err.1)(rr);
        assert_eq!(
            ec != 0,
            er != 0,
            "{ctx}: isError disagreement C_rc={rc:#x}(err={ec}) R_rc={rr:#x}(err={er})"
        );
        if ec != 0 {
            let nc = cstr((err_name.0)(rc));
            let nr = cstr((err_name.1)(rr));
            assert_eq!(nc, nr, "{ctx}: error-name mismatch");
            return;
        }
        assert_eq!(rc, rr, "{ctx}: success return size differs");
        // Compare the full output buffers (including the untouched 0xAA tail).
        assert_bytes_eq(ctx, dbuf_c, dbuf_r);
    }
}

// ================================================================= TEST 1 ====
// ZDICT_trainFromBuffer — many randomized sample sets.

#[test]
fn zdict_train_from_buffer() {
    let train = fnpair!("ZDICT_trainFromBuffer", FnTrain);
    let is_err = fnpair!("ZDICT_isError", FnIsError);
    let err_name = fnpair!("ZDICT_getErrorName", FnErrName);
    let mut rng = Rng::new(0xD1C7_0001);

    // Vary sample count, sizes (uniform + skewed), and dict capacity.
    let shape_sets: [&[Shape]; 4] = [
        &[Shape::Text],
        &[Shape::Repetitive],
        &[Shape::Text, Shape::Repetitive, Shape::Mixed],
        &[Shape::Mixed, Shape::LongRange, Shape::Text],
    ];

    let mut cases = 0usize;
    for &nb in &[5usize, 10, 40, 120, 300] {
        for skewed in [false, true] {
            for shapes in shape_sets.iter() {
                let (buf, sizes) = build_corpus(&mut rng, nb, 24, 400, skewed, shapes);
                for &cap in &[64usize, 4096, 16384, 65536] {
                    let ctx = format!(
                        "trainFromBuffer nb={nb} skewed={skewed} shapes={shapes:?} cap={cap} corpus={}",
                        buf.len()
                    );
                    let mut dc = vec![FILL; cap];
                    let mut dr = vec![FILL; cap];
                    unsafe {
                        let rc = (train.0)(
                            dc.as_mut_ptr() as *mut c_void,
                            cap,
                            buf.as_ptr() as *const c_void,
                            sizes.as_ptr(),
                            nb as c_uint,
                        );
                        let rr = (train.1)(
                            dr.as_mut_ptr() as *mut c_void,
                            cap,
                            buf.as_ptr() as *const c_void,
                            sizes.as_ptr(),
                            nb as c_uint,
                        );
                        assert_train_result(&ctx, &is_err, &err_name, rc, rr, &dc, &dr);
                    }
                    cases += 1;
                }
            }
        }
    }
    eprintln!("zdict_train_from_buffer: {cases} cases");
}

// ================================================================= TEST 2 ====
// ZDICT_trainFromBuffer_cover + ZDICT_optimizeTrainFromBuffer_cover.

#[test]
fn zdict_cover() {
    let train = fnpair!("ZDICT_trainFromBuffer_cover", FnTrainCover);
    let opt = fnpair!("ZDICT_optimizeTrainFromBuffer_cover", FnOptCover);
    let is_err = fnpair!("ZDICT_isError", FnIsError);
    let err_name = fnpair!("ZDICT_getErrorName", FnErrName);
    let mut rng = Rng::new(0xD1C7_0002);

    // A single moderately sized corpus keeps runtime bounded while still
    // exercising the algorithm; a couple of variants for diversity.
    let corpora: Vec<(Vec<u8>, Vec<size_t>)> = vec![
        build_corpus(&mut rng, 80, 32, 300, false, &[Shape::Text, Shape::Repetitive]),
        build_corpus(&mut rng, 60, 16, 500, true, &[Shape::Mixed, Shape::Text]),
    ];

    let ks = [50u32, 200, 500, 1000];
    let ds = [6u32, 8];
    let split_points = [0.0f64, 0.75, 1.0];
    let dict_ids = [0u32, 0xABCD_1234];

    let mut cases = 0usize;
    for (ci, (buf, sizes)) in corpora.iter().enumerate() {
        let nb = sizes.len() as c_uint;
        // --- non-optimize: sweep k, d, splitPoint, shrinkDict, nbThreads ----
        for &k in &ks {
            for &d in &ds {
                for &sp in &split_points {
                    for &shrink in &[0u32, 1] {
                        for &nt in &[0u32, 1] {
                            let mut p = ZDICT_cover_params_t::default();
                            p.k = k;
                            p.d = d;
                            p.steps = 0;
                            p.nbThreads = nt;
                            p.splitPoint = sp;
                            p.shrinkDict = shrink;
                            p.shrinkDictMaxRegression = if shrink == 1 { 5 } else { 0 };
                            p.zParams.compressionLevel = 6;
                            p.zParams.notificationLevel = 0;
                            p.zParams.dictID = dict_ids[(k as usize) % dict_ids.len()];
                            let cap = 16384usize;
                            let ctx = format!(
                                "cover corpus#{ci} k={k} d={d} sp={sp} shrink={shrink} nt={nt} cap={cap}"
                            );
                            let mut dc = vec![FILL; cap];
                            let mut dr = vec![FILL; cap];
                            unsafe {
                                let rc = (train.0)(
                                    dc.as_mut_ptr() as *mut c_void,
                                    cap,
                                    buf.as_ptr() as *const c_void,
                                    sizes.as_ptr(),
                                    nb,
                                    p,
                                );
                                let rr = (train.1)(
                                    dr.as_mut_ptr() as *mut c_void,
                                    cap,
                                    buf.as_ptr() as *const c_void,
                                    sizes.as_ptr(),
                                    nb,
                                    p,
                                );
                                assert_train_result(&ctx, &is_err, &err_name, rc, rr, &dc, &dr);
                            }
                            cases += 1;
                        }
                    }
                }
            }
        }

        // --- optimize: params struct is IN/OUT. Keep steps tiny so it is fast.
        for &d in &ds {
            for &clevel in &[0i32, 1, 6, 19] {
                for &dictid in &dict_ids {
                    let cap = 16384usize;
                    let mut pc = ZDICT_cover_params_t::default();
                    pc.k = 200; // fix k so we don't sweep k internally
                    pc.d = d;
                    pc.steps = 4; // small
                    pc.nbThreads = 1;
                    pc.splitPoint = 0.0;
                    pc.zParams.compressionLevel = clevel;
                    pc.zParams.notificationLevel = 0;
                    pc.zParams.dictID = dictid;
                    let mut pr = pc;
                    let ctx = format!(
                        "optimizeCover corpus#{ci} d={d} clevel={clevel} dictid={dictid} cap={cap}"
                    );
                    let mut dc = vec![FILL; cap];
                    let mut dr = vec![FILL; cap];
                    unsafe {
                        let rc = (opt.0)(
                            dc.as_mut_ptr() as *mut c_void,
                            cap,
                            buf.as_ptr() as *const c_void,
                            sizes.as_ptr(),
                            nb,
                            &mut pc,
                        );
                        let rr = (opt.1)(
                            dr.as_mut_ptr() as *mut c_void,
                            cap,
                            buf.as_ptr() as *const c_void,
                            sizes.as_ptr(),
                            nb,
                            &mut pr,
                        );
                        assert_train_result(&ctx, &is_err, &err_name, rc, rr, &dc, &dr);
                        // Compare the MUTATED params struct field-for-field.
                        assert_eq!(pc, pr, "{ctx}: mutated cover_params differ (C={pc:?} R={pr:?})");
                    }
                    cases += 1;
                }
            }
        }
    }
    eprintln!("zdict_cover: {cases} cases");
}

// ================================================================= TEST 3 ====
// ZDICT_trainFromBuffer_fastCover + optimize variant (adds f, accel).

#[test]
fn zdict_fastcover() {
    let train = fnpair!("ZDICT_trainFromBuffer_fastCover", FnTrainFast);
    let opt = fnpair!("ZDICT_optimizeTrainFromBuffer_fastCover", FnOptFast);
    let is_err = fnpair!("ZDICT_isError", FnIsError);
    let err_name = fnpair!("ZDICT_getErrorName", FnErrName);
    let mut rng = Rng::new(0xD1C7_0003);

    let corpora: Vec<(Vec<u8>, Vec<size_t>)> = vec![
        build_corpus(&mut rng, 100, 32, 300, false, &[Shape::Text, Shape::Repetitive]),
        build_corpus(&mut rng, 70, 16, 500, true, &[Shape::Mixed, Shape::LongRange, Shape::Text]),
    ];

    let ks = [50u32, 200, 500, 1000];
    let ds = [6u32, 8];
    let fs = [15u32, 20, 23];
    let accels = [1u32, 2, 5, 10];
    let dict_ids = [0u32, 0x0055_00AA];

    let mut cases = 0usize;
    for (ci, (buf, sizes)) in corpora.iter().enumerate() {
        let nb = sizes.len() as c_uint;
        // Non-optimize sweep. To bound combinations, iterate a curated set.
        for &k in &ks {
            for &d in &ds {
                for (fi, &f) in fs.iter().enumerate() {
                    // pair each f with one accel to avoid a full cross product
                    let accel = accels[fi % accels.len()];
                    for &sp in &[0.0f64, 0.75, 1.0] {
                        for &shrink in &[0u32, 1] {
                            let mut p = ZDICT_fastCover_params_t::default();
                            p.k = k;
                            p.d = d;
                            p.f = f;
                            p.steps = 0;
                            p.nbThreads = 0;
                            p.splitPoint = sp;
                            p.accel = accel;
                            p.shrinkDict = shrink;
                            p.shrinkDictMaxRegression = if shrink == 1 { 5 } else { 0 };
                            p.zParams.compressionLevel = 6;
                            p.zParams.dictID = dict_ids[(k as usize) % dict_ids.len()];
                            let cap = 16384usize;
                            let ctx = format!(
                                "fastCover corpus#{ci} k={k} d={d} f={f} accel={accel} sp={sp} shrink={shrink} cap={cap}"
                            );
                            let mut dc = vec![FILL; cap];
                            let mut dr = vec![FILL; cap];
                            unsafe {
                                let rc = (train.0)(
                                    dc.as_mut_ptr() as *mut c_void,
                                    cap,
                                    buf.as_ptr() as *const c_void,
                                    sizes.as_ptr(),
                                    nb,
                                    p,
                                );
                                let rr = (train.1)(
                                    dr.as_mut_ptr() as *mut c_void,
                                    cap,
                                    buf.as_ptr() as *const c_void,
                                    sizes.as_ptr(),
                                    nb,
                                    p,
                                );
                                assert_train_result(&ctx, &is_err, &err_name, rc, rr, &dc, &dr);
                            }
                            cases += 1;
                        }
                    }
                }
            }
        }

        // Optimize (IN/OUT). Bound with fixed k, small steps.
        for &d in &ds {
            for &f in &[15u32, 20, 23] {
                for &accel in &[1u32, 5, 10] {
                    for &clevel in &[0i32, 1, 6, 19] {
                        for &dictid in &dict_ids {
                            let cap = 16384usize;
                            let mut pc = ZDICT_fastCover_params_t::default();
                            pc.k = 200;
                            pc.d = d;
                            pc.f = f;
                            pc.steps = 4;
                            pc.nbThreads = 1;
                            pc.splitPoint = 0.0;
                            pc.accel = accel;
                            pc.zParams.compressionLevel = clevel;
                            pc.zParams.dictID = dictid;
                            let mut pr = pc;
                            let ctx = format!(
                                "optimizeFastCover corpus#{ci} d={d} f={f} accel={accel} clevel={clevel} dictid={dictid} cap={cap}"
                            );
                            let mut dc = vec![FILL; cap];
                            let mut dr = vec![FILL; cap];
                            unsafe {
                                let rc = (opt.0)(
                                    dc.as_mut_ptr() as *mut c_void,
                                    cap,
                                    buf.as_ptr() as *const c_void,
                                    sizes.as_ptr(),
                                    nb,
                                    &mut pc,
                                );
                                let rr = (opt.1)(
                                    dr.as_mut_ptr() as *mut c_void,
                                    cap,
                                    buf.as_ptr() as *const c_void,
                                    sizes.as_ptr(),
                                    nb,
                                    &mut pr,
                                );
                                assert_train_result(&ctx, &is_err, &err_name, rc, rr, &dc, &dr);
                                assert_eq!(
                                    pc, pr,
                                    "{ctx}: mutated fastCover_params differ (C={pc:?} R={pr:?})"
                                );
                            }
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    eprintln!("zdict_fastcover: {cases} cases");
}

// ================================================================= TEST 4 ====
// ZDICT_trainFromBuffer_legacy.

#[test]
fn zdict_legacy() {
    let train = fnpair!("ZDICT_trainFromBuffer_legacy", FnTrainLegacy);
    let is_err = fnpair!("ZDICT_isError", FnIsError);
    let err_name = fnpair!("ZDICT_getErrorName", FnErrName);
    let mut rng = Rng::new(0xD1C7_0004);

    let corpora: Vec<(Vec<u8>, Vec<size_t>)> = vec![
        build_corpus(&mut rng, 120, 32, 300, false, &[Shape::Text, Shape::Repetitive]),
        build_corpus(&mut rng, 80, 16, 500, true, &[Shape::Mixed, Shape::Text]),
    ];

    let mut cases = 0usize;
    for (ci, (buf, sizes)) in corpora.iter().enumerate() {
        let nb = sizes.len() as c_uint;
        for &sel in &[0u32, 1, 9] {
            for &dictid in &[0u32, 0x1234_0000] {
                for &cap in &[4096usize, 16384, 65536] {
                    let mut p = ZDICT_legacy_params_t::default();
                    p.selectivityLevel = sel;
                    p.zParams.compressionLevel = 3;
                    p.zParams.dictID = dictid;
                    let ctx = format!(
                        "legacy corpus#{ci} sel={sel} dictid={dictid} cap={cap}"
                    );
                    let mut dc = vec![FILL; cap];
                    let mut dr = vec![FILL; cap];
                    unsafe {
                        let rc = (train.0)(
                            dc.as_mut_ptr() as *mut c_void,
                            cap,
                            buf.as_ptr() as *const c_void,
                            sizes.as_ptr(),
                            nb,
                            p,
                        );
                        let rr = (train.1)(
                            dr.as_mut_ptr() as *mut c_void,
                            cap,
                            buf.as_ptr() as *const c_void,
                            sizes.as_ptr(),
                            nb,
                            p,
                        );
                        assert_train_result(&ctx, &is_err, &err_name, rc, rr, &dc, &dr);
                    }
                    cases += 1;
                }
            }
        }
    }
    eprintln!("zdict_legacy: {cases} cases");
}

// ================================================================= TEST 5 ====
// finalizeDictionary, addEntropyTablesFromBuffer, getDictID, getDictHeaderSize,
// isError, getErrorName.

#[test]
fn zdict_finalize_and_helpers() {
    let train = fnpair!("ZDICT_trainFromBuffer", FnTrain);
    let finalize = fnpair!("ZDICT_finalizeDictionary", FnFinalize);
    let add_entropy = fnpair!("ZDICT_addEntropyTablesFromBuffer", FnAddEntropy);
    let get_id = fnpair!("ZDICT_getDictID", FnGetDictID);
    let get_hdr = fnpair!("ZDICT_getDictHeaderSize", FnGetHdrSize);
    let is_err = fnpair!("ZDICT_isError", FnIsError);
    let err_name = fnpair!("ZDICT_getErrorName", FnErrName);
    let mut rng = Rng::new(0xD1C7_0005);

    // First train a real dictionary to use as raw content / valid dict input.
    let (buf, sizes) = build_corpus(&mut rng, 150, 32, 300, false, &[Shape::Text, Shape::Repetitive]);
    let nb = sizes.len() as c_uint;
    let cap = 32768usize;
    let mut trained = vec![FILL; cap];
    let trained_len = unsafe {
        let rc = (train.0)(
            trained.as_mut_ptr() as *mut c_void,
            cap,
            buf.as_ptr() as *const c_void,
            sizes.as_ptr(),
            nb,
        );
        let rr = (train.1)(
            vec![FILL; cap].as_mut_ptr() as *mut c_void,
            cap,
            buf.as_ptr() as *const c_void,
            sizes.as_ptr(),
            nb,
        );
        assert_eq!((is_err.0)(rc), (is_err.1)(rr), "train for helpers: isError differ");
        assert!(( is_err.0)(rc) == 0, "expected a valid trained dictionary for helper tests");
        assert_eq!(rc, rr, "train for helpers: size differs");
        rc
    };
    trained.truncate(trained_len);

    // ---- finalizeDictionary: use raw content, various maxDictSize / dictID --
    {
        let raw_content = gen(Shape::Text, 2048, &mut rng);
        for &max_dict in &[512usize, 4096, 32768] {
            for &clevel in &[0i32, 3, 19] {
                for &dictid in &[0u32, 0x0000_5AB1] {
                    let mut p = ZDICT_params_t::default();
                    p.compressionLevel = clevel;
                    p.notificationLevel = 0;
                    p.dictID = dictid;
                    let ctx = format!(
                        "finalize maxDict={max_dict} clevel={clevel} dictid={dictid} content={}",
                        raw_content.len()
                    );
                    let mut dc = vec![FILL; max_dict];
                    let mut dr = vec![FILL; max_dict];
                    unsafe {
                        let rc = (finalize.0)(
                            dc.as_mut_ptr() as *mut c_void,
                            max_dict,
                            raw_content.as_ptr() as *const c_void,
                            raw_content.len(),
                            buf.as_ptr() as *const c_void,
                            sizes.as_ptr(),
                            nb,
                            p,
                        );
                        let rr = (finalize.1)(
                            dr.as_mut_ptr() as *mut c_void,
                            max_dict,
                            raw_content.as_ptr() as *const c_void,
                            raw_content.len(),
                            buf.as_ptr() as *const c_void,
                            sizes.as_ptr(),
                            nb,
                            p,
                        );
                        assert_train_result(&ctx, &is_err, &err_name, rc, rr, &dc, &dr);
                    }
                }
            }
        }
    }

    // ---- addEntropyTablesFromBuffer ---------------------------------------
    {
        // Buffer holds dict content at the front; capacity is larger so tables
        // can be added. Mirror the layout the API expects.
        let content = gen(Shape::Text, 4096, &mut rng);
        for &capacity in &[8192usize, 32768] {
            let ctx = format!("addEntropy contentSize={} capacity={capacity}", content.len());
            let mut dc = vec![FILL; capacity];
            let mut dr = vec![FILL; capacity];
            dc[..content.len()].copy_from_slice(&content);
            dr[..content.len()].copy_from_slice(&content);
            unsafe {
                let rc = (add_entropy.0)(
                    dc.as_mut_ptr() as *mut c_void,
                    content.len(),
                    capacity,
                    buf.as_ptr() as *const c_void,
                    sizes.as_ptr(),
                    nb,
                );
                let rr = (add_entropy.1)(
                    dr.as_mut_ptr() as *mut c_void,
                    content.len(),
                    capacity,
                    buf.as_ptr() as *const c_void,
                    sizes.as_ptr(),
                    nb,
                );
                assert_train_result(&ctx, &is_err, &err_name, rc, rr, &dc, &dr);
            }
        }
    }

    // ---- getDictID / getDictHeaderSize on real + garbage buffers ----------
    {
        // real trained dict
        let mut buffers: Vec<(String, Vec<u8>)> = Vec::new();
        buffers.push(("trained".into(), trained.clone()));
        // garbage / raw buffers
        for &len in &[0usize, 1, 4, 8, 64, 1024] {
            buffers.push((format!("garbage{len}"), gen(Shape::Random, len, &mut rng)));
        }
        // a buffer with the dict magic but truncated
        {
            let mut m = vec![0u8; 32];
            m[0..4].copy_from_slice(&0xEC30A437u32.to_le_bytes());
            buffers.push(("magic-trunc".into(), m));
        }
        for (name, b) in &buffers {
            let ctx = format!("getDictID/{name} len={}", b.len());
            unsafe {
                let idc = (get_id.0)(b.as_ptr() as *const c_void, b.len());
                let idr = (get_id.1)(b.as_ptr() as *const c_void, b.len());
                assert_eq!(idc, idr, "{ctx}: getDictID differs (C={idc} R={idr})");

                let hc = (get_hdr.0)(b.as_ptr() as *const c_void, b.len());
                let hr = (get_hdr.1)(b.as_ptr() as *const c_void, b.len());
                assert_eq!(
                    (is_err.0)(hc),
                    (is_err.1)(hr),
                    "{ctx}: getDictHeaderSize isError differs (C={hc:#x} R={hr:#x})"
                );
                if (is_err.0)(hc) != 0 {
                    assert_eq!(
                        cstr((err_name.0)(hc)),
                        cstr((err_name.1)(hr)),
                        "{ctx}: getDictHeaderSize error-name differs"
                    );
                } else {
                    assert_eq!(hc, hr, "{ctx}: getDictHeaderSize differs");
                }
            }
        }
    }

    // ---- isError / getErrorName over a sweep of codes ---------------------
    {
        for code in [0usize, 1, 2, 10, 20, 40, 63, 100, usize::MAX, usize::MAX - 5] {
            unsafe {
                let ec = (is_err.0)(code);
                let er = (is_err.1)(code);
                assert_eq!(ec, er, "isError({code:#x}) differs (C={ec} R={er})");
                let nc = cstr((err_name.0)(code));
                let nr = cstr((err_name.1)(code));
                assert_eq!(nc, nr, "getErrorName({code:#x}) differs");
            }
        }
    }
}

// ================================================================= TEST 6 ====
// COVER_* exported helpers — invoke the pure/simple ones directly.

#[test]
fn cover_helpers() {
    let compute_epochs = fnpair!("COVER_computeEpochs", FnComputeEpochs);
    let cover_sum = fnpair!("COVER_sum", FnCoverSum);
    let dsel_err = fnpair!("COVER_dictSelectionError", FnDictSelError);
    let dsel_is_err = fnpair!("COVER_dictSelectionIsError", FnDictSelIsError);

    // Verify the stateful/pointer-based helpers are at least dlsym-able from
    // both libraries (already fetched via fnpair! => both sides have them).
    // Direct invocation of these is unsafe without constructing valid internal
    // COVER_best_t state (mutex/cond, live threads, malloc'd dictContent that
    // COVER_best_finish/free will free), so we only bind them here.
    let _ = fnpair!("COVER_best_init", unsafe extern "C" fn(*mut c_void));
    let _ = fnpair!("COVER_best_start", unsafe extern "C" fn(*mut c_void));
    let _ = fnpair!("COVER_best_wait", unsafe extern "C" fn(*mut c_void));
    let _ = fnpair!("COVER_best_finish", unsafe extern "C" fn(*mut c_void, ZDICT_cover_params_t, COVER_dictSelection_t));
    let _ = fnpair!("COVER_best_destroy", unsafe extern "C" fn(*mut c_void));
    let _ = fnpair!("COVER_dictSelectionFree", unsafe extern "C" fn(COVER_dictSelection_t));
    let _ = fnpair!("COVER_selectDict", unsafe extern "C" fn());
    let _ = fnpair!("COVER_checkTotalCompressedSize", unsafe extern "C" fn());
    let _ = fnpair!("COVER_warnOnSmallCorpus", unsafe extern "C" fn(size_t, size_t, c_int));

    let mut rng = Rng::new(0xD1C7_0006);

    // --- COVER_computeEpochs: sweep maxDictSize, nbDmers, k, passes ---------
    let mut cases = 0usize;
    for &max_dict in &[0u32, 256, 4096, 16384, 110 * 1024] {
        // NOTE: nbDmers must be > 0. COVER_computeEpochs divides nbDmers by
        // epochs.size, and with nbDmers==0 the C reference itself computes
        // epochs.size==0 and performs 0/0 (SIGFPE). That is a C-side crash on
        // invalid input, not a Rust divergence, so we exclude it.
        for &nb_dmers in &[1u32, 10, 100, 1000, 100000] {
            for &k in &[1u32, 16, 50, 200, 1000] {
                for &passes in &[1u32, 4, 40] {
                    let ctx =
                        format!("computeEpochs maxDict={max_dict} nbDmers={nb_dmers} k={k} passes={passes}");
                    unsafe {
                        let ec = (compute_epochs.0)(max_dict, nb_dmers, k, passes);
                        let er = (compute_epochs.1)(max_dict, nb_dmers, k, passes);
                        assert_eq!(ec, er, "{ctx}: epoch info differs (C={ec:?} R={er:?})");
                    }
                    cases += 1;
                }
            }
        }
    }

    // --- COVER_sum: random samplesSizes arrays -----------------------------
    for _ in 0..200 {
        let n = rng.below(64);
        let szs: Vec<size_t> = (0..n).map(|_| rng.below(1_000_000)).collect();
        let ctx = format!("cover_sum n={n}");
        unsafe {
            let sc = (cover_sum.0)(szs.as_ptr(), n as c_uint);
            let sr = (cover_sum.1)(szs.as_ptr(), n as c_uint);
            assert_eq!(sc, sr, "{ctx}: sum differs (C={sc} R={sr})");
        }
    }
    // empty (null ptr allowed with 0 count)
    unsafe {
        let sc = (cover_sum.0)(std::ptr::null(), 0);
        let sr = (cover_sum.1)(std::ptr::null(), 0);
        assert_eq!(sc, sr, "cover_sum(null,0) differs");
    }

    // --- COVER_dictSelectionError + IsError --------------------------------
    for &code in &[0usize, 1, 2, 40, 63, usize::MAX, usize::MAX - 3] {
        unsafe {
            let sc = (dsel_err.0)(code);
            let sr = (dsel_err.1)(code);
            assert_eq!(sc.dictSize, sr.dictSize, "dictSelectionError({code:#x}) dictSize differs");
            assert_eq!(
                sc.totalCompressedSize, sr.totalCompressedSize,
                "dictSelectionError({code:#x}) totalCompressedSize differs"
            );
            // dictContent should be NULL for the error struct on both sides.
            assert_eq!(
                sc.dictContent.is_null(),
                sr.dictContent.is_null(),
                "dictSelectionError({code:#x}) dictContent null-ness differs"
            );
            // Feed the produced struct into IsError on both sides.
            let ic = (dsel_is_err.0)(sc);
            let ir = (dsel_is_err.1)(sr);
            assert_eq!(ic, ir, "dictSelectionIsError({code:#x}) differs (C={ic} R={ir})");
        }
    }
    eprintln!("cover_helpers: {cases} computeEpochs cases");
}

// ================================================================= TEST 7 ====
// Deprecated ZBUFF_* streaming — full round trip + cross decode.

/// Streaming compress `src` through ONE library's ZBUFF encoder, in randomized
/// chunk sizes. Returns the produced frame bytes.
///
/// `which`: 0 = C side, 1 = Rust side.
#[allow(clippy::too_many_arguments)]
unsafe fn zbuff_compress(
    create: FnZbuffCreate,
    free: FnZbuffFree,
    cinit: FnZbuffCInit,
    ccont: FnZbuffCContinue,
    cflush: FnZbuffCFlush,
    cend: FnZbuffCFlush,
    is_err: FnIsError,
    err_name: FnErrName,
    level: c_int,
    src: &[u8],
    rng: &mut Rng,
    ctx: &str,
) -> Vec<u8> {
    let cctx = create();
    assert!(!cctx.is_null(), "{ctx}: ZBUFF_createCCtx null");
    let ir = cinit(cctx, level);
    assert!(is_err(ir) == 0, "{ctx}: compressInit error: {}", cstr(err_name(ir)));

    let mut out = Vec::new();
    let mut in_pos = 0usize;
    let mut chunk_buf = vec![FILL; 1 << 17];

    while in_pos < src.len() {
        // randomized input chunk (include 1-byte and large chunks)
        let remaining = src.len() - in_pos;
        let chunk = pick_chunk(rng, remaining);
        let mut src_size = chunk;
        let mut dst_cap = chunk_buf.len();
        let hint = ccont(
            cctx,
            chunk_buf.as_mut_ptr() as *mut c_void,
            &mut dst_cap,
            src[in_pos..].as_ptr() as *const c_void,
            &mut src_size,
        );
        assert!(is_err(hint) == 0, "{ctx}: compressContinue error: {}", cstr(err_name(hint)));
        out.extend_from_slice(&chunk_buf[..dst_cap]);
        in_pos += src_size;
    }
    // flush + end
    loop {
        let mut dst_cap = chunk_buf.len();
        let rem = cflush(cctx, chunk_buf.as_mut_ptr() as *mut c_void, &mut dst_cap);
        assert!(is_err(rem) == 0, "{ctx}: compressFlush error: {}", cstr(err_name(rem)));
        out.extend_from_slice(&chunk_buf[..dst_cap]);
        if rem == 0 {
            break;
        }
    }
    loop {
        let mut dst_cap = chunk_buf.len();
        let rem = cend(cctx, chunk_buf.as_mut_ptr() as *mut c_void, &mut dst_cap);
        assert!(is_err(rem) == 0, "{ctx}: compressEnd error: {}", cstr(err_name(rem)));
        out.extend_from_slice(&chunk_buf[..dst_cap]);
        if rem == 0 {
            break;
        }
    }
    let fr = free(cctx);
    assert!(is_err(fr) == 0, "{ctx}: freeCCtx error");
    out
}

/// Streaming decompress `frame` through ONE library's ZBUFF decoder in
/// randomized chunk sizes. Returns the produced plaintext.
#[allow(clippy::too_many_arguments)]
unsafe fn zbuff_decompress(
    create: FnZbuffCreate,
    free: FnZbuffFree,
    dinit: FnZbuffDInit,
    dcont: FnZbuffDContinue,
    is_err: FnIsError,
    err_name: FnErrName,
    frame: &[u8],
    rng: &mut Rng,
    ctx: &str,
) -> Vec<u8> {
    let dctx = create();
    assert!(!dctx.is_null(), "{ctx}: ZBUFF_createDCtx null");
    let ir = dinit(dctx);
    assert!(is_err(ir) == 0, "{ctx}: decompressInit error: {}", cstr(err_name(ir)));

    let mut out = Vec::new();
    let mut in_pos = 0usize;
    let mut out_buf = vec![FILL; 1 << 17];
    loop {
        let remaining = frame.len() - in_pos;
        if remaining == 0 {
            break;
        }
        let chunk = pick_chunk(rng, remaining);
        let mut src_size = chunk;
        let mut dst_cap = out_buf.len();
        let rc = dcont(
            dctx,
            out_buf.as_mut_ptr() as *mut c_void,
            &mut dst_cap,
            frame[in_pos..].as_ptr() as *const c_void,
            &mut src_size,
        );
        assert!(is_err(rc) == 0, "{ctx}: decompressContinue error: {}", cstr(err_name(rc)));
        out.extend_from_slice(&out_buf[..dst_cap]);
        in_pos += src_size;
        if rc == 0 && src_size == 0 && dst_cap == 0 {
            break; // frame fully decoded and nothing left
        }
    }
    let fr = free(dctx);
    assert!(is_err(fr) == 0, "{ctx}: freeDCtx error");
    out
}

fn pick_chunk(rng: &mut Rng, remaining: usize) -> usize {
    match rng.below(6) {
        0 => 1,                                   // 1-byte chunk
        1 => remaining,                           // huge chunk (all remaining)
        2 => (1 + rng.below(4)).min(remaining),   // tiny
        3 => (1 + rng.below(64)).min(remaining),  // small
        4 => (1 + rng.below(4096)).min(remaining),// medium
        _ => (1 + rng.below(70000)).min(remaining), // large
    }
    .max(1)
}

#[test]
fn zbuff_streaming_roundtrip() {
    // C-side fns
    let cc_create = fnpair!("ZBUFF_createCCtx", FnZbuffCreate);
    let cc_free = fnpair!("ZBUFF_freeCCtx", FnZbuffFree);
    let cc_init = fnpair!("ZBUFF_compressInit", FnZbuffCInit);
    let cc_initdict = fnpair!("ZBUFF_compressInitDictionary", FnZbuffCInitDict);
    let cc_cont = fnpair!("ZBUFF_compressContinue", FnZbuffCContinue);
    let cc_flush = fnpair!("ZBUFF_compressFlush", FnZbuffCFlush);
    let cc_end = fnpair!("ZBUFF_compressEnd", FnZbuffCFlush);
    let dc_create = fnpair!("ZBUFF_createDCtx", FnZbuffCreate);
    let dc_free = fnpair!("ZBUFF_freeDCtx", FnZbuffFree);
    let dc_init = fnpair!("ZBUFF_decompressInit", FnZbuffDInit);
    let dc_initdict = fnpair!("ZBUFF_decompressInitDictionary", FnZbuffDInitDict);
    let dc_cont = fnpair!("ZBUFF_decompressContinue", FnZbuffDContinue);
    let is_err = fnpair!("ZBUFF_isError", FnIsError);
    let err_name = fnpair!("ZBUFF_getErrorName", FnErrName);

    // recommended sizes must match exactly
    for (name, sym) in [
        ("ZBUFF_recommendedCInSize", "ci"),
        ("ZBUFF_recommendedCOutSize", "co"),
        ("ZBUFF_recommendedDInSize", "di"),
        ("ZBUFF_recommendedDOutSize", "do"),
    ] {
        let f = pair::<FnZbuffRecommended>(name);
        unsafe {
            let vc = (*f.0)();
            let vr = (*f.1)();
            assert_eq!(vc, vr, "{name} ({sym}) differs C={vc} R={vr}");
        }
    }

    let mut rng = Rng::new(0xD1C7_0007);

    let shapes = [
        Shape::Zeros,
        Shape::Repetitive,
        Shape::Text,
        Shape::Random,
        Shape::Mixed,
        Shape::LongRange,
        Shape::SingleByte,
        Shape::TwoSymbol,
    ];

    for &level in &[1i32, 3, 9, 19] {
        for &shape in &shapes {
            for &len in &[0usize, 1, 100, 4096, 130 * 1024, 300 * 1024] {
                let src = gen(shape, len, &mut rng);

                // Compress with each library independently (own decode chunks).
                let ctx_c = format!("ZBUFF compress C level={level} shape={shape:?} len={len}");
                let ctx_r = format!("ZBUFF compress R level={level} shape={shape:?} len={len}");
                let frame_c = unsafe {
                    zbuff_compress(
                        cc_create.0, cc_free.0, cc_init.0, cc_cont.0, cc_flush.0, cc_end.0,
                        is_err.0, err_name.0, level, &src, &mut rng, &ctx_c,
                    )
                };
                let frame_r = unsafe {
                    zbuff_compress(
                        cc_create.1, cc_free.1, cc_init.1, cc_cont.1, cc_flush.1, cc_end.1,
                        is_err.1, err_name.1, level, &src, &mut rng, &ctx_r,
                    )
                };
                // Produced frame bytes must be identical.
                assert_bytes_eq(
                    &format!("ZBUFF frame bytes level={level} shape={shape:?} len={len}"),
                    &frame_c,
                    &frame_r,
                );

                // Decompress each frame with each decoder (4 combinations) and
                // confirm the plaintext round-trips everywhere.
                for (fname, frame) in [("C", &frame_c), ("R", &frame_r)] {
                    let out_c = unsafe {
                        zbuff_decompress(
                            dc_create.0, dc_free.0, dc_init.0, dc_cont.0, is_err.0, err_name.0,
                            frame, &mut rng,
                            &format!("ZBUFF Cdecode of {fname}-frame level={level} shape={shape:?} len={len}"),
                        )
                    };
                    let out_r = unsafe {
                        zbuff_decompress(
                            dc_create.1, dc_free.1, dc_init.1, dc_cont.1, is_err.1, err_name.1,
                            frame, &mut rng,
                            &format!("ZBUFF Rdecode of {fname}-frame level={level} shape={shape:?} len={len}"),
                        )
                    };
                    assert_bytes_eq(
                        &format!("ZBUFF roundtrip {fname}-frame Cdecode level={level} shape={shape:?} len={len}"),
                        &src, &out_c,
                    );
                    assert_bytes_eq(
                        &format!("ZBUFF roundtrip {fname}-frame Rdecode level={level} shape={shape:?} len={len}"),
                        &src, &out_r,
                    );
                }
            }
        }
    }

    // ---- dictionary-based streaming round trip -----------------------------
    {
        let dict = gen(Shape::Text, 4096, &mut rng);
        let src = gen(Shape::Text, 20000, &mut rng);
        let level = 5i32;
        let ctx = "ZBUFF dict";
        // Compress with dict on both sides via the InitDictionary path.
        let compress_with_dict = |create: FnZbuffCreate,
                                   free: FnZbuffFree,
                                   initdict: FnZbuffCInitDict,
                                   ccont: FnZbuffCContinue,
                                   cflush: FnZbuffCFlush,
                                   cend: FnZbuffCFlush,
                                   iserr: FnIsError,
                                   ername: FnErrName,
                                   rng: &mut Rng|
         -> Vec<u8> {
            unsafe {
                let c = create();
                let ir = initdict(c, dict.as_ptr() as *const c_void, dict.len(), level);
                assert!(iserr(ir) == 0, "{ctx}: compressInitDictionary error: {}", cstr(ername(ir)));
                let mut out = Vec::new();
                let mut in_pos = 0usize;
                let mut cb = vec![FILL; 1 << 17];
                while in_pos < src.len() {
                    let chunk = pick_chunk(rng, src.len() - in_pos);
                    let mut ss = chunk;
                    let mut dc = cb.len();
                    let h = ccont(c, cb.as_mut_ptr() as *mut c_void, &mut dc,
                                  src[in_pos..].as_ptr() as *const c_void, &mut ss);
                    assert!(iserr(h) == 0, "{ctx}: dict compressContinue error");
                    out.extend_from_slice(&cb[..dc]);
                    in_pos += ss;
                }
                loop {
                    let mut dc = cb.len();
                    let r = cflush(c, cb.as_mut_ptr() as *mut c_void, &mut dc);
                    assert!(iserr(r) == 0, "{ctx}: dict flush error");
                    out.extend_from_slice(&cb[..dc]);
                    if r == 0 { break; }
                }
                loop {
                    let mut dc = cb.len();
                    let r = cend(c, cb.as_mut_ptr() as *mut c_void, &mut dc);
                    assert!(iserr(r) == 0, "{ctx}: dict end error");
                    out.extend_from_slice(&cb[..dc]);
                    if r == 0 { break; }
                }
                assert!(iserr(free(c)) == 0, "{ctx}: dict freeCCtx error");
                out
            }
        };
        let fc = compress_with_dict(
            cc_create.0, cc_free.0, cc_initdict.0, cc_cont.0, cc_flush.0, cc_end.0,
            is_err.0, err_name.0, &mut rng,
        );
        let fr = compress_with_dict(
            cc_create.1, cc_free.1, cc_initdict.1, cc_cont.1, cc_flush.1, cc_end.1,
            is_err.1, err_name.1, &mut rng,
        );
        assert_bytes_eq(&format!("{ctx}: frame bytes"), &fc, &fr);

        // Decode with dict on both sides (cross decode).
        let decompress_with_dict = |create: FnZbuffCreate,
                                     free: FnZbuffFree,
                                     initdict: FnZbuffDInitDict,
                                     dcont: FnZbuffDContinue,
                                     iserr: FnIsError,
                                     ername: FnErrName,
                                     frame: &[u8],
                                     rng: &mut Rng|
         -> Vec<u8> {
            unsafe {
                let d = create();
                let ir = initdict(d, dict.as_ptr() as *const c_void, dict.len());
                assert!(iserr(ir) == 0, "{ctx}: decompressInitDictionary error: {}", cstr(ername(ir)));
                let mut out = Vec::new();
                let mut in_pos = 0usize;
                let mut ob = vec![FILL; 1 << 17];
                while in_pos < frame.len() {
                    let chunk = pick_chunk(rng, frame.len() - in_pos);
                    let mut ss = chunk;
                    let mut dc = ob.len();
                    let r = dcont(d, ob.as_mut_ptr() as *mut c_void, &mut dc,
                                  frame[in_pos..].as_ptr() as *const c_void, &mut ss);
                    assert!(iserr(r) == 0, "{ctx}: dict decompressContinue error");
                    out.extend_from_slice(&ob[..dc]);
                    in_pos += ss;
                    if r == 0 && ss == 0 && dc == 0 { break; }
                }
                assert!(iserr(free(d)) == 0, "{ctx}: dict freeDCtx error");
                out
            }
        };
        for (fname, frame) in [("C", &fc), ("R", &fr)] {
            let oc = decompress_with_dict(
                dc_create.0, dc_free.0, dc_initdict.0, dc_cont.0, is_err.0, err_name.0, frame, &mut rng,
            );
            let or = decompress_with_dict(
                dc_create.1, dc_free.1, dc_initdict.1, dc_cont.1, is_err.1, err_name.1, frame, &mut rng,
            );
            assert_bytes_eq(&format!("{ctx}: {fname}-frame Cdecode roundtrip"), &src, &oc);
            assert_bytes_eq(&format!("{ctx}: {fname}-frame Rdecode roundtrip"), &src, &or);
        }
    }

    // ---- error path: isError / getErrorName over sweep of codes -----------
    for code in [0usize, 1, 2, 10, 40, 63, usize::MAX, usize::MAX - 7] {
        unsafe {
            assert_eq!(
                (is_err.0)(code), (is_err.1)(code),
                "ZBUFF_isError({code:#x}) differs"
            );
            assert_eq!(
                cstr((err_name.0)(code)), cstr((err_name.1)(code)),
                "ZBUFF_getErrorName({code:#x}) differs"
            );
        }
    }
}
