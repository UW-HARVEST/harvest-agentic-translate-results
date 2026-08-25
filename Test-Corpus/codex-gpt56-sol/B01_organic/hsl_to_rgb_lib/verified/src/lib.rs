use std::ffi::{c_float, c_void};
use std::mem::MaybeUninit;

#[cfg(target_arch = "x86_64")]
use std::arch::asm;

#[link(name = "m")]
unsafe extern "C" {
    fn fmodf(x: c_float, y: c_float) -> c_float;
    fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn add(lhs: c_float, rhs: c_float) -> c_float {
    let mut result = lhs;
    unsafe {
        asm!(
            "addss {result}, {rhs}",
            result = inout(xmm_reg) result,
            rhs = in(xmm_reg) rhs,
            options(pure, nomem, nostack)
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn add(lhs: c_float, rhs: c_float) -> c_float {
    lhs + rhs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn sub(lhs: c_float, rhs: c_float) -> c_float {
    let mut result = lhs;
    unsafe {
        asm!(
            "subss {result}, {rhs}",
            result = inout(xmm_reg) result,
            rhs = in(xmm_reg) rhs,
            options(pure, nomem, nostack)
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn sub(lhs: c_float, rhs: c_float) -> c_float {
    lhs - rhs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn mul(lhs: c_float, rhs: c_float) -> c_float {
    let mut result = lhs;
    unsafe {
        asm!(
            "mulss {result}, {rhs}",
            result = inout(xmm_reg) result,
            rhs = in(xmm_reg) rhs,
            options(pure, nomem, nostack)
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn mul(lhs: c_float, rhs: c_float) -> c_float {
    lhs * rhs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn div(lhs: c_float, rhs: c_float) -> c_float {
    let mut result = lhs;
    unsafe {
        asm!(
            "divss {result}, {rhs}",
            result = inout(xmm_reg) result,
            rhs = in(xmm_reg) rhs,
            options(pure, nomem, nostack)
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn div(lhs: c_float, rhs: c_float) -> c_float {
    lhs / rhs
}

#[inline(always)]
fn abs(value: c_float) -> c_float {
    c_float::from_bits(value.to_bits() & 0x7fff_ffff)
}

#[inline(always)]
unsafe fn read_triplet(src: *const c_float) -> [c_float; 3] {
    let mut values = MaybeUninit::<[c_float; 3]>::uninit();
    unsafe {
        memcpy(
            values.as_mut_ptr().cast(),
            src.cast(),
            std::mem::size_of::<[c_float; 3]>(),
        );
        values.assume_init()
    }
}

#[inline(always)]
unsafe fn write_triplet(dest: *mut c_float, values: &[c_float; 3]) {
    unsafe {
        memcpy(
            dest.cast(),
            values.as_ptr().cast(),
            std::mem::size_of::<[c_float; 3]>(),
        );
    }
}

/// Converts an HSL triplet to RGB using the original library's calculations.
///
/// # Safety
///
/// `src` must point to three readable floats and `dest` must point to three
/// writable floats. The regions may overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    let [h, s, l] = unsafe { read_triplet(src) };

    if s == 0.0 {
        unsafe { write_triplet(dest, &[l, l, l]) };
        return;
    }

    let c = mul(sub(1.0, abs(sub(add(l, l), 1.0))), s);
    let m = sub(l, mul(0.5, c));
    let x = mul(
        c,
        sub(1.0, abs(sub(unsafe { fmodf(div(h, 60.0), 2.0) }, 1.0))),
    );

    let rgb = if h >= 0.0 && h < 60.0 {
        [add(c, m), add(x, m), m]
    } else if h >= 60.0 && h < 120.0 {
        [add(x, m), add(c, m), m]
    } else if h < 120.0 && h < 180.0 {
        [m, add(c, m), add(x, m)]
    } else if h >= 180.0 && h < 240.0 {
        [m, add(x, m), add(c, m)]
    } else if h >= 240.0 && h < 300.0 {
        [add(x, m), m, add(c, m)]
    } else if h >= 300.0 && h < 360.0 {
        [add(c, m), m, add(x, m)]
    } else {
        [m, m, m]
    };

    unsafe { write_triplet(dest, &rgb) };
}
