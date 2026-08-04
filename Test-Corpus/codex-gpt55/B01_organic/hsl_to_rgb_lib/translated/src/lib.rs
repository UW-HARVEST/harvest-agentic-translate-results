use std::ffi::c_float;

#[link(name = "m")]
unsafe extern "C" {
    fn fmodf(x: c_float, y: c_float) -> c_float;
}

#[inline(never)]
fn fadd(a: c_float, b: c_float) -> c_float {
    a + b
}

#[inline(never)]
fn fsub(a: c_float, b: c_float) -> c_float {
    a - b
}

#[inline(never)]
fn fmul(a: c_float, b: c_float) -> c_float {
    a * b
}

#[inline(never)]
fn fdiv(a: c_float, b: c_float) -> c_float {
    a / b
}

fn fabsf_bits(x: c_float) -> c_float {
    c_float::from_bits(x.to_bits() & 0x7fff_ffff)
}

#[unsafe(no_mangle)]
pub extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    unsafe {
        let h = *src.add(0);
        let s = *src.add(1);
        let l = *src.add(2);

        if s == 0.0 {
            *dest.add(0) = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
            return;
        }

        let c = fmul(
            fsub(1.0_f32, fabsf_bits(fsub(fadd(l, l), 1.0_f32))),
            s,
        );
        let m = fsub(l, fmul(0.5_f32, c));
        let x = fmul(
            c,
            fsub(
                1.0_f32,
                fabsf_bits(fsub(fmodf(fdiv(h, 60.0_f32), 2.0_f32), 1.0_f32)),
            ),
        );

        if h >= 0.0_f32 && h < 60.0_f32 {
            *dest.add(0) = fadd(c, m);
            *dest.add(1) = fadd(x, m);
            *dest.add(2) = m;
        } else if h >= 60.0_f32 && h < 120.0_f32 {
            *dest.add(0) = fadd(x, m);
            *dest.add(1) = fadd(c, m);
            *dest.add(2) = m;
        } else if h < 120.0_f32 && h < 180.0_f32 {
            *dest.add(0) = m;
            *dest.add(1) = fadd(c, m);
            *dest.add(2) = fadd(x, m);
        } else if h >= 180.0_f32 && h < 240.0_f32 {
            *dest.add(0) = m;
            *dest.add(1) = fadd(x, m);
            *dest.add(2) = fadd(c, m);
        } else if h >= 240.0_f32 && h < 300.0_f32 {
            *dest.add(0) = fadd(x, m);
            *dest.add(1) = m;
            *dest.add(2) = fadd(c, m);
        } else if h >= 300.0_f32 && h < 360.0_f32 {
            *dest.add(0) = fadd(c, m);
            *dest.add(1) = m;
            *dest.add(2) = fadd(x, m);
        } else {
            *dest.add(0) = m;
            *dest.add(1) = m;
            *dest.add(2) = m;
        }
    }
}
