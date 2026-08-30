//! Out-of-process script runner used by the differential tests.
//!
//! The integration tests cannot reliably capture file descriptor 1 in-process
//! because the `cargo test` harness writes its own progress output there from
//! other threads. This runner executes a script of API calls against a single
//! shared library (either the C one or the Rust one) in a dedicated process, so
//! its stdout contains nothing but the bytes the library produced plus the
//! canonical observation lines.
//!
//! Usage:
//!
//! ```text
//! driver_runner <path-to-libStaticAlias.so> <step> [<step> ...]
//!
//!   d:<initial_value>:<iterations>   call driver(initial_value, iterations)
//!   a:<outer>                        call static_alias(&outer) and report
//! ```
//!
//! Observation lines are written with the C `printf` (not Rust's `println!`) so
//! that they share the library's `FILE *stdout` buffer and therefore interleave
//! with the library's own output in a fully deterministic order.

use std::ffi::{c_int, CString};

use libloading::{Library, Symbol};

type StaticAliasFn = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
type DriverFn = unsafe extern "C" fn(c_int, c_int);

fn emit(line: &str) {
    let c = CString::new(line).expect("no interior NUL");
    unsafe {
        libc::printf(c"%s\n".as_ptr(), c.as_ptr());
    }
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let so_path = args.next().expect("usage: driver_runner <so> <step>...");
    let steps: Vec<String> = args
        .map(|a| a.into_string().expect("step must be UTF-8"))
        .collect();

    let lib = unsafe { Library::new(&so_path) }
        .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", so_path));
    let static_alias: Symbol<StaticAliasFn> =
        unsafe { lib.get(b"static_alias\0") }.expect("static_alias");
    let driver: Symbol<DriverFn> = unsafe { lib.get(b"driver\0") }.expect("driver");

    // Address of the static returned by the previous `if`-branch call, used to
    // check that it really is stable storage.
    let mut prev_internal: Option<*mut c_int> = None;

    for step in &steps {
        let mut parts = step.split(':');
        match parts.next() {
            Some("d") => {
                let iv: c_int = parts.next().expect("d:<iv>:<it>").parse().expect("iv");
                let it: c_int = parts.next().expect("d:<iv>:<it>").parse().expect("it");
                unsafe { driver(iv, it) };
            }
            Some("a") => {
                let v: c_int = parts.next().expect("a:<outer>").parse().expect("outer");
                let mut cell: c_int = v;
                let outer_ptr: *mut c_int = &mut cell;
                let ret = unsafe { static_alias(outer_ptr) };
                assert!(!ret.is_null(), "static_alias returned NULL");

                let target = if ret == outer_ptr { "OUTER" } else { "INTERNAL" };
                let ret_value = unsafe { *ret };
                let stable = if ret == outer_ptr {
                    "n/a"
                } else {
                    match prev_internal {
                        None => "first",
                        Some(p) if p == ret => "yes",
                        Some(_) => "no",
                    }
                };
                if ret != outer_ptr {
                    prev_internal = Some(ret);
                }
                emit(&format!(
                    "A in={v} target={target} ret={ret_value} outer={cell} stable={stable}"
                ));
            }
            other => panic!("unknown step {other:?}"),
        }
    }

    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
}
