use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_int;

struct StringArena {
    storage: Vec<CString>,
}

impl StringArena {
    fn new() -> Self {
        Self { storage: Vec::new() }
    }

    fn stralloc(&mut self, s: &str) {
        if let Ok(cs) = CString::new(s) {
            self.storage.push(cs);
        }
    }

    fn strreset(&mut self) {
        self.storage.clear();
    }
}

fn strkey(n: i32) -> String {
    format!("test_{}", n)
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_geti(num: c_int) {
    let num = num as i32;
    let mut sa = StringArena::new();

    for i in 0..num {
        sa.stralloc(&strkey(i));
    }
    sa.strreset();

    for _j in 0..2 {
        let default_value = -2;
        let mut strmap: HashMap<String, i32> = HashMap::new();

        assert!(!strmap.contains_key("foo"));
        assert!(!strmap.contains_key("foo"));
        assert!(!strmap.contains_key("foo"));

        let mut i = 0;
        while i < num {
            strmap.insert(strkey(i), i * 3);
            i += 2;
        }

        for (k, v) in &strmap {
            println!("{} {}", k, v);
        }

        for i in 0..num {
            let value = *strmap.get(&strkey(i)).unwrap_or(&default_value);
            if i & 1 != 0 {
                assert!(value == -2);
            } else {
                assert!(value == i * 3);
            }
        }

        let mut i = 2;
        while i < num {
            strmap.remove(&strkey(i));
            i += 4;
        }

        for i in 0..num {
            let value = *strmap.get(&strkey(i)).unwrap_or(&default_value);
            if i & 3 != 0 {
                assert!(value == -2);
            } else {
                assert!(value == i * 3);
            }
        }

        for i in 0..num {
            strmap.remove(&strkey(i));
        }

        for i in 0..num {
            let value = *strmap.get(&strkey(i)).unwrap_or(&default_value);
            assert!(value == -2);
        }
    }
}
