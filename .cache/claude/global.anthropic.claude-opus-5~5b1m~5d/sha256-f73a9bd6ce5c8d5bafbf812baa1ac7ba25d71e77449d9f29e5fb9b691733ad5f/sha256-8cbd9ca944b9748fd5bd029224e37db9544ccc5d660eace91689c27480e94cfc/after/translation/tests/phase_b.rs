// Phase B — valid-path differential tests, one test per CONFIGS.md row.
//
// Every call goes through the two shared objects via libloading.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;

const DEFAULT_SEED: usize = 0x31415926;

// ===========================================================================
// Row 1 / 2 — stbds_hash_bytes over every length class and byte pattern
// ===========================================================================

#[test]
fn row01_hash_bytes_lengths_and_seeds() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xB0001);
    let lens = [
        0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 16, 17, 23, 24, 31, 32, 33, 64, 127,
    ];
    let seeds = [0usize, 1, DEFAULT_SEED, usize::MAX, 0xDEADBEEFCAFEBABE];
    for &len in &lens {
        for &seed in &seeds {
            for _iter in 0..40 {
                let mut buf = rng.bytes(len.max(1));
                unsafe {
                    let hc = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    let hr = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    diff("1", &format!("hash_bytes(len={len},seed={seed:#x})"), hc, hr);
                }
            }
        }
    }
}

#[test]
fn row02_hash_bytes_extreme_byte_patterns() {
    let _g = guard();
    let p = pair();
    let patterns: Vec<Vec<u8>> = {
        let mut v = Vec::new();
        for len in 0..40usize {
            v.push(vec![0x00u8; len.max(1)]);
            v.push(vec![0xFFu8; len.max(1)]);
            v.push(vec![0x80u8; len.max(1)]);
            v.push((0..len.max(1)).map(|i| (i as u8) | 0x80).collect());
            v.push((0..len.max(1)).map(|i| 0x7F ^ (i as u8)).collect());
        }
        v
    };
    let seeds = [0usize, 1, 2, DEFAULT_SEED, usize::MAX];
    for pat in &patterns {
        for len in 0..=pat.len() {
            for &seed in &seeds {
                let mut buf = pat.clone();
                unsafe {
                    let hc = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    let hr = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    diff("2", &format!("hash_bytes({pat:x?}[..{len}],{seed:#x})"), hc, hr);
                }
            }
        }
    }
}

// ===========================================================================
// Row 3 — stbds_hash_string
// ===========================================================================

#[test]
fn row03_hash_string() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xB0003);
    let seeds = [0usize, 1, DEFAULT_SEED, usize::MAX, 0x0123456789ABCDEF];
    for len in 0..=64usize {
        for &seed in &seeds {
            for iter in 0..8 {
                // mix pure-ASCII and arbitrary non-zero bytes (>=0x80 exercises
                // the signed-char promotion in `(unsigned char) *str++`)
                let body: Vec<u8> = if iter % 2 == 0 {
                    rng.ascii(len)
                } else {
                    rng.bytes(len)
                        .into_iter()
                        .map(|b| if b == 0 { 0x80 } else { b })
                        .collect()
                };
                let mut s = cstring(&body);
                unsafe {
                    let hc = (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                    let hr = (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                    diff("3", &format!("hash_string(len={len},seed={seed:#x})"), hc, hr);
                }
            }
        }
    }
    // explicit edge cases
    for body in [
        vec![],
        b"a".to_vec(),
        vec![0xFF],
        vec![0x80, 0xFF, 0x7F],
        b"test_0".to_vec(),
        vec![0xC3, 0xA9],
    ] {
        let mut s = cstring(&body);
        for &seed in &seeds {
            unsafe {
                let hc = (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                let hr = (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                diff("3", &format!("hash_string({body:x?},{seed:#x})"), hc, hr);
            }
        }
    }
}

// ===========================================================================
// Row 4 — rand_seed does not influence the explicit-seed hash entry points
// ===========================================================================

#[test]
fn row04_rand_seed_then_hash() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xB0004);
    for &s in &[0usize, 1, usize::MAX, DEFAULT_SEED, 0xABCDEF] {
        setup(&p, s);
        for _ in 0..20 {
            let n = rng.below(40);
            let mut buf = rng.bytes(n.max(1));
            let mut st = cstring(&rng.ascii(n));
            unsafe {
                diff(
                    "4",
                    "hash_bytes after rand_seed",
                    (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, n, s),
                    (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, n, s),
                );
                diff(
                    "4",
                    "hash_string after rand_seed",
                    (p.c.hash_string)(st.as_mut_ptr() as *mut c_char, s),
                    (p.r.hash_string)(st.as_mut_ptr() as *mut c_char, s),
                );
            }
        }
    }
}

// ===========================================================================
// Row 5 — the seed advance inside make_hash_index must match
// ===========================================================================

#[test]
fn row05_seed_advance_across_table_creation() {
    let _g = guard();
    let p = pair();
    for &s in &[0usize, 1, DEFAULT_SEED, usize::MAX, 12345] {
        setup(&p, s);
        // create many independent tables so the global seed advances repeatedly
        let mut seeds_c = Vec::new();
        let mut seeds_r = Vec::new();
        for _ in 0..25 {
            let mut mc = Map::new(&p.c, 8, 4);
            let mut mr = Map::new(&p.r, 8, 4);
            let mut k = 7i32.to_ne_bytes();
            mc.put(&mut k, &1i32.to_ne_bytes(), STBDS_HM_BINARY);
            mr.put(&mut k, &1i32.to_ne_bytes(), STBDS_HM_BINARY);
            seeds_c.push(mc.snap().seed);
            seeds_r.push(mr.snap().seed);
            mc.free();
            mr.free();
        }
        diff("5", &format!("table seed chain (start {s:#x})"), seeds_c, seeds_r);
    }
}

// ===========================================================================
// Rows 6-8 — stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

#[test]
fn row06_arrgrowf_fresh() {
    let _g = guard();
    let p = pair();
    for &elemsize in &[1usize, 2, 4, 8, 16, 24, 64] {
        for &(addlen, min_cap) in &[
            (0usize, 0usize),
            (0, 1),
            (1, 0),
            (0, 4),
            (5, 0),
            (0, 100),
            (100, 7),
            (3, 3),
            (1, 1000),
        ] {
            unsafe {
                let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                // `arrgrowf(NULL,e,0,0)` hits `min_cap <= arrcap(NULL)` and
                // returns NULL unchanged (ERRORS.md row 1) — both must agree.
                diff(
                    "6",
                    &format!("NULL-ness elemsize={elemsize} addlen={addlen} min_cap={min_cap}"),
                    ac.is_null(),
                    ar.is_null(),
                );
                if ac.is_null() {
                    continue;
                }
                diff(
                    "6",
                    &format!("fresh header elemsize={elemsize} addlen={addlen} min_cap={min_cap}"),
                    (header(ac).length, header(ac).capacity, header(ac).temp, header(ac).hash_table.is_null()),
                    (header(ar).length, header(ar).capacity, header(ar).temp, header(ar).hash_table.is_null()),
                );
                (p.c.arrfreef)(ac);
                (p.r.arrfreef)(ar);
            }
        }
    }
}

#[test]
fn row07_arrgrowf_repeated_growth_and_early_return() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xB0007);
    for &elemsize in &[1usize, 4, 8, 16, 24] {
        unsafe {
            let mut ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            let mut ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            for step in 0..120 {
                let addlen = rng.below(6);
                let min_cap = rng.below(40);
                // simulate the user having pushed elements
                let prev_len = header(ac).length;
                diff("7", "length in sync", prev_len, header(ar).length);
                let before_c = header(ac);
                let before_r = header(ar);
                // `min_cap <= stbds_arrcap(a)` early-return is fully determined
                // by the pre-state, so predict it rather than comparing raw
                // addresses (realloc may coincidentally return the same block).
                let effective = min_cap.max(prev_len + addlen);
                let early = effective <= before_c.capacity;
                diff("7", "pre-state in sync", before_c.capacity, before_r.capacity);
                let nc = (p.c.arrgrowf)(ac, elemsize, addlen, min_cap);
                let nr = (p.r.arrgrowf)(ar, elemsize, addlen, min_cap);
                if early {
                    assert_eq!(nc, ac, "row 7: C must early-return the same pointer");
                    assert_eq!(nr, ar, "row 7: RUST must early-return the same pointer");
                    diff("7", "header untouched on early return", before_c.capacity, header(nc).capacity);
                    diff("7", "header untouched on early return", before_r.capacity, header(nr).capacity);
                }
                ac = nc;
                ar = nr;
                diff(
                    "7",
                    &format!("capacity step={step} elemsize={elemsize} addlen={addlen} min_cap={min_cap}"),
                    (header(ac).length, header(ac).capacity),
                    (header(ar).length, header(ar).capacity),
                );
                // grow the logical length like arrput would
                let newlen = (prev_len + addlen).min(header(ac).capacity);
                (*header_mut(ac)).length = newlen;
                (*header_mut(ar)).length = newlen;
            }
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

#[test]
fn row08_arrgrowf_payload_survives_realloc() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xB0008);
    let elemsize = 8usize;
    unsafe {
        let mut ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
        let mut ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
        let mut payload: Vec<u8> = Vec::new();
        for round in 0..60 {
            let add = 1 + rng.below(5);
            ac = (p.c.arrgrowf)(ac, elemsize, add, 0);
            ar = (p.r.arrgrowf)(ar, elemsize, add, 0);
            let n = payload.len() / elemsize;
            for _ in 0..add {
                let e = rng.bytes(elemsize);
                payload.extend_from_slice(&e);
            }
            let total = payload.len();
            std::ptr::copy_nonoverlapping(payload.as_ptr(), ac as *mut u8, total);
            std::ptr::copy_nonoverlapping(payload.as_ptr(), ar as *mut u8, total);
            (*header_mut(ac)).length = total / elemsize;
            (*header_mut(ar)).length = total / elemsize;
            let mut bc = vec![0u8; total];
            let mut br = vec![0u8; total];
            std::ptr::copy_nonoverlapping(ac as *const u8, bc.as_mut_ptr(), total);
            std::ptr::copy_nonoverlapping(ar as *const u8, br.as_mut_ptr(), total);
            diff("8", &format!("payload round={round} n={n}"), bc, br);
            diff(
                "8",
                &format!("header round={round}"),
                (header(ac).length, header(ac).capacity),
                (header(ar).length, header(ar).capacity),
            );
        }
        (p.c.arrfreef)(ac);
        (p.r.arrfreef)(ar);
    }
}

// ===========================================================================
// Rows 9-11 — stbds_hmput_default
// ===========================================================================

#[test]
fn row09_hmput_default_fresh_and_idempotent() {
    let _g = guard();
    let p = pair();
    for &(elemsize, keysize) in &[(8usize, 4usize), (16, 8), (24, 8), (12, 4)] {
        setup(&p, DEFAULT_SEED);
        let mut mc = Map::new(&p.c, elemsize, keysize);
        let mut mr = Map::new(&p.r, elemsize, keysize);
        let v: Vec<u8> = vec![0xA5; elemsize - keysize];
        mc.put_default(&v);
        mr.put_default(&v);
        diff("9", "snapshot after 1st default", mc.snap(), mr.snap());
        diff("9", "elems after 1st default", mc.elem_bytes(), mr.elem_bytes());
        let t_before_c = mc.t;
        let t_before_r = mr.t;
        mc.put_default(&v);
        mr.put_default(&v);
        diff("9", "idempotent (same ptr)", mc.t == t_before_c, mr.t == t_before_r);
        diff("9", "snapshot after 2nd default", mc.snap(), mr.snap());
        diff("9", "elems after 2nd default", mc.elem_bytes(), mr.elem_bytes());
        mc.free();
        mr.free();
    }
}

#[test]
fn row10_hmput_default_resurrect_zero_length() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);
    setup(&p, DEFAULT_SEED);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    mc.put_default(&[1u8, 2, 3, 4]);
    mr.put_default(&[1u8, 2, 3, 4]);
    unsafe {
        // force the "length == 0" branch
        (*header_mut(to_arr(mc.t, elemsize))).length = 0;
        (*header_mut(to_arr(mr.t, elemsize))).length = 0;
    }
    mc.put_default(&[9u8, 8, 7, 6]);
    mr.put_default(&[9u8, 8, 7, 6]);
    diff("10", "snapshot after resurrect", mc.snap(), mr.snap());
    diff("10", "elems after resurrect", mc.elem_bytes(), mr.elem_bytes());
    mc.free();
    mr.free();
}

#[test]
fn row11_hmput_default_then_miss_lookup() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);
    setup(&p, DEFAULT_SEED);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    mc.put_default(&(-2i32).to_ne_bytes());
    mr.put_default(&(-2i32).to_ne_bytes());
    for k in -3i32..12 {
        let mut kb = k.to_ne_bytes();
        let tc = mc.geti(&mut kb, STBDS_HM_BINARY);
        let tr = mr.geti(&mut kb, STBDS_HM_BINARY);
        diff("11", &format!("miss temp k={k}"), tc, tr);
        diff(
            "11",
            &format!("default value read k={k}"),
            mc.value_at(tc),
            mr.value_at(tr),
        );
        diff("11", &format!("snap k={k}"), mc.snap(), mr.snap());
    }
    mc.free();
    mr.free();
}

// ===========================================================================
// Rows 12-14 — binary maps, several element/key sizes
// ===========================================================================

fn binary_sweep(row: &str, elemsize: usize, keysize: usize, seed_kind: usize) {
    let p = pair();
    let mut rng = Rng::new(0xB1200 + elemsize as u64 * 31 + keysize as u64);
    for &n in &[0usize, 1, 2, 5, 6, 7, 8, 9, 13, 50, 200, 1000] {
        // skip counts that cannot be met with distinct keys of `keysize` bytes
        let space: u128 = 1u128 << (8 * keysize.min(8));
        if (n as u128) * 2 > space {
            continue;
        }
        setup(&p, seed_kind);
        let mut mc = Map::new(&p.c, elemsize, keysize);
        let mut mr = Map::new(&p.r, elemsize, keysize);
        mc.put_default(&vec![0xEE; elemsize - keysize]);
        mr.put_default(&vec![0xEE; elemsize - keysize]);

        // distinct random keys
        let mut keys: Vec<Vec<u8>> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while keys.len() < n {
            let k = rng.bytes(keysize);
            if seen.insert(k.clone()) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let val = rng.bytes(elemsize - keysize);
            let mut kc = k.clone();
            let mut kr = k.clone();
            let tc = mc.put(&mut kc, &val, STBDS_HM_BINARY);
            let tr = mr.put(&mut kr, &val, STBDS_HM_BINARY);
            diff(row, &format!("put#{i} n={n} temp"), tc, tr);
            diff(row, &format!("put#{i} n={n} snapshot"), mc.snap(), mr.snap());
            diff(row, &format!("put#{i} n={n} elems"), mc.elem_bytes(), mr.elem_bytes());
        }
        // look up every inserted key
        for (i, k) in keys.iter().enumerate() {
            let mut kc = k.clone();
            let mut kr = k.clone();
            let tc = mc.geti(&mut kc, STBDS_HM_BINARY);
            let tr = mr.geti(&mut kr, STBDS_HM_BINARY);
            diff(row, &format!("geti#{i} n={n}"), tc, tr);
            assert!(tc >= 0, "{row}: C lost inserted key #{i}");
            diff(row, &format!("value#{i} n={n}"), mc.value_at(tc), mr.value_at(tr));
        }
        // misses
        for i in 0..20 {
            let mut kc = rng.bytes(keysize);
            let mut kr = kc.clone();
            let tc = mc.geti(&mut kc, STBDS_HM_BINARY);
            let tr = mr.geti(&mut kr, STBDS_HM_BINARY);
            diff(row, &format!("miss#{i} n={n}"), tc, tr);
            diff(row, &format!("miss#{i} snap n={n}"), mc.snap(), mr.snap());
        }
        diff(row, &format!("final elems n={n}"), mc.elem_bytes(), mr.elem_bytes());
        mc.free();
        mr.free();
    }
}

#[test]
fn row12_binary_elem8_key4() {
    let _g = guard();
    binary_sweep("12", 8, 4, DEFAULT_SEED);
}

#[test]
fn row13_binary_elem16_key8() {
    let _g = guard();
    binary_sweep("13", 16, 8, DEFAULT_SEED);
    binary_sweep("13", 16, 8, 0);
}

#[test]
fn row14_binary_odd_strides() {
    let _g = guard();
    binary_sweep("14", 24, 8, DEFAULT_SEED);
    binary_sweep("14", 12, 4, 7);
    binary_sweep("14", 5, 1, 1);
    binary_sweep("14", 6, 2, usize::MAX);
}

// ===========================================================================
// Row 15 — duplicate-key re-put (update path)
// ===========================================================================

#[test]
fn row15_binary_duplicate_put() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xB0015);
    for &n in &[1usize, 2, 8, 9, 40, 300] {
        setup(&p, DEFAULT_SEED);
        let (elemsize, keysize) = (8usize, 4usize);
        let mut mc = Map::new(&p.c, elemsize, keysize);
        let mut mr = Map::new(&p.r, elemsize, keysize);
        let keys: Vec<i32> = (0..n as i32).map(|i| i * 3 - 5).collect();
        for &k in &keys {
            let v = rng.bytes(4);
            mc.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY);
            mr.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY);
        }
        let len_c = mc.snap().length;
        // re-put every key with a new value
        for (i, &k) in keys.iter().enumerate() {
            let v = rng.bytes(4);
            let tc = mc.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY);
            let tr = mr.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY);
            diff("15", &format!("re-put#{i} temp n={n}"), tc, tr);
            diff("15", &format!("re-put#{i} snap n={n}"), mc.snap(), mr.snap());
            diff("15", &format!("re-put#{i} elems n={n}"), mc.elem_bytes(), mr.elem_bytes());
        }
        diff("15", "length unchanged by re-put", len_c, mc.snap().length);
        diff("15", "final snap", mc.snap(), mr.snap());
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Row 16 — probe wrap-around into the second scan / multi-bucket chains
// ===========================================================================

#[test]
fn row16_binary_probe_wraparound() {
    let _g = guard();
    let p = pair();
    // Many keys in a small table with a fixed seed forces long probe chains and
    // the `i = 0 .. limit` wrap scan.  Several seeds broaden coverage.
    for &seed in &[0usize, 1, 2, 3, DEFAULT_SEED, usize::MAX, 0xFFFF_0000] {
        setup(&p, seed);
        let (elemsize, keysize) = (8usize, 4usize);
        let mut mc = Map::new(&p.c, elemsize, keysize);
        let mut mr = Map::new(&p.r, elemsize, keysize);
        for k in 0i32..400 {
            let v = (k * 7).to_ne_bytes();
            let tc = mc.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY);
            let tr = mr.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY);
            diff("16", &format!("put k={k} seed={seed:#x}"), tc, tr);
            diff("16", &format!("buckets k={k} seed={seed:#x}"), mc.snap(), mr.snap());
        }
        for k in -50i32..450 {
            let tc = mc.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
            let tr = mr.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
            diff("16", &format!("geti k={k} seed={seed:#x}"), tc, tr);
        }
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Row 17 — insertion across every growth threshold
// ===========================================================================

#[test]
fn row17_growth_thresholds() {
    let _g = guard();
    let p = pair();
    setup(&p, DEFAULT_SEED);
    let (elemsize, keysize) = (8usize, 4usize);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    let mut slot_counts = Vec::new();
    for k in 0i32..600 {
        let v = k.to_ne_bytes();
        let tc = mc.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY);
        let tr = mr.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY);
        diff("17", &format!("temp k={k}"), tc, tr);
        let sc = mc.snap();
        let sr = mr.snap();
        diff("17", &format!("full snapshot k={k}"), sc.clone(), sr);
        diff("17", &format!("elems k={k}"), mc.elem_bytes(), mr.elem_bytes());
        if slot_counts.last() != Some(&sc.slot_count) {
            slot_counts.push(sc.slot_count);
        }
    }
    assert!(
        slot_counts.len() >= 6,
        "expected several growth steps, saw {slot_counts:?}"
    );
    mc.free();
    mr.free();
}

// ===========================================================================
// Row 18 — hmget_key_ts
// ===========================================================================

#[test]
fn row18_hmget_key_ts() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);

    // (a) a == NULL: allocates and writes -1
    setup(&p, DEFAULT_SEED);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    let tc = mc.geti_ts(&mut 5i32.to_ne_bytes(), STBDS_HM_BINARY);
    let tr = mr.geti_ts(&mut 5i32.to_ne_bytes(), STBDS_HM_BINARY);
    diff("18", "ts on NULL map", tc, tr);
    diff("18", "snap after ts on NULL", mc.snap(), mr.snap());

    // (b) hash_table == NULL (only the default slot exists)
    let tc = mc.geti_ts(&mut 5i32.to_ne_bytes(), STBDS_HM_BINARY);
    let tr = mr.geti_ts(&mut 5i32.to_ne_bytes(), STBDS_HM_BINARY);
    diff("18", "ts with no table", tc, tr);

    // (c) populate, then hits + misses; header `temp` must be untouched by _ts
    for k in 0i32..64 {
        mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
        mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
    }
    // set header temp to a known sentinel, then confirm _ts leaves it alone
    unsafe {
        (*header_mut(to_arr(mc.t, elemsize))).temp = 0x7777;
        (*header_mut(to_arr(mr.t, elemsize))).temp = 0x7777;
    }
    for k in -8i32..72 {
        let tc = mc.geti_ts(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
        let tr = mr.geti_ts(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
        diff("18", &format!("ts k={k}"), tc, tr);
        diff("18", &format!("ts leaves header temp k={k}"), mc.temp(), mr.temp());
        diff("18", &format!("ts header temp is sentinel k={k}"), mc.temp(), 0x7777);
        if tc >= 0 {
            diff("18", &format!("ts value k={k}"), mc.value_at(tc), mr.value_at(tr));
        }
    }
    mc.free();
    mr.free();
}

// ===========================================================================
// Rows 19-23 — hmdel_key
// ===========================================================================

fn del_order_test(row: &str, order: &str, n: i32, seed: usize) {
    let p = pair();
    setup(&p, seed);
    let (elemsize, keysize) = (8usize, 4usize);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    mc.put_default(&(-2i32).to_ne_bytes());
    mr.put_default(&(-2i32).to_ne_bytes());
    for k in 0..n {
        mc.put(&mut k.to_ne_bytes(), &(k * 3).to_ne_bytes(), STBDS_HM_BINARY);
        mr.put(&mut k.to_ne_bytes(), &(k * 3).to_ne_bytes(), STBDS_HM_BINARY);
    }
    let mut keys: Vec<i32> = (0..n).collect();
    match order {
        "fwd" => {}
        "rev" => keys.reverse(),
        _ => {
            let mut rng = Rng::new(0xD0000 + n as u64);
            for i in (1..keys.len()).rev() {
                let j = rng.below(i + 1);
                keys.swap(i, j);
            }
        }
    }
    for (step, &k) in keys.iter().enumerate() {
        let dc = mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
        let dr = mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
        diff(row, &format!("del({k}) [{order}] step={step}"), dc, dr);
        diff(row, &format!("snap after del({k}) [{order}]"), mc.snap(), mr.snap());
        diff(row, &format!("elems after del({k}) [{order}]"), mc.elem_bytes(), mr.elem_bytes());
        // deleting again must miss
        let dc2 = mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
        let dr2 = mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
        diff(row, &format!("re-del({k}) [{order}]"), dc2, dr2);
        // every survivor must still be findable, with the same index
        for &s in keys.iter().skip(step + 1) {
            let gc = mc.geti(&mut s.to_ne_bytes(), STBDS_HM_BINARY);
            let gr = mr.geti(&mut s.to_ne_bytes(), STBDS_HM_BINARY);
            diff(row, &format!("survivor {s} after del({k}) [{order}]"), gc, gr);
            assert!(gc >= 0, "{row}: C lost survivor {s} after deleting {k}");
            diff(row, &format!("survivor {s} value"), mc.value_at(gc), mr.value_at(gr));
        }
    }
    diff(row, "final snap", mc.snap(), mr.snap());
    mc.free();
    mr.free();
}

#[test]
fn row19_hmdel_orders() {
    let _g = guard();
    for &n in &[1i32, 2, 3, 8, 9, 30] {
        del_order_test("19", "fwd", n, DEFAULT_SEED);
        del_order_test("19", "rev", n, DEFAULT_SEED);
        del_order_test("19", "rand", n, DEFAULT_SEED);
    }
    del_order_test("19", "rand", 60, 0);
    del_order_test("19", "rand", 60, usize::MAX);
}

#[test]
fn row20_hmdel_until_shrink() {
    let _g = guard();
    let p = pair();
    setup(&p, DEFAULT_SEED);
    let (elemsize, keysize) = (8usize, 4usize);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    for k in 0i32..200 {
        mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
        mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
    }
    let mut slots = vec![mc.snap().slot_count];
    for k in 0i32..200 {
        let dc = mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
        let dr = mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
        diff("20", &format!("del({k})"), dc, dr);
        let sc = mc.snap();
        diff("20", &format!("snap after del({k})"), sc.clone(), mr.snap());
        if *slots.last().unwrap() != sc.slot_count {
            slots.push(sc.slot_count);
        }
    }
    assert!(slots.len() >= 3, "expected shrink steps, saw {slots:?}");
    // remaining lookups
    for k in 0i32..200 {
        diff(
            "20",
            &format!("post-shrink geti({k})"),
            mc.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY),
            mr.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY),
        );
    }
    mc.free();
    mr.free();
}

#[test]
fn row21_hmdel_tombstone_rebuild() {
    let _g = guard();
    let p = pair();
    // Insert enough to keep used_count above the shrink threshold while
    // accumulating tombstones past (sc>>3)+(sc>>4), forcing the same-size
    // rebuild branch.
    for &seed in &[DEFAULT_SEED, 0usize, 99] {
        setup(&p, seed);
        let (elemsize, keysize) = (8usize, 4usize);
        let mut mc = Map::new(&p.c, elemsize, keysize);
        let mut mr = Map::new(&p.r, elemsize, keysize);
        for k in 0i32..500 {
            mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
            mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
        }
        let mut rng = Rng::new(0xB0021);
        let mut live: Vec<i32> = (0..500).collect();
        for round in 0..400 {
            if live.is_empty() {
                break;
            }
            // delete one, insert one -> tombstones accumulate, used_count steady
            let i = rng.below(live.len());
            let k = live.swap_remove(i);
            diff(
                "21",
                &format!("del({k}) round={round} seed={seed:#x}"),
                mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0),
                mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0),
            );
            let nk = 1000 + round as i32;
            diff(
                "21",
                &format!("put({nk}) round={round}"),
                mc.put(&mut nk.to_ne_bytes(), &nk.to_ne_bytes(), STBDS_HM_BINARY),
                mr.put(&mut nk.to_ne_bytes(), &nk.to_ne_bytes(), STBDS_HM_BINARY),
            );
            live.push(nk);
            diff("21", &format!("snap round={round}"), mc.snap(), mr.snap());
            diff("21", &format!("elems round={round}"), mc.elem_bytes(), mr.elem_bytes());
        }
        for &k in &live {
            let gc = mc.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
            let gr = mr.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
            diff("21", &format!("survivor {k}"), gc, gr);
            assert!(gc >= 0, "21: C lost survivor {k}");
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn row22_delete_then_reinsert_tombstone_reuse() {
    let _g = guard();
    let p = pair();
    for &seed in &[DEFAULT_SEED, 5usize] {
        setup(&p, seed);
        let (elemsize, keysize) = (8usize, 4usize);
        let mut mc = Map::new(&p.c, elemsize, keysize);
        let mut mr = Map::new(&p.r, elemsize, keysize);
        for cycle in 0..6 {
            for k in 0i32..40 {
                let v = (k + cycle * 100).to_ne_bytes();
                diff(
                    "22",
                    &format!("put({k}) cycle={cycle}"),
                    mc.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY),
                    mr.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY),
                );
                diff("22", &format!("snap put({k}) cycle={cycle}"), mc.snap(), mr.snap());
            }
            for k in 0i32..40 {
                diff(
                    "22",
                    &format!("del({k}) cycle={cycle}"),
                    mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0),
                    mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0),
                );
                diff("22", &format!("snap del({k}) cycle={cycle}"), mc.snap(), mr.snap());
            }
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn row23_hmdel_nonzero_keyoffset() {
    let _g = guard();
    let p = pair();
    // element = { size_t pad; int key; int _; }  -> keyoffset 8, keysize 4
    let (elemsize, keysize, keyoffset) = (16usize, 4usize, 8usize);
    setup(&p, DEFAULT_SEED);
    let mut tc: *mut c_void = std::ptr::null_mut();
    let mut tr: *mut c_void = std::ptr::null_mut();
    unsafe {
        // Build the map with keyoffset 0 puts (that is what hmput_key supports),
        // then delete with the offset variant on a layout where the key really
        // does live at offset 0 -> exercise keyoffset != 0 by placing a copy.
        // hmput_key always uses keyoffset 0, so build with 0 and then use the
        // raw hmdel_key with keyoffset 0 and keyoffset 8 on distinct maps.
        for &ko in &[0usize, keyoffset] {
            setup(&p, DEFAULT_SEED);
            let mut mc = Map::new(&p.c, elemsize, keysize);
            let mut mr = Map::new(&p.r, elemsize, keysize);
            for k in 0i32..30 {
                let mut e = vec![0u8; elemsize - keysize];
                e[..4].copy_from_slice(&(k * 5).to_ne_bytes());
                mc.put(&mut k.to_ne_bytes(), &e, STBDS_HM_BINARY);
                mr.put(&mut k.to_ne_bytes(), &e, STBDS_HM_BINARY);
            }
            // mirror the key into offset `ko` of every element so a lookup at
            // that offset can succeed in both libraries identically
            if ko != 0 {
                for m in [&mc, &mr] {
                    let len = m.snap().length;
                    for i in 1..len {
                        let e = (to_arr(m.t, elemsize) as *mut u8).wrapping_add(i * elemsize);
                        let key = std::slice::from_raw_parts(e, keysize).to_vec();
                        std::ptr::copy_nonoverlapping(
                            key.as_ptr(),
                            e.wrapping_add(ko),
                            keysize,
                        );
                    }
                }
            }
            for k in 0i32..30 {
                let dc = mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, ko);
                let dr = mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, ko);
                diff("23", &format!("del({k}) keyoffset={ko}"), dc, dr);
                diff("23", &format!("snap del({k}) keyoffset={ko}"), mc.snap(), mr.snap());
                diff(
                    "23",
                    &format!("elems del({k}) keyoffset={ko}"),
                    mc.elem_bytes(),
                    mr.elem_bytes(),
                );
            }
            tc = mc.t;
            tr = mr.t;
            mc.free();
            mr.free();
        }
    }
    let _ = (tc, tr);
}
