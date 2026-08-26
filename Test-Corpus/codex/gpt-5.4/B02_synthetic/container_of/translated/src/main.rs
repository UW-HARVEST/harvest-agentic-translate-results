use std::ffi::CString;
use std::mem::{offset_of, zeroed};
use std::os::unix::ffi::OsStrExt;
use std::ptr;

#[repr(C)]
struct Test {
    a: libc::c_int,
    b: libc::c_int,
}

unsafe fn find_container_of_a(i: *mut libc::c_int) -> *mut Test {
    (i.cast::<u8>())
        .sub(offset_of!(Test, a))
        .cast::<Test>()
}

unsafe fn find_container_of_b(i: *mut libc::c_int) -> *mut Test {
    (i.cast::<u8>())
        .sub(offset_of!(Test, b))
        .cast::<Test>()
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();

    let arg1 = args
        .get(1)
        .map(|s| CString::new(s.as_bytes()).unwrap())
        .unwrap_or_else(|| CString::new(Vec::<u8>::new()).unwrap());
    let arg2 = args
        .get(2)
        .map(|s| CString::new(s.as_bytes()).unwrap())
        .unwrap_or_else(|| CString::new(Vec::<u8>::new()).unwrap());

    let arg1_ptr = if args.get(1).is_some() {
        arg1.as_ptr()
    } else {
        ptr::null()
    };
    let arg2_ptr = if args.get(2).is_some() {
        arg2.as_ptr()
    } else {
        ptr::null()
    };

    unsafe {
        let a = libc::atoi(arg1_ptr);
        let b = libc::atoi(arg2_ptr);

        let mut t: Test = zeroed();
        t.a = a;
        t.b = b;

        libc::printf(
            b"%d\n\0".as_ptr().cast(),
            (*find_container_of_a(ptr::addr_of_mut!(t.a))).a
                .wrapping_add((*find_container_of_b(ptr::addr_of_mut!(t.b))).b),
        );
    }
}
