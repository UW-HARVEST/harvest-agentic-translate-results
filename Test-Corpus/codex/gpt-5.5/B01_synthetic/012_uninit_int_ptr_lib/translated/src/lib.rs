use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[allow(non_snake_case)]
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    unsafe {
        printf(c"%d\n".as_ptr(), *intNumber);
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let data: *mut c_int;
    unsafe {
        core::arch::asm!(
            "sub rsp, 16",
            "mov {data}, [rsp + 8]",
            "add rsp, 16",
            data = lateout(reg) data,
            options(preserves_flags)
        );
    }
    printIntPtrLine(data);
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let mut data: c_int;
    data = 5;
    let data_addr: *mut c_int;
    data_addr = &mut data;
    printIntPtrLine(data_addr);
}

#[allow(non_snake_case)]
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
