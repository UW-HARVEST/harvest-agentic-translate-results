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
// Both are exported here with `#[no_mangle] extern "C"` and identical
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
//     the C code exactly (including its quirks, e.g. trailing garbage such as
//     "12abc" is accepted and NULL is not checked).
//   * `house_t` is only ever touched through raw pointers, never through a Rust
//     reference, so an invalid pointer faults (SIGSEGV) exactly like the C
//     rather than tripping a Rust null/alignment assertion (SIGABRT).

use std::ffi::{c_char, c_double, c_int, c_long, c_void};
use std::mem::{offset_of, size_of, MaybeUninit};

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
// Unchecked field access
// ---------------------------------------------------------------------------
//
// The C dereferences `house_t *` with no null or alignment check, so an invalid
// pointer produces a hardware fault (SIGSEGV). Rust inserts a "null pointer
// dereference" check at every raw-pointer place projection (`(*p).field`) when
// `-C debug-assertions` is on, which turns that fault into a SIGABRT panic and
// so is observably different. To be faithful under *every* build profile, the
// field loads and stores below go through libc's `memcpy` at an address
// computed by integer arithmetic: no Rust dereference happens at all, and a bad
// address faults inside `memcpy` exactly as the C does.

/// Byte address of a field, computed without dereferencing `base`.
#[inline]
fn field_addr(base: usize, off: usize) -> usize {
    base.wrapping_add(off)
}

/// `*(T *)(base + off)`
#[inline]
unsafe fn load<T: Copy>(base: usize, off: usize) -> T {
    let mut v = MaybeUninit::<T>::uninit();
    memcpy(
        v.as_mut_ptr() as *mut c_void,
        field_addr(base, off) as *const c_void,
        size_of::<T>(),
    );
    v.assume_init()
}

/// `*(T *)(base + off) = val;`
#[inline]
unsafe fn store<T: Copy>(base: usize, off: usize, val: T) {
    memcpy(
        field_addr(base, off) as *mut c_void,
        &val as *const T as *const c_void,
        size_of::<T>(),
    );
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

const OFF_FLOORS: usize = offset_of!(house_t, floors);
const OFF_BEDROOMS: usize = offset_of!(house_t, bedrooms);
const OFF_BATHROOMS: usize = offset_of!(house_t, bathrooms);

// ---------------------------------------------------------------------------
// static void add_floor(house_t *house)
// ---------------------------------------------------------------------------

/// `house->floors++;`
///
/// Signed overflow is undefined behaviour in C; gcc's actual codegen wraps, so
/// `wrapping_add` reproduces the observed behaviour instead of panicking.
unsafe fn add_floor(house: usize) {
    let v: c_int = load(house, OFF_FLOORS);
    store(house, OFF_FLOORS, v.wrapping_add(1));
}

// ---------------------------------------------------------------------------
// static void add_bedrooms(house_t *house, int extra_bedrooms)
// ---------------------------------------------------------------------------

/// `house->bedrooms += extra_bedrooms;`
unsafe fn add_bedrooms(house: usize, extra_bedrooms: c_int) {
    let v: c_int = load(house, OFF_BEDROOMS);
    store(house, OFF_BEDROOMS, v.wrapping_add(extra_bedrooms));
}

// ---------------------------------------------------------------------------
// static void print_house(house_t *house)
// ---------------------------------------------------------------------------

/// `printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)`
unsafe fn print_house(house: usize) {
    const FMT: &[u8] = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    let floors: c_int = load(house, OFF_FLOORS);
    let bedrooms: c_int = load(house, OFF_BEDROOMS);
    let bathrooms: c_double = load(house, OFF_BATHROOMS);
    printf(FMT.as_ptr() as *const c_char, floors, bedrooms, bathrooms);
}

// ---------------------------------------------------------------------------
// void run(house_t *the_house, int extra_bedrooms)   [public ABI symbol]
// ---------------------------------------------------------------------------

/// # Safety
///
/// `the_house` must point to a valid, writable `house_t`, exactly as required
/// by the C original (which likewise does not check for NULL).
#[no_mangle]
pub unsafe extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    let h = the_house as usize;

    print_house(h);
    add_floor(h);
    print_house(h);
    {
        // `the_house->bathrooms += 1.0;`
        let v: c_double = load(h, OFF_BATHROOMS);
        store(h, OFF_BATHROOMS, v + 1.0);
    }
    print_house(h);
    add_bedrooms(h, extra_bedrooms);
    print_house(h);
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
        store(val as usize, 0, tmp as c_int);
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
        run(&mut the_house as *mut house_t, x);
        run(&mut the_house as *mut house_t, x);
    } else {
        const MSG: &[u8] = b"An error occurred\n\0";
        printf(MSG.as_ptr() as *const c_char);
    }
}
