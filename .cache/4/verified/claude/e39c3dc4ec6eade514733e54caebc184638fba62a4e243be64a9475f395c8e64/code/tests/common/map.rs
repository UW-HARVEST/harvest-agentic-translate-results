//! A faithful re-implementation of the `stbds_hmput` / `stbds_hmget` /
//! `stbds_hmdel` / `stbds_hmdefault` *macros* on top of the raw exported
//! low-level functions.
//!
//! This is what drives the library "the way a real consumer does": the macros
//! are the only thing a C user writes, and they are what composes
//! `hmput_key` + `stbds_temp` + the element store into a pipeline.

#![allow(dead_code)]

use super::*;
use std::ffi::c_void;

#[derive(Clone, Copy, Debug)]
pub struct MapCfg {
    pub elemsize: usize,
    pub keysize: usize,
    /// Only `stbds_hmdel_key` takes a `keyoffset`; `hmput_key`/`hmget_key`
    /// hard-code 0. Keep them consistent unless deliberately testing the
    /// asymmetry.
    pub keyoffset: usize,
    pub mode: i32,
    /// Where this test's synthetic "value" lives inside an element.
    /// INVARIANT for snapshotting: the union of the key bytes and
    /// `valoffset..valoffset+valsize` must cover the whole element, otherwise
    /// the snapshot would compare uninitialised padding.
    pub valoffset: usize,
    pub valsize: usize,
    /// Force `KeyRepr::Raw` snapshots even in string mode. Needed for
    /// `STBDS_SH_NONE` string tables, where `hmput_key` `memcpy`s the *string
    /// bytes* into the element instead of storing a pointer, so the "key field"
    /// must not be dereferenced.
    pub force_raw_snap: bool,
}

impl MapCfg {
    /// `struct { int key; int value; }`
    pub fn int_int() -> Self {
        MapCfg {
            elemsize: 8,
            keysize: 4,
            keyoffset: 0,
            mode: STBDS_HM_BINARY,
            valoffset: 4,
            valsize: 4,
            force_raw_snap: false,
        }
    }
    /// `struct { char *key; long value; }`
    pub fn str_long() -> Self {
        MapCfg {
            elemsize: 16,
            keysize: 8,
            keyoffset: 0,
            mode: STBDS_HM_STRING,
            valoffset: 8,
            valsize: 8,
            force_raw_snap: false,
        }
    }
    pub fn key_repr(&self) -> KeyRepr {
        if self.mode >= STBDS_HM_STRING && !self.force_raw_snap {
            KeyRepr::StrPtr {
                keyoffset: self.keyoffset,
            }
        } else {
            KeyRepr::Raw
        }
    }
}

pub struct Map<'a> {
    pub lib: &'a Lib,
    pub cfg: MapCfg,
    /// The `t` pointer a C user holds (`raw_array + elemsize`), or NULL.
    pub t: *mut c_void,
}

impl<'a> Map<'a> {
    /// A map a C user would declare as `T *t = NULL;`
    pub fn empty(lib: &'a Lib, cfg: MapCfg) -> Self {
        Map {
            lib,
            cfg,
            t: std::ptr::null_mut(),
        }
    }

    /// `stbds_sh_new_arena(t)` / `stbds_sh_new_strdup(t)` /
    /// `stbds_shmode_func(elemsize, mode)` in general.
    pub fn with_shmode(lib: &'a Lib, cfg: MapCfg, sh_mode: i32) -> Self {
        let t = unsafe { (lib.shmode_func)(cfg.elemsize, sh_mode) };
        Map { lib, cfg, t }
    }

    #[inline]
    unsafe fn hdr(&self) -> *mut u8 {
        (self.t as *mut u8).sub(self.cfg.elemsize).sub(HDR_SIZE)
    }

    /// `stbds_temp((t)-1)`
    pub unsafe fn temp(&self) -> isize {
        rd_isize(self.hdr(), HDR_TEMP)
    }

    /// `stbds_temp_key((t)-1)` == `*(char**)header->hash_table`, i.e. the
    /// `stbds_hash_index::temp_key` field, as a string.
    ///
    /// Only meaningful right after a string-mode `hmput_key`; it is
    /// uninitialised heap memory otherwise (see `snap_map`).
    pub unsafe fn temp_key(&self) -> Option<Vec<u8>> {
        let tbl = rd_ptr(self.hdr(), HDR_HASH_TABLE);
        if tbl.is_null() {
            return None;
        }
        let tk = rd_ptr(tbl, HI_TEMP_KEY);
        if tk.is_null() {
            None
        } else {
            Some(cstr_bytes(tk))
        }
    }

    /// The raw `stbds_hash_index::temp_key` pointer value.
    ///
    /// Comparable between the two implementations only in `STBDS_SH_DEFAULT`
    /// mode, where it is always a caller-owned pointer.
    pub unsafe fn temp_key_raw(&self) -> Option<*mut u8> {
        let tbl = rd_ptr(self.hdr(), HDR_HASH_TABLE);
        if tbl.is_null() {
            None
        } else {
            Some(rd_ptr(tbl, HI_TEMP_KEY))
        }
    }

    /// The element's stored key pointer (`*(char**)(t[i] + keyoffset)`).
    pub unsafe fn elem_key_ptr(&self, i: isize) -> *mut u8 {
        rd_ptr(self.elem(i), self.cfg.keyoffset)
    }

    /// Does `temp_key` point at exactly the same address as the element's key
    /// pointer?  (True for every mode that writes `temp_key`.)
    pub unsafe fn temp_key_is_elem_key(&self, i: isize) -> Option<bool> {
        let tbl = rd_ptr(self.hdr(), HDR_HASH_TABLE);
        if tbl.is_null() {
            return None;
        }
        let tk = rd_ptr(tbl, HI_TEMP_KEY);
        let ek = rd_ptr(self.elem(i), self.cfg.keyoffset);
        Some(tk == ek)
    }

    /// `stbds_arrlen` of the underlying raw array.
    pub unsafe fn raw_len(&self) -> usize {
        if self.t.is_null() {
            0
        } else {
            rd_usize(self.hdr(), HDR_LENGTH)
        }
    }

    /// `stbds_hmlen(t)`
    pub unsafe fn hmlen(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            rd_usize(self.hdr(), HDR_LENGTH) as isize - 1
        }
    }

    /// Address of `t[i]`.
    pub unsafe fn elem(&self, i: isize) -> *mut u8 {
        (self.t as *mut u8).offset(i * self.cfg.elemsize as isize)
    }

    /// `stbds_hmput(t, k, v)` (binary) / `stbds_shput(t, k, v)` (string).
    ///
    /// `key` points at `keysize` bytes for binary mode, or at a NUL-terminated
    /// string for string mode.  Returns the resulting `stbds_temp` index.
    pub unsafe fn put(&mut self, key: *mut c_void, val: &[u8]) -> isize {
        let c = self.cfg;
        self.t = (self.lib.hmput_key)(self.t, c.elemsize, key, c.keysize, c.mode);
        let idx = self.temp();
        let e = self.elem(idx);
        if c.mode < STBDS_HM_STRING {
            // the macro's `(t)[temp].key = (k)` store
            std::ptr::copy_nonoverlapping(key as *const u8, e, c.keysize);
        }
        if !val.is_empty() {
            std::ptr::copy_nonoverlapping(val.as_ptr(), e.add(c.valoffset), val.len());
        }
        idx
    }

    /// `stbds_hmdefault(t, v)` — writes into `t[-1]`.
    pub unsafe fn set_default(&mut self, val: &[u8]) {
        let c = self.cfg;
        self.t = (self.lib.hmput_default)(self.t, c.elemsize);
        let e = self.elem(-1);
        if !val.is_empty() {
            std::ptr::copy_nonoverlapping(val.as_ptr(), e.add(c.valoffset), val.len());
        }
    }

    /// `stbds_hmgeti(t, k)` / `stbds_shgeti(t, k)`
    pub unsafe fn geti(&mut self, key: *mut c_void) -> isize {
        let c = self.cfg;
        self.t = (self.lib.hmget_key)(self.t, c.elemsize, key, c.keysize, c.mode);
        self.temp()
    }

    /// `stbds_hmgeti_ts(t, k, temp)` — returns `(temp_out, header_temp)` so a
    /// test can check that the `_ts` variant does *not* touch `header->temp`.
    pub unsafe fn geti_ts(&mut self, key: *mut c_void) -> (isize, isize) {
        let c = self.cfg;
        let mut temp: isize = 0x5a5a_5a5a;
        self.t = (self.lib.hmget_key_ts)(self.t, c.elemsize, key, c.keysize, &mut temp, c.mode);
        (temp, self.temp())
    }

    /// `stbds_hmdel(t, k)` / `stbds_shdel(t, k)`
    pub unsafe fn del(&mut self, key: *mut c_void) -> isize {
        let c = self.cfg;
        self.t = (self.lib.hmdel_key)(self.t, c.elemsize, key, c.keysize, c.keyoffset, c.mode);
        if self.t.is_null() {
            0
        } else {
            self.temp()
        }
    }

    /// `stbds_hmfree(t)`
    pub unsafe fn free(&mut self) {
        if !self.t.is_null() {
            (self.lib.hmfree_func)(
                (self.t as *mut u8).sub(self.cfg.elemsize) as *mut c_void,
                self.cfg.elemsize,
            );
            self.t = std::ptr::null_mut();
        }
    }

    pub unsafe fn snap(&self) -> Snap {
        snap_map(self.t, self.cfg.elemsize, self.cfg.key_repr())
    }

    /// Value bytes stored at `t[i]`.
    pub unsafe fn val_at(&self, i: isize) -> Vec<u8> {
        let c = self.cfg;
        std::slice::from_raw_parts(self.elem(i).add(c.valoffset), c.valsize).to_vec()
    }
}

/// Run the same op sequence against both implementations, comparing snapshots
/// after every single operation.
pub struct MapPair<'a> {
    pub c: Map<'a>,
    pub rs: Map<'a>,
}

impl<'a> MapPair<'a> {
    pub fn empty(p: &'a Pair, cfg: MapCfg) -> Self {
        MapPair {
            c: Map::empty(&p.c, cfg),
            rs: Map::empty(&p.rs, cfg),
        }
    }
    pub fn with_shmode(p: &'a Pair, cfg: MapCfg, sh_mode: i32) -> Self {
        MapPair {
            c: Map::with_shmode(&p.c, cfg, sh_mode),
            rs: Map::with_shmode(&p.rs, cfg, sh_mode),
        }
    }

    #[track_caller]
    pub unsafe fn check(&self, ctx: &str) {
        assert_snap_eq(&self.c.snap(), &self.rs.snap(), ctx);
    }

    /// The C side's snapshot bytes, for comparing two *different configurations*
    /// against each other (e.g. "every negative `mode` behaves like mode 0").
    pub unsafe fn snap_c(&self) -> Vec<u8> {
        self.c.snap().0
    }

    #[track_caller]
    pub unsafe fn put(&mut self, key: *mut c_void, val: &[u8], ctx: &str) -> isize {
        let a = self.c.put(key, val);
        let b = self.rs.put(key, val);
        assert_eq_ctx(a, b, &format!("{ctx}: put temp"));
        self.check(&format!("{ctx}: after put"));
        a
    }

    #[track_caller]
    pub unsafe fn set_default(&mut self, val: &[u8], ctx: &str) {
        self.c.set_default(val);
        self.rs.set_default(val);
        self.check(&format!("{ctx}: after hmdefault"));
    }

    #[track_caller]
    pub unsafe fn geti(&mut self, key: *mut c_void, ctx: &str) -> isize {
        let a = self.c.geti(key);
        let b = self.rs.geti(key);
        assert_eq_ctx(a, b, &format!("{ctx}: geti"));
        self.check(&format!("{ctx}: after geti"));
        a
    }

    #[track_caller]
    pub unsafe fn geti_ts(&mut self, key: *mut c_void, ctx: &str) -> isize {
        let a = self.c.geti_ts(key);
        let b = self.rs.geti_ts(key);
        assert_eq_ctx(a, b, &format!("{ctx}: geti_ts (temp, header_temp)"));
        self.check(&format!("{ctx}: after geti_ts"));
        a.0
    }

    #[track_caller]
    pub unsafe fn del(&mut self, key: *mut c_void, ctx: &str) -> isize {
        let a = self.c.del(key);
        let b = self.rs.del(key);
        assert_eq_ctx(a, b, &format!("{ctx}: del"));
        self.check(&format!("{ctx}: after del"));
        a
    }

    /// Compare `stbds_hash_index::temp_key` between the two implementations.
    /// Call only where the C is guaranteed to have written it.
    #[track_caller]
    pub unsafe fn check_temp_key(&self, i: isize, ctx: &str) {
        assert_eq_ctx(
            self.c.temp_key(),
            self.rs.temp_key(),
            &format!("{ctx}: temp_key string"),
        );
        assert_eq_ctx(
            self.c.temp_key_is_elem_key(i),
            self.rs.temp_key_is_elem_key(i),
            &format!("{ctx}: temp_key aliases the element key pointer"),
        );
    }

    #[track_caller]
    pub unsafe fn check_val(&self, i: isize, ctx: &str) {
        assert_eq_ctx(self.c.val_at(i), self.rs.val_at(i), &format!("{ctx}: value"));
    }

    #[track_caller]
    pub unsafe fn hmlen(&self, ctx: &str) -> isize {
        let a = self.c.hmlen();
        let b = self.rs.hmlen();
        assert_eq_ctx(a, b, &format!("{ctx}: hmlen"));
        a
    }

    pub unsafe fn free(&mut self) {
        self.c.free();
        self.rs.free();
    }
}
