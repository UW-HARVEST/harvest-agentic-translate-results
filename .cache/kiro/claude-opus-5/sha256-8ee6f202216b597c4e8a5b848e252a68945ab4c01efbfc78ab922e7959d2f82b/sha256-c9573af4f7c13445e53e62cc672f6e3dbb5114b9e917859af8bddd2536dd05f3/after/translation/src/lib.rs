// Rust translation of c_src/src/lib.c
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
//
// The translation is intentionally literal: signed arithmetic uses wrapping
// operations (matching what the C compiler emits for `int` overflow), the file
// scope `static` variables are reproduced as process-global mutable state, and
// the original allocation / libc calls (`malloc`, `free`, `memmove`, `memset`,
// `time`, `difftime`, `snprintf`) are kept so behaviour and output bytes match.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_double, c_int, c_void};

// ---------------------------------------------------------------------------
// libc declarations (the C source includes stdio.h/stdlib.h/string.h/time.h)
// ---------------------------------------------------------------------------

pub type time_t = i64;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn time(tloc: *mut time_t) -> time_t;
    fn difftime(time1: time_t, time0: time_t) -> c_double;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// File scope state: `static int global_counter` / `static int global_accumulator`
// ---------------------------------------------------------------------------

/// Wrapper giving the two `static int`s the same "plain global, no
/// synchronisation" semantics the C code has, without tripping over Rust's
/// `static mut` reference rules.
#[repr(transparent)]
struct CGlobal(core::cell::UnsafeCell<c_int>);

// The C library is not thread safe either; this mirrors it exactly.
unsafe impl Sync for CGlobal {}

impl CGlobal {
    const fn new(v: c_int) -> Self {
        CGlobal(core::cell::UnsafeCell::new(v))
    }
    #[inline]
    fn get(&self) -> c_int {
        unsafe { *self.0.get() }
    }
    #[inline]
    fn set(&self, v: c_int) {
        unsafe { *self.0.get() = v }
    }
}

static GLOBAL_COUNTER: CGlobal = CGlobal::new(0);
static GLOBAL_ACCUMULATOR: CGlobal = CGlobal::new(0);

// ---------------------------------------------------------------------------
// typedef int (*operation_func)(int, int, int);
// typedef void (*modifier_func)(int, int);
// ---------------------------------------------------------------------------

pub type operation_func = Option<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int>;
pub type modifier_func = Option<unsafe extern "C" fn(c_int, c_int)>;

// ---------------------------------------------------------------------------
// typedef struct { int id; int value; time_t timestamp; char name[32]; } DataRecord;
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DataRecord {
    pub id: c_int,
    pub value: c_int,
    pub timestamp: time_t,
    pub name: [c_char; 32],
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// `void increment_counter(int value, int unused_param)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn increment_counter(value: c_int, _unused_param: c_int) {
    GLOBAL_COUNTER.set(GLOBAL_COUNTER.get().wrapping_add(value));
}

/// `void update_accumulator(int value, int unused_param)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_accumulator(value: c_int, _unused_param: c_int) {
    GLOBAL_ACCUMULATOR.set(GLOBAL_ACCUMULATOR.get().wrapping_mul(2).wrapping_add(value));
}

/// `int apply_operation(operation_func op, int a, int b, int c)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(
    op: operation_func,
    a: c_int,
    b: c_int,
    c: c_int,
) -> c_int {
    // A NULL `op` faults exactly like the C code does.
    let f: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int =
        unsafe { core::mem::transmute(op) };
    unsafe { f(a, b, c) }
}

/// `int add_three(int a, int b, int c)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(c)
}

/// `int multiply_add(int a, int b, int c)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_add(c)
}

/// `int complex_calc(int a, int b, int c)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_sub(b)
        .wrapping_mul(c)
        .wrapping_add(GLOBAL_COUNTER.get())
}

/// `void shift_array_data(int *arr, int size, int shift_by)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array_data(arr: *mut c_int, size: c_int, shift_by: c_int) {
    if shift_by > 0 && shift_by < size {
        let remaining = size.wrapping_sub(shift_by);
        unsafe {
            memmove(
                arr as *mut c_void,
                arr.offset(shift_by as isize) as *const c_void,
                (remaining as isize as usize).wrapping_mul(core::mem::size_of::<c_int>()),
            );
            memset(
                arr.offset(remaining as isize) as *mut c_void,
                0,
                (shift_by as isize as usize).wrapping_mul(core::mem::size_of::<c_int>()),
            );
        }
    }
}

/// `int process_pointer_data(int *ptr, int multiplier)`
///
/// `core::ptr::read` rather than `*ptr`: a plain deref makes rustc emit its
/// injected null-dereference assertion whenever UB checks are on (debug
/// profile), which turns the C's `SIGSEGV` on `ptr == NULL` into a Rust panic
/// and `SIGABRT`. `ptr::read` is the same single aligned load with no added
/// check, so the library faults identically to the C in every profile.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_pointer_data(ptr: *mut c_int, multiplier: c_int) -> c_int {
    let value = unsafe { core::ptr::read(ptr) };
    value
        .wrapping_mul(multiplier)
        .wrapping_add(GLOBAL_ACCUMULATOR.get())
}

/// `int compute_with_dynamic_memory(int base, int count)`
///
/// Note `count * sizeof(int)` in the C converts `count` to `size_t` first, so a
/// negative `count` becomes a huge request and `malloc` returns NULL; the loop
/// guards then keep that NULL from ever being touched. `ptr::write`/`ptr::read`
/// are used instead of `*p = v` / `*p` so that an actual allocation failure
/// faults the same way the C does in every build profile (see
/// `process_pointer_data`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let temp_array = unsafe {
        malloc((count as isize as usize).wrapping_mul(core::mem::size_of::<c_int>())) as *mut c_int
    };

    let mut i: c_int = 0;
    while i < count {
        unsafe {
            core::ptr::write(temp_array.offset(i as isize), base.wrapping_add(i.wrapping_mul(3)));
        }
        i = i.wrapping_add(1);
    }

    let mut sum: c_int = 0;
    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(unsafe { core::ptr::read(temp_array.offset(i as isize)) });
        i = i.wrapping_add(1);
    }

    unsafe { free(temp_array as *mut c_void) };

    sum
}

/// `int get_time_based_value(int seed)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_time_based_value(seed: c_int) -> c_int {
    let mut current_time: time_t = 0;
    let reference_time: time_t;

    unsafe { time(&mut current_time) };

    // `seed * 3600` is evaluated in `int` before being widened to `time_t`.
    reference_time = current_time.wrapping_sub(seed.wrapping_mul(3600) as time_t);

    let diff = unsafe { difftime(current_time, reference_time) };

    ((diff / 100.0) as c_int).wrapping_add(seed)
}

/// `int manipulate_records(DataRecord *records, int num_records, int shift)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn manipulate_records(
    records: *mut DataRecord,
    num_records: c_int,
    shift: c_int,
) -> c_int {
    let mut total: c_int = 0;

    if shift > 0 && shift < num_records {
        let remaining = num_records.wrapping_sub(shift);
        unsafe {
            memmove(
                records as *mut c_void,
                records.offset(shift as isize) as *const c_void,
                (remaining as isize as usize).wrapping_mul(core::mem::size_of::<DataRecord>()),
            );
        }
    }

    let limit = num_records.wrapping_sub(shift);
    let mut i: c_int = 0;
    while i < limit {
        // `ptr::read` of the field pointer rather than `(*p).value`, so a NULL
        // `records` faults like the C instead of tripping rustc's injected
        // null-dereference assertion (see `process_pointer_data`).
        total = total.wrapping_add(unsafe {
            core::ptr::read(&raw const (*records.offset(i as isize)).value)
        });
        i = i.wrapping_add(1);
    }

    total
}

/// `int hatch(int param1, int param2, int param3, int param4)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hatch(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let mut mod_func: modifier_func;

    mod_func = Some(increment_counter);
    unsafe { (mod_func.unwrap())(param1, 999) };

    mod_func = Some(update_accumulator);
    unsafe { (mod_func.unwrap())(param2, 888) };

    let mut op_func: operation_func;

    op_func = Some(add_three);
    result = result.wrapping_add(unsafe { apply_operation(op_func, param1, param2, param3) });

    op_func = Some(multiply_add);
    result = result.wrapping_add(unsafe { apply_operation(op_func, param2, param3, param4) });

    op_func = Some(complex_calc);
    result = result.wrapping_add(unsafe { apply_operation(op_func, param1, param3, param4) });

    let dynamic_data =
        unsafe { malloc(10 * core::mem::size_of::<c_int>()) as *mut c_int };
    for i in 0..10i32 {
        unsafe { core::ptr::write(dynamic_data.offset(i as isize), param1.wrapping_add(i)) };
    }

    result = result
        .wrapping_add(unsafe { process_pointer_data(dynamic_data.offset(5), param2) });

    unsafe { shift_array_data(dynamic_data, 10, 3) };
    result = result.wrapping_add(unsafe { core::ptr::read(dynamic_data) });

    unsafe { free(dynamic_data as *mut c_void) };

    result = result.wrapping_add(unsafe { get_time_based_value(param3) });

    let records =
        unsafe { malloc(5 * core::mem::size_of::<DataRecord>()) as *mut DataRecord };

    for i in 0..5i32 {
        unsafe {
            let rec = records.offset(i as isize);
            core::ptr::write(&raw mut (*rec).id, i);
            core::ptr::write(&raw mut (*rec).value, param4.wrapping_add(i.wrapping_mul(10)));
            time(&raw mut (*rec).timestamp);
            snprintf(
                (&raw mut (*rec).name) as *mut c_char,
                32,
                c"Record_%d".as_ptr(),
                i,
            );
        }
    }

    result = result.wrapping_add(unsafe { manipulate_records(records, 5, 2) });

    unsafe { free(records as *mut c_void) };

    result = result.wrapping_add(unsafe { compute_with_dynamic_memory(param1, 8) });

    result = result
        .wrapping_add(GLOBAL_COUNTER.get().wrapping_add(GLOBAL_ACCUMULATOR.get()));

    result
}
