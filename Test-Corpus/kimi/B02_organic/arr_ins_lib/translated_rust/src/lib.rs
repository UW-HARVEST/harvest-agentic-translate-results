use std::os::raw::c_int;

static mut ARR: Vec<c_int> = Vec::new();

#[unsafe(no_mangle)]
pub extern "C" fn arr_ins(num: c_int) {
    unsafe {
        for i in 0..5 {
            ARR.clear();
            ARR.push(1);
            ARR.push(2);
            ARR.push(3);
            ARR.push(4);
            ARR.insert(i as usize, num);
            assert_eq!(ARR[i as usize], num);
            if i < 4 {
                assert_eq!(ARR[4], 4);
            }
        }
        ARR.clear();
    }
}