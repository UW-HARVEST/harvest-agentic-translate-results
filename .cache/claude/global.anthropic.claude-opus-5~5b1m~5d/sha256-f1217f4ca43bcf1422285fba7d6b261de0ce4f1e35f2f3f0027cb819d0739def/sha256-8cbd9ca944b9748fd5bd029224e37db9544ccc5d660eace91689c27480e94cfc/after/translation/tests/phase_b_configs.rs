//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//!
//! Every test loads BOTH `c_src/build/libdriver.so` and the Rust
//! `target/*/libdriver.so` via `libloading` and compares captured stdout
//! byte-for-byte.

mod common;
use common::*;
use std::ffi::c_char;

// --------------------------------------------------------------------- row 1
#[test]
fn row01_print_line_empty_string() {
    let buf = cstr(b"");
    assert_same("row01 printLine(\"\")", |api| unsafe {
        (api.print_line)(buf.as_ptr() as *const c_char)
    });
}

// --------------------------------------------------------------------- row 2
#[test]
fn row02_print_line_every_single_byte_value() {
    // All 255 possible one-byte C strings.
    for b in 1u8..=255 {
        let buf = cstr(&[b]);
        assert_same(&format!("row02 printLine(single byte {b:#04x})"), |api| unsafe {
            (api.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

// --------------------------------------------------------------------- row 3
#[test]
fn row03_print_line_random_short_ascii() {
    let mut rng = Rng::new(0x5EED_0003);
    for i in 0..256 {
        let len = 1 + rng.below(32) as usize;
        let body: Vec<u8> = (0..len).map(|_| rng.ascii_byte()).collect();
        let buf = cstr(&body);
        assert_same(&format!("row03 case {i} len {len}"), |api| unsafe {
            (api.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

// --------------------------------------------------------------------- row 4
#[test]
fn row04_print_line_random_full_byte_range() {
    // Includes high bytes / invalid UTF-8 — the Rust side must never try to
    // interpret the buffer as UTF-8.
    let mut rng = Rng::new(0x5EED_0004);
    for i in 0..256 {
        let len = 1 + rng.below(64) as usize;
        let body: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let buf = cstr(&body);
        assert_same(&format!("row04 case {i} len {len}"), |api| unsafe {
            (api.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

// --------------------------------------------------------------------- row 5
#[test]
fn row05_print_line_oversized_buffers() {
    let mut rng = Rng::new(0x5EED_0005);
    for &len in &[1024usize, 8 * 1024, 64 * 1024, 1024 * 1024] {
        let body: Vec<u8> = (0..len).map(|_| rng.ascii_byte()).collect();
        let buf = cstr(&body);
        assert_same(&format!("row05 len {len}"), |api| unsafe {
            (api.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

// --------------------------------------------------------------------- row 6
#[test]
fn row06_print_line_format_directive_payloads() {
    // `printLine` passes `line` as printf's *argument*, never as its format,
    // so these must all be echoed verbatim.
    let cases: &[&[u8]] = &[
        b"%s",
        b"%d",
        b"%n",
        b"%%",
        b"%99999d",
        b"%s%s%s%s%s%s%s%s",
        b"%p %x %lf %hhn",
        b"100%% sure",
    ];
    for (i, c) in cases.iter().enumerate() {
        let buf = cstr(c);
        assert_same(&format!("row06 case {i} {:?}", String::from_utf8_lossy(c)), |api| unsafe {
            (api.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

// --------------------------------------------------------------------- row 7
#[test]
fn row07_print_line_embedded_whitespace_and_newlines() {
    let cases: &[&[u8]] = &[
        b"\n",
        b"\n\n\n",
        b"a\nb",
        b"line1\nline2\nline3",
        b"\t\ttabbed",
        b"cr\rlf\n",
        b"trailing newline\n",
        b" ",
        b"   spaced   ",
    ];
    for (i, c) in cases.iter().enumerate() {
        let buf = cstr(c);
        assert_same(&format!("row07 case {i}"), |api| unsafe {
            (api.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

// --------------------------------------------------------------------- row 8
#[test]
fn row08_print_line_interior_pointers() {
    let mut rng = Rng::new(0x5EED_0008);
    for i in 0..128 {
        let len = 8 + rng.below(120) as usize;
        let body: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let buf = cstr(&body);
        let k = rng.below(len as u64) as usize; // 0 .. len-1
        assert_same(&format!("row08 case {i} offset {k} of {len}"), |api| unsafe {
            (api.print_line)(buf.as_ptr().add(k) as *const c_char)
        });
    }
}

// --------------------------------------------------------------------- row 9
#[test]
fn row09_print_line_repeated_calls_stateless() {
    let mut rng = Rng::new(0x5EED_0009);
    let bufs: Vec<Vec<u8>> = (0..1000)
        .map(|_| {
            let len = 1 + rng.below(16) as usize;
            cstr(&(0..len).map(|_| rng.ascii_byte()).collect::<Vec<u8>>())
        })
        .collect();
    // One captured stream per library, 1000 calls each: proves the ordering and
    // buffering of the whole sequence is identical, not just each call alone.
    assert_same("row09 1000 sequential printLine calls", |api| unsafe {
        for b in &bufs {
            (api.print_line)(b.as_ptr() as *const c_char);
        }
    });
}

// -------------------------------------------------------------------- row 10
#[test]
fn row10_good_single_call() {
    assert_same("row10 good()", |api| unsafe { (api.good)() });
}

// -------------------------------------------------------------------- row 11
#[test]
fn row11_good_repeated_calls() {
    assert_same("row11 good() x100", |api| unsafe {
        for _ in 0..100 {
            (api.good)();
        }
    });
}

// -------------------------------------------------------------------- row 12
#[test]
fn row12_driver_use_good_one() {
    assert_same("row12 driver(1)", |api| unsafe { (api.driver)(1) });
}

// -------------------------------------------------------------------- row 13
#[test]
fn row13_driver_random_nonzero() {
    let mut rng = Rng::new(0x5EED_0013);
    let mut vals: Vec<i32> = Vec::new();
    while vals.len() < 512 {
        let v = rng.next_u32() as i32;
        if v != 0 {
            vals.push(v);
        }
    }
    for (i, &v) in vals.iter().enumerate() {
        assert_same(&format!("row13 case {i} driver({v})"), |api| unsafe {
            (api.driver)(v)
        });
    }
}

// -------------------------------------------------------------------- row 14
#[test]
fn row14_driver_boundary_truthy_values() {
    let vals: &[i32] = &[
        1,
        -1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1 << 8,
        1 << 16,
        1 << 30,
        0x0000_0100,
        0x7fff_ffff,
        -0x8000_0000i64 as i32,
        2,
        -2,
        255,
        256,
        65_535,
        65_536,
    ];
    for &v in vals {
        assert_same(&format!("row14 driver({v})"), |api| unsafe { (api.driver)(v) });
    }
}

// -------------------------------------------------------------------- row 15
// driver(0) -> bad() reads an UNINITIALISED `char *` (CWE-457). See ERRORS.md
// row 11. There is no defined C behaviour to match byte-for-byte here: the
// garbage stack slot is caller-dependent, and on this toolchain the C library
// SIGSEGVs for some callers and prints stale data for others (measured: it
// crashes for the `driver(0)` call shape and prints for the direct `bad()`
// shape). The calls are therefore run crash-isolated in a forked child and the
// assertion is the part that IS well defined: the RUST library must never
// crash and must never emit a partial line.
#[test]
fn row15_driver_zero_takes_bad_path() {
    let l = libs();
    let mut c_crashes = 0;
    for _ in 0..32 {
        let c = run_in_child(|| unsafe { (l.c.driver)(0) });
        if c.crashed() {
            c_crashes += 1;
        } else {
            assert!(c.clean_line(), "C driver(0): partial line {}", show(&c.out));
        }
        let r = run_in_child(|| unsafe { (l.rust.driver)(0) });
        assert!(
            r.clean_line(),
            "Rust driver(0) must return normally with a complete line, got {r:?}"
        );
    }
    eprintln!("row15: C driver(0) crashed in {c_crashes}/32 runs (UB, expected)");
}

// -------------------------------------------------------------------- row 16
#[test]
fn row16_bad_direct_call() {
    let l = libs();
    let mut c_crashes = 0;
    for _ in 0..32 {
        let c = run_in_child(|| unsafe { (l.c.bad)() });
        if c.crashed() {
            c_crashes += 1;
        } else {
            assert!(c.clean_line(), "C bad(): partial line {}", show(&c.out));
        }
        let r = run_in_child(|| unsafe { (l.rust.bad)() });
        assert!(
            r.clean_line(),
            "Rust bad() must return normally with a complete line, got {r:?}"
        );
    }
    eprintln!("row16: C bad() crashed in {c_crashes}/32 runs (UB, expected)");
}

// -------------------------------------------------------------------- row 17
#[test]
fn row17_mixed_pipeline_single_stream() {
    // Full end-to-end pipeline, everything except the UB `bad` path, compared
    // as ONE stdout stream.
    let x = cstr(b"x");
    let long = cstr(&vec![b'z'; 300]);
    assert_same("row17 mixed pipeline", |api| unsafe {
        (api.driver)(1);
        (api.print_line)(x.as_ptr() as *const c_char);
        (api.good)();
        (api.print_line)(std::ptr::null());
        (api.driver)(7);
        (api.print_line)(long.as_ptr() as *const c_char);
        (api.driver)(-1);
        (api.print_line)(std::ptr::null());
        (api.good)();
    });
}

// -------------------------------------------------------------------- row 18
#[test]
fn row18_random_interleaving_of_all_entry_points() {
    // 200 random ops per stream. `bad`/`driver(0)` are excluded from the
    // byte-compared stream (UB payload) but covered by rows 15/16.
    let mut rng = Rng::new(0x5EED_0018);
    #[derive(Clone)]
    enum Op {
        PrintLine(Vec<u8>),
        PrintNull,
        Good,
        Driver(i32),
    }
    let ops: Vec<Op> = (0..200)
        .map(|_| match rng.below(4) {
            0 => {
                let len = rng.below(24) as usize;
                Op::PrintLine(cstr(&(0..len).map(|_| rng.nonzero_byte()).collect::<Vec<u8>>()))
            }
            1 => Op::PrintNull,
            2 => Op::Good,
            _ => {
                let mut v = rng.next_u32() as i32;
                if v == 0 {
                    v = 1;
                }
                Op::Driver(v)
            }
        })
        .collect();

    assert_same("row18 random interleaving", |api| unsafe {
        for op in &ops {
            match op {
                Op::PrintLine(b) => (api.print_line)(b.as_ptr() as *const c_char),
                Op::PrintNull => (api.print_line)(std::ptr::null()),
                Op::Good => (api.good)(),
                Op::Driver(v) => (api.driver)(*v),
            }
        }
    });
}
