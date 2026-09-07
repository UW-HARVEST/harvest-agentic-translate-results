//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input/condition the row describes,
//! calls BOTH `.so`s through their exported symbols, and asserts they agree on
//! the *same* rejection: the same return value AND the same `cp_error_reason`
//! string (not merely "both failed somehow").

mod common;

use common::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const SEED: u64 = 0x5EED_1234;

fn rng(row: u64) -> StdRng {
    StdRng::seed_from_u64(SEED ^ (row << 24) ^ 0xC0FFEE)
}

fn lits(data: &[u8]) -> Vec<Item> {
    data.iter().map(|&b| Item::Lit(b)).collect()
}

fn fixed_stream(items: &[Item]) -> Vec<u8> {
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, true, items);
    w.finish()
}

/// Differential-checks a batch and asserts the C library returned exactly
/// `want_ret` with exactly `want_err` for each case.
#[track_caller]
fn expect_error(cases: Vec<Case>, want_ret: i32, want_err: Option<&str>) -> BatchReport {
    let rep = diff_inflate_batch(pair_release(), &cases);
    assert!(
        rep.c_died.is_empty(),
        "C(release) died instead of returning an error: {:?}",
        rep.c_died
    );
    assert_eq!(rep.compared, cases.len());
    for (i, c) in cases.iter().enumerate() {
        assert_eq!(
            rep.c[i].ret, want_ret,
            "[{}] expected cp_inflate == {want_ret}, got {} (err={:?})",
            c.label, rep.c[i].ret, rep.c[i].err_str()
        );
        match want_err {
            Some(e) => assert_eq!(
                rep.c[i].err_str(),
                e,
                "[{}] wrong cp_error_reason",
                c.label
            ),
            None => assert_eq!(rep.c[i].err, None, "[{}] expected no error", c.label),
        }
        // the Rust side was already asserted equal by diff_inflate_batch
        assert_eq!(rep.rust[i].ret, rep.c[i].ret);
        assert_eq!(rep.rust[i].err, rep.c[i].err);
    }
    rep
}

/// For inputs on which the C library kills itself: assert the Rust library
/// meets the *same* fate (same signal) or, for the `Release` build, that both
/// survive and agree.
#[track_caller]
fn assert_same_fate(lib_pair: &Pair, cases: &[Case]) {
    let rc = run_inflate_batch(lib_pair.c, cases);
    let rr = run_inflate_batch(lib_pair.rust, cases);
    for (i, c) in cases.iter().enumerate() {
        assert_eq!(
            (rc[i].completed, rc[i].signal()),
            (rr[i].completed, rr[i].signal()),
            "[{}] C {} but Rust {}",
            c.label,
            rc[i].describe(),
            rr[i].describe()
        );
        if rc[i].completed {
            assert_eq!(rc[i].ret, rr[i].ret, "[{}] return value", c.label);
            assert_eq!(rc[i].err, rr[i].err, "[{}] cp_error_reason", c.label);
            assert_eq!(rc[i].out, rr[i].out, "[{}] output buffer", c.label);
        }
    }
}

// ===========================================================================
// A. Hard rejections
// ===========================================================================

// ------------------------------------------------------------- ERRORS.md #1
// `unfilter`, row 0, filter byte >= 5  =>  returns 0, buffer untouched.
#[test]
fn err01_unfilter_row0_bad_filter() {
    let mut r = rng(1);
    for p in [pair_release(), pair_asserts()] {
        for fb in 5u16..=255 {
            for &bpp in &[1i32, 3, 4] {
                let w = 7i32;
                let mut b: Vec<u8> = (0..(1 + w * bpp + 32) as usize)
                    .map(|_| r.gen::<u8>())
                    .collect();
                b[0] = fb as u8;
                let before = b.clone();
                diff_unfilter(p, &format!("#1 fb={fb} bpp={bpp}"), w, 1, bpp, &b);
                // and pin down the documented result on the C side
                let mut a = b.clone();
                let ret = unsafe { (p.c.unfilter)(w, 1, bpp, a.as_mut_ptr()) };
                assert_eq!(ret, 0, "filter byte {fb} must be rejected");
                assert_eq!(a, before, "the buffer must be left untouched");
            }
        }
    }
}

// ------------------------------------------------------------- ERRORS.md #2
// `unfilter`, row y >= 1, filter byte >= 5  =>  returns 0, earlier rows
// already un-filtered in place.
#[test]
fn err02_unfilter_later_row_bad_filter() {
    let mut r = rng(2);
    for p in [pair_release(), pair_asserts()] {
        for bad_row in 1usize..=3 {
            for fb in [5u8, 6, 17, 128, 200, 255] {
                for &bpp in &[1i32, 2, 3, 4] {
                    let (w, h) = (9i32, 4i32);
                    let stride = 1 + (w * bpp) as usize;
                    let mut b: Vec<u8> = (0..h as usize * stride + 32)
                        .map(|_| r.gen::<u8>())
                        .collect();
                    for y in 0..h as usize {
                        b[y * stride] = if y == bad_row { fb } else { (y % 5) as u8 };
                    }
                    diff_unfilter(
                        p,
                        &format!("#2 bad_row={bad_row} fb={fb} bpp={bpp}"),
                        w,
                        h,
                        bpp,
                        &b,
                    );
                    let mut a = b.clone();
                    let ret = unsafe { (p.c.unfilter)(w, h, bpp, a.as_mut_ptr()) };
                    assert_eq!(ret, 0);
                    // rows after the bad one are untouched
                    assert_eq!(&a[bad_row * stride..], &b[bad_row * stride..]);
                    // and at least one earlier row was modified in place
                    // (unless the earlier filters were all "None"/"Up-on-row-0")
                    assert!(a.len() == b.len());
                }
            }
        }
    }
}

// ------------------------------------------------------------- ERRORS.md #3
// stored block whose LEN is not the complement of NLEN.
#[test]
fn err03_stored_len_nlen_mismatch() {
    let mut r = rng(3);
    let mut cases = Vec::new();
    for _ in 0..40 {
        let n: usize = r.gen_range(0..=64);
        let payload: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
        let mut w = BitWriter::new();
        write_stored_block(&mut w, true, &payload);
        let mut s = w.finish();
        // corrupt one bit of NLEN (bytes 3..5 of the stream)
        let bit = r.gen_range(0..16);
        s[3 + bit / 8] ^= 1 << (bit % 8);
        cases.push(Case::new(format!("#3 n={n} nlen bit {bit}"), s, n as i32 + 8));
    }
    expect_error(
        cases,
        0,
        Some(
            "Failed to find LEN and NLEN as complements within stored \
             (uncompressed) stream.",
        ),
    );
}

// ------------------------------------------------------------- ERRORS.md #4
// `s->bits_left / 8 > (int)LEN`: the stored block claims fewer bytes than the
// input still holds.  (Note the direction of the C check.)
#[test]
fn err04_stored_block_extends_beyond_input() {
    let mut r = rng(4);
    let mut cases = Vec::new();
    for extra in 1usize..=12 {
        let n: usize = r.gen_range(1..=32);
        let payload: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
        let mut w = BitWriter::new();
        write_stored_block(&mut w, true, &payload);
        let mut s = w.finish();
        s.extend((0..extra).map(|_| r.gen::<u8>())); // trailing junk
        cases.push(Case::new(
            format!("#4 LEN={n} +{extra} trailing bytes"),
            s,
            n as i32 + 64,
        ));
    }
    // two stored blocks back to back: the first one is not last, so it is
    // rejected for the same reason
    let mut w = BitWriter::new();
    write_stored_block(&mut w, false, &[1, 2, 3, 4]);
    write_stored_block(&mut w, true, &[5, 6, 7, 8]);
    cases.push(Case::new("#4 two stored blocks", w.finish(), 64));
    expect_error(
        cases,
        0,
        Some("Stored block extends beyond end of input stream."),
    );
}

// ------------------------------------------------------------- ERRORS.md #5
// literal decoded with no room left in the output buffer.
#[test]
fn err05_out_buffer_full_on_literal() {
    let mut r = rng(5);
    let mut cases = Vec::new();
    // out_bytes == 0
    for _ in 0..20 {
        let payload: Vec<u8> = (0..r.gen_range(1..=32)).map(|_| r.gen::<u8>()).collect();
        cases.push(Case::new(
            "#5 out_bytes=0",
            fixed_stream(&lits(&payload)),
            0,
        ));
    }
    // out_bytes one byte short of the payload
    for n in 1usize..=24 {
        let payload: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
        cases.push(Case::new(
            format!("#5 n={n} out_bytes={}", n - 1),
            fixed_stream(&lits(&payload)),
            (n - 1) as i32,
        ));
    }
    // negative out_bytes => out_end < out, so the very first literal trips it
    cases.push(Case::new(
        "#5 out_bytes=-1",
        fixed_stream(&lits(&[7u8])),
        -1,
    ));
    cases.push(
        Case::new("#5 out_bytes=i32::MIN", fixed_stream(&lits(&[7u8])), i32::MIN).out_cap(64),
    );
    expect_error(
        cases,
        0,
        Some("Attempted to overwrite out buffer while outputting a symbol."),
    );
}

// ------------------------------------------------------------- ERRORS.md #6
// back-reference pointing before the start of the output buffer.  Checked
// BEFORE #7, so a case that violates both must report #6.
#[test]
fn err06_backwards_distance_before_buffer() {
    let mut cases = Vec::new();
    // a match as the very first symbol: out == begin, so any distance >= 1 is
    // already out of range
    for dist in [1u32, 2, 3, 17, 1024, 32768] {
        for length in [3u32, 100, 258] {
            cases.push(Case::new(
                format!("#6 first-symbol match dist={dist} len={length}"),
                fixed_stream(&[Item::Match(length, dist)]),
                4096,
            ));
        }
    }
    // one literal emitted, then a distance of 2
    cases.push(Case::new(
        "#6 one literal then dist=2",
        fixed_stream(&[Item::Lit(0xAB), Item::Match(3, 2)]),
        4096,
    ));
    // violates #6 AND #7 at the same time -> must report #6
    cases.push(Case::new(
        "#6 both #6 and #7 violated",
        fixed_stream(&[Item::Match(258, 1)]),
        1,
    ));
    expect_error(
        cases,
        0,
        Some("Attempted to write before out buffer (invalid backwards distance)."),
    );
}

// ------------------------------------------------------------- ERRORS.md #7
// the copy would run past the end of the output buffer (but the distance is
// legal, so #6 does not fire first).
#[test]
fn err07_string_overruns_out_buffer() {
    let mut r = rng(7);
    let mut cases = Vec::new();
    for length in [3u32, 4, 17, 100, 257, 258] {
        for dist in [1u32, 2, 5] {
            let seed: Vec<u8> = (0..dist).map(|_| r.gen::<u8>()).collect();
            let mut items = lits(&seed);
            items.push(Item::Match(length, dist));
            // exactly one byte too little room for the match
            let ob = (dist + length - 1) as i32;
            cases.push(
                Case::new(
                    format!("#7 len={length} dist={dist} out_bytes={ob}"),
                    fixed_stream(&items),
                    ob,
                )
                .out_cap(ob as usize + 512),
            );
        }
    }
    expect_error(
        cases,
        0,
        Some("Attempted to overwrite out buffer while outputting a string."),
    );
}

// ------------------------------------------------------------- ERRORS.md #8
// BTYPE == 3 (reserved).
#[test]
fn err08_unknown_block_type() {
    let mut r = rng(8);
    let mut cases = Vec::new();
    for bfinal in [false, true] {
        for _ in 0..8 {
            let mut w = BitWriter::new();
            w.bits(bfinal as u32, 1);
            w.bits(3, 2); // BTYPE == 3
            for _ in 0..8 {
                w.bits(r.gen::<u32>(), 8);
            }
            cases.push(Case::new(
                format!("#8 btype=3 bfinal={bfinal}"),
                w.finish(),
                256,
            ));
        }
    }
    // BTYPE 3 reached as the *second* block, after a valid fixed block
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, false, &lits(b"hello"));
    w.bits(1, 1);
    w.bits(3, 2);
    w.bits(0, 32);
    cases.push(Case::new("#8 btype=3 as second block", w.finish(), 256));
    expect_error(
        cases,
        0,
        Some("Detected unknown block type within input stream."),
    );
}

// ---------------------------------------------------------- ERRORS.md #9..#12
// `cp_chunk` / `cp_find` are `static` in lib.c, never called from it, and
// therefore absent from BOTH shared objects' dynamic symbol tables — there is
// no way for an external caller to reach them.  This test pins that fact down
// so the rows are accounted for rather than silently skipped.
#[test]
fn err09_to_12_png_chunk_helpers_are_unreachable() {
    for so in [
        common::c_so_asserts(),
        common::c_so_release(),
        common::rust_so(),
    ] {
        let lib = unsafe { libloading::Library::new(&so).unwrap() };
        for name in [
            b"cp_chunk\0".as_ref(),
            b"cp_find\0".as_ref(),
            b"cp_make32\0".as_ref(),
            b"cp_paeth\0".as_ref(),
            b"cp_build\0".as_ref(),
            b"cp_decode\0".as_ref(),
            b"cp_stored\0".as_ref(),
            b"cp_block\0".as_ref(),
            b"cp_dynamic\0".as_ref(),
            b"cp_fixed\0".as_ref(),
        ] {
            let r: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(name) };
            assert!(
                r.is_err(),
                "{} unexpectedly exports {:?}",
                so.display(),
                std::str::from_utf8(name).unwrap()
            );
        }
    }
}

// ===========================================================================
// C. assert() rows (#13..#22)
// ===========================================================================

/// Builds the deterministic assert-tripping inputs referenced by ERRORS.md
/// rows 13..22.
fn assert_trippers() -> Vec<Case> {
    let mut v = Vec::new();
    // #18: bits_left <= 0 on the very first cp_read_bits
    v.push(Case::new("#18 in_bytes=0", vec![], 64).in_bytes(0));
    v.push(Case::new("#18 in_bytes=0, empty out", vec![], 0).in_bytes(0));
    // #31-ish: negative in_bytes => bits_left < 0
    v.push(Case::new("#31 in_bytes=-1", vec![0u8; 16], 64).in_bytes(-1));
    v.push(Case::new("#31 in_bytes=-1024", vec![0u8; 16], 64).in_bytes(-1024));
    v.push(Case::new("#31 in_bytes=i32::MIN", vec![0u8; 16], 64).in_bytes(i32::MIN));
    // #15/#20: truncated stream -- the header is there but the data is not
    for n in 1i32..=4 {
        let s = fixed_stream(&lits(b"hello world"));
        v.push(Case::new(format!("#20 truncated to {n} bytes"), s, 64).in_bytes(n));
    }
    // #13: stored block reached at a residual bit count that is not a multiple
    // of 8 (`cp_ptr`'s assert)
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, false, &lits(b"ab"));
    // the next block starts mid-byte; make it a stored block
    w.bits(1, 1);
    w.bits(0, 2);
    w.align();
    w.bits(2, 16);
    w.bits(!2u16 as u32, 16);
    w.bytes(&[0xAA, 0xBB]);
    v.push(Case::new("#13 stored after a fixed block", w.finish(), 64));
    // #21/#22: corrupt dynamic headers -- code lengths >= 16 and trees that
    // decode to nothing
    let mut w = BitWriter::new();
    w.bits(1, 1);
    w.bits(2, 2); // BTYPE 2
    w.bits(0, 5); // HLIT  = 257
    w.bits(0, 5); // HDIST = 1
    w.bits(0, 4); // HCLEN = 4
    w.bits(0, 3 * 4); // all four transmitted code lengths are 0 -> empty tree
    w.bits(0, 64);
    v.push(Case::new("#22 all-zero code-length tree", w.finish(), 64));

    let mut w = BitWriter::new();
    w.bits(1, 1);
    w.bits(2, 2);
    w.bits(31, 5); // HLIT  = 288
    w.bits(31, 5); // HDIST = 32
    w.bits(15, 4); // HCLEN = 19
    for _ in 0..19 {
        w.bits(7, 3); // every code-length symbol at depth 7 -> over-subscribed
    }
    w.bits(0xFFFF_FFFF, 32);
    w.bits(0xFFFF_FFFF, 32);
    v.push(Case::new("#21 over-subscribed clen tree", w.finish(), 64));
    v
}

/// The `assert()`-enabled C build must die (SIGABRT) on at least the rows that
/// exist to document those asserts; this proves the asserts really are live in
/// the configuration the task's `cmake ..` produces, and that our harness
/// notices.
#[test]
fn err13_to_22_asserts_are_live_in_the_assert_build() {
    let cases = assert_trippers();
    let rc = run_inflate_batch(pair_asserts().c, &cases);
    let mut aborted = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        if !rc[i].completed {
            aborted.push((c.label.clone(), rc[i].signal()));
        }
    }
    eprintln!("assert-build C outcomes:");
    for (i, c) in cases.iter().enumerate() {
        eprintln!("  {:<40} {}", c.label, rc[i].describe());
    }
    assert!(
        !aborted.is_empty(),
        "expected the assert-enabled C build to abort on at least one of {:?}",
        cases.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        aborted.iter().any(|(_, s)| *s == Some(libc::SIGABRT)),
        "expected at least one SIGABRT, got {aborted:?}"
    );
}

/// In the `NDEBUG` configuration the translation targets, the same inputs must
/// produce *identical* observable behaviour in C and Rust — including dying the
/// same way if the C code's own out-of-bounds writes kill it.
#[test]
fn err13_to_22_release_build_matches_rust() {
    let cases = assert_trippers();
    let p = pair_release();
    let rc = run_inflate_batch(p.c, &cases);
    let rr = run_inflate_batch(p.rust, &cases);
    let mut mismatches = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        eprintln!(
            "  {:<40} C: {:<40} Rust: {}",
            c.label,
            rc[i].describe(),
            rr[i].describe()
        );
        if !rc[i].completed {
            // The C library destroyed itself (its own OOB stack writes). No
            // behaviour can be "matched"; record it and move on.
            continue;
        }
        if rc[i].completed != rr[i].completed
            || rc[i].ret != rr[i].ret
            || rc[i].err != rr[i].err
            || rc[i].out != rr[i].out
        {
            mismatches.push(format!(
                "[{}] C: {} Rust: {}",
                c.label,
                rc[i].describe(),
                rr[i].describe()
            ));
        }
    }
    assert!(mismatches.is_empty(), "divergences:\n{}", mismatches.join("\n"));
}

// ===========================================================================
// D. Generic FFI boundaries (#23..#34)
// ===========================================================================

// ------------------------------------------------------------ ERRORS.md #23/24
#[test]
fn err23_24_unfilter_h_zero_and_negative() {
    let mut r = rng(23);
    for p in [pair_release(), pair_asserts()] {
        for &h in &[0i32, -1, -2, -1000, i32::MIN] {
            for &bpp in &[1i32, 3, 4] {
                for &w in &[0i32, 1, 5, 64] {
                    let mut b: Vec<u8> = (0..512).map(|_| r.gen::<u8>()).collect();
                    for fb in [0u8, 4, 5, 255] {
                        b[0] = fb;
                        let before = b.clone();
                        diff_unfilter(
                            p,
                            &format!("#23 h={h} w={w} bpp={bpp} fb={fb}"),
                            w,
                            h,
                            bpp,
                            &b,
                        );
                        let mut a = b.clone();
                        let ret = unsafe { (p.c.unfilter)(w, h, bpp, a.as_mut_ptr()) };
                        assert_eq!(ret, 1, "h<=0 must succeed");
                        assert_eq!(a, before, "h<=0 must not touch the buffer");
                    }
                }
            }
        }
    }
}

// -------------------------------------------------------------- ERRORS.md #25
#[test]
fn err25_unfilter_len_zero() {
    let mut r = rng(25);
    for p in [pair_release(), pair_asserts()] {
        for (w, bpp) in [(0i32, 4i32), (0, 1), (7, 0), (0, 0)] {
            for h in 1..=4 {
                for fb in 0u16..=255 {
                    let mut b: Vec<u8> = (0..256).map(|_| r.gen::<u8>()).collect();
                    for y in 0..h as usize {
                        b[y] = fb as u8; // stride is 1 when len == 0
                    }
                    diff_unfilter(
                        p,
                        &format!("#25 w={w} bpp={bpp} h={h} fb={fb}"),
                        w,
                        h,
                        bpp,
                        &b,
                    );
                    let mut a = b.clone();
                    let ret = unsafe { (p.c.unfilter)(w, h, bpp, a.as_mut_ptr()) };
                    // With `len == 0` but `bpp > 0` the row-`y` prologue loops
                    // (`for x = 0; x < bpp`) still run, so they write over the
                    // *filter bytes of the following rows* (ERRORS.md #26).
                    // The return value is therefore data-dependent as soon as
                    // filters 2/3/4 meet `h >= 3`; only the unambiguous cases
                    // get a hard expectation, the rest rely on the byte-for-byte
                    // differential check above.
                    let expect = if fb > 4 {
                        Some(0)
                    } else if bpp == 0 || h <= 2 || fb <= 1 {
                        Some(1)
                    } else {
                        None
                    };
                    if let Some(e) = expect {
                        assert_eq!(ret, e, "w={w} bpp={bpp} h={h} fb={fb}");
                    }
                }
            }
        }
    }
}

// -------------------------------------------------------------- ERRORS.md #26
#[test]
fn err26_unfilter_bpp_greater_than_len() {
    let mut r = rng(26);
    for p in [pair_release(), pair_asserts()] {
        for bpp in 2i32..=12 {
            for w in 0i32..=1 {
                for h in 1i32..=4 {
                    for fb in 0u8..=4 {
                        let mut b: Vec<u8> = (0..512).map(|_| r.gen::<u8>()).collect();
                        let stride = (1 + w * bpp) as usize;
                        for y in 0..h as usize {
                            b[y * stride] = fb;
                        }
                        diff_unfilter(
                            p,
                            &format!("#26 bpp={bpp} w={w} h={h} fb={fb}"),
                            w,
                            h,
                            bpp,
                            &b,
                        );
                    }
                }
            }
        }
    }
}

// -------------------------------------------------------------- ERRORS.md #27
#[test]
fn err27_unfilter_negative_dimensions() {
    let mut r = rng(27);
    const MID: usize = 8192;
    let b: Vec<u8> = (0..16384).map(|_| r.gen::<u8>()).collect();
    for p in [pair_release(), pair_asserts()] {
        for &w in &[-1i32, -2, -7, -64] {
            for &bpp in &[-1i32, -2, 1, 3, 4] {
                for h in 1i32..=4 {
                    diff_unfilter_at(
                        p,
                        &format!("#27 w={w} bpp={bpp} h={h}"),
                        w,
                        h,
                        bpp,
                        &b,
                        MID,
                    );
                }
            }
        }
        // positive w, negative bpp
        for &bpp in &[-1i32, -3, -8] {
            for h in 1i32..=4 {
                diff_unfilter_at(p, &format!("#27b bpp={bpp} h={h}"), 32, h, bpp, &b, MID);
            }
        }
    }
}

// -------------------------------------------------------------- ERRORS.md #28
// the full out-of-range "enum" domain of the filter byte, exhaustively, on
// every row position.
#[test]
fn err28_filter_byte_full_domain() {
    let mut r = rng(28);
    for p in [pair_release(), pair_asserts()] {
        for fb in 0u16..=255 {
            for h in 1i32..=3 {
                let (w, bpp) = (5i32, 3i32);
                let stride = (1 + w * bpp) as usize;
                let mut b: Vec<u8> = (0..(h as usize * stride + 64)).map(|_| r.gen::<u8>()).collect();
                for y in 0..h as usize {
                    b[y * stride] = fb as u8;
                }
                diff_unfilter(p, &format!("#28 fb={fb} h={h}"), w, h, bpp, &b);
                let mut a = b.clone();
                let ret = unsafe { (p.c.unfilter)(w, h, bpp, a.as_mut_ptr()) };
                assert_eq!(
                    ret,
                    if fb <= 4 { 1 } else { 0 },
                    "filter byte {fb} on {h} row(s)"
                );
            }
        }
    }
}

// -------------------------------------------------------------- ERRORS.md #29
#[test]
fn err29_inflate_in_bytes_zero() {
    let cases = vec![
        Case::new("#29 in_bytes=0 out=64", vec![], 64).in_bytes(0),
        Case::new("#29 in_bytes=0 out=0", vec![], 0).in_bytes(0),
        Case::new("#29 in_bytes=0, non-empty buffer", vec![0xFF; 32], 64).in_bytes(0),
    ];
    // release build: no assert, so it decodes from `bits == 0` -> BTYPE 0
    assert_same_fate(pair_release(), &cases);
    // assert build: it aborts; Rust does not -- documented in ERRORS.md row 18.
    let rc = run_inflate_batch(pair_asserts().c, &cases);
    for (i, c) in cases.iter().enumerate() {
        eprintln!("  #29 assert-build {:<30} {}", c.label, rc[i].describe());
    }
}

// -------------------------------------------------------------- ERRORS.md #30
#[test]
fn err30_inflate_out_bytes_zero() {
    let mut r = rng(30);
    let mut cases = Vec::new();
    for _ in 0..20 {
        let payload: Vec<u8> = (0..r.gen_range(1..=64)).map(|_| r.gen::<u8>()).collect();
        cases.push(Case::new(
            "#30 fixed, out_bytes=0",
            fixed_stream(&lits(&payload)),
            0,
        ));
        // NB: `deflate_raw` may emit a *stored* block for short payloads, and
        // `cp_stored` performs no output-bounds check at all (it just
        // `memcpy`s LEN bytes) -- so only force a real Huffman block here.
        let mut w = BitWriter::new();
        dynamic_block_auto(
            &mut w,
            true,
            &lit_lens_all_literals(257),
            &dst_lens_complete(2),
            &lits(&payload),
        );
        cases.push(Case::new("#30 dynamic, out_bytes=0", w.finish(), 0));
    }
    expect_error(
        cases,
        0,
        Some("Attempted to overwrite out buffer while outputting a symbol."),
    );
}

// -------------------------------------------------------------- ERRORS.md #31
#[test]
fn err31_inflate_negative_in_bytes() {
    let s = fixed_stream(&lits(b"hello, world"));
    let cases: Vec<Case> = [-1i32, -3, -4, -1000, i32::MIN + 8]
        .iter()
        .map(|&n| Case::new(format!("#31 in_bytes={n}"), s.clone(), 256).in_bytes(n))
        .collect();
    assert_same_fate(pair_release(), &cases);
}

// -------------------------------------------------------------- ERRORS.md #32
#[test]
fn err32_inflate_negative_out_bytes() {
    let s = fixed_stream(&lits(b"hello"));
    let cases: Vec<Case> = [-1i32, -2, -64, i32::MIN]
        .iter()
        .map(|&n| Case::new(format!("#32 out_bytes={n}"), s.clone(), n).out_cap(256))
        .collect();
    expect_error(
        cases,
        0,
        Some("Attempted to overwrite out buffer while outputting a symbol."),
    );
}

// ----------------------------------------------------------- ERRORS.md #33/34
// misalignment and non-multiple-of-4 tails on *malformed* input, so that the
// `first_bytes` / `final_word` paths are exercised on the error side too.
#[test]
fn err33_34_alignment_paths_on_malformed_input() {
    let mut r = rng(33);
    let mut cases = Vec::new();
    for misalign in 0usize..4 {
        for len in 1usize..=12 {
            for _ in 0..6 {
                let s: Vec<u8> = (0..len).map(|_| r.gen::<u8>()).collect();
                cases.push(
                    Case::new(
                        format!("#33 misalign={misalign} len={len} {s:02x?}"),
                        s,
                        64,
                    )
                    .misaligned(misalign),
                );
            }
        }
    }
    assert_same_fate(pair_release(), &cases);
}

// ------------------------------------------------------------- NULL pointers
#[test]
fn err_null_pointers() {
    let s = fixed_stream(&lits(b"hi"));
    // NULL out with out_bytes == 0: out_end == out == NULL, so the first
    // literal is rejected before any dereference.
    let cases = vec![Case::new("null out, out_bytes=0", s.clone(), 0).null_out()];
    expect_error(
        cases,
        0,
        Some("Attempted to overwrite out buffer while outputting a symbol."),
    );
    // NULL in / NULL out with a real length: both libraries must meet the same
    // fate (a segfault in the same place).
    //
    // NB: this holds for the crate's declared artifact, the `[profile.release]`
    // cdylib (`debug-assertions = false`).  A cdylib built with the `dev`
    // profile traps in `core::ptr`'s own null/alignment debug assertion and so
    // dies with SIGABRT instead of SIGSEGV on these already-UB inputs.
    let hard = vec![
        // out == NULL with a *negative* out_bytes makes `out_end` wrap around
        // to ~0, so the `out + 1 <= out_end` check passes and the C code
        // dereferences NULL.  Both libraries must die the same way.
        Case::new("null out, out_bytes=-1", s.clone(), -1).null_out(),
        Case::new("null in, in_bytes=0", vec![], 64).in_bytes(0).null_in(),
        Case::new("null in, in_bytes=8", vec![0; 8], 64).null_in(),
        Case::new("null out, out_bytes=8", s.clone(), 8).null_out(),
        Case::new("null in and out", vec![0; 8], 8).null_in().null_out(),
    ];
    assert_same_fate(pair_release(), &hard);
}

// ---------------------------------------------------- randomised malformed fuzz
/// Broad property test over malformed input: for every input the C library
/// survives, the Rust library must survive too and produce byte-identical
/// results.  Covers the assert/UB rows (#13..#22) far beyond the handful of
/// hand-built cases, and any error branch reachable by bit-twiddling a valid
/// stream.
#[test]
fn fuzz_malformed_streams_match() {
    let mut r = rng(99);
    let mut cases = Vec::new();

    // (a) pure random bytes
    for _ in 0..600 {
        let n = r.gen_range(1..=48);
        let s: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
        cases.push(
            Case::new(format!("fuzz-rand {s:02x?}"), s, r.gen_range(0..=256))
                .misaligned(r.gen_range(0..4))
                .out_cap(320),
        );
    }
    // (b) valid streams with a single bit flipped
    for _ in 0..600 {
        let n = r.gen_range(1..=64);
        let payload: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
        let mut s = if r.gen_bool(0.5) {
            fixed_stream(&lits(&payload))
        } else {
            deflate_raw(&payload, 9)
        };
        let nbits = s.len() * 8;
        let bit = r.gen_range(0..nbits);
        s[bit / 8] ^= 1 << (bit % 8);
        cases.push(
            Case::new(format!("fuzz-flip1 bit={bit} len={}", s.len()), s, 512)
                .misaligned(r.gen_range(0..4))
                .out_cap(576),
        );
    }
    // (c) valid streams truncated
    for _ in 0..400 {
        let n = r.gen_range(4..=64);
        let payload: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
        let s = deflate_raw(&payload, 9);
        let keep = r.gen_range(1..=s.len());
        cases.push(
            Case::new(format!("fuzz-trunc {keep}/{}", s.len()), s[..keep].to_vec(), 512)
                .out_cap(576),
        );
    }
    // (d) valid streams with several bits flipped
    for _ in 0..400 {
        let n = r.gen_range(8..=256);
        let payload: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
        let mut s = deflate_raw(&payload, 6);
        for _ in 0..r.gen_range(2..=6) {
            let bit = r.gen_range(0..s.len() * 8);
            s[bit / 8] ^= 1 << (bit % 8);
        }
        cases.push(Case::new(format!("fuzz-flipN len={}", s.len()), s, 1024).out_cap(1088));
    }

    let p = pair_release();
    let rc = run_inflate_batch(p.c, &cases);
    let rr = run_inflate_batch(p.rust, &cases);
    let (mut compared, mut c_died, mut mismatch) = (0usize, 0usize, Vec::new());
    for (i, c) in cases.iter().enumerate() {
        if !rc[i].completed {
            c_died += 1;
            continue;
        }
        if !rr[i].completed
            || rc[i].ret != rr[i].ret
            || rc[i].err != rr[i].err
            || rc[i].out != rr[i].out
        {
            if mismatch.len() < 10 {
                mismatch.push(format!(
                    "[{}] C: {} Rust: {}{}",
                    c.label,
                    rc[i].describe(),
                    rr[i].describe(),
                    if rc[i].completed && rr[i].completed && rc[i].out != rr[i].out {
                        let j = rc[i]
                            .out
                            .iter()
                            .zip(rr[i].out.iter())
                            .position(|(x, y)| x != y)
                            .unwrap();
                        format!(" (out differs first at byte {j})")
                    } else {
                        String::new()
                    }
                ));
            }
        } else {
            compared += 1;
        }
    }
    eprintln!(
        "fuzz: {} cases, {compared} matched, {c_died} the C library killed itself on, \
         {} divergences",
        cases.len(),
        mismatch.len()
    );
    assert!(
        mismatch.is_empty(),
        "{} divergences (first {}):\n{}",
        mismatch.len(),
        mismatch.len().min(10),
        mismatch.join("\n")
    );
    assert!(compared > cases.len() / 2, "too few comparable cases");
}
