//! Boundary conditions around the algorithm's internal thresholds.
mod common;

use common::*;

/// `LZ4_64Klimit` = 64 KB + (MFLIMIT-1) = 65547 is where `LZ4_compress_fast*`
/// switches from the `byU16` hash table to `byU32`. Also covers MFLIMIT (12),
/// LASTLITERALS (5), MINMATCH (4) and the 64 KB dictionary window.
const AROUND_64K: [usize; 24] = [
    65524, 65525, 65526, 65530, 65531, 65532, 65533, 65534, 65535, 65536, 65537, 65538, 65539,
    65540, 65541, 65542, 65543, 65544, 65545, 65546, 65547, 65548, 65549, 65560,
];

/// Small sizes covering the literal-run / match-length encoding transitions.
const TINY: [usize; 30] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 254, 255, 256, 257,
    268, 269, 270, 271, 272,
];

#[test]
fn block_compress_around_table_type_switch() {
    let (c_def, r_def) = pair!("LZ4_compress_default", fn(*const u8, *mut u8, i32, i32) -> i32);
    let (c_fast, r_fast) = pair!(
        "LZ4_compress_fast",
        fn(*const u8, *mut u8, i32, i32, i32) -> i32
    );
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
            for &sz in AROUND_64K.iter().chain(TINY.iter()) {
                let data = g(sz, 301 + sz as u64);
                let bound = cbound(sz as i32).max(16);
                for &accel in &[1i32, 2, 65537] {
                    let mut a = vec![0x7Eu8; bound as usize + 64];
                    let mut b = vec![0x7Eu8; bound as usize + 64];
                    let ra = c_fast(data.as_ptr(), a.as_mut_ptr(), sz as i32, bound, accel);
                    let rb = r_fast(data.as_ptr(), b.as_mut_ptr(), sz as i32, bound, accel);
                    assert_eq!(ra, rb, "fast {} sz={} accel={}", gname, sz, accel);
                    beq!(a, b, "fast bytes {} sz={} accel={}", gname, sz, accel);
                    // and the block must decode back
                    let mut o = vec![0u8; sz + 64];
                    let n = c_dec(a.as_ptr(), o.as_mut_ptr(), ra, sz as i32 + 64);
                    assert_eq!(n, sz as i32, "fast decode {} sz={}", gname, sz);
                    assert_eq!(&o[..sz], &data[..]);
                    let mut o2 = vec![0u8; sz + 64];
                    assert_eq!(
                        n,
                        r_dec(b.as_ptr(), o2.as_mut_ptr(), rb, sz as i32 + 64),
                        "rust decode {} sz={}",
                        gname,
                        sz
                    );
                    beq!(o, o2, "decode bytes {} sz={}", gname, sz);
                }
                let mut a = vec![0x7Eu8; bound as usize + 64];
                let mut b = vec![0x7Eu8; bound as usize + 64];
                assert_eq!(
                    c_def(data.as_ptr(), a.as_mut_ptr(), sz as i32, bound),
                    r_def(data.as_ptr(), b.as_mut_ptr(), sz as i32, bound),
                    "default {} sz={}",
                    gname,
                    sz
                );
                beq!(a, b, "default bytes {} sz={}", gname, sz);

                for &lvl in &[1i32, 9, 12] {
                    let mut a = vec![0x7Eu8; bound as usize + 64];
                    let mut b = vec![0x7Eu8; bound as usize + 64];
                    assert_eq!(
                        c_hc(data.as_ptr(), a.as_mut_ptr(), sz as i32, bound, lvl),
                        r_hc(data.as_ptr(), b.as_mut_ptr(), sz as i32, bound, lvl),
                        "HC {} sz={} lvl={}",
                        gname,
                        sz,
                        lvl
                    );
                    beq!(a, b, "HC bytes {} sz={} lvl={}", gname, sz, lvl);
                }
            }
        }
    }
}

/// Data engineered to produce matches at exactly `LZ4_DISTANCE_MAX` (65535) and
/// just beyond it, where an offset can no longer be encoded.
#[test]
fn block_max_distance_matches() {
    let (c_fast, r_fast) = pair!(
        "LZ4_compress_fast",
        fn(*const u8, *mut u8, i32, i32, i32) -> i32
    );
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
        let marker = b"UNIQUE-MARKER-SEQUENCE-0123456789";
        for gap in [
            65500usize, 65519, 65520, 65530, 65534, 65535, 65536, 65537, 65600, 131071, 131072,
        ] {
            // marker, `gap` bytes of filler, marker again
            let mut data = Vec::with_capacity(marker.len() * 2 + gap + 64);
            data.extend_from_slice(marker);
            data.extend(gen_random(gap, 4242).into_iter());
            data.extend_from_slice(marker);
            let n = data.len();
            data.resize(n + 64, 0);
            data.truncate(n);

            let bound = cbound(n as i32);
            for &accel in &[1i32, 3] {
                let mut a = vec![0u8; bound as usize + 64];
                let mut b = vec![0u8; bound as usize + 64];
                let ra = c_fast(data.as_ptr(), a.as_mut_ptr(), n as i32, bound, accel);
                let rb = r_fast(data.as_ptr(), b.as_mut_ptr(), n as i32, bound, accel);
                assert_eq!(ra, rb, "maxdist fast gap={} accel={}", gap, accel);
                beq!(a, b, "maxdist fast bytes gap={} accel={}", gap, accel);
                let mut o = vec![0u8; n + 64];
                assert_eq!(
                    c_dec(a.as_ptr(), o.as_mut_ptr(), ra, n as i32 + 64),
                    n as i32
                );
                assert_eq!(&o[..n], &data[..]);
                let mut o2 = vec![0u8; n + 64];
                assert_eq!(
                    r_dec(b.as_ptr(), o2.as_mut_ptr(), rb, n as i32 + 64),
                    n as i32
                );
                beq!(o, o2, "maxdist decode bytes gap={}", gap);
            }
            for &lvl in &[1i32, 9, 12] {
                let mut a = vec![0u8; bound as usize + 64];
                let mut b = vec![0u8; bound as usize + 64];
                let ra = c_hc(data.as_ptr(), a.as_mut_ptr(), n as i32, bound, lvl);
                let rb = r_hc(data.as_ptr(), b.as_mut_ptr(), n as i32, bound, lvl);
                assert_eq!(ra, rb, "maxdist HC gap={} lvl={}", gap, lvl);
                beq!(a, b, "maxdist HC bytes gap={} lvl={}", gap, lvl);
            }
        }
    }
}

/// Long runs of a single byte force the very long match-length encodings
/// (repeated 0xFF bytes in the length field) and the `pattern analysis` path
/// in the HC matchfinder.
#[test]
fn block_long_repetitions() {
    let (c_fast, r_fast) = pair!(
        "LZ4_compress_fast",
        fn(*const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (c_hc, r_hc) = pair!(
        "LZ4_compress_HC",
        fn(*const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (c_ds, r_ds) = pair!(
        "LZ4_compress_destSize",
        fn(*const u8, *mut u8, *mut i32, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    let (c_dec, r_dec) = pair!(
        "LZ4_decompress_safe",
        fn(*const u8, *mut u8, i32, i32) -> i32
    );
    unsafe {
        for &period in &[1usize, 2, 3, 4, 5, 8, 15, 16, 17, 65535, 65536] {
            for &total in &[300usize, 70000, 300000] {
                let mut data: Vec<u8> = Vec::with_capacity(total + 64);
                for i in 0..total {
                    data.push((i % period) as u8);
                }
                data.resize(total + 64, 0);
                data.truncate(total);
                let bound = cbound(total as i32);

                for &accel in &[1i32, 5] {
                    let mut a = vec![0u8; bound as usize + 64];
                    let mut b = vec![0u8; bound as usize + 64];
                    let ra = c_fast(data.as_ptr(), a.as_mut_ptr(), total as i32, bound, accel);
                    let rb = r_fast(data.as_ptr(), b.as_mut_ptr(), total as i32, bound, accel);
                    assert_eq!(ra, rb, "rep fast period={} total={}", period, total);
                    beq!(a, b, "rep fast bytes period={} total={}", period, total);
                    let mut o = vec![0u8; total + 64];
                    assert_eq!(
                        c_dec(a.as_ptr(), o.as_mut_ptr(), ra, total as i32 + 64),
                        total as i32
                    );
                    assert_eq!(&o[..total], &data[..]);
                    let mut o2 = vec![0u8; total + 64];
                    assert_eq!(
                        r_dec(b.as_ptr(), o2.as_mut_ptr(), rb, total as i32 + 64),
                        total as i32
                    );
                    beq!(o, o2, "rep decode bytes period={}", period);
                }
                for &lvl in &[1i32, 3, 9, 10, 12] {
                    let mut a = vec![0u8; bound as usize + 64];
                    let mut b = vec![0u8; bound as usize + 64];
                    let ra = c_hc(data.as_ptr(), a.as_mut_ptr(), total as i32, bound, lvl);
                    let rb = r_hc(data.as_ptr(), b.as_mut_ptr(), total as i32, bound, lvl);
                    assert_eq!(ra, rb, "rep HC period={} total={} lvl={}", period, total, lvl);
                    beq!(a, b, "rep HC bytes period={} total={} lvl={}", period, total, lvl);
                }
                for &t in &[1i32, 16, 100, 5000] {
                    let mut a = vec![0u8; t as usize + 128];
                    let mut b = vec![0u8; t as usize + 128];
                    let mut sa = total as i32;
                    let mut sb = total as i32;
                    let ra = c_ds(data.as_ptr(), a.as_mut_ptr(), &mut sa, t);
                    let rb = r_ds(data.as_ptr(), b.as_mut_ptr(), &mut sb, t);
                    assert_eq!(ra, rb, "rep destSize period={} t={}", period, t);
                    assert_eq!(sa, sb, "rep destSize srcSize period={} t={}", period, t);
                    beq!(a, b, "rep destSize bytes period={} t={}", period, t);
                }
            }
        }
    }
}

/// Frame-level boundary cases: a declared `contentSize` that does not match the
/// data actually supplied, and a frame carrying a `dictID`.
#[test]
fn frame_content_size_and_dict_id() {
    use std::os::raw::c_void;

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    struct FrameInfo {
        block_size_id: u32,
        block_mode: u32,
        content_checksum_flag: u32,
        frame_type: u32,
        content_size: u64,
        dict_id: u32,
        block_checksum_flag: u32,
    }
    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    struct Preferences {
        frame_info: FrameInfo,
        compression_level: i32,
        auto_flush: u32,
        favor_dec_speed: u32,
        reserved: [u32; 3],
    }

    let (c_cnew, r_cnew) = pair!(
        "LZ4F_createCompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_cfree, r_cfree) = pair!("LZ4F_freeCompressionContext", fn(*mut c_void) -> usize);
    let (c_begin, r_begin) = pair!(
        "LZ4F_compressBegin",
        fn(*mut c_void, *mut u8, usize, *const Preferences) -> usize
    );
    let (c_upd, r_upd) = pair!(
        "LZ4F_compressUpdate",
        fn(*mut c_void, *mut u8, usize, *const u8, usize, *const c_void) -> usize
    );
    let (c_end, r_end) = pair!(
        "LZ4F_compressEnd",
        fn(*mut c_void, *mut u8, usize, *const c_void) -> usize
    );
    let (c_cb, _) = pair!("LZ4F_compressBound", fn(usize, *const Preferences) -> usize);
    let (c_dnew, r_dnew) = pair!(
        "LZ4F_createDecompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_dfree, r_dfree) = pair!("LZ4F_freeDecompressionContext", fn(*mut c_void) -> usize);
    let (c_gfi, r_gfi) = pair!(
        "LZ4F_getFrameInfo",
        fn(*mut c_void, *mut FrameInfo, *const u8, *mut usize) -> usize
    );
    let (c_dec, r_dec) = pair!(
        "LZ4F_decompress",
        fn(*mut c_void, *mut u8, *mut usize, *const u8, *mut usize, *const c_void) -> usize
    );

    let actual = 5000usize;
    unsafe {
        for &declared in &[0u64, 1, actual as u64 - 1, actual as u64, actual as u64 + 1, 1 << 40] {
            for &dict_id in &[0u32, 1, 0xDEAD_BEEF] {
                for &cc in &[0u32, 1] {
                    let mut p = Preferences::default();
                    p.frame_info.content_size = declared;
                    p.frame_info.dict_id = dict_id;
                    p.frame_info.content_checksum_flag = cc;
                    let data = gen_textish(actual, 311);

                    let mut cc_ctx: *mut c_void = std::ptr::null_mut();
                    let mut rc_ctx: *mut c_void = std::ptr::null_mut();
                    assert_eq!(c_cnew(&mut cc_ctx, 100), r_cnew(&mut rc_ctx, 100));
                    let mut ha = vec![0u8; 64];
                    let mut hb = vec![0u8; 64];
                    let ra = c_begin(cc_ctx, ha.as_mut_ptr(), ha.len(), &p);
                    let rb = r_begin(rc_ctx, hb.as_mut_ptr(), hb.len(), &p);
                    assert_eq!(ra, rb, "begin declared={} dictID={}", declared, dict_id);
                    beq!(ha[..ra], hb[..rb], "header declared={}", declared);
                    let mut framec = ha[..ra].to_vec();
                    let mut framer = hb[..rb].to_vec();

                    let cap = c_cb(actual, &p);
                    let mut a = vec![0u8; cap + 64];
                    let mut b = vec![0u8; cap + 64];
                    let ra = c_upd(
                        cc_ctx,
                        a.as_mut_ptr(),
                        cap,
                        data.as_ptr(),
                        actual,
                        std::ptr::null(),
                    );
                    let rb = r_upd(
                        rc_ctx,
                        b.as_mut_ptr(),
                        cap,
                        data.as_ptr(),
                        actual,
                        std::ptr::null(),
                    );
                    assert_eq!(ra, rb, "update declared={}", declared);
                    if ra <= cap {
                        beq!(a[..ra], b[..rb], "update bytes declared={}", declared);
                        framec.extend_from_slice(&a[..ra]);
                        framer.extend_from_slice(&b[..rb]);
                    }

                    let cap = c_cb(0, &p);
                    let mut a = vec![0u8; cap + 64];
                    let mut b = vec![0u8; cap + 64];
                    // compressEnd checks the declared contentSize against what
                    // was actually written and reports an error on mismatch.
                    let ra = c_end(cc_ctx, a.as_mut_ptr(), cap, std::ptr::null());
                    let rb = r_end(rc_ctx, b.as_mut_ptr(), cap, std::ptr::null());
                    assert_eq!(ra, rb, "end declared={} dictID={}", declared, dict_id);
                    if ra <= cap {
                        beq!(a[..ra], b[..rb], "end bytes declared={}", declared);
                        framec.extend_from_slice(&a[..ra]);
                        framer.extend_from_slice(&b[..rb]);
                    }
                    beq!(framec, framer, "frame declared={} dictID={}", declared, dict_id);
                    assert_eq!(c_cfree(cc_ctx), r_cfree(rc_ctx));

                    // decode side: frameInfo must report the declared values
                    let mut cd: *mut c_void = std::ptr::null_mut();
                    let mut rd: *mut c_void = std::ptr::null_mut();
                    assert_eq!(c_dnew(&mut cd, 100), r_dnew(&mut rd, 100));
                    let mut fic = FrameInfo::default();
                    let mut fir = FrameInfo::default();
                    let mut sc = framec.len();
                    let mut sr = framer.len();
                    let ra = c_gfi(cd, &mut fic, framec.as_ptr(), &mut sc);
                    let rb = r_gfi(rd, &mut fir, framer.as_ptr(), &mut sr);
                    assert_eq!(ra, rb, "getFrameInfo declared={}", declared);
                    assert_eq!(sc, sr);
                    assert_eq!(fic, fir, "frameInfo declared={} dictID={}", declared, dict_id);

                    let mut bufc = vec![0u8; actual + (1 << 16)];
                    let mut bufr = vec![0u8; actual + (1 << 16)];
                    let mut dsc = bufc.len();
                    let mut dsr = bufr.len();
                    let mut ssc = framec.len() - sc;
                    let mut ssr = framer.len() - sr;
                    let ra = c_dec(
                        cd,
                        bufc.as_mut_ptr(),
                        &mut dsc,
                        framec[sc..].as_ptr(),
                        &mut ssc,
                        std::ptr::null(),
                    );
                    let rb = r_dec(
                        rd,
                        bufr.as_mut_ptr(),
                        &mut dsr,
                        framer[sr..].as_ptr(),
                        &mut ssr,
                        std::ptr::null(),
                    );
                    assert_eq!(ra, rb, "decompress declared={}", declared);
                    assert_eq!(dsc, dsr);
                    assert_eq!(ssc, ssr);
                    beq!(bufc[..dsc], bufr[..dsr], "decompress bytes declared={}", declared);
                    assert_eq!(c_dfree(cd), r_dfree(rd));
                }
            }
        }
    }
}
