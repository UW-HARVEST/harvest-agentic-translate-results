#[link(name = "m")]
unsafe extern "C" {
    fn fmodf(x: f32, y: f32) -> f32;
}

macro_rules! sse_binary_op {
    ($name:ident, $instruction:literal) => {
        #[inline(always)]
        fn $name(mut lhs: f32, rhs: f32) -> f32 {
            unsafe {
                core::arch::asm!(
                    $instruction,
                    lhs = inout(xmm_reg) lhs,
                    rhs = in(xmm_reg) rhs,
                    options(nomem, nostack, preserves_flags)
                );
            }
            lhs
        }
    };
}

sse_binary_op!(add, "addss {lhs}, {rhs}");
sse_binary_op!(sub, "subss {lhs}, {rhs}");
sse_binary_op!(mul, "mulss {lhs}, {rhs}");
sse_binary_op!(div, "divss {lhs}, {rhs}");

#[inline(always)]
fn abs(value: f32) -> f32 {
    f32::from_bits(value.to_bits() & 0x7fff_ffff)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut f32, src: *const f32) {
    let (h, s, l) = unsafe { (*src, *src.add(1), *src.add(2)) };

    if s == 0.0 {
        unsafe {
            *dest = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
        }
        return;
    }

    let c = mul(sub(1.0, abs(sub(add(l, l), 1.0))), s);
    let m = sub(l, mul(c, 0.5));
    let remainder = unsafe { fmodf(div(h, 60.0), 2.0) };
    let x = mul(sub(1.0, abs(sub(remainder, 1.0))), c);

    let rgb = if h >= 0.0 && h < 60.0 {
        (add(c, m), add(x, m), m)
    } else if h >= 60.0 && h < 120.0 {
        (add(x, m), add(c, m), m)
    } else if h < 120.0 && h < 180.0 {
        (m, add(c, m), add(m, x))
    } else if h >= 180.0 && h < 240.0 {
        (m, add(x, m), add(m, c))
    } else if h >= 240.0 && h < 300.0 {
        (add(x, m), m, add(m, c))
    } else if h >= 300.0 && h < 360.0 {
        (add(c, m), m, add(m, x))
    } else {
        (m, m, m)
    };

    unsafe {
        *dest = rgb.0;
        *dest.add(1) = rgb.1;
        *dest.add(2) = rgb.2;
    }
}
