use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_int;

struct StbdsStringArena {
    strings: Vec<CString>,
}

impl StbdsStringArena {
    fn new() -> Self {
        Self { strings: Vec::new() }
    }

    fn stralloc(&mut self, s: &str) -> *mut i8 {
        let c = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
        let ptr = c.as_ptr() as *mut i8;
        self.strings.push(c);
        ptr
    }

    fn strreset(&mut self) {
        self.strings.clear();
    }
}

#[repr(C)]
struct StrMapEntry {
    key: CString,
    value: c_int,
}

fn strkey(n: c_int) -> String {
    format!("test_{}", n)
}

#[unsafe(no_mangle)]
pub extern "C" fn str_dups(num: c_int) {
    let mut sa = StbdsStringArena::new();

    for i in 0..num {
        let s = strkey(i);
        let _ = sa.stralloc(&s);
    }
    sa.strreset();

    let mut strmap: Vec<StrMapEntry> = Vec::new();
    let mut index: HashMap<Vec<u8>, usize> = HashMap::new();

    let key = CString::new("a").unwrap();
    let value = num;

    let owned_key = CString::new(key.as_bytes()).unwrap();
    let entry = StrMapEntry { key: owned_key, value };
    strmap.push(entry);
    index.insert(strmap[0].key.as_bytes().to_vec(), 0);

    assert_eq!(strmap[0].key.as_bytes()[0], b'a');
    assert_ne!(strmap[0].key.as_ptr(), key.as_ptr());
    assert_eq!(strmap[0].value, value);

    for i in 0..strmap.len() {
        println!("{} {}", strmap[i].key.to_string_lossy(), strmap[i].value);
    }

    drop(index);
    drop(strmap);
}
