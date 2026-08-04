use libc::{c_char, c_double};

#[repr(C)]
union RawDouble {
    x: u64,
    f: c_double,
}

fn driver(f: c_double) {
    let u = RawDouble { f };

    unsafe {
        libc::printf(
            b"%llx %a %.4f\n\0".as_ptr().cast::<c_char>(),
            u.x,
            f,
            f,
        );
    }
}

fn main() {
    let mut f: c_double = 0.0f32.into();

    unsafe {
        libc::scanf(b"%lf\0".as_ptr().cast::<c_char>(), &mut f);
    }

    driver(f);
}
