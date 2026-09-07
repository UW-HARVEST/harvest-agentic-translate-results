//! Faithful re-implementations of the `stbds_hm*` / `stbds_sh*` macros from
//! `c_src/src/lib.c`, driving BOTH `.so`s in lockstep through their exports.

#![allow(dead_code)]

use super::*;
use std::ffi::{c_char, c_int, c_void};

/// A pair of maps (one per `.so`) driven identically.
pub struct Driver<'a> {
    pub s: &'a Session<'a>,
    pub elemsize: usize,
    pub keysize: usize,
    pub kind: KeyKind,
    pub cm: *mut c_void,
    pub rm: *mut c_void,
    /// Keeps `SH_DEFAULT`-mode key buffers alive for the driver's lifetime.
    pub keep: Vec<CStrBuf>,
}

impl<'a> Driver<'a> {
    /// `t = NULL` — the map is bootstrapped lazily by the first `hmput_key`.
    pub fn lazy(s: &'a Session<'a>, elemsize: usize, keysize: usize, kind: KeyKind) -> Driver<'a> {
        Driver {
            s,
            elemsize,
            keysize,
            kind,
            cm: std::ptr::null_mut(),
            rm: std::ptr::null_mut(),
            keep: Vec::new(),
        }
    }

    /// `sh_new_arena(t)` / `sh_new_strdup(t)` / any explicit `shmode_func` mode.
    pub fn shmode(
        s: &'a Session<'a>,
        elemsize: usize,
        keysize: usize,
        kind: KeyKind,
        mode: c_int,
    ) -> Driver<'a> {
        unsafe {
            let cm = (s.c.shmode_func)(elemsize, mode);
            let rm = (s.r.shmode_func)(elemsize, mode);
            Driver {
                s,
                elemsize,
                keysize,
                kind,
                cm,
                rm,
                keep: Vec::new(),
            }
        }
    }

    pub fn c_raw(&self) -> *mut c_void {
        raw_of(self.cm, self.elemsize)
    }
    pub fn r_raw(&self) -> *mut c_void {
        raw_of(self.rm, self.elemsize)
    }

    pub fn snap_c(&self) -> MapSnap {
        unsafe { map_snap(self.cm, self.elemsize, self.kind) }
    }
    pub fn snap_r(&self) -> MapSnap {
        unsafe { map_snap(self.rm, self.elemsize, self.kind) }
    }
    pub fn check(&self, ctx: &str) {
        assert_snap_eq(ctx, &self.snap_c(), &self.snap_r());
    }

    /// `stbds_hmlen(t)` = `header(t-1)->length - 1`
    pub fn len_c(&self) -> isize {
        if self.cm.is_null() {
            0
        } else {
            unsafe { header(self.c_raw()).length as isize - 1 }
        }
    }

    unsafe fn write_elem(&self, idx: isize, off: usize, bytes: &[u8]) {
        unsafe {
            let ce = (self.cm as *mut u8).offset(idx * self.elemsize as isize).add(off);
            let re = (self.rm as *mut u8).offset(idx * self.elemsize as isize).add(off);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ce, bytes.len());
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), re, bytes.len());
        }
    }

    // -----------------------------------------------------------------
    // BINARY: `stbds_hmput(t,k,v)`
    //   t = hmput_key(t, sizeof*t, &k, sizeof t->key, 0);
    //   t[temp(t-1)].key = k; t[temp(t-1)].value = v;
    // -----------------------------------------------------------------
    pub fn hmput(&mut self, ctx: &str, key: &[u8], value: &[u8], mode: c_int) -> isize {
        assert_eq!(key.len(), self.keysize);
        unsafe {
            let kp = key.as_ptr() as *mut c_void;
            self.cm = (self.s.c.hmput_key)(self.cm, self.elemsize, kp, self.keysize, mode);
            self.rm = (self.s.r.hmput_key)(self.rm, self.elemsize, kp, self.keysize, mode);
            let ci = header(self.c_raw()).temp;
            let ri = header(self.r_raw()).temp;
            assert_eq!(ci, ri, "{ctx}: hmput_key temp index differs");
            self.write_elem(ci, 0, key);
            if self.elemsize > self.keysize {
                let vlen = (self.elemsize - self.keysize).min(value.len());
                self.write_elem(ci, self.keysize, &value[..vlen]);
            }
            ci
        }
    }

    /// `stbds_hmgeti(t,k)` — `hmget_key` + read `temp`.
    pub fn hmgeti(&mut self, ctx: &str, key: &[u8], mode: c_int) -> isize {
        unsafe {
            let kp = key.as_ptr() as *mut c_void;
            self.cm = (self.s.c.hmget_key)(self.cm, self.elemsize, kp, self.keysize, mode);
            self.rm = (self.s.r.hmget_key)(self.rm, self.elemsize, kp, self.keysize, mode);
            let ci = header(self.c_raw()).temp;
            let ri = header(self.r_raw()).temp;
            assert_eq!(ci, ri, "{ctx}: hmgeti index differs");
            ci
        }
    }

    /// `stbds_hmgeti_ts(t,k,temp)` — the low-level thread-safe form.
    pub fn hmgeti_ts(&mut self, ctx: &str, key: &[u8], mode: c_int) -> isize {
        unsafe {
            let kp = key.as_ptr() as *mut c_void;
            let mut ct: isize = 0x5A5A_5A5A;
            let mut rt: isize = 0x5A5A_5A5A;
            self.cm =
                (self.s.c.hmget_key_ts)(self.cm, self.elemsize, kp, self.keysize, &mut ct, mode);
            self.rm =
                (self.s.r.hmget_key_ts)(self.rm, self.elemsize, kp, self.keysize, &mut rt, mode);
            assert_eq!(ct, rt, "{ctx}: hmgeti_ts *temp differs");
            ct
        }
    }

    /// `stbds_hmdel(t,k)` — returns `t ? temp(t-1) : 0`.
    pub fn hmdel(&mut self, ctx: &str, key: &[u8], keyoffset: usize, mode: c_int) -> isize {
        unsafe {
            let kp = key.as_ptr() as *mut c_void;
            self.cm =
                (self.s.c.hmdel_key)(self.cm, self.elemsize, kp, self.keysize, keyoffset, mode);
            self.rm =
                (self.s.r.hmdel_key)(self.rm, self.elemsize, kp, self.keysize, keyoffset, mode);
            assert_eq!(
                self.cm.is_null(),
                self.rm.is_null(),
                "{ctx}: hmdel_key null-ness differs"
            );
            if self.cm.is_null() {
                return 0;
            }
            let ci = header(self.c_raw()).temp;
            let ri = header(self.r_raw()).temp;
            assert_eq!(ci, ri, "{ctx}: hmdel_key temp differs");
            ci
        }
    }

    // -----------------------------------------------------------------
    // STRING: `stbds_shput(t,k,v)`
    //   t = hmput_key(t, sizeof*t, (void*)k, sizeof t->key, STBDS_HM_STRING);
    //   t[temp(t-1)].value = v;
    // -----------------------------------------------------------------
    pub fn shput(&mut self, ctx: &str, key: &[u8], value: &[u8], mode: c_int) -> isize {
        let cs = CStrBuf::new(key);
        let kp = cs.ptr() as *mut c_void;
        self.keep.push(cs);
        unsafe {
            self.cm = (self.s.c.hmput_key)(self.cm, self.elemsize, kp, 8, mode);
            self.rm = (self.s.r.hmput_key)(self.rm, self.elemsize, kp, 8, mode);
            let ci = header(self.c_raw()).temp;
            let ri = header(self.r_raw()).temp;
            assert_eq!(ci, ri, "{ctx}: shput temp index differs");
            if self.elemsize > 8 {
                let vlen = (self.elemsize - 8).min(value.len());
                self.write_elem(ci, 8, &value[..vlen]);
            }
            ci
        }
    }

    /// `stbds_shputs(t,s)` — additionally re-reads the key back out of
    /// `temp_key(t-1)` (i.e. `*(char**)header(t-1)->hash_table`).
    pub fn shputs(&mut self, ctx: &str, key: &[u8], value: &[u8], mode: c_int) -> isize {
        let cs = CStrBuf::new(key);
        let kp = cs.ptr() as *mut c_void;
        self.keep.push(cs);
        unsafe {
            self.cm = (self.s.c.hmput_key)(self.cm, self.elemsize, kp, 8, mode);
            self.rm = (self.s.r.hmput_key)(self.rm, self.elemsize, kp, 8, mode);
            let ci = header(self.c_raw()).temp;
            let ri = header(self.r_raw()).temp;
            assert_eq!(ci, ri, "{ctx}: shputs temp index differs");
            // t[i] = s   (the whole struct, key field included)
            if self.elemsize > 8 {
                let vlen = (self.elemsize - 8).min(value.len());
                self.write_elem(ci, 8, &value[..vlen]);
            }
            self.write_elem(ci, 0, &(kp as usize).to_ne_bytes());
            // t[i].key = temp_key(t-1)
            let ctk = *(header(self.c_raw()).hash_table as *mut *mut c_char);
            let rtk = *(header(self.r_raw()).hash_table as *mut *mut c_char);
            let ck = read_cstr(ctk);
            let rk = read_cstr(rtk);
            assert_eq!(ck, rk, "{ctx}: temp_key string differs");
            let ce = (self.cm as *mut u8).offset(ci * self.elemsize as isize);
            let re = (self.rm as *mut u8).offset(ci * self.elemsize as isize);
            *(ce as *mut *mut c_char) = ctk;
            *(re as *mut *mut c_char) = rtk;
            ci
        }
    }

    /// `stbds_shgeti(t,k)`
    pub fn shgeti(&mut self, ctx: &str, key: &[u8], mode: c_int) -> isize {
        let cs = CStrBuf::new(key);
        unsafe {
            let kp = cs.ptr() as *mut c_void;
            self.cm = (self.s.c.hmget_key)(self.cm, self.elemsize, kp, 8, mode);
            self.rm = (self.s.r.hmget_key)(self.rm, self.elemsize, kp, 8, mode);
            let ci = header(self.c_raw()).temp;
            let ri = header(self.r_raw()).temp;
            assert_eq!(ci, ri, "{ctx}: shgeti index differs");
            ci
        }
    }

    /// `stbds_shgeti` via the `_ts` low-level entry point.
    pub fn shgeti_ts(&mut self, ctx: &str, key: &[u8], mode: c_int) -> isize {
        let cs = CStrBuf::new(key);
        unsafe {
            let kp = cs.ptr() as *mut c_void;
            let mut ct: isize = 0x5A5A_5A5A;
            let mut rt: isize = 0x5A5A_5A5A;
            self.cm = (self.s.c.hmget_key_ts)(self.cm, self.elemsize, kp, 8, &mut ct, mode);
            self.rm = (self.s.r.hmget_key_ts)(self.rm, self.elemsize, kp, 8, &mut rt, mode);
            assert_eq!(ct, rt, "{ctx}: shgeti_ts *temp differs");
            ct
        }
    }

    /// `stbds_shdel(t,k)`
    pub fn shdel(&mut self, ctx: &str, key: &[u8], keyoffset: usize, mode: c_int) -> isize {
        let cs = CStrBuf::new(key);
        unsafe {
            let kp = cs.ptr() as *mut c_void;
            self.cm = (self.s.c.hmdel_key)(self.cm, self.elemsize, kp, 8, keyoffset, mode);
            self.rm = (self.s.r.hmdel_key)(self.rm, self.elemsize, kp, 8, keyoffset, mode);
            assert_eq!(
                self.cm.is_null(),
                self.rm.is_null(),
                "{ctx}: shdel null-ness differs"
            );
            if self.cm.is_null() {
                return 0;
            }
            let ci = header(self.c_raw()).temp;
            let ri = header(self.r_raw()).temp;
            assert_eq!(ci, ri, "{ctx}: shdel temp differs");
            ci
        }
    }

    /// `stbds_hmdefault` / `stbds_shdefault`: `t = hmput_default(t, sizeof*t)`
    /// then write `t[-1].value`.
    pub fn hmdefault(&mut self, ctx: &str, value: &[u8]) {
        unsafe {
            self.cm = (self.s.c.hmput_default)(self.cm, self.elemsize);
            self.rm = (self.s.r.hmput_default)(self.rm, self.elemsize);
            assert_eq!(
                self.cm.is_null(),
                self.rm.is_null(),
                "{ctx}: hmput_default null-ness differs"
            );
            if !self.cm.is_null() && self.elemsize > 8 {
                let vlen = (self.elemsize - 8).min(value.len());
                self.write_elem(-1, 8, &value[..vlen]);
            }
        }
    }

    /// `stbds_hmfree(p)` / `stbds_shfree(p)`
    pub fn free(&mut self) {
        unsafe {
            if !self.cm.is_null() {
                (self.s.c.hmfree_func)(self.c_raw(), self.elemsize);
            }
            if !self.rm.is_null() {
                (self.s.r.hmfree_func)(self.r_raw(), self.elemsize);
            }
            self.cm = std::ptr::null_mut();
            self.rm = std::ptr::null_mut();
        }
    }
}
