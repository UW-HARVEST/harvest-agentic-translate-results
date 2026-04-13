use std::collections::HashMap;

static mut INTMAP: Option<HashMap<i32, i32>> = None;
static mut INTMAP_DEFAULT: i32 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn hm_geti(num: i32) {
    unsafe {
        INTMAP = Some(HashMap::new());
        INTMAP_DEFAULT = -2;
        
        for i in (0..num).step_by(2) {
            if let Some(ref mut map) = INTMAP {
                map.insert(i, i * 5);
            }
        }
        
        for i in 0..num {
            if let Some(ref map) = INTMAP {
                if i & 1 == 1 {
                    assert_eq!(map.get(&i).copied().unwrap_or(INTMAP_DEFAULT), -2);
                } else {
                    assert_eq!(map.get(&i).copied().unwrap_or(INTMAP_DEFAULT), i * 5);
                }
            }
        }
        
        for i in (0..num).step_by(2) {
            if let Some(ref mut map) = INTMAP {
                map.insert(i, i * 3);
            }
        }
        
        for i in 0..num {
            if let Some(ref map) = INTMAP {
                if i & 1 == 1 {
                    assert_eq!(map.get(&i).copied().unwrap_or(INTMAP_DEFAULT), -2);
                } else {
                    assert_eq!(map.get(&i).copied().unwrap_or(INTMAP_DEFAULT), i * 3);
                }
            }
        }
        
        for i in (2..num).step_by(4) {
            if let Some(ref mut map) = INTMAP {
                map.remove(&i);
            }
        }
        
        for i in 0..num {
            if let Some(ref map) = INTMAP {
                if i & 3 != 0 {
                    assert_eq!(map.get(&i).copied().unwrap_or(INTMAP_DEFAULT), -2);
                } else {
                    assert_eq!(map.get(&i).copied().unwrap_or(INTMAP_DEFAULT), i * 3);
                }
            }
        }
        
        for i in 0..num {
            if let Some(ref mut map) = INTMAP {
                map.remove(&i);
            }
        }
        
        for i in 0..num {
            if let Some(ref map) = INTMAP {
                assert_eq!(map.get(&i).copied().unwrap_or(INTMAP_DEFAULT), -2);
            }
        }
        
        INTMAP = None;
        
        INTMAP = Some(HashMap::new());
        for i in (0..num).step_by(2) {
            if let Some(ref mut map) = INTMAP {
                map.insert(i, i * 3);
            }
        }
        INTMAP = None;
    }
}