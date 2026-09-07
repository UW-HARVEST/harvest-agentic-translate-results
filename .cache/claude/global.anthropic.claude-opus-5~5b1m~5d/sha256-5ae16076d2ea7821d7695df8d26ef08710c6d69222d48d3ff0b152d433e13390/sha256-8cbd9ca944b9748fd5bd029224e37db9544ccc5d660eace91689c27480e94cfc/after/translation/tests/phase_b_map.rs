//! Phase B — valid-path differential tests for the HASH-MAP entry points,
//! driven exactly the way the `stbds_hm*` / `stbds_sh*` macros drive them.
//!
//! Covers CONFIGS.md rows C22–C56, C64, C65.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------------------
// A pair of hash maps (one per library) kept in lock-step.
// ---------------------------------------------------------------------------

/// Which `stbds_hash_index::string.mode` the map runs with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ShMode {
    /// no `stbds_shmode_func` call: `hmput_key` picks 0 or SH_DEFAULT itself
    Implicit,
    Explicit(c_int),
}

struct MapPair<'a> {
    c: &'a Lib,
    r: &'a Lib,
    hc: *mut c_void,
    hr: *mut c_void,
    elemsize: usize,
    keysize: usize,
    mode: c_int,
    shmode: ShMode,
    /// true once `temp_key` has definitely been written by the library
    tk_ready: bool,
    /// keeps `SH_DEFAULT` key buffers alive for the lifetime of the map
    #[allow(dead_code)]
    keep: Vec<Box<[u8]>>,
    label: String,
}

impl<'a> MapPair<'a> {
    fn new(
        c: &'a Lib,
        r: &'a Lib,
        elemsize: usize,
        keysize: usize,
        mode: c_int,
        shmode: ShMode,
        label: &str,
    ) -> MapPair<'a> {
        let (hc, hr) = match shmode {
            ShMode::Implicit => (std::ptr::null_mut(), std::ptr::null_mut()),
            ShMode::Explicit(m) => unsafe { ((c.shmode_func)(elemsize, m), (r.shmode_func)(elemsize, m)) },
        };
        MapPair {
            c,
            r,
            hc,
            hr,
            elemsize,
            keysize,
            mode,
            shmode,
            tk_ready: false,
            keep: Vec::new(),
            label: label.to_string(),
        }
    }

    /// Effective `string.mode` the library ends up using.
    fn eff_shmode(&self) -> u8 {
        match self.shmode {
            ShMode::Implicit => {
                if self.mode >= HM_STRING {
                    SH_DEFAULT as u8
                } else {
                    0
                }
            }
            ShMode::Explicit(m) => m as u8,
        }
    }

    /// Elements hold a `char *` key iff the effective string.mode selects one
    /// of the three pointer-storing switch arms.
    fn kind(&self) -> KeyKind {
        match self.eff_shmode() {
            1 | 2 | 3 => KeyKind::Ptr,
            _ => KeyKind::Raw,
        }
    }

    fn snap_both(&self) -> (MapSnap, MapSnap) {
        // `stbds_hash_index::temp_key` is written ONLY by `stbds_hmput_key`
        // and is never cleared, so after a `get`/`del` it is stale and (with
        // SH_STRDUP) dangling. It is therefore excluded from the generic
        // state check and verified separately right after a put by
        // `temp_key_strs()`.
        unsafe {
            (
                snap_map(self.hc, self.elemsize, self.kind(), false),
                snap_map(self.hr, self.elemsize, self.kind(), false),
            )
        }
    }

    /// `stbds_temp_key` from both libraries. Only call immediately after a
    /// successful string-mode put.
    fn temp_key_strs(&self) -> (Vec<u8>, Vec<u8>) {
        assert!(self.tk_ready, "temp_key is only defined right after a put");
        unsafe {
            let g = |h: *mut c_void| -> Vec<u8> {
                let hdr = Self::raw(h, self.elemsize).sub(HEADER_SIZE);
                let t = (hdr.add(16) as *const *mut c_void).read();
                let tk = (t as *const *const c_char).read();
                cstr_bytes(tk)
            };
            (g(self.hc), g(self.hr))
        }
    }

    fn check(&self, what: &str) {
        let (sc, sr) = self.snap_both();
        assert_eq!(sc, sr, "[{}] state diverged after {}", self.label, what);
    }

    unsafe fn raw(h: *mut c_void, elemsize: usize) -> *mut u8 {
        (h as *mut u8).sub(elemsize)
    }

    fn length(&self) -> usize {
        if self.hc.is_null() {
            return 0;
        }
        unsafe { (Self::raw(self.hc, self.elemsize).sub(HEADER_SIZE) as *const usize).read() }
    }

    fn temp(&self) -> isize {
        unsafe {
            (Self::raw(self.hc, self.elemsize)
                .sub(HEADER_SIZE)
                .add(24) as *const isize)
                .read()
        }
    }

    /// Define every byte of element `e` past the key, identically in both
    /// libraries. Necessary because the library only writes `keysize` bytes;
    /// the rest is caller-owned scratch whose contents legitimately differ
    /// (it is whatever `realloc` handed back).
    fn fill_value(&self, e: usize, tag: u64) {
        unsafe {
            for h in [self.hc, self.hr] {
                let p = Self::raw(h, self.elemsize).add(e * self.elemsize);
                for k in self.keysize..self.elemsize {
                    *p.add(k) = (e as u64)
                        .wrapping_mul(31)
                        .wrapping_add((k as u64).wrapping_mul(7))
                        .wrapping_add(tag) as u8;
                }
            }
        }
    }

    /// Additionally mirror the key bytes at `keyoffset` (only used by the
    /// `keyoffset != 0` configuration; `hmput_key` always stores at 0).
    fn mirror_key_at(&self, e: usize, keyoffset: usize, key: &[u8]) {
        unsafe {
            for h in [self.hc, self.hr] {
                let p = Self::raw(h, self.elemsize).add(e * self.elemsize).add(keyoffset);
                for (k, b) in key.iter().enumerate() {
                    *p.add(k) = *b;
                }
            }
        }
    }

    fn put(&mut self, key: &mut [u8], tag: u64, what: &str) -> isize {
        let kp = key.as_mut_ptr() as *mut c_void;
        self.hc = unsafe { (self.c.hmput_key)(self.hc, self.elemsize, kp, self.keysize, self.mode) };
        self.hr = unsafe { (self.r.hmput_key)(self.hr, self.elemsize, kp, self.keysize, self.mode) };
        self.tk_ready = self.mode >= HM_STRING && matches!(self.eff_shmode(), 1 | 2 | 3);
        let t = self.temp();
        let e = (t + 1) as usize;
        self.fill_value(e, tag);
        self.check(&format!("put {} (temp={})", what, t));
        t
    }

    fn put_default(&mut self, what: &str) {
        self.hc = unsafe { (self.c.hmput_default)(self.hc, self.elemsize) };
        self.hr = unsafe { (self.r.hmput_default)(self.hr, self.elemsize) };
        self.tk_ready = false;
        self.check(&format!("put_default {}", what));
    }

    fn get(&mut self, key: &mut [u8], what: &str) -> isize {
        let kp = key.as_mut_ptr() as *mut c_void;
        self.hc = unsafe { (self.c.hmget_key)(self.hc, self.elemsize, kp, self.keysize, self.mode) };
        self.hr = unsafe { (self.r.hmget_key)(self.hr, self.elemsize, kp, self.keysize, self.mode) };
        self.tk_ready = false;
        self.check(&format!("get {}", what));
        self.temp()
    }

    fn get_ts(&mut self, key: &mut [u8], what: &str) -> isize {
        let kp = key.as_mut_ptr() as *mut c_void;
        let mut tc: isize = 0x5555;
        let mut tr: isize = 0x5555;
        self.hc = unsafe {
            (self.c.hmget_key_ts)(self.hc, self.elemsize, kp, self.keysize, &mut tc, self.mode)
        };
        self.hr = unsafe {
            (self.r.hmget_key_ts)(self.hr, self.elemsize, kp, self.keysize, &mut tr, self.mode)
        };
        assert_eq!(
            tc, tr,
            "[{}] hmget_key_ts *temp diverged on {}",
            self.label, what
        );
        self.tk_ready = false;
        self.check(&format!("get_ts {}", what));
        tc
    }

    fn del(&mut self, key: &mut [u8], keyoffset: usize, what: &str) {
        let kp = key.as_mut_ptr() as *mut c_void;
        let nc = unsafe {
            (self.c.hmdel_key)(self.hc, self.elemsize, kp, self.keysize, keyoffset, self.mode)
        };
        let nr = unsafe {
            (self.r.hmdel_key)(self.hr, self.elemsize, kp, self.keysize, keyoffset, self.mode)
        };
        assert_eq!(
            nc.is_null(),
            nr.is_null(),
            "[{}] hmdel_key NULL-ness diverged on {}",
            self.label,
            what
        );
        self.hc = nc;
        self.hr = nr;
        self.tk_ready = false;
        self.check(&format!("del {}", what));
    }

    fn free(&mut self) {
        if self.hc.is_null() {
            return;
        }
        unsafe {
            (self.c.hmfree_func)(Self::raw(self.hc, self.elemsize) as *mut c_void, self.elemsize);
            (self.r.hmfree_func)(Self::raw(self.hr, self.elemsize) as *mut c_void, self.elemsize);
        }
        self.hc = std::ptr::null_mut();
        self.hr = std::ptr::null_mut();
    }
}

/// Reset the global hash seed in BOTH libraries so a test is reproducible
/// regardless of what ran before it in the same process.
fn sync_seed(c: &Lib, r: &Lib, seed: usize) {
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
}

fn key_u64(v: u64) -> [u8; 8] {
    v.to_le_bytes()
}

// ===========================================================================
// C22..C24  stbds_hmput_default
// ===========================================================================

#[test]
fn c22_hmput_default_from_null() {
    let (c, r) = both();
    for &elemsize in &[4usize, 8, 12, 16, 24] {
        sync_seed(&c, &r, 0x3141_5926);
        let hc = unsafe { (c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let hr = unsafe { (r.hmput_default)(std::ptr::null_mut(), elemsize) };
        let sc = unsafe { snap_map(hc, elemsize, KeyKind::Raw, false) };
        let sr = unsafe { snap_map(hr, elemsize, KeyKind::Raw, false) };
        assert_eq!(sc, sr, "hmput_default(NULL,{})", elemsize);
        assert_eq!(sc.length, 1);
        assert!(sc.table.is_none(), "no hash table is created");
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn c23_hmput_default_twice_is_noop() {
    let (c, r) = both();
    for &elemsize in &[8usize, 16] {
        sync_seed(&c, &r, 0x3141_5926);
        let mut hc = unsafe { (c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let mut hr = unsafe { (r.hmput_default)(std::ptr::null_mut(), elemsize) };
        for _ in 0..5 {
            hc = unsafe { (c.hmput_default)(hc, elemsize) };
            hr = unsafe { (r.hmput_default)(hr, elemsize) };
            let sc = unsafe { snap_map(hc, elemsize, KeyKind::Raw, false) };
            let sr = unsafe { snap_map(hr, elemsize, KeyKind::Raw, false) };
            assert_eq!(sc, sr);
            assert_eq!(sc.length, 1, "length must stay 1");
        }
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn c24_hmput_default_after_puts() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    let mut m = MapPair::new(&c, &r, 8, 8, HM_BINARY, ShMode::Implicit, "default-after-put");
    for i in 0..10u64 {
        let mut k = key_u64(i);
        m.put(&mut k, i, &format!("k{}", i));
    }
    m.put_default("after 10 puts");
    m.put_default("again");
    m.free();
}

// ===========================================================================
// C25..C29  hmget_key / hmget_key_ts
// ===========================================================================

#[test]
fn c25_hmget_key_ts_from_null() {
    let (c, r) = both();
    for &elemsize in &[4usize, 8, 16, 24] {
        let mut key = key_u64(0xDEAD_BEEF);
        let mut tc: isize = 7;
        let mut tr: isize = 7;
        let hc = unsafe {
            (c.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                8,
                &mut tc,
                HM_BINARY,
            )
        };
        let hr = unsafe {
            (r.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                8,
                &mut tr,
                HM_BINARY,
            )
        };
        assert_eq!(tc, -1);
        assert_eq!(tc, tr);
        let sc = unsafe { snap_map(hc, elemsize, KeyKind::Raw, false) };
        let sr = unsafe { snap_map(hr, elemsize, KeyKind::Raw, false) };
        assert_eq!(sc, sr, "hmget_key_ts(NULL,{})", elemsize);
        assert_eq!(sc.length, 1);
        assert!(sc.table.is_none());
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn c26_hmget_on_table_less_array() {
    let (c, r) = both();
    let elemsize = 8usize;
    let hc = unsafe { (c.hmput_default)(std::ptr::null_mut(), elemsize) };
    let hr = unsafe { (r.hmput_default)(std::ptr::null_mut(), elemsize) };
    let mut key = key_u64(1234);
    let mut tc: isize = 9;
    let mut tr: isize = 9;
    let hc2 = unsafe {
        (c.hmget_key_ts)(
            hc,
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            8,
            &mut tc,
            HM_BINARY,
        )
    };
    let hr2 = unsafe {
        (r.hmget_key_ts)(
            hr,
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            8,
            &mut tr,
            HM_BINARY,
        )
    };
    assert_eq!((tc, hc2 == hc), (tr, hr2 == hr));
    assert_eq!(tc, -1);
    unsafe {
        (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

#[test]
fn c27_c29_binary_get_present_and_absent() {
    let (c, r) = both();
    for &(elemsize, keysize) in &[(8usize, 4usize), (8, 8), (16, 8), (24, 4), (4, 4)] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("bin-get e{} k{}", elemsize, keysize);
        let mut m = MapPair::new(&c, &r, elemsize, keysize, HM_BINARY, ShMode::Implicit, &label);
        let mut rng = Rng::new(0x77 + elemsize as u64);
        let mut present: Vec<Vec<u8>> = Vec::new();
        for i in 0..40u64 {
            let mut k = rng.bytes(keysize.max(1));
            if k.len() < 8 {
                // keep keys distinct
                k[0] = i as u8;
            } else {
                k[..8].copy_from_slice(&key_u64(i));
            }
            let mut kk = k.clone();
            m.put(&mut kk, i, &format!("key {}", i));
            present.push(k);
        }
        for (i, k) in present.clone().iter().enumerate() {
            let mut kk = k.clone();
            let t1 = m.get(&mut kk, &format!("present {}", i));
            let mut kk = k.clone();
            let t2 = m.get_ts(&mut kk, &format!("present_ts {}", i));
            assert_eq!(t1, t2);
            assert!(t1 >= 0, "present key must be found");
        }
        for i in 0..40u64 {
            let mut k = vec![0u8; keysize.max(1)];
            k[0] = 0xF0 | (i as u8 & 0x0f);
            if keysize >= 8 {
                k[..8].copy_from_slice(&key_u64(0xFFFF_0000 + i));
            }
            if present.iter().any(|p| p[..] == k[..]) {
                continue;
            }
            let t = m.get_ts(&mut k, &format!("absent {}", i));
            assert_eq!(t, -1, "absent key must report -1");
        }
        m.free();
    }
}

#[test]
fn c28_string_get_present_and_absent() {
    let (c, r) = both();
    for shmode in [ShMode::Implicit, ShMode::Explicit(SH_STRDUP), ShMode::Explicit(SH_ARENA)] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("str-get {:?}", shmode);
        let mut m = MapPair::new(&c, &r, 16, 8, HM_STRING, shmode, &label);
        let mut keys: Vec<Box<[u8]>> = Vec::new();
        for i in 0..40usize {
            let mut k: Box<[u8]> = format!("key_{:03}\0", i).into_bytes().into_boxed_slice();
            m.put(&mut k, i as u64, &format!("skey {}", i));
            keys.push(k);
        }
        for (i, k) in keys.iter().enumerate() {
            let mut kk = k.clone();
            let t = m.get_ts(&mut kk, &format!("present {}", i));
            assert!(t >= 0);
        }
        for i in 0..20usize {
            let mut kk: Box<[u8]> = format!("nope_{:03}\0", i).into_bytes().into_boxed_slice();
            let t = m.get_ts(&mut kk, &format!("absent {}", i));
            assert_eq!(t, -1);
        }
        // SH_DEFAULT stores the caller's pointer, so the buffers must outlive
        // the map.
        m.keep = keys;
        m.free();
    }
}

// ===========================================================================
// C30..C35  hmput_key, binary mode
// ===========================================================================

#[test]
fn c30_c31_binary_put_growth_boundaries() {
    let (c, r) = both();
    // used_count_threshold for slot_count 8 is 6, for 16 it is 12 -> N around
    // those values crosses both grows.
    for &n in &[1usize, 2, 5, 6, 7, 8, 11, 12, 13, 14, 24, 25, 26] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("bin-grow n={}", n);
        let mut m = MapPair::new(&c, &r, 8, 8, HM_BINARY, ShMode::Implicit, &label);
        for i in 0..n as u64 {
            let mut k = key_u64(i.wrapping_mul(0x9E37_79B9));
            m.put(&mut k, i, &format!("i={}", i));
        }
        m.free();
    }
}

#[test]
fn c32_binary_put_many() {
    let (c, r) = both();
    for &n in &[100usize, 1000] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("bin-many n={}", n);
        let mut m = MapPair::new(&c, &r, 16, 8, HM_BINARY, ShMode::Implicit, &label);
        let mut rng = Rng::new(0xDEAD_0000 + n as u64);
        for i in 0..n as u64 {
            let mut k = key_u64(rng.next_u64());
            m.put(&mut k, i, &format!("i={}", i));
        }
        m.free();
    }
}

#[test]
fn c33_binary_reput_existing_key() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    let mut m = MapPair::new(&c, &r, 16, 8, HM_BINARY, ShMode::Implicit, "reput");
    let mut idx = Vec::new();
    for i in 0..30u64 {
        let mut k = key_u64(i);
        idx.push(m.put(&mut k, i, &format!("first {}", i)));
    }
    for round in 0..4u64 {
        for i in 0..30u64 {
            let mut k = key_u64(i);
            let t = m.put(&mut k, 1000 + round * 100 + i, &format!("re {} {}", round, i));
            assert_eq!(t, idx[i as usize], "re-put must reuse the same index");
        }
    }
    m.free();
}

#[test]
fn c34_c65_elemsize_keysize_matrix() {
    let (c, r) = both();
    for &elemsize in &[8usize, 12, 16, 24, 32] {
        for &keysize in &[1usize, 2, 4, 8, 16] {
            if keysize > elemsize {
                continue;
            }
            sync_seed(&c, &r, 0x3141_5926);
            let label = format!("matrix e{} k{}", elemsize, keysize);
            let mut m =
                MapPair::new(&c, &r, elemsize, keysize, HM_BINARY, ShMode::Implicit, &label);
            let mut rng = Rng::new(0x100 + (elemsize * 100 + keysize) as u64);
            let n = if keysize == 1 { 60 } else { 120 };
            let mut seen: Vec<Vec<u8>> = Vec::new();
            for i in 0..n {
                let mut k = rng.bytes(keysize);
                if seen.iter().any(|s| s[..] == k[..]) {
                    continue;
                }
                seen.push(k.clone());
                m.put(&mut k, i as u64, &format!("i={}", i));
            }
            for (i, k) in seen.clone().iter().enumerate() {
                let mut kk = k.clone();
                assert!(m.get_ts(&mut kk, &format!("hit {}", i)) >= 0);
            }
            m.free();
        }
    }
}

#[test]
fn c35_keysize_zero_all_keys_equal() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    // memcmp(...,0) == 0 for every pair -> only the hash distinguishes keys,
    // and hash_bytes(p,0,seed) is constant, so every put hits the same slot.
    let mut m = MapPair::new(&c, &r, 8, 0, HM_BINARY, ShMode::Implicit, "keysize0");
    for i in 0..20u64 {
        let mut k = key_u64(i);
        let t = m.put(&mut k, i, &format!("i={}", i));
        assert_eq!(t, 0, "every key collapses onto element 1");
    }
    let mut k = key_u64(999);
    assert_eq!(m.get_ts(&mut k, "any"), 0);
    m.free();
}

// ===========================================================================
// C36..C38  hmput_key, string / out-of-range modes on a fresh table
// ===========================================================================

#[test]
fn c36_string_mode_fresh_table_gets_sh_default() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    let mut m = MapPair::new(&c, &r, 16, 8, HM_STRING, ShMode::Implicit, "fresh-str");
    let mut keys = Vec::new();
    for i in 0..25usize {
        let mut k: Box<[u8]> = format!("s{}\0", i).into_bytes().into_boxed_slice();
        m.put(&mut k, i as u64, &format!("i={}", i));
        keys.push(k);
    }
    let (sc, _) = m.snap_both();
    assert_eq!(sc.table.unwrap().str_mode, SH_DEFAULT as u8);
    m.keep = keys;
    m.free();
}

#[test]
fn c37_ptr_to_string_mode_fresh_table() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    // mode == 2 is >= STBDS_HM_STRING, so the fresh table also gets
    // SH_DEFAULT and keys hash/compare as strings.
    let mut m = MapPair::new(&c, &r, 16, 8, HM_PTR_TO_STRING, ShMode::Implicit, "mode2");
    let mut keys = Vec::new();
    for i in 0..25usize {
        let mut k: Box<[u8]> = format!("p{}\0", i).into_bytes().into_boxed_slice();
        m.put(&mut k, i as u64, &format!("i={}", i));
        keys.push(k);
    }
    let (sc, _) = m.snap_both();
    assert_eq!(sc.table.unwrap().str_mode, SH_DEFAULT as u8);
    for (i, k) in keys.iter().enumerate() {
        let mut kk = k.clone();
        assert!(m.get_ts(&mut kk, &format!("hit {}", i)) >= 0);
    }
    m.keep = keys;
    m.free();
}

#[test]
fn c38_negative_mode_is_binary() {
    let (c, r) = both();
    for &mode in &[-1i32, -2, i32::MIN, i32::MIN + 1] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("neg-mode {}", mode);
        let mut m = MapPair::new(&c, &r, 8, 8, mode, ShMode::Implicit, &label);
        for i in 0..20u64 {
            let mut k = key_u64(i.wrapping_mul(7919));
            m.put(&mut k, i, &format!("i={}", i));
        }
        let (sc, _) = m.snap_both();
        assert_eq!(sc.table.unwrap().str_mode, 0, "negative mode -> binary");
        for i in 0..20u64 {
            let mut k = key_u64(i.wrapping_mul(7919));
            assert!(m.get_ts(&mut k, &format!("hit {}", i)) >= 0);
        }
        m.free();
    }
}

// ===========================================================================
// C39..C44  stbds_shmode_func
// ===========================================================================

#[test]
fn c39_sh_strdup() {
    let (c, r) = both();
    for &n in &[1usize, 8, 20, 60] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("strdup n={}", n);
        let mut m = MapPair::new(&c, &r, 16, 8, HM_STRING, ShMode::Explicit(SH_STRDUP), &label);
        for i in 0..n {
            let mut k: Box<[u8]> = format!("dup_{:04}\0", i).into_bytes().into_boxed_slice();
            m.put(&mut k, i as u64, &format!("i={}", i));
            // temp_key must point at the library's own copy with equal content
            let (tc, tr) = m.temp_key_strs();
            assert_eq!(tc, k[..k.len() - 1].to_vec(), "C temp_key");
            assert_eq!(tc, tr, "temp_key content diverged");
        }
        m.free();
    }
}

#[test]
fn c40_sh_arena() {
    let (c, r) = both();
    for &n in &[1usize, 8, 40, 200] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("arena n={}", n);
        let mut m = MapPair::new(&c, &r, 16, 8, HM_STRING, ShMode::Explicit(SH_ARENA), &label);
        let mut rng = Rng::new(0x555 + n as u64);
        for i in 0..n {
            // vary the length so arena blocks are crossed
            let extra = rng.below(40);
            let mut k: Box<[u8]> = format!("arena_{:05}{}\0", i, "x".repeat(extra))
                .into_bytes()
                .into_boxed_slice();
            m.put(&mut k, i as u64, &format!("i={}", i));
        }
        let (sc, _) = m.snap_both();
        let t = sc.table.unwrap();
        assert_eq!(t.str_mode, SH_ARENA as u8);
        assert!(!t.str_storage_null);
        m.free();
    }
}

#[test]
fn c41_sh_default_explicit() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    let mut m = MapPair::new(&c, &r, 24, 8, HM_STRING, ShMode::Explicit(SH_DEFAULT), "shdefault");
    let mut keys = Vec::new();
    for i in 0..50usize {
        let mut k: Box<[u8]> = format!("d_{:04}\0", i).into_bytes().into_boxed_slice();
        m.put(&mut k, i as u64, &format!("i={}", i));
        keys.push(k);
    }
    m.keep = keys;
    m.free();
}

#[test]
fn c42_c43_out_of_enum_string_mode_uses_memcpy() {
    let (c, r) = both();
    // string.mode 0 / 4 / 255 all fall into the `default:` memcpy arm.
    // Driven with mode == HM_BINARY so nothing ever tries to dereference the
    // raw key bytes as a `char *`.
    for &sm in &[SH_NONE, 4, 7, 255] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("shmode {} + binary", sm);
        let mut m = MapPair::new(&c, &r, 16, 8, HM_BINARY, ShMode::Explicit(sm), &label);
        let (sc, sr) = m.snap_both();
        assert_eq!(sc, sr, "shmode_func({}) initial state", sm);
        assert_eq!(sc.table.as_ref().unwrap().str_mode, sm as u8);
        assert_eq!(sc.length, 1);
        for i in 0..40u64 {
            let mut k = key_u64(i.wrapping_mul(0x1234_5678_9ABC_DEF1));
            m.put(&mut k, i, &format!("i={}", i));
        }
        for i in 0..40u64 {
            let mut k = key_u64(i.wrapping_mul(0x1234_5678_9ABC_DEF1));
            assert!(m.get_ts(&mut k, &format!("hit {}", i)) >= 0);
        }
        m.free();
    }

    // The genuine quirk: mode >= HM_STRING (string hashing) on a table whose
    // string.mode is out of enum range -> the key is memcpy'd as raw bytes
    // rather than stored as a pointer (ERRORS.md E37). A single insert is
    // enough; any further probe that hash-collides would make the C code
    // dereference those raw bytes as a `char *`, which is UB in the original.
    for &sm in &[SH_NONE, 4, 255] {
        sync_seed(&c, &r, 0x3141_5926);
        let elemsize = 16usize;
        let mut key: Vec<u8> = b"abcdefghij\0".to_vec();
        let hc = unsafe { (c.shmode_func)(elemsize, sm) };
        let hr = unsafe { (r.shmode_func)(elemsize, sm) };
        let hc = unsafe {
            (c.hmput_key)(
                hc,
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                8,
                HM_STRING,
            )
        };
        let hr = unsafe {
            (r.hmput_key)(
                hr,
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                8,
                HM_STRING,
            )
        };
        // compare only the memcpy'd key bytes (the rest of the element is
        // uninitialised caller scratch in both libraries)
        unsafe {
            let pc = (hc as *const u8).add(0); // element 1 == hash-biased base
            let pr = (hr as *const u8).add(0);
            let bc: Vec<u8> = (0..8).map(|k| *pc.add(k)).collect();
            let br: Vec<u8> = (0..8).map(|k| *pr.add(k)).collect();
            assert_eq!(bc, br, "raw memcpy'd key, shmode {}", sm);
            assert_eq!(bc, key[..8].to_vec(), "shmode {}", sm);
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn c44_shmode_func_truncates_mode() {
    let (c, r) = both();
    // (unsigned char) mode
    for &(m, expect) in &[
        (256i32, 0u8),
        (257, 1),
        (-1, 255),
        (1000, 232),
        (258, 2),
        (259, 3),
        (65536, 0),
        (i32::MIN, 0),
        (i32::MAX, 255),
    ] {
        sync_seed(&c, &r, 0x3141_5926);
        let elemsize = 16usize;
        let hc = unsafe { (c.shmode_func)(elemsize, m) };
        let hr = unsafe { (r.shmode_func)(elemsize, m) };
        let sc = unsafe { snap_map(hc, elemsize, KeyKind::Raw, false) };
        let sr = unsafe { snap_map(hr, elemsize, KeyKind::Raw, false) };
        assert_eq!(sc, sr, "shmode_func({}, ...)", m);
        assert_eq!(
            sc.table.as_ref().unwrap().str_mode,
            expect,
            "shmode_func mode {} must truncate to {}",
            m,
            expect
        );
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// C45..C53  stbds_hmdel_key
// ===========================================================================

#[test]
fn c45_delete_final_element() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    let mut m = MapPair::new(&c, &r, 16, 8, HM_BINARY, ShMode::Implicit, "del-final");
    let mut keys = Vec::new();
    for i in 0..20u64 {
        let mut k = key_u64(i.wrapping_mul(31337));
        m.put(&mut k, i, &format!("i={}", i));
        keys.push(k);
    }
    // delete in reverse insertion order: each victim is the final element
    for i in (0..20usize).rev() {
        let mut k = keys[i];
        let len = m.length();
        let t = m.get_ts(&mut k, "probe");
        assert_eq!(t, (len - 2) as isize, "victim must be the final element");
        m.del(&mut k, 0, &format!("rev {}", i));
    }
    assert_eq!(m.length(), 1);
    m.free();
}

#[test]
fn c46_delete_middle_element() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    let mut m = MapPair::new(&c, &r, 16, 8, HM_BINARY, ShMode::Implicit, "del-mid");
    let mut keys = Vec::new();
    for i in 0..40u64 {
        let mut k = key_u64(i.wrapping_mul(0xABCD));
        m.put(&mut k, i, &format!("i={}", i));
        keys.push(k);
    }
    // delete in insertion order: the victim is (almost) never the final one,
    // so the memmove + re-find + index patch path runs.
    for i in 0..40usize {
        let mut k = keys[i];
        m.del(&mut k, 0, &format!("fwd {}", i));
    }
    assert_eq!(m.length(), 1);
    m.free();
}

#[test]
fn c47_delete_with_nonzero_keyoffset() {
    let (c, r) = both();
    // `hmput_key` always stores the key at offset 0; `hmdel_key` is the only
    // function taking a keyoffset. Mirror the key at `keyoffset` so the
    // deletion can actually find it.
    for &(elemsize, keysize, keyoffset) in &[(16usize, 4usize, 8usize), (24, 8, 8), (32, 4, 20)] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("keyoffset e{} k{} o{}", elemsize, keysize, keyoffset);
        let mut m = MapPair::new(&c, &r, elemsize, keysize, HM_BINARY, ShMode::Implicit, &label);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..30u64 {
            let mut k = vec![0u8; keysize];
            for (j, b) in k.iter_mut().enumerate() {
                *b = (i as u8).wrapping_mul(17).wrapping_add(j as u8);
            }
            let mut kk = k.clone();
            let t = m.put(&mut kk, i, &format!("i={}", i));
            m.mirror_key_at((t + 1) as usize, keyoffset, &k);
            m.check(&format!("mirror {}", i));
            keys.push(k);
        }
        // Deleting with keyoffset also re-finds using the mirrored copy, so the
        // whole memmove/patch path is exercised through the offset key.
        for i in 0..30usize {
            let mut k = keys[i].clone();
            m.del(&mut k, keyoffset, &format!("del {}", i));
        }
        assert_eq!(m.length(), 1);
        m.free();
    }
}

#[test]
fn c48_delete_triggers_shrink() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    // grow well past slot_count 8 then delete almost everything so
    // used_count < used_count_shrink_threshold and slot_count > 8.
    let mut m = MapPair::new(&c, &r, 16, 8, HM_BINARY, ShMode::Implicit, "shrink");
    let mut keys = Vec::new();
    for i in 0..200u64 {
        let mut k = key_u64(i.wrapping_mul(0x5DEECE66D));
        m.put(&mut k, i, &format!("i={}", i));
        keys.push(k);
    }
    let (sc, _) = m.snap_both();
    assert!(sc.table.as_ref().unwrap().slot_count >= 256);
    let mut shrank = false;
    let mut prev = sc.table.unwrap().slot_count;
    for i in 0..200usize {
        let mut k = keys[i];
        m.del(&mut k, 0, &format!("del {}", i));
        let (sc, _) = m.snap_both();
        let now = sc.table.unwrap().slot_count;
        if now < prev {
            shrank = true;
        }
        prev = now;
    }
    assert!(shrank, "the shrink branch must have been taken");
    assert_eq!(m.length(), 1);
    m.free();
}

#[test]
fn c49_delete_insert_churn_triggers_rebuild() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    // Keep used_count high (so no shrink) while accumulating tombstones until
    // tombstone_count > tombstone_count_threshold forces a same-size rebuild.
    let mut m = MapPair::new(&c, &r, 16, 8, HM_BINARY, ShMode::Implicit, "rebuild");
    let mut live: Vec<[u8; 8]> = Vec::new();
    for i in 0..120u64 {
        let mut k = key_u64(i);
        m.put(&mut k, i, &format!("seed {}", i));
        live.push(k);
    }
    let mut next = 1000u64;
    let mut rebuilt = false;
    for step in 0..600usize {
        // delete one, insert one -> used_count stays put, tombstones grow
        let victim = live.remove(step % live.len().max(1));
        let (before, _) = m.snap_both();
        let tb = before.table.as_ref().unwrap().tombstone_count;
        let mut v = victim;
        m.del(&mut v, 0, &format!("churn del {}", step));
        let (after, _) = m.snap_both();
        if after.table.as_ref().unwrap().tombstone_count < tb {
            rebuilt = true;
        }
        let mut k = key_u64(next);
        m.put(&mut k, next, &format!("churn put {}", step));
        live.push(k);
        next += 1;
    }
    assert!(rebuilt, "the tombstone rebuild branch must have been taken");
    m.free();
}

#[test]
fn c50_delete_strdup_frees_key() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    let mut m = MapPair::new(&c, &r, 16, 8, HM_STRING, ShMode::Explicit(SH_STRDUP), "del-strdup");
    let mut keys: Vec<Box<[u8]>> = Vec::new();
    for i in 0..60usize {
        let mut k: Box<[u8]> = format!("sd_{:04}\0", i).into_bytes().into_boxed_slice();
        m.put(&mut k, i as u64, &format!("i={}", i));
        keys.push(k);
    }
    for i in 0..60usize {
        let mut k = keys[i].clone();
        m.del(&mut k, 0, &format!("del {}", i));
    }
    assert_eq!(m.length(), 1);
    m.free();
}

#[test]
fn c51_delete_arena_and_default() {
    let (c, r) = both();
    for shmode in [ShMode::Explicit(SH_ARENA), ShMode::Explicit(SH_DEFAULT), ShMode::Implicit] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("del-str {:?}", shmode);
        let mut m = MapPair::new(&c, &r, 24, 8, HM_STRING, shmode, &label);
        let mut keys: Vec<Box<[u8]>> = Vec::new();
        for i in 0..80usize {
            let mut k: Box<[u8]> = format!("ka_{:05}\0", i).into_bytes().into_boxed_slice();
            m.put(&mut k, i as u64, &format!("i={}", i));
            keys.push(k);
        }
        // interleave forward and reverse deletion so both the
        // old_index == final_index and old_index != final_index paths run
        for i in 0..40usize {
            let mut k = keys[i].clone();
            m.del(&mut k, 0, &format!("fwd {}", i));
            let mut k = keys[79 - i].clone();
            m.del(&mut k, 0, &format!("rev {}", i));
        }
        assert_eq!(m.length(), 1);
        m.keep = keys;
        m.free();
    }
}

#[test]
fn c52_mode2_delete_last_element_only() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    // mode == 2 finds the slot with STRING hashing but `mode == STBDS_HM_STRING`
    // is false, so the strdup'd key is NOT freed and the post-memmove re-find
    // takes the BINARY branch on a `char *` field. That re-find hashes raw
    // pointer bytes as a string and then trips `STBDS_ASSERT(slot >= 0)` in the
    // C build (asserts are live -- see ERRORS.md E15/E18), so only the
    // old_index == final_index shape (which skips the re-find entirely) is
    // reachable without aborting.
    let mut m = MapPair::new(&c, &r, 16, 8, HM_PTR_TO_STRING, ShMode::Explicit(SH_STRDUP), "mode2-del");
    let mut keys: Vec<Box<[u8]>> = Vec::new();
    for i in 0..30usize {
        let mut k: Box<[u8]> = format!("m2_{:04}\0", i).into_bytes().into_boxed_slice();
        m.put(&mut k, i as u64, &format!("i={}", i));
        keys.push(k);
    }
    // absent key -> find_slot < 0 -> early return, temp stays 0
    let mut absent: Box<[u8]> = b"m2_absent\0".to_vec().into_boxed_slice();
    m.del(&mut absent, 0, "absent");
    assert_eq!(m.temp(), 0);
    // reverse order: every victim is the final element
    for i in (0..30usize).rev() {
        let mut k = keys[i].clone();
        let len = m.length();
        assert_eq!(m.get_ts(&mut k, "probe").max(-1), (len - 2) as isize);
        let mut k = keys[i].clone();
        m.del(&mut k, 0, &format!("rev {}", i));
        assert_eq!(m.temp(), 1, "temp must report 1 removed");
    }
    assert_eq!(m.length(), 1);
    m.free();
}

#[test]
fn c53_delete_everything_stepwise() {
    let (c, r) = both();
    for &n in &[8usize, 33, 130] {
        for order in 0..3 {
            sync_seed(&c, &r, 0x3141_5926);
            let label = format!("delall n={} order={}", n, order);
            let mut m = MapPair::new(&c, &r, 16, 8, HM_BINARY, ShMode::Implicit, &label);
            let mut keys = Vec::new();
            for i in 0..n as u64 {
                let mut k = key_u64(i.wrapping_mul(0x2545F4914F6CDD1D));
                m.put(&mut k, i, &format!("i={}", i));
                keys.push(k);
            }
            let mut rng = Rng::new(0x9000 + n as u64 + order);
            match order {
                0 => {}
                1 => keys.reverse(),
                _ => {
                    for i in (1..keys.len()).rev() {
                        let j = rng.below(i + 1);
                        keys.swap(i, j);
                    }
                }
            }
            for (i, k) in keys.clone().iter().enumerate() {
                let mut kk = *k;
                m.del(&mut kk, 0, &format!("del {}", i));
                // deleting an already-absent key must be a no-op
                let mut kk = *k;
                m.del(&mut kk, 0, &format!("redel {}", i));
            }
            assert_eq!(m.length(), 1);
            m.free();
        }
    }
}

// ===========================================================================
// C54..C55  randomized full pipelines
// ===========================================================================

fn pipeline_binary(c: &Lib, r: &Lib, elemsize: usize, keysize: usize, seed: usize, ops: usize) {
    sync_seed(c, r, seed);
    let label = format!("pipeline-bin e{} k{} s{:#x}", elemsize, keysize, seed);
    let mut m = MapPair::new(c, r, elemsize, keysize, HM_BINARY, ShMode::Implicit, &label);
    let mut rng = Rng::new(seed as u64 ^ (elemsize as u64) << 8 ^ keysize as u64);
    let mut live: Vec<Vec<u8>> = Vec::new();
    let universe = 64usize;
    for step in 0..ops {
        // small key universe so collisions, re-puts and re-deletes are common
        let mut k = vec![0u8; keysize];
        let id = rng.below(universe) as u64;
        for (j, b) in k.iter_mut().enumerate() {
            *b = ((id.wrapping_mul(0x9E37_79B9)) >> (8 * (j % 8))) as u8;
        }
        match rng.below(10) {
            0..=4 => {
                let mut kk = k.clone();
                m.put(&mut kk, step as u64, &format!("s{} put", step));
                if !live.iter().any(|x| x[..] == k[..]) {
                    live.push(k);
                }
            }
            5..=6 => {
                let mut kk = k.clone();
                m.get(&mut kk, &format!("s{} get", step));
            }
            7 => {
                let mut kk = k.clone();
                m.get_ts(&mut kk, &format!("s{} get_ts", step));
            }
            _ => {
                let mut kk = k.clone();
                m.del(&mut kk, 0, &format!("s{} del", step));
                live.retain(|x| x[..] != k[..]);
            }
        }
    }
    m.free();
}

#[test]
fn c54_pipeline_binary_randomized() {
    let (c, r) = both();
    for &(elemsize, keysize) in &[(8usize, 8usize), (16, 8), (16, 4), (24, 4), (32, 16)] {
        pipeline_binary(&c, &r, elemsize, keysize, 0x3141_5926, 500);
    }
}

fn pipeline_string(c: &Lib, r: &Lib, shmode: ShMode, seed: usize, ops: usize) {
    sync_seed(c, r, seed);
    let label = format!("pipeline-str {:?} s{:#x}", shmode, seed);
    let mut m = MapPair::new(c, r, 24, 8, HM_STRING, shmode, &label);
    let mut rng = Rng::new(seed as u64 ^ 0xABCD);
    // Stable key buffers. SH_DEFAULT stores the caller's pointer verbatim,
    // so the buffers MUST outlive the map (and must not be reallocated) --
    // leak them for the duration of the test.
    let universe: Vec<&'static mut [u8]> = (0..48usize)
        .map(|i| {
            Box::leak(
                format!("pk_{:04}\0", i)
                    .into_bytes()
                    .into_boxed_slice(),
            ) as &'static mut [u8]
        })
        .collect();
    let ptrs: Vec<*mut u8> = universe.iter().map(|b| b.as_ptr() as *mut u8).collect();
    let klen = universe[0].len();
    for step in 0..ops {
        let kp = ptrs[rng.below(ptrs.len())];
        let k: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(kp, klen) };
        match rng.below(10) {
            0..=4 => {
                m.put(k, step as u64, &format!("s{} put", step));
            }
            5..=7 => {
                m.get_ts(k, &format!("s{} get_ts", step));
            }
            _ => {
                m.del(k, 0, &format!("s{} del", step));
            }
        }
    }
    m.free();
}

#[test]
fn c55_pipeline_string_randomized() {
    let (c, r) = both();
    for shmode in [
        ShMode::Implicit,
        ShMode::Explicit(SH_DEFAULT),
        ShMode::Explicit(SH_STRDUP),
        ShMode::Explicit(SH_ARENA),
    ] {
        pipeline_string(&c, &r, shmode, 0x3141_5926, 400);
    }
}

// ===========================================================================
// C56  stbds_hmfree_func
// ===========================================================================

#[test]
fn c56_hmfree_shapes() {
    let (c, r) = both();
    // (a) NULL is a no-op
    unsafe {
        (c.hmfree_func)(std::ptr::null_mut(), 8);
        (r.hmfree_func)(std::ptr::null_mut(), 8);
    }
    // (b) array with no hash table at all
    for &elemsize in &[8usize, 16] {
        let hc = unsafe { (c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let hr = unsafe { (r.hmput_default)(std::ptr::null_mut(), elemsize) };
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
    // (c) every string.mode, empty and non-empty
    for &sm in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &n in &[0usize, 1, 25] {
            for &elemsize in &[16usize, 24] {
                sync_seed(&c, &r, 0x3141_5926);
                let mode = if sm == SH_NONE { HM_BINARY } else { HM_STRING };
                let label = format!("free sm={} n={} e={}", sm, n, elemsize);
                let mut m =
                    MapPair::new(&c, &r, elemsize, 8, mode, ShMode::Explicit(sm), &label);
                let mut keys: Vec<Box<[u8]>> = Vec::new();
                for i in 0..n {
                    if sm == SH_NONE {
                        let mut k = key_u64(i as u64);
                        m.put(&mut k, i as u64, &format!("i={}", i));
                    } else {
                        let mut k: Box<[u8]> =
                            format!("f_{:04}\0", i).into_bytes().into_boxed_slice();
                        m.put(&mut k, i as u64, &format!("i={}", i));
                        keys.push(k);
                    }
                }
                m.keep = keys;
                m.free();
            }
        }
    }
    // (d) implicit-mode maps
    for &mode in &[HM_BINARY, HM_STRING] {
        sync_seed(&c, &r, 0x3141_5926);
        let mut m = MapPair::new(&c, &r, 16, 8, mode, ShMode::Implicit, "free implicit");
        let mut keys: Vec<Box<[u8]>> = Vec::new();
        for i in 0..12usize {
            if mode == HM_BINARY {
                let mut k = key_u64(i as u64);
                m.put(&mut k, i as u64, &format!("i={}", i));
            } else {
                let mut k: Box<[u8]> = format!("g_{:04}\0", i).into_bytes().into_boxed_slice();
                m.put(&mut k, i as u64, &format!("i={}", i));
                keys.push(k);
            }
        }
        m.keep = keys;
        m.free();
    }
}

// ===========================================================================
// C64  seed interaction
// ===========================================================================

#[test]
fn c64_seed_sweep_over_pipeline() {
    let (c, r) = both();
    for &seed in &[0usize, 1, 0x3141_5926, usize::MAX, 0xDEAD_BEEF_CAFE_BABE, 2, 0xFFFF] {
        pipeline_binary(&c, &r, 16, 8, seed, 300);
        pipeline_string(&c, &r, ShMode::Explicit(SH_STRDUP), seed, 200);
        pipeline_string(&c, &r, ShMode::Explicit(SH_ARENA), seed, 200);
    }
    sync_seed(&c, &r, 0x3141_5926);
}

// ===========================================================================
// Cross-check: keys built with the library's own `strkey` helper
// ===========================================================================

#[test]
fn strkey_driven_string_map() {
    let (c, r) = both();
    for shmode in [ShMode::Explicit(SH_STRDUP), ShMode::Explicit(SH_ARENA)] {
        sync_seed(&c, &r, 0x3141_5926);
        let label = format!("strkey {:?}", shmode);
        let mut m = MapPair::new(&c, &r, 16, 8, HM_STRING, shmode, &label);
        for i in 0..64i32 {
            // use the C library's own formatting, then hand the same buffer to
            // both libraries
            let p = unsafe { (c.strkey)(i) };
            let mut k: Box<[u8]> = {
                let mut v = unsafe { cstr_bytes(p) };
                v.push(0);
                v.into_boxed_slice()
            };
            let pr = unsafe { (r.strkey)(i) };
            assert_eq!(unsafe { cstr_bytes(pr) }, k[..k.len() - 1].to_vec());
            m.put(&mut k, i as u64, &format!("strkey {}", i));
        }
        for i in 0..64i32 {
            let mut v = unsafe { cstr_bytes((c.strkey)(i)) };
            v.push(0);
            let mut k = v.into_boxed_slice();
            assert!(m.get_ts(&mut k, &format!("hit {}", i)) >= 0);
        }
        m.free();
    }
}

/// Silence the unused-import warning for `c_char` when only some cfgs use it.
#[allow(dead_code)]
fn _unused(_: *mut c_char) {}
