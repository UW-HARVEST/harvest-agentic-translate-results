// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Both libraries are driven through their
// exported C symbols loaded with `libloading`; outputs are compared
// byte-for-byte. Randomized rows use a fixed seed for reproducibility.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// C1..C9 — the low-level entry point `printLine`, driven directly
// ---------------------------------------------------------------------------

/// C1: empty string.
#[test]
fn c1_print_line_empty() {
    diff_print_line(b"\0");
}

/// C2: single byte payload, every non-zero byte value.
#[test]
fn c2_print_line_single_byte_all_values() {
    for b in 1u8..=255 {
        diff_print_line(&[b, 0]);
    }
}

/// C3: random length 2..=98 of random non-zero bytes.
#[test]
fn c3_print_line_random_short() {
    let mut rng = Rng::new(SEED ^ 0xC3);
    for _ in 0..200 {
        let len = rng.range_i32(2, 98) as usize;
        let mut buf: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        buf.push(0);
        diff_print_line(&buf);
    }
}

/// C4: exactly the 99×'A' shape that `driver(99)` produces.
#[test]
fn c4_print_line_99_a() {
    let mut buf = vec![b'A'; 99];
    buf.push(0);
    diff_print_line(&buf);
}

/// C5: long payloads.
#[test]
fn c5_print_line_long() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    for &len in &[100usize, 255, 256, 257, 1024, 4095, 4096, 8192] {
        for _ in 0..3 {
            let mut buf: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
            buf.push(0);
            diff_print_line(&buf);
        }
    }
}

/// C6: `printf` format metacharacters must be treated as plain data.
#[test]
fn c6_print_line_format_specifiers() {
    let cases: &[&[u8]] = &[
        b"%s\0",
        b"%n\0",
        b"%d %i %u %x\0",
        b"%%\0",
        b"%999999999d\0",
        b"%s%s%s%s%s%s%s%s\0",
        b"100%% done: %p\0",
        b"\x25\x6e\x25\x6e\0",
    ];
    for c in cases {
        diff_print_line(c);
    }
}

/// C7: high-bit / control bytes.
#[test]
fn c7_print_line_high_bit_and_control_bytes() {
    let mut buf: Vec<u8> = (0x80u8..=0xff).collect();
    buf.push(0);
    diff_print_line(&buf);

    let mut buf2: Vec<u8> = (0x01u8..=0x1f).collect();
    buf2.push(0);
    diff_print_line(&buf2);

    diff_print_line(b"tab\there\r\nand \x01\x02\x7f end\0");
    // Invalid UTF-8 sequences: must still be forwarded verbatim.
    diff_print_line(&[0xff, 0xfe, 0xc0, 0x80, 0xed, 0xa0, 0x80, 0]);

    let mut rng = Rng::new(SEED ^ 0xC7);
    for _ in 0..100 {
        let len = rng.range_i32(1, 64) as usize;
        let mut b: Vec<u8> = (0..len)
            .map(|_| {
                let v = rng.byte();
                if v == 0 { 0xff } else { v }
            })
            .collect();
        b.push(0);
        diff_print_line(&b);
    }
}

/// C8: bytes after the first NUL must be ignored.
#[test]
fn c8_print_line_early_nul() {
    let mut rng = Rng::new(SEED ^ 0xC8);
    for _ in 0..100 {
        let head = rng.range_i32(0, 20) as usize;
        let tail = rng.range_i32(1, 40) as usize;
        let mut buf: Vec<u8> = (0..head).map(|_| rng.nonzero_byte()).collect();
        buf.push(0);
        buf.extend((0..tail).map(|_| rng.nonzero_byte()));
        buf.push(0);
        diff_print_line(&buf);
    }
}

/// C9: repeated calls — no hidden state.
#[test]
fn c9_print_line_repeated() {
    let l = libs();
    let payloads: Vec<Vec<u8>> = (0..10)
        .map(|i| {
            let mut v = vec![b'a' + i as u8; i * 3 + 1];
            v.push(0);
            v
        })
        .collect();

    let fc = l.print_line(Which::C);
    let c = capture(|| {
        for p in &payloads {
            unsafe { fc(p.as_ptr() as *const std::ffi::c_char) };
        }
    });
    let fr = l.print_line(Which::Rust);
    let rs = capture(|| {
        for p in &payloads {
            unsafe { fr(p.as_ptr() as *const std::ffi::c_char) };
        }
    });
    assert_same("printLine x10", &c, &rs);
}

// ---------------------------------------------------------------------------
// C10..C19 — `driver`
// ---------------------------------------------------------------------------

/// C10: data == 0.
#[test]
fn c10_driver_zero() {
    diff_driver(0);
}

/// C11: data == 1.
#[test]
fn c11_driver_one() {
    diff_driver(1);
}

/// C12: randomized in-branch values 2..=98.
#[test]
fn c12_driver_random_in_branch() {
    let mut rng = Rng::new(SEED ^ 0x12);
    for _ in 0..500 {
        diff_driver(rng.range_i32(2, 98));
    }
}

/// C13: in-branch upper edge.
#[test]
fn c13_driver_upper_edge() {
    diff_driver(98);
    diff_driver(99);
}

/// C14: first out-of-branch value.
#[test]
fn c14_driver_boundary_100() {
    diff_driver(100);
}

/// C15: oversized values.
#[test]
fn c15_driver_oversized() {
    diff_driver(101);
    diff_driver(1000);
    diff_driver(65536);
    let mut rng = Rng::new(SEED ^ 0x15);
    for _ in 0..200 {
        diff_driver(rng.range_i32(101, i32::MAX));
    }
}

/// C16: INT_MAX.
#[test]
fn c16_driver_int_max() {
    diff_driver(i32::MAX);
    diff_driver(i32::MAX - 1);
}

/// C17: exhaustive sweep of the whole in-branch domain.
#[test]
fn c17_driver_exhaustive_in_branch() {
    for d in 0..=99 {
        diff_driver(d);
    }
}

/// C18: dense sweep of the out-of-branch domain.
#[test]
fn c18_driver_dense_out_of_branch() {
    for d in 100..=400 {
        diff_driver(d);
    }
    let mut rng = Rng::new(SEED ^ 0x18);
    for _ in 0..100 {
        diff_driver(rng.range_i32(100, 1 << 30));
    }
}

/// C19: alternating in/out-of-branch calls in one capture window.
#[test]
fn c19_driver_alternating_sequence() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0x19);
    let seq: Vec<i32> = (0..64)
        .map(|i| {
            if i % 2 == 0 {
                rng.range_i32(0, 99)
            } else {
                rng.range_i32(100, i32::MAX)
            }
        })
        .collect();

    let fc = l.driver(Which::C);
    let c = capture(|| {
        for &d in &seq {
            unsafe { fc(d) };
        }
    });
    let fr = l.driver(Which::Rust);
    let rs = capture(|| {
        for &d in &seq {
            unsafe { fr(d) };
        }
    });
    assert_same(&format!("driver sequence {seq:?}"), &c, &rs);
}

/// C20: composed pipeline — `driver(n)` must equal `printLine("A"*n)` in both
/// libraries (verifies the internal memset/strncpy/printLine composition).
#[test]
fn c20_driver_matches_print_line_composition() {
    for n in 0..=99usize {
        let mut buf = vec![b'A'; n];
        buf.push(0);

        let dc = run_driver(Which::C, n as i32);
        let dr = run_driver(Which::Rust, n as i32);
        let pc = run_print_line(Which::C, &buf);
        let pr = run_print_line(Which::Rust, &buf);

        assert_same(&format!("driver({n}) vs printLine"), &dc, &dr);
        assert_same(&format!("printLine(A*{n})"), &pc, &pr);
        assert_same(&format!("driver({n}) == printLine(A*{n}) [C]"), &dc, &pc);
        assert_same(&format!("driver({n}) == printLine(A*{n}) [Rust]"), &dr, &pr);
    }
}

// ---------------------------------------------------------------------------
// C22 — buffering behaviour: pipe vs regular file (out of process)
// ---------------------------------------------------------------------------

/// C22: stdout as a pipe and as a regular file must produce the same bytes in
/// both libraries.
#[test]
fn c22_buffering_pipe_vs_file() {
    if runner_path().is_none() {
        eprintln!("skipping c22: runner example not built");
        return;
    }
    for d in [0, 1, 50, 99, 100, 1000, i32::MAX] {
        let arg = d.to_string();
        let cp = run_subprocess(Which::C, "driver", &arg).unwrap();
        let rp = run_subprocess(Which::Rust, "driver", &arg).unwrap();
        let cf = run_subprocess_to_file(Which::C, "driver", &arg).unwrap();
        let rf = run_subprocess_to_file(Which::Rust, "driver", &arg).unwrap();

        assert_eq!(cp.code, rp.code, "exit code (pipe) for driver({d})");
        assert_eq!(cp.signal, rp.signal, "signal (pipe) for driver({d})");
        assert_same(&format!("driver({d}) via pipe"), &cp.stdout, &rp.stdout);
        assert_same(&format!("driver({d}) via file"), &cf.stdout, &rf.stdout);
        assert_same(
            &format!("driver({d}) pipe==file [C]"),
            &cp.stdout,
            &cf.stdout,
        );
        assert_same(
            &format!("driver({d}) pipe==file [Rust]"),
            &rp.stdout,
            &rf.stdout,
        );
    }

    // C23: unbuffered stdout (`setvbuf(stdout, NULL, _IONBF, 0)`) — the mode in
    // which the exact libc writer used (`printf` vs the compiler's `puts`
    // rewrite) would become observable.
    for d in [0, 1, 50, 99, 100, i32::MAX] {
        let arg = d.to_string();
        let c = run_subprocess(Which::C, "driver!nobuf", &arg).unwrap();
        let r = run_subprocess(Which::Rust, "driver!nobuf", &arg).unwrap();
        assert_eq!(c.code, r.code, "exit code (nobuf) for driver({d})");
        assert_eq!(c.signal, r.signal, "signal (nobuf) for driver({d})");
        assert_same(&format!("driver({d}) unbuffered"), &c.stdout, &r.stdout);
        // and it must equal the buffered output byte for byte
        let cb = run_subprocess(Which::C, "driver", &arg).unwrap();
        assert_same(
            &format!("driver({d}) buffered==unbuffered [C]"),
            &cb.stdout,
            &c.stdout,
        );
    }
    for payload in ["", "41", "254041", "25730a", "414141414141414141"] {
        let c = run_subprocess(Which::C, "printLine!nobuf", payload).unwrap();
        let r = run_subprocess(Which::Rust, "printLine!nobuf", payload).unwrap();
        assert_same(
            &format!("printLine({payload}) unbuffered"),
            &c.stdout,
            &r.stdout,
        );
    }

    // Same for the low-level entry point.
    for payload in ["", "41", "254041", "25730a"] {
        let cp = run_subprocess(Which::C, "printLine", payload).unwrap();
        let rp = run_subprocess(Which::Rust, "printLine", payload).unwrap();
        let cf = run_subprocess_to_file(Which::C, "printLine", payload).unwrap();
        let rf = run_subprocess_to_file(Which::Rust, "printLine", payload).unwrap();
        assert_same(&format!("printLine({payload}) pipe"), &cp.stdout, &rp.stdout);
        assert_same(&format!("printLine({payload}) file"), &cf.stdout, &rf.stdout);
        assert_eq!(cp.code, rp.code);
        assert_eq!(cf.code, rf.code);
    }
}
