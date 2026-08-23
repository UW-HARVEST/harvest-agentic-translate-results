//! Differential tests for `hashtable.c` (CONFIGS rows 12-22, ERRORS rows 229-234).
//!
//! Everything goes through the two `.so`s' exported `hashtable_*` symbols; the
//! full observable table state (`size`, `order` and the *complete iteration
//! order*) is snapshotted after every single mutation and compared.
//!
//! Two things are worth knowing about what this can and cannot see:
//!
//! * `hashtable_iter`/`iter_next` walk `hashtable->ordered_list`, which is
//!   maintained in strict *insertion* order — it is **not** bucket order. So the
//!   iteration comparison pins down the `ordered_list` splicing done by
//!   `hashtable_set` / `hashtable_do_del` / `hashtable_clear` / rehash, but it
//!   is blind to the bucket chain (`hashtable->list`, `bucket->first/last`).
//!   Only `hashtable_get` / `hashtable_del` / `hashtable_iter_at` walk that
//!   chain, so every snapshot is paired with `hashtable_get` sweeps over the
//!   keys that must be present and the keys that must be gone.
//!
//! * Because `hashtable_find_pair` compares the *stored* `pair->hash` against a
//!   freshly computed one, `hashlittle()` is only ever compared with itself
//!   inside one library. A self-consistent but different hash function would
//!   still behave identically through this API, so `hashtable.c`'s public
//!   surface cannot by itself validate `lookup3.h`. (`tests/t00_smoke.rs`
//!   already checks that `hashtable_seed` matches in both libraries.)

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// snapshotting
// ---------------------------------------------------------------------------

/// One entry as an external caller can observe it: the `key_len` bytes of the
/// key, the value's `type` and its `json_integer_value`.
type Ent = (Vec<u8>, c_int, i64);

/// `(size, order, entries in iteration order)`.
type Snap = (usize, usize, Vec<Ent>);

/// Value as observed through the public getters (None == NULL pointer).
fn val_of(l: &Lib, p: *mut c_void) -> Option<(c_int, i64)> {
    if p.is_null() {
        return None;
    }
    let j = p as *const json_t;
    unsafe { Some(((*j).type_, (l.json_integer_value)(j))) }
}

/// Full observable state of `ht`, obtained only through the public iterator API.
fn snap(l: &Lib, ht: *mut hashtable_t) -> Snap {
    unsafe {
        let mut out: Vec<Ent> = Vec::new();
        let mut it = (l.hashtable_iter)(ht);
        while !it.is_null() {
            let kp = (l.hashtable_iter_key)(it);
            let kl = (l.hashtable_iter_key_len)(it);
            let kb: Vec<u8> = if kp.is_null() || kl == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(kp as *const u8, kl).to_vec()
            };
            let (ty, iv) = val_of(l, (l.hashtable_iter_value)(it)).unwrap_or((-1, i64::MIN));
            out.push((kb, ty, iv));
            it = (l.hashtable_iter_next)(ht, it);
            assert!(
                out.len() <= 1_000_000,
                "{}: hashtable iteration does not terminate",
                l.which
            );
        }
        ((*ht).size, (*ht).order, out)
    }
}

/// Like [`snap`] but also records `key_len` explicitly and the NUL-terminated
/// form of the key, so a misplaced terminator is caught too.
#[allow(clippy::type_complexity)]
fn snap_full(l: &Lib, ht: *mut hashtable_t) -> Vec<(usize, Vec<u8>, Vec<u8>, c_int, i64)> {
    unsafe {
        let mut out = Vec::new();
        let mut it = (l.hashtable_iter)(ht);
        while !it.is_null() {
            let kp = (l.hashtable_iter_key)(it);
            let kl = (l.hashtable_iter_key_len)(it);
            let kb: Vec<u8> = if kp.is_null() || kl == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(kp as *const u8, kl).to_vec()
            };
            let cz = cstr_bytes(kp);
            let (ty, iv) = val_of(l, (l.hashtable_iter_value)(it)).unwrap_or((-1, i64::MIN));
            out.push((kl, kb, cz, ty, iv));
            it = (l.hashtable_iter_next)(ht, it);
            assert!(out.len() <= 1_000_000, "{}: iteration loops", l.which);
        }
        out
    }
}

fn show_key(k: &[u8]) -> String {
    format!("{:?}", String::from_utf8_lossy(k))
}

fn fmt_snap(s: &Snap) -> String {
    let mut out = format!("size={} order={} n={} [", s.0, s.1, s.2.len());
    for (k, ty, v) in s.2.iter().take(48) {
        out.push_str(&format!("{}:t{}={} ", show_key(k), ty, v));
    }
    if s.2.len() > 48 {
        out.push_str("...");
    }
    out.push(']');
    out
}

fn first_diff(a: &Snap, b: &Snap) -> String {
    if a.0 != b.0 {
        return format!("size: C={} RUST={}", a.0, b.0);
    }
    if a.1 != b.1 {
        return format!("order: C={} RUST={}", a.1, b.1);
    }
    for i in 0..a.2.len().min(b.2.len()) {
        if a.2[i] != b.2[i] {
            return format!(
                "iteration position {}: C={}:t{}={} RUST={}:t{}={}",
                i,
                show_key(&a.2[i].0),
                a.2[i].1,
                a.2[i].2,
                show_key(&b.2[i].0),
                b.2[i].1,
                b.2[i].2,
            );
        }
    }
    format!("entry count: C={} RUST={}", a.2.len(), b.2.len())
}

// ---------------------------------------------------------------------------
// table plumbing
// ---------------------------------------------------------------------------

/// A `hashtable_t` in each library. The struct is self-referential (the bucket
/// array points at `&ht->list`), so it is boxed and never moved.
struct Tables {
    c: *mut hashtable_t,
    rs: *mut hashtable_t,
}

fn init_both(d: &Duo) -> Tables {
    let c = Box::into_raw(Box::new(hashtable_t::zeroed()));
    let rs = Box::into_raw(Box::new(hashtable_t::zeroed()));
    unsafe {
        let a = (d.c.hashtable_init)(c);
        let b = (d.rs.hashtable_init)(rs);
        eq("hashtable_init return value", a, b);
        assert_eq!(a, 0, "hashtable_init failed");
    }
    Tables { c, rs }
}

fn close_both(d: &Duo, t: &Tables) {
    unsafe {
        (d.c.hashtable_close)(t.c);
        (d.rs.hashtable_close)(t.rs);
        drop(Box::from_raw(t.c));
        drop(Box::from_raw(t.rs));
    }
}

/// NUL-terminated copy of `key` with 24 bytes of slack, so `hashlittle()`'s
/// word-at-a-time masking trick never reads outside the allocation.
fn kbuf(key: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(key.len() + 24);
    v.extend_from_slice(key);
    v.resize(key.len() + 24, 0);
    v
}

/// `hashtable_set` on both tables. The *same* key pointer is handed to both
/// libraries so `hashlittle()`'s alignment-dependent code paths line up.
fn set_raw(d: &Duo, t: &Tables, buf: &[u8], key_len: usize, v: i64) -> (c_int, c_int) {
    unsafe {
        let cv = (d.c.json_integer)(v);
        let rv = (d.rs.json_integer)(v);
        assert!(!cv.is_null() && !rv.is_null());
        let p = buf.as_ptr() as *const c_char;
        let a = (d.c.hashtable_set)(t.c, p, key_len, cv);
        let b = (d.rs.hashtable_set)(t.rs, p, key_len, rv);
        if a != 0 {
            decref(&d.c, cv);
        }
        if b != 0 {
            decref(&d.rs, rv);
        }
        (a, b)
    }
}

fn set_both(d: &Duo, t: &Tables, key: &[u8], v: i64) -> (c_int, c_int) {
    let buf = kbuf(key);
    set_raw(d, t, &buf, key.len(), v)
}

fn get_raw(
    d: &Duo,
    t: &Tables,
    buf: &[u8],
    key_len: usize,
) -> (Option<(c_int, i64)>, Option<(c_int, i64)>) {
    unsafe {
        let p = buf.as_ptr() as *const c_char;
        let a = (d.c.hashtable_get)(t.c, p, key_len);
        let b = (d.rs.hashtable_get)(t.rs, p, key_len);
        (val_of(&d.c, a), val_of(&d.rs, b))
    }
}

fn get_both(
    d: &Duo,
    t: &Tables,
    key: &[u8],
) -> (Option<(c_int, i64)>, Option<(c_int, i64)>) {
    let buf = kbuf(key);
    get_raw(d, t, &buf, key.len())
}

fn del_raw(d: &Duo, t: &Tables, buf: &[u8], key_len: usize) -> (c_int, c_int) {
    unsafe {
        let p = buf.as_ptr() as *const c_char;
        let a = (d.c.hashtable_del)(t.c, p, key_len);
        let b = (d.rs.hashtable_del)(t.rs, p, key_len);
        (a, b)
    }
}

fn del_both(d: &Duo, t: &Tables, key: &[u8]) -> (c_int, c_int) {
    let buf = kbuf(key);
    del_raw(d, t, &buf, key.len())
}

/// Snapshot both tables and demand exact equality. The message is only built
/// when the assertion actually fires (this runs millions of times).
#[track_caller]
fn check(d: &Duo, t: &Tables, what: impl FnOnce() -> String) {
    let a = snap(&d.c, t.c);
    let b = snap(&d.rs, t.rs);
    if a != b {
        panic!(
            "C vs RUST hashtable divergence [{}]\n  first difference: {}\n  C   : {}\n  RUST: {}",
            what(),
            first_diff(&a, &b),
            fmt_snap(&a),
            fmt_snap(&b),
        );
    }
}

#[track_caller]
fn eq_rv<T: PartialEq + std::fmt::Debug>(what: impl FnOnce() -> String, cv: T, rv: T) {
    if cv != rv {
        panic!(
            "C vs RUST divergence in {}\n  C   : {:?}\n  RUST: {:?}",
            what(),
            cv,
            rv
        );
    }
}

/// `hashtable_iter*` walks `ordered_list`, i.e. *insertion* order, so a
/// corrupted bucket chain (`bucket->first`/`->last`, `pair->list`) is invisible
/// to [`snap`]. Only a `hashtable_get` actually walks the chain, so every key
/// that is supposed to be present is looked up as well.
#[track_caller]
fn get_sweep(d: &Duo, t: &Tables, present: &[Vec<u8>], what: impl Fn() -> String) {
    for k in present.iter() {
        let (cv, rv) = get_both(d, t, k);
        eq_rv(|| format!("{}: get {}", what(), show_key(k)), cv, rv);
        assert!(
            cv.is_some(),
            "C: key {} is in the iteration order but hashtable_get misses it ({})",
            show_key(k),
            what()
        );
    }
}

/// Keys that must be absent: both libraries have to miss.
#[track_caller]
fn miss_sweep(d: &Duo, t: &Tables, absent: &[Vec<u8>], what: impl Fn() -> String) {
    for k in absent.iter() {
        let (cv, rv) = get_both(d, t, k);
        eq_rv(|| format!("{}: get absent {}", what(), show_key(k)), cv, rv);
        assert!(
            cv.is_none(),
            "C: deleted key {} is still reachable ({})",
            show_key(k),
            what()
        );
    }
}

fn key_n(i: usize) -> Vec<u8> {
    format!("k{}", i).into_bytes()
}

/// The order the C ends up with after `n` distinct insertions: it starts at
/// `INITIAL_HASHTABLE_ORDER == 3` and `hashtable_set` doubles *before* the
/// insert whenever `size >= 2^order`.
fn expected_order(n: usize) -> usize {
    let mut o = 3usize;
    while n > (1usize << o) {
        o += 1;
    }
    o
}

// ---------------------------------------------------------------------------
// 1. CONFIGS 12 — hashtable_init / hashtable_close
// ---------------------------------------------------------------------------

#[test]
fn init_state() {
    let d = duo();
    for round in 0..3 {
        let c = Box::into_raw(Box::new(hashtable_t::zeroed()));
        let rs = Box::into_raw(Box::new(hashtable_t::zeroed()));
        unsafe {
            let a = (d.c.hashtable_init)(c);
            let b = (d.rs.hashtable_init)(rs);
            eq(&format!("hashtable_init rv (round {})", round), a, b);
            eq("hashtable_init rv == 0", a, 0);
            eq("fresh size", (*c).size, (*rs).size);
            eq("fresh order", (*c).order, (*rs).order);
            eq("fresh size == 0", (*c).size, 0usize);
            eq("fresh order == INITIAL_HASHTABLE_ORDER", (*c).order, 3usize);
            assert!(!(*c).buckets.is_null() && !(*rs).buckets.is_null());
            // an empty table iterates to nothing (ERRORS 234)
            let ci = (d.c.hashtable_iter)(c);
            let ri = (d.rs.hashtable_iter)(rs);
            eq("hashtable_iter on empty table is NULL", ci.is_null(), ri.is_null());
            assert!(ci.is_null(), "C: hashtable_iter on an empty table must be NULL");
        }
        let t = Tables { c, rs };
        check(d, &t, || format!("fresh table round {}", round));
        close_both(d, &t);
    }
}

// ---------------------------------------------------------------------------
// 2. CONFIGS 13 — insertion + rehash
// ---------------------------------------------------------------------------

#[test]
fn set_counts_and_rehash() {
    let d = duo();
    for &n in &[1usize, 7, 8, 9, 100, 500] {
        let t = init_both(d);
        for i in 0..n {
            let k = key_n(i);
            let (a, b) = set_both(d, &t, &k, i as i64 * 3 + 1);
            eq_rv(|| format!("hashtable_set(n={}, i={})", n, i), a, b);
            eq_rv(|| format!("hashtable_set rv == 0 (n={}, i={})", n, i), a, 0);
            check(d, &t, || format!("after {} of {} inserts", i + 1, n));
            unsafe {
                eq_rv(|| format!("size after {} inserts", i + 1), (*t.c).size, i + 1);
                eq_rv(
                    || format!("C order after {} inserts", i + 1),
                    (*t.c).order,
                    expected_order(i + 1),
                );
            }
        }
        // every key must be retrievable
        for i in 0..n {
            let k = key_n(i);
            let (cv, rv) = get_both(d, &t, &k);
            eq_rv(|| format!("get k{} (n={})", i, n), cv, rv);
            eq_rv(
                || format!("get k{} value (n={})", i, n),
                cv,
                Some((JSON_INTEGER, i as i64 * 3 + 1)),
            );
        }
        close_both(d, &t);
    }
}

// ---------------------------------------------------------------------------
// 3. CONFIGS 14 — overwriting an existing key
// ---------------------------------------------------------------------------

#[test]
fn set_overwrite_existing() {
    let d = duo();

    // (a) the same single key, over and over
    let t = init_both(d);
    for i in 0..50i64 {
        let (a, b) = set_both(d, &t, b"same", i * 7);
        eq_rv(|| format!("overwrite #{}", i), a, b);
        check(d, &t, || format!("after overwrite #{}", i));
        unsafe { eq_rv(|| format!("size after overwrite #{}", i), (*t.c).size, 1usize) };
        let (cv, rv) = get_both(d, &t, b"same");
        eq_rv(|| format!("get after overwrite #{}", i), cv, rv);
        eq_rv(
            || format!("value after overwrite #{}", i),
            cv,
            Some((JSON_INTEGER, i * 7)),
        );
    }
    unsafe { eq_rv(|| "order must not grow".to_string(), (*t.c).order, 3usize) };
    close_both(d, &t);

    // (b) 50 entries with interleaved overwrites of several of them
    let t = init_both(d);
    for i in 0..50usize {
        let k = key_n(i);
        set_both(d, &t, &k, i as i64);
        check(d, &t, || format!("interleave insert {}", i));
        if i % 3 == 0 {
            for j in [0usize, 1, 7, 13, 29] {
                if j <= i {
                    let ok = key_n(j);
                    let (a, b) = set_both(d, &t, &ok, 100_000 + i as i64 * 1000 + j as i64);
                    eq_rv(|| format!("overwrite k{} at step {}", j, i), a, b);
                    check(d, &t, || format!("overwrite k{} at step {}", j, i));
                    unsafe {
                        eq_rv(
                            || format!("size unchanged by overwrite of k{} at {}", j, i),
                            (*t.c).size,
                            i + 1,
                        )
                    };
                }
            }
        }
    }
    close_both(d, &t);
}

// ---------------------------------------------------------------------------
// 4. CONFIGS 15 / ERRORS 229 — key shapes
// ---------------------------------------------------------------------------

#[test]
fn key_shapes() {
    let d = duo();
    let t = init_both(d);

    let mut keys: Vec<Vec<u8>> = Vec::new();
    keys.push(Vec::new()); // key_len 0
    keys.push(b"a".to_vec()); // 1
    keys.push(b"ab".to_vec()); // 2
    keys.push(vec![b'x'; 255]); // 255
    keys.push(vec![b'y'; 1000]); // 1000
    keys.push(vec![0u8]); // a single NUL
    keys.push(b"a\0b".to_vec()); // embedded NUL
    keys.push(b"a\0b\0".to_vec()); // trailing NUL
    keys.push(b"\0\0\0\0\0\0\0\0".to_vec()); // all NULs
    keys.push({
        let mut v = vec![b'z'; 12];
        v[5] = 0;
        v
    }); // NUL in the middle of a 12-byte key (block boundary)
    keys.push(vec![0u8; 13]); // 13 NULs: crosses the 12-byte block
    for n in [3usize, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 24, 25] {
        keys.push((0..n).map(|i| b'A' + (i % 26) as u8).collect());
    }

    for (i, k) in keys.iter().enumerate() {
        let (a, b) = set_both(d, &t, k, i as i64 + 1);
        eq_rv(|| format!("set key #{} (len {})", i, k.len()), a, b);
        eq_rv(|| format!("set key #{} rv == 0", i), a, 0);
        check(d, &t, || format!("after key #{} (len {})", i, k.len()));
        eq_rv(
            || format!("snap_full after key #{}", i),
            snap_full(&d.c, t.c),
            snap_full(&d.rs, t.rs),
        );
    }
    for (i, k) in keys.iter().enumerate() {
        let (cv, rv) = get_both(d, &t, k);
        eq_rv(|| format!("get key #{} (len {})", i, k.len()), cv, rv);
        eq_rv(
            || format!("get key #{} value", i),
            cv,
            Some((JSON_INTEGER, i as i64 + 1)),
        );
    }
    close_both(d, &t);

    // Two keys taken from the SAME buffer that differ only in key_len: the C
    // compares key_len *and* memcmp, so "ab"/2 and "ab"/1 are distinct keys.
    let t = init_both(d);
    let buf = kbuf(b"abcdef");
    for len in 0..=6usize {
        let (a, b) = set_raw(d, &t, &buf, len, 1000 + len as i64);
        eq_rv(|| format!("set prefix len {}", len), a, b);
        check(d, &t, || format!("after prefix len {}", len));
        unsafe {
            eq_rv(
                || format!("size after prefix len {}", len),
                (*t.c).size,
                len + 1,
            )
        };
    }
    for len in 0..=6usize {
        let (cv, rv) = get_raw(d, &t, &buf, len);
        eq_rv(|| format!("get prefix len {}", len), cv, rv);
        eq_rv(
            || format!("get prefix len {} value", len),
            cv,
            Some((JSON_INTEGER, 1000 + len as i64)),
        );
    }
    // key_len 7 was never stored (ERRORS 229)
    let (cv, rv) = get_raw(d, &t, &buf, 7);
    eq_rv(|| "get prefix len 7 (absent)".to_string(), cv, rv);
    eq_rv(|| "get prefix len 7 is NULL".to_string(), cv, None);
    eq_rv(
        || "snap_full prefix table".to_string(),
        snap_full(&d.c, t.c),
        snap_full(&d.rs, t.rs),
    );
    close_both(d, &t);
}

// ---------------------------------------------------------------------------
// 5. CONFIGS 16 / ERRORS 229 — get present / absent
// ---------------------------------------------------------------------------

#[test]
fn get_present_and_absent() {
    let d = duo();
    let t = init_both(d);
    let n = 100usize;
    for i in 0..n {
        set_both(d, &t, &key_n(i), i as i64 - 50);
    }
    check(d, &t, || "100-key table".to_string());

    for i in 0..n {
        let (cv, rv) = get_both(d, &t, &key_n(i));
        eq_rv(|| format!("get present k{}", i), cv, rv);
        eq_rv(
            || format!("get present k{} value", i),
            cv,
            Some((JSON_INTEGER, i as i64 - 50)),
        );
    }
    for i in 0..n {
        let k = format!("absent-{}", i).into_bytes();
        let (cv, rv) = get_both(d, &t, &k);
        eq_rv(|| format!("get absent {:?}", String::from_utf8_lossy(&k)), cv, rv);
        eq_rv(|| format!("get absent {} is NULL", i), cv, None);
    }
    // key_len one shorter / one longer than the stored key
    for i in 0..n {
        let k = key_n(i);
        let buf = kbuf(&k);
        let (cv, rv) = get_raw(d, &t, &buf, k.len() - 1);
        eq_rv(|| format!("get k{} with key_len-1", i), cv, rv);
        let (cv2, rv2) = get_raw(d, &t, &buf, k.len() + 1);
        eq_rv(|| format!("get k{} with key_len+1", i), cv2, rv2);
        eq_rv(|| format!("get k{} with key_len+1 is NULL", i), cv2, None);
    }
    // get on a table that has never held anything (empty bucket path)
    let e = init_both(d);
    for i in 0..20 {
        let (cv, rv) = get_both(d, &e, &key_n(i));
        eq_rv(|| format!("get on empty table k{}", i), cv, rv);
        eq_rv(|| format!("get on empty table k{} is NULL", i), cv, None);
    }
    close_both(d, &e);
    close_both(d, &t);
}

// ---------------------------------------------------------------------------
// 6. CONFIGS 17 / ERRORS 230 — deletion
// ---------------------------------------------------------------------------

#[test]
fn del_positions() {
    let d = duo();

    for n in 1..=12usize {
        for pos in 0..n {
            let t = init_both(d);
            for i in 0..n {
                set_both(d, &t, &key_n(i), i as i64);
            }
            check(d, &t, || format!("built n={}", n));

            let (a, b) = del_both(d, &t, &key_n(pos));
            eq_rv(|| format!("del k{} of n={}", pos, n), a, b);
            eq_rv(|| format!("del k{} of n={} rv == 0", pos, n), a, 0);
            check(d, &t, || format!("n={} after deleting k{}", n, pos));
            let mut alive: Vec<Vec<u8>> = (0..n).filter(|&i| i != pos).map(key_n).collect();
            get_sweep(d, &t, &alive, || format!("n={} after deleting k{}", n, pos));
            miss_sweep(d, &t, &[key_n(pos)], || {
                format!("n={} after deleting k{}", n, pos)
            });

            // deleting it again must miss (ERRORS 230)
            let (a2, b2) = del_both(d, &t, &key_n(pos));
            eq_rv(|| format!("re-del k{} of n={}", pos, n), a2, b2);
            eq_rv(|| format!("re-del k{} of n={} rv == -1", pos, n), a2, -1);
            check(d, &t, || format!("n={} after re-deleting k{}", n, pos));

            // an absent key must miss (ERRORS 230)
            let (a3, b3) = del_both(d, &t, b"no-such-key");
            eq_rv(|| format!("del absent (n={},pos={})", n, pos), a3, b3);
            eq_rv(|| format!("del absent rv == -1 (n={})", n), a3, -1);

            // drain the rest
            let mut left = n - 1;
            for i in 0..n {
                if i == pos {
                    continue;
                }
                let (x, y) = del_both(d, &t, &key_n(i));
                eq_rv(|| format!("drain del k{} (n={},pos={})", i, n, pos), x, y);
                eq_rv(|| format!("drain del k{} rv == 0", i), x, 0);
                left -= 1;
                check(d, &t, || format!("n={} pos={} drained k{}", n, pos, i));
                unsafe { eq_rv(|| format!("size after draining k{}", i), (*t.c).size, left) };
                alive.retain(|k| k != &key_n(i));
                get_sweep(d, &t, &alive, || {
                    format!("n={} pos={} drained k{}", n, pos, i)
                });
                miss_sweep(d, &t, &[key_n(i)], || {
                    format!("n={} pos={} drained k{}", n, pos, i)
                });
            }
            close_both(d, &t);
        }
    }

    // deleting from an empty table
    let t = init_both(d);
    let (a, b) = del_both(d, &t, b"anything");
    eq_rv(|| "del on empty table".to_string(), a, b);
    eq_rv(|| "del on empty table rv == -1".to_string(), a, -1);
    check(d, &t, || "empty table after failed del".to_string());
    close_both(d, &t);

    // 100 keys removed in a fixed pseudo-random order
    let t = init_both(d);
    let n = 100usize;
    for i in 0..n {
        set_both(d, &t, &key_n(i), i as i64);
    }
    let mut order: Vec<usize> = (0..n).collect();
    let mut rng = Rng::new(0x1357_9BDF_0246_8ACE);
    for i in (1..n).rev() {
        let j = rng.below(i + 1);
        order.swap(i, j);
    }
    let mut alive: Vec<Vec<u8>> = (0..n).map(key_n).collect();
    let mut dead: Vec<Vec<u8>> = Vec::new();
    for (step, &idx) in order.iter().enumerate() {
        let (x, y) = del_both(d, &t, &key_n(idx));
        eq_rv(|| format!("random del step {} (k{})", step, idx), x, y);
        eq_rv(|| format!("random del step {} rv == 0", step), x, 0);
        check(d, &t, || format!("random del step {} (k{})", step, idx));
        unsafe {
            eq_rv(
                || format!("size after random del step {}", step),
                (*t.c).size,
                n - step - 1,
            )
        };
        alive.retain(|k| k != &key_n(idx));
        dead.push(key_n(idx));
        get_sweep(d, &t, &alive, || format!("random del step {}", step));
        miss_sweep(d, &t, &dead, || format!("random del step {}", step));
    }
    unsafe {
        eq_rv(|| "order preserved by deletion".to_string(), (*t.c).order, 7usize);
    }
    close_both(d, &t);
}

// ---------------------------------------------------------------------------
// 7. CONFIGS 18 — clear and reuse
// ---------------------------------------------------------------------------

#[test]
fn clear_and_reuse() {
    let d = duo();
    for &n in &[0usize, 1, 100] {
        let t = init_both(d);
        for i in 0..n {
            set_both(d, &t, &key_n(i), i as i64 + 1);
        }
        check(d, &t, || format!("before clear (n={})", n));
        let (co, ro) = unsafe { ((*t.c).order, (*t.rs).order) };
        eq_rv(|| format!("order before clear (n={})", n), co, ro);

        unsafe {
            (d.c.hashtable_clear)(t.c);
            (d.rs.hashtable_clear)(t.rs);
        }
        check(d, &t, || format!("after clear (n={})", n));
        unsafe {
            eq_rv(|| format!("size after clear (n={})", n), (*t.c).size, 0usize);
            eq_rv(|| format!("order unchanged by clear (n={})", n), (*t.c).order, co);
            let ci = (d.c.hashtable_iter)(t.c);
            let ri = (d.rs.hashtable_iter)(t.rs);
            eq_rv(|| format!("iter after clear (n={})", n), ci.is_null(), ri.is_null());
            assert!(ci.is_null(), "C: iter after clear must be NULL");
        }
        // a cleared table must still be usable
        for i in 0..20usize {
            let k = format!("reused-{}", i).into_bytes();
            let (a, b) = set_both(d, &t, &k, 1000 + i as i64);
            eq_rv(|| format!("reuse set {} (n={})", i, n), a, b);
            eq_rv(|| format!("reuse set {} rv == 0", i), a, 0);
            check(d, &t, || format!("reuse insert {} (n={})", i, n));
        }
        for i in 0..20usize {
            let k = format!("reused-{}", i).into_bytes();
            let (cv, rv) = get_both(d, &t, &k);
            eq_rv(|| format!("reuse get {} (n={})", i, n), cv, rv);
            eq_rv(
                || format!("reuse get {} value", i),
                cv,
                Some((JSON_INTEGER, 1000 + i as i64)),
            );
        }
        // the old keys are gone
        for i in 0..n {
            let (cv, rv) = get_both(d, &t, &key_n(i));
            eq_rv(|| format!("old key k{} after clear (n={})", i, n), cv, rv);
            eq_rv(|| format!("old key k{} gone", i), cv, None);
        }
        // clearing twice, and clearing an already-empty table
        unsafe {
            (d.c.hashtable_clear)(t.c);
            (d.rs.hashtable_clear)(t.rs);
            (d.c.hashtable_clear)(t.c);
            (d.rs.hashtable_clear)(t.rs);
        }
        check(d, &t, || format!("after double clear (n={})", n));
        close_both(d, &t);
    }
}

// ---------------------------------------------------------------------------
// 8. CONFIGS 19 / ERRORS 233,234 — full traversal
// ---------------------------------------------------------------------------

#[test]
fn iteration_full_traversal() {
    let d = duo();
    for &n in &[0usize, 1, 8, 100] {
        let t = init_both(d);
        for i in 0..n {
            set_both(d, &t, &key_n(i), i as i64 * 11);
        }

        // step both iterators in lock-step, comparing key / key_len / value
        unsafe {
            let mut ci = (d.c.hashtable_iter)(t.c);
            let mut ri = (d.rs.hashtable_iter)(t.rs);
            eq_rv(
                || format!("hashtable_iter NULL-ness (n={})", n),
                ci.is_null(),
                ri.is_null(),
            );
            if n == 0 {
                // ERRORS 234
                assert!(ci.is_null(), "C: hashtable_iter on an empty table must be NULL");
            } else {
                assert!(!ci.is_null(), "C: hashtable_iter must be non-NULL for n={}", n);
            }
            let mut seen = 0usize;
            let mut last_c = std::ptr::null_mut();
            let mut last_r = std::ptr::null_mut();
            while !ci.is_null() {
                let ckl = (d.c.hashtable_iter_key_len)(ci);
                let rkl = (d.rs.hashtable_iter_key_len)(ri);
                eq_rv(|| format!("iter_key_len at {} (n={})", seen, n), ckl, rkl);

                let ckp = (d.c.hashtable_iter_key)(ci);
                let rkp = (d.rs.hashtable_iter_key)(ri);
                assert!(!ckp.is_null() && !rkp.is_null());
                let ck = std::slice::from_raw_parts(ckp as *const u8, ckl).to_vec();
                let rk = std::slice::from_raw_parts(rkp as *const u8, rkl).to_vec();
                eq_bytes(&format!("iter_key at {} (n={})", seen, n), &ck, &rk);
                // the key is also NUL-terminated
                eq_bytes(
                    &format!("iter_key NUL-terminated at {} (n={})", seen, n),
                    &cstr_bytes(ckp),
                    &cstr_bytes(rkp),
                );

                let cvv = val_of(&d.c, (d.c.hashtable_iter_value)(ci));
                let rvv = val_of(&d.rs, (d.rs.hashtable_iter_value)(ri));
                eq_rv(|| format!("iter_value at {} (n={})", seen, n), cvv, rvv);
                eq_rv(
                    || format!("iter_value at {} is the stored integer (n={})", seen, n),
                    cvv,
                    Some((JSON_INTEGER, {
                        let s = String::from_utf8_lossy(&ck).to_string();
                        s.trim_start_matches('k').parse::<i64>().unwrap() * 11
                    })),
                );

                last_c = ci;
                last_r = ri;
                ci = (d.c.hashtable_iter_next)(t.c, ci);
                ri = (d.rs.hashtable_iter_next)(t.rs, ri);
                eq_rv(
                    || format!("iter_next NULL-ness after {} (n={})", seen, n),
                    ci.is_null(),
                    ri.is_null(),
                );
                seen += 1;
                assert!(seen <= n + 1, "iteration overran n={}", n);
            }
            eq_rv(|| format!("entries traversed (n={})", n), seen, n);
            // ERRORS 233: iter_next from the last element is NULL in both
            if n > 0 {
                let c_end = (d.c.hashtable_iter_next)(t.c, last_c);
                let r_end = (d.rs.hashtable_iter_next)(t.rs, last_r);
                eq_rv(
                    || format!("iter_next at last element (n={})", n),
                    c_end.is_null(),
                    r_end.is_null(),
                );
                assert!(c_end.is_null(), "C: iter_next past the last element must be NULL");
            }
        }
        check(d, &t, || format!("traversal snapshot (n={})", n));
        eq_rv(
            || format!("snap_full (n={})", n),
            snap_full(&d.c, t.c),
            snap_full(&d.rs, t.rs),
        );
        close_both(d, &t);
    }
}

// ---------------------------------------------------------------------------
// 9. CONFIGS 20 / ERRORS 232 — iter_at + resume
// ---------------------------------------------------------------------------

#[test]
fn iter_at_resume() {
    let d = duo();
    let t = init_both(d);
    let n = 60usize;
    for i in 0..n {
        set_both(d, &t, &key_n(i), i as i64);
    }
    check(d, &t, || "60-key table".to_string());

    /// Traverse from `it` to the end, collecting (key, value).
    fn tail(l: &Lib, ht: *mut hashtable_t, mut it: *mut c_void) -> Vec<(Vec<u8>, i64)> {
        let mut out = Vec::new();
        unsafe {
            while !it.is_null() {
                let kl = (l.hashtable_iter_key_len)(it);
                let kp = (l.hashtable_iter_key)(it);
                out.push((
                    std::slice::from_raw_parts(kp as *const u8, kl).to_vec(),
                    (l.json_integer_value)((l.hashtable_iter_value)(it) as *const json_t),
                ));
                it = (l.hashtable_iter_next)(ht, it);
                assert!(out.len() < 100_000, "{}: runaway iteration", l.which);
            }
        }
        out
    }

    for i in 0..n {
        let k = key_n(i);
        let buf = kbuf(&k);
        unsafe {
            let ci = (d.c.hashtable_iter_at)(t.c, buf.as_ptr() as *const c_char, k.len());
            let ri = (d.rs.hashtable_iter_at)(t.rs, buf.as_ptr() as *const c_char, k.len());
            eq_rv(
                || format!("iter_at k{} NULL-ness", i),
                ci.is_null(),
                ri.is_null(),
            );
            assert!(!ci.is_null(), "C: iter_at on a present key must not be NULL");
            eq_rv(|| format!("iter_at k{} key_len", i),
                (d.c.hashtable_iter_key_len)(ci),
                (d.rs.hashtable_iter_key_len)(ri));
            eq_rv(
                || format!("resume from k{}", i),
                tail(&d.c, t.c, ci),
                tail(&d.rs, t.rs, ri),
            );
        }
    }

    // ERRORS 232: absent key -> NULL
    for i in 0..30usize {
        let k = format!("nope-{}", i).into_bytes();
        let buf = kbuf(&k);
        unsafe {
            let ci = (d.c.hashtable_iter_at)(t.c, buf.as_ptr() as *const c_char, k.len());
            let ri = (d.rs.hashtable_iter_at)(t.rs, buf.as_ptr() as *const c_char, k.len());
            eq_rv(|| format!("iter_at absent {:?}", k), ci.is_null(), ri.is_null());
            assert!(ci.is_null(), "C: iter_at on an absent key must be NULL");
        }
    }
    // wrong key_len -> NULL (k10 stored with len 3, probed with 2 and 4)
    for i in 0..n {
        let k = key_n(i);
        let buf = kbuf(&k);
        for len in [k.len().wrapping_sub(1), k.len() + 1] {
            if len == usize::MAX {
                continue;
            }
            unsafe {
                let ci = (d.c.hashtable_iter_at)(t.c, buf.as_ptr() as *const c_char, len);
                let ri = (d.rs.hashtable_iter_at)(t.rs, buf.as_ptr() as *const c_char, len);
                eq_rv(
                    || format!("iter_at k{} with key_len {}", i, len),
                    ci.is_null(),
                    ri.is_null(),
                );
            }
        }
    }
    // iter_at on an empty table
    let e = init_both(d);
    unsafe {
        let buf = kbuf(b"x");
        let ci = (d.c.hashtable_iter_at)(e.c, buf.as_ptr() as *const c_char, 1);
        let ri = (d.rs.hashtable_iter_at)(e.rs, buf.as_ptr() as *const c_char, 1);
        eq_rv(|| "iter_at on empty table".to_string(), ci.is_null(), ri.is_null());
        assert!(ci.is_null());
    }
    close_both(d, &e);
    check(d, &t, || "60-key table unchanged".to_string());
    close_both(d, &t);
}

// ---------------------------------------------------------------------------
// 10. CONFIGS 21 — hashtable_iter_set
// ---------------------------------------------------------------------------

#[test]
fn iter_set_replaces_value() {
    let d = duo();
    let t = init_both(d);
    let n = 30usize;
    for i in 0..n {
        set_both(d, &t, &key_n(i), i as i64);
    }
    check(d, &t, || "30-key table".to_string());

    unsafe {
        let mut ci = (d.c.hashtable_iter)(t.c);
        let mut ri = (d.rs.hashtable_iter)(t.rs);
        let mut pos = 0usize;
        while !ci.is_null() {
            // a fresh value of a rotating type, built by the owning library
            let (cv, rv) = match pos % 5 {
                0 => (
                    (d.c.json_integer)(500 + pos as i64),
                    (d.rs.json_integer)(500 + pos as i64),
                ),
                1 => {
                    let s = cs(&format!("v{}", pos));
                    ((d.c.json_string)(s.as_ptr()), (d.rs.json_string)(s.as_ptr()))
                }
                2 => ((d.c.json_null)(), (d.rs.json_null)()),
                3 => ((d.c.json_true)(), (d.rs.json_true)()),
                _ => ((d.c.json_false)(), (d.rs.json_false)()),
            };
            assert!(!cv.is_null() && !rv.is_null());
            (d.c.hashtable_iter_set)(ci, cv);
            (d.rs.hashtable_iter_set)(ri, rv);

            eq_rv(
                || format!("iter_value right after iter_set at {}", pos),
                val_of(&d.c, (d.c.hashtable_iter_value)(ci)),
                val_of(&d.rs, (d.rs.hashtable_iter_value)(ri)),
            );
            check(d, &t, || format!("after iter_set at position {}", pos));
            eq_rv(
                || format!("size unchanged by iter_set at {}", pos),
                (*t.c).size,
                n,
            );

            ci = (d.c.hashtable_iter_next)(t.c, ci);
            ri = (d.rs.hashtable_iter_next)(t.rs, ri);
            eq_rv(
                || format!("iter_next after iter_set at {}", pos),
                ci.is_null(),
                ri.is_null(),
            );
            pos += 1;
        }
        eq_rv(|| "positions visited".to_string(), pos, n);
    }
    // the replaced values are visible through hashtable_get too
    for i in 0..n {
        let (cv, rv) = get_both(d, &t, &key_n(i));
        eq_rv(|| format!("get k{} after iter_set sweep", i), cv, rv);
    }
    eq_rv(
        || "snap_full after iter_set sweep".to_string(),
        snap_full(&d.c, t.c),
        snap_full(&d.rs, t.rs),
    );

    // a second sweep, now via iter_at, to also cover replacing an already
    // replaced value
    for i in 0..n {
        let k = key_n(i);
        let buf = kbuf(&k);
        unsafe {
            let ci = (d.c.hashtable_iter_at)(t.c, buf.as_ptr() as *const c_char, k.len());
            let ri = (d.rs.hashtable_iter_at)(t.rs, buf.as_ptr() as *const c_char, k.len());
            assert!(!ci.is_null() && !ri.is_null());
            (d.c.hashtable_iter_set)(ci, (d.c.json_integer)(-(i as i64) - 1));
            (d.rs.hashtable_iter_set)(ri, (d.rs.json_integer)(-(i as i64) - 1));
        }
        check(d, &t, || format!("second iter_set sweep at k{}", i));
        let (cv, rv) = get_both(d, &t, &k);
        eq_rv(|| format!("get k{} after second sweep", i), cv, rv);
        eq_rv(
            || format!("value of k{} after second sweep", i),
            cv,
            Some((JSON_INTEGER, -(i as i64) - 1)),
        );
    }
    close_both(d, &t);
}

// ---------------------------------------------------------------------------
// 11. CONFIGS 13,14,16,17,19,22 — randomized operation sequences
// ---------------------------------------------------------------------------

#[test]
fn randomized_operation_sequences() {
    let d = duo();
    let mut rng = Rng::new(0xC0FF_EE00_1234_5678);
    const SEQUENCES: usize = 250;
    const OPS: usize = 250;

    for seq in 0..SEQUENCES {
        let t = init_both(d);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        let mut ctr: usize = 0;
        let mut vctr: i64 = 0;

        for op in 0..OPS {
            let choice = rng.below(100);
            match choice {
                // --- set a (probably) new key -------------------------------
                0..=44 => {
                    ctr += 1;
                    let shape = rng.below(5);
                    let len = rng.below(30);
                    let k: Vec<u8> = match shape {
                        0 => format!("k{}", ctr).into_bytes(),
                        1 => rng.ascii_string(len % 20),
                        2 => rng.random_bytes(1 + len),
                        3 => rng.utf8_string(1 + len % 6),
                        _ => {
                            let mut v = format!("dup{}", ctr % 8).into_bytes();
                            v.push(0);
                            v.extend_from_slice(b"tail");
                            v
                        }
                    };
                    vctr += 1;
                    let (a, b) = set_both(d, &t, &k, vctr);
                    eq_rv(|| format!("seq {} op {} set new", seq, op), a, b);
                    eq_rv(|| format!("seq {} op {} set new rv == 0", seq, op), a, 0);
                    if !keys.iter().any(|x| x == &k) {
                        keys.push(k);
                    }
                }
                // --- overwrite an existing key ------------------------------
                45..=59 => {
                    if keys.is_empty() {
                        continue;
                    }
                    let k = keys[rng.below(keys.len())].clone();
                    vctr += 1;
                    let before = unsafe { (*t.c).size };
                    let (a, b) = set_both(d, &t, &k, vctr);
                    eq_rv(|| format!("seq {} op {} overwrite", seq, op), a, b);
                    unsafe {
                        eq_rv(
                            || format!("seq {} op {} overwrite keeps size", seq, op),
                            (*t.c).size,
                            before,
                        )
                    };
                }
                // --- get ----------------------------------------------------
                60..=74 => {
                    let (k, expect_present) = if !keys.is_empty() && rng.bool() {
                        (keys[rng.below(keys.len())].clone(), true)
                    } else {
                        (format!("missing-{}-{}", seq, op).into_bytes(), false)
                    };
                    let (cv, rv) = get_both(d, &t, &k);
                    eq_rv(|| format!("seq {} op {} get", seq, op), cv, rv);
                    if !expect_present {
                        eq_rv(|| format!("seq {} op {} get absent is NULL", seq, op), cv, None);
                    } else {
                        assert!(cv.is_some(), "C: present key returned NULL");
                    }
                }
                // --- delete an existing key ---------------------------------
                75..=87 => {
                    if keys.is_empty() {
                        continue;
                    }
                    let i = rng.below(keys.len());
                    let k = keys.swap_remove(i);
                    let (a, b) = del_both(d, &t, &k);
                    eq_rv(|| format!("seq {} op {} del existing", seq, op), a, b);
                    eq_rv(|| format!("seq {} op {} del existing rv == 0", seq, op), a, 0);
                }
                // --- delete an absent key -----------------------------------
                88..=95 => {
                    let k = format!("gone-{}-{}", seq, op).into_bytes();
                    let (a, b) = del_both(d, &t, &k);
                    eq_rv(|| format!("seq {} op {} del absent", seq, op), a, b);
                    eq_rv(|| format!("seq {} op {} del absent rv == -1", seq, op), a, -1);
                }
                // --- full explicit iteration --------------------------------
                96..=98 => {
                    eq_rv(
                        || format!("seq {} op {} snap_full", seq, op),
                        snap_full(&d.c, t.c),
                        snap_full(&d.rs, t.rs),
                    );
                }
                // --- clear --------------------------------------------------
                _ => {
                    unsafe {
                        (d.c.hashtable_clear)(t.c);
                        (d.rs.hashtable_clear)(t.rs);
                    }
                    keys.clear();
                }
            }
            check(d, &t, || {
                format!("seq {} op {} (choice {})", seq, op, choice)
            });
            // iteration is insertion-ordered, so it cannot see a mangled bucket
            // chain: look every live key up through the hash path as well.
            get_sweep(d, &t, &keys, || {
                format!("seq {} op {} (choice {})", seq, op, choice)
            });
            eq_rv(
                || format!("seq {} op {} tracked key count", seq, op),
                unsafe { (*t.c).size },
                keys.len(),
            );
        }
        close_both(d, &t);
    }
}

// ---------------------------------------------------------------------------
// 12. ERRORS 231 — init_pair's overflow guard
// ---------------------------------------------------------------------------
//
// NOTE ON REACHABILITY: `hashtable_set` hashes the key
// (`hash = hash_str(key, key_len)`) *before* it ever reaches `init_pair`, and
// `hashlittle()` reads `key_len` bytes. So a `key_len` at (or anywhere near)
// `SIZE_MAX - offsetof(pair_t, key)` makes the hash loop walk off the end of
// the buffer and the process dies with SIGSEGV long before `init_pair`'s
// overflow guard could return `-1`. The guard is therefore *unreachable* in a
// well-formed process, in the C exactly as much as in the Rust.
//
// Both cases are still compared here:
//   * the huge-`key_len` call is made in a forked child, so the two libraries'
//     (identical) fatal behaviour can be observed without killing the runner;
//   * a `jsonp_malloc` that always fails is installed in another forked child,
//     which reaches `init_pair`'s *other* failure return and therefore
//     exercises `hashtable_set`'s `if (!pair) return -1;` for real.

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn setrlimit(resource: c_int, rl: *const RLimit) -> c_int;
}

const RLIMIT_CORE: c_int = 4;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Probe {
    /// `hashtable_set` returned; the payload is its return value.
    Returned(c_int),
    /// The child died from a signal.
    Signaled(c_int),
    ForkFailed,
    Other(c_int),
}

unsafe extern "C" fn failing_malloc(_size: usize) -> *mut c_void {
    std::ptr::null_mut()
}
unsafe extern "C" fn noop_free(_p: *mut c_void) {}

/// Run `hashtable_set(ht, key, key_len, value)` in a forked child so a crash
/// does not take the test process with it. When `no_mem` is set the child first
/// installs an allocator whose `malloc` always fails.
fn probe_set(
    l: &Lib,
    ht: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
    no_mem: bool,
) -> Probe {
    unsafe {
        let pid = fork();
        if pid < 0 {
            return Probe::ForkFailed;
        }
        if pid == 0 {
            let rl = RLimit { cur: 0, max: 0 };
            setrlimit(RLIMIT_CORE, &rl);
            if no_mem {
                (l.json_set_alloc_funcs)(Some(failing_malloc), Some(noop_free));
            }
            let rv = (l.hashtable_set)(ht, key, key_len, value);
            // 100 == returned 0, 101 == returned -1, 102 == anything else
            _exit(match rv {
                0 => 100,
                -1 => 101,
                _ => 102,
            });
        }
        let mut st: c_int = 0;
        if waitpid(pid, &mut st, 0) < 0 {
            return Probe::Other(-1);
        }
        if st & 0x7f == 0 {
            match (st >> 8) & 0xff {
                100 => Probe::Returned(0),
                101 => Probe::Returned(-1),
                other => Probe::Other(other),
            }
        } else if (st & 0x7f) != 0x7f {
            Probe::Signaled(st & 0x7f)
        } else {
            Probe::Other(st)
        }
    }
}

#[test]
fn long_key_rejection() {
    let d = duo();
    let t = init_both(d);
    // a few ordinary entries so the "snapshot unchanged" check has content
    for i in 0..10usize {
        set_both(d, &t, &key_n(i), i as i64);
    }
    check(d, &t, || "before long-key probes".to_string());
    let before_c = snap(&d.c, t.c);

    // A short, perfectly valid key buffer; only `key_len` is absurd.
    let buf = kbuf(b"key");

    // `offsetof(pair_t, key) == 56`, so the C's threshold is `SIZE_MAX - 56`:
    // the first three are at/above it, the last one is just below it (and would
    // ask jsonp_malloc for SIZE_MAX bytes).
    for &key_len in &[usize::MAX, usize::MAX - 8, usize::MAX - 56, usize::MAX - 57] {
        unsafe {
            let cv = (d.c.json_integer)(1);
            let rv = (d.rs.json_integer)(1);
            let cp = probe_set(&d.c, t.c, buf.as_ptr() as *const c_char, key_len, cv, false);
            let rp = probe_set(&d.rs, t.rs, buf.as_ptr() as *const c_char, key_len, rv, false);
            eq_rv(
                || format!("hashtable_set with key_len = SIZE_MAX-{}", usize::MAX - key_len),
                cp,
                rp,
            );
            // Whatever happens, the key must NOT be accepted.
            assert_ne!(
                cp,
                Probe::Returned(0),
                "C: hashtable_set with key_len = SIZE_MAX-{} must not succeed",
                usize::MAX - key_len
            );
            assert!(
                matches!(cp, Probe::Returned(-1) | Probe::Signaled(_)),
                "unexpected C outcome for key_len = SIZE_MAX-{}: {:?}",
                usize::MAX - key_len,
                cp
            );
            // hashtable_set never took ownership in *this* process (the call
            // happened in a child), so the values are ours to release.
            decref(&d.c, cv);
            decref(&d.rs, rv);
        }
    }

    // The other `init_pair` failure return — jsonp_malloc yields NULL — really
    // is reachable, and must surface as -1 from hashtable_set in both.
    for key in [&b"key"[..], &b""[..], &b"a-much-longer-key-value"[..]] {
        let kb = kbuf(key);
        unsafe {
            let cv = (d.c.json_integer)(2);
            let rv = (d.rs.json_integer)(2);
            let cp = probe_set(&d.c, t.c, kb.as_ptr() as *const c_char, key.len(), cv, true);
            let rp = probe_set(&d.rs, t.rs, kb.as_ptr() as *const c_char, key.len(), rv, true);
            eq_rv(
                || format!("hashtable_set under OOM, key {:?}", String::from_utf8_lossy(key)),
                cp,
                rp,
            );
            eq_rv(
                || format!("hashtable_set under OOM returns -1, key {:?}", String::from_utf8_lossy(key)),
                cp,
                Probe::Returned(-1),
            );
            decref(&d.c, cv);
            decref(&d.rs, rv);
        }
    }

    // the parent's tables are untouched
    check(d, &t, || "after long-key probes".to_string());
    eq_rv(
        || "C table unchanged by long-key probes".to_string(),
        before_c,
        snap(&d.c, t.c),
    );
    close_both(d, &t);
}
