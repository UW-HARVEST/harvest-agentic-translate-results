use std::ffi::{c_char, c_int, CString};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const PRINTF_FORMAT: &[u8] = b"%s %d\n\0";
const SOURCE_KEY: &[u8] = b"a\0";

struct StringArena {
    storage: Vec<CString>,
}

impl StringArena {
    fn new() -> Self {
        Self { storage: Vec::new() }
    }

    fn stralloc(&mut self, value: &str) {
        self.storage.push(CString::new(value).unwrap());
    }

    fn strreset(&mut self) {
        self.storage.clear();
    }
}

struct StrMapEntry {
    key: CString,
    value: c_int,
}

#[inline]
fn c_assert(condition: bool) {
    if !condition {
        std::process::abort();
    }
}

fn strkey(n: c_int) -> String {
    format!("test_{n}")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_puts(num: c_int) {
    let mut sa = StringArena::new();
    let mut i: c_int = 0;

    while i < num {
        sa.stralloc(&strkey(i));
        i += 1;
    }
    sa.strreset();

    let source_key = SOURCE_KEY.as_ptr() as *const c_char;
    let strmap = [StrMapEntry {
        key: CString::new("a").unwrap(),
        value: num,
    }];

    c_assert(strmap[0].key.as_bytes()[0] == b'a');
    c_assert(strmap[0].key.as_ptr() != source_key);
    c_assert(strmap[0].value == num);

    for entry in &strmap {
        let _ = printf(
            PRINTF_FORMAT.as_ptr() as *const c_char,
            entry.key.as_ptr(),
            entry.value,
        );
    }
}
