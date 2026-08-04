use std::io::Write;
use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct StbdsStruct {
    pub key: c_int,
    pub b: c_int,
    pub c: c_int,
    pub d: c_int,
}

#[repr(C)]
pub struct StbdsStruct2 {
    pub key: [c_int; 2],
    pub b: c_int,
    pub c: c_int,
    pub d: c_int,
}

static mut BUFFER: [u8; 256] = [0; 256];

#[unsafe(no_mangle)]
pub extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let mut cursor = std::io::Cursor::new(&mut BUFFER[..]);
        let _ = write!(cursor, "test_{}\0", n);
        BUFFER.as_mut_ptr() as *mut c_char
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn arr_del(num: c_int) {
    for i in 0..4 {
        let mut arr = vec![num, 2, 3, 4];
        arr.remove(i);

        let mut arr2 = vec![num, 2, 3, 4];
        arr2.swap_remove(i);
    }
}
