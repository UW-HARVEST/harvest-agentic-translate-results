//! Phase B, Group 4 of `CONFIGS.md`: the raw dynamic array
//! (`stbds_arrgrowf` / `stbds_arrfreef`) plus every `arr*` macro protocol
//! layered on top of them.

mod common;

use common::*;

/// Grow both libraries' arrays identically and compare the resulting headers.
fn both_grow(p: &Pair, elemsize: usize, ops: &[(usize, usize)]) {
    let mut ca = Arr::new(&p.c, elemsize);
    let mut ra = Arr::new(&p.r, elemsize);
    for (i, &(addlen, min_cap)) in ops.iter().enumerate() {
        ca.grow(addlen, min_cap);
        ra.grow(addlen, min_cap);
        diff_eq!(
            ca.snap(),
            ra.snap(),
            "e={elemsize} op#{i} grow(addlen={addlen}, min_cap={min_cap})"
        );
    }
    ca.free();
    ra.free();
}

// ---------------------------------------------------------------------------
// C18 — fresh allocation across elemsize x min_cap
// ---------------------------------------------------------------------------
#[test]
fn cfg_c18_arrgrowf_fresh_matrix() {
    let p = libs();
    for elemsize in [0usize, 1, 2, 4, 8, 20, 64, 128] {
        for min_cap in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 100, 1000] {
            both_grow(&p, elemsize, &[(0, min_cap)]);
        }
    }
}

// ---------------------------------------------------------------------------
// C19 — the `< 4` clamp, driven by addlen with min_cap == 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_c19_arrgrowf_min4_clamp() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 20] {
        for addlen in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 1000] {
            both_grow(&p, elemsize, &[(addlen, 0)]);
        }
    }
}

// ---------------------------------------------------------------------------
// C20 — no-op early return (min_cap <= cap)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c20_arrgrowf_noop() {
    let p = libs();
    for elemsize in [1usize, 4, 8] {
        let mut ca = Arr::new(&p.c, elemsize);
        let mut ra = Arr::new(&p.r, elemsize);
        ca.grow(0, 16);
        ra.grow(0, 16);
        diff_eq!(ca.snap(), ra.snap(), "e={elemsize} initial cap 16");
        let (cbefore, rbefore) = (ca.a, ra.a);
        for min_cap in [0usize, 1, 8, 15, 16] {
            ca.grow(0, min_cap);
            ra.grow(0, min_cap);
            assert_eq!(ca.a, cbefore, "C must return the same pointer (no-op)");
            assert_eq!(ra.a, rbefore, "Rust must return the same pointer (no-op)");
            diff_eq!(ca.snap(), ra.snap(), "e={elemsize} noop min_cap={min_cap}");
        }
        // addlen that still fits
        for addlen in [0usize, 1, 8, 16] {
            ca.grow(addlen, 0);
            ra.grow(addlen, 0);
            diff_eq!(ca.snap(), ra.snap(), "e={elemsize} noop addlen={addlen}");
        }
        ca.free();
        ra.free();
    }
}

// ---------------------------------------------------------------------------
// C21 / C22 — doubling clamp vs. explicit min_cap
// ---------------------------------------------------------------------------
#[test]
fn cfg_c21_c22_arrgrowf_doubling() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 20] {
        // start at cap 4, then ask for 5..40 each time from a fresh array
        for want in 5..=40usize {
            both_grow(&p, elemsize, &[(0, 4), (0, want)]);
        }
        // chain of doublings
        both_grow(
            &p,
            elemsize,
            &[(0, 4), (0, 5), (0, 9), (0, 17), (0, 33), (0, 65), (0, 1000), (0, 1001)],
        );
    }
}

// ---------------------------------------------------------------------------
// C23 — 200 randomized (addlen, min_cap) sequences on one live array
// ---------------------------------------------------------------------------
#[test]
fn cfg_c23_arrgrowf_random_sequences() {
    let p = libs();
    let mut rng = Rng::new(23);
    for elemsize in [1usize, 4, 8, 20] {
        for _ in 0..50 {
            let n = rng.range(1, 12);
            let ops: Vec<(usize, usize)> = (0..n)
                .map(|_| (rng.below(40), rng.below(200)))
                .collect();
            both_grow(&p, elemsize, &ops);
        }
    }
    // longer single-array walk with element writes so the payload is compared too
    for elemsize in [4usize, 8] {
        let mut ca = Arr::new(&p.c, elemsize);
        let mut ra = Arr::new(&p.r, elemsize);
        for step in 0..200 {
            match rng.below(4) {
                0 => {
                    let n = rng.below(20);
                    ca.setcap(n);
                    ra.setcap(n);
                }
                1 => {
                    let n = rng.below(20);
                    ca.setlen(n);
                    ra.setlen(n);
                    // fill the newly visible region deterministically
                    let val: Vec<u8> = (0..elemsize).map(|k| (step + k) as u8).collect();
                    for i in 0..n {
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                val.as_ptr(),
                                (ca.a as *mut u8).add(i * elemsize),
                                elemsize,
                            );
                            core::ptr::copy_nonoverlapping(
                                val.as_ptr(),
                                (ra.a as *mut u8).add(i * elemsize),
                                elemsize,
                            );
                        }
                    }
                }
                2 => {
                    let v: Vec<u8> = (0..elemsize).map(|_| rng.next_u32() as u8).collect();
                    ca.put(&v);
                    ra.put(&v);
                }
                _ => {
                    if ca.len() > 0 {
                        diff_eq!(ca.pop(), ra.pop(), "pop at step {step}");
                    }
                }
            }
            diff_eq!(ca.snap(), ra.snap(), "e={elemsize} random walk step {step}");
        }
        ca.free();
        ra.free();
    }
}

// ---------------------------------------------------------------------------
// C24 — arrput protocol, observing every capacity step
// ---------------------------------------------------------------------------
#[test]
fn cfg_c24_arrput_capacity_steps() {
    let p = libs();
    for n in [0usize, 1, 2, 3, 4, 5, 8, 9, 16, 17, 1000] {
        let mut ca = Arr::new(&p.c, 4);
        let mut ra = Arr::new(&p.r, 4);
        for i in 0..n {
            let v = (i as u32).to_ne_bytes();
            ca.put(&v);
            ra.put(&v);
            diff_eq!(ca.snap(), ra.snap(), "arrput n={n} i={i}");
        }
        diff_eq!(ca.snap(), ra.snap(), "arrput final n={n}");
        ca.free();
        ra.free();
    }
}

// ---------------------------------------------------------------------------
// C25 — 300 randomized mixed macro operations
// ---------------------------------------------------------------------------
#[test]
fn cfg_c25_arr_macro_fuzz() {
    let p = libs();
    let mut rng = Rng::new(25);
    for round in 0..8 {
        let elemsize = 4usize;
        let mut ca = Arr::new(&p.c, elemsize);
        let mut ra = Arr::new(&p.r, elemsize);
        for step in 0..300 {
            let len = ca.len() as usize;
            let op = rng.below(10);
            match op {
                0 | 1 | 2 => {
                    let v = (rng.next_u32()).to_ne_bytes();
                    ca.put(&v);
                    ra.put(&v);
                }
                3 => {
                    if len > 0 {
                        diff_eq!(ca.pop(), ra.pop(), "r{round} s{step} pop");
                    }
                }
                4 => {
                    if len > 0 {
                        let i = rng.below(len);
                        ca.deln(i, 1);
                        ra.deln(i, 1);
                    }
                }
                5 => {
                    if len > 1 {
                        let i = rng.below(len);
                        let n = rng.range(1, len - i);
                        ca.deln(i, n);
                        ra.deln(i, n);
                    }
                }
                6 => {
                    if len > 0 {
                        let i = rng.below(len);
                        ca.delswap(i);
                        ra.delswap(i);
                    }
                }
                7 => {
                    let i = rng.below(len + 1);
                    let v = (rng.next_u32()).to_ne_bytes();
                    ca.ins(i, &v);
                    ra.ins(i, &v);
                }
                8 => {
                    let i = rng.below(len + 1);
                    let n = rng.below(5);
                    ca.insn(i, n);
                    ra.insn(i, n);
                    // insn leaves the gap uninitialised in both libs; fill it so
                    // the byte-for-byte comparison stays meaningful.
                    for k in 0..n {
                        let v = (rng.next_u32()).to_ne_bytes();
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                v.as_ptr(),
                                (ca.a as *mut u8).add((i + k) * elemsize),
                                elemsize,
                            );
                            core::ptr::copy_nonoverlapping(
                                v.as_ptr(),
                                (ra.a as *mut u8).add((i + k) * elemsize),
                                elemsize,
                            );
                        }
                    }
                }
                _ => {
                    let n = rng.below(6);
                    let ci = ca.addn_index(n);
                    let ri = ra.addn_index(n);
                    diff_eq!(ci, ri, "r{round} s{step} addn_index({n})");
                    for k in 0..n {
                        let v = (rng.next_u32()).to_ne_bytes();
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                v.as_ptr(),
                                (ca.a as *mut u8).add((ci as usize + k) * elemsize),
                                elemsize,
                            );
                            core::ptr::copy_nonoverlapping(
                                v.as_ptr(),
                                (ra.a as *mut u8).add((ri as usize + k) * elemsize),
                                elemsize,
                            );
                        }
                    }
                }
            }
            diff_eq!(ca.snap(), ra.snap(), "r{round} s{step} op={op}");
        }
        ca.free();
        ra.free();
    }
}

// ---------------------------------------------------------------------------
// C26 — every element size
// ---------------------------------------------------------------------------
#[test]
fn cfg_c26_arrput_elem_sizes() {
    let p = libs();
    let mut rng = Rng::new(26);
    for elemsize in [1usize, 2, 3, 4, 5, 8, 12, 16, 20, 64] {
        let mut ca = Arr::new(&p.c, elemsize);
        let mut ra = Arr::new(&p.r, elemsize);
        for i in 0..128 {
            let v: Vec<u8> = (0..elemsize).map(|_| rng.next_u32() as u8).collect();
            ca.put(&v);
            ra.put(&v);
            diff_eq!(ca.snap(), ra.snap(), "e={elemsize} i={i}");
        }
        ca.free();
        ra.free();
    }
}

// ---------------------------------------------------------------------------
// C27 — arrfreef on a live array
// ---------------------------------------------------------------------------
#[test]
fn cfg_c27_arrfreef_live() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 20] {
        for n in [0usize, 1, 5, 100] {
            let mut ca = Arr::new(&p.c, elemsize);
            let mut ra = Arr::new(&p.r, elemsize);
            for i in 0..n {
                let v: Vec<u8> = (0..elemsize).map(|k| (i + k) as u8).collect();
                ca.put(&v);
                ra.put(&v);
            }
            // grow-then-free, repeatedly, to shake out double-free / bad-header bugs
            ca.free();
            ra.free();
            ca.grow(0, 8);
            ra.grow(0, 8);
            diff_eq!(ca.snap(), ra.snap(), "e={elemsize} n={n} after refill");
            ca.free();
            ra.free();
            assert!(ca.a.is_null() && ra.a.is_null());
        }
    }
}
