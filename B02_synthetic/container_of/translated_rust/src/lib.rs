#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Test {
    pub a: i32,
    pub b: i32,
}

#[no_mangle]
pub extern "C" fn find_container_of_a(i: *mut i32) -> *mut Test {
    // container_of(i, struct test, a): offset of a is 0
    i as *mut Test
}

#[no_mangle]
pub extern "C" fn find_container_of_b(i: *mut i32) -> *mut Test {
    // container_of(i, struct test, b): offset of b is sizeof(int) = 4
    unsafe { (i as *mut u8).sub(4) as *mut Test }
}

extern "C" {
    fn atoi(s: *const i8) -> i32;
    fn printf(fmt: *const i8, ...) -> i32;
}

/// Exported `main` for the shared library to match C .so symbol table.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main(_argc: i32, argv: *mut *mut i8) -> i32 {
    let a = atoi(*argv.offset(1));
    let b = atoi(*argv.offset(2));

    let mut t: Test = std::mem::zeroed();
    t.a = a;
    t.b = b;

    let result = (*find_container_of_a(&mut t.a)).a + (*find_container_of_b(&mut t.b)).b;
    printf(b"%d\n\0".as_ptr() as *const i8, result);
    0
}
