use libloading::{Library, Symbol};

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/librgb_to_hsv_lib.so");

type RgbToHsvFn = unsafe extern "C" fn(*mut f32, *const f32);

fn rust_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    // cargo builds cdylib into target/debug/
    format!("{manifest}/target/debug/librgb_to_hsv_lib.so")
}

fn call_with_lib(lib: &Library, src: &[f32; 3]) -> [f32; 3] {
    unsafe {
        let func: Symbol<RgbToHsvFn> = lib.get(b"rgb_to_hsv").expect("rgb_to_hsv not found");
        let mut dest = [0.0f32; 3];
        func(dest.as_mut_ptr(), src.as_ptr());
        dest
    }
}

fn assert_byte_identical(c_lib: &Library, r_lib: &Library, name: &str, src: &[f32; 3]) {
    let c_out = call_with_lib(c_lib, src);
    let r_out = call_with_lib(r_lib, src);
    let c_bytes: [u8; 12] = unsafe { std::mem::transmute(c_out) };
    let r_bytes: [u8; 12] = unsafe { std::mem::transmute(r_out) };
    assert_eq!(
        c_bytes, r_bytes,
        "{name}: src={src:?} c={c_out:?} rust={r_out:?}"
    );
}

#[test]
fn test_rgb_to_hsv_cases() {
    let c_lib = unsafe { Library::new(C_LIB).expect("Failed to load C library") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust library") };

    let cases: &[(&str, [f32; 3])] = &[
        ("black", [0.0, 0.0, 0.0]),
        ("white", [1.0, 1.0, 1.0]),
        ("red", [1.0, 0.0, 0.0]),
        ("green", [0.0, 1.0, 0.0]),
        ("blue", [0.0, 0.0, 1.0]),
        ("yellow", [1.0, 1.0, 0.0]),
        ("cyan", [0.0, 1.0, 1.0]),
        ("magenta", [1.0, 0.0, 1.0]),
        ("gray", [0.5, 0.5, 0.5]),
        ("dark_red", [0.5, 0.0, 0.0]),
        ("mixed1", [0.2, 0.4, 0.6]),
        ("mixed2", [0.8, 0.3, 0.1]),
        ("mixed3", [0.1, 0.9, 0.5]),
        ("near_zero", [0.001, 0.002, 0.003]),
        ("large", [255.0, 128.0, 64.0]),
        ("negative", [-0.5, 0.5, 0.25]),
    ];
    for (name, src) in cases {
        assert_byte_identical(&c_lib, &r_lib, name, src);
    }
}
