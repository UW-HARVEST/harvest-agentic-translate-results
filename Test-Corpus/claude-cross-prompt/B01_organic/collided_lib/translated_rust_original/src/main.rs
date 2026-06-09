// Translation of c_src/src/lib.c to Rust.
// The original C is a shared library (no main); this executable produces no
// output, mirroring the C library's behavior when compiled as a binary.

#![allow(dead_code)]

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum C2Type {
    Circle = 0,
    Aabb = 1,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

pub enum Shape {
    Circle(C2Circle),
    Aabb(C2AABB),
}

pub fn c2_v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

pub fn c2_maxv(a: C2v, b: C2v) -> C2v {
    // Match C's ternary: a > b ? a : b
    c2_v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

pub fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

pub fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

pub fn c2_sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

pub fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

pub fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> i32 {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    (d2 < r2) as i32
}

pub fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> i32 {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as i32
}

pub fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> i32 {
    let d0 = (b.max.x < a.min.x) as i32;
    let d1 = (a.max.x < b.min.x) as i32;
    let d2 = (b.max.y < a.min.y) as i32;
    let d3 = (a.max.y < b.min.y) as i32;
    (!(d0 | d1 | d2 | d3) & 1) as i32
}

/// Equivalent of the C `collided` function. Because the C original takes
/// `void*`, in safe Rust we accept typed `Shape` enums. The matching of
/// type tags exactly mirrors the original switch logic.
pub fn collided(a: &Shape, type_a: C2Type, b: &Shape, type_b: C2Type) -> i32 {
    match type_a {
        C2Type::Circle => match type_b {
            C2Type::Circle => {
                if let (Shape::Circle(ca), Shape::Circle(cb)) = (a, b) {
                    c2_circle_to_circle(*ca, *cb)
                } else {
                    0
                }
            }
            C2Type::Aabb => {
                if let (Shape::Circle(ca), Shape::Aabb(bb)) = (a, b) {
                    c2_circle_to_aabb(*ca, *bb)
                } else {
                    0
                }
            }
        },
        C2Type::Aabb => match type_b {
            C2Type::Circle => {
                if let (Shape::Aabb(aa), Shape::Circle(cb)) = (a, b) {
                    c2_circle_to_aabb(*cb, *aa)
                } else {
                    0
                }
            }
            C2Type::Aabb => {
                if let (Shape::Aabb(aa), Shape::Aabb(bb)) = (a, b) {
                    c2_aabb_to_aabb(*aa, *bb)
                } else {
                    0
                }
            }
        },
    }
}

fn main() {
    // The original C code is a library and has no main entry point.
    // No output is produced, mirroring the C library's behavior.
}
