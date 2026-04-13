use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_int};
use std::os::raw::c_void;
use std::ptr;

static mut BUFFER: [u8; 256] = [0; 256];

unsafe fn strkey(n: c_int) -> *const c_char {
    let s = format!("test_{}", n);
    let bytes = s.as_bytes();
    let len = bytes.len().min(255);
    BUFFER[..len].copy_from_slice(&bytes[..len]);
    BUFFER[len] = 0;
    BUFFER.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn str_dups(num: c_int) {
    unsafe {
        let mut strmap: HashMap<String, i32> = HashMap::new();
        let mut arena: Vec<String> = Vec::new();

        for i in 0..num {
            let key = CStr::from_ptr(strkey(i)).to_string_lossy().into_owned();
            arena.push(key);
        }
        arena.clear();

        let s_key = "a".to_string();
        let s_value = num;
        strmap.insert(s_key.clone(), s_value);

        assert!(strmap.contains_key("a"));
        assert_eq!(strmap.get("a").copied().unwrap(), s_value);

        for (key, value) in &strmap {
            println!("{} {}", key, value);
        }
    }
}
