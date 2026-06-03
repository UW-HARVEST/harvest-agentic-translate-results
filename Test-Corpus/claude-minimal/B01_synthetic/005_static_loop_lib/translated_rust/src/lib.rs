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

use std::sync::atomic::{AtomicI32, Ordering};

// Mirror the C `static int sum = 0;` inside `static_sum` with a module-level
// atomic so the running total is preserved across calls.
static SUM: AtomicI32 = AtomicI32::new(0);

/// Maintain a running total. Each call adds `update` to the persisted sum
/// and returns the new total. Equivalent to the C `static_sum` function.
#[no_mangle]
pub extern "C" fn static_sum(update: i32) -> i32 {
    // fetch_add returns the previous value, so add `update` to compute the
    // new total to return. Use wrapping addition to match C's signed int
    // overflow semantics (which in practice C compilers implement as wrap).
    let prev = SUM.fetch_add(update, Ordering::SeqCst);
    prev.wrapping_add(update)
}

/// Maintain a running total using the static variable in `static_sum`.
/// Equivalent to the C `driver` function.
#[no_mangle]
pub extern "C" fn driver(stride: i32) {
    for i in 0..10i32 {
        println!("{}", static_sum(i.wrapping_mul(stride)));
    }
}
