//! Phase B rows 35-46, 48-50: the string-key hash map in all four
//! `string.mode` configurations (`SH_DEFAULT`, `SH_STRDUP`, `SH_ARENA`, and an
//! out-of-enum-range mode), driven through `stbds_shmode_func` +
//! `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmdel_key`.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

const SEED: u64 = 0x5EED_1234;

fn rand_key(rng: &mut Rng, len: usize) -> String {
    (0..len)
        .map(|_| (b'a' + (rng.byte() % 26)) as char)
        .collect()
}

#[test]
fn cfg_35_str_default_single() {
    let h = setup(0x3141_5926);
    let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
    let mut keys = Keys::new();
    let k = keys.add_str("hello");
    let t = m.put(k, &[], HM_STRING, 0x22);
    assert_eq!(t, 0);
    m.check("str default single");
    unsafe {
        // string.mode must have been set to STBDS_SH_DEFAULT (1) implicitly
        for mm in [m.mc, m.mr] {
            let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
            let tb = &*(hd.hash_table as *const HashIndex);
            assert_eq!(tb.string.mode, 1, "implicit string.mode must be SH_DEFAULT");
        }
    }
    assert_eq!(m.get(k, HM_STRING), 0);
    m.check("str default single get");
    m.free();
}

#[test]
fn cfg_36_str_default_many() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 36);
    let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
    let mut keys = Keys::new();
    let mut names: Vec<String> = Vec::new();
    for i in 0..250u64 {
        let n = 1 + rng.below(12);
        let s = format!("{}_{}", rand_key(&mut rng, n), i);
        let k = keys.add_str(&s);
        m.put(k, &[], HM_STRING, (i & 0xff) as u8);
        names.push(s);
        if i % 19 == 0 {
            m.check(&format!("str default many {}", i));
        }
    }
    m.check("str default many final");
    for s in &names {
        let k = keys.add_str(s);
        // NOTE: SH_DEFAULT stores the caller pointer; a *different* pointer
        // with the same contents must still be found (strcmp, not ==).
        assert!(m.get(k, HM_STRING) >= 0, "key {} vanished", s);
    }
    m.check("str default many gets");
    m.free();
}

#[test]
fn cfg_37_str_default_dup() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 37);
    let mut m = MapPair::new(&h.c, &h.r, 24, 8, true);
    let mut keys = Keys::new();
    let pool: Vec<String> = (0..30).map(|i| format!("dup_key_{}", i)).collect();
    for i in 0..500u64 {
        let s = &pool[rng.below(pool.len())];
        // fresh buffer each time -> the update path must re-read the *stored*
        // pointer for temp_key
        let k = keys.add_str(s);
        m.put(k, &[], HM_STRING, (i & 0xff) as u8);
        if i % 17 == 0 {
            m.check(&format!("str dup op {}", i));
        }
        if keys.bufs.len() > 4000 {
            // keep the SH_DEFAULT pointers alive: never drop buffers here
        }
    }
    m.check("str dup final");
    m.free();
}

#[test]
fn cfg_38_str_default_get() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 38);
    let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
    let mut keys = Keys::new();
    let present: Vec<String> = (0..200).map(|i| format!("k{}", i * 3)).collect();
    for (i, s) in present.iter().enumerate() {
        let k = keys.add_str(s);
        m.put(k, &[], HM_STRING, i as u8);
    }
    for s in &present {
        let k = keys.add_str(s);
        assert!(m.get(k, HM_STRING) >= 0);
        assert!(m.get_ts(k, HM_STRING) >= 0);
    }
    for _ in 0..400 {
        let s = rand_key(&mut rng, 20);
        let k = keys.add_str(&s);
        assert_eq!(m.get(k, HM_STRING), -1);
        assert_eq!(m.get_ts(k, HM_STRING), -1);
    }
    m.check("str default get");
    m.free();
}

#[test]
fn cfg_39_str_default_del() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 39);
    let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
    let mut keys = Keys::new();
    let names: Vec<String> = (0..120).map(|i| format!("del_{}", i)).collect();
    for (i, s) in names.iter().enumerate() {
        let k = keys.add_str(s);
        m.put(k, &[], HM_STRING, i as u8);
    }
    m.check("str del setup");
    // absent
    let k = keys.add_str("no_such_key_at_all");
    assert_eq!(m.del(k, 0, HM_STRING), 0);
    m.check("str del absent");
    // random order deletes: exercises swap + strcmp re-find, tombstones, shrink
    let mut order: Vec<usize> = (0..names.len()).collect();
    for i in (1..order.len()).rev() {
        let j = rng.below(i + 1);
        order.swap(i, j);
    }
    for &i in &order {
        let k = keys.add_str(&names[i]);
        assert_eq!(m.del(k, 0, HM_STRING), 1, "delete of {} failed", names[i]);
        m.check(&format!("str del {}", names[i]));
    }
    m.free();
}

#[test]
fn cfg_40_str_strdup_many() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 40);
    let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
    m.shmode(SH_STRDUP);
    m.check("strdup fresh");
    let mut keys = Keys::new();
    let mut names: Vec<String> = Vec::new();
    for i in 0..250u64 {
        let n = 1 + rng.below(20);
        let s = format!("{}#{}", rand_key(&mut rng, n), i);
        let k = keys.add_str(&s);
        m.put(k, &[], HM_STRING, (i & 0xff) as u8);
        names.push(s);
        if i % 19 == 0 {
            m.check(&format!("strdup put {}", i));
        }
    }
    m.check("strdup final");
    for s in &names {
        let k = keys.add_str(s);
        assert!(m.get(k, HM_STRING) >= 0);
    }
    m.check("strdup gets");
    m.free();
}

#[test]
fn cfg_41_str_strdup_del() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 41);
    let mut m = MapPair::new(&h.c, &h.r, 24, 8, true);
    m.shmode(SH_STRDUP);
    let mut keys = Keys::new();
    let names: Vec<String> = (0..150).map(|i| format!("strdup_del_{}", i)).collect();
    for (i, s) in names.iter().enumerate() {
        let k = keys.add_str(s);
        m.put(k, &[], HM_STRING, i as u8);
    }
    m.check("strdup del setup");
    let mut order: Vec<usize> = (0..names.len()).collect();
    for i in (1..order.len()).rev() {
        let j = rng.below(i + 1);
        order.swap(i, j);
    }
    for &i in &order {
        let k = keys.add_str(&names[i]);
        assert_eq!(m.del(k, 0, HM_STRING), 1);
        m.check(&format!("strdup del {}", names[i]));
    }
    // reinsert after full drain
    for (i, s) in names.iter().enumerate().take(20) {
        let k = keys.add_str(s);
        m.put(k, &[], HM_STRING, i as u8);
        m.check("strdup reinsert");
    }
    m.free();
}

#[test]
fn cfg_42_str_strdup_free() {
    // hmfree_func walks i=1..length freeing every strdup'd key.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 42);
    for n in [0usize, 1, 5, 6, 7, 50, 300] {
        let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
        m.shmode(SH_STRDUP);
        let mut keys = Keys::new();
        for i in 0..n {
            let s = format!("{}~{}", rand_key(&mut rng, 8), i);
            let k = keys.add_str(&s);
            m.put(k, &[], HM_STRING, i as u8);
        }
        m.check(&format!("strdup free n={}", n));
        m.free();
        m.check("strdup after free");
    }
}

#[test]
fn cfg_43_str_arena_short() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 43);
    let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
    m.shmode(SH_ARENA);
    m.check("arena fresh");
    let mut keys = Keys::new();
    let mut names = Vec::new();
    for i in 0..30u64 {
        let s = format!("{}!{}", rand_key(&mut rng, 4), i);
        let k = keys.add_str(&s);
        m.put(k, &[], HM_STRING, (i & 0xff) as u8);
        names.push(s);
        m.check(&format!("arena short put {}", i));
    }
    for s in &names {
        let k = keys.add_str(s);
        assert!(m.get(k, HM_STRING) >= 0);
    }
    m.check("arena short gets");
    m.free();
}

#[test]
fn cfg_44_str_arena_multiblock() {
    // Force several arena blocks: block 0 -> 512 bytes, then 512, 1024, ...
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 44);
    let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
    m.shmode(SH_ARENA);
    let mut keys = Keys::new();
    let mut names = Vec::new();
    for i in 0..400u64 {
        let n = 40 + rng.below(40);
        let s = format!("{}@{}", rand_key(&mut rng, n), i);
        let k = keys.add_str(&s);
        m.put(k, &[], HM_STRING, (i & 0xff) as u8);
        names.push(s);
        if i % 13 == 0 {
            m.check(&format!("arena multiblock put {}", i));
        }
    }
    m.check("arena multiblock final");
    for s in &names {
        let k = keys.add_str(s);
        assert!(m.get(k, HM_STRING) >= 0, "arena key {} vanished", s);
    }
    m.check("arena multiblock gets");
    m.free();
}

#[test]
fn cfg_45_str_arena_oversized() {
    // A key longer than the current blocksize takes the "oversized single
    // block" splice path inside stbds_stralloc.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 45);
    for pre in [0usize, 1, 5] {
        let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
        m.shmode(SH_ARENA);
        let mut keys = Keys::new();
        for i in 0..pre {
            let s = format!("pre{}", i);
            let k = keys.add_str(&s);
            m.put(k, &[], HM_STRING, i as u8);
        }
        for len in [600usize, 1000, 4096] {
            let s = rand_key(&mut rng, len);
            let k = keys.add_str(&s);
            m.put(k, &[], HM_STRING, 0x55);
            m.check(&format!("arena oversized pre={} len={}", pre, len));
            let k2 = keys.add_str(&s);
            assert!(m.get(k2, HM_STRING) >= 0);
        }
        m.check(&format!("arena oversized pre={} final", pre));
        m.free();
    }
}

#[test]
fn cfg_46_str_arena_free() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 46);
    for n in [0usize, 1, 10, 200] {
        let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
        m.shmode(SH_ARENA);
        let mut keys = Keys::new();
        for i in 0..n {
            let s = format!("{}${}", rand_key(&mut rng, 30), i);
            let k = keys.add_str(&s);
            m.put(k, &[], HM_STRING, i as u8);
        }
        m.check(&format!("arena free n={}", n));
        m.free();
        m.check("arena after free");
    }
}

#[test]
fn cfg_48_shmode_four() {
    // string.mode == 4 is out of the enum range: the `switch` in hmput_key
    // falls through to `default:` -> plain memcpy of `keysize` key bytes,
    // even though `mode` selects the string comparison path. Keys are stored
    // as raw bytes, so render them as bytes (they are NOT pointers).
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 48);
    let keysize = 8usize;
    let mut m = MapPair::new(&h.c, &h.r, 16, keysize, false);
    m.shmode(4);
    unsafe {
        for mm in [m.mc, m.mr] {
            let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
            let tb = &*(hd.hash_table as *const HashIndex);
            assert_eq!(tb.string.mode, 4);
        }
    }
    m.check("shmode 4 fresh");
    // Distinct keys only: with distinct hashes `stbds_is_key_equal` (which
    // would strcmp a raw byte blob as a pointer) is never reached.
    let mut used: Vec<String> = Vec::new();
    let mut i = 0u64;
    while used.len() < 5 {
        let s = format!("{:07}", i * 1_000_003);
        i += 1;
        if used.iter().any(|u| u.as_bytes()[..keysize.min(u.len())] == s.as_bytes()[..keysize.min(s.len())]) {
            continue;
        }
        let k = keys_prefix(&s, keysize);
        let mut owner = Keys::new();
        let kp = owner.add_bytes(&k);
        m.put(kp, &k, HM_STRING, (used.len() as u8) | 0x40);
        std::mem::forget(owner);
        m.check(&format!("shmode 4 put {}", s));
        used.push(s);
    }
    let _ = &mut rng;
    m.free();
}

fn keys_prefix(s: &str, keysize: usize) -> Vec<u8> {
    let mut v: Vec<u8> = s.as_bytes().to_vec();
    v.push(0);
    v.resize(keysize.max(v.len()), 0);
    v.truncate(keysize.max(1));
    v
}

#[test]
fn cfg_49_shmode_truncation() {
    // `h->string.mode = (unsigned char) mode;` -- out-of-range enum values
    // must truncate identically.
    let h = setup(0x3141_5926);
    for mode in [
        0 as c_int,
        1,
        2,
        3,
        4,
        5,
        127,
        128,
        255,
        256,
        257,
        511,
        -1,
        -2,
        -255,
        -256,
        c_int::MAX,
        c_int::MIN,
    ] {
        for elemsize in [8usize, 16] {
            let mut m = MapPair::new(&h.c, &h.r, elemsize, 8, false);
            m.shmode(mode);
            let expect = (mode as u32 & 0xff) as u8;
            unsafe {
                for (nm, mm) in [("C", m.mc), ("RUST", m.mr)] {
                    let hd = &*header_of((mm as *mut u8).sub(elemsize) as *mut c_void);
                    let tb = &*(hd.hash_table as *const HashIndex);
                    assert_eq!(
                        tb.string.mode, expect,
                        "{} shmode_func({}) truncation wrong",
                        nm, mode
                    );
                }
            }
            m.check(&format!("shmode truncation mode={} e={}", mode, elemsize));
            m.free();
        }
    }
}

#[test]
fn cfg_50_mode_gt_one() {
    // mode > STBDS_HM_STRING: `mode >= STBDS_HM_STRING` selects the string
    // path in put/get, and `nt->string.mode` becomes SH_DEFAULT.
    let h = setup(0x3141_5926);
    for mode in [2 as c_int, 3, 7, 127, c_int::MAX] {
        let mut rng = Rng::new(SEED ^ 50 ^ (mode as u64));
        let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
        let mut keys = Keys::new();
        let names: Vec<String> = (0..120)
            .map(|i| format!("{}%{}", rand_key(&mut rng, 6), i))
            .collect();
        for (i, s) in names.iter().enumerate() {
            let k = keys.add_str(s);
            m.put(k, &[], mode, i as u8);
        }
        m.check(&format!("mode={} put", mode));
        unsafe {
            for mm in [m.mc, m.mr] {
                let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
                let tb = &*(hd.hash_table as *const HashIndex);
                assert_eq!(tb.string.mode, 1, "mode>1 must still yield SH_DEFAULT");
            }
        }
        for s in &names {
            let k = keys.add_str(s);
            assert!(m.get(k, mode) >= 0);
            assert!(m.get_ts(k, mode) >= 0);
        }
        m.check(&format!("mode={} get", mode));
        // Deleting the LAST element skips the swap+re-find, which for mode != 1
        // hashes raw pointer bytes (see ERRORS.md row 19 / err_19_del_mode_two).
        for i in (0..names.len()).rev() {
            let k = keys.add_str(&names[i]);
            let t = m.del(k, 0, mode);
            m.check(&format!("mode={} del last {}", mode, i));
            if t != 1 {
                break;
            }
        }
        m.free();
    }
}
