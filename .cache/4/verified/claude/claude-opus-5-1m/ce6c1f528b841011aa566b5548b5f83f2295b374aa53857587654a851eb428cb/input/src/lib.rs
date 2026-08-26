// Rust translation of the C library in c_src/ (StaticAlias).
//
// Public ABI (must match `nm -D` of the C shared library exactly):
//   int *static_alias(int *outer);
//   void driver(int initial_value, int iterations);
//
// Behavior is reproduced exactly, including the process-lifetime function-local
// `static int inner = 1;` state inside `static_alias`, the pointer aliasing
// (the returned pointer may be either `&inner` or the caller's `outer`), and the
// `printf("%d\n", ...)` output performed through C stdio so that buffering and
// byte-level output are identical to the C library.

#![allow(non_snake_case)] // crate/lib name mirrors the C library name `StaticAlias`

use core::cell::UnsafeCell;
use core::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Wrapper giving a `static` a stable, mutable-through-raw-pointer storage
/// location, mirroring C's function-local `static int inner`.
#[repr(transparent)]
struct StaticInt(UnsafeCell<c_int>);

// Matches C semantics: no synchronization at all (the C code is likewise not
// thread-safe with respect to its function-local static).
unsafe impl Sync for StaticInt {}

impl StaticInt {
    const fn new(value: c_int) -> Self {
        StaticInt(UnsafeCell::new(value))
    }

    #[inline]
    fn as_ptr(&self) -> *mut c_int {
        self.0.get()
    }
}

/// `static int inner = 1;` from `static_alias`.
static INNER: StaticInt = StaticInt::new(1);

/// C:
/// ```c
/// int*
/// static_alias(int *outer) {
///   static int inner = 1;
///   if(*outer >= inner) {
///     inner += *outer;
///     return &inner;
///   } else {
///     *outer += inner;
///     return outer;
///   }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    let inner = INNER.as_ptr();

    // *outer is read without any NULL check, exactly as in the C source.
    let outer_val = *outer;
    let inner_val = *inner;

    if outer_val >= inner_val {
        // inner += *outer;  (wrapping matches the compiled C behavior)
        *inner = inner_val.wrapping_add(outer_val);
        inner
    } else {
        // *outer += inner;
        *outer = outer_val.wrapping_add(inner_val);
        outer
    }
}

/// C:
/// ```c
/// void
/// driver(int initial_value, int iterations) {
///   int *running_sum = &initial_value;
///   for (int i = 0; i < iterations; i++) {
///     running_sum = static_alias(running_sum);
///     printf("%d\n", *running_sum);
///   }
///   return;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    // `initial_value` is a by-value parameter in C too, so it lives in the
    // callee's frame and may be mutated through `running_sum`.
    let mut initial_value = initial_value;
    let mut running_sum: *mut c_int = &mut initial_value;

    let mut i: c_int = 0;
    while i < iterations {
        running_sum = static_alias(running_sum);
        printf(b"%d\n\0".as_ptr() as *const c_char, *running_sum);
        i = i.wrapping_add(1);
    }
}
