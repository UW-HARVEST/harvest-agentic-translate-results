use std::os::raw::c_int;

fn print_int_ptr_line(int_number: Option<&c_int>) {
    println!("{}", int_number.unwrap());
}

fn bad() {
    let data: Option<&c_int> = None;
    print_int_ptr_line(data);
}

fn good() {
    let data: c_int = 5;
    print_int_ptr_line(Some(&data));
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
