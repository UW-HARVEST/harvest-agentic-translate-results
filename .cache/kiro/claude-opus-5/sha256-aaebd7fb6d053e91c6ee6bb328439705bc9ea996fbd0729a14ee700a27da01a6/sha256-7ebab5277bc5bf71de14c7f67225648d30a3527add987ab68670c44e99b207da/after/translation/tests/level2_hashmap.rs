//! Level 2: the hash-map core —
//! `stbds_shmode_func`, `stbds_hmput_default`, `stbds_hmput_key`,
//! `stbds_hmget_key`, `stbds_hmget_key_ts`, `stbds_hmdel_key`,
//! `stbds_hmfree_func`.
//!
//! Every call is made through the loaded `.so` exports. The C macros
//! (`shput`, `shget`, `shdel`, ...) are re-implemented here so that both
//! libraries are driven with exactly the same call sequence — this matters
//! because `stbds_hash_seed` is a mutable global that each new hash index
//! advances.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// String-keyed map driver (mirrors the `sh*` macros)
// ---------------------------------------------------------------------------

struct SMap<'a> {
    api: &'a Api,
    t: *mut StrMapEntry,
}

impl<'a> SMap<'a> {
    fn new(api: &'a Api) -> Self {
        SMap {
            api,
            t: std::ptr::null_mut(),
        }
    }

    fn temp(&self) -> isize {
        unsafe { header(self.t.sub(1) as *mut c_void).temp }
    }

    fn shgeti(&mut self, k: *mut c_char) -> isize {
        unsafe {
            self.t = (self.api.hmget_key)(
                self.t as *mut c_void,
                ELEMSIZE,
                k as *mut c_void,
                KEYSIZE,
                STBDS_HM_STRING,
            ) as *mut StrMapEntry;
            self.temp()
        }
    }

    fn shgeti_ts(&mut self, k: *mut c_char) -> isize {
        unsafe {
            let mut tmp: isize = 0x5a5a;
            self.t = (self.api.hmget_key_ts)(
                self.t as *mut c_void,
                ELEMSIZE,
                k as *mut c_void,
                KEYSIZE,
                &mut tmp,
                STBDS_HM_STRING,
            ) as *mut StrMapEntry;
            tmp
        }
    }

    fn shput(&mut self, k: *mut c_char, v: c_int) {
        unsafe {
            self.t = (self.api.hmput_key)(
                self.t as *mut c_void,
                ELEMSIZE,
                k as *mut c_void,
                KEYSIZE,
                STBDS_HM_STRING,
            ) as *mut StrMapEntry;
            let idx = self.temp();
            (*self.t.offset(idx)).value = v;
        }
    }

    fn shget(&mut self, k: *mut c_char) -> c_int {
        unsafe {
            self.shgeti(k);
            (*self.t.offset(self.temp())).value
        }
    }

    fn shdel(&mut self, k: *mut c_char) -> isize {
        unsafe {
            self.t = (self.api.hmdel_key)(
                self.t as *mut c_void,
                ELEMSIZE,
                k as *mut c_void,
                KEYSIZE,
                0,
                STBDS_HM_STRING,
            ) as *mut StrMapEntry;
            if self.t.is_null() {
                0
            } else {
                self.temp()
            }
        }
    }

    fn shdefault(&mut self, v: c_int) {
        unsafe {
            self.t = (self.api.hmput_default)(self.t as *mut c_void, ELEMSIZE) as *mut StrMapEntry;
            (*self.t.offset(-1)).value = v;
        }
    }

    fn shlen(&self) -> isize {
        unsafe {
            if self.t.is_null() {
                0
            } else {
                header(self.t.sub(1) as *mut c_void).length as isize - 1
            }
        }
    }

    fn sh_new(&mut self, mode: c_int) {
        unsafe {
            self.t = (self.api.shmode_func)(ELEMSIZE, mode) as *mut StrMapEntry;
        }
    }

    fn free(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.api.hmfree_func)(self.t.sub(1) as *mut c_void, ELEMSIZE);
            }
            self.t = std::ptr::null_mut();
        }
    }

    fn snap(&self) -> MapSnapshot {
        unsafe { snapshot_map(self.t) }
    }
}

/// Two keys with identical contents but independent storage — the string modes
/// that keep the caller's pointer must not make the two libraries alias.
struct KeyPair {
    c: CStr8,
    r: CStr8,
}

impl KeyPair {
    fn new(s: &str) -> KeyPair {
        KeyPair {
            c: CStr8::new(s),
            r: CStr8::new(s),
        }
    }
}

struct Keys {
    keys: Vec<KeyPair>,
}

impl Keys {
    fn new(n: usize) -> Keys {
        Keys {
            keys: (0..n).map(|i| KeyPair::new(&format!("test_{}", i))).collect(),
        }
    }
    fn c(&mut self, i: usize) -> *mut c_char {
        self.keys[i].c.as_ptr()
    }
    fn r(&mut self, i: usize) -> *mut c_char {
        self.keys[i].r.as_ptr()
    }
}

fn assert_same(step: &str, a: &SMap, b: &SMap) {
    let sa = a.snap();
    let sb = b.snap();
    assert_eq!(sa, sb, "map state mismatch after {}", step);
}

// ---------------------------------------------------------------------------
// stbds_shmode_func
// ---------------------------------------------------------------------------

#[test]
fn shmode_func_all_modes() {
    let _g = serial();
    let (c, r) = apis();

    for mode in [
        STBDS_SH_NONE,
        STBDS_SH_DEFAULT,
        STBDS_SH_STRDUP,
        STBDS_SH_ARENA,
        7,
        255,
    ] {
        for elemsize in [8usize, 12, 16, 24] {
            reset_seeds(&c, &r, DEFAULT_SEED);
            let tc = unsafe { (c.shmode_func)(elemsize, mode) };
            let tr = unsafe { (r.shmode_func)(elemsize, mode) };
            let hc = unsafe { header((tc as *mut u8).sub(elemsize) as *mut c_void) };
            let hr = unsafe { header((tr as *mut u8).sub(elemsize) as *mut c_void) };
            assert_eq!((hc.length, hc.capacity, hc.temp), (hr.length, hr.capacity, hr.temp));
            let ic = unsafe { snapshot_index(hc.hash_table as *mut HashIndex) };
            let ir = unsafe { snapshot_index(hr.hash_table as *mut HashIndex) };
            assert_eq!(ic, ir, "shmode_func({}, {}) index mismatch", elemsize, mode);
            unsafe {
                (c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hmput_default / stbds_hmget_key(_ts) edge cases
// ---------------------------------------------------------------------------

#[test]
fn hmput_default_on_null_and_existing() {
    let _g = serial();
    let (c, r) = apis();
    reset_seeds(&c, &r, DEFAULT_SEED);

    let mut mc = SMap::new(&c);
    let mut mr = SMap::new(&r);

    mc.shdefault(-2);
    mr.shdefault(-2);
    assert_same("shdefault on NULL", &mc, &mr);

    // second call must be a no-op apart from the value store
    mc.shdefault(-7);
    mr.shdefault(-7);
    assert_same("shdefault twice", &mc, &mr);

    mc.free();
    mr.free();
}

#[test]
fn hmget_key_on_null_and_tableless() {
    let _g = serial();
    let (c, r) = apis();
    reset_seeds(&c, &r, DEFAULT_SEED);

    let mut foo = KeyPair::new("foo");
    let mut mc = SMap::new(&c);
    let mut mr = SMap::new(&r);

    // NULL map: allocates the sentinel element, temp = -1
    let a = mc.shgeti(foo.c.as_ptr());
    let b = mr.shgeti(foo.r.as_ptr());
    assert_eq!(a, -1);
    assert_eq!(a, b, "shgeti on NULL map");
    assert_same("shgeti on NULL", &mc, &mr);

    // now the array exists but has no hash table -> temp = -1
    let a = mc.shgeti(foo.c.as_ptr());
    let b = mr.shgeti(foo.r.as_ptr());
    assert_eq!(a, -1);
    assert_eq!(a, b, "shgeti on table-less map");
    assert_same("shgeti on table-less", &mc, &mr);

    // _ts variant on the same states
    let a = mc.shgeti_ts(foo.c.as_ptr());
    let b = mr.shgeti_ts(foo.r.as_ptr());
    assert_eq!((a, b), (-1, -1));
    assert_same("shgeti_ts on table-less", &mc, &mr);

    mc.free();
    mr.free();

    // _ts on a fresh NULL map
    let mut mc = SMap::new(&c);
    let mut mr = SMap::new(&r);
    let a = mc.shgeti_ts(foo.c.as_ptr());
    let b = mr.shgeti_ts(foo.r.as_ptr());
    assert_eq!((a, b), (-1, -1));
    assert_same("shgeti_ts on NULL", &mc, &mr);
    mc.free();
    mr.free();
}

#[test]
fn hmdel_key_on_null_and_tableless() {
    let _g = serial();
    let (c, r) = apis();
    reset_seeds(&c, &r, DEFAULT_SEED);

    let mut foo = KeyPair::new("foo");
    let mut mc = SMap::new(&c);
    let mut mr = SMap::new(&r);

    // NULL map -> returns NULL, temp untouched
    assert_eq!(mc.shdel(foo.c.as_ptr()), 0);
    assert_eq!(mr.shdel(foo.r.as_ptr()), 0);
    assert!(mc.t.is_null() && mr.t.is_null());

    // table-less map
    mc.shgeti(foo.c.as_ptr());
    mr.shgeti(foo.r.as_ptr());
    let a = mc.shdel(foo.c.as_ptr());
    let b = mr.shdel(foo.r.as_ptr());
    assert_eq!(a, b, "shdel on table-less map");
    assert_same("shdel on table-less", &mc, &mr);
    mc.free();
    mr.free();
}

// ---------------------------------------------------------------------------
// String maps in every arena mode
// ---------------------------------------------------------------------------

fn string_map_workout(mode: Option<c_int>, num: usize, seed: usize) {
    let (c, r) = apis();
    reset_seeds(&c, &r, seed);

    let mut keys = Keys::new(num + 8);
    let mut mc = SMap::new(&c);
    let mut mr = SMap::new(&r);

    if let Some(m) = mode {
        mc.sh_new(m);
        mr.sh_new(m);
        assert_same("sh_new", &mc, &mr);
    }

    mc.shdefault(-2);
    mr.shdefault(-2);
    assert_same("shdefault", &mc, &mr);

    // inserts
    for i in (0..num).step_by(2) {
        mc.shput(keys.c(i), (i as c_int).wrapping_mul(3));
        mr.shput(keys.r(i), (i as c_int).wrapping_mul(3));
        assert_same(&format!("shput {}", i), &mc, &mr);
        assert_eq!(
            unsafe { temp_key(mc.t) },
            unsafe { temp_key(mr.t) },
            "temp_key after shput {}",
            i
        );
    }
    assert_eq!(mc.shlen(), mr.shlen());

    // overwrite existing keys (hits the "key already present" path, which also
    // stores temp_key in the first probe loop)
    for i in (0..num).step_by(6) {
        mc.shput(keys.c(i), (i as c_int).wrapping_mul(5));
        mr.shput(keys.r(i), (i as c_int).wrapping_mul(5));
        assert_same(&format!("shput overwrite {}", i), &mc, &mr);
        assert_eq!(
            unsafe { temp_key(mc.t) },
            unsafe { temp_key(mr.t) },
            "temp_key after overwrite {}",
            i
        );
    }

    // lookups (present and absent)
    for i in 0..num + 8 {
        let a = mc.shget(keys.c(i));
        let b = mr.shget(keys.r(i));
        assert_eq!(a, b, "shget({}) mismatch", i);
        assert_same(&format!("shget {}", i), &mc, &mr);
    }

    // deletions: every 4th, then all
    for i in (2..num).step_by(4) {
        let a = mc.shdel(keys.c(i));
        let b = mr.shdel(keys.r(i));
        assert_eq!(a, b, "shdel({}) mismatch", i);
        assert_same(&format!("shdel {}", i), &mc, &mr);
    }
    for i in 0..num {
        let a = mc.shget(keys.c(i));
        let b = mr.shget(keys.r(i));
        assert_eq!(a, b, "post-del shget({})", i);
    }
    assert_same("post-del gets", &mc, &mr);

    for i in 0..num {
        let a = mc.shdel(keys.c(i));
        let b = mr.shdel(keys.r(i));
        assert_eq!(a, b, "shdel all ({})", i);
        assert_same(&format!("shdel all {}", i), &mc, &mr);
    }
    for i in 0..num {
        assert_eq!(mc.shget(keys.c(i)), mr.shget(keys.r(i)));
    }
    assert_same("after deleting everything", &mc, &mr);

    // re-insert after mass deletion so the tombstone/shrink paths are used
    for i in 0..num.min(24) {
        mc.shput(keys.c(i), i as c_int);
        mr.shput(keys.r(i), i as c_int);
        assert_same(&format!("re-insert {}", i), &mc, &mr);
    }

    mc.free();
    mr.free();
}

#[test]
fn string_map_default_mode() {
    let _g = serial();
    for &num in &[0usize, 1, 2, 3, 8, 16, 17, 64, 200] {
        string_map_workout(None, num, DEFAULT_SEED);
    }
}

#[test]
fn string_map_strdup_mode() {
    let _g = serial();
    for &num in &[0usize, 1, 2, 3, 8, 16, 17, 64, 200] {
        string_map_workout(Some(STBDS_SH_STRDUP), num, DEFAULT_SEED);
    }
}

#[test]
fn string_map_arena_mode() {
    let _g = serial();
    for &num in &[0usize, 1, 2, 3, 8, 16, 17, 64, 200] {
        string_map_workout(Some(STBDS_SH_ARENA), num, DEFAULT_SEED);
    }
}

// `STBDS_SH_NONE` combined with `STBDS_HM_STRING` is not a legal combination in
// the original library: `hmput_key` would `memcpy` the *string bytes* over the
// element's `char *key` field and the next `strcmp` would dereference that as a
// pointer. It is therefore not exercised here — `sh_new_strdup` / `sh_new_arena`
// are the only ways a string map is created.

#[test]
fn string_map_varying_seeds() {
    let _g = serial();
    for &seed in &[0usize, 1, 0xffff_ffff_ffff_ffff, 0x1234_5678, 0xdead] {
        string_map_workout(None, 48, seed);
        string_map_workout(Some(STBDS_SH_STRDUP), 48, seed);
        string_map_workout(Some(STBDS_SH_ARENA), 48, seed);
    }
}

#[test]
fn string_map_large() {
    let _g = serial();
    // Big enough to force several table doublings and shrinks.
    let (c, r) = apis();
    reset_seeds(&c, &r, DEFAULT_SEED);
    let num = 2000usize;
    let mut keys = Keys::new(num);
    let mut mc = SMap::new(&c);
    let mut mr = SMap::new(&r);
    mc.sh_new(STBDS_SH_STRDUP);
    mr.sh_new(STBDS_SH_STRDUP);
    mc.shdefault(-2);
    mr.shdefault(-2);
    for i in 0..num {
        mc.shput(keys.c(i), i as c_int);
        mr.shput(keys.r(i), i as c_int);
    }
    assert_same("bulk insert", &mc, &mr);
    for i in 0..num {
        assert_eq!(mc.shget(keys.c(i)), mr.shget(keys.r(i)), "get {}", i);
    }
    assert_same("bulk get", &mc, &mr);
    for i in (0..num).step_by(3) {
        assert_eq!(mc.shdel(keys.c(i)), mr.shdel(keys.r(i)), "del {}", i);
    }
    assert_same("bulk del", &mc, &mr);
    for i in 0..num {
        assert_eq!(mc.shget(keys.c(i)), mr.shget(keys.r(i)), "get2 {}", i);
    }
    for i in 0..num {
        assert_eq!(mc.shdel(keys.c(i)), mr.shdel(keys.r(i)), "del2 {}", i);
    }
    assert_same("delete all", &mc, &mr);
    mc.free();
    mr.free();
}

// ---------------------------------------------------------------------------
// Binary-keyed maps
// ---------------------------------------------------------------------------

struct BMap<'a> {
    api: &'a Api,
    t: *mut u8,
    elemsize: usize,
    keysize: usize,
}

impl<'a> BMap<'a> {
    fn new(api: &'a Api, elemsize: usize, keysize: usize) -> Self {
        BMap {
            api,
            t: std::ptr::null_mut(),
            elemsize,
            keysize,
        }
    }

    fn raw(&self) -> *mut c_void {
        unsafe { self.t.sub(self.elemsize) as *mut c_void }
    }

    fn temp(&self) -> isize {
        unsafe { header(self.raw()).temp }
    }

    fn hmput(&mut self, key: &[u8], payload: &[u8]) {
        assert_eq!(key.len(), self.keysize);
        assert_eq!(key.len() + payload.len(), self.elemsize);
        unsafe {
            let mut k = key.to_vec();
            self.t = (self.api.hmput_key)(
                self.t as *mut c_void,
                self.elemsize,
                k.as_mut_ptr() as *mut c_void,
                self.keysize,
                STBDS_HM_BINARY,
            ) as *mut u8;
            let idx = self.temp();
            let e = self.t.offset(idx * self.elemsize as isize);
            std::ptr::copy_nonoverlapping(key.as_ptr(), e, self.keysize);
            std::ptr::copy_nonoverlapping(payload.as_ptr(), e.add(self.keysize), payload.len());
        }
    }

    fn hmgeti(&mut self, key: &[u8]) -> isize {
        unsafe {
            let mut k = key.to_vec();
            self.t = (self.api.hmget_key)(
                self.t as *mut c_void,
                self.elemsize,
                k.as_mut_ptr() as *mut c_void,
                self.keysize,
                STBDS_HM_BINARY,
            ) as *mut u8;
            self.temp()
        }
    }

    fn hmdel(&mut self, key: &[u8]) -> isize {
        unsafe {
            let mut k = key.to_vec();
            self.t = (self.api.hmdel_key)(
                self.t as *mut c_void,
                self.elemsize,
                k.as_mut_ptr() as *mut c_void,
                self.keysize,
                0,
                STBDS_HM_BINARY,
            ) as *mut u8;
            if self.t.is_null() {
                0
            } else {
                self.temp()
            }
        }
    }

    fn hmdefault(&mut self, payload: &[u8]) {
        unsafe {
            self.t =
                (self.api.hmput_default)(self.t as *mut c_void, self.elemsize) as *mut u8;
            let e = self.t.offset(-(self.elemsize as isize));
            std::ptr::copy_nonoverlapping(payload.as_ptr(), e.add(self.keysize), payload.len());
        }
    }

    fn free(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.api.hmfree_func)(self.raw(), self.elemsize);
            }
            self.t = std::ptr::null_mut();
        }
    }

    /// Header (minus addresses) + hash index + raw element bytes.
    fn snap(&self) -> (usize, usize, isize, Option<HashIndexSnapshot>, Vec<u8>) {
        unsafe {
            if self.t.is_null() {
                return (0, 0, 0, None, Vec::new());
            }
            let h = header(self.raw());
            let idx = snapshot_index(h.hash_table as *mut HashIndex);
            let bytes = std::slice::from_raw_parts(
                self.t.sub(self.elemsize),
                h.length * self.elemsize,
            )
            .to_vec();
            (h.length, h.capacity, h.temp, idx, bytes)
        }
    }
}

fn binary_map_workout(elemsize: usize, keysize: usize, num: u32, seed: usize) {
    let (c, r) = apis();
    reset_seeds(&c, &r, seed);
    let mut mc = BMap::new(&c, elemsize, keysize);
    let mut mr = BMap::new(&r, elemsize, keysize);
    let paylen = elemsize - keysize;

    let key = |i: u32| -> Vec<u8> {
        let mut v = Vec::with_capacity(keysize);
        for b in 0..keysize {
            v.push(((i as usize).wrapping_mul(2654435761).wrapping_add(b * 7) >> (b % 4 * 8)) as u8);
        }
        v
    };
    let pay = |i: u32| -> Vec<u8> { (0..paylen).map(|b| (i as usize + b) as u8).collect() };

    mc.hmdefault(&pay(0xdead));
    mr.hmdefault(&pay(0xdead));
    assert_eq!(mc.snap(), mr.snap(), "hmdefault");

    for i in 0..num {
        mc.hmput(&key(i), &pay(i));
        mr.hmput(&key(i), &pay(i));
        assert_eq!(mc.snap(), mr.snap(), "hmput {} (es={})", i, elemsize);
    }
    for i in 0..num + 5 {
        let a = mc.hmgeti(&key(i));
        let b = mr.hmgeti(&key(i));
        assert_eq!(a, b, "hmgeti {}", i);
        assert_eq!(mc.snap(), mr.snap(), "hmgeti state {}", i);
    }
    // overwrite
    for i in (0..num).step_by(3) {
        mc.hmput(&key(i), &pay(i + 1000));
        mr.hmput(&key(i), &pay(i + 1000));
        assert_eq!(mc.snap(), mr.snap(), "hmput overwrite {}", i);
    }
    // delete some, then all
    for i in (0..num).step_by(2) {
        let a = mc.hmdel(&key(i));
        let b = mr.hmdel(&key(i));
        assert_eq!(a, b, "hmdel {}", i);
        assert_eq!(mc.snap(), mr.snap(), "hmdel state {}", i);
    }
    for i in 0..num {
        let a = mc.hmdel(&key(i));
        let b = mr.hmdel(&key(i));
        assert_eq!(a, b, "hmdel all {}", i);
        assert_eq!(mc.snap(), mr.snap(), "hmdel all state {}", i);
    }
    for i in 0..num {
        assert_eq!(mc.hmgeti(&key(i)), mr.hmgeti(&key(i)));
    }
    assert_eq!(mc.snap(), mr.snap(), "final");
    mc.free();
    mr.free();
}

#[test]
fn binary_map_int_key() {
    let _g = serial();
    for &num in &[0u32, 1, 2, 7, 8, 9, 33, 150] {
        binary_map_workout(8, 4, num, DEFAULT_SEED);
    }
}

#[test]
fn binary_map_two_int_key() {
    let _g = serial();
    for &num in &[0u32, 1, 8, 40, 300] {
        binary_map_workout(12, 8, num, DEFAULT_SEED);
    }
}

#[test]
fn binary_map_large_elem() {
    let _g = serial();
    binary_map_workout(32, 16, 120, DEFAULT_SEED);
    binary_map_workout(64, 8, 90, DEFAULT_SEED);
}

#[test]
fn binary_map_varying_seeds() {
    let _g = serial();
    for &seed in &[0usize, 1, usize::MAX, 0xcafe_f00d] {
        binary_map_workout(8, 4, 70, seed);
        binary_map_workout(12, 8, 70, seed);
    }
}

// ---------------------------------------------------------------------------
// stbds_hmfree_func
// ---------------------------------------------------------------------------

#[test]
fn hmfree_func_null_is_noop() {
    let _g = serial();
    let (c, r) = apis();
    unsafe {
        (c.hmfree_func)(std::ptr::null_mut(), ELEMSIZE);
        (r.hmfree_func)(std::ptr::null_mut(), ELEMSIZE);
    }
}

#[test]
fn hmfree_func_frees_all_modes() {
    let _g = serial();
    let (c, r) = apis();
    for mode in [None, Some(STBDS_SH_STRDUP), Some(STBDS_SH_ARENA)] {
        reset_seeds(&c, &r, DEFAULT_SEED);
        let mut keys = Keys::new(40);
        let mut mc = SMap::new(&c);
        let mut mr = SMap::new(&r);
        if let Some(m) = mode {
            mc.sh_new(m);
            mr.sh_new(m);
        }
        mc.shdefault(-2);
        mr.shdefault(-2);
        for i in 0..40 {
            mc.shput(keys.c(i), i as c_int);
            mr.shput(keys.r(i), i as c_int);
        }
        assert_same("pre-free", &mc, &mr);
        mc.free();
        mr.free();
        assert!(mc.t.is_null() && mr.t.is_null());
    }
}

// ---------------------------------------------------------------------------
// Targeted coverage for the second (wrapped-around) probe loop of
// `stbds_hmput_key`, which — unlike the first loop — does *not* store
// `temp_key` when it finds an existing key.
// ---------------------------------------------------------------------------

#[test]
fn hmput_key_existing_key_temp_key_sweep() {
    let _g = serial();
    let (c, r) = apis();

    // `used_count_threshold == slot_count - slot_count/4`, and growth happens
    // *before* an insert, so `slot_count * 3 / 4` entries leave the table at its
    // maximum density (6/8 slots per bucket on average). That is where the
    // wrapped-around probe loop — which, unlike the first loop, does not store
    // `table->temp_key` when it finds an existing key — is taken most often.
    //
    // No deletes happen here, so `temp_key` is well defined after the first
    // insert and can be compared on every operation.
    for &num in &[6usize, 12, 48, 192, 768, 3072] {
        for mode in [STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            for &seed in &[DEFAULT_SEED, 0usize, 1, 0x9e37_79b9_7f4a_7c15] {
                reset_seeds(&c, &r, seed);
                let mut keys = Keys::new(num);
                let mut mc = SMap::new(&c);
                let mut mr = SMap::new(&r);
                mc.sh_new(mode);
                mr.sh_new(mode);
                mc.shdefault(-2);
                mr.shdefault(-2);
                for i in 0..num {
                    mc.shput(keys.c(i), i as c_int);
                    mr.shput(keys.r(i), i as c_int);
                    assert_eq!(
                        unsafe { temp_key(mc.t) },
                        unsafe { temp_key(mr.t) },
                        "temp_key mismatch inserting key {} (num {}, mode {}, seed {:#x})",
                        i,
                        num,
                        mode,
                        seed
                    );
                }
                assert_same("dense inserts", &mc, &mr);

                // Re-put every key twice: each hits the "already present"
                // branch, in the first or the wrapped-around probe loop
                // depending on where the key ended up.
                for round in 0..2 {
                    for i in 0..num {
                        mc.shput(keys.c(i), (i as c_int) ^ 0x55);
                        mr.shput(keys.r(i), (i as c_int) ^ 0x55);
                        assert_eq!(
                            unsafe { temp_key(mc.t) },
                            unsafe { temp_key(mr.t) },
                            "temp_key mismatch re-putting key {} round {} \
                             (num {}, mode {}, seed {:#x})",
                            i,
                            round,
                            num,
                            mode,
                            seed
                        );
                        assert_eq!(mc.temp(), mr.temp(), "index mismatch re-putting {}", i);
                    }
                }
                // Lookups also traverse the wrapped-around loop of
                // stbds_hm_find_slot.
                for i in 0..num {
                    assert_eq!(mc.shgeti(keys.c(i)), mr.shgeti(keys.r(i)), "geti {}", i);
                }
                assert_same("dense re-puts", &mc, &mr);
                mc.free();
                mr.free();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hmget_key_ts in binary mode
// ---------------------------------------------------------------------------

#[test]
fn hmget_key_ts_binary_mode() {
    let _g = serial();
    let (c, r) = apis();
    reset_seeds(&c, &r, DEFAULT_SEED);

    let elemsize = 8usize;
    let keysize = 4usize;
    let mut mc = BMap::new(&c, elemsize, keysize);
    let mut mr = BMap::new(&r, elemsize, keysize);

    let key = |i: i32| i.to_le_bytes().to_vec();

    // `_ts` on a NULL map: allocates and reports STBDS_INDEX_EMPTY without
    // touching the header's `temp`.
    let mut tc: isize = 0x1234;
    let mut tr: isize = 0x1234;
    unsafe {
        let mut k = key(1);
        mc.t = (c.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            k.as_mut_ptr() as *mut c_void,
            keysize,
            &mut tc,
            STBDS_HM_BINARY,
        ) as *mut u8;
        let mut k = key(1);
        mr.t = (r.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            k.as_mut_ptr() as *mut c_void,
            keysize,
            &mut tr,
            STBDS_HM_BINARY,
        ) as *mut u8;
    }
    assert_eq!((tc, tr), (-1, -1), "_ts on NULL map");
    assert_eq!(mc.snap(), mr.snap(), "_ts on NULL map state");

    for i in 0..80i32 {
        mc.hmput(&key(i), &(i as u32).to_le_bytes());
        mr.hmput(&key(i), &(i as u32).to_le_bytes());
    }
    assert_eq!(mc.snap(), mr.snap(), "after inserts");

    for i in -5..90i32 {
        let mut tc: isize = 0x7777;
        let mut tr: isize = 0x7777;
        unsafe {
            let mut k = key(i);
            mc.t = (c.hmget_key_ts)(
                mc.t as *mut c_void,
                elemsize,
                k.as_mut_ptr() as *mut c_void,
                keysize,
                &mut tc,
                STBDS_HM_BINARY,
            ) as *mut u8;
            let mut k = key(i);
            mr.t = (r.hmget_key_ts)(
                mr.t as *mut c_void,
                elemsize,
                k.as_mut_ptr() as *mut c_void,
                keysize,
                &mut tr,
                STBDS_HM_BINARY,
            ) as *mut u8;
        }
        assert_eq!(tc, tr, "_ts temp mismatch for key {}", i);
        assert_eq!(mc.snap(), mr.snap(), "_ts state for key {}", i);
    }

    mc.free();
    mr.free();
}

// ---------------------------------------------------------------------------
// Interleaved random operations
// ---------------------------------------------------------------------------

#[test]
fn randomised_interleaved_string_ops() {
    let _g = serial();
    let (c, r) = apis();

    for &seed in &[1usize, 12345, 0xfeed_face] {
        reset_seeds(&c, &r, DEFAULT_SEED);
        let universe = 300usize;
        let mut keys = Keys::new(universe);
        let mut mc = SMap::new(&c);
        let mut mr = SMap::new(&r);
        mc.sh_new(STBDS_SH_STRDUP);
        mr.sh_new(STBDS_SH_STRDUP);
        mc.shdefault(-2);
        mr.shdefault(-2);

        // xorshift so both sides see the same operation stream
        let mut state = seed as u64 | 1;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };

        for step in 0..4000usize {
            let x = next();
            let i = (x as usize >> 8) % universe;
            match x % 3 {
                0 => {
                    mc.shput(keys.c(i), i as c_int);
                    mr.shput(keys.r(i), i as c_int);
                }
                1 => {
                    let a = mc.shget(keys.c(i));
                    let b = mr.shget(keys.r(i));
                    assert_eq!(a, b, "step {} get {}", step, i);
                }
                _ => {
                    let a = mc.shdel(keys.c(i));
                    let b = mr.shdel(keys.r(i));
                    assert_eq!(a, b, "step {} del {}", step, i);
                }
            }
            assert_same(&format!("random step {} (op {})", step, x % 3), &mc, &mr);
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn randomised_interleaved_binary_ops() {
    let _g = serial();
    let (c, r) = apis();

    for &(elemsize, keysize) in &[(8usize, 4usize), (12, 8), (24, 16)] {
        reset_seeds(&c, &r, DEFAULT_SEED);
        let mut mc = BMap::new(&c, elemsize, keysize);
        let mut mr = BMap::new(&r, elemsize, keysize);
        let paylen = elemsize - keysize;

        let mut state = 0x2545_F491_4F6C_DD1Du64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };

        mc.hmdefault(&vec![0xabu8; paylen]);
        mr.hmdefault(&vec![0xabu8; paylen]);

        for step in 0..3000usize {
            let x = next();
            let id = (x >> 8) % 200;
            let key: Vec<u8> = (0..keysize).map(|b| (id.wrapping_mul(31) >> (b % 8 * 8)) as u8).collect();
            let payload: Vec<u8> = (0..paylen).map(|b| (id as usize + b) as u8).collect();
            match x % 3 {
                0 => {
                    mc.hmput(&key, &payload);
                    mr.hmput(&key, &payload);
                }
                1 => {
                    assert_eq!(
                        mc.hmgeti(&key),
                        mr.hmgeti(&key),
                        "step {} geti (es={})",
                        step,
                        elemsize
                    );
                }
                _ => {
                    assert_eq!(
                        mc.hmdel(&key),
                        mr.hmdel(&key),
                        "step {} del (es={})",
                        step,
                        elemsize
                    );
                }
            }
            assert_eq!(
                mc.snap(),
                mr.snap(),
                "random binary step {} (es={}, op {})",
                step,
                elemsize,
                x % 3
            );
        }
        mc.free();
        mr.free();
    }
}

// ---------------------------------------------------------------------------
// stbds_hmput_default on an array whose length is still 0
// ---------------------------------------------------------------------------

#[test]
fn hmput_default_on_zero_length_array() {
    let _g = serial();
    let (c, r) = apis();

    // `stbds_hmput_default` has two guards: `a == NULL` *and*
    // `stbds_header(...)->length == 0`. The second one is unreachable through
    // the `hmdefault` macro (every path that creates the array bumps the length
    // to 1 first), but it is reachable for a direct caller of the exported
    // symbol: `stbds_arrgrowf` leaves `length == 0`.
    for elemsize in [8usize, 16, 24] {
        let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        assert_eq!(unsafe { header(ac).length }, 0);
        assert_eq!(unsafe { header(ar).length }, 0);

        let hc = unsafe { (c.hmput_default)((ac as *mut u8).add(elemsize) as *mut c_void, elemsize) };
        let hr = unsafe { (r.hmput_default)((ar as *mut u8).add(elemsize) as *mut c_void, elemsize) };

        let bc = unsafe { header((hc as *mut u8).sub(elemsize) as *mut c_void) };
        let br = unsafe { header((hr as *mut u8).sub(elemsize) as *mut c_void) };
        assert_eq!(
            (bc.length, bc.capacity, bc.temp, bc.hash_table.is_null()),
            (br.length, br.capacity, br.temp, br.hash_table.is_null()),
            "hmput_default on a zero-length array (elemsize {})",
            elemsize
        );
        // the sentinel element must have been zeroed by both
        let ec = unsafe {
            std::slice::from_raw_parts((hc as *const u8).sub(elemsize), elemsize).to_vec()
        };
        let er = unsafe {
            std::slice::from_raw_parts((hr as *const u8).sub(elemsize), elemsize).to_vec()
        };
        assert_eq!(ec, er, "sentinel element bytes (elemsize {})", elemsize);
        assert!(ec.iter().all(|b| *b == 0), "sentinel must be zeroed");

        unsafe {
            (c.arrfreef)((hc as *mut u8).sub(elemsize) as *mut c_void);
            (r.arrfreef)((hr as *mut u8).sub(elemsize) as *mut c_void);
        }
    }
}

// ---------------------------------------------------------------------------
// Allocator accounting: stbds_hmfree_func must release the strdup'ed keys
// ---------------------------------------------------------------------------

unsafe extern "C" {
    /// glibc's `struct mallinfo2`; only `uordblks` (field 7) is used.
    fn mallinfo2() -> MallInfo2;
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MallInfo2 {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}

fn heap_in_use() -> usize {
    unsafe { mallinfo2().uordblks }
}

/// Builds a STRDUP-mode map with `n` keys, tears it down again and returns the
/// net heap growth across the whole cycle. If `stbds_hmfree_func` skips the
/// `free` of the strdup'ed keys this grows by roughly one allocator chunk per
/// key, which dwarfs ordinary allocator noise.
fn strdup_map_leak(api: &Api, n: usize) -> isize {
    let mut keys: Vec<CStr8> = (0..n).map(|i| CStr8::new(&format!("key_{}", i))).collect();
    let key_ptrs: Vec<*mut c_char> = keys.iter_mut().map(|k| k.as_ptr()).collect();
    let before = heap_in_use();
    let mut m = SMap::new(api);
    m.sh_new(STBDS_SH_STRDUP);
    m.shdefault(-2);
    for (i, p) in key_ptrs.iter().enumerate() {
        m.shput(*p, i as c_int);
    }
    m.free();
    let after = heap_in_use();
    keys.clear();
    after as isize - before as isize
}

#[test]
fn hmfree_func_releases_the_same_amount_of_memory() {
    let _g = serial();
    let (c, r) = apis();

    for &n in &[512usize, 2000] {
        // Warm the allocator up so the first-touch growth is not attributed to
        // either library.
        reset_seeds(&c, &r, DEFAULT_SEED);
        let _ = strdup_map_leak(&c, n);
        let _ = strdup_map_leak(&r, n);

        reset_seeds(&c, &r, DEFAULT_SEED);
        let lc = strdup_map_leak(&c, n);
        let lr = strdup_map_leak(&r, n);

        // Each strdup'ed key occupies at least 32 bytes of allocator chunk, so a
        // missing `free` would show up as >= 16 * n bytes retained.
        let budget = (n * 8) as isize;
        assert!(
            lc.abs() < budget,
            "C retained {} bytes after freeing a {}-key strdup map",
            lc,
            n
        );
        assert!(
            lr.abs() < budget,
            "Rust retained {} bytes after freeing a {}-key strdup map \
             (C retained {}) — stbds_hmfree_func is not releasing the keys",
            lr,
            n,
            lc
        );
        assert!(
            (lc - lr).abs() < budget,
            "heap retention differs: C {} bytes vs Rust {} bytes for {} keys",
            lc,
            lr,
            n
        );
    }
}
