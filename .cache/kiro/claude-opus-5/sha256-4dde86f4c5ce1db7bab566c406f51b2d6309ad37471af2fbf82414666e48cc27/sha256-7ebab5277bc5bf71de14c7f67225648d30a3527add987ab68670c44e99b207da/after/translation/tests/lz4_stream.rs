//! Streaming LZ4 API: LZ4_stream_t / LZ4_streamDecode_t.
mod common;

use common::*;

const STREAM_SIZE: usize = 16416; // LZ4_STREAM_MINSIZE for LZ4_MEMORY_USAGE=14
const SD_SIZE: usize = 32; // LZ4_STREAMDECODE_MINSIZE

/// Compact byte-slice comparison: reports the first differing offset instead of
/// dumping thousands of bytes.
fn cmp_bytes(a: &[u8], b: &[u8], ctx: &str) {
    common::cmp_bytes(a, b, ctx)
}

fn rd_u32(s: &[u8], off: usize) -> u32 {
    u32::from_ne_bytes(s[off..off + 4].try_into().unwrap())
}
fn rd_u64(s: &[u8], off: usize) -> u64 {
    u64::from_ne_bytes(s[off..off + 8].try_into().unwrap())
}

/// Compare the portions of `LZ4_stream_t` that must agree.
///
/// Layout (LZ4_MEMORY_USAGE=14, x86-64):
///   0     ..16384  hashTable[4096] u32
///   16384..16392  const BYTE* dictionary
///   16392..16400  const LZ4_stream_t_internal* dictCtx
///   16400..16404  U32 currentOffset
///   16404..16408  U32 tableType
///   16408..16412  U32 dictSize
///
/// `dictionary` points into caller-owned memory and `dictCtx` into a
/// library-owned object; each library legitimately holds a different address,
/// so only null-ness is compared for those.
fn assert_stream_eq(a: &[u8], b: &[u8], ctx: &str) {
    cmp_bytes(&a[0..16384], &b[0..16384], &format!("{} hashTable", ctx));
    assert_eq!(
        rd_u64(a, 16384) == 0,
        rd_u64(b, 16384) == 0,
        "{}: dictionary nullness mismatch",
        ctx
    );
    assert_eq!(
        rd_u64(a, 16392) == 0,
        rd_u64(b, 16392) == 0,
        "{}: dictCtx nullness mismatch",
        ctx
    );
    assert_eq!(
        rd_u32(a, 16400),
        rd_u32(b, 16400),
        "{}: currentOffset mismatch",
        ctx
    );
    assert_eq!(
        rd_u32(a, 16404),
        rd_u32(b, 16404),
        "{}: tableType mismatch",
        ctx
    );
    assert_eq!(
        rd_u32(a, 16408),
        rd_u32(b, 16408),
        "{}: dictSize mismatch",
        ctx
    );
}

/// `LZ4_streamDecode_t_internal`:
///   0 ..8   const BYTE* externalDict
///   8 ..16  const BYTE* prefixEnd
///   16..24  size_t extDictSize
///   24..32  size_t prefixSize
///
/// Both pointers refer to caller-owned buffers, which differ between the two
/// libraries; compare null-ness plus the two sizes.
fn assert_sd_eq(a: &[u8], b: &[u8], ctx: &str) {
    assert_eq!(
        rd_u64(a, 0) == 0,
        rd_u64(b, 0) == 0,
        "{}: externalDict nullness mismatch",
        ctx
    );
    assert_eq!(
        rd_u64(a, 8) == 0,
        rd_u64(b, 8) == 0,
        "{}: prefixEnd nullness mismatch",
        ctx
    );
    assert_eq!(
        rd_u64(a, 16),
        rd_u64(b, 16),
        "{}: extDictSize mismatch",
        ctx
    );
    assert_eq!(rd_u64(a, 24), rd_u64(b, 24), "{}: prefixSize mismatch", ctx);
}

#[test]
fn stream_size_matches() {
    let (c_ss, r_ss) = pair!("LZ4_sizeofStreamState", fn() -> i32);
    unsafe {
        assert_eq!(c_ss(), r_ss());
        assert_eq!(c_ss() as usize, STREAM_SIZE);
    }
}

#[test]
fn reset_stream_variants() {
    let (c_init, r_init) = pair!("LZ4_initStream", fn(*mut u8, usize) -> *mut u8);
    let (c_reset, r_reset) = pair!("LZ4_resetStream", fn(*mut u8));
    let (c_rfast, r_rfast) = pair!("LZ4_resetStream_fast", fn(*mut u8));
    let (c_load, r_load) = pair!("LZ4_loadDict", fn(*mut u8, *const u8, i32) -> i32);
    unsafe {
        let mut cs = Aligned::new(STREAM_SIZE);
        let mut rs = Aligned::new(STREAM_SIZE);
        // fill with garbage first
        for i in 0..STREAM_SIZE {
            *cs.ptr().add(i) = (i % 251) as u8;
            *rs.ptr().add(i) = (i % 251) as u8;
        }
        c_reset(cs.ptr());
        r_reset(rs.ptr());
        assert_stream_eq(cs.as_slice(), rs.as_slice(), "resetStream");

        cs.zero();
        rs.zero();
        assert_eq!(
            c_init(cs.ptr(), STREAM_SIZE).is_null(),
            r_init(rs.ptr(), STREAM_SIZE).is_null()
        );
        assert_stream_eq(cs.as_slice(), rs.as_slice(), "initStream");

        // resetStream_fast on an initialized, then a dict-loaded, stream
        c_rfast(cs.ptr());
        r_rfast(rs.ptr());
        assert_stream_eq(cs.as_slice(), rs.as_slice(), "resetStream_fast/init");

        let dict = gen_textish(70000, 1);
        assert_eq!(
            c_load(cs.ptr(), dict.as_ptr(), dict.len() as i32),
            r_load(rs.ptr(), dict.as_ptr(), dict.len() as i32)
        );
        assert_stream_eq(cs.as_slice(), rs.as_slice(), "loadDict then");
        c_rfast(cs.ptr());
        r_rfast(rs.ptr());
        assert_stream_eq(cs.as_slice(), rs.as_slice(), "resetStream_fast/dict");
    }
}

#[test]
fn load_dict_variants() {
    let (c_new, r_new) = pair!("LZ4_createStream", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
    let (c_load, r_load) = pair!("LZ4_loadDict", fn(*mut u8, *const u8, i32) -> i32);
    let (c_slow, r_slow) = pair!("LZ4_loadDictSlow", fn(*mut u8, *const u8, i32) -> i32);
    let (c_int, r_int) = pair!(
        "LZ4_loadDict_internal",
        fn(*mut u8, *const u8, i32, u32) -> i32
    );
    unsafe {
        let cs = c_new();
        let rs = r_new();
        assert!(!cs.is_null() && !rs.is_null());
        let cv = std::slice::from_raw_parts(cs, STREAM_SIZE);
        let rv = std::slice::from_raw_parts(rs, STREAM_SIZE);
        assert_stream_eq(cv, rv, "createStream");

        for (gname, g) in GENS {
            for &dsz in &[
                0usize, 1, 2, 3, 7, 8, 11, 12, 13, 64, 100, 1000, 65535, 65536, 65537, 70000,
                131072,
            ] {
                let dict = g(dsz, 66 + dsz as u64);
                let p = if dsz == 0 {
                    std::ptr::null()
                } else {
                    dict.as_ptr()
                };
                assert_eq!(
                    c_load(cs, p, dsz as i32),
                    r_load(rs, p, dsz as i32),
                    "loadDict {} dsz={}",
                    gname,
                    dsz
                );
                assert_stream_eq(cv, rv, &format!("loadDict {} dsz={}", gname, dsz));

                assert_eq!(
                    c_slow(cs, p, dsz as i32),
                    r_slow(rs, p, dsz as i32),
                    "loadDictSlow {} dsz={}",
                    gname,
                    dsz
                );
                assert_stream_eq(cv, rv, &format!("loadDictSlow {} dsz={}", gname, dsz));

                for mode in 0u32..2 {
                    assert_eq!(
                        c_int(cs, p, dsz as i32, mode),
                        r_int(rs, p, dsz as i32, mode),
                        "loadDict_internal {} dsz={} mode={}",
                        gname,
                        dsz,
                        mode
                    );
                    assert_stream_eq(
                        cv,
                        rv,
                        &format!("loadDict_internal {} dsz={} mode={}", gname, dsz, mode),
                    );
                }
            }
        }
        // negative dictSize
        assert_eq!(c_load(cs, std::ptr::null(), -1), r_load(rs, std::ptr::null(), -1));
        assert_stream_eq(cv, rv, "loadDict negative");

        assert_eq!(c_free(cs), r_free(rs));
        assert_eq!(c_free(std::ptr::null_mut()), r_free(std::ptr::null_mut()));
    }
}

#[test]
fn save_dict() {
    let (c_new, r_new) = pair!("LZ4_createStream", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
    let (c_load, r_load) = pair!("LZ4_loadDict", fn(*mut u8, *const u8, i32) -> i32);
    let (c_save, r_save) = pair!("LZ4_saveDict", fn(*mut u8, *mut u8, i32) -> i32);
    unsafe {
        let cs = c_new();
        let rs = r_new();
        let cv = std::slice::from_raw_parts(cs, STREAM_SIZE);
        let rv = std::slice::from_raw_parts(rs, STREAM_SIZE);
        for &dsz in &[0usize, 1, 100, 1000, 65536, 70000] {
            let dict = gen_textish(dsz, 5 + dsz as u64);
            for &maxd in &[0i32, 1, 100, 1000, 65536, 65537, -1] {
                c_load(cs, dict.as_ptr(), dsz as i32);
                r_load(rs, dict.as_ptr(), dsz as i32);
                // LZ4_saveDict clamps a negative/huge dictSize up to 64 KB, so the
                // destination must always be able to hold 64 KB regardless of `maxd`.
                let n = 65536 + 256;
                let mut a = vec![0x3Cu8; n];
                let mut b = vec![0x3Cu8; n];
                let ra = c_save(cs, a.as_mut_ptr(), maxd);
                let rb = r_save(rs, b.as_mut_ptr(), maxd);
                assert_eq!(ra, rb, "saveDict dsz={} maxd={}", dsz, maxd);
                beq!(a, b, "saveDict bytes dsz={} maxd={}", dsz, maxd);
                assert_stream_eq(cv, rv, &format!("saveDict dsz={} maxd={}", dsz, maxd));
            }
        }
        c_free(cs);
        r_free(rs);
    }
}

/// Multi-block streaming compression with linked blocks, then streaming decode.
#[test]
fn stream_compress_decompress_linked() {
    let (c_new, r_new) = pair!("LZ4_createStream", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
    let (c_cont, r_cont) = pair!(
        "LZ4_compress_fast_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (c_dnew, r_dnew) = pair!("LZ4_createStreamDecode", fn() -> *mut u8);
    let (c_dfree, r_dfree) = pair!("LZ4_freeStreamDecode", fn(*mut u8) -> i32);
    let (c_dcont, r_dcont) = pair!(
        "LZ4_decompress_safe_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);

    let chunkings: [&[usize]; 5] = [
        &[1],
        &[7, 3, 100],
        &[4096],
        &[65536, 1, 1000],
        &[300, 30000, 5],
    ];
    unsafe {
        for (gname, g) in GENS {
            for cks in &chunkings {
                // 1-byte blocks generate a huge number of calls; keep that case short.
                let total = if cks == &&[1usize][..] { 20_000 } else { 140_000 };
                let data = g(total, 909 + gname.len() as u64);
                for &accel in &[1i32, 3, 65537] {
                    let cs = c_new();
                    let rs = r_new();
                    let cv = std::slice::from_raw_parts(cs, STREAM_SIZE);
                    let rv = std::slice::from_raw_parts(rs, STREAM_SIZE);

                    // keep whole source alive (linked blocks reference previous data)
                    let mut blocks_c: Vec<Vec<u8>> = Vec::new();
                    let mut sizes: Vec<usize> = Vec::new();
                    let mut pos = 0usize;
                    let mut i = 0usize;
                    while pos < data.len() {
                        let n = cks[i % cks.len()].min(data.len() - pos);
                        i += 1;
                        let bound = cbound(n as i32).max(16);
                        let mut a = vec![0u8; bound as usize + 16];
                        let mut b = vec![0u8; bound as usize + 16];
                        let ra = c_cont(
                            cs,
                            data[pos..].as_ptr(),
                            a.as_mut_ptr(),
                            n as i32,
                            bound,
                            accel,
                        );
                        let rb = r_cont(
                            rs,
                            data[pos..].as_ptr(),
                            b.as_mut_ptr(),
                            n as i32,
                            bound,
                            accel,
                        );
                        assert_eq!(
                            ra, rb,
                            "compress_fast_continue {} pos={} n={} accel={}",
                            gname, pos, n, accel
                        );
                        beq!(a, b, "continue bytes {} pos={} n={}", gname, pos, n);
                        // the 16 KB hash-table compare is expensive; sample it
                        if i % 64 == 0 || pos + n == data.len() {
                            assert_stream_eq(cv, rv, &format!("continue {} pos={}", gname, pos));
                        }
                        a.truncate(ra as usize);
                        blocks_c.push(a);
                        sizes.push(n);
                        pos += n;
                    }
                    assert_eq!(c_free(cs), r_free(rs));

                    // streaming decode of the produced blocks
                    let cd = c_dnew();
                    let rd = r_dnew();
                    let cdv = std::slice::from_raw_parts(cd, SD_SIZE);
                    let rdv = std::slice::from_raw_parts(rd, SD_SIZE);
                    let mut outc = vec![0xD1u8; data.len() + 64];
                    let mut outr = vec![0xD1u8; data.len() + 64];
                    let mut off = 0usize;
                    for (bi, blk) in blocks_c.iter().enumerate() {
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
                        assert_eq!(ra, rb, "decompress_safe_continue {} blk={}", gname, bi);
                        assert_eq!(ra, want);
                        assert_sd_eq(cdv, rdv, &format!("decode state {} blk={}", gname, bi));
                        off += want as usize;
                    }
                    beq!(outc, outr);
                    assert_eq!(&outc[..data.len()], &data[..]);
                    assert_eq!(c_dfree(cd), r_dfree(rd));
                }
            }
        }
        let (cf, rf) = pair!("LZ4_freeStreamDecode", fn(*mut u8) -> i32);
        assert_eq!(cf(std::ptr::null_mut()), rf(std::ptr::null_mut()));
    }
}

/// Streaming compression starting from an explicit dictionary, and with
/// `LZ4_saveDict` between blocks (the "independent buffer" pattern).
#[test]
fn stream_with_dict_and_savedict() {
    let (c_new, r_new) = pair!("LZ4_createStream", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
    let (c_load, r_load) = pair!("LZ4_loadDict", fn(*mut u8, *const u8, i32) -> i32);
    let (c_cont, r_cont) = pair!(
        "LZ4_compress_fast_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (c_save, r_save) = pair!("LZ4_saveDict", fn(*mut u8, *mut u8, i32) -> i32);
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    let (c_dnew, r_dnew) = pair!("LZ4_createStreamDecode", fn() -> *mut u8);
    let (c_dfree, r_dfree) = pair!("LZ4_freeStreamDecode", fn(*mut u8) -> i32);
    let (c_setd, r_setd) = pair!("LZ4_setStreamDecode", fn(*mut u8, *const u8, i32) -> i32);
    let (c_dcont, r_dcont) = pair!(
        "LZ4_decompress_safe_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
    );

    unsafe {
        for (gname, g) in GENS {
            let dict = g(65536, 4);
            let data = g(60_000, 5 + gname.len() as u64);
            let blocksz = 4096usize;
            let cs = c_new();
            let rs = r_new();
            let cv = std::slice::from_raw_parts(cs, STREAM_SIZE);
            let rv = std::slice::from_raw_parts(rs, STREAM_SIZE);
            assert_eq!(
                c_load(cs, dict.as_ptr(), dict.len() as i32),
                r_load(rs, dict.as_ptr(), dict.len() as i32)
            );

            // dictionary carried in separate save buffers so the "prev block"
            // is not contiguous with the next input -> exercises extDict paths
            let mut csave = vec![0u8; 65536 + 64];
            let mut rsave = vec![0u8; 65536 + 64];
            let mut blocks: Vec<Vec<u8>> = Vec::new();
            let mut pos = 0usize;
            while pos < data.len() {
                let n = blocksz.min(data.len() - pos);
                let bound = cbound(n as i32);
                let mut a = vec![0u8; bound as usize + 16];
                let mut b = vec![0u8; bound as usize + 16];
                // copy the chunk into a fresh buffer each round
                let chunk = data[pos..pos + n].to_vec();
                let ra = c_cont(cs, chunk.as_ptr(), a.as_mut_ptr(), n as i32, bound, 1);
                let rb = r_cont(rs, chunk.as_ptr(), b.as_mut_ptr(), n as i32, bound, 1);
                assert_eq!(ra, rb, "dictstream continue {} pos={}", gname, pos);
                beq!(a, b, "dictstream bytes {} pos={}", gname, pos);
                let sa = c_save(cs, csave.as_mut_ptr(), 65536);
                let sb = r_save(rs, rsave.as_mut_ptr(), 65536);
                assert_eq!(sa, sb, "dictstream saveDict {} pos={}", gname, pos);
                beq!(csave, rsave, "dictstream save buffer {} pos={}", gname, pos);
                assert_stream_eq(cv, rv, &format!("dictstream {} pos={}", gname, pos));
                a.truncate(ra as usize);
                blocks.push(a);
                pos += n;
            }
            assert_eq!(c_free(cs), r_free(rs));

            // decode with setStreamDecode + saveDict-style external dict
            let cd = c_dnew();
            let rd = r_dnew();
            let cdv = std::slice::from_raw_parts(cd, SD_SIZE);
            let rdv = std::slice::from_raw_parts(rd, SD_SIZE);
            assert_eq!(
                c_setd(cd, dict.as_ptr(), dict.len() as i32),
                r_setd(rd, dict.as_ptr(), dict.len() as i32)
            );
            assert_sd_eq(cdv, rdv, "setStreamDecode");
            let mut outc = vec![0u8; data.len() + 64];
            let mut outr = vec![0u8; data.len() + 64];
            let mut off = 0usize;
            for (bi, blk) in blocks.iter().enumerate() {
                let want = blocksz.min(data.len() - off) as i32;
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
                assert_eq!(ra, rb, "dict decode {} blk={}", gname, bi);
                assert_sd_eq(cdv, rdv, &format!("dict decode state {} blk={}", gname, bi));
                off += want as usize;
            }
            beq!(outc, outr);
            assert_eq!(&outc[..data.len()], &data[..]);
            assert_eq!(c_dfree(cd), r_dfree(rd));
        }
    }
}

#[test]
fn attach_dictionary() {
    let (c_new, r_new) = pair!("LZ4_createStream", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
    let (c_load, r_load) = pair!("LZ4_loadDict", fn(*mut u8, *const u8, i32) -> i32);
    let (c_att, r_att) = pair!("LZ4_attach_dictionary", fn(*mut u8, *const u8));
    let (c_cont, r_cont) = pair!(
        "LZ4_compress_fast_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (c_reset, r_reset) = pair!("LZ4_resetStream_fast", fn(*mut u8));
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    let (c_ud, r_ud) = pair!(
        "LZ4_decompress_safe_usingDict",
        fn(*const u8, *mut u8, i32, i32, *const u8, i32) -> i32
    );
    unsafe {
        for (gname, g) in GENS {
            for &dsz in &[0usize, 1, 100, 1000, 65536, 70000] {
                let dict = g(dsz, 31 + dsz as u64);
                let cdict = c_new();
                let rdict = r_new();
                c_load(cdict, dict.as_ptr(), dsz as i32);
                r_load(rdict, dict.as_ptr(), dsz as i32);

                let cs = c_new();
                let rs = r_new();
                for &sz in &[1usize, 13, 1000, 20000] {
                    let data = g(sz, 41 + sz as u64);
                    c_reset(cs);
                    r_reset(rs);
                    c_att(cs, cdict);
                    r_att(rs, rdict);
                    let bound = cbound(sz as i32);
                    let mut a = vec![0u8; bound as usize + 16];
                    let mut b = vec![0u8; bound as usize + 16];
                    let ra = c_cont(cs, data.as_ptr(), a.as_mut_ptr(), sz as i32, bound, 1);
                    let rb = r_cont(rs, data.as_ptr(), b.as_mut_ptr(), sz as i32, bound, 1);
                    assert_eq!(ra, rb, "attach {} dsz={} sz={}", gname, dsz, sz);
                    beq!(a, b, "attach bytes {} dsz={} sz={}", gname, dsz, sz);
                    assert_stream_eq(
                        std::slice::from_raw_parts(cs, STREAM_SIZE),
                        std::slice::from_raw_parts(rs, STREAM_SIZE),
                        &format!("attach state {} dsz={} sz={}", gname, dsz, sz),
                    );
                    // verify round trip against the dictionary
                    let mut o = vec![0u8; sz + 64];
                    let dp = if dsz == 0 {
                        std::ptr::null()
                    } else {
                        dict.as_ptr()
                    };
                    let n = c_ud(a.as_ptr(), o.as_mut_ptr(), ra, sz as i32 + 64, dp, dsz as i32);
                    assert_eq!(n, sz as i32, "attach roundtrip {} dsz={}", gname, dsz);
                    assert_eq!(&o[..sz], &data[..]);
                    let mut o2 = vec![0u8; sz + 64];
                    let n2 = r_ud(b.as_ptr(), o2.as_mut_ptr(), rb, sz as i32 + 64, dp, dsz as i32);
                    assert_eq!(n, n2);
                    beq!(o, o2);
                }
                // attach NULL detaches
                c_att(cs, std::ptr::null());
                r_att(rs, std::ptr::null());
                assert_stream_eq(
                    std::slice::from_raw_parts(cs, STREAM_SIZE),
                    std::slice::from_raw_parts(rs, STREAM_SIZE),
                    "attach NULL",
                );
                c_free(cs);
                r_free(rs);
                c_free(cdict);
                r_free(rdict);
            }
        }
    }
}

#[test]
fn compress_force_extdict() {
    let (c_new, r_new) = pair!("LZ4_createStream", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
    let (c_load, r_load) = pair!("LZ4_loadDict", fn(*mut u8, *const u8, i32) -> i32);
    let (c_fed, r_fed) = pair!(
        "LZ4_compress_forceExtDict",
        fn(*mut u8, *const u8, *mut u8, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    unsafe {
        for (gname, g) in GENS {
            for &dsz in &[0usize, 100, 1000, 65536] {
                let dict = g(dsz, 71 + dsz as u64);
                for &sz in &[1usize, 13, 1000, 20000] {
                    let data = g(sz, 81 + sz as u64);
                    let cs = c_new();
                    let rs = r_new();
                    c_load(cs, dict.as_ptr(), dsz as i32);
                    r_load(rs, dict.as_ptr(), dsz as i32);
                    let bound = cbound(sz as i32);
                    let mut a = vec![0u8; bound as usize + 16];
                    let mut b = vec![0u8; bound as usize + 16];
                    let ra = c_fed(cs, data.as_ptr(), a.as_mut_ptr(), sz as i32);
                    let rb = r_fed(rs, data.as_ptr(), b.as_mut_ptr(), sz as i32);
                    assert_eq!(ra, rb, "forceExtDict {} dsz={} sz={}", gname, dsz, sz);
                    beq!(a, b, "forceExtDict bytes {} dsz={} sz={}", gname, dsz, sz);
                    assert_stream_eq(
                        std::slice::from_raw_parts(cs, STREAM_SIZE),
                        std::slice::from_raw_parts(rs, STREAM_SIZE),
                        "forceExtDict state",
                    );
                    c_free(cs);
                    r_free(rs);
                }
            }
        }
    }
}

#[test]
fn deprecated_stream_wrappers() {
    let (c_create, r_create) = pair!("LZ4_create", fn(*mut u8) -> *mut u8);
    let (c_reset, r_reset) = pair!("LZ4_resetStreamState", fn(*mut u8, *mut u8) -> i32);
    let (c_slide, r_slide) = pair!("LZ4_slideInputBuffer", fn(*mut u8) -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
    let (c_cc, r_cc) = pair!(
        "LZ4_compress_continue",
        fn(*mut u8, *const u8, *mut u8, i32) -> i32
    );
    let (c_lc, r_lc) = pair!(
        "LZ4_compress_limitedOutput_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    unsafe {
        let mut cbuf = vec![0u8; 1 << 17];
        let mut rbuf = vec![0u8; 1 << 17];
        let cs = c_create(cbuf.as_mut_ptr());
        let rs = r_create(rbuf.as_mut_ptr());
        assert!(!cs.is_null() && !rs.is_null());
        // LZ4_create ignores the input buffer and just returns a fresh stream
        assert_stream_eq(
            std::slice::from_raw_parts(cs, STREAM_SIZE),
            std::slice::from_raw_parts(rs, STREAM_SIZE),
            "LZ4_create",
        );

        let data = gen_textish(50_000, 17);
        let blocksz = 4096usize;
        let mut pos = 0usize;
        while pos < data.len() {
            let n = blocksz.min(data.len() - pos);
            let bound = cbound(n as i32);
            let mut a = vec![0u8; bound as usize + 16];
            let mut b = vec![0u8; bound as usize + 16];
            let ra = c_cc(cs, data[pos..].as_ptr(), a.as_mut_ptr(), n as i32);
            let rb = r_cc(rs, data[pos..].as_ptr(), b.as_mut_ptr(), n as i32);
            assert_eq!(ra, rb, "compress_continue pos={}", pos);
            beq!(a, b, "compress_continue bytes pos={}", pos);
            assert_stream_eq(
                std::slice::from_raw_parts(cs, STREAM_SIZE),
                std::slice::from_raw_parts(rs, STREAM_SIZE),
                "compress_continue state",
            );
            pos += n;
        }

        // slideInputBuffer returns a pointer into the stream's own dict buffer;
        // only its null-ness / relative behaviour is comparable.
        let pa = c_slide(cs);
        let pb = r_slide(rs);
        assert_eq!(pa.is_null(), pb.is_null());
        assert_stream_eq(
            std::slice::from_raw_parts(cs, STREAM_SIZE),
            std::slice::from_raw_parts(rs, STREAM_SIZE),
            "slideInputBuffer state",
        );

        assert_eq!(
            c_reset(cs, std::ptr::null_mut()),
            r_reset(rs, std::ptr::null_mut())
        );
        assert_stream_eq(
            std::slice::from_raw_parts(cs, STREAM_SIZE),
            std::slice::from_raw_parts(rs, STREAM_SIZE),
            "resetStreamState",
        );

        let mut pos = 0usize;
        while pos < data.len() {
            let n = blocksz.min(data.len() - pos);
            let bound = cbound(n as i32);
            for &cap in &[0i32, 16, bound / 2, bound] {
                let mut a = vec![0u8; cap.max(0) as usize + 64];
                let mut b = vec![0u8; cap.max(0) as usize + 64];
                let ra = c_lc(cs, data[pos..].as_ptr(), a.as_mut_ptr(), n as i32, cap);
                let rb = r_lc(rs, data[pos..].as_ptr(), b.as_mut_ptr(), n as i32, cap);
                assert_eq!(ra, rb, "limitedOutput_continue pos={} cap={}", pos, cap);
                beq!(a, b, "limitedOutput_continue bytes pos={} cap={}", pos, cap);
            }
            pos += n;
        }

        assert_eq!(c_free(cs), r_free(rs));
    }
}

/// Ring-buffer style decoding: `LZ4_decompress_safe_continue` where the
/// destination wraps around a buffer of `LZ4_decoderRingBufferSize` bytes.
#[test]
fn ring_buffer_decode() {
    let (c_new, r_new) = pair!("LZ4_createStream", fn() -> *mut u8);
    let (c_free, r_free) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
    let (c_cont, r_cont) = pair!(
        "LZ4_compress_fast_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (c_dnew, r_dnew) = pair!("LZ4_createStreamDecode", fn() -> *mut u8);
    let (c_dfree, r_dfree) = pair!("LZ4_freeStreamDecode", fn(*mut u8) -> i32);
    let (c_dcont, r_dcont) = pair!(
        "LZ4_decompress_safe_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
    );
    let (c_rbs, _) = pair!("LZ4_decoderRingBufferSize", fn(i32) -> i32);
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);

    unsafe {
        for (gname, g) in GENS {
            for &blocksz in &[128usize, 1024, 4096] {
                let data = g(60_000, 55 + blocksz as u64);
                // compress with a plain linked stream over the contiguous source
                let cs = c_new();
                let rs = r_new();
                let mut blocks: Vec<Vec<u8>> = Vec::new();
                let mut pos = 0usize;
                while pos < data.len() {
                    let n = blocksz.min(data.len() - pos);
                    let bound = cbound(n as i32);
                    let mut a = vec![0u8; bound as usize + 16];
                    let mut b = vec![0u8; bound as usize + 16];
                    let ra = c_cont(cs, data[pos..].as_ptr(), a.as_mut_ptr(), n as i32, bound, 1);
                    let rb = r_cont(rs, data[pos..].as_ptr(), b.as_mut_ptr(), n as i32, bound, 1);
                    assert_eq!(ra, rb);
                    beq!(a, b);
                    a.truncate(ra as usize);
                    blocks.push(a);
                    pos += n;
                }
                c_free(cs);
                r_free(rs);

                let rb_size = c_rbs(blocksz as i32) as usize;
                let cd = c_dnew();
                let rd = r_dnew();
                let mut ringc = vec![0u8; rb_size];
                let mut ringr = vec![0u8; rb_size];
                let mut rpos = 0usize;
                let mut total = 0usize;
                for (bi, blk) in blocks.iter().enumerate() {
                    if rpos + blocksz > rb_size {
                        rpos = 0;
                    }
                    let want = blocksz.min(data.len() - total) as i32;
                    let ra = c_dcont(
                        cd,
                        blk.as_ptr(),
                        ringc[rpos..].as_mut_ptr(),
                        blk.len() as i32,
                        want,
                    );
                    let rb = r_dcont(
                        rd,
                        blk.as_ptr(),
                        ringr[rpos..].as_mut_ptr(),
                        blk.len() as i32,
                        want,
                    );
                    assert_eq!(ra, rb, "ring {} bs={} blk={}", gname, blocksz, bi);
                    assert_eq!(ra, want);
                    assert_eq!(
                        &ringc[rpos..rpos + want as usize],
                        &data[total..total + want as usize],
                        "ring content {} bs={} blk={}",
                        gname,
                        blocksz,
                        bi
                    );
                    beq!(ringc, ringr, "ring buffers {} bs={} blk={}", gname, blocksz, bi);
                    assert_sd_eq(
                        std::slice::from_raw_parts(cd, SD_SIZE),
                        std::slice::from_raw_parts(rd, SD_SIZE),
                        "ring decode state",
                    );
                    rpos += want as usize;
                    total += want as usize;
                }
                c_dfree(cd);
                r_dfree(rd);
            }
        }
    }
}

#[test]
fn decompress_fast_continue_and_setstreamdecode_edge() {
    let (c_dnew, r_dnew) = pair!("LZ4_createStreamDecode", fn() -> *mut u8);
    let (c_dfree, r_dfree) = pair!("LZ4_freeStreamDecode", fn(*mut u8) -> i32);
    let (c_setd, r_setd) = pair!("LZ4_setStreamDecode", fn(*mut u8, *const u8, i32) -> i32);
    let (c_fc, r_fc) = pair!(
        "LZ4_decompress_fast_continue",
        fn(*mut u8, *const u8, *mut u8, i32) -> i32
    );
    let (c_new, _) = pair!("LZ4_createStream", fn() -> *mut u8);
    let (c_freestream, _) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
    let (c_cont, _) = pair!(
        "LZ4_compress_fast_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    unsafe {
        // setStreamDecode with various dict configurations
        let cd = c_dnew();
        let rd = r_dnew();
        let dict = gen_textish(70000, 3);
        for &(p, l) in &[
            (std::ptr::null(), 0i32),
            (dict.as_ptr(), 0i32),
            (dict.as_ptr(), 1i32),
            (dict.as_ptr(), 65536i32),
            (dict.as_ptr(), 70000i32),
        ] {
            assert_eq!(c_setd(cd, p, l), r_setd(rd, p, l), "setStreamDecode l={}", l);
            assert_sd_eq(
                std::slice::from_raw_parts(cd, SD_SIZE),
                std::slice::from_raw_parts(rd, SD_SIZE),
                &format!("setStreamDecode l={}", l),
            );
        }

        // decompress_fast_continue over a linked stream
        for (gname, g) in GENS {
            let data = g(30_000, 6);
            let blocksz = 2048usize;
            let st = c_new();
            let mut blocks: Vec<Vec<u8>> = Vec::new();
            let mut pos = 0usize;
            while pos < data.len() {
                let n = blocksz.min(data.len() - pos);
                let bound = cbound(n as i32);
                let mut a = vec![0u8; bound as usize + 16];
                let k = c_cont(st, data[pos..].as_ptr(), a.as_mut_ptr(), n as i32, bound, 1);
                a.truncate(k as usize);
                blocks.push(a);
                pos += n;
            }
            c_freestream(st);

            assert_eq!(c_setd(cd, std::ptr::null(), 0), r_setd(rd, std::ptr::null(), 0));
            let mut outc = vec![0u8; data.len() + 64];
            let mut outr = vec![0u8; data.len() + 64];
            let mut off = 0usize;
            for (bi, blk) in blocks.iter().enumerate() {
                let want = blocksz.min(data.len() - off) as i32;
                let ra = c_fc(cd, blk.as_ptr(), outc[off..].as_mut_ptr(), want);
                let rb = r_fc(rd, blk.as_ptr(), outr[off..].as_mut_ptr(), want);
                assert_eq!(ra, rb, "fast_continue {} blk={}", gname, bi);
                assert_sd_eq(
                    std::slice::from_raw_parts(cd, SD_SIZE),
                    std::slice::from_raw_parts(rd, SD_SIZE),
                    "fast_continue state",
                );
                off += want as usize;
            }
            beq!(outc, outr);
            assert_eq!(&outc[..data.len()], &data[..]);
        }
        assert_eq!(c_dfree(cd), r_dfree(rd));
    }
}
