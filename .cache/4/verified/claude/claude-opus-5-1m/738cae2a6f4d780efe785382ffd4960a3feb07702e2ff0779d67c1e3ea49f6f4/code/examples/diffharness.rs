//! Child process used by the application level differential tests
//! (`tests/configs_app.rs`, `tests/errors_app.rs`).
//!
//! ```text
//! diffharness <shared-object> <result-file> <exported-fn> [<exported-fn> ...]
//! ```
//!
//! The shared object is loaded with `libloading` and the named exported symbols
//! are called in order.  Everything the library prints goes to the inherited
//! `stdout` / `stderr` (the parent redirects them to files); the harness itself
//! only writes the call results into `<result-file>`.
//!
//! A separate process per scenario is what makes the application level functions
//! testable at all: they keep their state in `static` variables of the shared
//! object, they read `stdin`, and `while (getchar() != '\n');` never returns once
//! `stdin` is at EOF - so the parent applies a timeout and compares the two
//! implementations' partial output.

use std::ffi::c_int;
use std::io::Write;

use libloading::{Library, Symbol};

type VoidFn = unsafe extern "C" fn();
type MainFn = unsafe extern "C" fn() -> c_int;

const VOID_FNS: &[&str] = &[
    "shape_manager_init",
    "shape_manager_cleanup",
    "print_menu",
    "view_all_shapes",
    "create_new_scene",
    "add_shape_to_scene",
    "remove_shape_from_scene",
    "view_scene",
    "list_all_scenes",
    "save_scene_to_file",
    "load_scene_from_file",
    "compare_shapes",
    "compare_scenes",
    "delete_scene",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: diffharness <lib.so> <result-file> <fn>...");
        std::process::exit(2);
    }
    let lib_path = &args[1];
    let result_path = &args[2];
    let calls = &args[3..];

    let mut results = std::fs::File::create(result_path).expect("result file");

    let lib = unsafe { Library::new(lib_path) }.expect("dlopen");

    for name in calls {
        if name == "main" {
            let f: Symbol<MainFn> = unsafe { lib.get(b"main\0") }.expect("main");
            let rc = unsafe { f() };
            writeln!(results, "call main -> {}", rc).unwrap();
            results.flush().unwrap();
        } else if VOID_FNS.contains(&name.as_str()) {
            let sym = format!("{}\0", name);
            let f: Symbol<VoidFn> = unsafe { lib.get(sym.as_bytes()) }.expect("symbol");
            unsafe { f() };
            writeln!(results, "call {}", name).unwrap();
            results.flush().unwrap();
        } else {
            panic!("diffharness: unknown function `{}`", name);
        }
    }

    // Flush the C streams the library wrote to (the parent compares the bytes).
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
}
