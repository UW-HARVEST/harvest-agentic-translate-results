//! `harness = false` helper *executable* used by the error-path differential
//! tests (`tests/error_paths.rs`).
//!
//! It loads ONE of the two shared libraries (given by argv) via `libloading`
//! and performs a single call into it. This makes fatal outcomes observable and
//! comparable: `exit(30)` on allocation failure, `SIGSEGV` on NULL arguments.
//! Running these in-process would kill the test harness.
//!
//! Invoked with no arguments (which is what `cargo test` itself does) it is a
//! successful no-op.
//!
//! Usage:
//!   child create        <so> <path_hex> <dir_hex> <suffix_len>
//!   child null_path     <so> <dir_hex>  <suffix_len>
//!   child null_dir      <so> <path_hex> <suffix_len>
//!   child extract_null  <so> <separator_u8>
//!
//! On success the raw result bytes (`alloc_size` of them) are printed to stdout
//! as lowercase hex followed by a newline.

use std::ffi::{c_char, c_void};

unsafe extern "C" {
    fn free(p: *mut c_void);
}

type ExtractFilenameFn = unsafe extern "C" fn(*const c_char, c_char) -> *const c_char;
type CreateFilenameFn = unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char;

fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd hex length");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("bad hex"))
        .collect()
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        // Plain `cargo test` run of this target: nothing to do.
        println!("child helper: ok (no arguments)");
        return;
    }

    let mode = args[1].as_str();
    let so = &args[2];
    let lib = unsafe { libloading::Library::new(so).expect("dlopen") };

    unsafe {
        match mode {
            "create" | "null_path" | "null_dir" => {
                let create: libloading::Symbol<CreateFilenameFn> = lib
                    .get(b"FIO_createFilename_fromOutDir\0")
                    .expect("missing FIO_createFilename_fromOutDir");
                let create = *create;

                match mode {
                    "create" => {
                        let mut path = unhex(&args[3]);
                        path.push(0);
                        let mut dir = unhex(&args[4]);
                        dir.push(0);
                        let suffix: usize = args[5].parse().expect("suffix_len");

                        let flen = {
                            let p = &path[..path.len() - 1];
                            match p.iter().rposition(|&b| b == b'/') {
                                Some(i) => p.len() - i - 1,
                                None => p.len(),
                            }
                        };
                        let size = (dir.len() - 1)
                            .wrapping_add(1)
                            .wrapping_add(flen)
                            .wrapping_add(suffix)
                            .wrapping_add(1);

                        let r = create(
                            path.as_ptr() as *const c_char,
                            dir.as_ptr() as *const c_char,
                            suffix,
                        );
                        if r.is_null() {
                            eprintln!("child: returned NULL");
                            std::process::exit(2);
                        }
                        // Bound the amount we read/print: `size` can be enormous
                        // when `suffixLen` is huge but `calloc` still succeeds.
                        let head = size.min(4096);
                        let out = std::slice::from_raw_parts(r as *const u8, head).to_vec();
                        free(r as *mut c_void);
                        println!("size={size} head={}", hex(&out));
                    }
                    "null_path" => {
                        let mut dir = unhex(&args[3]);
                        dir.push(0);
                        let suffix: usize = args[4].parse().expect("suffix_len");
                        let r = create(std::ptr::null(), dir.as_ptr() as *const c_char, suffix);
                        println!("unexpectedly survived: {:p}", r);
                    }
                    "null_dir" => {
                        let mut path = unhex(&args[3]);
                        path.push(0);
                        let suffix: usize = args[4].parse().expect("suffix_len");
                        let r = create(path.as_ptr() as *const c_char, std::ptr::null(), suffix);
                        println!("unexpectedly survived: {:p}", r);
                    }
                    _ => unreachable!(),
                }
            }
            "extract_null" => {
                let extract: libloading::Symbol<ExtractFilenameFn> = lib
                    .get(b"extractFilename\0")
                    .expect("missing extractFilename");
                let extract = *extract;
                let sep: u8 = args[3].parse().expect("separator");
                let r = extract(std::ptr::null(), sep as c_char);
                println!("unexpectedly survived: {:p}", r);
            }
            other => {
                eprintln!("child: unknown mode {other}");
                std::process::exit(3);
            }
        }
    }
}
