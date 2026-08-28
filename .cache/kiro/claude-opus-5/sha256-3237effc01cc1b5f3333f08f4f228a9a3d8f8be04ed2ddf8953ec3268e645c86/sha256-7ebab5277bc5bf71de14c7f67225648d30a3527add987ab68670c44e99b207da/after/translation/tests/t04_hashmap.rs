//! Level 4: the binary-key hash map -- `stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_hmdel_key`,
//! `stbds_hmfree_func`.
//!
//! Every operation is a hand-expansion of the corresponding `stbds_hm*` macro
//! from `lib.c`, driven only through the exported symbols of the two `.so`s.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct Kv {
    key: c_int,
    value: c_int,
}

const ES: usize = std::mem::size_of::<Kv>(); // 8, no padding -> raw byte compare is valid
const KS: usize = std::mem::size_of::<c_int>();

/// `struct { int key; int value; } *t` living inside one implementation.
struct Map<'a> {
    im: &'a Impl,
    t: *mut Kv,
}

impl<'a> Map<'a> {
    fn new(im: &'a Impl) -> Self {
        Map {
            im,
            t: std::ptr::null_mut(),
        }
    }

    /// `stbds_temp((t)-1)`
    unsafe fn temp(&self) -> isize {
        unsafe { header(self.t.offset(-1) as *mut c_void).temp }
    }

    unsafe fn hmput(&mut self, k: c_int, v: c_int) {
        unsafe {
            let mut key = k;
            self.t = (self.im.hmput_key)(
                self.t as *mut c_void,
                ES,
                &mut key as *mut c_int as *mut c_void,
                KS,
                0, // the stbds_hmput macro passes literal 0
            ) as *mut Kv;
            (*self.t.offset(self.temp())).key = k;
            (*self.t.offset(self.temp())).value = v;
        }
    }

    unsafe fn hmgeti(&mut self, k: c_int) -> isize {
        unsafe {
            let mut key = k;
            self.t = (self.im.hmget_key)(
                self.t as *mut c_void,
                ES,
                &mut key as *mut c_int as *mut c_void,
                KS,
                STBDS_HM_BINARY,
            ) as *mut Kv;
            self.temp()
        }
    }

    unsafe fn hmget(&mut self, k: c_int) -> c_int {
        unsafe {
            self.hmgeti(k);
            (*self.t.offset(self.temp())).value
        }
    }

    unsafe fn hmgeti_ts(&mut self, k: c_int) -> isize {
        unsafe {
            let mut key = k;
            let mut temp: isize = 0;
            self.t = (self.im.hmget_key_ts)(
                self.t as *mut c_void,
                ES,
                &mut key as *mut c_int as *mut c_void,
                KS,
                &mut temp,
                STBDS_HM_BINARY,
            ) as *mut Kv;
            temp
        }
    }

    unsafe fn hmget_ts(&mut self, k: c_int) -> (isize, c_int) {
        unsafe {
            let temp = self.hmgeti_ts(k);
            (temp, (*self.t.offset(temp)).value)
        }
    }

    unsafe fn hmdefault(&mut self, v: c_int) {
        unsafe {
            self.t = (self.im.hmput_default)(self.t as *mut c_void, ES) as *mut Kv;
            (*self.t.offset(-1)).value = v;
        }
    }

    unsafe fn hmdel(&mut self, k: c_int) -> isize {
        unsafe {
            let mut key = k;
            self.t = (self.im.hmdel_key)(
                self.t as *mut c_void,
                ES,
                &mut key as *mut c_int as *mut c_void,
                KS,
                0, // STBDS_OFFSETOF(t, key)
                STBDS_HM_BINARY,
            ) as *mut Kv;
            if !self.t.is_null() { self.temp() } else { 0 }
        }
    }

    unsafe fn hmlen(&self) -> isize {
        unsafe {
            if self.t.is_null() {
                0
            } else {
                header(self.t.offset(-1) as *mut c_void).length as isize - 1
            }
        }
    }

    unsafe fn hmfree(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.im.hmfree_func)(self.t.offset(-1) as *mut c_void, ES);
            }
            self.t = std::ptr::null_mut();
        }
    }

    unsafe fn snap(&self) -> Vec<u8> {
        unsafe { snapshot_map(self.t as *mut c_void, ES, true) }
    }
}

/// Drives the same operation on both sides and compares the whole structure.
struct Both<'a> {
    c: Map<'a>,
    r: Map<'a>,
}

impl<'a> Both<'a> {
    fn new(p: &'a Pair) -> Self {
        Both {
            c: Map::new(&p.c),
            r: Map::new(&p.r),
        }
    }
    fn check(&self, what: &str) {
        let cs = unsafe { self.c.snap() };
        let rs = unsafe { self.r.snap() };
        assert_bytes_eq(what, &cs, &rs);
        assert_eq!(unsafe { self.c.hmlen() }, unsafe { self.r.hmlen() }, "{what}: hmlen");
    }
    fn put(&mut self, k: c_int, v: c_int) {
        unsafe {
            self.c.hmput(k, v);
            self.r.hmput(k, v);
        }
        self.check(&format!("after hmput({k}, {v})"));
    }
    fn geti(&mut self, k: c_int) -> isize {
        let cv = unsafe { self.c.hmgeti(k) };
        let rv = unsafe { self.r.hmgeti(k) };
        assert_eq!(cv, rv, "hmgeti({k})");
        self.check(&format!("after hmgeti({k})"));
        cv
    }
    fn get(&mut self, k: c_int) -> c_int {
        let cv = unsafe { self.c.hmget(k) };
        let rv = unsafe { self.r.hmget(k) };
        assert_eq!(cv, rv, "hmget({k})");
        self.check(&format!("after hmget({k})"));
        cv
    }
    fn get_ts(&mut self, k: c_int) -> c_int {
        let cv = unsafe { self.c.hmget_ts(k) };
        let rv = unsafe { self.r.hmget_ts(k) };
        assert_eq!(cv, rv, "hmget_ts({k})");
        self.check(&format!("after hmget_ts({k})"));
        cv.1
    }
    fn default_(&mut self, v: c_int) {
        unsafe {
            self.c.hmdefault(v);
            self.r.hmdefault(v);
        }
        self.check(&format!("after hmdefault({v})"));
    }
    fn del(&mut self, k: c_int) -> isize {
        let cv = unsafe { self.c.hmdel(k) };
        let rv = unsafe { self.r.hmdel(k) };
        assert_eq!(cv, rv, "hmdel({k})");
        self.check(&format!("after hmdel({k})"));
        cv
    }
    fn free(&mut self) {
        unsafe {
            self.c.hmfree();
            self.r.hmfree();
        }
        self.check("after hmfree");
    }
}

#[test]
fn hmput_default_on_null() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut b = Both::new(&p);
    b.check("initial (NULL)");
    b.default_(-2);
    b.default_(-7); // second call must be a no-op except for the stored value
    b.free();
}

#[test]
fn get_on_empty_map() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut b = Both::new(&p);
    assert_eq!(b.geti(1), -1);
    assert_eq!(b.geti(1), -1);
    assert_eq!(b.get(1), 0);
    b.free();
}

#[test]
fn put_get_grow() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut b = Both::new(&p);
    b.default_(-2);
    for i in 0..400i32 {
        b.put(i, i.wrapping_mul(5));
    }
    for i in 0..400i32 {
        assert_eq!(b.get(i), i.wrapping_mul(5));
        assert_eq!(b.get_ts(i), i.wrapping_mul(5));
    }
    for i in 400..520i32 {
        assert_eq!(b.geti(i), -1);
    }
    b.free();
}

#[test]
fn overwrite_existing_keys() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut b = Both::new(&p);
    for i in 0..200i32 {
        b.put(i, i);
    }
    for i in 0..200i32 {
        b.put(i, i.wrapping_mul(-3));
    }
    for i in 0..200i32 {
        assert_eq!(b.get(i), i.wrapping_mul(-3));
    }
    b.free();
}

#[test]
fn delete_creates_tombstones_and_rebuilds() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut b = Both::new(&p);
    b.default_(-2);
    for i in 0..300i32 {
        b.put(i, i.wrapping_mul(3));
    }
    // delete every 4th -> tombstone accumulation + rebuild
    let mut i = 2i32;
    while i < 300 {
        b.del(i);
        i += 4;
    }
    for i in 0..300i32 {
        b.get(i);
    }
    // delete everything -> shrink path
    for i in 0..300i32 {
        b.del(i);
    }
    for i in 0..300i32 {
        assert_eq!(b.get(i), -2);
    }
    b.free();
}

#[test]
fn delete_missing_keys() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut b = Both::new(&p);
    // delete on a NULL map
    assert_eq!(b.del(5), 0);
    b.default_(-1);
    // delete on a map with no hash table yet
    assert_eq!(b.del(5), 0);
    for i in 0..50i32 {
        b.put(i, i);
    }
    for i in 100..150i32 {
        assert_eq!(b.del(i), 0);
    }
    for i in 0..50i32 {
        assert_eq!(b.del(i), 1);
        assert_eq!(b.del(i), 0);
    }
    b.free();
}

#[test]
fn reinsert_after_delete() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut b = Both::new(&p);
    for round in 0..6i32 {
        for i in 0..120i32 {
            b.put(i, i + round * 1000);
        }
        for i in (0..120i32).step_by(3) {
            b.del(i);
        }
        for i in (0..120i32).step_by(3) {
            b.put(i, -i - round);
        }
    }
    b.free();
}

#[test]
fn negative_and_extreme_keys() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut b = Both::new(&p);
    let keys = [
        0i32,
        -1,
        1,
        i32::MIN,
        i32::MAX,
        -12345,
        0x7f00_0000,
        -0x7f00_0000,
        0x0080_0000,
        255,
        256,
        -256,
    ];
    for (n, &k) in keys.iter().enumerate() {
        b.put(k, n as c_int);
    }
    for &k in keys.iter() {
        b.geti(k);
        b.get(k);
        b.get_ts(k);
    }
    for &k in keys.iter() {
        b.del(k);
    }
    b.free();
}

#[test]
fn interleaved_mixed_workload() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut b = Both::new(&p);
    b.default_(-99);
    let mut x: u32 = 12345;
    for step in 0..4000u32 {
        x = x.wrapping_mul(1103515245).wrapping_add(12345);
        let k = ((x >> 8) % 250) as i32 - 120;
        match (x >> 3) % 5 {
            0 | 1 => b.put(k, step as c_int),
            2 => {
                b.geti(k);
            }
            3 => {
                b.get_ts(k);
            }
            _ => {
                b.del(k);
            }
        }
    }
    b.free();
}

#[test]
fn free_and_reuse() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut b = Both::new(&p);
    for round in 0..5 {
        for i in 0..100i32 {
            b.put(i, i + round);
        }
        b.free();
    }
    // hmfree on an already-NULL handle
    b.free();
}

#[test]
fn many_elemsizes_and_keysizes() {
    // exercise stbds_hmput_key / hmget_key / hmdel_key directly with element
    // and key sizes other than the {int,int} case
    let p = load_pair();
    for &(es, ks) in &[
        (8usize, 4usize),
        (8, 8),
        (16, 8),
        (16, 16),
        (24, 8),
        (12, 4),
        (32, 16),
        (4, 1),
        (4, 2),
        (64, 64),
    ] {
        p.reset_seed(DEFAULT_SEED);
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        for i in 0..150u64 {
            let mut key = vec![0u8; ks.max(8)];
            for (j, b) in key.iter_mut().enumerate() {
                *b = (i.wrapping_mul(31).wrapping_add(j as u64 * 7)) as u8;
            }
            ct = unsafe {
                (p.c.hmput_key)(ct, es, key.as_mut_ptr() as *mut c_void, ks, STBDS_HM_BINARY)
            };
            rt = unsafe {
                (p.r.hmput_key)(rt, es, key.as_mut_ptr() as *mut c_void, ks, STBDS_HM_BINARY)
            };
            assert_bytes_eq(
                &format!("hmput_key(es={es}, ks={ks}) #{i}"),
                &unsafe { snapshot_map(ct, es, false) },
                &unsafe { snapshot_map(rt, es, false) },
            );
        }
        for i in 0..150u64 {
            let mut key = vec![0u8; ks.max(8)];
            for (j, b) in key.iter_mut().enumerate() {
                *b = (i.wrapping_mul(31).wrapping_add(j as u64 * 7)) as u8;
            }
            let mut ctemp: isize = 0;
            let mut rtemp: isize = 0;
            ct = unsafe {
                (p.c.hmget_key_ts)(
                    ct,
                    es,
                    key.as_mut_ptr() as *mut c_void,
                    ks,
                    &mut ctemp,
                    STBDS_HM_BINARY,
                )
            };
            rt = unsafe {
                (p.r.hmget_key_ts)(
                    rt,
                    es,
                    key.as_mut_ptr() as *mut c_void,
                    ks,
                    &mut rtemp,
                    STBDS_HM_BINARY,
                )
            };
            assert_eq!(ctemp, rtemp, "hmget_key_ts(es={es}, ks={ks}) #{i}");
        }
        for i in (0..150u64).step_by(2) {
            let mut key = vec![0u8; ks.max(8)];
            for (j, b) in key.iter_mut().enumerate() {
                *b = (i.wrapping_mul(31).wrapping_add(j as u64 * 7)) as u8;
            }
            ct = unsafe {
                (p.c.hmdel_key)(ct, es, key.as_mut_ptr() as *mut c_void, ks, 0, STBDS_HM_BINARY)
            };
            rt = unsafe {
                (p.r.hmdel_key)(rt, es, key.as_mut_ptr() as *mut c_void, ks, 0, STBDS_HM_BINARY)
            };
            assert_bytes_eq(
                &format!("hmdel_key(es={es}, ks={ks}) #{i}"),
                &unsafe { snapshot_map(ct, es, false) },
                &unsafe { snapshot_map(rt, es, false) },
            );
        }
        unsafe {
            (p.c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
            (p.r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn hmfree_func_on_null() {
    let p = load_pair();
    unsafe {
        (p.c.hmfree_func)(std::ptr::null_mut(), ES);
        (p.r.hmfree_func)(std::ptr::null_mut(), ES);
    }
}
