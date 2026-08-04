use libloading::{Library, Symbol};
use std::env;

fn main() {
    let path = env::args().nth(1).expect("usage: test_runner <path-to-so>");
    unsafe {
        let lib = Library::new(&path).expect("failed to load library");
        let func: Symbol<unsafe extern "C" fn() -> i32> =
            lib.get(b"main").expect("failed to find main");
        let rc = func();
        std::process::exit(rc);
    }
}
