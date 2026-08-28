mod common;

use common::*;
use std::ffi::c_void;
use std::ptr::null;

#[test]
fn make_proxy_rejects_invalid_enum_without_writes() {
    let (c, rust) = apis();
    let initial = Proxy {
        radius: f32::from_bits(0x7fc0_1234),
        count: -123_456,
        verts: [V {
            x: f32::from_bits(0x4f00_0001),
            y: f32::from_bits(0xcf00_0001),
        }; 8],
    };

    unsafe {
        for invalid_type in [3, u32::MAX, 0x8000_0000] {
            let mut c_proxy = initial;
            let mut rust_proxy = initial;
            (c.c2MakeProxy)(null(), invalid_type, &mut c_proxy);
            (rust.c2MakeProxy)(null(), invalid_type, &mut rust_proxy);
            assert_same(
                &format!("c2MakeProxy invalid type {invalid_type:#x}"),
                &c_proxy,
                &rust_proxy,
            );
            assert_same(
                &format!("c2MakeProxy unchanged type {invalid_type:#x}"),
                &initial,
                &c_proxy,
            );
        }
    }
}

#[test]
fn simplex_functions_reject_invalid_counts_identically() {
    let (c, rust) = apis();

    unsafe {
        for invalid_count in [0, -1, 4, i32::MAX, i32::MIN] {
            let mut c_simplex = Simplex::default();
            c_simplex.count = invalid_count;
            c_simplex.div = 7.0;
            c_simplex.a.p = V { x: 11.0, y: 12.0 };
            c_simplex.b.p = V { x: 13.0, y: 14.0 };
            c_simplex.c.p = V { x: 15.0, y: 16.0 };
            let mut rust_simplex = c_simplex;

            let c_metric = (c.c2GJKSimplexMetric)(&mut c_simplex);
            let rust_metric = (rust.c2GJKSimplexMetric)(&mut rust_simplex);
            assert_f32(
                &format!("c2GJKSimplexMetric count {invalid_count}"),
                c_metric,
                rust_metric,
            );
            assert_eq!(c_metric.to_bits(), 0.0f32.to_bits());

            let c_direction = (c.c2D)(&mut c_simplex);
            let rust_direction = (rust.c2D)(&mut rust_simplex);
            assert_same(
                &format!("c2D count {invalid_count}"),
                &c_direction,
                &rust_direction,
            );
            assert_same("c2D zero result", &V { x: 0.0, y: 0.0 }, &c_direction);

            let sentinel = V {
                x: f32::from_bits(0x7fc0_1234),
                y: f32::from_bits(0xffc0_5678),
            };
            let mut c_a = sentinel;
            let mut c_b = sentinel;
            let mut rust_a = sentinel;
            let mut rust_b = sentinel;
            (c.c2Witness)(&mut c_simplex, &mut c_a, &mut c_b);
            (rust.c2Witness)(&mut rust_simplex, &mut rust_a, &mut rust_b);
            assert_same(&format!("c2Witness A count {invalid_count}"), &c_a, &rust_a);
            assert_same(&format!("c2Witness B count {invalid_count}"), &c_b, &rust_b);
            assert_same("c2Witness A zero result", &V::default(), &c_a);
            assert_same("c2Witness B zero result", &V::default(), &c_b);

            let c_point = (c.c2L)(&mut c_simplex);
            let rust_point = (rust.c2L)(&mut rust_simplex);
            assert_same(&format!("c2L count {invalid_count}"), &c_point, &rust_point);
            assert_same("c2L zero result", &V::default(), &c_point);
        }
    }
}

#[test]
fn collided_rejects_all_out_of_range_enums_identically() {
    let (c, rust) = apis();
    let null_shape = null::<c_void>();

    unsafe {
        for valid_type_a in [CIRCLE, AABB, CAPSULE] {
            for invalid_type_b in [3, u32::MAX, 0x8000_0000] {
                let c_result = (c.c2Collided)(null_shape, valid_type_a, null_shape, invalid_type_b);
                let rust_result =
                    (rust.c2Collided)(null_shape, valid_type_a, null_shape, invalid_type_b);
                assert_eq!(
                    c_result, rust_result,
                    "c2Collided typeA {valid_type_a}, invalid typeB {invalid_type_b:#x}"
                );
                assert_eq!(c_result, 0);
            }
        }

        for invalid_type_a in [3, u32::MAX, 0x8000_0000] {
            for type_b in [CIRCLE, AABB, CAPSULE, 3, u32::MAX] {
                let c_result = (c.c2Collided)(null_shape, invalid_type_a, null_shape, type_b);
                let rust_result = (rust.c2Collided)(null_shape, invalid_type_a, null_shape, type_b);
                assert_eq!(
                    c_result, rust_result,
                    "c2Collided invalid typeA {invalid_type_a:#x}, typeB {type_b:#x}"
                );
                assert_eq!(c_result, 0);
            }
        }
    }
}

#[test]
fn support_zero_and_one_past_capacity_counts_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xb895_02d3_c4a7_6e1f);

    unsafe {
        for case in 0..256 {
            let mut verts = [V::default(); 9];
            for vertex in &mut verts {
                *vertex = rng.v();
            }
            let direction = rng.v();
            assert_eq!(
                (c.c2Support)(verts.as_ptr(), 0, direction),
                (rust.c2Support)(verts.as_ptr(), 0, direction),
                "c2Support zero count case {case}"
            );
            assert_eq!(
                (c.c2Support)(verts.as_ptr(), 9, direction),
                (rust.c2Support)(verts.as_ptr(), 9, direction),
                "c2Support count 9 case {case}"
            );
        }
    }
}
