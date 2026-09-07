//! Test scaffolding: loads the C and/or Rust `libSieve.so` through
//! `libloading` and invokes their exported `sieve` symbol, in the order given.
//!
//! Every differential comparison runs in one of these child processes rather
//! than in the test process. `sieve`'s only observable is what it writes to
//! libc `stdout`, and file descriptor 1 is process-global — capturing it inside
//! the test binary would race with the test harness's own progress output. A
//! child process gives each measurement a private, uncontended stdout.
//!
//! It is *not* a translated C driver (`c_src/CMakeLists.txt` builds no
//! executable) and it is symmetric by construction: which library serves a step
//! is decided solely by that step's prefix.
//!
//! Usage: `runner <c-so> <rust-so> <step>...`  where each step is
//!        `c:<value>` or `r:<value>`, and `<value>` is decimal (possibly
//!        negative) or a `0x`-prefixed 32-bit pattern.

use std::ffi::c_int;

fn parse_val(raw: &str) -> c_int {
    if let Some(hex) = raw.strip_prefix("0x") {
        u32::from_str_radix(hex, 16).expect("bad hex value") as i32 as c_int
    } else {
        raw.parse::<i64>().expect("bad value") as i32 as c_int
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: runner <c-so> <rust-so> <step>...   (step = c:VAL | r:VAL)");
        std::process::exit(2);
    }

    // Parse everything before touching stdout, so a malformed spec cannot be
    // mistaken for a divergence.
    let steps: Vec<(bool, c_int)> = args[3..]
        .iter()
        .map(|s| match s.split_once(':') {
            Some(("c", v)) => (false, parse_val(v)),
            Some(("r", v)) => (true, parse_val(v)),
            _ => panic!("bad step {s:?}; expected c:VAL or r:VAL"),
        })
        .collect();

    unsafe {
        let c_lib = libloading::Library::new(&args[1]).expect("dlopen C libSieve.so");
        let r_lib = libloading::Library::new(&args[2]).expect("dlopen Rust libSieve.so");
        let c_sym: libloading::Symbol<unsafe extern "C" fn(c_int)> =
            c_lib.get(b"sieve\0").expect("C .so must export `sieve`");
        let r_sym: libloading::Symbol<unsafe extern "C" fn(c_int)> =
            r_lib.get(b"sieve\0").expect("Rust .so must export `sieve`");
        let c_sieve = *c_sym;
        let r_sieve = *r_sym;
        assert_ne!(
            c_sieve as usize, r_sieve as usize,
            "the two `sieve` symbols resolved to the SAME address: the loader de-duplicated the \
             libraries and any comparison would be vacuous"
        );

        for (rust, v) in steps {
            if rust {
                r_sieve(v)
            } else {
                c_sieve(v)
            }
        }
        // The library writes through libc `printf`; this process may be killed
        // before exit handlers run, so flush explicitly.
        libc::fflush(std::ptr::null_mut());
    }
}
