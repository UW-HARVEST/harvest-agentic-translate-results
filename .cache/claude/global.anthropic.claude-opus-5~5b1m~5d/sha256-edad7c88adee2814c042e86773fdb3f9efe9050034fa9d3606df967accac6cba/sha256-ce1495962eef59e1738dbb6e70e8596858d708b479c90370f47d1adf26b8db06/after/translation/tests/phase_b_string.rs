//! Phase B — valid-path differential tests for the STRING-key hash map,
//! i.e. every `table->string.mode` and every out-of-range `mode` value.
//!
//! Covers `CONFIGS.md` rows 37-51.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEED: u64 = 0xC0FF_EE00_1234_5678;

/// Owns the key buffers. `STBDS_SH_DEFAULT` stores the *caller's* pointer, so
/// the buffers must outlive the map.
struct Keys {
    bufs: Vec<Vec<u8>>,
}

impl Keys {
    fn new() -> Keys {
        Keys { bufs: Vec::new() }
    }
    /// Distinct-by-construction, randomised NUL-terminated key.
    fn push(&mut self, rng: &mut Rng, i: usize, extra: usize) -> *mut c_char {
        let mut v = format!("k{:07}_", i).into_bytes();
        for _ in 0..extra {
            v.push(0x21 + (rng.next_u64() % 94) as u8);
        }
        v.push(0);
        self.bufs.push(v);
        self.bufs.last_mut().unwrap().as_mut_ptr() as *mut c_char
    }
    fn scribble(&mut self) {
        for b in self.bufs.iter_mut() {
            let n = b.len();
            for k in 0..n - 1 {
                b[k] = b'Z';
            }
        }
    }
}

fn build_keys(seed: u64, n: usize, extra: usize) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|i| {
            let mut v = format!("k{:07}_", i).into_bytes();
            for _ in 0..extra {
                v.push(0x21 + (rng.next_u64() % 94) as u8);
            }
            v.push(0);
            v
        })
        .collect()
}

// ===========================================================================
// rows 37/38/39 - the three real string modes, N sweep, elemsize sweep
// ===========================================================================
fn string_mode_scenario(mode_tag: &str, sh_mode: c_int) {
    diff(mode_tag, |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ sh_mode as u64);
        for &n in &[1usize, 2, 6, 7, 10, 13, 60, 200] {
            for &es in &[16usize, 24, 32] {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let mut keys = build_keys(0x5EED_0000 + n as u64 * 31 + es as u64, n, 6);
                let t0 = (lib.shmode_func)(es, sh_mode);
                let mut t = t0;
                log.usz("n", n);
                log.usz("es", es);
                snap_map(log, t, es, KeyKind::StringAt(0));
                for (i, k) in keys.iter_mut().enumerate() {
                    t = shput(
                        lib,
                        t,
                        es,
                        k.as_mut_ptr() as *mut c_char,
                        HM_STRING,
                        8,
                        0x1000_0000 + i as u64,
                        false,
                    );
                    log.usz("i", i);
                    snap_map_tk(log, t, es, KeyKind::StringAt(0));
                }
                // look every key up again with a *fresh* buffer
                let fresh = build_keys(0x5EED_0000 + n as u64 * 31 + es as u64, n, 6);
                for (i, k) in fresh.iter().enumerate() {
                    let mut kk = k.clone();
                    let (nt, idx) = shgeti(lib, t, es, kk.as_mut_ptr() as *mut c_char, HM_STRING);
                    t = nt;
                    log.usz("get", i);
                    log.isz("idx", idx);
                }
                // and a batch of misses
                for j in 0..20usize {
                    let mut miss = format!("absent{:04}", j).into_bytes();
                    miss.push(0);
                    let (nt, idx) = shgeti(lib, t, es, miss.as_mut_ptr() as *mut c_char, HM_STRING);
                    t = nt;
                    log.isz("miss", idx);
                }
                snap_map(log, t, es, KeyKind::StringAt(0));
                hmfree(lib, t, es);
                drop(keys);
            }
        }
    });
}

#[test]
fn cfg37_sh_default() {
    string_mode_scenario("cfg37_SH_DEFAULT", SH_DEFAULT);
}

#[test]
fn cfg38_sh_strdup() {
    string_mode_scenario("cfg38_SH_STRDUP", SH_STRDUP);
}

#[test]
fn cfg39_sh_arena() {
    string_mode_scenario("cfg39_SH_ARENA", SH_ARENA);
}

// ===========================================================================
// row 38b - STRDUP really copies: scribble over the caller's buffers
// ===========================================================================
#[test]
fn cfg38b_strdup_copies_the_key() {
    diff("cfg38b", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 0x38B);
        for &n in &[1usize, 10, 60] {
            (lib.rand_seed)(0x3141_5926);
            let es = 16usize;
            let mut keys = Keys::new();
            let t0 = (lib.shmode_func)(es, SH_STRDUP);
            let mut t = t0;
            for i in 0..n {
                let p = keys.push(&mut rng, i, 5);
                t = shput(lib, t, es, p, HM_STRING, 8, i as u64, false);
            }
            log.usz("n", n);
            snap_map(log, t, es, KeyKind::StringAt(0));
            // now destroy the caller's buffers - the map must be unaffected
            keys.scribble();
            log.tag("after_scribble");
            snap_map(log, t, es, KeyKind::StringAt(0));
            hmfree(lib, t, es);
        }
    });
}

// ===========================================================================
// row 40 - SH_ARENA with long keys -> oversized arena blocks inside the map
// ===========================================================================
#[test]
fn cfg40_sh_arena_long_keys() {
    diff("cfg40", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 40);
        for &n in &[1usize, 10, 60] {
            let hs = rng.next_u64() as usize;
            (lib.rand_seed)(hs);
            let es = 16usize;
            let t0 = (lib.shmode_func)(es, SH_ARENA);
            let mut t = t0;
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..n {
                let extra = 500 + (i * 137) % 1500;
                let mut v = format!("k{:07}_", i).into_bytes();
                for _ in 0..extra {
                    v.push(0x21 + (rng.next_u64() % 94) as u8);
                }
                v.push(0);
                keys.push(v);
            }
            for (i, k) in keys.iter_mut().enumerate() {
                t = shput(
                    lib,
                    t,
                    es,
                    k.as_mut_ptr() as *mut c_char,
                    HM_STRING,
                    8,
                    i as u64,
                    false,
                );
                log.usz("n", n);
                log.usz("i", i);
                snap_map_tk(log, t, es, KeyKind::StringAt(0));
            }
            hmfree(lib, t, es);
        }
    });
}

// ===========================================================================
// row 41 - hmput_key(NULL, mode=1) -> implicit STBDS_SH_DEFAULT
// ===========================================================================
#[test]
fn cfg41_implicit_sh_default() {
    diff("cfg41", |lib, log| unsafe {
        for &n in &[1usize, 10, 60] {
            (lib.rand_seed)(0x3141_5926);
            let es = 24usize;
            let mut keys = build_keys(0xAB_0000 + n as u64, n, 4);
            let mut t: *mut c_void = std::ptr::null_mut();
            for (i, k) in keys.iter_mut().enumerate() {
                t = shput(
                    lib,
                    t,
                    es,
                    k.as_mut_ptr() as *mut c_char,
                    HM_STRING,
                    8,
                    i as u64,
                    false,
                );
                log.usz("n", n);
                log.usz("i", i);
                snap_map_tk(log, t, es, KeyKind::StringAt(0));
            }
            let fresh = build_keys(0xAB_0000 + n as u64, n, 4);
            for (i, k) in fresh.iter().enumerate() {
                let mut kk = k.clone();
                let (nt, idx) = shgeti(lib, t, es, kk.as_mut_ptr() as *mut c_char, HM_STRING);
                t = nt;
                log.usz("g", i);
                log.isz("idx", idx);
            }
            hmfree(lib, t, es);
        }
    });
}

// ===========================================================================
// row 42 - hmput_key(NULL, mode=0) -> implicit string.mode == 0 (SH_NONE)
// ===========================================================================
#[test]
fn cfg42_implicit_sh_none() {
    diff("cfg42", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 42);
        for &n in &[1usize, 10, 60] {
            (lib.rand_seed)(0x3141_5926);
            let es = 16usize;
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..n {
                let k = (i as u64).to_le_bytes();
                t = hmput(lib, t, es, &k, HM_BINARY, rng.next_u64());
            }
            log.usz("n", n);
            snap_map(log, t, es, KeyKind::Binary);
            hmfree(lib, t, es);
        }
    });
}

// ===========================================================================
// row 43 - duplicate string keys: temp_key update on the first-loop hit and
//          (deliberately) no update on the wrap-around hit
// ===========================================================================
#[test]
fn cfg43_string_duplicates_temp_key() {
    diff("cfg43", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 43);
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for rep in 0..4usize {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let es = 24usize;
                let universe = 25usize;
                let all = build_keys(0xDDDD_0000 + rep as u64, universe, 5);
                // keep a persistent copy per index (SH_DEFAULT stores our ptr)
                let mut owned: Vec<Vec<u8>> = all.clone();
                let t0 = (lib.shmode_func)(es, sh);
                let mut t = t0;
                let mut tk = TkValid::new();
                for step in 0..150usize {
                    let i = rng.below(universe);
                    let p = owned[i].as_mut_ptr() as *mut c_char;
                    let before = map_shape(t, es);
                    t = shput(lib, t, es, p, HM_STRING, 8, 0x900 + step as u64, false);
                    tk.after_put(before, map_shape(t, es));
                    log.i32v("sh", sh);
                    log.usz("rep", rep);
                    log.usz("step", step);
                    log.usz("i", i);
                    snap_map_tkv(log, t, es, KeyKind::StringAt(0), &tk);
                }
                hmfree(lib, t, es);
                drop(owned);
            }
        }
    });
}

// ===========================================================================
// row 43b - the `shputs` flavour, which writes hash_table->temp_key back into
//           the element.
//
// NOTE: `shputs` + `SH_STRDUP` + a *duplicate* key that is matched in the
// wrap-around loop (c_src/src/lib.c:746-759) is a genuine upstream stb_ds bug:
// that branch does not refresh `hash_table->temp_key`, so the element ends up
// aliasing another element's strdup'd key and `stbds_hmfree_func` then frees
// the same pointer twice ("free(): double free detected in tcache 2", SIGABRT).
// BOTH libraries do this identically, but the abort destroys the test process
// before anything can be compared, so duplicates are only driven through the
// modes where a stale `temp_key` is harmless:
//   * SH_DEFAULT - the map never owns/frees the keys;
//   * SH_ARENA   - keys are released blockwise by `stbds_strreset`.
// `SH_STRDUP` is driven with distinct keys only (every insert refreshes
// `temp_key`). The wrap-around no-refresh asymmetry itself is still compared,
// by `cfg43_string_duplicates_temp_key`, which snapshots `temp_key` after every
// duplicate put.
// ===========================================================================
#[test]
fn cfg43b_shputs_writes_temp_key_back() {
    diff("cfg43b", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 0x43B);
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (lib.rand_seed)(0x3141_5926);
            let es = 16usize;
            let universe = 20usize;
            let mut owned = build_keys(0xB00B, universe, 4);
            let t0 = (lib.shmode_func)(es, sh);
            let mut t = t0;
            let mut tk = TkValid::new();
            for step in 0..120usize {
                let i = if sh == SH_STRDUP {
                    // distinct keys only
                    if step >= universe {
                        break;
                    }
                    step
                } else {
                    rng.below(universe)
                };
                let p = owned[i].as_mut_ptr() as *mut c_char;
                let before = map_shape(t, es);
                t = shput(lib, t, es, p, HM_STRING, 8, 0x5000 + step as u64, true);
                tk.after_put(before, map_shape(t, es));
                log.i32v("sh", sh);
                log.usz("step", step);
                snap_map_tkv(log, t, es, KeyKind::StringAt(0), &tk);
            }
            hmfree(lib, t, es);
            drop(owned);
        }
    });
}

// ===========================================================================
// row 44 - hmget_key / hmget_key_ts against every string mode
// ===========================================================================
#[test]
fn cfg44_string_get_hit_and_miss() {
    diff("cfg44", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 44);
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &n in &[1usize, 10, 60] {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let es = 16usize;
                let mut owned = build_keys(0x4444_0000 + n as u64, n, 5);
                let t0 = (lib.shmode_func)(es, sh);
                let mut t = t0;
                for i in 0..n {
                    let p = owned[i].as_mut_ptr() as *mut c_char;
                    t = shput(lib, t, es, p, HM_STRING, 8, i as u64, false);
                }
                log.i32v("sh", sh);
                log.usz("n", n);
                snap_map(log, t, es, KeyKind::StringAt(0));
                let fresh = build_keys(0x4444_0000 + n as u64, n, 5);
                for (i, k) in fresh.iter().enumerate() {
                    let mut kk = k.clone();
                    let (nt, idx) = shgeti(lib, t, es, kk.as_mut_ptr() as *mut c_char, HM_STRING);
                    t = nt;
                    log.usz("i", i);
                    log.isz("hit", idx);
                    let mut temp: isize = 0x1234;
                    let nt = (lib.hmget_key_ts)(
                        t,
                        es,
                        kk.as_mut_ptr() as *mut c_void,
                        8,
                        &mut temp,
                        HM_STRING,
                    );
                    t = nt;
                    log.isz("hit_ts", temp);
                }
                for j in 0..40usize {
                    let mut miss = format!("nope-{:05}", j).into_bytes();
                    miss.push(0);
                    let (nt, idx) = shgeti(lib, t, es, miss.as_mut_ptr() as *mut c_char, HM_STRING);
                    t = nt;
                    log.isz("miss", idx);
                    let mut temp: isize = 0x1234;
                    let nt = (lib.hmget_key_ts)(
                        t,
                        es,
                        miss.as_mut_ptr() as *mut c_void,
                        8,
                        &mut temp,
                        HM_STRING,
                    );
                    t = nt;
                    log.isz("miss_ts", temp);
                }
                hmfree(lib, t, es);
                drop(owned);
            }
        }
    });
}

// ===========================================================================
// row 45 - hmdel_key: `mode == STBDS_HM_STRING` frees the strdup'd key,
//          `mode == 2` (out-of-range) does NOT (the C uses `==`, not `>=`).
//
//          For `mode != 1` the C's post-delete fix-up takes the *else* branch
//          at c_src/src/lib.c:845, which hashes the ADDRESS of the moved
//          element instead of its key string. That is address-dependent, so it
//          is only exercised here in the shape where the C skips the fix-up
//          entirely (`old_index == final_index`, i.e. deleting in reverse
//          insertion order).
// ===========================================================================
#[test]
fn cfg45_string_delete_mode_1_vs_2() {
    diff("cfg45", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 45);
        for &del_mode in &[1 as c_int, 2] {
            for &n in &[1usize, 2, 10, 60] {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let es = 16usize;
                let mut owned = build_keys(0x4500_0000 + n as u64, n, 5);
                let t0 = (lib.shmode_func)(es, SH_STRDUP);
                let mut t = t0;
                for i in 0..n {
                    let p = owned[i].as_mut_ptr() as *mut c_char;
                    t = shput(lib, t, es, p, HM_STRING, 8, i as u64, false);
                }
                log.i32v("del_mode", del_mode);
                log.usz("n", n);
                snap_map(log, t, es, KeyKind::StringAt(0));
                // reverse insertion order => old_index == final_index always
                for i in (0..n).rev() {
                    let mut kk = owned[i].clone();
                    let (nt, d) =
                        shdel(lib, t, es, kk.as_mut_ptr() as *mut c_char, 0, del_mode);
                    t = nt;
                    log.usz("i", i);
                    log.isz("d", d);
                    snap_map(log, t, es, KeyKind::StringAt(0));
                }
                hmfree(lib, t, es);
                drop(owned);
            }
        }
    });
}

// ===========================================================================
// row 46 - hmdel_key with mode == 1 on every string mode, random order
//          (this DOES exercise the memmove + string re-find_slot fix-up)
// ===========================================================================
#[test]
fn cfg46_string_delete_random_order() {
    diff("cfg46", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 46);
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &n in &[1usize, 2, 3, 10, 60] {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let es = 16usize;
                let mut owned = build_keys(0x4600_0000 + n as u64 + sh as u64 * 7, n, 5);
                let t0 = (lib.shmode_func)(es, sh);
                let mut t = t0;
                for i in 0..n {
                    let p = owned[i].as_mut_ptr() as *mut c_char;
                    t = shput(lib, t, es, p, HM_STRING, 8, i as u64, false);
                }
                log.i32v("sh", sh);
                log.usz("n", n);
                snap_map(log, t, es, KeyKind::StringAt(0));
                let mut order: Vec<usize> = (0..n).collect();
                for i in (1..n).rev() {
                    let j = rng.below(i + 1);
                    order.swap(i, j);
                }
                for &i in &order {
                    let mut kk = owned[i].clone();
                    let (nt, d) = shdel(lib, t, es, kk.as_mut_ptr() as *mut c_char, 0, HM_STRING);
                    t = nt;
                    log.usz("del", i);
                    log.isz("d", d);
                    snap_map(log, t, es, KeyKind::StringAt(0));
                }
                // deleting again -> not found
                for &i in &order {
                    let mut kk = owned[i].clone();
                    let (nt, d) = shdel(lib, t, es, kk.as_mut_ptr() as *mut c_char, 0, HM_STRING);
                    t = nt;
                    log.isz("redel", d);
                }
                hmfree(lib, t, es);
                drop(owned);
            }
        }
    });
}

// ===========================================================================
// row 47 - string map N=200 -> delete 190 (shrink + rebuild + string re-find)
// ===========================================================================
#[test]
fn cfg47_string_shrink_and_rebuild() {
    diff("cfg47", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 47);
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            let hs = rng.next_u64() as usize;
            (lib.rand_seed)(hs);
            let es = 16usize;
            let n = 200usize;
            let mut owned = build_keys(0x4700_0000 + sh as u64, n, 5);
            let t0 = (lib.shmode_func)(es, sh);
            let mut t = t0;
            for i in 0..n {
                let p = owned[i].as_mut_ptr() as *mut c_char;
                t = shput(lib, t, es, p, HM_STRING, 8, i as u64, false);
            }
            log.i32v("sh", sh);
            snap_map(log, t, es, KeyKind::StringAt(0));
            for i in 0..190usize {
                let mut kk = owned[i].clone();
                let (nt, d) = shdel(lib, t, es, kk.as_mut_ptr() as *mut c_char, 0, HM_STRING);
                t = nt;
                log.usz("i", i);
                log.isz("d", d);
                snap_map(log, t, es, KeyKind::StringAt(0));
            }
            hmfree(lib, t, es);
            drop(owned);
        }
    });
}

// ===========================================================================
// row 48 - hmfree_func across every string.mode and size, plus the degenerate
//          inputs (`hash_table == NULL`, `a == NULL`)
// ===========================================================================
#[test]
fn cfg48_hmfree_all_shapes() {
    diff("cfg48", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 48);

        // a == NULL: pure no-op
        (lib.hmfree_func)(std::ptr::null_mut(), 16);
        log.tag("free_null_ok");

        // array with hash_table == NULL (built by hmput_default)
        for &es in &[8usize, 16, 24] {
            let t = (lib.hmput_default)(std::ptr::null_mut(), es);
            snap_map(log, t, es, KeyKind::Binary);
            hmfree(lib, t, es);
            log.tag("free_no_table_ok");
        }

        for &sh in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &n in &[0usize, 1, 10, 60] {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let es = 16usize;
                let t0 = (lib.shmode_func)(es, sh);
                let mut t = t0;
                let mut owned = build_keys(0x4800_0000 + n as u64 + sh as u64, n, 5);
                for i in 0..n {
                    if sh == SH_NONE {
                        // SH_NONE takes the `memcpy` arm, so drive it the
                        // binary way (mode 0) - a string-mode *lookup* on a
                        // SH_NONE map would strcmp through the memcpy'd bytes
                        // as if they were a pointer.
                        let k = (i as u64).to_le_bytes();
                        t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
                    } else {
                        let p = owned[i].as_mut_ptr() as *mut c_char;
                        t = shput(lib, t, es, p, HM_STRING, 8, i as u64, false);
                    }
                }
                log.i32v("sh", sh);
                log.usz("n", n);
                if sh == SH_NONE {
                    snap_map(log, t, es, KeyKind::Binary);
                } else {
                    snap_map(log, t, es, KeyKind::StringAt(0));
                }
                hmfree(lib, t, es);
                log.tag("freed");
                drop(owned);
            }
        }
    });
}

// ===========================================================================
// row 49 - shmode_func with out-of-range `mode` -> `(unsigned char)` truncation
// ===========================================================================
#[test]
fn cfg49_shmode_out_of_range() {
    diff("cfg49", |lib, log| unsafe {
        let modes: [c_int; 20] = [
            0, 1, 2, 3, 4, 5, 7, 64, 127, 128, 254, 255, 256, 257, 259, 512, -1, -2, c_int::MIN,
            c_int::MAX,
        ];
        for &m in &modes {
            (lib.rand_seed)(0x3141_5926);
            let es = 16usize;
            let t0 = (lib.shmode_func)(es, m);
            let mut t = t0;
            log.i32v("mode", m);
            snap_map(log, t, es, KeyKind::Binary);
            let trunc = (m as u32 & 0xff) as u8;
            log.u8v("expect_trunc", trunc);

            // Now insert through it. Which switch arm the C takes depends only
            // on the truncated `string.mode`.
            let mut owned = build_keys(0x4900_0000u64.wrapping_add(m as u32 as u64), 12, 4);
            for i in 0..12usize {
                if trunc == 1 || trunc == 2 || trunc == 3 {
                    let p = owned[i].as_mut_ptr() as *mut c_char;
                    t = shput(lib, t, es, p, HM_STRING, 8, i as u64, false);
                } else {
                    // default arm: `memcpy(elem, key, keysize)`
                    let k = (i as u64).to_le_bytes();
                    t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
                }
            }
            if trunc == 1 || trunc == 2 || trunc == 3 {
                snap_map_tk(log, t, es, KeyKind::StringAt(0));
            } else {
                snap_map(log, t, es, KeyKind::Binary);
            }
            hmfree(lib, t, es);
            drop(owned);
        }
    });
}

// ===========================================================================
// row 50 - out-of-range `mode` argument to hmput_key / hmget_key / hmdel_key
// ===========================================================================
#[test]
fn cfg50a_out_of_range_mode_string_path() {
    // mode >= STBDS_HM_STRING(1) selects the string path: 1, 2, 3, 255,
    // 256, INT_MAX all behave identically for put/get.
    diff("cfg50a", |lib, log| unsafe {
        let modes: [c_int; 6] = [1, 2, 3, 255, 256, c_int::MAX];
        let mut rng = Rng::new(SEED ^ 0x50A);
        for &m in &modes {
            for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let es = 16usize;
                let n = 30usize;
                let mut owned = build_keys(0x50A0_0000u64.wrapping_add(m as u32 as u64), n, 5);
                let t0 = (lib.shmode_func)(es, sh);
                let mut t = t0;
                for i in 0..n {
                    let p = owned[i].as_mut_ptr() as *mut c_char;
                    t = shput(lib, t, es, p, HM_STRING, 8, i as u64, false);
                }
                log.i32v("mode", m);
                log.i32v("sh", sh);
                // puts with the out-of-range mode (duplicates + new keys)
                let mut extra = build_keys(0x50A1_0000u64.wrapping_add(m as u32 as u64), 10, 5);
                let mut tk = TkValid::new();
                for i in 0..10usize {
                    let p = extra[i].as_mut_ptr() as *mut c_char;
                    let before = map_shape(t, es);
                    t = shput(lib, t, es, p, HM_STRING.max(m), 8, 0x700 + i as u64, false);
                    tk.after_put(before, map_shape(t, es));
                    snap_map_tkv(log, t, es, KeyKind::StringAt(0), &tk);
                }
                for i in 0..n {
                    let p = owned[i].as_mut_ptr() as *mut c_char;
                    let before = map_shape(t, es);
                    t = shput(lib, t, es, p, m, 8, 0x800 + i as u64, false);
                    tk.after_put(before, map_shape(t, es));
                    snap_map_tkv(log, t, es, KeyKind::StringAt(0), &tk);
                }
                // gets with the out-of-range mode
                for i in 0..n {
                    let mut kk = owned[i].clone();
                    let (nt, idx) = shgeti(lib, t, es, kk.as_mut_ptr() as *mut c_char, m);
                    t = nt;
                    log.isz("idx", idx);
                }
                // deletes with the out-of-range mode, in reverse insertion
                // order so `old_index == final_index` and the C skips the
                // address-hashing fix-up branch
                let all_in_order: Vec<Vec<u8>> = owned
                    .iter()
                    .cloned()
                    .chain(extra.iter().cloned())
                    .collect();
                for i in (0..all_in_order.len()).rev() {
                    let mut kk = all_in_order[i].clone();
                    let (nt, d) = shdel(lib, t, es, kk.as_mut_ptr() as *mut c_char, 0, m);
                    t = nt;
                    log.usz("d_i", i);
                    log.isz("d", d);
                    snap_map(log, t, es, KeyKind::StringAt(0));
                }
                hmfree(lib, t, es);
                drop(owned);
                drop(extra);
            }
        }
    });
}

#[test]
fn cfg50b_out_of_range_mode_binary_path() {
    // mode < STBDS_HM_STRING(1) selects the binary path: 0, -1, -2 and
    // INT_MIN must all behave identically (memcmp / memcpy, never strcmp).
    diff("cfg50b", |lib, log| unsafe {
        let modes: [c_int; 4] = [0, -1, -2, c_int::MIN];
        let mut rng = Rng::new(SEED ^ 0x50B);
        for &m in &modes {
            // (i) on a plain binary map
            let hs = rng.next_u64() as usize;
            (lib.rand_seed)(hs);
            let es = 16usize;
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..40usize {
                let k = (i as u64).to_le_bytes();
                t = hmput(lib, t, es, &k, m, rng.next_u64());
            }
            log.i32v("mode", m);
            snap_map(log, t, es, KeyKind::Binary);
            for i in 0..40usize {
                let k = (i as u64).to_le_bytes();
                let (nt, idx) = hmgeti(lib, t, es, &k, m);
                t = nt;
                log.isz("idx", idx);
            }
            for i in 0..40usize {
                let k = (i as u64).to_le_bytes();
                let (nt, d) = hmdel(lib, t, es, &k, 8, 0, m);
                t = nt;
                log.isz("d", d);
                snap_map(log, t, es, KeyKind::Binary);
            }
            hmfree(lib, t, es);

            // (ii) binary mode applied to a STRING map: memcmp compares the
            // caller's string bytes against the element's stored *pointer*, so
            // nothing ever matches -> every put appends, every delete misses.
            let hs = rng.next_u64() as usize;
            (lib.rand_seed)(hs);
            let mut owned = build_keys(0x50B0_0000u64.wrapping_add(m as u32 as u64), 12, 5);
            let t0 = (lib.shmode_func)(es, SH_STRDUP);
            let mut t = t0;
            for i in 0..12usize {
                let p = owned[i].as_mut_ptr() as *mut c_char;
                t = shput(lib, t, es, p, m, 8, i as u64, false);
                log.usz("i", i);
                snap_map_tk(log, t, es, KeyKind::StringAt(0));
            }
            for i in 0..12usize {
                let mut kk = owned[i].clone();
                let (nt, d) = shdel(lib, t, es, kk.as_mut_ptr() as *mut c_char, 0, m);
                t = nt;
                log.isz("nodel", d);
            }
            snap_map(log, t, es, KeyKind::StringAt(0));
            hmfree(lib, t, es);
            drop(owned);
        }
    });
}

#[test]
fn cfg50c_string_mode_on_sh_none_map_inserts_only() {
    // `mode >= 1` on a `string.mode == SH_NONE` map: the insert arm memcpy's
    // the pointer *bytes* into the element, so a later lookup would strcmp
    // through those bytes as if they were a `char *`. Only distinct-key
    // inserts (which never reach `stbds_is_key_equal`) are well defined, and
    // that is exactly what is compared here.
    diff("cfg50c", |lib, log| unsafe {
        for &m in &[1 as c_int, 2, 255] {
            (lib.rand_seed)(0x3141_5926);
            let es = 16usize;
            let mut owned = build_keys(0x50C0_0000u64.wrapping_add(m as u32 as u64), 5, 4);
            let t0 = (lib.shmode_func)(es, SH_NONE);
            let mut t = t0;
            for i in 0..5usize {
                let p = owned[i].as_mut_ptr() as *mut c_char;
                t = (lib.hmput_key)(t, es, p as *mut c_void, 8, m);
                let raw = (t as *mut u8).sub(es) as *mut c_void;
                let idx = (*header(raw)).temp;
                log.i32v("mode", m);
                log.usz("i", i);
                log.isz("idx", idx);
                // only the header / table state is comparable here: the
                // element holds a raw pointer value that differs per library.
                let h = header(raw);
                log.usz("length", (*h).length);
                log.usz("capacity", (*h).capacity);
                let table = (*h).hash_table as *mut HashIndex;
                log.usz("slot_count", (*table).slot_count);
                log.usz("used_count", (*table).used_count);
            }
            hmfree(lib, t, es);
            drop(owned);
        }
    });
}

// ===========================================================================
// row 51 - elemsize == 0
//
// Only the well-defined shape is compared: `string.mode == SH_NONE` together
// with `keysize == 0`, so the insert arm is `memcpy(elem, key, 0)` (writes
// nothing) and `stbds_is_key_equal` is `memcmp(key, elem, 0) == 0` (always
// "equal"). Every element then lives at the same zero-sized address, so the
// second put is a duplicate hit.
//
// Deliberately NOT compared with elemsize == 0:
//   * `string.mode ∈ {SH_DEFAULT, SH_STRDUP, SH_ARENA}` - c_src/src/lib.c:786-788
//     store an 8-byte `char *` into a **zero-byte** element, i.e. a heap
//     overflow past the 32-byte header-only allocation. Both libraries commit
//     the identical overflow, but the consequences are allocator state, not
//     library behaviour.
//   * `SH_STRDUP` / `SH_ARENA` with a NULL key - `strlen(NULL)` segfaults in
//     both.
// ===========================================================================
#[test]
fn cfg51_zero_elemsize() {
    diff("cfg51", |lib, log| unsafe {
        // (a) shmode_func with elemsize 0 -> the hash pointer equals the raw
        //     array pointer, because ARR_TO_HASH adds `elemsize` == 0.
        (lib.rand_seed)(0x3141_5926);
        let t = (lib.shmode_func)(0, SH_NONE);
        log.tag("shmode_es0");
        snap_map(log, t, 0, KeyKind::Binary);
        let mut t = t;
        for i in 0..6usize {
            t = (lib.hmput_key)(t, 0, std::ptr::null_mut(), 0, HM_BINARY);
            log.usz("i", i);
            log.isz("temp", (*header(t)).temp);
            snap_map(log, t, 0, KeyKind::Binary);
        }
        // get + delete with elemsize 0
        let mut temp: isize = 0x99;
        t = (lib.hmget_key_ts)(t, 0, std::ptr::null_mut(), 0, &mut temp, HM_BINARY);
        log.isz("get_ts", temp);
        t = (lib.hmget_key)(t, 0, std::ptr::null_mut(), 0, HM_BINARY);
        log.isz("get", (*header(t)).temp);
        t = (lib.hmdel_key)(t, 0, std::ptr::null_mut(), 0, 0, HM_BINARY);
        log.isz("del", (*header(t)).temp);
        snap_map(log, t, 0, KeyKind::Binary);
        t = (lib.hmdel_key)(t, 0, std::ptr::null_mut(), 0, 0, HM_BINARY);
        log.isz("del2", (*header(t)).temp);
        snap_map(log, t, 0, KeyKind::Binary);
        hmfree(lib, t, 0);

        // (b) hmput_key straight onto NULL with elemsize 0
        (lib.rand_seed)(0x3141_5926);
        let t = (lib.hmput_key)(std::ptr::null_mut(), 0, std::ptr::null_mut(), 0, HM_BINARY);
        log.tag("null_es0");
        snap_map(log, t, 0, KeyKind::Binary);
        hmfree(lib, t, 0);

        // (c) hmput_default / hmget_key_ts with elemsize 0
        let t = (lib.hmput_default)(std::ptr::null_mut(), 0);
        log.tag("default_es0");
        snap_map(log, t, 0, KeyKind::Binary);
        let mut temp: isize = 0x77;
        let t = (lib.hmget_key_ts)(t, 0, std::ptr::null_mut(), 0, &mut temp, HM_BINARY);
        log.isz("temp_no_table", temp);
        snap_map(log, t, 0, KeyKind::Binary);
        hmfree(lib, t, 0);
    });
}
