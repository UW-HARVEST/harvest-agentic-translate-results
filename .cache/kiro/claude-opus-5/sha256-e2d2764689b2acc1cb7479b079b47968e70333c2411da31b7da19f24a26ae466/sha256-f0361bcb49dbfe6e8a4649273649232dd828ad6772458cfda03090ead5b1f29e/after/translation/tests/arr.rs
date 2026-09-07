//! Phase B rows 12-19: `stbds_arrgrowf`, `stbds_arrfreef`, `arr_push`,
//! `strkey`.

mod common;
use common::*;
use std::ffi::c_void;

const SEED: u64 = 0x5EED_1234;

/// Grow both libs' arrays in lock-step and compare header + contents.
struct Pair<'a> {
    c: &'a Lib,
    r: &'a Lib,
    ac: *mut c_void,
    ar: *mut c_void,
    elemsize: usize,
}

impl<'a> Pair<'a> {
    fn new(c: &'a Lib, r: &'a Lib, elemsize: usize) -> Pair<'a> {
        Pair { c, r, ac: std::ptr::null_mut(), ar: std::ptr::null_mut(), elemsize }
    }
    fn grow(&mut self, addlen: usize, min_cap: usize) {
        self.ac = unsafe { (self.c.arrgrowf)(self.ac, self.elemsize, addlen, min_cap) };
        self.ar = unsafe { (self.r.arrgrowf)(self.ar, self.elemsize, addlen, min_cap) };
    }
    fn desc(&self, dump: usize) -> (String, String) {
        unsafe {
            (
                describe_arr(self.ac, self.elemsize, dump),
                describe_arr(self.ar, self.elemsize, dump),
            )
        }
    }
    /// Emulate `arrput`: write `len` elements of filler then bump the length.
    fn set_len_and_fill(&mut self, n: usize, fill: u8) {
        unsafe {
            for (a, _) in [(self.ac, 0u8), (self.ar, 0u8)] {
                let h = header_of(a);
                std::ptr::write_bytes(
                    (a as *mut u8).add((*h).length * self.elemsize),
                    fill,
                    (n - (*h).length) * self.elemsize,
                );
                (*h).length = n;
            }
        }
    }
    fn free(&mut self) {
        unsafe {
            if !self.ac.is_null() {
                (self.c.arrfreef)(self.ac);
            }
            if !self.ar.is_null() {
                (self.r.arrfreef)(self.ar);
            }
        }
        self.ac = std::ptr::null_mut();
        self.ar = std::ptr::null_mut();
    }
}

#[test]
fn cfg_12_arrgrowf_fresh() {
    let (c, r) = load_pair();
    for elemsize in [1usize, 2, 4, 8, 16, 32, 64] {
        for addlen in [0usize, 1, 2, 7, 33] {
            for min_cap in [0usize, 1, 2, 4, 5, 8, 100, 1000] {
                let mut p = Pair::new(&c, &r, elemsize);
                p.grow(addlen, min_cap);
                let (dc, dr) = p.desc(0);
                assert_same(
                    &format!("arrgrowf fresh e={} add={} min={}", elemsize, addlen, min_cap),
                    &dc,
                    &dr,
                );
                p.free();
            }
        }
    }
}

#[test]
fn cfg_13_arrgrowf_noop() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 13);
    for elemsize in [1usize, 4, 8, 16] {
        let mut p = Pair::new(&c, &r, elemsize);
        p.grow(0, 64); // cap = 64
        for _ in 0..200 {
            let min_cap = rng.below(65); // <= cap -> no-op
            let before = p.desc(0);
            let (pc, pr) = (p.ac, p.ar);
            p.grow(0, min_cap);
            assert_eq!(pc, p.ac, "C arrgrowf should have been a no-op");
            assert_eq!(pr, p.ar, "Rust arrgrowf should have been a no-op");
            let after = p.desc(0);
            assert_same("arrgrowf noop", &after.0, &after.1);
            assert_eq!(before.0, after.0);
        }
        p.free();
    }
}

#[test]
fn cfg_14_arrgrowf_double() {
    let (c, r) = load_pair();
    for elemsize in [1usize, 4, 8, 16, 24] {
        for start in [4usize, 8, 16, 64, 1000] {
            for delta in [1usize, 2, 3] {
                let mut p = Pair::new(&c, &r, elemsize);
                p.grow(0, start);
                // min_cap in (cap, 2*cap) -> capacity becomes 2*cap
                p.grow(0, start + delta);
                let (dc, dr) = p.desc(0);
                assert_same(
                    &format!("arrgrowf double e={} start={} d={}", elemsize, start, delta),
                    &dc,
                    &dr,
                );
                p.free();
            }
        }
    }
}

#[test]
fn cfg_15_arrgrowf_exact() {
    let (c, r) = load_pair();
    for elemsize in [1usize, 4, 8, 16] {
        for start in [4usize, 8, 64] {
            for target in [start * 2, start * 2 + 1, start * 3, start * 100] {
                let mut p = Pair::new(&c, &r, elemsize);
                p.grow(0, start);
                p.grow(0, target);
                let (dc, dr) = p.desc(0);
                assert_same(
                    &format!("arrgrowf exact e={} start={} tgt={}", elemsize, start, target),
                    &dc,
                    &dr,
                );
                p.free();
            }
        }
    }
}

#[test]
fn cfg_16_arrgrowf_growth_sequence() {
    // The capacity sequence produced by append-one-at-a-time growth is the
    // single most characteristic behaviour of stbds_arrgrowf.
    let (c, r) = load_pair();
    for elemsize in [1usize, 4, 8, 16] {
        let mut p = Pair::new(&c, &r, elemsize);
        let mut cseq = String::new();
        let mut rseq = String::new();
        for n in 0..2000usize {
            // stbds_arrmaybegrow(a,1)
            let need = unsafe {
                p.ac.is_null() || (*header_of(p.ac)).length + 1 > (*header_of(p.ac)).capacity
            };
            let need_r = unsafe {
                p.ar.is_null() || (*header_of(p.ar)).length + 1 > (*header_of(p.ar)).capacity
            };
            assert_eq!(need, need_r, "grow decision diverged at n={}", n);
            if need {
                p.grow(1, 0);
                unsafe {
                    cseq.push_str(&format!("{},", (*header_of(p.ac)).capacity));
                    rseq.push_str(&format!("{},", (*header_of(p.ar)).capacity));
                }
            }
            // write element n
            unsafe {
                for a in [p.ac, p.ar] {
                    let h = header_of(a);
                    let idx = (*h).length;
                    let dst = (a as *mut u8).add(idx * elemsize);
                    for b in 0..elemsize {
                        *dst.add(b) = (n as u8).wrapping_add(b as u8);
                    }
                    (*h).length = idx + 1;
                }
            }
        }
        assert_eq!(cseq, rseq, "capacity sequence mismatch, elemsize={}", elemsize);
        let (dc, dr) = p.desc(2000);
        assert_same(&format!("growth sequence e={}", elemsize), &dc, &dr);
        p.free();
    }
}

#[test]
fn cfg_17_arrgrow_free_cycle() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..300 {
        let elemsize = [1usize, 2, 4, 8, 16, 32][rng.below(6)];
        let mut p = Pair::new(&c, &r, elemsize);
        let rounds = 1 + rng.below(12);
        for _ in 0..rounds {
            let addlen = rng.below(20);
            let min_cap = rng.below(200);
            p.grow(addlen, min_cap);
            // legally bump the length as arraddn would
            let cap = unsafe { (*header_of(p.ac)).capacity };
            let want = unsafe { (*header_of(p.ac)).length + addlen };
            if want <= cap {
                p.set_len_and_fill(want, rng.byte());
            }
            let (dc, dr) = p.desc(64);
            assert_same("arrgrow/free cycle", &dc, &dr);
        }
        p.free();
        let (dc, dr) = p.desc(0);
        assert_same("after free", &dc, &dr);
    }
}

#[test]
fn cfg_18_arr_push_range() {
    // `arr_push` is the library's own driver routine. It has no return value
    // and no observable output other than "does not crash / does not abort",
    // so run it in a child process and compare the exit status.
    let (c, r) = load_pair();
    for num in [0i32, 1, 2, 3, 49, 50, 51, 52, 99, 100, 101, 499, 500, 501, 1000, 5000] {
        let dc = run_in_child(|| unsafe { (c.arr_push)(num) });
        let dr = run_in_child(|| unsafe { (r.arr_push)(num) });
        assert_eq!(dc, dr, "arr_push({}) death mismatch", num);
        assert_eq!(dc, Death::Exited(0), "arr_push({}) should succeed", num);
    }
    // in-process too, so a leak/corruption shows up under the test allocator
    for num in [0i32, 1, 50, 100, 500] {
        unsafe {
            (c.arr_push)(num);
            (r.arr_push)(num);
        }
    }
}

#[test]
fn cfg_19_strkey() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 19);
    let mut vals: Vec<i32> = vec![0, 1, 2, 9, 10, 11, 99, 100, 101, 999, 1000, 12345, i32::MAX];
    vals.extend([-1i32, -9, -10, -99, -100, -12345, i32::MIN]);
    for _ in 0..500 {
        vals.push(rng.next_u32() as i32);
    }
    for n in vals {
        unsafe {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            assert!(!pc.is_null() && !pr.is_null());
            let sc = std::ffi::CStr::from_ptr(pc).to_bytes().to_vec();
            let sr = std::ffi::CStr::from_ptr(pr).to_bytes().to_vec();
            assert_eq!(
                sc,
                sr,
                "strkey({}) mismatch: C={:?} RUST={:?}",
                n,
                String::from_utf8_lossy(&sc),
                String::from_utf8_lossy(&sr)
            );
            // the pointer must be stable (a static buffer), not freshly malloc'd
            let pc2 = (c.strkey)(n);
            let pr2 = (r.strkey)(n);
            assert_eq!(pc, pc2, "C strkey buffer not static");
            assert_eq!(pr, pr2, "Rust strkey buffer not static");
        }
    }
}
