//! Level 1: the leaf functions - hashing, array growth, the string arena and
//! `strkey`. Both implementations are invoked exclusively through their
//! `dlopen`ed exports.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

/// Sanity check: the two libraries really are two independent objects (their
/// `stbds_hash_seed` globals must not be shared), otherwise every other test in
/// this suite would be comparing one implementation against itself.
#[test]
fn libraries_are_independent() {
    let (c, r) = load_pair();
    let _g = SEED_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        (c.rand_seed)(0x1111_1111);
        (r.rand_seed)(0x2222_2222);
        let ct = (c.shmode_func)(16, SH_ARENA);
        let rt = (r.shmode_func)(16, SH_ARENA);
        let cseed = (*(map_header(ct, 16).hash_table as *const HashIndex)).seed;
        let rseed = (*(map_header(rt, 16).hash_table as *const HashIndex)).seed;
        assert_eq!(cseed, 0x1111_1111, "C library global seed not honoured");
        assert_eq!(rseed, 0x2222_2222, "Rust library global seed not honoured");
        (c.hmfree_func)((ct as *mut u8).sub(16) as *mut c_void, 16);
        (r.hmfree_func)((rt as *mut u8).sub(16) as *mut c_void, 16);
    }
}

#[test]
fn hash_string_matches() {
    let (c, r) = load_pair();
    let mut inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"bob".to_vec(),
        b"sally".to_vec(),
        b"fred".to_vec(),
        b"jen".to_vec(),
        b"doug".to_vec(),
        b"the quick brown fox jumps over the lazy dog".to_vec(),
        vec![0x80, 0xff, 0x7f, 0x01],
        vec![0xff; 64],
    ];
    for n in 0..48usize {
        inputs.push((0..n).map(|i| (33 + (i * 7) % 90) as u8).collect());
    }
    // High-bit bytes: exercises the `(unsigned char)` conversion.
    for n in 1..40usize {
        inputs.push((0..n).map(|i| (128 + (i * 13) % 128) as u8).collect());
    }

    let seeds: [usize; 8] = [
        0,
        1,
        0x3141_5926,
        usize::MAX,
        0x8000_0000_0000_0000,
        0xdead_beef,
        0x0123_4567_89ab_cdef,
        42,
    ];

    let mut cases = 0;
    for inp in &inputs {
        let mut buf = inp.clone();
        buf.push(0);
        for &seed in &seeds {
            unsafe {
                let cv = (c.hash_string)(buf.as_mut_ptr() as *mut c_char, seed);
                let rv = (r.hash_string)(buf.as_mut_ptr() as *mut c_char, seed);
                assert_eq!(
                    cv, rv,
                    "hash_string({:?}, {seed:#x}): C={cv:#x} Rust={rv:#x}",
                    String::from_utf8_lossy(inp)
                );
            }
            cases += 1;
        }
    }
    assert!(cases > 500);
}

#[test]
fn hash_bytes_matches() {
    let (c, r) = load_pair();
    let seeds: [usize; 8] = [
        0,
        1,
        0x3141_5926,
        usize::MAX,
        0x8000_0000_0000_0000,
        0xdead_beef_cafe_babe,
        0x0123_4567_89ab_cdef,
        7,
    ];

    // Deterministic pseudo-random data with plenty of high-bit-set bytes,
    // which is where the C code relies on int overflow / sign extension.
    let mut data = vec![0u8; 200];
    let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
    for b in data.iter_mut() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *b = (x >> 24) as u8;
    }

    for len in 0..=128usize {
        for &seed in &seeds {
            unsafe {
                let cv = (c.hash_bytes)(data.as_mut_ptr() as *mut c_void, len, seed);
                let rv = (r.hash_bytes)(data.as_mut_ptr() as *mut c_void, len, seed);
                assert_eq!(cv, rv, "hash_bytes(len={len}, seed={seed:#x})");
            }
        }
    }

    // All-0xff and all-0x00 buffers for every partial-tail length.
    for fill in [0x00u8, 0xffu8, 0x80u8, 0x7fu8] {
        let mut buf = vec![fill; 40];
        for len in 0..40usize {
            for &seed in &seeds {
                unsafe {
                    let cv = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    let rv = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    assert_eq!(cv, rv, "hash_bytes(fill={fill:#x}, len={len}, seed={seed:#x})");
                }
            }
        }
    }

    // Every single-byte value, at every offset inside one 8-byte block.
    for off in 0..8usize {
        for v in 0..=255u8 {
            let mut buf = vec![0u8; 16];
            buf[off] = v;
            unsafe {
                let cv = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 8, 12345);
                let rv = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 8, 12345);
                assert_eq!(cv, rv, "hash_bytes single byte off={off} v={v}");
                let cv = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, off + 1, 12345);
                let rv = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, off + 1, 12345);
                assert_eq!(cv, rv, "hash_bytes tail off={off} v={v}");
            }
        }
    }
}

/// `stbds_arrgrowf` + `stbds_arrfreef`: compare the capacity/length/temp
/// progression for the `arrmaybegrow` pattern and for explicit `arrsetcap`.
#[test]
fn arrgrowf_matches() {
    let (c, r) = load_pair();

    // Degenerate call that returns the input unchanged (min_cap 0, addlen 0).
    unsafe {
        assert!((c.arrgrowf)(std::ptr::null_mut(), 4, 0, 0).is_null());
        assert!((r.arrgrowf)(std::ptr::null_mut(), 4, 0, 0).is_null());
    }

    for &elemsize in &[1usize, 2, 4, 8, 16, 24, 100] {
        for &addlen in &[1usize, 2, 3, 5, 8] {
            let mut cp: *mut c_void = std::ptr::null_mut();
            let mut rp: *mut c_void = std::ptr::null_mut();
            let mut clog = Vec::new();
            let mut rlog = Vec::new();
            unsafe {
                // Emulate repeated `arrmaybegrow(a, addlen); length += addlen`.
                for step in 0..40usize {
                    let cl = if cp.is_null() { 0 } else { header(cp).length };
                    let cc = if cp.is_null() { 0 } else { header(cp).capacity };
                    if cp.is_null() || cl + addlen > cc {
                        cp = (c.arrgrowf)(cp, elemsize, addlen, 0);
                    }
                    let rl = if rp.is_null() { 0 } else { header(rp).length };
                    let rc = if rp.is_null() { 0 } else { header(rp).capacity };
                    if rp.is_null() || rl + addlen > rc {
                        rp = (r.arrgrowf)(rp, elemsize, addlen, 0);
                    }
                    (*((cp as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length += addlen;
                    (*((rp as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length += addlen;

                    let ch = header(cp);
                    let rh = header(rp);
                    clog.extend_from_slice(
                        format!(
                            "{step}:{},{},{},{};",
                            ch.length,
                            ch.capacity,
                            ch.temp,
                            ch.hash_table.is_null()
                        )
                        .as_bytes(),
                    );
                    rlog.extend_from_slice(
                        format!(
                            "{step}:{},{},{},{};",
                            rh.length,
                            rh.capacity,
                            rh.temp,
                            rh.hash_table.is_null()
                        )
                        .as_bytes(),
                    );
                }
                (c.arrfreef)(cp);
                (r.arrfreef)(rp);
            }
            assert_same(
                &format!("arrgrowf elemsize={elemsize} addlen={addlen}"),
                &clog,
                &rlog,
            );
        }
    }

    // Explicit min_cap requests (arrsetcap) including shrink attempts.
    for &elemsize in &[1usize, 8, 40] {
        let mut cp: *mut c_void = std::ptr::null_mut();
        let mut rp: *mut c_void = std::ptr::null_mut();
        let mut clog = Vec::new();
        let mut rlog = Vec::new();
        unsafe {
            for &cap in &[1usize, 1, 2, 3, 4, 5, 9, 8, 100, 50, 4096, 4097, 1] {
                cp = (c.arrgrowf)(cp, elemsize, 0, cap);
                rp = (r.arrgrowf)(rp, elemsize, 0, cap);
                let ch = header(cp);
                let rh = header(rp);
                clog.extend_from_slice(format!("{},{};", ch.length, ch.capacity).as_bytes());
                rlog.extend_from_slice(format!("{},{};", rh.length, rh.capacity).as_bytes());
            }
            (c.arrfreef)(cp);
            (r.arrfreef)(rp);
        }
        assert_same(&format!("arrsetcap elemsize={elemsize}"), &clog, &rlog);
    }
}

/// `stbds_stralloc` / `stbds_strreset` on a standalone arena.
#[test]
fn stralloc_matches() {
    let (c, r) = load_pair();

    let mut words: Vec<Vec<u8>> = vec![
        b"a".to_vec(),
        b"bb".to_vec(),
        b"hello world".to_vec(),
        b"".to_vec(),
    ];
    // Lengths that straddle the 512-byte first block and the doubling schedule.
    for n in [1usize, 7, 63, 100, 255, 400, 500, 511, 512, 513, 1000, 2048, 5000] {
        words.push(vec![b'x'; n]);
    }
    for i in 0..60usize {
        words.push(vec![b'a' + (i % 26) as u8; 1 + (i * 37) % 300]);
    }

    let mut ca = StringArena::new();
    let mut ra = StringArena::new();
    let mut clog = Vec::new();
    let mut rlog = Vec::new();
    unsafe {
        for (i, w) in words.iter().enumerate() {
            let mut buf = w.clone();
            buf.push(0);
            let cp = (c.stralloc)(&mut ca, buf.as_mut_ptr() as *mut c_char);
            let rp = (r.stralloc)(&mut ra, buf.as_mut_ptr() as *mut c_char);
            clog.extend_from_slice(
                format!(
                    "{i}:{:?} rem={} blk={} mode={} snull={};",
                    cstr_bytes(cp),
                    ca.remaining,
                    ca.block,
                    ca.mode,
                    ca.storage.is_null()
                )
                .as_bytes(),
            );
            rlog.extend_from_slice(
                format!(
                    "{i}:{:?} rem={} blk={} mode={} snull={};",
                    cstr_bytes(rp),
                    ra.remaining,
                    ra.block,
                    ra.mode,
                    ra.storage.is_null()
                )
                .as_bytes(),
            );
        }
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
    }
    assert_same("stralloc", &clog, &rlog);

    // strreset must zero the whole arena struct.
    let zero = StringArena::new();
    for (name, a) in [("C", &ca), ("Rust", &ra)] {
        assert!(a.storage.is_null(), "{name}: storage not cleared");
        assert_eq!(a.remaining, zero.remaining, "{name}: remaining not cleared");
        assert_eq!(a.block, zero.block, "{name}: block not cleared");
        assert_eq!(a.mode, zero.mode, "{name}: mode not cleared");
    }

    // strreset on an already-empty arena must be a no-op.
    let mut ce = StringArena::new();
    let mut re = StringArena::new();
    unsafe {
        (c.strreset)(&mut ce);
        (r.strreset)(&mut re);
    }
    assert!(ce.storage.is_null() && re.storage.is_null());
}

#[test]
fn strkey_matches() {
    let (c, r) = load_pair();
    unsafe {
        for n in [
            0i32,
            1,
            -1,
            7,
            42,
            -12345,
            999999,
            i32::MAX,
            i32::MIN,
            123456789,
        ] {
            let cv = cstr_bytes((c.strkey)(n));
            let rv = cstr_bytes((r.strkey)(n));
            assert_same(&format!("strkey({n})"), &cv, &rv);
        }
        // The buffer is static: the previous result must still be visible and
        // must be overwritten in place by the next call.
        let cp = (c.strkey)(5);
        let rp = (r.strkey)(5);
        let cp2 = (c.strkey)(6);
        let rp2 = (r.strkey)(6);
        assert_eq!(cp, cp2, "C strkey buffer moved");
        assert_eq!(rp, rp2, "Rust strkey buffer moved");
        assert_same("strkey reuse", &cstr_bytes(cp), &cstr_bytes(rp));
    }
}
