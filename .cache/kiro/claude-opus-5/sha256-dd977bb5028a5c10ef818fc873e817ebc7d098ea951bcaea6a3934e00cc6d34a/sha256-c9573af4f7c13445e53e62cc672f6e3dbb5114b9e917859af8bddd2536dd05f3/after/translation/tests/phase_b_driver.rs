//! Phase B — CONFIGS.md rows 39..41: the top-level driver (`strkey`,
//! `str_dups`, incl. its `printf` output) and the seed axis over the full
//! map pipeline.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

/// Row 39 — `strkey(n)`: the `sprintf` into the static 256-byte buffer.
#[test]
fn cfg_39_strkey() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 39);
    let mut ns: Vec<c_int> = vec![
        0,
        1,
        2,
        9,
        10,
        99,
        100,
        999,
        1000,
        12345,
        -1,
        -9,
        -10,
        -12345,
        c_int::MIN,
        c_int::MAX,
        c_int::MIN + 1,
        c_int::MAX - 1,
    ];
    for _ in 0..200 {
        ns.push(rng.next_u64() as u32 as i32);
    }
    for n in ns {
        unsafe {
            let cp = (p.c.strkey)(n);
            let rp = (p.rs.strkey)(n);
            let cs = std::ffi::CStr::from_ptr(cp).to_bytes().to_vec();
            let rs = std::ffi::CStr::from_ptr(rp).to_bytes().to_vec();
            assert_eq!(cs, rs, "strkey({n})");
            assert_eq!(cs, format!("test_{n}").into_bytes(), "strkey({n}) content");
            // The buffer is reused: a second call must overwrite in place.
            let cp2 = (p.c.strkey)(n);
            assert_eq!(cp2, cp, "strkey must return the same static buffer");
            let rp2 = (p.rs.strkey)(n);
            assert_eq!(rp2, rp, "strkey must return the same static buffer");
        }
    }
}

/// Row 40 — `str_dups(num)`: full end-to-end driver, stdout compared byte-for-byte.
#[test]
fn cfg_40_str_dups() {
    let (p, _g) = begin(DEFAULT_SEED);
    for num in [0i32, 1, 2, 3, 8, 63, 64, 100, 500, 1000, -1, -100] {
        // Keep the two libraries' global hash seeds in lockstep: str_dups builds
        // one fresh hash index, which advances the seed LCG once.
        sync_seed(p, DEFAULT_SEED);
        let cout = capture_stdout("c", || unsafe { (p.c.str_dups)(num) });
        sync_seed(p, DEFAULT_SEED);
        let rout = capture_stdout("rs", || unsafe { (p.rs.str_dups)(num) });
        assert_eq!(
            cout,
            rout,
            "str_dups({num}) stdout differs:\n C: {:?}\n R: {:?}",
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout)
        );
        // The driver prints exactly one line: the strdup'd key and the value.
        assert_eq!(
            cout,
            format!("a {num}\n").into_bytes(),
            "unexpected str_dups({num}) output"
        );
    }
}

/// Row 40b — `str_dups` called repeatedly, so the seed LCG advances across calls
/// and the static `buffer` is reused.
#[test]
fn cfg_40b_str_dups_repeated() {
    let (p, _g) = begin(DEFAULT_SEED);
    sync_seed(p, DEFAULT_SEED);
    let cout = capture_stdout("c-rep", || unsafe {
        for n in 0..25i32 {
            (p.c.str_dups)(n * 7);
        }
    });
    sync_seed(p, DEFAULT_SEED);
    let rout = capture_stdout("rs-rep", || unsafe {
        for n in 0..25i32 {
            (p.rs.str_dups)(n * 7);
        }
    });
    assert_eq!(cout, rout, "repeated str_dups stdout differs");
    let expect: String = (0..25i32).map(|n| format!("a {}\n", n * 7)).collect();
    assert_eq!(cout, expect.into_bytes());
}

/// Row 41 — the seed axis applied to the whole map pipeline: probe order,
/// growth points and bucket contents are all seed dependent.
#[test]
fn cfg_41_seeded_pipeline() {
    let mut sr = Rng::new(SEED ^ 41);
    let mut seeds: Vec<usize> = vec![0, 1, 2, DEFAULT_SEED, usize::MAX, 1 << 63, 0xffff_ffff];
    for _ in 0..8 {
        seeds.push(sr.next_usize());
    }
    for &s0 in &seeds {
        let (p, _g) = begin(s0);
        let mut rng = Rng::new(SEED ^ (s0 as u64));
        let (e, ks) = (16usize, 8usize);
        unsafe {
            let mut cm = Map::empty(&p.c, e);
            let mut rm = Map::empty(&p.rs, e);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for step in 0..500usize {
                let op = rng.below(10);
                if op < 6 || keys.is_empty() {
                    let mut k = rng.bytes(ks);
                    while keys.iter().any(|x| *x == k) {
                        k = rng.bytes(ks);
                    }
                    let v = rng.bytes(8);
                    let tc = cm.put_kv(k.as_mut_ptr() as *mut c_void, ks, STBDS_HM_BINARY, 8, &v);
                    let tr = rm.put_kv(k.as_mut_ptr() as *mut c_void, ks, STBDS_HM_BINARY, 8, &v);
                    assert_eq!(tc, tr, "seed={s0:#x} step={step} put temp");
                    keys.push(k);
                } else if op < 8 {
                    let j = rng.below(keys.len());
                    let mut k = keys.swap_remove(j);
                    let tc = cm.del(k.as_mut_ptr() as *mut c_void, ks, 0, STBDS_HM_BINARY);
                    let tr = rm.del(k.as_mut_ptr() as *mut c_void, ks, 0, STBDS_HM_BINARY);
                    assert_eq!(tc, tr, "seed={s0:#x} step={step} del temp");
                } else {
                    let mut k = rng.bytes(ks);
                    let tc = cm.get(k.as_mut_ptr() as *mut c_void, ks, STBDS_HM_BINARY);
                    let tr = rm.get(k.as_mut_ptr() as *mut c_void, ks, STBDS_HM_BINARY);
                    assert_eq!(tc, tr, "seed={s0:#x} step={step} get temp");
                }
                assert_eq!(
                    cm.dump(false),
                    rm.dump(false),
                    "seed={s0:#x} step={step} state"
                );
            }
            cm.free();
            rm.free();
        }
    }
}
