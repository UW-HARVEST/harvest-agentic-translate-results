mod common;
use common::*;
use libloading::Library;

fn omni_manifold_call(lib: &Library, type_a: i32, a1: f32, a2: f32, a3: f32, a4: f32, a5: f32,
                      type_b: i32, b1: f32, b2: f32, b3: f32, b4: f32, b5: f32) -> c2Manifold {
    let mut m = c2Manifold::default();
    unsafe {
        let f: libloading::Symbol<unsafe extern "C" fn(*mut c2Manifold, i32, f32, f32, f32, f32, f32,
                                                       i32, f32, f32, f32, f32, f32)> = get(lib, b"omni_manifold");
        f(&mut m, type_a, a1, a2, a3, a4, a5, type_b, b1, b2, b3, b4, b5);
    }
    m
}

#[test]
fn check_layout() {
    println!("size_of c2Manifold = {}", std::mem::size_of::<c2Manifold>());
    println!("size_of c2v = {}", std::mem::size_of::<c2v>());

    let (c, r) = load_libs();

    let mc = omni_manifold_call(&c, C2_TYPE_AABB, 0.0, 0.0, 2.0, 2.0, 0.0, C2_TYPE_CAPSULE, 0.0, 0.0, 2.0, 0.0, 0.5);
    let mr = omni_manifold_call(&r, C2_TYPE_AABB, 0.0, 0.0, 2.0, 2.0, 0.0, C2_TYPE_CAPSULE, 0.0, 0.0, 2.0, 0.0, 0.5);
    println!("C: count={} n=({:08x},{:08x}) depths=[{},{}] cp0=({},{}) cp1=({},{})",
        mc.count, mc.n.x.to_bits(), mc.n.y.to_bits(),
        mc.depths[0], mc.depths[1],
        mc.contact_points[0].x, mc.contact_points[0].y,
        mc.contact_points[1].x, mc.contact_points[1].y);
    println!("R: count={} n=({:08x},{:08x}) depths=[{},{}] cp0=({},{}) cp1=({},{})",
        mr.count, mr.n.x.to_bits(), mr.n.y.to_bits(),
        mr.depths[0], mr.depths[1],
        mr.contact_points[0].x, mr.contact_points[0].y,
        mr.contact_points[1].x, mr.contact_points[1].y);
}
