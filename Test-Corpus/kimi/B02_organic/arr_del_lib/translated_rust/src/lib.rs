use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn arr_del(num: c_int) {
    for i in 0..4 {
        let mut arr: Vec<c_int> = Vec::new();
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        arr.remove(i);
        drop(arr);

        let mut arr: Vec<c_int> = Vec::new();
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        arr.swap_remove(i);
        drop(arr);
    }
}