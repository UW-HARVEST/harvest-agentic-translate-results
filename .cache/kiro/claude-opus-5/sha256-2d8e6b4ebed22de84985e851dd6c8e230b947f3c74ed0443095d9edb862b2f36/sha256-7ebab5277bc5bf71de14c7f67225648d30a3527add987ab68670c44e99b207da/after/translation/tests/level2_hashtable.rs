//! Level 2: hashtable_seed.c and hashtable.c
//!
//! Both hashtables are driven with the same operation stream and their
//! observable state (size, order, iteration order, keys, values) is compared
//! after every step.

mod common;

use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_void};

const SEED: usize = 0x5eed_1234;

/// Seed both libraries identically so that bucket assignment (and therefore
/// rehash behaviour and intra-bucket ordering) is deterministic and equal.
fn seed_both() -> (&'static Lib, &'static Lib) {
    let (c, r) = libs();
    for l in [c, r] {
        let f: Symbol<FnJsonObjectSeed> = l.sym("json_object_seed");
        unsafe { f(SEED) };
    }
    (c, r)
}

fn read_seed(l: &Lib) -> u32 {
    let s: Symbol<*mut u32> = l.sym("hashtable_seed");
    unsafe { **s }
}

#[test]
fn hashtable_seed_matches() {
    let (c, r) = seed_both();
    assert_eq!(read_seed(c), read_seed(r), "hashtable_seed value");
    assert_eq!(read_seed(c), SEED as u32);

    // A second call must be a no-op (seed already non-zero).
    for l in [c, r] {
        let f: Symbol<FnJsonObjectSeed> = l.sym("json_object_seed");
        unsafe {
            f(0xffff_ffff);
            f(0);
        }
    }
    assert_eq!(read_seed(c), SEED as u32, "C seed unchanged");
    assert_eq!(read_seed(r), SEED as u32, "Rust seed unchanged");
}

struct Ht<'a> {
    l: &'a Lib,
    t: Box<HashtableT>,
}

impl<'a> Ht<'a> {
    fn new(l: &'a Lib) -> Self {
        let mut t = Box::new(HashtableT::default());
        let f: Symbol<FnHtInit> = l.sym("hashtable_init");
        assert_eq!(unsafe { f(&mut *t) }, 0);
        Ht { l, t }
    }

    fn p(&mut self) -> *mut HashtableT {
        &mut *self.t
    }

    fn set(&mut self, key: &[u8], v: i64) -> i32 {
        let mk: Symbol<FnJsonInteger> = self.l.sym("json_integer");
        let f: Symbol<FnHtSet> = self.l.sym("hashtable_set");
        let val = unsafe { mk(v) };
        unsafe { f(&mut *self.t, key.as_ptr() as *const c_char, key.len(), val) }
    }

    fn get(&mut self, key: &[u8]) -> Option<i64> {
        let f: Symbol<FnHtGet> = self.l.sym("hashtable_get");
        let iv: Symbol<FnJsonIntegerValue> = self.l.sym("json_integer_value");
        unsafe {
            let p = f(&mut *self.t, key.as_ptr() as *const c_char, key.len());
            if p.is_null() {
                None
            } else {
                Some(iv(p as *const JsonT))
            }
        }
    }

    fn del(&mut self, key: &[u8]) -> i32 {
        let f: Symbol<FnHtDel> = self.l.sym("hashtable_del");
        unsafe { f(&mut *self.t, key.as_ptr() as *const c_char, key.len()) }
    }

    fn clear(&mut self) {
        let f: Symbol<FnHtClear> = self.l.sym("hashtable_clear");
        unsafe { f(&mut *self.t) };
    }

    /// (size, order, [(key, key_len, integer value, refcount)] in iteration order)
    fn snapshot(&mut self) -> (usize, usize, Vec<(Vec<u8>, usize, i64, usize)>) {
        let it: Symbol<FnHtIter> = self.l.sym("hashtable_iter");
        let next: Symbol<FnHtIterNext> = self.l.sym("hashtable_iter_next");
        let key: Symbol<FnHtIterKey> = self.l.sym("hashtable_iter_key");
        let klen: Symbol<FnHtIterKeyLen> = self.l.sym("hashtable_iter_key_len");
        let val: Symbol<FnHtIterValue> = self.l.sym("hashtable_iter_value");
        let iv: Symbol<FnJsonIntegerValue> = self.l.sym("json_integer_value");

        let mut out = Vec::new();
        unsafe {
            let mut i = it(&mut *self.t);
            while !i.is_null() {
                let kp = key(i) as *const u8;
                let kl = klen(i);
                let k = std::slice::from_raw_parts(kp, kl).to_vec();
                // key must be NUL terminated
                assert_eq!(*kp.add(kl), 0, "{}: key not NUL terminated", self.l.name);
                let v = val(i) as *const JsonT;
                out.push((k, kl, iv(v), (*v).refcount));
                i = next(&mut *self.t, i);
            }
        }
        (self.t.size, self.t.order, out)
    }

    /// Independently verify the bucket chains are consistent by looking every
    /// key up again through `hashtable_get` / `hashtable_iter_at`.
    fn cross_check(&mut self) -> Vec<(Vec<u8>, Option<i64>, bool)> {
        let snap = self.snapshot();
        let iat: Symbol<FnHtIterAt> = self.l.sym("hashtable_iter_at");
        let mut out = Vec::new();
        for (k, _, _, _) in &snap.2 {
            let g = self.get(k);
            let found = unsafe {
                !iat(&mut *self.t, k.as_ptr() as *const c_char, k.len()).is_null()
            };
            out.push((k.clone(), g, found));
        }
        out
    }
}

impl Drop for Ht<'_> {
    fn drop(&mut self) {
        let f: Symbol<FnHtClose> = self.l.sym("hashtable_close");
        unsafe { f(&mut *self.t) };
    }
}

#[test]
fn hashtable_init_state_matches() {
    let (c, r) = seed_both();
    let mut hc = Ht::new(c);
    let mut hr = Ht::new(r);
    assert_eq!(hc.snapshot(), hr.snapshot(), "fresh hashtable");
    assert_eq!(hc.t.size, 0);
    assert_eq!(hc.t.order, 3, "INITIAL_HASHTABLE_ORDER");
    assert_eq!(hr.t.order, 3, "INITIAL_HASHTABLE_ORDER (rust)");
    // iter on an empty table
    let itc: Symbol<FnHtIter> = c.sym("hashtable_iter");
    let itr: Symbol<FnHtIter> = r.sym("hashtable_iter");
    assert!(unsafe { itc(hc.p()) }.is_null());
    assert!(unsafe { itr(hr.p()) }.is_null());
    // get/del on an empty table
    assert_eq!(hc.get(b"nope"), hr.get(b"nope"));
    assert_eq!(hc.del(b"nope"), hr.del(b"nope"));
}

fn test_keys() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"b".to_vec(),
        b"ab".to_vec(),
        b"ba".to_vec(),
        b"abc".to_vec(),
        b"key".to_vec(),
        b"KEY".to_vec(),
        b"a longer key that exceeds the usual short-string cases".to_vec(),
        vec![0u8],
        vec![0u8, 0u8],
        b"a\0b".to_vec(),
        vec![0xffu8, 0xfe, 0x00, 0x41],
        "ünïcödé".as_bytes().to_vec(),
    ];
    for i in 0..120u32 {
        v.push(format!("k{i}").into_bytes());
    }
    for i in 0..40u32 {
        v.push(vec![b'x'; i as usize]);
    }
    v
}

#[test]
fn hashtable_set_get_del_matches() {
    let (c, r) = seed_both();
    let mut hc = Ht::new(c);
    let mut hr = Ht::new(r);
    let keys = test_keys();

    // Insert everything (this crosses several rehash boundaries: order goes
    // 3 -> 4 -> ... as size exceeds hashsize(order)).
    for (i, k) in keys.iter().enumerate() {
        let a = hc.set(k, i as i64);
        let b = hr.set(k, i as i64);
        assert_eq!(a, b, "set({k:02x?}) rc");
        assert_eq!(hc.snapshot(), hr.snapshot(), "after set({k:02x?})");
    }
    assert_eq!(hc.cross_check(), hr.cross_check(), "cross check after inserts");

    // Overwrite each key with a new value (exercises the "replace existing"
    // path, which keeps the original insertion position).
    for (i, k) in keys.iter().enumerate() {
        let a = hc.set(k, 1000 + i as i64);
        let b = hr.set(k, 1000 + i as i64);
        assert_eq!(a, b, "overwrite({k:02x?}) rc");
        assert_eq!(hc.snapshot(), hr.snapshot(), "after overwrite({k:02x?})");
    }

    // Look up keys that are not present, including prefixes of present keys.
    for k in [&b"zzz"[..], b"k", b"k1234", b"a longer key that exceeds"] {
        assert_eq!(hc.get(k), hr.get(k), "get missing {k:02x?}");
        assert_eq!(hc.del(k), hr.del(k), "del missing {k:02x?}");
    }

    // Delete in an interleaved order.
    let mut order: Vec<usize> = (0..keys.len()).collect();
    let mut s: u64 = 7;
    for i in (1..order.len()).rev() {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = ((s >> 33) as usize) % (i + 1);
        order.swap(i, j);
    }
    for &i in &order {
        let k = &keys[i];
        let a = hc.del(k);
        let b = hr.del(k);
        assert_eq!(a, b, "del({k:02x?}) rc");
        assert_eq!(hc.snapshot(), hr.snapshot(), "after del({k:02x?})");
        // deleting twice must fail the same way
        assert_eq!(hc.del(k), hr.del(k), "double del({k:02x?})");
    }
    assert_eq!(hc.t.size, 0);
    assert_eq!(hr.t.size, 0);
    assert_eq!(hc.snapshot(), hr.snapshot(), "empty again");
}

#[test]
fn hashtable_clear_matches() {
    let (c, r) = seed_both();
    let mut hc = Ht::new(c);
    let mut hr = Ht::new(r);
    for i in 0..60i64 {
        let k = format!("clear{i}").into_bytes();
        hc.set(&k, i);
        hr.set(&k, i);
    }
    assert_eq!(hc.snapshot(), hr.snapshot(), "before clear");
    hc.clear();
    hr.clear();
    assert_eq!(hc.snapshot(), hr.snapshot(), "after clear");
    // reuse after clear
    for i in 0..30i64 {
        let k = format!("again{i}").into_bytes();
        assert_eq!(hc.set(&k, i), hr.set(&k, i));
        assert_eq!(hc.snapshot(), hr.snapshot(), "after clear+set {i}");
    }
}

#[test]
fn hashtable_iter_set_matches() {
    let (c, r) = seed_both();
    let mut hc = Ht::new(c);
    let mut hr = Ht::new(r);
    for i in 0..20i64 {
        let k = format!("i{i}").into_bytes();
        hc.set(&k, i);
        hr.set(&k, i);
    }

    let itc: Symbol<FnHtIter> = c.sym("hashtable_iter");
    let itr: Symbol<FnHtIter> = r.sym("hashtable_iter");
    let nextc: Symbol<FnHtIterNext> = c.sym("hashtable_iter_next");
    let nextr: Symbol<FnHtIterNext> = r.sym("hashtable_iter_next");
    let setc: Symbol<FnHtIterSet> = c.sym("hashtable_iter_set");
    let setr: Symbol<FnHtIterSet> = r.sym("hashtable_iter_set");
    let mkc: Symbol<FnJsonInteger> = c.sym("json_integer");
    let mkr: Symbol<FnJsonInteger> = r.sym("json_integer");

    unsafe {
        let mut ic = itc(hc.p());
        let mut ir = itr(hr.p());
        let mut n = 0i64;
        while !ic.is_null() {
            assert!(!ir.is_null());
            setc(ic, mkc(500 + n));
            setr(ir, mkr(500 + n));
            ic = nextc(hc.p(), ic);
            ir = nextr(hr.p(), ir);
            n += 1;
        }
        assert!(ir.is_null());
    }
    assert_eq!(hc.snapshot(), hr.snapshot(), "after iter_set sweep");
}

#[test]
fn hashtable_iter_at_matches() {
    let (c, r) = seed_both();
    let mut hc = Ht::new(c);
    let mut hr = Ht::new(r);
    let keys = test_keys();
    for (i, k) in keys.iter().enumerate() {
        hc.set(k, i as i64);
        hr.set(k, i as i64);
    }

    let iatc: Symbol<FnHtIterAt> = c.sym("hashtable_iter_at");
    let iatr: Symbol<FnHtIterAt> = r.sym("hashtable_iter_at");
    let nextc: Symbol<FnHtIterNext> = c.sym("hashtable_iter_next");
    let nextr: Symbol<FnHtIterNext> = r.sym("hashtable_iter_next");
    let keyc: Symbol<FnHtIterKey> = c.sym("hashtable_iter_key");
    let keyr: Symbol<FnHtIterKey> = r.sym("hashtable_iter_key");
    let klc: Symbol<FnHtIterKeyLen> = c.sym("hashtable_iter_key_len");
    let klr: Symbol<FnHtIterKeyLen> = r.sym("hashtable_iter_key_len");
    let vc: Symbol<FnHtIterValue> = c.sym("hashtable_iter_value");
    let vr: Symbol<FnHtIterValue> = r.sym("hashtable_iter_value");
    let ivc: Symbol<FnJsonIntegerValue> = c.sym("json_integer_value");
    let ivr: Symbol<FnJsonIntegerValue> = r.sym("json_integer_value");

    // For every key: iter_at then walk the rest of the ordered list; the tails
    // must be identical.
    for k in keys.iter().chain([b"missing".to_vec()].iter()) {
        unsafe {
            let mut ic = iatc(hc.p(), k.as_ptr() as *const c_char, k.len());
            let mut ir = iatr(hr.p(), k.as_ptr() as *const c_char, k.len());
            assert_eq!(ic.is_null(), ir.is_null(), "iter_at({k:02x?}) presence");
            let mut tc = Vec::new();
            let mut tr = Vec::new();
            while !ic.is_null() {
                let kl = klc(ic);
                tc.push((
                    std::slice::from_raw_parts(keyc(ic) as *const u8, kl).to_vec(),
                    kl,
                    ivc(vc(ic) as *const JsonT),
                ));
                ic = nextc(hc.p(), ic);
            }
            while !ir.is_null() {
                let kl = klr(ir);
                tr.push((
                    std::slice::from_raw_parts(keyr(ir) as *const u8, kl).to_vec(),
                    kl,
                    ivr(vr(ir) as *const JsonT),
                ));
                ir = nextr(hr.p(), ir);
            }
            assert_eq!(tc, tr, "iter_at({k:02x?}) tail");
        }
    }
}

#[test]
fn hashtable_random_op_stream_matches() {
    let (c, r) = seed_both();
    let mut hc = Ht::new(c);
    let mut hr = Ht::new(r);
    let mut s: u64 = 0xdead_beef_cafe_babe;
    let mut next = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        s >> 33
    };

    for step in 0..4000u32 {
        let op = next() % 100;
        let k = format!("key{}", next() % 90).into_bytes();
        match op {
            0..=59 => {
                let v = (next() % 1000) as i64;
                assert_eq!(hc.set(&k, v), hr.set(&k, v), "step {step}: set");
            }
            60..=84 => {
                assert_eq!(hc.del(&k), hr.del(&k), "step {step}: del");
            }
            85..=97 => {
                assert_eq!(hc.get(&k), hr.get(&k), "step {step}: get");
            }
            _ => {
                hc.clear();
                hr.clear();
            }
        }
        if step % 13 == 0 {
            assert_eq!(hc.snapshot(), hr.snapshot(), "step {step}: snapshot");
        }
    }
    assert_eq!(hc.snapshot(), hr.snapshot(), "final snapshot");
    assert_eq!(hc.cross_check(), hr.cross_check(), "final cross check");
}

#[test]
fn hashtable_grows_to_same_order() {
    let (c, r) = seed_both();
    let mut hc = Ht::new(c);
    let mut hr = Ht::new(r);
    // Push well past several rehash thresholds (8, 16, 32, ... entries).
    for i in 0..5000i64 {
        let k = format!("grow-{i}").into_bytes();
        hc.set(&k, i);
        hr.set(&k, i);
        if i % 250 == 0 {
            assert_eq!(
                (hc.t.size, hc.t.order),
                (hr.t.size, hr.t.order),
                "size/order at {i}"
            );
        }
    }
    assert_eq!(hc.snapshot(), hr.snapshot(), "after 5000 inserts");
    // Shrinking never lowers the order in jansson; check that too.
    for i in 0..5000i64 {
        let k = format!("grow-{i}").into_bytes();
        assert_eq!(hc.del(&k), hr.del(&k));
    }
    assert_eq!((hc.t.size, hc.t.order), (hr.t.size, hr.t.order), "after deletes");
}

// `hashtable_key_to_iter` is a macro in C, but jansson exports the equivalent
// `json_object_key_to_iter`; verify it agrees with `hashtable_iter_at` results.
#[test]
fn json_object_key_to_iter_matches() {
    let (c, r) = seed_both();
    for l in [c, r] {
        let obj: Symbol<FnNew0> = l.sym("json_object");
        let set: Symbol<FnJsonObjectSetNew> = l.sym("json_object_set_new");
        let mk: Symbol<FnJsonInteger> = l.sym("json_integer");
        let iter: Symbol<FnJsonObjectIter> = l.sym("json_object_iter");
        let ikey: Symbol<FnJsonObjectIterKey> = l.sym("json_object_iter_key");
        let k2i: Symbol<FnJsonObjectKeyToIter> = l.sym("json_object_key_to_iter");
        let del: Symbol<FnJsonDelete> = l.sym("json_delete");
        unsafe {
            let o = obj();
            for i in 0..10i64 {
                let k = cs(&format!("k{i}"));
                set(o, k.as_ptr(), mk(i));
            }
            let it = iter(o);
            let kp = ikey(it);
            let back = k2i(kp);
            assert_eq!(back, it, "{}: key_to_iter round trip", l.name);
            assert!(k2i(std::ptr::null()).is_null(), "{}: key_to_iter(NULL)", l.name);
            del(o);
        }
    }
}

// Keep c_void import used.
const _: Option<*mut c_void> = None;
