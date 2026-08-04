use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn arr_del(num: c_int) {
    // Mirror the C function's behavior: build a small dynamic array of
    // [num, 2, 3, 4], delete an element using two different deletion
    // strategies, and free the array. The function has no observable
    // outputs (no I/O, no return value), so producing byte-identical
    // output amounts to performing the same sequence of operations and
    // returning normally.
    for i in 0..4usize {
        // arrdel: remove element at index `i`, shifting later elements down.
        let mut arr: Vec<c_int> = Vec::new();
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        // Equivalent of stbds_arrdel(arr, i): memmove(&arr[i], &arr[i+1], ...)
        arr.remove(i);
        drop(arr);

        // arrdelswap: replace element at index `i` with last element, then
        // shrink length by 1.
        let mut arr: Vec<c_int> = Vec::new();
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        // Equivalent of stbds_arrdelswap(arr, i): arr[i] = arr[len-1]; --len
        arr.swap_remove(i);
        drop(arr);
    }
}
