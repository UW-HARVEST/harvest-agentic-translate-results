use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn arr_del(num: c_int) {
    for i in 0..4usize {
        let mut arr = vec![num, 2, 3, 4];
        arr.remove(i);

        let mut arr = vec![num, 2, 3, 4];
        let last = arr.len() - 1;
        arr.swap(i, last);
        arr.pop();
    }
}
