//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both libraries are loaded as `.so` files and driven through their exported
//! C symbols only.

mod common;

use common::{Impl, Libs, Rng, assert_same, capture_stdout};
use std::ffi::c_char;

/// Helper: compare `driver(data)` between the two libraries.
fn diff_driver(libs: &Libs, data: i32) {
    let c = libs.call_driver(Impl::C, data);
    let r = libs.call_driver(Impl::Rust, data);
    assert_same(&format!("driver({data})"), &c, &r);
}

/// Helper: compare `printLine(bytes)` between the two libraries.
/// `bytes` must already contain its terminating NUL.
fn diff_print_line_raw(libs: &Libs, ctx: &str, bytes: &[u8]) {
    assert!(bytes.contains(&0), "test bug: buffer is not NUL terminated");
    let c = unsafe { libs.call_print_line(Impl::C, bytes.as_ptr() as *const c_char) };
    let r = unsafe { libs.call_print_line(Impl::Rust, bytes.as_ptr() as *const c_char) };
    assert_same(ctx, &c, &r);
}

fn diff_print_line(libs: &Libs, ctx: &str, s: &[u8]) {
    let mut buf = s.to_vec();
    buf.push(0);
    diff_print_line_raw(libs, ctx, &buf);
}

// ---------------------------------------------------------------- rows 1-2, 8

#[test]
fn cfg_01_driver_zero() {
    let libs = Libs::load();
    diff_driver(&libs, 0);
    // Sanity anchor against the C semantics: strncpy of 0 bytes, dest[0]=0.
    assert_eq!(libs.call_driver(Impl::C, 0), b"\n".to_vec());
}

#[test]
fn cfg_02_driver_one() {
    let libs = Libs::load();
    diff_driver(&libs, 1);
    assert_eq!(libs.call_driver(Impl::C, 1), b"A\n".to_vec());
}

#[test]
fn cfg_03_driver_interior_range_exhaustive() {
    let libs = Libs::load();
    for data in 2..=98 {
        diff_driver(&libs, data);
    }
}

// ------------------------------------------------------------------- row 4

#[test]
fn cfg_04_driver_max_accepted() {
    let libs = Libs::load();
    diff_driver(&libs, 99);
    let c = libs.call_driver(Impl::C, 99);
    let mut expected = vec![b'A'; 99];
    expected.push(b'\n');
    assert_eq!(c, expected, "C's own behaviour at the range boundary changed");
}

// --------------------------------------------------------------- rows 5-7

#[test]
fn cfg_05_driver_at_reject_boundary() {
    let libs = Libs::load();
    diff_driver(&libs, 100);
    assert_eq!(libs.call_driver(Impl::C, 100), b"\n".to_vec());
}

#[test]
fn cfg_06_driver_rejected_range_randomized() {
    let libs = Libs::load();
    diff_driver(&libs, 101);
    let mut rng = Rng::new(0xC0FFEE_1234);
    for _ in 0..500 {
        let data = rng.range_i32(101, i32::MAX);
        diff_driver(&libs, data);
    }
}

#[test]
fn cfg_07_driver_int_max() {
    let libs = Libs::load();
    diff_driver(&libs, i32::MAX);
    diff_driver(&libs, i32::MAX - 1);
}

// ------------------------------------------------------------------- row 8

#[test]
fn cfg_08_driver_accepted_range_randomized_sweep() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xDEADBEEF);

    // Shuffled full coverage of the accepted range...
    let mut all: Vec<i32> = (0..=99).collect();
    rng.shuffle(&mut all);
    for data in all {
        diff_driver(&libs, data);
    }
    // ...plus many independent random draws.
    for _ in 0..2000 {
        let data = rng.range_i32(0, 99);
        diff_driver(&libs, data);
    }
}

// ------------------------------------------------------------------- row 9

#[test]
fn cfg_09_driver_repeated_same_value() {
    let libs = Libs::load();
    for data in [0, 1, 50, 99, 100, 12345] {
        for _ in 0..5 {
            diff_driver(&libs, data);
        }
    }
}

// ------------------------------------------------------------------ row 10

#[test]
fn cfg_10_driver_interleaved_long_then_short() {
    let libs = Libs::load();
    // 99, 0, 98, 1, 97, 2, ... A `dest` that is not re-zeroed on every call
    // would leak the previous (longer) result into the shorter one.
    let mut lo = 0i32;
    let mut hi = 99i32;
    while lo <= hi {
        diff_driver(&libs, hi);
        diff_driver(&libs, lo);
        lo += 1;
        hi -= 1;
    }

    // Same idea but comparing whole multi-call streams rather than single calls.
    let seq = [99i32, 0, 99, 1, 100, 99, 2, 0, 98, 100];
    let cf = libs.driver(Impl::C);
    let c_stream = capture_stdout(|| {
        for &d in &seq {
            unsafe { cf(d) };
        }
    });
    drop(cf);
    let rf = libs.driver(Impl::Rust);
    let r_stream = capture_stdout(|| {
        for &d in &seq {
            unsafe { rf(d) };
        }
    });
    drop(rf);
    assert_same("driver() multi-call stream", &c_stream, &r_stream);
}

// -------------------------------------------------------------- rows 11-13

#[test]
fn cfg_11_print_line_null() {
    let libs = Libs::load();
    let c = unsafe { libs.call_print_line(Impl::C, std::ptr::null()) };
    let r = unsafe { libs.call_print_line(Impl::Rust, std::ptr::null()) };
    assert_same("printLine(NULL)", &c, &r);
    assert!(c.is_empty(), "C printLine(NULL) must emit nothing");
}

#[test]
fn cfg_12_print_line_empty() {
    let libs = Libs::load();
    diff_print_line(&libs, "printLine(\"\")", b"");
    let c = unsafe { libs.call_print_line(Impl::C, b"\0".as_ptr() as *const c_char) };
    assert_eq!(c, b"\n".to_vec());
}

#[test]
fn cfg_13_print_line_one_byte() {
    let libs = Libs::load();
    for b in [b'A', b'z', b'0', b' ', b'%', b'\\', 0x7f] {
        diff_print_line(&libs, &format!("printLine({:?})", b as char), &[b]);
    }
    // `%s`-style bytes must be treated as data, not as a format string.
    diff_print_line(&libs, "printLine(\"%s%d%n\")", b"%s%d%n");
    diff_print_line(&libs, "printLine(\"%%\")", b"%%");
}

// ------------------------------------------------------------------ row 14

#[test]
fn cfg_14_print_line_random_printable_ascii() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x5EED_0014);
    for i in 0..600 {
        let len = rng.range_i32(0, 200) as usize;
        let s: Vec<u8> = (0..len).map(|_| rng.byte_in(0x20, 0x7e)).collect();
        diff_print_line(&libs, &format!("printLine(random ascii #{i}, len {len})"), &s);
    }
}

// ------------------------------------------------------------------ row 15

#[test]
fn cfg_15_print_line_random_high_bytes() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x5EED_0015);
    for i in 0..600 {
        let len = rng.range_i32(1, 128) as usize;
        // 0x01..=0xFF, i.e. arbitrary non-NUL bytes including invalid UTF-8.
        let s: Vec<u8> = (0..len).map(|_| rng.byte_in(0x01, 0xff)).collect();
        diff_print_line(&libs, &format!("printLine(random raw bytes #{i}, len {len})"), &s);
    }
    // Explicit invalid-UTF-8 sequences.
    for s in [
        &[0x80u8][..],
        &[0xff, 0xfe][..],
        &[0xc3][..],
        &[0xed, 0xa0, 0x80][..],
        &[0xf4, 0x90, 0x80, 0x80][..],
    ] {
        diff_print_line(&libs, &format!("printLine(invalid utf8 {s:02x?})"), s);
    }
}

// ------------------------------------------------------------------ row 16

#[test]
fn cfg_16_print_line_embedded_nul() {
    let libs = Libs::load();
    diff_print_line_raw(&libs, "printLine(\"ab\\0cd\")", b"ab\0cd\0");
    diff_print_line_raw(&libs, "printLine(\"\\0hidden\")", b"\0hidden\0");
    let c = unsafe { libs.call_print_line(Impl::C, b"ab\0cd\0".as_ptr() as *const c_char) };
    assert_eq!(c, b"ab\n".to_vec(), "only the prefix before the NUL is printed");
}

// ------------------------------------------------------------------ row 17

#[test]
fn cfg_17_print_line_driver_shaped_buffer() {
    let libs = Libs::load();
    // Exactly the buffer driver(99) hands to printLine.
    let mut buf = vec![b'A'; 99];
    buf.push(0);
    diff_print_line_raw(&libs, "printLine(99 x 'A')", &buf);

    // And every length the driver can produce, fed to printLine directly.
    for n in 0..=99 {
        let s = vec![b'A'; n];
        diff_print_line(&libs, &format!("printLine({n} x 'A')"), &s);
    }
}

// ------------------------------------------------------------------ row 18

#[test]
fn cfg_18_print_line_long_string() {
    let libs = Libs::load();
    for len in [1023usize, 1024, 1025, 4095, 4096, 4097, 65536] {
        let s = vec![b'Z'; len];
        diff_print_line(&libs, &format!("printLine({len} x 'Z')"), &s);
    }
}

// ------------------------------------------------------------------ row 19

#[test]
fn cfg_19_interleaved_both_entry_points() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x5EED_0019);

    // Build one shared script of operations, then replay it against each
    // library and compare the *whole* accumulated stdout stream.
    enum Op {
        Driver(i32),
        Line(Vec<u8>),
        Null,
    }
    let mut ops = Vec::new();
    for _ in 0..300 {
        match rng.next_u64() % 4 {
            0 => ops.push(Op::Driver(rng.range_i32(0, 99))),
            1 => ops.push(Op::Driver(rng.range_i32(100, i32::MAX))),
            2 => {
                let len = rng.range_i32(0, 60) as usize;
                let mut s: Vec<u8> = (0..len).map(|_| rng.byte_in(0x21, 0x7e)).collect();
                s.push(0);
                ops.push(Op::Line(s));
            }
            _ => ops.push(Op::Null),
        }
    }

    let run = |which: Impl| -> Vec<u8> {
        let d = libs.driver(which);
        let p = libs.print_line(which);
        capture_stdout(|| {
            for op in &ops {
                unsafe {
                    match op {
                        Op::Driver(n) => d(*n),
                        Op::Line(s) => p(s.as_ptr() as *const c_char),
                        Op::Null => p(std::ptr::null()),
                    }
                }
            }
        })
    };
    let c = run(Impl::C);
    let r = run(Impl::Rust);
    assert_same("interleaved driver()/printLine() stream", &c, &r);
    assert!(!c.is_empty());
}
