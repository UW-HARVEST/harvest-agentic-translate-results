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

static MATRIX: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

const FLAG_READ: c_int = 0b0000_0001;
const FLAG_WRITE: c_int = 0b0000_0010;
const FLAG_EXECUTE: c_int = 0b0000_0100;
const FLAG_DELETE: c_int = 0b0000_1000;

struct DynamicArray {
    data: Vec<c_int>,
    capacity: usize,
}

impl DynamicArray {
    fn new(initial_capacity: usize) -> Self {
        DynamicArray {
            data: Vec::with_capacity(initial_capacity),
            capacity: initial_capacity,
        }
    }

    fn add_element(&mut self, value: c_int) -> bool {
        if self.data.len() >= self.capacity {
            // Mimic expand_array: double capacity
            let new_capacity = self.capacity * 2;
            self.data.reserve(new_capacity - self.data.len());
            self.capacity = new_capacity;
        }
        self.data.push(value);
        true
    }

    fn size(&self) -> usize {
        self.data.len()
    }
}

fn process_flags(flags: c_int) -> c_int {
    let has_read = flags & FLAG_READ;
    let read_enabled = if has_read != 0 { 1 } else { 0 };

    let has_write = flags & FLAG_WRITE;
    let write_enabled = if has_write != 0 { 1 } else { 0 };

    let has_execute = flags & FLAG_EXECUTE;
    let execute_enabled = if has_execute != 0 { 1 } else { 0 };

    let has_delete = flags & FLAG_DELETE;
    let delete_enabled = if has_delete != 0 { 1 } else { 0 };

    read_enabled + write_enabled + execute_enabled + delete_enabled
}

fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;
    for i in 0..3 {
        for j in 0..4 {
            sum += MATRIX[i][j];
        }
    }
    sum
}

#[no_mangle]
pub extern "C" fn matrixsum(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0b0000;

    let valid1 = if param1 != 0 { 1 } else { 0 };
    let valid2 = if param2 != 0 { 1 } else { 0 };
    let valid3 = if param3 != 0 { 1 } else { 0 };
    let valid4 = if param4 != 0 { 1 } else { 0 };

    if valid1 != 0 {
        permissions |= FLAG_READ;
    }
    if valid2 != 0 {
        permissions |= FLAG_WRITE;
    }
    if valid3 != 0 {
        permissions |= FLAG_EXECUTE;
    }
    if valid4 != 0 {
        permissions |= FLAG_DELETE;
    }

    let mut arr = DynamicArray::new(2);
    arr.add_element(param1);
    arr.add_element(param2);
    arr.add_element(param3);
    arr.add_element(param4);

    let mut sum: c_int = 0;
    for i in 0..arr.size() {
        sum += arr.data[i];
    }

    let flag_count = process_flags(permissions);
    let matrix_sum = calculate_matrix_checksum();

    (sum * hex_multiplier) + (flag_count * hex_base) + (matrix_sum & 0xFFF)
}
