//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH `.so`s through `libloading` and compares a
//! value-only trace (array contents, header fields, and the full
//! `stbds_hash_index` scalar + bucket state) byte-for-byte.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEEDS: [usize; 5] = [0, 1, 0x31415926, usize::MAX, 0xDEADBEEFCAFE1234];

// ===========================================================================
// Rows 1-3 — stbds_hash_bytes
// ===========================================================================

#[test]
fn cfg_01_hash_bytes_small() {
    diff("cfg_01_hash_bytes_small", |lib, t| {
        let mut rng = Rng::new(0xA1);
        unsafe {
            for len in 0..=64usize {
                for _ in 0..40 {
                    let buf = rng.bytes(len);
                    let seed = rng.next_u64() as usize;
                    let h = (lib.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                    t.push(Ev::U(h));
                }
            }
            // every fixed seed x every tail length
            for &seed in SEEDS.iter() {
                for len in 0..=32usize {
                    let buf: Vec<u8> = (0..len).map(|i| i as u8).collect();
                    t.push(Ev::U((lib.hash_bytes)(
                        buf.as_ptr() as *mut c_void,
                        len,
                        seed,
                    )));
                }
            }
        }
    });
}

#[test]
fn cfg_02_hash_bytes_high_bit() {
    // The C block load `d[0]|(d[1]<<8)|(d[2]<<16)|(d[3]<<24)` is computed in
    // `int`, so bytes >= 0x80 at offsets 3 and 7 sign-extend into `size_t`.
    diff("cfg_02_hash_bytes_high_bit", |lib, t| {
        let mut rng = Rng::new(0xB2);
        unsafe {
            for len in 0..=24usize {
                for _ in 0..40 {
                    let mut buf = rng.bytes(len);
                    for i in 0..buf.len() {
                        if i % 4 == 3 {
                            buf[i] |= 0x80;
                        }
                    }
                    let seed = rng.next_u64() as usize;
                    t.push(Ev::U((lib.hash_bytes)(
                        buf.as_ptr() as *mut c_void,
                        len,
                        seed,
                    )));
                }
            }
            // exhaustive over the tail byte values 0x00..0xFF for each tail len
            for len in 1..=7usize {
                for b in 0..=255u8 {
                    let buf: Vec<u8> = vec![b; len];
                    t.push(Ev::U((lib.hash_bytes)(
                        buf.as_ptr() as *mut c_void,
                        len,
                        0x31415926,
                    )));
                }
            }
        }
    });
}

#[test]
fn cfg_03_hash_bytes_large() {
    diff("cfg_03_hash_bytes_large", |lib, t| {
        let mut rng = Rng::new(0xC3);
        unsafe {
            for _ in 0..200 {
                let len = 65 + rng.below(536);
                let buf = rng.bytes(len);
                let seed = rng.next_u64() as usize;
                t.push(Ev::U((lib.hash_bytes)(
                    buf.as_ptr() as *mut c_void,
                    len,
                    seed,
                )));
            }
        }
    });
}

// ===========================================================================
// Rows 4-5 — stbds_hash_string
// ===========================================================================

#[test]
fn cfg_04_hash_string_random() {
    diff("cfg_04_hash_string_random", |lib, t| {
        let mut rng = Rng::new(0xD4);
        unsafe {
            for len in 0..=64usize {
                for _ in 0..40 {
                    let s = rng.ascii_cstring(len);
                    let seed = rng.next_u64() as usize;
                    t.push(Ev::U((lib.hash_string)(s.as_ptr() as *mut c_char, seed)));
                }
            }
            for &seed in SEEDS.iter() {
                for len in 0..=40usize {
                    let s: Vec<u8> = (0..len).map(|i| b'a' + (i as u8 % 26)).chain([0]).collect();
                    t.push(Ev::U((lib.hash_string)(s.as_ptr() as *mut c_char, seed)));
                }
            }
        }
    });
}

#[test]
fn cfg_05_hash_string_high_bytes() {
    diff("cfg_05_hash_string_high_bytes", |lib, t| {
        let mut rng = Rng::new(0xE5);
        unsafe {
            // (unsigned char) cast => zero extension, never sign extension
            for b in 1..=255u8 {
                for len in 1..=8usize {
                    let mut s = vec![b; len];
                    s.push(0);
                    t.push(Ev::U((lib.hash_string)(
                        s.as_ptr() as *mut c_char,
                        0x31415926,
                    )));
                }
            }
            for _ in 0..500 {
                let len = rng.below(40);
                let s = rng.cstring(len);
                let seed = rng.next_u64() as usize;
                t.push(Ev::U((lib.hash_string)(s.as_ptr() as *mut c_char, seed)));
            }
        }
    });
}

// ===========================================================================
// Row 6 — stbds_rand_seed and the global seed LCG
// ===========================================================================

#[test]
fn cfg_06_rand_seed_lcg() {
    diff("cfg_06_rand_seed_lcg", |lib, t| unsafe {
        for &seed in SEEDS.iter() {
            (lib.rand_seed)(seed);
            // each fresh table consumes one step of the global LCG; observe the
            // per-table `seed` field for a run of consecutive fresh tables
            let mut keep = Vec::new();
            for _ in 0..8 {
                let p = (lib.shmode_func)(16, STBDS_SH_DEFAULT);
                let raw = (p as *mut u8).sub(16) as *mut c_void;
                t.push(Ev::Tbl(snap_table(raw)));
                keep.push((p, raw));
            }
            for (_, raw) in keep {
                (lib.hmfree_func)(raw, 16);
            }
        }
    });
}

// ===========================================================================
// Rows 7-11 — stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

#[test]
fn cfg_07_arrgrowf_fresh() {
    diff("cfg_07_arrgrowf_fresh", |lib, t| unsafe {
        for &elemsize in [1usize, 4, 8, 16, 64].iter() {
            for &addlen in [0usize, 1, 2, 7].iter() {
                for &min_cap in [0usize, 1, 4, 5, 100].iter() {
                    let a = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    t.push(Ev::Bool(a.is_null()));
                    t.push(Ev::Arr(snap_arr(a, elemsize)));
                    // addlen == 0 && min_cap == 0 makes `min_cap <= arrcap(NULL)`
                    // true, so the C returns NULL unchanged and there is nothing
                    // to free (freeing NULL is covered in the error-path suite).
                    if !a.is_null() {
                        (lib.arrfreef)(a);
                    }
                }
            }
        }
    });
}

#[test]
fn cfg_08_arrgrowf_double() {
    diff("cfg_08_arrgrowf_double", |lib, t| unsafe {
        let mut rng = Rng::new(0x108);
        for &elemsize in [1usize, 4, 8, 16].iter() {
            for _ in 0..20 {
                let mut a = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
                // 12 doublings: capacity 4 -> ~16k, i.e. bounded memory
                for _ in 0..12 {
                    let h = (a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
                    // simulate arrput-style growth: fill to capacity then add 1
                    (*h).length = (*h).capacity;
                    let addlen = 1 + rng.below(3);
                    a = (lib.arrgrowf)(a, elemsize, addlen, 0);
                    t.push(Ev::U((*(((a as *mut u8).sub(HEADER_SIZE)) as *mut ArrayHeader)).capacity));
                    t.push(Ev::U((*(((a as *mut u8).sub(HEADER_SIZE)) as *mut ArrayHeader)).length));
                }
                (lib.arrfreef)(a);
            }
        }
    });
}

#[test]
fn cfg_09_arrgrowf_noop() {
    diff("cfg_09_arrgrowf_noop", |lib, t| unsafe {
        for &elemsize in [1usize, 4, 8, 32].iter() {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let cap = (*(((a as *mut u8).sub(HEADER_SIZE)) as *mut ArrayHeader)).capacity;
            for min_cap in 0..=cap {
                let b = (lib.arrgrowf)(a, elemsize, 0, min_cap);
                t.push(Ev::Bool(std::ptr::eq(a as *const u8, b as *const u8)));
                t.push(Ev::Arr(snap_arr(b, elemsize)));
            }
            (lib.arrfreef)(a);
        }
    });
}

#[test]
fn cfg_10_arrgrowf_jump() {
    diff("cfg_10_arrgrowf_jump", |lib, t| unsafe {
        let mut rng = Rng::new(0x110);
        for &elemsize in [1usize, 4, 8, 16].iter() {
            for _ in 0..50 {
                let mut a = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
                for _ in 0..6 {
                    let min_cap = 1 + rng.below(5000);
                    a = (lib.arrgrowf)(a, elemsize, 0, min_cap);
                    t.push(Ev::Arr(snap_arr(a, elemsize)));
                }
                (lib.arrfreef)(a);
            }
        }
    });
}

#[test]
fn cfg_11_arrgrowf_cycle() {
    diff("cfg_11_arrgrowf_cycle", |lib, t| unsafe {
        let mut rng = Rng::new(0x111);
        for _ in 0..100 {
            let elemsize = 1 + rng.below(64);
            let mut a = std::ptr::null_mut();
            for _ in 0..10 {
                let addlen = rng.below(8);
                let min_cap = rng.below(64);
                a = (lib.arrgrowf)(a, elemsize, addlen, min_cap);
                let h = (a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
                t.push(Ev::U((*h).length));
                t.push(Ev::U((*h).capacity));
                t.push(Ev::I((*h).temp));
                t.push(Ev::Bool((*h).hash_table.is_null()));
                (*h).length = (*h).capacity.min((*h).length + addlen);
            }
            (lib.arrfreef)(a);
        }
    });
}

// ===========================================================================
// Rows 12-13 — arr_ins / strkey
// ===========================================================================

#[test]
fn cfg_12_arr_ins() {
    diff("cfg_12_arr_ins", |lib, t| unsafe {
        let mut rng = Rng::new(0x112);
        let mut nums: Vec<c_int> = vec![0, 1, 4, -1, c_int::MIN, c_int::MAX];
        for _ in 0..2000 {
            nums.push(rng.next_u32() as c_int);
        }
        for n in nums {
            (lib.arr_ins)(n);
            t.push(Ev::I(n as isize));
        }
    });
}

#[test]
fn cfg_13_strkey() {
    diff("cfg_13_strkey", |lib, t| unsafe {
        let mut rng = Rng::new(0x113);
        let mut ns: Vec<c_int> = vec![0, -1, 1, 9, 10, 99, 100, c_int::MIN, c_int::MAX];
        for _ in 0..2000 {
            ns.push(rng.next_u32() as c_int);
        }
        for n in ns {
            let p = (lib.strkey)(n);
            t.push(Ev::Bytes(cstr_bytes(p)));
        }
    });
}

// ===========================================================================
// Row 14 — stbds_hmput_default
// ===========================================================================

#[test]
fn cfg_14_hmput_default() {
    diff("cfg_14_hmput_default", |lib, t| unsafe {
        for &elemsize in [8usize, 16, 32].iter() {
            let mut p = (lib.hmput_default)(std::ptr::null_mut(), elemsize);
            let raw = (p as *mut u8).sub(elemsize) as *mut c_void;
            t.push(Ev::Arr(snap_arr(raw, elemsize)));
            // second call must be a no-op (length != 0)
            let p2 = (lib.hmput_default)(p, elemsize);
            t.push(Ev::Bool(std::ptr::eq(p as *const u8, p2 as *const u8)));
            p = p2;
            let raw = (p as *mut u8).sub(elemsize) as *mut c_void;
            t.push(Ev::Arr(snap_arr(raw, elemsize)));
            // write the default value and re-check
            std::ptr::write_bytes((p as *mut u8).sub(elemsize), 0xAB, elemsize);
            t.push(Ev::Arr(snap_arr(raw, elemsize)));
            (lib.hmfree_func)(raw, elemsize);
        }
    });
}

// ===========================================================================
// Rows 15-24 — binary hash maps
// ===========================================================================

/// (elemsize, keysize) layouts with NO padding, so the whole element is
/// deterministic once the driver writes key+value.
const BIN_LAYOUTS: [(usize, usize); 5] = [(8, 4), (16, 8), (20, 8), (12, 4), (32, 16)];

fn key_bytes(rng: &mut Rng, keysize: usize) -> Vec<u8> {
    rng.bytes(keysize)
}

fn run_bin_map(
    lib: &Lib,
    t: &mut Trace,
    elemsize: usize,
    keysize: usize,
    n: usize,
    seed: usize,
    rngseed: u64,
) {
    unsafe {
        (lib.rand_seed)(seed);
        let mut rng = Rng::new(rngseed);
        let mut hm = Hm::new(lib, elemsize, keysize, 0);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..n {
            let k = key_bytes(&mut rng, keysize);
            let v: Vec<u8> = (0..elemsize - keysize).map(|j| (i as u8).wrapping_add(j as u8)).collect();
            let idx = hm.put_kv(&k, &v, STBDS_HM_BINARY);
            t.push(Ev::I(idx));
            keys.push(k);
            let (a, tb) = hm.snap();
            t.push(Ev::Arr(a));
            t.push(Ev::Tbl(tb));
        }
        // lookups: present + absent
        for k in keys.iter() {
            t.push(Ev::I(hm.get(k, STBDS_HM_BINARY)));
            t.push(Ev::I(hm.get_ts(k, STBDS_HM_BINARY)));
        }
        for _ in 0..n.max(4) {
            let k = key_bytes(&mut rng, keysize);
            t.push(Ev::I(hm.get(&k, STBDS_HM_BINARY)));
        }
        let (a, tb) = hm.snap();
        t.push(Ev::Arr(a));
        t.push(Ev::Tbl(tb));
        hm.free();
        t.push(Ev::Tag("freed"));
    }
}

#[test]
fn cfg_15_bin_map_8_4_counts() {
    diff("cfg_15_bin_map_8_4_counts", |lib, t| {
        for &n in [0usize, 1, 6, 7, 8, 9, 100, 1000].iter() {
            for (si, &seed) in SEEDS.iter().enumerate() {
                run_bin_map(lib, t, 8, 4, n, seed, 0x1500 + si as u64 * 7 + n as u64);
            }
        }
    });
}

#[test]
fn cfg_16_bin_map_16_8() {
    diff("cfg_16_bin_map_16_8", |lib, t| {
        for (si, &seed) in SEEDS.iter().enumerate() {
            run_bin_map(lib, t, 16, 8, 1000, seed, 0x1600 + si as u64);
        }
    });
}

#[test]
fn cfg_17_bin_map_20_8() {
    diff("cfg_17_bin_map_20_8", |lib, t| {
        for (si, &seed) in SEEDS.iter().enumerate() {
            run_bin_map(lib, t, 20, 8, 500, seed, 0x1700 + si as u64);
        }
    });
}

#[test]
fn cfg_18_bin_map_reput() {
    diff("cfg_18_bin_map_reput", |lib, t| unsafe {
        let mut rng = Rng::new(0x118);
        for &(elemsize, keysize) in BIN_LAYOUTS.iter() {
            (lib.rand_seed)(0x31415926);
            let mut hm = Hm::new(lib, elemsize, keysize, 0);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..300usize {
                // 50% re-put an existing key, 50% a brand new one
                let k = if !keys.is_empty() && rng.next_u64() % 2 == 0 {
                    keys[rng.below(keys.len())].clone()
                } else {
                    let k = key_bytes(&mut rng, keysize);
                    keys.push(k.clone());
                    k
                };
                let v: Vec<u8> = (0..elemsize - keysize)
                    .map(|j| (i as u8).wrapping_mul(3).wrapping_add(j as u8))
                    .collect();
                t.push(Ev::I(hm.put_kv(&k, &v, STBDS_HM_BINARY)));
                let (a, tb) = hm.snap();
                t.push(Ev::Arr(a));
                t.push(Ev::Tbl(tb));
            }
            hm.free();
        }
    });
}

#[test]
fn cfg_19_bin_map_get() {
    diff("cfg_19_bin_map_get", |lib, t| unsafe {
        let mut rng = Rng::new(0x119);
        for &(elemsize, keysize) in BIN_LAYOUTS.iter() {
            (lib.rand_seed)(7);
            let mut hm = Hm::new(lib, elemsize, keysize, 0);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for batch in 0..20usize {
                for i in 0..10usize {
                    let k = key_bytes(&mut rng, keysize);
                    let v: Vec<u8> = vec![(batch * 10 + i) as u8; elemsize - keysize];
                    hm.put_kv(&k, &v, STBDS_HM_BINARY);
                    keys.push(k);
                }
                for k in keys.iter() {
                    t.push(Ev::I(hm.get(k, STBDS_HM_BINARY)));
                }
                for _ in 0..10 {
                    let k = key_bytes(&mut rng, keysize);
                    t.push(Ev::I(hm.get(&k, STBDS_HM_BINARY)));
                }
                let (a, tb) = hm.snap();
                t.push(Ev::Arr(a));
                t.push(Ev::Tbl(tb));
            }
            hm.free();
        }
    });
}

#[test]
fn cfg_20_bin_map_get_ts() {
    diff("cfg_20_bin_map_get_ts", |lib, t| unsafe {
        let mut rng = Rng::new(0x120);
        // a == NULL through hmget_key_ts allocates the default element
        for &elemsize in [8usize, 16, 32].iter() {
            let mut temp: isize = 0x1234;
            let key = [0u8; 8];
            let p = (lib.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                4,
                &mut temp,
                STBDS_HM_BINARY,
            );
            t.push(Ev::I(temp));
            let raw = (p as *mut u8).sub(elemsize) as *mut c_void;
            t.push(Ev::Arr(snap_arr(raw, elemsize)));
            (lib.hmfree_func)(raw, elemsize);
        }
        for &(elemsize, keysize) in BIN_LAYOUTS.iter() {
            (lib.rand_seed)(0xABCD);
            let mut hm = Hm::new(lib, elemsize, keysize, 0);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..200usize {
                let k = key_bytes(&mut rng, keysize);
                let v: Vec<u8> = vec![i as u8; elemsize - keysize];
                hm.put_kv(&k, &v, STBDS_HM_BINARY);
                keys.push(k);
            }
            for k in keys.iter() {
                t.push(Ev::I(hm.get_ts(k, STBDS_HM_BINARY)));
                // get_ts must NOT touch header->temp
                t.push(Ev::I(hm.temp()));
            }
            for _ in 0..50 {
                let k = key_bytes(&mut rng, keysize);
                t.push(Ev::I(hm.get_ts(&k, STBDS_HM_BINARY)));
            }
            hm.free();
        }
    });
}

#[test]
fn cfg_21_bin_map_del() {
    diff("cfg_21_bin_map_del", |lib, t| unsafe {
        let mut rng = Rng::new(0x121);
        for &(elemsize, keysize) in BIN_LAYOUTS.iter() {
            for &seed in [0usize, 1, 0x31415926].iter() {
                (lib.rand_seed)(seed);
                let mut hm = Hm::new(lib, elemsize, keysize, 0);
                let mut keys: Vec<Vec<u8>> = Vec::new();
                for i in 0..400usize {
                    let k = key_bytes(&mut rng, keysize);
                    let v: Vec<u8> = vec![i as u8; elemsize - keysize];
                    hm.put_kv(&k, &v, STBDS_HM_BINARY);
                    keys.push(k);
                }
                // delete a random subset -> crosses swap-with-last, shrink and
                // tombstone-rebuild paths
                let mut order: Vec<usize> = (0..keys.len()).collect();
                for i in (1..order.len()).rev() {
                    let j = rng.below(i + 1);
                    order.swap(i, j);
                }
                for &oi in order.iter() {
                    t.push(Ev::I(hm.del(&keys[oi], STBDS_HM_BINARY)));
                    let (a, tb) = hm.snap();
                    t.push(Ev::Arr(a));
                    t.push(Ev::Tbl(tb));
                }
                // everything is gone
                for k in keys.iter() {
                    t.push(Ev::I(hm.get(k, STBDS_HM_BINARY)));
                }
                hm.free();
            }
        }
    });
}

#[test]
fn cfg_22_bin_map_del_keyoffset() {
    // hmput/hmget hard-code keyoffset=0; only hmdel_key takes it. A non-zero
    // keyoffset therefore makes the delete miss - deterministically.
    diff("cfg_22_bin_map_del_keyoffset", |lib, t| unsafe {
        let mut rng = Rng::new(0x122);
        for &keyoffset in [4usize, 8, 12].iter() {
            (lib.rand_seed)(3);
            let mut hm = Hm::new(lib, 16, 4, keyoffset);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..60usize {
                let k = key_bytes(&mut rng, 4);
                let v: Vec<u8> = vec![i as u8; 12];
                hm.put_kv(&k, &v, STBDS_HM_BINARY);
                keys.push(k);
            }
            for k in keys.iter() {
                t.push(Ev::I(hm.del(k, STBDS_HM_BINARY)));
                let (a, tb) = hm.snap();
                t.push(Ev::Arr(a));
                t.push(Ev::Tbl(tb));
            }
            hm.free();
        }
    });
}

#[test]
fn cfg_23_tombstone_reuse() {
    diff("cfg_23_tombstone_reuse", |lib, t| unsafe {
        let mut rng = Rng::new(0x123);
        for &seed in SEEDS.iter() {
            (lib.rand_seed)(seed);
            let mut hm = Hm::new(lib, 8, 4, 0);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..200usize {
                let k = key_bytes(&mut rng, 4);
                hm.put_kv(&k, &vec![i as u8; 4], STBDS_HM_BINARY);
                keys.push(k);
            }
            // delete half, then re-insert them: forces tombstone reuse
            for i in (0..keys.len()).step_by(2) {
                hm.del(&keys[i], STBDS_HM_BINARY);
            }
            let (a, tb) = hm.snap();
            t.push(Ev::Arr(a));
            t.push(Ev::Tbl(tb));
            for i in (0..keys.len()).step_by(2) {
                t.push(Ev::I(hm.put_kv(&keys[i], &vec![0x77u8; 4], STBDS_HM_BINARY)));
                let (a, tb) = hm.snap();
                t.push(Ev::Arr(a));
                t.push(Ev::Tbl(tb));
            }
            hm.free();
        }
    });
}

#[test]
fn cfg_24_bin_map_lifecycle() {
    diff("cfg_24_bin_map_lifecycle", |lib, t| unsafe {
        let mut rng = Rng::new(0x124);
        for &(elemsize, keysize) in [(8usize, 4usize), (16, 8), (20, 8)].iter() {
            for &seed in [0usize, 12345, usize::MAX].iter() {
                (lib.rand_seed)(seed);
                let mut hm = Hm::new(lib, elemsize, keysize, 0);
                hm.put_default();
                t.push(Ev::Arr(snap_arr(hm.raw(), elemsize)));
                let mut live: Vec<Vec<u8>> = Vec::new();
                for i in 0..600usize {
                    match rng.below(4) {
                        0 | 1 => {
                            let k = key_bytes(&mut rng, keysize);
                            t.push(Ev::I(hm.put_kv(&k, &vec![i as u8; elemsize - keysize], STBDS_HM_BINARY)));
                            live.push(k);
                        }
                        2 => {
                            if !live.is_empty() {
                                let j = rng.below(live.len());
                                t.push(Ev::I(hm.del(&live[j], STBDS_HM_BINARY)));
                                live.swap_remove(j);
                            }
                        }
                        _ => {
                            let k = if !live.is_empty() && rng.next_u64() % 2 == 0 {
                                live[rng.below(live.len())].clone()
                            } else {
                                key_bytes(&mut rng, keysize)
                            };
                            t.push(Ev::I(hm.get(&k, STBDS_HM_BINARY)));
                        }
                    }
                    let (a, tb) = hm.snap();
                    t.push(Ev::Arr(a));
                    t.push(Ev::Tbl(tb));
                }
                hm.free();
                t.push(Ev::Tag("freed"));
            }
        }
    });
}

// ===========================================================================
// Rows 25-33 — string hash maps
// ===========================================================================

/// A stable pool of C strings; the SAME byte contents are used for both
/// libraries so that `SH_DEFAULT` (store-the-caller-pointer) is comparable by
/// content, and the strings outlive the maps.
struct KeyPool {
    bufs: Vec<Vec<u8>>,
}

impl KeyPool {
    fn new(rng: &mut Rng, n: usize, minlen: usize, maxlen: usize) -> KeyPool {
        let mut bufs = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while bufs.len() < n {
            let len = minlen + rng.below(maxlen - minlen + 1);
            let s = rng.ascii_cstring(len);
            if seen.insert(s.clone()) {
                bufs.push(s);
            }
        }
        KeyPool { bufs }
    }
    fn ptr(&self, i: usize) -> *mut c_char {
        self.bufs[i].as_ptr() as *mut c_char
    }
    fn len(&self) -> usize {
        self.bufs.len()
    }
}

fn run_str_map(
    lib: &Lib,
    t: &mut Trace,
    sh_mode: Option<c_int>,
    hm_mode: c_int,
    n: usize,
    minlen: usize,
    maxlen: usize,
    seed: usize,
    rngseed: u64,
    do_gets: bool,
    do_dels: bool,
) {
    unsafe {
        (lib.rand_seed)(seed);
        let mut rng = Rng::new(rngseed);
        let pool = KeyPool::new(&mut rng, n, minlen, maxlen);
        let mut hm = match sh_mode {
            Some(m) => Hm::from_shmode(lib, 16, 8, m),
            None => Hm::new(lib, 16, 8, 0),
        };
        t.push(Ev::Tbl(snap_table(hm.raw())));
        for i in 0..pool.len() {
            let v = (i as u64).to_le_bytes().to_vec();
            let idx = hm.put_str(pool.ptr(i), &v, hm_mode);
            t.push(Ev::I(idx));
            t.push(Ev::Bytes(cstr_bytes(hm.temp_key())));
            t.push(Ev::Str(snap_str_elems(&hm)));
            t.push(Ev::Tbl(snap_table(hm.raw())));
        }
        if do_gets {
            for i in 0..pool.len() {
                t.push(Ev::I(hm.get_str(pool.ptr(i), hm_mode)));
            }
            // absent keys, incl. prefixes and suffixes of present ones
            for i in 0..pool.len().min(50) {
                let mut s = pool.bufs[i].clone();
                s.pop();
                s.push(b'Z');
                s.push(0);
                t.push(Ev::I(hm.get_str(s.as_ptr() as *mut c_char, hm_mode)));
                if s.len() > 2 {
                    let mut p = pool.bufs[i].clone();
                    p.truncate(p.len().saturating_sub(2));
                    p.push(0);
                    t.push(Ev::I(hm.get_str(p.as_ptr() as *mut c_char, hm_mode)));
                }
            }
        }
        if do_dels {
            let mut order: Vec<usize> = (0..pool.len()).collect();
            for i in (1..order.len()).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            for &oi in order.iter() {
                t.push(Ev::I(hm.del_str(pool.ptr(oi), hm_mode)));
                t.push(Ev::Str(snap_str_elems(&hm)));
                t.push(Ev::Tbl(snap_table(hm.raw())));
            }
        }
        hm.free();
        t.push(Ev::Tag("freed"));
    }
}

#[test]
fn cfg_25_str_map_default_mode() {
    diff("cfg_25_str_map_default_mode", |lib, t| {
        for (si, &seed) in SEEDS.iter().enumerate() {
            run_str_map(lib, t, None, STBDS_HM_STRING, 200, 1, 20, seed, 0x2500 + si as u64, true, false);
        }
    });
}

#[test]
fn cfg_26_str_map_strdup() {
    diff("cfg_26_str_map_strdup", |lib, t| {
        for (si, &seed) in SEEDS.iter().enumerate() {
            run_str_map(
                lib,
                t,
                Some(STBDS_SH_STRDUP),
                STBDS_HM_STRING,
                200,
                1,
                20,
                seed,
                0x2600 + si as u64,
                true,
                false,
            );
        }
    });
}

#[test]
fn cfg_27_str_map_arena_short() {
    diff("cfg_27_str_map_arena_short", |lib, t| {
        for (si, &seed) in SEEDS.iter().enumerate() {
            run_str_map(
                lib,
                t,
                Some(STBDS_SH_ARENA),
                STBDS_HM_STRING,
                200,
                1,
                40,
                seed,
                0x2700 + si as u64,
                true,
                false,
            );
        }
    });
}

#[test]
fn cfg_28_str_map_arena_long() {
    diff("cfg_28_str_map_arena_long", |lib, t| {
        // keys longer than the initial 512-byte blocksize -> dedicated-block path
        for (si, &seed) in SEEDS.iter().enumerate() {
            run_str_map(
                lib,
                t,
                Some(STBDS_SH_ARENA),
                STBDS_HM_STRING,
                60,
                1,
                900,
                seed,
                0x2800 + si as u64,
                true,
                false,
            );
        }
    });
}

#[test]
fn cfg_29_str_map_sh_none() {
    // string.mode == STBDS_SH_NONE with mode=1: string hashing but the
    // `default:` memcpy branch stores the first `keysize` BYTES OF THE STRING in
    // the key slot (not a pointer).  Distinct keys only, and no lookups, since
    // a lookup would reinterpret those bytes as a char*.
    diff("cfg_29_str_map_sh_none", |lib, t| unsafe {
        for &seed in SEEDS.iter() {
            (lib.rand_seed)(seed);
            let mut rng = Rng::new(0x129);
            let pool = KeyPool::new(&mut rng, 6, 9, 20);
            let mut hm = Hm::from_shmode(lib, 16, 8, STBDS_SH_NONE);
            t.push(Ev::Tbl(snap_table(hm.raw())));
            for i in 0..pool.len() {
                let v = (i as u64).to_le_bytes().to_vec();
                t.push(Ev::I(hm.put_str(pool.ptr(i), &v, STBDS_HM_STRING)));
                t.push(Ev::Arr(snap_arr(hm.raw(), hm.elemsize)));
                t.push(Ev::Tbl(snap_table(hm.raw())));
            }
            hm.free();
        }
    });
}

#[test]
fn cfg_30_str_map_undefined_sh_mode() {
    diff("cfg_30_str_map_undefined_sh_mode", |lib, t| unsafe {
        for &shmode in [4i32, 5, 17, 255].iter() {
            (lib.rand_seed)(0x31415926);
            let mut rng = Rng::new(0x130 + shmode as u64);
            let pool = KeyPool::new(&mut rng, 6, 9, 20);
            let mut hm = Hm::from_shmode(lib, 16, 8, shmode);
            t.push(Ev::Tbl(snap_table(hm.raw())));
            for i in 0..pool.len() {
                let v = (i as u64).to_le_bytes().to_vec();
                t.push(Ev::I(hm.put_str(pool.ptr(i), &v, STBDS_HM_STRING)));
                t.push(Ev::Arr(snap_arr(hm.raw(), hm.elemsize)));
                t.push(Ev::Tbl(snap_table(hm.raw())));
            }
            hm.free();
        }
    });
}

#[test]
fn cfg_31_str_map_lookup_shapes() {
    diff("cfg_31_str_map_lookup_shapes", |lib, t| unsafe {
        for &shmode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA].iter() {
            (lib.rand_seed)(11);
            let mut rng = Rng::new(0x131 + shmode as u64);
            let pool = KeyPool::new(&mut rng, 120, 0, 30);
            let mut hm = Hm::from_shmode(lib, 16, 8, shmode);
            for i in 0..pool.len() {
                hm.put_str(pool.ptr(i), &(i as u64).to_le_bytes().to_vec(), STBDS_HM_STRING);
            }
            for i in 0..pool.len() {
                t.push(Ev::I(hm.get_str(pool.ptr(i), STBDS_HM_STRING)));
            }
            let empty = [0u8; 1];
            t.push(Ev::I(hm.get_str(empty.as_ptr() as *mut c_char, STBDS_HM_STRING)));
            for i in 0..pool.len() {
                let mut s = pool.bufs[i].clone();
                s.pop();
                s.extend_from_slice(b"x\0");
                t.push(Ev::I(hm.get_str(s.as_ptr() as *mut c_char, STBDS_HM_STRING)));
            }
            t.push(Ev::Str(snap_str_elems(&hm)));
            t.push(Ev::Tbl(snap_table(hm.raw())));
            hm.free();
        }
    });
}

#[test]
fn cfg_32_str_map_del_all_modes() {
    diff("cfg_32_str_map_del_all_modes", |lib, t| {
        for &shmode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA].iter() {
            for (si, &seed) in SEEDS.iter().enumerate() {
                run_str_map(
                    lib,
                    t,
                    Some(shmode),
                    STBDS_HM_STRING,
                    120,
                    1,
                    25,
                    seed,
                    0x3200 + si as u64 * 31 + shmode as u64,
                    true,
                    true,
                );
            }
        }
    });
}

#[test]
fn cfg_33_str_map_del_mode2_last() {
    // hmdel_key with mode == 2: `mode >= STBDS_HM_STRING` so hashing/compare are
    // string-based, but `mode == STBDS_HM_STRING` is FALSE, so the strdup free is
    // skipped and the re-find takes the *binary* branch. That re-find only runs
    // when old_index != final_index, so deleting the LAST element is well-defined.
    diff("cfg_33_str_map_del_mode2_last", |lib, t| unsafe {
        for &shmode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA].iter() {
            (lib.rand_seed)(0x31415926);
            let mut rng = Rng::new(0x133 + shmode as u64);
            let pool = KeyPool::new(&mut rng, 40, 3, 25);
            let mut hm = Hm::from_shmode(lib, 16, 8, shmode);
            for i in 0..pool.len() {
                hm.put_str(pool.ptr(i), &(i as u64).to_le_bytes().to_vec(), STBDS_HM_STRING);
            }
            // repeatedly delete the last-inserted key (old_index == final_index)
            for i in (0..pool.len()).rev() {
                t.push(Ev::I(hm.del_str(pool.ptr(i), 2)));
                t.push(Ev::Str(snap_str_elems(&hm)));
                t.push(Ev::Tbl(snap_table(hm.raw())));
            }
            hm.free();
        }
    });
}

// ===========================================================================
// Rows 34-35 — out-of-range mode values across the FFI boundary
// ===========================================================================

#[test]
fn cfg_34_mode_out_of_range() {
    // C enums/ints accept any value; the dispatch is `mode >= 1`, so 2, 7 and
    // INT_MAX behave like STRING and 0, -1, INT_MIN behave like BINARY.
    diff("cfg_34_mode_out_of_range", |lib, t| unsafe {
        for &mode in [c_int::MIN, -5, -1, 0].iter() {
            // binary-behaving modes: full put/get/del cycle
            (lib.rand_seed)(0x31415926);
            let mut rng = Rng::new(0x134 ^ (mode as u32 as u64));
            let mut hm = Hm::new(lib, 8, 4, 0);
            let mut keys = Vec::new();
            for i in 0..120usize {
                let k = key_bytes(&mut rng, 4);
                t.push(Ev::I(hm.put_kv(&k, &vec![i as u8; 4], mode)));
                keys.push(k);
            }
            for k in keys.iter() {
                t.push(Ev::I(hm.get(k, mode)));
            }
            for k in keys.iter() {
                t.push(Ev::I(hm.del(k, mode)));
            }
            t.push(Ev::Arr(snap_arr(hm.raw(), 8)));
            t.push(Ev::Tbl(snap_table(hm.raw())));
            hm.free();
        }
        for &mode in [2i32, 7, 1000, c_int::MAX].iter() {
            // string-behaving modes: put + get (delete of a non-last element is
            // covered in the error-path suite because it asserts)
            (lib.rand_seed)(0x31415926);
            let mut rng = Rng::new(0x1340 ^ (mode as u32 as u64));
            let pool = KeyPool::new(&mut rng, 80, 1, 25);
            let mut hm = Hm::from_shmode(lib, 16, 8, STBDS_SH_DEFAULT);
            for i in 0..pool.len() {
                t.push(Ev::I(hm.put_str(
                    pool.ptr(i),
                    &(i as u64).to_le_bytes().to_vec(),
                    mode,
                )));
            }
            for i in 0..pool.len() {
                t.push(Ev::I(hm.get_str(pool.ptr(i), mode)));
            }
            t.push(Ev::Str(snap_str_elems(&hm)));
            t.push(Ev::Tbl(snap_table(hm.raw())));
            hm.free();
        }
    });
}

#[test]
fn cfg_35_shmode_out_of_range() {
    diff("cfg_35_shmode_out_of_range", |lib, t| unsafe {
        for &m in [
            0i32,
            1,
            2,
            3,
            4,
            255,
            256,
            259,
            -1,
            -256,
            c_int::MIN,
            c_int::MAX,
        ]
        .iter()
        {
            (lib.rand_seed)(0x31415926);
            let p = (lib.shmode_func)(16, m);
            let raw = (p as *mut u8).sub(16) as *mut c_void;
            t.push(Ev::Arr(snap_arr(raw, 16)));
            t.push(Ev::Tbl(snap_table(raw)));
            (lib.hmfree_func)(raw, 16);
        }
    });
}

// ===========================================================================
// Rows 36-39 — string arena
// ===========================================================================

fn arena_zero() -> StringArena {
    StringArena {
        storage: std::ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    }
}

#[test]
fn cfg_36_stralloc_fill() {
    diff("cfg_36_stralloc_fill", |lib, t| unsafe {
        let mut rng = Rng::new(0x136);
        for _ in 0..20 {
            let mut a = arena_zero();
            for _ in 0..300 {
                let len = 1 + rng.below(600);
                let s = rng.ascii_cstring(len);
                let p = (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
                t.push(Ev::Bytes(cstr_bytes(p)));
                t.push(Ev::U(a.remaining));
                t.push(Ev::U(a.block as usize));
                t.push(Ev::Bool(a.storage.is_null()));
            }
            (lib.strreset)(&mut a);
            t.push(Ev::U(a.remaining));
            t.push(Ev::U(a.block as usize));
            t.push(Ev::Bool(a.storage.is_null()));
        }
    });
}

#[test]
fn cfg_37_stralloc_oversize() {
    diff("cfg_37_stralloc_oversize", |lib, t| unsafe {
        let mut rng = Rng::new(0x137);
        // (a) first allocation is oversize -> storage == NULL branch
        for &len in [513usize, 1024, 4096, 100000].iter() {
            let mut a = arena_zero();
            let s = rng.ascii_cstring(len);
            let p = (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
            t.push(Ev::Bytes(cstr_bytes(p)));
            t.push(Ev::U(a.remaining));
            t.push(Ev::U(a.block as usize));
            t.push(Ev::Bool(a.storage.is_null()));
            // (b) then a short one, then another oversize -> storage != NULL branch
            let s2 = rng.ascii_cstring(8);
            let p2 = (lib.stralloc)(&mut a, s2.as_ptr() as *mut c_char);
            t.push(Ev::Bytes(cstr_bytes(p2)));
            t.push(Ev::U(a.remaining));
            t.push(Ev::U(a.block as usize));
            let s3 = rng.ascii_cstring(len * 2);
            let p3 = (lib.stralloc)(&mut a, s3.as_ptr() as *mut c_char);
            t.push(Ev::Bytes(cstr_bytes(p3)));
            t.push(Ev::U(a.remaining));
            t.push(Ev::U(a.block as usize));
            (lib.strreset)(&mut a);
            t.push(Ev::U(a.remaining));
            t.push(Ev::Bool(a.storage.is_null()));
        }
    });
}

#[test]
fn cfg_38_stralloc_block_field() {
    // `blocksize = 512 << (a->block >> 1)`: saturates at 1<<20 for block >= 22,
    // and for block >= 128 the shift count reaches 64 (masked to 6 bits on
    // x86-64).  `block` is part of the public `stbds_string_arena`, so any value
    // 0..255 is a real input.
    diff("cfg_38_stralloc_block_field", |lib, t| unsafe {
        let mut rng = Rng::new(0x138);
        for blk in 0u8..=255 {
            // `512 << ((blk>>1) & 63)` is the blocksize the C will ask realloc
            // for.  Values above a few MB make realloc fail, and the C then
            // dereferences the NULL block -> SIGSEGV; that OOM path is compared
            // as a signal in the error-path suite instead.
            let sh = ((blk as usize) >> 1) & 63;
            let blocksize = 512usize.wrapping_shl(sh as u32);
            if blocksize > (4usize << 20) {
                continue;
            }
            for &len in [1usize, 8, 511, 512, 513, 2000].iter() {
                let mut a = arena_zero();
                a.block = blk;
                let s = rng.ascii_cstring(len);
                let p = (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
                t.push(Ev::U(blk as usize));
                t.push(Ev::U(len));
                t.push(Ev::Bytes(cstr_bytes(p)));
                t.push(Ev::U(a.remaining));
                t.push(Ev::U(a.block as usize));
                t.push(Ev::Bool(a.storage.is_null()));
                // a second allocation on the same arena
                let s2 = rng.ascii_cstring(16);
                let p2 = (lib.stralloc)(&mut a, s2.as_ptr() as *mut c_char);
                t.push(Ev::Bytes(cstr_bytes(p2)));
                t.push(Ev::U(a.remaining));
                t.push(Ev::U(a.block as usize));
                (lib.strreset)(&mut a);
            }
        }
    });
}

#[test]
fn cfg_39_strreset_shapes() {
    diff("cfg_39_strreset_shapes", |lib, t| unsafe {
        let mut rng = Rng::new(0x139);
        // empty arena
        let mut a = arena_zero();
        (lib.strreset)(&mut a);
        t.push(Ev::U(a.remaining));
        t.push(Ev::U(a.block as usize));
        t.push(Ev::U(a.mode as usize));
        t.push(Ev::Bool(a.storage.is_null()));
        // double reset
        (lib.strreset)(&mut a);
        t.push(Ev::Bool(a.storage.is_null()));
        // 1 block, many blocks
        for &n in [1usize, 2, 10, 100].iter() {
            let mut a = arena_zero();
            a.mode = 3;
            for _ in 0..n {
                let n2 = 1 + rng.below(700);
                let s = rng.ascii_cstring(n2);
                (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
            }
            t.push(Ev::U(a.remaining));
            t.push(Ev::U(a.block as usize));
            (lib.strreset)(&mut a);
            t.push(Ev::U(a.remaining));
            t.push(Ev::U(a.block as usize));
            t.push(Ev::U(a.mode as usize));
            t.push(Ev::Bool(a.storage.is_null()));
            (lib.strreset)(&mut a);
            t.push(Ev::Bool(a.storage.is_null()));
        }
    });
}

// ===========================================================================
// Row 40 — keysize sweep inside the map (all siphash paths)
// ===========================================================================

#[test]
fn cfg_40_keysize_sweep() {
    diff("cfg_40_keysize_sweep", |lib, t| unsafe {
        let mut rng = Rng::new(0x140);
        for keysize in 1..=40usize {
            let elemsize = keysize + 8;
            (lib.rand_seed)(0x31415926);
            let mut hm = Hm::new(lib, elemsize, keysize, 0);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..60usize {
                let k = key_bytes(&mut rng, keysize);
                t.push(Ev::I(hm.put_kv(&k, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_BINARY)));
                keys.push(k);
            }
            for k in keys.iter() {
                t.push(Ev::I(hm.get(k, STBDS_HM_BINARY)));
            }
            for k in keys.iter() {
                t.push(Ev::I(hm.del(k, STBDS_HM_BINARY)));
            }
            t.push(Ev::Arr(snap_arr(hm.raw(), elemsize)));
            t.push(Ev::Tbl(snap_table(hm.raw())));
            hm.free();
        }
    });
}

// ===========================================================================
// Rows 41-42 — long randomized model checks
// ===========================================================================

#[test]
fn cfg_41_random_ops_binary() {
    diff("cfg_41_random_ops_binary", |lib, t| unsafe {
        for (si, &seed) in SEEDS.iter().enumerate() {
            (lib.rand_seed)(seed);
            let mut rng = Rng::new(0x4100 + si as u64);
            let mut hm = Hm::new(lib, 16, 8, 0);
            let mut live: Vec<Vec<u8>> = Vec::new();
            for i in 0..5000usize {
                match rng.below(10) {
                    0..=3 => {
                        let k = if !live.is_empty() && rng.below(4) == 0 {
                            live[rng.below(live.len())].clone()
                        } else {
                            let k = key_bytes(&mut rng, 8);
                            live.push(k.clone());
                            k
                        };
                        t.push(Ev::I(hm.put_kv(&k, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_BINARY)));
                    }
                    4..=5 => {
                        let k = if !live.is_empty() && rng.below(2) == 0 {
                            live[rng.below(live.len())].clone()
                        } else {
                            key_bytes(&mut rng, 8)
                        };
                        t.push(Ev::I(hm.get(&k, STBDS_HM_BINARY)));
                    }
                    6 => {
                        let k = key_bytes(&mut rng, 8);
                        t.push(Ev::I(hm.get_ts(&k, STBDS_HM_BINARY)));
                    }
                    7..=8 => {
                        if !live.is_empty() {
                            let j = rng.below(live.len());
                            let k = live[j].clone();
                            t.push(Ev::I(hm.del(&k, STBDS_HM_BINARY)));
                            // the key may have been a duplicate; drop one copy
                            live.swap_remove(j);
                            live.retain(|x| x != &k);
                        } else {
                            let k = key_bytes(&mut rng, 8);
                            t.push(Ev::I(hm.del(&k, STBDS_HM_BINARY)));
                        }
                    }
                    _ => {
                        hm.put_default();
                        t.push(Ev::Tag("put_default"));
                    }
                }
                if i % 7 == 0 {
                    let (a, tb) = hm.snap();
                    t.push(Ev::Arr(a));
                    t.push(Ev::Tbl(tb));
                }
            }
            let (a, tb) = hm.snap();
            t.push(Ev::Arr(a));
            t.push(Ev::Tbl(tb));
            hm.free();
        }
    });
}

#[test]
fn cfg_42_random_ops_string() {
    diff("cfg_42_random_ops_string", |lib, t| unsafe {
        for &shmode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA].iter() {
            (lib.rand_seed)(0x31415926);
            let mut rng = Rng::new(0x4200 + shmode as u64);
            let pool = KeyPool::new(&mut rng, 400, 0, 40);
            let mut hm = Hm::from_shmode(lib, 16, 8, shmode);
            let mut live: std::collections::BTreeSet<usize> = Default::default();
            for i in 0..2000usize {
                let ki = rng.below(pool.len());
                match rng.below(10) {
                    0..=4 => {
                        t.push(Ev::I(hm.put_str(
                            pool.ptr(ki),
                            &(i as u64).to_le_bytes().to_vec(),
                            STBDS_HM_STRING,
                        )));
                        live.insert(ki);
                    }
                    5..=7 => {
                        t.push(Ev::I(hm.get_str(pool.ptr(ki), STBDS_HM_STRING)));
                    }
                    _ => {
                        t.push(Ev::I(hm.del_str(pool.ptr(ki), STBDS_HM_STRING)));
                        live.remove(&ki);
                    }
                }
                if i % 5 == 0 {
                    t.push(Ev::Str(snap_str_elems(&hm)));
                    t.push(Ev::Tbl(snap_table(hm.raw())));
                }
            }
            t.push(Ev::Str(snap_str_elems(&hm)));
            t.push(Ev::Tbl(snap_table(hm.raw())));
            hm.free();
        }

        // SH_NONE / undefined: put-only (see cfg_29 rationale)
        for &shmode in [STBDS_SH_NONE, 9].iter() {
            (lib.rand_seed)(0x31415926);
            let mut rng = Rng::new(0x4280 + shmode as u64);
            let pool = KeyPool::new(&mut rng, 5, 9, 30);
            let mut hm = Hm::from_shmode(lib, 16, 8, shmode);
            for i in 0..pool.len() {
                t.push(Ev::I(hm.put_str(
                    pool.ptr(i),
                    &(i as u64).to_le_bytes().to_vec(),
                    STBDS_HM_STRING,
                )));
                t.push(Ev::Arr(snap_arr(hm.raw(), 16)));
                t.push(Ev::Tbl(snap_table(hm.raw())));
            }
            hm.free();
        }
    });
}

// ===========================================================================
// Rows 43-45 — collision / probe-shape stress (added after the first pass:
// these force the wrap-around inner loops, deep probe chains, heavy tombstone
// churn and repeated table shrink/rebuild that the earlier rows only reach
// incidentally)
// ===========================================================================

#[test]
fn cfg_43_tiny_keyspace_stress() {
    // keysize == 1 => at most 256 distinct keys, so puts are dominated by the
    // "key already present" path and the table stays small while the array
    // churns; keysize == 2 gives a mid-sized keyspace.
    diff("cfg_43_tiny_keyspace_stress", |lib, t| unsafe {
        for &keysize in [1usize, 2].iter() {
            for &seed in [0usize, 0x31415926, usize::MAX].iter() {
                (lib.rand_seed)(seed);
                let mut rng = Rng::new(0x4300u64.wrapping_add(keysize as u64 * 31).wrapping_add(seed as u64));
                let elemsize = keysize + 8;
                let mut hm = Hm::new(lib, elemsize, keysize, 0);
                for i in 0..4000usize {
                    let k = rng.bytes(keysize);
                    match rng.below(10) {
                        0..=4 => {
                            t.push(Ev::I(hm.put_kv(
                                &k,
                                &(i as u64).to_le_bytes().to_vec(),
                                STBDS_HM_BINARY,
                            )));
                        }
                        5..=6 => t.push(Ev::I(hm.get(&k, STBDS_HM_BINARY))),
                        7 => t.push(Ev::I(hm.get_ts(&k, STBDS_HM_BINARY))),
                        _ => t.push(Ev::I(hm.del(&k, STBDS_HM_BINARY))),
                    }
                    if i % 11 == 0 {
                        let (a, tb) = hm.snap();
                        t.push(Ev::Arr(a));
                        t.push(Ev::Tbl(tb));
                    }
                }
                let (a, tb) = hm.snap();
                t.push(Ev::Arr(a));
                t.push(Ev::Tbl(tb));
                hm.free();
            }
        }
    });
}

#[test]
fn cfg_44_high_load_probe_shapes() {
    // Fill each table size to just below its growth threshold and keep it there,
    // so most probes traverse a bucket tail, wrap around to `i in 0..pos&7`, and
    // then step to the next bucket (`pos += step; step += 8`).
    diff("cfg_44_high_load_probe_shapes", |lib, t| unsafe {
        for &seed in SEEDS.iter() {
            (lib.rand_seed)(seed);
            let mut rng = Rng::new(0x4400u64.wrapping_add(seed as u64));
            let mut hm = Hm::new(lib, 16, 8, 0);
            let mut live: Vec<Vec<u8>> = Vec::new();
            // grow to a 1024-slot table
            for i in 0..760usize {
                let k = rng.bytes(8);
                hm.put_kv(&k, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_BINARY);
                live.push(k);
            }
            t.push(Ev::Tbl(snap_table(hm.raw())));
            // hover at high load: one delete + one insert, 3000 times
            for i in 0..3000usize {
                let j = rng.below(live.len());
                t.push(Ev::I(hm.del(&live[j], STBDS_HM_BINARY)));
                let k = rng.bytes(8);
                t.push(Ev::I(hm.put_kv(&k, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_BINARY)));
                live[j] = k;
                if i % 13 == 0 {
                    let (a, tb) = hm.snap();
                    t.push(Ev::Arr(a));
                    t.push(Ev::Tbl(tb));
                }
                // and a lookup of a present and an absent key
                t.push(Ev::I(hm.get(&live[rng.below(live.len())], STBDS_HM_BINARY)));
                let miss = rng.bytes(8);
                t.push(Ev::I(hm.get(&miss, STBDS_HM_BINARY)));
            }
            let (a, tb) = hm.snap();
            t.push(Ev::Arr(a));
            t.push(Ev::Tbl(tb));
            hm.free();
        }
    });
}

#[test]
fn cfg_45_string_high_load() {
    // The same high-load hovering for string maps, in all four `string.mode`s
    // that support lookups.
    diff("cfg_45_string_high_load", |lib, t| unsafe {
        for &shmode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA].iter() {
            (lib.rand_seed)(0x31415926);
            let mut rng = Rng::new(0x4500 + shmode as u64);
            let pool = KeyPool::new(&mut rng, 1200, 0, 60);
            let mut hm = Hm::from_shmode(lib, 16, 8, shmode);
            for i in 0..760usize {
                hm.put_str(pool.ptr(i), &(i as u64).to_le_bytes().to_vec(), STBDS_HM_STRING);
            }
            t.push(Ev::Tbl(snap_table(hm.raw())));
            for i in 0..1500usize {
                let a = rng.below(pool.len());
                let b = rng.below(pool.len());
                t.push(Ev::I(hm.del_str(pool.ptr(a), STBDS_HM_STRING)));
                t.push(Ev::I(hm.put_str(
                    pool.ptr(b),
                    &(i as u64).to_le_bytes().to_vec(),
                    STBDS_HM_STRING,
                )));
                t.push(Ev::Bytes(cstr_bytes(hm.temp_key())));
                t.push(Ev::I(hm.get_str(pool.ptr(rng.below(pool.len())), STBDS_HM_STRING)));
                if i % 17 == 0 {
                    t.push(Ev::Str(snap_str_elems(&hm)));
                    t.push(Ev::Tbl(snap_table(hm.raw())));
                }
            }
            t.push(Ev::Str(snap_str_elems(&hm)));
            t.push(Ev::Tbl(snap_table(hm.raw())));
            hm.free();
        }
    });
}
