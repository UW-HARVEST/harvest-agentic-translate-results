//! Level 5: string-keyed maps -- `stbds_shmode_func` plus the
//! `STBDS_HM_STRING` paths of `hmput_key` / `hmget_key` / `hmdel_key` /
//! `hmfree_func` in all three `string.mode` flavours (DEFAULT, STRDUP, ARENA).

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void, CStr, CString};

#[repr(C)]
#[derive(Copy, Clone)]
struct Sv {
    key: *mut c_char,
    value: c_int,
}

const ES: usize = std::mem::size_of::<Sv>(); // 16 (contains padding -> no raw byte compare)
const KS: usize = std::mem::size_of::<*mut c_char>();

struct SMap<'a> {
    im: &'a Impl,
    t: *mut Sv,
}

impl<'a> SMap<'a> {
    fn new(im: &'a Impl) -> Self {
        SMap {
            im,
            t: std::ptr::null_mut(),
        }
    }

    unsafe fn temp(&self) -> isize {
        unsafe { header(self.t.offset(-1) as *mut c_void).temp }
    }

    /// `sh_new_strdup` / `sh_new_arena`
    unsafe fn new_mode(&mut self, mode: c_int) {
        unsafe { self.t = (self.im.shmode_func)(ES, mode) as *mut Sv }
    }

    /// `stbds_shput`
    unsafe fn shput(&mut self, k: *mut c_char, v: c_int, mode: c_int) {
        unsafe {
            self.t = (self.im.hmput_key)(self.t as *mut c_void, ES, k as *mut c_void, KS, mode)
                as *mut Sv;
            (*self.t.offset(self.temp())).value = v;
        }
    }

    /// `stbds_shgeti`
    unsafe fn shgeti(&mut self, k: *mut c_char, mode: c_int) -> isize {
        unsafe {
            self.t = (self.im.hmget_key)(self.t as *mut c_void, ES, k as *mut c_void, KS, mode)
                as *mut Sv;
            self.temp()
        }
    }

    unsafe fn shget(&mut self, k: *mut c_char, mode: c_int) -> c_int {
        unsafe {
            self.shgeti(k, mode);
            (*self.t.offset(self.temp())).value
        }
    }

    unsafe fn shgeti_ts(&mut self, k: *mut c_char, mode: c_int) -> isize {
        unsafe {
            let mut temp: isize = 0;
            self.t = (self.im.hmget_key_ts)(
                self.t as *mut c_void,
                ES,
                k as *mut c_void,
                KS,
                &mut temp,
                mode,
            ) as *mut Sv;
            temp
        }
    }

    /// `stbds_shdel`
    unsafe fn shdel(&mut self, k: *mut c_char, mode: c_int) -> isize {
        unsafe {
            self.t =
                (self.im.hmdel_key)(self.t as *mut c_void, ES, k as *mut c_void, KS, 0, mode)
                    as *mut Sv;
            if !self.t.is_null() { self.temp() } else { 0 }
        }
    }

    unsafe fn shfree(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.im.hmfree_func)(self.t.offset(-1) as *mut c_void, ES);
            }
            self.t = std::ptr::null_mut();
        }
    }

    /// Structural snapshot: header + hash index + every element rendered as
    /// (key string | NULL, value).  The raw key *pointers* are meaningless
    /// across implementations, and byte 12..16 of each element is padding.
    ///
    /// `tk_valid` says whether `temp_key` has been written yet; before the first
    /// string-mode insert it holds indeterminate bytes in both builds and must
    /// not be dereferenced.
    unsafe fn snap(&self, tk_valid: bool) -> Vec<u8> {
        unsafe {
            let mut out = Vec::new();
            if self.t.is_null() {
                out.push(0);
                return out;
            }
            out.push(1);
            let raw = self.t.offset(-1) as *mut c_void;
            snapshot_header(&mut out, raw);
            snapshot_hash_index(&mut out, raw);

            let h = header(raw);
            for i in 0..h.length {
                let e = (raw as *mut Sv).add(i);
                if (*e).key.is_null() {
                    out.push(0);
                } else {
                    out.push(1);
                    out.extend_from_slice(CStr::from_ptr((*e).key).to_bytes_with_nul());
                }
                out.extend_from_slice(&(*e).value.to_le_bytes());
            }

            if tk_valid && !h.hash_table.is_null() {
                let tk = (*(h.hash_table as *mut HashIndex)).temp_key;
                if tk.is_null() {
                    out.push(0);
                } else {
                    out.push(1);
                    out.extend_from_slice(CStr::from_ptr(tk).to_bytes_with_nul());
                }
            }
            out
        }
    }

    /// `string.mode` of the attached hash index, if any.
    unsafe fn string_mode(&self) -> Option<u8> {
        unsafe {
            if self.t.is_null() {
                return None;
            }
            let h = header(self.t.offset(-1) as *mut c_void);
            if h.hash_table.is_null() {
                None
            } else {
                Some((*(h.hash_table as *mut HashIndex)).string.mode)
            }
        }
    }
}

struct SBoth<'a> {
    c: SMap<'a>,
    r: SMap<'a>,
    mode: c_int,
    /// set once an insert has written `temp_key`
    tk_valid: bool,
}

impl<'a> SBoth<'a> {
    fn new(p: &'a Pair, mode: c_int) -> Self {
        SBoth {
            c: SMap::new(&p.c),
            r: SMap::new(&p.r),
            mode,
            tk_valid: false,
        }
    }
    fn check(&self, what: &str) {
        assert_bytes_eq(
            what,
            &unsafe { self.c.snap(self.tk_valid) },
            &unsafe { self.r.snap(self.tk_valid) },
        );
    }
    fn new_mode(&mut self, m: c_int) {
        unsafe {
            self.c.new_mode(m);
            self.r.new_mode(m);
        }
        self.tk_valid = false;
        self.check(&format!("after shmode_func({m})"));
    }
    fn put(&mut self, k: &CString, v: c_int) {
        let kp = k.as_ptr() as *mut c_char;
        unsafe {
            self.c.shput(kp, v, self.mode);
            self.r.shput(kp, v, self.mode);
        }
        // hmput_key writes temp_key for every string.mode except SH_NONE
        if self.mode >= STBDS_HM_STRING {
            let cm = unsafe { self.c.string_mode() };
            let rm = unsafe { self.r.string_mode() };
            assert_eq!(cm, rm, "string.mode differs");
            if matches!(cm, Some(1) | Some(2) | Some(3)) {
                self.tk_valid = true;
            }
        }
        self.check(&format!("after shput({:?}, {v})", k));
    }
    fn geti(&mut self, k: &CString) -> isize {
        let kp = k.as_ptr() as *mut c_char;
        let cv = unsafe { self.c.shgeti(kp, self.mode) };
        let rv = unsafe { self.r.shgeti(kp, self.mode) };
        assert_eq!(cv, rv, "shgeti({:?})", k);
        self.check(&format!("after shgeti({:?})", k));
        cv
    }
    fn get(&mut self, k: &CString) -> c_int {
        let kp = k.as_ptr() as *mut c_char;
        let cv = unsafe { self.c.shget(kp, self.mode) };
        let rv = unsafe { self.r.shget(kp, self.mode) };
        assert_eq!(cv, rv, "shget({:?})", k);
        self.check(&format!("after shget({:?})", k));
        cv
    }
    fn geti_ts(&mut self, k: &CString) -> isize {
        let kp = k.as_ptr() as *mut c_char;
        let cv = unsafe { self.c.shgeti_ts(kp, self.mode) };
        let rv = unsafe { self.r.shgeti_ts(kp, self.mode) };
        assert_eq!(cv, rv, "shgeti_ts({:?})", k);
        self.check(&format!("after shgeti_ts({:?})", k));
        cv
    }
    fn del(&mut self, k: &CString) -> isize {
        let kp = k.as_ptr() as *mut c_char;
        let cv = unsafe { self.c.shdel(kp, self.mode) };
        let rv = unsafe { self.r.shdel(kp, self.mode) };
        assert_eq!(cv, rv, "shdel({:?})", k);
        // A delete may shrink or rebuild the index, and stbds_make_hash_index
        // does not carry `temp_key` over, so it becomes indeterminate again.
        self.tk_valid = false;
        self.check(&format!("after shdel({:?})", k));
        cv
    }
    fn free(&mut self) {
        unsafe {
            self.c.shfree();
            self.r.shfree();
        }
        self.check("after shfree");
    }
}

fn keys(n: usize) -> Vec<CString> {
    (0..n)
        .map(|i| CString::new(format!("test_{i}")).unwrap())
        .collect()
}

#[test]
fn shmode_func_all_modes() {
    let p = load_pair();
    for m in [
        STBDS_SH_NONE,
        STBDS_SH_DEFAULT,
        STBDS_SH_STRDUP,
        STBDS_SH_ARENA,
        7,
        255,
    ] {
        p.reset_seed(DEFAULT_SEED);
        let mut b = SBoth::new(&p, STBDS_HM_STRING);
        b.new_mode(m);
        b.free();
    }
}

/// `string.mode == STBDS_SH_DEFAULT`: the map stores the caller's pointers.
#[test]
fn string_map_default_mode() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let ks = keys(300);
    let mut b = SBoth::new(&p, STBDS_HM_STRING);
    for (i, k) in ks.iter().enumerate() {
        b.put(k, i as c_int);
    }
    for (i, k) in ks.iter().enumerate() {
        assert_eq!(b.get(k), i as c_int);
        b.geti_ts(k);
    }
    let missing = keys(400);
    for k in &missing[300..] {
        assert_eq!(b.geti(k), -1);
    }
    // overwrite
    for (i, k) in ks.iter().enumerate() {
        b.put(k, -(i as c_int));
    }
    for k in ks.iter().step_by(3) {
        b.del(k);
    }
    for k in ks.iter() {
        b.geti(k);
    }
    for k in ks.iter() {
        b.del(k);
    }
    b.free();
}

#[test]
fn string_map_strdup_mode() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let ks = keys(250);
    let mut b = SBoth::new(&p, STBDS_HM_STRING);
    b.new_mode(STBDS_SH_STRDUP);
    for (i, k) in ks.iter().enumerate() {
        b.put(k, i as c_int * 7);
    }
    for (i, k) in ks.iter().enumerate() {
        assert_eq!(b.get(k), i as c_int * 7);
    }
    for k in ks.iter().step_by(2) {
        b.del(k);
    }
    for (i, k) in ks.iter().enumerate() {
        if i % 2 == 1 {
            assert_eq!(b.get(k), i as c_int * 7);
        } else {
            assert_eq!(b.geti(k), -1);
        }
    }
    // re-insert the deleted half (fresh strdup allocations)
    for k in ks.iter().step_by(2) {
        b.put(k, 12345);
    }
    b.free(); // must free every duplicated key
}

#[test]
fn string_map_arena_mode() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    // include keys long enough to force the arena's oversized-block path
    let mut ks: Vec<CString> = keys(200);
    ks.push(CString::new("z".repeat(1000)).unwrap());
    ks.push(CString::new("y".repeat(600)).unwrap());
    ks.push(CString::new("x".repeat(5000)).unwrap());

    let mut b = SBoth::new(&p, STBDS_HM_STRING);
    b.new_mode(STBDS_SH_ARENA);
    for (i, k) in ks.iter().enumerate() {
        b.put(k, i as c_int);
    }
    for (i, k) in ks.iter().enumerate() {
        assert_eq!(b.get(k), i as c_int);
    }
    for k in ks.iter().step_by(3) {
        b.del(k);
    }
    for k in ks.iter() {
        b.geti(k);
    }
    b.free();
}

#[test]
fn string_map_edge_case_keys() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let mut ks: Vec<CString> = vec![
        CString::new("").unwrap(),
        CString::new("a").unwrap(),
        CString::new("A").unwrap(),
        CString::new(vec![0xffu8, 0xfe, 0x01]).unwrap(),
        CString::new(vec![0x80u8; 40]).unwrap(),
    ];
    for n in 1..40usize {
        ks.push(CString::new("k".repeat(n)).unwrap());
    }
    let mut b = SBoth::new(&p, STBDS_HM_STRING);
    for (i, k) in ks.iter().enumerate() {
        b.put(k, i as c_int);
    }
    for k in ks.iter() {
        b.get(k);
        b.geti_ts(k);
    }
    for k in ks.iter() {
        b.del(k);
    }
    b.free();
}

#[test]
fn string_map_mode_two_ptr_to_string() {
    // stbds_pshget/pshput pass STBDS_HM_PTR_TO_STRING (== 2); everything
    // `mode >= STBDS_HM_STRING` must behave identically for put/lookup.
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let ks = keys(120);
    let mut b = SBoth::new(&p, 2);
    for (i, k) in ks.iter().enumerate() {
        b.put(k, i as c_int);
    }
    for (i, k) in ks.iter().enumerate() {
        assert_eq!(b.get(k), i as c_int);
        b.geti_ts(k);
    }
    b.free();
}

#[test]
fn string_map_delete_all_then_shrink() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    let ks = keys(400);
    let mut b = SBoth::new(&p, STBDS_HM_STRING);
    for (i, k) in ks.iter().enumerate() {
        b.put(k, i as c_int);
    }
    for k in ks.iter().rev() {
        b.del(k);
    }
    b.free();
}
