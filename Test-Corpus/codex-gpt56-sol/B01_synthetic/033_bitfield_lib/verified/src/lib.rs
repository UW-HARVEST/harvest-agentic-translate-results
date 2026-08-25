use std::ffi::{c_char, c_int, c_uchar, c_uint};

const PRINT_FORMAT: &[u8] = b"%u %u %d %d\n\0";

#[repr(C)]
pub struct Foo {
    bits: c_uint,
    z: c_int,
}

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const Foo) {
    if foo.is_null() {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            std::arch::asm!(
                "mov eax, dword ptr [{pointer}]",
                pointer = in(reg) foo,
                out("eax") _,
                options(nostack, readonly)
            );
        }
        #[cfg(not(target_arch = "x86_64"))]
        unsafe {
            std::ptr::read_volatile(foo.cast::<u8>());
        }
    }
    let foo = unsafe { &*foo };
    let x = foo.bits & 0x3;
    let y = (foo.bits >> 2) & 0x7;
    let b = ((foo.bits >> 5) & 0x1) as c_int;

    unsafe {
        printf(PRINT_FORMAT.as_ptr().cast(), x, y, b, foo.z);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: c_uchar, z: c_int) {
    let foo = Foo {
        bits: (x & 0x3) | ((y & 0x7) << 2) | (((b as c_uint) & 0x1) << 5),
        z,
    };

    unsafe {
        print_foo(&foo);
    }
}
