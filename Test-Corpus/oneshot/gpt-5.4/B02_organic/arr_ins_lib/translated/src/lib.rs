use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn arr_ins(num: c_int) {
    let mut arr: Vec<c_int> = Vec::new();

    for i in 0..5usize {
        arr.clear();
        arr.push(1);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        arr.insert(i, num);
        assert!(arr[i] == num);
        if i < 4 {
            assert!(arr[4] == 4);
        }
    }
}
