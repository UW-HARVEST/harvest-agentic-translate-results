//! Phase B — CONFIGS.md rows 49..51: `strkey`, `intput` and global-seed
//! lock-step.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

#[track_caller]
fn strkey_both(p: &Pair, n: c_int) {
    let (sc, sr) = unsafe {
        (
            cstr_bytes((p.c.strkey)(n)),
            cstr_bytes((p.r.strkey)(n)),
        )
    };
    assert_eq!(
        sc, sr,
        "strkey({n}) diverged: C={:?} Rust={:?}",
        String::from_utf8_lossy(&sc),
        String::from_utf8_lossy(&sr)
    );
    // sprintf(buffer, "test_%d", n)
    assert_eq!(
        String::from_utf8_lossy(&sc),
        format!("test_{n}"),
        "strkey({n}) is not `test_%d`"
    );
}

/// Row 49 — `strkey` over hand-picked extremes plus 4096 random `int`s.
#[test]
fn cfg_49_strkey() {
    let p = Pair::new();
    for n in [
        0,
        1,
        -1,
        7,
        -7,
        9,
        10,
        11,
        99,
        -99,
        100,
        12345,
        -12345,
        1_000_000_000,
        -1_000_000_000,
        c_int::MAX,
        c_int::MIN,
        c_int::MIN + 1,
    ] {
        strkey_both(&p, n);
    }
    for n in -300..=300 {
        strkey_both(&p, n);
    }
    // every decimal digit count 1..10, both signs, at both ends of each decade
    let mut pow = 1i64;
    for _ in 0..10 {
        for delta in [-1i64, 0, 1] {
            let v = pow + delta;
            if v >= 1 && v <= i32::MAX as i64 {
                strkey_both(&p, v as c_int);
                strkey_both(&p, -(v as c_int));
            }
        }
        pow *= 10;
    }
    // ...and a dense sweep of every magnitude scale
    let mut rng = Rng::new(0xC0FFEE_49);
    for k in 0..10u32 {
        let hi = 10i64.pow(k).min(i32::MAX as i64) as u64;
        for _ in 0..256 {
            let m = (rng.next_u64() % hi.max(1)) as i64;
            strkey_both(&p, m as c_int);
            strkey_both(&p, -(m as c_int));
        }
    }
    for _ in 0..4096 {
        strkey_both(&p, rng.i32v());
    }
}

/// Read the global `stbds_hash_seed` of one library by creating a fresh table
/// (`stbds_make_hash_index(_, NULL)` copies the global into `t->seed` and then
/// advances the global).
unsafe fn probe_seed(l: &Lib) -> (usize, usize) {
    let h = (l.shmode_func)(16, STBDS_SH_NONE);
    let raw = (h as *mut u8).sub(16) as *mut ArrayHeader;
    let t = (*raw.sub(1)).hash_table as *mut HashIndex;
    let seed = (*t).seed;
    let slot_count = (*t).slot_count;
    (l.hmfree_func)(raw.sub(1).add(1) as *mut c_void, 16);
    (seed, slot_count)
}

/// Row 50 — `intput` must complete without aborting for every `num` except
/// 9 and 11 (those two are `ERRORS.md` #44/#45), and must advance the global
/// hash seed identically on both sides.
#[test]
fn cfg_50_intput() {
    let p = Pair::new();

    let mut cases: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        3,
        7,
        8,
        10,
        12,
        13,
        100,
        -100,
        c_int::MAX,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MAX - 1,
    ];
    let mut rng = Rng::new(0xC0FFEE_50);
    while cases.len() < 300 {
        let v = rng.i32v();
        if v != 9 && v != 11 {
            cases.push(v);
        }
    }

    for &gseed in &[0usize, 1, 0x3141_5926, usize::MAX] {
        p.seed(gseed);
        for (i, &num) in cases.iter().enumerate() {
            unsafe {
                (p.c.intput)(num);
                (p.r.intput)(num);
                // both globals must have advanced by the same amount: intput
                // creates exactly one fresh hash index per call
                let (s1, c1) = probe_seed(p.c);
                let (s2, c2) = probe_seed(p.r);
                assert_eq!(
                    s1, s2,
                    "global stbds_hash_seed diverged after intput({num}) \
                     (case {i}, gseed={gseed:#x})"
                );
                assert_eq!(c1, c2);
            }
        }
    }
}

/// Row 51 — global-seed lock-step across a long mixed op-stream: every table
/// created must carry the same `seed`, whether fresh (global) or inherited.
#[test]
fn cfg_51_seed_lockstep() {
    let p = Pair::new();
    for &gseed in &[0usize, 1, 2, 0x3141_5926, usize::MAX, 0x8000_0000_0000_0000] {
        p.seed(gseed);
        let mut rng = Rng::new(0xC0FFEE_51 ^ gseed as u64);

        // interleave: fresh tables (advance the global), map growth (inherits),
        // map shrink/rebuild (inherits), intput (advances), shmode_func
        // (advances).
        let mut maps: Vec<MapPair> = Vec::new();
        for step in 0..300u64 {
            match rng.below(6) {
                0 => {
                    let m = MapPair::shmode(&p, 16, 8, STBDS_HM_BINARY, STBDS_SH_NONE, KeyKind::Binary);
                    m.check(&format!("step {step}: shmode_func table"));
                    maps.push(m);
                }
                1 => unsafe {
                    (p.c.intput)(5);
                    (p.r.intput)(5);
                    let (s1, _) = probe_seed(p.c);
                    let (s2, _) = probe_seed(p.r);
                    assert_eq!(s1, s2, "step {step}: seed diverged after intput");
                },
                2 => {
                    let mut m = MapPair::null(16, 8, STBDS_HM_BINARY, KeyKind::Binary);
                    for i in 0..1 + rng.below(60) {
                        let mut k = (i as u64).wrapping_mul(0x9E37_79B9).to_le_bytes().to_vec();
                        m.put(&p, &mut k, &(i as u64).to_le_bytes());
                    }
                    m.check(&format!("step {step}: grown map"));
                    maps.push(m);
                }
                3 => {
                    if let Some(m) = maps.last_mut() {
                        for i in 0..1 + rng.below(40) {
                            let mut k = (i as u64).wrapping_mul(0x9E37_79B9).to_le_bytes().to_vec();
                            m.put(&p, &mut k, &(i as u64 ^ step).to_le_bytes());
                        }
                        m.check(&format!("step {step}: extended map"));
                    }
                }
                4 => {
                    if let Some(m) = maps.last_mut() {
                        for i in 0..1 + rng.below(40) {
                            let mut k = (i as u64).wrapping_mul(0x9E37_79B9).to_le_bytes().to_vec();
                            m.del(&p, &mut k, 0);
                        }
                        m.check(&format!("step {step}: shrunk map"));
                    }
                }
                _ => {
                    if !maps.is_empty() {
                        let i = rng.below(maps.len());
                        let mut m = maps.remove(i);
                        m.check(&format!("step {step}: before free"));
                        m.free(&p);
                    }
                }
            }
            // all live maps must still agree
            for (i, m) in maps.iter().enumerate() {
                m.check(&format!("step {step}: live map {i}"));
            }
        }
        for m in maps.iter_mut() {
            m.free(&p);
        }
    }
}

/// `intput` is idempotent w.r.t. its own map (it allocates a fresh one each
/// call and leaks it), so calling it many times in a row must stay in lock-step.
#[test]
fn cfg_50b_intput_repeated() {
    let p = Pair::new();
    p.seed(0x3141_5926);
    for i in 0..500 {
        let num = if i % 3 == 0 { 0 } else { i + 100 };
        unsafe {
            (p.c.intput)(num);
            (p.r.intput)(num);
        }
    }
    unsafe {
        let (s1, _) = probe_seed(p.c);
        let (s2, _) = probe_seed(p.r);
        assert_eq!(s1, s2, "seed diverged after 500 intput calls");
    }
    let _: *const c_char = std::ptr::null();
}
