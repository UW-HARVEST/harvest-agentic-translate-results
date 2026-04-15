use std::collections::HashMap;
use std::io::Write;
use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct stbds_struct {
    pub key: c_int,
    pub b: c_int,
    pub c: c_int,
    pub d: c_int,
}

#[repr(C)]
pub struct stbds_struct2 {
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
pub extern "C" fn intput(num: c_int) {
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();

    intmap.insert(num, 7);
    intmap.insert(11, 3);
    intmap.insert(9, num);

    assert_eq!(intmap.get(&9).copied().unwrap_or(0), num);
    assert_eq!(intmap.get(&11).copied().unwrap_or(0), 3);
    assert_eq!(intmap.get(&num).copied().unwrap_or(0), 7);
}
