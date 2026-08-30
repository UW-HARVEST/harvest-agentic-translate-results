//! Phase B (continued) — environment / ABI axes that the row tests cannot see.
//!
//! Each test here targets a specific class of C-to-Rust translation error that
//! byte-comparing outputs in the default environment would silently miss:
//!
//! * `locale_*`          — a Rust-side reimplementation of `printf`'s `%.1f`
//!                         instead of calling libc. Identical in the "C"
//!                         locale, divergent under `LC_NUMERIC=de_DE`.
//! * `global_state_*`    — `the_house` translated as `thread_local!` instead of
//!                         `static mut`. Identical on one thread, divergent as
//!                         soon as a second thread calls in.
//! * `input_is_readonly` — `parse_val` casts `const char *` to `char *`; a
//!                         translation that actually wrote through it would
//!                         fault on read-only memory.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
}

const LC_ALL: c_int = 6; // glibc
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;

// ---------------------------------------------------------------------------
// Locale axis
// ---------------------------------------------------------------------------

fn with_locale<F: FnOnce()>(name: &str, f: F) -> bool {
    let cname = format!("{name}\0");
    unsafe {
        let got = setlocale(LC_ALL, cname.as_ptr() as *const c_char);
        if got.is_null() {
            return false; // locale not installed on this host
        }
        f();
        // Restore the process locale for every other test.
        let c = b"C\0";
        setlocale(LC_ALL, c.as_ptr() as *const c_char);
        true
    }
}

/// `%.1f` under a locale whose decimal separator is a comma.
///
/// Both libraries must go through libc `printf`, so both must print `,5`.
#[test]
fn locale_lc_numeric_comma_decimal_separator() {
    // Take the lock for the whole test: setlocale is process-global.
    let _g = lock();
    let p = pair();

    let mut tried_any = false;
    for name in ["de_DE.utf8", "de_DE.UTF-8", "fr_FR.utf8", "ru_RU.utf8", "es_ES.utf8"] {
        let installed = with_locale(name, || {
            // Drive both libraries through the low-level entry point...
            for x in [0i32, 7, -7, c_int::MAX, c_int::MIN] {
                set_errno(0);
                let c_out = capture(|| unsafe { p.c.run(x) });
                set_errno(0);
                let rust_out = capture(|| unsafe { p.rust.run(x) });
                assert_eq!(
                    String::from_utf8_lossy(&c_out),
                    String::from_utf8_lossy(&rust_out),
                    "locale {name}: run({x}) diverged"
                );
                assert!(
                    c_out.contains(&b','),
                    "locale {name} should use ',' as the decimal separator, \
                     but the C printed {:?} — pick another locale",
                    String::from_utf8_lossy(&c_out)
                );
                assert!(
                    !c_out.contains(&b'.'),
                    "locale {name}: unexpected '.' in {:?}",
                    String::from_utf8_lossy(&c_out)
                );
            }
            // ...and through the convenience wrapper, accepting and rejecting.
            for s in ["123", "-2147483648", "", "abc", "2147483648"] {
                let mut buf = s.as_bytes().to_vec();
                buf.push(0);
                set_errno(0);
                let c_out = capture(|| unsafe { p.c.driver(buf.as_ptr() as *const c_char) });
                set_errno(0);
                let rust_out = capture(|| unsafe { p.rust.driver(buf.as_ptr() as *const c_char) });
                assert_eq!(
                    String::from_utf8_lossy(&c_out),
                    String::from_utf8_lossy(&rust_out),
                    "locale {name}: driver({s:?}) diverged"
                );
            }
        });
        if installed {
            tried_any = true;
            break;
        }
    }
    assert!(
        tried_any,
        "no comma-decimal locale installed; cannot run this check"
    );
}

/// Explicitly re-assert agreement in the "C" and "C.utf8" locales, and that
/// switching locales back and forth leaves both implementations in step.
#[test]
fn locale_round_trip_c_and_back() {
    let _g = lock();
    let p = pair();
    for name in ["C", "C.utf8", "POSIX", "en_US.utf8"] {
        with_locale(name, || {
            set_errno(0);
            let c_out = capture(|| unsafe { p.c.run(3) });
            set_errno(0);
            let rust_out = capture(|| unsafe { p.rust.run(3) });
            assert_eq!(
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&rust_out),
                "locale {name} diverged"
            );
        });
    }
}

// ---------------------------------------------------------------------------
// Global-state axis: `the_house` is process-global, not per-thread.
// ---------------------------------------------------------------------------

/// The C's `static house_t the_house` is one object per process. A translation
/// using `thread_local!` would silently reset the state on a new thread.
#[test]
fn global_state_is_process_global_not_thread_local() {
    // Advance the state well away from its initial value first, so that a
    // freshly-initialised (per-thread) house is unmistakably distinguishable
    // from the shared one.
    advance_both_silently(500, 1);

    // Establish where we are on this thread.
    let out_main = assert_same(&Call::Run(0));
    let floors_main = floors_of(&out_main);
    assert!(
        floors_main > 400,
        "expected deep state before the thread check, got floors={floors_main}"
    );

    // A second thread must observe the *continued* state, not a fresh house.
    let out_other = std::thread::spawn(|| assert_same(&Call::Run(0)))
        .join()
        .expect("worker thread panicked");
    let floors_other = floors_of(&out_other);

    assert_eq!(
        floors_other,
        floors_main + 1,
        "the second thread saw floors={floors_other} after floors={floors_main} \
         on the first thread; state is not shared across threads (a \
         `thread_local!` translation of `static house_t the_house`?)"
    );
    assert!(
        floors_other > 400,
        "a freshly-initialised (per-thread) house would report floors=2/3; \
         got {floors_other}"
    );

    // Back on the original thread the state must have advanced further still.
    let out_back = assert_same(&Call::Run(0));
    assert_eq!(floors_of(&out_back), floors_other + 1);

    // And across a whole chain of distinct threads.
    let mut prev = floors_of(&out_back);
    for _ in 0..8 {
        let out = std::thread::spawn(|| assert_same(&Call::Run(1)))
            .join()
            .unwrap();
        let f = floors_of(&out);
        assert_eq!(f, prev + 1, "state lost between threads");
        prev = f;
    }
}

fn floors_of(out: &[u8]) -> i64 {
    let text = String::from_utf8_lossy(out);
    let line = text.lines().next().expect("at least one line");
    line.split(' ')
        .nth(3)
        .unwrap_or_else(|| panic!("cannot find floors in {line:?}"))
        .parse()
        .unwrap_or_else(|e| panic!("cannot parse floors in {line:?}: {e}"))
}

// ---------------------------------------------------------------------------
// Const-correctness axis
// ---------------------------------------------------------------------------

/// `parse_val` does `char *endp = (char *)str;` — it casts the `const` away but
/// must never write through it. Passing a read-only page proves neither
/// implementation writes to the caller's buffer.
#[test]
fn input_is_readonly_and_unmodified() {
    let _g = lock();
    let p = pair();
    let page = 4096usize;

    for s in [
        "12345", "  -42junk", "", "abc", "99999999999999999999999", "2147483648",
        "-2147483649", "+0007",
    ] {
        unsafe {
            let mem = mmap(
                std::ptr::null_mut(),
                page,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(mem as isize != -1, "mmap failed");
            // Fill with a recognisable pattern, then place the NUL-terminated
            // input at the start.
            std::ptr::write_bytes(mem as *mut u8, 0xAB, page);
            let bytes = s.as_bytes();
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), mem as *mut u8, bytes.len());
            *(mem as *mut u8).add(bytes.len()) = 0;

            let snapshot: Vec<u8> = std::slice::from_raw_parts(mem as *const u8, page).to_vec();

            // Make the page read-only: any write by either library faults.
            assert_eq!(mprotect(mem, page, PROT_READ), 0, "mprotect failed");

            set_errno(0);
            let c_out = capture(|| p.c.driver(mem as *const c_char));
            set_errno(0);
            let rust_out = capture(|| p.rust.driver(mem as *const c_char));

            assert_eq!(
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&rust_out),
                "read-only input {s:?} diverged"
            );

            let after: Vec<u8> = std::slice::from_raw_parts(mem as *const u8, page).to_vec();
            assert!(
                after == snapshot,
                "input buffer was modified for {s:?}"
            );
            munmap(mem, page);
        }
    }
}
