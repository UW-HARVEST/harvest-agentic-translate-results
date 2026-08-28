//! A driver that keeps a C-backed and a Rust-backed stb_ds hash map in lockstep
//! and compares their full structural state after every operation.
#![allow(dead_code)]

use super::snap::{self, KeyKind, MapSnap};
use super::{Pair, HEADER_SIZE, STBDS_HM_BINARY, STBDS_HM_STRING};
use std::ffi::{c_int, c_void};

pub unsafe fn temp_of(t: *mut c_void, elemsize: usize) -> isize {
    unsafe {
        let raw = (t as *mut u8).sub(elemsize);
        (raw.sub(HEADER_SIZE).add(24) as *const isize).read_unaligned()
    }
}

pub struct MapPair<'a> {
    pub p: &'a Pair,
    pub elemsize: usize,
    /// value passed as `keysize` to the stbds entry points
    pub keysize: usize,
    /// bytes at the start of an element that the library itself fills in
    pub lib_written: usize,
    pub kind: KeyKind,
    pub mode: c_int,
    pub read_temp_key: bool,
    /// `stbds_temp_key` stops being comparable once a STRDUP-mode delete has
    /// freed a key that a later "key already present" hit may leave it pointing
    /// at (the C code only refreshes it in the first of the two probe loops).
    pub check_temp_key: bool,
    pub shmode: c_int,
    pub ct: *mut c_void,
    pub rt: *mut c_void,
}

impl<'a> MapPair<'a> {
    /// Empty binary-keyed map (`hmput`-style usage starting from `NULL`).
    pub fn binary(p: &'a Pair, elemsize: usize, keysize: usize) -> MapPair<'a> {
        MapPair {
            p,
            elemsize,
            keysize,
            lib_written: keysize,
            kind: KeyKind::Binary,
            mode: STBDS_HM_BINARY,
            read_temp_key: false,
            check_temp_key: false,
            shmode: super::STBDS_SH_NONE,
            ct: std::ptr::null_mut(),
            rt: std::ptr::null_mut(),
        }
    }

    /// Empty string-keyed map (`shput` from `NULL` => `STBDS_SH_DEFAULT`).
    pub fn string(p: &'a Pair, elemsize: usize) -> MapPair<'a> {
        MapPair {
            p,
            elemsize,
            keysize: 8,
            lib_written: 8,
            kind: KeyKind::StrPtr,
            mode: STBDS_HM_STRING,
            read_temp_key: false,
            check_temp_key: true,
            shmode: super::STBDS_SH_DEFAULT,
            ct: std::ptr::null_mut(),
            rt: std::ptr::null_mut(),
        }
    }

    /// String-keyed map created through `stbds_shmode_func` (`sh_new_arena` /
    /// `sh_new_strdup`).
    pub fn string_mode(p: &'a Pair, elemsize: usize, shmode: c_int) -> MapPair<'a> {
        let mut m = MapPair::string(p, elemsize);
        m.shmode = shmode;
        unsafe {
            m.ct = (p.c.shmode_func)(elemsize, shmode);
            m.rt = (p.r.shmode_func)(elemsize, shmode);
        }
        m
    }

    pub fn snap_c(&self) -> MapSnap {
        unsafe { snap::snap_map_ex(self.ct, self.elemsize, self.kind, self.read_temp_key) }
    }

    pub fn snap_r(&self) -> MapSnap {
        unsafe { snap::snap_map_ex(self.rt, self.elemsize, self.kind, self.read_temp_key) }
    }

    /// Compare the whole structure; panics with `ctx` on any difference.
    pub fn check(&self, ctx: &str) {
        let c = self.snap_c();
        let r = self.snap_r();
        assert_eq!(self.ct.is_null(), self.rt.is_null(), "{ctx}: null-ness differs");
        if c != r {
            assert_eq!(c.header, r.header, "{ctx}: array header differs");
            assert_eq!(c.temp_key, r.temp_key, "{ctx}: temp_key differs");
            match (&c.index, &r.index) {
                (Some(ci), Some(ri)) => {
                    assert_eq!(ci.slot_count, ri.slot_count, "{ctx}: slot_count");
                    assert_eq!(ci.used_count, ri.used_count, "{ctx}: used_count");
                    assert_eq!(
                        ci.tombstone_count, ri.tombstone_count,
                        "{ctx}: tombstone_count"
                    );
                    assert_eq!(ci.seed, ri.seed, "{ctx}: seed");
                    assert_eq!(ci.arena, ri.arena, "{ctx}: string arena");
                    assert_eq!(ci.hashes, ri.hashes, "{ctx}: bucket hashes");
                    assert_eq!(ci.indices, ri.indices, "{ctx}: bucket indices");
                    assert_eq!(ci, ri, "{ctx}: hash index");
                }
                _ => assert_eq!(
                    c.index.is_some(),
                    r.index.is_some(),
                    "{ctx}: hash-table presence differs"
                ),
            }
            assert_eq!(c.elems, r.elems, "{ctx}: elements differ");
            assert_eq!(c, r, "{ctx}");
        }
    }

    /// Fill the trailing (caller-owned) part of element `idx` — this is what the
    /// `hmput` / `shput` macros do after the library call returns.
    unsafe fn write_payload(&self, t: *mut c_void, idx: isize, key: &[u8], payload: &[u8]) {
        unsafe {
            let e = (t as *mut u8).offset(idx * self.elemsize as isize);
            if self.kind == KeyKind::Binary {
                // `(t)[i].key = k` — identical to the memcpy the library did
                std::ptr::copy_nonoverlapping(key.as_ptr(), e, self.keysize);
            }
            let tail = self.elemsize - self.lib_written;
            for i in 0..tail {
                *e.add(self.lib_written + i) = payload[i % payload.len()];
            }
        }
    }

    /// `hmput` / `shput`.  `key` is the raw key bytes for binary maps, or the
    /// NUL-terminated key for string maps.
    pub fn put(&mut self, key: &mut [u8], payload: &[u8], ctx: &str) {
        unsafe {
            let kp = key.as_mut_ptr() as *mut c_void;
            self.ct = (self.p.c.hmput_key)(self.ct, self.elemsize, kp, self.keysize, self.mode);
            self.rt = (self.p.r.hmput_key)(self.rt, self.elemsize, kp, self.keysize, self.mode);
            let ci = temp_of(self.ct, self.elemsize);
            let ri = temp_of(self.rt, self.elemsize);
            assert_eq!(ci, ri, "{ctx}: hmput_key temp differs");
            if self.mode >= STBDS_HM_STRING && self.check_temp_key {
                // `stbds_temp_key` is only meaningful right after a put: it is
                // uninitialised before the first one and may dangle after a
                // delete in STRDUP mode.
                assert_eq!(
                    self.temp_key_c(),
                    self.temp_key_r(),
                    "{ctx}: stbds_temp_key differs"
                );
            }
            self.write_payload(self.ct, ci, key, payload);
            self.write_payload(self.rt, ri, key, payload);
        }
        self.check(ctx);
    }

    unsafe fn temp_key(t: *mut c_void, elemsize: usize) -> Option<Vec<u8>> {
        unsafe {
            let raw = (t as *mut u8).sub(elemsize);
            let ht = snap::read_ptr(raw.sub(HEADER_SIZE), 16);
            if ht.is_null() {
                None
            } else {
                snap::cstr(snap::read_ptr(ht, snap::HI_TEMP_KEY) as *const std::ffi::c_char)
            }
        }
    }

    pub fn temp_key_c(&self) -> Option<Vec<u8>> {
        unsafe { Self::temp_key(self.ct, self.elemsize) }
    }

    pub fn temp_key_r(&self) -> Option<Vec<u8>> {
        unsafe { Self::temp_key(self.rt, self.elemsize) }
    }

    /// `hmgeti` / `shgeti`: returns the slot index reported by both libraries.
    pub fn get(&mut self, key: &mut [u8], ctx: &str) -> isize {
        unsafe {
            let kp = key.as_mut_ptr() as *mut c_void;
            self.ct = (self.p.c.hmget_key)(self.ct, self.elemsize, kp, self.keysize, self.mode);
            self.rt = (self.p.r.hmget_key)(self.rt, self.elemsize, kp, self.keysize, self.mode);
            let ci = temp_of(self.ct, self.elemsize);
            let ri = temp_of(self.rt, self.elemsize);
            assert_eq!(ci, ri, "{ctx}: hmget_key temp differs");
            // `hmget` dereferences t[temp]; compare what a caller would read
            let ce = std::slice::from_raw_parts(
                (self.ct as *const u8).offset(ci * self.elemsize as isize),
                self.elemsize,
            );
            let re = std::slice::from_raw_parts(
                (self.rt as *const u8).offset(ri * self.elemsize as isize),
                self.elemsize,
            );
            if self.kind == KeyKind::Binary {
                assert_eq!(ce, re, "{ctx}: hmget_key element bytes differ");
            } else {
                assert_eq!(
                    &ce[8..],
                    &re[8..],
                    "{ctx}: hmget_key element payload differs"
                );
            }
            self.check(ctx);
            ci
        }
    }

    /// `hmget_key_ts` — same as `get` but through the explicit-temp entry point.
    pub fn get_ts(&mut self, key: &mut [u8], ctx: &str) -> isize {
        unsafe {
            let kp = key.as_mut_ptr() as *mut c_void;
            let mut ctemp: isize = 0x5a5a;
            let mut rtemp: isize = 0x5a5a;
            self.ct = (self.p.c.hmget_key_ts)(
                self.ct,
                self.elemsize,
                kp,
                self.keysize,
                &mut ctemp,
                self.mode,
            );
            self.rt = (self.p.r.hmget_key_ts)(
                self.rt,
                self.elemsize,
                kp,
                self.keysize,
                &mut rtemp,
                self.mode,
            );
            assert_eq!(ctemp, rtemp, "{ctx}: hmget_key_ts *temp differs");
            self.check(ctx);
            ctemp
        }
    }

    /// `hmdel` / `shdel`: returns the value the macro yields
    /// (`t ? stbds_temp(t-1) : 0`).
    pub fn del(&mut self, key: &mut [u8], keyoffset: usize, ctx: &str) -> isize {
        unsafe {
            let kp = key.as_mut_ptr() as *mut c_void;
            self.ct = (self.p.c.hmdel_key)(
                self.ct,
                self.elemsize,
                kp,
                self.keysize,
                keyoffset,
                self.mode,
            );
            self.rt = (self.p.r.hmdel_key)(
                self.rt,
                self.elemsize,
                kp,
                self.keysize,
                keyoffset,
                self.mode,
            );
            assert_eq!(self.ct.is_null(), self.rt.is_null(), "{ctx}: hmdel null-ness");
            let ci = if self.ct.is_null() {
                0
            } else {
                temp_of(self.ct, self.elemsize)
            };
            let ri = if self.rt.is_null() {
                0
            } else {
                temp_of(self.rt, self.elemsize)
            };
            assert_eq!(ci, ri, "{ctx}: hmdel_key temp differs");
            if self.shmode == super::STBDS_SH_STRDUP {
                self.check_temp_key = false;
            }
            self.check(ctx);
            ci
        }
    }

    /// `hmdefault` — installs the default element slot.
    pub fn put_default(&mut self, payload: &[u8], ctx: &str) {
        unsafe {
            self.ct = (self.p.c.hmput_default)(self.ct, self.elemsize);
            self.rt = (self.p.r.hmput_default)(self.rt, self.elemsize);
            // `(t)[-1].value = v`
            let ce = (self.ct as *mut u8).offset(-(self.elemsize as isize));
            let re = (self.rt as *mut u8).offset(-(self.elemsize as isize));
            for i in 0..self.elemsize {
                *ce.add(i) = payload[i % payload.len()];
                *re.add(i) = payload[i % payload.len()];
            }
        }
        self.check(ctx);
    }

    pub fn free(&mut self) {
        unsafe {
            if !self.ct.is_null() {
                (self.p.c.hmfree_func)(
                    (self.ct as *mut u8).sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
                self.ct = std::ptr::null_mut();
            }
            if !self.rt.is_null() {
                (self.p.r.hmfree_func)(
                    (self.rt as *mut u8).sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
                self.rt = std::ptr::null_mut();
            }
        }
    }
}

/// NUL-terminated key buffer sized to `keysize` for binary maps.
pub fn key_bytes(keysize: usize, seed: u64) -> Vec<u8> {
    let mut v = vec![0u8; keysize];
    let mut s = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0xdead_beef_cafe_babe;
    for b in v.iter_mut() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        *b = (s >> 33) as u8;
    }
    v
}

pub fn cstring(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}
