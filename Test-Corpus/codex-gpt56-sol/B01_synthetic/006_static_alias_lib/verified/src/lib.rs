use std::ffi::{c_char, c_int};

static mut INNER: c_int = 1;
const INTEGER_LINE_FORMAT: &[u8] = b"%d\n\0";

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    #[cfg(not(target_arch = "x86_64"))]
    fn raise(signal: c_int) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    if outer.is_null() {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let ignored: c_int;
            std::arch::asm!(
                "mov {value:e}, dword ptr [{address}]",
                value = out(reg) ignored,
                address = in(reg) outer,
                options(nostack, readonly, preserves_flags)
            );
            std::hint::black_box(ignored);
        }
        #[cfg(not(target_arch = "x86_64"))]
        unsafe {
            raise(11);
        }
    }

    let inner = &raw mut INNER;

    if unsafe { *outer >= *inner } {
        unsafe {
            *inner += *outer;
        }
        inner
    } else {
        unsafe {
            *outer += *inner;
        }
        outer
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    let mut initial_value = initial_value;
    let mut running_sum = &raw mut initial_value;
    let mut i = 0;

    while i < iterations {
        running_sum = unsafe { static_alias(running_sum) };
        unsafe {
            printf(INTEGER_LINE_FORMAT.as_ptr().cast::<c_char>(), *running_sum);
        }
        i += 1;
    }
}
