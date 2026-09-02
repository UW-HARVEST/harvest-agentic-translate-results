//! `harness = false` helper binary.
//!
//! The three `shared.h` helpers terminate the process with `exit(EXIT_FAILURE)`
//! on failure, so they cannot be exercised in-process. `tests/err_alloc.rs`
//! re-runs THIS binary once per implementation and compares the exit status and
//! the bytes written to stderr.
//!
//! Usage: `exit_helper <c|rust> <case>`; with no arguments it is a no-op so that
//! plain `cargo test` passes.

mod common;

use std::ffi::{c_void, CString};
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        // Invoked directly by `cargo test` — nothing to do.
        return;
    }

    let (c, r) = common::libs();
    let lib = match args[1].as_str() {
        "c" => c,
        "rust" => r,
        other => {
            eprintln!("unknown impl {other}");
            exit(97);
        }
    };

    unsafe {
        match args[2].as_str() {
            // ERRORS.md E1
            "calloc_oom" => {
                let p = (lib.os_calloc)(usize::MAX, 2);
                println!("UNEXPECTED_SUCCESS {p:?}");
            }
            // ERRORS.md E2
            "realloc_oom" => {
                let p = (lib.os_realloc)(std::ptr::null_mut(), usize::MAX);
                println!("UNEXPECTED_SUCCESS {p:?}");
            }
            // ERRORS.md E2, from a live pointer
            "realloc_oom_live" => {
                let live = (lib.os_calloc)(16, 1);
                let p = (lib.os_realloc)(live, usize::MAX);
                println!("UNEXPECTED_SUCCESS {p:?}");
            }
            // ERRORS.md E3
            "strdup_null" => {
                let p = (lib.os_strdup)(std::ptr::null());
                println!("UNEXPECTED_SUCCESS {p:?}");
            }
            // Sanity: the happy paths must NOT exit.
            "ok" => {
                let p = (lib.os_calloc)(4, 8);
                assert!(!p.is_null());
                let q = (lib.os_realloc)(p, 64);
                assert!(!q.is_null());
                let s = CString::new("hello").unwrap();
                let d = (lib.os_strdup)(s.as_ptr());
                assert!(!d.is_null());
                common::free(d as *mut c_void);
                common::free(q);
                println!("OK");
            }

            // ---- ERRORS.md G8: NULL across the FFI boundary --------------
            // Every prototype is __attribute__((nonnull)) and the C
            // dereferences unconditionally. Both implementations must fault
            // the same way; a defensive NULL check in the Rust would show up
            // here as a clean exit where the C died.
            "gad_null_fp" => {
                let a = (lib.get_alert_data)(0, std::ptr::null_mut());
                println!("NO_FAULT {a:?}");
            }
            "free_null" => {
                (lib.free_alert_data)(std::ptr::null_mut());
                println!("NO_FAULT");
            }
            "init_null_fq" => {
                let t = common::tm::new(19, 3, 116);
                let rc = (lib.init_file_queue)(std::ptr::null_mut(), &t, 0);
                println!("NO_FAULT {rc}");
            }
            "init_null_tm" => {
                let mut fq = common::file_queue::zeroed();
                let rc = (lib.init_file_queue)(&mut fq, std::ptr::null(), 0x004);
                println!("NO_FAULT {rc}");
            }
            "readmon_null_fq" => {
                let t = common::tm::new(19, 3, 116);
                let a = (lib.read_file_mon)(std::ptr::null_mut(), &t, 0);
                println!("NO_FAULT {a:?}");
            }
            "readmon_null_tm" => {
                // fp must be non-NULL and GetAlertData must fail so that the
                // NULL `tm` is actually dereferenced.
                std::env::set_current_dir(common::scratch_dir()).ok();
                common::write_alerts_log(b"not an alert\n");
                let mut fq = common::file_queue::zeroed();
                let t = common::tm::new(19, 3, 116);
                let rc = (lib.init_file_queue)(&mut fq, &t, 0x004);
                assert_eq!(rc, 0);
                let a = (lib.read_file_mon)(&mut fq, std::ptr::null(), 0);
                println!("NO_FAULT {a:?}");
            }
            "merror_null_template" => {
                (lib.merror)(
                    std::ptr::null(),
                    b"f\0".as_ptr() as *const std::ffi::c_char,
                    1,
                    b"m\0".as_ptr() as *const std::ffi::c_char,
                );
                println!("NO_FAULT");
            }
            // U4 probe: merror's `char buffer[256]` is UNINITIALIZED in the C.
            // Call it once with a real template (dirtying that stack slot),
            // then with a NULL template. If glibc's snprintf writes nothing for
            // a NULL format, the C would emit the STALE bytes while the Rust
            // (whose buffer is zeroed) emits just "\n".
            "merror_null_after_real" => {
                let t = std::ffi::CString::new("STALE-STALE-STALE %s %d %s").unwrap();
                let n = std::ffi::CString::new("A".repeat(200)).unwrap();
                let m = std::ffi::CString::new("BBBB").unwrap();
                (lib.merror)(t.as_ptr(), n.as_ptr(), 12345, m.as_ptr());
                (lib.merror)(
                    std::ptr::null(),
                    b"f\0".as_ptr() as *const std::ffi::c_char,
                    1,
                    b"m\0".as_ptr() as *const std::ffi::c_char,
                );
                println!("NO_FAULT");
            }
            other => {
                eprintln!("unknown case {other}");
                exit(98);
            }
        }
    }
}
