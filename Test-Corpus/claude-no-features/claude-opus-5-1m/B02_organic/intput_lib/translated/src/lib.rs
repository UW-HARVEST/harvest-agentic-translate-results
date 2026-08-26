use std::collections::HashMap;
use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn intput(num: c_int) {
    // Translation of:
    //   struct { int key; int value; } *intmap = NULL;
    //   intmap = NULL;
    //   hmput(intmap, num, 7);
    //   hmput(intmap, 11, 3);
    //   hmput(intmap, 9, num);
    //   STBDS_ASSERT(hmget(intmap, 9) == num);
    //   STBDS_ASSERT(hmget(intmap, 11) == 3);
    //   STBDS_ASSERT(hmget(intmap, num) == 7);
    //
    // The stbds hashmap on insert with an existing key overwrites the value;
    // on lookup of an absent key, it returns the default value (0 here).
    // We model this with a Rust HashMap and a default of 0 on missing keys.

    let mut intmap: HashMap<c_int, c_int> = HashMap::new();

    intmap.insert(num, 7);
    intmap.insert(11, 3);
    intmap.insert(9, num);

    let v9 = *intmap.get(&9).unwrap_or(&0);
    assert!(v9 == num);

    let v11 = *intmap.get(&11).unwrap_or(&0);
    assert!(v11 == 3);

    let vnum = *intmap.get(&num).unwrap_or(&0);
    assert!(vnum == 7);
}
