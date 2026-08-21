//! `CONFIGS.md` rows 22-25 and `ERRORS.md` rows 16-17 / G3-G4 — the two exported
//! data symbols, exercised exactly as a C consumer would (read, call through,
//! and *write*, since neither pointer is `const` in `mdmacros.h`).

mod common;

use common::*;
use std::ffi::{c_char, c_int};

#[test]
fn row22_g_op_initially_points_at_op_for_the_selected_op() {
    let (c, r) = libs();
    for (lbl, l) in [("C", c), ("Rust", r)] {
        let gop = l.g_op();
        let selected = l.func2(&format!("op_{OP}"));
        unsafe {
            assert_eq!(
                *gop as usize, selected as usize,
                "{lbl}: G_OP should hold op_{OP} (OP_FN(OP)) at load time"
            );
        }
    }
    // And it behaves like that operation for real inputs.
    let mut rng = Rng::with_seed(0x6607_0022);
    let mut pairs: Vec<(c_int, c_int)> = BOUNDARIES
        .iter()
        .flat_map(|a| BOUNDARIES.iter().map(move |b| (*a, *b)))
        .collect();
    for _ in 0..128 {
        pairs.push((rng.next_mixed(), rng.next_mixed()));
    }
    for (a, b) in pairs {
        let cv = unsafe { (*c.g_op())(a, b) };
        let rv = unsafe { (*r.g_op())(a, b) };
        assert_eq!(cv, rv, "G_OP({a},{b}) diverges");
        let want = match OP {
            "add" => a.wrapping_add(b),
            "sub" => a.wrapping_sub(b),
            _ => a.wrapping_mul(b),
        };
        assert_eq!(cv, want, "G_OP({a},{b}) should behave like op_{OP}");
    }
}

#[test]
fn row23_g_op_is_writable_like_c() {
    // ERRORS.md row 16. `extern int (*G_OP)(int,int);` is a non-const global, so
    // a store through it must succeed in BOTH libraries. When G_OP was an
    // immutable Rust `static` it landed in .data.rel.ro and this store faulted.
    let (c, r) = libs();
    let mut rng = Rng::with_seed(0x6607_0023);

    for target in ["op_add", "op_sub", "op_mul"] {
        for _ in 0..32 {
            let (a, b) = (rng.next_mixed(), rng.next_mixed());
            let cv = unsafe {
                let g = c.g_op();
                *g = c.func2(target);
                (*g)(a, b)
            };
            let rv = unsafe {
                let g = r.g_op();
                *g = r.func2(target);
                (*g)(a, b)
            };
            assert_eq!(cv, rv, "after G_OP = {target}, G_OP({a},{b}) diverges");
            let want = match target {
                "op_add" => a.wrapping_add(b),
                "op_sub" => a.wrapping_sub(b),
                _ => a.wrapping_mul(b),
            };
            assert_eq!(cv, want);
        }
    }

    // Restore, so ordering between tests cannot matter.
    unsafe {
        *c.g_op() = c.func2(&format!("op_{OP}"));
        *r.g_op() = r.func2(&format!("op_{OP}"));
    }
}

#[test]
fn g_op_cross_library_dispatch() {
    // ERRORS.md G3: store the *other* library's function pointer and call
    // through it. Both must simply follow the pointer.
    let (c, r) = libs();
    let saved_c = unsafe { *c.g_op() };
    let saved_r = unsafe { *r.g_op() };

    unsafe {
        *c.g_op() = r.func2("op_mul"); // C's global -> Rust's op_mul
        *r.g_op() = c.func2("op_mul"); // Rust's global -> C's op_mul
        let mut rng = Rng::with_seed(0x6607_0033);
        for _ in 0..64 {
            let (a, b) = (rng.next_mixed(), rng.next_mixed());
            assert_eq!((*c.g_op())(a, b), (*r.g_op())(a, b));
            assert_eq!((*c.g_op())(a, b), a.wrapping_mul(b));
        }
        *c.g_op() = saved_c;
        *r.g_op() = saved_r;
    }
}

#[test]
fn row24_g_op_name_matches_c() {
    // ERRORS.md G4: STR(OP) -> "add" / "sub" / "mul", byte-compared with NUL.
    let (c, r) = libs();
    let cs = c.g_op_name_str();
    let rs = r.g_op_name_str();
    assert_eq!(show(&cs), show(&rs), "G_OP_NAME text diverges");
    assert_eq!(show(&cs), OP, "G_OP_NAME should be STR(OP)");
    assert_eq!(cs.len(), 3, "all three op names are 3 bytes");
    // Verify the NUL terminator is really there in both.
    for (lbl, l) in [("C", c), ("Rust", r)] {
        unsafe {
            let p = *l.g_op_name();
            assert_eq!(*p.offset(3) as u8, 0, "{lbl}: G_OP_NAME not NUL-terminated");
        }
    }
}

#[test]
fn row25_g_op_name_is_writable_like_c() {
    // ERRORS.md row 17: the `const` in `const char *G_OP_NAME` qualifies the
    // characters, not the pointer, so assigning a new pointer must work.
    let (c, r) = libs();
    let saved_c = unsafe { *c.g_op_name() };
    let saved_r = unsafe { *r.g_op_name() };

    let replacement = b"zzz\0";
    unsafe {
        *c.g_op_name() = replacement.as_ptr() as *const c_char;
        *r.g_op_name() = replacement.as_ptr() as *const c_char;
    }
    assert_eq!(show(&c.g_op_name_str()), "zzz");
    assert_eq!(show(&r.g_op_name_str()), "zzz");

    unsafe {
        *c.g_op_name() = saved_c;
        *r.g_op_name() = saved_r;
    }
    assert_eq!(show(&c.g_op_name_str()), OP);
    assert_eq!(show(&r.g_op_name_str()), OP);
}

#[test]
fn helper_ptr_does_not_read_g_op() {
    // helper_ptr copies OP_FN(OP) into a local (`int (*fp)(int,int) = op_<OP>;`),
    // so overwriting G_OP must NOT change what helper_ptr does. This pins a
    // plausible mistranslation (routing helper_ptr through the global).
    let (c, r) = libs();
    let saved_c = unsafe { *c.g_op() };
    let saved_r = unsafe { *r.g_op() };
    unsafe {
        // point the globals at an operation that differs from OP
        let other = if OP == "sub" { "op_add" } else { "op_sub" };
        *c.g_op() = c.func2(other);
        *r.g_op() = r.func2(other);
    }

    for (a, b) in [(9, 4), (-3, 8), (c_int::MAX, 1), (c_int::MIN, -1)] {
        let got = diff2("helper_ptr", a, b);
        let want = match OP {
            "add" => a.wrapping_add(b),
            "sub" => a.wrapping_sub(b),
            _ => a.wrapping_mul(b),
        };
        assert_eq!(got, want, "helper_ptr must ignore G_OP");
    }

    unsafe {
        *c.g_op() = saved_c;
        *r.g_op() = saved_r;
    }
}
