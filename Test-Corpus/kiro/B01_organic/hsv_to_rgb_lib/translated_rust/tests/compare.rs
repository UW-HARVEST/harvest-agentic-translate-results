use libloading::{Library, Symbol};

type HsvToRgbFn = unsafe extern "C" fn(*mut f32, *const f32);

fn c_lib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

#[test]
fn test_hsv_to_rgb_matches_c() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
    let c_fn: Symbol<HsvToRgbFn> = unsafe { lib.get(b"hsv_to_rgb").unwrap() };

    // Test cases: (h, s, v) covering all switch branches + edge cases
    let cases: &[[f32; 3]] = &[
        [0.0, 0.0, 0.0],       // s==0, black
        [0.0, 0.0, 1.0],       // s==0, white
        [0.0, 1.0, 1.0],       // i=0, pure red
        [60.0, 1.0, 1.0],      // i=1, pure yellow
        [120.0, 1.0, 1.0],     // i=2, pure green
        [180.0, 1.0, 1.0],     // i=3, pure cyan
        [240.0, 1.0, 1.0],     // i=4, pure blue
        [300.0, 1.0, 1.0],     // i=5, pure magenta
        [359.0, 1.0, 1.0],     // near 360
        [30.0, 0.5, 0.8],      // i=0, partial saturation
        [90.0, 0.3, 0.6],      // i=1
        [150.0, 0.7, 0.9],     // i=2
        [210.0, 0.4, 0.5],     // i=3
        [270.0, 0.6, 0.7],     // i=4
        [330.0, 0.8, 0.4],     // i=5
        [0.0, 0.0, 0.5],       // s==0, gray
        [45.0, 0.25, 0.75],    // fractional
        [200.0, 0.9, 0.1],     // low value
    ];

    for (idx, hsv) in cases.iter().enumerate() {
        let mut c_out = [0.0f32; 3];
        let mut rs_out = [0.0f32; 3];

        unsafe { (c_fn)(c_out.as_mut_ptr(), hsv.as_ptr()) };
        unsafe { hsv_to_rgb_lib::hsv_to_rgb(rs_out.as_mut_ptr(), hsv.as_ptr()) };

        let c_bytes = bytemuck_cast(&c_out);
        let rs_bytes = bytemuck_cast(&rs_out);

        assert_eq!(
            c_bytes, rs_bytes,
            "Mismatch at case {idx}: hsv={hsv:?}\n  C={c_out:?}\n  Rust={rs_out:?}"
        );
    }
}

fn bytemuck_cast(f: &[f32; 3]) -> [u8; 12] {
    let mut out = [0u8; 12];
    for (i, val) in f.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&val.to_ne_bytes());
    }
    out
}
