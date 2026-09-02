// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/driver.c`.
//!
//! The C translation unit exports exactly two public symbols, `driver` and
//! `run` (`nm -D libdriver.so` -> `T driver`, `T run`). Everything else in the
//! original file (`the_house`, `add_floor`, `add_bedrooms`,
//! `add_floor_to_the_house`, `print_the_house`, `parse_val`) is `static` and
//! therefore has internal linkage; those are kept private here as well.
//!
//! Output is produced through the platform C library (`printf`) and parsing is
//! delegated to the platform `strtol`/`errno` so that the emitted bytes, the
//! stdout buffering behaviour and the numeric edge cases are identical to the
//! original library rather than merely similar.

use core::ffi::{c_char, c_double, c_int, c_long};

unsafe extern "C" {
    /// Variadic `printf` from the platform C library, exactly as used by the C
    /// translation unit.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;

    /// `strtol` from the platform C library. Used instead of a Rust parser so
    /// that the accepted syntax (leading whitespace, `+`/`-`, base-10 prefix
    /// handling), the `endptr` semantics and the `ERANGE` reporting match the
    /// original byte for byte.
    unsafe fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;

    /// glibc's accessor for the thread-local `errno` lvalue, so that
    /// `errno = 0` and the subsequent `errno == 0` test observe the very same
    /// storage that `strtol` writes to.
    unsafe fn __errno_location() -> *mut c_int;
}

/// `INT_MIN` / `INT_MAX` from `<limits.h>` for the range test in `parse_val`.
const INT_MIN: c_long = c_int::MIN as c_long;
const INT_MAX: c_long = c_int::MAX as c_long;

/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

/// ```c
/// static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
/// ```
///
/// File-scope mutable state: the C library keeps mutating this single instance,
/// so successive calls to `driver` / `run` observe the values left behind by
/// earlier calls. That statefulness is part of the observable behaviour and is
/// preserved here. Like the C original, access is not synchronised.
static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

/// Borrow the single mutable `the_house` instance.
///
/// # Safety
///
/// The caller must not create overlapping mutable borrows. As in the C
/// original, this is single-threaded-only state.
#[inline]
unsafe fn the_house() -> &'static mut House {
    unsafe { &mut *(&raw mut THE_HOUSE) }
}

/// ```c
/// static void add_floor(house_t *house) {
///     house->floors++;
/// }
/// ```
///
/// `wrapping_add` reproduces the two's-complement result that the C compiler
/// emits for `int` overflow; it must not become a Rust panic or a saturating
/// value.
fn add_floor(house: &mut House) {
    house.floors = house.floors.wrapping_add(1);
}

/// ```c
/// static void add_bedrooms(house_t *house, int extra_bedrooms) {
///     house->bedrooms += extra_bedrooms;
/// }
/// ```
fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// ```c
/// static void add_floor_to_the_house() {
///     add_floor(&the_house);
/// }
/// ```
unsafe fn add_floor_to_the_house() {
    add_floor(unsafe { the_house() });
}

/// ```c
/// static void print_the_house() {
///     printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...);
/// }
/// ```
unsafe fn print_the_house() {
    let house = *unsafe { the_house() };
    unsafe {
        c_printf(
            c"The house has %d floors, %d bedrooms, and %.1f bathrooms\n".as_ptr(),
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

/// ```c
/// void run(int extra_bedrooms) {
///     print_the_house();
///     add_floor_to_the_house();
///     print_the_house();
///     the_house.bathrooms += 1.0;
///     print_the_house();
///     add_bedrooms(&the_house, extra_bedrooms);
///     print_the_house();
/// }
/// ```
///
/// `run` has external linkage in the C source (it is not declared in
/// `driver.h`, but it is not `static` either), so it is part of the exported
/// ABI and is re-exported here under the same unmangled name.
///
/// # Safety
///
/// Mutates process-wide state and writes to `stdout` via the C library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(extra_bedrooms: c_int) {
    unsafe {
        print_the_house();
        add_floor_to_the_house();
        print_the_house();
        the_house().bathrooms += 1.0;
        print_the_house();
        add_bedrooms(the_house(), extra_bedrooms);
        print_the_house();
    }
}

/// ```c
/// static bool parse_val(const char *str, int *val) {
///     errno = 0;
///     char *endp = (char *)str;
///     long tmp = strtol(str, &endp, 10);
///     if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
///         *val = tmp;
///         return true;
///     } else {
///         return false;
///     }
/// }
/// ```
///
/// The order of the four conjuncts is preserved. Note the original's quirks,
/// which are reproduced rather than fixed:
/// * trailing garbage is accepted (`"12abc"` parses as `12`) because only
///   `endp != str` is checked, not `*endp == '\0'`;
/// * `*val` is left untouched on failure;
/// * an out-of-range magnitude is rejected via `errno == ERANGE` (non-zero)
///   before the `INT_MIN`/`INT_MAX` comparison ever matters.
unsafe fn parse_val(str: *const c_char, val: *mut c_int) -> bool {
    unsafe {
        let errno = __errno_location();
        *errno = 0;

        let mut endp: *mut c_char = str as *mut c_char;
        let tmp: c_long = strtol(str, &mut endp, 10);

        if endp != str as *mut c_char && *errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX {
            *val = tmp as c_int;
            true
        } else {
            false
        }
    }
}

/// ```c
/// void driver(const char *in) {
///     int x;
///     if (parse_val(in, &x)) {
///         run(x);
///         run(x);
///     } else {
///         printf("An error occurred\n");
///     }
/// }
/// ```
///
/// `x` is deliberately left uninitialised in the C original; it is only ever
/// read when `parse_val` returned true, in which case it has been written.
///
/// # Safety
///
/// `in` must be a valid pointer to a NUL-terminated C string. A null or
/// dangling pointer is passed straight through to `strtol`, matching the C
/// library's (non-)handling of that case.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(r#in: *const c_char) {
    unsafe {
        let mut x: c_int = 0;
        if parse_val(r#in, &mut x) {
            run(x);
            run(x);
        } else {
            c_printf(c"An error occurred\n".as_ptr());
        }
    }
}
