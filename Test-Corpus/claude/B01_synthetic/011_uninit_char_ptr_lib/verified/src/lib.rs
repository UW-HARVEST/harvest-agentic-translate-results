// Rust translation of the C library in c_src/ (MIT Lincoln Laboratory `driver`).
//
// The C library consists of a single translation unit (c_src/src/driver.c) with
// four external (non-`static`) functions, all of which are exported by the
// shared object:
//
//     void printLine(const char *line);
//     void bad(void);
//     void good(void);
//     void driver(int useGood);
//
// Only `driver` is declared in the public header (c_src/include/driver.h), but
// the C build exports all four symbols, so all four are reproduced here with
// their exact C linkage names and signatures.
//
// Output is produced by calling libc's `printf` directly (rather than Rust's
// `println!`) so that the FILE* stream, buffering behaviour and byte output are
// identical to the C library's.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    // int printf(const char *restrict format, ...);
    #[link_name = "printf"]
    unsafe fn libc_printf(format: *const c_char, ...) -> c_int;
}

/// Format string used by the C code: `printf("%s\n", line);`
const FMT_S_NL: &[u8] = b"%s\n\0";

/// The string literal assigned in the C `good()` function.
const GOOD_STRING: &[u8] = b"string\0";

/// C:
/// ```c
/// void printLine(const char *line)
/// {
///     if (line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            libc_printf(FMT_S_NL.as_ptr() as *const c_char, line);
        }
    }
}

/// C:
/// ```c
/// void bad()
/// {
///     char *data;
///     printLine(data);
/// }
/// ```
///
/// `data` is never initialised in the C source: this is the intentional defect
/// (use of an uninitialised variable, CWE-457). The bug is *not* fixed here --
/// the defective call to `printLine` with an uninitialised pointer is preserved
/// rather than removed or "corrected".
///
/// Reading the uninitialised object is undefined behaviour in C, so `bad()` has
/// no single "correct" observable behaviour to translate. Verified against the
/// actual gcc output (see `ERRORS.md` §UB):
///
/// * `-O1`  emits `mov $0x0,%edi; call printLine`  -> `printLine(NULL)`
/// * `-O2`/`-O3`/`-Os` emit `xor %edi,%edi; jmp printLine` -> `printLine(NULL)`
/// * `-O0`  emits `mov -0x8(%rbp),%rax; ...; call printLine`, i.e. it reads
///   whatever stack residue happens to live at `[rbp-8]`.
///
/// A null pointer is therefore *exactly* what every optimising build does, and
/// that is what is reproduced here.
///
/// The `-O0` case (which is what `c_src/CMakeLists.txt` produces by default,
/// since `CMAKE_BUILD_TYPE` is empty) is not reproducible by **any** translation:
/// measurements on the real `.so` show its output changing with the preceding
/// call sequence, and in one case `puts` printed `driver`'s own machine-code
/// bytes because the residual pointer aimed into the code segment. Reproducing
/// that would require embedding gcc's exact object code and load address. A
/// frame-exact `naked_asm!` replica of the `-O0` prologue was prototyped and
/// confirmed to match only some residue states while injecting real UB into
/// this library, so it was rejected.
///
/// Consequently `bad()`/`driver(0)` are differentially tested against an `-O2`
/// C build, where the behaviour is well defined; `tests/ub_bad.rs` pins the
/// `-O0` vs `-O2` codegen difference so this exclusion cannot silently widen.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // char *data;  /* deliberately left uninitialised in the C original */
    let data: *const c_char = ptr::null();
    unsafe {
        printLine(data);
    }
}

/// C:
/// ```c
/// void good()
/// {
///     char *data;
///     data = "string";
///     printLine(data);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let data: *const c_char = GOOD_STRING.as_ptr() as *const c_char;
    unsafe {
        printLine(data);
    }
}

/// C:
/// ```c
/// void driver(int useGood)
/// {
///     if (useGood)
///     {
///         good();
///     }
///     else
///     {
///         bad();
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    unsafe {
        if useGood != 0 {
            good();
        } else {
            bad();
        }
    }
}
