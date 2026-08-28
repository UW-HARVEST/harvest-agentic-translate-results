//! Phase B rows B35-B47, B59-B61, B63, B64: string-mode (`mode >= 1`) hash maps
//! across all four `string.mode` arena modes, driven through the low-level
//! `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmdel_key` / `stbds_shmode_func`
//! entry points.

mod common;
use common::*;
use std::os::raw::{c_char, c_int};

/// `struct { char *key; size_t value; }` — 16 bytes, no padding, so every byte
/// of every element is written by either the library or the macro emulation.
const ES: usize = 16;
const VALUE_OFF: usize = 8;

/// Owns the NUL-terminated key buffers.  For `STBDS_SH_DEFAULT` the library
/// stores these very pointers, so they must outlive the map.
struct Keys(Vec<Vec<u8>>);

impl Keys {
    fn new() -> Keys {
        Keys(Vec::new())
    }
    fn add(&mut self, s: &[u8]) -> usize {
        let mut v = s.to_vec();
        v.push(0);
        // 8 bytes of slack so the SH_NONE `memcpy(key, keysize=8)` stays in bounds
        v.extend_from_slice(&[0u8; 8]);
        self.0.push(v);
        self.0.len() - 1
    }
    fn ptr(&mut self, i: usize) -> *mut c_char {
        self.0[i].as_mut_ptr() as *mut c_char
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

fn ascii_keys(rng: &mut Rng, n: usize, minlen: usize, maxlen: usize) -> Keys {
    let mut ks = Keys::new();
    let mut seen = std::collections::HashSet::new();
    while ks.len() < n {
        let l = rng.range(minlen, maxlen);
        let s = rng.ascii(l);
        if seen.insert(s.clone()) {
            ks.add(&s);
        }
    }
    ks
}

fn val(v: u64) -> [u8; 8] {
    v.to_ne_bytes()
}

/// B35 — `hmput_key(mode=1)` on a NULL map → implicit `string.mode = SH_DEFAULT`
#[test]
fn cfg_b35_implicit_sh_default() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut rng = Rng::new(35);
        let mut keys = ascii_keys(&mut rng, 40, 1, 20);
        let mut p = Pair::new(c, r, Shape::string(ES));
        for i in 0..keys.len() {
            let kp = keys.ptr(i);
            p.shput_value(kp, VALUE_OFF, &val(i as u64), HM_STRING, &format!("B35 put {i}"));
            let t = p.c.snapshot().table.unwrap();
            assert_eq!(t.arena_mode, 1, "B35 implicit SH_DEFAULT");
        }
        for i in 0..keys.len() {
            let kp = keys.ptr(i);
            let got = p.sgeti(kp, HM_STRING, &format!("B35 get {i}"));
            assert!(got >= 0, "B35 key {i} present");
        }
        p.free("B35 free");
    });
}

fn string_mode_suite(name: &str, sh_mode: c_int, nkeys: usize, minlen: usize, maxlen: usize) {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut rng = Rng::new(0x5000 + sh_mode as u64 * 31 + nkeys as u64);
        let mut keys = ascii_keys(&mut rng, nkeys, minlen, maxlen);
        let mut p = Pair::new(c, r, Shape::string(ES));
        p.shmode(sh_mode, &format!("{name} shmode"));
        let t = p.c.snapshot().table.unwrap();
        assert_eq!(t.arena_mode as c_int, sh_mode, "{name} arena mode");
        assert_eq!(t.slot_count, 8, "{name} fresh 8 slots");
        assert_eq!(p.c.hmlen(), 0, "{name} hmlen 0");

        for i in 0..keys.len() {
            let kp = keys.ptr(i);
            p.shput_value(
                kp,
                VALUE_OFF,
                &val(i as u64 * 7 + 1),
                HM_STRING,
                &format!("{name} put {i}"),
            );
        }
        // gets (present)
        for i in 0..keys.len() {
            let kp = keys.ptr(i);
            let got = p.sgeti(kp, HM_STRING, &format!("{name} get {i}"));
            assert!(got >= 0, "{name} key {i} must be present");
        }
        // gets (absent)
        let mut absent = ascii_keys(&mut rng, 10, 30, 40);
        for i in 0..absent.len() {
            let kp = absent.ptr(i);
            let got = p.sgeti(kp, HM_STRING, &format!("{name} get-absent {i}"));
            assert_eq!(got, -1, "{name} absent key {i}");
        }
        // duplicate puts (existing-key path, sets temp_key)
        for i in 0..keys.len() {
            let kp = keys.ptr(i);
            p.shput_value(
                kp,
                VALUE_OFF,
                &val(i as u64 * 13),
                HM_STRING,
                &format!("{name} reput {i}"),
            );
        }
        assert_eq!(p.c.hmlen(), nkeys as isize, "{name} no duplicates");
        // deletes
        for i in (0..keys.len()).step_by(2) {
            let kp = keys.ptr(i);
            let d = p.sdel(kp, HM_STRING, &format!("{name} del {i}"));
            assert_eq!(d, 1, "{name} del {i} reports 1");
        }
        for i in (0..keys.len()).step_by(2) {
            let kp = keys.ptr(i);
            let d = p.sdel(kp, HM_STRING, &format!("{name} redel {i}"));
            assert_eq!(d, 0, "{name} redel {i} reports 0");
        }
        p.free(&format!("{name} free"));
    });
}

/// B36 — `SH_STRDUP`
#[test]
fn cfg_b36_sh_strdup() {
    for &n in &[1usize, 6, 12, 100] {
        string_mode_suite(&format!("B36 STRDUP n={n}"), SH_STRDUP, n, 1, 24);
    }
}

/// B37 — `SH_ARENA` (short keys and keys crossing the 512-byte block boundary)
#[test]
fn cfg_b37_sh_arena() {
    for &n in &[1usize, 6, 12, 100] {
        string_mode_suite(&format!("B37 ARENA n={n}"), SH_ARENA, n, 1, 24);
    }
    // long keys → dedicated arena blocks
    string_mode_suite("B37 ARENA long", SH_ARENA, 20, 400, 900);
    string_mode_suite("B37 ARENA mixed", SH_ARENA, 60, 1, 700);
}

/// B39 — explicit `SH_DEFAULT`
#[test]
fn cfg_b39_sh_default() {
    for &n in &[1usize, 6, 12, 100] {
        string_mode_suite(&format!("B39 DEFAULT n={n}"), SH_DEFAULT, n, 1, 24);
    }
}

/// B38 — `SH_NONE` (0) with a *string* mode put: `switch` falls to `default:`
/// and `memcpy`s `keysize` bytes of the string into the key field instead of
/// storing a `char *`.  Only distinct fresh inserts are safe: any hash match
/// would make `is_key_equal` dereference those bytes as a pointer.
#[test]
fn cfg_b38_sh_none_string_mode() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut rng = Rng::new(38);
        // >= 8 chars so the 8-byte memcpy is fully inside the string body
        let mut keys = ascii_keys(&mut rng, 20, 9, 20);
        let mut p = Pair::new(c, r, Shape::string(ES).raw_keys());
        p.shmode(SH_NONE, "B38 shmode");
        assert_eq!(p.c.snapshot().table.unwrap().arena_mode, 0);
        for i in 0..keys.len() {
            let kp = keys.ptr(i);
            p.shput_value(
                kp,
                VALUE_OFF,
                &val(i as u64),
                HM_STRING,
                &format!("B38 put {i}"),
            );
        }
        assert_eq!(p.c.hmlen(), 20, "B38 all inserted");
        p.free("B38 free");
    });
}

/// B40 — present/absent gets already covered per-mode; here across all four
/// modes on the *same* key set with `hmget_key_ts` too.
#[test]
fn cfg_b40_get_all_modes() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(40 + sh as u64);
            let mut keys = ascii_keys(&mut rng, 30, 1, 16);
            let mut absent = ascii_keys(&mut rng, 10, 20, 30);
            let mut p = Pair::new(c, r, Shape::string(ES));
            p.shmode(sh, &format!("B40 sh={sh} shmode"));
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                p.shput_value(kp, VALUE_OFF, &val(i as u64), HM_STRING, &format!("B40 put {i}"));
            }
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                let a = p.sgeti(kp, HM_STRING, &format!("B40 sh={sh} get {i}"));
                let mut kc = keys.0[i].clone();
                let b = p.geti_ts(&kc, HM_STRING, &format!("B40 sh={sh} get_ts {i}"));
                kc.clear();
                assert_eq!(a, b, "B40 sh={sh} get vs get_ts {i}");
            }
            for i in 0..absent.len() {
                let kp = absent.ptr(i);
                assert_eq!(
                    p.sgeti(kp, HM_STRING, &format!("B40 sh={sh} absent {i}")),
                    -1
                );
            }
            p.free(&format!("B40 sh={sh} free"));
        });
    }
}

/// B41 — deletes for every `string.mode`, including the `SH_STRDUP` key free
#[test]
fn cfg_b41_delete_all_modes() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(41 + sh as u64 * 977);
            let mut keys = ascii_keys(&mut rng, 60, 1, 20);
            let mut p = Pair::new(c, r, Shape::string(ES));
            p.shmode(sh, &format!("B41 sh={sh} shmode"));
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                p.shput_value(kp, VALUE_OFF, &val(i as u64), HM_STRING, &format!("B41 put {i}"));
            }
            // delete every key in a shuffled order → memmove + re-find,
            // tombstone rebuilds and shrinks
            let mut order: Vec<usize> = (0..keys.len()).collect();
            for i in (1..order.len()).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            for (n, &i) in order.iter().enumerate() {
                let kp = keys.ptr(i);
                let d = p.sdel(kp, HM_STRING, &format!("B41 sh={sh} del#{n} key{i}"));
                assert_eq!(d, 1, "B41 sh={sh} del key {i}");
            }
            assert_eq!(p.c.hmlen(), 0);
            p.free(&format!("B41 sh={sh} free"));
        });
    }
}

/// B42 — `hmfree_func` for every `string.mode`
#[test]
fn cfg_b42_hmfree_all_modes() {
    for &sh in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &n in &[0usize, 1, 7, 40] {
            with_libs(DEFAULT_SEED, |c, r| unsafe {
                let mut rng = Rng::new(42 + sh as u64 * 13 + n as u64);
                let mut keys = ascii_keys(&mut rng, n.max(1), 9, 24);
                let shape = if sh == SH_NONE {
                    Shape::string(ES).raw_keys()
                } else {
                    Shape::string(ES)
                };
                let mut p = Pair::new(c, r, shape);
                p.shmode(sh, &format!("B42 sh={sh} n={n} shmode"));
                for i in 0..n {
                    let kp = keys.ptr(i);
                    p.shput_value(
                        kp,
                        VALUE_OFF,
                        &val(i as u64),
                        HM_STRING,
                        &format!("B42 sh={sh} n={n} put {i}"),
                    );
                }
                p.assert_same(&format!("B42 sh={sh} n={n} before free"));
                p.free(&format!("B42 sh={sh} n={n} free"));
            });
        }
    }
}

/// B43 — duplicate string key: existing-key path writes `temp_key`
#[test]
fn cfg_b43_temp_key_on_duplicate() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(43 + sh as u64);
            let mut keys = ascii_keys(&mut rng, 12, 3, 16);
            let mut p = Pair::new(c, r, Shape::string(ES));
            p.shmode(sh, "B43 shmode");
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                p.shput_value(kp, VALUE_OFF, &val(i as u64), HM_STRING, &format!("B43 put {i}"));
                // temp_key must equal the key just inserted (new-element path)
                let expect: Vec<u8> = keys.0[i][..keys.0[i].len() - 9].to_vec();
                p.assert_temp_key(&expect, &format!("B43 sh={sh} temp_key after put {i}"));
            }
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                p.shput_value(
                    kp,
                    VALUE_OFF,
                    &val(1000 + i as u64),
                    HM_STRING,
                    &format!("B43 reput {i}"),
                );
                // Existing-element path.  lib.c:733 updates temp_key only in the
                // *forward* half of the probe loop; lib.c:749-750 (the wrap half)
                // returns without touching it, so temp_key may legitimately keep
                // an older key here.  Both libraries must nevertheless agree.
                p.assert_temp_key_same(&format!("B43 sh={sh} temp_key after reput {i}"));
            }
            p.free("B43 free");
        });
    }
}

/// B43b — `stbds_shputs` on *fresh* inserts only: the new-element path
/// (lib.c:786-788) always writes `temp_key`, so `.key = stbds_temp_key(t-1)`
/// stores exactly the key the library just recorded, for every arena mode.
#[test]
fn cfg_b43b_shputs_fresh_inserts() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(0x43B + sh as u64);
            let mut keys = ascii_keys(&mut rng, 60, 1, 40);
            let mut p = Pair::new(c, r, Shape::string(ES));
            p.shmode(sh, "B43b shmode");
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                let mut e = vec![0u8; ES];
                e[VALUE_OFF..].copy_from_slice(&val(0xAABB + i as u64));
                p.shputs(kp, &e, HM_STRING, &format!("B43b sh={sh} shputs {i}"));
                let expect: Vec<u8> = keys.0[i][..keys.0[i].len() - 9].to_vec();
                p.assert_temp_key(&expect, &format!("B43b sh={sh} temp_key {i}"));
            }
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                assert!(p.sgeti(kp, HM_STRING, &format!("B43b sh={sh} get {i}")) >= 0);
            }
            p.free(&format!("B43b sh={sh} free"));
        });
    }
}

/// B44 / B46 — `mode > STBDS_HM_STRING`.  Put/get take the string path; the
/// `mode == STBDS_HM_STRING` sub-tests inside `hmdel_key` are **false**, so the
/// delete only stays well defined when `old_index == final_index` (deleting the
/// newest element).  The out-of-range-delete case is covered in Phase C (E16).
#[test]
fn cfg_b44_b46_high_modes() {
    for &mode in &[2 as c_int, 3, 7, 1000, c_int::MAX] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(44 + mode as u64 as u64);
            let mut keys = ascii_keys(&mut rng, 30, 1, 20);
            let mut p = Pair::new(c, r, Shape::string(ES));
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                p.shput_value(kp, VALUE_OFF, &val(i as u64), mode, &format!("B44 m={mode} put {i}"));
            }
            let t = p.c.snapshot().table.unwrap();
            assert_eq!(t.arena_mode, 1, "B44 m={mode}: mode>=1 → SH_DEFAULT");
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                assert!(p.sgeti(kp, mode, &format!("B44 m={mode} get {i}")) >= 0);
            }
            // mode 1 lookups must see the same table
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                let a = p.sgeti(kp, mode, &format!("B44 m={mode} get2 {i}"));
                let b = p.sgeti(kp, HM_STRING, &format!("B44 m={mode} get1 {i}"));
                assert_eq!(a, b, "B44 m={mode} mode>1 == mode1 for get");
            }
            // deleting the *newest* element: old_index == final_index, so the
            // `mode == 1` re-find branch is skipped and mode>1 is well defined.
            for i in (0..keys.len()).rev() {
                let kp = keys.ptr(i);
                let d = p.sdel(kp, mode, &format!("B44 m={mode} del-newest {i}"));
                assert_eq!(d, 1, "B44 m={mode} del {i}");
            }
            assert_eq!(p.c.hmlen(), 0);
            p.free(&format!("B44 m={mode} free"));
        });
    }
}

/// B47 — string key shapes: empty, 1 char, last-byte-only differences, long,
/// bytes >= 0x80
#[test]
fn cfg_b47_key_shapes() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(47 + sh as u64);
            let mut keys = Keys::new();
            keys.add(b"");
            keys.add(b"a");
            keys.add(b"b");
            for suffix in b'a'..=b'z' {
                let mut s = b"prefix_that_is_long_enough".to_vec();
                s.push(suffix);
                keys.add(&s);
            }
            keys.add(&vec![b'x'; 200]);
            keys.add(&vec![b'y'; 511]);
            keys.add(&vec![b'z'; 512]);
            keys.add(&vec![b'w'; 513]);
            keys.add(&vec![b'v'; 4096]);
            for n in 1usize..=32 {
                keys.add(&rng.cstr_bytes(n));
            }
            let mut p = Pair::new(c, r, Shape::string(ES));
            p.shmode(sh, &format!("B47 sh={sh} shmode"));
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                p.shput_value(kp, VALUE_OFF, &val(i as u64), HM_STRING, &format!("B47 sh={sh} put {i}"));
            }
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                assert!(
                    p.sgeti(kp, HM_STRING, &format!("B47 sh={sh} get {i}")) >= 0,
                    "B47 sh={sh} key {i} present"
                );
            }
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                p.sdel(kp, HM_STRING, &format!("B47 sh={sh} del {i}"));
            }
            p.free(&format!("B47 sh={sh} free"));
        });
    }
}

/// B59/B60/B61 — composed pipelines per arena mode with deep compare each step
#[test]
fn cfg_b59_b61_pipelines() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for seed in 1u64..=3 {
            with_libs(DEFAULT_SEED, |c, r| unsafe {
                let mut rng = Rng::new(sh as u64 * 1009 + seed);
                let long = sh == SH_ARENA;
                let mut keys = if long {
                    ascii_keys(&mut rng, 80, 1, 700)
                } else {
                    ascii_keys(&mut rng, 80, 1, 30)
                };
                let mut p = Pair::new(c, r, Shape::string(ES));
                p.shmode(sh, &format!("PIPE sh={sh} s={seed} shmode"));
                for i in 0..60 {
                    let kp = keys.ptr(i);
                    p.shput_value(kp, VALUE_OFF, &val(i as u64), HM_STRING, &format!("PIPE put {i}"));
                }
                for i in (0..60).step_by(3) {
                    let kp = keys.ptr(i);
                    p.sdel(kp, HM_STRING, &format!("PIPE del {i}"));
                }
                for i in 60..80 {
                    let kp = keys.ptr(i);
                    p.shput_value(kp, VALUE_OFF, &val(i as u64), HM_STRING, &format!("PIPE put2 {i}"));
                }
                for i in 0..80 {
                    let kp = keys.ptr(i);
                    p.sgeti(kp, HM_STRING, &format!("PIPE get {i}"));
                }
                p.put_default("PIPE default");
                p.free(&format!("PIPE sh={sh} s={seed} free"));
            });
        }
    }
}

/// B59b — fully randomized string-map op stream
#[test]
fn cfg_b59b_random_string_ops() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for seed in 1u64..=2 {
            with_libs(DEFAULT_SEED, |c, r| unsafe {
                let mut rng = Rng::new(0xB59B + sh as u64 * 71 + seed);
                let mut keys = ascii_keys(&mut rng, 50, 1, 40);
                let mut p = Pair::new(c, r, Shape::string(ES));
                p.shmode(sh, "B59b shmode");
                for i in 0..1200 {
                    let idx = rng.below(keys.len());
                    let kp = keys.ptr(idx);
                    match rng.below(10) {
                        0..=4 => {
                            p.shput_value(
                                kp,
                                VALUE_OFF,
                                &val(rng.next_u64()),
                                HM_STRING,
                                &format!("B59b sh={sh} s={seed} i={i} put"),
                            );
                        }
                        5..=6 => {
                            p.sgeti(kp, HM_STRING, &format!("B59b sh={sh} s={seed} i={i} get"));
                        }
                        7 => {
                            // `stbds_shputs` writes `.key = stbds_temp_key(t-1)`.
                            // lib.c:746-759 (the *wrap* half of the probe loop)
                            // returns from the existing-key path WITHOUT updating
                            // temp_key, so `shputs` can store a *stale* pointer.
                            // For SH_STRDUP / SH_ARENA that stale pointer is
                            // library-private (and may already be freed), so the
                            // observable result legitimately depends on pointer
                            // identity and is not comparable across the two
                            // libraries.  Under SH_DEFAULT every key pointer is a
                            // caller pointer, identical in both, so shputs stays
                            // deterministic there.
                            if sh == SH_DEFAULT {
                                let mut e = vec![0u8; ES];
                                e[VALUE_OFF..].copy_from_slice(&val(rng.next_u64()));
                                p.shputs(kp, &e, HM_STRING, &format!("B59b i={i} shputs"));
                            } else {
                                p.sgeti(kp, HM_STRING, &format!("B59b i={i} get(alt)"));
                            }
                        }
                        8 => {
                            p.sdel(kp, HM_STRING, &format!("B59b sh={sh} s={seed} i={i} del"));
                        }
                        _ => {
                            p.put_default(&format!("B59b i={i} default"));
                        }
                    }
                }
                p.free(&format!("B59b sh={sh} s={seed} free"));
            });
        }
    }
}

/// B63 — `hmput_default` on string maps: element `-1` is zeroed (key ptr NULL),
/// and `hmfree_func`'s STRDUP loop starts at `i = 1` so it is not freed.
#[test]
fn cfg_b63_default_element_on_string_maps() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(63 + sh as u64);
            let mut keys = ascii_keys(&mut rng, 12, 1, 20);
            let mut p = Pair::new(c, r, Shape::string(ES));
            p.shmode(sh, "B63 shmode");
            p.put_default("B63 default before");
            for i in 0..keys.len() {
                let kp = keys.ptr(i);
                p.shput_value(kp, VALUE_OFF, &val(i as u64), HM_STRING, &format!("B63 put {i}"));
            }
            p.put_default("B63 default after");
            // key pointer of the default element must be NULL in both
            let s = p.c.snapshot();
            assert_eq!(s.keys[0], None, "B63 default key is NULL");
            p.free("B63 free");
        });
    }
}

/// B64 — `stbds_shmode_func` over elemsize × mode
#[test]
fn cfg_b64_shmode_matrix() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[8usize, 12, 16, 24, 64] {
            for mode in 0..=3 {
                let shape = if mode == 0 {
                    Shape {
                        elemsize: es,
                        keysize: 8,
                        keyoffset: 0,
                        string_key: false,
                        cmp_temp_key: false,
                    }
                } else {
                    Shape {
                        elemsize: es,
                        keysize: 8,
                        keyoffset: 0,
                        string_key: true,
                        cmp_temp_key: false,
                    }
                };
                let mut p = Pair::new(c, r, shape);
                p.shmode(mode, &format!("B64 es={es} mode={mode}"));
                let t = p.c.snapshot().table.unwrap();
                assert_eq!(t.arena_mode as c_int, mode);
                assert_eq!(t.slot_count, 8);
                assert_eq!(t.used_count, 0);
                assert_eq!(p.c.hmlen(), 0);
                p.free(&format!("B64 es={es} mode={mode} free"));
            }
        }
    });
}
