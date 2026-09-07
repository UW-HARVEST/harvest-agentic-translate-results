//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads BOTH `c_src/build/libdriver.so` and the Rust cdylib via
//! `libloading` and compares return value + stdout + stderr byte-for-byte.

mod common;

use common::*;
use std::ffi::c_int;

// --- Row 1 ----------------------------------------------------------------
#[test]
fn row01_forward_random_negative() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..256 {
        let x = rng.range_i32(i32::MIN, -1);
        diff_forward(x);
    }
}

// --- Row 2 ----------------------------------------------------------------
#[test]
fn row02_forward_random_positive_no_overflow() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..256 {
        let x = rng.range_i32(1, (1 << 30) - 1);
        diff_forward(x);
    }
}

// --- Row 3 ----------------------------------------------------------------
#[test]
fn row03_forward_zero() {
    diff_forward(0);
}

// --- Row 4 ----------------------------------------------------------------
#[test]
fn row04_forward_random_overflowing() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..256 {
        let x = rng.range_i32(1 << 30, i32::MAX);
        diff_forward(x);
    }
}

// --- Row 5 ----------------------------------------------------------------
#[test]
fn row05_forward_boundaries() {
    for x in [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        (1 << 30) - 1,
        1 << 30,
        (1 << 30) + 1,
        i32::MAX - 1,
        i32::MAX,
    ] {
        diff_forward(x);
    }
}

// --- Row 6 ----------------------------------------------------------------
#[test]
fn row06_forward_fully_random() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..2048 {
        diff_forward(rng.next_u32() as i32);
    }
}

// --- Row 7 ----------------------------------------------------------------
#[test]
fn row07_open_missing_files() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..64 {
        let mut name = b"/tmp/difftest-absent-".to_vec();
        for _ in 0..16 {
            name.push(b'a' + (rng.below(26) as u8));
        }
        assert!(!std::path::Path::new(&String::from_utf8(name.clone()).unwrap()).exists());
        diff_open(Some(&name));
    }
}

// --- Row 8 ----------------------------------------------------------------
#[test]
fn row08_open_empty_file() {
    diff_open_file("empty", b"");
}

// --- Row 9 ----------------------------------------------------------------
#[test]
fn row09_open_single_line_with_newline() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..64 {
        let content = gen_lines(&mut rng, 1, 30, true);
        diff_open_file("one-nl", &content);
    }
}

// --- Row 10 ---------------------------------------------------------------
#[test]
fn row10_open_single_line_no_newline() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..64 {
        let mut content = gen_lines(&mut rng, 1, 30, false);
        if content.is_empty() {
            content.push(b'x');
        }
        diff_open_file("one-nonl", &content);
    }
}

// --- Row 11 ---------------------------------------------------------------
#[test]
fn row11_open_many_lines() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..64 {
        let n = 2 + rng.below(39) as usize;
        let content = gen_lines(&mut rng, n, 30, true);
        diff_open_file("many-nl", &content);
    }
}

// --- Row 12 ---------------------------------------------------------------
#[test]
fn row12_open_many_lines_no_trailing_newline() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..64 {
        let n = 2 + rng.below(39) as usize;
        let content = gen_lines(&mut rng, n, 30, false);
        diff_open_file("many-nonl", &content);
    }
}

// --- Row 13 ---------------------------------------------------------------
#[test]
fn row13_open_buffer_boundary_line_lengths() {
    let mut rng = Rng::new(SEED ^ 13);
    for len in [0usize, 1, 97, 98, 99, 100, 101, 102, 197, 198, 199, 200, 201] {
        for trailing in [true, false] {
            let mut content: Vec<u8> = (0..len).map(|_| rng.printable()).collect();
            if trailing {
                content.push(b'\n');
            }
            diff_open_file("bound", &content);
            // and the same line repeated twice, to exercise chunk carry-over
            let mut twice = content.clone();
            twice.extend_from_slice(&content);
            diff_open_file("bound2", &twice);
        }
    }
}

// --- Row 14 ---------------------------------------------------------------
#[test]
fn row14_open_very_long_single_line() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..32 {
        let len = 500 + rng.below(4501) as usize;
        let content: Vec<u8> = (0..len).map(|_| rng.printable()).collect();
        diff_open_file("longline", &content);
    }
}

// --- Row 15 ---------------------------------------------------------------
#[test]
fn row15_open_embedded_nul_bytes() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..64 {
        let n = 1 + rng.below(20) as usize;
        let mut content = Vec::new();
        for _ in 0..n {
            let len = rng.below(40) as usize;
            for _ in 0..len {
                // ~20% NUL bytes so `printf("%s")` truncation is hit often
                content.push(if rng.below(5) == 0 { 0 } else { rng.printable() });
            }
            content.push(b'\n');
        }
        diff_open_file("nuls", &content);
    }
}

// --- Row 16 ---------------------------------------------------------------
#[test]
fn row16_open_crlf_and_lone_cr() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..64 {
        let n = 1 + rng.below(20) as usize;
        let mut content = Vec::new();
        for _ in 0..n {
            let len = rng.below(30) as usize;
            for _ in 0..len {
                content.push(rng.printable());
            }
            match rng.below(3) {
                0 => content.extend_from_slice(b"\r\n"),
                1 => content.push(b'\r'),
                _ => content.push(b'\n'),
            }
        }
        diff_open_file("crlf", &content);
    }
}

// --- Row 17 ---------------------------------------------------------------
#[test]
fn row17_open_arbitrary_binary() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..64 {
        let len = rng.below(4097) as usize;
        let content = gen_binary(&mut rng, len);
        diff_open_file("binary", &content);
    }
}

// --- Row 18 ---------------------------------------------------------------
#[test]
fn row18_open_larger_than_stdio_buffer() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..16 {
        let target = 8 * 1024 + rng.below(56 * 1024) as usize;
        let mut content = Vec::with_capacity(target + 64);
        while content.len() < target {
            let len = rng.below(120) as usize;
            for _ in 0..len {
                content.push(rng.printable());
            }
            content.push(b'\n');
        }
        diff_open_file("big", &content);
    }
}

// --- Row 19 ---------------------------------------------------------------
#[test]
fn row19_open_only_newlines() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..64 {
        let n = 1 + rng.below(200) as usize;
        let content = vec![b'\n'; n];
        diff_open_file("newlines", &content);
    }
}

// --- Row 20 ---------------------------------------------------------------
#[test]
fn row20_open_success_returns_open_eof_handle() {
    // `diff_open` already asserts non-NULL-ness parity plus feof/ferror parity
    // on the returned handle; exercise it across several successful shapes.
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..32 {
        diff_open_file("state", &gen_lines(&mut rng, 5, 50, true));
    }
    diff_open_file("state-empty", b"");
    diff_open_file("state-nonl", b"abc");
}

// --- Row 21 ---------------------------------------------------------------
#[test]
fn row21_driver_negative_num_short_circuits() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..32 {
        let num = rng.range_i32(i32::MIN, -1);
        // valid file
        let f = TempFile::with_bytes("d-neg", &gen_lines(&mut rng, 3, 20, true));
        diff_driver(num, Some(&f.name_bytes()));
        // missing file
        diff_driver(num, Some(b"/tmp/difftest-absent-zzzzzzzz"));
    }
}

// --- Row 22 ---------------------------------------------------------------
#[test]
fn row22_driver_ok_num_missing_file() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..64 {
        let num = rng.range_i32(0, (1 << 30) - 1);
        let mut name = b"/tmp/difftest-absent-".to_vec();
        for _ in 0..16 {
            name.push(b'a' + (rng.below(26) as u8));
        }
        diff_driver(num, Some(&name));
    }
}

// --- Row 23 ---------------------------------------------------------------
#[test]
fn row23_driver_ok_num_valid_file_all_shapes() {
    let mut rng = Rng::new(SEED ^ 23);
    let shapes: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"one line\n".to_vec(),
        b"no newline".to_vec(),
        gen_lines(&mut rng, 10, 30, true),
        gen_lines(&mut rng, 10, 30, false),
        (0..99).map(|_| b'a').collect(),
        (0..100).map(|_| b'b').collect(),
        (0..101).map(|_| b'c').collect(),
        {
            let mut v = (0..99).map(|_| b'd').collect::<Vec<u8>>();
            v.push(b'\n');
            v
        },
        b"with\0nul\nbytes\0here\n".to_vec(),
        b"crlf\r\nlines\r\n".to_vec(),
        gen_binary(&mut rng, 1000),
        vec![b'\n'; 50],
        gen_lines(&mut rng, 400, 100, true),
    ];
    for (i, s) in shapes.iter().enumerate() {
        for num in [0i32, 1, 7, 12345, (1 << 30) - 1] {
            diff_driver_file(&format!("d-shape{i}"), num, s);
        }
    }
}

// --- Row 24 ---------------------------------------------------------------
#[test]
fn row24_driver_random_cross_product() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..512 {
        let num = rng.next_u32() as i32;
        let kind = rng.below(6);
        let n_lines = 1 + rng.below(8) as usize;
        let bin_len = rng.below(500) as usize;
        let content: Vec<u8> = match kind {
            0 => Vec::new(),
            1 => gen_lines(&mut rng, n_lines, 40, true),
            2 => gen_lines(&mut rng, n_lines, 40, false),
            3 => gen_binary(&mut rng, bin_len),
            4 => (0..rng.below(300)).map(|_| rng.printable()).collect(),
            _ => {
                let mut v = Vec::new();
                for _ in 0..rng.below(10) {
                    v.push(0);
                    v.push(rng.printable());
                    v.push(b'\n');
                }
                v
            }
        };
        // half the iterations point at a missing file instead
        if rng.below(2) == 0 {
            diff_driver_file("d-rand", num, &content);
        } else {
            diff_driver(num, Some(b"/tmp/difftest-absent-qqqqqqqqqqqq"));
        }
    }
}

// --- Row 25 ---------------------------------------------------------------
#[test]
fn row25_driver_overflowing_num_valid_file() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..64 {
        let num = rng.range_i32(1 << 30, i32::MAX);
        let content = gen_lines(&mut rng, 3, 25, true);
        diff_driver_file("d-ovf", num, &content);
    }
    for num in [1 << 30, (1 << 30) + 1, i32::MAX - 1, i32::MAX] {
        diff_driver_file("d-ovf-b", num, b"hello\n");
    }
}

// --- Row 26 ---------------------------------------------------------------
#[test]
fn row26_mixed_sequence_statelessness() {
    let mut rng = Rng::new(SEED ^ 26);
    let f = TempFile::with_bytes("mixed", b"line one\nline two\n");
    let name = f.name_bytes();
    for _ in 0..256 {
        match rng.below(3) {
            0 => diff_forward(rng.next_u32() as i32),
            1 => diff_open(Some(&name)),
            _ => diff_driver(rng.next_u32() as i32, Some(&name)),
        }
    }
}

// --- Row 27 (feature parity) ---------------------------------------------
// The crate has no `[features]`, so the default build and
// `--no-default-features` are the same configuration; both are executed by
// `run_all.sh`. This test simply records that the loaded Rust `.so` exports the
// full C symbol set in whichever configuration it was built.
#[test]
fn row27_symbol_parity_at_runtime() {
    // Loading succeeds only if all three symbols resolve.
    let c = c_lib();
    let r = rust_lib();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    let _: c_int = capture(|| unsafe { (r.forward_goto_example)(3) }).ret;
}
