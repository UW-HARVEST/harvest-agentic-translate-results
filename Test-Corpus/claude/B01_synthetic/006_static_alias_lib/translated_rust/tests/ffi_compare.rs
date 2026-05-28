// Integration test that loads both the C and Rust shared libraries
// via libloading and compares the outputs of the exported FFI symbols.
//
// Because each library has its own private static `inner` (in
// `static_alias`), and that state would persist across tests within
// the same process, we run each comparison in a freshly-forked child
// process so each library starts with `inner == 1`.

use libloading::{Library, Symbol};
use std::ffi::c_int;

fn rust_so_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    for c in &["target/debug/libStaticAlias.so", "target/release/libStaticAlias.so"] {
        let p = std::path::Path::new(&manifest).join(c);
        if p.exists() {
            return p;
        }
    }
    panic!("could not locate Rust libStaticAlias.so under target/");
}

fn c_so_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::Path::new(&manifest).join("c_src/build/libStaticAlias.so")
}

type StaticAliasFn = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
type DriverFn = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn fork() -> i32;
    fn _exit(code: i32) -> !;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn dup2(a: i32, b: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn read(fd: i32, buf: *mut u8, n: usize) -> isize;
    fn write(fd: i32, buf: *const u8, n: usize) -> isize;
    fn fflush(stream: *mut core::ffi::c_void) -> i32;
}

fn drain(fd: i32) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = unsafe { read(fd, tmp.as_mut_ptr(), tmp.len()) };
        if n <= 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n as usize]);
    }
    buf
}

fn run_driver_in_child(so_path: &std::path::Path, initial: c_int, iters: c_int) -> Vec<u8> {
    let mut fds = [0i32; 2];
    unsafe {
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let pid = fork();
        if pid == 0 {
            // Child: redirect stdout to the pipe write end.
            dup2(fds[1], 1);
            close(fds[0]);
            close(fds[1]);
            let lib = Library::new(so_path).expect("child dlopen");
            let dr: Symbol<DriverFn> = lib.get(b"driver\0").expect("child driver symbol");
            (dr)(initial, iters);
            fflush(core::ptr::null_mut());
            _exit(0);
        } else {
            close(fds[1]);
            let buf = drain(fds[0]);
            close(fds[0]);
            let mut status = 0i32;
            waitpid(pid, &mut status, 0);
            buf
        }
    }
}

fn run_static_alias_sequence_in_child(
    so_path: &std::path::Path,
    inputs: &[c_int],
) -> Vec<(c_int, bool, c_int)> {
    let mut fds = [0i32; 2];
    unsafe {
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let pid = fork();
        if pid == 0 {
            close(fds[0]);
            let lib = Library::new(so_path).expect("child dlopen");
            let sa: Symbol<StaticAliasFn> =
                lib.get(b"static_alias\0").expect("child static_alias symbol");
            let mut ser: Vec<u8> = Vec::new();
            for &v in inputs {
                let mut outer: c_int = v;
                let outer_ptr: *mut c_int = &mut outer;
                let ret = (sa)(outer_ptr);
                let aliases: i32 = if ret == outer_ptr { 1 } else { 0 };
                let ret_val = *ret;
                ser.extend_from_slice(&outer.to_le_bytes());
                ser.extend_from_slice(&aliases.to_le_bytes());
                ser.extend_from_slice(&ret_val.to_le_bytes());
            }
            let mut written = 0usize;
            while written < ser.len() {
                let n = write(fds[1], ser.as_ptr().add(written), ser.len() - written);
                if n <= 0 {
                    break;
                }
                written += n as usize;
            }
            close(fds[1]);
            _exit(0);
        } else {
            close(fds[1]);
            let buf = drain(fds[0]);
            close(fds[0]);
            let mut status = 0i32;
            waitpid(pid, &mut status, 0);
            let mut out = Vec::new();
            let mut i = 0;
            while i + 12 <= buf.len() {
                let a = i32::from_le_bytes(buf[i..i + 4].try_into().unwrap());
                let b = i32::from_le_bytes(buf[i + 4..i + 8].try_into().unwrap());
                let c = i32::from_le_bytes(buf[i + 8..i + 12].try_into().unwrap());
                out.push((a, b != 0, c));
                i += 12;
            }
            out
        }
    }
}

#[test]
fn static_alias_matches_c() {
    // Sequences chosen to exercise both branches of static_alias and
    // various boundary conditions.
    //
    //   inner starts at 1.
    //   if (*outer >= inner) { inner += *outer; return &inner; }
    //   else                 { *outer += inner;  return outer;  }
    let sequences: Vec<Vec<c_int>> = vec![
        vec![0, 2, 1, 10, -5, 20, 0, 100, 50, -1000],
        vec![1, 1, 1, 1, 1],
        vec![0, 0, 0, 0],
        vec![-100, -100, -100, 1000, 0, 0, 0],
        vec![5, 4, 3, 2, 1, 0, -1, -2, -3],
        vec![i32::MAX / 2, 1, 1, 1, i32::MIN / 2],
        vec![1],
        vec![],
    ];

    for seq in &sequences {
        let c_out = run_static_alias_sequence_in_child(&c_so_path(), seq);
        let r_out = run_static_alias_sequence_in_child(&rust_so_path(), seq);
        assert_eq!(
            c_out, r_out,
            "static_alias sequence {:?} differs:\n  C: {:?}\n  R: {:?}",
            seq, c_out, r_out
        );
    }
}

#[test]
fn driver_matches_c() {
    for &(initial, iters) in &[
        (0, 0),
        (0, 1),
        (0, 5),
        (1, 10),
        (100, 3),
        (-50, 8),
        (5, 1),
        (-1, 4),
        (1000, 6),
    ] {
        let c_out = run_driver_in_child(&c_so_path(), initial, iters);
        let r_out = run_driver_in_child(&rust_so_path(), initial, iters);
        assert_eq!(
            c_out,
            r_out,
            "driver({}, {}) stdout differs:\n  C: {:?}\n  R: {:?}",
            initial,
            iters,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
