//! Phase C — error-path differential tests for `unfilter`
//! (`ERRORS.md` rows 9..13).

mod common;

use common::*;

fn span(h: i32, len: i32) -> usize {
    let rows = if h > 0 { h as i64 } else { 0 } + 2;
    let l = (len as i64).abs() + 2;
    (rows * l + 64) as usize
}

fn build(w: i32, h: i32, bpp: i32, filters: &[u8], seed: u64) -> Case {
    let len = w.wrapping_mul(bpp);
    let pad = span(h, len) as isize;
    let total = (2 * pad + span(h, len) as isize) as usize;
    let mut rng = Rng::new(seed);
    let mut scratch: Vec<u8> = (0..total).map(|_| rng.u8()).collect();
    for (r, &f) in filters.iter().enumerate() {
        let off = pad + (r as isize) * (len as isize + 1);
        if off >= 0 && (off as usize) < total {
            scratch[off as usize] = f;
        }
    }
    Case::unfilter(scratch, w, h, bpp, pad)
}

/// ERRORS row 9: the row-0 filter byte is a de-facto enum; every one of the
/// 251 out-of-range values must be rejected identically, and nothing may have
/// been written before the rejection.
#[test]
fn err09_row0_bad_filter() {
    for f in 5u8..=255 {
        for (w, h, bpp) in [(8i32, 1i32, 3i32), (8, 4, 3), (1, 1, 1), (0, 2, 4), (-3, 3, 2)] {
            let case = build(w, h, bpp, &[f, 0, 0, 0], 0xE9 ^ f as u64);
            let o = diff(&case, &format!("err09 filter={f} w={w} h={h} bpp={bpp}"));
            assert_eq!(o.ret, 0, "row-0 filter {f} must be rejected");
            assert_eq!(o.err, None, "`unfilter` must not touch cp_error_reason");
            assert_eq!(o.scratch, case.scratch, "nothing may be written before the rejection");
            assert_eq!(o.status, Status::Exited(0));
        }
    }
}

/// ERRORS row 10: an out-of-range filter byte on a later row.  The rows before
/// it have already been unfiltered in place, so the partial mutation is part of
/// the observable result.
#[test]
fn err10_rowy_bad_filter() {
    let mut rng = Rng::new(0xE10);
    let mut saw_partial_mutation = false;
    for bad_row in 1usize..5 {
        for bad in [5u8, 6, 63, 128, 200, 255] {
            for _ in 0..12 {
                let h = bad_row as i32 + rng.range(1, 3);
                let w = rng.range(2, 12);
                let bpp = rng.range(1, 5);
                let mut filters: Vec<u8> = (0..h).map(|_| rng.pick(&[1u8, 2, 3, 4])).collect();
                filters[bad_row] = bad;
                let case = build(w, h, bpp, &filters, rng.next_u64());
                let o = diff(
                    &case,
                    &format!("err10 bad_row={bad_row} bad={bad} w={w} h={h} bpp={bpp}"),
                );
                assert_eq!(o.ret, 0);
                assert_eq!(o.err, None);
                saw_partial_mutation |= o.scratch != case.scratch;
            }
        }
    }
    assert!(
        saw_partial_mutation,
        "the partial-mutation-before-rejection path was never actually exercised"
    );
}

/// ERRORS row 10b: the *last* row carries the bad filter - every earlier row
/// is fully processed first.
#[test]
fn err10b_last_row_bad_filter() {
    let mut rng = Rng::new(0xE10B);
    for h in 2i32..8 {
        for bad in [5u8, 255] {
            let w = rng.range(2, 10);
            let bpp = rng.range(1, 4);
            let mut filters: Vec<u8> = (0..h).map(|_| rng.below(5) as u8).collect();
            *filters.last_mut().unwrap() = bad;
            let case = build(w, h, bpp, &filters, rng.next_u64());
            let o = diff(&case, &format!("err10b h={h} bad={bad}"));
            assert_eq!(o.ret, 0);
        }
    }
}

/// ERRORS row 11: `h <= 0` reads nothing at all - not even the filter byte.
/// Proved by putting an *invalid* filter byte everywhere: if the byte were
/// read, the call would return 0.
#[test]
fn err11_h_nonpositive_no_access() {
    for h in [0i32, -1, -2, -9999, i32::MIN] {
        for (w, bpp) in [(8i32, 3i32), (0, 0), (-4, -4), (1, 1), (100, 4)] {
            let len = w.wrapping_mul(bpp);
            let pad = span(h, len) as isize;
            let total = (2 * pad + span(h, len) as isize) as usize;
            let scratch = vec![0xFFu8; total]; // 0xFF = invalid filter byte
            let case = Case::unfilter(scratch, w, h, bpp, pad);
            let o = diff(&case, &format!("err11 h={h} w={w} bpp={bpp}"));
            assert_eq!(o.ret, 1, "h<=0 must return 1");
            assert_eq!(o.err, None);
            assert_eq!(o.scratch, case.scratch);
        }
    }
}

/// ERRORS row 12: `raw == NULL` is fine as long as `h <= 0`.
#[test]
fn err12_null_raw_h_nonpositive() {
    for h in [0i32, -1, i32::MIN] {
        for (w, bpp) in [(0i32, 0i32), (8, 3), (-1, -1), (i32::MAX, 7)] {
            let case = Case::unfilter_null(w, h, bpp);
            let o = diff(&case, &format!("err12 h={h} w={w} bpp={bpp}"));
            assert_eq!(o.status, Status::Exited(0), "must not crash");
            assert_eq!(o.ret, 1);
        }
    }
}

/// ERRORS row 13: `raw == NULL` with `h > 0` dereferences NULL in both
/// libraries - the same fatal signal.
#[test]
fn err13_null_raw_h_positive() {
    for h in [1i32, 2, 100] {
        for (w, bpp) in [(0i32, 0i32), (8, 3)] {
            let case = Case::unfilter_null(w, h, bpp);
            let o = diff(&case, &format!("err13 h={h} w={w} bpp={bpp}"));
            assert_eq!(
                o.status,
                Status::Signaled(libc::SIGSEGV),
                "expected both libraries to fault on *NULL"
            );
        }
    }
}

/// The `case 1` prologue of a non-first row is `for (x = 0; x < bpp; x++)
/// raw[x] += 0;` - the value never changes, but the read-modify-write is still
/// performed, so with a huge `bpp` it walks off the end of the buffer.  Both
/// libraries must fault identically (this is the check that a translation which
/// "optimises away" the no-op loop would fail).
#[test]
fn err15_row1_sub_prologue_touches_bpp_bytes() {
    for bpp in [1 << 20, 1 << 24] {
        // w = 0 => len = 0, so only the prologue loop runs at all.
        let scratch = vec![0u8; 4096];
        // row 0 filter = 0 (no-op), row 1 filter = 1 (Sub)
        let mut scratch = scratch;
        scratch[64] = 0;
        scratch[65] = 1; // row 1's filter byte lives at raw + 0*(len+1) + 1
        let case = Case::unfilter(scratch, 0, 2, bpp, 64);
        let o = diff(&case, &format!("err15 bpp={bpp}"));
        assert_eq!(
            o.status,
            Status::Signaled(libc::SIGSEGV),
            "expected both libraries to walk off the buffer: {o:?}"
        );
    }
    // ... and with a bpp that stays inside the buffer it must succeed in both.
    for bpp in [1i32, 7, 64] {
        let mut scratch = vec![0x5Au8; 4096];
        scratch[64] = 0;
        scratch[65] = 1;
        let case = Case::unfilter(scratch, 0, 2, bpp, 64);
        let o = diff(&case, &format!("err15 in-bounds bpp={bpp}"));
        assert_eq!(o.ret, 1);
        assert_eq!(o.status, Status::Exited(0));
    }
}

/// Generic boundary sweep: one step past the valid filter range, and the
/// extreme `w`/`bpp` values, mixed with valid rows.
#[test]
fn err14_filter_boundary_and_extreme_scalars() {
    let mut rng = Rng::new(0xE14);
    // exactly one step past the documented range
    for f in [4u8, 5] {
        for h in 1i32..4 {
            let case = build(6, h, 3, &vec![f; h as usize], rng.next_u64());
            let o = diff(&case, &format!("err14 f={f} h={h}"));
            assert_eq!(o.ret, if f == 4 { 1 } else { 0 });
        }
    }
    // extreme scalars with h <= 0 (no memory access, so safe to probe)
    for w in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        for bpp in [i32::MIN, -1, 0, 1, i32::MAX] {
            let case = Case::unfilter(vec![0xAA; 256], w, 0, bpp, 0);
            let o = diff(&case, &format!("err14 extreme w={w} bpp={bpp}"));
            assert_eq!(o.ret, 1);
            assert_eq!(o.scratch, case.scratch);
        }
    }
    // extreme w*bpp overflow with h == 1 and filter 0 (no loop runs)
    for (w, bpp) in [(i32::MAX, 2), (0x10000, 0x10000), (i32::MIN, -1), (46341, 46341)] {
        let mut scratch = vec![0u8; 4096];
        scratch[0] = 0; // filter None
        let case = Case::unfilter(scratch, w, 1, bpp, 0);
        let o = diff(&case, &format!("err14 overflow w={w} bpp={bpp}"));
        assert_eq!(o.ret, 1);
    }
}
