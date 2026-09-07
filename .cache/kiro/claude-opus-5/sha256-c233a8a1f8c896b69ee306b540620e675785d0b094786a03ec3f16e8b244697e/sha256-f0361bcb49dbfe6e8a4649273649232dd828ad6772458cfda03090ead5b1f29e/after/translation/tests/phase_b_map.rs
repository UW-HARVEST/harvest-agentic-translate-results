//! Phase B — CONFIGS.md rows 14-50: the hash-map core driven exactly the way
//! the `stbds_hm*` / `stbds_sh*` macros drive it, but through the `.so`
//! exports of both libraries in lock-step.

mod common;
use common::*;
use std::ffi::{c_int, c_void};

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

/// Runs one operation sequence against both libraries simultaneously and
/// compares the complete observable state after every single step.
struct Pair<'a> {
    b: &'a Both,
    c: *mut c_void,
    r: *mut c_void,
    elemsize: usize,
    keysize: usize,
    kind: KeyKind,
    payload_off: usize,
    payload_len: usize,
    /// label used in assertion messages
    tag: String,
    step: usize,
}

impl<'a> Pair<'a> {
    fn new(
        b: &'a Both,
        elemsize: usize,
        keysize: usize,
        kind: KeyKind,
        payload_off: usize,
        payload_len: usize,
        tag: &str,
    ) -> Pair<'a> {
        Pair {
            b,
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            elemsize,
            keysize,
            kind,
            payload_off,
            payload_len,
            tag: tag.to_string(),
            step: 0,
        }
    }

    /// Start from a table created by `stbds_shmode_func` (rows 24-26, 28, 50).
    fn shmode(&mut self, mode: c_int) {
        unsafe {
            self.c = (self.b.c.shmode_func)(self.elemsize, mode);
            self.r = (self.b.r.shmode_func)(self.elemsize, mode);
        }
        self.check("shmode_func");
    }

    fn snapshots(&self) -> (Snapshot, Snapshot) {
        unsafe {
            (
                snap(
                    self.c,
                    self.elemsize,
                    self.keysize,
                    self.kind,
                    self.payload_off,
                    self.payload_len,
                ),
                snap(
                    self.r,
                    self.elemsize,
                    self.keysize,
                    self.kind,
                    self.payload_off,
                    self.payload_len,
                ),
            )
        }
    }

    fn check(&mut self, what: &str) {
        self.step += 1;
        let (cs, rs) = self.snapshots();
        assert_eq!(
            cs, rs,
            "[{}] step {} after {what}: state diverged",
            self.tag, self.step
        );
    }

    /// `stbds_hmput_key` + the macro's `t[temp].value = v`
    fn put(&mut self, key: *mut c_void, mode: c_int, payload: &[u8]) -> (isize, isize) {
        unsafe {
            // Poke a sentinel so that "did the library write temp_key?" is a
            // deterministic question (the field is otherwise uninitialised).
            poke_temp_key(self.c, self.elemsize);
            poke_temp_key(self.r, self.elemsize);
            let ctable_before = if self.c.is_null() {
                std::ptr::null_mut()
            } else {
                (*hdr(hash_to_arr(self.c, self.elemsize))).hash_table
            };
            let rtable_before = if self.r.is_null() {
                std::ptr::null_mut()
            } else {
                (*hdr(hash_to_arr(self.r, self.elemsize))).hash_table
            };

            self.c = (self.b.c.hmput_key)(self.c, self.elemsize, key, self.keysize, mode);
            self.r = (self.b.r.hmput_key)(self.r, self.elemsize, key, self.keysize, mode);
            let ct = (*hdr(hash_to_arr(self.c, self.elemsize))).temp;
            let rt = (*hdr(hash_to_arr(self.r, self.elemsize))).temp;
            assert_eq!(ct, rt, "[{}] step {} put temp", self.tag, self.step);
            if self.payload_len > 0 {
                assert_eq!(payload.len(), self.payload_len);
                for p in [self.c, self.r] {
                    let e = (p as *mut u8).add(self.elemsize * (ct as usize) + self.payload_off);
                    std::ptr::copy_nonoverlapping(payload.as_ptr(), e, self.payload_len);
                }
            }
            // `stbds_temp_key` — only comparable when the table was not
            // reallocated by this put (a rehash leaves the field uninitialised
            // in both libraries alike).
            let ctable_after = (*hdr(hash_to_arr(self.c, self.elemsize))).hash_table;
            let rtable_after = (*hdr(hash_to_arr(self.r, self.elemsize))).hash_table;
            let c_same = !ctable_before.is_null() && ctable_before == ctable_after;
            let r_same = !rtable_before.is_null() && rtable_before == rtable_after;
            assert_eq!(
                c_same, r_same,
                "[{}] step {} put table-realloc decision",
                self.tag, self.step
            );
            if c_same {
                let ck = classify_temp_key(self.c, self.elemsize);
                let rk = classify_temp_key(self.r, self.elemsize);
                assert_eq!(ck, rk, "[{}] step {} put temp_key", self.tag, self.step);
            }
            self.check("put");
            (ct, rt)
        }
    }

    /// `stbds_hmget_key` — returns the `header->temp` both sides computed
    fn get(&mut self, key: *mut c_void, mode: c_int) -> isize {
        unsafe {
            self.c = (self.b.c.hmget_key)(self.c, self.elemsize, key, self.keysize, mode);
            self.r = (self.b.r.hmget_key)(self.r, self.elemsize, key, self.keysize, mode);
            let ct = (*hdr(hash_to_arr(self.c, self.elemsize))).temp;
            let rt = (*hdr(hash_to_arr(self.r, self.elemsize))).temp;
            assert_eq!(ct, rt, "[{}] step {} get temp", self.tag, self.step);
            self.check("get");
            ct
        }
    }

    /// `stbds_hmget_key_ts`
    fn get_ts(&mut self, key: *mut c_void, mode: c_int) -> isize {
        unsafe {
            let mut ctemp: isize = 0x5A5A;
            let mut rtemp: isize = 0x5A5A;
            self.c = (self.b.c.hmget_key_ts)(
                self.c,
                self.elemsize,
                key,
                self.keysize,
                &mut ctemp,
                mode,
            );
            self.r = (self.b.r.hmget_key_ts)(
                self.r,
                self.elemsize,
                key,
                self.keysize,
                &mut rtemp,
                mode,
            );
            assert_eq!(ctemp, rtemp, "[{}] step {} get_ts temp", self.tag, self.step);
            self.check("get_ts");
            ctemp
        }
    }

    /// `stbds_hmdel_key` + the macro's `(t) ? stbds_temp((t)-1) : 0`
    fn del(&mut self, key: *mut c_void, keyoffset: usize, mode: c_int) -> isize {
        unsafe {
            self.c =
                (self.b.c.hmdel_key)(self.c, self.elemsize, key, self.keysize, keyoffset, mode);
            self.r =
                (self.b.r.hmdel_key)(self.r, self.elemsize, key, self.keysize, keyoffset, mode);
            assert_eq!(
                self.c.is_null(),
                self.r.is_null(),
                "[{}] step {} del nullness",
                self.tag,
                self.step
            );
            let ct = if self.c.is_null() {
                0
            } else {
                (*hdr(hash_to_arr(self.c, self.elemsize))).temp
            };
            let rt = if self.r.is_null() {
                0
            } else {
                (*hdr(hash_to_arr(self.r, self.elemsize))).temp
            };
            assert_eq!(ct, rt, "[{}] step {} del temp", self.tag, self.step);
            self.check("del");
            ct
        }
    }

    /// `stbds_hmput_default` + `t[-1].value = v`
    fn put_default(&mut self, payload: &[u8]) {
        unsafe {
            self.c = (self.b.c.hmput_default)(self.c, self.elemsize);
            self.r = (self.b.r.hmput_default)(self.r, self.elemsize);
            if self.payload_len > 0 {
                for p in [self.c, self.r] {
                    let e = (p as *mut u8)
                        .sub(self.elemsize)
                        .add(self.payload_off);
                    std::ptr::copy_nonoverlapping(payload.as_ptr(), e, self.payload_len);
                }
            }
            self.check("put_default");
        }
    }

    fn free(&mut self) {
        unsafe {
            if !self.c.is_null() {
                (self.b.c.hmfree_func)(hash_to_arr(self.c, self.elemsize), self.elemsize);
            }
            if !self.r.is_null() {
                (self.b.r.hmfree_func)(hash_to_arr(self.r, self.elemsize), self.elemsize);
            }
        }
        self.c = std::ptr::null_mut();
        self.r = std::ptr::null_mut();
    }
}

fn u64key(v: u64) -> CBuf {
    CBuf::new(&v.to_le_bytes())
}

// ---------------------------------------------------------------------------
// rows 14-16 — stbds_hmput_default
// ---------------------------------------------------------------------------

#[test]
fn row_14_to_16_hmput_default() {
    let (_g, b) = both();
    for &es in &[8usize, 16, 24, 32] {
        seed_both(b, 0x3141_5926);
        // row 14: a == NULL
        let mut p = Pair::new(b, es, 8, KeyKind::Raw, es - 8, 8, &format!("default es={es}"));
        p.put_default(&(-2i64).to_le_bytes());
        // row 15: second call is a no-op
        p.put_default(&(-3i64).to_le_bytes());
        p.put_default(&(-4i64).to_le_bytes());
        // row 16: hash_table is still NULL -> temp == -1 (ERRORS.md #10)
        let k = u64key(0xdead_beef);
        assert_eq!(p.get(k.ptr(), HM_BINARY), -1);
        assert_eq!(p.get_ts(k.ptr(), HM_BINARY), -1);
        assert_eq!(p.get(k.ptr(), HM_STRING), -1);
        // ERRORS.md #25: delete with no table
        assert_eq!(p.del(k.ptr(), 0, HM_BINARY), 0);
        p.free();
    }
}

// ---------------------------------------------------------------------------
// rows 17-23, 31-32, 35-36 — binary maps
// ---------------------------------------------------------------------------

fn binary_workout(b: &Both, keysize: usize, payload_len: usize, n: usize, rngseed: u64, mode: c_int) {
    let elemsize = keysize + payload_len;
    let tag = format!("bin ks={keysize} pl={payload_len} n={n} mode={mode}");
    seed_both(b, 0x3141_5926 ^ (n as usize));
    let mut p = Pair::new(b, elemsize, keysize, KeyKind::Raw, keysize, payload_len, &tag);
    let mut rng = Rng::new(rngseed);

    // keep every key buffer alive; both libraries get the *same* pointer
    let keys: Vec<CBuf> = (0..n).map(|_| CBuf::new(&rng.bytes(keysize))).collect();

    p.put_default(&vec![0xEEu8; payload_len]);
    for (i, k) in keys.iter().enumerate() {
        let mut pl = vec![0u8; payload_len];
        for (j, x) in pl.iter_mut().enumerate() {
            *x = (i as u8).wrapping_mul(7).wrapping_add(j as u8);
        }
        p.put(k.ptr(), mode, &pl);
    }
    // row 31/32: lookups, present and absent
    for k in keys.iter() {
        p.get(k.ptr(), mode);
        p.get_ts(k.ptr(), mode);
    }
    for _ in 0..n.max(4) {
        let miss = CBuf::new(&rng.bytes(keysize));
        p.get(miss.ptr(), mode);
        p.get_ts(miss.ptr(), mode);
    }
    // row 23: duplicate puts
    for k in keys.iter() {
        p.put(k.ptr(), mode, &vec![0x99u8; payload_len]);
    }
    p.free();
}

#[test]
fn row_17_to_23_binary_maps() {
    let (_g, b) = both();
    // row 17/18: keysize 8, crossing every growth threshold
    for &n in &[0usize, 1, 2, 5, 6, 7, 8, 12, 13, 23, 24, 25, 48, 100, 400, 1000] {
        binary_workout(b, 8, 8, n, 0xA1 ^ n as u64, HM_BINARY);
    }
    // row 19: 4-byte keys
    for &n in &[1usize, 6, 12, 48, 200] {
        binary_workout(b, 4, 8, n, 0xB2 ^ n as u64, HM_BINARY);
    }
    // row 20: 16-byte keys with a 16-byte payload
    for &n in &[1usize, 6, 12, 48, 200] {
        binary_workout(b, 16, 16, n, 0xC3 ^ n as u64, HM_BINARY);
    }
    // row 21: keysize == elemsize (no payload)
    for &n in &[1usize, 6, 12, 48, 200] {
        binary_workout(b, 8, 0, n, 0xD4 ^ n as u64, HM_BINARY);
    }
    // odd key sizes exercising the hash tail switch
    for ks in [1usize, 2, 3, 5, 6, 7, 9, 15, 17, 31] {
        binary_workout(b, ks, 8, 40, 0xE5 ^ ks as u64, HM_BINARY);
    }
}

/// row 22 — `keysize == 0`.  Every key hashes identically *and* `memcmp(_,_,0)`
/// always reports "equal", so the map collapses to a single entry.
#[test]
fn row_22_keysize_zero() {
    let (_g, b) = both();
    seed_both(b, 0x3141_5926);
    let mut p = Pair::new(b, 8, 0, KeyKind::Raw, 0, 8, "keysize=0");
    let mut rng = Rng::new(0x220);
    p.put_default(&(-2i64).to_le_bytes());
    let keys: Vec<CBuf> = (0..20).map(|_| CBuf::new(&rng.bytes(8))).collect();
    for (i, k) in keys.iter().enumerate() {
        p.put(k.ptr(), HM_BINARY, &(i as u64).to_le_bytes());
    }
    for k in keys.iter() {
        p.get(k.ptr(), HM_BINARY);
        p.get_ts(k.ptr(), HM_BINARY);
    }
    p.del(keys[0].ptr(), 0, HM_BINARY);
    p.del(keys[1].ptr(), 0, HM_BINARY);
    p.free();
}

// ---------------------------------------------------------------------------
// rows 24-30, 33-34 — string maps in all four `string.mode`s
// ---------------------------------------------------------------------------

fn string_workout(
    b: &Both,
    shmode: Option<c_int>,
    mode: c_int,
    n: usize,
    maxlen: usize,
    rngseed: u64,
) {
    let elemsize = 16usize;
    let keysize = 8usize;
    let tag = format!("str shmode={shmode:?} mode={mode} n={n} maxlen={maxlen}");
    seed_both(b, 0x3141_5926 ^ n);
    let mut p = Pair::new(b, elemsize, keysize, KeyKind::Str, 8, 8, &tag);
    if let Some(m) = shmode {
        p.shmode(m);
    }
    let mut rng = Rng::new(rngseed);

    // distinct keys (a length-prefixed unique tail guarantees distinctness)
    let mut keys: Vec<CBuf> = Vec::new();
    for i in 0..n {
        let l = 1 + rng.below(maxlen);
        let mut s = rng.cstring(l);
        s.pop(); // drop NUL
        s.extend_from_slice(format!("#{i}").as_bytes());
        s.push(0);
        keys.push(CBuf::new(&s));
    }

    p.put_default(&(-2i64).to_le_bytes());
    for (i, k) in keys.iter().enumerate() {
        p.put(k.cptr() as *mut c_void, mode, &(i as u64 * 3).to_le_bytes());
    }
    for k in keys.iter() {
        p.get(k.cptr() as *mut c_void, mode);
        p.get_ts(k.cptr() as *mut c_void, mode);
    }
    // absent keys, including the empty string and prefixes of present keys
    let empty = CBuf::new(b"\0");
    p.get(empty.cptr() as *mut c_void, mode);
    p.get_ts(empty.cptr() as *mut c_void, mode);
    for _ in 0..n.max(4) {
        let l = 1 + rng.below(maxlen);
        let miss = CBuf::new(&rng.cstring(l));
        p.get(miss.cptr() as *mut c_void, mode);
    }
    // duplicate puts (hits the "key already present" early return, which is
    // where the C updates `stbds_temp_key`)
    for (i, k) in keys.iter().enumerate() {
        p.put(k.cptr() as *mut c_void, mode, &(i as u64 * 5).to_le_bytes());
    }
    p.free();
    drop(keys);
}

#[test]
fn row_24_to_27_string_maps() {
    let (_g, b) = both();
    for &n in &[0usize, 1, 2, 6, 7, 12, 48, 200] {
        // row 24: SH_DEFAULT (caller-owned keys)
        string_workout(b, Some(SH_DEFAULT), HM_STRING, n, 12, 0x24 ^ n as u64);
        // row 25: SH_STRDUP
        string_workout(b, Some(SH_STRDUP), HM_STRING, n, 12, 0x25 ^ n as u64);
        // row 26: SH_ARENA
        string_workout(b, Some(SH_ARENA), HM_STRING, n, 12, 0x26 ^ n as u64);
        // row 27: no shmode_func at all -> table auto-created as SH_DEFAULT
        string_workout(b, None, HM_STRING, n, 12, 0x27 ^ n as u64);
    }
    // row 26 variant: keys longer than the 512-byte arena block
    string_workout(b, Some(SH_ARENA), HM_STRING, 20, 900, 0x261);
    string_workout(b, Some(SH_STRDUP), HM_STRING, 20, 900, 0x251);
    string_workout(b, Some(SH_DEFAULT), HM_STRING, 20, 2000, 0x241);
    // SH_NONE explicitly (memcpy branch, binary mode arg)
    for &n in &[1usize, 6, 30] {
        seed_both(b, 7);
        let mut p = Pair::new(b, 16, 8, KeyKind::Raw, 8, 8, "shmode SH_NONE binary");
        p.shmode(SH_NONE);
        let mut rng = Rng::new(0x2711 ^ n as u64);
        let keys: Vec<CBuf> = (0..n).map(|_| CBuf::new(&rng.bytes(8))).collect();
        p.put_default(&(-2i64).to_le_bytes());
        for (i, k) in keys.iter().enumerate() {
            p.put(k.ptr(), HM_BINARY, &(i as u64).to_le_bytes());
        }
        for k in keys.iter() {
            p.get(k.ptr(), HM_BINARY);
        }
        p.free();
    }
}

/// row 28 — `stbds_shmode_func` with an out-of-range `string.mode`.
/// The `switch` in `stbds_hmput_key` falls to `default:` and `memcpy`s the key
/// bytes, even though the map is being used in string mode.  Driven with
/// BINARY `mode` so the (garbage) stored bytes are never dereferenced.
#[test]
fn row_28_out_of_range_string_mode() {
    let (_g, b) = both();
    for shmode in [4i32, 5, 100, 255, -1, -2, 256, 257, 1000, i32::MAX, i32::MIN] {
        for &n in &[1usize, 6, 30] {
            seed_both(b, 0x3141_5926);
            let tag = format!("shmode={shmode} n={n}");
            let mut p = Pair::new(b, 16, 8, KeyKind::Raw, 8, 8, &tag);
            p.shmode(shmode);
            let mut rng = Rng::new(0x280 ^ shmode as u64 ^ n as u64);
            let keys: Vec<CBuf> = (0..n).map(|_| CBuf::new(&rng.bytes(8))).collect();
            p.put_default(&(-2i64).to_le_bytes());
            for (i, k) in keys.iter().enumerate() {
                p.put(k.ptr(), HM_BINARY, &(i as u64).to_le_bytes());
            }
            for k in keys.iter() {
                p.get(k.ptr(), HM_BINARY);
                p.get_ts(k.ptr(), HM_BINARY);
            }
            for k in keys.iter() {
                p.del(k.ptr(), 0, HM_BINARY);
            }
            p.free();
        }
    }
}

/// rows 29-30 — `mode` values with no valid enum variant.
#[test]
fn row_29_30_out_of_range_mode() {
    let (_g, b) = both();
    // row 30: mode < 1 behaves as BINARY
    for mode in [-1i32, -2, -1000, i32::MIN] {
        for &n in &[1usize, 6, 30, 100] {
            binary_workout(b, 8, 8, n, 0x300 ^ n as u64 ^ mode as u64, mode);
        }
    }
    // row 29: mode >= 2 behaves as STRING for hashing/compare
    for mode in [2i32, 3, 7, 1000, i32::MAX] {
        for &n in &[0usize, 1, 6, 30, 100] {
            string_workout(b, Some(SH_DEFAULT), mode, n, 12, 0x290 ^ n as u64);
            string_workout(b, Some(SH_STRDUP), mode, n, 12, 0x291 ^ n as u64);
            string_workout(b, Some(SH_ARENA), mode, n, 12, 0x292 ^ n as u64);
            string_workout(b, None, mode, n, 12, 0x293 ^ n as u64);
            let _ = mode;
        }
    }
}

// ---------------------------------------------------------------------------
// rows 35-44 — deletion
// ---------------------------------------------------------------------------

#[test]
fn row_35_binary_delete() {
    let (_g, b) = both();
    for &n in &[1usize, 2, 6, 7, 12, 24, 48, 200] {
        for order in 0..3 {
            let tag = format!("del bin n={n} order={order}");
            seed_both(b, 0x3141_5926 ^ n);
            let mut p = Pair::new(b, 16, 8, KeyKind::Raw, 8, 8, &tag);
            let mut rng = Rng::new(0x350 ^ n as u64 ^ order);
            let keys: Vec<CBuf> = (0..n).map(|_| CBuf::new(&rng.bytes(8))).collect();
            p.put_default(&(-2i64).to_le_bytes());
            for (i, k) in keys.iter().enumerate() {
                p.put(k.ptr(), HM_BINARY, &(i as u64).to_le_bytes());
            }
            let mut idx: Vec<usize> = (0..n).collect();
            match order {
                0 => {}                 // row 41: insertion order
                1 => idx.reverse(),      // row 42: reverse order
                _ => {
                    // pseudo-random order
                    for i in (1..n).rev() {
                        let j = rng.below(i + 1);
                        idx.swap(i, j);
                    }
                }
            }
            // absent key first (ERRORS.md #26)
            let miss = CBuf::new(&rng.bytes(8));
            assert_eq!(p.del(miss.ptr(), 0, HM_BINARY), 0);
            for &i in &idx {
                p.del(keys[i].ptr(), 0, HM_BINARY);
                // deleting twice reports "not found"
                assert_eq!(p.del(keys[i].ptr(), 0, HM_BINARY), 0);
                p.get(keys[i].ptr(), HM_BINARY);
            }
            p.free();
        }
    }
}

/// row 36 — non-zero `keyoffset` (the only parameter `hmdel_key` has that
/// `hmput_key` hardcodes to 0).
#[test]
fn row_36_delete_keyoffset() {
    let (_g, b) = both();
    for &n in &[1usize, 6, 30] {
        for keyoffset in [8usize, 16] {
            let tag = format!("del keyoffset={keyoffset} n={n}");
            seed_both(b, 0x3141_5926);
            let mut p = Pair::new(b, 24, 8, KeyKind::Raw, 8, 16, &tag);
            let mut rng = Rng::new(0x360 ^ n as u64 ^ keyoffset as u64);
            let keys: Vec<CBuf> = (0..n).map(|_| CBuf::new(&rng.bytes(8))).collect();
            p.put_default(&[0u8; 16]);
            for (i, k) in keys.iter().enumerate() {
                let mut pl = [0u8; 16];
                pl[..8].copy_from_slice(&(i as u64).to_le_bytes());
                pl[8..].copy_from_slice(&(i as u64 * 11).to_le_bytes());
                p.put(k.ptr(), HM_BINARY, &pl);
            }
            for k in keys.iter() {
                p.del(k.ptr(), keyoffset, HM_BINARY);
            }
            p.free();
        }
    }
}

/// rows 37-40 — string-mode deletion in every `string.mode`, plus the
/// out-of-range `mode == 2` variant restricted to last-element deletes (which
/// is the only shape that does not trip the relocation `assert`; see
/// ERRORS.md #29 and tests/phase_c_abort.rs).
#[test]
fn row_37_to_40_string_delete() {
    let (_g, b) = both();
    for (shmode, label) in [
        (Some(SH_DEFAULT), "SH_DEFAULT"),
        (Some(SH_STRDUP), "SH_STRDUP"),
        (Some(SH_ARENA), "SH_ARENA"),
        (None, "auto"),
    ] {
        for &n in &[1usize, 2, 6, 7, 12, 24, 48, 200] {
            for order in 0..3u64 {
                let tag = format!("strdel {label} n={n} order={order}");
                seed_both(b, 0x3141_5926 ^ n);
                let mut p = Pair::new(b, 16, 8, KeyKind::Str, 8, 8, &tag);
                if let Some(m) = shmode {
                    p.shmode(m);
                }
                let mut rng = Rng::new(0x370 ^ n as u64 ^ order);
                let keys: Vec<CBuf> = (0..n)
                    .map(|i| {
                        let l = 1 + rng.below(14);
                        let mut s = rng.cstring(l);
                        s.pop();
                        s.extend_from_slice(format!("#{i}").as_bytes());
                        s.push(0);
                        CBuf::new(&s)
                    })
                    .collect();
                p.put_default(&(-2i64).to_le_bytes());
                for (i, k) in keys.iter().enumerate() {
                    p.put(k.cptr() as *mut c_void, HM_STRING, &(i as u64 * 3).to_le_bytes());
                }
                let mut idx: Vec<usize> = (0..n).collect();
                match order {
                    0 => {}
                    1 => idx.reverse(),
                    _ => {
                        for i in (1..n).rev() {
                            let j = rng.below(i + 1);
                            idx.swap(i, j);
                        }
                    }
                }
                let miss = CBuf::new(b"definitely-absent\0");
                assert_eq!(p.del(miss.cptr() as *mut c_void, 0, HM_STRING), 0);
                for &i in &idx {
                    p.del(keys[i].cptr() as *mut c_void, 0, HM_STRING);
                    p.get(keys[i].cptr() as *mut c_void, HM_STRING);
                }
                p.free();
            }
        }
    }

    // row 40: mode == 2 (out-of-range enum).  `mode == STBDS_HM_STRING` is
    // false, so the strdup key is NOT freed and the relocation lookup uses the
    // raw element address.  Deleting only the last element avoids relocation.
    for (shmode, label) in [
        (Some(SH_DEFAULT), "SH_DEFAULT"),
        (Some(SH_STRDUP), "SH_STRDUP"),
        (Some(SH_ARENA), "SH_ARENA"),
    ] {
        for &n in &[1usize, 2, 6, 12, 48] {
            for mode in [2i32, 9, i32::MAX] {
                let tag = format!("strdel-mode{mode} {label} n={n}");
                seed_both(b, 0x3141_5926 ^ n);
                let mut p = Pair::new(b, 16, 8, KeyKind::Str, 8, 8, &tag);
                if let Some(m) = shmode {
                    p.shmode(m);
                }
                let mut rng = Rng::new(0x400 ^ n as u64 ^ mode as u64);
                let keys: Vec<CBuf> = (0..n)
                    .map(|i| {
                        let l = 1 + rng.below(10);
                        let mut s = rng.cstring(l);
                        s.pop();
                        s.extend_from_slice(format!("#{i}").as_bytes());
                        s.push(0);
                        CBuf::new(&s)
                    })
                    .collect();
                p.put_default(&(-2i64).to_le_bytes());
                for (i, k) in keys.iter().enumerate() {
                    p.put(k.cptr() as *mut c_void, mode, &(i as u64 * 3).to_le_bytes());
                }
                // last-inserted element is at array index `length-1`; delete in
                // reverse insertion order so old_index == final_index always.
                for i in (0..n).rev() {
                    p.del(keys[i].cptr() as *mut c_void, 0, mode);
                    p.get(keys[i].cptr() as *mut c_void, mode);
                }
                p.free();
            }
        }
    }
}

/// row 43 — long randomized interleaved put/get/get_ts/del sequences.
#[test]
fn row_43_random_interleaved() {
    let (_g, b) = both();
    for trial in 0..12u64 {
        let tag = format!("interleaved trial={trial}");
        seed_both(b, 0x3141_5926 ^ trial as usize);
        let mut p = Pair::new(b, 16, 8, KeyKind::Raw, 8, 8, &tag);
        let mut rng = Rng::new(0x430 ^ trial);
        let pool: Vec<CBuf> = (0..64).map(|_| CBuf::new(&rng.bytes(8))).collect();
        p.put_default(&(-2i64).to_le_bytes());
        for step in 0..600u64 {
            let k = &pool[rng.below(pool.len())];
            match rng.below(5) {
                0 | 1 => {
                    p.put(k.ptr(), HM_BINARY, &step.to_le_bytes());
                }
                2 => {
                    p.get(k.ptr(), HM_BINARY);
                }
                3 => {
                    p.get_ts(k.ptr(), HM_BINARY);
                }
                _ => {
                    p.del(k.ptr(), 0, HM_BINARY);
                }
            }
        }
        p.free();
    }

    // same but string mode, all three arena modes
    for shmode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for trial in 0..6u64 {
            let tag = format!("interleaved str shmode={shmode} trial={trial}");
            seed_both(b, 0x3141_5926 ^ trial as usize);
            let mut p = Pair::new(b, 16, 8, KeyKind::Str, 8, 8, &tag);
            p.shmode(shmode);
            let mut rng = Rng::new(0x440 ^ trial ^ shmode as u64);
            let pool: Vec<CBuf> = (0..64)
                .map(|i| {
                    let l = 1 + rng.below(20);
                    let mut s = rng.cstring(l);
                    s.pop();
                    s.extend_from_slice(format!("#{i}").as_bytes());
                    s.push(0);
                    CBuf::new(&s)
                })
                .collect();
            p.put_default(&(-2i64).to_le_bytes());
            for step in 0..600u64 {
                let k = &pool[rng.below(pool.len())];
                match rng.below(5) {
                    0 | 1 => {
                        p.put(k.cptr() as *mut c_void, HM_STRING, &step.to_le_bytes());
                    }
                    2 => {
                        p.get(k.cptr() as *mut c_void, HM_STRING);
                    }
                    3 => {
                        p.get_ts(k.cptr() as *mut c_void, HM_STRING);
                    }
                    _ => {
                        p.del(k.cptr() as *mut c_void, 0, HM_STRING);
                    }
                }
            }
            p.free();
        }
    }
}

/// rows 44-49 — `stbds_hmfree_func` in every mode, incl. the NULL and
/// no-table shapes (ERRORS.md #7, #24, #25).
#[test]
fn row_44_to_49_free_and_null() {
    let (_g, b) = both();
    unsafe {
        // ERRORS.md #7: hmfree_func(NULL, _) is a no-op
        (b.c.hmfree_func)(std::ptr::null_mut(), 16);
        (b.r.hmfree_func)(std::ptr::null_mut(), 16);

        // ERRORS.md #24: hmdel_key(NULL, ...) returns NULL
        let k = u64key(1);
        for mode in [HM_BINARY, HM_STRING, -1, 2] {
            let c = (b.c.hmdel_key)(std::ptr::null_mut(), 16, k.ptr(), 8, 0, mode);
            let r = (b.r.hmdel_key)(std::ptr::null_mut(), 16, k.ptr(), 8, 0, mode);
            assert!(c.is_null() && r.is_null(), "hmdel_key(NULL) mode={mode}");
        }
    }

    // row 49: array whose hash_table is NULL (only hmput_default ran)
    for &es in &[8usize, 16, 32] {
        seed_both(b, 3);
        let mut p = Pair::new(b, es, 8, KeyKind::Raw, es - 8, 8, "free no-table");
        p.put_default(&[1u8; 8]);
        p.free();
    }

    // rows 45-48: free with each string.mode, with live entries
    for shmode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &n in &[0usize, 1, 6, 40] {
            seed_both(b, 0x3141_5926);
            let kind = if shmode == SH_NONE {
                KeyKind::Raw
            } else {
                KeyKind::Str
            };
            let mode = if shmode == SH_NONE { HM_BINARY } else { HM_STRING };
            let tag = format!("free shmode={shmode} n={n}");
            let mut p = Pair::new(b, 16, 8, kind, 8, 8, &tag);
            p.shmode(shmode);
            let mut rng = Rng::new(0x450 ^ n as u64 ^ shmode as u64);
            let keys: Vec<CBuf> = (0..n)
                .map(|i| {
                    if shmode == SH_NONE {
                        CBuf::new(&rng.bytes(8))
                    } else {
                        let l = 1 + rng.below(600);
                        let mut s = rng.cstring(l);
                        s.pop();
                        s.extend_from_slice(format!("#{i}").as_bytes());
                        s.push(0);
                        CBuf::new(&s)
                    }
                })
                .collect();
            p.put_default(&(-2i64).to_le_bytes());
            for (i, k) in keys.iter().enumerate() {
                p.put(k.ptr(), mode, &(i as u64).to_le_bytes());
            }
            p.free();
        }
    }
}

/// row 50 — `stbds_shmode_func` across element sizes and modes.
#[test]
fn row_50_shmode_func() {
    let (_g, b) = both();
    for &es in &[8usize, 16, 24, 32, 1, 7, 64] {
        for mode in [0i32, 1, 2, 3, 4, 255, 256, -1, i32::MAX, i32::MIN] {
            seed_both(b, 0x3141_5926);
            unsafe {
                let c = (b.c.shmode_func)(es, mode);
                let r = (b.r.shmode_func)(es, mode);
                let cs = snap(c, es, 0, KeyKind::Raw, 0, 0);
                let rs = snap(r, es, 0, KeyKind::Raw, 0, 0);
                assert_eq!(cs, rs, "shmode_func es={es} mode={mode}");
                // `(unsigned char) mode` truncation must match
                assert_eq!(cs.arena_mode, (mode as u32 & 0xff) as u8);
                (b.c.hmfree_func)(hash_to_arr(c, es), es);
                (b.r.hmfree_func)(hash_to_arr(r, es), es);
            }
        }
    }
}
