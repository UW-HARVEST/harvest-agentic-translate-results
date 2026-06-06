use std::collections::HashMap;
use std::ffi::c_int;

/// Insert (num, 7), (11, 3), (9, num) into a hashmap and assert their values.
///
/// This mirrors the C `intput` function which uses the stb_ds dynamic hashmap.
/// The observable behavior of this function is limited to the assertions it
/// performs; it produces no output and returns nothing. We therefore implement
/// it with a plain `HashMap` and reproduce the assertion order/semantics
/// exactly as in the C source.
#[unsafe(no_mangle)]
pub extern "C" fn intput(num: c_int) {
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();

    intmap.insert(num, 7);
    intmap.insert(11, 3);
    intmap.insert(9, num);

    // STBDS_ASSERT(hmget(intmap, 9) == num);
    assert!(*intmap.get(&9).unwrap() == num);
    // STBDS_ASSERT(hmget(intmap, 11) == 3);
    assert!(*intmap.get(&11).unwrap() == 3);
    // STBDS_ASSERT(hmget(intmap, num) == 7);
    assert!(*intmap.get(&num).unwrap() == 7);
}
