//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//! Part 1: `lz4.c` (rows 1-41) and `lz4hc.c` (rows 42-70).
mod common;
use common::*;

const SEED: u64 = 0x4552_524F_5200_0001;

// ===================================================== lz4.c rows 1-19 =======

/// Rows 1-2, 6: oversized / negative `srcSize`, and the empty-input + zero-capacity
/// case, across every one-shot compressor.
#[test]
fn e_lz4_bad_src_size() {
    let (cc, rc) = syms::<FnCompressDefault>("LZ4_compress_default");
    let (cf, rf) = syms::<FnCompressFast>("LZ4_compress_fast");
    let src = mkdata(Shape::Random, 64, &mut Rng::new(1));
    let mut cd = vec![0u8; 4096];
    let mut rd = vec![0u8; 4096];

    // The C only reads srcSize bytes when the size check passes, so a huge
    // srcSize with a small buffer is safe: the check rejects it first.
    for bad in [
        0x7E00_0001i32, 0x7E00_0002, 0x7FFF_FFFF, -1, -2, -64, i32::MIN,
    ] {
        let a = unsafe { cc(src.as_ptr(), cd.as_mut_ptr(), bad, cd.len() as i32) };
        let b = unsafe { rc(src.as_ptr(), rd.as_mut_ptr(), bad, rd.len() as i32) };
        assert_eq!(a, b, "row1/2 compress_default(srcSize={bad}): C={a} R={b}");
        assert_eq!(a, 0, "row1/2 C should reject srcSize={bad}");
        for acc in [1i32, 0, 65537] {
            let a = unsafe { cf(src.as_ptr(), cd.as_mut_ptr(), bad, cd.len() as i32, acc) };
            let b = unsafe { rf(src.as_ptr(), rd.as_mut_ptr(), bad, rd.len() as i32, acc) };
            assert_eq!(a, b, "row1/2 compress_fast(srcSize={bad}, acc={acc})");
            assert_eq!(a, 0);
        }
    }

    // Row 6: srcSize == 0 with dstCapacity == 0 (and negative capacity)
    for cap in [0i32, -1, i32::MIN] {
        let a = unsafe { cc(src.as_ptr(), cd.as_mut_ptr(), 0, cap) };
        let b = unsafe { rc(src.as_ptr(), rd.as_mut_ptr(), 0, cap) };
        assert_eq!(a, b, "row6 compress_default(0, cap={cap}): C={a} R={b}");
        assert_eq!(a, 0, "row6 C should reject cap={cap}");
    }
    // srcSize == 0 with capacity 1 must succeed identically
    let a = unsafe { cc(src.as_ptr(), cd.as_mut_ptr(), 0, 1) };
    let b = unsafe { rc(src.as_ptr(), rd.as_mut_ptr(), 0, 1) };
    assert_eq!(a, b);
    assert_eq!(a, 1, "empty block is one byte");
    assert_eq!(cd[0], rd[0]);
}

/// Rows 3-5: `dstCapacity` too small at each of the three overflow sites
/// (literal run, match token, final last-literals copy).
#[test]
fn e_lz4_dst_too_small() {
    let (cc, rc) = syms::<FnCompressDefault>("LZ4_compress_default");
    let mut rng = Rng::new(SEED);
    for &shape in ALL_SHAPES.iter() {
        for &len in &[1usize, 12, 13, 20, 100, 1000, 5000, 70000] {
            let src = mkdata(shape, len, &mut rng);
            // sweep EVERY capacity from 0 up to the successful size, so all three
            // overflow branches are hit for at least some input.
            let full = c_compress(&src).len() as i32;
            for cap in 0..=full {
                let mut cd = vec![0xA5u8; (cap as usize).max(1)];
                let mut rd = vec![0xA5u8; (cap as usize).max(1)];
                let a = unsafe { cc(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap) };
                let b = unsafe { rc(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap) };
                assert_eq!(a, b, "rows3-5 len={len} cap={cap} {shape:?}: C={a} R={b}");
                if a > 0 {
                    assert_eq!(&cd[..a as usize], &rd[..a as usize]);
                }
            }
        }
    }
}

/// Row 7: `LZ4_compress_destSize` with `*dstCapacity < 1`.
#[test]
fn e_lz4_dest_size_zero_target() {
    type FnDestSize = unsafe extern "C" fn(*const u8, *mut u8, *mut i32, i32) -> i32;
    let (c, r) = syms::<FnDestSize>("LZ4_compress_destSize");
    let src = mkdata(Shape::Textish, 1000, &mut Rng::new(2));
    for tgt in [0i32, -1, i32::MIN] {
        let mut cd = vec![0u8; 16];
        let mut rd = vec![0u8; 16];
        let mut ci = 1000i32;
        let mut ri = 1000i32;
        let a = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), &mut ci, tgt) };
        let b = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), &mut ri, tgt) };
        assert_eq!(a, b, "row7 destSize(tgt={tgt}): C={a} R={b}");
        assert_eq!(ci, ri, "row7 *srcSizePtr (tgt={tgt})");
        assert_eq!(a, 0, "row7 C should reject tgt={tgt}");
    }
}

/// Rows 8-11: scalar rejections.
#[test]
fn e_lz4_scalar_rejections() {
    let (cb, rb) = syms::<FnCompressBound>("LZ4_compressBound");
    for n in [0x7E00_0001i32, 0x7FFF_FFFF, -1, -2, i32::MIN] {
        let a = unsafe { cb(n) };
        let b = unsafe { rb(n) };
        assert_eq!(a, b, "rows8-9 compressBound({n}): C={a} R={b}");
        assert_eq!(a, 0, "rows8-9 C should return 0 for {n}");
    }
    let (cr, rr) = syms::<FnCompressBound>("LZ4_decoderRingBufferSize");
    for n in [-1i32, -2, i32::MIN, 0x7E00_0001, 0x7FFF_FFFF] {
        let a = unsafe { cr(n) };
        let b = unsafe { rr(n) };
        assert_eq!(a, b, "rows10-11 decoderRingBufferSize({n}): C={a} R={b}");
        assert_eq!(a, 0, "rows10-11 C should return 0 for {n}");
    }
}

/// Rows 12-14: `LZ4_initStream` rejects NULL, undersized and misaligned buffers.
#[test]
fn e_lz4_init_stream_rejections() {
    type FnInitStream = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;
    let (c, r) = syms::<FnInitStream>("LZ4_initStream");
    type FnI = unsafe extern "C" fn() -> i32;
    let (cs, _) = syms::<FnI>("LZ4_sizeofStreamState");
    let sz = unsafe { cs() } as usize;

    // Row 12: NULL buffer
    for size in [0usize, 1, sz, sz + 1, usize::MAX] {
        let a = unsafe { c(std::ptr::null_mut(), size) };
        let b = unsafe { r(std::ptr::null_mut(), size) };
        assert_eq!(a.is_null(), b.is_null(), "row12 initStream(NULL,{size})");
        assert!(a.is_null(), "row12 C should reject NULL");
    }
    // Row 13: too small
    let mut buf = vec![0u64; sz / 8 + 4];
    for size in [0usize, 1, 8, sz - 1] {
        let a = unsafe { c(buf.as_mut_ptr() as *mut u8, size) };
        let b = unsafe { r(buf.as_mut_ptr() as *mut u8, size) };
        assert_eq!(a.is_null(), b.is_null(), "row13 initStream(size={size})");
        assert!(a.is_null(), "row13 C should reject size={size}");
    }
    // exactly big enough succeeds
    let a = unsafe { c(buf.as_mut_ptr() as *mut u8, sz) };
    let b = unsafe { r(buf.as_mut_ptr() as *mut u8, sz) };
    assert_eq!(a.is_null(), b.is_null());
    assert!(!a.is_null());
    // Row 14: misaligned (offset the pointer by 1..7 bytes)
    let mut big = vec![0u8; sz + 16];
    for off in 1..8usize {
        let p = unsafe { big.as_mut_ptr().add(off) };
        let a = unsafe { c(p, sz) };
        let b = unsafe { r(p, sz) };
        assert_eq!(
            a.is_null(),
            b.is_null(),
            "row14 initStream(misaligned by {off}): C null={} R null={}",
            a.is_null(),
            b.is_null()
        );
    }
}

/// Row 15: `free` on NULL.
#[test]
fn e_lz4_free_null() {
    type FnFree = unsafe extern "C" fn(*mut u8) -> i32;
    for name in ["LZ4_freeStream", "LZ4_freeStreamDecode", "LZ4_freeStreamHC", "LZ4_freeHC"] {
        let (c, r) = syms::<FnFree>(name);
        let a = unsafe { c(std::ptr::null_mut()) };
        let b = unsafe { r(std::ptr::null_mut()) };
        assert_eq!(a, b, "row15 {name}(NULL): C={a} R={b}");
        assert_eq!(a, 0, "row15 {name}(NULL) should return 0");
    }
}

/// Rows 16-19: `loadDict` / `saveDict` / `setStreamDecode` boundary returns.
#[test]
fn e_lz4_dict_boundaries() {
    type FnCreate = unsafe extern "C" fn() -> *mut u8;
    type FnFree = unsafe extern "C" fn(*mut u8) -> i32;
    type FnLoad = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
    type FnSave = unsafe extern "C" fn(*mut u8, *mut u8, i32) -> i32;
    type FnSetDec = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
    let (cc, rc) = syms::<FnCreate>("LZ4_createStream");
    let (cfr, rfr) = syms::<FnFree>("LZ4_freeStream");
    let mut rng = Rng::new(3);
    let dict = mkdata(Shape::Textish, 200_000, &mut rng);

    for name in ["LZ4_loadDict", "LZ4_loadDictSlow"] {
        let (cl, rl) = syms::<FnLoad>(name);
        for ds in [0i32, 1, 2, 3, 4, 5, 65535, 65536, 65537, 70000, 200_000] {
            let a0 = unsafe { cc() };
            let b0 = unsafe { rc() };
            let a = unsafe { cl(a0, dict.as_ptr(), ds) };
            let b = unsafe { rl(b0, dict.as_ptr(), ds) };
            assert_eq!(a, b, "rows16-17 {name}(ds={ds}): C={a} R={b}");
            if ds < 4 {
                assert_eq!(a, 0, "row16 {name} should drop dict smaller than HASH_UNIT");
            }
            if ds > 65536 {
                assert_eq!(a, 65536, "row17 {name} should clamp to 64 KB");
            }
            unsafe { cfr(a0) };
            unsafe { rfr(b0) };
        }
        // NULL dictionary with size 0
        let a0 = unsafe { cc() };
        let b0 = unsafe { rc() };
        let a = unsafe { cl(a0, std::ptr::null(), 0) };
        let b = unsafe { rl(b0, std::ptr::null(), 0) };
        assert_eq!(a, b, "{name}(NULL, 0)");
        unsafe { cfr(a0) };
        unsafe { rfr(b0) };
    }

    // Row 18: saveDict clamps
    let (cs, rs) = syms::<FnSave>("LZ4_saveDict");
    let (cl, rl) = syms::<FnLoad>("LZ4_loadDict");
    for ds in [0i32, 1, 3, 4, 65535, 65536, 65537, 70000] {
        let a0 = unsafe { cc() };
        let b0 = unsafe { rc() };
        unsafe { cl(a0, dict.as_ptr(), 200_000) };
        unsafe { rl(b0, dict.as_ptr(), 200_000) };
        let mut ca = vec![0u8; 200_016];
        let mut ra = vec![0u8; 200_016];
        let a = unsafe { cs(a0, ca.as_mut_ptr(), ds) };
        let b = unsafe { rs(b0, ra.as_mut_ptr(), ds) };
        assert_eq!(a, b, "row18 saveDict(ds={ds}): C={a} R={b}");
        assert_eq!(ca, ra, "row18 saveDict buffer (ds={ds})");
        unsafe { cfr(a0) };
        unsafe { rfr(b0) };
    }

    // Row 19: setStreamDecode always returns 1
    type FnCreateDec = unsafe extern "C" fn() -> *mut u8;
    let (ccd, rcd) = syms::<FnCreateDec>("LZ4_createStreamDecode");
    let (cfd, rfd) = syms::<FnFree>("LZ4_freeStreamDecode");
    let (csd, rsd) = syms::<FnSetDec>("LZ4_setStreamDecode");
    for ds in [0i32, 1, 100, 65536, 70000] {
        let a0 = unsafe { ccd() };
        let b0 = unsafe { rcd() };
        let a = unsafe { csd(a0, dict.as_ptr(), ds) };
        let b = unsafe { rsd(b0, dict.as_ptr(), ds) };
        assert_eq!(a, b, "row19 setStreamDecode(ds={ds}): C={a} R={b}");
        assert_eq!(a, 1);
        unsafe { cfd(a0) };
        unsafe { rfd(b0) };
    }
    // NULL dictionary
    let a0 = unsafe { ccd() };
    let b0 = unsafe { rcd() };
    let a = unsafe { csd(a0, std::ptr::null(), 0) };
    let b = unsafe { rsd(b0, std::ptr::null(), 0) };
    assert_eq!(a, b, "row19 setStreamDecode(NULL, 0)");
    unsafe { cfd(a0) };
    unsafe { rfd(b0) };
}

// =================================================== lz4.c rows 20-37 ========

/// Rows 20-23: `LZ4_decompress_safe*` explicit argument rejections, including
/// a genuine NULL `src` pointer (the C checks it) and negative capacities.
#[test]
fn e_lz4_decompress_arg_rejections() {
    let (c, r) = syms::<FnDecompressSafe>("LZ4_decompress_safe");
    let (cp, rp) = syms::<FnDecompressPartial>("LZ4_decompress_safe_partial");
    let src = mkdata(Shape::Textish, 500, &mut Rng::new(4));
    let comp = c_compress(&src);
    let mut cd = vec![0u8; 1024];
    let mut rd = vec![0u8; 1024];

    // Row 20: src == NULL (with every combination of sizes)
    for ss in [0i32, 1, 10, comp.len() as i32] {
        for ds in [0i32, 1, 500, 1024, -1] {
            let a = unsafe { c(std::ptr::null(), cd.as_mut_ptr(), ss, ds) };
            let b = unsafe { r(std::ptr::null(), rd.as_mut_ptr(), ss, ds) };
            assert_eq!(a, b, "row20 decompress_safe(NULL, ss={ss}, ds={ds}): C={a} R={b}");
            assert_eq!(a, -1, "row20 C should return -1");
            let a = unsafe { cp(std::ptr::null(), cd.as_mut_ptr(), ss, ds, ds) };
            let b = unsafe { rp(std::ptr::null(), rd.as_mut_ptr(), ss, ds, ds) };
            assert_eq!(a, b, "row20 partial(NULL, ss={ss}, ds={ds}): C={a} R={b}");
        }
    }
    // Row 21: negative dstCapacity
    for ds in [-1i32, -2, -1000, i32::MIN] {
        let a = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), comp.len() as i32, ds) };
        let b = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), comp.len() as i32, ds) };
        assert_eq!(a, b, "row21 decompress_safe(dstCapacity={ds}): C={a} R={b}");
        assert_eq!(a, -1, "row21 C should return -1");
    }
    // Row 22: srcSize == 0 (and negative)
    for ss in [0i32, -1, -100, i32::MIN] {
        let a = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), ss, 1024) };
        let b = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), ss, 1024) };
        assert_eq!(a, b, "row22 decompress_safe(srcSize={ss}): C={a} R={b}");
    }
    // Row 23: dstCapacity == 0 on a non-empty block
    let a = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), comp.len() as i32, 0) };
    let b = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), comp.len() as i32, 0) };
    assert_eq!(a, b, "row23 decompress_safe(dstCapacity=0): C={a} R={b}");
    assert!(a < 0, "row23 C should fail");
    // ... but the exact empty block with capacity 0 is accepted
    let empty = c_compress(&[]);
    let a = unsafe { c(empty.as_ptr(), cd.as_mut_ptr(), empty.len() as i32, 0) };
    let b = unsafe { r(empty.as_ptr(), rd.as_mut_ptr(), empty.len() as i32, 0) };
    assert_eq!(a, b, "row23 empty block with capacity 0");
}

/// Rows 24-32: malformed-stream rejections. Each hand-built block targets one
/// specific branch of `LZ4_decompress_generic`, and the exact negative return
/// (which encodes the failure offset) must match.
#[test]
fn e_lz4_malformed_streams() {
    let (c, r) = syms::<FnDecompressSafe>("LZ4_decompress_safe");
    let (cp, rp) = syms::<FnDecompressPartial>("LZ4_decompress_safe_partial");

    // Hand-crafted blocks, one per malformed-input class.
    let cases: Vec<(&str, Vec<u8>)> = vec![
        // row 27: offset == 0  (token: 0 literals, minmatch; offset 0x0000)
        ("offset==0", vec![0x00, 0x00, 0x00]),
        ("offset==0 with literals", vec![0x40, b'a', b'b', b'c', b'd', 0x00, 0x00]),
        // row 28: offset larger than what has been produced
        ("offset too large", vec![0x40, b'a', b'b', b'c', b'd', 0xFF, 0xFF]),
        ("offset just past output", vec![0x40, b'a', b'b', b'c', b'd', 0x05, 0x00]),
        // row 24: literal-length varint runs off the end
        ("literal len truncated", vec![0xF0]),
        ("literal len truncated 2", vec![0xF0, 0xFF]),
        ("literal len truncated 3", vec![0xF0, 0xFF, 0xFF, 0xFF]),
        // row 25: literal length overflowing 32 bits
        (
            "literal len overflow",
            {
                let mut v = vec![0xF0u8];
                v.extend(std::iter::repeat_n(0xFFu8, 40));
                v.push(0x01);
                v
            },
        ),
        // row 26: match-length varint truncated / overflowing
        ("match len truncated", vec![0x0F, 0x04, 0x00]),
        (
            "match len overflow",
            {
                let mut v = vec![0x1Fu8, b'x', 0x01, 0x00];
                v.extend(std::iter::repeat_n(0xFFu8, 40));
                v.push(0x01);
                v
            },
        ),
        // row 30: last-literals length does not match the remaining input
        ("last literals short", vec![0x50, b'a', b'b', b'c']),
        ("last literals long", vec![0x20, b'a', b'b', b'c', b'd']),
        // row 31/32: match runs past the output tail
        ("match past tail", vec![0x5F, b'a', b'b', b'c', b'd', b'e', 0x01, 0x00]),
        ("full-block match overrun", vec![0x4F, b'a', b'b', b'c', b'd', 0x02, 0x00, 0xFF, 0x00]),
        // token with impossible literal count relative to input
        ("token only", vec![0x10]),
        ("token only 2", vec![0xFF]),
        ("single zero byte", vec![0x00]),
    ];

    for (name, blk) in cases.iter() {
        for cap in [0i32, 1, 4, 16, 64, 1000, 65536] {
            let mut cd = vec![0x5Au8; cap.max(1) as usize];
            let mut rd = vec![0x5Au8; cap.max(1) as usize];
            let a = unsafe { c(blk.as_ptr(), cd.as_mut_ptr(), blk.len() as i32, cap) };
            let b = unsafe { r(blk.as_ptr(), rd.as_mut_ptr(), blk.len() as i32, cap) };
            assert_eq!(
                a, b,
                "rows24-32 [{name}] cap={cap} srcLen={}: C={a} R={b}\nblock={}",
                blk.len(),
                hexish(blk)
            );
            assert_eq!(&cd, &rd, "rows24-32 [{name}] cap={cap}: dst buffer differs");
            // Row 33: the same input through the partial decoder
            for tgt in [0i32, 1, cap / 2, cap] {
                let mut cd = vec![0x5Au8; cap.max(1) as usize];
                let mut rd = vec![0x5Au8; cap.max(1) as usize];
                let a = unsafe { cp(blk.as_ptr(), cd.as_mut_ptr(), blk.len() as i32, tgt, cap) };
                let b = unsafe { rp(blk.as_ptr(), rd.as_mut_ptr(), blk.len() as i32, tgt, cap) };
                assert_eq!(
                    a, b,
                    "row33 partial [{name}] tgt={tgt} cap={cap}: C={a} R={b}"
                );
                assert_eq!(&cd, &rd, "row33 partial [{name}] dst buffer differs");
            }
        }
    }
}

/// Rows 24-32 (broad): exhaustive small-input sweep. Every 1-, 2- and 3-byte
/// input, and a large randomized sweep of 4-6 byte inputs, so no malformed
/// branch escapes.
#[test]
fn e_lz4_exhaustive_tiny_inputs() {
    let (c, r) = syms::<FnDecompressSafe>("LZ4_decompress_safe");
    let mut rng = Rng::new(SEED ^ 0xF);

    // all 1-byte and 2-byte inputs
    for a0 in 0u16..256 {
        let blk = [a0 as u8];
        for cap in [0i32, 1, 8, 64] {
            let mut cd = vec![0x33u8; cap.max(1) as usize];
            let mut rd = vec![0x33u8; cap.max(1) as usize];
            let x = unsafe { c(blk.as_ptr(), cd.as_mut_ptr(), 1, cap) };
            let y = unsafe { r(blk.as_ptr(), rd.as_mut_ptr(), 1, cap) };
            assert_eq!(x, y, "1-byte {a0:#04x} cap={cap}: C={x} R={y}");
            assert_eq!(cd, rd);
        }
    }
    for a0 in 0u16..256 {
        for b0 in 0u16..256 {
            let blk = [a0 as u8, b0 as u8];
            for cap in [0i32, 1, 8, 64] {
                let mut cd = vec![0x33u8; cap.max(1) as usize];
                let mut rd = vec![0x33u8; cap.max(1) as usize];
                let x = unsafe { c(blk.as_ptr(), cd.as_mut_ptr(), 2, cap) };
                let y = unsafe { r(blk.as_ptr(), rd.as_mut_ptr(), 2, cap) };
                assert_eq!(x, y, "2-byte {a0:#04x}{b0:#04x} cap={cap}: C={x} R={y}");
                assert_eq!(cd, rd);
            }
        }
    }
    // randomized 3..24 byte inputs
    for round in 0..200_000 {
        let n = rng.range(3, 24);
        let blk = mkdata(Shape::Random, n, &mut rng);
        let cap = [0i32, 1, 4, 17, 64, 300][round % 6];
        let mut cd = vec![0x33u8; cap.max(1) as usize];
        let mut rd = vec![0x33u8; cap.max(1) as usize];
        let x = unsafe { c(blk.as_ptr(), cd.as_mut_ptr(), n as i32, cap) };
        let y = unsafe { r(blk.as_ptr(), rd.as_mut_ptr(), n as i32, cap) };
        assert_eq!(
            x, y,
            "tiny fuzz round={round} n={n} cap={cap}: C={x} R={y} block={}",
            hexish(&blk)
        );
        assert_eq!(cd, rd, "tiny fuzz round={round}: dst differs");
    }
}

/// Rows 34-37: `LZ4_decompress_fast` and the legacy `LZ4_uncompress` on
/// malformed input. These use the unsafe generic and return `-1`.
#[test]
fn e_lz4_fast_decompress_failures() {
    let (c, r) = syms::<FnDecompressFast>("LZ4_decompress_fast");
    let (cu, ru) = syms::<FnDecompressFast>("LZ4_uncompress");
    let mut rng = Rng::new(SEED ^ 0x11);
    // Give a huge output slack so the unsafe decoder cannot escape our
    // allocation before it trips its own output-overflow defence.
    for round in 0..20000 {
        let n = rng.range(1, 24);
        let blk = mkdata(Shape::Random, n, &mut rng);
        let os = [1i32, 4, 16, 64, 1000][round % 5];
        let mut cd = vec![0x44u8; os as usize + 65536];
        let mut rd = vec![0x44u8; os as usize + 65536];
        let x = unsafe { c(blk.as_ptr(), cd.as_mut_ptr(), os) };
        let y = unsafe { r(blk.as_ptr(), rd.as_mut_ptr(), os) };
        assert_eq!(
            x, y,
            "rows34-37 decompress_fast round={round} n={n} os={os}: C={x} R={y} block={}",
            hexish(&blk)
        );
        if x >= 0 {
            assert_eq!(&cd[..os as usize], &rd[..os as usize]);
        }
        let mut cd = vec![0x44u8; os as usize + 65536];
        let mut rd = vec![0x44u8; os as usize + 65536];
        let x = unsafe { cu(blk.as_ptr(), cd.as_mut_ptr(), os) };
        let y = unsafe { ru(blk.as_ptr(), rd.as_mut_ptr(), os) };
        assert_eq!(x, y, "rows34-37 LZ4_uncompress round={round} os={os}");
    }
}

/// Rows 38-39: `LZ4_compress_fast_continue` rejections.
#[test]
fn e_lz4_continue_rejections() {
    type FnCreate = unsafe extern "C" fn() -> *mut u8;
    type FnFree = unsafe extern "C" fn(*mut u8) -> i32;
    type FnCont = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
    let (cc, rc) = syms::<FnCreate>("LZ4_createStream");
    let (cfr, rfr) = syms::<FnFree>("LZ4_freeStream");
    let (ct, rt) = syms::<FnCont>("LZ4_compress_fast_continue");
    let src = mkdata(Shape::Textish, 4096, &mut Rng::new(5));

    // Row 39: bad srcSize
    for bad in [0x7E00_0001i32, 0x7FFF_FFFF, -1, i32::MIN] {
        let a0 = unsafe { cc() };
        let b0 = unsafe { rc() };
        let mut cd = vec![0u8; 8192];
        let mut rd = vec![0u8; 8192];
        let a = unsafe { ct(a0, src.as_ptr(), cd.as_mut_ptr(), bad, 8192, 1) };
        let b = unsafe { rt(b0, src.as_ptr(), rd.as_mut_ptr(), bad, 8192, 1) };
        assert_eq!(a, b, "row39 continue(srcSize={bad}): C={a} R={b}");
        assert_eq!(a, 0);
        unsafe { cfr(a0) };
        unsafe { rfr(b0) };
    }
    // Row 38: insufficient dstCapacity, sweeping every capacity
    let a0 = unsafe { cc() };
    let b0 = unsafe { rc() };
    let full = c_compress(&src).len() as i32;
    for cap in 0..=full {
        let mut cd = vec![0u8; cap.max(1) as usize];
        let mut rd = vec![0u8; cap.max(1) as usize];
        let a = unsafe { ct(a0, src.as_ptr(), cd.as_mut_ptr(), 4096, cap, 1) };
        let b = unsafe { rt(b0, src.as_ptr(), rd.as_mut_ptr(), 4096, cap, 1) };
        assert_eq!(a, b, "row38 continue(cap={cap}): C={a} R={b}");
        if a > 0 {
            assert_eq!(&cd[..a as usize], &rd[..a as usize]);
        }
    }
    unsafe { cfr(a0) };
    unsafe { rfr(b0) };
}

/// Row 40: `LZ4_attach_dictionary` with a NULL / empty dictionary stream.
#[test]
fn e_lz4_attach_null() {
    type FnCreate = unsafe extern "C" fn() -> *mut u8;
    type FnFree = unsafe extern "C" fn(*mut u8) -> i32;
    type FnAttach = unsafe extern "C" fn(*mut u8, *const u8);
    type FnCont = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
    let (cc, rc) = syms::<FnCreate>("LZ4_createStream");
    let (cfr, rfr) = syms::<FnFree>("LZ4_freeStream");
    let (ca, ra) = syms::<FnAttach>("LZ4_attach_dictionary");
    let (ct, rt) = syms::<FnCont>("LZ4_compress_fast_continue");
    let src = mkdata(Shape::Textish, 4096, &mut Rng::new(6));

    for use_empty_dict in [false, true] {
        let a0 = unsafe { cc() };
        let b0 = unsafe { rc() };
        if use_empty_dict {
            // an empty (never-loaded) dictionary stream
            let ad = unsafe { cc() };
            let bd = unsafe { rc() };
            unsafe { ca(a0, ad) };
            unsafe { ra(b0, bd) };
            unsafe { cfr(ad) };
            unsafe { rfr(bd) };
        } else {
            unsafe { ca(a0, std::ptr::null()) };
            unsafe { ra(b0, std::ptr::null()) };
        }
        let mut cd = vec![0u8; 8192];
        let mut rd = vec![0u8; 8192];
        let a = unsafe { ct(a0, src.as_ptr(), cd.as_mut_ptr(), 4096, 8192, 1) };
        let b = unsafe { rt(b0, src.as_ptr(), rd.as_mut_ptr(), 4096, 8192, 1) };
        assert_eq!(a, b, "row40 attach(empty={use_empty_dict})");
        assert_eq!(&cd[..a as usize], &rd[..a as usize]);
        unsafe { cfr(a0) };
        unsafe { rfr(b0) };
    }
}

/// Row 41: `LZ4_uncompress_unknownOutputSize` on malformed input.
#[test]
fn e_lz4_uncompress_unknown_failures() {
    type Fn4 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
    let (c, r) = syms::<Fn4>("LZ4_uncompress_unknownOutputSize");
    let mut rng = Rng::new(SEED ^ 0x21);
    for round in 0..40000 {
        let n = rng.range(0, 24);
        let blk = mkdata(Shape::Random, n, &mut rng);
        let cap = [0i32, 1, 4, 17, 64, 300][round % 6];
        let mut cd = vec![0x66u8; cap.max(1) as usize];
        let mut rd = vec![0x66u8; cap.max(1) as usize];
        let x = unsafe { c(blk.as_ptr(), cd.as_mut_ptr(), n as i32, cap) };
        let y = unsafe { r(blk.as_ptr(), rd.as_mut_ptr(), n as i32, cap) };
        assert_eq!(
            x, y,
            "row41 round={round} n={n} cap={cap}: C={x} R={y} block={}",
            hexish(&blk)
        );
        assert_eq!(cd, rd);
    }
}

// ================================================== lz4hc.c rows 42-70 =======

/// Rows 42, 48-50, 54: HC `srcSize` rejections at every level (the level-2
/// `lz4mid` path has its own explicit checks).
#[test]
fn e_hc_bad_src_size() {
    let (c, r) = syms::<FnCompressHC>("LZ4_compress_HC");
    let src = mkdata(Shape::Random, 64, &mut Rng::new(7));
    let mut cd = vec![0u8; 4096];
    let mut rd = vec![0u8; 4096];
    for lvl in [i32::MIN, -1, 0, 1, 2, 3, 9, 10, 12, 13, i32::MAX] {
        for bad in [0x7E00_0001i32, 0x7FFF_FFFF, -1, -64, i32::MIN] {
            let a = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), bad, 4096, lvl) };
            let b = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), bad, 4096, lvl) };
            assert_eq!(a, b, "row42 compress_HC(srcSize={bad}, lvl={lvl}): C={a} R={b}");
            assert_eq!(a, 0, "row42 C should reject srcSize={bad}");
        }
        // negative maxOutputSize (row 49, checked explicitly on the lz4mid path)
        for cap in [-1i32, -1000, i32::MIN] {
            let a = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), 64, cap, lvl) };
            let b = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), 64, cap, lvl) };
            assert_eq!(a, b, "row49 compress_HC(cap={cap}, lvl={lvl}): C={a} R={b}");
        }
    }
}

/// Rows 43-45: HC `dstCapacity` overflow at every encode site, every level.
#[test]
fn e_hc_dst_too_small() {
    let (c, r) = syms::<FnCompressHC>("LZ4_compress_HC");
    let mut rng = Rng::new(SEED ^ 0x31);
    for lvl in [1i32, 2, 3, 9, 10, 12] {
        for &shape in ALL_SHAPES.iter() {
            for &len in &[1usize, 13, 100, 1000, 5000] {
                let src = mkdata(shape, len, &mut rng);
                let full = c_compress_hc(&src, lvl).len() as i32;
                for cap in 0..=full {
                    let mut cd = vec![0xB7u8; cap.max(1) as usize];
                    let mut rd = vec![0xB7u8; cap.max(1) as usize];
                    let a = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap, lvl) };
                    let b = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap, lvl) };
                    assert_eq!(
                        a, b,
                        "rows43-45 lvl={lvl} len={len} cap={cap} {shape:?}: C={a} R={b}"
                    );
                    if a > 0 {
                        assert_eq!(&cd[..a as usize], &rd[..a as usize]);
                    }
                }
            }
        }
    }
}

/// Rows 46-47: `LZ4_compress_HC_extStateHC*` with a misaligned / undersized state.
#[test]
fn e_hc_bad_state() {
    type FnExt = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
    type FnI = unsafe extern "C" fn() -> i32;
    let (cs, _) = syms::<FnI>("LZ4_sizeofStateHC");
    let sz = unsafe { cs() } as usize;
    let src = mkdata(Shape::Textish, 1000, &mut Rng::new(8));
    let mut big = vec![0u8; sz + 64];

    for name in ["LZ4_compress_HC_extStateHC", "LZ4_compress_HC_extStateHC_fastReset"] {
        let (c, r) = syms::<FnExt>(name);
        for off in 1..8usize {
            let p = unsafe { big.as_mut_ptr().add(off) };
            let mut cd = vec![0u8; 4096];
            let mut rd = vec![0u8; 4096];
            for lvl in [2i32, 9, 12] {
                let a = unsafe { c(p, src.as_ptr(), cd.as_mut_ptr(), 1000, 4096, lvl) };
                let b = unsafe { r(p, src.as_ptr(), rd.as_mut_ptr(), 1000, 4096, lvl) };
                assert_eq!(
                    a, b,
                    "rows46-47 {name}(misaligned by {off}, lvl={lvl}): C={a} R={b}"
                );
                if a > 0 {
                    assert_eq!(&cd[..a as usize], &rd[..a as usize]);
                }
            }
        }
    }
}

/// Rows 51-52, 55: `LZ4_compress_HC_destSize` / `_continue_destSize` rejections.
#[test]
fn e_hc_dest_size_rejections() {
    type FnDS = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, *mut i32, i32, i32) -> i32;
    type FnI = unsafe extern "C" fn() -> i32;
    let (c, r) = syms::<FnDS>("LZ4_compress_HC_destSize");
    let (cs, _) = syms::<FnI>("LZ4_sizeofStateHC");
    let sz = unsafe { cs() } as usize;
    let src = mkdata(Shape::Textish, 1000, &mut Rng::new(9));

    for lvl in [i32::MIN, 0, 1, 2, 3, 9, 10, 12, 13, i32::MAX] {
        for tgt in [0i32, -1, i32::MIN] {
            let mut cst = vec![0u64; sz / 8 + 4];
            let mut rst = vec![0u64; sz / 8 + 4];
            let mut cd = vec![0u8; 16];
            let mut rd = vec![0u8; 16];
            let mut ci = 1000i32;
            let mut ri = 1000i32;
            let a = unsafe {
                c(cst.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), &mut ci, tgt, lvl)
            };
            let b = unsafe {
                r(rst.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), &mut ri, tgt, lvl)
            };
            assert_eq!(a, b, "row51 HC_destSize(tgt={tgt}, lvl={lvl}): C={a} R={b}");
            assert_eq!(ci, ri, "row51 *srcSizePtr (tgt={tgt}, lvl={lvl})");
            assert_eq!(a, 0, "row51 C should reject tgt={tgt}");
        }
    }

    // Row 55: streaming destSize with target 0
    type FnCreateHC = unsafe extern "C" fn() -> *mut u8;
    type FnFreeHC = unsafe extern "C" fn(*mut u8) -> i32;
    type FnReset = unsafe extern "C" fn(*mut u8, i32);
    type FnContDS = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, *mut i32, i32) -> i32;
    let (cch, rch) = syms::<FnCreateHC>("LZ4_createStreamHC");
    let (cfh, rfh) = syms::<FnFreeHC>("LZ4_freeStreamHC");
    let (crs, rrs) = syms::<FnReset>("LZ4_resetStreamHC");
    let (cds, rds) = syms::<FnContDS>("LZ4_compress_HC_continue_destSize");
    for lvl in [2i32, 9, 12] {
        for tgt in [0i32, -1, i32::MIN] {
            let a0 = unsafe { cch() };
            let b0 = unsafe { rch() };
            unsafe { crs(a0, lvl) };
            unsafe { rrs(b0, lvl) };
            let mut cd = vec![0u8; 16];
            let mut rd = vec![0u8; 16];
            let mut ci = 1000i32;
            let mut ri = 1000i32;
            let a = unsafe { cds(a0, src.as_ptr(), cd.as_mut_ptr(), &mut ci, tgt) };
            let b = unsafe { rds(b0, src.as_ptr(), rd.as_mut_ptr(), &mut ri, tgt) };
            assert_eq!(a, b, "row55 continue_destSize(tgt={tgt}, lvl={lvl}): C={a} R={b}");
            assert_eq!(ci, ri, "row55 *srcSizePtr");
            unsafe { cfh(a0) };
            unsafe { rfh(b0) };
        }
    }
}

/// Rows 53-54: `LZ4_compress_HC_continue` rejections.
#[test]
fn e_hc_continue_rejections() {
    type FnCreateHC = unsafe extern "C" fn() -> *mut u8;
    type FnFreeHC = unsafe extern "C" fn(*mut u8) -> i32;
    type FnReset = unsafe extern "C" fn(*mut u8, i32);
    type FnCont = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
    let (cch, rch) = syms::<FnCreateHC>("LZ4_createStreamHC");
    let (cfh, rfh) = syms::<FnFreeHC>("LZ4_freeStreamHC");
    let (crs, rrs) = syms::<FnReset>("LZ4_resetStreamHC");
    let (cc, rc) = syms::<FnCont>("LZ4_compress_HC_continue");
    let src = mkdata(Shape::Textish, 4096, &mut Rng::new(10));

    for lvl in [2i32, 9, 12] {
        // Row 54: bad srcSize
        for bad in [0x7E00_0001i32, 0x7FFF_FFFF, -1, i32::MIN] {
            let a0 = unsafe { cch() };
            let b0 = unsafe { rch() };
            unsafe { crs(a0, lvl) };
            unsafe { rrs(b0, lvl) };
            let mut cd = vec![0u8; 8192];
            let mut rd = vec![0u8; 8192];
            let a = unsafe { cc(a0, src.as_ptr(), cd.as_mut_ptr(), bad, 8192) };
            let b = unsafe { rc(b0, src.as_ptr(), rd.as_mut_ptr(), bad, 8192) };
            assert_eq!(a, b, "row54 HC_continue(srcSize={bad}, lvl={lvl}): C={a} R={b}");
            assert_eq!(a, 0);
            unsafe { cfh(a0) };
            unsafe { rfh(b0) };
        }
        // Row 53: insufficient dstCapacity (fresh stream each time, since a
        // failed _continue leaves the state undefined per lz4hc.h:141)
        let full = c_compress_hc(&src, lvl).len() as i32;
        for cap in 0..=full {
            let a0 = unsafe { cch() };
            let b0 = unsafe { rch() };
            unsafe { crs(a0, lvl) };
            unsafe { rrs(b0, lvl) };
            let mut cd = vec![0u8; cap.max(1) as usize];
            let mut rd = vec![0u8; cap.max(1) as usize];
            let a = unsafe { cc(a0, src.as_ptr(), cd.as_mut_ptr(), 4096, cap) };
            let b = unsafe { rc(b0, src.as_ptr(), rd.as_mut_ptr(), 4096, cap) };
            assert_eq!(a, b, "row53 HC_continue(cap={cap}, lvl={lvl}): C={a} R={b}");
            if a > 0 {
                assert_eq!(&cd[..a as usize], &rd[..a as usize]);
            }
            unsafe { cfh(a0) };
            unsafe { rfh(b0) };
        }
    }
}

/// Rows 56-58: `LZ4_initStreamHC` rejects NULL, undersized, misaligned.
#[test]
fn e_hc_init_stream_rejections() {
    type FnInit = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;
    type FnI = unsafe extern "C" fn() -> i32;
    let (c, r) = syms::<FnInit>("LZ4_initStreamHC");
    let (cs, rs) = syms::<FnI>("LZ4_sizeofStateHC");
    assert_eq!(unsafe { cs() }, unsafe { rs() });
    let sz = unsafe { cs() } as usize;

    for size in [0usize, 1, sz - 1, sz, sz + 1, usize::MAX] {
        let a = unsafe { c(std::ptr::null_mut(), size) };
        let b = unsafe { r(std::ptr::null_mut(), size) };
        assert_eq!(a.is_null(), b.is_null(), "row56 initStreamHC(NULL,{size})");
        assert!(a.is_null(), "row56 C should reject NULL");
    }
    let mut buf = vec![0u64; sz / 8 + 8];
    for size in [0usize, 1, 8, sz - 1] {
        let a = unsafe { c(buf.as_mut_ptr() as *mut u8, size) };
        let b = unsafe { r(buf.as_mut_ptr() as *mut u8, size) };
        assert_eq!(a.is_null(), b.is_null(), "row57 initStreamHC(size={size})");
        assert!(a.is_null(), "row57 C should reject size={size}");
    }
    let a = unsafe { c(buf.as_mut_ptr() as *mut u8, sz) };
    let b = unsafe { r(buf.as_mut_ptr() as *mut u8, sz) };
    assert_eq!(a.is_null(), b.is_null());
    assert!(!a.is_null());
    let mut big = vec![0u8; sz + 16];
    for off in 1..8usize {
        let p = unsafe { big.as_mut_ptr().add(off) };
        let a = unsafe { c(p, sz) };
        let b = unsafe { r(p, sz) };
        assert_eq!(a.is_null(), b.is_null(), "row58 initStreamHC(misaligned {off})");
    }
}

/// Rows 60-64: `LZ4_loadDictHC` / `LZ4_saveDictHC` boundary returns.
#[test]
fn e_hc_dict_boundaries() {
    type FnCreateHC = unsafe extern "C" fn() -> *mut u8;
    type FnFreeHC = unsafe extern "C" fn(*mut u8) -> i32;
    type FnReset = unsafe extern "C" fn(*mut u8, i32);
    type FnLoad = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
    type FnSave = unsafe extern "C" fn(*mut u8, *mut u8, i32) -> i32;
    type FnCont = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
    let (cch, rch) = syms::<FnCreateHC>("LZ4_createStreamHC");
    let (cfh, rfh) = syms::<FnFreeHC>("LZ4_freeStreamHC");
    let (crs, rrs) = syms::<FnReset>("LZ4_resetStreamHC");
    let (cl, rl) = syms::<FnLoad>("LZ4_loadDictHC");
    let (cs, rs) = syms::<FnSave>("LZ4_saveDictHC");
    let (cc, rc) = syms::<FnCont>("LZ4_compress_HC_continue");
    let mut rng = Rng::new(11);
    let dict = mkdata(Shape::Textish, 200_000, &mut rng);

    for lvl in [1i32, 2, 3, 9, 12] {
        // Rows 60-61
        for ds in [0i32, 1, 2, 3, 4, 5, 65535, 65536, 65537, 70000, 200_000] {
            let a0 = unsafe { cch() };
            let b0 = unsafe { rch() };
            unsafe { crs(a0, lvl) };
            unsafe { rrs(b0, lvl) };
            let a = unsafe { cl(a0, dict.as_ptr(), ds) };
            let b = unsafe { rl(b0, dict.as_ptr(), ds) };
            assert_eq!(a, b, "rows60-61 loadDictHC(ds={ds}, lvl={lvl}): C={a} R={b}");
            if ds > 65536 {
                assert_eq!(a, 65536, "row60 should clamp to 64 KB");
            }
            unsafe { cfh(a0) };
            unsafe { rfh(b0) };
        }
        // Rows 62-64: saveDictHC clamps after a real session
        for ds in [0i32, 1, 3, 4, 100, 65535, 65536, 70000] {
            let a0 = unsafe { cch() };
            let b0 = unsafe { rch() };
            unsafe { crs(a0, lvl) };
            unsafe { rrs(b0, lvl) };
            // build a short prefix so dictSize > prefixSize is exercised
            let prefix = 5000usize;
            let mut cd = vec![0u8; 8192 + prefix];
            let mut rd = vec![0u8; 8192 + prefix];
            unsafe { cc(a0, dict.as_ptr(), cd.as_mut_ptr(), prefix as i32, cd.len() as i32) };
            unsafe { rc(b0, dict.as_ptr(), rd.as_mut_ptr(), prefix as i32, rd.len() as i32) };
            let mut ca = vec![0u8; 200_016];
            let mut ra = vec![0u8; 200_016];
            let a = unsafe { cs(a0, ca.as_mut_ptr(), ds) };
            let b = unsafe { rs(b0, ra.as_mut_ptr(), ds) };
            assert_eq!(a, b, "rows62-64 saveDictHC(ds={ds}, lvl={lvl}): C={a} R={b}");
            assert_eq!(ca, ra, "rows62-64 saveDictHC buffer (ds={ds}, lvl={lvl})");
            if ds < 4 {
                assert_eq!(a, 0, "row63 dictSize < 4 must give 0");
            }
            unsafe { cfh(a0) };
            unsafe { rfh(b0) };
        }
        // saveDictHC(NULL, 0)
        let a0 = unsafe { cch() };
        let b0 = unsafe { rch() };
        unsafe { crs(a0, lvl) };
        unsafe { rrs(b0, lvl) };
        let a = unsafe { cs(a0, std::ptr::null_mut(), 0) };
        let b = unsafe { rs(b0, std::ptr::null_mut(), 0) };
        assert_eq!(a, b, "row64 saveDictHC(NULL, 0) lvl={lvl}");
        unsafe { cfh(a0) };
        unsafe { rfh(b0) };
    }
}

/// Rows 65-66: `LZ4_resetStreamStateHC` success and init-failure returns.
#[test]
fn e_hc_reset_stream_state() {
    type FnRSS = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
    type FnI = unsafe extern "C" fn() -> i32;
    let (c, r) = syms::<FnRSS>("LZ4_resetStreamStateHC");
    let (cs, rs) = syms::<FnI>("LZ4_sizeofStreamStateHC");
    assert_eq!(unsafe { cs() }, unsafe { rs() }, "LZ4_sizeofStreamStateHC");
    let sz = unsafe { cs() } as usize;
    let mut inbuf = mkdata(Shape::Textish, 1000, &mut Rng::new(12));

    // Row 66: success on a correctly sized & aligned state
    let mut cst = vec![0u64; sz / 8 + 4];
    let mut rst = vec![0u64; sz / 8 + 4];
    let a = unsafe { c(cst.as_mut_ptr() as *mut u8, inbuf.as_mut_ptr()) };
    let b = unsafe { r(rst.as_mut_ptr() as *mut u8, inbuf.as_mut_ptr()) };
    assert_eq!(a, b, "row66 resetStreamStateHC success: C={a} R={b}");
    assert_eq!(a, 0);
    // Row 65: init failure -> 1 (misaligned state)
    let mut big = vec![0u8; sz + 16];
    for off in 1..8usize {
        let p = unsafe { big.as_mut_ptr().add(off) };
        let a = unsafe { c(p, inbuf.as_mut_ptr()) };
        let b = unsafe { r(p, inbuf.as_mut_ptr()) };
        assert_eq!(a, b, "row65 resetStreamStateHC(misaligned {off}): C={a} R={b}");
    }
    // NULL state -> LZ4_initStreamHC returns NULL -> 1
    let a = unsafe { c(std::ptr::null_mut(), inbuf.as_mut_ptr()) };
    let b = unsafe { r(std::ptr::null_mut(), inbuf.as_mut_ptr()) };
    assert_eq!(a, b, "row65 resetStreamStateHC(NULL): C={a} R={b}");
    assert_eq!(a, 1, "row65 C should return 1");
}

/// Rows 68-70: deprecated HC wrappers with out-of-range levels, and the silent
/// clamp in `LZ4_setCompressionLevel`.
#[test]
fn e_hc_deprecated_level_clamps() {
    type Fn3 = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
    type Fn4 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
    type Fn5 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(13);

    for &len in &[0usize, 1, 13, 1000] {
        let src = mkdata(Shape::Textish, len, &mut rng);
        let bound = unsafe { cb(len as i32) }.max(1) as usize;
        // Row 68: level fixed at 0 -> mapped to DEFAULT
        let (c, r) = syms::<Fn3>("LZ4_compressHC");
        let mut cd = vec![0u8; bound];
        let mut rd = vec![0u8; bound];
        let a = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32) };
        let b = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32) };
        assert_eq!(a, b, "row68 LZ4_compressHC len={len}");
        assert_eq!(&cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);
        // and with a too-small destination
        let (c, r) = syms::<Fn4>("LZ4_compressHC_limitedOutput");
        for cap in 0..=(a.max(1)) {
            let mut cd = vec![0u8; cap.max(1) as usize];
            let mut rd = vec![0u8; cap.max(1) as usize];
            let x = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap) };
            let y = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap) };
            assert_eq!(x, y, "row68 compressHC_limitedOutput len={len} cap={cap}");
        }
        // Row 69: LZ4_compressHC2* level clamps
        let (c, r) = syms::<Fn4>("LZ4_compressHC2");
        for lvl in [i32::MIN, -1, 0, 1, 2, 12, 13, 100, i32::MAX] {
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let x = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, lvl) };
            let y = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, lvl) };
            assert_eq!(x, y, "row69 compressHC2 len={len} lvl={lvl}");
            assert_eq!(&cd[..x.max(0) as usize], &rd[..y.max(0) as usize]);
            let (c2, r2) = syms::<Fn5>("LZ4_compressHC2_limitedOutput");
            for cap in [0i32, 1, x / 2, x, x + 1] {
                let mut cd = vec![0u8; cap.max(1) as usize];
                let mut rd = vec![0u8; cap.max(1) as usize];
                let p = unsafe { c2(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap, lvl) };
                let q = unsafe { r2(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap, lvl) };
                assert_eq!(p, q, "row69 compressHC2_limitedOutput len={len} cap={cap} lvl={lvl}");
            }
        }
    }

    // Row 70: LZ4_setCompressionLevel silent clamp, observed through output
    type FnCreateHC = unsafe extern "C" fn() -> *mut u8;
    type FnFreeHC = unsafe extern "C" fn(*mut u8) -> i32;
    type FnReset = unsafe extern "C" fn(*mut u8, i32);
    type FnSetLvl = unsafe extern "C" fn(*mut u8, i32);
    type FnCont = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
    let (cch, rch) = syms::<FnCreateHC>("LZ4_createStreamHC");
    let (cfh, rfh) = syms::<FnFreeHC>("LZ4_freeStreamHC");
    let (crs, rrs) = syms::<FnReset>("LZ4_resetStreamHC");
    let (csl, rsl) = syms::<FnSetLvl>("LZ4_setCompressionLevel");
    let (cc, rc) = syms::<FnCont>("LZ4_compress_HC_continue");
    let src = mkdata(Shape::Textish, 20000, &mut rng);
    for lvl in [i32::MIN, -1, 0, 1, 2, 12, 13, 100, i32::MAX] {
        let a0 = unsafe { cch() };
        let b0 = unsafe { rch() };
        unsafe { crs(a0, 9) };
        unsafe { rrs(b0, 9) };
        unsafe { csl(a0, lvl) };
        unsafe { rsl(b0, lvl) };
        let mut cd = vec![0u8; 40000];
        let mut rd = vec![0u8; 40000];
        let a = unsafe { cc(a0, src.as_ptr(), cd.as_mut_ptr(), 20000, 40000) };
        let b = unsafe { rc(b0, src.as_ptr(), rd.as_mut_ptr(), 20000, 40000) };
        assert_eq!(a, b, "row70 setCompressionLevel({lvl})");
        assert_eq!(&cd[..a as usize], &rd[..b as usize]);
        unsafe { cfh(a0) };
        unsafe { rfh(b0) };
    }
}
