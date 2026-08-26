//! Abort-parity for the one live `assert()` in the C that an external caller
//! can reach: `strconv.c:53`
//!
//! ```c
//! int jsonp_strtod(strbuffer_t *strbuffer, double *out) {
//!     ...
//!     value = strtod(strbuffer->value, &end);
//!     assert(end == strbuffer->value + strbuffer->length);
//! ```
//!
//! `c_src/CMakeLists.txt` does not define `NDEBUG`, so the assertion is live and
//! `jsonp_strtod` is an exported symbol. Handing it a strbuffer that `strtod()`
//! does not consume in full therefore makes the C library abort — and the Rust
//! must abort too, not silently return 0.
//!
//! The check runs in a child process (this same test binary re-executed with
//! `JANSSON_ABORT_PROBE` set), because the probe intentionally kills itself.
mod common;
use common::*;
use std::ffi::{c_char, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

/// Build a strbuffer whose `length` deliberately disagrees with what `strtod`
/// will consume, then call `jsonp_strtod`.
fn probe(which: &str, input: &[u8]) -> ! {
    let d = duo();
    let l = if which == "C" { &d.c } else { &d.rs };
    unsafe {
        let mut sb = strbuffer_t::zeroed();
        assert_eq!((l.strbuffer_init)(&mut sb), 0);
        assert_eq!(
            (l.strbuffer_append_bytes)(&mut sb, input.as_ptr() as *const c_char, input.len()),
            0
        );
        let mut out = 0.0f64;
        let rc = (l.jsonp_strtod)(&mut sb, &mut out);
        // If we reach here the library did NOT abort. Report that distinctly.
        println!("NO_ABORT rc={} out={:#018x}", rc, out.to_bits());
        (l.strbuffer_close)(&mut sb);
        std::process::exit(42);
    }
}

fn run_child(which: &str, case: &str) -> (Option<i32>, Option<i32>, String) {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .args(["--exact", "strtod_assert_abort_parity", "--nocapture"])
        .env("JANSSON_ABORT_PROBE", which)
        .env("JANSSON_ABORT_CASE", case)
        .output()
        .expect("re-exec self");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code(), out.status.signal(), text)
}

#[test]
fn strtod_assert_abort_parity() {
    // ---- child role -------------------------------------------------------
    if let Ok(which) = std::env::var("JANSSON_ABORT_PROBE") {
        let case = std::env::var("JANSSON_ABORT_CASE").unwrap_or_default();
        let input: &[u8] = match case.as_str() {
            // strtod stops at the 'e' with no exponent digits -> consumes "1"
            "trailing-e" => b"1e",
            // strtod consumes nothing at all
            "garbage" => b"zzz",
            // strtod consumes "1", leaving " x"
            "trailing-space" => b"1 x",
            // strtod consumes "1.5", leaving "abc"
            "trailing-alpha" => b"1.5abc",
            // fully consumed: MUST NOT abort
            "ok" => b"1.5",
            "ok-int" => b"42",
            "ok-exp" => b"1e10",
            _ => b"1e",
        };
        probe(&which, input);
    }

    // ---- parent role ------------------------------------------------------
    for case in [
        "trailing-e",
        "garbage",
        "trailing-space",
        "trailing-alpha",
        "ok",
        "ok-int",
        "ok-exp",
    ] {
        let (ccode, csig, ctext) = run_child("C", case);
        let (rcode, rsig, rtext) = run_child("RUST", case);
        eq(
            &format!("jsonp_strtod[{}] exit code", case),
            ccode,
            rcode,
        );
        eq(&format!("jsonp_strtod[{}] signal", case), csig, rsig);
        // For the non-consumable cases both must die on SIGABRT (6).
        if case.starts_with("ok") {
            assert_eq!(
                csig, None,
                "case {}: C should not abort on a fully consumed buffer (stderr: {})",
                case, ctext
            );
            assert!(
                ctext.contains("NO_ABORT"),
                "case {}: expected the C probe to return normally, got: {}",
                case,
                ctext
            );
            assert!(
                rtext.contains("NO_ABORT"),
                "case {}: expected the RUST probe to return normally, got: {}",
                case,
                rtext
            );
        } else {
            assert_eq!(
                csig,
                Some(6),
                "case {}: expected the C to abort (SIGABRT); stdout+stderr: {}",
                case,
                ctext
            );
            assert_eq!(
                rsig,
                Some(6),
                "case {}: expected the RUST to abort (SIGABRT); stdout+stderr: {}",
                case,
                rtext
            );
            // Both must name the same assertion and function.
            for (who, t) in [("C", &ctext), ("RUST", &rtext)] {
                assert!(
                    t.contains("end == strbuffer->value + strbuffer->length"),
                    "{} case {}: assertion text missing from: {}",
                    who,
                    case,
                    t
                );
                assert!(
                    t.contains("jsonp_strtod"),
                    "{} case {}: function name missing from: {}",
                    who,
                    case,
                    t
                );
            }
        }
    }
}

/// The C `.so` imports `__assert_fail`; the Rust `.so` must be able to reach the
/// same abort path (it imports `__assert_fail` too now).
#[test]
fn both_libraries_import_assert_fail() {
    for p in [c_so_path(), rust_so_path()] {
        let out = std::process::Command::new("nm")
            .args(["-D", "--undefined-only", p.to_str().unwrap()])
            .output()
            .expect("nm");
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            text.contains("__assert_fail"),
            "{} does not import __assert_fail:\n{}",
            p.display(),
            text
        );
    }
}

/// Sanity: `jsonp_strtod` on fully-consumable input agrees between the two
/// libraries (the non-aborting path), for a wide set of numeric strings.
#[test]
fn jsonp_strtod_consumable_inputs_agree() {
    let d = duo();
    unsafe {
        let mut inputs: Vec<Vec<u8>> = vec![
            b"0".to_vec(),
            b"-0".to_vec(),
            b"1".to_vec(),
            b"1.5".to_vec(),
            b"-1.5".to_vec(),
            b"1e10".to_vec(),
            b"1E-10".to_vec(),
            b"1e300".to_vec(),
            b"1e308".to_vec(),
            b"1e309".to_vec(),
            b"1e999".to_vec(),
            b"-1e999".to_vec(),
            b"1e-999".to_vec(),
            b"0.0000000000000000001".to_vec(),
            b"4.9406564584124654e-324".to_vec(),
            b"1.7976931348623157e308".to_vec(),
        ];
        inputs.push(format!("{}", "9".repeat(400)).into_bytes());
        inputs.push(format!("0.{}", "1".repeat(400)).into_bytes());
        let mut rng = Rng::new(0x57D0_1234);
        for _ in 0..2000 {
            let m = rng.range_i64(-1_000_000_000, 1_000_000_000);
            let e = rng.range_i64(-320, 320);
            inputs.push(format!("{}e{}", m, e).into_bytes());
            inputs.push(format!("{}.{}", m.abs(), rng.next_u32() % 1_000_000).into_bytes());
        }
        for inp in &inputs {
            let mut csb = strbuffer_t::zeroed();
            let mut rsb = strbuffer_t::zeroed();
            assert_eq!((d.c.strbuffer_init)(&mut csb), 0);
            assert_eq!((d.rs.strbuffer_init)(&mut rsb), 0);
            (d.c.strbuffer_append_bytes)(&mut csb, inp.as_ptr() as *const c_char, inp.len());
            (d.rs.strbuffer_append_bytes)(&mut rsb, inp.as_ptr() as *const c_char, inp.len());
            let mut cout = f64::NAN;
            let mut rout = f64::NAN;
            let crc = (d.c.jsonp_strtod)(&mut csb, &mut cout);
            let rrc = (d.rs.jsonp_strtod)(&mut rsb, &mut rout);
            let what = format!("jsonp_strtod {:?}", String::from_utf8_lossy(inp));
            eq(&format!("{} ret", what), crc, rrc);
            eq(&format!("{} out", what), cout.to_bits(), rout.to_bits());
            // the buffer itself must be unchanged (to_locale is a no-op in "C")
            eq(
                &format!("{} buffer", what),
                cstr_bytes(csb.value),
                cstr_bytes(rsb.value),
            );
            eq(&format!("{} length", what), csb.length, rsb.length);
            (d.c.strbuffer_close)(&mut csb);
            (d.rs.strbuffer_close)(&mut rsb);
        }
        let _ = std::mem::size_of::<*mut c_void>();
    }
}
