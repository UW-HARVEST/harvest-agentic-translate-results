use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_int;

pub struct StbdsStringArena {
    storage: Vec<CString>,
}

impl StbdsStringArena {
    fn new() -> Self {
        Self { storage: Vec::new() }
    }

    fn stralloc(&mut self, s: &CStr) -> *mut c_char {
        let owned = CString::new(s.to_bytes()).unwrap_or_default();
        let ptr = owned.as_ptr() as *mut c_char;
        self.storage.push(owned);
        ptr
    }

    fn strreset(&mut self) {
        self.storage.clear();
    }
}

static mut BUFFER: [u8; 256] = [0; 256];

fn strkey_bytes(n: c_int) -> Vec<u8> {
    format!("test_{}", n).into_bytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn str_put(num: c_int) {
    let mut strmap: HashMap<String, c_int> = HashMap::new();
    let mut arena = StbdsStringArena::new();

    for i in 0..num {
        let bytes = strkey_bytes(i);
        if let Ok(cstr) = CString::new(bytes) {
            let _ = arena.stralloc(cstr.as_c_str());
        }
    }
    arena.strreset();

    strmap.insert("a".to_string(), num);

    if let Some(value) = strmap.get("a") {
        println!("a {}", value);
    }

    let _ = unsafe { &mut BUFFER };
}