//! Phase B — CONFIGS.md rows 24..47: the binary-key hash map, driven through
//! the low-level `stbds_hmput_key` / `stbds_hmget_key(_ts)` / `stbds_hmdel_key`
//! exports (i.e. what the `hmput`/`hmget`/`hmdel` macros expand to).

mod common;
use common::*;
use std::ffi::c_void;

/// Build the key/value buffers for one entry: `elemsize == keysize + valsize`
/// so that every byte of every live element is caller-initialised and the
/// payload comparison is meaningful.
struct Kv {
    key: Vec<u8>,
    val: Vec<u8>,
}

fn make_kvs(rng: &mut Rng, n: usize, keysize: usize, valsize: usize, distinct: bool) -> Vec<Kv> {
    let mut out: Vec<Kv> = vec![];
    let mut seen: Vec<Vec<u8>> = vec![];
    let mut guard = 0;
    while out.len() < n {
        guard += 1;
        if guard > 100_000 {
            break;
        }
        let key = rng.bytes(keysize);
        if distinct && keysize > 0 && seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        out.push(Kv {
            val: rng.bytes(valsize),
            key,
        });
    }
    out
}

fn bin_map<'a>(l: &'a Pair, keysize: usize, valsize: usize) -> Map<'a> {
    Map::new(l, keysize + valsize, keysize, HM_BINARY, Pay::Raw)
}

/// rows 24-26: 1, 2, 5, 6, 7, 8, 64, 200 distinct keys — crosses the 8-slot
/// `used_count_threshold == 6` rehash and several further doublings.
#[test]
fn row_24_26_put_counts() {
    let (l, _g) = libs();
    for &n in [1usize, 2, 3, 4, 5, 6, 7, 8, 9, 12, 13, 16, 64, 200].iter() {
        for trial in 0..4 {
            seed_both(l, 0x3141_5926 + trial);
            let mut rng = Rng::new(0xC_0024 + (n as u64) * 977 + trial as u64);
            let kvs = make_kvs(&mut rng, n, 4, 4, true);
            let mut m = bin_map(l, 4, 4);
            unsafe {
                for (i, kv) in kvs.iter().enumerate() {
                    let mut k = kv.key.clone();
                    let kp = k.as_mut_ptr() as *mut c_void;
                    m.put(kp, kp, &kv.val, 4);
                    m.assert_eq(&format!("n={n} trial={trial} after put {i}"));
                }
                // read every key back
                for (i, kv) in kvs.iter().enumerate() {
                    let mut k = kv.key.clone();
                    let kp = k.as_mut_ptr() as *mut c_void;
                    let (a, b) = m.get_ts(kp, kp);
                    assert_eq!(a, b, "n={n} get {i}: C temp={a} Rust temp={b}");
                    m.assert_eq(&format!("n={n} after get {i}"));
                }
                m.free();
            }
        }
    }
}

/// row 27: duplicate keys re-put — `temp` updated, `length` unchanged.
#[test]
fn row_27_duplicate_puts() {
    let (l, _g) = libs();
    seed_both(l, 7);
    let mut rng = Rng::new(0xC_0027);
    let kvs = make_kvs(&mut rng, 10, 4, 4, true);
    let mut m = bin_map(l, 4, 4);
    unsafe {
        for kv in kvs.iter() {
            let mut k = kv.key.clone();
            m.put(k.as_mut_ptr() as *mut c_void, k.as_mut_ptr() as *mut c_void, &kv.val, 4);
        }
        m.assert_eq("after initial 10 puts");
        // now re-put each key several times with new values
        for round in 0..5 {
            for (i, kv) in kvs.iter().enumerate() {
                let mut k = kv.key.clone();
                let v = rng.bytes(4);
                let kp = k.as_mut_ptr() as *mut c_void;
                m.put(kp, kp, &v, 4);
                m.assert_eq(&format!("dup round={round} key={i}"));
            }
        }
        let (sc, _) = m.snaps();
        assert_eq!(sc.length, 11, "length must not grow on duplicate puts");
        m.free();
    }
}

/// row 28: `keysize` x `elemsize` matrix.
#[test]
fn row_28_keysize_elemsize_matrix() {
    let (l, _g) = libs();
    for &keysize in [1usize, 2, 3, 4, 8, 16].iter() {
        for &valsize in [0usize, 1, 4, 8, 16].iter() {
            seed_both(l, 0x1234_5678);
            let mut rng = Rng::new(0xC_0028 + (keysize * 100 + valsize) as u64);
            // small key spaces cannot supply many distinct keys
            let n = if keysize == 1 { 40 } else { 30 };
            let kvs = make_kvs(&mut rng, n, keysize, valsize, true);
            let mut m = bin_map(l, keysize, valsize);
            unsafe {
                for (i, kv) in kvs.iter().enumerate() {
                    let mut k = kv.key.clone();
                    let kp = k.as_mut_ptr() as *mut c_void;
                    m.put(kp, kp, &kv.val, keysize);
                    m.assert_eq(&format!("ks={keysize} vs={valsize} put {i}"));
                }
                for kv in kvs.iter() {
                    let mut k = kv.key.clone();
                    let kp = k.as_mut_ptr() as *mut c_void;
                    let (a, b) = m.get(kp, kp);
                    assert_eq!(a, b, "ks={keysize} vs={valsize} get");
                }
                m.assert_eq(&format!("ks={keysize} vs={valsize} final"));
                m.free();
            }
        }
    }
}

/// row 29 / ERRORS.md row 53: `keysize == 0` — `memcmp(...,0) == 0`, so every
/// key compares equal and the map can never hold more than one entry.
#[test]
fn row_29_keysize_zero() {
    let (l, _g) = libs();
    for &valsize in [0usize, 4, 8].iter() {
        seed_both(l, 99);
        let mut rng = Rng::new(0xC_0029 + valsize as u64);
        let mut m = bin_map(l, 0, valsize);
        unsafe {
            for i in 0..20 {
                let mut k = rng.bytes(8); // buffer is bigger than keysize on purpose
                let v = rng.bytes(valsize);
                let kp = k.as_mut_ptr() as *mut c_void;
                m.put(kp, kp, &v, 0);
                m.assert_eq(&format!("keysize=0 vs={valsize} put {i}"));
            }
            let (sc, _) = m.snaps();
            assert_eq!(sc.length, 2, "keysize=0 must collapse to a single entry");
            let mut k = rng.bytes(8);
            let kp = k.as_mut_ptr() as *mut c_void;
            let (a, b) = m.get_ts(kp, kp);
            assert_eq!(a, b);
            assert_eq!(a, 0, "the single entry lives at index 0");
            m.del(kp, kp, 0);
            m.assert_eq("keysize=0 after del");
            m.free();
        }
    }
}

/// row 30 / row 76: reseed the global LCG so the table `seed` (and therefore the
/// whole probe order) differs, then rebuild the same map.
#[test]
fn row_30_76_global_seed_variation() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xC_0030);
    for &seed in [0usize, 1, 2, 0x3141_5926, usize::MAX, 0xF00D_BAAD].iter() {
        seed_both(l, seed);
        let kvs = make_kvs(&mut rng, 64, 4, 4, true);
        let mut m = bin_map(l, 4, 4);
        unsafe {
            for (i, kv) in kvs.iter().enumerate() {
                let mut k = kv.key.clone();
                let kp = k.as_mut_ptr() as *mut c_void;
                m.put(kp, kp, &kv.val, 4);
                m.assert_eq(&format!("seed={seed:#x} put {i}"));
            }
            // second map in the same session: its seed comes from the LCG-advanced global
            let mut m2 = bin_map(l, 4, 4);
            for kv in kvs.iter().take(20) {
                let mut k = kv.key.clone();
                let kp = k.as_mut_ptr() as *mut c_void;
                m2.put(kp, kp, &kv.val, 4);
            }
            m2.assert_eq(&format!("seed={seed:#x} second map (LCG-advanced global)"));
            let (s1, _) = m.snaps();
            let (s2, _) = m2.snaps();
            assert_ne!(
                s1.table.as_ref().unwrap().seed,
                s2.table.as_ref().unwrap().seed,
                "the LCG must advance the global seed"
            );
            m.free();
            m2.free();
        }
    }
    seed_both(l, 0x3141_5926);
}

/// rows 31-32 / ERRORS.md rows 14-15: `hmget_key_ts` present vs absent key.
#[test]
fn row_31_32_get_ts_present_absent() {
    let (l, _g) = libs();
    for &n in [1usize, 6, 7, 20, 100].iter() {
        seed_both(l, 4242);
        let mut rng = Rng::new(0xC_0031 + n as u64);
        let kvs = make_kvs(&mut rng, n, 4, 4, true);
        let mut m = bin_map(l, 4, 4);
        unsafe {
            for kv in kvs.iter() {
                let mut k = kv.key.clone();
                let kp = k.as_mut_ptr() as *mut c_void;
                m.put(kp, kp, &kv.val, 4);
            }
            for kv in kvs.iter() {
                let mut k = kv.key.clone();
                let kp = k.as_mut_ptr() as *mut c_void;
                let (a, b) = m.get_ts(kp, kp);
                assert_eq!(a, b, "present key");
                assert!(a >= 0, "present key must resolve");
            }
            for _ in 0..500 {
                let mut k = rng.bytes(4);
                if kvs.iter().any(|kv| kv.key == k) {
                    continue;
                }
                let kp = k.as_mut_ptr() as *mut c_void;
                let (a, b) = m.get_ts(kp, kp);
                assert_eq!(a, b, "absent key");
                assert_eq!(a, -1, "absent key must yield STBDS_INDEX_EMPTY");
            }
            m.assert_eq(&format!("n={n} after lookups"));
            m.free();
        }
    }
}

/// rows 33-35 / ERRORS.md rows 12-13, 16: bootstrap paths.
#[test]
fn row_33_35_get_bootstrap_paths() {
    let (l, _g) = libs();
    seed_both(l, 11);
    let elemsize = 8usize;
    let keysize = 4usize;
    let mut key = vec![1u8, 2, 3, 4];
    let kp = key.as_mut_ptr() as *mut c_void;

    unsafe {
        // (a) a == NULL, hmget_key_ts
        let mut tc: isize = 0x5A;
        let mut tr: isize = 0x5A;
        let pc = (l.c.hmget_key_ts)(std::ptr::null_mut(), elemsize, kp, keysize, &mut tc, HM_BINARY);
        let pr = (l.r.hmget_key_ts)(std::ptr::null_mut(), elemsize, kp, keysize, &mut tr, HM_BINARY);
        assert_eq!(tc, tr, "null-bootstrap temp");
        assert_eq!(tc, -1);
        assert!(!pc.is_null() && !pr.is_null());
        assert_snap(
            &snap_hash(pc, elemsize, Pay::Raw),
            &snap_hash(pr, elemsize, Pay::Raw),
            "hmget_key_ts(NULL)",
        );

        // (b) a == NULL, hmget_key (also writes header->temp)
        let qc = (l.c.hmget_key)(std::ptr::null_mut(), elemsize, kp, keysize, HM_BINARY);
        let qr = (l.r.hmget_key)(std::ptr::null_mut(), elemsize, kp, keysize, HM_BINARY);
        assert_snap(
            &snap_hash(qc, elemsize, Pay::Raw),
            &snap_hash(qr, elemsize, Pay::Raw),
            "hmget_key(NULL)",
        );
        assert_eq!(header(snap_base(qc, elemsize)).temp, -1);
        assert_eq!(header(snap_base(qr, elemsize)).temp, -1);

        // (c) array built by arrgrowf, hash_table == NULL
        for (lib, out) in [(&l.c, &mut tc), (&l.r, &mut tr)] {
            let raw = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 2);
            let h = (raw as *mut u8).wrapping_sub(HEADER_SIZE) as *mut Header;
            (*h).length = 1;
            std::ptr::write_bytes(raw as *mut u8, 0, elemsize);
            let t = (raw as *mut u8).wrapping_add(elemsize) as *mut c_void;
            let mut tmp: isize = 0x77;
            let back = (lib.hmget_key_ts)(t, elemsize, kp, keysize, &mut tmp, HM_BINARY);
            assert_eq!(back, t, "{}: must return a unchanged", lib.name);
            *out = tmp;
            (lib.arrfreef)(raw);
        }
        assert_eq!(tc, tr, "table==NULL temp");
        assert_eq!(tc, -1);

        // cleanup
        (l.c.hmfree_func)(snap_base(pc, elemsize), elemsize);
        (l.r.hmfree_func)(snap_base(pr, elemsize), elemsize);
        (l.c.hmfree_func)(snap_base(qc, elemsize), elemsize);
        (l.r.hmfree_func)(snap_base(qr, elemsize), elemsize);
    }
}

fn snap_base(t: *mut c_void, elemsize: usize) -> *mut c_void {
    (t as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// rows 36-39 / ERRORS.md rows 17-19: `stbds_hmput_default`.
#[test]
fn row_36_39_hmput_default() {
    let (l, _g) = libs();
    for &elemsize in [4usize, 8, 16, 24].iter() {
        seed_both(l, 5);
        unsafe {
            // (a) a == NULL
            let mut pc = (l.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let mut pr = (l.r.hmput_default)(std::ptr::null_mut(), elemsize);
            assert_snap(
                &snap_hash(pc, elemsize, Pay::Raw),
                &snap_hash(pr, elemsize, Pay::Raw),
                "hmput_default(NULL)",
            );
            assert_eq!(header(snap_base(pc, elemsize)).length, 1);

            // (b) non-null, length != 0 -> exact no-op incl. pointer identity
            let again_c = (l.c.hmput_default)(pc, elemsize);
            let again_r = (l.r.hmput_default)(pr, elemsize);
            assert_eq!(again_c, pc, "C no-op must return the same pointer");
            assert_eq!(again_r, pr, "Rust no-op must return the same pointer");
            assert_snap(
                &snap_hash(again_c, elemsize, Pay::Raw),
                &snap_hash(again_r, elemsize, Pay::Raw),
                "hmput_default(non-empty)",
            );

            // (c) non-null with length == 0
            for (lib, slot) in [(&l.c, &mut pc), (&l.r, &mut pr)] {
                let raw = snap_base(*slot, elemsize);
                (*((raw as *mut u8).wrapping_sub(HEADER_SIZE) as *mut Header)).length = 0;
                *slot = (lib.hmput_default)(*slot, elemsize);
            }
            assert_snap(
                &snap_hash(pc, elemsize, Pay::Raw),
                &snap_hash(pr, elemsize, Pay::Raw),
                "hmput_default(length==0)",
            );
            assert_eq!(header(snap_base(pc, elemsize)).length, 1);

            // (d) then real puts on top of the default (row 39)
            let mut m = Map::new(l, elemsize, 4, HM_BINARY, Pay::Raw);
            m.c = pc;
            m.r = pr;
            let mut rng = Rng::new(0xC_0039 + elemsize as u64);
            for i in 0..40 {
                let mut k = rng.bytes(4);
                let v = rng.bytes(elemsize - 4);
                let kp = k.as_mut_ptr() as *mut c_void;
                m.put(kp, kp, &v, 4);
                m.assert_eq(&format!("es={elemsize} default+put {i}"));
            }
            m.free();
        }
    }
}

/// rows 40-42 / ERRORS.md rows 29-31, 40: `hmdel_key` basics.
#[test]
fn row_40_42_del_basics() {
    let (l, _g) = libs();
    unsafe {
        // ERRORS.md row 29: a == NULL -> returns NULL on both sides
        let mut k = vec![9u8, 9, 9, 9];
        let kp = k.as_mut_ptr() as *mut c_void;
        let dc = (l.c.hmdel_key)(std::ptr::null_mut(), 8, kp, 4, 0, HM_BINARY);
        let dr = (l.r.hmdel_key)(std::ptr::null_mut(), 8, kp, 4, 0, HM_BINARY);
        assert!(dc.is_null(), "C hmdel_key(NULL) must return NULL");
        assert!(dr.is_null(), "Rust hmdel_key(NULL) must return NULL");
    }

    for &n in [1usize, 2, 6, 7, 8, 20, 64].iter() {
        for del_pos in ["first", "last", "middle", "absent"] {
            seed_both(l, 0xABCD);
            let mut rng = Rng::new(0xC_0040 + n as u64 * 31);
            let kvs = make_kvs(&mut rng, n, 4, 4, true);
            let mut m = bin_map(l, 4, 4);
            unsafe {
                for kv in kvs.iter() {
                    let mut kb = kv.key.clone();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    m.put(kp, kp, &kv.val, 4);
                }
                let mut target = match del_pos {
                    "first" => kvs[0].key.clone(),
                    "last" => kvs[n - 1].key.clone(),
                    "middle" => kvs[n / 2].key.clone(),
                    _ => {
                        let mut t = rng.bytes(4);
                        while kvs.iter().any(|kv| kv.key == t) {
                            t = rng.bytes(4);
                        }
                        t
                    }
                };
                let kp = target.as_mut_ptr() as *mut c_void;
                m.del(kp, kp, 0);
                m.assert_eq(&format!("n={n} del {del_pos}"));
                let (sc, _) = m.snaps();
                if del_pos == "absent" {
                    assert_eq!(sc.temp, 0, "absent delete must leave temp == 0");
                    assert_eq!(sc.length, n + 1);
                } else {
                    assert_eq!(sc.temp, 1, "successful delete must set temp == 1");
                    assert_eq!(sc.length, n);
                }
                // every remaining key must still be findable, identically
                for kv in kvs.iter() {
                    if kv.key == target {
                        continue;
                    }
                    let mut kb = kv.key.clone();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    let (a, b) = m.get_ts(kp, kp);
                    assert_eq!(a, b, "n={n} {del_pos}: survivor lookup");
                    assert!(a >= 0, "n={n} {del_pos}: survivor must still be found");
                }
                m.assert_eq(&format!("n={n} del {del_pos} final"));
                m.free();
            }
        }
    }
}

/// row 43 / ERRORS.md rows 38, 42: delete down past
/// `used_count_shrink_threshold` on a 16+-slot table -> shrink rebuild;
/// and confirm an 8-slot table never shrinks (`ucst` forced to 0).
#[test]
fn row_43_shrink_rebuild() {
    let (l, _g) = libs();
    for &n in [7usize, 13, 25, 49, 100].iter() {
        seed_both(l, 0x5EED);
        let mut rng = Rng::new(0xC_0043 + n as u64);
        let kvs = make_kvs(&mut rng, n, 4, 4, true);
        let mut m = bin_map(l, 4, 4);
        unsafe {
            for kv in kvs.iter() {
                let mut kb = kv.key.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                m.put(kp, kp, &kv.val, 4);
            }
            let (s0, _) = m.snaps();
            let slots0 = s0.table.as_ref().unwrap().slot_count;
            assert!(slots0 >= 8);
            for (i, kv) in kvs.iter().enumerate() {
                let mut kb = kv.key.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                m.del(kp, kp, 0);
                m.assert_eq(&format!("n={n} shrink del {i}"));
            }
            let (s1, _) = m.snaps();
            assert_eq!(s1.length, 1, "all entries deleted");
            assert_eq!(
                s1.table.as_ref().unwrap().slot_count,
                8,
                "n={n}: table must have shrunk all the way back to 8 slots"
            );
            m.free();
        }
    }
}

/// row 44-45 / ERRORS.md rows 24, 39: tombstone accumulation -> same-size
/// rebuild, and tombstone reuse on re-insert.
#[test]
fn row_44_45_tombstone_rebuild_and_reuse() {
    let (l, _g) = libs();
    for &n in [8usize, 20, 50].iter() {
        seed_both(l, 0x7071);
        let mut rng = Rng::new(0xC_0044 + n as u64);
        let kvs = make_kvs(&mut rng, n, 4, 4, true);
        let mut m = bin_map(l, 4, 4);
        unsafe {
            for kv in kvs.iter() {
                let mut kb = kv.key.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                m.put(kp, kp, &kv.val, 4);
            }
            // churn: delete then immediately re-insert the same key, many times
            for round in 0..40 {
                let idx = rng.below(kvs.len());
                let mut kb = kvs[idx].key.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                m.del(kp, kp, 0);
                m.assert_eq(&format!("n={n} churn {round} del"));
                let v = rng.bytes(4);
                m.put(kp, kp, &v, 4);
                m.assert_eq(&format!("n={n} churn {round} reput"));
                // and everything must still be reachable
                for kv in kvs.iter() {
                    let mut b = kv.key.clone();
                    let p = b.as_mut_ptr() as *mut c_void;
                    let (a, b2) = m.get_ts(p, p);
                    assert_eq!(a, b2, "n={n} churn {round} lookup");
                    assert!(a >= 0, "n={n} churn {round}: key vanished");
                }
            }
            m.free();
        }
    }
}

/// row 46: non-zero `keyoffset` passed to `hmdel_key`.  `hmput_key` always
/// stores keys at offset 0, so a non-zero offset makes the delete miss — both
/// libraries must miss identically.
#[test]
fn row_46_del_keyoffset() {
    let (l, _g) = libs();
    let keysize = 4usize;
    let elemsize = 16usize;
    for &keyoffset in [0usize, 4, 8, 12].iter() {
        seed_both(l, 0x0FF5);
        let mut rng = Rng::new(0xC_0046 + keyoffset as u64);
        let kvs = make_kvs(&mut rng, 30, keysize, elemsize - keysize, true);
        let mut m = Map::new(l, elemsize, keysize, HM_BINARY, Pay::Raw);
        unsafe {
            for kv in kvs.iter() {
                let mut kb = kv.key.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                m.put(kp, kp, &kv.val, keysize);
            }
            for (i, kv) in kvs.iter().enumerate() {
                let mut kb = kv.key.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                m.del(kp, kp, keyoffset);
                m.assert_eq(&format!("keyoffset={keyoffset} del {i}"));
            }
            m.free();
        }
    }
}

/// row 47 / row 78: long randomized put/get/del mix — the composed pipeline.
#[test]
fn row_47_78_randomized_mix() {
    let (l, _g) = libs();
    for &(elemsize, keysize) in [(8usize, 4usize), (16, 4), (24, 8), (32, 16), (9, 1)].iter() {
        for trial in 0..3 {
            seed_both(l, 0x1000 + trial * 7);
            let mut rng = Rng::new(0xC_0047 + (elemsize * 1000 + keysize) as u64 + trial as u64);
            let pool = make_kvs(&mut rng, 60, keysize, elemsize - keysize, true);
            let mut m = Map::new(l, elemsize, keysize, HM_BINARY, Pay::Raw);
            unsafe {
                for op in 0..400 {
                    let idx = rng.below(pool.len());
                    let mut kb = pool[idx].key.clone();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    match rng.below(10) {
                        0..=4 => {
                            let v = rng.bytes(elemsize - keysize);
                            m.put(kp, kp, &v, keysize);
                        }
                        5..=6 => {
                            let (a, b) = m.get_ts(kp, kp);
                            assert_eq!(a, b, "es={elemsize} ks={keysize} op={op} get_ts");
                        }
                        7 => {
                            let (a, b) = m.get(kp, kp);
                            assert_eq!(a, b, "es={elemsize} ks={keysize} op={op} get");
                        }
                        _ => {
                            m.del(kp, kp, 0);
                        }
                    }
                    m.assert_eq(&format!("es={elemsize} ks={keysize} trial={trial} op={op}"));
                }
                m.free();
            }
        }
    }
}
