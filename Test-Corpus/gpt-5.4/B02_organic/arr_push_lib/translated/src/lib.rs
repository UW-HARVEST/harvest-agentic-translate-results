use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn arr_push(num: c_int) {
    let mut arr: Vec<c_int> = Vec::new();
    let mut i = 0;
    while i < num {
        let mut j = 0;
        while j < i {
            arr.push(j);
            j += 1;
        }
        arr = Vec::new();
        i += 50;
    }
}
