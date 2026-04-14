use std::collections::HashMap;
use std::ffi::{CString, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn sh_puts(num: c_int) {
    let num = num as i32;

    let mut arena_strings: Vec<CString> = Vec::new();
    for i in 0..num {
        let s = format!("test_{}", i);
        arena_strings.push(CString::new(s).unwrap());
    }
    arena_strings.clear();

    {
        let key = CString::new("a").unwrap();
        let value = num;

        let mut strmap: HashMap<String, i32> = HashMap::new();
        strmap.insert(key.to_string_lossy().into_owned(), value);

        let stored = strmap.get("a").copied().unwrap_or_default();
        assert_eq!(stored, value);

        for (k, v) in &strmap {
            println!("{} {}", k, v);
        }
    }
}
