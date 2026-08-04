use std::collections::HashMap;
use std::ffi::{c_char, c_int, CStr, CString};
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
pub extern "C" fn sh_puts(num: c_int) {
    unsafe {
        let mut sa: Vec<CString> = Vec::new();
        
        for i in 0..num {
            let key = strkey(i);
            let cstr = CStr::from_ptr(key);
            if let Ok(s) = cstr.to_str() {
                if let Ok(owned) = CString::new(s) {
                    sa.push(owned);
                }
            }
        }
        
        sa.clear();
        
        let mut strmap: HashMap<String, c_int> = HashMap::new();
        
        let s_key = "a".to_string();
        let s_value = num;
        
        strmap.insert(s_key.clone(), s_value);
        
        assert!(strmap.contains_key("a"));
        assert_eq!(strmap.get("a"), Some(&s_value));
        
        for (key, value) in &strmap {
            println!("{} {}", key, value);
        }
        
        drop(strmap);
    }
}
