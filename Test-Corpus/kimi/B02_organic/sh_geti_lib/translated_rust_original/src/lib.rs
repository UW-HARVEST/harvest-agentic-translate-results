use std::collections::HashMap;
use std::ffi::{c_char, c_int, CStr, CString};
use std::os::raw::c_void;
use std::ptr;

static mut BUFFER: [u8; 256] = [0; 256];

fn strkey(n: i32) -> String {
    format!("test_{}", n)
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_geti(num: c_int) {
    let num = num as i32;
    
    for j in 0..2 {
        let mut strmap: HashMap<String, i32> = HashMap::new();
        
        let default_val = -2;
        
        if j == 0 {
        } else {
        }
        
        for i in (0..num).step_by(2) {
            let key = strkey(i);
            strmap.insert(key, i * 3);
        }
        
        for (key, value) in &strmap {
            println!("{} {}", key, value);
        }
        
        for i in 0..num {
            let key = strkey(i);
            let expected = if i & 1 == 1 { default_val } else { i * 3 };
            let actual = strmap.get(&key).copied().unwrap_or(default_val);
            assert_eq!(actual, expected);
        }
        
        for i in (2..num).step_by(4) {
            let key = strkey(i);
            strmap.remove(&key);
        }
        
        for i in 0..num {
            let key = strkey(i);
            let expected = if i & 3 != 0 { default_val } else { i * 3 };
            let actual = strmap.get(&key).copied().unwrap_or(default_val);
            assert_eq!(actual, expected);
        }
        
        for i in 0..num {
            let key = strkey(i);
            strmap.remove(&key);
        }
        
        for i in 0..num {
            let key = strkey(i);
            let actual = strmap.get(&key).copied().unwrap_or(default_val);
            assert_eq!(actual, default_val);
        }
    }
}