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

#[repr(C)]
#[derive(Copy, Clone)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    for byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

/// Translation of `void driver(int floors)` from driver.c.
#[no_mangle]
pub extern "C" fn driver(floors: c_int) {
    // Zero-initialize the struct, equivalent to `house_t house = {0};` in C.
    // Using MaybeUninit::zeroed ensures any padding bytes are zeroed too,
    // matching the C semantics for `print_hex` reading the raw bytes.
    let mut house: HouseT = unsafe { std::mem::MaybeUninit::<HouseT>::zeroed().assume_init() };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    let size = std::mem::size_of::<HouseT>();
    let ptr = &house as *const HouseT as *const u8;
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(ptr, size) };
    print_hex(bytes);
}
