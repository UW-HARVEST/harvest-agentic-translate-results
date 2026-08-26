use std::io::{self, Read, Write};

fn main() {
    let mut data = [b' '];
    let _ = io::stdin().read(&mut data);

    let result = (data[0] as i8).wrapping_add(1);
    let promoted = result as i32 as u32;

    let _ = writeln!(io::stdout(), "{promoted:02x}");
}
