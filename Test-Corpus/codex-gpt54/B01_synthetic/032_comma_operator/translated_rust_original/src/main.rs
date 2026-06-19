use libc::c_int;

fn driver(x: c_int) {
    let mut i: c_int = 0;
    let mut j: c_int = 0;

    while i < x {
        println!("{} {}", i, j);
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        libc::scanf(c"%d".as_ptr(), &mut x);
    }

    driver(x);
}
