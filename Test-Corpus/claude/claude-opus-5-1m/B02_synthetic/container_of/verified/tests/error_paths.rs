//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! The C code contains no explicit error handling at all, so its "error surface"
//! consists of (a) the faults it lets happen on unchecked dereferences and
//! (b) the silent value mangling its unchecked library calls and arithmetic
//! perform. Both kinds are compared here.
//!
//! Rows whose trigger kills the process are executed in a `fork()`ed child so
//! that the *exact* terminating signal can be compared, rather than merely
//! observing that "both failed somehow".

mod common;

use std::ffi::{c_char, c_int, c_void, CString};

use common::{run_in_child, Argv, Outcome, Pair, Rng, Sink};

const SEED: u64 = 0x0BAD_0BAD_0BAD_0BAD;

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Calls `main` in a forked child and reports how the child died plus whatever
/// it printed. `argv` is built by the caller so that NULL / wild entries can be
/// placed anywhere.
fn child_main(imp: &common::Impl, argc: c_int, argv: *mut *mut c_char) -> (Outcome, Vec<u8>) {
    let entry = imp.main();
    run_in_child(move || {
        unsafe { entry(argc, argv) };
    })
}

/// Asserts that both implementations fail identically for the same `argv`.
fn assert_same_fault(pair: &Pair, argc: c_int, build: impl Fn() -> Vec<*mut c_char>, label: &str) {
    let mut c_argv = build();
    let mut r_argv = build();

    let (c_outcome, c_out) = child_main(&pair.c, argc, c_argv.as_mut_ptr());
    let (r_outcome, r_out) = child_main(&pair.rust, argc, r_argv.as_mut_ptr());

    assert_eq!(
        c_outcome, r_outcome,
        "{label}: C terminated as {c_outcome:?}, Rust as {r_outcome:?}"
    );
    assert_eq!(
        c_outcome,
        Outcome::Signaled(libc::SIGSEGV),
        "{label}: expected the C reference to die from SIGSEGV"
    );
    assert_eq!(
        c_out, r_out,
        "{label}: output before the fault differs (C={c_out:?}, Rust={r_out:?})"
    );
    assert!(
        c_out.is_empty(),
        "{label}: the C reference is expected to print nothing before faulting"
    );
}

/// Runs both `main`s on the same arguments and returns the (identical) output,
/// failing if they differ. Also returns the return value.
fn agreeing_output(pair: &Pair, args: &[&[u8]]) -> (c_int, Vec<u8>) {
    let mut c_argv = Argv::new(args);
    let mut r_argv = Argv::new(args);
    let argc = c_argv.argc();

    let (c_rc, c_out) = common::run_main(&pair.c, argc, &mut c_argv, Sink::Pipe);
    let (r_rc, r_out) = common::run_main(&pair.rust, argc, &mut r_argv, Sink::Pipe);

    assert_eq!(c_rc, r_rc, "return value mismatch for {:?}", common::pretty(args));
    assert_eq!(
        c_out,
        r_out,
        "output mismatch for {:?}: C={:?} Rust={:?}",
        common::pretty(args),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    (c_rc, c_out)
}

/// Convenience: `main("driver", a, b)` must print exactly `expected`.
fn assert_prints(pair: &Pair, a: &[u8], b: &[u8], expected: &str) {
    let (rc, out) = agreeing_output(pair, &[b"driver", a, b]);
    assert_eq!(rc, 0, "the C main always returns 0");
    assert_eq!(
        String::from_utf8_lossy(&out),
        expected,
        "unexpected value for a={:?} b={:?}",
        String::from_utf8_lossy(a),
        String::from_utf8_lossy(b)
    );
}

// ---------------------------------------------------------------------------
// Row 1 — argc < 2, argv[1] == NULL
// ---------------------------------------------------------------------------

#[test]
fn row01_main_argc0_argv1_null_segv() {
    let pair = Pair::default_config();
    let prog = CString::new("driver").unwrap();

    // argv = { "driver", NULL, NULL }  (the trailing slot mirrors envp)
    let build = || {
        vec![
            prog.as_ptr() as *mut c_char,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ]
    };

    // Every argc value a real runtime could pass with this argv, plus 0.
    for argc in [0, 1] {
        assert_same_fault(&pair, argc, build, &format!("argv[1] == NULL, argc={argc}"));
    }
}

// ---------------------------------------------------------------------------
// Row 2 — argc == 2, argv[2] == NULL (the fault happens after the first
// conversion succeeded and before anything is printed)
// ---------------------------------------------------------------------------

#[test]
fn row02_main_argc2_argv2_null_segv() {
    let pair = Pair::default_config();
    let prog = CString::new("driver").unwrap();
    let mut rng = Rng::new(SEED ^ 2);

    for _ in 0..8 {
        let first = CString::new(format!("{}", rng.next_i32())).unwrap();
        let build = || {
            vec![
                prog.as_ptr() as *mut c_char,
                first.as_ptr() as *mut c_char,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ]
        };
        assert_same_fault(&pair, 2, build, "argv[2] == NULL");
    }
}

// ---------------------------------------------------------------------------
// Row 3 — argv itself is NULL
// ---------------------------------------------------------------------------

#[test]
fn row03_main_argv_null_segv() {
    let pair = Pair::default_config();
    let c_entry = pair.c.main();
    let r_entry = pair.rust.main();

    for argc in [0, 1, 3] {
        let (c_outcome, c_out) = run_in_child(move || {
            unsafe { c_entry(argc, std::ptr::null_mut()) };
        });
        let (r_outcome, r_out) = run_in_child(move || {
            unsafe { r_entry(argc, std::ptr::null_mut()) };
        });
        assert_eq!(c_outcome, r_outcome, "argv == NULL, argc={argc}");
        assert_eq!(c_outcome, Outcome::Signaled(libc::SIGSEGV));
        assert_eq!(c_out, r_out);
        assert!(c_out.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Row 4 — argv[1] is a non-NULL wild pointer
// ---------------------------------------------------------------------------

#[test]
fn row04_main_argv1_wild_pointer_segv() {
    let pair = Pair::default_config();
    let prog = CString::new("driver").unwrap();
    let ok = CString::new("1").unwrap();

    // Addresses that are certainly not mapped in either child.
    for wild in [0x10usize, 0x1000, 0xDEAD_0000_BEEF_0000, usize::MAX & !0xF] {
        // argv[1] wild
        let build = || {
            vec![
                prog.as_ptr() as *mut c_char,
                wild as *mut c_char,
                ok.as_ptr() as *mut c_char,
                std::ptr::null_mut(),
            ]
        };
        assert_same_fault(&pair, 3, build, &format!("argv[1] = {wild:#x}"));

        // argv[2] wild — the fault must come after argv[1] converted fine
        let build2 = || {
            vec![
                prog.as_ptr() as *mut c_char,
                ok.as_ptr() as *mut c_char,
                wild as *mut c_char,
                std::ptr::null_mut(),
            ]
        };
        assert_same_fault(&pair, 3, build2, &format!("argv[2] = {wild:#x}"));
    }
}

// ---------------------------------------------------------------------------
// Row 5 — empty subject sequence converts silently to 0
// ---------------------------------------------------------------------------

#[test]
fn row05_atoi_no_digits() {
    let pair = Pair::default_config();
    for arg in [
        &b"abc"[..],
        b"",
        b"@",
        b"\x80",
        b"\xff\xfe",
        b"0x",   // base 10: the "0" converts, "x" stops the scan
        b"x0",
        b".5",
        b"/9",
        b"::",
    ] {
        // "0x" converts the leading zero, everything else converts nothing;
        // either way the result is 0 and no error is signalled.
        assert_prints(&pair, arg, b"0", "0\n");
        assert_prints(&pair, b"0", arg, "0\n");
        // ... and it composes with a real value instead of poisoning it.
        assert_prints(&pair, arg, b"5", "5\n");
    }
}

// ---------------------------------------------------------------------------
// Row 6 — whitespace-only input
// ---------------------------------------------------------------------------

#[test]
fn row06_atoi_whitespace_only() {
    let pair = Pair::default_config();
    const SPACES: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];

    for arg in [
        &b" "[..],
        b"  ",
        b"\t",
        b"\n",
        b"\x0b",
        b"\x0c",
        b"\r",
        b" \t\n\x0b\x0c\r",
    ] {
        assert_prints(&pair, arg, b"0", "0\n");
        assert_prints(&pair, arg, b"-3", "-3\n");
    }

    // A whitespace-only argument cannot distinguish *which* bytes count as
    // whitespace (anything unrecognised also ends the empty subject sequence at
    // 0), so each byte is additionally checked in front of real digits, where
    // getting the set wrong changes the value.
    for s in SPACES {
        let prefixed = [&[s][..], b"37"].concat();
        assert_prints(&pair, &prefixed, b"0", "37\n");
        let signed = [&[s][..], b"-37"].concat();
        assert_prints(&pair, &signed, b"0", "-37\n");
        let repeated = [&[s][..], &[s][..], &[s][..], b"5"].concat();
        assert_prints(&pair, &repeated, b"0", "5\n");
    }

    // Whitespace *after* the sign or inside the digits is not skipped.
    for arg in [&b"- 37"[..], b"+ 37", b"3 7", b"3\t7", b"3\n7"] {
        let (_, out) = agreeing_output(&pair, &[b"driver", arg, b"0"]);
        assert!(
            out == b"0\n" || out == b"3\n",
            "unexpected value for {:?}: {:?}",
            String::from_utf8_lossy(arg),
            String::from_utf8_lossy(&out)
        );
    }

    // Every non-whitespace control byte must NOT be skipped: with digits behind
    // it the conversion has to stop before them and yield 0. (0x00 is excluded:
    // a process argument can never contain an interior NUL.)
    for b in 1u8..=0x1f {
        if SPACES.contains(&b) {
            continue;
        }
        let arg = [&[b][..], b"41"].concat();
        assert_prints(&pair, &arg, b"0", "0\n");
    }
    for b in [0x7fu8, 0x80, 0xa0, 0xff] {
        let arg = [&[b][..], b"41"].concat();
        assert_prints(&pair, &arg, b"0", "0\n");
    }
}

// ---------------------------------------------------------------------------
// Row 7 — sign without digits
// ---------------------------------------------------------------------------

#[test]
fn row07_atoi_sign_without_digits() {
    let pair = Pair::default_config();
    for arg in [
        &b"+"[..], b"-", b"++5", b"--5", b"+-5", b"-+5", b"+ 5", b"- 5", b"\t-", b"  +",
    ] {
        assert_prints(&pair, arg, b"0", "0\n");
        assert_prints(&pair, arg, b"11", "11\n");
    }
}

// ---------------------------------------------------------------------------
// Row 8 — trailing garbage after the digits
// ---------------------------------------------------------------------------

#[test]
fn row08_atoi_trailing_garbage() {
    let pair = Pair::default_config();
    for (arg, expected) in [
        (&b"12abc"[..], 12),
        (b"3.9", 3),
        (b"7 8", 7),
        (b"5-", 5),
        (b"-5+", -5),
        (b"0009z", 9),
        (b"1e5", 1),
        (b"2\t2", 2),
        (b"-0x10", 0),
    ] {
        assert_prints(&pair, arg, b"0", &format!("{expected}\n"));
    }
}

// ---------------------------------------------------------------------------
// Row 9 — value above LONG_MAX saturates, then truncates to -1
// ---------------------------------------------------------------------------

#[test]
fn row09_atoi_above_long_max() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 9);

    for arg in [
        &b"9223372036854775808"[..],
        b"9223372036854775809",
        b"18446744073709551616",
        b"+9223372036854775808",
        b"00009223372036854775808",
    ] {
        // LONG_MAX == 0x7FFF_FFFF_FFFF_FFFF, whose low 32 bits are 0xFFFFFFFF.
        assert_prints(&pair, arg, b"0", "-1\n");
    }

    // Randomised: 25..80 digit numbers are always > LONG_MAX.
    for _ in 0..100 {
        let n = rng.below(56) + 25;
        let mut s = Vec::new();
        for i in 0..n {
            let d = if i == 0 {
                (rng.below(9) + 1) as u8
            } else {
                rng.below(10) as u8
            };
            s.push(b'0' + d);
        }
        assert_prints(&pair, &s, b"0", "-1\n");
    }
}

// ---------------------------------------------------------------------------
// Row 10 — value below LONG_MIN saturates, then truncates to 0
// ---------------------------------------------------------------------------

#[test]
fn row10_atoi_below_long_min() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 10);

    for arg in [
        &b"-9223372036854775809"[..],
        b"-18446744073709551616",
        b"-00009223372036854775809",
    ] {
        // LONG_MIN == 0x8000_0000_0000_0000, whose low 32 bits are zero.
        assert_prints(&pair, arg, b"0", "0\n");
    }

    for _ in 0..100 {
        let n = rng.below(56) + 25;
        let mut s = vec![b'-'];
        for i in 0..n {
            let d = if i == 0 {
                (rng.below(9) + 1) as u8
            } else {
                rng.below(10) as u8
            };
            s.push(b'0' + d);
        }
        assert_prints(&pair, &s, b"0", "0\n");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — fits in long but not in int: silent (int) truncation
// ---------------------------------------------------------------------------

#[test]
fn row11_atoi_long_but_not_int() {
    let pair = Pair::default_config();
    for (arg, expected) in [
        (&b"2147483648"[..], -2147483648i64),
        (b"2147483649", -2147483647),
        (b"-2147483649", 2147483647),
        (b"4294967296", 0),
        (b"4294967297", 1),
        (b"-4294967296", 0),
        (b"1099511627776", 0),
        (b"9223372036854775807", -1),
        (b"-9223372036854775808", 0),
    ] {
        assert_prints(&pair, arg, b"0", &format!("{expected}\n"));
    }
}

// ---------------------------------------------------------------------------
// Rows 12–14 — argc is never read
// ---------------------------------------------------------------------------

fn assert_argc_ignored(pair: &Pair, argc: c_int) {
    let args: [&[u8]; 3] = [b"driver", b"20", b"22"];
    let mut c_argv = Argv::new(&args);
    let mut r_argv = Argv::new(&args);

    let (c_rc, c_out) = common::run_main(&pair.c, argc, &mut c_argv, Sink::Pipe);
    let (r_rc, r_out) = common::run_main(&pair.rust, argc, &mut r_argv, Sink::Pipe);

    assert_eq!(c_rc, r_rc, "argc={argc}");
    assert_eq!(c_out, r_out, "argc={argc}");
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        "42\n",
        "argc={argc}: the C code ignores argc entirely"
    );
}

#[test]
fn row12_argc_zero_ignored() {
    let pair = Pair::default_config();
    assert_argc_ignored(&pair, 0);
    assert_argc_ignored(&pair, 1);
    assert_argc_ignored(&pair, 2);
}

#[test]
fn row13_argc_negative_ignored() {
    let pair = Pair::default_config();
    assert_argc_ignored(&pair, -1);
    assert_argc_ignored(&pair, -1000);
    assert_argc_ignored(&pair, c_int::MIN);
}

#[test]
fn row14_argc_int_max_ignored() {
    let pair = Pair::default_config();
    assert_argc_ignored(&pair, c_int::MAX);
    assert_argc_ignored(&pair, c_int::MAX - 1);
}

// ---------------------------------------------------------------------------
// Row 15 — the unchecked int addition wraps
// ---------------------------------------------------------------------------

#[test]
fn row15_int_addition_overflow_wraps() {
    let pair = Pair::default_config();
    for (a, b, expected) in [
        (&b"2147483647"[..], &b"1"[..], "-2147483648\n"),
        (b"2147483647", b"2147483647", "-2\n"),
        (b"-2147483648", b"-1", "2147483647\n"),
        (b"-2147483648", b"-2147483648", "0\n"),
        (b"2147483646", b"2", "-2147483648\n"),
        (b"1073741824", b"1073741824", "-2147483648\n"),
    ] {
        assert_prints(&pair, a, b, expected);
    }

    // Randomised: every pair of int32 values, wrapping.
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..300 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let expected = format!("{}\n", a.wrapping_add(b));
        assert_prints(
            &pair,
            a.to_string().as_bytes(),
            b.to_string().as_bytes(),
            &expected,
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 16–20 — the unchecked pointer arithmetic
// ---------------------------------------------------------------------------

fn both_find(pair: &Pair, which: char, addr: usize) -> (usize, usize) {
    let (c_fn, r_fn) = match which {
        'a' => (pair.c.find_container_of_a(), pair.rust.find_container_of_a()),
        _ => (pair.c.find_container_of_b(), pair.rust.find_container_of_b()),
    };
    let c_out = unsafe { c_fn(addr as *mut c_int) } as usize;
    let r_out = unsafe { r_fn(addr as *mut c_int) } as usize;
    assert_eq!(
        c_out, r_out,
        "find_container_of_{which}({addr:#x}): C={c_out:#x} Rust={r_out:#x}"
    );
    (c_out, r_out)
}

#[test]
fn row16_find_container_of_a_null() {
    let pair = Pair::default_config();
    let (c_out, _) = both_find(&pair, 'a', 0);
    assert_eq!(c_out, 0, "offsetof(struct test, a) == 0, so NULL stays NULL");
}

#[test]
fn row17_find_container_of_b_null() {
    let pair = Pair::default_config();
    let (c_out, _) = both_find(&pair, 'b', 0);
    assert_eq!(
        c_out, 0xFFFF_FFFF_FFFF_FFFC,
        "NULL - offsetof(struct test, b) wraps below zero"
    );
}

#[test]
fn row18_find_container_of_b_underflow() {
    let pair = Pair::default_config();
    for addr in 1usize..=3 {
        let (c_out, _) = both_find(&pair, 'b', addr);
        assert_eq!(c_out, addr.wrapping_sub(4));
    }
    // ... and `a` never wraps, because it subtracts nothing.
    for addr in 0usize..=3 {
        let (c_out, _) = both_find(&pair, 'a', addr);
        assert_eq!(c_out, addr);
    }
}

#[test]
fn row19_find_container_of_b_max() {
    let pair = Pair::default_config();
    for addr in [usize::MAX, usize::MAX - 1, usize::MAX - 3, usize::MAX - 4] {
        let (c_out, _) = both_find(&pair, 'b', addr);
        assert_eq!(c_out, addr - 4);
    }
}

#[test]
fn row20_misaligned_pointers() {
    let pair = Pair::default_config();
    let mut rng = Rng::new(SEED ^ 20);

    // Odd / unaligned addresses are invalid `int *` values in C, but the
    // functions never dereference them, so they must be accepted as-is.
    for _ in 0..500 {
        let addr = (rng.next_u64() | 1) as usize;
        both_find(&pair, 'a', addr);
        both_find(&pair, 'b', addr);
    }
    for addr in [1usize, 3, 5, 7, 9, 0x1001, 0xFFFF_FFFF_FFFF_FFFF] {
        both_find(&pair, 'a', addr);
        both_find(&pair, 'b', addr);
    }
}

// ---------------------------------------------------------------------------
// Row 21 — printf failure is ignored: main still returns 0
// ---------------------------------------------------------------------------

/// Runs `main` in a forked child whose stdout is `path` (`/dev/full` makes every
/// write fail with `ENOSPC`) or closed entirely (`EBADF`), and reports the exit
/// status the caller would see.
fn child_main_with_broken_stdout(imp: &common::Impl, path: Option<&str>) -> Outcome {
    let entry = imp.main();
    let prog = CString::new("driver").unwrap();
    let a = CString::new("1").unwrap();
    let b = CString::new("2").unwrap();
    let mut argv = vec![
        prog.as_ptr() as *mut c_char,
        a.as_ptr() as *mut c_char,
        b.as_ptr() as *mut c_char,
        std::ptr::null_mut(),
    ];
    let argv_ptr = argv.as_mut_ptr();
    let cpath = path.map(|p| CString::new(p).unwrap());

    // Serialise with every other test that touches fd 1 or forks.
    let _guard = common::fd1_guard();

    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            match &cpath {
                Some(p) => {
                    let fd = libc::open(p.as_ptr(), libc::O_WRONLY);
                    if fd < 0 {
                        libc::_exit(101);
                    }
                    libc::dup2(fd, 1);
                    if fd != 1 {
                        libc::close(fd);
                    }
                }
                None => {
                    libc::close(1);
                }
            }

            let rc = entry(3, argv_ptr);
            // Force the C stream flush to actually attempt the failing write,
            // exactly as process exit would.
            fflush(std::ptr::null_mut());
            libc::_exit(rc);
        }

        let mut status: c_int = 0;
        assert_eq!(libc::waitpid(pid, &mut status, 0), pid);
        if libc::WIFSIGNALED(status) {
            Outcome::Signaled(libc::WTERMSIG(status))
        } else {
            Outcome::Exited(libc::WEXITSTATUS(status))
        }
    }
}

#[test]
fn row21_printf_failure_ignored() {
    let pair = Pair::default_config();

    // stdout closed -> write(2) fails with EBADF.
    let c_closed = child_main_with_broken_stdout(&pair.c, None);
    let r_closed = child_main_with_broken_stdout(&pair.rust, None);
    assert_eq!(c_closed, r_closed, "closed stdout");
    assert_eq!(
        c_closed,
        Outcome::Exited(0),
        "the C main returns 0 even when printf fails"
    );

    // /dev/full -> write(2) fails with ENOSPC.
    if std::path::Path::new("/dev/full").exists() {
        let c_full = child_main_with_broken_stdout(&pair.c, Some("/dev/full"));
        let r_full = child_main_with_broken_stdout(&pair.rust, Some("/dev/full"));
        assert_eq!(c_full, r_full, "/dev/full stdout");
        assert_eq!(c_full, Outcome::Exited(0));
    }
}
