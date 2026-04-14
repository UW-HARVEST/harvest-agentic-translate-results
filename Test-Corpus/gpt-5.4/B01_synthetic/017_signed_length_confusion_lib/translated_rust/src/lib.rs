use std::os::raw::c_int;

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    let source = "A".repeat(99);
    let mut dest = String::new();
    if data < 100 {
        let count = if data < 0 { 0 } else { data as usize };
        dest.push_str(&source[..count]);
    }
    print_line(Some(&dest));
}
