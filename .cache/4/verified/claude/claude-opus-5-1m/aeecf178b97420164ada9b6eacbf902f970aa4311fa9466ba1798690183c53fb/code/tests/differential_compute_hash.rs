//! Differential tests for `compute_hash`.
//!
//! Covers CONFIGS.md rows 19-28 and ERRORS.md rows 8 and 9.
//!
//! `compute_hash` never dereferences `mb->data`; it only *compares* the stored
//! pointer values.  That lets the test drive every one of the 7 reachable
//! (data-order x struct-order) combinations deterministically by planting
//! synthetic addresses, including values that distinguish an unsigned pointer
//! comparison (what C does) from a signed one.

mod common;

use common::*;
use core::ffi::c_int;

/// Two `MemoryBlock`s laid out so that `&arr[0] < &arr[1]` is guaranteed.
struct Pair {
    arr: [MemoryBlock; 2],
}

impl Pair {
    fn new(d0: usize, d1: usize) -> Pair {
        Pair {
            arr: [
                MemoryBlock {
                    data: d0 as *mut c_int,
                    size: 3,
                },
                MemoryBlock {
                    data: d1 as *mut c_int,
                    size: 4,
                },
            ],
        }
    }
    fn lo(&mut self) -> *mut MemoryBlock {
        &mut self.arr[0] as *mut MemoryBlock
    }
    fn hi(&mut self) -> *mut MemoryBlock {
        &mut self.arr[1] as *mut MemoryBlock
    }
}

fn check(c: &Impl, r: &Impl, a: *mut MemoryBlock, b: *mut MemoryBlock, ctx: &str) -> c_int {
    let cv = unsafe { (c.compute_hash)(a, b) };
    let rv = unsafe { (r.compute_hash)(a, b) };
    assert_eq!(cv, rv, "{ctx}: compute_hash mismatch (C={cv}, Rust={rv})");
    cv
}

#[test]
fn compute_hash_differential() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0x0000_0003);

    // sanity: the two struct slots really are ordered
    {
        let mut p = Pair::new(1, 2);
        assert!(p.lo() < p.hi(), "test bug: array elements not ordered");
    }

    // ---- row 19: same object -> 0 ---------------------------------------
    {
        let mut p = Pair::new(0x1000, 0x2000);
        let lo = p.lo();
        assert_eq!(check(&c, &r, lo, lo, "row19/same"), 0);
        let hi = p.hi();
        assert_eq!(check(&c, &r, hi, hi, "row19/same-hi"), 0);
    }

    // ---- row 20: data1 < data2, p1 < p2 -> 110 --------------------------
    {
        let mut p = Pair::new(0x1000, 0x2000);
        let (lo, hi) = (p.lo(), p.hi());
        assert_eq!(check(&c, &r, lo, hi, "row20"), 110);
    }

    // ---- row 21: data1 < data2, p1 > p2 -> 120 --------------------------
    {
        // arr[1] (the higher struct) holds the *lower* data pointer
        let mut p = Pair::new(0x9000, 0x1000);
        let (lo, hi) = (p.lo(), p.hi());
        assert_eq!(check(&c, &r, hi, lo, "row21"), 120);
    }

    // ---- row 22: data1 > data2, p1 < p2 -> 210 --------------------------
    {
        let mut p = Pair::new(0x9000, 0x1000);
        let (lo, hi) = (p.lo(), p.hi());
        assert_eq!(check(&c, &r, lo, hi, "row22"), 210);
    }

    // ---- row 23: data1 > data2, p1 > p2 -> 220 --------------------------
    {
        let mut p = Pair::new(0x1000, 0x9000);
        let (lo, hi) = (p.lo(), p.hi());
        assert_eq!(check(&c, &r, hi, lo, "row23"), 220);
    }

    // ---- row 24: data1 == data2 (aliased), p1 < p2 -> 10 ----------------
    {
        let mut p = Pair::new(0x4242, 0x4242);
        let (lo, hi) = (p.lo(), p.hi());
        assert_eq!(check(&c, &r, lo, hi, "row24"), 10);
    }

    // ---- row 25: data1 == data2 (aliased), p1 > p2 -> 20 ----------------
    {
        let mut p = Pair::new(0x4242, 0x4242);
        let (lo, hi) = (p.lo(), p.hi());
        assert_eq!(check(&c, &r, hi, lo, "row25"), 20);
    }

    // ---- row 26: data1 == data2 == NULL --------------------------------
    {
        let mut p = Pair::new(0, 0);
        let (lo, hi) = (p.lo(), p.hi());
        assert_eq!(check(&c, &r, lo, hi, "row26/lo-hi"), 10);
        assert_eq!(check(&c, &r, hi, lo, "row26/hi-lo"), 20);
    }
    // NULL vs non-NULL
    {
        let mut p = Pair::new(0, 0x10);
        let (lo, hi) = (p.lo(), p.hi());
        assert_eq!(check(&c, &r, lo, hi, "row26/null-lt"), 110);
        assert_eq!(check(&c, &r, hi, lo, "row26/null-gt"), 220);
    }

    // ---- row 27: unsigned pointer comparison ---------------------------
    // If the comparison were *signed*, 0x8000_0000_0000_0000 would look
    // negative and these expectations would flip.
    {
        let high = 0x8000_0000_0000_0000usize;
        let mut p = Pair::new(1, high);
        let (lo, hi) = (p.lo(), p.hi());
        assert_eq!(check(&c, &r, lo, hi, "row27/low-vs-high-bit"), 110);
        assert_eq!(check(&c, &r, hi, lo, "row27/high-bit-vs-low"), 220);
    }
    {
        let mut p = Pair::new(usize::MAX, 1);
        let (lo, hi) = (p.lo(), p.hi());
        assert_eq!(check(&c, &r, lo, hi, "row27/max-vs-1"), 210);
    }
    // random synthetic pointer values, including the extremes
    let specials = [
        0usize,
        1,
        2,
        4,
        0x7FFF_FFFF_FFFF_FFFF,
        0x8000_0000_0000_0000,
        0x8000_0000_0000_0001,
        usize::MAX,
        usize::MAX - 1,
        0xFFFF_FFFF,
        0x1_0000_0000,
    ];
    for (i, &d0) in specials.iter().enumerate() {
        for (j, &d1) in specials.iter().enumerate() {
            let mut p = Pair::new(d0, d1);
            let (lo, hi) = (p.lo(), p.hi());
            check(&c, &r, lo, hi, &format!("row27/special[{i}][{j}]/lo-hi"));
            check(&c, &r, hi, lo, &format!("row27/special[{i}][{j}]/hi-lo"));
            check(&c, &r, lo, lo, &format!("row27/special[{i}][{j}]/same"));
        }
    }
    for i in 0..2000 {
        let d0 = match rng.below(4) {
            0 => 0,
            1 => rng.next_u64() as usize,
            2 => rng.below(8) as usize,
            _ => (rng.next_u64() | 0x8000_0000_0000_0000) as usize,
        };
        let d1 = match rng.below(4) {
            0 => 0,
            1 => rng.next_u64() as usize,
            2 => d0, // force equality sometimes
            _ => (rng.next_u64() >> 1) as usize,
        };
        let mut p = Pair::new(d0, d1);
        let (lo, hi) = (p.lo(), p.hi());
        check(&c, &r, lo, hi, &format!("row27/rand{i}/lo-hi"));
        check(&c, &r, hi, lo, &format!("row27/rand{i}/hi-lo"));
    }

    // ---- row 28: fed with real allocate_block results -------------------
    for i in 0..200 {
        let n1 = rng.below(20) as usize;
        let n2 = rng.below(20) as usize;
        unsafe {
            let m1 = (c.allocate_block)(n1, rng.next_i32());
            let m2 = (c.allocate_block)(n2, rng.next_i32());
            assert!(!m1.is_null() && !m2.is_null());
            check(&c, &r, m1, m2, &format!("row28/real{i}/1-2"));
            check(&c, &r, m2, m1, &format!("row28/real{i}/2-1"));
            check(&c, &r, m1, m1, &format!("row28/real{i}/1-1"));
            (c.free_block)(m1);
            (r.free_block)(m2);
        }
    }

    // ---- ERRORS #8: mb1 == NULL -> SIGSEGV in both ----------------------
    {
        let mut p = Pair::new(0x1000, 0x2000);
        let good = p.lo() as usize;
        let (ca, ra) = fork_pair(|which, _buf| {
            let imp = if which { &r } else { &c };
            unsafe {
                let v = (imp.compute_hash)(core::ptr::null_mut(), good as *mut MemoryBlock);
                core::hint::black_box(v);
            }
            0
        });
        assert_eq!(
            ca.signal(),
            Some(11),
            "ERRORS#8: C compute_hash(NULL, p) should SIGSEGV, got {}",
            ca.describe()
        );
        assert_eq!(
            ca.signal(),
            ra.signal(),
            "ERRORS#8: divergence: C={} Rust={}",
            ca.describe(),
            ra.describe()
        );
    }

    // ---- ERRORS #9: mb2 == NULL -> SIGSEGV in both ----------------------
    {
        let mut p = Pair::new(0x1000, 0x2000);
        let good = p.lo() as usize;
        let (ca, ra) = fork_pair(|which, _buf| {
            let imp = if which { &r } else { &c };
            unsafe {
                let v = (imp.compute_hash)(good as *mut MemoryBlock, core::ptr::null_mut());
                core::hint::black_box(v);
            }
            0
        });
        assert_eq!(
            ca.signal(),
            Some(11),
            "ERRORS#9: C compute_hash(p, NULL) should SIGSEGV, got {}",
            ca.describe()
        );
        assert_eq!(
            ca.signal(),
            ra.signal(),
            "ERRORS#9: divergence: C={} Rust={}",
            ca.describe(),
            ra.describe()
        );
    }

    // ---- both NULL -----------------------------------------------------
    {
        let (ca, ra) = fork_pair(|which, _buf| {
            let imp = if which { &r } else { &c };
            unsafe {
                let v = (imp.compute_hash)(core::ptr::null_mut(), core::ptr::null_mut());
                core::hint::black_box(v);
            }
            0
        });
        assert_eq!(
            ca.signal(),
            ra.signal(),
            "compute_hash(NULL,NULL): divergence: C={} Rust={}",
            ca.describe(),
            ra.describe()
        );
    }
}
