// Differential test helper.
//
// Loads one or two shared objects via `libloading` and calls their exported
// `driver` symbol for every IEEE-754 bit pattern read from stdin (one 16-digit
// hex value per line).  When two libraries are given, the call for each value is
// interleaved: `lib1(v)` then `lib2(v)`.
//
// This runs as a *separate process* so the captured stdout contains nothing but
// the library's own output — the `cargo test` harness writes its progress lines
// to the parent's fd 1, which would otherwise contaminate any in-process
// redirection of stdout.

use std::io::{BufRead, Write};

type DriverFn = unsafe extern "C" fn(f64);

extern "C" {
    fn setlocale(category: i32, locale: *const std::ffi::c_char) -> *mut std::ffi::c_char;
    fn fesetround(mode: i32) -> i32;
}

// glibc `locale.h`: LC_ALL == 6 on Linux.
const LC_ALL: i32 = 6;

// glibc `bits/fenv.h` on x86/x86_64.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const FE_MODES: [(&str, i32); 4] = [
    ("nearest", 0x000),
    ("downward", 0x400),
    ("upward", 0x800),
    ("towardzero", 0xc00),
];
#[cfg(target_arch = "aarch64")]
const FE_MODES: [(&str, i32); 4] = [
    ("nearest", 0x000000),
    ("upward", 0x400000),
    ("downward", 0x800000),
    ("towardzero", 0xc00000),
];

/// Apply the ambient process state the caller asked for, *before* any library
/// call, exactly as a real consumer of the C library would.
fn apply_ambient_state() {
    if let Ok(loc) = std::env::var("DRIVER_DUMP_LOCALE") {
        let c = std::ffi::CString::new(loc.clone()).expect("locale name");
        let r = unsafe { setlocale(LC_ALL, c.as_ptr()) };
        assert!(!r.is_null(), "setlocale(LC_ALL, {loc:?}) failed");
    }
    if let Ok(mode) = std::env::var("DRIVER_DUMP_ROUND") {
        let &(_, v) = FE_MODES
            .iter()
            .find(|(n, _)| *n == mode)
            .unwrap_or_else(|| panic!("unknown rounding mode {mode:?}"));
        let r = unsafe { fesetround(v) };
        assert_eq!(r, 0, "fesetround({mode}) failed");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert!(
        args.len() == 1 || args.len() == 2,
        "usage: driver_dump <lib1.so> [lib2.so]   (bit patterns on stdin)"
    );

    apply_ambient_state();

    // Keep the `Library` values alive for the whole run.
    let libs: Vec<libloading::Library> = args
        .iter()
        .map(|p| unsafe { libloading::Library::new(p) }.expect("dlopen failed"))
        .collect();
    let funcs: Vec<DriverFn> = libs
        .iter()
        .map(|l| unsafe { *l.get::<DriverFn>(b"driver\0").expect("no `driver` symbol") })
        .collect();

    let stdin = std::io::stdin();
    let mut values: Vec<f64> = Vec::new();
    for line in stdin.lock().lines() {
        let line = line.expect("read stdin");
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let bits = u64::from_str_radix(t, 16).expect("bad hex bit pattern");
        values.push(f64::from_bits(bits));
    }

    for v in values {
        for f in &funcs {
            unsafe { f(v) };
        }
    }

    // glibc flushes `stdout` at exit; make sure nothing of ours is pending.
    std::io::stdout().flush().ok();
}
