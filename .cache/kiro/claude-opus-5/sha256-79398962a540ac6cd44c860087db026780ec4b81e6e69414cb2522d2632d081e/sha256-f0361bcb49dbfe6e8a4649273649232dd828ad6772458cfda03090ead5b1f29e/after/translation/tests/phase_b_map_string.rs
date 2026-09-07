//! Phase B rows 33–45 — hash map with string keys, across every `string.mode`.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

const ES: usize = 16; // { char *key; int value; } + padding, like sh_puts
const KS: usize = 8;

fn cmpk(c_map: *mut c_void, r_map: *mut c_void, es: usize, tk: bool, ctx: &str) {
    unsafe {
        let sc = snap_map(c_map, es, ElemFmt::KeyPtr, tk);
        let sr = snap_map(r_map, es, ElemFmt::KeyPtr, tk);
        assert_snap_eq(&sc, &sr, ctx);
    }
}

fn cmpraw(c_map: *mut c_void, r_map: *mut c_void, es: usize, ctx: &str) {
    unsafe {
        let sc = snap_map(c_map, es, ElemFmt::Raw, false);
        let sr = snap_map(r_map, es, ElemFmt::Raw, false);
        assert_snap_eq(&sc, &sr, ctx);
    }
}

/// Build `n` distinct NUL-terminated keys. Inner buffers are heap-stable, so
/// raw pointers taken from them remain valid for the whole test.
fn make_keys(rng: &mut Rng, n: usize, minlen: usize, maxlen: usize) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut i = 0usize;
    while out.len() < n {
        let len = minlen + rng.below(maxlen - minlen + 1);
        let mut v = rng.ascii(len);
        // guarantee uniqueness
        let tag = format!("#{}", i);
        v.pop();
        v.extend_from_slice(tag.as_bytes());
        v.push(0);
        if !out.contains(&v) {
            out.push(v);
        }
        i += 1;
    }
    out
}

fn kp(v: &[u8]) -> *mut c_char {
    v.as_ptr() as *mut c_char
}

/// CONFIGS row 33 — `hmput_key(mode=STRING)` bootstrapped from NULL:
/// `string.mode` is auto-set to `SH_DEFAULT` and the key pointer is stored verbatim.
#[test]
fn cfg_33_string_bootstrap_from_null() {
    run(0x3141_5926, |c, r| unsafe {
        let mut rng = Rng::new(0x5eed_f000);
        let keys = make_keys(&mut rng, 30, 1, 20);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        for (i, k) in keys.iter().enumerate() {
            mc = sh_put(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
            mr = sh_put(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
            cmpk(mc, mr, ES, true, &format!("row33 insert #{}", i));
            let t = (*header_of(mc, ES)).hash_table as *mut HashIndex;
            assert_eq!((*t).string.mode, SH_DEFAULT as u8, "auto string.mode");
            // SH_DEFAULT stores the caller's pointer verbatim
            let stored = *((mc as *mut u8).offset((*header_of(mc, ES)).temp * ES as isize)
                as *mut *mut c_char);
            assert_eq!(stored, kp(k), "SH_DEFAULT must store the caller pointer");
        }
        for (i, k) in keys.iter().enumerate() {
            let (m1, tc) = sh_get(c, mc, ES, kp(k), HM_STRING);
            let (m2, tr) = sh_get(r, mr, ES, kp(k), HM_STRING);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "row33 get #{}", i);
            assert!(tc >= 0);
        }
        // misses
        let misses = make_keys(&mut rng, 20, 1, 20);
        for k in &misses {
            let mut probe = k.clone();
            probe.pop();
            probe.extend_from_slice(b"ZZmiss\0");
            let (m1, tc) = sh_get(c, mc, ES, kp(&probe), HM_STRING);
            let (m2, tr) = sh_get(r, mr, ES, kp(&probe), HM_STRING);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr);
            assert_eq!(tc, -1);
        }
        cmpk(mc, mr, ES, true, "row33 final");
        hm_free(c, mc, ES);
        hm_free(r, mr, ES);
    });
}

/// CONFIGS row 34 — `shmode_func(SH_DEFAULT)`, 1..40 keys, hit + miss, temp_key.
#[test]
fn cfg_34_sh_default_mode() {
    for n in [1usize, 5, 6, 7, 8, 13, 25, 40] {
        run(0x3141_5926, |c, r| unsafe {
            let mut rng = Rng::new(0x5eed_1_0000 + n as u64);
            let keys = make_keys(&mut rng, n, 1, 24);
            let mut mc = (c.shmode_func)(ES, SH_DEFAULT);
            let mut mr = (r.shmode_func)(ES, SH_DEFAULT);
            cmpk(mc, mr, ES, false, "row34 fresh");
            for (i, k) in keys.iter().enumerate() {
                mc = sh_put(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                mr = sh_put(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                cmpk(mc, mr, ES, true, &format!("row34 n={} insert #{}", n, i));
            }
            for k in &keys {
                let (m1, tc) = sh_get(c, mc, ES, kp(k), HM_STRING);
                let (m2, tr) = sh_get(r, mr, ES, kp(k), HM_STRING);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr);
                assert!(tc >= 0);
            }
            hm_free(c, mc, ES);
            hm_free(r, mr, ES);
        });
    }
}

/// CONFIGS row 35 — `SH_STRDUP`: stored pointer differs from the input, content equal.
#[test]
fn cfg_35_sh_strdup_mode() {
    for n in [1usize, 6, 7, 8, 20, 45] {
        run(0x3141_5926, |c, r| unsafe {
            let mut rng = Rng::new(0x5eed_2_0000 + n as u64);
            let keys = make_keys(&mut rng, n, 1, 30);
            let mut mc = (c.shmode_func)(ES, SH_STRDUP);
            let mut mr = (r.shmode_func)(ES, SH_STRDUP);
            for (i, k) in keys.iter().enumerate() {
                mc = sh_put(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                mr = sh_put(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                cmpk(mc, mr, ES, true, &format!("row35 n={} insert #{}", n, i));
                let t = (*header_of(mc, ES)).temp;
                let sc = *((mc as *mut u8).offset(t * ES as isize) as *mut *mut c_char);
                let sr = *((mr as *mut u8).offset(t * ES as isize) as *mut *mut c_char);
                assert_ne!(sc, kp(k), "C: strdup'd key must not alias the input");
                assert_ne!(sr, kp(k), "Rust: strdup'd key must not alias the input");
            }
            for k in &keys {
                let (m1, tc) = sh_get(c, mc, ES, kp(k), HM_STRING);
                let (m2, tr) = sh_get(r, mr, ES, kp(k), HM_STRING);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr);
                assert!(tc >= 0);
            }
            cmpk(mc, mr, ES, true, "row35 final");
            hm_free(c, mc, ES);
            hm_free(r, mr, ES);
        });
    }
}

/// CONFIGS row 36 — `SH_ARENA`: keys arena-allocated, crossing the 512-byte block.
#[test]
fn cfg_36_sh_arena_mode() {
    for n in [1usize, 8, 20, 40, 64] {
        run(0x3141_5926, |c, r| unsafe {
            let mut rng = Rng::new(0x5eed_3_0000 + n as u64);
            // long keys so 64 of them overflow several arena blocks
            let keys = make_keys(&mut rng, n, 20, 40);
            let mut mc = (c.shmode_func)(ES, SH_ARENA);
            let mut mr = (r.shmode_func)(ES, SH_ARENA);
            for (i, k) in keys.iter().enumerate() {
                mc = sh_put(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                mr = sh_put(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                cmpk(mc, mr, ES, true, &format!("row36 n={} insert #{}", n, i));
                let t = (*header_of(mc, ES)).temp;
                let sc = *((mc as *mut u8).offset(t * ES as isize) as *mut *mut c_char);
                assert_ne!(sc, kp(k), "arena key must not alias the input");
            }
            for k in &keys {
                let (m1, tc) = sh_get(c, mc, ES, kp(k), HM_STRING);
                let (m2, tr) = sh_get(r, mr, ES, kp(k), HM_STRING);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr);
                assert!(tc >= 0);
            }
            cmpk(mc, mr, ES, true, "row36 final");
            hm_free(c, mc, ES);
            hm_free(r, mr, ES);
        });
    }
}

/// CONFIGS row 37 — `SH_NONE` with `mode = STBDS_HM_STRING`.
///
/// The `switch (table->string.mode)` `default:` arm runs, so the *first
/// `keysize` bytes of the string* are `memcpy`'d into the element instead of a
/// pointer. Insertion only: a subsequent lookup would reinterpret those bytes
/// as a `char *` (UB in the C too), so it is deliberately not exercised.
#[test]
fn cfg_37_sh_none_memcpy_branch() {
    run(0x3141_5926, |c, r| unsafe {
        let mut rng = Rng::new(0x5eed_4_0000);
        let keys = make_keys(&mut rng, 6, 12, 20);
        let mut mc = (c.shmode_func)(ES, SH_NONE);
        let mut mr = (r.shmode_func)(ES, SH_NONE);
        let t = (*header_of(mc, ES)).hash_table as *mut HashIndex;
        assert_eq!((*t).string.mode, 0);
        for (i, k) in keys.iter().enumerate() {
            mc = sh_put_v(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
            mr = sh_put_v(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
            cmpraw(mc, mr, ES, &format!("row37 insert #{}", i));
            // the element must literally contain the first 8 bytes of the key
            let tp = (*header_of(mc, ES)).temp;
            let e = std::slice::from_raw_parts((mc as *const u8).offset(tp * ES as isize), KS);
            assert_eq!(e, &k[..KS], "default: branch must memcpy the key bytes");
        }
        // free without touching the bogus key field: string.mode != SH_STRDUP,
        // so hmfree_func does not dereference it.
        hm_free(c, mc, ES);
        hm_free(r, mr, ES);
    });
}

/// CONFIGS row 38 — re-put an existing string key (found branch sets `temp_key`).
#[test]
fn cfg_38_string_update_existing() {
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        run(0x3141_5926, |c, r| unsafe {
            let mut rng = Rng::new(0x5eed_5_0000 + mode as u64);
            let keys = make_keys(&mut rng, 30, 1, 24);
            let mut mc = (c.shmode_func)(ES, mode);
            let mut mr = (r.shmode_func)(ES, mode);
            for (i, k) in keys.iter().enumerate() {
                mc = sh_put(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                mr = sh_put(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
            }
            let len_before = hm_len(mc, ES);
            for round in 0..3u64 {
                for (i, k) in keys.iter().enumerate() {
                    let pay = ((i as u64) ^ (round << 32)).to_le_bytes();
                    mc = sh_put_v(c, mc, ES, kp(k), &pay, HM_STRING);
                    mr = sh_put_v(r, mr, ES, kp(k), &pay, HM_STRING);
                    cmpk(
                        mc,
                        mr,
                        ES,
                        true,
                        &format!("row38 mode={} round={} update #{}", mode, round, i),
                    );
                }
            }
            assert_eq!(hm_len(mc, ES), len_before, "updates must not grow the map");
            assert_eq!(hm_len(mr, ES), len_before);
            hm_free(c, mc, ES);
            hm_free(r, mr, ES);
        });
    }
}

/// CONFIGS row 39 — keys with long shared prefixes (strcmp discrimination).
#[test]
fn cfg_39_string_shared_prefixes() {
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        run(0x3141_5926, |c, r| unsafe {
            let base = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for b in 1u8..=40 {
                let mut v = base.to_vec();
                v.push(b'a' + (b % 26));
                v.push(b'0' + (b / 26));
                v.push(0);
                keys.push(v);
            }
            // also differ only in the final byte
            for b in 1u8..=20 {
                let mut v = base.to_vec();
                *v.last_mut().unwrap() = b'A' + b;
                v.push(0);
                keys.push(v);
            }
            let mut mc = (c.shmode_func)(ES, mode);
            let mut mr = (r.shmode_func)(ES, mode);
            for (i, k) in keys.iter().enumerate() {
                mc = sh_put(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                mr = sh_put(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                cmpk(mc, mr, ES, true, &format!("row39 mode={} insert #{}", mode, i));
            }
            for (i, k) in keys.iter().enumerate() {
                let (m1, tc) = sh_get(c, mc, ES, kp(k), HM_STRING);
                let (m2, tr) = sh_get(r, mr, ES, kp(k), HM_STRING);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr, "row39 get #{}", i);
                assert!(tc >= 0);
            }
            hm_free(c, mc, ES);
            hm_free(r, mr, ES);
        });
    }
}

/// CONFIGS row 40 — the empty-string key.
#[test]
fn cfg_40_string_empty_key() {
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        run(0x3141_5926, |c, r| unsafe {
            let empty: Vec<u8> = vec![0];
            let other: Vec<u8> = b"x\0".to_vec();
            let mut mc = (c.shmode_func)(ES, mode);
            let mut mr = (r.shmode_func)(ES, mode);
            mc = sh_put(c, mc, ES, kp(&empty), &1u64.to_le_bytes(), HM_STRING);
            mr = sh_put(r, mr, ES, kp(&empty), &1u64.to_le_bytes(), HM_STRING);
            cmpk(mc, mr, ES, true, &format!("row40 mode={} empty key", mode));
            mc = sh_put(c, mc, ES, kp(&other), &2u64.to_le_bytes(), HM_STRING);
            mr = sh_put(r, mr, ES, kp(&other), &2u64.to_le_bytes(), HM_STRING);
            cmpk(mc, mr, ES, true, &format!("row40 mode={} + other", mode));

            let (m1, tc) = sh_get(c, mc, ES, kp(&empty), HM_STRING);
            let (m2, tr) = sh_get(r, mr, ES, kp(&empty), HM_STRING);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr);
            assert!(tc >= 0, "empty key must be findable");
            hm_free(c, mc, ES);
            hm_free(r, mr, ES);
        });
    }
}

/// CONFIGS row 41 — delete on `SH_STRDUP` (frees the dup) incl. swap-with-last re-find.
#[test]
fn cfg_41_strdup_delete() {
    run(0x3141_5926, |c, r| unsafe {
        let mut rng = Rng::new(0x5eed_6_0000);
        let keys = make_keys(&mut rng, 40, 4, 24);
        let mut mc = (c.shmode_func)(ES, SH_STRDUP);
        let mut mr = (r.shmode_func)(ES, SH_STRDUP);
        for (i, k) in keys.iter().enumerate() {
            mc = sh_put(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
            mr = sh_put(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
        }
        // delete front-first: always the swap-with-last path
        for i in 0..39usize {
            let (m1, tc) = sh_del(c, mc, ES, kp(&keys[i]), 0, HM_STRING);
            let (m2, tr) = sh_del(r, mr, ES, kp(&keys[i]), 0, HM_STRING);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "row41 del #{}", i);
            assert_eq!(tc, 1);
            cmpk(mc, mr, ES, false, &format!("row41 after del #{}", i));
            for k in &keys[(i + 1)..] {
                let (m3, gc) = sh_get(c, mc, ES, kp(k), HM_STRING);
                let (m4, gr) = sh_get(r, mr, ES, kp(k), HM_STRING);
                mc = m3;
                mr = m4;
                assert_eq!(gc, gr);
                assert!(gc >= 0, "row41 key lost after delete #{}", i);
            }
        }
        hm_free(c, mc, ES);
        hm_free(r, mr, ES);
    });
}

/// CONFIGS row 42 — delete on `SH_ARENA` (no per-key free; arena survives rehash).
#[test]
fn cfg_42_arena_delete() {
    run(0x3141_5926, |c, r| unsafe {
        let mut rng = Rng::new(0x5eed_7_0000);
        let keys = make_keys(&mut rng, 60, 10, 40);
        let mut mc = (c.shmode_func)(ES, SH_ARENA);
        let mut mr = (r.shmode_func)(ES, SH_ARENA);
        for (i, k) in keys.iter().enumerate() {
            mc = sh_put(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
            mr = sh_put(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
        }
        // delete enough to trigger the shrink path (used_count < slot_count>>2)
        for i in 0..55usize {
            let (m1, tc) = sh_del(c, mc, ES, kp(&keys[i]), 0, HM_STRING);
            let (m2, tr) = sh_del(r, mr, ES, kp(&keys[i]), 0, HM_STRING);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "row42 del #{}", i);
            cmpk(mc, mr, ES, false, &format!("row42 after del #{}", i));
        }
        hm_free(c, mc, ES);
        hm_free(r, mr, ES);
    });
}

/// CONFIGS row 43 — `hmfree_func` on a `SH_STRDUP` map frees each key.
#[test]
fn cfg_43_hmfree_strdup() {
    run(0x3141_5926, |c, r| unsafe {
        for n in [0usize, 1, 9, 40] {
            let mut rng = Rng::new(0x5eed_8_0000 + n as u64);
            let keys = make_keys(&mut rng, n.max(1), 4, 30);
            let mut mc = (c.shmode_func)(ES, SH_STRDUP);
            let mut mr = (r.shmode_func)(ES, SH_STRDUP);
            for (i, k) in keys.iter().take(n).enumerate() {
                mc = sh_put(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                mr = sh_put(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
            }
            cmpk(mc, mr, ES, n > 0, &format!("row43 n={} before free", n));
            hm_free(c, mc, ES);
            hm_free(r, mr, ES);
        }
    });
}

/// CONFIGS row 44 — `hmfree_func` on a `SH_ARENA` map (`strreset` frees the blocks).
#[test]
fn cfg_44_hmfree_arena() {
    run(0x3141_5926, |c, r| unsafe {
        for n in [0usize, 1, 9, 40, 100] {
            let mut rng = Rng::new(0x5eed_9_0000 + n as u64);
            let keys = make_keys(&mut rng, n.max(1), 10, 40);
            let mut mc = (c.shmode_func)(ES, SH_ARENA);
            let mut mr = (r.shmode_func)(ES, SH_ARENA);
            for (i, k) in keys.iter().take(n).enumerate() {
                mc = sh_put(c, mc, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
                mr = sh_put(r, mr, ES, kp(k), &(i as u64).to_le_bytes(), HM_STRING);
            }
            cmpk(mc, mr, ES, n > 0, &format!("row44 n={} before free", n));
            hm_free(c, mc, ES);
            hm_free(r, mr, ES);
        }
    });
}

/// CONFIGS row 45 — randomized 200-op churn over each of the three string modes.
#[test]
fn cfg_45_string_random_churn() {
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for trial in 0..4u64 {
            run(0x3141_5926, |c, r| unsafe {
                let mut rng = Rng::new(0x5eed_a_0000 + mode as u64 * 100 + trial);
                let pool = make_keys(&mut rng, 50, 1, 30);
                let mut mc = (c.shmode_func)(ES, mode);
                let mut mr = (r.shmode_func)(ES, mode);
                let mut live: Vec<usize> = Vec::new();

                for op in 0..200usize {
                    let choice = rng.below(100);
                    if choice < 50 || live.is_empty() {
                        let idx = rng.below(pool.len());
                        let pay = rng.next_u64().to_le_bytes();
                        mc = sh_put_v(c, mc, ES, kp(&pool[idx]), &pay, HM_STRING);
                        mr = sh_put_v(r, mr, ES, kp(&pool[idx]), &pay, HM_STRING);
                        if !live.contains(&idx) {
                            live.push(idx);
                        }
                    } else if choice < 85 {
                        let li = rng.below(live.len());
                        let idx = live[li];
                        let (m1, tc) = sh_del(c, mc, ES, kp(&pool[idx]), 0, HM_STRING);
                        let (m2, tr) = sh_del(r, mr, ES, kp(&pool[idx]), 0, HM_STRING);
                        mc = m1;
                        mr = m2;
                        assert_eq!(tc, tr, "mode={} trial={} op={} del", mode, trial, op);
                        live.remove(li);
                    } else {
                        let idx = rng.below(pool.len());
                        let (m1, tc) = sh_get(c, mc, ES, kp(&pool[idx]), HM_STRING);
                        let (m2, tr) = sh_get(r, mr, ES, kp(&pool[idx]), HM_STRING);
                        mc = m1;
                        mr = m2;
                        assert_eq!(tc, tr, "mode={} trial={} op={} get", mode, trial, op);
                        assert_eq!(tc >= 0, live.contains(&idx), "presence vs model");
                    }
                    // temp_key is excluded: on SH_STRDUP maps a delete frees
                    // the string temp_key still points at, so reading it is UB
                    // (dangling) in the C too and only reflects heap reuse.
                    cmpk(
                        mc,
                        mr,
                        ES,
                        false,
                        &format!("churn mode={} trial={} op={}", mode, trial, op),
                    );
                }
                hm_free(c, mc, ES);
                hm_free(r, mr, ES);
            });
        }
    }
}
