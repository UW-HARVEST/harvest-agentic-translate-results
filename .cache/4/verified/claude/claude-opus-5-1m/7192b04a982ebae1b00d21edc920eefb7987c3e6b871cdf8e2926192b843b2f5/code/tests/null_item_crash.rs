//! The C `parse_number` never NULL-checks `item`: on the SUCCESS path it writes
//! `item->valuedouble` unconditionally. Passing `item == NULL` together with a
//! parsable number therefore faults in the C, and the Rust translation must
//! behave the same way rather than, say, returning an error or aborting with a
//! Rust panic message.
//!
//! Because the call is expected to die, it is performed in a CHILD process and
//! the resulting exit status / signal is compared between the two `.so`s.

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const IMPL_ENV: &str = "HARVEST_NULL_ITEM_IMPL";
const INPUT_ENV: &str = "HARVEST_NULL_ITEM_INPUT";

/// Child-process helper. Never run as part of the normal suite.
#[test]
#[ignore = "helper: executed in a child process, expected to fault"]
fn null_item_child() {
    let which = std::env::var(IMPL_ENV).expect("missing impl env var");
    let input = std::env::var(INPUT_ENV).unwrap_or_else(|_| "123".to_string());
    let f = match which.as_str() {
        "c" => c_parse_number(),
        "rust" => rust_parse_number(),
        other => panic!("bad impl {other:?}"),
    };
    let mut content = input.into_bytes();
    let len = content.len();
    let mut buf = ParseBuffer {
        content: content.as_mut_ptr(),
        length: len,
        offset: 0,
        depth: POISON_DEPTH,
    };
    let ret = unsafe { f(std::ptr::null_mut(), &mut buf) };
    // Reached only if the implementation did NOT fault; encode the outcome.
    println!("survived ret={ret} offset={}", buf.offset);
    std::process::exit(70 + (ret & 1));
}

/// `(exit code, terminating signal)` of the child for one implementation.
fn child_status(which: &str, input: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "--exact",
            "null_item_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(IMPL_ENV, which)
        .env(INPUT_ENV, input)
        .output()
        .expect("spawn child");
    (out.status.code(), out.status.signal())
}

#[test]
fn null_item_on_success_path_behaves_identically() {
    for input in ["123", "0", "-1.5e3", "2147483648", "-1e999", ".5", "1e"] {
        let c = child_status("c", input);
        let r = child_status("rust", input);
        assert_eq!(
            c, r,
            "item == NULL with {input:?}: C {c:?} vs Rust {r:?} must match \
             (code, signal)"
        );
        // Sanity: the C really does fault here (SIGSEGV / SIGBUS), i.e. the
        // comparison above is not vacuously comparing two clean exits.
        assert!(
            c.1.is_some(),
            "expected the C to be killed by a signal for {input:?}, got {c:?}"
        );
    }
}

/// Control: on the FAILURE paths `item == NULL` is harmless in the C, so both
/// must exit cleanly with the same status.
#[test]
fn null_item_on_failure_path_is_clean_in_both() {
    for input in ["+", "]", "", "e", "."] {
        let c = child_status("c", input);
        let r = child_status("rust", input);
        assert_eq!(c, r, "item == NULL with {input:?}: C {c:?} vs Rust {r:?}");
        assert_eq!(
            c,
            (Some(70), None),
            "expected a clean `false` return for {input:?}, got {c:?}"
        );
    }
}
