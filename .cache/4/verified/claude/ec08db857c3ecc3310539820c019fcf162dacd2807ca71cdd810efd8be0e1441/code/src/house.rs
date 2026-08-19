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

//! Translation of the `house_t` state machine from `c_src/src/main.c`.
//!
//! The C file keeps a single file-scope `house_t the_house` that all of
//! `add_floor`, `add_bedrooms`, `add_floor_to_the_house`, `print_the_house` and
//! the externally linked `run()` mutate.  That process-global lifetime is
//! reproduced here so that repeated `run()` calls accumulate exactly like the C
//! version does (`main()` calls `run(x)` twice without resetting the state).

use std::cell::UnsafeCell;
use std::io::Write;
use std::os::raw::c_int;

/// Mirrors the C `house_t` struct.
///
/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct House {
    pub floors: i32,
    pub bedrooms: i32,
    pub bathrooms: f64,
}

/// `static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};`
pub const THE_HOUSE_INIT: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

/// Single-threaded process-global mirroring the C file-scope `the_house`.
///
/// The C program is single threaded and so is this translation; the `Sync`
/// promise is only needed so the value can live in a `static`.
struct GlobalHouse(UnsafeCell<House>);

// SAFETY: the translated program never touches `the_house` from more than one
// thread, exactly like the C original.
unsafe impl Sync for GlobalHouse {}

static THE_HOUSE: GlobalHouse = GlobalHouse(UnsafeCell::new(THE_HOUSE_INIT));

/// Borrow the process-global house.
///
/// # Safety
/// The caller must not create overlapping borrows (the C code never does).
unsafe fn the_house() -> &'static mut House {
    &mut *THE_HOUSE.0.get()
}

/// `static void add_floor(house_t *house) { house->floors++; }`
///
/// `int` overflow is undefined behaviour in C; every compiler used for the
/// original emits a wrapping 32-bit increment, so `wrapping_add` matches.
pub fn add_floor(house: &mut House) {
    house.floors = house.floors.wrapping_add(1);
}

/// `static void add_bedrooms(house_t *house, int extra_bedrooms)`
///
/// `house->bedrooms += extra_bedrooms;` — again wrapping to match the emitted
/// two's-complement addition.
pub fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// `static void add_floor_to_the_house() { add_floor(&the_house); }`
pub fn add_floor_to_the_house(house: &mut House) {
    add_floor(house);
}

/// `static void print_the_house()`
///
/// ```c
/// printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...);
/// ```
///
/// `bathrooms` is only ever `2.5 + k` for an integral `k`, i.e. an exactly
/// representable multiple of `0.5`, so `%.1f` never has to round and Rust's
/// `{:.1}` produces the identical digits.
pub fn print_the_house(out: &mut impl Write, house: &House) {
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        house.floors, house.bedrooms, house.bathrooms
    );
}

/// The body of `void run(int extra_bedrooms)`, parameterised over the sink and
/// the house so it can be unit/differentially tested.
pub fn run_on(out: &mut impl Write, house: &mut House, extra_bedrooms: i32) {
    print_the_house(out, house);
    add_floor_to_the_house(house);
    print_the_house(out, house);
    house.bathrooms += 1.0;
    print_the_house(out, house);
    add_bedrooms(house, extra_bedrooms);
    print_the_house(out, house);
}

/// The body of `void run(int extra_bedrooms)` acting on the process-global
/// `the_house` and on `stdout`, i.e. the exact semantics of the C function.
///
/// The `#[no_mangle] extern "C" fn run` wrappers that make this callable under
/// the C ABI live in the leaf targets (`src/main.rs` for the executable,
/// `ffi/src/lib.rs` for the shared object) so that the reusable library itself
/// stays free of exported symbols.
pub fn run_global(extra_bedrooms: c_int) {
    // SAFETY: single-threaded, and no other borrow of the global is live.
    let house = unsafe { the_house() };
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    run_on(&mut out, house, extra_bedrooms as i32);
    // `printf` leaves the bytes in libc's stdout buffer; flushing here keeps the
    // externally visible byte stream identical while making the ordering
    // deterministic for callers that mix this with other writers.
    let _ = out.flush();
}

/// Snapshot of the process-global house (test/introspection helper).
pub fn the_house_snapshot() -> House {
    // SAFETY: single-threaded read of the global.
    unsafe { *THE_HOUSE.0.get() }
}
