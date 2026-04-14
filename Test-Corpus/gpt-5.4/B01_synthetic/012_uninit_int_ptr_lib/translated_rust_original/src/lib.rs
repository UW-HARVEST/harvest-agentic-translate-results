use std::os::raw::c_int;

fn print_int_ptr_line(int_number: Option<&i32>) {
    let value = int_number.map_or(0, |v| *v);
    println!("{}", value);
}

fn bad() {
    let data: Option<&i32> = None;
    print_int_ptr_line(data);
}

fn good() {
    let data = 5;
    let data_addr = &data;
    print_int_ptr_line(Some(data_addr));
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
