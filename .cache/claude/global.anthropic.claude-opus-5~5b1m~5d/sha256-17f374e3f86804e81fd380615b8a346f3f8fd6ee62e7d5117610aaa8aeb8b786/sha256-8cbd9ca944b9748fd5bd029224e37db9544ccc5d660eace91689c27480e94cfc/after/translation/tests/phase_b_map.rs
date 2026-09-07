//! Phase B — hash-map differential tests (CONFIGS.md rows 12-37, 45-46).
//!
//! Every map is driven through the *low-level* exported entry points
//! (`stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`,
//! `stbds_hmdel_key`, `stbds_hmput_default`, `stbds_shmode_func`,
//! `stbds_hmfree_func`), with the macro bodies from `lib.c` reproduced by the
//! `Map` driver in `tests/common/map.rs`.
//!
//! After every operation the *entire* observable state is compared: the
//! `stbds_array_header`, all `length*elemsize` element bytes (or key contents
//! for string maps), and the whole `stbds_hash_index` including every bucket's
//! `hash[]` / `index[]` array (CONFIGS.md row 46).

mod common;

use common::map::*;
use common::*;
use std::ffi::{c_char, c_int, c_void};

/// Both libs keep a *global* `stbds_hash_seed`. Every test starts by forcing
/// both to the same value so their tables get identical seeds.
fn setup(seed: usize) -> Pair {
    let p = pair();
    p.reseed(seed);
    p
}

// ===========================================================================
// Rows 12-17 — binary maps, stbds_hmput_key
// ===========================================================================

/// Insert `keys` into a fresh binary map in both libs, comparing state after
/// every single put.
fn run_binary_puts(p: &Pair, elemsize: usize, keysize: usize, keys: &[Vec<u8>], ctx: &str) {
    let mut cm = Map::new(&p.c, elemsize, keysize);
    let mut rm = Map::new(&p.rs, elemsize, keysize);
    for (n, k) in keys.iter().enumerate() {
        let mut ck = k.clone();
        let mut rk = k.clone();
        let value: Vec<u8> = (0..elemsize - keysize).map(|j| (n as u8) ^ (j as u8)).collect();
        let ci = cm.put_binary(&mut ck, &value, HM_BINARY);
        let ri = rm.put_binary(&mut rk, &value, HM_BINARY);
        let c2 = format!("{ctx} put#{n} key={k:02x?}");
        dq(format!("{c2} / index"), ci, ri);
        assert_same_binary(&c2, &cm, &rm);
    }
    // Now look every key up.
    for (n, k) in keys.iter().enumerate() {
        let mut ck = k.clone();
        let mut rk = k.clone();
        let ci = cm.geti(ck.as_mut_ptr() as *mut c_void, HM_BINARY);
        let ri = rm.geti(rk.as_mut_ptr() as *mut c_void, HM_BINARY);
        dq(format!("{ctx} get#{n} key={k:02x?}"), ci, ri);
        assert!(ci >= 0 || keysize == 0, "{ctx}: key #{n} vanished");
    }
    assert_same_binary(&format!("{ctx} after gets"), &cm, &rm);
    cm.free();
    rm.free();
}

#[test]
fn cfg_12_binary_single_int_key() {
    let p = setup(0x3141_5926);
    for v in [0u32, 1, 0xffff_ffff, 0x8000_0000, 42] {
        run_binary_puts(&p, 4, 4, &[v.to_le_bytes().to_vec()], &format!("int key {v:#x}"));
    }
}

#[test]
fn cfg_13_binary_struct_key_grow_thresholds() {
    // struct { int key,b,c,d; } -> elemsize 16, keysize 4
    let mut rng = Rng::new(0x13);
    for &n in &[1usize, 2, 5, 6, 7, 8, 9, 12, 20, 24, 100, 500, 1000] {
        for trial in 0..3 {
            let p = setup(0x3141_5926 + trial);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            let mut seen = std::collections::HashSet::new();
            while keys.len() < n {
                let k = rng.next_u32();
                if seen.insert(k) {
                    keys.push(k.to_le_bytes().to_vec());
                }
            }
            run_binary_puts(&p, 16, 4, &keys, &format!("n={n} trial={trial}"));
        }
    }
    // Sequential keys too (very different hash distribution).
    for &n in &[8usize, 100, 777] {
        let p = setup(0x3141_5926);
        let keys: Vec<Vec<u8>> = (0..n as u32).map(|i| i.to_le_bytes().to_vec()).collect();
        run_binary_puts(&p, 16, 4, &keys, &format!("sequential n={n}"));
    }
}

#[test]
fn cfg_14_binary_wide_keys() {
    let mut rng = Rng::new(0x14);
    // keysize == elemsize == 8
    for &n in &[1usize, 7, 8, 50, 300] {
        let p = setup(0x3141_5926);
        let keys: Vec<Vec<u8>> = (0..n).map(|_| rng.next_u64().to_le_bytes().to_vec()).collect();
        run_binary_puts(&p, 8, 8, &keys, &format!("u64 key n={n}"));
    }
    // struct { int key[2],b,c,d; } -> elemsize 20, keysize 8
    for &n in &[1usize, 8, 50, 300] {
        let p = setup(0x3141_5926);
        let keys: Vec<Vec<u8>> = (0..n).map(|_| rng.next_u64().to_le_bytes().to_vec()).collect();
        run_binary_puts(&p, 20, 8, &keys, &format!("struct2 key n={n}"));
    }
}

#[test]
fn cfg_15_binary_duplicate_puts() {
    let p = setup(0x3141_5926);
    let mut rng = Rng::new(0x15);
    let elemsize = 16usize;
    let keysize = 4usize;
    let mut cm = Map::new(&p.c, elemsize, keysize);
    let mut rm = Map::new(&p.rs, elemsize, keysize);
    let keys: Vec<u32> = (0..40).map(|_| rng.next_u32()).collect();
    // First round of inserts.
    for (n, &k) in keys.iter().enumerate() {
        let mut ck = k.to_le_bytes().to_vec();
        let mut rk = ck.clone();
        let v = vec![n as u8; elemsize - keysize];
        dq(
            format!("insert {n}"),
            cm.put_binary(&mut ck, &v, HM_BINARY),
            rm.put_binary(&mut rk, &v, HM_BINARY),
        );
        assert_same_binary(&format!("insert {n}"), &cm, &rm);
    }
    let len_before = cm.hdr().unwrap().length;
    // Re-put every key several times: hits the "key already present" branch.
    for round in 0..4 {
        for (n, &k) in keys.iter().enumerate() {
            let mut ck = k.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            let v = vec![(n as u8).wrapping_add(round); elemsize - keysize];
            let ci = cm.put_binary(&mut ck, &v, HM_BINARY);
            let ri = rm.put_binary(&mut rk, &v, HM_BINARY);
            dq(format!("re-put round={round} n={n}"), ci, ri);
            assert_same_binary(&format!("re-put round={round} n={n}"), &cm, &rm);
        }
        assert_eq!(cm.hdr().unwrap().length, len_before, "length must not grow on re-put");
        assert_eq!(rm.hdr().unwrap().length, len_before);
    }
    cm.free();
    rm.free();
}

#[test]
fn cfg_16_binary_odd_keysizes() {
    let mut rng = Rng::new(0x16);
    for &keysize in &[1usize, 2, 3, 5, 6, 7, 9, 12, 16] {
        for &extra in &[0usize, 1, 8] {
            let elemsize = keysize + extra;
            for &n in &[1usize, 8, 60] {
                let p = setup(0x3141_5926);
                let mut seen = std::collections::HashSet::new();
                let mut keys: Vec<Vec<u8>> = Vec::new();
                while keys.len() < n {
                    let k = rng.bytes(keysize);
                    if seen.insert(k.clone()) {
                        keys.push(k);
                    }
                    if seen.len() >= (1usize << (8 * keysize.min(2))) {
                        break;
                    }
                }
                run_binary_puts(
                    &p,
                    elemsize,
                    keysize,
                    &keys,
                    &format!("keysize={keysize} elemsize={elemsize} n={keys_len}", keys_len = keys.len()),
                );
            }
        }
    }
}

#[test]
fn cfg_17_binary_zero_keysize() {
    // memcmp(...,0) == 0 always, so *every* key compares equal; whether a slot
    // "matches" is then decided purely by the hash. Fully deterministic, and
    // both libs must agree.
    let p = setup(0x3141_5926);
    let mut rng = Rng::new(0x17);
    let elemsize = 8usize;
    let mut cm = Map::new(&p.c, elemsize, 0);
    let mut rm = Map::new(&p.rs, elemsize, 0);
    for n in 0..40 {
        let mut ck: Vec<u8> = Vec::new();
        let mut rk: Vec<u8> = Vec::new();
        let v = rng.bytes(elemsize);
        let ci = cm.put_binary(&mut ck, &v, HM_BINARY);
        let ri = rm.put_binary(&mut rk, &v, HM_BINARY);
        dq(format!("zero-keysize put#{n}"), ci, ri);
        assert_same_binary(&format!("zero-keysize put#{n}"), &cm, &rm);
    }
    for n in 0..10 {
        let ci = cm.geti(std::ptr::null_mut(), HM_BINARY);
        let ri = rm.geti(std::ptr::null_mut(), HM_BINARY);
        dq(format!("zero-keysize get#{n}"), ci, ri);
    }
    cm.free();
    rm.free();
}

// ===========================================================================
// Rows 18-22 — string maps in every `string.mode`
// ===========================================================================

/// `sh_mode == None` means "let `hmput_key` pick it" (fresh table -> SH_DEFAULT
/// for mode>=1, 0 for mode==0).
fn run_string_puts(
    p: &Pair,
    sh_mode: Option<c_int>,
    put_mode: c_int,
    keys: &[Vec<u8>],
    ctx: &str,
) {
    let elemsize = 16usize; // struct { char *key; int value; ... }
    let keysize = 8usize; // sizeof(char*)
    let (mut cm, mut rm) = match sh_mode {
        None => (
            Map::new(&p.c, elemsize, keysize),
            Map::new(&p.rs, elemsize, keysize),
        ),
        Some(m) => (
            Map::with_shmode(&p.c, elemsize, keysize, m),
            Map::with_shmode(&p.rs, elemsize, keysize, m),
        ),
    };
    // Keep the caller-side key buffers alive forever: SH_DEFAULT stores them.
    let mut kptrs: Vec<*mut c_char> = Vec::new();
    for (n, k) in keys.iter().enumerate() {
        // Each library gets its *own* copy of the key buffer, so that
        // SH_DEFAULT's stored pointers are independent.
        let ckp = leak_cstring(k);
        let rkp = leak_cstring(k);
        kptrs.push(ckp);
        let value: Vec<u8> = (0..elemsize - keysize).map(|j| (n as u8) ^ (j as u8)).collect();
        let ci = cm.put_string(ckp, &value, put_mode);
        let ri = rm.put_string(rkp, &value, put_mode);
        let c2 = format!("{ctx} put#{n} key={:?}", String::from_utf8_lossy(k));
        dq(format!("{c2} / index"), ci, ri);
        assert_same_string(&c2, &cm, &rm);
    }
    // Lookups with *fresh* (pointer-different, content-equal) key buffers.
    for (n, k) in keys.iter().enumerate() {
        let ckp = leak_cstring(k);
        let rkp = leak_cstring(k);
        let ci = cm.geti(ckp as *mut c_void, put_mode.max(HM_STRING));
        let ri = rm.geti(rkp as *mut c_void, put_mode.max(HM_STRING));
        dq(
            format!("{ctx} get#{n} key={:?}", String::from_utf8_lossy(k)),
            ci,
            ri,
        );
        assert!(ci >= 0, "{ctx}: string key #{n} vanished");
    }
    // A miss.
    let miss_c = leak_cstring(b"\x01definitely absent key \x7f");
    let miss_r = leak_cstring(b"\x01definitely absent key \x7f");
    dq(
        format!("{ctx} miss"),
        cm.geti(miss_c as *mut c_void, HM_STRING),
        rm.geti(miss_r as *mut c_void, HM_STRING),
    );
    assert_same_string(&format!("{ctx} final"), &cm, &rm);
    cm.free();
    rm.free();
}

fn random_string_keys(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let len = 1 + rng.below(24);
        let s = rng.cstring(len);
        let s = s[..s.len() - 1].to_vec(); // drop the NUL; leak_cstring re-adds it
        if seen.insert(s.clone()) {
            out.push(s);
        }
    }
    out
}

#[test]
fn cfg_18_string_default_mode_implicit() {
    let mut rng = Rng::new(0x18);
    for &n in &[1usize, 5, 6, 7, 8, 20, 100, 400] {
        let p = setup(0x3141_5926);
        let keys = random_string_keys(&mut rng, n);
        run_string_puts(&p, None, HM_STRING, &keys, &format!("implicit SH_DEFAULT n={n}"));
    }
    // strkey-style keys (as in the C's own unit tests).
    let p = setup(0x3141_5926);
    let keys: Vec<Vec<u8>> = (0..300).map(|i| format!("test_{i}").into_bytes()).collect();
    run_string_puts(&p, None, HM_STRING, &keys, "implicit SH_DEFAULT strkey");
}

/// `stbds_temp_key(t-1)` — the `stbds_hash_index::temp_key` field — is only a
/// defined observable directly after `stbds_hmput_key` *inserted* a new key in
/// a string mode (lines 786-788). `stbds_make_hash_index` never initialises the
/// field, so it is uninitialised memory after any grow / shrink / rebuild.
#[test]
fn cfg_18b_temp_key_after_insert() {
    let mut rng = Rng::new(0x18_b);
    for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        let p = setup(0x3141_5926);
        let elemsize = 16usize;
        let keysize = 8usize;
        let (mut cm, mut rm) = match sh {
            None => (
                Map::new(&p.c, elemsize, keysize),
                Map::new(&p.rs, elemsize, keysize),
            ),
            Some(m) => (
                Map::with_shmode(&p.c, elemsize, keysize, m),
                Map::with_shmode(&p.rs, elemsize, keysize, m),
            ),
        };
        // Insert-only (no deletes): every put either inserts a brand-new key
        // (which sets temp_key) or, on the very next line, matches in the first
        // probe loop (which also sets temp_key).
        let keys = random_string_keys(&mut rng, 300);
        for (n, k) in keys.iter().enumerate() {
            let v = vec![n as u8; elemsize - keysize];
            let ci = cm.put_string(leak_cstring(k), &v, HM_STRING);
            let ri = rm.put_string(leak_cstring(k), &v, HM_STRING);
            let ctx = format!("temp_key sh={sh:?} put#{n}");
            dq(format!("{ctx} / index"), ci, ri);
            // A brand-new insert always lands at the last element.
            assert_eq!(ci, cm.len() - 1, "[{ctx}] expected a fresh insert");
            dq(format!("{ctx} / temp_key"), cm.temp_key(), rm.temp_key());
            assert_eq!(
                cm.temp_key().as_deref(),
                Some(&k[..]),
                "[{ctx}] temp_key must be the just-inserted key"
            );
            assert_same_string(&ctx, &cm, &rm);
        }
        cm.free();
        rm.free();
    }

    // A *binary* map never touches temp_key (the `default:` memcpy branch), so
    // it must stay whatever the (uninitialised) table had — not compared, but
    // the rest of the state must still match.
    let p = setup(0x3141_5926);
    let mut cm = Map::with_shmode(&p.c, 16, 4, SH_NONE);
    let mut rm = Map::with_shmode(&p.rs, 16, 4, SH_NONE);
    for i in 0..100u32 {
        let mut ck = i.to_le_bytes().to_vec();
        let mut rk = ck.clone();
        let v = vec![i as u8; 12];
        dq(
            format!("binary temp_key untouched #{i}"),
            cm.put_binary(&mut ck, &v, HM_BINARY),
            rm.put_binary(&mut rk, &v, HM_BINARY),
        );
        assert_same_binary(&format!("binary temp_key untouched #{i}"), &cm, &rm);
    }
    cm.free();
    rm.free();
}

/// The documented quirk at `lib.c:747-751`: when a re-put finds the key in the
/// *wrap-around* probe loop, the C does **not** update `stbds_temp_key` (unlike
/// the first loop at 732-733). Exercised on a table that never grows, so
/// `temp_key` is a valid pointer throughout (see the note in
/// `cfg_18b_temp_key_after_insert`), and with the *same* key buffers handed to
/// both libraries so `temp_key` is comparable as a raw pointer.
#[test]
fn cfg_18c_temp_key_wraparound_quirk() {
    let p = setup(0x3141_5926);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut cm = Map::with_shmode(&p.c, elemsize, keysize, SH_DEFAULT);
    let mut rm = Map::with_shmode(&p.rs, elemsize, keysize, SH_DEFAULT);
    // Fill the table to just below its grow threshold so the re-put phase never
    // reallocates the index (which would leave `temp_key` uninitialised): the
    // sequence 6->16, 12->32, 24->64 means 47 inserts land on slot_count 64
    // with used_count 47 and used_count_threshold 48. At 73% load many buckets
    // are full in the `(pos&7)..7` range, which is what forces lookups into the
    // wrap-around loop.
    let kptrs: Vec<*mut c_char> = (0..47)
        .map(|i| leak_cstring(format!("wrap_{i}").as_bytes()))
        .collect();
    for (i, &kp) in kptrs.iter().enumerate() {
        let v = vec![i as u8; elemsize - keysize];
        // SH_DEFAULT only stores the pointer, so sharing it between the two
        // libraries is safe and makes `temp_key` pointer-comparable.
        dq(
            format!("wrap insert#{i}"),
            cm.put_string(kp, &v, HM_STRING),
            rm.put_string(kp, &v, HM_STRING),
        );
        dq(
            format!("wrap insert#{i} / temp_key ptr"),
            unsafe { raw_temp_key(&cm) },
            unsafe { raw_temp_key(&rm) },
        );
        assert_same_string(&format!("wrap insert#{i}"), &cm, &rm);
    }
    assert_eq!(
        cm.table().unwrap().slot_count,
        64,
        "table must be at slot_count 64"
    );
    assert_eq!(cm.table().unwrap().used_count, 47);
    // Re-put every key many times in every order: some lookups resolve in the
    // first in-bucket loop (temp_key updated), some in the wrap-around loop
    // (temp_key left stale). Both libraries must agree either way.
    let mut stale_hits = 0usize;
    let mut fresh_hits = 0usize;
    for round in 0..40 {
        for (i, &kp) in kptrs.iter().enumerate() {
            let v = vec![(round as u8).wrapping_add(i as u8); elemsize - keysize];
            let ci = cm.put_string(kp, &v, HM_STRING);
            let ri = rm.put_string(kp, &v, HM_STRING);
            let ctx = format!("wrap re-put round={round} i={i}");
            dq(format!("{ctx} / index"), ci, ri);
            let ctk = unsafe { raw_temp_key(&cm) };
            dq(
                format!("{ctx} / temp_key ptr"),
                ctk,
                unsafe { raw_temp_key(&rm) },
            );
            if ctk == kp as usize {
                fresh_hits += 1; // matched in the first in-bucket loop
            } else {
                stale_hits += 1; // matched in the wrap-around loop -> stale
            }
            assert_same_string(&ctx, &cm, &rm);
        }
        assert_eq!(cm.table().unwrap().slot_count, 64, "must not have grown");
        assert_eq!(cm.len(), 47);
    }
    // Prove the quirky branch was actually taken (otherwise this test would be
    // vacuous).
    assert!(
        stale_hits > 0,
        "never reached the wrap-around probe loop (fresh={fresh_hits})"
    );
    assert!(fresh_hits > 0, "never reached the first probe loop");
    cm.free();
    rm.free();
}

/// `stbds_temp_key(t-1)` as a raw pointer (usable only when both libraries were
/// handed the identical key buffer, i.e. `STBDS_SH_DEFAULT`).
unsafe fn raw_temp_key(m: &Map) -> usize {
    unsafe {
        let hdr = header(hash_to_arr(m.h, m.elemsize));
        *(hdr.hash_table as *mut usize)
    }
}

#[test]
fn cfg_19_string_strdup_mode() {
    let mut rng = Rng::new(0x19);
    for &n in &[1usize, 6, 8, 20, 100, 400] {
        let p = setup(0x3141_5926);
        let keys = random_string_keys(&mut rng, n);
        run_string_puts(
            &p,
            Some(SH_STRDUP),
            HM_STRING,
            &keys,
            &format!("SH_STRDUP n={n}"),
        );
    }
    // Verify the stored pointer really is a *copy* (not the caller's buffer).
    let p = setup(0x3141_5926);
    let elemsize = 16usize;
    let mut cm = Map::with_shmode(&p.c, elemsize, 8, SH_STRDUP);
    let mut rm = Map::with_shmode(&p.rs, elemsize, 8, SH_STRDUP);
    let k = leak_cstring(b"copy-me");
    cm.put_string(k, &[0u8; 8], HM_STRING);
    rm.put_string(k, &[0u8; 8], HM_STRING);
    for m in [&cm, &rm] {
        let stored = unsafe { *(m.elem(1) as *mut *mut c_char) };
        assert_ne!(stored, k, "{}: SH_STRDUP must copy the key", m.lib.name);
        assert_eq!(unsafe { cstr(stored) }, b"copy-me");
    }
    assert_same_string("strdup copy", &cm, &rm);
    cm.free();
    rm.free();
}

#[test]
fn cfg_20_string_arena_mode() {
    let mut rng = Rng::new(0x20);
    for &n in &[1usize, 6, 8, 20, 100, 400, 900] {
        let p = setup(0x3141_5926);
        let keys = random_string_keys(&mut rng, n);
        run_string_puts(
            &p,
            Some(SH_ARENA),
            HM_STRING,
            &keys,
            &format!("SH_ARENA n={n}"),
        );
    }
    // Long keys, to drive the arena's block chain and the oversize path.
    let p = setup(0x3141_5926);
    let keys: Vec<Vec<u8>> = (0..60)
        .map(|i| {
            let len = 1 + (i * 137) % 4000;
            let mut v = vec![b'A' + (i % 26) as u8; len];
            v.extend_from_slice(format!("#{i}").as_bytes());
            v
        })
        .collect();
    run_string_puts(&p, Some(SH_ARENA), HM_STRING, &keys, "SH_ARENA long keys");
}

#[test]
fn cfg_21_shmode_none_binary_puts() {
    // A table created by shmode_func with SH_NONE, then driven with *binary*
    // puts: `switch (table->string.mode)` falls to `default` -> memcpy.
    let mut rng = Rng::new(0x21);
    for &n in &[1usize, 8, 60, 300] {
        let p = setup(0x3141_5926);
        let elemsize = 16usize;
        let keysize = 4usize;
        let mut cm = Map::with_shmode(&p.c, elemsize, keysize, SH_NONE);
        let mut rm = Map::with_shmode(&p.rs, elemsize, keysize, SH_NONE);
        let mut seen = std::collections::HashSet::new();
        let mut count = 0;
        while count < n {
            let k = rng.next_u32();
            if !seen.insert(k) {
                continue;
            }
            let mut ck = k.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            let v = vec![count as u8; elemsize - keysize];
            dq(
                format!("SH_NONE n={n} put#{count}"),
                cm.put_binary(&mut ck, &v, HM_BINARY),
                rm.put_binary(&mut rk, &v, HM_BINARY),
            );
            assert_same_binary(&format!("SH_NONE n={n} put#{count}"), &cm, &rm);
            count += 1;
        }
        assert_eq!(cm.table().unwrap().arena_mode, SH_NONE as u8);
        cm.free();
        rm.free();
    }
}

#[test]
fn cfg_22_shmode_default_explicit() {
    let mut rng = Rng::new(0x22);
    for &n in &[1usize, 8, 60, 300] {
        let p = setup(0x3141_5926);
        let keys = random_string_keys(&mut rng, n);
        run_string_puts(
            &p,
            Some(SH_DEFAULT),
            HM_STRING,
            &keys,
            &format!("explicit SH_DEFAULT n={n}"),
        );
    }
}

// ===========================================================================
// Rows 23-25 — stbds_hmget_key_ts / stbds_hmget_key
// ===========================================================================

#[test]
fn cfg_23_hmget_ts_binary() {
    let p = setup(0x3141_5926);
    let mut rng = Rng::new(0x23);
    let elemsize = 16usize;
    let keysize = 4usize;

    // (a) a == NULL: allocates and sets *temp = -1
    {
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        let mut k = 1234u32.to_le_bytes();
        let ct = cm.geti_ts(k.as_mut_ptr() as *mut c_void, HM_BINARY);
        let rt = rm.geti_ts(k.as_mut_ptr() as *mut c_void, HM_BINARY);
        dq("ts on NULL / temp", ct, rt);
        assert_eq!(ct, -1);
        dq("ts on NULL / header", cm.hdr(), rm.hdr());
        dq("ts on NULL / table", cm.table(), rm.table());
        assert!(cm.table().is_none(), "no table should exist yet");
        cm.free();
        rm.free();
    }

    // (b) populated: hits and misses
    let mut cm = Map::new(&p.c, elemsize, keysize);
    let mut rm = Map::new(&p.rs, elemsize, keysize);
    let present: Vec<u32> = (0..200u32).map(|_| rng.next_u32() | 1).collect();
    for (n, &k) in present.iter().enumerate() {
        let mut ck = k.to_le_bytes().to_vec();
        let mut rk = ck.clone();
        let v = vec![n as u8; elemsize - keysize];
        cm.put_binary(&mut ck, &v, HM_BINARY);
        rm.put_binary(&mut rk, &v, HM_BINARY);
    }
    for (n, &k) in present.iter().enumerate() {
        let mut ck = k.to_le_bytes();
        let mut rk = k.to_le_bytes();
        dq(
            format!("ts hit #{n}"),
            cm.geti_ts(ck.as_mut_ptr() as *mut c_void, HM_BINARY),
            rm.geti_ts(rk.as_mut_ptr() as *mut c_void, HM_BINARY),
        );
    }
    for n in 0..200 {
        // even numbers are never present (all keys have bit 0 set)
        let k = (n as u32) << 1;
        let mut ck = k.to_le_bytes();
        let mut rk = k.to_le_bytes();
        let ct = cm.geti_ts(ck.as_mut_ptr() as *mut c_void, HM_BINARY);
        let rt = rm.geti_ts(rk.as_mut_ptr() as *mut c_void, HM_BINARY);
        dq(format!("ts miss #{n}"), ct, rt);
        assert_eq!(ct, -1, "expected a miss for {k}");
    }
    assert_same_binary("ts final", &cm, &rm);
    cm.free();
    rm.free();
}

#[test]
fn cfg_24_hmget_ts_string_all_modes() {
    let mut rng = Rng::new(0x24);
    for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        let p = setup(0x3141_5926);
        let elemsize = 16usize;
        let keysize = 8usize;
        let (mut cm, mut rm) = match sh {
            None => (
                Map::new(&p.c, elemsize, keysize),
                Map::new(&p.rs, elemsize, keysize),
            ),
            Some(m) => (
                Map::with_shmode(&p.c, elemsize, keysize, m),
                Map::with_shmode(&p.rs, elemsize, keysize, m),
            ),
        };
        let keys = random_string_keys(&mut rng, 150);
        for (n, k) in keys.iter().enumerate() {
            let v = vec![n as u8; elemsize - keysize];
            cm.put_string(leak_cstring(k), &v, HM_STRING);
            rm.put_string(leak_cstring(k), &v, HM_STRING);
        }
        let ctx = format!("string ts sh={sh:?}");
        // hits with pointer-different but content-equal keys
        for (n, k) in keys.iter().enumerate() {
            dq(
                format!("{ctx} hit#{n}"),
                cm.geti_ts(leak_cstring(k) as *mut c_void, HM_STRING),
                rm.geti_ts(leak_cstring(k) as *mut c_void, HM_STRING),
            );
        }
        // misses
        for n in 0..150 {
            let miss = format!("\x02absent-{n}").into_bytes();
            let ct = cm.geti_ts(leak_cstring(&miss) as *mut c_void, HM_STRING);
            let rt = rm.geti_ts(leak_cstring(&miss) as *mut c_void, HM_STRING);
            dq(format!("{ctx} miss#{n}"), ct, rt);
            assert_eq!(ct, -1);
        }
        assert_same_string(&ctx, &cm, &rm);
        cm.free();
        rm.free();
    }
}

#[test]
fn cfg_25_hmget_key_writes_temp() {
    // The non-_ts wrapper additionally stores the result into
    // stbds_header(p - elemsize)->temp; verify via the header snapshot.
    let p = setup(0x3141_5926);
    let mut rng = Rng::new(0x25);
    let elemsize = 12usize;
    let keysize = 4usize;
    let mut cm = Map::new(&p.c, elemsize, keysize);
    let mut rm = Map::new(&p.rs, elemsize, keysize);
    let keys: Vec<u32> = (0..120).map(|_| rng.next_u32() | 1).collect();
    for (n, &k) in keys.iter().enumerate() {
        let mut ck = k.to_le_bytes().to_vec();
        let mut rk = ck.clone();
        let v = vec![n as u8; elemsize - keysize];
        cm.put_binary(&mut ck, &v, HM_BINARY);
        rm.put_binary(&mut rk, &v, HM_BINARY);
    }
    for (n, &k) in keys.iter().enumerate() {
        for probe in [k, k ^ 1] {
            let mut ck = probe.to_le_bytes();
            let mut rk = probe.to_le_bytes();
            let ci = cm.geti(ck.as_mut_ptr() as *mut c_void, HM_BINARY);
            let ri = rm.geti(rk.as_mut_ptr() as *mut c_void, HM_BINARY);
            dq(format!("get#{n} probe={probe:#x}"), ci, ri);
            dq(format!("get#{n} header"), cm.hdr(), rm.hdr());
            assert_eq!(cm.hdr().unwrap().temp, ci, "temp must equal the returned index");
        }
    }
    // a == NULL through hmget_key
    let mut k = 7u32.to_le_bytes();
    let ch = unsafe {
        (p.c.hmget_key)(
            std::ptr::null_mut(),
            elemsize,
            k.as_mut_ptr() as *mut c_void,
            keysize,
            HM_BINARY,
        )
    };
    let rh = unsafe {
        (p.rs.hmget_key)(
            std::ptr::null_mut(),
            elemsize,
            k.as_mut_ptr() as *mut c_void,
            keysize,
            HM_BINARY,
        )
    };
    dq(
        "hmget_key(NULL) header",
        unsafe { hash_hdr_snapshot(ch, elemsize) },
        unsafe { hash_hdr_snapshot(rh, elemsize) },
    );
    assert_eq!(unsafe { hash_hdr_snapshot(ch, elemsize) }.temp, -1);
    unsafe {
        (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
        (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
    }
    cm.free();
    rm.free();
}

// ===========================================================================
// Rows 26-30 — stbds_hmdel_key
// ===========================================================================

/// Build a binary map with `n` distinct u32 keys.
fn build_binary<'a>(
    p: &'a Pair,
    elemsize: usize,
    keysize: usize,
    keys: &[u32],
) -> (Map<'a>, Map<'a>) {
    let mut cm = Map::new(&p.c, elemsize, keysize);
    let mut rm = Map::new(&p.rs, elemsize, keysize);
    for (n, &k) in keys.iter().enumerate() {
        let mut ck = k.to_le_bytes()[..keysize].to_vec();
        let mut rk = ck.clone();
        let v = vec![n as u8; elemsize - keysize];
        cm.put_binary(&mut ck, &v, HM_BINARY);
        rm.put_binary(&mut rk, &v, HM_BINARY);
    }
    assert_same_binary("build_binary", &cm, &rm);
    (cm, rm)
}

#[test]
fn cfg_26_hmdel_interior() {
    let mut rng = Rng::new(0x26);
    for &n in &[2usize, 3, 8, 30, 200] {
        let p = setup(0x3141_5926);
        let keys: Vec<u32> = {
            let mut seen = std::collections::HashSet::new();
            let mut v = Vec::new();
            while v.len() < n {
                let k = rng.next_u32();
                if seen.insert(k) {
                    v.push(k);
                }
            }
            v
        };
        let (mut cm, mut rm) = build_binary(&p, 16, 4, &keys);
        // Delete keys in a shuffled order; most deletes are interior.
        let mut order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
        for (step, &oi) in order.iter().enumerate() {
            let k = keys[oi];
            let mut ck = k.to_le_bytes();
            let mut rk = k.to_le_bytes();
            let ctx = format!("n={n} del step={step} key={k:#x}");
            dq(
                format!("{ctx} / result"),
                cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
            );
            assert_same_binary(&ctx, &cm, &rm);
            // every remaining key must still be findable, identically
            for (j, &kk) in keys.iter().enumerate() {
                if order[..=step].contains(&j) {
                    continue;
                }
                let mut a = kk.to_le_bytes();
                let mut b = kk.to_le_bytes();
                dq(
                    format!("{ctx} / lookup {kk:#x}"),
                    cm.geti(a.as_mut_ptr() as *mut c_void, HM_BINARY),
                    rm.geti(b.as_mut_ptr() as *mut c_void, HM_BINARY),
                );
            }
        }
        cm.free();
        rm.free();
    }
}

#[test]
fn cfg_27_hmdel_last_element() {
    let mut rng = Rng::new(0x27_0000);
    for &n in &[1usize, 2, 8, 40, 200] {
        let p = setup(0x3141_5926);
        let keys: Vec<u32> = {
            let mut seen = std::collections::HashSet::new();
            let mut v = Vec::new();
            while v.len() < n {
                let k = rng.next_u32();
                if seen.insert(k) {
                    v.push(k);
                }
            }
            v
        };
        let (mut cm, mut rm) = build_binary(&p, 16, 4, &keys);
        // Repeatedly delete whatever key currently sits in the LAST element,
        // which is the `old_index == final_index` fast path.
        for step in 0..n {
            let len = cm.hdr().unwrap().length;
            assert_eq!(len, rm.hdr().unwrap().length);
            let last = len - 1;
            let mut ck = unsafe { std::slice::from_raw_parts(cm.elem(last), 4) }.to_vec();
            let mut rk = unsafe { std::slice::from_raw_parts(rm.elem(last), 4) }.to_vec();
            assert_eq!(ck, rk, "element bytes diverged before delete");
            let ctx = format!("n={n} last-del step={step}");
            dq(
                format!("{ctx} / result"),
                cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
            );
            assert_same_binary(&ctx, &cm, &rm);
        }
        assert_eq!(cm.len(), 0);
        cm.free();
        rm.free();
    }
}

#[test]
fn cfg_28_29_30_hmdel_shrink_rebuild_tombstone() {
    let mut rng = Rng::new(0x28);
    // Insert enough to reach slot_count 64+, then delete most of them; this
    // walks through the tombstone-rebuild path and then the shrink path.
    for &n in &[40usize, 100, 400, 1500] {
        let p = setup(0x3141_5926);
        let keys: Vec<u32> = {
            let mut seen = std::collections::HashSet::new();
            let mut v = Vec::new();
            while v.len() < n {
                let k = rng.next_u32();
                if seen.insert(k) {
                    v.push(k);
                }
            }
            v
        };
        let (mut cm, mut rm) = build_binary(&p, 16, 4, &keys);
        let big = cm.table().unwrap().slot_count;
        assert!(big >= 64, "expected a grown table, got {big}");
        let mut saw_shrink = false;
        let mut saw_rebuild = false;
        let mut prev = cm.table().unwrap();
        for (step, &k) in keys.iter().enumerate() {
            let mut ck = k.to_le_bytes();
            let mut rk = k.to_le_bytes();
            let ctx = format!("n={n} shrink/rebuild step={step}");
            dq(
                format!("{ctx} / result"),
                cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
            );
            assert_same_binary(&ctx, &cm, &rm);
            let now = cm.table().unwrap();
            if now.slot_count < prev.slot_count {
                saw_shrink = true;
            } else if now.slot_count == prev.slot_count
                && now.tombstone_count < prev.tombstone_count
                && prev.tombstone_count > prev.tombstone_count_threshold
            {
                saw_rebuild = true;
            }
            prev = now;
        }
        assert!(saw_shrink, "n={n}: never exercised the shrink path");
        let _ = saw_rebuild; // the shrink check (line 854) wins on pure deletes
        cm.free();
        rm.free();
    }

    // Row 30 — tombstone reuse: delete then re-insert, repeatedly.
    for &n in &[10usize, 60, 300] {
        let p = setup(0x3141_5926);
        let keys: Vec<u32> = {
            let mut seen = std::collections::HashSet::new();
            let mut v = Vec::new();
            while v.len() < n {
                let k = rng.next_u32();
                if seen.insert(k) {
                    v.push(k);
                }
            }
            v
        };
        let (mut cm, mut rm) = build_binary(&p, 16, 4, &keys);
        // Batched delete-then-reinsert keeps `used_count` high (so the shrink
        // check never fires) while letting tombstones accumulate past
        // `tombstone_count_threshold`, which triggers the *rebuild* branch
        // (line 858). Re-inserting afterwards hits the tombstone-reuse branch
        // (`found_empty_slot`'s `tombstone >= 0`).
        let mut saw_rebuild = false;
        let mut saw_tombstone_reuse = false;
        let batch = (n / 2).max(5);
        for round in 0..6 {
            let mut prev = cm.table().unwrap();
            for step in 0..batch {
                let k = keys[(round * 3 + step) % n];
                let mut ck = k.to_le_bytes();
                let mut rk = k.to_le_bytes();
                let ctx = format!("tomb n={n} round={round} del step={step}");
                dq(
                    format!("{ctx} / del"),
                    cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                    rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                );
                assert_same_binary(&ctx, &cm, &rm);
                let now = cm.table().unwrap();
                if now.slot_count == prev.slot_count
                    && now.tombstone_count == 0
                    && prev.tombstone_count >= prev.tombstone_count_threshold
                {
                    saw_rebuild = true;
                }
                prev = now;
            }
            for step in 0..batch {
                let k = keys[(round * 3 + step) % n];
                let mut ck = k.to_le_bytes().to_vec();
                let mut rk = ck.clone();
                let v = vec![step as u8; 12];
                let ctx = format!("tomb n={n} round={round} put step={step}");
                dq(
                    format!("{ctx} / re-put"),
                    cm.put_binary(&mut ck, &v, HM_BINARY),
                    rm.put_binary(&mut rk, &v, HM_BINARY),
                );
                assert_same_binary(&ctx, &cm, &rm);
                let now = cm.table().unwrap();
                if now.tombstone_count < prev.tombstone_count {
                    saw_tombstone_reuse = true;
                }
                prev = now;
            }
        }
        assert!(saw_rebuild, "n={n}: never exercised the rebuild path");
        assert!(
            saw_tombstone_reuse,
            "n={n}: never exercised the tombstone-reuse path"
        );
        cm.free();
        rm.free();
    }
}

// ===========================================================================
// Rows 31-32 — string deletes in every string.mode
// ===========================================================================

#[test]
fn cfg_31_32_string_deletes() {
    let mut rng = Rng::new(0x31);
    for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        for &n in &[1usize, 2, 8, 60, 300] {
            let p = setup(0x3141_5926);
            let elemsize = 16usize;
            let keysize = 8usize;
            let (mut cm, mut rm) = match sh {
                None => (
                    Map::new(&p.c, elemsize, keysize),
                    Map::new(&p.rs, elemsize, keysize),
                ),
                Some(m) => (
                    Map::with_shmode(&p.c, elemsize, keysize, m),
                    Map::with_shmode(&p.rs, elemsize, keysize, m),
                ),
            };
            let keys = random_string_keys(&mut rng, n);
            for (i, k) in keys.iter().enumerate() {
                let v = vec![i as u8; elemsize - keysize];
                cm.put_string(leak_cstring(k), &v, HM_STRING);
                rm.put_string(leak_cstring(k), &v, HM_STRING);
            }
            assert_same_string(&format!("sh={sh:?} n={n} built"), &cm, &rm);
            // Shuffle the delete order so both interior and last-element
            // deletes happen.
            let mut order: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            for (step, &oi) in order.iter().enumerate() {
                let ctx = format!("sh={sh:?} n={n} del step={step}");
                dq(
                    format!("{ctx} / result"),
                    cm.del(leak_cstring(&keys[oi]) as *mut c_void, 0, HM_STRING),
                    rm.del(leak_cstring(&keys[oi]) as *mut c_void, 0, HM_STRING),
                );
                assert_same_string(&ctx, &cm, &rm);
                // remaining keys still findable
                for (j, kk) in keys.iter().enumerate() {
                    if order[..=step].contains(&j) {
                        continue;
                    }
                    dq(
                        format!("{ctx} / lookup"),
                        cm.geti(leak_cstring(kk) as *mut c_void, HM_STRING),
                        rm.geti(leak_cstring(kk) as *mut c_void, HM_STRING),
                    );
                }
            }
            assert_eq!(cm.len(), 0);
            cm.free();
            rm.free();
        }
    }
}

// ===========================================================================
// Rows 33-36 — full randomized pipelines
// ===========================================================================

#[derive(Debug)]
enum Op {
    Put(u32),
    Get(u32),
    Del(u32),
}

#[test]
fn cfg_33_pipeline_binary() {
    for (elemsize, keysize) in [(4usize, 4usize), (16, 4), (8, 8), (20, 8), (5, 1), (11, 3)] {
        for seedvar in 0..3usize {
            let p = setup(0x3141_5926 + seedvar * 7919);
            let mut rng = Rng::new(0x33_0000 + (elemsize * 100 + keysize + seedvar) as u64);
            let mut cm = Map::new(&p.c, elemsize, keysize);
            let mut rm = Map::new(&p.rs, elemsize, keysize);
            // Small key space -> lots of collisions, tombstones, re-inserts.
            let space = 300u32;
            for step in 0..2000 {
                let k = (rng.next_u32() % space) as u32;
                let op = match rng.below(10) {
                    0..=4 => Op::Put(k),
                    5..=7 => Op::Get(k),
                    _ => Op::Del(k),
                };
                let ctx = format!("bin es={elemsize} ks={keysize} sv={seedvar} step={step} {op:?}");
                match op {
                    Op::Put(k) => {
                        let mut ck = k.to_le_bytes()[..keysize.min(4)].to_vec();
                        ck.resize(keysize, 0);
                        let mut rk = ck.clone();
                        let v: Vec<u8> = (0..elemsize - keysize)
                            .map(|j| (step as u8) ^ (j as u8))
                            .collect();
                        dq(
                            format!("{ctx} / index"),
                            cm.put_binary(&mut ck, &v, HM_BINARY),
                            rm.put_binary(&mut rk, &v, HM_BINARY),
                        );
                    }
                    Op::Get(k) => {
                        let mut ck = k.to_le_bytes()[..keysize.min(4)].to_vec();
                        ck.resize(keysize, 0);
                        let mut rk = ck.clone();
                        dq(
                            format!("{ctx} / index"),
                            cm.geti(ck.as_mut_ptr() as *mut c_void, HM_BINARY),
                            rm.geti(rk.as_mut_ptr() as *mut c_void, HM_BINARY),
                        );
                    }
                    Op::Del(k) => {
                        let mut ck = k.to_le_bytes()[..keysize.min(4)].to_vec();
                        ck.resize(keysize, 0);
                        let mut rk = ck.clone();
                        dq(
                            format!("{ctx} / result"),
                            cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                            rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                        );
                    }
                }
                assert_same_binary(&ctx, &cm, &rm);
            }
            cm.free();
            rm.free();
        }
    }
}

/// Soak variant of row 33 / row 46: the same randomized op stream under MANY
/// different *global* seeds. `stbds_rand_seed` changes the seed every new table
/// gets, which changes every hash, every probe position and therefore the whole
/// bucket layout — so this is a genuinely different code path per seed, not a
/// repeat. The full `stbds_hash_index` (all buckets) is compared after each op.
#[test]
fn cfg_33b_soak_multi_seed() {
    for seedvar in 0..24usize {
        let global = (seedvar as usize).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let p = setup(global);
        let mut rng = Rng::new(0x33_b000 + seedvar as u64);
        let elemsize = 16usize;
        let keysize = 4usize;
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        for step in 0..600 {
            let k = (rng.next_u32() % 120) as u32;
            let ctx = format!("soak seed={global:#x} step={step} k={k}");
            match rng.below(10) {
                0..=4 => {
                    let mut ck = k.to_le_bytes().to_vec();
                    let mut rk = ck.clone();
                    let v: Vec<u8> = (0..elemsize - keysize)
                        .map(|j| (step as u8) ^ (j as u8))
                        .collect();
                    dq(
                        format!("{ctx} / put"),
                        cm.put_binary(&mut ck, &v, HM_BINARY),
                        rm.put_binary(&mut rk, &v, HM_BINARY),
                    );
                }
                5..=7 => {
                    let mut ck = k.to_le_bytes();
                    let mut rk = k.to_le_bytes();
                    dq(
                        format!("{ctx} / get"),
                        cm.geti(ck.as_mut_ptr() as *mut c_void, HM_BINARY),
                        rm.geti(rk.as_mut_ptr() as *mut c_void, HM_BINARY),
                    );
                }
                _ => {
                    let mut ck = k.to_le_bytes();
                    let mut rk = k.to_le_bytes();
                    dq(
                        format!("{ctx} / del"),
                        cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                        rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                    );
                }
            }
            assert_same_binary(&ctx, &cm, &rm);
        }
        // The seed really did differ between runs.
        if let Some(t) = cm.table() {
            assert_eq!(Some(t.seed), rm.table().map(|x| x.seed));
        }
        cm.free();
        rm.free();
    }
}

#[test]
fn cfg_34_35_36_pipeline_string() {
    for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        for seedvar in 0..2usize {
            let p = setup(0x3141_5926 + seedvar * 104_729);
            let mut rng = Rng::new(0x34_0000 + seedvar as u64);
            let elemsize = 16usize;
            let keysize = 8usize;
            let (mut cm, mut rm) = match sh {
                None => (
                    Map::new(&p.c, elemsize, keysize),
                    Map::new(&p.rs, elemsize, keysize),
                ),
                Some(m) => (
                    Map::with_shmode(&p.c, elemsize, keysize, m),
                    Map::with_shmode(&p.rs, elemsize, keysize, m),
                ),
            };
            // Small key space of *strings*.
            let space: Vec<Vec<u8>> = (0..250).map(|i| format!("key_{i}").into_bytes()).collect();
            for step in 0..2000 {
                let k = &space[rng.below(space.len())];
                let which = rng.below(10);
                let ctx = format!(
                    "str sh={sh:?} sv={seedvar} step={step} which={which} key={:?}",
                    String::from_utf8_lossy(k)
                );
                match which {
                    0..=4 => {
                        let v: Vec<u8> = (0..elemsize - keysize)
                            .map(|j| (step as u8) ^ (j as u8))
                            .collect();
                        dq(
                            format!("{ctx} / index"),
                            cm.put_string(leak_cstring(k), &v, HM_STRING),
                            rm.put_string(leak_cstring(k), &v, HM_STRING),
                        );
                    }
                    5..=7 => {
                        dq(
                            format!("{ctx} / index"),
                            cm.geti(leak_cstring(k) as *mut c_void, HM_STRING),
                            rm.geti(leak_cstring(k) as *mut c_void, HM_STRING),
                        );
                    }
                    _ => {
                        dq(
                            format!("{ctx} / result"),
                            cm.del(leak_cstring(k) as *mut c_void, 0, HM_STRING),
                            rm.del(leak_cstring(k) as *mut c_void, 0, HM_STRING),
                        );
                    }
                }
                assert_same_string(&ctx, &cm, &rm);
            }
            cm.free();
            rm.free();
        }
    }
}

// ===========================================================================
// Row 37 — stbds_hmfree_func in every shape
// ===========================================================================

#[test]
fn cfg_37_hmfree_all_shapes() {
    let mut rng = Rng::new(0x37);
    // (a) a == NULL
    unsafe {
        (p_null().c.hmfree_func)(std::ptr::null_mut(), 16);
        (p_null().rs.hmfree_func)(std::ptr::null_mut(), 16);
    }
    // (b) hash_table == NULL (built by hmput_default only)
    for &elemsize in &[4usize, 16] {
        let p = setup(0x3141_5926);
        let ch = unsafe { (p.c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let rh = unsafe { (p.rs.hmput_default)(std::ptr::null_mut(), elemsize) };
        dq(
            "hmfree no-table header",
            unsafe { hash_hdr_snapshot(ch, elemsize) },
            unsafe { hash_hdr_snapshot(rh, elemsize) },
        );
        unsafe {
            (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
            (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
        }
    }
    // (c) every string.mode, populated
    for sh in [None, Some(SH_NONE), Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        let p = setup(0x3141_5926);
        let elemsize = 16usize;
        let keysize = 8usize;
        let (mut cm, mut rm) = match sh {
            None => (
                Map::new(&p.c, elemsize, keysize),
                Map::new(&p.rs, elemsize, keysize),
            ),
            Some(m) => (
                Map::with_shmode(&p.c, elemsize, keysize, m),
                Map::with_shmode(&p.rs, elemsize, keysize, m),
            ),
        };
        let keys = random_string_keys(&mut rng, 120);
        let binary = sh == Some(SH_NONE);
        for (i, k) in keys.iter().enumerate() {
            let v = vec![i as u8; elemsize - keysize];
            if binary {
                let mut ck = k.clone();
                ck.resize(keysize, 0);
                let mut rk = ck.clone();
                cm.put_binary(&mut ck, &v, HM_BINARY);
                rm.put_binary(&mut rk, &v, HM_BINARY);
            } else {
                cm.put_string(leak_cstring(k), &v, HM_STRING);
                rm.put_string(leak_cstring(k), &v, HM_STRING);
            }
        }
        if binary {
            assert_same_binary(&format!("hmfree sh={sh:?}"), &cm, &rm);
        } else {
            assert_same_string(&format!("hmfree sh={sh:?}"), &cm, &rm);
        }
        cm.free();
        rm.free();
    }
}

fn p_null() -> Pair {
    pair()
}

// ===========================================================================
// Row 45 — non-zero keyoffset (only stbds_hmdel_key takes one)
// ===========================================================================

#[test]
fn cfg_45_hmdel_keyoffset() {
    // (a) An element layout where the key bytes are DUPLICATED at offset 0 and
    //     offset 4, so `keyoffset = 4` must behave exactly like `keyoffset = 0`.
    let mut rng = Rng::new(0x45);
    for &n in &[1usize, 2, 8, 60, 250] {
        let p = setup(0x3141_5926);
        let elemsize = 8usize;
        let keysize = 4usize;
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        let mut keys: Vec<u32> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while keys.len() < n {
            let k = rng.next_u32();
            if seen.insert(k) {
                keys.push(k);
            }
        }
        for &k in &keys {
            let mut ck = k.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            // value == key, so the key is duplicated at offset 4
            let v = k.to_le_bytes().to_vec();
            cm.put_binary(&mut ck, &v, HM_BINARY);
            rm.put_binary(&mut rk, &v, HM_BINARY);
        }
        assert_same_binary(&format!("keyoffset build n={n}"), &cm, &rm);
        let mut order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
        for (step, &oi) in order.iter().enumerate() {
            let k = keys[oi];
            let mut ck = k.to_le_bytes();
            let mut rk = k.to_le_bytes();
            let ctx = format!("keyoffset=4 n={n} step={step}");
            dq(
                format!("{ctx} / result"),
                cm.del(ck.as_mut_ptr() as *mut c_void, 4, HM_BINARY),
                rm.del(rk.as_mut_ptr() as *mut c_void, 4, HM_BINARY),
            );
            assert_same_binary(&ctx, &cm, &rm);
        }
        assert_eq!(cm.len(), 0);
        cm.free();
        rm.free();
    }

    // (b) A keyoffset that points at a sentinel field which never equals any
    //     key -> `stbds_hm_find_slot` always returns -1, so both libs must
    //     return `a` with temp == 0 and an unchanged length.
    {
        let p = setup(0x3141_5926);
        let elemsize = 16usize;
        let keysize = 4usize;
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        const SENTINEL: u32 = 0xDEAD_BEEF;
        let keys: Vec<u32> = (0..64u32).map(|i| i * 3 + 1).collect();
        for &k in &keys {
            let mut ck = k.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            let mut v = SENTINEL.to_le_bytes().to_vec();
            v.extend_from_slice(&SENTINEL.to_le_bytes());
            v.extend_from_slice(&SENTINEL.to_le_bytes());
            cm.put_binary(&mut ck, &v, HM_BINARY);
            rm.put_binary(&mut rk, &v, HM_BINARY);
        }
        for &keyoffset in &[4usize, 8, 12] {
            for &k in &keys {
                let mut ck = k.to_le_bytes();
                let mut rk = k.to_le_bytes();
                let ctx = format!("sentinel keyoffset={keyoffset} key={k:#x}");
                dq(
                    format!("{ctx} / result"),
                    cm.del(ck.as_mut_ptr() as *mut c_void, keyoffset, HM_BINARY),
                    rm.del(rk.as_mut_ptr() as *mut c_void, keyoffset, HM_BINARY),
                );
                assert_same_binary(&ctx, &cm, &rm);
                assert_eq!(cm.temp(), 0, "[{ctx}] expected a miss");
            }
        }
        assert_eq!(cm.len(), keys.len() as isize);
        cm.free();
        rm.free();
    }
}

// ===========================================================================
// Cross-cutting fuzz: random (elemsize, keysize, sh_mode, mode) x random ops
// ===========================================================================

/// Randomises the *configuration* as well as the data, so combinations that no
/// hand-written row happens to name still get exercised. Guarded to stay out of
/// the two configurations that abort the C by design (see VERIFICATION.md):
/// `mode >= 2` interior deletes, and strdup/arena tables driven with raw binary
/// keys (which would `strdup`/`stralloc` non-NUL-terminated bytes).
#[test]
fn cfg_fuzz_random_configurations() {
    let mut rng = Rng::new(0xF0F0_1234);
    for case in 0..1200usize {
        let p = setup(rng.next_u64() as usize);

        // Pick a shape.
        let string_keys = rng.next_u64() & 1 == 0;
        let keysize = if string_keys { 8 } else { 1 + rng.below(12) };
        // For string keys the element's first member is a `char *`, so a real
        // `sizeof *(t)` is always a multiple of 8; a non-multiple would make the
        // *C* do unaligned pointer loads, which is not a configuration the
        // library can be handed by a real consumer.
        let elemsize = if string_keys {
            keysize + 8 * rng.below(3)
        } else {
            keysize + rng.below(9)
        };
        // sh_mode: None = "let hmput_key decide", else an explicit shmode_func.
        let sh_choices: &[Option<c_int>] = if string_keys {
            // SH_NONE is excluded: with `string.mode == 0` the `switch`
            // `default:` arm memcpy's the first `keysize` CHARACTERS of the key
            // into the element, and the next `stbds_is_key_equal` with
            // `mode >= 1` then does `strcmp(key, *(char **) elem)` — i.e. the C
            // dereferences those characters as a pointer and faults. A real
            // consumer can only reach it by calling
            // `stbds_shmode_func(es, STBDS_SH_NONE)` and then `stbds_shput`;
            // both libraries fault identically, so there is nothing
            // differential to observe.
            &[None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)]
        } else {
            // Binary keys are raw bytes with no NUL, so SH_STRDUP / SH_ARENA
            // would run strlen off the end of the key — the C would fault too,
            // but there is nothing differential to learn from that.
            &[None, Some(SH_NONE)]
        };
        let sh = sh_choices[rng.below(sh_choices.len())];
        // mode: for binary, anything <= 0; for string, anything >= 1.
        let mode: c_int = if string_keys {
            [1i32, 2, 3, 17, i32::MAX][rng.below(5)]
        } else {
            [0i32, -1, -7, i32::MIN][rng.below(4)]
        };
        // Interior deletes are only safe when mode == 0 (binary) or mode == 1
        // (string); otherwise restrict deletes to the last element.
        let safe_interior_delete = mode == HM_BINARY || mode == HM_STRING || mode < 0;

        let (mut cm, mut rm) = match sh {
            None => (
                Map::new(&p.c, elemsize, keysize),
                Map::new(&p.rs, elemsize, keysize),
            ),
            Some(m) => (
                Map::with_shmode(&p.c, elemsize, keysize, m),
                Map::with_shmode(&p.rs, elemsize, keysize, m),
            ),
        };

        // A small key space so collisions/tombstones/regrowth all happen.
        let space = 3 + rng.below(120);
        let keys: Vec<Vec<u8>> = (0..space)
            .map(|i| {
                if string_keys {
                    format!("f{case}_{i}").into_bytes()
                } else {
                    let mut v = (i as u64).to_le_bytes().to_vec();
                    v.resize(keysize, 0xA5);
                    v
                }
            })
            .collect();

        let nops = 60 + rng.below(240);
        for step in 0..nops {
            let ki = rng.below(keys.len());
            let k = &keys[ki];
            let ctx = format!(
                "fuzz case={case} step={step} es={elemsize} ks={keysize} sh={sh:?} \
                 mode={mode} string={string_keys} key={ki}"
            );
            match rng.below(10) {
                0..=4 => {
                    let v: Vec<u8> = (0..elemsize - keysize)
                        .map(|j| (step as u8) ^ (j as u8) ^ (case as u8))
                        .collect();
                    if string_keys {
                        dq(
                            format!("{ctx} / put"),
                            cm.put_string(leak_cstring(k), &v, mode),
                            rm.put_string(leak_cstring(k), &v, mode),
                        );
                    } else {
                        let mut ck = k.clone();
                        let mut rk = k.clone();
                        dq(
                            format!("{ctx} / put"),
                            cm.put_binary(&mut ck, &v, mode),
                            rm.put_binary(&mut rk, &v, mode),
                        );
                    }
                }
                5..=7 => {
                    let (cp, rp): (*mut c_void, *mut c_void) = if string_keys {
                        (
                            leak_cstring(k) as *mut c_void,
                            leak_cstring(k) as *mut c_void,
                        )
                    } else {
                        (
                            leak_bytes(k) as *mut c_void,
                            leak_bytes(k) as *mut c_void,
                        )
                    };
                    dq(
                        format!("{ctx} / get"),
                        cm.geti(cp, mode),
                        rm.geti(rp, mode),
                    );
                    dq(
                        format!("{ctx} / get_ts"),
                        cm.geti_ts(cp, mode),
                        rm.geti_ts(rp, mode),
                    );
                }
                _ => {
                    // Choose the key to delete so that the operation is safe for
                    // this `mode`.
                    let target: Option<Vec<u8>> = if safe_interior_delete {
                        Some(k.clone())
                    } else if cm.len() > 0 {
                        // the key currently stored in the LAST element
                        let last = cm.hdr().unwrap().length - 1;
                        if string_keys {
                            let kp = unsafe {
                                std::ptr::read_unaligned(cm.elem(last) as *const *mut c_char)
                            };
                            if kp.is_null() { None } else { Some(unsafe { cstr(kp) }) }
                        } else {
                            Some(
                                unsafe { std::slice::from_raw_parts(cm.elem(last), keysize) }
                                    .to_vec(),
                            )
                        }
                    } else {
                        None
                    };
                    if let Some(t) = target {
                        let (cp, rp): (*mut c_void, *mut c_void) = if string_keys {
                            (
                                leak_cstring(&t) as *mut c_void,
                                leak_cstring(&t) as *mut c_void,
                            )
                        } else {
                            (
                                leak_bytes(&t) as *mut c_void,
                                leak_bytes(&t) as *mut c_void,
                            )
                        };
                        dq(
                            format!("{ctx} / del"),
                            cm.del(cp, 0, mode),
                            rm.del(rp, 0, mode),
                        );
                    }
                }
            }
            if string_keys {
                assert_same_string(&ctx, &cm, &rm);
            } else {
                assert_same_binary(&ctx, &cm, &rm);
            }
        }
        cm.free();
        rm.free();
    }
}

/// Leak raw key bytes (no NUL appended) so a pointer stays valid for the whole
/// test even if a `SH_DEFAULT` table stores it.
fn leak_bytes(b: &[u8]) -> *mut u8 {
    Box::leak(b.to_vec().into_boxed_slice()).as_mut_ptr()
}
