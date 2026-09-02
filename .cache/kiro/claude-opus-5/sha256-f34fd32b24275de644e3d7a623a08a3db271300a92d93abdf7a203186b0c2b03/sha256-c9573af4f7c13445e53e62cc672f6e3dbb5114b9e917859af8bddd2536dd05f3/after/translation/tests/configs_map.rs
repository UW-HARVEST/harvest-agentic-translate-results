//! Phase B — CONFIGS.md rows 25..59 and 63..65: the hash-map entry points
//! (`stbds_hmput_default`, `stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmdel_key`, `stbds_hmfree_func`,
//! `stbds_shmode_func`) driven exactly the way the `stbds_hm*`/`stbds_sh*`
//! macros drive them.

mod common;

use common::*;
use std::collections::HashSet;
use std::ffi::{c_int, c_void, CString};

/// A pair of maps kept in lockstep, one per shared object.
struct Dual<'a> {
    c: &'a Api,
    r: &'a Api,
    pc: *mut c_void,
    pr: *mut c_void,
    shape: MapShape,
    /// `mode` argument handed to `hmput_key`/`hmget_key`/`hmdel_key`.
    mode: c_int,
    /// compare `temp_key` too (only meaningful for real string modes)
    check_temp_key: bool,
}

impl<'a> Dual<'a> {
    /// Map created implicitly by the first `hmput_key`/`hmget_key` call.
    fn implicit(
        c: &'a Api,
        r: &'a Api,
        shape: MapShape,
        mode: c_int,
        seed: usize,
        check_temp_key: bool,
    ) -> Dual<'a> {
        unsafe {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
        }
        Dual {
            c,
            r,
            pc: std::ptr::null_mut(),
            pr: std::ptr::null_mut(),
            shape,
            mode,
            check_temp_key,
        }
    }

    /// Map created explicitly by `stbds_shmode_func`.
    fn shmode(
        c: &'a Api,
        r: &'a Api,
        shape: MapShape,
        mode: c_int,
        shmode: c_int,
        seed: usize,
        check_temp_key: bool,
    ) -> Dual<'a> {
        unsafe {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let pc = (c.shmode_func)(shape.elemsize, shmode);
            let pr = (r.shmode_func)(shape.elemsize, shmode);
            let d = Dual {
                c,
                r,
                pc,
                pr,
                shape,
                mode,
                check_temp_key,
            };
            d.check("after shmode_func");
            d
        }
    }

    fn check(&self, label: &str) {
        unsafe {
            same(
                label,
                &dump_map(self.pc, self.shape),
                &dump_map(self.pr, self.shape),
            );
        }
    }

    /// `stbds_temp_key` is only comparable when we know it was written by this
    /// operation: `stbds_make_hash_index` leaves the field uninitialised, so
    /// after a rehash it holds heap garbage.  The caller primes it to 0 before
    /// the call and only compares when no rehash happened.
    fn check_temp_key_now(&self, label: &str) {
        if self.check_temp_key && !self.pc.is_null() {
            unsafe {
                same(
                    &format!("{label} [temp_key]"),
                    &dump_temp_key(self.pc, self.shape.elemsize),
                    &dump_temp_key(self.pr, self.shape.elemsize),
                );
            }
        }
    }

    fn put(&mut self, key: *mut c_void, tag: u64, label: &str) {
        unsafe {
            // prime `temp_key` so that "was it written?" is observable
            let (tab_c_before, tab_r_before) = if self.pc.is_null() {
                (std::ptr::null_mut(), std::ptr::null_mut())
            } else {
                set_temp_key(self.pc, self.shape.elemsize, 0);
                set_temp_key(self.pr, self.shape.elemsize, 0);
                (
                    map_table(self.pc, self.shape.elemsize),
                    map_table(self.pr, self.shape.elemsize),
                )
            };
            self.pc = (self.c.hmput_key)(
                self.pc,
                self.shape.elemsize,
                key,
                self.shape.keysize,
                self.mode,
            );
            self.pr = (self.r.hmput_key)(
                self.pr,
                self.shape.elemsize,
                key,
                self.shape.keysize,
                self.mode,
            );
            let tc = arr_temp((self.pc as *mut u8).sub(self.shape.elemsize) as *mut c_void);
            let tr = arr_temp((self.pr as *mut u8).sub(self.shape.elemsize) as *mut c_void);
            assert_eq!(tc, tr, "{label}: hmput_key temp (slot index) differs");
            // emulate the value store of `stbds_hmput` / `stbds_hmputs`
            fill_value(self.pc, self.shape, (tc + 1) as usize, tag);
            fill_value(self.pr, self.shape, (tr + 1) as usize, tag);
            self.check(label);
            let tab_c = map_table(self.pc, self.shape.elemsize);
            let tab_r = map_table(self.pr, self.shape.elemsize);
            let rehashed_c = tab_c != tab_c_before;
            let rehashed_r = tab_r != tab_r_before;
            assert_eq!(
                rehashed_c, rehashed_r,
                "{label}: one side rehashed and the other did not"
            );
            if !rehashed_c {
                self.check_temp_key_now(label);
            }
        }
    }

    fn get(&mut self, key: *mut c_void, label: &str) -> isize {
        unsafe {
            self.pc = (self.c.hmget_key)(
                self.pc,
                self.shape.elemsize,
                key,
                self.shape.keysize,
                self.mode,
            );
            self.pr = (self.r.hmget_key)(
                self.pr,
                self.shape.elemsize,
                key,
                self.shape.keysize,
                self.mode,
            );
            let tc = arr_temp((self.pc as *mut u8).sub(self.shape.elemsize) as *mut c_void);
            let tr = arr_temp((self.pr as *mut u8).sub(self.shape.elemsize) as *mut c_void);
            assert_eq!(tc, tr, "{label}: hmget_key temp differs");
            self.check(label);
            tc
        }
    }

    fn get_ts(&mut self, key: *mut c_void, label: &str) -> isize {
        unsafe {
            let mut vc: isize = 0x5A5A;
            let mut vr: isize = 0x5A5A;
            self.pc = (self.c.hmget_key_ts)(
                self.pc,
                self.shape.elemsize,
                key,
                self.shape.keysize,
                &mut vc,
                self.mode,
            );
            self.pr = (self.r.hmget_key_ts)(
                self.pr,
                self.shape.elemsize,
                key,
                self.shape.keysize,
                &mut vr,
                self.mode,
            );
            assert_eq!(vc, vr, "{label}: hmget_key_ts *temp differs");
            self.check(label);
            vc
        }
    }

    fn del(&mut self, key: *mut c_void, label: &str) -> isize {
        unsafe {
            // a delete on a STRDUP map frees the string `temp_key` points at
            self.check_temp_key = false;
            let nc = (self.c.hmdel_key)(
                self.pc,
                self.shape.elemsize,
                key,
                self.shape.keysize,
                self.shape.keyoffset,
                self.mode,
            );
            let nr = (self.r.hmdel_key)(
                self.pr,
                self.shape.elemsize,
                key,
                self.shape.keysize,
                self.shape.keyoffset,
                self.mode,
            );
            assert_eq!(
                nc.is_null(),
                nr.is_null(),
                "{label}: hmdel_key NULL-ness differs"
            );
            self.pc = nc;
            self.pr = nr;
            if self.pc.is_null() {
                return 0;
            }
            // `stbds_hmdel` yields `stbds_temp((t)-1)`
            let tc = arr_temp((self.pc as *mut u8).sub(self.shape.elemsize) as *mut c_void);
            let tr = arr_temp((self.pr as *mut u8).sub(self.shape.elemsize) as *mut c_void);
            assert_eq!(tc, tr, "{label}: hmdel_key temp (found flag) differs");
            self.check(label);
            tc
        }
    }

    fn put_default(&mut self, label: &str) {
        unsafe {
            self.pc = (self.c.hmput_default)(self.pc, self.shape.elemsize);
            self.pr = (self.r.hmput_default)(self.pr, self.shape.elemsize);
            self.check(label);
        }
    }

    fn free(self) {
        unsafe {
            hmfree(self.c, self.pc, self.shape.elemsize);
            hmfree(self.r, self.pr, self.shape.elemsize);
        }
    }
}

/// `n` distinct random keys of `keysize` bytes.
fn distinct_keys(rng: &mut Rng, keysize: usize, n: usize) -> Vec<Vec<u8>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let k = rng.bytes(keysize);
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    out
}

/// `n` distinct random NUL-terminated keys.
fn distinct_strings(rng: &mut Rng, n: usize) -> Vec<CString> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let s = rng.ascii_len(1, 24);
        if seen.insert(s.clone()) {
            out.push(s);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// rows 25..27 — stbds_hmput_default
// ---------------------------------------------------------------------------

#[test]
fn row25_hmput_default_null() {
    let (c, r, _g) = both();
    for elemsize in [8usize, 12, 16, 24, 40] {
        let shape = MapShape::bytes(elemsize, elemsize.min(8));
        let mut d = Dual::implicit(c, r, shape, HM_BINARY, 7, false);
        d.put_default("row25 first");
        d.free();
    }
}

#[test]
fn row26_hmput_default_twice() {
    let (c, r, _g) = both();
    for elemsize in [8usize, 16, 40] {
        let shape = MapShape::bytes(elemsize, 8);
        let mut d = Dual::implicit(c, r, shape, HM_BINARY, 11, false);
        d.put_default("row26 #1");
        unsafe {
            // the default element belongs to the caller: write it identically
            fill_value(d.pc, shape, 0, 0xABCD);
            fill_value(d.pr, shape, 0, 0xABCD);
        }
        for i in 0..5 {
            let pc_before = d.pc;
            let pr_before = d.pr;
            d.put_default(&format!("row26 #{}", i + 2));
            assert_eq!(pc_before, d.pc, "C hmput_default must be a no-op");
            assert_eq!(pr_before, d.pr, "R hmput_default must be a no-op");
        }
        d.free();
    }
}

#[test]
fn row27_hmput_default_on_populated() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 27);
    let shape = MapShape::bytes(16, 8);
    let mut d = Dual::implicit(c, r, shape, HM_BINARY, 13, false);
    let keys = distinct_keys(&mut rng, 8, 20);
    for (i, k) in keys.iter().enumerate() {
        let mut kk = k.clone();
        d.put(kk.as_mut_ptr() as *mut c_void, i as u64, &format!("row27 put {i}"));
        d.put_default(&format!("row27 default after {i}"));
    }
    d.free();
}

// ---------------------------------------------------------------------------
// rows 28..32 — stbds_hmget_key / stbds_hmget_key_ts
// ---------------------------------------------------------------------------

#[test]
fn row28_hmget_ts_null_map() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 28);
    for elemsize in [8usize, 16, 40] {
        for mode in [HM_BINARY, HM_STRING] {
            let shape = MapShape::bytes(elemsize, 8);
            let mut d = Dual::implicit(c, r, shape, mode, 17, false);
            let mut k = rng.bytes(8);
            let t = d.get_ts(k.as_mut_ptr() as *mut c_void, "row28 get_ts(NULL)");
            assert_eq!(t, INDEX_EMPTY, "hmget_key_ts(NULL) must yield -1");
            d.free();
        }
    }
}

#[test]
fn row29_hmget_no_index() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 29);
    for mode in [HM_BINARY, HM_STRING] {
        let shape = MapShape::bytes(16, 8);
        let mut d = Dual::implicit(c, r, shape, mode, 19, false);
        // first get bootstraps the array but leaves hash_table == NULL
        let mut k = rng.bytes(8);
        d.get_ts(k.as_mut_ptr() as *mut c_void, "row29 bootstrap");
        assert!(
            unsafe { arr_table((d.pc as *mut u8).sub(16) as *mut c_void) }.is_null(),
            "hash_table must still be NULL"
        );
        for i in 0..10 {
            let mut k2 = rng.bytes(8);
            let t = d.get_ts(k2.as_mut_ptr() as *mut c_void, &format!("row29 ts {i}"));
            assert_eq!(t, -1);
            let t2 = d.get(k2.as_mut_ptr() as *mut c_void, &format!("row29 g {i}"));
            assert_eq!(t2, -1);
        }
        d.free();
    }
}

#[test]
fn row30_row32_binary_hit_and_miss() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 30);
    for (elemsize, keysize) in [(8usize, 4usize), (16, 8), (24, 16), (40, 1), (16, 2)] {
        let shape = MapShape::bytes(elemsize, keysize);
        let mut d = Dual::implicit(c, r, shape, HM_BINARY, 23, false);
        let present = distinct_keys(&mut rng, keysize, 120);
        for (i, k) in present.iter().enumerate() {
            let mut kk = k.clone();
            d.put(
                kk.as_mut_ptr() as *mut c_void,
                i as u64,
                &format!("row30 put e={elemsize} k={keysize} #{i}"),
            );
        }
        for (i, k) in present.iter().enumerate() {
            let mut kk = k.clone();
            let t = d.get(
                kk.as_mut_ptr() as *mut c_void,
                &format!("row30 get hit #{i}"),
            );
            assert!(t >= 0, "key {i} should be present");
            let t2 = d.get_ts(
                kk.as_mut_ptr() as *mut c_void,
                &format!("row30 get_ts hit #{i}"),
            );
            assert_eq!(t, t2);
        }
        let absent: Vec<Vec<u8>> = distinct_keys(&mut rng, keysize, 200)
            .into_iter()
            .filter(|k| !present.contains(k))
            .collect();
        for (i, k) in absent.iter().enumerate() {
            let mut kk = k.clone();
            let t = d.get(
                kk.as_mut_ptr() as *mut c_void,
                &format!("row30 get miss #{i}"),
            );
            assert_eq!(t, INDEX_EMPTY, "absent key must yield -1");
        }
        d.free();
    }
}

#[test]
fn row31_string_hit_and_miss() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 31);
    for shmode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let shape = MapShape::strp(16);
        let mut d = Dual::shmode(c, r, shape, HM_STRING, shmode, 29, true);
        let present = distinct_strings(&mut rng, 120);
        for (i, s) in present.iter().enumerate() {
            d.put(
                s.as_ptr() as *mut c_void,
                i as u64,
                &format!("row31 sh={shmode} put #{i}"),
            );
        }
        for (i, s) in present.iter().enumerate() {
            let t = d.get(s.as_ptr() as *mut c_void, &format!("row31 get hit #{i}"));
            assert!(t >= 0);
            let t2 = d.get_ts(s.as_ptr() as *mut c_void, &format!("row31 ts hit #{i}"));
            assert_eq!(t, t2);
        }
        let absent: Vec<CString> = distinct_strings(&mut rng, 200)
            .into_iter()
            .filter(|s| !present.contains(s))
            .collect();
        for (i, s) in absent.iter().enumerate() {
            let t = d.get(s.as_ptr() as *mut c_void, &format!("row31 get miss #{i}"));
            assert_eq!(t, INDEX_EMPTY);
        }
        d.free();
    }
}

// ---------------------------------------------------------------------------
// rows 33..36, 44 — stbds_hmput_key, binary
// ---------------------------------------------------------------------------

#[test]
fn row33_put_single_binary() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 33);
    for _ in 0..200 {
        let shape = MapShape::bytes(8, 4);
        let mut d = Dual::implicit(c, r, shape, HM_BINARY, rng.next_u64() as usize, false);
        let mut k = rng.bytes(4);
        d.put(k.as_mut_ptr() as *mut c_void, 1, "row33 single put");
        d.free();
    }
}

#[test]
fn row34_put_many_binary_cross_product() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 34);
    for (elemsize, keysize) in [
        (8usize, 1usize),
        (8, 2),
        (8, 4),
        (8, 8),
        (16, 4),
        (16, 8),
        (24, 16),
        (40, 8),
        (40, 32),
    ] {
        let shape = MapShape::bytes(elemsize, keysize);
        let mut d = Dual::implicit(c, r, shape, HM_BINARY, 31, false);
        let n = if keysize == 1 { 200 } else { 300 };
        let keys = distinct_keys(&mut rng, keysize, n.min(if keysize == 1 { 200 } else { n }));
        for (i, k) in keys.iter().enumerate() {
            let mut kk = k.clone();
            d.put(
                kk.as_mut_ptr() as *mut c_void,
                i as u64,
                &format!("row34 e={elemsize} k={keysize} #{i}"),
            );
        }
        d.free();
    }
}

#[test]
fn row35_put_duplicates_binary() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 35);
    let shape = MapShape::bytes(16, 8);
    let mut d = Dual::implicit(c, r, shape, HM_BINARY, 37, false);
    let keys = distinct_keys(&mut rng, 8, 60);
    for (i, k) in keys.iter().enumerate() {
        let mut kk = k.clone();
        d.put(kk.as_mut_ptr() as *mut c_void, i as u64, &format!("row35 p{i}"));
    }
    let before = unsafe { hmlen(d.pc, 16) };
    for round in 0..4 {
        for (i, k) in keys.iter().enumerate() {
            let mut kk = k.clone();
            d.put(
                kk.as_mut_ptr() as *mut c_void,
                (1000 * round + i) as u64,
                &format!("row35 dup r{round} #{i}"),
            );
        }
        assert_eq!(
            unsafe { hmlen(d.pc, 16) },
            before,
            "re-putting existing keys must not grow the map"
        );
    }
    d.free();
}

#[test]
fn row36_engineered_bucket_collisions() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 36);
    let shape = MapShape::bytes(16, 8);
    for target in 0..8usize {
        let mut d = Dual::implicit(c, r, shape, HM_BINARY, 41, false);
        // bootstrap the table so we can read its seed
        let mut boot = rng.bytes(8);
        d.put(boot.as_mut_ptr() as *mut c_void, 0, "row36 bootstrap");
        let seed = unsafe {
            let t = arr_table((d.pc as *mut u8).sub(16) as *mut c_void);
            std::ptr::read_unaligned(t.add(hi::SEED) as *const usize)
        };
        // find keys whose probe position within an 8-slot table equals `target`
        let mut found = 0;
        let mut tries = 0;
        while found < 40 && tries < 400_000 {
            tries += 1;
            let mut k = rng.bytes(8);
            let mut h = unsafe { (c.hash_bytes)(k.as_mut_ptr() as *mut c_void, 8, seed) };
            let hr = unsafe { (r.hash_bytes)(k.as_mut_ptr() as *mut c_void, 8, seed) };
            assert_eq!(h, hr);
            if h < 2 {
                h += 2;
            }
            if h & 7 == target {
                d.put(
                    k.as_mut_ptr() as *mut c_void,
                    (found + 1) as u64,
                    &format!("row36 target={target} #{found}"),
                );
                found += 1;
            }
        }
        assert!(found > 0, "no colliding key found for target {target}");
        d.free();
    }
}

#[test]
fn row44_rehash_boundaries() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 44);
    let shape = MapShape::bytes(16, 8);
    let mut d = Dual::implicit(c, r, shape, HM_BINARY, 43, false);
    let keys = distinct_keys(&mut rng, 8, 400);
    for (i, k) in keys.iter().enumerate() {
        let mut kk = k.clone();
        d.put(kk.as_mut_ptr() as *mut c_void, i as u64, &format!("row44 #{i}"));
        unsafe {
            let t = arr_table((d.pc as *mut u8).sub(16) as *mut c_void);
            let sc = std::ptr::read_unaligned(t.add(hi::SLOT_COUNT) as *const usize);
            let t2 = arr_table((d.pr as *mut u8).sub(16) as *mut c_void);
            let sr = std::ptr::read_unaligned(t2.add(hi::SLOT_COUNT) as *const usize);
            assert_eq!(sc, sr, "slot_count diverged at insert {i}");
        }
    }
    d.free();
}

// ---------------------------------------------------------------------------
// rows 37..43, 59 — string modes / shmode_func cross product
// ---------------------------------------------------------------------------

#[test]
fn row37_put_string_implicit_default_mode() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 37);
    let shape = MapShape::strp(16);
    let mut d = Dual::implicit(c, r, shape, HM_STRING, 47, true);
    let keys = distinct_strings(&mut rng, 300);
    for (i, s) in keys.iter().enumerate() {
        d.put(s.as_ptr() as *mut c_void, i as u64, &format!("row37 #{i}"));
    }
    // string.mode must have become STBDS_SH_DEFAULT on both sides
    unsafe {
        for (name, p) in [("C", d.pc), ("R", d.pr)] {
            let t = arr_table((p as *mut u8).sub(16) as *mut c_void);
            let m = *t.add(hi::STRING + 17);
            assert_eq!(m as c_int, SH_DEFAULT, "{name} string.mode");
        }
    }
    d.free();
}

#[test]
fn row38_put_string_duplicates_temp_key() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 38);
    for shmode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let shape = MapShape::strp(24);
        let mut d = Dual::shmode(c, r, shape, HM_STRING, shmode, 53, true);
        let keys = distinct_strings(&mut rng, 80);
        for (i, s) in keys.iter().enumerate() {
            d.put(
                s.as_ptr() as *mut c_void,
                i as u64,
                &format!("row38 sh={shmode} put #{i}"),
            );
        }
        let before = unsafe { hmlen(d.pc, 24) };
        for round in 0..4 {
            for (i, s) in keys.iter().enumerate() {
                d.put(
                    s.as_ptr() as *mut c_void,
                    (500 * round + i) as u64,
                    &format!("row38 sh={shmode} dup r{round} #{i}"),
                );
            }
            assert_eq!(unsafe { hmlen(d.pc, 24) }, before);
        }
        d.free();
    }
}

#[test]
fn row39_row40_row42_string_modes() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 39);
    for shmode in [SH_STRDUP, SH_ARENA, SH_DEFAULT] {
        for elemsize in [8usize, 16, 24, 40] {
            let shape = MapShape::strp(elemsize);
            let mut d = Dual::shmode(c, r, shape, HM_STRING, shmode, 59, true);
            let keys = distinct_strings(&mut rng, 300);
            for (i, s) in keys.iter().enumerate() {
                d.put(
                    s.as_ptr() as *mut c_void,
                    i as u64,
                    &format!("row39 sh={shmode} e={elemsize} #{i}"),
                );
            }
            for (i, s) in keys.iter().enumerate() {
                let t = d.get(
                    s.as_ptr() as *mut c_void,
                    &format!("row39 sh={shmode} get #{i}"),
                );
                assert!(t >= 0);
            }
            d.free();
        }
    }
}

#[test]
fn row41_sh_none_with_string_mode() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 41);
    // `string.mode == STBDS_SH_NONE` with `mode == STBDS_HM_STRING`: the C
    // `switch` falls into `default:` and `memcpy`s `keysize` bytes *of the
    // string contents* into the element.  The stored bytes are therefore NOT a
    // valid pointer, so the element must be dumped as raw bytes and the map may
    // never be probed with a colliding key.
    let shape = MapShape {
        elemsize: 16,
        keyoffset: 0,
        keysize: 8,
        kind: KeyKind::Bytes,
    };
    let mut d = Dual::shmode(c, r, shape, HM_STRING, SH_NONE, 61, false);
    let keys = distinct_strings(&mut rng, 40);
    for (i, s) in keys.iter().enumerate() {
        // pad so that memcpy of 8 bytes stays inside the string
        let padded = CString::new(format!("{}________", s.to_str().unwrap())).unwrap();
        d.put(
            padded.as_ptr() as *mut c_void,
            i as u64,
            &format!("row41 #{i}"),
        );
    }
    d.free();
}

#[test]
fn row43_sh_none_binary() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 43);
    for keysize in [4usize, 8] {
        let shape = MapShape::bytes(16, keysize);
        let mut d = Dual::shmode(c, r, shape, HM_BINARY, SH_NONE, 67, false);
        let keys = distinct_keys(&mut rng, keysize, 200);
        for (i, k) in keys.iter().enumerate() {
            let mut kk = k.clone();
            d.put(
                kk.as_mut_ptr() as *mut c_void,
                i as u64,
                &format!("row43 k={keysize} #{i}"),
            );
        }
        for (i, k) in keys.iter().enumerate() {
            let mut kk = k.clone();
            assert!(d.get(kk.as_mut_ptr() as *mut c_void, &format!("row43 get #{i}")) >= 0);
        }
        d.free();
    }
}

#[test]
fn row59_shmode_func_cross_product() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 59);
    for elemsize in [8usize, 16, 24, 40] {
        for shmode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            // creation only: compares the freshly built index verbatim
            let shape = MapShape::bytes(elemsize, 8);
            let d = Dual::shmode(c, r, shape, HM_BINARY, shmode, 71, false);
            d.free();

            // binary puts on a string-mode arena: the C stores a pointer /
            // strdup / arena copy while comparing with memcmp.  Deterministic
            // and must match.
            let kind = if shmode == SH_NONE {
                KeyKind::Bytes
            } else {
                KeyKind::CStrPtr
            };
            let shape2 = MapShape {
                elemsize,
                keyoffset: 0,
                keysize: 8,
                kind,
            };
            let mut d2 = Dual::shmode(c, r, shape2, HM_BINARY, shmode, 73, false);
            let keys = distinct_strings(&mut rng, 24);
            for (i, s) in keys.iter().enumerate() {
                d2.put(
                    s.as_ptr() as *mut c_void,
                    i as u64,
                    &format!("row59 e={elemsize} sh={shmode} #{i}"),
                );
            }
            d2.free();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 45..54 — stbds_hmdel_key
// ---------------------------------------------------------------------------

#[test]
fn row45_row46_del_binary() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 45);
    for elemsize in [8usize, 16, 40] {
        let shape = MapShape::bytes(elemsize, 8);
        let mut d = Dual::implicit(c, r, shape, HM_BINARY, 79, false);
        let keys = distinct_keys(&mut rng, 8, 60);
        for (i, k) in keys.iter().enumerate() {
            let mut kk = k.clone();
            d.put(kk.as_mut_ptr() as *mut c_void, i as u64, &format!("row45 p{i}"));
        }
        // delete the middle element (forces the move-down + slot re-find path)
        let mut mid = keys[keys.len() / 2].clone();
        let t = d.del(mid.as_mut_ptr() as *mut c_void, "row45 del middle");
        assert_eq!(t, 1, "hmdel_key must report 1 for a found key");
        // delete the *last* element (old_index == final_index, no move)
        let last_idx = unsafe { hmlen(d.pc, elemsize) } as usize;
        let mut lastkey = vec![0u8; 8];
        unsafe {
            let e = (d.pc as *mut u8).add(elemsize * (last_idx - 1));
            std::ptr::copy_nonoverlapping(e, lastkey.as_mut_ptr(), 8);
        }
        let t2 = d.del(lastkey.as_mut_ptr() as *mut c_void, "row45 del last");
        assert_eq!(t2, 1);
        d.free();
    }
}

#[test]
fn row47_delete_all_random_order() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 47);
    let shape = MapShape::bytes(16, 8);
    let mut d = Dual::implicit(c, r, shape, HM_BINARY, 83, false);
    let mut keys = distinct_keys(&mut rng, 8, 300);
    for (i, k) in keys.iter().enumerate() {
        let mut kk = k.clone();
        d.put(kk.as_mut_ptr() as *mut c_void, i as u64, &format!("row47 p{i}"));
    }
    // Fisher-Yates with the deterministic RNG
    for i in (1..keys.len()).rev() {
        let j = rng.below(i + 1);
        keys.swap(i, j);
    }
    for (i, k) in keys.iter().enumerate() {
        let mut kk = k.clone();
        let t = d.del(kk.as_mut_ptr() as *mut c_void, &format!("row47 d{i}"));
        assert_eq!(t, 1, "delete #{i} should find its key");
    }
    assert_eq!(unsafe { hmlen(d.pc, 16) }, 0);
    d.free();
}

#[test]
fn row48_interleaved_ops() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 48);
    let shape = MapShape::bytes(16, 8);
    let mut d = Dual::implicit(c, r, shape, HM_BINARY, 89, false);
    let pool = distinct_keys(&mut rng, 8, 200);
    let mut live: Vec<usize> = Vec::new();
    for step in 0..2000usize {
        let op = rng.below(10);
        let idx = rng.below(pool.len());
        let mut k = pool[idx].clone();
        let p = k.as_mut_ptr() as *mut c_void;
        match op {
            0..=4 => {
                d.put(p, step as u64, &format!("row48 s{step} put"));
                if !live.contains(&idx) {
                    live.push(idx);
                }
            }
            5..=7 => {
                d.del(p, &format!("row48 s{step} del"));
                live.retain(|&x| x != idx);
            }
            8 => {
                d.get(p, &format!("row48 s{step} get"));
            }
            _ => {
                d.get_ts(p, &format!("row48 s{step} get_ts"));
            }
        }
        assert_eq!(
            unsafe { hmlen(d.pc, 16) } as usize,
            live.len(),
            "step {step}: length bookkeeping"
        );
    }
    d.free();
}

#[test]
fn row49_row50_row51_del_string_modes() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 49);
    for shmode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let shape = MapShape::strp(16);
        let mut d = Dual::shmode(c, r, shape, HM_STRING, shmode, 97, true);
        let mut keys = distinct_strings(&mut rng, 200);
        for (i, s) in keys.iter().enumerate() {
            d.put(
                s.as_ptr() as *mut c_void,
                i as u64,
                &format!("row49 sh={shmode} p{i}"),
            );
        }
        for i in (1..keys.len()).rev() {
            let j = rng.below(i + 1);
            keys.swap(i, j);
        }
        for (i, s) in keys.iter().enumerate() {
            let t = d.del(
                s.as_ptr() as *mut c_void,
                &format!("row49 sh={shmode} d{i}"),
            );
            assert_eq!(t, 1);
        }
        d.free();
    }
}

#[test]
fn row52_shrink_path() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 52);
    let shape = MapShape::bytes(16, 8);
    let mut d = Dual::implicit(c, r, shape, HM_BINARY, 101, false);
    let keys = distinct_keys(&mut rng, 8, 250);
    for (i, k) in keys.iter().enumerate() {
        let mut kk = k.clone();
        d.put(kk.as_mut_ptr() as *mut c_void, i as u64, &format!("row52 p{i}"));
    }
    let mut shrinks = 0;
    let mut prev_sc = unsafe {
        let t = arr_table((d.pc as *mut u8).sub(16) as *mut c_void);
        std::ptr::read_unaligned(t.add(hi::SLOT_COUNT) as *const usize)
    };
    for (i, k) in keys.iter().enumerate() {
        let mut kk = k.clone();
        d.del(kk.as_mut_ptr() as *mut c_void, &format!("row52 d{i}"));
        let sc = unsafe {
            let t = arr_table((d.pc as *mut u8).sub(16) as *mut c_void);
            std::ptr::read_unaligned(t.add(hi::SLOT_COUNT) as *const usize)
        };
        if sc < prev_sc {
            shrinks += 1;
        }
        prev_sc = sc;
    }
    assert!(shrinks > 0, "the shrink path was never exercised");
    d.free();
}

#[test]
fn row53_tombstone_rebuild_path() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 53);
    let shape = MapShape::bytes(16, 8);
    let mut d = Dual::implicit(c, r, shape, HM_BINARY, 103, false);
    // Keep used_count high (no shrink) while accumulating tombstones by
    // deleting and re-inserting different keys.
    let base = distinct_keys(&mut rng, 8, 200);
    for (i, k) in base.iter().enumerate() {
        let mut kk = k.clone();
        d.put(kk.as_mut_ptr() as *mut c_void, i as u64, &format!("row53 p{i}"));
    }
    let extra = distinct_keys(&mut rng, 8, 400);
    for step in 0..400usize {
        let mut del = base[step % base.len()].clone();
        d.del(del.as_mut_ptr() as *mut c_void, &format!("row53 d{step}"));
        let mut ins = extra[step].clone();
        d.put(
            ins.as_mut_ptr() as *mut c_void,
            (10_000 + step) as u64,
            &format!("row53 r{step}"),
        );
    }
    d.free();
}

#[test]
fn row54_nonzero_keyoffset() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 54);
    // The raw ABI allows keyoffset != 0 in hmdel_key (hmput_key/hmget_key
    // hard-code 0).  With `mode == STBDS_HM_STRING` a non-zero keyoffset makes
    // the C `strcmp` a *pointer* out of caller value bytes, which faults in the
    // C just as much as in Rust — so the non-zero offset is only exercised in
    // binary mode, where `memcmp` stays inside the element.
    let cases: Vec<(usize, c_int, c_int)> = vec![
        (0, HM_BINARY, SH_NONE),
        (0, HM_STRING, SH_STRDUP),
        (0, HM_STRING, SH_ARENA),
        (8, HM_BINARY, SH_NONE),
        (16, HM_BINARY, SH_NONE),
    ];
    for (keyoffset, mode, shmode) in cases {
        let kind = if mode == HM_STRING {
            KeyKind::CStrPtr
        } else {
            KeyKind::Bytes
        };
        let shape = MapShape {
            elemsize: 24,
            keyoffset: 0,
            keysize: 8,
            kind,
        };
        let mut d = Dual::shmode(c, r, shape, mode, shmode, 107, false);
        let strings = distinct_strings(&mut rng, 40);
        let mut bkeys = distinct_keys(&mut rng, 8, 40);
        for i in 0..40 {
            if mode == HM_STRING {
                d.put(
                    strings[i].as_ptr() as *mut c_void,
                    i as u64,
                    &format!("row54 ko={keyoffset} put {i}"),
                );
            } else {
                d.put(
                    bkeys[i].as_mut_ptr() as *mut c_void,
                    i as u64,
                    &format!("row54 ko={keyoffset} put {i}"),
                );
            }
        }
        for i in 0..20 {
            let kp: *mut c_void = if mode == HM_STRING {
                strings[i].as_ptr() as *mut c_void
            } else {
                bkeys[i].as_mut_ptr() as *mut c_void
            };
            unsafe {
                let nc = (c.hmdel_key)(d.pc, 24, kp, 8, keyoffset, mode);
                let nr = (r.hmdel_key)(d.pr, 24, kp, 8, keyoffset, mode);
                assert_eq!(nc.is_null(), nr.is_null());
                d.pc = nc;
                d.pr = nr;
                let tc = arr_temp((d.pc as *mut u8).sub(24) as *mut c_void);
                let tr = arr_temp((d.pr as *mut u8).sub(24) as *mut c_void);
                assert_eq!(tc, tr, "row54 ko={keyoffset} del {i} temp");
                if keyoffset == 0 {
                    assert_eq!(tc, 1, "row54 keyoffset=0 delete must find the key");
                } else {
                    assert_eq!(tc, 0, "row54 keyoffset!=0 delete must miss");
                }
            }
            d.check(&format!("row54 ko={keyoffset} mode={mode} after del {i}"));
        }
        d.free();
    }
}

// ---------------------------------------------------------------------------
// rows 55..58 — stbds_hmfree_func
// ---------------------------------------------------------------------------

#[test]
fn row55_row56_row57_hmfree_modes() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 55);
    for shmode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for n in [0usize, 1, 7, 8, 40, 200] {
            let kind = if shmode == SH_NONE {
                KeyKind::Bytes
            } else {
                KeyKind::CStrPtr
            };
            let shape = MapShape {
                elemsize: 16,
                keyoffset: 0,
                keysize: 8,
                kind,
            };
            let mode = if shmode == SH_NONE { HM_BINARY } else { HM_STRING };
            let mut d = Dual::shmode(c, r, shape, mode, shmode, 109, false);
            if shmode == SH_NONE {
                let keys = distinct_keys(&mut rng, 8, n);
                for (i, k) in keys.iter().enumerate() {
                    let mut kk = k.clone();
                    d.put(
                        kk.as_mut_ptr() as *mut c_void,
                        i as u64,
                        &format!("row55 sh={shmode} n={n} #{i}"),
                    );
                }
            } else {
                let keys = distinct_strings(&mut rng, n);
                for (i, s) in keys.iter().enumerate() {
                    d.put(
                        s.as_ptr() as *mut c_void,
                        i as u64,
                        &format!("row55 sh={shmode} n={n} #{i}"),
                    );
                }
            }
            d.free(); // must not double-free / leak-crash on either side
        }
    }
}

#[test]
fn row58_hmfree_no_table() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 58);
    for elemsize in [8usize, 16, 40] {
        let shape = MapShape::bytes(elemsize, 8);
        let mut d = Dual::implicit(c, r, shape, HM_BINARY, 113, false);
        let mut k = rng.bytes(8);
        d.get_ts(k.as_mut_ptr() as *mut c_void, "row58 bootstrap");
        assert!(unsafe { arr_table((d.pc as *mut u8).sub(elemsize) as *mut c_void) }.is_null());
        d.free();
    }
    // and hmput_default-created maps, which also have no index
    for elemsize in [8usize, 16] {
        let shape = MapShape::bytes(elemsize, 8);
        let mut d = Dual::implicit(c, r, shape, HM_BINARY, 127, false);
        d.put_default("row58 default");
        d.free();
    }
}

// ---------------------------------------------------------------------------
// rows 63..65 — composed pipelines
// ---------------------------------------------------------------------------

#[test]
fn row63_row64_pipelines() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 63);
    let configs: Vec<(c_int, c_int, KeyKind)> = vec![
        (SH_STRDUP, HM_STRING, KeyKind::CStrPtr),
        (SH_ARENA, HM_STRING, KeyKind::CStrPtr),
        (SH_DEFAULT, HM_STRING, KeyKind::CStrPtr),
        (SH_NONE, HM_BINARY, KeyKind::Bytes),
    ];
    for (shmode, mode, kind) in configs {
        for elemsize in [16usize, 24, 40] {
            let shape = MapShape {
                elemsize,
                keyoffset: 0,
                keysize: 8,
                kind,
            };
            let mut d = Dual::shmode(c, r, shape, mode, shmode, 131, mode == HM_STRING);
            let strings = distinct_strings(&mut rng, 200);
            let mut bkeys = distinct_keys(&mut rng, 8, 200);
            let keyptr = |i: usize, s: &Vec<CString>, b: &mut Vec<Vec<u8>>| -> *mut c_void {
                if mode == HM_STRING {
                    s[i].as_ptr() as *mut c_void
                } else {
                    b[i].as_mut_ptr() as *mut c_void
                }
            };
            for i in 0..200 {
                let p = keyptr(i, &strings, &mut bkeys);
                d.put(p, i as u64, &format!("row63 sh={shmode} e={elemsize} put {i}"));
            }
            for i in 0..100 {
                let p = keyptr(i, &strings, &mut bkeys);
                d.get(p, &format!("row63 sh={shmode} get {i}"));
            }
            for i in 0..100 {
                let p = keyptr(i, &strings, &mut bkeys);
                d.del(p, &format!("row63 sh={shmode} del {i}"));
            }
            for i in 0..100 {
                let p = keyptr(i, &strings, &mut bkeys);
                d.put(
                    p,
                    (10_000 + i) as u64,
                    &format!("row63 sh={shmode} re-put {i}"),
                );
            }
            d.free();
        }
    }
}

#[test]
fn row65_mixed_with_reseed() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 65);
    for round in 0..20usize {
        let seed = rng.next_u64() as usize;
        let shape = MapShape::bytes(24, 8);
        let mut d = Dual::implicit(c, r, shape, HM_BINARY, seed, false);
        d.put_default(&format!("row65 r{round} default"));
        let keys = distinct_keys(&mut rng, 8, 60);
        for (i, k) in keys.iter().enumerate() {
            let mut kk = k.clone();
            d.put(
                kk.as_mut_ptr() as *mut c_void,
                i as u64,
                &format!("row65 r{round} put {i}"),
            );
            if i % 3 == 0 {
                d.get(kk.as_mut_ptr() as *mut c_void, &format!("row65 r{round} get {i}"));
            }
            if i % 5 == 4 {
                let mut old = keys[i / 2].clone();
                d.del(
                    old.as_mut_ptr() as *mut c_void,
                    &format!("row65 r{round} del {i}"),
                );
            }
        }
        d.put_default(&format!("row65 r{round} default 2"));
        d.free();
    }
}
