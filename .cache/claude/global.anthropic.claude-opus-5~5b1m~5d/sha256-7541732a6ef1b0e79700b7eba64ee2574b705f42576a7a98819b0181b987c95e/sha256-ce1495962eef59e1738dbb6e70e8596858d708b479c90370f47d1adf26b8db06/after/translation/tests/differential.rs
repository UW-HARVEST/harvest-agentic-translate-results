//! Phase B (valid-path) + Phase C (error/boundary-path) differential tests.
//!
//! Every test loads BOTH `c_src/build/libdriver.so` and
//! `translation/target/{release,debug}/libdriver.so` with `libloading` and
//! compares the bytes each writes to stdout. One test per row of `CONFIGS.md`
//! and `ERRORS.md`.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Phase B — CONFIGS.md rows
// ---------------------------------------------------------------------------

/// C1 — the all-zero bit pattern.
fn cfg_zero() {
    assert_same_all("C1 zero", &[0]);
}

/// C2 — every value that occupies only byte 0 (`%02x` zero-padding sweep).
fn cfg_low_byte_sweep() {
    let xs: Vec<c_int> = (0..=255).collect();
    assert_same_all("C2 byte0 sweep", &xs);
}

/// C3 — every value that occupies only byte 1.
fn cfg_byte1_sweep() {
    let xs: Vec<c_int> = (0..=255).map(|n: i32| n << 8).collect();
    assert_same_all("C3 byte1 sweep", &xs);
}

/// C4 — every value that occupies only byte 2.
fn cfg_byte2_sweep() {
    let xs: Vec<c_int> = (0..=255).map(|n: i32| n << 16).collect();
    assert_same_all("C4 byte2 sweep", &xs);
}

/// C5 — every value that occupies only byte 3 (includes the sign bit).
fn cfg_byte3_sweep() {
    let xs: Vec<c_int> = (0..=255u32).map(|n| (n << 24) as c_int).collect();
    assert_same_all("C5 byte3 sweep", &xs);
}

/// C6 — all four bytes equal and >= 0x80: worst case for a signed-char bug.
fn cfg_all_bytes_high() {
    let xs: Vec<c_int> = (0x80..=0xffu32)
        .map(|b| (b | (b << 8) | (b << 16) | (b << 24)) as c_int)
        .collect();
    assert_same_all("C6 all bytes high", &xs);
}

/// C7 — randomized negatives over INT_MIN..=-1.
fn cfg_negative_random() {
    let mut rng = Rng::new();
    let xs: Vec<c_int> = (0..512)
        .map(|_| rng.range_i32(i32::MIN as i64, -1))
        .collect();
    assert!(xs.iter().all(|&x| x < 0));
    assert_same_all("C7 negative random", &xs);
}

/// C8 — randomized positives over 0..=INT_MAX.
fn cfg_positive_random() {
    let mut rng = Rng::seeded(SEED ^ 0xA5A5_A5A5);
    let xs: Vec<c_int> = (0..512)
        .map(|_| rng.range_i32(0, i32::MAX as i64))
        .collect();
    assert!(xs.iter().all(|&x| x >= 0));
    assert_same_all("C8 positive random", &xs);
}

/// C9 — 2048 randomized full-range bit patterns.
fn cfg_full_range_random() {
    let mut rng = Rng::seeded(SEED ^ 0x1234_5678);
    let xs: Vec<c_int> = (0..2048).map(|_| rng.next_i32()).collect();
    assert_same_all("C9 full range random", &xs);
}

/// C10 — powers of two, their negations and off-by-ones (byte-carry boundaries).
fn cfg_powers_of_two() {
    let mut xs: Vec<c_int> = Vec::new();
    for k in 0..32u32 {
        let p = 1u32 << k;
        xs.push(p as c_int);
        xs.push((p as i32).wrapping_neg());
        xs.push(p.wrapping_sub(1) as c_int);
        xs.push(p.wrapping_add(1) as c_int);
        xs.push(!p as c_int);
    }
    assert_same_all("C10 powers of two", &xs);
}

/// C11 — four distinct bytes: pins the byte order of the memory dump.
fn cfg_endianness_witnesses() {
    let xs: Vec<c_int> = [
        0x0102_0304u32,
        0x0403_0201,
        0xdead_beef,
        0xefbe_adde,
        0x0011_2233,
        0x3322_1100,
        0xf0e0_d0c0,
    ]
    .iter()
    .map(|&v| v as c_int)
    .collect();
    assert_same_all("C11 endianness witnesses", &xs);

    // Additionally pin the expected textual form against the C output, so a
    // both-wrong-the-same-way byte order would still be visible in the record.
    let out = run_one(Impl::C, 0x0102_0304u32 as c_int);
    assert_eq!(
        out, b"04030201\n",
        "C emits little-endian byte order; got {:?}",
        String::from_utf8_lossy(&out)
    );
    assert_eq!(run_one(Impl::Rust, 0x0102_0304u32 as c_int), out);
}

/// C12 — bytes 0x00 and 0x0a in every position.
fn cfg_nul_and_newline_bytes() {
    let mut xs: Vec<c_int> = Vec::new();
    for shift in [0u32, 8, 16, 24] {
        xs.push((0x0au32 << shift) as c_int);
        xs.push((0xffff_ffffu32 & !(0xffu32 << shift)) as c_int); // one zero byte
    }
    xs.extend(
        [0x0a0a_0a0au32, 0x000a_000a, 0x0a00_0a00, 0x0a00_0000, 0x0000_000a]
            .iter()
            .map(|&v| v as c_int),
    );
    assert_same_all("C12 NUL/newline bytes", &xs);
}

/// C13 — 1000 randomized calls through a single loaded handle.
fn cfg_repeated_calls_same_handle() {
    let mut rng = Rng::seeded(SEED ^ 0xDEAD_BEEF);
    let xs: Vec<c_int> = (0..1000).map(|_| rng.next_i32()).collect();

    let c = run_many(Impl::C, &xs);
    let r = run_many(Impl::Rust, &xs);
    assert_eq!(c, r, "C13: 1000 repeated calls diverged");
    assert_eq!(c.len(), 9000, "C13: expected 9 bytes per call");
}

/// C14 — C and Rust calls interleaved through the same `FILE *stdout`.
fn cfg_interleaved_c_and_rust() {
    let mut rng = Rng::seeded(SEED ^ 0x0F0F_0F0F);
    let xs: Vec<c_int> = (0..1000).map(|_| rng.next_i32()).collect();

    let cf = driver_of(Impl::C);
    let rf = driver_of(Impl::Rust);

    // Interleaved: each value printed by C then immediately by Rust. Adjacent
    // 9-byte records must be identical pairs.
    let both = capture_stdout_via_file(|| {
        for &x in &xs {
            unsafe {
                cf(x);
                rf(x);
            }
        }
    });
    assert_eq!(both.len(), 18 * xs.len(), "C14: unexpected total length");
    for (i, rec) in both.chunks(18).enumerate() {
        assert_eq!(
            &rec[..9],
            &rec[9..],
            "C14: record {i} (x = {}) diverged: C \"{}\" vs Rust \"{}\"",
            xs[i],
            String::from_utf8_lossy(&rec[..9]),
            String::from_utf8_lossy(&rec[9..])
        );
    }
}

/// C15 — stdout is a regular file (fully buffered). This is the mode used by
/// the rest of the suite; asserted explicitly here.
fn cfg_stdout_fully_buffered_file() {
    let mut rng = Rng::seeded(SEED ^ 0xFEED_FACE);
    let xs: Vec<c_int> = (0..256).map(|_| rng.next_i32()).collect();

    let cf = driver_of(Impl::C);
    let rf = driver_of(Impl::Rust);
    let c = capture_stdout_via_file(|| xs.iter().for_each(|&x| unsafe { cf(x) }));
    let r = capture_stdout_via_file(|| xs.iter().for_each(|&x| unsafe { rf(x) }));
    assert_eq!(c, r, "C15: file-buffered stdout diverged");
    assert_eq!(c.len(), 9 * xs.len());
}

/// C16 — stdout is a pipe.
fn cfg_stdout_pipe() {
    let mut rng = Rng::seeded(SEED ^ 0xCAFE_D00D);
    let xs: Vec<c_int> = (0..256).map(|_| rng.next_i32()).collect();

    let cf = driver_of(Impl::C);
    let rf = driver_of(Impl::Rust);
    let c = capture_stdout_via_pipe(|| xs.iter().for_each(|&x| unsafe { cf(x) }));
    let r = capture_stdout_via_pipe(|| xs.iter().for_each(|&x| unsafe { rf(x) }));
    assert_eq!(c, r, "C16: piped stdout diverged");
    assert_eq!(c.len(), 9 * xs.len(), "C16: expected 9 bytes per call");

    // Cross-check: piping must give the same bytes as writing to a file.
    assert_eq!(
        c,
        capture_stdout_via_file(|| xs.iter().for_each(|&x| unsafe { cf(x) })),
        "C16: pipe vs file differ for C"
    );
}

/// C17 — the output shape invariant, checked against an independent model:
/// 8 lowercase hex digits (little-endian bytes of `x`) followed by `\n`.
fn cfg_output_shape_invariant() {
    let mut rng = Rng::seeded(SEED ^ 0x1357_9BDF);
    let mut xs: Vec<c_int> = vec![0, -1, 1, i32::MIN, i32::MAX];
    xs.extend((0..512).map(|_| rng.next_i32()));

    for x in xs {
        let expected: String = {
            let bytes = (x as u32).to_le_bytes();
            let mut s = String::new();
            for b in bytes {
                s.push_str(&format!("{b:02x}"));
            }
            s.push('\n');
            s
        };
        let c = run_one(Impl::C, x);
        let r = run_one(Impl::Rust, x);
        assert_eq!(
            String::from_utf8_lossy(&c),
            expected,
            "C17: C output for {x} does not match the little-endian hex model"
        );
        assert_eq!(c, r, "C17: Rust diverged from C for {x}");
        assert_eq!(c.len(), 9);
        assert!(c[..8].iter().all(|b| b.is_ascii_hexdigit()
            && !b.is_ascii_uppercase()));
        assert_eq!(c[8], b'\n');
    }
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md rows (B1..B11) and generic FFI boundaries
// ---------------------------------------------------------------------------

/// B1-B6 — the extremes of the `int` domain, i.e. "one step past the range".
/// The C library rejects nothing, so both must *accept* these identically.
fn boundary_extremes() {
    let xs: Vec<c_int> = vec![
        i32::MIN,             // B1
        i32::MAX,             // B2
        i32::MIN + 1,         // B3
        i32::MAX - 1,         // B3
        0,                    // B4
        -1,                   // B5
        0x8000_0000u32 as c_int, // B6 (== i32::MIN, reached via unsigned cast)
        0x7fff_ffff,
    ];
    assert_same_all("B1-B6 extremes", &xs);

    // Pin the exact expected bytes for the two extremes.
    assert_eq!(run_one(Impl::C, i32::MIN), b"00000080\n");
    assert_eq!(run_one(Impl::Rust, i32::MIN), b"00000080\n");
    assert_eq!(run_one(Impl::C, i32::MAX), b"ffffff7f\n");
    assert_eq!(run_one(Impl::Rust, i32::MAX), b"ffffff7f\n");
    assert_eq!(run_one(Impl::Rust, -1), b"ffffffff\n");
    assert_eq!(run_one(Impl::Rust, 0), b"00000000\n");
}

/// B7 — out-of-range "enum" values. A C enum parameter accepts any `int`;
/// `driver` has no variant check, so a value with no valid variant must be
/// handled identically (printed, not rejected) by both implementations.
fn out_of_range_enum_values() {
    let xs: Vec<c_int> = vec![
        -2,
        -1,
        256,
        257,
        65_536,
        65_537,
        12_345_678,
        0x7fff_ffff,
        i32::MIN,
        1_000_000_000,
        -1_000_000_000,
    ];
    assert_same_all("B7 out-of-range enum values", &xs);
}

/// B8 — embedded NUL and newline bytes must not truncate or reframe output.
fn embedded_nul_and_newline_bytes() {
    let xs: Vec<c_int> = [
        10u32, 2560, 0x0a00_0a00, 0x000a_0000, 0x0a0a_0a0a, 0x0000_0000, 0xff00_ff00, 0x00ff_00ff,
    ]
    .iter()
    .map(|&v| v as c_int)
    .collect();
    assert_same_all("B8 NUL/newline", &xs);

    // Every record is exactly 9 bytes with exactly one newline, at the end.
    for &x in &xs {
        let out = run_one(Impl::C, x);
        assert_eq!(out.len(), 9, "B8: driver({x}) emitted {} bytes", out.len());
        assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 1);
        assert_eq!(run_one(Impl::Rust, x), out);
    }
}

/// B9 — a single high-bit byte at each position. A `char`-instead-of-
/// `unsigned char` bug would print `ffffffXX` here instead of `XX`.
fn high_bit_per_byte_position() {
    let mut xs: Vec<c_int> = Vec::new();
    for shift in [0u32, 8, 16, 24] {
        for b in [0x80u32, 0x81, 0xa5, 0xfe, 0xff] {
            xs.push((b << shift) as c_int);
        }
    }
    assert_same_all("B9 high bit per position", &xs);

    for &x in &xs {
        let out = run_one(Impl::Rust, x);
        assert_eq!(
            out.len(),
            9,
            "B9: driver(0x{:08x}) emitted {} bytes — sign-extension bug?",
            x as u32,
            out.len()
        );
    }
}

/// B10 — broad randomized sweep over the full 32-bit range (fixed seed).
fn randomized_full_range() {
    let mut rng = Rng::seeded(SEED ^ 0xB105_F00D);
    let xs: Vec<c_int> = (0..4096).map(|_| rng.next_i32()).collect();

    // Batch comparison keeps this fast; the per-value loop below narrows any
    // failure down to the exact input.
    let c = run_many(Impl::C, &xs);
    let r = run_many(Impl::Rust, &xs);
    if c != r {
        for &x in &xs {
            assert_same("B10 randomized", x);
        }
        panic!("B10: batch diverged but no single value did — state-dependent bug");
    }
    assert_eq!(c.len(), 9 * xs.len());
}

/// B11 — repeated interleaving C/Rust/C to confirm no shared-state corruption.
fn interleaved_calls_share_stdout() {
    let mut rng = Rng::seeded(SEED ^ 0x2468_ACE0);
    let xs: Vec<c_int> = (0..1000).map(|_| rng.next_i32()).collect();

    let cf = driver_of(Impl::C);
    let rf = driver_of(Impl::Rust);
    let out = capture_stdout_via_file(|| {
        for &x in &xs {
            unsafe {
                cf(x);
                rf(x);
                cf(x);
            }
        }
    });
    assert_eq!(out.len(), 27 * xs.len());
    for (i, rec) in out.chunks(27).enumerate() {
        assert_eq!(&rec[0..9], &rec[9..18], "B11: C vs Rust at {i}");
        assert_eq!(&rec[9..18], &rec[18..27], "B11: Rust vs C at {i}");
        assert_eq!(rec[8], b'\n');
    }
}

/// ERRORS.md structural note: the pointer/length helper is unreachable from
/// outside either `.so`, so null-pointer and bad-length inputs cannot be
/// supplied by an external caller in either implementation.
fn print_hex_is_not_exported_by_either() {
    assert!(
        print_hex_is_hidden(),
        "`print_hex` is `static` in C and must stay unexported in Rust too"
    );
}

/// Symbol parity smoke check from inside the test binary: `driver` resolves in
/// both `.so`s, and a symbol the C library does not export resolves in neither.
fn symbol_parity_smoke() {
    // Resolving both handles is enough; `driver_of` panics if a symbol is absent.
    let _ = driver_of(Impl::C);
    let _ = driver_of(Impl::Rust);
    assert_same("parity smoke", 0x1234_5678);
}

// ---------------------------------------------------------------------------
// Custom test runner (`harness = false`)
//
// Rows are run strictly sequentially so that nothing else can write to fd 1
// while a capture window is open. Each row is wrapped in `catch_unwind` so one
// failing row still lets the rest report, and the process exits non-zero if any
// row failed.
// ---------------------------------------------------------------------------

fn main() {
    let rows: &[(&str, fn())] = &[
        // Phase B — CONFIGS.md
        ("C1  cfg_zero", cfg_zero),
        ("C2  cfg_low_byte_sweep", cfg_low_byte_sweep),
        ("C3  cfg_byte1_sweep", cfg_byte1_sweep),
        ("C4  cfg_byte2_sweep", cfg_byte2_sweep),
        ("C5  cfg_byte3_sweep", cfg_byte3_sweep),
        ("C6  cfg_all_bytes_high", cfg_all_bytes_high),
        ("C7  cfg_negative_random", cfg_negative_random),
        ("C8  cfg_positive_random", cfg_positive_random),
        ("C9  cfg_full_range_random", cfg_full_range_random),
        ("C10 cfg_powers_of_two", cfg_powers_of_two),
        ("C11 cfg_endianness_witnesses", cfg_endianness_witnesses),
        ("C12 cfg_nul_and_newline_bytes", cfg_nul_and_newline_bytes),
        ("C13 cfg_repeated_calls_same_handle", cfg_repeated_calls_same_handle),
        ("C14 cfg_interleaved_c_and_rust", cfg_interleaved_c_and_rust),
        ("C15 cfg_stdout_fully_buffered_file", cfg_stdout_fully_buffered_file),
        ("C16 cfg_stdout_pipe", cfg_stdout_pipe),
        ("C17 cfg_output_shape_invariant", cfg_output_shape_invariant),
        // Phase C — ERRORS.md
        ("B1-B6 boundary_extremes", boundary_extremes),
        ("B7  out_of_range_enum_values", out_of_range_enum_values),
        ("B8  embedded_nul_and_newline_bytes", embedded_nul_and_newline_bytes),
        ("B9  high_bit_per_byte_position", high_bit_per_byte_position),
        ("B10 randomized_full_range", randomized_full_range),
        ("B11 interleaved_calls_share_stdout", interleaved_calls_share_stdout),
        // Structural
        ("S1  print_hex_is_not_exported_by_either", print_hex_is_not_exported_by_either),
        ("S2  symbol_parity_smoke", symbol_parity_smoke),
    ];

    // Optional substring filter, like libtest: `cargo test --test differential -- cfg_zero`
    let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with('-'));

    println!("running {} differential rows", rows.len());
    let mut passed = 0usize;
    let mut failed: Vec<&str> = Vec::new();
    let mut skipped = 0usize;

    for (name, f) in rows {
        if let Some(ref pat) = filter {
            if !name.contains(pat.as_str()) {
                skipped += 1;
                continue;
            }
        }
        print!("row {name} ... ");
        // Flush our own progress line BEFORE the row redirects fd 1, otherwise
        // it would be captured as if the library had printed it.
        use std::io::Write;
        std::io::stdout().flush().ok();

        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => {
                println!("ok");
                passed += 1;
            }
            Err(_) => {
                println!("FAILED");
                failed.push(name);
            }
        }
        std::io::stdout().flush().ok();
    }

    println!();
    if failed.is_empty() {
        println!("test result: ok. {passed} passed; 0 failed; {skipped} filtered out");
    } else {
        println!("failing rows:");
        for n in &failed {
            println!("  - {n}");
        }
        println!(
            "test result: FAILED. {passed} passed; {} failed; {skipped} filtered out",
            failed.len()
        );
        std::process::exit(1);
    }
}
