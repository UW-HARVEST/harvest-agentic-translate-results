//! Phase B — CONFIGS.md rows 34..42 and 48: string-keyed maps.
//!
//! Driven through `stbds_shmode_func` + the low-level `stbds_hm*_key` entry
//! points, exactly as `sh_new_arena` / `sh_new_strdup` / `shput` / `shget` /
//! `shdel` expand.

mod common;

use common::*;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void};

/// `sizeof (t)->key` for a `char *` key — what all the `sh*` macros pass.
const KEYSIZE: usize = std::mem::size_of::<*mut c_char>();
const KIND: KeyKind = KeyKind::StringPtr { keyoffset: 0 };

/// A NUL-terminated key buffer whose address stays stable for the whole test
/// (required by `STBDS_SH_DEFAULT`, which stores the caller's pointer).
struct Keys(Vec<Box<[u8]>>);

impl Keys {
    fn new() -> Keys {
        Keys(Vec::new())
    }
    fn add(&mut self, s: &[u8]) -> *mut c_char {
        let mut v = s.to_vec();
        v.push(0);
        self.0.push(v.into_boxed_slice());
        self.0.last_mut().unwrap().as_mut_ptr() as *mut c_char
    }
}

fn name(i: usize) -> Vec<u8> {
    format!("key_{i:06}_{}", "x".repeat(i % 37)).into_bytes()
}

fn value_of(v: u64, n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n);
    let mut s = v ^ 0x1357_9BDF_2468_ACE0;
    while out.len() < n {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        out.extend_from_slice(&s.to_le_bytes());
    }
    out.truncate(n);
    out
}

/// Row 34 — `stbds_shmode_func(elemsize, STBDS_SH_NONE)`: the table exists with
/// `string.mode == 0`, so puts take the `default:` (binary `memcpy`) branch.
#[test]
fn cfg_34_shmode_none() {
    let p = Pair::new();
    for &elemsize in &[8usize, 16, 24] {
        for &gseed in &[0usize, 0x3141_5926, usize::MAX] {
            p.seed(gseed);
            let mut m = MapPair::shmode(&p, elemsize, 4, STBDS_HM_BINARY, STBDS_SH_NONE, KeyKind::Binary);
            m.check("shmode(NONE) fresh");
            let s = m.snaps().0;
            assert!(s.has_table);
            assert_eq!(s.length, 1);
            assert_eq!(s.slot_count, 8);
            assert_eq!(s.arena_mode, 0);
            assert_eq!(s.seed, gseed);
            for i in 0..30u64 {
                let mut k = (i as u32).to_le_bytes().to_vec();
                m.put(&p, &mut k, &value_of(i, elemsize.saturating_sub(4)));
                m.check(&format!("shmode(NONE) put {i}"));
            }
            for i in 0..30u64 {
                let mut k = (i as u32).to_le_bytes().to_vec();
                assert!(m.get(&p, &mut k) >= 0);
                m.check("shmode(NONE) get");
            }
            m.free(&p);
        }
    }
}

/// Rows 35/36/37 — the three string-storage modes.
fn string_mode_suite(p: &Pair, sh_mode: c_int, elemsize: usize, gseed: usize, n: usize) {
    p.seed(gseed);
    let mut keys = Keys::new();
    let mut m = MapPair::shmode(p, elemsize, KEYSIZE, STBDS_HM_STRING, sh_mode, KIND);
    m.check("fresh string table");
    let s = m.snaps().0;
    assert_eq!(s.arena_mode, sh_mode as u8);
    assert_eq!(s.seed, gseed);

    let vlen = elemsize - KEYSIZE;
    let mut ptrs = Vec::new();
    for i in 0..n {
        let kb = name(i);
        let kp = keys.add(&kb);
        ptrs.push(kp);
        let before = m.length();
        m.put_str(p, kp, &value_of(i as u64, vlen));
        m.check(&format!("sh_mode={sh_mode} put {i}"));
        if m.length() > before {
            m.check_temp_key(&format!("sh_mode={sh_mode} temp_key after insert {i}"));
        }
    }
    // every key found, with the right stored string
    for (i, &kp) in ptrs.iter().enumerate() {
        let t = m.get_str(p, kp);
        // `stbds_hmput_key` sets temp = (old raw length) - 1, and element
        // `temp` on the hash side is raw element `temp + 1`.
        assert_eq!(t, i as isize, "string get index");
        m.check("string get");
        let (sc, _) = m.snaps();
        assert_eq!(
            sc.keys[(t + 1) as usize].as_deref(),
            Some(name(i).as_slice()),
            "stored key string"
        );
    }
    // misses
    for i in n..n + 16 {
        let kb = name(i + 100_000);
        let kp = keys.add(&kb);
        assert_eq!(m.get_str(p, kp), -1, "string miss");
        m.check("string miss");
    }
    m.free(p);
}

/// Row 35 — `STBDS_SH_DEFAULT`: the caller's pointer is stored verbatim.
#[test]
fn cfg_35_sh_default() {
    let p = Pair::new();
    for &elemsize in &[16usize, 24] {
        for &gseed in &[0usize, 0x3141_5926, usize::MAX] {
            for &n in &[0usize, 1, 6, 7, 13, 60] {
                string_mode_suite(&p, STBDS_SH_DEFAULT, elemsize, gseed, n);
            }
        }
    }
}

/// Row 36 — `STBDS_SH_STRDUP`: every key is `strdup`'d and freed by
/// `stbds_hmfree_func`.
#[test]
fn cfg_36_sh_strdup() {
    let p = Pair::new();
    for &elemsize in &[16usize, 24] {
        for &gseed in &[0usize, 0x3141_5926, usize::MAX] {
            for &n in &[0usize, 1, 6, 7, 13, 60] {
                string_mode_suite(&p, STBDS_SH_STRDUP, elemsize, gseed, n);
            }
        }
    }
}

/// Row 37 — `STBDS_SH_ARENA`: keys come from the table's own arena; the arena
/// bookkeeping (`remaining`, `block`, block count) must evolve identically.
#[test]
fn cfg_37_sh_arena() {
    let p = Pair::new();
    for &elemsize in &[16usize, 24] {
        for &gseed in &[0usize, 0x3141_5926, usize::MAX] {
            for &n in &[0usize, 1, 6, 7, 13, 60] {
                string_mode_suite(&p, STBDS_SH_ARENA, elemsize, gseed, n);
            }
        }
    }
    // enough keys to force several arena blocks
    p.seed(0xABCD);
    let mut keys = Keys::new();
    let mut m = MapPair::shmode(&p, 16, KEYSIZE, STBDS_HM_STRING, STBDS_SH_ARENA, KIND);
    let mut max_block = 0u8;
    for i in 0..600usize {
        let kb = format!("{}_{}", "a".repeat(50 + i % 90), i).into_bytes();
        let kp = keys.add(&kb);
        m.put_str(&p, kp, &value_of(i as u64, 8));
        m.check(&format!("arena growth put {i}"));
        let s = m.snaps().0;
        max_block = max_block.max(s.arena_block);
    }
    assert!(max_block >= 2, "expected arena block growth, got {max_block}");
    let s = m.snaps().0;
    assert!(s.arena_block_count > 1, "expected several arena blocks");
    m.free(&p);
}

/// Row 38 — string map bootstrapped straight from a NULL handle: the first
/// `stbds_hmput_key(_, _, _, _, 1)` sets `string.mode = STBDS_SH_DEFAULT`.
#[test]
fn cfg_38_string_map_from_null() {
    let p = Pair::new();
    for &gseed in &[0usize, 1, 0x3141_5926] {
        p.seed(gseed);
        let mut keys = Keys::new();
        let mut m = MapPair::null(16, KEYSIZE, STBDS_HM_STRING, KIND);
        for i in 0..40usize {
            let kb = name(i);
            let kp = keys.add(&kb);
            let before = m.length();
            m.put_str(&p, kp, &value_of(i as u64, 8));
            m.check(&format!("string map from NULL put {i}"));
            if i == 0 {
                let s = m.snaps().0;
                assert_eq!(
                    s.arena_mode, STBDS_SH_DEFAULT as u8,
                    "first string put must set STBDS_SH_DEFAULT"
                );
            }
            if m.length() > before {
                m.check_temp_key("from-NULL temp_key");
            }
        }
        for i in 0..40usize {
            let kb = name(i);
            let kp = keys.add(&kb);
            assert!(m.get_str(&p, kp) >= 0);
            m.check("from-NULL get");
        }
        m.free(&p);
    }
}

/// Row 39 — out-of-range `mode` above `STBDS_HM_STRING` behaves like `mode == 1`
/// for put/get.
#[test]
fn cfg_39_mode_above_string() {
    let p = Pair::new();
    let mut traces: Vec<(c_int, Vec<Snap>)> = Vec::new();
    for &mode in &[1 as c_int, 2, 3, 1000, c_int::MAX] {
        p.seed(0x2468_ACE0);
        let mut keys = Keys::new();
        let mut m = MapPair::shmode(&p, 16, KEYSIZE, mode, STBDS_SH_STRDUP, KIND);
        let mut trace = Vec::new();
        for i in 0..40usize {
            let kb = name(i);
            let kp = keys.add(&kb);
            m.put_str(&p, kp, &value_of(i as u64, 8));
            m.check(&format!("mode={mode} put {i}"));
            trace.push(m.snaps().0);
        }
        for i in 0..40usize {
            let kb = name(i);
            let kp = keys.add(&kb);
            assert!(m.get_str(&p, kp) >= 0, "mode={mode} get");
            m.check(&format!("mode={mode} get {i}"));
            trace.push(m.snaps().0);
        }
        traces.push((mode, trace));
        m.free(&p);
    }
    let (_, base) = &traces[0];
    for (mode, tr) in traces.iter().skip(1) {
        assert_eq!(base.len(), tr.len());
        for (i, (a, b)) in base.iter().zip(tr.iter()).enumerate() {
            assert_eq!(a, b, "mode={mode} step {i} differs from mode=1");
        }
    }
}

/// Row 40 — deletes on string maps in each storage mode, both delete-last and
/// delete-middle (memmove + string re-index).
#[test]
fn cfg_40_string_delete() {
    let p = Pair::new();
    for &sh_mode in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for &n in &[1usize, 2, 7, 13, 40] {
            // ---- delete last (no memmove) ----
            p.seed(0x1111_9999);
            let mut keys = Keys::new();
            let mut m = MapPair::shmode(&p, 16, KEYSIZE, STBDS_HM_STRING, sh_mode, KIND);
            let mut ptrs = Vec::new();
            for i in 0..n {
                let kb = name(i);
                let kp = keys.add(&kb);
                ptrs.push(kp);
                m.put_str(&p, kp, &value_of(i as u64, 8));
            }
            for (i, &kp) in ptrs.iter().enumerate().rev() {
                assert_eq!(m.del_str(&p, kp, 0), 1, "sh={sh_mode} del last {i}");
                m.check(&format!("sh={sh_mode} n={n} del last {i}"));
            }
            assert_eq!(m.snaps().0.length, 1);
            m.free(&p);

            // ---- delete first (memmove + re-index) ----
            p.seed(0x1111_9999);
            let mut keys = Keys::new();
            let mut m = MapPair::shmode(&p, 16, KEYSIZE, STBDS_HM_STRING, sh_mode, KIND);
            let mut ptrs = Vec::new();
            for i in 0..n {
                let kb = name(i);
                let kp = keys.add(&kb);
                ptrs.push(kp);
                m.put_str(&p, kp, &value_of(i as u64, 8));
            }
            let mut live = ptrs.clone();
            while !live.is_empty() {
                let kp = live.remove(0);
                assert_eq!(m.del_str(&p, kp, 0), 1, "sh={sh_mode} del first");
                m.check(&format!("sh={sh_mode} n={n} del first, {} live", live.len()));
                for &q in &live {
                    assert!(m.get_str(&p, q) >= 0, "survivor must remain findable");
                    m.check("survivor get");
                }
                // repeat delete of the same key is a no-op
                // (for STRDUP the stored copy is gone, so probe with the
                // caller's buffer, which is still alive)
                assert_eq!(m.del_str(&p, kp, 0), 0, "repeat delete");
                m.check("repeat delete");
            }
            m.free(&p);
        }
    }
}

/// Row 41 — duplicate-key puts on a string map: the found path sets `temp_key`
/// from the *stored* pointer and allocates nothing new.
#[test]
fn cfg_41_string_duplicate_puts() {
    let p = Pair::new();
    for &sh_mode in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        p.seed(0x3333_7777);
        let mut keys = Keys::new();
        let mut m = MapPair::shmode(&p, 16, KEYSIZE, STBDS_HM_STRING, sh_mode, KIND);
        let mut ptrs = Vec::new();
        for i in 0..25usize {
            let kb = name(i);
            let kp = keys.add(&kb);
            ptrs.push(kp);
            m.put_str(&p, kp, &value_of(i as u64, 8));
        }
        let before = m.snaps().0;
        for round in 0..4u64 {
            for i in 0..ptrs.len() {
                // a *different* buffer with the same contents, so the found
                // path is exercised through strcmp rather than pointer equality
                let kb = name(i);
                let dup = keys.add(&kb);
                let t = m.put_str(&p, dup, &value_of(round * 1000 + i as u64, 8));
                assert_eq!(t, i as isize, "duplicate put must reuse the slot");
                m.check(&format!("sh={sh_mode} dup put round {round} key {i}"));
            }
            let now = m.snaps().0;
            assert_eq!(now.length, before.length, "no growth on duplicate puts");
            assert_eq!(now.used_count, before.used_count);
            assert_eq!(
                now.arena_remaining, before.arena_remaining,
                "duplicate puts must not consume arena space"
            );
        }
        m.free(&p);
    }
}

/// Row 42 — randomized op-stream per string-storage mode.
#[test]
fn cfg_42_string_op_stream() {
    let p = Pair::new();
    for &sh_mode in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for &gseed in &[0usize, 0x3141_5926, usize::MAX] {
            let mut rng = Rng::new(0xC0FFEE_42 ^ sh_mode as u64 ^ gseed as u64);
            p.seed(gseed);
            let mut keys = Keys::new();
            let mut m = MapPair::shmode(&p, 24, KEYSIZE, STBDS_HM_STRING, sh_mode, KIND);
            // small pool of short, similar strings -> plenty of collisions
            let pool: Vec<Vec<u8>> = (0..50)
                .map(|i| format!("k{}{}", i % 17, "z".repeat(i % 11)).into_bytes())
                .collect();
            let mut model: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
            for step in 0..800u64 {
                let ki = rng.below(pool.len());
                let kb = pool[ki].clone();
                let kp = keys.add(&kb);
                match rng.below(10) {
                    0..=4 => {
                        let val = value_of(step, 16);
                        m.put_str(&p, kp, &val);
                        model.insert(kb.clone(), val);
                    }
                    5..=7 => {
                        let t = m.get_str(&p, kp);
                        assert_eq!(
                            t >= 0,
                            model.contains_key(&kb),
                            "string hit/miss disagrees with the model at step {step}"
                        );
                    }
                    _ => {
                        let t = m.del_str(&p, kp, 0);
                        assert_eq!(
                            t,
                            model.remove(&kb).is_some() as isize,
                            "string delete disagrees with the model at step {step}"
                        );
                    }
                }
                m.check(&format!("string op-stream sh={sh_mode} step {step}"));
            }
            for kb in pool.iter() {
                let kp = keys.add(kb);
                let t = m.get_str(&p, kp);
                assert_eq!(t >= 0, model.contains_key(kb));
                m.check("string final sweep");
            }
            m.free(&p);
        }
    }
}

/// Row 48 — `stbds_hmfree_func` over every table flavour.
#[test]
fn cfg_48_hmfree_all_flavours() {
    let p = Pair::new();

    // (a) binary map
    p.seed(7);
    let mut m = MapPair::null(8, 4, STBDS_HM_BINARY, KeyKind::Binary);
    for i in 0..20u32 {
        let mut k = i.to_le_bytes().to_vec();
        m.put(&p, &mut k, &i.to_le_bytes());
    }
    m.check("binary before free");
    m.free(&p);
    assert!(m.hc.is_null() && m.hr.is_null());

    // (b) each string-storage mode
    for &sh_mode in &[STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        p.seed(11);
        let mut keys = Keys::new();
        let mode = if sh_mode == STBDS_SH_NONE {
            STBDS_HM_BINARY
        } else {
            STBDS_HM_STRING
        };
        let kind = if sh_mode == STBDS_SH_NONE {
            KeyKind::Binary
        } else {
            KIND
        };
        let ks = if sh_mode == STBDS_SH_NONE { 4 } else { KEYSIZE };
        let mut m = MapPair::shmode(&p, 16, ks, mode, sh_mode, kind);
        for i in 0..30usize {
            if sh_mode == STBDS_SH_NONE {
                let mut k = (i as u32).to_le_bytes().to_vec();
                m.put(&p, &mut k, &value_of(i as u64, 12));
            } else {
                let kb = name(i);
                let kp = keys.add(&kb);
                m.put_str(&p, kp, &value_of(i as u64, 8));
            }
        }
        m.check(&format!("sh={sh_mode} before free"));
        m.free(&p);
    }

    // (c) table-less map from stbds_hmput_default
    let (hc, hr) = unsafe {
        (
            (p.c.hmput_default)(std::ptr::null_mut(), 16),
            (p.r.hmput_default)(std::ptr::null_mut(), 16),
        )
    };
    unsafe {
        (p.c.hmfree_func)((hc as *mut u8).sub(16) as *mut c_void, 16);
        (p.r.hmfree_func)((hr as *mut u8).sub(16) as *mut c_void, 16);
    }
}

/// Find `n` distinct key strings whose probe position is exactly `pos`.
fn strings_with_pos(p: &Pair, seed: usize, slot_count: usize, pos: usize, n: usize) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut i = 0u64;
    while out.len() < n {
        i += 1;
        assert!(i < 5_000_000, "no key string found with pos={pos}");
        let mut kb = format!("pk{i}").into_bytes();
        kb.push(0);
        let mut h = unsafe { (p.c.hash_string)(kb.as_mut_ptr() as *mut c_char, seed) };
        if h < 2 {
            h += 2;
        }
        if h & (slot_count - 1) == pos {
            kb.pop(); // drop the NUL, `Keys::add` re-appends it
            if !out.contains(&kb) {
                out.push(kb);
            }
        }
    }
    out
}

/// Row 41 (continued) — the C's forward-scan / wrap-scan **asymmetry** in
/// `stbds_hmput_key`'s "key already present" path:
///
/// * forward scan (C L729-735) sets `stbds_temp` **and**, for string modes,
///   `stbds_temp_key`;
/// * wrap-around scan (C L747-751) sets **only** `stbds_temp` — `temp_key` keeps
///   whatever it held before.
///
/// Constructed by giving two keys the same initial probe position 7: the first
/// lands in slot 7, the second spills to slot 0 via the wrap scan, so a
/// duplicate put of the *second* key is found by the wrap scan and must leave
/// `temp_key` pointing at the *first* key's stored string.
#[test]
fn cfg_41b_temp_key_scan_asymmetry() {
    let p = Pair::new();
    let mut hit_wrap = 0usize;
    for &gseed in &[0usize, 1, 7, 0x3141_5926, usize::MAX] {
        for &sh_mode in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            p.seed(gseed);
            let mut keys = Keys::new();
            let mut m = MapPair::shmode(&p, 16, KEYSIZE, STBDS_HM_STRING, sh_mode, KIND);
            let seed = m.snaps().0.seed;

            // two keys that both probe to slot 7 of the 8-slot table
            let ks = strings_with_pos(&p, seed, 8, 7, 2);
            let (ka, kb) = (ks[0].clone(), ks[1].clone());

            let pa = keys.add(&ka);
            m.put_str(&p, pa, &1u64.to_le_bytes());
            m.check("A insert");
            m.check_temp_key("temp_key after A insert");

            let pb = keys.add(&kb);
            m.put_str(&p, pb, &2u64.to_le_bytes());
            m.check("B insert");
            m.check_temp_key("temp_key after B insert");

            let s = m.snaps().0;
            assert_eq!(s.used_count, 2);
            // A occupies slot 7, B spilled to slot 0 through the wrap scan
            assert!(s.buckets[0].1[7] >= 0, "slot 7 must hold A: {:?}", s.buckets);
            assert!(s.buckets[0].1[0] >= 0, "slot 0 must hold B: {:?}", s.buckets);

            // duplicate put of A -> forward scan -> temp_key := A's stored string
            let dup_a = keys.add(&ka);
            assert_eq!(m.put_str(&p, dup_a, &11u64.to_le_bytes()), 0);
            m.check("A duplicate put");
            m.check_temp_key("temp_key after A duplicate (forward scan)");
            let tk_after_a = temp_key_of(&m);
            assert_eq!(
                tk_after_a.as_deref(),
                Some(ka.as_slice()),
                "the forward-scan branch must set temp_key to A"
            );

            // duplicate put of B -> wrap scan -> temp_key must be UNCHANGED (A)
            let dup_b = keys.add(&kb);
            assert_eq!(m.put_str(&p, dup_b, &22u64.to_le_bytes()), 1);
            m.check("B duplicate put");
            m.check_temp_key("temp_key after B duplicate (wrap scan)");
            let tk_after_b = temp_key_of(&m);
            assert_eq!(
                tk_after_b.as_deref(),
                Some(ka.as_slice()),
                "the wrap-around-scan branch must NOT touch temp_key (it must \
                 still point at A, not B)"
            );
            hit_wrap += 1;

            m.free(&p);
        }
    }
    assert!(hit_wrap > 0, "the wrap-scan found-existing branch was never taken");
}

/// `table->temp_key` as a C string (valid only right after a put that wrote it).
fn temp_key_of(m: &MapPair) -> Option<Vec<u8>> {
    unsafe {
        let hdr = ((m.hc as *mut u8).sub(m.elemsize) as *mut ArrayHeader).sub(1);
        let t = (*hdr).hash_table as *mut HashIndex;
        if t.is_null() || (*t).temp_key.is_null() {
            None
        } else {
            Some(cstr_bytes((*t).temp_key))
        }
    }
}

/// Row 53 — the `mode` × `table->string.mode` cross-product cell that neither a
/// pure binary nor a pure string test reaches: a table created by
/// `stbds_shmode_func(e, STBDS_SH_DEFAULT/STRDUP/ARENA)` (i.e. `sh_new_arena` /
/// `sh_new_strdup`) but then driven with **binary** `mode = 0` calls, as a
/// caller mixing `hmput` with `sh_new_*` would do.
///
/// The C keeps the two knobs completely independent: hashing/comparison follow
/// `mode` (siphash + `memcmp`), while key *storage* follows
/// `table->string.mode` (pointer / `stbds_strdup` / `stbds_stralloc`). So the
/// stored key is a `char *` while `stbds_is_key_equal` memcmps the raw key bytes
/// against that pointer — which never matches, so every put inserts a fresh
/// element and every lookup misses. All of that must match exactly.
#[test]
fn cfg_53_binary_mode_on_string_storage_table() {
    let p = Pair::new();
    for &sh_mode in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for &gseed in &[0usize, 0x3141_5926, usize::MAX] {
            p.seed(gseed);
            let mut keys = Keys::new();
            // elemsize 16 = char* key + u64 value; keysize 8 = sizeof(char*)
            let mut m = MapPair::shmode(&p, 16, KEYSIZE, STBDS_HM_BINARY, sh_mode, KIND);
            let s = m.snaps().0;
            assert_eq!(s.arena_mode, sh_mode as u8);

            for i in 0..30usize {
                // 8-byte keys with an embedded NUL so `strdup`/`stralloc` see a
                // well-formed C string while `memcmp` sees 8 bytes.
                let kb = format!("k{i:04}").into_bytes(); // 5 bytes + NUL from Keys::add
                let kp = keys.add(&kb);
                let before = m.length();
                m.put_str(&p, kp, &(i as u64).to_le_bytes());
                m.check(&format!("sh={sh_mode} binary put {i}"));
                assert_eq!(
                    m.length(),
                    before + 1,
                    "a binary memcmp against a stored char* never matches, so \
                     every put must insert"
                );
                m.check_temp_key(&format!("sh={sh_mode} temp_key after insert {i}"));
            }
            // and every lookup misses, for the same reason
            for i in 0..30usize {
                let kb = format!("k{i:04}").into_bytes();
                let kp = keys.add(&kb);
                assert_eq!(
                    m.get_str(&p, kp),
                    -1,
                    "sh={sh_mode} binary lookup of a char*-stored key must miss"
                );
                m.check(&format!("sh={sh_mode} binary get {i}"));
                assert_eq!(m.del_str(&p, kp, 0), 0, "sh={sh_mode} binary delete must miss");
                m.check(&format!("sh={sh_mode} binary del {i}"));
            }
            m.free(&p);
        }
    }
}
