use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

#[cold]
unsafe fn force_segfault() -> ! {
    let address = 0_usize;
    unsafe {
        std::arch::asm!(
            "mov byte ptr [{address}], 0",
            address = in(reg) address,
            options(noreturn, nostack)
        );
    }
}

unsafe fn fma_array_impl(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    if len > 0 && (out.is_null() || mul1.is_null() || mul2.is_null() || add.is_null()) {
        unsafe {
            force_segfault();
        }
    }

    for i in 0..len {
        let index = i as usize;
        let value = unsafe {
            (*mul1.wrapping_add(index))
                .wrapping_mul(*mul2.wrapping_add(index))
                .wrapping_add(*add.wrapping_add(index))
        };
        unsafe {
            *out.wrapping_add(index) = value;
        }
    }
}

unsafe fn driver_impl(out: *mut c_int, len: c_int) {
    unsafe {
        fma_array_impl(out, out, out, out, len);
    }
    for i in 0..len {
        unsafe {
            printf(c"%d\n".as_ptr(), *out.wrapping_add(i as usize));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    unsafe {
        fma_array_impl(out, mul1, mul2, add, len);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(out: *mut c_int, len: c_int) {
    unsafe {
        driver_impl(out, len);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    let mut data = [0_i32; 100];
    let mut count = 0;

    while count < data.len() {
        if unsafe { scanf(c"%d".as_ptr(), &mut data[count]) } != 1 {
            break;
        }
        count += 1;
    }

    unsafe {
        driver_impl(data.as_mut_ptr(), count as c_int);
    }
    0
}
