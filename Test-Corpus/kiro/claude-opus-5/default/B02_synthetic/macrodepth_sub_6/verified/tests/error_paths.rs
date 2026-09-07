//! Phase C -- error-path differential tests, one per `ERRORS.md` row.
//!
//! `ERRORS.md` rows 1-9 concern the driver's argv/`atoi` surface and live in
//! `binary_diff.rs`; rows 10-20 are library-level and live here. Everything goes
//! through `.so` exports.

mod harness;

use core::ffi::c_int;
use harness::{capture_stdout, Pair, Rng, Side, GRID};

/// Compares `use_generated` (return value and printed bytes) over a set of `n`.
fn ug(inputs: &[c_int], tag: &str) {
    let p = Pair::load();
    let (rc, bc) = {
        let f = p.fn1(Side::C, "use_generated");
        let mut v = Vec::new();
        let b = capture_stdout(&format!("c-err-{tag}"), || {
            for &n in inputs {
                v.push(f(n));
            }
        });
        (v, b)
    };
    let (rr, br) = {
        let f = p.fn1(Side::Rust, "use_generated");
        let mut v = Vec::new();
        let b = capture_stdout(&format!("rs-err-{tag}"), || {
            for &n in inputs {
                v.push(f(n));
            }
        });
        (v, b)
    };
    for (i, &n) in inputs.iter().enumerate() {
        assert_eq!(
            rc[i], rr[i],
            "use_generated({n}) diverged (C={} Rust={})",
            rc[i], rr[i]
        );
    }
    assert_eq!(bc, br, "use_generated stdout diverged for {tag}");
    // Every one of these `n` is outside the switch label set 0..=6, so C must
    // return INIT_FOR(OP) untouched. Assert the sentinel explicitly rather than
    // settling for "both did the same thing".
    let init = p.cfg.init();
    for (i, &n) in inputs.iter().enumerate() {
        if !(0..=6).contains(&n) {
            assert_eq!(
                rc[i], init,
                "C ground truth: use_generated({n}) should be INIT_FOR(OP)={init}"
            );
            assert_eq!(rr[i], init, "Rust: use_generated({n}) should be {init}");
        }
    }
}

/// Row 10: `n == 7` -- `REP7` exists but `DISPATCH_REP` has no `case 7`.
#[test]
fn err_row10_use_generated_seven() {
    ug(&[7], "n7");
}

/// Row 11: negative `n`.
#[test]
fn err_row11_use_generated_negative() {
    ug(&[-1, -2, -7, -100, c_int::MIN + 1, c_int::MIN], "negative");
}

/// Row 12: `n` far above the range.
#[test]
fn err_row12_use_generated_above_range() {
    ug(&[8, 9, 100, 1000, c_int::MAX - 1, c_int::MAX], "above");
}

/// Row 20: `n` is a plain `int` switch selector, so *every* 32-bit value is a
/// real input crossing the FFI boundary -- the out-of-range-"enum" class.
/// Randomized sweep biased towards the range edges.
#[test]
fn err_row20_use_generated_out_of_range_selector() {
    let mut rng = Rng::new();
    let mut v: Vec<c_int> = Vec::new();
    for _ in 0..2048 {
        v.push(rng.next_i32());
    }
    // dense coverage of the two edges
    v.extend(-4..=11);
    // powers of two and their neighbours, both signs
    for s in 0..31 {
        let x = 1i32 << s;
        v.extend([x, x.wrapping_neg(), x - 1, x + 1]);
    }
    ug(&v, "selector");
}

/// Rows 13-15: signed overflow in the leaf ops must wrap, not trap or panic.
/// The exact wrapped value is compared, and the C result is additionally
/// checked against two's-complement truncation.
#[test]
fn err_rows13_15_leaf_overflow() {
    let p = Pair::load();
    let cases: &[(&str, fn(i64, i64) -> i64)] = &[
        ("op_add", |a, b| a + b),
        ("op_sub", |a, b| a - b),
        ("op_mul", |a, b| a * b),
    ];
    for &(name, wide) in cases {
        let c = p.fn2(Side::C, name);
        let r = p.fn2(Side::Rust, name);
        for &a in GRID.iter() {
            for &b in GRID.iter() {
                let cv = c(a, b);
                let rv = r(a, b);
                assert_eq!(cv, rv, "{name}({a}, {b}) diverged");
                let expect = wide(a as i64, b as i64) as u64 as u32 as i32;
                assert_eq!(
                    cv, expect,
                    "{name}({a}, {b}): C={cv} is not the two's-complement truncation {expect}"
                );
            }
        }
    }
}

/// Row 16: overflow in `helper_call`'s `r + acc`.
#[test]
fn err_row16_helper_call_sum_overflow() {
    let p = Pair::load();
    let c = p.fn2(Side::C, "helper_call");
    let r = p.fn2(Side::Rust, "helper_call");

    let mut inputs: Vec<(c_int, c_int)> = Vec::new();
    for &a in GRID.iter() {
        for &b in GRID.iter() {
            inputs.push((a, b));
        }
    }
    // Values chosen so that r lands exactly on the boundary and acc pushes it over.
    for d in -8..=8 {
        inputs.push((c_int::MAX, d));
        inputs.push((c_int::MIN, d));
        inputs.push((d, c_int::MAX));
        inputs.push((d, c_int::MIN));
    }

    let mut cv = Vec::new();
    let bc = capture_stdout("c-hc-ovf", || {
        for &(a, b) in &inputs {
            cv.push(c(a, b));
        }
    });
    let mut rv = Vec::new();
    let br = capture_stdout("rs-hc-ovf", || {
        for &(a, b) in &inputs {
            rv.push(r(a, b));
        }
    });
    for (i, &(a, b)) in inputs.iter().enumerate() {
        assert_eq!(cv[i], rv[i], "helper_call({a}, {b}) diverged");
    }
    assert_eq!(bc, br, "helper_call stdout diverged on overflow inputs");
}

/// Row 17: there is no pointer parameter anywhere in the API -- every `int` bit
/// pattern is valid input and neither side may reject it. Sweeps the whole
/// public `int, int` surface with adversarial values.
#[test]
fn err_row17_no_pointer_surface() {
    let p = Pair::load();
    let mut rng = Rng::new();
    let mut inputs: Vec<(c_int, c_int)> = vec![
        (0, 0),
        (c_int::MIN, c_int::MIN),
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, c_int::MAX),
        (-1, -1),
    ];
    for _ in 0..256 {
        inputs.push((rng.next_i32(), rng.next_i32()));
    }
    for name in ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr"] {
        let c = p.fn2(Side::C, name);
        let r = p.fn2(Side::Rust, name);
        let mut cv = Vec::new();
        let bc = capture_stdout(&format!("c-nop-{name}"), || {
            for &(a, b) in &inputs {
                cv.push(c(a, b));
            }
        });
        let mut rv = Vec::new();
        let br = capture_stdout(&format!("rs-nop-{name}"), || {
            for &(a, b) in &inputs {
                rv.push(r(a, b));
            }
        });
        assert_eq!(cv, rv, "{name} diverged over the adversarial int sweep");
        assert_eq!(bc, br, "{name} stdout diverged");
    }
}

/// Row 18: `G_OP` is a *writable* global in C. Overwriting it from the caller
/// must not change anything the library itself computes, because `helper_call`
/// and `helper_ptr` both use `OP_FN(OP)` directly (`mdcore.c:40`, `:48`) rather
/// than reading the global. Verified identically on both sides, then restored.
#[test]
fn err_row18_g_op_is_writable_and_unused_internally() {
    let p = Pair::load();

    for side in [Side::C, Side::Rust] {
        let before_hc;
        let before_hp;
        {
            let hc = p.fn2(side, "helper_call");
            let hp = p.fn2(side, "helper_ptr");
            before_hc = capture_ret(&hc, 7, 3);
            before_hp = capture_ret(&hp, 7, 3);
        }

        // Point G_OP at a different leaf.
        let other: harness::Op2 = *p.fn2(side, "op_mul");
        let saved = p.g_op(side);
        // SAFETY: `G_OP` is a non-const `int (*)(int,int)` global in C and a
        // `static mut` of the same layout in Rust; the test is single-threaded
        // and restores the original value below.
        unsafe {
            let sym: libloading::Symbol<'_, *mut harness::Op2> = match side {
                Side::C => p.c.get(b"G_OP\0").unwrap(),
                Side::Rust => p.rust.get(b"G_OP\0").unwrap(),
            };
            **sym = other;
        }

        let after_g = p.g_op(side);
        assert_eq!(after_g(6, 7), 42, "{side:?}: G_OP write did not take effect");

        let hc = p.fn2(side, "helper_call");
        let hp = p.fn2(side, "helper_ptr");
        assert_eq!(
            capture_ret(&hc, 7, 3),
            before_hc,
            "{side:?}: helper_call must not read G_OP"
        );
        assert_eq!(
            capture_ret(&hp, 7, 3),
            before_hp,
            "{side:?}: helper_ptr must not read G_OP"
        );

        // SAFETY: as above -- restore so later assertions see the original.
        unsafe {
            let sym: libloading::Symbol<'_, *mut harness::Op2> = match side {
                Side::C => p.c.get(b"G_OP\0").unwrap(),
                Side::Rust => p.rust.get(b"G_OP\0").unwrap(),
            };
            **sym = saved;
        }
    }
}

fn capture_ret(f: &harness::Op2, a: c_int, b: c_int) -> c_int {
    let mut out = 0;
    let _ = capture_stdout("ret", || out = f(a, b));
    out
}

/// Row 19: `G_OP_NAME` is never NULL and always a 3-byte NUL-terminated name.
#[test]
fn err_row19_g_op_name_never_null() {
    let p = Pair::load();
    let c = p.g_op_name(Side::C);
    let r = p.g_op_name(Side::Rust);
    assert_eq!(c.len(), 3, "C G_OP_NAME length");
    assert_eq!(r.len(), 3, "Rust G_OP_NAME length");
    assert_eq!(c, r);
}

/// Row 21: both data exports must live in a *writable* section.
///
/// `const char *G_OP_NAME` qualifies the pointee, not the pointer, so in C the
/// global itself is mutable and gcc emits it into `.data`. An external caller may
/// therefore store through the symbol. If the Rust side declared it as an
/// immutable `static`, the address-bearing initializer would place it in
/// `.data.rel.ro`, which the loader marks read-only under RELRO, and this test
/// would die with SIGSEGV instead of failing an assertion.
///
/// The pointee is a string literal in `.rodata` on both sides, so writing
/// *through* the pointer faults in C too and is deliberately not attempted.
#[test]
fn err_row21_data_exports_are_writable() {
    let p = Pair::load();

    for side in [Side::C, Side::Rust] {
        // --- G_OP_NAME ---
        let original = p.g_op_name(side);
        let replacement = b"zzz\0";
        // SAFETY: `G_OP_NAME` is one `const char *` slot in a writable section on
        // both sides; `replacement` is a 'static NUL-terminated literal, so the
        // pointer stays valid, and the original value is restored below.
        unsafe {
            let sym: libloading::Symbol<'_, *mut *const core::ffi::c_char> = match side {
                Side::C => p.c.get(b"G_OP_NAME\0").unwrap(),
                Side::Rust => p.rust.get(b"G_OP_NAME\0").unwrap(),
            };
            let saved = **sym;
            **sym = replacement.as_ptr() as *const core::ffi::c_char;
            assert_eq!(
                p.g_op_name(side),
                b"zzz".to_vec(),
                "{side:?}: store to G_OP_NAME did not take effect"
            );
            **sym = saved;
        }
        assert_eq!(
            p.g_op_name(side),
            original,
            "{side:?}: G_OP_NAME restore failed"
        );

        // --- G_OP (already covered behaviourally by row 18; here just the store) ---
        let saved_op = p.g_op(side);
        // SAFETY: as above -- `G_OP` is a writable function-pointer global and the
        // original value is put back immediately.
        unsafe {
            let sym: libloading::Symbol<'_, *mut harness::Op2> = match side {
                Side::C => p.c.get(b"G_OP\0").unwrap(),
                Side::Rust => p.rust.get(b"G_OP\0").unwrap(),
            };
            let other: harness::Op2 = *p.fn2(side, "op_sub");
            **sym = other;
            assert_eq!(p.g_op(side)(10, 4), 6, "{side:?}: store to G_OP failed");
            **sym = saved_op;
        }
    }
}

/// Row 22: storing *through* `G_OP_NAME`, into the name string itself.
///
/// In C the target is the `STR(OP)` string literal, which gcc puts in `.rodata`;
/// the Rust side points at an immutable `static [u8; 4]`, likewise `.rodata`. The
/// store must therefore be fatal on *both* sides with the *same* signal. This
/// cannot be observed in-process, so each attempt runs in a forked child and the
/// termination status is compared.
#[test]
fn err_row22_name_storage_is_read_only_on_both_sides() {
    let p = Pair::load();

    let probe = |side: Side| {
        // Read the pointer in the parent so the child does nothing but store.
        let addr: usize = {
            let lib = match side {
                Side::C => &p.c,
                Side::Rust => &p.rust,
            };
            // SAFETY: `G_OP_NAME` is one `const char *` slot.
            let sym: libloading::Symbol<'_, *mut *const core::ffi::c_char> =
                unsafe { lib.get(b"G_OP_NAME\0") }.unwrap();
            unsafe { **sym as usize }
        };
        harness::probe_in_child(move || {
            // SAFETY: deliberately invalid -- this is the behaviour under test.
            // It runs only in a forked child whose death is the expected result.
            unsafe {
                core::ptr::write_volatile(
                    addr as *mut core::ffi::c_char,
                    b'X' as core::ffi::c_char,
                )
            };
        })
    };

    let c_outcome = probe(Side::C);
    let rust_outcome = probe(Side::Rust);
    assert_eq!(
        c_outcome, rust_outcome,
        "storing through G_OP_NAME must terminate identically: \
         C={c_outcome:?} Rust={rust_outcome:?}"
    );
    assert_eq!(
        c_outcome,
        harness::Probe::Signaled(11),
        "the name string is expected to be read-only (SIGSEGV) in the C build"
    );
}
