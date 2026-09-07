//! Phase B — CONFIGS.md rows 59-63: `strkey`, `arr_ins`, and end-to-end
//! pipelines that compose the low-level entry points the way a real consumer
//! (the `stbds_*` macros) does.

mod common;
use common::*;
use std::ffi::CStr;

#[test]
fn row59_strkey() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 59);
    let mut vals: Vec<i32> = vec![0, 1, -1, 9, 10, 99, 100, i32::MIN, i32::MAX, i32::MIN + 1];
    for _ in 0..500 {
        vals.push(rng.next_i32());
    }
    for &n in &vals {
        unsafe {
            let a = CStr::from_ptr((p.c.strkey)(n)).to_bytes().to_vec();
            let b = CStr::from_ptr((p.rs.strkey)(n)).to_bytes().to_vec();
            assert_eq!(a, b, "strkey({n})");
            assert_eq!(a, format!("test_{n}").into_bytes(), "strkey({n}) contents");
        }
    }
    // the static buffer must be reused (the C returns the same address every
    // time), so a second call must overwrite the first result in both libs
    unsafe {
        let p1 = (p.c.strkey)(1);
        let p2 = (p.c.strkey)(22222);
        assert_eq!(p1, p2, "C strkey must return its static buffer");
        let r1 = (p.rs.strkey)(1);
        let r2 = (p.rs.strkey)(22222);
        assert_eq!(r1, r2, "Rust strkey must return its static buffer");
        assert_eq!(
            CStr::from_ptr(p1).to_bytes(),
            CStr::from_ptr(r1).to_bytes(),
            "aliased strkey buffer contents"
        );
    }
}

#[test]
fn row60_arr_ins() {
    // `arr_ins` is void and only observable through its internal asserts:
    // if either library's assertions fail the process aborts and the test
    // fails.  Both are driven over the whole int range.
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 60);
    let mut vals: Vec<i32> = vec![0, 1, 2, 3, 4, 5, -1, i32::MIN, i32::MAX];
    for _ in 0..500 {
        vals.push(rng.next_i32());
    }
    for &n in &vals {
        unsafe {
            (p.c.arr_ins)(n);
            (p.rs.arr_ins)(n);
        }
    }
}

fn i32k(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}

fn keyset(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    while v.len() < n {
        let len = 1 + rng.below(24);
        let k = rng.ascii(len);
        if !v.contains(&k) {
            v.push(k);
        }
    }
    v
}

/// Rows 61 and 63: full `sh_new_strdup` pipeline with state compared at every
/// single step, under both the default and a randomized global seed.
#[test]
fn row61_63_strdup_pipeline() {
    let (p, _g) = libs();
    let mut outer = Rng::new(0xC0FFEE ^ 61);
    let mut seeds = vec![DEFAULT_SEED, 0usize, usize::MAX];
    for _ in 0..5 {
        seeds.push(outer.next_u64() as usize);
    }
    for &gseed in &seeds {
        let mut rng = Rng::new(0xC0FFEE ^ 61 ^ gseed as u64);
        reset_seed(p, gseed);
        let mut m = Maps::shmode(p, cfg_string(), SH_STRDUP);
        let keys = keyset(&mut rng, 200);
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
        }
        for k in &keys {
            assert!(m.get(k) >= 0, "seed={gseed:#x} key missing");
            assert!(m.get_ts(k) >= 0);
        }
        // delete half
        let mut live = keys.clone();
        let mut gone = Vec::new();
        for _ in 0..100 {
            let k = live.remove(rng.below(live.len()));
            assert_eq!(m.del(&k), 1);
            gone.push(k);
        }
        for k in &gone {
            assert_eq!(m.get(k), -1);
        }
        for k in &live {
            assert!(m.get(k) >= 0);
        }
        // re-put the deleted half
        for k in &gone {
            m.put_string(k, &rng.bytes(8));
        }
        for k in &keys {
            assert!(m.get(k) >= 0);
        }
        m.free();
    }
}

/// Rows 62 and 63: full implicit BINARY `stbds_struct` pipeline.
#[test]
fn row62_63_binary_pipeline() {
    let (p, _g) = libs();
    let mut outer = Rng::new(0xC0FFEE ^ 62);
    let mut seeds = vec![DEFAULT_SEED, 1usize, usize::MAX];
    for _ in 0..5 {
        seeds.push(outer.next_u64() as usize);
    }
    for &gseed in &seeds {
        let mut rng = Rng::new(0xC0FFEE ^ 62 ^ gseed as u64);
        reset_seed(p, gseed);
        let mut m = Maps::empty(p, cfg_struct());
        let mut keys: Vec<i32> = Vec::new();
        while keys.len() < 300 {
            let k = rng.next_i32();
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, &k) in keys.iter().enumerate() {
            m.put_binary(&i32k(k), &rng.bytes(12));
            if i % 7 == 0 {
                // interleaved lookups while the table is still growing
                assert!(m.get(&i32k(k)) >= 0);
                assert!(m.get(&i32k(keys[i / 2])) >= 0);
            }
        }
        let mut live = keys.clone();
        let mut gone = Vec::new();
        for _ in 0..150 {
            let k = live.remove(rng.below(live.len()));
            assert_eq!(m.del(&i32k(k)), 1);
            gone.push(k);
        }
        for &k in &gone {
            assert_eq!(m.get(&i32k(k)), -1);
        }
        for &k in &live {
            assert!(m.get(&i32k(k)) >= 0);
        }
        for &k in &gone {
            m.put_binary(&i32k(k), &rng.bytes(12));
        }
        for &k in &keys {
            assert!(m.get(&i32k(k)) >= 0);
        }
        m.free();
    }
}

/// Row 63 variant: arena pipeline under a randomized global seed, mixing
/// `hmput_default` (a default value) with the map operations.
#[test]
fn row63_arena_pipeline_with_default() {
    let (p, _g) = libs();
    let mut outer = Rng::new(0xC0FFEE ^ 63);
    for t in 0..6u64 {
        let gseed = outer.next_u64() as usize;
        let mut rng = Rng::new(0xC0FFEE ^ 63 ^ t);
        reset_seed(p, gseed);
        let mut m = Maps::shmode(p, cfg_string(), SH_ARENA);
        // set a non-zero default value at element 0, as `stbds_hmdefault` does
        unsafe {
            for b in 8..16 {
                *(m.ch as *mut u8).sub(16).add(b) = (0x5A + b) as u8;
                *(m.rh as *mut u8).sub(16).add(b) = (0x5A + b) as u8;
            }
        }
        m.compare("after hmdefault");
        let keys = keyset(&mut rng, 250);
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
        }
        for k in &keys {
            assert!(m.get(k) >= 0);
        }
        let mut live = keys.clone();
        while live.len() > 30 {
            let k = live.remove(rng.below(live.len()));
            assert_eq!(m.del(&k), 1);
        }
        for k in &live {
            assert!(m.get(k) >= 0);
        }
        m.free();
    }
}
