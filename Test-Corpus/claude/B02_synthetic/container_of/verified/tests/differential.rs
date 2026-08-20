//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads **both** shared objects
//! (the untouched C ground truth and the Rust translation's `cdylib`) with
//! `libloading` and compares what their exported symbols produce, byte for byte.
//! Nothing is ever called directly in-process, so the `#[no_mangle]` wrappers
//! are exercised exactly as an external C consumer would exercise them.

mod common;

use std::ffi::c_int;

use common::{assert_main_matches, pretty, run_main, Argv, CTest, Pair, Rng, Sink};

const SEED: u64 = 0x00c0_ffee_1234_5678;

// ---------------------------------------------------------------------------
// Pointer corpora shared by the `find_container_of_*` rows.
// ---------------------------------------------------------------------------

/// Addresses the C pointer arithmetic treats specially: around zero (where the
/// `-4` of `find_container_of_b` wraps), around page and word boundaries, and at
/// the top of the address space.
fn boundary_addresses() -> Vec<usize> {
    let mut v = vec![
        0usize,
        1,
        2,
        3,
        4,
        5,
        7,
        8,
        15,
        16,
        4095,
        4096,
        4097,
        0x7FFF_FFFF,
        0x8000_0000,
        0x1_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        0x8000_0000_0000_0000,
        usize::MAX - 4,
        usize::MAX - 3,
        usize::MAX - 2,
        usize::MAX - 1,
        usize::MAX,
    ];
    v.dedup();
    v
}

fn random_addresses(count: usize, seed: u64) -> Vec<usize> {
    let mut rng = Rng::new(seed);
    (0..count).map(|_| rng.next_u64() as usize).collect()
}

/// Compares one exported `find_container_of_*` across both libraries for a whole
/// address corpus. Nothing is dereferenced, so any address is fair game — the C
/// function only does arithmetic.
fn compare_find(pair: &Pair, which: char, addresses: &[usize]) {
    let (c_fn, r_fn) = match which {
        'a' => (pair.c.find_container_of_a(), pair.rust.find_container_of_a()),
        'b' => (pair.c.find_container_of_b(), pair.rust.find_container_of_b()),
        _ => unreachable!(),
    };

    for &addr in addresses {
        let c_out = unsafe { c_fn(addr as *mut c_int) } as usize;
        let r_out = unsafe { r_fn(addr as *mut c_int) } as usize;
        assert_eq!(
            c_out, r_out,
            "find_container_of_{which}({addr:#018x}): C returned {c_out:#018x}, Rust returned {r_out:#018x}"
        );
    }
}

// --- Row 1 / 2 --------------------------------------------------------------

#[test]
fn row01_find_a_random_pointers() {
    let pair = Pair::default_config();
    compare_find(&pair, 'a', &random_addresses(4096, SEED));
}

#[test]
fn row02_find_a_boundary_pointers() {
    let pair = Pair::default_config();
    compare_find(&pair, 'a', &boundary_addresses());
}

// --- Row 3 ------------------------------------------------------------------

#[test]
fn row03_find_a_live_objects() {
    let pair = Pair::default_config();
    let c_fn = pair.c.find_container_of_a();
    let r_fn = pair.rust.find_container_of_a();
    let mut rng = Rng::new(SEED ^ 3);

    // A stack object, a boxed object, and elements of arrays of several
    // lengths, so the recovered pointer is checked at many alignments.
    for len in [1usize, 2, 3, 8, 64] {
        let mut objects: Vec<CTest> = (0..len)
            .map(|_| CTest {
                a: rng.next_i32(),
                b: rng.next_i32(),
            })
            .collect();

        for idx in 0..len {
            let expected = &mut objects[idx] as *mut CTest;
            let field = unsafe { &mut (*expected).a as *mut c_int };

            let c_out = unsafe { c_fn(field) };
            let r_out = unsafe { r_fn(field) };
            assert_eq!(c_out as usize, r_out as usize, "pointer mismatch at idx {idx}");
            assert_eq!(c_out as usize, expected as usize, "C did not recover &t");

            // The recovered pointer must be usable: read the member back.
            let c_val = unsafe { (*c_out).a };
            let r_val = unsafe { (*r_out).a };
            assert_eq!(c_val, r_val);
            assert_eq!(c_val, objects[idx].a);
        }
    }

    let mut boxed = Box::new(CTest { a: -7, b: 99 });
    let field = &mut boxed.a as *mut c_int;
    assert_eq!(unsafe { c_fn(field) } as usize, unsafe { r_fn(field) } as usize);
}

// --- Row 4 / 5 --------------------------------------------------------------

#[test]
fn row04_find_b_random_pointers() {
    let pair = Pair::default_config();
    compare_find(&pair, 'b', &random_addresses(4096, SEED ^ 0xB));
}

#[test]
fn row05_find_b_boundary_pointers() {
    let pair = Pair::default_config();
    compare_find(&pair, 'b', &boundary_addresses());
}

// --- Row 6 ------------------------------------------------------------------

#[test]
fn row06_find_b_live_objects() {
    let pair = Pair::default_config();
    let c_fn = pair.c.find_container_of_b();
    let r_fn = pair.rust.find_container_of_b();
    let mut rng = Rng::new(SEED ^ 6);

    for len in [1usize, 2, 3, 8, 64] {
        let mut objects: Vec<CTest> = (0..len)
            .map(|_| CTest {
                a: rng.next_i32(),
                b: rng.next_i32(),
            })
            .collect();

        for idx in 0..len {
            let expected = &mut objects[idx] as *mut CTest;
            let field = unsafe { &mut (*expected).b as *mut c_int };

            let c_out = unsafe { c_fn(field) };
            let r_out = unsafe { r_fn(field) };
            assert_eq!(c_out as usize, r_out as usize, "pointer mismatch at idx {idx}");
            assert_eq!(c_out as usize, expected as usize, "C did not recover &t");

            let c_val = unsafe { (*c_out).b };
            let r_val = unsafe { (*r_out).b };
            assert_eq!(c_val, r_val);
            assert_eq!(c_val, objects[idx].b);
        }
    }
}

// --- Row 7 ------------------------------------------------------------------

#[test]
fn row07_composed_container_of_invariant() {
    let pair = Pair::default_config();
    let (c_a, c_b) = (pair.c.find_container_of_a(), pair.c.find_container_of_b());
    let (r_a, r_b) = (
        pair.rust.find_container_of_a(),
        pair.rust.find_container_of_b(),
    );
    let mut rng = Rng::new(SEED ^ 7);

    for _ in 0..512 {
        // Randomised allocation shape so the object lands at varied addresses.
        let pad = rng.below(4) as usize;
        let mut storage: Vec<CTest> = (0..pad + 1)
            .map(|_| CTest {
                a: rng.next_i32(),
                b: rng.next_i32(),
            })
            .collect();
        let t = &mut storage[pad] as *mut CTest;

        let pa = unsafe { &mut (*t).a as *mut c_int };
        let pb = unsafe { &mut (*t).b as *mut c_int };

        let c_from_a = unsafe { c_a(pa) };
        let c_from_b = unsafe { c_b(pb) };
        let r_from_a = unsafe { r_a(pa) };
        let r_from_b = unsafe { r_b(pb) };

        assert_eq!(c_from_a as usize, r_from_a as usize);
        assert_eq!(c_from_b as usize, r_from_b as usize);
        assert_eq!(c_from_a as usize, c_from_b as usize, "the invariant main relies on");
        assert_eq!(c_from_a as usize, t as usize);

        // The exact expression `main` evaluates.
        let c_sum = unsafe { (*c_from_a).a.wrapping_add((*c_from_b).b) };
        let r_sum = unsafe { (*r_from_a).a.wrapping_add((*r_from_b).b) };
        assert_eq!(c_sum, r_sum);
    }
}

// ---------------------------------------------------------------------------
// Argument corpora for the `main` rows.
// ---------------------------------------------------------------------------

const SPACES: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];

fn plain_decimal(rng: &mut Rng) -> Vec<u8> {
    format!("{}", rng.next_i32()).into_bytes()
}

fn whitespace_prefixed(rng: &mut Rng) -> Vec<u8> {
    let mut s = Vec::new();
    let n = rng.below(5) + 1;
    for _ in 0..n {
        s.push(*rng.pick(&SPACES));
    }
    s.extend_from_slice(&plain_decimal(rng));
    if rng.bool() {
        // Trailing whitespace is *not* part of the subject sequence.
        s.push(*rng.pick(&SPACES));
    }
    s
}

fn signed_with_zeros(rng: &mut Rng) -> Vec<u8> {
    let mut s = Vec::new();
    match rng.below(3) {
        0 => s.push(b'+'),
        1 => s.push(b'-'),
        _ => {}
    }
    for _ in 0..rng.below(6) {
        s.push(b'0');
    }
    s.extend_from_slice(format!("{}", rng.next_u32() % 100_000).as_bytes());
    s
}

fn trailing_garbage(rng: &mut Rng) -> Vec<u8> {
    let mut s = plain_decimal(rng);
    let tails: [&[u8]; 8] = [
        b"abc", b".9", b" 8", b"-", b"+3", b"e5", b"\x00zz", b"_",
    ];
    let tail = *rng.pick(&tails);
    // `\x00` cannot be embedded in a real argument; use a plain letter instead.
    if tail.contains(&0) {
        s.extend_from_slice(b"z");
    } else {
        s.extend_from_slice(tail);
    }
    s
}

fn no_digits(rng: &mut Rng) -> Vec<u8> {
    let fixed: [&[u8]; 12] = [
        b"", b" ", b"abc", b"+", b"-", b"++5", b"+-5", b"- 5", b"0x", b"@!#", b"\x80\xff", b"\t\t",
    ];
    if rng.bool() {
        return rng.pick(&fixed).to_vec();
    }
    // Random junk guaranteed to contain no ASCII digit before any digit could
    // start a subject sequence.
    let n = rng.below(8) + 1;
    (0..n)
        .map(|_| {
            let mut b = (rng.below(255) + 1) as u8;
            while b.is_ascii_digit() {
                b = (rng.below(255) + 1) as u8;
            }
            b
        })
        .collect()
}

fn huge_digits(rng: &mut Rng) -> Vec<u8> {
    let mut s = Vec::new();
    if rng.bool() {
        s.push(if rng.bool() { b'-' } else { b'+' });
    }
    for _ in 0..rng.below(4) {
        s.push(b'0');
    }
    let n = rng.below(182) + 19; // 19..=200 digits
    for i in 0..n {
        let d = if i == 0 {
            (rng.below(9) + 1) as u8
        } else {
            rng.below(10) as u8
        };
        s.push(b'0' + d);
    }
    s
}

fn fuzz_bytes(rng: &mut Rng) -> Vec<u8> {
    let n = rng.below(25);
    (0..n).map(|_| (rng.below(255) + 1) as u8).collect()
}

// --- Row 8 ------------------------------------------------------------------

#[test]
fn row08_main_plain_decimals() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..400 {
        let a = plain_decimal(&mut rng);
        let b = plain_decimal(&mut rng);
        assert_main_matches(&pair, &[b"driver", &a, &b], Sink::Pipe);
    }
}

// --- Row 9 ------------------------------------------------------------------

#[test]
fn row09_main_leading_whitespace() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 9);

    // Every isspace byte on its own, in both positions.
    for s in SPACES {
        let arg = [&[s][..], b"42"].concat();
        assert_main_matches(&pair, &[b"driver", &arg, b"1"], Sink::Pipe);
        assert_main_matches(&pair, &[b"driver", b"1", &arg], Sink::Pipe);
    }

    for _ in 0..400 {
        let a = whitespace_prefixed(&mut rng);
        let b = whitespace_prefixed(&mut rng);
        assert_main_matches(&pair, &[b"driver", &a, &b], Sink::Pipe);
    }
}

// --- Row 10 -----------------------------------------------------------------

#[test]
fn row10_main_signs_and_leading_zeros() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 10);

    for fixed in [
        &b"+0"[..], b"-0", b"0000", b"+000012", b"-000012", b"0000000000000000000000001",
    ] {
        assert_main_matches(&pair, &[b"driver", fixed, b"0"], Sink::Pipe);
    }

    for _ in 0..400 {
        let a = signed_with_zeros(&mut rng);
        let b = signed_with_zeros(&mut rng);
        assert_main_matches(&pair, &[b"driver", &a, &b], Sink::Pipe);
    }
}

// --- Row 11 -----------------------------------------------------------------

#[test]
fn row11_main_trailing_garbage() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..400 {
        let a = trailing_garbage(&mut rng);
        let b = trailing_garbage(&mut rng);
        assert_main_matches(&pair, &[b"driver", &a, &b], Sink::Pipe);
    }
}

// --- Row 12 -----------------------------------------------------------------

#[test]
fn row12_main_no_digits() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..400 {
        let a = no_digits(&mut rng);
        let b = no_digits(&mut rng);
        assert_main_matches(&pair, &[b"driver", &a, &b], Sink::Pipe);
    }
}

// --- Row 13 -----------------------------------------------------------------

#[test]
fn row13_main_empty_arguments() {
    let pair = Pair::default_config();
    assert_main_matches(&pair, &[b"driver", b"", b"7"], Sink::Pipe);
    assert_main_matches(&pair, &[b"driver", b"7", b""], Sink::Pipe);
    assert_main_matches(&pair, &[b"driver", b"", b""], Sink::Pipe);
    assert_main_matches(&pair, &[b"", b"", b""], Sink::Pipe);
}

// --- Row 14 -----------------------------------------------------------------

const INT_EDGES: [i32; 9] = [
    i32::MIN,
    i32::MIN + 1,
    -2,
    -1,
    0,
    1,
    2,
    i32::MAX - 1,
    i32::MAX,
];

#[test]
fn row14_main_int_boundary_cross_product() {
    let pair = Pair::default_config();
    for a in INT_EDGES {
        for b in INT_EDGES {
            let sa = a.to_string().into_bytes();
            let sb = b.to_string().into_bytes();
            assert_main_matches(&pair, &[b"driver", &sa, &sb], Sink::Pipe);
        }
    }
}

// --- Row 15 -----------------------------------------------------------------

const NUMERIC_EDGE_STRINGS: [&[u8]; 18] = [
    b"2147483647",
    b"2147483648",
    b"2147483649",
    b"-2147483648",
    b"-2147483649",
    b"4294967295",
    b"4294967296",
    b"4294967297",
    b"-4294967296",
    b"9223372036854775806",
    b"9223372036854775807",
    b"9223372036854775808",
    b"9223372036854775809",
    b"-9223372036854775807",
    b"-9223372036854775808",
    b"-9223372036854775809",
    b"18446744073709551616",
    b"-18446744073709551616",
];

#[test]
fn row15_main_long_boundary_cross_product() {
    let pair = Pair::default_config();
    for a in NUMERIC_EDGE_STRINGS {
        for b in NUMERIC_EDGE_STRINGS {
            assert_main_matches(&pair, &[b"driver", a, b], Sink::Pipe);
        }
    }
}

// --- Row 16 -----------------------------------------------------------------

#[test]
fn row16_main_huge_digit_strings() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..300 {
        let a = huge_digits(&mut rng);
        let b = huge_digits(&mut rng);
        assert_main_matches(&pair, &[b"driver", &a, &b], Sink::Pipe);
    }
}

// --- Row 17 -----------------------------------------------------------------

#[test]
fn row17_main_unstructured_fuzz() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..600 {
        let a = fuzz_bytes(&mut rng);
        let b = fuzz_bytes(&mut rng);
        assert_main_matches(&pair, &[b"driver", &a, &b], Sink::Pipe);
    }
}

// --- Row 18 -----------------------------------------------------------------

#[test]
fn row18_main_extra_arguments_ignored() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..200 {
        let extra_count = rng.below(7) + 1; // 1..=7 extra arguments
        let owned: Vec<Vec<u8>> = (0..extra_count + 3)
            .map(|i| {
                if i == 0 {
                    b"driver".to_vec()
                } else if rng.bool() {
                    plain_decimal(&mut rng)
                } else {
                    fuzz_bytes(&mut rng)
                }
            })
            .collect();
        let refs: Vec<&[u8]> = owned.iter().map(|v| v.as_slice()).collect();
        assert_main_matches(&pair, &refs, Sink::Pipe);
    }
}

// --- Row 19 -----------------------------------------------------------------

#[test]
fn row19_main_argc_is_never_read() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 19);
    let bogus_argcs: [c_int; 8] = [0, 1, 2, 3, 4, 99, -1, c_int::MAX];

    for &argc in &bogus_argcs {
        for _ in 0..16 {
            let a = plain_decimal(&mut rng);
            let b = plain_decimal(&mut rng);
            let args: [&[u8]; 3] = [b"driver", &a, &b];

            let mut c_argv = Argv::new(&args);
            let mut r_argv = Argv::new(&args);
            let (c_rc, c_out) = run_main(&pair.c, argc, &mut c_argv, Sink::Pipe);
            let (r_rc, r_out) = run_main(&pair.rust, argc, &mut r_argv, Sink::Pipe);

            assert_eq!(c_rc, r_rc, "argc={argc} argv={:?}", pretty(&args));
            assert_eq!(
                c_out,
                r_out,
                "argc={argc} argv={:?}: C={:?} Rust={:?}",
                pretty(&args),
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
        }
    }
}

// --- Row 20 -----------------------------------------------------------------

#[test]
fn row20_main_repeated_invocations() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 20);

    // Same loaded libraries, 256 calls each: no state may leak between calls
    // and no output may be lost or duplicated by buffering.
    for _ in 0..256 {
        let a = plain_decimal(&mut rng);
        let b = plain_decimal(&mut rng);
        assert_main_matches(&pair, &[b"driver", &a, &b], Sink::Pipe);
    }

    // A single capture containing many calls in a row, to compare the
    // concatenated stream rather than one line at a time.
    let args: Vec<(Vec<u8>, Vec<u8>)> = (0..64)
        .map(|_| (plain_decimal(&mut rng), plain_decimal(&mut rng)))
        .collect();

    let mut collected = Vec::new();
    for imp in [&pair.c, &pair.rust] {
        let entry = imp.main();
        let out = common::capture_stdout(Sink::Pipe, || {
            for (a, b) in &args {
                let mut argv = Argv::new(&[b"driver", a, b]);
                let argc = argv.argc();
                unsafe { entry(argc, argv.as_ptr()) };
            }
        });
        collected.push(out);
    }
    assert_eq!(
        collected[0],
        collected[1],
        "concatenated stream mismatch:\nC   ={:?}\nRust={:?}",
        String::from_utf8_lossy(&collected[0]),
        String::from_utf8_lossy(&collected[1])
    );
}

// --- Row 21 -----------------------------------------------------------------

#[test]
fn row21_find_functions_against_o2_reference() {
    let pair = Pair::o2_config();
    compare_find(&pair, 'a', &random_addresses(2048, SEED ^ 21));
    compare_find(&pair, 'a', &boundary_addresses());
    compare_find(&pair, 'b', &random_addresses(2048, SEED ^ 0x21));
    compare_find(&pair, 'b', &boundary_addresses());
}

// --- Row 22 -----------------------------------------------------------------

#[test]
fn row22_main_against_o2_reference() {
    let pair = Pair::o2_config();
    let mut rng = Rng::new(SEED ^ 22);

    for a in INT_EDGES {
        for b in INT_EDGES {
            let sa = a.to_string().into_bytes();
            let sb = b.to_string().into_bytes();
            assert_main_matches(&pair, &[b"driver", &sa, &sb], Sink::Pipe);
        }
    }
    for a in NUMERIC_EDGE_STRINGS {
        for b in NUMERIC_EDGE_STRINGS {
            assert_main_matches(&pair, &[b"driver", a, b], Sink::Pipe);
        }
    }
    for _ in 0..200 {
        let (a, b) = match rng.below(6) {
            0 => (plain_decimal(&mut rng), plain_decimal(&mut rng)),
            1 => (whitespace_prefixed(&mut rng), signed_with_zeros(&mut rng)),
            2 => (trailing_garbage(&mut rng), no_digits(&mut rng)),
            3 => (huge_digits(&mut rng), huge_digits(&mut rng)),
            4 => (fuzz_bytes(&mut rng), fuzz_bytes(&mut rng)),
            _ => (no_digits(&mut rng), plain_decimal(&mut rng)),
        };
        assert_main_matches(&pair, &[b"driver", &a, &b], Sink::Pipe);
    }
}

// --- Row 26 -----------------------------------------------------------------

#[test]
fn row26_main_very_long_arguments() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 26);

    for len in [1000usize, 10_000, 100_000] {
        // All digits: `strtol` must scan the whole run and still saturate.
        let mut digits = Vec::with_capacity(len + 1);
        digits.push(b'1');
        for _ in 1..len {
            digits.push(b'0' + rng.below(10) as u8);
        }
        assert_main_matches(&pair, &[b"driver", &digits, b"1"], Sink::Pipe);

        let mut negative = vec![b'-'];
        negative.extend_from_slice(&digits);
        assert_main_matches(&pair, &[b"driver", &negative, b"-1"], Sink::Pipe);

        // A long run of leading zeros followed by a small value: no overflow.
        let mut zeros = vec![b'0'; len];
        zeros.extend_from_slice(b"123");
        assert_main_matches(&pair, &[b"driver", &zeros, b"1"], Sink::Pipe);

        // A long whitespace run in front of a value.
        let mut spaces: Vec<u8> = (0..len).map(|_| *rng.pick(&SPACES)).collect();
        spaces.extend_from_slice(b"-99");
        assert_main_matches(&pair, &[b"driver", &spaces, b"1"], Sink::Pipe);

        // Long garbage: no digits at all.
        let junk = vec![b'q'; len];
        assert_main_matches(&pair, &[b"driver", &junk, b"1"], Sink::Pipe);
    }
}

// --- Row 27 -----------------------------------------------------------------

#[test]
fn row27_main_does_not_modify_its_inputs() {
    // The C `main` only reads `argv`; neither implementation may write to the
    // argument strings or to the pointer array.
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 27);

    for _ in 0..200 {
        let a = plain_decimal(&mut rng);
        let b = whitespace_prefixed(&mut rng);
        let args: [&[u8]; 3] = [b"driver", &a, &b];

        for imp in [&pair.c, &pair.rust] {
            let mut argv = Argv::new(&args);
            let argc = argv.argc();
            let before: Vec<Vec<u8>> = argv.snapshot();
            let _ = run_main(imp, argc, &mut argv, Sink::Pipe);
            let after: Vec<Vec<u8>> = argv.snapshot();
            assert_eq!(
                before,
                after,
                "{} modified its argv for {:?}",
                imp.name,
                pretty(&args)
            );
        }
    }
}

// --- Row 28 -----------------------------------------------------------------

#[test]
fn row28_printf_decimal_rendering_boundaries() {
    // `printf("%d\n", …)` is the one piece of C the Rust side reimplements from
    // scratch rather than transliterating, so its output is checked at every
    // value where the number of emitted characters changes, plus a random sample.
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 28);

    let mut values: Vec<i32> = vec![0, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    let mut power: i64 = 1;
    while power <= 1_000_000_000 {
        for delta in [-1i64, 0, 1] {
            let v = power + delta;
            if v <= i32::MAX as i64 {
                values.push(v as i32);
            }
            if -v >= i32::MIN as i64 {
                values.push(-v as i32);
            }
        }
        power *= 10;
    }
    for _ in 0..500 {
        values.push(rng.next_i32());
    }

    for v in values {
        // `main` prints a + b, so pass the value and zero to render exactly `v`.
        let sa = v.to_string().into_bytes();
        let (_, out) = {
            let mut c_argv = Argv::new(&[b"driver", &sa, b"0"]);
            let mut r_argv = Argv::new(&[b"driver", &sa, b"0"]);
            let argc = c_argv.argc();
            let (c_rc, c_out) = run_main(&pair.c, argc, &mut c_argv, Sink::Pipe);
            let (r_rc, r_out) = run_main(&pair.rust, argc, &mut r_argv, Sink::Pipe);
            assert_eq!(c_rc, r_rc, "return value mismatch for {v}");
            assert_eq!(
                c_out,
                r_out,
                "rendering mismatch for {v}: C={:?} Rust={:?}",
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
            (c_rc, c_out)
        };
        // And the rendering really is the plain decimal form plus one newline.
        assert_eq!(out, format!("{v}\n").into_bytes(), "unexpected text for {v}");
    }
}

// --- Row 23 -----------------------------------------------------------------

#[test]
fn row23_main_output_across_stdout_kinds() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 23);

    for sink in [Sink::File, Sink::Pipe] {
        for a in INT_EDGES {
            let sa = a.to_string().into_bytes();
            assert_main_matches(&pair, &[b"driver", &sa, b"1"], sink);
        }
        for _ in 0..150 {
            let a = plain_decimal(&mut rng);
            let b = plain_decimal(&mut rng);
            assert_main_matches(&pair, &[b"driver", &a, &b], sink);
        }
    }
}
