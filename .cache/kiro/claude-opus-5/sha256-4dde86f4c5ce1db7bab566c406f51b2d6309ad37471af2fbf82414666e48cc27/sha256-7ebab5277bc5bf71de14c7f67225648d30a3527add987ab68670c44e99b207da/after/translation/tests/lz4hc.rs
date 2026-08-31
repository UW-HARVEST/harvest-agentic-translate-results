//! LZ4 HC (high compression) API.
mod common;

use common::*;

/// `LZ4_STREAMHC_MINSIZE`
const HC_SIZE: usize = 262200;

/* LZ4HC_CCtx_internal layout (x86-64):
 *   0      ..131072  U32 hashTable[32768]
 *   131072 ..262144  U16 chainTable[65536]
 *   262144 ..262152  const BYTE* end
 *   262152 ..262160  const BYTE* prefixStart
 *   262160 ..262168  const BYTE* dictStart
 *   262168 ..262172  U32 dictLimit
 *   262172 ..262176  U32 lowLimit
 *   262176 ..262180  U32 nextToUpdate
 *   262180 ..262182  short compressionLevel
 *   262182           i8 favorDecSpeed
 *   262183           i8 dirty
 *   262184 ..262192  const LZ4HC_CCtx_internal* dictCtx
 */
const O_END: usize = 262144;
const O_PREFIX: usize = 262152;
const O_DICTSTART: usize = 262160;
const O_DICTLIMIT: usize = 262168;
const O_LOWLIMIT: usize = 262172;
const O_NEXTUPD: usize = 262176;
const O_CLEVEL: usize = 262180;
const O_FAVOR: usize = 262182;
const O_DIRTY: usize = 262183;
const O_DICTCTX: usize = 262184;

fn rd_u32(s: &[u8], off: usize) -> u32 {
    u32::from_ne_bytes(s[off..off + 4].try_into().unwrap())
}
fn rd_u64(s: &[u8], off: usize) -> u64 {
    u64::from_ne_bytes(s[off..off + 8].try_into().unwrap())
}
fn rd_i16(s: &[u8], off: usize) -> i16 {
    i16::from_ne_bytes(s[off..off + 2].try_into().unwrap())
}

/// Compare everything in `LZ4_streamHC_t` except the raw pointer values, which
/// legitimately differ (they address caller buffers and library-owned contexts).
/// `tables` selects whether the 256 KB hash/chain tables are compared too.
fn assert_hc_eq(a: &[u8], b: &[u8], tables: bool, ctx: &str) {
    if tables {
        cmp_bytes(&a[0..131072], &b[0..131072], &format!("{} hashTable", ctx));
        cmp_bytes(
            &a[131072..262144],
            &b[131072..262144],
            &format!("{} chainTable", ctx),
        );
    }
    for (off, name) in [
        (O_END, "end"),
        (O_PREFIX, "prefixStart"),
        (O_DICTSTART, "dictStart"),
        (O_DICTCTX, "dictCtx"),
    ] {
        assert_eq!(
            rd_u64(a, off) == 0,
            rd_u64(b, off) == 0,
            "{}: {} nullness mismatch",
            ctx,
            name
        );
    }
    for (off, name) in [
        (O_DICTLIMIT, "dictLimit"),
        (O_LOWLIMIT, "lowLimit"),
        (O_NEXTUPD, "nextToUpdate"),
    ] {
        assert_eq!(
            rd_u32(a, off),
            rd_u32(b, off),
            "{}: {} mismatch",
            ctx,
            name
        );
    }
    assert_eq!(
        rd_i16(a, O_CLEVEL),
        rd_i16(b, O_CLEVEL),
        "{}: compressionLevel mismatch",
        ctx
    );
    assert_eq!(
        a[O_FAVOR], b[O_FAVOR],
        "{}: favorDecSpeed mismatch",
        ctx
    );
    assert_eq!(a[O_DIRTY], b[O_DIRTY], "{}: dirty mismatch", ctx);
}

const LEVELS: [i32; 17] = [
    i32::MIN,
    -1,
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    100,
];

const HC_SIZES: [usize; 12] = [0, 1, 2, 5, 13, 63, 100, 1000, 4096, 20000, 65536, 100000];

#[test]
fn hc_sizeof_state() {
    let (cf, rf) = pair!("LZ4_sizeofStateHC", fn() -> i32);
    let (cg, rg) = pair!("LZ4_sizeofStreamStateHC", fn() -> i32);
    unsafe {
        assert_eq!(cf(), rf());
        assert_eq!(cf() as usize, HC_SIZE);
        assert_eq!(cg(), rg());
    }
}

#[test]
fn hc_compress_oneshot() {
    let (c_hc, r_hc) = pair!(
        "LZ4_compress_HC",
        fn(*const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    let (c_dec, r_dec) = pair!(
        "LZ4_decompress_safe",
        fn(*const u8, *mut u8, i32, i32) -> i32
    );
    unsafe {
        for (gname, g) in GENS {
            for &sz in &HC_SIZES {
                let data = g(sz, 21 + sz as u64);
                let bound = cbound(sz as i32).max(16);
                for &lvl in &LEVELS {
                    let mut a = vec![0xA5u8; bound as usize + 64];
                    let mut b = vec![0xA5u8; bound as usize + 64];
                    let ra = c_hc(data.as_ptr(), a.as_mut_ptr(), sz as i32, bound, lvl);
                    let rb = r_hc(data.as_ptr(), b.as_mut_ptr(), sz as i32, bound, lvl);
                    assert_eq!(ra, rb, "compress_HC {} sz={} lvl={}", gname, sz, lvl);
                    beq!(a, b, "compress_HC bytes {} sz={} lvl={}", gname, sz, lvl);
                    if ra > 0 {
                        let mut o = vec![0u8; sz + 64];
                        let n = c_dec(a.as_ptr(), o.as_mut_ptr(), ra, sz as i32 + 64);
                        assert_eq!(n, sz as i32, "HC roundtrip {} sz={} lvl={}", gname, sz, lvl);
                        assert_eq!(&o[..sz], &data[..]);
                        let mut o2 = vec![0u8; sz + 64];
                        let n2 = r_dec(b.as_ptr(), o2.as_mut_ptr(), rb, sz as i32 + 64);
                        assert_eq!(n, n2);
                        beq!(o, o2, "HC roundtrip bytes {} sz={}", gname, sz);
                    }
                }
            }
        }
    }
}

#[test]
fn hc_compress_tight_capacities() {
    let (c_hc, r_hc) = pair!(
        "LZ4_compress_HC",
        fn(*const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    unsafe {
        for (gname, g) in GENS {
            for &sz in &[1usize, 17, 100, 1000, 4096, 20000] {
                let data = g(sz, 43 + sz as u64);
                let bound = cbound(sz as i32).max(16);
                for &lvl in &[1i32, 3, 9, 10, 12] {
                    // find the exact compressed size first
                    let mut tmp = vec![0u8; bound as usize + 64];
                    let exact = c_hc(data.as_ptr(), tmp.as_mut_ptr(), sz as i32, bound, lvl);
                    let mut caps: Vec<i32> = vec![-1, 0, 1, 2, 3];
                    for d in 0..8 {
                        caps.push((exact - d).max(0));
                    }
                    caps.push(exact);
                    caps.push(exact + 1);
                    caps.push(exact / 2);
                    for &cap in &caps {
                        let n = cap.max(0) as usize + 128;
                        let mut a = vec![0x5Bu8; n];
                        let mut b = vec![0x5Bu8; n];
                        let ra = c_hc(data.as_ptr(), a.as_mut_ptr(), sz as i32, cap, lvl);
                        let rb = r_hc(data.as_ptr(), b.as_mut_ptr(), sz as i32, cap, lvl);
                        assert_eq!(ra, rb, "HC {} sz={} lvl={} cap={}", gname, sz, lvl, cap);
                        beq!(a, b, "HC bytes {} sz={} lvl={} cap={}", gname, sz, lvl, cap);
                    }
                }
            }
        }
        // invalid srcSize
        let data = gen_textish(4096, 5);
        for &bad in &[-1i32, i32::MIN, 0x7E000001, i32::MAX] {
            let mut a = vec![0u8; 8192];
            let mut b = vec![0u8; 8192];
            let ra = c_hc(data.as_ptr(), a.as_mut_ptr(), bad, 8192, 9);
            let rb = r_hc(data.as_ptr(), b.as_mut_ptr(), bad, 8192, 9);
            assert_eq!(ra, rb, "HC srcSize={}", bad);
        }
    }
}

#[test]
fn hc_extstate() {
    let (c_ext, r_ext) = pair!(
        "LZ4_compress_HC_extStateHC",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (c_fr, r_fr) = pair!(
        "LZ4_compress_HC_extStateHC_fastReset",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (c_init, r_init) = pair!("LZ4_initStreamHC", fn(*mut u8, usize) -> *mut u8);
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    unsafe {
        let mut cs = Aligned::new(HC_SIZE);
        let mut rs = Aligned::new(HC_SIZE);
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 13, 100, 1000, 4096, 20000, 65536] {
                let data = g(sz, 61 + sz as u64);
                let bound = cbound(sz as i32).max(16);
                for &lvl in &[-1i32, 0, 1, 2, 9, 10, 12, 13] {
                    cs.zero();
                    rs.zero();
                    let mut a = vec![0u8; bound as usize + 32];
                    let mut b = vec![0u8; bound as usize + 32];
                    let ra = c_ext(
                        cs.ptr(),
                        data.as_ptr(),
                        a.as_mut_ptr(),
                        sz as i32,
                        bound,
                        lvl,
                    );
                    let rb = r_ext(
                        rs.ptr(),
                        data.as_ptr(),
                        b.as_mut_ptr(),
                        sz as i32,
                        bound,
                        lvl,
                    );
                    assert_eq!(ra, rb, "HC extState {} sz={} lvl={}", gname, sz, lvl);
                    beq!(a, b, "HC extState bytes {} sz={} lvl={}", gname, sz, lvl);
                    assert_hc_eq(
                        cs.as_slice(),
                        rs.as_slice(),
                        true,
                        &format!("HC extState state {} sz={} lvl={}", gname, sz, lvl),
                    );

                    // fastReset needs an initialized state
                    cs.zero();
                    rs.zero();
                    assert_eq!(
                        c_init(cs.ptr(), HC_SIZE).is_null(),
                        r_init(rs.ptr(), HC_SIZE).is_null()
                    );
                    let mut a = vec![0u8; bound as usize + 32];
                    let mut b = vec![0u8; bound as usize + 32];
                    let ra = c_fr(
                        cs.ptr(),
                        data.as_ptr(),
                        a.as_mut_ptr(),
                        sz as i32,
                        bound,
                        lvl,
                    );
                    let rb = r_fr(
                        rs.ptr(),
                        data.as_ptr(),
                        b.as_mut_ptr(),
                        sz as i32,
                        bound,
                        lvl,
                    );
                    assert_eq!(ra, rb, "HC fastReset {} sz={} lvl={}", gname, sz, lvl);
                    beq!(a, b, "HC fastReset bytes {} sz={} lvl={}", gname, sz, lvl);
                    assert_hc_eq(
                        cs.as_slice(),
                        rs.as_slice(),
                        true,
                        &format!("HC fastReset state {} sz={} lvl={}", gname, sz, lvl),
                    );
                }
            }
        }
        // LZ4_initStreamHC with bad alignment / size
        let mut buf = Aligned::new(HC_SIZE + 16);
        for extra in [0usize, 1, 2, 3, 4, 7] {
            let p = buf.ptr().add(extra);
            for size in [0usize, 8, HC_SIZE - 1, HC_SIZE, HC_SIZE + 1] {
                let a = c_init(p, size);
                let b = r_init(p, size);
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "initStreamHC(off={},size={})",
                    extra,
                    size
                );
            }
        }
    }
}

#[test]
fn hc_destsize() {
    let (c_ds, r_ds) = pair!(
        "LZ4_compress_HC_destSize",
        fn(*mut u8, *const u8, *mut u8, *mut i32, i32, i32) -> i32
    );
    unsafe {
        let mut cs = Aligned::new(HC_SIZE);
        let mut rs = Aligned::new(HC_SIZE);
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 13, 100, 1000, 4096, 20000, 65536] {
                let data = g(sz, 71 + sz as u64);
                let mut targets: Vec<i32> = vec![0, 1, 2, 3, 4, 5, 8, 16, 17, 64, 100, 255];
                targets.push(sz as i32 / 4 + 1);
                targets.push(sz as i32 / 2 + 1);
                targets.push(sz as i32 + 16);
                for &t in &targets {
                    for &lvl in &[1i32, 3, 9, 10, 12] {
                        cs.zero();
                        rs.zero();
                        let cap = t.max(0) as usize + 128;
                        let mut a = vec![0x2Eu8; cap];
                        let mut b = vec![0x2Eu8; cap];
                        let mut sa = sz as i32;
                        let mut sb = sz as i32;
                        let ra = c_ds(cs.ptr(), data.as_ptr(), a.as_mut_ptr(), &mut sa, t, lvl);
                        let rb = r_ds(rs.ptr(), data.as_ptr(), b.as_mut_ptr(), &mut sb, t, lvl);
                        assert_eq!(
                            ra, rb,
                            "HC destSize ret {} sz={} t={} lvl={}",
                            gname, sz, t, lvl
                        );
                        assert_eq!(
                            sa, sb,
                            "HC destSize srcSize {} sz={} t={} lvl={}",
                            gname, sz, t, lvl
                        );
                        beq!(a, b, "HC destSize bytes {} sz={} t={} lvl={}", gname, sz, t, lvl);
                        assert_hc_eq(
                            cs.as_slice(),
                            rs.as_slice(),
                            true,
                            &format!("HC destSize state {} sz={} t={}", gname, sz, t),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn hc_reset_and_level_controls() {
    let (c_new, r_new) = pair!("LZ4_createStreamHC", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStreamHC", fn(*mut u8) -> i32);
    let (c_reset, r_reset) = pair!("LZ4_resetStreamHC", fn(*mut u8, i32));
    let (c_rfast, r_rfast) = pair!("LZ4_resetStreamHC_fast", fn(*mut u8, i32));
    let (c_setl, r_setl) = pair!("LZ4_setCompressionLevel", fn(*mut u8, i32));
    let (c_fav, r_fav) = pair!("LZ4_favorDecompressionSpeed", fn(*mut u8, i32));
    let (c_rss, r_rss) = pair!("LZ4_resetStreamStateHC", fn(*mut u8, *mut u8) -> i32);
    unsafe {
        let cs = c_new();
        let rs = r_new();
        assert!(!cs.is_null() && !rs.is_null());
        let cv = std::slice::from_raw_parts(cs, HC_SIZE);
        let rv = std::slice::from_raw_parts(rs, HC_SIZE);
        assert_hc_eq(cv, rv, true, "createStreamHC");

        for &lvl in &LEVELS {
            c_reset(cs, lvl);
            r_reset(rs, lvl);
            assert_hc_eq(cv, rv, true, &format!("resetStreamHC lvl={}", lvl));
            c_rfast(cs, lvl);
            r_rfast(rs, lvl);
            assert_hc_eq(cv, rv, true, &format!("resetStreamHC_fast lvl={}", lvl));
            c_setl(cs, lvl);
            r_setl(rs, lvl);
            assert_hc_eq(cv, rv, false, &format!("setCompressionLevel lvl={}", lvl));
            for f in [0i32, 1, 2, -1] {
                c_fav(cs, f);
                r_fav(rs, f);
                assert_hc_eq(cv, rv, false, &format!("favorDecompressionSpeed {}", f));
            }
        }

        let mut cbuf = vec![0u8; 1 << 12];
        let mut rbuf = vec![0u8; 1 << 12];
        assert_eq!(
            c_rss(cs, cbuf.as_mut_ptr()),
            r_rss(rs, rbuf.as_mut_ptr()),
            "resetStreamStateHC"
        );
        assert_hc_eq(cv, rv, true, "resetStreamStateHC");

        assert_eq!(c_free(cs), r_free(rs));
        assert_eq!(c_free(std::ptr::null_mut()), r_free(std::ptr::null_mut()));
    }
}

#[test]
fn hc_load_and_save_dict() {
    let (c_new, r_new) = pair!("LZ4_createStreamHC", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStreamHC", fn(*mut u8) -> i32);
    let (c_load, r_load) = pair!("LZ4_loadDictHC", fn(*mut u8, *const u8, i32) -> i32);
    let (c_save, r_save) = pair!("LZ4_saveDictHC", fn(*mut u8, *mut u8, i32) -> i32);
    unsafe {
        let cs = c_new();
        let rs = r_new();
        let cv = std::slice::from_raw_parts(cs, HC_SIZE);
        let rv = std::slice::from_raw_parts(rs, HC_SIZE);
        for (gname, g) in GENS {
            for &dsz in &[0usize, 1, 3, 4, 12, 100, 1000, 65535, 65536, 65537, 70000] {
                let dict = g(dsz, 81 + dsz as u64);
                let p = if dsz == 0 {
                    std::ptr::null()
                } else {
                    dict.as_ptr()
                };
                assert_eq!(
                    c_load(cs, p, dsz as i32),
                    r_load(rs, p, dsz as i32),
                    "loadDictHC {} dsz={}",
                    gname,
                    dsz
                );
                assert_hc_eq(cv, rv, true, &format!("loadDictHC {} dsz={}", gname, dsz));

                // LZ4_saveDictHC clamps to 64 KB, so always give it 64 KB of room
                for &maxd in &[0i32, 1, 100, 1000, 65536, 65537, -1] {
                    c_load(cs, p, dsz as i32);
                    r_load(rs, p, dsz as i32);
                    let mut a = vec![0x4Du8; 65536 + 256];
                    let mut b = vec![0x4Du8; 65536 + 256];
                    let ra = c_save(cs, a.as_mut_ptr(), maxd);
                    let rb = r_save(rs, b.as_mut_ptr(), maxd);
                    assert_eq!(ra, rb, "saveDictHC {} dsz={} maxd={}", gname, dsz, maxd);
                    beq!(a, b, "saveDictHC bytes {} dsz={} maxd={}", gname, dsz, maxd);
                    assert_hc_eq(cv, rv, false, "saveDictHC state");
                }
            }
        }
        c_free(cs);
        r_free(rs);
    }
}

#[test]
fn hc_stream_continue() {
    let (c_new, r_new) = pair!("LZ4_createStreamHC", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStreamHC", fn(*mut u8) -> i32);
    let (c_reset, r_reset) = pair!("LZ4_resetStreamHC", fn(*mut u8, i32));
    let (c_load, r_load) = pair!("LZ4_loadDictHC", fn(*mut u8, *const u8, i32) -> i32);
    let (c_cont, r_cont) = pair!(
        "LZ4_compress_HC_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    let (c_dnew, r_dnew) = pair!("LZ4_createStreamDecode", fn() -> *mut u8);
    let (c_dfree, r_dfree) = pair!("LZ4_freeStreamDecode", fn(*mut u8) -> i32);
    let (c_setd, _) = pair!("LZ4_setStreamDecode", fn(*mut u8, *const u8, i32) -> i32);
    let (c_dcont, r_dcont) = pair!(
        "LZ4_decompress_safe_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
    );
    let chunkings: [&[usize]; 4] = [&[13], &[4096], &[65536, 1, 1000], &[300, 30000, 5]];
    unsafe {
        for (gname, g) in GENS {
            let data = g(90_000, 91 + gname.len() as u64);
            for cks in &chunkings {
                for &lvl in &[1i32, 3, 9, 12] {
                    for &dsz in &[0usize, 1000, 65536] {
                        let dict = g(dsz, 95 + dsz as u64);
                        let cs = c_new();
                        let rs = r_new();
                        let cv = std::slice::from_raw_parts(cs, HC_SIZE);
                        let rv = std::slice::from_raw_parts(rs, HC_SIZE);
                        c_reset(cs, lvl);
                        r_reset(rs, lvl);
                        if dsz > 0 {
                            assert_eq!(
                                c_load(cs, dict.as_ptr(), dsz as i32),
                                r_load(rs, dict.as_ptr(), dsz as i32)
                            );
                        }
                        let mut blocks: Vec<Vec<u8>> = Vec::new();
                        let mut sizes: Vec<usize> = Vec::new();
                        let mut pos = 0usize;
                        let mut i = 0usize;
                        while pos < data.len() {
                            let n = cks[i % cks.len()].min(data.len() - pos);
                            let bound = cbound(n as i32).max(16);
                            let mut a = vec![0u8; bound as usize + 16];
                            let mut b = vec![0u8; bound as usize + 16];
                            let ra =
                                c_cont(cs, data[pos..].as_ptr(), a.as_mut_ptr(), n as i32, bound);
                            let rb =
                                r_cont(rs, data[pos..].as_ptr(), b.as_mut_ptr(), n as i32, bound);
                            assert_eq!(
                                ra, rb,
                                "HC continue {} lvl={} dsz={} pos={}",
                                gname, lvl, dsz, pos
                            );
                            beq!(
                                a,
                                b,
                                "HC continue bytes {} lvl={} dsz={} pos={}",
                                gname,
                                lvl,
                                dsz,
                                pos
                            );
                            if i % 16 == 0 || pos + n == data.len() {
                                assert_hc_eq(
                                    cv,
                                    rv,
                                    true,
                                    &format!("HC continue state {} lvl={} pos={}", gname, lvl, pos),
                                );
                            }
                            a.truncate(ra as usize);
                            blocks.push(a);
                            sizes.push(n);
                            pos += n;
                            i += 1;
                        }
                        assert_eq!(c_free(cs), r_free(rs));

                        // decode and check we recover the source
                        let cd = c_dnew();
                        let rd = r_dnew();
                        if dsz > 0 {
                            c_setd(cd, dict.as_ptr(), dsz as i32);
                            let (_, rsetd) =
                                pair!("LZ4_setStreamDecode", fn(*mut u8, *const u8, i32) -> i32);
                            rsetd(rd, dict.as_ptr(), dsz as i32);
                        }
                        let mut outc = vec![0u8; data.len() + 64];
                        let mut outr = vec![0u8; data.len() + 64];
                        let mut off = 0usize;
                        for (bi, blk) in blocks.iter().enumerate() {
                            let want = sizes[bi] as i32;
                            let ra = c_dcont(
                                cd,
                                blk.as_ptr(),
                                outc[off..].as_mut_ptr(),
                                blk.len() as i32,
                                want,
                            );
                            let rb = r_dcont(
                                rd,
                                blk.as_ptr(),
                                outr[off..].as_mut_ptr(),
                                blk.len() as i32,
                                want,
                            );
                            assert_eq!(ra, rb, "HC decode {} blk={}", gname, bi);
                            assert_eq!(ra, want, "HC decode length {} blk={}", gname, bi);
                            off += want as usize;
                        }
                        beq!(outc, outr, "HC decode output {}", gname);
                        assert_eq!(&outc[..data.len()], &data[..]);
                        c_dfree(cd);
                        r_dfree(rd);
                    }
                }
            }
        }
    }
}

#[test]
fn hc_continue_destsize() {
    let (c_new, r_new) = pair!("LZ4_createStreamHC", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStreamHC", fn(*mut u8) -> i32);
    let (c_reset, r_reset) = pair!("LZ4_resetStreamHC", fn(*mut u8, i32));
    let (c_load, r_load) = pair!("LZ4_loadDictHC", fn(*mut u8, *const u8, i32) -> i32);
    let (c_cds, r_cds) = pair!(
        "LZ4_compress_HC_continue_destSize",
        fn(*mut u8, *const u8, *mut u8, *mut i32, i32) -> i32
    );
    unsafe {
        for (gname, g) in GENS {
            let data = g(40_000, 101 + gname.len() as u64);
            for &lvl in &[1i32, 9, 12] {
                for &dsz in &[0usize, 1000] {
                    let dict = g(dsz, 103 + dsz as u64);
                    let cs = c_new();
                    let rs = r_new();
                    let cv = std::slice::from_raw_parts(cs, HC_SIZE);
                    let rv = std::slice::from_raw_parts(rs, HC_SIZE);
                    c_reset(cs, lvl);
                    r_reset(rs, lvl);
                    if dsz > 0 {
                        c_load(cs, dict.as_ptr(), dsz as i32);
                        r_load(rs, dict.as_ptr(), dsz as i32);
                    }
                    let mut pos = 0usize;
                    let mut round = 0usize;
                    while pos < data.len() {
                        let t = [1i32, 4, 17, 100, 1000, 4096][round % 6];
                        let avail = (data.len() - pos) as i32;
                        let cap = t.max(0) as usize + 128;
                        let mut a = vec![0x6Fu8; cap];
                        let mut b = vec![0x6Fu8; cap];
                        let mut sa = avail;
                        let mut sb = avail;
                        let ra = c_cds(cs, data[pos..].as_ptr(), a.as_mut_ptr(), &mut sa, t);
                        let rb = r_cds(rs, data[pos..].as_ptr(), b.as_mut_ptr(), &mut sb, t);
                        assert_eq!(
                            ra, rb,
                            "HC continue_destSize {} lvl={} pos={} t={}",
                            gname, lvl, pos, t
                        );
                        assert_eq!(
                            sa, sb,
                            "HC continue_destSize srcSize {} lvl={} pos={} t={}",
                            gname, lvl, pos, t
                        );
                        beq!(
                            a,
                            b,
                            "HC continue_destSize bytes {} lvl={} pos={} t={}",
                            gname,
                            lvl,
                            pos,
                            t
                        );
                        assert_hc_eq(
                            cv,
                            rv,
                            round % 8 == 0,
                            &format!("HC continue_destSize state {} pos={}", gname, pos),
                        );
                        if sa <= 0 {
                            break;
                        }
                        pos += sa as usize;
                        round += 1;
                    }
                    assert_eq!(c_free(cs), r_free(rs));
                }
            }
        }
    }
}

#[test]
fn hc_attach_dictionary() {
    let (c_new, r_new) = pair!("LZ4_createStreamHC", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStreamHC", fn(*mut u8) -> i32);
    let (c_load, r_load) = pair!("LZ4_loadDictHC", fn(*mut u8, *const u8, i32) -> i32);
    let (c_att, r_att) = pair!("LZ4_attach_HC_dictionary", fn(*mut u8, *const u8));
    let (c_reset, r_reset) = pair!("LZ4_resetStreamHC_fast", fn(*mut u8, i32));
    let (c_cont, r_cont) = pair!(
        "LZ4_compress_HC_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    let (c_ud, r_ud) = pair!(
        "LZ4_decompress_safe_usingDict",
        fn(*const u8, *mut u8, i32, i32, *const u8, i32) -> i32
    );
    unsafe {
        for (gname, g) in GENS {
            for &dsz in &[0usize, 1, 100, 1000, 65536, 70000] {
                let dict = g(dsz, 111 + dsz as u64);
                for &lvl in &[1i32, 9, 12] {
                    let cdict = c_new();
                    let rdict = r_new();
                    c_load(cdict, dict.as_ptr(), dsz as i32);
                    r_load(rdict, dict.as_ptr(), dsz as i32);
                    let cs = c_new();
                    let rs = r_new();
                    for &sz in &[1usize, 13, 1000, 20000] {
                        let data = g(sz, 113 + sz as u64);
                        c_reset(cs, lvl);
                        r_reset(rs, lvl);
                        c_att(cs, cdict);
                        r_att(rs, rdict);
                        let bound = cbound(sz as i32);
                        let mut a = vec![0u8; bound as usize + 16];
                        let mut b = vec![0u8; bound as usize + 16];
                        let ra = c_cont(cs, data.as_ptr(), a.as_mut_ptr(), sz as i32, bound);
                        let rb = r_cont(rs, data.as_ptr(), b.as_mut_ptr(), sz as i32, bound);
                        assert_eq!(
                            ra, rb,
                            "HC attach {} dsz={} sz={} lvl={}",
                            gname, dsz, sz, lvl
                        );
                        beq!(a, b, "HC attach bytes {} dsz={} sz={} lvl={}", gname, dsz, sz, lvl);
                        assert_hc_eq(
                            std::slice::from_raw_parts(cs, HC_SIZE),
                            std::slice::from_raw_parts(rs, HC_SIZE),
                            true,
                            &format!("HC attach state {} dsz={} sz={}", gname, dsz, sz),
                        );
                        // round trip
                        let dp = if dsz == 0 {
                            std::ptr::null()
                        } else {
                            dict.as_ptr()
                        };
                        let mut o = vec![0u8; sz + 64];
                        let n = c_ud(a.as_ptr(), o.as_mut_ptr(), ra, sz as i32 + 64, dp, dsz as i32);
                        assert_eq!(n, sz as i32, "HC attach roundtrip {} dsz={}", gname, dsz);
                        assert_eq!(&o[..sz], &data[..]);
                        let mut o2 = vec![0u8; sz + 64];
                        let n2 =
                            r_ud(b.as_ptr(), o2.as_mut_ptr(), rb, sz as i32 + 64, dp, dsz as i32);
                        assert_eq!(n, n2);
                        beq!(o, o2, "HC attach roundtrip bytes");
                    }
                    c_att(cs, std::ptr::null());
                    r_att(rs, std::ptr::null());
                    assert_hc_eq(
                        std::slice::from_raw_parts(cs, HC_SIZE),
                        std::slice::from_raw_parts(rs, HC_SIZE),
                        false,
                        "HC attach NULL",
                    );
                    c_free(cs);
                    r_free(rs);
                    c_free(cdict);
                    r_free(rdict);
                }
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Lz4hcMatch {
    off: i32,
    len: i32,
    back: i32,
}

/// `LZ4HC_searchExtDict` is an internal helper that is only meaningful when its
/// index arguments are consistent with the dictionary context (it walks
/// `chainTable` and only terminates thanks to the `LZ4_DISTANCE_MAX` check on
/// `ipIndex - matchIndex`). It is exercised here the way `lz4hc.c` itself calls
/// it: `gDictEndIndex` is the index just past the dictionary and `ipIndex` is
/// the current position measured in the same index space.
#[test]
fn hc_search_ext_dict() {
    let (cf, rf) = pair!(
        "LZ4HC_searchExtDict",
        fn(*const u8, u32, *const u8, *const u8, *const u8, u32, i32, i32) -> Lz4hcMatch
    );
    let (c_new, r_new) = pair!("LZ4_createStreamHC", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStreamHC", fn(*mut u8) -> i32);
    let (c_load, r_load) = pair!("LZ4_loadDictHC", fn(*mut u8, *const u8, i32) -> i32);
    unsafe {
        for (gname, g) in GENS {
            for &dsz in &[64usize, 1000, 65536] {
                let dict = g(dsz, 121 + dsz as u64);
                let cdict = c_new();
                let rdict = r_new();
                let cloaded = c_load(cdict, dict.as_ptr(), dsz as i32);
                let rloaded = r_load(rdict, dict.as_ptr(), dsz as i32);
                assert_eq!(cloaded, rloaded, "loadDictHC {} dsz={}", gname, dsz);

                // Source data that both does and does not share content with the
                // dictionary, so matches are sometimes found and sometimes not.
                let same = dict.clone();
                let other = gen_random(2000, 999);
                for (which, src) in [("dictdata", &same), ("random", &other)] {
                    let n = src.len();
                    for &start in &[0usize, 1, 7, 64, 100] {
                        if start + 32 >= n {
                            continue;
                        }
                        let ip = src[start..].as_ptr();
                        let ihigh = src[n - 5..].as_ptr(); // iend - LASTLITERALS
                        // gDictEndIndex == dictionary size means the dictionary
                        // sits immediately before the current block.
                        for &gap in &[0u32, 1, 100] {
                            let g_end = dsz as u32 + gap;
                            let ipindex = g_end + start as u32;
                            for &best in &[0i32, 3, 8] {
                                for &attempts in &[1i32, 2, 4, 64, 256] {
                                    let a =
                                        cf(ip, ipindex, ip, ihigh, cdict, g_end, best, attempts);
                                    let b =
                                        rf(ip, ipindex, ip, ihigh, rdict, g_end, best, attempts);
                                    assert_eq!(
                                        a, b,
                                        "searchExtDict {}/{} dsz={} start={} gEnd={} \
                                         best={} att={}",
                                        gname, which, dsz, start, g_end, best, attempts
                                    );
                                }
                            }
                        }
                    }
                }
                c_free(cdict);
                r_free(rdict);
            }
        }
    }
}

#[test]
fn hc_deprecated_wrappers() {
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    unsafe {
        let (c1, r1) = pair!("LZ4_compressHC", fn(*const u8, *mut u8, i32) -> i32);
        let (c2, r2) = pair!(
            "LZ4_compressHC_limitedOutput",
            fn(*const u8, *mut u8, i32, i32) -> i32
        );
        let (c3, r3) = pair!("LZ4_compressHC2", fn(*const u8, *mut u8, i32, i32) -> i32);
        let (c4, r4) = pair!(
            "LZ4_compressHC2_limitedOutput",
            fn(*const u8, *mut u8, i32, i32, i32) -> i32
        );
        let (c5, r5) = pair!(
            "LZ4_compressHC_withStateHC",
            fn(*mut u8, *const u8, *mut u8, i32) -> i32
        );
        let (c6, r6) = pair!(
            "LZ4_compressHC_limitedOutput_withStateHC",
            fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
        );
        let (c7, r7) = pair!(
            "LZ4_compressHC2_withStateHC",
            fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
        );
        let (c8, r8) = pair!(
            "LZ4_compressHC2_limitedOutput_withStateHC",
            fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
        );
        let mut cs = Aligned::new(HC_SIZE);
        let mut rs = Aligned::new(HC_SIZE);
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 13, 100, 1000, 4096, 20000] {
                let data = g(sz, 131 + sz as u64);
                let bound = cbound(sz as i32).max(16);
                macro_rules! chk {
                    ($tag:literal, $cc:expr, $rr:expr) => {{
                        let mut a = vec![0u8; bound as usize + 64];
                        let mut b = vec![0u8; bound as usize + 64];
                        let ra = $cc(&mut a, cs.ptr());
                        let rb = $rr(&mut b, rs.ptr());
                        assert_eq!(ra, rb, "{} {} sz={}", $tag, gname, sz);
                        beq!(a, b, "{} bytes {} sz={}", $tag, gname, sz);
                    }};
                }
                chk!(
                    "compressHC",
                    |a: &mut Vec<u8>, _s: *mut u8| c1(data.as_ptr(), a.as_mut_ptr(), sz as i32),
                    |b: &mut Vec<u8>, _s: *mut u8| r1(data.as_ptr(), b.as_mut_ptr(), sz as i32)
                );
                for &cap in &[0i32, 16, bound / 2, bound] {
                    let n = cap.max(0) as usize + 64;
                    let mut a = vec![0u8; n];
                    let mut b = vec![0u8; n];
                    assert_eq!(
                        c2(data.as_ptr(), a.as_mut_ptr(), sz as i32, cap),
                        r2(data.as_ptr(), b.as_mut_ptr(), sz as i32, cap),
                        "compressHC_limitedOutput {} sz={} cap={}",
                        gname,
                        sz,
                        cap
                    );
                    beq!(a, b, "compressHC_limitedOutput bytes {} sz={}", gname, sz);
                }
                for &lvl in &[-1i32, 0, 1, 9, 12, 13] {
                    let mut a = vec![0u8; bound as usize + 64];
                    let mut b = vec![0u8; bound as usize + 64];
                    assert_eq!(
                        c3(data.as_ptr(), a.as_mut_ptr(), sz as i32, lvl),
                        r3(data.as_ptr(), b.as_mut_ptr(), sz as i32, lvl),
                        "compressHC2 {} sz={} lvl={}",
                        gname,
                        sz,
                        lvl
                    );
                    beq!(a, b, "compressHC2 bytes {} sz={} lvl={}", gname, sz, lvl);

                    let mut a = vec![0u8; bound as usize + 64];
                    let mut b = vec![0u8; bound as usize + 64];
                    assert_eq!(
                        c4(data.as_ptr(), a.as_mut_ptr(), sz as i32, bound, lvl),
                        r4(data.as_ptr(), b.as_mut_ptr(), sz as i32, bound, lvl),
                        "compressHC2_limitedOutput {} sz={} lvl={}",
                        gname,
                        sz,
                        lvl
                    );
                    beq!(a, b, "compressHC2_limitedOutput bytes {} sz={}", gname, sz);

                    cs.zero();
                    rs.zero();
                    let mut a = vec![0u8; bound as usize + 64];
                    let mut b = vec![0u8; bound as usize + 64];
                    assert_eq!(
                        c7(cs.ptr(), data.as_ptr(), a.as_mut_ptr(), sz as i32, lvl),
                        r7(rs.ptr(), data.as_ptr(), b.as_mut_ptr(), sz as i32, lvl),
                        "compressHC2_withStateHC {} sz={} lvl={}",
                        gname,
                        sz,
                        lvl
                    );
                    beq!(a, b, "compressHC2_withStateHC bytes {} sz={}", gname, sz);
                    assert_hc_eq(cs.as_slice(), rs.as_slice(), true, "compressHC2_withStateHC");

                    cs.zero();
                    rs.zero();
                    let mut a = vec![0u8; bound as usize + 64];
                    let mut b = vec![0u8; bound as usize + 64];
                    assert_eq!(
                        c8(cs.ptr(), data.as_ptr(), a.as_mut_ptr(), sz as i32, bound, lvl),
                        r8(rs.ptr(), data.as_ptr(), b.as_mut_ptr(), sz as i32, bound, lvl),
                        "compressHC2_limitedOutput_withStateHC {} sz={} lvl={}",
                        gname,
                        sz,
                        lvl
                    );
                    beq!(a, b, "compressHC2_limitedOutput_withStateHC bytes");
                    assert_hc_eq(cs.as_slice(), rs.as_slice(), true, "HC2_limited_withStateHC");
                }

                cs.zero();
                rs.zero();
                let mut a = vec![0u8; bound as usize + 64];
                let mut b = vec![0u8; bound as usize + 64];
                assert_eq!(
                    c5(cs.ptr(), data.as_ptr(), a.as_mut_ptr(), sz as i32),
                    r5(rs.ptr(), data.as_ptr(), b.as_mut_ptr(), sz as i32),
                    "compressHC_withStateHC {} sz={}",
                    gname,
                    sz
                );
                beq!(a, b, "compressHC_withStateHC bytes {} sz={}", gname, sz);
                assert_hc_eq(cs.as_slice(), rs.as_slice(), true, "compressHC_withStateHC");

                cs.zero();
                rs.zero();
                let mut a = vec![0u8; bound as usize + 64];
                let mut b = vec![0u8; bound as usize + 64];
                assert_eq!(
                    c6(cs.ptr(), data.as_ptr(), a.as_mut_ptr(), sz as i32, bound),
                    r6(rs.ptr(), data.as_ptr(), b.as_mut_ptr(), sz as i32, bound),
                    "compressHC_limitedOutput_withStateHC {} sz={}",
                    gname,
                    sz
                );
                beq!(a, b, "compressHC_limitedOutput_withStateHC bytes");
                assert_hc_eq(cs.as_slice(), rs.as_slice(), true, "HC_limited_withStateHC");
            }
        }
    }
}

#[test]
fn hc_deprecated_stream_wrappers() {
    let (c_create, r_create) = pair!("LZ4_createHC", fn(*const u8) -> *mut u8);
    let (c_freehc, r_freehc) = pair!("LZ4_freeHC", fn(*mut u8) -> i32);
    let (c_slide, r_slide) = pair!("LZ4_slideInputBufferHC", fn(*mut u8) -> *mut u8);
    let (c_cc, r_cc) = pair!(
        "LZ4_compressHC_continue",
        fn(*mut u8, *const u8, *mut u8, i32) -> i32
    );
    let (c_lc, r_lc) = pair!(
        "LZ4_compressHC_limitedOutput_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
    );
    let (c_2c, r_2c) = pair!(
        "LZ4_compressHC2_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
    );
    let (c_2lc, r_2lc) = pair!(
        "LZ4_compressHC2_limitedOutput_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    unsafe {
        let inbuf = vec![0u8; 1 << 12];
        let cs = c_create(inbuf.as_ptr());
        let rs = r_create(inbuf.as_ptr());
        assert!(!cs.is_null() && !rs.is_null());
        assert_hc_eq(
            std::slice::from_raw_parts(cs, HC_SIZE),
            std::slice::from_raw_parts(rs, HC_SIZE),
            true,
            "LZ4_createHC",
        );

        let data = gen_textish(40_000, 141);
        let blocksz = 4096usize;
        let mut pos = 0usize;
        while pos < data.len() {
            let n = blocksz.min(data.len() - pos);
            let bound = cbound(n as i32);
            let mut a = vec![0u8; bound as usize + 16];
            let mut b = vec![0u8; bound as usize + 16];
            let ra = c_cc(cs, data[pos..].as_ptr(), a.as_mut_ptr(), n as i32);
            let rb = r_cc(rs, data[pos..].as_ptr(), b.as_mut_ptr(), n as i32);
            assert_eq!(ra, rb, "compressHC_continue pos={}", pos);
            beq!(a, b, "compressHC_continue bytes pos={}", pos);
            assert_hc_eq(
                std::slice::from_raw_parts(cs, HC_SIZE),
                std::slice::from_raw_parts(rs, HC_SIZE),
                true,
                "compressHC_continue state",
            );
            pos += n;
        }

        let pa = c_slide(cs);
        let pb = r_slide(rs);
        assert_eq!(pa.is_null(), pb.is_null(), "slideInputBufferHC nullness");
        assert_hc_eq(
            std::slice::from_raw_parts(cs, HC_SIZE),
            std::slice::from_raw_parts(rs, HC_SIZE),
            true,
            "slideInputBufferHC state",
        );

        let mut pos = 0usize;
        let mut round = 0usize;
        while pos < data.len() {
            let n = blocksz.min(data.len() - pos);
            let bound = cbound(n as i32);
            for &cap in &[0i32, 16, bound / 2, bound] {
                let mut a = vec![0u8; cap.max(0) as usize + 64];
                let mut b = vec![0u8; cap.max(0) as usize + 64];
                let ra = c_lc(cs, data[pos..].as_ptr(), a.as_mut_ptr(), n as i32, cap);
                let rb = r_lc(rs, data[pos..].as_ptr(), b.as_mut_ptr(), n as i32, cap);
                assert_eq!(ra, rb, "compressHC_limitedOutput_continue pos={}", pos);
                beq!(a, b, "compressHC_limitedOutput_continue bytes pos={}", pos);
            }
            for &lvl in &[-1i32, 1, 9, 12] {
                let mut a = vec![0u8; bound as usize + 64];
                let mut b = vec![0u8; bound as usize + 64];
                let ra = c_2c(cs, data[pos..].as_ptr(), a.as_mut_ptr(), n as i32, lvl);
                let rb = r_2c(rs, data[pos..].as_ptr(), b.as_mut_ptr(), n as i32, lvl);
                assert_eq!(ra, rb, "compressHC2_continue pos={} lvl={}", pos, lvl);
                beq!(a, b, "compressHC2_continue bytes pos={} lvl={}", pos, lvl);

                let mut a = vec![0u8; bound as usize + 64];
                let mut b = vec![0u8; bound as usize + 64];
                let ra = c_2lc(cs, data[pos..].as_ptr(), a.as_mut_ptr(), n as i32, bound, lvl);
                let rb = r_2lc(rs, data[pos..].as_ptr(), b.as_mut_ptr(), n as i32, bound, lvl);
                assert_eq!(
                    ra, rb,
                    "compressHC2_limitedOutput_continue pos={} lvl={}",
                    pos, lvl
                );
                beq!(a, b, "compressHC2_limitedOutput_continue bytes pos={}", pos);
            }
            assert_hc_eq(
                std::slice::from_raw_parts(cs, HC_SIZE),
                std::slice::from_raw_parts(rs, HC_SIZE),
                round % 4 == 0,
                "HC deprecated continue state",
            );
            pos += n;
            round += 1;
        }

        assert_eq!(c_freehc(cs), r_freehc(rs));
        assert_eq!(
            c_freehc(std::ptr::null_mut()),
            r_freehc(std::ptr::null_mut())
        );
    }
}
