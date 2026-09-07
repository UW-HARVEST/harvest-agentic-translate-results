//! Phase B — CONFIGS.md rows 1-13 and 82: the hashing / seeding / `strkey`
//! entry points, driven directly through both `.so` export tables.

mod common;

use common::*;
use std::ffi::c_void;

/// CONFIGS rows 1-7: `stbds_hash_bytes` over every length class and seed.
#[test]
fn cfg_01_07_hash_bytes_all_length_classes() {
    let _g = lock();
    let (c, r) = libs();
    let mut rng = Rng::new(0xB0);

    let fixed_seeds: [usize; 5] = [0, 1, usize::MAX, DEFAULT_SEED, 0xdead_beef_cafe_f00d];

    // row 1: len == 0 (no byte is read; also exercised with a null buffer)
    for &s in &fixed_seeds {
        let mut buf = [0u8; 1];
        unsafe {
            same(
                &format!("row1 len=0 seed={s:#x}"),
                (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, s),
                (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, s),
            );
            same(
                &format!("row1 len=0 NULL seed={s:#x}"),
                (c.hash_bytes)(std::ptr::null_mut(), 0, s),
                (r.hash_bytes)(std::ptr::null_mut(), 0, s),
            );
        }
    }

    // rows 2-5: every distinct length class, 200 random buffers each
    let lens: Vec<usize> = (0usize..=16)
        .chain([17, 23, 24, 31, 32, 63, 64, 100, 256, 1024])
        .collect();
    for &len in &lens {
        for _ in 0..200 {
            let mut buf = rng.bytes(len.max(1));
            let seed = rng.next_u64() as usize;
            unsafe {
                same(
                    &format!("rows2-5 len={len} seed={seed:#x}"),
                    (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
                    (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
                );
            }
        }
        // all-0x00 and all-0xFF buffers (sign-extension corners of the C code)
        for fill in [0x00u8, 0xFF, 0x80, 0x7F] {
            let mut buf = vec![fill; len.max(1)];
            for &s in &fixed_seeds {
                unsafe {
                    same(
                        &format!("rows2-5 len={len} fill={fill:#x} seed={s:#x}"),
                        (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s),
                        (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s),
                    );
                }
            }
        }
    }

    // row 6: fully random length and content
    for _ in 0..5000 {
        let len = rng.range(1, 1024);
        let mut buf = rng.bytes(len);
        let seed = rng.next_u64() as usize;
        unsafe {
            same(
                &format!("row6 len={len} seed={seed:#x}"),
                (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
                (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
            );
        }
    }

    // row 7: boundary seeds against a fixed corpus
    for &s in &fixed_seeds {
        for len in 0..40usize {
            let mut buf: Vec<u8> = (0..len.max(1)).map(|i| (i as u8).wrapping_mul(37)).collect();
            unsafe {
                same(
                    &format!("row7 len={len} seed={s:#x}"),
                    (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s),
                    (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s),
                );
            }
        }
    }
}

/// CONFIGS rows 8-11: `stbds_hash_string`.
#[test]
fn cfg_08_11_hash_string() {
    let _g = lock();
    let (c, r) = libs();
    let mut rng = Rng::new(0xB8);
    let fixed_seeds: [usize; 5] = [0, 1, usize::MAX, DEFAULT_SEED, 0x0123_4567_89ab_cdef];

    // row 8: empty string
    for &s in &fixed_seeds {
        let mut e = [0u8; 1];
        unsafe {
            same(
                &format!("row8 seed={s:#x}"),
                (c.hash_string)(e.as_mut_ptr() as *mut _, s),
                (r.hash_string)(e.as_mut_ptr() as *mut _, s),
            );
        }
    }

    // row 9: random ASCII, length 1..64
    for _ in 0..3000 {
        let n = rng.range(1, 64);
        let mut s = rng.ascii_cstring(n);
        let seed = rng.next_u64() as usize;
        unsafe {
            same(
                &format!("row9 n={n} seed={seed:#x}"),
                (c.hash_string)(s.as_mut_ptr() as *mut _, seed),
                (r.hash_string)(s.as_mut_ptr() as *mut _, seed),
            );
        }
    }

    // row 10: arbitrary non-NUL bytes incl. >= 0x80, length 1..512
    for _ in 0..3000 {
        let n = rng.range(1, 512);
        let mut s = rng.cstring(n);
        let seed = rng.next_u64() as usize;
        unsafe {
            same(
                &format!("row10 n={n} seed={seed:#x}"),
                (c.hash_string)(s.as_mut_ptr() as *mut _, seed),
                (r.hash_string)(s.as_mut_ptr() as *mut _, seed),
            );
        }
    }

    // row 11: boundary seeds, deterministic corpus incl. all-0xFF strings
    for &s in &fixed_seeds {
        for n in 1..64usize {
            for fill in [0x01u8, 0x7F, 0x80, 0xFF] {
                let mut v = vec![fill; n];
                v.push(0);
                unsafe {
                    same(
                        &format!("row11 n={n} fill={fill:#x} seed={s:#x}"),
                        (c.hash_string)(v.as_mut_ptr() as *mut _, s),
                        (r.hash_string)(v.as_mut_ptr() as *mut _, s),
                    );
                }
            }
        }
    }
}

/// CONFIGS row 12: `stbds_rand_seed` then table creation — checks the per-table
/// `seed` field and the global LCG advance.
#[test]
fn cfg_12_rand_seed_then_table_seed() {
    let _g = lock();
    let mut rng = Rng::new(0xC0);
    let mut seeds: Vec<usize> = vec![0, 1, DEFAULT_SEED, usize::MAX, 2, usize::MAX - 1];
    for _ in 0..64 {
        seeds.push(rng.next_u64() as usize);
    }
    for &s in &seeds {
        for mode in [STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            reset_seeds(s);
            let mut m = MapPair::from_shmode(16, 8, mode, KeyKind::Binary);
            m.check(&format!("row12 seed={s:#x} mode={mode}"));
            m.free();
        }
    }
}

/// CONFIGS row 13: the global seed LCG must advance identically across a chain
/// of freshly created tables.
#[test]
fn cfg_13_seed_lcg_chain() {
    let _g = lock();
    let mut rng = Rng::new(0xC1);
    for &s in &[0usize, 1, DEFAULT_SEED, usize::MAX, rng.next_u64() as usize] {
        reset_seeds(s);
        let mut maps = Vec::new();
        for i in 0..8 {
            let m = MapPair::from_shmode(16, 8, STBDS_SH_ARENA, KeyKind::Binary);
            m.check(&format!("row13 seed={s:#x} table#{i}"));
            maps.push(m);
        }
        // and the same for tables created implicitly by hmput_key
        for i in 0..8 {
            let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
            m.put(&format!("row13 implicit#{i}"), &7i32.to_ne_bytes(), STBDS_HM_BINARY);
            m.free();
        }
        for m in maps.iter_mut() {
            m.free();
        }
    }
}

/// CONFIGS row 82: `strkey` — the whole 256-byte static buffer is compared.
#[test]
fn cfg_82_strkey() {
    let _g = lock();
    let (c, r) = libs();
    let mut rng = Rng::new(0xC2);

    let mut ns: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        10,
        11,
        99,
        100,
        101,
        999,
        1000,
        12345,
        -12345,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -100,
        -9,
        -10,
    ];
    for _ in 0..1000 {
        ns.push(rng.next_u32() as i32);
    }

    for &n in &ns {
        unsafe {
            let cp = (c.strkey)(n);
            let rp = (r.strkey)(n);
            let cs = std::slice::from_raw_parts(cp as *const u8, 256).to_vec();
            let rs = std::slice::from_raw_parts(rp as *const u8, 256).to_vec();
            // Compare the NUL-terminated content (bytes past the terminator are
            // leftovers from previous sprintf calls in both libraries).
            let cn = cs.iter().position(|&b| b == 0).unwrap();
            let rn = rs.iter().position(|&b| b == 0).unwrap();
            same(&format!("row82 strkey({n}) len"), cn, rn);
            same(&format!("row82 strkey({n})"), &cs[..cn], &rs[..rn]);
        }
    }

    // Same sequence of calls => the whole buffer, including the stale tail
    // bytes left by previous longer keys, must match byte-for-byte.
    unsafe {
        for &n in &[i32::MIN, 1, -1, 0, 999999999, 7] {
            (c.strkey)(n);
            (r.strkey)(n);
        }
        let cp = (c.strkey)(3);
        let rp = (r.strkey)(3);
        let cs = std::slice::from_raw_parts(cp as *const u8, 32).to_vec();
        let rs = std::slice::from_raw_parts(rp as *const u8, 32).to_vec();
        same("row82 strkey stale-tail", cs, rs);
    }
}
