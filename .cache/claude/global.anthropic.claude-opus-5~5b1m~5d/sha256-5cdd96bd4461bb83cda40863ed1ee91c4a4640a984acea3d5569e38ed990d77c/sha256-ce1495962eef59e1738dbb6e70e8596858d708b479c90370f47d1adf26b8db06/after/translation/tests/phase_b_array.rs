//! Phase B — dynamic-array surface: `stbds_arrgrowf`, `stbds_arrfreef`, the
//! whole `arr*` macro pipeline built on top of them, plus `arr_ins` and
//! `strkey`. Rows C09–C16 of CONFIGS.md.
mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

const ELEMSIZES: &[usize] = &[0, 1, 2, 3, 4, 7, 8, 12, 16, 24, 64];

/// Call `stbds_arrgrowf(NULL, ...)` on both libs and compare the fresh header.
fn growf_fresh(p: &Pair, elemsize: usize, addlen: usize, min_cap: usize) {
    unsafe {
        let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
        let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
        let ctx = format!("arrgrowf(NULL, {elemsize}, {addlen}, {min_cap})");
        same_val(&format!("{ctx} null-ness"), ca.is_null(), ra.is_null());
        if ca.is_null() {
            return;
        }
        let ch = header_of(ca);
        let rh = header_of(ra);
        same_val(&format!("{ctx}.length"), ch.length, rh.length);
        same_val(&format!("{ctx}.capacity"), ch.capacity, rh.capacity);
        same_val(&format!("{ctx}.temp"), ch.temp, rh.temp);
        same_val(
            &format!("{ctx}.hash_table"),
            ch.hash_table.is_null(),
            rh.hash_table.is_null(),
        );
        (p.c.arrfreef)(ca);
        (p.r.arrfreef)(ra);
    }
}

// --- C09 : fresh-allocation cross product -----------------------------------
#[test]
fn c09_arrgrowf_fresh_matrix() {
    let p = fresh_pair(1);
    for &elemsize in ELEMSIZES {
        for &addlen in &[0usize, 1, 2, 3, 4, 5, 7, 8, 63, 64, 100, 1000] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 7, 8, 100, 1000, 4096] {
                growf_fresh(&p, elemsize, addlen, min_cap);
            }
        }
    }
    // randomized
    let mut rng = Rng::new(0xC09);
    for _ in 0..3000 {
        let elemsize = ELEMSIZES[rng.below(ELEMSIZES.len())];
        let addlen = rng.below(4096);
        let min_cap = rng.below(4096);
        growf_fresh(&p, elemsize, addlen, min_cap);
    }
}

// --- C10 : repeated doubling on an existing array ---------------------------
#[test]
fn c10_arrgrowf_doubling() {
    let p = fresh_pair(2);
    let mut rng = Rng::new(0xC10);
    for &elemsize in &[1usize, 4, 8, 16, 24] {
        let mut a = DiffArr::new(&p, elemsize);
        for i in 0..400u32 {
            let v = i.to_le_bytes().repeat(8);
            a.put(&v[..elemsize.max(1)]);
            a.check(&format!("c10 elemsize={elemsize} put#{i}"));
        }
        // random addn growth
        for _ in 0..50 {
            let n = rng.below(37);
            let (ci, ri) = a.addn(n);
            same_val(&format!("c10 addn({n}) index"), ci, ri);
            a.check(&format!("c10 elemsize={elemsize} addn({n})"));
        }
        a.free();
    }
}

// --- C11 : explicit capacity (min_cap > 2*cap) ------------------------------
#[test]
fn c11_arrgrowf_explicit_cap() {
    let p = fresh_pair(3);
    let mut rng = Rng::new(0xC11);
    for &elemsize in &[1usize, 4, 8, 24] {
        let mut a = DiffArr::new(&p, elemsize);
        for _ in 0..40 {
            let want = rng.below(5000);
            a.setcap(want);
            a.check(&format!("c11 setcap({want}) elemsize={elemsize}"));
            let n = rng.below(10);
            for k in 0..n {
                a.put(&[k as u8; 64][..elemsize.max(1)]);
            }
            a.check(&format!("c11 after {n} puts"));
        }
        a.free();
    }
}

// --- C12 : no-op return + payload preserved across realloc ------------------
#[test]
fn c12_arrgrowf_noop_and_preserve() {
    let p = fresh_pair(4);
    let elemsize = 8usize;
    unsafe {
        let mut ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
        let mut ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
        // fill the payload deterministically
        for i in 0..10usize {
            let v = (i as u64).to_le_bytes();
            std::ptr::copy(v.as_ptr(), (ca as *mut u8).add(i * 8), 8);
            std::ptr::copy(v.as_ptr(), (ra as *mut u8).add(i * 8), 8);
        }
        (*((ca as usize - HEADER_SIZE) as *mut CHeader)).length = 10;
        (*((ra as usize - HEADER_SIZE) as *mut CHeader)).length = 10;

        // min_cap <= cap → identical pointer returned (no realloc)
        for min_cap in [0usize, 1, 5, 10] {
            let c2 = (p.c.arrgrowf)(ca, elemsize, 0, min_cap);
            let r2 = (p.r.arrgrowf)(ra, elemsize, 0, min_cap);
            same_val(
                &format!("c12 noop min_cap={min_cap}: same pointer"),
                c2 == ca,
                r2 == ra,
            );
            ca = c2;
            ra = r2;
            same(
                &format!("c12 noop min_cap={min_cap}"),
                &snap_array(ca, elemsize),
                &snap_array(ra, elemsize),
            );
        }
        // grow → payload must survive
        for min_cap in [11usize, 12, 25, 26, 1000] {
            ca = (p.c.arrgrowf)(ca, elemsize, 0, min_cap);
            ra = (p.r.arrgrowf)(ra, elemsize, 0, min_cap);
            same(
                &format!("c12 grow min_cap={min_cap}"),
                &snap_array(ca, elemsize),
                &snap_array(ra, elemsize),
            );
        }
        (p.c.arrfreef)(ca);
        (p.r.arrfreef)(ra);
    }
}

// --- C13 : arrfreef ----------------------------------------------------------
#[test]
fn c13_arrfreef() {
    let p = fresh_pair(5);
    for &elemsize in ELEMSIZES {
        let mut a = DiffArr::new(&p, elemsize);
        for i in 0..20u8 {
            a.put(&[i; 64][..elemsize.max(1)]);
        }
        a.check(&format!("c13 before free elemsize={elemsize}"));
        a.free();
        a.check(&format!("c13 after free elemsize={elemsize}"));
    }
}

// --- C14 : random macro pipeline --------------------------------------------
#[test]
fn c14_array_macro_pipeline() {
    let p = fresh_pair(6);
    for (round, &elemsize) in [1usize, 4, 8, 24].iter().enumerate() {
        let mut rng = Rng::new(0xC14 + round as u64);
        let mut a = DiffArr::new(&p, elemsize);
        for step in 0..400 {
            let len = unsafe { DiffArr::len(a.ca) };
            let cap = unsafe { DiffArr::cap(a.ca) };
            let choice = rng.below(9);
            let ctx = format!("c14 elemsize={elemsize} step={step} op={choice} len={len} cap={cap}");
            match choice {
                0 | 1 | 2 => {
                    let v: Vec<u8> = rng.bytes(elemsize.max(1));
                    a.put(&v);
                }
                3 => {
                    if len > 0 {
                        let (cv, rv) = a.pop();
                        same_val(&format!("{ctx} pop value"), cv, rv);
                    }
                }
                4 => {
                    let n = rng.below(9);
                    let (ci, ri) = a.addn(n);
                    same_val(&format!("{ctx} addn index"), ci, ri);
                }
                5 => {
                    let i = if len == 0 { 0 } else { rng.below(len + 1) };
                    let v: Vec<u8> = rng.bytes(elemsize.max(1));
                    a.ins(i, &v);
                }
                6 => {
                    if len > 0 {
                        let i = rng.below(len);
                        let n = 1 + rng.below(len - i);
                        a.deln(i, n);
                    }
                }
                7 => {
                    if len > 0 {
                        let i = rng.below(len);
                        a.delswap(i);
                    }
                }
                _ => {
                    let n = rng.below(64);
                    a.setlen(n);
                    // setlen may expose uninitialised tail bytes -> normalise
                    let newlen = unsafe { DiffArr::len(a.ca) };
                    if newlen > len && elemsize > 0 {
                        unsafe {
                            for x in [a.ca, a.ra] {
                                let base = x as *mut u8;
                                for b in (len * elemsize)..(newlen * elemsize) {
                                    *base.add(b) = 0xAB;
                                }
                            }
                        }
                    }
                }
            }
            a.check(&ctx);
        }
        a.free();
    }
}

// --- C15 / E61 : arr_ins ----------------------------------------------------
#[test]
fn c15_arr_ins() {
    let p = fresh_pair(7);
    let mut rng = Rng::new(0xC15);
    let mut nums: Vec<c_int> = vec![0, 1, 2, 3, 4, 5, -1, -4, c_int::MIN, c_int::MAX, 1 << 30];
    for _ in 0..500 {
        nums.push(rng.next_u32() as c_int);
    }
    unsafe {
        for n in nums {
            // arr_ins is `void`; it asserts internally. Both must survive and
            // leave the process in the same state (no crash, no abort).
            (p.c.arr_ins)(n);
            (p.r.arr_ins)(n);
        }
    }
}

#[test]
fn e61_arr_ins_all() {
    let p = fresh_pair(8);
    unsafe {
        for n in [0, 1, 4, -1, c_int::MIN, c_int::MAX] {
            (p.c.arr_ins)(n);
            (p.r.arr_ins)(n);
        }
    }
}

// --- C16 / E60 : strkey -----------------------------------------------------
#[test]
fn c16_strkey() {
    let p = fresh_pair(9);
    let mut rng = Rng::new(0xC16);
    let mut ns: Vec<c_int> = vec![
        0,
        1,
        -1,
        9,
        10,
        -9,
        -10,
        99,
        100,
        999,
        1000,
        c_int::MIN,
        c_int::MAX,
        c_int::MIN + 1,
        c_int::MAX - 1,
    ];
    for _ in 0..2000 {
        ns.push(rng.next_u32() as c_int);
    }
    unsafe {
        for n in ns {
            let cs = cstr((p.c.strkey)(n));
            let rs = cstr((p.r.strkey)(n));
            same_val(&format!("strkey({n})"), cs, rs);
        }
    }
}

#[test]
fn e60_strkey_extremes() {
    let p = fresh_pair(10);
    unsafe {
        for n in [c_int::MIN, c_int::MAX, -1, 0, -2147483647] {
            let cs = cstr((p.c.strkey)(n));
            let rs = cstr((p.r.strkey)(n));
            same_val(&format!("strkey({n})"), cs.clone(), rs);
            // and the returned pointer must be stable/static within each lib
            let again = cstr((p.c.strkey)(n));
            same_val("strkey stability", cs, again);
        }
    }
}

// --- C48 : large elemsize / min_cap -----------------------------------------
#[test]
fn c48_arrgrowf_large() {
    let p = fresh_pair(11);
    for &elemsize in &[256usize, 1024, 4096] {
        for &min_cap in &[1usize, 4, 1000, 4096] {
            growf_fresh(&p, elemsize, 0, min_cap);
            growf_fresh(&p, elemsize, min_cap, 0);
        }
    }
    // real payload round-trip at a big elemsize
    let elemsize = 1024usize;
    let mut a = DiffArr::new(&p, elemsize);
    let mut rng = Rng::new(0xC48);
    for _ in 0..40 {
        let v = rng.bytes(elemsize);
        a.put(&v);
    }
    a.check("c48 big elemsize payload");
    a.free();
}

// --- E01..E06 : arrgrowf rejection / boundary rows ---------------------------
#[test]
fn e01_arrgrowf_nogrow() {
    let p = fresh_pair(12);
    let elemsize = 4usize;
    unsafe {
        let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
        let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
        for (addlen, min_cap) in [(0usize, 0usize), (0, 1), (0, 16), (5, 5), (16, 0), (16, 16)] {
            let c2 = (p.c.arrgrowf)(ca, elemsize, addlen, min_cap);
            let r2 = (p.r.arrgrowf)(ra, elemsize, addlen, min_cap);
            same_val(
                &format!("e01 arrgrowf({addlen},{min_cap}) returns same ptr"),
                c2 == ca,
                r2 == ra,
            );
            same_val(
                &format!("e01 arrgrowf({addlen},{min_cap}) cap"),
                header_of(c2).capacity,
                header_of(r2).capacity,
            );
        }
        (p.c.arrfreef)(ca);
        (p.r.arrfreef)(ra);
    }
}

#[test]
fn e02_arrgrowf_null_a() {
    let p = fresh_pair(13);
    for &elemsize in ELEMSIZES {
        for &(addlen, min_cap) in &[(0usize, 1usize), (1, 0), (3, 2), (0, 9), (7, 3)] {
            growf_fresh(&p, elemsize, addlen, min_cap);
        }
    }
}

#[test]
fn e03_arrgrowf_zero_zero_null() {
    let p = fresh_pair(14);
    unsafe {
        for &elemsize in ELEMSIZES {
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            same_val(
                &format!("e03 arrgrowf(NULL,{elemsize},0,0) must return NULL"),
                ca.is_null(),
                ra.is_null(),
            );
            assert!(ca.is_null(), "C returns NULL for the (0,0) no-op case");
        }
    }
}

#[test]
fn e04_arrgrowf_min_cap_clamp() {
    let p = fresh_pair(15);
    unsafe {
        for (addlen, min_cap, want) in [
            (1usize, 0usize, 4usize),
            (2, 0, 4),
            (3, 0, 4),
            (4, 0, 4),
            (5, 0, 5),
            (0, 1, 4),
            (0, 3, 4),
            (0, 4, 4),
            (0, 5, 5),
        ] {
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), 4, addlen, min_cap);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), 4, addlen, min_cap);
            let cc = header_of(ca).capacity;
            let rc = header_of(ra).capacity;
            same_val(&format!("e04 cap({addlen},{min_cap})"), cc, rc);
            same_val(&format!("e04 cap({addlen},{min_cap}) == {want}"), cc, want);
            (p.c.arrfreef)(ca);
            (p.r.arrfreef)(ra);
        }
    }
}

#[test]
fn e05_arrgrowf_elemsize_zero() {
    let p = fresh_pair(16);
    unsafe {
        for min_cap in [1usize, 4, 5, 100, 1 << 20] {
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            same_val(
                &format!("e05 elemsize=0 min_cap={min_cap} cap"),
                header_of(ca).capacity,
                header_of(ra).capacity,
            );
            same_val(
                &format!("e05 elemsize=0 min_cap={min_cap} len"),
                header_of(ca).length,
                header_of(ra).length,
            );
            (p.c.arrfreef)(ca);
            (p.r.arrfreef)(ra);
        }
    }
}

#[test]
fn e06_arrgrowf_addlen_wrap() {
    let p = fresh_pair(17);
    let elemsize = 4usize;
    unsafe {
        // fresh array with len 10
        let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
        let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
        (*((ca as usize - HEADER_SIZE) as *mut CHeader)).length = 10;
        (*((ra as usize - HEADER_SIZE) as *mut CHeader)).length = 10;
        // addlen = SIZE_MAX  =>  min_len wraps to 9 (<= min_cap 10) => no-op
        for addlen in [usize::MAX, usize::MAX - 1, usize::MAX - 8] {
            let c2 = (p.c.arrgrowf)(ca, elemsize, addlen, 0);
            let r2 = (p.r.arrgrowf)(ra, elemsize, addlen, 0);
            same_val(
                &format!("e06 addlen={addlen:#x} returns same ptr"),
                c2 == ca,
                r2 == ra,
            );
            same_val(
                &format!("e06 addlen={addlen:#x} cap"),
                header_of(c2).capacity,
                header_of(r2).capacity,
            );
        }
        (p.c.arrfreef)(ca);
        (p.r.arrfreef)(ra);
    }
}

#[test]
fn e62_arrfreef_valid() {
    let p = fresh_pair(18);
    unsafe {
        for &elemsize in ELEMSIZES {
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            same_val(
                &format!("e62 pre-free header elemsize={elemsize}"),
                (header_of(ca).length, header_of(ca).capacity, header_of(ca).temp),
                (header_of(ra).length, header_of(ra).capacity, header_of(ra).temp),
            );
            (p.c.arrfreef)(ca);
            (p.r.arrfreef)(ra);
        }
    }
}

#[test]
fn b03_oversized_lengths_array() {
    let p = fresh_pair(19);
    unsafe {
        // huge min_cap with elemsize 0 -> only the header is allocated
        for min_cap in [usize::MAX / 64, usize::MAX / 2, usize::MAX] {
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            same_val(
                &format!("b03 elemsize=0 min_cap={min_cap:#x}"),
                header_of(ca).capacity,
                header_of(ra).capacity,
            );
            (p.c.arrfreef)(ca);
            (p.r.arrfreef)(ra);
        }
    }
}

// generic null-pointer boundary for the array half of the API
#[test]
fn b01_null_pointers_array() {
    let p = fresh_pair(20);
    unsafe {
        for &elemsize in ELEMSIZES {
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            same_val("b01 arrgrowf(NULL,_,0,0)", ca.is_null(), ra.is_null());
            let c2: *mut c_void = (p.c.hmput_default)(std::ptr::null_mut(), elemsize.max(1));
            let r2: *mut c_void = (p.r.hmput_default)(std::ptr::null_mut(), elemsize.max(1));
            same_val("b01 hmput_default(NULL)", c2.is_null(), r2.is_null());
            same(
                "b01 hmput_default(NULL) state",
                &snap_map(c2, elemsize.max(1), KeyRepr::Inline),
                &snap_map(r2, elemsize.max(1), KeyRepr::Inline),
            );
            (p.c.hmfree_func)((c2 as usize - elemsize.max(1)) as *mut c_void, elemsize.max(1));
            (p.r.hmfree_func)((r2 as usize - elemsize.max(1)) as *mut c_void, elemsize.max(1));
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}
