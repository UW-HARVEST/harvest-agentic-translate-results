use std::mem::offset_of;

#[repr(C)]
pub struct Test {
    pub a: i32,
    pub b: i32,
}

#[no_mangle]
pub extern "C" fn find_container_of_a(i: *const i32) -> *const Test {
    unsafe { (i as *const u8).sub(offset_of!(Test, a)) as *const Test }
}

#[no_mangle]
pub extern "C" fn find_container_of_b(i: *const i32) -> *const Test {
    unsafe { (i as *const u8).sub(offset_of!(Test, b)) as *const Test }
}

#[cfg(not(test))]
extern "C" {
    fn atoi(s: *const i8) -> i32;
    fn printf(fmt: *const i8, ...) -> i32;
    fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8;
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main(_argc: i32, argv: *const *const i8) -> i32 {
    unsafe {
        let a = atoi(*argv.add(1));
        let b = atoi(*argv.add(2));

        let mut t: Test = std::mem::zeroed();
        memset(&mut t as *mut Test as *mut u8, 0, std::mem::size_of::<Test>());
        t.a = a;
        t.b = b;

        let sum = (*find_container_of_a(&t.a)).a.wrapping_add((*find_container_of_b(&t.b)).b);
        printf(b"%d\n\0".as_ptr() as *const i8, sum);
        0
    }
}
