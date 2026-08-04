use std::collections::HashMap;
use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn intput(num: c_int) {
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();
    intmap.insert(num, 7);
    intmap.insert(11, 3);
    intmap.insert(9, num);
    assert_eq!(intmap.get(&9).copied(), Some(num));
    assert_eq!(intmap.get(&11).copied(), Some(3));
    assert_eq!(intmap.get(&num).copied(), Some(7));
}
