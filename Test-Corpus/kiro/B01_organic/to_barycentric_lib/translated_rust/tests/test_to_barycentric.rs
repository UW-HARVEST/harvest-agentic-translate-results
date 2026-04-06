use libloading::{Library, Symbol};
use to_barycentric_lib::{lm_vec2, to_barycentric};

type ToBarycentricFn = unsafe extern "C" fn(lm_vec2, lm_vec2, lm_vec2, lm_vec2) -> lm_vec2;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libtranslated_rust.so"
    );
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

fn call_c(lib: &Library, p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    unsafe {
        let f: Symbol<ToBarycentricFn> = lib.get(b"to_barycentric").unwrap();
        f(p1, p2, p3, p)
    }
}

fn v(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

fn assert_eq_bytes(c: &lm_vec2, r: &lm_vec2, label: &str) {
    let cb = unsafe { std::slice::from_raw_parts(c as *const _ as *const u8, 8) };
    let rb = unsafe { std::slice::from_raw_parts(r as *const _ as *const u8, 8) };
    assert_eq!(cb, rb, "{label}: C={{x:{},y:{}}} Rust={{x:{},y:{}}}", c.x, c.y, r.x, r.y);
}

#[test]
fn test_to_barycentric_cases() {
    let lib = c_lib();
    let cases: Vec<(lm_vec2, lm_vec2, lm_vec2, lm_vec2)> = vec![
        // basic triangle with point inside
        (v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(0.25, 0.25)),
        // point at vertex p1
        (v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(0.0, 0.0)),
        // point at vertex p2
        (v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(1.0, 0.0)),
        // point at vertex p3
        (v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(0.0, 1.0)),
        // negative coords
        (v(-1.0, -1.0), v(2.0, -1.0), v(0.0, 3.0), v(0.5, 0.5)),
        // large values
        (v(100.0, 200.0), v(300.0, 400.0), v(500.0, 100.0), v(250.0, 250.0)),
        // fractional
        (v(0.1, 0.2), v(0.3, 0.4), v(0.5, 0.6), v(0.25, 0.35)),
        // point outside triangle
        (v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(2.0, 2.0)),
    ];

    for (i, (p1, p2, p3, p)) in cases.iter().enumerate() {
        let c_res = call_c(&lib, v(p1.x, p1.y), v(p2.x, p2.y), v(p3.x, p3.y), v(p.x, p.y));
        let r_res = to_barycentric(v(p1.x, p1.y), v(p2.x, p2.y), v(p3.x, p3.y), v(p.x, p.y));
        assert_eq_bytes(&c_res, &r_res, &format!("case {i}"));
    }
}
