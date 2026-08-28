//! Phase D — hardware-enforced bounds equivalence.
//!
//! `check()` can only compare *values*, so it cannot see a read one element past
//! the window. These tests place the buffers against `PROT_NONE` guard pages so
//! that any access outside `[dst, dst + numElem)` or past `src`'s terminator
//! faults immediately. Both the C `.so` and the Rust `.so` are driven through the
//! identical layout: if the Rust touches memory the C does not, the process dies
//! and the test fails.

mod common;

use common::*;
use std::ffi::c_void;

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;

extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
    fn sysconf(name: i32) -> i64;
}

fn page_size() -> usize {
    // _SC_PAGESIZE == 30 on Linux
    let v = unsafe { sysconf(30) };
    assert!(v > 0, "sysconf(_SC_PAGESIZE) failed");
    v as usize
}

/// Three consecutive pages: `PROT_NONE | RW | PROT_NONE`.
struct Fenced {
    base: *mut c_void,
    total: usize,
    /// Start of the writable middle page.
    data: *mut WcharT,
    /// Number of `WcharT` elements the middle page holds.
    elems: usize,
}

impl Fenced {
    fn new() -> Self {
        let ps = page_size();
        let total = ps * 3;
        let base = unsafe {
            mmap(
                std::ptr::null_mut(),
                total,
                PROT_NONE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(base as isize != -1, "mmap failed");
        let mid = unsafe { (base as *mut u8).add(ps) };
        let rc = unsafe { mprotect(mid as *mut c_void, ps, PROT_READ | PROT_WRITE) };
        assert_eq!(rc, 0, "mprotect failed");
        Fenced {
            base,
            total,
            data: mid as *mut WcharT,
            elems: ps / std::mem::size_of::<WcharT>(),
        }
    }

    /// Pointer to a region of `n` elements whose END is flush against the upper
    /// guard page, so element `n` is unmapped.
    fn flush_end(&self, n: usize) -> *mut WcharT {
        assert!(n <= self.elems);
        unsafe { self.data.add(self.elems - n) }
    }

    /// Fill `[p, p+n)`.
    fn fill(p: *mut WcharT, vals: &[WcharT]) {
        for (i, v) in vals.iter().enumerate() {
            unsafe { *p.add(i) = *v };
        }
    }

    fn read(p: *const WcharT, n: usize) -> Vec<WcharT> {
        (0..n).map(|i| unsafe { *p.add(i) }).collect()
    }
}

impl Drop for Fenced {
    fn drop(&mut self) {
        unsafe { munmap(self.base, self.total) };
    }
}

/// Run one fenced scenario against both libraries and compare.
///
/// `build` fills the dst window (length `n`) and returns the src contents; the
/// src is placed in its own fenced page, flush against its guard page, so a read
/// past its terminator also faults.
fn fenced_case(
    name: &str,
    n: usize,
    num_elem: usize,
    dst_init: &[WcharT],
    src_init: Option<&[WcharT]>,
) {
    assert_eq!(dst_init.len(), n);
    let mut results = Vec::new();
    let l = libs();
    for (which, f) in [("C", l.c), ("RS", l.rs)] {
        let dfence = Fenced::new();
        let dst = dfence.flush_end(n);
        Fenced::fill(dst, dst_init);

        let (sfence, src_ptr) = match src_init {
            None => (None, std::ptr::null::<WcharT>()),
            Some(s) => {
                let sf = Fenced::new();
                let sp = sf.flush_end(s.len());
                Fenced::fill(sp, s);
                (Some(sf), sp as *const WcharT)
            }
        };

        let ret = unsafe { f(dst, num_elem, src_ptr) };
        let after = Fenced::read(dst, n);
        let src_after = sfence
            .as_ref()
            .map(|sf| Fenced::read(sf.flush_end(src_init.unwrap().len()), src_init.unwrap().len()));
        results.push((which, ret, after, src_after));
    }
    let (_, c_ret, c_dst, c_src) = &results[0];
    let (_, rs_ret, rs_dst, rs_src) = &results[1];
    assert_eq!(c_ret, rs_ret, "`{name}`: return codes diverged");
    assert_eq!(c_dst, rs_dst, "`{name}`: dst diverged");
    assert_eq!(c_src, rs_src, "`{name}`: src diverged");
}

#[test]
fn fenced_unterminated_dst_reads_exactly_num_elem() {
    // The scan loop must stop at dst+numElem; reading dst[numElem] would fault.
    for n in [1usize, 2, 3, 7, 8, 63, 64, 255, 256, 1024] {
        let dst: Vec<WcharT> = (0..n).map(|i| (i as WcharT) | 0x4000_0000).collect();
        fenced_case(
            &format!("fenced_unterminated_n{n}"),
            n,
            n,
            &dst,
            Some(&[0x41, 0x42, 0]),
        );
    }
}

#[test]
fn fenced_copy_loop_writes_exactly_num_elem() {
    // dst empty, src far longer: the copy loop must stop at dst+numElem.
    // `n` is capped so the (2n+9)-element src still fits in one page.
    for n in [1usize, 2, 3, 7, 8, 64, 255, 256, 400] {
        let mut dst: Vec<WcharT> = vec![0x5A5A_5A5A; n];
        dst[0] = 0;
        let src: Vec<WcharT> = (0..(n * 2 + 8)).map(|i| (i as WcharT) | 0x100).collect();
        let mut src = src;
        src.push(0);
        fenced_case(
            &format!("fenced_copyloop_n{n}"),
            n,
            n,
            &dst,
            Some(&src),
        );
    }
}

#[test]
fn fenced_src_read_stops_at_terminator() {
    // src is flush against its guard page with the terminator as its LAST element:
    // reading one element past the terminator would fault.
    for l in [0usize, 1, 2, 7, 64, 512] {
        let n = l + 4;
        let mut dst: Vec<WcharT> = vec![0x1234_5678; n];
        dst[0] = 0;
        let mut src: Vec<WcharT> = (0..l).map(|i| (i as WcharT) | 0x200).collect();
        src.push(0); // terminator is the last mapped element
        fenced_case(&format!("fenced_src_term_l{l}"), n, n, &dst, Some(&src));
    }
}

#[test]
fn fenced_num_elem_smaller_than_allocation() {
    // numElem < the mapped region: the guard page is further away, but the library
    // must still stop at dst+numElem. Verified by value comparison here (the
    // fence catches the coarse case above).
    let ps = page_size();
    let elems = ps / 4;
    for &num_elem in &[1usize, 2, 17, 100] {
        let n = elems.min(num_elem + 32);
        let dst: Vec<WcharT> = (0..n).map(|i| (i as WcharT) | 0x7000_0000).collect();
        fenced_case(
            &format!("fenced_sub_window_{num_elem}"),
            n,
            num_elem,
            &dst,
            Some(&[0x41, 0x42, 0x43, 0]),
        );
    }
}

#[test]
fn fenced_null_dst_and_null_src_never_dereference() {
    // A NULL deref would fault; both must return 22 quietly.
    let l = libs();
    for &n in &[0usize, 1, 4096, usize::MAX] {
        let c = unsafe { (l.c)(std::ptr::null_mut(), n, std::ptr::null()) };
        let r = unsafe { (l.rs)(std::ptr::null_mut(), n, std::ptr::null()) };
        assert_eq!((c, r), (22, 22), "n={n:#x}");
    }
    // src == NULL with a fenced dst: exactly one element (dst[0]) may be written.
    for n in [1usize, 2, 64, 1024] {
        let dst: Vec<WcharT> = (0..n).map(|i| (i as WcharT) | 0x0300_0000).collect();
        fenced_case(&format!("fenced_src_null_n{n}"), n, n, &dst, None);
    }
}

#[test]
fn fenced_overflowing_num_elem_touches_only_dst0() {
    // dst + numElem wraps; the loops must not run, so only dst[0] is written and
    // nothing outside the window is read.
    for &num_elem in &[usize::MAX, usize::MAX / 4, 1usize << 62, 1usize << 63] {
        for n in [1usize, 2, 64] {
            let mut dst: Vec<WcharT> = vec![0x0BAD_F00D; n];
            dst[0] = 0;
            fenced_case(
                &format!("fenced_wrap_{num_elem:#x}_n{n}"),
                n,
                num_elem,
                &dst,
                Some(&[0x41, 0]),
            );
        }
    }
}
