#![allow(non_camel_case_types)]

#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

#[cfg(target_arch = "x86_64")]
unsafe fn null_pointer_fault() -> ! {
    unsafe {
        core::arch::asm!(
            "mov byte ptr [{address}], 0",
            address = in(reg) 0_usize,
            options(noreturn, nostack)
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> f64 {
    if rnd.is_null() {
        unsafe { null_pointer_fault() }
    }

    let rnd = unsafe { &mut *rnd };
    let mut x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;

    let value = x.wrapping_add(y);
    let exponent = 1023_u64;
    let mantissa = value >> 12;
    let result = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
