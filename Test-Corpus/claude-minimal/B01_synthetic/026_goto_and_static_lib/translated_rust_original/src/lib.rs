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

use std::os::raw::c_int;
use std::sync::Mutex;

// Mirror of `static int y = 123;` from the C source. Wrapped in a Mutex to
// safely mutate global state from Rust.
static Y: Mutex<c_int> = Mutex::new(123);

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;

    // Use a labeled block to emulate the C `goto fail;` pattern.
    'fail: {
        if x != 1 {
            println!("Error: x != 1");
            result = 1;
            break 'fail;
        }

        let y_val = *Y.lock().unwrap();
        if y_val != 2 {
            println!("Error: x == 1 but y != 2");
            result = 2;
            break 'fail;
        }

        if z != 3 {
            println!("Error: x == 1 and y == 2, but z != 3");
            result = 3;
            break 'fail;
        }

        println!("Ok!");
        return result;
    }

    println!("Operation failed");
    result
}

#[no_mangle]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    {
        let mut y = Y.lock().unwrap();
        *y = local_y;
    }
    let result = multi_stage(x, z);
    println!("Result: {}", result);
}
