use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_int};
use std::os::raw::c_void;

static mut BUFFER: [u8; 256] = [0; 256];

fn strkey(n: i32) -> &'static str {
    unsafe {
        let s = format!("test_{}", n);
        let bytes = s.as_bytes();
        let len = bytes.len().min(255);
        BUFFER[..len].copy_from_slice(&bytes[..len]);
        BUFFER[len] = 0;
        CStr::from_ptr(BUFFER.as_ptr() as *const c_char)
            .to_str()
            .unwrap_or("")
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn str_put(num: c_int) {
    let num = num as i32;
    let mut strmap: HashMap<String, i32> = HashMap::new();
    let mut sa: Vec<String> = Vec::new();

    for i in 0..num {
        sa.push(strkey(i).to_string());
    }
    sa.clear();

    let s_key = "a".to_string();
    let s_value = num;
    strmap.insert(s_key.clone(), s_value);

    assert!(strmap.contains_key("a"));
    assert_eq!(strmap.get("a"), Some(&s_value));

    for (key, value) in &strmap {
        println!("{} {}", key, value);
    }
}
