// Rust translation of the MIT Lincoln Laboratory `driver` C library.
//
// Original C library (c_src/):
//   * include/driver.h  -- declares `void driver(double f);`
//   * src/driver.c      -- defines it as:
//
//         typedef union { uint64_t x; double f; } raw_double_t;
//
//         void driver(double f) {
//             raw_double_t u = {.f = f};
//             printf("%llx %a %.4f\n", u.x, f, f);
//         }
//
// The complete public ABI of the C shared library consists of the single
// exported symbol `driver` (verified with `nm -D` on the CMake-built
// libdriver.so). No namespace-renaming preprocessor macros are present in the
// public header, so the linker symbol is plainly `driver`.
//
// Byte-identical output requirement
// --------------------------------
// The C code hands three conversions to the platform's `printf`:
//   * `%llx`  -- the raw IEEE-754 bit pattern of the double, as hex
//   * `%a`    -- the C99 hexadecimal-float form of the double
//   * `%.4f`  -- fixed-point with four fractional digits
//
// `%a` and `%.4f` are exquisitely implementation-defined at the margins
// (glibc trims trailing mantissa zeros for `%a`, renders subnormals with a
// leading `0x0.` digit and a `p-1022` exponent, spells non-finite values as
// `inf`/`-inf`/`nan`/`-nan` following the sign bit, and rounds `%.4f` off the
// *exact* binary value using the current rounding mode -- round-half-to-even
// at ties). Rather than re-deriving all of that, this translation delegates
// the formatting to the very same libc `printf` the C library calls. That also
// keeps stdout buffering, locale handling, and interleaving with any C caller's
// own output bit-for-bit identical.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    /// The platform `printf` from libc, which Rust's std already links.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Mirror of the C translation unit's `raw_double_t`:
///
/// ```c
/// typedef union {
///     uint64_t x;
///     double f;
/// } raw_double_t;
/// ```
///
/// Written to through the `.f` member and read back through `.x`, i.e. a
/// straight type-pun of the double's storage bytes. On every target this
/// library builds for, `double` and `uint64_t` are both 8 bytes with identical
/// alignment, so the read is fully initialized and endianness-faithful --
/// exactly what `f64::to_bits` gives us.
#[repr(C)]
union raw_double_t {
    x: u64,
    f: f64,
}

impl raw_double_t {
    /// `raw_double_t u = {.f = f};` followed by reading `u.x`.
    #[inline]
    fn from_f64(f: f64) -> Self {
        raw_double_t { f }
    }

    #[inline]
    fn bits(&self) -> u64 {
        // Safe in practice: `f` occupies all 8 bytes of the union, so no
        // padding is observed. Equivalent to the C `u.x` read.
        unsafe { self.x }
    }
}

/// The format string handed to `printf`, NUL-terminated for C.
///
/// `%llx` consumes an `unsigned long long`; `%a` and `%.4f` each consume a
/// `double`. The variadic call below passes `u64` and `f64`, which match those
/// C types on all supported platforms (`unsigned long long` is 64-bit
/// everywhere Rust targets, and varargs promote `f64` to `double`
/// unchanged).
const FMT: &[u8] = b"%llx %a %.4f\n\0";

/// `void driver(double f);`
///
/// Prints the double's raw bit pattern, its hexadecimal-float form, and its
/// fixed-point form with four fractional digits, separated by spaces and
/// terminated by a newline.
#[unsafe(no_mangle)]
pub extern "C" fn driver(f: f64) {
    let u = raw_double_t::from_f64(f);
    let bits = u.bits();

    // The C code's `printf` return value is discarded; do likewise.
    unsafe {
        printf(FMT.as_ptr() as *const c_char, bits, f, f);
    }
}

// ---------------------------------------------------------------------------
// A note on why `%llx` is passed a `u64` and not a `c_ulonglong` cast:
// `c_ulonglong` *is* `u64` on every platform Rust supports, so the two are the
// same type. The explicit alias below documents the intent and would surface a
// compile error on any hypothetical platform where that stopped holding.
// ---------------------------------------------------------------------------
const _: () = {
    // `unsigned long long` must be 64-bit for the `%llx` conversion to read
    // exactly the bits we pass.
    assert!(std::mem::size_of::<std::ffi::c_ulonglong>() == 8);
    // `double` must be 64-bit for the union pun to be a faithful bit copy.
    assert!(std::mem::size_of::<f64>() == 8);
    assert!(std::mem::size_of::<raw_double_t>() == 8);
    // Keep the imports honest across platforms.
    assert!(std::mem::size_of::<c_int>() == 4);
    assert!(std::mem::size_of::<c_uint>() == 4);
};
