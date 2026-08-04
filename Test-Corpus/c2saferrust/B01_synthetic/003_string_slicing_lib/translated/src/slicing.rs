
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
#[no_mangle]
pub fn slice(mystr: &str, start_ptr: Option<i32>, stop_ptr: Option<i32>) -> i32 {
    let len = mystr.len();
    let start = if let Some(start) = start_ptr {
        if start < 0 || start as usize > len {
            println!("Error: start is off the end of the string!");
            return 1;
        }
        start
    } else {
        0
    };

    let stop = if let Some(stop) = stop_ptr {
        if stop < 0 || stop as usize > len {
            println!("Error: stop is off the end of the string!");
            return 1;
        }
        if stop <= start {
            println!("Error: stop must come after start!");
            return 1;
        }
        stop
    } else {
        len as i32
    };

    let start = start as usize;
    let stop = stop as usize;

    if !mystr.is_char_boundary(start) || !mystr.is_char_boundary(stop) {
        println!("Error: indices must lie on UTF-8 character boundaries!");
        return 1;
    }

    println!("{}", &mystr[start..stop]);
    0
}

