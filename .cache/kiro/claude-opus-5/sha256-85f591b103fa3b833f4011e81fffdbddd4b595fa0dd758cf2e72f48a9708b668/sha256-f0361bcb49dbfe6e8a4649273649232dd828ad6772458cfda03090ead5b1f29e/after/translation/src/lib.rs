// Rust translation of c_src/src/driver.c and c_src/include/driver.h
//
// Original C sources: Copyright 2025 MIT Lincoln Laboratory (MIT-style license,
// see c_src/ for the full notice).
//
// This crate reproduces the complete public ABI of the C `driver` shared
// library. `nm -D libdriver.so` on the C build defines exactly two symbols:
//
//     T driver
//     T run
//
// Both are exported here with `#[no_mangle] extern "C"` and the identical
// signatures. Everything else in driver.c is `static` (internal linkage) and is
// therefore translated to private Rust functions.
//
// Byte-identical output notes:
//   * All printing goes through the platform C library's `printf`, so numeric
//     formatting (`%d`, `%.1f`) and stdout buffering behave exactly as they do
//     in the C library. Using Rust's own `println!`/`std::io::stdout` would
//     write through a separate buffer and could reorder output relative to a C
//     caller, and `%.1f` rounding would come from a different implementation.
//   * Parsing goes through the platform C library's `strtol` plus `errno`, so
//     leading-whitespace handling, partial parses, and ERANGE reporting match
//     the C code exactly (including its bugs, e.g. trailing garbage such as
//     "12abc" is accepted).

use std::ffi::{c_char, c_double, c_int, c_long, c_void};

// ---------------------------------------------------------------------------
// libc bindings (declared directly to avoid an external crate dependency)
// ---------------------------------------------------------------------------

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
    fn __errno_location() -> *mut c_int;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// Raw field access that faults exactly like C's `house->field`
// ---------------------------------------------------------------------------
//
// A plain Rust dereference (`(*p).floors`, or `&mut *p`) is instrumented when
// the crate is built with `-C debug-assertions` (the default `dev` profile):
// rustc inserts a null/alignment UB check that turns an invalid pointer into a
// non-unwinding panic and SIGABRT. The C library has no such check, so
// `run(NULL, ...)` there dies with SIGSEGV. To keep the observable behaviour of
// an invalid pointer identical in EVERY cargo profile, every field access goes
// through libc `memcpy` on a byte-offset address: the address arithmetic never
// dereferences in Rust, and the faulting load/store happens inside libc,
// producing the same SIGSEGV the C code produces.
//
// For valid pointers this is semantically identical to a direct field access:
// `house_t` is `#[repr(C)]` with no padding between `floors`, `bedrooms`, and
// `bathrooms`, and each `memcpy` touches exactly the bytes of one field.

#[inline]
unsafe fn load_field<T: Copy>(base: *const house_t, byte_offset: usize) -> T {
    let src = (base as *const u8).add(byte_offset);
    let mut out = core::mem::MaybeUninit::<T>::uninit();
    memcpy(
        out.as_mut_ptr() as *mut c_void,
        src as *const c_void,
        core::mem::size_of::<T>(),
    );
    out.assume_init()
}

#[inline]
unsafe fn store_field<T: Copy>(base: *mut house_t, byte_offset: usize, value: T) {
    let dst = (base as *mut u8).add(byte_offset);
    memcpy(
        dst as *mut c_void,
        &value as *const T as *const c_void,
        core::mem::size_of::<T>(),
    );
}

const OFF_FLOORS: usize = core::mem::offset_of!(house_t, floors);
const OFF_BEDROOMS: usize = core::mem::offset_of!(house_t, bedrooms);
const OFF_BATHROOMS: usize = core::mem::offset_of!(house_t, bathrooms);

/// `house->floors`
#[inline]
unsafe fn get_floors(h: *const house_t) -> c_int {
    load_field(h, OFF_FLOORS)
}
/// `house->floors = v`
#[inline]
unsafe fn set_floors(h: *mut house_t, v: c_int) {
    store_field(h, OFF_FLOORS, v)
}
/// `house->bedrooms`
#[inline]
unsafe fn get_bedrooms(h: *const house_t) -> c_int {
    load_field(h, OFF_BEDROOMS)
}
/// `house->bedrooms = v`
#[inline]
unsafe fn set_bedrooms(h: *mut house_t, v: c_int) {
    store_field(h, OFF_BEDROOMS, v)
}
/// `house->bathrooms`
#[inline]
unsafe fn get_bathrooms(h: *const house_t) -> c_double {
    load_field(h, OFF_BATHROOMS)
}
/// `house->bathrooms = v`
#[inline]
unsafe fn set_bathrooms(h: *mut house_t, v: c_double) {
    store_field(h, OFF_BATHROOMS, v)
}

/// `errno = value;`
#[inline]
unsafe fn set_errno(value: c_int) {
    *__errno_location() = value;
}

/// `errno`
#[inline]
unsafe fn get_errno() -> c_int {
    *__errno_location()
}

// C `<limits.h>` bounds for `int`, as compared against a `long` in the original.
const INT_MIN: c_long = c_int::MIN as c_long;
const INT_MAX: c_long = c_int::MAX as c_long;

// ---------------------------------------------------------------------------
// typedef struct { int floors; int bedrooms; double bathrooms; } house_t;
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
}

// ---------------------------------------------------------------------------
// static void add_floor(house_t *house)
// ---------------------------------------------------------------------------

/// `house->floors++;`
///
/// Signed overflow is undefined behaviour in C; gcc's actual codegen wraps, so
/// `wrapping_add` reproduces the observed behaviour instead of panicking.
///
/// Takes a raw pointer rather than `&mut house_t` so that a NULL argument
/// faults with SIGSEGV exactly as the C original does. Forming a Rust
/// reference from NULL would instead trip a debug-mode UB check and abort
/// (SIGABRT), which is an observable difference from the C behaviour.
unsafe fn add_floor(house: *mut house_t) {
    set_floors(house, get_floors(house).wrapping_add(1));
}

// ---------------------------------------------------------------------------
// static void add_bedrooms(house_t *house, int extra_bedrooms)
// ---------------------------------------------------------------------------

/// `house->bedrooms += extra_bedrooms;`
unsafe fn add_bedrooms(house: *mut house_t, extra_bedrooms: c_int) {
    set_bedrooms(house, get_bedrooms(house).wrapping_add(extra_bedrooms));
}

// ---------------------------------------------------------------------------
// static void print_house(house_t *house)
// ---------------------------------------------------------------------------

/// `printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)`
unsafe fn print_house(house: *const house_t) {
    const FMT: &[u8] = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    printf(
        FMT.as_ptr() as *const c_char,
        get_floors(house),
        get_bedrooms(house),
        get_bathrooms(house),
    );
}

// ---------------------------------------------------------------------------
// void run(house_t *the_house, int extra_bedrooms)   [public ABI symbol]
// ---------------------------------------------------------------------------

/// # Safety
///
/// `the_house` must point to a valid, writable `house_t`, exactly as required
/// by the C original (which likewise does not check for NULL). A NULL argument
/// faults on the first dereference, matching the C library's SIGSEGV.
#[no_mangle]
pub unsafe extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    set_bathrooms(the_house, get_bathrooms(the_house) + 1.0);
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

// ---------------------------------------------------------------------------
// static bool parse_val(const char *str, int *val)
// ---------------------------------------------------------------------------

/// ```c
/// errno = 0;
/// char *endp = (char *)str;
/// long tmp = strtol(str, &endp, 10);
/// if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
///     *val = tmp;
///     return true;
/// } else {
///     return false;
/// }
/// ```
///
/// The check order is preserved verbatim. Note the original never rejects
/// trailing garbage and never rejects a NULL `str`; that behaviour is kept.
unsafe fn parse_val(str_: *const c_char, val: *mut c_int) -> bool {
    set_errno(0);
    let mut endp: *mut c_char = str_ as *mut c_char;
    let tmp: c_long = strtol(str_, &mut endp, 10);
    if endp != str_ as *mut c_char && get_errno() == 0 && tmp >= INT_MIN && tmp <= INT_MAX {
        // C's implicit long -> int narrowing conversion.
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// void driver(const char *in)   [public ABI symbol]
// ---------------------------------------------------------------------------

/// # Safety
///
/// `in_` is passed straight to `strtol`, so it must be a valid NUL-terminated
/// C string, exactly as the C original requires.
#[no_mangle]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    // `int x;` -- uninitialized in C, only read after a successful parse.
    let mut x: c_int = 0;

    if parse_val(in_, &mut x) {
        let mut the_house = house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        const MSG: &[u8] = b"An error occurred\n\0";
        printf(MSG.as_ptr() as *const c_char);
    }
}
