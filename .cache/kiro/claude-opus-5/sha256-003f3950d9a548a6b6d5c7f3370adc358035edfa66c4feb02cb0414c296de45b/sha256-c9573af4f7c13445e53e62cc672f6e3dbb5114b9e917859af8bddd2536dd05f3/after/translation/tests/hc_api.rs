//! Phase B — HC API (lz4hc.c) differential tests. CONFIGS.md rows 46-78.
//!
//! Every call goes through the `.so` exports of BOTH implementations.

mod common;
use common::*;

type FnHC = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnHCExt = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnHCDestSize = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, *mut i32, i32, i32) -> i32;
type FnCreateHCStream = unsafe extern "C" fn() -> *mut u8;
type FnFreeHCStream = unsafe extern "C" fn(*mut u8) -> i32;
type FnInitStreamHC = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;
type FnResetHC = unsafe extern "C" fn(*mut u8, i32);
type FnLoadDictHC = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
type FnAttachHC = unsafe extern "C" fn(*mut u8, *const u8);
type FnHCContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
type FnHCContinueDestSize = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, *mut i32, i32) -> i32;
type FnSaveDictHC = unsafe extern "C" fn(*mut u8, *mut u8, i32) -> i32;
type FnFavor = unsafe extern "C" fn(*mut u8, i32);
type FnSetLevel = unsafe extern "C" fn(*mut u8, i32);
type FnVoidI32 = unsafe extern "C" fn() -> i32;
type FnDecSafe = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;

// obsolete HC
type FnHC3 = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
type FnHC4 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnHC5 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnHCState4 = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;
type FnHCState5 = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
type FnHCState6 = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnCreateHC = unsafe extern "C" fn(*const u8) -> *mut u8;
type FnFreeHC = unsafe extern "C" fn(*mut u8) -> i32;
type FnSlideHC = unsafe extern "C" fn(*mut u8) -> *mut u8;
type FnResetStreamStateHC = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

/// Every compression level worth distinguishing, incl. the clamping paths.
/// 1-2 = lz4mid, 3-9 = lz4hc, 10-12 = lz4opt (12 = ultra).
const LEVELS: [i32; 15] = [
    i32::MIN,
    -100,
    -5,
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
];
/// A smaller sweep for the O(n^2)-ish combinatorial tests.
const LEVELS_S: [i32; 8] = [0, 1, 2, 3, 6, 9, 10, 12];

fn hc_state() -> Aligned<{ LZ4_STREAMHC_SIZE + 128 }> {
    Aligned::new()
}

fn two_bufs(n: usize) -> (Vec<u8>, Vec<u8>) {
    (vec![0xAAu8; n], vec![0xAAu8; n])
}

// ============================================================== rows 46-56, 71

#[test]
fn row46_56_compress_hc_levels() {
    let (c, r) = sym::<FnHC>("LZ4_compress_HC");
    let (cds, rds) = sym::<FnDecSafe>("LZ4_decompress_safe");
    let mut rng = Rng::new(0x4643_0046);

    for &lvl in &LEVELS {
        for &shape in &SHAPES {
            for &len in BOUNDARY_SIZES.iter() {
                let src = make_data(&mut rng, len, shape);
                let bound = lz4_compress_bound(len as i32) as usize;
                let (mut cd, mut rd) = two_bufs(bound + 8);
                let (a, b) = unsafe {
                    (
                        c(src.as_ptr(), cd.as_mut_ptr(), len as i32, bound as i32, lvl),
                        r(src.as_ptr(), rd.as_mut_ptr(), len as i32, bound as i32, lvl),
                    )
                };
                let ctx = format!("compress_HC lvl={lvl} shape={shape:?} len={len}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);

                // round-trip through the C decoder to prove the bitstream is valid
                if a > 0 {
                    let (mut co, mut ro) = two_bufs(len + 16);
                    let (x, y) = unsafe {
                        (
                            cds(cd.as_ptr(), co.as_mut_ptr(), a, len as i32),
                            rds(rd.as_ptr(), ro.as_mut_ptr(), b, len as i32),
                        )
                    };
                    eq(&format!("{ctx} decode ret"), x, y);
                    eq(&format!("{ctx} decode len"), x, len as i32);
                    eq_bytes(&format!("{ctx} decode"), &co[..len], &src);
                }
            }
        }
    }
}

#[test]
fn row55_compress_hc_limited_output() {
    let (c, r) = sym::<FnHC>("LZ4_compress_HC");
    let mut rng = Rng::new(0x4643_0055);
    for &lvl in &LEVELS_S {
        for &shape in &SHAPES {
            for _ in 0..8 {
                let len = rng.range(1, 50_000);
                let src = make_data(&mut rng, len, shape);
                let bound = lz4_compress_bound(len as i32) as usize;
                for cap in [
                    0usize,
                    1,
                    2,
                    13,
                    len / 8,
                    len / 4,
                    len / 2,
                    len.saturating_sub(1),
                    len,
                    bound - 1,
                    bound,
                ] {
                    let (mut cd, mut rd) = two_bufs(cap + 16);
                    let (a, b) = unsafe {
                        (
                            c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap as i32, lvl),
                            r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap as i32, lvl),
                        )
                    };
                    let ctx = format!("HC limited lvl={lvl} shape={shape:?} len={len} cap={cap}");
                    eq(&ctx, a, b);
                    eq_bytes(&ctx, &cd, &rd);
                }
            }
        }
    }
}

#[test]
fn row57_58_hc_ext_state() {
    let (ce, re) = sym::<FnHCExt>("LZ4_compress_HC_extStateHC");
    let (cf, rf) = sym::<FnHCExt>("LZ4_compress_HC_extStateHC_fastReset");
    let (cs, rs) = sym::<FnVoidI32>("LZ4_sizeofStateHC");
    let ssz = unsafe { cs() } as usize;
    eq("LZ4_sizeofStateHC", ssz as i32, unsafe { rs() });
    let (csss, rsss) = sym::<FnVoidI32>("LZ4_sizeofStreamStateHC");
    eq("LZ4_sizeofStreamStateHC", unsafe { csss() }, unsafe {
        rsss()
    });

    let mut rng = Rng::new(0x4643_0057);
    let mut cst = hc_state();
    let mut rst = hc_state();

    for &lvl in &LEVELS_S {
        for &shape in &SHAPES {
            // row 57: fresh state
            for _ in 0..4 {
                let len = rng.range(0, 60_000);
                let src = make_data(&mut rng, len, shape);
                let bound = lz4_compress_bound(len as i32) as usize;
                let (mut cd, mut rd) = two_bufs(bound + 8);
                cst.fill0();
                rst.fill0();
                let (a, b) = unsafe {
                    (
                        ce(
                            cst.ptr(),
                            src.as_ptr(),
                            cd.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            lvl,
                        ),
                        re(
                            rst.ptr(),
                            src.as_ptr(),
                            rd.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            lvl,
                        ),
                    )
                };
                let ctx = format!("HC extState lvl={lvl} shape={shape:?} len={len}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);
            }
            // row 58: fastReset with a reused state
            cst.fill0();
            rst.fill0();
            for _ in 0..6 {
                let len = rng.range(0, 25_000);
                let src = make_data(&mut rng, len, shape);
                let bound = lz4_compress_bound(len as i32) as usize;
                let (mut cd, mut rd) = two_bufs(bound + 8);
                let (a, b) = unsafe {
                    (
                        cf(
                            cst.ptr(),
                            src.as_ptr(),
                            cd.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            lvl,
                        ),
                        rf(
                            rst.ptr(),
                            src.as_ptr(),
                            rd.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            lvl,
                        ),
                    )
                };
                let ctx = format!("HC extState_fastReset lvl={lvl} shape={shape:?} len={len}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);
            }
        }
    }
}

#[test]
fn row59_hc_dest_size() {
    let (c, r) = sym::<FnHCDestSize>("LZ4_compress_HC_destSize");
    let mut rng = Rng::new(0x4643_0059);
    let mut cst = hc_state();
    let mut rst = hc_state();
    for &lvl in &LEVELS_S {
        for &shape in &SHAPES {
            for _ in 0..6 {
                let len = rng.range(1, 50_000);
                let src = make_data(&mut rng, len, shape);
                let bound = lz4_compress_bound(len as i32) as usize;
                for target in [
                    0usize,
                    1,
                    2,
                    3,
                    12,
                    13,
                    len / 16 + 1,
                    len / 4,
                    len / 2,
                    len,
                    bound,
                    bound + 50,
                ] {
                    let (mut cd, mut rd) = two_bufs(target + 16);
                    let mut cs = len as i32;
                    let mut ru = len as i32;
                    cst.fill0();
                    rst.fill0();
                    let (a, b) = unsafe {
                        (
                            c(
                                cst.ptr(),
                                src.as_ptr(),
                                cd.as_mut_ptr(),
                                &mut cs,
                                target as i32,
                                lvl,
                            ),
                            r(
                                rst.ptr(),
                                src.as_ptr(),
                                rd.as_mut_ptr(),
                                &mut ru,
                                target as i32,
                                lvl,
                            ),
                        )
                    };
                    let ctx =
                        format!("HC destSize lvl={lvl} shape={shape:?} len={len} target={target}");
                    eq(&ctx, a, b);
                    eq(&format!("{ctx} srcSizePtr"), cs, ru);
                    eq_bytes(&ctx, &cd, &rd);
                }
            }
        }
    }
}

// ==================================================== rows 60-68, 77 streaming HC

struct HcOps {
    create: libloading::Symbol<'static, FnCreateHCStream>,
    free: libloading::Symbol<'static, FnFreeHCStream>,
    init: libloading::Symbol<'static, FnInitStreamHC>,
    reset: libloading::Symbol<'static, FnResetHC>,
    reset_fast: libloading::Symbol<'static, FnResetHC>,
    load: libloading::Symbol<'static, FnLoadDictHC>,
    attach: libloading::Symbol<'static, FnAttachHC>,
    cont: libloading::Symbol<'static, FnHCContinue>,
    cont_ds: libloading::Symbol<'static, FnHCContinueDestSize>,
    save: libloading::Symbol<'static, FnSaveDictHC>,
    favor: libloading::Symbol<'static, FnFavor>,
    set_level: libloading::Symbol<'static, FnSetLevel>,
}

fn hc_ops() -> (HcOps, HcOps) {
    let (a1, b1) = sym::<FnCreateHCStream>("LZ4_createStreamHC");
    let (a2, b2) = sym::<FnFreeHCStream>("LZ4_freeStreamHC");
    let (a3, b3) = sym::<FnInitStreamHC>("LZ4_initStreamHC");
    let (a4, b4) = sym::<FnResetHC>("LZ4_resetStreamHC");
    let (a5, b5) = sym::<FnResetHC>("LZ4_resetStreamHC_fast");
    let (a6, b6) = sym::<FnLoadDictHC>("LZ4_loadDictHC");
    let (a7, b7) = sym::<FnAttachHC>("LZ4_attach_HC_dictionary");
    let (a8, b8) = sym::<FnHCContinue>("LZ4_compress_HC_continue");
    let (a9, b9) = sym::<FnHCContinueDestSize>("LZ4_compress_HC_continue_destSize");
    let (a10, b10) = sym::<FnSaveDictHC>("LZ4_saveDictHC");
    let (a11, b11) = sym::<FnFavor>("LZ4_favorDecompressionSpeed");
    let (a12, b12) = sym::<FnSetLevel>("LZ4_setCompressionLevel");
    (
        HcOps {
            create: a1,
            free: a2,
            init: a3,
            reset: a4,
            reset_fast: a5,
            load: a6,
            attach: a7,
            cont: a8,
            cont_ds: a9,
            save: a10,
            favor: a11,
            set_level: a12,
        },
        HcOps {
            create: b1,
            free: b2,
            init: b3,
            reset: b4,
            reset_fast: b5,
            load: b6,
            attach: b7,
            cont: b8,
            cont_ds: b9,
            save: b10,
            favor: b11,
            set_level: b12,
        },
    )
}

#[derive(Clone, Copy, Debug)]
enum HcSeed {
    Fresh,
    Reset,
    ResetFast,
    Init,
    LoadDict(usize),
    /// `LZ4_attach_HC_dictionary` — only takes effect for non-lz4mid levels.
    AttachDict(usize),
    AttachNull,
}

/// Run a full streaming HC session on ONE implementation.
#[allow(clippy::too_many_arguments)]
fn run_hc(
    o: &HcOps,
    seed: HcSeed,
    lvl: i32,
    favor: i32,
    dict: &[u8],
    src: &[u8],
    chunks: &[usize],
    tight: bool,
    scratch: *mut u8,
    scratch_len: usize,
) -> (Vec<(i32, Vec<u8>)>, i32, Vec<u8>) {
    unsafe {
        let mut dict_stream: *mut u8 = std::ptr::null_mut();
        let s = match seed {
            HcSeed::Init => {
                let p = (o.init)(scratch, scratch_len);
                assert!(!p.is_null(), "initStreamHC returned NULL");
                (o.set_level)(p, lvl);
                p
            }
            _ => {
                let p = (o.create)();
                (o.set_level)(p, lvl);
                p
            }
        };
        match seed {
            HcSeed::Fresh | HcSeed::Init => {}
            HcSeed::Reset => (o.reset)(s, lvl),
            HcSeed::ResetFast => (o.reset_fast)(s, lvl),
            HcSeed::LoadDict(n) => {
                (o.load)(s, dict.as_ptr(), n as i32);
            }
            HcSeed::AttachDict(n) => {
                dict_stream = (o.create)();
                (o.set_level)(dict_stream, lvl);
                (o.load)(dict_stream, dict.as_ptr(), n as i32);
                (o.attach)(s, dict_stream);
            }
            HcSeed::AttachNull => (o.attach)(s, std::ptr::null()),
        }
        (o.favor)(s, favor);

        let mut out = Vec::new();
        let mut off = 0usize;
        for &clen in chunks {
            if off >= src.len() {
                break;
            }
            let clen = clen.min(src.len() - off);
            let bound = lz4_compress_bound(clen as i32) as usize;
            let cap = if tight { bound.saturating_sub(1) } else { bound };
            let mut d = vec![0xCCu8; cap + 16];
            let n = (o.cont)(
                s,
                src.as_ptr().add(off),
                d.as_mut_ptr(),
                clen as i32,
                cap as i32,
            );
            d.truncate(if n > 0 { n as usize } else { 0 });
            out.push((n, d));
            off += clen;
        }

        let mut safe = vec![0xDDu8; 70_000];
        let saved = (o.save)(s, safe.as_mut_ptr(), 65536);
        safe.truncate(if saved > 0 { saved as usize } else { 0 });

        if !matches!(seed, HcSeed::Init) {
            (o.free)(s);
        }
        if !dict_stream.is_null() {
            (o.free)(dict_stream);
        }
        (out, saved, safe)
    }
}

#[test]
fn row60_68_hc_streaming() {
    let (co, ro) = hc_ops();
    let mut rng = Rng::new(0x4643_0060);
    let mut csc = hc_state();
    let mut rsc = hc_state();

    let seeds = [
        HcSeed::Fresh,
        HcSeed::Reset,
        HcSeed::ResetFast,
        HcSeed::Init,
        HcSeed::LoadDict(0),
        HcSeed::LoadDict(1),
        HcSeed::LoadDict(1000),
        HcSeed::LoadDict(65535),
        HcSeed::LoadDict(65536),
        HcSeed::LoadDict(100_000),
        HcSeed::AttachDict(1000),
        HcSeed::AttachDict(65536),
        HcSeed::AttachNull,
    ];

    for seed in seeds {
        for &lvl in &LEVELS_S {
            for &favor in &[0i32, 1] {
                for &shape in &SHAPES {
                    for &tight in &[false, true] {
                        let dict = make_data(&mut rng, 100_000, shape);
                        let total = rng.range(1, 90_000);
                        let src = make_data(&mut rng, total, shape);
                        let mut chunks = Vec::new();
                        let mut acc = 0usize;
                        while acc < total {
                            let c = match rng.below(6) {
                                0 => 1,
                                1 => rng.range(1, 64),
                                2 => rng.range(1, 4096),
                                3 => 65536,
                                4 => 65535,
                                _ => rng.range(1, 30_000),
                            };
                            chunks.push(c);
                            acc += c;
                        }
                        csc.fill0();
                        rsc.fill0();
                        let (cb, cn, cs) = run_hc(
                            &co,
                            seed,
                            lvl,
                            favor,
                            &dict,
                            &src,
                            &chunks,
                            tight,
                            csc.ptr(),
                            LZ4_STREAMHC_SIZE + 128,
                        );
                        let (rb, rn, rs) = run_hc(
                            &ro,
                            seed,
                            lvl,
                            favor,
                            &dict,
                            &src,
                            &chunks,
                            tight,
                            rsc.ptr(),
                            LZ4_STREAMHC_SIZE + 128,
                        );
                        let ctx = format!(
                            "HC stream seed={seed:?} lvl={lvl} favor={favor} shape={shape:?} tight={tight} total={total}"
                        );
                        eq(&format!("{ctx} nblocks"), cb.len(), rb.len());
                        for (i, (a, b)) in cb.iter().zip(rb.iter()).enumerate() {
                            eq(&format!("{ctx} ret[{i}]"), a.0, b.0);
                            eq_bytes(&format!("{ctx} block[{i}]"), &a.1, &b.1);
                        }
                        eq(&format!("{ctx} saveDictHC ret"), cn, rn);
                        eq_bytes(&format!("{ctx} saveDictHC"), &cs, &rs);
                    }
                }
            }
        }
    }
}

#[test]
fn row61_hc_continue_dest_size() {
    let (co, ro) = hc_ops();
    let mut rng = Rng::new(0x4643_0061);
    for &lvl in &LEVELS_S {
        for &shape in &SHAPES {
            for &dictlen in &[0usize, 1000, 65536] {
                for _ in 0..3 {
                    let dict = make_data(&mut rng, dictlen.max(1), shape);
                    let total = rng.range(1, 60_000);
                    let src = make_data(&mut rng, total, shape);
                    let targets: Vec<usize> = vec![
                        1,
                        2,
                        13,
                        64,
                        total / 8 + 1,
                        total / 4 + 1,
                        total / 2 + 1,
                        total + 100,
                    ];
                    let mut got = Vec::new();
                    for o in [&co, &ro] {
                        unsafe {
                            let s = (o.create)();
                            (o.set_level)(s, lvl);
                            if dictlen > 0 {
                                (o.load)(s, dict.as_ptr(), dictlen as i32);
                            }
                            let mut rec = Vec::new();
                            let mut off = 0usize;
                            for &t in &targets {
                                let mut d = vec![0xBBu8; t + 16];
                                let mut avail = (total - off) as i32;
                                let n = (o.cont_ds)(
                                    s,
                                    src.as_ptr().add(off),
                                    d.as_mut_ptr(),
                                    &mut avail,
                                    t as i32,
                                );
                                rec.push((n, avail, d));
                                if n > 0 && avail > 0 {
                                    off += avail as usize;
                                }
                                if off >= total {
                                    break;
                                }
                            }
                            (o.free)(s);
                            got.push(rec);
                        }
                    }
                    let ctx = format!(
                        "HC continue_destSize lvl={lvl} shape={shape:?} dict={dictlen} total={total}"
                    );
                    eq(&format!("{ctx} nsteps"), got[0].len(), got[1].len());
                    for (i, (a, b)) in got[0].iter().zip(got[1].iter()).enumerate() {
                        eq(&format!("{ctx} ret[{i}]"), a.0, b.0);
                        eq(&format!("{ctx} consumed[{i}]"), a.1, b.1);
                        eq_bytes(&format!("{ctx} out[{i}]"), &a.2, &b.2);
                    }
                }
            }
        }
    }
}

#[test]
fn row64_save_dict_hc_sizes() {
    let (co, ro) = hc_ops();
    let mut rng = Rng::new(0x4643_0064);
    for &lvl in &LEVELS_S {
        for &maxdict in &[0i32, 1, 100, 4096, 65535, 65536, 100_000] {
            for &shape in &SHAPES {
                let total = rng.range(1, 70_000);
                let src = make_data(&mut rng, total, shape);
                let chunk = total / 3 + 1;
                let mut got = Vec::new();
                for o in [&co, &ro] {
                    unsafe {
                        let s = (o.create)();
                        (o.set_level)(s, lvl);
                        let mut off = 0usize;
                        while off < total {
                            let c = chunk.min(total - off);
                            let bound = lz4_compress_bound(c as i32) as usize;
                            let mut d = vec![0u8; bound + 8];
                            (o.cont)(
                                s,
                                src.as_ptr().add(off),
                                d.as_mut_ptr(),
                                c as i32,
                                bound as i32,
                            );
                            off += c;
                        }
                        let mut safe = vec![0x7Eu8; 120_000];
                        let n = (o.save)(s, safe.as_mut_ptr(), maxdict);
                        (o.free)(s);
                        safe.truncate(if n > 0 { n as usize } else { 0 });
                        got.push((n, safe));
                    }
                }
                let ctx = format!("saveDictHC lvl={lvl} maxdict={maxdict} shape={shape:?}");
                eq(&format!("{ctx} ret"), got[0].0, got[1].0);
                eq_bytes(&ctx, &got[0].1, &got[1].1);
            }
        }
    }
}

#[test]
fn row65_66_favor_and_level_changes() {
    let (co, ro) = hc_ops();
    let mut rng = Rng::new(0x4643_0065);
    // Mid-stream level and favor changes: the C keeps compressing with the new
    // setting, so the emitted blocks must match exactly.
    for &shape in &SHAPES {
        for _ in 0..10 {
            let total = rng.range(1000, 60_000);
            let src = make_data(&mut rng, total, shape);
            let nchunks = rng.range(2, 8);
            let chunk = total / nchunks + 1;
            // A script of (level, favor) applied before each chunk.
            let script: Vec<(i32, i32)> = (0..nchunks + 2)
                .map(|_| {
                    (
                        *[0i32, 1, 2, 3, 9, 10, 12, -1, 13]
                            .get(rng.below(9))
                            .unwrap(),
                        (rng.below(2)) as i32,
                    )
                })
                .collect();
            let mut got = Vec::new();
            for o in [&co, &ro] {
                unsafe {
                    let s = (o.create)();
                    let mut rec = Vec::new();
                    let mut off = 0usize;
                    let mut i = 0usize;
                    while off < total {
                        let (lvl, fav) = script[i.min(script.len() - 1)];
                        (o.set_level)(s, lvl);
                        (o.favor)(s, fav);
                        let c = chunk.min(total - off);
                        let bound = lz4_compress_bound(c as i32) as usize;
                        let mut d = vec![0xABu8; bound + 8];
                        let n = (o.cont)(
                            s,
                            src.as_ptr().add(off),
                            d.as_mut_ptr(),
                            c as i32,
                            bound as i32,
                        );
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        rec.push((n, d));
                        off += c;
                        i += 1;
                    }
                    (o.free)(s);
                    got.push(rec);
                }
            }
            let ctx = format!("HC level/favor script shape={shape:?} total={total} {script:?}");
            eq(&format!("{ctx} nblocks"), got[0].len(), got[1].len());
            for (i, (a, b)) in got[0].iter().zip(got[1].iter()).enumerate() {
                eq(&format!("{ctx} ret[{i}]"), a.0, b.0);
                eq_bytes(&format!("{ctx} block[{i}]"), &a.1, &b.1);
            }
        }
    }
}

// ============================================================ rows 69-76 obsolete

#[test]
fn row71_74_obsolete_hc_oneshot() {
    let (c1, r1) = sym::<FnHC3>("LZ4_compressHC");
    let (c2, r2) = sym::<FnHC4>("LZ4_compressHC_limitedOutput");
    let (c3, r3) = sym::<FnHC4>("LZ4_compressHC2");
    let (c4, r4) = sym::<FnHC5>("LZ4_compressHC2_limitedOutput");
    let (c5, r5) = sym::<FnHCState4>("LZ4_compressHC_withStateHC");
    let (c6, r6) = sym::<FnHCState5>("LZ4_compressHC_limitedOutput_withStateHC");
    let (c7, r7) = sym::<FnHCState5>("LZ4_compressHC2_withStateHC");
    let (c8, r8) = sym::<FnHCState6>("LZ4_compressHC2_limitedOutput_withStateHC");

    let mut rng = Rng::new(0x4643_0071);
    let mut cst = hc_state();
    let mut rst = hc_state();

    for &shape in &SHAPES {
        for _ in 0..8 {
            let len = rng.range(0, 50_000);
            let src = make_data(&mut rng, len, shape);
            let bound = lz4_compress_bound(len as i32) as usize;

            // LZ4_compressHC (no level)
            let (mut cd, mut rd) = two_bufs(bound + 8);
            let (a, b) = unsafe {
                (
                    c1(src.as_ptr(), cd.as_mut_ptr(), len as i32),
                    r1(src.as_ptr(), rd.as_mut_ptr(), len as i32),
                )
            };
            let ctx = format!("compressHC shape={shape:?} len={len}");
            eq(&ctx, a, b);
            eq_bytes(&ctx, &cd, &rd);

            // withStateHC (no level)
            cst.fill0();
            rst.fill0();
            let (mut cd, mut rd) = two_bufs(bound + 8);
            let (a, b) = unsafe {
                (
                    c5(cst.ptr(), src.as_ptr(), cd.as_mut_ptr(), len as i32),
                    r5(rst.ptr(), src.as_ptr(), rd.as_mut_ptr(), len as i32),
                )
            };
            let ctx = format!("compressHC_withStateHC shape={shape:?} len={len}");
            eq(&ctx, a, b);
            eq_bytes(&ctx, &cd, &rd);

            for cap in [0usize, 1, 13, len / 2, bound] {
                let (mut cd, mut rd) = two_bufs(cap + 16);
                let (a, b) = unsafe {
                    (
                        c2(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap as i32),
                        r2(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap as i32),
                    )
                };
                let ctx = format!("compressHC_limitedOutput len={len} cap={cap}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd, &rd);

                cst.fill0();
                rst.fill0();
                let (mut cd, mut rd) = two_bufs(cap + 16);
                let (a, b) = unsafe {
                    (
                        c6(
                            cst.ptr(),
                            src.as_ptr(),
                            cd.as_mut_ptr(),
                            len as i32,
                            cap as i32,
                        ),
                        r6(
                            rst.ptr(),
                            src.as_ptr(),
                            rd.as_mut_ptr(),
                            len as i32,
                            cap as i32,
                        ),
                    )
                };
                let ctx = format!("compressHC_limitedOutput_withStateHC len={len} cap={cap}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd, &rd);
            }

            for &lvl in &LEVELS {
                let (mut cd, mut rd) = two_bufs(bound + 8);
                let (a, b) = unsafe {
                    (
                        c3(src.as_ptr(), cd.as_mut_ptr(), len as i32, lvl),
                        r3(src.as_ptr(), rd.as_mut_ptr(), len as i32, lvl),
                    )
                };
                let ctx = format!("compressHC2 lvl={lvl} len={len}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd, &rd);

                cst.fill0();
                rst.fill0();
                let (mut cd, mut rd) = two_bufs(bound + 8);
                let (a, b) = unsafe {
                    (
                        c7(cst.ptr(), src.as_ptr(), cd.as_mut_ptr(), len as i32, lvl),
                        r7(rst.ptr(), src.as_ptr(), rd.as_mut_ptr(), len as i32, lvl),
                    )
                };
                let ctx = format!("compressHC2_withStateHC lvl={lvl} len={len}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd, &rd);

                for cap in [0usize, 1, len / 2, bound] {
                    let (mut cd, mut rd) = two_bufs(cap + 16);
                    let (a, b) = unsafe {
                        (
                            c4(
                                src.as_ptr(),
                                cd.as_mut_ptr(),
                                len as i32,
                                cap as i32,
                                lvl,
                            ),
                            r4(
                                src.as_ptr(),
                                rd.as_mut_ptr(),
                                len as i32,
                                cap as i32,
                                lvl,
                            ),
                        )
                    };
                    let ctx = format!("compressHC2_limitedOutput lvl={lvl} len={len} cap={cap}");
                    eq(&ctx, a, b);
                    eq_bytes(&ctx, &cd, &rd);

                    cst.fill0();
                    rst.fill0();
                    let (mut cd, mut rd) = two_bufs(cap + 16);
                    let (a, b) = unsafe {
                        (
                            c8(
                                cst.ptr(),
                                src.as_ptr(),
                                cd.as_mut_ptr(),
                                len as i32,
                                cap as i32,
                                lvl,
                            ),
                            r8(
                                rst.ptr(),
                                src.as_ptr(),
                                rd.as_mut_ptr(),
                                len as i32,
                                cap as i32,
                                lvl,
                            ),
                        )
                    };
                    let ctx = format!(
                        "compressHC2_limitedOutput_withStateHC lvl={lvl} len={len} cap={cap}"
                    );
                    eq(&ctx, a, b);
                    eq_bytes(&ctx, &cd, &rd);
                }
            }
        }
    }
}

#[test]
fn row69_70_75_76_obsolete_hc_streaming() {
    let (cc, rc) = sym::<FnCreateHC>("LZ4_createHC");
    let (cfr, rfr) = sym::<FnFreeHC>("LZ4_freeHC");
    let (csl, rsl) = sym::<FnSlideHC>("LZ4_slideInputBufferHC");
    let (crs, rrs) = sym::<FnResetStreamStateHC>("LZ4_resetStreamStateHC");
    let (cc1, rc1) = sym::<FnHCState4>("LZ4_compressHC_continue");
    let (cc2, rc2) = sym::<FnHCState5>("LZ4_compressHC_limitedOutput_continue");
    let (cc3, rc3) = sym::<FnHCState5>("LZ4_compressHC2_continue");
    let (cc4, rc4) = sym::<FnHCState6>("LZ4_compressHC2_limitedOutput_continue");

    let mut rng = Rng::new(0x4643_0069);
    for &shape in &SHAPES {
        for _ in 0..5 {
            let total = rng.range(1, 40_000);
            let src = make_data(&mut rng, total, shape);
            let nchunks = rng.range(1, 5);
            let chunk = total / nchunks + 1;

            // rows 70/75/76: LZ4_createHC + slideInputBufferHC lifecycle
            for &lvl in &[0i32, 1, 3, 9, 12] {
                let mut got = Vec::new();
                for (create, free, slide, k1, k2, k3, k4) in [
                    (&cc, &cfr, &csl, &cc1, &cc2, &cc3, &cc4),
                    (&rc, &rfr, &rsl, &rc1, &rc2, &rc3, &rc4),
                ] {
                    unsafe {
                        let mut inbuf = vec![0u8; 65536 + chunk + 16];
                        let s = create(inbuf.as_ptr());
                        assert!(!s.is_null());
                        let mut rec = Vec::new();
                        let mut off = 0usize;
                        let mut cursor = 0usize;
                        let mut which = 0usize;
                        while off < total {
                            let c = chunk.min(total - off);
                            if cursor + c > inbuf.len() {
                                let p = slide(s);
                                assert!(!p.is_null());
                                cursor = p.offset_from(inbuf.as_ptr()) as usize;
                            }
                            inbuf[cursor..cursor + c].copy_from_slice(&src[off..off + c]);
                            let bound = lz4_compress_bound(c as i32) as usize;
                            let cap = if which % 2 == 0 { bound } else { bound / 2 + 1 };
                            let mut d = vec![0x99u8; bound + 16];
                            let ip = inbuf.as_ptr().add(cursor);
                            let n = match which % 4 {
                                0 => k1(s, ip, d.as_mut_ptr(), c as i32),
                                1 => k2(s, ip, d.as_mut_ptr(), c as i32, cap as i32),
                                2 => k3(s, ip, d.as_mut_ptr(), c as i32, lvl),
                                _ => k4(s, ip, d.as_mut_ptr(), c as i32, cap as i32, lvl),
                            };
                            rec.push((n, d));
                            cursor += c;
                            off += c;
                            which += 1;
                        }
                        free(s);
                        got.push(rec);
                    }
                }
                let ctx = format!("obsolete HC stream lvl={lvl} shape={shape:?} total={total}");
                eq(&format!("{ctx} nblocks"), got[0].len(), got[1].len());
                for (i, (a, b)) in got[0].iter().zip(got[1].iter()).enumerate() {
                    eq(&format!("{ctx} ret[{i}]"), a.0, b.0);
                    eq_bytes(&format!("{ctx} block[{i}]"), &a.1, &b.1);
                }
            }

            // row 69: LZ4_resetStreamStateHC
            let mut got = Vec::new();
            for (reset, cont) in [(&crs, &cc1), (&rrs, &rc1)] {
                let mut st = hc_state();
                let mut inbuf = vec![0u8; total + 16];
                inbuf[..total].copy_from_slice(&src);
                unsafe {
                    let rr = reset(st.ptr(), inbuf.as_mut_ptr());
                    let mut rec = vec![(rr, Vec::new())];
                    let mut off = 0usize;
                    while off < total {
                        let c = chunk.min(total - off);
                        let bound = lz4_compress_bound(c as i32) as usize;
                        let mut d = vec![0x88u8; bound + 8];
                        let n = cont(
                            st.ptr(),
                            inbuf.as_ptr().add(off),
                            d.as_mut_ptr(),
                            c as i32,
                        );
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        rec.push((n, d));
                        off += c;
                    }
                    got.push(rec);
                }
            }
            let ctx = format!("resetStreamStateHC shape={shape:?} total={total}");
            eq(&format!("{ctx} nblocks"), got[0].len(), got[1].len());
            for (i, (a, b)) in got[0].iter().zip(got[1].iter()).enumerate() {
                eq(&format!("{ctx} ret[{i}]"), a.0, b.0);
                eq_bytes(&format!("{ctx} block[{i}]"), &a.1, &b.1);
            }
        }
    }
}

/// Row 77: `LZ4HC_searchExtDict` is only reachable through an HC stream that
/// has an ATTACHED dictionary context and a level above lz4mid. Drive that
/// path hard with dict/block content deliberately sharing long spans.
#[test]
fn row77_hc_search_ext_dict() {
    let (co, ro) = hc_ops();
    // Confirm the symbol is exported by both, even though it is called indirectly.
    let _ = sym::<unsafe extern "C" fn()>("LZ4HC_searchExtDict");

    let mut rng = Rng::new(0x4643_0077);
    for &lvl in &[3i32, 6, 9, 10, 11, 12] {
        for &shape in &SHAPES {
            for &dictlen in &[64usize, 1000, 65535, 65536] {
                for _ in 0..3 {
                    let dict = make_data(&mut rng, dictlen, shape);
                    let total = rng.range(64, 40_000);
                    let mut src = make_data(&mut rng, total, shape);
                    // Force long ext-dict matches by copying dict spans into src.
                    let mut i = 0usize;
                    while i + 32 < src.len() {
                        if rng.bool() {
                            let n = rng.range(16, 200).min(dict.len()).min(src.len() - i);
                            let s0 = rng.below(dict.len().saturating_sub(n) + 1);
                            src[i..i + n].copy_from_slice(&dict[s0..s0 + n]);
                            i += n;
                        } else {
                            i += rng.range(1, 64);
                        }
                    }
                    let chunk = total / (rng.range(1, 5)) + 1;
                    let mut got = Vec::new();
                    for o in [&co, &ro] {
                        unsafe {
                            let ds = (o.create)();
                            (o.set_level)(ds, lvl);
                            (o.load)(ds, dict.as_ptr(), dictlen as i32);
                            let s = (o.create)();
                            (o.set_level)(s, lvl);
                            (o.attach)(s, ds);
                            let mut rec = Vec::new();
                            let mut off = 0usize;
                            while off < total {
                                let c = chunk.min(total - off);
                                let bound = lz4_compress_bound(c as i32) as usize;
                                let mut d = vec![0x77u8; bound + 8];
                                let n = (o.cont)(
                                    s,
                                    src.as_ptr().add(off),
                                    d.as_mut_ptr(),
                                    c as i32,
                                    bound as i32,
                                );
                                d.truncate(if n > 0 { n as usize } else { 0 });
                                rec.push((n, d));
                                off += c;
                            }
                            (o.free)(s);
                            (o.free)(ds);
                            got.push(rec);
                        }
                    }
                    let ctx = format!(
                        "searchExtDict lvl={lvl} shape={shape:?} dict={dictlen} total={total}"
                    );
                    eq(&format!("{ctx} nblocks"), got[0].len(), got[1].len());
                    for (i, (a, b)) in got[0].iter().zip(got[1].iter()).enumerate() {
                        eq(&format!("{ctx} ret[{i}]"), a.0, b.0);
                        eq_bytes(&format!("{ctx} block[{i}]"), &a.1, &b.1);
                    }
                }
            }
        }
    }
}
