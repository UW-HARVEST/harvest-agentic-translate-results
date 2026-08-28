//! Phase C (rows 3/6/9), **isolated binary**.
//!
//! This test mutates the process address space (`mmap` with `MAP_FIXED_NOREPLACE`)
//! while calling into both shared objects. Any other test running concurrently
//! could observe a half-set-up address space, so this file deliberately contains
//! exactly ONE test: cargo gives each integration-test file its own process, and
//! a single test means no other thread is touching the mappings.

mod support;

use support::{CallOutcome, Rng, call_in_child, decode, libs};

/// For subscripts far outside `g_pow43` the C's load address is unmapped, so
/// neither object can be called without a fault and the index arithmetic is
/// invisible. This test *makes* it visible: a page is mapped at exactly the
/// address each object's subscript targets, and filled so that each 4-byte slot
/// holds a hash of its index **relative to that object's own table base**.
///
/// Both objects then see the same value at the same *relative* index, so
/// `c(x) == r(x)` holds iff they computed the same subscript — while a different
/// subscript (e.g. a logical instead of arithmetic `>>`, or `/64` truncation
/// instead of flooring) yields a different slot and a different result.
#[test]
fn deep_oob_subscript_parity_via_mapped_pages() {
    let l = libs();
    let (Some(cv), Some(rv)) = (l.c_table, l.rust_table) else {
        panic!("tables must be locatable");
    };

    // Inputs whose subscript lands far outside the table, including the
    // `x + sign` overflow window at the top of the range (rows 9) and the
    // far-below-table region (row 3).
    let mut xs: Vec<i32> = Vec::new();
    xs.extend((i32::MAX - 80)..=i32::MAX); // x + sign wraps negative
    xs.extend([
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 63,
        -2_000_000_000,
        -1_000_000_000,
        -16_000_000,
        -1_000_000,
        -100_000,
        1_000_000,
        16_000_000,
        1_000_000_000,
        2_000_000_000,
        1_073_741_824,
        2_147_483_583,
    ]);
    // Witness inputs found by exhaustive search over all 2^31 values of `x`
    // (see `scripts/equiv_check.rs`). These are the ONLY five inputs for which
    // carrying out the `frac` division in `f64` instead of `f32` — the most
    // plausible float mistranslation, since C's usual arithmetic conversions
    // make the division single-precision — changes `poly` and hence the result.
    // All five land far outside the table, so the mapped-page technique is the
    // only way to observe them at all.
    xs.extend([
        1_163_220_262,
        1_207_959_461,
        1_297_437_987,
        1_342_177_186,
        1_431_655_712,
    ]);

    let mut rng = Rng::new(0xDEEF_0000);
    xs.extend((0..400).map(|_| rng.next_i32()));

    let mut verified = 0usize;
    let mut wrapped_negative = 0usize;

    for x in xs {
        let d = decode(x);
        if (0..support::TABLE_LEN as i32).contains(&d.idx) {
            continue; // in bounds: already covered bit-for-bit elsewhere
        }
        if d.computed && x.wrapping_add(d.sign) < 0 {
            wrapped_negative += 1;
        }

        // Byte offset of the load, exactly as C computes it.
        let off = (d.idx as isize).wrapping_mul(4);
        let c_addr = cv.base.wrapping_add(off as usize);
        let r_addr = rv.base.wrapping_add(off as usize);

        // Already readable? Then the plain differential path applies.
        if support::is_readable(c_addr, 4) && support::is_readable(r_addr, 4) {
            continue;
        }

        let c_page = c_addr & !(support::PAGE - 1);
        let r_page = r_addr & !(support::PAGE - 1);
        if c_page == r_page {
            continue; // would need one page to serve both bases
        }
        let (Some(cp), Some(rp)) = (support::FixedPage::at(c_page), support::FixedPage::at(r_page))
        else {
            continue; // address space already occupied there; nothing to do
        };

        // Fill both pages so slot content depends only on the index *relative*
        // to that object's own table.
        for (page, base) in [(&cp, cv.base), (&rp, rv.base)] {
            let start = page.addr();
            for s in (0..support::PAGE).step_by(4) {
                let a = start + s;
                let rel = (a as i64 - base as i64) / 4;
                // SAFETY: `a` is inside the page we just mapped read/write.
                unsafe { std::ptr::write(a as *mut f32, support::slot_value(rel)) };
            }
        }

        // Call in a forked child (the child inherits the synthetic pages). An
        // implementation that computes a *different* subscript reads a page we
        // did not map and is reported as `Signal(SIGSEGV)` instead of taking
        // this test binary down, so the divergence surfaces as a clean assertion
        // failure rather than a crash.
        let c_out = call_in_child(l.c, x);
        let r_out = call_in_child(l.rust, x);

        // The value predicted for that exact subscript.
        let want = if d.computed {
            let poly = f32::from_bits(d.poly_bits);
            ((support::slot_value(d.idx as i64) * poly) * (d.mult as f32)).to_bits()
        } else {
            support::slot_value(d.idx as i64).to_bits()
        };

        assert_eq!(
            c_out, r_out,
            "deep out-of-bounds subscript differs at x={x}: C and Rust did not \
             read the same relative index (decoded {d:?})"
        );
        assert_eq!(
            c_out,
            CallOutcome::Returned(want),
            "C .so read an unexpected subscript at x={x} (decoded {d:?})"
        );
        assert_eq!(
            r_out,
            CallOutcome::Returned(want),
            "Rust .so read an unexpected subscript at x={x} (decoded {d:?})"
        );

        verified += 1;
        drop(cp);
        drop(rp);
    }

    assert!(
        verified > 20,
        "too few deep out-of-bounds inputs verified: {verified}"
    );
    assert!(
        wrapped_negative > 0,
        "the x+sign overflow window (row 9) was not reached"
    );
    eprintln!("deep OOB subscript parity verified for {verified} inputs");
}

