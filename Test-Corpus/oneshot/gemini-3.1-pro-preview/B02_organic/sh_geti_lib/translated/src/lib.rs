use std::collections::HashMap;
use std::os::raw::c_int;

fn strkey(n: i32) -> String {
    format!("test_{}", n)
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_geti(num: c_int) {
    for _ in 0..2 {
        let mut strmap: HashMap<String, i32> = HashMap::new();
        let default_val = -2;

        assert!(!strmap.contains_key("foo"));

        let mut i = 0;
        while i < num {
            strmap.insert(strkey(i), i * 3);
            i += 2;
        }

        for idx in 0..num {
            if idx % 2 == 0 {
                let k = strkey(idx);
                if let Some(&v) = strmap.get(&k) {
                    println!("{} {}", k, v);
                }
            }
        }

        for i in 0..num {
            let val = strmap.get(&strkey(i)).copied().unwrap_or(default_val);
            if (i & 1) != 0 {
                assert_eq!(val, -2);
            } else {
                assert_eq!(val, i * 3);
            }
        }

        let mut i = 2;
        while i < num {
            strmap.remove(&strkey(i));
            i += 4;
        }

        for i in 0..num {
            let val = strmap.get(&strkey(i)).copied().unwrap_or(default_val);
            if (i & 3) != 0 {
                assert_eq!(val, -2);
            } else {
                assert_eq!(val, i * 3);
            }
        }

        for i in 0..num {
            strmap.remove(&strkey(i));
        }

        for i in 0..num {
            let val = strmap.get(&strkey(i)).copied().unwrap_or(default_val);
            assert_eq!(val, -2);
        }
    }
}
