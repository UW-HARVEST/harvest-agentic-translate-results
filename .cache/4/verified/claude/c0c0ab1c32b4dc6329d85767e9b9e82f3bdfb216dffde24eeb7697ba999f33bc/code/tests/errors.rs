// Phase C -- one differential test per row of ERRORS.md.
//
// Every test constructs the exact invalid input/condition, runs it through both
// implementations (C program / C `.so` and Rust program / Rust `.so`) and
// asserts they reject it identically -- same bytes on stdout, same exit code,
// and for the faulting rows the same termination signal.

mod common;

use common::{assert_program_matches, run_c_exe, run_c_so_main, run_rust_exe, run_rust_so_main, Rng};
use std::os::raw::c_int;

/// Shared body for the program-level rows: C and Rust must agree, and the C
/// behaviour asserted in ERRORS.md ("prints `expect_lines`, exit status 0")
/// must actually hold.
fn check_program(label: &str, stdin: &[u8], expect_lines: usize) {
    assert_program_matches(label, stdin);

    let c = run_c_exe(stdin);
    assert_eq!(c.status, Ok(0), "[{label}] C exit status must be 0");
    let lines = if c.stdout.is_empty() {
        0
    } else {
        c.stdout.iter().filter(|&&b| b == b'\n').count()
    };
    assert_eq!(
        lines, expect_lines,
        "[{label}] C printed {lines} lines, ERRORS.md says {expect_lines}: {:?}",
        String::from_utf8_lossy(&c.stdout)
    );

    // ... and the Rust program / both `main` exports agree on all of it.
    let r = run_rust_exe(stdin);
    assert_eq!(r.status, Ok(0), "[{label}] Rust exit status must be 0");
    assert_eq!(run_c_so_main(stdin).status, Ok(0));
    assert_eq!(run_rust_so_main(stdin).status, Ok(0));
}

// -------------------------------------------------- rows 1-12: scanf ----

#[test]
fn err01_empty_stdin() {
    check_program("err01", b"", 0);
}

#[test]
fn err02_whitespace_only_stdin() {
    for input in [
        &b" "[..],
        &b"\n"[..],
        &b"\t"[..],
        &b"\x0b"[..],
        &b"\x0c"[..],
        &b"\r"[..],
        &b" \t\n\x0b\x0c\r"[..],
        &b"\n\n\n\n\n"[..],
        &b"                                                              "[..],
    ] {
        check_program("err02", input, 0);
    }
}

#[test]
fn err03_leading_non_numeric() {
    for input in [
        &b"abc"[..],
        &b"abc\n"[..],
        &b"zz 1 2\n"[..],
        &b"  \t hello 5\n"[..],
        &b"."[..],
        &b"/1\n"[..],
        &b":5\n"[..],
        &b"x10\n"[..],
        &b"e\n"[..],
        &b"*"[..],
    ] {
        check_program("err03", input, 0);
    }
}

#[test]
fn err04_sign_then_non_digit() {
    for input in [
        &b"-x"[..],
        &b"-x\n"[..],
        &b"+ 5\n"[..],
        &b"- 5\n"[..],
        &b"--5\n"[..],
        &b"++5\n"[..],
        &b"+-5\n"[..],
        &b"+.\n"[..],
        &b"-.5\n"[..],
        &b"-\n"[..],
        &b"+\n"[..],
        &b"-\t"[..],
    ] {
        check_program("err04", input, 0);
    }
}

#[test]
fn err05_sign_then_eof() {
    for input in [&b"-"[..], &b"+"[..], &b"  -"[..], &b"\n\t+"[..]] {
        check_program("err05", input, 0);
    }
}

#[test]
fn err06_invalid_token_after_k_valid() {
    let mut rng = Rng::new(0xE006);
    for k in 0..12usize {
        let tokens: Vec<String> = (0..k).map(|_| format!("{}", rng.range_incl(-1000, 1000))).collect();
        let mut s = tokens.join(" ");
        if k > 0 {
            s.push(' ');
        }
        s.push_str("zz 7 8\n");
        check_program(&format!("err06(k={k})"), s.as_bytes(), k);
    }
}

#[test]
fn err07_partial_token() {
    // The digit prefix converts (1 item), the offending byte kills the next
    // conversion -> exactly one line of output.
    for (input, lines) in [
        (&b"5x\n"[..], 1),
        (&b"1.5\n"[..], 1),
        (&b"0x10\n"[..], 1),
        (&b"1e3\n"[..], 1),
        (&b"12abc\n"[..], 1),
        (&b"7,8\n"[..], 1),
        (&b"3 4x 5\n"[..], 2),
        (&b"9-\n"[..], 1),
        (&b"9+\n"[..], 1),
    ] {
        check_program("err07", input, lines);
    }
}

#[test]
fn err08_more_than_capacity() {
    let mut rng = Rng::new(0xE008);
    for k in [101usize, 102, 128, 150, 200] {
        let tokens: Vec<String> = (0..k).map(|_| format!("{}", rng.next_i32())).collect();
        let mut s = tokens.join(" ");
        s.push('\n');
        check_program(&format!("err08(k={k})"), s.as_bytes(), 100);
    }
}

#[test]
fn err09_int_range_truncation() {
    for input in [
        "2147483648",
        "2147483649",
        "-2147483649",
        "-2147483650",
        "4294967296",
        "4294967297",
        "3000000000",
        "-3000000000",
        "9223372036854775807",
        "-9223372036854775808",
    ] {
        check_program("err09", format!("{input}\n").as_bytes(), 1);
    }
}

#[test]
fn err10_above_long_max() {
    for input in [
        "9223372036854775808",
        "9223372036854775809",
        "99999999999999999999",
        "9999999999999999999999999999999999999999",
        "+9223372036854775808",
        "00009223372036854775808",
    ] {
        check_program("err10", format!("{input}\n").as_bytes(), 1);
    }
    // (int)LONG_MAX == -1, so driver prints (-1)*(-1)+(-1) == 0
    let c = run_c_exe(b"9223372036854775808\n");
    assert_eq!(c.stdout, b"0\n", "unexpected C result for LONG_MAX clamp");
}

#[test]
fn err11_below_long_min() {
    for input in [
        "-9223372036854775809",
        "-9223372036854775810",
        "-99999999999999999999",
        "-9999999999999999999999999999999999999999",
        "-00009223372036854775809",
    ] {
        check_program("err11", format!("{input}\n").as_bytes(), 1);
    }
    // (int)LONG_MIN == 0, so driver prints 0*0+0 == 0
    let c = run_c_exe(b"-9223372036854775809\n");
    assert_eq!(c.stdout, b"0\n", "unexpected C result for LONG_MIN clamp");
}

#[test]
fn err12_nul_and_high_bytes() {
    for input in [
        &b"\0"[..],
        &b"\0\0\0"[..],
        &b"\x805\n"[..],
        &b"\xff\n"[..],
        &b"\x7f"[..],
        &b"1 \0 2\n"[..],
        &b"1\0"[..],
        &b"\x01\x02\x03"[..],
        &b"\xc3\xa9\n"[..],
    ] {
        let expect = if input.starts_with(b"1") { 1 } else { 0 };
        check_program("err12", input, expect);
    }
}

// ------------------------------------------------ rows 13-16: fma_array ----

#[test]
fn err13_fma_len_zero() {
    let l = common::libs();
    let base: Vec<i32> = (0..16).map(|x| x * 7 - 3).collect();
    let mut cbuf = base.clone();
    let mut rbuf = base.clone();
    unsafe {
        let p = cbuf.as_mut_ptr();
        (l.c_fma())(p, p.add(4), p.add(8), p.add(12), 0);
        let p = rbuf.as_mut_ptr();
        (l.rust_fma())(p, p.add(4), p.add(8), p.add(12), 0);
    }
    assert_eq!(cbuf, base, "C fma_array must not touch memory with len==0");
    assert_eq!(rbuf, base, "Rust fma_array must not touch memory with len==0");
}

#[test]
fn err14_fma_len_negative() {
    let l = common::libs();
    let base: Vec<i32> = (0..16).map(|x| x * 13 + 1).collect();
    for len in [-1i32, -2, -100, -1000, c_int::MIN + 1, c_int::MIN] {
        let mut cbuf = base.clone();
        let mut rbuf = base.clone();
        unsafe {
            let p = cbuf.as_mut_ptr();
            (l.c_fma())(p, p.add(4), p.add(8), p.add(12), len);
            let p = rbuf.as_mut_ptr();
            (l.rust_fma())(p, p.add(4), p.add(8), p.add(12), len);
        }
        assert_eq!(cbuf, base, "C fma_array touched memory with len={len}");
        assert_eq!(rbuf, base, "Rust fma_array touched memory with len={len}");
        assert_eq!(cbuf, rbuf);
    }
}

#[test]
fn err15_fma_null_ptrs_len_le_zero() {
    let l = common::libs();
    for len in [0i32, -1, -1000, c_int::MIN] {
        unsafe {
            (l.c_fma())(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                len,
            );
            (l.rust_fma())(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                len,
            );
        }
    }
    // Reaching here at all is the assertion: neither implementation
    // dereferences the NULLs when the loop body cannot run.
}

#[test]
fn err16_fma_null_ptrs_len_positive_segv() {
    for len in ["1", "2", "1000"] {
        let c = common::run_so(&common::c_so(), &["fma_null", len], b"");
        let r = common::run_so(&common::rust_so(), &["fma_null", len], b"");
        assert_eq!(
            c.status,
            Err(libc::SIGSEGV),
            "C fma_array(NULL.., {len}) should fault: {}",
            c.describe()
        );
        assert_eq!(
            c.status, r.status,
            "termination differs for fma_array(NULL.., {len}): C {} vs Rust {}",
            c.describe(),
            r.describe()
        );
        assert_eq!(c.stdout, r.stdout);
    }
}

// -------------------------------------------------- rows 17-20: driver ----

/// Formats a buffer the way `so_runner` dumps it on stderr.
fn dumped(values: &[i32]) -> String {
    values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn err17_driver_len_zero() {
    let base: Vec<i32> = vec![5, -7, i32::MAX, i32::MIN];
    common::assert_driver_matches_out_of_process("err17", "driver", &base, 0);

    let c = common::run_driver_so(&common::c_so(), "driver", &base, 0);
    assert!(
        c.stdout.is_empty(),
        "C driver printed {:?} with len==0",
        String::from_utf8_lossy(&c.stdout)
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        dumped(&base),
        "C driver mutated the buffer with len==0"
    );
}

#[test]
fn err18_driver_len_negative() {
    let base: Vec<i32> = vec![5, -7, i32::MAX, i32::MIN];
    for len in [-1i32, -2, -1000, c_int::MIN + 1, c_int::MIN] {
        common::assert_driver_matches_out_of_process("err18", "driver", &base, len);

        let c = common::run_driver_so(&common::c_so(), "driver", &base, len);
        assert!(
            c.stdout.is_empty(),
            "C driver printed something with len={len}"
        );
        assert_eq!(
            String::from_utf8_lossy(&c.stderr),
            dumped(&base),
            "C driver mutated the buffer with len={len}"
        );
    }
}

#[test]
fn err19_driver_null_len_le_zero() {
    for len in [0i32, -1, -1000, c_int::MIN] {
        let ls = len.to_string();
        let c = common::run_so(&common::c_so(), &["driver_null", &ls], b"");
        let r = common::run_so(&common::rust_so(), &["driver_null", &ls], b"");
        assert_eq!(
            c.status,
            Ok(0),
            "C driver(NULL, {len}) must not fault: {}",
            c.describe()
        );
        assert!(
            c.stdout.is_empty(),
            "C driver(NULL, {len}) printed {:?}",
            String::from_utf8_lossy(&c.stdout)
        );
        assert_eq!(c.status, r.status, "driver(NULL, {len}) status differs");
        assert_eq!(c.stdout, r.stdout, "driver(NULL, {len}) stdout differs");
    }
}

#[test]
fn err20_driver_null_len_positive_segv() {
    for len in ["1", "2", "1000"] {
        let c = common::run_so(&common::c_so(), &["driver_null", len], b"");
        let r = common::run_so(&common::rust_so(), &["driver_null", len], b"");
        assert_eq!(
            c.status,
            Err(libc::SIGSEGV),
            "C driver(NULL, {len}) should fault: {}",
            c.describe()
        );
        assert_eq!(
            c.status, r.status,
            "termination differs for driver(NULL, {len}): C {} vs Rust {}",
            c.describe(),
            r.describe()
        );
        assert_eq!(c.stdout, r.stdout);
    }
}

// -------------------------------------------------- rows 21-23: misc ----

#[test]
fn err21_oversized_len_reads_past_logical_end() {
    // `len` larger than the caller's logical element count: the C code has no
    // bounds check, so the surplus elements are transformed too.  The backing
    // allocation is big enough that the memory is still valid, which makes the
    // comparison well-defined.
    let l = common::libs();
    let mut rng = Rng::new(0xE021);
    for _ in 0..50 {
        let logical = rng.range_incl(1, 32) as usize;
        let slack = rng.range_incl(1, 32) as usize;
        let total = logical + slack;
        let base: Vec<i32> = (0..total).map(|_| rng.next_i32()).collect();
        let len = total as c_int; // one .. slack elements past `logical`

        let mut cbuf = base.clone();
        let mut rbuf = base.clone();
        unsafe {
            let p = cbuf.as_mut_ptr();
            (l.c_fma())(p, p, p, p, len);
            let p = rbuf.as_mut_ptr();
            (l.rust_fma())(p, p, p, p, len);
        }
        assert_eq!(cbuf, rbuf, "oversized-len fma_array differs (logical={logical}, len={len})");

        // ... and through `driver`, whose print loop walks the same surplus.
        common::assert_driver_matches_out_of_process("err21", "driver", &base, len);
    }
}

#[test]
fn err22_fma_huge_len_segv() {
    // len == INT_MAX on a 4 element buffer: `int i` walks off the end long
    // before it could overflow, so both must die with SIGSEGV (and Rust must
    // not trap on the `i + 1` increment first).
    let c = common::run_so(&common::c_so(), &["fma_huge_len"], b"");
    let r = common::run_so(&common::rust_so(), &["fma_huge_len"], b"");
    assert_eq!(
        c.status,
        Err(libc::SIGSEGV),
        "C fma_array(len=INT_MAX) should fault: {}",
        c.describe()
    );
    assert_eq!(
        c.status,
        r.status,
        "termination differs for fma_array(len=INT_MAX): C {} vs Rust {}",
        c.describe(),
        r.describe()
    );
}

#[test]
fn err23_exit_status_is_always_zero() {
    // Rows 1-12 all return 0; sample each rejection class once more here so the
    // row has its own named test.
    for input in [
        &b""[..],
        &b"   "[..],
        &b"abc"[..],
        &b"-"[..],
        &b"+q"[..],
        &b"1 2 zz"[..],
        &b"5x"[..],
        &b"\0"[..],
        &b"99999999999999999999999"[..],
    ] {
        assert_eq!(run_c_exe(input).status, Ok(0));
        assert_eq!(run_rust_exe(input).status, Ok(0));
        assert_eq!(run_c_so_main(input).status, Ok(0));
        assert_eq!(run_rust_so_main(input).status, Ok(0));
    }
}
