use std::ffi::{c_char, c_int, c_long, c_void};
use std::ptr;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn signal(signal: c_int, handler: *const c_void) -> *const c_void;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
}

const SIGSEGV: c_int = 11;
static mut INNER: c_int = 1;

#[cfg(target_arch = "x86_64")]
unsafe fn terminate_with_sigsegv() -> ! {
    signal(SIGSEGV, ptr::null());
    let address = 0_usize;
    let value: u32;
    std::arch::asm!(
        "mov {value:e}, dword ptr [{address}]",
        address = in(reg) address,
        value = out(reg) value,
        options(nostack, readonly)
    );
    std::hint::black_box(value);
    std::hint::unreachable_unchecked()
}

#[cfg(target_arch = "aarch64")]
unsafe fn terminate_with_sigsegv() -> ! {
    signal(SIGSEGV, ptr::null());
    let address = 0_usize;
    let value: u32;
    std::arch::asm!(
        "ldr {value:w}, [{address}]",
        address = in(reg) address,
        value = out(reg) value,
        options(nostack, readonly)
    );
    std::hint::black_box(value);
    std::hint::unreachable_unchecked()
}

#[no_mangle]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    if outer.is_null() {
        terminate_with_sigsegv();
    }

    let inner = ptr::addr_of_mut!(INNER);
    let outer_value = outer.read();
    let inner_value = inner.read();

    if outer_value >= inner_value {
        inner.write(inner_value.wrapping_add(outer_value));
        inner
    } else {
        outer.write(outer_value.wrapping_add(inner_value));
        outer
    }
}

#[no_mangle]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc != 3 {
        printf(
            b"Error: should only be two (integer) arguments!\n\0"
                .as_ptr()
                .cast(),
        );
        return 1;
    }

    if argv.is_null() {
        terminate_with_sigsegv();
    }

    let first = argv.add(1).read();
    if first.is_null() {
        terminate_with_sigsegv();
    }

    let mut end = ptr::null_mut();
    let mut initial_value = strtol(first, &mut end, 10) as c_int;
    if end == first {
        printf(
            b"Error: first argument must be an integer!\n\0"
                .as_ptr()
                .cast(),
        );
        return 1;
    }

    let second = argv.add(2).read();
    if second.is_null() {
        terminate_with_sigsegv();
    }

    let iterations = strtol(second, &mut end, 10) as c_int;
    if end == second {
        printf(
            b"Error: second argument must be an integer!\n\0"
                .as_ptr()
                .cast(),
        );
        return 1;
    }

    let mut running_sum = ptr::addr_of_mut!(initial_value);
    let mut i = 0;
    while i < iterations {
        running_sum = static_alias(running_sum);
        printf(b"%d\n\0".as_ptr().cast(), running_sum.read());
        i += 1;
    }

    0
}
