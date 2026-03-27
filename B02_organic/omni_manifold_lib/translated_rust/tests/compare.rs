use std::path::PathBuf;
use std::process::Command;
use libloading::os::unix::Library as UnixLibrary;
extern crate libc;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2v { x: f32, y: f32 }

#[repr(C)]
#[derive(Clone, Debug)]
struct c2Manifold {
    count: i32,
    depths: [f32; 2],
    contact_points: [c2v; 2],
    n: c2v,
}

impl c2Manifold {
    fn zeroed() -> Self { unsafe { std::mem::zeroed() } }
}

fn fmt_manifold(m: &c2Manifold) -> String {
    format!(
        "count={}, depths=[{}, {}], cp=[({},{}),({},{})], n=({},{})",
        m.count, m.depths[0], m.depths[1],
        m.contact_points[0].x, m.contact_points[0].y,
        m.contact_points[1].x, m.contact_points[1].y,
        m.n.x, m.n.y,
    )
}

fn manifold_to_bytes(m: &c2Manifold) -> Vec<u8> {
    unsafe {
        std::slice::from_raw_parts(m as *const _ as *const u8, std::mem::size_of::<c2Manifold>()).to_vec()
    }
}

fn call_c_omni(helper: &std::path::Path, type_a: i32, a: &[f32; 5], type_b: i32, b: &[f32; 5]) -> Vec<u8> {
    let output = Command::new(helper)
        .args([
            &type_a.to_string(), &a[0].to_string(), &a[1].to_string(), &a[2].to_string(), &a[3].to_string(), &a[4].to_string(),
            &type_b.to_string(), &b[0].to_string(), &b[1].to_string(), &b[2].to_string(), &b[3].to_string(), &b[4].to_string(),
        ])
        .output()
        .expect("Failed to run c_helper");
    assert!(output.status.success(), "c_helper crashed (exit={:?}): {}", output.status.code(), String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).unwrap();
    stdout.trim().split_whitespace().map(|s| s.parse::<u8>().unwrap()).collect()
}

fn bytes_to_manifold(bytes: &[u8]) -> c2Manifold {
    assert_eq!(bytes.len(), std::mem::size_of::<c2Manifold>());
    let mut m = c2Manifold::zeroed();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), &mut m as *mut _ as *mut u8, bytes.len());
    }
    m
}

type OmniManifoldFn = unsafe extern "C" fn(*mut c2Manifold, i32, f32, f32, f32, f32, f32, i32, f32, f32, f32, f32, f32);

const CAPSULE: i32 = 0;
const CIRCLE: i32 = 1;
const AABB: i32 = 2;

struct TestCase { name: &'static str, type_a: i32, a: [f32; 5], type_b: i32, b: [f32; 5] }

fn test_cases() -> Vec<TestCase> {
    vec![
        TestCase { name: "circle_circle_overlap", type_a: CIRCLE, a: [0.0, 0.0, 5.0, 0.0, 0.0], type_b: CIRCLE, b: [3.0, 0.0, 5.0, 0.0, 0.0] },
        TestCase { name: "circle_circle_no_overlap", type_a: CIRCLE, a: [0.0, 0.0, 1.0, 0.0, 0.0], type_b: CIRCLE, b: [10.0, 0.0, 1.0, 0.0, 0.0] },
        TestCase { name: "circle_circle_concentric", type_a: CIRCLE, a: [1.0, 1.0, 3.0, 0.0, 0.0], type_b: CIRCLE, b: [1.0, 1.0, 5.0, 0.0, 0.0] },
        TestCase { name: "circle_aabb_overlap", type_a: CIRCLE, a: [1.0, 1.0, 2.0, 0.0, 0.0], type_b: AABB, b: [0.0, 0.0, 3.0, 3.0, 0.0] },
        TestCase { name: "circle_aabb_inside", type_a: CIRCLE, a: [1.5, 1.5, 0.5, 0.0, 0.0], type_b: AABB, b: [0.0, 0.0, 3.0, 3.0, 0.0] },
        TestCase { name: "circle_aabb_no_overlap", type_a: CIRCLE, a: [10.0, 10.0, 1.0, 0.0, 0.0], type_b: AABB, b: [0.0, 0.0, 3.0, 3.0, 0.0] },
        TestCase { name: "aabb_aabb_overlap", type_a: AABB, a: [0.0, 0.0, 4.0, 4.0, 0.0], type_b: AABB, b: [2.0, 2.0, 6.0, 6.0, 0.0] },
        TestCase { name: "aabb_aabb_no_overlap", type_a: AABB, a: [0.0, 0.0, 1.0, 1.0, 0.0], type_b: AABB, b: [5.0, 5.0, 6.0, 6.0, 0.0] },
        TestCase { name: "aabb_circle", type_a: AABB, a: [0.0, 0.0, 4.0, 4.0, 0.0], type_b: CIRCLE, b: [5.0, 2.0, 2.0, 0.0, 0.0] },
        TestCase { name: "capsule_circle_overlap", type_a: CAPSULE, a: [0.0, 0.0, 4.0, 0.0, 1.0], type_b: CIRCLE, b: [2.0, 1.5, 1.0, 0.0, 0.0] },
        TestCase { name: "circle_capsule", type_a: CIRCLE, a: [2.0, 1.5, 1.0, 0.0, 0.0], type_b: CAPSULE, b: [0.0, 0.0, 4.0, 0.0, 1.0] },
        TestCase { name: "capsule_capsule_overlap", type_a: CAPSULE, a: [0.0, 0.0, 4.0, 0.0, 1.0], type_b: CAPSULE, b: [2.0, 0.0, 6.0, 0.0, 1.0] },
        TestCase { name: "capsule_capsule_perp", type_a: CAPSULE, a: [0.0, 0.0, 4.0, 0.0, 0.5], type_b: CAPSULE, b: [2.0, -2.0, 2.0, 2.0, 0.5] },
        TestCase { name: "aabb_capsule", type_a: AABB, a: [0.0, 0.0, 4.0, 4.0, 0.0], type_b: CAPSULE, b: [3.0, 2.0, 7.0, 2.0, 0.5] },
        TestCase { name: "capsule_aabb", type_a: CAPSULE, a: [3.0, 2.0, 7.0, 2.0, 0.5], type_b: AABB, b: [0.0, 0.0, 4.0, 4.0, 0.0] },
        TestCase { name: "circle_circle_neg", type_a: CIRCLE, a: [-3.0, -2.0, 4.0, 0.0, 0.0], type_b: CIRCLE, b: [1.0, 1.0, 3.0, 0.0, 0.0] },
        TestCase { name: "aabb_aabb_large", type_a: AABB, a: [100.0, 100.0, 200.0, 200.0, 0.0], type_b: AABB, b: [150.0, 150.0, 250.0, 250.0, 0.0] },
        TestCase { name: "circle_circle_small", type_a: CIRCLE, a: [0.001, 0.002, 0.01, 0.0, 0.0], type_b: CIRCLE, b: [0.005, 0.002, 0.01, 0.0, 0.0] },
        TestCase { name: "capsule_circle_no_overlap", type_a: CAPSULE, a: [0.0, 0.0, 4.0, 0.0, 0.5], type_b: CIRCLE, b: [2.0, 10.0, 0.5, 0.0, 0.0] },
        TestCase { name: "capsule_capsule_no_overlap", type_a: CAPSULE, a: [0.0, 0.0, 2.0, 0.0, 0.3], type_b: CAPSULE, b: [0.0, 5.0, 2.0, 5.0, 0.3] },
    ]
}

#[test]
fn test_omni_manifold_c_vs_rust() {
    let helper = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/c_helper");
    assert!(helper.exists(), "c_helper not found at {:?}. Run: gcc -o target/c_helper c_helper.c -Lc_src/build -lomni_manifold_lib -Wl,-rpath,$PWD/c_src/build -lm", helper);

    let cases = test_cases();
    let mut failures = Vec::new();

    for tc in &cases {
        // Call C via subprocess
        let c_bytes = call_c_omni(&helper, tc.type_a, &tc.a, tc.type_b, &tc.b);
        let c_m = bytes_to_manifold(&c_bytes);

        // Call Rust via libloading with RTLD_DEEPBIND
        let rust_lib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libomni_manifold_lib.so");
        let rust_lib = unsafe {
            UnixLibrary::open(Some(&rust_lib_path), libc::RTLD_NOW | libc::RTLD_DEEPBIND)
                .expect("Failed to load Rust library")
        };
        let rust_omni: libloading::os::unix::Symbol<OmniManifoldFn> =
            unsafe { rust_lib.get(b"omni_manifold").expect("Failed to find Rust omni_manifold") };

        let mut rust_m = c2Manifold::zeroed();
        unsafe {
            rust_omni(&mut rust_m, tc.type_a, tc.a[0], tc.a[1], tc.a[2], tc.a[3], tc.a[4],
                      tc.type_b, tc.b[0], tc.b[1], tc.b[2], tc.b[3], tc.b[4]);
        }
        let rust_bytes = manifold_to_bytes(&rust_m);

        if c_bytes != rust_bytes {
            failures.push(format!(
                "MISMATCH [{}]:\n  C:    {}\n  Rust: {}",
                tc.name, fmt_manifold(&c_m), fmt_manifold(&rust_m),
            ));
        }
    }

    if !failures.is_empty() {
        panic!("Manifold mismatches:\n{}", failures.join("\n\n"));
    }
}
