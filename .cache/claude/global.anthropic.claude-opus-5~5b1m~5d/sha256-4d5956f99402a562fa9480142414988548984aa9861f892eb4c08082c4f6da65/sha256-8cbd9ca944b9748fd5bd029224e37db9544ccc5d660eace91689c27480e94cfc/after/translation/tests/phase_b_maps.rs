//! Phase B — differential tests for the hash-map entry points, driven exactly
//! the way the `stbds_hm*` / `stbds_sh*` macros drive them.
//!
//! Element layouts are chosen so that **every** byte of every element is
//! written by either the library or the test, i.e. there is no struct padding
//! whose value would be uninitialised `realloc` memory.
//!
//! CONFIGS.md rows 8-23, 27, 31-35.

mod common;

use common::*;
use std::ffi::{c_void, CString};

/// Binary-key element: `ks` key bytes at offset 0 followed by an `i32` value.
fn bin_kind(ks: usize) -> KeyKind {
    KeyKind::Bytes { ks, cmp_end: ks + 4 }
}
fn bin_elemsize(ks: usize) -> usize {
    ks + 4
}

/// String-key element: `char *key` at offset 0, `i32 value` at offset 8,
/// four bytes of trailing padding (never compared).
const STR_ES: usize = 16;
const STR_KS: usize = 8;
const STR_VOFF: usize = 8;
fn str_kind() -> KeyKind {
    KeyKind::StrPtr { off: 0, cmp_end: 12 }
}

/// Distinct random byte keys of width `ks`.
fn distinct_keys(rng: &mut Rng, ks: usize, n: usize) -> Vec<Vec<u8>> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let capacity_limit = if ks == 1 { 256usize } else { usize::MAX };
    while out.len() < n.min(capacity_limit) {
        let k = rng.bytes(ks);
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    out
}

fn distinct_strings(rng: &mut Rng, n: usize) -> Vec<CString> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let len = 1 + rng.below(24);
        let s = rng.cstring(len);
        if seen.insert(s.clone()) {
            out.push(s);
        }
    }
    out
}

/// Seed both libraries identically so their tables get identical hash seeds.
fn seed(p: &Pair, s: usize) {
    unsafe {
        (p.c.rand_seed)(s);
        (p.r.rand_seed)(s);
    }
}

// ===========================================================================
// Rows 8-10 — binary maps, every key width, insert + lookup + duplicates
// ===========================================================================

#[test]
fn cfg_08_binary_map_widths() {
    let (_g, p) = libs();
    for &ks in &[1usize, 2, 3, 4, 5, 7, 8, 9, 16] {
        let es = bin_elemsize(ks);
        for &n in &[1usize, 2, 7, 8, 9, 50, 300] {
            let n = if ks == 1 { n.min(200) } else { n };
            let mut rng = Rng::new(0x1000 + (ks as u64) * 977 + n as u64);
            seed(p, 0x3141_5926);
            let keys = distinct_keys(&mut rng, ks, n);
            unsafe {
                let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
                let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
                for (i, k) in keys.iter().enumerate() {
                    let mut kb = k.clone();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    let tc = mc.put_key(kp);
                    let tr = mr.put_key(kp);
                    assert_eq!(tc, tr, "put temp ks={ks} n={n} i={i}");
                    mc.set_i32(tc, ks, i as i32 * 7 - 3);
                    mr.set_i32(tr, ks, i as i32 * 7 - 3);
                    assert_eq!(
                        mc.snap(bin_kind(ks)),
                        mr.snap(bin_kind(ks)),
                        "state after put ks={ks} n={n} i={i}"
                    );
                }
                // lookups: all present
                for (i, k) in keys.iter().enumerate() {
                    let mut kb = k.clone();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    let ic = mc.geti(kp);
                    let ir = mr.geti(kp);
                    assert_eq!(ic, ir, "geti ks={ks} n={n} i={i}");
                    assert_eq!(mc.get_i32(ic, ks), i as i32 * 7 - 3);
                    assert_eq!(mr.get_i32(ir, ks), i as i32 * 7 - 3);
                }
                // lookups: absent keys
                for _ in 0..20 {
                    let mut kb = rng.bytes(ks);
                    if keys.iter().any(|k| *k == kb) {
                        continue;
                    }
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    assert_eq!(mc.geti(kp), mr.geti(kp), "geti miss ks={ks}");
                }
                assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)));
                mc.free();
                mr.free();
            }
        }
    }
}

#[test]
fn cfg_09_binary_map_growth() {
    let (_g, p) = libs();
    let ks = 4usize;
    let es = bin_elemsize(ks);
    let mut rng = Rng::new(0x9E00_0001);
    seed(p, 0x3141_5926);
    let keys = distinct_keys(&mut rng, ks, 300);
    unsafe {
        let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
        let mut slot_counts = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            let mut kb = k.clone();
            let kp = kb.as_mut_ptr() as *mut c_void;
            let tc = mc.put_key(kp);
            let tr = mr.put_key(kp);
            assert_eq!(tc, tr, "i={i}");
            mc.set_i32(tc, ks, i as i32);
            mr.set_i32(tr, ks, i as i32);
            let sc = mc.snap(bin_kind(ks));
            assert_eq!(sc, mr.snap(bin_kind(ks)), "i={i}");
            let n = sc.table.as_ref().unwrap().slot_count;
            if slot_counts.last() != Some(&n) {
                slot_counts.push(n);
            }
        }
        assert_eq!(slot_counts, vec![8, 16, 32, 64, 128, 256, 512], "growth ladder");
        mc.free();
        mr.free();
    }
}

#[test]
fn cfg_10_binary_map_duplicates() {
    let (_g, p) = libs();
    let ks = 4usize;
    let es = bin_elemsize(ks);
    let mut rng = Rng::new(0xD0_0D_0001);
    seed(p, 7);
    let keys = distinct_keys(&mut rng, ks, 40);
    unsafe {
        let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
        for round in 0..6 {
            for (i, k) in keys.iter().enumerate() {
                let mut kb = k.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                let tc = mc.put_key(kp);
                let tr = mr.put_key(kp);
                assert_eq!(tc, tr, "dup put round={round} i={i}");
                mc.set_i32(tc, ks, (round * 1000 + i) as i32);
                mr.set_i32(tr, ks, (round * 1000 + i) as i32);
                assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)));
            }
            assert_eq!(mc.len(), 40, "duplicate puts must not grow the map");
            assert_eq!(mr.len(), 40);
        }
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Rows 11-14 — deletion orders, shrink / rebuild, tombstone reuse
// ===========================================================================

unsafe fn build_binary<'a>(
    p: &'a Pair,
    ks: usize,
    keys: &[Vec<u8>],
) -> (Map<'a>, Map<'a>) {
    let es = bin_elemsize(ks);
    let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
    let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
    for (i, k) in keys.iter().enumerate() {
        let mut kb = k.clone();
        let kp = kb.as_mut_ptr() as *mut c_void;
        let tc = mc.put_key(kp);
        let tr = mr.put_key(kp);
        assert_eq!(tc, tr);
        mc.set_i32(tc, ks, i as i32);
        mr.set_i32(tr, ks, i as i32);
    }
    (mc, mr)
}

fn del_order_test(name: &str, order: impl Fn(&mut Rng, usize) -> Vec<usize>) {
    let (_g, p) = libs();
    let ks = 4usize;
    for &n in &[1usize, 2, 8, 9, 64, 200] {
        let mut rng = Rng::new(0x9000 + n as u64);
        seed(p, 0x3141_5926);
        let keys = distinct_keys(&mut rng, ks, n);
        unsafe {
            let (mut mc, mut mr) = build_binary(p, ks, &keys);
            assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)), "{name} n={n} built");
            for (step, idx) in order(&mut rng, n).into_iter().enumerate() {
                let mut kb = keys[idx].clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                let dc = mc.del(kp);
                let dr = mr.del(kp);
                assert_eq!(dc, dr, "{name} n={n} step={step} del return");
                assert_eq!(
                    mc.snap(bin_kind(ks)),
                    mr.snap(bin_kind(ks)),
                    "{name} n={n} step={step} state after del"
                );
                // every remaining key must still be findable, identically
                for (j, k2) in keys.iter().enumerate() {
                    let mut kb2 = k2.clone();
                    let kp2 = kb2.as_mut_ptr() as *mut c_void;
                    assert_eq!(mc.geti(kp2), mr.geti(kp2), "{name} n={n} step={step} geti j={j}");
                }
            }
            assert_eq!(mc.len(), 0);
            assert_eq!(mr.len(), 0);
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn cfg_11_binary_del_forward() {
    del_order_test("forward", |_r, n| (0..n).collect());
}

#[test]
fn cfg_12_binary_del_reverse() {
    del_order_test("reverse", |_r, n| (0..n).rev().collect());
}

#[test]
fn cfg_13_binary_del_random() {
    del_order_test("random", |r, n| {
        let mut v: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = r.below(i + 1);
            v.swap(i, j);
        }
        v
    });
}

#[test]
fn cfg_14_binary_tombstone_reuse() {
    let (_g, p) = libs();
    let ks = 4usize;
    let mut rng = Rng::new(0x70_0B_0001);
    seed(p, 0x3141_5926);
    let keys = distinct_keys(&mut rng, ks, 128);
    unsafe {
        let (mut mc, mut mr) = build_binary(p, ks, &keys[..64]);
        // delete half
        for i in (0..64).step_by(2) {
            let mut kb = keys[i].clone();
            let kp = kb.as_mut_ptr() as *mut c_void;
            assert_eq!(mc.del(kp), mr.del(kp), "del i={i}");
            assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)), "after del i={i}");
        }
        let tombstones = mc.snap(bin_kind(ks)).table.unwrap().tombstone_count;
        assert!(tombstones > 0, "expected live tombstones, got {tombstones}");
        // insert new keys -> must reuse the tombstoned slots
        for i in 64..128 {
            let mut kb = keys[i].clone();
            let kp = kb.as_mut_ptr() as *mut c_void;
            let tc = mc.put_key(kp);
            let tr = mr.put_key(kp);
            assert_eq!(tc, tr, "reinsert i={i}");
            mc.set_i32(tc, ks, i as i32);
            mr.set_i32(tr, ks, i as i32);
            assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)), "reinsert state i={i}");
        }
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Row 15 — hmput_default (map with no hash table) then hmput_key
// ===========================================================================

#[test]
fn cfg_15_hmput_default_then_put() {
    let (_g, p) = libs();
    let ks = 4usize;
    let es = bin_elemsize(ks);
    seed(p, 0x3141_5926);
    let mut rng = Rng::new(0xDEF0);
    unsafe {
        let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
        mc.put_default();
        mr.put_default();
        // default element value
        mc.set_i32(-1, ks, -2);
        mr.set_i32(-1, ks, -2);
        let sc = mc.snap(bin_kind(ks));
        assert_eq!(sc, mr.snap(bin_kind(ks)), "after hmput_default");
        assert!(sc.table.is_none(), "hmput_default must not create a hash table");

        // lookup on a table-less map
        let mut kb = rng.bytes(ks);
        let kp = kb.as_mut_ptr() as *mut c_void;
        assert_eq!(mc.geti(kp), -1);
        assert_eq!(mr.geti(kp), -1);
        assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)));

        // second hmput_default is a no-op (length != 0)
        mc.put_default();
        mr.put_default();
        assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)));
        assert_eq!(mc.len(), 0);

        // now insert for real -> table is created
        let keys = distinct_keys(&mut rng, ks, 30);
        for (i, k) in keys.iter().enumerate() {
            let mut kb = k.clone();
            let kp = kb.as_mut_ptr() as *mut c_void;
            let tc = mc.put_key(kp);
            let tr = mr.put_key(kp);
            assert_eq!(tc, tr, "i={i}");
            mc.set_i32(tc, ks, i as i32);
            mr.set_i32(tr, ks, i as i32);
            assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)), "i={i}");
        }
        assert!(mc.snap(bin_kind(ks)).table.is_some());
        // the default element must be preserved
        assert_eq!(mc.get_i32(-1, ks), -2);
        assert_eq!(mr.get_i32(-1, ks), -2);
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Row 16 — shmode_func(STBDS_SH_NONE) with a string `mode`
// ===========================================================================

/// `string.mode == STBDS_SH_NONE` makes `hmput_key`'s `switch` fall through to
/// `default: memcpy(elem, key, keysize)` even when `mode >= STBDS_HM_STRING`,
/// so the *bytes of the string* are copied into the key slot rather than the
/// pointer.  Only one insert is safe (a second one could `strcmp` those bytes
/// as a pointer, which crashes in C too), so exactly one insert is compared.
#[test]
fn cfg_16_shmode_none_string_mode() {
    let (_g, p) = libs();
    for &mode in &[STBDS_HM_STRING, 2, 9] {
        seed(p, 0x3141_5926);
        let key = CString::new("a-key-far-longer-than-eight-bytes").unwrap();
        unsafe {
            let mut mc = Map::shmode(&p.c, STR_ES, STR_KS, mode, STBDS_SH_NONE);
            let mut mr = Map::shmode(&p.r, STR_ES, STR_KS, mode, STBDS_SH_NONE);
            let kp = key.as_ptr() as *mut c_void;
            let tc = mc.put_key(kp);
            let tr = mr.put_key(kp);
            assert_eq!(tc, tr, "mode={mode}");
            mc.set_i32(tc, STR_VOFF, 1234);
            mr.set_i32(tr, STR_VOFF, 1234);
            // key bytes were memcpy'd verbatim -> compare as raw bytes
            let kind = KeyKind::Bytes { ks: STR_KS, cmp_end: 12 };
            let sc = mc.snap(kind);
            assert_eq!(sc, mr.snap(kind), "mode={mode} SH_NONE memcpy branch");
            assert_eq!(
                sc.elems[1].0.as_deref().unwrap(),
                &key.as_bytes()[..8],
                "the first 8 bytes of the string must be memcpy'd into the key slot"
            );
            assert_eq!(sc.table.as_ref().unwrap().arena_mode, 0);
            mc.free();
            mr.free();
        }
    }
}

// ===========================================================================
// Rows 17-21 — string maps in every `string.mode`
// ===========================================================================

fn string_map_test(sh_mode: c_int_alias, tag: &str) {
    let (_g, p) = libs();
    for &n in &[1usize, 2, 8, 9, 64, 257] {
        let mut rng = Rng::new(0x5100u64.wrapping_add(n as u64).wrapping_add((sh_mode as i64 as u64).wrapping_mul(31)));
        seed(p, 0x3141_5926);
        let keys = distinct_strings(&mut rng, n);
        unsafe {
            let mut mc = if sh_mode < 0 {
                Map::new(&p.c, STR_ES, STR_KS, STBDS_HM_STRING)
            } else {
                Map::shmode(&p.c, STR_ES, STR_KS, STBDS_HM_STRING, sh_mode)
            };
            let mut mr = if sh_mode < 0 {
                Map::new(&p.r, STR_ES, STR_KS, STBDS_HM_STRING)
            } else {
                Map::shmode(&p.r, STR_ES, STR_KS, STBDS_HM_STRING, sh_mode)
            };
            // shdefault(map, -2)
            mc.put_default();
            mr.put_default();
            mc.set_i32(-1, STR_VOFF, -2);
            mr.set_i32(-1, STR_VOFF, -2);

            for (i, k) in keys.iter().enumerate() {
                let kp = k.as_ptr() as *mut c_void;
                let tc = mc.put_key(kp);
                let tr = mr.put_key(kp);
                assert_eq!(tc, tr, "{tag} n={n} put i={i}");
                mc.set_i32(tc, STR_VOFF, i as i32 * 3);
                mr.set_i32(tr, STR_VOFF, i as i32 * 3);
                assert_eq!(
                    map_temp_key(mc.t, STR_ES),
                    map_temp_key(mr.t, STR_ES),
                    "{tag} n={n} temp_key after put i={i}"
                );
                assert_eq!(
                    mc.snap(str_kind()),
                    mr.snap(str_kind()),
                    "{tag} n={n} state after put i={i}"
                );
            }

            // hits
            for (i, k) in keys.iter().enumerate() {
                let kp = k.as_ptr() as *mut c_void;
                let ic = mc.geti(kp);
                let ir = mr.geti(kp);
                assert_eq!(ic, ir, "{tag} n={n} geti i={i}");
                assert_eq!(mc.get_i32(ic, STR_VOFF), i as i32 * 3);
                assert_eq!(mr.get_i32(ir, STR_VOFF), i as i32 * 3);
            }
            // misses
            for _ in 0..20 {
                let len = 25 + rng.below(8);
                let miss = rng.cstring(len);
                let kp = miss.as_ptr() as *mut c_void;
                assert_eq!(mc.geti(kp), -1, "{tag} miss");
                assert_eq!(mr.geti(kp), -1, "{tag} miss");
            }
            assert_eq!(mc.snap(str_kind()), mr.snap(str_kind()), "{tag} n={n} after gets");

            // delete everything, in a shuffled order
            let mut order: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            for (step, idx) in order.into_iter().enumerate() {
                let kp = keys[idx].as_ptr() as *mut c_void;
                let dc = mc.del(kp);
                let dr = mr.del(kp);
                assert_eq!(dc, dr, "{tag} n={n} del step={step}");
                assert_eq!(
                    mc.snap(str_kind()),
                    mr.snap(str_kind()),
                    "{tag} n={n} state after del step={step}"
                );
            }
            assert_eq!(mc.len(), 0);
            assert_eq!(mr.len(), 0);
            // every key now resolves to the default element
            for k in &keys {
                let kp = k.as_ptr() as *mut c_void;
                assert_eq!(mc.geti(kp), -1);
                assert_eq!(mr.geti(kp), -1);
                assert_eq!(mc.get_i32(-1, STR_VOFF), -2);
                assert_eq!(mr.get_i32(-1, STR_VOFF), -2);
            }
            mc.free();
            mr.free();
        }
    }
}

#[allow(non_camel_case_types)]
type c_int_alias = std::ffi::c_int;

#[test]
fn cfg_17_shmode_default() {
    string_map_test(STBDS_SH_DEFAULT, "SH_DEFAULT");
}

#[test]
fn cfg_18_shmode_strdup() {
    string_map_test(STBDS_SH_STRDUP, "SH_STRDUP");
}

#[test]
fn cfg_19_shmode_arena() {
    string_map_test(STBDS_SH_ARENA, "SH_ARENA");
}

#[test]
fn cfg_20_implicit_string_map() {
    // sh_mode < 0 => no shmode_func call: the table is created implicitly by
    // hmput_key, which sets string.mode = STBDS_SH_DEFAULT for mode >= 1.
    string_map_test(-1, "implicit-string");
}

#[test]
fn cfg_21_implicit_binary_map() {
    let (_g, p) = libs();
    for &ks in &[4usize, 8] {
        let es = bin_elemsize(ks);
        let mut rng = Rng::new(0x2100 + ks as u64);
        seed(p, 0x3141_5926);
        let keys = distinct_keys(&mut rng, ks, 100);
        unsafe {
            let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
            let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
            for (i, k) in keys.iter().enumerate() {
                let mut kb = k.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                let tc = mc.put_key(kp);
                let tr = mr.put_key(kp);
                assert_eq!(tc, tr);
                mc.set_i32(tc, ks, i as i32);
                mr.set_i32(tr, ks, i as i32);
            }
            let sc = mc.snap(bin_kind(ks));
            assert_eq!(sc, mr.snap(bin_kind(ks)));
            assert_eq!(
                sc.table.as_ref().unwrap().arena_mode,
                0,
                "binary mode must leave string.mode = STBDS_SH_NONE"
            );
            for (i, k) in keys.iter().enumerate() {
                let mut kb = k.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                assert_eq!(mc.del(kp), mr.del(kp), "del i={i}");
                assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)), "del i={i}");
            }
            mc.free();
            mr.free();
        }
    }
}

// ===========================================================================
// Row 22 — the low-level stbds_hmget_key_ts entry point
// ===========================================================================

#[test]
fn cfg_22_hmget_key_ts_lowlevel() {
    let (_g, p) = libs();
    let ks = 4usize;
    let es = bin_elemsize(ks);
    let mut rng = Rng::new(0x7700);
    seed(p, 0x3141_5926);

    // (a) a == NULL
    unsafe {
        let mut kb = rng.bytes(ks);
        let kp = kb.as_mut_ptr() as *mut c_void;
        let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
        let tc = mc.geti_ts(kp);
        let tr = mr.geti_ts(kp);
        assert_eq!((tc, tr), (-1, -1), "hmget_key_ts(NULL) must yield temp = -1");
        let sc = mc.snap(bin_kind(ks));
        assert_eq!(sc, mr.snap(bin_kind(ks)));
        assert_eq!(sc.length, 1);
        assert!(sc.table.is_none());
        // NB: hmget_key_ts does NOT write the array's `temp` field.
        assert_eq!(sc.temp, mr.snap(bin_kind(ks)).temp);
        mc.free();
        mr.free();
    }

    // (b) table-less map, (c) populated map: hits and misses
    for &n in &[1usize, 8, 9, 100] {
        seed(p, 0x3141_5926);
        let keys = distinct_keys(&mut rng, ks, n);
        unsafe {
            let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
            let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
            mc.put_default();
            mr.put_default();
            let mut kb = keys[0].clone();
            let kp = kb.as_mut_ptr() as *mut c_void;
            assert_eq!(mc.geti_ts(kp), -1, "table-less ts lookup");
            assert_eq!(mr.geti_ts(kp), -1);

            for (i, k) in keys.iter().enumerate() {
                let mut kb = k.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                let tc = mc.put_key(kp);
                let tr = mr.put_key(kp);
                assert_eq!(tc, tr);
                mc.set_i32(tc, ks, i as i32);
                mr.set_i32(tr, ks, i as i32);
            }
            for (i, k) in keys.iter().enumerate() {
                let mut kb = k.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                let tc = mc.geti_ts(kp);
                let tr = mr.geti_ts(kp);
                assert_eq!(tc, tr, "ts hit n={n} i={i}");
                assert_eq!(mc.get_i32(tc, ks), i as i32);
            }
            for _ in 0..20 {
                let mut kb = rng.bytes(ks);
                if keys.iter().any(|k| *k == kb) {
                    continue;
                }
                let kp = kb.as_mut_ptr() as *mut c_void;
                assert_eq!(mc.geti_ts(kp), mr.geti_ts(kp), "ts miss n={n}");
            }
            assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)));
            mc.free();
            mr.free();
        }
    }
}

// ===========================================================================
// Row 23 — non-zero `keyoffset` (only `stbds_hmdel_key` takes one)
// ===========================================================================

/// `stbds_hmput_key` hardcodes `keyoffset = 0`, so the key always lands at
/// offset 0; `stbds_hmdel_key` however forwards a caller-supplied `keyoffset`
/// to `stbds_hm_find_slot` / `stbds_is_key_equal`.  Element layout here is
/// `[key(4) | mirror(4) | value(4)]` and the test mirrors the key into offset 4
/// after every insert so that both offsets are consistent.
#[test]
fn cfg_23_keyoffset_nonzero() {
    let (_g, p) = libs();
    let ks = 4usize;
    let es = 12usize;
    for &n in &[1usize, 2, 9, 64, 150] {
        let mut rng = Rng::new(0x2300 + n as u64);
        seed(p, 0x3141_5926);
        let keys = distinct_keys(&mut rng, ks, n);
        unsafe {
            let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
            let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
            mc.keyoffset = 4;
            mr.keyoffset = 4;
            for (i, k) in keys.iter().enumerate() {
                let mut kb = k.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                let tc = mc.put_key(kp);
                let tr = mr.put_key(kp);
                assert_eq!(tc, tr);
                for (m, base) in [(mc.t, tc), (mr.t, tr)] {
                    let e = (m as *mut u8).offset(base * es as isize);
                    std::ptr::copy_nonoverlapping(k.as_ptr(), e.add(4), ks);
                }
                mc.set_i32(tc, 8, i as i32);
                mr.set_i32(tr, 8, i as i32);
            }
            let kind = KeyKind::Bytes { ks: 4, cmp_end: 12 };
            assert_eq!(mc.snap(kind), mr.snap(kind), "keyoffset built n={n}");
            let mut order: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            for (step, idx) in order.into_iter().enumerate() {
                let mut kb = keys[idx].clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                let dc = mc.del(kp);
                let dr = mr.del(kp);
                assert_eq!(dc, dr, "keyoffset del n={n} step={step}");
                assert_eq!(mc.snap(kind), mr.snap(kind), "keyoffset del state n={n} step={step}");
            }
            mc.free();
            mr.free();
        }
    }
}

// ===========================================================================
// Row 27 — stbds_hmfree_func in every string mode + table-less
// ===========================================================================

#[test]
fn cfg_27_hmfree_all_modes() {
    let (_g, p) = libs();
    let mut rng = Rng::new(0x2700);
    for sh in [STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        seed(p, 0x3141_5926);
        let keys = distinct_strings(&mut rng, 40);
        unsafe {
            let mode = if sh == STBDS_SH_NONE { STBDS_HM_BINARY } else { STBDS_HM_STRING };
            let mut mc = Map::shmode(&p.c, STR_ES, STR_KS, mode, sh);
            let mut mr = Map::shmode(&p.r, STR_ES, STR_KS, mode, sh);
            for (i, k) in keys.iter().enumerate() {
                let kp = k.as_ptr() as *mut c_void;
                let tc = mc.put_key(kp);
                let tr = mr.put_key(kp);
                assert_eq!(tc, tr, "sh={sh} i={i}");
                mc.set_i32(tc, STR_VOFF, i as i32);
                mr.set_i32(tr, STR_VOFF, i as i32);
            }
            let kind = if sh == STBDS_SH_NONE {
                KeyKind::Bytes { ks: STR_KS, cmp_end: 12 }
            } else {
                str_kind()
            };
            assert_eq!(mc.snap(kind), mr.snap(kind), "sh={sh}");
            mc.free();
            mr.free();
            assert!(mc.t.is_null() && mr.t.is_null());
        }
    }
    // table-less map (hmput_default only)
    seed(p, 0x3141_5926);
    unsafe {
        let mut mc = Map::new(&p.c, 8, 4, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, 8, 4, STBDS_HM_BINARY);
        mc.put_default();
        mr.put_default();
        assert!(mc.snap(KeyKind::Bytes { ks: 4, cmp_end: 8 }).table.is_none());
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Rows 31-35 — randomized full-pipeline fuzz with deep state comparison
// ===========================================================================

fn fuzz_binary(tag: &str, ks: usize, rng_seed: u64, hash_seed: usize, nops: usize, keyspace: usize) {
    let (_g, p) = libs();
    let es = bin_elemsize(ks);
    let mut rng = Rng::new(rng_seed);
    seed(p, hash_seed);
    let keys = distinct_keys(&mut rng, ks, keyspace);
    let kind = bin_kind(ks);
    unsafe {
        let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
        mc.put_default();
        mr.put_default();
        mc.set_i32(-1, ks, -2);
        mr.set_i32(-1, ks, -2);
        for op in 0..nops {
            let idx = rng.below(keys.len());
            let mut kb = keys[idx].clone();
            let kp = kb.as_mut_ptr() as *mut c_void;
            match rng.below(10) {
                0..=3 => {
                    let v = rng.next_u32() as i32;
                    let tc = mc.put_key(kp);
                    let tr = mr.put_key(kp);
                    assert_eq!(tc, tr, "{tag} op={op} put");
                    mc.set_i32(tc, ks, v);
                    mr.set_i32(tr, ks, v);
                }
                4..=5 => {
                    assert_eq!(mc.geti(kp), mr.geti(kp), "{tag} op={op} geti");
                }
                6 => {
                    assert_eq!(mc.geti_ts(kp), mr.geti_ts(kp), "{tag} op={op} geti_ts");
                }
                7 => {
                    mc.put_default();
                    mr.put_default();
                }
                _ => {
                    assert_eq!(mc.del(kp), mr.del(kp), "{tag} op={op} del");
                }
            }
            assert_eq!(mc.snap(kind), mr.snap(kind), "{tag} op={op} state");
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn cfg_31_fuzz_binary_pipeline() {
    fuzz_binary("bin4", 4, 0x3131_3131, 0x3141_5926, 4000, 512);
    fuzz_binary("bin4-seed0", 4, 0xABCD_1234, 0, 2000, 64);
    fuzz_binary("bin1", 1, 0x1111_2222, 99, 2000, 200);
    fuzz_binary("bin3", 3, 0x3333_4444, usize::MAX, 2000, 300);
}

#[test]
fn cfg_35_fuzz_binary_wide_keys() {
    fuzz_binary("bin8", 8, 0x8888_1111, 0x3141_5926, 3000, 400);
    fuzz_binary("bin16", 16, 0x1616_1616, 7, 3000, 400);
    fuzz_binary("bin9", 9, 0x0909_0909, 0xDEAD, 2000, 300);
    fuzz_binary("bin7", 7, 0x0707_0707, 1, 2000, 300);
}

fn fuzz_string(tag: &str, sh: c_int_alias, rng_seed: u64, hash_seed: usize, nops: usize, keyspace: usize) {
    let (_g, p) = libs();
    let mut rng = Rng::new(rng_seed);
    seed(p, hash_seed);
    let keys = distinct_strings(&mut rng, keyspace);
    let kind = str_kind();
    unsafe {
        let mut mc = Map::shmode(&p.c, STR_ES, STR_KS, STBDS_HM_STRING, sh);
        let mut mr = Map::shmode(&p.r, STR_ES, STR_KS, STBDS_HM_STRING, sh);
        mc.put_default();
        mr.put_default();
        mc.set_i32(-1, STR_VOFF, -2);
        mr.set_i32(-1, STR_VOFF, -2);
        for op in 0..nops {
            let idx = rng.below(keys.len());
            let kp = keys[idx].as_ptr() as *mut c_void;
            match rng.below(10) {
                0..=3 => {
                    let v = rng.next_u32() as i32;
                    let tc = mc.put_key(kp);
                    let tr = mr.put_key(kp);
                    assert_eq!(tc, tr, "{tag} op={op} put");
                    mc.set_i32(tc, STR_VOFF, v);
                    mr.set_i32(tr, STR_VOFF, v);
                    assert_eq!(
                        map_temp_key(mc.t, STR_ES),
                        map_temp_key(mr.t, STR_ES),
                        "{tag} op={op} temp_key"
                    );
                }
                4..=5 => {
                    assert_eq!(mc.geti(kp), mr.geti(kp), "{tag} op={op} geti");
                }
                6 => {
                    assert_eq!(mc.geti_ts(kp), mr.geti_ts(kp), "{tag} op={op} geti_ts");
                }
                7 => {
                    mc.put_default();
                    mr.put_default();
                }
                _ => {
                    assert_eq!(mc.del(kp), mr.del(kp), "{tag} op={op} del");
                }
            }
            assert_eq!(mc.snap(kind), mr.snap(kind), "{tag} op={op} state");
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn cfg_32_fuzz_string_strdup_pipeline() {
    fuzz_string("strdup", STBDS_SH_STRDUP, 0x5D5D_5D5D, 0x3141_5926, 4000, 400);
    fuzz_string("strdup-s0", STBDS_SH_STRDUP, 0x1A2B_3C4D, 0, 2000, 90);
}

#[test]
fn cfg_33_fuzz_string_arena_pipeline() {
    fuzz_string("arena", STBDS_SH_ARENA, 0xA4A4_A4A4, 0x3141_5926, 4000, 400);
    fuzz_string("arena-smax", STBDS_SH_ARENA, 0x9F9F_9F9F, usize::MAX, 2000, 90);
}

#[test]
fn cfg_34_fuzz_string_default_pipeline() {
    fuzz_string("default", STBDS_SH_DEFAULT, 0xD3D3_D3D3, 0x3141_5926, 4000, 400);
    fuzz_string("default-s7", STBDS_SH_DEFAULT, 0x7E7E_7E7E, 7, 2000, 90);
}

/// Same fuzz driver but with `mode` values that are *not* valid enum members
/// (`mode = 2` and `mode = 5` still select the "string" branch everywhere
/// except `hmdel_key`'s `mode == STBDS_HM_STRING` strdup-free check).
#[test]
fn cfg_32b_fuzz_string_out_of_range_mode() {
    let (_g, p) = libs();
    for &mode in &[2i32, 5, 1000] {
        for &sh in &[STBDS_SH_DEFAULT, STBDS_SH_ARENA] {
            let mut rng = Rng::new(0xB00B_0000 + mode as u64);
            seed(p, 0x3141_5926);
            let keys = distinct_strings(&mut rng, 120);
            let kind = str_kind();
            unsafe {
                let mut mc = Map::shmode(&p.c, STR_ES, STR_KS, mode, sh);
                let mut mr = Map::shmode(&p.r, STR_ES, STR_KS, mode, sh);
                mc.put_default();
                mr.put_default();
                mc.set_i32(-1, STR_VOFF, -2);
                mr.set_i32(-1, STR_VOFF, -2);
                for op in 0..1500 {
                    let idx = rng.below(keys.len());
                    let kp = keys[idx].as_ptr() as *mut c_void;
                    // NB: no deletes here.  For `mode != STBDS_HM_STRING`
                    // exactly, `stbds_hmdel_key` takes its *binary* re-find
                    // branch (`mode == STBDS_HM_STRING` is `==`, not `>=`) and
                    // hashes the address of the key pointer instead of the
                    // string, so `STBDS_ASSERT(slot >= 0)` fires.  That abort is
                    // a genuine C behaviour and is covered in Phase C
                    // (`err_29_hmdel_mode_eq_vs_ge`).
                    match rng.below(8) {
                        0..=3 => {
                            let v = rng.next_u32() as i32;
                            let tc = mc.put_key(kp);
                            let tr = mr.put_key(kp);
                            assert_eq!(tc, tr, "mode={mode} sh={sh} op={op} put");
                            mc.set_i32(tc, STR_VOFF, v);
                            mr.set_i32(tr, STR_VOFF, v);
                        }
                        4..=5 => assert_eq!(mc.geti(kp), mr.geti(kp), "mode={mode} op={op}"),
                        _ => assert_eq!(mc.geti_ts(kp), mr.geti_ts(kp), "mode={mode} op={op} ts"),
                    }
                    assert_eq!(mc.snap(kind), mr.snap(kind), "mode={mode} sh={sh} op={op}");
                }
                mc.free();
                mr.free();
            }
        }
    }
}
