use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn arr_ins(num: c_int) {
    for i in 0..5 {
        let mut arr = vec![1, 2, 3, 4];
        arr.insert(i, num);
        assert_eq!(arr[i], num);
        if i < 4 {
            assert_eq!(arr[4], 4);
        }
    }
}
