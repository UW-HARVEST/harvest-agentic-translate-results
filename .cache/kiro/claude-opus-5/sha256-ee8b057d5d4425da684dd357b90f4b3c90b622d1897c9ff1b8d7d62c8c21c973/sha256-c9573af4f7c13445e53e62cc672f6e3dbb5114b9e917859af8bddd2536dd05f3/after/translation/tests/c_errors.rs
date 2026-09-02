//! Phase C — one differential test per `ERRORS.md` row that is not already
//! covered by a Phase B test, plus the generic C-API boundaries (null pointers,
//! zero and oversized lengths, out-of-range enum values crossing FFI).
//!
//! Every assertion compares the C and the Rust `.so` against each other and
//! pins the *specific* sentinel / error value, never merely "both failed".

mod common;
use common::*;
use std::ffi::{c_char, c_void};

// ---------------------------------------------------------------------------
// Generic boundary: NULL pointers into every entry point that accepts one
// ---------------------------------------------------------------------------

/// ERRORS.md rows 7, 12, 13, 17, 29 + generic null-pointer boundary.
#[test]
fn null_pointer_boundary() {
    let (l, _g) = libs();
    let elemsize = 16usize;
    let keysize = 4usize;
    let mut key = vec![1u8, 2, 3, 4];
    let kp = key.as_mut_ptr() as *mut c_void;

    unsafe {
        // hmfree_func(NULL) -> immediate return, no crash
        (l.c.hmfree_func)(std::ptr::null_mut(), elemsize);
        (l.r.hmfree_func)(std::ptr::null_mut(), elemsize);

        // hmdel_key(NULL) -> NULL (the ONLY function that returns NULL)
        let dc = (l.c.hmdel_key)(std::ptr::null_mut(), elemsize, kp, keysize, 0, HM_BINARY);
        let dr = (l.r.hmdel_key)(std::ptr::null_mut(), elemsize, kp, keysize, 0, HM_BINARY);
        assert!(dc.is_null() && dr.is_null(), "hmdel_key(NULL) must return NULL");

        // ... for every mode, including out-of-range ones
        for &m in [-1i32, 0, 1, 2, 1000, i32::MIN, i32::MAX].iter() {
            let a = (l.c.hmdel_key)(std::ptr::null_mut(), elemsize, kp, keysize, 0, m);
            let b = (l.r.hmdel_key)(std::ptr::null_mut(), elemsize, kp, keysize, 0, m);
            assert_eq!(a.is_null(), b.is_null(), "hmdel_key(NULL, mode={m})");
            assert!(a.is_null(), "hmdel_key(NULL, mode={m}) must be NULL");
        }

        // arrgrowf(NULL, e, 0, 0) -> stays NULL
        for &es in [0usize, 1, 4, 16].iter() {
            let a = (l.c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let b = (l.r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(a.is_null() && b.is_null(), "arrgrowf(NULL,{es},0,0) must stay NULL");
        }

        // hmget_key_ts(NULL) -> allocates, *temp = -1
        let mut tc: isize = 0x5A;
        let mut tr: isize = 0x5A;
        let pc = (l.c.hmget_key_ts)(std::ptr::null_mut(), elemsize, kp, keysize, &mut tc, HM_BINARY);
        let pr = (l.r.hmget_key_ts)(std::ptr::null_mut(), elemsize, kp, keysize, &mut tr, HM_BINARY);
        assert_eq!(tc, tr);
        assert_eq!(tc, -1, "hmget_key_ts(NULL) must write STBDS_INDEX_EMPTY");
        assert_snap(
            &snap_hash(pc, elemsize, Pay::Raw),
            &snap_hash(pr, elemsize, Pay::Raw),
            "hmget_key_ts(NULL)",
        );
        (l.c.hmfree_func)(base(pc, elemsize), elemsize);
        (l.r.hmfree_func)(base(pr, elemsize), elemsize);

        // hmput_default(NULL)
        let qc = (l.c.hmput_default)(std::ptr::null_mut(), elemsize);
        let qr = (l.r.hmput_default)(std::ptr::null_mut(), elemsize);
        assert_snap(
            &snap_hash(qc, elemsize, Pay::Raw),
            &snap_hash(qr, elemsize, Pay::Raw),
            "hmput_default(NULL)",
        );
        (l.c.hmfree_func)(base(qc, elemsize), elemsize);
        (l.r.hmfree_func)(base(qr, elemsize), elemsize);

        // hash_bytes(NULL, 0, seed): len 0 dereferences nothing
        for &seed in [0usize, 1, usize::MAX, 0x3141_5926].iter() {
            let a = (l.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let b = (l.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(a, b, "hash_bytes(NULL, 0, {seed:#x})");
        }

        // strreset on a zeroed arena
        let mut az = Arena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut bz = az;
        (l.c.strreset)(&mut az);
        (l.r.strreset)(&mut bz);
        assert_eq!(snap_arena(&az), snap_arena(&bz), "strreset(zeroed)");
    }
}

fn base(t: *mut c_void, elemsize: usize) -> *mut c_void {
    (t as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

// ---------------------------------------------------------------------------
// Generic boundary: zero and (wrapping) oversized lengths
// ---------------------------------------------------------------------------

/// ERRORS.md rows 2, 53, 58 + generic zero/oversized-length boundary.
#[test]
fn zero_and_oversized_lengths() {
    let (l, _g) = libs();
    unsafe {
        // elemsize == 0 for arrgrowf: a header-only allocation
        for &mc in [1usize, 4, 100].iter() {
            let a = (l.c.arrgrowf)(std::ptr::null_mut(), 0, 0, mc);
            let b = (l.r.arrgrowf)(std::ptr::null_mut(), 0, 0, mc);
            assert!(!a.is_null() && !b.is_null());
            let ha = header(a);
            let hb = header(b);
            assert_eq!(ha.length, hb.length, "elemsize=0 mc={mc} length");
            assert_eq!(ha.capacity, hb.capacity, "elemsize=0 mc={mc} capacity");
            assert_eq!(ha.temp, hb.temp, "elemsize=0 mc={mc} temp");
            assert_eq!(ha.capacity, mc.max(4));
            (l.c.arrfreef)(a);
            (l.r.arrfreef)(b);
        }

        // `elemsize * min_cap` wrapping size_t to exactly 0 -> a 32-byte
        // allocation with a huge recorded capacity.  The C has no overflow
        // check; both sides must wrap identically.  (The payload is of course
        // never touched.)
        for &(es, mc) in [
            (1usize << 32, 1usize << 32),
            (1usize << 63, 2usize),
            (1usize << 62, 4usize),
            (1usize << 40, 1usize << 24),
        ]
        .iter()
        {
            let a = (l.c.arrgrowf)(std::ptr::null_mut(), es, 0, mc);
            let b = (l.r.arrgrowf)(std::ptr::null_mut(), es, 0, mc);
            assert!(!a.is_null() && !b.is_null(), "es={es:#x} mc={mc:#x}");
            assert_eq!(
                es.wrapping_mul(mc),
                0,
                "test precondition: the byte size must wrap to 0"
            );
            let ha = header(a);
            let hb = header(b);
            assert_eq!(ha.capacity, hb.capacity, "es={es:#x} mc={mc:#x} capacity");
            assert_eq!(ha.length, hb.length);
            assert_eq!(ha.temp, hb.temp);
            // the `min_cap < 4` clamp still applies before the multiply wraps
            assert_eq!(ha.capacity, mc.max(4));
            (l.c.arrfreef)(a);
            (l.r.arrfreef)(b);
        }

        // `addlen` wrapping into the size computation.  Only combinations whose
        // byte size wraps to exactly 0 are probed here: the C writes a 32-byte
        // header regardless of what `realloc` was asked for, so a wrap that
        // lands on a *small non-zero* size makes BOTH libraries overflow the
        // heap by the same amount and abort the process — that is matching
        // behaviour, but it destroys the harness, so it is documented in
        // ERRORS.md row 2 rather than executed.  The wrap itself is covered
        // safely by `b_arr::err_row_2_addlen_wraparound`.
        for &(es, addlen) in [(1usize << 63, 2usize), (1usize << 62, 4usize)].iter() {
            let a = (l.c.arrgrowf)(std::ptr::null_mut(), es, addlen, 0);
            let b = (l.r.arrgrowf)(std::ptr::null_mut(), es, addlen, 0);
            assert!(!a.is_null() && !b.is_null(), "es={es:#x} addlen={addlen}");
            assert_eq!(header(a).capacity, header(b).capacity, "es={es:#x} capacity");
            assert_eq!(header(a).length, header(b).length, "es={es:#x} length");
            assert_eq!(header(a).temp, header(b).temp, "es={es:#x} temp");
            assert_eq!(header(a).capacity, addlen.max(4));
            assert_eq!(es.wrapping_mul(addlen.max(4)), 0, "precondition");
            (l.c.arrfreef)(a);
            (l.r.arrfreef)(b);
        }

        // hash_bytes with len 0 on a real buffer
        let mut buf = vec![0xAAu8; 32];
        for &seed in [0usize, 1, 2, usize::MAX].iter() {
            let a = (l.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            let b = (l.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(a, b, "hash_bytes len=0 seed={seed:#x}");
        }

        // hash_string on the empty string
        let mut empty = vec![0u8];
        for &seed in [0usize, 1, usize::MAX].iter() {
            let a = (l.c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            let b = (l.r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(a, b, "hash_string(\"\") seed={seed:#x}");
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 9, 10 — both `return -1` sites in stbds_hm_find_slot
// ---------------------------------------------------------------------------

/// The forward half-scan and the wrap-around half-scan each have their own
/// `return -1`.  Which one fires depends on `pos & 7`, so a large number of
/// absent lookups against tables of every size class exercises both.
#[test]
fn err_rows_9_10_find_slot_returns_minus_one() {
    let (l, _g) = libs();
    for &n in [1usize, 3, 6, 7, 8, 15, 16, 40, 100, 300].iter() {
        for seedv in [0usize, 1, 0x3141_5926, 0xDEAD_BEEF] {
            seed_both(l, seedv);
            let mut rng = Rng::new(0xF_0009 + n as u64 * 31 + seedv as u64);
            let mut present: Vec<Vec<u8>> = vec![];
            let mut m = Map::new(l, 8, 4, HM_BINARY, Pay::Raw);
            unsafe {
                while present.len() < n {
                    let mut k = rng.bytes(4);
                    if present.contains(&k) {
                        continue;
                    }
                    present.push(k.clone());
                    let v = rng.bytes(4);
                    let kp = k.as_mut_ptr() as *mut c_void;
                    m.put(kp, kp, &v, 4);
                }
                let mut misses = 0;
                for _ in 0..600 {
                    let mut k = rng.bytes(4);
                    if present.contains(&k) {
                        continue;
                    }
                    let kp = k.as_mut_ptr() as *mut c_void;
                    let (a, b) = m.get_ts(kp, kp);
                    assert_eq!(a, b, "n={n} seed={seedv:#x} absent lookup");
                    assert_eq!(a, -1, "n={n}: absent key must give exactly -1");
                    misses += 1;
                }
                assert!(misses > 500);
                m.assert_eq(&format!("n={n} seed={seedv:#x} after misses"));
                m.free();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 27, 28 — the asymmetric `temp_key` update on duplicate keys
// ---------------------------------------------------------------------------

/// `hmput_key` updates `temp_key` when the duplicate is found in the *forward*
/// half-scan but NOT in the wrap-around half-scan.  Heavy duplicate re-putting
/// across many seeds and table sizes hits both; whichever branch is taken, the
/// C and the Rust must take the same one, which shows up as `temp_key`
/// pointing at the same string on both sides.
#[test]
fn err_rows_27_28_duplicate_temp_key_asymmetry() {
    let (l, _g) = libs();
    for &shmode in [SH_DEFAULT, SH_STRDUP, SH_ARENA].iter() {
        for seedv in [0usize, 1, 7, 0x3141_5926, 0xBEEF] {
            seed_both(l, seedv);
            let mut rng = Rng::new(0xF_0027 + shmode as u64 * 97 + seedv as u64);
            let elemsize = 16usize;
            let mut keep_c = Keep::new();
            let mut keep_r = Keep::new();
            let mut m = Map::from_shmode(l, elemsize, 8, HM_STRING, shmode, Pay::StrPtr);
            let mut ks: Vec<Vec<u8>> = vec![];
            unsafe {
                while ks.len() < 40 {
                    let n = 1 + rng.below(20);
                    let k = rng.cstring(n);
                    if ks.contains(&k) {
                        continue;
                    }
                    ks.push(k.clone());
                    let (kc, kr) = (keep_c.add(&k), keep_r.add(&k));
                    let v = rng.bytes(8);
                    m.put(kc, kr, &v, 8);
                }
                for round in 0..20 {
                    for (i, k) in ks.iter().enumerate() {
                        let (kc, kr) = (keep_c.add(k), keep_r.add(k));
                        let v = rng.bytes(8);
                        m.put(kc, kr, &v, 8);
                        m.assert_eq(&format!("shmode={shmode} round={round} dup {i}"));
                        assert_eq!(
                            temp_key_str(m.raw_c()),
                            temp_key_str(m.raw_r()),
                            "shmode={shmode} round={round} dup {i}: temp_key"
                        );
                    }
                }
                m.free();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 51, 52 — out-of-range enum values across the FFI boundary
// ---------------------------------------------------------------------------

/// `mode` is a C `int`, so *any* `i32` is a real input.  Values `>= 1` take the
/// string path, values `<= 0` the binary path.  Verified right at the `mode == 0`
/// / `mode == 1` boundary and one step past it in both directions, plus the
/// extremes, on a table whose storage mode makes the config safe to drive.
#[test]
fn err_rows_51_52_out_of_range_mode_enum() {
    let (l, _g) = libs();
    let modes: [i32; 14] = [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        255,
        256,
        1000,
        i32::MAX,
    ];

    for &m in modes.iter() {
        let string_path = m >= 1;
        seed_both(l, 0x1357);
        let mut rng = Rng::new(0xF_0051 ^ (m as i64 as u64));
        let elemsize = 16usize;
        let keysize = 8usize;

        unsafe {
            if string_path {
                // SH_DEFAULT storage: `is_key_equal` dereferences the stored
                // char*, which is exactly what the string path expects.
                let mut keep_c = Keep::new();
                let mut keep_r = Keep::new();
                let mut map = Map::from_shmode(l, elemsize, keysize, m, SH_DEFAULT, Pay::StrPtr);
                let mut ks: Vec<Vec<u8>> = vec![];
                while ks.len() < 25 {
                    let n = 1 + rng.below(20);
                    let k = rng.cstring(n);
                    if ks.contains(&k) {
                        continue;
                    }
                    ks.push(k.clone());
                    let (kc, kr) = (keep_c.add(&k), keep_r.add(&k));
                    let v = rng.bytes(8);
                    map.put(kc, kr, &v, 8);
                    map.assert_eq(&format!("mode={m} string put"));
                }
                for k in ks.iter() {
                    let (kc, kr) = (keep_c.add(k), keep_r.add(k));
                    let (a, b) = map.get_ts(kc, kr);
                    assert_eq!(a, b, "mode={m} string get");
                    assert!(a >= 0, "mode={m}: string path must resolve the key");
                }
                // an absent key must give exactly -1
                let absent = rng.cstring(31);
                let (kc, kr) = (keep_c.add(&absent), keep_r.add(&absent));
                let (a, b) = map.get_ts(kc, kr);
                assert_eq!(a, b);
                assert_eq!(a, -1, "mode={m}: absent string key");
                // delete the LAST element only (see ERRORS.md row 37)
                for k in ks.iter().rev() {
                    let (kc, kr) = (keep_c.add(k), keep_r.add(k));
                    map.del(kc, kr, 0);
                    map.assert_eq(&format!("mode={m} string del-last"));
                }
                map.free();
            } else {
                // binary path: SH_NONE storage memcpy's the key bytes
                let mut map = Map::from_shmode(l, elemsize, keysize, m, SH_NONE, Pay::Raw);
                let mut ks: Vec<Vec<u8>> = vec![];
                while ks.len() < 25 {
                    let k = rng.bytes(keysize);
                    if ks.contains(&k) {
                        continue;
                    }
                    ks.push(k.clone());
                    let mut kb = k.clone();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    let v = rng.bytes(elemsize - keysize);
                    map.put(kp, kp, &v, keysize);
                    map.assert_eq(&format!("mode={m} binary put"));
                }
                for k in ks.iter() {
                    let mut kb = k.clone();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    let (a, b) = map.get_ts(kp, kp);
                    assert_eq!(a, b, "mode={m} binary get");
                    assert!(a >= 0, "mode={m}: binary path must resolve the key");
                }
                let mut absent = rng.bytes(keysize);
                while ks.contains(&absent) {
                    absent = rng.bytes(keysize);
                }
                let kp = absent.as_mut_ptr() as *mut c_void;
                let (a, b) = map.get_ts(kp, kp);
                assert_eq!(a, b);
                assert_eq!(a, -1, "mode={m}: absent binary key");
                for k in ks.iter() {
                    let mut kb = k.clone();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    map.del(kp, kp, 0);
                    map.assert_eq(&format!("mode={m} binary del"));
                }
                map.free();
            }
        }
    }
}

/// ERRORS.md row 50 + generic out-of-range enum: `shmode_func` accepts any
/// `int` and truncates it to `unsigned char`.  Sweep every value 0..=257 and
/// the extremes.
#[test]
fn err_row_50_shmode_full_sweep() {
    let (l, _g) = libs();
    let elemsize = 16usize;
    let mut modes: Vec<i32> = (0..=257).collect();
    modes.extend_from_slice(&[-1, -2, -255, -256, -257, i32::MIN, i32::MAX, 65535, 65536]);
    for &shmode in modes.iter() {
        seed_both(l, 0x2468);
        unsafe {
            let tc = (l.c.shmode_func)(elemsize, shmode);
            let tr = (l.r.shmode_func)(elemsize, shmode);
            let sc = snap_hash(tc, elemsize, Pay::Raw);
            let sr = snap_hash(tr, elemsize, Pay::Raw);
            assert_snap(&sc, &sr, &format!("shmode_func(mode={shmode})"));
            assert_eq!(
                sc.table.as_ref().unwrap().arena_mode,
                (shmode as u32 & 0xff) as u8,
                "mode={shmode}"
            );
            (l.c.hmfree_func)(base(tc, elemsize), elemsize);
            (l.r.hmfree_func)(base(tr, elemsize), elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 30, 31 — hmdel_key rejections
// ---------------------------------------------------------------------------

/// row 30: `hash_table == NULL` -> `temp = 0`, `a` returned unchanged, length
/// untouched.  row 31: key absent -> same, but via `find_slot < 0`.
#[test]
fn err_rows_30_31_del_rejections() {
    let (l, _g) = libs();
    let elemsize = 16usize;
    let keysize = 4usize;
    let mut key = vec![7u8, 7, 7, 7];
    let kp = key.as_mut_ptr() as *mut c_void;

    // row 30: an array with no hash table at all
    unsafe {
        let mut lens: Vec<(usize, isize)> = vec![];
        for lib in [&l.c, &l.r] {
            let raw = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let h = (raw as *mut u8).wrapping_sub(HEADER_SIZE) as *mut Header;
            (*h).length = 3;
            (*h).temp = 0x1234;
            std::ptr::write_bytes(raw as *mut u8, 0, elemsize * 3);
            let t = (raw as *mut u8).wrapping_add(elemsize) as *mut c_void;
            let back = (lib.hmdel_key)(t, elemsize, kp, keysize, 0, HM_BINARY);
            assert_eq!(back, t, "{}: table==NULL must return `a` unchanged", lib.name);
            lens.push(((*h).length, (*h).temp));
            (lib.arrfreef)(raw);
        }
        assert_eq!(lens[0], lens[1], "table==NULL: (length, temp)");
        assert_eq!(lens[0].0, 3, "length must be untouched");
        assert_eq!(lens[0].1, 0, "temp must be reset to 0");
    }

    // row 31: absent key on a real table, every size class
    for &n in [1usize, 6, 7, 8, 20, 64].iter() {
        seed_both(l, 0x9753);
        let mut rng = Rng::new(0xF_0031 + n as u64);
        let mut m = Map::new(l, 8, 4, HM_BINARY, Pay::Raw);
        let mut present: Vec<Vec<u8>> = vec![];
        unsafe {
            while present.len() < n {
                let mut k = rng.bytes(4);
                if present.contains(&k) {
                    continue;
                }
                present.push(k.clone());
                let v = rng.bytes(4);
                let p = k.as_mut_ptr() as *mut c_void;
                m.put(p, p, &v, 4);
            }
            let before = m.snaps().0;
            for _ in 0..200 {
                let mut k = rng.bytes(4);
                if present.contains(&k) {
                    continue;
                }
                let p = k.as_mut_ptr() as *mut c_void;
                m.del(p, p, 0);
                m.assert_eq(&format!("n={n} absent del"));
                let (sc, _) = m.snaps();
                assert_eq!(sc.temp, 0, "absent delete must set temp = 0");
                assert_eq!(sc.length, before.length, "absent delete must not shrink");
                assert_eq!(
                    sc.table.as_ref().unwrap().used_count,
                    before.table.as_ref().unwrap().used_count,
                    "absent delete must not touch used_count"
                );
                assert_eq!(
                    sc.table.as_ref().unwrap().tombstone_count,
                    before.table.as_ref().unwrap().tombstone_count,
                    "absent delete must not create a tombstone"
                );
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 79 — hmput_key on a NULL map with a string mode
// ---------------------------------------------------------------------------

/// `hmput_key(NULL, ..., mode >= STBDS_HM_STRING)` builds its own table and
/// sets `string.mode = STBDS_SH_DEFAULT` (ERRORS.md row 21) — no
/// `shmode_func` involved.  This is the `shput`-on-a-fresh-map path.
#[test]
fn cfg_row_79_hmput_key_null_string_mode() {
    let (l, _g) = libs();
    for &m in [1i32, 2, 1000, i32::MAX].iter() {
        for &elemsize in [8usize, 16, 24].iter() {
            seed_both(l, 0xFACE);
            let mut rng = Rng::new(0xF_0079 + m as u64 + elemsize as u64);
            let mut keep_c = Keep::new();
            let mut keep_r = Keep::new();
            // NOTE: starts from NULL, not from shmode_func
            let mut map = Map::new(l, elemsize, 8, m, Pay::StrPtr);
            let mut ks: Vec<Vec<u8>> = vec![];
            unsafe {
                while ks.len() < 30 {
                    let n = 1 + rng.below(25);
                    let k = rng.cstring(n);
                    if ks.contains(&k) {
                        continue;
                    }
                    ks.push(k.clone());
                    let (kc, kr) = (keep_c.add(&k), keep_r.add(&k));
                    let v = rng.bytes(elemsize - 8);
                    map.put(kc, kr, &v, 8);
                    map.assert_eq(&format!("mode={m} es={elemsize} fresh-string put"));
                    assert_eq!(
                        temp_key_str(map.raw_c()),
                        temp_key_str(map.raw_r()),
                        "mode={m} es={elemsize}: temp_key"
                    );
                }
                let (sc, _) = map.snaps();
                assert_eq!(
                    sc.table.as_ref().unwrap().arena_mode,
                    1,
                    "mode={m}: a fresh string-mode table must get SH_DEFAULT"
                );
                for k in ks.iter() {
                    let (kc, kr) = (keep_c.add(k), keep_r.add(k));
                    let (a, b) = map.get_ts(kc, kr);
                    assert_eq!(a, b, "mode={m} fresh-string get");
                    assert!(a >= 0);
                }
                if m == 1 {
                    // mode == 1 makes the compaction re-find safe, so delete
                    // in a shuffled order
                    let mut order: Vec<usize> = (0..ks.len()).collect();
                    for i in (1..order.len()).rev() {
                        let j = rng.below(i + 1);
                        order.swap(i, j);
                    }
                    for &idx in order.iter() {
                        let (kc, kr) = (keep_c.add(&ks[idx]), keep_r.add(&ks[idx]));
                        map.del(kc, kr, 0);
                        map.assert_eq(&format!("mode={m} fresh-string del"));
                    }
                    let (sc, _) = map.snaps();
                    assert_eq!(sc.length, 1);
                } else {
                    for k in ks.iter().rev() {
                        let (kc, kr) = (keep_c.add(k), keep_r.add(k));
                        map.del(kc, kr, 0);
                        map.assert_eq(&format!("mode={m} fresh-string del-last"));
                    }
                }
                map.free();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 11, 15, 23 — the `hash < 2` adjustment and the
// `INDEX_DELETED` read-back, argued rather than executed
// ---------------------------------------------------------------------------

/// `if (hash < 2) hash += 2` (`find_slot` and `hmput_key`) can only be observed
/// with a siphash / `hash_string` preimage of 0 or 1 — a 2^-63 event that is not
/// constructible.  What *is* testable is the premise the adjustment rests on:
/// the two libraries produce bit-identical hashes for the same input, so they
/// necessarily make the same `< 2` decision.  Likewise a matched slot can never
/// carry `STBDS_INDEX_DELETED`, because tombstones store `hash == 1` while every
/// probe hash is `>= 2`.  This test pins both premises.
#[test]
fn err_rows_11_15_23_hash_below_two_premises() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xF_0011);
    let mut saw_small = false;
    for _ in 0..200_000 {
        let n = rng.below(24);
        let mut buf = rng.bytes(n.max(1));
        let seed = rng.next_u64() as usize;
        unsafe {
            let a = (l.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, n, seed);
            let b = (l.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, n, seed);
            assert_eq!(a, b, "hash_bytes parity is the premise for the `< 2` branch");
            if a < 2 {
                saw_small = true;
            }
            let mut s = rng.cstring(n);
            let a2 = (l.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
            let b2 = (l.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(a2, b2, "hash_string parity");
            if a2 < 2 {
                saw_small = true;
            }
        }
    }
    // Documented expectation: not reachable by sampling.
    assert!(!saw_small, "a hash < 2 was actually found — extend this test");

    // Premise for row 15: every live bucket hash is >= 2 and every tombstone
    // hash is exactly 1, so `bucket->hash[i] == hash` can never select a
    // tombstone.  Verified over a churned table.
    seed_both(l, 0x2020);
    let mut rng = Rng::new(0xF_0015);
    let mut m = Map::new(l, 8, 4, HM_BINARY, Pay::Raw);
    let mut pool: Vec<Vec<u8>> = vec![];
    unsafe {
        while pool.len() < 60 {
            let k = rng.bytes(4);
            if !pool.contains(&k) {
                pool.push(k);
            }
        }
        let mut saw_tombstone = false;
        for op in 0..600 {
            let idx = rng.below(pool.len());
            let mut kb = pool[idx].clone();
            let kp = kb.as_mut_ptr() as *mut c_void;
            if rng.below(2) == 0 {
                let v = rng.bytes(4);
                m.put(kp, kp, &v, 4);
            } else {
                m.del(kp, kp, 0);
            }
            m.assert_eq(&format!("tombstone premise op={op}"));
            let (sc, _) = m.snaps();
            let Some(tbl) = sc.table.as_ref() else { continue };
            for (bi, (hashes, indices)) in tbl.buckets.iter().enumerate() {
                for slot in 0..8 {
                    let h = hashes[slot];
                    let ix = indices[slot];
                    match h {
                        0 => assert_eq!(ix, -1, "op={op} bucket {bi} slot {slot}: EMPTY"),
                        1 => {
                            assert_eq!(ix, -2, "op={op} bucket {bi} slot {slot}: DELETED");
                            saw_tombstone = true;
                        }
                        _ => {
                            assert!(h >= 2, "live hash must be >= 2");
                            assert!(ix >= 0, "op={op} bucket {bi} slot {slot}: live index");
                        }
                    }
                }
            }
        }
        assert!(saw_tombstone, "the churn must have created tombstones");
        m.free();
    }
}
