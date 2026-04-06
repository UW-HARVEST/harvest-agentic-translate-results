use libloading::{Library, Symbol};
use std::os::raw::c_int;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libtranslated_rust.so"
    );
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

fn rust_lib() -> Library {
    // cdylib output path
    let path = format!(
        "{}/target/debug/libcharinbuf_lib.so",
        env!("CARGO_MANIFEST_DIR")
    );
    unsafe { Library::new(&path).expect("Failed to load Rust .so") }
}

fn call_charinbuf(lib: &Library, mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            lib.get(b"charinbuf").unwrap();
        f(mode, value, opt1, opt2)
    }
}

#[test]
fn test_mode0_uint16_validation() {
    let c = c_lib();
    let r = rust_lib();
    for v in [-1, 0, 1, 100, 65535, 65536, 70000] {
        let c_res = call_charinbuf(&c, 0, v, 0, 0);
        let r_res = call_charinbuf(&r, 0, v, 0, 0);
        assert_eq!(c_res, r_res, "charinbuf(0, {v}, 0, 0): C={c_res} Rust={r_res}");
    }
}

#[test]
fn test_mode1_string_empty() {
    let c = c_lib();
    let r = rust_lib();
    let c_res = call_charinbuf(&c, 1, 0, 0, 0);
    let r_res = call_charinbuf(&r, 1, 0, 0, 0);
    assert_eq!(c_res, r_res, "charinbuf mode 1: C={c_res} Rust={r_res}");
}

#[test]
fn test_mode2_malloc_free() {
    let c = c_lib();
    let r = rust_lib();
    let c_res = call_charinbuf(&c, 2, 0, 0, 0);
    let r_res = call_charinbuf(&r, 2, 0, 0, 0);
    assert_eq!(c_res, r_res, "charinbuf mode 2: C={c_res} Rust={r_res}");
}

#[test]
fn test_mode3_function_pointers() {
    let c = c_lib();
    let r = rust_lib();
    for (v, o1, o2) in [(10, 3, 2), (0, 0, 0), (5, 10, 3), (-1, 1, 1)] {
        let c_res = call_charinbuf(&c, 3, v, o1, o2);
        let r_res = call_charinbuf(&r, 3, v, o1, o2);
        assert_eq!(c_res, r_res, "charinbuf(3, {v}, {o1}, {o2}): C={c_res} Rust={r_res}");
    }
}

#[test]
fn test_mode4_memchr() {
    let c = c_lib();
    let r = rust_lib();
    let c_res = call_charinbuf(&c, 4, 0, 0, 0);
    let r_res = call_charinbuf(&r, 4, 0, 0, 0);
    assert_eq!(c_res, r_res, "charinbuf mode 4: C={c_res} Rust={r_res}");
}

#[test]
fn test_invalid_modes() {
    let c = c_lib();
    let r = rust_lib();
    for m in [-1, 5, 99] {
        let c_res = call_charinbuf(&c, m, 0, 0, 0);
        let r_res = call_charinbuf(&r, m, 0, 0, 0);
        assert_eq!(c_res, r_res, "charinbuf({m}, 0, 0, 0): C={c_res} Rust={r_res}");
    }
}
