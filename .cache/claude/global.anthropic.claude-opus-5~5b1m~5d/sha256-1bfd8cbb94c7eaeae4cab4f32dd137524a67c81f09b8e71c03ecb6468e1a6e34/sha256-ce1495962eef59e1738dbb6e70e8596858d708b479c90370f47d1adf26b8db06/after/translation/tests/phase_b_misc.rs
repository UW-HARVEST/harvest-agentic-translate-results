//! Phase B — CONFIGS.md rows 166..171: the deprecated ZBUFF streaming API,
//! the ZSTDMT surface in a **non**-`ZSTD_MULTITHREAD` build, and the POOL
//! surface in the same build.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_ulonglong, c_void};

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnCreateAdv = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnZbInit = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnZbInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int) -> usize;
type FnZbInitAdv = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    usize,
    ZSTD_parameters,
    c_ulonglong,
) -> usize;
type FnZbCont =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut usize, *const c_void, *mut usize) -> usize;
type FnZbFlush = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut usize) -> usize;
type FnZbDInit = unsafe extern "C" fn(*mut c_void) -> usize;
type FnZbDInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;

/// A pair of ZBUFF contexts (one per library).
struct ZbPair {
    c: *mut c_void,
    r: *mut c_void,
    fc: FnFree,
    fr: FnFree,
}
impl Drop for ZbPair {
    fn drop(&mut self) {
        unsafe {
            (self.fc)(self.c);
            (self.fr)(self.r);
        }
    }
}
unsafe fn zb_pair(create: &str, free: &str) -> ZbPair {
    let (cc, cr) = duo::<FnCreate>(create);
    let (fc, fr) = duo::<FnFree>(free);
    let c = cc();
    let r = cr();
    assert!(!c.is_null() && !r.is_null(), "{create} returned NULL");
    ZbPair { c, r, fc, fr }
}
unsafe fn zb_pair_adv(create: &str, free: &str) -> ZbPair {
    let (cc, cr) = duo::<FnCreateAdv>(create);
    let (fc, fr) = duo::<FnFree>(free);
    let m = ZSTD_customMem::default();
    let c = cc(m);
    let r = cr(m);
    assert!(!c.is_null() && !r.is_null(), "{create} returned NULL");
    ZbPair { c, r, fc, fr }
}

// ------------------------------------------------------------------ row 168

#[test]
fn row168_zbuff_scalars() {
    unsafe {
        for n in [
            "ZBUFF_recommendedCInSize",
            "ZBUFF_recommendedCOutSize",
            "ZBUFF_recommendedDInSize",
            "ZBUFF_recommendedDOutSize",
        ] {
            let (a, b) = duo::<FnSizeT0>(n);
            eqv(n, a(), b());
        }
        let (ic, ir) = duo::<FnIsError>("ZBUFF_isError");
        let (nc, nr) = duo::<FnErrName>("ZBUFF_getErrorName");
        let mut vals: Vec<usize> = vec![0, 1, 2, 70, usize::MAX];
        for k in 0..140usize {
            vals.push(usize::MAX - k);
        }
        let mut rng = Rng::new(168);
        for _ in 0..500 {
            vals.push(rng.next_u64() as usize);
        }
        for v in vals {
            eqv(&format!("ZBUFF_isError({v})"), ic(v), ir(v));
            eqv(&format!("ZBUFF_getErrorName({v})"), cstr(nc(v)), cstr(nr(v)));
        }
    }
}

// ------------------------------------------------------------------ rows 166, 167

/// Drive a whole ZBUFF compress → ZBUFF decompress round trip on both
/// libraries in lockstep, with the given chunk sizes.
#[allow(clippy::too_many_arguments)]
unsafe fn zbuff_roundtrip(
    tag: &str,
    src: &[u8],
    level: c_int,
    in_chunk: usize,
    out_chunk: usize,
    dict: Option<&[u8]>,
    params: Option<(ZSTD_parameters, c_ulonglong)>,
    flush_every: usize,
) {
    let cz = zb_pair("ZBUFF_createCCtx", "ZBUFF_freeCCtx");
    let (cont_c, cont_r) = duo::<FnZbCont>("ZBUFF_compressContinue");
    let (fl_c, fl_r) = duo::<FnZbFlush>("ZBUFF_compressFlush");
    let (end_c, end_r) = duo::<FnZbFlush>("ZBUFF_compressEnd");

    // init
    match (dict, params) {
        (_, Some((p, pledged))) => {
            let (a, b) = duo::<FnZbInitAdv>("ZBUFF_compressInit_advanced");
            let (dp, ds) = match dict {
                Some(d) if !d.is_empty() => (d.as_ptr() as *const c_void, d.len()),
                _ => (std::ptr::null(), 0),
            };
            eqv(
                &format!("{tag} ZBUFF_compressInit_advanced"),
                a(cz.c, dp, ds, p, pledged),
                b(cz.r, dp, ds, p, pledged),
            );
        }
        (Some(d), None) => {
            let (a, b) = duo::<FnZbInitDict>("ZBUFF_compressInitDictionary");
            let (dp, ds) = if d.is_empty() {
                (std::ptr::null(), 0)
            } else {
                (d.as_ptr() as *const c_void, d.len())
            };
            eqv(
                &format!("{tag} ZBUFF_compressInitDictionary"),
                a(cz.c, dp, ds, level),
                b(cz.r, dp, ds, level),
            );
        }
        (None, None) => {
            let (a, b) = duo::<FnZbInit>("ZBUFF_compressInit");
            eqv(
                &format!("{tag} ZBUFF_compressInit"),
                a(cz.c, level),
                b(cz.r, level),
            );
        }
    }

    let mut framec: Vec<u8> = Vec::new();
    let mut framer: Vec<u8> = Vec::new();
    let mut pos = 0usize;
    let mut iter = 0usize;
    while pos < src.len() {
        iter += 1;
        let want = in_chunk.min(src.len() - pos);
        let mut obc = vec![0xA3u8; out_chunk.max(1)];
        let mut obr = vec![0xA3u8; out_chunk.max(1)];
        let mut dcc = out_chunk;
        let mut dcr = out_chunk;
        let mut scc = want;
        let mut scr = want;
        let a = cont_c(
            cz.c,
            obc.as_mut_ptr() as *mut c_void,
            &mut dcc,
            src[pos..].as_ptr() as *const c_void,
            &mut scc,
        );
        let b = cont_r(
            cz.r,
            obr.as_mut_ptr() as *mut c_void,
            &mut dcr,
            src[pos..].as_ptr() as *const c_void,
            &mut scr,
        );
        eqv(&format!("{tag} continue#{iter} ret"), a, b);
        eqv(&format!("{tag} continue#{iter} dstConsumed"), dcc, dcr);
        eqv(&format!("{tag} continue#{iter} srcConsumed"), scc, scr);
        eqbuf(
            &format!("{tag} continue#{iter} out"),
            &obc[..dcc.min(obc.len())],
            &obr[..dcr.min(obr.len())],
        );
        if is_err(a) {
            // e.g. ZBUFF_compressInit_advanced() with a pledged srcSize that is
            // smaller than the data actually fed -> ZSTD_error_srcSize_wrong.
            // Both libraries already agreed on the code above; nothing more to
            // compare for this configuration.
            return;
        }
        framec.extend_from_slice(&obc[..dcc.min(obc.len())]);
        framer.extend_from_slice(&obr[..dcr.min(obr.len())]);
        pos += scc;
        if scc == 0 && dcc == 0 {
            break;
        }
        if flush_every != 0 && iter % flush_every == 0 {
            loop {
                let mut obc = vec![0u8; out_chunk.max(1)];
                let mut obr = vec![0u8; out_chunk.max(1)];
                let mut dcc = out_chunk;
                let mut dcr = out_chunk;
                let a = fl_c(cz.c, obc.as_mut_ptr() as *mut c_void, &mut dcc);
                let b = fl_r(cz.r, obr.as_mut_ptr() as *mut c_void, &mut dcr);
                eqv(&format!("{tag} flush#{iter} ret"), a, b);
                eqv(&format!("{tag} flush#{iter} dstConsumed"), dcc, dcr);
                eqbuf(
                    &format!("{tag} flush#{iter} out"),
                    &obc[..dcc.min(obc.len())],
                    &obr[..dcr.min(obr.len())],
                );
                framec.extend_from_slice(&obc[..dcc.min(obc.len())]);
                framer.extend_from_slice(&obr[..dcr.min(obr.len())]);
                if is_err(a) || a == 0 {
                    break;
                }
            }
        }
    }
    // end of frame
    let mut guard = 0;
    loop {
        guard += 1;
        let mut obc = vec![0u8; out_chunk.max(1)];
        let mut obr = vec![0u8; out_chunk.max(1)];
        let mut dcc = out_chunk;
        let mut dcr = out_chunk;
        let a = end_c(cz.c, obc.as_mut_ptr() as *mut c_void, &mut dcc);
        let b = end_r(cz.r, obr.as_mut_ptr() as *mut c_void, &mut dcr);
        eqv(&format!("{tag} end#{guard} ret"), a, b);
        eqv(&format!("{tag} end#{guard} dstConsumed"), dcc, dcr);
        eqbuf(
            &format!("{tag} end#{guard} out"),
            &obc[..dcc.min(obc.len())],
            &obr[..dcr.min(obr.len())],
        );
        framec.extend_from_slice(&obc[..dcc.min(obc.len())]);
        framer.extend_from_slice(&obr[..dcr.min(obr.len())]);
        if is_err(a) || a == 0 || guard > 5000 {
            break;
        }
    }
    eqbuf(&format!("{tag} whole frame"), &framec, &framer);

    // ---- decompress it back with ZBUFF, again in lockstep
    let dz = zb_pair("ZBUFF_createDCtx", "ZBUFF_freeDCtx");
    match dict {
        Some(d) if !d.is_empty() => {
            let (a, b) = duo::<FnZbDInitDict>("ZBUFF_decompressInitDictionary");
            eqv(
                &format!("{tag} ZBUFF_decompressInitDictionary"),
                a(dz.c, d.as_ptr() as *const c_void, d.len()),
                b(dz.r, d.as_ptr() as *const c_void, d.len()),
            );
        }
        _ => {
            let (a, b) = duo::<FnZbDInit>("ZBUFF_decompressInit");
            eqv(
                &format!("{tag} ZBUFF_decompressInit"),
                a(dz.c),
                b(dz.r),
            );
        }
    }
    let (dcont_c, dcont_r) = duo::<FnZbCont>("ZBUFF_decompressContinue");
    let mut outc: Vec<u8> = Vec::new();
    let mut outr: Vec<u8> = Vec::new();
    let mut ipos = 0usize;
    let mut it = 0usize;
    loop {
        it += 1;
        let want = in_chunk.min(framec.len() - ipos);
        let mut obc = vec![0u8; out_chunk.max(1)];
        let mut obr = vec![0u8; out_chunk.max(1)];
        let mut dcc = out_chunk;
        let mut dcr = out_chunk;
        let mut scc = want;
        let mut scr = want;
        let a = dcont_c(
            dz.c,
            obc.as_mut_ptr() as *mut c_void,
            &mut dcc,
            framec[ipos..].as_ptr() as *const c_void,
            &mut scc,
        );
        let b = dcont_r(
            dz.r,
            obr.as_mut_ptr() as *mut c_void,
            &mut dcr,
            framer[ipos..].as_ptr() as *const c_void,
            &mut scr,
        );
        eqv(&format!("{tag} dcontinue#{it} ret"), a, b);
        eqv(&format!("{tag} dcontinue#{it} dstConsumed"), dcc, dcr);
        eqv(&format!("{tag} dcontinue#{it} srcConsumed"), scc, scr);
        eqbuf(
            &format!("{tag} dcontinue#{it} out"),
            &obc[..dcc.min(obc.len())],
            &obr[..dcr.min(obr.len())],
        );
        outc.extend_from_slice(&obc[..dcc.min(obc.len())]);
        outr.extend_from_slice(&obr[..dcr.min(obr.len())]);
        ipos += scc;
        if is_err(a) || a == 0 || it > 20000 || (scc == 0 && dcc == 0) {
            break;
        }
    }
    eqbuf(&format!("{tag} decompressed"), &outc, &outr);
    assert_eq!(&outc[..], src, "{tag}: ZBUFF round-trip content mismatch");
}

#[test]
fn row166_167_zbuff_roundtrip() {
    unsafe {
        let mut rng = Rng::new(166);
        let dict = gen_class(4, 4096, 1);
        for lvl in [1, 3, 9, 19] {
            for &(ic, oc) in &[
                (1usize, 1usize),
                (3, 7),
                (17, 4096),
                (1024, 1024),
                (1 << 16, 1 << 16),
                (1 << 20, 1 << 20),
            ] {
                let cls = rng.below(N_CLASSES);
                let sz = [0usize, 1, 33, 5000, 70_000][rng.below(5)];
                let src = gen_class(cls, sz, rng.next_u64());
                zbuff_roundtrip(
                    &format!("row166 lvl={lvl} in={ic} out={oc} cls={cls} sz={sz}"),
                    &src,
                    lvl,
                    ic,
                    oc,
                    None,
                    None,
                    0,
                );
                zbuff_roundtrip(
                    &format!("row166dict lvl={lvl} in={ic} out={oc} cls={cls} sz={sz}"),
                    &src,
                    lvl,
                    ic,
                    oc,
                    Some(&dict),
                    None,
                    0,
                );
                zbuff_roundtrip(
                    &format!("row166flush lvl={lvl} in={ic} out={oc} cls={cls} sz={sz}"),
                    &src,
                    lvl,
                    ic,
                    oc,
                    None,
                    None,
                    2,
                );
            }
        }
        // ZBUFF_compressInit_advanced over a ZSTD_parameters grid
        let (gp, _) =
            duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_parameters>("ZSTD_getParams");
        for lvl in [1, 5, 12, 19] {
            for pledged in [0u64, 1000, ZSTD_CONTENTSIZE_UNKNOWN] {
                let mut p = gp(lvl, 0, 0);
                p.fParams.checksumFlag = (lvl % 2) as c_int;
                p.fParams.contentSizeFlag = ((lvl + 1) % 2) as c_int;
                let sz = 4000usize;
                let src = gen_class(4, sz, lvl as u64);
                zbuff_roundtrip(
                    &format!("row166adv lvl={lvl} pledged={pledged}"),
                    &src,
                    lvl,
                    1024,
                    1024,
                    None,
                    Some((p, if pledged == 0 { sz as u64 } else { pledged })),
                    0,
                );
            }
        }
        // the *_advanced constructors with an all-NULL ZSTD_customMem
        {
            let cz = zb_pair_adv("ZBUFF_createCCtx_advanced", "ZBUFF_freeCCtx");
            let (a, b) = duo::<FnZbInit>("ZBUFF_compressInit");
            eqv(
                "row166 createCCtx_advanced + compressInit",
                a(cz.c, 3),
                b(cz.r, 3),
            );
            let dz = zb_pair_adv("ZBUFF_createDCtx_advanced", "ZBUFF_freeDCtx");
            let (a, b) = duo::<FnZbDInit>("ZBUFF_decompressInit");
            eqv(
                "row167 createDCtx_advanced + decompressInit",
                a(dz.c),
                b(dz.r),
            );
        }
        // free(NULL) must behave the same
        {
            let (fc, fr) = duo::<FnFree>("ZBUFF_freeCCtx");
            eqv(
                "ZBUFF_freeCCtx(NULL)",
                fc(std::ptr::null_mut()),
                fr(std::ptr::null_mut()),
            );
            let (fc, fr) = duo::<FnFree>("ZBUFF_freeDCtx");
            eqv(
                "ZBUFF_freeDCtx(NULL)",
                fc(std::ptr::null_mut()),
                fr(std::ptr::null_mut()),
            );
        }
    }
}

// ------------------------------------------------------------------ row 169

#[test]
fn row169_zstdmt_non_mt_build() {
    unsafe {
        // The build does NOT define ZSTD_MULTITHREAD, so
        // ZSTDMT_createCCtx_advanced() always returns NULL (zstdmt_compress.c
        // L992-1001). Consequently the only ZSTDMT entry points that can be
        // reached with a defined result are the ones that explicitly accept
        // NULL: freeCCtx and sizeof_CCtx. All the others dereference `mtctx`
        // unconditionally, so passing NULL is UB in the C too and is NOT
        // differentiable (both libraries fault). This is recorded in
        // CONFIGS.md.
        let (cc, cr) = duo::<
            unsafe extern "C" fn(c_uint_, ZSTD_customMem, *mut c_void) -> *mut c_void,
        >("ZSTDMT_createCCtx_advanced");
        let m = ZSTD_customMem::default();
        for nb in [0u32, 1, 2, 4, 200, u32::MAX] {
            let a = cc(nb, m, std::ptr::null_mut());
            let b = cr(nb, m, std::ptr::null_mut());
            eqv(&format!("ZSTDMT_createCCtx_advanced({nb}) null?"), a.is_null(), b.is_null());
            eqv(&format!("ZSTDMT_createCCtx_advanced({nb}) ptr"), a as usize, b as usize);
        }
        let (fc, fr) = duo::<FnFree>("ZSTDMT_freeCCtx");
        eqv(
            "ZSTDMT_freeCCtx(NULL)",
            fc(std::ptr::null_mut()),
            fr(std::ptr::null_mut()),
        );
        let (sc, sr) = duo::<FnFree>("ZSTDMT_sizeof_CCtx");
        eqv(
            "ZSTDMT_sizeof_CCtx(NULL)",
            sc(std::ptr::null_mut()),
            sr(std::ptr::null_mut()),
        );
        // and the MT parameters remain settable/gettable on a normal CCtx
        let cctx = CtxPair::cctx();
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (gpc, gpr) = duo::<FnGetParam>("ZSTD_CCtx_getParameter");
        for (p, name) in [
            (ZSTD_c_nbWorkers, "nbWorkers"),
            (ZSTD_c_jobSize, "jobSize"),
            (ZSTD_c_overlapLog, "overlapLog"),
            (ZSTD_c_rsyncable, "rsyncable"),
        ] {
            for v in [-1, 0, 1, 2, 9, 1 << 20, i32::MAX] {
                let a = spc(cctx.c, p, v);
                let b = spr(cctx.r, p, v);
                eqv(&format!("row169 set {name}={v}"), a, b);
                let mut xc = 0;
                let mut xr = 0;
                eqv(
                    &format!("row169 get {name} status"),
                    gpc(cctx.c, p, &mut xc),
                    gpr(cctx.r, p, &mut xr),
                );
                eqv(&format!("row169 get {name} value"), xc, xr);
            }
        }
    }
}

#[allow(non_camel_case_types)]
type c_uint_ = u32;

// ------------------------------------------------------------------ row 170

static mut POOL_HITS_C: u32 = 0;

unsafe extern "C" fn pool_job(opaque: *mut c_void) {
    // increments a counter through the opaque pointer
    if !opaque.is_null() {
        let p = opaque as *mut u32;
        *p = (*p).wrapping_add(1);
    }
}

#[test]
fn row170_pool_non_mt_build() {
    unsafe {
        let (crc, crr) = duo::<unsafe extern "C" fn(usize, usize) -> *mut c_void>("POOL_create");
        let (cac, car) = duo::<unsafe extern "C" fn(usize, usize, ZSTD_customMem) -> *mut c_void>(
            "POOL_create_advanced",
        );
        let (frc, frr) = duo::<unsafe extern "C" fn(*mut c_void)>("POOL_free");
        let (jjc, jjr) = duo::<unsafe extern "C" fn(*mut c_void)>("POOL_joinJobs");
        let (rsc, rsr) = duo::<unsafe extern "C" fn(*mut c_void, usize) -> c_int>("POOL_resize");
        let (szc, szr) = duo::<unsafe extern "C" fn(*const c_void) -> usize>("POOL_sizeof");
        let (adc, adr) = duo::<
            unsafe extern "C" fn(*mut c_void, unsafe extern "C" fn(*mut c_void), *mut c_void),
        >("POOL_add");
        let (tac, tar) = duo::<
            unsafe extern "C" fn(*mut c_void, unsafe extern "C" fn(*mut c_void), *mut c_void) -> c_int,
        >("POOL_tryAdd");

        for nt in [0usize, 1, 2, 4, 64] {
            for qs in [0usize, 1, 4, 64] {
                let pc = crc(nt, qs);
                let pr = crr(nt, qs);
                eqv(&format!("POOL_create({nt},{qs}) null?"), pc.is_null(), pr.is_null());
                eqv(&format!("POOL_sizeof after create({nt},{qs})"), szc(pc), szr(pr));
                eqv(&format!("POOL_resize({nt})"), rsc(pc, nt), rsr(pr, nt));
                jjc(pc);
                jjr(pr);
                // POOL_add / POOL_tryAdd run the job synchronously in this build
                let mut ctr_c: u32 = 0;
                let mut ctr_r: u32 = 0;
                adc(pc, pool_job, &mut ctr_c as *mut u32 as *mut c_void);
                adr(pr, pool_job, &mut ctr_r as *mut u32 as *mut c_void);
                eqv(&format!("POOL_add ran job ({nt},{qs})"), ctr_c, ctr_r);
                let a = tac(pc, pool_job, &mut ctr_c as *mut u32 as *mut c_void);
                let b = tar(pr, pool_job, &mut ctr_r as *mut u32 as *mut c_void);
                eqv(&format!("POOL_tryAdd ret ({nt},{qs})"), a, b);
                eqv(&format!("POOL_tryAdd ran job ({nt},{qs})"), ctr_c, ctr_r);
                frc(pc);
                frr(pr);

                let pc = cac(nt, qs, ZSTD_customMem::default());
                let pr = car(nt, qs, ZSTD_customMem::default());
                eqv(
                    &format!("POOL_create_advanced({nt},{qs}) null?"),
                    pc.is_null(),
                    pr.is_null(),
                );
                eqv(
                    &format!("POOL_sizeof after create_advanced({nt},{qs})"),
                    szc(pc),
                    szr(pr),
                );
                frc(pc);
                frr(pr);
            }
        }
        // NULL handling
        eqv(
            "POOL_sizeof(NULL)",
            szc(std::ptr::null()),
            szr(std::ptr::null()),
        );
        frc(std::ptr::null_mut());
        frr(std::ptr::null_mut());
        jjc(std::ptr::null_mut());
        jjr(std::ptr::null_mut());
        eqv(
            "POOL_resize(NULL,4)",
            rsc(std::ptr::null_mut(), 4),
            rsr(std::ptr::null_mut(), 4),
        );
        let _ = &POOL_HITS_C;
    }
}

// ------------------------------------------------------------------ row 171

#[test]
fn row171_error_names_for_every_produced_error() {
    unsafe {
        let (nc, nr) = duo::<FnErrName>("ZSTD_getErrorName");
        let (gsc, gsr) =
            duo::<unsafe extern "C" fn(c_uint_) -> *const c_char>("ZSTD_getErrorString");
        let (gcc, gcr) = duo::<unsafe extern "C" fn(usize) -> c_uint_>("ZSTD_getErrorCode");
        // Every error the library can return is (size_t)-code for code in
        // 0..=ZSTD_error_maxCode (120); walk them all plus a margin.
        for code in 0..=140usize {
            let v = 0usize.wrapping_sub(code);
            eqv(&format!("row171 getErrorName(-{code})"), cstr(nc(v)), cstr(nr(v)));
            eqv(&format!("row171 getErrorCode(-{code})"), gcc(v), gcr(v));
            eqv(
                &format!("row171 getErrorString({code})"),
                cstr(gsc(code as u32)),
                cstr(gsr(code as u32)),
            );
        }
    }
}
