//! Differential tests for the `abort()` path of `bin2hex`.
//!
//! The aborting calls happen in a re-executed child process so the harness
//! survives; the child's termination signal / exit status is then compared
//! between the C and the Rust implementation.

mod common;

use common::Impls;
use std::process::{Command, Output};

const IMPL_ENV: &str = "BIN2HEX_DIFF_IMPL";
const CASE_ENV: &str = "BIN2HEX_DIFF_CASE";

/// (hex_maxlen, bin_len) argument pairs that must make both implementations abort.
fn abort_cases() -> Vec<(&'static str, usize, usize)> {
    vec![
        // hex_maxlen == bin_len * 2  -> no room for the NUL terminator
        ("exact_no_room_0", 0, 0),
        ("exact_no_room_1", 2, 1),
        ("exact_no_room_8", 16, 8),
        // hex_maxlen < bin_len * 2
        ("too_small_1", 1, 1),
        ("too_small_2", 3, 4),
        ("too_small_3", 0, 1),
        // bin_len >= SIZE_MAX / 2 (checked before any dereference)
        ("bin_len_size_max_half", usize::MAX, usize::MAX / 2),
        ("bin_len_size_max", usize::MAX, usize::MAX),
    ]
}

fn spawn(case: &str, which: &str) -> Output {
    let exe = std::env::current_exe().expect("current_exe");
    Command::new(exe)
        .args(["--exact", "abort_child", "--ignored", "--nocapture"])
        .env(CASE_ENV, case)
        .env(IMPL_ENV, which)
        .output()
        .expect("spawn child")
}

fn describe(o: &Output) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        format!(
            "code={:?} signal={:?}",
            o.status.code(),
            o.status.signal()
        )
    }
    #[cfg(not(unix))]
    {
        format!("code={:?}", o.status.code())
    }
}

#[cfg(unix)]
const fn libc_sigabrt() -> i32 {
    6
}

#[test]
fn abort_paths_match() {
    for (case, hex_maxlen, bin_len) in abort_cases() {
        let c = spawn(case, "c");
        let r = spawn(case, "rust");
        assert_eq!(
            describe(&c),
            describe(&r),
            "case {case} (hex_maxlen={hex_maxlen}, bin_len={bin_len})"
        );
        assert!(
            !c.status.success(),
            "case {case}: C implementation was expected to abort, got {}",
            describe(&c)
        );
        // Be specific: the C code calls abort(), i.e. death by SIGABRT. Without
        // this, two children that both died for an unrelated reason (e.g. a
        // harness panic) would compare equal and pass.
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(
                c.status.signal(),
                Some(libc_sigabrt()),
                "case {case}: expected SIGABRT from the C .so, got {} (stderr: {})",
                describe(&c),
                String::from_utf8_lossy(&c.stderr)
            );
        }
    }
}

/// Helper process body. Not a real test: it is `#[ignore]`d and only executed
/// via `spawn` above.
#[test]
#[ignore = "helper process for abort_paths_match"]
fn abort_child() {
    let case = std::env::var(CASE_ENV).expect("case env");
    let which = std::env::var(IMPL_ENV).expect("impl env");
    let (_, hex_maxlen, bin_len) = abort_cases()
        .into_iter()
        .find(|(n, _, _)| *n == case)
        .expect("known case");

    let im = Impls::load();
    let f = match which.as_str() {
        "c" => im.c_bin2hex,
        "rust" => im.rust_bin2hex,
        other => panic!("unknown impl {other}"),
    };

    // Real allocations where the sizes are sane, dangling-but-aligned pointers
    // for the absurd ones; the C code validates its arguments before touching
    // either buffer, so nothing is ever dereferenced on these paths.
    let mut hex_buf: Vec<u8> = vec![0u8; hex_maxlen.min(1024)];
    let bin_buf: Vec<u8> = vec![0u8; bin_len.min(1024)];
    let hex_ptr = if hex_buf.is_empty() {
        std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut i8
    } else {
        hex_buf.as_mut_ptr() as *mut i8
    };
    let bin_ptr = if bin_buf.is_empty() {
        std::ptr::NonNull::<u8>::dangling().as_ptr() as *const u8
    } else {
        bin_buf.as_ptr()
    };

    unsafe { f(hex_ptr, hex_maxlen, bin_ptr, bin_len) };

    // Unreachable if the implementation behaves like the C original.
    eprintln!("did not abort");
    std::process::exit(0);
}
