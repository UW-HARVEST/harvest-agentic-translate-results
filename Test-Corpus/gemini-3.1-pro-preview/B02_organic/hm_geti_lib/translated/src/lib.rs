use std::collections::HashMap;
use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct StbdsStruct {
    pub key: c_int,
    pub b: c_int,
    pub c: c_int,
    pub d: c_int,
}

#[repr(C)]
pub struct StbdsStruct2 {
    pub key: [c_int; 2],
    pub b: c_int,
    pub c: c_int,
    pub d: c_int,
}

static mut BUFFER: [u8; 256] = [0; 256];

#[unsafe(no_mangle)]
pub extern "C" fn strkey(n: c_int) -> *mut c_char {
    use std::io::Write;
    unsafe {
        let mut cursor = std::io::Cursor::new(&mut BUFFER[..]);
        let _ = write!(cursor, "test_{}\0", n);
        BUFFER.as_mut_ptr() as *mut c_char
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hm_geti(num: c_int) {
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();
    let default_val = -2;

    let i = 1;
    assert!(!intmap.contains_key(&i));
    
    assert!(!intmap.contains_key(&i));
    assert_eq!(intmap.get(&i).copied().unwrap_or(default_val), -2);

    let mut i_val = 0;
    while i_val < num {
        intmap.insert(i_val, i_val * 5);
        i_val += 2;
    }

    for i in 0..num {
        if (i & 1) != 0 {
            assert_eq!(intmap.get(&i).copied().unwrap_or(default_val), -2);
        } else {
            assert_eq!(intmap.get(&i).copied().unwrap_or(default_val), i * 5);
        }
        
        if (i & 1) != 0 {
            assert_eq!(intmap.get(&i).copied().unwrap_or(default_val), -2);
        } else {
            assert_eq!(intmap.get(&i).copied().unwrap_or(default_val), i * 5);
        }
    }

    let mut i_val = 0;
    while i_val < num {
        intmap.insert(i_val, i_val * 3);
        i_val += 2;
    }

    for i in 0..num {
        if (i & 1) != 0 {
            assert_eq!(intmap.get(&i).copied().unwrap_or(default_val), -2);
        } else {
            assert_eq!(intmap.get(&i).copied().unwrap_or(default_val), i * 3);
        }
    }

    let mut i_val = 2;
    while i_val < num {
        intmap.remove(&i_val);
        i_val += 4;
    }

    for i in 0..num {
        if (i & 3) != 0 {
            assert_eq!(intmap.get(&i).copied().unwrap_or(default_val), -2);
        } else {
            assert_eq!(intmap.get(&i).copied().unwrap_or(default_val), i * 3);
        }
    }

    for i in 0..num {
        intmap.remove(&i);
    }

    for i in 0..num {
        assert_eq!(intmap.get(&i).copied().unwrap_or(default_val), -2);
    }

    intmap.clear();

    let mut i_val = 0;
    while i_val < num {
        intmap.insert(i_val, i_val * 3);
        i_val += 2;
    }

    intmap.clear();
}
