//! Phase B rows C14..C19 -- `stbds_arrgrowf` / `stbds_arrfreef`.

mod common;
use common::*;
use std::ffi::c_void;

/// One differential `stbds_arrgrowf` call on a *fresh* (NULL) array.
/// Returns the two arrays so the caller can keep growing them.
#[track_caller]
fn grow_fresh(c: &Api, rs: &Api, elemsize: usize, addlen: usize, min_cap: usize) -> (*mut c_void, *mut c_void) {
    unsafe {
        let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
        let b = (rs.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
        assert_same("arrgrowf(NULL) null-ness", &a.is_null(), &b.is_null());
        if !a.is_null() {
            let sa = snap_arr(a, 0); // elems are uninitialised realloc memory here
            let sb = snap_arr(b, 0);
            assert_same(
                &format!("arrgrowf(NULL,{elemsize},{addlen},{min_cap}) header"),
                &sa,
                &sb,
            );
        }
        (a, b)
    }
}

#[track_caller]
fn grow(c: &Api, rs: &Api, a: &mut *mut c_void, b: &mut *mut c_void, elemsize: usize, addlen: usize, min_cap: usize) {
    unsafe {
        let (oa, ob) = (*a, *b);
        // `stbds_arrgrowf` returns its argument unchanged iff the requested
        // capacity is already satisfied (L286).  Otherwise it calls `realloc`,
        // whose return identity is allocator-dependent and therefore NOT part
        // of the observable contract -- so only the early-return case is
        // asserted to preserve the pointer.
        let early_return = |p: *mut c_void| -> bool {
            let (len, cap) = if p.is_null() {
                (0usize, 0usize)
            } else {
                ((*header_of(p)).length, (*header_of(p)).capacity)
            };
            let eff = std::cmp::max(min_cap, len.wrapping_add(addlen));
            eff <= cap
        };
        let ea = early_return(oa);
        let eb = early_return(ob);
        assert_same("arrgrowf early-return decision", &ea, &eb);
        let na = (c.arrgrowf)(oa, elemsize, addlen, min_cap);
        let nb = (rs.arrgrowf)(ob, elemsize, addlen, min_cap);
        assert_same("arrgrowf null-ness", &na.is_null(), &nb.is_null());
        if ea {
            assert_eq!(na as usize, oa as usize, "C must return input unchanged");
            assert_eq!(nb as usize, ob as usize, "RUST must return input unchanged");
        }
        *a = na;
        *b = nb;
        if !na.is_null() {
            assert_same(
                &format!("arrgrowf({elemsize},{addlen},{min_cap}) state"),
                &snap_arr(na, elemsize),
                &snap_arr(nb, elemsize),
            );
        }
    }
}

/// Emulates `stbds_arrput(a, v)` (grow-if-needed, then store at `length++`).
#[track_caller]
fn arrput(c: &Api, rs: &Api, a: &mut *mut c_void, b: &mut *mut c_void, elemsize: usize, payload: &[u8]) {
    unsafe {
        let need_grow = |p: *mut c_void| {
            p.is_null() || (*header_of(p)).length + 1 > (*header_of(p)).capacity
        };
        let ga = need_grow(*a);
        let gb = need_grow(*b);
        assert_same("arrmaybegrow decision", &ga, &gb);
        if ga {
            grow(c, rs, a, b, elemsize, 1, 0);
        }
        for (p, _) in [(*a, 0), (*b, 1)] {
            let h = header_of(p);
            let idx = (*h).length;
            if elemsize > 0 {
                let dst = (p as *mut u8).add(idx * elemsize);
                for k in 0..elemsize {
                    *dst.add(k) = payload[k % payload.len()];
                }
            }
            (*h).length = idx + 1;
        }
        assert_same("after arrput", &snap_arr(*a, elemsize), &snap_arr(*b, elemsize));
    }
}

fn free_pair(c: &Api, rs: &Api, a: *mut c_void, b: *mut c_void) {
    unsafe {
        if !a.is_null() {
            (c.arrfreef)(a);
        }
        if !b.is_null() {
            (rs.arrfreef)(b);
        }
    }
}

// --- C14 --------------------------------------------------------------------
#[test]
fn cfg_c14_arrgrowf_fresh_clamp() {
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [1usize, 2, 3, 4, 8, 12, 16, 40, 64] {
            for min_cap in 1..=3usize {
                let (a, b) = grow_fresh(c, rs, elemsize, 0, min_cap);
                assert_eq!((*header_of(a)).capacity, 4, "min_cap must clamp up to 4");
                assert_eq!((*header_of(a)).length, 0);
                assert_eq!((*header_of(a)).temp, 0);
                assert!((*header_of(a)).hash_table.is_null());
                free_pair(c, rs, a, b);
            }
        }
    });
}

// --- C15 --------------------------------------------------------------------
#[test]
fn cfg_c15_arrgrowf_fresh_matrix() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(15);
        let sizes = [0usize, 1, 2, 3, 4, 5, 7, 8, 12, 16, 24, 40, 64, 100];
        for &elemsize in &sizes {
            // exhaustive small grid
            for addlen in 0..=17usize {
                for min_cap in 0..=17usize {
                    let (a, b) = grow_fresh(c, rs, elemsize, addlen, min_cap);
                    free_pair(c, rs, a, b);
                }
            }
            // randomized larger requests
            for _ in 0..200 {
                let addlen = rng.below(4096);
                let min_cap = rng.below(4096);
                let (a, b) = grow_fresh(c, rs, elemsize, addlen, min_cap);
                free_pair(c, rs, a, b);
            }
        }
    });
}

// --- C16 --------------------------------------------------------------------
#[test]
fn cfg_c16_arrgrowf_double() {
    // min_cap < 2*cap  =>  capacity doubles
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [1usize, 4, 8, 16, 40] {
            let (mut a, mut b) = grow_fresh(c, rs, elemsize, 0, 4);
            assert_eq!((*header_of(a)).capacity, 4);
            for step in 0..12 {
                let cap = (*header_of(a)).capacity;
                grow(c, rs, &mut a, &mut b, elemsize, 0, cap + 1);
                assert_eq!(
                    (*header_of(a)).capacity,
                    2 * cap,
                    "doubling path failed at step {step}"
                );
            }
            free_pair(c, rs, a, b);
        }
    });
}

// --- C17 --------------------------------------------------------------------
#[test]
fn cfg_c17_arrgrowf_exact() {
    // min_cap >= 2*cap  =>  capacity == min_cap (already >= 4)
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(17);
        for elemsize in [1usize, 4, 8, 16, 40] {
            for _ in 0..80 {
                let (mut a, mut b) = grow_fresh(c, rs, elemsize, 0, 4);
                let target = 8 + rng.below(2000);
                grow(c, rs, &mut a, &mut b, elemsize, 0, target);
                assert_eq!((*header_of(a)).capacity, target);
                free_pair(c, rs, a, b);
            }
        }
    });
}

// --- C18 --------------------------------------------------------------------
#[test]
fn cfg_c18_arrgrowf_minlen_wins() {
    // min_len = arrlen + addlen > min_cap  =>  min_cap = min_len
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(18);
        for elemsize in [1usize, 8, 16] {
            for _ in 0..120 {
                let (mut a, mut b) = grow_fresh(c, rs, elemsize, 0, 4);
                // push some elements so arrlen > 0
                let n = 1 + rng.below(4);
                for i in 0..n {
                    arrput(c, rs, &mut a, &mut b, elemsize, &[(i as u8).wrapping_mul(7) | 1]);
                }
                let len = (*header_of(a)).length;
                let addlen = 1 + rng.below(500);
                grow(c, rs, &mut a, &mut b, elemsize, addlen, 0);
                let want = len + addlen;
                let cap = (*header_of(a)).capacity;
                assert!(cap >= want, "cap {cap} < min_len {want}");
                free_pair(c, rs, a, b);
            }
        }
    });
}

// --- C19 --------------------------------------------------------------------
#[test]
fn cfg_c19_arrgrowf_chain_then_free() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(19);
        for elemsize in [1usize, 3, 8, 16, 40] {
            let (mut a, mut b) = (std::ptr::null_mut(), std::ptr::null_mut());
            for i in 0..600usize {
                let pat = [(i & 0xff) as u8, ((i >> 8) & 0xff) as u8, 0x5A, 0xC3];
                arrput(c, rs, &mut a, &mut b, elemsize, &pat);
            }
            // header fields the library must preserve across realloc
            assert_eq!((*header_of(a)).length, 600);
            assert_eq!((*header_of(a)).temp, 0);
            assert!((*header_of(a)).hash_table.is_null());
            // interleave explicit growth requests
            for _ in 0..50 {
                let addlen = rng.below(64);
                let min_cap = rng.below(2048);
                grow(c, rs, &mut a, &mut b, elemsize, addlen, min_cap);
            }
            assert_same(
                "chain final state",
                &snap_arr(a, elemsize),
                &snap_arr(b, elemsize),
            );
            free_pair(c, rs, a, b);
        }
    });
}

#[test]
fn cfg_c19b_arrgrowf_temp_and_hashtable_preserved() {
    // A grow must not clobber `temp` / `hash_table` on a non-NULL array.
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [8usize, 16] {
            let (mut a, mut b) = grow_fresh(c, rs, elemsize, 0, 4);
            (*header_of(a)).temp = -12345;
            (*header_of(b)).temp = -12345;
            (*header_of(a)).hash_table = 0xDEAD_BEEF_usize as *mut c_void;
            (*header_of(b)).hash_table = 0xDEAD_BEEF_usize as *mut c_void;
            for _ in 0..10 {
                let cap = (*header_of(a)).capacity;
                grow(c, rs, &mut a, &mut b, elemsize, 0, cap * 4);
            }
            assert_eq!((*header_of(a)).temp, -12345);
            assert_eq!((*header_of(a)).hash_table as usize, 0xDEAD_BEEF);
            (*header_of(a)).hash_table = std::ptr::null_mut();
            (*header_of(b)).hash_table = std::ptr::null_mut();
            free_pair(c, rs, a, b);
        }
    });
}
