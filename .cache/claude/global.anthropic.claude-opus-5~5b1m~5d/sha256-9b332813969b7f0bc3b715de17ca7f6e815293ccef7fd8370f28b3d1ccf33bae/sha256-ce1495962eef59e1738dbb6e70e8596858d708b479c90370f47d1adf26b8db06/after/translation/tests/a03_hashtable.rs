//! Differential tests for src/hashtable.c and src/hashtable_seed.c.
//!
//! The hashtable is the single most order-sensitive part of the library: it
//! decides object iteration order, which in turn decides the byte layout of
//! every `json_dumps` of an object. That order depends on
//!   * `hashlittle()` (lookup3.h) producing the same 32-bit hash, and
//!   * the bucket/insert/rehash bookkeeping producing the same linked lists.
//! So these tests compare full iteration sequences, not just membership.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

/// A caller-allocated `hashtable_t`, initialised through the library under
/// test and closed on drop.
struct Ht<'a> {
    api: &'a Api,
    t: Box<hashtable_t>,
}

impl<'a> Ht<'a> {
    unsafe fn new(api: &'a Api) -> (Ht<'a>, std::ffi::c_int) {
        let mut t = Box::new(hashtable_t::zeroed());
        let ret = (api.hashtable_init)(&mut *t);
        (Ht { api, t }, ret)
    }

    fn p(&mut self) -> *mut hashtable_t {
        &mut *self.t
    }

    unsafe fn set(&mut self, key: &[u8], value: *mut json_t) -> std::ffi::c_int {
        let api = self.api;
        (api.hashtable_set)(self.p(), key.as_ptr() as *const c_char, key.len(), value)
    }

    unsafe fn get(&mut self, key: &[u8]) -> *mut c_void {
        let api = self.api;
        (api.hashtable_get)(self.p(), key.as_ptr() as *const c_char, key.len())
    }

    unsafe fn del(&mut self, key: &[u8]) -> std::ffi::c_int {
        let api = self.api;
        (api.hashtable_del)(self.p(), key.as_ptr() as *const c_char, key.len())
    }

    /// The full ordered iteration: (key bytes, key_len, integer value).
    /// Values are always `json_integer`s in these tests so the value can be
    /// compared as a plain number.
    unsafe fn iterate(&mut self) -> Vec<(Vec<u8>, size_t, i64)> {
        let api = self.api;
        let mut out = Vec::new();
        let mut it = (api.hashtable_iter)(self.p());
        while !it.is_null() {
            let kp = (api.hashtable_iter_key)(it) as *const c_char;
            let klen = (api.hashtable_iter_key_len)(it);
            let key: Vec<u8> = (0..klen).map(|i| *(kp as *const u8).add(i)).collect();
            let v = (api.hashtable_iter_value)(it) as *mut json_t;
            let n = if v.is_null() { i64::MIN } else { (api.json_integer_value)(v) };
            out.push((key, klen, n));
            it = (api.hashtable_iter_next)(self.p(), it);
        }
        out
    }

    /// `size` and `order` are part of the observable state: `order` proves the
    /// rehash happened at the same moment in both implementations.
    fn state(&self) -> (size_t, size_t) {
        (self.t.size, self.t.order)
    }
}

impl Drop for Ht<'_> {
    fn drop(&mut self) {
        unsafe {
            let api = self.api;
            (api.hashtable_close)(&mut *self.t);
        }
    }
}

#[test]
fn seed_is_installed_identically_in_both_libraries() {
    let _g = global_state_lock();
    // Everything else in this file depends on this: json_object_seed() is
    // one-shot (it only acts while hashtable_seed == 0), and `both()` calls it
    // with FIXED_SEED before anything else touches a hashtable.
    let (c, r) = both();
    assert_eq!(
        c.hashtable_seed(),
        FIXED_SEED as u32,
        "C: json_object_seed did not install the fixed seed"
    );
    diff_eq!(c.hashtable_seed(), r.hashtable_seed(), "hashtable_seed value");
}

#[test]
fn json_object_seed_is_one_shot() {
    let _g = global_state_lock();
    // A second call with a different value must be ignored, because the C only
    // seeds `if (hashtable_seed == 0)`.
    let (c, r) = both();
    unsafe {
        (c.json_object_seed)(0xAAAA_BBBB);
        (r.json_object_seed)(0xAAAA_BBBB);
    }
    diff_eq!(c.hashtable_seed(), r.hashtable_seed(), "seed after second call");
    assert_eq!(c.hashtable_seed(), FIXED_SEED as u32, "C: seed must not change");
}

#[test]
fn hashtable_init_fresh_state() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (ch, cret) = Ht::new(c);
        let (rh, rret) = Ht::new(r);
        diff_eq!(cret, rret, "hashtable_init return");
        // INITIAL_HASHTABLE_ORDER is 3 => 8 buckets.
        diff_eq!(ch.state(), rh.state(), "state after init");
        assert_eq!(ch.state(), (0, 3), "C: fresh table is size 0, order 3");
    }
}

#[test]
fn hashtable_iter_on_empty_table_is_null() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        diff_eq!(
            (c.hashtable_iter)(ch.p()).is_null(),
            (r.hashtable_iter)(rh.p()).is_null(),
            "hashtable_iter on empty table"
        );
        diff_eq!(ch.iterate(), rh.iterate(), "iteration of empty table");
    }
}

#[test]
fn hashtable_get_and_del_on_absent_key() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        ch.set(b"present", (c.json_integer)(1));
        rh.set(b"present", (r.json_integer)(1));

        for key in [&b""[..], b"absent", b"presen", b"presentt", b"PRESENT"] {
            diff_eq!(
                ch.get(key).is_null(),
                rh.get(key).is_null(),
                "hashtable_get({:?}) on absent key",
                String::from_utf8_lossy(key)
            );
            // del must report -1 for a key that is not there.
            diff_eq!(
                ch.del(key),
                rh.del(key),
                "hashtable_del({:?}) on absent key",
                String::from_utf8_lossy(key)
            );
        }
        diff_eq!(ch.state(), rh.state(), "state unchanged by failed ops");
    }
}

#[test]
fn hashtable_growth_across_rehash_thresholds() {
    let _g = global_state_lock();
    // The grow test is `size >= hashsize(order)`, i.e. it rehashes when the
    // load factor reaches 1: at 8, then 16, then 32, then 64 entries. Insert
    // well past several thresholds and compare the FULL iteration order after
    // every single insertion, so the exact moment and result of each rehash is
    // checked.
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        for i in 0..300usize {
            let key = format!("key{i:04}");
            diff_eq!(
                ch.set(key.as_bytes(), (c.json_integer)(i as i64)),
                rh.set(key.as_bytes(), (r.json_integer)(i as i64)),
                "hashtable_set #{i} return"
            );
            diff_eq!(ch.state(), rh.state(), "size/order after insert #{i}");
            diff_eq!(ch.iterate(), rh.iterate(), "iteration order after insert #{i}");
        }
    }
}

#[test]
fn hashtable_replace_existing_key_keeps_position() {
    let _g = global_state_lock();
    // Replacing a value takes the `if (pair)` branch: the value is swapped in
    // place and the pair keeps its position in the ordered list (and `size`
    // must not change).
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        for i in 0..20 {
            let k = format!("k{i}");
            ch.set(k.as_bytes(), (c.json_integer)(i));
            rh.set(k.as_bytes(), (r.json_integer)(i));
        }
        for i in (0..20).rev() {
            let k = format!("k{i}");
            diff_eq!(
                ch.set(k.as_bytes(), (c.json_integer)(1000 + i)),
                rh.set(k.as_bytes(), (r.json_integer)(1000 + i)),
                "replace k{i} return"
            );
            diff_eq!(ch.state(), rh.state(), "size must not grow on replace of k{i}");
            diff_eq!(ch.iterate(), rh.iterate(), "order preserved on replace of k{i}");
        }
    }
}

#[test]
fn hashtable_delete_first_middle_last_and_only() {
    let _g = global_state_lock();
    // hashtable_do_del has four distinct bucket-fixup branches:
    //   only element in bucket / first in bucket / last in bucket / middle.
    // Deleting at many different positions over many table sizes reaches them.
    let (c, r) = both();
    unsafe {
        for n in [1usize, 2, 3, 8, 9, 17, 40] {
            for del_idx in 0..n {
                let (mut ch, _) = Ht::new(c);
                let (mut rh, _) = Ht::new(r);
                for i in 0..n {
                    let k = format!("k{i:03}");
                    ch.set(k.as_bytes(), (c.json_integer)(i as i64));
                    rh.set(k.as_bytes(), (r.json_integer)(i as i64));
                }
                let k = format!("k{del_idx:03}");
                diff_eq!(
                    ch.del(k.as_bytes()),
                    rh.del(k.as_bytes()),
                    "del {k} from table of {n} return"
                );
                diff_eq!(ch.state(), rh.state(), "state after del {k} from {n}");
                diff_eq!(
                    ch.iterate(),
                    rh.iterate(),
                    "iteration after del {k} from table of {n}"
                );
            }
        }
    }
}

#[test]
fn hashtable_delete_every_element_in_every_order() {
    let _g = global_state_lock();
    // Drains the table completely, comparing after each removal — this reaches
    // the "bucket becomes empty" fixup repeatedly and confirms the ordered list
    // is unlinked identically.
    let (c, r) = both();
    let mut rng = Rng::new(0x47_0001);
    unsafe {
        for trial in 0..40 {
            let n = 1 + rng.below(30);
            let (mut ch, _) = Ht::new(c);
            let (mut rh, _) = Ht::new(r);
            let mut keys: Vec<String> = (0..n).map(|i| format!("key-{i}")).collect();
            for (i, k) in keys.iter().enumerate() {
                ch.set(k.as_bytes(), (c.json_integer)(i as i64));
                rh.set(k.as_bytes(), (r.json_integer)(i as i64));
            }
            // Shuffle the deletion order deterministically.
            for i in (1..keys.len()).rev() {
                let j = rng.below(i + 1);
                keys.swap(i, j);
            }
            for k in &keys {
                diff_eq!(
                    ch.del(k.as_bytes()),
                    rh.del(k.as_bytes()),
                    "trial {trial}: del {k}"
                );
                diff_eq!(ch.state(), rh.state(), "trial {trial}: state after del {k}");
                diff_eq!(ch.iterate(), rh.iterate(), "trial {trial}: order after del {k}");
            }
        }
    }
}

#[test]
fn hashtable_clear_then_reuse() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        // Grow past a rehash first so clear() has to reset a bigger bucket array.
        for i in 0..40 {
            let k = format!("a{i}");
            ch.set(k.as_bytes(), (c.json_integer)(i));
            rh.set(k.as_bytes(), (r.json_integer)(i));
        }
        (c.hashtable_clear)(ch.p());
        (r.hashtable_clear)(rh.p());
        // clear() resets size to 0 but deliberately KEEPS the grown order.
        diff_eq!(ch.state(), rh.state(), "state after clear");
        diff_eq!(ch.iterate(), rh.iterate(), "iteration after clear");

        // The table must be fully usable again afterwards.
        for i in 0..25 {
            let k = format!("b{i}");
            diff_eq!(
                ch.set(k.as_bytes(), (c.json_integer)(100 + i)),
                rh.set(k.as_bytes(), (r.json_integer)(100 + i)),
                "reuse after clear, insert {i}"
            );
            diff_eq!(ch.state(), rh.state(), "state on reuse insert {i}");
            diff_eq!(ch.iterate(), rh.iterate(), "order on reuse insert {i}");
        }
        // Clearing an already-empty table is also valid.
        (c.hashtable_clear)(ch.p());
        (r.hashtable_clear)(rh.p());
        (c.hashtable_clear)(ch.p());
        (r.hashtable_clear)(rh.p());
        diff_eq!(ch.state(), rh.state(), "state after double clear");
    }
}

#[test]
fn hashtable_iter_at_finds_the_right_pair() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        let n = 50;
        for i in 0..n {
            let k = format!("k{i:03}");
            ch.set(k.as_bytes(), (c.json_integer)(i));
            rh.set(k.as_bytes(), (r.json_integer)(i));
        }
        for i in 0..n {
            let k = format!("k{i:03}");
            let cit = (c.hashtable_iter_at)(ch.p(), k.as_ptr() as *const c_char, k.len());
            let rit = (r.hashtable_iter_at)(rh.p(), k.as_ptr() as *const c_char, k.len());
            diff_eq!(cit.is_null(), rit.is_null(), "iter_at({k}) null-ness");
            assert!(!cit.is_null());
            // Compare the pair the iterator points at, plus the whole tail of
            // the iteration from that point on.
            let ckl = (c.hashtable_iter_key_len)(cit);
            let rkl = (r.hashtable_iter_key_len)(rit);
            diff_eq!(ckl, rkl, "iter_at({k}) key_len");
            let ckp = (c.hashtable_iter_key)(cit) as *const u8;
            let rkp = (r.hashtable_iter_key)(rit) as *const u8;
            let ck: Vec<u8> = (0..ckl).map(|j| *ckp.add(j)).collect();
            let rk: Vec<u8> = (0..rkl).map(|j| *rkp.add(j)).collect();
            diff_eq!(ck, rk, "iter_at({k}) key bytes");
            diff_eq!(
                (c.json_integer_value)((c.hashtable_iter_value)(cit) as *mut json_t),
                (r.json_integer_value)((r.hashtable_iter_value)(rit) as *mut json_t),
                "iter_at({k}) value"
            );

            // Walk the rest of the sequence from here.
            let mut cseq = Vec::new();
            let mut rseq = Vec::new();
            let mut ci = cit;
            let mut ri = rit;
            while !ci.is_null() {
                cseq.push((c.hashtable_iter_key_len)(ci));
                ci = (c.hashtable_iter_next)(ch.p(), ci);
            }
            while !ri.is_null() {
                rseq.push((r.hashtable_iter_key_len)(ri));
                ri = (r.hashtable_iter_next)(rh.p(), ri);
            }
            diff_eq!(cseq, rseq, "tail of iteration from iter_at({k})");
        }
        // iter_at on an absent key returns NULL.
        let missing = b"nope";
        diff_eq!(
            (c.hashtable_iter_at)(ch.p(), missing.as_ptr() as *const c_char, 4).is_null(),
            (r.hashtable_iter_at)(rh.p(), missing.as_ptr() as *const c_char, 4).is_null(),
            "iter_at on absent key"
        );
    }
}

#[test]
fn hashtable_iter_set_replaces_value_in_place() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        for i in 0..15 {
            let k = format!("k{i}");
            ch.set(k.as_bytes(), (c.json_integer)(i));
            rh.set(k.as_bytes(), (r.json_integer)(i));
        }
        let mut cit = (c.hashtable_iter)(ch.p());
        let mut rit = (r.hashtable_iter)(rh.p());
        let mut n = 0;
        while !cit.is_null() {
            (c.hashtable_iter_set)(cit, (c.json_integer)(9000 + n));
            (r.hashtable_iter_set)(rit, (r.json_integer)(9000 + n));
            n += 1;
            cit = (c.hashtable_iter_next)(ch.p(), cit);
            rit = (r.hashtable_iter_next)(rh.p(), rit);
        }
        diff_eq!(rit.is_null(), cit.is_null(), "both iterations ended together");
        diff_eq!(ch.state(), rh.state(), "state after iter_set sweep");
        diff_eq!(ch.iterate(), rh.iterate(), "values after iter_set sweep");
    }
}

#[test]
fn hashtable_iter_next_then_delete_current() {
    let _g = global_state_lock();
    // The documented safe-delete idiom: grab `next` first, then delete the
    // element the current iterator points at.
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        for i in 0..30 {
            let k = format!("k{i:02}");
            ch.set(k.as_bytes(), (c.json_integer)(i));
            rh.set(k.as_bytes(), (r.json_integer)(i));
        }
        let mut cit = (c.hashtable_iter)(ch.p());
        let mut rit = (r.hashtable_iter)(rh.p());
        let mut step = 0;
        while !cit.is_null() {
            let cnext = (c.hashtable_iter_next)(ch.p(), cit);
            let rnext = (r.hashtable_iter_next)(rh.p(), rit);

            let ckl = (c.hashtable_iter_key_len)(cit);
            let ckp = (c.hashtable_iter_key)(cit) as *const u8;
            let ckey: Vec<u8> = (0..ckl).map(|j| *ckp.add(j)).collect();
            let rkl = (r.hashtable_iter_key_len)(rit);
            let rkp = (r.hashtable_iter_key)(rit) as *const u8;
            let rkey: Vec<u8> = (0..rkl).map(|j| *rkp.add(j)).collect();
            diff_eq!(ckey.clone(), rkey.clone(), "step {step}: current key");

            // Delete every other element so the iteration also has to survive
            // walking over survivors.
            if step % 2 == 0 {
                diff_eq!(ch.del(&ckey), rh.del(&rkey), "step {step}: delete current");
            }
            diff_eq!(ch.state(), rh.state(), "step {step}: state");

            cit = cnext;
            rit = rnext;
            step += 1;
        }
        diff_eq!(cit.is_null(), rit.is_null(), "iterations ended together");
        diff_eq!(ch.iterate(), rh.iterate(), "final iteration order");
    }
}

#[test]
fn hashtable_keys_with_embedded_nul_are_distinct() {
    let _g = global_state_lock();
    // Keys are compared with memcmp over key_len, so "ab\0cd" (len 5) and "ab"
    // (len 2) are different keys, and so are "ab\0cd" and "ab\0ef".
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        let keys: Vec<&[u8]> = vec![
            b"ab",
            b"ab\0cd",
            b"ab\0ef",
            b"\0",
            b"\0\0",
            b"",
            b"a\0",
            b"\0a",
        ];
        for (i, k) in keys.iter().enumerate() {
            diff_eq!(
                ch.set(k, (c.json_integer)(i as i64)),
                rh.set(k, (r.json_integer)(i as i64)),
                "set NUL-key #{i}"
            );
            diff_eq!(ch.state(), rh.state(), "state after NUL-key #{i}");
        }
        // All eight must coexist.
        diff_eq!(ch.state(), rh.state(), "all NUL keys distinct");
        assert_eq!(ch.state().0, keys.len(), "C: all NUL-containing keys are distinct");
        diff_eq!(ch.iterate(), rh.iterate(), "iteration over NUL-containing keys");

        for k in &keys {
            let cv = ch.get(k);
            let rv = rh.get(k);
            diff_eq!(cv.is_null(), rv.is_null(), "get NUL-key {k:?}");
            diff_eq!(
                (c.json_integer_value)(cv as *mut json_t),
                (r.json_integer_value)(rv as *mut json_t),
                "value for NUL-key {k:?}"
            );
        }
    }
}

#[test]
fn hashtable_empty_and_very_long_keys() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        let long_a = vec![b'x'; 100_000];
        let mut long_b = vec![b'x'; 100_000];
        *long_b.last_mut().unwrap() = b'y'; // differs only in the final byte
        let keys: Vec<&[u8]> = vec![b"", &long_a, &long_b];
        for (i, k) in keys.iter().enumerate() {
            diff_eq!(
                ch.set(k, (c.json_integer)(i as i64)),
                rh.set(k, (r.json_integer)(i as i64)),
                "set long key #{i}"
            );
        }
        diff_eq!(ch.state(), rh.state(), "state with long keys");
        for (i, k) in keys.iter().enumerate() {
            diff_eq!(
                (c.json_integer_value)(ch.get(k) as *mut json_t),
                (r.json_integer_value)(rh.get(k) as *mut json_t),
                "get long key #{i}"
            );
        }
        diff_eq!(ch.iterate(), rh.iterate(), "iteration with long keys");
    }
}

#[test]
fn hashtable_binary_keys_full_byte_range() {
    let _g = global_state_lock();
    // Exercises hashlittle() over keys containing every byte value and every
    // length modulo 4 (its main loop consumes 12 bytes at a time with a
    // switch-based tail, so length % 12 matters).
    let (c, r) = both();
    let mut rng = Rng::new(0x47_0002);
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        // Every length from 0..40 with pseudo-random bytes, so every tail case
        // of hashlittle's switch is hit.
        for len in 0..40usize {
            let k: Vec<u8> = (0..len).map(|_| rng.next_u32() as u8).collect();
            keys.push(k);
        }
        // Plus single-byte keys covering all 256 byte values.
        for b in 0..=255u8 {
            keys.push(vec![b]);
        }
        for (i, k) in keys.iter().enumerate() {
            diff_eq!(
                ch.set(k, (c.json_integer)(i as i64)),
                rh.set(k, (r.json_integer)(i as i64)),
                "set binary key #{i} (len {})",
                k.len()
            );
            diff_eq!(ch.state(), rh.state(), "state after binary key #{i}");
        }
        // The iteration order here is a direct fingerprint of hashlittle()
        // plus the bucket bookkeeping.
        diff_eq!(ch.iterate(), rh.iterate(), "iteration over binary keys");
        for (i, k) in keys.iter().enumerate() {
            diff_eq!(
                (c.json_integer_value)(ch.get(k) as *mut json_t),
                (r.json_integer_value)(rh.get(k) as *mut json_t),
                "get binary key #{i}"
            );
        }
    }
}

#[test]
fn hashtable_randomised_operation_sequences() {
    let _g = global_state_lock();
    // Property-style: the same random mix of set/get/del/clear/iterate applied
    // to both tables, with the complete state compared after every step.
    let (c, r) = both();
    let mut rng = Rng::new(0x47_0003);
    unsafe {
        for trial in 0..60 {
            let (mut ch, _) = Ht::new(c);
            let (mut rh, _) = Ht::new(r);
            // A small key space so collisions, replacements and failed
            // deletions all happen frequently.
            let space = 1 + rng.below(40);
            for step in 0..200 {
                let k = format!("k{}", rng.below(space));
                match rng.below(12) {
                    0..=6 => {
                        let v = rng.json_int();
                        diff_eq!(
                            ch.set(k.as_bytes(), (c.json_integer)(v)),
                            rh.set(k.as_bytes(), (r.json_integer)(v)),
                            "trial {trial} step {step}: set {k}"
                        );
                    }
                    7 | 8 => {
                        let cv = ch.get(k.as_bytes());
                        let rv = rh.get(k.as_bytes());
                        diff_eq!(
                            cv.is_null(),
                            rv.is_null(),
                            "trial {trial} step {step}: get {k} null-ness"
                        );
                        if !cv.is_null() {
                            diff_eq!(
                                (c.json_integer_value)(cv as *mut json_t),
                                (r.json_integer_value)(rv as *mut json_t),
                                "trial {trial} step {step}: get {k} value"
                            );
                        }
                    }
                    9 | 10 => {
                        diff_eq!(
                            ch.del(k.as_bytes()),
                            rh.del(k.as_bytes()),
                            "trial {trial} step {step}: del {k}"
                        );
                    }
                    _ => {
                        (c.hashtable_clear)(ch.p());
                        (r.hashtable_clear)(rh.p());
                    }
                }
                diff_eq!(ch.state(), rh.state(), "trial {trial} step {step}: state");
                diff_eq!(
                    ch.iterate(),
                    rh.iterate(),
                    "trial {trial} step {step}: iteration order"
                );
            }
        }
    }
}

#[test]
fn hashtable_values_of_every_json_type() {
    let _g = global_state_lock();
    // The table stores borrowed `json_t *` of any type and decrefs them on
    // delete/replace/close; make sure every type survives that path.
    let (c, r) = both();
    unsafe {
        let (mut ch, _) = Ht::new(c);
        let (mut rh, _) = Ht::new(r);
        let makers: Vec<(&str, fn(&Api) -> *mut json_t)> = vec![
            ("object", |a| unsafe { (a.json_object)() }),
            ("array", |a| unsafe { (a.json_array)() }),
            ("string", |a| unsafe { (a.json_string)(b"hi\0".as_ptr() as *const c_char) }),
            ("integer", |a| unsafe { (a.json_integer)(42) }),
            ("real", |a| unsafe { (a.json_real)(1.5) }),
            ("true", |a| unsafe { (a.json_true)() }),
            ("false", |a| unsafe { (a.json_false)() }),
            ("null", |a| unsafe { (a.json_null)() }),
        ];
        for (name, mk) in &makers {
            diff_eq!(
                ch.set(name.as_bytes(), mk(c)),
                rh.set(name.as_bytes(), mk(r)),
                "set {name} value"
            );
        }
        diff_eq!(ch.state(), rh.state(), "state with all value types");
        for (name, _) in &makers {
            let cv = ch.get(name.as_bytes()) as *mut json_t;
            let rv = rh.get(name.as_bytes()) as *mut json_t;
            diff_eq!(cv.is_null(), rv.is_null(), "get {name} null-ness");
            diff_eq!(typeof_(cv), typeof_(rv), "type of stored {name}");
        }
        // Replace each with a different type, exercising the decref-on-replace
        // path for singletons (refcount == (size_t)-1) as well as heap values.
        for (name, _) in &makers {
            diff_eq!(
                ch.set(name.as_bytes(), (c.json_integer)(7)),
                rh.set(name.as_bytes(), (r.json_integer)(7)),
                "replace {name} with integer"
            );
        }
        diff_eq!(ch.iterate(), rh.iterate(), "after replacing all with integers");
    }
}
