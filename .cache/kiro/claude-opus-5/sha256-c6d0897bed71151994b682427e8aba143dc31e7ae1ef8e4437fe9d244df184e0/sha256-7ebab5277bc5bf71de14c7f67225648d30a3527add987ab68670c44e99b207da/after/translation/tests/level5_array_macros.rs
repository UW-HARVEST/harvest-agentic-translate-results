//! Level 5: the public API from `include/lib.h` (`arr_push`) and the
//! header-only array macros, all of which funnel into `stbds_arrgrowf`.

mod common;

use common::*;
use std::ffi::c_void;

const ESZ: usize = 4; // sizeof(int)

/// A dynamic array handle bound to one library, driven exactly like the
/// `stbds_arr*` macros drive it.
struct Arr<'a> {
    lib: &'a Lib,
    p: *mut u8,
}

impl<'a> Arr<'a> {
    fn new(lib: &'a Lib) -> Self {
        Arr {
            lib,
            p: std::ptr::null_mut(),
        }
    }

    unsafe fn hdr(&self) -> *mut ArrHeader {
        (self.p as *mut ArrHeader).offset(-1)
    }
    unsafe fn len(&self) -> usize {
        if self.p.is_null() {
            0
        } else {
            (*self.hdr()).length
        }
    }
    unsafe fn cap(&self) -> usize {
        if self.p.is_null() {
            0
        } else {
            (*self.hdr()).capacity
        }
    }
    unsafe fn at(&self, i: usize) -> *mut i32 {
        (self.p as *mut i32).add(i)
    }

    /// `stbds_arrgrow(a, b, c)`
    unsafe fn grow(&mut self, addlen: usize, min_cap: usize) {
        self.p = self.lib.arrgrowf(self.p as *mut c_void, ESZ, addlen, min_cap);
    }

    /// `stbds_arrmaybegrow(a, n)`
    unsafe fn maybegrow(&mut self, n: usize) {
        if self.p.is_null() || (*self.hdr()).length + n > (*self.hdr()).capacity {
            self.grow(n, 0);
        }
    }

    /// `stbds_arrput(a, v)`
    unsafe fn put(&mut self, v: i32) {
        self.maybegrow(1);
        let h = self.hdr();
        self.at((*h).length).write(v);
        (*h).length += 1;
    }

    /// `stbds_arrpop(a)`
    unsafe fn pop(&mut self) -> i32 {
        let h = self.hdr();
        (*h).length -= 1;
        self.at((*h).length).read()
    }

    /// `stbds_arrsetcap(a, n)`
    unsafe fn setcap(&mut self, n: usize) {
        self.grow(0, n);
    }

    /// `stbds_arrsetlen(a, n)`
    unsafe fn setlen(&mut self, n: usize) {
        if self.cap() < n {
            self.setcap(n);
        }
        if !self.p.is_null() {
            (*self.hdr()).length = n;
        }
    }

    /// `stbds_arraddnindex(a, n)`
    unsafe fn addnindex(&mut self, n: usize) -> isize {
        self.maybegrow(n);
        if n != 0 {
            let h = self.hdr();
            (*h).length += n;
            ((*h).length - n) as isize
        } else if self.p.is_null() {
            0
        } else {
            (*self.hdr()).length as isize
        }
    }

    /// `stbds_arraddnptr(a, n)` - returns the offset, since raw addresses
    /// cannot be compared between libraries.
    unsafe fn addnptr_off(&mut self, n: usize) -> isize {
        self.maybegrow(n);
        if n != 0 {
            let h = self.hdr();
            (*h).length += n;
            ((*h).length - n) as isize
        } else {
            0
        }
    }

    /// `stbds_arrinsn(a, i, n)`
    unsafe fn insn(&mut self, i: usize, n: usize) {
        let _ = self.addnindex(n);
        let len = (*self.hdr()).length;
        std::ptr::copy(
            self.at(i),
            self.at(i + n),
            len - n - i,
        );
    }

    /// `stbds_arrins(a, i, v)`
    unsafe fn ins(&mut self, i: usize, v: i32) {
        self.insn(i, 1);
        self.at(i).write(v);
    }

    /// `stbds_arrdeln(a, i, n)`
    unsafe fn deln(&mut self, i: usize, n: usize) {
        let h = self.hdr();
        let len = (*h).length;
        std::ptr::copy(self.at(i + n), self.at(i), len - n - i);
        (*h).length -= n;
    }

    /// `stbds_arrdelswap(a, i)`
    unsafe fn delswap(&mut self, i: usize) {
        let h = self.hdr();
        let last = self.at((*h).length - 1).read();
        self.at(i).write(last);
        (*h).length -= 1;
    }

    /// `stbds_arrfree(a)`
    unsafe fn free(&mut self) {
        if !self.p.is_null() {
            self.lib.arrfreef(self.p as *mut c_void);
            self.p = std::ptr::null_mut();
        }
    }

    unsafe fn snap(&self) -> Snap {
        snap_arr(self.p, Fmt::Raw, 0)
    }

    unsafe fn contents(&self) -> Vec<i32> {
        (0..self.len()).map(|i| self.at(i).read()).collect()
    }
}

#[test]
fn arr_push_public_api() {
    let _g = guard();
    let libs = libs();
    // `arr_push` has no return value; it exists to exercise the grow/free
    // cycle. Run the whole documented range of shapes through both libraries.
    for num in [
        -1000i32, -1, 0, 1, 2, 49, 50, 51, 99, 100, 101, 149, 150, 500, 1000, 3000,
    ] {
        unsafe {
            libs.c.arr_push(num);
            libs.rs.arr_push(num);
        }
    }
}

#[test]
fn arr_push_emulated_step_by_step() {
    // Replay `arr_push`'s body through the exported `arrgrowf`/`arrfreef`,
    // comparing the header and payload after every single push.
    let _g = guard();
    let libs = libs();
    let num = 400i32;
    unsafe {
        let mut c = Arr::new(&libs.c);
        let mut r = Arr::new(&libs.rs);
        assert_eq!(c.len(), 0);
        assert_eq!(r.len(), 0);
        let mut i = 0i32;
        while i < num {
            let mut j = 0i32;
            while j < i {
                c.put(j);
                r.put(j);
                assert_eq!(c.snap(), r.snap(), "arr_push i={i} j={j} header");
                assert_eq!(c.contents(), r.contents(), "arr_push i={i} j={j} payload");
                j += 1;
            }
            c.free();
            r.free();
            assert_eq!(c.snap(), r.snap(), "arr_push i={i} after free");
            i = i.wrapping_add(50);
        }
    }
}

#[test]
fn arr_setcap_setlen() {
    let _g = guard();
    let libs = libs();
    unsafe {
        let mut c = Arr::new(&libs.c);
        let mut r = Arr::new(&libs.rs);
        for n in [0usize, 1, 2, 3, 4, 5, 8, 9, 16, 17, 100, 99, 4, 0, 1000, 1] {
            c.setcap(n);
            r.setcap(n);
            assert_eq!(c.snap(), r.snap(), "setcap({n})");
        }
        for n in [0usize, 1, 5, 4, 100, 3, 2000, 0] {
            c.setlen(n);
            r.setlen(n);
            assert_eq!(c.snap(), r.snap(), "setlen({n})");
        }
        c.free();
        r.free();
    }
}

#[test]
fn arr_addn_variants() {
    let _g = guard();
    let libs = libs();
    unsafe {
        let mut c = Arr::new(&libs.c);
        let mut r = Arr::new(&libs.rs);
        for n in [0usize, 1, 0, 3, 4, 0, 7, 100, 0, 1] {
            let ci = c.addnindex(n);
            let ri = r.addnindex(n);
            assert_eq!(ci, ri, "arraddnindex({n})");
            assert_eq!(c.snap(), r.snap(), "arraddnindex({n}) header");
        }
        c.free();
        r.free();

        let mut c = Arr::new(&libs.c);
        let mut r = Arr::new(&libs.rs);
        for n in [1usize, 2, 0, 5, 64, 0, 3] {
            let co = c.addnptr_off(n);
            let ro = r.addnptr_off(n);
            assert_eq!(co, ro, "arraddnptr({n})");
            assert_eq!(c.snap(), r.snap(), "arraddnptr({n}) header");
        }
        c.free();
        r.free();
    }
}

#[test]
fn arr_insert_delete_pop() {
    let _g = guard();
    let libs = libs();
    let mut rng = Rng::new(0xF00DF00D);
    unsafe {
        let mut c = Arr::new(&libs.c);
        let mut r = Arr::new(&libs.rs);

        for i in 0..64i32 {
            c.put(i);
            r.put(i);
        }
        assert_eq!(c.contents(), r.contents());

        for step in 0..1500 {
            let len = c.len();
            assert_eq!(len, r.len(), "length diverged at step {step}");
            match rng.below(6) {
                0 => {
                    let v = rng.next_i32();
                    c.put(v);
                    r.put(v);
                }
                1 if len > 0 => {
                    let cv = c.pop();
                    let rv = r.pop();
                    assert_eq!(cv, rv, "arrpop at step {step}");
                }
                2 => {
                    let i = if len == 0 { 0 } else { rng.below(len as u64 + 1) as usize };
                    let v = rng.next_i32();
                    c.ins(i, v);
                    r.ins(i, v);
                }
                3 if len > 0 => {
                    let i = rng.below(len as u64) as usize;
                    let n = 1 + rng.below((len - i) as u64) as usize;
                    c.deln(i, n);
                    r.deln(i, n);
                }
                4 if len > 0 => {
                    let i = rng.below(len as u64) as usize;
                    c.delswap(i);
                    r.delswap(i);
                }
                5 => {
                    let n = rng.below(8) as usize;
                    let i = if len == 0 { 0 } else { rng.below(len as u64 + 1) as usize };
                    c.insn(i, n);
                    r.insn(i, n);
                    // Fill the hole so the payload comparison is meaningful.
                    for k in 0..n {
                        let v = rng.next_i32();
                        c.at(i + k).write(v);
                        r.at(i + k).write(v);
                    }
                }
                _ => continue,
            }
            assert_eq!(c.snap(), r.snap(), "header diverged at step {step}");
            assert_eq!(c.contents(), r.contents(), "payload diverged at step {step}");
        }
        c.free();
        r.free();
    }
}

#[test]
fn arr_free_and_reuse() {
    let _g = guard();
    let libs = libs();
    unsafe {
        let mut c = Arr::new(&libs.c);
        let mut r = Arr::new(&libs.rs);
        for round in 0..50 {
            for i in 0..(round * 3) {
                c.put(i);
                r.put(i);
            }
            assert_eq!(c.snap(), r.snap(), "round {round}");
            assert_eq!(c.contents(), r.contents(), "round {round}");
            c.free();
            r.free();
            assert_eq!(c.snap(), r.snap(), "round {round} after free");
        }
    }
}
