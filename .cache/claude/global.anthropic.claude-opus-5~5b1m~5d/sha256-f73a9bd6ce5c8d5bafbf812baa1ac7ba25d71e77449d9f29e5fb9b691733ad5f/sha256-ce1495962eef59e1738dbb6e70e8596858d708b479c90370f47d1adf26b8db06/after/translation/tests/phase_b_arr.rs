//! Phase B — CONFIGS.md rows 10-14: `stbds_arrgrowf` / `stbds_arrfreef`.

mod common;

use common::*;
use std::ffi::c_void;

unsafe fn hdr_snap(tag: &str, a: *mut c_void, same: bool, is_null: bool) -> Vec<String> {
    unsafe {
        let mut v = vec![format!("{tag} null={is_null} same_as_input={same}")];
        if !is_null {
            let h = &*header(a);
            v.push(format!("  length={}", h.length));
            v.push(format!("  capacity={}", h.capacity));
            v.push(format!("  temp={}", h.temp));
            v.push(format!("  has_table={}", !h.hash_table.is_null()));
        }
        v
    }
}

/// rows 10 + 3 — fresh growth matrix (`a == NULL`), covering `min_len > min_cap`,
/// the `min_cap < 4` clamp and the doubling clamp.
#[test]
fn growf_fresh_matrix() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        for &es in &[1usize, 4, 8, 12, 16, 64] {
            for &addlen in &[0usize, 1, 2, 7, 100] {
                for &min_cap in &[0usize, 1, 3, 4, 5, 1000] {
                    unsafe {
                        let a = (api.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                        t.push(format!("es={es} addlen={addlen} min_cap={min_cap}"));
                        t.extend(hdr_snap("fresh", a, false, a.is_null()));
                        // addlen == 0 && min_cap == 0 hits the `min_cap <= arrcap`
                        // early return and yields NULL (see ERRORS.md row 3)
                        if a.is_null() {
                            assert!(addlen == 0 && min_cap == 0);
                            continue;
                        }
                        // the whole capacity must be writable
                        let cap = (*header(a)).capacity;
                        std::ptr::write_bytes(a as *mut u8, 0xA7, cap * es);
                        let sl = std::slice::from_raw_parts(a as *const u8, cap * es);
                        t.push(format!("  wrote {} bytes, first={:?}", sl.len(), sl.first()));
                        (api.arrfreef)(a);
                    }
                }
            }
        }
    }
    assert_traces_eq("growf fresh matrix", &tc, &tr);
}

/// row 11 — request that already fits: early `return a`, identical pointer,
/// header untouched.
#[test]
fn growf_noop() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        for &es in &[1usize, 4, 8, 16] {
            unsafe {
                // capacity becomes max(min_cap, 4)
                let a = (api.arrgrowf)(std::ptr::null_mut(), es, 0, 10);
                (*header(a)).length = 3;
                (*header(a)).temp = 42;
                for &(addlen, min_cap) in
                    &[(0usize, 0usize), (0, 1), (0, 10), (1, 0), (7, 0), (7, 10), (0, 9)]
                {
                    let b = (api.arrgrowf)(a, es, addlen, min_cap);
                    t.push(format!("es={es} addlen={addlen} min_cap={min_cap}"));
                    t.extend(hdr_snap("noop", b, b == a, b.is_null()));
                    assert_eq!(b, a, "{}: expected the no-op early return", api.tag);
                }
                (api.arrfreef)(a);
            }
        }
    }
    assert_traces_eq("growf noop", &tc, &tr);
}

/// row 12 — repeated growth: doubling path vs. explicit-bigger path, driven by
/// a randomised `arraddnptr`-style usage pattern; the payload bytes are
/// compared too (realloc must preserve them).
#[test]
fn growf_repeated() {
    for seed in 0..24u64 {
        let p = seeded(DEFAULT_SEED);
        let mut tc = Vec::new();
        let mut tr = Vec::new();
        for api in p.both() {
            let t = if api.tag == "C" { &mut tc } else { &mut tr };
            let mut rng = Rng::new(seed);
            let es = [1usize, 2, 4, 8, 12, 16][(seed as usize) % 6];
            unsafe {
                let mut a: *mut c_void = std::ptr::null_mut();
                let mut len: usize = 0;
                let mut fill: u8 = 1;
                for step in 0..60 {
                    let addlen = rng.below(9) as usize;
                    let setcap = rng.below(5) == 0;
                    let prev = a;
                    let cap_before = if a.is_null() { 0 } else { (*header(a)).capacity };
                    if setcap {
                        let n = rng.below(40) as usize;
                        a = (api.arrgrowf)(a, es, 0, n);
                        // identity is only guaranteed on the early-return path
                        // (no reallocation); otherwise realloc may or may not move
                        let cap_after = if a.is_null() { 0 } else { (*header(a)).capacity };
                        let ident = if cap_after == cap_before {
                            format!("ptr_same={}", a == prev)
                        } else {
                            "ptr=grown".into()
                        };
                        t.push(format!("[{step}] setcap({n}) {ident} null={}", a.is_null()));
                    } else {
                        let need = a.is_null()
                            || (*header(a)).length + addlen > (*header(a)).capacity;
                        if need {
                            a = (api.arrgrowf)(a, es, addlen, 0);
                        }
                        let cap_after = if a.is_null() { 0 } else { (*header(a)).capacity };
                        let ident = if cap_after == cap_before {
                            format!("ptr_same={}", a == prev)
                        } else {
                            "ptr=grown".into()
                        };
                        t.push(format!(
                            "[{step}] maybegrow(addlen={addlen}) grew={need} {ident} null={}",
                            a.is_null()
                        ));
                        if !a.is_null() {
                            len += addlen;
                            (*header(a)).length = len;
                            // fill the newly added region
                            if addlen > 0 {
                                std::ptr::write_bytes(
                                    (a as *mut u8).add((len - addlen) * es),
                                    fill,
                                    addlen * es,
                                );
                                fill = fill.wrapping_add(1);
                            }
                        }
                    }
                    if a.is_null() {
                        t.push("  (still NULL)".into());
                        continue;
                    }
                    let h = &*header(a);
                    t.push(format!("  len={} cap={} temp={}", h.length, h.capacity, h.temp));
                    t.push(format!(
                        "  data={}",
                        hex(std::slice::from_raw_parts(a as *const u8, h.length * es))
                    ));
                }
                if !a.is_null() {
                    (api.arrfreef)(a);
                }
            }
        }
        assert_traces_eq(&format!("growf repeated seed={seed}"), &tc, &tr);
    }
}

/// row 13 — large allocation / wrapping arithmetic
#[test]
fn growf_large() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            for &(es, addlen, min_cap) in &[
                (1usize, 0usize, 1usize << 20),
                (1, 1 << 20, 0),
                (8, 0, 1 << 17),
                (16, 1 << 16, 1),
                (64, 0, 1 << 14),
            ] {
                let a = (api.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                let h = &*header(a);
                t.push(format!(
                    "es={es} addlen={addlen} min_cap={min_cap} -> len={} cap={}",
                    h.length, h.capacity
                ));
                // touch both ends of the block
                let n = h.capacity * es;
                *(a as *mut u8) = 0x5a;
                *((a as *mut u8).add(n - 1)) = 0xa5;
                t.push(format!("  ends={:02x},{:02x}", *(a as *const u8), *((a as *const u8).add(n - 1))));
                (api.arrfreef)(a);
            }
        }
    }
    assert_traces_eq("growf large", &tc, &tr);
}

/// row 14 — `stbds_arrfreef` round-trips (frees the header, not the payload
/// pointer)
#[test]
fn arrfreef_roundtrip() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        let mut rng = Rng::new(0x1234_5678);
        unsafe {
            for i in 0..200 {
                let es = 1 + (rng.below(17) as usize);
                let min_cap = 1 + rng.below(50) as usize; // 0/0 would return NULL
                let a = (api.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
                let h = &*header(a);
                t.push(format!("[{i}] es={es} min_cap={min_cap} cap={}", h.capacity));
                std::ptr::write_bytes(a as *mut u8, 0x11, h.capacity * es);
                (api.arrfreef)(a);
            }
        }
    }
    assert_traces_eq("arrfreef roundtrip", &tc, &tr);
}
