//! Phase B — valid-path differential tests. One test per row of `CONFIGS.md`.
//!
//! Every call goes through `dlsym` on the two `.so` handles; the Rust crate is
//! never called directly. Randomized rows use the fixed seed from `common::SEED`.

mod common;

use common::*;

// ===========================================================================
// Rows 1-8: `printLine` — the lowest-level entry point
// ===========================================================================

#[test]
fn cfg_01_print_line_ascii() {
    for s in [
        "hello",
        "Calling good()...",
        "Finished bad()",
        "ERROR: Array index is negative.",
        "ERROR: Array index is out-of-bounds",
        "a b c\td",
    ] {
        let out = diff_print_line_bytes(s.as_bytes());
        assert_eq!(out, format!("{s}\n").into_bytes(), "printLine({s:?}) shape");
    }
}

#[test]
fn cfg_02_print_line_empty() {
    let out = diff_print_line_bytes(b"");
    assert_eq!(out, b"\n", "printLine(\"\") must emit exactly one newline");
}

#[test]
fn cfg_03_print_line_every_single_byte() {
    // 0x00 excluded: it is the C-string terminator, giving the empty string.
    for b in 1u8..=255 {
        let out = diff_print_line_bytes(&[b]);
        assert_eq!(out, vec![b, b'\n'], "printLine of single byte {b:#04x}");
    }
}

#[test]
fn cfg_04_print_line_percent_data() {
    // If either side passed `line` as printf's *format* string, these would
    // consume garbage varargs (or crash on %n) instead of printing literally.
    for s in [
        "%s", "%d", "%x", "%p", "%n", "%%", "100%", "%s%s%s%s%s%s%s%s",
        "%d items at %s each", "%99999999d", "%.*f", "%1$s %2$s",
        "%n%n%n%n", "a%sb%dc%pd",
    ] {
        let out = diff_print_line_bytes(s.as_bytes());
        assert_eq!(out, format!("{s}\n").into_bytes(), "printLine({s:?}) must print literally");
    }
}

#[test]
fn cfg_05_print_line_embedded_ws() {
    for s in ["a\nb", "\n", "\n\n\n", "a\tb", "a\rb", "line1\nline2\nline3", "  padded  "] {
        let out = diff_print_line_bytes(s.as_bytes());
        assert_eq!(out, format!("{s}\n").into_bytes());
    }
}

#[test]
fn cfg_06_print_line_non_utf8() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xff, 0xfe, 0xfd],
        vec![0xc3],                   // truncated 2-byte UTF-8 lead
        vec![0xe2, 0x82],             // truncated 3-byte UTF-8
        vec![0xf0, 0x9f, 0x92],       // truncated 4-byte UTF-8
        (0x80u8..=0xff).collect(),
        vec![b'a', 0xff, b'b', 0x80, b'c'],
    ];
    for c in cases {
        let out = diff_print_line_bytes(&c);
        let mut want = c.clone();
        want.push(b'\n');
        assert_eq!(out, want, "printLine on non-UTF-8 bytes {c:?}");
    }
}

#[test]
fn cfg_07_print_line_huge() {
    // 100 000 bytes crosses glibc's stdout buffer several times over, so a
    // buffering/flush divergence would show up here.
    for len in [4095usize, 4096, 4097, 8192, 65_536, 100_000] {
        let s = vec![b'Z'; len];
        let out = diff_print_line_bytes(&s);
        assert_eq!(out.len(), len + 1, "printLine of {len} bytes");
        assert_eq!(*out.last().unwrap(), b'\n');
    }
}

#[test]
fn cfg_08_print_line_random() {
    let mut rng = Rng::new(SEED);
    for _ in 0..512 {
        let len = rng.below(257) as usize;
        let bytes = rng.bytes(len);
        let out = diff_print_line_bytes(&bytes);
        let mut want = bytes.clone();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

// ===========================================================================
// Rows 9-13: `printIntLine` — the other lowest-level entry point
// ===========================================================================

#[test]
fn cfg_09_print_int_line_zero() {
    let out = diff_print_int_line(0);
    assert_eq!(out, b"0\n");
}

#[test]
fn cfg_10_print_int_line_extremes() {
    for v in [i32::MAX, i32::MIN, -1, 1, i32::MIN + 1, i32::MAX - 1] {
        let out = diff_print_int_line(v);
        assert_eq!(out, format!("{v}\n").into_bytes(), "printIntLine({v})");
    }
}

#[test]
fn cfg_11_print_int_line_bit_positions() {
    for bit in 0..31 {
        let v = 1i32 << bit;
        for cand in [v, -v, v - 1, v.wrapping_neg().wrapping_sub(1)] {
            let out = diff_print_int_line(cand);
            assert_eq!(out, format!("{cand}\n").into_bytes(), "printIntLine({cand})");
        }
    }
    // The sign bit on its own is INT_MIN.
    let out = diff_print_int_line(i32::MIN);
    assert_eq!(out, b"-2147483648\n");
}

#[test]
fn cfg_12_print_int_line_random() {
    let mut rng = Rng::new(SEED ^ 0x1111);
    for _ in 0..4096 {
        let v = rng.next_i32();
        let out = diff_print_int_line(v);
        assert_eq!(out, format!("{v}\n").into_bytes());
    }
}

#[test]
fn cfg_13_print_int_line_sequence() {
    // Many calls inside ONE capture, so the accumulated byte stream (and any
    // buffering difference) is compared, not just one line at a time.
    let mut rng = Rng::new(SEED ^ 0x2222);
    let vals: Vec<i32> = (0..2000).map(|_| rng.next_i32()).collect();
    let v2 = vals.clone();
    let v3 = vals.clone();
    let out = diff(
        "printIntLine x2000 in sequence",
        || unsafe {
            let f = print_int_line::c();
            for v in &v2 {
                f(*v);
            }
        },
        || unsafe {
            let f = print_int_line::rs();
            for v in &v3 {
                f(*v);
            }
        },
    );
    let want: String = vals.iter().map(|v| format!("{v}\n")).collect();
    assert_eq!(out, want.into_bytes());
}

// ===========================================================================
// Rows 14-18: `bad` — mid level
// ===========================================================================

/// The exact 10 lines `bad`/`goodB2G`/`goodG2B` print when index `i` is written.
fn slot_pattern(i: usize) -> Vec<u8> {
    (0..10)
        .map(|k| if k == i { "1\n" } else { "0\n" })
        .collect::<String>()
        .into_bytes()
}

/// All ten elements still zero (what an out-of-bounds write produces).
fn all_zero_pattern() -> Vec<u8> {
    "0\n".repeat(10).into_bytes()
}

#[test]
fn cfg_14_bad_index_zero() {
    let out = diff_bad(0);
    assert_eq!(out, slot_pattern(0), "bad(0) writes the first slot");
}

#[test]
fn cfg_15_bad_interior_indices() {
    for i in 1..=8 {
        let out = diff_bad(i);
        assert_eq!(out, slot_pattern(i as usize), "bad({i})");
    }
}

#[test]
fn cfg_16_bad_index_nine() {
    let out = diff_bad(9);
    assert_eq!(out, slot_pattern(9), "bad(9) writes the last in-bounds slot");
}

#[test]
fn cfg_17_bad_all_in_bounds_sweep() {
    // The `1` must move by exactly one line per index; a fencepost error in the
    // translation would show up as two rows being identical.
    let mut seen = Vec::new();
    for i in 0..10 {
        let out = diff_bad(i);
        assert_eq!(out, slot_pattern(i as usize));
        assert!(!seen.contains(&out), "bad({i}) produced a duplicate pattern");
        seen.push(out);
    }
    assert_eq!(seen.len(), 10);
}

#[test]
fn cfg_18_bad_repeated_calls_reinitialize() {
    // `int buffer[10] = {0}` is a fresh automatic, not `static`: repeated calls
    // must not accumulate `1`s.
    let out = diff(
        "bad(3) x5 then bad(6) x2 then bad(3)",
        || unsafe {
            let f = bad::c();
            for _ in 0..5 {
                f(3);
            }
            f(6);
            f(6);
            f(3);
        },
        || unsafe {
            let f = bad::rs();
            for _ in 0..5 {
                f(3);
            }
            f(6);
            f(6);
            f(3);
        },
    );
    let mut want = Vec::new();
    for _ in 0..5 {
        want.extend_from_slice(&slot_pattern(3));
    }
    want.extend_from_slice(&slot_pattern(6));
    want.extend_from_slice(&slot_pattern(6));
    want.extend_from_slice(&slot_pattern(3));
    assert_eq!(out, want, "buffer must be re-zeroed on every call");
}

// ===========================================================================
// Rows 19-23: `good` — mid level (drives the two `static` helpers)
// ===========================================================================

/// `good(d)` for valid `d` = goodG2B's fixed index-7 pattern, then goodB2G's.
fn good_pattern(d: usize) -> Vec<u8> {
    let mut v = slot_pattern(7);
    v.extend_from_slice(&slot_pattern(d));
    v
}

#[test]
fn cfg_19_good_index_zero() {
    let out = diff_good(0);
    assert_eq!(out, good_pattern(0));
}

#[test]
fn cfg_20_good_index_seven_same_as_g2b() {
    let out = diff_good(7);
    assert_eq!(out, good_pattern(7));
    // Both halves are the same 10 lines here, so the total must be that pattern
    // twice — catches a translation that accidentally calls one helper twice or
    // drops one of them.
    assert_eq!(out.len(), slot_pattern(7).len() * 2);
}

#[test]
fn cfg_21_good_index_nine() {
    let out = diff_good(9);
    assert_eq!(out, good_pattern(9));
}

#[test]
fn cfg_22_good_all_in_bounds_sweep() {
    for d in 0..10usize {
        let out = diff_good(d as i32);
        assert_eq!(out, good_pattern(d), "good({d})");
        // goodG2B's half is invariant (its `data` is hardcoded 7).
        assert_eq!(&out[..slot_pattern(7).len()], &slot_pattern(7)[..]);
    }
}

#[test]
fn cfg_23_good_repeated_calls() {
    let out = diff(
        "good(2) x3, good(8), good(2)",
        || unsafe {
            let f = good::c();
            for _ in 0..3 {
                f(2);
            }
            f(8);
            f(2);
        },
        || unsafe {
            let f = good::rs();
            for _ in 0..3 {
                f(2);
            }
            f(8);
            f(2);
        },
    );
    let mut want = Vec::new();
    for _ in 0..3 {
        want.extend_from_slice(&good_pattern(2));
    }
    want.extend_from_slice(&good_pattern(8));
    want.extend_from_slice(&good_pattern(2));
    assert_eq!(out, want);
}

// ===========================================================================
// Rows 24-29: `driver` — the composed wrapper, and interleaving
// ===========================================================================

fn driver_pattern(g: i32, b: i32) -> Vec<u8> {
    let mut v = b"Calling good()...\n".to_vec();
    v.extend_from_slice(&slot_pattern(7));
    if (0..10).contains(&g) {
        v.extend_from_slice(&slot_pattern(g as usize));
    } else {
        v.extend_from_slice(b"ERROR: Array index is out-of-bounds\n");
    }
    v.extend_from_slice(b"Finished good()\n");
    v.extend_from_slice(b"Calling bad()...\n");
    if b < 0 {
        v.extend_from_slice(b"ERROR: Array index is negative.\n");
    } else if b < 10 {
        v.extend_from_slice(&slot_pattern(b as usize));
    } else {
        v.extend_from_slice(&all_zero_pattern());
    }
    v.extend_from_slice(b"Finished bad()\n");
    v
}

#[test]
fn cfg_24_driver_both_valid_equal() {
    for d in [0, 3, 7, 9] {
        let out = diff_driver(d, d);
        assert_eq!(out, driver_pattern(d, d), "driver({d}, {d})");
    }
}

#[test]
fn cfg_25_driver_valid_distinct_args() {
    // Asymmetric args: a swapped/aliased parameter would still pass row 24.
    for (g, b) in [(0, 9), (9, 0), (1, 8), (2, 5), (4, 6)] {
        let out = diff_driver(g, b);
        assert_eq!(out, driver_pattern(g, b), "driver({g}, {b})");
    }
}

#[test]
fn cfg_26_driver_full_cross_product_valid() {
    for g in 0..10 {
        for b in 0..10 {
            let out = diff_driver(g, b);
            assert_eq!(out, driver_pattern(g, b), "driver({g}, {b})");
        }
    }
}

#[test]
fn cfg_27_driver_cross_product_mixed() {
    const AXIS: [i32; 7] = [-1, 0, 5, 9, 10, i32::MIN, i32::MAX];
    for g in AXIS {
        for b in AXIS {
            // `bad` has no upper-bound check, so any `badData >= 12` corrupts the
            // caller's frame (see `BAD_OOB_INFRAME`). Map high values onto the
            // largest in-frame overrun instead.
            let b = if b >= 10 { 11 } else { b };
            let out = diff_driver(g, b);
            assert_eq!(out, driver_pattern(g, b), "driver({g}, {b})");
        }
    }
}

#[test]
fn cfg_28_driver_random_pairs() {
    let mut rng = Rng::new(SEED ^ 0x3333);
    for _ in 0..1024 {
        let g = mixed_index(&mut rng);
        let b = mixed_index_bad_safe(&mut rng);
        let out = diff_driver(g, b);
        assert_eq!(out, driver_pattern(g, b), "driver({g}, {b})");
    }
}

#[test]
fn cfg_29_interleaved_all_entry_points() {
    // One randomized script replayed against each library inside a single
    // capture. Catches ordering / residual-state / flush divergences that
    // per-call tests cannot see.
    #[derive(Clone, Debug)]
    enum Step {
        Line(Vec<u8>),
        LineNull,
        Int(i32),
        Bad(i32),
        Good(i32),
        Driver(i32, i32),
    }

    let mut rng = Rng::new(SEED ^ 0x4444);
    let mut script = Vec::new();
    for _ in 0..512 {
        script.push(match rng.below(6) {
            0 => {
                let n = rng.below(40) as usize;
                Step::Line(rng.bytes(n))
            }
            1 => Step::LineNull,
            2 => Step::Int(rng.next_i32()),
            3 => Step::Bad(mixed_index_bad_safe(&mut rng)),
            4 => Step::Good(mixed_index(&mut rng)),
            _ => Step::Driver(mixed_index(&mut rng), mixed_index_bad_safe(&mut rng)),
        });
    }

    // Pre-build NUL-terminated buffers so both replays use identical pointers.
    let bufs: Vec<Vec<u8>> = script
        .iter()
        .map(|s| match s {
            Step::Line(b) => {
                let mut v = b.clone();
                v.push(0);
                v
            }
            _ => Vec::new(),
        })
        .collect();

    let replay = |use_c: bool| {
        let (pl, pil, fb, fg, fd) = if use_c {
            (print_line::c(), print_int_line::c(), bad::c(), good::c(), driver::c())
        } else {
            (print_line::rs(), print_int_line::rs(), bad::rs(), good::rs(), driver::rs())
        };
        for (i, step) in script.iter().enumerate() {
            unsafe {
                match step {
                    Step::Line(_) => pl(bufs[i].as_ptr() as *const std::ffi::c_char),
                    Step::LineNull => pl(std::ptr::null()),
                    Step::Int(v) => pil(*v),
                    Step::Bad(v) => fb(*v),
                    Step::Good(v) => fg(*v),
                    Step::Driver(g, b) => fd(*g, *b),
                }
            }
        }
    };

    let out = diff("interleaved 512-step script", || replay(true), || replay(false));
    assert!(!out.is_empty());

    // Independently reconstruct the expected stream from the C source's logic.
    let mut want: Vec<u8> = Vec::new();
    for (i, step) in script.iter().enumerate() {
        match step {
            Step::Line(b) => {
                want.extend_from_slice(b);
                want.push(b'\n');
                let _ = &bufs[i];
            }
            Step::LineNull => {}
            Step::Int(v) => want.extend_from_slice(format!("{v}\n").as_bytes()),
            Step::Bad(v) => {
                if *v < 0 {
                    want.extend_from_slice(b"ERROR: Array index is negative.\n");
                } else if *v < 10 {
                    want.extend_from_slice(&slot_pattern(*v as usize));
                } else {
                    want.extend_from_slice(&all_zero_pattern());
                }
            }
            Step::Good(v) => {
                want.extend_from_slice(&slot_pattern(7));
                if (0..10).contains(v) {
                    want.extend_from_slice(&slot_pattern(*v as usize));
                } else {
                    want.extend_from_slice(b"ERROR: Array index is out-of-bounds\n");
                }
            }
            Step::Driver(g, b) => want.extend_from_slice(&driver_pattern(*g, *b)),
        }
    }
    assert_eq!(out, want, "interleaved stream must match the C source's logic");
}

// ===========================================================================
// Row 30: the out-of-bounds-write shape (CWE-787), reproduced not fixed
// ===========================================================================

#[test]
fn cfg_30_bad_oob_write_shape() {
    // `bad` checks only `data >= 0`, so these perform an out-of-bounds write and
    // then print the ten (still zero) in-bounds elements.
    //
    // Only `BAD_OOB_INFRAME` (= [10, 11]) is compared: those two writes land in
    // `bad`'s own frame padding / its immediately-reinitialised `i` slot. From
    // `buffer[12]` on, the compiled C clobbers its saved `%rbp` and return
    // address and takes SIGSEGV, which is caller-dependent UB no translation can
    // mirror. See `BAD_OOB_INFRAME`'s docs for the frame layout.
    for &d in BAD_OOB_INFRAME {
        let out = diff_bad(d);
        assert_eq!(out, all_zero_pattern(), "bad({d}) must print ten zeros");
    }
    // Many repetitions, and interleaved with in-bounds calls, so a lingering
    // effect of the overrun would surface.
    let mut rng = Rng::new(SEED ^ 0x5555);
    for _ in 0..256 {
        let d = BAD_OOB_INFRAME[rng.below(BAD_OOB_INFRAME.len() as u64) as usize];
        let out = diff_bad(d);
        assert_eq!(out, all_zero_pattern(), "bad({d})");
        let k = rng.range_i32(0, 9);
        let out = diff_bad(k);
        assert_eq!(out, slot_pattern(k as usize), "bad({k}) after an overrun");
    }
}

#[test]
fn aaa_00_harness_must_be_single_threaded() {
    assert_single_threaded();
}
