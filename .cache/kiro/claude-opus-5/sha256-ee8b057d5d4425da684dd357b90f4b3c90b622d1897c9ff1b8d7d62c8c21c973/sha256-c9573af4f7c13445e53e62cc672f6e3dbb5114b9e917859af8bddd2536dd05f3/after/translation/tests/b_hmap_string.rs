//! Phase B — CONFIGS.md rows 48..64: the string-key hash map.  Covers all four
//! `STBDS_SH_*` key-storage modes and out-of-range `mode` values crossing the
//! FFI boundary.

mod common;
use common::*;
use std::ffi::c_void;

/// String-keyed map mirrored across both libraries.  The two sides get
/// *separate* key buffers because `STBDS_SH_DEFAULT` stores the caller's
/// pointer verbatim.
struct SMap<'a> {
    m: Map<'a>,
    keep_c: Keep,
    keep_r: Keep,
}

impl<'a> SMap<'a> {
    fn new(l: &'a Pair, elemsize: usize, mode: i32, shmode: i32, pay: Pay) -> SMap<'a> {
        SMap {
            m: Map::from_shmode(l, elemsize, 8, mode, shmode, pay),
            keep_c: Keep::new(),
            keep_r: Keep::new(),
        }
    }

    unsafe fn put(&mut self, key: &[u8], val: &[u8]) {
        let kc = self.keep_c.add(key);
        let kr = self.keep_r.add(key);
        self.m.put(kc, kr, val, 8);
    }

    unsafe fn get_ts(&mut self, key: &[u8]) -> (isize, isize) {
        let kc = self.keep_c.add(key);
        let kr = self.keep_r.add(key);
        self.m.get_ts(kc, kr)
    }

    unsafe fn del(&mut self, key: &[u8]) {
        let kc = self.keep_c.add(key);
        let kr = self.keep_r.add(key);
        self.m.del(kc, kr, 0);
    }

    unsafe fn assert_eq(&self, what: &str) {
        self.m.assert_eq(what);
    }

    unsafe fn assert_temp_key(&self, what: &str) {
        let a = temp_key_str(self.m.raw_c());
        let b = temp_key_str(self.m.raw_r());
        assert_eq!(a, b, "[{what}] temp_key string");
    }

    unsafe fn free(&mut self) {
        self.m.free();
    }
}

fn keys(rng: &mut Rng, n: usize, minlen: usize, maxlen: usize) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = vec![];
    let mut guard = 0;
    while out.len() < n {
        guard += 1;
        if guard > 100_000 {
            break;
        }
        let len = minlen + rng.below(maxlen - minlen + 1);
        let k = rng.cstring(len);
        if out.contains(&k) {
            continue;
        }
        out.push(k);
    }
    out
}

/// rows 48-51: `stbds_shmode_func` for each `STBDS_SH_*` value, several
/// `elemsize`s — the freshly created table must be identical.
#[test]
fn row_48_51_shmode_func() {
    let (l, _g) = libs();
    for &shmode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA].iter() {
        for &elemsize in [8usize, 16, 24, 32].iter() {
            seed_both(l, 0x2222);
            unsafe {
                let tc = (l.c.shmode_func)(elemsize, shmode);
                let tr = (l.r.shmode_func)(elemsize, shmode);
                assert_snap(
                    &snap_hash(tc, elemsize, Pay::Raw),
                    &snap_hash(tr, elemsize, Pay::Raw),
                    &format!("shmode_func(es={elemsize}, mode={shmode})"),
                );
                let s = snap_hash(tc, elemsize, Pay::Raw);
                let t = s.table.as_ref().unwrap();
                assert_eq!(t.slot_count, 8);
                assert_eq!(t.arena_mode, shmode as u8);
                assert_eq!(s.length, 1);
                (l.c.hmfree_func)((tc as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
                (l.r.hmfree_func)((tr as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

/// ERRORS.md row 50: out-of-range `mode` for `shmode_func` is truncated to
/// `unsigned char` with no validation.
#[test]
fn err_row_50_shmode_out_of_range() {
    let (l, _g) = libs();
    for &shmode in [4i32, 5, 127, 128, 200, 255, 256, 257, 511, -1, -128, i32::MIN, i32::MAX].iter() {
        seed_both(l, 0x3333);
        let elemsize = 16usize;
        unsafe {
            let tc = (l.c.shmode_func)(elemsize, shmode);
            let tr = (l.r.shmode_func)(elemsize, shmode);
            assert_snap(
                &snap_hash(tc, elemsize, Pay::Raw),
                &snap_hash(tr, elemsize, Pay::Raw),
                &format!("shmode_func out-of-range mode={shmode}"),
            );
            let s = snap_hash(tc, elemsize, Pay::Raw);
            assert_eq!(
                s.table.as_ref().unwrap().arena_mode,
                (shmode as u32 & 0xff) as u8,
                "mode={shmode} must be truncated to unsigned char"
            );
            (l.c.hmfree_func)((tc as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
            (l.r.hmfree_func)((tr as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// row 52 / ERRORS.md row 26: `string.mode == SH_NONE` with `mode == HM_STRING`
/// takes the `default:` `memcpy(elem, key, keysize)` branch — i.e. the first 8
/// bytes of the *string itself* are stored in the element.  Distinct keys only:
/// with this configuration a duplicate key would make `is_key_equal`
/// dereference string bytes as a pointer (in both libraries alike).
#[test]
fn row_52_sh_none_string_mode() {
    let (l, _g) = libs();
    for &elemsize in [8usize, 16].iter() {
        for trial in 0..4 {
            seed_both(l, 0x4444 + trial);
            let mut rng = Rng::new(0xD_0052 + elemsize as u64 + trial as u64 * 13);
            let ks = keys(&mut rng, 30, 9, 20); // >= 8 payload bytes so memcpy is in-bounds
            let mut sm = SMap::new(l, elemsize, HM_STRING, SH_NONE, Pay::Raw);
            unsafe {
                for (i, k) in ks.iter().enumerate() {
                    let v = rng.bytes(elemsize - 8);
                    sm.put(k, &v);
                    sm.assert_eq(&format!("SH_NONE es={elemsize} put {i}"));
                }
                sm.free();
            }
        }
    }
}

fn string_roundtrip(shmode: i32, mode: i32, elemsize: usize, n: usize, minlen: usize, maxlen: usize, tag: u64) {
    let (l, _g) = libs();
    seed_both(l, 0x5555 + tag as usize);
    let mut rng = Rng::new(0xD_0053 + tag);
    let ks = keys(&mut rng, n, minlen, maxlen);
    let mut sm = SMap::new(l, elemsize, mode, shmode, Pay::StrPtr);
    unsafe {
        for (i, k) in ks.iter().enumerate() {
            let v = rng.bytes(elemsize - 8);
            sm.put(k, &v);
            sm.assert_eq(&format!("shmode={shmode} mode={mode} n={n} put {i}"));
            sm.assert_temp_key(&format!("shmode={shmode} mode={mode} put {i}"));
        }
        // lookups
        for (i, k) in ks.iter().enumerate() {
            let (a, b) = sm.get_ts(k);
            assert_eq!(a, b, "shmode={shmode} mode={mode} get {i}");
            assert!(a >= 0, "shmode={shmode} mode={mode}: key {i} must be found");
            sm.assert_eq(&format!("shmode={shmode} mode={mode} get {i}"));
        }
        // absent lookups
        for _ in 0..100 {
            let k = rng.cstring(30);
            if ks.contains(&k) {
                continue;
            }
            let (a, b) = sm.get_ts(&k);
            assert_eq!(a, b, "absent");
            assert_eq!(a, -1);
        }
        // duplicate re-puts (row 60): hits the temp_key update path
        for (i, k) in ks.iter().enumerate() {
            let v = rng.bytes(elemsize - 8);
            sm.put(k, &v);
            sm.assert_eq(&format!("shmode={shmode} mode={mode} reput {i}"));
            sm.assert_temp_key(&format!("shmode={shmode} mode={mode} reput {i}"));
            let (sc, _) = sm.m.snaps();
            assert_eq!(sc.length, n + 1, "re-put must not grow the map");
        }
        sm.free();
    }
}

/// rows 53, 57: `SH_DEFAULT`, several sizes.
#[test]
fn row_53_57_sh_default() {
    for (i, &n) in [1usize, 2, 6, 7, 8, 13, 64].iter().enumerate() {
        for &elemsize in [8usize, 16, 24].iter() {
            string_roundtrip(SH_DEFAULT, HM_STRING, elemsize, n, 1, 24, 100 + i as u64 * 7 + elemsize as u64);
        }
    }
}

/// row 54: `SH_STRDUP`.
#[test]
fn row_54_sh_strdup() {
    for (i, &n) in [1usize, 6, 7, 8, 30, 64].iter().enumerate() {
        for &elemsize in [8usize, 16].iter() {
            string_roundtrip(SH_STRDUP, HM_STRING, elemsize, n, 0, 30, 300 + i as u64 * 11 + elemsize as u64);
        }
    }
}

/// row 55: `SH_ARENA`.
#[test]
fn row_55_sh_arena() {
    for (i, &n) in [1usize, 6, 7, 8, 30, 64].iter().enumerate() {
        for &elemsize in [8usize, 16].iter() {
            string_roundtrip(SH_ARENA, HM_STRING, elemsize, n, 0, 30, 500 + i as u64 * 13 + elemsize as u64);
        }
    }
}

/// row 56: `SH_ARENA` with keys long enough to blow past the arena blocksize
/// (`512 << (block>>1)`), mixing short and very long keys.
#[test]
fn row_56_sh_arena_long_keys() {
    let (l, _g) = libs();
    for trial in 0..4u64 {
        seed_both(l, 0x6666 + trial as usize);
        let mut rng = Rng::new(0xD_0056 + trial);
        let mut sm = SMap::new(l, 16, HM_STRING, SH_ARENA, Pay::StrPtr);
        let mut all: Vec<Vec<u8>> = vec![];
        unsafe {
            for i in 0..80 {
                let len = match i % 5 {
                    0 => 600 + rng.below(600),
                    1 => 511,
                    2 => 512,
                    3 => 513,
                    _ => rng.below(40),
                };
                let mut k = rng.cstring(len);
                // guarantee uniqueness
                let tagpos = k.len() - 1;
                k[tagpos] = b'A' + (i as u8 % 26);
                k.push(0);
                if all.contains(&k) {
                    continue;
                }
                all.push(k.clone());
                let v = rng.bytes(8);
                sm.put(&k, &v);
                sm.assert_eq(&format!("arena-long trial={trial} put {i} len={len}"));
                sm.assert_temp_key(&format!("arena-long trial={trial} put {i}"));
            }
            for (i, k) in all.iter().enumerate() {
                let (a, b) = sm.get_ts(k);
                assert_eq!(a, b, "arena-long get {i}");
                assert!(a >= 0);
            }
            sm.assert_eq("arena-long final");
            sm.free();
        }
    }
}

/// rows 58-59 / ERRORS.md row 36: deletes in `SH_STRDUP` (frees the dup) and
/// `SH_ARENA` (does not), including the compaction + re-find path.
#[test]
fn row_58_59_string_deletes() {
    let (l, _g) = libs();
    for &shmode in [SH_DEFAULT, SH_STRDUP, SH_ARENA].iter() {
        for &n in [1usize, 2, 7, 8, 30, 64].iter() {
            for &elemsize in [8usize, 16].iter() {
                seed_both(l, 0x7777);
                let mut rng = Rng::new(0xD_0058 + (shmode as u64) * 101 + n as u64 + elemsize as u64);
                let ks = keys(&mut rng, n, 0, 25);
                let mut sm = SMap::new(l, elemsize, HM_STRING, shmode, Pay::StrPtr);
                unsafe {
                    for k in ks.iter() {
                        let v = rng.bytes(elemsize - 8);
                        sm.put(k, &v);
                    }
                    // delete in a shuffled order so both "last" and "middle"
                    // (compaction + re-find) branches are hit
                    let mut order: Vec<usize> = (0..ks.len()).collect();
                    for i in (1..order.len()).rev() {
                        let j = rng.below(i + 1);
                        order.swap(i, j);
                    }
                    for (step, &idx) in order.iter().enumerate() {
                        sm.del(&ks[idx]);
                        sm.assert_eq(&format!("shmode={shmode} n={n} es={elemsize} del step {step}"));
                        // survivors still reachable
                        for (j, k) in ks.iter().enumerate() {
                            if order[..=step].contains(&j) {
                                continue;
                            }
                            let (a, b) = sm.get_ts(k);
                            assert_eq!(a, b, "shmode={shmode} survivor {j} after step {step}");
                            assert!(a >= 0, "shmode={shmode} survivor {j} vanished");
                        }
                    }
                    let (sc, _) = sm.m.snaps();
                    assert_eq!(sc.length, 1);
                    sm.free();
                }
            }
        }
    }
}

/// row 61 / ERRORS.md row 52: `mode == 2` (out-of-range enum value, still
/// `>= STBDS_HM_STRING`) with `SH_DEFAULT` storage — string path.
#[test]
fn row_61_mode_two_default() {
    for (i, &m) in [2i32, 3, 17, 1000, i32::MAX].iter().enumerate() {
        string_roundtrip(SH_DEFAULT, m, 16, 30, 1, 20, 700 + i as u64);
    }
}

/// row 62 / ERRORS.md rows 36-37: `mode == 2` with `SH_STRDUP`.  The
/// `mode == STBDS_HM_STRING` guard is exact, so no key is freed; the re-find
/// after compaction would pass the *element address* to `strcmp`, which cannot
/// match and would trip `STBDS_ASSERT(slot >= 0)` in the C.  Only the
/// `old_index == final_index` (delete-the-last-element) branch is reachable
/// without aborting, so that is what is compared.
#[test]
fn row_62_mode_two_strdup_delete_last() {
    let (l, _g) = libs();
    for &m in [2i32, 5, 1000].iter() {
        for &n in [1usize, 3, 8, 20].iter() {
            seed_both(l, 0x8888);
            let mut rng = Rng::new(0xD_0062 + m as u64 + n as u64);
            let ks = keys(&mut rng, n, 1, 20);
            let mut sm = SMap::new(l, 16, m, SH_STRDUP, Pay::StrPtr);
            unsafe {
                for k in ks.iter() {
                    let v = rng.bytes(8);
                    sm.put(k, &v);
                }
                // delete in reverse insertion order: always the last element
                for (step, k) in ks.iter().enumerate().rev() {
                    sm.del(k);
                    sm.assert_eq(&format!("mode={m} n={n} del-last step {step}"));
                }
                let (sc, _) = sm.m.snaps();
                assert_eq!(sc.length, 1);
                sm.free();
            }
        }
    }
}

/// row 63 / ERRORS.md row 51: negative `mode` -> binary path (`memcmp`) even
/// though the table was created by `shmode_func`.
///
/// Part A uses `SH_NONE`, where the `default:` `memcpy` branch stores the key
/// bytes in the element, so binary lookups actually resolve.  Part B uses the
/// pointer-storing modes, where the element holds a `char *`; a binary
/// `memcmp` of the key against those pointer bytes can never match, so every
/// lookup and delete must miss — identically in both libraries.
#[test]
fn row_63_negative_mode() {
    let (l, _g) = libs();

    // --- Part A: SH_NONE, binary semantics end to end -------------------
    for &m in [-1i32, -2, -1000, i32::MIN].iter() {
        seed_both(l, 0x9999);
        let mut rng = Rng::new(0xD_0063 ^ (m as i64 as u64));
        let elemsize = 16usize;
        let keysize = 8usize;
        let mut m2 = Map::from_shmode(l, elemsize, keysize, m, SH_NONE, Pay::Raw);
        unsafe {
            let mut inserted: Vec<Vec<u8>> = vec![];
            for i in 0..30 {
                let mut k = rng.bytes(keysize);
                if inserted.contains(&k) {
                    continue;
                }
                inserted.push(k.clone());
                let v = rng.bytes(elemsize - keysize);
                let kp = k.as_mut_ptr() as *mut c_void;
                m2.put(kp, kp, &v, keysize);
                m2.assert_eq(&format!("mode={m} SH_NONE put {i}"));
            }
            for k in inserted.iter() {
                let mut kb = k.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                let (a, b) = m2.get_ts(kp, kp);
                assert_eq!(a, b, "mode={m} SH_NONE get");
                assert!(a >= 0, "mode={m} SH_NONE: binary key must be found");
            }
            for (i, k) in inserted.iter().enumerate() {
                let mut kb = k.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                m2.del(kp, kp, 0);
                m2.assert_eq(&format!("mode={m} SH_NONE del {i}"));
            }
            let (sc, _) = m2.snaps();
            assert_eq!(sc.length, 1, "mode={m}: all entries deleted");
            m2.free();
        }
    }

    // --- Part B: pointer-storing modes, binary lookups must all miss ----
    for &m in [-1i32, -7, i32::MIN].iter() {
        for &shmode in [SH_DEFAULT, SH_STRDUP, SH_ARENA].iter() {
            seed_both(l, 0x9A9A);
            let mut rng = Rng::new(0xD_1063 ^ (m as i64 as u64) ^ (shmode as u64) * 7);
            // Binary mode hashes/compares exactly `keysize` (8) bytes, so the
            // key buffers must be at least 8 bytes long and byte-identical on
            // both sides.
            let ks = keys(&mut rng, 25, 8, 20);
            let mut sm = SMap::new(l, 16, m, shmode, Pay::StrPtr);
            unsafe {
                for (i, k) in ks.iter().enumerate() {
                    let v = rng.bytes(8);
                    sm.put(k, &v);
                    sm.assert_eq(&format!("mode={m} shmode={shmode} put {i}"));
                }
                for (i, k) in ks.iter().enumerate() {
                    let (a, b) = sm.get_ts(k);
                    assert_eq!(a, b, "mode={m} shmode={shmode} get {i}");
                    assert_eq!(
                        a, -1,
                        "mode={m} shmode={shmode}: a binary memcmp against a stored char* must miss"
                    );
                }
                for (i, k) in ks.iter().enumerate() {
                    sm.del(k);
                    sm.assert_eq(&format!("mode={m} shmode={shmode} del {i}"));
                    let (sc, _) = sm.m.snaps();
                    assert_eq!(sc.temp, 0, "missed delete must leave temp == 0");
                    assert_eq!(sc.length, ks.len() + 1, "missed delete must not shrink");
                }
                sm.free();
            }
        }
    }
}

/// row 64: very large positive `mode` values on the string path, with deletes
/// restricted to the safe (last-element) branch — see row 62.
#[test]
fn row_64_huge_mode_string_path() {
    let (l, _g) = libs();
    for &m in [1000i32, 0x7FFF, i32::MAX].iter() {
        seed_both(l, 0xAAAA);
        let mut rng = Rng::new(0xD_0064 + m as u64);
        let ks = keys(&mut rng, 40, 1, 30);
        let mut sm = SMap::new(l, 24, m, SH_ARENA, Pay::StrPtr);
        unsafe {
            for (i, k) in ks.iter().enumerate() {
                let v = rng.bytes(16);
                sm.put(k, &v);
                sm.assert_eq(&format!("mode={m} put {i}"));
                sm.assert_temp_key(&format!("mode={m} put {i}"));
            }
            for (i, k) in ks.iter().enumerate() {
                let (a, b) = sm.get_ts(k);
                assert_eq!(a, b, "mode={m} get {i}");
                assert!(a >= 0);
            }
            for (step, k) in ks.iter().enumerate().rev() {
                sm.del(k);
                sm.assert_eq(&format!("mode={m} del-last {step}"));
            }
            sm.free();
        }
    }
}

/// row 77: end-to-end pipeline on `SH_STRDUP` — reseed, 100 randomized puts,
/// gets, 50 deletes, gets, then `hmfree_func`, comparing after every step.
#[test]
fn row_77_end_to_end_strdup() {
    let (l, _g) = libs();
    for trial in 0..3u64 {
        seed_both(l, (0xBBBB + trial) as usize);
        let mut rng = Rng::new(0xD_0077 + trial);
        let pool = keys(&mut rng, 120, 0, 40);
        let mut sm = SMap::new(l, 16, HM_STRING, SH_STRDUP, Pay::StrPtr);
        let mut live: Vec<usize> = vec![];
        unsafe {
            for op in 0..400 {
                let idx = rng.below(pool.len());
                match rng.below(10) {
                    0..=5 => {
                        let v = rng.bytes(8);
                        sm.put(&pool[idx], &v);
                        if !live.contains(&idx) {
                            live.push(idx);
                        }
                        sm.assert_temp_key(&format!("e2e trial={trial} op={op} put"));
                    }
                    6..=7 => {
                        let (a, b) = sm.get_ts(&pool[idx]);
                        assert_eq!(a, b, "e2e trial={trial} op={op} get");
                        if live.contains(&idx) {
                            assert!(a >= 0, "e2e op={op}: live key must be found");
                        } else {
                            assert_eq!(a, -1, "e2e op={op}: dead key must be absent");
                        }
                    }
                    _ => {
                        sm.del(&pool[idx]);
                        live.retain(|&x| x != idx);
                    }
                }
                sm.assert_eq(&format!("e2e trial={trial} op={op}"));
            }
            let (sc, _) = sm.m.snaps();
            assert_eq!(sc.length, live.len() + 1);
            sm.free();
        }
    }
}
