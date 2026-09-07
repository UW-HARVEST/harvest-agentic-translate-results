//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH shared objects through `libloading` and compares the
//! bytes each writes to `stdout`.

mod common;

use common::*;
use std::ffi::c_char;

// ---------------------------------------------------------------------- C1
#[test]
fn cfg_c1_print_line_empty() {
    let buf = CBuf::new(b"");
    assert_same_and_eq("C1 empty string", b"\n", |api| api.print_line(buf.as_ptr()));
}

// ---------------------------------------------------------------------- C2
#[test]
fn cfg_c2_print_line_single_byte_all_values() {
    for b in 1u8..=255 {
        let buf = CBuf::new(&[b]);
        let expected = [b, b'\n'];
        assert_same_and_eq(&format!("C2 single byte 0x{b:02x}"), &expected, |api| {
            api.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------- C3
#[test]
fn cfg_c3_print_line_random_ascii() {
    let mut rng = Rng::new(SEED ^ 3);
    for case in 0..512 {
        let len = 1 + rng.below(64);
        let bytes: Vec<u8> = (0..len).map(|_| 0x20 + rng.below(0x5f) as u8).collect();
        let buf = CBuf::new(&bytes);
        let mut expected = bytes.clone();
        expected.push(b'\n');
        assert_same_and_eq(&format!("C3 random ascii #{case}"), &expected, |api| {
            api.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------- C4
#[test]
fn cfg_c4_print_line_random_arbitrary_bytes() {
    let mut rng = Rng::new(SEED ^ 4);
    for case in 0..512 {
        let len = 1 + rng.below(64);
        // 0x01..=0xff — deliberately includes invalid UTF-8 sequences.
        let bytes: Vec<u8> = (0..len).map(|_| 1 + rng.below(255) as u8).collect();
        let buf = CBuf::new(&bytes);
        let mut expected = bytes.clone();
        expected.push(b'\n');
        assert_same_and_eq(&format!("C4 random bytes #{case}"), &expected, |api| {
            api.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------- C5
#[test]
fn cfg_c5_print_line_format_specifiers() {
    let cases: &[&[u8]] = &[
        b"%s",
        b"%d",
        b"%n",
        b"%%",
        b"%p",
        b"%10000d",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n",
        b"100%% done",
        b"%.*s",
        b"%hhn",
        b"%1$s %2$s",
        b"\\%s\\",
    ];
    for (i, c) in cases.iter().enumerate() {
        let buf = CBuf::new(c);
        let mut expected = c.to_vec();
        expected.push(b'\n');
        assert_same_and_eq(&format!("C5 fmt specifier #{i}"), &expected, |api| {
            api.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------- C6
#[test]
fn cfg_c6_print_line_whitespace_shapes() {
    let cases: &[&[u8]] = &[
        b"\n",
        b"\n\n\n",
        b"a\nb",
        b"a\r\nb",
        b"\t",
        b"  leading",
        b"trailing  ",
        b"mixed \t\r\n end",
        b"\x0b\x0c",
        b"line1\nline2\nline3\n",
    ];
    for (i, c) in cases.iter().enumerate() {
        let buf = CBuf::new(c);
        let mut expected = c.to_vec();
        expected.push(b'\n');
        assert_same_and_eq(&format!("C6 whitespace #{i}"), &expected, |api| {
            api.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------- C7
#[test]
fn cfg_c7_print_line_long_boundaries() {
    let mut rng = Rng::new(SEED ^ 7);
    for &len in &[1usize, 2, 511, 512, 513, 1023, 1024, 1025, 4095, 4096, 4097, 8192, 65537] {
        let bytes: Vec<u8> = (0..len).map(|_| 1 + rng.below(255) as u8).collect();
        let buf = CBuf::new(&bytes);
        let mut expected = bytes.clone();
        expected.push(b'\n');
        assert_same_and_eq(&format!("C7 length {len}"), &expected, |api| {
            api.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------- C8
#[test]
fn cfg_c8_print_line_null_branch() {
    assert_same_and_eq("C8 NULL branch", b"", |api| {
        api.print_line(std::ptr::null::<c_char>())
    });
}

// ---------------------------------------------------------------------- C9
#[test]
fn cfg_c9_print_int_line_boundaries() {
    let values: &[i32] = &[
        0,
        1,
        -1,
        9,
        10,
        -9,
        -10,
        99,
        100,
        -99,
        -100,
        999,
        1000,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        1 << 16,
        -(1 << 16),
    ];
    for &v in values {
        let expected = format!("{v}\n").into_bytes();
        assert_same_and_eq(&format!("C9 int {v}"), &expected, |api| api.print_int_line(v));
    }
}

// ---------------------------------------------------------------------- C10
#[test]
fn cfg_c10_print_int_line_random() {
    let mut rng = Rng::new(SEED ^ 10);
    for case in 0..1024 {
        let v = rng.next_i32();
        let expected = format!("{v}\n").into_bytes();
        assert_same_and_eq(&format!("C10 random int #{case} = {v}"), &expected, |api| {
            api.print_int_line(v)
        });
    }
}

// ---------------------------------------------------------------------- C11
#[test]
fn cfg_c11_print_int_line_random_small() {
    let mut rng = Rng::new(SEED ^ 11);
    for case in 0..512 {
        let v = rng.range_i32(-1000, 1000);
        let expected = format!("{v}\n").into_bytes();
        assert_same_and_eq(&format!("C11 small int #{case} = {v}"), &expected, |api| {
            api.print_int_line(v)
        });
    }
}

// ---------------------------------------------------------------------- C12
#[test]
fn cfg_c12_good_single_call() {
    assert_same_and_eq("C12 good()", b"0\n2\n", |api| api.good());
}

// ---------------------------------------------------------------------- C13
#[test]
fn cfg_c13_bad_single_call() {
    // CWE-482: `intOne + intTwo;` is a discarded expression, so intSum stays 0.
    assert_same_and_eq("C13 bad()", b"0\n0\n", |api| api.bad());
}

// ---------------------------------------------------------------------- C14
#[test]
fn cfg_c14_driver_single_call() {
    let expected = b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n";
    assert_same_and_eq("C14 driver()", expected, |api| api.driver());
}

// ---------------------------------------------------------------------- C15
#[test]
fn cfg_c15_driver_repeated() {
    let one = "Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n";
    let expected = one.repeat(8).into_bytes();
    assert_same_and_eq("C15 driver() x8", &expected, |api| {
        for _ in 0..8 {
            api.driver();
        }
    });
}

// ---------------------------------------------------------------------- C16
#[test]
fn cfg_c16_good_bad_interleaved() {
    let expected = b"0\n2\n0\n0\n0\n0\n0\n2\n0\n2\n0\n0\n";
    assert_same_and_eq("C16 good/bad interleaved", expected, |api| {
        api.good();
        api.bad();
        api.bad();
        api.good();
        api.good();
        api.bad();
    });
}

// ---------------------------------------------------------------------- C17
#[test]
fn cfg_c17_random_interleaving_all_entry_points() {
    // Build one randomized program, then replay the identical program against
    // both shared objects. 8 independent programs of 256 steps each.
    #[derive(Clone)]
    enum Step {
        Line(Vec<u8>),
        NullLine,
        Int(i32),
        Bad,
        Good,
        Driver,
    }

    let mut rng = Rng::new(SEED ^ 17);
    for prog in 0..8 {
        let mut steps = Vec::new();
        for _ in 0..256 {
            steps.push(match rng.below(6) {
                0 => {
                    let len = rng.below(40);
                    Step::Line((0..len).map(|_| 1 + rng.below(255) as u8).collect())
                }
                1 => Step::NullLine,
                2 => Step::Int(rng.next_i32()),
                3 => Step::Bad,
                4 => Step::Good,
                _ => Step::Driver,
            });
        }
        // Pre-allocate the NUL-terminated buffers so both runs use identical bytes.
        let bufs: Vec<Option<CBuf>> = steps
            .iter()
            .map(|s| match s {
                Step::Line(b) => Some(CBuf::new(b)),
                _ => None,
            })
            .collect();

        assert_same(&format!("C17 random program #{prog}"), |api| {
            for (s, b) in steps.iter().zip(bufs.iter()) {
                match s {
                    Step::Line(_) => api.print_line(b.as_ref().unwrap().as_ptr()),
                    Step::NullLine => api.print_line(std::ptr::null()),
                    Step::Int(v) => api.print_int_line(*v),
                    Step::Bad => api.bad(),
                    Step::Good => api.good(),
                    Step::Driver => api.driver(),
                }
            }
        });
    }
}

// ---------------------------------------------------------------------- C18
#[test]
fn cfg_c18_leaf_interleaving_puts_vs_printf() {
    // The C compiler lowers `printf("%s\n", s)` to `puts(s)` while
    // `printf("%d\n", n)` stays `printf`. Interleaving the two leaves exercises
    // both stdio paths on the same stream.
    let mut rng = Rng::new(SEED ^ 18);
    for case in 0..128 {
        let n = 1 + rng.below(16);
        let mut bufs = Vec::new();
        let mut ints = Vec::new();
        let mut expected = Vec::new();
        for _ in 0..n {
            let len = rng.below(24);
            let bytes: Vec<u8> = (0..len).map(|_| 1 + rng.below(255) as u8).collect();
            expected.extend_from_slice(&bytes);
            expected.push(b'\n');
            bufs.push(CBuf::new(&bytes));
            let v = rng.next_i32();
            expected.extend_from_slice(format!("{v}\n").as_bytes());
            ints.push(v);
        }
        assert_same_and_eq(&format!("C18 leaf interleave #{case}"), &expected, |api| {
            for (b, v) in bufs.iter().zip(ints.iter()) {
                api.print_line(b.as_ptr());
                api.print_int_line(*v);
            }
        });
    }
}
