//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every row drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols (loaded with `libloading`) and asserts the produced stdout bytes are
//! identical.  Randomised rows use the fixed seed `common::SEED`.

mod common;

use common::*;

/// Reference model of the C behaviour, used as a third opinion so a test can
/// fail even if C and Rust were to agree on something wrong.
fn model(ops: &[Op]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let push_int = |o: &mut Vec<u8>, v: i32| {
        o.extend_from_slice(v.to_string().as_bytes());
        o.push(b'\n');
    };
    for op in ops {
        match op {
            Op::PrintLine(b) => {
                out.extend_from_slice(b);
                out.push(b'\n');
            }
            Op::PrintLineNull => {}
            Op::PrintIntLine(v) => push_int(&mut out, *v),
            Op::PrintIntLineWide(v) => push_int(&mut out, *v as i32),
            Op::Bad | Op::BadExtraArgs(..) => {
                push_int(&mut out, 0);
                push_int(&mut out, 0);
            }
            Op::Good | Op::GoodExtraArgs(..) => {
                push_int(&mut out, 0);
                push_int(&mut out, 2);
            }
            Op::Driver | Op::DriverExtraArgs(..) => {
                out.extend_from_slice(b"Calling good()...\n");
                push_int(&mut out, 0);
                push_int(&mut out, 2);
                out.extend_from_slice(b"Finished good()\n");
                out.extend_from_slice(b"Calling bad()...\n");
                push_int(&mut out, 0);
                push_int(&mut out, 0);
                out.extend_from_slice(b"Finished bad()\n");
            }
        }
    }
    out
}

/// Differential check plus a cross-check against the reference model.
fn check(label: &str, ops: &[Op]) {
    let out = diff(label, ops);
    let want = model(ops);
    assert_eq!(
        out,
        want,
        "[{label}] C/Rust agreed but disagree with the reference model\n  got  = {:?}\n  want = {:?}",
        String::from_utf8_lossy(&out[..out.len().min(300)]),
        String::from_utf8_lossy(&want[..want.len().min(300)])
    );
}

// ===========================================================================
// printIntLine — rows C01..C08
// ===========================================================================

#[test]
fn c01_print_int_line_zero() {
    check("C01 zero", &[Op::PrintIntLine(0)]);
}

#[test]
fn c02_print_int_line_positive_1_to_9_digits() {
    let mut rng = Rng::new(SEED ^ 0x02);
    for digits in 1..=9u32 {
        let lo = if digits == 1 { 1 } else { 10i64.pow(digits - 1) };
        let hi = 10i64.pow(digits) - 1;
        for _ in 0..64 {
            let v = rng.range_i64(lo, hi) as i32;
            check(&format!("C02 +{digits}d {v}"), &[Op::PrintIntLine(v)]);
        }
    }
}

#[test]
fn c03_print_int_line_positive_10_digits() {
    let mut rng = Rng::new(SEED ^ 0x03);
    for _ in 0..256 {
        let v = rng.range_i64(1_000_000_000, i32::MAX as i64) as i32;
        check(&format!("C03 {v}"), &[Op::PrintIntLine(v)]);
    }
}

#[test]
fn c04_print_int_line_negative_1_to_9_digits() {
    let mut rng = Rng::new(SEED ^ 0x04);
    for digits in 1..=9u32 {
        let lo = if digits == 1 { 1 } else { 10i64.pow(digits - 1) };
        let hi = 10i64.pow(digits) - 1;
        for _ in 0..64 {
            let v = -(rng.range_i64(lo, hi)) as i32;
            check(&format!("C04 -{digits}d {v}"), &[Op::PrintIntLine(v)]);
        }
    }
}

#[test]
fn c05_print_int_line_negative_10_digits() {
    let mut rng = Rng::new(SEED ^ 0x05);
    for _ in 0..256 {
        let v = rng.range_i64(i32::MIN as i64 + 1, -1_000_000_000) as i32;
        check(&format!("C05 {v}"), &[Op::PrintIntLine(v)]);
    }
}

#[test]
fn c06_print_int_line_extremes() {
    check("C06 INT_MAX", &[Op::PrintIntLine(i32::MAX)]);
    check("C06 INT_MIN", &[Op::PrintIntLine(i32::MIN)]);
    // and both in one call sequence
    check(
        "C06 both",
        &[Op::PrintIntLine(i32::MIN), Op::PrintIntLine(i32::MAX)],
    );
}

#[test]
fn c07_print_int_line_digit_and_power_boundaries() {
    let mut interesting: Vec<i64> = Vec::new();
    // powers of ten ±1
    let mut p: i64 = 1;
    while p <= 10_000_000_000 {
        for d in [-1i64, 0, 1] {
            interesting.push(p + d);
            interesting.push(-(p + d));
        }
        p *= 10;
    }
    // powers of two ±1
    for k in 0..32u32 {
        let q = 1i64 << k;
        for d in [-1i64, 0, 1] {
            interesting.push(q + d);
            interesting.push(-(q + d));
        }
    }
    interesting.push(0);
    for v in interesting {
        let v = v as i32; // wrap exactly like the C `int` parameter would
        check(&format!("C07 {v}"), &[Op::PrintIntLine(v)]);
    }
}

#[test]
fn c08_print_int_line_full_range_sweep() {
    let mut rng = Rng::new(SEED ^ 0x08);
    // batch many values per capture to keep the test fast while still covering
    // 4096 distinct random i32 values.
    for _ in 0..64 {
        let ops: Vec<Op> = (0..64).map(|_| Op::PrintIntLine(rng.next_i32())).collect();
        check("C08 sweep", &ops);
    }
}

// ===========================================================================
// printLine — rows C09..C15
// ===========================================================================

#[test]
fn c09_print_line_empty() {
    let out = diff("C09 empty", &[Op::PrintLine(Vec::new())]);
    assert_eq!(out, b"\n", "empty string must yield exactly one newline");
}

#[test]
fn c10_print_line_single_byte() {
    let mut rng = Rng::new(SEED ^ 0x10);
    for _ in 0..128 {
        let b = rng.byte_printable();
        check(&format!("C10 {b:#04x}"), &[Op::PrintLine(vec![b])]);
    }
    // every printable ASCII byte, exhaustively
    for b in 0x20u8..=0x7E {
        check(&format!("C10 all {b:#04x}"), &[Op::PrintLine(vec![b])]);
    }
}

#[test]
fn c11_print_line_random_printable() {
    let mut rng = Rng::new(SEED ^ 0x11);
    for _ in 0..256 {
        let len = rng.range_usize(2, 64);
        let payload = rng.bytes_printable(len);
        check("C11 printable", &[Op::PrintLine(payload)]);
    }
}

#[test]
fn c12_print_line_full_byte_range() {
    let mut rng = Rng::new(SEED ^ 0x12);
    for _ in 0..256 {
        let len = rng.range_usize(1, 256);
        let payload = rng.bytes_any(len); // 0x01..=0xFF, invalid UTF-8 included
        check("C12 raw bytes", &[Op::PrintLine(payload)]);
    }
    // exhaustive single non-NUL byte
    for b in 1u8..=255 {
        check(&format!("C12 byte {b:#04x}"), &[Op::PrintLine(vec![b])]);
    }
}

#[test]
fn c13_print_line_format_specifier_payloads() {
    let payloads: &[&[u8]] = &[
        b"%s",
        b"%d",
        b"%n",
        b"%%",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"%1000000d",
        b"%.*s",
        b"%99999999999999999999d",
        b"100%% sure",
        b"%p %x %o %e %g %c",
        b"%-+ #0*.*hhlljjzztLd",
    ];
    for p in payloads {
        check(
            &format!("C13 {:?}", String::from_utf8_lossy(p)),
            &[Op::PrintLine(p.to_vec())],
        );
    }
    // randomly assembled specifier soup
    let mut rng = Rng::new(SEED ^ 0x13);
    let atoms: &[&[u8]] = &[b"%s", b"%d", b"%n", b"%%", b"%p", b"x", b" ", b"%9$s"];
    for _ in 0..128 {
        let n = rng.range_usize(1, 12);
        let mut payload = Vec::new();
        for _ in 0..n {
            payload.extend_from_slice(atoms[rng.range_usize(0, atoms.len() - 1)]);
        }
        check("C13 soup", &[Op::PrintLine(payload)]);
    }
}

#[test]
fn c14_print_line_embedded_control_chars() {
    let payloads: &[&[u8]] = &[
        b"a\nb",
        b"\n",
        b"\n\n\n",
        b"a\r\nb",
        b"\ttabbed\t",
        b"line1\nline2\nline3\n",
        b"\x0b\x0c\x1b[31m",
    ];
    for p in payloads {
        check("C14 control", &[Op::PrintLine(p.to_vec())]);
    }
    let mut rng = Rng::new(SEED ^ 0x14);
    let ctrl = [b'\n', b'\r', b'\t', b' ', b'A', 0x0b, 0x0c, 0x1b];
    for _ in 0..128 {
        let len = rng.range_usize(1, 48);
        let payload: Vec<u8> = (0..len)
            .map(|_| ctrl[rng.range_usize(0, ctrl.len() - 1)])
            .collect();
        check("C14 random control", &[Op::PrintLine(payload)]);
    }
}

#[test]
fn c15_print_line_large_payloads() {
    let mut rng = Rng::new(SEED ^ 0x15);
    for len in [4096usize, 65536, 1024 * 1024] {
        let payload = rng.bytes_printable(len);
        let out = diff(&format!("C15 len={len}"), &[Op::PrintLine(payload.clone())]);
        assert_eq!(out.len(), len + 1, "expected payload + newline");
        assert_eq!(&out[..len], &payload[..]);
        assert_eq!(out[len], b'\n');
    }
    // sizes straddling the usual 4096-byte stdio buffer
    for len in [4093usize, 4094, 4095, 4096, 4097, 4098, 8191, 8192, 8193] {
        let payload = rng.bytes_any(len);
        check(&format!("C15 straddle {len}"), &[Op::PrintLine(payload)]);
    }
}

// ===========================================================================
// bad / good / driver — rows C16..C18, C20
// ===========================================================================

#[test]
fn c16_bad_preserves_cwe482_defect() {
    let out = diff("C16 bad", &[Op::Bad]);
    assert_eq!(
        out, b"0\n0\n",
        "bad() must print 0 twice — the discarded `intOne + intTwo` defect"
    );
}

#[test]
fn c17_good() {
    let out = diff("C17 good", &[Op::Good]);
    assert_eq!(out, b"0\n2\n");
}

#[test]
fn c18_driver_full_pipeline() {
    let out = diff("C18 driver", &[Op::Driver]);
    assert_eq!(
        out,
        b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n"
    );
}

#[test]
fn c20_driver_repeated() {
    let ops: Vec<Op> = (0..8).map(|_| Op::Driver).collect();
    check("C20 driver x8", &ops);
}

// ===========================================================================
// Row C19 — randomised mixed low-level call sequences
// ===========================================================================

fn random_op(rng: &mut Rng) -> Op {
    match rng.range_usize(0, 9) {
        0 => Op::PrintIntLine(rng.next_i32()),
        1 => Op::PrintIntLine(rng.range_i64(-1000, 1000) as i32),
        2 => {
            let n = rng.range_usize(0, 40);
            Op::PrintLine(rng.bytes_printable(n))
        }
        3 => {
            let n = rng.range_usize(1, 80);
            Op::PrintLine(rng.bytes_any(n))
        }
        4 => Op::PrintLineNull,
        5 => Op::Bad,
        6 => Op::Good,
        7 => Op::Driver,
        8 => Op::PrintIntLineWide(rng.next_u64() as i64),
        _ => Op::PrintLine(b"%s %d %n".to_vec()),
    }
}

fn random_script(rng: &mut Rng) -> Vec<Op> {
    let n = rng.range_usize(1, 40);
    (0..n).map(|_| random_op(rng)).collect()
}

#[test]
fn c19_random_mixed_sequences() {
    let mut rng = Rng::new(SEED ^ 0x19);
    for i in 0..300 {
        let ops = random_script(&mut rng);
        check(&format!("C19 script #{i}"), &ops);
    }
}

// ===========================================================================
// Rows C21..C23 — buffering-mode axis
// ===========================================================================

fn buffering_row(label: &str, mode: Buffering, seed_salt: u64) {
    let mut rng = Rng::new(SEED ^ seed_salt);
    for i in 0..60 {
        let ops = random_script(&mut rng);
        let out = diff_with(&format!("{label} #{i}"), mode, &ops);
        assert_eq!(
            out,
            model(&ops),
            "[{label} #{i}] output disagrees with the reference model"
        );
    }
}

#[test]
fn c21_unbuffered_stdout() {
    buffering_row("C21 unbuffered", Buffering::Unbuffered, 0x21);
}

#[test]
fn c22_line_buffered_stdout() {
    buffering_row("C22 line-buffered", Buffering::LineBuffered, 0x22);
}

#[test]
fn c23_fully_buffered_stdout() {
    buffering_row("C23 fully-buffered", Buffering::FullyBuffered, 0x23);
    buffering_row("C23 default", Buffering::Default, 0x2323);
}

// ===========================================================================
// Row C24 — the two libraries interleaved into one shared stdio buffer
// ===========================================================================

#[test]
fn c24_interleaved_shared_buffer() {
    let mut rng = Rng::new(SEED ^ 0x24);
    for mode in [
        Buffering::Default,
        Buffering::Unbuffered,
        Buffering::LineBuffered,
        Buffering::FullyBuffered,
    ] {
        for i in 0..40 {
            let ops = random_script(&mut rng);

            // C call then Rust call for each op, on the SAME stdout, with no
            // intervening flush.  Any per-library buffering divergence (e.g.
            // `puts` vs `printf`) would reorder or lose bytes here.
            let interleaved = capture_with(mode, || {
                for op in &ops {
                    apply(c_api(), std::slice::from_ref(op));
                    apply(rust_api(), std::slice::from_ref(op));
                }
            });

            let mut want = Vec::new();
            for op in &ops {
                let one = model(std::slice::from_ref(op));
                want.extend_from_slice(&one);
                want.extend_from_slice(&one);
            }
            assert_eq!(
                interleaved,
                want,
                "[C24 {mode:?} #{i}] interleaved stream mismatch\n  got  = {:?}\n  want = {:?}",
                String::from_utf8_lossy(&interleaved[..interleaved.len().min(300)]),
                String::from_utf8_lossy(&want[..want.len().min(300)])
            );
        }
    }
}
