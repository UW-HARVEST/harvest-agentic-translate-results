//! Block-level LZ4 API: single-shot compression / decompression.
mod common;

use common::*;

const ACCELS: [i32; 10] = [0, 1, 2, 3, 4, 10, 100, 65537, 65538, -1];

/// Compress with the C library (reference) and return the frame bytes.
fn c_compress(data: &[u8]) -> Vec<u8> {
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    let (cf, _) = pair!("LZ4_compress_default", fn(*const u8, *mut u8, i32, i32) -> i32);
    unsafe {
        let cap = cbound(data.len() as i32).max(16);
        let mut out = vec![0u8; cap as usize];
        let n = cf(data.as_ptr(), out.as_mut_ptr(), data.len() as i32, cap);
        assert!(n > 0 || data.is_empty(), "C compress failed");
        out.truncate(n as usize);
        out
    }
}

#[test]
fn compress_default_and_fast() {
    let (c_def, r_def) = pair!("LZ4_compress_default", fn(*const u8, *mut u8, i32, i32) -> i32);
    let (c_fast, r_fast) = pair!(
        "LZ4_compress_fast",
        fn(*const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (cbound, rbound) = pair!("LZ4_compressBound", fn(i32) -> i32);

    unsafe {
        for (gname, g) in GENS {
            for &sz in &SIZES {
                let data = g(sz, sz as u64 * 31 + gname.len() as u64);
                let bound = cbound(data.len() as i32).max(16);
                assert_eq!(bound, rbound(data.len() as i32).max(16));

                // full capacity
                let mut a = vec![0xCDu8; bound as usize + 64];
                let mut b = vec![0xCDu8; bound as usize + 64];
                let ra = c_def(data.as_ptr(), a.as_mut_ptr(), data.len() as i32, bound);
                let rb = r_def(data.as_ptr(), b.as_mut_ptr(), data.len() as i32, bound);
                assert_eq!(ra, rb, "compress_default {} sz={}", gname, sz);
                beq!(a, b, "compress_default bytes {} sz={}", gname, sz);

                for &accel in &ACCELS {
                    let mut a = vec![0xCDu8; bound as usize + 64];
                    let mut b = vec![0xCDu8; bound as usize + 64];
                    let ra = c_fast(data.as_ptr(), a.as_mut_ptr(), data.len() as i32, bound, accel);
                    let rb = r_fast(data.as_ptr(), b.as_mut_ptr(), data.len() as i32, bound, accel);
                    assert_eq!(ra, rb, "compress_fast {} sz={} accel={}", gname, sz, accel);
                    beq!(a, b, "compress_fast bytes {} sz={} accel={}", gname, sz, accel);
                }
            }
        }
    }
}

#[test]
fn compress_tight_and_failing_capacities() {
    let (c_def, r_def) = pair!("LZ4_compress_default", fn(*const u8, *mut u8, i32, i32) -> i32);
    unsafe {
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 5, 17, 100, 1000, 4096, 20000] {
                let data = g(sz, 12345 + sz as u64);
                let exact = c_compress(&data).len() as i32;
                // Try a spread of capacities around/below the exact size, plus degenerate ones.
                let mut caps: Vec<i32> = vec![-1, 0, 1, 2, 3];
                for d in 0..8 {
                    caps.push((exact - d).max(0));
                }
                caps.push(exact);
                caps.push(exact + 1);
                caps.push(exact / 2);
                for &cap in &caps {
                    let n = (cap.max(0) as usize) + 128;
                    let mut a = vec![0x5Au8; n];
                    let mut b = vec![0x5Au8; n];
                    let ra = c_def(data.as_ptr(), a.as_mut_ptr(), data.len() as i32, cap);
                    let rb = r_def(data.as_ptr(), b.as_mut_ptr(), data.len() as i32, cap);
                    assert_eq!(ra, rb, "compress {} sz={} cap={}", gname, sz, cap);
                    beq!(a, b, "compress bytes {} sz={} cap={}", gname, sz, cap);
                }
            }
        }
    }
}

#[test]
fn compress_invalid_srcsize() {
    let (c_def, r_def) = pair!("LZ4_compress_default", fn(*const u8, *mut u8, i32, i32) -> i32);
    let data = gen_textish(4096, 5);
    unsafe {
        for &bad in &[-1i32, -100, i32::MIN, 0x7E000001, i32::MAX] {
            let mut a = vec![0u8; 8192];
            let mut b = vec![0u8; 8192];
            let ra = c_def(data.as_ptr(), a.as_mut_ptr(), bad, 8192);
            let rb = r_def(data.as_ptr(), b.as_mut_ptr(), bad, 8192);
            assert_eq!(ra, rb, "compress srcSize={}", bad);
            if ra <= 0 {
                beq!(a, b);
            }
        }
    }
}

#[test]
fn compress_extstate() {
    let (c_size, _) = pair!("LZ4_sizeofState", fn() -> i32);
    let ssz = unsafe { c_size() } as usize;
    let (c_ext, r_ext) = pair!(
        "LZ4_compress_fast_extState",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (c_fr, r_fr) = pair!(
        "LZ4_compress_fast_extState_fastReset",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (c_init, r_init) = pair!("LZ4_initStream", fn(*mut u8, usize) -> *mut u8);
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);

    unsafe {
        let mut cs = Aligned::new(ssz);
        let mut rs = Aligned::new(ssz);
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 13, 100, 1000, 4096, 65536, 100000] {
                let data = g(sz, 777 + sz as u64);
                let bound = cbound(data.len() as i32).max(16);
                for &accel in &[0i32, 1, 3, 100, 65537, -5] {
                    cs.zero();
                    rs.zero();
                    let mut a = vec![0u8; bound as usize + 32];
                    let mut b = vec![0u8; bound as usize + 32];
                    let ra = c_ext(
                        cs.ptr(),
                        data.as_ptr(),
                        a.as_mut_ptr(),
                        data.len() as i32,
                        bound,
                        accel,
                    );
                    let rb = r_ext(
                        rs.ptr(),
                        data.as_ptr(),
                        b.as_mut_ptr(),
                        data.len() as i32,
                        bound,
                        accel,
                    );
                    assert_eq!(ra, rb, "extState {} sz={} accel={}", gname, sz, accel);
                    beq!(a, b, "extState bytes {} sz={}", gname, sz);
                    assert_eq!(
                        cs.as_slice(),
                        rs.as_slice(),
                        "extState state bytes {} sz={} accel={}",
                        gname,
                        sz,
                        accel
                    );
                }

                // fastReset requires an initialized state
                cs.zero();
                rs.zero();
                assert_eq!(
                    c_init(cs.ptr(), ssz).is_null(),
                    r_init(rs.ptr(), ssz).is_null()
                );
                for &accel in &[1i32, 2, 7] {
                    let mut a = vec![0u8; bound as usize + 32];
                    let mut b = vec![0u8; bound as usize + 32];
                    let ra = c_fr(
                        cs.ptr(),
                        data.as_ptr(),
                        a.as_mut_ptr(),
                        data.len() as i32,
                        bound,
                        accel,
                    );
                    let rb = r_fr(
                        rs.ptr(),
                        data.as_ptr(),
                        b.as_mut_ptr(),
                        data.len() as i32,
                        bound,
                        accel,
                    );
                    assert_eq!(ra, rb, "fastReset {} sz={} accel={}", gname, sz, accel);
                    beq!(a, b, "fastReset bytes {} sz={}", gname, sz);
                    beq!(cs.as_slice(), rs.as_slice(), "fastReset state bytes");
                }
            }
        }
        // LZ4_initStream with too-small / unaligned buffers
        let mut buf = Aligned::new(ssz + 16);
        for extra in [0usize, 1, 2, 3, 4, 7] {
            let p = buf.ptr().add(extra);
            for size in [0usize, 8, ssz - 1, ssz, ssz + 1] {
                let a = c_init(p, size);
                let b = r_init(p, size);
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "initStream(off={},size={})",
                    extra,
                    size
                );
                if !a.is_null() {
                    assert_eq!(a, p as *mut u8);
                    assert_eq!(b, p as *mut u8);
                }
            }
        }
    }
}

#[test]
fn compress_destsize() {
    let (c_ds, r_ds) = pair!(
        "LZ4_compress_destSize",
        fn(*const u8, *mut u8, *mut i32, i32) -> i32
    );
    unsafe {
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 13, 100, 1000, 4096, 20000, 65536, 100000] {
                let data = g(sz, 4242 + sz as u64);
                let mut targets: Vec<i32> = vec![0, 1, 2, 3, 4, 5, 8, 16, 17, 64, 100, 255, 256];
                targets.push((sz as i32) / 4 + 1);
                targets.push((sz as i32) / 2 + 1);
                targets.push(sz as i32 + 16);
                targets.push(sz as i32 * 2 + 32);
                for &t in &targets {
                    let cap = (t.max(0) as usize) + 128;
                    let mut a = vec![0x33u8; cap];
                    let mut b = vec![0x33u8; cap];
                    let mut sa = data.len() as i32;
                    let mut sb = data.len() as i32;
                    let ra = c_ds(data.as_ptr(), a.as_mut_ptr(), &mut sa, t);
                    let rb = r_ds(data.as_ptr(), b.as_mut_ptr(), &mut sb, t);
                    assert_eq!(ra, rb, "destSize ret {} sz={} target={}", gname, sz, t);
                    assert_eq!(sa, sb, "destSize srcSize {} sz={} target={}", gname, sz, t);
                    beq!(a, b, "destSize bytes {} sz={} target={}", gname, sz, t);
                }
            }
        }
    }
}

#[test]
fn compress_destsize_extstate() {
    let (c_size, _) = pair!("LZ4_sizeofState", fn() -> i32);
    let ssz = unsafe { c_size() } as usize;
    let (c_ds, r_ds) = pair!(
        "LZ4_compress_destSize_extState",
        fn(*mut u8, *const u8, *mut u8, *mut i32, i32, i32) -> i32
    );
    unsafe {
        let mut cs = Aligned::new(ssz);
        let mut rs = Aligned::new(ssz);
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 100, 1000, 4096, 20000, 100000] {
                let data = g(sz, 5150 + sz as u64);
                for &t in &[0i32, 1, 4, 17, 100, 1000, sz as i32 + 16] {
                    for &accel in &[0i32, 1, 5, 65537] {
                        cs.zero();
                        rs.zero();
                        let cap = (t.max(0) as usize) + 128;
                        let mut a = vec![0x77u8; cap];
                        let mut b = vec![0x77u8; cap];
                        let mut sa = data.len() as i32;
                        let mut sb = data.len() as i32;
                        let ra = c_ds(cs.ptr(), data.as_ptr(), a.as_mut_ptr(), &mut sa, t, accel);
                        let rb = r_ds(rs.ptr(), data.as_ptr(), b.as_mut_ptr(), &mut sb, t, accel);
                        assert_eq!(ra, rb, "destSize_extState {} sz={} t={}", gname, sz, t);
                        assert_eq!(sa, sb, "destSize_extState srcSize {} sz={}", gname, sz);
                        beq!(a, b, "destSize_extState bytes {} sz={}", gname, sz);
                        beq!(cs.as_slice(), rs.as_slice(), "destSize_extState state");
                    }
                }
            }
        }
    }
}

#[test]
fn decompress_safe_roundtrip_and_cross() {
    let (c_dec, r_dec) = pair!(
        "LZ4_decompress_safe",
        fn(*const u8, *mut u8, i32, i32) -> i32
    );
    let (c_def, r_def) = pair!("LZ4_compress_default", fn(*const u8, *mut u8, i32, i32) -> i32);
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    unsafe {
        for (gname, g) in GENS {
            for &sz in &SIZES {
                let data = g(sz, 8888 + sz as u64);
                let bound = cbound(data.len() as i32).max(16);
                let mut comp_c = vec![0u8; bound as usize];
                let mut comp_r = vec![0u8; bound as usize];
                let nc = c_def(data.as_ptr(), comp_c.as_mut_ptr(), data.len() as i32, bound);
                let nr = r_def(data.as_ptr(), comp_r.as_mut_ptr(), data.len() as i32, bound);
                assert_eq!(nc, nr);
                beq!(comp_c, comp_r);

                for src in [&comp_c, &comp_r] {
                    for &cap in &[
                        sz as i32,
                        sz as i32 + 1,
                        sz as i32 + 100,
                        (sz as i32) - 1,
                        0,
                    ] {
                        if cap < 0 {
                            continue;
                        }
                        let n = cap as usize + 64;
                        let mut a = vec![0xEEu8; n];
                        let mut b = vec![0xEEu8; n];
                        let ra = c_dec(src.as_ptr(), a.as_mut_ptr(), nc, cap);
                        let rb = r_dec(src.as_ptr(), b.as_mut_ptr(), nc, cap);
                        assert_eq!(ra, rb, "decompress_safe {} sz={} cap={}", gname, sz, cap);
                        beq!(a, b, "decompress_safe bytes {} sz={} cap={}", gname, sz, cap);
                        if cap >= sz as i32 {
                            assert_eq!(ra, sz as i32);
                            assert_eq!(&a[..sz], &data[..]);
                        }
                    }
                    // truncated compressed input
                    for cut in 1..=8i32 {
                        if nc - cut < 0 {
                            continue;
                        }
                        let cap = sz as i32 + 64;
                        let mut a = vec![0xEEu8; cap as usize + 64];
                        let mut b = vec![0xEEu8; cap as usize + 64];
                        let ra = c_dec(src.as_ptr(), a.as_mut_ptr(), nc - cut, cap);
                        let rb = r_dec(src.as_ptr(), b.as_mut_ptr(), nc - cut, cap);
                        assert_eq!(ra, rb, "trunc decompress {} sz={} cut={}", gname, sz, cut);
                        beq!(a, b, "trunc decompress bytes {} sz={} cut={}", gname, sz, cut);
                    }
                }
            }
        }
    }
}

#[test]
fn decompress_safe_partial() {
    let (c_dec, r_dec) = pair!(
        "LZ4_decompress_safe_partial",
        fn(*const u8, *mut u8, i32, i32, i32) -> i32
    );
    unsafe {
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 13, 100, 1000, 4096, 20000, 65536] {
                let data = g(sz, 606 + sz as u64);
                let comp = c_compress(&data);
                let cs = comp.len() as i32;
                let mut targets: Vec<i32> = vec![0, 1, 2, 3, 15, 16, 17, 64, 100];
                targets.push(sz as i32 / 2);
                targets.push(sz as i32 - 1);
                targets.push(sz as i32);
                targets.push(sz as i32 + 1);
                targets.push(sz as i32 + 1000);
                for &t in &targets {
                    if t < 0 {
                        continue;
                    }
                    for &cap in &[t, t + 1, t + 64, sz as i32 + 64] {
                        if cap < 0 {
                            continue;
                        }
                        let n = cap as usize + 64;
                        let mut a = vec![0x11u8; n];
                        let mut b = vec![0x11u8; n];
                        let ra = c_dec(comp.as_ptr(), a.as_mut_ptr(), cs, t, cap);
                        let rb = r_dec(comp.as_ptr(), b.as_mut_ptr(), cs, t, cap);
                        assert_eq!(ra, rb, "partial {} sz={} t={} cap={}", gname, sz, t, cap);
                        beq!(a, b, "partial bytes {} sz={} t={} cap={}", gname, sz, t, cap);
                    }
                }
            }
        }
    }
}

#[test]
fn decompress_corrupt_inputs() {
    let (c_dec, r_dec) = pair!(
        "LZ4_decompress_safe",
        fn(*const u8, *mut u8, i32, i32) -> i32
    );
    let (c_part, r_part) = pair!(
        "LZ4_decompress_safe_partial",
        fn(*const u8, *mut u8, i32, i32, i32) -> i32
    );
    let mut rng = Rng::new(0xC0FFEE);
    unsafe {
        for (_, g) in GENS {
            for &sz in &[64usize, 1000, 5000] {
                let data = g(sz, 31337 + sz as u64);
                let base = c_compress(&data);
                for trial in 0..200 {
                    let mut comp = base.clone();
                    let nmut = 1 + (trial % 3);
                    for _ in 0..nmut {
                        let i = (rng.below(comp.len() as u32)) as usize;
                        comp[i] = (rng.next_u32() & 0xFF) as u8;
                    }
                    let cap = sz as i32 + 128;
                    let mut a = vec![0x22u8; cap as usize + 64];
                    let mut b = vec![0x22u8; cap as usize + 64];
                    let ra = c_dec(comp.as_ptr(), a.as_mut_ptr(), comp.len() as i32, cap);
                    let rb = r_dec(comp.as_ptr(), b.as_mut_ptr(), comp.len() as i32, cap);
                    assert_eq!(ra, rb, "corrupt decompress sz={} trial={}", sz, trial);
                    beq!(a, b, "corrupt decompress bytes sz={} trial={}", sz, trial);

                    let mut a = vec![0x22u8; cap as usize + 64];
                    let mut b = vec![0x22u8; cap as usize + 64];
                    let t = sz as i32 / 2;
                    let ra = c_part(comp.as_ptr(), a.as_mut_ptr(), comp.len() as i32, t, cap);
                    let rb = r_part(comp.as_ptr(), b.as_mut_ptr(), comp.len() as i32, t, cap);
                    assert_eq!(ra, rb, "corrupt partial sz={} trial={}", sz, trial);
                    beq!(a, b, "corrupt partial bytes sz={} trial={}", sz, trial);
                }
            }
        }
        // fully random "compressed" data
        for trial in 0..400 {
            let comp = gen_random(4 + (trial % 200), 777 + trial as u64);
            let cap = 4096i32;
            let mut a = vec![0x44u8; cap as usize + 64];
            let mut b = vec![0x44u8; cap as usize + 64];
            let ra = c_dec(comp.as_ptr(), a.as_mut_ptr(), comp.len() as i32, cap);
            let rb = r_dec(comp.as_ptr(), b.as_mut_ptr(), comp.len() as i32, cap);
            assert_eq!(ra, rb, "random input trial={}", trial);
            beq!(a, b, "random input bytes trial={}", trial);
        }
    }
}

#[test]
fn decompress_using_dict() {
    let (c_ud, r_ud) = pair!(
        "LZ4_decompress_safe_usingDict",
        fn(*const u8, *mut u8, i32, i32, *const u8, i32) -> i32
    );
    let (c_pud, r_pud) = pair!(
        "LZ4_decompress_safe_partial_usingDict",
        fn(*const u8, *mut u8, i32, i32, i32, *const u8, i32) -> i32
    );
    let (c_fed, r_fed) = pair!(
        "LZ4_decompress_safe_forceExtDict",
        fn(*const u8, *mut u8, i32, i32, *const u8, usize) -> i32
    );
    let (c_pfed, r_pfed) = pair!(
        "LZ4_decompress_safe_partial_forceExtDict",
        fn(*const u8, *mut u8, i32, i32, i32, *const u8, usize) -> i32
    );
    let (c_p64, r_p64) = pair!(
        "LZ4_decompress_safe_withPrefix64k",
        fn(*const u8, *mut u8, i32, i32) -> i32
    );

    // Produce dictionary-compressed blocks using the C streaming compressor.
    let (c_new, _) = pair!("LZ4_createStream", fn() -> *mut u8);
    let (c_freestream, _) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
    let (c_load, _) = pair!("LZ4_loadDict", fn(*mut u8, *const u8, i32) -> i32);
    let (c_cont, _) = pair!(
        "LZ4_compress_fast_continue",
        fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
    );
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);

    unsafe {
        for (gname, g) in GENS {
            for &dsz in &[0usize, 1, 100, 1000, 65536, 70000] {
                for &sz in &[1usize, 13, 100, 1000, 4096, 20000] {
                    let dict = g(dsz, 1000 + dsz as u64);
                    let data = g(sz, 2000 + sz as u64);
                    let st = c_new();
                    c_load(st, dict.as_ptr(), dict.len() as i32);
                    let bound = cbound(sz as i32);
                    let mut comp = vec![0u8; bound as usize];
                    let n = c_cont(st, data.as_ptr(), comp.as_mut_ptr(), sz as i32, bound, 1);
                    assert!(n > 0);
                    c_freestream(st);
                    comp.truncate(n as usize);

                    let cap = sz as i32 + 64;
                    for &(dp, dl) in &[
                        (dict.as_ptr(), dict.len() as i32),
                        (std::ptr::null(), 0i32),
                        (dict.as_ptr(), 0i32),
                    ] {
                        let mut a = vec![0x66u8; cap as usize + 64];
                        let mut b = vec![0x66u8; cap as usize + 64];
                        let ra = c_ud(comp.as_ptr(), a.as_mut_ptr(), n, cap, dp, dl);
                        let rb = r_ud(comp.as_ptr(), b.as_mut_ptr(), n, cap, dp, dl);
                        assert_eq!(
                            ra, rb,
                            "usingDict {} dsz={} sz={} dl={}",
                            gname, dsz, sz, dl
                        );
                        beq!(a, b, "usingDict bytes {} dsz={} sz={}", gname, dsz, sz);
                        if dl == dict.len() as i32 && dl > 0 {
                            assert_eq!(ra, sz as i32);
                            assert_eq!(&a[..sz], &data[..]);
                        }

                        for &t in &[0i32, 1, 17, sz as i32 / 2, sz as i32, sz as i32 + 5] {
                            let mut a = vec![0x66u8; cap as usize + 64];
                            let mut b = vec![0x66u8; cap as usize + 64];
                            let ra = c_pud(comp.as_ptr(), a.as_mut_ptr(), n, t, cap, dp, dl);
                            let rb = r_pud(comp.as_ptr(), b.as_mut_ptr(), n, t, cap, dp, dl);
                            assert_eq!(ra, rb, "partial_usingDict {} dsz={} t={}", gname, dsz, t);
                            beq!(a, b, "partial_usingDict bytes {} dsz={}", gname, dsz);
                        }

                        // forceExtDict variants take size_t dictSize
                        let mut a = vec![0x66u8; cap as usize + 64];
                        let mut b = vec![0x66u8; cap as usize + 64];
                        let ra = c_fed(comp.as_ptr(), a.as_mut_ptr(), n, cap, dp, dl as usize);
                        let rb = r_fed(comp.as_ptr(), b.as_mut_ptr(), n, cap, dp, dl as usize);
                        assert_eq!(ra, rb, "forceExtDict {} dsz={} sz={}", gname, dsz, sz);
                        beq!(a, b, "forceExtDict bytes {} dsz={} sz={}", gname, dsz, sz);

                        let mut a = vec![0x66u8; cap as usize + 64];
                        let mut b = vec![0x66u8; cap as usize + 64];
                        let t = sz as i32 / 3;
                        let ra = c_pfed(comp.as_ptr(), a.as_mut_ptr(), n, t, cap, dp, dl as usize);
                        let rb = r_pfed(comp.as_ptr(), b.as_mut_ptr(), n, t, cap, dp, dl as usize);
                        assert_eq!(ra, rb, "partial_forceExtDict {} dsz={}", gname, dsz);
                        beq!(a, b, "partial_forceExtDict bytes {} dsz={}", gname, dsz);
                    }
                }
            }
        }

        // withPrefix64k: dictionary is contiguous with dst
        for (gname, g) in GENS {
            for &sz in &[1usize, 100, 1000, 20000] {
                let dsz = 65536usize;
                let dict = g(dsz, 909);
                let data = g(sz, 808 + sz as u64);
                let st = c_new();
                c_load(st, dict.as_ptr(), dsz as i32);
                let bound = cbound(sz as i32);
                let mut comp = vec![0u8; bound as usize];
                let n = c_cont(st, data.as_ptr(), comp.as_mut_ptr(), sz as i32, bound, 1);
                c_freestream(st);
                assert!(n > 0);

                let cap = sz as i32 + 64;
                let mut a = vec![0x88u8; dsz + cap as usize + 64];
                let mut b = vec![0x88u8; dsz + cap as usize + 64];
                a[..dsz].copy_from_slice(&dict);
                b[..dsz].copy_from_slice(&dict);
                let ra = c_p64(comp.as_ptr(), a[dsz..].as_mut_ptr(), n, cap);
                let rb = r_p64(comp.as_ptr(), b[dsz..].as_mut_ptr(), n, cap);
                assert_eq!(ra, rb, "withPrefix64k {} sz={}", gname, sz);
                beq!(a, b, "withPrefix64k bytes {} sz={}", gname, sz);
                assert_eq!(ra, sz as i32);
                assert_eq!(&a[dsz..dsz + sz], &data[..]);
            }
        }
    }
}

#[test]
fn deprecated_fast_decompress() {
    let (c_df, r_df) = pair!("LZ4_decompress_fast", fn(*const u8, *mut u8, i32) -> i32);
    let (c_dfd, r_dfd) = pair!(
        "LZ4_decompress_fast_usingDict",
        fn(*const u8, *mut u8, i32, *const u8, i32) -> i32
    );
    let (c_dfp, r_dfp) = pair!(
        "LZ4_decompress_fast_withPrefix64k",
        fn(*const u8, *mut u8, i32) -> i32
    );
    let (c_unc, r_unc) = pair!("LZ4_uncompress", fn(*const u8, *mut u8, i32) -> i32);
    let (c_unc2, r_unc2) = pair!(
        "LZ4_uncompress_unknownOutputSize",
        fn(*const u8, *mut u8, i32, i32) -> i32
    );
    unsafe {
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 13, 100, 1000, 4096, 20000, 65536] {
                let data = g(sz, 4321 + sz as u64);
                let comp = c_compress(&data);
                let mut a = vec![0x99u8; sz + 128];
                let mut b = vec![0x99u8; sz + 128];
                let ra = c_df(comp.as_ptr(), a.as_mut_ptr(), sz as i32);
                let rb = r_df(comp.as_ptr(), b.as_mut_ptr(), sz as i32);
                assert_eq!(ra, rb, "decompress_fast {} sz={}", gname, sz);
                beq!(a, b, "decompress_fast bytes {} sz={}", gname, sz);

                let mut a = vec![0x99u8; sz + 128];
                let mut b = vec![0x99u8; sz + 128];
                let ra = c_unc(comp.as_ptr(), a.as_mut_ptr(), sz as i32);
                let rb = r_unc(comp.as_ptr(), b.as_mut_ptr(), sz as i32);
                assert_eq!(ra, rb, "LZ4_uncompress {} sz={}", gname, sz);
                beq!(a, b);

                let mut a = vec![0x99u8; sz + 128];
                let mut b = vec![0x99u8; sz + 128];
                let ra = c_unc2(
                    comp.as_ptr(),
                    a.as_mut_ptr(),
                    comp.len() as i32,
                    sz as i32 + 64,
                );
                let rb = r_unc2(
                    comp.as_ptr(),
                    b.as_mut_ptr(),
                    comp.len() as i32,
                    sz as i32 + 64,
                );
                assert_eq!(ra, rb, "uncompress_unknownOutputSize {} sz={}", gname, sz);
                beq!(a, b);

                // usingDict with empty dict is equivalent to plain fast decode
                let mut a = vec![0x99u8; sz + 128];
                let mut b = vec![0x99u8; sz + 128];
                let ra = c_dfd(
                    comp.as_ptr(),
                    a.as_mut_ptr(),
                    sz as i32,
                    std::ptr::null(),
                    0,
                );
                let rb = r_dfd(
                    comp.as_ptr(),
                    b.as_mut_ptr(),
                    sz as i32,
                    std::ptr::null(),
                    0,
                );
                assert_eq!(ra, rb, "fast_usingDict {} sz={}", gname, sz);
                beq!(a, b);
            }
        }

        // fast_withPrefix64k / fast_usingDict with a real dictionary
        let (c_new, _) = pair!("LZ4_createStream", fn() -> *mut u8);
        let (c_freestream, _) = pair!("LZ4_freeStream", fn(*mut u8) -> i32);
        let (c_load, _) = pair!("LZ4_loadDict", fn(*mut u8, *const u8, i32) -> i32);
        let (c_cont, _) = pair!(
            "LZ4_compress_fast_continue",
            fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32
        );
        let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
        for (_, g) in GENS {
            for &sz in &[1usize, 100, 5000] {
                let dsz = 65536usize;
                let dict = g(dsz, 4545);
                let data = g(sz, 5656 + sz as u64);
                let st = c_new();
                c_load(st, dict.as_ptr(), dsz as i32);
                let bound = cbound(sz as i32);
                let mut comp = vec![0u8; bound as usize];
                let n = c_cont(st, data.as_ptr(), comp.as_mut_ptr(), sz as i32, bound, 1);
                c_freestream(st);
                assert!(n > 0);

                let mut a = vec![0xAAu8; dsz + sz + 128];
                let mut b = vec![0xAAu8; dsz + sz + 128];
                a[..dsz].copy_from_slice(&dict);
                b[..dsz].copy_from_slice(&dict);
                let ra = c_dfp(comp.as_ptr(), a[dsz..].as_mut_ptr(), sz as i32);
                let rb = r_dfp(comp.as_ptr(), b[dsz..].as_mut_ptr(), sz as i32);
                assert_eq!(ra, rb, "fast_withPrefix64k sz={}", sz);
                beq!(a, b);

                let mut a = vec![0xAAu8; sz + 128];
                let mut b = vec![0xAAu8; sz + 128];
                let ra = c_dfd(
                    comp.as_ptr(),
                    a.as_mut_ptr(),
                    sz as i32,
                    dict.as_ptr(),
                    dsz as i32,
                );
                let rb = r_dfd(
                    comp.as_ptr(),
                    b.as_mut_ptr(),
                    sz as i32,
                    dict.as_ptr(),
                    dsz as i32,
                );
                assert_eq!(ra, rb, "fast_usingDict dict sz={}", sz);
                beq!(a, b);
            }
        }
    }
}

#[test]
fn deprecated_compress_wrappers() {
    let (c_size, _) = pair!("LZ4_sizeofState", fn() -> i32);
    let ssz = unsafe { c_size() } as usize;
    let (cbound, _) = pair!("LZ4_compressBound", fn(i32) -> i32);
    unsafe {
        let (c1, r1) = pair!("LZ4_compress", fn(*const u8, *mut u8, i32) -> i32);
        let (c2, r2) = pair!(
            "LZ4_compress_limitedOutput",
            fn(*const u8, *mut u8, i32, i32) -> i32
        );
        let (c3, r3) = pair!(
            "LZ4_compress_withState",
            fn(*mut u8, *const u8, *mut u8, i32) -> i32
        );
        let (c4, r4) = pair!(
            "LZ4_compress_limitedOutput_withState",
            fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32
        );
        let mut cs = Aligned::new(ssz);
        let mut rs = Aligned::new(ssz);
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 13, 100, 1000, 4096, 20000] {
                let data = g(sz, 191 + sz as u64);
                let bound = cbound(sz as i32).max(16);
                let mut a = vec![0u8; bound as usize + 32];
                let mut b = vec![0u8; bound as usize + 32];
                assert_eq!(
                    c1(data.as_ptr(), a.as_mut_ptr(), sz as i32),
                    r1(data.as_ptr(), b.as_mut_ptr(), sz as i32),
                    "LZ4_compress {} sz={}",
                    gname,
                    sz
                );
                beq!(a, b);

                for &cap in &[0i32, 1, 16, bound / 2, bound] {
                    let n = cap.max(0) as usize + 64;
                    let mut a = vec![0u8; n];
                    let mut b = vec![0u8; n];
                    assert_eq!(
                        c2(data.as_ptr(), a.as_mut_ptr(), sz as i32, cap),
                        r2(data.as_ptr(), b.as_mut_ptr(), sz as i32, cap),
                        "limitedOutput {} sz={} cap={}",
                        gname,
                        sz,
                        cap
                    );
                    beq!(a, b);
                }

                cs.zero();
                rs.zero();
                let mut a = vec![0u8; bound as usize + 32];
                let mut b = vec![0u8; bound as usize + 32];
                assert_eq!(
                    c3(cs.ptr(), data.as_ptr(), a.as_mut_ptr(), sz as i32),
                    r3(rs.ptr(), data.as_ptr(), b.as_mut_ptr(), sz as i32),
                    "withState {} sz={}",
                    gname,
                    sz
                );
                beq!(a, b);
                beq!(cs.as_slice(), rs.as_slice());

                cs.zero();
                rs.zero();
                let mut a = vec![0u8; bound as usize + 32];
                let mut b = vec![0u8; bound as usize + 32];
                assert_eq!(
                    c4(cs.ptr(), data.as_ptr(), a.as_mut_ptr(), sz as i32, bound),
                    r4(rs.ptr(), data.as_ptr(), b.as_mut_ptr(), sz as i32, bound),
                    "limitedOutput_withState {} sz={}",
                    gname,
                    sz
                );
                beq!(a, b);
                beq!(cs.as_slice(), rs.as_slice());
            }
        }
    }
}
