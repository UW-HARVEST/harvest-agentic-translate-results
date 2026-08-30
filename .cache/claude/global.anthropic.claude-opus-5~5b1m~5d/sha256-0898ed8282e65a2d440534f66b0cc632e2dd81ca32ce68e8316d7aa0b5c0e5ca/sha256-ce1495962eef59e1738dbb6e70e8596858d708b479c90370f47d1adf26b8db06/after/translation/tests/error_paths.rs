//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no explicit error
//! returns, so the rejection surface consists of (a) fatal pointer faults and
//! (b) silent truncation / saturation of out-of-contract values. Both classes
//! must behave identically in the Rust translation.

mod common;

use common::{assert_same, impls, Which};

#[repr(align(8))]
struct Aligned([u8; 16]);

// ---------------------------------------------------------------------------
// helper: run a fatal call in a fresh child process and report how it died
// ---------------------------------------------------------------------------

/// Result of a child process that called `print_foo` with a bad pointer.
#[derive(Debug, PartialEq, Eq)]
struct Death {
    signal: Option<i32>,
    code: Option<i32>,
}

fn run_faulting_child(which: Which, addr: usize) -> Death {
    spawn_child(which, &[("DIFF_CRASH_ADDR", addr.to_string())])
}

/// Spawns a child that places the `foo_t` image `straddle` bytes before an
/// unmapped page, so that only the first `straddle` bytes are readable.
fn run_page_edge_child(which: Which, straddle: usize) -> Death {
    spawn_child(which, &[("DIFF_CRASH_STRADDLE", straddle.to_string())])
}

fn spawn_child(which: Which, extra: &[(&str, String)]) -> Death {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = std::process::Command::new(exe);
    for (k, v) in extra {
        cmd.env(k, v);
    }
    let status = cmd
        .env(
            "DIFF_CRASH_LIB",
            match which {
                Which::C => "c",
                Which::Rust => "rust",
            },
        )
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn faulting child");
    Death {
        signal: status.signal(),
        code: status.code(),
    }
}

extern "C" {
    fn mmap(
        addr: *mut std::ffi::c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut std::ffi::c_void;
    fn mprotect(addr: *mut std::ffi::c_void, len: usize, prot: i32) -> i32;
}
const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const PAGE: usize = 4096;

/// Executed in the child process spawned by [`spawn_child`].
fn crash_child_main(which: &str) -> ! {
    let w = match which {
        "c" => Which::C,
        "rust" => Which::Rust,
        other => panic!("bad DIFF_CRASH_LIB={other}"),
    };
    let f = impls().print_foo(w);

    if let Ok(s) = std::env::var("DIFF_CRASH_STRADDLE") {
        // Two pages; the second is made unreadable. The image starts
        // `straddle` bytes before the unreadable page, so exactly `straddle`
        // bytes of it are accessible. This pins down the *read footprint*: the
        // C code touches byte 0 and bytes 4..8, so anything below 8 must fault
        // and exactly 8 must succeed — in both implementations.
        let straddle: usize = s.parse().expect("numeric DIFF_CRASH_STRADDLE");
        let base = unsafe {
            mmap(
                std::ptr::null_mut(),
                2 * PAGE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(base as isize > 0, "mmap failed");
        unsafe {
            std::ptr::write_bytes(base as *mut u8, 0x5A, 2 * PAGE);
            assert_eq!(mprotect((base as *mut u8).add(PAGE).cast(), PAGE, PROT_NONE), 0);
            f((base as *const u8).add(PAGE - straddle));
        }
        std::process::exit(0);
    }

    let addr: usize = std::env::var("DIFF_CRASH_ADDR")
        .expect("DIFF_CRASH_ADDR")
        .parse()
        .expect("numeric DIFF_CRASH_ADDR");
    unsafe { f(addr as *const u8) };
    // Reached only if the call did *not* fault.
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// row 1 — print_foo(NULL)
// ---------------------------------------------------------------------------
fn row01_print_foo_null_pointer() {
    let c = run_faulting_child(Which::C, 0);
    let r = run_faulting_child(Which::Rust, 0);
    assert_eq!(
        c, r,
        "print_foo(NULL): C and Rust must terminate the same way (C={c:?}, Rust={r:?})"
    );
    assert_eq!(c.signal, Some(11), "expected SIGSEGV from print_foo(NULL), got {c:?}");
}

// ---------------------------------------------------------------------------
// row 2 — print_foo(non-null unmapped pointer)
// ---------------------------------------------------------------------------
fn row02_print_foo_unmapped_pointer() {
    for addr in [0x1usize, 0x8, 0xdead_beef] {
        let c = run_faulting_child(Which::C, addr);
        let r = run_faulting_child(Which::Rust, addr);
        assert_eq!(
            c, r,
            "print_foo({addr:#x}): C and Rust must terminate the same way (C={c:?}, Rust={r:?})"
        );
        assert_eq!(c.signal, Some(11), "expected SIGSEGV for {addr:#x}, got {c:?}");
    }
}

// ---------------------------------------------------------------------------
// row 3 — misaligned pointer: no alignment check in C
// ---------------------------------------------------------------------------
fn row03_print_foo_misaligned() {
    let mut cases = Vec::new();
    for off in 1..=3usize {
        for bits in [0x00u8, 0x21, 0xFF] {
            cases.push(format!("print_foo(buf+{off}) bits={bits:#04x}"));
        }
    }
    assert_same("err03", &cases, |w| {
        let f = impls().print_foo(w);
        for off in 1..=3usize {
            for bits in [0x00u8, 0x21, 0xFF] {
                let mut buf = Aligned([0u8; 16]);
                buf.0[off] = bits;
                buf.0[off + 4] = 0x21;
                buf.0[off + 5] = 0x43;
                buf.0[off + 6] = 0x65;
                buf.0[off + 7] = 0x87;
                unsafe { f(buf.0.as_ptr().add(off)) };
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 4–5 — x out of the 2-bit range
// ---------------------------------------------------------------------------
fn row04_driver_x_out_of_range() {
    let xs: Vec<u32> = (4u32..=64).chain([255, 256, 1023, 0x1_0000, 0x8000_0000]).collect();
    let cases: Vec<String> = xs.iter().map(|x| format!("driver(x={x}, 0, 0, 0)")).collect();
    assert_same("err04", &cases, |w| {
        let f = impls().driver(w);
        for &x in &xs {
            unsafe { f(x, 0, 0, 0) };
        }
    });
}

fn row05_driver_x_uint_max() {
    let cases = vec!["driver(UINT_MAX, 0, 0, 0)".to_string()];
    assert_same("err05", &cases, |w| {
        let f = impls().driver(w);
        unsafe { f(u32::MAX, 0, 0, 0) };
    });
}

// ---------------------------------------------------------------------------
// rows 6–7 — y out of the 3-bit range
// ---------------------------------------------------------------------------
fn row06_driver_y_out_of_range() {
    let ys: Vec<u32> = (8u32..=64).chain([255, 256, 1023, 0x1_0000, 0x8000_0000]).collect();
    let cases: Vec<String> = ys.iter().map(|y| format!("driver(0, y={y}, 0, 0)")).collect();
    assert_same("err06", &cases, |w| {
        let f = impls().driver(w);
        for &y in &ys {
            unsafe { f(0, y, 0, 0) };
        }
    });
}

fn row07_driver_y_uint_max() {
    let cases = vec!["driver(0, UINT_MAX, 0, 0)".to_string()];
    assert_same("err07", &cases, |w| {
        let f = impls().driver(w);
        unsafe { f(0, u32::MAX, 0, 0) };
    });
}

// ---------------------------------------------------------------------------
// row 8 — non-canonical `_Bool` byte (the "out-of-range enum across FFI" case)
// ---------------------------------------------------------------------------
fn row08_driver_invalid_bool_byte() {
    let bs: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 0x0F, 0x10, 0x7E, 0x7F, 0x80, 0x81, 0xFE, 0xFF];
    let cases: Vec<String> = bs.iter().map(|b| format!("driver(0, 0, b={b:#04x}, 0)")).collect();
    assert_same("err08", &cases, |w| {
        let f = impls().driver(w);
        for &b in &bs {
            unsafe { f(0, 0, b, 0) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 9 — wide non-zero value in the bool argument register
// ---------------------------------------------------------------------------
fn row09_driver_wide_bool_register() {
    let bs: Vec<u32> = vec![0x100, 0x200, 0xFF00, 0xFFFF_FF00, 0xFFFF_FE00, 0x1234_5600];
    let cases: Vec<String> = bs
        .iter()
        .map(|b| format!("driver(0, 0, b_reg={b:#010x}, 0)"))
        .collect();
    assert_same("err09", &cases, |w| {
        let f = impls().driver_wide(w);
        for &b in &bs {
            unsafe { f(0, 0, b, 0) };
        }
    });
}

// ---------------------------------------------------------------------------
// rows 10–12 — z boundary values
// ---------------------------------------------------------------------------
fn row10_driver_z_int_min() {
    let cases = vec!["driver(0, 0, 0, INT_MIN)".to_string()];
    assert_same("err10", &cases, |w| {
        let f = impls().driver(w);
        unsafe { f(0, 0, 0, i32::MIN) };
    });
}

fn row11_driver_z_int_max() {
    let cases = vec!["driver(0, 0, 0, INT_MAX)".to_string()];
    assert_same("err11", &cases, |w| {
        let f = impls().driver(w);
        unsafe { f(0, 0, 0, i32::MAX) };
    });
}

fn row12_driver_z_minus_one() {
    let zs: Vec<i32> = vec![-1, -2, i32::MIN, i32::MIN + 1, i32::MAX, 0];
    let cases: Vec<String> = zs.iter().map(|z| format!("driver(0, 0, 0, z={z})")).collect();
    assert_same("err12", &cases, |w| {
        let f = impls().driver(w);
        for &z in &zs {
            unsafe { f(0, 0, 0, z) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 13 — padding bits 6..7 of the bit-field byte must be ignored
// ---------------------------------------------------------------------------
fn row13_print_foo_padding_bits_set() {
    let mut cases = Vec::new();
    for low in 0u8..64 {
        for high in [0x00u8, 0x40, 0x80, 0xC0] {
            cases.push(format!("print_foo(bits={:#04x})", low | high));
        }
    }
    assert_same("err13", &cases, |w| {
        let f = impls().print_foo(w);
        for low in 0u8..64 {
            for high in [0x00u8, 0x40, 0x80, 0xC0] {
                let buf = Aligned([low | high, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
                unsafe { f(buf.0.as_ptr()) };
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 14 — inter-field padding bytes 1..3 must be ignored
// ---------------------------------------------------------------------------
fn row14_print_foo_padding_bytes_garbage() {
    let pads: Vec<[u8; 3]> = vec![
        [0, 0, 0],
        [0xFF, 0xFF, 0xFF],
        [0x01, 0x02, 0x03],
        [0x80, 0x00, 0x7F],
        [0xAA, 0x55, 0xAA],
    ];
    let cases: Vec<String> = pads
        .iter()
        .map(|p| format!("print_foo(bits=0x2A, pad={p:02x?}, z=-1)"))
        .collect();
    assert_same("err14", &cases, |w| {
        let f = impls().print_foo(w);
        for p in &pads {
            let buf = Aligned([
                0x2A, p[0], p[1], p[2], 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            unsafe { f(buf.0.as_ptr()) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 15 — every out-of-contract input at once
// ---------------------------------------------------------------------------
fn row15_driver_all_invalid_at_once() {
    let cases = vec!["driver(UINT_MAX, UINT_MAX, 0xFF, INT_MIN)".to_string()];
    assert_same("err15", &cases, |w| {
        let f = impls().driver(w);
        unsafe { f(u32::MAX, u32::MAX, 0xFF, i32::MIN) };
    });

    // and the documented expected text, to pin the C behaviour down explicitly
    let _g = common::stdout_guard();
    let out = common::capture(|| {
        let f = impls().driver(Which::C);
        unsafe { f(u32::MAX, u32::MAX, 0xFF, i32::MIN) };
    });
    drop(_g);
    assert_eq!(String::from_utf8_lossy(&out), "3 7 1 -2147483648\n");
}

// ---------------------------------------------------------------------------
// rows 16–17 — read footprint at a page boundary
// ---------------------------------------------------------------------------
fn row16_print_foo_exactly_eight_readable_bytes() {
    let c = run_page_edge_child(Which::C, 8);
    let r = run_page_edge_child(Which::Rust, 8);
    assert_eq!(
        c, r,
        "8 readable bytes must suffice for both implementations (C={c:?}, Rust={r:?})"
    );
    assert_eq!(c.code, Some(0), "expected clean exit with 8 readable bytes, got {c:?}");
}

fn row17_print_foo_truncated_object_faults_identically() {
    for straddle in 1..=7usize {
        let c = run_page_edge_child(Which::C, straddle);
        let r = run_page_edge_child(Which::Rust, straddle);
        assert_eq!(
            c, r,
            "{straddle} readable byte(s): C and Rust must behave identically (C={c:?}, Rust={r:?})"
        );
        assert_eq!(
            c.signal,
            Some(11),
            "expected SIGSEGV with only {straddle} readable bytes, got {c:?}"
        );
    }
}

fn main() {
    if let Ok(which) = std::env::var("DIFF_CRASH_LIB") {
        crash_child_main(&which);
    }
    common::run_tests(&[
        ("row01_print_foo_null_pointer", row01_print_foo_null_pointer),
        ("row02_print_foo_unmapped_pointer", row02_print_foo_unmapped_pointer),
        ("row03_print_foo_misaligned", row03_print_foo_misaligned),
        ("row04_driver_x_out_of_range", row04_driver_x_out_of_range),
        ("row05_driver_x_uint_max", row05_driver_x_uint_max),
        ("row06_driver_y_out_of_range", row06_driver_y_out_of_range),
        ("row07_driver_y_uint_max", row07_driver_y_uint_max),
        ("row08_driver_invalid_bool_byte", row08_driver_invalid_bool_byte),
        ("row09_driver_wide_bool_register", row09_driver_wide_bool_register),
        ("row10_driver_z_int_min", row10_driver_z_int_min),
        ("row11_driver_z_int_max", row11_driver_z_int_max),
        ("row12_driver_z_minus_one", row12_driver_z_minus_one),
        ("row13_print_foo_padding_bits_set", row13_print_foo_padding_bits_set),
        ("row14_print_foo_padding_bytes_garbage", row14_print_foo_padding_bytes_garbage),
        ("row15_driver_all_invalid_at_once", row15_driver_all_invalid_at_once),
        (
            "row16_print_foo_exactly_eight_readable_bytes",
            row16_print_foo_exactly_eight_readable_bytes,
        ),
        (
            "row17_print_foo_truncated_object_faults_identically",
            row17_print_foo_truncated_object_faults_identically,
        ),
    ]);
}
