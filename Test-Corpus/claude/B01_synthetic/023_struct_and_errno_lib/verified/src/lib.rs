// Rust translation of c_src/src/driver.c
//
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

use std::ffi::{c_char, c_double, c_int, c_long};
use std::ptr::{read_volatile, write_volatile};

// libc bindings. The C library used the platform's stdio/strtol, so we call the
// very same routines to guarantee byte-identical output (formatting rules,
// rounding of `%.1f`, and stdout buffering behaviour all match exactly).
extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
    fn __errno_location() -> *mut c_int;
}

/// C: `typedef struct { int floors; int bedrooms; double bathrooms; } house_t;`
///
/// `#[repr(C)]` keeps the layout (offsets 0, 4, 8; size 16, align 8) identical
/// to the C struct, since `run` is part of the public ABI.
#[repr(C)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
}

// The C code never null-checks `house_t *`, so `house->floors` on an invalid
// pointer is simply a machine load/store that faults (SIGSEGV on Linux). A plain
// Rust `(*house).floors` — and even `addr_of!((*house).floors)` — is compiled
// with an extra "null pointer dereference occurred" UB-check whenever
// `-C debug-assertions` / `-Z ub-checks` is on (the default for cargo's dev and
// test profiles). That check turns C's SIGSEGV into a Rust panic-abort
// (SIGABRT), which is an observable divergence from the C.
//
// The same applies to `ptr::read_volatile::<i32>` / `read_unaligned` /
// `copy_nonoverlapping`, which carry their own `assert_unsafe_precondition!`
// null/alignment checks and abort where the C would either work (misaligned but
// mapped, which x86 allows) or fault.
//
// So the field accesses are done with integer address arithmetic (no `Deref`
// projection, hence no MIR null/alignment check) plus byte-wise
// `read_volatile::<u8>` / `write_volatile::<u8>` (alignment 1, so no
// precondition can ever trip). That combination is the only one that is
// check-free in *every* build profile: for a valid pointer it moves exactly the
// same bytes the C does, and for an invalid pointer the process faults exactly
// like the C does (verified against the C `.so` in tests/phase_c_errors.rs).
// The offsets come from `offset_of!` on the `#[repr(C)]` type, so they cannot
// drift from the C layout.
const OFF_FLOORS: usize = core::mem::offset_of!(house_t, floors);
const OFF_BEDROOMS: usize = core::mem::offset_of!(house_t, bedrooms);
const OFF_BATHROOMS: usize = core::mem::offset_of!(house_t, bathrooms);

// Layout parity with the C `house_t` is part of `run`'s ABI.
const _: () = assert!(OFF_FLOORS == 0);
const _: () = assert!(OFF_BEDROOMS == 4);
const _: () = assert!(OFF_BATHROOMS == 8);
const _: () = assert!(core::mem::size_of::<house_t>() == 16);
const _: () = assert!(core::mem::align_of::<house_t>() == 8);

/// Address of a `house_t` field, computed without dereferencing anything.
#[inline]
fn field_addr(house: *mut house_t, off: usize) -> usize {
    (house as usize).wrapping_add(off)
}

#[inline]
unsafe fn load_bytes<const N: usize>(addr: usize) -> [u8; N] {
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = read_volatile(addr.wrapping_add(i) as *const u8);
        i += 1;
    }
    out
}

#[inline]
unsafe fn store_bytes<const N: usize>(addr: usize, bytes: [u8; N]) {
    let mut i = 0;
    while i < N {
        write_volatile(addr.wrapping_add(i) as *mut u8, bytes[i]);
        i += 1;
    }
}

#[inline]
unsafe fn get_floors(house: *mut house_t) -> c_int {
    c_int::from_ne_bytes(load_bytes::<4>(field_addr(house, OFF_FLOORS)))
}
#[inline]
unsafe fn set_floors(house: *mut house_t, v: c_int) {
    store_bytes(field_addr(house, OFF_FLOORS), v.to_ne_bytes());
}
#[inline]
unsafe fn get_bedrooms(house: *mut house_t) -> c_int {
    c_int::from_ne_bytes(load_bytes::<4>(field_addr(house, OFF_BEDROOMS)))
}
#[inline]
unsafe fn set_bedrooms(house: *mut house_t, v: c_int) {
    store_bytes(field_addr(house, OFF_BEDROOMS), v.to_ne_bytes());
}
#[inline]
unsafe fn get_bathrooms(house: *mut house_t) -> c_double {
    c_double::from_ne_bytes(load_bytes::<8>(field_addr(house, OFF_BATHROOMS)))
}
#[inline]
unsafe fn set_bathrooms(house: *mut house_t, v: c_double) {
    store_bytes(field_addr(house, OFF_BATHROOMS), v.to_ne_bytes());
}

// C: static void add_floor(house_t *house) { house->floors++; }
//
// `house->floors++` on `INT_MAX` is signed overflow (UB in C); gcc at the
// optimisation level used by c_src/CMakeLists.txt wraps two's-complement, so
// `wrapping_add` reproduces it.
unsafe fn add_floor(house: *mut house_t) {
    set_floors(house, get_floors(house).wrapping_add(1));
}

// C: static void add_bedrooms(house_t *house, int extra_bedrooms)
unsafe fn add_bedrooms(house: *mut house_t, extra_bedrooms: c_int) {
    set_bedrooms(house, get_bedrooms(house).wrapping_add(extra_bedrooms));
}

// C: static void print_house(house_t *house)
unsafe fn print_house(house: *mut house_t) {
    // Field loads happen before the call, exactly as in C.
    let floors = get_floors(house);
    let bedrooms = get_bedrooms(house);
    let bathrooms = get_bathrooms(house);
    printf(
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0".as_ptr() as *const c_char,
        floors,
        bedrooms,
        bathrooms,
    );
}

/// C: `void run(house_t *the_house, int extra_bedrooms)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    // C: the_house->bathrooms += 1.0;
    set_bathrooms(the_house, get_bathrooms(the_house) + 1.0);
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

// C: static bool parse_val(const char *str, int *val)
unsafe fn parse_val(str: *const c_char, val: *mut c_int) -> bool {
    *__errno_location() = 0;
    let mut endp: *mut c_char = str as *mut c_char;
    let tmp: c_long = strtol(str, &mut endp, 10);
    if endp != str as *mut c_char
        && *__errno_location() == 0
        && tmp >= c_int::MIN as c_long
        && tmp <= c_int::MAX as c_long
    {
        // C: *val = tmp;  (implicit long -> int conversion)
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

/// C: `void driver(const char *in)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    // C: int x;  (uninitialized)
    let mut x: c_int = 0;
    if parse_val(in_, &mut x) {
        // C: house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
        let mut the_house = house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        printf(b"An error occurred\n\0".as_ptr() as *const c_char);
    }
}
