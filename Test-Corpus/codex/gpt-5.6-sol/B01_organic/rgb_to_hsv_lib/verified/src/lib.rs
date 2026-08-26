use std::ffi::c_float;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rgb_to_hsv(dest: *mut c_float, src: *const c_float) {
    // Debug Rust aborts on null dereferences; C faults synchronously on this target.
    if src.is_null() {
        unsafe { fault_on_null_read(src) };
    }
    if dest.is_null() {
        unsafe { fault_on_null_write(dest) };
    }

    let r = unsafe { *src.add(0) };
    let g = unsafe { *src.add(1) };
    let b = unsafe { *src.add(2) };
    let mut h: c_float = 0.0;
    let s: c_float;
    let v: c_float;
    let mut min = r;
    let mut max = r;

    min = if min < g { min } else { g };
    min = if min < b { min } else { b };
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };

    let delta = max - min;
    v = max;
    if delta == 0.0 || max == 0.0 {
        unsafe {
            *dest.add(0) = h;
            *dest.add(1) = 0.0;
            *dest.add(2) = v;
        }
        return;
    }

    s = delta / max;
    if r == max {
        h = (g - b) / delta;
    } else if g == max {
        h = 2.0 + (b - r) / delta;
    } else {
        h = 4.0 + (r - g) / delta;
    }

    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }

    unsafe {
        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn fault_on_null_read(address: *const c_float) {
    unsafe {
        core::arch::asm!(
            "mov {value:e}, dword ptr [{address}]",
            value = out(reg) _,
            address = in(reg) address,
            options(nostack, readonly)
        );
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn fault_on_null_write(address: *mut c_float) {
    unsafe {
        core::arch::asm!(
            "mov dword ptr [{address}], {value:e}",
            address = in(reg) address,
            value = in(reg) 0_u32,
            options(nostack)
        );
    }
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn fault_on_null_read(address: *const c_float) {
    unsafe { std::ptr::read_volatile(address) };
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn fault_on_null_write(address: *mut c_float) {
    unsafe { std::ptr::write_volatile(address, 0.0) };
}
