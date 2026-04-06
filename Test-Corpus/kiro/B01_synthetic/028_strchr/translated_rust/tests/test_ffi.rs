use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("Failed to load C library") }
}

fn rust_lib() -> Library {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    path.push(profile);
    path.push("libdriver.so");
    unsafe { Library::new(&path).expect("Failed to load Rust library") }
}

#[test]
fn test_foo() {
    let c = c_lib();
    let r = rust_lib();
    let c_foo: Symbol<unsafe extern "C" fn(*const c_char, c_char) -> c_int> =
        unsafe { c.get(b"foo").unwrap() };
    let r_foo: Symbol<unsafe extern "C" fn(*const c_char, c_char) -> c_int> =
        unsafe { r.get(b"foo").unwrap() };

    let cases: &[(&str, u8)] = &[
        ("Hello World", b'l'),
        ("Hello World", b'o'),
        ("Hello World", b'z'),
        ("", b'a'),
        ("AAAA", b'A'),
        ("aAbAcA", b'A'),
        ("test string with x chars xxxx", b'x'),
        ("no match here", b'Z'),
        ("AxAxAx", b'A'),
        ("AxAxAx", b'x'),
    ];

    for (s, ch) in cases {
        let cs = CString::new(*s).unwrap();
        let c_result = unsafe { c_foo(cs.as_ptr(), *ch as c_char) };
        let r_result = unsafe { r_foo(cs.as_ptr(), *ch as c_char) };
        assert_eq!(c_result, r_result, "foo mismatch for input={:?} c={:?}", s, *ch as char);
    }
}

#[test]
fn test_driver_via_subprocess() {
    // Test driver output by running both C and Rust binaries with same input
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_bin = format!("{}/c_src/build/driver", manifest);
    let r_bin = format!("{}/target/debug/driver", manifest);

    let inputs = &[
        "Hello World",
        "AAAA xxxx",
        "No special chars",
        "AxAxAx",
    ];

    for input in inputs {
        let c_out = std::process::Command::new(&c_bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).ok();
                child.wait_with_output()
            })
            .expect("C binary failed");

        let r_out = std::process::Command::new(&r_bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).ok();
                child.wait_with_output()
            })
            .expect("Rust binary failed");

        assert_eq!(c_out.stdout, r_out.stdout,
            "driver output mismatch for input={:?}\nC:    {:?}\nRust: {:?}",
            input,
            String::from_utf8_lossy(&c_out.stdout),
            String::from_utf8_lossy(&r_out.stdout));
    }
}
