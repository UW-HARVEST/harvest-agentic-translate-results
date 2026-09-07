//! Included from the test files: a paired driver that applies the *same*
//! operation to the C and the Rust `.so` and compares the resulting state.

use super::*;
use std::ffi::CStr;

pub struct Pair<'a> {
    pub c: &'a Lib,
    pub r: &'a Lib,
    pub ct: *mut c_void,
    pub rt: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub mode: c_int,
    pub kind: KeyKind,
    pub label: String,
}

impl<'a> Pair<'a> {
    /// Both maps start as NULL (created lazily by the first `hmput_key`).
    pub fn empty(
        c: &'a Lib,
        r: &'a Lib,
        elemsize: usize,
        keysize: usize,
        mode: c_int,
        kind: KeyKind,
        label: &str,
    ) -> Pair<'a> {
        Pair {
            c,
            r,
            ct: std::ptr::null_mut(),
            rt: std::ptr::null_mut(),
            elemsize,
            keysize,
            mode,
            kind,
            label: label.to_string(),
        }
    }

    /// Both maps created via `stbds_shmode_func(elemsize, sh_mode)`.
    pub unsafe fn shmode(
        c: &'a Lib,
        r: &'a Lib,
        elemsize: usize,
        keysize: usize,
        mode: c_int,
        sh_mode: c_int,
        kind: KeyKind,
        label: &str,
    ) -> Pair<'a> {
        let ct = (c.shmode_func)(elemsize, sh_mode);
        let rt = (r.shmode_func)(elemsize, sh_mode);
        Pair {
            c,
            r,
            ct,
            rt,
            elemsize,
            keysize,
            mode,
            kind,
            label: label.to_string(),
        }
    }

    pub unsafe fn temp(&self, t: *mut c_void) -> isize {
        map_header(t, self.elemsize).temp
    }

    unsafe fn elem(&self, t: *mut c_void, i: isize) -> *mut u8 {
        (t as *mut u8).offset(i * self.elemsize as isize)
    }

    pub unsafe fn snap(&self, t: *mut c_void) -> String {
        snapshot_map(t, self.elemsize, self.keysize, self.kind)
    }

    #[track_caller]
    pub unsafe fn check(&self, step: &str) {
        assert_same(
            &format!("{} / {}", self.label, step),
            &self.snap(self.ct),
            &self.snap(self.rt),
        );
    }

    /// `hmput(t, key, val)` for a binary (inline) key.
    #[track_caller]
    pub unsafe fn put_bin(&mut self, key: &[u8], val: &[u8]) {
        assert_eq!(key.len(), self.keysize);
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        self.ct = (self.c.hmput_key)(
            self.ct,
            self.elemsize,
            kc.as_mut_ptr() as *mut c_void,
            self.keysize,
            self.mode,
        );
        self.rt = (self.r.hmput_key)(
            self.rt,
            self.elemsize,
            kr.as_mut_ptr() as *mut c_void,
            self.keysize,
            self.mode,
        );
        let tc = self.temp(self.ct);
        let tr = self.temp(self.rt);
        assert_eq!(tc, tr, "{}: put temp mismatch", self.label);
        // emulate the macro's `t[temp].key = k; t[temp].value = v;`
        for (t, base) in [(self.ct, self.elem(self.ct, tc)), (self.rt, self.elem(self.rt, tr))] {
            let _ = t;
            std::ptr::copy_nonoverlapping(key.as_ptr(), base, self.keysize);
            if !val.is_empty() {
                std::ptr::copy_nonoverlapping(val.as_ptr(), base.add(self.keysize), val.len());
            }
        }
    }

    /// `shput(t, key, val)` — the library owns/records the key.
    #[track_caller]
    pub unsafe fn put_str(&mut self, key: &CStr, val: &[u8]) {
        let kp = key.as_ptr() as *mut c_void;
        self.ct = (self.c.hmput_key)(self.ct, self.elemsize, kp, self.keysize, self.mode);
        self.rt = (self.r.hmput_key)(self.rt, self.elemsize, kp, self.keysize, self.mode);
        let tc = self.temp(self.ct);
        let tr = self.temp(self.rt);
        assert_eq!(tc, tr, "{}: shput temp mismatch", self.label);
        if !val.is_empty() {
            for base in [self.elem(self.ct, tc), self.elem(self.rt, tr)] {
                std::ptr::copy_nonoverlapping(val.as_ptr(), base.add(8), val.len());
            }
        }
    }

    /// `hmgeti(t, key)` for a binary key — returns the shared index.
    #[track_caller]
    pub unsafe fn get_bin(&mut self, key: &[u8]) -> isize {
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        self.ct = (self.c.hmget_key)(
            self.ct,
            self.elemsize,
            kc.as_mut_ptr() as *mut c_void,
            self.keysize,
            self.mode,
        );
        self.rt = (self.r.hmget_key)(
            self.rt,
            self.elemsize,
            kr.as_mut_ptr() as *mut c_void,
            self.keysize,
            self.mode,
        );
        let tc = self.temp(self.ct);
        let tr = self.temp(self.rt);
        assert_eq!(tc, tr, "{}: get temp mismatch (key {:?})", self.label, key);
        tc
    }

    #[track_caller]
    pub unsafe fn get_bin_ts(&mut self, key: &[u8]) -> isize {
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        let mut tc: isize = 0x5555;
        let mut tr: isize = 0x5555;
        self.ct = (self.c.hmget_key_ts)(
            self.ct,
            self.elemsize,
            kc.as_mut_ptr() as *mut c_void,
            self.keysize,
            &mut tc,
            self.mode,
        );
        self.rt = (self.r.hmget_key_ts)(
            self.rt,
            self.elemsize,
            kr.as_mut_ptr() as *mut c_void,
            self.keysize,
            &mut tr,
            self.mode,
        );
        assert_eq!(tc, tr, "{}: get_ts temp mismatch", self.label);
        tc
    }

    #[track_caller]
    pub unsafe fn get_str(&mut self, key: &CStr) -> isize {
        let kp = key.as_ptr() as *mut c_void;
        self.ct = (self.c.hmget_key)(self.ct, self.elemsize, kp, self.keysize, self.mode);
        self.rt = (self.r.hmget_key)(self.rt, self.elemsize, kp, self.keysize, self.mode);
        let tc = self.temp(self.ct);
        let tr = self.temp(self.rt);
        assert_eq!(tc, tr, "{}: shget temp mismatch", self.label);
        tc
    }

    /// Reads the value bytes (`elemsize - keysize` of them) at index `i`.
    pub unsafe fn value_at(&self, t: *mut c_void, i: isize, off: usize) -> Vec<u8> {
        let base = self.elem(t, i).add(off);
        (off..self.elemsize).map(|k| *base.add(k - off)).collect()
    }

    #[track_caller]
    pub unsafe fn check_value(&self, i: isize, off: usize) {
        if i < 0 {
            return;
        }
        let a = self.value_at(self.ct, i, off);
        let b = self.value_at(self.rt, i, off);
        assert_eq!(a, b, "{}: value bytes differ at {}", self.label, i);
    }

    #[track_caller]
    pub unsafe fn del_bin(&mut self, key: &[u8], keyoffset: usize) -> isize {
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        self.ct = (self.c.hmdel_key)(
            self.ct,
            self.elemsize,
            kc.as_mut_ptr() as *mut c_void,
            self.keysize,
            keyoffset,
            self.mode,
        );
        self.rt = (self.r.hmdel_key)(
            self.rt,
            self.elemsize,
            kr.as_mut_ptr() as *mut c_void,
            self.keysize,
            keyoffset,
            self.mode,
        );
        let tc = if self.ct.is_null() { 0 } else { self.temp(self.ct) };
        let tr = if self.rt.is_null() { 0 } else { self.temp(self.rt) };
        assert_eq!(
            self.ct.is_null(),
            self.rt.is_null(),
            "{}: hmdel NULL-ness differs",
            self.label
        );
        assert_eq!(tc, tr, "{}: del temp mismatch", self.label);
        tc
    }

    #[track_caller]
    pub unsafe fn del_str(&mut self, key: &CStr) -> isize {
        let kp = key.as_ptr() as *mut c_void;
        self.ct = (self.c.hmdel_key)(self.ct, self.elemsize, kp, self.keysize, 0, self.mode);
        self.rt = (self.r.hmdel_key)(self.rt, self.elemsize, kp, self.keysize, 0, self.mode);
        let tc = if self.ct.is_null() { 0 } else { self.temp(self.ct) };
        let tr = if self.rt.is_null() { 0 } else { self.temp(self.rt) };
        assert_eq!(tc, tr, "{}: shdel temp mismatch", self.label);
        tc
    }

    /// `stbds_temp_key(t-1)` — the scratch key pointer the `shputs` macro reads.
    ///
    /// Only meaningful once at least one insert has happened *and* no table
    /// growth happened during a "key already present" lookup (the C leaves
    /// `temp_key` uninitialised in a freshly grown table if the key is found by
    /// the wrap-around half of the probe loop).
    #[track_caller]
    pub unsafe fn check_temp_key(&self, expect: &CStr, step: &str) {
        let kc = (*(map_header(self.ct, self.elemsize).hash_table as *mut HashIndex)).temp_key;
        let kr = (*(map_header(self.rt, self.elemsize).hash_table as *mut HashIndex)).temp_key;
        assert_eq!(cstr(kc), cstr(kr), "{}/{}: temp_key differs", self.label, step);
        assert_eq!(
            cstr(kc),
            cstr(expect.as_ptr()),
            "{}/{}: temp_key content",
            self.label,
            step
        );
    }

    pub unsafe fn put_default(&mut self) {
        self.ct = (self.c.hmput_default)(self.ct, self.elemsize);
        self.rt = (self.r.hmput_default)(self.rt, self.elemsize);
    }

    pub unsafe fn free(&mut self) {
        if !self.ct.is_null() {
            (self.c.hmfree_func)((self.ct as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
        }
        if !self.rt.is_null() {
            (self.r.hmfree_func)((self.rt as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
        }
        self.ct = std::ptr::null_mut();
        self.rt = std::ptr::null_mut();
    }
}

/// Resets both libraries' global hash seed so their `make_hash_index`
/// seed sequences start from the same point.
pub unsafe fn seed_both(c: &Lib, r: &Lib, s: usize) {
    (c.rand_seed)(s);
    (r.rand_seed)(s);
}
