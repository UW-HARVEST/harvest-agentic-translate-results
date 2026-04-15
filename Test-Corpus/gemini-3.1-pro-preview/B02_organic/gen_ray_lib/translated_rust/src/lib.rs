#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[derive(Copy, Clone)]
struct C2Circle {
    p: c2v,
    r: f32,
}

#[derive(Copy, Clone)]
struct C2Aabb {
    min: c2v,
    max: c2v,
}

#[derive(Copy, Clone)]
struct C2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[derive(Copy, Clone)]
struct C2Ray {
    p: c2v,
    d: c2v,
    t: f32,
}

#[derive(Copy, Clone)]
struct C2m {
    x: c2v,
    y: c2v,
}

fn c2_v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2_dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_len(a: c2v) -> f32 {
    c2_dot(a, a).sqrt()
}

fn c2_add(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

fn c2_sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c2_mulvs(a: c2v, b: f32) -> c2v {
    c2v {
        x: a.x * b,
        y: a.y * b,
    }
}

fn c2_div(a: c2v, b: f32) -> c2v {
    c2_mulvs(a, 1.0 / b)
}

fn c2_norm(a: c2v) -> c2v {
    c2_div(a, c2_len(a))
}

fn c2_minv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x < b.x { a.x } else { b.x },
        y: if a.y < b.y { a.y } else { b.y },
    }
}

fn c2_maxv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x > b.x { a.x } else { b.x },
        y: if a.y > b.y { a.y } else { b.y },
    }
}

fn c2_skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

fn c2_absv(a: c2v) -> c2v {
    c2v {
        x: if a.x < 0.0 { -a.x } else { a.x },
        y: if a.y < 0.0 { -a.y } else { a.y },
    }
}

fn c2_rayto_circle(a: C2Ray, b: C2Circle, out: &mut c2Raycast) -> i32 {
    let p = b.p;
    let m = c2_sub(a.p, p);
    let c = c2_dot(m, m) - b.r * b.r;
    let b_dot = c2_dot(m, a.d);
    let disc = b_dot * b_dot - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -b_dot - disc.sqrt();
    if t >= 0.0 && t <= a.t {
        out.t = t;
        let impact = c2_add(a.p, c2_mulvs(a.d, t));
        out.n = c2_norm(c2_sub(impact, p));
        return 1;
    }
    0
}

fn c2_aabbto_aabb(a: C2Aabb, b: C2Aabb) -> i32 {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    if !(d0 || d1 || d2 || d3) { 1 } else { 0 }
}

fn c2_signed_dist_point_to_plane_one_dimensional(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

fn c2_ray_to_plane_one_dimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0
    } else {
        let d = da - db;
        if d != 0.0 {
            da / d
        } else {
            0.0
        }
    }
}

fn c2_rayto_aabb(a: C2Ray, b: C2Aabb, out: &mut c2Raycast) -> i32 {
    let p0 = a.p;
    let p1 = c2_add(a.p, c2_mulvs(a.d, a.t));
    let a_box = C2Aabb {
        min: c2_minv(p0, p1),
        max: c2_maxv(p0, p1),
    };
    if c2_aabbto_aabb(a_box, b) == 0 {
        return 0;
    }
    let ab = c2_sub(p1, p0);
    let n = c2_skew(ab);
    let abs_n = c2_absv(n);
    let half_extents = c2_mulvs(c2_sub(b.max, b.min), 0.5);
    let center_of_b_box = c2_mulvs(c2_add(b.min, b.max), 0.5);
    
    let dot_val = c2_dot(n, c2_sub(p0, center_of_b_box));
    let abs_dot_val = if dot_val < 0.0 { -dot_val } else { dot_val };
    let d = abs_dot_val - c2_dot(abs_n, half_extents);
    
    if d > 0.0 {
        return 0;
    }
    
    let da0 = c2_signed_dist_point_to_plane_one_dimensional(p0.x, -1.0, b.min.x);
    let db0 = c2_signed_dist_point_to_plane_one_dimensional(p1.x, -1.0, b.min.x);
    let da1 = c2_signed_dist_point_to_plane_one_dimensional(p0.x, 1.0, b.max.x);
    let db1 = c2_signed_dist_point_to_plane_one_dimensional(p1.x, 1.0, b.max.x);
    let da2 = c2_signed_dist_point_to_plane_one_dimensional(p0.y, -1.0, b.min.y);
    let db2 = c2_signed_dist_point_to_plane_one_dimensional(p1.y, -1.0, b.min.y);
    let da3 = c2_signed_dist_point_to_plane_one_dimensional(p0.y, 1.0, b.max.y);
    let db3 = c2_signed_dist_point_to_plane_one_dimensional(p1.y, 1.0, b.max.y);
    
    let mut t0 = c2_ray_to_plane_one_dimensional(da0, db0);
    let mut t1 = c2_ray_to_plane_one_dimensional(da1, db1);
    let mut t2 = c2_ray_to_plane_one_dimensional(da2, db2);
    let mut t3 = c2_ray_to_plane_one_dimensional(da3, db3);
    
    let hit0 = t0 <= 1.0;
    let hit1 = t1 <= 1.0;
    let hit2 = t2 <= 1.0;
    let hit3 = t3 <= 1.0;
    
    let hit = hit0 || hit1 || hit2 || hit3;
    
    if hit {
        t0 = if hit0 { t0 } else { 0.0 };
        t1 = if hit1 { t1 } else { 0.0 };
        t2 = if hit2 { t2 } else { 0.0 };
        t3 = if hit3 { t3 } else { 0.0 };
        
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            out.t = t0 * a.t;
            out.n = c2_v(-1.0, 0.0);
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            out.t = t1 * a.t;
            out.n = c2_v(1.0, 0.0);
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            out.t = t2 * a.t;
            out.n = c2_v(0.0, -1.0);
        } else {
            out.t = t3 * a.t;
            out.n = c2_v(0.0, 1.0);
        }
        1
    } else {
        0
    }
}

fn c2_ccw90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

fn c2_mulmvt(a: C2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

fn c2_aabbto_point(a: C2Aabb, b: c2v) -> i32 {
    let d0 = b.x < a.min.x;
    let d1 = b.y < a.min.y;
    let d2 = b.x > a.max.x;
    let d3 = b.y > a.max.y;
    if !(d0 || d1 || d2 || d3) { 1 } else { 0 }
}

fn c2_circle_to_point(a: C2Circle, b: c2v) -> i32 {
    let n = c2_sub(a.p, b);
    let d2 = c2_dot(n, n);
    if d2 < a.r * a.r { 1 } else { 0 }
}

fn c2_rayto_capsule(a: C2Ray, b: C2Capsule, out: &mut c2Raycast) -> i32 {
    let mut m = C2m {
        x: c2_v(0.0, 0.0),
        y: c2_norm(c2_sub(b.b, b.a)),
    };
    m.x = c2_ccw90(m.y);
    
    let cap_n = c2_sub(b.b, b.a);
    let y_bb = c2_mulmvt(m, cap_n);
    let y_ap = c2_mulmvt(m, c2_sub(a.p, b.a));
    let y_ad = c2_mulmvt(m, a.d);
    let y_ae = c2_add(y_ap, c2_mulvs(y_ad, a.t));
    
    let capsule_bb = C2Aabb {
        min: c2_v(-b.r, 0.0),
        max: c2_v(b.r, y_bb.y),
    };
    
    out.n = c2_norm(cap_n);
    out.t = 0.0;
    
    if c2_aabbto_point(capsule_bb, y_ap) != 0 {
        return 1;
    } else {
        let capsule_a = C2Circle { p: b.a, r: b.r };
        let capsule_b = C2Circle { p: b.b, r: b.r };
        if c2_circle_to_point(capsule_a, a.p) != 0 {
            return 1;
        } else if c2_circle_to_point(capsule_b, a.p) != 0 {
            return 1;
        }
    }
    
    let abs_y_ae_x = if y_ae.x < 0.0 { -y_ae.x } else { y_ae.x };
    let abs_y_ap_x = if y_ap.x < 0.0 { -y_ap.x } else { y_ap.x };
    let min_abs_x = if abs_y_ae_x < abs_y_ap_x { abs_y_ae_x } else { abs_y_ap_x };
    
    if y_ae.x * y_ap.x < 0.0 || min_abs_x < b.r {
        let ca = C2Circle { p: b.a, r: b.r };
        let cb = C2Circle { p: b.b, r: b.r };
        
        if abs_y_ap_x < b.r {
            if y_ap.y < 0.0 {
                return c2_rayto_circle(a, ca, out);
            } else {
                return c2_rayto_circle(a, cb, out);
            }
        } else {
            let c = if y_ap.x > 0.0 { b.r } else { -b.r };
            let d = y_ae.x - y_ap.x;
            let t = (c - y_ap.x) / d;
            let y = y_ap.y + (y_ae.y - y_ap.y) * t;
            
            if y <= 0.0 {
                return c2_rayto_circle(a, ca, out);
            }
            if y >= y_bb.y {
                return c2_rayto_circle(a, cb, out);
            } else {
                out.n = if c > 0.0 { m.x } else { c2_skew(m.y) };
                out.t = t * a.t;
                return 1;
            }
        }
    }
    0
}

enum C2Shape {
    Circle(C2Circle),
    Aabb(C2Aabb),
    Capsule(C2Capsule),
}

fn c2_cast_ray(a: C2Ray, b: &C2Shape, out: &mut c2Raycast) -> i32 {
    match b {
        C2Shape::Circle(c) => c2_rayto_circle(a, *c, out),
        C2Shape::Aabb(aabb) => c2_rayto_aabb(a, *aabb, out),
        C2Shape::Capsule(cap) => c2_rayto_capsule(a, *cap, out),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gen_ray(
    cast1: *mut c2Raycast,
    cast2: *mut c2Raycast,
    cast3: *mut c2Raycast,
    mp_x: f32, mp_y: f32,
    r_p_x: f32, r_p_y: f32,
    c_p_x: f32, c_p_y: f32, c_r: f32,
    cap_a_x: f32, cap_a_y: f32, cap_b_x: f32, cap_b_y: f32, cap_r: f32,
    bb_min_x: f32, bb_min_y: f32, bb_max_x: f32, bb_max_y: f32,
) -> std::os::raw::c_int {
    let mut hit = 0;
    
    let mp = c2_v(mp_x, mp_y);
    
    let mut ray = C2Ray {
        p: c2_v(r_p_x, r_p_y),
        d: c2_v(0.0, 0.0),
        t: 0.0,
    };
    ray.d = c2_norm(c2_sub(mp, ray.p));
    ray.t = c2_dot(mp, ray.d) - c2_dot(ray.p, ray.d);
    
    let c = C2Shape::Circle(C2Circle {
        p: c2_v(c_p_x, c_p_y),
        r: c_r,
    });
    
    if !cast1.is_null() {
        hit += c2_cast_ray(ray, &c, unsafe { &mut *cast1 });
    }
    
    let cap = C2Shape::Capsule(C2Capsule {
        a: c2_v(cap_a_x, cap_a_y),
        b: c2_v(cap_b_x, cap_b_y),
        r: cap_r,
    });
    
    if !cast2.is_null() {
        hit += c2_cast_ray(ray, &cap, unsafe { &mut *cast2 }) << 1;
    }
    
    let bb = C2Shape::Aabb(C2Aabb {
        min: c2_v(bb_min_x, bb_min_y),
        max: c2_v(bb_max_x, bb_max_y),
    });
    
    if !cast3.is_null() {
        hit += c2_cast_ray(ray, &bb, unsafe { &mut *cast3 }) << 2;
    }
    
    hit
}
