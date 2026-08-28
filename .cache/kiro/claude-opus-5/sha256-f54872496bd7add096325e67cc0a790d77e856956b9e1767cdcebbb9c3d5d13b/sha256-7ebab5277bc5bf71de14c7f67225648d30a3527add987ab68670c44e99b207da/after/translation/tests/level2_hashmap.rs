//! Level 2: the hash map core - `stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_hmdel_key`,
//! `stbds_shmode_func` and `stbds_hmfree_func`.
//!
//! Every operation replays exactly what the corresponding `hm*` / `sh*` macro
//! from `lib.c` expands to, on both implementations, and the complete map state
//! (array header, hash-index bookkeeping, every bucket slot, and all live
//! element bytes) is compared after each step.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

/// One map under test, driven through the loaded `.so` exports.
struct Map<'a> {
    im: &'a Impl,
    t: *mut c_void,
    elemsize: usize,
    keysize: usize,
    kind: KeyKind,
}

impl<'a> Map<'a> {
    fn new(im: &'a Impl, elemsize: usize, keysize: usize, kind: KeyKind) -> Map<'a> {
        Map {
            im,
            t: std::ptr::null_mut(),
            elemsize,
            keysize,
            kind,
        }
    }

    fn from_shmode(im: &'a Impl, elemsize: usize, keysize: usize, mode: c_int) -> Map<'a> {
        let t = unsafe { (im.shmode_func)(elemsize, mode) };
        Map {
            im,
            t,
            elemsize,
            keysize,
            kind: KeyKind::StringPtr,
        }
    }

    fn temp(&self) -> isize {
        unsafe { map_temp(self.t, self.elemsize) }
    }

    /// Raw (array side) pointer to element `i` of the *hash side* view, i.e.
    /// `&t[i]`.
    fn elem(&self, i: isize) -> *mut u8 {
        unsafe { (self.t as *mut u8).offset(i * self.elemsize as isize) }
    }

    /// Fills the bytes after the key with a deterministic pattern, standing in
    /// for the `(t)[temp].value = v` part of the `hmput`/`shput` macros. The
    /// key bytes themselves are left exactly as the library wrote them.
    fn write_tail(&self, i: isize, seed: u8) {
        unsafe {
            let e = self.elem(i);
            for b in self.keysize..self.elemsize {
                *e.add(b) = seed.wrapping_mul(31).wrapping_add(b as u8);
            }
        }
    }

    /// `stbds_hmput(t, k, v)` / `stbds_shput(t, k, v)`
    fn put(&mut self, key: *mut c_void, mode: c_int, tail_seed: u8) -> isize {
        unsafe {
            self.t = (self.im.hmput_key)(self.t, self.elemsize, key, self.keysize, mode);
            let temp = self.temp();
            self.write_tail(temp, tail_seed);
            temp
        }
    }

    /// `stbds_hmgeti(t, k)` / `stbds_shgeti(t, k)`
    fn geti(&mut self, key: *mut c_void, mode: c_int) -> isize {
        unsafe {
            self.t = (self.im.hmget_key)(self.t, self.elemsize, key, self.keysize, mode);
            self.temp()
        }
    }

    /// `stbds_hmgeti_ts(t, k, temp)`
    fn geti_ts(&mut self, key: *mut c_void, mode: c_int) -> isize {
        unsafe {
            let mut temp: isize = 0x5555_5555;
            self.t =
                (self.im.hmget_key_ts)(self.t, self.elemsize, key, self.keysize, &mut temp, mode);
            temp
        }
    }

    /// `stbds_hmdel(t, k)` / `stbds_shdel(t, k)` (keyoffset is 0 for every
    /// layout used here, matching `STBDS_OFFSETOF(t, key)`).
    fn del(&mut self, key: *mut c_void, mode: c_int) -> isize {
        unsafe {
            self.t = (self.im.hmdel_key)(self.t, self.elemsize, key, self.keysize, 0, mode);
            if self.t.is_null() {
                0
            } else {
                self.temp()
            }
        }
    }

    /// `stbds_hmdefault(t, v)`
    fn put_default(&mut self, tail_seed: u8) {
        unsafe {
            self.t = (self.im.hmput_default)(self.t, self.elemsize);
            self.write_tail(-1, tail_seed);
        }
    }

    fn snapshot(&self) -> Vec<u8> {
        unsafe { snapshot_map(self.t, self.elemsize, self.keysize, self.kind) }
    }

    fn len(&self) -> usize {
        if self.t.is_null() {
            0
        } else {
            unsafe { map_header(self.t, self.elemsize).length }
        }
    }

    /// `table->temp_key`, as set by the string-mode paths of `hmput_key`.
    fn temp_key(&self) -> Vec<u8> {
        unsafe {
            let h = map_header(self.t, self.elemsize);
            if h.hash_table.is_null() {
                return b"<no table>".to_vec();
            }
            cstr_bytes((*(h.hash_table as *const HashIndex)).temp_key)
        }
    }

    fn free(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.im.hmfree_func)(
                    (self.t as *mut u8).sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
            }
            self.t = std::ptr::null_mut();
        }
    }
}

/// Runs one closure against both implementations, comparing state after the
/// step described by `label`.
struct Pair<'a> {
    c: Map<'a>,
    r: Map<'a>,
    step: usize,
}

impl<'a> Pair<'a> {
    fn check(&mut self, label: &str) {
        let cs = self.c.snapshot();
        let rs = self.r.snapshot();
        assert_same(&format!("step {} ({label})", self.step), &cs, &rs);
        self.step += 1;
    }

    fn both<F: Fn(&mut Map) -> isize>(&mut self, label: &str, f: F) {
        let cv = f(&mut self.c);
        let rv = f(&mut self.r);
        assert_eq!(cv, rv, "step {} ({label}): temp/index C={cv} Rust={rv}", self.step);
        self.check(label);
    }

    /// A string-mode `shput`. When the insert is new, `hmput_key` also assigns
    /// `table->temp_key` (that is what `stbds_shputs` reads back), so compare it.
    fn shput(&mut self, label: &str, key: *mut c_void, tail_seed: u8) {
        let clen = self.c.len();
        let rlen = self.r.len();
        let cv = self.c.put(key, HM_STRING, tail_seed);
        let rv = self.r.put(key, HM_STRING, tail_seed);
        assert_eq!(cv, rv, "step {} ({label}): temp C={cv} Rust={rv}", self.step);
        if self.c.len() > clen || self.r.len() > rlen {
            assert_eq!(self.c.len() > clen, self.r.len() > rlen, "step {} ({label}): insert/update disagree", self.step);
            assert_same(
                &format!("step {} ({label}) temp_key", self.step),
                &self.c.temp_key(),
                &self.r.temp_key(),
            );
        }
        self.check(label);
    }

    fn free(&mut self) {
        self.c.free();
        self.r.free();
    }
}

// ---------------------------------------------------------------------------
// Binary hash maps
// ---------------------------------------------------------------------------

fn binary_map_case(c: &Impl, r: &Impl, elemsize: usize, keysize: usize, n: usize) {
    let _g = seeded(c, r, 0x3141_5926);
    let mut p = Pair {
        c: Map::new(c, elemsize, keysize, KeyKind::Inline),
        r: Map::new(r, elemsize, keysize, KeyKind::Inline),
        step: 0,
    };

    // Keys are `keysize`-byte little-endian counters.
    let key_of = |i: usize| -> Vec<u8> {
        let mut k = vec![0u8; keysize.max(8)];
        k[..8].copy_from_slice(&(i as u64).to_le_bytes());
        k
    };

    // Insert - crosses every table growth boundary (8, 16, 32, ... slots).
    for i in 0..n {
        let mut k = key_of(i);
        let kp = k.as_mut_ptr() as *mut c_void;
        p.both(&format!("put {i}"), |m| m.put(kp, HM_BINARY, i as u8));
    }

    // Re-insert existing keys: must hit the "key already present" path.
    for i in (0..n).step_by(3) {
        let mut k = key_of(i);
        let kp = k.as_mut_ptr() as *mut c_void;
        p.both(&format!("reput {i}"), |m| m.put(kp, HM_BINARY, i as u8));
    }

    // Lookups, present and absent.
    for i in 0..n + 20 {
        let mut k = key_of(i);
        let kp = k.as_mut_ptr() as *mut c_void;
        p.both(&format!("geti {i}"), |m| m.geti(kp, HM_BINARY));
        p.both(&format!("geti_ts {i}"), |m| m.geti_ts(kp, HM_BINARY));
    }

    // Delete every 2nd key (tombstones -> rebuild), then the rest (-> shrink).
    for i in (0..n).step_by(2) {
        let mut k = key_of(i);
        let kp = k.as_mut_ptr() as *mut c_void;
        p.both(&format!("del {i}"), |m| m.del(kp, HM_BINARY));
        // Deleting the same key twice: second call must find nothing.
        p.both(&format!("del-again {i}"), |m| m.del(kp, HM_BINARY));
    }
    // Re-insert into the tombstoned slots.
    for i in (0..n).step_by(4) {
        let mut k = key_of(i);
        let kp = k.as_mut_ptr() as *mut c_void;
        p.both(&format!("reinsert {i}"), |m| m.put(kp, HM_BINARY, (i + 1) as u8));
    }
    // Drain completely.
    for i in 0..n {
        let mut k = key_of(i);
        let kp = k.as_mut_ptr() as *mut c_void;
        p.both(&format!("drain {i}"), |m| m.del(kp, HM_BINARY));
    }
    p.free();
}

#[test]
fn binary_map_matches() {
    let (c, r) = load_pair();
    // (elemsize, keysize): mirrors `struct {int key; int b,c,d;}`,
    // `struct {int key[2]; ...}` and `struct {size_t key; size_t value;}`.
    for &(elemsize, keysize, n) in &[
        (16usize, 4usize, 120usize),
        (16, 8, 120),
        (8, 4, 200),
        (16, 16, 80),
        (32, 8, 120),
        (24, 8, 64),
    ] {
        binary_map_case(&c, &r, elemsize, keysize, n);
    }
}

// ---------------------------------------------------------------------------
// String hash maps
// ---------------------------------------------------------------------------

/// `mode` is the `STBDS_SH_*` storage mode: `None` means "start from a NULL
/// map" (which yields `STBDS_SH_DEFAULT`), otherwise the map is created with
/// `stbds_shmode_func` (`sh_new_strdup` / `sh_new_arena`).
fn string_map_case(c: &Impl, r: &Impl, elemsize: usize, sh_mode: Option<c_int>, n: usize) {
    let _g = seeded(c, r, 0x3141_5926);
    let keysize = std::mem::size_of::<*mut c_char>();

    // Keys must outlive the maps: STBDS_SH_DEFAULT stores the caller pointer.
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..n + 20 {
        let mut s = format!("test_{i}").into_bytes();
        if i % 7 == 0 {
            // Longer keys exercise the arena block logic.
            s.extend(std::iter::repeat(b'y').take(i % 600));
        }
        s.push(0);
        keys.push(s);
    }
    let kp = |keys: &mut Vec<Vec<u8>>, i: usize| keys[i].as_mut_ptr() as *mut c_void;

    let (cm, rm) = match sh_mode {
        None => (
            Map::new(c, elemsize, keysize, KeyKind::StringPtr),
            Map::new(r, elemsize, keysize, KeyKind::StringPtr),
        ),
        Some(m) => (
            Map::from_shmode(c, elemsize, keysize, m),
            Map::from_shmode(r, elemsize, keysize, m),
        ),
    };
    let mut p = Pair { c: cm, r: rm, step: 0 };
    p.check("created");

    for i in 0..n {
        let k = kp(&mut keys, i);
        p.shput(&format!("shput {i}"), k, i as u8);
    }
    for i in (0..n).step_by(3) {
        let k = kp(&mut keys, i);
        p.shput(&format!("shput-again {i}"), k, i as u8);
    }
    for i in 0..n + 20 {
        let k = kp(&mut keys, i);
        p.both(&format!("shgeti {i}"), |m| m.geti(k, HM_STRING));
        p.both(&format!("shgeti_ts {i}"), |m| m.geti_ts(k, HM_STRING));
    }
    for i in (0..n).step_by(2) {
        let k = kp(&mut keys, i);
        p.both(&format!("shdel {i}"), |m| m.del(k, HM_STRING));
        p.both(&format!("shdel-again {i}"), |m| m.del(k, HM_STRING));
    }
    for i in (0..n).step_by(4) {
        let k = kp(&mut keys, i);
        p.shput(&format!("shre {i}"), k, (i + 3) as u8);
    }
    for i in 0..n {
        let k = kp(&mut keys, i);
        p.both(&format!("shdrain {i}"), |m| m.del(k, HM_STRING));
    }
    p.free();
}

#[test]
fn string_map_default_mode_matches() {
    let (c, r) = load_pair();
    for &(elemsize, n) in &[(16usize, 120usize), (24, 80), (8, 100), (40, 60)] {
        string_map_case(&c, &r, elemsize, None, n);
    }
}

#[test]
fn string_map_strdup_mode_matches() {
    let (c, r) = load_pair();
    for &(elemsize, n) in &[(16usize, 120usize), (24, 80), (8, 100)] {
        string_map_case(&c, &r, elemsize, Some(SH_STRDUP), n);
    }
}

#[test]
fn string_map_arena_mode_matches() {
    let (c, r) = load_pair();
    for &(elemsize, n) in &[(16usize, 120usize), (24, 80), (8, 100)] {
        string_map_case(&c, &r, elemsize, Some(SH_ARENA), n);
    }
}

#[test]
fn string_map_other_shmodes_match() {
    let (c, r) = load_pair();
    // Only STBDS_SH_DEFAULT is meaningful here. `shmode_func` with
    // STBDS_SH_NONE would make `hmput_key` memcpy the key *bytes* inline while
    // `stbds_is_key_equal` still dereferences them as a `char *`, which faults
    // in the C reference as well - it is not a reachable configuration.
    string_map_case(&c, &r, 16, Some(SH_DEFAULT), 60);
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn hmput_default_matches() {
    let (c, r) = load_pair();
    let _g = seeded(&c, &r, 0x3141_5926);
    let elemsize = 16usize;
    let keysize = 8usize;

    // hmdefault on a fresh (NULL) map.
    let mut p = Pair {
        c: Map::new(&c, elemsize, keysize, KeyKind::Inline),
        r: Map::new(&r, elemsize, keysize, KeyKind::Inline),
        step: 0,
    };
    p.c.put_default(7);
    p.r.put_default(7);
    p.check("hmdefault on NULL");

    // hmdefault again: map already has length >= 1, must be a no-op.
    p.c.put_default(9);
    p.r.put_default(9);
    p.check("hmdefault again");

    // Then normal inserts, and hmdefault once more afterwards.
    for i in 0..20usize {
        let mut k = (i as u64).to_le_bytes();
        let kp = k.as_mut_ptr() as *mut c_void;
        p.both(&format!("put {i}"), |m| m.put(kp, HM_BINARY, i as u8));
    }
    p.c.put_default(11);
    p.r.put_default(11);
    p.check("hmdefault after inserts");
    p.free();
}

#[test]
fn get_before_put_matches() {
    let (c, r) = load_pair();
    let _g = seeded(&c, &r, 0x3141_5926);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut p = Pair {
        c: Map::new(&c, elemsize, keysize, KeyKind::Inline),
        r: Map::new(&r, elemsize, keysize, KeyKind::Inline),
        step: 0,
    };

    let mut k = 42u64.to_le_bytes();
    let kp = k.as_mut_ptr() as *mut c_void;

    // hmgeti on a NULL map allocates the default element but no hash table.
    p.both("geti on NULL", |m| m.geti(kp, HM_BINARY));
    // ... and again, now with a non-NULL map whose hash_table is still NULL.
    p.both("geti with NULL table", |m| m.geti(kp, HM_BINARY));
    p.both("geti_ts with NULL table", |m| m.geti_ts(kp, HM_BINARY));
    // hmdel on a map with no hash table returns early.
    p.both("del with NULL table", |m| m.del(kp, HM_BINARY));
    // Now insert into that map: hmput_key must build the table.
    p.both("put after geti", |m| m.put(kp, HM_BINARY, 1));
    p.both("geti after put", |m| m.geti(kp, HM_BINARY));
    p.free();

    // hmget_key_ts on a NULL map, straight through the exports.
    unsafe {
        let mut ct: isize = 0x1234;
        let mut rt: isize = 0x1234;
        let cp = (c.hmget_key_ts)(std::ptr::null_mut(), elemsize, kp, keysize, &mut ct, HM_BINARY);
        let rp = (r.hmget_key_ts)(std::ptr::null_mut(), elemsize, kp, keysize, &mut rt, HM_BINARY);
        assert_eq!(ct, rt, "hmget_key_ts(NULL) temp");
        assert_same(
            "hmget_key_ts(NULL) state",
            &snapshot_map(cp, elemsize, keysize, KeyKind::Inline),
            &snapshot_map(rp, elemsize, keysize, KeyKind::Inline),
        );
        (c.hmfree_func)((cp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((rp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

#[test]
fn null_map_operations_match() {
    let (c, r) = load_pair();
    let _g = seeded(&c, &r, 0x3141_5926);
    let mut k = 1u64.to_le_bytes();
    let kp = k.as_mut_ptr() as *mut c_void;
    unsafe {
        // hmdel_key(NULL) returns NULL.
        assert!((c.hmdel_key)(std::ptr::null_mut(), 16, kp, 8, 0, HM_BINARY).is_null());
        assert!((r.hmdel_key)(std::ptr::null_mut(), 16, kp, 8, 0, HM_BINARY).is_null());
        // hmfree_func(NULL) is a no-op.
        (c.hmfree_func)(std::ptr::null_mut(), 16);
        (r.hmfree_func)(std::ptr::null_mut(), 16);
    }
}

/// `shmode_func` state immediately after creation, for every mode.
#[test]
fn shmode_func_matches() {
    let (c, r) = load_pair();
    for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA, 7] {
        for &elemsize in &[8usize, 16, 24, 64] {
            let _g = seeded(&c, &r, 0xabcd_1234);
            unsafe {
                let ct = (c.shmode_func)(elemsize, mode);
                let rt = (r.shmode_func)(elemsize, mode);
                assert_same(
                    &format!("shmode_func(mode={mode}, elemsize={elemsize})"),
                    &snapshot_map(ct, elemsize, 8, KeyKind::Inline),
                    &snapshot_map(rt, elemsize, 8, KeyKind::Inline),
                );
                (c.hmfree_func)((ct as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)((rt as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

/// The seed evolution driven by repeated `stbds_make_hash_index` calls must
/// stay in lock-step between the two libraries.
#[test]
fn seed_evolution_matches() {
    let (c, r) = load_pair();
    for start in [0usize, 1, 0x3141_5926, usize::MAX, 0x8000_0000_0000_0000] {
        let _g = seeded(&c, &r, start);
        let mut cseeds = Vec::new();
        let mut rseeds = Vec::new();
        unsafe {
            for _ in 0..24 {
                let ct = (c.shmode_func)(16, SH_ARENA);
                let rt = (r.shmode_func)(16, SH_ARENA);
                cseeds.push((*(map_header(ct, 16).hash_table as *const HashIndex)).seed);
                rseeds.push((*(map_header(rt, 16).hash_table as *const HashIndex)).seed);
                (c.hmfree_func)((ct as *mut u8).sub(16) as *mut c_void, 16);
                (r.hmfree_func)((rt as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
        assert_eq!(cseeds, rseeds, "seed evolution from {start:#x}");
    }
}
