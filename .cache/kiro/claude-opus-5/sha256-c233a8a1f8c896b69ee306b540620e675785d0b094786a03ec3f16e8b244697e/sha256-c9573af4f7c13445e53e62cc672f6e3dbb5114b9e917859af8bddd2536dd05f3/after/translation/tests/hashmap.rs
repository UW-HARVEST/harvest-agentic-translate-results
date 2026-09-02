//! Phase B rows 13-44, 59-60: the hash-map core, driven exactly the way the
//! `stbds_hm*` / `stbds_sh*` macros drive it, through `dlsym` only.

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void, CStr, CString};

// ---------------------------------------------------------------------------
// A driver that mirrors what the stb_ds macros do around the raw exports.
// ---------------------------------------------------------------------------

pub struct Map<'a> {
    lib: &'a Lib,
    pub h: *mut c_void,
    elemsize: usize,
    keysize: usize,
    mode: c_int,
}

impl<'a> Map<'a> {
    fn null(lib: &'a Lib, elemsize: usize, keysize: usize, mode: c_int) -> Map<'a> {
        Map { lib, h: std::ptr::null_mut(), elemsize, keysize, mode }
    }

    unsafe fn shmode(
        lib: &'a Lib,
        elemsize: usize,
        keysize: usize,
        mode: c_int,
        sh: c_int,
    ) -> Map<'a> {
        let h = (lib.shmode_func)(elemsize, sh);
        Map { lib, h, elemsize, keysize, mode }
    }

    unsafe fn raw(&self) -> *mut u8 {
        (self.h as *mut u8).sub(self.elemsize)
    }
    unsafe fn hdr(&self) -> *mut ArrayHeader {
        self.raw().sub(HDRSIZE) as *mut ArrayHeader
    }
    unsafe fn temp(&self) -> isize {
        (*self.hdr()).temp
    }

    /// Mirrors `t[stbds_temp(t-1)].value = v` -- initialises the bytes the
    /// library itself never writes, so the dumps contain no realloc garbage.
    unsafe fn write_val(&self, idx: isize, val: u8) {
        if self.keysize >= self.elemsize || self.h.is_null() {
            return;
        }
        let e = (self.h as *mut u8).offset(idx * self.elemsize as isize);
        for k in self.keysize..self.elemsize {
            *e.add(k) = val.wrapping_add(k as u8);
        }
    }

    unsafe fn put(&mut self, key: *mut c_void, val: u8) -> isize {
        self.h = (self.lib.hmput_key)(self.h, self.elemsize, key, self.keysize, self.mode);
        let idx = self.temp();
        self.write_val(idx, val);
        idx
    }

    unsafe fn get(&mut self, key: *mut c_void) -> isize {
        self.h = (self.lib.hmget_key)(self.h, self.elemsize, key, self.keysize, self.mode);
        self.temp()
    }

    unsafe fn get_ts(&mut self, key: *mut c_void) -> isize {
        let mut t: isize = 0x7fff_0000;
        self.h =
            (self.lib.hmget_key_ts)(self.h, self.elemsize, key, self.keysize, &mut t, self.mode);
        t
    }

    unsafe fn del_off(&mut self, key: *mut c_void, keyoffset: usize) -> isize {
        self.h = (self.lib.hmdel_key)(
            self.h,
            self.elemsize,
            key,
            self.keysize,
            keyoffset,
            self.mode,
        );
        if self.h.is_null() {
            0
        } else {
            self.temp()
        }
    }

    unsafe fn del(&mut self, key: *mut c_void) -> isize {
        self.del_off(key, 0)
    }

    unsafe fn put_default(&mut self, val: u8) {
        self.h = (self.lib.hmput_default)(self.h, self.elemsize);
        self.write_val(-1, val);
    }

    unsafe fn dump(&self) -> String {
        dump_table(self.h, self.elemsize, self.keysize)
    }

    unsafe fn len(&self) -> isize {
        if self.h.is_null() {
            0
        } else {
            (*self.hdr()).length as isize - 1
        }
    }

    unsafe fn table_ptr(&self) -> *mut HashIndex {
        if self.h.is_null() {
            std::ptr::null_mut()
        } else {
            (*self.hdr()).hash_table as *mut HashIndex
        }
    }

    #[allow(dead_code)]
    unsafe fn string_mode(&self) -> u8 {
        let t = self.table_ptr();
        if t.is_null() {
            0
        } else {
            (*t).string.mode
        }
    }

    unsafe fn free(&mut self) {
        if !self.h.is_null() {
            (self.lib.hmfree_func)(self.raw() as *mut c_void, self.elemsize);
        }
        self.h = std::ptr::null_mut();
    }
}

/// A pair of drivers, one per library, always fed the identical op sequence.
struct Both<'a> {
    c: Map<'a>,
    r: Map<'a>,
    what: String,
    step: usize,
}

impl<'a> Both<'a> {
    unsafe fn null(p: &'a Pair, elemsize: usize, keysize: usize, mode: c_int, what: &str) -> Both<'a> {
        reset_seed(p, 0x31415926);
        Both {
            c: Map::null(&p.c, elemsize, keysize, mode),
            r: Map::null(&p.r, elemsize, keysize, mode),
            what: what.to_string(),
            step: 0,
        }
    }

    unsafe fn shmode(
        p: &'a Pair,
        elemsize: usize,
        keysize: usize,
        mode: c_int,
        sh: c_int,
        what: &str,
    ) -> Both<'a> {
        reset_seed(p, 0x31415926);
        Both {
            c: Map::shmode(&p.c, elemsize, keysize, mode, sh),
            r: Map::shmode(&p.r, elemsize, keysize, mode, sh),
            what: what.to_string(),
            step: 0,
        }
    }

    unsafe fn check(&mut self, op: &str) {
        self.step += 1;
        let cd = self.c.dump();
        let rd = self.r.dump();
        assert_eq_dump(&format!("{} @step {} after {}", self.what, self.step, op), &cd, &rd);
    }

    unsafe fn put(&mut self, key: *mut c_void, val: u8) {
        let ci = self.c.put(key, val);
        let ri = self.r.put(key, val);
        assert_eq!(ci, ri, "{} put index mismatch (step {})", self.what, self.step);
        self.check(&format!("put -> {ci}"));
    }

    unsafe fn get(&mut self, key: *mut c_void) -> isize {
        let ci = self.c.get(key);
        let ri = self.r.get(key);
        assert_eq!(ci, ri, "{} get index mismatch (step {})", self.what, self.step);
        self.check(&format!("get -> {ci}"));
        ci
    }

    unsafe fn get_ts(&mut self, key: *mut c_void) -> isize {
        let ci = self.c.get_ts(key);
        let ri = self.r.get_ts(key);
        assert_eq!(ci, ri, "{} get_ts temp mismatch (step {})", self.what, self.step);
        self.check(&format!("get_ts -> {ci}"));
        ci
    }

    unsafe fn del(&mut self, key: *mut c_void) -> isize {
        let ci = self.c.del(key);
        let ri = self.r.del(key);
        assert_eq!(ci, ri, "{} del result mismatch (step {})", self.what, self.step);
        self.check(&format!("del -> {ci}"));
        ci
    }

    unsafe fn del_off(&mut self, key: *mut c_void, off: usize) -> isize {
        let ci = self.c.del_off(key, off);
        let ri = self.r.del_off(key, off);
        assert_eq!(ci, ri, "{} del_off result mismatch (step {})", self.what, self.step);
        self.check(&format!("del_off({off}) -> {ci}"));
        ci
    }

    unsafe fn put_default(&mut self, val: u8) {
        self.c.put_default(val);
        self.r.put_default(val);
        self.check("put_default");
    }

    unsafe fn free(&mut self) {
        self.c.free();
        self.r.free();
    }

    /// `stbds_make_hash_index` leaves `temp_key` uninitialised.  Seed both
    /// libraries' field with the same pointer so later comparisons are
    /// meaningful rather than comparing two different `realloc` leftovers.
    unsafe fn seed_temp_key(&mut self, s: &std::ffi::CStr) {
        for m in [&self.c, &self.r] {
            let t = m.table_ptr();
            if !t.is_null() {
                (*t).temp_key = s.as_ptr() as *mut c_char;
            }
        }
    }
}

fn reset_seed(p: &Pair, s: usize) {
    unsafe {
        (p.c.rand_seed)(s);
        (p.r.rand_seed)(s);
    }
}

// ---------------------------------------------------------------------------
// binary-mode key pool
// ---------------------------------------------------------------------------

struct BinKeys {
    keys: Vec<Box<[u8]>>,
}

impl BinKeys {
    fn new(rng: &mut Rng, n: usize, keysize: usize) -> BinKeys {
        // distinct keys (unless keysize == 0, where all keys are equal anyway)
        let mut seen = std::collections::HashSet::new();
        let mut keys = Vec::new();
        let mut guard = 0;
        while keys.len() < n && guard < n * 1000 + 1000 {
            guard += 1;
            let v = rng.bytes(keysize.max(1));
            let cmp = v[..keysize.min(v.len())].to_vec();
            if keysize > 0 && !seen.insert(cmp) {
                continue;
            }
            keys.push(v.into_boxed_slice());
            if keysize == 0 {
                // all keys compare equal; just take n copies
            }
        }
        BinKeys { keys }
    }
    fn ptr(&mut self, i: usize) -> *mut c_void {
        self.keys[i].as_mut_ptr() as *mut c_void
    }
}

// ---------------------------------------------------------------------------
// rows 13-18: stbds_hmput_key, BINARY
// ---------------------------------------------------------------------------

#[test]
fn cfg13_put_binary_bootstrap() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 13);
    unsafe {
        for _ in 0..50 {
            let mut k = BinKeys::new(&mut rng, 1, 8);
            let mut b = Both::null(p, 8, 8, STBDS_HM_BINARY, "cfg13");
            b.put(k.ptr(0), 0x11);
            b.free();
        }
    }
}

unsafe fn binary_put_get_del(
    p: &Pair,
    elemsize: usize,
    keysize: usize,
    n: usize,
    tag: &str,
    rng: &mut Rng,
) {
    let mut keys = BinKeys::new(rng, n, keysize);
    let count = keys.keys.len();
    let mut b = Both::null(p, elemsize, keysize, STBDS_HM_BINARY, tag);
    b.put_default(0xAA);
    for i in 0..count {
        b.put(keys.ptr(i), (i as u8).wrapping_mul(7));
    }
    for i in 0..count {
        b.get(keys.ptr(i));
        b.get_ts(keys.ptr(i));
    }
    // absent keys
    let mut absent = BinKeys::new(rng, 8, keysize);
    for i in 0..absent.keys.len() {
        b.get(absent.ptr(i));
        b.get_ts(absent.ptr(i));
    }
    for i in 0..count {
        b.del(keys.ptr(i));
    }
    for i in 0..count {
        b.get(keys.ptr(i));
    }
    b.free();
}

#[test]
fn cfg14_put_binary_e8_k8_counts() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 14);
    unsafe {
        for n in [1usize, 2, 7, 8, 9, 32, 200] {
            for t in 0..3 {
                binary_put_get_del(p, 8, 8, n, &format!("cfg14 n={n} t={t}"), &mut rng);
            }
        }
    }
}

#[test]
fn cfg15_put_binary_e16_k4() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 15);
    unsafe {
        for n in [1usize, 8, 9, 40, 120] {
            for t in 0..3 {
                binary_put_get_del(p, 16, 4, n, &format!("cfg15 n={n} t={t}"), &mut rng);
            }
        }
    }
}

#[test]
fn cfg16_put_binary_e16_k16() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 16);
    unsafe {
        for n in [1usize, 8, 9, 40, 120] {
            for t in 0..3 {
                binary_put_get_del(p, 16, 16, n, &format!("cfg16 n={n} t={t}"), &mut rng);
            }
        }
    }
}

#[test]
fn cfg17_put_binary_e4_k1_duplicates() {
    let _g = serial();
    // keysize 1 -> only 256 distinct keys, so random draws collide constantly
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 17);
    unsafe {
        for t in 0..6 {
            let mut b = Both::null(p, 4, 1, STBDS_HM_BINARY, &format!("cfg17 t={t}"));
            b.put_default(0x5A);
            let mut store: Vec<Box<[u8]>> = (0..=255u8).map(|v| vec![v].into_boxed_slice()).collect();
            for step in 0..400 {
                let i = rng.below(256);
                let k = store[i].as_mut_ptr() as *mut c_void;
                match rng.below(4) {
                    0 | 1 => b.put(k, step as u8),
                    2 => {
                        b.get(k);
                    }
                    _ => {
                        b.del(k);
                    }
                }
            }
            b.free();
        }
    }
}

#[test]
fn cfg18_put_binary_zero_keysize() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 18);
    unsafe {
        for t in 0..5 {
            let mut b = Both::null(p, 8, 0, STBDS_HM_BINARY, &format!("cfg18 t={t}"));
            b.put_default(0x33);
            let mut store: Vec<Box<[u8]>> = (0..16).map(|_| rng.bytes(8).into_boxed_slice()).collect();
            for step in 0..40 {
                let i = rng.below(16);
                let k = store[i].as_mut_ptr() as *mut c_void;
                match rng.below(3) {
                    0 => b.put(k, step as u8),
                    1 => {
                        b.get(k);
                    }
                    _ => {
                        b.del(k);
                    }
                }
            }
            b.free();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 19-22: get / get_ts
// ---------------------------------------------------------------------------

#[test]
fn cfg19_get_binary_null() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 19);
    unsafe {
        for elemsize in [4usize, 8, 16] {
            let mut k = BinKeys::new(&mut rng, 1, 4);
            let mut b = Both::null(p, elemsize, 4, STBDS_HM_BINARY, "cfg19");
            let i = b.get(k.ptr(0));
            assert_eq!(i, -1, "get on NULL table must yield -1");
            b.free();
        }
    }
}

#[test]
fn cfg20_21_22_get_present_absent() {
    let _g = serial();
    // rows 20, 21, 22 are exercised inside binary_put_get_del; here we assert
    // the *values* of the sentinels explicitly.
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 20);
    unsafe {
        for n in [1usize, 8, 9, 33] {
            let mut keys = BinKeys::new(&mut rng, n, 8);
            let mut absent = BinKeys::new(&mut rng, n, 8);
            let mut b = Both::null(p, 16, 8, STBDS_HM_BINARY, &format!("cfg20 n={n}"));
            b.put_default(0x01);
            for i in 0..n {
                b.put(keys.ptr(i), i as u8);
            }
            for i in 0..n {
                let g = b.get(keys.ptr(i));
                assert!(g >= 0, "present key must have index >= 0, got {g}");
                let t = b.get_ts(keys.ptr(i));
                assert_eq!(g, t, "get and get_ts must agree");
            }
            for i in 0..n {
                let g = b.get(absent.ptr(i));
                let t = b.get_ts(absent.ptr(i));
                assert_eq!(g, -1);
                assert_eq!(t, -1);
            }
            b.free();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 23-27: delete paths (shrink, reverse order, tombstone rebuild)
// ---------------------------------------------------------------------------

#[test]
fn cfg23_24_del_present_absent() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 23);
    unsafe {
        for n in [4usize, 9, 40] {
            let mut keys = BinKeys::new(&mut rng, n, 8);
            let mut absent = BinKeys::new(&mut rng, 4, 8);
            let mut b = Both::null(p, 16, 8, STBDS_HM_BINARY, &format!("cfg23 n={n}"));
            b.put_default(0x02);
            for i in 0..n {
                b.put(keys.ptr(i), i as u8);
            }
            assert_eq!(b.del(keys.ptr(0)), 1, "deleting a present key sets temp=1");
            for i in 0..absent.keys.len() {
                assert_eq!(b.del(absent.ptr(i)), 0, "deleting an absent key leaves temp=0");
            }
            b.free();
        }
    }
}

#[test]
fn cfg25_del_all_forward_shrink() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 25);
    unsafe {
        for n in [9usize, 17, 40, 100, 300] {
            let mut keys = BinKeys::new(&mut rng, n, 8);
            let mut b = Both::null(p, 16, 8, STBDS_HM_BINARY, &format!("cfg25 n={n}"));
            b.put_default(0x03);
            for i in 0..n {
                b.put(keys.ptr(i), i as u8);
            }
            for i in 0..n {
                b.del(keys.ptr(i));
            }
            assert_eq!(b.c.len(), 0);
            b.free();
        }
    }
}

#[test]
fn cfg26_del_all_reverse() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 26);
    unsafe {
        for n in [9usize, 17, 40, 100, 300] {
            let mut keys = BinKeys::new(&mut rng, n, 8);
            let mut b = Both::null(p, 16, 8, STBDS_HM_BINARY, &format!("cfg26 n={n}"));
            b.put_default(0x04);
            for i in 0..n {
                b.put(keys.ptr(i), i as u8);
            }
            for i in (0..n).rev() {
                b.del(keys.ptr(i));
            }
            b.free();
        }
    }
}

#[test]
fn cfg27_del_random_with_reinserts() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 27);
    unsafe {
        for t in 0..6 {
            let n = 120;
            let mut keys = BinKeys::new(&mut rng, n, 8);
            let mut b = Both::null(p, 16, 8, STBDS_HM_BINARY, &format!("cfg27 t={t}"));
            b.put_default(0x05);
            for i in 0..n {
                b.put(keys.ptr(i), i as u8);
            }
            for step in 0..500 {
                let i = rng.below(n);
                match rng.below(5) {
                    0 | 1 => {
                        b.del(keys.ptr(i));
                    }
                    2 | 3 => b.put(keys.ptr(i), step as u8),
                    _ => {
                        b.get(keys.ptr(i));
                    }
                }
            }
            b.free();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 28-29
// ---------------------------------------------------------------------------

#[test]
fn cfg28_put_default_orders() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 28);
    unsafe {
        // before any put
        let mut b = Both::null(p, 16, 8, STBDS_HM_BINARY, "cfg28 before");
        b.put_default(0x10);
        b.put_default(0x11);
        let mut keys = BinKeys::new(&mut rng, 5, 8);
        for i in 0..5 {
            b.put(keys.ptr(i), i as u8);
        }
        b.put_default(0x12);
        b.free();

        // after puts on a bootstrapped table
        let mut b = Both::null(p, 16, 8, STBDS_HM_BINARY, "cfg28 after");
        let mut keys = BinKeys::new(&mut rng, 5, 8);
        for i in 0..5 {
            b.put(keys.ptr(i), i as u8);
        }
        b.put_default(0x13);
        b.put_default(0x14);
        b.free();
    }
}

#[test]
fn cfg29_hmfree_binary_populated() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 29);
    unsafe {
        for n in [1usize, 9, 60] {
            let mut keys = BinKeys::new(&mut rng, n, 8);
            let mut b = Both::null(p, 16, 8, STBDS_HM_BINARY, &format!("cfg29 n={n}"));
            for i in 0..n {
                b.put(keys.ptr(i), i as u8);
            }
            b.free();
            assert!(b.c.h.is_null() && b.r.h.is_null());
        }
    }
}

// ---------------------------------------------------------------------------
// string-key modes: rows 30-40
// ---------------------------------------------------------------------------

struct StrKeys {
    v: Vec<CString>,
}

impl StrKeys {
    fn random(rng: &mut Rng, n: usize, minlen: usize, maxlen: usize) -> StrKeys {
        let mut seen = std::collections::HashSet::new();
        let mut v = Vec::new();
        while v.len() < n {
            let len = minlen + rng.below(maxlen - minlen + 1);
            let s = rng.cstring(len);
            if seen.insert(s.clone()) {
                v.push(s);
            }
        }
        StrKeys { v }
    }
    fn prefixed(n: usize) -> StrKeys {
        StrKeys { v: (0..n).map(|i| CString::new(format!("test_{i}")).unwrap()).collect() }
    }
    fn ptr(&self, i: usize) -> *mut c_void {
        self.v[i].as_ptr() as *mut c_void
    }
}

unsafe fn string_pipeline(p: &Pair, sh: Option<c_int>, keys: &StrKeys, tag: &str) {
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut b = match sh {
        None => Both::null(p, elemsize, keysize, STBDS_HM_STRING, tag),
        Some(m) => Both::shmode(p, elemsize, keysize, STBDS_HM_STRING, m, tag),
    };
    b.put_default(0xC0);
    for i in 0..keys.v.len() {
        b.put(keys.ptr(i), (i as u8).wrapping_mul(11));
    }
    for i in 0..keys.v.len() {
        b.get(keys.ptr(i));
        b.get_ts(keys.ptr(i));
    }
    // absent keys
    for s in ["", "zzzz_absent", "test_999999"] {
        let cs = CString::new(s).unwrap();
        b.get(cs.as_ptr() as *mut c_void);
        b.get_ts(cs.as_ptr() as *mut c_void);
    }
    // re-put every key (existing-key branch)
    for i in 0..keys.v.len() {
        b.put(keys.ptr(i), 0x7E);
    }
    for i in 0..keys.v.len() {
        b.del(keys.ptr(i));
    }
    for i in 0..keys.v.len() {
        b.get(keys.ptr(i));
    }
    b.free();
}

#[test]
fn cfg30_string_autobootstrap_default_mode() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 30);
    unsafe {
        for n in [1usize, 2, 7, 8, 9, 32, 200] {
            let keys = StrKeys::random(&mut rng, n, 1, 24);
            string_pipeline(p, None, &keys, &format!("cfg30 n={n}"));
        }
    }
}

#[test]
fn cfg31_string_sh_default() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 31);
    unsafe {
        for n in [1usize, 2, 7, 8, 9, 32, 200] {
            let keys = StrKeys::random(&mut rng, n, 1, 24);
            string_pipeline(p, Some(SH_DEFAULT), &keys, &format!("cfg31 n={n}"));
        }
    }
}

#[test]
fn cfg32_string_sh_strdup() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 32);
    unsafe {
        for n in [1usize, 2, 7, 8, 9, 32, 200] {
            let keys = StrKeys::random(&mut rng, n, 1, 24);
            string_pipeline(p, Some(SH_STRDUP), &keys, &format!("cfg32 n={n}"));
        }
    }
}

#[test]
fn cfg33_string_sh_arena() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 33);
    unsafe {
        for n in [1usize, 2, 7, 8, 9, 32, 200] {
            let keys = StrKeys::random(&mut rng, n, 1, 24);
            string_pipeline(p, Some(SH_ARENA), &keys, &format!("cfg33 n={n}"));
        }
    }
}

#[test]
fn cfg34_string_mode_with_sh_none() {
    let _g = serial();
    // string.mode == SH_NONE takes the `default:` memcpy branch even though
    // `mode == STBDS_HM_STRING`, so keys are compared with strcmp but stored
    // as raw bytes.  Keys must be >= keysize bytes long for this to be sane.
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 34);
    unsafe {
        for n in [1usize, 4, 9] {
            let keys = StrKeys::random(&mut rng, n, 12, 20);
            let mut b = Both::shmode(p, 24, 8, STBDS_HM_STRING, SH_NONE, &format!("cfg34 n={n}"));
            b.put_default(0xB1);
            for i in 0..n {
                b.put(keys.ptr(i), i as u8);
            }
            b.free();
        }
    }
}

#[test]
fn cfg35_duplicate_keys_distinct_pointers_and_temp_key() {
    let _g = serial();
    let p = pair();
    unsafe {
        for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
            let tag = format!("cfg35 sh={sh:?}");
            let a = CString::new("alpha").unwrap();
            let a2 = CString::new("alpha").unwrap(); // same content, other pointer
            let bkey = CString::new("beta").unwrap();
            assert_ne!(a.as_ptr(), a2.as_ptr());

            let mut b = match sh {
                None => Both::null(p, 16, 8, STBDS_HM_STRING, &tag),
                Some(m) => Both::shmode(p, 16, 8, STBDS_HM_STRING, m, &tag),
            };
            b.put_default(0x21);
            b.put(a.as_ptr() as *mut c_void, 0x31);
            b.put(bkey.as_ptr() as *mut c_void, 0x32);
            // now compare table->temp_key -- it has definitely been written
            let tk_c = temp_key_str(&b.c);
            let tk_r = temp_key_str(&b.r);
            assert_eq!(tk_c, tk_r, "{tag}: temp_key after fresh put");

            b.put(a2.as_ptr() as *mut c_void, 0x33); // existing-key branch
            let tk_c = temp_key_str(&b.c);
            let tk_r = temp_key_str(&b.r);
            assert_eq!(tk_c, tk_r, "{tag}: temp_key after existing-key put");
            b.free();
        }
    }
}

unsafe fn temp_key_str(m: &Map) -> String {
    let h = &*m.hdr();
    let t = h.hash_table as *const HashIndex;
    if t.is_null() {
        return "<no table>".to_string();
    }
    if (*t).string.mode == 0 {
        return "<raw mode>".to_string();
    }
    let p = (*t).temp_key;
    if p.is_null() {
        "<null>".to_string()
    } else {
        format!("{:?}", CStr::from_ptr(p))
    }
}

#[test]
fn cfg35b_temp_key_after_every_put_randomized() {
    // `stbds_hmput_key` writes `table->temp_key` in three of its four exits and
    // deliberately does NOT write it in the wrap-around inner loop.  A single
    // hand-picked key never reaches that loop, so drive many randomized op
    // streams and compare `temp_key` after every op.
    //
    // `stbds_make_hash_index` never initialises `temp_key`, so a freshly
    // created / grown / shrunk / rebuilt table leaves it as whatever `realloc`
    // returned.  To make the field comparable we seed it with the SAME sentinel
    // string in both libraries every time the table object changes identity.
    // Anything the libraries then write (or leave alone) is observable.
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 0x35b);
    let sentinel = CString::new("<<unwritten>>").unwrap();
    unsafe {
        let mut compared = 0usize;
        let mut writes_seen = 0usize;
        for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
            for t in 0..8 {
                // short keys over a small alphabet -> lots of bucket collisions
                let n = 90;
                let keys = StrKeys::random(&mut rng, n, 1, 4);
                // Distinct buffers holding the SAME bytes, so a stale vs. a
                // refreshed `temp_key` is distinguishable.  These must outlive
                // the map: in SH_DEFAULT mode the library stores keys BY
                // POINTER, so a dropped buffer would leave it holding a
                // dangling pointer.
                let dups: Vec<CString> =
                    keys.v.iter().map(|k| CString::new(k.to_bytes()).unwrap()).collect();
                let tag = format!("cfg35b sh={sh:?} t={t}");
                let mut b = match sh {
                    None => Both::null(p, 16, 8, STBDS_HM_STRING, &tag),
                    Some(m) => Both::shmode(p, 16, 8, STBDS_HM_STRING, m, &tag),
                };
                b.put_default(0x91);
                b.seed_temp_key(&sentinel);
                for step in 0..500usize {
                    let i = rng.below(keys.v.len());
                    let kp = if step % 2 == 0 {
                        keys.ptr(i)
                    } else {
                        dups[i].as_ptr() as *mut c_void
                    };
                    let ct_before = b.c.table_ptr();
                    let rt_before = b.r.table_ptr();
                    // Only `put` can write temp_key, and only `del` can free the
                    // buffer temp_key points at (STRDUP mode frees the key), so
                    // reseed after every del and compare only after puts.
                    let was_put = match rng.below(8) {
                        0 => {
                            b.del(kp);
                            false
                        }
                        1 => {
                            b.get(kp);
                            true // get never touches temp_key: value must persist
                        }
                        _ => {
                            b.put(kp, step as u8);
                            true
                        }
                    };
                    assert_eq!(
                        b.c.table_ptr() != ct_before,
                        b.r.table_ptr() != rt_before,
                        "{tag}: table (re)creation diverged at step {step}"
                    );
                    if !was_put || b.c.table_ptr() != ct_before {
                        // a del may have freed the pointee, or the table object
                        // was replaced (temp_key is then uninitialised memory)
                        b.seed_temp_key(&sentinel);
                        continue;
                    }
                    let ck = temp_key_str(&b.c);
                    let rk = temp_key_str(&b.r);
                    assert_eq!(ck, rk, "{tag}: temp_key diverged at step {step}");
                    compared += 1;
                    if ck != "\"<<unwritten>>\"" && ck != "<raw mode>" {
                        writes_seen += 1;
                    }
                }
                b.free();
            }
        }
        assert!(compared > 5000, "temp_key compared only {compared} times");
        assert!(
            writes_seen > 1000,
            "the libraries only wrote temp_key {writes_seen} times -- the test would be vacuous"
        );
    }
}

#[test]
fn cfg36_common_prefix_keys() {    let _g = serial();
    let p = pair();
    unsafe {
        for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
            for n in [2usize, 12, 40, 150] {
                let keys = StrKeys::prefixed(n);
                string_pipeline(p, sh, &keys, &format!("cfg36 sh={sh:?} n={n}"));
            }
        }
    }
}

#[test]
fn cfg37_empty_string_key() {
    let _g = serial();
    let p = pair();
    unsafe {
        for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
            let tag = format!("cfg37 sh={sh:?}");
            let e = CString::new("").unwrap();
            let x = CString::new("x").unwrap();
            let mut b = match sh {
                None => Both::null(p, 16, 8, STBDS_HM_STRING, &tag),
                Some(m) => Both::shmode(p, 16, 8, STBDS_HM_STRING, m, &tag),
            };
            b.put_default(0x41);
            b.put(e.as_ptr() as *mut c_void, 0x42);
            b.get(e.as_ptr() as *mut c_void);
            b.put(x.as_ptr() as *mut c_void, 0x43);
            b.get(e.as_ptr() as *mut c_void);
            b.del(e.as_ptr() as *mut c_void);
            b.get(e.as_ptr() as *mut c_void);
            b.free();
        }
    }
}

#[test]
fn cfg38_long_string_keys() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 38);
    unsafe {
        for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
            let tag = format!("cfg38 sh={sh:?}");
            let mut ks = Vec::new();
            for len in [600usize, 2000, 600, 40] {
                ks.push(rng.cstring(len));
            }
            let keys = StrKeys { v: ks };
            string_pipeline(p, sh, &keys, &tag);
        }
    }
}

#[test]
fn cfg39_40_string_del_all_and_free() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 39);
    unsafe {
        for sh in [Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
            for n in [9usize, 40, 200] {
                let keys = StrKeys::random(&mut rng, n, 1, 30);
                let tag = format!("cfg39 sh={sh:?} n={n}");
                let mut b = Both::shmode(p, 16, 8, STBDS_HM_STRING, sh.unwrap(), &tag);
                b.put_default(0x51);
                for i in 0..n {
                    b.put(keys.ptr(i), i as u8);
                }
                for i in 0..n {
                    b.del(keys.ptr(i));
                }
                b.free();

                // and: free while still populated (row 40)
                let tag = format!("cfg40 sh={sh:?} n={n}");
                let mut b = Both::shmode(p, 16, 8, STBDS_HM_STRING, sh.unwrap(), &tag);
                for i in 0..n {
                    b.put(keys.ptr(i), i as u8);
                }
                b.free();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 41-44: randomized op streams, whole table compared after every op
// ---------------------------------------------------------------------------

#[test]
fn cfg41_random_stream_binary() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 41);
    unsafe {
        for t in 0..4 {
            let n = 60;
            let mut keys = BinKeys::new(&mut rng, n, 8);
            let mut b = Both::null(p, 16, 8, STBDS_HM_BINARY, &format!("cfg41 t={t}"));
            for step in 0..400 {
                let i = rng.below(n);
                let k = keys.ptr(i);
                match rng.below(6) {
                    0 | 1 => b.put(k, step as u8),
                    2 => {
                        b.get(k);
                    }
                    3 => {
                        b.get_ts(k);
                    }
                    4 => {
                        b.del(k);
                    }
                    _ => b.put_default(step as u8),
                }
            }
            b.free();
        }
    }
}

unsafe fn random_stream_string(p: &Pair, sh: Option<c_int>, tag: &str, rng: &mut Rng) {
    let n = 60;
    let keys = StrKeys::random(rng, n, 1, 20);
    let mut b = match sh {
        None => Both::null(p, 16, 8, STBDS_HM_STRING, tag),
        Some(m) => Both::shmode(p, 16, 8, STBDS_HM_STRING, m, tag),
    };
    for step in 0..400 {
        let i = rng.below(n);
        let k = keys.ptr(i);
        match rng.below(6) {
            0 | 1 => b.put(k, step as u8),
            2 => {
                b.get(k);
            }
            3 => {
                b.get_ts(k);
            }
            4 => {
                b.del(k);
            }
            _ => b.put_default(step as u8),
        }
    }
    b.free();
}

#[test]
fn cfg42_random_stream_string_default() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 42);
    unsafe {
        for t in 0..4 {
            random_stream_string(p, Some(SH_DEFAULT), &format!("cfg42 t={t}"), &mut rng);
            random_stream_string(p, None, &format!("cfg42b t={t}"), &mut rng);
        }
    }
}

#[test]
fn cfg43_random_stream_string_strdup() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 43);
    unsafe {
        for t in 0..4 {
            random_stream_string(p, Some(SH_STRDUP), &format!("cfg43 t={t}"), &mut rng);
        }
    }
}

#[test]
fn cfg44_random_stream_string_arena() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 44);
    unsafe {
        for t in 0..4 {
            random_stream_string(p, Some(SH_ARENA), &format!("cfg44 t={t}"), &mut rng);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 59-60
// ---------------------------------------------------------------------------

#[test]
fn cfg59_pipeline_under_various_global_seeds() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 59);
    unsafe {
        for seed in [0usize, 1, usize::MAX, 0xdead_beef, rng.next_u64() as usize] {
            for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
                (p.c.rand_seed)(seed);
                (p.r.rand_seed)(seed);
                let keys = StrKeys::random(&mut rng, 40, 1, 20);
                let tag = format!("cfg59 seed={seed:#x} sh={sh:?}");
                // NOTE: Both::* resets the seed, so drive the maps directly.
                let elemsize = 16usize;
                let mut b = match sh {
                    None => Both {
                        c: Map::null(&p.c, elemsize, 8, STBDS_HM_STRING),
                        r: Map::null(&p.r, elemsize, 8, STBDS_HM_STRING),
                        what: tag.clone(),
                        step: 0,
                    },
                    Some(m) => Both {
                        c: Map::shmode(&p.c, elemsize, 8, STBDS_HM_STRING, m),
                        r: Map::shmode(&p.r, elemsize, 8, STBDS_HM_STRING, m),
                        what: tag.clone(),
                        step: 0,
                    },
                };
                b.put_default(0x61);
                for i in 0..keys.v.len() {
                    b.put(keys.ptr(i), i as u8);
                }
                for i in 0..keys.v.len() {
                    b.get(keys.ptr(i));
                }
                for i in 0..keys.v.len() {
                    b.del(keys.ptr(i));
                }
                b.free();
            }
        }
    }
}

#[test]
fn cfg60_put_default_before_put() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 60);
    unsafe {
        // binary
        let mut keys = BinKeys::new(&mut rng, 12, 8);
        let mut b = Both::null(p, 16, 8, STBDS_HM_BINARY, "cfg60 binary");
        b.put_default(0x71);
        for i in 0..12 {
            b.put(keys.ptr(i), i as u8);
        }
        b.free();
        // string
        let keys = StrKeys::random(&mut rng, 12, 1, 16);
        let mut b = Both::null(p, 16, 8, STBDS_HM_STRING, "cfg60 string");
        b.put_default(0x72);
        for i in 0..12 {
            b.put(keys.ptr(i), i as u8);
        }
        b.free();
    }
}

// ---------------------------------------------------------------------------
// extra: non-zero keyoffset through stbds_hmdel_key (public ABI parameter that
// the convenience macros always pass as 0)
// ---------------------------------------------------------------------------

#[test]
fn cfg_extra_del_nonzero_keyoffset() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 0x6b);
    unsafe {
        for n in [4usize, 12, 40] {
            let mut keys = BinKeys::new(&mut rng, n, 4);
            let mut b = Both::null(p, 16, 4, STBDS_HM_BINARY, &format!("cfg_extra ko n={n}"));
            b.put_default(0x81);
            for i in 0..n {
                b.put(keys.ptr(i), i as u8);
            }
            for off in [4usize, 8, 12] {
                for i in 0..n {
                    b.del_off(keys.ptr(i), off);
                }
            }
            b.free();
        }
    }
}
