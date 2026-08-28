#[repr(C)]
#[derive(Clone, Copy)]
pub struct LmVec2 {
    pub x: f32,
    pub y: f32,
}

fn lm_v2(x: f32, y: f32) -> LmVec2 {
    LmVec2 { x, y }
}

fn lm_sub2(a: LmVec2, b: LmVec2) -> LmVec2 {
    lm_v2(sub_f32(a.x, b.x), sub_f32(a.y, b.y))
}

fn lm_dot2(a: LmVec2, b: LmVec2) -> f32 {
    let x_product = mul_f32(a.x, b.x);
    let y_product = mul_f32(b.y, a.y);
    add_f32(y_product, x_product)
}

#[cfg(target_arch = "x86_64")]
fn scalar_op(mut lhs: f32, rhs: f32, instruction: &str) -> f32 {
    unsafe {
        match instruction {
            "addss" => core::arch::asm!(
                "addss {lhs}, {rhs}",
                lhs = inout(xmm_reg) lhs,
                rhs = in(xmm_reg) rhs,
                options(pure, nomem, nostack)
            ),
            "subss" => core::arch::asm!(
                "subss {lhs}, {rhs}",
                lhs = inout(xmm_reg) lhs,
                rhs = in(xmm_reg) rhs,
                options(pure, nomem, nostack)
            ),
            "mulss" => core::arch::asm!(
                "mulss {lhs}, {rhs}",
                lhs = inout(xmm_reg) lhs,
                rhs = in(xmm_reg) rhs,
                options(pure, nomem, nostack)
            ),
            "divss" => core::arch::asm!(
                "divss {lhs}, {rhs}",
                lhs = inout(xmm_reg) lhs,
                rhs = in(xmm_reg) rhs,
                options(pure, nomem, nostack)
            ),
            _ => unreachable!(),
        }
    }
    lhs
}

#[cfg(target_arch = "x86_64")]
fn add_f32(lhs: f32, rhs: f32) -> f32 {
    scalar_op(lhs, rhs, "addss")
}

#[cfg(target_arch = "x86_64")]
fn sub_f32(lhs: f32, rhs: f32) -> f32 {
    scalar_op(lhs, rhs, "subss")
}

#[cfg(target_arch = "x86_64")]
fn mul_f32(lhs: f32, rhs: f32) -> f32 {
    scalar_op(lhs, rhs, "mulss")
}

#[cfg(target_arch = "x86_64")]
fn div_f32(lhs: f32, rhs: f32) -> f32 {
    scalar_op(lhs, rhs, "divss")
}

#[cfg(not(target_arch = "x86_64"))]
fn add_f32(lhs: f32, rhs: f32) -> f32 {
    lhs + rhs
}

#[cfg(not(target_arch = "x86_64"))]
fn sub_f32(lhs: f32, rhs: f32) -> f32 {
    lhs - rhs
}

#[cfg(not(target_arch = "x86_64"))]
fn mul_f32(lhs: f32, rhs: f32) -> f32 {
    lhs * rhs
}

#[cfg(not(target_arch = "x86_64"))]
fn div_f32(lhs: f32, rhs: f32) -> f32 {
    lhs / rhs
}

#[unsafe(no_mangle)]
pub extern "C" fn to_barycentric(p1: LmVec2, p2: LmVec2, p3: LmVec2, p: LmVec2) -> LmVec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);
    let dot00 = lm_dot2(v0, v0);
    let dot01 = lm_dot2(v0, v1);
    let dot02 = lm_dot2(v0, v2);
    let dot11 = lm_dot2(v1, v1);
    let dot12 = lm_dot2(v1, v2);
    let inv_denom = div_f32(
        1.0_f32,
        sub_f32(mul_f32(dot00, dot11), mul_f32(dot01, dot01)),
    );
    let u = mul_f32(
        sub_f32(mul_f32(dot11, dot02), mul_f32(dot01, dot12)),
        inv_denom,
    );
    let v = mul_f32(
        sub_f32(mul_f32(dot00, dot12), mul_f32(dot01, dot02)),
        inv_denom,
    );
    lm_v2(u, v)
}
