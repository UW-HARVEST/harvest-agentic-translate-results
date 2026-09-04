mod common;

use common::*;
use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Output};

fn invalid_simplex(count: i32) -> Simplex {
    Simplex {
        a: Sv {
            sA: V { x: 1.0, y: 2.0 },
            sB: V { x: 3.0, y: 4.0 },
            p: V { x: 5.0, y: 6.0 },
            u: 7.0,
            iA: 8,
            iB: 9,
        },
        b: Sv {
            sA: V { x: 10.0, y: 11.0 },
            sB: V { x: 12.0, y: 13.0 },
            p: V { x: 14.0, y: 15.0 },
            u: 16.0,
            iA: 17,
            iB: 18,
        },
        c: Sv {
            sA: V { x: 19.0, y: 20.0 },
            sB: V { x: 21.0, y: 22.0 },
            p: V { x: 23.0, y: 24.0 },
            u: 25.0,
            iA: 26,
            iB: 27,
        },
        d: Sv::default(),
        div: 28.0,
        count,
    }
}

#[test]
fn defined_rejections_and_count_boundaries_match() {
    unsafe {
        let api = Pair::load();

        let sentinel = Proxy {
            radius: -123.5,
            count: -77,
            verts: [V {
                x: 456.25,
                y: -789.75,
            }; 8],
        };
        for invalid_tag in [3, -1, i32::MAX] {
            let mut c_proxy = sentinel;
            let mut rust_proxy = sentinel;
            (api.c.c2MakeProxy)(std::ptr::null(), invalid_tag, &mut c_proxy);
            (api.rust.c2MakeProxy)(std::ptr::null(), invalid_tag, &mut rust_proxy);
            same(
                c_proxy,
                rust_proxy,
                &format!("c2MakeProxy invalid tag {invalid_tag}"),
            );
            same(c_proxy, sentinel, "c2MakeProxy leaves output unchanged");
        }

        for count in [0, 1, 4, -1, i32::MAX] {
            let mut c_simplex = invalid_simplex(count);
            let mut rust_simplex = c_simplex;
            same(
                (api.c.c2GJKSimplexMetric)(&mut c_simplex),
                (api.rust.c2GJKSimplexMetric)(&mut rust_simplex),
                &format!("metric default count {count}"),
            );
            same(
                (api.c.c2D)(&mut c_simplex),
                (api.rust.c2D)(&mut rust_simplex),
                &format!("direction default count {count}"),
            );
            same(
                (api.c.c2L)(&mut c_simplex),
                (api.rust.c2L)(&mut rust_simplex),
                &format!("location default count {count}"),
            );
        }

        for count in [0, 4, -1, i32::MAX] {
            let mut c_simplex = invalid_simplex(count);
            let mut rust_simplex = c_simplex;
            let mut c_a = V {
                x: 111.0,
                y: 222.0,
            };
            let mut c_b = V {
                x: 333.0,
                y: 444.0,
            };
            let mut rust_a = c_a;
            let mut rust_b = c_b;
            (api.c.c2Witness)(&mut c_simplex, &mut c_a, &mut c_b);
            (api.rust.c2Witness)(&mut rust_simplex, &mut rust_a, &mut rust_b);
            same(c_a, rust_a, &format!("witness A default count {count}"));
            same(c_b, rust_b, &format!("witness B default count {count}"));
        }

        for type_a in [CIRCLE, AABB, CAPSULE] {
            for invalid_b in [3, -1, i32::MAX] {
                same(
                    (api.c.c2Collided)(
                        std::ptr::null(),
                        type_a,
                        std::ptr::null(),
                        invalid_b,
                    ),
                    (api.rust.c2Collided)(
                        std::ptr::null(),
                        type_a,
                        std::ptr::null(),
                        invalid_b,
                    ),
                    &format!("c2Collided valid A {type_a}, invalid B {invalid_b}"),
                );
            }
        }
        for invalid_a in [3, -1, i32::MAX] {
            for type_b in [CIRCLE, AABB, CAPSULE, 3, -1] {
                same(
                    (api.c.c2Collided)(
                        std::ptr::null(),
                        invalid_a,
                        std::ptr::null(),
                        type_b,
                    ),
                    (api.rust.c2Collided)(
                        std::ptr::null(),
                        invalid_a,
                        std::ptr::null(),
                        type_b,
                    ),
                    &format!("c2Collided invalid A {invalid_a}, B {type_b}"),
                );
            }
        }

        let mut verts = [V::default(); 9];
        for (index, vertex) in verts.iter_mut().enumerate() {
            *vertex = V {
                x: index as f32,
                y: -(index as f32),
            };
        }
        let direction = V { x: 1.0, y: 0.0 };
        for count in [0, -1, 9] {
            same(
                (api.c.c2Support)(verts.as_ptr(), count, direction),
                (api.rust.c2Support)(verts.as_ptr(), count, direction),
                &format!("c2Support boundary count {count}"),
            );
        }
    }
}

fn run_probe(library: &Path, case: &str) -> Output {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("ffi_undefined_boundary_probe_child")
        .arg("--nocapture")
        .env("FFI_PROBE_LIBRARY", library)
        .env("FFI_PROBE_CASE", case)
        .output()
        .unwrap_or_else(|error| panic!("failed to spawn probe {case}: {error}"))
}

fn outcome_signature(output: &Output) -> (Option<i32>, Option<i32>, Vec<u8>) {
    (
        output.status.code(),
        output.status.signal(),
        output.stdout.clone(),
    )
}

#[test]
fn undefined_boundary_process_outcomes_match() {
    if std::env::var_os("FFI_PROBE_CASE").is_some() {
        return;
    }
    let cases = [
        "bbverts_null_out",
        "bbverts_null_box",
        "makeproxy_null_shape",
        "makeproxy_null_output",
        "metric_null",
        "c22_null",
        "c23_null",
        "direction_null",
        "support_null_vertices",
        "witness_null_simplex",
        "witness_null_a",
        "witness_null_b",
        "location_null",
        "gjk_null_a",
        "gjk_null_b",
        "collided_null_a",
        "collided_null_b",
        "gjk_invalid_a_tag",
        "gjk_invalid_b_tag",
    ];
    for case in cases {
        let c = run_probe(&c_library_path(), case);
        let rust = run_probe(&rust_library_path(), case);
        if matches!(case, "gjk_invalid_a_tag" | "gjk_invalid_b_tag") {
            assert_eq!(
                (c.status.code(), c.status.signal()),
                (rust.status.code(), rust.status.signal()),
                "process status differs for C-undefined case {case}"
            );
            continue;
        }
        assert_eq!(
            outcome_signature(&c),
            outcome_signature(&rust),
            "process outcome differs for {case}\nC stderr:\n{}\nRust stderr:\n{}",
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&rust.stderr),
        );
    }
}

#[test]
fn ffi_undefined_boundary_probe_child() {
    let Some(case) = std::env::var_os("FFI_PROBE_CASE") else {
        return;
    };
    let library = std::env::var_os("FFI_PROBE_LIBRARY").expect("probe library path");
    unsafe {
        let api = Api::load(Path::new(&library));
        let case = case.to_string_lossy();
        let mut out = [V::default(); 4];
        let mut bb = Box2 {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 1.0, y: 1.0 },
        };
        let circle = Circle {
            p: V { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        let mut proxy = Proxy::default();
        let mut simplex = invalid_simplex(1);
        let mut witness_a = V::default();
        let mut witness_b = V::default();
        match case.as_ref() {
            "bbverts_null_out" => {
                (api.c2BBVerts)(std::ptr::null_mut(), &mut bb);
            }
            "bbverts_null_box" => {
                (api.c2BBVerts)(out.as_mut_ptr(), std::ptr::null_mut());
            }
            "makeproxy_null_shape" => {
                (api.c2MakeProxy)(std::ptr::null(), CIRCLE, &mut proxy);
            }
            "makeproxy_null_output" => {
                (api.c2MakeProxy)(ptr(&circle), CIRCLE, std::ptr::null_mut());
            }
            "metric_null" => {
                let value = (api.c2GJKSimplexMetric)(std::ptr::null_mut());
                println!("{:08x}", value.to_bits());
            }
            "c22_null" => (api.c22)(std::ptr::null_mut()),
            "c23_null" => (api.c23)(std::ptr::null_mut()),
            "direction_null" => {
                let value = (api.c2D)(std::ptr::null_mut());
                println!("{:08x}{:08x}", value.x.to_bits(), value.y.to_bits());
            }
            "support_null_vertices" => {
                let value = (api.c2Support)(std::ptr::null(), 1, V { x: 1.0, y: 0.0 });
                println!("{value}");
            }
            "witness_null_simplex" => {
                (api.c2Witness)(std::ptr::null_mut(), &mut witness_a, &mut witness_b);
            }
            "witness_null_a" => {
                (api.c2Witness)(&mut simplex, std::ptr::null_mut(), &mut witness_b);
            }
            "witness_null_b" => {
                (api.c2Witness)(&mut simplex, &mut witness_a, std::ptr::null_mut());
            }
            "location_null" => {
                let value = (api.c2L)(std::ptr::null_mut());
                println!("{:08x}{:08x}", value.x.to_bits(), value.y.to_bits());
            }
            "gjk_null_a" => probe_gjk(&api, std::ptr::null(), CIRCLE, ptr(&circle), CIRCLE),
            "gjk_null_b" => probe_gjk(&api, ptr(&circle), CIRCLE, std::ptr::null(), CIRCLE),
            "collided_null_a" => {
                let value =
                    (api.c2Collided)(std::ptr::null(), CIRCLE, ptr(&circle), CIRCLE);
                println!("{value}");
            }
            "collided_null_b" => {
                let value =
                    (api.c2Collided)(ptr(&circle), CIRCLE, std::ptr::null(), CIRCLE);
                println!("{value}");
            }
            "gjk_invalid_a_tag" => probe_gjk(&api, ptr(&circle), 3, ptr(&circle), CIRCLE),
            "gjk_invalid_b_tag" => probe_gjk(&api, ptr(&circle), CIRCLE, ptr(&circle), 3),
            _ => panic!("unknown probe case {case}"),
        }
    }
}

unsafe fn probe_gjk(
    api: &Api,
    a: *const c_void,
    type_a: i32,
    b: *const c_void,
    type_b: i32,
) {
    let mut out_a = V::default();
    let mut out_b = V::default();
    let mut iterations = -1;
    let mut cache = Cache::default();
    let distance = unsafe {
        (api.c2GJK)(
            a,
            type_a,
            std::ptr::null(),
            b,
            type_b,
            std::ptr::null(),
            &mut out_a,
            &mut out_b,
            1,
            &mut iterations,
            &mut cache,
        )
    };
    println!(
        "{:08x}:{:08x}:{:08x}:{:08x}:{:08x}:{iterations}:{}",
        distance.to_bits(),
        out_a.x.to_bits(),
        out_a.y.to_bits(),
        out_b.x.to_bits(),
        out_b.y.to_bits(),
        cache.count
    );
}
