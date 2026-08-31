//! Differential tests: every call goes through `libloading` into either the C
//! `libdriver.so` or the Rust `libdriver.so`, and the bytes the two write to
//! stdout must be identical.
//!
//! Ordering follows the call hierarchy in `c_src/include/driver.h` /
//! `c_src/src/driver.c`:
//!
//!   * `print_foo(const foo_t *)` -- leaf, does the formatting
//!   * `driver(unsigned, unsigned, bool, int)` -- builds a `foo_t`, calls
//!     `print_foo`
//!
//! Both are non-`static` in the C source and therefore part of the shared
//! object's interface; neither is ever invoked as a plain Rust function here.

mod harness;

use harness::{Libs, capture_stdout};
use std::ffi::{c_int, c_uint};

/// Mirror of the storage the C compiler allocates for
///
/// ```c
/// typedef struct {
///     unsigned int x : 2;
///     unsigned int y : 3;
///     bool b : 1;
///     int z;
/// } foo_t;
/// ```
///
/// Only used as an opaque 8-byte parcel: the same bytes are handed to both
/// libraries, so any disagreement about which bit means what shows up as a
/// mismatch in the captured output rather than being hidden by this
/// declaration.
#[repr(C)]
#[derive(Clone, Copy)]
struct FooRaw {
    bits: u32,
    z: c_int,
}

type PrintFoo = unsafe extern "C" fn(*const FooRaw);
type Driver = unsafe extern "C" fn(c_uint, c_uint, u8, c_int);

/// `sizeof(foo_t)` / `_Alignof(foo_t)` as produced by the platform C compiler,
/// measured independently of the Rust declaration above.
fn c_struct_layout() -> Option<(usize, usize)> {
    let dir = std::env::temp_dir().join(format!("driver-layout-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let src = dir.join("layout.c");
    let bin = dir.join("layout");
    std::fs::write(
        &src,
        r#"#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
typedef struct {
    unsigned int x : 2;
    unsigned int y : 3;
    bool b : 1;
    int z;
} foo_t;
int main(void) {
    printf("%zu %zu %zu\n", sizeof(foo_t), _Alignof(foo_t), offsetof(foo_t, z));
    return 0;
}
"#,
    )
    .ok()?;
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let ok = std::process::Command::new(cc)
        .arg(&src)
        .arg("-o")
        .arg(&bin)
        .status()
        .ok()?
        .success();
    if !ok {
        return None;
    }
    let out = std::process::Command::new(&bin).output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let mut it = text.split_whitespace();
    let size = it.next()?.parse().ok()?;
    let align = it.next()?.parse().ok()?;
    let off_z: usize = it.next()?.parse().ok()?;
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(off_z, 4, "offsetof(foo_t, z)");
    Some((size, align))
}

/// Small deterministic PRNG so failures are reproducible.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }
}

const Z_CASES: &[c_int] = &[
    0,
    1,
    -1,
    2,
    -2,
    7,
    -7,
    42,
    -42,
    255,
    -255,
    256,
    65535,
    -65536,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    0x5555_5555u32 as c_int,
    0xAAAA_AAAAu32 as c_int,
];

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// The Rust `foo_t` must occupy the same storage as the C one, otherwise the
/// pointer handed to `print_foo` would not describe the same object.
#[test]
fn foo_t_layout_matches_c() {
    match c_struct_layout() {
        Some((size, align)) => {
            assert_eq!(size, size_of::<FooRaw>(), "sizeof(foo_t)");
            assert_eq!(align, align_of::<FooRaw>(), "_Alignof(foo_t)");
        }
        None => eprintln!("skipping layout check: no working C compiler found"),
    }
}

// ---------------------------------------------------------------------------
// print_foo -- the leaf function
// ---------------------------------------------------------------------------

fn print_foo_pair(libs: &Libs) -> (libloading::Symbol<'_, PrintFoo>, libloading::Symbol<'_, PrintFoo>)
{
    unsafe {
        (
            libs.c.get(b"print_foo\0").expect("C print_foo"),
            libs.rust.get(b"print_foo\0").expect("Rust print_foo"),
        )
    }
}

fn check_print_foo(c: &PrintFoo, r: &PrintFoo, foo: &FooRaw) {
    let out_c = capture_stdout(|| unsafe { c(foo) });
    let out_r = capture_stdout(|| unsafe { r(foo) });
    assert_eq!(
        out_c,
        out_r,
        "print_foo(bits=0x{:08x}, z={}) mismatch\n  C   : {:?}\n  Rust: {:?}",
        foo.bits,
        foo.z,
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r),
    );
}

/// Sweeps every value of the byte that holds all three bit-fields (plus the
/// two padding bits above them) against a spread of `z` values.
#[test]
fn print_foo_low_byte_sweep() {
    let libs = Libs::load();
    let (c, r) = print_foo_pair(&libs);
    for bits in 0u32..256 {
        for &z in Z_CASES {
            check_print_foo(&c, &r, &FooRaw { bits, z });
        }
    }
}

/// Also drives the padding bits of the storage unit above bit 7, which the C
/// accessors must ignore.
#[test]
fn print_foo_full_storage_unit() {
    let libs = Libs::load();
    let (c, r) = print_foo_pair(&libs);
    let interesting = [
        0x0000_0000u32,
        0xFFFF_FFFF,
        0xFFFF_FF00,
        0x0000_00FF,
        0x5555_5555,
        0xAAAA_AAAA,
        0x8000_0000,
        0x0000_0100,
        0x1234_5678,
        0xDEAD_BEEF,
    ];
    for base in interesting {
        for low in 0u32..64 {
            let bits = (base & !0x3F) | low;
            for &z in &[0, -1, i32::MIN, i32::MAX, 12345] {
                check_print_foo(&c, &r, &FooRaw { bits, z });
            }
        }
    }
}

#[test]
fn print_foo_random() {
    let libs = Libs::load();
    let (c, r) = print_foo_pair(&libs);
    let mut rng = Rng::new(0xC0FF_EE12_3456_789A);
    for _ in 0..4000 {
        let foo = FooRaw {
            bits: rng.next_u32(),
            z: rng.next_u32() as c_int,
        };
        check_print_foo(&c, &r, &foo);
    }
}

// ---------------------------------------------------------------------------
// driver -- the public entry point from driver.h
// ---------------------------------------------------------------------------

fn driver_pair(libs: &Libs) -> (libloading::Symbol<'_, Driver>, libloading::Symbol<'_, Driver>) {
    unsafe {
        (
            libs.c.get(b"driver\0").expect("C driver"),
            libs.rust.get(b"driver\0").expect("Rust driver"),
        )
    }
}

fn check_driver(c: &Driver, r: &Driver, x: c_uint, y: c_uint, b: u8, z: c_int) {
    let out_c = capture_stdout(|| unsafe { c(x, y, b, z) });
    let out_r = capture_stdout(|| unsafe { r(x, y, b, z) });
    assert_eq!(
        out_c,
        out_r,
        "driver({x}, {y}, {b}, {z}) mismatch\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r),
    );
}

/// Every `x` in 0..8 and `y` in 0..16 exercises the 2- and 3-bit truncation
/// both inside and beyond the representable range.
#[test]
fn driver_small_exhaustive() {
    let libs = Libs::load();
    let (c, r) = driver_pair(&libs);
    for x in 0u32..8 {
        for y in 0u32..16 {
            for b in [0u8, 1] {
                for &z in &[0, -1, 1, i32::MIN, i32::MAX] {
                    check_driver(&c, &r, x, y, b, z);
                }
            }
        }
    }
}

/// Wide `x`/`y` values: `unsigned int` arguments far outside the bit-field
/// width, including ones whose low bits differ from their value.
#[test]
fn driver_wide_unsigned_args() {
    let libs = Libs::load();
    let (c, r) = driver_pair(&libs);
    let wide = [
        0u32,
        1,
        3,
        4,
        7,
        8,
        31,
        32,
        255,
        256,
        0x5555_5555,
        0xAAAA_AAAA,
        0xFFFF_FFFF,
        0xFFFF_FFFE,
        0x8000_0000,
        u32::MAX - 3,
    ];
    for &x in &wide {
        for &y in &wide {
            for b in [0u8, 1] {
                for &z in &[0, -12345, 0x7FFF_FFFF] {
                    check_driver(&c, &r, x, y, b, z);
                }
            }
        }
    }
}

/// `z` is a plain `int` and must survive untruncated.
#[test]
fn driver_z_cases() {
    let libs = Libs::load();
    let (c, r) = driver_pair(&libs);
    for &z in Z_CASES {
        for b in [0u8, 1] {
            check_driver(&c, &r, 1, 2, b, z);
        }
    }
}

/// The third parameter is a C `_Bool`. Passing a byte other than 0 or 1 is not
/// something a conforming C caller can do, but it is observable through the
/// ABI, so pin down that both libraries agree on whatever the C compiler
/// actually emitted.
#[test]
fn driver_bool_byte_patterns() {
    let libs = Libs::load();
    let (c, r) = driver_pair(&libs);
    for b in 0u8..=255 {
        for &z in &[0, -1, 99] {
            check_driver(&c, &r, 2, 5, b, z);
        }
    }
}

#[test]
fn driver_random() {
    let libs = Libs::load();
    let (c, r) = driver_pair(&libs);
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for _ in 0..4000 {
        let x = rng.next_u32();
        let y = rng.next_u32();
        let b = (rng.next_u32() & 1) as u8;
        let z = rng.next_u32() as c_int;
        check_driver(&c, &r, x, y, b, z);
    }
}

/// Repeated calls in one capture: confirms the two libraries share the same
/// stdio buffering behaviour (line-by-line, newline terminated) and not just
/// the same single-call output.
#[test]
fn driver_batched_output_stream() {
    let libs = Libs::load();
    let (c, r) = driver_pair(&libs);
    let cases: Vec<(c_uint, c_uint, u8, c_int)> = vec![
        (0, 0, 0, 0),
        (3, 7, 1, -1),
        (4, 8, 0, i32::MAX),
        (u32::MAX, u32::MAX, 1, i32::MIN),
        (1, 1, 1, 1),
    ];
    let out_c = capture_stdout(|| {
        for &(x, y, b, z) in &cases {
            unsafe { c(x, y, b, z) };
        }
    });
    let out_r = capture_stdout(|| {
        for &(x, y, b, z) in &cases {
            unsafe { r(x, y, b, z) };
        }
    });
    assert_eq!(
        out_c,
        out_r,
        "batched driver output mismatch\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r),
    );
    assert_eq!(out_c.iter().filter(|&&b| b == b'\n').count(), cases.len());
}

// ---------------------------------------------------------------------------
// Exported interface
// ---------------------------------------------------------------------------

/// Every symbol the C shared object defines dynamically must also be defined
/// by the Rust shared object under the exact same name.
#[test]
fn rust_so_exports_every_c_symbol() {
    fn defined_symbols(path: &std::path::Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", path.display());
        let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut f = l.split_whitespace();
                let _addr = f.next()?;
                let kind = f.next()?;
                let name = f.next()?;
                // Only code/data symbols, skipping the toolchain-generated
                // bookkeeping entries that are not part of the interface.
                if matches!(kind, "T" | "t" | "D" | "B" | "R" | "W") {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect();
        names.sort();
        names.dedup();
        names
    }

    let c = defined_symbols(&harness::c_lib_path());
    let rust = defined_symbols(&harness::rust_lib_path());
    assert!(!c.is_empty(), "nm reported no symbols for the C library");

    let missing: Vec<&String> = c.iter().filter(|s| !rust.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C   : {c:?}\n  Rust: {rust:?}"
    );
}
