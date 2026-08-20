//! Out-of-process probe used by the differential integration tests.
//!
//! It `dlopen`s an arbitrary shared library (either the C reference build or
//! the Rust `cdylib`) and invokes one of its exported C-ABI symbols. Running
//! this in a child process lets the tests
//!
//!   * drive the exported `main` with a real piped `stdin`/`stdout`, and
//!   * compare *crashes* (SIGSEGV from null/invalid pointers, stack overflow
//!     from the C VLAs) between the two libraries without taking the test
//!     runner down.
//!
//! Usage: `soprobe <library.so> <op> [args...]`
//!
//! The library is always loaded through `libloading`; no function from this
//! crate's own library is ever called directly.

use std::os::raw::c_int;
use std::process::ExitCode;

type FmaArrayFn =
    unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
type CallFmaFn = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
type MainFn = unsafe extern "C" fn() -> c_int;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: soprobe <library.so> <op> [args...]");
        return ExitCode::from(2);
    }
    let lib_path = &args[1];
    let op = args[2].as_str();

    let lib = unsafe { libloading::Library::new(lib_path) };
    let lib = match lib {
        Ok(l) => l,
        Err(e) => {
            eprintln!("dlopen({lib_path}) failed: {e}");
            return ExitCode::from(3);
        }
    };

    match op {
        // Call the exported `main`, which reads this process's stdin and
        // writes this process's stdout. Exit with whatever it returned so the
        // caller can compare the C `main`'s `return 0` too.
        "main" => {
            let f: libloading::Symbol<MainFn> = match unsafe { lib.get(b"main\0") } {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("dlsym(main) failed: {e}");
                    return ExitCode::from(4);
                }
            };
            let rc = unsafe { f() };
            // libc `exit` (reached through `process::exit`) flushes the C
            // stdio buffers the C `main`'s `printf` wrote into; the Rust
            // `main` flushes its own stdout before returning.
            std::process::exit(rc);
        }

        // call_fma(NULL, len) -- expected to fault for len > 0.
        "call_fma_null" => {
            let len: c_int = args[3].parse().unwrap();
            let f: libloading::Symbol<CallFmaFn> = unsafe { lib.get(b"call_fma\0").unwrap() };
            let v = unsafe { f(std::ptr::null(), len) };
            println!("{v}");
            ExitCode::SUCCESS
        }

        // call_fma(valid, len) with an arbitrary (possibly negative or huge)
        // len, printing the returned value.
        "call_fma" => {
            let len: c_int = args[3].parse().unwrap();
            let n: usize = args[4].parse().unwrap();
            let seed: u64 = args[5].parse().unwrap();
            let data: Vec<c_int> = (0..n).map(|i| (seed as c_int).wrapping_add(i as c_int)).collect();
            let f: libloading::Symbol<CallFmaFn> = unsafe { lib.get(b"call_fma\0").unwrap() };
            let ptr = if n == 0 { std::ptr::null() } else { data.as_ptr() };
            let v = unsafe { f(ptr, len) };
            println!("{v}");
            ExitCode::SUCCESS
        }

        // fma_array with a null `out` -- expected to fault for len > 0.
        "fma_array_null_out" => {
            let len: c_int = args[3].parse().unwrap();
            let n = if len > 0 { len as usize } else { 1 };
            let ones: Vec<c_int> = vec![1; n];
            let zeros: Vec<c_int> = vec![0; n];
            let f: libloading::Symbol<FmaArrayFn> = unsafe { lib.get(b"fma_array\0").unwrap() };
            unsafe { f(std::ptr::null_mut(), ones.as_ptr(), ones.as_ptr(), zeros.as_ptr(), len) };
            println!("ok");
            ExitCode::SUCCESS
        }

        // fma_array with null read-only inputs -- expected to fault for len > 0.
        // `which` selects mul1 / mul2 / add / all.
        "fma_array_null_in" => {
            let len: c_int = args[3].parse().unwrap();
            let which = args[4].as_str();
            let n = if len > 0 { len as usize } else { 1 };
            let mut out: Vec<c_int> = vec![0; n];
            let ones: Vec<c_int> = vec![1; n];
            let nul: *const c_int = std::ptr::null();
            let (m1, m2, ad) = match which {
                "mul1" => (nul, ones.as_ptr(), ones.as_ptr()),
                "mul2" => (ones.as_ptr(), nul, ones.as_ptr()),
                "add" => (ones.as_ptr(), ones.as_ptr(), nul),
                _ => (nul, nul, nul),
            };
            let f: libloading::Symbol<FmaArrayFn> = unsafe { lib.get(b"fma_array\0").unwrap() };
            unsafe { f(out.as_mut_ptr(), m1, m2, ad, len) };
            println!("ok");
            ExitCode::SUCCESS
        }

        // fma_array with *valid but too-small* buffers and an arbitrary `len`.
        // `bufn` elements are allocated, `len` is what gets passed; the C has
        // no bounds check, so len > bufn walks off the end exactly as the Rust
        // must.
        "fma_array_len" => {
            let bufn: usize = args[3].parse().unwrap();
            let len: c_int = args[4].parse().unwrap();
            let mut out: Vec<c_int> = vec![0; bufn.max(1)];
            let ones: Vec<c_int> = vec![1; bufn.max(1)];
            let zeros: Vec<c_int> = vec![0; bufn.max(1)];
            let f: libloading::Symbol<FmaArrayFn> = unsafe { lib.get(b"fma_array\0").unwrap() };
            unsafe {
                f(
                    out.as_mut_ptr(),
                    ones.as_ptr(),
                    ones.as_ptr(),
                    zeros.as_ptr(),
                    len,
                )
            };
            println!("ok");
            ExitCode::SUCCESS
        }

        // fma_array with every pointer null; safe iff len <= 0.
        "fma_array_all_null" => {
            let len: c_int = args[3].parse().unwrap();
            let f: libloading::Symbol<FmaArrayFn> = unsafe { lib.get(b"fma_array\0").unwrap() };
            unsafe {
                f(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    len,
                )
            };
            println!("ok");
            ExitCode::SUCCESS
        }

        // Just prove the library loads and every expected symbol resolves.
        "symbols" => {
            for name in [&b"fma_array\0"[..], &b"call_fma\0"[..], &b"main\0"[..]] {
                let r = unsafe { lib.get::<*const ()>(name) };
                let n = String::from_utf8_lossy(&name[..name.len() - 1]).to_string();
                match r {
                    Ok(_) => println!("{n}: ok"),
                    Err(e) => {
                        println!("{n}: MISSING ({e})");
                        return ExitCode::from(5);
                    }
                }
            }
            ExitCode::SUCCESS
        }

        other => {
            eprintln!("unknown op {other}");
            ExitCode::from(2)
        }
    }
}
