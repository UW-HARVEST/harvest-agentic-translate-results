//! Level 7: the same hash-map entry points driven with element and key sizes
//! that `lib.c`'s own driver never uses, to exercise the generic
//! `elemsize * i` / `memcmp(key, ..., keysize)` arithmetic.
mod harness;

use harness::*;
use std::ffi::c_void;
use std::os::raw::c_int;

/// A wide element: 20-byte key followed by 4 bytes of payload.
const ELEM: usize = 24;
const KEYSIZE: usize = 20;

struct WideMap<'a> {
    im: &'a Impl,
    t: *mut c_void,
}

impl<'a> WideMap<'a> {
    fn new(im: &'a Impl) -> Self {
        WideMap {
            im,
            t: std::ptr::null_mut(),
        }
    }

    fn raw(&self) -> *mut c_void {
        unsafe { (self.t as *mut u8).sub(ELEM) as *mut c_void }
    }

    fn temp(&self) -> isize {
        unsafe { header(self.raw()).temp }
    }

    fn put(&mut self, key: &[u8; KEYSIZE], v: c_int) -> isize {
        let mut k = *key;
        self.t = unsafe {
            (self.im.hmput_key)(
                self.t,
                ELEM,
                k.as_mut_ptr() as *mut c_void,
                KEYSIZE,
                HM_BINARY,
            )
        };
        let idx = self.temp();
        unsafe {
            let e = (self.t as *mut u8).offset(idx * ELEM as isize);
            std::ptr::copy_nonoverlapping(k.as_ptr(), e, KEYSIZE);
            std::ptr::copy_nonoverlapping(
                (&raw const v) as *const u8,
                e.add(KEYSIZE),
                std::mem::size_of::<c_int>(),
            );
        }
        idx
    }

    fn geti(&mut self, key: &[u8; KEYSIZE]) -> isize {
        let mut k = *key;
        self.t = unsafe {
            (self.im.hmget_key)(
                self.t,
                ELEM,
                k.as_mut_ptr() as *mut c_void,
                KEYSIZE,
                HM_BINARY,
            )
        };
        self.temp()
    }

    fn del(&mut self, key: &[u8; KEYSIZE]) -> isize {
        let mut k = *key;
        self.t = unsafe {
            (self.im.hmdel_key)(
                self.t,
                ELEM,
                k.as_mut_ptr() as *mut c_void,
                KEYSIZE,
                0,
                HM_BINARY,
            )
        };
        if self.t.is_null() { 0 } else { self.temp() }
    }

    /// Raw bytes of the whole backing array, plus the header — everything the
    /// two implementations must agree on.
    fn snap(&self) -> Option<(usize, usize, isize, bool, Vec<u8>, Option<IndexSnap>)> {
        if self.t.is_null() {
            return None;
        }
        unsafe {
            let h = header(self.raw());
            let bytes =
                std::slice::from_raw_parts(self.raw() as *const u8, h.length * ELEM).to_vec();
            Some((
                h.length,
                h.capacity,
                h.temp,
                h.hash_table.is_null(),
                bytes,
                index_snap(h.hash_table as *mut HashIndex),
            ))
        }
    }

    fn free(&mut self) {
        if !self.t.is_null() {
            unsafe { (self.im.hmfree_func)(self.raw(), ELEM) };
            self.t = std::ptr::null_mut();
        }
    }
}

fn wide_key(n: u32) -> [u8; KEYSIZE] {
    let mut k = [0u8; KEYSIZE];
    // Fill with a pattern that includes high-bit bytes and embedded zeros, so
    // memcmp (not strcmp) semantics are what is being tested.
    for (i, b) in k.iter_mut().enumerate() {
        *b = ((n as usize).wrapping_mul(31).wrapping_add(i * 7) & 0xff) as u8;
    }
    k[0] = (n & 0xff) as u8;
    k[1] = ((n >> 8) & 0xff) as u8;
    k[9] = 0;
    k[10] = 0x80;
    k
}

fn compare(label: &str, c: &WideMap, r: &WideMap) {
    match (c.snap(), r.snap()) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            assert_eq!(
                (a.0, a.1, a.2, a.3),
                (b.0, b.1, b.2, b.3),
                "{label}: header (len, cap, temp, table_null) mismatch"
            );
            assert_eq!(a.4.len(), b.4.len(), "{label}: element byte count mismatch");
            for i in 0..a.4.len() {
                assert_eq!(
                    a.4[i], b.4[i],
                    "{label}: element byte {i} (element {}, offset {}) C={} Rust={}",
                    i / ELEM,
                    i % ELEM,
                    a.4[i],
                    b.4[i]
                );
            }
            match (&a.5, &b.5) {
                (None, None) => {}
                (Some(x), Some(y)) => {
                    assert_eq!(x.slot_count, y.slot_count, "{label}: slot_count");
                    assert_eq!(x.used_count, y.used_count, "{label}: used_count");
                    assert_eq!(x.tombstone_count, y.tombstone_count, "{label}: tombstones");
                    assert_eq!(x.seed, y.seed, "{label}: seed");
                    assert_eq!(x.buckets, y.buckets, "{label}: bucket contents");
                }
                _ => panic!("{label}: hash index nullness mismatch"),
            }
        }
        _ => panic!("{label}: map nullness mismatch"),
    }
}

#[test]
fn wide_keys_put_get_delete() {
    let _g = global_lock();
    let p = pair();
    unsafe {
        (p.c.rand_seed)(0x31415926);
        (p.rs.rand_seed)(0x31415926);
    }
    let mut cm = WideMap::new(&p.c);
    let mut rm = WideMap::new(&p.rs);

    for i in 0..300u32 {
        let k = wide_key(i);
        assert_eq!(cm.put(&k, i as c_int), rm.put(&k, i as c_int), "put {i}");
        compare(&format!("after put {i}"), &cm, &rm);
    }
    for i in 0..320u32 {
        let k = wide_key(i);
        assert_eq!(cm.geti(&k), rm.geti(&k), "get {i}");
    }
    compare("after all gets", &cm, &rm);

    for i in (0..300u32).step_by(3) {
        let k = wide_key(i);
        assert_eq!(cm.del(&k), rm.del(&k), "del {i}");
        compare(&format!("after del {i}"), &cm, &rm);
    }
    for i in 0..300u32 {
        let k = wide_key(i);
        assert_eq!(cm.geti(&k), rm.geti(&k), "get-after-del {i}");
    }
    compare("final", &cm, &rm);

    cm.free();
    rm.free();
}

/// Keys that differ only in their last byte, and keys that share a prefix up to
/// an embedded NUL — the latter would collide under strcmp but not memcmp.
#[test]
fn wide_keys_embedded_nuls() {
    let _g = global_lock();
    let p = pair();
    unsafe {
        (p.c.rand_seed)(7);
        (p.rs.rand_seed)(7);
    }
    let mut cm = WideMap::new(&p.c);
    let mut rm = WideMap::new(&p.rs);

    let mut keys: Vec<[u8; KEYSIZE]> = Vec::new();
    for i in 0..40u8 {
        let mut k = [0u8; KEYSIZE];
        k[0] = b'x';
        k[1] = 0; // everything after this is invisible to strcmp
        k[KEYSIZE - 1] = i;
        keys.push(k);
    }
    for i in 0..40u8 {
        let mut k = [0xffu8; KEYSIZE];
        k[KEYSIZE - 1] = i;
        keys.push(k);
    }

    for (i, k) in keys.iter().enumerate() {
        assert_eq!(cm.put(k, i as c_int), rm.put(k, i as c_int), "put {i}");
        compare(&format!("nul put {i}"), &cm, &rm);
    }
    // All keys must be distinct entries — memcmp, not strcmp.
    assert_eq!(
        unsafe { header(cm.raw()).length } as usize,
        keys.len() + 1,
        "keys with embedded NULs were treated as duplicates"
    );
    for (i, k) in keys.iter().enumerate() {
        assert_eq!(cm.geti(k), rm.geti(k), "get {i}");
    }
    compare("nul final", &cm, &rm);
    cm.free();
    rm.free();
}

/// A one-byte element with a one-byte key: the smallest possible layout.
#[test]
fn single_byte_elements() {
    let _g = global_lock();
    let p = pair();
    unsafe {
        (p.c.rand_seed)(0x31415926);
        (p.rs.rand_seed)(0x31415926);
    }
    let mut ct: *mut c_void = std::ptr::null_mut();
    let mut rt: *mut c_void = std::ptr::null_mut();
    for i in 0..200u8 {
        let mut k = i;
        ct = unsafe {
            (p.c.hmput_key)(ct, 1, (&raw mut k) as *mut c_void, 1, HM_BINARY)
        };
        let mut k = i;
        rt = unsafe {
            (p.rs.hmput_key)(rt, 1, (&raw mut k) as *mut c_void, 1, HM_BINARY)
        };
        let craw = unsafe { (ct as *mut u8).sub(1) as *mut c_void };
        let rraw = unsafe { (rt as *mut u8).sub(1) as *mut c_void };
        let ch = unsafe { header(craw) };
        let rh = unsafe { header(rraw) };
        assert_eq!(
            (ch.length, ch.capacity, ch.temp),
            (rh.length, rh.capacity, rh.temp),
            "1-byte elements: header mismatch at key {i}"
        );
        let cb =
            unsafe { std::slice::from_raw_parts(craw as *const u8, ch.length) }.to_vec();
        let rb =
            unsafe { std::slice::from_raw_parts(rraw as *const u8, rh.length) }.to_vec();
        assert_eq!(cb, rb, "1-byte elements: contents mismatch at key {i}");
        let ci = unsafe { index_snap(ch.hash_table as *mut HashIndex) }.unwrap();
        let ri = unsafe { index_snap(rh.hash_table as *mut HashIndex) }.unwrap();
        assert_eq!(ci.buckets, ri.buckets, "1-byte elements: buckets at key {i}");
        assert_eq!(ci.seed, ri.seed, "1-byte elements: seed at key {i}");
    }
    unsafe {
        (p.c.hmfree_func)((ct as *mut u8).sub(1) as *mut c_void, 1);
        (p.rs.hmfree_func)((rt as *mut u8).sub(1) as *mut c_void, 1);
    }
}
