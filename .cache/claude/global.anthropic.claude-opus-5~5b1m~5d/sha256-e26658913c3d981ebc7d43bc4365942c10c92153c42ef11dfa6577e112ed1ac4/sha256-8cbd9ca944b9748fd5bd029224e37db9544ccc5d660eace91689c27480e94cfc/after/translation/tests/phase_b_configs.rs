//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every row loads BOTH `libdriver.so` files via `libloading` and compares the
//! bytes each writes to stdout. Randomized rows use a fixed seed.

mod common;

use common::*;
use std::ffi::CString;

// ---------------------------------------------------------------- row 1
/// CONFIGS row 1 — `printLine`, non-NULL printable ASCII, random length 1..64.
#[test]
fn cfg_1_print_line_random_ascii() {
    let mut rng = Rng::new(SEED);
    for i in 0..512 {
        let len = rng.range(1, 64);
        let payload: Vec<u8> = (0..len).map(|_| rng.range(0x20, 0x7E) as u8).collect();
        diff_print_line(&format!("cfg_1[{i}] len={len}"), &payload);
    }
}

// ---------------------------------------------------------------- row 2
/// CONFIGS row 2 — every single-byte string, `0x01..=0xFF`.
#[test]
fn cfg_2_print_line_every_single_byte() {
    for b in 1u8..=255 {
        diff_print_line(&format!("cfg_2[byte=0x{b:02x}]"), &[b]);
    }
}

// ---------------------------------------------------------------- row 3
/// CONFIGS row 3 — the empty string (length 0, valid).
#[test]
fn cfg_3_print_line_empty() {
    diff_print_line("cfg_3[empty]", b"");
}

// ---------------------------------------------------------------- row 4
/// CONFIGS row 4 — random arbitrary/high-bit (non-UTF-8) bytes, length 1..256.
#[test]
fn cfg_4_print_line_random_binary() {
    let mut rng = Rng::new(SEED ^ 0x04);
    for i in 0..512 {
        let len = rng.range(1, 256);
        let payload: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        diff_print_line(&format!("cfg_4[{i}] len={len}"), &payload);
    }
}

// ---------------------------------------------------------------- row 5
/// CONFIGS row 5 — strings carrying embedded `\n`, `\r`, `\t`.
#[test]
fn cfg_5_print_line_embedded_newlines() {
    let mut rng = Rng::new(SEED ^ 0x05);
    for i in 0..256 {
        let len = rng.range(1, 48);
        let payload: Vec<u8> = (0..len)
            .map(|_| match rng.range(0, 3) {
                0 => b'\n',
                1 => b'\r',
                2 => b'\t',
                _ => rng.range(0x41, 0x5A) as u8,
            })
            .collect();
        diff_print_line(&format!("cfg_5[{i}] len={len}"), &payload);
    }
}

// ---------------------------------------------------------------- row 6
/// CONFIGS row 6 — conversion-like sequences. The C passes the string as a
/// `printf` *argument*, never as the format, so these must be literal.
#[test]
fn cfg_6_print_line_format_specifiers() {
    const FRAGMENTS: [&[u8]; 10] = [
        b"%", b"%s", b"%n", b"%d", b"%%", b"%p", b"%1000000d", b"%.*s", b"{}", b"ok",
    ];
    // Hand-picked worst cases first.
    for (i, f) in FRAGMENTS.iter().enumerate() {
        diff_print_line(&format!("cfg_6[fragment {i}]"), f);
    }
    let mut rng = Rng::new(SEED ^ 0x06);
    for i in 0..256 {
        let n = rng.range(1, 8);
        let mut payload = Vec::new();
        for _ in 0..n {
            payload.extend_from_slice(rng.pick(&FRAGMENTS));
        }
        diff_print_line(&format!("cfg_6[{i}] {:?}", String::from_utf8_lossy(&payload)), &payload);
    }
}

// ---------------------------------------------------------------- row 7
/// CONFIGS row 7 — length boundaries around common buffer sizes.
#[test]
fn cfg_7_print_line_lengths_boundary() {
    let mut rng = Rng::new(SEED ^ 0x07);
    for &len in &[1usize, 2, 3, 15, 16, 17, 63, 64, 65, 255, 256, 257, 1023, 1024, 1025, 4095] {
        let payload: Vec<u8> = (0..len).map(|_| rng.range(0x21, 0x7E) as u8).collect();
        diff_print_line(&format!("cfg_7[len={len}]"), &payload);
    }
}

// ---------------------------------------------------------------- row 8
/// CONFIGS row 8 — NULL, the guard's false branch, as a valid call.
#[test]
fn cfg_8_print_line_null() {
    let c_fn = print_line(c_lib());
    let r_fn = print_line(rust_lib());
    let c_out = capture(|| unsafe { c_fn(std::ptr::null()) });
    let r_out = capture(|| unsafe { r_fn(std::ptr::null()) });
    assert_same("cfg_8[NULL]", &c_out, &r_out);
    assert_same("cfg_8[NULL] (vs. expected: no output)", b"", &c_out);
}

// ---------------------------------------------------------------- row 9
/// CONFIGS row 9 — `bad()`.
#[test]
fn cfg_9_bad_single() {
    let out = diff_nullary("bad");
    assert_same("cfg_9[bad literal]", b"bad()\n", &out);
}

// ---------------------------------------------------------------- row 10
/// CONFIGS row 10 — `good()`, which must also emit the `static helperGood()`
/// line, in that order.
#[test]
fn cfg_10_good_single() {
    let out = diff_nullary("good");
    assert_same("cfg_10[good literal]", b"good()\nhelperGood()\n", &out);
}

// ---------------------------------------------------------------- row 11
/// CONFIGS row 11 — `driver()`, the whole composed pipeline. Note that the C's
/// `bad()` does NOT call `helperBad()`; that asymmetry is replicated.
#[test]
fn cfg_11_driver_single() {
    let out = diff_nullary("driver");
    let expected: &[u8] = b"Calling good()...\n\
                            good()\n\
                            helperGood()\n\
                            Finished good()\n\
                            Calling bad()...\n\
                            bad()\n\
                            Finished bad()\n";
    assert_same("cfg_11[driver literal]", expected, &out);
    assert!(
        !String::from_utf8_lossy(&out).contains("helperBad"),
        "the C `bad()` never calls `helperBad()`; Rust must not either"
    );
}

// ---------------------------------------------------------------- row 12
/// CONFIGS row 12 — repeated nullary calls: no hidden state accumulates.
#[test]
fn cfg_12_nullary_repeated() {
    for name in ["bad", "good", "driver"] {
        let c_fn = nullary(c_lib(), name);
        let r_fn = nullary(rust_lib(), name);
        let c_out = capture(|| {
            for _ in 0..64 {
                unsafe { c_fn() }
            }
        });
        let r_out = capture(|| {
            for _ in 0..64 {
                unsafe { r_fn() }
            }
        });
        assert_same(&format!("cfg_12[{name} x64]"), &c_out, &r_out);
        let single = diff_nullary(name);
        assert_same(
            &format!("cfg_12[{name} x64 == single x64]"),
            &single.repeat(64),
            &c_out,
        );
    }
}

// ---------------------------------------------------------------- rows 13/14

/// One step of a randomized call script.
enum Op {
    PrintLine(Vec<u8>),
    PrintLineNull,
    Bad,
    Good,
    Driver,
}

fn random_script(seed: u64, n: usize) -> Vec<Op> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| match rng.range(0, 5) {
            0 | 1 => {
                let len = rng.range(0, 40);
                Op::PrintLine((0..len).map(|_| rng.range(0x20, 0x7E) as u8).collect())
            }
            2 => Op::PrintLineNull,
            3 => Op::Bad,
            4 => Op::Good,
            _ => Op::Driver,
        })
        .collect()
}

fn run_op(lib: &libloading::Library, op: &Op) {
    match op {
        Op::PrintLine(p) => {
            let cs = CString::new(p.clone()).unwrap();
            unsafe { print_line(lib)(cs.as_ptr()) }
        }
        Op::PrintLineNull => unsafe { print_line(lib)(std::ptr::null()) },
        Op::Bad => unsafe { nullary(lib, "bad")() },
        Op::Good => unsafe { nullary(lib, "good")() },
        Op::Driver => unsafe { nullary(lib, "driver")() },
    }
}

/// CONFIGS row 13 — a randomized interleaved sequence across all four entry
/// points, run as one uninterrupted stream against each library.
#[test]
fn cfg_13_random_interleaved_sequence() {
    let script = random_script(SEED ^ 0x13, 200);
    let c_out = capture(|| {
        for op in &script {
            run_op(c_lib(), op)
        }
    });
    let r_out = capture(|| {
        for op in &script {
            run_op(rust_lib(), op)
        }
    });
    assert_same("cfg_13[200-op script]", &c_out, &r_out);
    assert!(!c_out.is_empty(), "script produced no output");
}

/// CONFIGS row 14 — C and Rust called *alternately on the same `stdout`
/// stream*. Both `.so`s import the same `puts@GLIBC` and therefore share one
/// `FILE` buffer; the merged stream must equal each op's output emitted twice,
/// with no reordering or lost bytes from mismatched buffering.
#[test]
fn cfg_14_cross_library_shared_stdout() {
    let script = random_script(SEED ^ 0x14, 120);

    let merged = capture(|| {
        for op in &script {
            run_op(c_lib(), op);
            run_op(rust_lib(), op);
        }
    });

    let mut expected = Vec::new();
    for op in &script {
        let per_op = capture(|| run_op(c_lib(), op));
        expected.extend_from_slice(&per_op);
        expected.extend_from_slice(&per_op);
    }

    assert_same("cfg_14[alternating shared stdout]", &expected, &merged);
}
