//! Phase B/C -- differential tests that go through the C ABI of **both**
//! shared libraries via `libloading`.
//!
//! `CONFIGS.md` rows 1-4 and `ERRORS.md` rows 20-22.
//!
//! Nothing here calls a Rust function directly: `driver` and `main` are looked
//! up with `dlsym` in `target/<prof>/libdriver.so` exactly as an external C
//! caller would, which also exercises the `#[no_mangle]` export wrappers.
//!
//! Everything lives in a *single* `#[test]` function on purpose: comparing the
//! two libraries requires temporarily redirecting fd 1, and libtest's own
//! progress output (which goes straight to fd 1 when another test in the same
//! binary finishes) would otherwise be captured along with `driver`'s output.

mod common;

use common::{capture_with_stdin, Rng, StdoutTap};
use libloading::{Library, Symbol};
use std::os::raw::c_int;

type DriverFn = unsafe extern "C" fn(f32);
type MainFn = unsafe extern "C" fn() -> c_int;

struct Case {
    bits: u32,
    out: Vec<u8>,
}

#[test]
fn ffi_differential_suite() {
    let c_lib = unsafe { Library::new(common::c_shared_lib()) }.expect("dlopen C .so");
    let r_lib = unsafe { Library::new(common::rust_shared_lib()) }.expect("dlopen Rust .so");
    let c: Symbol<DriverFn> = unsafe { c_lib.get(b"driver\0") }.expect("dlsym driver in C .so");
    let r: Symbol<DriverFn> = unsafe { r_lib.get(b"driver\0") }.expect("dlsym driver in Rust .so");

    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;
    let mut kept: Vec<Case> = Vec::new();

    {
        let mut tap = StdoutTap::new();
        let mut check = |bits: u32, keep: bool| {
            let x = f32::from_bits(bits);
            let ((), cout) = tap.run(|| unsafe { c(x) });
            let ((), rout) = tap.run(|| unsafe { r(x) });
            checked += 1;
            if cout != rout && fails.len() < 25 {
                fails.push(format!(
                    "bits={bits:#010x}: C={:?} Rust={:?}",
                    String::from_utf8_lossy(&cout),
                    String::from_utf8_lossy(&rout)
                ));
            }
            if keep {
                kept.push(Case { bits, out: cout });
            }
        };

        // -- CONFIGS.md row 1: exhaustive low bit patterns -------------------
        // +0 and the entire first block of subnormals (exponent field 0).
        for bits in 0u32..=0x0001_0000 {
            check(bits, false);
        }

        // -- CONFIGS.md row 1: randomized full 32-bit sweep ------------------
        // normals, subnormals, inf, every NaN payload, both signs
        let rng = Rng::new(0x00C0_FFEE_1234_5678);
        for i in 0..80_000 {
            let bits = rng.next_u32();
            check(bits, i < 500);
        }

        // -- CONFIGS.md row 2 / ERRORS.md rows 20-21: named specials ---------
        let named: [u32; 21] = [
            0x0000_0000, // +0
            0x8000_0000, // -0
            0x0000_0001, // +FLT_TRUE_MIN
            0x8000_0001, // -FLT_TRUE_MIN
            0x0080_0000, // +FLT_MIN
            0x8080_0000, // -FLT_MIN
            0x007f_ffff, // largest subnormal
            0x7f7f_ffff, // +FLT_MAX
            0xff7f_ffff, // -FLT_MAX
            0x7f80_0000, // +inf
            0xff80_0000, // -inf
            0x7fc0_0000, // +qNaN
            0xffc0_0000, // -qNaN
            0x7fa0_0000, // +sNaN
            0xffa0_0000, // -sNaN
            0x7f80_0001, // smallest sNaN payload
            0x7fff_ffff, // NaN, all payload bits set
            0x3f80_0000, // 1.0
            0xbf80_0000, // -1.0
            0x4b7f_ffff, // 16777215.0
            0x4b80_0000, // 16777216.0
        ];
        for bits in named {
            check(bits, true);
        }
        // NaNs with arbitrary payloads must not be canonicalised anywhere.
        let rng = Rng::new(0x5EED_0002);
        for _ in 0..3000 {
            let payload = rng.next_u32() & 0x007f_ffff;
            if payload != 0 {
                check(0x7f80_0000 | payload, false);
                check(0xff80_0000 | payload, false);
            }
        }

        // -- CONFIGS.md row 3: every exponent field x corner mantissas -------
        for ef in 0u32..=255 {
            for mant in [0u32, 1, 0x0000_00ff, 0x0040_0000, 0x007f_fffe, 0x007f_ffff] {
                for sign in [0u32, 0x8000_0000] {
                    check(sign | (ef << 23) | mant, false);
                }
            }
        }
    }

    assert!(
        fails.is_empty(),
        "driver() diverged for {} of {checked} bit patterns:\n{}",
        fails.len(),
        fails.join("\n")
    );
    eprintln!("FFI driver(): {checked} bit patterns matched");

    // -- ERRORS.md row 22: output shape ------------------------------------
    // `driver` hard-codes `sizeof(float)`, so the output is always 8 lowercase
    // hex digits followed by '\n' -- and it is a byte-exact echo of the input
    // pattern in native (little-endian) order, with no NaN canonicalisation.
    for case in &kept {
        assert_eq!(case.out.len(), 9, "width for bits={:#010x}", case.bits);
        assert_eq!(*case.out.last().unwrap(), b'\n');
        assert!(
            case.out[..8]
                .iter()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "not lowercase hex: {:?}",
            String::from_utf8_lossy(&case.out)
        );
        let expect: String = f32::from_bits(case.bits)
            .to_ne_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            + "\n";
        assert_eq!(
            String::from_utf8_lossy(&case.out),
            expect,
            "byte order / echo for bits={:#010x}",
            case.bits
        );
    }

    // -- CONFIGS.md row 4: the exported `main` symbol -----------------------
    // Called once per library: C `stdio` and Rust's `Stdin` both keep a
    // persistent buffer, so a second call would not re-read the file.
    let input = b"  \n\t -3.5e-2 leftover";
    let (cret, cout) = {
        let m: Symbol<MainFn> = unsafe { c_lib.get(b"main\0") }.expect("dlsym main in C .so");
        capture_with_stdin(input, || unsafe { m() })
    };
    let (rret, rout) = {
        let m: Symbol<MainFn> = unsafe { r_lib.get(b"main\0") }.expect("dlsym main in Rust .so");
        capture_with_stdin(input, || unsafe { m() })
    };
    assert_eq!(cret, 0, "C main should return 0");
    assert_eq!(rret, cret, "main return value");
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "main stdout"
    );
    // Cross-check the FFI path against the standalone executables.
    let via_exe = common::c_run(input);
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&via_exe.stdout),
        "FFI main vs C executable"
    );
    eprintln!("FFI main(): matched ({})", String::from_utf8_lossy(&cout).trim());
}
