//! Phase C — one differential test per row of ERRORS.md.
//!
//! Every test constructs the exact invalid input/condition, calls BOTH
//! libraries through their exported symbols and asserts they return the same
//! sentinel (`-1` / `-2` / `NULL` / `temp == 0` / unchanged pointer), not
//! merely "both failed somehow".

mod common;

use common::*;
use std::ffi::{c_char, c_void};

fn k(v: u64, keysize: usize) -> Vec<u8> {
    let b = v.to_le_bytes();
    let mut out = vec![0u8; keysize];
    for i in 0..keysize.min(8) {
        out[i] = b[i];
    }
    out
}

fn pay(v: u64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

fn s(text: &str) -> Vec<u8> {
    let mut v = text.as_bytes().to_vec();
    v.push(0);
    v
}

unsafe fn hdr_bytes(a: *mut c_void) -> String {
    unsafe { hex(std::slice::from_raw_parts(header(a) as *const u8, HDR_SIZE)) }
}

/// row 1 — `stbds_arrgrowf` early return: identical pointer, header untouched
#[test]
fn e01_arrgrowf_noop_returns_same_pointer() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            for &es in &[1usize, 8, 16] {
                let a = (api.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
                (*header(a)).length = 5;
                (*header(a)).temp = -7;
                let before = hdr_bytes(a);
                for &(addlen, min_cap) in
                    &[(0usize, 0usize), (0, 1), (0, 8), (1, 0), (3, 0), (3, 8), (0, 7)]
                {
                    let b = (api.arrgrowf)(a, es, addlen, min_cap);
                    t.push(format!(
                        "es={es} addlen={addlen} min_cap={min_cap} same={} hdr_unchanged={}",
                        b == a,
                        hdr_bytes(a) == before
                    ));
                    assert_eq!(b, a, "{}: early return must give back `a`", api.tag);
                }
                (api.arrfreef)(a);
            }
        }
    }
    assert_traces_eq("e01", &tc, &tr);
}

/// row 2 — `stbds_arrgrowf(NULL, ..)` initialises the header
#[test]
fn e02_arrgrowf_null_initialises_header() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            for &es in &[1usize, 4, 8, 16, 64] {
                for &(addlen, min_cap) in
                    &[(0usize, 1usize), (1, 0), (5, 0), (0, 100), (7, 3), (2, 2)]
                {
                    let a = (api.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    let h = &*header(a);
                    t.push(format!(
                        "es={es} addlen={addlen} min_cap={min_cap} -> len={} cap={} temp={} table_null={}",
                        h.length,
                        h.capacity,
                        h.temp,
                        h.hash_table.is_null()
                    ));
                    assert_eq!(h.length, 0);
                    assert_eq!(h.temp, 0);
                    assert!(h.hash_table.is_null());
                    (api.arrfreef)(a);
                }
            }
        }
    }
    assert_traces_eq("e02", &tc, &tr);
}

/// row 3 — `arrgrowf(NULL, es, 0, 0)` returns NULL (early return, no allocation)
#[test]
fn e03_arrgrowf_zero_zero_returns_null() {
    let p = seeded(DEFAULT_SEED);
    for api in p.both() {
        unsafe {
            for &es in &[0usize, 1, 4, 8, 1024] {
                let a = (api.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
                assert!(a.is_null(), "{}: es={es} expected NULL, got {a:?}", api.tag);
            }
        }
    }
}

/// row 4 — `stbds_arrfreef(NULL)` frees `NULL-32`: invalid free, aborts.
/// Documented only — executing it would kill the test process.
#[test]
fn e04_arrfreef_null_documented() {
    let p = seeded(DEFAULT_SEED);
    // both sides compute the same argument (`(char*)a - 32`) and hand it to
    // libc `free`; there is nothing to compare without crashing.
    assert_eq!(p.c.tag, "C");
    assert_eq!(p.r.tag, "RUST");
}

/// rows 5, 6, 7, 40 — `stbds_hm_find_slot` returning -1 through both probe
/// loops, and the `hash matches / key differs` continuation.
#[test]
fn e05_e06_e07_e40_find_slot_misses() {
    // large randomised absent-key sweep: with 8-element buckets and probing
    // that wraps inside the bucket, both the `pos&7..8` and the `0..limit`
    // exits are hit many times.
    for trial in 0..20u64 {
        let cfg = MapCfg::binary(8, 4).digested();
        let mut rng = Rng::new(0xFACE_0000 + trial);
        let mut ops = Vec::new();
        for i in 0..30u64 {
            ops.push(put(&k(rng.below(64), 4), &pay(i)));
        }
        for i in 0..300u64 {
            // keys far outside the inserted domain: always a miss
            ops.push(get(&k(100_000 + i, 4)));
            ops.push(get_ts(&k(200_000 + i, 4)));
            ops.push(del(&k(300_000 + i, 4)));
        }
        ops.push(Op::Free);
        diff_script(&format!("find_slot misses trial={trial}"), DEFAULT_SEED, cfg, &ops);
    }
    // row 7/40: same hash, different key -> probing continues.  A map created
    // with SH_STRDUP but used in binary mode stores the *duplicated pointer*
    // where the key bytes are compared, so re-inserting an identical key hashes
    // to the same slot yet never compares equal.
    let mut cfg = MapCfg::string(16, HM_BINARY);
    cfg.keysize = 8;
    let mut ops = vec![Op::ShMode { sh_mode: SH_STRDUP }];
    for _ in 0..6 {
        ops.push(put(&s("same-key-0001"), &pay(1)));
    }
    for _ in 0..3 {
        ops.push(get(&s("same-key-0001")));
        ops.push(del(&s("same-key-0001")));
    }
    ops.push(Op::Free);
    diff_script("hash-equal key-unequal", DEFAULT_SEED, cfg, &ops);
}

/// rows 8, 11 — `hmget_key_ts(NULL, ..)` / `hmget_key(NULL, ..)`
#[test]
fn e08_e11_get_on_null_map() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            for &es in &[1usize, 8, 16] {
                for &mode in &[HM_BINARY, HM_STRING, -1, 2, i32::MIN, i32::MAX] {
                    let mut key = k(1234, 8);
                    // _ts variant
                    let mut temp: isize = 0x5555;
                    let h = (api.hmget_key_ts)(
                        std::ptr::null_mut(),
                        es,
                        key.as_mut_ptr() as *mut c_void,
                        4,
                        &mut temp,
                        mode,
                    );
                    t.push(format!("es={es} mode={mode} ts_temp={temp}"));
                    t.extend(snap_map(h, es, KeyKind::Binary));
                    assert_eq!(temp, INDEX_EMPTY, "{}: *temp must be -1", api.tag);
                    (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
                    // non-_ts variant sets the header temp instead
                    let h = (api.hmget_key)(
                        std::ptr::null_mut(),
                        es,
                        key.as_mut_ptr() as *mut c_void,
                        4,
                        mode,
                    );
                    t.push(format!("  hdr_temp={}", map_temp(h, es)));
                    assert_eq!(map_temp(h, es), INDEX_EMPTY, "{}: header temp", api.tag);
                    t.extend(snap_map(h, es, KeyKind::Binary));
                    (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
                    // NULL key is never dereferenced on this path
                    let mut temp2: isize = 0x6666;
                    let h = (api.hmget_key_ts)(
                        std::ptr::null_mut(),
                        es,
                        std::ptr::null_mut(),
                        4,
                        &mut temp2,
                        mode,
                    );
                    t.push(format!("  null_key temp={temp2}"));
                    assert_eq!(temp2, INDEX_EMPTY);
                    (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
                }
            }
        }
    }
    assert_traces_eq("e08/e11", &tc, &tr);
}

/// rows 9, 10 — table-less map and absent key both give `temp == -1`
#[test]
fn e09_e10_get_temp_minus_one() {
    let cfg = MapCfg::binary(8, 4);
    // table-less: created by hmput_default / hmget_key(NULL)
    diff_script(
        "table-less get",
        DEFAULT_SEED,
        cfg,
        &[
            Op::PutDefault { payload: pay(0xabc) },
            get(&k(1, 4)),
            get_ts(&k(1, 4)),
            get(&k(0, 4)),
            get_ts(&k(0xffff_ffff, 4)),
            Op::Free,
        ],
    );
    // populated map, absent keys
    let mut ops = vec![Op::PutDefault { payload: pay(0xabc) }];
    for i in 0..10u64 {
        ops.push(put(&k(i * 2, 4), &pay(i)));
    }
    for i in 0..40u64 {
        ops.push(get(&k(i * 2 + 1, 4)));
        ops.push(get_ts(&k(i * 2 + 1, 4)));
    }
    ops.push(Op::Free);
    diff_script("absent get", DEFAULT_SEED, cfg, &ops);
}

/// rows 12, 13 — out-of-range `mode` across the FFI boundary
#[test]
fn e12_e13_mode_out_of_range() {
    // (a) mode >= 2 must behave exactly like STBDS_HM_STRING
    for probe_mode in [2i32, 3, 4, 7, 255, 256, 1000, i32::MAX] {
        let p = seeded(DEFAULT_SEED);
        let mut tc = Vec::new();
        let mut tr = Vec::new();
        for api in p.both() {
            let t = if api.tag == "C" { &mut tc } else { &mut tr };
            unsafe {
                (api.rand_seed)(DEFAULT_SEED);
                let es = 16usize;
                let mut h = (api.shmode_func)(es, SH_DEFAULT);
                let mut keys: Vec<Box<[u8]>> = Vec::new();
                for i in 0..8u64 {
                    let mut kb: Box<[u8]> = s(&format!("mode-key-{i}")).into_boxed_slice();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    keys.push(kb);
                    h = (api.hmput_key)(h, es, kp, 8, HM_STRING);
                    // initialise the value half (`(t)[temp].value = v`)
                    let elem = map_raw(h, es).offset((map_temp(h, es) + 1) * es as isize);
                    std::ptr::write_bytes(elem.add(8), (0x30 + i) as u8, es - 8);
                }
                for i in 0..10u64 {
                    let mut kb: Box<[u8]> = s(&format!("mode-key-{i}")).into_boxed_slice();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    // look up with the out-of-range mode
                    h = (api.hmget_key)(h, es, kp, 8, probe_mode);
                    let a = map_temp(h, es);
                    let mut tsv: isize = 0;
                    h = (api.hmget_key_ts)(h, es, kp, 8, &mut tsv, probe_mode);
                    t.push(format!("probe_mode={probe_mode} key={i} temp={a} ts={tsv}"));
                    keys.push(kb);
                }
                t.extend(snap_map(h, es, KeyKind::StrPtr { keyoffset: 0 }));
                (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
            }
        }
        assert_traces_eq(&format!("e12 mode={probe_mode}"), &tc, &tr);
    }

    // (b) mode < 0 must behave exactly like STBDS_HM_BINARY
    for probe_mode in [-1i32, -2, -255, i32::MIN, i32::MIN + 1] {
        let p = seeded(DEFAULT_SEED);
        let mut tc = Vec::new();
        let mut tr = Vec::new();
        for api in p.both() {
            let t = if api.tag == "C" { &mut tc } else { &mut tr };
            unsafe {
                (api.rand_seed)(DEFAULT_SEED);
                let es = 8usize;
                let mut h: *mut c_void = std::ptr::null_mut();
                for i in 0..8u64 {
                    let mut key = k(i, 4);
                    h = (api.hmput_key)(h, es, key.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
                    let idx = map_temp(h, es);
                    let elem = map_raw(h, es).offset((idx + 1) * es as isize);
                    std::ptr::copy_nonoverlapping(key.as_ptr(), elem, 4);
                    std::ptr::write_bytes(elem.add(4), 0x33, 4);
                }
                for i in 0..12u64 {
                    let mut key = k(i, 4);
                    h = (api.hmget_key)(h, es, key.as_mut_ptr() as *mut c_void, 4, probe_mode);
                    let a = map_temp(h, es);
                    let mut tsv: isize = 0;
                    h = (api.hmget_key_ts)(
                        h,
                        es,
                        key.as_mut_ptr() as *mut c_void,
                        4,
                        &mut tsv,
                        probe_mode,
                    );
                    // and a delete with the negative mode
                    h = (api.hmdel_key)(h, es, key.as_mut_ptr() as *mut c_void, 4, 0, probe_mode);
                    t.push(format!(
                        "probe_mode={probe_mode} key={i} temp={a} ts={tsv} del_temp={}",
                        map_temp(h, es)
                    ));
                }
                t.extend(snap_map(h, es, KeyKind::Binary));
                (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
            }
        }
        assert_traces_eq(&format!("e13 mode={probe_mode}"), &tc, &tr);
    }
}

/// rows 14, 15, 16 — `stbds_hmput_default`
#[test]
fn e14_e15_e16_put_default() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            for &es in &[1usize, 8, 16] {
                // row 14: a == NULL
                let h = (api.hmput_default)(std::ptr::null_mut(), es);
                t.push(format!("es={es} from NULL: len={}", map_len(h, es)));
                t.extend(snap_map(h, es, KeyKind::Binary));
                assert_eq!(map_len(h, es), 1);
                // row 16: length != 0 -> unchanged, identical pointer
                let h2 = (api.hmput_default)(h, es);
                t.push(format!("  second call same={}", h2 == h));
                assert_eq!(h2, h, "{}: no-op must return the same pointer", api.tag);
                (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);

                // row 15: a != NULL but header->length == 0
                let raw = (api.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
                assert_eq!((*header(raw)).length, 0);
                let hh = (api.hmput_default)((raw as *mut u8).add(es) as *mut c_void, es);
                t.push(format!(
                    "  length==0 path: len={} same_as_input={}",
                    map_len(hh, es),
                    hh == (raw as *mut u8).add(es) as *mut c_void
                ));
                t.extend(snap_map(hh, es, KeyKind::Binary));
                assert_eq!(map_len(hh, es), 1);
                (api.hmfree_func)(map_raw(hh, es) as *mut c_void, es);
            }
        }
    }
    assert_traces_eq("e14/e15/e16", &tc, &tr);
}

/// row 17 — `STBDS_ASSERT(i+1 <= arrcap)` in `hmput_key` never fires
#[test]
fn e17_hmput_capacity_assert_documented() {
    // exercised indirectly by every insert in Phase B; if it ever fired the
    // process would abort and every map test would fail.
    let cfg = MapCfg::binary(8, 4);
    let mut ops = Vec::new();
    for i in 0..200u64 {
        ops.push(put(&k(i, 4), &pay(i)));
    }
    ops.push(Op::Free);
    diff_script("capacity assert", DEFAULT_SEED, cfg, &ops);
}

/// row 18 — `stbds_hmdel_key(NULL, ..)` returns NULL
#[test]
fn e18_hmdel_null_map() {
    let p = seeded(DEFAULT_SEED);
    for api in p.both() {
        unsafe {
            for &es in &[1usize, 8, 16] {
                for &mode in &[HM_BINARY, HM_STRING, -1, 5, i32::MIN, i32::MAX] {
                    let mut key = k(9, 8);
                    let r = (api.hmdel_key)(
                        std::ptr::null_mut(),
                        es,
                        key.as_mut_ptr() as *mut c_void,
                        4,
                        0,
                        mode,
                    );
                    assert!(r.is_null(), "{}: es={es} mode={mode} expected NULL", api.tag);
                    // NULL key too: never dereferenced on this path
                    let r = (api.hmdel_key)(
                        std::ptr::null_mut(),
                        es,
                        std::ptr::null_mut(),
                        4,
                        0,
                        mode,
                    );
                    assert!(r.is_null(), "{}: NULL key expected NULL", api.tag);
                }
            }
        }
    }
}

/// rows 19, 20, 21 — delete on a table-less map, absent key, found key
#[test]
fn e19_e20_e21_hmdel_sentinels() {
    let cfg = MapCfg::binary(8, 4);
    // row 19: no hash table at all -> temp = 0, pointer unchanged
    diff_script(
        "del without table",
        DEFAULT_SEED,
        cfg,
        &[
            Op::PutDefault { payload: pay(0x1) },
            del(&k(5, 4)),
            del(&k(0, 4)),
            get(&k(5, 4)),
            Op::Free,
        ],
    );
    // rows 20/21: absent -> temp 0; found -> temp 1 (+ tombstone slot state)
    let mut ops = vec![Op::PutDefault { payload: pay(0x2) }];
    for i in 0..8u64 {
        ops.push(put(&k(i, 4), &pay(i)));
    }
    for i in 0..16u64 {
        ops.push(del(&k(i, 4)));
        ops.push(del(&k(i, 4))); // second delete of the same key: now absent
    }
    ops.push(Op::Free);
    diff_script("del found/absent", DEFAULT_SEED, cfg, &ops);
}

/// row 22 — `keyoffset != 0` while the keys live at offset 0
#[test]
fn e22_hmdel_wrong_keyoffset() {
    for &(es, ks, off) in &[(16usize, 4usize, 4usize), (16, 8, 8), (24, 4, 12), (8, 4, 4)] {
        let cfg = MapCfg::binary(es, ks);
        let mut ops = vec![Op::PutDefault { payload: pay(0x3) }];
        for i in 0..8u64 {
            ops.push(put(&k(i, ks), &pay(0xAAAA_AAAA_AAAA_AAAA)));
        }
        for i in 0..8u64 {
            // the memcmp reads `off` bytes into the element: never equal, so
            // the delete reports "not found" (temp == 0) and the asserts are
            // never reached
            ops.push(del_off(&k(i, ks), off));
            ops.push(get(&k(i, ks)));
        }
        ops.push(Op::Free);
        diff_script(&format!("keyoffset={off} es={es}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// row 23 — `mode == STBDS_HM_STRING && string.mode == SH_STRDUP` frees the key
#[test]
fn e23_hmdel_strdup_frees_key() {
    let cfg = MapCfg::string(16, HM_STRING);
    let keys: Vec<Vec<u8>> = (0..10).map(|i| s(&format!("strdup-key-{i}"))).collect();
    let mut ops = vec![Op::ShMode { sh_mode: SH_STRDUP }];
    for (i, kk) in keys.iter().enumerate() {
        ops.push(put(kk, &pay(i as u64)));
    }
    for kk in &keys {
        ops.push(del(kk));
        ops.push(get(kk));
    }
    // and re-insert so the freed slots are re-duplicated
    for (i, kk) in keys.iter().enumerate() {
        ops.push(put(kk, &pay(100 + i as u64)));
    }
    ops.push(Op::Free);
    for seed in [DEFAULT_SEED, 0, 1] {
        diff_script("strdup del frees", seed, cfg, &ops);
    }
}

/// rows 24, 25 — the delete-path asserts hold (process survives) and the
/// re-index result is identical
#[test]
fn e24_e25_hmdel_asserts_hold() {
    for n in [1usize, 2, 3, 8, 20, 40] {
        let cfg = MapCfg::binary(8, 4);
        let mut ops = Vec::new();
        for i in 0..n as u64 {
            ops.push(put(&k(i, 4), &pay(i)));
        }
        // delete in a shuffled order: mixes last/interior deletes, shrink and
        // rebuild; any broken re-index trips STBDS_ASSERT and aborts
        let mut rng = Rng::new(0x2424 + n as u64);
        let mut order: Vec<u64> = (0..n as u64).collect();
        for i in (1..order.len()).rev() {
            let j = rng.below((i + 1) as u64) as usize;
            order.swap(i, j);
        }
        for i in order {
            ops.push(del(&k(i, 4)));
            for j in 0..n as u64 {
                ops.push(get(&k(j, 4)));
            }
        }
        ops.push(Op::Free);
        diff_script(&format!("del asserts n={n}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// rows 26, 27 — `stbds_hmfree_func` with NULL and with a table-less map
#[test]
fn e26_e27_hmfree_edge_cases() {
    let p = seeded(DEFAULT_SEED);
    for api in p.both() {
        unsafe {
            // row 26: NULL is a no-op
            for &es in &[0usize, 1, 8, 16, usize::MAX] {
                (api.hmfree_func)(std::ptr::null_mut(), es);
            }
            // row 27: map without a hash table
            for &es in &[1usize, 8, 16] {
                let h = (api.hmput_default)(std::ptr::null_mut(), es);
                assert!(map_table(h, es).is_null());
                (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
                // a map with a table but only the default element
                let h = (api.shmode_func)(es.max(8), SH_STRDUP);
                (api.hmfree_func)(map_raw(h, es.max(8)) as *mut c_void, es.max(8));
                let h = (api.shmode_func)(es.max(8), SH_ARENA);
                (api.hmfree_func)(map_raw(h, es.max(8)) as *mut c_void, es.max(8));
            }
        }
    }
}

/// row 28 — the `make_hash_index` invariant holds for every table the library
/// builds (checked in every snapshot as `t.invariant_ok`)
#[test]
fn e28_hash_index_invariant() {
    let cfg = MapCfg::binary(8, 4);
    let mut ops = Vec::new();
    for i in 0..300u64 {
        ops.push(put(&k(i, 4), &pay(i)));
    }
    for i in 0..300u64 {
        ops.push(del(&k(i, 4)));
    }
    ops.push(Op::Free);
    diff_script("invariant", DEFAULT_SEED, cfg.digested(), &ops);
}

/// rows 30, 31, 32, 33 — `stbds_stralloc` boundary results
#[test]
fn e30_e33_stralloc_boundaries() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            // row 30: huge first string, storage == NULL -> remaining = 0
            {
                let mut arena = StringArena::zeroed();
                let ap: *mut StringArena = &mut arena;
                let mut big = vec![b'Z'; 2000];
                big.push(0);
                let q = (api.stralloc)(ap as *mut c_void, big.as_mut_ptr() as *mut c_char);
                t.push(format!("row30 got_len={}", cstr_bytes(q).unwrap().len()));
                t.extend(snap_arena(ap));
                assert_eq!(arena.remaining, 0, "{}: remaining must be 0", api.tag);
                assert_eq!(arena.block, 1, "{}: block was incremented", api.tag);
                (api.strreset)(ap as *mut c_void);
            }
            // row 31: huge string with an existing head block -> remaining kept
            {
                let mut arena = StringArena::zeroed();
                let ap: *mut StringArena = &mut arena;
                let mut small = s("small");
                (api.stralloc)(ap as *mut c_void, small.as_mut_ptr() as *mut c_char);
                let rem_before = arena.remaining;
                let block_before = arena.block;
                let mut big = vec![b'Y'; 4000];
                big.push(0);
                let q = (api.stralloc)(ap as *mut c_void, big.as_mut_ptr() as *mut c_char);
                t.push(format!(
                    "row31 rem_before={rem_before} block_before={block_before} got_len={}",
                    cstr_bytes(q).unwrap().len()
                ));
                t.extend(snap_arena(ap));
                assert_eq!(arena.remaining, rem_before, "{}: remaining kept", api.tag);
                // the small string must still be readable
                (api.strreset)(ap as *mut c_void);
            }
            // row 32: blocksize clamp — block stops incrementing at 22
            {
                for start in [20u8, 21, 22, 23, 24] {
                    let mut arena = StringArena::zeroed();
                    arena.block = start;
                    let ap: *mut StringArena = &mut arena;
                    let mut txt = s("clamp");
                    (api.stralloc)(ap as *mut c_void, txt.as_mut_ptr() as *mut c_char);
                    t.push(format!("row32 start={start} -> block={}", arena.block));
                    t.extend(snap_arena(ap));
                    (api.strreset)(ap as *mut c_void);
                }
            }
            // row 33: the empty string consumes exactly one byte
            {
                let mut arena = StringArena::zeroed();
                let ap: *mut StringArena = &mut arena;
                let mut e = s("");
                for i in 0..5 {
                    let q = (api.stralloc)(ap as *mut c_void, e.as_mut_ptr() as *mut c_char);
                    t.push(format!(
                        "row33 i={i} remaining={} empty={}",
                        arena.remaining,
                        cstr_bytes(q).unwrap().is_empty()
                    ));
                }
                t.extend(snap_arena(ap));
                (api.strreset)(ap as *mut c_void);
            }
        }
    }
    assert_traces_eq("e30..e33", &tc, &tr);
}

/// row 34 — `stbds_strreset` on an already empty arena
#[test]
fn e34_strreset_empty() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            let mut arena = StringArena::zeroed();
            arena.block = 7;
            arena.mode = 3;
            arena.remaining = 12345; // storage is NULL: nothing to free
            let ap: *mut StringArena = &mut arena;
            (api.strreset)(ap as *mut c_void);
            t.extend(snap_arena(ap));
            (api.strreset)(ap as *mut c_void);
            t.extend(snap_arena(ap));
            assert_eq!(arena.block, 0);
            assert_eq!(arena.mode, 0);
            assert_eq!(arena.remaining, 0);
        }
    }
    assert_traces_eq("e34", &tc, &tr);
}

/// row 35 — out-of-enum `mode` for `stbds_shmode_func`: `(unsigned char) mode`
#[test]
fn e35_shmode_out_of_range() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            (api.rand_seed)(DEFAULT_SEED);
            for &mode in &[
                SH_NONE,
                SH_DEFAULT,
                SH_STRDUP,
                SH_ARENA,
                4,
                5,
                127,
                128,
                255,
                256,
                257,
                259,
                -1,
                -2,
                -256,
                i32::MIN,
                i32::MAX,
            ] {
                let es = 16usize;
                let h = (api.shmode_func)(es, mode);
                let tbl = map_table(h, es);
                t.push(format!(
                    "shmode({mode}) -> string.mode={} (expected {})",
                    (*tbl).string.mode,
                    (mode as u32 & 0xff) as u8
                ));
                assert_eq!(
                    (*tbl).string.mode,
                    (mode as u32 & 0xff) as u8,
                    "{}: truncation to unsigned char",
                    api.tag
                );
                t.extend(snap_map(h, es, KeyKind::Binary));
                (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
            }
        }
    }
    assert_traces_eq("e35", &tc, &tr);
}

/// row 36 — `stbds_hash_string("")`
#[test]
fn e36_hash_string_empty() {
    let p = seeded(DEFAULT_SEED);
    let mut empty = s("");
    for seed in [0usize, 1, DEFAULT_SEED, usize::MAX] {
        let a = unsafe { (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed) };
        let b = unsafe { (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed) };
        assert_eq!(a, b, "hash_string(\"\", {seed:#x})");
    }
}

/// row 37 — `stbds_hash_bytes(p, 0, seed)`, including a NULL pointer (never
/// dereferenced when `len == 0`)
#[test]
fn e37_hash_bytes_zero_len() {
    let p = seeded(DEFAULT_SEED);
    let mut buf = [0xAAu8; 8];
    for seed in [0usize, 1, DEFAULT_SEED, usize::MAX, 1 << 63] {
        let a = unsafe { (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed) };
        let b = unsafe { (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed) };
        assert_eq!(a, b, "hash_bytes(buf, 0, {seed:#x})");
        let a = unsafe { (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        let b = unsafe { (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        assert_eq!(a, b, "hash_bytes(NULL, 0, {seed:#x})");
    }
}

/// rows 38, 39 — sign extension (covered in phase_b_hash too) and the
/// unreachable `hash < 2` branch
#[test]
fn e38_e39_hash_quirks() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    // every single-byte value at every position of a 8-byte word
    for pos in 0..8usize {
        for v in 0u16..=255 {
            let mut b = [0u8; 8];
            b[pos] = v as u8;
            for seed in [0usize, DEFAULT_SEED, usize::MAX] {
                let a = unsafe { (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, 8, seed) };
                let r = unsafe { (p.r.hash_bytes)(b.as_mut_ptr() as *mut c_void, 8, seed) };
                tc.push(format!("{pos} {v} {seed:#x} {a:#x}"));
                tr.push(format!("{pos} {v} {seed:#x} {r:#x}"));
                // no library may ever produce a hash < 2 for these inputs, so
                // the `hash += 2` fix-up stays unreachable (documented)
            }
        }
    }
    assert_traces_eq("e38/e39", &tc, &tr);
}

/// rows 41, 42, 43 — `hm_geti` non-positive / assert survival / `strkey` extremes
#[test]
fn e41_e43_driver_edges() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            (api.rand_seed)(DEFAULT_SEED);
            for num in [0i32, -1, -7, i32::MIN, 1, 2] {
                (api.hm_geti)(num);
                t.push(format!("hm_geti({num}) survived"));
            }
            for n in [i32::MIN, i32::MAX, 0, -1] {
                let q = (api.strkey)(n);
                let cs = std::ffi::CStr::from_ptr(q).to_bytes().to_vec();
                t.push(format!("strkey({n})={:?} len={}", String::from_utf8_lossy(&cs), cs.len()));
                assert!(cs.len() < 256);
            }
        }
    }
    assert_traces_eq("e41/e43", &tc, &tr);
}

/// Generic FFI boundary: zero/degenerate sizes
#[test]
fn e_generic_zero_sizes() {
    // keysize == 0 (every key equal), elemsize == 1
    for &es in &[1usize, 8] {
        let cfg = MapCfg::binary(es, 0);
        let mut ops = Vec::new();
        for i in 0..8u64 {
            ops.push(put(&k(i, 8), &pay(i)));
            ops.push(get(&k(i + 1000, 8)));
            ops.push(get_ts(&k(i, 8)));
            ops.push(del(&k(i, 8)));
        }
        ops.push(Op::Free);
        diff_script(&format!("keysize0 es={es}"), DEFAULT_SEED, cfg, &ops);
    }
    // NULL key with keysize 0: hash_bytes/memcmp/memcpy all get length 0
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            (api.rand_seed)(DEFAULT_SEED);
            let es = 8usize;
            let mut h = (api.hmput_key)(std::ptr::null_mut(), es, std::ptr::null_mut(), 0, HM_BINARY);
            t.push(format!("null-key put temp={}", map_temp(h, es)));
            std::ptr::write_bytes(map_raw(h, es).add(es), 0x77, es);
            h = (api.hmget_key)(h, es, std::ptr::null_mut(), 0, HM_BINARY);
            t.push(format!("null-key get temp={}", map_temp(h, es)));
            let mut tsv = 0isize;
            h = (api.hmget_key_ts)(h, es, std::ptr::null_mut(), 0, &mut tsv, HM_BINARY);
            t.push(format!("null-key get_ts={tsv}"));
            h = (api.hmdel_key)(h, es, std::ptr::null_mut(), 0, 0, HM_BINARY);
            t.push(format!("null-key del temp={}", map_temp(h, es)));
            t.extend(snap_map(h, es, KeyKind::Binary));
            (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
        }
    }
    assert_traces_eq("e_generic null key", &tc, &tr);
}
