use std::alloc::{self, Layout};
use std::os::raw::{c_int, c_size_t};
use std::ptr::NonNull;

const FLAG_READ: c_int = 0b00000001;
const FLAG_WRITE: c_int = 0b00000010;
const FLAG_EXECUTE: c_int = 0b00000100;
const FLAG_DELETE: c_int = 0b00001000;

static MATRIX: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

struct DynamicArray {
    data: NonNull<c_int>,
    size: usize,
    capacity: usize,
}

impl DynamicArray {
    fn new(initial_capacity: usize) -> Option<Box<Self>> {
        let layout = Layout::array::<c_int>(initial_capacity).ok()?;
        let ptr = unsafe { alloc::alloc(layout) };
        let data = NonNull::new(ptr as *mut c_int)?;
        
        Some(Box::new(DynamicArray {
            data,
            size: 0,
            capacity: initial_capacity,
        }))
    }

    fn expand(&mut self) -> bool {
        let new_capacity = self.capacity.checked_mul(2)?;
        let old_layout = Layout::array::<c_int>(self.capacity).unwrap();
        let new_layout = match Layout::array::<c_int>(new_capacity) {
            Ok(l) => l,
            Err(_) => return false,
        };
        
        let new_ptr = unsafe {
            alloc::realloc(self.data.as_ptr() as *mut u8, old_layout, new_layout.size())
        };
        
        match NonNull::new(new_ptr as *mut c_int) {
            Some(ptr) => {
                self.data = ptr;
                self.capacity = new_capacity;
                true
            }
            None => false,
        }
    }

    fn add(&mut self, value: c_int) -> bool {
        if self.size >= self.capacity {
            if !self.expand() {
                return false;
            }
        }
        
        unsafe {
            self.data.as_ptr().add(self.size).write(value);
        }
        self.size += 1;
        true
    }
}

impl Drop for DynamicArray {
    fn drop(&mut self) {
        let layout = Layout::array::<c_int>(self.capacity).unwrap();
        unsafe {
            alloc::dealloc(self.data.as_ptr() as *mut u8, layout);
        }
    }
}

fn process_flags(flags: c_int) -> c_int {
    let read_enabled = (flags & FLAG_READ) != 0;
    let write_enabled = (flags & FLAG_WRITE) != 0;
    let execute_enabled = (flags & FLAG_EXECUTE) != 0;
    let delete_enabled = (flags & FLAG_DELETE) != 0;
    
    read_enabled as c_int + write_enabled as c_int + execute_enabled as c_int + delete_enabled as c_int
}

fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;
    for row in &MATRIX {
        for &val in row {
            sum = sum.wrapping_add(val);
        }
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn matrixsum(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;
    
    let mut permissions: c_int = 0b0000;
    
    let valid1 = param1 != 0;
    let valid2 = param2 != 0;
    let valid3 = param3 != 0;
    let valid4 = param4 != 0;
    
    if valid1 {
        permissions |= FLAG_READ;
    }
    if valid2 {
        permissions |= FLAG_WRITE;
    }
    if valid3 {
        permissions |= FLAG_EXECUTE;
    }
    if valid4 {
        permissions |= FLAG_DELETE;
    }
    
    let mut arr = match DynamicArray::new(2) {
        Some(a) => a,
        None => return -1,
    };
    
    if !arr.add(param1) {
        return -1;
    }
    if !arr.add(param2) {
        return -1;
    }
    if !arr.add(param3) {
        return -1;
    }
    if !arr.add(param4) {
        return -1;
    }
    
    let mut sum: c_int = 0;
    for i in 0..arr.size {
        sum = sum.wrapping_add(unsafe { arr.data.as_ptr().add(i).read() });
    }
    
    let flag_count = process_flags(permissions);
    let matrix_sum = calculate_matrix_checksum();
    
    let result = sum.wrapping_mul(hex_multiplier)
        .wrapping_add(flag_count.wrapping_mul(hex_base))
        .wrapping_add(matrix_sum & 0xFFF);
    
    result
}
