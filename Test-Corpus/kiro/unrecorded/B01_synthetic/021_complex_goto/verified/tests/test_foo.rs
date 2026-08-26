use libloading::{Library, Symbol};
use std::os::unix::io::FromRawFd;
use std::io::Read;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/libdriver.so");
const RUST_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/release/libtranslated_rust.so");

type FooFn = unsafe extern "C" fn(i32, i32);

/// Capture stdout output from calling `f` by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    unsafe {
        let mut pipefd = [0i32; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        assert!(old_stdout >= 0);
        libc::dup2(pipefd[1], 1);
        libc::close(pipefd[1]);

        f();

        // flush C stdout
        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut file = std::fs::File::from_raw_fd(pipefd[0]);
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn call_foo(lib: &Library, x: i32, y: i32) -> String {
    capture_stdout(|| unsafe {
        let foo: Symbol<FooFn> = lib.get(b"foo").expect("symbol foo not found");
        foo(x, y);
    })
}

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn compare_foo(x: i32, y: i32) {
    let _lock = TEST_LOCK.lock().unwrap();
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("load Rust lib") };
    let c_out = call_foo(&c_lib, x, y);
    let rust_out = call_foo(&rust_lib, x, y);
    assert_eq!(c_out, rust_out, "mismatch for foo({x}, {y}):\nC:    {c_out:?}\nRust: {rust_out:?}");
}

#[test] fn foo_0_0() { compare_foo(0, 0); }
#[test] fn foo_1_0() { compare_foo(1, 0); }
#[test] fn foo_0_1() { compare_foo(0, 1); }
#[test] fn foo_1_1() { compare_foo(1, 1); }
#[test] fn foo_1_4() { compare_foo(1, 4); }
#[test] fn foo_3_2() { compare_foo(3, 2); }
#[test] fn foo_2_2() { compare_foo(2, 2); }
#[test] fn foo_3_3() { compare_foo(3, 3); }
#[test] fn foo_4_1() { compare_foo(4, 1); }
#[test] fn foo_5_5() { compare_foo(5, 5); }
#[test] fn foo_1_2() { compare_foo(1, 2); }
#[test] fn foo_2_1() { compare_foo(2, 1); }
#[test] fn foo_3_0() { compare_foo(3, 0); }
#[test] fn foo_0_3() { compare_foo(0, 3); }
#[test] fn foo_4_4() { compare_foo(4, 4); }
#[test] fn foo_10_1() { compare_foo(10, 1); }
#[test] fn foo_1_10() { compare_foo(1, 10); }
