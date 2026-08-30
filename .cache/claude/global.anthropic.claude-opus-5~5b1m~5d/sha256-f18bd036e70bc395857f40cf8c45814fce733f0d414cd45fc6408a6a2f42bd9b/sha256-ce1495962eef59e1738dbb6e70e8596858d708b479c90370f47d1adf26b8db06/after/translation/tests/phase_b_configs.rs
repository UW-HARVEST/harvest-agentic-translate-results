// Phase B -- valid-path differential tests.
// One test per row of CONFIGS.md. Every call goes through the `.so` exports of
// BOTH libraries; outputs are compared byte-for-byte.

mod harness;
use harness::*;

// ---------------------------------------------------------------- printIntLine

/// C1: the value the rest of the library actually prints.
#[test]
fn cfg_c1_print_int_zero() {
    assert_same("C1 printIntLine(0)", |api| api.print_int_line(0));
}

/// C2: digit-count transitions at small magnitudes.
#[test]
fn cfg_c2_print_int_small() {
    let vals: Vec<i32> = vec![1, -1, 9, 10, -9, -10, 99, 100, -100, 999, 1000, -999, -1000];
    for v in &vals {
        assert_same(&format!("C2 printIntLine({v})"), |api| api.print_int_line(*v));
    }
    assert_same_chunked("C2 batch", &vals, 8, |api, v| api.print_int_line(*v));
}

/// C3: `%d` boundaries, incl. the non-negatable INT_MIN.
#[test]
fn cfg_c3_print_int_boundaries() {
    let vals: Vec<i32> = vec![
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        -2147483647,
        2147483646,
        0,
    ];
    for v in &vals {
        assert_same(&format!("C3 printIntLine({v})"), |api| api.print_int_line(*v));
    }
}

/// C4: +/- powers of two and their neighbours.
#[test]
fn cfg_c4_print_int_powers_of_two() {
    let mut vals: Vec<i32> = Vec::new();
    for k in 0..32u32 {
        let p = 1i64 << k;
        for d in [-1i64, 0, 1] {
            let v = p + d;
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                vals.push(v as i32);
            }
            let v = -p + d;
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                vals.push(v as i32);
            }
        }
    }
    assert_same_chunked("C4 powers of two", &vals, 16, |api, v| api.print_int_line(*v));
}

/// C5: 4096 randomized full-range i32 values.
#[test]
fn cfg_c5_print_int_random() {
    let mut rng = Rng::new(SEED);
    let vals: Vec<i32> = (0..4096).map(|_| rng.next_i32()).collect();
    assert_same_chunked("C5 random ints", &vals, 64, |api, v| api.print_int_line(*v));
}

/// C6: 512 randomized values inside ONE capture (ordering + buffering).
#[test]
fn cfg_c6_print_int_batched() {
    let mut rng = Rng::new(SEED ^ 0xC6);
    let vals: Vec<i32> = (0..512).map(|_| rng.next_i32()).collect();
    assert_same("C6 512 ints, one capture", |api| {
        for v in &vals {
            api.print_int_line(*v);
        }
    });
}

// ------------------------------------------------------------------- printLine

/// C7: non-NULL, empty payload.
#[test]
fn cfg_c7_print_line_empty() {
    assert_same_line("C7 printLine(\"\")", b"");
}

/// C8: every single non-NUL byte value as a 1-byte payload.
#[test]
fn cfg_c8_print_line_every_single_byte() {
    for b in 1u8..=255 {
        assert_same_line(&format!("C8 printLine(0x{b:02x})"), &[b]);
    }
}

/// C9: plain ASCII.
#[test]
fn cfg_c9_print_line_ascii() {
    for p in [
        &b"hello"[..],
        b"hello world",
        b"  leading and trailing  ",
        b"a b\tc",
        b"The quick brown fox jumps over the lazy dog.",
        b"0123456789",
        b"~!@#^&*()_+-=[]{}|;:',.<>/?\\\"",
    ] {
        assert_same_line("C9 ascii", p);
    }
}

/// C10: payload full of printf conversion specifiers -- it is DATA, and must
/// never be interpreted as a format string.
#[test]
fn cfg_c10_print_line_percent_payload() {
    for p in [
        &b"%s"[..],
        b"%d",
        b"%n",
        b"%%",
        b"%.9999f",
        b"%s %s %s %s %s %s %s %s",
        b"%n%n%n%n",
        b"100%",
        b"%",
        b"%1$s %2$d",
        b"%*d",
        b"%hhn %hn %ln %lln",
        b"%p %x %X %o %e %g %a %c",
    ] {
        assert_same_line("C10 percent payload", p);
    }
}

/// C11: control bytes, incl. embedded newlines.
#[test]
fn cfg_c11_print_line_control_bytes() {
    for p in [
        &b"\n"[..],
        b"\r",
        b"\t",
        b"\x0b\x0c",
        b"\x1b[31mred\x1b[0m",
        b"\x07bell",
        b"line1\nline2\nline3",
        b"crlf\r\nend",
        b"\x01\x02\x03\x04\x05\x06\x08\x0e\x0f\x10\x1f\x7f",
    ] {
        assert_same_line("C11 control bytes", p);
    }
}

/// C12: non-UTF-8 / high bytes -- printf is byte-oriented, so these must pass
/// through verbatim.
#[test]
fn cfg_c12_print_line_non_utf8() {
    let mut cases: Vec<Vec<u8>> = vec![
        vec![0x80],                   // lone continuation byte
        vec![0xff],                   // never valid in UTF-8
        vec![0xfe, 0xff],             // invalid
        vec![0xc0, 0x80],             // overlong encoding of NUL
        vec![0xe0, 0x80, 0x80],       // overlong
        vec![0xed, 0xa0, 0x80],       // UTF-16 surrogate half
        vec![0xf5, 0x80, 0x80, 0x80], // > U+10FFFF
        vec![0xc2],                   // truncated 2-byte sequence
        vec![0xe2, 0x82],             // truncated 3-byte sequence
        vec![0xf0, 0x9f, 0x92],       // truncated 4-byte sequence
        (0x80u8..=0xff).collect(),    // every high byte
    ];
    // Mixed ASCII / high bytes.
    cases.push(b"ok\xffbad\x80mix".to_vec());
    for p in &cases {
        assert_same_line("C12 non-utf8", p);
    }
}

/// C13: valid multi-byte UTF-8.
#[test]
fn cfg_c13_print_line_utf8() {
    for s in [
        "é",
        "ñandú",
        "日本語テキスト",
        "Ελληνικά",
        "Русский",
        "🦀 rust 🚀 emoji 🎉",
        "e\u{301}\u{302}\u{303} combining",
        "\u{10FFFF}",
        "\u{FEFF}bom",
        "mixed ascii + 日本語 + 🦀",
    ] {
        assert_same_line("C13 utf8", s.as_bytes());
    }
}

/// C14: length sweep across every plausible stdio buffer boundary.
#[test]
fn cfg_c14_print_line_length_sweep() {
    let mut lens: Vec<usize> = vec![];
    for base in [1usize, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 8190] {
        for d in [-1isize, 0, 1] {
            let l = base as isize + d;
            if l >= 0 {
                lens.push(l as usize);
            }
        }
    }
    lens.push(63);
    lens.push(65);
    lens.push(127);
    lens.push(129);
    lens.push(255);
    lens.push(257);
    lens.sort_unstable();
    lens.dedup();

    let mut rng = Rng::new(SEED ^ 0xC14);
    for len in lens {
        // A repeating ASCII pattern...
        let pat: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        assert_same_line(&format!("C14 pattern len {len}"), &pat);
        // ...and random non-NUL bytes of the same length.
        let rnd = rng.bytes(len);
        assert_same_line(&format!("C14 random len {len}"), &rnd);
    }
}

/// C15: payloads far larger than any stdio buffer.
#[test]
fn cfg_c15_print_line_huge() {
    let mut rng = Rng::new(SEED ^ 0xC15);
    for len in [64 * 1024usize, 256 * 1024, 1024 * 1024] {
        let p = rng.bytes(len);
        assert_same_line(&format!("C15 huge len {len}"), &p);
    }
}

/// C16: 512 randomized payloads, random length and random non-NUL bytes.
#[test]
fn cfg_c16_print_line_random() {
    let mut rng = Rng::new(SEED ^ 0xC16);
    for i in 0..512 {
        let len = rng.below(301) as usize;
        let p = rng.bytes(len);
        assert_same_line(&format!("C16 random payload #{i} (len {len})"), &p);
    }
}

/// C17: 256 randomized payloads issued back-to-back in ONE capture.
#[test]
fn cfg_c17_print_line_batched() {
    let mut rng = Rng::new(SEED ^ 0xC17);
    let payloads: Vec<Vec<u8>> = (0..256)
        .map(|_| {
            let len = rng.below(64) as usize;
            rng.bytes(len)
        })
        .collect();
    assert_same("C17 256 payloads, one capture", |api| {
        for p in &payloads {
            with_cstr(p, |ptr| api.print_line(ptr));
        }
    });
}

// ------------------------------------------------------------- good / bad / driver

/// C18: the low-level `good()` entry point, called directly.
#[test]
fn cfg_c18_good_direct() {
    assert_same("C18 good()", |api| api.good());
}

/// C19: the low-level `bad()` entry point (the `alloca(10)` path), directly.
#[test]
fn cfg_c19_bad_direct() {
    assert_same("C19 bad()", |api| api.bad());
}

/// C20: 64 alternating good/bad calls in ONE capture -- a frame corrupted by
/// `bad()`'s out-of-bounds write would surface on a later call.
#[test]
fn cfg_c20_good_bad_alternating() {
    assert_same("C20 alternating good/bad x64", |api| {
        for i in 0..64 {
            if i % 2 == 0 {
                api.good();
            } else {
                api.bad();
            }
        }
    });
    assert_same("C20 bad x64 then good x64", |api| {
        for _ in 0..64 {
            api.bad();
        }
        for _ in 0..64 {
            api.good();
        }
    });
}

/// C21: driver(1) -> good().
#[test]
fn cfg_c21_driver_true() {
    assert_same("C21 driver(1)", |api| api.driver(1));
}

/// C22: driver(0) -> bad().
#[test]
fn cfg_c22_driver_false() {
    assert_same("C22 driver(0)", |api| api.driver(0));
}

/// C23: 1024 randomized flags, with 0 deliberately mixed in.
#[test]
fn cfg_c23_driver_random_flag() {
    let mut rng = Rng::new(SEED ^ 0xC23);
    let flags: Vec<i32> = (0..1024)
        .map(|i| if i % 7 == 0 { 0 } else { rng.next_i32() })
        .collect();
    assert_same_chunked("C23 random driver flags", &flags, 64, |api, f| api.driver(*f));
}

/// C24: randomized flag sequence in ONE capture (mode switching mid-stream).
#[test]
fn cfg_c24_driver_random_sequence() {
    let mut rng = Rng::new(SEED ^ 0xC24);
    let flags: Vec<i32> = (0..512)
        .map(|_| match rng.below(4) {
            0 => 0,
            1 => 1,
            2 => -1,
            _ => rng.next_i32(),
        })
        .collect();
    assert_same("C24 driver flag sequence, one capture", |api| {
        for f in &flags {
            api.driver(*f);
        }
    });
}

/// C25: randomized interleaving of ALL FIVE entry points in ONE capture --
/// the composed pipeline, including ordering and buffering across functions.
#[test]
fn cfg_c25_all_entry_points_interleaved() {
    #[derive(Clone)]
    enum Step {
        Line(Vec<u8>),
        Int(i32),
        Good,
        Bad,
        Driver(i32),
    }

    let mut rng = Rng::new(SEED ^ 0xC25);
    let steps: Vec<Step> = (0..2000)
        .map(|_| match rng.below(6) {
            0 => {
                let len = rng.below(40) as usize;
                Step::Line(rng.bytes(len))
            }
            1 => Step::Int(rng.next_i32()),
            2 => Step::Good,
            3 => Step::Bad,
            4 => Step::Driver(if rng.below(2) == 0 { 0 } else { rng.next_i32() }),
            _ => Step::Line(b"%s%n interleaved \xff\x80".to_vec()),
        })
        .collect();

    let run = |api: &Api, group: &[Step]| {
        for s in group {
            match s {
                Step::Line(p) => with_cstr(p, |ptr| api.print_line(ptr)),
                Step::Int(v) => api.print_int_line(*v),
                Step::Good => api.good(),
                Step::Bad => api.bad(),
                Step::Driver(f) => api.driver(*f),
            }
        }
    };

    // Whole sequence in one capture...
    assert_same("C25 2000 interleaved steps, one capture", |api| {
        run(api, &steps)
    });
    // ...and in chunks, so a divergence is easy to localise.
    for (i, group) in steps.chunks(50).enumerate() {
        assert_same(&format!("C25 interleaved chunk {i}"), |api| run(api, group));
    }
}
