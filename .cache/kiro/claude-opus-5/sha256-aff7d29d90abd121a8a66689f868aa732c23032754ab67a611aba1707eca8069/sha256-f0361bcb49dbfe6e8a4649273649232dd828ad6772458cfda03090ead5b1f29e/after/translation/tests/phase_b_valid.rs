//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call goes through `dlsym` on the two
//! `.so`s; the captured `stdout` bytes must be identical.

mod common;

use common::{Impl, Pcg32, SEED, api, assert_same, capture_stdout, cstr, render};

// ---------------------------------------------------------------------------
// Row 1 — printLine: length 0 and 1, every single byte value 0x01..=0xFF
// ---------------------------------------------------------------------------
#[test]
fn cfg_row1_print_line_tiny_all_bytes() {
    assert_same("row1/empty", |a| unsafe {
        let s = cstr(b"");
        (a.print_line)(s.as_ptr());
    });

    for b in 1u8..=255 {
        let payload = [b];
        assert_same(&format!("row1/byte 0x{b:02x}"), |a| unsafe {
            let s = cstr(&payload);
            (a.print_line)(s.as_ptr());
        });
    }
}

// ---------------------------------------------------------------------------
// Row 2 — printLine: random printable-ASCII payloads, len 2..=64
// ---------------------------------------------------------------------------
#[test]
fn cfg_row2_print_line_random_ascii() {
    let mut rng = Pcg32::new(SEED ^ 2);
    for i in 0..512 {
        let len = rng.range(2, 64) as usize;
        let payload = rng.ascii(len);
        assert_same(&format!("row2/iter {i} len {len}"), |a| unsafe {
            let s = cstr(&payload);
            (a.print_line)(s.as_ptr());
        });
    }
}

// ---------------------------------------------------------------------------
// Row 3 — printLine: random arbitrary bytes (non-UTF-8 included), len 1..=256
// ---------------------------------------------------------------------------
#[test]
fn cfg_row3_print_line_random_bytes() {
    let mut rng = Pcg32::new(SEED ^ 3);
    for i in 0..512 {
        let len = rng.range(1, 256) as usize;
        let payload = rng.bytes_nonzero(len);
        assert_same(&format!("row3/iter {i} len {len}"), |a| unsafe {
            let s = cstr(&payload);
            (a.print_line)(s.as_ptr());
        });
    }
}

// ---------------------------------------------------------------------------
// Row 4 — printLine: printf format directives must be copied, not interpreted
// ---------------------------------------------------------------------------
#[test]
fn cfg_row4_print_line_format_directives() {
    let cases: &[&[u8]] = &[
        b"%s",
        b"%d",
        b"%n",
        b"%%",
        b"%p %p %p %p",
        b"%s%s%s%s%s%s%s%s",
        b"%1000000d",
        b"%.2147483647f",
        b"%*d",
        b"%hhn%hn%n%ln%lln",
        b"100%% done: %s -> %d",
        b"%",
        b"a%",
        b"%\x80%\xff",
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_same(&format!("row4/case {i} {:?}", render(c)), |a| unsafe {
            let s = cstr(c);
            (a.print_line)(s.as_ptr());
        });
    }

    // Randomized: sprinkle directives into random ASCII.
    let mut rng = Pcg32::new(SEED ^ 4);
    let frags: &[&[u8]] = &[b"%s", b"%d", b"%n", b"%%", b"%x", b"%p", b"%"];
    for i in 0..256 {
        let mut payload = Vec::new();
        let parts = rng.range(1, 8);
        for _ in 0..parts {
            let n = rng.range(0, 6) as usize;
            payload.extend_from_slice(&rng.ascii(n));
            payload.extend_from_slice(frags[rng.range(0, frags.len() as u32 - 1) as usize]);
        }
        assert_same(&format!("row4/rand {i}"), |a| unsafe {
            let s = cstr(&payload);
            (a.print_line)(s.as_ptr());
        });
    }
}

// ---------------------------------------------------------------------------
// Row 5 — printLine: embedded control bytes / newlines
// ---------------------------------------------------------------------------
#[test]
fn cfg_row5_print_line_control_bytes() {
    let cases: &[&[u8]] = &[
        b"\n",
        b"\n\n\n",
        b"a\nb",
        b"trailing\n",
        b"\nleading",
        b"\r\n",
        b"a\r\nb\r\n",
        b"tab\there",
        b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f",
        b"\x7f\x1b[31mred\x1b[0m",
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_same(&format!("row5/case {i}"), |a| unsafe {
            let s = cstr(c);
            (a.print_line)(s.as_ptr());
        });
    }

    // Randomized mixture of control bytes and printable ASCII.
    let mut rng = Pcg32::new(SEED ^ 5);
    for i in 0..256 {
        let len = rng.range(1, 96) as usize;
        let payload: Vec<u8> = (0..len)
            .map(|_| {
                if rng.range(0, 2) == 0 {
                    rng.range(1, 0x1f) as u8
                } else {
                    rng.range(0x20, 0x7e) as u8
                }
            })
            .collect();
        assert_same(&format!("row5/rand {i} len {len}"), |a| unsafe {
            let s = cstr(&payload);
            (a.print_line)(s.as_ptr());
        });
    }
}

// ---------------------------------------------------------------------------
// Row 6 — printLine: large payloads straddling stdio buffer sizes
// ---------------------------------------------------------------------------
#[test]
fn cfg_row6_print_line_large() {
    let sizes = [
        1usize, 63, 64, 65, 127, 128, 129, 511, 512, 513, 1023, 1024, 1025, 4095, 4096, 4097, 8191,
        8192, 8193, 65535, 65536, 65537, 1_048_576,
    ];
    let mut rng = Pcg32::new(SEED ^ 6);
    for &n in &sizes {
        // Deterministic non-UTF-8-containing filler.
        let payload = rng.bytes_nonzero(n);
        assert_same(&format!("row6/size {n}"), |a| unsafe {
            let s = cstr(&payload);
            (a.print_line)(s.as_ptr());
        });
    }
}

// ---------------------------------------------------------------------------
// Row 7 — printIntLine: boundary integers
// ---------------------------------------------------------------------------
#[test]
fn cfg_row7_print_int_line_boundaries() {
    let vals: &[i32] = &[
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -1_000_000_000,
        -100_000,
        -1000,
        -100,
        -10,
        -9,
        -1,
        0,
        1,
        9,
        10,
        99,
        100,
        999,
        1000,
        100_000,
        1_000_000_000,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &v in vals {
        assert_same(&format!("row7/{v}"), |a| unsafe {
            (a.print_int_line)(v);
        });
    }
}

// ---------------------------------------------------------------------------
// Row 8 — printIntLine: 4096 uniformly random i32
// ---------------------------------------------------------------------------
#[test]
fn cfg_row8_print_int_line_random() {
    let mut rng = Pcg32::new(SEED ^ 8);
    // Batch them into one capture per chunk to keep the test fast while still
    // covering every value; a divergence anywhere fails the byte comparison.
    for chunk in 0..32 {
        let vals: Vec<i32> = (0..128).map(|_| rng.next_i32()).collect();
        assert_same(&format!("row8/chunk {chunk}"), |a| unsafe {
            for &v in &vals {
                (a.print_int_line)(v);
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Row 9 — printIntLine: small magnitudes (-999..=999)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row9_print_int_line_small() {
    let mut rng = Pcg32::new(SEED ^ 9);
    for chunk in 0..8 {
        let vals: Vec<i32> = (0..128).map(|_| rng.range(0, 1998) as i32 - 999).collect();
        assert_same(&format!("row9/chunk {chunk}"), |a| unsafe {
            for &v in &vals {
                (a.print_int_line)(v);
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Row 10 — good(), single call
// ---------------------------------------------------------------------------
#[test]
fn cfg_row10_good_single() {
    assert_same("row10/good", |a| unsafe { (a.good)() });

    // Pin the absolute expected bytes too, so a *mutual* regression can't hide.
    let out = capture_stdout(|| unsafe { (api(Impl::C).good)() });
    assert_eq!(out, b"0\n2\n", "C good() baseline changed: {}", render(&out));
}

// ---------------------------------------------------------------------------
// Row 11 — bad(), single call (the discarded-sum bug must be reproduced)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row11_bad_single() {
    assert_same("row11/bad", |a| unsafe { (a.bad)() });

    let out = capture_stdout(|| unsafe { (api(Impl::C).bad)() });
    assert_eq!(
        out,
        b"0\n0\n",
        "C bad() baseline changed: {}",
        render(&out)
    );
}

// ---------------------------------------------------------------------------
// Row 12 — driver(), the full end-to-end pipeline
// ---------------------------------------------------------------------------
#[test]
fn cfg_row12_driver_single() {
    assert_same("row12/driver", |a| unsafe { (a.driver)() });

    let out = capture_stdout(|| unsafe { (api(Impl::C).driver)() });
    let expected: &[u8] = b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n";
    assert_eq!(
        out,
        expected,
        "C driver() baseline changed: {}",
        render(&out)
    );
}

// ---------------------------------------------------------------------------
// Row 13 — good/bad randomly interleaved, 256 calls in one capture
// ---------------------------------------------------------------------------
#[test]
fn cfg_row13_good_bad_interleaved() {
    let mut rng = Pcg32::new(SEED ^ 13);
    let plan: Vec<bool> = (0..256).map(|_| rng.range(0, 1) == 1).collect();
    assert_same("row13/good+bad interleaved", |a| unsafe {
        for &use_good in &plan {
            if use_good {
                (a.good)()
            } else {
                (a.bad)()
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Row 14 — driver() called 8 times in one capture
// ---------------------------------------------------------------------------
#[test]
fn cfg_row14_driver_repeated() {
    assert_same("row14/driver x8", |a| unsafe {
        for _ in 0..8 {
            (a.driver)();
        }
    });
}

// ---------------------------------------------------------------------------
// Row 15 — all five entry points randomly interleaved, 512 calls
// ---------------------------------------------------------------------------
#[test]
fn cfg_row15_all_entry_points_interleaved() {
    enum Step {
        Line(Vec<u8>),
        NullLine,
        Int(i32),
        Good,
        Bad,
        Driver,
    }

    let mut rng = Pcg32::new(SEED ^ 15);
    let plan: Vec<Step> = (0..512)
        .map(|_| match rng.range(0, 5) {
            0 => {
                let n = rng.range(0, 48) as usize;
                Step::Line(rng.bytes_nonzero(n))
            }
            1 => Step::NullLine,
            2 => Step::Int(rng.next_i32()),
            3 => Step::Good,
            4 => Step::Bad,
            _ => Step::Driver,
        })
        .collect();

    assert_same("row15/all entry points", |a| unsafe {
        for step in &plan {
            match step {
                Step::Line(p) => {
                    let s = cstr(p);
                    (a.print_line)(s.as_ptr());
                }
                Step::NullLine => (a.print_line)(std::ptr::null()),
                Step::Int(v) => (a.print_int_line)(*v),
                Step::Good => (a.good)(),
                Step::Bad => (a.bad)(),
                Step::Driver => (a.driver)(),
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Binary-equivalent end-to-end check.
//
// Neither project builds an executable (no `add_executable` in CMakeLists.txt,
// no `[[bin]]`/`src/main.rs`), so this stands in for the "compare the two
// binaries' stdout" gate: it runs the top-level entry point a real `main` would
// call and compares the complete byte stream.
// ---------------------------------------------------------------------------
#[test]
fn driver_stdout_parity() {
    let c = capture_stdout(|| unsafe { (api(Impl::C).driver)() });
    let rs = capture_stdout(|| unsafe { (api(Impl::Rust).driver)() });
    assert_eq!(
        c,
        rs,
        "driver() stdout differs\n  C:    {}\n  Rust: {}",
        render(&c),
        render(&rs)
    );
    assert!(!c.is_empty(), "capture harness produced no output at all");
}
