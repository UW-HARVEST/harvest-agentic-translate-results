//! Low-level map driver shared by the Phase B / Phase C differential tests.
//!
//! It re-implements the `stbds_hmput` / `stbds_shput` / `stbds_hmgeti` /
//! `stbds_hmdel` *macros* from `c_src/src/lib.c` on top of the exported
//! `stbds_hm*_key` functions, which is exactly how a real consumer drives the
//! library.

#![allow(dead_code)]

use super::*;
use std::ffi::{c_char, c_int, c_void};

/// One library's map, driven at the lowest level.
pub struct Map<'a> {
    pub lib: &'a Lib,
    pub elemsize: usize,
    pub keysize: usize,
    /// `h` is the *hash* pointer (`t` in the macros); NULL before the first op.
    pub h: *mut c_void,
}

impl<'a> Map<'a> {
    pub fn new(lib: &'a Lib, elemsize: usize, keysize: usize) -> Map<'a> {
        Map { lib, elemsize, keysize, h: std::ptr::null_mut() }
    }

    /// Start from a table pre-created by `stbds_shmode_func`.
    pub fn with_shmode(lib: &'a Lib, elemsize: usize, keysize: usize, sh_mode: c_int) -> Map<'a> {
        let h = unsafe { (lib.shmode_func)(elemsize, sh_mode) };
        Map { lib, elemsize, keysize, h }
    }

    /// `stbds_header(t-1)->temp`. `stbds_hmdel_key(NULL, ...)` returns NULL, so
    /// a map can legitimately become NULL again mid-stream (the macros guard
    /// with `(t) ? stbds_temp((t)-1) : 0`).
    pub fn temp(&self) -> isize {
        if self.h.is_null() {
            0
        } else {
            unsafe { header(hash_to_arr(self.h, self.elemsize)).temp }
        }
    }

    pub fn len(&self) -> isize {
        // stbds_hmlen(t) = header(t-1)->length - 1
        if self.h.is_null() {
            0
        } else {
            unsafe { header(hash_to_arr(self.h, self.elemsize)).length as isize - 1 }
        }
    }

    /// Raw element pointer for element index `i` (index 0 is the default slot).
    pub fn elem(&self, i: usize) -> *mut u8 {
        (hash_to_arr(self.h, self.elemsize) as *mut u8).wrapping_add(self.elemsize * i)
    }

    /// `stbds_hmput(t,k,v)`: call `hmput_key`, then write the whole element
    /// (key **and** value) at the returned index, like the macro does.
    pub fn put_binary(&mut self, key: &mut [u8], value: &[u8], mode: c_int) -> isize {
        assert_eq!(key.len(), self.keysize);
        assert_eq!(self.keysize + value.len(), self.elemsize);
        self.h = unsafe {
            (self.lib.hmput_key)(
                self.h,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                mode,
            )
        };
        let t = self.temp();
        unsafe {
            // t[temp].key = k;  t[temp].value = v;
            let e = (self.h as *mut u8).wrapping_add(self.elemsize * (t as usize));
            std::ptr::copy_nonoverlapping(key.as_ptr(), e, self.keysize);
            std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(self.keysize), value.len());
        }
        t
    }

    /// `stbds_shput(t,k,v)`: `hmput_key` stores the key itself (per
    /// `table->string.mode`); the macro only writes the value.
    pub fn put_string(&mut self, key: *mut c_char, value: &[u8], mode: c_int) -> isize {
        assert_eq!(self.keysize + value.len(), self.elemsize);
        self.h = unsafe {
            (self.lib.hmput_key)(self.h, self.elemsize, key as *mut c_void, self.keysize, mode)
        };
        let t = self.temp();
        unsafe {
            let e = (self.h as *mut u8).wrapping_add(self.elemsize * (t as usize));
            std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(self.keysize), value.len());
        }
        t
    }

    /// `stbds_hmgeti(t,k)` — returns the element index or -1.
    pub fn geti(&mut self, key: *mut c_void, mode: c_int) -> isize {
        self.h =
            unsafe { (self.lib.hmget_key)(self.h, self.elemsize, key, self.keysize, mode) };
        self.temp()
    }

    /// `stbds_hmgeti_ts(t,k,temp)`
    pub fn geti_ts(&mut self, key: *mut c_void, mode: c_int) -> isize {
        let mut t: isize = 0x5a5a_5a5a;
        self.h = unsafe {
            (self.lib.hmget_key_ts)(
                self.h,
                self.elemsize,
                key,
                self.keysize,
                &mut t,
                mode,
            )
        };
        t
    }

    /// `stbds_hmdel(t,k)` — returns 1 if something was deleted, 0 otherwise.
    pub fn del(&mut self, key: *mut c_void, keyoffset: usize, mode: c_int) -> isize {
        self.h = unsafe {
            (self.lib.hmdel_key)(
                self.h,
                self.elemsize,
                key,
                self.keysize,
                keyoffset,
                mode,
            )
        };
        if self.h.is_null() { 0 } else { self.temp() }
    }

    pub fn free(&mut self) {
        if !self.h.is_null() {
            unsafe { (self.lib.hmfree_func)(hash_to_arr(self.h, self.elemsize), self.elemsize) };
            self.h = std::ptr::null_mut();
        }
    }

    // --- comparable snapshots --------------------------------------------

    pub fn hdr(&self) -> Option<HdrSnapshot> {
        if self.h.is_null() {
            None
        } else {
            Some(unsafe { hash_hdr_snapshot(self.h, self.elemsize) })
        }
    }

    pub fn table(&self) -> Option<TableSnapshot> {
        if self.h.is_null() {
            None
        } else {
            unsafe { table_snapshot(self.h, self.elemsize) }
        }
    }

    /// Raw element bytes — only comparable for *binary* maps (string maps store
    /// pointers, which differ between the two libraries).
    pub fn raw_elems(&self) -> Vec<u8> {
        if self.h.is_null() {
            Vec::new()
        } else {
            unsafe { hash_elems(self.h, self.elemsize) }
        }
    }

    /// For string maps: (key contents, value bytes) per element. Element 0 is
    /// the zeroed default slot, whose key pointer is NULL.
    pub fn string_elems(&self) -> Vec<(Option<Vec<u8>>, Vec<u8>)> {
        if self.h.is_null() {
            return Vec::new();
        }
        let n = unsafe { header(hash_to_arr(self.h, self.elemsize)).length };
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            unsafe {
                let e = self.elem(i);
                let kp = std::ptr::read_unaligned(e as *const *mut c_char);
                let k = if kp.is_null() { None } else { Some(cstr(kp)) };
                let v =
                    std::slice::from_raw_parts(e.add(self.keysize), self.elemsize - self.keysize)
                        .to_vec();
                out.push((k, v));
            }
        }
        out
    }

    /// `stbds_temp_key(t-1)` contents (or None if the pointer is NULL).
    pub fn temp_key(&self) -> Option<Vec<u8>> {
        if self.h.is_null() {
            return None;
        }
        unsafe {
            let hdr = header(hash_to_arr(self.h, self.elemsize));
            if hdr.hash_table.is_null() {
                return None;
            }
            let tk = std::ptr::read_unaligned(hdr.hash_table as *const *mut c_char);
            if tk.is_null() { None } else { Some(cstr(tk)) }
        }
    }
}

/// Compare two maps' full observable state (binary keys).
pub fn assert_same_binary(ctx: &str, c: &Map, r: &Map) {
    dq(format!("{ctx} / null-ness"), c.h.is_null(), r.h.is_null());
    dq(format!("{ctx} / header"), c.hdr(), r.hdr());
    dq(format!("{ctx} / elems"), c.raw_elems(), r.raw_elems());
    dq(format!("{ctx} / table"), c.table(), r.table());
}

/// Compare two maps' full observable state (string keys: compare contents).
pub fn assert_same_string(ctx: &str, c: &Map, r: &Map) {
    dq(format!("{ctx} / null-ness"), c.h.is_null(), r.h.is_null());
    dq(format!("{ctx} / header"), c.hdr(), r.hdr());
    dq(
        format!("{ctx} / string elems"),
        c.string_elems(),
        r.string_elems()
    );
    // The table's arena `storage` pointer differs; TableSnapshot already
    // reduces it to a bool.
    dq(format!("{ctx} / table"), c.table(), r.table());
    // NOTE: `stbds_hash_index::temp_key` is deliberately NOT compared here.
    // `stbds_make_hash_index` never initialises that field, so after any table
    // grow / shrink / rebuild it holds uninitialised malloc bytes in *both*
    // libraries and reading it is UB. It is only a meaningful observable right
    // after an insert in string mode (see `cfg_18b_temp_key`).
}

/// Leak a NUL-terminated key so `STBDS_SH_DEFAULT` tables (which store the
/// caller's pointer) stay valid for the lifetime of the test.
pub fn leak_cstring(bytes: &[u8]) -> *mut c_char {
    let mut v = bytes.to_vec();
    if v.last() != Some(&0) {
        v.push(0);
    }
    Box::leak(v.into_boxed_slice()).as_mut_ptr() as *mut c_char
}
