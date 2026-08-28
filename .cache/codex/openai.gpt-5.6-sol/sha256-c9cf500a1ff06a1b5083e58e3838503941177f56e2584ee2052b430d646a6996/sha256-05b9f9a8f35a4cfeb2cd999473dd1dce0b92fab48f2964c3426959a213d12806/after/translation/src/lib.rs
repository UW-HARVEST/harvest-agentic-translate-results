use std::ffi::c_int;

// Keep the optimized C library's scalar operand order, including NaN payload behavior.
#[cfg(target_arch = "x86_64")]
mod float_ops {
    use std::arch::asm;

    #[inline(always)]
    pub fn add(mut left: f32, right: f32) -> f32 {
        unsafe {
            asm!(
                "addss {left}, {right}",
                left = inout(xmm_reg) left,
                right = in(xmm_reg) right,
                options(nomem, nostack, preserves_flags)
            );
        }
        left
    }

    #[inline(always)]
    pub fn sub(mut left: f32, right: f32) -> f32 {
        unsafe {
            asm!(
                "subss {left}, {right}",
                left = inout(xmm_reg) left,
                right = in(xmm_reg) right,
                options(nomem, nostack, preserves_flags)
            );
        }
        left
    }

    #[inline(always)]
    pub fn mul(mut left: f32, right: f32) -> f32 {
        unsafe {
            asm!(
                "mulss {left}, {right}",
                left = inout(xmm_reg) left,
                right = in(xmm_reg) right,
                options(nomem, nostack, preserves_flags)
            );
        }
        left
    }

    #[inline(always)]
    pub fn sqrt(mut value: f32) -> f32 {
        unsafe {
            asm!(
                "sqrtss {value}, {value}",
                value = inout(xmm_reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }
}

#[cfg(not(target_arch = "x86_64"))]
mod float_ops {
    #[inline(always)]
    pub fn add(left: f32, right: f32) -> f32 {
        left + right
    }

    #[inline(always)]
    pub fn sub(left: f32, right: f32) -> f32 {
        left - right
    }

    #[inline(always)]
    pub fn mul(left: f32, right: f32) -> f32 {
        left * right
    }

    #[inline(always)]
    pub fn sqrt(value: f32) -> f32 {
        value.sqrt()
    }
}

#[inline(always)]
fn clamped_sqrt(value: f32) -> f32 {
    float_ops::sqrt(if 0.0_f32 > value { 0.0 } else { value })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(mut dest: *mut f32, mut src: *const f32, count: c_int) {
    let mut i = 0;

    while i < count {
        let first = unsafe { src.read() };
        let second = unsafe { src.add(1).read() };
        let dxy = unsafe { src.add(2).read() };

        if first < second {
            let dx2 = first;
            let dy2 = second;
            let sqd = float_ops::sub(
                float_ops::mul(dy2, dy2),
                float_ops::mul(float_ops::add(dx2, dx2), dy2),
            );
            let sqd = float_ops::add(sqd, float_ops::mul(dx2, dx2));
            let sqd = float_ops::add(sqd, float_ops::mul(float_ops::mul(dxy, 4.0_f32), dxy));
            let sum = float_ops::add(first, second);
            let lambda = float_ops::mul(float_ops::add(clamped_sqrt(sqd), sum), 0.5_f32);

            unsafe {
                dest.write(float_ops::sub(dx2, lambda));
                dest.add(1).write(dxy);
            }
        } else {
            let dy2 = first;
            let dx2 = second;
            let sqd = float_ops::sub(
                float_ops::mul(dy2, dy2),
                float_ops::mul(float_ops::add(dx2, dx2), dy2),
            );
            let sqd = float_ops::add(sqd, float_ops::mul(dx2, dx2));
            let dxy_term = float_ops::mul(float_ops::mul(dxy, 4.0_f32), dxy);
            let sqd = float_ops::add(dxy_term, sqd);
            let sum = float_ops::add(first, second);
            let lambda = float_ops::mul(float_ops::add(clamped_sqrt(sqd), sum), 0.5_f32);

            unsafe {
                dest.write(dxy);
                dest.add(1).write(float_ops::sub(dx2, lambda));
            }
        }

        src = unsafe { src.add(3) };
        dest = unsafe { dest.add(2) };
        i += 1;
    }
}
