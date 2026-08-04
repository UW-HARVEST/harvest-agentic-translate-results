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

use std::mem::MaybeUninit;

fn print_int_ptr_line(int_number: *const i32) {
    // Equivalent to: printf("%d\n", *intNumber);
    unsafe {
        println!("{}", *int_number);
    }
}

fn bad() {
    // Mirrors the C code's uninitialized pointer usage:
    //   int *data;
    //   printIntPtrLine(data);
    let data: MaybeUninit<*const i32> = MaybeUninit::uninit();
    let data_ptr = unsafe { data.assume_init() };
    print_int_ptr_line(data_ptr);
}

fn good() {
    let data: i32 = 5;
    let data_addr: *const i32 = &data;
    print_int_ptr_line(data_addr);
}

#[no_mangle]
pub extern "C" fn driver(use_good: i32) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
