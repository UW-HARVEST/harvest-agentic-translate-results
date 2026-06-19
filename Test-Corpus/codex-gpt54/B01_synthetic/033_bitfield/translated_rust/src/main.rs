use std::mem::MaybeUninit;

#[derive(Clone, Copy)]
struct Foo {
    x: u32,
    y: u32,
    b: bool,
    z: i32,
}

unsafe extern "C" {
    fn scanf(format: *const libc::c_char, ...) -> libc::c_int;
}

fn print_foo(foo: &Foo) {
    println!("{} {} {} {}", foo.x, foo.y, i32::from(foo.b), foo.z);
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo {
        x: x & 0b11,
        y: y & 0b111,
        b,
        z,
    };
    print_foo(&foo);
}

fn main() {
    let mut x = MaybeUninit::<libc::c_uint>::new(0);
    let mut y = MaybeUninit::<libc::c_uint>::new(0);
    let mut b = MaybeUninit::<libc::c_int>::new(0);
    let mut z = MaybeUninit::<libc::c_int>::new(0);

    unsafe {
        scanf(c"%u".as_ptr(), x.as_mut_ptr());
        scanf(c"%u".as_ptr(), y.as_mut_ptr());
        scanf(c"%d".as_ptr(), b.as_mut_ptr());
        scanf(c"%d".as_ptr(), z.as_mut_ptr());

        driver(
            x.assume_init() as u32,
            y.assume_init() as u32,
            b.assume_init() != 0,
            z.assume_init() as i32,
        );
    }
}
