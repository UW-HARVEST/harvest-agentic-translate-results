use std::io::{self, Read, Write};

#[derive(Copy, Clone, Default)]
struct C2v {
    x: f32,
    y: f32,
}

#[derive(Copy, Clone, Default)]
struct C2Raycast {
    t: f32,
    n: C2v,
}

#[derive(Copy, Clone)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Copy, Clone)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[derive(Copy, Clone)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[derive(Copy, Clone)]
struct C2Ray {
    p: C2v,
    d: C2v,
    t: f32,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum C2Type {
    Circle,
    Aabb,
    Capsule,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum C2Shape {
    Circle(C2Circle),
    Aabb(C2AABB),
    Capsule(C2Capsule),
}

#[derive(Copy, Clone)]
struct C2m {
    x: C2v,
    y: C2v,
}

fn c2_v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_len(a: C2v) -> f32 {
    c2_dot(a, a).sqrt()
}

fn c2_add(mut a: C2v, b: C2v) -> C2v {
    a.x += b.x;
    a.y += b.y;
    a
}

fn c2_sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2_mulvs(mut a: C2v, b: f32) -> C2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2_div(a: C2v, b: f32) -> C2v {
    c2_mulvs(a, 1.0f32 / b)
}

fn c2_norm(a: C2v) -> C2v {
    c2_div(a, c2_len(a))
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2_skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

fn c2_absv(a: C2v) -> C2v {
    c2_v(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

fn c2_ray_to_circle(a: C2Ray, b: C2Circle, out: &mut C2Raycast) -> i32 {
    let p = b.p;
    let m = c2_sub(a.p, p);
    let c = c2_dot(m, m) - b.r * b.r;
    let bb = c2_dot(m, a.d);
    let disc = bb * bb - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -bb - disc.sqrt();
    if t >= 0.0 && t <= a.t {
        out.t = t;
        let impact = c2_add(a.p, c2_mulvs(a.d, t));
        out.n = c2_norm(c2_sub(impact, p));
        return 1;
    }
    0
}

fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> i32 {
    let d0 = (b.max.x < a.min.x) as i32;
    let d1 = (a.max.x < b.min.x) as i32;
    let d2 = (b.max.y < a.min.y) as i32;
    let d3 = (a.max.y < b.min.y) as i32;
    (!((d0 | d1 | d2 | d3) != 0)) as i32
}

#[inline]
fn c2_signed_dist_point_to_plane_one_dimensional(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

#[inline]
fn c2_ray_to_plane_one_dimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0f32
    } else {
        let d = da - db;
        if d != 0.0 {
            da / d
        } else {
            0.0
        }
    }
}

fn c2_ray_to_aabb(a: C2Ray, b: C2AABB, out: &mut C2Raycast) -> i32 {
    let p0 = a.p;
    let p1 = c2_add(a.p, c2_mulvs(a.d, a.t));
    let a_box = C2AABB {
        min: c2_minv(p0, p1),
        max: c2_maxv(p0, p1),
    };
    if c2_aabb_to_aabb(a_box, b) == 0 {
        return 0;
    }
    let ab = c2_sub(p1, p0);
    let n = c2_skew(ab);
    let abs_n = c2_absv(n);
    let half_extents = c2_mulvs(c2_sub(b.max, b.min), 0.5f32);
    let center_of_b_box = c2_mulvs(c2_add(b.min, b.max), 0.5f32);
    let dot_val = c2_dot(n, c2_sub(p0, center_of_b_box));
    let d = (if dot_val < 0.0 { -dot_val } else { dot_val }) - c2_dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = c2_signed_dist_point_to_plane_one_dimensional(p0.x, -1.0f32, b.min.x);
    let db0 = c2_signed_dist_point_to_plane_one_dimensional(p1.x, -1.0f32, b.min.x);
    let da1 = c2_signed_dist_point_to_plane_one_dimensional(p0.x, 1.0f32, b.max.x);
    let db1 = c2_signed_dist_point_to_plane_one_dimensional(p1.x, 1.0f32, b.max.x);
    let da2 = c2_signed_dist_point_to_plane_one_dimensional(p0.y, -1.0f32, b.min.y);
    let db2 = c2_signed_dist_point_to_plane_one_dimensional(p1.y, -1.0f32, b.min.y);
    let da3 = c2_signed_dist_point_to_plane_one_dimensional(p0.y, 1.0f32, b.max.y);
    let db3 = c2_signed_dist_point_to_plane_one_dimensional(p1.y, 1.0f32, b.max.y);
    let mut t0 = c2_ray_to_plane_one_dimensional(da0, db0);
    let mut t1 = c2_ray_to_plane_one_dimensional(da1, db1);
    let mut t2 = c2_ray_to_plane_one_dimensional(da2, db2);
    let mut t3 = c2_ray_to_plane_one_dimensional(da3, db3);
    let hit0 = (t0 <= 1.0f32) as i32;
    let hit1 = (t1 <= 1.0f32) as i32;
    let hit2 = (t2 <= 1.0f32) as i32;
    let hit3 = (t3 <= 1.0f32) as i32;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        t0 = (hit0 as f32) * t0;
        t1 = (hit1 as f32) * t1;
        t2 = (hit2 as f32) * t2;
        t3 = (hit3 as f32) * t3;
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

fn c2_ccw90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

fn c2_mulmv_t(a: C2m, b: C2v) -> C2v {
    C2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

fn c2_aabb_to_point(a: C2AABB, b: C2v) -> i32 {
    let d0 = (b.x < a.min.x) as i32;
    let d1 = (b.y < a.min.y) as i32;
    let d2 = (b.x > a.max.x) as i32;
    let d3 = (b.y > a.max.y) as i32;
    (!((d0 | d1 | d2 | d3) != 0)) as i32
}

fn c2_circle_to_point(a: C2Circle, b: C2v) -> i32 {
    let n = c2_sub(a.p, b);
    let d2 = c2_dot(n, n);
    (d2 < a.r * a.r) as i32
}

fn c2_ray_to_capsule(a: C2Ray, b: C2Capsule, out: &mut C2Raycast) -> i32 {
    let m_y = c2_norm(c2_sub(b.b, b.a));
    let m_x = c2_ccw90(m_y);
    let m = C2m { x: m_x, y: m_y };
    let cap_n = c2_sub(b.b, b.a);
    let y_bb = c2_mulmv_t(m, cap_n);
    let y_ap = c2_mulmv_t(m, c2_sub(a.p, b.a));
    let y_ad = c2_mulmv_t(m, a.d);
    let y_ae = c2_add(y_ap, c2_mulvs(y_ad, a.t));
    let capsule_bb = C2AABB {
        min: c2_v(-b.r, 0.0),
        max: c2_v(b.r, y_bb.y),
    };
    out.n = c2_norm(cap_n);
    out.t = 0.0;
    if c2_aabb_to_point(capsule_bb, y_ap) != 0 {
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
    let abs_yae_x = if y_ae.x < 0.0 { -y_ae.x } else { y_ae.x };
    let abs_yap_x = if y_ap.x < 0.0 { -y_ap.x } else { y_ap.x };
    let min_abs = if abs_yae_x < abs_yap_x { abs_yae_x } else { abs_yap_x };
    if y_ae.x * y_ap.x < 0.0 || min_abs < b.r {
        let ca = C2Circle { p: b.a, r: b.r };
        let cb = C2Circle { p: b.b, r: b.r };
        if abs_yap_x < b.r {
            if y_ap.y < 0.0 {
                return c2_ray_to_circle(a, ca, out);
            } else {
                return c2_ray_to_circle(a, cb, out);
            }
        } else {
            let c = if y_ap.x > 0.0 { b.r } else { -b.r };
            let d = y_ae.x - y_ap.x;
            let t = (c - y_ap.x) / d;
            let y = y_ap.y + (y_ae.y - y_ap.y) * t;
            if y <= 0.0 {
                return c2_ray_to_circle(a, ca, out);
            }
            if y >= y_bb.y {
                return c2_ray_to_circle(a, cb, out);
            } else {
                out.n = if c > 0.0 { m.x } else { c2_skew(m.y) };
                out.t = t * a.t;
                return 1;
            }
        }
    }
    0
}

fn c2_cast_ray(a: C2Ray, b: &C2Shape, _type_b: C2Type, out: &mut C2Raycast) -> i32 {
    match b {
        C2Shape::Circle(c) => c2_ray_to_circle(a, *c, out),
        C2Shape::Aabb(bb) => c2_ray_to_aabb(a, *bb, out),
        C2Shape::Capsule(cp) => c2_ray_to_capsule(a, *cp, out),
    }
}

fn spec_ray(
    cast: &mut C2Raycast,
    mp_x: f32,
    mp_y: f32,
    c_p_x: f32,
    c_p_y: f32,
    c_r: f32,
    r_p_x: f32,
    r_p_y: f32,
) -> i32 {
    let mp = c2_v(mp_x, mp_y);

    let c = C2Circle {
        p: c2_v(c_p_x, c_p_y),
        r: c_r,
    };

    let mut ray = C2Ray {
        p: c2_v(r_p_x, r_p_y),
        d: c2_v(0.0, 0.0),
        t: 0.0,
    };
    ray.d = c2_norm(c2_sub(mp, ray.p));
    ray.t = c2_dot(mp, ray.d) - c2_dot(ray.p, ray.d);

    c2_cast_ray(ray, &C2Shape::Circle(c), C2Type::Circle, cast)
}

// Format a float in C's printf %f style (default 6 digits after the decimal point).
fn format_c_float(f: f32) -> String {
    // C's %f for float promotes to double; Rust's f32 -> f64 conversion is exact for f32.
    let d = f as f64;
    if d.is_nan() {
        return "nan".to_string();
    }
    if d.is_infinite() {
        return if d < 0.0 { "-inf".to_string() } else { "inf".to_string() };
    }
    format!("{:.6}", d)
}

fn read_all_stdin() -> String {
    let mut s = String::new();
    let _ = io::stdin().read_to_string(&mut s);
    s
}

fn main() {
    let input = read_all_stdin();
    let mut iter = input.split_ascii_whitespace();
    let mut next_f32 = || -> Option<f32> {
        let token = iter.next()?;
        token.parse::<f32>().ok()
    };

    let mp_x = match next_f32() { Some(v) => v, None => return };
    let mp_y = match next_f32() { Some(v) => v, None => return };
    let c_p_x = match next_f32() { Some(v) => v, None => return };
    let c_p_y = match next_f32() { Some(v) => v, None => return };
    let c_r = match next_f32() { Some(v) => v, None => return };
    let r_p_x = match next_f32() { Some(v) => v, None => return };
    let r_p_y = match next_f32() { Some(v) => v, None => return };

    let mut cast = C2Raycast::default();
    let hit = spec_ray(&mut cast, mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y);

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(
        handle,
        "{} {} {} {}",
        hit,
        format_c_float(cast.t),
        format_c_float(cast.n.x),
        format_c_float(cast.n.y)
    );
}
