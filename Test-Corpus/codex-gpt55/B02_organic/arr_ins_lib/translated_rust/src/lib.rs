use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn arr_ins(num: c_int) {
    for i in 0..5_usize {
        let mut arr = Vec::with_capacity(4);
        arr.push(1);
        arr.push(2);
        arr.push(3);
        arr.push(4);

        arr.insert(i, num);

        assert_eq!(arr[i], num);
        if i < 4 {
            assert_eq!(arr[4], 4);
        }
    }
}
