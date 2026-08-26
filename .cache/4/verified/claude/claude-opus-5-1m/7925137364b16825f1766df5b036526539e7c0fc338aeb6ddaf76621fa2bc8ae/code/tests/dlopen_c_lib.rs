// CONFIGS.md row C24 — differential test that reaches the C ground truth through
// a real `dlopen`/`dlsym` FFI boundary using `libloading`.
//
// `c_src/` is never modified: the same `c_src/src/main.c` is compiled a *second*
// time into a position-independent shared object under `target/`, its exported
// `main` symbol is resolved with libloading and invoked with stdin/stdout
// redirected to temporary files.  Its output is compared against the Rust
// artifact, which is still driven the only way an external caller can drive an
// executable — as a child process.
//
// This whole file contains a single #[test] because it temporarily rewires the
// process-wide descriptors 0 and 1.

mod common;

use common::*;
use std::ffi::{c_int, c_void, CString};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn freopen(path: *const i8, mode: *const i8, stream: *mut c_void) -> *mut c_void;
    static mut stdin: *mut c_void;
    static mut stdout: *mut c_void;
}

fn build_shared_object() -> Option<PathBuf> {
    let md = manifest_dir();
    let out_dir = md.join("target/c_build");
    std::fs::create_dir_all(&out_dir).ok()?;
    let so = out_dir.join("libcdriver.so");
    let status = Command::new("cc")
        .arg("-shared")
        .arg("-fPIC")
        .arg("-o")
        .arg(&so)
        .arg(md.join("c_src/src/main.c"))
        .status()
        .ok()?;
    if !status.success() || !so.exists() {
        return None;
    }
    Some(so)
}

/// Calls the C `main` from a freshly loaded copy of the shared object.
///
/// A fresh copy (unique file name) is loaded for every case so that the
/// `static int y = 123;` in the library's data segment starts from its
/// initial value, exactly like a fresh process would.
fn call_c_main_via_dlopen(template_so: &Path, case: usize, input: &[u8]) -> (c_int, Vec<u8>) {
    let dir = template_so.parent().unwrap();
    let so = dir.join(format!("libcdriver_case{case}.so"));
    std::fs::copy(template_so, &so).expect("copy shared object");

    let in_path = dir.join(format!("dlopen_in_{case}"));
    let out_path = dir.join(format!("dlopen_out_{case}"));
    std::fs::write(&in_path, input).expect("write dlopen stdin file");
    let out_file = std::fs::File::create(&out_path).expect("create dlopen stdout file");

    let lib = unsafe { libloading::Library::new(&so) }.expect("dlopen the C shared object");
    let c_main: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { lib.get(b"main\0") }.expect("dlsym `main`");

    let in_c = CString::new(in_path.to_str().unwrap()).unwrap();
    let mode_r = CString::new("r").unwrap();

    let rc = unsafe {
        let saved_out = dup(1);
        assert!(saved_out >= 0, "dup(1) failed");

        // Fresh stdin stream state (clears EOF/error flags and the buffer).
        let s = freopen(in_c.as_ptr(), mode_r.as_ptr(), stdin);
        assert!(!s.is_null(), "freopen(stdin) failed");

        // stdout: keep glibc's FILE, just point descriptor 1 at the temp file.
        assert!(dup2(out_file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        let rc = c_main();

        fflush(stdout);
        assert!(dup2(saved_out, 1) >= 0, "restoring fd 1 failed");
        close(saved_out);
        rc
    };

    drop(c_main);
    drop(lib);
    let bytes = std::fs::read(&out_path).expect("read dlopen stdout file");
    std::fs::remove_file(&in_path).ok();
    std::fs::remove_file(&out_path).ok();
    std::fs::remove_file(&so).ok();
    (rc, bytes)
}

#[test]
fn c24_dlopen_c_main_matches_rust_binary() {
    let Some(so) = build_shared_object() else {
        eprintln!("cc unavailable — skipping the dlopen differential test");
        return;
    };

    let mut cases: Vec<String> = vec![
        "1 2 3".into(),
        "0 2 3".into(),
        "1 5 3".into(),
        "1 2 9".into(),
        "1 2".into(),
        "1".into(),
        "".into(),
        "   \n\t ".into(),
        "abc".into(),
        "-".into(),
        "+1 +2 +3".into(),
        "0000000001 000002 0003".into(),
        "4294967297 4294967298 4294967299".into(),
        "2147483648 2 3".into(),
        "99999999999999999999 2 3".into(),
        "1\n2\n3\n".into(),
        "1 2 3 trailing junk".into(),
        "1.5 2.5 3.5".into(),
        "0x10 2 3".into(),
    ];
    let mut rng = Rng::new(0xC24);
    for _ in 0..40 {
        cases.push(format!("{} {} {}", rng.next_i32(), rng.next_i32(), rng.next_i32()));
    }

    for (i, input) in cases.iter().enumerate() {
        let (rc, c_out) = call_c_main_via_dlopen(&so, i, input.as_bytes());
        let rust = run(Path::new(RUST_BIN), input.as_bytes());

        assert_eq!(
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust.stdout),
            "C24 stdout mismatch (dlopen'd C `main` vs Rust binary) for stdin {input:?}"
        );
        assert_eq!(
            rc,
            rust.code.unwrap_or(-1),
            "C24 return value mismatch for stdin {input:?}"
        );
        // ...and the dlopen'd C `main` must agree with the C *process* too.
        let c_proc = run(&c_bin(), input.as_bytes());
        assert_eq!(
            c_out, c_proc.stdout,
            "C24 dlopen'd C `main` disagrees with the C process for stdin {input:?}"
        );
    }
}
